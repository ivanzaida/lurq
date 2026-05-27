mod checkbox;
mod column;
mod rect;
mod row;
mod scroll;
mod slider;
mod spacer;
mod stack;
mod text;
mod text_input;

pub use checkbox::Checkbox;
pub use column::Column;
pub use rect::Rect;
pub use row::Row;
pub use scroll::{ScrollBoth, ScrollHorizontal, ScrollVertical};
pub use slider::Slider;
pub use spacer::Spacer;
pub use stack::Stack;
pub use text::Text;
pub use text_input::TextInput;

#[macro_export]
macro_rules! impl_into_node {
  ($struct_name:ident) => {
    pub struct $struct_name {
      pub(crate) node: $crate::node::Node,
    }

    impl $struct_name {
      pub(crate) fn from_node(node: $crate::node::Node) -> Self {
        Self { node }
      }

      pub fn node_id(&self) -> $crate::core::NodeId {
        self.node.node_id()
      }

      pub fn fill(mut self, hex: &str) -> Self {
        self.node = self.node.fill(hex);
        self
      }

      pub fn background(mut self, color: $crate::node::color::Color) -> Self {
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

      pub fn pad(mut self, all: f32) -> Self {
        self.node = self.node.pad(all);
        self
      }

      pub fn pad_xy(mut self, horizontal: f32, vertical: f32) -> Self {
        self.node = self.node.pad_xy(horizontal, vertical);
        self
      }

      pub fn pad_left(mut self, val: f32) -> Self {
        self.node = self.node.pad_left(val);
        self
      }

      pub fn pad_right(mut self, val: f32) -> Self {
        self.node = self.node.pad_right(val);
        self
      }

      pub fn pad_top(mut self, val: f32) -> Self {
        self.node = self.node.pad_top(val);
        self
      }

      pub fn pad_bottom(mut self, val: f32) -> Self {
        self.node = self.node.pad_bottom(val);
        self
      }

      pub fn padding(mut self, padding: $crate::node::padding::Padding) -> Self {
        self.node = self.node.padding(padding);
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

      pub fn corner_radius(mut self, radius: $crate::node::border::BorderRadius) -> Self {
        self.node = self.node.corner_radius(radius);
        self
      }

      pub fn rounded(mut self, radius: f32) -> Self {
        self.node = self.node.rounded(radius);
        self
      }

      pub fn border_inside(mut self, width: f32, color: $crate::node::color::Color) -> Self {
        self.node = self.node.border_inside(width, color);
        self
      }

      pub fn border_outside(mut self, width: f32, color: $crate::node::color::Color) -> Self {
        self.node = self.node.border_outside(width, color);
        self
      }

      pub fn border_center(mut self, width: f32, color: $crate::node::color::Color) -> Self {
        self.node = self.node.border_center(width, color);
        self
      }

      pub fn border_custom(mut self, border: $crate::node::border::Border) -> Self {
        self.node = self.node.border_custom(border);
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

      pub fn pad_dimension(mut self, padding: $crate::node::padding::Padding) -> Self {
        self.node = self.node.padding(padding);
        self
      }

      pub fn pad_dimension_all(self, all: $crate::node::dimension::Dimension) -> Self {
        self.padding($crate::node::padding::Padding::all(all))
      }
    }

    impl From<$struct_name> for $crate::node::Element {
      fn from(component: $struct_name) -> Self {
        $crate::node::Element::from_node(component.node)
      }
    }
  };
}

#[cfg(test)]
mod tests {
  use crate::{
    app::Runtime,
    components::{Column, Rect, Text},
    layout::{Constraints, Size},
  };

  #[test]
  fn typed_components_convert_to_elements() {
    let mut rt = Runtime::new();
    rt.set_root(
      Column::new()
        .spacing(4.0)
        .child(Text::new("hello"))
        .child(Rect::new(10.0, 20.0).rounded(2.0)),
    );

    let result = rt.compute_layout(Constraints::loose(Size::new(200.0, 200.0))).unwrap();
    assert_eq!(result.children.len(), 2);
  }
}
