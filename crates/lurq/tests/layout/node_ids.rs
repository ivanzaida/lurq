use std::collections::HashSet;

use lurq::{
  app::{Tree, theme::Theme, component::Component, ctx::Ctx},
  core::{NodeId, Signal},
  node::{Element, ElementRef},
};

use crate::support::run_pass;

fn rt() -> Tree {
  Tree::new()
}

fn count_nodes(node: ElementRef<'_>) -> usize {
  1 + node.children().iter().map(count_nodes).sum::<usize>()
}

fn all_ids_assigned(node: ElementRef<'_>) -> bool {
  if !node.node_id().is_assigned() {
    return false;
  }
  node.children().iter().all(all_ids_assigned)
}

fn collect_ids(node: ElementRef<'_>, out: &mut Vec<NodeId>) {
  out.push(node.node_id());
  for child in node.children() {
    collect_ids(child, out);
  }
}

fn all_unique(node: ElementRef<'_>) -> bool {
  let mut ids = Vec::new();
  collect_ids(node, &mut ids);
  let set: HashSet<u64> = ids.iter().map(|id| id.value()).collect();
  set.len() == ids.len()
}

fn make_chain(depth: usize) -> Element {
  let mut node = Element::new();
  for _ in 0..depth {
    node = lurq::components::Column::new().child(node).into();
  }
  node
}

fn make_wide(width: usize) -> Element {
  lurq::components::Row::new()
    .with_children((0..width).map(|_| Element::new()))
    .into()
}

fn make_tree(depth: usize, branching: usize) -> Element {
  if depth == 0 {
    return Element::new();
  }
  lurq::components::Row::new()
    .with_children((0..branching).map(|_| make_tree(depth - 1, branching)))
    .into()
}

struct StableRoot;

impl Component for StableRoot {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    lurq::components::Row::new()
      .child(lurq::components::Rect::new(20.0, 20.0))
      .child(lurq::components::Text::new("stable"))
  }
}

#[derive(Clone)]
struct SignalProp(Signal<i32>);

impl PartialEq for SignalProp {
  fn eq(&self, _other: &Self) -> bool {
    true
  }
}

struct SignalChild {
  signal: Signal<i32>,
}

impl Component for SignalChild {
  type Props = SignalProp;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      signal: ctx.props::<Self::Props>().0.clone(),
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    lurq::components::Row::new()
      .child(lurq::components::Text::new(&format!("{}", self.signal.get())))
      .child(lurq::components::Rect::new(20.0, 20.0))
  }
}

struct SignalParent {
  signal: Signal<i32>,
}

impl Component for SignalParent {
  type Props = SignalProp;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      signal: ctx.props::<Self::Props>().0.clone(),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    lurq::components::Column::new()
      .child(lurq::components::Rect::new(20.0, 20.0))
      .child(ctx.mount::<SignalChild>(SignalProp(self.signal.clone())))
  }
}

// ============================================================================
// Nodes start unassigned
// ============================================================================

#[test]
fn new_node_has_unassigned_id() {
  let node = Element::new();
  assert!(!node.node_id().is_assigned());
}

#[test]
fn text_node_has_unassigned_id() {
  let node = lurq::components::Text::new("hello");
  assert!(!node.node_id().is_assigned());
}

#[test]
fn tree_root_assigns_ids() {
  let node = lurq::components::Column::new()
    .child(lurq::components::Row::new().child(Element::new()).child(Element::new()))
    .child(Element::new());
  let mut rt = rt();
  rt.set_root(node);
  assert!(all_ids_assigned(rt.root().unwrap()));
}

// ============================================================================
// set_root assigns IDs
// ============================================================================

#[test]
fn set_root_assigns_single_node() {
  let mut rt = rt();
  rt.set_root(Element::new());
  let root = rt.root().unwrap();
  assert!(root.node_id().is_assigned());
}

#[test]
fn set_root_assigns_all_children() {
  let mut rt = rt();
  let node = lurq::components::Column::new()
    .child(Element::new())
    .child(lurq::components::Row::new().child(Element::new()).child(Element::new()));
  rt.set_root(node);
  let root = rt.root().unwrap();
  assert!(all_ids_assigned(root));
  assert_eq!(count_nodes(root), 5);
}

#[test]
fn set_root_assigns_unique_ids() {
  let mut rt = rt();
  let node = lurq::components::Row::new().with_children((0..10).map(|_| Element::new()));
  rt.set_root(node);
  let root = rt.root().unwrap();
  assert!(all_unique(root));
}

// ============================================================================
// Replacing root frees old IDs
// ============================================================================

#[test]
fn replacing_root_reuses_freed_ids() {
  let mut rt = rt();
  rt.set_root(lurq::components::Row::new().with_children((0..5).map(|_| Element::new())));
  let mut first_ids = Vec::new();
  collect_ids(rt.root().unwrap(), &mut first_ids);
  assert_eq!(first_ids.len(), 6);

  rt.set_root(lurq::components::Row::new().with_children((0..5).map(|_| Element::new())));
  let mut second_ids = Vec::new();
  collect_ids(rt.root().unwrap(), &mut second_ids);

  let first_set: HashSet<u64> = first_ids.iter().map(|id| id.value()).collect();
  let second_set: HashSet<u64> = second_ids.iter().map(|id| id.value()).collect();
  assert!(
    !first_set.is_disjoint(&second_set),
    "IDs should be reused after freeing"
  );
}

