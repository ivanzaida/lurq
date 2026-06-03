---
title: Animation And Transforms
description: Transitions, keyframe animations, easing, animatable properties, and 2D transform behavior.
---

# Animation And Transforms

`lurq` has two timeline systems:

- transitions, which animate a property when its target value changes,
- keyframe animations, which play a registered sequence on a node.

Both systems are owned by `Tree` and tick during the normal runtime pass. While either engine has active timelines, the runtime keeps requesting redraws.

## Transitions

Attach transitions to an element with `.transition(...)`. A transition watches the selected animatable property. When the target value changes, the transition engine interpolates from the previous visual value to the new target.

```rust
use lurq::{
  animation::{Easing, Transition},
  node::{CursorIcon, color::Color},
};

lurq::components::Rect::new(80.0, 40.0)
  .background("#3b82f6")
  .rounded(4.0)
  .border_inside(1.0, Color::from_hex("#1d4ed8"))
  .transition(Transition::background_color().duration_ms(250))
  .transition(Transition::all().duration_ms(350).easing(Easing::EASE_OUT))
  .hovered(|style| {
    style
      .background("#ef4444")
      .rounded(18.0)
      .border_inside(2.0, Color::from_hex("#f8fafc"))
  })
  .cursor(CursorIcon::Pointer)
```

Use a property-specific transition when only one value should animate. Use `Transition::all()` when several style, size, offset, opacity, or transform values can change together.

Transition builders include:

| Builder | Purpose |
|---------|---------|
| `Transition::all()` | Animate every supported property that changes |
| `Transition::background_color()` | Animate fill/background color |
| `Transition::border_color()` | Animate border color |
| `Transition::border_width_top()` and related sides | Animate border widths |
| `Transition::border_radius_top_left()` and related corners | Animate corner radii |
| `Transition::offset_x()` / `Transition::offset_y()` | Animate relative offset |
| `Transition::width()` / `Transition::height()` | Animate frame dimensions |
| `Transition::opacity()` | Animate opacity |
| `Transition::transform()` | Animate `Transform2D` |

`duration(...)`, `duration_ms(...)`, `delay(...)`, `delay_ms(...)`, `easing(...)`, and `linear()` configure timing.

## Keyframe Animations

Register keyframes on the tree before nodes reference them:

```rust
use lurq::{
  animation::{AnimatableProperty, Animation, Easing, Keyframes, KeyframesId},
  node::color::Color,
};

let mut tree = lurq::app::Tree::new();
const COLOR_CYCLE: KeyframesId = KeyframesId::new(1);

tree.register_keyframes(
  Keyframes::new(COLOR_CYCLE)
    .frame(0.0, |f| {
      f.set(AnimatableProperty::BackgroundColor, Color::from_hex("#3b82f6"));
    })
    .frame(0.5, |f| {
      f.set(AnimatableProperty::BackgroundColor, Color::from_hex("#8b5cf6"));
      f.easing(Easing::EASE_IN_OUT);
    })
    .frame(1.0, |f| {
      f.set(AnimatableProperty::BackgroundColor, Color::from_hex("#3b82f6"));
    }),
);
```

Then attach the animation by ID:

```rust
lurq::components::Rect::new(64.0, 64.0)
  .background("#3b82f6")
  .animation(Animation::new(COLOR_CYCLE).duration_ms(1600).linear().infinite())
```

Keyframe offsets are normalized from `0.0` to `1.0`. The engine finds the surrounding frames for the current progress, applies the frame easing when present, and interpolates properties that exist in both frames.

`Animation` supports:

| Method | Purpose |
|--------|---------|
| `duration(...)` / `duration_ms(...)` | Total duration for one iteration |
| `delay(...)` / `delay_ms(...)` | Delay before playback starts |
| `easing(...)` / `linear()` | Default easing for frame spans |
| `direction(...)` / `alternate()` | Normal, reverse, alternate, or alternate-reverse playback |
| `fill_mode(...)` | None, forwards, backwards, or both |
| `iteration_count(...)` / `infinite()` | Finite or infinite playback |

## Animatable Properties

