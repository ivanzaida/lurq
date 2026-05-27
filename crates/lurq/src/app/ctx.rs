use std::{
  any::Any,
  sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
  },
};

use super::{component::Component, theme::Theme};
use crate::{
  core::{
    ContextMap, ElementRef, ElementRefMut, ReactiveContext, Store, cell_ref::Ref, effect::Effect, memo::Memo,
    signal::Signal, tracking,
  },
  node::{Element, Node},
};

static NEXT_COMPONENT_SLOT_ID: AtomicU64 = AtomicU64::new(1);

fn next_component_slot_id() -> u64 {
  NEXT_COMPONENT_SLOT_ID.fetch_add(1, Ordering::Relaxed)
}

pub struct Ctx {
  dirty: Arc<AtomicBool>,
  props: Option<Box<dyn Any + Send>>,
  theme: Option<Theme>,
  context_map: ContextMap,
  slot_children: Option<Vec<Element>>,
  children: Vec<ChildSlot>,
  child_cursor: usize,
  element_ref_cursor: usize,
  watch_handles: Vec<Box<dyn Any + Send + Sync>>,
  render_watch_handles: Vec<Box<dyn Any + Send + Sync>>,
  effects: Vec<Effect>,
  element_refs: Vec<ElementRefMut>,
  rendering: bool,
}

struct ChildSlot {
  id: u64,
  key: Option<String>,
  component: Box<dyn AnyComponent>,
  ctx: Ctx,
  rendered: Option<Node>,
  mounted: bool,
}

trait AnyComponent: Send + Sync + 'static {
  fn render(&self, ctx: &mut Ctx) -> Element;
  fn on_mounted(&self);
  fn on_unmounted(&self);
  fn type_name(&self) -> &'static str;
}

struct ComponentWrapper<C: Component> {
  component: C,
}

impl<C: Component> AnyComponent for ComponentWrapper<C> {
  fn render(&self, ctx: &mut Ctx) -> Element {
    self.component.render(ctx).into()
  }

  fn on_mounted(&self) {
    self.component.on_mounted();
  }

  fn on_unmounted(&self) {
    self.component.on_unmounted();
  }

  fn type_name(&self) -> &'static str {
    std::any::type_name::<C>()
  }
}

impl Ctx {
  pub fn new_root() -> Self {
    Self::new()
  }

  pub(crate) fn new() -> Self {
    Self {
      dirty: Arc::new(AtomicBool::new(true)),
      props: None,
      theme: None,
      context_map: ContextMap::default(),
      slot_children: None,
      children: Vec::new(),
      child_cursor: 0,
      element_ref_cursor: 0,
      watch_handles: Vec::new(),
      render_watch_handles: Vec::new(),
      effects: Vec::new(),
      element_refs: Vec::new(),
      rendering: false,
    }
  }

  pub(crate) fn with_theme(mut self, theme: Theme) -> Self {
    self.theme = Some(theme);
    self
  }

  pub fn is_dirty(&self) -> bool {
    self.dirty.load(Ordering::Relaxed)
  }

  pub(crate) fn clear_dirty(&self) {
    self.dirty.store(false, Ordering::Relaxed);
  }

