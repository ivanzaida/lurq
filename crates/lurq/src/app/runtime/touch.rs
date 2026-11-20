//! What a touch shell asks of the tree: what a finger that lands on a point does, the scroll of a finger's
//! gesture, and the stretch of a list pulled past its end.
//!
//! A scroll container whose content is larger than its viewport gives when a finger pulls it past an end: the
//! content follows the finger, less and less the further it is pulled ([`rubber`]), and springs back when the
//! finger lifts. Momentum that reaches an end carries the content past it and the same spring brings it back.
//! Both are a scroll view of the platform's: `UIScrollView` on iOS.

use super::{
  MouseModifiers, ScrollInput, Tree, find_node_by_id, find_scroll_node_id_by_state, hit_test_tree,
  mark_scroll_layout_dirty, scrollbar_hit_at, trim_hits_to_scrollbar,
};
use crate::{
  app::events::{ScrollPhase, ScrollSource},
  core::NodeId,
  layout::layout_kind::{ScrollDirection, ScrollState},
  node::{node::Node, node_kind::NodeKind},
  time::{Duration, Instant},
};

/// How much of a finger's travel past an end the content follows at first; `UIScrollView`'s constant.
const RUBBER_BAND: f32 = 0.55;
/// The angular frequency, per second, of the spring that brings the content back: `UIScrollView`'s, a critically
/// damped spring of mass 1 and stiffness 100.
const SPRING_FREQUENCY: f32 = 10.0;
/// The spring is over once the content is this close to its end, in logical pixels, and no sooner than
/// [`SPRING_MIN`] after it started, when momentum has had time to carry the content out.
const SPRING_REST: f32 = 0.5;
const SPRING_MIN: Duration = Duration::from_millis(100);
/// How long a list takes to scroll to its top when the status bar is tapped.
const TO_TOP: Duration = Duration::from_millis(350);
/// The angular frequency, per second, of the spring that brings a list that scrolls by pages to a page, and
/// the slowest lift, in logical pixels per second, that throws it at the next one. Both set by eye.
const PAGE_FREQUENCY: f32 = 14.0;
const PAGE_FLICK: f32 = 200.0;

/// How a finger's scroll goes on after the finger lifts. The default is a scroll view of the platform's:
/// `UIScrollView` at its normal deceleration rate.
///
/// Set with [`Tree::set_touch_momentum`]; speeds are in logical pixels per second.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TouchMomentum {
  /// The share of its speed the scroll keeps over each millisecond, below 1: `0.998` is `UIScrollView`'s
  /// normal rate and `0.99` its fast one, which stops in about a fifth of the distance.
  pub deceleration_rate: f32,
  /// The stretch of the gesture, ending at the lift, the speed is measured over. A finger that stopped for
  /// longer before it lifted leaves no momentum.
  pub velocity_window: Duration,
  /// The slowest lift that leaves momentum.
  pub min_speed: f32,
  /// The fastest momentum: a quicker lift goes on at this speed.
  pub max_speed: f32,
  /// The speed momentum stops at.
  pub stop_speed: f32,
}

impl Default for TouchMomentum {
  fn default() -> Self {
    Self {
      deceleration_rate: 0.998,
      velocity_window: Duration::from_millis(100),
      min_speed: 50.0,
      max_speed: 8000.0,
      stop_speed: 15.0,
    }
  }
}

/// How far content pulled `pull` past an end of a viewport `size` long is drawn past it.
fn rubber(pull: f32, size: f32) -> f32 {
  if size <= 0.0 {
    return 0.0;
  }
  pull.signum() * (1.0 - 1.0 / (pull.abs() * RUBBER_BAND / size + 1.0)) * size
}

/// The pull content drawn `shown` past an end stands for: [`rubber`]'s inverse.
fn pull_of(shown: f32, size: f32) -> f32 {
  let share = (shown.abs() / size.max(f32::EPSILON)).min(0.999);
  shown.signum() * size / RUBBER_BAND * (1.0 / (1.0 - share) - 1.0)
}

/// The scroll container a finger's gesture scrolls.
struct Taker {
  state: ScrollState,
  /// Whether it scrolls sideways and down.
  axes: (bool, bool),
}

impl Taker {
  fn offsets(&self) -> (f32, f32) {
    (self.state.scroll_x(), self.state.scroll_y())
  }

  /// How far the offset can go along each axis; none along an axis the container does not scroll.
  fn ends(&self) -> (f32, f32) {
    let width = (self.state.content_width() - self.state.viewport_width()).max(0.0);
    let height = (self.state.content_height() - self.state.viewport_height()).max(0.0);
    (
      if self.axes.0 { width } else { 0.0 },
      if self.axes.1 { height } else { 0.0 },
    )
  }

  fn viewport(&self) -> (f32, f32) {
    (self.state.viewport_width(), self.state.viewport_height())
  }
}

/// Along one axis: whether an offset that would move by `change` is held at an end it can be pulled past.
fn at_end(offset: f32, end: f32, change: f32) -> bool {
  end > 0.0 && ((change > 0.0 && offset >= end) || (change < 0.0 && offset <= 0.0))
}

struct Spring {
  from: (f32, f32),
  /// Logical pixels per second.
  velocity: (f32, f32),
  started: Instant,
  /// A finger let the content go and its velocity is not known yet; see [`Tree::touch_scroll_bounce`].
  released: bool,
}

impl Spring {
  /// A critically damped spring's place `elapsed` after it started.
  fn at(&self, elapsed: f32) -> (f32, f32) {
    self.at_frequency(SPRING_FREQUENCY, elapsed)
  }

