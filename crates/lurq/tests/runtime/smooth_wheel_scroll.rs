//! A notch of a mouse's wheel says where the scroll is going, and the offset is animated there over the frames
//! that follow ([`Tree::wheel`]). Notches that come meanwhile add to where it is going; anything else that
//! scrolls the container stops it. A delta in pixels ([`Tree::scroll`]: a touchpad, a finger) is applied at
//! once, and so is a notch with motion reduced or the theme's `smooth_scroll` off.
//!
//! The tests set the tree's clock and run the passes the tree asks for, as a shell does.

use std::{
  sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
  },
  time::{Duration, Instant},
};

use lurq::{
  app::{
    App, Tree,
    events::{MouseButton, ScrollEvent, ScrollPhase},
    synthetic_input::{self, SyntheticInput},
  },
  components::{Column, Rect, ScrollVertical},
  layout::{
    layout_kind::{ScrollAxis, ScrollState},
    quad::QuadContent,
  },
  node::{Element, color::Color},
};

use crate::support::{TestSurface, render_pass_with_app};

/// Over the page, and over the list at its top.
const POINTER: (f32, f32) = (150.0, 25.0);
/// What the shell scrolls for one notch of the wheel.
const NOTCH: f32 = 40.0;
/// Longer than any notch takes.
const LATER: u64 = 1000;
const MARK: Color = Color::new(255, 0, 255, 255);

struct Stage {
  app: App,
  tree: Tree,
  start: Instant,
  scale: f32,
}

impl Stage {
  fn new(root: impl Into<Element>) -> Self {
    Self::with(App::new(), 1.0, root)
  }

  /// A tree drawn at `scale` device pixels to a logical one, with `app`'s theme.
  fn with(app: App, scale: f32, root: impl Into<Element>) -> Self {
    let mut stage = Self {
      app,
      tree: Tree::new(),
      start: Instant::now(),
      scale,
    };
    stage.tree.set_scale_factor(scale);
    stage.tree.set_root(root);
    stage.clock(0);
    render_pass_with_app(&mut stage.tree, &mut stage.app);
    // The pointer is where the wheel will turn, and what it hovers there is drawn.
    stage.tree.mouse_move(POINTER.0 * scale, POINTER.1 * scale);
    stage.settle(0);
    stage.clock(0);
    stage
  }

  fn clock(&mut self, millis: u64) {
    self
      .tree
      .set_animation_clock(Some(self.start + Duration::from_millis(millis)));
  }

  /// The frame `millis` after the start, if the tree asks for one. Tells whether it did.
  fn frame(&mut self, millis: u64) -> bool {
    self.clock(millis);
    self.tree.pass(&mut self.app, &TestSurface).required
  }

  /// Every frame the tree asks for from `from` on, 8 ms apart as on a 120 Hz display: when the last one was.
  fn settle(&mut self, from: u64) -> u64 {
    let mut at = from;
    while self.frame(at) {
      at += 8;
      assert!(at < from + LATER, "the tree still asks for frames a second on");
    }
    at
  }

  /// Notches of the wheel at the pointer, at the tree's time: down the page when positive.
  fn notch(&mut self, notches: f32) {
    self.tree.wheel(
      POINTER.0 * self.scale,
      POINTER.1 * self.scale,
      0.0,
      -notches * NOTCH,
      ScrollPhase::Scroll,
    );
  }

  /// Whether the tree asks for nothing more: no redraw, no animation, no next frame.
  fn idle(&self) -> bool {
    !self.tree.needs_redraw() && !self.tree.has_active_timeline() && self.tree.next_timeline_change().is_none()
  }
}

/// A 300 by 200 page that scrolls down over 2000 of content, with a marked 20 px square 100 from its top.
fn page(state: &ScrollState) -> ScrollVertical {
  ScrollVertical::new(
    Column::new()
      .spacing(0.0)
      .child(Rect::new(300.0, 100.0).background("#111827"))
      .child(Rect::new(20.0, 20.0).background(MARK))
      .child(Rect::new(300.0, 1880.0).background("#111827")),
  )
  .with_scroll_state(state.clone())
  .size(300.0, 200.0)
}

/// How far the list scrolls: its content is this much taller than its viewport.
const OVERFLOW: f32 = 30.0;

