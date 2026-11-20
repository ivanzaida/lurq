//! The scroll of a mouse's wheel. A notch moves the content 40 logical pixels, which drawn in one frame is a
//! jump: the eye loses the line it was on. [`Tree::wheel`] says instead where the scroll container under the
//! pointer is going, and the offset is animated there, a step with every pass, as a browser does. The curve
//! and how long it takes are [`ScrollState`]'s (`WHEEL_SCROLL_DURATION`, `WHEEL_SCROLL_EASING`), which holds
//! where the scroll is going beside the offset it is drawn at, so that whatever else scrolls the container (a
//! scroll to an offset, a drag of the thumb, a finger) stops it there.
//!
//! [`Tree::scroll`] is the other entry: a delta in pixels, of a touchpad, a precise wheel or a finger. It moves
//! the content at once, since it is already as smooth as the fingers are and an animation would fall behind
//! them. The tests of the tree that send a scroll and read the offset in the next line call that one, and so
//! check what it does and no less. The shell calls `wheel` for a mouse's notches alone, and synthetic input
//! (`SyntheticInputKind::Wheel`, the MCP server's `wheel`) is a mouse's wheel: a harness that reads the offset
//! after it waits until the tree has no animation left (`Tree::has_active_timeline`, `lurq_wait`), or turns
//! the animation off (`Theme::set_smooth_scroll`).
//!
//! The steps run on the tree's clock ([`Tree::set_animation_clock`]) and count as a running animation
//! ([`Tree::has_active_timeline`], [`Tree::next_timeline_change`]): a shell paces their frames as it does a
//! transition's, and asks for none once the content is where it was going.

use super::{
  MouseModifiers, ScrollInput, Tree, find_node_by_id, find_scroll_node_id_by_state, mark_scroll_layout_dirty,
};
use crate::{
  app::{
    events::{ScrollPhase, ScrollSource},
    theme::Theme,
  },
  core::NodeId,
  layout::layout_kind::ScrollState,
  time::Instant,
};

/// A scroll container a wheel left on its way.
pub(super) struct WheelScroll {
  target_id: NodeId,
  state: ScrollState,
  /// Where the wheel turned, in logical pixels: where the events of its steps are from.
  pointer: (f32, f32),
  /// When the wheel turned, or the last step moved the content: the next frame is due from then.
  stepped_at: Instant,
  /// What sent the content on its way: the wheel, or the app (`scroll_state_to`).
  source: ScrollSource,
}

impl Tree {
  /// Notches of a mouse's wheel at a point, in physical pixels: `delta_x` and `delta_y` are how far they
  /// scroll, in logical pixels, as [`scroll`](Self::scroll) takes them. The scroll container that would take
  /// the delta goes there over the frames that follow instead of in the next one, and what it cannot take is
  /// the container's around it, as with `scroll`: that is decided on where each is going, not on where it is
  /// drawn, so a notch over a list on its way to its end scrolls the page.
  ///
  /// Notches that come while the content is on its way add to where it is going, and one the other way turns
  /// it round at once. [`ScrollState::scroll_target`] is where a container is going; its offset is what is
  /// drawn. `on_scroll` handlers hear the wheel's event as they do with `scroll`, before anything moves, and
  /// then one event with no delta for every step; `on_scroll_end` handlers hear the content come to rest where
  /// it was going.
  ///
  /// With motion reduced, or the theme's [`smooth_scroll`](Theme::set_smooth_scroll) off, this is `scroll`.
  pub fn wheel(&mut self, x: f32, y: f32, delta_x: f32, delta_y: f32, phase: ScrollPhase) {
    self.wheel_with_meta(x, y, delta_x, delta_y, phase, false, false, false, false);
  }

