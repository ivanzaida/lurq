# Slider: hand-off

`ui::Slider`, the horizontal slider of one value (board: `IdugJ`, in "Slider & Progress" of `Components / light`).

## Changed outside the component's own files

The engine's slider paints its parts itself, from a `SliderPartStyle` each, and that style had no ring: the board's
focus ring (two wide, two from the edge, `Ring`) could not be put around the thumb without replacing the thumb's
own edge. So:

- `node/slider_style.rs`: `SliderPartStyle` has an `outline: Option<Outline>` and a builder `outline(Outline)`,
  merged like its other fields. A part's outline is a ring around it, as a node's is.
- `layout/layout_engine.rs`: the slider paints the outline of its track, its fill and its thumb, if they have one.
  The thumb's focused style (`thumb_focused`) can now be a ring.
- `layout/layout_engine/outline_quads.rs`: the end of `push_outline_quad` is a method of its own, `push_ring`,
  which a node's outline and a slider part's ring both go through: kept with the pending outlines, painted over
  what comes after the node, with the clip let out by the ring's reach.

Nothing else of the engine changed. The engine's `Slider` builder and its docs are as they were; its own
`.thumb_focused(|s| s.outline(..))` now works too, which `styling-events.md` could say.

## For `docs/src/content/docs/design-system.md`

```md
## Slider

A value picked along a track, between a least and a greatest: volume, the size of a brush, a price no higher than.

```rust
use lurq::ui::Slider;

// volume: Signal<i32>
Slider::new(volume.clone()).label("Volume");

// A fraction, in steps of a twentieth.
Slider::new_f32(opacity.clone()).step(0.05).label("Opacity");

// From 10 to 50 in steps of 5.
Slider::new(size.clone()).min(10).max(50).step(5);
```

| Builder | |
| --- | --- |
| `Slider::new(Signal<i32>)`, `Slider::new_f32(Signal<f32>)` | The value it is bound to: whole numbers, from 0 to 100 in steps of 1, or fractions, from 0 to 1 in steps of a hundredth |
| `min(v)`, `max(v)`, `step(v)` | The least value, the greatest and how far apart the values are, from the least. Any number: `min(10)`, `step(0.25)` |
| `label(&str)` | What the slider is called, to what reads the tree. The slider shows no text: the field it is in does |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the slider's node |

- It is the engine's [`Slider`](../styling-events/#slider-styling) dressed by the recipe: a drag of the thumb, a press on the track and the arrow keys set the signal, snapped to the step and kept in the range. Tab reaches it wherever it is (`tab_index(0)`), and the focus ring is the thumb's.
- Parts: `SliderPart::Root` (the slider's node: its size, its cursor, its opacity), `Track`, `Range` (the track from its start to the thumb), `Thumb`. State: `SliderState::Disabled`.
- In the design the slider is 240 by 16; the track is 4 high and round, in `Secondary`; the range is `Primary`; the thumb is 16, round, in `Background` with an edge of 1.5 in `Primary` inside it. A disabled slider is dimmed. `style(SliderPart::Root, |s| s.width(Length::Fill))` makes it as wide as what holds it.
- The engine paints the track, the range and the thumb, not nodes: of their style it takes the width and the height in pixels, the fill, the corners, the border and the outline (`focus_ring()` too), at rest, under the pointer (`hovered`), and for the thumb focused from the keyboard (`focus_visible`). An opacity, a shadow or a transition of one of those three parts is not painted.
- To what reads the tree (`lurq_snapshot` of the MCP server) it is a `slider` called by its label.
- A renderer that draws another slider starts from `view.slider()`, the engine's slider bound to the value with the root's style, and `view.dressed(slider)` gives it the recipe's track, range and thumb. `view.value()` reads the value, for one that shows it.
```

## For `CHANGELOG.md`

- `ui::Slider`: a value picked along a track. The engine's slider dressed by a recipe, bound to a `Signal<i32>` or a `Signal<f32>` with `min`, `max` and `step`, the parts `Root`, `Track`, `Range`, `Thumb` and the state `Disabled` for a recipe, and `SliderView::slider()`, `dressed()` and `value()` for a renderer. The engine's `SliderPartStyle` has an `outline`: a ring around a slider's part, the thumb's focus ring.

## Decided

- The ring of the focus is the thumb's (the board draws no focused slider): the theme's focus ring around the thumb,
  over its edge, shown for the keyboard only. It is the engine's `thumb_focused` style, so it changes no size.
- The range is the engine's fill: it runs from the start of the track to the thumb's middle, under the thumb. The
  board draws it to the thumb's edge and the track from the other edge; with an opaque thumb they look the same.
- The default range of a slider of whole numbers is 0 to 100, not the engine's 0 to 1; a slider of fractions keeps
  the engine's 0 to 1 in steps of a hundredth.
- `min`, `max`, `step` take any number (`impl Into<f64>`), so one builder serves both kinds of value.
- No `on_change`: the slider is bound to the app's signal, as the engine's is. An app that wants to be told watches
  the signal (the gallery's playground does, with `ctx.watch`).
- `label` names the slider to tooling and shows nothing: the board's slider has no text of its own.

## Not done

- `Slider/Vertical` (`xGvSx`): not the same component with one prop. On the board it is a separate component, and
  the engine's slider is horizontal only (`SliderState::part_rects`, the pointer's ratio, the fill): a vertical one
  needs an orientation in the engine first.
- `Range Slider` (`fujuW`) and the angle slider: not asked for.
- Marks: the board's "with marks" is the slider with a row of text under it, which the gallery page builds; the
  marks are not part of the component, and they line up with the steps only roughly (the thumb's middle runs 8 in
  from each end).
- A hover or pressed look: the board draws none. The engine paints a part's hovered style while the pointer is over
  the slider or dragging, so a theme can give one; there is no hook for "pressed".
- Tests were written, not run (the batch is tested when merged). Most likely to need a look:
  `a_press_on_the_track_sets_the_value_where_it_is_and_a_drag_follows_the_pointer` (the drag in the headless
  stage) and the exact position of the ring in `a_forced_focus_draws_the_ring_around_the_thumb`.

## Differs from the board

- Disabled is the theme's `OpacityLevel::Disabled` (0.5), as for every component; the board dims the slider to 0.45.
- The track's and the range's ends are both round (radius `Full`, 2 on a track of 4); the board squares the ends
  that meet the thumb, which the thumb covers.
- The cursor over the slider is a pointer, the default while disabled; the board does not say.
