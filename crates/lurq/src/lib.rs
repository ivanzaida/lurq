extern crate self as lurq;

pub use lurq_macros::DevtoolsInspectable;

pub mod animation;
pub mod app;
#[cfg(feature = "clipboard")]
pub mod clipboard;
pub mod components;
pub mod core;
#[cfg(feature = "image")]
pub mod images;
pub mod layout;
#[cfg(feature = "markdown")]
pub mod markdown;
pub mod node;
#[cfg(feature = "persistent_storage")]
pub mod persistent_storage;
#[cfg(feature = "render")]
pub(crate) mod render;
pub mod responsive;
#[cfg(feature = "svg")]
pub mod svg;

#[cfg(feature = "resources")]
pub mod resources;
#[cfg(feature = "router")]
pub mod router;