  fn at_frequency(&self, frequency: f32, elapsed: f32) -> (f32, f32) {
    let decay = (-frequency * elapsed).exp();
    let axis = |from: f32, velocity: f32| (from + (velocity + frequency * from) * elapsed) * decay;
    (axis(self.from.0, self.velocity.0), axis(self.from.1, self.velocity.1))
  }
}

/// A list that scrolls by pages on its way to one.
struct PageMove {
  /// The offset of the page.
  to: (f32, f32),
  /// How far the content is from it, and how fast it was going when the finger lifted.
  spring: Spring,
}

impl PageMove {
  /// A list at `offset` let go at rest, on its way to the page at `to` from `now`.
  fn from_rest(offset: (f32, f32), to: (f32, f32), now: Instant) -> Self {
    Self {
      to,
      spring: Spring {
        from: (offset.0 - to.0, offset.1 - to.1),
        velocity: (0.0, 0.0),
        started: now,
        released: false,
      },
    }
  }

  /// Where the list is at `now`, and whether that is its page, at rest.
  fn at(&self, now: Instant) -> ((f32, f32), bool) {
    let elapsed = now.saturating_duration_since(self.spring.started);
    let left = self.spring.at_frequency(PAGE_FREQUENCY, elapsed.as_secs_f32());
    if elapsed >= SPRING_MIN && left.0.abs() < SPRING_REST && left.1.abs() < SPRING_REST {
      return (self.to, true);
    }
    // Thrown hard, the spring would carry the list past the page: it stops there.
    let within = |from: f32, left: f32| if from * left < 0.0 { 0.0 } else { left };
    let at = (
      self.to.0 + within(self.spring.from.0, left.0),
      self.to.1 + within(self.spring.from.1, left.1),
    );
    (at, false)
  }
}

/// A list on its way to a page beside the gesture's own list: one the gesture moved along with its own, or one a
/// finger stopped on its way before it went on to scroll another list.
struct Aside {
  state: ScrollState,
  /// The list's node, which hears its steps and its end.
  node: Option<NodeId>,
  page: PageMove,
}

impl Aside {
  /// `state`'s list goes on to the page at `to` from where it is: it is held until it is there.
  fn to(state: ScrollState, node: Option<NodeId>, to: (f32, f32), now: Instant) -> Self {
    state.set_held(true);
    state.set_page_target(Some(to));
    let page = PageMove::from_rest((state.scroll_x(), state.scroll_y()), to, now);
    Self { state, node, page }
  }
}

/// Along one axis: the offset of the page content at `offset` goes to when a finger that started on the page
/// at `began` lifts at `speed`, in logical pixels per second of the offset. Thrown, it goes to the next page
/// that way; let go, to the nearest; and no further than the page beside the one it started on.
fn page_target(offset: f32, began: f32, size: f32, end: f32, speed: f32) -> f32 {
  if size <= 0.0 || end <= 0.0 {
    return offset.clamp(0.0, end.max(0.0));
  }
  let from = (began / size).round();
  let at = offset / size;
  let page = if speed > PAGE_FLICK {
    at.ceil()
  } else if speed < -PAGE_FLICK {
    at.floor()
  } else {
    at.round()
  };
  (page.clamp(from - 1.0, from + 1.0) * size).clamp(0.0, end)
}

/// A finger's scroll gesture and what it left the content doing.
#[derive(Default)]
pub(super) struct TouchScroll {
  taker: Option<Taker>,
  /// The taker and every node around it, whose layouts hold its content's place.
  around: Vec<NodeId>,
  /// The finger's travel past an end, in logical pixels: negative before the start, positive beyond the end.
  pull: (f32, f32),
  spring: Option<Spring>,
  to_top: Option<ToTop>,
  /// Where the content was when the gesture started: a list that scrolls by pages goes no further than the
  /// page beside that one.
  began: (f32, f32),
  page: Option<PageMove>,
  /// The page a list was going to when a finger caught it.
  caught_page: Option<(f32, f32)>,
  /// A gesture is under way, the finger's or its momentum's: between its first event and its last.
  moving: bool,
  /// A finger caught the content on its way and holds it still: it is not at rest until the finger lets go.
  caught: bool,
  /// What last moved the content.
  source: ScrollSource,
  /// Where the gesture started, in logical pixels: where the events of what it leaves are from.
  at: (f32, f32),
  /// Whether the gesture goes down, as its first travel did, and not across: its list, where it scrolls one
  /// way, takes that part of the finger's travel and none of the other.
  vertical: bool,
  /// The lists other than the gesture's own that it moved: each is put on a page when the gesture is over, if
  /// it scrolls by pages.
  moved: Vec<(NodeId, ScrollState)>,
  /// The lists on their way to a page beside the gesture.
  asides: Vec<Aside>,
}

/// A list on its way to its top.
struct ToTop {
  state: ScrollState,
  /// The list and every node around it.
  around: Vec<NodeId>,
  from: f32,
  started: Instant,
}

impl TouchScroll {
  /// Whether the gesture, or what it left, still has the content.
  fn busy(&self) -> bool {
    self.moving || self.caught || self.page.is_some() || self.spring.is_some()
  }
}

impl Tree {
  /// How far apart, in physical pixels, two presses may land and still be one double click, or a double or
  /// triple click in text. A mouse lands on the same few pixels; two taps of a finger do not, so a touch shell
  /// widens it.
  pub fn set_double_click_distance(&mut self, distance: f32) {
    self.double_click_distance = Some(distance);
  }

