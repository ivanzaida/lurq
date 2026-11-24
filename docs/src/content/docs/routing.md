---
title: Routing
description: Declarative routes, nested layouts, links, params, guards, and navigation.
---

# Routing

Requires the `router` feature flag.

```toml
lurq = { version = "0.73.0", features = ["router"] }
```

The router is an in-process UI router. It does not depend on browser URLs; it owns a reactive path signal and renders the matching route tree inside your `Tree`.

Use it when an app has multiple screens, a persistent shell, nested content areas, or navigation history.

## Minimal Router

Create a `RouterHandle` in `create` with `ctx.router`, then render it with `Router::mount`.

```rust
use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Router, Text},
  node::Element,
  router::{RouterHandle, Routes},
};

struct AppRoot {
  router: RouterHandle,
}

impl Component for AppRoot {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let router = ctx.router(
      Routes::new()
        .route("/", |_ctx| Text::new("Home").into())
        .route("/settings", |_ctx| Text::new("Settings").into()),
    );

    router.replace("/");
    Self { router }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    Router::mount(ctx, self.router.clone())
  }
}
```

`ctx.router` creates the handle once for the component. Call `replace("/")` or `push("/")` during `create` to set the initial route.

## Layout Routes

Use `layout` when several routes share a shell. Render `Outlet::mount(ctx)` where the child route should appear.

```rust
use lurq::{
  app::ctx::Ctx,
  components::{Column, Link, Outlet, Text},
  node::Element,
  router::Routes,
};

fn routes() -> Routes {
  Routes::new().layout(
    "/",
    |ctx| app_shell(ctx),
    |routes| {
      routes
        .route("/", |_ctx| Text::new("Dashboard").into())
        .route("/profile", |_ctx| Text::new("Profile").into())
        .route("/settings", |_ctx| Text::new("Settings").into())
    },
  )
}

fn app_shell(ctx: &mut Ctx) -> Element {
  Column::new()
    .child(Link::build(ctx, "Dashboard", "/"))
    .child(Link::build(ctx, "Profile", "/profile"))
    .child(Link::build(ctx, "Settings", "/settings"))
    .child(Outlet::mount(ctx))
    .into()
}
```

Nested layouts work the same way: a child layout can render another `Outlet`.

## A Stack Of Screens

`StackOutlet` goes where an `Outlet` would, and moves between screens as a phone's apps do. It is added in
**lurq 0.61.0**. An `Outlet` shows the current route and drops the one before; a `StackOutlet` keeps every
entry of the history, from the first to the current one, as a screen of its own:

- a `push` brings the new screen in from the right, over the one before;
- `back` takes the top screen out to the right and unmounts it;
- a screen under the top one stays mounted, with its state and its scroll, and is not laid out or painted
  until it shows again;
- on a touch screen a finger in from the left edge drags the top screen, and takes it back when lifted past
  half the width or fast enough;
- the bar's way back and the finger from the edge [go back](#going-back-and-forward) from the top screen as
  Alt+← does, so the screen hears it first: one with something unsaved that takes `on_history_back` stays, and a
  screen a finger dragged comes back;
- a `replace` swaps the top screen with no move.

```rust
use lurq::{
  app::ctx::Ctx,
  components::{NavigationBar, NavigationItem, StackOutlet, StackOutletProps, Text},
  node::Element,
  router::Routes,
};

fn routes() -> Routes {
  Routes::new().layout("/", shell, |routes| {
    routes
      .route("/", |ctx| {
        ctx.navigation_item(NavigationItem::new("Inbox"));
        Text::new("Inbox").into()
      })
      .route("/message/:id", |ctx| {
        ctx.navigation_item(NavigationItem::new("Message"));
        ctx.mount::<MessageScreen>(())
      })
  })
}

fn shell(ctx: &mut Ctx) -> Element {
  StackOutlet::mount_with(ctx, StackOutletProps::new().bar(NavigationBar::new()))
}
```

Give the history its first entry in `create` with `router.replace("/")`: it is the screen at the bottom of the
stack, the one the others go back to.

