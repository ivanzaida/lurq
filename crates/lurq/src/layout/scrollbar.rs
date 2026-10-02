use crate::node::color::Color;

const DEFAULT_SCROLLBAR_WIDTH: f32 = 8.0;
const DEFAULT_SCROLLBAR_MIN_THUMB_LENGTH: f32 = 24.0;
const DEFAULT_SCROLLBAR_TRACK_COLOR: Color = Color::new(0, 0, 0, 0);
const DEFAULT_SCROLLBAR_THUMB_COLOR: Color = Color::new(0, 0, 0, 80);
const DEFAULT_SCROLLBAR_THUMB_RADIUS: f32 = 4.0;
const DEFAULT_SCROLLBAR_TRACK_RADIUS: f32 = 4.0;
const DEFAULT_SCROLLBAR_PADDING: f32 = 2.0;

const THIN_SCROLLBAR_WIDTH: f32 = 4.0;
const THIN_SCROLLBAR_MIN_THUMB_LENGTH: f32 = 16.0;
const THIN_SCROLLBAR_THUMB_RADIUS: f32 = 2.0;
const THIN_SCROLLBAR_TRACK_RADIUS: f32 = 2.0;
const THIN_SCROLLBAR_PADDING: f32 = 1.0;

const WIDE_SCROLLBAR_WIDTH: f32 = 12.0;
const WIDE_SCROLLBAR_MIN_THUMB_LENGTH: f32 = 32.0;
const WIDE_SCROLLBAR_THUMB_RADIUS: f32 = 6.0;
const WIDE_SCROLLBAR_TRACK_RADIUS: f32 = 6.0;
const WIDE_SCROLLBAR_PADDING: f32 = 2.0;

#[derive(Clone, lurq_macros::Accessors)]
pub struct ScrollBarStyle {
  pub width: f32,
  pub min_thumb_length: f32,
  pub track_color: Color,
  pub thumb_color: Color,
  pub thumb_radius: f32,
  pub track_radius: f32,
  /// The gap around the bar: from the edge it runs along and from the two
  /// edges its track ends at. [`Self::edge_inset`] and [`Self::end_inset`]
  /// override each side.
  pub padding: f32,
  /// The gap between the bar and the edge it runs along (the right edge for
  /// a vertical bar, the bottom edge for a horizontal one), measured from the
  /// scroll container's outer bounds. `None` uses [`Self::padding`].
  pub edge_inset: Option<f32>,
  /// The gap between each end of the track and the container's outer edge
  /// (top and bottom for a vertical bar), e.g. to clear rounded corners: the
  /// track is the container's length minus twice this. `None` uses
  /// [`Self::padding`].
  pub end_inset: Option<f32>,
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
      width: DEFAULT_SCROLLBAR_WIDTH,
      min_thumb_length: DEFAULT_SCROLLBAR_MIN_THUMB_LENGTH,
      track_color: DEFAULT_SCROLLBAR_TRACK_COLOR,
      thumb_color: DEFAULT_SCROLLBAR_THUMB_COLOR,
      thumb_radius: DEFAULT_SCROLLBAR_THUMB_RADIUS,
      track_radius: DEFAULT_SCROLLBAR_TRACK_RADIUS,
      padding: DEFAULT_SCROLLBAR_PADDING,
      edge_inset: None,
      end_inset: None,
      visible: ScrollBarVisibility::Auto,
      placement: ScrollBarPlacement::Overlay,
    }
  }
}

impl ScrollBarStyle {
  pub fn thin() -> Self {
    Self {
      width: THIN_SCROLLBAR_WIDTH,
      min_thumb_length: THIN_SCROLLBAR_MIN_THUMB_LENGTH,
      thumb_radius: THIN_SCROLLBAR_THUMB_RADIUS,
      track_radius: THIN_SCROLLBAR_TRACK_RADIUS,
      padding: THIN_SCROLLBAR_PADDING,
      ..Self::default()
    }
  }

  pub fn wide() -> Self {
    Self {
      width: WIDE_SCROLLBAR_WIDTH,
      min_thumb_length: WIDE_SCROLLBAR_MIN_THUMB_LENGTH,
      thumb_radius: WIDE_SCROLLBAR_THUMB_RADIUS,
      track_radius: WIDE_SCROLLBAR_TRACK_RADIUS,
      padding: WIDE_SCROLLBAR_PADDING,
      ..Self::default()
    }
  }