  /// How long after a press the next one is still a double click, or a double or triple click in text. A
  /// shell sets the platform's; without one it is half a second.
  pub fn set_double_click_interval(&mut self, interval: Duration) {
    self.double_click_interval = Some(interval);
  }

  /// Sets how a finger's scroll goes on after the finger lifts, in every list of the window. It takes effect
  /// with the next touch.
  pub fn set_touch_momentum(&mut self, momentum: TouchMomentum) {
    self.touch_momentum = momentum;
  }

  /// How a finger's scroll goes on after the finger lifts; see [`set_touch_momentum`](Self::set_touch_momentum).
  pub fn touch_momentum(&self) -> TouchMomentum {
    self.touch_momentum
  }

  /// Whether a finger that moves from the point, in physical pixels, scrolls: a scroll container with content
  /// beyond its viewport is under it, and no control that is dragged instead (a slider, a scrollbar that is
  /// always shown, an element that says so with `finger_drags`). Anywhere else a finger is a pointer with its button down.
  pub fn touch_scrolls_at(&mut self, x: f32, y: f32) -> bool {
    let scale = self.scale_factor();
    let (lx, ly) = (x / scale, y / scale);
    self.rebuild_if_dirty();
    let (Some(root), Some(layout)) = (&self.root, &self.last_layout) else {
      return false;
    };
    // A finger on a disabled element scrolls what it would scroll were the element enabled.
    let mut hits = super::scroll_hits(root, layout, lx, ly, &self.layout_engine.radii());
    trim_hits_to_scrollbar(&mut hits, lx, ly);
    // A bar that is always there is dragged at once. A scroll indicator, which comes and goes, is content to
    // a finger that moves, and is dragged by one that rested on it first, as the platform's.
    let on_scrollbar = hits.first().is_some_and(|(node, _)| {
      scrollbar_hit_at(node, lx, ly).is_some()
        && !node
          .scrollbar_style(crate::layout::scrollbar::ScrollBarStyle::default())
          .visible
          .fades()
    });
    // A slider, and what says a finger drags it: a dial, an area of colors, a pad that is drawn on.
    let on_slider = hits
      .iter()
      .any(|(node, _)| matches!(node.node_kind(), NodeKind::Slider { .. }) || node.events.finger_drags);
    if on_scrollbar || on_slider {
      return false;
    }
    hits.iter().any(|(node, _)| {
      node.scroll_container().is_some_and(|(state, _)| {
        state.content_height() > state.viewport_height() || state.content_width() > state.viewport_width()
      })
    })
  }

  /// Whether the keyboard is to follow a finger that lands at the point, in physical pixels: a text input is
  /// typed into, and a list under the point asks for it
  /// ([`KeyboardDismissal::Interactive`](crate::layout::layout_kind::KeyboardDismissal)).
  pub fn touch_keyboard_follows_at(&mut self, x: f32, y: f32) -> bool {
    if !self.focused_text_input() {
      return false;
    }
    let scale = self.scale_factor();
    let (lx, ly) = (x / scale, y / scale);
    self.rebuild_if_dirty();
    let (Some(root), Some(layout)) = (&self.root, &self.last_layout) else {
      return false;
    };
    let mut hits = Vec::new();
    hit_test_tree(root, layout, 0.0, 0.0, lx, ly, &self.layout_engine.radii(), &mut hits);
    hits.iter().any(|(node, _)| {
      node.scroll_container().is_some_and(|(state, _)| {
        state.keyboard_dismissal() == crate::layout::layout_kind::KeyboardDismissal::Interactive
      })
    })
  }

  /// The platform put the keyboard away itself, as it does when a finger takes it down: the text input that
  /// was typed into is left.
  pub fn keyboard_was_put_away(&mut self) {
    if self.focused_text_input() {
      self.blur_focus();
    }
  }

