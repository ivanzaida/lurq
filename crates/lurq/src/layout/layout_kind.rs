use std::sync::{Arc, Mutex};

use crate::{
  animation::Easing,
  layout::{
    Alignment, Offset, Size, StackAlignment,
    scrollbar::{
      ScrollBarGeometry, ScrollBarPart, ScrollBarReveal, ScrollBarRevealFrame, ScrollBarStyle, ScrollBarVisibility,
    },
  },
  node::{dimension::Dimension, spacing_value::SpacingValue},
  time::{Duration, Instant},
};

const DEFAULT_FLEX_GROW: f32 = 1.0;
const DEFAULT_FLEX_SHRINK: f32 = 0.0;
const SCROLL_END_EPSILON: f32 = 2.0;
/// Text taller than its box by less than this fits: rows of a fractional
/// line height add up to a hair more than the box sized from them.
const TEXT_OVERFLOW_EPSILON: f32 = 0.5;
/// How long a mouse's wheel takes to scroll the content to where a notch sends it. Browsers and most apps on
/// Windows spread a notch over a tenth of a second or a little more. This is the long end of that, and along
/// [`WHEEL_SCROLL_EASING`] the content has gone nine tenths of the way in the first 80 ms, so a notch still
/// feels like one.
const WHEEL_SCROLL_DURATION: Duration = Duration::from_millis(150);
/// The curve of a wheel's scroll: a cubic ease-out, `1 - (1 - t)^3` as a Bezier curve. The content starts at its
/// fastest, three times its mean speed, so the frame after the notch already moves, a sixth of the way at
/// 120 Hz, and it slows down into its place with no stop to see. A notch that comes on the way starts the curve
/// again from where the content is with the rest of the way added, which only makes it faster: a wheel that
/// is spun never comes to rest between two notches.
const WHEEL_SCROLL_EASING: Easing = Easing::CubicBezier {
  x1: 1.0 / 3.0,
  y1: 1.0,
  x2: 2.0 / 3.0,
  y2: 1.0,
};

#[derive(Clone)]
pub enum LayoutKind {
  Leaf,
  Row {
    spacing: SpacingValue,
    align: Alignment,
    justify: Justify,
    wrap: FlexWrap,
  },
  Column {
    spacing: SpacingValue,
    align: Alignment,
    justify: Justify,
    wrap: FlexWrap,
  },
  Stack {
    align: StackAlignment,
  },
  /// Children in the cells of columns and rows: see [`crate::layout::grid`]. Boxed: a node holds its layout
  /// kind inline.
  Grid(Box<crate::layout::grid::GridSpec>),
  LogicalModifier,
  ScrollModifier {
    state: ScrollState,
    direction: ScrollDirection,
    culling: bool,
    /// A middle-button press scrolls the container by how far the pointer
    /// moves from where the button went down; see
    /// `Node::middle_button_autoscroll`.
    middle_button_autoscroll: bool,
  },
}

/// What a finger that scrolls a list does to a phone's on-screen keyboard.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum KeyboardDismissal {
  /// Nothing: the keyboard stays up.
  #[default]
  None,
  /// The keyboard goes away when the finger starts to scroll.
  OnDrag,
  /// The keyboard goes down with the finger once the finger reaches it, and comes back if the finger goes
  /// back up: the platform's own, where there is a platform to do it. Elsewhere it is nothing.
  Interactive,
}

#[derive(Clone)]
pub struct ScrollState {
  inner: Arc<Mutex<ScrollStateInner>>,
}

/// A thumb at `start`, `length` long, while the content is drawn `overscroll` past an end: it shortens by as
/// much against that end, as a scroll indicator of the platform does, down to a dot as wide as the bar.
fn squeezed_thumb(start: f32, length: f32, overscroll: f32, bar_width: f32) -> (f32, f32) {
  if overscroll == 0.0 {
    return (start, length);
  }
  let squeezed = (length - overscroll.abs()).max(bar_width.min(length));
  if overscroll < 0.0 {
    (start, squeezed)
  } else {
    (start + length - squeezed, squeezed)
  }
}

#[derive(Clone, Copy)]
enum PendingScroll {
  Offset(f32),
  Start,
  End,
  PreservePrependAnchor {
    previous_content: f32,
    previous_scroll: f32,
  },
  /// Scroll by the least amount that shows `[start, end]` of the content.
  Reveal {
    start: f32,
    end: f32,
  },
  /// The offset that keeps what is shown in place after the content above it
  /// changed height (a virtualized list's anchor row).
  Anchor(f32),
}

impl PendingScroll {
  /// A scroll to a place someone asked for (an offset, an edge, a reveal),
  /// unlike a prepend anchor or a kept anchor, which only keep the current
  /// place.
  fn is_request(self) -> bool {
    !matches!(self, Self::PreservePrependAnchor { .. } | Self::Anchor(_))
  }

  /// The place kept, once the offset it was taken at has gone `by` further: a wheel's scroll went on between
  /// the moment the place was taken and the layout that keeps it. A request is to a place of its own.
  fn moved_by(self, by: f32) -> Self {
    match self {
      Self::Anchor(offset) => Self::Anchor(offset + by),
      Self::PreservePrependAnchor {
        previous_content,
        previous_scroll,
      } => Self::PreservePrependAnchor {
        previous_content,
        previous_scroll: previous_scroll + by,
      },
      request => request,
    }
  }
}

/// A wheel's scroll on its way along one axis; see [`ScrollState::wheel_by_with_overflow`].
#[derive(Clone, Copy)]
struct WheelScroll {
  /// Where the offset is going.
  target: f32,
  /// How far from there the offset was when the wheel last turned, and when that was.
  distance: f32,
  turned: Instant,
  /// The offset the last step left. A layout that has moved it since (content that came in above what is
  /// shown, an offset held at the end of shorter content) moved where it is going as far.
  stepped: f32,
  /// It is going to the end of the content, and goes on to it if the content grows meanwhile: once there, the
  /// offset stays at the end as any offset at the end does.
  to_end: bool,
}

impl WheelScroll {
  /// A scroll from `offset` to `target`, of content that can go as far as `max`, that starts at `now`.
  fn to(target: f32, offset: f32, max: f32, now: Instant) -> Self {
    Self {
      target,
      distance: target - offset,
      turned: now,
      stepped: offset,
      to_end: max > 0.0 && max - target <= SCROLL_END_EPSILON,
    }
  }

  /// The scroll a wheel that turns by `delta` at `now` leaves, for an offset at `offset` that can go as far as
  /// `max`, and how much of the delta it takes. The delta adds to where the scroll was already going, and the
  /// way there starts again from the offset.
  fn turned(wheel: Option<Self>, offset: f32, max: f32, delta: f32, now: Instant) -> (Option<Self>, f32) {
    if delta == 0.0 {
      return (wheel, 0.0);
    }
    let from = wheel.map_or(offset, |wheel| wheel.target);
    let target = (from + delta).clamp(0.0, max);
    let wheel = (target != offset).then(|| Self::to(target, offset, max, now));
    (wheel, target - from)
  }

  /// Where the offset is at `now`, for an offset drawn at `offset` that can go as far as `max`, and whether
  /// that is where it was going.
  fn step(&mut self, offset: f32, max: f32, now: Instant) -> (f32, bool) {
    self.target += offset - self.stepped;
    let target = if self.to_end { max } else { self.target.clamp(0.0, max) };
    if target != self.target {
      // The content is shorter than where the scroll was going, or its end has moved: the scroll goes there
      // from where it is.
      *self = Self::to(target, offset, max, now);
    }
    let progress = now.saturating_duration_since(self.turned).as_secs_f64() / WHEEL_SCROLL_DURATION.as_secs_f64();
    let next = if progress >= 1.0 {
      self.target
    } else {
      let left = self.distance * (1.0 - WHEEL_SCROLL_EASING.evaluate(progress) as f32);
      self.target - left
    };
    self.stepped = next;
    (next, next == self.target)
  }
}

/// Scroll position and range when the current layout pass first measured
/// this scroll container.
#[derive(Clone, Copy)]
struct ScrollPassStart {
  scroll_x: f32,
  scroll_y: f32,
  max_scroll_x: f32,
  max_scroll_y: f32,
}

