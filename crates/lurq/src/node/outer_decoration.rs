//! What the code that mounts a component puts on the root of the element
//! `mount` returns: an `id`, classes or a `key` (`Ctx::mount::<C>(props).id("x")`),
//! and what a wrapper such as `Draggable` puts on the root of the child it is
//! given (`Element::decorate_root`).
//!
//! A theme for a subtree whose root is the component (`Ctx::in_theme` around `mount`, or `.theme(..)` on what
//! `mount` returns) is such a decoration too: the scope is on the root the component's own renders replace.
//!
//! The component's root node holds them in the mounting component's rendered
//! tree, but the component's own render knows nothing of them. When the
//! component renders again by itself (a dirty refresh, which does not render
//! the mounting component again), its fresh root replaces the decorated one in
//! the live tree; the root being replaced hands its decoration to the fresh
//! one there, so a lookup by the outer id still finds the component. The
//! mounting component's cached output keeps the decoration on its placeholder
//! for the component (`component_output.rs`), and hands it to the root it is
//! put together with.
//!
//! An element can also declare a component where it stands, with no `Ctx` at
//! hand (`Element::component`): `Row::new().child(Picture::from_url(..))` is
//! a picture with a state of its own. Such an element is a node that holds
//! what mounts the component and nothing else. Every render's output is
//! searched for them once the render returned, when its `Ctx` is free again,
//! and each is mounted in that `Ctx` as `Ctx::mount` would have mounted it
//! (`Node::mount_declared_components`): the component that renders the
//! element is the one that keeps the component's slot. What was put on the
//! element meanwhile, an id, a key, classes, a wrapper's decoration, is this
//! file's record, and goes to the component's root as it does after `mount`.
//! A tree no component renders (`Tree::set_root` with an element) has no
//! `Ctx` to mount in: what it declares shows nothing, and says so in the log.

use std::sync::{
  Arc,
  atomic::{AtomicBool, Ordering},
};

use crate::{
  app::ctx::Ctx,
  node::{Element, Node},
};

/// Whether any element of the process ever declared a component. Until one does, no render's output is searched.
static DECLARED_IN_USE: AtomicBool = AtomicBool::new(false);

/// What mounts a declared component: in the `Ctx` of the render it is found in, by the key it was given.
pub(crate) type DeclaredMount = Arc<dyn Fn(&mut Ctx, Option<&str>) -> Element + Send + Sync>;

/// A component an element declares where it stands: see the module.
#[derive(Clone)]
pub(crate) struct DeclaredComponent {
  pub(crate) mount: DeclaredMount,
  pub(crate) key: Option<Arc<str>>,
}

/// A decoration that can be put on a node again, on a component's fresh root.
pub(crate) type NodeDecoration = Arc<dyn Fn(Node) -> Node + Send + Sync>;

/// The decoration put on a component's root after `mount` returned it. The
/// outer id and key replace the component's own; the outer classes are
/// appended to its own; the decorations run on it in the order they were put.
#[derive(Clone, Default)]
pub(crate) struct OuterDecoration {
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
  classes: Vec<Arc<str>>,
  decorations: Vec<NodeDecoration>,
  /// The theme of the subtree the root begins: `Node::theme_scope` put on it from outside.
  theme: Option<crate::app::theme::Theme>,
  /// The component the node stands for until a render's output is searched: `Element::component`.
  declared: Option<DeclaredComponent>,
}

impl Node {
  /// The decoration record of a component's root (a node `mount` returned),
  /// created on first use; `None` for any other node, which records nothing.
  fn outer_decoration_mut(&mut self) -> Option<&mut OuterDecoration> {
    // A node that declares a component records as its root will: what is put on it goes to that root.
    if self.component_slot_id.is_none() && !self.declares_component() {
      return None;
    }
    Some(self.outer_decoration.get_or_insert_with(Box::default))
  }

  /// A node that stands for `component` until the render that returns it has returned.
  pub(crate) fn declaring(component: DeclaredComponent) -> Self {
    DECLARED_IN_USE.store(true, Ordering::Relaxed);
    let mut node = Node::new();
    node.outer_decoration = Some(Box::new(OuterDecoration {
      declared: Some(component),
      ..OuterDecoration::default()
    }));
    node
  }

  /// Whether the node stands for a component still to be mounted.
  pub(crate) fn declares_component(&self) -> bool {
    self
      .outer_decoration
      .as_deref()
      .is_some_and(|outer| outer.declared.is_some())
  }

  /// Mounts in `ctx` the components this output of its render declares, each in the place of the node that
  /// declared it, in the order of the tree. The root of a component the render mounted is passed by: its own
  /// render mounted what is declared in it.
  pub(crate) fn mount_declared_components(&mut self, ctx: &mut Ctx) {
    if DECLARED_IN_USE.load(Ordering::Relaxed) {
      self.replace_declared(ctx);
    }
  }

