# Progress: hand-off

`ui::Progress` and `ui::RingProgress`, from the board's `Progress` (`Ss4Vg`), `Ring Progress` (`DWQLT`),
`Loader/Linear` (`g7K92c`), `Loader/Progress Labeled` (`CReHp`) and `Loader/Top Bar` (`pmfFc`) in Components / light
(`Ovt1D`), and where they are used: Slider & Progress (`j3Sid`: 60%, 25%, the tones success, warning and destructive),
Loading (`mKt0p`: the linear loader, the labeled one, the ring, the top bar), a file's row in File Upload (a rail of 4)
and the Progress Dialog (a rail of 6). `Meter` (`ZdD5u`) and `Gauge` (`CiuDa`) were read and not built: the parts are
named so that a recipe or a renderer makes them (under "Decided").

Files: `crates/lurq/src/ui/progress.rs`, `crates/lurq/tests/ui/progress.rs`, `crates/gallery/src/pages/progress.rs`
(page "Progress", group Data), `design/handoff/progress.md`, and lines in the three `mod.rs` files.

## Changed outside the component's own files

- `crates/lurq/src/animation/interpolate.rs`: a new `AnimatableValue::Percent(f32)`, a width or a height as a share of
  the parent's. `read_target` reads a width or a height in percent (`Dimension::Pct`) as `Percent`, which a transition
  and a keyframe move to another share; between a share and pixels the value changes at once. Before, a width in
  percent was read as 0 px, so a transition of it ran from or to nothing. A unit test, `lerp_percent`.
- `crates/lurq/src/node/node.rs`, `Node::effective_frame`: an animated `Percent` width or height lays the node out at
  that share (`Dimension::Pct`).
- `crates/lurq/src/animation/animation.rs`, `AnimationEngine::new`: registers the progress's keyframes, in phase:
  `0xFF21` (`ui::PROGRESS_LEAD`) and `0xFF22` (`ui::PROGRESS_SEGMENT`). `0xFF23` to `0xFF28` are left free.

## For `docs/src/content/docs/design-system.md`

## Progress

How far a piece of work is: a file uploading, items being copied. A bar along a rail, the thin bar across the top of a window, or a ring with the value in its middle.

```rust
use lurq::ui::{Progress, ProgressSize, ProgressVariant, RingProgress};

// A fraction of one.
Progress::new(0.62).label("Uploading theme.json").show_value(true).size(ProgressSize::Md);

// 42 of 128 items, said as the app says it.
Progress::new(42).max(128).show_value(true).format(|done, all| format!("{done} of {all}"));

// Work whose end is not known.
Progress::indeterminate().label("Connecting");

// The thin bar across the top of a page, first in the page's column.
Progress::new(loaded.get()).top_bar(true);

// The same value around a circle, with it in the middle.
RingProgress::new(0.7).variant(ProgressVariant::Success);
```

| Builder | |
| --- | --- |
| `Progress::new(value)` | How far the work is: a fraction of one, or of `max`. Kept between nothing and `max` |
| `Progress::indeterminate()` | Work whose end is not known: a segment crosses the rail again and again |
| `value(n)`, `max(m)` | How far the work is, and the whole of it (1 when not given). A `max` of nothing or less: nothing is done |
| `variant(ProgressVariant)` | The color of what is filled: `Default` (primary), `Success`, `Warning`, `Destructive` |
| `size(ProgressSize)` | The rail's height: `Sm` 4, `Md` 6, `Lg` 8 (the default). A ring of 40, 56 or 72 |
| `label(&str)` | What the work is, over the start of the rail, and what the progress is called |
| `show_value(bool)` | The value over the end of the rail, as a percentage or as `format` says it. None while it is not known. A ring shows it unless told not to |
| `format(Fn(f32, f32) -> String)` | Says the value, given the value and `max`: "42 of 128" |
| `top_bar(bool)` | The thin bar across the top of a window or a surface: 3 high, no rail, as wide as what holds it |
| `width(impl Into<Length>)` | How wide the progress is: 240 when not given |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the root |