struct ScrollStateInner {
  scroll_x: f32,
  scroll_y: f32,
  /// How far the content is drawn past its ends; see [`ScrollState::set_overscroll`].
  overscroll_x: f32,
  overscroll_y: f32,
  /// See [`ScrollState::set_scrolls_to_top`].
  scrolls_to_top: bool,
  /// See [`ScrollState::set_keyboard_dismissal`].
  keyboard_dismissal: KeyboardDismissal,
  /// See [`ScrollState::set_paging`].
  paging: bool,
  pending_scroll_x: Option<PendingScroll>,
  pending_scroll_y: Option<PendingScroll>,
  /// A wheel's scroll on its way along each axis.
  wheel_x: Option<WheelScroll>,
  wheel_y: Option<WheelScroll>,
  max_scroll_x: f32,
  max_scroll_y: f32,
  content_width: f32,
  content_height: f32,
  container_width: f32,
  container_height: f32,
  viewport_width: f32,
  viewport_height: f32,
  viewport_abs_x: f32,
  viewport_abs_y: f32,
  /// How far in from the viewport's left and top edges the bars start: the reach of what sticks at those edges,
  /// for a container whose bars keep clear of it (`scrollbars_clear_of_sticky`). Nothing for one that does not.
  bar_clear_x: f32,
  bar_clear_y: f32,
  /// The same at the right and the bottom edges: what sticks there, which the bars end before.
  bar_clear_end_x: f32,
  bar_clear_end_y: f32,
  /// Whether the bar across has a lane of its own under the content now (`reserve_bar_across`): the viewport
  /// ends above it, so the bar down needs no room of the corner kept from it.
  reserved_across: bool,
  /// Which ways the container scrolls, told at each layout: where it scrolls both and both bars are there, each
  /// track ends before the other's lane, so that the two keep out of the corner where they meet.
  scrolls: (bool, bool),
  /// Viewport top in layout space (the space of `ElementRef` positions), set
  /// when element refs are updated after layout — unlike `viewport_abs_y`,
  /// which is set while painting and lags one frame behind layout.
  layout_viewport_y: f32,
  /// Set by the first measurement of a layout pass; later measurements in the
  /// same pass (a flex parent re-laying out a shrunk or stretched child)
  /// restart from it, so the final measurement decides the offset.
  pass_start: Option<ScrollPassStart>,
  thumb_hovered: bool,
  /// The bar under the pointer, whose thumb is drawn hovered.
  hovered_bar: Option<ScrollAxis>,
  /// Whether the pointer is over the scroll container, anywhere in it.
  pointer_over: bool,
  /// Whether a [`ScrollBarVisibility::OnHover`] bar is shown.
  reveal: ScrollBarReveal,
  /// The offset the last painted frame showed, which tells a frame that the
  /// content scrolled; `None` before the first frame.
  painted_scroll: Option<(f32, f32)>,
  dragging: bool,
  /// The bar whose thumb is dragged, drawn pressed; `None` while the track is paged or nothing is held.
  drag_axis: Option<ScrollAxis>,
  drag_start_x: f32,
  drag_start_y: f32,
  drag_start_scroll_x: f32,
  drag_start_scroll_y: f32,
  scrollbar_style: ScrollBarStyle,
  scroll_dirty: bool,
  layout_subscribers: crate::core::layout_dirty::LayoutSubscribers,
  equal_overflow_scroll_probe_enabled: bool,
  /// Vertical scroll requests that took effect: `set_scroll` calls and
  /// pending requests (see [`PendingScroll::is_request`]) resolved by a
  /// layout pass. Lets a scroll anchor captured earlier tell that it stands
  /// for a position someone has since scrolled away from.
  scroll_y_requests: u64,
}

impl ScrollStateInner {
  /// Moves the offset to where a wheel's scroll on its way is at `now`. Tells whether it moved.
  fn step_wheel(&mut self, now: Instant) -> bool {
    let mut moved = false;
    if let Some(mut wheel) = self.wheel_x.take() {
      let (next, there) = wheel.step(self.scroll_x, self.max_scroll_x, now);
      if next != self.scroll_x {
        let by = next - self.scroll_x;
        self.pending_scroll_x = self.pending_scroll_x.map(|pending| pending.moved_by(by));
        self.scroll_x = next;
        moved = true;
      }
      self.wheel_x = (!there).then_some(wheel);
    }
    if let Some(mut wheel) = self.wheel_y.take() {
      let (next, there) = wheel.step(self.scroll_y, self.max_scroll_y, now);
      if next != self.scroll_y {
        let by = next - self.scroll_y;
        self.pending_scroll_y = self.pending_scroll_y.map(|pending| pending.moved_by(by));
        self.scroll_y = next;
        moved = true;
      }
      self.wheel_y = (!there).then_some(wheel);
    }
    if moved {
      self.scroll_dirty = true;
      self.layout_subscribers.mark();
    }
    moved
  }
}

impl ScrollState {
  /// Whether this is the state `other` is.
  pub(crate) fn same(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.inner, &other.inner)
  }
}

/// Where the track of `axis`'s bar starts in from the viewport's start, how long it is, and how long its thumb
/// is: what painting, hit testing and a drag of the thumb all go by. The track keeps the style's end insets and,
/// where the container asks, clear of what sticks at the start: it runs beside what scrolls, and its thumb is the
/// share of that which shows. `None` where there is no track.
fn bar_track(inner: &ScrollStateInner, axis: ScrollAxis, style: &ScrollBarStyle) -> Option<(f32, f32, f32)> {
  let (viewport, content, clear, clear_end) = match axis {
    ScrollAxis::Horizontal => (
      inner.viewport_width,
      inner.content_width,
      inner.bar_clear_x,
      inner.bar_clear_end_x,
    ),
    ScrollAxis::Vertical => (
      inner.viewport_height,
      inner.content_height,
      inner.bar_clear_y,
      inner.bar_clear_end_y,
    ),
  };
  // What sticks never takes the whole viewport from the bar.
  let (clear, clear_end) = if clear + clear_end < viewport {
    (clear, clear_end)
  } else {
    (0.0, 0.0)
  };
  // Where the bar across is there too, the track ends before its lane: the corner is no bar's.
  let (other_scrolls, other_content, other_viewport) = match axis {
    ScrollAxis::Horizontal => (inner.scrolls.1, inner.content_height, inner.viewport_height),
    ScrollAxis::Vertical => (inner.scrolls.0, inner.content_width, inner.viewport_width),
  };
  let other_shows = match style.visible {
    ScrollBarVisibility::Never => false,
    ScrollBarVisibility::Always => true,
    _ => other_content > other_viewport,
  };
  // A bar across in a lane of its own is under the viewport already: the bar down ends where the viewport does.
  let other_reserved = axis == ScrollAxis::Vertical && inner.reserved_across;
  let corner = if other_scrolls && other_shows && !other_reserved {
    style.width + style.resolved_edge_inset()
  } else {
    0.0
  };
  let start = clear + style.resolved_end_inset();
  let track = viewport - clear - clear_end - style.resolved_end_inset() * 2.0 - corner;
  if track <= 0.0 {
    return None;
  }
  let ratio = (viewport - clear - clear_end) / (content - clear - clear_end).max(1.0);
  let travel = thumb_travel(track, style);
  let thumb = (travel * ratio).max(style.min_thumb_length).min(travel);
  Some((start, track, thumb))
}

/// How far along a track of `track` its thumb may go: the track less the thumb's inset at each end. The thumb's
/// place, its drag and what takes the pointer all measure by it.
fn thumb_travel(track: f32, style: &ScrollBarStyle) -> f32 {
  (track - style.thumb_inset * 2.0).max(0.0)
}

impl Default for ScrollState {
  fn default() -> Self {
    Self::new()
  }
}

impl ScrollState {
  pub(crate) fn track_layout(&self, watch: &Arc<crate::core::layout_dirty::LayoutWatch>) {
    self.inner.lock().unwrap().layout_subscribers.bind(watch);
  }

