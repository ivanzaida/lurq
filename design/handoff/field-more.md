## Changed outside the component's own files

- `crates/lurq/src/node/node.rs`: `EventHandlers::focuses: Option<ElementRef>`, after `on_edge_pan`. It is in the
  boxed handlers, so `Node` does not grow.
- `crates/lurq/src/components/mod.rs`: `.focuses(&ElementRef)` on every element (`impl_into_node!`), after
  `ref_element`. It sets the field above.
- `crates/lurq/src/app/runtime.rs`, in the pointer dispatch, after `dispatch_builtin_pointer`: on a left click whose
  default was not prevented, the first node under the pointer with `focuses`, before any node that is focusable,
  makes the request `Ctx::focus` makes. The request is applied when the click is handled
  (`apply_reactive_updates_after_event` → `apply_focus_request_inner`), by the lookup that was there: the first
  focusable node under the element the ref is on, and nothing where that node or a node around it is disabled.
- `crates/lurq/src/ui/text_box.rs` is new: what a box that is typed in or chosen from has, for `Input`, `Textarea`
  and, when it is built, `Select`. `ui/mod.rs` exports `FieldControl`, `InField`, `Judgment`, `TextBoxSize`,
  `TextBoxState` and `TextareaSize` (two lines changed besides the added ones: the `field` and `textarea` uses).

For `docs/src/content/docs/focus-navigation.md`, a bullet after the one for `.focusable(false)` under
`## Focusable Elements`:

```markdown
- `.focuses(&element_ref)` makes a click on an element focus another, as a `<label for>` focuses its control: the click focuses the first focusable element under the one the ref is on, as `ctx.focus(&element_ref)` does, once the click's handlers have run. A builder that has no `Ctx` can use it: it makes an `ElementRef::new()`, puts it on what holds the control with `.ref_element(..)` (and a `key`, so that a ref made anew in each render does not make that node another one), and names it on the label. A click on something under the element that takes the focus itself (a link in the label) focuses that instead, a click whose default a handler prevented focuses nothing, and a disabled target is not focused.
```

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
| `label(&str)` | What it is called, to what reads the tree. It draws nothing: a [field](#field) gives it its own label |
| `icon(name)` | A glyph of the theme's set at the start of the box |
| `size(InputSize)` | `Sm` 32, `Md` 36 (the default), `Lg` 40 high, with text of 13, 14, 15 |
| `invalid(bool)`, `warning(bool)`, `valid(bool)` | What the app judged of what it holds: the edge in the destructive, warning or success color. One is shown, in that order |
| `read_only(bool)` | It is focused, selected and copied, and every edit is refused |
| `full_width(bool)` | As wide as what holds it, in place of 260 |
| `on_input(handler)` | Called before an edit with the `TextInputEvent`; `prevent_default()` refuses it |
| `name(..)` | The name of its value in a form (feature `form`) |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the control, the box that is typed in |

- It is the engine's `TextInput`, single line, dressed by the recipe: the control *is* the box, so that the ring of its focus is drawn on it. A text input is focus-visible however it was focused, so a click shows the ring too. Tab reaches it wherever it is (`tab_index(0)`).
- Parts: `InputPart::Root` (the place it takes: its width, and the room between the icon and the text), `Control` (the box: edge, fill, height, the type of the value), `Placeholder` (laid over the control's type), `Icon`. States: `InputState::Invalid`, `Warning`, `Valid`, `ReadOnly`, `Disabled`, `FullWidth`, `WithIcon`. Sizes: `InputSize::Sm`, `Md`, `Lg` (`InputSize` is `TextBoxSize`, which a textarea has too).
- In the design the box is 260 by 36, `Background` in an edge of 1 in the `Input` color, `radius-md`, the text 12 from the edge in the body's type, the placeholder in `MutedForeground`. The edge is the only signal of its state: `Ring` under the pointer, a line of 2 in `Ring` inside the box in the focus (an outline, so that the edge under the pointer does not take it away), `Destructive`, `Warning`, `Success` for a judgment (which keeps its color under the pointer and in the focus). Disabled is half opaque on `Muted`; read only is on `Muted` in the `Border` color. The icon is 16 in `MutedForeground` (in `Success` while it is valid), 8 before the text.
- In a [field](#field) it is called by the field's label unless it has a `label` of its own, it is judged as the field is unless the app judged the input itself, and it is disabled where the field is.
- A text input does not take the foreground of its node, so the renderer names the color of the value and of the placeholder from the parts' foregrounds (`InputView::value_style()`, `placeholder_style()`).
- To what reads the tree the control is a `textbox`, with `a11y_invalid`, `a11y_readonly` where it is read only, and the name its label or its field's gives it (`InputView::label`).
- A renderer that draws another input starts from `view.control()`, the bound text input in the control's style; `view.icon()` is the icon laid over its start, and `view.text_start()` how far the text starts from the edge.
- What an input and a textarea have in common is one recipe's worth of rules on the box (its edge and fill, the focus inside it, the placeholder's color, the judgments, read only, disabled, the type of the sizes): a theme that changes one of them for both patches each recipe, by the same part and state names.

## Field

A control with what it is called above it, and under it what it is for or what is wrong with what it holds. The field finishes its control: the label names it, a press of the label focuses it, and an input in a field with an error is invalid without being told.

```rust
use lurq::ui::{Field, Input, Switch, Textarea};

Field::new("Display name", Input::new(name.clone()).full_width(true))
  .description("Shown in the title bar.")
  .required(true);

// The error makes the input invalid: its edge is destructive.
Field::new("Email", Input::new(email.clone()).full_width(true))
  .error("Enter a full email address.");

Field::new("What happened", Textarea::new(story.clone())).warning(true);

// Any element is a control; the label beside it, where a form lines its labels up.
Field::new("Notifications", Switch::new("Pause notifications")).horizontal(true);
```

| Builder | |
| --- | --- |
| `new(&str, impl FieldControl)` | Its label and its control: an `Input`, a `Textarea`, another component, any `Element` (an engine node goes in with `.into()`) |
| `description(&str)` | What the control is for, under it |
| `error(&str)` | What is wrong, in place of the description; the field is invalid |
| `invalid(bool)`, `warning(bool)`, `valid(bool)` | The description in the destructive, warning or success color, and the control judged so |
| `disabled(bool)` | Its control takes no input, and a press of its label focuses nothing |
| `required(bool)` | A `*` after the label |
| `optional(bool)`, `optional_text(&str)` | "Optional" after the label, or the app's own words. A required field shows none |
| `horizontal(bool)` | The label in a column of 110 beside the control and its description |
| `full_width(bool)` | As wide as what holds it, in place of 260 |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the field's root |

- A field's control is a `FieldControl`: what the field tells it (`InField`: the label, the judgment, whether it is disabled) it takes or leaves. An `Input` and a `Textarea` take all three, each only where the app did not say it of the control itself: an input with a label of its own keeps it, and one the app judged keeps that judgment whatever the field says. Any `Element`, and the other components of the design system for now, take none: a switch in a disabled field is disabled by the app. A control of an app's own implements `FieldControl` and passes what it is told to the control in it.
- A press of the label focuses the control: the first node under what holds it that takes the focus (`.focuses(&ElementRef)` on the label, the ref on the row that holds the control). A control that is disabled is not focused, and a disabled field's label focuses nothing. A renderer that holds the control another way puts `view.focus` on what holds it.
- An input that is to fill the field says so (`Input::full_width`).
- Parts: `FieldPart::Root`, `Label` (the label and its mark), `Required`, `Optional`, `Body` (the control and the description, where the label is beside them), `Description`. States: `FieldState::Invalid`, `Warning`, `Valid`, `Horizontal`, `FullWidth`, `Disabled` (no rule of the design's: a theme dims what it likes).
- In the design the field is 260 wide; the label is 6 above the control in a field label's type (13, medium) and `Foreground`, the description 6 under it in a caption's (12) and `MutedForeground`. The `*` is 4 after the label, semibold, in `Destructive`; "Optional" is 11 in `MutedForeground`. Beside the control the label is 110 wide and as high as a control, 12 from it, and the field is 360 wide.

## Textarea

Text of more than one line, typed into a box: a bio, a note, what happened.

```rust
use lurq::ui::{Textarea, TextareaSize};

// bio: Signal<String>
Textarea::new(bio.clone()).placeholder("A few words about you");

// Two rows while it holds less, four before its text scrolls; wrong.
Textarea::new(report.clone()).rows(2, 4).invalid(true);

// Read, selected and copied, not changed; in small type.
Textarea::new(terms.clone()).read_only(true).size(TextareaSize::Sm);
```

| Builder | |
| --- | --- |
| `new(Signal<String>)` | What it holds. What is typed is written into the signal, and what the app writes there shows |
| `placeholder(&str)` | What it shows while it holds nothing |
| `rows(min, max)` | The rows it shows at least, and at most before its text scrolls inside it. 3 and 8 unless told (`TEXTAREA_MIN_ROWS`, `TEXTAREA_MAX_ROWS`) |
| `size(TextareaSize)` | The type of an input's sizes: `Sm` 13, `Md` 14 (the default), `Lg` 15, each with a line of 1.5. A row is that high |
| `invalid(bool)`, `warning(bool)`, `valid(bool)` | What the app judged of what it holds: the edge in the destructive, warning or success color. One is shown, in that order |
| `read_only(bool)` | It is focused, selected and copied, and every edit is refused |
| `on_change(handler)` | Called on every edit with the text it holds once the edit applies. A read-only one makes no edit |
| `label(&str)` | What it is called, to what reads the tree. It draws nothing: a [field](#field) gives it its own label |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the box |

- It is the engine's multiline `TextInput`: as high as its text between its rows, its own scroll bar past them, Enter
  a new line. Tab reaches it wherever it is, as every control of the design system; a disabled one is passed by and
  takes no click. A read-only one is a stop of Tab and refuses every edit.
- Parts: `TextareaPart::Root` (the box, which is the text input), `Text` (the type and color of what it holds),
  `Placeholder` (laid over the text's type). States: `TextareaState::Invalid`, `Warning`, `Valid`, `ReadOnly`,
  `Disabled`. Sizes: `TextareaSize::Sm`, `Md`, `Lg` (`TextareaSize` is `TextBoxSize`, an input's sizes).
- In the design the box is as wide as its room, in `Background` with an edge of 1 in `Input`, the medium radius,
  padded 12 across and 8 down; its text is in the body's type with a line of 1.5, in `Foreground`, and the
  placeholder in `MutedForeground`. Under the pointer the edge is `Ring`; focused, a ring of 2 is drawn inside the
  box, over the edge. Invalid, warning and valid, the edge and the ring are `Destructive`, `Warning` and `Success`.
  Read only, it is on `Muted` in the `Border` color, as a read-only input. Disabled, it is at the theme's disabled
  opacity on `Muted`. Small, its text is 13; large, 15; the padding stays.
- In a [field](#field) it is called by the field's label unless it has a `label` of its own, it is judged as the
  field is unless the app judged it itself, it is disabled where the field is, and a press of the label focuses it.
- A text input draws in its own text style and does not take the foreground of what it is in, so the `Text` and
  `Placeholder` parts' foregrounds are resolved to colors of the theme when the textarea is built
  (`TextareaView::text_style`, `placeholder_style`).
- To what reads the tree it is a `textbox`, `a11y_multiline`, with `a11y_invalid`, `a11y_readonly` where it is read
  only, and the name its label or its field's gives it.
- A renderer that draws another textarea starts from `view.input()`, the behavior in the box's style, and puts what
  it likes around it: a count of characters, a grip.

## For `CHANGELOG.md`

- A field finishes its control: an `Input` or a `Textarea` in it is called by its label and judged as it is, and a press of the label focuses the control (`FieldControl`, and `.focuses(&ElementRef)` on any element); `Field::disabled`; `Textarea` has `read_only`, the sizes Sm, Md, Lg (`TextareaSize`) and `warning` and `valid`; `Input::label`. `Field::new` takes a `FieldControl`: an engine node goes in with `.into()`.

## Decided

- **A field's control is a `FieldControl`, finished when the field is built.** The field keeps it unbuilt (a closure)
  until its own judgment is known, and gives it an `InField`: its label, its judgment, whether it is disabled.
  Rust has no specialization, so the trait cannot be implemented for everything that converts into an `Element` and,
  apart from that, for `Input`: it is implemented for `Element`, for `Input` and `Textarea`, and, as they are, for
  every other component of the design system and for `Row`, `Column` and `Stack` (`as_they_are!` in `field.rs`). An
  engine node of another kind goes in with `.into()`: that is the one break of `Field::new`'s signature, which took
  `impl Into<Element>`. A component that comes to take what a field says (Select, Checkbox, RadioGroup) implements
  the trait in its own file and is taken off the list in `field.rs`, the one line that must change there.
- **The app's word on the control wins.** An input the app judged keeps its judgment in a field with another; an
  input with a `label` keeps its name; a field's `disabled` is or'ed with the input's.
- **The label focuses through the engine, not through the field's `Ctx`.** A builder has none. A node says "a click
  of me focuses that element" (`focuses`), and the runtime makes the request `Ctx::focus` makes, so the lookup and
  its rules (the first focusable node under the ref's element, nothing disabled) are the ones there were. It is on a
  click, as an HTML label is, and not on the press: the press of a label blurs what had the focus, as any press where
  nothing is focusable does, and the click gives it to the control.
- **The field's ref is made anew in each build**, and the row it is on has a key (`"control"`). A node with a ref and
  no key, id or slot is matched with its old self only if the ref is the same one, so without the key the row would
  be another node in each render and the input in it would lose the focus while the app rebuilds the field on every
  edit (an error that follows the value). The test of the label types twice with such a field.
- **`Field::disabled` and `FieldState::Disabled`.** "A label of a disabled field focuses nothing" needed the field to
  know it. The design draws no disabled field, so the state has no rule; the control shows that it is disabled.
- **`ui/text_box.rs`** holds what a box that is typed in or chosen from has: `TextBoxSize` (`InputSize` and
  `TextareaSize` are names of it), `Judgment` (with the order of the three and their colors), `TextBoxState` (the
  names the components' own state enums share), `BoxStates` (the builder's size, judgments, read only, disabled and
  label, the `Case` from them, and what a field gives), the box's style at rest (`box_style`), its rules
  (`box_rules`: pointer, focus, read only, disabled, the judgments, the placeholder's color), the type of the sizes
  (`box_sizes`, with or without heights), `ink` (a part's foreground into a text style), and `text_box` (the engine's
  text input as a box: a stop of Tab, a `textbox`, its name, invalid and read only to what reads the tree, and the
  refusal of every edit when it is read only). The recipes stay the components' own: each says what is its own (the
  input's height, type and icon; the textarea's padding down and line of 1.5) and calls these. `Select` takes
  `box_style`, `box_rules` and `BoxStates` as they are; `text_box` and `ink` are for what is typed in.
- **A textarea's sizes change its type, not its padding**: the board draws no textarea in sizes, and an input's sizes
  keep their padding of 12 across. A row is the size's type times 1.5 high: 19.5, 21, 22.5.
- **Read only is the input's**: an `on_input` that refuses every edit, so its handlers are not called and the box
  stays a stop of Tab, selectable and copyable. A textarea's `on_change` is not called for a refused edit.
- **The textarea's placeholder takes the text's type**, with what its own part lays over it, as the input's does: its
  rules no longer repeat the body's type and the line of 1.5.
- **The name is tested through the view.** `describe("a11y_name", ..)` records nothing unless `devtools` or `mcp` is
  on, and the ui tests are compiled with neither; `InputView::label` and `TextareaView::label` say the name, and the
  tests read them from a theme's renderer.

## Not done

- **Select, Checkbox, RadioGroup do not take what a field says yet**: they are in `as_they_are!` until they implement
  `FieldControl`. A field's `disabled` does not reach them, and a press of the label focuses them without naming them.
- **A field's description is not the control's description** to what reads the tree (`aria-describedby`): no
  attribute for it was named, and the error is not read with the control.
- **`.focuses` acts on a left click in the pointer's dispatch only.** Whether a tap reaches it depends on the touch
  path making a click of it, which was not checked. Nothing focuses the control from a label by the keyboard, as in
  a browser.
- **No test reads the `a11y_*` attributes themselves**, for the reason under Decided.

## Differs from the board

- **A disabled field.** The board draws none; `Field::disabled` dims nothing but the control.
- **A textarea in sizes.** The board draws an input's sizes and no textarea in them; a textarea's are the input's
  type at the textarea's padding.