  /// [`wheel`](Self::wheel) with the keys held as it turned, which its [`ScrollEvent`](crate::app::events::ScrollEvent)
  /// tells: Ctrl and the wheel zoom what zooms by it.
  #[allow(clippy::too_many_arguments)]
  pub fn wheel_with_meta(
    &mut self,
    x: f32,
    y: f32,
    delta_x: f32,
    delta_y: f32,
    phase: ScrollPhase,
    shift: bool,
    ctrl: bool,
    alt: bool,
    meta: bool,
  ) {
    // What is on its way is where it is now before the wheel turns it, and its container has heard so: no
    // part of a frame's way is lost, and none goes untold.
    self.step_wheel_scrolls();
    let input = ScrollInput {
      // With motion reduced it scrolls at once, as `scroll` does; it is a notch still.
      animated: self.wheel_motion,
      notched: true,
      modifiers: MouseModifiers { shift, ctrl, alt, meta },
      source: ScrollSource::Wheel,
    };
    self.scroll_event(x, y, delta_x, delta_y, phase, input);
  }

  /// Sends the scroll container `state` is the state of to its top, as a wheel turned that far would: over
  /// the frames that follow, on the wheel's curve, a step with every pass, and at once where a wheel's
  /// scroll is not animated either (motion is reduced, or the theme's [`smooth_scroll`](Theme::set_smooth_scroll)
  /// is off). A notch of the wheel that comes meanwhile adds to where the content is going, and anything else
  /// that scrolls the container stops it, as with [`wheel`](Self::wheel); `on_scroll` handlers hear an event
  /// with no delta for every step. `false` when no container of the tree has the state, or it is at its top
  /// and going nowhere else. What [`Ctx::scroll_to_top_of`](crate::app::ctx::Ctx::scroll_to_top_of) asks for.
  pub fn scroll_state_to_top(&mut self, state: &ScrollState) -> bool {
    self.rebuild_if_dirty();
    self.step_wheel_scrolls();
    let Some(target_id) = self
      .root
      .as_ref()
      .and_then(|root| find_scroll_node_id_by_state(root, state))
    else {
      return false;
    };
    let (_, going) = state.scroll_target();
    if going <= 0.0 && state.scroll_y() <= 0.0 {
      return false;
    }
    if self.wheel_motion {
      let now = self.now();
      state.wheel_by_with_overflow(0.0, -going, now);
      Self::wheel_turned(
        &mut self.wheel_scrolls,
        target_id,
        state,
        (0.0, 0.0),
        now,
        ScrollSource::Code,
      );
    } else {
      state.set_scroll(state.scroll_x(), 0.0);
      if let Some(root) = &self.root {
        mark_scroll_layout_dirty(root, target_id);
      }
    }
    self.needs_redraw = true;
    true
  }

  /// Sends the scroll container `state` is the state of to the offset `(x, y)`, held to its content, as a
  /// wheel turned that far would: over the frames that follow, on the wheel's curve, and at once where a
  /// wheel's scroll is not animated. What it does otherwise is what
  /// [`scroll_state_to_top`](Self::scroll_state_to_top) does. `false` when no container of the tree has the
  /// state, or it is going there already. What [`Ctx::scroll_state_to`](crate::app::ctx::Ctx::scroll_state_to)
  /// asks for.
  pub fn scroll_state_to(&mut self, state: &ScrollState, x: f32, y: f32) -> bool {
    self.rebuild_if_dirty();
    self.step_wheel_scrolls();
    let Some(target_id) = self
      .root
      .as_ref()
      .and_then(|root| find_scroll_node_id_by_state(root, state))
    else {
      return false;
    };
    let (going_x, going_y) = state.scroll_target();
    let (delta_x, delta_y) = (x.max(0.0) - going_x, y.max(0.0) - going_y);
    if delta_x.abs() < 0.01 && delta_y.abs() < 0.01 {
      return false;
    }
    if self.wheel_motion {
      let now = self.now();
      state.wheel_by_with_overflow(delta_x, delta_y, now);
      Self::wheel_turned(
        &mut self.wheel_scrolls,
        target_id,
        state,
        (0.0, 0.0),
        now,
        ScrollSource::Code,
      );
    } else {
      state.set_scroll(x, y);
      if let Some(root) = &self.root {
        mark_scroll_layout_dirty(root, target_id);
      }
    }
    self.needs_redraw = true;
    true
  }