  pub fn new() -> Self {
    Self {
      inner: Arc::new(Mutex::new(ScrollStateInner {
        scroll_x: 0.0,
        scroll_y: 0.0,
        overscroll_x: 0.0,
        overscroll_y: 0.0,
        scrolls_to_top: true,
        keyboard_dismissal: KeyboardDismissal::None,
        paging: false,
        pending_scroll_x: None,
        pending_scroll_y: None,
        wheel_x: None,
        wheel_y: None,
        max_scroll_x: 0.0,
        max_scroll_y: 0.0,
        content_width: 0.0,
        content_height: 0.0,
        container_width: 0.0,
        container_height: 0.0,
        viewport_width: 0.0,
        viewport_height: 0.0,
        viewport_abs_x: 0.0,
        viewport_abs_y: 0.0,
        bar_clear_x: 0.0,
        scrolls: (false, false),
        bar_clear_y: 0.0,
        bar_clear_end_x: 0.0,
        bar_clear_end_y: 0.0,
        reserved_across: false,
        layout_viewport_y: 0.0,
        pass_start: None,
        thumb_hovered: false,
        hovered_bar: None,
        pointer_over: false,
        reveal: ScrollBarReveal::default(),
        painted_scroll: None,
        dragging: false,
        drag_axis: None,
        drag_start_x: 0.0,
        drag_start_y: 0.0,
        drag_start_scroll_x: 0.0,
        drag_start_scroll_y: 0.0,
        scrollbar_style: ScrollBarStyle::default(),
        scroll_dirty: false,
        layout_subscribers: Default::default(),
        equal_overflow_scroll_probe_enabled: false,
        scroll_y_requests: 0,
      })),
    }
  }

  pub fn scroll_x(&self) -> f32 {
    self.inner.lock().unwrap().scroll_x
  }

  /// Whether a tap on a phone's status bar scrolls this container to its top; it does unless set otherwise.
  /// The tap scrolls one container: the outermost under the middle of the window that scrolls down, is away
  /// from its top and takes the tap. A screen with more than one such list turns it off on those the tap is
  /// not for.
  pub fn set_scrolls_to_top(&self, scrolls_to_top: bool) {
    self.inner.lock().unwrap().scrolls_to_top = scrolls_to_top;
  }

  pub fn scrolls_to_top(&self) -> bool {
    self.inner.lock().unwrap().scrolls_to_top
  }

  /// Whether a finger that starts to scroll this container puts a phone's on-screen keyboard away, by taking
  /// the focus from the text input being typed into; it does not unless set. A tap outside a text input leaves
  /// the keyboard up, so a screen of inputs that scrolls wants this, or a control that puts it away.
  pub fn set_dismisses_keyboard(&self, dismisses_keyboard: bool) {
    self.set_keyboard_dismissal(if dismisses_keyboard {
      KeyboardDismissal::OnDrag
    } else {
      KeyboardDismissal::None
    });
  }

  pub fn dismisses_keyboard(&self) -> bool {
    self.keyboard_dismissal() == KeyboardDismissal::OnDrag
  }

  /// What a finger that scrolls this container does to a phone's on-screen keyboard; see
  /// [`KeyboardDismissal`]. Nothing, unless set.
  pub fn set_keyboard_dismissal(&self, dismissal: KeyboardDismissal) {
    self.inner.lock().unwrap().keyboard_dismissal = dismissal;
  }

  pub fn keyboard_dismissal(&self) -> KeyboardDismissal {
    self.inner.lock().unwrap().keyboard_dismissal
  }

  /// Whether a finger scrolls this container a page at a time, a page being as long as the viewport: when
  /// the finger lifts the content goes on to the page it was thrown at, or back to the nearest, and never
  /// further than the page beside the one the finger started on. It does not unless set. A wheel and a
  /// scroll bar scroll as they always do.
  pub fn set_paging(&self, paging: bool) {
    self.inner.lock().unwrap().paging = paging;
  }

  pub fn paging(&self) -> bool {
    self.inner.lock().unwrap().paging
  }

  /// The page the content is at along each axis, counted from 0: the nearest, while it is between two.
  pub fn page(&self) -> (usize, usize) {
    let inner = self.inner.lock().unwrap();
    let page = |offset: f32, size: f32| {
      if size > 0.0 {
        (offset / size).round().max(0.0) as usize
      } else {
        0
      }
    };
    (
      page(inner.scroll_x, inner.viewport_width),
      page(inner.scroll_y, inner.viewport_height),
    )
  }

  /// How far the content is drawn past its ends, along each axis: negative before the start, positive beyond
  /// the end. The offset itself stays within the content.
  pub fn overscroll(&self) -> (f32, f32) {
    let inner = self.inner.lock().unwrap();
    (inner.overscroll_x, inner.overscroll_y)
  }

  /// Draws the content past its ends: what a touch shell does while a finger pulls a list beyond its end, and
  /// while the list springs back. Drawn in the next frame.
  pub fn set_overscroll(&self, x: f32, y: f32) {
    let mut inner = self.inner.lock().unwrap();
    if (inner.overscroll_x, inner.overscroll_y) == (x, y) {
      return;
    }
    (inner.overscroll_x, inner.overscroll_y) = (x, y);
    inner.scroll_dirty = true;
    inner.layout_subscribers.mark();
  }

  /// Where the content is laid out relative to the viewport: the scroll offset and the overscroll.
  pub(crate) fn content_offset(&self) -> crate::layout::offset::Offset {
    let inner = self.inner.lock().unwrap();
    crate::layout::offset::Offset::new(
      -(inner.scroll_x + inner.overscroll_x),
      -(inner.scroll_y + inner.overscroll_y),
    )
  }

  pub fn scroll_y(&self) -> f32 {
    self.inner.lock().unwrap().scroll_y
  }

  /// Scrolls to `(x, y)`, clamped to the content as last laid out. The new offset is drawn in the next frame,
  /// wherever the call comes from: an event handler, a timer or code outside any event. An offset the content is
  /// already at asks for no frame. A wheel's scroll on its way stops: the content stays where this puts it.
  pub fn set_scroll(&self, x: f32, y: f32) {
    self.stop_wheel();
    self.keep_scroll(x, y);
  }

  /// [`Self::set_scroll`] for an offset that keeps what is shown where it is, after the content above it
  /// changed: a wheel's scroll on its way goes on from there.
  pub(crate) fn keep_scroll(&self, x: f32, y: f32) {
    let mut inner = self.inner.lock().unwrap();
    let offset = (x.clamp(0.0, inner.max_scroll_x), y.clamp(0.0, inner.max_scroll_y));
    inner.scroll_y_requests = inner.scroll_y_requests.wrapping_add(1);
    if offset == (inner.scroll_x, inner.scroll_y) {
      return;
    }
    (inner.scroll_x, inner.scroll_y) = offset;
    inner.scroll_dirty = true;
    inner.layout_subscribers.mark();
  }

  /// Queue an absolute scroll offset to resolve after the next layout measurement, which the queued offset asks
  /// for, wherever the call comes from. Queuing the offset the content is already at, with no other scroll
  /// queued, asks for no layout or frame: it resolves in the next layout that comes.
  pub fn set_scroll_pending(&self, x: f32, y: f32) {
    let mut inner = self.inner.lock().unwrap();
    (inner.wheel_x, inner.wheel_y) = (None, None);
    let offset = (x.max(0.0), y.max(0.0));
    let already_there = inner.pending_scroll_x.is_none()
      && inner.pending_scroll_y.is_none()
      && offset == (inner.scroll_x, inner.scroll_y);
    inner.pending_scroll_x = Some(PendingScroll::Offset(offset.0));
    inner.pending_scroll_y = Some(PendingScroll::Offset(offset.1));
    if already_there {
      return;
    }
    inner.scroll_dirty = true;
    inner.layout_subscribers.mark();
  }

  /// Queue the smallest vertical scroll that shows content rows
  /// `start..end`, resolved after the next layout measurement.
  pub(crate) fn reveal_y_pending(&self, start: f32, end: f32) {
    let mut inner = self.inner.lock().unwrap();
    inner.wheel_y = None;
    inner.pending_scroll_y = Some(PendingScroll::Reveal { start, end });
    inner.scroll_dirty = true;
    inner.layout_subscribers.mark();
  }

  /// Queue the vertical offset that keeps the shown content in place once the
  /// content above it changed height, resolved after the next layout
  /// measurement. Not a scroll request: a request already queued wins, and
  /// it does not count in `scroll_y_requests`.
  pub(crate) fn keep_scroll_y_pending(&self, y: f32) {
    let mut inner = self.inner.lock().unwrap();
    if inner.pending_scroll_y.is_some_and(PendingScroll::is_request) {
      return;
    }
    inner.pending_scroll_y = Some(PendingScroll::Anchor(y.max(0.0)));
    inner.scroll_dirty = true;
    inner.layout_subscribers.mark();
  }

  /// Queue a vertical scroll to the top after the next layout measurement.
  pub fn scroll_to_top_pending(&self) {
    let mut inner = self.inner.lock().unwrap();
    inner.wheel_y = None;
    inner.pending_scroll_y = Some(PendingScroll::Start);
    inner.scroll_dirty = true;
    inner.layout_subscribers.mark();
  }

