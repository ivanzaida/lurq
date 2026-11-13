# Tabs: hand-off

`ui::Tabs` and `ui::Tab`, from the board's reusable components in Components / light (`Ovt1D`, section "Tabs",
`reuzJ`): `Tabs` (`hKzBj`, the default: a muted rail), `Tabs/Line` (`ZcYHq`, with counts and a divider before the
archived tab), `Tabs/Pill` (`qBtJe`), `Tabs/Vertical` (`w4bN8N`, the line variant down the side) and `Tabs/Overflow`
(`D7nzv`, "overflow · scroll and more menu"), each used again in Components / dark (`z2R0d`). The board draws the
selected tab, counts (`hKzBj` with its counts turned on, `ZcYHq` with them), a divider, the list down the side and the
overflowing form; it draws no hover, focus, disabled tab, icon, badge or close control for these tabs (its "Tab
hover" is the window tabs', which were not built). What lurq has for what the board does not draw is under "Decided"
and "Differs from the board".

Files: `crates/lurq/src/ui/tabs.rs`, `crates/lurq/tests/ui/tabs.rs`, `crates/gallery/src/pages/tabs.rs` (page "Tabs",
group Navigation), this file, and lines in the three `mod.rs` files.

## Changed outside the component's own files

- **`PartStyle::start_line(Border)`** (`crates/lurq/src/ui/part_style.rs`): a line along the start side of a part, the
  counterpart of `underline` (which is the bottom), for the mark of the selected tab and the line of the list when the
  list runs down the side (`Tabs/Vertical` strokes the left side only). A field `start_line`, its builder, and a line
  each in `is_empty`, `merge`, `apply` (`border_left`) and `state_style` (`border_left`). The docs' Part Styles table
  wants a row: "`start_line(border)` | A line along the start side of the part only: what marks the selected tab of a
  list that runs down".
- **`a11y_selected` reported by `lurq_inspect`** (`crates/lurq/src/mcp/tools/inspect.rs`, `semantic_state`): a node
  described with `a11y_selected` reports `state.selected` as `true` or `false`, as `a11y_checked` reports
  `state.checked`. The docs' `mcp.md` paragraph on `lurq_inspect` wants "one with `a11y_selected`, such as a tab of the
  design system, reports `state.selected`".
- **No change to the focus groups, Tab order or selection.** `FocusGroup` with `activate` and `one_stop`, the selected
  tab's `focus_entry`, Home and End, a disabled tab passed by, and the focused tab scrolled into view in a list that
  scrolls sideways (`scroll_into_view` takes both axes) were all there.
- `lurq::components::Tabs` was read and not changed.

## For `docs/src/content/docs/design-system.md`

## Tabs

A list of tabs of which one is selected, and the panel of the selected one under it or beside it: the sections of a
settings page, the views of a record, the filters over a list.

```rust
use lurq::ui::{Tab, Tabs, TabsOrientation, TabsVariant, icons};

// section: Signal<&'static str>. Selecting a tab sets it.
Tabs::new(section, [
  Tab::new("general", "General").panel(|| general()),
  Tab::new("appearance", "Appearance").icon(icons::SETTINGS).panel(|| appearance()),
  Tab::new("advanced", "Advanced").disabled(true),
])
.label("Settings");

// A row over a line, with counts, the archived set apart.
Tabs::new(memory, [
  Tab::new("all", "All").count(7),
  Tab::new("memory", "Memory").count(9),
  Tab::new("archived", "Archived").count(0).separated(true),
])
.variant(TabsVariant::Line);

// The app holds which is selected, and is told of each other one selected.
Tabs::controlled(filter, [(0, "Overview"), (1, "Activity")], move |index| set_filter(index))
  .variant(TabsVariant::Pill)
  .orientation(TabsOrientation::Vertical);
```

| Builder | |
| --- | --- |
| `Tabs::new(Signal<T>, tabs)` | The selected tab is the one with the signal's value; selecting another sets it. A tab is a `(value, label)` or a `Tab` |
| `Tabs::controlled(value, tabs, select)` | The app holds which is selected: the tab of `value`, and `select` is told the value of each other tab selected |
| `variant(TabsVariant)` | `Default`: a muted rail with the selected tab lifted out of it. `Line`: a row over a line, the selected tab underlined. `Pill`: round tabs, the selected one in the primary fill |
| `orientation(TabsOrientation)` | `Horizontal` (the default): the list along the top, the panel under it. `Vertical`: the list down the start side, the panel beside it |
| `manual(bool)` | The arrow keys move the focus alone, and Enter or Space selects: for panels slow to build. Otherwise the selection follows the focus |
| `label(&str)` | What the list is called to what reads the tree |
| `overflow(TabsOverflow)` | The overflowing form: the list scrolls between two scroll buttons, and More at its end opens a menu of every tab. `TabsOverflow::new(ctx.signal(false))` is made once, in the app's `create` |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `disabled` makes every tab disabled. `id` is the root's; in the overflowing form the scroll buttons are `<id>-back` and `<id>-on` and the More button `<id>-more` |
| `Tab::new(value, label)` | A tab. `icon(name)` before the label, `count(n)` after it, `disabled(bool)`, `separated(bool)` a divider before it, `panel(Fn() -> Element)` what it shows while selected, `id` (its item in the More menu is `<id>-item`) |

- Exactly one tab is selected, and the app holds which. A press of a tab selects it; a press of the selected one or of a disabled one asks for nothing.
- The list is one stop of Tab, at the selected tab. The arrow keys of the orientation (Left and Right, or Up and Down) select the tab after and the one before, Home and End the first and the last, and a disabled tab is passed by; with `manual(true)` they move the focus and Space or Enter selects. Tab from the list goes to the panel, and on to what is in it.
- The panel of a tab is built only while the tab is selected, keyed by the tab, so one tab's panel never takes another's nodes. What the runtime keeps of a panel's nodes (a scroll offset, a caret, an open select) goes when another tab is selected; what the app keeps in its own signals stays. A panel that is to come back as it was keeps its state in the app: its values in signals made in the app's `create`, its scroll in a `ScrollState` it gives the scroll area (`with_scroll_state`).
- The panel is a stop of Tab itself, so a panel with nothing focusable in it is still reached and read; its ring is the focus ring.
- Parts: `TabsPart::Root` (the list and the panel), `List`, `Tab` (what is pressed and focused), `Label`, `Icon`, `Count`, `Divider`, `Panel`, and in the overflowing form `Bar` (the row of the list, its buttons and More), `ScrollButton`, `ScrollIcon`, `More`, `MoreLabel`, `MoreIcon`. A tab's parts are styled in its states: `TabsState::Selected`, `Disabled`; the whole has `Horizontal` or `Vertical`, and `Overflow`. Variants: `TabsVariant::Default`, `Line`, `Pill`.
- In the design the default list is a rail of `Muted`, `radius-lg`, padded 3, its tabs 2 apart; a tab is 28 high, padded 12, `radius-md`, its label 13/500 in `MutedForeground` and a count of 12 in `MutedForeground` 6 after it; the selected tab is `Background` with the small shadow and its label in `Foreground`. The line list is 44 high over a line of 1 in `Border`, as wide as what holds it, its tabs 28 apart; a label is 14 in `MutedForeground` and a count 13 at 0.7, 8 apart; the selected tab is underlined with 2 in `Foreground`, its label 500 in `Foreground` and its count whole. A divider is 1 by 16 in `Border`. Down the side the line list is at least 180 wide with a line of 1 in `Border` at its start, a tab 32 high, padded 12, its label 13, and the selected tab marked with 2 in `Foreground` at its start. The pills are 30 high, padded 14, round, 4 apart, the label 13/500; the selected one is `Primary` with its label in `PrimaryForeground`. The panel is 16 under the list, or 24 beside it.
- The overflowing form is a row 40 high (over the line in the line variant): the list, its tabs 20 apart with labels of 13, between two scroll buttons 28 wide with chevrons of 14 in `Foreground`, and More at the end, set apart by a line of 1 in `Border`, padded 8 and 4, its label 13 and a chevron of 14 in `MutedForeground`. A scroll button moves the list most of the way it shows; it is for the pointer and no stop of Tab, since the arrow keys scroll the list to the tab they select. More opens a menu of every tab (`ui::Menu`), the selected one marked as a radio and a disabled one not to be chosen; choosing one selects it and scrolls the list the least that shows it.
- A tab under the pointer has its label in `Foreground`; a disabled tab is dimmed and does not light up. The focus ring is the tab's; in the overflowing form it is drawn inside the tab, where the list that scrolls does not cut it off.
- To what reads the tree (`lurq_snapshot` of the MCP server) the list is a `tablist` called by `label`, with `a11y_orientation`; a tab is a `tab` called by its label, with `a11y_selected` (which `lurq_inspect` reports as `state.selected`); the panel a `tabpanel` called by its tab. More has `a11y_haspopup` `menu` and `a11y_expanded`.
- A renderer that draws other tabs has `view.tabs` (each with its label, icon, count, and whether it is selected, disabled, set apart), `view.part(part)`, `view.tab_part(index, part)`, `view.tab(index)` (a tab with its behavior and its name to tooling), `view.list()` (the tabs as one focus group), `view.divider()`, `view.panel()` and `view.bar()` (the overflowing form).
- `lurq::components::Tabs` is another thing: a set of places of an app, each a router with a history of its own kept while it is not shown, with a bar to move between them across the bottom or the top. Tabs of the design system are a choice within a page.

## For `CHANGELOG.md`

- `ui::Tabs`: a list of tabs of which one is selected, with the panel of the selected one, as a muted rail, a row over a line with counts, or pills, along the top or down the side; one stop of Tab whose arrow keys select, an icon, a count, a disabled tab, a divider, and an overflowing form whose list scrolls between two buttons with a menu of every tab; `PartStyle::start_line`.

## Decided

- **A builder, in the manner of `SegmentedControl`.** `Tabs::new(Signal<T>, tabs)` and `Tabs::controlled(value, tabs, select)`, built on `Chosen` from `shared.rs`; a tab is a `Tab<T>` with its value, or a `(value, label)`. Nothing is kept but what the app gives.
- **Not built on `lurq::components::Tabs`.** That is a mounted set of routers, each tab a history of screens kept offstage while another is shown, with a bar for an app's places (a phone's tab bar, a window's top bar), its items picked by string keys, needing a `Ctx`. The design system's tabs are a choice within a page bound to the app's `Signal<T>`, whose panels are plain elements; built on the engine's set they would need a router per panel and a component to mount. Neither is a second implementation of the other.
- **The selection follows the focus** (`FocusGroup::activate(true)`), as the segmented control's does and as the WAI's tabs pattern recommends where a panel is quick to show; `manual(true)` turns it off for panels that are slow to build (the group without `activate`: the arrows move the focus, Space and Enter press).
- **A press of the selected tab asks for nothing**, so `controlled`'s handler is told only of a change.
- **Each tab is resolved in its own case** (selected or not, disabled or not: four resolutions), the whole in its variant, its orientation and whether it overflows. The orientation is a pair of states, `Horizontal` and `Vertical`, since a rule cannot say "not vertical" and the line variant draws a bottom line in one and a start line in the other.
- **The panel is built only while its tab is selected** and keyed by the tab's place, so that one tab's panel never reconciles into another's nodes. Keeping every panel and hiding the others would need a node that is kept out of layout and input without a `Ctx` (the engine has that only as `Ctx::mount_offstage`); an app that needs a panel's state across switches keeps it, as the docs section says.
- **The panel is a stop of Tab** (`tab_index(0)`), as the WAI pattern has a panel whose first content is not focusable; its ring is the focus ring, of `radius-md`. A panel full of focusable things is one stop more.
- **The root is as wide as what holds it** (`Length::Fill`), so the line list's line runs across the width as the board's 820 does, and a vertical panel has the room beside the list. The rail and the pills stay as wide as their tabs, at the start.
- **The rings of the tabs are drawn past the list** (the list and the root do not clip), except in the overflowing form, whose list scrolls and so clips: there the ring is a line of 2 in `Ring` inside the tab (the input's way), and the More button's too.
- **Icons** are 14 (`IconSize::Sm`), before the label, 6 from it in the default and pill variants and 8 in the line variant: the gap the board gives a label and its count. **Counts** are text, in the caption's 12 in the default variant and 13 at 0.7 in the line variant as the board has them; a pill's count is `PrimaryForeground` at 0.7 while selected. A count is anything that displays: "12", "99+".
- **A divider** before a tab is `Tab::separated(true)`, as the board sets the archived tab apart: 1 by 16 in `Border`, with the list's gap on each side; down the side it is a line of 1 across the list.
- **Hover**: the label in `Foreground`, the segmented control's hover; the selected pill keeps `PrimaryForeground`, a disabled tab keeps its color.
- **The overflowing form** is the board's: the list between two scroll buttons and a More menu. The builder cannot know which tabs are out of view, so the menu lists every tab, the selected one marked; choosing one there selects it and scrolls the list the least that shows it, from where the list and the tab were last laid out (an element ref on each, made fresh in each build). The scroll position, More's anchor and whether the menu is open are the app's, in a `TabsOverflow` it makes once. A scroll button moves the list by 0.8 of the room it shows. The board draws it in the line variant; the default and pill variants take the same bar without the lines. A list down the side scrolls with what holds it and takes no notice of `overflow`.
- **The board's `Tabs/Overflow` numbers are the line variant's in a smaller row** (40 high, labels of 13, tabs 20 apart), so they are rules of the line variant in the `Overflow` state, not a variant of their own.
- **More is called "More tabs"** to what reads the tree; the scroll buttons "Scroll back" and "Scroll on".

## Not done

Nothing. The tests in `crates/lurq/tests/ui/tabs.rs` were written and compiled, not run, as the batch's flow has it.

## Differs from the board

- The board draws no hover, focus, disabled tab, icon or panel: what lurq has for them is under "Decided". It draws no close control or badge on these tabs (the window and document tabs, not built, have them), so a tab has neither; a count is the board's only mark.
- The scroll buttons of the overflowing form are always drawn, as the board draws them; they do not hide or dim at the ends of the list, which a builder cannot see.
- The More menu lists every tab, not only those out of view.
- The list down the side is at least 180 wide, as the board's is, and wider for a longer label.
