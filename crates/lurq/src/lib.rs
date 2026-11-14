extern crate self as lurq;

pub use lurq_macros::DevtoolsInspectable;
#[cfg(feature = "persistent_storage")]
pub use lurq_macros::PersistentValue;
#[cfg(feature = "query")]
pub use lurq_macros::query;

pub mod animation;
pub mod app;
#[cfg(feature = "canvas")]
pub mod canvas;
#[cfg(feature = "clipboard")]
pub mod clipboard;
pub mod components;
pub mod core;
pub mod files;
#[cfg(feature = "raster")]
pub mod images;
pub mod layout;
#[cfg(feature = "markdown")]
pub mod markdown;
pub mod node;
#[cfg(feature = "persistent_storage")]
pub mod persistent_storage;
#[cfg(feature = "query")]
pub mod query;
#[cfg(feature = "render")]
pub(crate) mod render;
pub mod responsive;
#[cfg(test)]
mod size_budget;
#[cfg(feature = "svg")]
pub mod svg;
pub mod ui;

/// The clock lurq reads, chosen by the platform: std's everywhere but in a
/// browser, where std's `Instant::now` and `SystemTime::now` panic and the
/// page's clock is read instead. An app that runs in a browser too reads the
/// time through these.
pub mod time {
  #[cfg(not(target_arch = "wasm32"))]
  pub use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

  #[cfg(target_arch = "wasm32")]
  pub use web_time::{Duration, Instant, SystemTime, UNIX_EPOCH};
}

/// The version of lurq: what an app shows in an about box, and the gallery in its header.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(feature = "mcp")]
pub mod mcp;
#[cfg(feature = "resources")]
pub mod resources;
#[cfg(feature = "router")]
pub mod router;