#[test]
fn replacing_root_new_tree_fully_assigned() {
  let mut rt = rt();
  rt.set_root(Element::new());
  rt.set_root(
    lurq::components::Column::new()
      .child(Element::new())
      .child(Element::new()),
  );
  let root = rt.root().unwrap();
  assert!(all_ids_assigned(root));
  assert!(all_unique(root));
}

#[test]
fn rebuild_preserves_matching_node_ids() {
  let mut rt = rt();
  rt.mount_root::<StableRoot>(Theme::default(), ());
  let mut before = Vec::new();
  collect_ids(rt.root().unwrap(), &mut before);

  rt.rebuild();

  let mut after = Vec::new();
  collect_ids(rt.root().unwrap(), &mut after);
  assert_eq!(after, before);
}

#[test]
fn dirty_subtree_refresh_preserves_matching_node_ids() {
  let signal = Signal::new(0);
  let mut rt = rt();
  rt.mount_root::<SignalParent>(Theme::default(), SignalProp(signal.clone()));
  let mut before = Vec::new();
  collect_ids(rt.root().unwrap(), &mut before);

  signal.set(1);
  run_pass(&mut rt);

  let mut after = Vec::new();
  collect_ids(rt.root().unwrap(), &mut after);
  assert_eq!(after, before);
}

// ============================================================================
// Depth 1 — single node
// ============================================================================

#[test]
fn depth_1() {
  let mut rt = rt();
  rt.set_root(Element::new());
  assert!(all_ids_assigned(rt.root().unwrap()));
  assert_eq!(count_nodes(rt.root().unwrap()), 1);
}

// ============================================================================
// Depth 2 — parent + children
// ============================================================================

#[test]
fn depth_2_single_child() {
  let mut rt = rt();
  rt.set_root(lurq::components::Column::new().child(Element::new()));
  let root = rt.root().unwrap();
  assert!(all_ids_assigned(root));
  assert!(all_unique(root));
  assert_eq!(count_nodes(root), 2);
}

#[test]
fn depth_2_many_children() {
  let mut rt = rt();
  rt.set_root(make_wide(50));
  let root = rt.root().unwrap();
  assert!(all_ids_assigned(root));
  assert!(all_unique(root));
  assert_eq!(count_nodes(root), 51);
}

// ============================================================================
// Depth 10
// ============================================================================

#[test]
fn depth_10_chain() {
  let mut rt = rt();
  rt.set_root(make_chain(10));
  let root = rt.root().unwrap();
  assert!(all_ids_assigned(root));
  assert!(all_unique(root));
  assert_eq!(count_nodes(root), 11);
}

// ============================================================================
// Depth 50
// ============================================================================

#[test]
fn depth_50_chain() {
  let mut rt = rt();
  rt.set_root(make_chain(50));
  let root = rt.root().unwrap();
  assert!(all_ids_assigned(root));
  assert!(all_unique(root));
  assert_eq!(count_nodes(root), 51);
}

// ============================================================================
// Depth 100
// ============================================================================

#[test]
fn depth_100_chain() {
  let mut rt = rt();
  rt.set_root(make_chain(100));
  let root = rt.root().unwrap();
  assert!(all_ids_assigned(root));
  assert!(all_unique(root));
  assert_eq!(count_nodes(root), 101);
}

// ============================================================================
// Depth 255
// ============================================================================

#[test]
fn depth_255_chain() {
  let mut rt = rt();
  rt.set_root(make_chain(255));
  let root = rt.root().unwrap();
  assert!(all_ids_assigned(root));
  assert!(all_unique(root));
  assert_eq!(count_nodes(root), 256);
}

// ============================================================================
// Wide tree at depth 255 — chain with leaf fan at the bottom
// ============================================================================

#[test]
fn depth_255_chain_with_wide_leaf() {
  let mut rt = rt();
  let leaf = make_wide(20);
  let mut node = leaf;
  for _ in 0..254 {
    node = lurq::components::Column::new().child(node).into();
  }
  rt.set_root(node);
  let root = rt.root().unwrap();
  assert!(all_ids_assigned(root));
  assert!(all_unique(root));
  assert_eq!(count_nodes(root), 255 + 20);
}

// ============================================================================
// Branching tree — binary tree of various depths
// ============================================================================

#[test]
fn binary_tree_depth_8() {
  let mut rt = rt();
  rt.set_root(make_tree(8, 2));
  let root = rt.root().unwrap();
  assert!(all_ids_assigned(root));
  assert!(all_unique(root));
  let expected = (1_usize << 9) - 1; // 2^9 - 1 = 511
  assert_eq!(count_nodes(root), expected);
}

