# Select: hand-off

`ui::Select`, one value chosen from a list that opens under a trigger (board: `fte2x` Select, with `iPsCT` Menu and
`q4kDx` Menu Item for its list, in "Select & Menu" of `Components / light`).

## Changed outside the component's own files

The engine's select draws its trigger and its list itself, from a `SelectStyle`. Five things the board draws were
out of a recipe's reach, so the engine's select gained them, each as small as it could be. None changes what an app
that does not use it sees, apart from the last.

- **A ring on the trigger.** The board's focus and open look is a line of 2 inside the box in `Ring`, the one
  `ui::Input` has (an outline of 2 at an offset of -2). As the slider's session did for its thumb:
  `node/select_style.rs`: `SelectPartStyle` has an `outline: Option<Outline>` and a builder `outline(Outline)`,
  merged like its other fields; `layout/layout_engine/select_quads.rs`: `push_select_trigger_quads` keeps the
  resolved trigger's outline as a ring through `push_ring`, so `trigger_focused` and `trigger_open` can be a ring.
  The menu and the rows do not paint an outline.
- **Groups.** The board's list has groups with a label over each and a line between them. `components/select.rs`:
  `SelectOption::group(label)` starts a group at the option. `node/select_style.rs`:
  `SelectStyle::group_label(part)` (the label's row: padding, fill, type, color) and `SelectStyle::separator(part)`
  (its `background` is the line's color, `min_height` its thickness, 1 unset, `padding` the room around it).
  `app/runtime/select_menu.rs`: the menu puts the separator before a group's first option, unless it is the first
  row, and the label after it unless the label is empty; those rows are not options, so the keys and the pointer
  do not reach them, and the option the menu scrolls into view is found by where its row is.
  `node/select_state.rs` and `node/node.rs`: the labels are kept with the options (`set_extras`,
  `select_option_extras`).
- **A hint.** The board's options have a short note at their end ("beta", "soon", in the mono type).
  `SelectOption::hint(text)`, drawn after the trailing element in `SelectStyle::option_hint(part)` laid over the
  row's style; the closed trigger does not show it.
- **The gap of the trigger.** `SelectStyle::trigger_gap(gap)`, the room between the trigger's label and its
  chevron, which was a literal 8 in `default_trigger`; it stays 8 unset.
- **A trailing check keeps its room on the chosen row only** (`select_menu.rs`, `option_row`). Where the mark is
  after the label in a single select, the rows that are not chosen no longer keep an empty slot of its width, so
  their labels and trailing elements run to the row's end, as the board draws them. A leading mark and a multi
  select keep the slot on every row, so labels stay in line.
  `tests/input/select/style.rs::trailing_checkmark_sits_after_the_label` still holds: it compares where the labels
  start.

The engine's docs (`styling-events.md`, Select) could say `group`, `hint`, `group_label`, `separator`,
`option_hint`, `trigger_gap` and `SelectPartStyle::outline`; they were not edited here.

## For `docs/src/content/docs/design-system.md`

## Select

One value chosen from a list that opens under a trigger shaped like an input: a theme, a time zone, a page size.

```rust
use lurq::{components::SelectOption, ui::{Select, SelectSize}};

// theme: Signal<String>
Select::new(theme.clone(), [
  ("system".to_owned(), "System"),
  ("light".to_owned(), "Light"),
  ("dark".to_owned(), "Dark"),
]);

// Nothing chosen until the signal has an option's value; an option can have a detail, or be disabled.
Select::new(zone.clone(), [
  SelectOption::new("Europe/Kyiv".to_owned(), "Kyiv").detail("UTC+2"),
  SelectOption::new("Asia/Tokyo".to_owned(), "Tokyo").disabled(true),
])
.placeholder("Select a time zone")
.size(SelectSize::Sm)
.invalid(zone.get().is_empty());

// Groups with a label over each and a line between them, and a hint at an option's end.
Select::new(platform.clone(), [
  SelectOption::new("windows".to_owned(), "Windows").group("Desktop"),
  SelectOption::new("linux".to_owned(), "Linux"),
  SelectOption::new("ios".to_owned(), "iOS").group("Mobile").hint("beta"),
]);
```

| Builder | |
| --- | --- |
| `new(Signal<T>, options)` | What it is bound to, and its options: `(value, label)` pairs or `SelectOption`s (`detail`, a second line; `hint`, a note at its end; `leading` and `trailing` elements; `group(label)`, the group it starts; `disabled`). The option whose value the signal has is chosen; choosing another sets the signal |
| `placeholder(&str)` | What the trigger says while no option is chosen |
| `size(SelectSize)` | `Sm` 32, `Md` 36 (the default), `Lg` 40 high, with text of 13, 14, 15: an input's sizes |
| `invalid(bool)`, `warning(bool)`, `valid(bool)` | What the app judged of what is chosen: the edge and the ring in the destructive, warning or success color. One is shown, in that order, as an input shows them |
| `full_width(bool)` | As wide as what holds it, in place of 260 |
| `label(&str)` | What it is called, to what reads the tree. The field it is in shows its label |
| `name(..)` | The name of its value in a form, the chosen option's label (feature `form`) |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the select's node. `state(SelectState::Open, true)` draws the trigger as it is while open, for a gallery |

- It is the engine's [`Select`](../styling-events/#select) dressed by the recipe, not a second one: a press, or Enter, Space and the arrows from the keyboard, open the list under it (over it where there is no room); the arrows, Home, End and the first letters move through it, skipping disabled options and the groups' labels; Enter or a press chooses, and Escape closes. Tab reaches it wherever it is (`tab_index(0)`); a disabled one does not open and Tab passes it by.
- It stands beside an [input](#input): the same box, sizes, edge, ring and judgments. The ring is a line of 2 inside the box in `Ring`, shown from the keyboard and while the list is open.
- Parts: `SelectPart::Trigger` (the select's node and box: its size, edge, fill, padding, ring, opacity, cursor, the type of what it says, and in its gap the room before the chevron), `Value` and `Placeholder` (laid over the trigger's type and color), `Chevron` (its side and color), `Menu` (the panel: fill, edge, corners, shadow, padding; its gap is the room under the trigger, its height the most it is before it scrolls), `Item` (an option: height, padding, corners, type, color; `hovered` is the item under the pointer or the keys; its gap is the room before the check), `Check` (the chosen item's mark), `Detail`, `Hint`, `Group` (a group's label), `Separator` (the line between groups: its fill, its height, and in its padding the room around it). States: `SelectState::Open`, `Invalid`, `Warning`, `Valid`, `Disabled`, `FullWidth`, and of an item `Chosen` and `ItemDisabled`. Sizes: `SelectSize::Sm`, `Md`, `Lg`.
- In the design the trigger is 260 by 36, `Background` in an edge of 1 in the `Input` color, `radius-md`, the text 12 from the edge in the body's type, the placeholder and a chevron of 16 in `MutedForeground`, 8 apart; `Ring` under the pointer; `Destructive`, `Warning` or `Success` for a judgment, under the pointer, in the focus and open too; disabled is half opaque on `Muted`. The list is 4 under it and as wide, on `Popover` in an edge of `Border`, `radius-lg`, with the theme's middle shadow and 4 of padding. An item is 32 high, its text 8 in, `radius-sm`, in the body's type, on `Highlight` under the pointer or the keys; the chosen one has a check of 16 in `Foreground` at its end, 8 after the label; a disabled one is at 0.45. A hint is 11 in the mono type in `MutedForeground`. A group's label is 11, semibold, in `MutedForeground`, in line with the items' text; a line of 1 in `Border` is between groups, 4 from each.
- The engine paints the trigger, the panel and the rows itself, from its `SelectStyle`, not from nodes the recipe styles: of the trigger it takes the fill, the edge, the corners, the shadow, the ring and the padding, at rest, `hovered`, `focus_visible` and in the state `Open`; of an item, the fill, edge, corners, padding, height (at least), type, color and opacity, at rest, `hovered` and in the states `Chosen` and `ItemDisabled`; of a group's label, its padding, fill, type and color; of the separator, its fill, height and padding. A transition is not painted.
- To what reads the tree (`lurq_snapshot` of the MCP server) it is a `combobox`, called by its label where it has one.
- A renderer that draws another select starts from `view.select_style()`, the engine's `SelectStyle` the recipe makes, changes what it likes (another chevron, a menu as wide as its widest option), and builds it with `view.select(style)`, the engine's select bound to the value with the trigger's size, opacity and cursor.

## For `CHANGELOG.md`

- `ui::Select`: one value chosen from a list that opens under a trigger shaped like an input. The engine's select dressed by a recipe, bound to a `Signal<T>` with `(value, label)` pairs or `SelectOption`s, with the sizes and the judgments of an input (`invalid`, `warning`, `valid`), `placeholder`, `full_width` and `disabled`, the parts `Trigger`, `Value`, `Placeholder`, `Chevron`, `Menu`, `Item`, `Check`, `Detail`, `Hint`, `Group`, `Separator` and the states `Open`, `Invalid`, `Warning`, `Valid`, `Disabled`, `FullWidth`, `Chosen`, `ItemDisabled` for a recipe, and `SelectView::select_style()` and `select(style)` for a renderer. The engine's select has groups (`SelectOption::group`, `SelectStyle::group_label` and `separator`), a hint at an option's end (`SelectOption::hint`, `SelectStyle::option_hint`), `SelectStyle::trigger_gap` and a ring on its trigger (`SelectPartStyle::outline`); a trailing check keeps its room on the chosen row only.

## Decided

- The builder is not generic: `Select::new(value, options)` takes the signal and the options together, as
  `RadioGroup::new` does, and keeps the engine's select to be built (a closure over `Signal<T>`), so `Select`,
  `SelectView` and the theme's renderer are one type for every `T`. The options are the engine's `SelectOption<T>`,
  re-used rather than a `Choice<T>`: they have the detail line, the hint, the group, the elements beside the label
  and the disabled flag the engine draws.
- What the board draws as the chosen item, the highlight's fill and a check, is two things: the highlight is the
  item under the pointer or the keys (the board's Menu draws `accent` on such an item), the check is the chosen
  one. On opening, the keys' highlight is on the chosen item, which is the board's picture.
- The item under the keys looks as the one under the pointer (the engine does that when `option_highlighted` is
  not set); a recipe has no separate interaction for it.
- The open look of the trigger is a state of the recipe (`Open`), resolved apart and given to the engine as
  `trigger_open`: the whole trigger as the open case has it, merged last, so it wins over the hover and the focus.
- One judgment is shown, what is wrong over what is doubtful over what is right, as `ui::Input` does; each keeps
  its color under the pointer, in the focus and while open.
- `Menu`'s gap is the room between the trigger and the list (`menu_gap`), its height the most the list is
  (`max_menu_height`; the recipe does not set it, and the engine's 240 holds); `Item`'s gap is the room before the
  check and beside an option's elements; `Trigger`'s the room before the chevron.
- Groups are said on the option that starts one (`SelectOption::group`), not as a nested list: the options stay one
  list in the engine, so the keys, type-ahead, the chosen index and the signal are as they were, and a group is
  only rows drawn before an option.
- The text of the trigger is the trigger's type (as the input's control has it), with `Value` and `Placeholder`
  over it; the sizes change the trigger's height and font size, as the input's.
- The disabled option is at the board's 0.45, a number of the board's; the whole select disabled is the theme's
  `OpacityLevel::Disabled` (0.5), which the board also draws.
- No `on_change`: the select is bound to the app's signal, as the engine's is. An app that wants to be told
  watches the signal (the gallery's playground does, with `ctx.watch`).
- No read only: a select has none on the web or on the board; an app that shows a value it does not let be changed
  disables the select or shows the value as text.
- The engine's select is a combobox to the MCP tree already; the recipe adds only `a11y_name` from `label`.
- In a field it takes part as `ui::Input` does: `invalid(bool)`, `warning(bool)` and `valid(bool)` set its
  judgment, and `label(&str)` its accessible name (`a11y_name`), the four props a `FieldControl` trait can set.

## Not done

Nothing the board draws for a select, and nothing of what one does, is left out.

The select's ui tests (`cargo test -p lurq --features form,router --test ui_tests select`, 12) and the engine's
select tests (`--test input_tests select`, 125) pass after the merge of the overlays batch. Four of the ui tests
had failed at the merge, all for the tests' own two faults, not the component's:

- Their scenes' roots were as small as the select: the headless stage lays a scene out with loose constraints, and
  the open list, a layer drawn below the select, fell outside the root's box, where nothing is painted. A window
  gives its root the whole window, so the tests now do (`window()`, a column of the stage's 400 by 300), as
  `theming.rs` does for its select.
- With the list painted, the label of the chosen option is painted twice, in the trigger and in the list, and the
  tests measured the trigger's: they now read the list's, the one painted last.

## Differs from the board

- A group's label is padded 5 over and 5 under; the board pads it 6 over and 4 under, which puts its text 1 lower
  in a row of the same 23. A part's padding is one number for both (`PartStyle::padding_y`). The smallest change
  that removes it is a `padding_top` and `padding_bottom` in `PartStyle` (`ui/part_style.rs`, applied in `apply`
  and read by `engine_part` in `ui/select.rs`).
- The trigger's ring while open is the focus ring of the input (an outline of 2 inside the box); the board draws a
  stroke of 2, which looks the same.
- An item's corners are `radius-sm` on every item; the engine rounds the first and the last item to the panel's
  corners only where an item has no radius of its own, so this does not change the board's look.
- The check is the theme's `check` icon in the color the recipe gives (`Foreground`).
- The cursor over the trigger is a pointer, the default while disabled; the board does not say.
