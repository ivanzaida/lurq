# Tree View

`ui::TreeView` and `ui::TreeItem`, from the board's Tree View (`LTunt` in `Ovt1D`, Components / light; used in the
Examples' dock layout, `p9Ijf`, and inside Tree Select, `LjwTK`). Branch `claude/ds-tree-view-58d44a`. This session
owned focus groups, typeahead and the keys of a tree.

## Changed outside the component's own files

- **Engine: nested focus groups.** `FocusGroup::nested(bool)` (`crates/lurq/src/node/focus_group.rs`): in a vertical
  group, Right opens a closed element and goes into an open one, Left closes an open element and goes out of a closed
  one or one that does not open, `*` opens every closed element at the focused one's level in the element it is in.
  An element says where it stands with two builder methods every component has (`crates/lurq/src/components/mod.rs`,
  next to `focus_label`): `.focus_level(n)` (1 at the top; an element with none is at 1) and
  `.focus_expanded(open, toggle)` (it opens and closes, whether it is open, and `toggle(bool)` asks for the other
  state). What an element holds is the stops after it at a higher level, up to the next one at its level or lower, so
  the group may be one flat list (a tree's rows) or elements inside elements (a sidebar's sections): only the levels
  and the order of the tree count. The keys call `toggle`; what opens comes with the next render, from the app's state.
  - `FocusStep::Open`, `Close`, `OpenSiblings` and `FocusStep::is_nested()`; `FocusGroup::step` maps ArrowRight,
    ArrowLeft and `*` to them for a nested vertical group only. A group that is not nested is as it was.
  - `node/focus_group.rs`: `pub(crate) struct FocusBranch { level, expanded, toggle }`, re-exported in `node/mod.rs`.
    `node/node.rs`: one field `focus_branch: Option<Arc<FocusBranch>>` (one pointer; few nodes have one), its default,
    its clone, `NodeUpdate::focus_level` / `focus_expanded`, `Node::focus_branch()`. `node/dsl.rs`: the default.
  - `app/runtime/nested_focus.rs` (new): `Tree::focus_group_branch_key`. `app/runtime.rs`: `mod nested_focus;` and, in
    `focus_group_key`, a hunk of five lines handing the three nested steps to it before the stops are walked, and the
    three steps named in the `match` of `next`.
  - `app/runtime/tab_navigation.rs`: the walk of a group's stops (`group_stops_around`) is one function
    (`walk_group`) that also keeps each stop's node; `group_stop_nodes(group, dropped)` returns the stops with their
    nodes. `group_stops` and `group_stops_around` are as they were.
  - `app/runtime/typeahead.rs`: typeahead reads each stop's label from the node of that walk, in place of searching the
    group for each stop by its id: in a tree of thousands of rows a letter nothing starts with was a search of the
    whole group for every row.
  - Its tests, written and not run: `crates/lurq/tests/input/tab_order/nested_focus_group.rs`, registered in
    `tests/input/tab_order/mod.rs`.
