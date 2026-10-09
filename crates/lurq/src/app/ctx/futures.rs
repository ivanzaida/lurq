//! Futures, streams and future actions of a component: their states, handles,
//! tasks, and the `Ctx` methods that create them.

#[cfg(feature = "tokio")]
use std::sync::mpsc::{self, TryRecvError};
use std::{
  any::Any,
  future::Future,
  pin::Pin,
  sync::Arc,
  task::{Context as TaskContext, Poll, Wake, Waker},
};

use parking_lot::Mutex;

use super::Ctx;
use crate::core::signal::{Signal, SignalValue};

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

pub struct StreamHandle<T: SignalValue, E: SignalValue> {
  state: Signal<FutureState<T, E>>,
  task: AsyncTask,
}

#[derive(Clone)]
pub struct StreamEmitter<T: SignalValue, E: SignalValue> {
  state: Signal<FutureState<T, E>>,
  #[cfg(feature = "tokio")]
  sender: Option<mpsc::Sender<FutureCompletion>>,
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
pub(super) type RuntimeFutureHandle = Option<tokio::runtime::Handle>;
#[cfg(not(feature = "tokio"))]
pub(super) type RuntimeFutureHandle = ();

#[derive(Clone)]
pub(super) struct AsyncTask {
  inner: Arc<Mutex<AsyncTaskInner>>,
}

#[derive(Default)]
struct AsyncTaskInner {
  work: AsyncWork,
  /// Set when the slot that owns the task is dropped: the task never starts
  /// work again, even through a handle that outlived its component.
  closed: bool,
}

/// The work a task is running. Dropping it cancels the work.
#[derive(Default)]
struct AsyncWork {
  future: Option<Pin<Box<dyn Future<Output = ()> + Send>>>,
  #[cfg(feature = "tokio")]
  tokio_task: Option<TokioAsyncTask>,
}

/// A `ctx.future`, `ctx.stream` or `ctx.future_action` slot of a component.
/// Its task lives as long as the slot: dropping the slot (its component
/// unmounts, or a render no longer reaches it) cancels the task.
pub(super) struct FutureSlot {
  deps: Option<Box<dyn Any + Send + Sync>>,
  handle: Box<dyn Any + Send + Sync>,
  pub(super) task: AsyncTask,
}

impl Drop for FutureSlot {
  fn drop(&mut self) {
    self.task.close();
  }
}

#[cfg(feature = "tokio")]
struct TokioAsyncTask {
  join: tokio::task::JoinHandle<()>,
  receiver: mpsc::Receiver<FutureCompletion>,
  finish_on_message: bool,
}

/// Dropping a `JoinHandle` only detaches its task, so the task is aborted
/// explicitly. `abort` only marks the task cancelled and schedules it: the
/// runtime drops the future on its own threads, and on a runtime that is
/// shutting down or gone the call does nothing (shutdown drops the future).
#[cfg(feature = "tokio")]
impl Drop for TokioAsyncTask {
  fn drop(&mut self) {
    self.join.abort();
  }
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

impl<T: SignalValue, E: SignalValue> Clone for StreamHandle<T, E> {
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

impl<T, E> StreamEmitter<T, E>
where
  T: SignalValue + Clone + PartialEq + Send + Sync + 'static,
  E: SignalValue + Clone + PartialEq + Send + Sync + 'static,
{
  pub fn emit(&self, data: T) -> bool {
    #[cfg(feature = "tokio")]
    if let Some(sender) = &self.sender {
      let state = self.state.clone();
      return sender
        .send(Box::new(move || state.set(FutureState::fulfilled(data))))
        .is_ok();
    }

    self.state.set(FutureState::fulfilled(data));
    true
  }

  pub fn reject(&self, error: E) -> bool {
    #[cfg(feature = "tokio")]
    if let Some(sender) = &self.sender {
      let state = self.state.clone();
      return sender
        .send(Box::new(move || {
          let previous_data = state.get_untracked().data;
          state.set(FutureState::rejected(error, previous_data));
        }))
        .is_ok();
    }

    let previous_data = self.state.get_untracked().data;
    self.state.set(FutureState::rejected(error, previous_data));
    true
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

impl<T: SignalValue, E: SignalValue> StreamHandle<T, E> {
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

  /// Starts the action with `args`, replacing a run still in flight. Does
  /// nothing once the component that created the action has unmounted.
  pub fn run(&self, args: A) {
    if self.task.is_closed() {
      return;
    }
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

  /// Replaces the running work with a future polled by `tick_futures`.
  fn set(&self, future: Pin<Box<dyn Future<Output = ()> + Send>>) {
    let work = AsyncWork {
      future: Some(future),
      #[cfg(feature = "tokio")]
      tokio_task: None,
    };
    self.replace_work(work);
  }

  /// Replaces the running work with `future` spawned on `runtime`.
  ///
  /// The task is spawned before the lock is taken: on a runtime that is gone,
  /// `spawn` drops the future on this thread, and its destructors may use a
  /// handle of this task. A `close` that lands between the spawn and the lock
  /// is seen by `replace_work`, which then aborts the new task.
  #[cfg(feature = "tokio")]
  fn spawn(
    &self,
    runtime: &tokio::runtime::Handle,
    future: impl Future<Output = ()> + Send + 'static,
    receiver: mpsc::Receiver<FutureCompletion>,
    finish_on_message: bool,
  ) {
    if self.is_closed() {
      return;
    }
    let spawned = TokioAsyncTask {
      join: runtime.spawn(future),
      receiver,
      finish_on_message,
    };
    self.replace_work(AsyncWork {
      future: None,
      tokio_task: Some(spawned),
    });
  }

  /// Makes `work` the running work, unless the task is closed. Whichever work
  /// ends up unused (the replaced one, or `work` itself on a closed task) is
  /// dropped once the lock is released: dropping it runs user destructors,
  /// which may use a handle of this task.
  fn replace_work(&self, work: AsyncWork) {
    let unused = {
      let mut inner = self.inner.lock();
      if inner.closed {
        work
      } else {
        std::mem::replace(&mut inner.work, work)
      }
    };
    drop(unused);
  }

  fn cancel(&self) {
    // Dropped once the lock is released: dropping a future runs its destructors.
    let cancelled = std::mem::take(&mut self.inner.lock().work);
    drop(cancelled);
  }

  /// Cancels the running work and keeps the task from starting again.
  fn close(&self) {
    let cancelled = {
      let mut inner = self.inner.lock();
      inner.closed = true;
      std::mem::take(&mut inner.work)
    };
    drop(cancelled);
  }

  fn is_closed(&self) -> bool {
    self.inner.lock().closed
  }

  pub(super) fn is_active(&self) -> bool {
    let inner = self.inner.lock();
    inner.work.future.is_some() || {
      #[cfg(feature = "tokio")]
      {
        inner.work.tokio_task.is_some()
      }
      #[cfg(not(feature = "tokio"))]
      {
        false
      }
    }
  }

  pub(super) fn poll(&self, cx: &mut TaskContext<'_>) -> bool {
    // Release the task lock before polling: a completing future sets its state
    // signal, and an observer of that signal may start this task again.
    let future = self.inner.lock().work.future.take();
    if let Some(mut future) = future {
      match future.as_mut().poll(cx) {
        Poll::Ready(()) => return true,
        Poll::Pending => {
          let mut inner = self.inner.lock();
          if !inner.closed && inner.work.future.is_none() {
            inner.work.future = Some(future);
          }
          return false;
        }
      }
    }

    #[cfg(feature = "tokio")]
    {
      let mut completion = None;
      let mut disconnected = false;
      let mut finished = None;
      {
        let mut inner = self.inner.lock();
        if let Some(task) = inner.work.tokio_task.as_mut() {
          match task.receiver.try_recv() {
            Ok(received) => {
              if task.finish_on_message {
                disconnected = true;
              }
              completion = Some(received);
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => disconnected = true,
          }
        }
        if disconnected {
          finished = inner.work.tokio_task.take();
        }
      }
      drop(finished);

      if let Some(completion) = completion {
        completion();
        return true;
      }
    }

    false
  }
}

pub(super) fn noop_waker() -> Waker {
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
    let completion_task = async move {
      let result = future.await;
      let completion: FutureCompletion = Box::new(move || match result {
        Ok(data) => completion_state.set(FutureState::fulfilled(data)),
        Err(error) => {
          let previous_data = completion_state.get_untracked().data;
          completion_state.set(FutureState::rejected(error, previous_data));
        }
      });
      let _ = sender.send(completion);
    };
    task.spawn(&handle, completion_task, receiver, true);
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

fn start_stream_task<T, E, Fut>(
  state: Signal<FutureState<T, E>>,
  task: AsyncTask,
  runtime_handle: RuntimeFutureHandle,
  factory: impl FnOnce(StreamEmitter<T, E>) -> Fut,
) where
  T: SignalValue + Clone + PartialEq + Send + Sync + 'static,
  E: SignalValue + Clone + PartialEq + Send + Sync + 'static,
  Fut: Future<Output = ()> + Send + 'static,
{
  let previous_data = state.get_untracked().data;
  state.set(FutureState::pending(previous_data));

  #[cfg(feature = "tokio")]
  if let Some(handle) = runtime_handle {
    let (sender, receiver) = mpsc::channel::<FutureCompletion>();
    let emitter = StreamEmitter {
      state,
      sender: Some(sender),
    };
    task.spawn(&handle, factory(emitter), receiver, false);
    return;
  }

  #[cfg(not(feature = "tokio"))]
  let _ = runtime_handle;

  let emitter = StreamEmitter {
    state,
    #[cfg(feature = "tokio")]
    sender: None,
  };
  task.set(Box::pin(factory(emitter)));
}

impl Ctx {
  /// Runs a finite async operation and restarts it when `deps` changes between renders.
  ///
  /// Use this for requests, loads, and other one-shot work that has a single result.
  /// Do not use `future` to model a continuous subscription by manually changing a
  /// dependency after each completion; that creates a render-dependent re-arm gap.
  /// Use [`Ctx::stream`] for receiver/watch/event sources that can produce multiple
  /// values over time.
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

  /// Runs a continuous async producer and updates the handle state for every emitted item.
  ///
  /// The stream task starts on first render, restarts when `deps` changes between
  /// renders, and is cancelled when the component unmounts or stops calling
  /// `stream` at this cursor position. Call [`StreamEmitter::emit`] from the task
  /// for each item and [`StreamEmitter::reject`] to publish an error while keeping
  /// the stream alive.
  ///
  /// Use this for `watch::Receiver`, websocket/event subscriptions, file watchers,
  /// and other sources that can yield more than one value.
  pub fn stream<D, T, E, F, Fut>(&mut self, deps: D, factory: F) -> StreamHandle<T, E>
  where
    D: Clone + PartialEq + Send + Sync + 'static,
    T: SignalValue + Clone + PartialEq + Send + Sync + 'static,
    E: SignalValue + Clone + PartialEq + Send + Sync + 'static,
    F: Fn(D, StreamEmitter<T, E>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
  {
    let cursor = self.future_cursor;
    self.future_cursor += 1;
    let runtime_handle = self.runtime_future_handle();

    if cursor < self.future_slots.len() {
      let slot = &mut self.future_slots[cursor];
      if let Some(handle) = slot.handle.downcast_ref::<StreamHandle<T, E>>() {
        let deps_changed = slot.deps.as_ref().and_then(|old| old.downcast_ref::<D>()) != Some(&deps);
        let handle = handle.clone();
        if deps_changed {
          slot.deps = Some(Box::new(deps.clone()));
          start_stream_task(
            handle.state.clone(),
            handle.task.clone(),
            runtime_handle.clone(),
            move |emitter| factory(deps, emitter),
          );
        }
        return handle;
      }
    }

    let state = self.signal(FutureState::idle());
    let task = AsyncTask::new();
    let handle = StreamHandle {
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
    start_stream_task(state, task, runtime_handle, move |emitter| factory(deps, emitter));
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
}