/// A 300 by 200 page with a 300 by 150 list at its top whose content is `OVERFLOW` taller, and more below.
fn list_in_a_page(page: &ScrollState, list: &ScrollState) -> ScrollVertical {
  ScrollVertical::new(
    Column::new()
      .spacing(0.0)
      .child(
        ScrollVertical::new(Rect::new(300.0, 150.0 + OVERFLOW).background("#1f2937"))
          .with_scroll_state(list.clone())
          .size(300.0, 150.0),
      )
      .child(Rect::new(300.0, 1000.0).background("#111827")),
  )
  .with_scroll_state(page.clone())
  .size(300.0, 200.0)
}

#[test]
fn a_notch_is_spread_over_frames_and_ends_a_notch_further() {
  let state = ScrollState::new();
  let mut stage = Stage::new(page(&state));

  stage.notch(1.0);
  assert_eq!(
    state.scroll_target(),
    (0.0, NOTCH),
    "the notch says where the scroll goes"
  );
  assert!(stage.frame(8), "the notch asks for a frame");
  let first = state.scroll_y();
  assert!(
    first > 0.0 && first < NOTCH,
    "the first frame moves, and not all the way: {first}"
  );
  // Eased out: every frame moves on, by less than the one before it.
  let (mut offset, mut step) = (first, first);
  for at in [16, 24, 32, 40] {
    assert!(stage.frame(at));
    let moved = state.scroll_y() - offset;
    assert!(moved > 0.0 && moved < step, "at {at} ms the offset moved by {moved}");
    (offset, step) = (state.scroll_y(), moved);
  }

  let ended = stage.settle(48);
  assert_eq!(state.scroll_y(), NOTCH, "exactly a notch further");
  assert_eq!(state.scroll_target(), (0.0, NOTCH));
  assert!(
    (100..=220).contains(&ended),
    "a notch takes about as long as in a browser: {ended} ms"
  );
}

#[test]
fn notches_that_come_while_it_moves_add_to_where_it_goes() {
  let state = ScrollState::new();
  let mut stage = Stage::new(page(&state));

  stage.notch(1.0);
  stage.frame(8);
  stage.frame(16);
  let (before, step_before) = (state.scroll_y(), state.scroll_y());
  stage.clock(24);
  stage.notch(1.0);
  assert_eq!(state.scroll_target(), (0.0, 2.0 * NOTCH));
  assert!(
    state.scroll_y() > before,
    "the notch comes to an offset that kept moving until it came"
  );
  let at_notch = state.scroll_y();
  stage.frame(32);
  assert!(
    state.scroll_y() - at_notch > step_before / 2.0,
    "the frame after the second notch moves on at once, with no start from rest: {} then {}",
    at_notch,
    state.scroll_y()
  );

  stage.settle(40);
  assert_eq!(state.scroll_y(), 2.0 * NOTCH, "two notches end two notches further");
}

#[test]
fn a_fast_spin_covers_what_it_covered_before() {
  let state = ScrollState::new();
  let mut stage = Stage::new(page(&state));

  // Sixty notches a second, with a frame every 8 ms: how far behind where it is going the offset is after
  // each notch.
  let mut frame = 0;
  let mut behind = Vec::new();
  for notch in 0..40 {
    let at = notch * 16;
    while frame < at {
      stage.frame(frame);
      frame += 8;
    }
    stage.clock(at);
    stage.notch(1.0);
    behind.push(state.scroll_target().1 - state.scroll_y());
  }
  assert_eq!(state.scroll_target().1, 40.0 * NOTCH, "every notch counts");
  assert!(
    behind[39] < 4.0 * NOTCH && (behind[39] - behind[19]).abs() < 1.0,
    "the offset keeps up with the wheel, and falls no further behind as the spin goes on: {behind:?}"
  );
  let ended = stage.settle(frame);
  assert_eq!(state.scroll_y(), 40.0 * NOTCH);
  assert!(
    ended <= 39 * 16 + 220,
    "and it stops as soon after the last notch as after one: {ended}"
  );
}