  /// Queue a vertical scroll to the bottom after the next layout measurement.
  pub fn scroll_to_bottom_pending(&self) {
    let mut inner = self.inner.lock().unwrap();
    inner.wheel_y = None;
    inner.pending_scroll_y = Some(PendingScroll::End);
    inner.scroll_dirty = true;
    inner.layout_subscribers.mark();
  }

  /// Whether a scroll to the bottom (`scroll_to_bottom_pending`, a stick to
  /// the bottom) waits for the next layout measurement.
  pub(crate) fn opens_at_end(&self) -> bool {
    matches!(self.inner.lock().unwrap().pending_scroll_y, Some(PendingScroll::End))
  }

  /// The vertical offset a queued `set_scroll_pending` waits to land on.
  pub(crate) fn pending_scroll_y_offset(&self) -> Option<f32> {
    match self.inner.lock().unwrap().pending_scroll_y {
      Some(PendingScroll::Offset(offset)) => Some(offset),
      _ => None,
    }
  }

  /// Drops a queued vertical scroll when it is to `offset` (a
  /// `set_scroll_pending` whose reason is gone); any other request stays.
  pub(crate) fn drop_pending_scroll_y_offset(&self, offset: f32) {
    let mut inner = self.inner.lock().unwrap();
    if matches!(inner.pending_scroll_y, Some(PendingScroll::Offset(pending)) if pending == offset) {
      inner.pending_scroll_y = None;
    }
  }

  /// Whether a vertical scroll request (an offset, an edge or a reveal, not a
  /// prepend anchor) waits for the next layout measurement.
  pub(crate) fn has_pending_scroll_y_request(&self) -> bool {
    self
      .inner
      .lock()
      .unwrap()
      .pending_scroll_y
      .is_some_and(PendingScroll::is_request)
  }

  /// The vertical offset the next layout measurement settles on when it
  /// measures `content_height` of content in the current viewport, as
  /// [`Self::update_layout_with_container`] resolves it: a pending scroll
  /// lands, an offset at the bottom stays at the bottom, any other keeps its
  /// place within the new range.
  pub(crate) fn next_layout_scroll_y(&self, content_height: f32) -> f32 {
    let inner = self.inner.lock().unwrap();
    let max_scroll = (content_height - inner.viewport_height).max(0.0);
    let was_at_bottom =
      inner.max_scroll_y > 0.0 && inner.max_scroll_y - inner.scroll_y <= SCROLL_END_EPSILON && inner.wheel_y.is_none();
    let target = match inner.pending_scroll_y {
      Some(pending) => resolve_pending_scroll(
        pending,
        content_height,
        max_scroll,
        inner.scroll_y,
        inner.viewport_height,
      ),
      None if was_at_bottom => max_scroll,
      None => inner.scroll_y,
    };
    target.clamp(0.0, max_scroll)
  }

  /// Count of vertical scroll requests that took effect; see
  /// `ScrollStateInner::scroll_y_requests`.
  pub(crate) fn scroll_y_requests(&self) -> u64 {
    self.inner.lock().unwrap().scroll_y_requests
  }

  pub(crate) fn equal_overflow_scroll_probe_enabled(&self) -> bool {
    self.inner.lock().unwrap().equal_overflow_scroll_probe_enabled
  }

  /// Queue a horizontal scroll to the left edge after the next layout measurement.
  pub fn scroll_to_left_pending(&self) {
    let mut inner = self.inner.lock().unwrap();
    inner.wheel_x = None;
    inner.pending_scroll_x = Some(PendingScroll::Start);
    inner.scroll_dirty = true;
    inner.layout_subscribers.mark();
  }

  /// Queue a horizontal scroll to the right edge after the next layout measurement.
  pub fn scroll_to_right_pending(&self) {
    let mut inner = self.inner.lock().unwrap();
    inner.wheel_x = None;
    inner.pending_scroll_x = Some(PendingScroll::End);
    inner.scroll_dirty = true;
    inner.layout_subscribers.mark();
  }

  /// Queue a bottom scroll only when the current vertical offset is within `threshold` of the bottom.
  pub fn stick_to_bottom_if_near_end(&self, threshold: f32) -> bool {
    let mut inner = self.inner.lock().unwrap();
    if inner.max_scroll_y - inner.scroll_y <= threshold.max(0.0) {
      inner.wheel_y = None;
      inner.pending_scroll_y = Some(PendingScroll::End);
      inner.scroll_dirty = true;
      inner.layout_subscribers.mark();
      true
    } else {
      false
    }
  }

  /// Queue a right-edge scroll only when the current horizontal offset is within `threshold` of the right edge.
  pub fn stick_to_right_if_near_end(&self, threshold: f32) -> bool {
    let mut inner = self.inner.lock().unwrap();
    if inner.max_scroll_x - inner.scroll_x <= threshold.max(0.0) {
      inner.wheel_x = None;
      inner.pending_scroll_x = Some(PendingScroll::End);
      inner.scroll_dirty = true;
      inner.layout_subscribers.mark();
      true
    } else {
      false
    }
  }

  /// Preserve the current vertical viewport position when content is prepended before the next layout.
  pub fn preserve_prepend_anchor_pending(&self) {
    let mut inner = self.inner.lock().unwrap();
    inner.pending_scroll_y = Some(PendingScroll::PreservePrependAnchor {
      previous_content: inner.content_height,
      previous_scroll: inner.scroll_y,
    });
    inner.scroll_dirty = true;
    inner.layout_subscribers.mark();
  }

  /// Preserve the current horizontal viewport position when content is prepended before the next layout.
  pub fn preserve_horizontal_prepend_anchor_pending(&self) {
    let mut inner = self.inner.lock().unwrap();
    inner.pending_scroll_x = Some(PendingScroll::PreservePrependAnchor {
      previous_content: inner.content_width,
      previous_scroll: inner.scroll_x,
    });
    inner.scroll_dirty = true;
    inner.layout_subscribers.mark();
  }

  /// Scrolls by `(dx, dy)` at once, as far as the content goes. A wheel's scroll on its way along an axis that
  /// is scrolled stops: the content moves from where it is drawn.
  pub fn scroll_by(&self, dx: f32, dy: f32) {
    self.scroll_by_with_overflow(dx, dy);
  }

  /// [`Self::scroll_by`], telling how much of each delta the content could not take.
  pub fn scroll_by_with_overflow(&self, dx: f32, dy: f32) -> (f32, f32) {
    let mut inner = self.inner.lock().unwrap();
    if dx != 0.0 {
      inner.pending_scroll_x = None;
      inner.wheel_x = None;
    }
    if dy != 0.0 {
      inner.pending_scroll_y = None;
      inner.wheel_y = None;
    }

    let old_x = inner.scroll_x;
    let old_y = inner.scroll_y;
    inner.scroll_x = (inner.scroll_x + dx).clamp(0.0, inner.max_scroll_x);
    inner.scroll_y = (inner.scroll_y + dy).clamp(0.0, inner.max_scroll_y);

    let consumed_x = inner.scroll_x - old_x;
    let consumed_y = inner.scroll_y - old_y;
    if consumed_x != 0.0 || consumed_y != 0.0 {
      inner.scroll_dirty = true;
      inner.layout_subscribers.mark();
    }

    (dx - consumed_x, dy - consumed_y)
  }

  /// Where the offset is going: where a wheel's scroll on its way leaves it
  /// ([`Tree::wheel`](crate::app::Tree::wheel)), and else where it is. [`Self::scroll_x`] and
  /// [`Self::scroll_y`] are the offset drawn now.
  pub fn scroll_target(&self) -> (f32, f32) {
    let inner = self.inner.lock().unwrap();
    (
      inner.wheel_x.map_or(inner.scroll_x, |wheel| wheel.target),
      inner.wheel_y.map_or(inner.scroll_y, |wheel| wheel.target),
    )
  }

