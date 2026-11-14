# Splitter

`ui::Splitter` and `ui::SplitterPane`, from the board's Splitter Handle (`ZTLMY`) and Resizable (`SfT7O`) in `Ovt1D`
(Components / light; the states Idle, Hover and Dragging in the frame `z56hN`, Desktop Shell; the Resizable in
`neZOA`, Surfaces / Layout; the panes of the Examples' windows in `ZB05g`, the dock layout `GKlOr`). Branch
`claude/ds-splitter-920456`. This session owned dragging a divider between panes with the pointer captured.

The board's Resizable is not a component of its own: it is a splitter whose dividers have a grip, in a box (a side bar
beside an editor over a terminal). It is `Splitter::grip(true)` with `framed(true)`, and there is no `ui::Resizable`.
The board draws no handle on the edge or the corner of a single box.

## Changed outside the component's own files

- **Engine: the cursor of a drag.** `Tree::cursor` (`crates/lurq/src/app/runtime.rs`) returns, while an `on_drag_*`
  session holds the pointer, the cursor that was under the pointer where the drag began, until it is let go: over
  other elements, and after the pointer left the window (the shell calls `mouse_leave_window` and goes on passing the
  moves of a held button, which the drag already received). Before, the cursor stayed only while the pointer stayed
  in the window: `mouse_leave_window` set it to the default, and the drag never set it back. `ActiveDrag` has a field
  `cursor: CursorIcon`, set where the drag starts (in `dispatch_mouse`, where `pending_drag` becomes `active_drag`,
  from the cursor the press computed) and `CursorIcon::Default` where the struct is built. This holds for every
  `on_drag_*` session (`Draggable` too), as a pointer capture does in a browser.
- **Engine: a size an element ref gives its element, with no render.** `ElementRefMut::set_size(width, height)` and
  `ElementRefMut::size()` (`crates/lurq/src/core/element_ref.rs`): each an `Option<Dimension>` (pixels or a share of
  the parent) that the element is laid out at in place of the width or the height it was built with, until it is set
  again; `None` gives back what it was built with. The least and the most size it was built with still hold. It marks
  the ref's layout dirty, as `set_bounds` does, so the next pass lays the element and what holds it out again and
  `Tree::needs_redraw` asks for that pass; nothing is rendered again. `ElementRefInner` has a field `size_override`,
  `ElementRef::size_override()` (crate) reads it.
  - `crates/lurq/src/node/node.rs`: `Node::effective_frame` lays the ref's size over the frame (after the state
    styles, before a running animation's width or height), `Node::size_override()` and `Node::fixed_frame_size()`
    (crate) read it.
  - `crates/lurq/src/layout/layout_engine.rs`: the two checks of a cached size against a fixed frame
    (`mark_layout_dirty` and `cached_result_matches_node_tree`) read `fixed_frame_size()` in place of `node.frame`,
    so that an element whose ref resized it is not taken for stale and laid out again at every pass.
- Their tests, written and not run: `crates/lurq/tests/dnd/held_drag.rs` (registered in `tests/dnd/mod.rs`) and
  `crates/lurq/tests/layout/ref_size.rs` (registered in `tests/layout/mod.rs`).
- `docs/src/content/docs/styling-events.md`, Drag And Drop, wants a sentence the merge adds (this session does not
  edit `docs/`), after "Low-level node drag handlers are also available": "A drag holds the pointer from the press to
  the release: its moves reach `on_drag_move` wherever the pointer goes, in the window and out of it, nothing is
  hovered and no text is selected meanwhile, and the window shows the cursor the drag began with (`Tree::cursor`)."
