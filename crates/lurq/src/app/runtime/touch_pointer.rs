//! A finger as the tree sees it: its own events, and the pointer it stands for.
//!
//! **Touch events.** Every finger sends `on_touch_start`, `on_touch_move`, `on_touch_end` and `on_touch_cancel`
//! to what it landed on ([`Tree::touch_event`]), and one that rests sends `on_long_press`
//! ([`Tree::touch_long_press`]).
//!
//! **Gestures.** Two fingers on one element that listens for them pinch (`on_pinch`) and rotate
//! (`on_rotate`): each is told from the other, and from two fingers that only rest or scroll, once the fingers
//! moved apart or turned by more than a slop. From then on the two fingers are the gesture's, and
//! [`Tree::touch_event`] says so, for the shell to stop treating one of them as the pointer.
//!
//! The fingers that are down together, from the first to land until the last lifts, are also told as a whole
//! ([`Session`]): a tap of however many fingers (`on_tap`), their travel while it lasts (`on_pan`), a flick
//! when they lift (`on_swipe`), and one finger coming in from an edge of the window (`on_edge_pan`), which
//! from the left edge goes back in the history as a phone does, and from the right edge forward.
//!
//! As on a phone, a finger that taps and lands again at once, held and dragged, zooms with one finger
//! (`on_pinch`, down to zoom in), and three fingers are the editing commands where no handler took their
//! gesture: swiped left they undo and right redo, pinched in they copy and out paste, and tapped they open the
//! menu of editing commands ([`EditMenuRequest`](crate::app::window::EditMenuRequest)).
//!
//! **The pointer.** A touch shell gives the first finger to the tree as the pointer through the methods here
//! rather than through the mouse's, because a finger is not a mouse in three ways a list of the platform shows:
//!
//! - it hovers nothing. The mouse events still come, `on_mouse_enter` and `on_mouse_leave` among them, but no element
//!   takes its hovered style from a finger;
//! - a tap clicks when the finger lifts, and what it tapped stays pressed for a moment after, so that a tap quicker
//!   than a frame still shows;
//! - what a finger holds stays pressed only while the finger is near it. Moved further than [`PRESS_REACH`] from it,
//!   the element no longer shows as pressed, and shows so again when the finger comes back; lifted within that reach,
//!   even beside the element, the finger clicks it.

use std::{collections::VecDeque, time::Duration};

use super::{Tree, find_node_by_id, hit_test_tree, set_node_active};
use crate::{
  app::events::{
    EdgePanEvent, EditCommand, EventControl, GesturePhase, HistoryDirection, LongPressEvent, MouseButton, PanEvent,
    PinchEvent, RotateEvent, ScreenEdge, SwipeDirection, SwipeEvent, TapEvent, TouchEvent, TouchPhase,
  },
  core::NodeId,
  node::node::Node,
  time::Instant,
};

/// How far from what it presses a finger may go and still hold it, in logical pixels: a control's reach in
/// UIKit.
const PRESS_REACH: f32 = 70.0;

/// How far two fingers move apart or together before they pinch, in logical pixels.
const PINCH_SLOP: f32 = 8.0;
/// How far two fingers turn before they rotate, in radians: about six degrees.
const ROTATE_SLOP: f32 = 0.1;
/// A screen reports the fingers that moved in a frame one after the other: two moves of a gesture's fingers this
/// close are of one frame. One finger's move waits for its partner's of the same frame, unless the partner has
/// been still for [`GESTURE_STILL`], as a pivot is.
const GESTURE_FRAME: Duration = Duration::from_millis(4);
const GESTURE_STILL: Duration = Duration::from_millis(40);

/// How far fingers travel before they pan and are no longer a tap, in logical pixels.
const TRAVEL_SLOP: f32 = 10.0;
/// How long fingers may stay down and still tap.
const TAP_TIME: Duration = Duration::from_millis(500);
/// How far a flick goes at least, in logical pixels, how fast in logical pixels per second, and how much
/// further along its way than across it.
const SWIPE_DISTANCE: f32 = 50.0;
const SWIPE_SPEED: f32 = 300.0;
const SWIPE_STRAIGHTNESS: f32 = 2.0;
/// How near an edge of the window a finger lands to come in from it, in logical pixels.
const EDGE_WIDTH: f32 = 20.0;
/// How far across the window a finger from the left or the right edge goes to move the history, as a share of
/// the window's width, and the speed that moves it from any distance, in logical pixels per second.
const BACK_SHARE: f32 = 1.0 / 3.0;
const BACK_SPEED: f32 = 500.0;
/// The stretch of travel, ending now, a velocity is measured over.
const VELOCITY_WINDOW: Duration = Duration::from_millis(100);
/// How far one finger drags, in logical pixels, to zoom by two: down zooms in, up out, as on a phone.
const ONE_FINGER_ZOOM: f32 = 150.0;
/// How far three fingers pinch, as a share of how far apart they were, to copy (in) or paste (out).
const THREE_PINCH_IN: f32 = 0.6;
const THREE_PINCH_OUT: f32 = 1.6;

/// A finger of a session: its id, where it landed and where it is, in logical pixels.
type SessionFinger = (u64, (f32, f32), (f32, f32));

/// The fingers that are down together, from the first to land until the last lifts.
struct Session {
  started: Instant,
  /// The fingers down now, with where each landed and is.
  fingers: Vec<SessionFinger>,
  /// The most that were down at once.
  most: u32,
  /// The middle of where they landed, kept as fingers join.
  landed: (f32, f32),
  /// The elements every finger landed on, from the innermost out.
  chain: Vec<NodeId>,
  /// The middle of the fingers, and how far it went since they landed. A finger that joins or leaves moves
  /// the middle, which is not travel.
  middle: (f32, f32),
  translation: (f32, f32),
  /// The translation over the last [`VELOCITY_WINDOW`].
  samples: VecDeque<(Instant, (f32, f32))>,
  /// A finger went further than [`TRAVEL_SLOP`]: this is no tap.
  travelled: bool,
  /// The pan was started, with the translation last told.
  pan: Option<(f32, f32)>,
  /// The edge the one finger landed at, and whether it came in from it.
  edge: Option<(ScreenEdge, bool)>,
  /// A handler of the edge pan prevented the default: lifting does not go back.
  edge_prevented: bool,
  /// The one finger landed as the second tap of a double tap: held and dragged, it zooms.
  double_held: bool,
  /// The one-finger zoom under way: how far down the finger was when it started, and the scale last told.
  zoom: Option<(f32, f32)>,
  /// Three fingers: how far they were from their middle, on average, when the third landed, and now.
  spread: Option<(f32, f32)>,
  /// How fast the fingers went when the first of several lifted: the others lift a moment later, and a flick of
  /// them all is told by it.
  parting: Option<(f32, f32)>,
}