  /// A mouse's wheel turned by `(dx, dy)` at `now`: the offset goes there over [`WHEEL_SCROLL_DURATION`], a
  /// step with every [`Self::step_wheel`], instead of at once. Tells how much of each delta the content could
  /// not take, as [`Self::scroll_by_with_overflow`] does: that is decided on where the offset is going, so a
  /// container on its way to its end takes no more.
  ///
  /// A delta adds to where the scroll was already going, and the way there starts again from where the offset
  /// is at `now`: one the other way turns it round at once. A scroll someone asked for that waits for a layout
  /// is dropped, as a scroll at once drops it; a place kept is still kept.
  pub(crate) fn wheel_by_with_overflow(&self, dx: f32, dy: f32, now: Instant) -> (f32, f32) {
    let mut inner = self.inner.lock().unwrap();
    // The offset is where it is at `now` before the wheel turns it. The tree has stepped it there already and
    // told its container (`Tree::wheel`).
    inner.step_wheel(now);
    if dx != 0.0 && inner.pending_scroll_x.is_some_and(PendingScroll::is_request) {
      inner.pending_scroll_x = None;
    }
    if dy != 0.0 && inner.pending_scroll_y.is_some_and(PendingScroll::is_request) {
      inner.pending_scroll_y = None;
    }
    let (wheel_x, taken_x) = WheelScroll::turned(inner.wheel_x, inner.scroll_x, inner.max_scroll_x, dx, now);
    let (wheel_y, taken_y) = WheelScroll::turned(inner.wheel_y, inner.scroll_y, inner.max_scroll_y, dy, now);
    (inner.wheel_x, inner.wheel_y) = (wheel_x, wheel_y);
    (dx - taken_x, dy - taken_y)
  }

  /// Whether a wheel's scroll is on its way.
  pub(crate) fn wheel_scrolling(&self) -> bool {
    let inner = self.inner.lock().unwrap();
    inner.wheel_x.is_some() || inner.wheel_y.is_some()
  }

  /// Moves the offset to where a wheel's scroll on its way is at `now`. Tells whether it moved.
  pub(crate) fn step_wheel(&self, now: Instant) -> bool {
    self.inner.lock().unwrap().step_wheel(now)
  }

  /// Ends a wheel's scroll on its way where it was going, at once. Tells whether the offset moved.
  pub(crate) fn end_wheel(&self) -> bool {
    let mut inner = self.inner.lock().unwrap();
    let Some(turned) = [inner.wheel_x, inner.wheel_y]
      .into_iter()
      .flatten()
      .map(|wheel| wheel.turned)
      .max()
    else {
      return false;
    };
    inner.step_wheel(turned + WHEEL_SCROLL_DURATION)
  }

  /// Stops a wheel's scroll on its way where the offset is.
  pub(crate) fn stop_wheel(&self) {
    let mut inner = self.inner.lock().unwrap();
    (inner.wheel_x, inner.wheel_y) = (None, None);
  }

  pub fn content_width(&self) -> f32 {
    self.inner.lock().unwrap().content_width
  }

  pub fn content_height(&self) -> f32 {
    self.inner.lock().unwrap().content_height
  }

  pub fn viewport_width(&self) -> f32 {
    self.inner.lock().unwrap().viewport_width
  }

  pub fn viewport_height(&self) -> f32 {
    self.inner.lock().unwrap().viewport_height
  }

  /// Whether two handles share the same underlying scroll state.
  pub(crate) fn ptr_eq(&self, other: &ScrollState) -> bool {
    Arc::ptr_eq(&self.inner, &other.inner)
  }

  pub fn style(&self) -> ScrollBarStyle {
    self.inner.lock().unwrap().scrollbar_style.clone()
  }

  pub fn set_style(&self, style: ScrollBarStyle) {
    self.inner.lock().unwrap().scrollbar_style = style;
  }

  pub fn is_thumb_hovered(&self) -> bool {
    self.inner.lock().unwrap().thumb_hovered
  }

  pub fn is_dragging(&self) -> bool {
    self.inner.lock().unwrap().dragging
  }

  /// The bar under the pointer, whose thumb is drawn in [`ScrollBarStyle::thumb_hovered_color`].
  pub fn hovered_bar(&self) -> Option<ScrollAxis> {
    self.inner.lock().unwrap().hovered_bar
  }

  /// The bar whose thumb is dragged, drawn in [`ScrollBarStyle::thumb_pressed_color`].
  pub fn dragged_bar(&self) -> Option<ScrollAxis> {
    let inner = self.inner.lock().unwrap();
    inner.drag_axis.filter(|_| inner.dragging)
  }

  /// Sets the bar under the pointer, and with it [`Self::is_thumb_hovered`].
  pub(crate) fn set_hovered_bar(&self, bar: Option<ScrollAxis>) {
    let mut inner = self.inner.lock().unwrap();
    inner.hovered_bar = bar;
    inner.thumb_hovered = bar.is_some();
  }

  pub fn set_thumb_hovered(&self, hovered: bool) {
    let mut inner = self.inner.lock().unwrap();
    inner.thumb_hovered = hovered;
    if !hovered {
      inner.hovered_bar = None;
    }
  }

  /// Records whether the pointer is over the scroll container, which holds
  /// a [`ScrollBarVisibility::OnHover`] bar shown.
  pub(crate) fn set_pointer_over(&self, over: bool) {
    self.inner.lock().unwrap().pointer_over = over;
  }

  /// What the bars styled `style` show in the frame painted at `now`. A bar
  /// of any visibility but [`ScrollBarVisibility::OnHover`] is fully shown
  /// and never changes by itself.
  pub(crate) fn reveal_scrollbar(&self, style: &ScrollBarStyle, now: Instant) -> ScrollBarRevealFrame {
    let mut inner = self.inner.lock().unwrap();
    // Content pulled past an end moves as content that scrolls does.
    let offset = (inner.scroll_x + inner.overscroll_x, inner.scroll_y + inner.overscroll_y);
    let scrolled = inner
      .painted_scroll
      .replace(offset)
      .is_some_and(|painted| painted != offset);
    if !style.visible.fades() {
      inner.reveal = ScrollBarReveal::default();
      return ScrollBarRevealFrame {
        opacity: 1.0,
        next_change: None,
      };
    }
    // A finger's indicator is shown by content that moves, not by a pointer over it.
    let held = (style.visible == ScrollBarVisibility::OnHover && inner.pointer_over) || inner.dragging;
    let overflows = inner.content_width > inner.viewport_width || inner.content_height > inner.viewport_height;
    inner.reveal.advance(style, now, held, scrolled, overflows)
  }

  pub fn begin_drag(&self, mouse_y: f32) {
    self.begin_drag_axis(ScrollAxis::Vertical, 0.0, mouse_y);
  }

  pub fn begin_drag_axis(&self, axis: ScrollAxis, mouse_x: f32, mouse_y: f32) {
    let mut inner = self.inner.lock().unwrap();
    // The thumb takes the content from where it is drawn: a wheel's scroll on its way stops.
    match axis {
      ScrollAxis::Horizontal => (inner.pending_scroll_x, inner.wheel_x) = (None, None),
      ScrollAxis::Vertical => (inner.pending_scroll_y, inner.wheel_y) = (None, None),
    }
    inner.dragging = true;
    inner.drag_axis = Some(axis);
    inner.drag_start_x = mouse_x;
    inner.drag_start_y = mouse_y;
    inner.drag_start_scroll_x = inner.scroll_x;
    inner.drag_start_scroll_y = inner.scroll_y;
  }

  pub fn end_drag(&self) {
    let mut inner = self.inner.lock().unwrap();
    inner.dragging = false;
    inner.drag_axis = None;
  }

  pub fn drag_to(&self, mouse_y: f32, style: &crate::layout::scrollbar::ScrollBarStyle) {
    self.drag_to_axis(ScrollAxis::Vertical, 0.0, mouse_y, style);
  }

  pub fn drag_to_axis(
    &self,
    axis: ScrollAxis,
    mouse_x: f32,
    mouse_y: f32,
    style: &crate::layout::scrollbar::ScrollBarStyle,
  ) {
    let mut inner = self.inner.lock().unwrap();
    if !inner.dragging {
      return;
    }

    match axis {
      ScrollAxis::Horizontal => {
        let Some((_, track_width, thumb_width)) = bar_track(&inner, ScrollAxis::Horizontal, style) else {
          return;
        };
        let scrollable_track = thumb_travel(track_width, style) - thumb_width;
        if scrollable_track <= 0.0 {
          return;
        }

        let delta_px = mouse_x - inner.drag_start_x;
        let scroll_delta = delta_px / scrollable_track * inner.max_scroll_x;
        inner.wheel_x = None;
        inner.scroll_x = (inner.drag_start_scroll_x + scroll_delta).clamp(0.0, inner.max_scroll_x);
      }
      ScrollAxis::Vertical => {
        let Some((_, track_height, thumb_height)) = bar_track(&inner, ScrollAxis::Vertical, style) else {
          return;
        };
        let scrollable_track = thumb_travel(track_height, style) - thumb_height;
        if scrollable_track <= 0.0 {
          return;
        }

        let delta_px = mouse_y - inner.drag_start_y;
        let scroll_delta = delta_px / scrollable_track * inner.max_scroll_y;
        inner.wheel_y = None;
        inner.scroll_y = (inner.drag_start_scroll_y + scroll_delta).clamp(0.0, inner.max_scroll_y);
      }
    }
    inner.scroll_dirty = true;
    inner.layout_subscribers.mark();
  }

