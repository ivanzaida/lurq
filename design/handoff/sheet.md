# Sheet and Drawer: hand-off

`ui::Sheet` and `ui::Drawer`, from the board's `Sheet` (`wvm6l`, Components / light › Overlays › Large) and
`Drawer` (`UaJAF`, Components / light › Surfaces › Floating), with `Mobile/Bottom Sheet` (`Qawrh`),
`Tablet/Form Sheet` (`wt2Lv`) and `Mobile/Navigation Drawer` (`HSa8x`) read so that the parts can be restyled into
them. The board draws each once, at rest, with no veil around it; Foundations › Motion gives a dialog, a sheet and a
drawer the slow duration (240).

Files: `crates/lurq/src/ui/sheet.rs`, `crates/lurq/src/ui/drawer.rs`, `crates/lurq/tests/ui/sheet.rs`,
`crates/lurq/tests/ui/drawer.rs`, `crates/gallery/src/pages/sheet.rs` (page "Sheet", group Surfaces),
`crates/gallery/src/pages/drawer.rs` (page "Drawer", group Surfaces), `design/handoff/sheet.md`, and lines in the
three `mod.rs` files.

## Changed outside the component's own files

**What the engine's `Sheet` did before this batch** (`crates/lurq/src/components/sheet.rs`, docs
`docs/src/content/docs/modals.md`, Sheets). It is a mounted `Component` (`Sheet::mount(ctx, props, content)`,
`mount_with`, `mount_keyed`, `mount_keyed_with`), built for a phone:

- It comes up from under the window's bottom edge when its signal is set and goes down when it is cleared, by
  Escape, by a press on the backdrop or by a finger, through the phases closed, entering, open, moving and leaving.
  It sets off 1 ms after its first frame is laid out (under the window), every move is a spring of
  `SheetStyle::duration` (500 ms) that starts at the speed a finger let it go with, and it is taken to be at rest
  50 ms after. It stays in the tree until it has gone down. With motion reduced it fades where it stands.
