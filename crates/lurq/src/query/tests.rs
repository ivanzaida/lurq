use super::*;
use crate::app::ctx::Ctx;

#[derive(Default)]
struct Gate {
  result: Mutex<Option<Result<u64, String>>>,
  waker: Mutex<Option<Waker>>,
}

impl Gate {
  fn resolve(&self, value: Result<u64, String>) {
    *self.result.lock() = Some(value);
    if let Some(waker) = self.waker.lock().take() {
      waker.wake();
    }
  }
}

struct Wait(Arc<Gate>);
impl Future for Wait {
  type Output = Result<u64, String>;
  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    *self.0.waker.lock() = Some(cx.waker().clone());
    self.0.result.lock().take().map_or(Poll::Pending, Poll::Ready)
  }
}

#[derive(Default)]
struct Probe {
  calls: Mutex<Vec<u64>>,
  gates: Mutex<VecDeque<Arc<Gate>>>,
}

impl Probe {
  fn gate(&self) -> Arc<Gate> {
    let gate = Arc::new(Gate::default());
    self.gates.lock().push_back(gate.clone());
    gate
  }
}

#[derive(Clone)]
struct Key(Arc<Probe>, u64);
impl PartialEq for Key {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0) && self.1 == other.1
  }
}
impl Eq for Key {}
impl Hash for Key {
  fn hash<H: Hasher>(&self, state: &mut H) {
    0_u8.hash(state);
  } // Deliberate collisions.
}

#[crate::query]
async fn read(key: Key) -> Result<u64, String> {
  key.0.calls.lock().push(key.1);
  let gate = key.0.gates.lock().pop_front();
  match gate {
    Some(gate) => Wait(gate).await,
    None => Ok(key.1),
  }
}

#[crate::query]
async fn other(key: Key) -> Result<u64, String> {
  Ok(key.1 + 100)
}

fn tick(client: &QueryClient, now: Instant) {
  let waker = Waker::from(Arc::new(TaskWake {
    ready: AtomicBool::new(true),
    client: Weak::new(),
  }));
  client.tick(now, &mut Context::from_waker(&waker));
}

#[test]
fn descriptors_are_lazy_requests_are_shared_and_hash_collisions_are_safe() {
  let client = QueryClient::new();
  let probe = Arc::new(Probe::default());
  let descriptor = read(Key(probe.clone(), 1));
  assert!(probe.calls.lock().is_empty());
  let now = Instant::now();
  let (first, _a) = client.observe(descriptor.clone(), now);
  let (second, _b) = client.observe(descriptor, now);
  let (different, _c) = client.observe(read(Key(probe.clone(), 2)), now);
  let (family, _d) = client.observe(other(Key(probe.clone(), 1)), now);
  assert!(first.loading());
  tick(&client, now);
  assert_eq!(*first.data().unwrap(), 1);
  assert!(Arc::ptr_eq(&first.data().unwrap(), &second.data().unwrap()));
  assert_eq!(*different.data().unwrap(), 2);
  assert_eq!(*family.data().unwrap(), 101);
  assert_eq!(probe.calls.lock().len(), 2);
  assert_eq!(client.inspect().len(), 3);
}

#[test]
fn exact_and_family_invalidation_respect_observers_and_client_scope() {
  let client = QueryClient::new();
  let isolated = QueryClient::new();
  let probe = Arc::new(Probe::default());
  let now = Instant::now();
  let (one, _a) = client.observe(read(Key(probe.clone(), 1)), now);
  let (_two, b) = client.observe(read(Key(probe.clone(), 2)), now);
  let (_other, _c) = client.observe(other(Key(probe.clone(), 1)), now);
  let (_isolated, _d) = isolated.observe(read(Key(probe.clone(), 1)), now);
  tick(&client, now);
  tick(&isolated, now);
  assert_eq!(probe.calls.lock().len(), 3);
  drop(b);
  client.invalidate(read(Key(probe.clone(), 999)));
  tick(&client, now);
  assert_eq!(client.inspect().len(), 3);
  assert_eq!(probe.calls.lock().len(), 3);
  one.invalidate();
  one.invalidate();
  tick(&client, now);
  assert_eq!(probe.calls.lock().len(), 4);
  client.invalidate(read::all());
  tick(&client, now);
  tick(&isolated, now);
  assert_eq!(probe.calls.lock().len(), 5);
  let read_entries: Vec<_> = client
    .inspect()
    .into_iter()
    .filter(|entry| entry.name.ends_with("::read"))
    .collect();
  assert_eq!(read_entries.iter().filter(|entry| entry.stale).count(), 1);
  let (_two, _b) = client.observe(read(Key(probe.clone(), 2)), now);
  tick(&client, now);
  assert_eq!(probe.calls.lock().len(), 6);
}