  /// Sets [`Self::edge_inset`] and [`Self::end_inset`].
  pub fn insets(mut self, edge: f32, end: f32) -> Self {
    self.edge_inset = Some(edge);
    self.end_inset = Some(end);
    self
  }

  /// The resolved gap between the bar and the edge it runs along.
  pub fn resolved_edge_inset(&self) -> f32 {
    self.edge_inset.unwrap_or(self.padding)
  }

  /// The resolved gap between each track end and the container's edge.
  pub fn resolved_end_inset(&self) -> f32 {
    self.end_inset.unwrap_or(self.padding)
  }

  pub fn hidden() -> Self {
    Self {
      visible: ScrollBarVisibility::Never,
      ..Self::default()
    }
  }
}

/// How far the area that takes the pointer for a scrollbar reaches beyond
/// what is painted, so a press just off a thin bar still lands on it.
const SCROLLBAR_HIT_SLOP: f32 = 4.0;

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

/// The part of a scrollbar under the pointer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ScrollBarPart {
  Thumb,
  /// The track between its start and the thumb.
  TrackBefore,
  /// The track between the thumb and its end.
  TrackAfter,
}

impl ScrollBarGeometry {
  /// The part of the scrollbar at `(x, y)` for a horizontal or a vertical bar,
  /// or `None` where the pointer belongs to the content.
  ///
  /// Where the bar visibly owns its lane, a `Reserved` gutter or an overlay
  /// bar with a painted track, the whole lane takes the pointer: the track
  /// along the axis and, across it, [`SCROLLBAR_HIT_SLOP`] or the edge inset
  /// beyond the track on both sides, whichever is larger, which covers the
  /// gutter and reaches the container edge. A plain overlay bar (transparent
  /// track) shows only its thumb over the content, so only the thumb takes the
  /// pointer, with [`SCROLLBAR_HIT_SLOP`] around it on every side; the rest of
  /// the lane is content.
  pub(crate) fn hit_part_at(&self, horizontal: bool, style: &ScrollBarStyle, x: f32, y: f32) -> Option<ScrollBarPart> {
    let (along, across) = if horizontal { (x, y) } else { (y, x) };
    let (track_start, track_length, cross_start, thickness) = if horizontal {
      (self.track_x, self.track_width, self.track_y, self.track_height)
    } else {
      (self.track_y, self.track_height, self.track_x, self.track_width)
    };
    let (thumb_start, thumb_length) = if horizontal {
      (self.thumb_x, self.thumb_width)
    } else {
      (self.thumb_y, self.thumb_height)
    };

    let owns_lane = style.placement == ScrollBarPlacement::Reserved || style.track_color.a() > 0;
    if !owns_lane {
      let on_thumb = across >= cross_start - SCROLLBAR_HIT_SLOP
        && across <= cross_start + thickness + SCROLLBAR_HIT_SLOP
        && along >= thumb_start - SCROLLBAR_HIT_SLOP
        && along <= thumb_start + thumb_length + SCROLLBAR_HIT_SLOP;
      return on_thumb.then_some(ScrollBarPart::Thumb);
    }

    let reach = SCROLLBAR_HIT_SLOP.max(style.resolved_edge_inset());
    let in_lane = across >= cross_start - reach
      && across <= cross_start + thickness + reach
      && along >= track_start
      && along <= track_start + track_length;
    if !in_lane {
      return None;
    }
    Some(if along < thumb_start {
      ScrollBarPart::TrackBefore
    } else if along > thumb_start + thumb_length {
      ScrollBarPart::TrackAfter
    } else {
      ScrollBarPart::Thumb
    })
  }
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

  let track_x = viewport_x + viewport_width - style.width - style.resolved_edge_inset();
  let track_y = viewport_y + style.resolved_end_inset();
  let track_width = style.width;
  let track_height = viewport_height - style.resolved_end_inset() * 2.0;

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

  let track_x = viewport_x + style.resolved_end_inset();
  let track_y = viewport_y + viewport_height - style.width - style.resolved_edge_inset();
  let track_width = viewport_width - style.resolved_end_inset() * 2.0;
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
