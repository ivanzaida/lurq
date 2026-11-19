//! Layers that closed and are still on screen while their exit animations
//! play (`node/presence.rs`).
//!
//! The overlay host is built again in every pass from the layers the layout
//! declares, and a layer no declaration continues is dropped. One with an
//! exit animation in it is kept instead, as it was when it closed: the host
//! holds its nodes at the place the layer had among the others, the
//! animations play on them, and the pass that comes when the longest has
//! ended drops it. The app does not render it any more, so it shows what it
//! showed when it closed.
//!
//! A leaving layer is only painted. Its root takes no pointer and has the
//! `LeavingLayer` role, so it is no modal and Tab passes it by; focus, hover
//! and a press that were in it are let go when it begins to leave; its nodes
//! lose their element ids, element refs and component slots, so no lookup finds them
//! and a component that renders again leaves them alone. A layer opened
//! again while it leaves (the same key, or else declared by the same node)
//! takes it back: it ends its leaving, takes back its names and is continued
//! by the layer built, its ids, its focus, its scroll and what was typed in it
//! kept, and what it enters with goes on from where its exit had got to
//! (`OverlayHostReuse::claim`). There is one of it, never one entering over
//! one leaving.
//!
//! A layer hidden because its anchor went out of view does not leave: it is
//! still open, and comes back with its anchor.
//!
//! **Children.** A child removed from a container with presence leaves the
//! same way, where it was among its siblings and in flow. Reconciling a
//! render with the tree keeps its node (`Node::take_leaving_children`) and
//! notes it; the tree then starts its exit animations and makes it something
//! it only paints, as a layer, and removes it in the first pass after the
//! longest of them ends.

use super::{
  FocusTarget, OverlayHostReuse, Tree, find_node_by_id, find_node_by_id_mut, overlay_layers::OverlayLayers,
  reset_element_ref_flags_recursive,
};
use crate::{core::NodeId, node::Node, time::Instant};

/// A layer the overlay host keeps while its exit animations play.
pub(super) struct LeavingLayer {
  /// The id of the layer's root, a child of the overlay host.
  pub(super) root: NodeId,
  /// When the longest of its exit animations ends and the layer is dropped.
  pub(super) until: Instant,
  /// The node that declared the layer: a layer it declares again while this one leaves takes this one back.
  pub(super) origin: Option<NodeId>,
  /// What had the focus in it as it began to leave: it has it again if the layer is taken back.
  pub(super) focus: Option<FocusTarget>,
}

impl Tree {
  /// The layers the host draws on their way out in the pass at `now`, each
  /// with the place it had among the old host's layers, in that order: the
  /// ones that were leaving and still are, and the old layers no layer of
  /// this pass continues that have an exit animation, which begin to leave.
  /// The layers whose time is up are left in `old_parts` to be dropped.
  pub(super) fn leaving_layers(
    &mut self,
    old_parts: &mut OverlayHostReuse,
    layers: &OverlayLayers,
    old_origins: &[Option<NodeId>],
    now: Instant,
  ) -> Vec<(usize, Node)> {
    let mut records = Vec::new();
    let mut leaving = Vec::new();
    for (position, layer) in std::mem::take(&mut old_parts.leaving) {
      let record = self.leaving_layers.iter().find(|record| record.root == layer.node_id());
      match record {
        Some(record) if now < record.until => {
          records.push(LeavingLayer {
            root: record.root,
            until: record.until,
            origin: record.origin,
            focus: record.focus,
          });
          leaving.push((position, layer));
        }
        _ => old_parts.dropped.push(layer),
      }
    }

    for index in 0..old_parts.old_overlays.len() {
      if !old_parts.is_free(index) || !old_parts.old_overlays[index].has_exit_animation() {
        continue;
      }
      let position = old_parts.old_overlay_positions[index];
      let origin = old_parts.old_origin(index, old_origins);
      let hidden = self
        .overlay_layer_anchors
        .get(position)
        .and_then(Option::as_ref)
        .is_some_and(|anchor| layers.is_hidden(anchor));
      if hidden {
        continue;
      }
      let layer = &mut old_parts.old_overlays[index];
      let animations = &mut self.animation_engine;
      let until = layer
        .begin_exit(&mut |node| animations.forget_node(node))
        .and_then(|lasts| now.checked_add(lasts));
      let Some(until) = until else {
        continue;
      };
      let mut layer = std::mem::take(layer);
      old_parts.old_overlay_used[index] = true;
      reset_element_ref_flags_recursive(&layer);
      layer.turn_into_leaving_layer();
      records.push(LeavingLayer {
        root: layer.node_id(),
        until,
        origin,
        focus: None,
      });
      self.layers_left.push(layer.node_id());
      leaving.push((position, layer));
    }

    self.leaving_layers = records;
    leaving.sort_by_key(|(position, _)| *position);
    leaving
  }