#[test]
fn obsolete_completion_cannot_overwrite_a_replacement() {
  let client = QueryClient::new();
  let probe = Arc::new(Probe::default());
  let old = probe.gate();
  let now = Instant::now();
  let (handle, _observer) = client.observe(read(Key(probe.clone(), 1)), now);
  tick(&client, now);
  let replacement = probe.gate();
  handle.invalidate();
  tick(&client, now);
  replacement.resolve(Ok(20));
  tick(&client, now);
  old.resolve(Ok(10));
  tick(&client, now);
  assert_eq!(*handle.data().unwrap(), 20);
  assert!(!handle.fetching());
  assert!(!client.inspect()[0].stale);
  assert_eq!(probe.calls.lock().len(), 2);
}

#[test]
fn refresh_retains_data_and_errors_do_not_create_retry_loops() {
  let client = QueryClient::new();
  let probe = Arc::new(Probe::default());
  let now = Instant::now();
  let (handle, _observer) = client.observe(read(Key(probe.clone(), 1)), now);
  tick(&client, now);
  let failure = probe.gate();
  handle.refresh();
  handle.refresh();
  tick(&client, now);
  assert!(!handle.loading());
  assert!(handle.fetching());
  assert_eq!(*handle.data().unwrap(), 1);
  failure.resolve(Err("offline".into()));
  tick(&client, now);
  assert_eq!(&*handle.error().unwrap(), "offline");
  assert_eq!(*handle.data().unwrap(), 1);
  assert!(!handle.fetching());
  for _ in 0..5 {
    tick(&client, now);
  }
  assert_eq!(probe.calls.lock().len(), 2);
}

#[test]
fn freshness_and_retention_have_independent_deadlines() {
  let client = QueryClient::with_options(QueryClientOptions {
    stale_time: Duration::from_secs(1),
    gc_time: Duration::from_secs(10),
  });
  let probe = Arc::new(Probe::default());
  let now = Instant::now();
  let (handle, observer) = client.observe(read(Key(probe.clone(), 1)), now);
  tick(&client, now);
  drop(observer);
  let (_, observer) = client.observe(read(Key(probe.clone(), 1)), now + Duration::from_millis(500));
  tick(&client, now + Duration::from_millis(500));
  assert_eq!(probe.calls.lock().len(), 1);
  drop(observer);
  tick(&client, now + Duration::from_secs(2));
  assert_eq!(probe.calls.lock().len(), 1); // Time alone does not refetch.
  let (_, observer) = client.observe(read(Key(probe.clone(), 1)), now + Duration::from_secs(2));
  assert!(handle.fetching());
  assert_eq!(*handle.data().unwrap(), 1);
  tick(&client, now + Duration::from_secs(2));
  assert_eq!(probe.calls.lock().len(), 2);
  drop(observer);
  tick(&client, Instant::now() + Duration::from_secs(11));
  assert!(client.inspect().is_empty());
  assert!(handle.data().is_none());
  handle.refresh();
  tick(&client, Instant::now());
  assert!(client.inspect().is_empty());
}

