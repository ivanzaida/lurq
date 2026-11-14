# Input OTP

## Changed outside the component's own files

- `crates/lurq/src/animation/animation.rs`: `AnimationEngine::new` registers the code's caret blink (`crate::ui::input_otp_keyframes()`), not in phase, so that a caret starts shown in each cell it moves to. Its identifier is `0xFF41` (`ui::INPUT_OTP_CARET_BLINK`): the last registered before was `0xFF34`, and `0xFF35` to `0xFF40` are left to the other sessions of the batch. If another component of the batch took `0xFF41` as well, one of the two moves.
- `crates/lurq/src/ui/mod.rs`, `crates/lurq/tests/ui/mod.rs`, `crates/gallery/src/pages/mod.rs`: the module, its exports and the gallery's page entry (after Select in `Group::Inputs`), nothing else.
- No change to the engine's text input, focus or paste: the code is one engine `TextInput`, and every edit is made by the component's handlers on it.

## For `docs/src/content/docs/design-system.md`

## Input OTP

A one-time code: a few characters typed one to a cell, the code a message or an authenticator app gives to prove who a person is.

```rust
use lurq::ui::{Field, InputOtp, InputOtpKind};

Field::new(
  "Verification code",
  InputOtp::new(code.clone())
    .groups([3, 3])
    .on_complete(move |code| verify(code)),
)
.description("We sent a code to ivan@lurq.dev.");

// A recovery code of letters and digits, taken as capitals however it is typed.
InputOtp::new(recovery.clone())
  .length(8)
  .groups([4, 4])
  .kind(InputOtpKind::Alphanumeric)
  .uppercase(true);
```

