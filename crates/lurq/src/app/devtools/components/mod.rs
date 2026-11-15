//! The Components tab: the tree of the inspected window, and beside it what is known of the node selected in it.
//! Built from the board (`pencil/lurq-devtools.pen`, Components · dark) with the design system's components.

mod inspector;
mod search;
mod summary;
pub(crate) mod tree;

use std::sync::{
  Arc,
  atomic::{AtomicU64, Ordering},
};

use super::{
  DevToolsBoolCallback, DevToolsDebugOverlayCallback, DevToolsPathCallback, kit::FILL, snapshot::DevToolsSnapshot,
};
use crate::{
  app::ctx::Ctx,
  components::Row,
  core::{ElementRef, NodeId, Signal},
  node::Element,
  ui::{Splitter, SplitterPane},
};

/// The board's widths: the tree as it opens, and the least a person can make each side.
const TREE_WIDTH: f32 = 540.0;
const TREE_MIN_WIDTH: f32 = 320.0;
const INSPECTOR_MIN_WIDTH: f32 = 480.0;

/// Which list of the selected node the inspector shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, crate::DevtoolsInspectable)]
pub(crate) enum DetailTab {
  Props,
  Context,
  Reactive,
  Attributes,
}

/// What the tab tells the inspected window: each is the runtime's, and is missing in a test.
#[derive(Clone, Default)]
pub(crate) struct Inspected {
  pub(crate) on_debug_overlay_path: Option<DevToolsDebugOverlayCallback>,
  pub(crate) on_overlay_enabled: Option<DevToolsBoolCallback>,
  pub(crate) on_pick_inspected: Option<DevToolsBoolCallback>,
  pub(crate) on_selected_path: Option<DevToolsPathCallback>,
  pub(crate) on_save_node_screenshot: Option<DevToolsPathCallback>,
}

/// What the tab keeps between renders.
#[derive(Clone)]
pub(crate) struct ComponentsState {
  /// The way to the selected node from the root: the root itself until another is selected.
  pub(crate) selected_path: Signal<Vec<usize>>,
  /// The nodes whose children are hidden. A node is open unless it is here.
  pub(crate) closed: Signal<Vec<NodeId>>,
  pub(crate) search: Signal<String>,
  /// Whether the selected node is outlined in the inspected window.
  pub(crate) overlay: Signal<bool>,
  /// Whether the next press in the inspected window selects the node under it.
  pub(crate) pick: Signal<bool>,
  pub(crate) detail: Signal<DetailTab>,
  /// The search box, and how many times its key was pressed: the focus goes to it once for each.
  pub(crate) search_field: ElementRef,
  search_asked: Signal<u64>,
  search_answered: Arc<AtomicU64>,
  /// How many times the tree was asked to bring the selected row into view.
  shown: Arc<AtomicU64>,
}

impl ComponentsState {
  pub(crate) fn new(ctx: &mut Ctx) -> Self {
    Self {
      selected_path: ctx.signal(Vec::new()),
      closed: ctx.signal(Vec::new()),
      search: ctx.signal(String::new()),
      overlay: ctx.signal(true),
      pick: ctx.signal(false),
      detail: ctx.signal(DetailTab::Props),
      search_field: ElementRef::new(),
      search_asked: ctx.signal(0),
      search_answered: Arc::default(),
      shown: Arc::default(),
    }
  }

  /// The state of a tab that is in no window: what a test acts on.
  #[cfg(test)]
  pub(crate) fn detached() -> Self {
    Self {
      selected_path: Signal::new(Vec::new()),
      closed: Signal::new(Vec::new()),
      search: Signal::new(String::new()),
      overlay: Signal::new(true),
      pick: Signal::new(false),
      detail: Signal::new(DetailTab::Props),
      search_field: ElementRef::new(),
      search_asked: Signal::new(0),
      search_answered: Arc::default(),
      shown: Arc::default(),
    }
  }

  /// The key of the search was pressed: the focus goes to the box at the next render.
  pub(crate) fn ask_search(&self) {
    self.search_asked.update(|asked| *asked += 1);
  }

  /// Whether the key of the search was pressed since this was last asked. It reads the count, so what asks is
  /// rendered again at the next press.
  pub(crate) fn search_wanted(&self) -> bool {
    let asked = self.search_asked.get();
    self.search_answered.swap(asked, Ordering::Relaxed) != asked
  }

  /// Which asking to bring the selected row into view is the last: the tree answers each once.
  pub(crate) fn shown(&self) -> u64 {
    self.shown.load(Ordering::Relaxed)
  }

  /// Opens a closed node and closes an open one.
  pub(crate) fn toggle(&self, id: NodeId) {
    self
      .closed
      .update(|closed| match closed.iter().position(|at| *at == id) {
        Some(index) => {
          closed.remove(index);
        }
        None => closed.push(id),
      });
  }

