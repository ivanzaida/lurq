//! Writes beside the cache's reads: a mutation runs an async call that changes data, says whether it is pending
//! and what went wrong, and tells the cache what the write changed.
//!
//! A run belongs to the client, not to the component that started it: it is polled by the client's tick, so it
//! finishes, and its effects on the cache apply, when its component is gone. A write is never deduplicated, and
//! never cancelled: a service may have accepted it already.

use super::*;

/// Implemented by the `#[lurq::mutation]` macro.
pub trait MutationDefinition: Send + Sync + 'static {
  type Args: Clone + Send + Sync + 'static;
  type Data: Send + Sync + 'static;
  type Error: Send + Sync + 'static;
  fn name() -> &'static str;
  fn run(args: Self::Args) -> QueryFuture<Self::Data, Self::Error>;
  /// Invalidates what the attribute's `invalidates` lists, after a success.
  fn invalidate(_client: &QueryClient) {}
}

/// A mutation and the arguments of one run of it. The generated function makes one, and starts nothing:
/// [`QueryClient::mutate`] runs it.
pub struct MutationCall<D: MutationDefinition> {
  args: D::Args,
}

impl<D: MutationDefinition> MutationCall<D> {
  #[doc(hidden)]
  pub fn new(args: D::Args) -> Self {
    Self { args }
  }
}

/// The function `#[lurq::mutation]` generates, as `ctx.mutation(rename_task)` takes it: it names the mutation,
/// and its arguments come with each run.
pub trait MutationFn<Marker> {
  type Definition: MutationDefinition;
}

macro_rules! mutation_fn {
  ($($argument:ident),*) => {
    impl<F, D, $($argument),*> MutationFn<($($argument,)*)> for F
    where
      F: Fn($($argument),*) -> MutationCall<D>,
      D: MutationDefinition,
    {
      type Definition = D;
    }
  };
}
mutation_fn!();
mutation_fn!(A1);
mutation_fn!(A1, A2);
mutation_fn!(A1, A2, A3);
mutation_fn!(A1, A2, A3, A4);
mutation_fn!(A1, A2, A3, A4, A5);
mutation_fn!(A1, A2, A3, A4, A5, A6);
mutation_fn!(A1, A2, A3, A4, A5, A6, A7);
mutation_fn!(A1, A2, A3, A4, A5, A6, A7, A8);

/// What a handle does with a run asked for while an earlier one of its is pending.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MutationConcurrency {
  /// It waits its turn: the handle's runs go one after another, in the order they were asked for. No write is
  /// lost, and none overtakes an earlier one.
  #[default]
  Queue,
  /// It starts at once, beside the others.
  Parallel,
  /// It is not run, and `run` returns `false`: for a button that must not send twice.
  Reject,
}

/// What a definition does, without its type: a handle is named by its arguments, its result and its error.
struct Definition<A, T, E> {
  run: fn(A) -> QueryFuture<T, E>,
  invalidate: fn(&QueryClient),
}

impl<A, T, E> Clone for Definition<A, T, E> {
  fn clone(&self) -> Self {
    *self
  }
}
impl<A, T, E> Copy for Definition<A, T, E> {}

impl<A, T, E> Definition<A, T, E> {
  fn of<D: MutationDefinition<Args = A, Data = T, Error = E>>() -> Self {
    Self {
      run: D::run,
      invalidate: D::invalidate,
    }
  }
}

type Snapshot = Box<dyn Any + Send>;
type OnSuccess<A, T> = Arc<dyn Fn(&QueryClient, &A, &T) + Send + Sync>;
type Optimistic<A> = Arc<dyn Fn(&QueryClient, &A) -> Snapshot + Send + Sync>;
type OnError<A, E> = Arc<dyn Fn(&QueryClient, &A, &E, Option<Snapshot>) + Send + Sync>;

/// What a run does to the cache: the handle's as it was when the run was asked for.
struct Effects<A, T, E> {
  on_success: Option<OnSuccess<A, T>>,
  optimistic: Option<Optimistic<A>>,
  on_error: Option<OnError<A, E>>,
}

impl<A, T, E> Clone for Effects<A, T, E> {
  fn clone(&self) -> Self {
    Self {
      on_success: self.on_success.clone(),
      optimistic: self.optimistic.clone(),
      on_error: self.on_error.clone(),
    }
  }
}

struct MutationValue<T, E> {
  data: Option<Arc<T>>,
  error: Option<Arc<E>>,
  /// The runs asked for and not finished: waiting or running.
  pending: usize,
}

struct Queued<A, T, E> {
  args: A,
  effects: Effects<A, T, E>,
  /// It waits for the runs before it to finish.
  serial: bool,
}