  /// Says once in the log that this tree, which no component renders, declares a component: there is no `Ctx`
  /// to mount it in, and it shows nothing.
  pub(crate) fn warn_of_declared_components(&self) {
    if DECLARED_IN_USE.load(Ordering::Relaxed) && self.holds_declared() {
      static SAID: std::sync::Once = std::sync::Once::new();
      SAID.call_once(|| {
        tracing::warn!(
          "an element declares a component (`Element::component`) in a tree no component renders: it shows \
           nothing. Give the window a root component (`Tree::mount_root`)."
        );
      });
    }
  }

  fn holds_declared(&self) -> bool {
    self.declares_component()
      || self.children.iter().any(Node::holds_declared)
      || self
        .overlay_declaration
        .as_deref()
        .is_some_and(|overlay| overlay.node.holds_declared())
      || self
        .modal_declaration
        .as_deref()
        .is_some_and(|modal| modal.node.holds_declared())
  }

  fn replace_declared(&mut self, ctx: &mut Ctx) {
    let declared = self
      .outer_decoration
      .as_deref_mut()
      .and_then(|outer| outer.declared.take());
    if let Some(declared) = declared {
      let marker = std::mem::take(self);
      let key = declared
        .key
        .clone()
        .or_else(|| marker.outer_decoration.as_deref().and_then(|outer| outer.key.clone()));
      // Nothing can change what the mount gives from here on: an unchanged component is held back, and its
      // live nodes are moved over (`component_output.rs`).
      let mut output = ctx
        .mount_holding(|ctx| (declared.mount)(ctx, key.as_deref()))
        .into_node();
      output.adopt_outer_decoration(&marker);
      *self = output;
      return;
    }
    if self.component_slot_id.is_some() {
      return;
    }
    for child in &mut self.children {
      child.replace_declared(ctx);
    }
    if let Some(overlay) = self.overlay_declaration.as_deref_mut() {
      overlay.node.replace_declared(ctx);
    }
    if let Some(modal) = self.modal_declaration.as_deref_mut() {
      modal.node.replace_declared(ctx);
    }
  }

  /// Records an id put on a component's root by the code that mounted it.
  pub(crate) fn record_outer_id(&mut self, id: &Arc<str>) {
    if let Some(outer) = self.outer_decoration_mut() {
      outer.id = Some(id.clone());
    }
  }

  /// Records a key put on a component's root by the code that mounted it.
  pub(crate) fn record_outer_key(&mut self, key: &Arc<str>) {
    if let Some(outer) = self.outer_decoration_mut() {
      outer.key = Some(key.clone());
    }
  }

  /// Records a class put on a component's root by the code that mounted it.
  pub(crate) fn record_outer_class(&mut self, class: &Arc<str>) {
    if let Some(outer) = self.outer_decoration_mut()
      && !outer.classes.contains(class)
    {
      outer.classes.push(class.clone());
    }
  }

  /// Records the theme of a subtree put on a component's root by the code that mounted it.
  pub(crate) fn record_outer_theme(&mut self, theme: &crate::app::theme::Theme) {
    if let Some(outer) = self.outer_decoration_mut() {
      outer.theme = Some(theme.clone());
    }
  }

  /// Records a decoration put on a component's root by a wrapper of it.
  pub(crate) fn record_outer_decoration(&mut self, decorate: &NodeDecoration) {
    if let Some(outer) = self.outer_decoration_mut() {
      outer.decorations.push(decorate.clone());
    }
  }

  /// Whether what was put on this root and on `other` from outside is the same and is all told by values: an
  /// id, a key, classes, and no decoration of a wrapper, which nothing compares.
  pub(crate) fn same_plain_outer_decoration(&self, other: &Node) -> bool {
    type Plain<'a> = (Option<&'a Arc<str>>, Option<&'a Arc<str>>, &'a [Arc<str>]);
    fn plain(node: &Node) -> Option<Plain<'_>> {
      match node.outer_decoration.as_deref() {
        None => Some((None, None, &[])),
        Some(outer) if outer.decorations.is_empty() && outer.declared.is_none() && outer.theme.is_none() => {
          Some((outer.id.as_ref(), outer.key.as_ref(), outer.classes.as_slice()))
        }
        Some(_) => None,
      }
    }
    matches!((plain(self), plain(other)), (Some(this), Some(that)) if this == that)
  }

  /// Puts the decoration of `replaced`, the decorated root this node replaces
  /// after its component rendered again by itself, on this node, and keeps
  /// it recorded for the component's next render.
  pub(crate) fn adopt_outer_decoration(&mut self, replaced: &Node) {
    let Some(outer) = replaced.outer_decoration.as_deref() else {
      return;
    };
    if let Some(id) = &outer.id {
      self.element_id = Some(id.clone());
    }
    if let Some(key) = &outer.key {
      self.component_key = Some(key.clone());
    }
    for class in &outer.classes {
      self.push_class(class.clone());
    }
    for decorate in &outer.decorations {
      *self = decorate(std::mem::take(self));
    }
    if let Some(theme) = &outer.theme {
      self.theme_scope = Some(Box::new(theme.clone()));
    }
    self.outer_decoration = Some(Box::new(outer.clone()));
  }
}
