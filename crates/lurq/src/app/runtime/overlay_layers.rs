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
  /// What runs after a press outside it or Escape closed it.
  pub(super) on_dismiss: Option<crate::app::ctx::OverlayDismiss>,
}

#[derive(Default)]
pub(super) struct OverlayLayers {
  pub(super) dismiss_entries: Vec<OverlayDismissEntry>,
  /// The anchor of each layer built, in the order the host stacks them;
  /// `None` for a select menu or a modal.
  pub(super) anchors: Vec<Option<OwnedElementRef>>,
  /// Whether each layer built, in the same order, is an overlay that takes the focus (`Overlay::takes_focus`):
  /// what tells it from another layer on the same anchor, a tooltip's beside a menu's.
  pub(super) takes_focus: Vec<bool>,
  /// The node that declares each layer built, in the same order, where it
  /// has an id.
  pub(super) origins: Vec<Option<NodeId>>,
  /// What places each layer built by its anchor, in the same order; `None`
  /// for a select menu or a modal, which are placed otherwise.
  pub(super) placings: Vec<Option<super::OverlayPlacing>>,
  /// The anchors of the overlays not drawn because their anchor is out of view.
  hidden_anchors: Vec<OwnedElementRef>,
  /// The anchors of the open overlays that take the focus (`Overlay::takes_focus`) or that Tab reaches after
  /// their anchor (`Overlay::tab_after_anchor`), drawn or hidden.
  pub(super) focus_movers: Vec<OwnedElementRef>,
  /// Those of `focus_movers` that move the focus into them as they open (`Overlay::takes_focus`).
  pub(super) focus_takers: Vec<OwnedElementRef>,
  /// Those of `focus_movers` that hold Tab while the focus is in them (`Overlay::traps_tab`).
  pub(super) tab_traps: Vec<OwnedElementRef>,
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
      on_dismiss: dismiss.on_dismiss,
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

  /// The anchor that the overlay which moves the focus and is anchored to `anchor` in `layers` had in the last
  /// layout, where it had another: the same node declares it, and what it was anchored to then anchors nothing
  /// that moves the focus now. A tour's card, which goes from one element to the next, is one overlay: it took
  /// the focus as it first opened, and gives it back to what had it then.
  pub(super) fn anchor_before(&self, anchor: &OwnedElementRef, layers: &OverlayLayers) -> Option<&OwnedElementRef> {
    let index = layers
      .anchors
      .iter()
      .position(|now| now.as_ref().is_some_and(|now| now.same_handle(anchor)))?;
    let origin = (*layers.origins.get(index)?)?;
    let old = self
      .overlay_layer_origins
      .iter()
      .position(|declared| *declared == Some(origin))?;
    let before = self.overlay_layer_anchors.get(old)?.as_ref()?;
    let moved = self.overlay_focus_movers.iter().any(|mover| mover.same_handle(before));
    let open = layers.focus_movers.iter().any(|mover| mover.same_handle(before));
    (moved && !open && !before.same_handle(anchor)).then_some(before)
  }

