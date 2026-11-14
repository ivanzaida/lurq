# Combobox and Multi Select: hand-off

`ui::Combobox` and `ui::MultiSelect`, from the board's `Combobox` (`tv8im`), `Multi Select` (`f0w5e`) and `Listbox`
(`W3EUVW`) in Components / light (`Ovt1D`), and where they are used: the open combobox (`XSlgv`, in Composite, with
its list of Menu Items, the first one highlighted and checked), the multi select in Selection (`MC7NK`: two tags, a list
of four with checkboxes, the third under the pointer), the listbox in Forms & Fields (`n6ouc`: a group label, a chosen
item with a check, one highlighted, one disabled), and the inline loader (`QjMSn`: a spinner of 16 and "Loading
results…") for the loading row.

Files: `crates/lurq/src/ui/combobox.rs`, `crates/lurq/src/ui/multi_select.rs`, `crates/lurq/tests/ui/combobox.rs`,
`crates/lurq/tests/ui/multi_select.rs`, `crates/gallery/src/pages/combobox.rs` and `multi_select.rs` (pages "Combobox"
and "Multi Select", group Inputs), `design/handoff/combobox.md`, and lines in the three `mod.rs` files.

## Changed outside the component's own files

This session owns the engine's select, its list in a layer, and what typing does to it; the changes are of that kind.

- `crates/lurq/src/components/select.rs`: the engine's `Select` has a combobox mode.
  - `Select::combobox(TextInput)`: the input goes into the trigger (`SelectTriggerState::input`, a new field, which the
    default trigger puts where the label is) and keeps the focus. The select pushes onto the input an `on_input`
    handler that opens the list on every edit that is not refused (`SelectState::typed`), and an `on_key_down` handler
    for while the focus is in it: Down and Up open the list (on the chosen option, or the first or last) or move the
    highlight, Alt+Down opens with nothing highlighted and Alt+Up closes, Enter chooses the highlighted option (with
    nothing highlighted it closes the list and leaves the key to the input), Escape closes the list and on a closed
    list clears what is typed as an edit (select all, Backspace, so the input's handlers hear it), Tab closes the list
    and goes on. The other keys are the input's.
  - `Select::free_text(bool)`: typing highlights nothing (what is typed may be no option's).
  - `Select::status(Element)`: a row the open list draws after its options, which cannot be chosen.
  - `SelectOption::value()`, `label()`, `starts_group()`, `is_disabled()`: what a combobox filters by.
  - `components::SelectTriggerState` is exported (it is the argument of `Select::trigger`, which was public already).
- `crates/lurq/src/node/select_state.rs`: the select holds its combobox's `TextInputState` and `free_text` (per
  render), the status row's builder, and `highlight_first` (runtime): something was typed, and when the next render
  gives the filtered options the first that can be chosen is highlighted (`copy_runtime_state_from`). `typed()` opens
  the list; `open_with_highlight_from(forward)`; `commit` of a single combobox puts the option's label in its input
  (`TextInputState::set_value`, which calls no handler) with the caret at its end; `input_focused()`. The input holds
  the select by a `WeakSelectState` (new), as the select holds the input: no cycle.
- `crates/lurq/src/node/select_style.rs`: `SelectStyle::checkmark_unchecked(SelectIcon)`, what the check's slot holds on
  an option that is not chosen: the empty box of a list of checkboxes.
- `crates/lurq/src/app/runtime/select_menu.rs`: the status row after the options; the unchecked mark; and to what
  reads the tree the list is a `listbox` (with `a11y_activedescendant`, the label of the option under the keys) of
  `option`s, each called by its label with `a11y_selected` and, where it is, `a11y_disabled`. A `ui::Select`'s list
  says so too now.
- `crates/lurq/src/node/node.rs`: a press of a combobox opens its list and leaves it open (a plain select's press
  toggles it, as before); `Node::clone_select_state`.
- `crates/lurq/src/app/runtime.rs`: a press on a combobox focuses its text input, not the select node
  (`combobox_input_id`).
- `crates/lurq/src/layout/layout_engine/select_quads.rs`: a combobox's box is drawn focused (its `trigger_focused`, the
  ring) while its input has the focus.
- `crates/lurq/src/ui/select.rs`: what turns a select's recipe into the engine's style is a function of the crate,
  `engine_style(recipe, forced, in_state)`, which `SelectView::select_style` calls and the combobox and the multi select
  call too; `px`, `spacing`, `opacity`, `icon` and `engine_part` are `pub(crate)`. Nothing a select does changed.

- `crates/lurq/src/layout/layout_engine.rs`, `layout_flex_wrap`: a child of a wrapping row or column with a grow
  factor and a basis (`flex_full(grow, shrink, Some(basis))`) is its basis long when the lines are broken, and grows
  into what is left of its line after, as a flex item of a wrapping flexbox does. A child without a basis is laid out
  as before. The multi select's input keeps 48 on the line of the tags this way and takes the rest of it: a text input
  measured on its own is as wide as it may be, and went to a line of its own.

No keyframes were added.

## For `docs/src/content/docs/design-system.md`

## Combobox

A select a person types in, whose list what is typed filters: a framework, a country, an assignee among many.

```rust
use lurq::{components::SelectOption, ui::Combobox};

Combobox::new(framework.clone(), [
  ("lurq".to_owned(), "lurq"),
  ("gpui".to_owned(), "GPUI"),
  ("slint".to_owned(), "Slint"),
])
.placeholder("Select framework");

// Options an app fetches as a person types: it keeps all it was given.
Combobox::new(country.clone(), fetched.get().iter().map(|name| (name.clone(), name.as_str())).collect::<Vec<_>>())
  .on_input(move |typed| fetched.set(search(typed)))
  .filter(|_, _| true)
  .loading(searching.get());

// A value that is no option's: what is typed becomes the value of a combobox of String.
Combobox::new(label.clone(), [SelectOption::new("bug".to_owned(), "bug")]).allow_custom(true);
```

| Builder | |
| --- | --- |
| `Combobox::new(value, options)` | Bound to a `Signal<T>`: the option with its value is chosen, and choosing another sets it. The options are a [select](#select)'s: `(value, label)` or `SelectOption`s with a detail, a hint, elements before and after the label, a group they start, or disabled |
| `placeholder(&str)` | What the box says while nothing is typed |
| `filter(Fn(&str, &str) -> bool)` | Which options are shown while something is typed, given the label and what is typed. By default the labels that contain it, without regard to case |
| `on_input(Fn(&str))` | Called with what the box will hold at each edit: for an app that fetches its options |
| `loading(bool)` | The list ends in a row with a spinner, "Loading results…" (`loading_text(&str)` says otherwise) |
| `empty(&str)` | What the list says while what is typed leaves no option: "No results" |
| `allow_custom(bool)` | What is typed may be no option's: typing highlights nothing, and Enter or leaving keeps it. A label of an option, in any case, chooses the option; anything else becomes the value of a combobox of `String` |
| `query(Signal<String>)` | Where the app keeps what is typed, to read or set it. Without one the combobox keeps it while it is in the tree, one for each signal of a value |
| `size(ComboboxSize)` | `Sm` 32, `Md` 36 (the default), `Lg` 40 high, with text of 13, 14, 15 |
| `invalid(bool)`, `warning(bool)`, `valid(bool)` | What is chosen is wrong, doubtful or right: the edge and the ring in the destructive, warning or success color. Of the three, the first that is set is shown |
| `disabled(bool)`, `full_width(bool)`, `label(&str)`, `name(..)` | As a [select](#select) has them. `name` is the name of the value in a form: the chosen option's label |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the box; its input has the id with `-input` after it |

- The box is a select's with a text input in it, which keeps the focus the whole time. A press anywhere on the box focuses the input and opens the list; typing opens it too and filters it, with the first option that can be chosen highlighted. Down and Up move the highlight, which a row shows as it shows the pointer; Enter chooses; Escape closes the list and a second Escape clears what was typed; Tab chooses nothing, closes the list and goes on.
- Choosing puts the option's label in the box. While the box shows the chosen label the list opens whole; what is typed otherwise filters it. When the focus leaves, what was typed goes back to the chosen label, unless custom values are allowed. A group keeps its label over the first of its options that what is typed leaves.
- It is a control a [field](#field) holds (`FieldControl`): the field names it, judges it and disables it.
- Parts: `ComboboxPart::Trigger` (the box), `Value` (what is typed), `Placeholder`, `Chevron`, `Menu` (the panel of the list), `Item`, `Check`, `Detail`, `Hint`, `Group`, `Separator` (a select's, by the same names), `Status` (the row of no results or of loading) and `Spinner`. States: `ComboboxState::Open`, `Invalid`, `Warning`, `Valid`, `Disabled`, `FullWidth`, and of an item `Chosen` and `ItemDisabled`.
- In the design the box is a select's: 260 by 36, `Background` in an edge of 1 in `Input`, 6 round, its text 12 in, the placeholder in `MutedForeground`, two chevrons of 16 at its end; the ring's color under the pointer and a line of 2 inside it while the focus is in it or the list is open. The list is a select's: 4 under the box on `Popover`, 8 round, items of 32 on `Highlight` under the pointer or the keys, the chosen one with a check of 16 at its end. The status row is 32 high, 8 in, in `MutedForeground` at 13, its spinner 16 in `Foreground`.
- It is the engine's `Select` in its combobox mode (`Select::combobox`): one implementation of the list, its rows, groups, checks and adornments with the select's. To what reads the tree (`lurq_snapshot` of the MCP server) the box is a `combobox` with `expanded` while its list is open, the input in it a `textbox` called by the combobox's label, and the list a `listbox` of `option`s with `a11y_selected`, which says in `a11y_activedescendant` which option the keys are on.
- A renderer that draws another combobox has `view.select_style()`, `view.input()` (the text input, with what leaving it does), `view.chevron()`, `view.status()`, `view.query()`, `view.shown()` (the labels the list shows) and `view.select(style, trigger)`, which builds the combobox and calls `trigger` with a `SelectTriggerState` whose `input` is the input to place.

## Multi Select

Several values chosen from a list that what is typed filters, held as tags in the box: the labels of an issue, the people a review goes to.

```rust
use lurq::ui::MultiSelect;

MultiSelect::new(languages.clone(), [
  ("rust".to_owned(), "Rust"),
  ("wgsl".to_owned(), "WGSL"),
  ("typescript".to_owned(), "TypeScript"),
  ("swift".to_owned(), "Swift"),
])
.placeholder("Select languages")
.max_shown(3)
.clearable(true);
```

| Builder | |
| --- | --- |
| `MultiSelect::new(value, options)` | Bound to a `Signal<Vec<T>>`: the options with its values are chosen, shown as tags in the order they were chosen; choosing one adds its value, or takes it out. The options are a select's |
| `placeholder(&str)` | What the box says while nothing is chosen and nothing is typed |
| `max_shown(usize)` | How many tags the box shows: the rest are one tag that counts them, "+3" |
| `clearable(bool)` | A cross at the end of the box, while something is chosen, that takes every value away |
| `filter(..)`, `on_input(..)`, `loading(bool)`, `loading_text(&str)`, `empty(&str)`, `query(..)` | As a [combobox](#combobox) has them |
| `size(ComboboxSize)` | `Sm` 32, `Md` 36, `Lg` 40 high at least, with tags of 20, 24, 28 |
| `invalid`, `warning`, `valid`, `disabled`, `full_width`, `label`, `name`, `state`, `style`, `rules`, `force`, `id`, `key` | As a combobox has them. A disabled one keeps its tags and takes none away; `full_width` is in place of 280 |

- The box holds the chosen values as tags ([tag](#tag)s with `pill`), and a text input after them, which keeps the focus while the list is open. The tags go on to the next line where they do not fit, and the box grows with them.
- Typing opens the list and filters it; Down and Up move the highlight; Enter or a press chooses an option or takes it back, and the list stays open while one chooses, with what was typed kept, to choose another of it. A tag's cross takes its value away, and so does Backspace in the empty input, the last one's. Escape closes the list and a second Escape clears what was typed; Tab goes on, and what was typed is cleared when the focus leaves. A press of a tag or of the clear control does not open the list. Tab reaches the cross of each tag before the input.
- It is a control a [field](#field) holds.
- Parts: a combobox's, by the same names, and `MultiSelectPart::Check` (the box before a chosen option), `CheckIcon`, `Unchecked` (the box before one that is not), `Tags` (the lines of the tags and the input: its gap), `Tag` and `TagLabel` (what is laid over a tag's root and label), `Clear` and `ClearIcon`. Its states and sizes are a combobox's (`ComboboxState`, `ComboboxSize`).
- In the design the box is a select's, 280 wide and at least 36 high, padded 6 at its start, 10 at its end and 6 over and under, a chevron down of 16 at its end. A tag is 24 high on `Secondary`, 4 round, padded 8 before its text of 13 in `SecondaryForeground` and 6 after its cross of 12 in `MutedForeground`; the tags are 6 apart, across and down, and what is typed keeps 48 on their line. The list is a select's, with a box of 16 before each option, 8 before its label, 4 round: `Background` in an edge of `Input`, and, chosen, `Primary` with a check of 12 in `PrimaryForeground`.
- It is the engine's multiple `Select` in its combobox mode. To what reads the tree it is a `combobox` with `expanded`, its list a `listbox` of `option`s with `a11y_selected`, and each tag's cross a `button` called "Remove" and the tag's text; the clear control is a `button` called "Clear".
- A renderer that draws another multi select has `view.select_style()`, `view.input()`, `view.tags()`, `view.tag_rules()`, `view.clear()`, `view.chevron()`, `view.status()`, `view.chosen()`, `view.shown()`, `view.query()` and `view.select(style, trigger)`.

## For `CHANGELOG.md`

- `ui::Combobox`: a select a person types in, whose list what is typed filters (by label without regard to case, or by the app's function), with the focus in the box the whole time, Down, Up, Enter, Escape twice and Tab, options an app fetches as one types, a row of no results and one of loading, custom values, and a select's options, groups, sizes and judgments; the engine's `Select` has a combobox mode (`Select::combobox`, `free_text`, `status`), and its list says to tooling that it is a listbox of options.
- `ui::MultiSelect`: several values chosen from a list that what is typed filters, held as removable tags in a box that grows with them, with a list of checkboxes that stays open while one chooses, Backspace taking the last value away, a count past `max_shown`, a clear control, and a combobox's keys, sizes and judgments.

## Decided

- **The list is the engine's select menu, opened under an input**, not a `Popup` of rows the component builds. The engine's list already has the rows, groups and their labels and lines, the check, the leading and trailing elements built for their state, the hint and the detail, disabled options, the highlight of the keys, scrolling the highlighted row into view, flipping above the box where there is no room under it, and closing on a press outside; a second implementation would have had to keep all of that equal to the select's. What the engine lacked was a select a person types in, which is the engine's to have: `Select::combobox` puts a text input in the trigger and gives it the keys, and the select's state is what the list is drawn from.
- **The focus is the input's**: the select node is no stop of Tab, a press anywhere on it focuses the input, and the select's keys are handlers on the input that take only the keys of the list, so the caret's keys, Space and every typed character stay the input's. The box is drawn focused while its input is, as an input's own box is.
- **Typing highlights the first match**, which Enter then chooses: the combobox of the board chooses one of its options. With custom values nothing is highlighted, so Enter keeps what is typed; Down moves into the list. The highlight goes to the first option of the next render's options (`highlight_first`), because the options the app filters are known only when it renders again.
- **The whole list while the box shows the chosen label**: choosing puts the label in the box, and opening the list again lists everything rather than the chosen option alone. Anything else typed filters it.
- **What is typed is the combobox's**, as its open state is the engine's, though a builder has no `Ctx`: one signal for each signal of a value, kept while a combobox bound to it is in the tree (the input of the last one built holds what keeps the entry; when nothing holds it the entry goes). The engine knows a select across renders by the signal it is bound to in the same way. An app that wants it gives `query(Signal<String>)`. A mounted component would have needed a `Ctx` at the call site, which `Combobox::new(value, options)` has not.
- **When the value changes other than by typing** (the app sets it, or an option is chosen), what is typed becomes what the box shows of it; while it does not, what is typed is left alone.
- **Leaving the box** puts back the chosen label, or with custom values keeps what was typed: an option whose label it is, without regard to case, is chosen, and otherwise a value of `String` becomes the text. A value of another type cannot be made from text, so a combobox of one keeps the text in the box and the app hears it by `on_input`. Enter does what leaving does.
- **A press opens a combobox and leaves it open**: it is pressed to type in, and a press that toggled would close it under a press to place the caret. It closes when an option is chosen, with Escape, with Tab, or with a press outside it.
- **A press outside a combobox reaches what it lands on** (`OutsidePress::PassThrough`), as a press outside a text
  field does: it closes the list and the focus goes to what was pressed, or nowhere, and what was typed is put back
  or cleared. A select's press outside only closes its list, which keeps the focus; in a box that is typed in, the
  caret would stay in it after a press elsewhere.
- **Escape on a closed list clears what is typed as an edit** (select all, Backspace), so `on_input` hears it, and leaves the list closed. On a closed list with nothing typed, Escape is the input's: it blurs it.
- **The multi select** is the engine's multiple select as a combobox. Choosing keeps the list open and what was typed, to choose another of it; leaving clears what was typed. Its tags are `ui::Tag` with `pill` (round and without a line, as the brief asks for a token in a box), with the board's corners of 4, padding and text of 13 laid over it by the parts `Tag` and `TagLabel`, so a theme styles them in the multi select apart from other tags. A tag and the clear control stop the press, so taking a value away does not open the list. The tags are in the order the values were chosen, which is the order of the signal.
- **The boxes of the list are the engine's check slot**, the checked box as the check and the empty box as the new unchecked mark, before the label, so a row's label is where it is in a select.
- **The multi select's states and sizes are the combobox's** (`ComboboxState`, `ComboboxSize`): the same names, the same box; no enum of its own repeats them.
- **Tooling**: the combobox is the select node, whose role the engine says (`combobox`) and whose `expanded` the inspection reads from the open state, at the moment it is read; a description set at the render could not say it, since the list opens and closes without one. The input in it is a `textbox` called by the label. The option the keys are on is the listbox's `a11y_activedescendant`, as the list is built each time it is drawn and the box is not.

## Not done

Nothing.

## Differs from the board

- **Where is typed**: the board's open combobox (`XSlgv`) has a box that reads like a select's ("Select framework" and two chevrons) and a search field at the top of the list (a magnifier and "Search framework" over a line). The brief has what is typed in the box, with the focus there the whole time, and so this does: the box is the field, and the list has no field of its own.
- **The list is the select's**: the board's Listbox (`W3EUVW`) has items in a type of 13, the chosen one of a weight of 500, and fills of `Accent` and `Muted`; the open combobox and the multi select draw their rows as Menu Items (14, and `Accent` under the pointer). Both are drawn with the select's recipe here: rows of 14 on `Highlight` (the role the select's list has), the chosen one with the check, and no weight of its own.
- **What the board does not draw** was built in the manner of what it does: the box under the pointer, focused, invalid, warning, valid and disabled are a select's; the row of no results and of loading is 32 high like an item, in the muted foreground at 13, the loading one with the inline loader's spinner of 16 and text; the count past `max_shown` is a tag like the others ("+3"); the clear control is a cross of 12 in a button of 16 in the muted foreground, as a tag's cross; the multi select's sizes are a select's heights with the tag sizes of 20, 24 and 28.
- **No part of a label is marked** as what was typed: the board marks none.
