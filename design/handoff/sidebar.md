# Sidebar

## Changed outside the component's own files

- `crates/gallery/src/shell.rs`: the gallery's side bar, drawn by hand, is now a `ui::Sidebar` (`sidebar(ctx, state, current)`):
  the groups of pages in `Group::ALL` with how many each has, an item for each page with its icon and `.href(page.path)`, the
  page the route is at selected. The shell keeps two signals more, `collapsed` and `sidebar_open`, and the header has a
  `SidebarTrigger` before the brand: on a desktop it collapses the sidebar to its rail, below the `md` breakpoint (the window
  goes down to 720) it opens the sidebar over the page, which closes when a page is chosen. The page's own line at its start
  is gone: the sidebar's line along its end is it. `SIDEBAR`, `Link`, `overline` and `row_height` are no longer used by the
  shell (`kit::overline` and `kit::row_height` stay, other pages use them). The sidebar is the component's: 260 wide on the
  muted fill, its labels as the board writes them, where it was 248 on the canvas with the labels in the mono family.
- The three `mod.rs` files: the `mod` lines, the `pub use` of `ui::sidebar`, and the gallery's page entry after Steps.
- `crates/lurq/src/ui/drawer.rs`: `Drawer::scroll(bool)` (and `DrawerView::scroll`, a public field): whether what the
  drawer holds scrolls. It does by default; what does not is laid out at the panel's height (`DrawerView::body` returns the
  body column, stretched and growing, not a `ScrollVertical`), beside the page and over it. The sidebar sets it over the
  page, so that its header and its footer stay while its own body scrolls. Tested in `tests/ui/drawer.rs`
  (`a_drawer_that_does_not_scroll_lays_what_it_holds_out_at_its_height`). The line for the `## Drawer` section of the
  docs, in its table of builders after `overlay(bool)`:
  `| `scroll(bool)` | Whether what it holds scrolls where the drawer is too small for it. It does unless told; what does not is laid out at the panel's height, for what holds a scroll of its own with something that stays beside it, as a sidebar's header and footer |`
- `crates/lurq/src/ui/sheet.rs`: `Sheet::body_scrolls(bool)`, what `Drawer::scroll` gives the sheet over the page: a body
  that does not scroll is laid out in the room the panel has for it (`SheetView::body`). The name is not `scroll`, which a
  sheet has already for its `ScrollState`. The line for the `## Sheet` section of the docs, in its table of builders:
  `| `body_scrolls(bool)` | Whether the body scrolls. It does unless told; one that does not is laid out in the room the panel has for it, for what holds a scroll of its own |`
- No change to the engine, `shared.rs`, `part_style.rs` or `recipe.rs`. No keyframes. `collapsible::opening` (crate-private)
  is used as it is, for what opens and closes with its height. The keys of the tree are the engine's
  `FocusGroup::nested` with `focus_level` and `focus_expanded`, from the TreeView session.

## For `docs/src/content/docs/design-system.md`

## Sidebar

The way round an app, down one side of its window: its groups of places, the one the app is at, a header with the app or the workspace and a footer with who is signed in.

```rust
use lurq::ui::{Avatar, Sidebar, SidebarGroup, SidebarHeading, SidebarHeadingVariant, SidebarItem, SidebarTrigger, icons};

let heading = |collapsed| SidebarHeading::new("lurq").description("Workspace").icon(icons::BOX).collapsed(collapsed);
let account = SidebarHeading::new("Ivan Zaida")
  .variant(SidebarHeadingVariant::Account)
  .description("ivan@lurq.dev")
  .mark(Avatar::new("Ivan Zaida"));

Sidebar::new()
  .label("Workspace")
  .header(heading(false))
  .header_collapsed(heading(true))
  .group(
    SidebarGroup::new("Project")
      .item(SidebarItem::new("Overview").icon(icons::HOME).href("/"))
      .item(SidebarItem::new("Inbox").icon(icons::INBOX).count(12).selected(ctx.route_path() == "/inbox"))
      .item(
        SidebarItem::new("Components")
          .icon(icons::FOLDER)
          .child(SidebarItem::new("Buttons").icon(icons::FILE))
          .child(SidebarItem::new("Inputs").icon(icons::FILE)),
      ),
  )
  .footer(account)
  .collapsed(collapsed.clone())
  .open(open.clone())
  .mount(ctx);

// Wherever the app puts it: collapses the sidebar, or opens it over the page on a narrow window.
SidebarTrigger::new(collapsed.clone()).open(open.clone());
```

