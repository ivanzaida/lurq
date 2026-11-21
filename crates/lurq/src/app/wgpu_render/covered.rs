//! What the window's frame need not paint: a rect that paints nothing, and whatever an opaque rect over the whole
//! window hides. A screen's background over the app's, or a page over the one under it, would otherwise have every
//! pixel of the window shaded once for each, under what is seen; measured on an iPhone 17, a feed swiped over a
//! video drew six rects the size of the window in each frame, two thirds of its main pass.

use crate::layout::{opacity_layer::LayerStep, render_list::RectCmd};

/// A rect whose draw changes no pixel: no color, gradient, shadow or stroke to show.
pub(super) fn paints_nothing(rect: &RectCmd) -> bool {
  rect.color.a() == 0 && rect.gradient.is_none() && (rect.stroke == [0.0; 4] || rect.stroke_color.a() == 0)
}

/// Whether `rect` covers every pixel of a target `width` by `height` with one opaque color: square, upright and
/// cut by no clip inside the target.
pub(super) fn covers(rect: &RectCmd, width: f32, height: f32) -> bool {
  let clip = &rect.clip;
  let clip_spares =
    !clip.active || (clip.x <= 0.0 && clip.y <= 0.0 && clip.x + clip.width >= width && clip.y + clip.height >= height);
  rect.color.a() == 255
    && rect.gradient.is_none()
    && rect.shadow.is_none()
    && rect.stroke == [0.0; 4]
    && rect.radii == [0.0; 4]
    && rect.transform == [1.0, 0.0, 0.0, 1.0]
    && clip.rounded.iter().all(Option::is_none)
    && clip_spares
    && rect.x <= 0.0
    && rect.y <= 0.0
    && rect.x + rect.width >= width
    && rect.y + rect.height >= height
}

/// The draws of a target that its last covering draw hides: those of the steps from `from_step` on, up to the
/// covering draw `cover`, at step `cover_step`. A backdrop blur reads what the target holds when it runs, and
/// keeps what it read: what was painted before one is painted, so the blur keeps what it would show without the
/// cover.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Covered {
  from_step: usize,
  cover_step: usize,
  cover: usize,
}

impl Covered {
  /// The last draw of `steps` that `is_cover` tells covers the target, if any, with what it hides.
  pub(super) fn of(steps: &[LayerStep], is_cover: impl Fn(usize) -> bool) -> Option<Self> {
    let mut from_step = 0;
    let mut covered = None;
    for (step, kind) in steps.iter().enumerate() {
      match kind {
        LayerStep::Backdrop(_) => from_step = step + 1,
        LayerStep::Draws(draws) => {
          if let Some(cover) = draws.clone().rev().find(|draw| is_cover(*draw)) {
            covered = Some(Self {
              from_step,
              cover_step: step,
              cover,
            });
          }
        }
        LayerStep::Composite(_) => {}
      }
    }
    covered
  }

  /// Whether the layer composited at `step` is hidden.
  pub(super) fn hides_step(&self, step: usize) -> bool {
    (self.from_step..self.cover_step).contains(&step)
  }

  /// Whether `draw`, of the draws of `step`, is hidden.
  pub(super) fn hides_draw(&self, step: usize, draw: usize) -> bool {
    self.hides_step(step) || (step == self.cover_step && step >= self.from_step && draw < self.cover)
  }
}

#[cfg(test)]
mod tests;
