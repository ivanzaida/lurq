# Lurq Bug Report: `lurq_read_tree` Prints A Masked Text Input's Value In Clear Text

## Status

Open. Reported 2026-09-10 from the Kontur desktop app (`H:/projects/pencil-web/apps/desktop`, KONTUR-54 QA follow-ups). Handoff for the fix is at the end.

## Summary

`TextInput::mask()` / `mask_char()` is paint-only. The MCP tree tools read the input's **real** value and print it twice — once as the node's text and once as a `value="…"` attribute — with no mask handling at all. An agent, a QA harness or anyone reading a captured `lurq_read_tree` outline therefore sees the password a user typed into a masked field, even though the screenshot beside it shows bullets.

This surprises consumers in exactly the wrong direction: `mask()` reads like "this value is not shown", and the rendered frame agrees, so a tree outline is assumed to be as safe as the screenshot and gets pasted into an issue, a log or a committed evidence file. Kontur hit this while writing runtime evidence for its sign-in form: the outline of a field the user sees as `••••••••` carried the value verbatim.

The tools are gated — the server binds `127.0.0.1` only and every request must carry the bearer token from the discovery file (`crates/lurq/src/mcp/server.rs:265`, `:343`), and in Kontur the endpoint is additionally behind an opt-in per-device preference — so this is not a remote disclosure. It is a *masking* bug: the one API whose whole purpose is "do not show this value" does not reach the surface that most often ends up in a file.

## Environment

- OS: Windows 11 Pro 10.0.26200, 1.5× scale
- Crate: `lurq`
- Version: `0.18.0`; **unchanged in `0.18.1`** (that release changed the default mask glyph to `•`, not who can read the value)
- Consumer: `kontur-desktop`, features `winit`, `wgpu`, `dx12`, `i18n`, `persistent_storage`, `router`, `form`, `mcp`
- Tools involved: `lurq_read_tree`, `lurq_find_by_id` / `lurq_find` (ref records), `lurq_set_value`

## Reproduction

```rust
TextInput::new(password)
  .id("signin-password")
  .name("password")
  .single_line()
  .placeholder("Enter your password")
  .mask()            // or .mask_char('\u{2022}')
```

1. Run any lurq app with the `mcp` feature and such a field (Kontur: `cargo run -p kontur-desktop` against any server, `/signin`).
2. Type a password into it, or set one with `lurq_set_value`.
3. Call `lurq_read_tree` with `filter="all"`.

## Observed

```text
- TextInput #signin-password [ref_560] "show-hide-demo-value" value="show-hide-demo-value" @1253,711 528x69 {label=Password} {invalid=false}
```

The value appears twice: as the quoted node text and as `value="…"`. `lurq_screenshot` of the same frame shows `••••••••••••••••••••`. `lurq_find_by_id("signin-password")` returns the same value in its ref line, and `lurq_set_value` echoes it back in its JSON reply.

Nothing marks the field as masked, so a reader cannot even tell that the value they are looking at was meant to be hidden.

## Expected

