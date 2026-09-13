//! Shared, typed async queries. Enable the `query` Cargo feature.
//!
//! Define a query with [`macro@crate::query`], provide a [`QueryClient`] once,
//! and observe its descriptor with [`crate::app::ctx::Ctx::query`].
//! Arguments are the cache key. Constructing a descriptor never starts work.

use std::{
  any::{Any, TypeId},
  collections::{HashMap, VecDeque, hash_map::DefaultHasher},
  future::Future,
  hash::{Hash, Hasher},
  marker::PhantomData,
  pin::Pin,
  sync::{
    Arc, Weak,
    atomic::{AtomicBool, AtomicU64, Ordering},
  },
  task::{Context, Poll, Wake, Waker},
  time::{Duration, Instant},
};

use parking_lot::Mutex;
use sealed::Selector as _;

use crate::{app::window::WindowWaker, core::Signal};

/// The future returned by a query's implementation.
pub type QueryFuture<T, E> = Pin<Box<dyn Future<Output = Result<T, E>> + Send + 'static>>;

/// Implemented by the `#[lurq::query]` macro.
pub trait QueryDefinition: Send + Sync + 'static {
  type Args: Clone + Eq + Hash + Send + Sync + 'static;
  type Data: Send + Sync + 'static;
  type Error: Send + Sync + 'static;
  fn name() -> &'static str;
  fn run(args: Self::Args) -> QueryFuture<Self::Data, Self::Error>;
  fn options() -> QueryOptions {
    QueryOptions::default()
  }
}

/// Per-definition overrides, normally supplied by the query attribute.
#[derive(Clone, Copy, Debug, Default)]
pub struct QueryOptions {
  pub stale_time: Option<Duration>,
  pub gc_time: Option<Duration>,
}

/// Defaults used by definitions without an explicit override.
#[derive(Clone, Copy, Debug)]
pub struct QueryClientOptions {
  pub stale_time: Duration,
  pub gc_time: Duration,
}

impl Default for QueryClientOptions {
  fn default() -> Self {
    Self {
      stale_time: Duration::from_secs(30),
      gc_time: Duration::from_secs(300),
    }
  }
}

/// A lazy query descriptor. Use the generated function to construct one.
pub struct Query<D: QueryDefinition> {
  args: D::Args,
}

impl<D: QueryDefinition> Clone for Query<D> {
  fn clone(&self) -> Self {
    Self {
      args: self.args.clone(),
    }
  }
}

impl<D: QueryDefinition> Query<D> {
  #[doc(hidden)]
  pub fn new(args: D::Args) -> Self {
    Self { args }
  }
}

/// A selector for all cached argument combinations of one definition.
pub struct QueryFamily<D: QueryDefinition>(PhantomData<fn() -> D>);

impl<D: QueryDefinition> QueryFamily<D> {
  #[doc(hidden)]
  pub fn new() -> Self {
    Self(PhantomData)
  }
}

impl<D: QueryDefinition> Default for QueryFamily<D> {
  fn default() -> Self {
    Self::new()
  }
}

mod sealed {
  use super::*;
  pub trait Selector {
    fn matches(&self, family: TypeId, args: &dyn Any) -> bool;
  }
}

/// An exact descriptor or a generated `query_name::all()` selector.
pub trait QuerySelector: sealed::Selector + Send + Sync + 'static {}

impl<D: QueryDefinition> sealed::Selector for Query<D> {
  fn matches(&self, family: TypeId, args: &dyn Any) -> bool {
    family == TypeId::of::<D>() && args.downcast_ref::<D::Args>() == Some(&self.args)
  }
}
impl<D: QueryDefinition> QuerySelector for Query<D> {}
impl<D: QueryDefinition> sealed::Selector for QueryFamily<D> {
  fn matches(&self, family: TypeId, _: &dyn Any) -> bool {
    family == TypeId::of::<D>()
  }
}
impl<D: QueryDefinition> QuerySelector for QueryFamily<D> {}

struct Value<T, E> {
  data: Option<Arc<T>>,
  error: Option<Arc<E>>,
  fetching: bool,
}

struct Shared<T, E> {
  value: Mutex<Value<T, E>>,
  // Track metadata separately: arbitrary payloads do not need Clone,
  // PartialEq, or debug-inspection bounds to participate in the cache.
  revision: AtomicU64,
}