#[test]
fn a_notch_the_other_way_turns_it_at_once() {
  let state = ScrollState::new();
  let mut stage = Stage::new(page(&state));
  stage.notch(3.0);
  stage.settle(0);
  let from = state.scroll_y();
  assert_eq!(from, 3.0 * NOTCH);

  stage.clock(LATER);
  stage.notch(1.0);
  stage.frame(LATER + 8);
  stage.frame(LATER + 16);
  let down = state.scroll_y();
  assert!(down > from && down < from + NOTCH);

  stage.clock(LATER + 24);
  stage.notch(-2.0);
  assert_eq!(state.scroll_target(), (0.0, from - NOTCH));
  let turned = state.scroll_y();
  stage.frame(LATER + 32);
  assert!(
    state.scroll_y() < turned,
    "the next frame already goes back: {turned} then {}",
    state.scroll_y()
  );
  stage.settle(LATER + 40);
  assert_eq!(state.scroll_y(), from - NOTCH, "one notch down and two up");
}

#[test]
fn a_delta_in_pixels_is_applied_at_once() {
  let state = ScrollState::new();
  let mut stage = Stage::new(page(&state));

  stage
    .tree
    .scroll(POINTER.0, POINTER.1, 0.0, -NOTCH, ScrollPhase::Scroll);
  assert_eq!(
    state.scroll_y(),
    NOTCH,
    "a touchpad's delta moves the content with the fingers"
  );
  assert!(!stage.tree.has_active_timeline());
  assert!(stage.frame(8), "the frame that draws it");
  assert!(!stage.frame(16), "and no other");
  assert_eq!(state.scroll_y(), NOTCH);
}

#[test]
fn a_notch_is_applied_at_once_with_motion_reduced() {
  let state = ScrollState::new();
  let app = App::new();
  app.theme().set_reduced_motion(Some(true));
  let mut stage = Stage::with(app, 1.0, page(&state));

  stage.notch(1.0);
  assert_eq!(state.scroll_y(), NOTCH);
  assert!(!stage.tree.has_active_timeline());
  assert!(stage.frame(8));
  assert!(stage.idle());
}

#[test]
fn motion_reduced_while_a_notch_is_on_its_way_ends_it_at_once() {
  let state = ScrollState::new();
  let mut stage = Stage::new(page(&state));

  stage.notch(1.0);
  stage.frame(8);
  assert!(state.scroll_y() < NOTCH);
  stage.app.theme().set_reduced_motion(Some(true));
  stage.frame(16);
  assert_eq!(state.scroll_y(), NOTCH, "the scroll is where it was going");
  stage.settle(24);
  assert!(stage.idle());
}

#[test]
fn a_notch_is_applied_at_once_with_smooth_scroll_off() {
  let state = ScrollState::new();
  let app = App::new();
  assert!(app.theme().smooth_scroll(), "on unless the app turns it off");
  app.theme().set_smooth_scroll(false);
  let mut stage = Stage::with(app, 1.0, page(&state));

  stage.notch(1.0);
  assert_eq!(state.scroll_y(), NOTCH);
  assert!(!stage.tree.has_active_timeline());
  assert!(stage.frame(8));
  assert!(stage.idle());
}

#[test]
fn a_notch_at_the_end_of_a_list_goes_to_the_page_around_it() {
  let (page, list) = (ScrollState::new(), ScrollState::new());
  let mut stage = Stage::new(list_in_a_page(&page, &list));

  // The list has less than a notch left: it takes the notch, and what it cannot take is dropped.
  stage.notch(1.0);
  assert_eq!(
    (list.scroll_target().1, page.scroll_target().1),
    (OVERFLOW, 0.0),
    "the list goes to its end and the page stays"
  );
  stage.frame(8);
  assert!(
    list.scroll_y() > 0.0 && list.scroll_y() < OVERFLOW,
    "{}",
    list.scroll_y()
  );

  // The list is going to its end, though it is not drawn there yet: the next notch is the page's, whole.
  stage.clock(16);
  stage.notch(1.0);
  assert_eq!((list.scroll_target().1, page.scroll_target().1), (OVERFLOW, NOTCH));

  stage.settle(16);
  assert_eq!((list.scroll_y(), page.scroll_y()), (OVERFLOW, NOTCH));

  // And back: the list has room that way again and takes the notch; the page keeps its place.
  stage.clock(LATER);
  stage.notch(-1.0);
  assert_eq!((list.scroll_target().1, page.scroll_target().1), (0.0, NOTCH));
  stage.clock(LATER + 8);
  stage.notch(-1.0);
  assert_eq!(
    (list.scroll_target().1, page.scroll_target().1),
    (0.0, 0.0),
    "the list is going to its start: the page takes the next one"
  );
  stage.settle(LATER + 8);
  assert_eq!((list.scroll_y(), page.scroll_y()), (0.0, 0.0));
}

