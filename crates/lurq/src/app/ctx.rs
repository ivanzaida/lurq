use std::{
  any::Any,
  sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
  },
};

use super::{component::Component, theme::Theme};
use crate::{
  core::{cell_ref::Ref, effect::Effect, memo::Memo, signal::Signal},
  node::node::Node,
};

pub struct Ctx {
  dirty: Arc<AtomicBool>,
  theme: Option<Theme>,
  children: Vec<ChildSlot>,
  child_cursor: usize,
  watch_handles: Vec<Box<dyn Any + Send + Sync>>,
  effects: Vec<Effect>,
  mounted: bool,
}

struct ChildSlot {
  key: Option<String>,
  component: Box<dyn AnyComponent>,
  ctx: Ctx,
}

trait AnyComponent: Send + Sync + 'static {
  fn render(&self, ctx: &mut Ctx) -> Node;
  fn on_mounted(&self);
  fn on_unmounted(&self);
  fn type_name(&self) -> &'static str;
}

struct ComponentWrapper<C: Component> {
  component: C,
}

impl<C: Component> AnyComponent for ComponentWrapper<C> {
  fn render(&self, ctx: &mut Ctx) -> Node {
    self.component.render(ctx)
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
      theme: None,
      children: Vec::new(),
      child_cursor: 0,
      watch_handles: Vec::new(),
      effects: Vec::new(),
      mounted: false,
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

  pub(crate) fn mark_dirty(&self) {
    self.dirty.store(true, Ordering::Relaxed);
  }

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

  pub fn node_ref(&self) -> crate::core::NodeRef {
    crate::core::NodeRef::new()
  }

  pub fn theme(&self) -> &Theme {
    self.theme.as_ref().expect("theme not set")
  }

  pub fn interaction(&self) -> crate::node::interaction_state::InteractionState {
    crate::node::interaction_state::InteractionState::new()
  }

  pub fn on_effect(&mut self, f: impl Fn() + Send + Sync + 'static) {
    self.effects.push(Effect::new(f));
  }

  pub fn watch<T: Send + Sync + 'static>(&mut self, signal: &Signal<T>, f: impl Fn(&T) + Send + Sync + 'static) {
    let sub = signal.subscribe(f);
    self.watch_handles.push(Box::new(sub));
  }

  pub fn mount<C: Component>(&mut self, props: C::Props) -> Node {
    let cursor = self.child_cursor;
    self.child_cursor += 1;

    let type_name = std::any::type_name::<C>();
    let can_reuse = self
      .children
      .get(cursor)
      .is_some_and(|slot| slot.key.is_none() && slot.component.type_name() == type_name);

    if can_reuse {
      let slot = &mut self.children[cursor];
      slot.ctx.begin_render();
      return slot.component.render(&mut slot.ctx);
    }

    let mut child_ctx = Ctx::new();
    if let Some(ref t) = self.theme {
      child_ctx.theme = Some(t.clone());
    }
    let component = C::create(&mut child_ctx, props);
    let wrapper = ComponentWrapper { component };
    child_ctx.begin_render();
    let node = wrapper.render(&mut child_ctx);

    let slot = ChildSlot {
      key: None,
      component: Box::new(wrapper),
      ctx: child_ctx,
    };

    if cursor < self.children.len() {
      self.children[cursor] = slot;
    } else {
      self.children.push(slot);
    }

    node
  }

  pub fn mount_keyed<C: Component>(&mut self, key: &str, props: C::Props) -> Node {
    let cursor = self.child_cursor;
    self.child_cursor += 1;

    let type_name = std::any::type_name::<C>();
    let can_reuse = self
      .children
      .get(cursor)
      .is_some_and(|slot| slot.key.as_deref() == Some(key) && slot.component.type_name() == type_name);

    if can_reuse {
      let slot = &mut self.children[cursor];
      slot.ctx.begin_render();
      return slot.component.render(&mut slot.ctx);
    }

    let mut child_ctx = Ctx::new();
    if let Some(ref t) = self.theme {
      child_ctx.theme = Some(t.clone());
    }
    let component = C::create(&mut child_ctx, props);
    let wrapper = ComponentWrapper { component };
    child_ctx.begin_render();
    let node = wrapper.render(&mut child_ctx);

    let slot = ChildSlot {
      key: Some(key.to_owned()),
      component: Box::new(wrapper),
      ctx: child_ctx,
    };

    if cursor < self.children.len() {
      self.children[cursor] = slot;
    } else {
      self.children.push(slot);
    }

    node
  }

  pub fn begin_render(&mut self) {
    self.child_cursor = 0;
  }

  pub(crate) fn end_render(&mut self) {
    self.children.truncate(self.child_cursor);

    if !self.mounted {
      self.mounted = true;
      for slot in &self.children {
        slot.component.on_mounted();
      }
    }
  }

  pub(crate) fn any_dirty(&self) -> bool {
    if self.is_dirty() {
      return true;
    }
    self.children.iter().any(|slot| slot.ctx.any_dirty())
  }

  pub(crate) fn estimated_memory_bytes(&self) -> usize {
    std::mem::size_of::<Self>()
      + self.children.capacity() * std::mem::size_of::<ChildSlot>()
      + self.watch_handles.capacity() * std::mem::size_of::<Box<dyn Any + Send + Sync>>()
      + self.effects.capacity() * std::mem::size_of::<Effect>()
      + self
        .children
        .iter()
        .map(ChildSlot::estimated_memory_bytes)
        .sum::<usize>()
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