| Builder | |
| --- | --- |
| `label(&str)` | What the navigation region is called to tooling. "Sidebar" unless told |
| `header(element)`, `footer(element)` | What stays at its top and its bottom while its items scroll: a `SidebarHeading`, a search. The footer has a line over it |
| `header_collapsed(element)`, `footer_collapsed(element)` | What the rail shows in their place: the mark alone, an avatar. Nothing unless told |
| `group(SidebarGroup)`, `groups(..)`, `item(SidebarItem)` | Groups of items; `item` adds to a group of no label |
| `collapsed(bool \| Signal<bool>)` | The rail of the items' icons, 52 wide. Its width moves between the two over the theme's slow duration |
| `open(Signal<bool>)` | On a narrow window, whether it is open over the page. `SidebarTrigger` sets it; choosing an item, Escape and a press on the veil clear it |
| `narrow(bool)` | Whether it lies over the page whatever the window's width: it does below the `md` breakpoint unless told |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the root |
| `mount(ctx)` | Puts it where it goes. Converted into an element on its own it is drawn as it is at rest, with no tooltips on the rail: for a gallery or a test |

| `SidebarGroup` | |
| --- | --- |
| `new(&str)` | A group under a label, written in capitals; an empty one has none |
| `count(usize)` | How many it has, at the end of its label |
| `action(element)` | An action at the end of its label: a small ghost button. The arrow keys of the items pass it by |
| `collapsible(bool)`, `open(bool)` | Its label opens and closes it; `open` is how it starts, and the sidebar keeps it from then on |
| `item(SidebarItem)`, `items(..)`, `key` | Its items; `key` tells two groups of one label apart |

| `SidebarItem` | |
| --- | --- |
| `new(&str)`, `icon(name)` | Its name, and the icon before it: what the rail shows of it (the first letter of its name where it has none) |
| `count(usize)`, `badge(element)`, `shortcut(&str)` | At its end: a count in the mono family, a `Badge`, the keys that go to it as `Kbd::shortcut` writes them |
| `selected(bool)` | It is where the app is: the app's, following its route |
| `disabled(bool)` | It is dimmed and takes nothing |
| `on_select(handler)`, `href(path)` | What choosing it does: the handler, and with the `router` feature the path the router goes to. A press, Enter and Space choose |
| `child(SidebarItem)`, `children(..)`, `open(bool)` | Nested items, which open and close with their height under it; `open` is how they start, open where one of them is selected unless told |
| `force(..)`, `id`, `key` | As a button's. `key` tells two items of one name apart |

