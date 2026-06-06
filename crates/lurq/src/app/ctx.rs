#[cfg(feature = "devtools")]
use std::sync::atomic::AtomicUsize;
#[cfg(feature = "tokio")]
use std::sync::mpsc::{self, TryRecvError};
#[cfg(feature = "devtools")]
use std::time::{SystemTime, UNIX_EPOCH};
use std::{
  any::Any,
  collections::HashSet,
  future::Future,
  pin::Pin,
  ptr::NonNull,
  sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
  },
  task::{Context as TaskContext, Poll, Wake, Waker},
  time::{Duration, Instant},
};

use parking_lot::Mutex;

#[cfg(feature = "devtools")]
use super::component::{ComponentInfo, DevtoolsInspectable};
#[cfg(feature = "i18n")]
use super::i18n::I18n;
use super::{app_state::App, component::Component, theme::Theme};
use crate::{
  core::{
    ContextMap, ElementRef, ElementRefMut, ReactiveContext, Store,
    cell_ref::Ref,
    effect::Effect,
    memo::Memo,
    signal::{Signal, SignalValue},
    tracking,
  },
  node::{Element, Node},
};

static NEXT_COMPONENT_SLOT_ID: AtomicU64 = AtomicU64::new(1);

fn next_component_slot_id() -> u64 {
  NEXT_COMPONENT_SLOT_ID.fetch_add(1, Ordering::Relaxed)
}

#[cfg(feature = "devtools")]
fn set_component_debug_metadata(node: &mut Node, ctx: &Ctx) {
  node.set_component_props_debug(ctx.props_debug());
  node.set_component_signals_debug(ctx.signals_debug());
  node.set_component_memos_debug(ctx.memos_debug());
  node.set_component_effects_debug(ctx.effects_debug());
  node.set_component_contexts_debug(ctx.contexts_debug());
}

fn attach_component_metadata(
  mut node: Node,
  tag_name: Arc<str>,
  slot_id: u64,
  key: Option<&str>,
  #[cfg(feature = "devtools")] ctx: &Ctx,
) -> Node {
  if node.component_slot_id().is_some() {
    node = Node::logical().child(node);
  }

  node.set_tag_name(tag_name);
  node.set_component_slot_id(slot_id);
  node.set_component_key(key);
  #[cfg(feature = "devtools")]
  set_component_debug_metadata(&mut node, ctx);
  node
}

pub(crate) fn component_tag_name<C: 'static>() -> Arc<str> {
  let type_name = std::any::type_name::<C>();
  let base = type_name.split('<').next().unwrap_or(type_name);
  Arc::from(base.rsplit("::").next().unwrap_or(base))
}

struct ModalEntry {
  scope_id: u64,
  cursor: usize,
  node: Node,
}

#[derive(Clone)]
pub struct ModalContext {
  open: Signal<bool>,
}

impl ModalContext {
  pub fn open(&self) {
    self.open.set(true);
  }

  pub fn close(&self) {
    self.open.set(false);
  }

  pub fn is_open(&self) -> bool {
    self.open.get()
  }

  pub fn signal(&self) -> Signal<bool> {
    self.open.clone()
  }
}

#[derive(Clone)]
pub struct Timeout {
  timer: Timer,
}

#[derive(Clone)]
pub struct Interval {
  timer: Timer,
}

#[derive(Clone)]
struct Timer {
  inner: Arc<Mutex<TimerInner>>,
}

struct TimerInner {
  duration: Duration,
  repeat: bool,
  next_fire: Option<Instant>,
  callback: Arc<dyn Fn() + Send + Sync>,
}

impl Timeout {
  pub fn start(&self) {
    self.timer.start();
  }

  pub fn restart(&self) {
    self.timer.restart();
  }

  pub fn cancel(&self) {
    self.timer.stop();
  }

  pub fn is_active(&self) -> bool {
    self.timer.is_active()
  }
}

impl Interval {
  pub fn start(&self) {
    self.timer.start();
  }

  pub fn restart(&self) {
    self.timer.restart();
  }

  pub fn stop(&self) {
    self.timer.stop();
  }

  pub fn is_active(&self) -> bool {
    self.timer.is_active()
  }
}

impl Timer {
  fn new(duration: Duration, repeat: bool, callback: impl Fn() + Send + Sync + 'static) -> Self {
    Self {
      inner: Arc::new(Mutex::new(TimerInner {
        duration,
        repeat,
        next_fire: None,
        callback: Arc::new(callback),
      })),
    }
  }

  fn start(&self) {
    let now = Instant::now();
    let mut inner = self.inner.lock();
    if inner.next_fire.is_none() {
      inner.next_fire = Some(now + inner.duration);
    }
  }

  fn restart(&self) {
    self.restart_at(Instant::now());
  }

  fn restart_at(&self, now: Instant) {
    let mut inner = self.inner.lock();
    inner.next_fire = Some(now + inner.duration);
  }

  fn stop(&self) {
    self.inner.lock().next_fire = None;
  }

  fn is_active(&self) -> bool {
    self.inner.lock().next_fire.is_some()
  }

  fn tick(&self, now: Instant) -> bool {
    let callback = {
      let mut inner = self.inner.lock();
      let Some(next_fire) = inner.next_fire else {
        return false;
      };
      if now < next_fire {
        return false;
      }
      if inner.repeat {
        inner.next_fire = Some(now + inner.duration);
      } else {
        inner.next_fire = None;
      }
      inner.callback.clone()
    };
    callback();
    true
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, crate::DevtoolsInspectable)]
pub enum FutureStatus {
  Idle,
  Pending,
  Fulfilled,
  Rejected,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FutureState<T, E> {
  pub status: FutureStatus,
  pub data: Option<T>,
  pub error: Option<E>,
}

impl<T, E> crate::app::component::DevtoolsInspectable for FutureState<T, E>
where
  T: crate::app::component::DevtoolsInspectable,
  E: crate::app::component::DevtoolsInspectable,
{
  fn write_info(&self, buffer: &mut Vec<crate::app::component::ComponentInfo>) {
    let mut children = Vec::new();
    crate::app::component::DevtoolsInspectable::write_info(&self.status, &mut children);
    crate::app::component::DevtoolsInspectable::write_info(&self.data, &mut children);
    crate::app::component::DevtoolsInspectable::write_info(&self.error, &mut children);
    buffer.push(crate::app::component::ComponentInfo::with_children(
      "FutureState",
      std::any::type_name::<Self>(),
      children,
    ));
  }
}

impl<T, E> FutureState<T, E> {
  pub fn idle() -> Self {
    Self {
      status: FutureStatus::Idle,
      data: None,
      error: None,
    }
  }

  pub fn pending(data: Option<T>) -> Self {
    Self {
      status: FutureStatus::Pending,
      data,
      error: None,
    }
  }

  pub fn fulfilled(data: T) -> Self {
    Self {
      status: FutureStatus::Fulfilled,
      data: Some(data),
      error: None,
    }
  }

  pub fn rejected(error: E, data: Option<T>) -> Self {
    Self {
      status: FutureStatus::Rejected,
      data,
      error: Some(error),
    }
  }

  pub fn is_idle(&self) -> bool {
    self.status == FutureStatus::Idle
  }

  pub fn is_pending(&self) -> bool {
    self.status == FutureStatus::Pending
  }

  pub fn is_fulfilled(&self) -> bool {
    self.status == FutureStatus::Fulfilled
  }