  /// Starts a finger's scroll gesture at the point, in physical pixels, with the finger's first travel. One
  /// scroll container takes the whole gesture, its momentum included, as a scroll view of the platform does:
  /// the innermost one under the point that scrolls along the travel's main axis and has room to move that
  /// way, or the innermost along that axis when none has room. So a list inside a page that has reached its
  /// end hands the next gesture, not the rest of this one, to the page; a gesture across a sideways strip
  /// scrolls the page; and content the gesture brings under the point does not take it over.
  pub fn touch_scroll_begin(&mut self, x: f32, y: f32, delta_x: f32, delta_y: f32) {
    let scale = self.scale_factor();
    let (lx, ly) = (x / scale, y / scale);
    self.rebuild_if_dirty();
    let heard_before = std::mem::take(&mut self.touch_scroll_targets);
    let (Some(root), Some(layout)) = (&self.root, &self.last_layout) else {
      return;
    };
    let hits = super::scroll_hits(root, layout, lx, ly, &self.layout_engine.radii());

    let vertical = delta_y.abs() >= delta_x.abs();
    // The way the offset goes: a finger moving up or left brings later content.
    let forward = if vertical { delta_y < 0.0 } else { delta_x < 0.0 };
    let along_axis = |node: &Node| {
      let (state, direction) = node.scroll_container()?;
      let along = match direction {
        ScrollDirection::Both => true,
        ScrollDirection::Vertical => vertical,
        ScrollDirection::Horizontal => !vertical,
      };
      along.then_some(state)
    };
    let has_room = |node: &Node| {
      along_axis(node).is_some_and(|state| {
        let (offset, end) = if vertical {
          (state.scroll_y(), state.content_height() - state.viewport_height())
        } else {
          (state.scroll_x(), state.content_width() - state.viewport_width())
        };
        if forward { offset < end } else { offset > 0.0 }
      })
    };
    let taker = hits
      .iter()
      .position(|(node, _)| has_room(node))
      .or_else(|| hits.iter().position(|(node, _)| along_axis(node).is_some()));
    // The nodes inside the taker hear the gesture's scroll events with it; the containers around it get none.
    let heard = taker.map_or(hits.len(), |taker| taker + 1);
    self
      .touch_scroll_targets
      .extend(hits[..heard].iter().map(|(node, _)| node.node_id()));

    let around = taker.map_or(Vec::new(), |taker| {
      hits[taker..].iter().map(|(node, _)| node.node_id()).collect()
    });
    let taker = taker.and_then(|taker| {
      let node = hits[taker].0;
      let (state, direction) = node.scroll_container()?;
      Some(Taker {
        state,
        axes: (
          !matches!(direction, ScrollDirection::Vertical),
          !matches!(direction, ScrollDirection::Horizontal),
        ),
      })
    });
    drop(hits);
    // A container that puts the keyboard away when scrolled does, by the focus leaving the text input.
    if taker.as_ref().is_some_and(|taker| taker.state.dismisses_keyboard()) && self.focused_text_input() {
      self.blur_focus();
    }

    // A finger that catches content on its way back holds it where it is. Content of another container is
    // put back at once: one gesture, one container.
    let now = self.press_now();
    let mut previous = std::mem::take(&mut self.touch_scroll);
    let mut asides = std::mem::take(&mut previous.asides);
    let caught = previous
      .taker
      .as_ref()
      .zip(taker.as_ref())
      .is_some_and(|(previous, taker)| previous.state.ptr_eq(&taker.state));
    let pull = match &taker {
      Some(taker) if caught => {
        let (shown, viewport) = (taker.state.overscroll(), taker.viewport());
        (pull_of(shown.0, viewport.0), pull_of(shown.1, viewport.1))
      }
      _ => (0.0, 0.0),
    };
    if !caught && let Some(taker) = &previous.taker {
      taker.state.set_overscroll(0.0, 0.0);
      let going = previous.caught_page.or(previous.page.as_ref().map(|page| page.to));
      match going {
        // A list a finger stopped on its way to a page is not left between two: it goes on to that page
        // beside the gesture, which is another list's.
        Some(page) => asides.push(Aside::to(
          taker.state.clone(),
          previous.around.first().copied(),
          page,
          now,
        )),
        // What the last gesture left its list doing is over.
        None if taker.state.held() => {
          let state = taker.state.clone();
          self.scroll_rests(&state, &heard_before, previous.at, previous.source);
        }
        None => {}
      }
    }
    // The lists the last gesture moved beside its own are put on their pages, where it did not get to.
    let moved = std::mem::take(&mut previous.moved);
    Self::pages_of_moved(&mut asides, moved, taker.as_ref().map(|taker| &taker.state), now);
    // A list on its way to a page beside the last gesture is this gesture's now.
    let own = taker.as_ref().and_then(|taker| {
      let at = asides.iter().position(|aside| aside.state.ptr_eq(&taker.state))?;
      Some(asides.remove(at))
    });
    if let (Some(taker), Some(_)) = (&taker, &own) {
      taker.state.set_page_target(None);
    }
    // A list caught on its way to a page is still of the page it was going to.
    let began = match (&taker, previous.caught_page, own) {
      (Some(_), _, Some(own)) => own.page.to,
      (Some(_), Some(page), None) if caught => page,
      (Some(taker), ..) => taker.offsets(),
      (None, ..) => (0.0, 0.0),
    };
    self.touch_scroll = TouchScroll {
      taker,
      around,
      pull,
      spring: None,
      to_top: None,
      began,
      page: None,
      caught_page: None,
      moving: false,
      caught: false,
      source: ScrollSource::Finger,
      at: (lx, ly),
      vertical,
      moved: Vec::new(),
      asides,
    };
    self.mark_touch_scroll_moved();
  }

  /// The gesture moved these lists: those other than its own are remembered, and put on a page when it is
  /// over.
  pub(super) fn touch_scrolled(&mut self, lists: &[(NodeId, ScrollState)]) {
    let scroll = &mut self.touch_scroll;
    for (node, state) in lists {
      let own = scroll.taker.as_ref().is_some_and(|taker| taker.state.ptr_eq(state));
      if !own && !scroll.moved.iter().any(|(_, moved)| moved.ptr_eq(state)) {
        scroll.moved.push((*node, state.clone()));
      }
    }
  }

  /// Sends each of `moved` that scrolls by pages and is between two to the nearer one, beside the gesture:
  /// all but `own`, the gesture's list, which goes where the finger threw it.
  fn pages_of_moved(
    asides: &mut Vec<Aside>,
    moved: Vec<(NodeId, ScrollState)>,
    own: Option<&ScrollState>,
    now: Instant,
  ) {
    for (node, state) in moved {
      if !state.paging()
        || own.is_some_and(|own| own.ptr_eq(&state))
        || asides.iter().any(|aside| aside.state.ptr_eq(&state))
      {
        continue;
      }
      let offset = (state.scroll_x(), state.scroll_y());
      let viewport = (state.viewport_width(), state.viewport_height());
      let ends = (
        (state.content_width() - viewport.0).max(0.0),
        (state.content_height() - viewport.1).max(0.0),
      );
      let to = (
        page_target(offset.0, offset.0, viewport.0, ends.0, 0.0),
        page_target(offset.1, offset.1, viewport.1, ends.1, 0.0),
      );
      if to != offset {
        asides.push(Aside::to(state, Some(node), to, now));
      }
    }
  }

