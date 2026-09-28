//! A left press outside an open popup closes it: an overlay with
//! `dismiss_on_outside_click(true)` (`Popup`, `Popover`, `Overlay`) or a
//! `Select`'s menu.
//!
//! By default ([`OutsidePress::Consume`]) that press only closes the popup,
//! like a native menu: it is not dispatched, and its release and click are
//! swallowed too, so the element under the pointer is neither pressed, clicked
//! nor focused. When every popup the press closes is
//! [`OutsidePress::PassThrough`], the press is dispatched as usual and the
//! popups close afterwards, unless a handler prevents the default.
//!
//! A press is outside an overlay when it lands neither on the overlay's
//! content, nor on its anchor, nor on a layer stacked above it (a select menu
//! or a popup opened from inside it, which the overlay host places after it).

use super::{OverlayDismissEntry, Tree, close_open_selects_except, point_in_element_rect};
use crate::{
  app::{ctx::OutsidePress, hit_test::hit_test_tree},
  core::{NodeId, Signal},
  layout::layout_result::LayoutResult,
  node::{Node, SyntheticNodeRole, node_kind::NodeKind},
};

impl Tree {
  /// Closes the popups a left press at logical `(x, y)` is outside of when at
  /// least one of them consumes the press, and returns whether it did. The
  /// caller then drops the press and its release.
  pub(super) fn consume_outside_press(&mut self, x: f32, y: f32) -> bool {
    self.rebuild_if_dirty();
    let (Some(root), Some(result)) = (&self.root, &self.last_layout) else {
      return false;
    };
    // Without an overlay host no popup or select menu is on screen.
    if !root.has_synthetic_role(SyntheticNodeRole::OverlayHost) {
      return false;
    }
    let layer = pressed_layer(root, result, x, y);
    let overlays: Vec<&OverlayDismissEntry> = self
      .overlay_dismiss_entries
      .iter()
      .filter(|entry| is_outside(entry, x, y, layer) && entry.open.get_untracked())
      .collect();
    let selects = selects_closed_by_press(root, result, x, y);
    let consumed = overlays
      .iter()
      .any(|entry| entry.outside_press == OutsidePress::Consume)
      || selects
        .as_ref()
        .is_some_and(|selects| any_open_select_consumes(root, selects.pressed));
    if !consumed {
      return false;
    }

    let close: Vec<Signal<bool>> = overlays.iter().map(|entry| entry.open.clone()).collect();
    if let Some(selects) = selects {
      close_open_selects_except(root, selects.pressed);
    }
    for open in close {
      open.set(false);
    }
    self.needs_redraw = true;
    true
  }

  /// The open states of the overlays a left press at logical `(x, y)` is
  /// outside of; the press closes them after it has been dispatched.
  pub(super) fn overlay_dismiss_signals_at(&self, x: f32, y: f32) -> Vec<Signal<bool>> {
    let layer = match (&self.root, &self.last_layout) {
      (Some(root), Some(result)) => pressed_layer(root, result, x, y),
      _ => 0,
    };
    self
      .overlay_dismiss_entries
      .iter()
      .filter(|entry| is_outside(entry, x, y, layer))
      .map(|entry| entry.open.clone())
      .collect()
  }
}

fn is_outside(entry: &OverlayDismissEntry, x: f32, y: f32, pressed_layer: usize) -> bool {
  entry.dismiss_on_outside_click
    && pressed_layer < entry.layer
    && !point_in_element_rect(x, y, entry.anchor.bounds())
    && !point_in_element_rect(x, y, entry.bounds)
}

/// The overlay host child the topmost element at `(x, y)` belongs to: `0`
/// for the page (or nothing), `n` for the host's `n`th overlay layer.
fn pressed_layer(root: &Node, result: &LayoutResult, x: f32, y: f32) -> usize {
  if !root.has_synthetic_role(SyntheticNodeRole::OverlayHost) {
    return 0;
  }
  root
    .children()
    .iter()
    .zip(result.children.iter())
    .enumerate()
    .skip(1)
    .rev()
    .find(|(_, (layer, layout))| {
      let mut hits = Vec::new();
      hit_test_tree(layer, &layout.result, layout.offset.x, layout.offset.y, x, y, &mut hits);
      !hits.is_empty()
    })
    .map_or(0, |(index, _)| index)
}

/// The selects a left press closes, as the press dispatch does: every open
/// select except the pressed one (its own click toggles it). `None` when the
/// press lands on a select menu, which closes nothing.
struct ClosedSelects {
  pressed: Option<NodeId>,
}

fn selects_closed_by_press(root: &Node, result: &LayoutResult, x: f32, y: f32) -> Option<ClosedSelects> {
  let mut hits = Vec::new();
  hit_test_tree(root, result, 0.0, 0.0, x, y, &mut hits);
  if hits
    .iter()
    .any(|(node, _)| node.has_synthetic_role(SyntheticNodeRole::SelectMenu))
  {
    return None;
  }
  let pressed = hits
    .iter()
    .find(|(node, _)| matches!(node.node_kind(), NodeKind::Select { .. }))
    .map(|(node, _)| node.node_id());
  Some(ClosedSelects { pressed })
}

fn any_open_select_consumes(node: &Node, except: Option<NodeId>) -> bool {
  if let NodeKind::Select { state } = node.node_kind()
    && Some(node.node_id()) != except
    && state.is_open()
    && state.outside_press() == OutsidePress::Consume
  {
    return true;
  }
  node
    .children()
    .iter()
    .any(|child| any_open_select_consumes(child, except))
}