impl<T, E> Shared<T, E> {
  fn new() -> Self {
    Self {
      value: Mutex::new(Value {
        data: None,
        error: None,
        fetching: false,
      }),
      revision: AtomicU64::new(0),
    }
  }
  fn notify(&self) {
    self.revision.fetch_add(1, Ordering::Release);
  }
}

/// A reactive view of one cached entry. Clones do not add mounted observers.
/// After eviction, a saved handle is empty and inert; observe the query again.
pub struct QueryHandle<T, E> {
  shared: Arc<Shared<T, E>>,
  revision: Signal<u64>,
  entry: Weak<dyn ErasedEntry>,
  client: QueryClient,
}

impl<T, E> Clone for QueryHandle<T, E> {
  fn clone(&self) -> Self {
    Self {
      shared: self.shared.clone(),
      revision: self.revision.clone(),
      entry: self.entry.clone(),
      client: self.client.clone(),
    }
  }
}

impl<T, E> QueryHandle<T, E> {
  pub fn data(&self) -> Option<Arc<T>> {
    self.revision.get();
    self.shared.value.lock().data.clone()
  }
  pub fn error(&self) -> Option<Arc<E>> {
    self.revision.get();
    self.shared.value.lock().error.clone()
  }
  /// Whether a request is scheduled or running with no cached data.
  pub fn loading(&self) -> bool {
    self.revision.get();
    let value = self.shared.value.lock();
    value.fetching && value.data.is_none()
  }
  pub fn fetching(&self) -> bool {
    self.revision.get();
    self.shared.value.lock().fetching
  }
  /// Mark this entry stale; mounted observers trigger a background refresh.
  pub fn invalidate(&self) {
    self.client.enqueue(Command::Entry(self.entry.clone(), false));
  }
  /// Fetch now, even if fresh or inactive. Joins an existing current request.
  pub fn refresh(&self) {
    self.client.enqueue(Command::Entry(self.entry.clone(), true));
  }
}

/// Metadata for inspection, without exposing arguments or fetched values.
#[derive(Clone, Debug)]
pub struct QueryInfo {
  pub name: &'static str,
  pub observers: usize,
  pub fetching: bool,
  pub has_data: bool,
  pub has_error: bool,
  pub stale: bool,
}

type EntryMap = HashMap<(TypeId, u64), Vec<Arc<dyn ErasedEntry>>>;
type Matcher = Box<dyn Fn(TypeId, &dyn Any) -> bool + Send + Sync>;

enum Command {
  Invalidate(Matcher),
  Entry(Weak<dyn ErasedEntry>, bool),
}

#[derive(Default)]
struct RegistryOwner {
  pending: AtomicBool,
}

struct Binding {
  owner: Weak<RegistryOwner>,
  wake: Option<WindowWaker>,
  #[cfg(feature = "tokio")]
  runtime: Option<tokio::runtime::Handle>,
}

struct ClientInner {
  options: QueryClientOptions,
  entries: Mutex<EntryMap>,
  commands: Mutex<VecDeque<Command>>,
  bindings: Mutex<Vec<Binding>>,
  // Serialize polling, observation, eviction and detachment across tree threads.
  // User futures may enqueue commands while polled, so enqueue never takes this lock.
  driver: Mutex<()>,
  #[cfg(feature = "tokio")]
  stopped_runtimes: Mutex<std::collections::HashSet<tokio::runtime::Id>>,
}

/// A shared cache, provided once through `ctx.provide(QueryClient::new())`.
/// Pass clones to multiple trees to share their cache, requests and invalidation.
/// Use `new()` for independent caches. Commands are safe to enqueue from any thread.
#[derive(Clone)]
pub struct QueryClient {
  inner: Arc<ClientInner>,
}

impl PartialEq for QueryClient {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.inner, &other.inner)
  }
}
impl Eq for QueryClient {}

impl crate::app::component::DevtoolsInspectable for QueryClient {
  fn inspect(&self, formatter: &mut crate::app::component::DevtoolsFormatter<'_>) {
    formatter.value(std::any::type_name::<Self>(), "shared query cache");
  }
}

impl Default for QueryClient {
  fn default() -> Self {
    Self::new()
  }
}