  /// The gesture's content starts to move, where it was at rest: what the gesture scrolls hears the scroll's
  /// start, once for the finger and all it leaves the content doing.
  fn touch_scroll_starts(&mut self) {
    let Some(taker) = &self.touch_scroll.taker else {
      return;
    };
    if taker.state.held() {
      return;
    }
    taker.state.set_held(true);
    tracing::debug!(
      target: "lurq::scroll",
      x = taker.state.scroll_x(),
      y = taker.state.scroll_y(),
      page = ?taker.state.page(),
      "start"
    );
    let at = self.touch_scroll.at;
    for target in self.touch_scroll_targets.clone() {
      self.dispatch_scroll_handlers_for_node(target, at.0, at.1, 0.0, 0.0, ScrollPhase::Start, ScrollSource::Finger);
    }
  }

  /// Tells the content of the gesture's container at rest once nothing has it any more: no finger, no
  /// momentum, no way to a page and no spring back. A shell that drives the gesture itself has this from
  /// [`touch_tick`](Self::touch_tick); [`finger`](Self::finger) and [`finger_tick`](Self::finger_tick) ask.
  pub(super) fn touch_scroll_rest(&mut self, now: Instant) {
    // The finger and its momentum are over: what they moved beside their own list is put on a page.
    if !self.touch_scroll.moving && !self.touch_scroll.caught && !self.touch_scroll.moved.is_empty() {
      let scroll = &mut self.touch_scroll;
      let moved = std::mem::take(&mut scroll.moved);
      let own = scroll.taker.as_ref().map(|taker| taker.state.clone());
      Self::pages_of_moved(&mut scroll.asides, moved, own.as_ref(), now);
      self.needs_redraw = true;
    }
    let scroll = &self.touch_scroll;
    let Some(taker) = &scroll.taker else {
      return;
    };
    if scroll.busy() || !taker.state.held() {
      return;
    }
    let (state, at, source) = (taker.state.clone(), scroll.at, scroll.source);
    let heard = self.touch_scroll_targets.clone();
    self.scroll_rests(&state, &heard, at, source);
  }

  /// The node of the gesture's container: the one the gesture started on or, where a rebuild replaced it, the
  /// one that has its state now.
  fn touch_scroll_node(&self) -> Option<NodeId> {
    let taker = self.touch_scroll.taker.as_ref()?;
    let root = self.root.as_ref()?;
    self
      .touch_scroll
      .around
      .first()
      .copied()
      .filter(|id| find_node_by_id(root, *id).is_some())
      .or_else(|| find_scroll_node_id_by_state(root, &taker.state))
  }

  /// Whether what the gesture scrolls goes by pages ([`ScrollState::set_paging`]): the shell leaves it no
  /// momentum when the finger lifts, and tells how fast it lifted instead
  /// ([`touch_scroll_page`](Self::touch_scroll_page)).
  pub fn touch_scroll_pages(&self) -> bool {
    self
      .touch_scroll
      .taker
      .as_ref()
      .is_some_and(|taker| taker.state.paging())
  }

  /// A finger let go of a list that scrolls by pages, at a velocity in physical pixels per second: the list
  /// goes on to a page. Does nothing to content pulled past an end, which springs back to its last page.
  pub fn touch_scroll_page(&mut self, velocity_x: f32, velocity_y: f32, now: Instant) {
    let scale = self.scale_factor();
    let Some(taker) = &self.touch_scroll.taker else {
      return;
    };
    if !taker.state.paging() || self.touch_scroll.spring.is_some() {
      return;
    }
    let (offsets, ends, viewport) = (taker.offsets(), taker.ends(), taker.viewport());
    // A finger going up or left brings later content: the offset goes the other way from the finger.
    let speed = (-velocity_x / scale, -velocity_y / scale);
    let began = self.touch_scroll.began;
    let to = (
      page_target(offsets.0, began.0, viewport.0, ends.0, speed.0),
      page_target(offsets.1, began.1, viewport.1, ends.1, speed.1),
    );
    if to == offsets {
      return;
    }
    taker.state.set_page_target(Some(to));
    tracing::debug!(
      target: "lurq::scroll",
      page = ?taker.state.target_page(),
      from = ?taker.state.rested_page(),
      "snapping to page"
    );
    self.touch_scroll.source = ScrollSource::Snap;
    self.touch_scroll.page = Some(PageMove {
      to,
      spring: Spring {
        from: (offsets.0 - to.0, offsets.1 - to.1),
        // Along an axis the list does not move on, how fast the finger went is nothing to it.
        velocity: (
          if to.0 == offsets.0 { 0.0 } else { speed.0 },
          if to.1 == offsets.1 { 0.0 } else { speed.1 },
        ),
        started: now,
        released: false,
      },
    });
    self.needs_redraw = true;
  }

