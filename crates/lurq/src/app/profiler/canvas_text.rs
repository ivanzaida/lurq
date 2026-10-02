//! Per-pass Canvas text aggregation; no shared-engine resets or collector locks.
use std::{cell::Cell, marker::PhantomData, rc::Rc, time::Instant};

use super::CanvasTextProfile;

thread_local! {
  static CURRENT: Cell<Option<CanvasTextProfile>> = const { Cell::new(None) };
}

/// Stack-scoped thread ownership also isolates nested passes from another window.
pub(crate) struct PassScope {
  previous: Option<CanvasTextProfile>,
  _same_thread: PhantomData<Rc<()>>,
}

impl PassScope {
  pub(super) fn new(active: bool) -> Self {
    Self {
      previous: CURRENT.replace(active.then(CanvasTextProfile::default)),
      _same_thread: PhantomData,
    }
  }
}

impl Drop for PassScope {
  fn drop(&mut self) {
    CURRENT.set(self.previous);
  }
}

pub(super) fn snapshot() -> Option<CanvasTextProfile> {
  CURRENT.get()
}

fn update(change: impl FnOnce(&mut CanvasTextProfile)) {
  if let Some(mut profile) = CURRENT.get() {
    change(&mut profile);
    CURRENT.set(Some(profile));
  }
}

pub(crate) enum Call {
  Measure,
  Fill,
}

pub(crate) fn call(kind: Call) {
  update(|profile| {
    let count = match kind {
      Call::Measure => &mut profile.measure_calls,
      Call::Fill => &mut profile.fill_calls,
    };
    *count = count.saturating_add(1);
  });
}

pub(crate) enum Stage {
  Total,
  BufferFontShape,
  GlyphPrepare,
  BitmapComposition,
}

pub(crate) struct Timer {
  started: Option<Instant>,
  stage: Stage,
}

impl Timer {
  pub(crate) fn new(stage: Stage) -> Self {
    Self {
      started: CURRENT.get().map(|_| Instant::now()),
      stage,
    }
  }
}

impl Drop for Timer {
  fn drop(&mut self) {
    if let Some(started) = self.started {
      update(|profile| {
        let duration = match self.stage {
          Stage::Total => &mut profile.total,
          Stage::BufferFontShape => &mut profile.buffer_font_shape,
          Stage::GlyphPrepare => &mut profile.glyph_prepare,
          Stage::BitmapComposition => &mut profile.bitmap_composition,
        };
        *duration += started.elapsed();
      });
    }
  }
}

pub(crate) fn shape_hit(hit: bool) {
  update(|profile| {
    profile.shape_calls = profile.shape_calls.saturating_add(1);
    let count = if hit {
      &mut profile.shape_cache_hits
    } else {
      &mut profile.shape_cache_misses
    };
    *count = count.saturating_add(1);
  });
}

pub(crate) fn evicted() {
  update(|profile| profile.shape_cache_evictions = profile.shape_cache_evictions.saturating_add(1));
}

pub(crate) fn produced(bytes: usize) {
  update(|profile| profile.produced_bitmap_bytes = profile.produced_bitmap_bytes.saturating_add(bytes as u64));
}
