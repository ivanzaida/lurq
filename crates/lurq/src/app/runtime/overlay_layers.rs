//! What building the overlay host's layers learns besides the layers: how a
//! press or Escape dismisses each overlay, which anchor each layer hangs
//! from, and which overlays are hidden because their anchor is scrolled or
//! clipped out of view.
//!
//! A hidden overlay stays open but is not drawn, so it is dismissed like a
//! CSS popover that is out of view: a left press anywhere closes it without
//! being consumed (the press also reaches what is under the pointer), Escape
//! closes it, and keyboard focus that was inside it goes back to its anchor
//! (the trigger), or else to the nearest focusable ancestor of the anchor.

use super::{FocusTarget, OverlayDismissEntry, Tree, find_node_by_path};
use crate::{
  app::ctx::OutsidePress,
  core::{ElementRect, ElementRef as OwnedElementRef, NodeId, Signal},
  node::Node,
};

/// How a hidden overlay or modal is dismissed: its signal-backed open state,
/// if any, and whether a press outside it or Escape closes it.
pub(super) struct HiddenDismiss {
  pub(super) open: Option<Signal<bool>>,
  pub(super) on_outside_click: bool,
  pub(super) on_escape: bool,
}

#[derive(Default)]
pub(super) struct OverlayLayers {
  pub(super) dismiss_entries: Vec<OverlayDismissEntry>,
  /// The anchor of each layer built, in the order the host stacks them;
  /// `None` for a select menu or a modal.
  pub(super) anchors: Vec<Option<OwnedElementRef>>,
  /// The node that declares each layer built, in the same order, where it
  /// has an id.
  pub(super) origins: Vec<Option<NodeId>>,
  /// What places each layer built by its anchor, in the same order; `None`
  /// for a select menu or a modal, which are placed otherwise.
  pub(super) placings: Vec<Option<super::OverlayPlacing>>,
  /// The anchors of the overlays not drawn because their anchor is out of view.
  hidden_anchors: Vec<OwnedElementRef>,
  /// The anchors of the open overlays that move the focus (`Overlay::move_focus`), drawn or hidden.
  pub(super) focus_movers: Vec<OwnedElementRef>,
  /// The nodes that declare the modals built that take the focus (`Modal::takes_focus`).
  pub(super) focus_modals: Vec<NodeId>,
}

impl OverlayLayers {
  /// Records an overlay or modal that is open but not drawn because its
  /// `anchor` (a modal's target element) is out of view. One that a press or
  /// Escape dismisses keeps a dismiss entry: every press is outside it and
  /// passes through, and Escape closes it.
  pub(super) fn hide(&mut self, anchor: &OwnedElementRef, dismiss: HiddenDismiss, layer: usize) {
    self.hidden_anchors.push(anchor.clone());
    let Some(open) = dismiss.open else {
      return;
    };
    if !dismiss.on_outside_click && !dismiss.on_escape {
      return;
    }
    self.dismiss_entries.push(OverlayDismissEntry {
      anchor: anchor.clone(),
      bounds: ElementRect::default(),
      open,
      dismiss_on_outside_click: dismiss.on_outside_click,
      outside_press: OutsidePress::PassThrough,
      dismiss_on_escape: dismiss.on_escape,
      layer,
      shown: false,
    });
  }

  pub(super) fn is_hidden(&self, anchor: &OwnedElementRef) -> bool {
    self.hidden_anchors.iter().any(|hidden| hidden.same_handle(anchor))
  }
}

impl Tree {
  /// The anchor of the overlay layer that held keyboard focus in the last
  /// layout, when `layers` hides that overlay now. `had_host` tells whether
  /// the last root was the overlay host, whose children the focused path
  /// starts in.
  pub(super) fn focused_overlay_hidden(&self, had_host: bool, layers: &OverlayLayers) -> Option<OwnedElementRef> {
    if !had_host {
      return None;
    }
    let layer = *self.focused_path.as_deref()?.first()?;
    let anchor = self.overlay_layer_anchors.get(layer.checked_sub(1)?)?.as_ref()?;
    layers.is_hidden(anchor).then(|| anchor.clone())
  }

