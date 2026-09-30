use std::{
  panic::{self, AssertUnwindSafe},
  sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
    mpsc,
  },
  thread,
  time::Duration,
};

use lurq::{
  app::ctx::Ctx,
  core::{Effect, Signal},
};

/// Runs `f` on its own thread so a deadlock fails the test instead of hanging it.
fn within_deadline<R: Send + 'static>(f: impl FnOnce() -> R + Send + 'static) -> R {
  let (done, finished) = mpsc::channel();
  thread::spawn(move || {
    let result = panic::catch_unwind(AssertUnwindSafe(f));
    let _ = done.send(result);
  });
  match finished.recv_timeout(Duration::from_secs(10)) {
    Ok(Ok(value)) => value,
    Ok(Err(payload)) => panic::resume_unwind(payload),
    Err(_) => panic!("deadlock: a signal write from a notification callback did not return"),
  }
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
  payload
    .downcast_ref::<String>()
    .cloned()
    .or_else(|| payload.downcast_ref::<&str>().map(|message| (*message).to_owned()))
    .unwrap_or_default()
}

#[test]
fn watch_callback_can_set_its_own_signal() {
  let seen = within_deadline(|| {
    let mut ctx = Ctx::new_root();
    let signal = ctx.signal(0_u32);
    let inner = signal.clone();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let log = seen.clone();
    ctx.watch(&signal, move |value| {
      log.lock().unwrap().push(*value);
      if *value == 1 {
        inner.set(2);
        assert_eq!(
          inner.get_untracked(),
          2,
          "a write inside a callback applies immediately"
        );
      }
    });
    signal.set(1);
    assert_eq!(signal.get(), 2);
    seen.lock().unwrap().clone()
  });
  assert_eq!(seen, vec![1, 2]);
}

#[test]
fn watch_callback_write_is_delivered_after_the_current_pass() {
  let (first, second) = within_deadline(|| {
    let mut ctx = Ctx::new_root();
    let signal = ctx.signal(0_u32);
    let inner = signal.clone();
    let first = Arc::new(Mutex::new(Vec::new()));
    let second = Arc::new(Mutex::new(Vec::new()));
    let first_log = first.clone();
    ctx.watch(&signal, move |value| {
      first_log.lock().unwrap().push(*value);
      if *value < 3 {
        inner.set(*value + 1);
      }
    });
    let second_log = second.clone();
    ctx.watch(&signal, move |value| second_log.lock().unwrap().push(*value));
    signal.set(1);
    let first = first.lock().unwrap().clone();
    let second = second.lock().unwrap().clone();
    (first, second)
  });
  assert_eq!(first, vec![1, 2, 3]);
  assert_eq!(second, vec![1, 2, 3], "every observer sees each pass in order");
}

#[test]
fn several_writes_in_one_callback_are_coalesced_to_the_latest_value() {
  let seen = within_deadline(|| {
    let mut ctx = Ctx::new_root();
    let signal = ctx.signal(0_u32);
    let inner = signal.clone();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let log = seen.clone();
    ctx.watch(&signal, move |value| {
      log.lock().unwrap().push(*value);
      if *value == 1 {
        inner.set(2);
        inner.set(3);
      }
    });
    signal.set(1);
    seen.lock().unwrap().clone()
  });
  assert_eq!(seen, vec![1, 3]);
}

#[test]
fn self_feeding_watch_stops_with_a_panic_and_the_signal_keeps_working() {
  let (message, value, seen_after) = within_deadline(|| {
    let mut ctx = Ctx::new_root();
    let signal = ctx.signal(0_u64);
    let inner = signal.clone();
    let armed = Arc::new(AtomicUsize::new(1));
    let armed_in_watch = armed.clone();
    let calls = Arc::new(AtomicUsize::new(0));
    let calls_in_watch = calls.clone();
    ctx.watch(&signal, move |value| {
      calls_in_watch.fetch_add(1, Ordering::SeqCst);
      if armed_in_watch.load(Ordering::SeqCst) == 1 {
        inner.set(*value + 1);
      }
    });
    let payload = panic::catch_unwind(AssertUnwindSafe(|| signal.set(1))).expect_err("an endless loop must not pass");
    armed.store(0, Ordering::SeqCst);
    let before = calls.load(Ordering::SeqCst);
    signal.set(7);
    (
      panic_message(payload.as_ref()),
      signal.get(),
      calls.load(Ordering::SeqCst) - before,
    )
  });
  assert!(message.contains("notification passes"), "unexpected panic: {message}");
  assert_eq!(value, 7);
  assert_eq!(seen_after, 1, "the signal still notifies after the loop was stopped");
}

#[test]
fn update_inside_a_watch_of_the_same_signal_panics_instead_of_deadlocking() {
  let message = within_deadline(|| {
    let mut ctx = Ctx::new_root();
    let signal = ctx.signal(0_u32);
    let inner = signal.clone();
    ctx.watch(&signal, move |value| {
      if *value == 1 {
        inner.update(|value| *value += 1);
      }
    });
    let payload = panic::catch_unwind(AssertUnwindSafe(|| signal.set(1))).expect_err("update must report misuse");
    panic_message(payload.as_ref())
  });
  assert!(message.contains("Signal::update"), "unexpected panic: {message}");
}