  pub fn props<T: Send + PartialEq + 'static>(&self) -> &T {
    self
      .props
      .as_ref()
      .and_then(|props| props.downcast_ref::<T>())
      .expect("component props are not available for this type")
  }

  fn set_props<T: Send + PartialEq + 'static>(&mut self, props: T) {
    self.props = Some(Box::new(props));
  }

  pub(crate) fn set_root_props<T: Send + PartialEq + 'static>(&mut self, props: T) {
    self.set_props(props);
  }

  fn props_changed<T: Send + PartialEq + 'static>(&self, props: &T) -> bool {
    self.props.as_ref().and_then(|existing| existing.downcast_ref::<T>()) != Some(props)
  }

  // --- Reactive primitives ---

  pub fn signal<T: Send + Sync + 'static>(&mut self, initial: T) -> Signal<T> {
    let sig = Signal::new(initial);
    let dirty = self.dirty.clone();
    let handle = sig.watch(move || {
      dirty.store(true, Ordering::Relaxed);
    });
    self.watch_handles.push(Box::new(handle));
    sig
  }

  pub fn memo<T: Clone + PartialEq + Send + Sync + 'static>(
    &mut self,
    f: impl Fn() -> T + Send + Sync + 'static,
  ) -> Memo<T> {
    Memo::new(f)
  }

  pub fn create_ref<T: Send + Sync + 'static>(&self, initial: T) -> Ref<T> {
    Ref::new(initial)
  }

  pub fn on_effect(&mut self, f: impl Fn() + Send + Sync + 'static) {
    self.effects.push(Effect::new(f));
  }

  pub fn watch<T: Send + Sync + 'static>(&mut self, signal: &Signal<T>, f: impl Fn(&T) + Send + Sync + 'static) {
    let sub = signal.subscribe(f);
    self.watch_handles.push(Box::new(sub));
  }

  // --- Store + Lenses ---

  pub fn store<T: Clone + Send + Sync + 'static>(&mut self, initial: T) -> Store<T> {
    let store = Store::new(initial);
    let dirty = self.dirty.clone();
    let handle = store.signal().watch(move || {
      dirty.store(true, Ordering::Relaxed);
    });
    self.watch_handles.push(Box::new(handle));
    store
  }

  // --- Context (Dependency Injection) ---

  pub fn provide<T: Clone + Send + Sync + 'static>(&mut self, value: T) {
    self.context_map.provide(value);
  }

  pub fn use_context<T: Clone + Send + Sync + 'static>(&self) -> Option<T> {
    self.context_map.get::<T>()
  }

  pub fn create_context<T: Clone + std::hash::Hash + Send + Sync + 'static>(&mut self, value: T) -> ReactiveContext<T> {
    let ctx = ReactiveContext::new(value);
    self.context_map.provide(ctx.clone());
    let dirty = self.dirty.clone();
    ctx.subscribe(move || {
      dirty.store(true, Ordering::Relaxed);
    });
    ctx
  }

  pub fn consume_context<T: Clone + std::hash::Hash + Send + Sync + 'static>(&mut self) -> Option<ReactiveContext<T>> {
    let ctx = self.context_map.get::<ReactiveContext<T>>()?;
    let dirty = self.dirty.clone();
    ctx.subscribe(move || {
      dirty.store(true, Ordering::Relaxed);
    });
    Some(ctx)
  }

  // --- Theme ---

  pub fn theme(&self) -> &Theme {
    self.theme.as_ref().expect("theme not set")
  }

  // --- Slot Children ---

  pub fn children(&self) -> &[Element] {
    self.slot_children.as_deref().unwrap_or(&[])
  }

  pub fn has_children(&self) -> bool {
    self.slot_children.as_ref().is_some_and(|c| !c.is_empty())
  }

  // --- Helpers ---

  pub fn element_ref(&mut self) -> ElementRef {
    self.element_ref_mut().as_ref()
  }

  pub fn element_ref_mut(&mut self) -> ElementRefMut {
    if !self.rendering {
      return ElementRefMut::new();
    }

    let cursor = self.element_ref_cursor;
    self.element_ref_cursor += 1;

    if cursor == self.element_refs.len() {
      self.element_refs.push(ElementRefMut::new());
    }

    self.element_refs[cursor].clone()
  }

  pub fn interaction(&self) -> crate::node::interaction_state::InteractionState {
    crate::node::interaction_state::InteractionState::new()
  }

  // --- Component mounting ---

  pub fn mount<C: Component>(&mut self, props: C::Props) -> Element {
    self.mount_inner::<C>(None, props, None)
  }

  pub fn mount_keyed<C: Component>(&mut self, key: &str, props: C::Props) -> Element {
    self.mount_inner::<C>(Some(key), props, None)
  }

  pub fn mount_with<C: Component>(&mut self, props: C::Props, slot_children: Vec<Element>) -> Element {
    self.mount_inner::<C>(None, props, Some(slot_children))
  }

  pub fn mount_keyed_with<C: Component>(&mut self, key: &str, props: C::Props, slot_children: Vec<Element>) -> Element {
    self.mount_inner::<C>(Some(key), props, Some(slot_children))
  }

  fn mount_inner<C: Component>(
    &mut self,
    key: Option<&str>,
    props: C::Props,
    slot_children: Option<Vec<Element>>,
  ) -> Element {
    let cursor = self.child_cursor;
    self.child_cursor += 1;

    let type_name = std::any::type_name::<C>();
    let can_reuse = self.children.get(cursor).is_some_and(|slot| {
      let key_match = match key {
        Some(k) => slot.key.as_deref() == Some(k),
        None => slot.key.is_none(),
      };
      key_match && slot.component.type_name() == type_name
    });

    if can_reuse {
      let slot = &mut self.children[cursor];
      let has_slot_children = slot.ctx.slot_children.is_some() || slot_children.is_some();
      let props_changed = slot.ctx.props_changed(&props);
      slot.ctx.slot_children = slot_children;
      if props_changed {
        slot.ctx.set_props(props);
      }
      if has_slot_children || props_changed || slot.ctx.any_dirty() || slot.rendered.is_none() {
        slot.ctx.begin_render();
        let mut element = slot.component.render(&mut slot.ctx);
        slot.ctx.end_render();
        element.node.set_component_slot_id(slot.id);
        slot.rendered = Some(element.node.clone_for_reuse());
        return element;
      }
      return Element::from_node(slot.rendered.as_ref().unwrap().clone_for_reuse());
    }

    let mut child_ctx = Ctx::new();
    child_ctx.theme = self.theme.clone();
    child_ctx.context_map = self.context_map.clone();
    child_ctx.slot_children = slot_children;
    child_ctx.set_props(props);
    let component = C::create(&mut child_ctx);
    let wrapper = ComponentWrapper { component };
    child_ctx.begin_render();
    let mut element = wrapper.render(&mut child_ctx);
    child_ctx.end_render();
    let slot_id = next_component_slot_id();
    element.node.set_component_slot_id(slot_id);

    let slot = ChildSlot {
      id: slot_id,
      key: key.map(str::to_owned),
      component: Box::new(wrapper),
      ctx: child_ctx,
      rendered: Some(element.node.clone_for_reuse()),
      mounted: false,
    };

    self.set_child_slot(cursor, slot);

    element
  }

  // --- Keyed list rendering ---

  pub fn for_each<T, K, KF, CF>(
    &mut self,
    items: impl IntoIterator<Item = T>,
    key_fn: KF,
    component_fn: CF,
  ) -> Vec<Element>
  where
    K: std::fmt::Display,
    KF: Fn(&T) -> K,
    CF: Fn(&mut Ctx, T) -> Element,
  {
    items
      .into_iter()
      .map(|item| {
        let key = format!("{}", key_fn(&item));
        let cursor = self.child_cursor;
        self.child_cursor += 1;

        let can_reuse = self
          .children
          .get(cursor)
          .is_some_and(|slot| slot.key.as_deref() == Some(&key));

        if can_reuse {
          let slot = &mut self.children[cursor];
          slot.ctx.begin_render();
          let mut element = component_fn(&mut slot.ctx, item);
          slot.ctx.end_render();
          element.node.set_component_slot_id(slot.id);
          slot.rendered = Some(element.node.clone_for_reuse());
          return element;
        }

        let mut child_ctx = Ctx::new();
        child_ctx.theme = self.theme.clone();
        child_ctx.context_map = self.context_map.clone();
        child_ctx.begin_render();
        let mut element = component_fn(&mut child_ctx, item);
        child_ctx.end_render();
        let slot_id = next_component_slot_id();
        element.node.set_component_slot_id(slot_id);

        let slot = ChildSlot {
          id: slot_id,
          key: Some(key),
          component: Box::new(ForEachSlot),
          ctx: child_ctx,
          rendered: Some(element.node.clone_for_reuse()),
          mounted: false,
        };

        self.set_child_slot(cursor, slot);

        element
      })
      .collect()
  }

  // --- Error boundary ---

  pub fn error_boundary(
    &mut self,
    component_fn: impl FnOnce(&mut Ctx) -> Element,
    fallback_fn: impl FnOnce() -> Element,
  ) -> Element {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| component_fn(self)));
    match result {
      Ok(node) => node,
      Err(_) => fallback_fn(),
    }
  }

  // --- Render lifecycle ---

  pub fn begin_render(&mut self) {
    self.child_cursor = 0;
    self.element_ref_cursor = 0;
    self.render_watch_handles.clear();
    tracking::start_tracking();
    self.rendering = true;
  }

  fn set_child_slot(&mut self, cursor: usize, slot: ChildSlot) {
    if cursor < self.children.len() {
      self.children[cursor].component.on_unmounted();
      self.children[cursor] = slot;
    } else {
      self.children.push(slot);
    }
  }

  pub(crate) fn end_render(&mut self) {
    for slot in &self.children[self.child_cursor..] {
      slot.component.on_unmounted();
    }
    self.children.truncate(self.child_cursor);

    for slot in &mut self.children {
      if !slot.mounted {
        slot.component.on_mounted();
        slot.mounted = true;
      }
    }

    self.element_refs.truncate(self.element_ref_cursor);
    self.rendering = false;
    let deps = tracking::stop_tracking();
    let dirty = self.dirty.clone();
    self.render_watch_handles = deps
      .into_iter()
      .map(|dep| {
        let dirty = dirty.clone();
        let handle = (dep.subscribe_fn)(Arc::new(move || {
          dirty.store(true, Ordering::Relaxed);
        }));
        Box::new(handle) as Box<dyn Any + Send + Sync>
      })
      .collect();
    self.clear_dirty();
  }

  pub(crate) fn any_dirty(&self) -> bool {
    if self.is_dirty() {
      return true;
    }
    self.children.iter().any(|slot| slot.ctx.any_dirty())
  }

  pub(crate) fn refresh_dirty_subtrees(&mut self) -> Vec<(u64, Node)> {
    let mut replacements = Vec::new();

    for slot in &mut self.children {
      if !slot.ctx.any_dirty() {
        continue;
      }

      if slot.ctx.is_dirty() {
        let old_rendered = slot.rendered.take();
        slot.ctx.begin_render();
        let mut element = slot.component.render(&mut slot.ctx);
        slot.ctx.end_render();
        element.node.set_component_slot_id(slot.id);
        if let Some(old) = old_rendered.as_ref() {
          element.node.preserve_runtime_state_from(old);
        }
        slot.rendered = Some(element.node.clone_for_reuse());
      } else {
        let nested_replacements = slot.ctx.refresh_dirty_subtrees();
        if let Some(rendered) = &mut slot.rendered {
          for (slot_id, replacement) in nested_replacements {
            rendered.replace_component_slot(slot_id, replacement);
          }
        }
      }

      if let Some(rendered) = &slot.rendered {
        replacements.push((slot.id, rendered.clone_for_reuse()));
      }
    }

    replacements
  }

  pub(crate) fn estimated_memory_bytes(&self) -> usize {
    std::mem::size_of::<Self>()
      + self
        .props
        .as_ref()
        .map(|_| std::mem::size_of::<Box<dyn Any + Send>>())
        .unwrap_or(0)
      + self.children.capacity() * std::mem::size_of::<ChildSlot>()
      + self.watch_handles.capacity() * std::mem::size_of::<Box<dyn Any + Send + Sync>>()
      + self.render_watch_handles.capacity() * std::mem::size_of::<Box<dyn Any + Send + Sync>>()
      + self.effects.capacity() * std::mem::size_of::<Effect>()
      + self.element_refs.capacity() * std::mem::size_of::<ElementRefMut>()
      + self
        .children
        .iter()
        .map(ChildSlot::estimated_memory_bytes)
        .sum::<usize>()
  }
}

struct ForEachSlot;

impl AnyComponent for ForEachSlot {
  fn render(&self, _ctx: &mut Ctx) -> Element {
    Element::new()
  }
  fn on_mounted(&self) {}
  fn on_unmounted(&self) {}
  fn type_name(&self) -> &'static str {
    "ForEachSlot"
  }
}

impl ChildSlot {
  fn estimated_memory_bytes(&self) -> usize {
    self.key.as_ref().map(|key| key.capacity()).unwrap_or(0)
      + std::mem::size_of::<Box<dyn AnyComponent>>()
      + self.ctx.estimated_memory_bytes()
  }
}

impl Drop for Ctx {
  fn drop(&mut self) {
    for slot in &self.children {
      slot.component.on_unmounted();
    }
  }
}