| Builder | |
| --- | --- |
| `InputOtp::new(Signal<String>)` | The code. The cells show what the signal holds, and what is typed is set on it |
| `length(usize)` | How many cells it has: 6 unless it says |
| `groups(sizes)` | The sizes of the groups of cells, from the first: `[3, 3]` puts a dash after the third cell. Cells past the groups are a group of their own |
| `kind(InputOtpKind)` | `Digits` (the default) or `Alphanumeric`. Anything else typed or pasted is not taken |
| `uppercase(bool)` | A letter is taken as a capital |
| `mask(bool)` | A dot for each character in place of it, as a PIN is shown |
| `on_complete(handler)` | Called with the code when an edit fills its last cell |
| `size(InputOtpSize)` | `Sm`, `Md`, `Lg`: cells of 32 by 36, 36 by 40, 40 by 44, with type of 14, 16, 18 |
| `invalid`, `warning`, `valid`, `read_only`, `disabled`, `label`, `name` | As an [input](#input) has them; `name` is the whole code's in its form |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the input under the cells |

- To a person it is one field. Tab puts the focus on the first empty cell. Typing fills the cell that has the focus and goes on to the next; typing on a filled cell replaces its character. Backspace empties the cell that has the focus, or, on an empty one, goes back and empties the one before; Delete empties it and stays. Left and Right move, no further than the first empty cell; Home and End go to the first cell and to the first empty one. A click puts the focus on the cell clicked, or on the first empty one where that is before it.
- A code has no gaps: emptying a cell before the last filled one moves the cells after it up.
- A paste of a whole code, or more, fills every cell from the first; a shorter one fills the cells from the one that has the focus. What the code does not take of it (spaces, dashes) is dropped, so `123-456` is `123456`.
- To a form, to tooling and to the platform it is one text input: under the cells is the engine's `TextInput`, bound to the code, which draws nothing of its own. It is the one stop of Tab, the one `textbox` to what reads the tree (`lurq_snapshot` of the MCP server), called by its label, with the code as its value, and the one value of its form under its `name`. It says it holds a `ContentKind::OneTimeCode`, so a phone offers the code from a message that just came, and asks for the digits keyboard where the code is of digits. A masked code is a masked input.
- The cell the focus is on, and whether the input has the focus, are kept by the code's signal for as long as its input is in the tree, since a builder keeps nothing between renders: two codes bound to one signal share them.
- Parts: `InputOtpPart::Root` (the row of cells and separators, and the room between them), `Cell`, `Value` (the character, or its dot), `Caret` (in the empty cell that has the focus), `Separator` (the dash between groups). States: `InputOtpState::Invalid`, `Warning`, `Valid`, `ReadOnly`, `Disabled`, `Masked`, and on a cell of its own `Active` (the focus is on it) and `Filled`.
- In the design a cell is 36 wide and 40 high, 6 from the next, in the background's fill with an edge of 1 in the `Input` color, `radius-md` round, its character in the mono type at 16 in `Foreground`. The cell the focus is on has a line of 2 inside it in the `Ring` color, and while it is empty a caret 1.5 wide and 18 high in `Foreground`, which blinks unless the theme reduces motion. The dash between groups is the `minus` glyph at 16 in `MutedForeground`, 6 from the cells on either side. A judgment colors every edge and the ring; a disabled code is half opaque on the muted fill, a read-only one on the muted fill in the `Border` color.
- The mobile skin of the board (Mobile/OTP Field) is a rule of the theme: `Root` 8 apart, `Cell` 46 by 56 and `radius-lg`, `Value` in the sans type at 22 and semibold, `Caret` 2 by 24.
- A renderer that draws other cells starts from `view.input()`, the behavior, and lays its own cells over it: `view.characters`, `view.shown(index)` (what a cell shows, its dot where masked), `view.active` (the cell the focus is on, while the code has it), `view.separators`, and `view.cell_part(index, part)` for the style of a part of one cell. `view.cell(index)`, `view.separator()` and `view.cells()` are the default ones. A cell built by `view.cell(index)` is one a click is known to be on.

## For `CHANGELOG.md`

- `ui::InputOtp`, a one-time code of the design system: cells of digits, or letters and digits, in groups with a dash between, that a person types, pastes or has filled in by the platform as one field, and that a form and tooling read as one text input whose value is the code; `on_complete` when the last cell is filled, a mask, the sizes and judgments of an input, and `FieldControl`.

## Decided

- **One input under drawn cells, not one input per cell.** The engine's `TextInput`, bound to the code, lies under the cells, sized to them, in no color; the cells over it take no pointer, so a click anywhere on the code is the input's. That keeps what the brief asks of tooling and forms with nothing added to the engine: one stop of Tab, one `textbox` called by its label whose value is the code, one value of a form, the system's paste and the platform's suggestion of a one-time code into one input. One input per cell would have needed the engine to move the focus from one input to the next from a handler, six textboxes hidden from tooling and a seventh for the form, and a code filled in by the platform would have landed in the first cell only.
- **Every edit is the component's.** The input's own edits are refused (`prevent_default`) and its handlers write the code: `on_input` takes the text of a key or of an input method's commit (the event's key, when the new value holds it), `on_paste` takes the clipboard's text, `on_key_down` takes Backspace, Delete, the arrows, Home and End. So what the input's own caret is never matters, and it is drawn in no color.
- **Where the focus is, kept by the signal.** A builder keeps nothing between renders, and the cell the focus is on is not in the code. It is kept in a cursor (two signals: focused, and the cell) found by the id of the code's signal in a table of the thread, held by the input's handlers and gone when they are: no leak, and a code that leaves the tree and comes back starts out of the focus. A mounted `Component` would have kept it too, but then a `Field` could not hold the code (a field's control is built without a `Ctx`).
- **A click is placed by the cells' boxes.** Each cell has an `ElementRef`; the input's `on_click` puts the focus on the cell whose box has the pointer's x. The focus first goes on the first empty cell (`on_focus`), as for Tab, and the click then moves it.
- **No gaps in a code.** The value is the characters typed, in order, so an emptied cell before the last filled one moves the rest up. A code with a hole in it would not be a value a form or a server could read.
- **A cell has no hover.** The pointer is the input's, under the cells; the board draws no hover on a cell.
- **The cells are apart, as the board draws them.** The board has no joined cells (a group drawn as one box with shared edges); a theme could draw them with a renderer and `cell_part`.
- **`Active` and `Filled` are states of a cell**, resolved apart from the code's own (four resolutions of the recipe), so that a theme styles the cell the focus is on, or a filled one, by a rule.
- **Sizes**: the board draws one, 36 by 40 with type of 16; small and large are a step of 4 either way with type of 14 and 18, as an input's sizes are a step of 4.
- **The caret blinks** with a keyframe of its own at the pace of a text input's caret (530 ms shown, 530 hidden), not in phase, so that it starts shown in each cell the focus moves to.
- **An input method's composition** is shown in no cell until it is committed, as the input under the cells draws nothing; what it commits fills the cells as typing does. A code of digits is typed with no composition.
- **Up and Down** go to the first cell and to the first empty one, as Home and End, as they do in a one-line input.
- **Read only**, which an input has, is offered too: Left and Right move, nothing changes, the cells are on the muted fill.

## Not done

Nothing.

## Differs from the board

- The separator is the theme's `minus` glyph at 16 in `MutedForeground`; the board's is the minus icon with a stroke of 1.333 in place of the icon's 2. A glyph of the icon font has one weight.
- The board names the cells Slot 0, 1, 2, then Slot 4, 5, 6 around its divider; the parts are the cells 0 to 5 and a separator.
