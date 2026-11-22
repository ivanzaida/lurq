//! Paint of glass: a node with a glass gives a
//! [`QuadContent::Backdrop`] quad that holds its box as the glass's shape,
//! before everything else it paints.
//!
//! The glass nodes under a glass container (`Node::glass_container`) are one
//! surface. The container gives the quad, before anything in it is painted,
//! and each glass node under it adds its box to that quad instead of giving
//! one of its own, so the whole surface is drawn at once, under everything
//! the container holds. The first shape's node gives the material. A quad
//! holds [`MAX_GLASS_SHAPES`] shapes; a glass node that finds it full is a
//! glass of its own.

use std::time::Duration;

use super::{LayoutEngine, transformed_quad_frame};
use crate::{
  layout::{
    glass::{GlassTouch, MAX_GLASS_SHAPES},
    layout_result::LayoutResult,
    quad::{ClipRect, GlassQuad, GlassQuadShape, Quad, QuadContent},
  },
  node::{node::Node, transform::Transform2D},
  time::Instant,
};

/// A glass that answers a press (`Glass::interactive`), held down or on its
/// way back: how far it is pressed in, as a weight on a spring, and where
/// it was last touched.
pub(super) struct GlassPress {
  /// 0 at rest, 1 held down; past either on its way, as a spring goes.
  amount: f32,
  speed: f32,
  /// Where the pointer was while the glass was held, in the window's
  /// logical pixels.
  touched: [f32; 2],
  /// The frame `amount` was last moved for.
  at: Instant,
}

/// The spring a pressed glass is on: quick, and it overshoots once on its
/// way back.
const PRESS_STIFFNESS: f32 = 420.0;
const PRESS_DAMPING: f32 = 22.0;
/// How many logical pixels a glass held down grows by on its shorter side,
/// and the most it grows as a part of itself.
const PRESS_GROWTH: f32 = 7.0;
const PRESS_GROWTH_MOST: f32 = 0.12;
/// How long after a frame the next one of a glass that is moving is painted.
const PRESS_FRAME: Duration = Duration::from_millis(8);

impl LayoutEngine {
  /// Gives `node`'s glass: its shape goes to the container's quad when the
  /// node is under one with room, and to a quad of its own otherwise. A
  /// glass container gives the quad its glass nodes fill, with its own
  /// shape first when it is a glass itself.
  #[allow(clippy::too_many_arguments)]
  pub(super) fn push_glass_quad(
    &self,
    node: &Node,
    result: &LayoutResult,
    abs_x: f32,
    abs_y: f32,
    transform: Transform2D,
    clip: ClipRect,
    quads: &mut Vec<Quad>,
  ) {
    let mut glass = node.effective_glass();
    let container = node.glass_merge();
    if glass.is_none() && container.is_none() {
      return;
    }
    if let Some(glass) = &mut glass
      && let Some((amount, [x, y])) = self.glass_press(node)
    {
      // Lit from where it is touched, as far as its longer side reaches.
      glass.touch = Some(GlassTouch {
        x: x - abs_x,
        y: y - abs_y,
        radius: result.size.width.max(result.size.height).clamp(48.0, 160.0) * 0.75,
        strength: amount.clamp(0.0, 1.0),
      });
    }
    let (x, y, quad_transform, transform_origin) = transformed_quad_frame(abs_x, abs_y, transform);
    let shape = glass.map(|_| GlassQuadShape {
      x,
      y,
      width: result.size.width,
      height: result.size.height,
      border_radius: node.get_border_radius(&self.radii.borrow()),
      transform: quad_transform,
      transform_origin,
    });
    if container.is_none()
      && let (Some(glass), Some(shape), Some(scope)) = (glass, shape, self.glass_scope.get())
      && let Some(QuadContent::Backdrop {
        glass: Some(surface), ..
      }) = quads.get_mut(scope).map(|quad| &mut quad.content)
      && surface.shapes.len() < MAX_GLASS_SHAPES
    {
      if surface.shapes.is_empty() {
        surface.glass = glass;
      }
      surface.shapes.push(shape);
      surface.keys.push(node.node_id().value());
      return;
    }
    quads.push(Quad {
      x,
      y,
      width: result.size.width,
      height: result.size.height,
      opacity: node.effective_opacity().clamp(0.0, 1.0),
      transform: quad_transform,
      transform_origin,
      content: QuadContent::Backdrop {
        blur: 0.0,
        glass: Some(Box::new(GlassQuad {
          glass: glass.unwrap_or_default(),
          merge: container.unwrap_or(0.0).max(0.0),
          shapes: shape.into_iter().collect(),
          keys: shape.map(|_| node.node_id().value()).into_iter().collect(),
        })),
      },
      border_radius: None,
      border: None,
      clip,
    });
  }

