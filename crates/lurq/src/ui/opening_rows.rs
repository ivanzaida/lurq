//! The rows of a windowed list that open and close under a parent row: a property grid's category, a tree's node.
//!
//! A component whose rows are items of a [`VirtualizedList`](crate::components::VirtualizedList) has no panel to
//! hold the rows of what opens, so nothing whose height runs as an accordion's panel's does. [`OpeningRows`] gives
//! each such row the motion instead: its height runs from nothing to its own as its parent opens, the first first,
//! and back to nothing as it closes, the last first, so that together they slide open and shut under the parent as a
//! panel would, over the theme's base duration. What it keeps from render to render, the component holds:
//!
//! - which parents are opening, closing, and moving, by a key of the component's own ([`OpeningRows::started`] as a
//!   person opens or closes one);
//! - the rows of a closing parent, which the component keeps in its list, inert and out of the keys, until the motion
//!   has ended and a timeout lets them go;
//! - the rows of an opening parent, which are built at no height and given theirs on the next turn of the loop, for
//!   their height to run from nothing;
//! - what the list counts a row it has not measured as: where its height starts while its parent moves
//!   ([`RowMotion::estimate`]), so that what is below is built where it is on the way.
//!
//! Where motion is reduced nothing moves: a parent opens and closes at once, and the rows of a closing one go at
//! once.

use std::{
  collections::{HashMap, HashSet},
  sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
  },
  time::Duration,
};

use super::theme::current_theme;
use crate::{
  animation::{Easing, Transition},
  app::{
    ctx::{Ctx, Timeout},
    theme::{MotionDuration, MotionEasing, Theme},
  },
  components::Column,
  core::Signal,
  layout::Alignment,
  node::{Element, dimension::Dimension},
};

/// The motion of the rows of a windowed list under parents that open and close. Made in a mounted component's
/// `create`, which keeps it.
pub(crate) struct OpeningRows {
  /// Parents whose rows are kept while they close.
  closing: Mutex<HashSet<Arc<str>>>,
  /// Parents just opened, whose rows are built at no height first.
  opening: Mutex<HashSet<Arc<str>>>,
  /// Parents in motion, and whether it started from rest: their rows then move one after another.
  moving: Mutex<HashMap<Arc<str>, bool>>,
  /// What ends the motion: after it, the closed rows go.
  settle: Timeout,
  /// What gives rows built at no height their height, on the turn of the loop after the one that laid them out.
  grow: Timeout,
  reduced: AtomicBool,
}

/// Where the parents are, as one render reads them.
pub(crate) struct Moving {
  closing: HashSet<Arc<str>>,
  opening: HashSet<Arc<str>>,
  moving: HashMap<Arc<str>, bool>,
  base: Duration,
  easing: Easing,
}

/// How one row of a moving parent moves: the height it runs to, from where, when and how, and whether it is going.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RowMotion {
  /// The height the row runs to.
  pub(crate) height: f32,
  /// What the list counts the row as before it measures it: the height it runs from while its parent moves, its
  /// height at rest.
  pub(crate) estimate: f32,
  pub(crate) delay: Duration,
  pub(crate) duration: Duration,
  pub(crate) easing: Easing,
  /// The row of a closing parent: kept while it closes, inert, reached neither by Tab nor by the keys of the list.
  pub(crate) kept: bool,
}

impl OpeningRows {
  /// The motion of a mounted component, which `wake` renders again as it moves on.
  pub(crate) fn new(ctx: &mut Ctx, wake: &Signal<u64>) -> Arc<Self> {
    // The motion is over once its rows have run their turns: the opening's duration, and a frame for the last.
    let over = current_theme().duration_value(MotionDuration::Base) + Duration::from_millis(50);
    Arc::new_cyclic(|this: &std::sync::Weak<Self>| {
      let settle = ctx.create_timeout(over, {
        let (this, wake) = (this.clone(), wake.clone());
        move || {
          if let Some(this) = this.upgrade() {
            this.settled();
          }
          wake.update(|wake| *wake = wake.wrapping_add(1));
        }
      });
      let grow = ctx.create_timeout(Duration::ZERO, {
        let (this, wake) = (this.clone(), wake.clone());
        move || {
          if let Some(this) = this.upgrade() {
            this.opening.lock().unwrap().clear();
          }
          wake.update(|wake| *wake = wake.wrapping_add(1));
        }
      });
      Self {
        closing: Mutex::default(),
        opening: Mutex::default(),
        moving: Mutex::default(),
        settle,
        grow,
        reduced: AtomicBool::new(false),
      }
    })
  }

