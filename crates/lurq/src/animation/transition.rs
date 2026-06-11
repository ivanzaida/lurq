use std::{
  collections::HashMap,
  time::{Duration, Instant},
};

use super::{
  easing::Easing,
  interpolate::{
    AnimatableProperty, AnimatableValue, clear_overrides, property_accessors, read_target, write_property,
  },
};
use crate::core::NodeId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TransitionProperty {
  All,
  Single(AnimatableProperty),
}

#[derive(Clone, Debug)]
pub struct Transition {
  pub property: TransitionProperty,
  pub duration: Duration,
  pub delay: Duration,
  pub easing: Easing,
}

impl Transition {
  fn single(prop: AnimatableProperty) -> Self {
    Self {
      property: TransitionProperty::Single(prop),
      duration: Duration::from_millis(300),
      delay: Duration::ZERO,
      easing: Easing::EASE,
    }
  }

  pub fn all() -> Self {
    Self {
      property: TransitionProperty::All,
      duration: Duration::from_millis(300),
      delay: Duration::ZERO,
      easing: Easing::EASE,
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
}

property_accessors!(Transition {
  BackgroundColor => background_color,
  BorderColor => border_color,
  BorderWidthTop => border_width_top,
  BorderWidthRight => border_width_right,
  BorderWidthBottom => border_width_bottom,
  BorderWidthLeft => border_width_left,
  BorderRadiusTopLeft => border_radius_top_left,
  BorderRadiusTopRight => border_radius_top_right,
  BorderRadiusBottomRight => border_radius_bottom_right,
  BorderRadiusBottomLeft => border_radius_bottom_left,
  OffsetX => offset_x,
  OffsetY => offset_y,
  Width => width,
  Height => height,
  Opacity => opacity,
  Transform => transform,
});

const ALL_PROPERTIES: &[AnimatableProperty] = &[
  AnimatableProperty::BackgroundColor,
  AnimatableProperty::BorderColor,
  AnimatableProperty::BorderWidthTop,
  AnimatableProperty::BorderWidthRight,
  AnimatableProperty::BorderWidthBottom,
  AnimatableProperty::BorderWidthLeft,
  AnimatableProperty::BorderRadiusTopLeft,
  AnimatableProperty::BorderRadiusTopRight,
  AnimatableProperty::BorderRadiusBottomRight,
  AnimatableProperty::BorderRadiusBottomLeft,
  AnimatableProperty::OffsetX,
  AnimatableProperty::OffsetY,
  AnimatableProperty::Width,
  AnimatableProperty::Height,
  AnimatableProperty::Opacity,
  AnimatableProperty::Transform,
];

struct TransitionRun {
  from: AnimatableValue,
  to: AnimatableValue,
  start_time: Instant,
  duration: Duration,
  delay: Duration,
  easing: Easing,
}

pub struct TransitionEngine {
  active: HashMap<(NodeId, AnimatableProperty), TransitionRun>,
  prev_values: HashMap<(NodeId, AnimatableProperty), AnimatableValue>,
  pub has_active: bool,
}

impl TransitionEngine {
  pub fn new() -> Self {
    Self {
      active: HashMap::new(),
      prev_values: HashMap::new(),
      has_active: false,
    }
  }

  pub(crate) fn clear_state(&mut self) {
    self.active.clear();
    self.prev_values.clear();
    self.has_active = false;
  }

  pub(crate) fn tick(&mut self, root: &mut crate::node::Node, now: Instant) -> bool {
    clear_overrides(root);
    self.has_active = false;
    let needs_layout = self.tick_recursive(root, now);
    self.has_active = !self.active.is_empty();
    needs_layout
  }

  fn tick_recursive(&mut self, node: &mut crate::node::Node, now: Instant) -> bool {
    let node_id = node.node_id();
    let mut needs_layout = false;
    if node_id.is_assigned() && !node.transitions.is_empty() {
      needs_layout = self.process_node(node, now);
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

  fn process_node(&mut self, node: &mut crate::node::Node, now: Instant) -> bool {
    let node_id = node.node_id();
    let mut layout_affected = false;

    let properties: Vec<(AnimatableProperty, Duration, Duration, Easing)> = node
      .transitions
      .iter()
      .flat_map(|spec| {
        let props: &[AnimatableProperty] = match spec.property {
          TransitionProperty::All => ALL_PROPERTIES,
          TransitionProperty::Single(ref p) => std::slice::from_ref(p),
        };
        props.iter().map(move |p| (*p, spec.duration, spec.delay, spec.easing))
      })
      .collect();

    for (prop, duration, delay, easing) in &properties {
      let current = match read_target(node, *prop) {
        Some(v) => v,
        None => continue,
      };

      let key = (node_id, *prop);

      if let Some(prev) = self.prev_values.get(&key) {
        if *prev != current {
          let from = if let Some(running) = self.active.get(&key) {
            let elapsed = now.duration_since(running.start_time);
            if elapsed >= running.delay {
              let raw = (elapsed - running.delay).as_secs_f64() / running.duration.as_secs_f64();
              let t = running.easing.evaluate(raw.clamp(0.0, 1.0)) as f32;
              running.from.lerp(&running.to, t)
            } else {
              running.from
            }
          } else {
            *prev
          };

          self.active.insert(
            key,
            TransitionRun {
              from,
              to: current,
              start_time: now,
              duration: *duration,
              delay: *delay,
              easing: *easing,
            },
          );
        }
      }

      self.prev_values.insert(key, current);
    }

    let mut completed = Vec::new();
    for (&(nid, prop), run) in &self.active {
      if nid != node_id {
        continue;
      }
      let elapsed = now.duration_since(run.start_time);
      if elapsed < run.delay {
        layout_affected |= write_property(node, prop, &run.from);
        continue;
      }
      let raw = (elapsed - run.delay).as_secs_f64() / run.duration.as_secs_f64();
      if raw >= 1.0 {
        layout_affected |= write_property(node, prop, &run.to);
        completed.push((nid, prop));
      } else {
        let t = run.easing.evaluate(raw) as f32;
        let interpolated = run.from.lerp(&run.to, t);
        layout_affected |= write_property(node, prop, &interpolated);
      }
    }

    for key in completed {
      self.active.remove(&key);
    }

    layout_affected
  }
}