  /// Scrolls what the gesture started on by a travel in physical pixels: a finger's, or its momentum's after
  /// it lifted. Tells whether the content moved; momentum ends when it does not, the content having reached
  /// an end.
  ///
  /// The content moves as far as the finger. [`scroll`](Self::scroll) moves it by its delta in logical pixels,
  /// which is a wheel's or a trackpad's speed, so the travel is converted first. A finger's travel past an end
  /// pulls the content past it, and the gesture's last event, with [`ScrollPhase::End`], lets it spring back.
  pub fn touch_scroll(
    &mut self,
    x: f32,
    y: f32,
    delta_x: f32,
    delta_y: f32,
    phase: ScrollPhase,
    momentum: bool,
  ) -> bool {
    let scale = self.scale_factor();
    // How far the offset is asked to go.
    let mut change = (-delta_x / scale, -delta_y / scale);
    // A list that scrolls one way takes the finger's travel that way and none of the other: the gesture is its
    // own from the finger's first points, and a strip in the page does not creep sideways under a finger that
    // scrolls the page down on a slant.
    if let Some(taker) = &self.touch_scroll.taker
      && taker.axes != (true, true)
    {
      if self.touch_scroll.vertical {
        change.0 = 0.0;
      } else {
        change.1 = 0.0;
      }
    }
    let pulled = self.touch_scroll.pull;
    if !momentum {
      // Travel back toward the content first lets go of the pull.
      let release = |pull: &mut f32, change: &mut f32| {
        if *pull != 0.0 && pull.signum() != change.signum() {
          let given = change.abs().min(pull.abs());
          *pull -= pull.signum() * given;
          *change -= change.signum() * given;
        }
      };
      release(&mut self.touch_scroll.pull.0, &mut change.0);
      release(&mut self.touch_scroll.pull.1, &mut change.1);
    }

    let before = self.touch_scroll.taker.as_ref().map(Taker::offsets);
    let source = if momentum {
      ScrollSource::Momentum
    } else {
      ScrollSource::Finger
    };
    self.touch_scroll.source = source;
    self.touch_scroll.moving = !matches!(phase, ScrollPhase::End);
    self.touch_scroll_starts();
    self.touch_scroll_latched = true;
    let input = ScrollInput {
      animated: false,
      notched: false,
      modifiers: MouseModifiers::default(),
      source,
    };
    self.scroll_event(x, y, -change.0, -change.1, phase, input);
    self.touch_scroll_latched = false;
    let after = self.touch_scroll.taker.as_ref().map(Taker::offsets);

    // Content already springing back is the spring's: the event that follows a lift, which starts the momentum,
    // must not put it at its end for the frames until the spring's next step.
    let springing = self.touch_scroll.spring.is_some();
    if !momentum
      && !springing
      && let (Some(taker), Some(before), Some(after)) = (&self.touch_scroll.taker, before, after)
    {
      let ends = taker.ends();
      let left = (change.0 - (after.0 - before.0), change.1 - (after.1 - before.1));
      let mut pull = self.touch_scroll.pull;
      if at_end(after.0, ends.0, left.0) {
        pull.0 += left.0;
      }
      if at_end(after.1, ends.1, left.1) {
        pull.1 += left.1;
      }
      let viewport = taker.viewport();
      taker
        .state
        .set_overscroll(rubber(pull.0, viewport.0), rubber(pull.1, viewport.1));
      self.touch_scroll.pull = pull;
      if matches!(phase, ScrollPhase::End) && pull != (0.0, 0.0) {
        self.touch_scroll.spring = Some(Spring {
          from: taker.state.overscroll(),
          velocity: (0.0, 0.0),
          started: Instant::now(),
          released: true,
        });
        self.touch_scroll.pull = (0.0, 0.0);
      }
    }
    let moved = before != after || pulled != self.touch_scroll.pull;
    if moved {
      self.mark_touch_scroll_moved();
    }
    moved
  }

  /// A finger landed. Content springing back stops where it is and stays under the finger: `true` when there
  /// was any. The gesture that follows goes on from there, and [`touch_scroll_settle`](Self::touch_scroll_settle)
  /// lets the content go when the finger lifts without scrolling.
  pub fn touch_scroll_catch(&mut self) -> bool {
    // A list on its way to its top stops where it is, and its scroll is over there.
    if let Some(to_top) = self.touch_scroll.to_top.take() {
      let list = to_top.around.first().copied();
      self.scroll_rests(&to_top.state, list.as_slice(), (0.0, 0.0), ScrollSource::Code);
      return true;
    }
    let Some(taker) = &self.touch_scroll.taker else {
      return false;
    };
    // A list on its way to a page stops between two, and is still of the page it was going to.
    if let Some(page) = self.touch_scroll.page.take() {
      self.touch_scroll.began = page.to;
      self.touch_scroll.caught_page = Some(page.to);
      self.touch_scroll.caught = true;
      return true;
    }
    if self.touch_scroll.spring.take().is_none() {
      return false;
    }
    let (shown, viewport) = (taker.state.overscroll(), taker.viewport());
    self.touch_scroll.pull = (pull_of(shown.0, viewport.0), pull_of(shown.1, viewport.1));
    self.touch_scroll.caught = true;
    true
  }

  /// A finger caught content that was moving, by its momentum: it holds it still, as
  /// [`touch_scroll_catch`](Self::touch_scroll_catch) says of content springing back.
  pub(super) fn touch_scroll_caught(&mut self) {
    self.touch_scroll.caught = self.touch_scroll.taker.is_some();
  }

  /// Lets go of content a finger held still: past an end, it springs back from where it is, and between two
  /// pages of a list that scrolls by them, it goes to the nearest.
  pub fn touch_scroll_settle(&mut self) {
    let Some(taker) = &self.touch_scroll.taker else {
      return;
    };
    let shown = taker.state.overscroll();
    self.touch_scroll.pull = (0.0, 0.0);
    self.touch_scroll.caught = false;
    if shown != (0.0, 0.0) {
      self.touch_scroll.spring = Some(Spring {
        from: shown,
        velocity: (0.0, 0.0),
        started: Instant::now(),
        released: true,
      });
    } else {
      self.touch_scroll_page(0.0, 0.0, Instant::now());
    }
  }

