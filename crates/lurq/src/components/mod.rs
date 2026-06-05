mod button;
mod checkbox;
mod column;
mod drag_container;
mod draggable;
mod drop_zone;
#[cfg(feature = "form")]
mod form;
#[cfg(feature = "image")]
mod image;
#[cfg(feature = "router")]
mod link;
#[cfg(feature = "router")]
mod outlet;
mod rect;
#[cfg(feature = "router")]
mod router_component;
mod row;
mod scroll;
mod slider;
mod slot;
mod spacer;
mod stack;
#[cfg(feature = "svg")]
mod svg;
mod text;
mod text_input;

pub use button::Button;
pub use checkbox::Checkbox;
pub use column::Column;
pub use drag_container::{DragBounds, DragContainer, DragContainerProps};
pub use draggable::{Draggable, DraggableProps, DropMissBehavior};
pub use drop_zone::{DropZone, DropZoneProps};
#[cfg(feature = "form")]
pub(crate) use form::FormContext;
#[cfg(feature = "form")]
pub use form::{
  Control, ControlOptions, ControlState, ErrorVisibility, Form, FormCheckboxInput, FormCheckboxInputProps,
  FormControlField, FormData, FormErrors, FormField, FormFieldProps, FormHandle, FormOptions, FormPrimaryButton,
  FormPrimaryButtonProps, FormProps, FormSecondaryButton, FormSecondaryButtonProps, FormSliderInput,
  FormSliderInputProps, FormTextInput, FormTextInputProps, FormValue, FormValues, ResolvedControl, ValidationResult,
  validators,
};
#[cfg(feature = "image")]
pub use image::Image;
#[cfg(feature = "router")]
pub use link::Link;
#[cfg(feature = "router")]
pub use outlet::Outlet;
pub use rect::Rect;
#[cfg(feature = "router")]
pub use router_component::{Router, RouterProps};
pub use row::Row;
pub use scroll::{ScrollBoth, ScrollHorizontal, ScrollVertical};
pub use slider::Slider;
pub use slot::Slot;
pub use spacer::Spacer;
pub use stack::Stack;
#[cfg(feature = "svg")]
pub use svg::Svg;
pub use text::Text;
pub use text_input::{TextInput, TextInputOverflow};

