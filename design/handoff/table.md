# Table

`ui::Table` and `ui::TableColumn`, from the board's Table (`UNRzJ`), Table/Empty (`Th2wb`) and Table/Loading (`z2NoZz`)
in Components / light (`Ovt1D`), with the Data Grid (`Kv08r`) read for what a table whose rows are selected has (the
checkbox column, a sorted header, a number at its end, the footer with a count and a pager), Skeleton/Table (`S1pEV`)
and the density foundations (`rxG03`, `f4pIgE`, `oPJUO`, `Fge9t`). The Examples (`ZB05g`) draw no table. Branch
`claude/ds-table-f99e09`. This session owned columns that share widths across rows, a header that stays while rows
scroll, and rows in view only.

## Changed outside the component's own files

- **Engine: the tracks of a table's columns** (`crates/lurq/src/layout/column_tracks.rs`, new; `pub mod column_tracks`
  in `layout/mod.rs`). `ColumnTracks` is one list of tracks shared by the header and every row of a table: each
  `TrackSpec` is a width in pixels, a share of what the others leave, or its content's (`TrackWidth::{Px, Share,
  Content}`), with a least and a most. A row given the tracks lays its children out one to a track, each as wide as
  its track; what holds the rows measures the cells of the rows in it before they are laid out, and a track keeps the
  widest cell it has had, so a column does not narrow as rows scroll by. Only the rows that are built are measured:
  no grid of every cell. The list is a handle compared by identity: a row whose list is another is laid out again.
  - Two builder methods every component has (`components/mod.rs`, after `sticky`): `.column_tracks(&tracks)` (a row
    of a table) and `.measures_column_tracks(&tracks)` (what holds its rows: its children and theirs, three deep).
  - `node/node.rs`: three fields of `RareStyle` (`column_tracks`, `measured_tracks`, `fills_viewport_width`), the three
    `NodeUpdate` methods that set them, `Node::column_tracks()`, `measured_column_tracks()`, `fills_viewport_width()`,
    and the three compared in `own_layout_signature_matches`, so that a rebuilt row with another list or a scroll
    that starts filling its width is laid out again.
  - `layout/layout_engine/track_layout.rs` (new): `LayoutEngine::measure_column_tracks` (measures the cells of the
    tracks sized by content, records them, and drops the layouts of the rows and what is between them and the
    measuring node where a track widened) and `layout_track_row` (cells at the tracks' widths, the row as tall as its
    tallest, `align` in the row; overlays' declarations and absolute children as a grid has them).
    `layout/layout_engine.rs`: `mod track_layout;`; at the top of `layout_node_content` the measuring and the dispatch
    of a `Row` with tracks to `layout_track_row`; in `layout_node_from_cache`, a node that measures tracks is not
    repaired in place, as a grid is not, so that a row that came in or changed is measured.
  - Its tests, written and not run: in `column_tracks.rs` (how widths are shared, held, kept) and `track_layout.rs`
    (rows line up whatever is in their cells, a track keeps its width, rows in a box of their own are measured).
- **Engine: `ScrollBoth::fill_width(bool)`** (`components/scroll.rs`; `layout_engine.rs`, `layout_scroll` and a new
  `fills_viewport_width(node, direction)`): content that scrolls sideways is laid out at least as wide as the viewport,
  so a table's columns share the room there is and it scrolls sideways only where they need more. Off unless set.
- **Engine: Tab into what a row holds** (`app/runtime/tab_navigation.rs`, `collect_node` and `Stop`/`Walk`): a stop
  records the stop it is inside (a button with `stops_inside(true)`, a row of a table). A focus group that is one stop
  keeps, besides its one stop, the stops inside that one: Tab goes from a focused row into its buttons and on out of
  the group, and Shift+Tab from a button back to its row. Where the focus is on such a button, the group's kept stop is
  its row. A group whose stops have nothing inside them is as it was. The Tree View session's `walk_group` hunk in the
  same file is apart from this one.
- **Engine: a revealed element is clear of what sticks** (`app/runtime/scroll_into_view.rs`, `clear_of_sticky`): the
  viewport an element is scrolled into leaves out the room of the sticky elements at its edges (their inset and
  size), unless the element is in one, as the web's `scroll-padding` would: the row the arrow keys move to is not left
  behind a table's header, nor under its sticky first column. It walks only subtrees whose layout holds a sticky node.
- **Engine: a style by the hover or the press of an interaction group** (`node/interaction_state.rs`,
  `node/style.rs`, `node/node.rs`, `components/mod.rs`, `ui/recipe.rs`, `app/runtime/interaction_groups.rs`,
  `layout/layout_engine.rs`): `.within_hovered_style(..)` and `.within_pressed_style(..)` beside the Tree View session's
  `.within_style(..)`, laid over it while the group is under the pointer and while it is held down, and not while it
  only holds the focus. A lit node's state records the two (`InteractionState::is_within_hovered`,
  `is_within_pressed`); `refresh_interaction_groups` lights each group once with whether it is hovered and active. In a
  recipe, `Selector::new().within().hovered()` / `.pressed()`, `Recipe::within_hovered`, `within_pressed`, and two
  fields of `PartStyles`. Plain `within` is as it was. A stuck cell of a table needs them: lit by the focus, a row
  clicked would keep its first column veiled after the pointer left. Its test, written and not run:
  `tests/input/interaction_group.rs`.
- `TableSort` is `DevtoolsInspectable` (empty impl) under the `devtools` feature, so `Signal<Option<TableSort>>` is a
  signal there.
- The board's **Table Toolbar** (`nX63b`) was built by the toolbar session as a `ui::Toolbar` of the app's controls
  (its handoff, `design/handoff/toolbar.md`); the Table page puts that bar over a table, its density bound to the
  table's. No component of its own was added for it.
- No keyframes were registered (none is used).

## For `docs/src/content/docs/design-system.md`

## Table

Rows of the same things in columns: crates and their status, files and their sizes, people and their roles. The columns of the header and of every row are the same widths whatever is in a cell.

```rust
use lurq::ui::{Badge, Table, TableColumn, TableSort};