impl Session {
  fn velocity(&self, now: Instant) -> (f32, f32) {
    let Some((at, from)) = self
      .samples
      .iter()
      .find(|(at, _)| now.saturating_duration_since(*at) <= VELOCITY_WINDOW)
    else {
      return (0.0, 0.0);
    };
    let seconds = now.saturating_duration_since(*at).as_secs_f32();
    if seconds <= 0.0 {
      return (0.0, 0.0);
    }
    (
      (self.translation.0 - from.0) / seconds,
      (self.translation.1 - from.1) / seconds,
    )
  }
}

/// How far fingers are from their middle, on average.
fn spread_of(fingers: &[SessionFinger]) -> f32 {
  let middle = middle_of(fingers);
  let count = fingers.len().max(1) as f32;
  fingers
    .iter()
    .map(|(_, _, at)| (at.0 - middle.0).hypot(at.1 - middle.1))
    .sum::<f32>()
    / count
}

fn middle_of(fingers: &[SessionFinger]) -> (f32, f32) {
  let count = fingers.len().max(1) as f32;
  let sum = fingers
    .iter()
    .fold((0.0, 0.0), |sum, (_, _, at)| (sum.0 + at.0, sum.1 + at.1));
  (sum.0 / count, sum.1 / count)
}

/// Sends an event to the handlers `$handlers` of the elements of `$chain`, innermost first, until one stops its
/// propagation. Gives the event's control, which tells whether the default was prevented.
macro_rules! bubble {
  ($root:expr, $chain:expr, $handlers:ident, |$target:ident, $control:ident| $event:expr) => {{
    let $control = EventControl::new();
    'bubble: for $target in $chain.iter().copied() {
      let Some(node) = find_node_by_id($root, $target) else {
        continue;
      };
      let event = $event;
      for handler in &node.events.$handlers {
        handler.call(&event);
        if event.immediate_propagation_stopped() {
          break 'bubble;
        }
      }
      if event.propagation_stopped() {
        break;
      }
    }
    $control
  }};
}

/// Whose a finger is, as [`Tree::touch_event`] tells the shell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TouchRoute {
  /// The shell's: the first such finger is the pointer.
  Pointer,
  /// The app's: a handler of its `on_touch_start` prevented the default. It is not the pointer.
  Taken,
  /// A gesture's: it pinches or rotates with another finger. Neither is the pointer, and what the pointer was
  /// doing ends without a click.
  Gesture,
}

/// A finger that is down, for its events.
pub(super) struct ActiveTouch {
  id: u64,
  /// What it landed on, from the innermost element out.
  targets: Vec<NodeId>,
  /// Where it is, in logical pixels.
  position: (f32, f32),
  /// A handler of its `on_touch_start` took it: it is not the pointer.
  taken: bool,
  /// It pinched or rotated: it is not the pointer, even once the other finger lifted.
  gestured: bool,
}

/// Two fingers on one element that listens for a pinch or a rotation.
struct Gesture {
  fingers: [u64; 2],
  /// The elements both fingers are on, from the innermost out.
  chain: Vec<NodeId>,
  /// How far apart the fingers were when the second landed, and which way the line between them pointed.
  landed_distance: f32,
  angle: f32,
  /// How far they turned since, turns past half a circle included.
  turned: f32,
  /// The distance the pinch started at and the scale last told, once they pinch.
  pinch: Option<(f32, f32)>,
  /// The turn the rotation started at and the rotation last told, once they rotate.
  rotate: Option<(f32, f32)>,
  /// When each finger last moved: a frame's moves are measured together.
  moved_at: [Option<Instant>; 2],
}

/// How far apart two points are, which way the line from the first to the second points, and its middle.
fn between(a: (f32, f32), b: (f32, f32)) -> (f32, f32, (f32, f32)) {
  let (dx, dy) = (b.0 - a.0, b.1 - a.1);
  (dx.hypot(dy), dy.atan2(dx), ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0))
}

/// What the finger that is the pointer presses.
struct Press {
  /// Where it landed, in physical pixels.
  position: (f32, f32),
  /// The element it holds, in logical pixels: x, y, width, height.
  rect: (f32, f32, f32, f32),
  /// Whether the elements under the press show as pressed.
  lit: bool,
}

impl Press {
  /// Whether the point, in logical pixels, is on the element (`reach` of 0) or within `reach` of it.
  fn holds(&self, x: f32, y: f32, reach: f32) -> bool {
    let (left, top, width, height) = self.rect;
    x >= left - reach && x <= left + width + reach && y >= top - reach && y <= top + height + reach
  }
}

#[derive(Default)]
pub(super) struct TouchPointerState {
  pub(super) touches: Vec<ActiveTouch>,
  gesture: Option<Gesture>,
  session: Option<Session>,
  press: Option<Press>,
  /// A finger is the pointer: nothing is hovered.
  pub(super) down: bool,
  /// The touch that landed last is a pen's: it moves the app's pointer neither way, and what it presses as the
  /// pointer is a pen's (`PointerKind::Pen`) until another lands.
  pub(super) pen: bool,
  /// The fingers down together are no click: a second one landed, or a handler took their tap. The pointer's
  /// release then ends its press without a click. Cleared when the next fingers land.
  no_click: bool,
  /// The last tap, for the count of the next: when, where, of how many fingers, and its count.
  last_tap: Option<LastTap>,
  /// What pressed for a click held back and told now.
  pub(super) replaying: Option<crate::app::events::PointerKind>,
  /// The mouse's events come from a pen: a browser gives a pen's pointer events to winit as the mouse's, and the
  /// shell says so from their `pointerType`.
  pub(super) mouse_pen: bool,
  /// The elements a tap left showing as pressed, and until when.
  tap_highlight: Option<(Vec<NodeId>, Instant)>,
}

/// A tap, as the next is counted from.
#[derive(Clone, Copy)]
struct LastTap {
  at: Instant,
  position: (f32, f32),
  fingers: u32,
  count: u32,
}

/// How far apart, in logical pixels, two taps are one double tap at most: a fingertip's width and a half.
const DOUBLE_TAP_DISTANCE: f32 = 45.0;

impl Tree {
  /// Gives a finger's event to what the finger landed on: `on_touch_start` and the rest, bubbling from the
  /// innermost element out, and the pinch or rotation it makes with another finger. `x` and `y` are physical
  /// pixels. Tells whose the finger is: the shell makes it the pointer only when it is its own.
  ///
  /// It happens now, or at the time of the tree's clock of presses where one is set
  /// ([`set_press_clock`](Self::set_press_clock)): a test gives a drag's speed by that clock, not by sleeping.
  pub fn touch_event(&mut self, id: u64, phase: TouchPhase, x: f32, y: f32, force: Option<f32>) -> TouchRoute {
    self.touch_event_by(id, phase, x, y, force, false)
  }

