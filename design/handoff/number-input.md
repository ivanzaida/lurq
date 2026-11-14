# Number Input: hand-off

`ui::NumberInput`, from the board's `Number Input` (`ROx7j`) in Components / light (`Ovt1D`, Inputs › Composite ›
Compact, `pg0qN`) and `Mobile/Quantity Stepper` (`lMTuf`) in Mobile / light (`jwTe9`, Touch Controls › Selection ›
Stepper). The board draws each once, at rest, and nowhere else: no sizes, no unit, no disabled or invalid one, no
other place of its buttons. What it does not draw is decided below.

Files: `crates/lurq/src/ui/number_input.rs`, `crates/lurq/tests/ui/number_input.rs`,
`crates/gallery/src/pages/number_input.rs` (page "Number Input", group Inputs), `design/handoff/number-input.md`, and
lines in the three `mod.rs` files.

## Changed outside the component's own files

This session owns what the engine's text input takes as it is typed and the keys that step a value. Two engine
changes, and one in shared code:

- **A press that repeats while it is held.** `crates/lurq/src/node/node.rs`: `EventHandlers` has a field more,
  `press_repeats: bool`. `crates/lurq/src/components/mod.rs`: every element has `.repeat_press(bool)`.
  `crates/lurq/src/app/runtime/press_repeat.rs` (new) and `crates/lurq/src/app/runtime.rs`: a press of the primary
  button on such an element runs its `on_click` handlers at once, focuses what its `focuses` names, and repeats them
  while the button is held and the pointer is over it, first after 400 ms and then every 50 ms
  (`REPEAT_DELAY`, `REPEAT_INTERVAL`); a pointer that leaves pauses it, as does an element disabled meanwhile (a step
  to the end of a range), and the release (or `mouse_press_taken_by_os`) ends it and clicks nothing. The runtime
  keeps the held press in `Tree::press_repeat`, ticks it in `tick_timers_at`, counts it in
  `has_active_tick_sources` and gives its time to `next_timer_at`, so the shell wakes for it as for a timer.
  `crates/lurq/src/app/runtime/hover.rs`: `Tree::pointer_position()`, where the pointer last was. Needed because a
  builder has no `Ctx` and so no timer of its own. `EventHandlers` is public with public fields: the CHANGELOG's
  Breaking list wants "`EventHandlers` has a field more, `press_repeats`: one written out field by field must add
  it."
- **A text input that keeps what is half typed.** `crates/lurq/src/components/text_input.rs`:
  `TextInput::keep_text_while_focused(bool)`. `crates/lurq/src/node/node_kind/text_input/mod.rs` keeps the flag;
  `geometry.rs`, `copy_runtime_state_from`: when a render binds a focused input that has it to another signal, the text
  it showed is written into the new signal. A number input makes its text anew from the value in each build, and a
  value that changes as it is typed renders it again: without this, `1.` became `1` under the caret, and `-` nothing.
  The input needs a key (the number input gives its control one) so the render finds it again.
- `crates/lurq/src/ui/text_box.rs`: `text_box_as(control, states, handlers, role)`, `text_box` with another role to
  tooling (`spinbutton`); `text_box` calls it with `textbox`. `describe` adds an attribute rather than replacing one,
  so the role could not be said over `text_box`'s.

## For `docs/src/content/docs/design-system.md`

## Number Input

A number a person types or steps: a quantity, a price, a size in pixels.

```rust
use lurq::ui::{NumberFormat, NumberInput, NumberInputLayout};

// A number there always is, between 1 and 99.
NumberInput::new(quantity.clone()).min(1.0).max(99.0).label("Quantity");

// A price with two decimals, its thousands set apart, the buttons stacked at the end.
NumberInput::new(price.clone())
  .min(0.0)
  .step(0.5)
  .precision(2)
  .format(NumberFormat::new().prefix("$").grouping(true))
  .layout(NumberInputLayout::Stacked);

// A number that may be left out: an emptied field is None, not 0.
NumberInput::optional(limit.clone()).placeholder("No limit");

// The app's own value, told back.
NumberInput::controlled(seats.get()).on_change(move |value| seats.set(value));
```