struct Running<A, T, E> {
  args: A,
  effects: Effects<A, T, E>,
  snapshot: Option<Snapshot>,
  task: Task<T, E>,
}

struct Work<A, T, E> {
  queue: VecDeque<Queued<A, T, E>>,
  running: Vec<Running<A, T, E>>,
  /// Whether the client's tick has it: while it has runs to start or to finish.
  active: bool,
}

/// The state of one `ctx.mutation`, or of one [`QueryClient::mutate`].
pub(super) struct MutationShared<A, T, E> {
  value: Mutex<MutationValue<T, E>>,
  revision: AtomicU64,
  // Locked before the value and before the client's list of mutations, never after them.
  work: Mutex<Work<A, T, E>>,
  definition: Definition<A, T, E>,
}

impl<A: Clone + Send + Sync + 'static, T: Send + Sync + 'static, E: Send + Sync + 'static> MutationShared<A, T, E> {
  fn new(definition: Definition<A, T, E>) -> Self {
    Self {
      definition,
      value: Mutex::new(MutationValue {
        data: None,
        error: None,
        pending: 0,
      }),
      revision: AtomicU64::new(0),
      work: Mutex::new(Work {
        queue: VecDeque::new(),
        running: Vec::new(),
        active: false,
      }),
    }
  }

  fn notify(&self) {
    self.revision.fetch_add(1, Ordering::Release);
  }

  /// Asks for a run. `false` when `concurrency` rejects it.
  fn enqueue(
    self: &Arc<Self>,
    client: &QueryClient,
    args: A,
    effects: Effects<A, T, E>,
    concurrency: MutationConcurrency,
  ) -> bool {
    {
      let mut work = self.work.lock();
      if concurrency == MutationConcurrency::Reject && !(work.queue.is_empty() && work.running.is_empty()) {
        return false;
      }
      work.queue.push_back(Queued {
        args,
        effects,
        serial: concurrency != MutationConcurrency::Parallel,
      });
      {
        let mut value = self.value.lock();
        value.pending += 1;
        value.error = None;
      }
      if !work.active {
        work.active = true;
        client.inner.mutations.lock().push(self.clone());
      }
    }
    self.notify();
    client.inner.wake_all();
    true
  }

  fn finish(&self, client: &QueryClient, run: Running<A, T, E>, result: Result<T, E>) {
    let Running {
      args,
      effects,
      snapshot,
      task,
    } = run;
    drop(task);
    let (data, error) = match result {
      Ok(data) => {
        let data = Arc::new(data);
        if let Some(on_success) = &effects.on_success {
          on_success(client, &args, &data);
        }
        (self.definition.invalidate)(client);
        (Some(data), None)
      }
      Err(error) => {
        let error = Arc::new(error);
        if let Some(on_error) = &effects.on_error {
          on_error(client, &args, &error, snapshot);
        }
        (None, Some(error))
      }
    };
    {
      let mut value = self.value.lock();
      value.pending -= 1;
      if let Some(data) = data {
        value.data = Some(data);
        value.error = None;
      }
      if error.is_some() {
        value.error = error;
      }
    }
    self.notify();
  }
}

pub(super) trait ErasedMutation: Send + Sync {
  fn revision(&self) -> u64;
  fn ready(&self, can_start: bool) -> bool;
  /// Starts the runs whose turn it is and polls the running ones. Called under the driver's lock.
  fn tick(&self, client: &QueryClient) -> bool;
}

