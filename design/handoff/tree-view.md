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
```

| Builder | |
| --- | --- |
| `new(items)` | The rows at the top, each a `TreeItem<T>` with what it holds. `T` is what rows are told apart by: a path, an id |
| `TreeItem::new(id, label)`, `folder(id, label)`, `file(id, label)` | A row; a folder has the icons of a folder and of an open one, a file the icon of a file |
| `TreeItem::icon(name)`, `open_icon(name)` | The icon before the label, and another while the row is open |
| `TreeItem::trailing(element)` | What stands at the row's end: a count, a badge |
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

- What is open and what is selected are the app's, as an [accordion](#accordion)'s open items are: a signal the tree reads and writes, or the app's own state and a handler. The rows are one flat list of what is shown: what a closed row holds is not built, and it opens and closes at once, as a file tree does.
- A press of a row selects it, a press of its chevron opens or closes it (and does not select it), a double press activates it. With `multiple`, a press with Ctrl (Cmd) adds the row or takes it out, one with Shift selects the rows from the last selected to it. The last of the selection is where a range starts.
- The tree is one stop of Tab, at the selected row or the first. Up and Down move through the rows that are shown, Home and End to the first and the last, without going round. Right opens a closed row and goes into an open one; Left closes an open row and goes out of a closed one or a leaf to the row it is in; `*` opens every row at the focused one's level. A letter goes to the next row that starts with it. Space selects the focused row (with `multiple` it adds it or takes it out; with Shift it selects to it), Enter activates it. With `multiple`, Shift with Up, Down, Home or End moves and selects the rows on the way, and Ctrl+A selects every row shown. A disabled row is passed by.
- The keys are the engine's: the tree is a [focus group](../focus-navigation/#focus-groups) that is `nested`, `one_stop`, with `typeahead` and without `wrap`, and each row says its level and whether it is open with `focus_level` and `focus_expanded`. A list of items in items (a sidebar's) gets the same keys the same way.
- To what reads the tree it is a `tree` (`a11y_multiselectable` with `multiple`) of `treeitem`s, each with `a11y_level`, `a11y_posinset` and `a11y_setsize` among the rows of its level in its row, `a11y_selected`, `a11y_expanded` where it opens and `a11y_busy` while its children are on their way.
- Parts: `TreeViewPart::Root` (the rows, one under the other), `Row` (what is pressed, selected and focused), `Indent` (one step of a row's indent, for each level it is in), `Guide` (the line in a step), `Content` (what a row holds after its indent), `Chevron` (also the room of one in a row that does not open, and the spinner's), `Icon`, `Label`, `Trailing`. States of a row: `TreeViewState::Selected`, `Expanded`, `Disabled`.
- In the design a row is 28 high, 2 under the one before, with `radius-sm` and 8 at each side; each level moves it in by 16. It holds a chevron of 14 in `MutedForeground` (right while it is closed, down while it is open), an icon of 16 in `MutedForeground` and its label in the body's type at 13, 6 apart. The selected row is on `Highlight`, its label at 500 and its icon in `Foreground`. Under the pointer and held down a row has the `StateHover` and `StatePressed` veils, the focus from the keyboard is a ring of 2 in `Ring` inside it, and a disabled row is half opaque. A guide is a line of 1 in `Border` under the middle of the chevron of the row it is in, as high as a row and the room under it. What is at a row's end is 12 in `MutedForeground`.
- A theme's renderer is given `TreeViewView`: `view.rows` (each a `TreeRow` with its label, level, position, whether it opens, is open, is loading, is selected, is disabled, its icon and what is at its end), `view.root()` (the column with the keys and its role) and, on each row, `row.button()` (the behavior with the row's style), `row.indent()`, `row.chevron()`, `row.icon()`, `row.label()` and `row.render()`. It is set for `TreeView<T>` of any `T`: `theme.set_renderer::<TreeView<String>>(..)` draws trees of every id.

## For `CHANGELOG.md`

- `ui::TreeView` and `ui::TreeItem`: things inside things, a file tree, with folders and files, an open icon, what stands at a row's end, disabled rows, children an app brings when a row is first opened, guide lines, one selected or many with Ctrl and Shift, activation by a double press or Enter, and the keys of a tree from the engine's new `FocusGroup::nested` with `focus_level` and `focus_expanded`, which a sidebar's nested items use too; typeahead in a long group no longer searches the group for each element.

## Decided

- **A builder, not a mounted component.** What is open and what is selected are the app's: a signal (`expanded`, `selected`) or what the app holds and a handler (`expanded_ids`, `selected_ids`), as the accordion's and the radio group's. A tree given neither shows its rows closed and selects nothing; that is said in the docs. The anchor of a range with Shift needs no state of the tree's: it is the last of the selection, and a range is written with its anchor last.
- **`TreeView<T>` is generic over the id**, as `AccordionItem<T>` is, and the theme keeps its recipe and renderer by name: `theme.set_renderer::<TreeView<String>>(..)` reaches trees of every `T`, and `TreeViewView` and `TreeRow` are not generic (their behavior is in closures).
- **`TreeView::new(items)` and `selected(signal)` take either signal** through `TreeSelection<T>` (`From<Signal<Option<T>>>`, `From<Signal<Vec<T>>>`): the brief's one method for one or many. `multiple(true)` with a `Signal<Option<T>>` selects one.
- **The rows are one flat list** of what is shown, in a plain `Column`, each row a stop of the one focus group: Up, Down, Home, End, `*`, typeahead and Shift with the keys all work over the shown rows with no state of the tree's. Not the engine's virtual list: see Not done.
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
- **What is at a row's end is the app's element**, always shown, in the muted foreground at 12. An action shown only under the pointer is in Not done.
- **No renaming, no dragging** of rows: the board draws neither; an app renames in a dialog or puts an input in a row's place.
- **Tree Select (`LjwTK`)** would take the whole tree: a select's trigger (the path chosen, `crates / lurq / lib.rs`) and a popover holding a `TreeView` with `selected`, closing on `on_select` of a leaf. Its panel on the board is the same rows in a popover of 8 padding. **Tree Table (`YiE9E`)** would take a row's indent, chevron and keys (`TreeRow::indent`, `chevron`, the nested focus group with `focus_level` and `focus_expanded`) in its first column, with the other columns of a table row; its rows are 30 high, its chevron 12, and it draws no icons. **Column View (`aCNzy`) and Path Control (`COHly`)**, beside them on the board, take nothing of it but the items.
- **The gallery's page** is in Navigation, with a playground (one or many selected, icons, guides, lazy children, a disabled row), the board's file tree, many selected, children that come a second after they are asked for, guide lines, and the states with a badge at a row's end.

## Not done

- **The engine's virtual list for a tree of thousands of rows.** What I found: `components::VirtualizedList` needs a `Ctx` and mounts each row as a component (`mount_keyed`), and builds only the rows near its viewport; a row out of that window is not in the tree, so it is not a stop of the focus group, and End, Home, PageDown, typeahead, `*`, Shift+End and Ctrl+A cannot reach it. A tree of thousands of shown rows works as built, one flat `Column`, and what a closed row holds is never built, but each render builds every shown row. The smallest change that would give it: a virtualized focus group, the list telling the engine how many rows it has and a key of each (`VirtualizedList::focus_rows(n, key)`), the engine's group keys stepping over that count rather than over built stops, and the list revealing the row it steps to (`reveal_key`, which it has) and focusing the first stop in it after the next layout. Then `TreeView::mount(ctx)` puts its flat rows in the list as they are.
- **An action at a row's end shown only under the pointer** (VS Code's "new file" on a folder row): the board does not draw one, and it needs the engine to style a part by the hover of the element it is in, which it does not (a part is styled by its own node's state). The smallest change: a state style of "hovered within" (`Node::state_style` taking an ancestor's hover, `Recipe::hovered_within(part, ..)`), which a sidebar's item actions would use too. Until then an app puts the action in `trailing`, always shown.

## Differs from the board

- The board's tree is 220 wide; the component has no width of its own and its rows are as wide as the tree is given (`style(TreeViewPart::Root, |s| s.width(220.0))` for the board's).
- Under the pointer, held down, focused from the keyboard, disabled, a guide, what is at a row's end and the spinner of a lazy row are not drawn on the board: decided above.