#[test]
fn a_notch_back_from_the_end_leaves_it() {
  let state = ScrollState::new();
  let mut stage = Stage::new(page(&state));
  state.scroll_to_bottom_pending();
  stage.settle(0);
  let end = state.scroll_y();
  assert_eq!(end, 1800.0);

  // The first steps are within the distance an offset at the end is held there when the content grows.
  stage.clock(LATER);
  stage.notch(-1.0);
  for at in 1..=2 {
    stage.frame(LATER + at);
  }
  assert!(
    state.scroll_y() < end && state.scroll_y() > end - 2.0,
    "{}",
    state.scroll_y()
  );
  stage.settle(LATER + 8);
  assert_eq!(
    state.scroll_y(),
    end - NOTCH,
    "a whole notch back, none of it held at the end"
  );
}

#[test]
fn a_scroll_to_an_offset_wins_over_a_notch_on_its_way() {
  for pending in [false, true] {
    let state = ScrollState::new();
    let mut stage = Stage::new(page(&state));

    stage.notch(1.0);
    stage.frame(8);
    if pending {
      state.set_scroll_pending(0.0, 500.0);
    } else {
      state.set_scroll(0.0, 500.0);
    }
    assert_eq!(
      state.scroll_target(),
      (0.0, if pending { state.scroll_y() } else { 500.0 })
    );
    stage.settle(16);
    assert_eq!(state.scroll_y(), 500.0, "pending: {pending}");
    assert!(stage.idle());
  }
}

#[test]
fn a_delta_in_pixels_wins_over_a_notch_on_its_way() {
  let state = ScrollState::new();
  let mut stage = Stage::new(page(&state));

  stage.notch(1.0);
  stage.frame(8);
  let at = state.scroll_y();
  stage.clock(16);
  stage.tree.scroll(POINTER.0, POINTER.1, 0.0, -10.0, ScrollPhase::Scroll);
  assert_eq!(
    state.scroll_y(),
    at + 10.0,
    "the fingers move the content from where it is"
  );
  stage.settle(16);
  assert_eq!(state.scroll_y(), at + 10.0, "and the rest of the notch is dropped");
}

#[test]
fn a_drag_of_the_thumb_wins_over_a_notch_on_its_way() {
  let state = ScrollState::new();
  let mut stage = Stage::new(page(&state));
  let (x, y, width, height) = state
    .thumb_rect_for_axis(ScrollAxis::Vertical, &state.style())
    .expect("the page overflows");
  let thumb = (x + width / 2.0, y + height / 2.0);

  stage.notch(1.0);
  stage.frame(8);
  let at = state.scroll_y();
  assert!(at > 0.0 && at < NOTCH);

  stage.tree.mouse_move(thumb.0, thumb.1);
  stage.tree.mouse_down(thumb.0, thumb.1, MouseButton::Left);
  assert_eq!(state.scroll_target().1, at, "the press holds the content where it is");
  stage.frame(16);
  assert_eq!(state.scroll_y(), at);

  stage.tree.mouse_move(thumb.0, thumb.1 + 50.0);
  let dragged = state.scroll_y();
  assert!(dragged > 10.0 * NOTCH, "the thumb takes the content along: {dragged}");
  stage.frame(24);
  stage.tree.mouse_up(thumb.0, thumb.1 + 50.0, MouseButton::Left);
  stage.settle(32);
  assert_eq!(state.scroll_y(), dragged);
  assert!(stage.idle());
}

