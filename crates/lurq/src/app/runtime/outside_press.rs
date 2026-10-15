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
//! A press on another dropdown's trigger is dispatched whatever the popups
//! say, like a menu bar: one press closes the open dropdown and opens the
//! pressed one. The first interactive element under the pointer (focusable,
//! focus-disabled, a button or a select, as the press dispatch decides) is a
//! trigger when it is a `Select`, or the anchor itself of a signal-driven
//! overlay that dismisses on outside presses and switches on an anchor press,
//! declared open or closed. The press closes the consuming popups and the
//! open selects but the pressed one before it is dispatched, so a trigger that
//! prevents the press's default still switches.
//!
//! A press is outside an overlay when it lands neither on the overlay's
//! content, nor on its anchor, nor on a layer stacked above it (a select menu
//! or a popup opened from inside it, which the overlay host places after it).

use super::{
  OverlayDismissEntry, Tree, close_open_selects_except, point_in_element_rect, scrollbar_hit_at, trim_hits_to_scrollbar,
};
use crate::{
  app::{
    ctx::OutsidePress,
    hit_test::{HitRect, hit_test_tree},
  },
  core::{ElementRef, NodeId, Signal},
  layout::layout_result::LayoutResult,
  node::{Node, SyntheticNodeRole, node_kind::NodeKind},
};

impl Tree {
  /// Handles a left press at logical `(x, y)` that is outside open popups,
  /// at least one of which consumes it, and returns whether the press is
  /// consumed. A consumed press closes the popups it is outside of, and the
  /// caller drops it and its release. A press on another dropdown's trigger
  /// closes the consuming popups and the open selects but the pressed one,
  /// and is not consumed.
  pub(super) fn consume_outside_press(&mut self, x: f32, y: f32) -> bool {
    self.rebuild_if_dirty();
    let (Some(root), Some(result)) = (&self.root, &self.last_layout) else {
      return false;
    };
    // Without an overlay host no popup or select menu is on screen.
    if !root.has_synthetic_role(SyntheticNodeRole::OverlayHost) {
      return false;
    }
    let mut hits = Vec::new();
    hit_test_tree(root, result, 0.0, 0.0, x, y, &mut hits);
    trim_hits_to_scrollbar(&mut hits, x, y);
    let on_select_menu = hits
      .iter()
      .any(|(node, _)| node.has_synthetic_role(SyntheticNodeRole::SelectMenu));
    let layer = pressed_layer(root, result, x, y);
    let overlays: Vec<&OverlayDismissEntry> = self
      .overlay_dismiss_entries
      .iter()
      .filter(|entry| is_outside(entry, x, y, layer) && entry.open.get_untracked())
      .collect();
    let consumed = overlays
      .iter()
      .any(|entry| entry.outside_press == OutsidePress::Consume)
      || (!on_select_menu && any_open_select_consumes(root));
    if !consumed {
      return false;
    }

    let trigger = trigger_under(&hits, x, y, &self.dropdown_anchors);
    // A trigger's press is dispatched, which closes the pass-through popups
    // unless a handler prevents its default.
    let close: Vec<Signal<bool>> = overlays
      .iter()
      .filter(|entry| trigger.is_none() || entry.outside_press == OutsidePress::Consume)
      .map(|entry| entry.open.clone())
      .collect();
    if !on_select_menu {
      let pressed_select = match trigger {
        Some(Trigger::Select(id)) => Some(id),
        _ => None,
      };
      close_open_selects_except(root, pressed_select);
    }
    for open in close {
      open.set(false);
    }
    self.needs_redraw = true;
    trigger.is_none()
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
/// for the page or window chrome (or nothing), `n` for the host's `n`th other overlay layer.
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
    .filter(|(_, (layer, _))| !layer.has_synthetic_role(SyntheticNodeRole::WindowChromeLayer))
    .find(|(_, (layer, layout))| {
      let mut hits = Vec::new();
      hit_test_tree(layer, &layout.result, layout.offset.x, layout.offset.y, x, y, &mut hits);
      !hits.is_empty()
    })
    .map_or(0, |(index, _)| index)
}

/// The dropdown trigger a press lands on.
#[derive(Clone, Copy)]
enum Trigger {
  /// A select, whose own press toggles its menu.
  Select(NodeId),
  /// The anchor of a dropdown, whose handlers open it.
  Anchor,
}

/// The trigger the first interactive element under a press at `(x, y)` is,
/// if any. `hits` runs from the innermost node outward, trimmed to a
/// scrollbar that takes the press.
fn trigger_under(hits: &[(&Node, HitRect)], x: f32, y: f32, anchors: &[ElementRef]) -> Option<Trigger> {
  let (node, _) = hits.first()?;
  if scrollbar_hit_at(node, x, y).is_some() {
    return None;
  }
  let (node, _) = hits.iter().find(|(node, _)| is_interactive(node))?;
  if matches!(node.node_kind(), NodeKind::Select { .. }) {
    return Some(Trigger::Select(node.node_id()));
  }
  let element_ref = node.element_ref.as_ref()?;
  anchors
    .iter()
    .any(|anchor| anchor.same_handle(element_ref))
    .then_some(Trigger::Anchor)
}

/// Whether a press on `node` belongs to it, as the press dispatch decides
/// focus and built-in behaviour: it takes focus (or would, when enabled), is a
/// button or is a select.
fn is_interactive(node: &Node) -> bool {
  node.is_focusable()
    || node.is_focus_disabled()
    || node.button_kind_value().is_some()
    || matches!(node.node_kind(), NodeKind::Select { .. })
}

fn any_open_select_consumes(node: &Node) -> bool {
  if let NodeKind::Select { state } = node.node_kind()
    && state.is_open()
    && state.outside_press() == OutsidePress::Consume
  {
    return true;
  }
  node.children().iter().any(any_open_select_consumes)
}