  /// [`touch_event`](Self::touch_event) of a finger or, where `pen`, of a pen: a finger makes the app's pointer
  /// coarse ([`App::pointer`](crate::app::App::pointer)), a pen leaves it as it is.
  pub(crate) fn touch_event_by(
    &mut self,
    id: u64,
    phase: TouchPhase,
    x: f32,
    y: f32,
    force: Option<f32>,
    pen: bool,
  ) -> TouchRoute {
    if phase == TouchPhase::Start {
      self.touch_pointer.pen = pen;
    }
    let now = self.press_clock.unwrap_or_else(Instant::now);
    self.touch_event_at(id, phase, x, y, force, now)
  }

  /// What presses now as the pointer: a finger or a pen while one is down, else the mouse.
  pub(super) fn pointer_kind_now(&self) -> crate::app::events::PointerKind {
    use crate::app::events::PointerKind;
    if let Some(kind) = self.touch_pointer.replaying {
      return kind;
    }
    match (self.touch_pointer.down, self.touch_pointer.pen) {
      (false, _) if self.touch_pointer.mouse_pen => PointerKind::Pen,
      (false, _) => PointerKind::Mouse,
      (true, false) => PointerKind::Touch,
      (true, true) => PointerKind::Pen,
    }
  }

  /// Whether the mouse's events to come are a pen's, as a browser's `pointerType` says: they press as a pen
  /// (`PointerKind::Pen`) and leave the app's pointer as it is.
  #[cfg_attr(not(all(feature = "winit", target_arch = "wasm32")), allow(dead_code))]
  pub(crate) fn set_mouse_is_pen(&mut self, pen: bool) {
    self.touch_pointer.mouse_pen = pen;
  }

  /// Whether the press of the finger that is the pointer ends without a click: a second finger landed with it,
  /// or a handler took its tap.
  pub(super) fn touch_clicks_nothing(&self) -> bool {
    self.touch_pointer.down && self.touch_pointer.no_click
  }

  /// [`touch_event`](Self::touch_event) at a time of the caller's, for tests of what depends on how fast
  /// fingers go.
  pub(super) fn touch_event_at(
    &mut self,
    id: u64,
    phase: TouchPhase,
    x: f32,
    y: f32,
    force: Option<f32>,
    now: Instant,
  ) -> TouchRoute {
    let scale = self.scale_factor();
    let (lx, ly) = (x / scale, y / scale);
    if phase == TouchPhase::Start {
      if !self.touch_pointer.pen {
        self.pointer_input(crate::app::Pointer::Coarse);
      }
      self.rebuild_if_dirty();
      let mut targets = Vec::new();
      if let (Some(root), Some(layout)) = (&self.root, &self.last_layout) {
        let mut hits = Vec::new();
        hit_test_tree(root, layout, 0.0, 0.0, lx, ly, &self.layout_engine.radii(), &mut hits);
        // A touch on a disabled element is still a touch on the elements it is in: a pan over it moves them.
        super::take_disabled_hit(&mut hits);
        targets.extend(hits.iter().map(|(node, _)| node.node_id()));
      }
      self.touch_pointer.touches.retain(|touch| touch.id != id);
      self.touch_pointer.touches.push(ActiveTouch {
        id,
        targets,
        position: (lx, ly),
        taken: false,
        gestured: false,
      });
    }
    let Some(index) = self.touch_pointer.touches.iter().position(|touch| touch.id == id) else {
      return TouchRoute::Pointer;
    };
    self.touch_pointer.touches[index].position = (lx, ly);

    let control = EventControl::new();
    if let Some(root) = &self.root {
      for target in &self.touch_pointer.touches[index].targets {
        let Some(node) = find_node_by_id(root, *target) else {
          continue;
        };
        let event = TouchEvent {
          id,
          phase,
          x: lx,
          y: ly,
          force,
          target_id: *target,
          control: control.clone(),
        };
        let handlers = match phase {
          TouchPhase::Start => &node.events.on_touch_start,
          TouchPhase::Move => &node.events.on_touch_move,
          TouchPhase::End => &node.events.on_touch_end,
          TouchPhase::Cancel => &node.events.on_touch_cancel,
        };
        for handler in handlers {
          handler.call(&event);
          if event.immediate_propagation_stopped() {
            break;
          }
        }
        if event.propagation_stopped() {
          break;
        }
      }
    }

    if phase == TouchPhase::Start {
      self.touch_pointer.touches[index].taken = control.default_prevented();
    }
    match phase {
      TouchPhase::Start => self.touch_gesture_landed(now),
      TouchPhase::Move => self.touch_gesture_moved(id, now),
      TouchPhase::End | TouchPhase::Cancel => self.touch_gesture_lifted(id),
    }
    if !self.touch_pointer.touches[index].taken {
      self.touch_session(id, phase, (lx, ly), now);
    }
    let touch = &self.touch_pointer.touches[index];
    let route = if touch.taken {
      TouchRoute::Taken
    } else if touch.gestured {
      TouchRoute::Gesture
    } else {
      TouchRoute::Pointer
    };
    if matches!(phase, TouchPhase::End | TouchPhase::Cancel) {
      self.touch_pointer.touches.remove(index);
    }
    self.apply_reactive_updates_after_event();
    route
  }