A masked input reports its **masked** form, or omits the value entirely and says it is masked — e.g. `value="••••••••" {masked=true}`, or `value=<masked len=20>`. Callers that legitimately need the real value (an app's own test harness) should have to ask for it explicitly, not get it by default from the tool everyone runs first.

## Root cause

All file references are `H:/projects/lurq-0.19` (the `0.18.1` worktree); the same lines exist in `0.18.0`.

That worktree carries unreleased work on top of `0.18.1`, so its line numbers are not automatically the release's. Every line cited below was re-checked against the published `lurq 0.18.1` crate source (`~/.cargo/registry/src/index.crates.io-*/lurq-0.18.1/src/…`) on 2026-09-10 and is identical there — `mcp/tools.rs:239, 241, 312, 343, 355, 358, 447, 1094`, `mcp/server.rs:265, 343`, `node/node.rs:3488`, `node/node_kind.rs:418, 495, 508, 560` — so a maintainer can follow them straight in the release.

1. **The tree's `value=` attribute reads the raw value.**
   `crates/lurq/src/mcp/tools.rs:241`
   ```rust
   NodeKind::TextInput { state, .. } => Some(format!("value={:?}", state.value())),
   ```
   `TextInputState::value` (`crates/lurq/src/node/node_kind.rs:418`) is `self.value.get_untracked()` — the backing signal, never the mask. The summary is taken at `crates/lurq/src/mcp/tools.rs:312` and appended to the outline line at `:358`.

2. **The node's text is the raw value too.**
   `Node::sync_dynamic_content_recursive` (`crates/lurq/src/node/node.rs:3488`) copies `TextInputState::rendered_text()` into `text_content`. `rendered_text` (`crates/lurq/src/node/node_kind.rs:495`) returns the value or the placeholder — **unmasked**. The mask is applied only one function later, in `rendered_text_for_layout` (`node_kind.rs:508`), which is what the paint path uses:
   ```rust
   match self.mask() {
     Some(mask) if !self.is_showing_placeholder() => mask_text(&text, mask),
     _ => text,
   }
   ```
   MCP prints `text_content` at `crates/lurq/src/mcp/tools.rs:355`, and stores it in the ref record at `:343` (printed again at `:447`).

3. **`mask` is not exposed anywhere else.** `TextInputState::mask()` is `pub(crate)` (`node_kind.rs:560`), so even a consumer that wanted to post-process the outline cannot tell which inputs are masked.

So the value is disclosed by two independent paths, and the flag that would let either of them do the right thing is private.

## Handoff — fix in lurq

```text
Archetype: implementation in lurq (worktree; a patch release is enough)
Scope: MCP tree/ref/set-value reporting of masked TextInput values; a public "is masked" query
Out of scope: the mask glyph itself (fixed in 0.18.1), MCP authentication, redaction of any other node kind
Consumer follow-up: kontur-desktop drops the warning it now carries in `apps/desktop/README.md` and in `docs/evidence/kontur-54/README.md` once a release reports masked fields as masked
```

1. Report the masked form. In `node_value_summary` (`crates/lurq/src/mcp/tools.rs:239`) branch on the input's mask: when one is set and the placeholder is not showing, print the masked string (`mask_text`) — the same characters the frame draws — and add a `masked=true` marker so a reader knows the value was withheld. Consider `len=<chars>` instead of the glyph run for long values; do not print the real value.
2. Do the same for the node text: either mask `text_content` for MCP output (preferred: keep `text_content` as the layout path expects and mask at the MCP boundary, in `snapshot_node` and in the `RefRecord` at `crates/lurq/src/mcp/tools.rs:343`), or make `rendered_text` mask-aware and let the layout path keep using `rendered_text_for_layout`. Whichever is chosen, `lurq_find`, `lurq_find_by_id` and the ref list must agree with `lurq_read_tree`.
3. `lurq_set_value` (`crates/lurq/src/mcp/tools.rs:1094`) echoes the value it wrote; echo the masked form for a masked input, or just `{"ok": true, "kind": "TextInput", "masked": true}`.
4. Make the mask readable by consumers: a public `is_masked()` / `mask()` on the text-input handle (`TextInputHandle`) and on the devtools snapshot node, so an app's own tooling can redact deliberately. `DevToolsSnapshot` should mask a masked input's `text` for the same reason the MCP tree should.
5. If an explicit escape hatch is wanted, put it behind a named argument on the tool (`{"reveal_masked": true}`) rather than making it the default, and say in the tool description that the value is masked by default.
6. Document it: the `mask` paragraph of `docs/src/content/docs/styling-events.md` should say what MCP and DevTools report, and the MCP tool descriptions should say masked values are not returned. CHANGELOG: "MCP tree, find and set-value report a masked input's masked form instead of its value."

### Acceptance criteria

- `lurq_read_tree` on a masked, non-empty `TextInput` contains neither the value as node text nor as `value=`, in any `filter` or `max_chars` combination.
- The same holds for `lurq_find`, `lurq_find_by_id`, the ref-record list and `lurq_set_value`'s reply.
- An unmasked `TextInput` is unchanged: value as text and as `value=`.
- A masked input showing its placeholder still reports the placeholder (it is not a secret, and the paint path already treats it that way — `is_showing_placeholder`).
- The reader can tell a value was withheld (`masked=true` or an equivalent marker), so silence is not mistaken for an empty field.
- `DevToolsSnapshot` node text for a masked input is masked as well.
- A consumer can ask whether an input is masked through a public API.

### Regression tests

- `crates/lurq/tests/`: mount a form with one masked and one unmasked single-line input, set both through `lurq_set_value`, then assert the tree outline contains the unmasked value once and the masked value zero times, and that the masked line carries the marker.
- The same assertion for `lurq_find_by_id` and for the ref-record list, so the two code paths cannot drift.
- `lurq_set_value`'s reply for a masked input does not contain the value.
- Empty masked input with a placeholder: the placeholder is still reported.
- `mask_char('#')` reports `#` runs, so the marker follows the configured glyph rather than a hard-coded bullet.
- `DevToolsSnapshot::from_tree` on the same tree: the masked node's `text` is masked.
- A property-style check over the existing MCP test fixtures that no tree outline contains a value the test wrote into a masked field.

## Interim note for consumers

Until a release lands, treat a `lurq_read_tree` outline as carrying every text input's real value, masked or not: redact before writing it to a file, and never paste one containing a real credential. Kontur's capture script redacts the run's generated password from every outline it writes and uses a throwaway string for the show/hide evidence; the exposure is recorded in `apps/desktop/README.md` beside the Local MCP preference and in `docs/evidence/kontur-54/README.md`.