#[test]
fn a_handler_sees_every_step_of_a_notch() {
  let state = ScrollState::new();
  let seen = Arc::new(Mutex::new(Vec::new()));
  let reached = Arc::new(AtomicUsize::new(0));
  let (handler_state, handler_seen, handler_reached) = (state.clone(), seen.clone(), reached.clone());
  // The page has two notches to go to its end.
  let root = ScrollVertical::new(Rect::new(300.0, 200.0 + 2.0 * NOTCH).background("#111827"))
    .with_scroll_state(state.clone())
    .on_scroll(move |event: ScrollEvent| {
      handler_seen
        .lock()
        .unwrap()
        .push((event.delta_y, handler_state.scroll_y()));
    })
    .on_scroll_reach_bottom(move |_| {
      handler_reached.fetch_add(1, Ordering::SeqCst);
    })
    .size(300.0, 200.0);
  let mut stage = Stage::new(root);

  stage.notch(2.0);
  assert_eq!(
    *seen.lock().unwrap(),
    [(-2.0 * NOTCH, 0.0)],
    "the wheel's event comes first, with its delta, before anything moves"
  );
  let mut drawn = Vec::new();
  let mut at = 8;
  while stage.frame(at) {
    drawn.push(state.scroll_y());
    at += 8;
  }
  drawn.dedup();
  let steps: Vec<f32> = seen.lock().unwrap()[1..].iter().map(|(_, offset)| *offset).collect();
  assert!(steps.len() > 4, "{steps:?}");
  assert_eq!(
    steps, drawn,
    "one event for every offset drawn, and the offset is the one drawn"
  );
  assert!(
    seen.lock().unwrap()[1..].iter().all(|(delta, _)| *delta == 0.0),
    "a step has no delta of the wheel's"
  );
  assert_eq!(reached.load(Ordering::SeqCst), 1, "the end is reached once, when it is");
}

#[test]
fn a_notch_on_the_way_tells_the_handler_how_far_the_content_had_come() {
  let state = ScrollState::new();
  let seen = Arc::new(Mutex::new(Vec::new()));
  let (handler_state, handler_seen) = (state.clone(), seen.clone());
  let root = page(&state).on_scroll(move |event: ScrollEvent| {
    handler_seen
      .lock()
      .unwrap()
      .push((event.delta_y, handler_state.scroll_y()));
  });
  let mut stage = Stage::new(root);

  stage.notch(1.0);
  stage.frame(8);
  let drawn = state.scroll_y();
  // The next notch comes 4 ms after that frame: the content has moved on since, and no frame said so.
  stage.clock(12);
  stage.notch(1.0);
  let at_notch = state.scroll_y();
  assert!(at_notch > drawn);
  assert_eq!(
    *seen.lock().unwrap(),
    [(-NOTCH, 0.0), (0.0, drawn), (0.0, at_notch), (-NOTCH, at_notch)],
    "the step to where the content is, then the wheel's event from there"
  );
}

#[test]
fn a_handler_that_prevents_the_scroll_prevents_the_animation() {
  let state = ScrollState::new();
  let root = page(&state).on_scroll(|event: ScrollEvent| event.prevent_default());
  let mut stage = Stage::new(root);

  stage.notch(1.0);
  assert_eq!(state.scroll_target(), (0.0, 0.0));
  stage.settle(8);
  assert_eq!(state.scroll_y(), 0.0);
}

#[test]
fn no_frame_is_asked_for_once_a_notch_has_ended() {
  let state = ScrollState::new();
  let mut stage = Stage::new(page(&state));
  assert!(stage.idle());
  assert!(!stage.frame(4), "an idle tree draws nothing");

  stage.notch(1.0);
  assert!(stage.tree.needs_redraw(), "the notch asks for its first frame");
  assert!(stage.frame(8));
  assert!(
    stage.tree.has_active_timeline(),
    "the frames after it are an animation's"
  );
  assert!(
    stage.tree.next_timeline_change().is_some(),
    "which a shell paces as any other"
  );
  assert!(!stage.tree.needs_redraw(), "not a redraw asked for at once");

  let ended = stage.settle(16);
  assert_eq!(state.scroll_y(), NOTCH);
  assert!(stage.idle());
  for later in [ended + 8, ended + 500, ended + 5000] {
    assert!(!stage.frame(later), "no frame {later} ms on");
  }
}

/// Where the marked square is drawn, in device pixels.
fn mark_y(stage: &Stage) -> f32 {
  let tree = &stage.tree;
  tree
    .resolve_quads(tree.last_layout().expect("a layout"))
    .iter()
    .find(|quad| matches!(&quad.content, QuadContent::Rect { color, .. } if *color == MARK))
    .map(|quad| quad.y * stage.scale)
    .expect("the mark")
}