| Builder | |
| --- | --- |
| `NumberInput::new(Signal<f64>)` | Bound to a number there always is: an emptied field takes back the number it had when it is left |
| `NumberInput::optional(Signal<Option<f64>>)` | Bound to a number that may be none: an emptied field sets `None` |
| `NumberInput::controlled(Option<f64>)` | Shows what the app gives it and tells each change to `on_change` |
| `min(f64)`, `max(f64)` | The ends of the range, where it has them. With no least value a minus can be typed and Home moves the caret; with no greatest End does |
| `step(f64)` | How far apart the values a step takes it to are, from the least (from 0 with none): 1 when not given |
| `precision(usize)` | How many decimals the value keeps and shows: as many as the step has when not given. With none no decimal separator is taken |
| `format(NumberFormat)` | How the number is written out of the focus: `prefix`, `suffix`, `grouping`, `separators(decimal, group)`, or `NumberFormat::custom(format, parse)`, the app's own |
| `layout(NumberInputLayout)` | `Split` (the board's: a minus at the start, a plus at the end, the number in the middle), `Stacked` (a chevron up over one down at the end, the number at the start), `Bare` (no buttons) |
| `size(NumberInputSize)` | `Sm` 32, `Md` 36, `Lg` 40 high, the type 13, 14, 15 |
| `placeholder(&str)` | What an empty input says |
| `invalid`, `warning`, `valid`, `read_only` | As an [input](#input) has them. A read-only one is focused and selected, and its keys and buttons change nothing |
| `on_change(Fn(Option<f64>))` | Called with the value each time it changes |
| `name(..)` | The name of its text in a form's data. A form's number is bound with `NumberInput::new(form.number("seats"))` |
| `label`, `full_width`, `disabled`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As an [input](#input) has them. `id` is the id of the box that is typed in |

- What is typed is kept as text while it is typed, and is the value as soon as it is a number. A minus (first, where the range has room below 0), digits and one decimal separator (where the precision keeps decimals) are taken; anything else is not. A number pasted as it is written (`$2,000`) is read and typed as the number. On Enter and when the focus leaves, the text is put right: the number rounded to the precision, kept in the range and written in the format; what is not a number goes back to the value.
- In the focus the number is typed plain, with the decimal separator and nothing around it; out of it, it is written in the format. A locale's decimal separator is the format's: `NumberFormat::new().separators(',', '.')`.
- Up and Down take a step, Page Up and Page Down ten, Home and End go to the least and the greatest value where the range has them. A step from a value between two steps goes to the next one; from an empty field it goes to 0, or to the end of the range nearer to it. A wheel over the input scrolls what it is in and leaves the number as it is.
- The buttons take a step when pressed, and while held another after 400 ms and then every 50 ms. They are not stops of Tab and take no focus: a press leaves the focus on the number being typed. The one toward an end of the range the value is at is disabled and dimmed (states `AtMin`, `AtMax`).
- Parts: `NumberInputPart::Root` (its width), `Control` (the box that is typed in, which holds the ring of the focus), `Placeholder`, `Buttons` (what holds the buttons over the box: its inset and corners), `Decrease`, `Increase`, `Divider` (the line between a button and the number), `Icon`. Variants: `NumberInputLayout`. States: `NumberInputState::Invalid`, `Warning`, `Valid`, `ReadOnly`, `Disabled`, `FullWidth`, `AtMin`, `AtMax`.
- In the design the box is 132 by 36, an input's box (the `Background` fill in an edge of 1 in `Input`, 6 round), its number in the middle in the mono type at 14 in `Foreground`. The minus and the plus are 36 wide at the ends, each with a line of 1 in `Input` between it and the number, their glyphs 16; under the pointer a button is in `Highlight`. The ring's color under the pointer, a line of 2 inside the box in the focus, and the judgments' colors are the input's.
- The board's mobile quantity stepper is the number input restyled: rules make the `Control` 44 high, round, in `Secondary` with no edge, the number in the sans type at 16, semibold; the `Buttons` with no inset and round; `Decrease` and `Increase` 44 wide; the `Divider` 0 wide; the `Icon` 20. The gallery's page has them.
- To what reads the tree (`lurq_snapshot` of the MCP server) the box is a `spinbutton` called by its label, with `a11y_valuenow`, `a11y_valuetext` (as it is written), `a11y_valuemin` and `a11y_valuemax`; the buttons are `button`s called "Increase" and "Decrease", with `a11y_disabled` at an end of the range.
- A renderer that draws another number input has `view.control()` (the box with all of its typing, keys and putting right), `view.button(part, icon, steps)`, `view.divider()`, `view.buttons()` (laid over the box as the layout has them) and `view.buttons_reach()` (how far they reach in from each end).

## For `CHANGELOG.md`

- `ui::NumberInput`: a number typed or stepped, bound to a `Signal<f64>`, a `Signal<Option<f64>>` (an emptied field is none) or the app's own value, with a range, a step, a precision, a format (`ui::NumberFormat`: prefix, suffix, grouping, a locale's separators, or the app's formatter and parser), the board's minus and plus at its ends or a stacked pair at its end, the arrow keys, Page Up and Page Down, Home and End, and buttons that repeat while held; every element has `repeat_press`, a press that runs its click again while it is held, and `TextInput::keep_text_while_focused` keeps what is half typed when a render binds the input to another signal.

## Decided

- **A builder, with two engine changes in place of a mounted component.** What is typed is kept as text, and a held button repeats on a clock; a builder has no `Ctx` for either, and a mounted component could not go where a `FieldControl` goes (`Field::new(.., control)` has no `Ctx` to mount it with). So the engine keeps the text of a focused input across renders (`keep_text_while_focused`) and repeats a held press (`repeat_press`), and the input stays a builder like `ui::Input`.
- **The control is the box**, as `ui::Input`'s is: the ring of the focus is the box's own outline, around the whole input, and the buttons are laid over its ends in a row that lets the pointer through elsewhere. The board's buttons carry their line as an inner stroke on one side; a part has a border on every side or a line at its start or bottom only, so the line is a part of its own, `Divider`. The buttons stand 1 in from the box's edge so the edge stays whole under a lit button: 34 wide with the edge and the line, 36 at the medium size as the board has them (30 and 38 at the small and the large).
- **The board's layout is the default, `Split`.** `Stacked` (chevrons at the end, one over the other) is what desktop inputs commonly have, and `Bare` is for a field where the keys are enough; the board draws neither.
- **A button under the pointer is in `Highlight`**, as a ghost button's is; the board draws no hover. The one toward an end reached is dimmed with the theme's disabled opacity and takes nothing.
- **The buttons take no focus.** They are `focusable(false)`, so a press of one leaves the focus where it was: on the number being typed (the keyboard stays up on a phone, and the text stays typed), or nowhere. They are not stops of Tab; the keys do what they do.
- **No wheel.** A number input that a wheel changes changes as a page is scrolled under the pointer; browsers have moved away from it, and the board says nothing. The keys and the buttons step it.
- **The value is written as it is typed, not kept in the range until it is put right**: typing 150 toward 100 passes through 1 and 15, and clamping those would stop the typing. Enter and the focus leaving clamp and round. Pasted text that is a number as written is read and typed as the number, rounded, where that number could have been typed.
- **Precision from the step**: a step of 0.25 keeps two decimals, a whole step none (and then takes no separator), unless `precision` says.
- **A step goes to the step's grid from the least value** (from 0 with none), as a browser's number input does: from 2.5 in steps of 1, up is 3 and down 2. A step from an empty field goes to 0, kept in the range.
- **The format is shown out of the focus, the plain number in it.** Typing a prefix, a suffix or a group separator would need the input to keep them in place as the caret moves; typing the number alone and writing it out on leaving is what spreadsheets do. `NumberFormat::custom` formats what is shown and parses what it wrote (and what is pasted); what is typed is read with the format's decimal separator.
- **In a form** the number is bound to the form's signal (`FormHandle::number`), as a slider's is. `name` puts the text on the form's data (`FormData`) as it is shown; the form's values (`FormValues`) have the number.
- **An app's value set while the input has the focus** is shown once the focus leaves: while it is typed in, the text is the person's.

## Not done

Nothing.

## Differs from the board

- The board's lines between the buttons and the number are the buttons' own inner strokes; here they are a part of their own (`Divider`), at the same places.
- The board's glyphs are drawn with a stroke of 1.333 (16) and 1.667 (20); the theme's icon font has its own weight.
- Sizes, `Stacked`, `Bare`, a button under the pointer, disabled buttons at the ends of the range, invalid, disabled, read only and the format are not on the board: decided above.
