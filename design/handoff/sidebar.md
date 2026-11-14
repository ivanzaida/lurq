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
- No change to the engine, `shared.rs`, `part_style.rs` or `recipe.rs`. No keyframes. `collapsible::opening` (crate-private)
  is used as it is, for what opens and closes with its height.

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

- The items are one list to the keyboard (`FocusGroup::vertical().one_stop(true).typeahead(true)`): Tab stops at it once, on the selected item; Up and Down move through the items shown, nested ones included, Home and End go to the first and the last, and a letter to the item whose name starts with it. Right opens an item's nested items and Left closes them; on the rail Right opens its menu.
- Collapsed, it is the rail: each item is its icon, says its name (and its keys) in a `Tooltip` at its right, and an item with nested items opens them in a `Menu` there, the selected one marked. An item whose nested item is selected is the selected one on the rail.
- On a window narrower than the `md` breakpoint it lies over the page in a `Drawer` with `overlay(true)`, as wide as it is beside the page; whatever `collapsed` says, it is whole there. Choosing an item closes it.
- Which item is selected is the app's: with a router, `.selected(ctx.route_path() == path)` on each, and `.href(path)` to go there. The gallery's shell does this for its pages.
- `SidebarTrigger::new(collapsed).open(open)` is a ghost button of the `panel-left` icon, called by which way it goes: "Collapse sidebar", "Expand sidebar", or on a narrow window "Open sidebar", "Close sidebar". `narrow(bool)` tells it as the sidebar is told.
- `SidebarHeading::new(title)` is the row of the header and the footer: `description`, `icon(name)` (a mark on the primary fill) or `mark(element)` (an avatar), `trailing_icon(name)`, `variant(SidebarHeadingVariant::Workspace | Account)`, `collapsed(bool)` (the mark alone, for the rail), `on_click` and `anchor(ElementRef)` (a button, from which a switcher's menu can hang).
- Parts: `SidebarPart::Root`, `Header`, `Body` (what scrolls), `Footer`, `Group`, `GroupLabel`, `GroupText`, `GroupCount`, `GroupChevron`, `GroupAction`, `Item`, `ItemIcon`, `ItemLabel`, `ItemCount`, `ItemBadge`, `ItemShortcut`, `ItemChevron`, `ItemIndent` (one for each level an item is nested), `Nested`, `NestedContent`, `Heading`, `HeadingMark`, `HeadingText`, `HeadingTitle`, `HeadingDescription`, `HeadingTrailing`. States: `SidebarState::Collapsed`, `Overlay`, `Selected`, `Disabled`, `Open`, `Interactive` (a group's label that is pressed, a heading with a handler). Variants of a heading: `SidebarHeadingVariant::Workspace`, `Account`.
- In the design the sidebar is 260 wide on `Muted` with a line of 1 in `Border` along its end, padded 8, 2 between rows. An item is 32 high (it follows the density: 24, 40 and 44 at compact, comfortable and touch), padded 8 across, 8 between an icon of 16 in `MutedForeground` and its name in 13; the selected one is on `Background` with an edge of `Border`, its name in the medium weight and its icon in `Foreground`. Under the pointer an item has the `StateHover` veil, held down `StatePressed`, and the ring of the focus is inside it. A group's label is 11/600 in capitals, 0.6 apart, in `MutedForeground`, 10 over it and 4 under it. A nested item is 18 further in, after the chevron of 14 of the item it is in. The heading of the header is 44 high with a mark of 28 on `Primary`, its title 13/600 over a description of 11 in `MutedForeground`; the footer's is under a line of 1 and 48 high, its name in the medium weight. The rail is 52 wide, its items 36 by 32, 4 apart.
- It is a mounted component: the tooltips of the rail are mounted, and it keeps which nested lists and groups are open and the anchor and the open signal of each menu of the rail.
- To what reads the tree it is a `navigation` called by its label, its items `link` (with `href`) or `button`, the selected one with `a11y_current` `page`, an item with nested items with `a11y_expanded`, and a nested list a `group`.
- A theme's renderer (`SidebarRenderer`) is given a `SidebarView` with the groups built, the header and the footer as the sidebar is now, and `header()`, `body()`, `footer()` and `root(children)` to lay them out; `render()` is the default.

## For `CHANGELOG.md`

- `ui::Sidebar`: the way round an app down one side of its window, with groups, nested items that open with their height, counts, badges and shortcuts, the selected item, a header and a footer (`SidebarHeading`), the rail of icons with tooltips and menus whose width moves, the sidebar over the page on a narrow window, and `SidebarTrigger`; the gallery's own sidebar is one now.

## Decided

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

- Over the page, the header and the footer scroll with the items: the drawer's body is a scroll of its own (`DrawerView::body`, a `ScrollVertical`), and the sidebar is in it, so its own scroll has no height to keep them out of. The smallest change: a drawer that lies over the page lays what it holds out at the panel's height and does not scroll it, behind a flag (`Drawer::scroll(false)`, passed to the sheet as `SheetPart::Body` with no scroll), which the sidebar sets.

## Differs from the board

- The trigger has one icon, `panel-left`, whichever way it goes; its name says the way. The theme's set has no `panel-left-open` and `panel-left-close`.
- The reusable Sidebar Item, alone in the Lists frame and the App shell examples, has its selected one on `Highlight` (the board's `accent`); the sidebar's selected item is on `Background` with an edge, as the board's Sidebar draws it, wherever it is.
- The board's header has a search under the workspace; that is the app's element in `header`, a `ui::Input`, not a part of the sidebar.