  pub fn thumb_rect(&self, style: &crate::layout::scrollbar::ScrollBarStyle) -> (f32, f32, f32, f32) {
    let inner = self.inner.lock().unwrap();
    let track_x = inner.viewport_abs_x + inner.viewport_width - style.width - style.resolved_edge_inset();
    let (start, track_height, thumb_height) =
      bar_track(&inner, ScrollAxis::Vertical, style).unwrap_or((style.resolved_end_inset(), 0.0, 0.0));
    let track_y = inner.viewport_abs_y + start;
    let scroll_ratio = if inner.max_scroll_y > 0.0 {
      inner.scroll_y / inner.max_scroll_y
    } else {
      0.0
    };
    let inset = style.thumb_inset;
    let thumb_y = track_y + inset + (thumb_travel(track_height, style) - thumb_height) * scroll_ratio;

    (track_x + inset, thumb_y, style.width - inset * 2.0, thumb_height)
  }

  pub fn thumb_rect_for_axis(
    &self,
    axis: ScrollAxis,
    style: &crate::layout::scrollbar::ScrollBarStyle,
  ) -> Option<(f32, f32, f32, f32)> {
    let geo = self.scrollbar_geometry_for_axis(axis, style)?;
    Some((geo.thumb_x, geo.thumb_y, geo.thumb_width, geo.thumb_height))
  }

  pub fn scrollbar_geometry_for_axis(
    &self,
    axis: ScrollAxis,
    style: &crate::layout::scrollbar::ScrollBarStyle,
  ) -> Option<ScrollBarGeometry> {
    let inner = self.inner.lock().unwrap();
    match (style.visible, axis) {
      (ScrollBarVisibility::Never, _) => return None,
      (
        ScrollBarVisibility::Auto | ScrollBarVisibility::OnHover | ScrollBarVisibility::WhileScrolling,
        ScrollAxis::Horizontal,
      ) if inner.content_width <= inner.viewport_width => {
        return None;
      }
      (
        ScrollBarVisibility::Auto | ScrollBarVisibility::OnHover | ScrollBarVisibility::WhileScrolling,
        ScrollAxis::Vertical,
      ) if inner.content_height <= inner.viewport_height => {
        return None;
      }
      _ => {}
    }

    match axis {
      ScrollAxis::Horizontal => {
        let (start, track_width, thumb_width) = bar_track(&inner, ScrollAxis::Horizontal, style)?;
        let track_x = inner.viewport_abs_x + start;
        let track_y = inner.viewport_abs_y + inner.container_height - style.width - style.resolved_edge_inset();
        let track_height = style.width;
        if track_height <= 0.0 {
          return None;
        }
        let max_scroll = (inner.content_width - inner.viewport_width).max(0.0);
        let scroll_ratio = if max_scroll > 0.0 {
          inner.scroll_x / max_scroll
        } else {
          0.0
        };
        let inset = style.thumb_inset;
        let thickness = track_height - inset * 2.0;
        let thumb_x = track_x + inset + (thumb_travel(track_width, style) - thumb_width) * scroll_ratio;
        let (thumb_x, thumb_width) = squeezed_thumb(thumb_x, thumb_width, inner.overscroll_x, thickness);

        Some(ScrollBarGeometry {
          track_x,
          track_y,
          track_width,
          track_height,
          thumb_x,
          thumb_y: track_y + inset,
          thumb_width,
          thumb_height: thickness,
        })
      }
      ScrollAxis::Vertical => {
        let (start, track_height, thumb_height) = bar_track(&inner, ScrollAxis::Vertical, style)?;
        let track_x = inner.viewport_abs_x + inner.container_width - style.width - style.resolved_edge_inset();
        let track_y = inner.viewport_abs_y + start;
        let track_width = style.width;
        if track_width <= 0.0 {
          return None;
        }
        let max_scroll = (inner.content_height - inner.viewport_height).max(0.0);
        let scroll_ratio = if max_scroll > 0.0 {
          inner.scroll_y / max_scroll
        } else {
          0.0
        };
        let inset = style.thumb_inset;
        let thickness = track_width - inset * 2.0;
        let thumb_y = track_y + inset + (thumb_travel(track_height, style) - thumb_height) * scroll_ratio;
        let (thumb_y, thumb_height) = squeezed_thumb(thumb_y, thumb_height, inner.overscroll_y, thickness);

        Some(ScrollBarGeometry {
          track_x,
          track_y,
          track_width,
          track_height,
          thumb_x: track_x + inset,
          thumb_y,
          thumb_width: thickness,
          thumb_height,
        })
      }
    }
  }

  /// The part of `axis`'s scrollbar that takes the pointer at `(x, y)` in
  /// window coordinates (see [`ScrollBarGeometry::hit_part_at`]), or `None`
  /// where the pointer belongs to the content. A bar shown while the content
  /// fits along its axis (`ScrollBarVisibility::Always`) cannot scroll
  /// anything, so it takes no pointer, and neither does a
  /// `ScrollBarVisibility::OnHover` bar that the last frame did not paint.
  pub(crate) fn scrollbar_part_at(
    &self,
    axis: ScrollAxis,
    style: &ScrollBarStyle,
    x: f32,
    y: f32,
  ) -> Option<ScrollBarPart> {
    let overflows = match axis {
      ScrollAxis::Horizontal => self.content_width() > self.viewport_width(),
      ScrollAxis::Vertical => self.content_height() > self.viewport_height(),
    };
    if !overflows {
      return None;
    }
    if style.visible.fades() && !self.inner.lock().unwrap().reveal.is_shown() {
      return None;
    }
    self
      .scrollbar_geometry_for_axis(axis, style)?
      .hit_part_at(axis == ScrollAxis::Horizontal, style, x, y)
  }

  /// Scrolls one viewport along `axis`, forward or back, clamped to the
  /// content: what a press on the scrollbar track does.
  pub(crate) fn page_axis(&self, axis: ScrollAxis, forward: bool) {
    let page = match axis {
      ScrollAxis::Horizontal => self.viewport_width(),
      ScrollAxis::Vertical => self.viewport_height(),
    };
    let delta = if forward { page } else { -page };
    match axis {
      ScrollAxis::Horizontal => self.scroll_by(delta, 0.0),
      ScrollAxis::Vertical => self.scroll_by(0.0, delta),
    }
  }

  pub(crate) fn take_scroll_dirty(&self) -> bool {
    let mut inner = self.inner.lock().unwrap();
    let dirty = inner.scroll_dirty;
    inner.scroll_dirty = false;
    dirty
  }

  pub(crate) fn has_scroll_dirty(&self) -> bool {
    self.inner.lock().unwrap().scroll_dirty
  }

  /// How far in from the viewport's left and top edges its bars start: the reach of what sticks at those
  /// edges, measured at each layout of a container whose bars keep clear of it, and nothing for one whose bars
  /// do not.
  /// Tells the state whether its bar across has a lane of its own under the content now.
  pub(crate) fn set_reserved_across(&self, reserved: bool) {
    self.inner.lock().unwrap().reserved_across = reserved;
  }

  /// Tells the state which ways its container scrolls, across and down.
  pub(crate) fn set_scrolls(&self, across: bool, down: bool) {
    self.inner.lock().unwrap().scrolls = (across, down);
  }

  pub(crate) fn set_bar_clearance(&self, x: f32, y: f32, end_x: f32, end_y: f32) {
    let mut inner = self.inner.lock().unwrap();
    inner.bar_clear_x = x.max(0.0);
    inner.bar_clear_y = y.max(0.0);
    inner.bar_clear_end_x = end_x.max(0.0);
    inner.bar_clear_end_y = end_y.max(0.0);
  }