  /// Momentum reached an end of the content at a finger's velocity, in physical pixels per second: it carries
  /// the content past the end, and the content springs back. Content a finger pulled past the end and let go
  /// while still moving outward goes on outward at that velocity first, less of it the further it is pulled,
  /// as it followed the finger. Does nothing when the content is not at an end that way.
  pub fn touch_scroll_bounce(&mut self, velocity_x: f32, velocity_y: f32, now: Instant) {
    let scale = self.scale_factor();
    let Some(taker) = &self.touch_scroll.taker else {
      return;
    };
    let released = match &self.touch_scroll.spring {
      Some(spring) if spring.released => Some(spring.at(now.saturating_duration_since(spring.started).as_secs_f32())),
      Some(_) => return,
      None => None,
    };
    let (offsets, ends) = (taker.offsets(), taker.ends());
    let speed = |velocity: f32, offset: f32, end: f32| {
      let speed = -velocity / scale;
      if at_end(offset, end, speed) { speed } else { 0.0 }
    };
    let velocity = (
      speed(velocity_x, offsets.0, ends.0),
      speed(velocity_y, offsets.1, ends.1),
    );
    let from = released.unwrap_or((0.0, 0.0));
    let viewport = taker.viewport();
    // How much of a pull's travel content already drawn `shown` past the end follows: `rubber`'s slope.
    let follows = |shown: f32, size: f32| RUBBER_BAND * (1.0 - (shown.abs() / size.max(f32::EPSILON)).min(1.0)).powi(2);
    let velocity = if released.is_some() {
      (
        velocity.0 * follows(from.0, viewport.0),
        velocity.1 * follows(from.1, viewport.1),
      )
    } else {
      velocity
    };
    if velocity != (0.0, 0.0) {
      self.touch_scroll.spring = Some(Spring {
        from,
        velocity,
        started: now,
        released: false,
      });
    } else if let Some(spring) = &mut self.touch_scroll.spring {
      spring.released = false;
    }
  }

  /// The status bar was tapped: the page scrolls to its top, as a scroll view of the platform does. The page is
  /// the outermost container at the middle of the window that scrolls down and is away from its top; `false`
  /// when there is none.
  pub fn touch_scroll_to_top(&mut self, now: Instant) -> bool {
    self.rebuild_if_dirty();
    let size = self.window.info().logical_size();
    let (Some(root), Some(layout)) = (&self.root, &self.last_layout) else {
      return false;
    };
    let mut hits = Vec::new();
    hit_test_tree(
      root,
      layout,
      0.0,
      0.0,
      size.width / 2.0,
      size.height / 2.0,
      &self.layout_engine.radii(),
      &mut hits,
    );
    let page = hits.iter().rposition(|(node, _)| {
      node.scroll_container().is_some_and(|(state, direction)| {
        !matches!(direction, ScrollDirection::Horizontal) && state.scroll_y() > 0.0 && state.scrolls_to_top()
      })
    });
    let Some(page) = page else {
      return false;
    };
    let Some((state, _)) = hits[page].0.scroll_container() else {
      return false;
    };
    let around = hits[page..].iter().map(|(node, _)| node.node_id()).collect();
    drop(hits);
    // Whatever a gesture left moving gives way: a list on its way to a page is put there.
    if let Some(taker) = &self.touch_scroll.taker {
      taker.state.set_overscroll(0.0, 0.0);
      if let Some(page) = &self.touch_scroll.page {
        taker.state.set_scroll(page.to.0, page.to.1);
      }
    }
    self.touch_scroll.spring = None;
    self.touch_scroll.page = None;
    self.touch_scroll.caught = false;
    self.touch_scroll.pull = (0.0, 0.0);
    state.set_held(true);
    self.touch_scroll.to_top = Some(ToTop {
      from: state.scroll_y(),
      state,
      around,
      started: now,
    });
    self.needs_redraw = true;
    true
  }

  /// Moves content that is springing back, or on its way to its top, to where it is at `now`. `true` while it
  /// is still on its way: the shell runs another turn for the next step.
  pub fn touch_tick(&mut self, now: Instant) -> bool {
    let to_top = self.touch_to_top_tick(now) | self.touch_tap_highlight_tick(now) | self.touch_page_tick(now);
    let springs = self.touch_spring_tick(now);
    self.touch_scroll_rest(now);
    let asides = self.touch_asides_tick(now);
    springs || to_top || asides
  }

  /// Moves the lists on their way to a page beside the gesture to where they are at `now`. Each hears its
  /// steps as the gesture's own list does, and its end on its page.
  fn touch_asides_tick(&mut self, now: Instant) -> bool {
    if self.touch_scroll.asides.is_empty() {
      return false;
    }
    let mut going = Vec::new();
    let mut moved = false;
    for mut aside in std::mem::take(&mut self.touch_scroll.asides) {
      let (to, rests) = aside.page.at(now);
      let state = aside.state.clone();
      let was = (state.scroll_x(), state.scroll_y());
      let was_end = (state.content_height() - state.viewport_height()).max(0.0);
      state.set_scroll(to.0, to.1);
      // The node it had or, where a rebuild replaced that, the one that has its state now.
      aside.node = self.root.as_ref().and_then(|root| {
        aside
          .node
          .filter(|id| find_node_by_id(root, *id).is_some())
          .or_else(|| find_scroll_node_id_by_state(root, &state))
      });
      if let (Some(root), Some(node)) = (&self.root, aside.node) {
        mark_scroll_layout_dirty(root, node);
      }
      if let Some(node) = aside.node
        && (state.scroll_x(), state.scroll_y()) != was
      {
        moved = true;
        self.dispatch_scroll_handlers_for_node(node, 0.0, 0.0, 0.0, 0.0, ScrollPhase::Scroll, ScrollSource::Snap);
        self.dispatch_scroll_reach_handlers_for_node(
          node,
          0.0,
          0.0,
          0.0,
          0.0,
          ScrollPhase::Scroll,
          was.1,
          was_end,
          ScrollSource::Snap,
        );
      }
      if rests {
        let heard: Vec<NodeId> = aside.node.into_iter().collect();
        self.scroll_rests(&state, &heard, (0.0, 0.0), ScrollSource::Snap);
      } else {
        going.push(aside);
      }
    }
    self.needs_redraw = true;
    if moved {
      self.apply_reactive_updates_after_event();
    }
    let still = !going.is_empty();
    self.touch_scroll.asides = going;
    still
  }

