# List, Listbox and Data List

`ui::ListItem` with `ui::List` (the board's Contained List as a form of the list), `ui::Listbox` and `ui::DataList`,
from the board's List Item (`t58ShI`), Listbox (`W3EUVW`), Contained List (`fSY1s`) and Data List (`t8MqZ`) in `Ovt1D`
(Components / light), read with the Sortable List (`l3Ne8i`), the mobile rows (`sGO20`, `JuLIF`, `H57hLk`) and the
Settings Row (`qYnUc`) beside them. This session owned selection in a list and the keys of a listbox.

## Changed outside the component's own files

- **Engine: the page keys in a focus group.** `FocusGroup::pages(bool)` (`crates/lurq/src/node/focus_group.rs`): in a
  vertical group PageUp and PageDown move the focus to the stop as far back or on as the innermost scroll that runs
  down around the focused one shows, and at least one on; a group that does not scroll is one page (the keys go to
  its ends). `FocusStep::PageBack` and `PageOn`, mapped in `FocusGroup::step` only where `pages` is on. The walk is
  `crates/lurq/src/app/runtime/page_focus.rs` (new): `Tree::focus_group_page(stops, current, on)`, one walk of the
  last layout for the stops' heights and the scroll's. `app/runtime.rs`: `mod page_focus;` and the two steps in the
  `match` of `next` in `focus_group_key`. `app/runtime/nested_focus.rs`: the two steps named with the others that are
  not a nested group's.
- **Engine: Shift with the keys presses with Shift.** `FocusGroup::shift_extends(bool)`: a key that moves the focus
  with Shift held presses what it moves to with Shift (`activate_focused_button(modifiers)`), as a press of the
  pointer with Shift does: the one hunk in `focus_group_key` is `if group.activate || (group.shift_extends &&
  modifiers.shift)`. So Shift with the arrows, Home, End and the page keys extends a range through the row's own press
  handler, wherever the engine moves the focus (by a page as well), and no component counts rows on its own.
- **Engine: typeahead chooses where the choice follows the focus.** `app/runtime/typeahead.rs`: in a group with
  `activate`, the element a letter moves to is pressed too, as an arrow's is. A listbox of one chooses by its first
  letters.
- **Engine: the router of what is being built.** `crate::router::current_navigator()` (`router/navigator.rs`, re-exported
  from `router`; `mod navigator` is `pub(crate)` now): the `Navigator` `Ctx::navigator` would give, for a builder that
  has no `Ctx` (a list item with `href`). `Ctx::begin_render` pushes the navigator of the component's contexts on a
  thread-local stack and `end_render` pops it, and `Ctx::provide` of a `Navigator` during a render (the router's own)
  replaces the top. All of it is under `#[cfg(feature = "router")]`.
- **The range and toggle model, moved out of the tree view and shared.** `crates/lurq/src/ui/selection.rs` (new):
  `Pick` (`Only`, `Toggle`, `Range`; `Pick::of(&MouseEvent, multiple)` reads Shift, Ctrl/Cmd and Space from the
  keyboard), `ListSelection<T>` (the rows in order, which take input, what is selected with the anchor last, what
  writes the next; `pick(index, pick)`, `select_all()`), and `select_all_key(&KeyboardEvent, select_all)` for Ctrl+A.
  `ui/tree_view.rs` uses it: its `Model` holds a `ListSelection` with its `on_select` and `on_activate`, its own
  `Pick`, `Reach` and `extend` are gone, and its root group has `.shift_extends(self.multiple)` in place of a key
  handler that counted rows for Shift with Up, Down, Home and End. What it does is the same, and with many selected
  `on_select` is now told of the row Shift with a key reaches too, as it is of one a press with Shift reaches.
  **What a table's rows would call:** build one `ListSelection::new(ids, enabled, selected, multiple, write)` per
  render, give each row's press `selection.pick(index, Pick::of(&event, multiple))` and its `on_key_down`
  `select_all_key(&event, &select_all)`, and give the rows' group `FocusGroup::vertical().one_stop(true)
  .shift_extends(multiple).pages(true)`.
- `crates/lurq/src/components/select.rs`: three `pub(crate)` readers on `SelectOption` for a list that draws its options
  itself: `detail_text()`, `hint_text()`, `adornments(&SelectAdornmentState)`.
- `crates/lurq/src/ui/shared.rs`: `impl From<Choice<T>> for SelectOption<T>` (its label, its icon as the leading
  element, disabled), so a listbox takes `Choice`s as a toggle group does.
- Tests of the engine, written and not run: `crates/lurq/tests/input/tab_order/list_keys.rs`, registered in
  `tests/input/tab_order/mod.rs` (`input_tests`).
- The tree view session works in `focus_group.rs`, `focus_group_key` and `typeahead.rs` too (a virtualized focus
  group): these hunks are apart from theirs, but both name steps in the `match` of `next`.
- `docs/src/content/docs/focus-navigation.md`, Focus Groups, wants a paragraph the merge adds: "A group with `pages`
  moves the focus by a page with PageUp and PageDown: as far as the scroll it is in shows. A group with
  `shift_extends` presses what a key moves the focus to with Shift, when Shift is held, so that a list selects a range
  with Shift and the keys as it does with Shift and a press. In a group whose choice follows the focus (`activate`),
  typeahead chooses as well."
- No keyframes were registered.

## For `docs/src/content/docs/design-system.md`

## List

Rows one under the other: settings, people, files, notifications. A row has a title, a description under it, what stands before it and what stands at its end.

```rust
use lurq::ui::{Avatar, List, ListItem, Switch, icons};

List::new()
  .item(
    ListItem::new("Ivan Zaida")
      .description("Pushed 3 commits to feat/node-blur")
      .leading(Avatar::new("Ivan Zaida"))
      .meta("2m"),
  )
  .item(ListItem::new("Dependabot").description("Bumped wgpu to 26.0").selected(true));

// Settings: a value and a chevron where a row leads somewhere, a switch where it holds one.
List::new()
  .dividers(true)
  .inset(40.0)
  .item(ListItem::new("Wi-Fi").icon(icons::WIFI).value("Home").chevron(true).on_click(|| open_wifi()))
  .item(ListItem::new("Dark mode").icon(icons::MOON).trailing(Switch::new("Dark mode").label_hidden(true)));

// A contained list: the rows in a box, under a header.
List::new().contained(true).header("Recent").item(ListItem::new("theme.md").chevron(true).href("/docs/theme"));
```

| Builder | |
| --- | --- |
| `ListItem::new(title)` | A row that says `title`, with nothing before or after it |
| `description(&str)` | A second line under the title, in the muted color |
| `leading(element)`, `icon(name)` | What stands before the text: an avatar, a picture; or an icon of the theme's set |
| `checkbox(bool)`, `radio(bool)` | A checkbox or a radio before the text, checked or not: the row is that control, which its press changes through `on_click` |
| `value(&str)`, `meta(&str)` | A value at the end in the muted color ("Home"), or a note in the mono type ("2m") |
| `trailing(element)`, `shortcut(keys)`, `chevron(bool)` | What the app puts at the end (a switch, a badge), the keys that do what the row does, and a chevron for a row that leads somewhere |
| `on_click(handler)`, `href(path)` | What a press does, or where it goes in the app's router: the row is pressed |
| `selected(bool)`, `disabled(bool)`, `density(Density)` | The selected row is highlighted; a disabled one is dimmed and takes nothing; how high it is, whatever the theme is at |
| `List::new()`, `item(row)`, `items(rows)` | The rows, one under the other |
| `dividers(bool)`, `inset(f32)` | A line between each two rows, and how far from the start it starts |
| `contained(bool)`, `header(&str)`, `action(element)` | The rows in a box, touching, under a header with a title (in capitals) and an action |
| `label(&str)`, `density(Density)`, `force(..)` | What the list is called to what reads the tree; the density of every row; every row drawn as under the pointer or focused |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them, on the row and on the list |

- A row given `on_click` or `href` is pressed: by the pointer, and by Enter and Space while it is focused, and Tab stops at it. A row that is not pressed is not a stop of Tab and answers no pointer: a control in it (a switch at its end) is pressed on its own. In a list Up and Down, Home and End move between the rows that are pressed and the controls in rows, and a letter to the next whose title starts with it.
- A row with `href` goes to its path in the router around it (`router::current_navigator`), after its `on_click`; outside a router it goes nowhere.
- What is selected is the app's: `selected(true)` on the row. A list of which one or many are chosen by a person is a [listbox](#listbox).
- To what reads the tree a list is a `list` called by its label or its header. A row is a `listitem`; a pressed one a `button`, a `link` with a path, a `checkbox` or `radio` with one before its text (with `a11y_checked`), and a selected one says `a11y_selected`.
- Parts of a row: `ListItemPart::Root` (what is pressed, selected and focused), `Leading`, `Icon`, `Content`, `Title`, `Description`, `Trailing`, `Value`, `Meta`, `Chevron`. States: `ListItemState::Selected`, `Disabled`, `Pressable`, `TwoLine`, `WithIcon`, `Contained`. Parts of a list: `ListPart::Root`, `Header`, `HeaderTitle`, `HeaderAction`, `Items`, `Divider` (what holds a line; its padding at the start is the inset). States: `ListState::Contained`, `Divided`.
- In the design a row is 36 high (28, 44 and 56 at the compact, comfortable and touch densities), 12 in, with the medium radius and 12 between what it holds; with a description it is 16 higher, 52 at the default, its title at 500. The title is the body's type at 14, the description 12 in `MutedForeground` 2 under it, a value 13 and a chevron of 14 in `MutedForeground`, a note the mono type at 11, an icon 16 in `MutedForeground`. The selected row is on `Highlight`. A pressed row under the pointer and held down has the `StateHover` and `StatePressed` veils, the focus from the keyboard is a ring of 2 in `Ring` inside it, and a disabled row is half opaque. Under a finger a row is the board's mobile row: 56 high, 16 in, its title at 16, its value at 15, its chevron 18, its icon of 18 on a tile of 32 on `Muted` with the large radius.
- Rows are 2 apart; with lines between them they touch, a line of 1 in `Border` from the inset to the end. A contained list is a box on `Background` with an edge of 1 in `Border` and the large radius; its header is 32 high on `Muted`, 12 in, with a line under it, its title the mono type at 11 in `MutedForeground`, 1 apart; its rows are square and their titles 13.
- A theme's renderer is given `ListItemView` (`view.root(children)`, `view.leading()`, `view.content()`, `view.trailing()`, `view.render()`) and `ListView` (`view.root()`, `view.header()`, `view.rows()`, `view.divider()`).

## Listbox

A list in the page of which one option is chosen, or any number: a renderer, a time zone, the columns a table shows.

```rust
use lurq::{components::SelectOption, ui::Listbox};

// renderer: Signal<&str>
Listbox::new(renderer, [
  SelectOption::new("wgpu", "wgpu").group("Renderers"),
  SelectOption::new("dx12", "DirectX 12"),
  SelectOption::new("metal", "Metal"),
  SelectOption::new("gl", "OpenGL (legacy)").disabled(true),
])
.label("Renderer");

// Any number: a Signal<Vec<&str>>.
Listbox::new(columns, [("name", "Name"), ("size", "Size"), ("kind", "Kind")]).label("Columns");

// A long list scrolls in a box with a most height.
Listbox::new(zone, zones).style(ListboxPart::Root, |s| s.max_height(240.0));
```

| Builder | |
| --- | --- |
| `new(value, options)` | A `Signal<T>` of which the option of its value is chosen, or a `Signal<Vec<T>>` of which those of its values are; options as a [select](#select) takes them: `(value, label)`, a `Choice`, or a `SelectOption` with `group`, `detail`, `hint`, `leading`, `trailing`, `disabled` |
| `follow_focus(bool)` | Of one, whether the arrows choose as they move. They do unless told otherwise; without it Space, Enter and a press choose |
| `on_change(handler)` | Told what is chosen after each press or key that changes it |
| `label(&str)` | What it is called, to what reads the tree. A [field](#field) gives it its label |
| `invalid(bool)`, `disabled(bool)`, `full_width(bool)`, `density(Density)` | Its edge in `Destructive`; no option chosen; as wide as what holds it, in place of 220; how high an option is |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the box; each option's is that id, `-` and its place from 0 |

- It is one stop of Tab, at the chosen option or the first. Up and Down move through the options, Home and End to the first and the last, PageUp and PageDown by what the box shows, without going round, and a letter to the next option that starts with it; a disabled option is passed by.
- Of one, the choice follows the focus, as in a radio group: a key that moves the focus chooses what it moves to, a letter too. With `follow_focus(false)` the focus and the choice are apart: the focus moves alone and Space, Enter or a press choose.
- Of many, a press chooses one alone, with Ctrl (Cmd) it adds or takes out one, with Shift it chooses from the last chosen to it. Space adds or takes out the focused option; Shift with the arrows, Home, End and the page keys chooses from where the choice started to where the focus goes; Ctrl+A chooses every option that can be. The last of the `Vec` is where a range starts.
- The keys are the engine's: the options are a [focus group](../focus-navigation/#focus-groups) that is `one_stop`, with `typeahead`, `pages` and without `wrap`; one of one `activate`s, one of many `shift_extends`. The range and the toggle are `ui::selection`'s, which the [tree view](#tree-view) shares.
- To what reads the tree it is a `listbox` (`a11y_multiselectable` of many) called by its label, with `option`s that say `a11y_selected`, `a11y_posinset` and `a11y_setsize`; a group's label is a `group`.
- Parts: `ListboxPart::Root` (the box; its most height makes the options scroll), `Content` (the options with the keys), `Group`, `Option`, `Leading`, `Label`, `Detail`, `Hint`, `Trailing`, `Check`. States: `ListboxState::Chosen`, `OptionDisabled` (of an option), `Disabled`, `Invalid`, `Multiple`, `FullWidth`.
- In the design the box is 220 on `Background` with an edge of 1 in `Border` and the large radius, its options 1 apart, 4 in. An option is 32 high (24, 40 and 44 at the compact, comfortable and touch densities), 8 in, with the small radius, its label in the body's type at 13. The chosen one is on `Highlight`, its label at 500, with a check of 14 at its end; the one under the pointer is on `Muted`; one held down has the `StatePressed` veil; the focus from the keyboard is a ring of 2 in `Ring` inside it; a disabled one says its label in `MutedForeground`, faded. A group's label is 11 at 600 in `MutedForeground`, 6 over it, 4 under it, 8 in.
- A theme's renderer is given `ListboxView`: `view.options` (each a `ListboxOption` with its label, group, detail, whether it is chosen and disabled, and `option.button()`, `option.group_label()`, `option.text()`, `option.check()`, `option.render()`), `view.content()` (the column with the keys) and `view.root(content)`. It is set for `Listbox<T>` of any `T`.

## Data List

Terms with their values: what a thing is, item by item. The details of a build, of an order, of a file.

```rust
use lurq::ui::{Badge, DataList};

DataList::new()
  .text("Version", "0.60.0")
  .text("Renderer", "wgpu")
  .text("License", "MIT")
  .item("Status", Badge::new("Stable").variant("success"));

// Each term over its value, for a narrow column.
DataList::new().vertical(true).text("Ship to", "Ivan Zaida, 12 Khreshchatyk St, Kyiv");
```

| Builder | |
| --- | --- |
| `text(term, value)` | An item whose value is a text, in the value's type |
| `item(term, element)` | An item whose value is what the app puts there: a badge, a link, tags |
| `vertical(bool)` | Each term over its value, in place of beside it |
| `label(&str)` | What the list is called, to what reads the tree |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them |

- Side by side, the terms are one column as wide as the widest of them and the values the column of the rest of the width (a grid of two columns, `Auto` and `Fr(1.0)`); a line runs under each item but the last, the term's and the value's meeting. One over the other, each item is a column.
- To what reads the tree each term is a `term` and each value its `definition`.
- Parts: `DataListPart::Root`, `Item` (an item one over the other), `Term`, `Value`. States: `DataListState::Vertical`, `Divided` (of an item's parts: a line under it).
- In the design each term and its value are 10 from the lines over and under them and 12 apart, both in the body's type at 13, the term in `MutedForeground` and the value in `Foreground`; the line is 1 in `Border`. One over the other the term is 12, 2 over its value.
- A theme's renderer is given `DataListView`: `view.items` (each a term with its text or its element), `view.term(item, lined)`, `view.value(item, lined)` and `view.render()`.

## For `CHANGELOG.md`

- `ui::ListItem` and `ui::List`: rows of a list with a title, a description, what stands before them (an avatar, an icon, a checkbox, a radio) and at their end (a value, a note, a switch, keys, a chevron), pressed or not, a path in the router, a height for each density with the mobile row under a finger, lines between rows from an inset, and the contained list in a box with a header; `router::current_navigator` for a builder that goes somewhere.
- `ui::Listbox`: a list in the page of which one option or many are chosen, from a `Signal<T>` or a `Signal<Vec<T>>` and what a select takes, with groups, disabled options, and the keys of a listbox: one stop, the choice following the focus or waiting for Space, the page keys, typeahead, and ranges with Shift, Ctrl with a press and Ctrl+A; the engine's `FocusGroup::pages` and `FocusGroup::shift_extends`, and one model of ranges in `ui::selection` that the tree view now shares.
- `ui::DataList`: terms with their values, side by side with the terms as wide as the widest or one over the other, a value a text or any element.

## Decided

- **Builders, not mounted components.** What is chosen and what is selected are the app's: a listbox reads and writes the
  app's signal, as a select does, and a row is `selected` by the app. The anchor of a range needs no state: it is the
  last of the `Vec`, and a range is written with its anchor last (the tree view's rule, now shared).
- **Of one, the choice follows the focus.** The board's listbox stands in a form (Forms & Fields): it is a field's
  value, as a radio group is, which lurq's radio group chooses with the arrows. `follow_focus(false)` gives the other
  for a choice that costs something as it is made. Of many, the focus and the choice are apart.
- **A plain press of many chooses one alone**, with Ctrl adding or taking out one: the brief's model, a list of files'.
  An app that wants a press to add or take out writes it in `on_change`, or uses a [checkbox group](#checkbox).
- **The listbox takes what a select takes**: `SelectOption<T>`, `(value, label)` and now `Choice<T>`, so that a group's
  label, a detail, a hint and elements before and after the label are written one way for both.
- **Shift with the keys is the engine's**, through a press with Shift, so that the page keys extend a range too and a
  table gets it by one flag; the tree view moved to it.
- **The list's rows are a focus group** (vertical, without wrap, with typeahead) but not one stop: a list of pressed
  rows is a list of buttons or links, each a stop of Tab, which the arrows move between too. A listbox is one stop.
- **Density sets a row's height** (the theme's densities for the list item, a sidebar's heights for a listbox's
  option), and at `Density::Touch` a list item is the board's mobile row: its numbers are the board's 56, 16, 16, 15,
  18 and the tile of 32. A row with a description is 16 higher at every density: the board gives 52 at the default.
- **A checkbox or a radio before a row's text is the row's own**, drawn with the checkbox's and the radio's recipes as a
  choice card draws them: the row is the control, one stop, and its press checks it.
- **The contained list's header is written in capitals**, as the board's RECENT and the mobile groups' NETWORK are: the
  app gives "Recent".
- **The data list's terms are as wide as the widest**, through a grid with an `Auto` column: the board's fixed 100 is a
  drawing of that. No engine change was needed, so the Table session's shared columns are not used.
- **The Sortable List** (`l3Ne8i`) is rows that are dragged: a row restyles into its row (`Root` on `Card` with an edge
  of 1 in `Border` and the medium radius, 36 high, a grip icon as `icon`), and the dragging is drag and drop's, not the
  list's. **The Settings Row** (`qYnUc`) is a row with a description and a control at its end
  (`ListItem::new(..).description(..).trailing(Switch::new(..))` in a list with `dividers`).
- **The gallery's pages** are in Data: List (a playground of every prop and the board's people, a contained list,
  settings, rows under a finger, the states), Listbox (one, many, a choice that waits, a long list with the page keys,
  the states) and Data List (side by side, one over the other, values that are elements; read and not operated).

## Not done

- **A listbox of thousands of options.** The options are one `Column` in a scroll: every option is built at each render,
  which is fine for hundreds. The tree view session is adding a virtualized focus group (a list that tells the engine
  how many rows it has, so that the keys reach rows that are not built). With it a listbox would put its options in
  `components::VirtualizedList` from `Listbox::mount(ctx)`, give the group the count and a key of each option, and
  keep the rest as it is: `ListSelection` takes ids, not built rows, so a range and Ctrl+A over rows that are not built
  need nothing more; `FocusGroup::pages` would have to step over the virtual count by the viewport's height divided by
  an option's (the walk in `page_focus.rs` reads built stops only).

## Differs from the board

- The board's list item is 52 with an avatar and a description; a row of one line, which the board draws only in the
  contained list (36) and on mobile (56), is 36 at the default density, and a description adds 16 at every density.
- The board's Settings Row is 12 over and under and 14 in; as a list item it is 6 over and under at least, 52 high and
  12 in. Its title is 13 there and 14 here (the List Item's).
- The mobile Select Row's check is a round mark of 24 on `Primary` before the text; `checkbox(true)` draws the design
  system's checkbox. An app gives the round mark with `leading(..)`.
- The mobile grouped list's header is over its box, not in it: the app puts its heading over a contained list without a
  header, and gives the list `style(ListPart::Root, |s| s.radius(RadiusSize::Xl))` for the board's corners.
- The board's data list gives the terms a column of 100; here it is as wide as the widest term.
- Not drawn on the board, decided above: a row under the pointer, held down, focused or disabled; an option held down
  or focused; a disabled or invalid listbox; a listbox of many; a data list one over the other.