The animation engine supports color, float, and decomposed transform values:

```rust
use lurq::{
  animation::{AnimatableProperty, AnimatableValue},
  node::{color::Color, transform::Decomposed},
};

AnimatableValue::Color(Color::from_hex("#22c55e"));
AnimatableValue::Float(0.5);
AnimatableValue::Transform(Decomposed::IDENTITY.with_scale(1.2, 1.2));
```

The supported properties are:

- `BackgroundColor`
- `BorderColor`
- `BorderWidthTop`, `BorderWidthRight`, `BorderWidthBottom`, `BorderWidthLeft`
- `BorderRadiusTopLeft`, `BorderRadiusTopRight`, `BorderRadiusBottomRight`, `BorderRadiusBottomLeft`
- `OffsetX`, `OffsetY`
- `Width`, `Height`
- `Opacity`
- `Transform`

`Width`, `Height`, `OffsetX`, and `OffsetY` can invalidate layout while they animate. `Opacity` and `Transform` are paint-only in normal use.

## Transforms

Use `.transform(Transform2D)` for a visual 2D transform around the element center:

```rust
use lurq::node::transform::Transform2D;

lurq::components::Rect::new(80.0, 48.0)
  .background("#3b82f6")
  .transform(Transform2D::rotate_deg(12.0))
```

`Transform2D` has helpers for:

| Helper | Purpose |
|--------|---------|
| `Transform2D::translate(x, y)` | Translate in paint space |
| `Transform2D::scale(sx, sy)` | Scale independently on x and y |
| `Transform2D::scale_uniform(s)` | Scale both axes |
| `Transform2D::rotate(radians)` | Rotate in radians |
| `Transform2D::rotate_deg(degrees)` | Rotate in degrees |
| `Transform2D::skew(ax, ay)` | Skew by radians |
| `.then(&other)` | Compose transforms |

Composition is applied right-to-left. In this example, the scale is applied first, then the rotation:

```rust
lurq::components::Rect::new(80.0, 48.0)
  .transform(Transform2D::rotate_deg(20.0).then(&Transform2D::scale(1.2, 0.9)))
```

Transforms are visual-only for parent layout. The element keeps its measured layout size, siblings are not moved by the transform, and the transformed subtree paints from that layout slot. Hit testing uses transformed coordinates, so pointer events, text selection, and text input carets follow the painted position.

## Transform Animation

Transform interpolation uses `Decomposed` values: translation, scale, rotation, and skew are interpolated independently, then recomposed into a matrix.

```rust
use lurq::{
  animation::{AnimatableProperty, Animation, Keyframes, KeyframesId},
  node::transform::Decomposed,
};

const SPIN: KeyframesId = KeyframesId::new(14);

tree.register_keyframes(
  Keyframes::new(SPIN)
    .frame(0.0, |f| {
      f.set(AnimatableProperty::Transform, Decomposed::IDENTITY.with_rotate(0.0));
    })
    .frame(1.0, |f| {
      f.set(
        AnimatableProperty::Transform,
        Decomposed::IDENTITY.with_rotate(std::f32::consts::TAU),
      );
    }),
);

lurq::components::Rect::new(48.0, 48.0)
  .animation(Animation::new(SPIN).duration_ms(1200).linear().infinite())
```

Use `Decomposed` directly when the path matters. For example, `Transform2D::rotate_deg(360.0)` is visually the identity matrix, so it cannot express a full spin by itself.

## Text Under Transforms

Text defaults to `TextTransformMode::Bitmap`. It rasterizes glyphs normally and applies the transform to glyph quads at render time. This is the best mode for animated transforms because changing the angle does not create a new glyph atlas entry for every frame.

Use `TextTransformMode::Rasterized` for static transformed text that needs sharper rotated glyph edges:

```rust
use lurq::node::{TextTransformMode, transform::Transform2D};

lurq::components::Text::new("Static label")
  .text_transform_mode(TextTransformMode::Rasterized)
  .transform(Transform2D::rotate_deg(-8.0))
```

Because the transform matrix is part of the rasterized glyph cache key, continuously animated text should usually stay on `Bitmap`.
