---
title: Ctx
description: Per-component render context API for state, children, contexts, refs, and effects.
---

# Ctx

## Overview

`Ctx` is the per-component render context. It is passed to `Component::create` and `Component::render`.

Use it to:

- create reactive state owned by the component
- mount child components
- pass context values down the tree
- read slot children supplied by a parent
- create element refs and interaction state
- register effects, watchers, keyed list slots, and error boundaries

```rust
use lurq::{
  app::{component::Component, ctx::Ctx},
  core::Signal,
  node::Element,
};

struct Counter {
  count: Signal<i32>,
}

impl Component for Counter {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self { count: ctx.signal(0) }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    let count = self.count.clone();
    lurq::components::Text::new(&format!("Count: {}", self.count.get()))
      .on_click(move |_| count.update(|n| *n += 1))
  }
}
```

## Dirty State

```rust
ctx.is_dirty();
```

`is_dirty` reports whether this component context is marked dirty. Runtime uses this internally to decide whether a component subtree needs to render again.

Application code usually does not need to call it.

## Props

```rust
let props = ctx.props::<Self::Props>();
```

`props` returns the current props for the component that owns this context. Component props must implement `PartialEq`; reused child components rerender when the incoming props differ from the props stored on their context.

## Manual Root Contexts

```rust
let mut ctx = Ctx::new_root();
```

`new_root` creates a standalone root context. Runtime normally creates and owns root contexts for mounted components, so application code rarely needs this directly. It is useful for tests and low-level component mounting.

Standalone contexts do not have a runtime theme unless one is attached internally by `Tree`.

## Signals

```rust
let count = ctx.signal(0);

count.get();
count.set(1);
count.update(|n| *n += 1);
```

`ctx.signal(initial)` creates a `Signal<T>` and wires it to the current component. When the signal changes, the component context is marked dirty.

Use `Signal` for state that should trigger a render when it changes.

With the `devtools` feature enabled, `T` must implement `DevtoolsInspectable` so DevTools can show signal values. Without `devtools`, `Signal<T>` has no debug-inspection bound.

## Batch Updates

```rust
ctx.batch(|| {
  count.set(1);
  count.set(2);
});
```

`batch` defers dirty marking for contexts in the same component tree until the closure completes.

## Stores And Lenses

```rust
#[derive(Clone)]
struct User {
  name: String,
  age: u32,
}

let user = ctx.store(User { name: "Ada".into(), age: 36 });
let name = user.lens(
  |user| user.name.clone(),
  |user, name| user.name = name,
);

name.set("Grace".into());
```

`ctx.store(initial)` creates structured reactive state. Like signals, store updates mark the owning component dirty.

Use lenses when child code should read or update one field without taking ownership of the whole store value.

## Memos

```rust
let count = ctx.signal(0);
let doubled = ctx.memo({
  let count = count.clone();
  move || count.get() * 2
});

let value = doubled.get();
```

`ctx.memo(f)` creates a derived value. The memo tracks reactive reads inside `f` and recomputes when those dependencies change.

## Refs

```rust
let latest_id = ctx.create_ref::<Option<u64>>(None);
latest_id.set(Some(42));
```

`create_ref` creates persistent non-reactive state. Updating a ref does not mark the component dirty.

Use refs for handles, cached values, counters, or other state that should survive renders but should not cause renders.

## Effects

```rust
let count = ctx.signal(0);

ctx.on_effect({
  let count = count.clone();
  move || println!("count = {}", count.get())
});
```

`on_effect` runs immediately and reruns when any tracked reactive value read inside the effect changes.

Effects are retained by the context, so they live as long as the component context lives.

## Watchers

```rust
let count = ctx.signal(0);

ctx.watch(&count, |value| {
  println!("count changed to {value}");
});
```

`watch` subscribes to a specific signal and keeps the subscription alive for the context lifetime.

Use `watch` when you want an explicit callback for one signal instead of automatic dependency tracking.

## Context Values

### Static Context

```rust
#[derive(Clone)]
struct Locale(String);

ctx.provide(Locale("en-US".into()));
```

Descendants can read the value by type:

```rust
if let Some(locale) = ctx.use_context::<Locale>() {
  println!("locale = {}", locale.0);
}
```

`provide` stores a cloned value by type. `use_context` returns `None` if no ancestor provided that type.

### Reactive Context

```rust
let theme_name = ctx.create_context("light".to_string());
theme_name.set("dark".to_string());
```

Descendants can consume it:

```rust
let theme_name = ctx.consume_context::<String>().unwrap();
let current = theme_name.get();
```

`create_context` stores a `ReactiveContext<T>` and subscribes the creating context to changes. `consume_context` retrieves the reactive context and subscribes the consuming context to changes.

`ReactiveContext<T>` requires `T: Clone + Hash + Send + Sync + 'static` so it can detect meaningful value changes.

## Theme

`theme()` returns the current runtime theme. Root and child contexts get the theme from `Tree::mount_root`.

Theme typography exposes strict named text styles. `Text::new` uses `theme.typography().body`, and `Text::new("Label").variant(TypographyStyle::Label)` resolves the named style during layout.
See [Theme](./theme/) for the full palette, typography, radius, spacing, and form role tables.

```rust
use lurq::{
  app::theme::{PaletteColor, SpacingSize, TypographyStyle},
  layout::text_style::TextStyle,
  node::color::Color,
};

ctx.theme().set_palette_color(PaletteColor::Accent, Color::from_hex("#2563eb"));
ctx.theme().set_spacing_value(SpacingSize::Sm, 8.0);

ctx.theme().set_typography_style(TypographyStyle::Body, TextStyle {
  font_size: 16.0,
  ..TextStyle::default()
});

ctx.theme().set_typography_style(
  TypographyStyle::Label,
  TextStyle {
    font_size: 13.0,
    ..TextStyle::default()
  },
);
```

Use strict theme keys with APIs such as `.background(PaletteColor::Accent)`, `Text::new("Label").variant(TypographyStyle::Label)`, `.rounded(RadiusSize::Md)`, `.spacing(SpacingSize::Sm)`, and `.padding(SpacingSize::Md)`. The lookup methods `palette_color`, `typography_style`, `radius_value`, and `spacing_value` are also available when component code needs the concrete value.

Components that call `ctx.theme()` during render subscribe to theme version changes. Mutating that theme rerenders those subscriber components on the next pass. Use `theme.lens(getter, setter)` when component code needs a focused handle for one theme value:

```rust
let brand = ctx.theme().lens(
  |theme| theme.palette_color(PaletteColor::Accent),
  move |theme, color| theme.set_palette_color(PaletteColor::Accent, color),
);

brand.set(Color::from_hex("#2563eb"));
```

Only call `theme()` from a context managed by runtime. A manually-created root context without a theme will panic.

## Window

```rust
let window = ctx.window();

let logical = window.logical_size();
let minimized = window.is_minimized;
let full_screen = window.is_full_screen;
let decorated = window.is_decorated;
```

`ctx.window()` returns a reactive window handle. Reading it subscribes the component to window resize, move, scale, minimized, fullscreen, and decoration-state changes.

The handle dereferences to `WindowInfo`, so geometry helpers such as `position()`, `resolved_size()`, `logical_size()`, `logical_width()`, and `logical_height()` are available directly.

Window commands are queued and applied by the active platform shell:

```rust
let window = ctx.window();

window.close();
window.set_minimized(true);
window.set_full_screen(true);
window.set_decorations(false);
window.set_title_bar_color(lurq::node::color::Color::from_hex("#101215"));
window.set_icon(lurq::app::WindowIcon::from_rgba(vec![255, 0, 0, 255], 1, 1));
window.set_corner_radius(lurq::app::WindowCornerRadius::RoundedSmall);
window.resize(1280, 720);
window.move_to(120, 80);
window.start_drag();
window.start_resize(WindowResizeDirection::SouthEast);
window.stop_drag();
```

Use `set_decorations(false)` or `set_decorated(false)` for a custom title bar. Rust reserves `move` as a keyword, so direct move calls use `window.r#move(x, y)`; `move_to(x, y)` is provided for normal method syntax.

