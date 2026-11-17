//! The mouse as fingers, for a phone emulated on a desktop ([`App::emulate`](crate::app::App::emulate)): what the
//! shell gives the tree for the mouse's moves, its left button and its modifiers, through the tree's model of a
//! finger ([`Tree::finger`]), so that a click is a tap and a drag a pan, as a touch screen's finger would be.
//!
//! - With no modifier, the mouse is one finger.
//! - **Alt**: two fingers, the second mirrored about where the drag began, so a drag apart or together pinches and one
//!   around that point rotates.
//! - **Alt+Shift**: two fingers side by side, which move together.
//! - **Alt+Shift+Ctrl**: three fingers side by side. With the keys held and no button, the wheel pinches them about the
//!   mouse: away from the person spreads them (a paste), toward closes them (a copy); letting go of the keys lifts
//!   them.
//!
//! While a modifier is held, or fingers are down, the fingers are drawn where they are or would land
//! ([`Tree::set_touch_marks`]). Positions are physical pixels.

#![cfg_attr(not(feature = "winit"), allow(dead_code))]

use super::Tree;
use crate::app::events::TouchPhase;

/// The ids of the mouse's fingers, far from any a screen gives.
const FIRST_FINGER: u64 = 0x6000_0000;
/// How far apart fingers that land together are, in logical pixels.
const FINGER_GAP: f32 = 40.0;
/// How far from the mouse three fingers the wheel pinches land, in logical pixels, and how far the wheel turns,
/// in its logical pixels, to spread them by e (about 2.7).
const PINCH_RADIUS: f32 = 60.0;
const PINCH_WHEEL: f32 = 200.0;

/// How many fingers the mouse is, and how they are placed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Fingers {
  One,
  /// Two, the second mirrored about where the drag began.
  Mirrored,
  /// Two side by side.
  Two,
  /// Three side by side.
  Three,
}

impl Fingers {
  /// The fingers the held modifiers make.
  pub(crate) fn of(alt: bool, shift: bool, ctrl: bool) -> Self {
    match (alt, shift, ctrl) {
      (true, true, true) => Self::Three,
      (true, true, false) => Self::Two,
      (true, ..) => Self::Mirrored,
      _ => Self::One,
    }
  }
}

#[derive(Debug)]
pub(crate) struct MouseFingers {
  /// Where the mouse is.
  cursor: (f32, f32),
  /// The fingers the modifiers held make now.
  held: Fingers,
  /// The fingers down, and where the drag began.
  down: Option<(Fingers, (f32, f32))>,
  /// Three fingers the wheel pinches: where they stand about, and how far from it, in physical pixels.
  pinch: Option<((f32, f32), f32)>,
}

impl Default for MouseFingers {
  fn default() -> Self {
    Self {
      cursor: (0.0, 0.0),
      held: Fingers::One,
      down: None,
      pinch: None,
    }
  }
}

impl MouseFingers {
  /// Where the fingers are with the mouse at `cursor`, a drag having begun at `start`.
  fn places(fingers: Fingers, cursor: (f32, f32), start: (f32, f32), gap: f32) -> Vec<(f32, f32)> {
    match fingers {
      Fingers::One => vec![cursor],
      Fingers::Mirrored => {
        let half = gap / 2.0;
        vec![
          (cursor.0 + half, cursor.1),
          (2.0 * start.0 - cursor.0 - half, 2.0 * start.1 - cursor.1),
        ]
      }
      Fingers::Two => vec![(cursor.0 - gap / 2.0, cursor.1), (cursor.0 + gap / 2.0, cursor.1)],
      Fingers::Three => vec![(cursor.0 - gap, cursor.1), cursor, (cursor.0 + gap, cursor.1)],
    }
  }

  fn gap(tree: &Tree) -> f32 {
    FINGER_GAP * tree.scale_factor()
  }

  /// Draws the fingers where they are or would land: while fingers other than one are held or down.
  /// Three fingers on a circle about `center`, `radius` from it.
  fn around(center: (f32, f32), radius: f32) -> Vec<(f32, f32)> {
    (0..3)
      .map(|index| {
        let turn = -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * index as f32 / 3.0;
        (center.0 + radius * turn.cos(), center.1 + radius * turn.sin())
      })
      .collect()
  }

  fn mark(&self, tree: &mut Tree) {
    if let Some((center, radius)) = self.pinch {
      tree.set_touch_marks(Self::around(center, radius));
      return;
    }
    let gap = Self::gap(tree);
    let marks = match self.down {
      Some((fingers, start)) => Self::places(fingers, self.cursor, start, gap),
      None if self.held != Fingers::One => Self::places(self.held, self.cursor, self.cursor, gap),
      None => Vec::new(),
    };
    tree.set_touch_marks(marks);
  }

  /// The modifiers held changed.
  pub(crate) fn modifiers(&mut self, tree: &mut Tree, alt: bool, shift: bool, ctrl: bool) {
    self.held = Fingers::of(alt, shift, ctrl);
    if self.held != Fingers::Three {
      self.end_pinch(tree, TouchPhase::End);
    }
    self.mark(tree);
  }