impl QueryClient {
  pub fn new() -> Self {
    Self::with_options(QueryClientOptions::default())
  }
  pub fn with_options(options: QueryClientOptions) -> Self {
    Self {
      inner: Arc::new(ClientInner {
        options,
        entries: Mutex::new(HashMap::new()),
        commands: Mutex::new(VecDeque::new()),
        bindings: Mutex::new(Vec::new()),
        driver: Mutex::new(()),
        #[cfg(feature = "tokio")]
        stopped_runtimes: Mutex::new(std::collections::HashSet::new()),
      }),
    }
  }
  /// Invalidate one descriptor or a whole generated query family.
  /// Missing entries are not created. Work is applied by `Tree::tick_futures`.
  pub fn invalidate(&self, selector: impl QuerySelector) {
    self.enqueue(Command::Invalidate(Box::new(move |family, args| {
      selector.matches(family, args)
    })));
  }
  /// Snapshot cache metadata. Ordering is unspecified.
  pub fn inspect(&self) -> Vec<QueryInfo> {
    let now = Instant::now();
    self.entries().iter().map(|entry| entry.info(now)).collect()
  }
  fn enqueue(&self, command: Command) {
    self.inner.commands.lock().push_back(command);
    self.inner.wake_all();
  }
  fn entries(&self) -> Vec<Arc<dyn ErasedEntry>> {
    self.inner.entries.lock().values().flatten().cloned().collect()
  }
  pub(crate) fn observe<D: QueryDefinition>(
    &self,
    query: Query<D>,
    now: Instant,
  ) -> (QueryHandle<D::Data, D::Error>, Observer) {
    let driver = self.inner.driver.lock();
    let mut hasher = DefaultHasher::new();
    query.args.hash(&mut hasher);
    let key = (TypeId::of::<D>(), hasher.finish());
    let entry = {
      let mut entries = self.inner.entries.lock();
      let bucket = entries.entry(key).or_default();
      // No tree is required to keep ticking while a caller retains a client.
      // Enforce elapsed retention before reusing a previously detached cache.
      bucket.retain(|entry| {
        if entry.expired(now) {
          entry.evict();
          false
        } else {
          true
        }
      });
      if let Some(entry) = bucket
        .iter()
        .find(|entry| entry.args().downcast_ref::<D::Args>() == Some(&query.args))
      {
        entry.clone()
      } else {
        let overrides = D::options();
        let options = QueryClientOptions {
          stale_time: overrides.stale_time.unwrap_or(self.inner.options.stale_time),
          gc_time: overrides.gc_time.unwrap_or(self.inner.options.gc_time),
        };
        let entry: Arc<dyn ErasedEntry> = Arc::new(Entry::<D> {
          args: query.args,
          options,
          shared: Arc::new(Shared::new()),
          inner: Mutex::new(EntryInner::default()),
        });
        bucket.push(entry.clone());
        entry
      }
    };
    let typed = entry
      .as_any()
      .downcast_ref::<Entry<D>>()
      .expect("query definition identity");
    entry.observe(now);
    let revision = Signal::new(entry.revision());
    let handle = QueryHandle {
      shared: typed.shared.clone(),
      revision: revision.clone(),
      entry: Arc::downgrade(&entry),
      client: self.clone(),
    };
    drop(driver);
    self.inner.wake_all();
    (
      handle,
      Observer {
        entry,
        client: self.clone(),
        revision,
      },
    )
  }
  fn tick(&self, now: Instant, cx: &mut Context<'_>) -> bool {
    let driver = self.inner.driver.lock();
    let commands = std::mem::take(&mut *self.inner.commands.lock());
    let mut changed = !commands.is_empty();
    for command in commands {
      match command {
        Command::Invalidate(matches) => {
          for entry in self.entries() {
            if matches(entry.family(), entry.args()) {
              entry.invalidate();
            }
          }
        }
        Command::Entry(entry, refresh) => {
          if let Some(entry) = entry.upgrade() {
            if refresh {
              entry.refresh();
            } else {
              entry.invalidate();
            }
          }
        }
      }
    }
    let entries = self.entries();
    for entry in entries {
      if entry.expired(now) {
        entry.evict();
        changed = true;
        self.inner.entries.lock().retain(|_, bucket| {
          bucket.retain(|candidate| !Arc::ptr_eq(candidate, &entry));
          !bucket.is_empty()
        });
      } else {
        changed |= entry.tick(now, &self.inner, cx);
      }
    }
    drop(driver);
    if changed {
      self.inner.wake_all();
    }
    changed
  }
  fn ready(&self) -> bool {
    let can_start = self.inner.can_start();
    !self.inner.commands.lock().is_empty() || self.entries().iter().any(|entry| entry.ready(can_start))
  }
  fn deadline(&self) -> Option<Instant> {
    self.entries().iter().filter_map(|entry| entry.deadline()).min()
  }
}