#[test]
fn every_step_of_a_notch_is_drawn_on_a_whole_device_pixel() {
  let state = ScrollState::new();
  let mut stage = Stage::with(App::new(), 1.5, page(&state));
  let rest = mark_y(&stage);

  stage.notch(1.0);
  let (mut at, mut between) = (8, 0);
  while stage.frame(at) {
    let y = mark_y(&stage);
    assert!((y - y.round()).abs() < 1e-3, "at {at} ms the content is drawn at {y}");
    let offset = state.scroll_y() * 1.5;
    between += usize::from((offset - offset.round()).abs() > 0.01);
    at += 8;
  }
  assert!(between > 3, "the offset itself is between device pixels on its way");
  assert_eq!(state.scroll_y(), NOTCH);
  assert_eq!(rest - mark_y(&stage), 60.0, "a notch is 60 device pixels at 150%");
}

#[test]
fn content_that_grows_above_takes_a_notch_on_its_way_along() {
  let state = ScrollState::new();
  let mut stage = Stage::new(page(&state));
  stage.notch(2.0);
  stage.settle(0);
  assert_eq!(state.scroll_y(), 2.0 * NOTCH);

  stage.clock(LATER);
  stage.notch(1.0);
  stage.frame(LATER + 8);
  let at = state.scroll_y();
  assert!(at > 2.0 * NOTCH && at < 3.0 * NOTCH);

  // A hundred more comes in above what is shown, and the scroll keeps what it shows in place.
  state.preserve_prepend_anchor_pending();
  stage.tree.set_root(
    ScrollVertical::new(
      Column::new()
        .spacing(0.0)
        .child(Rect::new(300.0, 100.0).background("#1f2937"))
        .child(Rect::new(300.0, 2000.0).background("#111827")),
    )
    .with_scroll_state(state.clone())
    .size(300.0, 200.0),
  );
  stage.frame(LATER + 16);
  assert!(
    state.scroll_y() > at + 100.0,
    "the notch goes on from where the content now is: {}",
    state.scroll_y()
  );
  stage.settle(LATER + 24);
  assert_eq!(state.scroll_y(), 3.0 * NOTCH + 100.0, "and ends a notch further there");
  assert!(stage.idle());
}

#[test]
fn a_notch_to_the_end_goes_on_to_an_end_that_moves() {
  let state = ScrollState::new();
  let longer = |by: f32| {
    ScrollVertical::new(Rect::new(300.0, 200.0 + NOTCH + by).background("#111827"))
      .with_scroll_state(state.clone())
      .size(300.0, 200.0)
  };
  let mut stage = Stage::new(longer(0.0));

  // The notch takes the content to its end; the content grows by 25 on the way, as a chat's does.
  stage.notch(1.0);
  stage.frame(8);
  stage.frame(16);
  stage.tree.set_root(longer(25.0));
  stage.frame(24);
  assert!(
    state.scroll_y() < NOTCH,
    "it is not thrown to the new end: {}",
    state.scroll_y()
  );
  stage.settle(32);
  assert_eq!(state.scroll_y(), NOTCH + 25.0, "it ends at the end, where it was going");

  // And it stays at the end from then on, as any offset at the end does.
  stage.tree.set_root(longer(60.0));
  stage.settle(LATER);
  assert_eq!(state.scroll_y(), NOTCH + 60.0);
}

#[test]
fn a_notch_whose_container_is_gone_asks_for_nothing_more() {
  let state = ScrollState::new();
  let mut stage = Stage::new(page(&state));

  stage.notch(1.0);
  stage.frame(8);
  stage.tree.set_root(Rect::new(300.0, 200.0).background("#111827"));
  stage.settle(16);
  assert!(stage.idle());
  assert_eq!(state.scroll_target().1, state.scroll_y(), "nothing is on its way");
}

#[test]
fn a_synthetic_wheel_is_a_mouses_wheel() {
  let state = ScrollState::new();
  let mut stage = Stage::new(page(&state));

  synthetic_input::apply(
    &mut stage.tree,
    &SyntheticInput::wheel(POINTER.0, POINTER.1, 0.0, -NOTCH),
  );
  assert_eq!(state.scroll_target(), (0.0, NOTCH));
  stage.frame(8);
  assert!(state.scroll_y() > 0.0 && state.scroll_y() < NOTCH);
  // What a harness waits for before it reads the offset: `lurq_wait` waits for the same.
  assert!(stage.tree.has_active_timeline());
  stage.settle(16);
  assert_eq!(state.scroll_y(), NOTCH);
  assert!(!stage.tree.has_active_timeline());
}

