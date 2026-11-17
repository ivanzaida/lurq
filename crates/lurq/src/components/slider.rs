use crate::{
  core::Signal,
  impl_into_node,
  node::{SliderOrientation, SliderPartStyle},
};

impl_into_node!(
  /// The engine's slider: a track, its fill and a thumb bound to a value, dragged, pressed and moved with the keys.
  /// It is what [`ui::Slider`](crate::ui::Slider) dresses with the design system's recipe, and what an app normally
  /// wants; with no style of its own this one fills its track and thumb as the recipe the theme keeps for it says
  /// (`engine_slider`).
  Slider
);

/// What the engine's slider node fills its track and its thumb with where its style sets no fill comes from the
/// theme, through this recipe, by the parts of [`ui::Slider`](crate::ui::Slider): `SliderPart::Track`'s fill and
/// `SliderPart::Thumb`'s.
impl crate::ui::Themed for Slider {
  const NAME: &'static str = "engine_slider";
  type Renderer = ();

  /// The design's roles for 0.62's look: the track in the border's color, the thumb in the primary color.
  fn recipe() -> crate::ui::Recipe {
    use crate::app::theme::PaletteColor::{Border, Primary};
    fills(Border, Primary)
  }
}

fn fills(track: crate::app::theme::PaletteColor, thumb: crate::app::theme::PaletteColor) -> crate::ui::Recipe {
  use crate::ui::{Recipe, SliderPart};
  Recipe::new()
    .base(SliderPart::Track, |s| s.background(track))
    .base(SliderPart::Thumb, |s| s.background(thumb))
}

impl Slider {
  /// 0.62's roles: the recipe [`UiDefinition::legacy`](crate::ui::UiDefinition::legacy) gives the slider node.
  pub(crate) fn legacy_recipe() -> crate::ui::Recipe {
    use crate::layout::layout_engine::control_fill::legacy;
    fills(legacy::TRACK, legacy::THUMB)
  }

  pub fn new(value: Signal<i32>) -> Self {
    Self::from_node(crate::node::Node::slider(value))
  }

  pub fn new_f32(value: Signal<f32>) -> Self {
    Self::from_node(crate::node::Node::slider_f32(value))
  }

  /// A slider of two thumbs on one track, bound to a low value and a high
  /// one: the track between them is filled, a press on the track moves the
  /// nearer thumb, and neither passes the other. Each thumb is a stop of Tab,
  /// the low one first, and the arrow keys move the one focused.
  pub fn new_range(values: Signal<(i32, i32)>) -> Self {
    Self::from_node(crate::node::Node::range_slider(values))
  }

  /// [`Slider::new_range`] of fractions.
  pub fn new_range_f32(values: Signal<(f32, f32)>) -> Self {
    Self::from_node(crate::node::Node::range_slider_f32(values))
  }

  /// Which way the track runs. A vertical slider has its least value at the
  /// bottom and is dragged along its height; its track, fill and thumb are
  /// laid out along the height, so a track style's `width` is its thickness.
  pub fn orientation(mut self, orientation: SliderOrientation) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::slider_orientation(node, orientation));
    self
  }

  /// [`Slider::orientation`] vertical.
  pub fn vertical(self) -> Self {
    self.orientation(SliderOrientation::Vertical)
  }

  pub fn range(mut self, min: i32, max: i32) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::range(node, min, max));
    self
  }

  pub fn range_f32(mut self, min: f32, max: f32) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::range_f32(node, min, max));
    self
  }

  pub fn step(mut self, step: f32) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::slider_step(node, step));
    self
  }

  /// Called each time a person changes the value: a drag as it moves, a press on the track, an arrow, Page or
  /// Home/End key, a change from tooling. Given the low value and the high one; of a slider of one value, that
  /// value twice. Not called when the app sets the signal, nor when a new range or step snaps it. Each call adds a
  /// handler.
  pub fn on_change(mut self, handler: impl Fn(f32, f32) + Send + Sync + 'static) -> Self {
    let handler: crate::node::node_kind::SliderChangeHandler = std::sync::Arc::new(handler);
    self.update_node(|node| {
      if let Some(state) = node.slider_state() {
        state.add_on_change(handler);
      }
    });
    self
  }

  /// Called when an interaction that changed the value is over, with the value as it then is: the drag let go,
  /// the press on the track or the key done. Each call adds a handler.
  pub fn on_change_end(mut self, handler: impl Fn(f32, f32) + Send + Sync + 'static) -> Self {
    let handler: crate::node::node_kind::SliderChangeHandler = std::sync::Arc::new(handler);
    self.update_node(|node| {
      if let Some(state) = node.slider_state() {
        state.add_on_change_end(handler);
      }
    });
    self
  }

  pub fn track_style(mut self, style: SliderPartStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::slider_track_style(node, style));
    self
  }

  pub fn track(mut self, f: impl FnOnce(SliderPartStyle) -> SliderPartStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::slider_track_style(node, f(SliderPartStyle::new())));
    self
  }

  pub fn track_hovered_style(mut self, style: SliderPartStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::slider_track_hovered_style(node, style));
    self
  }

  pub fn track_hovered(mut self, f: impl FnOnce(SliderPartStyle) -> SliderPartStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::slider_track_hovered_style(node, f(SliderPartStyle::new())));
    self
  }

  pub fn fill_style(mut self, style: SliderPartStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::slider_fill_style(node, style));
    self
  }

  pub fn fill(mut self, f: impl FnOnce(SliderPartStyle) -> SliderPartStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::slider_fill_style(node, f(SliderPartStyle::new())));
    self
  }

  pub fn fill_hovered_style(mut self, style: SliderPartStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::slider_fill_hovered_style(node, style));
    self
  }

  pub fn fill_hovered(mut self, f: impl FnOnce(SliderPartStyle) -> SliderPartStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::slider_fill_hovered_style(node, f(SliderPartStyle::new())));
    self
  }

  pub fn thumb_style(mut self, style: SliderPartStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::slider_thumb_style(node, style));
    self
  }

  pub fn thumb(mut self, f: impl FnOnce(SliderPartStyle) -> SliderPartStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::slider_thumb_style(node, f(SliderPartStyle::new())));
    self
  }

  pub fn thumb_hovered_style(mut self, style: SliderPartStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::slider_thumb_hovered_style(node, style));
    self
  }

  pub fn thumb_hovered(mut self, f: impl FnOnce(SliderPartStyle) -> SliderPartStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::slider_thumb_hovered_style(node, f(SliderPartStyle::new())));
    self
  }

  /// Thumb style while the slider has keyboard focus (focus-visible, like a
  /// browser's range input ring: a click or drag does not show it), layered
  /// under the hovered style. Size is ignored: focus changes paint, never
  /// layout. For a style on any focus, use the element's `focused_style`.
  pub fn thumb_focused_style(mut self, style: SliderPartStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::slider_thumb_focused_style(node, style));
    self
  }

  pub fn thumb_focused(mut self, f: impl FnOnce(SliderPartStyle) -> SliderPartStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::slider_thumb_focused_style(node, f(SliderPartStyle::new())));
    self
  }
}

impl Default for Slider {
  fn default() -> Self {
    Self::new(Signal::new(0))
  }
}