  pub fn is_rejected(&self) -> bool {
    self.status == FutureStatus::Rejected
  }
}

pub struct FutureHandle<T: SignalValue, E: SignalValue> {
  state: Signal<FutureState<T, E>>,
  task: AsyncTask,
}

pub struct FutureAction<A, T: SignalValue, E: SignalValue> {
  state: Signal<FutureState<T, E>>,
  task: AsyncTask,
  runner: Arc<Mutex<ActionRunner<A, T, E>>>,
  runtime_handle: RuntimeFutureHandle,
}

type BoxFutureResult<T, E> = Pin<Box<dyn Future<Output = Result<T, E>> + Send>>;
type ActionRunner<A, T, E> = Arc<dyn Fn(A) -> BoxFutureResult<T, E> + Send + Sync>;
#[cfg(feature = "tokio")]
type FutureCompletion = Box<dyn FnOnce() + Send>;
#[cfg(feature = "tokio")]
type RuntimeFutureHandle = Option<tokio::runtime::Handle>;
#[cfg(not(feature = "tokio"))]
type RuntimeFutureHandle = ();

#[derive(Clone)]
struct AsyncTask {
  inner: Arc<Mutex<AsyncTaskInner>>,
}

#[derive(Default)]
struct AsyncTaskInner {
  future: Option<Pin<Box<dyn Future<Output = ()> + Send>>>,
  #[cfg(feature = "tokio")]
  tokio_task: Option<TokioAsyncTask>,
}

struct FutureSlot {
  deps: Option<Box<dyn Any + Send + Sync>>,
  handle: Box<dyn Any + Send + Sync>,
  task: AsyncTask,
}

#[cfg(feature = "tokio")]
struct TokioAsyncTask {
  join: tokio::task::JoinHandle<()>,
  receiver: mpsc::Receiver<FutureCompletion>,
}

struct NoopWake;

impl Wake for NoopWake {
  fn wake(self: Arc<Self>) {}
}

impl<T: SignalValue, E: SignalValue> Clone for FutureHandle<T, E> {
  fn clone(&self) -> Self {
    Self {
      state: self.state.clone(),
      task: self.task.clone(),
    }
  }
}

impl<A, T: SignalValue, E: SignalValue> Clone for FutureAction<A, T, E> {
  fn clone(&self) -> Self {
    Self {
      state: self.state.clone(),
      task: self.task.clone(),
      runner: self.runner.clone(),
      runtime_handle: self.runtime_handle.clone(),
    }
  }
}

impl<T: SignalValue, E: SignalValue> FutureHandle<T, E> {
  pub fn state(&self) -> Signal<FutureState<T, E>> {
    self.state.clone()
  }

  pub fn cancel(&self) {
    self.task.cancel();
  }

  pub fn is_active(&self) -> bool {
    self.task.is_active()
  }
}

impl<A: Send + Sync + 'static, T, E> FutureAction<A, T, E>
where
  T: SignalValue + Clone + PartialEq + Send + Sync + 'static,
  E: SignalValue + Clone + PartialEq + Send + Sync + 'static,
{
  pub fn state(&self) -> Signal<FutureState<T, E>> {
    self.state.clone()
  }

  pub fn run(&self, args: A) {
    let runner = self.runner.lock().clone();
    let future = runner(args);
    start_future_task(
      self.state.clone(),
      self.task.clone(),
      self.runtime_handle.clone(),
      future,
    );
  }

  pub fn cancel(&self) {
    self.task.cancel();
  }

  pub fn is_active(&self) -> bool {
    self.task.is_active()
  }
}

impl AsyncTask {
  fn new() -> Self {
    Self {
      inner: Arc::new(Mutex::new(AsyncTaskInner::default())),
    }
  }

  fn set(&self, future: Pin<Box<dyn Future<Output = ()> + Send>>) {
    self.cancel();
    self.inner.lock().future = Some(future);
  }

  #[cfg(feature = "tokio")]
  fn set_tokio(&self, join: tokio::task::JoinHandle<()>, receiver: mpsc::Receiver<FutureCompletion>) {
    self.cancel();
    self.inner.lock().tokio_task = Some(TokioAsyncTask { join, receiver });
  }

  fn cancel(&self) {
    let mut inner = self.inner.lock();
    inner.future = None;
    #[cfg(feature = "tokio")]
    if let Some(task) = inner.tokio_task.take() {
      task.join.abort();
    }
  }

  fn is_active(&self) -> bool {
    let inner = self.inner.lock();
    inner.future.is_some() || {
      #[cfg(feature = "tokio")]
      {
        inner.tokio_task.is_some()
      }
      #[cfg(not(feature = "tokio"))]
      {
        false
      }
    }
  }

  fn poll(&self, cx: &mut TaskContext<'_>) -> bool {
    if let Some(mut future) = self.inner.lock().future.take() {
      match future.as_mut().poll(cx) {
        Poll::Ready(()) => return true,
        Poll::Pending => {
          let mut inner = self.inner.lock();
          if inner.future.is_none() {
            inner.future = Some(future);
          }
          return false;
        }
      }
    }

    #[cfg(feature = "tokio")]
    {
      let mut completion = None;
      let mut disconnected = false;
      {
        let mut inner = self.inner.lock();
        if let Some(task) = inner.tokio_task.as_mut() {
          match task.receiver.try_recv() {
            Ok(received) => completion = Some(received),
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => disconnected = true,
          }
        }
        if completion.is_some() || disconnected {
          inner.tokio_task = None;
        }
      }

      if let Some(completion) = completion {
        completion();
        return true;
      }
    }

    false
  }
}

fn noop_waker() -> Waker {
  Waker::from(Arc::new(NoopWake))
}

fn start_future_task<T, E>(
  state: Signal<FutureState<T, E>>,
  task: AsyncTask,
  runtime_handle: RuntimeFutureHandle,
  future: BoxFutureResult<T, E>,
) where
  T: SignalValue + Clone + PartialEq + Send + Sync + 'static,
  E: SignalValue + Clone + PartialEq + Send + Sync + 'static,
{
  let previous_data = state.get_untracked().data;
  state.set(FutureState::pending(previous_data));

  #[cfg(feature = "tokio")]
  if let Some(handle) = runtime_handle {
    let completion_state = state.clone();
    let (sender, receiver) = mpsc::channel::<FutureCompletion>();
    let join = handle.spawn(async move {
      let result = future.await;
      let completion: FutureCompletion = Box::new(move || match result {
        Ok(data) => completion_state.set(FutureState::fulfilled(data)),
        Err(error) => {
          let previous_data = completion_state.get_untracked().data;
          completion_state.set(FutureState::rejected(error, previous_data));
        }
      });
      let _ = sender.send(completion);
    });
    task.set_tokio(join, receiver);
    return;
  }

  #[cfg(not(feature = "tokio"))]
  let _ = runtime_handle;

  let completion_state = state.clone();
  task.set(Box::pin(async move {
    match future.await {
      Ok(data) => completion_state.set(FutureState::fulfilled(data)),
      Err(error) => {
        let previous_data = completion_state.get_untracked().data;
        completion_state.set(FutureState::rejected(error, previous_data));
      }
    }
  }));
}

pub struct Ctx {
  dirty: Arc<AtomicBool>,
  batch: Arc<BatchState>,
  props: Option<Box<dyn Any + Send>>,
  #[cfg(feature = "devtools")]
  props_debug: Option<DevtoolsInspectableDebug>,
  #[cfg(feature = "devtools")]
  signals_debug: Vec<ComponentSignalDebug>,
  #[cfg(feature = "devtools")]
  memos_debug: Vec<ComponentMemoDebug>,
  #[cfg(feature = "devtools")]
  effects_debug: Vec<ComponentEffectDebug>,
  #[cfg(feature = "devtools")]
  contexts_debug: Vec<ComponentContextDebug>,
  theme: Option<Theme>,
  window: Option<crate::app::window::Window>,
  #[cfg(feature = "i18n")]
  i18n: Option<I18n>,
  app: Option<NonNull<App>>,
  #[cfg(feature = "tokio")]
  runtime_future_handle: RuntimeFutureHandle,
  context_map: ContextMap,
  slot_children: Option<Vec<Element>>,
  children: Vec<ChildSlot>,
  child_cursor: usize,
  modal_registry: Arc<Mutex<Vec<ModalEntry>>>,
  modal_scope_id: u64,
  modal_cursor: usize,
  modal_context: Option<ModalContext>,
  element_ref_cursor: usize,
  future_cursor: usize,
  watch_handles: Vec<Box<dyn Any + Send + Sync>>,
  render_watch_handles: Vec<Box<dyn Any + Send + Sync>>,
  effects: Vec<Effect>,
  timers: Vec<Timer>,
  future_slots: Vec<FutureSlot>,
  element_refs: Vec<ElementRefMut>,
  rendering: bool,
}

#[cfg(feature = "devtools")]
#[derive(Clone, Debug, PartialEq)]
pub struct DevtoolsInspectableDebug {
  pub type_name: Arc<str>,
  pub fields: Vec<ComponentInfo>,
}

#[cfg(feature = "devtools")]
#[derive(Clone, Debug)]
pub struct ComponentSignalDebug {
  pub id: usize,
  pub type_name: Arc<str>,
  value: Arc<Mutex<Option<Arc<str>>>>,
  history: Arc<Mutex<Vec<ComponentValueChangeDebug>>>,
  subscriber_count: Arc<AtomicUsize>,
}

#[cfg(feature = "devtools")]
#[derive(Clone, Debug)]
pub struct ComponentMemoDebug {
  pub id: usize,
  pub type_name: Arc<str>,
  value: Arc<Mutex<Option<Arc<str>>>>,
  history: Arc<Mutex<Vec<ComponentValueChangeDebug>>>,
  subscriber_count: Arc<AtomicUsize>,
}

#[cfg(feature = "devtools")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComponentValueChangeDebug {
  pub timestamp: String,
  pub from_value: String,
  pub to_value: String,
}

