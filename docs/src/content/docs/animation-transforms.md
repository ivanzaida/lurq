---
title: Animation And Transforms
description: Transitions, keyframe animations, easing, animatable properties, and 2D transform behavior.
---

# Animation And Transforms

`lurq` has two timeline systems:

- transitions, which animate a property when its target value changes,
- keyframe animations, which play a registered sequence on a node.

Both systems are owned by `Tree` and tick during the normal runtime pass. While either engine has active timelines, the shell asks for a frame each time they next draw something else (see [Frame Pacing](#frame-pacing)). What neither can move, an app steps itself in a callback run once a frame (see [Your Own Frames](#your-own-frames)).

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
| `Transition::blur()` | Animate the [blur](../blur/) radius |
| `Transition::backdrop_blur()` | Animate the [backdrop blur](../blur/#frosted-glass) radius |
| `Transition::transform()` | Animate `Transform2D` |
| `Transition::foreground()` | Animate the [foreground](../styling-events/#foreground): the color of the text in the element and under it |
| `Transition::outline()` | Animate the [outline](../styling-events/#outline): its width, offset and color together |
| `Transition::box_shadow()` | Animate the [box shadows](../styling-events/#box-shadows) |

Colors, border widths and shadows given as theme roles (`PaletteColor`, `BorderSize`, `ShadowStyle`) are read against
the theme, so they animate like concrete values. A role whose value changes because the theme does, a window that
turns dark, a palette color the app sets, is at its new value at once, as everything else in the window is: a
transition is for what the element does itself, a state it enters or a style set on it.

- The foreground, the outline and the box shadow are painted only: a transition of them lays nothing out.
- An outline that appears grows in from no width and no opacity at its own offset and color, and one that goes away
  fades back the same way, so a focus ring with `Transition::outline()` comes and goes smoothly even though the element
  has no outline at rest.
- Box shadows animate shadow by shadow, as in CSS: one with no partner in the other list comes from, or goes to,
  nothing where it is. A list in which a shadow turns from outer to inset changes halfway, and a list of more than four
  shadows changes at once.
- A background, a border color, a foreground or an offset runs between two values, not from none and not to none. An
  element with no fill at rest and one in its `hovered` style has the fill at once when the pointer comes and loses it
  at once when the pointer goes. A run on its way when the value goes away (the fill fading from the pressed one back
  to the hovered one, and the pointer leaves) ends there and then, and the next value the element gets is there at
  once. For a fade both ways give the element the value at rest too: a fill with no opacity, `Color::new(0, 0, 0, 0)`.

`duration(...)`, `duration_ms(...)`, `delay(...)`, `delay_ms(...)`, `easing(...)`, and `linear()` configure timing.

## Reduced Motion

A user can ask every app for less motion in the system's accessibility settings. lurq reads the setting on Windows
(Settings › Accessibility › Visual effects › Animation effects), macOS (System Settings › Accessibility › Display ›
Reduce motion) and iOS (Settings › Accessibility › Motion › Reduce Motion), when a window opens and each time one takes
focus, so a change made in the settings shows when the user comes back.

```rust
// What is in effect: the app's decision, or else the system's setting.
let reduced = ctx.reduced_motion();

// The app decides, whatever the system says; `None` follows the system again.
app.theme().set_reduced_motion(Some(true));
```

- While motion is reduced, a transition of where an element is or how large it is (`transform`, `offset_x`, `offset_y`,
  `width`, `height`) ends at once: the element is at its new place and size in the next frame. Sheets and stack
  navigation, which move with such transitions, switch without the slide.
- A fade or a change of color is not motion and still runs: opacity, colors, the foreground, the outline, box shadows,
  blur, radii and border widths.
- Keyframe animations are the app's and are not changed. Read `ctx.reduced_motion()`, which renders the component again
  when the setting changes, to hold back one that moves or to show a fade in its place.
- A callback of [`ctx.on_frame`](#your-own-frames) is the app's too and goes on running: what it moves is for the app
  to hold back.
- A notch of a mouse's wheel scrolls at once, where it is otherwise [animated](../styling-events/#the-mouse-wheel).
- Linux and other platforms have no setting every desktop shares, so lurq reads none there: motion is reduced only when
  the app says so with `set_reduced_motion`.
- A tree without a window, as in a test, never reads the system: its motion does not depend on the machine.

`Easing::Spring { velocity }` eases as a spring that comes to rest without going past its end: quick at first,
settling slowly. `velocity` is the speed the value already has when the transition starts, as a share of the
way per duration, and 0 from rest; something a finger lets go of goes on at the finger's speed with it.

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
| `fill_mode(...)` | None, forwards, backwards, or both; forwards holds the pose the run stops on, part way into an iteration for a fractional count |
| `iteration_count(...)` / `infinite()` | Finite or infinite playback |

## Presence: Entering And Leaving Layers

The content of a layer (an `Overlay`, a `Popup` or a `Modal`) is mounted when the layer opens, so the animation an
element in it plays is how it enters. `.exit_animation(...)` names the animation an element leaves with:

```rust
use lurq::{
  animation::{AnimatableProperty, Animation, Keyframes, KeyframesId},
  components::{Column, Popup},
};

const FADE_IN: KeyframesId = KeyframesId::new(10);
const FADE_OUT: KeyframesId = KeyframesId::new(11);

tree.register_keyframes(
  Keyframes::new(FADE_IN)
    .frame(0.0, |f| {
      f.set(AnimatableProperty::Opacity, 0.0);
    })
    .frame(1.0, |f| {
      f.set(AnimatableProperty::Opacity, 1.0);
    }),
);
tree.register_keyframes(
  Keyframes::new(FADE_OUT)
    .frame(0.0, |f| {
      f.set(AnimatableProperty::Opacity, 1.0);
    })
    .frame(1.0, |f| {
      f.set(AnimatableProperty::Opacity, 0.0);
    }),
);

let menu = Column::new()
  .animation(Animation::new(FADE_IN).duration_ms(120))
  .exit_animation(Animation::new(FADE_OUT).duration_ms(90));
Popup::new(anchor, menu).open(open.clone())
```

When a layer closes, by its open state or because what declared it is no longer rendered, lurq keeps what the layer
showed on screen until the longest exit animation in it ends, and drops it in the first pass after that. A layer with
no exit animation in it is gone with the pass that closes it, as before.

- Any element in the layer can have its own: a dialog's backdrop can fade while its panel fades and scales. An exit
  animation that ends before the longest holds its last pose until the layer is dropped, whatever its fill mode, and
  one that repeats without end plays once. An element without one stays as it was, and an animation it was playing
  goes on.
- An exit animation starts from its first frame, also when it uses the keyframes the element entered with, as
  `Animation::new(FADE_IN).direction(AnimationDirection::Reverse)` does.
- A leaving layer is only painted: it shows what it showed when it closed and is not rendered again. No press, hover
  or wheel reaches it and what is under it takes them; focus that was in it is let go; Tab passes it by; a leaving
  modal is not a Tab scope and traps nothing; `get_element_by_id`, `find_element` and element refs do not find what
  is in it.
- It keeps its place among the other layers: a popup closed inside a dialog leaves over the dialog, and a layer under
  another stays under it.
- Opening the layer again while it leaves takes it back: the layer of the same key, or else the one the same element
  declared, ends its leaving and is the layer open, the same element with its ids, so there is one of it, never one
  entering over one leaving. What had the focus in it has it again, its scroll and what was typed in it are kept,
  and what it enters with goes on from where its exit had got to: a fade out at 75 % turns into a fade in from 75 %.
  A popover, a dialog, a sheet, a menu, a tooltip and a hover card pressed open again while they go all do so.
- A popup that is hidden because its anchor scrolled out of view is still open, so it does not leave: it is gone at
  once and comes back with its anchor.
- Outside a layer an exit animation plays for a child of a container with `.presence()`: see
  [Children of a container](#children-of-a-container). Anywhere else an element that is removed is gone with the
  render that removes it.
- Exit animations are keyframe animations, so [reduced motion](#reduced-motion) leaves them to the app: use a fade,
  or none, where `ctx.reduced_motion()` is set.

### Children of a container

`.presence()` on a container keeps a child that a render removes from it until the child's exit animation ends:

```rust
let mut list = Column::new().presence();
for task in &tasks {
  list = list.child(
    Row::new()
      .key(task.id.clone())
      .exit_animation(Animation::new(FADE_OUT).duration_ms(150))
      .child(Text::new(&task.title)),
  );
}
```

- The child has to have a key or an id (`.key(...)`, `.id(...)`) and an `.exit_animation(...)` of its own. Children
  without a key are matched by their place, where one that was removed and one whose content changed look the same.
- It stays where it was among its siblings and in flow: the ones after it do not move until it is gone. An exit
  animation of its `Height` or `Width` to zero lets them close the gap as it leaves.
- Like a leaving layer it is only painted: no pointer, focus or Tab, and no lookup finds it. A child of the same key
  or id added again while it leaves ends its leaving and is the child added: the same element, with its ids, its
  scroll and what was typed in it, and the focus it had, so the two are never there together, and what it enters
  with goes on from where its exit had got to. A panel opened again while its rows fade out holds its rows once,
  and a banner or a drawer shown again while it goes is there once.
- What wants the one coming back to be another, entering beside the one still leaving, gives it a key of its own
  each time it is shown: `.key(format!("saved-{shown}"))`, with `shown` counted up by the app. A
  [toast](../design-system/#toast) does so: each `Toasts::show` is a toast of its own, so one shown again while the
  same one leaves comes in beside it.
- The elements inside it leave with their own exit animations too, and the child is gone when the longest ends.
- It works wherever the container is: in the page, in a component that renders by itself, in a popup or a dialog.
- A container without `.presence()` drops a removed child at once, whatever the child's animations. A list that
  mounts only the rows in view, like `VirtualizedList`, removes rows as they scroll out, so it has none.

Two layers are told apart by where they are declared. A popup that the press closing another one opens is another
layer: it enters while the first leaves. Two layers declared at the same place in turn (`if a { popup_a } else {
popup_b }`) are one layer to lurq, as two elements at one place are one element, unless their contents have different
keys (`content.key("a")`).

## Frame Pacing

Each pass that ticks the timelines also works out when they next draw something else, and the winit shell asks for the next frame then, at most one frame per display refresh:

- a span eased by `Easing::Steps { count, .. }` holds its value between steps, so it gets one frame per step;
- a delay draws nothing new until it ends;
- a span that changes continuously (`Easing::Linear`, the cubic Bézier easings) gets a frame every display refresh.

Between frames the event loop sleeps until the next one is due. For a continuous animation the shell wakes half a refresh after the last present and the pass then waits in the swapchain for the next vblank, which keeps one frame per vblank; input, timers, MCP calls and wakes from other threads that arrive during that wait are handled after it, up to about half a refresh later. A `Steps` animation draws long before its frame could wait, so during it they are handled as they arrive.

Each frame lays out and draws the whole window, so a continuous animation costs a full frame at the display's rate (60, 120 or 144 times a second) for as long as it runs. For an indicator that turns or pulses for a long time, such as a loading spinner, give it steps: 12 to 15 steps a turn look smooth for a small icon and cost a fifth of the frames at 60 Hz, and less on faster displays.

```rust
use lurq::animation::{Animation, Easing, StepPosition};

Animation::new(SPIN)
  .duration_ms(1000)
  .easing(Easing::Steps { count: 12, position: StepPosition::JumpEnd })
  .infinite()
```

There is no frame-rate cap per animation: steps are the cap, and they put every frame on an even pose.

Each node's run starts when the node first ticks, so spinners that mount at different moments step at different moments, and every distinct phase adds its own frames: four runs' spinners mounted one after another draw four times as many frames as one. Register the keyframes with `Tree::register_keyframes_in_phase` instead of `Tree::register_keyframes` to play every run on the process's animation clock: a run starts at the last whole number of its periods since the clock's origin (`lurq::animation::clock_origin()`), so a spinner that mounts late shows the same pose as the others and they all step together, one frame per step however many are shown. The clock is one per process, so spinners in different windows are in phase too.

- **Direction.** The period is the animation's duration for `Normal` and `Reverse`, and twice the duration for `Alternate` and `AlternateReverse`, so every run plays the same iterations forward and the same ones backward and shows the same pose in every direction.
- **Iteration count.** A late run starts part-way through an iteration, as a CSS animation with a negative delay does, so a finite iteration count ends on the clock rather than a whole count after the node mounted. A run that mounts after that end in its period does not play.
- **Delay.** The delay counts from the run's start on the clock, not from the mount: every run plays from a whole number of periods after the origin plus the delay. A node that mounts within the delay of its period waits only for the rest of it, showing nothing (or the first pose with a backwards fill) while nodes that mounted earlier already play; one that mounts later plays at once. Runs of the same animation show the same pose whenever they all play.

```rust
tree.register_keyframes_in_phase(spin_keyframes());
```

A minimized window, or one that macOS reports occluded, draws no animation frames. Animations keep their clock while it is hidden: when the window shows again it draws one frame at once, with each animation where its time puts it, and paces the frames after it as before. Windows reports a minimized window but not one covered by other windows.

A shell that drives `Tree::pass` itself does the same with `Tree::next_timeline_change()`: after a pass that only animated, `Tree::needs_redraw()` is false, and the next frame is due at that time.

## Your Own Frames

A transition and a keyframe animation move what lurq can interpolate. What an app steps itself (a spring, a body with physics, the inertia of a drag, a game or a simulation on a canvas, a value that follows another) needs to be called once for each frame the window draws, in step with the display. `ctx.on_frame` is that call, lurq's `requestAnimationFrame`:

```rust
use lurq::app::ctx::{Ctx, Frame, FrameCallback};

fn create(ctx: &mut Ctx) -> Self {
  let x = ctx.signal(0.0_f32);
  let frames = ctx.on_frame({
    let x = x.clone();
    let mut speed = 240.0_f32;
    move |frame: Frame| {
      // The time since the frame before, in seconds: the step does not depend on the display's rate.
      let next = x.get_untracked() + speed * frame.delta_secs();
      if !(0.0..=300.0).contains(&next) {
        speed = -speed;
      }
      x.set(next.clamp(0.0, 300.0));
      true // another frame
    }
  });
  Self { x, frames }
}
```

- **What it is told.** `Frame::time` is the frame's time on the runtime's clock, the time the frame's animations are drawn at. `Frame::delta` is the time since the frame this callback last ran for (`delta_secs()` gives it in seconds), and zero on its first frame.
- **When it runs.** At the start of the frame, before the window is rendered and laid out, as `requestAnimationFrame` runs before a browser's layout: a signal it sets is in the frame it ran for, not the next. It runs once in every frame the window draws while it is active, a frame drawn for an input or a timer too, so count time by `delta` and not by calls. The window's callbacks run first, then the components', parents before children.
- **When it stops.** When it returns `false`, when its handle is cancelled (`FrameCallback::cancel()`), and when its component unmounts. `FrameCallback::start()` starts a stopped one again, with a delta of zero on its first frame; `is_active()` says whether it runs. A spring that came to rest returns `false`, and the handler that gives it somewhere to go calls `start()`.
- **What it costs.** While any callback of a window runs, the window draws a frame at every refresh of its display, each laid out and painted whole, whether or not the callback changed anything: a callback that runs is a continuous animation to the [frame pacing](#frame-pacing), paced by the same rule and through the same `Tree::next_timeline_change()`. With none running nothing is drawn for them and the loop sleeps. Stop what has nothing to move.
- **Beside a video.** A playing video's frames are drawn from a list the window keeps, with no layout, when nothing else changed. A frame a callback runs for is always laid out, so what it moves keeps the display's pace beside a video.
- **A hidden window.** Callbacks are paused while the window is minimized or occluded, as animations are. The first frame after it shows again has a delta of zero, as a first frame: a simulation does not jump by the time it was not seen. `Frame::time` is the clock's own and has moved on. A frame the machine was late for while the window was shown is given its real delta; clamp it where a long step would break the integration (`frame.delta_secs().min(1.0 / 30.0)`).
- **Reduced motion** does not stop it: what a callback moves is the app's. Read `ctx.reduced_motion()` to move less, or to set the end value at once.
- **Where it is called.** In `Component::create`, as a timer is: the callback is the component's until it unmounts, and the struct keeps the handle if it stops or starts it. Called in `render`, it is the callback of that place in the render, as a `ctx.future` is: every render puts its closure in place of the last one's and keeps whether it runs and when it last ran, and a render that no longer makes the call drops it. A component kept off the stage runs none.
- **Outside a component.** `WindowHandle::on_frame` (`ctx.window().on_frame(..)`, or a handle kept from one) does the same for that window with no component behind it: a game loop the app keeps beside its tree. The window holds the callback until it stops, and `start()` gives it back. There is no `App::on_frame`: frames belong to a window, and each window has its display's rate.
- **Not an `Interval`.** A timer fires on the event loop's clock, between frames or twice in one, and is not paused with the window: use it for what happens every so often. `WgpuFrameExtension::wants_redraw` of a [`GpuViewport`](../gpu-viewport/) keeps frames coming for what an extension draws on the card; it calls no code of the app's tree.

In a test a callback runs on the tree's clock: `Tree::set_animation_clock(Some(at))` before each `Tree::pass` gives it `time` and `delta` exactly, with no sleep.

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
- `BorderDash`
- `Opacity`
- `Transform`

A width or a height in percent is animated as a share of the parent's (`AnimatableValue::Percent`): a transition or a keyframe moves it from one share to another, and between a share and pixels it changes at once. The dash and the gap of a border are one value (`Transition::border_dash()`), so one dash that makes part of an outline stays one dash on its way: the arc of a ring.

A height the content decides is one too: a transition of `Height` between a height in pixels and none runs to, and from, the height of the content, which the layout reads as it goes; a panel opens from `height(0.0)` to no height and closes back.

`Width`, `Height`, `OffsetX`, and `OffsetY` can invalidate layout while they animate. `Opacity` and `Transform` are paint-only in normal use. While an element's opacity is below 1, a subtree that paints more than one primitive is drawn into an offscreen layer every frame (see [Opacity](../styling-events/#opacity)); fading a large panel costs one layer of its size.

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