  /// Selects the node at `path`, and outlines it in the inspected window while the overlay is on.
  pub(crate) fn select(&self, path: Vec<usize>, inspected: &Inspected) {
    if let Some(told) = &inspected.on_selected_path {
      told(path.clone());
    }
    if let Some(told) = &inspected.on_debug_overlay_path {
      told(overlay_path(self.overlay.get_untracked(), path.clone()));
    }
    self.selected_path.set(path);
  }

  /// Selects a node picked in the inspected window, opening what it is in so that its row is shown.
  pub(crate) fn show_picked(&self, snapshot: &DevToolsSnapshot, path: Vec<usize>, inspected: &Inspected) {
    let way = tree::ancestors(snapshot, &path);
    self.closed.update(|closed| closed.retain(|id| !way.contains(id)));
    self.select(path, inspected);
    self.shown.fetch_add(1, Ordering::Relaxed);
  }

  pub(crate) fn set_overlay(&self, on: bool, inspected: &Inspected) {
    self.overlay.set(on);
    if let Some(told) = &inspected.on_overlay_enabled {
      told(on);
    }
    if let Some(told) = &inspected.on_debug_overlay_path {
      told(overlay_path(on, self.selected_path.get_untracked()));
    }
  }

  pub(crate) fn set_pick(&self, on: bool, inspected: &Inspected) {
    self.pick.set(on);
    if let Some(told) = &inspected.on_pick_inspected {
      told(on);
    }
  }
}

/// The node the inspected window outlines: the selected one while the overlay is on.
pub(crate) fn overlay_path(overlay: bool, selected: Vec<usize>) -> Option<Vec<usize>> {
  overlay.then_some(selected)
}

/// The tab: the tree and the inspector, with a divider between them that a person drags.
pub(super) fn view(
  ctx: &mut Ctx,
  snapshot: &DevToolsSnapshot,
  state: &ComponentsState,
  inspected: &Inspected,
) -> Element {
  let tree = tree::panel(ctx, snapshot, state, inspected);
  let inspector = inspector::panel(snapshot, state, inspected);
  let panes = Splitter::new()
    .id("devtools-components")
    .pane(
      SplitterPane::new(tree)
        .length(TREE_WIDTH)
        .min(TREE_MIN_WIDTH)
        .label("Component tree"),
    )
    .pane(SplitterPane::new(inspector).min(INSPECTOR_MIN_WIDTH).label("Inspector"))
    .mount(ctx);
  Row::new().size(FILL, FILL).flex(1.0).child(panes).into()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn the_overlay_is_on_the_selected_node_while_it_is_on() {
    assert_eq!(overlay_path(true, vec![1, 2]), Some(vec![1, 2]));
    assert_eq!(overlay_path(false, vec![1, 2]), None);
  }

  #[test]
  fn a_node_is_closed_and_opened_again() {
    let state = ComponentsState::detached();
    let id = crate::core::IdGenerator::new().next();
    state.toggle(id);
    assert_eq!(state.closed.get_untracked(), [id]);
    state.toggle(id);
    assert!(state.closed.get_untracked().is_empty());
  }

  #[test]
  fn the_search_is_focused_once_for_each_press_of_its_key() {
    let state = ComponentsState::detached();
    assert!(!state.search_wanted());
    state.ask_search();
    assert!(state.search_wanted());
    assert!(!state.search_wanted(), "one press is one focus");
  }

  #[test]
  fn a_picked_node_is_selected_with_the_nodes_it_is_in_opened() {
    use crate::app::devtools::snapshot::{DevToolsNode, DevToolsNodeKind};
    let ids = crate::core::IdGenerator::new();
    let (root, child) = (ids.next(), ids.next());
    let leaf = DevToolsNode::for_test(ids.next(), "Text", DevToolsNodeKind::Element, vec![]);
    let snapshot = DevToolsSnapshot {
      root: Some(DevToolsNode::for_test(
        root,
        "Root",
        DevToolsNodeKind::Component,
        vec![DevToolsNode::for_test(
          child,
          "Row",
          DevToolsNodeKind::Element,
          vec![leaf],
        )],
      )),
      frame: Default::default(),
    };
    let state = ComponentsState::detached();
    state.toggle(root);
    state.toggle(child);
    let before = state.shown();

    state.show_picked(&snapshot, vec![0, 0], &Inspected::default());

    assert!(state.closed.get_untracked().is_empty());
    assert_eq!(state.selected_path.get_untracked(), [0, 0]);
    assert_eq!(
      state.shown(),
      before + 1,
      "the tree is asked to bring its row into view"
    );
  }
}