#[cfg(feature = "devtools")]
#[derive(Clone, Debug, PartialEq)]
pub struct ComponentEffectDebug {
  pub id: usize,
}

#[cfg(feature = "devtools")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComponentContextKind {
  Provided,
  Consumed,
}

#[cfg(feature = "devtools")]
#[derive(Clone, Debug, PartialEq)]
pub struct ComponentContextDebug {
  pub kind: ComponentContextKind,
  pub type_name: Arc<str>,
}

#[cfg(feature = "devtools")]
impl DevtoolsInspectableDebug {
  fn from_props<T: DevtoolsInspectable + 'static>(props: &T) -> Self {
    let mut fields = Vec::new();
    props.write_info(&mut fields);
    Self {
      type_name: Arc::from(std::any::type_name::<T>()),
      fields,
    }
  }
}

#[cfg(feature = "devtools")]
impl ComponentSignalDebug {
  pub fn formatted_value(&self) -> Option<String> {
    self.value.lock().as_ref().map(|value| value.to_string())
  }

  pub fn history(&self) -> Vec<ComponentValueChangeDebug> {
    self.history.lock().clone()
  }

  pub fn subscriber_count(&self) -> usize {
    self.subscriber_count.load(Ordering::Relaxed)
  }

  pub(crate) fn estimated_memory_bytes(&self) -> usize {
    let history = self.history.lock();
    self.value.lock().as_ref().map(|value| value.len()).unwrap_or(0)
      + history.capacity() * std::mem::size_of::<ComponentValueChangeDebug>()
      + history
        .iter()
        .map(ComponentValueChangeDebug::estimated_memory_bytes)
        .sum::<usize>()
  }
}

#[cfg(feature = "devtools")]
impl ComponentMemoDebug {
  pub fn formatted_value(&self) -> Option<String> {
    self.value.lock().as_ref().map(|value| value.to_string())
  }

  pub fn history(&self) -> Vec<ComponentValueChangeDebug> {
    self.history.lock().clone()
  }

  pub fn subscriber_count(&self) -> usize {
    self.subscriber_count.load(Ordering::Relaxed)
  }

  pub(crate) fn estimated_memory_bytes(&self) -> usize {
    let history = self.history.lock();
    self.value.lock().as_ref().map(|value| value.len()).unwrap_or(0)
      + history.capacity() * std::mem::size_of::<ComponentValueChangeDebug>()
      + history
        .iter()
        .map(ComponentValueChangeDebug::estimated_memory_bytes)
        .sum::<usize>()
  }
}

#[cfg(feature = "devtools")]
impl ComponentValueChangeDebug {
  fn estimated_memory_bytes(&self) -> usize {
    self.timestamp.capacity() + self.from_value.capacity() + self.to_value.capacity()
  }
}

#[cfg(feature = "devtools")]
impl PartialEq for ComponentSignalDebug {
  fn eq(&self, other: &Self) -> bool {
    self.id == other.id
      && self.type_name == other.type_name
      && self.formatted_value() == other.formatted_value()
      && self.history() == other.history()
      && self.subscriber_count() == other.subscriber_count()
  }
}

#[cfg(feature = "devtools")]
impl PartialEq for ComponentMemoDebug {
  fn eq(&self, other: &Self) -> bool {
    self.id == other.id
      && self.type_name == other.type_name
      && self.formatted_value() == other.formatted_value()
      && self.history() == other.history()
      && self.subscriber_count() == other.subscriber_count()
  }
}

#[cfg(feature = "devtools")]
fn update_debug_value_history(
  current_value: &Arc<Mutex<Option<Arc<str>>>>,
  history: &Arc<Mutex<Vec<ComponentValueChangeDebug>>>,
  next_value: Option<Arc<str>>,
) {
  let mut current_value = current_value.lock();
  if *current_value == next_value {
    return;
  }

  if let Some(previous) = current_value.as_ref() {
    let mut history = history.lock();
    history.push(ComponentValueChangeDebug {
      timestamp: current_compact_timestamp(),
      from_value: previous.to_string(),
      to_value: next_value
        .as_ref()
        .map(|value| value.to_string())
        .unwrap_or_else(|| "<unknown>".to_owned()),
    });
    if history.len() > 64 {
      let overflow = history.len() - 64;
      history.drain(0..overflow);
    }
  }

  *current_value = next_value;
}

#[cfg(feature = "devtools")]
fn current_compact_timestamp() -> String {
  let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
  let millis = duration.subsec_millis();
  let seconds_of_day = duration.as_secs() % 86_400;
  let hour = seconds_of_day / 3_600;
  let minute = seconds_of_day % 3_600 / 60;
  let second = seconds_of_day % 60;
  format!("{hour:02}:{minute:02}:{second:02}.{millis:03}")
}

#[cfg(feature = "devtools")]
fn format_debug_value<T: DevtoolsInspectable + 'static>(value: &T) -> Option<Arc<str>> {
  let mut fields = Vec::new();
  value.write_info(&mut fields);
  format_debug_fields(&fields).map(Arc::from)
}

#[cfg(feature = "devtools")]
fn format_debug_fields(fields: &[ComponentInfo]) -> Option<String> {
  if fields.is_empty() {
    return None;
  }

  if fields.len() == 1 {
    let info = &fields[0];
    if matches!(info.name(), "value" | "variant") {
      if let Some(value) = info.formatted_value() {
        return Some(value.to_owned());
      }
    }
  }

  Some(fields.iter().map(format_debug_info).collect::<Vec<_>>().join(", "))
}

