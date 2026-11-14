//! What the panels that point at an element share: the hover card, the
//! toggletip and the teaching tip. The side of the element a panel is on,
//! which side the window let it take, the room between them, and the arrow
//! from the panel to the element, with the panel's edge where it has one.
//!
//! The tooltip draws its own arrow, which this one is drawn as: a square in
//! the panel's color, turned a quarter of a right angle, half of it shown.

use std::{f32::consts::SQRT_2, sync::Arc};

use parking_lot::Mutex;

use super::{TooltipSide, part_style::Length, recipe::ResolvedPart};
use crate::{
  app::ctx::{Ctx, Placement},
  components::{Column, Row, Stack},
  core::{ElementRect, ElementRef, Signal},
  layout::{
    Alignment, StackAlignment,
    layout_kind::{Justify, LayoutKind},
  },
  node::{Element, border::Border, dimension::Dimension, transform::Transform2D},
};

/// The side of its anchor an overlay of `placement` is on.
pub(crate) fn side_of(placement: Placement) -> TooltipSide {
  match placement {
    Placement::TopStart | Placement::Top | Placement::TopEnd => TooltipSide::Top,
    Placement::BottomStart | Placement::Bottom | Placement::BottomEnd => TooltipSide::Bottom,
    Placement::LeftStart | Placement::Left | Placement::LeftEnd => TooltipSide::Left,
    Placement::RightStart | Placement::Right | Placement::RightEnd => TooltipSide::Right,
  }
}

fn vertical(side: TooltipSide) -> bool {
  matches!(side, TooltipSide::Top | TooltipSide::Bottom)
}

fn opposite(side: TooltipSide) -> TooltipSide {
  match side {
    TooltipSide::Top => TooltipSide::Bottom,
    TooltipSide::Bottom => TooltipSide::Top,
    TooltipSide::Left => TooltipSide::Right,
    TooltipSide::Right => TooltipSide::Left,
  }
}

/// Sets `signal` to `value` if it is not that already: a signal set tells
/// what reads it even of the same value.
pub(crate) fn put<T: crate::core::SignalValue + PartialEq + Clone + Send + Sync + 'static>(
  signal: &Signal<T>,
  value: T,
) {
  if signal.get_untracked() != value {
    signal.set(value);
  }
}

/// `element`, in a row of its own where its root lays out no children (a
/// text, a wrapper of one child): what declares an overlay must be a
/// container, or the overlay is never laid out.
pub(crate) fn holds_children(element: Element) -> Element {
  match element.node.layout_kind() {
    LayoutKind::Row { .. } | LayoutKind::Column { .. } | LayoutKind::Stack { .. } | LayoutKind::Grid(_) => element,
    _ => Row::new().child(element).into(),
  }
}

/// What a panel of a mounted component learns from where the overlay put
/// it, which the overlay does not say: whether it is on the other side of
/// its anchor, where there was no room on its own, and how far along its
/// edge the arrow is moved to point at the anchor's middle, where the panel
/// was pushed along to stay in the window or lines up with an edge of it.
#[derive(Clone)]
pub(crate) struct Placed {
  flipped: Signal<bool>,
  aim: Signal<f32>,
  placing: Arc<Mutex<Placing>>,
}

/// What the reader of where the panel was put needs from the last render.
struct Placing {
  side: TooltipSide,
  anchor: ElementRef,
  along: f32,
}

impl Placed {
  /// What reads where the overlay puts `shown`, the root of what shows.
  pub(crate) fn new(ctx: &mut Ctx, shown: &ElementRef) -> Self {
    let placed = Self {
      flipped: ctx.signal(false),
      aim: ctx.signal(0.0_f32),
      placing: Arc::new(Mutex::new(Placing {
        side: TooltipSide::Bottom,
        anchor: ElementRef::new(),
        along: 0.0,
      })),
    };
    let reader = placed.clone();
    shown.observe_rect(move |rect| reader.read(rect));
    placed
  }

  /// The side the panel is on: that of `placement`, or the other one. Read in
  /// a render, it renders again when the panel goes to the other side.
  pub(crate) fn side(&self, placement: Placement) -> TooltipSide {
    let side = side_of(placement);
    if self.flipped.get() { opposite(side) } else { side }
  }

  /// How far along the panel's edge from its middle the arrow is moved.
  pub(crate) fn aim(&self) -> f32 {
    self.aim.get()
  }

  /// What this render gives the reader: the panel's placement, its anchor,
  /// and how long its arrow is along the edge (0 without one).
  pub(crate) fn note(&self, placement: Placement, anchor: &ElementRef, along: f32) {
    let mut placing = self.placing.lock();
    placing.side = side_of(placement);
    placing.anchor = anchor.clone();
    placing.along = along;
  }