`set_icon` accepts a `WindowIcon` built from RGBA pixels. `set_title_bar_color` and `set_corner_radius` customize native window chrome where the platform supports it; with the winit shell, title bar color maps to the Windows title background API, while corner radius maps to the Windows corner preference API and macOS AppKit content-view layer clipping. Unsupported platforms no-op. Use `clear_icon()`, `clear_title_bar_color()`, and `reset_corner_radius()` to return those settings to the platform default.

For custom title bars, call `window.start_drag()` from the press handler for the draggable region. Call `window.start_resize(direction)` from the press handler for custom edge or corner resize handles. The active platform shell uses its native window drag and resize APIs when available. `window.stop_drag()` ends custom drag state for shells that need it; on winit, the OS ends native drag and resize operations automatically on release.

```rust
use lurq::{
  app::events::MouseButton,
  components::{Stack, Text},
};

let window = ctx.window();

Stack::new()
  .height(36.0)
  .child(Text::new("My App").padding_horizontal(12.0))
  .on_mouse_down(move |event| {
    if event.button == MouseButton::Left {
      window.start_drag();
    }
  })
```

Native resize should be started immediately after the left mouse button press. For an undecorated window, render small edge and corner hit zones and call the matching direction:

```rust
use lurq::{
  app::{events::MouseButton, WindowResizeDirection},
  components::Rect,
  node::CursorIcon,
};

let window = ctx.window();

Rect::new(8.0, 8.0)
  .cursor(CursorIcon::NwseResize)
  .on_mouse_down(move |event| {
    if event.button == MouseButton::Left {
      window.start_resize(WindowResizeDirection::SouthEast);
    }
  })
```

## Slot Children

Parents pass slot children with `mount_with` or `mount_keyed_with`:

```rust
ctx.mount_with::<Panel>(PanelProps { title: "Info" }, vec![
  lurq::components::Text::new("Panel body"),
])
```

The child component reads them through its own context:

```rust
fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
  let child_count = ctx.children().len();

  lurq::components::Column::new()
    .child(lurq::components::Text::new(&self.title))
    .child(lurq::components::Text::new(&format!("{child_count} slot children")))
}
```

```rust
ctx.has_children();
ctx.children();
```

`children()` returns an empty slice when no slot children were provided.

## Element Refs

```rust
let element_ref = ctx.element_ref();

lurq::components::Rect::new(100.0, 40.0)
  .ref_element(element_ref.clone())
```

After layout, the ref exposes the element rect:

```rust
let (x, y, width, height) = element_ref.rect();
let attached = element_ref.is_attached();
let hovered = element_ref.hovered();
let active = element_ref.active();
let focused = element_ref.focused();
```

Use element refs when code outside normal layout traversal needs an element's measured rect or current interaction flags.

Element refs can also scope outside-click hooks:

```rust
let panel_ref = ctx.element_ref();
ctx.on_click_outside(panel_ref.clone(), |_| {
  println!("clicked outside the panel");
});

lurq::components::Rect::new(240.0, 160.0)
  .ref_element(panel_ref)
```

`on_click_outside` fires on left clicks whose pointer position is outside the referenced element's measured bounds. The hook is render-scoped: if the component stops calling it, the listener is removed on the next render.

Mutable element refs use the same handle type as `Tree::find_element_mut`:

```rust
let element_ref = ctx.element_ref_mut();

lurq::components::Rect::new(100.0, 40.0)
  .ref_element(element_ref.clone())

element_ref.set_relative_bounds(15.0, 20.0, 120.0, 60.0);
```

## Interaction State

```rust
let state = ctx.interaction();

lurq::components::Rect::new(100.0, 40.0)
  .interactive(state.clone())
  .on_mouse_enter(|| println!("hover"))
```

`InteractionState` tracks runtime interaction flags:

```rust
state.is_hovered();
state.is_active();
state.is_focused();
```

Hover, active, and focus are updated by runtime input dispatch.

## Mounting Child Components

```rust
ctx.mount::<Counter>(());
ctx.mount_keyed::<TodoItem>(todo.id.as_str(), todo.clone());
```

