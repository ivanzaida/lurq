# Handoff: Input and Field

Built from the board's `Input` (`Ptl3b`) and `Field` (`zz052`) in the Inputs section of `Components / light`, with the
states drawn around them (States, More states, Sizes, Extra states, Fields, Field extras) and the required / optional
marks and the label beside the control from `Form > Fieldset · required, optional, horizontal label`.

Files: `crates/lurq/src/ui/input.rs`, `crates/lurq/src/ui/field.rs`, `crates/lurq/tests/ui/input.rs`,
`crates/gallery/src/pages/input.rs`; a line each in `ui/mod.rs` (`mod` and `pub use` for both), `tests/ui/mod.rs`
and `pages/mod.rs` (the page "Input" in the group Inputs, before Switch).

No shared code was changed: nothing in `shared.rs`, `part_style.rs`, `recipe.rs` or the engine.

## For `docs/src/content/docs/design-system.md`

## Input

One line of text that a person types: a name, an email address, a search.

```rust
use lurq::ui::{Input, InputSize, icons};

Input::new(name.clone()).placeholder("Display name");

Input::new(query.clone()).placeholder("Search components").icon(icons::SEARCH).size(InputSize::Sm);

// What the app checked: the edge says so.
Input::new(email.clone()).invalid(!email_ok);
```

| Builder | |
| --- | --- |
| `new(Signal<String>)` | What the input shows; what is typed is set on it, and what the app sets on it is shown |
| `placeholder(&str)` | What it says while it is empty |
| `icon(name)` | A glyph of the theme's set at the start of the box |
| `size(InputSize)` | `Sm` 32, `Md` 36 (the default), `Lg` 40 high, with text of 13, 14, 15 |
| `invalid(bool)`, `warning(bool)`, `valid(bool)` | What the app judged of what it holds: the edge in the destructive, warning or success color. One is shown, in that order |
| `read_only(bool)` | It is focused, selected and copied, and every edit is refused |
| `full_width(bool)` | As wide as what holds it, in place of 260 |
| `on_input(handler)` | Called before an edit with the `TextInputEvent`; `prevent_default()` refuses it |
| `name(..)` | The name of its value in a form (feature `form`) |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the control, the box that is typed in |