- **Detents**: `Content` (as high as the content, the default), `Medium` (half the window), `Large` (the window less
  the status bar's room and `top_margin`, 10), `Fraction(share)` of `Large`, `Height(px)`; each clamped to `Large`.
  It comes up at the first; `SheetProps::detent(signal)` says which it is at and sends it to another.
- **Drag**: a one-finger vertical pan on the panel. Down is the sheet's unless the content's list
  (`SheetProps::scroll`, or the `SheetScroll` a `StackOutlet` in it provides) has something above to show; up is the
  sheet's until it is at its highest. Past its highest, and past its lowest when it is not dismissable, it gives way
  less and less (0.55). On release it goes to the detent nearest to where it was thrown (its position plus 0.15 s
  of the finger's speed), or down where that is nearer and it is dismissable.
- **Clear of the phone's edges and the keyboard**: it provides `SafeAreaTaken` (`ctx.safe_area()` is nothing in
  it), its content is padded by the safe area's bottom, left and right, `Medium` and `Fraction` rise by the
  keyboard's height, and `ctx.blur()` puts the keyboard down as it closes.
- **What it draws itself, in fixed colors**: a backdrop over the window (a row in `SheetStyle::backdrop`, black, at
  `dim` (0.3) times how much of its lowest height is in view, a press on which closes a dismissable sheet); its
  panel, `id("sheet")`, as wide as the window up to `max_width`, `room + 400` high so that a finger can pull it up
  without a gap under it, in `background` (`#FFFFFF`), round by `corner_radius` (20) and clipped; and a grabber
  laid over its top, 36 by 5 at 5 under the edge, in `rgba(60, 60, 67, 0.3)`. `SheetStyle::dark()` has `#1C1C1E`, a
  grabber of `rgba(235, 235, 245, 0.3)` and a dim of 0.5. No theme role reaches any of them.
- It is a `Modal` with `target(Root)`: Tab is trapped in it, Escape closes a dismissable one, and the focus did not
  move into it. `SheetHost` around the app draws the app back behind a sheet at its full height.
- It placed itself by the window's whole height, so under a `WindowChrome`'s title bar (where every modal over
  the whole window is now laid out) its bottom was cut off by the bar's height.

**What this session changed in the engine and in shared code.** Every existing use of the engine's sheet works as
before.

- `crates/lurq/src/components/sheet.rs`:
  - `SheetProps::dress(Fn(SheetNode, Column) -> Column)`, with `pub enum SheetNode { Backdrop, Panel, Grabber }` and
    `pub type SheetDress`: what dresses the nodes the sheet draws itself, after it has given them their place and the
    colors of its `SheetStyle`. `ui::Sheet` gives them the recipe's look (the panel's color, edge, corners and
    shadow; the veil's scrim). The backdrop and the grabber's bar are a `Column` now (a `Row` before; neither holds
    anything). Props with a dress compare by the `Arc`.
  - `SheetProps::dismiss_on_escape(bool)`: Escape apart from a press beside and a drag down; `dismissable` still
    says it when this is not given.
  - `SheetProps::takes_focus(bool)`: `Modal::takes_focus` for the sheet's layer. Off by default; `ui::Sheet` turns it
    on.
  - The sheet's room is the window's height less `WindowHandle::title_bar_inset()`, as its layer's is: under a
    title bar it stands at the window's bottom again.
  - `crates/lurq/src/components/mod.rs` exports `SheetDress` and `SheetNode`.
- `crates/lurq/src/node/node.rs`, `Node::offset_for_size`: an `OffsetX` or `OffsetY` that a keyframe or a transition
  runs with `AnimatableValue::Percent(p)` moves the node by `p` percent of its own width or height. Before, only a
  `Float` override was read and a percent one was ignored. Needed to slide a panel in from beyond the window's edge
  whatever its size.
- `crates/lurq/src/animation/animation.rs`: `AnimationEngine::new` registers `ui::sheet_keyframes()`, the
  identifiers `0xFF51` to `0xFF58`: `ui::SHEET_IN_FROM_START`, `SHEET_IN_FROM_END`, `SHEET_IN_FROM_TOP`,
  `SHEET_IN_FROM_BOTTOM`, `SHEET_OUT_TO_START`, `SHEET_OUT_TO_END`, `SHEET_OUT_TO_TOP`, `SHEET_OUT_TO_BOTTOM`.
- `crates/lurq/src/ui/part_style.rs`: `PartStyle::end_line(Border)` and `PartStyle::top_line(Border)` (fields
  `end_line`, `top_line`), a line along the end side and along the top of a part beside `underline` and
  `start_line`: the edge of a sheet at the window's start, and of a drawer at the bottom of a layout. `PartStyle` is
  public with public fields: the CHANGELOG's Breaking list wants "`PartStyle` has two fields more, `end_line` and
  `top_line`: one written out field by field must add them."
- For `docs/src/content/docs/modals.md`, Sheets, under the list: "`SheetProps::dismiss_on_escape(bool)` lets Escape
  close it apart from a press beside and a drag; `SheetProps::takes_focus(true)` moves the focus into it as it comes
  up and back as it goes down; `SheetProps::dress(..)` gives the backdrop, the panel and the grabber another look
  (`ui::Sheet` dresses them in the theme's). Under a window's own title bar it stands at the window's bottom." And in
  `animation-transforms.md`, where keyframes are described: "an `OffsetX` or `OffsetY` keyframe of
  `AnimatableValue::Percent(p)` moves the element by `p` percent of its own size."

## For `docs/src/content/docs/design-system.md`

## Sheet

A panel that slides in from an edge of the window, over the page, for what goes with the page without leaving it: edit a profile, filter a list, share a document.

```rust
use lurq::{
  components::SheetDetent,
  ui::{Button, ButtonVariant, Input, Sheet, SheetSide, SheetSize},
};

// open: Signal<bool>, the app's.
let close = open.clone();
Sheet::new(open.clone())
  .title("Edit profile")
  .description("Changes save when you close the panel.")
  .close_button(true)
  .child(Input::new(name.clone()))
  .footer(Button::new("Close").variant(ButtonVariant::Outline).on_click(move |_| close.set(false)))
  .footer(Button::new("Save"));

// From the start, wider.
Sheet::new(filters_open.clone()).side(SheetSide::Start).size(SheetSize::Md).title("Filters").children(filters);

// At the bottom, mounted: a finger drags it between its heights and down to put it away.
Sheet::new(goal_open.clone())
  .side(SheetSide::Bottom)
  .title("Move goal")
  .detents([SheetDetent::Content, SheetDetent::Large])
  .footer(Button::new("Submit"))
  .mount(ctx);
```

| Builder | |
| --- | --- |
| `new(Signal<bool>)` | Shown while the signal holds `true`. The sheet does not keep it: Escape, a press on the veil and the close button set it to `false` |
| `side(SheetSide)` | The edge it comes from: `End` (the default) and `Start` as high as the window, `Top` and `Bottom` as wide as it |
| `size(SheetSize)` | Across from its edge: `Sm` 320, `Md` 400, `Lg` 540 wide at a side, `Sm` 240, `Md` 360, `Lg` 480 high at the top or the bottom, `Full` the window. Without one it is 320 wide at a side and as high as what it holds at the top or the bottom. Never larger than the window |
| `title(&str)`, `description(&str)` | What it is, and more of it under the title. The title is what the sheet is called |
| `label(&str)` | What the sheet is called to tooling where it has no title |
| `child(element)`, `children(..)` | The body, which scrolls between the header and the footer |
| `footer(element)` | Adds to the footer, the least action first and the main one last: a row at the panel's end under a line, or at the bottom a column of buttons as wide as the sheet with the main one on top |
| `close_button(bool)` | A ghost button with the close icon at the end of the header |
| `grabber(bool)` | The handle at the top. A sheet at the bottom has it unless told otherwise |
| `dismiss_on_veil(bool)`, `dismiss_on_escape(bool)` | Whether a press on the veil (and, mounted, a drag down), and Escape, close it. Both on by default |
| `detents([SheetDetent])`, `detent(Signal<usize>)`, `scroll(ScrollState)` | For a mounted sheet at the bottom: the heights it stands at, which it is at, and the body's scrolling, which hands a finger to the sheet once the body is at its top |
| `inline(bool)` | The panel where it is declared, without the veil and without taking the window: for a gallery, a screenshot, a test |
| `mount(ctx)` | At the bottom, the engine's draggable `Sheet` in this look; at another edge, the same as converting it |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the panel's; `key` tells two sheets of one component apart |

- It is the engine's `Modal` with `target(Root)` and `takes_focus(true)`, as a [dialog](#dialog) is: declared anywhere, it takes no room there and covers the window under the window's own title bar while it is open. As it opens the focus goes to its first control (the close button where it has one), and as it closes back to what had it. Nothing under the veil takes the pointer, and Tab and Shift+Tab go round the sheet's own controls. A press on the panel goes no further.
- It slides in from its edge over the theme's slow duration in the entering curve as the veil fades in, and out to it over the base duration in the leaving curve: keyframes that move the panel by its own width or height (`ui::SHEET_IN_FROM_START` .. `ui::SHEET_OUT_TO_BOTTOM`). With motion reduced (`Theme::reduced_motion`, state `Modal` with `ReducedMotion`) it fades where it stands.
- At the start and the end the body takes the room the header and the footer leave, so the footer stands at the panel's bottom; at the top and the bottom the sheet is as high as what it holds while that fits, and the body gives way and scrolls when it does not.
- `mount(ctx)` puts a sheet at the bottom on the engine's [sheet](../modals/#sheets): a finger drags it between its `detents` (or the height its size gives it, or its content's) and throws it down to close it, it rises over the on-screen keyboard and keeps clear of a phone's edges, and it comes up with a spring. The engine lays out the panel and the backdrop; the recipe's `Panel` and `Veil` dress them (`SheetProps::dress`), and what the panel holds is the sheet's. A theme's renderer builds the sheet that is converted, not the mounted one, which is made of the view's parts.
- Parts: `SheetPart::Veil`, `Panel`, `Grabber` (the strip at the top of a bottom sheet), `Handle` (its bar), `Header`, `Heading`, `Title`, `Description`, `Body`, `Footer`, `Close` (a [button](#button) restyled), `Separator` (the [separator](#separator) over the footer, restyled). Variants: `SheetSide`. Sizes: `SheetSize`. States: `SheetState::Modal` (over the window, not inline), `ReducedMotion`.
- In the design a sheet at a side is in `Background` with a line of 1 in `Border` along the edge it shows the page and the large shadow. The header is padded 20 at the top and the sides, the title 16/600 in `Foreground` over the description in 13 `MutedForeground`, 4 apart, the close button's icon of 16 in `MutedForeground` at its end; the body is padded 20 with 16 between what it holds; the footer is padded 16 and 20, its buttons 8 apart at the end, under a line of `Border`. At the top the line is along its bottom. At the bottom it is the board's drawer: a line all round, corners of `radius-xl` (12) at its top, a handle of 40 by 4 in `Muted` 8 under its top edge, the heading in the middle 20 under it, and its buttons as wide as it, 8 apart, 20 from the edges. The veil is `Scrim`.
- The board's phone and tablet sheets are the same parts restyled: the phone's bottom sheet a panel of 360 round 20 at its top with a handle of 36 in `Input`, its title 18 and its description 14, padded 16; the tablet's form sheet a panel of 520 by 360 round 16 with an edge all round. The gallery's page has them.
- To what reads the tree (`lurq_snapshot` of the MCP server) the panel is a `dialog` called by its title, or by its label.
- A renderer that draws another sheet has `view.panel()`, `view.grabber()`, `view.heading()`, `view.close()`, `view.header()`, `view.body()`, `view.footer()`, `view.separator()`, `view.contents()` (all of them in order), `view.veil(panel)` and `view.modal(panel)`.

## Drawer

A panel at one side of a layout, beside the page rather than over it, that opens and closes: the navigation of an app, a pane of details or of filters.

```rust
use lurq::{
  components::Row,
  ui::{Drawer, SheetSide},
};

// open: Signal<bool>, the app's. The drawer pushes the page aside.
Row::new()
  .child(Drawer::new(open.clone()).label("Navigation").children(places))
  .child(page);

// On a narrow window it lies over the page with a veil.
Drawer::new(open.clone()).label("Navigation").overlay(narrow).children(places);
```

| Builder | |
| --- | --- |
| `new(Signal<bool>)` | Open while the signal holds `true`. The app's: over the page, Escape and a press on the veil set it to `false` |
| `side(SheetSide)` | The side of the layout it stands at: `Start` (the default) and `End` open with its width, `Top` and `Bottom` with the height of what it holds |
| `label(&str)` | What it is called to tooling |
| `child(element)`, `children(..)` | What it holds, which scrolls where it is too small |
| `width(f32)`, `height(f32)` | A width of its own at a side (300 without one), a height of its own at the top or the bottom |
| `overlay(bool)` | Over the page with a veil, as a [sheet](#sheet) at its edge, rather than beside it: for a window too narrow to push the page aside |
| `dismiss_on_veil(bool)`, `dismiss_on_escape(bool)` | Over the page, whether a press on the veil, and Escape, close it. Both on by default |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the panel's |

- It is declared in the layout where it stands, at the start or the end of a row or the top or the bottom of a column. As it opens its room grows from nothing to the panel's over the theme's slow duration and pushes the page aside, and the panel, which keeps its size, comes out from the layout's edge and fades in; as it closes the room closes over it as it fades out, and what it held is gone, so that Tab does not reach it. With motion reduced it opens and closes at once.
- `overlay(true)` builds a [sheet](#sheet) at the drawer's edge, as large across as the drawer's panel, holding what the drawer holds laid out as in the drawer: it takes no room in the layout, slides in over a veil, and is a dialog to tooling.
- Parts: `DrawerPart::Root` (the room it takes, with the transition of its width or height), `Panel`, `Body`. Variants: `SheetSide`. States: `DrawerState::Open`.
- In the design (the board's navigation drawer) the panel is 300 wide in `Background` with a line of 1 in `Border` along the edge it shows the page; what it holds pads itself.
- To what reads the tree it is a `complementary` region called by its label.
- A renderer that draws another drawer has `view.body()`, `view.panel()` and `view.root(panel)`, and `view.is_open`.

## For `CHANGELOG.md`

- `ui::Sheet`: a panel that slides in from an edge of the window (`SheetSide::Start`, `End`, `Top`, `Bottom`) over a veil, in the board's sizes, with a header of a title, a description and a close button, a body that scrolls between it and a footer of buttons, the board's grabber and stacked buttons at the bottom, the focus going in and back and Tab kept inside; mounted at the bottom it is the engine's `Sheet` a finger drags between detents, which now takes `SheetProps::dress`, `dismiss_on_escape` and `takes_focus` and stands under a window's own title bar. An offset keyframe in percent moves an element by its own size, and `PartStyle` has `end_line` and `top_line`.
- `ui::Drawer`: a panel at one side of a layout that opens and closes with its width (or its height at the top or the bottom), pushing the page aside, or lies over the page as a `ui::Sheet` with `overlay(true)`.

## Decided

- **`ui::Sheet` is a builder, and a `Modal` with a panel at every edge.** A sheet from a side on a desktop needs none of the engine sheet's detents, drag or keyboard, and converting a builder into an element has no `Ctx` to mount the engine's component with. So converting it builds a `Modal` with the panel at its edge, and a sheet at the bottom that a finger is to drag is mounted with `Sheet::mount(ctx)`, which puts it on the engine's `Sheet` (whose timers and finger need a component). The engine's sheet is dressed in the recipe's look rather than drawn twice: it keeps laying out its panel (which reaches under the window so that a finger pulling it up shows no gap) and its backdrop, and `SheetProps::dress` gives them the colors, the edge, the corners and the shadow the component resolves. The spring of 500 ms that the engine set by eye beside the platform is kept for the mounted sheet: it is the feel of a finger, not the theme's slow step.
- **The slide is keyframes of the panel's own size**, so that one keyframe slides a panel of any width or height: the engine reads an offset keyframe in percent as a share of the node's size. The veil and the panel are siblings in a stack (the veil under, what holds the panel over it), so that the veil fades without fading the panel. A press beside the panel lands on what holds it, which closes the sheet.
- **`SheetState::Modal`** is set on what is laid over the window and not on an inline one, so that a sheet drawn in a gallery does not slide in where it stands. With motion reduced it fades.
- **The size is across from the edge**: a width at a side and a height at the top or the bottom, so that `SheetSize` means one thing at every edge. The board draws one sheet, 320 wide: that is `Sm` and what a sheet given no size is; `Md` 400 and `Lg` 540 are the steps a wider form needs, and 240, 360 and 480 the heights at the top and the bottom. A mounted sheet at the bottom with no detents stands at the height its size gives it (`Full` is the engine's `Large`), or at its content's.
- **The board's `Drawer` component is the sheet at the bottom.** The board draws the drawer as a panel round at its top with a grabber, its heading in the middle and its buttons as wide as it; the brief makes `ui::Drawer` the panel beside the page. So the board's drawer is `Sheet` with `SheetSide::Bottom`, and `ui::Drawer` takes the board's navigation drawer (`HSa8x`) for its panel.
- **The bottom sheet's layout is the view's**: the heading in the middle and the buttons one over the other, the main one (the last given) on top as the board has Submit over Cancel, with no line over them. A part's style says nothing of alignment, so a theme that wants the heading at the start renders it.
- **The grabber is on any sheet at the bottom**, as the board draws it, mounted or not; `grabber(false)` takes it off. The engine's own grabber is not used: the sheet's is in what the panel holds, so that it is laid out in flow and counted in the sheet's height.
- **The close button is 24, its icon 16 in `MutedForeground`**, so that the icon stands where the board draws it, 20 from the top and the end, with 4 of the header's 20 being the heading's own. The board draws the icon alone.
- **A sheet given no side is at the end**: the board's sheet has its line on its left, toward a page on its left.
- **Escape, a press beside and a drag** are two props, as the dialog's are; mounted, `dismiss_on_veil` covers the press beside and the drag down (the engine's `dismissable`), and `dismiss_on_escape` Escape.
- **`ui::Drawer` has no header**: what it holds is the app's (the board's navigation drawer is an account and a list of places, padding themselves). It is called by its `label`.
- **The drawer's panel keeps its size while its room opens**, and stands against the page in it, so that it comes out from the layout's edge rather than reflowing. It fades in and out (its root keeps a leaving child, `presence()`), and while the drawer is closed it holds nothing, so Tab never reaches what a closed drawer held.
- **A drawer over the page is a `ui::Sheet`**, not a second panel: `overlay(true)` builds the sheet at the drawer's edge with the drawer's width, its body laid out as the drawer's.
- **`PartStyle::end_line` and `top_line`**: the sheet at the start shows the page a line on its end, and the drawer at the bottom one on its top; `PartStyle` had lines only along the bottom and the start.

## Not done

Nothing.

## Differs from the board

- The board's `Drawer` (`UaJAF`) is `ui::Sheet` at the bottom here, and `ui::Drawer` is the panel beside the page the brief describes; see Decided.
- The board draws the sheet's close button as its icon alone; here it is a ghost button of 24 around the icon, which has a hover and a ring of the focus.
- The board's sheets stand alone; the veil, the slide and the sizes other than 320 are not on the board.
- The board's phone bottom sheet (`Qawrh`) has its heading at the start; restyled by rules in the gallery it keeps the heading in the middle, as alignment is not a field of a part's style. The board's list of actions in a card is the app's content and is left out there.
- The board's tablet form sheet (`wt2Lv`) has a bar of Cancel, the title and Done; restyled in the gallery it is the sheet's header with a close button, at the top.
- The board's navigation drawer casts the large shadow, standing over a phone's screen; `ui::Drawer` beside the page has none, and over the page (`overlay`) it is a sheet, which has it.
