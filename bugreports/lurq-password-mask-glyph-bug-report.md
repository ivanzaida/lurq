# Lurq Bug Report: Masked Text Input Renders Asterisks That Sit Above The Text Line

## Status

Open. Reported 2026-09-10 from the Kontur desktop app (`H:/projects/pencil-web/apps/desktop`). Handoff for the fix is at the end.

## Summary

`TextInput::mask()` hides the value by drawing `*` for every character. In every UI font the asterisk is a superscript-height glyph (it spans from above the x-height to the cap height), so a masked password looks like a row of marks floating in the upper half of the field instead of text on the baseline. Users read it as misaligned. Every platform password field uses a bullet (`•` U+2022, WinUI uses `●` U+25CF) precisely because it is centred on the x-height.

Screenshot from Kontur's sign-in form (field 60 px tall, 14 px Inter/Segoe UI): the three `***` sit at the top third of the field while the caret and the placeholder text sit on the baseline in the vertical centre.

## Environment

- OS: Windows 11 Pro 10.0.26200, 1.5× scale
- Crate: `lurq`
- Version: `0.18.0` (`def8cf4`)
- Consumer: `kontur-desktop`, features `winit`, `wgpu`, `dx12`, `i18n`, `persistent_storage`, `form`, `mcp`
- Font: the app's text style (Inter when embedded, otherwise the system fallback Segoe UI); the glyph shape is the same problem in both.

## Reproduction

Kontur: `apps/desktop/src/auth/form.rs:89` — `control = control.mask();` on a single-line `TextInput` with `height(52)`, `padding_horizontal(14)`, 14 px text. Launch the desktop app against any server (`cargo run -p kontur-desktop`), go to `/signin`, type into `signin-password`.

Minimal lurq:

```rust
TextInput::new(value)
  .single_line()
  .height(52.0)
  .padding_horizontal(14.0)
  .mask()
```

Type three characters. Compare with `.unmask()` or with `.mask_char('\u{2022}')`.

## Observed

Masked characters render as `*` (`crates/lurq/src/node/node.rs:1425`, `text_input_mask` → `text_input_mask_char(self, '*')`). The masked string goes through the normal layout path (`node_kind.rs:508` `rendered_text_for_layout` → `mask_text`), so the baseline is correct; the glyph itself is the problem.

Glyph boxes in Segoe UI (units per em 2048, x-height 1024, cap height 1434):

| Glyph | yMin | yMax | Vertical centre |
| --- | --- | --- | --- |
| `*` U+002A | 729 | 1434 | 1082 (above the x-height) |
| `•` U+2022 | 408 | 912 | 660 |
| `●` U+25CF | 137 | 1018 | 578 |
| `x` U+0078 | 0 | 1024 | 512 |

The asterisk's centre is above the x-height line, i.e. above the entire lowercase text body; the bullet's centre is inside it.

## Expected

`.mask()` renders a glyph centred on the text body, like every OS password field: `•` U+2022 by default. `.mask_char(char)` stays for apps that want something else.

## Root cause

The default mask character in `NodeUpdate::text_input_mask` (`crates/lurq/src/node/node.rs:1425`) is `'*'`, and the docs (`docs/src/content/docs/styling-events.md:417`) describe that as the behaviour. Nothing else is wrong: caret geometry is computed over the masked string (`caret_source_text`) and remapped by `mask.len_utf8()` (`remap_caret_positions`), so a multi-byte mask character is already supported by design.

## Handoff — fix in lurq

```text
Archetype: implementation in lurq (worktree; release as 0.18.1 patch)
Scope: default mask glyph, docs, regression tests
Out of scope: per-font glyph metrics, custom mask rendering
Consumer follow-up: kontur-desktop bumps to 0.18.1 (no code change needed once the default is the bullet)
```

1. Change the default in `text_input_mask` to `'\u{2022}'` (bullet). Keep `mask_char` as the override and mention `'\u{25cf}'` for a WinUI-style heavy dot.
2. Glyph availability: the bullet exists in Inter, Segoe UI, SF, Roboto, Noto and DejaVu; if the text pipeline has a "missing glyph" path, make sure a font without U+2022 falls back to the font-fallback chain rather than a notdef box (document what happens; do not silently revert to `*`).
3. Verify the multi-byte mask end to end: `remap_caret_positions` divides byte offsets by `mask.len_utf8()` (3 for U+2022) — add a test that types `abc`, clicks between glyphs and confirms the caret index maps to char boundaries 0..=3; add a selection + Backspace test over the masked value; add a layout test that the masked width equals three bullet advances (`crates/lurq/tests/input/text_input/basic_layout.rs:47` `mask_renders_default_mask_character_for_value` currently asserts `***` and must expect `•••`; `mask_char_sets_custom_mask_character_for_value` and `unmask_clears_text_input_mask` stay as they are).
4. Regression test for the default: `.mask()` on a `TextInput` yields `rendered_text_for_layout() == "•••"` for `abc`; `.mask_char('*')` still yields `***`; `.unmask()` restores the value.
5. Docs: `styling-events.md` (mask paragraph), CHANGELOG entry "Masked inputs render U+2022 by default (was `*`)". Release `0.18.1`.

## Interim workaround for consumers

`.mask_char('\u{2022}')` on the input. Kontur will not carry this line permanently; it waits for the patch and bumps the dependency.