- **How the Sidebar's nested items use it**: the list of items gets
  `.focus_group(FocusGroup::vertical().nested(true).one_stop(true).typeahead(true))`; each item's button gets
  `.focus_level(depth)` (1 for a top item, 2 for one inside it), and an item that opens also
  `.focus_expanded(open, move |open| /* set it in the app's state */)`. Its children may be in a `Collapsible`'s panel
  or a column under the button, as long as they come after it in the tree and are not there while it is closed (a
  `Collapsible`'s panel is not, as it is closed). The sidebar session should not change the engine for this. If an
  item's children are left in the tree while it is closed (to animate their height out), they are still stops until
  they leave: wrap them in `FocusGroup::none()` while closing, or drop them at once.
- The Menubar session's row of menus is in `focus_group.rs` and `focus_group_key` too: its hunks and these are apart,
  but both touch `FocusStep` and the `match` of `next` in `focus_group_key`.
- **Engine: a focus group of rows a list builds only in part.** `pub trait FocusRows` (`node/focus_group.rs`,
  re-exported from `lurq::node`): `count`, `label`, `enabled`, `level`, `expanded`, `set_expanded`, `page`,
  `reveal`. The element of a group gives them with `.focus_rows(Arc<dyn FocusRows>)`, and each row's stop says which
  row it is with `.focus_row(index)` (both builder methods in `components/mod.rs`, after `focus_expanded`, stored in
  the node's `FocusBranch`, which gained the fields `row` and `rows`; `NodeUpdate::focus_row` / `focus_rows` in
  `node/node.rs`). Its keys step over the rows: Up, Down, Home, End, PageUp and PageDown (`FocusStep::PageBack`,
  `PageOn`, taken only by a group with rows: the search for the group in `focus_group_key` passes a group without
  them by for those two), Right, Left, `*` and typeahead. A row that is built is focused at once; one that is not is
  asked for (`reveal`) and focused after the layout that has it.
  - `node/focus_group.rs`: `FocusRows`, `pub(crate) fn step_rows(step, current, count, page, wrap, enabled)` (the
    one step over rows, which the tree's Shift selection uses too), the two new `FocusStep`s and `is_page`.
  - `app/runtime/nested_focus.rs`: `PendingRowFocus`, `Tree::focus_rows_key`, `Tree::focus_row`,
    `Tree::pending_rows_key`, `Tree::apply_pending_row_focus`, and the helpers `same_rows`, `group_of_rows`,
    `row_stop`, `row_parent`; the nested `match` names the page steps.
  - `app/runtime.rs`: the fields `pending_row_focus` and `within_states` on `Tree` and in its constructor;
    `focus_group_key` asks `pending_rows_key` first, filters the page steps in its group search, hands a group with
    rows to `focus_rows_key`, and names the page steps in the `match` of `next`; `update_layout` calls
    `apply_pending_row_focus` after `blur_focus_in_dropped_child`; `mouse_down_with_meta`, `focus_tab` and
    `apply_focus_request_inner` drop a pending row (a press, Tab and the app's focus are where the keys go on from).
  - `app/runtime/typeahead.rs`: a group with rows matches over the rows (`focus_group_typeahead`); `find` takes a
    count and a label of each, and the search over stops uses it too. `typeahead::label` is as the merge left it.
  - `components/virtualized_list.rs`: `VirtualizedList::reveal_request(u64)` (an option, its runtime's
    `last_reveal_request`, and the condition in `rows` that queues a reveal): the row of `reveal_key` is revealed
    again each time the number changes, for a list asked twice for one row it was scrolled away from between.
  - `app/runtime/tab_navigation.rs`: no function was touched this time; the rows use `group_stop_nodes` from before.
- **Engine: interaction groups, a part styled by the hover and the focus of the element it is in.**
  `.interaction_group(true)` on an element and `.within_style(Style)` on what it holds (builder methods in
  `components/mod.rs`, after `when_disabled`): while the pointer is on the group, or the focus is on it or in it,
  the elements in it show their `within_style`; an element in two groups answers to the innermost. Paint only, as
  the disabled style is.
  - `node/style.rs`: `StateStyles::within`. `node/interaction_state.rs`: the flag `within`, `is_within`,
    `set_within`, `same`. `node/node.rs`: `RareStyle::interaction_group`; `NodeUpdate::within_style` /
    `interaction_group`; `Node::set_style_within`, `is_interaction_group`, `has_within_style`; `state_style`,
    `state_style_value` and `effective_box_shadow` read the within style, under every other state.
    `layout/layout_engine.rs`: the plain-node check of the paint fast path also asks for no within style.
  - `app/runtime/interaction_groups.rs` (new): `Tree::refresh_interaction_groups`, which lights what the groups under
    the pointer and around the focus hold and puts out what the others held (the tree keeps the states it lit).
    Called from `apply_reactive_updates_after_event` (every event), and in `pass_inner` before the cached frame is
    tried and after the layout.
  - `ui/recipe.rs`: `Selector::within`, `Recipe::within(part, ..)`, `PartStyles::within`, resolved apart from the
    other interactions, given to the node by `ResolvedPart::apply`, laid under a forced interaction by `rest()`, and
    handed on by `rules_for`.
  - Tests, run: `tests/input/tab_order/focus_rows.rs` and `tests/input/interaction_group.rs`, registered in their
    `mod.rs` files.
- **How a sidebar's item and a table's row use it**: the item's (or the row's) pressed element gets
  `.interaction_group(true)`, and what shows only on hover is a part whose recipe hides it at rest and shows it
  `within`: `.base(Action, |s| s.opacity(0.0)).within(Action, |s| s.opacity(1.0))
  .density(Density::Touch, Action, |s| s.opacity(1.0))`, as the tree's `Reveal` part. A table's row is the group of
  its cells: a cell's action shows while the row is pointed at or a cell of it has the focus.
- For `docs/src/content/docs/focus-navigation.md`, Focus Groups, a second paragraph for the merge: "A group of rows a
  list builds only in part, a virtual list of thousands, gives the rows to the group with `.focus_rows(rows)`, an
  `Arc` of an `impl FocusRows` (how many rows, each one's label, level, whether it is open and takes the focus, a
  page, and a way to build and show a row), and each built row says which it is with `.focus_row(index)`. The keys
  step over the rows, not over what is built: PageUp and PageDown move by a page, and a row that is not built is
  asked for and has the focus once a layout has it. Keys pressed meanwhile go on from it; a press, Tab or the app's
  focus stop the wait. `TreeView::mount` is one."
- For the docs page of state styles (`docs/src/content/docs/styling.md`, beside `disabled_style`), a paragraph for
  the merge: "`interaction_group(true)` makes an element a group, and `within_style(style)` (`Recipe::within` in a
  recipe) is what an element in it shows while the pointer is on the group or the focus is on it or in it: an
  action at the end of a row that shows only then. An element in two groups answers to the innermost. It paints
  only."
- `docs/src/content/docs/focus-navigation.md`, Focus Groups, wants a paragraph the merge adds (this session does not
  edit `docs/`): "A nested group (`FocusGroup::vertical().nested(true)`) is a tree: each element says its level with
  `.focus_level(n)`, and one that opens says so with `.focus_expanded(open, toggle)`. Right opens a closed element and
  goes into an open one, Left closes an open one and goes out of a closed one to the element it is in, `*` opens every
  element at the focused one's level. What an element holds is the elements after it at a higher level; what a closed
  one holds is not there, so the other keys go through what is shown."
- `app/runtime.rs`: `DOUBLE_CLICK_INTERVAL` and `DOUBLE_CLICK_DISTANCE` are `pub(crate)`, for the tree's own double
  press (see Decided).
- No keyframes were registered (none is used).

## For `docs/src/content/docs/design-system.md`

## Tree View

Things inside things, one level under the other: the files of a project in their folders, the layers of a scene, settings in their groups.

```rust
use lurq::ui::{TreeItem, TreeView};

// open: Signal<Vec<&str>>, selected: Signal<Option<&str>>
TreeView::new([
  TreeItem::folder("crates", "crates").children([
    TreeItem::folder("lurq", "lurq").children([
      TreeItem::file("lib.rs", "lib.rs"),
      TreeItem::file("theme.rs", "theme.rs"),
    ]),
  ]),
  TreeItem::file("Cargo.toml", "Cargo.toml"),
])
.label("Files")
.expanded(open)
.selected(selected)
.on_activate(|file| open_file(file));

// Any number selected, with Ctrl and Shift.
TreeView::new(project).expanded(open).selected(many).multiple(true);

// Children the app brings when the row is first opened.
TreeItem::folder("registry", "registry").lazy(true);

// What stands at a folder's end while its row is pointed at or focused.
TreeItem::folder("src", "src").trailing(new_file_button).trailing_on_hover(true);

// Ten thousand rows: in the engine's virtual list, which builds those in view.
TreeView::new(items)
  .expanded(open)
  .style(TreeViewPart::Root, |s| s.height(480.0))
  .mount(ctx);
```

| Builder | |
| --- | --- |
| `new(items)` | The rows at the top, each a `TreeItem<T>` with what it holds. `T` is what rows are told apart by: a path, an id |
| `TreeItem::new(id, label)`, `folder(id, label)`, `file(id, label)` | A row; a folder has the icons of a folder and of an open one, a file the icon of a file |
| `TreeItem::icon(name)`, `open_icon(name)` | The icon before the label, and another while the row is open |
| `TreeItem::trailing(element)` | What stands at the row's end: a count, a badge |
| `TreeItem::trailing_on_hover(bool)` | What stands at the row's end shows only while the row is pointed at or the focus is in it, and always at the touch density: an action |
| `TreeItem::children(items)`, `child(item)` | What the row holds |
| `TreeItem::lazy(bool)` | It has children the app brings when it is first opened: it opens as a folder does, with a spinner in place of its chevron while it is open with none |
| `TreeItem::disabled(bool)`, `element_id(id)` | A row that takes nothing; the id of the row's element |
| `expanded(Signal<Vec<T>>)` | The rows that are open. Opening a row adds it, closing one takes it out |
| `expanded_ids(ids, toggle)` | The same held by the app: `toggle(&id, open)` is told of each row a press or a key asks to open or close |
| `selected(signal)` | What is selected: a `Signal<Option<T>>`, or a `Signal<Vec<T>>` with `multiple` |
| `selected_ids(ids, change)` | The same held by the app: `change(next)` is told what is to be selected |
| `multiple(bool)` | Any number selected, with Ctrl and Shift as a list of files has them |
| `guides(bool)` | A line down each level, under the chevron of the row it is in |
| `label(&str)` | What the tree is called, to what reads the tree. It draws nothing |
| `on_select(handler)` | Told of the row a press or Space selects |
| `on_activate(handler)` | Told of the row a double press or Enter activates: open the file. A row that opens opens or closes as well |
| `on_expand(handler)` | Told of each row as it opens: where a lazy row's children are brought |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the root; `force` draws every row so |
| `mount(ctx)` | In place of converting it: the tree in the engine's virtual list, for thousands of rows. Give it a height |

- What is open and what is selected are the app's, as an [accordion](#accordion)'s open items are: a signal the tree reads and writes, or the app's own state and a handler. The rows are one flat list of what is shown: what a closed row holds is not built, and it opens and closes at once, as a file tree does.
- A press of a row selects it, a press of its chevron opens or closes it (and does not select it), a double press activates it. With `multiple`, a press with Ctrl (Cmd) adds the row or takes it out, one with Shift selects the rows from the last selected to it. The last of the selection is where a range starts.
- The tree is one stop of Tab, at the selected row or the first. Up and Down move through the rows that are shown, Home and End to the first and the last, without going round. Right opens a closed row and goes into an open one; Left closes an open row and goes out of a closed one or a leaf to the row it is in; `*` opens every row at the focused one's level. A letter goes to the next row that starts with it. Space selects the focused row (with `multiple` it adds it or takes it out; with Shift it selects to it), Enter activates it. With `multiple`, Shift with Up, Down, Home or End moves and selects the rows on the way, and Ctrl+A selects every row shown. A disabled row is passed by.
- The keys are the engine's: the tree is a [focus group](../focus-navigation/#focus-groups) that is `nested`, `one_stop`, with `typeahead` and without `wrap`, and each row says its level and whether it is open with `focus_level` and `focus_expanded`. A list of items in items (a sidebar's) gets the same keys the same way.
- `mount(ctx)` puts the rows in the engine's virtual list, which builds the rows near its view: a tree of ten thousand rows costs about one screen of them. The keys still reach every row: the tree gives its rows to its focus group ([`FocusRows`](../focus-navigation/#focus-groups)), so Up, Down, Home, End, PageUp and PageDown (a view of rows), Right, Left, `*` and typeahead go to any row, the list builds and shows the row they go to, and the focus goes to it once it is built. Shift with them and Ctrl+A select over every row. The list fills the tree, which is given a height; its rows are drawn by `TreeRow::render`, so a theme's rules reach them and its renderer of the whole tree does not.
- What an item marks with `trailing_on_hover` is in a `Reveal` part, hidden at rest and shown `within` the row: each row is an interaction group (`interaction_group`), lit while the pointer is on it or the focus is in it. At the touch density, where nothing points, it always shows.
- To what reads the tree it is a `tree` (`a11y_multiselectable` with `multiple`) of `treeitem`s, each with `a11y_level`, `a11y_posinset` and `a11y_setsize` among the rows of its level in its row, `a11y_selected`, `a11y_expanded` where it opens and `a11y_busy` while its children are on their way.
- Parts: `TreeViewPart::Root` (the rows, one under the other), `Row` (what is pressed, selected and focused), `Indent` (one step of a row's indent, for each level it is in), `Guide` (the line in a step), `Content` (what a row holds after its indent), `Chevron` (also the room of one in a row that does not open, and the spinner's), `Icon`, `Label`, `Trailing`, `Reveal` (around what is at the end of a row marked to show it on hover). States of a row: `TreeViewState::Selected`, `Expanded`, `Disabled`.
- In the design a row is 28 high, 2 under the one before, with `radius-sm` and 8 at each side; each level moves it in by 16. It holds a chevron of 14 in `MutedForeground` (right while it is closed, down while it is open), an icon of 16 in `MutedForeground` and its label in the body's type at 13, 6 apart. The selected row is on `Highlight`, its label at 500 and its icon in `Foreground`. Under the pointer and held down a row has the `StateHover` and `StatePressed` veils, the focus from the keyboard is a ring of 2 in `Ring` inside it, and a disabled row is half opaque. A guide is a line of 1 in `Border` under the middle of the chevron of the row it is in, as high as a row and the room under it. What is at a row's end is 12 in `MutedForeground`.
- A theme's renderer is given `TreeViewView`: `view.rows` (each a `TreeRow` with its label, level, position, whether it opens, is open, is loading, is selected, is disabled, its icon and what is at its end), `view.root()` (the column with the keys and its role) and, on each row, `row.button()` (the behavior with the row's style), `row.indent()`, `row.chevron()`, `row.icon()`, `row.label()` and `row.render()`. It is set for `TreeView<T>` of any `T`: `theme.set_renderer::<TreeView<String>>(..)` draws trees of every id.