- It is the engine's `TextInput`, single line, dressed by the recipe: the control *is* the box, so that the ring of its focus is its own edge. A text input is focus-visible however it was focused, so a click shows the thicker edge too. Tab reaches it wherever it is (`tab_index(0)`).
- Parts: `InputPart::Root` (the place it takes: its width, and the room between the icon and the text), `Control` (the box: edge, fill, height, the type of the value), `Placeholder` (laid over the control's type), `Icon`. States: `InputState::Invalid`, `Warning`, `Valid`, `ReadOnly`, `Disabled`, `FullWidth`, `WithIcon`. Sizes: `InputSize::Sm`, `Md`, `Lg`.
- In the design the box is 260 by 36, `Background` in an edge of 1 in the `Input` color, `radius-md`, the text 12 from the edge in the body's type, the placeholder in `MutedForeground`. The edge is the only signal of its state: `Ring` under the pointer, `Ring` and 2 thick in the focus, `Destructive`, `Warning`, `Success` for a judgment (which keeps its color under the pointer and in the focus). Disabled is half opaque on `Muted`; read only is on `Muted` in the `Border` color. The icon is 16 in `MutedForeground` (in `Success` while it is valid), 8 before the text.
- A text input does not take the foreground of its node, so the renderer names the color of the value and of the placeholder from the parts' foregrounds (`InputView::value_style()`, `placeholder_style()`).
- To what reads the tree the control is a `textbox`.
- A renderer that draws another input starts from `view.control()`, the bound text input in the control's style; `view.icon()` is the icon laid over its start, and `view.text_start()` how far the text starts from the edge.

## Field

A control with what it is called above it, and under it what it is for or what is wrong with what it holds.

```rust
use lurq::ui::{Field, Input, Switch};

Field::new("Display name", Input::new(name.clone()).full_width(true))
  .description("Shown in the title bar.")
  .required(true);

Field::new("Email", Input::new(email.clone()).invalid(true).full_width(true))
  .error("Enter a full email address.");

// Any element is a control; the label beside it, where a form lines its labels up.
Field::new("Notifications", Switch::new("Pause notifications")).horizontal(true);
```

| Builder | |
| --- | --- |
| `new(&str, impl Into<Element>)` | Its label and its control: any element |
| `description(&str)` | What the control is for, under it |
| `error(&str)` | What is wrong, in place of the description; the field is invalid |
| `invalid(bool)`, `warning(bool)`, `valid(bool)` | The description in the destructive, warning or success color |
| `required(bool)` | A `*` after the label |
| `optional(bool)`, `optional_text(&str)` | "Optional" after the label, or the app's own words. A required field shows none |
| `horizontal(bool)` | The label in a column of 110 beside the control and its description |
| `full_width(bool)` | As wide as what holds it, in place of 260 |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the field's root |

- The field does not reach into its control: an input whose content is wrong is told so itself (`Input::invalid`), and the field says why. An input that is to fill the field says so (`Input::full_width`).
- Parts: `FieldPart::Root`, `Label` (the label and its mark), `Required`, `Optional`, `Body` (the control and the description, where the label is beside them), `Description`. States: `FieldState::Invalid`, `Warning`, `Valid`, `Horizontal`, `FullWidth`.
- In the design the field is 260 wide; the label is 6 above the control in a field label's type (13, medium) and `Foreground`, the description 6 under it in a caption's (12) and `MutedForeground`. The `*` is 4 after the label, semibold, in `Destructive`; "Optional" is 11 in `MutedForeground`. Beside the control the label is 110 wide and as high as a control, 12 from it, and the field is 360 wide.

## For `CHANGELOG.md`

- `ui::Input` and `ui::Field`: an input is the engine's text input bound to a `Signal<String>` in the design's box, with a placeholder, a leading icon, the sizes `Sm`, `Md`, `Lg`, an edge that says its state (hover, focus, `invalid`, `warning`, `valid`), `read_only`, `disabled` and `full_width`; a field holds any control with its label, a `required` or `optional` mark, a description or an `error` under it, or its label beside it (`horizontal`). Parts and states for a recipe, and `InputView::control()`, `icon()` for a renderer.

## Decided

- The control is the `TextInput` node itself, with the edge, fill and padding on it, and the icon is laid over it in a `Stack` with `pointer_events_none`. The engine has no focus-within, and the board draws the focus as the box's own edge: a box around the text input could not show it. The text input's left padding is the control's padding, the icon and the root's gap (`InputView::text_start`).
- The focus is the board's thick edge (`Border::inside(BorderSize::Md, Ring)`), not the theme's focus ring outside the box. A state style changes what is painted only; an inside border does not move the text.
- Of `invalid`, `warning` and `valid` one is shown, in that order; `Field::error` makes the field invalid.
- Read only is built here, not in the engine: an `on_input` handler that refuses every edit. The input is still focusable and its text selectable and copyable.
- `Input::name` exists only with the feature `form`, as `TextInput::name` does.
- The field's control sits in a row the width of the field, so a control keeps its own width (a switch is not stretched); a full-width input fills it.

## Not done

- The field's label does not focus its control when it is pressed, and the control is not told its label as its accessible name: a builder has no `Ctx` and no handle on the control's node. The control's `a11y_name` would be the place for it.
- A field does not hand its state to its control: an invalid field and its invalid input are said twice by the app.
- No trailing icon, no clear button, no addon: the board draws them only in Input Group, Search Field and Password Input, which are not this component.
- The tests in `crates/lurq/tests/ui/input.rs` were compiled and not run, as the batch is tested when it is merged. The ones most likely to need a number adjusted: the text 12 and 36 from the box's edge (the text quad's x), and the horizontal label's middle level with the control's.
- `crates/gallery/src/playground.rs` builds its text props with a hand-made `TextInput` (`field()`): it could be an `Input` now. Left alone, it is shared gallery code.

## Differs from the board

- Hover, focus and the judgments are drawn by the board on separate instances; here a judgment keeps its color under the pointer and in the focus (2 thick), which the board does not draw.
- The board's Valid instance has a check icon at the start in `Success`: here the icon is the app's (`icon(icons::CHECK)`), and the `Valid` state only colors whatever icon there is.
- The horizontal field's width (360) is the Fieldset's inner width on the board (400 less 20 on each side); the board has no horizontal Field component of its own.