  /// The leaving layers, each by its root, with the node that declared it: what a layer declared again takes back.
  pub(super) fn leaving_origins(&self) -> Vec<(NodeId, Option<NodeId>)> {
    self
      .leaving_layers
      .iter()
      .map(|layer| (layer.root, layer.origin))
      .collect()
  }

  /// Whether a leaving layer's time is up at `now`: the pass drops it.
  pub(super) fn leaving_layer_due(&self, now: Instant) -> bool {
    self.leaving_layers.iter().any(|layer| now >= layer.until)
  }

  /// Starts the exit animations of the children that reconciling a render
  /// with the tree just kept, and makes each something the tree only paints.
  /// One whose animations take no time is removed at once.
  pub(super) fn begin_leaving_children(&mut self) {
    // A child that came back while it left went with the tree the render replaced: its time is no one's to wait
    // for, and its id may be another node's by the time it would have been up.
    // A child that came back while it left is continued by the one that came back, under its ids: its time is no
    // one's to wait for, and what it enters with goes on from where its exit had got to.
    let ended = crate::node::presence::take_ended_leaving();
    if !ended.is_empty() {
      let focus_back = self
        .leaving_children
        .iter()
        .filter(|child| ended.contains(&child.root))
        .find_map(|child| child.focus);
      self.leaving_children.retain(|child| !ended.contains(&child.root));
      if let Some(target) = focus_back {
        self.focus_node(target);
      }
      let now = self.now();
      let mut ids = Vec::new();
      if let Some(root) = self.root.as_ref() {
        for id in &ended {
          if let Some(node) = find_node_by_id(root, *id) {
            node.subtree_ids(&mut ids);
          }
        }
      }
      self.animation_engine.turn_round(&ids, now);
    }
    let began = crate::node::presence::take_began_leaving();
    if began.is_empty() {
      return;
    }
    let now = self.now();
    let Some(root) = self.root.as_mut() else {
      return;
    };
    for id in began {
      // A child whose container was itself replaced went with it.
      let Some(node) = find_node_by_id_mut(root, id) else {
        continue;
      };
      let animations = &mut self.animation_engine;
      let until = node
        .begin_exit(&mut |node| animations.forget_node(node))
        .and_then(|lasts| now.checked_add(lasts));
      let Some(until) = until else {
        if let Some(mut removed) = remove_child_by_id(root, id) {
          removed.free_ids(&self.id_gen);
        }
        continue;
      };
      reset_element_ref_flags_recursive(node);
      node.turn_into_leaving_layer();
      self.leaving_children.push(LeavingLayer {
        root: id,
        until,
        origin: None,
        focus: None,
      });
      self.layers_left.push(id);
    }
    self.cached_render_list = None;
    self.needs_redraw = true;
  }

  /// Removes the leaving children whose time is up at `now` from the tree,
  /// which lays out again without them.
  pub(super) fn drop_leaving_children(&mut self, now: Instant) {
    if !self.leaving_children.iter().any(|child| now >= child.until) {
      return;
    }
    let (due, leaving): (Vec<_>, Vec<_>) = std::mem::take(&mut self.leaving_children)
      .into_iter()
      .partition(|child| now >= child.until);
    self.leaving_children = leaving;
    let mut removed_any = false;
    for child in due {
      // One whose container is gone went with it.
      let removed = self.root.as_mut().and_then(|root| remove_child_by_id(root, child.root));
      let Some(mut removed) = removed else {
        continue;
      };
      #[cfg(feature = "canvas")]
      super::detach_canvases_not_carried(super::canvas_bindings(&removed), self.root.as_ref());
      removed.free_ids(&self.id_gen);
      removed_any = true;
    }
    if removed_any {
      self.tree_rebuilt_since_layout = true;
      self.cached_render_list = None;
      self.needs_redraw = true;
      // Paths into the tree count children, and the ones after a removed child moved up.
      self.reapply_interaction_state();
    }
  }