// sort: Signal<Option<TableSort>>, selected: Signal<Vec<String>>
let columns = vec![
  TableColumn::new("name", "Name").text(|row: &Crate| row.name.clone()).mono(true).sortable(true),
  TableColumn::new("status", "Status").width(120.0).cell(|row: &Crate| Badge::new(&row.status).into()),
  TableColumn::new("size", "Size").width(70.0).numeric().text(|row: &Crate| format!("{} KB", row.size)).sort_key(|row: &Crate| row.size),
  TableColumn::actions("actions", |row: &Crate| open_button(row)),
];
// The app gives the rows in the order it wants.
let mut rows = rows;
if let Some(by) = sort.get() {
  by.sort(&columns, &mut rows);
}
Table::new(columns, rows)
  .row_key(|row: &Crate| row.name.clone())
  .label("Crates")
  .sort(sort)
  .selected(selected)
  .height(320.0)
  .on_row_activate(|name| open(name));

// Thousands of rows: mounted, it builds those in view.
Table::new_shared(columns, rows.clone()).row_key(|row: &Crate| row.id).height(480.0).mount(ctx);
```

| Builder | |
| --- | --- |
| `new(columns, rows)`, `new_shared(columns, Arc<Vec<R>>)` | The columns and the rows, in the order the app wants them. Shared rows given again at each render are not copied |
| `TableColumn::new(id, title)` | A column that takes a share of the room the others leave |
| `TableColumn::text(\|row\| String)`, `cell(\|row\| Element)` | A row's cell: a text in the cell's type, or what the app builds (a badge, an avatar and a name) |
| `TableColumn::width(px)`, `share(f32)`, `fit()`, `min_width(px)`, `max_width(px)` | Its width: in pixels, a share of what is left, or as wide as the widest of its cells shown, between a least and a most |
| `TableColumn::align(ColumnAlign)`, `numeric()`, `mono(bool)`, `muted(bool)` | Where its content sits; a column of numbers (at its end, mono 12 in the muted color); its text in the mono type or the muted color |
| `TableColumn::sortable(bool)`, `sort_by(\|a, b\|)`, `sort_key(\|row\|)` | A press of its header sorts the rows by it; what `TableSort::sort` compares rows by |
| `TableColumn::sticky(bool)` | The first column stays at the start while the rows scroll sideways |
| `TableColumn::controls(bool)`, `TableColumn::actions(id, cell)` | Its cells hold what is pressed, and a press of it is not the row's; a column of a row's buttons at its end |
| `row_key(\|row\| K)` | What tells a row apart, which the selection and the handlers name it by. Without it, a row's place |
| `sort(Signal<Option<TableSort>>)`, `sorted_by(sort, change)`, `on_sort(handler)` | What the rows are sorted by: a press of a sortable header goes through ascending, descending and none |
| `selected(Signal<Vec<K>>)`, `selected_keys(keys, change)` | The rows selected, with a column of checkboxes |
| `checkboxes(bool)` | Whether a table whose rows are selected has the column of checkboxes; without it a press of a row selects it |
| `on_row_click(handler)`, `on_row_activate(handler)` | Told of a press of a row, and of a double press or Enter |
| `density(Density)` | Rows of 36 compact, 44 at the default, 52 comfortable and touch |
| `height(..)`, `max_height(..)` | The table's height: the header stays and the rows scroll under it |
| `empty(element)` | What stands under the header where there are no rows: by default an empty state that says so |
| `loading(bool)`, `loading_rows(n)` | Rows of skeletons under the header while the rows are on their way |
| `pagination(element)`, `count(bool)`, `total(n)` | A footer that counts the rows and what is selected, with the pagination at its end |
| `mount(ctx)` | The table mounted: it builds the rows in view and a few around them, and keeps its columns' measured widths |
| `label(&str)`, `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. A row's id is the table's with `-row-` and its place; a header's `-header-` and its column's id; the header's checkbox `-select-all` |