#[test]
fn ternary_tree_depth_5() {
  let mut rt = rt();
  rt.set_root(make_tree(5, 3));
  let root = rt.root().unwrap();
  assert!(all_ids_assigned(root));
  assert!(all_unique(root));
  // (3^6 - 1) / (3 - 1) = 364
  let expected = (3_usize.pow(6) - 1) / 2;
  assert_eq!(count_nodes(root), expected);
}

// ============================================================================
// ID recycling across multiple mount/unmount cycles
// ============================================================================

#[test]
fn ids_recycled_over_many_cycles() {
  let mut rt = rt();
  for _ in 0..100 {
    rt.set_root(make_wide(10));
    let root = rt.root().unwrap();
    assert!(all_ids_assigned(root));
    assert!(all_unique(root));
  }
  let mut ids = Vec::new();
  collect_ids(rt.root().unwrap(), &mut ids);
  let max_id = ids.iter().map(|id| id.value()).max().unwrap();
  assert!(
    max_id <= 11,
    "IDs should be recycled, max should be <=11, got {}",
    max_id
  );
}

#[test]
fn ids_recycled_varying_tree_sizes() {
  let mut rt = rt();
  for i in 0..50 {
    let width = (i % 10) + 1;
    rt.set_root(make_wide(width));
    assert!(all_ids_assigned(rt.root().unwrap()));
    assert!(all_unique(rt.root().unwrap()));
  }
}

// ============================================================================
// Mixed modifier chains — ensure wrapping modifiers get IDs too
// ============================================================================

#[test]
fn modifier_wrappers_get_ids() {
  let mut rt = rt();
  let node = lurq::components::Spacer::new()
    .size(100.0, 100.0)   // FrameModifier wrapper
    .pad(10.0)             // PaddingModifier wrapper
    .offset(5.0, 5.0)     // OffsetModifier wrapper
    .flex(1.0); // FlexModifier wrapper
  rt.set_root(node);
  let root = rt.root().unwrap();
  assert!(all_ids_assigned(root));
  assert!(all_unique(root));
  assert_eq!(count_nodes(root), 5); // flex > offset > padding > frame > leaf
}

#[test]
fn scroll_container_gets_ids() {
  let mut rt = rt();
  let node = lurq::components::ScrollVertical::new(
    lurq::components::Column::new().with_children((0..10).map(|_| Element::new())),
  );
  rt.set_root(node);
  let root = rt.root().unwrap();
  assert!(all_ids_assigned(root));
  assert!(all_unique(root));
  assert_eq!(count_nodes(root), 12); // scroll + column + 10 leaves
}

// ============================================================================
// Stack layout gets IDs
// ============================================================================

#[test]
fn stack_children_get_ids() {
  let mut rt = rt();
  let node = lurq::components::Stack::new().with_children(vec![
    lurq::components::Spacer::new().size(100.0, 100.0),
    lurq::components::Spacer::new().size(50.0, 50.0),
  ]);
  rt.set_root(node);
  let root = rt.root().unwrap();
  assert!(all_ids_assigned(root));
  assert!(all_unique(root));
  assert_eq!(count_nodes(root), 5); // stack + 2*(frame + leaf)
}

// ============================================================================
// Depth 255 with modifiers at every level
// ============================================================================

#[test]
fn depth_255_with_modifiers() {
  let mut rt = rt();
  let mut node = Element::new();
  for i in 0..255 {
    node = if i % 3 == 0 {
      lurq::components::Column::new().child(node).into()
    } else if i % 3 == 1 {
      lurq::components::Row::new().child(node).into()
    } else {
      lurq::components::Stack::new().child(node).pad(1.0).into()
    };
  }
  rt.set_root(node);
  let root = rt.root().unwrap();
  assert!(all_ids_assigned(root));
  assert!(all_unique(root));
  assert_eq!(count_nodes(root), 341);
}

// ============================================================================
// Replacing deep tree with shallow and vice versa
// ============================================================================

#[test]
fn replace_deep_with_shallow() {
  let mut rt = rt();
  rt.set_root(make_chain(255));
  assert_eq!(count_nodes(rt.root().unwrap()), 256);

  rt.set_root(Element::new());
  let root = rt.root().unwrap();
  assert!(root.node_id().is_assigned());
  assert_eq!(count_nodes(root), 1);
}

#[test]
fn replace_shallow_with_deep() {
  let mut rt = rt();
  rt.set_root(Element::new());
  rt.set_root(make_chain(255));
  let root = rt.root().unwrap();
  assert!(all_ids_assigned(root));
  assert!(all_unique(root));
  assert_eq!(count_nodes(root), 256);
}

#[test]
fn replace_deep_with_deep_recycles() {
  let mut rt = rt();
  rt.set_root(make_chain(100));
  rt.set_root(make_chain(100));
  let root = rt.root().unwrap();
  assert!(all_ids_assigned(root));
  assert!(all_unique(root));

  let mut ids = Vec::new();
  collect_ids(root, &mut ids);
  let max_id = ids.iter().map(|id| id.value()).max().unwrap();
  assert!(
    max_id <= 101,
    "IDs should be recycled, max should be <=101, got {}",
    max_id
  );
}
