pub mod background_color;
pub mod border;
pub mod border_size_value;
pub mod checkbox_style;
pub mod color;
pub mod cursor;
pub mod dimension;
pub(crate) mod dsl;
pub mod element;
pub mod gradient;
pub mod interaction_state;
pub(crate) mod layout_cache;
pub(crate) mod node;
pub(crate) mod node_kind;
pub mod padding;
pub mod radius_value;
pub mod select_style;
pub mod slider_style;
pub mod spacing_value;
pub mod style;
pub mod text_color;
pub(crate) mod text_selection;
pub mod transform;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextTransformMode {
  #[default]
  Bitmap,
  Rasterized,
}

pub use background_color::BackgroundColor;
pub use border_size_value::BorderSizeValue;
pub use checkbox_style::CheckboxStyle;
pub use cursor::CursorIcon;
pub use element::{Element, ElementChildren, ElementRef};
pub use gradient::{Gradient, GradientKind, GradientStop};
#[cfg(feature = "form")]
pub use node::FormData;
pub use node::{BackgroundSize, ButtonKind};
pub(crate) use node::{Node, NodeUpdate};
pub use radius_value::RadiusValue;
pub use select_style::{SelectPartStyle, SelectStyle};
pub use slider_style::SliderPartStyle;
pub use spacing_value::SpacingValue;
pub use style::Style;
pub use text_color::TextColor;