  /// Follows the fingers that are down together: their pan while it lasts, and their tap or swipe, and the
  /// way back from the left edge, when the last one lifts.
  fn touch_session(&mut self, id: u64, phase: TouchPhase, at: (f32, f32), now: Instant) {
    let window = self.window.info().logical_size();
    // A finger that lands where one tapped just before is the second tap of a double tap.
    let (interval, clock) = (
      self.double_click_interval.unwrap_or(super::DOUBLE_CLICK_INTERVAL),
      self.press_clock.unwrap_or(now),
    );
    let double_held = self.touch_pointer.last_tap.is_some_and(|last| {
      last.fingers == 1
        && clock.saturating_duration_since(last.at) <= interval
        && (last.position.0 - at.0).hypot(last.position.1 - at.1) <= DOUBLE_TAP_DISTANCE
    });
    let state = &mut self.touch_pointer;
    let Some(index) = state.touches.iter().position(|touch| touch.id == id) else {
      return;
    };
    match phase {
      TouchPhase::Start => {
        let targets = &state.touches[index].targets;
        match &mut state.session {
          Some(session) => {
            session.fingers.push((id, at, at));
            session.most = session.most.max(session.fingers.len() as u32);
            // A click is one finger's: two together are a tap of two, or a gesture.
            state.no_click = true;
            session.chain.retain(|target| targets.contains(target));
            session.middle = middle_of(&session.fingers);
            let count = session.fingers.len() as f32;
            session.landed = (
              session.landed.0 + (at.0 - session.landed.0) / count,
              session.landed.1 + (at.1 - session.landed.1) / count,
            );
            session.samples.clear();
            // Only one finger comes in from an edge, and only one zooms.
            if session.edge.is_some_and(|(_, came_in)| !came_in) {
              session.edge = None;
            }
            session.double_held = false;
            // Three fingers are the editing commands' gesture: none of them is the pointer from now on.
            session.spread = (session.fingers.len() == 3).then(|| {
              let spread = spread_of(&session.fingers);
              (spread, spread)
            });
            if session.fingers.len() == 3 {
              let held: Vec<u64> = session.fingers.iter().map(|(id, ..)| *id).collect();
              for touch in &mut state.touches {
                touch.gestured |= held.contains(&touch.id);
              }
            }
          }
          None => {
            let edge = if at.0 <= EDGE_WIDTH {
              Some(ScreenEdge::Left)
            } else if at.0 >= window.width - EDGE_WIDTH {
              Some(ScreenEdge::Right)
            } else if at.1 <= EDGE_WIDTH {
              Some(ScreenEdge::Top)
            } else if at.1 >= window.height - EDGE_WIDTH {
              Some(ScreenEdge::Bottom)
            } else {
              None
            };
            state.no_click = false;
            state.session = Some(Session {
              started: now,
              fingers: vec![(id, at, at)],
              most: 1,
              landed: at,
              chain: targets.clone(),
              middle: at,
              translation: (0.0, 0.0),
              samples: VecDeque::new(),
              travelled: false,
              pan: None,
              edge: edge.map(|edge| (edge, false)),
              edge_prevented: false,
              double_held,
              zoom: None,
              spread: None,
              parting: None,
            });
          }
        }
      }
      TouchPhase::Move => self.touch_session_moved(id, at, now),
      TouchPhase::End | TouchPhase::Cancel => self.touch_session_lifted(id, phase == TouchPhase::Cancel, now),
    }
  }

  fn touch_session_moved(&mut self, id: u64, at: (f32, f32), now: Instant) {
    let state = &mut self.touch_pointer;
    let Some(session) = &mut state.session else {
      return;
    };
    let Some(finger) = session.fingers.iter_mut().find(|(held, ..)| *held == id) else {
      return;
    };
    finger.2 = at;
    session.travelled |= (at.0 - finger.1.0).hypot(at.1 - finger.1.1) > TRAVEL_SLOP;
    let middle = middle_of(&session.fingers);
    session.translation.0 += middle.0 - session.middle.0;
    session.translation.1 += middle.1 - session.middle.1;
    session.middle = middle;
    session.samples.push_back((now, session.translation));
    while session
      .samples
      .front()
      .is_some_and(|(at, _)| now.saturating_duration_since(*at) > VELOCITY_WINDOW)
    {
      session.samples.pop_front();
    }
    if session.fingers.len() == 3
      && let Some((_, spread)) = &mut session.spread
    {
      *spread = spread_of(&session.fingers);
    }
    let Some(root) = &self.root else {
      return;
    };
    let (translation, velocity, fingers) = (session.translation, session.velocity(now), session.fingers.len() as u32);

    // The second tap of a double tap, held and dragged: one finger zooms where it landed, down to zoom in.
    if session.double_held && (session.zoom.is_some() || session.travelled) {
      let listens = || {
        session
          .chain
          .iter()
          .any(|target| find_node_by_id(root, *target).is_some_and(|node| !node.events.on_pinch.is_empty()))
      };
      if session.zoom.is_none() && !listens() {
        // Nothing zooms there: an ordinary drag.
        session.double_held = false;
      } else {
        let down = at.1 - session.landed.1;
        let (phase, scale, delta) = match &mut session.zoom {
          None => {
            session.zoom = Some((down, 1.0));
            (GesturePhase::Start, 1.0, 1.0)
          }
          Some((from, last)) => {
            let scale = 2f32.powf((down - *from) / ONE_FINGER_ZOOM);
            let delta = scale / *last;
            *last = scale;
            (GesturePhase::Change, scale, delta)
          }
        };
        if let Some(touch) = state.touches.iter_mut().find(|touch| touch.id == id) {
          touch.gestured = true;
        }
        send_pinch(root, &session.chain, phase, scale, delta, session.landed);
        return;
      }
    }

    // One finger coming in from the edge it landed at.
    if let Some((edge, came_in)) = &mut session.edge {
      let (inward, across) = match edge {
        ScreenEdge::Left => (translation.0, translation.1),
        ScreenEdge::Right => (-translation.0, translation.1),
        ScreenEdge::Top => (translation.1, translation.0),
        ScreenEdge::Bottom => (-translation.1, translation.0),
      };
      // From the left or the right edge a finger goes back or forward wherever something hears it: a handler on the
      // way, or a router anywhere in the window (`runtime/navigation.rs`).
      let sideways = matches!(edge, ScreenEdge::Left | ScreenEdge::Right);
      let listens = || {
        session.chain.iter().any(|target| {
          find_node_by_id(root, *target).is_some_and(|node| {
            !node.events.on_edge_pan.is_empty() || (sideways && !node.events.on_history_swipe.is_empty())
          })
        }) || (sideways && super::navigation::hears_history(root))
      };
      let starts = !*came_in && inward > TRAVEL_SLOP && inward > across.abs() && listens();
      if !*came_in && !starts && session.travelled {
        // It went another way: an ordinary touch that happened to land by the edge.
        session.edge = None;
      } else if starts || *came_in {
        let phase = if starts {
          GesturePhase::Start
        } else {
          GesturePhase::Change
        };
        *came_in = true;
        let (edge, inward_speed) = (
          *edge,
          match edge {
            ScreenEdge::Left => velocity.0,
            ScreenEdge::Right => -velocity.0,
            ScreenEdge::Top => velocity.1,
            ScreenEdge::Bottom => -velocity.1,
          },
        );
        let control = bubble!(root, session.chain, on_edge_pan, |target, control| EdgePanEvent {
          phase,
          edge,
          translation: inward.max(0.0),
          velocity: inward_speed,
          x: at.0,
          y: at.1,
          target_id: target,
          control: control.clone(),
        });
        session.edge_prevented |= control.default_prevented();
        if let Some(touch) = state.touches.iter_mut().find(|touch| touch.id == id) {
          touch.gestured = true;
        }
        return;
      }
    }

    // The fingers' travel.
    let starts = session.pan.is_none() && translation.0.hypot(translation.1) > TRAVEL_SLOP;
    if !starts && session.pan.is_none() {
      return;
    }
    let last = session.pan.unwrap_or((0.0, 0.0));
    session.pan = Some(translation);
    let phase = if starts {
      GesturePhase::Start
    } else {
      GesturePhase::Change
    };
    let control = bubble!(root, session.chain, on_pan, |target, control| PanEvent {
      phase,
      fingers,
      translation_x: translation.0,
      translation_y: translation.1,
      delta_x: translation.0 - last.0,
      delta_y: translation.1 - last.1,
      velocity_x: velocity.0,
      velocity_y: velocity.1,
      x: middle.0,
      y: middle.1,
      target_id: target,
      control: control.clone(),
    });
    if starts && control.default_prevented() {
      // The pan's handler took the fingers: they are no longer the pointer's.
      let held: Vec<u64> = session.fingers.iter().map(|(id, ..)| *id).collect();
      for touch in &mut state.touches {
        touch.gestured |= held.contains(&touch.id);
      }
    }
  }