/// A page of 300 by 200 over 2000 of content, with the state it scrolls by.
fn page_with_state() -> (Stage, ScrollState) {
  let state = ScrollState::new();
  let stage = Stage::new(
    ScrollVertical::new(Rect::new(300.0, 2000.0))
      .with_scroll_state(state.clone())
      .size(300.0, 200.0),
  );
  (stage, state)
}

#[test]
fn a_container_sent_to_its_top_goes_there_over_the_frames_that_follow() {
  let (mut stage, state) = page_with_state();
  assert!(
    !stage.tree.scroll_state_to_top(&state),
    "at its top already: nothing to do"
  );
  state.set_scroll(0.0, 600.0);
  stage.settle(0);

  stage.clock(100);
  assert!(stage.tree.scroll_state_to_top(&state));
  assert_eq!(state.scroll_y(), 600.0, "nothing moves before a frame");
  assert_eq!(state.scroll_target().1, 0.0, "it is going to its top");
  assert!(stage.frame(140), "the tree asks for the frames of its way");
  let on_its_way = state.scroll_y();
  assert!(
    0.0 < on_its_way && on_its_way < 600.0,
    "on its way after 40 ms: {on_its_way}"
  );
  stage.settle(148);
  assert_eq!(state.scroll_y(), 0.0);
  assert!(stage.idle(), "and then asks for nothing more");
}

#[test]
fn a_container_sent_to_an_offset_goes_there_over_the_frames_that_follow_held_to_its_content() {
  let (mut stage, state) = page_with_state();
  assert!(
    !stage.tree.scroll_state_to(&state, 0.0, 0.0),
    "there already: nothing to do"
  );
  stage.clock(100);
  assert!(stage.tree.scroll_state_to(&state, 0.0, 500.0));
  assert_eq!(state.scroll_y(), 0.0, "nothing moves before a frame");
  assert_eq!(state.scroll_target().1, 500.0);
  assert!(stage.frame(140), "the tree asks for the frames of its way");
  assert!(0.0 < state.scroll_y() && state.scroll_y() < 500.0);
  let end = stage.settle(148);
  assert_eq!(state.scroll_y(), 500.0);
  assert!(stage.idle());

  // Past the end of the content, it goes to the end.
  stage.clock(end + 10);
  stage.tree.scroll_state_to(&state, 0.0, 5000.0);
  stage.settle(end + 10);
  assert_eq!(state.scroll_y(), 1800.0);
}

#[test]
fn a_notch_of_the_wheel_turns_a_container_on_its_way_to_its_top_round() {
  let (mut stage, state) = page_with_state();
  state.set_scroll(0.0, 600.0);
  stage.settle(0);
  stage.clock(100);
  stage.tree.scroll_state_to_top(&state);
  stage.frame(140);
  let on_its_way = state.scroll_y();

  stage.notch(2.0);
  assert_eq!(
    state.scroll_target().1,
    2.0 * NOTCH,
    "the wheel adds to where it was going, the top"
  );
  stage.settle(148);
  assert_eq!(state.scroll_y(), 2.0 * NOTCH);
  assert!(on_its_way > 0.0);
}

#[test]
fn with_motion_reduced_a_container_is_at_its_top_in_the_next_frame() {
  let app = App::new();
  app.theme().set_reduced_motion(Some(true));
  let state = ScrollState::new();
  let mut stage = Stage::with(
    app,
    1.0,
    ScrollVertical::new(Rect::new(300.0, 2000.0))
      .with_scroll_state(state.clone())
      .size(300.0, 200.0),
  );
  state.set_scroll(0.0, 600.0);
  stage.settle(0);
  stage.clock(100);
  assert!(stage.tree.scroll_state_to_top(&state));
  assert_eq!(state.scroll_y(), 0.0, "at once");
  stage.settle(100);
  assert_eq!(state.scroll_y(), 0.0);
}

#[test]
fn a_state_that_scrolls_nothing_in_the_tree_is_sent_nowhere() {
  let (mut stage, _state) = page_with_state();
  let stranger = ScrollState::new();
  assert!(!stage.tree.scroll_state_to_top(&stranger));
  assert!(stage.idle());
}