  /// Makes `node`, a glass container, the one the glass nodes painted from
  /// here on add their shapes to: its quad is among the ones the node gave,
  /// `quads[start..]`. Returns the scope to give back to
  /// [`Self::leave_glass_scope`]; `None` for a node that is no container.
  pub(super) fn enter_glass_scope(&self, node: &Node, start: usize, quads: &[Quad]) -> Option<Option<usize>> {
    node.glass_merge()?;
    let index = quads[start..]
      .iter()
      .position(|quad| matches!(quad.content, QuadContent::Backdrop { glass: Some(_), .. }))?;
    Some(self.glass_scope.replace(Some(start + index)))
  }

  /// Starts the frame painted at `now`, with the pointer at `pointer` (the
  /// window's logical pixels; `None` where there is none): the glasses that
  /// answer a press move by this time.
  pub(crate) fn begin_glass_frame(&self, now: Instant, pointer: Option<[f32; 2]>) {
    self.glass_frame_at.set(Some(now));
    self.glass_pointer.set(pointer);
    self.glass_change_at.set(None);
  }

  /// When a glass painted since [`Self::begin_glass_frame`] next looks
  /// different with nothing else happening: one that is springing. `None`
  /// when every glass is at rest.
  pub(crate) fn take_glass_change(&self) -> Option<Instant> {
    // A glass that was not painted in the frame is gone: it comes back at rest.
    let frame = self.glass_frame_at.get();
    self
      .glass_presses
      .borrow_mut()
      .retain(|_, press| Some(press.at) == frame);
    self.glass_change_at.take()
  }

  /// How far `node`'s glass is pressed in this frame and where it was
  /// touched; `None` for a glass that does not answer a press, and for one
  /// at rest. Moves the glass's spring to the frame's time.
  fn glass_press(&self, node: &Node) -> Option<(f32, [f32; 2])> {
    if !node.effective_glass().is_some_and(|glass| glass.interactive) {
      return None;
    }
    let now = self.glass_frame_at.get()?;
    let held = node.style_state.is_active();
    let mut presses = self.glass_presses.borrow_mut();
    let key = node.node_id().value();
    if !held && !presses.contains_key(&key) {
      return None;
    }
    let press = presses.entry(key).or_insert_with(|| GlassPress {
      amount: 0.0,
      speed: 0.0,
      touched: self.glass_pointer.get().unwrap_or_default(),
      at: now,
    });
    if held && let Some(pointer) = self.glass_pointer.get() {
      press.touched = pointer;
    }
    let target = f32::from(u8::from(held));
    // A frame that comes late moves the spring no further than a slow one would.
    let mut left = now.saturating_duration_since(press.at).as_secs_f32().min(0.064);
    while left > 0.0 {
      let step = left.min(0.004);
      let pull = (target - press.amount) * PRESS_STIFFNESS - press.speed * PRESS_DAMPING;
      press.speed += pull * step;
      press.amount += press.speed * step;
      left -= step;
    }
    press.at = now;
    let resting = (press.amount - target).abs() < 0.002 && press.speed.abs() < 0.02;
    if resting {
      press.amount = target;
      press.speed = 0.0;
    } else {
      let next = now + PRESS_FRAME;
      self
        .glass_change_at
        .set(Some(self.glass_change_at.get().map_or(next, |at| at.min(next))));
    }
    let found = (press.amount, press.touched);
    if resting && !held {
      presses.remove(&key);
      return None;
    }
    Some(found)
  }

  /// How much larger `node` is painted for its glass being held down: 1
  /// and a little. `None` for a glass at rest.
  pub(super) fn glass_press_scale(&self, node: &Node, result: &LayoutResult) -> Option<f32> {
    // The fast way out for all that is no glass.
    node.rare_style.glass.as_ref()?;
    let (amount, _) = self.glass_press(node)?;
    let shorter = result.size.width.min(result.size.height).max(1.0);
    Some(1.0 + (PRESS_GROWTH / shorter).min(PRESS_GROWTH_MOST) * amount)
  }

  /// Tells how dark what lies behind each glass was found to be when the
  /// frame before was put together: by the ids of the glasses' nodes, 0 over
  /// a light backdrop to 1 over a dark one. A glass that follows its
  /// backdrop gives what is on it its colour by this (see
  /// [`Self::glass_ink`]).
  pub(crate) fn set_glass_tones(&self, tones: impl Iterator<Item = (u64, f32)>) {
    let mut held = self.glass_tones.borrow_mut();
    held.clear();
    held.extend(tones);
  }

  /// The colour `node`'s glass gives what is on it: the glass's ink, as its
  /// backdrop has it. `None` for a node without a glass, and for a glass
  /// that names no ink.
  pub(super) fn glass_ink(&self, node: &Node) -> Option<crate::node::color::Color> {
    let glass = node.effective_glass()?;
    glass.ink?;
    let dark = self
      .glass_tones
      .borrow()
      .get(&node.node_id().value())
      .copied()
      .unwrap_or(f32::from(u8::from(self.dark_appearance.get())));
    glass.toned(dark).ink
  }

  pub(super) fn leave_glass_scope(&self, scope: Option<Option<usize>>) {
    if let Some(outer) = scope {
      self.glass_scope.set(outer);
    }
  }
}