  fn touch_session_lifted(&mut self, id: u64, cancelled: bool, now: Instant) {
    let window = self.window.info().logical_size();
    let Some(session) = &mut self.touch_pointer.session else {
      return;
    };
    let Some(index) = session.fingers.iter().position(|(held, ..)| *held == id) else {
      return;
    };
    let lifted_at = session.fingers[index].2;
    if session.fingers.len() > 1 {
      if session.parting.is_none() && !session.samples.is_empty() {
        session.parting = Some(session.velocity(now));
      }
      session.fingers.remove(index);
      session.middle = middle_of(&session.fingers);
      session.samples.clear();
      return;
    }
    let Some(session) = self.touch_pointer.session.take() else {
      return;
    };
    let velocity = match session.parting {
      Some(parting) if session.samples.is_empty() => parting,
      _ => session.velocity(now),
    };
    let (translation, middle) = (session.translation, session.middle);
    let mut history = None;
    // What three fingers did, where no handler took their gesture: an editing command, or its menu.
    let mut edit = None;
    let mut menu = false;
    let three = session.most == 3;
    let three_pinch = session
      .spread
      .filter(|_| three && !cancelled)
      .and_then(|(landed, spread)| {
        let share = spread / landed.max(1.0);
        if share < THREE_PINCH_IN {
          Some(EditCommand::Copy)
        } else if share > THREE_PINCH_OUT {
          Some(EditCommand::Paste)
        } else {
          None
        }
      });
    if let Some(root) = &self.root {
      if let Some((_, scale)) = session.zoom {
        send_pinch(root, &session.chain, GesturePhase::End, scale, 1.0, session.landed);
      } else if let Some((edge, true)) = session.edge {
        let (inward, inward_speed) = match edge {
          ScreenEdge::Left => (translation.0, velocity.0),
          ScreenEdge::Right => (-translation.0, -velocity.0),
          ScreenEdge::Top => (translation.1, velocity.1),
          ScreenEdge::Bottom => (-translation.1, -velocity.1),
        };
        let control = bubble!(root, session.chain, on_edge_pan, |target, control| EdgePanEvent {
          phase: GesturePhase::End,
          edge,
          translation: inward.max(0.0),
          velocity: inward_speed,
          x: lifted_at.0,
          y: lifted_at.1,
          target_id: target,
          control: control.clone(),
        });
        let far_enough = !cancelled
          && !session.edge_prevented
          && !control.default_prevented()
          && (inward > window.width * BACK_SHARE || inward_speed > BACK_SPEED);
        history = match edge {
          ScreenEdge::Left if far_enough => Some(HistoryDirection::Back),
          ScreenEdge::Right if far_enough => Some(HistoryDirection::Forward),
          _ => None,
        };
      } else {
        if session.pan.is_some() {
          let last = session.pan.unwrap_or(translation);
          bubble!(root, session.chain, on_pan, |target, control| PanEvent {
            phase: GesturePhase::End,
            fingers: session.most,
            translation_x: translation.0,
            translation_y: translation.1,
            delta_x: translation.0 - last.0,
            delta_y: translation.1 - last.1,
            velocity_x: velocity.0,
            velocity_y: velocity.1,
            x: middle.0,
            y: middle.1,
            target_id: target,
            control: control.clone(),
          });
        }
        let lasted = now.saturating_duration_since(session.started);
        let (along, across, direction) = if translation.0.abs() >= translation.1.abs() {
          let direction = if translation.0 < 0.0 {
            SwipeDirection::Left
          } else {
            SwipeDirection::Right
          };
          (translation.0.abs(), translation.1.abs(), direction)
        } else {
          let direction = if translation.1 < 0.0 {
            SwipeDirection::Up
          } else {
            SwipeDirection::Down
          };
          (translation.1.abs(), translation.0.abs(), direction)
        };
        let speed = velocity.0.hypot(velocity.1);
        if cancelled {
          // The system took the touch: it was neither a tap nor a swipe.
        } else if !session.travelled && lasted <= TAP_TIME {
          // Counted on the clock of presses, as a double click is.
          let at = self.press_clock.unwrap_or(now);
          let count = match self.touch_pointer.last_tap {
            Some(last)
              if last.fingers == session.most
                && at.saturating_duration_since(last.at)
                  <= self.double_click_interval.unwrap_or(super::DOUBLE_CLICK_INTERVAL)
                && (last.position.0 - session.landed.0).hypot(last.position.1 - session.landed.1)
                  <= DOUBLE_TAP_DISTANCE =>
            {
              last.count + 1
            }
            _ => 1,
          };
          self.touch_pointer.last_tap = Some(LastTap {
            at,
            position: session.landed,
            fingers: session.most,
            count,
          });
          let tap = |target: NodeId, control: &EventControl| TapEvent {
            fingers: session.most,
            count,
            x: session.landed.0,
            y: session.landed.1,
            target_id: target,
            control: control.clone(),
          };
          let control = bubble!(root, session.chain, on_tap, |target, control| tap(target, &control));
          let mut taken = control.default_prevented();
          if count == 2 {
            let control = bubble!(root, session.chain, on_double_tap, |target, control| tap(
              target, &control
            ));
            taken |= control.default_prevented();
          }
          // A tap a handler took clicks nothing, as a long press that was taken does.
          if taken {
            self.touch_pointer.no_click = true;
          }
          menu = three && !taken;
        } else if let Some(command) = three_pinch {
          edit = Some(command);
        } else if along >= SWIPE_DISTANCE && along >= across * SWIPE_STRAIGHTNESS && speed >= SWIPE_SPEED {
          let control = bubble!(root, session.chain, on_swipe, |target, control| SwipeEvent {
            direction,
            fingers: session.most,
            velocity: speed,
            x: session.landed.0,
            y: session.landed.1,
            target_id: target,
            control: control.clone(),
          });
          if three && !control.default_prevented() {
            edit = match direction {
              SwipeDirection::Left => Some(EditCommand::Undo),
              SwipeDirection::Right => Some(EditCommand::Redo),
              _ => None,
            };
          }
        }
      }
    }
    if let Some(direction) = history {
      self.dispatch_history_swipe(
        session.landed.0,
        session.landed.1,
        direction,
        crate::app::events::NavigationSource::Edge,
      );
    }
    if let Some(command) = edit {
      self.three_finger_edit(command);
    }
    if menu {
      self.open_edit_menu(
        session.landed,
        vec![
          EditCommand::Undo,
          EditCommand::Cut,
          EditCommand::Copy,
          EditCommand::Paste,
          EditCommand::Redo,
        ],
      );
    }
  }