#[cfg(feature = "devtools")]
fn format_debug_info(info: &ComponentInfo) -> String {
  if let Some(value) = info.formatted_value() {
    if info.name() == "value" {
      value.to_owned()
    } else {
      format!("{}: {}", info.name(), value)
    }
  } else if let Some(children) = format_debug_fields(info.children()) {
    if info.name() == "Some" {
      format!("Some({children})")
    } else {
      format!("{}: {{{children}}}", info.name())
    }
  } else {
    format!("{}: {}", info.name(), info.type_name())
  }
}

#[derive(Default)]
struct BatchState {
  inner: Mutex<BatchStateInner>,
}

#[derive(Default)]
struct BatchStateInner {
  depth: usize,
  pending_dirty: Vec<Arc<AtomicBool>>,
}

struct BatchGuard<'a> {
  state: &'a BatchState,
}

impl BatchState {
  fn batch<R>(&self, f: impl FnOnce() -> R) -> R {
    self.inner.lock().depth += 1;
    let guard = BatchGuard { state: self };
    let result = f();
    drop(guard);
    result
  }

  fn mark_dirty(&self, dirty: &Arc<AtomicBool>) {
    let mut inner = self.inner.lock();
    if inner.depth == 0 {
      drop(inner);
      dirty.store(true, Ordering::Relaxed);
      return;
    }

    inner.pending_dirty.push(dirty.clone());
  }

  fn end_batch(&self) {
    let pending = {
      let mut inner = self.inner.lock();
      debug_assert!(inner.depth > 0);
      inner.depth = inner.depth.saturating_sub(1);
      if inner.depth > 0 {
        return;
      }
      std::mem::take(&mut inner.pending_dirty)
    };
    let mut seen = HashSet::new();
    for dirty in pending {
      let ptr = Arc::as_ptr(&dirty) as usize;
      if seen.insert(ptr) {
        dirty.store(true, Ordering::Relaxed);
      }
    }
  }
}

impl Drop for BatchGuard<'_> {
  fn drop(&mut self) {
    self.state.end_batch();
  }
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
  fn tag_name(&self) -> Arc<str>;
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

  fn tag_name(&self) -> Arc<str> {
    component_tag_name::<C>()
  }
}

impl Ctx {
  pub fn new_root() -> Self {
    Self::new()
  }

  pub(crate) fn new() -> Self {
    Self {
      dirty: Arc::new(AtomicBool::new(true)),
      batch: Arc::new(BatchState::default()),
      props: None,
      #[cfg(feature = "devtools")]
      props_debug: None,
      #[cfg(feature = "devtools")]
      signals_debug: Vec::new(),
      #[cfg(feature = "devtools")]
      memos_debug: Vec::new(),
      #[cfg(feature = "devtools")]
      effects_debug: Vec::new(),
      #[cfg(feature = "devtools")]
      contexts_debug: Vec::new(),
      theme: None,
      window: None,
      #[cfg(feature = "i18n")]
      i18n: None,
      app: None,
      #[cfg(feature = "tokio")]
      runtime_future_handle: None,
      context_map: ContextMap::default(),
      slot_children: None,
      children: Vec::new(),
      child_cursor: 0,
      modal_registry: Arc::new(Mutex::new(Vec::new())),
      modal_scope_id: 0,
      modal_cursor: 0,
      modal_context: None,
      element_ref_cursor: 0,
      future_cursor: 0,
      watch_handles: Vec::new(),
      render_watch_handles: Vec::new(),
      effects: Vec::new(),
      timers: Vec::new(),
      future_slots: Vec::new(),
      element_refs: Vec::new(),
      rendering: false,
    }
  }

  pub(crate) fn with_theme(mut self, theme: Theme) -> Self {
    self.theme = Some(theme);
    self
  }

  pub(crate) fn with_window(mut self, window: crate::app::window::Window) -> Self {
    self.window = Some(window);
    self
  }

  #[cfg(feature = "i18n")]
  pub(crate) fn with_i18n(mut self, i18n: I18n) -> Self {
    self.i18n = Some(i18n);
    self
  }

  pub(crate) fn set_app_ref(&mut self, app: &mut App) {
    self.app = Some(NonNull::from(&mut *app));
    #[cfg(feature = "tokio")]
    {
      self.runtime_future_handle = app.tokio_handle();
    }
    for slot in &mut self.children {
      slot.ctx.set_app_ref(app);
    }
  }

  fn runtime_future_handle(&self) -> RuntimeFutureHandle {
    #[cfg(feature = "tokio")]
    {
      self.runtime_future_handle.clone()
    }
    #[cfg(not(feature = "tokio"))]
    {}
  }

  pub fn is_dirty(&self) -> bool {
    self.dirty.load(Ordering::Relaxed)
  }

  pub(crate) fn clear_dirty(&self) {
    self.dirty.store(false, Ordering::Relaxed);
  }

  pub fn batch<R>(&self, f: impl FnOnce() -> R) -> R {
    self.batch.batch(f)
  }

