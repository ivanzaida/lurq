use crate::node::color::Color;

#[derive(Clone, lurq_macros::Accessors)]
pub struct ScrollBarStyle {
  pub width: f32,
  pub min_thumb_length: f32,
  pub track_color: Color,
  pub thumb_color: Color,
  pub thumb_radius: f32,
  pub track_radius: f32,
  pub padding: f32,
  pub visible: ScrollBarVisibility,
  pub placement: ScrollBarPlacement,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum ScrollBarVisibility {
  #[default]
  Auto,
  Always,
  Never,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum ScrollBarPlacement {
  #[default]
  Overlay,
  Reserved,
}

impl Default for ScrollBarStyle {
  fn default() -> Self {
    Self {
      width: 8.0,
      min_thumb_length: 24.0,
      track_color: Color::new(0, 0, 0, 0),
      thumb_color: Color::new(0, 0, 0, 80),
      thumb_radius: 4.0,
      track_radius: 4.0,
      padding: 2.0,
      visible: ScrollBarVisibility::Auto,
      placement: ScrollBarPlacement::Overlay,
    }
  }
}

impl ScrollBarStyle {
  pub fn thin() -> Self {
    Self {
      width: 4.0,
      min_thumb_length: 16.0,
      thumb_radius: 2.0,
      track_radius: 2.0,
      padding: 1.0,
      ..Self::default()
    }
  }

  pub fn wide() -> Self {
    Self {
      width: 12.0,
      min_thumb_length: 32.0,
      thumb_radius: 6.0,
      track_radius: 6.0,
      padding: 2.0,
      ..Self::default()
    }
  }

  pub fn hidden() -> Self {
    Self {
      visible: ScrollBarVisibility::Never,
      ..Self::default()
    }
  }
}

pub struct ScrollBarGeometry {
  pub track_x: f32,
  pub track_y: f32,
  pub track_width: f32,
  pub track_height: f32,
  pub thumb_x: f32,
  pub thumb_y: f32,
  pub thumb_width: f32,
  pub thumb_height: f32,
}

pub fn compute_vertical_scrollbar(
  style: &ScrollBarStyle,
  viewport_x: f32,
  viewport_y: f32,
  viewport_width: f32,
  viewport_height: f32,
  content_height: f32,
  scroll_y: f32,
) -> Option<ScrollBarGeometry> {
  match style.visible {
    ScrollBarVisibility::Never => return None,
    ScrollBarVisibility::Auto if content_height <= viewport_height => return None,
    _ => {}
  }

  let track_x = viewport_x + viewport_width - style.width - style.padding;
  let track_y = viewport_y + style.padding;
  let track_width = style.width;
  let track_height = viewport_height - style.padding * 2.0;

  let ratio = viewport_height / content_height.max(1.0);
  let thumb_height = (track_height * ratio).max(style.min_thumb_length).min(track_height);
  let max_scroll = (content_height - viewport_height).max(0.0);
  let scroll_ratio = if max_scroll > 0.0 { scroll_y / max_scroll } else { 0.0 };
  let thumb_y = track_y + (track_height - thumb_height) * scroll_ratio;

  Some(ScrollBarGeometry {
    track_x,
    track_y,
    track_width,
    track_height,
    thumb_x: track_x,
    thumb_y,
    thumb_width: track_width,
    thumb_height,
  })
}

pub fn compute_horizontal_scrollbar(
  style: &ScrollBarStyle,
  viewport_x: f32,
  viewport_y: f32,
  viewport_width: f32,
  viewport_height: f32,
  content_width: f32,
  scroll_x: f32,
) -> Option<ScrollBarGeometry> {
  match style.visible {
    ScrollBarVisibility::Never => return None,
    ScrollBarVisibility::Auto if content_width <= viewport_width => return None,
    _ => {}
  }

  let track_x = viewport_x + style.padding;
  let track_y = viewport_y + viewport_height - style.width - style.padding;
  let track_width = viewport_width - style.padding * 2.0;
  let track_height = style.width;

  let ratio = viewport_width / content_width.max(1.0);
  let thumb_width = (track_width * ratio).max(style.min_thumb_length).min(track_width);
  let max_scroll = (content_width - viewport_width).max(0.0);
  let scroll_ratio = if max_scroll > 0.0 { scroll_x / max_scroll } else { 0.0 };
  let thumb_x = track_x + (track_width - thumb_width) * scroll_ratio;

  Some(ScrollBarGeometry {
    track_x,
    track_y,
    track_width,
    track_height,
    thumb_x,
    thumb_y: track_y,
    thumb_width,
    thumb_height: track_height,
  })
}
