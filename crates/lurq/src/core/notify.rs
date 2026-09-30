//! Re-entrancy-safe notification building blocks shared by signals and effects.
//!
//! A notification pass never holds a lock while it calls observers, so an
//! observer may read or write the signal it observes and subscribe or
//! unsubscribe observers. A write made while a pass is running does not start a
//! nested pass: it marks the running pass dirty and the running pass repeats
//! once it has reached every observer, delivering the latest value.

use std::{
  cell::RefCell,
  sync::{
    Arc,
    atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering},
  },
};

use parking_lot::Mutex;

/// Consecutive passes a signal or effect may trigger for itself from its own
/// observers before the loop is reported as endless.
pub(crate) const MAX_SELF_TRIGGERED_PASSES: usize = 100;

const IDLE: u8 = 0;
const RUNNING: u8 = 1;
const DIRTY: u8 = 2;

thread_local! {
  /// Addresses of the notification loops this thread is running, innermost last.
  static ACTIVE_LOOPS: RefCell<Vec<usize>> = const { RefCell::new(Vec::new()) };
}

/// Runs notification passes one at a time and coalesces requests that arrive
/// while a pass is running into one more pass.
#[derive(Default)]
pub(crate) struct NotifyLoop {
  state: AtomicU8,
  self_triggered: AtomicBool,
}

impl NotifyLoop {
  /// Runs `pass` until no further pass was requested while it ran.
  ///
  /// Returns immediately when another pass of this loop is already running on
  /// any thread; that pass runs once more after its current round. Panics after
  /// [`MAX_SELF_TRIGGERED_PASSES`] consecutive rounds that were requested from
  /// the running pass itself, because such a loop would never settle.
  pub(crate) fn run(&self, kind: &str, id: usize, mut pass: impl FnMut()) {
    if !self.try_start() {
      return;
    }
    let mut guard = RunGuard::enter(self);
    let mut self_triggered = 0;
    loop {
      pass();
      if self
        .state
        .compare_exchange(RUNNING, IDLE, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
      {
        guard.finished = true;
        return;
      }
      self.state.store(RUNNING, Ordering::Release);
      if self.self_triggered.swap(false, Ordering::AcqRel) {
        self_triggered += 1;
        assert!(
          self_triggered <= MAX_SELF_TRIGGERED_PASSES,
          "{kind} {id} was written by its own observers on {MAX_SELF_TRIGGERED_PASSES} consecutive notification \
           passes; an observer writes what it observes on every change, so the notification passes never settle"
        );
      } else {
        self_triggered = 0;
      }
    }
  }

  /// Whether a running pass has been asked to repeat; the repeat reaches every
  /// observer registered before it starts.
  pub(crate) fn has_pending_pass(&self) -> bool {
    self.state.load(Ordering::Acquire) == DIRTY
  }

  /// Whether this thread is currently running a pass of this loop.
  pub(crate) fn is_running_on_this_thread(&self) -> bool {
    let address = self.address();
    ACTIVE_LOOPS.with(|active| active.borrow().contains(&address))
  }

  fn try_start(&self) -> bool {
    if self
      .state
      .compare_exchange(IDLE, RUNNING, Ordering::AcqRel, Ordering::Acquire)
      .is_ok()
    {
      return true;
    }
    // Only the running thread can finish the pass, so while this thread runs it
    // the state cannot fall back to idle underneath this flag.
    if self.is_running_on_this_thread() {
      self.self_triggered.store(true, Ordering::Release);
    }
    loop {
      match self
        .state
        .compare_exchange(RUNNING, DIRTY, Ordering::AcqRel, Ordering::Acquire)
      {
        Ok(_) | Err(DIRTY) => return false,
        Err(_) => {
          if self
            .state
            .compare_exchange(IDLE, RUNNING, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
          {
            return true;
          }
        }
      }
    }
  }

  fn address(&self) -> usize {
    std::ptr::from_ref(self) as usize
  }
}

/// Registers the running loop for this thread and resets the loop if a pass
/// panics, so a panicking observer does not leave it running forever.
struct RunGuard<'a> {
  owner: &'a NotifyLoop,
  finished: bool,
}

impl<'a> RunGuard<'a> {
  fn enter(owner: &'a NotifyLoop) -> Self {
    ACTIVE_LOOPS.with(|active| active.borrow_mut().push(owner.address()));
    Self { owner, finished: false }
  }
}

impl Drop for RunGuard<'_> {
  fn drop(&mut self) {
    let address = self.owner.address();
    ACTIVE_LOOPS.with(|active| {
      let mut active = active.borrow_mut();
      if let Some(position) = active.iter().rposition(|entry| *entry == address) {
        active.remove(position);
      }
    });
    if !self.finished {
      self.owner.self_triggered.store(false, Ordering::Release);
      self.owner.state.store(IDLE, Ordering::Release);
    }
  }
}