  /// The anchor that the overlay which moved the focus and was anchored to `anchor` in the last layout has in
  /// `layers`, where it has another: see [`anchor_before`](Self::anchor_before).
  fn anchor_after<'a>(&self, anchor: &OwnedElementRef, layers: &'a OverlayLayers) -> Option<&'a OwnedElementRef> {
    let old = self
      .overlay_layer_anchors
      .iter()
      .position(|before| before.as_ref().is_some_and(|before| before.same_handle(anchor)))?;
    let origin = (*self.overlay_layer_origins.get(old)?)?;
    let index = layers.origins.iter().position(|declared| *declared == Some(origin))?;
    let now = layers.anchors.get(index)?.as_ref()?;
    let moves = layers.focus_movers.iter().any(|mover| mover.same_handle(now));
    (moves && !now.same_handle(anchor)).then_some(now)
  }

  /// The anchor of the overlay that takes the focus (`Overlay::takes_focus`) and held keyboard focus in the
  /// last layout, when it is not open in `layers` any more: the focus goes back to it. Where the anchor was in
  /// a layer that closed with it, as a submenu's item is in its menu when an item of the submenu is chosen, the
  /// focus goes back from that layer instead, and so on out: to what opened the outermost that closed.
  pub(super) fn focused_overlay_closed(&self, had_host: bool, layers: &OverlayLayers) -> Option<OwnedElementRef> {
    if !had_host {
      return None;
    }
    let closed = |layer: usize| -> Option<&OwnedElementRef> {
      let anchor = self.overlay_layer_anchors.get(layer.checked_sub(1)?)?.as_ref()?;
      let moved = self.overlay_focus_movers.iter().any(|mover| mover.same_handle(anchor));
      let open = layers.focus_movers.iter().any(|mover| mover.same_handle(anchor));
      // One that went to another anchor is open still.
      (moved && !open && self.anchor_after(anchor, layers).is_none()).then_some(anchor)
    };
    let mut layer = *self.focused_path.as_deref()?.first()?;
    let mut anchor = closed(layer)?;
    // Bounded by the number of layers: a layer's anchor is in a layer under it.
    for _ in 0..self.overlay_layer_parents.len() {
      let Some(outer) = self.overlay_layer_parents.get(layer - 1).copied().flatten() else {
        break;
      };
      let Some(outer_anchor) = closed(outer) else {
        break;
      };
      (layer, anchor) = (outer, outer_anchor);
    }
    Some(anchor.clone())
  }

  /// For each layer of the host just built whose overlay takes the focus, the layer its anchor is in, where
  /// that is a layer and not the page: see `overlay_layer_parents`. Only the layers are searched, not the page.
  pub(super) fn focus_layer_parents(&self) -> Vec<Option<usize>> {
    let Some(host) = self.root.as_ref() else {
      return Vec::new();
    };
    let layers = host.children().get(1..).unwrap_or_default();
    self
      .overlay_layer_anchors
      .iter()
      .map(|anchor| {
        let anchor = anchor.as_ref()?;
        if !self.overlay_focus_movers.iter().any(|mover| mover.same_handle(anchor)) {
          return None;
        }
        layers
          .iter()
          .position(|layer| path_to_ref(layer, anchor).is_some())
          .map(|index| index + 1)
      })
      .collect()
  }

  /// The layers of the overlay host whose overlays take the focus, each with its anchor: what Tab walks right
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

  /// The layer of the overlay that holds Tab (`Overlay::traps_tab`) and has the focus in it, or in an overlay
  /// hung from an element of it, and so on in: the scope of Tab. None where a modal is over it, which holds Tab
  /// itself.
  pub(super) fn tab_trap(&self) -> Option<&Node> {
    let host = self.root.as_ref()?;
    if self.overlay_tab_traps.is_empty() || !host.has_synthetic_role(crate::node::SyntheticNodeRole::OverlayHost) {
      return None;
    }
    let mut layer = *self.focused_path.as_deref()?.first()?;
    // Bounded by the number of layers: a layer's anchor is in a layer under it.
    for _ in 0..=self.overlay_layer_parents.len() {
      let anchor = self.overlay_layer_anchors.get(layer.checked_sub(1)?)?.as_ref();
      if anchor.is_some_and(|anchor| self.overlay_tab_traps.iter().any(|trap| trap.same_handle(anchor))) {
        let over = host.children().get(layer + 1..).unwrap_or_default();
        let under_modal = over
          .iter()
          .any(|above| above.has_synthetic_role(crate::node::SyntheticNodeRole::Modal));
        return host.children().get(layer).filter(|_| !under_modal);
      }
      layer = self.overlay_layer_parents.get(layer - 1).copied().flatten()?;
    }
    None
  }

  /// The first element that takes the focus in the topmost layer of the overlay host whose overlay takes the
  /// focus and opened in this pass: where the focus goes. `layers` are the layers as the host stacks them. Of the
  /// layers on one anchor it is the one that takes the focus, not a tooltip's on the same element.
  pub(super) fn opened_overlay_focus(&self, layers: &OverlayLayers) -> Option<NodeId> {
    let host = self.root.as_ref()?;
    layers
      .focus_takers
      .iter()
      .filter(|mover| !self.overlay_focus_movers.iter().any(|old| old.same_handle(mover)))
      // One that came from another anchor opened before: the focus is where it was left.
      .filter(|mover| self.anchor_before(mover, layers).is_none())
      .filter_map(|mover| {
        layers
          .anchors
          .iter()
          .zip(&layers.takes_focus)
          .rposition(|(anchor, takes)| *takes && anchor.as_ref().is_some_and(|anchor| anchor.same_handle(mover)))
      })
      .max()
      .and_then(|layer| host.children().get(layer + 1))
      .and_then(|layer| autofocused(layer).or_else(|| first_focusable(layer)))
  }

  /// Remembers what has the focus as the overlays that take it open in this
  /// pass, the anchors of those open now being `layers.focus_movers`, and
  /// forgets what was remembered for those that are not open any more, but
  /// for `returning`, the one the focus goes back from in this pass. One
  /// that Tab reaches after its anchor and that took nothing gives the focus
  /// back to its anchor.
  pub(super) fn note_focus_returns(&mut self, layers: &OverlayLayers, returning: Option<&OwnedElementRef>) {
    let movers = &layers.focus_movers;
    let had = self
      .focused_node
      .zip(self.focused_event_node)
      .map(|(input_id, event_id)| FocusTarget { input_id, event_id });
    for mover in movers {
      if !self.overlay_focus_movers.iter().any(|old| old.same_handle(mover)) {
        // One that came from another anchor keeps what it gives the focus back to, under its new anchor.
        if let Some(before) = self.anchor_before(mover, layers).cloned()
          && let Some(kept) = self
            .overlay_focus_returns
            .iter_mut()
            .find(|(anchor, _)| anchor.same_handle(&before))
        {
          kept.0 = mover.clone();
          continue;
        }
        let takes = layers.focus_takers.iter().any(|taker| taker.same_handle(mover));
        self.overlay_focus_returns.push((mover.clone(), had.filter(|_| takes)));
      }
    }
    self.overlay_focus_returns.retain(|(anchor, _)| {
      movers.iter().any(|mover| mover.same_handle(anchor)) || returning.is_some_and(|back| back.same_handle(anchor))
    });
  }

  /// Gives keyboard focus, lost with an overlay that closed or is hidden,
  /// back: to what had it when an overlay that takes the focus opened, where
  /// that is still in the tree; else to the overlay's `anchor`, the anchor or
  /// the first focusable node inside it, else the nearest focusable ancestor.
  /// Does nothing if focus is somewhere already.
  pub(super) fn return_focus_to_anchor(&mut self, anchor: &OwnedElementRef) {
    let had = self
      .overlay_focus_returns
      .iter()
      .position(|(held, _)| held.same_handle(anchor))
      .and_then(|index| {
        let open = self.overlay_focus_movers.iter().any(|mover| mover.same_handle(anchor));
        // An overlay hidden with its anchor is still open, and keeps what it gives the focus back to.
        if open {
          self.overlay_focus_returns[index].1
        } else {
          self.overlay_focus_returns.remove(index).1
        }
      });
    if self.focused_node.is_some() {
      return;
    }
    let root = self.root.as_ref();
    let back = had
      .filter(|had| root.is_some_and(|root| super::find_path_by_id(root, had.input_id).is_some()))
      .or_else(|| {
        let target = root.and_then(|root| focus_home(root, anchor))?;
        Some(FocusTarget {
          input_id: target,
          event_id: target,
        })
      });
    if let Some(target) = back {
      self.focus_node(target);
    }
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

/// The first element marked `autofocus` in `node` that takes the focus.
fn autofocused(node: &Node) -> Option<NodeId> {
  if node.is_autofocus() && takes_focus(node) {
    return Some(node.node_id());
  }
  node.children().iter().find_map(autofocused)
}

fn first_focusable(node: &Node) -> Option<NodeId> {
  if takes_focus(node) {
    return Some(node.node_id());
  }
  node.children().iter().find_map(first_focusable)
}

pub(super) fn path_to_ref(node: &Node, anchor: &OwnedElementRef) -> Option<Vec<usize>> {
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