  /// An editing command three fingers made, run as its key would be on what has the focus. On iOS the system's
  /// text input makes these itself where it has the focus, and lurq leaves it to it.
  fn three_finger_edit(&mut self, command: EditCommand) {
    if self.layout_engine.native_text_selection() && self.focused_text_input() {
      return;
    }
    let synthetic = crate::app::events::is_synthetic_input();
    self.dispatch_edit(command, None, synthetic);
    self.apply_reactive_updates_after_event();
  }

  /// Opens the menu of editing commands at a point in logical pixels.
  pub(crate) fn open_edit_menu(&mut self, at: (f32, f32), commands: Vec<EditCommand>) {
    self.window.set_edit_menu(Some(crate::app::window::EditMenuRequest {
      x: at.0,
      y: at.1,
      commands,
    }));
    self.apply_reactive_updates_after_event();
  }

  /// Closes the menu of editing commands a finger opened, if it is open.
  pub(crate) fn close_edit_menu(&mut self) {
    if self.window.edit_menu_signal().get_untracked().is_some() {
      self.window.set_edit_menu(None);
    }
  }

  /// A finger landed: with another that is down, on an element that listens for a pinch or a rotation, it may
  /// start one.
  fn touch_gesture_landed(&mut self, now: Instant) {
    if self.touch_pointer.gesture.is_some() {
      // A third finger: the two are no pinch nor rotation, but part of a gesture of three.
      let free = self.touch_pointer.touches.iter().filter(|touch| !touch.taken).count();
      if free > 2
        && let Some(gesture) = self.touch_pointer.gesture.take()
        && let Some(root) = &self.root
      {
        let middle = self
          .touch_pointer
          .touches
          .first()
          .map_or((0.0, 0.0), |touch| touch.position);
        if let Some((_, scale)) = gesture.pinch {
          send_pinch(root, &gesture.chain, GesturePhase::End, scale, 1.0, middle);
        }
        if let Some((_, rotation)) = gesture.rotate {
          send_rotate(root, &gesture.chain, GesturePhase::End, rotation, 0.0, middle);
        }
      }
      return;
    }
    let Some(root) = &self.root else {
      return;
    };
    let mut free = self
      .touch_pointer
      .touches
      .iter()
      .filter(|touch| !touch.taken && !touch.gestured);
    let (Some(first), Some(second), None) = (free.next(), free.next(), free.next()) else {
      return;
    };
    let chain: Vec<NodeId> = first
      .targets
      .iter()
      .copied()
      .filter(|target| second.targets.contains(target))
      .collect();
    let listens = chain.iter().any(|target| {
      find_node_by_id(root, *target)
        .is_some_and(|node| !node.events.on_pinch.is_empty() || !node.events.on_rotate.is_empty())
    });
    if !listens {
      return;
    }
    let (distance, angle, _) = between(first.position, second.position);
    self.touch_pointer.gesture = Some(Gesture {
      fingers: [first.id, second.id],
      chain,
      landed_distance: distance,
      angle,
      turned: 0.0,
      pinch: None,
      rotate: None,
      moved_at: [Some(now), Some(now)],
    });
  }

