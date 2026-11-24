use std::{
  collections::{HashMap, HashSet},
  sync::Arc,
  time::Duration,
};

use super::{
  animation::step_time,
  easing::Easing,
  interpolate::{
    AnimatableProperty, AnimatableValue, TargetTheme, clear_overrides, property_accessors, read_target, write_property,
  },
};
use crate::{app::theme::Theme, core::NodeId, time::Instant};

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
  Blur => blur,
  BackdropBlur => backdrop_blur,
  Glass => glass,
  Transform => transform,
  Foreground => foreground,
  Outline => outline,
  BoxShadow => box_shadow,
  BorderDash => border_dash,
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
  AnimatableProperty::Blur,
  AnimatableProperty::BackdropBlur,
  AnimatableProperty::Glass,
  AnimatableProperty::Transform,
  AnimatableProperty::Foreground,
  AnimatableProperty::Outline,
  AnimatableProperty::BoxShadow,
  AnimatableProperty::BorderDash,
];

/// Whether animating `prop` moves an element or changes its size on screen.
fn moves(prop: AnimatableProperty) -> bool {
  matches!(
    prop,
    AnimatableProperty::OffsetX
      | AnimatableProperty::OffsetY
      | AnimatableProperty::Width
      | AnimatableProperty::Height
      | AnimatableProperty::Transform
      // A dash running along an outline moves: a ring's arc going round.
      | AnimatableProperty::BorderDash
  )
}

struct TransitionRun {
  from: AnimatableValue,
  to: AnimatableValue,
  start_time: Instant,
  duration: Duration,
  delay: Duration,
  easing: Easing,
}

impl TransitionRun {
  /// How far the run is `elapsed` after its delay, from 0 at its start to 1 at its end. A run of no duration is at
  /// its end at once.
  fn progress(&self, elapsed: Duration) -> f64 {
    if self.duration.is_zero() {
      return 1.0;
    }
    elapsed.as_secs_f64() / self.duration.as_secs_f64()
  }

  /// When the run next draws something other than what it draws at `now`, which is `raw` of the way through it:
  /// the next step of a step easing or the end. A continuous easing changes at once, so it returns `now`.
  fn next_change(&self, now: Instant, raw: f64) -> Option<Instant> {
    match self.easing.distance_to_next_step(raw, true) {
      Some(until) => step_time(now, self.duration, until),
      None => Some(now),
    }
  }
}

pub struct TransitionEngine {
  active: HashMap<(NodeId, AnimatableProperty), TransitionRun>,
  prev_values: HashMap<(NodeId, AnimatableProperty), AnimatableValue>,
  /// The theme each node's values were last read in (`Theme::identity`): a node built in another theme since (a
  /// stage switched to the other appearance) heads to that theme's values at once, as a change of theme.
  node_themes: HashMap<NodeId, usize>,
  /// The pass each node was first ticked in: what it has in the frame it comes in (a fill its settled layout gives
  /// it, after a first pass gave it none) is there at once, as its first value is.
  born: HashMap<NodeId, u64>,
  /// The passes begun: see [`Self::begin_pass`].
  pass: u64,
  /// The theme the node being ticked is read in: a theme an element of the tree carries, or none (0) for the
  /// window's, whose changes `theme_changed` follows by their values.
  theme_identity: usize,
  seen: HashSet<NodeId>,
  next_change: Option<Instant>,
  pub has_active: bool,
  /// The theme the values nodes head to are read against, and the theme and version it was copied from.
  theme: TargetTheme,
  theme_seen: Option<(Theme, u64)>,
  /// Whether the colors, widths or shadows of the theme are other ones than in the frame before. What a node
  /// heads to then changed because the theme did, not because the node did: see [`Self::process_node`].
  theme_changed: bool,
  /// The themes that elements of the tree carry, each with the values last read from it: a subtree is read
  /// against its own theme, and a change of that theme is a change of theme for the subtree.
  scopes: Vec<(Theme, TargetTheme)>,
  /// Whether motion is reduced (`Theme::reduced_motion`): what moves or resizes an element then ends at once.
  reduced_motion: bool,
  /// Whether a fade ends at once too while motion is reduced (`Theme::reduce_fades`).
  reduce_fades: bool,
  /// Which values this engine holds: a subtree it ticked clean is skipped only while it holds the values it had.
  epoch: u64,
  /// What the walk of the children of the node last ticked found, child by child: see [`Self::tick_subtree`].
  child_results: Vec<Option<Option<Arc<crate::node::layout_cache::TickedIds>>>>,
}