  /// A person opened (`to`) or closed the parent `key`: its motion starts. From rest its rows move one after
  /// another; turned round on the way, each moves back from where it is.
  pub(crate) fn started(&self, key: &Arc<str>, to: bool) {
    if self.reduced.load(Ordering::Relaxed) {
      return;
    }
    let mut moving = self.moving.lock().unwrap();
    let from_rest = !moving.contains_key(key);
    moving.insert(key.clone(), from_rest);
    drop(moving);
    let (mut closing, mut opening) = (self.closing.lock().unwrap(), self.opening.lock().unwrap());
    if to {
      // Rows kept as they closed are there already, and go back from their height.
      if !closing.remove(key) {
        opening.insert(key.clone());
      }
    } else {
      opening.remove(key);
      closing.insert(key.clone());
    }
    drop((closing, opening));
    self.settle.restart();
  }

  fn settled(&self) {
    self.closing.lock().unwrap().clear();
    self.opening.lock().unwrap().clear();
    self.moving.lock().unwrap().clear();
  }

  /// Where the parents are for a render in `theme`: where motion is reduced, nothing moves and the closing rows go.
  pub(crate) fn read(&self, theme: &Theme) -> Moving {
    let reduced = theme.reduced_motion();
    self.reduced.store(reduced, Ordering::Relaxed);
    if reduced {
      self.settled();
    }
    Moving {
      closing: self.closing.lock().unwrap().clone(),
      opening: self.opening.lock().unwrap().clone(),
      moving: self.moving.lock().unwrap().clone(),
      base: theme.duration_value(MotionDuration::Base),
      easing: theme.easing_value(MotionEasing::Standard),
    }
  }

  /// After a render: rows it built at no height are given their height on the next turn of the loop.
  pub(crate) fn rendered(&self, moving: &Moving) {
    if !moving.opening.is_empty() {
      self.grow.restart();
    }
  }
}

impl Moving {
  /// Whether the rows of the parent `key`, closed now, are kept while they close.
  pub(crate) fn keeps(&self, key: &str) -> bool {
    self.closing.contains(key)
  }

  /// Whether the parent `key` is opening or closing.
  pub(crate) fn moves(&self, key: &str) -> bool {
    self.moving.contains_key(key)
  }

  /// A row under no parent in motion: at its height, and its frame ready for a motion to come.
  pub(crate) fn still(&self, height: f32) -> RowMotion {
    RowMotion {
      height,
      estimate: height,
      delay: Duration::ZERO,
      duration: self.base,
      easing: self.easing.clone(),
      kept: false,
    }
  }

  /// How the row `place` of the rows under the parent `key` moves to `height`: kept where the parent is closing.
  /// `count` is how many move in turn: the rows under the parent, or fewer, as many as a view holds, for a parent of
  /// thousands; a row after them moves with the last as the parent opens and first as it closes.
  pub(crate) fn row(&self, key: &str, place: usize, count: usize, height: f32, kept: bool) -> RowMotion {
    let count = count.max(1) as u32;
    let place = (place as u32).min(count - 1);
    let first = self.opening.contains(key);
    let moving = self.moving.get(key).copied();
    let in_turn = moving.unwrap_or(false);
    let (delay, duration) = if in_turn {
      let step = self.base / count;
      let turn = if kept { count - 1 - place } else { place };
      (step * turn, step)
    } else {
      (Duration::ZERO, self.base)
    };
    RowMotion {
      height: if kept || first { 0.0 } else { height },
      estimate: match (moving.is_some(), kept) {
        (true, false) => 0.0,
        _ => height,
      },
      delay,
      duration,
      easing: if in_turn { Easing::Linear } else { self.easing.clone() },
      kept,
    }
  }
}

impl RowMotion {
  /// `row` in a frame of the height it runs to, clipped, inert while it is going.
  pub(crate) fn frame(&self, row: impl Into<Element>) -> Element {
    Column::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Stretch)
      .clip()
      .height(self.height)
      .transition(
        Transition::height()
          .duration(self.duration)
          .delay_ms(self.delay.as_millis() as u64)
          .easing(self.easing.clone()),
      )
      .inert(self.kept)
      .child(row)
      .into()
  }
}