- `docs/src/content/docs/layout.md` (or `retained_nodes.md`, Element Lookup) wants a paragraph: "`ElementRefMut::
  set_size(width, height)` lays the element out at another width or height (`Dimension::Px` or `Dimension::Pct`) than
  it was built with, until it is set again or set to `None`; its least and most size still hold. Nothing renders
  again: the next pass lays it and its parent out. A splitter resizes its panes so while a divider moves."
- **Engine: inert elements** (built after the batch's merge, to close what this handoff had under Not done; it
  replaces a rule of the first fix that counted a zero-size element that clips as dropped, which is gone). An element
  that is inert, and everything in it, is out of the focus and the pointer while it is still painted.
  - `NodeUpdate::inert` and `Node::is_inert()` (`crates/lurq/src/node/node.rs`): a field `inert` in `RareStyle`; a
    node is inert when built so or when its ref says so. The builder every component has, `.inert(bool)`
    (`crates/lurq/src/components/mod.rs`, after `focusable`).
  - `ElementRefMut::set_inert(bool)` and `ElementRef::is_inert()` (crate) (`crates/lurq/src/core/element_ref.rs`): a
    field `inert` in `ElementRefInner`; setting it marks the ref's layout dirty, so the pass it asks for lets the
    focus leave. An element built inert stays inert whatever its ref says.
  - `crates/lurq/src/app/runtime/tab_navigation.rs`: `collect_node` and `walk_group`'s `collect` pass an inert node by
    as they pass a disabled one (one condition each: Tab, a focus group's arrows, typeahead); a new
    `path_is_inert(root, path)`. Nothing else there changed (the TreeView session works beside it).
  - `crates/lurq/src/app/hit_test.rs`: `hit_test_tree_with_transform` returns at an inert node as at one that takes
    no hits, so what is under it gets the pointer.
  - `crates/lurq/src/app/runtime.rs`: `focus_node` refuses a target inside an inert element;
    `blur_focus_in_dropped_child` lets the focus inside an inert element go where that of an element taken out of the
    tree goes (the stop of its focus group that takes its place, the next or the last, or nothing; a group inside the
    inert element does not count); `reapply_interaction_state` calls it when a render made the focus's element inert.
  - Docs: `docs/src/content/docs/focus-navigation.md`, a section "Inert Elements" under Tab Order (edited here, as
    asked).
  - Tests, run: `crates/lurq/tests/input/tab_order/inert.rs` (registered in `tests/input/tab_order/mod.rs`).
- **Engine: a description an element ref gives, with no render.** `ElementRefMut::describe(name, value)`
  (`crates/lurq/src/core/element_ref.rs`): says `value` under `name` to tooling in place of what the element's
  `describe(name, ..)` said when it was built. `ElementRefInner` has a field `descriptions`, `ElementRef::descriptions()`
  (crate) reads it. `Node::debug_attrs()` (`crates/lurq/src/node/node.rs`) now returns a
  `Cow<[(Arc<str>, Arc<str>)]>`: borrowed as before where the ref describes nothing, otherwise the node's attributes
  with the ref's laid over them by name; `Element::debug_attrs()` (`node/element.rs`) returns the same. Every caller
  (DevTools' snapshot, the MCP server's `lurq_inspect`, `lurq_read_tree` and `lurq_find` through `inspection_attrs`
  and `semantics`, `ui::Field`) reads it with `.iter()` and compiles unchanged, and reads the new value at once.
  `ElementHandle::description(name)` (`app/runtime.rs`, with `mcp` or `devtools`) is the tree's lookup of it. Docs:
  a paragraph in `docs/src/content/docs/mcp.md`, Making Your App Agent-Friendly (edited here). Tests, run with `mcp`:
  `element_ref::tests` at the end of `element_ref.rs`, `a_ref_describes_its_element_without_a_render` in
  `tests/layout/ref_size.rs`, `a_divider_tells_tooling_where_it_is_as_it_moves` in `tests/ui/splitter.rs`. The lib
  does not build with `devtools` now (`ui/combobox.rs` 232 and 308: `Signal<T>` with no `DevtoolsInspectable` bound),
  not of this session.
- `crates/gallery/src/pages/mod.rs`: the page is the first of `Group::Desktop`, at the end of `PAGES`. If the
  Sidebar, Menubar or Toolbar session put pages in that group too, the merge orders them as the board does.
- No keyframes were registered (none is used).

## For `docs/src/content/docs/design-system.md`

## Splitter

Panes that share a room by the dividers between them, which a person drags: the side bar, the editor and the terminal of a window.

```rust
use lurq::ui::{Splitter, SplitterPane};

// layout: Signal<Vec<f32>>, saved with the window.
Splitter::horizontal()
  .sizes(layout.clone())
  .pane(SplitterPane::new(files).size(240.0).min(160.0).max(480.0).collapsible(true).label("Files"))
  .pane(SplitterPane::new(
    Splitter::vertical()
      .pane(SplitterPane::new(editor).min(120.0))
      .pane(SplitterPane::new(terminal).size(160.0).min(80.0).collapsible(true))
      .mount(ctx),
  ))
  .pane(SplitterPane::new(outline).share(0.2).min(100.0))
  .on_resize_end(|sizes| save(sizes))
  .mount(ctx);