/// A new epoch: for a new engine, and for one that forgot what it held.
fn next_epoch() -> u64 {
  static EPOCH: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
  EPOCH.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

impl Default for TransitionEngine {
  fn default() -> Self {
    Self::new()
  }
}

impl TransitionEngine {
  pub fn new() -> Self {
    Self {
      active: HashMap::new(),
      prev_values: HashMap::new(),
      node_themes: HashMap::new(),
      born: HashMap::new(),
      pass: 0,
      theme_identity: 0,
      seen: HashSet::new(),
      next_change: None,
      has_active: false,
      theme: TargetTheme::default(),
      theme_seen: None,
      theme_changed: false,
      scopes: Vec::new(),
      reduced_motion: false,
      reduce_fades: false,
      epoch: next_epoch(),
      child_results: Vec::new(),
    }
  }

  /// Takes the palette, border sizes and shadows of `theme`, which colors, widths and shadows given as roles
  /// are read against. Copies nothing while the theme and its version are the ones last taken.
  pub(crate) fn set_theme(&mut self, theme: &Theme) {
    self.reduced_motion = theme.reduced_motion();
    self.reduce_fades = theme.reduce_fades();
    let version = theme.version();
    if self
      .theme_seen
      .as_ref()
      .is_some_and(|(seen, seen_version)| *seen_version == version && seen.same_theme(theme))
    {
      return;
    }
    let taken = TargetTheme {
      palette: theme.palette().clone(),
      border_sizes: theme.border_sizes().clone(),
      shadows: theme.shared_shadows(),
    };
    // What counts is what a role resolves to: a theme at another version with the same colors, widths
    // and shadows, or another theme that has them, changes no value a node heads to. The first theme
    // taken is no change either.
    self.theme_changed = self.theme_seen.is_some()
      && (taken.palette != self.theme.palette
        || taken.border_sizes != self.theme.border_sizes
        || taken.shadows != self.theme.shadows);
    self.theme = taken;
    self.theme_seen = Some((theme.clone(), version));
  }

  pub(crate) fn clear_state(&mut self) {
    self.epoch = next_epoch();
    self.active.clear();
    self.prev_values.clear();
    self.node_themes.clear();
    self.born.clear();
    self.seen.clear();
    self.next_change = None;
    self.has_active = false;
  }

  /// Starts a frame: forgets which transitioning nodes last frame's walks
  /// visited. Mirrors `AnimationEngine::begin_frame` — a frame ticks the base
  /// tree and each overlay subtree separately, so stale runs are only
  /// detectable after all walks in [`Self::finish_frame`].
  /// Starts a pass of the window, whose layouts (several, where what it lays out settles) are one frame: a node
  /// that comes in during it has what it has in them at once.
  pub(crate) fn begin_pass(&mut self) {
    self.pass += 1;
  }

  pub(crate) fn begin_frame(&mut self) {
    self.seen.clear();
    self.next_change = None;
    self.theme_changed = false;
  }

  /// Ends a frame after every tick walk ran: drops runs and prev-value
  /// tracking for nodes no walk visited. A node unmounting mid-transition
  /// otherwise leaves its run in `active` forever (only completed runs are
  /// removed in `process_node`), wedging `has_active` and with it the layout
  /// fast path.
  pub(crate) fn finish_frame(&mut self) {
    let seen = &self.seen;
    self.active.retain(|(node_id, _), _| seen.contains(node_id));
    self.prev_values.retain(|(node_id, _), _| seen.contains(node_id));
    self.node_themes.retain(|node_id, _| seen.contains(node_id));
    self.born.retain(|node_id, _| seen.contains(node_id));
    self.has_active = !self.active.is_empty();
  }

  /// The earliest time a run ticked this frame draws something else; see [`TransitionRun::next_change`].
  pub(crate) fn next_change(&self) -> Option<Instant> {
    self.next_change
  }

  pub(crate) fn tick(&mut self, root: &mut crate::node::Node, now: Instant) -> bool {
    self.theme_identity = 0;
    clear_overrides(root);
    self.has_active = false;
    let needs_layout = self.tick_root(root, now);
    self.has_active = !self.active.is_empty();
    needs_layout
  }

  pub(crate) fn tick_preserving_active_state(&mut self, root: &mut crate::node::Node, now: Instant) -> bool {
    self.theme_identity = 0;
    clear_overrides(root);
    let needs_layout = self.tick_root(root, now);
    self.has_active = !self.active.is_empty();
    needs_layout
  }

  /// Ticks the tree under `root`.
  fn tick_root(&mut self, root: &mut crate::node::Node, now: Instant) -> bool {
    self.tick_subtree(root, now).0
  }

  /// Ticks the transitions under `node`. Returns whether the layout changed, and whether nothing runs under it
  /// any more and no animation plays there (the subtree is then ticked clean) with the nodes under it that have
  /// transitions. A clean subtree keeps that on its node, as it is then: one unchanged since (its generation is the
  /// one it was ticked at, no state of interaction and no theme changed anywhere, the engine holds what it held) has
  /// nothing to tick: its values are where they were heading, and it is skipped, its nodes still seen so that what
  /// they head to stays known. A frame that scrolls a long document walks none of it.
  fn tick_subtree(
    &mut self,
    node: &mut crate::node::Node,
    now: Instant,
  ) -> (bool, Option<Option<Arc<crate::node::layout_cache::TickedIds>>>) {
    if !node.prepare_layout_tracking().timeline {
      return (false, Some(None));
    }
    let ticked = node.layout_cache.take_ticked();
    let unchanged = ticked.as_ref().is_some_and(|ticked| {
      !self.theme_changed && ticked.epoch == self.epoch && ticked.is_current(node.layout_cache.watch())
    });
    if let (true, Some(ticked)) = (unchanged, ticked) {
      let ids = ticked.ids.clone();
      if let Some(ids) = &ids {
        let seen = &mut self.seen;
        ids.each(&mut |id| {
          seen.insert(id);
        });
      }
      node.layout_cache.set_ticked(Some(ticked));
      return (false, Some(ids));
    }
    let needs_layout = self.tick_recursive(node, now);
    let node_id = node.node_id();
    let children = std::mem::take(&mut self.child_results);
    let clean = !needs_layout
      && node.playing_animation().is_none()
      && node.animation_overrides.is_empty()
      && !self.active.keys().any(|(id, _)| *id == node_id)
      && children.len() == node.children.len()
      && children.iter().all(Option::is_some);
    if !clean {
      return (needs_layout, None);
    }
    let own = (node_id.is_assigned() && !node.transitions.is_empty()).then_some(node_id);
    let children: Vec<_> = children.into_iter().flatten().flatten().collect();
    let ids =
      (own.is_some() || !children.is_empty()).then(|| Arc::new(crate::node::layout_cache::TickedIds { own, children }));
    node
      .layout_cache
      .set_ticked(Some(Box::new(crate::node::layout_cache::TickedClean {
        generation: node.layout_cache.watch().generation(),
        ticks: node.layout_cache.watch().ticks(),
        epoch: self.epoch,
        world: crate::node::layout_cache::world_changes(),
        ids: ids.clone(),
      })));
    (needs_layout, Some(ids))
  }

  fn tick_recursive(&mut self, node: &mut crate::node::Node, now: Instant) -> bool {
    #[cfg(test)]
    crate::core::layout_dirty::visit();
    let scope = node.theme_scope.as_deref().cloned().map(|theme| {
      let outer = std::mem::replace(&mut self.theme_identity, theme.identity());
      (self.enter_scope(&theme), outer)
    });
    let node_id = node.node_id();
    let mut needs_layout = false;
    if node_id.is_assigned() && !node.transitions.is_empty() {
      self.seen.insert(node_id);
      needs_layout = self.process_node(node, now);
    }

    let mut results = Vec::with_capacity(node.children.len());
    for child in &mut node.children {
      let (child_layout, clean) = self.tick_subtree(child, now);
      needs_layout |= child_layout;
      results.push(clean);
    }
    self.child_results = results;

    if needs_layout {
      node.layout_cache.invalidate();
    }
    if let Some(((theme, changed), outer)) = scope {
      self.theme = theme;
      self.theme_changed = changed;
      self.theme_identity = outer;
    }

    needs_layout
  }

  /// Enters the subtree of an element that carries `theme`: what its nodes head to is read against that
  /// theme, and a change of its colors, widths or shadows since the last frame is a change of theme for
  /// them. Returns the theme and the flag to restore after the subtree.
  fn enter_scope(&mut self, theme: &Theme) -> (TargetTheme, bool) {
    let taken = TargetTheme {
      palette: theme.palette().clone(),
      border_sizes: theme.border_sizes().clone(),
      shadows: theme.shared_shadows(),
    };
    let same = |a: &TargetTheme, b: &TargetTheme| {
      a.palette == b.palette && a.border_sizes == b.border_sizes && a.shadows == b.shadows
    };
    let changed = match self.scopes.iter_mut().find(|(seen, _)| seen.same_theme(theme)) {
      Some((_, values)) => {
        let changed = !same(values, &taken);
        *values = taken.clone();
        changed
      }
      None => {
        self.scopes.push((theme.clone(), taken.clone()));
        false
      }
    };
    let outer = (std::mem::replace(&mut self.theme, taken), self.theme_changed);
    self.theme_changed = changed;
    outer
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

    // Read in another theme than last time: what changed with it is there at once.
    let moved = self
      .node_themes
      .insert(node_id, self.theme_identity)
      .is_some_and(|before| before != self.theme_identity);
    // In the frame the node comes in, what it has is there at once: the passes of that frame settle it.
    let newborn = *self.born.entry(node_id).or_insert(self.pass) == self.pass;
    for (prop, duration, delay, easing) in &properties {
      let key = (node_id, *prop);
      let Some(current) = read_target(node, *prop, &self.theme) else {
        // The node has no value of the property any more: a fill that only its hovered style gave it, after
        // the pointer left. Nothing is there to head to, so a run on its way ends here and the value it was
        // heading to is forgotten. Left running, it went on to that value and drew it on a node that no
        // longer has it (a row faded to its hovered fill after the pointer had gone), and the value kept as
        // the last one seen started the next run from a color the node had long lost.
        if self.prev_values.remove(&key).is_some() {
          self.active.remove(&key);
        }
        continue;
      };

      // A value that is another one because the theme is, a color of the other appearance, is there at
      // once, with the rest of the window: only what an element does itself is a transition. A window
      // that turned dark with its buttons still fading from light showed them as light blocks on it.
      // What a node comes in with, in the passes of the frame it comes in, is there at once: those passes settle
      // it (a form that turns, a fill its settled layout gives it), and nothing was drawn of it before.
      if (self.theme_changed || moved || newborn) && self.prev_values.get(&key).is_some_and(|prev| *prev != current) {
        self.active.remove(&key);
        self.prev_values.insert(key, current);
        continue;
      }

      if let Some(prev) = self.prev_values.get(&key)
        && *prev != current
      {
        let from = if let Some(running) = self.active.get(&key) {
          let elapsed = now.duration_since(running.start_time);
          if elapsed >= running.delay {
            let raw = running.progress(elapsed - running.delay);
            let t = running.easing.evaluate(raw.clamp(0.0, 1.0)) as f32;
            running.from.lerp(&running.to, t)
          } else {
            running.from
          }
        } else {
          *prev
        };

        // With motion reduced, what moves or resizes an element is there at once. A fade or a
        // change of color is no motion and still runs, unless the app has fades end at once too.
        let (duration, delay) = if self.reduced_motion && (self.reduce_fades || moves(*prop)) {
          (Duration::ZERO, Duration::ZERO)
        } else {
          (*duration, *delay)
        };
        self.active.insert(
          key,
          TransitionRun {
            from,
            to: current,
            start_time: now,
            duration,
            delay,
            easing: *easing,
          },
        );
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
        let at = run.start_time + run.delay;
        self.next_change = Some(self.next_change.map_or(at, |next| next.min(at)));
        continue;
      }
      let raw = run.progress(elapsed - run.delay);
      if raw >= 1.0 {
        layout_affected |= write_property(node, prop, &run.to);
        completed.push((nid, prop));
      } else {
        let t = run.easing.evaluate(raw) as f32;
        let interpolated = run.from.lerp(&run.to, t);
        layout_affected |= write_property(node, prop, &interpolated);
        if let Some(at) = run.next_change(now, raw) {
          self.next_change = Some(self.next_change.map_or(at, |next| next.min(at)));
        }
      }
    }

    for key in completed {
      self.active.remove(&key);
    }

    layout_affected
  }
}