- `mount` matches children by slot position and component type.
- `mount_keyed` matches by slot position, key, and component type.
- Matching children reuse the existing component instance and context.
- Non-matching children are unmounted and replaced.

Use keyed mounts for dynamic lists where identity matters.

## Mounting With Slot Children

```rust
ctx.mount_with::<Panel>(props, vec![lurq::components::Text::new("body")]);
ctx.mount_keyed_with::<Panel>("settings", props, vec![lurq::components::Text::new("body")]);
```

These work like `mount` and `mount_keyed`, but pass slot children into the child context.

## Keyed List Helper

```rust
let elements = ctx.for_each(
  self.items.get(),
  |item| item.id,
  |_ctx, item| {
    lurq::components::Row::new()
      .child(lurq::components::Text::new(&item.title))
  },
);

lurq::components::Column::new().with_children(elements)
```

`for_each` creates keyed child contexts for arbitrary render closures, not just `Component` implementations. It is useful when each list item needs its own local context for child mounts, effects, or refs.

## Error Boundary

```rust
ctx.error_boundary(
  |ctx| risky_component(ctx),
  || lurq::components::Text::new("Something went wrong"),
)
```

`error_boundary` catches panics from the component closure and returns the fallback element instead.

## Timers

```rust
use std::time::Duration;

let timeout = ctx.create_timeout(Duration::from_secs(2), || { /* fires once */ });
timeout.start();

let interval = ctx.create_interval(Duration::from_millis(500), || { /* fires repeatedly */ });
interval.start();
```

`Timeout` has `.start()`, `.restart()`, `.cancel()`, and `.is_active()`. `Interval` has `.start()`, `.restart()`, `.stop()`, and `.is_active()`. Create timers in `Component::create` and store them in the struct.

See [Futures And Timers](./futures-timers/) for full details.

## Futures

```rust
let handle = ctx.future(deps, |deps| async move {
  Ok::<_, String>("result".to_owned())
});
let state = handle.state().get();
```

`ctx.future` runs an async operation that restarts when `deps` changes. Returns a `FutureHandle` with a reactive `Signal<FutureState<T, E>>`.

```rust
let action = ctx.future_action(|args: String| async move {
  Ok::<_, String>(args)
});
action.run("go".to_owned());
```

`ctx.future_action` creates a future that only runs when `.run(args)` is called.

See [Futures And Timers](./futures-timers/) for full details.

## Forms

Requires the `form` feature.

```rust
let form = ctx
  .form(FormOptions::new().field("user", "Ada"))
  .on_submit(|values| { /* handle submission */ });
```

Returns a `FormHandle` that owns field signals and a submit callback. See [Forms](./forms/) for full details.

## Routing

Requires the `router` feature.

```rust
let router = ctx.router(Routes::new().route("/", |_ctx| Element::new()));
let navigator = ctx.navigator();
let path = ctx.route_path();
let params = ctx.route_params();
```

`ctx.router` creates a `RouterHandle` from a route table. `ctx.navigator` reads the current router navigator from context, and `route_path` / `route_params` expose the current match during render.

See [Routing](./routing/) for route definitions, layouts, links, guards, and testing patterns.

## Internationalization

Requires the `i18n` feature.

```rust
let label = ctx.t("hello");
let greeting = ctx.t_args("welcome", [("name", "Ada")]);
let ns_label = ctx.t_ns("errors", "not_found");
let i18n = ctx.i18n();
```

Translation lookups are reactive — components re-render when the locale changes. See [Internationalization](./i18n/) for full details.

## Modals

```rust
lurq::components::Modal::new(lurq::components::Text::new("Modal content"))
  .open(self.open.clone())
  .target(lurq::components::Root);
```

Modals are render-flow components. Use `Modal::new(...).open(signal)` and choose a target with `Parent`, `Root`, or an `ElementRef`. See [Modals](./modals/) for full details.

## Render Lifecycle Methods

```rust
ctx.begin_render();
```

`begin_render` resets the child cursor before rendering children. Runtime and the component mounting internals call this as part of normal rendering.

Application components normally should not call render lifecycle methods directly.