- What the rows are, their order, what is selected and what they are sorted by are the app's. A press of a sortable header asks for the rows sorted by it, ascending, then descending, then not at all, and the app gives them in that order; `TableSort::sort(&columns, &mut rows)` sorts them by the column's key or its text.
- With a selection a table has a column of checkboxes. A press of a row selects it alone, with Ctrl (Cmd) adds it or takes it out, with Shift selects the rows from the last selected one to it; a press of its checkbox adds or takes it out, with Shift the range. The header's checkbox selects every row the table shows, or none where all are, and is mixed while some are. A press in a column of `controls` is the control's alone.
- The header stays while the rows scroll in the table's own height, and a table wider than its box scrolls sideways, its header with it; a sticky first column stays at the start. Converted into an element a table builds every row; mounted (`mount(ctx)`) it builds the rows in view and a few around them, so ten thousand rows cost about what a screen of them does. A table with no height builds every row either way.
- The columns are one list of tracks for the header and every row (`ColumnTracks`), measured from the rows that are built: a column sized by its content is as wide as the widest of its cells shown so far, and does not narrow as rows scroll by.
- The rows are one stop of Tab, at the focused, the first selected or the first row. Up and Down move through them, Home and End to the first and the last, and in a mounted table PageUp and PageDown by what is shown. Space selects the focused row (adds it or takes it out), Enter activates it, Ctrl+A selects them all. Tab from a row goes into what it holds that takes the focus, its buttons, and then out of the table. The sortable headers and the header's checkbox are a stop of their own before the rows, Left and Right between them. A row the keys reach is scrolled into view clear of the header.
- To what reads the tree it is a `table`, or a `grid` (`a11y_multiselectable`) where its rows are selected, with `a11y_rowcount` (the header's among them) and `a11y_colcount`, `a11y_busy` while loading; its header and rows are `row`s with `a11y_rowindex` (the header's 1), `a11y_selected` where rows are selected; a header is a `columnheader` with `a11y_sort` (`ascending`, `descending`, `none`) where it sorts; a cell is a `cell`.
- Parts: `TablePart::Root` (the box), `Scroll` (what scrolls both ways), `Header`, `HeaderCell`, `SortIcon`, `Row`, `Cell`, `Select` (the checkbox column's cell), `Empty`, `Footer`, `Count`. States: `TableState::Selectable`, `Selected`, `Last`, `Sortable`, `Sorted`, `Muted`, `Mono`, `Numeric`, `Sticky`, `Footed`, `Disabled`.
- In the design the table is a box with `radius-lg`, an inner edge of 1 in `Border` and the `Background` fill. The header is 40 high on `Muted` with a line of 1 in `Border` under it, its titles 13/500 in `MutedForeground`; a row is 44 high with the same line under it but the last, its text 13 in `Foreground`. Both are padded 16 at each side with 16 between columns, 12 where the rows have checkboxes, whose column is 16 wide. A sorted column's title is in `Foreground` with an arrow of 14, 4 after it; a sortable one has a muted mark of `chevrons-vertical`, and its title is in `Foreground` under the pointer. A selected row is on `Highlight`. A number is mono 12 in `MutedForeground` at its column's end. The footer is 44 high, padded 16 and 8, its count 13 in `MutedForeground`. Under the pointer and held down a row has the `StateHover` and `StatePressed` veils, the focus from the keyboard is a ring of 2 in `Ring` inside it, and a disabled table is half opaque.
- A theme's renderer is given `TableView`: `view.headers` (each a `TableHeader` with `cell()`, `sort_icon()`, `render()`), `view.rows` (each a `TableRow` with its place, whether it is selected, its `cells` and `button()`, `checkbox()`, `render()`), `view.root()`, `view.header()`, `view.select_all()`, `view.body()` (the rows and the room of those not built), `view.skeletons()`, `view.empty()`, `view.footer()` and `view.tracks`, the list every row of the table lays its cells out in. It is set for `Table<R, K>` of any `R` and `K`.

## For `CHANGELOG.md`

- `ui::Table` and `ui::TableColumn`: rows in columns that line up whatever is in a cell, from the engine's new shared column tracks (`ColumnTracks`, `.column_tracks`, `.measures_column_tracks`); sorting the app holds, selection with checkboxes, Ctrl and Shift, a header that stays while the rows scroll and a sticky first column (`ScrollBoth::fill_width`, and elements revealed clear of what sticks), rows built in view only in a mounted table of thousands, densities, empty and loading forms, a footer with a count and a pagination, the keys of a table of records, and Tab from a focused row into its buttons in a one-stop focus group.

## Decided

- **Rows, not a grid of cells.** The board's table is a table of records, each row one thing: a row is what is pressed, selected and focused, Up and Down move between rows, Tab goes into a row's buttons, and there is no focus on a cell and no Left and Right between cells. The **Data Grid** (`Kv08r`) would add that: a focus on each cell (a `FocusGroup::both()` over the cells, the row and the column of the focused cell), editing a cell in place, resizing and reordering columns by their headers, and keeping the board's footer `Page 1 of 8 ‹ ›`. It would take everything else of this: `ColumnTracks`, the header that stays, the window of rows, the selection model and the checkbox column. The **Tree Table** (`YiE9E`) would take the rows and tracks of this and a tree view's row in its first column (`TreeRow::indent`, `chevron`, `FocusGroup::nested` with `focus_level` and `focus_expanded` on the rows), its rows 40 high with 12 between columns.
- **A builder that can be mounted.** `Table` converts into an element as every component does, building every row; `mount(ctx)` puts it in a host (`TableHost`) that keeps what a builder cannot: the scroll of its rows, which rows are built, the measured widths of its columns between renders, and the row that has the focus, kept built. Its body is the engine's `FocusRows` (the host's state): the group steps over the count of rows, PageUp and PageDown by what the view holds under the header less one, and a row not built is revealed (scrolled to under the header and built) and focused by the engine once a layout has it. A table that is not mounted builds every row and its body is a plain one-stop group: PageUp and PageDown come with `FocusGroup::pages(true)` from the list session, one line at that merge, in `TableView::body` (the `.focus_group(group)` of the branch with no window, which becomes `group.pages(true)`). Building only the rows in view needs a render on scroll, which needs a component; a table of a few rows needs none of it.
- **The engine's part is the tracks, not a grid of every cell.** `Grid` would have lined the cells up, but every cell would be an item of one grid, with no element for a row to press, hover and select, and every row measured to size each column. A row of a table is its own element and lays its cells out in tracks the rows share; what holds the rows measures the rows built. The gap between columns is in the cells (`padding_right`) and not the row's spacing, so that a cell that stays covers the gap after it as it slides over the others.
- **Rows of one height.** A row's height is the recipe's at the density, in pixels: a mounted table reckons where each row is from it, with the room of the rows not built as two spacers. A recipe that gives rows no height in pixels builds every row. A cell is cut at its column's edge, not wrapped.
- **The window of rows** is what is in view and 8 rows past each edge, built again when half of that margin is left (on a wheel's scroll, from where it is going, and after each layout, for a drag of the bar or a reveal) or when the rows built are far from what is in view. The focused row stays built wherever the rows scroll, so the focus is not lost. The first render, before the table knows its height, builds 40.
- **The sort is the app's.** `sort(Signal<Option<TableSort>>)` and `on_sort`; the table orders nothing, so a table of a page of a server's rows sorts on the server. `TableSort::sort` is the helper for rows the app holds.
- **The selection is `Vec<K>`**, the last selected the anchor of a range, as the tree view's; `K: Eq + Hash`, so that a selection of thousands is looked up once a row. Keys default to a row's place (`usize`), which a sort changes: `row_key` is how a selection follows its rows.
- **The checkbox of a row takes neither the focus nor the press** (it is `inert` in a cell that is pressed): Space on the row is its press, and Tab does not stop at a checkbox in every row. The header's checkbox is a stop, with the sortable headers.
- **The board's Table draws a row in `$accent` with no checkboxes**: read as a row selected by a press (`selected(..).checkboxes(false)`), as the Data Grid draws its selected row in `$accent`. Under the pointer a row is veiled, as every list of the system is.
- **A stuck first column takes the row's veil.** A cell that stays covers what scrolls under it in the row's fill (the background, or the highlight of a selected row), and so the row's veil under it. The row is an interaction group, and a stuck cell, the checkbox's and the first column's, has the veil of `StateHover` and `StatePressed` by the row's hover and press (`TableState::Sticky` with `within().hovered()` and `within().pressed()`), not by its focus. The stuck checkbox cells have the row's fill too, and the header's the header's, where before what scrolled showed under them.
- **States the board does not draw**: the mark of a sortable column at rest (`chevrons-vertical`, muted) so a person sees what sorts, its title in `Foreground` under the pointer; the veils, the ring inside a row, a faded disabled table.
- **Densities**: the board draws its table at 44 and the bar's three choices; compact is the theme's default row (36), comfortable and touch 52, the header 4 lower than a row at each.
- **The empty state by default** is `EmptyState::new("No rows").icon(icons::INBOX).size(EmptyStateSize::Sm)`, the board's small one; the board's own (`No crates match “blurr”`, a search-close icon and "Clear filter") is the app's, given with `empty(..)`, as the page shows.
- **The footer** is a count at the start ("24 rows", "1 of 24 rows selected") and the app's `Pagination` at the end; the board's `Page 1 of 8 ‹ ›` is the compact pagination's own text, "1–3 of 24".
- **The scroll is the box's corners**: the scroll clips what is in it to `radius-lg` (top corners only with a footer), since a clip of its own inside the rounded box would cut the header's fill square over the box's corners.
- **The gallery's page** is in Data: a playground (sorting, selection with checkboxes or by a press, density, the height in which the header stays, a sticky first column, the footer, empty and loading), the board's table, rows selected with a footer and a pagination, sorting, a header that stays in 280 with a sticky column, a mounted table of ten thousand rows, the empty and the loading forms, the board's Table Toolbar over a table, and the states.

## Not done

Nothing.

## Differs from the board

- The board's table is 640 wide (Table/Empty and Table/Loading 420, their Status and Updated columns 90); the component is as wide as what holds it, and its columns' widths are the app's.
- The skeleton rows have a line of 70 in every column after the first, where Table/Loading draws a round pill of 64 by 20 in its Status column: a column does not say what kind of content it holds.
- The mark of a sortable column at rest, the title under the pointer, densities other than the default, the veils, the ring and a disabled table are not drawn on the board: decided above.
- The Data Grid draws the header's checkbox clear while one row is selected; here it is mixed while some rows are, as a checkbox of a group is.
- The Data Grid's footer pager says "Page 1 of 8"; the compact `Pagination` says "1–3 of 24".
