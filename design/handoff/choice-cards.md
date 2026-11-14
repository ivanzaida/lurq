# Choice cards and the fieldset: hand-off

`ui::CheckboxCard` and `ui::RadioCard` with their groups `ui::CheckboxCardGroup` and `ui::RadioCardGroup`, and
`ui::Fieldset` with `ui::ErrorSummary` and `ui::FieldError`, from the board's `Checkbox Card` (`lCj7S`), `Radio Card`
(`IhFJO`), `Fieldset` (`LJ6do`) and `Error Summary` (`Mi6Ue`) in Components / light (`Ovt1D`), and where they are used:
Advanced Inputs › Cards (`y0OK6t`: a checkbox card checked and one not, a radio card chosen) and Forms & Fields › Form
(`e4TPc`: the fieldset with a required field, an optional one, one with its label beside it and its actions; the
summary over two errors), with their copies in Components / dark.

Files: `crates/lurq/src/ui/choice_card.rs`, `crates/lurq/src/ui/fieldset.rs`, `crates/lurq/tests/ui/choice_card.rs`,
`crates/lurq/tests/ui/fieldset.rs`, `crates/gallery/src/pages/checkbox_card.rs`, `radio_card.rs` and `fieldset.rs`
(pages "Checkbox Card", "Radio Card" and "Fieldset", group Inputs), this file, and lines in the three `mod.rs` files.

## Changed outside the component's own files

- `crates/lurq/src/app/runtime.rs`, the press of the pointer: an element that takes the focus itself and is given
  `.focuses(&ElementRef)` gives the focus on to its target when it is clicked. Before, the search for a `focuses` stopped
  at the first node that takes the focus, its own included, so a button with one focused itself. A label that does not
  take the focus, and what is under a control that does, are as before.
- `crates/lurq/src/app/runtime.rs`, `activate_focused_button`: Enter or Space on a focused button that has a `focuses`
  focuses its target after its `on_click` handlers run, as a click does. This is what makes the lines of an
  `ErrorSummary` links that the keyboard follows: a builder has no `Ctx` to focus a field from a handler.
- `crates/lurq/src/components/mod.rs`: the doc of `focuses` says both.

Tests of these: `a_press_or_the_keyboard_on_a_line_focuses_its_field` and
`a_summary_of_a_form_lists_its_errors_once_a_submit_was_attempted` in `tests/ui/fieldset.rs`.

## For `docs/src/content/docs/design-system.md`

## Checkbox Card

A checkbox whose whole card is pressed, with a title, a description and an icon or a price: the add-ons of a plan, the reports a team is sent.

```rust
use lurq::ui::{CheckboxCard, CheckboxCardGroup, icons};

CheckboxCard::new("Analytics")
  .description("Share anonymous usage data.")
  .checked(analytics.get())
  .on_change(move |checked| analytics.set(checked));

// Cards over one list: those whose values the signal has are checked.
CheckboxCardGroup::new(add_ons, [
  ("backups", CheckboxCard::new("Backups").icon(icons::SHIELD).price("$4")),
  ("support", CheckboxCard::new("Priority support").icon(icons::MAIL).price("$20")),
])
.horizontal(true);
```

| Builder | |
| --- | --- |
| `description(&str)` | More of it, under the title in the muted color; what the card is described by to tooling |
| `icon(name)`, `price(&str)` | An icon of 20 before the text; what the choice costs, at the end of the title |
| `checked(bool)` | Whether it is checked. The card does not keep it: the app says so |
| `on_change(handler)` | Called on a press with what the card is to be: checked if it was not |
| `invalid(bool)`, `full_width(bool)` | The edge in the destructive color; as wide as what holds it in place of 220 |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the card |

| `CheckboxCardGroup` | |
| --- | --- |
| `new(Signal<Vec<T>>, [(T, CheckboxCard)])` | The cards whose values the signal has are checked; a press adds a value or takes it out |
| `controlled(checked, [(T, CheckboxCard)], press)` | The app holds what is checked and is told each value pressed |
| `horizontal(bool)` | Side by side, 12 apart and as high as the highest, going on to the next line where there is no room |
| `label(&str)`, `invalid(bool)`, `disabled(bool)`, `style(..)`, `id`, `key` | What the group is called to tooling; every card invalid or disabled |

