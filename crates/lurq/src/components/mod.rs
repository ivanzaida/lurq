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
mod popup;
mod rect;
#[cfg(feature = "router")]
mod router_component;
mod row;
mod scroll;
mod select;
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
pub use popup::{Popover, Popup};
pub use rect::Rect;
#[cfg(feature = "router")]
pub use router_component::{Router, RouterProps};
pub use row::Row;
pub use scroll::{ScrollBoth, ScrollHorizontal, ScrollVertical};
pub use select::Select;
pub use slider::Slider;
pub use slot::Slot;
pub use spacer::Spacer;
pub use stack::Stack;
#[cfg(feature = "svg")]
pub use svg::Svg;
pub use text::{Text, TextOverflow};
pub use text_input::{TextInput, TextInputOverflow};

pub use crate::app::ctx::{CollisionStrategy, Modal, ModalTarget, OpenState, Overlay, Parent, Placement, Root};

#[macro_export]
macro_rules! impl_into_node {
  ($struct_name:ident) => {
    pub struct $struct_name {
      pub(crate) node: Box<$crate::node::Node>,
    }

    impl $struct_name {
      pub(crate) fn from_node(node: $crate::node::Node) -> Self {
        Self {
          node: Box::new(node.with_tag_name(Self::tag_name())),
        }
      }

      pub(crate) fn update_node(&mut self, f: impl FnOnce(&mut $crate::node::Node)) {
        f(&mut *self.node);
      }

      pub(crate) fn tag_name() -> std::sync::Arc<str> {
        std::sync::Arc::from(stringify!($struct_name))
      }

      pub fn node_id(&self) -> $crate::core::NodeId {
        self.node.node_id()
      }

      pub fn background(mut self, color: impl Into<$crate::node::BackgroundColor>) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::background(node, color));
        self
      }

      pub fn background_gradient(mut self, gradient: impl Into<$crate::node::Gradient>) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::background_gradient(node, gradient));
        self
      }

      pub fn size(
        mut self,
        width: impl Into<$crate::node::dimension::Dimension>,
        height: impl Into<$crate::node::dimension::Dimension>,
      ) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::size(node, width, height));
        self
      }

      pub fn width(mut self, width: impl Into<$crate::node::dimension::Dimension>) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::width(node, width));
        self
      }

      pub fn height(mut self, height: impl Into<$crate::node::dimension::Dimension>) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::height(node, height));
        self
      }

      pub fn min_width(mut self, width: impl Into<$crate::node::dimension::Dimension>) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::min_width(node, width));
        self
      }

      pub fn max_width(mut self, width: impl Into<$crate::node::dimension::Dimension>) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::max_width(node, width));
        self
      }

      pub fn min_height(mut self, height: impl Into<$crate::node::dimension::Dimension>) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::min_height(node, height));
        self
      }

      pub fn max_height(mut self, height: impl Into<$crate::node::dimension::Dimension>) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::max_height(node, height));
        self
      }

      pub fn min_size(
        mut self,
        width: impl Into<$crate::node::dimension::Dimension>,
        height: impl Into<$crate::node::dimension::Dimension>,
      ) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::min_size(node, width, height));
        self
      }

      pub fn max_size(
        mut self,
        width: impl Into<$crate::node::dimension::Dimension>,
        height: impl Into<$crate::node::dimension::Dimension>,
      ) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::max_size(node, width, height));
        self
      }

      pub fn padding_left(mut self, val: impl Into<$crate::node::SpacingValue>) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::padding_left(node, val));
        self
      }

      pub fn padding_right(mut self, val: impl Into<$crate::node::SpacingValue>) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::padding_right(node, val));
        self
      }

      pub fn padding_top(mut self, val: impl Into<$crate::node::SpacingValue>) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::padding_top(node, val));
        self
      }

      pub fn padding_bottom(mut self, val: impl Into<$crate::node::SpacingValue>) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::padding_bottom(node, val));
        self
      }

      pub fn padding_horizontal(mut self, val: impl Into<$crate::node::SpacingValue>) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::padding_horizontal(node, val));
        self
      }

      pub fn padding_vertical(mut self, val: impl Into<$crate::node::SpacingValue>) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::padding_vertical(node, val));
        self
      }

      pub fn padding(mut self, padding: impl Into<$crate::node::padding::Padding>) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::padding(node, padding));
        self
      }

      pub fn padding_custom(mut self, padding: $crate::node::padding::Padding) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::padding_custom(node, padding));
        self
      }

      pub fn frame(mut self, frame: $crate::layout::layout_kind::FrameConstraints) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::frame(node, frame));
        self
      }

      pub fn offset(mut self, x: f32, y: f32) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::offset(node, x, y));
        self
      }

      pub fn relative(mut self, x: f32, y: f32) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::relative(node, x, y));
        self
      }

      pub fn absolute(
        mut self,
        x: f32,
        y: f32,
        width: impl Into<$crate::node::dimension::Dimension>,
        height: impl Into<$crate::node::dimension::Dimension>,
      ) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::absolute(node, x, y, width, height));
        self
      }

      pub fn absolute_position(mut self, x: f32, y: f32) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::absolute_position(node, x, y));
        self
      }

      pub fn align(mut self, alignment: $crate::layout::Alignment) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::align(node, alignment));
        self
      }

      pub fn flex(mut self, factor: f32) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::flex(node, factor));
        self
      }

      pub fn flex_shrink(mut self, factor: f32) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::flex_shrink(node, factor));
        self
      }

      pub fn flex_full(mut self, grow: f32, shrink: f32, basis: Option<f32>) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::flex_full(node, grow, shrink, basis));
        self
      }

      pub fn corner_radius(mut self, radius: impl Into<$crate::node::RadiusValue>) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::corner_radius(node, radius));
        self
      }

      pub fn corner_radius_custom(mut self, radius: $crate::node::border::BorderRadius) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::corner_radius_custom(node, radius));
        self
      }

      pub fn corner_radius_top_left(mut self, radius: impl Into<$crate::node::RadiusValue>) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::corner_radius_top_left(node, radius));
        self
      }

      pub fn corner_radius_top_right(mut self, radius: impl Into<$crate::node::RadiusValue>) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::corner_radius_top_right(node, radius));
        self
      }

      pub fn corner_radius_bottom_right(mut self, radius: impl Into<$crate::node::RadiusValue>) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::corner_radius_bottom_right(node, radius));
        self
      }

      pub fn corner_radius_bottom_left(mut self, radius: impl Into<$crate::node::RadiusValue>) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::corner_radius_bottom_left(node, radius));
        self
      }

      pub fn rounded(mut self, radius: impl Into<$crate::node::RadiusValue>) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::rounded(node, radius));
        self
      }

      pub fn border_inside(
        mut self,
        width: impl Into<$crate::node::BorderSizeValue>,
        color: impl Into<$crate::node::BackgroundColor>,
      ) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::border_inside(node, width, color));
        self
      }

      pub fn border_outside(
        mut self,
        width: impl Into<$crate::node::BorderSizeValue>,
        color: impl Into<$crate::node::BackgroundColor>,
      ) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::border_outside(node, width, color));
        self
      }

      pub fn border_center(
        mut self,
        width: impl Into<$crate::node::BorderSizeValue>,
        color: impl Into<$crate::node::BackgroundColor>,
      ) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::border_center(node, width, color));
        self
      }

      pub fn border(mut self, border: $crate::node::border::Border) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::border(node, border));
        self
      }

      pub fn border_custom(mut self, border: $crate::node::border::Borders) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::border_custom(node, border));
        self
      }

      pub fn border_top(mut self, border: $crate::node::border::Border) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::border_top(node, border));
        self
      }

      pub fn border_right(mut self, border: $crate::node::border::Border) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::border_right(node, border));
        self
      }

      pub fn border_bottom(mut self, border: $crate::node::border::Border) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::border_bottom(node, border));
        self
      }

      pub fn border_left(mut self, border: $crate::node::border::Border) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::border_left(node, border));
        self
      }

      #[cfg(feature = "image")]
      pub fn background_image(mut self, data: impl Into<$crate::images::ImageKind>) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::background_image(node, data));
        self
      }

      #[cfg(feature = "image")]
      pub fn background_size(mut self, size: $crate::node::BackgroundSize) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::background_size(node, size));
        self
      }

      #[cfg(feature = "image")]
      pub fn background_cover(mut self) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::background_cover(node));
        self
      }

      #[cfg(feature = "image")]
      pub fn background_contain(mut self) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::background_contain(node));
        self
      }

      pub fn cursor(mut self, cursor: $crate::node::CursorIcon) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::cursor(node, cursor));
        self
      }

      pub fn hovered_style(mut self, style: $crate::node::Style) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::hovered_style(node, style));
        self
      }

      pub fn active_style(mut self, style: $crate::node::Style) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::active_style(node, style));
        self
      }

      pub fn focused_style(mut self, style: $crate::node::Style) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::focused_style(node, style));
        self
      }

      pub fn hovered(mut self, f: impl FnOnce($crate::node::Style) -> $crate::node::Style) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::hovered(node, f));
        self
      }

      pub fn active(mut self, f: impl FnOnce($crate::node::Style) -> $crate::node::Style) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::active(node, f));
        self
      }

      pub fn focused(mut self, f: impl FnOnce($crate::node::Style) -> $crate::node::Style) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::focused(node, f));
        self
      }

      pub fn on_click(mut self, f: impl Fn(&$crate::app::events::MouseEvent) + Send + Sync + 'static) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::on_click(node, f));
        self
      }

      pub fn on_mouse_click(
        mut self,
        button: $crate::app::events::MouseButton,
        f: impl Fn(&$crate::app::events::MouseEvent) + Send + Sync + 'static,
      ) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::on_mouse_click(node, button, f));
        self
      }

      pub fn on_dblclick(mut self, f: impl Fn(&$crate::app::events::MouseEvent) + Send + Sync + 'static) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::on_dblclick(node, f));
        self
      }

      pub fn on_mouse_down(mut self, f: impl Fn(&$crate::app::events::MouseEvent) + Send + Sync + 'static) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::on_mouse_down(node, f));
        self
      }

      pub fn on_mouse_up(mut self, f: impl Fn(&$crate::app::events::MouseEvent) + Send + Sync + 'static) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::on_mouse_up(node, f));
        self
      }

      pub fn on_mouse_move(mut self, f: impl Fn(&$crate::app::events::MouseEvent) + Send + Sync + 'static) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::on_mouse_move(node, f));
        self
      }

      pub fn on_drag_start(mut self, f: impl Fn(&$crate::app::events::DragEvent) + Send + Sync + 'static) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::on_drag_start(node, f));
        self
      }

      pub fn on_drag_move(mut self, f: impl Fn(&$crate::app::events::DragEvent) + Send + Sync + 'static) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::on_drag_move(node, f));
        self
      }

      pub fn on_drag_end(mut self, f: impl Fn(&$crate::app::events::DragEvent) + Send + Sync + 'static) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::on_drag_end(node, f));
        self
      }

      pub fn on_drop(mut self, f: impl Fn(&$crate::app::events::DropEvent) + Send + Sync + 'static) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::on_drop(node, f));
        self
      }

      pub fn on_mouse_enter(mut self, f: impl Fn() + Send + Sync + 'static) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::on_mouse_enter(node, f));
        self
      }

      pub fn on_mouse_leave(mut self, f: impl Fn() + Send + Sync + 'static) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::on_mouse_leave(node, f));
        self
      }

      pub fn on_key_down(mut self, f: impl Fn(&$crate::app::events::KeyboardEvent) + Send + Sync + 'static) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::on_key_down(node, f));
        self
      }

      pub fn on_key_up(mut self, f: impl Fn(&$crate::app::events::KeyboardEvent) + Send + Sync + 'static) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::on_key_up(node, f));
        self
      }

      pub fn on_focus(mut self, f: impl Fn() + Send + Sync + 'static) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::on_focus(node, f));
        self
      }

      pub fn on_blur(mut self, f: impl Fn() + Send + Sync + 'static) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::on_blur(node, f));
        self
      }

      pub fn on_scroll(mut self, f: impl Fn(&$crate::app::events::ScrollEvent) + Send + Sync + 'static) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::on_scroll(node, f));
        self
      }

      pub fn on_scroll_start(mut self, f: impl Fn(&$crate::app::events::ScrollEvent) + Send + Sync + 'static) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::on_scroll_start(node, f));
        self
      }

      pub fn on_scroll_end(mut self, f: impl Fn(&$crate::app::events::ScrollEvent) + Send + Sync + 'static) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::on_scroll_end(node, f));
        self
      }

      pub fn scrollbar(mut self, style: $crate::layout::scrollbar::ScrollBarStyle) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::scrollbar(node, style));
        self
      }

      pub fn scrollbar_hovered(
        mut self,
        f: impl Fn($crate::layout::scrollbar::ScrollBarStyle) -> $crate::layout::scrollbar::ScrollBarStyle
        + Send
        + Sync
        + 'static,
      ) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::scrollbar_hovered(node, f));
        self
      }

      pub fn hit_test(mut self, behavior: $crate::node::HitTestBehavior) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::hit_test(node, behavior));
        self
      }

      pub fn pointer_events_none(mut self) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::pointer_events_none(node));
        self
      }

      pub fn ref_element(mut self, element_ref: impl Into<$crate::core::ElementRef>) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::ref_element(node, element_ref));
        self
      }

      pub fn interactive(mut self, state: $crate::node::interaction_state::InteractionState) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::interactive(node, state));
        self
      }

      pub fn focusable(mut self, focusable: bool) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::focusable(node, focusable));
        self
      }

      pub fn tab_index(mut self, tab_index: i32) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::tab_index(node, tab_index));
        self
      }

      #[cfg(feature = "form")]
      pub fn name(mut self, name: impl Into<std::sync::Arc<str>>) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::name(node, name));
        self
      }

      pub fn clip(mut self) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::clip(node));
        self
      }

      pub fn overflow_visible(mut self) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::overflow_visible(node));
        self
      }

      pub fn intrinsic(mut self, width: f32, height: f32) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::intrinsic(node, width, height));
        self
      }

      pub fn opacity(mut self, value: f32) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::opacity(node, value));
        self
      }

      pub fn transition(mut self, spec: $crate::animation::Transition) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::transition(node, spec));
        self
      }

      pub fn animation(mut self, spec: $crate::animation::Animation) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::animation(node, spec));
        self
      }

      pub fn transform(mut self, t: $crate::node::transform::Transform2D) -> Self {
        self.update_node(|node| $crate::node::NodeUpdate::transform(node, t));
        self
      }
    }

    impl From<$struct_name> for $crate::node::Element {
      fn from(component: $struct_name) -> Self {
        $crate::node::Element::from_node(*component.node)
      }
    }
  };
}