#[test]
fn last_unmount_allows_completion_but_eviction_rejects_late_results() {
  let client = QueryClient::with_options(QueryClientOptions {
    gc_time: Duration::from_secs(1),
    ..Default::default()
  });
  let probe = Arc::new(Probe::default());
  let gate = probe.gate();
  let now = Instant::now();
  let (handle, observer) = client.observe(read(Key(probe.clone(), 1)), now);
  tick(&client, now);
  drop(observer);
  gate.resolve(Ok(5));
  tick(&client, now);
  assert_eq!(*handle.data().unwrap(), 5);
  let late = probe.gate();
  handle.refresh();
  tick(&client, now);
  tick(&client, Instant::now() + Duration::from_secs(2));
  late.resolve(Ok(9));
  tick(&client, Instant::now());
  assert!(handle.data().is_none());
  assert!(client.inspect().is_empty());
}

#[test]
fn component_slots_are_stable_and_release_removed_queries() {
  let mut ctx = Ctx::new_root();
  let client = QueryClient::new();
  let probe = Arc::new(Probe::default());
  ctx.provide(client.clone());
  ctx.begin_render();
  let handle = ctx.query(read(Key(probe.clone(), 1)));
  handle.data();
  ctx.end_render();
  ctx.tick_futures();
  assert!(ctx.is_dirty());
  for _ in 0..3 {
    ctx.begin_render();
    ctx.query(read(Key(probe.clone(), 1))).data();
    ctx.end_render();
  }
  assert_eq!(probe.calls.lock().len(), 1);
  assert_eq!(client.inspect()[0].observers, 1);
  ctx.begin_render();
  let next = ctx.query(read(Key(probe.clone(), 2)));
  assert!(next.data().is_none());
  ctx.end_render();
  assert_eq!(client.inspect().iter().map(|entry| entry.observers).sum::<usize>(), 1);
  ctx.begin_render();
  ctx.end_render();
  assert!(client.inspect().iter().all(|entry| entry.observers == 0));
  assert!(ctx.next_query_deadline().is_some());
  ctx.tick_futures(); // Registry still runs without a query slot.
  assert_eq!(*next.data().unwrap(), 2);
}

#[test]
fn client_replacement_rebinds_observers_and_retained_clients_keep_their_cache() {
  let mut ctx = Ctx::new_root();
  let old_client = QueryClient::new();
  let new_client = QueryClient::new();
  let probe = Arc::new(Probe::default());
  let old_gate = probe.gate();
  ctx.provide(old_client.clone());
  ctx.begin_render();
  let old_handle = ctx.query(read(Key(probe.clone(), 1)));
  ctx.end_render();
  ctx.tick_futures();
  ctx.provide(new_client.clone());
  ctx.begin_render();
  let new_handle = ctx.query(read(Key(probe.clone(), 1)));
  ctx.end_render();
  ctx.tick_futures();
  old_gate.resolve(Ok(99));
  ctx.tick_futures();
  assert_eq!(*new_handle.data().unwrap(), 1);
  assert_eq!(*old_handle.data().unwrap(), 99); // Isolated old client, not the new session.
  drop(ctx);
  assert_eq!(old_client.inspect()[0].observers, 0);
  assert_eq!(new_client.inspect()[0].observers, 0);
  assert_eq!(*new_handle.data().unwrap(), 1);
}

#[test]
fn shared_client_publishes_on_each_tree_tick_without_duplicate_fetches() {
  let client = QueryClient::new();
  let probe = Arc::new(Probe::default());
  let mut first = Ctx::new_root();
  let mut second = Ctx::new_root();
  first.provide(client.clone());
  second.provide(client.clone());
  first.begin_render();
  let one = first.query(read(Key(probe.clone(), 7)));
  one.data();
  first.end_render();
  second.begin_render();
  let two = second.query(read(Key(probe.clone(), 7)));
  two.data();
  second.end_render();
  assert_eq!(client.inspect()[0].observers, 2);
  assert!(first.tick_futures());
  assert!(first.is_dirty());
  assert!(!second.is_dirty()); // Another tree cannot publish our signals.
  assert!(second.has_active_futures());
  assert!(second.tick_futures());
  assert!(second.is_dirty());
  assert_eq!(probe.calls.lock().len(), 1);
  assert!(Arc::ptr_eq(&one.data().unwrap(), &two.data().unwrap()));
  assert!(!first.tick_futures());
  assert!(!second.tick_futures());

  drop(first);
  assert_eq!(client.inspect()[0].observers, 1);
  two.invalidate();
  second.tick_futures();
  assert_eq!(probe.calls.lock().len(), 2);
}

