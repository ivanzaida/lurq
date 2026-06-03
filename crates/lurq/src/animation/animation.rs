use std::{
  collections::HashMap,
  time::{Duration, Instant},
};

use super::{
  easing::Easing,
  interpolate::write_property,
  keyframes::{KeyframeEntry, Keyframes, KeyframesId},
};
use crate::core::NodeId;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AnimationDirection {
  Normal,
  Reverse,
  Alternate,
  AlternateReverse,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AnimationFillMode {
  None,
  Forwards,
  Backwards,
  Both,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AnimationIterationCount {
  Count(f64),
  Infinite,
}

#[derive(Clone, Debug)]
pub struct Animation {
  pub keyframes: KeyframesId,
  pub duration: Duration,
  pub delay: Duration,
  pub easing: Easing,
  pub direction: AnimationDirection,
  pub fill_mode: AnimationFillMode,
  pub iteration_count: AnimationIterationCount,
}

impl Animation {
  pub fn new(keyframes: impl Into<KeyframesId>) -> Self {
    Self {
      keyframes: keyframes.into(),
      duration: Duration::from_millis(300),
      delay: Duration::ZERO,
      easing: Easing::EASE,
      direction: AnimationDirection::Normal,
      fill_mode: AnimationFillMode::None,
      iteration_count: AnimationIterationCount::Count(1.0),
    }
  }

  pub fn duration(mut self, d: Duration) -> Self {
    self.duration = d;
    self
  }

  pub fn duration_ms(mut self, ms: u64) -> Self {
    self.duration = Duration::from_millis(ms);
    self
  }

  pub fn delay(mut self, d: Duration) -> Self {
    self.delay = d;
    self
  }

  pub fn delay_ms(mut self, ms: u64) -> Self {
    self.delay = Duration::from_millis(ms);
    self
  }

  pub fn easing(mut self, e: Easing) -> Self {
    self.easing = e;
    self
  }

  pub fn linear(mut self) -> Self {
    self.easing = Easing::Linear;
    self
  }

  pub fn direction(mut self, d: AnimationDirection) -> Self {
    self.direction = d;
    self
  }

  pub fn fill_mode(mut self, f: AnimationFillMode) -> Self {
    self.fill_mode = f;
    self
  }

  pub fn iteration_count(mut self, c: AnimationIterationCount) -> Self {
    self.iteration_count = c;
    self
  }

  pub fn infinite(mut self) -> Self {
    self.iteration_count = AnimationIterationCount::Infinite;
    self
  }

  pub fn alternate(mut self) -> Self {
    self.direction = AnimationDirection::Alternate;
    self
  }
}

struct AnimationRun {
  keyframes: KeyframesId,
  duration: Duration,
  delay: Duration,
  easing: Easing,
  direction: AnimationDirection,
  fill_mode: AnimationFillMode,
  iteration_count: f64,
  start_time: Instant,
  finished: bool,
}

pub struct AnimationEngine {
  active: HashMap<(NodeId, KeyframesId), AnimationRun>,
  keyframe_store: HashMap<KeyframesId, Vec<KeyframeEntry>>,
  pub has_active: bool,
}

impl AnimationEngine {
  pub fn new() -> Self {
    Self {
      active: HashMap::new(),
      keyframe_store: HashMap::new(),
      has_active: false,
    }
  }

  pub fn register_keyframes(&mut self, keyframes: Keyframes) {
    self.keyframe_store.insert(keyframes.id, keyframes.frames);
  }

  pub(crate) fn clear_state(&mut self) {
    self.active.clear();
    self.has_active = false;
  }

  pub(crate) fn tick(&mut self, root: &mut crate::node::Node, now: Instant) {
    self.has_active = false;
    self.tick_recursive(root, now);
    self.has_active = !self.active.is_empty();
  }

  fn tick_recursive(&mut self, node: &mut crate::node::Node, now: Instant) -> bool {
    let mut needs_layout = false;
    let node_id = node.node_id();
    if node_id.is_assigned() {
      if let Some(spec) = &node.animation {
        let spec = spec.clone();
        needs_layout = self.process_node(node, &spec, now);
      }
    }

    for child in &mut node.children {
      if self.tick_recursive(child, now) {
        needs_layout = true;
      }
    }

    if needs_layout {
      node.layout_cache.invalidate();
    }

    needs_layout
  }

  fn process_node(&mut self, node: &mut crate::node::Node, spec: &Animation, now: Instant) -> bool {
    let node_id = node.node_id();
    let key = (node_id, spec.keyframes);
    let iteration_count = match spec.iteration_count {
      AnimationIterationCount::Count(c) => c,
      AnimationIterationCount::Infinite => f64::INFINITY,
    };

    let run = self.active.entry(key).or_insert_with(|| AnimationRun {
      keyframes: spec.keyframes,
      duration: spec.duration,
      delay: spec.delay,
      easing: spec.easing,
      direction: spec.direction,
      fill_mode: spec.fill_mode,
      iteration_count,
      start_time: now,
      finished: false,
    });

    if run.finished {
      return false;
    }

    let frames = match self.keyframe_store.get(&run.keyframes) {
      Some(f) => f,
      None => return false,
    };

    let elapsed = now.duration_since(run.start_time);

    if elapsed < run.delay {
      if matches!(run.fill_mode, AnimationFillMode::Backwards | AnimationFillMode::Both) {
        let progress = directed_progress(0.0, run.direction, 0.0);
        return apply_keyframe_at(node, frames, progress as f32, &run.easing);
      }
      return false;
    }

    let active_elapsed = (elapsed - run.delay).as_secs_f64();
    let dur = run.duration.as_secs_f64();

    if dur <= 0.0 {
      run.finished = true;
      self.active.get_mut(&(node_id, spec.keyframes)).unwrap().finished = true;
      return false;
    }

    let total = dur * run.iteration_count;
    if active_elapsed >= total {
      run.finished = true;
      if matches!(run.fill_mode, AnimationFillMode::Forwards | AnimationFillMode::Both) {
        let final_iter = (run.iteration_count - 1.0).max(0.0);
        let progress = directed_progress(1.0, run.direction, final_iter);
        return apply_keyframe_at(node, frames, progress as f32, &run.easing);
      }
      return false;
    }

    let iteration = (active_elapsed / dur).floor();
    let raw_progress = (active_elapsed / dur) - iteration;
    let progress = directed_progress(raw_progress, run.direction, iteration);
    apply_keyframe_at(node, frames, progress as f32, &run.easing)
  }
}

fn directed_progress(progress: f64, direction: AnimationDirection, iteration: f64) -> f64 {
  match direction {
    AnimationDirection::Normal => progress,
    AnimationDirection::Reverse => 1.0 - progress,
    AnimationDirection::Alternate => {
      if iteration as u64 % 2 == 0 {
        progress
      } else {
        1.0 - progress
      }
    }
    AnimationDirection::AlternateReverse => {
      if iteration as u64 % 2 == 0 {
        1.0 - progress
      } else {
        progress
      }
    }
  }
}

fn apply_keyframe_at(
  node: &mut crate::node::Node,
  frames: &[KeyframeEntry],
  progress: f32,
  overall_easing: &Easing,
) -> bool {
  if frames.is_empty() {
    return false;
  }

  let mut layout_affected = false;
  let (lo, hi) = find_surrounding_frames(frames, progress);
  let lo_frame = &frames[lo];
  let hi_frame = &frames[hi];

  if lo == hi {
    for (prop, value) in &lo_frame.values {
      layout_affected |= write_property(node, *prop, value);
    }
    return layout_affected;
  }

  let span = hi_frame.offset - lo_frame.offset;
  let local = if span > 0.0 {
    ((progress - lo_frame.offset) / span).clamp(0.0, 1.0)
  } else {
    1.0
  };

  let easing = lo_frame.easing.as_ref().unwrap_or(overall_easing);
  let t = easing.evaluate(local as f64) as f32;

  for (prop, lo_val) in &lo_frame.values {
    if let Some((_, hi_val)) = hi_frame.values.iter().find(|(p, _)| p == prop) {
      let interpolated = lo_val.lerp(hi_val, t);
      layout_affected |= write_property(node, *prop, &interpolated);
    } else {
      layout_affected |= write_property(node, *prop, lo_val);
    }
  }

  layout_affected
}

fn find_surrounding_frames(frames: &[KeyframeEntry], progress: f32) -> (usize, usize) {
  if frames.len() == 1 {
    return (0, 0);
  }

  for i in 1..frames.len() {
    if frames[i].offset >= progress {
      return (i - 1, i);
    }
  }

  let last = frames.len() - 1;
  (last, last)
}
