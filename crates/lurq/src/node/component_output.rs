//! What a component slot keeps of its last render (`ChildSlot::rendered` in
//! `app/ctx.rs`), and how an unchanged component's output is put together
//! from it again.
//!
//! A slot keeps its component's own output: the nodes its render built, with
//! a placeholder where it mounted a component of its own. That component's
//! output is in its own slot, so a component rendering again by itself
//! changes no cache above it, and keeping an output copies only the nodes of
//! one component, not the outputs nested in it. When the parent renders again
//! and the component is unchanged, its output is put together from its slot
//! and the slots of the components it mounted; each placeholder takes back
//! what the mounting code put on the root (`outer_decoration.rs`).
//!
//! The root of a component the render was handed (the slot children of
//! `mount_with`, the child a wrapper such as `Draggable` is given) is not a
//! component of this one: it is kept whole, with what this render put on it.
//! When its component renders again by itself afterwards, the output is put
//! together with its fresh render there, as the live tree has it.
//!
//! A component an element declares (`Element::component`) is mounted once
//! its parent's render returned, where nothing can change the node it gives
//! any more. When it is unchanged, what its parent's output holds for it is
//! no copy of its slot's output but a node that stands for it
//! (`Node::held_component`), and the tree, as it puts the parent's output in
//! the place of the old one, moves the component's live nodes over into
//! that node's place: nothing is copied and nothing is freed, and the nodes
//! keep their ids, their state and their layout. The copy from the slot is
//! still what a component not found in the old tree is made of, one found
//! there more than once, one in a modal's or an overlay's declaration, and
//! one whose root the parent decorates otherwise than before.

use std::{
  collections::{HashMap, HashSet},
  sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
  },
};

use super::node::{Node, SyntheticNodeRole};

/// Whether any render of the process ever held a component back. Until one does, no output is searched.
static HELD_IN_USE: AtomicBool = AtomicBool::new(false);

/// What is known of a held component while its parent's output and the old tree are looked through.
#[derive(Default)]
struct Held<'a> {
  /// The node that stands for it in the new output, where there is one such node.
  stands: Option<&'a Node>,
  /// Its root in the old tree, where it is there once.
  root: Option<&'a Node>,
  /// How often it stands in the new output, and how often its root is in the old tree.
  count: (usize, usize),
  /// One of them is in the declaration of a modal or an overlay.
  declared: bool,
}

impl Node {
  /// A placeholder this node's slot cache holds for a child component's
  /// root; never in the live tree.
  pub(crate) fn is_component_slot_placeholder(&self) -> bool {
    self.has_synthetic_role(SyntheticNodeRole::ComponentSlot)
  }

  fn component_slot_placeholder(&self) -> Node {
    let mut placeholder = Node::new();
    placeholder.component_slot_id = self.component_slot_id;
    placeholder.component_key = self.component_key.clone();
    placeholder.outer_decoration = self.outer_decoration.clone();
    placeholder.synthetic_role = Some(SyntheticNodeRole::ComponentSlot);
    placeholder
  }

  /// A copy of this output for its slot to keep: the root of each component
  /// below it that `own` says the rendering component mounted becomes a
  /// placeholder.
  pub(crate) fn clone_own_output(&self, own: &dyn Fn(u64) -> bool) -> Box<Node> {
    Box::new(self.clone_with_component_roots(&mut |root: &Node| {
      own(root.component_slot_id?).then(|| root.component_slot_placeholder())
    }))
  }

  /// [`Self::clone_for_reuse`], except that `component_root` is asked about
  /// each component root below this node, in children and declarations: the
  /// node it returns stands for that root's whole subtree, and `None` copies
  /// the root like any other node.
  pub(crate) fn clone_with_component_roots(&self, component_root: &mut dyn FnMut(&Node) -> Option<Node>) -> Node {
    let overlay_declaration = self
      .overlay_declaration
      .as_ref()
      .map(|spec| Box::new(spec.clone_with_node(spec.node.clone_below(component_root))));
    let modal_declaration = self
      .modal_declaration
      .as_ref()
      .map(|spec| Box::new(spec.clone_with_node(spec.node.clone_below(component_root))));
    // A plain loop, as `clone_children_for_reuse` has it, for unoptimized
    // builds' stack.
    let mut children = Vec::with_capacity(self.children.len());
    for child in &self.children {
      children.push(child.clone_below(component_root));
    }
    self.clone_around(overlay_declaration, modal_declaration, children)
  }

  /// A node that stands, in the output of its parent's render, for the unchanged component in slot `slot`:
  /// see the module.
  pub(crate) fn held_component(slot: u64, key: Option<&str>) -> Node {
    HELD_IN_USE.store(true, Ordering::Relaxed);
    let mut held = Node::new();
    held.component_slot_id = Some(slot);
    held.component_key = key.map(Arc::from);
    held.synthetic_role = Some(SyntheticNodeRole::HeldComponent);
    held
  }

  pub(crate) fn is_held_component(&self) -> bool {
    self.has_synthetic_role(SyntheticNodeRole::HeldComponent)
  }