#[test]
fn update_of_another_signal_inside_a_watch_is_applied() {
  let value = within_deadline(|| {
    let mut ctx = Ctx::new_root();
    let source = ctx.signal(0_u32);
    let target = ctx.signal(10_u32);
    let inner = target.clone();
    ctx.watch(&source, move |value| inner.update(|target| *target += *value));
    source.set(5);
    target.get()
  });
  assert_eq!(value, 15);
}

#[test]
fn watch_can_subscribe_and_unsubscribe_on_its_own_signal() {
  let (late, removed) = within_deadline(|| {
    let mut ctx = Ctx::new_root();
    let signal = ctx.signal(0_u32);
    let late = Arc::new(Mutex::new(Vec::new()));
    let removed = Arc::new(Mutex::new(Vec::new()));

    let late_ctx: Arc<Mutex<Option<Ctx>>> = Arc::new(Mutex::new(None));
    let removable = Arc::new(Mutex::new(Some(Ctx::new_root())));

    let watched = signal.clone();
    let late_slot = late_ctx.clone();
    let late_log = late.clone();
    let removable_slot = removable.clone();
    ctx.watch(&signal, move |value| {
      if *value == 1 {
        let mut subscriber = Ctx::new_root();
        let log = late_log.clone();
        subscriber.watch(&watched, move |value| log.lock().unwrap().push(*value));
        *late_slot.lock().unwrap() = Some(subscriber);
        drop(removable_slot.lock().unwrap().take());
      }
    });
    // Registered after the watch above, so it would be called later in the same pass.
    let removed_log = removed.clone();
    removable
      .lock()
      .unwrap()
      .as_mut()
      .unwrap()
      .watch(&signal, move |value| removed_log.lock().unwrap().push(*value));

    signal.set(1);
    signal.set(2);
    let late = late.lock().unwrap().clone();
    let removed = removed.lock().unwrap().clone();
    (late, removed)
  });
  assert_eq!(
    late,
    vec![2],
    "a subscriber added during a pass receives the following passes"
  );
  assert!(
    removed.is_empty(),
    "a subscriber removed during a pass is not called for the rest of it: {removed:?}"
  );
}

#[test]
fn effect_that_writes_a_signal_it_reads_settles() {
  let (value, runs) = within_deadline(|| {
    let mut ctx = Ctx::new_root();
    let signal = ctx.signal(0_u32);
    let inner = signal.clone();
    let runs = Arc::new(AtomicUsize::new(0));
    let effect_runs = runs.clone();
    ctx.on_effect(move || {
      effect_runs.fetch_add(1, Ordering::SeqCst);
      let value = inner.get();
      if value % 2 == 1 {
        inner.set(value + 1);
      }
    });
    signal.set(3);
    (signal.get(), runs.load(Ordering::SeqCst))
  });
  assert_eq!(value, 4);
  assert_eq!(runs, 3, "initial run, the external write, then the effect's own write");
}

#[test]
fn effect_reruns_after_a_watch_writes_its_dependency() {
  let seen = within_deadline(|| {
    let mut ctx = Ctx::new_root();
    let source = ctx.signal(0_u32);
    let mirror = ctx.signal(0_u32);
    let seen = Arc::new(Mutex::new(Vec::new()));
    let log = seen.clone();
    let mirror_in_effect = mirror.clone();
    let source_in_effect = source.clone();
    ctx.on_effect(move || {
      let value = source_in_effect.get();
      log.lock().unwrap().push(value);
      mirror_in_effect.set(value);
    });
    let source_in_watch = source.clone();
    ctx.watch(&mirror, move |value| {
      if *value == 1 {
        source_in_watch.set(2);
      }
    });
    source.set(1);
    seen.lock().unwrap().clone()
  });
  assert_eq!(seen, vec![0, 1, 2]);
}

#[test]
fn signals_written_from_each_others_watches_settle_inside_a_batch() {
  let (a, b) = within_deadline(|| {
    let mut ctx = Ctx::new_root();
    let a = ctx.signal(0_u32);
    let b = ctx.signal(0_u32);
    let b_in_watch = b.clone();
    ctx.watch(&a, move |value| b_in_watch.set(*value * 2));
    let a_in_watch = a.clone();
    ctx.watch(&b, move |value| {
      if *value < 8 {
        a_in_watch.set(*value);
      }
    });
    ctx.batch(|| a.set(1));
    (a.get(), b.get())
  });
  assert_eq!((a, b), (4, 8));
}

#[test]
fn raw_signal_watch_through_effect_sets_the_signal_it_reads() {
  let value = within_deadline(|| {
    let signal = Signal::new(0_u32);
    let inner = signal.clone();
    let _effect = Effect::new(move || {
      let value = inner.get();
      if (1..5).contains(&value) {
        inner.set(value + 1);
      }
    });
    signal.set(1);
    signal.get()
  });
  assert_eq!(value, 5);
}