## For `CHANGELOG.md`

- `ui::TreeView` and `ui::TreeItem`: things inside things, a file tree, with folders and files, an open icon, what stands at a row's end, disabled rows, children an app brings when a row is first opened, guide lines, one selected or many with Ctrl and Shift, activation by a double press or Enter, and the keys of a tree from the engine's new `FocusGroup::nested` with `focus_level` and `focus_expanded`, which a sidebar's nested items use too; typeahead in a long group no longer searches the group for each element. `TreeView::mount(ctx)` puts a tree of thousands of rows in the virtual list with every key reaching every row, and `TreeItem::trailing_on_hover` shows what is at a row's end only while the row is pointed at or focused.
- Engine: `FocusRows` (`.focus_rows(..)` on a group, `.focus_row(i)` on its rows) for a focus group of rows a list builds only in part, whose keys, PageUp and PageDown among them, and typeahead reach rows not built and focus them once they are; `VirtualizedList::reveal_request`; interaction groups, `.interaction_group(true)` with `.within_style(..)` and `Recipe::within`, for what shows only while the element it is in is pointed at or holds the focus.

## Decided

- **A builder, not a mounted component.** What is open and what is selected are the app's: a signal (`expanded`, `selected`) or what the app holds and a handler (`expanded_ids`, `selected_ids`), as the accordion's and the radio group's. A tree given neither shows its rows closed and selects nothing; that is said in the docs. The anchor of a range with Shift needs no state of the tree's: it is the last of the selection, and a range is written with its anchor last.
- **`TreeView<T>` is generic over the id**, as `AccordionItem<T>` is, and the theme keeps its recipe and renderer by name: `theme.set_renderer::<TreeView<String>>(..)` reaches trees of every `T`, and `TreeViewView` and `TreeRow` are not generic (their behavior is in closures).
- **`TreeView::new(items)` and `selected(signal)` take either signal** through `TreeSelection<T>` (`From<Signal<Option<T>>>`, `From<Signal<Vec<T>>>`): the brief's one method for one or many. `multiple(true)` with a `Signal<Option<T>>` selects one.
- **The rows are one flat list** of what is shown: in a plain `Column` where the tree is converted into an element, each row a stop of the one focus group; in the engine's virtual list with `mount(ctx)`, the rows a `FocusRows` the group steps over. Two forms and not one: the virtual list needs a `Ctx` and a height, which a tree of fifty rows in a dialog should not have to give.
- **The virtual list's rows are components keyed by where the item is** (`0/3/1`): `T` need not be shown as a text. Every render of the tree draws the rows in view again (they hold the selection's closures), which is a screen of rows. The list reveals a row it had not built at the top of its view, as its `reveal_key` does; a row the keys step to that is built is scrolled into view by the least.
- **The group's keys and the tree's Shift selection take one step**: `step_rows` in the engine, so Shift+PageDown selects to the row PageDown goes to. PageUp and PageDown are a group's only where it has rows: a group that builds every row leaves them to what scrolls.
- **Interaction groups are the engine's, not the tree's**: a node's state, as disabled is, kept by the tree for the groups under the pointer and around the focus. What shows `within` paints only: it may not size a part, so a row does not move as its action comes and goes. The touch density shows it always, by a density rule of the recipe, since a finger points at nothing until it presses.
- **A row opens at once**, with no height transition: a file tree does, and the board draws a tree that is open or closed, not on its way. The chevron is the board's two icons, right and down, and not one icon turned: the board draws two, and a part cannot be turned by a recipe.
- **The focus and the selection are apart**: the arrows move the focus and Space selects, as the brief says and as a tree with many selected must; a press selects and focuses. **Activating a row that opens opens or closes it** as well as telling `on_activate`, as Enter and a double press on a folder do in a file manager.
- **A row selects as the button goes down, and counts its own double press.** The engine holds back the click of an
  element that has an `on_dblclick` (or is in one) until the double press's interval has passed, and the focus a
  click gives with it: with `on_dblclick` on the rows, a press selected and focused a row half a second late, and a
  press of another row in that time focused the row before. So a row selects on `on_mouse_down`, and a second press
  of the same row within the engine's interval and distance (`DOUBLE_CLICK_INTERVAL`, `DOUBLE_CLICK_DISTANCE`, as a
  window that sets no other has them) activates it; Space still selects through the row's click. A press of the
  chevron is not a first press of the row. A window's own double-click interval (`Tree::set_double_click_interval`)
  is not read by the tree: it would take the window's interval in reach of a handler.
