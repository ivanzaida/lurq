pub mod background_color;
pub mod border;
pub mod checkbox_style;
pub mod color;
pub mod cursor;
pub mod dimension;
pub(crate) mod dsl;
pub mod element;
pub mod interaction_state;
pub(crate) mod layout_cache;
pub(crate) mod node;
pub(crate) mod node_kind;
pub mod padding;
pub mod radius_value;
pub mod slider_style;
pub mod spacing_value;
pub mod style;
pub(crate) mod text_selection;
pub mod transform;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextTransformMode {
  #[default]
  Bitmap,
  Rasterized,
}

pub use background_color::BackgroundColor;
pub use checkbox_style::CheckboxStyle;
pub use cursor::CursorIcon;
pub use element::{Element, ElementChildren, ElementRef};
pub use node::BackgroundSize;
pub(crate) use node::Node;
pub use radius_value::RadiusValue;
pub use slider_style::SliderPartStyle;
pub use spacing_value::SpacingValue;
pub use style::Style;