#[test]
fn notifications_stay_on_the_observers_tree_thread() {
  use std::sync::mpsc;
  let client = QueryClient::new();
  let probe = Arc::new(Probe::default());
  let mut first = Ctx::new_root();
  first.provide(client.clone());
  first.begin_render();
  first.query(read(Key(probe.clone(), 1))).data();
  first.end_render();
  let (command, receiver) = mpsc::channel::<()>();
  let (reply, replies) = mpsc::channel();
  let threads = Arc::new(Mutex::new(Vec::new()));
  let callbacks = threads.clone();
  let second_client = client.clone();
  let second_probe = probe.clone();
  let thread = std::thread::spawn(move || {
    let mut second = Ctx::new_root();
    second.provide(second_client);
    second.begin_render();
    let handle = second.query(read(Key(second_probe, 1)));
    second.end_render();
    second.on_effect(move || {
      handle.data();
      callbacks.lock().push(std::thread::current().id());
    });
    reply.send(std::thread::current().id()).unwrap();
    receiver.recv().unwrap();
    assert!(second.tick_futures());
    reply.send(std::thread::current().id()).unwrap();
  });
  let owner = replies.recv().unwrap();
  first.tick_futures();
  assert_eq!(threads.lock().as_slice(), &[owner]);
  command.send(()).unwrap();
  replies.recv().unwrap();
  thread.join().unwrap();
  assert_eq!(threads.lock().as_slice(), &[owner, owner]);
  assert_eq!(probe.calls.lock().len(), 1);
}

#[test]
fn simultaneous_drivers_and_observers_share_one_pending_request() {
  let client = QueryClient::new();
  let probe = Arc::new(Probe::default());
  let gate = probe.gate();
  let barrier = Arc::new(std::sync::Barrier::new(3));
  let mut threads = Vec::new();
  for _ in 0..2 {
    let client = client.clone();
    let probe = probe.clone();
    let barrier = barrier.clone();
    threads.push(std::thread::spawn(move || {
      let mut ctx = Ctx::new_root();
      ctx.provide(client);
      barrier.wait();
      ctx.begin_render();
      let handle = ctx.query(read(Key(probe, 4)));
      ctx.end_render();
      ctx.tick_futures();
      barrier.wait();
      barrier.wait();
      ctx.tick_futures();
      assert_eq!(*handle.data().unwrap(), 12);
    }));
  }
  barrier.wait();
  barrier.wait();
  assert_eq!(probe.calls.lock().len(), 1);
  gate.resolve(Ok(12));
  barrier.wait();
  for thread in threads {
    thread.join().unwrap();
  }
  assert_eq!(probe.calls.lock().len(), 1);
}