  /// Lets go of the focus, the hover and the press that are in a layer or a
  /// child that began to leave: what is only painted holds none of them.
  pub(super) fn release_input_in_leaving_layers(&mut self) {
    let left = std::mem::take(&mut self.layers_left);
    if left.is_empty() {
      return;
    }
    let Some(root) = self.root.as_ref() else {
      return;
    };
    let inside = |id: &NodeId| {
      left
        .iter()
        .filter_map(|leaving| find_node_by_id(root, *leaving))
        .any(|leaving| find_node_by_id(leaving, *id).is_some())
    };
    let loses_focus = self.focused_node.iter().chain(&self.focused_event_node).any(inside);
    // What had the focus is remembered by the layer it was in, for the layer to give it back if it is taken back.
    if loses_focus && let Some(input_id) = self.focused_node {
      let target = FocusTarget {
        input_id,
        event_id: self.focused_event_node.unwrap_or(input_id),
      };
      let holder = left.iter().copied().find(|leaving| {
        find_node_by_id(root, *leaving).is_some_and(|layer| find_node_by_id(layer, input_id).is_some())
      });
      for record in self.leaving_layers.iter_mut().chain(self.leaving_children.iter_mut()) {
        if Some(record.root) == holder {
          record.focus = Some(target);
        }
      }
    }
    let hover_path = self.hover_path.iter().copied().filter(|id| !inside(id)).collect();
    let active_path = self.active_path.iter().copied().filter(|id| !inside(id)).collect();
    self.hover_path = hover_path;
    self.active_path = active_path;
    if loses_focus {
      self.blur_focus();
    }
  }
}

/// The host's layers in the order it stacks them: `overlays`, the open ones,
/// with each of `leaving` back at the place it had among the old host's
/// layers, or on top when fewer layers are left. What `layers` knows of each
/// open layer by its place (its anchor, the node that declares it, which
/// layer a press or Escape dismisses) follows it to its new place.
pub(super) fn stack_layers(overlays: Vec<Node>, leaving: Vec<(usize, Node)>, layers: &mut OverlayLayers) -> Vec<Node> {
  if leaving.is_empty() {
    return overlays;
  }
  let leaving_count = leaving.len();
  let mut stack = Vec::with_capacity(overlays.len() + leaving_count);
  let mut anchors = Vec::with_capacity(stack.capacity());
  let mut origins = Vec::with_capacity(stack.capacity());
  let mut takes_focus = Vec::with_capacity(stack.capacity());
  // Where each open layer is in the stack.
  let mut places = Vec::with_capacity(overlays.len());
  let mut leaving = leaving.into_iter().peekable();
  let mut open = overlays
    .into_iter()
    .zip(std::mem::take(&mut layers.anchors))
    .zip(std::mem::take(&mut layers.origins))
    .zip(std::mem::take(&mut layers.takes_focus))
    .peekable();
  loop {
    let leaving_is_next = match (leaving.peek(), open.peek()) {
      (Some((position, _)), Some(_)) => *position <= stack.len(),
      (Some(_), None) => true,
      (None, Some(_)) => false,
      (None, None) => break,
    };
    if leaving_is_next {
      let (_, layer) = leaving.next().expect("peeked");
      stack.push(layer);
      anchors.push(None);
      origins.push(None);
      takes_focus.push(false);
    } else {
      let (((layer, anchor), origin), takes) = open.next().expect("peeked");
      places.push(stack.len());
      stack.push(layer);
      anchors.push(anchor);
      origins.push(origin);
      takes_focus.push(takes);
    }
  }
  layers.anchors = anchors;
  layers.origins = origins;
  layers.takes_focus = takes_focus;
  // An entry's layer counts the host's children, the page first. A hidden layer's is where it would be.
  for entry in &mut layers.dismiss_entries {
    entry.layer = match entry.layer.checked_sub(1).and_then(|place| places.get(place)) {
      Some(place) => place + 1,
      None => entry.layer + leaving_count,
    };
  }
  stack
}

/// Takes the node `id` out of the children of the node it is a child of, somewhere under `node`, whose
/// layout is no longer the cached one.
fn remove_child_by_id(node: &mut Node, id: NodeId) -> Option<Node> {
  if let Some(index) = node.children.iter().position(|child| child.node_id() == id) {
    node.layout_cache.invalidate();
    return Some(node.children.remove(index));
  }
  node.children.iter_mut().find_map(|child| remove_child_by_id(child, id))
}