  /// Record one measurement of the scroll container. A layout pass can
  /// measure a container more than once (a flex parent re-lays out a child it
  /// shrank or stretched); every measurement resolves the pending scroll and
  /// the stick-to-end behaviour from the state the pass started with, so only
  /// the final measurement counts. [`Self::finish_layout_pass`] then drops the
  /// resolved pending scroll.
  pub(crate) fn update_layout_with_container(
    &self,
    content_w: f32,
    content_h: f32,
    viewport_w: f32,
    viewport_h: f32,
    container_w: f32,
    container_h: f32,
  ) {
    let mut inner = self.inner.lock().unwrap();
    let start = inner.pass_start.unwrap_or(ScrollPassStart {
      scroll_x: inner.scroll_x,
      scroll_y: inner.scroll_y,
      max_scroll_x: inner.max_scroll_x,
      max_scroll_y: inner.max_scroll_y,
    });
    inner.pass_start = Some(start);
    // An offset at the end stays at the end when the content grows. One a wheel's scroll is moving is the
    // scroll's: near the end it is leaving it, or on its way to it (`WheelScroll::to_end`).
    let was_at_right =
      start.max_scroll_x > 0.0 && start.max_scroll_x - start.scroll_x <= SCROLL_END_EPSILON && inner.wheel_x.is_none();
    let was_at_bottom =
      start.max_scroll_y > 0.0 && start.max_scroll_y - start.scroll_y <= SCROLL_END_EPSILON && inner.wheel_y.is_none();

    inner.content_width = content_w;
    inner.content_height = content_h;
    inner.container_width = container_w;
    inner.container_height = container_h;
    inner.viewport_width = viewport_w.max(0.0);
    inner.viewport_height = viewport_h.max(0.0);
    inner.max_scroll_x = (content_w - inner.viewport_width).max(0.0);
    inner.max_scroll_y = (content_h - inner.viewport_height).max(0.0);
    inner.scroll_x = match inner.pending_scroll_x {
      Some(pending) => resolve_pending_scroll(
        pending,
        inner.content_width,
        inner.max_scroll_x,
        start.scroll_x,
        inner.viewport_width,
      ),
      None if was_at_right => inner.max_scroll_x,
      None => start.scroll_x,
    };
    inner.scroll_y = match inner.pending_scroll_y {
      Some(pending) => resolve_pending_scroll(
        pending,
        inner.content_height,
        inner.max_scroll_y,
        start.scroll_y,
        inner.viewport_height,
      ),
      None if was_at_bottom => inner.max_scroll_y,
      None => start.scroll_y,
    };
    inner.scroll_x = inner.scroll_x.clamp(0.0, inner.max_scroll_x);
    inner.scroll_y = inner.scroll_y.clamp(0.0, inner.max_scroll_y);
  }

  /// End the current layout pass: the last measurement's offset stands and
  /// the pending scroll it resolved is consumed. No-op when the container was
  /// not measured in this pass (its pending scroll keeps waiting).
  pub(crate) fn finish_layout_pass(&self) {
    let mut inner = self.inner.lock().unwrap();
    if inner.pass_start.take().is_some() {
      inner.pending_scroll_x = None;
      if inner.pending_scroll_y.take().is_some_and(PendingScroll::is_request) {
        inner.scroll_y_requests = inner.scroll_y_requests.wrapping_add(1);
      }
    }
  }

  /// Record the layout of a multiline text input, which scrolls its own text:
  /// its box, `viewport_width` by `viewport_height`, showing `content_height`
  /// of text and padding from `scroll_y`, clamped to that content. The box is
  /// the input's whole box, as for any scroll container, so the bar is placed
  /// from the box's edges; the padding is part of what scrolls. Unlike
  /// [`Self::update_layout_with_container`], no pending scroll or
  /// stick-to-end applies: the input decides its offset itself.
  pub(crate) fn set_text_layout(&self, viewport_width: f32, viewport_height: f32, content_height: f32, scroll_y: f32) {
    let mut inner = self.inner.lock().unwrap();
    inner.viewport_width = viewport_width.max(0.0);
    inner.viewport_height = viewport_height.max(0.0);
    inner.container_width = inner.viewport_width;
    inner.container_height = inner.viewport_height;
    inner.content_width = inner.viewport_width;
    inner.content_height = if content_height - inner.viewport_height < TEXT_OVERFLOW_EPSILON {
      content_height.min(inner.viewport_height)
    } else {
      content_height
    };
    inner.max_scroll_x = 0.0;
    inner.max_scroll_y = (inner.content_height - inner.viewport_height).max(0.0);
    inner.scroll_x = 0.0;
    inner.scroll_y = scroll_y.clamp(0.0, inner.max_scroll_y);
  }

  pub(crate) fn set_layout_viewport_y(&self, y: f32) {
    self.inner.lock().unwrap().layout_viewport_y = y;
  }

  /// Viewport top in layout space; see `ScrollStateInner::layout_viewport_y`.
  pub(crate) fn layout_viewport_y(&self) -> f32 {
    self.inner.lock().unwrap().layout_viewport_y
  }

  pub(crate) fn set_viewport_position(&self, x: f32, y: f32) {
    let mut inner = self.inner.lock().unwrap();
    inner.viewport_abs_x = x;
    inner.viewport_abs_y = y;
  }
}

fn resolve_pending_scroll(
  pending: PendingScroll,
  content_size: f32,
  max_scroll: f32,
  current: f32,
  viewport: f32,
) -> f32 {
  match pending {
    PendingScroll::Offset(offset) | PendingScroll::Anchor(offset) => offset,
    PendingScroll::Start => 0.0,
    PendingScroll::End => max_scroll,
    PendingScroll::PreservePrependAnchor {
      previous_content,
      previous_scroll,
    } => previous_scroll + (content_size - previous_content),
    PendingScroll::Reveal { start, end } => {
      if start < current || end - start > viewport {
        start
      } else if end > current + viewport {
        end - viewport
      } else {
        current
      }
    }
  }
}

#[derive(Clone, Copy, PartialEq)]
pub struct FlexParams {
  pub grow: f32,
  pub shrink: f32,
  pub basis: Option<f32>,
}

impl FlexParams {
  pub fn grow(factor: f32) -> Self {
    Self {
      grow: factor,
      shrink: 0.0,
      basis: None,
    }
  }
}

impl Default for FlexParams {
  fn default() -> Self {
    Self {
      grow: DEFAULT_FLEX_GROW,
      shrink: DEFAULT_FLEX_SHRINK,
      basis: None,
    }
  }
}

/// How far a shrinking child of a single-line `Row`/`Column` gives way when
/// the line overflows. Set with `.shrink_limit(...)`; it applies only to a
/// child with a `flex_shrink` factor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ShrinkLimit {
  /// Shrink down to the child's `min_width`/`min_height` (0 without one).
  #[default]
  MinSize,
  /// Shrink down to the child's content minimum, and never below an explicit
  /// `min_width`/`min_height`. The content minimum of a `Row` in a row (or
  /// a `Column` in a column) is its padding and spacing, the natural size of
  /// every child that does not shrink and the limit of every child that does.
  /// The content minimum of a line on the other axis (a `Column` in a row)
  /// is the widest of its children's: a child with an explicit
  /// `min_width`/`min_height` counts at it, so a text that may wrap says
  /// `min_width(0.0)`. Anything else (text, rect, stack, scroll container,
  /// wrapping line) keeps its natural size, like a flex item with CSS
  /// `min-width: auto`.
  Content,
  /// [`Content`](Self::Content) first, then [`MinSize`](Self::MinSize): the
  /// child shrinks down to its content minimum while the children of its
  /// order can take the overflow above their limits, and when they cannot,
  /// it gives way below it down to its `min_width`/`min_height`, the last
  /// such child of the order first. Its own line is then laid out as a
  /// collapsed line: what in it has this limit gives way below its content
  /// too. Steps whose descriptions wrap before any title is cut.
  ContentFirst,
  /// Keep the natural size or drop out of the line: a dropped child is laid
  /// out at zero size, takes no spacing, and is neither drawn nor hit.
  /// Nothing inside it is a Tab stop or takes focus, focus inside it is
  /// blurred, keys never activate it, and its element refs report a
  /// zero-size rect where the next child starts. It stays mounted and keeps
  /// its state.
  Drop,
}

