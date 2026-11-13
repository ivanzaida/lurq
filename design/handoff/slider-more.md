# Slider, vertical and range: hand-off

`ui::Slider::orientation` (board: `Slider/Vertical`, `xGvSx`, in "Slider variants" of `Slider & Progress`) and
`ui::RangeSlider` (board: `Range Slider`, `fujuW`, in "Small" of `Specialized Inputs`), with what the engine's
slider needed for them. Both are built; the forms batch had left them open because the engine's slider was
horizontal and of one value. A second round closed what the first listed as not done: Home, End, Page Up and
Page Down on every slider, a range slider in a form, and two numbers for a range in the MCP server's
`lurq_set_value`.

## Changed outside the component's own files

This is engine work. The range slider is the engine's `Slider` node with a second value, not a node kind of its
own: every lookup, the hover, the focus, the disabled state, the styles, the resources, devtools and the MCP tools
take a `NodeKind::Slider` already, and they keep working with a range unchanged.

- `node/slider_style.rs`: `pub enum SliderOrientation { Horizontal, Vertical }`, re-exported from `lurq::node` (and
  from `lurq::ui`).
- `node/node_kind/slider.rs`: `SliderState` holds an orientation and the active thumb (`SliderThumb::Low`/`High`),
  and its value can be `Signal<(i32, i32)>` or `Signal<(f32, f32)>`, the low value first. A thumb's value stops at
  the other's (`set_value_of`); the range and the step snap both values and keep them in order. `part_rects`
  returns `SliderParts`: the track, each thumb, the fill (`fill(thickness)`: from the start to the thumb, from
  the thumb down to the bottom of a vertical one, or between a range's thumbs) and the axis a thumb travels
  (`SliderAxis`, which turns a point into a ratio along the track either way). The preferred size of an unsized
  vertical slider is 20 by 120 (the horizontal one's turned). The orientation is in the layout signature.
- `node/node.rs`, `node/node_kind.rs`, `node/mod.rs`: `Node::range_slider`, `Node::range_slider_f32`,
  `slider_orientation` (builder and `NodeUpdate`), and the exports.
- `components/slider.rs`: `Slider::new_range`, `Slider::new_range_f32`, `orientation(..)`, `vertical()`.
- `layout/layout_engine.rs`: the slider paints its fill and each thumb from `SliderParts`; a vertical fill's
  thickness is its style's `width`. Of a range, only the active thumb takes the focused style.
- `app/runtime.rs`:
  - A press takes the nearer thumb and the drag keeps it to its end (`SliderDrag` holds the thumb and the axis,
    and is found again by its binding with the same thumb). A finger's drag holds the thumb from where it is
    along either axis, as before.
  - `find_slider_by_y_recursive` is `find_slider_in_lane_recursive(.., x, y)`: with nothing under the pointer, a
    press in a horizontal slider's rows or a vertical slider's columns is a press on it; a disabled slider is
    still never found.
  - `dispatch_builtin_pointer` takes `y` too.
  - `focus_tab`: Tab on a focused range moves from its low thumb to its high one before it leaves the slider,
    Shift+Tab from the high to the low; a slider reached by Tab is entered at its low thumb going forward and its
    high one going back.
  - The arrow keys are as they were (right and up raise, left and down lower), so a vertical slider needed nothing;
    of a range they move the active thumb.
  - Page Up and Page Down move the focused thumb a tenth of the range in whole steps, at least one
    (`SliderState::page`); Home and End take it to the least and the greatest value (`SliderState::to_end`). A
    range's thumb stops at the other's value, as it does for the arrows.
  - A range slider in a form submits two entries under its name, the low value then the high one
    (`SliderState::value_strings`); a slider of one value submits one, as before.
  - `SliderHandle::values()` is new; `value()` is a range's low value; `set_from_ratio` now does what a press does
    (of a range, the nearer thumb moves; a slider of one value is set as before).
- `components/form/`: a range is a field of its own kind.
  - `value.rs`: `FormValue::Range(f64, f64)`, `as_range()`, from `(f64, f64)` and `(i32, i32)`;
    `FormValues::get_range`.
  - `handle.rs`: `FormHandle::range(name) -> Signal<(f64, f64)>`, `range_field`, `range_control`. A submit reads the
    two entries under the name into the field; a reset, a field's reset, dirtiness and the touched state of a submit
    take ranges as they take numbers.
  - `options.rs`: `FormOptions::validate_range(name, Fn(Option<(f64, f64)>, &FormValues))`.
  - `validators.rs`: `validators::range_span(min, max, span, message)`: both values between `min` and `max`, and at
    least `span` apart.
  - `compound/form_range_slider_input.rs`: `FormRangeSliderInput` and `FormRangeSliderInputProps`, the counterpart
    of `FormSliderInput`: the field's label, hint and error around the engine's range slider bound to a range
    control, validated when it loses the focus. Exported from `lurq::components` beside `FormSliderInput`.
- `app/ctx.rs`: `ctx.range_control(name)`, beside `number_control`.
- `mcp/tools/set_value.rs`, `mcp/registry.rs`: `lurq_set_value` on a range slider takes `[low, high]` and sets
  both, snapped and put in order; one number still moves the nearer thumb. A slider of one value refuses an array.
  The tool's description and its `value` schema say so.
- `ui/mod.rs`: the exports of `RangeSlider` and `SliderOrientation`.
- `tests/input/mod.rs`: `slider::range`, `slider::vertical`, `slider::keys` and `forms::range_slider`, with their
  files beside the other slider and form tests. `mcp/tools/tests.rs` has a test of `lurq_set_value` on a range.

### For `docs/src/content/docs/styling-events.md`

At the end of "Slider Styling", after the image-backed parts:

#### Vertical And Range Sliders

`.orientation(SliderOrientation::Vertical)` (or `.vertical()`) runs the track up the slider: the least value is at the bottom, a drag goes along the height, and the up and down arrows move it as the right and left ones do. The parts are laid along the height, so a vertical track's style gives its thickness as `width` and leaves `height` to the slider's, and a fill's `width` is its thickness. An unsized vertical slider asks for 20 by 120.

```rust
use lurq::{components::Slider, core::Signal, node::SliderOrientation};

let level = Signal::new(60);

Slider::new(level)
  .range(0, 100)
  .orientation(SliderOrientation::Vertical)
  .height(160.0)
  .track(|style| style.width(4.0).background("#334155"))
  .fill(|style| style.background("#f97316"))
  .thumb(|style| style.size(14.0, 14.0).background("#f8fafc"));
```

`Slider::new_range(Signal<(i32, i32)>)` and `Slider::new_range_f32(Signal<(f32, f32)>)` put two thumbs on one track, bound to a low value and a high one, with the fill between them. A press on the track moves the nearer thumb there (of two at one place, the one on the side of the press), a drag moves the thumb it began on to its end, and neither thumb passes the other: the low one stops at the high one's value. Each thumb is a stop of Tab, the low one first going forward and the high one first going back, and the keys move the focused thumb, which alone takes the `thumb_focused` style. Both thumbs take the same `thumb` style. A range slider can be vertical too.

```rust
let price = lurq::core::Signal::new((20, 80));
lurq::components::Slider::new_range(price).range(0, 100).step(5);
```

#### Slider Keys

| Key | |
| --- | --- |
| Right, Up | One step toward the greatest value |
| Left, Down | One step toward the least |
| Page Up, Page Down | A tenth of the range, in whole steps and at least one step, as a browser's range input pages |
| Home, End | To the least value, to the greatest |

The keys are the same for a vertical slider, and for the focused thumb of a range slider, which stops at the other thumb's value. A disabled slider is not a stop of Tab and takes no key.

In a form a range slider submits two entries under its name, its low value and then its high one. `SliderHandle::values()` reads both values, `value()` the low one, and `set_from_ratio` moves the thumb nearer the ratio, as a press does.

### For `docs/src/content/docs/forms.md`

Under "Field Signals", after the paragraph on `FormSliderInput`:

A range slider's field is a pair: `form.range(name)` returns `Signal<(f64, f64)>`, the low value first, and `FormRangeSliderInput` bridges it to the engine's range slider as `FormSliderInput` does a number. The form submits the two values under the field's name, validates the field when the slider loses the focus, and a reset puts both thumbs back at the default.

```rust
use lurq::components::{FormOptions, FormRangeSliderInput, FormRangeSliderInputProps, validators};

let form = ctx.form(
  FormOptions::new()
    .field("price", (20, 80))
    .validate_range("price", validators::range_span(0.0, 100.0, 10.0, "At least 10 apart")),
);

ctx.mount::<FormRangeSliderInput>(
  FormRangeSliderInputProps::new(form.range_control("price")).label("Price").range(0, 100),
)
```

In "Built-in validators", a row: `validators::range_span(min, max, span, message)`, with `validate_range`: both values between `min` and `max`, and at least `span` apart.

In "Form Context Controls", a row: `ctx.range_control(name)`, "Resolve a range field control from the nearest form."

In "FormValues", a row `get_range(name)` returning `Option<(f64, f64)>`; and `FormValue` has a `Range(f64, f64)` variant too, from `(f64, f64)` and `(i32, i32)`.

### For `docs/src/content/docs/mcp.md`

The row of `lurq_set_value`: "Set a TextInput / Checkbox / Slider / Select value directly, no keystroke simulation. A range slider takes `[low, high]` and sets both; one number moves its nearer thumb."

## For `docs/src/content/docs/design-system.md`

## Slider

A value picked along a track, between a least and a greatest: volume, the size of a brush, a price no higher than. A range slider picks two on one track: a price between, the hours a shop is open.

```rust
use lurq::ui::{RangeSlider, Slider, SliderOrientation};

// volume: Signal<i32>
Slider::new(volume.clone()).label("Volume");

// A fraction, in steps of a twentieth.
Slider::new_f32(opacity.clone()).step(0.05).label("Opacity");

// From 10 to 50 in steps of 5.
Slider::new(size.clone()).min(10).max(50).step(5);

// Up, the least value at the bottom; 100 high.
Slider::new(level.clone()).orientation(SliderOrientation::Vertical).length(100.0);

// price: Signal<(i32, i32)>, the low value first.
RangeSlider::new(price.clone()).step(5).label("Price");
```

| Builder | |
| --- | --- |
| `Slider::new(Signal<i32>)`, `Slider::new_f32(Signal<f32>)` | The value it is bound to: whole numbers, from 0 to 100 in steps of 1, or fractions, from 0 to 1 in steps of a hundredth |
| `RangeSlider::new(Signal<(i32, i32)>)`, `RangeSlider::new_f32(Signal<(f32, f32)>)` | The two values a range slider is bound to, the low one first, in the same ranges and steps |
| `min(v)`, `max(v)`, `step(v)` | The least value, the greatest and how far apart the values are, from the least. Any number: `min(10)`, `step(0.25)` |
| `orientation(SliderOrientation)` | `Horizontal`, or `Vertical`: 16 wide and 140 high, the least value at the bottom |
| `length(f32)` | How long the slider is along its track: its width across, its height up |
| `label(&str)` | What the slider is called, to what reads the tree. The slider shows no text: the field it is in does |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the slider's node |

- It is the engine's [`Slider`](../styling-events/#slider-styling) dressed by the recipe: a drag of the thumb, a press on the track and the keys set the signal, snapped to the step and kept in the range. Tab reaches it wherever it is (`tab_index(0)`), and the focus ring is the thumb's. A vertical slider is dragged along its height, and the up and down arrows move it.
- The keys: the arrows a step (right and up toward the greatest value), Page Up and Page Down a tenth of the range in whole steps and at least one, Home and End to the least and the greatest. The [engine's slider keys](../styling-events/#slider-keys) are the same for a vertical slider and for each thumb of a range slider.
- A range slider is the engine's slider with two thumbs. A press on the track moves the nearer thumb, a drag the thumb it began on, and neither passes the other, by the pointer or the keys. Each thumb is a stop of Tab, the low one first; the keys move the one focused, and the ring is around it. In a form it is `FormRangeSliderInput` (see [Forms](../forms/)).
- Parts: `SliderPart::Root` (the slider's node: its size, its cursor, its opacity), `Track`, `Range` (the track from its start to the thumb, or between a range slider's thumbs), `Thumb` (both of a range slider's). States: `SliderState::Disabled`, `Vertical`, `Range`. A range slider is drawn by the slider's recipe and renderer in the state `Range`, so a theme that restyles the slider restyles it too, and `rules.state(SliderState::Range, ..)` styles it alone.
- In the design the slider is 240 by 16; the track is 4 high and round, in `Secondary`; the range is `Primary`; the thumb is 16, round, in `Background` with an edge of 1.5 in `Primary` inside it. The vertical one is 16 by 140, its track 4 wide and as high as it. A disabled slider is dimmed. `style(SliderPart::Root, |s| s.width(Length::Fill))` makes it as wide as what holds it.
- The engine paints the track, the range and the thumb, not nodes: of their style it takes the width and the height in pixels, the fill, the corners, the border and the outline (`focus_ring()` too), at rest, under the pointer (`hovered`), and for the thumb focused from the keyboard (`focus_visible`). An opacity, a shadow or a transition of one of those three parts is not painted. On a vertical slider a part's width is its thickness.
- To what reads the tree (`lurq_snapshot` of the MCP server) it is a `slider` called by its label; a range slider's value is `low,high`.
- A renderer that draws another slider starts from `view.slider()`, the engine's slider bound to the value (or the two) with its orientation, the root's style and the length, and `view.dressed(slider)` gives it the recipe's track, range and thumbs. `view.value()` reads the value, `view.values()` a range slider's two, and `view.is_range()` tells which it is.

## For `CHANGELOG.md`

- `ui::Slider::orientation` and `length`, and `ui::RangeSlider`: the board's vertical slider and range slider. The engine's `Slider` has `orientation(SliderOrientation)` (`vertical()`), laid out, painted and dragged along its height with the least value at the bottom, and `Slider::new_range` / `new_range_f32`: two thumbs on one track bound to a `Signal<(i32, i32)>` or `Signal<(f32, f32)>`, the fill between them, a press moving the nearer thumb, the thumbs not crossing, each a stop of Tab. Every slider takes Home and End (to its ends) and Page Up and Page Down (a tenth of the range). In a form a range slider is `FormRangeSliderInput`, a `FormHandle::range` field submitted as two values, with `validate_range` and `validators::range_span`; `lurq_set_value` sets both values from `[low, high]`. `SliderHandle::values()` reads both; `set_from_ratio` moves the nearer thumb of a range.

## Decided

- A range is the same node with a second value: it changes less of what exists than a node kind of its own, which
  every lookup, hover, focus, disabled, style and tooling path would have had to learn.
- One `Signal<(i32, i32)>`, not two signals: one binding, which a drag finds the slider by again after a render
  replaces its node, and one write that keeps the low value below the high one. The low value is first.
- The thumbs stop at each other, they do not swap: a drag of the low thumb past the high one leaves both at the
  high one's value. Of two thumbs at one place, a press beyond them takes the one on that side, so they part; a
  press right on them takes the active one.
- Tab: one node with two stops, the active thumb in the slider's state. Tab from the low thumb goes to the high
  one, then on; Shift+Tab the other way, entering at the high one. The focus ring is the active thumb's.
- The arrow keys of a vertical slider are the engine's as they were (up raises, down lowers), not reversed.
- `ui::RangeSlider` has no `Themed` of its own: it is drawn by `Slider`'s recipe and renderer, in the state
  `SliderState::Range`. The board draws it from the slider's parts, and a theme would otherwise have to restyle
  both. A renderer tells one from the other by `view.is_range()`.
- The vertical slider is `orientation(SliderOrientation::Vertical)` and the state `SliderState::Vertical` in the
  recipe (Root 16 by 140, Track 4 wide with its height undone by `Length::Auto`), not a component of its own.
- `length` is a prop of both (the board's 240 and 140 when not given): a vertical slider's length is its height,
  which `style(Root, ..)` would set with an orientation in mind.
- `SliderOrientation` is the engine's type, re-exported by `lurq::ui`.
- The gallery's playground has an `orientation` select; the page has a "Vertical" section (three channels, the
  last with `length(100.0)`), a "Range slider" section (a price in steps of 5, opening hours to 24, a disabled
  one), and a "Keyboard" section of what each key does.
- A page (Page Up, Page Down) is a tenth of the range rounded to whole steps, and at least one step; not ten
  steps. The engine's slider always has a step (1, or a hundredth), so "where there is no step" never happens, and
  ten steps would cross a slider of 0 to 10 in one press and hardly move one of 0 to 1000. A tenth of the range is
  what a browser's range input pages by, and ten presses always cross the track.
- Home and End are absolute and Page Up and Page Down relative, as the arrows are; none of them wraps, and a
  range's thumb stops at the other.
- A range in a form is a field kind of its own (`FormValue::Range`, `FormHandle::range`) rather than two number
  fields: one name, one value to validate and to reset, as the slider has one binding. It is submitted as two
  entries under the name, the form data's way of holding several values for one name (a select of many does the
  same), and read back into the field as a pair. A range slider in a form with no `range` field for its name
  submits its entries as the form keeps any other unregistered field: as text, the last one kept.
- `lurq_set_value` keeps one number for the nearer thumb, and takes `[low, high]` for both; a pair is snapped and
  ordered, as the form's values are. An array for a slider of one value is an error, not its first number.

## Not done

## Differs from the board

- The board's vertical slider draws its track from the top to the thumb and its range from the thumb to the
  bottom, each with two round corners; the engine paints the whole track and the range over it from the thumb's
  middle, both round at each end. Under the opaque thumb they look the same.
- The board's range slider draws the range from one thumb's edge to the other's and the track in two pieces
  beside them; the engine paints the whole track and the range between the thumbs' middles, under them. They look
  the same.