#[test]
fn future_wakers_follow_surviving_trees_and_ignore_detached_ones() {
  use std::sync::atomic::AtomicUsize;
  let first = QueryRegistry::default();
  let second = QueryRegistry::default();
  let client = QueryClient::new();
  let one = Arc::new(AtomicUsize::new(0));
  let two = Arc::new(AtomicUsize::new(0));
  for (registry, counter) in [(&first, one.clone()), (&second, two.clone())] {
    registry.attach(
      &client,
      Some(Arc::new(move || {
        counter.fetch_add(1, Ordering::Relaxed);
      })),
      #[cfg(feature = "tokio")]
      None,
    );
  }
  let probe = Arc::new(Probe::default());
  let gate = probe.gate();
  let now = Instant::now();
  let (handle, _observer) = client.observe(read(Key(probe.clone(), 3)), now);
  tick(&client, now);
  one.store(0, Ordering::Relaxed);
  two.store(0, Ordering::Relaxed);
  handle.invalidate();
  assert!(one.load(Ordering::Relaxed) > 0);
  assert!(two.load(Ordering::Relaxed) > 0);
  let replacement = probe.gate();
  tick(&client, now);
  drop(first);
  one.store(0, Ordering::Relaxed);
  two.store(0, Ordering::Relaxed);
  replacement.resolve(Ok(9));
  assert_eq!(one.load(Ordering::Relaxed), 0);
  assert!(two.load(Ordering::Relaxed) > 0);
  tick(&client, now);
  assert_eq!(*handle.data().unwrap(), 9);
  assert_eq!(probe.calls.lock().len(), 2); // Detach did not restart the request.
  drop(second);
  one.store(0, Ordering::Relaxed);
  two.store(0, Ordering::Relaxed);
  gate.resolve(Ok(1));
  assert_eq!(one.load(Ordering::Relaxed), 0);
  assert_eq!(two.load(Ordering::Relaxed), 0);
}

#[test]
fn last_tree_detaches_without_erasing_data_and_reattachment_honors_retention() {
  let client = QueryClient::new();
  let probe = Arc::new(Probe::default());
  let mut first = Ctx::new_root();
  first.provide(client.clone());
  first.begin_render();
  let saved = first.query(read(Key(probe.clone(), 8)));
  first.end_render();
  first.tick_futures();
  let late = probe.gate();
  saved.refresh();
  first.tick_futures();
  drop(first);
  assert!(!saved.fetching());
  assert_eq!(*saved.data().unwrap(), 8);
  late.resolve(Ok(99));
  let mut second = Ctx::new_root();
  second.provide(client.clone());
  second.begin_render();
  let next = second.query(read(Key(probe.clone(), 8)));
  second.end_render();
  second.tick_futures();
  assert_eq!(*next.data().unwrap(), 8);
  assert_eq!(probe.calls.lock().len(), 3);
  drop(second);
  let after_expiry = Instant::now() + Duration::from_secs(301);
  let (fresh, _observer) = client.observe(read(Key(probe.clone(), 8)), after_expiry);
  assert!(fresh.data().is_none());
  tick(&client, after_expiry);
  assert_eq!(probe.calls.lock().len(), 4);
}

#[test]
fn cross_thread_commands_and_future_wakes_wake_the_runtime() {
  let registry = QueryRegistry::default();
  let client = QueryClient::new();
  let wakes = Arc::new(std::sync::atomic::AtomicUsize::new(0));
  let counter = wakes.clone();
  registry.attach(
    &client,
    Some(Arc::new(move || {
      counter.fetch_add(1, Ordering::Relaxed);
    })),
    #[cfg(feature = "tokio")]
    None,
  );
  let probe = Arc::new(Probe::default());
  let gate = probe.gate();
  let now = Instant::now();
  let (handle, _observer) = client.observe(read(Key(probe, 1)), now);
  tick(&client, now);
  assert!(!client.ready());
  std::thread::spawn(move || gate.resolve(Ok(7))).join().unwrap();
  assert!(client.ready());
  assert!(wakes.load(Ordering::Relaxed) > 0);
  assert!(handle.data().is_none()); // Completion publication stays on the UI tick.
  tick(&client, now);
  let baseline = wakes.load(Ordering::Relaxed);
  std::thread::spawn(move || handle.invalidate()).join().unwrap();
  assert!(wakes.load(Ordering::Relaxed) > baseline);
  assert!(client.ready());
}