- The items are a tree to the keyboard (`FocusGroup::vertical().one_stop(true).typeahead(true).nested(true)`, each item at its `focus_level`, one with nested items `focus_expanded`): Tab stops at it once, on the selected item; Up and Down move through the items shown, nested ones included, Home and End go to the first and the last, and a letter to the item whose name starts with it. Right opens a closed item and on an open one goes to its first nested item; Left closes an open one and on a nested item goes to the item it is in. A group that opens and closes is an item its items are in. On the rail the items are a list, and Right opens an item's menu.
- Collapsed, it is the rail: each item is its icon, says its name (and its keys) in a `Tooltip` at its right, and an item with nested items opens them in a `Menu` there, the selected one marked. An item whose nested item is selected is the selected one on the rail.
- On a window narrower than the `md` breakpoint it lies over the page in a `Drawer` with `overlay(true)` and `scroll(false)`, as wide as it is beside the page, its header and footer staying while its items scroll; whatever `collapsed` says, it is whole there. Choosing an item closes it.
- Which item is selected is the app's: with a router, `.selected(ctx.route_path() == path)` on each, and `.href(path)` to go there. The gallery's shell does this for its pages.
- `SidebarTrigger::new(collapsed).open(open)` is a ghost button of the `panel-left` icon, called by which way it goes: "Collapse sidebar", "Expand sidebar", or on a narrow window "Open sidebar", "Close sidebar". `narrow(bool)` tells it as the sidebar is told.
- `SidebarHeading::new(title)` is the row of the header and the footer: `description`, `icon(name)` (a mark on the primary fill) or `mark(element)` (an avatar), `trailing_icon(name)`, `variant(SidebarHeadingVariant::Workspace | Account)`, `collapsed(bool)` (the mark alone, for the rail), `on_click` and `anchor(ElementRef)` (a button, from which a switcher's menu can hang).
- Parts: `SidebarPart::Root`, `Header`, `Body` (what scrolls), `Footer` (its room at the sidebar's sides), `FooterContent` (the line and what is under it), `Group`, `GroupLabel`, `GroupText`, `GroupCount`, `GroupChevron`, `GroupAction`, `Item`, `ItemIcon`, `ItemLabel`, `ItemCount`, `ItemBadge`, `ItemShortcut`, `ItemChevron`, `ItemIndent` (one for each level an item is nested), `Nested`, `NestedContent`, `Heading`, `HeadingMark`, `HeadingText`, `HeadingTitle`, `HeadingDescription`, `HeadingTrailing`. States: `SidebarState::Collapsed`, `Overlay`, `Selected`, `Disabled`, `Open`, `Interactive` (a group's label that is pressed, a heading with a handler). Variants of a heading: `SidebarHeadingVariant::Workspace`, `Account`.
- In the design the sidebar is 260 wide on `Muted` with a line of 1 in `Border` along its end, padded 8, 2 between rows. The 8 at its sides is its header's, body's and footer's: the body reaches the sidebar's edge, and at its end it has the lane of the theme's scroll bar (its width and its inset on either side, 12 with the default bar) where that is more than 8, so that the bar, which is drawn over what scrolls, lies beside the rows and not over them. The rail shows no bar: it scrolls with the wheel and the keys. An item is 32 high (it follows the density: 24, 40 and 44 at compact, comfortable and touch), padded 8 across, 8 between an icon of 16 in `MutedForeground` and its name in 13; the selected one is on `Background` with an edge of `Border`, its name in the medium weight and its icon in `Foreground`. Under the pointer an item has the `StateHover` veil, held down `StatePressed`, and the ring of the focus is inside it. A group's label is 11/600 in capitals, 0.6 apart, in `MutedForeground`, 10 over it and 4 under it. A nested item is 18 further in, after the chevron of 14 of the item it is in. The heading of the header is 44 high with a mark of 28 on `Primary`, its title 13/600 over a description of 11 in `MutedForeground`; the footer's is under a line of 1 and 48 high, its name in the medium weight. The rail is 52 wide, its items 36 by 32, 4 apart.
- It is a mounted component: the tooltips of the rail are mounted, and it keeps which nested lists and groups are open and the anchor and the open signal of each menu of the rail.
- To what reads the tree it is a `navigation` called by its label, its items `link` (with `href`) or `button`, the selected one with `a11y_current` `page`, an item with nested items with `a11y_expanded`, and a nested list a `group`.
- A theme's renderer (`SidebarRenderer`) is given a `SidebarView` with the groups built, the header and the footer as the sidebar is now, and `header()`, `body()`, `footer()` and `root(children)` to lay them out; `render()` is the default.

## For `CHANGELOG.md`

- `ui::Sidebar`: the way round an app down one side of its window, with groups, nested items that open with their height, counts, badges and shortcuts, the selected item, a header and a footer (`SidebarHeading`), the rail of icons with tooltips and menus whose width moves, the sidebar over the page on a narrow window, and `SidebarTrigger`; the gallery's own sidebar is one now.
- `ui::Drawer::scroll(false)` and `ui::Sheet::body_scrolls(false)`: what the drawer or the sheet holds is laid out at the panel's height and does not scroll, for what has a scroll of its own with something that stays beside it.

## Decided

- The scroll bar of the body: `ui::Dialog`'s body and the command palette's list leave the bar over their content, with nothing at their end, and their rows are text that the bar seldom reaches; a sidebar's rows have counts, badges and the selected item's edge at their end, which the bar covered in the gallery. So the body's `ScrollVertical` reaches the sidebar's edge (the root has no padding at its sides, its regions have the 8), and the body's padding at its end is the theme's bar lane where that is more than the body's own (`SidebarView::bar_room`): the bar lies in that padding. The rail hides its bar (`ScrollBarStyle::hidden()`): its 52 have no room beside the icons.

- It is a mounted component (`Sidebar::mount(ctx)`), not a builder: the rail names its items in tooltips, which are mounted, and it keeps which nested lists and groups are open, and an anchor and an open signal for each menu the rail opens, none of which a builder can keep. A sidebar converted into an element on its own is drawn as it is at rest, without tooltips: what the gallery's detached window and a test without a `Ctx` use.
- Whether a nested list is open is the sidebar's, not the app's, as a tree's is: an app gives how it starts (`open(bool)`, or open where a nested item is selected). The selected item is the app's, as the brief says.
- The nested list is not a `ui::Collapsible`: a collapsible puts what it holds in `FocusGroup::none()`, out of the arrows of the list around it, and the nested items must be in the sidebar's one list. It is opened as a collapsible is, with `collapsible::opening` (a height of none and of the content's, over the base duration, the content fading).
- Over the page it is a `Drawer` with `overlay(true)` (a `Sheet`), as wide as the sidebar beside the page, and on the drawer's fill, which is the background, as the board's Mobile/Navigation Drawer is. The rail is never shown over the page.
- A narrow window is one below the `md` breakpoint (768), as `Breakpoint` orders them; with no breakpoint (below `sm`) it is narrow too. The board's Tablet/Sidebar keeps a sidebar beside the page on a tablet, so `md` and not `lg`.
- The name of an item is 13, as the board's Sidebar draws it, where the reusable Sidebar Item, used alone in the Lists frame, is 14. A group's label is uppercased by the renderer, as the board writes it: the theme has no transform of text.
- The item under the pointer has the `StateHover` veil: the board draws no hover, and `Highlight` is the muted fill itself in the light theme, so it would not show on the sidebar.
- The indent is a part, `ItemIndent`, 10 wide for each level, which with the item's gap of 8 is the board's 18: a theme can draw a line in it (a table of contents' guide).
- A group's count is in the mono family at 11 in `MutedForeground`, as an item's count is; the board's groups have no count, the brief asks for one.
- On the rail, a menu has the item's name over its items as a label: the item's name is otherwise only in its tooltip. The selected nested item is marked in it as the chosen tab is in the tabs' menu (`MenuItem::radio`).
- An item with nested items and a handler of its own is chosen and opened by one press.

## Not done

Nothing.

## Differs from the board

- The rows end 12 before the sidebar's edge, not 8, with the default scroll bar: the bar is drawn over what scrolls (an overlay), and its lane is kept clear of the rows, whether or not the items overflow (a gutter that came and went with the overflow would move the rows as the list grows). Of the two ways the engine has, a padding at the body's end and not a `ScrollBarPlacement::Reserved` gutter: a reserved gutter would come on top of the body's own padding of 8 at its end. The header and the footer keep the board's 8.

- The trigger has one icon, `panel-left`, whichever way it goes; its name says the way. The theme's set has no `panel-left-open` and `panel-left-close`.
- The reusable Sidebar Item, alone in the Lists frame and the App shell examples, has its selected one on `Highlight` (the board's `accent`); the sidebar's selected item is on `Background` with an edge, as the board's Sidebar draws it, wherever it is.
- The board's header has a search under the workspace; that is the app's element in `header`, a `ui::Input`, not a part of the sidebar.