// The board's Resizable: a grip on each divider, in a box of its own.
Splitter::horizontal().grip(true).framed(true).pane(sidebar).pane(content);
```

| Builder | |
| --- | --- |
| `Splitter::horizontal()`, `vertical()`, `new(orientation)` | Panes side by side, or one over the other |
| `pane(SplitterPane)`, `panes(..)` | A pane after the others, with a divider before it |
| `SplitterPane::new(content)` | A pane holding `content`, laid out in its whole width from its top: a height of `Dimension::Pct(100.0)` fills it |
| `SplitterPane::size(px)`, `share(0..1)` | How large it is to begin with: pixels it keeps as the splitter grows and shrinks, or a share it keeps. A pane with neither takes the rest; where all have one, the last does |
| `SplitterPane::min(px)`, `max(px)` | The least and the most it is |
| `SplitterPane::collapsible(bool)` | It closes when a divider is dragged under half its least size, and opens by a drag back, a double press on the divider or Enter. The pane that takes the rest does not close |
| `SplitterPane::label(&str)`, `key(..)` | What it is called: the divider after it is called so to tooling |
| `sizes(Signal<Vec<f32>>)` | The sizes of the panes, the app's: pixels along the splitter as a divider last left them, 0 for a closed pane, written as a divider moves and once it is first laid out |
| `on_resize(handler)` | Called with the sizes each time a divider moves: while it is dragged, and for each key |
| `on_resize_end(handler)` | Called with the sizes once a divider has moved: let go after a drag, or moved by a key, a double press or Enter |
| `step(px)`, `large_step(px)` | What an arrow key moves a divider by, and with Shift: 10 and 50 |
| `grip(bool)` | A grip in the middle of each divider |
| `framed(bool)` | A box of its own: a fill, an edge and round corners, as the board draws its specimens |
| `label(&str)` | What a divider is called where the pane before it has no label |
| `mount(ctx)` | Mounted where it is declared: it keeps its sizes where the app gives it no signal, and shows the sizes the app sets in the signal from outside |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `force` draws the dividers; `id` is the root's, and the dividers are `{id}-divider-{n}` |

- A drag holds the pointer: the divider follows it wherever it goes, in the window and out of it, the cursor is the resizing one over the divider and for the whole drag, and no text is selected by it. A double press puts the divider back where it began, or opens the closed pane beside it. A closed pane is inert (`.inert`, see Focus And Keyboard Navigation): what is in it stays mounted and takes neither the focus nor the pointer.
- A divider is a stop of Tab. The arrow keys of its orientation (Left and Right across, Up and Down one over the other) move it by a step, with Shift by a larger one; Home and End take the pane before it to its least and most size, and Enter closes and opens the collapsible pane beside it.
- The panes say what gives way when the window grows or shrinks: one of pixels keeps them, one of a share keeps its share, and the pane given no size takes the rest. No open pane is made smaller than its least size while there is room: where the rest would be, the panes nearest it give way down to theirs.
- Converted into an element, a splitter keeps its sizes while what holds it does not render again, unless it is given `sizes`; mounted, it keeps them, and an app that sets `sizes` from outside (to restore a layout) sees the panes take them. Pixels set from a saved layout are the pane's pixels, and a pane of a share takes the share its pixels were of the room they were saved in.
- A drag renders nothing again while it moves: the panes are resized by sizes their element refs are given (`ElementRefMut::set_size`), and what is in them is only laid out again at the next pass. Nor does letting it go: tooling reads where it stands from the divider at once (`ElementRefMut::describe`).
- Parts: `SplitterPart::Root`, `Pane`, `Handle` (the room around a divider's line that takes the pointer and the focus), `Line`, `Bar` (the wider line under the pointer, dragged and focused), `Grip`, `GripIcon`. States: `SplitterState::Vertical`, `Framed`, `Disabled`.
- In the design a divider is a line of 1 in the `Border` color; under the pointer and focused from the keyboard it is 3 wide in `Ring`, and dragged in `Primary`. The room that takes the pointer is 12 across the line, over the edges of the panes beside it. The grip is 12 by 20 (20 by 12 across a vertical splitter), round by the small radius, in `Border`, with a grip icon of 10 in `Foreground`. Framed, the splitter has the `Background` fill, an edge of 1 in `Border` and the large radius.
- To what reads the tree a divider is a `separator` that can be focused, with `a11y_orientation` (that of its line), `a11y_valuenow` (the pixels of the pane before it), `a11y_valuemin`, `a11y_valuemax` (where it has a most) and the name of the pane before it, or the splitter's `label`. `a11y_valuenow` follows the divider as it is dragged and moved by the keys.
- Splitters nest, one orientation in the other: a pane holds a splitter of its own.
- A renderer that draws another splitter is given the view: `view.part(..)`, `view.thickness()` (the room between two panes), `view.sizes()`, and `view.render()` the default.

## For `CHANGELOG.md`

- `ui::Splitter` and `ui::SplitterPane`: panes that share a room by dividers a person drags, holding the pointer with its cursor for the whole drag, with pixel and share panes, least and most sizes, panes that close, the keys, a double press back, the app's saved sizes and the board's Resizable grip; the engine's `on_drag_*` sessions keep their cursor until let go, and `ElementRefMut::set_size` resizes an element without a render.

## Decided

- **Resizable is the splitter with a grip.** The board's `SfT7O` is a box of panes with a grip on the divider, not a
  handle on one box, so it is `grip(true)` and `framed(true)` of the same component. A `ui::Resizable` for a handle on
  an edge or a corner of a box was not built: the board draws none.
- **A drag resizes the panes through their refs, not by a render.** The engine's flex has no least size for a growing
  pane and no way to move a pane but to build it again, so the splitter gives each pane a ref and sets its size with
  `ElementRefMut::set_size` while a divider moves. What renders the splitter, and what is in its panes, is not built
  again; only the layout runs. The sizes are also written to the signal as they move (read without subscribing by
  the splitter), so that a render later builds the panes where they are.
- **A pane of pixels keeps them; one of a share keeps its share; one takes the rest.** A share is laid out as a percent
  of the splitter (`Dimension::Pct`), so it follows the window in the layout itself, with no frame late. The pane that
  takes the rest grows by flex. The least sizes the window would break are put back once the splitter is laid out in
  the new room (its row's rect observer), one frame later, and only when a pane is under its least.
- **The room that takes the pointer is a copy of the row.** The splitter is a stack of two rows: the panes with the
  room of the lines between them, and over it a copy whose panes take no pointer and whose dividers' handles do, 12
  across, standing out of the line's room over the edges of the panes on both sides. A stack's top child that is hit
  hides what is under it, so a press on a handle reaches no pane, and a press beside it reaches the pane alone. Each
  pane and its copy take the same sizes.
- **12 across.** The board draws no room for the pointer; 12 is the grip's width, so the grip and the room are one,
  and it centers the line of 1 exactly (5.5 on each side).
- **The bar is a second line in the handle's foreground.** A state style changes no size, so the line of 1 stays and a
  bar of 3 over it takes its color from the handle's foreground (`BackgroundColor::CurrentForeground`): transparent at
  rest, `Ring` under the pointer and focused, `Primary` while pressed, which lasts for the whole drag.
- **A closed pane is 0 along the splitter** and keeps its content mounted. It opens to its least size when a drag
  takes it past half of it, and to the size it began with by a double press or Enter.
- **Not framed by default.** The board's specimens are boxes with an edge, but in the Examples' windows the panes are
  divided by lines with no box; `framed(true)` draws the specimen.
- **Mounted, or converted.** A builder has no `Ctx`: converted into an element, the splitter's own sizes live as long
  as what holds it does not render again; `mount(ctx)` keeps them in a host, which also renders the splitter again
  when the app writes the signal (a write that is not the splitter's own). A divider let go renders nothing: it tells
  tooling where it is through its handle's ref (`ElementRefMut::describe`).
- **The line has a room of its own between the panes**, as on the board (Pane A, the Handle of 1, Pane B, with no gap):
  the line does not lie on the first pane's edge. The handle that takes the pointer is a stack centered on that room,
  its line, bar and grip centered in it (`StackAlignment::Center`: a stack lays its children at its top start
  otherwise, which put them 5.5 to the left and the grip at the top until the merge's tests found it).
- **A closed pane is inert**, through its ref (`ElementRefMut::set_inert`), so that a drag closes and opens it with no
  render, in a splitter converted into an element too: what it holds stays mounted, takes neither the focus nor the
  pointer, and the focus in it leaves when it closes.
- Keys: a step of 10 and with Shift 50, as other desktop splitters move.

## Not done

Nothing. The two items this handoff had here (what is in a closed pane taking the focus, and `a11y_valuenow` of a
splitter not rendered again after a drag) are built by the two engine changes added after the merge, above.

## Differs from the board

- The board draws no focused divider: focused from the keyboard it is drawn as under the pointer, the bar of 3 in
  `Ring`.
- Under the pointer and dragged the board widens the line to 3 by its frame, which moves the panes; here the bar of 3
  is drawn over the line of 1, centered on it, and the panes do not move.
- The board's grip is 0.5 to the left of the line's middle (at 134 for a line at 140); here it is centered on the line.
- The board draws the grip only on a divider that stands; across a vertical splitter it is turned a quarter, 20 by
  12, with the same icon turned.
- The board's specimen is a box with an edge; by default the splitter draws none, as the Examples' windows do
  (`framed(true)` draws it).