  /// Moves content springing back from past an end to where it is at `now`.
  fn touch_spring_tick(&mut self, now: Instant) -> bool {
    let (Some(taker), Some(spring)) = (&self.touch_scroll.taker, &self.touch_scroll.spring) else {
      return false;
    };
    let elapsed = now.saturating_duration_since(spring.started);
    let shown = spring.at(elapsed.as_secs_f32());
    let rests = elapsed >= SPRING_MIN && shown.0.abs() < SPRING_REST && shown.1.abs() < SPRING_REST;
    if rests {
      taker.state.set_overscroll(0.0, 0.0);
      self.touch_scroll.spring = None;
    } else {
      taker.state.set_overscroll(shown.0, shown.1);
    }
    self.mark_touch_scroll_moved();
    !rests
  }

  fn touch_to_top_tick(&mut self, now: Instant) -> bool {
    let Some(to_top) = &self.touch_scroll.to_top else {
      return false;
    };
    let progress = now.saturating_duration_since(to_top.started).as_secs_f32() / TO_TOP.as_secs_f32();
    let done = progress >= 1.0;
    // Fast at first, easing into the top.
    let left = (1.0 - progress.min(1.0)).powi(3);
    let was = to_top.state.scroll_y();
    to_top.state.set_scroll(to_top.state.scroll_x(), to_top.from * left);
    if let Some(root) = &self.root {
      mark_moved(root, &to_top.around);
    }
    self.needs_redraw = true;
    // The list hears each step once the content is there, and the end of the way.
    let (state, list) = (to_top.state.clone(), to_top.around.first().copied());
    let list = self.root.as_ref().and_then(|root| {
      list
        .filter(|id| find_node_by_id(root, *id).is_some())
        .or_else(|| find_scroll_node_id_by_state(root, &state))
    });
    if let Some(list) = list
      && state.scroll_y() != was
    {
      self.dispatch_scroll_handlers_for_node(list, 0.0, 0.0, 0.0, 0.0, ScrollPhase::Scroll, ScrollSource::Code);
    }
    if done {
      self.touch_scroll.to_top = None;
      self.scroll_rests(&state, list.as_slice(), (0.0, 0.0), ScrollSource::Code);
    }
    !done
  }

  /// Moves a list that scrolls by pages to where it is at `now` on its way to a page.
  fn touch_page_tick(&mut self, now: Instant) -> bool {
    let (Some(taker), Some(page)) = (&self.touch_scroll.taker, &self.touch_scroll.page) else {
      return false;
    };
    let (to, rests) = page.at(now);
    let (was, was_end) = (taker.offsets(), taker.ends().1);
    taker.state.set_scroll(to.0, to.1);
    if rests {
      self.touch_scroll.page = None;
    }
    let moved = taker.offsets() != was;
    self.mark_touch_scroll_moved();
    // The list hears every step of the way once the content is there, the last one on the page itself.
    if moved && let Some(list) = self.touch_scroll_node() {
      let at = self.touch_scroll.at;
      self.dispatch_scroll_handlers_for_node(list, at.0, at.1, 0.0, 0.0, ScrollPhase::Scroll, ScrollSource::Snap);
      // A list that came to its first or its last page reached that end.
      self.dispatch_scroll_reach_handlers_for_node(
        list,
        at.0,
        at.1,
        0.0,
        0.0,
        ScrollPhase::Scroll,
        was.1,
        was_end,
        ScrollSource::Snap,
      );
      self.apply_reactive_updates_after_event();
    }
    !rests
  }

  /// Whether content is springing back, or on its way to its top or to a page; see
  /// [`touch_tick`](Self::touch_tick).
  pub fn touch_animating(&self) -> bool {
    self.touch_scroll.spring.is_some()
      || self.touch_scroll.to_top.is_some()
      || self.touch_scroll.page.is_some()
      || !self.touch_scroll.asides.is_empty()
      || self.touch_tap_highlight_showing()
  }

  /// The taker's content moved: its layout and those around it place it anew, and the frame is drawn.
  fn mark_touch_scroll_moved(&mut self) {
    if let Some(root) = &self.root {
      mark_moved(root, &self.touch_scroll.around);
    }
    self.needs_redraw = true;
  }
}

/// The content of the first of `around`, a scroll container, moved: its layout and those of the nodes around
/// it, which follow it in `around`, place it anew.
fn mark_moved(root: &Node, around: &[NodeId]) {
  for (index, target) in around.iter().enumerate() {
    let Some(node) = find_node_by_id(root, *target) else {
      continue;
    };
    if index == 0 {
      node.layout_cache.mark_local_dirty();
    }
    node.layout_cache.mark_descendant_dirty();
  }
}