  pub fn props<T: Send + PartialEq + 'static>(&self) -> &T {
    self
      .props
      .as_ref()
      .and_then(|props| props.downcast_ref::<T>())
      .expect("component props are not available for this type")
  }

  #[cfg(feature = "devtools")]
  fn set_props<T: Send + PartialEq + DevtoolsInspectable + 'static>(&mut self, props: T) {
    self.props_debug = Some(DevtoolsInspectableDebug::from_props(&props));
    self.props = Some(Box::new(props));
  }

  #[cfg(not(feature = "devtools"))]
  fn set_props<T: Send + PartialEq + 'static>(&mut self, props: T) {
    self.props = Some(Box::new(props));
  }

  #[cfg(feature = "devtools")]
  pub(crate) fn set_root_props<T: Send + PartialEq + DevtoolsInspectable + 'static>(&mut self, props: T) {
    self.set_props(props);
  }

  #[cfg(not(feature = "devtools"))]
  pub(crate) fn set_root_props<T: Send + PartialEq + 'static>(&mut self, props: T) {
    self.set_props(props);
  }

  #[cfg(feature = "devtools")]
  pub(crate) fn update_root_props<T: Send + PartialEq + DevtoolsInspectable + 'static>(&mut self, props: T) -> bool {
    if !self.props_changed(&props) {
      return false;
    }
    self.set_props(props);
    self.dirty.store(true, Ordering::Relaxed);
    true
  }

  #[cfg(not(feature = "devtools"))]
  pub(crate) fn update_root_props<T: Send + PartialEq + 'static>(&mut self, props: T) -> bool {
    if !self.props_changed(&props) {
      return false;
    }
    self.set_props(props);
    self.dirty.store(true, Ordering::Relaxed);
    true
  }

  fn props_changed<T: Send + PartialEq + 'static>(&self, props: &T) -> bool {
    self.props.as_ref().and_then(|existing| existing.downcast_ref::<T>()) != Some(props)
  }

  #[cfg(feature = "devtools")]
  pub(crate) fn props_debug(&self) -> Option<DevtoolsInspectableDebug> {
    self.props_debug.clone()
  }

  #[cfg(feature = "devtools")]
  pub(crate) fn signals_debug(&self) -> Vec<ComponentSignalDebug> {
    self.signals_debug.clone()
  }

  #[cfg(feature = "devtools")]
  pub(crate) fn memos_debug(&self) -> Vec<ComponentMemoDebug> {
    self.memos_debug.clone()
  }

  #[cfg(feature = "devtools")]
  pub(crate) fn effects_debug(&self) -> Vec<ComponentEffectDebug> {
    self.effects_debug.clone()
  }

  #[cfg(feature = "devtools")]
  pub(crate) fn contexts_debug(&self) -> Vec<ComponentContextDebug> {
    self.contexts_debug.clone()
  }

  // --- Reactive primitives ---

  #[cfg(feature = "devtools")]
  pub fn signal<T: Send + Sync + DevtoolsInspectable + 'static>(&mut self, initial: T) -> Signal<T> {
    let debug_value = Arc::new(Mutex::new(format_debug_value(&initial)));
    let debug_history = Arc::new(Mutex::new(Vec::new()));
    let sig = Signal::new(initial);
    self.signals_debug.push(ComponentSignalDebug {
      id: sig.id(),
      type_name: Arc::from(std::any::type_name::<T>()),
      value: debug_value.clone(),
      history: debug_history.clone(),
      subscriber_count: sig.devtools_subscriber_count(),
    });
    let debug_handle = sig.subscribe_debug(move |value| {
      update_debug_value_history(&debug_value, &debug_history, format_debug_value(value));
    });
    let dirty = self.dirty.clone();
    let batch = self.batch.clone();
    let handle = sig.watch(move || {
      batch.mark_dirty(&dirty);
    });
    self.watch_handles.push(Box::new(debug_handle));
    self.watch_handles.push(Box::new(handle));
    sig
  }

  #[cfg(not(feature = "devtools"))]
  pub fn signal<T: SignalValue + Send + Sync + 'static>(&mut self, initial: T) -> Signal<T> {
    let sig = Signal::new(initial);
    let dirty = self.dirty.clone();
    let batch = self.batch.clone();
    let handle = sig.watch(move || {
      batch.mark_dirty(&dirty);
    });
    self.watch_handles.push(Box::new(handle));
    sig
  }

  #[cfg(feature = "devtools")]
  pub fn memo<T: Clone + PartialEq + Send + Sync + DevtoolsInspectable + 'static>(
    &mut self,
    f: impl Fn() -> T + Send + Sync + 'static,
  ) -> Memo<T> {
    let memo = Memo::new(f);
    let debug_value = Arc::new(Mutex::new(memo.with(|value| format_debug_value(value))));
    let debug_history = Arc::new(Mutex::new(Vec::new()));
    self.memos_debug.push(ComponentMemoDebug {
      id: memo.id(),
      type_name: Arc::from(std::any::type_name::<T>()),
      value: debug_value.clone(),
      history: debug_history.clone(),
      subscriber_count: memo.devtools_subscriber_count(),
    });
    let debug_handle = memo.subscribe(move |value| {
      update_debug_value_history(&debug_value, &debug_history, format_debug_value(value));
    });
    self.watch_handles.push(Box::new(debug_handle));
    memo
  }

  #[cfg(not(feature = "devtools"))]
  pub fn memo<T: SignalValue + Clone + PartialEq + Send + Sync + 'static>(
    &mut self,
    f: impl Fn() -> T + Send + Sync + 'static,
  ) -> Memo<T> {
    Memo::new(f)
  }

  pub fn create_ref<T: Send + Sync + 'static>(&self, initial: T) -> Ref<T> {
    Ref::new(initial)
  }

  pub fn on_effect(&mut self, f: impl Fn() + Send + Sync + 'static) {
    let effect = Effect::new(f);
    #[cfg(feature = "devtools")]
    self.effects_debug.push(ComponentEffectDebug { id: effect.id() });
    self.effects.push(effect);
  }

  pub fn create_timeout(&mut self, duration: Duration, f: impl Fn() + Send + Sync + 'static) -> Timeout {
    let timer = Timer::new(duration, false, f);
    self.timers.push(timer.clone());
    Timeout { timer }
  }

  pub fn create_interval(&mut self, duration: Duration, f: impl Fn() + Send + Sync + 'static) -> Interval {
    let timer = Timer::new(duration, true, f);
    self.timers.push(timer.clone());
    Interval { timer }
  }

  #[cfg(feature = "form")]
  pub fn form(&mut self, options: crate::components::FormOptions) -> crate::components::FormHandle {
    let dirty = self.dirty.clone();
    let batch = self.batch.clone();
    crate::components::FormHandle::with_dirty(
      options,
      Arc::new(move || {
        batch.mark_dirty(&dirty);
      }),
    )
  }

  #[cfg(feature = "form")]
  pub fn form_view<R>(&mut self, form: crate::components::FormHandle, render: impl FnOnce(&mut Ctx) -> R) -> Element
  where
    R: Into<Element>,
  {
    self.form_view_with(crate::components::FormProps::new(form), render)
  }

  #[cfg(feature = "form")]
  pub fn form_view_with<R>(
    &mut self,
    props: crate::components::FormProps,
    render: impl FnOnce(&mut Ctx) -> R,
  ) -> Element
  where
    R: Into<Element>,
  {
    let previous_context = self.context_map.clone();
    if let Some(form) = props.form.clone() {
      self.provide(crate::components::FormContext::new(form));
    }
    let child = render(self).into();
    self.context_map = previous_context;
    crate::components::Form::element(props, child)
  }

  #[cfg(feature = "form")]
  pub fn form_control<T>(&mut self, control: &crate::components::Control<T>) -> crate::components::ResolvedControl<T>
  where
    T: SignalValue + Clone + PartialEq + Send + Sync + 'static,
  {
    control.resolve()
  }

  #[cfg(feature = "form")]
  pub fn string_control(&mut self, name: impl Into<Arc<str>>) -> crate::components::ResolvedControl<String> {
    self.current_form().string_control(name).resolve()
  }

  #[cfg(feature = "form")]
  pub fn number_control(&mut self, name: impl Into<Arc<str>>) -> crate::components::ResolvedControl<f64> {
    self.current_form().number_control(name).resolve()
  }

  #[cfg(feature = "form")]
  pub fn bool_control(&mut self, name: impl Into<Arc<str>>) -> crate::components::ResolvedControl<bool> {
    self.current_form().bool_control(name).resolve()
  }

  #[cfg(feature = "form")]
  fn current_form(&mut self) -> crate::components::FormHandle {
    self
      .use_context::<crate::components::FormContext>()
      .map(|ctx| ctx.form())
      .expect("form controls must be resolved inside a Form render context")
  }

  #[cfg(feature = "router")]
  pub fn router(&mut self, routes: crate::router::Routes) -> crate::router::RouterHandle {
    let current_path = Signal::new(String::new());
    crate::router::RouterHandle::new_with_signal(routes, current_path)
  }

  #[cfg(feature = "router")]
  pub fn navigator(&mut self) -> Option<crate::router::Navigator> {
    self.use_context::<crate::router::Navigator>()
  }

  #[cfg(feature = "router")]
  pub fn route_params(&mut self) -> crate::router::Params {
    self
      .use_context::<crate::router::route_match::RouterMatches>()
      .and_then(|matches| matches.0.last().map(|m| m.params().clone()))
      .or_else(|| {
        self
          .use_context::<crate::router::route_match::OutletDepth>()
          .and_then(|depth| {
            self
              .use_context::<crate::router::route_match::RouterMatches>()
              .and_then(|matches| matches.0.get(depth.0).map(|m| m.params().clone()))
          })
      })
      .unwrap_or_default()
  }

  #[cfg(feature = "router")]
  pub fn route_path(&mut self) -> String {
    self
      .use_context::<crate::router::Navigator>()
      .map(|nav| nav.path().get())
      .unwrap_or_default()
  }

  pub fn future<D, T, E, F, Fut>(&mut self, deps: D, factory: F) -> FutureHandle<T, E>
  where
    D: Clone + PartialEq + Send + Sync + 'static,
    T: SignalValue + Clone + PartialEq + Send + Sync + 'static,
    E: SignalValue + Clone + PartialEq + Send + Sync + 'static,
    F: Fn(D) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<T, E>> + Send + 'static,
  {
    let cursor = self.future_cursor;
    self.future_cursor += 1;
    let runtime_handle = self.runtime_future_handle();

    if cursor < self.future_slots.len() {
      let slot = &mut self.future_slots[cursor];
      if let Some(handle) = slot.handle.downcast_ref::<FutureHandle<T, E>>() {
        let deps_changed = slot.deps.as_ref().and_then(|old| old.downcast_ref::<D>()) != Some(&deps);
        let handle = handle.clone();
        if deps_changed {
          slot.deps = Some(Box::new(deps.clone()));
          start_future_task(
            handle.state.clone(),
            handle.task.clone(),
            runtime_handle.clone(),
            Box::pin(factory(deps)),
          );
        }
        return handle;
      }
      slot.task.cancel();
    }

    let state = self.signal(FutureState::idle());
    let task = AsyncTask::new();
    let handle = FutureHandle {
      state: state.clone(),
      task: task.clone(),
    };
    let slot = FutureSlot {
      deps: Some(Box::new(deps.clone())),
      handle: Box::new(handle.clone()),
      task: task.clone(),
    };
    if cursor < self.future_slots.len() {
      self.future_slots[cursor] = slot;
    } else {
      self.future_slots.push(slot);
    }
    start_future_task(state, task, runtime_handle, Box::pin(factory(deps)));
    handle
  }

  pub fn future_action<A, T, E, F, Fut>(&mut self, factory: F) -> FutureAction<A, T, E>
  where
    A: Send + Sync + 'static,
    T: SignalValue + Clone + PartialEq + Send + Sync + 'static,
    E: SignalValue + Clone + PartialEq + Send + Sync + 'static,
    F: Fn(A) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<T, E>> + Send + 'static,
  {
    let cursor = self.future_cursor;
    self.future_cursor += 1;
    let runtime_handle = self.runtime_future_handle();
    let runner: ActionRunner<A, T, E> = Arc::new(move |args| Box::pin(factory(args)));

    if cursor < self.future_slots.len() {
      let slot = &mut self.future_slots[cursor];
      if let Some(action) = slot.handle.downcast_mut::<FutureAction<A, T, E>>() {
        *action.runner.lock() = runner;
        action.runtime_handle = runtime_handle;
        return action.clone();
      }
      slot.task.cancel();
    }

    let state = self.signal(FutureState::idle());
    let task = AsyncTask::new();
    let action = FutureAction {
      state,
      task: task.clone(),
      runner: Arc::new(Mutex::new(runner)),
      runtime_handle,
    };
    let slot = FutureSlot {
      deps: None,
      handle: Box::new(action.clone()),
      task,
    };
    if cursor < self.future_slots.len() {
      self.future_slots[cursor] = slot;
    } else {
      self.future_slots.push(slot);
    }
    action
  }

  pub fn watch<T: SignalValue + Send + Sync + 'static>(
    &mut self,
    signal: &Signal<T>,
    f: impl Fn(&T) + Send + Sync + 'static,
  ) {
    let sub = signal.subscribe(f);
    self.watch_handles.push(Box::new(sub));
  }

  // --- Store + Lenses ---

  pub fn store<T: SignalValue + Clone + Send + Sync + 'static>(&mut self, initial: T) -> Store<T> {
    let store = Store::new(initial);
    let dirty = self.dirty.clone();
    let batch = self.batch.clone();
    let handle = store.signal().watch(move || {
      batch.mark_dirty(&dirty);
    });
    self.watch_handles.push(Box::new(handle));
    store
  }

  // --- Context (Dependency Injection) ---

  pub fn provide<T: Clone + Send + Sync + 'static>(&mut self, value: T) {
    #[cfg(feature = "devtools")]
    self.push_context_debug(ComponentContextKind::Provided, std::any::type_name::<T>());
    self.context_map.provide(value);
  }

  pub fn use_context<T: Clone + Send + Sync + 'static>(&mut self) -> Option<T> {
    #[cfg(feature = "devtools")]
    self.push_context_debug(ComponentContextKind::Consumed, std::any::type_name::<T>());
    self.context_map.get::<T>()
  }

  pub fn create_context<T: Clone + std::hash::Hash + Send + Sync + 'static>(&mut self, value: T) -> ReactiveContext<T> {
    #[cfg(feature = "devtools")]
    self.push_context_debug(
      ComponentContextKind::Provided,
      std::any::type_name::<ReactiveContext<T>>(),
    );
    let ctx = ReactiveContext::new(value);
    self.context_map.provide(ctx.clone());
    let dirty = self.dirty.clone();
    let batch = self.batch.clone();
    ctx.subscribe(move || {
      batch.mark_dirty(&dirty);
    });
    ctx
  }

  pub fn consume_context<T: Clone + std::hash::Hash + Send + Sync + 'static>(&mut self) -> Option<ReactiveContext<T>> {
    #[cfg(feature = "devtools")]
    self.push_context_debug(
      ComponentContextKind::Consumed,
      std::any::type_name::<ReactiveContext<T>>(),
    );
    let ctx = self.context_map.get::<ReactiveContext<T>>()?;
    let dirty = self.dirty.clone();
    let batch = self.batch.clone();
    ctx.subscribe(move || {
      batch.mark_dirty(&dirty);
    });
    Some(ctx)
  }

  #[cfg(feature = "devtools")]
  fn push_context_debug(&mut self, kind: ComponentContextKind, type_name: &'static str) {
    if self
      .contexts_debug
      .iter()
      .any(|ctx| ctx.kind == kind && ctx.type_name.as_ref() == type_name)
    {
      return;
    }
    self.contexts_debug.push(ComponentContextDebug {
      kind,
      type_name: Arc::from(type_name),
    });
  }

  // --- Theme ---

  pub fn theme(&self) -> &Theme {
    let theme = self.theme.as_ref().expect("theme not set");
    theme.track_access();
    theme
  }

  /// Current window geometry (position, resolved/logical size, scale factor).
  /// Reading this subscribes the component to window resize/move/scale changes.
  pub fn window(&self) -> crate::app::window::WindowInfo {
    let window = self.window.as_ref().expect("window not set");
    window.track_access();
    window.info()
  }

  pub fn app_ref(&self) -> &App {
    unsafe { self.app.expect("app ref not set").as_ref() }
  }

  pub fn app_ref_mut(&mut self) -> &mut App {
    unsafe { self.app.expect("app ref not set").as_mut() }
  }

  #[cfg(feature = "i18n")]
  pub fn i18n(&self) -> &I18n {
    let i18n = self.i18n.as_ref().expect("i18n not set");
    i18n.track_access();
    i18n
  }

  #[cfg(feature = "i18n")]
  pub fn t(&self, key: &str) -> Arc<str> {
    self.i18n().t(key)
  }

  #[cfg(feature = "i18n")]
  pub fn t_ns(&self, namespace: &str, key: &str) -> Arc<str> {
    self.i18n().t_ns(namespace, key)
  }

  #[cfg(feature = "i18n")]
  pub fn t_args<I, K, V>(&self, key: &str, args: I) -> Arc<str>
  where
    I: IntoIterator<Item = (K, V)>,
    K: AsRef<str>,
    V: ToString,
  {
    self.i18n().t_args(key, args)
  }

  #[cfg(feature = "i18n")]
  pub fn t_ns_args<I, K, V>(&self, namespace: &str, key: &str, args: I) -> Arc<str>
  where
    I: IntoIterator<Item = (K, V)>,
    K: AsRef<str>,
    V: ToString,
  {
    self.i18n().t_ns_args(namespace, key, args)
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

  pub fn modal<R>(&mut self, open: Signal<bool>, render: impl FnOnce(&mut Ctx) -> R)
  where
    R: Into<Element>,
  {
    if !open.get() {
      return;
    }

    let previous = self.modal_context.replace(ModalContext { open });
    let modal = render(self).into();
    self.modal_context = previous;
    self.push_modal(modal);
  }

  pub fn modal_context(&self) -> Option<&ModalContext> {
    self.modal_context.as_ref()
  }

  fn push_modal(&mut self, modal: impl Into<Element>) {
    let cursor = self.modal_cursor;
    self.modal_cursor += 1;
    let modal = modal.into().node;

    let mut registry = self.modal_registry.lock();
    if let Some(entry) = registry
      .iter_mut()
      .find(|entry| entry.scope_id == self.modal_scope_id && entry.cursor == cursor)
    {
      entry.node = modal;
      return;
    }

    registry.push(ModalEntry {
      scope_id: self.modal_scope_id,
      cursor,
      node: modal,
    });
  }

  pub(crate) fn modal_nodes(&self) -> Vec<Node> {
    self
      .modal_registry
      .lock()
      .iter()
      .map(|entry| entry.node.clone_for_reuse())
      .collect()
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
      let context_changed = slot.ctx.context_map.revision() != self.context_map.revision();
      slot.ctx.context_map = self.context_map.clone();
      slot.ctx.slot_children = slot_children;
      if props_changed {
        slot.ctx.set_props(props);
      }
      if has_slot_children || props_changed || context_changed || slot.ctx.any_dirty() || slot.rendered.is_none() {
        slot.ctx.begin_render();
        let mut element = slot.component.render(&mut slot.ctx);
        slot.ctx.end_render();
        element.node = attach_component_metadata(
          element.node,
          slot.component.tag_name(),
          slot.id,
          slot.key.as_deref(),
          #[cfg(feature = "devtools")]
          &slot.ctx,
        );
        slot.rendered = Some(element.node.clone_for_reuse());
        return element;
      }
      return Element::from_node(slot.rendered.as_ref().unwrap().clone_for_reuse());
    }

    let mut child_ctx = Ctx::new();
    child_ctx.batch = self.batch.clone();
    child_ctx.theme = self.theme.clone();
    child_ctx.window = self.window.clone();
    child_ctx.app = self.app;
    #[cfg(feature = "tokio")]
    {
      child_ctx.runtime_future_handle = self.runtime_future_handle.clone();
    }
    child_ctx.modal_registry = self.modal_registry.clone();
    child_ctx.modal_context = self.modal_context.clone();
    #[cfg(feature = "i18n")]
    {
      child_ctx.i18n = self.i18n.clone();
    }
    child_ctx.context_map = self.context_map.clone();
    child_ctx.slot_children = slot_children;
    child_ctx.set_props(props);
    let slot_id = next_component_slot_id();
    child_ctx.modal_scope_id = slot_id;
    let component = C::create(&mut child_ctx);
    let wrapper = ComponentWrapper { component };
    child_ctx.begin_render();
    let mut element = wrapper.render(&mut child_ctx);
    child_ctx.end_render();
    element.node = attach_component_metadata(
      element.node,
      wrapper.tag_name(),
      slot_id,
      key,
      #[cfg(feature = "devtools")]
      &child_ctx,
    );

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
    let cursor = self.child_cursor;
    self.child_cursor += 1;

    let can_reuse_group = self
      .children
      .get(cursor)
      .is_some_and(|slot| slot.key.is_none() && slot.component.type_name() == ForEachSlot::TYPE_NAME);

    if !can_reuse_group {
      let mut group_ctx = Ctx::new();
      group_ctx.batch = self.batch.clone();
      group_ctx.theme = self.theme.clone();
      group_ctx.window = self.window.clone();
      group_ctx.app = self.app;
      #[cfg(feature = "tokio")]
      {
        group_ctx.runtime_future_handle = self.runtime_future_handle.clone();
      }
      group_ctx.modal_registry = self.modal_registry.clone();
      group_ctx.modal_context = self.modal_context.clone();
      #[cfg(feature = "i18n")]
      {
        group_ctx.i18n = self.i18n.clone();
      }
      group_ctx.context_map = self.context_map.clone();
      let slot_id = next_component_slot_id();
      group_ctx.modal_scope_id = slot_id;
      let slot = ChildSlot {
        id: slot_id,
        key: None,
        component: Box::new(ForEachSlot),
        ctx: group_ctx,
        rendered: None,
        mounted: false,
      };
      self.set_child_slot(cursor, slot);
    }

    let slot = &mut self.children[cursor];
    slot.ctx.context_map = self.context_map.clone();
    slot.ctx.begin_render();
    let elements = items
      .into_iter()
      .map(|item| {
        let key = format!("{}", key_fn(&item));
        slot.ctx.render_for_each_item(key, item, &component_fn)
      })
      .collect();
    slot.ctx.end_render();
    elements
  }

  fn render_for_each_item<T, CF>(&mut self, key: String, item: T, component_fn: &CF) -> Element
  where
    CF: Fn(&mut Ctx, T) -> Element,
  {
    let cursor = self.child_cursor;
    self.child_cursor += 1;

    if let Some(found) = self.children[cursor..]
      .iter()
      .position(|slot| {
        slot.key.as_deref() == Some(key.as_str()) && slot.component.type_name() == ForEachSlot::TYPE_NAME
      })
      .map(|offset| cursor + offset)
    {
      if found != cursor {
        let slot = self.children.remove(found);
        self.children.insert(cursor, slot);
      }
    }

    let can_reuse = self.children.get(cursor).is_some_and(|slot| {
      slot.key.as_deref() == Some(key.as_str()) && slot.component.type_name() == ForEachSlot::TYPE_NAME
    });

    if can_reuse {
      let slot = &mut self.children[cursor];
      slot.ctx.context_map = self.context_map.clone();
      slot.ctx.begin_render();
      let mut element = component_fn(&mut slot.ctx, item);
      slot.ctx.end_render();
      element.node = attach_component_metadata(
        element.node,
        slot.component.tag_name(),
        slot.id,
        slot.key.as_deref(),
        #[cfg(feature = "devtools")]
        &slot.ctx,
      );
      slot.rendered = Some(element.node.clone_for_reuse());
      return element;
    }

    let mut child_ctx = Ctx::new();
    child_ctx.batch = self.batch.clone();
    child_ctx.theme = self.theme.clone();
    child_ctx.window = self.window.clone();
    child_ctx.app = self.app;
    #[cfg(feature = "tokio")]
    {
      child_ctx.runtime_future_handle = self.runtime_future_handle.clone();
    }
    child_ctx.modal_registry = self.modal_registry.clone();
    child_ctx.modal_context = self.modal_context.clone();
    #[cfg(feature = "i18n")]
    {
      child_ctx.i18n = self.i18n.clone();
    }
    child_ctx.context_map = self.context_map.clone();
    let slot_id = next_component_slot_id();
    child_ctx.modal_scope_id = slot_id;
    child_ctx.begin_render();
    let mut element = component_fn(&mut child_ctx, item);
    child_ctx.end_render();
    element.node = attach_component_metadata(
      element.node,
      component_tag_name::<ForEachSlot>(),
      slot_id,
      Some(key.as_str()),
      #[cfg(feature = "devtools")]
      &child_ctx,
    );

    let slot = ChildSlot {
      id: slot_id,
      key: Some(key),
      component: Box::new(ForEachSlot),
      ctx: child_ctx,
      rendered: Some(element.node.clone_for_reuse()),
      mounted: false,
    };

    self.insert_child_slot(cursor, slot);

    element
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
    self.modal_cursor = 0;
    self.element_ref_cursor = 0;
    self.future_cursor = 0;
    self.render_watch_handles.clear();
    tracking::start_tracking();
    self.rendering = true;
  }

  fn set_child_slot(&mut self, cursor: usize, slot: ChildSlot) {
    if cursor < self.children.len() {
      self.children[cursor].ctx.clear_modal_entries_recursive();
      self.children[cursor].component.on_unmounted();
      self.children[cursor] = slot;
    } else {
      self.children.push(slot);
    }
  }

  fn insert_child_slot(&mut self, cursor: usize, slot: ChildSlot) {
    if cursor < self.children.len() {
      self.children.insert(cursor, slot);
    } else {
      self.children.push(slot);
    }
  }

  fn clear_modal_entries_recursive(&mut self) {
    self
      .modal_registry
      .lock()
      .retain(|entry| entry.scope_id != self.modal_scope_id);
    for slot in &mut self.children {
      slot.ctx.clear_modal_entries_recursive();
    }
  }

  pub(crate) fn end_render(&mut self) {
    self
      .modal_registry
      .lock()
      .retain(|entry| entry.scope_id != self.modal_scope_id || entry.cursor < self.modal_cursor);

    for slot in &self.children[self.child_cursor..] {
      slot.component.on_unmounted();
    }
    for slot in &mut self.children[self.child_cursor..] {
      slot.ctx.clear_modal_entries_recursive();
    }
    self.children.truncate(self.child_cursor);

    for slot in &mut self.children {
      if !slot.mounted {
        slot.component.on_mounted();
        slot.mounted = true;
      }
    }

    self.element_refs.truncate(self.element_ref_cursor);
    for slot in &self.future_slots[self.future_cursor..] {
      slot.task.cancel();
    }
    self.future_slots.truncate(self.future_cursor);
    self.rendering = false;
    let deps = tracking::stop_tracking();
    let dirty = self.dirty.clone();
    let batch = self.batch.clone();
    self.render_watch_handles = deps
      .into_iter()
      .map(|dep| {
        let dirty = dirty.clone();
        let batch = batch.clone();
        let handle = (dep.subscribe_fn)(Arc::new(move || {
          batch.mark_dirty(&dirty);
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

  pub(crate) fn tick_timers(&mut self, now: Instant) -> bool {
    let mut fired = false;
    for timer in &self.timers {
      fired |= timer.tick(now);
    }
    for slot in &mut self.children {
      fired |= slot.ctx.tick_timers(now);
    }
    fired
  }

  pub(crate) fn tick_futures(&mut self) -> bool {
    let waker = noop_waker();
    let mut cx = TaskContext::from_waker(&waker);
    self.poll_futures(&mut cx)
  }

  fn poll_futures(&mut self, cx: &mut TaskContext<'_>) -> bool {
    let mut completed = false;
    for slot in &self.future_slots {
      completed |= slot.task.poll(cx);
    }
    for slot in &mut self.children {
      completed |= slot.ctx.poll_futures(cx);
    }
    completed
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
        element.node = attach_component_metadata(
          element.node,
          slot.component.tag_name(),
          slot.id,
          slot.key.as_deref(),
          #[cfg(feature = "devtools")]
          &slot.ctx,
        );
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
        } else {
          replacements.extend(nested_replacements);
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
      + {
        #[cfg(feature = "devtools")]
        {
          self.memos_debug.capacity() * std::mem::size_of::<ComponentMemoDebug>()
            + self.effects_debug.capacity() * std::mem::size_of::<ComponentEffectDebug>()
        }
        #[cfg(not(feature = "devtools"))]
        {
          0
        }
      }
      + self.effects.capacity() * std::mem::size_of::<Effect>()
      + self.timers.capacity() * std::mem::size_of::<Timer>()
      + self.future_slots.capacity() * std::mem::size_of::<FutureSlot>()
      + self.element_refs.capacity() * std::mem::size_of::<ElementRefMut>()
      + self
        .children
        .iter()
        .map(ChildSlot::estimated_memory_bytes)
        .sum::<usize>()
  }
}

struct ForEachSlot;

impl ForEachSlot {
  const TYPE_NAME: &'static str = "ForEachSlot";
}

impl AnyComponent for ForEachSlot {
  fn render(&self, _ctx: &mut Ctx) -> Element {
    Element::new()
  }
  fn on_mounted(&self) {}
  fn on_unmounted(&self) {}
  fn type_name(&self) -> &'static str {
    Self::TYPE_NAME
  }
  fn tag_name(&self) -> Arc<str> {
    Arc::from("ForEachSlot")
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

#[cfg(test)]
mod tests {
  use super::Ctx;

  #[test]
  fn batch_defers_dirty_marking_until_end() {
    let mut ctx = Ctx::new_root();
    let signal = ctx.signal(0);
    ctx.clear_dirty();

    ctx.batch(|| {
      signal.set(1);
      assert!(!ctx.is_dirty());
    });

    assert!(ctx.is_dirty());
  }

  #[cfg(feature = "devtools")]
  #[test]
  fn signal_debug_value_updates_when_signal_changes() {
    let mut ctx = Ctx::new_root();
    let signal = ctx.signal(0_i32);
    let debug = ctx.signals_debug();

    assert_eq!(debug[0].formatted_value().as_deref(), Some("0"));

    signal.set(42);

    assert_eq!(debug[0].formatted_value().as_deref(), Some("42"));
  }

  #[cfg(feature = "devtools")]
  #[test]
  fn signal_debug_history_records_value_changes() {
    let mut ctx = Ctx::new_root();
    let signal = ctx.signal(0_i32);
    let debug = ctx.signals_debug();

    signal.set(1);
    signal.set(2);

    let history = debug[0].history();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].from_value, "0");
    assert_eq!(history[0].to_value, "1");
    assert_eq!(history[1].from_value, "1");
    assert_eq!(history[1].to_value, "2");
  }

  #[cfg(feature = "devtools")]
  #[test]
  fn signal_debug_subscriber_count_excludes_runtime_and_debug_hooks() {
    let mut ctx = Ctx::new_root();
    let signal = ctx.signal(0_i32);
    let debug = ctx.signals_debug();

    assert_eq!(debug[0].subscriber_count(), 0);

    ctx.watch(&signal, |_| {});
    assert_eq!(debug[0].subscriber_count(), 1);

    let memo_signal = signal.clone();
    let _memo = ctx.memo(move || memo_signal.get() + 1);
    assert_eq!(debug[0].subscriber_count(), 2);
  }

  #[cfg(feature = "devtools")]
  #[derive(crate::DevtoolsInspectable)]
  struct DebugSignalValue {
    count: i32,
    active: bool,
  }

  #[cfg(feature = "devtools")]
  #[test]
  fn signal_debug_value_uses_devtools_inspectable_fields() {
    let mut ctx = Ctx::new_root();
    let signal = ctx.signal(DebugSignalValue { count: 2, active: true });
    let debug = ctx.signals_debug();

    assert_eq!(debug[0].formatted_value().as_deref(), Some("count: 2, active: true"));

    signal.set(DebugSignalValue {
      count: 3,
      active: false,
    });

    assert_eq!(debug[0].formatted_value().as_deref(), Some("count: 3, active: false"));
  }

  #[cfg(feature = "devtools")]
  #[test]
  fn memo_debug_value_updates_when_dependencies_change() {
    let mut ctx = Ctx::new_root();
    let signal = ctx.signal(2_i32);
    let memo_signal = signal.clone();
    let _memo = ctx.memo(move || memo_signal.get() * 2);
    let debug = ctx.memos_debug();

    assert_eq!(debug[0].formatted_value().as_deref(), Some("4"));

    signal.set(3);

    assert_eq!(debug[0].formatted_value().as_deref(), Some("6"));
  }
}