- A progress is read, not operated: it takes no focus and no press. What is filled is a share of the rail, and moves to a new value over the theme's slow duration; while motion is reduced it is there at once.
- Work that is not known runs a segment of 90 of 240 across the rail, growing out of its start and leaving at its end, once in `ui::PROGRESS_PERIOD` (1.6 s), in phase with every other such bar. While motion is reduced the segment stands still 70 of 240 from the start of the rail, where no value would be, as the board draws it.
- Parts: `ProgressPart::Root` (the header over the rail), `Header` (the label and the value), `Label`, `Value`, `Track` (the rail), `Fill` (what is filled; the moving segment). States: `ProgressState::Indeterminate`, `TopBar`.
- In the design the rail is 240 by 8, round, in `Secondary`, filled in `Primary`, or `Success`, `Warning`, `Destructive`. Under a label and in a dialog it is 6, in a row of a list 4. The label is 8 over the rail in the label's type (13, medium) and `Foreground`; the value is at the end in the mono type of 11, `MutedForeground`. The top bar is 3 high with no rail, filled with corners of 2.
- A ring (`RingProgress`) is 72 across, the ring a tenth of that wide, in `Secondary`, with what is done over it in the variant's color from the top and clockwise, and the value in the middle in the label's type of 13 at a weight of 600 (12 and 11 in the rings of 56 and 40). It takes `value`, `max`, `variant`, `size`, `label` (its name to tooling: the board draws no text beside it), `show_value`, `format`, `state`, `style`, `rules`, `id` and `key`. It always knows the value: what does not is a spinner's. Parts: `RingProgressPart::Root`, `Track` (the disc, in the color of what is not done), `Fill` (its background is the arc's color), `Middle` (painted over the disc in the page's color), `Value`. On a surface of another color, give the middle that color: `.style(RingProgressPart::Middle, |s| s.background(PaletteColor::Card))`. The ring's arc moves to a new value at once.
- To what reads the tree (`lurq_snapshot` of the MCP server) a progress and a ring are a `progressbar` called by their label, with `a11y_valuenow`, `a11y_valuemin` (0), `a11y_valuemax` and `a11y_valuetext` (the value as it is said); one that does not know how far the work is has no value.
- A renderer that draws another progress has `view.fraction()`, `view.value()`, `view.max()`, `view.value_text()`, `view.root()` (the root with its name to tooling), `view.header()` and `view.track()` (the rail with what is filled, or with the moving segment). A meter is the same parts restyled: `Progress::new(96).max(128).label("Disk space").show_value(true).variant(ProgressVariant::Warning).format(..)` with a radius of 2 on the `Track` and the `Fill`; a meter of cells is a renderer that draws a row of them from `view.fraction()`. A ring's renderer has `view.fraction()`, `view.value_text()`, `view.root()`, `view.disc()` and `view.middle()`.

## For `CHANGELOG.md`

- `ui::Progress` and `ui::RingProgress`: how far a piece of work is, as a bar along a rail in four colors and three sizes with a label and its value, a bar that does not know how far it is with a segment crossing its rail (still while motion is reduced), the thin bar across the top of a window, and a ring with the value in its middle; a width or a height in percent now transitions and animates.

## Decided

- **A builder.** The value is the app's, given each render; nothing is kept. A progress is not operated, so it has no handler, no `disabled` and no `force`, and the playground's states switch is off.
- **A fraction of one by default**, `max` for other ranges, as `Slider::new_f32` is a fraction. A value is kept between nothing and `max`; a `max` of nothing or less has no room, and nothing is done. The value is said as a whole percentage ("62%"); `format` is given the value as it is kept and `max`.
- **The fill is a share of the rail** (`Dimension::Pct`), so the rail can be as wide as anything, and the fill moves by a transition of its width over `MotionDuration::Slow` (240 ms), standard easing. The engine moved only widths in pixels; it now moves shares too (see the change above). With motion reduced the transition engine sets a width at once.
- **The moving segment is two nodes in the rail**: the room before it, which grows from nothing to the whole rail, and the segment, which grows to 37.5% (the board's 90 of 240) over the first 37.5% of the crossing and keeps it, so it leaves the rail only as the crossing ends and the next comes in from the start. Both play linearly in 1.6 s, in phase (`register_keyframes_in_phase`), so they stay together and every such bar of a window moves as one. The fill and the segment are keyed apart, so a bar that comes to know its value grows from nothing rather than from the segment.
- **With motion reduced** the segment stands still at the board's pose, 70 of 240 from the start: a fill that does not start at the start of the rail does not read as a value. The keyframe engine does not reduce motion itself, so the progress reads `Theme::reduced_motion()` as it builds.
- **The top bar is a state**, not a size: it changes the rail (no fill of its own, square, 3 high) and the width (all the room there is). It is not a layer over the window: it goes first in what it is the top of, as the board draws it in a surface, so that a page with a title bar of its own puts it under that.
- **The label and the value are a header over the rail**, 8 above it: the label takes the room there is, so the value stands at the end with no label too. A progress that does not know how far it is says no value, and keeps its label.
- **Sizes are the rails the board draws**: 8 in the component, 6 under a label and in the Progress Dialog, 4 in a file's row and in Loader/Linear. The default is the component's own, `Lg`. The ring takes the same sizes, 72 (the board's) and 56 and 40, its value 13, 12 and 11.
- **The ring always knows the value.** Work whose end is not known is a spinner's (the Spinner session's), and the board draws no ring that does not know.
- **The ring is a disc with a conic gradient and a middle over it.** The engine draws an arc of a given sweep only through an SVG (`components::Svg`, behind the `svg` feature, off in the gallery and in the tests) or a canvas (behind `canvas`, and needing a `Ctx`). Core has a conic gradient, clockwise from the top: the disc is the fill's color up to the value and the track's after it, with two stops at the value so the arc ends square, as the board's sector does. The middle is four fifths across (the board's inner radius of 0.8), painted in `Background`.
- **To tooling** the root says `progressbar` with `a11y_valuenow`, `a11y_valuemin`, `a11y_valuemax`, and `a11y_valuetext` from `format`, so a reader hears "42 of 128" where the app says it so.
- **A meter and a gauge are not built.** A meter is a progress restyled (the gallery's "As a meter"); the board's meter of 16 cells is a renderer that draws cells from `view.fraction()`. A gauge is half a ring: a ring renderer with a conic gradient from 270°, clipped to its top half.

## Not done

- **The ring's arc moves to a new value at once.** A gradient is not animatable. Smallest change: an `AnimatableProperty::BackgroundGradient` that lerps the stops of two gradients of one kind and one count of stops (positions and colors), read from the node's background gradient and written as an override the quad reads; the disc then takes `Transition::background_gradient()` in the recipe.
- **The ring's middle is painted, not cut out**, so on a surface of another color than `Background` the app gives `RingProgressPart::Middle` that color. Smallest change: an arc in core, not behind a feature: a border whose color may be a gradient (`Border` taking a `Gradient` as `BackgroundColor` does through `background_gradient`), so that the track's inside border of a tenth of the side carries the conic gradient and the middle is nothing.
- **The top bar's fill is round at its start too** (corners of 2 all round; the board's start is square). A part has one radius for every corner. Smallest change: `PartStyle::radius` taking a `ThemedBorderRadius` as well (a radius for each corner), applied with `NodeUpdate::rounded`; the recipe then gives the top bar's fill `ThemedBorderRadius::new(0.0, 2.0, 2.0, 0.0)`.

## Differs from the board

- The top bar's fill is rounded 2 at its start as well as its end; the board's start is square (under "Not done").
- The ring's middle is painted in `Background`, where the board's ring is hollow (under "Not done").
- The board draws one ring, of 72; the rings of 56 and 40 are lurq's, a tenth of their side wide.
- The board draws the linear loader as a still frame, a segment of 90 at 70 from the start of a rail of 4; lurq's crosses the rail, and `Progress::indeterminate()` is 8 high unless given `ProgressSize::Sm`, as any progress is.