- The card is what is pressed: a click anywhere on it, or Space or Enter while it has the focus. It is the one stop of Tab and the ring of the focus is around it; the box in it is drawn and takes no focus of its own. In a group each card is its own stop.
- Parts: `ChoiceCardPart::Root` (what is pressed), `Icon`, `Text`, `Heading` (the title and the price), `Title`, `Price`, `Description`. States: `ChoiceCardState::Chosen`, `Invalid`, `Disabled`, `FullWidth`. The box is the checkbox's own `CheckboxPart::Box` and `Indicator`, resolved from the theme's checkbox recipe: a theme that restyles a checkbox restyles the card's box. A group's parts are `ChoiceCardGroupPart::Root` and `Item` (laid over each card's root), its state `ChoiceCardGroupState::Horizontal`.
- In the design the card is 220 wide, padded 16, in `Background` with the large radius; the text first, the title in 14 and medium over the description 4 under it in 13 of `MutedForeground`, and the box 12 after it at the top. Not checked its edge is 1 of `Input`; checked, 2 of `Primary`. Under the pointer the edge of a card not checked is `Ring`; an invalid card is edged in `Destructive`; a disabled one is dimmed.
- To what reads the tree (`lurq_snapshot` of the MCP server) the card is a `checkbox` called by its title, described by its description, with `a11y_checked` saying whether it is checked; a group is a `group`.
- In a [field](#field) a card is invalid where the field is and disabled where the field is, and keeps its title as its name; a group takes the field's label as its name unless it has one.
- A renderer that draws another card is given a `ChoiceCardView`: `view.root()` is the behavior with the root's style, `view.text()`, `view.icon()` and `view.control()` the default parts. The same renderer type serves the radio card; `view.kind` says which it is.

## Radio Card

A radio whose whole card is pressed, with a title, a description and an icon or a price: the plan of an account, the density of a list.

```rust
use lurq::ui::{RadioCard, RadioCardGroup};

RadioCardGroup::new(density, [
  (Density::Comfortable, RadioCard::new("Comfortable").description("Roomy rows, 36px controls.")),
  (Density::Compact, RadioCard::new("Compact").description("Dense rows, 28px controls.")),
  (Density::Touch, RadioCard::new("Spacious").description("For touch.").disabled(true)),
]);

// One card alone: the app says whether it is chosen and is told when it is pressed.
RadioCard::new("Team").price("$12").chosen(plan.get() == Plan::Team).on_choose(move || plan.set(Plan::Team));
```

| Builder | |
| --- | --- |
| `description(&str)`, `icon(name)`, `price(&str)` | As a [checkbox card](#checkbox-card) has them |
| `chosen(bool)` | Whether it is the choice made: the dot in its mark. The card does not keep it |
| `on_choose(handler)` | Called on a press while the card is not chosen. A press of the one chosen does nothing |
| `invalid(bool)`, `full_width(bool)`, `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a checkbox card has them |

| `RadioCardGroup` | |
| --- | --- |
| `new(Signal<T>, [(T, RadioCard)])` | The card whose value the signal has is chosen; a press of another sets the signal |
| `controlled(Option<T>, [(T, RadioCard)], choose)` | The app holds the choice, none while it is `None`, and is told each value pressed |
| `horizontal(bool)`, `label(&str)`, `invalid(bool)`, `disabled(bool)`, `style(..)`, `id`, `key` | As a checkbox card group has them |

- A group is one stop of Tab, at the chosen card or the first. The arrow keys move the focus to the next card and choose it, Home and End the first and the last, and a disabled card is passed by: the focus group of a [radio group](#radio), `FocusGroup::both().activate(true).one_stop(true)`.
- Parts and states are a checkbox card's (`ChoiceCardPart`, `ChoiceCardState`); its mark is the radio's own `RadioPart::Mark` and `Dot`, from the theme's radio recipe.
- In the design it is a checkbox card with the radio's mark of 16 in place of the box: not chosen, an edge of 1 of `Input`; chosen, 2 of `Primary`, with the dot of 8 in the mark.
- To what reads the tree the card is a `radio` called by its title, described by its description, with `a11y_checked`; a group is a `radiogroup`.

## Fieldset

Fields that go together, under a legend that says what they are: the profile of an account, the address of an order.

```rust
use lurq::ui::{Button, Field, Fieldset, Input};

Fieldset::new("Profile")
  .description("Shown to people you share a workspace with.")
  .field(Field::new("Display name", Input::new(name).full_width(true)).required(true))
  .field(Field::new("Bio", Input::new(bio).full_width(true)).optional(true))
  .child(Button::new("Save profile"))
  .disabled(locked.get());
```

| Builder | |
| --- | --- |
| `description(&str)` | More of it, 4 under the legend, in the muted color |
| `field(Field)` | Adds a field, made as wide as the fieldset (`Field::full_width`) |
| `child(element)`, `children(..)` | Adds anything else under what is there: a row of buttons, a group of cards, an [error summary](#error-summary) |
| `disabled(bool)` | Every control in it takes no input and Tab passes it by; the whole fieldset is dimmed |
| `full_width(bool)` | As wide as what holds it, in place of 400 |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [field](#field) has them |

- What it holds is one under another, 16 apart, and stretched to its width: a field keeps a width of its own unless it is made full width, which `field` does.
- Parts: `FieldsetPart::Root`, `Legend` (the title over the description), `Title`, `Description`. States: `FieldsetState::Disabled`, `FullWidth`.
- In the design the fieldset is 400 wide, padded 20, with an edge of 1 of `Border` inside it and the large radius. The legend's title is 15 and semibold in `Foreground`, its description 13 in `MutedForeground`.
- A disabled fieldset disables its root, and an element that is disabled takes everything in it out of input, whatever it is: a control the app built itself too. The fieldset is dimmed as a whole, so a control in it is not dimmed again by its own disabled state.
- To what reads the tree it is a `group` called by its legend and described by its description.

## Error Summary

What is wrong in a form, at its top, a line for each field with a link to it: what a person reads when a form they sent was not accepted.

```rust
use lurq::ui::{ErrorSummary, FieldError};

ErrorSummary::new([
  FieldError::new("Display name", "Display name is required").focuses(&name_field),
  FieldError::new("Email", "Email must include a domain").focuses(&email_field),
]);

// From a form's own errors, once a submit of it was attempted (the `form` feature).
ErrorSummary::form(&form, [("name", "Display name", &name_field), ("email", "Email", &email_field)]);
```

| Builder | |
| --- | --- |
| `new(errors)` | A line for each `FieldError`, in the order given. With none the summary draws nothing |
| `FieldError::new(label, message)` | The field's label and what is wrong with it. The line shows the message |
| `FieldError::focuses(&ElementRef)` | A press of the line, or Enter or Space on it, focuses the first control under the ref |
| `form(&FormHandle, [(name, label, &ElementRef)])` | The form's errors once a submit was attempted: a line for each listed field that has one, in their order, then one for each other field with an error, called by its name and focusing nothing. It reads them as signals: what renders it renders again when they change |
| `title(&str)` | The title in the app's own words, in place of "2 fields need attention" |
| `full_width(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a fieldset has them; `force` draws every line as under the pointer or focused |

- A line that focuses a field is a stop of Tab, underlined under the pointer, with the ring of the focus around it. One that focuses nothing is read and not pressed.
- Parts: `ErrorSummaryPart::Root`, `Icon`, `Body` (the title over the lines), `Title`, `Item` (a line), `Bullet`, `Message`. State: `ErrorSummaryState::FullWidth`.
- In the design the summary is 400 wide, padded 12 and 14, in `DestructiveMuted` with an edge of 1 of `Destructive` inside it and the medium radius. The alert icon of 16 in `Destructive` is 10 before the body; the title is 13 and semibold in `Destructive`, and the lines are 6 apart under it, a dot of 4 in `Destructive` 8 before each message in 13 of `Foreground`.
- To what reads the tree it is an `alert` called by its title; each line is a `link` called by the field's label and its message.
- A line focuses its field through `.focuses(&ElementRef)`, which a button has: a click of it or Enter or Space on it focuses the target. A ref on a column around a `Field` will do: the control is the first thing under it that takes the focus.

## For `CHANGELOG.md`

- `ui::CheckboxCard` and `ui::CheckboxCardGroup`: a checkbox whose whole card is pressed, with a title, a description and an icon or a price, its box the checkbox's own, edged in the primary color while it is checked; and cards over one list bound to a signal, one under another or side by side.
- `ui::RadioCard` and `ui::RadioCardGroup`: a radio whose whole card is pressed, its mark the radio's own; and cards that are one choice bound to a signal, one stop of Tab that the arrow keys move the choice through, as in a group of radios.
- `ui::Fieldset`: fields under a legend and its description, in an edged box, that a disabled fieldset takes out of input together with anything else in it.
- `ui::ErrorSummary` and `ui::FieldError`: what is wrong in a form, at its top, with a line for each field that a press or Enter takes the focus to; `ErrorSummary::form` makes one from a `FormHandle`'s errors once a submit was attempted. A button given `.focuses(&ElementRef)` gives the focus on when it is clicked or pressed from the keyboard; it focused itself.

## Decided

- **One view, one recipe for the two cards.** A checkbox card and a radio card differ in the role, the press and what is at the end; everything else is one `ChoiceCardView`, one `ChoiceCardPart` and `ChoiceCardState`, and one recipe that both `Themed` impls return. A theme still styles them apart (`checkbox_card`, `radio_card`).
- **The box and the mark are the checkbox's and the radio's own parts**, resolved from the theme's `Checkbox` and `Radio` recipes with the card's states (checked, invalid, disabled), and drawn in a plain row: not `pressable`, so they take no focus and their own focus style never shows. `CheckboxView::indicator()` could not be used: a `CheckboxView` is made by `Checkbox::build` alone.
- **The card's surface is the board's, not `ui::Card`'s outline variant**: the board draws it in `$background` with `$radius-lg` and an edge of `$input`, where a card has `$card`, `$radius-xl`, `$border` and a shadow.
- **The radio card group is not a second radio group**: it takes `Chosen` from `shared.rs` and the very `FocusGroup` of `RadioGroup` (`both().activate(true).one_stop(true)`) with each card's `focus_entry(chosen)`. It holds cards, not `Choice`s, because a card has a description, an icon and a price that a `Choice` has no room for: `(T, RadioCard)`.
- **A checkbox card group** was built too, as `CheckboxGroup` is beside `Checkbox`: an app binds a list of add-ons to a `Signal<Vec<T>>` in one call.
- **The price stands at the end of the title**, in 14 and semibold. The board draws neither an icon nor a price on its cards; the brief asks for them. The icon is 20 (`IconSize::Lg`) in the foreground, 12 before the text.
- **Hover**: an edge of `$ring` on a card that is not chosen, as a pressed `ui::Card` has; a chosen or invalid card keeps its own edge under the pointer. The board draws no hover.
- **The width is the board's 220**, with `full_width(true)` for a card that fills a column, as a `Field` has. Cards side by side in a group are stretched to the height of the highest in their line.
- **A disabled fieldset disables its root**, which the engine carries to everything in it, and is dimmed as a whole, as an HTML fieldset is. It does not tell its children they are disabled: they are built already, and a fieldset that did would have to hold `FieldControl`s and not elements.
- **`Fieldset::field` takes a `Field`** and makes it full width, since a field is 260 wide of its own and the board's fill the fieldset; anything else goes in with `child`.
- **The summary's title counts the errors** given ("1 field needs attention", "2 fields need attention") in English, with `title` for the app's own words. A line shows the message alone, as the board does; the label is in its name to tooling.
- **`ErrorSummary::form` shows nothing until a submit was attempted** (`FormHandle::submit_attempted`), as a summary is for a form that was sent. A field the app does not list is still a line, called by its name in the form, so that no error is left out.
- **The summary is a builder, not a mounted component**: what is wrong is the app's (or the form's) state, given each render.

## Not done

Nothing.

## Differs from the board

- The board's cards and summary have no hover, focus or disabled state drawn: the hover edge, the ring and the dimming are this file's (under "Decided").
- The board's card has no icon or price: their place and type are this file's.
- The board's fieldset lays its "Time zone" field with its label beside it at a label of 110; the gallery's uses `Field::horizontal`, which is the field's own (110 wide, 12 from the control).
- The gallery's fieldset has an input for the time zone where the board has a select: a select needs an anchor and a signal of its own that the playground's builder would hold for nothing.