  /// The anchor of the overlay that moves the focus (`Overlay::move_focus`) and held keyboard focus in the
  /// last layout, when it is not open in `layers` any more: the focus goes back to it.
  pub(super) fn focused_overlay_closed(&self, had_host: bool, layers: &OverlayLayers) -> Option<OwnedElementRef> {
    if !had_host {
      return None;
    }
    let layer = *self.focused_path.as_deref()?.first()?;
    let anchor = self.overlay_layer_anchors.get(layer.checked_sub(1)?)?.as_ref()?;
    let moved = self.overlay_focus_movers.iter().any(|mover| mover.same_handle(anchor));
    let open = layers.focus_movers.iter().any(|mover| mover.same_handle(anchor));
    (moved && !open).then(|| anchor.clone())
  }

  /// The layers of the overlay host whose overlays move the focus, each with its anchor: what Tab walks right
  /// after the anchor.
  pub(super) fn focus_moving_layers(&self) -> Vec<(OwnedElementRef, &Node)> {
    let Some(host) = self.root.as_ref() else {
      return Vec::new();
    };
    self
      .overlay_layer_anchors
      .iter()
      .enumerate()
      .filter_map(|(index, anchor)| {
        let anchor = anchor.as_ref()?;
        self
          .overlay_focus_movers
          .iter()
          .any(|mover| mover.same_handle(anchor))
          .then(|| Some((anchor.clone(), host.children().get(index + 1)?)))?
      })
      .collect()
  }

  /// The first element that takes the focus in the topmost layer of the overlay host whose overlay moves the
  /// focus and opened in this pass: where the focus goes. `layers` are the layers as the host stacks them.
  pub(super) fn opened_overlay_focus(&self, layers: &OverlayLayers) -> Option<NodeId> {
    let host = self.root.as_ref()?;
    layers
      .focus_movers
      .iter()
      .filter(|mover| !self.overlay_focus_movers.iter().any(|old| old.same_handle(mover)))
      .filter_map(|mover| {
        layers
          .anchors
          .iter()
          .position(|anchor| anchor.as_ref().is_some_and(|anchor| anchor.same_handle(mover)))
      })
      .max()
      .and_then(|layer| host.children().get(layer + 1))
      .and_then(first_focusable)
  }

  /// Gives keyboard focus, lost with a hidden overlay, back to the overlay's
  /// `anchor`: the anchor or the first focusable node inside it, else the
  /// nearest focusable ancestor. Does nothing if focus is somewhere already.
  pub(super) fn return_focus_to_anchor(&mut self, anchor: &OwnedElementRef) {
    if self.focused_node.is_some() {
      return;
    }
    let Some(target) = self.root.as_ref().and_then(|root| focus_home(root, anchor)) else {
      return;
    };
    self.focus_node(FocusTarget {
      input_id: target,
      event_id: target,
    });
  }
}

fn focus_home(root: &Node, anchor: &OwnedElementRef) -> Option<NodeId> {
  let path = path_to_ref(root, anchor)?;
  let anchor_node = find_node_by_path(root, &path)?;
  if let Some(trigger) = first_focusable(anchor_node) {
    return Some(trigger);
  }
  (0..path.len())
    .rev()
    .filter_map(|depth| find_node_by_path(root, &path[..depth]))
    .find(|node| takes_focus(node))
    .map(Node::node_id)
}

fn takes_focus(node: &Node) -> bool {
  node.is_focusable() && !node.is_focus_disabled()
}

fn first_focusable(node: &Node) -> Option<NodeId> {
  if takes_focus(node) {
    return Some(node.node_id());
  }
  node.children().iter().find_map(first_focusable)
}

fn path_to_ref(node: &Node, anchor: &OwnedElementRef) -> Option<Vec<usize>> {
  if node
    .element_ref
    .as_ref()
    .is_some_and(|current| current.same_handle(anchor))
  {
    return Some(Vec::new());
  }
  node.children().iter().enumerate().find_map(|(index, child)| {
    let mut path = path_to_ref(child, anchor)?;
    path.insert(0, index);
    Some(path)
  })
}