- **A press of the chevron opens and closes and does not select**, as the button goes down; a press of the rest of the row selects and does not open. A double press of the row activates it.
- **States the board does not draw**: under the pointer and held down are the veils every pressed part of the system has; the focus is a ring inside the row (an outline 2 in, as a select's item and a tab's have), since rows touch one another; a disabled row is faded and is not a stop of the keys, as a disabled radio or menu item is. Disabled rows still open and close by what the app holds, and show what they hold when open.
- **Guide lines are built and off by default**: the board's tree draws none. Each is in a step of the indent, under the chevron of the row it is in, and reaches 2 under its row so that the lines of a level are not broken by the room between rows.
- **What is at a row's end is the app's element**, in the muted foreground at 12, always shown or, with `trailing_on_hover`, while the row is pointed at or focused.
- **No renaming, no dragging** of rows: the board draws neither; an app renames in a dialog or puts an input in a row's place.
- **Tree Select (`LjwTK`)** would take the whole tree: a select's trigger (the path chosen, `crates / lurq / lib.rs`) and a popover holding a `TreeView` with `selected`, closing on `on_select` of a leaf. Its panel on the board is the same rows in a popover of 8 padding. **Tree Table (`YiE9E`)** would take a row's indent, chevron and keys (`TreeRow::indent`, `chevron`, the nested focus group with `focus_level` and `focus_expanded`) in its first column, with the other columns of a table row; its rows are 30 high, its chevron 12, and it draws no icons. **Column View (`aCNzy`) and Path Control (`COHly`)**, beside them on the board, take nothing of it but the items.
- **The gallery's page** is in Navigation, with a playground (one or many selected, icons, guides, lazy children, a disabled row, an action on hover), the board's file tree, many selected, children that come a second after they are asked for, guide lines, ten thousand rows in the virtual list, an action on hover, and the states with a badge at a row's end.

## Not done

Nothing.

## Differs from the board

- The board's tree is 220 wide; the component has no width of its own and its rows are as wide as the tree is given (`style(TreeViewPart::Root, |s| s.width(220.0))` for the board's).
- Under the pointer, held down, focused from the keyboard, disabled, a guide, what is at a row's end and the spinner of a lazy row are not drawn on the board: decided above.
