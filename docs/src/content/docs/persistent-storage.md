---
title: Persistent Storage
description: Store small typed values across app launches with the persistent_storage feature.
---

# Persistent Storage

Persistent storage is an app-level key/value store for small values that should survive process restarts, such as preferences, counters, selected tabs, recent filters, and simple UI state.

Enable it with the `persistent_storage` feature:

```toml
lurq = { version = "0.14.0", features = ["persistent_storage"] }
```

The feature uses `redb` as the file-backed store. Values are stored as typed bytes, not JSON.

## Configure A Storage File

By default, `App::new()` uses in-memory storage. Configure a file path before mounting or running the app when values should persist across launches:

```rust
let mut app = lurq::app::App::new();

app.set_persistent_storage_path(std::path::PathBuf::from("app-state.redb"))?;
```

The parent directory is created when needed.

## Read And Write Values

Inside components, use `Ctx`:

```rust
use lurq::{
  app::{component::Component, ctx::Ctx},
  node::Element,
};

struct Preferences;

impl Component for Preferences {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    ctx.set_persistent_value("sidebar_open", true).unwrap();
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let sidebar_open = ctx.persistent_value::<bool>("sidebar_open").unwrap_or(false);

    lurq::components::Text::new(&format!("Sidebar open: {sidebar_open}"))
  }
}
```

`ctx.persistent_value::<T>(key)` returns `Option<T>`.

`ctx.set_persistent_value(key, value)` returns `Result<(), PersistentStorageError>`.

## Supported Types

Persistent values are generic over supported scalar types:

```rust
ctx.set_persistent_value("name", "Ada")?;
ctx.set_persistent_value("launch_count", 12_u64)?;
ctx.set_persistent_value("compact", false)?;
ctx.set_persistent_value("zoom", 1.25_f32)?;

let name = ctx.persistent_value::<String>("name");
let launch_count = ctx.persistent_value::<u64>("launch_count");
let compact = ctx.persistent_value::<bool>("compact");
let zoom = ctx.persistent_value::<f32>("zoom");
```

Supported values include:

- `bool`
- `String`
- `&str` for writes
- `char`
- signed and unsigned integer types
- `usize` and `isize`
- `f32` and `f64`

If a key exists but was stored with a different type, reads return `None`.

## App-Level Access

The same API is available on `App`:

```rust
let app = lurq::app::App::new();

app.set_persistent_value("theme", "dark")?;

let theme = app.persistent_value::<String>("theme");
```

Use app-level access for setup code, tests, and non-component runtime logic. Use `Ctx` access inside components.

## Behavior

Persistent storage is shared by the app. It is not component-local state, and reading from it does not subscribe the component to changes.

Use signals or stores for reactive state:

```rust
let tab = ctx.signal(ctx.persistent_value::<String>("tab").unwrap_or("home".into()));
```

Then write back to persistent storage when the value should be saved.

## Removing Values

The lower-level storage handle exposes removal:

```rust
ctx.app_ref()
  .persistent_storage()
  .remove_value("sidebar_open")?;
```

Use removal when a missing value should fall back to the app default.
