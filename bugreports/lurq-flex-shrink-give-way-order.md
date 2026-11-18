# lurq handoff — a shrinking row cannot say "this gives way first", nor "never below my content"

2026-10-03. Written by the Orchester coordinator for an agent working in the lurq repository. Feature request, not a
correctness bug: lurq does what its code says. Verified against `lurq 0.34.2` (`crates/lurq/src/layout/layout_engine/flex_shrink.rs`).

```text
Archetype: implementation in lurq (toolkit owner)
Repository: H:/projects/lurq (work in a worktree under .tmp/worktrees/)
Consumer: H:/projects/orchester, crates/desktop/src/screens/status_bar/ (item.rs, approvals_item.rs, activity_item.rs)
Scope: flex shrink of single-line Row/Column: a content-based minimum and an explicit give-way order
Out of scope: wrapping rows, grid, Orchester's own status bar copy
```

## The consumer's case

Orchester's status bar is one Row of items. When the window is narrow, the items must give way in a fixed order, and
some words must never be cut:

1. what the waiting approvals ask (`· Designer asks to use pencil batch_design`) — ellipsize first;
2. which run auto-approves (`· Run 3 · KON-3`) — next;
3. the event times (`15:32`) — next;
4. last, the activity's words and the approval count (`1 approval waiting`).

Never cut: the hand icon, *Review*, *Auto-approving*, *Stop*.

## What lurq does today

`shrunk_main_sizes` distributes the overflow over every child with `shrink > 0` **in proportion to the factors**,
clamping each at `min_main_size` (an explicit minimum set in code) and re-sharing what a clamped child could not take.
Two things follow:

- **No order.** Every shrinking child loses something at once. To approximate "1 before 2 before 3", the consumer set
  factors a thousandfold apart; the smaller shares are then fractions of a pixel, which still turn into a stray `…` on
  an item further down the order. With factors far enough apart to make those shares vanish, `f32` loses the small
  factors in `remaining_shrink` and part of the overflow is never assigned.
- **No content minimum.** A child shrinks to its explicit minimum (usually 0), never "to the width of its icon and
  first word". When the approvals item had to give way, it went below its icon and count and disappeared entirely.

## Orchester's workaround (branch `claude/stale-approval`, commit 904e76f0)

- Each item is split into pieces (icon + count, ` · detail`, time) on one raised background, so the parts that must
  stay are separate non-shrinking children.
- Shrink weights are powers of two so their sum stays exact in `f32`.
- Each shrinking label has 1 px of empty space after it that gives way before its words (`yielding` in
  `status_bar/item.rs`), so a sub-pixel share never shows as an ellipsis.

It works at 960 in the cases tested, but on a still narrower bar (a longer MCP address, plan-limit percentages) the
count is cut last (`1 approval wai…`) before *Stop*, and the code is fragile.

## What would solve it in lurq

Either of these (both would be ideal):

1. **Give-way order (priority).** E.g. `.shrink_order(n)` / `.flex_shrink_priority(n)`: children with the lowest order
   absorb the overflow first, down to their minimum, before any child of the next order shrinks at all. Within one
   order, today's proportional sharing. This is the common "priority+" pattern for toolbars and status bars.
2. **Content-based minimum.** E.g. `.min_width(MinSize::Content)` or `MinSize::FirstWord` / a minimum taken from a
   designated child: the child never shrinks below the measured width of its non-shrinking content (icon + first
   label), the way CSS `min-width: auto` keeps a flex item from shrinking below its min-content size.

Also worth fixing regardless: sub-pixel shares (< 0.5 px) should not truncate text into an ellipsis — round the
distribution so a child either keeps its natural width or loses at least a pixel.

## Done when

- A Row with three shrinking text children at orders 1, 2, 3 and an overflow smaller than child 1's slack shrinks only
  child 1; children 2 and 3 keep their natural width exactly (no ellipsis).
- A child with a content minimum never lays out narrower than its icon + first word, even when its factor is the
  largest.
- Tests in `crates/lurq/tests/layout/` (with name filters), docs in the layout guide, a CHANGELOG entry; semver as the
  change requires (additive API → minor).
- Then Orchester can replace the piece-splitting and `yielding` workaround in `status_bar/` with the new API.