  /// Before this output is matched with `old`, the tree it takes the place of: finds the components it holds
  /// back, and returns the slots of those whose live nodes `old` has and can give
  /// ([`Self::take_held_components_from`]). The others are made here from their slots' copies, which `cached`
  /// gives, so that they are matched with `old` as any fresh output is.
  pub(crate) fn settle_held_components(
    &mut self,
    old: &Node,
    cached: &dyn Fn(u64) -> Option<Box<Node>>,
  ) -> HashSet<u64> {
    if !HELD_IN_USE.load(Ordering::Relaxed) {
      return HashSet::new();
    }
    let moved = {
      let mut held = HashMap::new();
      self.find_held(false, &mut held);
      if held.is_empty() {
        return HashSet::new();
      }
      old.find_held_roots(false, &mut held);
      held
        .into_iter()
        .filter(|(_, held)| {
          held.count == (1, 1)
            && !held.declared
            && matches!((held.stands, held.root), (Some(stands), Some(root)) if stands.same_plain_outer_decoration(root))
        })
        .map(|(slot, _)| slot)
        .collect::<HashSet<u64>>()
    };
    self.make_held(&moved, cached);
    moved
  }

  /// After this output was matched with `old`: the live nodes of each component in `moved` leave `old` and
  /// take the place of the node that stood for the component here.
  pub(crate) fn take_held_components_from(&mut self, old: &mut Node, moved: HashSet<u64>) {
    if moved.is_empty() {
      return;
    }
    let mut taken = HashMap::with_capacity(moved.len());
    old.take_roots(&moved, &mut taken);
    self.put_held(&mut taken);
  }

  fn find_held<'a>(&'a self, declared: bool, held: &mut HashMap<u64, Held<'a>>) {
    if self.is_held_component() {
      if let Some(slot) = self.component_slot_id {
        let entry = held.entry(slot).or_default();
        entry.stands = Some(self);
        entry.count.0 += 1;
        entry.declared |= declared;
      }
      return;
    }
    for child in &self.children {
      child.find_held(declared, held);
    }
    if let Some(spec) = self.overlay_declaration.as_deref() {
      spec.node.find_held(true, held);
    }
    if let Some(spec) = self.modal_declaration.as_deref() {
      spec.node.find_held(true, held);
    }
  }

  fn find_held_roots<'a>(&'a self, declared: bool, held: &mut HashMap<u64, Held<'a>>) {
    if let Some(entry) = self.component_slot_id.and_then(|slot| held.get_mut(&slot)) {
      entry.root = Some(self);
      entry.count.1 += 1;
      entry.declared |= declared;
      return;
    }
    for child in &self.children {
      child.find_held_roots(declared, held);
    }
    if let Some(spec) = self.overlay_declaration.as_deref() {
      spec.node.find_held_roots(true, held);
    }
    if let Some(spec) = self.modal_declaration.as_deref() {
      spec.node.find_held_roots(true, held);
    }
  }

  /// Puts the copy from its slot in the place of every held component that is not in `moved`.
  fn make_held(&mut self, moved: &HashSet<u64>, cached: &dyn Fn(u64) -> Option<Box<Node>>) {
    if self.is_held_component() {
      let Some(slot) = self.component_slot_id else {
        return;
      };
      if moved.contains(&slot) {
        return;
      }
      if let Some(mut output) = cached(slot) {
        output.adopt_outer_decoration(self);
        // What it holds back itself is made too: nothing of it was looked for in the old tree.
        output.make_held(&HashSet::new(), cached);
        *self = *output;
      }
      return;
    }
    for child in &mut self.children {
      child.make_held(moved, cached);
    }
    if let Some(spec) = self.overlay_declaration.as_deref_mut() {
      spec.node.make_held(moved, cached);
    }
    if let Some(spec) = self.modal_declaration.as_deref_mut() {
      spec.node.make_held(moved, cached);
    }
  }

  fn take_roots(&mut self, moved: &HashSet<u64>, taken: &mut HashMap<u64, Node>) {
    if let Some(slot) = self.component_slot_id
      && moved.contains(&slot)
    {
      taken.insert(slot, std::mem::take(self));
      return;
    }
    for child in &mut self.children {
      child.take_roots(moved, taken);
    }
  }

  fn put_held(&mut self, taken: &mut HashMap<u64, Node>) {
    if self.is_held_component() {
      if let Some(mut root) = self.component_slot_id.and_then(|slot| taken.remove(&slot)) {
        // A theme of a subtree put on the node that stood for the component is the live root's now.
        if let Some(theme) = self.theme_scope.as_deref() {
          root.set_theme_scope(theme);
        }
        *self = root;
      }
      return;
    }
    if taken.is_empty() {
      return;
    }
    for child in &mut self.children {
      child.put_held(taken);
    }
  }

  fn clone_below(&self, component_root: &mut dyn FnMut(&Node) -> Option<Node>) -> Node {
    if self.component_slot_id.is_some()
      && let Some(node) = component_root(self)
    {
      return node;
    }
    self.clone_with_component_roots(component_root)
  }
}