pub(crate) struct Observer<K, F: ?Sized> {
  pub(crate) id: usize,
  pub(crate) kind: K,
  pub(crate) callback: Arc<F>,
}

impl<K: Copy, F: ?Sized> Clone for Observer<K, F> {
  fn clone(&self) -> Self {
    Self {
      id: self.id,
      kind: self.kind,
      callback: Arc::clone(&self.callback),
    }
  }
}

type Snapshot<K, F> = Arc<Vec<Observer<K, F>>>;

/// Copy-on-write observer list: a pass iterates a shared snapshot without
/// holding the lock or allocating, and a change made during a pass copies the
/// list instead of waiting for the pass. The list is allocated on first use.
pub(crate) struct ObserverList<K, F: ?Sized> {
  entries: Mutex<Option<Snapshot<K, F>>>,
  removals: AtomicUsize,
}

impl<K: Copy, F: ?Sized> ObserverList<K, F> {
  pub(crate) fn new() -> Self {
    Self {
      entries: Mutex::new(None),
      removals: AtomicUsize::new(0),
    }
  }

  pub(crate) fn add(&self, id: usize, kind: K, callback: Arc<F>) {
    let mut entries = self.entries.lock();
    Arc::make_mut(entries.get_or_insert_with(Arc::default)).push(Observer { id, kind, callback });
  }

  pub(crate) fn remove(&self, id: usize) {
    let removed = {
      let mut entries = self.entries.lock();
      let Some(entries) = entries.as_mut() else {
        return;
      };
      let Some(index) = entries.iter().position(|observer| observer.id == id) else {
        return;
      };
      self.removals.fetch_add(1, Ordering::AcqRel);
      Arc::make_mut(entries).remove(index)
    };
    // Dropping the callback can drop captured subscriptions of this same list.
    drop(removed);
  }

  pub(crate) fn has_observers(&self) -> bool {
    self.entries.lock().as_ref().is_some_and(|entries| !entries.is_empty())
  }

  #[cfg(feature = "devtools")]
  pub(crate) fn count(&self, matches: impl Fn(K) -> bool) -> usize {
    self.entries.lock().as_ref().map_or(0, |entries| {
      entries.iter().filter(|observer| matches(observer.kind)).count()
    })
  }

  /// Calls `call` for every observer registered when the pass starts that is
  /// still registered when its turn comes.
  pub(crate) fn for_each(&self, mut call: impl FnMut(&F)) {
    let removals = self.removals.load(Ordering::Acquire);
    let Some(snapshot) = self.entries.lock().clone() else {
      return;
    };
    for observer in snapshot.iter() {
      if self.removals.load(Ordering::Acquire) != removals && !self.contains(observer.id) {
        continue;
      }
      call(&observer.callback);
    }
  }

  fn contains(&self, id: usize) -> bool {
    self
      .entries
      .lock()
      .as_ref()
      .is_some_and(|entries| entries.iter().any(|observer| observer.id == id))
  }
}