/// When and how far a shrinking flex child gives way: a node's
/// `.shrink_order(...)`, `.shrink_limit(...)` and `.shrink_drop_below(...)`;
/// and whether a child stands for what its line dropped (`.drop_marker(...)`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct ShrinkRule {
  pub(crate) order: i32,
  pub(crate) limit: ShrinkLimit,
  /// Shrink no further than this main size, then drop (see
  /// `.shrink_drop_below`).
  pub(crate) drop_below: Option<f32>,
  /// Takes room only while another child of the line is dropped (see
  /// `.drop_marker`).
  pub(crate) marker: bool,
}

/// One end of a [`SizeBounds`] axis: the size it is at, and whether that size is itself within.
type Bound = Option<(f32, bool)>;

/// The sizes of a `Stack` at which a child of it is laid out: a child's `.drop_outside(...)`. With no bound
/// said it holds at every size. A bound that is not a number never holds.
///
/// ```
/// use lurq::layout::layout_kind::SizeBounds;
///
/// // From 240 wide up; under 240 and over 96; 96 and under.
/// let large = SizeBounds::new().min_width(240.0);
/// let medium = SizeBounds::new().above_width(96.0).below_width(240.0);
/// let small = SizeBounds::new().max_width(96.0);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SizeBounds {
  width: (Bound, Bound),
  height: (Bound, Bound),
}

impl SizeBounds {
  pub const fn new() -> Self {
    Self {
      width: (None, None),
      height: (None, None),
    }
  }

  /// At `width` and wider.
  pub const fn min_width(mut self, width: f32) -> Self {
    self.width.0 = Some((width, true));
    self
  }

  /// Wider than `width`.
  pub const fn above_width(mut self, width: f32) -> Self {
    self.width.0 = Some((width, false));
    self
  }

  /// At `width` and narrower.
  pub const fn max_width(mut self, width: f32) -> Self {
    self.width.1 = Some((width, true));
    self
  }

  /// Narrower than `width`.
  pub const fn below_width(mut self, width: f32) -> Self {
    self.width.1 = Some((width, false));
    self
  }

  /// At `height` and higher.
  pub const fn min_height(mut self, height: f32) -> Self {
    self.height.0 = Some((height, true));
    self
  }

  /// Higher than `height`.
  pub const fn above_height(mut self, height: f32) -> Self {
    self.height.0 = Some((height, false));
    self
  }

  /// At `height` and lower.
  pub const fn max_height(mut self, height: f32) -> Self {
    self.height.1 = Some((height, true));
    self
  }

  /// Lower than `height`.
  pub const fn below_height(mut self, height: f32) -> Self {
    self.height.1 = Some((height, false));
    self
  }

  /// Whether a stack of `size` is within the bounds.
  pub fn holds(&self, size: Size) -> bool {
    let within = |value: f32, (min, max): (Bound, Bound)| {
      min.is_none_or(|(bound, inclusive)| if inclusive { value >= bound } else { value > bound })
        && max.is_none_or(|(bound, inclusive)| if inclusive { value <= bound } else { value < bound })
    };
    within(size.width, self.width) && within(size.height, self.height)
  }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum FlexWrap {
  #[default]
  NoWrap,
  Wrap,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Justify {
  #[default]
  Start,
  End,
  Center,
  SpaceBetween,
  SpaceAround,
  SpaceEvenly,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Overflow {
  Visible,
  #[default]
  Hidden,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum ScrollDirection {
  Horizontal,
  #[default]
  Vertical,
  Both,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ScrollAxis {
  Horizontal,
  Vertical,
}

#[derive(Clone, Copy, Default, PartialEq)]
pub struct FrameConstraints {
  pub width: Option<Dimension>,
  pub height: Option<Dimension>,
  pub min_width: Option<Dimension>,
  pub max_width: Option<Dimension>,
  pub min_height: Option<Dimension>,
  pub max_height: Option<Dimension>,
  /// The box's width over its height, as CSS `aspect-ratio`: it gives the axis that is auto from the one that
  /// is known. Set with `.aspect_ratio(...)`.
  pub aspect_ratio: Option<f32>,
}

#[derive(Clone, Copy, Default, PartialEq)]
pub enum Position {
  #[default]
  Static,
  Absolute {
    x: f32,
    y: f32,
    width: Option<Dimension>,
    height: Option<Dimension>,
  },
}

/// Where an element sticks while the scroll container it is in scrolls, as CSS `position: sticky` with
/// `top`, `right`, `bottom` and `left`. Set with `.sticky(...)`.
///
/// The element is laid out in flow and keeps its room there. While scrolling would take it past an edge it
/// has an inset for, it stays that far inside the edge of the scroll container's viewport instead, for as
/// long as the box of its parent has room for it: it never leaves that box, so the end of a section takes
/// the section's header with it. It is painted over its siblings, which scroll under it.
///
/// An inset is in logical pixels. An element in no scroll container stays where it is laid out.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Sticky {
  pub top: Option<f32>,
  pub right: Option<f32>,
  pub bottom: Option<f32>,
  pub left: Option<f32>,
}

impl Sticky {
  /// Sticks `inset` below the viewport's top edge: a header.
  pub fn top(inset: f32) -> Self {
    Self::default().with_top(inset)
  }

  /// Sticks `inset` above the viewport's bottom edge: a footer.
  pub fn bottom(inset: f32) -> Self {
    Self::default().with_bottom(inset)
  }

  /// Sticks `inset` from the viewport's left edge: a row's first column.
  pub fn left(inset: f32) -> Self {
    Self::default().with_left(inset)
  }

  /// Sticks `inset` from the viewport's right edge.
  pub fn right(inset: f32) -> Self {
    Self::default().with_right(inset)
  }

  pub fn with_top(mut self, inset: f32) -> Self {
    self.top = Some(inset);
    self
  }

  pub fn with_right(mut self, inset: f32) -> Self {
    self.right = Some(inset);
    self
  }

  pub fn with_bottom(mut self, inset: f32) -> Self {
    self.bottom = Some(inset);
    self
  }

  pub fn with_left(mut self, inset: f32) -> Self {
    self.left = Some(inset);
    self
  }

  /// Where an element of `size` whose place in flow is `natural` is put, given the viewport of its scroll
  /// container and the box it may not leave, each as `[x, y, width, height]` in the coordinates `natural`
  /// is in. Where both edges of an axis have an inset and the viewport has no room for both, the start
  /// edge's holds.
  pub(crate) fn place(&self, natural: Offset, size: Size, viewport: [f32; 4], block: [f32; 4]) -> Offset {
    Offset {
      x: stick(
        natural.x,
        size.width,
        self.left,
        self.right,
        (viewport[0], viewport[2]),
        (block[0], block[2]),
      ),
      y: stick(
        natural.y,
        size.height,
        self.top,
        self.bottom,
        (viewport[1], viewport[3]),
        (block[1], block[3]),
      ),
    }
  }
}

/// One axis of [`Sticky::place`]: the start of an element `length` long whose place in flow is `natural`,
/// held `start` and `end` inside a viewport and inside a block, each given as (start, length).
fn stick(
  natural: f32,
  length: f32,
  start: Option<f32>,
  end: Option<f32>,
  viewport: (f32, f32),
  block: (f32, f32),
) -> f32 {
  let mut at = natural;
  if let Some(end) = end.filter(|inset| inset.is_finite()) {
    at = at.min(viewport.0 + viewport.1 - end - length);
  }
  if let Some(start) = start.filter(|inset| inset.is_finite()) {
    at = at.max(viewport.0 + start);
  }
  // It stays in its block, and a block too small for that leaves it where it is laid out.
  let low = block.0.min(natural);
  let high = (block.0 + block.1 - length).max(natural);
  at.clamp(low, high)
}

impl FrameConstraints {
  pub fn with_width(mut self, value: Option<Dimension>) -> Self {
    self.width = value;
    self
  }
  pub fn with_height(mut self, value: Option<Dimension>) -> Self {
    self.height = value;
    self
  }
  pub fn with_min_width(mut self, value: Option<Dimension>) -> Self {
    self.min_width = value;
    self
  }
  pub fn with_max_width(mut self, value: Option<Dimension>) -> Self {
    self.max_width = value;
    self
  }
  pub fn with_min_height(mut self, value: Option<Dimension>) -> Self {
    self.min_height = value;
    self
  }
  pub fn with_max_height(mut self, value: Option<Dimension>) -> Self {
    self.max_height = value;
    self
  }
  pub fn with_aspect_ratio(mut self, value: Option<f32>) -> Self {
    self.aspect_ratio = value;
    self
  }
}