  /// Whether a wheel's scroll is on its way in some container.
  pub(super) fn wheel_scrolling(&self) -> bool {
    self.wheel_scrolls.iter().any(|scroll| scroll.state.wheel_scrolling())
  }

  /// When the frame for the next step of a wheel's scroll is due: at once, at the display's pace.
  pub(super) fn next_wheel_scroll_step(&self) -> Option<Instant> {
    self
      .wheel_scrolls
      .iter()
      .filter(|scroll| scroll.state.wheel_scrolling())
      .map(|scroll| scroll.stepped_at)
      .min()
  }

  /// Remembers that the wheel sent `state`, the scroll container `target_id`, on its way at `now`.
  pub(super) fn wheel_turned(
    wheel_scrolls: &mut Vec<WheelScroll>,
    target_id: NodeId,
    state: &ScrollState,
    pointer: (f32, f32),
    now: Instant,
    source: ScrollSource,
  ) {
    wheel_scrolls.retain(|scroll| !scroll.state.same(state));
    if state.wheel_scrolling() {
      wheel_scrolls.push(WheelScroll {
        target_id,
        state: state.clone(),
        pointer,
        stepped_at: now,
        source,
      });
    }
  }

  /// A pass's step: takes whether the wheel is animated from `theme`, and moves what a wheel left on its way
  /// ([`Self::step_wheel_scrolls`]).
  pub(super) fn tick_wheel_scrolls(&mut self, theme: &Theme) {
    self.wheel_motion = theme.smooth_scroll() && !theme.reduced_motion();
    self.step_wheel_scrolls();
  }

  /// Moves every scroll a wheel left on its way to where it is now: all the way, if the wheel is no longer
  /// animated. A container whose content moved hears it as it hears an autoscroll's step, by a scroll event
  /// with no delta and by its scroll-reach handlers. One that has gone from the tree is forgotten.
  fn step_wheel_scrolls(&mut self) {
    if self.wheel_scrolls.is_empty() {
      return;
    }
    let now = self.now();
    let mut any_moved = false;
    for mut scroll in std::mem::take(&mut self.wheel_scrolls) {
      let state = scroll.state.clone();
      let target_id = self.root.as_ref().and_then(|root| {
        if find_node_by_id(root, scroll.target_id).is_some() {
          Some(scroll.target_id)
        } else {
          // A rebuild replaced the node: the one that now holds its scroll state.
          find_scroll_node_id_by_state(root, &state)
        }
      });
      let Some(target_id) = target_id else {
        state.stop_wheel();
        continue;
      };
      scroll.target_id = target_id;
      let previous_scroll_y = state.scroll_y();
      let previous_max_scroll_y = (state.content_height() - state.viewport_height()).max(0.0);
      let moved = if self.wheel_motion {
        state.step_wheel(now)
      } else {
        state.end_wheel()
      };
      if moved {
        any_moved = true;
        scroll.stepped_at = now;
        if let Some(root) = &self.root {
          mark_scroll_layout_dirty(root, target_id);
        }
        let (x, y) = scroll.pointer;
        self.dispatch_scroll_handlers_for_node(target_id, x, y, 0.0, 0.0, ScrollPhase::Scroll, scroll.source);
        self.dispatch_scroll_reach_handlers_for_node(
          target_id,
          x,
          y,
          0.0,
          0.0,
          ScrollPhase::Scroll,
          previous_scroll_y,
          previous_max_scroll_y,
          scroll.source,
        );
      }
      if state.wheel_scrolling() {
        self.wheel_scrolls.push(scroll);
      } else if moved && !state.moving() {
        // It is where it was going, and nothing else has it: the scroll is over.
        self.scroll_rests(&state, &[target_id], scroll.pointer, scroll.source);
      }
    }
    if any_moved {
      self.apply_reactive_updates_after_event();
    }
  }
}