  /// The wheel turned by `delta_y`, its logical pixels, away from the person when positive. With Alt+Shift+Ctrl
  /// held and no button down it pinches three fingers about the mouse, landing them at its first turn: `true`
  /// then, and the wheel scrolls nothing.
  pub(crate) fn wheel(&mut self, tree: &mut Tree, delta_y: f32) -> bool {
    if self.held != Fingers::Three || self.down.is_some() {
      return false;
    }
    let (center, radius) = match self.pinch {
      Some(pinch) => pinch,
      None => {
        let pinch = (self.cursor, PINCH_RADIUS * tree.scale_factor());
        for (index, (x, y)) in Self::around(pinch.0, pinch.1).into_iter().enumerate() {
          tree.finger(FIRST_FINGER + index as u64, TouchPhase::Start, x, y, None);
        }
        pinch
      }
    };
    let radius = (radius * (delta_y / PINCH_WHEEL).exp()).max(2.0);
    self.pinch = Some((center, radius));
    for (index, (x, y)) in Self::around(center, radius).into_iter().enumerate() {
      tree.finger(FIRST_FINGER + index as u64, TouchPhase::Move, x, y, None);
    }
    self.mark(tree);
    true
  }

  /// Lifts the three fingers the wheel pinched, if it did.
  fn end_pinch(&mut self, tree: &mut Tree, phase: TouchPhase) {
    let Some((center, radius)) = self.pinch.take() else {
      return;
    };
    for (index, (x, y)) in Self::around(center, radius).into_iter().enumerate() {
      tree.finger(FIRST_FINGER + index as u64, phase, x, y, None);
    }
  }

  /// The mouse moved: the fingers down move with it.
  pub(crate) fn moved(&mut self, tree: &mut Tree, x: f32, y: f32) {
    self.cursor = (x, y);
    if let Some((fingers, start)) = self.down {
      let gap = Self::gap(tree);
      for (index, (x, y)) in Self::places(fingers, self.cursor, start, gap).into_iter().enumerate() {
        tree.finger(FIRST_FINGER + index as u64, TouchPhase::Move, x, y, None);
      }
    }
    self.mark(tree);
  }

  /// The left button went down: the fingers the modifiers make land.
  pub(crate) fn pressed(&mut self, tree: &mut Tree) {
    if self.down.is_some() || self.pinch.is_some() {
      return;
    }
    let gap = Self::gap(tree);
    self.down = Some((self.held, self.cursor));
    for (index, (x, y)) in Self::places(self.held, self.cursor, self.cursor, gap)
      .into_iter()
      .enumerate()
    {
      tree.finger(FIRST_FINGER + index as u64, TouchPhase::Start, x, y, None);
    }
    self.mark(tree);
  }

  /// The left button came up: the fingers lift.
  pub(crate) fn released(&mut self, tree: &mut Tree) {
    self.lift(tree, TouchPhase::End);
  }

  /// The mouse left the window, or the window lost it: fingers down are taken away, as the system takes a touch.
  pub(crate) fn left(&mut self, tree: &mut Tree) {
    self.end_pinch(tree, TouchPhase::Cancel);
    self.lift(tree, TouchPhase::Cancel);
    tree.set_touch_marks(Vec::new());
  }

  fn lift(&mut self, tree: &mut Tree, phase: TouchPhase) {
    let Some((fingers, start)) = self.down.take() else {
      return;
    };
    let gap = Self::gap(tree);
    for (index, (x, y)) in Self::places(fingers, self.cursor, start, gap).into_iter().enumerate() {
      tree.finger(FIRST_FINGER + index as u64, phase, x, y, None);
    }
    self.mark(tree);
  }
}

impl Tree {
  /// Draws dots where fingers the mouse stands for are, or would land, over everything: points in physical pixels.
  /// Emptied, nothing is drawn.
  pub fn set_touch_marks(&mut self, marks: Vec<(f32, f32)>) {
    if self.touch_marks != marks {
      self.touch_marks = marks;
      self.needs_redraw = true;
    }
  }

  /// Paints the marks of [`set_touch_marks`](Self::set_touch_marks): a ring of the theme's foreground, as large as
  /// a fingertip.
  pub(super) fn push_touch_marks(
    &self,
    rects: &mut Vec<crate::layout::render_list::RectCmd>,
    order: usize,
    scale: f32,
  ) {
    if self.touch_marks.is_empty() {
      return;
    }
    let foreground = self.layout_engine.palette().foreground;
    let radius = 14.0 * scale;
    let fill = crate::node::color::Color::new(foreground.r(), foreground.g(), foreground.b(), 56);
    let ring = crate::node::color::Color::new(foreground.r(), foreground.g(), foreground.b(), 160);
    for (index, (x, y)) in self.touch_marks.iter().enumerate() {
      rects.push(crate::layout::render_list::RectCmd {
        order: order + index,
        x: x - radius,
        y: y - radius,
        width: radius * 2.0,
        height: radius * 2.0,
        color: fill,
        radii: [radius; 4],
        stroke: [1.5 * scale; 4],
        stroke_color: ring,
        stroke_style: crate::layout::render_list::StrokeStyle::default(),
        transform: [1.0, 0.0, 0.0, 1.0],
        transform_origin: [0.0, 0.0],
        clip: crate::layout::quad::ClipRect::default(),
        gradient: None,
        shadow: None,
      });
    }
  }
}
