# Navbar and Navigation Menu

## Changed outside the component's own files

- `crates/lurq/src/node/focus_group.rs`: `FocusGroup` has a field more, `panel_row`, set in `FocusGroup::new` (after
  `menu_row`), and a new `pub fn panel_row(bool)`, which sets `menu_row` too: a row of panels is a row of menus whose
  elements are links as well as buttons.
- `crates/lurq/src/app/runtime/menu_row.rs`:
  - `Tree::menu_row_key`: in a row of panels, Left and Right that leave an open panel close it (as before) and then
    focus the element beside it from the keyboard without pressing it; where that element says it opens and closes
    (`.focus_expanded(open, toggle)`, the toggle of a nested focus group's branch), its toggle is called with `true`.
    A row of menus (`menu_row` alone, the menubar) still presses its neighbour.
  - `find_menu_rows`: a row of panels is passed by, so Alt with a letter and Alt alone never act on a site's links,
    and a menubar after a navbar in the tree is still the window's first row of menus.
  - The module's doc says what a row of panels is.
- No change to `shared.rs`, `part_style.rs`, `recipe.rs` or `runtime.rs`. No keyframes.

## For `docs/src/content/docs/design-system.md`

## Navbar

The way round a site or an app along the top of its window: the brand at its start, its links, and a search, actions and who is signed in at its end.

```rust
use lurq::ui::{Avatar, NavItem, NavLink, NavPanel, Navbar, NavbarBrand, SearchField, TextBoxSize, icons};

Navbar::new()
  .label("Main")
  .brand(NavbarBrand::new("lurq").icon(icons::BOX).on_select(move || go_home()))
  .item(
    NavItem::new("Docs").current(at_docs).panel(
      NavPanel::new()
        .columns(2)
        .link(NavLink::new("Getting started").description("Install lurq and open your first window.").href("/docs"))
        .link(NavLink::new("Theming").description("Roles, palettes and dark mode.").href("/docs/theming")),
    ),
  )
  .item(NavItem::new("Components").href("/components"))
  .item(NavItem::new("Blog").href("/blog"))
  .end(SearchField::new(query.clone()).placeholder("Search").size(TextBoxSize::Sm))
  .end(account) // an Avatar that opens a Menu
  .sticky(true)
  .mount(ctx)
```

| `Navbar` | |
| --- | --- |
| `brand(element)` | What stands at its start: a `NavbarBrand` (a mark beside a name, pressed where it has `on_select`), or an element of the app's |
| `item(NavItem)`, `items(..)` | A link after the others; one with a `panel` opens a panel of links, as in a [navigation menu](#navigation-menu) |
| `end(element)` | What stands at its end, after what stands there: a [search field](#search-field), a [button](#button), an [avatar](#avatar) with its [menu](#menu) |
| `sticky(bool)` | It stays at the top of the page that scrolls it |
| `open(signal)`, `on_open_change(handler)` | Which item's panel is open, by its index: the app's `Signal<Option<usize>>`; and what hears it change. Without a signal the bar keeps its own |
| `menu_open(signal)`, `on_menu_open_change(handler)` | On a narrow window, whether the links are open over the page; and what hears the menu button turn it |
| `narrow(bool)` | Whether its links are behind the menu button, whatever the window's width: they are below the `md` breakpoint unless told |
| `label(&str)` | What the navigation region is called to tooling: "Main" unless told |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the bar's; its links are `{id}-links` (the items `{id}-links-0` and on, a panel `{id}-links-0-panel`), the menu button `{id}-menu`, the links over the page `{id}-drawer` |

- It is mounted (`mount(ctx)`): its links are a mounted navigation menu, and on a narrow window a mounted [sidebar](#sidebar). Converted into an element on its own it is drawn with what the signals it is given say, for a gallery or a test.
- Its links are a [navigation menu](#navigation-menu) in its links variant: each a stop of Tab, the page shown (`NavItem::current`) in the foreground and the others in the muted foreground, brought forward under the pointer. One with a panel opens it under the bar by a press, under the pointer after a moment, and with Enter, Space and Down, with all a navigation menu does.
- On a window narrower than the `md` breakpoint (or where `narrow(true)` says) the links go behind a ghost button of the `menu` icon at the bar's start, called "Open menu", into a `Sidebar` over the page, which slides in from the start over a veil: each link an item, the page shown selected, an item with a panel a group of its links under its name. Choosing one closes it, as Escape and a press on the veil do. The brand and what stands at the end stay in the bar.
- In the design the bar is 56 tall in `Background`, with a line of 1 in `Border` along its bottom, padded 16 at its sides with 24 between what it holds. The brand is a mark of 20 in `Foreground` 8 before its name in 16/600; the links are 20 apart in 13/500; what stands at the end is 24 apart. On a narrow window it is padded 8 with 8 between what it holds.
- Parts: `NavbarPart::Root`, `Brand`, `BrandMark`, `BrandName`, `Links` (the room of the links, which takes what the bar has to spare), `End`. States: `NavbarState::Narrow`, `Sticky`, `Interactive` (the brand is pressed). How its links look is the navigation menu's recipe, in its `Links` variant: a theme restyles them there, into the board's tablet tab bar, say (a bar on `Muted` with the full radius, its items 36 tall and round, the page shown on `Background` with a shadow).
- To what reads the tree the bar is a `navigation` called by its label, its links `link`s (the page shown with `a11y_current` `page`) or `button`s that open a panel, and the menu button a `button` with `a11y_expanded` and `a11y_controls` naming the links over the page.
- A renderer for `Navbar` gets a `NavbarView`: `view.root()` is the bar with its style and its role; `brand`, `links`, `end`, `menu_button` and `drawer` are what it holds, built, to put in it. `view.render()` builds the bar.

## Navigation Menu

The way round a site along a row of links, some of which open a panel of links under them: Docs, Components, Blog.

```rust
use lurq::ui::{Card, NavItem, NavLink, NavPanel, NavigationMenu, icons};

NavigationMenu::new()
  .label("Site")
  .item(
    NavItem::new("Docs").panel(
      NavPanel::new()
        .columns(2)
        .featured(Card::new().title("lurq 0.62").description("A navbar, a navigation menu and a sidebar."))
        .link(NavLink::new("Getting started").description("Install lurq and open your first window.").icon(icons::PLAY_CIRCLE).href("/docs"))
        .link(NavLink::new("Theming").description("Roles, palettes and dark mode.").href("/docs/theming")),
    ),
  )
  .item(NavItem::new("Blog").href("/blog"))
  .open(open.clone())
  .mount(ctx)
```

| `NavigationMenu` | |
| --- | --- |
| `item(NavItem)`, `items(..)` | An item after the others |
| `open(signal)` | Which item's panel is open, by its index: the app's `Signal<Option<usize>>`, which a press, the pointer and the keys set, and Escape, a press outside, a choice and the pointer going clear set to `None`. Without one the menu keeps its own |
| `on_open_change(handler)` | What hears a panel open, another take its place, or the open one close |
| `variant(NavigationMenuVariant)` | `Menu`, the board's: each item a box lit under the pointer and while its panel is open. `Links`: the text alone, as a navbar has it |
| `open_delay(..)`, `close_delay(..)` | How long the pointer rests on an item before its panel opens (the theme's base duration), and how long a panel stays after the pointer has left it and its item (the slow one) |
| `disabled(bool)`, `label(&str)` | No item can be pressed; what the navigation region is called to tooling, "Main" unless told |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the row's; its items are `{id}-0` and on unless they have ids of their own, each panel `{id}-0-panel`, its links `{id}-0-0` and on |

| `NavItem`, `NavPanel`, `NavLink` | |
| --- | --- |
| `NavItem::new(label)` | A link: `href(path)` (the router's), `on_select(handler)`, `current(bool)`, `icon(name)`, `disabled(bool)`, `id`. With `panel(NavPanel)` it opens a panel in place of going somewhere, and has a chevron |
| `NavPanel::new()` | `link(NavLink)`, `links(..)`; `columns(n)`, the first filled first (one unless told); `featured(element)`, a column of its own before the links: a [card](#card) |
| `NavLink::new(title)` | `description(&str)` under its title, `icon(name)` at its start, `href(path)`, `on_select(handler)`, `current(bool)`, `disabled(bool)`, `id` |

- It is mounted (`mount(ctx)`): it keeps the timers of its delays, where the pointer is and how the open panel was opened. Converted into an element on its own it opens its panels by a press and the keys, from the signal `open` gives.
- The pointer: a press of an item with a panel opens the panel, and a press again closes it; resting on the item opens it after the open delay (a press that follows keeps it). One panel is open at a time, and with one open the pointer coming onto another item opens that one at once. The panel stays while the pointer is on the item, on the panel or crossing the 6 between them, and closes when it has left them for the close delay; a press outside it closes it and goes on to what it is on. Choosing a link runs it, goes to its path and closes the panel.
- The keyboard: each item is a stop of Tab, as links on a page are, and Left and Right move between the items as well. Enter, Space and Down open an item's panel with the focus on its first link (Down on an item whose panel the pointer opened takes the focus into it); Up and Down go through the panel's links, column after column; Left and Right close it and go to the item beside it, opening that one's panel where it has one and only focusing a link, which is not followed; Escape closes it with the focus back on its item. Tab goes from an item into its open panel and from its last link on to what follows the item, closing the panel; the focus leaving the item and the panel closes it. Alt does nothing to it.
- Under the pointer, focused from the keyboard, or with its panel open, an item is filled with `Highlight`, the board's `accent`; the page shown, or an item whose panel holds it, is on `Muted`. In the design an item is 36 tall, padded 12 with the medium radius, its text 13/500 in `Foreground`, and an item with a panel has a chevron of 14 in `MutedForeground` 6 after it, up while its panel is open; the items are 4 apart. The panel opens 6 under its item, from its start, in `Popover` with an edge of 1 in `Border`, the large radius and the medium shadow, padded 8 with 8 between its columns, each 208 wide; it fades in over the theme's base duration and out over the fast one. A link is padded 12 with the medium radius, its title 14/500 over its description 13 in `MutedForeground`, 4 apart and 1.4 high, an icon of 16 in `MutedForeground` 12 before them; it is lit with `Highlight` under the pointer and focused. In the `Links` variant an item is its text alone, 32 tall, 20 apart, in `MutedForeground`, in `Foreground` under the pointer, while its panel is open and where it is the page shown, with the focus ring around it; its panel opens 18 under it, 6 past the edge of a navbar.
- The engine's part is `FocusGroup::panel_row(true)`, which the row has: a [row of menus](#menubar) whose elements are links as well as buttons. Left and Right in an open panel that nothing in it takes close it and focus the element beside it, asking it to open where it says it opens and closes (`focus_expanded(open, toggle)`), and press nothing; Alt is a menubar's, not its.
- Parts: `NavigationMenuPart::Bar`, `Item`, `ItemIcon`, `Label`, `Chevron`, `Viewport` (the room between an item and its panel, which keeps the panel under the pointer), `Panel`, `Column`, `Featured`, `Link`, `LinkIcon`, `LinkText`, `LinkTitle`, `LinkDescription`. States: `NavigationMenuState::Open`, `Current`, `Disabled`; variants: `NavigationMenuVariant::Menu`, `Links`.
- To what reads the tree the row is a `navigation` called by its label (in a navbar, a `list` in the navbar's), an item a `link` (with `a11y_current` `page` for the page shown) or a `button` with `a11y_expanded` and `a11y_controls` naming its panel, the panel a `group` called by its item, and its links `link`s with their description.
- A renderer for `NavigationMenu` gets a `NavigationMenuView`: `view.bar()` is the row with its style, its role and its focus group; `view.item(index)` an item's behavior with its style, `view.item_icon(index)`, `view.label(index)` and `view.chevron(index)` what it holds, and `view.panel(index)` its panel hung from it, to put after it (`view.panel_content(index)` is the panel alone); `view.item_part(index, part)` is the style of a part of one item. `view.render()` builds the menu.

## For `CHANGELOG.md`

- `ui::Navbar`: the way round a site or an app along the top of its window, its brand (`NavbarBrand`), its links (`NavItem`, of which one may open a panel of links) and what stands at its end, sticky where told. Its links are a `NavigationMenu` in its links variant; on a window narrower than `md` they go behind a menu button into a `Sidebar` over the page. See [Navbar](docs/src/content/docs/design-system.md#navbar).
- `ui::NavigationMenu`: a row of links, some of which open a panel of links (`NavPanel` of `NavLink`s in columns, with a featured element) under them, by a press, under the pointer after a delay and from the keyboard, the open panel handed to its neighbour by the pointer and by Left and Right. Each item is a stop of Tab. The engine's part is `FocusGroup::panel_row`. See [Navigation Menu](docs/src/content/docs/design-system.md#navigation-menu).

## Decided

- **A stop of Tab for each link, not one stop with the arrows.** A navbar is a `navigation` region of links, not a `menubar`: a screen reader lists its links and moves through them with Tab as it does through a page's, a browser's own `<nav>` is so, and the WAI-ARIA authoring practices give site navigation the disclosure pattern (each link and each button that opens a panel in the order of Tab) and keep the one-stop, arrow-key pattern for an app's `menubar`, warning against it for a site. The arrows move between the items as well, which the disclosure pattern allows and costs nothing; the menubar's Alt with a letter is not a site's.
- **Both are mounted components.** The navigation menu keeps two timers (the open and the close delay) and a third that runs after the focus moves, where the pointer is, how the open panel was opened, and a signal and an anchor for each panel, which a builder cannot keep (as `HoverCard` and `Sidebar` are mounted). The navbar mounts its navigation menu, or on a narrow window its sidebar, and keeps the narrow window's open signal where the app gives none. Each converts into an element on its own for a gallery or a test, opening what the app's signals say.
- **What closes a panel depends on how it opened.** Opened under the pointer or by its press, the pointer leaving the item and the panel for the close delay closes it, and a press that follows the pointer's opening keeps it (a person who rested on Docs and then pressed it wants it open). Opened from the keyboard, it takes the focus (`Popup::takes_focus`) and the pointer does not close it; opened by the pointer, the focus stays where it is (`Popup::tab_after_anchor`) and Tab goes from the item into it. Either way the focus leaving the item and the panel closes it, as does Escape, a press outside, and a choice.
- **The room between an item and its panel is a part of the panel's layer, `Viewport`**, hit by the pointer: crossing it keeps the panel as the panel itself does, with no timer racing the pointer. A panel's `Viewport` is 6 high in the menu variant (the board's gap) and 18 in the links variant (to 6 past a navbar's edge).
- **No shared viewport.** The board draws one open panel, under the first item from the bar's start, and no move between items; each panel hangs from its own item and fades in and out as another takes its place. A viewport that moves and resizes from one item to the next would be a single layer anchored to the row with a transition of its offset and size, and the board does not ask for it.
- **The engine is extended, not copied**: `FocusGroup::panel_row` is `menu_row` with two differences a row of links needs. A link beside a panel is focused and not pressed (going past a link must not follow it), and a button that opens a panel is asked to open through the toggle it already gives a nested group (`focus_expanded`), rather than pressed, which says to it that the keyboard opened it. Alt is a menubar's: a navbar first in the tree would otherwise take the window's Alt.
- **The narrow form is the `Sidebar`'s over the page**, mounted with `narrow(true)` and the navbar's open signal: its drawer, its veil, its keys (a tree, Tab stopping once on the page shown), closing when a link is chosen. An item with a panel is a group of the panel's links, labelled by the item; what the panel features is not repeated there. The menu button is a ghost `ui::Button` of the `menu` icon (the `close` icon while the links are open), not `SidebarTrigger`, whose icon and name are a sidebar's.
- **`NavbarBrand` is pressed through `on_select`, not `href`**: a builder has no router to go to a path with, and the navbar's brand is the app's element; an app with a router pushes its home in the handler.
- **`sticky`**: the board draws no page scrolling under a navbar, but a header that stays is what a person expects of one, and the engine's `Sticky::top(0.0)` gives it as it does `Banner::sticky`.
- **The parts are named so that the board's app bar and tab bar are restyles.** The tablet tab bar (`flEPC`) is the navigation menu's `Links` variant restyled: `Bar` on `Muted` with the full radius and padding 4, `Item` 36 tall, round, padded 16, the `Current` item on `Background` with a small shadow, `Label` 14/500. The mobile app bar (`ZuqgI`) is the navbar's narrow form restyled: `Root` padded 4, `BrandName` 17/600, with the menu button and the end's action at its ends.
- **`on_open_change` and `on_menu_open_change`**: what a stage's Events tab hears of a press that opens a panel or the links over the page, and what an app that counts its menu's use needs, where the app's signal could be watched only from a component of its own.
- **A link's icon and a panel's featured element** are built though the board draws neither: the brief asks for them, and a site's panel has them. The icon is 16 in `MutedForeground` at the link's start, 12 before its text, 2 lower to stand at the title's line; the featured element is a column of the columns' width before them, its look the app's (a `Card`).
- **The current item of the menu variant is on `Muted`**: the board draws no page shown in a navigation menu; `Highlight` is the open item's, and the navbar's links bring the page shown forward with the foreground instead.

## Not done

Nothing.

## Differs from the board

- The board draws the navigation menu with Docs open (filled with `accent`, its chevron up) and its panel; it draws no item under the pointer, no focus, no page shown, no disabled item, no icon on a link and no featured column: these are as the decisions above say, the focus of the menu variant `Highlight` as the menubar's titles, that of the links variant the theme's focus ring.
- The board's navbar has no item that opens a panel and no narrow form: its links' chevron and panel are the navigation menu's, its panel 6 past the navbar's edge; the narrow form is the sidebar's drawer behind a ghost button of 36, where the board's mobile app bar has buttons of 44 and padding 4.
- The board's navbar's search is the `Input` reference (`Ptl3b`) 200 by 32 with its search icon; the gallery and the docs use `SearchField` at its small size in a row of 200, which is that input with a clear button.
- The board's navbar's links are text in a row with no box; here each is a 32 tall item with no padding, so that the focus ring and the pointer have a box, and the text stands where the board has it.