#[macro_export]
macro_rules! impl_into_node {
  ($struct_name:ident) => {
    pub struct $struct_name {
      pub(crate) node: $crate::node::Node,
    }

    impl $struct_name {
      pub(crate) fn from_node(node: $crate::node::Node) -> Self {
        Self {
          node: node.with_tag_name(Self::tag_name()),
        }
      }

      pub(crate) fn tag_name() -> std::sync::Arc<str> {
        std::sync::Arc::from(stringify!($struct_name))
      }

      pub fn node_id(&self) -> $crate::core::NodeId {
        self.node.node_id()
      }

      pub fn background(mut self, color: impl Into<$crate::node::BackgroundColor>) -> Self {
        self.node = self.node.background(color);
        self
      }

      pub fn size(
        mut self,
        width: impl Into<$crate::node::dimension::Dimension>,
        height: impl Into<$crate::node::dimension::Dimension>,
      ) -> Self {
        self.node = self.node.size(width, height);
        self
      }

      pub fn width(mut self, width: impl Into<$crate::node::dimension::Dimension>) -> Self {
        self.node = self.node.width(width);
        self
      }

      pub fn height(mut self, height: impl Into<$crate::node::dimension::Dimension>) -> Self {
        self.node = self.node.height(height);
        self
      }

      pub fn padding_left(mut self, val: impl Into<$crate::node::SpacingValue>) -> Self {
        self.node = self.node.padding_left(val);
        self
      }

      pub fn padding_right(mut self, val: impl Into<$crate::node::SpacingValue>) -> Self {
        self.node = self.node.padding_right(val);
        self
      }

      pub fn padding_top(mut self, val: impl Into<$crate::node::SpacingValue>) -> Self {
        self.node = self.node.padding_top(val);
        self
      }

      pub fn padding_bottom(mut self, val: impl Into<$crate::node::SpacingValue>) -> Self {
        self.node = self.node.padding_bottom(val);
        self
      }

      pub fn padding_horizontal(mut self, val: impl Into<$crate::node::SpacingValue>) -> Self {
        self.node = self.node.padding_horizontal(val);
        self
      }

      pub fn padding_vertical(mut self, val: impl Into<$crate::node::SpacingValue>) -> Self {
        self.node = self.node.padding_vertical(val);
        self
      }

      pub fn padding(mut self, padding: impl Into<$crate::node::padding::Padding>) -> Self {
        self.node = self.node.padding(padding);
        self
      }

      pub fn padding_custom(mut self, padding: $crate::node::padding::Padding) -> Self {
        self.node = self.node.padding_custom(padding);
        self
      }

      pub fn frame(mut self, frame: $crate::layout::layout_kind::FrameConstraints) -> Self {
        self.node = self.node.frame(frame);
        self
      }

      pub fn offset(mut self, x: f32, y: f32) -> Self {
        self.node = self.node.offset(x, y);
        self
      }

      pub fn relative(mut self, x: f32, y: f32) -> Self {
        self.node = self.node.relative(x, y);
        self
      }

      pub fn absolute(
        mut self,
        x: f32,
        y: f32,
        width: impl Into<$crate::node::dimension::Dimension>,
        height: impl Into<$crate::node::dimension::Dimension>,
      ) -> Self {
        self.node = self.node.absolute(x, y, width, height);
        self
      }

      pub fn absolute_position(mut self, x: f32, y: f32) -> Self {
        self.node = self.node.absolute_position(x, y);
        self
      }

      pub fn align(mut self, alignment: $crate::layout::Alignment) -> Self {
        self.node = self.node.align(alignment);
        self
      }

      pub fn flex(mut self, factor: f32) -> Self {
        self.node = self.node.flex(factor);
        self
      }

      pub fn flex_shrink(mut self, factor: f32) -> Self {
        self.node = self.node.flex_shrink(factor);
        self
      }

      pub fn flex_full(mut self, grow: f32, shrink: f32, basis: Option<f32>) -> Self {
        self.node = self.node.flex_full(grow, shrink, basis);
        self
      }

      pub fn corner_radius(mut self, radius: impl Into<$crate::node::RadiusValue>) -> Self {
        self.node = self.node.corner_radius(radius);
        self
      }

      pub fn corner_radius_custom(mut self, radius: $crate::node::border::BorderRadius) -> Self {
        self.node = self.node.corner_radius_custom(radius);
        self
      }

      pub fn corner_radius_top_left(mut self, radius: impl Into<$crate::node::RadiusValue>) -> Self {
        self.node = self.node.corner_radius_top_left(radius);
        self
      }

      pub fn corner_radius_top_right(mut self, radius: impl Into<$crate::node::RadiusValue>) -> Self {
        self.node = self.node.corner_radius_top_right(radius);
        self
      }

      pub fn corner_radius_bottom_right(mut self, radius: impl Into<$crate::node::RadiusValue>) -> Self {
        self.node = self.node.corner_radius_bottom_right(radius);
        self
      }

      pub fn corner_radius_bottom_left(mut self, radius: impl Into<$crate::node::RadiusValue>) -> Self {
        self.node = self.node.corner_radius_bottom_left(radius);
        self
      }

      pub fn rounded(mut self, radius: impl Into<$crate::node::RadiusValue>) -> Self {
        self.node = self.node.rounded(radius);
        self
      }

      pub fn border_inside(mut self, width: f32, color: impl Into<$crate::node::BackgroundColor>) -> Self {
        self.node = self.node.border_inside(width, color);
        self
      }

      pub fn border_outside(mut self, width: f32, color: impl Into<$crate::node::BackgroundColor>) -> Self {
        self.node = self.node.border_outside(width, color);
        self
      }

      pub fn border_center(mut self, width: f32, color: impl Into<$crate::node::BackgroundColor>) -> Self {
        self.node = self.node.border_center(width, color);
        self
      }

      pub fn border(mut self, border: $crate::node::border::Border) -> Self {
        self.node = self.node.border(border);
        self
      }

      pub fn border_custom(mut self, border: $crate::node::border::Borders) -> Self {
        self.node = self.node.border_custom(border);
        self
      }

      pub fn border_top(mut self, border: $crate::node::border::Border) -> Self {
        self.node = self.node.border_top(border);
        self
      }

      pub fn border_right(mut self, border: $crate::node::border::Border) -> Self {
        self.node = self.node.border_right(border);
        self
      }

      pub fn border_bottom(mut self, border: $crate::node::border::Border) -> Self {
        self.node = self.node.border_bottom(border);
        self
      }

      pub fn border_left(mut self, border: $crate::node::border::Border) -> Self {
        self.node = self.node.border_left(border);
        self
      }

      #[cfg(feature = "image")]
      pub fn background_image(mut self, data: impl Into<$crate::images::ImageKind>) -> Self {
        self.node = self.node.background_image(data);
        self
      }

      #[cfg(feature = "image")]
      pub fn background_size(mut self, size: $crate::node::BackgroundSize) -> Self {
        self.node = self.node.background_size(size);
        self
      }

      #[cfg(feature = "image")]
      pub fn background_cover(mut self) -> Self {
        self.node = self.node.background_cover();
        self
      }

      #[cfg(feature = "image")]
      pub fn background_contain(mut self) -> Self {
        self.node = self.node.background_contain();
        self
      }

      pub fn cursor(mut self, cursor: $crate::node::CursorIcon) -> Self {
        self.node = self.node.cursor(cursor);
        self
      }

      pub fn hovered_style(mut self, style: $crate::node::Style) -> Self {
        self.node = self.node.hovered_style(style);
        self
      }

      pub fn active_style(mut self, style: $crate::node::Style) -> Self {
        self.node = self.node.active_style(style);
        self
      }

      pub fn focused_style(mut self, style: $crate::node::Style) -> Self {
        self.node = self.node.focused_style(style);
        self
      }

      pub fn hovered(mut self, f: impl FnOnce($crate::node::Style) -> $crate::node::Style) -> Self {
        self.node = self.node.hovered(f);
        self
      }

      pub fn active(mut self, f: impl FnOnce($crate::node::Style) -> $crate::node::Style) -> Self {
        self.node = self.node.active(f);
        self
      }

      pub fn focused(mut self, f: impl FnOnce($crate::node::Style) -> $crate::node::Style) -> Self {
        self.node = self.node.focused(f);
        self
      }

      pub fn on_click(mut self, f: impl Fn(&$crate::app::events::MouseEvent) + Send + Sync + 'static) -> Self {
        self.node = self.node.on_click(f);
        self
      }

      pub fn on_dblclick(mut self, f: impl Fn(&$crate::app::events::MouseEvent) + Send + Sync + 'static) -> Self {
        self.node = self.node.on_dblclick(f);
        self
      }

      pub fn on_mouse_down(mut self, f: impl Fn(&$crate::app::events::MouseEvent) + Send + Sync + 'static) -> Self {
        self.node = self.node.on_mouse_down(f);
        self
      }

      pub fn on_mouse_up(mut self, f: impl Fn(&$crate::app::events::MouseEvent) + Send + Sync + 'static) -> Self {
        self.node = self.node.on_mouse_up(f);
        self
      }

      pub fn on_mouse_move(mut self, f: impl Fn(&$crate::app::events::MouseEvent) + Send + Sync + 'static) -> Self {
        self.node = self.node.on_mouse_move(f);
        self
      }

      pub fn on_drag_start(mut self, f: impl Fn(&$crate::app::events::DragEvent) + Send + Sync + 'static) -> Self {
        self.node = self.node.on_drag_start(f);
        self
      }

      pub fn on_drag_move(mut self, f: impl Fn(&$crate::app::events::DragEvent) + Send + Sync + 'static) -> Self {
        self.node = self.node.on_drag_move(f);
        self
      }

      pub fn on_drag_end(mut self, f: impl Fn(&$crate::app::events::DragEvent) + Send + Sync + 'static) -> Self {
        self.node = self.node.on_drag_end(f);
        self
      }

      pub fn on_drop(mut self, f: impl Fn(&$crate::app::events::DropEvent) + Send + Sync + 'static) -> Self {
        self.node = self.node.on_drop(f);
        self
      }

      pub fn on_mouse_enter(mut self, f: impl Fn() + Send + Sync + 'static) -> Self {
        self.node = self.node.on_mouse_enter(f);
        self
      }

      pub fn on_mouse_leave(mut self, f: impl Fn() + Send + Sync + 'static) -> Self {
        self.node = self.node.on_mouse_leave(f);
        self
      }

      pub fn on_key_down(mut self, f: impl Fn(&$crate::app::events::KeyboardEvent) + Send + Sync + 'static) -> Self {
        self.node = self.node.on_key_down(f);
        self
      }

      pub fn on_key_up(mut self, f: impl Fn(&$crate::app::events::KeyboardEvent) + Send + Sync + 'static) -> Self {
        self.node = self.node.on_key_up(f);
        self
      }

      pub fn on_focus(mut self, f: impl Fn() + Send + Sync + 'static) -> Self {
        self.node = self.node.on_focus(f);
        self
      }

      pub fn on_blur(mut self, f: impl Fn() + Send + Sync + 'static) -> Self {
        self.node = self.node.on_blur(f);
        self
      }

      pub fn on_scroll(mut self, f: impl Fn(&$crate::app::events::ScrollEvent) + Send + Sync + 'static) -> Self {
        self.node = self.node.on_scroll(f);
        self
      }

      pub fn on_scroll_start(mut self, f: impl Fn(&$crate::app::events::ScrollEvent) + Send + Sync + 'static) -> Self {
        self.node = self.node.on_scroll_start(f);
        self
      }

      pub fn on_scroll_end(mut self, f: impl Fn(&$crate::app::events::ScrollEvent) + Send + Sync + 'static) -> Self {
        self.node = self.node.on_scroll_end(f);
        self
      }

      pub fn scrollbar(mut self, style: $crate::layout::scrollbar::ScrollBarStyle) -> Self {
        self.node = self.node.scrollbar(style);
        self
      }

      pub fn scrollbar_hovered(
        mut self,
        f: impl Fn($crate::layout::scrollbar::ScrollBarStyle) -> $crate::layout::scrollbar::ScrollBarStyle
        + Send
        + Sync
        + 'static,
      ) -> Self {
        self.node = self.node.scrollbar_hovered(f);
        self
      }

      pub fn ref_element(mut self, element_ref: impl Into<$crate::core::ElementRef>) -> Self {
        self.node = self.node.ref_element(element_ref);
        self
      }

      pub fn interactive(mut self, state: $crate::node::interaction_state::InteractionState) -> Self {
        self.node = self.node.interactive(state);
        self
      }

      pub fn focusable(mut self, focusable: bool) -> Self {
        self.node = self.node.focusable(focusable);
        self
      }

      pub fn tab_index(mut self, tab_index: i32) -> Self {
        self.node = self.node.tab_index(tab_index);
        self
      }

      #[cfg(feature = "form")]
      pub fn name(mut self, name: impl Into<std::sync::Arc<str>>) -> Self {
        self.node = self.node.name(name);
        self
      }

      pub fn clip(mut self) -> Self {
        self.node = self.node.clip();
        self
      }

      pub fn overflow_visible(mut self) -> Self {
        self.node = self.node.overflow_visible();
        self
      }

      pub fn intrinsic(mut self, width: f32, height: f32) -> Self {
        self.node = self.node.intrinsic(width, height);
        self
      }

      pub fn opacity(mut self, value: f32) -> Self {
        self.node = self.node.opacity(value);
        self
      }

      pub fn transition(mut self, spec: $crate::animation::Transition) -> Self {
        self.node = self.node.transition(spec);
        self
      }

      pub fn animation(mut self, spec: $crate::animation::Animation) -> Self {
        self.node = self.node.animation(spec);
        self
      }

      pub fn transform(mut self, t: $crate::node::transform::Transform2D) -> Self {
        self.node = self.node.transform(t);
        self
      }
    }

    impl From<$struct_name> for $crate::node::Element {
      fn from(component: $struct_name) -> Self {
        $crate::node::Element::from_node(component.node)
      }
    }
  };
}
