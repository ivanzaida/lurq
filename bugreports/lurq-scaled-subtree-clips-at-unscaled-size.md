# Lurq Bug Report: Clips Inside A Scaled Subtree Stay At The Unscaled Size

## Status

Resolved on branch `claude/context-and-scrim` (base `9c61a9e`, 0.22.1).

Fix:

- [`ClipRect::transformed` maps a layout-space clip into screen space.](../crates/lurq/src/layout/quad.rs)
- [Quad collection maps every clip it creates through the subtree's transform.](../crates/lurq/src/layout/layout_engine.rs)
- [`Transform2D::is_axis_aligned`.](../crates/lurq/src/node/transform.rs)

Regression tests ([`animation/transform/scaled_clip.rs`](../crates/lurq/tests/animation/transform/scaled_clip.rs)):

- `clip_inside_upscaled_subtree_covers_the_scaled_card`
- `clip_inside_downscaled_subtree_covers_the_scaled_card`
- `clip_inside_rotated_subtree_is_the_bounding_box_of_the_rotated_card`
- `scaled_card_keeps_all_of_its_text_inside_its_clip`

## Summary

A container scaled with `.transform(Transform2D::scale(..))`, as a zoomable canvas does, draws its cards scaled, but a clipping card inside it (`clip()` / `Overflow::Hidden` with a background, a scroll viewport, a text input) clips its children to the card's unscaled layout rect. Zoomed in, the clip sits at the wrong place and is too small, so most of the text disappears. Zoomed out, the clip is too large and at the wrong place, so content can leak out of the card.

Reported by Orchester's pipeline editor. Present since transforms were added.

## Environment

- Crate: `lurq` 0.22.1, no features required
- Any transform other than identity on an ancestor of a clipping node. Translation alone is affected too.

## Root Cause

Quads are positioned in screen space: `transformed_quad_frame` maps a quad's origin through the accumulated transform, and the shader applies the linear part. Clip rects are tested against fragment positions, so they are screen-space too. `collect_quads` built every clip from the node's layout rect (`abs_x`, `abs_y`, layout size) and never applied the transform. Under a transform, the clip and the content it clips were therefore in different coordinate spaces.

Two guards limited the damage without fixing it. A plain logical wrapper did not clip at all under a transform, and a styled node clipped only when it had a background or border (`hidden_overflow_creates_clip`). A card has a background, so it got the wrong clip.

Text layout is not involved. Glyphs are laid out at the unscaled width and transformed with the quad, which is consistent. Only the clip was wrong.

## Reproduction

```rust
let card = Column::new().size(200.0, 40.0).padding(10.0).background(CARD).clip().child(Text::new("Card title"));
let board = Stack::new().size(400.0, 200.0).padding(Padding::new().left(150.0).top(80.0)).child(card)
  .transform(Transform2D::scale_uniform(2.0));
// The card paints at (100, 60) 400x80; its text quad's clip is (150, 80) 200x40,
// so the first glyphs (x = 119) fall left of the clip and are discarded.
```

## Fix

`ClipRect::transformed(transform)` maps a layout-space clip into screen space:

- Axis-aligned transforms (scale, flip, translation) map exactly. Corner radii follow flips and scale by the smaller axis scale; `ClipRect` has one radius per corner, so a non-uniform scale gets circular corners that stay inside the elliptical ones.
- Rotations and skews clip to the bounding box of the transformed rect, without radii. An axis-aligned clip cannot follow a rotated edge, and the bounding box never cuts content inside the rotated card.

Every clip `collect_quads` creates now goes through it: plain wrappers, overflow and scroll-viewport clips (new `content_clip`: inset by the border in layout space, transform, then intersect with the inherited clip), text-input content clips, selection highlights and the caret. Without a transform, the existing code path runs unchanged. Plain wrappers and background-less nodes now also clip under axis-aligned transforms, because their clip is now correct there. Under rotation and skew they still do not clip, as before.

## Behavior Changes

- Clipping inside scaled or translated subtrees now matches the painted content. Zoomed-in cards keep their text; zoomed-out cards no longer leak content outside the card.
- `clip()` on a plain wrapper, or on a node without a background, now clips under scale and translation. Before, it was ignored under any transform.
- Under rotation or skew, a clipping card clips to its rotated bounding box instead of to an unrotated rect at the wrong position.

## Verification

- `cargo test -p lurq --test animation_tests scaled_clip`: 4 passed; all 4 fail on `9c61a9e`.
- `animation_tests`, `layout_tests`, `runtime_tests`, `input_tests`, `components_tests`, `debugging_tests`, `dnd_tests` (features `form,router,markdown`): pass.
- The `Validate crates` commands and `cargo check --workspace --all-features --all-targets` pass.
