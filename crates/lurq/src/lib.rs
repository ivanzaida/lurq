extern crate self as lurq;

pub use lurq_macros::DevtoolsInspectable;

/// What lurq's macros reach from the crates they are expanded in. Not for an app to name.
#[doc(hidden)]
pub mod __private {
  #[cfg(feature = "devtools")]
  pub use inventory;
}

/// Registers the inspector of each type, so that DevTools reads a value of it wherever it is given one with no
/// bound on its type: what a context holds. `#[derive(DevtoolsInspectable)]` does this for a type without type
/// parameters; a type whose `DevtoolsInspectable` is written by hand is registered here. Without the `devtools`
/// feature it does nothing.
///
/// ```
/// #[derive(Clone, Debug)]
/// struct Depth(usize);
///
/// impl lurq::app::component::DevtoolsInspectable for Depth {
///   fn inspect(&self, formatter: &mut lurq::app::component::DevtoolsFormatter<'_>) {
///     formatter.value_debug(self);
///   }
/// }
///
/// lurq::register_inspectable!(Depth);
/// ```
#[cfg(feature = "devtools")]
#[macro_export]
macro_rules! register_inspectable {
  ($($ty:ty),+ $(,)?) => {
    $(
      $crate::__private::inventory::submit! {
        $crate::app::component::RegisteredInspector {
          type_id: || ::std::any::TypeId::of::<$ty>(),
          inspect: |value, buffer| {
            if let ::std::option::Option::Some(value) = value.downcast_ref::<$ty>() {
              $crate::app::component::DevtoolsInspectable::write_info(value, buffer);
            }
          },
        }
      }
    )+
  };
}

/// Registers the inspector of each type for DevTools: nothing without the `devtools` feature.
#[cfg(not(feature = "devtools"))]
#[macro_export]
macro_rules! register_inspectable {
  ($($ty:ty),+ $(,)?) => {};
}
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
pub mod code;
pub mod components;
pub mod core;
pub mod eyedropper;
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
pub mod resources;
#[cfg(feature = "router")]
pub mod router;