  fn read(&self, rect: ElementRect) {
    // Read and let go before a signal is set: what the signal tells may read it too.
    let (side, anchor, along) = {
      let placing = self.placing.lock();
      (placing.side, placing.anchor.clone(), placing.along)
    };
    let ((x, y), (anchor_x, anchor_y)) = (rect.center(), anchor.bounds().center());
    let other = match side {
      TooltipSide::Top => y > anchor_y,
      TooltipSide::Bottom => y < anchor_y,
      TooltipSide::Left => x > anchor_x,
      TooltipSide::Right => x < anchor_x,
    };
    put(&self.flipped, other);
    // The root is as long as the panel along the side's edge: its room is on the other axis.
    let (off, length) = if vertical(side) {
      (anchor_x - x, rect.width)
    } else {
      (anchor_y - y, rect.height)
    };
    let limit = (length / 2.0 - along).max(0.0);
    let off = (off.clamp(-limit, limit) * 2.0).round() / 2.0;
    if (self.aim.get_untracked() - off).abs() > 0.25 {
      self.aim.set(off);
    }
  }
}

/// How long the arrow is along the panel's edge, and how far out from it it
/// goes: the size of the arrow's part, which a recipe gives as it is above or
/// below the panel. `None` where the recipe gives it no size in pixels.
pub(crate) fn arrow_size(arrow: &ResolvedPart, side: TooltipSide) -> Option<(f32, f32)> {
  let theme = arrow.theme();
  let px = |length: Option<Length>| match length?.resolve(theme)? {
    Dimension::Px(px) => Some(px),
    _ => None,
  };
  let rest = arrow.rest();
  let (width, height) = (px(rest.width)?, px(rest.height)?);
  Some(if vertical(side) {
    (width, height)
  } else {
    (height, width)
  })
}

/// The arrow on the `side` of a panel toward what it points at: a square in
/// the fill of `panel`, turned a quarter of a right angle, its middle on the
/// edge of the arrow's part that meets the panel, so that the part, which
/// clips, shows the half that points away from it. Where the panel has an
/// edge, the square has it too, and it reaches over the panel's edge by its
/// width so that the panel is open where the arrow leaves it: it is to be
/// drawn after the panel. Moved along the edge by `aim`. `None` where the
/// recipe gives the arrow's part no size in pixels or the panel no fill.
pub(crate) fn arrow(arrow: &ResolvedPart, panel: &ResolvedPart, side: TooltipSide, aim: f32) -> Option<Element> {
  let (along, away) = arrow_size(arrow, side)?;
  let rest = panel.rest();
  let fill = rest.background?;
  let edge = rest.border.map(|border| {
    let width = border.width.resolve(&panel.theme().border_sizes());
    (width, Border::inside(width, border.color))
  });
  let over = edge.as_ref().map_or(0.0, |(width, _)| *width);
  let side_length = along / SQRT_2;
  // The edge of the arrow's part that meets the panel is its near one above or left of what it points at, its
  // far one below or right of it; the part reaches over the panel's edge by the edge's width.
  let (shift, over) = match side {
    TooltipSide::Top | TooltipSide::Left => (-away / 2.0, -over),
    TooltipSide::Bottom | TooltipSide::Right => (away / 2.0, over),
  };
  let (dx, dy) = if vertical(side) { (0.0, shift) } else { (shift, 0.0) };
  let mut tip = Row::new()
    .width(side_length)
    .height(side_length)
    .flex_shrink(0.0)
    .background(fill)
    .offset(dx, dy)
    .transform(Transform2D::rotate_deg(45.0));
  if let Some((_, border)) = edge {
    tip = tip.border(border);
  }
  // The part's long side is its main axis, so the square is not shrunk to its short one. It is moved along the
  // panel by the aim, not laid out there: the panel's size does not change with it.
  Some(if vertical(side) {
    Row::new()
      .justify(Justify::Center)
      .align_items(Alignment::Center)
      .flex_shrink(0.0)
      .part(arrow)
      .offset(aim, -over)
      .child(tip)
      .into()
  } else {
    Column::new()
      .justify(Justify::Center)
      .align_items(Alignment::Center)
      .flex_shrink(0.0)
      .part(arrow)
      .offset(-over, aim)
      .child(tip)
      .into()
  })
}

/// What shows: `panel`, with the room the arrow takes between it and what it
/// points at, and the arrow drawn over its edge. `root` is the part that
/// holds them and keeps the room between all of it and what it points at;
/// it lets the panel's shadow out.
pub(crate) fn frame(
  root: &ResolvedPart,
  panel: Element,
  arrow: Option<Element>,
  away: f32,
  side: TooltipSide,
) -> Stack {
  let (align, padded) = match side {
    TooltipSide::Top => (StackAlignment::BottomCenter, Column::new().padding_bottom(away)),
    TooltipSide::Bottom => (StackAlignment::TopCenter, Column::new().padding_top(away)),
    TooltipSide::Left => (StackAlignment::CenterEnd, Column::new().padding_right(away)),
    TooltipSide::Right => (StackAlignment::CenterStart, Column::new().padding_left(away)),
  };
  let away_room = if arrow.is_some() { padded } else { Column::new() };
  let mut stack = Stack::new().stack_align(align).part(root).overflow_visible().child(
    away_room
      .align_items(Alignment::Stretch)
      .overflow_visible()
      .child(panel),
  );
  if let Some(arrow) = arrow {
    stack = stack.child(arrow);
  }
  stack
}