#[cfg(feature = "tokio")]
#[test]
fn tokio_queries_publish_on_ui_tick_and_obsolete_tasks_are_aborted() {
  let runtime = tokio::runtime::Builder::new_current_thread().build().unwrap();
  let registry = QueryRegistry::default();
  let client = QueryClient::new();
  registry.attach(&client, None, Some(runtime.handle().clone()));
  let probe = Arc::new(Probe::default());
  let gate = probe.gate();
  let now = Instant::now();
  let (handle, _observer) = client.observe(read(Key(probe.clone(), 1)), now);
  tick(&client, now);
  runtime.block_on(async {
    tokio::task::yield_now().await;
  });
  assert_eq!(probe.calls.lock().len(), 1);
  handle.invalidate();
  tick(&client, now);
  runtime.block_on(async {
    tokio::task::yield_now().await;
  });
  assert!(handle.data().is_none());
  tick(&client, now);
  assert_eq!(*handle.data().unwrap(), 1);
  gate.resolve(Ok(999));
  runtime.block_on(async {
    tokio::task::yield_now().await;
  });
  tick(&client, now);
  assert_eq!(*handle.data().unwrap(), 1);
}

#[cfg(feature = "tokio")]
#[test]
fn closing_the_starting_tree_does_not_abort_shared_tokio_work() {
  let runtime = tokio::runtime::Builder::new_current_thread().build().unwrap();
  let first = QueryRegistry::default();
  let second = QueryRegistry::default();
  let client = QueryClient::new();
  first.attach(&client, None, Some(runtime.handle().clone()));
  second.attach(&client, None, Some(runtime.handle().clone()));
  let probe = Arc::new(Probe::default());
  let gate = probe.gate();
  let now = Instant::now();
  let (handle, _observer) = client.observe(read(Key(probe.clone(), 1)), now);
  tick(&client, now);
  runtime.block_on(async {
    tokio::task::yield_now().await;
  });
  drop(first);
  gate.resolve(Ok(12));
  runtime.block_on(async {
    tokio::task::yield_now().await;
  });
  tick(&client, now);
  assert_eq!(*handle.data().unwrap(), 12);
  assert_eq!(probe.calls.lock().len(), 1);
}

#[cfg(feature = "tokio")]
#[test]
fn stopped_tokio_runtime_falls_back_and_waits_without_spinning_if_none_survive() {
  let first_runtime = tokio::runtime::Builder::new_current_thread().build().unwrap();
  let second_runtime = tokio::runtime::Builder::new_current_thread().build().unwrap();
  let first = QueryRegistry::default();
  let second = QueryRegistry::default();
  let client = QueryClient::new();
  first.attach(&client, None, Some(first_runtime.handle().clone()));
  second.attach(&client, None, Some(second_runtime.handle().clone()));
  let probe = Arc::new(Probe::default());
  let old = probe.gate();
  let now = Instant::now();
  let (handle, _observer) = client.observe(read(Key(probe.clone(), 1)), now);
  tick(&client, now);
  first_runtime.block_on(async {
    tokio::task::yield_now().await;
  });
  drop(first_runtime);
  tick(&client, now); // Recognize runtime shutdown.
  tick(&client, now); // Start on the surviving runtime.
  second_runtime.block_on(async {
    tokio::task::yield_now().await;
  });
  tick(&client, now);
  assert_eq!(*handle.data().unwrap(), 1);
  assert_eq!(probe.calls.lock().len(), 2);
  old.resolve(Ok(999));
  let pending = probe.gate();
  handle.refresh();
  tick(&client, now);
  second_runtime.block_on(async {
    tokio::task::yield_now().await;
  });
  drop(second_runtime);
  tick(&client, now);
  assert!(!client.ready()); // Queued work must not busy-poll a stopped runtime.
  let replacement = tokio::runtime::Builder::new_current_thread().build().unwrap();
  second.attach(&client, None, Some(replacement.handle().clone()));
  assert!(client.ready());
  tick(&client, now);
  replacement.block_on(async {
    tokio::task::yield_now().await;
  });
  tick(&client, now);
  pending.resolve(Ok(888));
  tick(&client, now);
  assert_eq!(*handle.data().unwrap(), 1);
  assert_eq!(probe.calls.lock().len(), 4);
}