  fn touch_gesture_moved(&mut self, id: u64, now: Instant) {
    let state = &mut self.touch_pointer;
    let Some(gesture) = &mut state.gesture else {
      return;
    };
    let Some(mine) = gesture.fingers.iter().position(|finger| *finger == id) else {
      return;
    };
    gesture.moved_at[mine] = Some(now);
    // The partner's move of this frame is still to come: the line between them is measured once both moved.
    let waits = gesture.moved_at[1 - mine].is_some_and(|at| {
      let since = now.saturating_duration_since(at);
      since > GESTURE_FRAME && since < GESTURE_STILL
    });
    if waits {
      return;
    }
    let position = |finger: u64| {
      state
        .touches
        .iter()
        .find(|touch| touch.id == finger)
        .map(|touch| touch.position)
    };
    let (Some(first), Some(second)) = (position(gesture.fingers[0]), position(gesture.fingers[1])) else {
      return;
    };
    let (distance, angle, middle) = between(first, second);
    // The shorter way round from the last angle: the line between the fingers turns a little at a time.
    let turn = (angle - gesture.angle + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    gesture.angle = angle;
    gesture.turned += turn;

    let mut pinch = None;
    match &mut gesture.pinch {
      None if (distance - gesture.landed_distance).abs() > PINCH_SLOP && distance > 0.0 => {
        gesture.pinch = Some((distance, 1.0));
        pinch = Some((GesturePhase::Start, 1.0, 1.0));
      }
      Some((from, last)) if *from > 0.0 => {
        let scale = distance / *from;
        if scale != *last && *last > 0.0 {
          pinch = Some((GesturePhase::Change, scale, scale / *last));
          *last = scale;
        }
      }
      _ => {}
    }
    let mut rotate = None;
    match &mut gesture.rotate {
      None if gesture.turned.abs() > ROTATE_SLOP => {
        gesture.rotate = Some((gesture.turned, 0.0));
        rotate = Some((GesturePhase::Start, 0.0, 0.0));
      }
      Some((from, last)) => {
        let rotation = gesture.turned - *from;
        if rotation != *last {
          rotate = Some((GesturePhase::Change, rotation, rotation - *last));
          *last = rotation;
        }
      }
      None => {}
    }
    if pinch.is_none() && rotate.is_none() {
      return;
    }
    let (fingers, chain) = (gesture.fingers, gesture.chain.clone());
    for touch in &mut state.touches {
      touch.gestured |= fingers.contains(&touch.id);
    }
    if let Some(root) = &self.root {
      if let Some((phase, scale, delta)) = pinch {
        send_pinch(root, &chain, phase, scale, delta, middle);
      }
      if let Some((phase, rotation, delta)) = rotate {
        send_rotate(root, &chain, phase, rotation, delta, middle);
      }
    }
  }

  /// One of a gesture's fingers lifted: what was under way ends where it was.
  fn touch_gesture_lifted(&mut self, id: u64) {
    if !self
      .touch_pointer
      .gesture
      .as_ref()
      .is_some_and(|gesture| gesture.fingers.contains(&id))
    {
      return;
    }
    let Some(gesture) = self.touch_pointer.gesture.take() else {
      return;
    };
    let position = |finger: u64| {
      self
        .touch_pointer
        .touches
        .iter()
        .find(|touch| touch.id == finger)
        .map(|touch| touch.position)
    };
    let (Some(first), Some(second), Some(root)) =
      (position(gesture.fingers[0]), position(gesture.fingers[1]), &self.root)
    else {
      return;
    };
    let (_, _, middle) = between(first, second);
    if let Some((_, scale)) = gesture.pinch {
      send_pinch(root, &gesture.chain, GesturePhase::End, scale, 1.0, middle);
    }
    if let Some((_, rotation)) = gesture.rotate {
      send_rotate(root, &gesture.chain, GesturePhase::End, rotation, 0.0, middle);
    }
  }

  /// A finger rested at the point, in physical pixels: `on_long_press`, bubbling from the innermost element
  /// there out. `true` when a handler took it, and the finger then clicks nothing when it lifts.
  pub fn touch_long_press(&mut self, x: f32, y: f32) -> bool {
    let scale = self.scale_factor();
    let (lx, ly) = (x / scale, y / scale);
    self.rebuild_if_dirty();
    let mut taken = false;
    let mut on_text_input = false;
    if let (Some(root), Some(layout)) = (&self.root, &self.last_layout) {
      let mut hits = Vec::new();
      hit_test_tree(root, layout, 0.0, 0.0, lx, ly, &self.layout_engine.radii(), &mut hits);
      // A finger resting on a disabled element presses nothing.
      if super::take_disabled_hit(&mut hits).is_some() {
        hits.clear();
      }
      on_text_input = hits
        .iter()
        .any(|(node, _)| matches!(node.node_kind(), crate::node::node_kind::NodeKind::TextInput { .. }));
      let control = EventControl::new();
      'bubble: for (node, _) in &hits {
        let event = LongPressEvent {
          x: lx,
          y: ly,
          target_id: node.node_id(),
          control: control.clone(),
        };
        for handler in &node.events.on_long_press {
          handler.call(&event);
          taken = true;
          if event.immediate_propagation_stopped() {
            break 'bubble;
          }
        }
        if event.propagation_stopped() {
          break;
        }
      }
    }
    // In a text input nothing took it in: the caret goes under the finger, with the loupe.
    if !taken {
      taken = self.touch_text_long_press(lx, ly, on_text_input);
    }
    self.apply_reactive_updates_after_event();
    taken
  }

  /// The touches down whose ids are in `ids`, and where each is, in physical pixels.
  #[cfg_attr(not(feature = "mcp"), allow(dead_code))]
  pub(crate) fn held_touches(&self, ids: std::ops::Range<u64>) -> Vec<(u64, (f32, f32))> {
    let scale = self.scale_factor();
    self
      .touch_pointer
      .touches
      .iter()
      .filter(|touch| ids.contains(&touch.id))
      .map(|touch| (touch.id, (touch.position.0 * scale, touch.position.1 * scale)))
      .collect()
  }

  /// Whether a touch of this id is down: it started and has not ended.
  #[cfg_attr(not(feature = "winit"), allow(dead_code))]
  pub(crate) fn touch_is_down(&self, id: u64) -> bool {
    self.touch_pointer.touches.iter().any(|touch| touch.id == id)
  }

  /// A pen hovers over the point, in physical pixels, without touching the screen: what is under it shows its
  /// hovered style and hears `on_mouse_enter` and `on_mouse_move`, as under a mouse, told a pen
  /// (`PointerKind::Pen`). The app's pointer is left as it is. A shell gives it a pen in range where the platform
  /// reports one: an Apple Pencil's hover on an iPad, a pen over a Windows screen, a browser's `pointerType` pen
  /// moving with no button down. Nothing hovers while a finger or a pen is down.
  pub fn pen_hover(&mut self, x: f32, y: f32) {
    if self.touch_pointer.down {
      return;
    }
    let was = std::mem::replace(&mut self.touch_pointer.mouse_pen, true);
    self.mouse_move(x, y);
    self.touch_pointer.mouse_pen = was;
  }

  /// The pen that hovered went out of range: nothing is hovered by it.
  pub fn pen_hover_ended(&mut self) {
    if !self.touch_pointer.down {
      self.mouse_leave_window();
    }
  }

  /// The finger that is the pointer presses at the point, in physical pixels.
  pub fn touch_press(&mut self, x: f32, y: f32) {
    self.end_tap_highlight();
    self.touch_pointer.down = true;
    self.mouse_move(x, y);
    self.mouse_down(x, y, MouseButton::Left);
    self.touch_pointer.press = self.touch_press_at(x, y);
    self.unhover_for_touch();
  }

  /// The finger that is the pointer moves to the point with its press held.
  pub fn touch_pointer_move(&mut self, x: f32, y: f32) {
    // A finger moving the caret with the loupe selects nothing.
    if self.touch_caret_moving() {
      let scale = self.scale_factor();
      self.move_touch_caret(x / scale, y / scale);
      return;
    }
    self.mouse_move(x, y);
    self.unhover_for_touch();
    // What a drag holds (a slider, a selection, a drag session) is not let go by distance.
    if self.has_active_input_interaction() {
      return;
    }
    let scale = self.scale_factor();
    let Some(press) = &mut self.touch_pointer.press else {
      return;
    };
    let lit = press.holds(x / scale, y / scale, PRESS_REACH);
    if lit == press.lit {
      return;
    }
    press.lit = lit;
    if let Some(root) = &self.root {
      for target in &self.active_path {
        if let Some(node) = find_node_by_id(root, *target) {
          set_node_active(node, lit);
        }
      }
    }
    self.cached_render_list = None;
    self.needs_redraw = true;
  }