pub(crate) struct Observer {
  entry: Arc<dyn ErasedEntry>,
  client: QueryClient,
  revision: Signal<u64>,
}

impl Observer {
  pub(crate) fn matches<D: QueryDefinition>(&self, client: &QueryClient, query: &Query<D>) -> bool {
    Arc::ptr_eq(&self.client.inner, &client.inner) && query.matches(self.entry.family(), self.entry.args())
  }
  pub(crate) fn handle<D: QueryDefinition>(&self) -> QueryHandle<D::Data, D::Error> {
    let entry = self
      .entry
      .as_any()
      .downcast_ref::<Entry<D>>()
      .expect("query observer type");
    QueryHandle {
      shared: entry.shared.clone(),
      revision: self.revision.clone(),
      entry: Arc::downgrade(&self.entry),
      client: self.client.clone(),
    }
  }

  // Publish on this observer's tree thread, never on another tree's poller.
  pub(crate) fn publish(&self) -> bool {
    let revision = self.entry.revision();
    if self.revision.get_untracked() == revision {
      return false;
    }
    self.revision.set(revision);
    true
  }
}

impl Drop for Observer {
  fn drop(&mut self) {
    {
      let _driver = self.client.inner.driver.lock();
      self.entry.unobserve(Instant::now());
    }
    self.client.inner.wake_all();
  }
}