`StackOutlet::mount(ctx)` is the stack alone. With `StackOutletProps::bar` it draws a navigation bar over its
screens, under the safe area's top, 44 logical pixels high and 32 in a window lower than 480, as a phone held
on its side is: the top screen's title, and the way back with the title of the screen
under it. The titles move with the screens, and with the finger. `NavigationBar::new()` is a bar in the theme's
colors, light or dark as the theme is ([Colors from the theme](../ios/#colors-from-the-theme)); `background`,
`tint`, `title` and `border` set others.

A screen says what the bar shows for it by calling `ctx.navigation_item` in its `render`, or in its route:

| `NavigationItem` | |
|------------------|--|
| `new(title)` | The title in the middle of the bar |
| `back_title(text)` | What the way back says on the screen pushed over this one, in place of the title |
| `back_gesture(false)` | A finger from the edge does not take this screen back; the bar's way back still does |
| `leading(element)` | What is at the bar's leading end in place of the way back |
| `trailing(element)` | What is at the bar's trailing end while this screen is on top |
| `title_view(element)` | What stands in the middle of the bar in place of the title |
| `bar_hidden(true)` | No bar over this screen: it reaches the top of the outlet, under the status bar |
| `hides_tab_bar(true)` | The bar of tabs under the stack goes while this screen is the top one; see [Tabs](#tabs) |
| `large_title(true)` | The title large at the top of the content, scrolling away with it |
| `prompt(text)` | A line of small text over the title, for which the bar grows |
| `toolbar(element)` | A row of the screen's controls across the bottom of the outlet, in the bar's color |
| `search(NavigationSearch)` | A search field at the top of the content, under the title |
| `large_title_view(element)` | A title of the screen's own in the large title's place |
| `search_view(element)` | A search field of the screen's own in the search field's place |


### Content under the bar

The bar is drawn over the top of the screens, and its `background` is not all there: what is under it shows
through, blurred by `blur`. What is under it depends on the screen:

- **A screen whose content is in `StackOutlet::large_title_scroll(ctx, content)`** has a list that reaches the
  top of the outlet, under the bar, and the bottom, under a bar of tabs. The list keeps its content clear of
  both: the first row starts under the bar and the last ends over the tabs, and in between the rows scroll
  under the bars and show through them. It is the list to use for any screen that scrolls, with a large title
  or without.
- **Any other screen** is laid out under the bar, as if the bar took room: nothing of it is behind the bar.

A screen's toolbar is over the screen the same way, at the bottom: `ctx.safe_area().bottom` is as much more for
that screen, and its list goes on under the toolbar.

A bar with an opaque `background` hides what is under it, and `blur(0.0)` leaves it sharp.

A screen with a large title puts its content in `StackOutlet::large_title_scroll(ctx, content)`, which draws
the title over the content and scrolls the two as one. While the large title is in view the bar shows no title
and no color of its own; when the title has scrolled away the bar's title and color come in. Pulled down
past its top, the content takes the title with it, and the title grows a little. When a screen is pushed over
one whose large title is in view, that title travels to the new screen's way back, shrinking into its label, and
back again when the screen is taken off: one title, which takes the way back's color as it goes. Where the way
back says "Back" for want of room, the title fades as that word comes in.
`StackOutlet::screen_scroll(ctx)` gives that list's `ScrollState`.

```rust
ctx.navigation_item(NavigationItem::new("Inbox").large_title(true));
StackOutlet::large_title_scroll(ctx, messages)
```

A search field is drawn by `large_title_scroll` too, under the title, and scrolls away with the content. While it
is searched in, the field takes the bar's place and a large title goes, as the platform's search does;
`NavigationSearch::hides_bar(false)` leaves the bar and the title where they are.
`NavigationSearch::new(signal)` takes the signal the typed text is kept in, which the screen reads to choose
what it shows. While the field is typed into, and after while it holds text, it takes the bar's place: the bar
and a large title go, and beside the field comes a way to cancel, which clears the text and puts the keyboard
away.

```rust
ctx.navigation_item(
  NavigationItem::new("Inbox")
    .large_title(true)
    .search(NavigationSearch::new(self.sought.clone()).placeholder("Search mail")),
);
StackOutlet::large_title_scroll(ctx, messages_matching(&self.sought.get()))
```

A screen that draws its own title or its own field gives them with `large_title_view` and `search_view`:
`large_title_scroll` puts them where the outlet's would be, and they scroll away the same way. A field of the
screen's own says when it is searched in, for the bar and the title to give it their place:

```rust
if let Some(searching) = StackOutlet::searching(ctx) {
  searching.set(self.field_focused.get() || !self.sought.get().is_empty());
}
```

A screen with a toolbar ends above it: the toolbar stands over the home indicator's room, so the screen has no
bottom of the safe area to keep clear of.

For a bar that is the app's own altogether, give the outlet a builder with `StackOutletProps::bar_with`. It
is called with the state of the stack and returns the bar, which the outlet puts across its top:

```rust
let bar: NavigationBarBuilder = Arc::new(|state: &NavigationBarState| {
  Row::new()
    .padding_top(state.safe_area.top)
    .height(state.safe_area.top + 52.0)
    .child(
      Text::new(&state.top.item.title)
        // 0 in place, 1 gone: the title fades as its screen goes out, in step with it.
        .opacity(1.0 - state.out)
        .transition(Transition::opacity().duration(state.duration).easing(state.easing)),
    )
    .into()
});
StackOutlet::mount_with(ctx, StackOutletProps::new().bar_with(bar.clone()))
```

`state.top` is the screen in front and `state.under` the one behind it while they move, each with its
`NavigationItem`, what the way back from it says, whether its large title is still in view and whether it is
searching. `state.under_titles` names every screen under the top one, the nearest first, and `state.low` says
the window is too low for large titles. `state.out` is how far out of view the front screen is;
with a transition of `state.duration` and `state.easing` a part of the bar moves with the screens, and follows
a finger, for which the duration is zero. Make the builder once, in `create`: a new one at each render renders
the outlet again.

A screen comes in and goes out on a spring, `Easing::Spring`, and one let go by a finger goes on at the
finger's speed.

A screen of a stack reads its own entry: `ctx.route_params()` and `ctx.route_state()` give what the screen was
opened with, whatever was pushed over it since. `ctx.route_path()` is the router's current path on every
screen.

Everything the bar is drawn with is a field of `NavigationBar`, set over the defaults, which are the
platform's sizes:

```rust
NavigationBar {
  back: "Назад".to_owned(), // the word for going back, in the app's language
  back_menu: false,          // no list of the screens under on a long press
  height: 50.0,
  ..NavigationBar::new()
}
```

| `NavigationBar` | Default | |
|-----------------|---------|--|
| `background`, `tint`, `title`, `border` | `Background`, `Foreground`, `Foreground`, `Border` | The bar's colors, from the theme |
| `blur` | 40 | How much a screen's list is blurred where it shows through the bar, the toolbar and the list of screens under; see [Blur](../blur/) |
| `large_title_max_scale` | 1.5 | A large title is as much larger as the person asked text to be, up to this; the bar's own text keeps its size |
| `height`, `low_height`, `low_window` | 44, 32, 480 | The bar's height, and its height in a window lower than `low_window` |
| `title_size`, `title_weight` | 17, semibold | The title in the bar |
| `text_size` | 17 | The way back, a search field, the list of the screens under |
| `large_title_size`, `large_title_weight`, `large_title_height` | 34, bold, 52 | A large title |
| `inset` | 16 | How far a large title and a search field stand in from the sides |
| `back` | `Back` | What the way back says when the title under has no room, or there is none |
| `back_menu`, `back_menu_width` | on, 250 | The list a long press on the way back opens |
| `prompt_size`, `prompt_height` | 13, 22 | A prompt over the title |
| `toolbar_height` | 44 | A screen's toolbar |
| `search_height`, `search_background`, `search_placeholder` | 36, from `title` | A search field |

How the screens and the bar move is `StackMotion`, given with `StackOutletProps::motion`. Its defaults were
set by eye beside the platform's:

```rust
StackOutletProps::new().motion(StackMotion {
  duration: Duration::from_millis(400),
  ..StackMotion::default()
})
```

| `StackMotion` | Default | |
|---------------|---------|--|
| `duration` | 500 ms | How long a screen takes to come in or go out, on a spring |
| `parallax` | 0.3 | How far the screen under gives way, as a share of the width |
| `dim` | 0.1 | How dark the screen under gets |
| `back_share` | 0.5 | How far across a finger takes the top screen for it to go back |
| `back_speed` | 500 | The speed of the finger that takes it back from anywhere, in logical pixels a second |
| `title_fade` | 200 ms | How long the bar's title and color take to come and go under a large title |
| `large_title_growth` | 1.1 | How much larger a large title gets when pulled down |
| `large_title_pull` | 150 | The pull, in logical pixels, that takes it there |

A long press on the way back lists the screens under the top one, the nearest first; a tap on one goes back
to it at once, past the screens between (`navigator.back_by(steps)`).

In the bar, a title too long for it ends in an ellipsis; the way back says `Back` when the title of the screen
under has no room, and is its arrow alone when `Back` has none. A window lower than 480 logical pixels, a phone
on its side, has no large titles: the title is in the bar. What a screen gives as `leading`, `trailing` or
`title_view` is shown as the screen last rendered it.

`StackOutletProps::background` paints behind each screen; a screen with no background of its own shows the
one under it through while it moves.

## Links

`Link` reads the router navigator from context, so it must be rendered inside `Router::mount`.

```rust
Link::build(ctx, "Settings", "/settings")
```

For custom link contents, use `build_empty` and attach children:

```rust
Link::build_empty(ctx, "/settings")
  .child(Text::new("Open settings"))
```

Links navigate with `push`, so they add a history entry.

## Programmatic Navigation

Read the current navigator with `ctx.navigator()`.

```rust
if let Some(navigator) = ctx.navigator() {
  navigator.push("/profile");
  navigator.replace("/login");
  navigator.back();
  navigator.back_by(2); // two entries at once
  navigator.forward();
}
```

Use `push` for normal navigation and `replace` for redirects, login callbacks, initial route setup, or cases where the old path should not stay in history.

`back` and `forward` move the router alone. A button of the app's own that goes back as a person does runs the
command instead, `ctx.history_back()`: what is on the page hears it first ([below](#going-back-and-forward)).

The current path is reactive:

```rust
let path = ctx.route_path();
```

Reading it during render marks the component as dependent on route changes.

## Going back and forward

A mounted router is where going back ends: Alt+← (Cmd+[ on a Mac), the mouse's back button, a trackpad's swipe or a
finger from the left edge, Android's back, a browser's back, and `ctx.history_back()` all reach it as one command,
[`NavigationCommand`](../styling-events/#going-back-and-forward), with nothing to write. On its way the command
passes the elements from the focus (or from under the pointer) out to the router, and an element that handles
`on_history_back` takes it first; one that has nothing to do calls `event.pass_on()`.

```rust
use lurq::{app::events::HistoryEvent, components::Column};

// A form asks before its unsaved changes are left; saved, it lets the router go back.
Column::new()
  .on_history_back(move |event: HistoryEvent| {
    if unsaved.get_untracked() {
      asking.set(true);
    } else {
      event.pass_on();
    }
  })
  .child(form)
```

Which router: the one the command meets on its way, the innermost first, where it has an entry to go to; then the
others mounted in the window, the innermost first. A stack inside a tab goes back before the app's router around
it, and a router at its first entry passes the command to the one it is in. A router that is offstage (a tab not
shown) is not asked.

## The Address Of A Page

In a [browser](../web/) a router can be the one the page's address follows:

```rust
let router = ctx.router(routes());
router.replace("/");
router.follow_address();
```

From then on the two say the same:

- The address has the router's path as its fragment: `index.html#/settings`, with the path's query after it (`#/search?q=lurq`). A fragment changes without the browser asking the server for anything, so the page needs no rule on the server.
- A `push` is an entry of the browser's history as it is one of the router's, and a `replace` the same entry with another path. [Links](#links) push, so a page a person went to by a link is one the back button returns from.
- The browser's back and forward buttons [go back and forward](#going-back-and-forward) as Alt+← does: a dialog that takes going back closes, and the browser is put back where the router is; with nothing taking it the router moves to the browser's entry. `back`, `back_by` and `forward` move the browser. A jump of more than one entry (a long press on the browser's back) moves the router there at once.
- An address a person types into the bar or follows is navigated to as a `push` is, through the routes' [guards](#guards). A guard that redirects puts the path it sends to into the address; one that denies leaves the router where it is and the address says so.
- A page loaded with a path in its address starts there, in place of the path the router was given: a reload stays on the screen it was on, and a link to a screen opens it.

Call it once the router has its first path, which is where a page loaded without one starts. Only one router of an app follows the address, the one whose screens the address should name: the routers of [tabs](#tabs) keep histories of their own. Outside a browser a window has no address and `follow_address` does nothing, so the same code runs on a desktop.

The entries of the two histories are kept one for one while the page lives. A reload starts the router's history again with the one entry it is at, while the browser still has the ones before: going back to one of those starts the router's history again there, so `can_back()` is `false` on it though the browser's button goes on.

What a history entry carries as its state (`push_with_state`) is in memory and does not outlive a reload; what should, belongs in the path or its query.

## Params

Use `:name` in a route pattern to capture one path segment.

```rust
Routes::new().route("/users/:id", |ctx| ctx.mount::<UserPage>(()))
```

Inside the route component, read params with `ctx.route_params()`.

```rust
use lurq::{
  app::{component::Component, ctx::Ctx},
  components::Text,
  node::Element,
};

struct UserPage;

impl Component for UserPage {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let params = ctx.route_params();
    let id = params.get("id").unwrap_or("unknown");
    Text::new(&format!("User {id}"))
  }
}
```

`Params::get_parsed` parses directly into any type that implements `FromStr`.

```rust
let id: Option<u64> = ctx.route_params().get_parsed("id");
```

## Pattern Rules

Patterns are normalized around path segments:

| Pattern | Matches | Notes |
|---------|---------|-------|
| `/` | `/` | Root route |
| `/settings` | `/settings` | Static segments are case-sensitive |
| `/users/:id` | `/users/42` | Params capture one non-empty segment |
| `/files/*` | `/files/readme` | Wildcard matches one segment and does not capture |
| `/docs/**rest` | `/docs/a/b/c` | Catch-all captures remaining segments |

Specific routes win over broader routes. For example, `/users/settings` is preferred over `/users/:id` when both are present.

## Fallback Routes

Use `fallback` for unmatched paths.

```rust
Routes::new()
  .route("/", |_ctx| Text::new("Home").into())
  .fallback(|_ctx| Text::new("Not found").into())
```

Fallbacks can also be used inside a layout route so the shared shell remains visible for unknown child paths.

## Guards

Attach a guard to the route defined immediately before it.

```rust
use lurq::{
  components::Text,
  router::{GuardAction, Routes},
};

let authenticated = false;

let routes = Routes::new()
  .route("/login", |_ctx| Text::new("Login").into())
  .route("/account", |_ctx| Text::new("Account").into())
  .guard(move |_route| {
    if authenticated {
      GuardAction::Allow
    } else {
      GuardAction::Redirect("/login".to_owned())
    }
  });
```

Guards can:

- `Allow` the navigation
- `Deny` it and keep the current path
- `Redirect(path)` to another route

Use guards for coarse route access rules. Keep field validation and form submission logic inside components.

## Testing Router Flows

Router handles are cloneable, so tests can keep a copy and drive navigation directly.

```rust
let router = ctx.router(
  Routes::new()
    .route("/", |_ctx| Text::new("Home").into())
    .route("/about", |_ctx| Text::new("About").into()),
);

router.push("/about");
```

For link behavior, send pointer down/up through the tree. Click events are synthesized by the tree from matching pointer press and release events.

```rust
tree.mouse_down(x, y, MouseButton::Left);
tree.mouse_up(x, y, MouseButton::Left);
```

## Tabs

Added in **lurq 0.61.0**. `Tabs` is a set of places side by side, each a router with a history of its own, one
of them shown. A tab that is not shown stays as it was, with its screens, their state and their scroll: it is
kept out of layout, painting and input until it is shown again. `TabOutlet` shows the active one.

```rust
use lurq::components::{TabBar, TabItem, TabOutlet, TabOutletProps, Tabs};

fn create(ctx: &mut Ctx) -> Self {
  let inbox = ctx.router(inbox_routes());
  inbox.replace("/");
  let settings = ctx.router(settings_routes());
  settings.replace("/");
  let tabs = Tabs::new();
  tabs
    .add("inbox", TabItem::new("Inbox").symbol("tray").badge("3"), inbox)
    .add("settings", TabItem::new("Settings").symbol("gear"), settings);
  Self { tabs }
}

fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
  TabOutlet::mount(ctx, TabOutletProps::new(self.tabs.clone()).bar(TabBar::new()))
}
```

Each tab's routes are an ordinary router's: with a `StackOutlet` as their layout a tab is a stack of screens,
and going back moves in that tab alone.

### The set

| `Tabs` | |
| --- | --- |
| `add(key, item, router)`, `insert(index, ..)` | A tab, told from the others by its key. The first added is the active one |
| `remove(key)` | Takes a tab out with its history; the next one is shown if it was the active one |
| `move_to(key, index)` | Moves a tab to a place among the others, as taking it out and putting it back there does; it keeps its history, its screens and whether it is the one shown |
| `select(key)` | Shows a tab |
| `press(key)` | What a press on a tab's place does: shows it, or, for the active one, goes back to its first screen, and once there scrolls its page to the top |
| `active()`, `active_signal()` | The key of the tab that is shown |
| `list()` | The tabs in order, each with its item, its router and whether it is the active one |
| `router(key)`, `active_router()` | A tab's router: where a link opened from outside the tabs goes |
| `set_item(key, item)`, `set_badge(key, badge)` | Changes what a tab's place shows |
| `set_bar_hidden(hidden)` | Takes the outlet's bar out of view, for a screen that has the whole window |

A component inside the outlet gets the set with `ctx.use_context::<Tabs>()`.

### The bar

`TabOutletProps::bar(TabBar)` draws a bar to choose a tab with; without it the outlet shows the active tab and
nothing else, for an app that draws a strip of tabs of its own from `Tabs::list`.

A `TabItem` is a title and, over it, a picture: `icon` and `selected_icon` take any element; `symbol` and
`selected_symbol` name one of the platform's symbols, drawn in the bar's colors on iOS (with the `raster`
feature) and left out elsewhere. `badge` is a few characters at the picture's corner, and a dot when it has none. `search()` marks the tab
to search in: a floating bar stands it apart, a round place of its own at the bar's end.

| `TabBar` | Starts as | |
| --- | --- | --- |
| `place` | `Bottom` | `Bottom`, over the home indicator's room, or `Top`, under the status bar's |
| `background`, `border` | `Background`, `Border` | From the theme, light or dark as it is ([Colors from the theme](../ios/#colors-from-the-theme)) |
| `tint`, `color` | `Foreground`, `MutedForeground` | The active tab's picture and title, and the others' |
| `blur` | 40 | How much the tab's content is blurred where it shows through the bar; see [Blur](../blur/) |
| `height` | 49 | Without the room of the home indicator |
| `low_height`, `low_window`, `low_title_size`, `low_symbol_size` | 32, 480, 12, 13 | In a window lower than `low_window`, a phone on its side, the bar is lower and a title stands beside its picture |
| `title_size`, `title_weight` | 10, medium | |
| `symbol_size`, `spacing` | 18, 2 | |
| `badge_background`, `badge_color`, `badge_size` | `Destructive`, `DestructiveForeground`, 13 | |
| `item_max_width` | none | In a wider bar the places stand together in the middle |
| `hides_with_keyboard` | `true` | The bar goes while the on-screen keyboard is up |
| `hide_duration` | 250 ms | How long it takes to go and to come back |
| `contrast_color` | darker grey | The tabs that are not chosen, when the person asked for more contrast |
| `max_items`, `more_title`, `more_symbol` | 5, "More", `ellipsis` | With more tabs than places, the last place lists the rest: a press on one of them shows it |
| `minimizes_on_scroll` | `false` | The bar goes as a stack screen's list is scrolled down, and comes back as it is scrolled up |
| `floating` | `false` | A rounded bar that stands clear of the window's edges, with the chosen tab's place marked; `TabBar::new().floating()` |
| `floating_margin`, `floating_bottom`, `floating_height`, `floating_selection` | 21, 21, 62, grey | A floating bar's room at the sides and under it, its height and the mark behind the chosen tab |

The active tab's picture is its `selected_symbol`; without one, the filled one of its `symbol` where the
platform has it (`gear.fill` for `gear`).

A screen of a stack that has the whole window says so with `NavigationItem::hides_tab_bar(true)`: the bar goes
down as that screen comes in, comes back as it goes out, and follows a finger that drags the screen back.

`TabOutletProps::bar_with(place, builder)` draws a bar of the app's own from a `TabBarState`: the set, the
tabs in order and the window's safe area. A press on a place calls `Tabs::press`.

### Tabs a person opens, in a strip of the app's own

A desktop app with tabs as a browser has them draws its strip elsewhere than the outlet, in the title bar,
and mounts the outlet with no bar. The strip is drawn from the set (`Tabs::list`, each tab's
`router.path()` for what it is called), and what a person does to it is told to the set: `select`, `remove`,
`move_to` for a tab dragged along the strip. A tab opened while the app runs needs a router made outside a
render, where there is no `Ctx`: `RouterHandle::new(routes)`.

```rust
use lurq::{components::{TabItem, Tabs}, router::{RouterHandle, Routes}};

fn open(tabs: &Tabs, key: &str, routes: Routes, path: &str) {
  let router = RouterHandle::new(routes);
  router.replace(path);
  tabs.add(key, TabItem::new("Page"), router);
  tabs.select(key);
}
```

What is outside the outlet (a side bar, a search) goes where the tab that is shown is:
`tabs.active_router()`, read in a render with `tabs.active()` so that it is built again when another tab is
shown. The gallery (`crates/gallery/src/tabs.rs`) does this with the design system's
[`WindowTabs`](../design-system/#window-tabs) in a [`WindowChrome`](../design-system/#window-chrome).

### The safe area under a bar

A bar across the bottom is drawn over the tabs' content, which shows through it, blurred. `ctx.safe_area()`
says what the component that asks keeps clear of: the window's safe area, and at the bottom the bar's height
too while the bar is there. A screen pads by that and not by `ctx.window().safe_area`:

- content that does not scroll is padded at the bottom of the screen;
- a list is not padded itself: its content is, inside it, so that the rows go on under the bar and the last
  one ends over it.

For a screen that hides the bar, and while the keyboard is up, the bottom is the window's alone.

## The screen in front

A `StackOutlet` keeps the screens under the top one, and a `TabOutlet` the tabs that are not shown: they stay as
they were, and what plays in them (a video, a sound) would play on, heard, under a screen pushed over it or in a
tab nobody sees. `ctx.is_in_front()` says whether the screen a component is in is the one in front. Read in
`render`, by the screen or by anything mounted inside it at any depth, it renders the component again when it
changes:

```rust
fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
  let playing = self.playing.clone();
  // Plays while the person wants it to and its screen is in front, and is where it was when it is again.
  let plays = playing.get() && ctx.is_in_front();
  VideoPlayer::new()
    .source(&self.source)
    .playing_controlled(plays, move |to| playing.set(to))
    .mount(ctx)
}
```

- **In front** is the top screen of its stack: the one at the end of the stack's history. Where the stack is in a
  set of tabs, it is in front only while its tab is the one shown; a stack in a stack, only while the screen that
  holds it is. A component in neither a stack nor a set of tabs is in front.
- **A screen pushed** is in front from the moment it is pushed, and the one under it is not, though both are seen
  while the new one comes in. **Going back**, the screen under is in front from the moment the stack goes back, and
  the one that leaves is not.
- **While a finger from the edge drags the top screen aside**, the top screen is still the one in front, and the
  one seen under it is not. It is from the moment the finger lets go far enough for the stack to go back; let go
  short of that, nothing changes and nobody is told.
- **A screen that leaves the stage is told.** A component that is not shown is not rendered ([offstage
  components](../ctx/#mounting-with-slot-children)): a tab that is no longer shown, a screen two under the top. One that
  leaves as what it reads changes is rendered once more first, so a screen hears that it is no longer in front,
  and the player in it is told to pause, in the pass that takes it away.
- A sheet, a dialog or another window over a screen does not change it: `ctx.window().info().is_focused` says whether
  the window is the one a person works in.
- **lurq's players do not stop by themselves** where their screen is not in front: a music app plays on in another
  tab. The app says, as above.