  /// The finger that is the pointer lifts at the point: the press ends, with a click when the finger is on
  /// what it pressed or within reach of it, and the pointer leaves.
  pub fn touch_release(&mut self, x: f32, y: f32) {
    self.end_touch_caret();
    // A second finger landed with it, or a handler took its tap: no click.
    if self.touch_clicks_nothing() {
      self.touch_cancel_press(x, y);
      return;
    }
    let scale = self.scale_factor();
    let press = self.touch_pointer.press.take();
    // Beside the element but within reach, the finger still clicks it: the release is the element's.
    let at = match press {
      Some(press)
        if !self.has_active_input_interaction()
          && !press.holds(x / scale, y / scale, 0.0)
          && press.holds(x / scale, y / scale, PRESS_REACH) =>
      {
        press.position
      }
      _ => (x, y),
    };
    self.mouse_up(at.0, at.1, MouseButton::Left);
    self.touch_pointer.down = false;
    self.mouse_leave_window();
    // A press that selected in the focused text input (a double tap selects a word) opens the editing commands.
    if self.focused_input_at(at.0, at.1) {
      self.after_touch_tap();
    }
  }

  /// The press of the finger that is the pointer ends without a click, and the pointer leaves: a scroll
  /// started, a long press was taken, or the system took the touch.
  pub fn touch_cancel_press(&mut self, x: f32, y: f32) {
    self.end_touch_caret();
    self.touch_pointer.press = None;
    self.mouse_press_taken_by_os(x, y, MouseButton::Left);
    self.touch_pointer.down = false;
    self.mouse_leave_window();
  }

  /// A tap at the point, in physical pixels: a press and its release at once, so the click is not held back,
  /// while what was tapped goes on showing as pressed for `highlight`.
  pub fn touch_tap(&mut self, x: f32, y: f32, highlight: Duration) {
    // A handler took the tap, or it was of more fingers than one: nothing is pressed.
    if self.touch_pointer.no_click {
      return;
    }
    self.touch_press(x, y);
    let pressed = self.active_path.clone();
    self.touch_pointer.press = None;
    self.mouse_up(x, y, MouseButton::Left);
    self.touch_pointer.down = false;
    self.mouse_leave_window();
    // The click may have rebuilt what was tapped: only what is still there shows.
    let mut shown = Vec::new();
    if let Some(root) = &self.root {
      for target in pressed {
        if let Some(node) = find_node_by_id(root, target) {
          set_node_active(node, true);
          shown.push(target);
        }
      }
    }
    if !shown.is_empty() {
      self.touch_pointer.tap_highlight = Some((shown, Instant::now() + highlight));
      self.cached_render_list = None;
      self.needs_redraw = true;
    }
    // A tap that selected in the focused text input (a double tap selects a word) opens the editing commands.
    if self.focused_input_at(x, y) {
      self.after_touch_tap();
    }
  }

  /// Whether the focused text input is under a point, in physical pixels.
  fn focused_input_at(&self, x: f32, y: f32) -> bool {
    let scale = self.scale_factor();
    let (Some(root), Some(layout), Some(focused)) = (&self.root, &self.last_layout, self.focused_node) else {
      return false;
    };
    let mut hits = Vec::new();
    hit_test_tree(
      root,
      layout,
      0.0,
      0.0,
      x / scale,
      y / scale,
      &self.layout_engine.radii(),
      &mut hits,
    );
    hits.iter().any(|(node, _)| {
      node.node_id() == focused && matches!(node.node_kind(), crate::node::node_kind::NodeKind::TextInput { .. })
    })
  }

  /// Ends a tap's pressed look once its time is up. `true` while one is still showing: the shell runs another
  /// turn for it.
  pub(super) fn touch_tap_highlight_tick(&mut self, now: Instant) -> bool {
    match &self.touch_pointer.tap_highlight {
      Some((_, until)) if now >= *until => {
        self.end_tap_highlight();
        false
      }
      Some(_) => true,
      None => false,
    }
  }

  pub(super) fn touch_tap_highlight_showing(&self) -> bool {
    self.touch_pointer.tap_highlight.is_some()
  }

  fn end_tap_highlight(&mut self) {
    let Some((shown, _)) = self.touch_pointer.tap_highlight.take() else {
      return;
    };
    if let Some(root) = &self.root {
      for target in shown {
        if let Some(node) = find_node_by_id(root, target) {
          set_node_active(node, false);
        }
      }
    }
    self.cached_render_list = None;
    self.needs_redraw = true;
  }

  /// What a press at the point holds: the innermost element there that takes clicks, or the innermost one.
  fn touch_press_at(&self, x: f32, y: f32) -> Option<Press> {
    let scale = self.scale_factor();
    let (root, layout) = (self.root.as_ref()?, self.last_layout.as_ref()?);
    let mut hits = Vec::new();
    hit_test_tree(
      root,
      layout,
      0.0,
      0.0,
      x / scale,
      y / scale,
      &self.layout_engine.radii(),
      &mut hits,
    );
    if super::take_disabled_hit(&mut hits).is_some() {
      return None;
    }
    let (_, rect) = hits
      .iter()
      .find(|(node, _)| !node.events.on_click.is_empty())
      .or_else(|| hits.first())?;
    Some(Press {
      position: (x, y),
      rect: (rect.x, rect.y, rect.width, rect.height),
      lit: true,
    })
  }

  /// Takes the hovered style off what the pointer is over: a finger hovers nothing.
  fn unhover_for_touch(&mut self) {
    let Some(root) = &self.root else {
      return;
    };
    let mut changed = false;
    for target in &self.hover_path {
      if let Some(node) = find_node_by_id(root, *target) {
        changed |= node.style_state.is_hovered();
        node.set_style_hovered(false);
        if let Some((state, _)) = node.scroll_container() {
          state.set_pointer_over(false);
        }
      }
    }
    if changed {
      self.cached_render_list = None;
      self.needs_redraw = true;
    }
  }
}

fn send_pinch(root: &Node, chain: &[NodeId], phase: GesturePhase, scale: f32, delta: f32, middle: (f32, f32)) {
  let control = EventControl::new();
  for target in chain {
    let Some(node) = find_node_by_id(root, *target) else {
      continue;
    };
    let event = PinchEvent {
      phase,
      scale,
      delta,
      x: middle.0,
      y: middle.1,
      target_id: *target,
      control: control.clone(),
    };
    for handler in &node.events.on_pinch {
      handler.call(&event);
      if event.immediate_propagation_stopped() {
        return;
      }
    }
    if event.propagation_stopped() {
      return;
    }
  }
}

fn send_rotate(root: &Node, chain: &[NodeId], phase: GesturePhase, rotation: f32, delta: f32, middle: (f32, f32)) {
  let control = EventControl::new();
  for target in chain {
    let Some(node) = find_node_by_id(root, *target) else {
      continue;
    };
    let event = RotateEvent {
      phase,
      rotation,
      delta,
      x: middle.0,
      y: middle.1,
      target_id: *target,
      control: control.clone(),
    };
    for handler in &node.events.on_rotate {
      handler.call(&event);
      if event.immediate_propagation_stopped() {
        return;
      }
    }
    if event.propagation_stopped() {
      return;
    }
  }
}