trait ErasedEntry: Any + Send + Sync {
  fn as_any(&self) -> &dyn Any;
  fn args(&self) -> &dyn Any;
  fn family(&self) -> TypeId;
  fn observe(&self, now: Instant);
  fn unobserve(&self, now: Instant);
  fn invalidate(&self);
  fn refresh(&self);
  fn expired(&self, now: Instant) -> bool;
  fn evict(&self);
  fn suspend(&self);
  fn revision(&self) -> u64;
  fn ready(&self, can_start: bool) -> bool;
  fn deadline(&self) -> Option<Instant>;
  fn tick(&self, now: Instant, client: &Arc<ClientInner>, cx: &mut Context<'_>) -> bool;
  fn info(&self, now: Instant) -> QueryInfo;
}

struct Entry<D: QueryDefinition> {
  args: D::Args,
  options: QueryClientOptions,
  shared: Arc<Shared<D::Data, D::Error>>,
  inner: Mutex<EntryInner<D::Data, D::Error>>,
}

struct EntryInner<T, E> {
  observers: usize,
  inactive_since: Option<Instant>,
  updated_at: Option<Instant>,
  invalidated: bool,
  generation: u64,
  scheduled: bool,
  evicted: bool,
  task: Option<Task<T, E>>,
}

impl<T, E> Default for EntryInner<T, E> {
  fn default() -> Self {
    Self {
      observers: 0,
      inactive_since: None,
      updated_at: None,
      invalidated: true,
      generation: 0,
      scheduled: false,
      evicted: false,
      task: None,
    }
  }
}

impl<T, E> EntryInner<T, E> {
  fn stale(&self, now: Instant, stale_time: Duration) -> bool {
    self.invalidated
      || self
        .updated_at
        .is_none_or(|at| now.saturating_duration_since(at) >= stale_time)
  }
}

impl<D: QueryDefinition> Entry<D> {
  fn mark_fetching(&self, fetching: bool) {
    {
      let mut value = self.shared.value.lock();
      value.fetching = fetching;
      if fetching {
        value.error = None;
      }
    }
    self.shared.notify();
  }
}

impl<D: QueryDefinition> ErasedEntry for Entry<D> {
  fn as_any(&self) -> &dyn Any {
    self
  }
  fn args(&self) -> &dyn Any {
    &self.args
  }
  fn family(&self) -> TypeId {
    TypeId::of::<D>()
  }
  fn observe(&self, now: Instant) {
    let start = {
      let mut inner = self.inner.lock();
      inner.observers += 1;
      inner.inactive_since = None;
      let start = inner.stale(now, self.options.stale_time) && inner.task.is_none() && !inner.scheduled;
      inner.scheduled |= start;
      start
    };
    if start {
      self.mark_fetching(true);
    }
  }
  fn unobserve(&self, now: Instant) {
    let mut inner = self.inner.lock();
    inner.observers -= 1;
    if inner.observers == 0 {
      inner.inactive_since = Some(now);
    }
  }
  fn invalidate(&self) {
    let (fetching, obsolete) = {
      let mut inner = self.inner.lock();
      if inner.evicted {
        return;
      }
      inner.invalidated = true;
      inner.generation = inner.generation.wrapping_add(1);
      inner.scheduled = inner.observers > 0;
      (inner.scheduled, inner.task.take())
    };
    drop(obsolete);
    self.mark_fetching(fetching);
  }
  fn refresh(&self) {
    {
      let mut inner = self.inner.lock();
      if inner.evicted || inner.scheduled || inner.task.is_some() {
        return;
      }
      inner.scheduled = true;
      // An explicit inactive refresh gets a full retention period to finish.
      if inner.observers == 0 {
        inner.inactive_since = Some(Instant::now());
      }
    }
    self.mark_fetching(true);
  }
  fn expired(&self, now: Instant) -> bool {
    self
      .inner
      .lock()
      .inactive_since
      .is_some_and(|at| now.saturating_duration_since(at) >= self.options.gc_time)
  }
  fn evict(&self) {
    let obsolete = {
      let mut inner = self.inner.lock();
      inner.evicted = true;
      inner.scheduled = false;
      inner.generation = inner.generation.wrapping_add(1);
      inner.task.take()
    };
    drop(obsolete);
    *self.shared.value.lock() = Value {
      data: None,
      error: None,
      fetching: false,
    };
    self.shared.notify();
  }
  fn suspend(&self) {
    let obsolete = {
      let mut inner = self.inner.lock();
      if inner.task.is_none() && !inner.scheduled {
        return;
      }
      inner.invalidated = true;
      inner.scheduled = false;
      inner.generation = inner.generation.wrapping_add(1);
      inner.task.take()
    };
    drop(obsolete);
    self.mark_fetching(false);
  }
  fn revision(&self) -> u64 {
    self.shared.revision.load(Ordering::Acquire)
  }
  fn ready(&self, can_start: bool) -> bool {
    let inner = self.inner.lock();
    (inner.scheduled && can_start)
      || inner
        .task
        .as_ref()
        .is_some_and(|task| task.wake.ready.load(Ordering::Acquire))
  }
  fn deadline(&self) -> Option<Instant> {
    self
      .inner
      .lock()
      .inactive_since
      .and_then(|at| at.checked_add(self.options.gc_time))
  }
  fn tick(&self, now: Instant, client: &Arc<ClientInner>, _cx: &mut Context<'_>) -> bool {
    let (start, generation, existing) = {
      let mut inner = self.inner.lock();
      if inner.evicted {
        return false;
      }
      if inner.scheduled && !client.can_start() {
        return false;
      }
      let start = inner.scheduled;
      inner.scheduled = false;
      (start, inner.generation, inner.task.take())
    };
    let task = if start {
      let wake = Arc::new(TaskWake {
        ready: AtomicBool::new(true),
        client: Arc::downgrade(client),
      });
      #[cfg(feature = "tokio")]
      let runtime = client.runtime();
      let future = D::run(self.args.clone());
      #[cfg(feature = "tokio")]
      let execution = if let Some(runtime) = runtime {
        Execution::Tokio(runtime.spawn(future), runtime.id())
      } else {
        Execution::Cooperative(future)
      };
      #[cfg(not(feature = "tokio"))]
      let execution = Execution::Cooperative(future);
      Some(Task {
        generation,
        execution,
        wake,
      })
    } else {
      existing
    };
    let Some(mut task) = task else {
      return false;
    };
    let result = if task.wake.ready.swap(false, Ordering::AcqRel) {
      let waker = Waker::from(task.wake.clone());
      task.poll(&mut Context::from_waker(&waker))
    } else {
      Poll::Pending
    };
    match result {
      Poll::Pending => {
        let mut inner = self.inner.lock();
        if !inner.evicted && inner.generation == task.generation {
          inner.task = Some(task);
        }
        false
      }
      #[cfg(feature = "tokio")]
      Poll::Ready(TaskResult::RuntimeStopped(runtime)) => {
        client.stopped_runtimes.lock().insert(runtime);
        {
          let mut inner = self.inner.lock();
          inner.invalidated = true;
          inner.scheduled = inner.observers > 0;
        }
        let fetching = self.inner.lock().scheduled;
        self.mark_fetching(fetching);
        true
      }
      Poll::Ready(TaskResult::Finished(result)) => {
        {
          let mut inner = self.inner.lock();
          if inner.evicted || inner.generation != task.generation {
            return false;
          }
          if result.is_ok() {
            inner.updated_at = Some(now);
            inner.invalidated = false;
          }
        }
        {
          let mut value = self.shared.value.lock();
          value.fetching = false;
          match result {
            Ok(data) => {
              value.data = Some(Arc::new(data));
              value.error = None;
            }
            Err(error) => {
              value.error = Some(Arc::new(error));
            }
          }
        }
        self.shared.notify();
        true
      }
    }
  }
  fn info(&self, now: Instant) -> QueryInfo {
    let inner = self.inner.lock();
    let value = self.shared.value.lock();
    QueryInfo {
      name: D::name(),
      observers: inner.observers,
      fetching: value.fetching,
      has_data: value.data.is_some(),
      has_error: value.error.is_some(),
      stale: inner.stale(now, self.options.stale_time),
    }
  }
}

struct TaskWake {
  ready: AtomicBool,
  client: Weak<ClientInner>,
}

impl Wake for TaskWake {
  fn wake(self: Arc<Self>) {
    self.wake_by_ref();
  }
  fn wake_by_ref(self: &Arc<Self>) {
    self.ready.store(true, Ordering::Release);
    if let Some(client) = self.client.upgrade() {
      client.wake_all();
    }
  }
}

enum Execution<T, E> {
  Cooperative(QueryFuture<T, E>),
  #[cfg(feature = "tokio")]
  Tokio(tokio::task::JoinHandle<Result<T, E>>, tokio::runtime::Id),
}

enum TaskResult<T, E> {
  Finished(Result<T, E>),
  #[cfg(feature = "tokio")]
  RuntimeStopped(tokio::runtime::Id),
}

struct Task<T, E> {
  generation: u64,
  execution: Execution<T, E>,
  wake: Arc<TaskWake>,
}

impl<T, E> Task<T, E> {
  fn poll(&mut self, cx: &mut Context<'_>) -> Poll<TaskResult<T, E>> {
    match &mut self.execution {
      Execution::Cooperative(future) => future.as_mut().poll(cx).map(TaskResult::Finished),
      #[cfg(feature = "tokio")]
      Execution::Tokio(task, runtime) => match Pin::new(task).poll(cx) {
        Poll::Ready(Ok(result)) => Poll::Ready(TaskResult::Finished(result)),
        Poll::Ready(Err(error)) if error.is_panic() => std::panic::resume_unwind(error.into_panic()),
        Poll::Ready(Err(_)) => Poll::Ready(TaskResult::RuntimeStopped(*runtime)),
        Poll::Pending => Poll::Pending,
      },
    }
  }
}

impl<T, E> Drop for Task<T, E> {
  fn drop(&mut self) {
    #[cfg(feature = "tokio")]
    if let Execution::Tokio(task, _) = &self.execution {
      task.abort();
    }
  }
}

/// Shared by every context in one tree, so inactive clients still get driven.
#[derive(Clone, Default)]
pub(crate) struct QueryRegistry {
  inner: Arc<RegistryInner>,
}

#[derive(Default)]
struct RegistryInner {
  owner: Arc<RegistryOwner>,
  clients: Mutex<Vec<QueryClient>>,
}

impl QueryRegistry {
  pub(crate) fn attach(
    &self,
    client: &QueryClient,
    wake: Option<WindowWaker>,
    #[cfg(feature = "tokio")] runtime: Option<tokio::runtime::Handle>,
  ) {
    let newly_attached = {
      let _driver = client.inner.driver.lock();
      let mut bindings = client.inner.bindings.lock();
      bindings.retain(|binding| binding.owner.strong_count() > 0);
      let binding = Binding {
        owner: Arc::downgrade(&self.inner.owner),
        wake,
        #[cfg(feature = "tokio")]
        runtime,
      };
      if let Some(existing) = bindings
        .iter_mut()
        .find(|binding| Weak::ptr_eq(&binding.owner, &Arc::downgrade(&self.inner.owner)))
      {
        #[cfg(feature = "tokio")]
        let changed = existing.runtime.as_ref().map(tokio::runtime::Handle::id)
          != binding.runtime.as_ref().map(tokio::runtime::Handle::id);
        #[cfg(not(feature = "tokio"))]
        let changed = false;
        *existing = binding;
        changed
      } else {
        bindings.push(binding);
        true
      }
    };
    let mut clients = self.inner.clients.lock();
    if !clients
      .iter()
      .any(|candidate| Arc::ptr_eq(&candidate.inner, &client.inner))
    {
      clients.push(client.clone());
    }
    drop(clients);
    if newly_attached {
      client.inner.wake_all();
    }
  }
  pub(crate) fn tick(&self, now: Instant, cx: &mut Context<'_>) -> bool {
    let clients = self.inner.clients.lock().clone();
    let mut changed = self.inner.owner.pending.swap(false, Ordering::AcqRel);
    for client in clients {
      changed |= client.tick(now, cx);
    }
    self.inner.clients.lock().retain(|client| {
      Arc::strong_count(&client.inner) > 1
        || !client.inner.entries.lock().is_empty()
        || !client.inner.commands.lock().is_empty()
    });
    changed | self.inner.owner.pending.swap(false, Ordering::AcqRel)
  }
  pub(crate) fn ready(&self) -> bool {
    self.inner.owner.pending.load(Ordering::Acquire) || self.inner.clients.lock().iter().any(QueryClient::ready)
  }
  pub(crate) fn deadline(&self) -> Option<Instant> {
    self.inner.clients.lock().iter().filter_map(QueryClient::deadline).min()
  }
}

impl Drop for RegistryInner {
  fn drop(&mut self) {
    // Detach only this tree. Other trees keep driving shared requests.
    for client in self.clients.lock().iter() {
      let driver = client.inner.driver.lock();
      let last = {
        let mut bindings = client.inner.bindings.lock();
        bindings.retain(|binding| {
          binding.owner.strong_count() > 0 && !Weak::ptr_eq(&binding.owner, &Arc::downgrade(&self.owner))
        });
        bindings.is_empty()
      };
      if last {
        // Without a tree there is no UI driver. Keep successful data for a
        // later attachment, but cancel pending work and reject its completion.
        for entry in client.entries() {
          entry.suspend();
        }
      }
      drop(driver);
      client.inner.wake_all();
    }
  }
}
impl ClientInner {
  #[cfg(feature = "tokio")]
  fn runtime(&self) -> Option<tokio::runtime::Handle> {
    let bindings = self.bindings.lock();
    let stopped = self.stopped_runtimes.lock();
    bindings
      .iter()
      .filter(|binding| binding.owner.strong_count() > 0)
      .filter_map(|binding| binding.runtime.as_ref())
      .find(|runtime| !stopped.contains(&runtime.id()))
      .cloned()
  }

  fn can_start(&self) -> bool {
    #[cfg(feature = "tokio")]
    {
      self.runtime().is_some() || self.stopped_runtimes.lock().is_empty()
    }
    #[cfg(not(feature = "tokio"))]
    {
      true
    }
  }

  fn wake_all(&self) {
    let bindings: Vec<_> = self
      .bindings
      .lock()
      .iter()
      .filter_map(|binding| binding.owner.upgrade().map(|owner| (owner, binding.wake.clone())))
      .collect();
    for (owner, wake) in bindings {
      owner.pending.store(true, Ordering::Release);
      if let Some(wake) = wake {
        wake();
      }
    }
  }
}

#[cfg(test)]
mod tests;