impl<A: Clone + Send + Sync + 'static, T: Send + Sync + 'static, E: Send + Sync + 'static> ErasedMutation
  for MutationShared<A, T, E>
{
  fn revision(&self) -> u64 {
    self.revision.load(Ordering::Acquire)
  }

  fn ready(&self, can_start: bool) -> bool {
    let work = self.work.lock();
    (can_start
      && work
        .queue
        .front()
        .is_some_and(|next| !next.serial || work.running.is_empty()))
      || work
        .running
        .iter()
        .any(|run| run.task.wake.ready.load(Ordering::Acquire))
  }

  fn tick(&self, client: &QueryClient) -> bool {
    let mut changed = false;
    while client.inner.can_start() {
      let next = {
        let mut work = self.work.lock();
        if work
          .queue
          .front()
          .is_some_and(|next| !next.serial || work.running.is_empty())
        {
          work.queue.pop_front()
        } else {
          None
        }
      };
      let Some(Queued { args, effects, .. }) = next else {
        break;
      };
      // The optimistic write is made as the run starts: one that waits its turn writes over what the runs
      // before it left, not under them.
      let snapshot = effects.optimistic.as_ref().map(|optimistic| optimistic(client, &args));
      let task = client.inner.start(0, (self.definition.run)(args.clone()));
      self.work.lock().running.push(Running {
        args,
        effects,
        snapshot,
        task,
      });
      changed = true;
    }

    let running = std::mem::take(&mut self.work.lock().running);
    let mut kept = Vec::with_capacity(running.len());
    for mut run in running {
      let result = if run.task.wake.ready.swap(false, Ordering::AcqRel) {
        let waker = Waker::from(run.task.wake.clone());
        run.task.poll(&mut Context::from_waker(&waker))
      } else {
        Poll::Pending
      };
      match result {
        Poll::Pending => kept.push(run),
        Poll::Ready(TaskResult::Finished(result)) => {
          self.finish(client, run, result);
          changed = true;
        }
        // The runtime it ran on shut down under it. Whether the write was made is not known, so it is not
        // sent again: the run ends with neither a result nor an error.
        #[cfg(feature = "tokio")]
        Poll::Ready(TaskResult::RuntimeStopped(runtime)) => {
          client.inner.stopped_runtimes.lock().insert(runtime);
          self.value.lock().pending -= 1;
          self.notify();
          changed = true;
        }
      }
    }

    let mut work = self.work.lock();
    kept.append(&mut work.running);
    work.running = kept;
    if work.queue.is_empty() && work.running.is_empty() {
      work.active = false;
      let this = self as *const Self as *const ();
      client
        .inner
        .mutations
        .lock()
        .retain(|candidate| Arc::as_ptr(candidate) as *const () != this);
    }
    changed
  }
}

pub(super) fn run_detached<D: MutationDefinition>(client: &QueryClient, call: MutationCall<D>) {
  let effects = Effects {
    on_success: None,
    optimistic: None,
    on_error: None,
  };
  Arc::new(MutationShared::new(Definition::of::<D>())).enqueue(
    client,
    call.args,
    effects,
    MutationConcurrency::Parallel,
  );
}

/// A mutation as a component holds it: what runs it, and a reactive view of its runs.
///
/// `S` is what [`optimistic`](Self::optimistic) hands to [`on_error`](Self::on_error): `()` without it.
///
/// A handle outlives a render, and a clone of it (in a press handler) runs the mutation with the effects the
/// handle had when it was cloned. Its state is its component's: after the component unmounts the handle still
/// runs the mutation, and what it reads no longer changes.
pub struct MutationHandle<A, T, E, S = ()> {
  shared: Arc<MutationShared<A, T, E>>,
  revision: Signal<u64>,
  client: QueryClient,
  effects: Effects<A, T, E>,
  concurrency: MutationConcurrency,
  snapshot: PhantomData<fn() -> S>,
}

impl<A, T, E, S> Clone for MutationHandle<A, T, E, S> {
  fn clone(&self) -> Self {
    Self {
      shared: self.shared.clone(),
      revision: self.revision.clone(),
      client: self.client.clone(),
      effects: self.effects.clone(),
      concurrency: self.concurrency,
      snapshot: PhantomData,
    }
  }
}

impl<A: Clone + Send + Sync + 'static, T: Send + Sync + 'static, E: Send + Sync + 'static, S: Send + 'static>
  MutationHandle<A, T, E, S>
{
  /// Asks for a run with `args`: the tuple of the mutation's arguments. It returns at once, and the run goes on
  /// when the component that asked for it is gone. `false` when the run was rejected
  /// ([`MutationConcurrency::Reject`], with one pending).
  ///
  /// A run asked for while another is pending waits its turn, unless [`concurrency`](Self::concurrency) says
  /// otherwise.
  pub fn run(&self, args: A) -> bool {
    self
      .shared
      .enqueue(&self.client, args, self.effects.clone(), self.concurrency)
  }

  /// Whether a run is waiting or on its way.
  pub fn pending(&self) -> bool {
    self.revision.get();
    self.shared.value.lock().pending > 0
  }

  /// What the last run that failed failed with. The next run clears it, and so does a success.
  pub fn error(&self) -> Option<Arc<E>> {
    self.revision.get();
    self.shared.value.lock().error.clone()
  }

  /// What the last successful run returned.
  pub fn data(&self) -> Option<Arc<T>> {
    self.revision.get();
    self.shared.value.lock().data.clone()
  }

  /// Forgets the last result and the last error. Runs on their way go on.
  pub fn reset(&self) {
    {
      let mut value = self.shared.value.lock();
      value.data = None;
      value.error = None;
    }
    self.shared.notify();
    self.client.inner.wake_all();
  }

  /// What a run asked for while another is pending does: it waits its turn unless told otherwise.
  pub fn concurrency(mut self, concurrency: MutationConcurrency) -> Self {
    self.concurrency = concurrency;
    self
  }

  /// Called after a run succeeds, with the client, the run's arguments and its result, before the definition's
  /// `invalidates` apply: where the cache is told what the write changed. A second `on_success` is called after
  /// the first.
  ///
  /// It is called when the run finishes, on the thread of a window that has the client, whether or not the
  /// component is still mounted.
  pub fn on_success(mut self, effect: impl Fn(&QueryClient, &A, &T) + Send + Sync + 'static) -> Self {
    self.effects.on_success = Some(match self.effects.on_success.take() {
      Some(first) => Arc::new(move |client, args, data| {
        first(client, args, data);
        effect(client, args, data);
      }),
      None => Arc::new(effect),
    });
    self
  }

  /// Called after a run fails, with the client, the run's arguments, its error, and what
  /// [`optimistic`](Self::optimistic) returned as the run started: where an optimistic write is taken back. A
  /// second `on_error` is not called with the snapshot, which the first took: it replaces the first.
  pub fn on_error(mut self, effect: impl Fn(&QueryClient, &A, &E, S) + Send + Sync + 'static) -> Self {
    self.effects.on_error = Some(Arc::new(move |client, args, error, snapshot| {
      // Without an optimistic write the snapshot is `()`.
      let snapshot = snapshot.unwrap_or_else(|| Box::new(()));
      if let Ok(snapshot) = snapshot.downcast::<S>() {
        effect(client, args, error, *snapshot);
      }
    }));
    self
  }
}

impl<A: Clone + Send + Sync + 'static, T: Send + Sync + 'static, E: Send + Sync + 'static> MutationHandle<A, T, E> {
  /// Called as a run starts, before its request is sent, with the client and the run's arguments: where the
  /// cache is written as the write will leave it ([`QueryClient::update_data`]). What it returns, the value to
  /// go back to, is handed to [`on_error`](Self::on_error) if the run fails.
  ///
  /// Set it before `on_error`, which takes what this returns.
  ///
  /// # Panics
  ///
  /// When `on_error` was set first: its snapshot would be of another type.
  pub fn optimistic<S: Send + 'static>(
    mut self,
    write: impl Fn(&QueryClient, &A) -> S + Send + Sync + 'static,
  ) -> MutationHandle<A, T, E, S> {
    assert!(
      self.effects.on_error.is_none(),
      "MutationHandle::optimistic must be set before on_error, which takes its snapshot"
    );
    self.effects.optimistic = Some(Arc::new(move |client, args| Box::new(write(client, args)) as Snapshot));
    MutationHandle {
      shared: self.shared,
      revision: self.revision,
      client: self.client,
      effects: self.effects,
      concurrency: self.concurrency,
      snapshot: PhantomData,
    }
  }
}

/// The mutation of one `ctx.mutation` call, kept by its component between renders.
pub(crate) struct MutationSlot {
  shared: Arc<dyn Any + Send + Sync>,
  erased: Arc<dyn ErasedMutation>,
  revision: Signal<u64>,
  client: QueryClient,
  definition: TypeId,
}

impl MutationSlot {
  pub(crate) fn new<D: MutationDefinition>(client: &QueryClient) -> Self {
    let shared = Arc::new(MutationShared::new(Definition::of::<D>()));
    Self {
      definition: TypeId::of::<D>(),
      erased: shared.clone(),
      shared,
      revision: Signal::new(0),
      client: client.clone(),
    }
  }

  /// The slot's handle, if the slot is of this mutation in this client.
  pub(crate) fn handle<D: MutationDefinition>(
    &self,
    client: &QueryClient,
  ) -> Option<MutationHandle<D::Args, D::Data, D::Error>> {
    if !Arc::ptr_eq(&self.client.inner, &client.inner) || self.definition != TypeId::of::<D>() {
      return None;
    }
    let shared = self
      .shared
      .clone()
      .downcast::<MutationShared<D::Args, D::Data, D::Error>>()
      .ok()?;
    Some(MutationHandle {
      shared,
      revision: self.revision.clone(),
      client: self.client.clone(),
      effects: Effects {
        on_success: None,
        optimistic: None,
        on_error: None,
      },
      concurrency: MutationConcurrency::default(),
      snapshot: PhantomData,
    })
  }

  // Publish on this slot's tree thread, never on another tree's poller.
  pub(crate) fn publish(&self) -> bool {
    let revision = self.erased.revision();
    if self.revision.get_untracked() == revision {
      return false;
    }
    self.revision.set(revision);
    true
  }
}
