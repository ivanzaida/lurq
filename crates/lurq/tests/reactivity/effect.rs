use std::sync::{Arc, atomic::{AtomicI32, Ordering}};
use lurq::core::{Effect, Signal};

#[test]
fn effect_runs_immediately() {
  let counter = Arc::new(AtomicI32::new(0));
  let c = counter.clone();
  let _effect = Effect::new(move || {
    c.fetch_add(1, Ordering::Relaxed);
  });
  assert_eq!(counter.load(Ordering::Relaxed), 1);
}

#[test]
fn effect_reruns_on_dependency_change() {
  let s = Signal::new(0);
  let sc = s.clone();
  let counter = Arc::new(AtomicI32::new(0));
  let c = counter.clone();
  let _effect = Effect::new(move || {
    let _ = sc.get();
    c.fetch_add(1, Ordering::Relaxed);
  });
  assert_eq!(counter.load(Ordering::Relaxed), 1);
  s.set(1);
  assert_eq!(counter.load(Ordering::Relaxed), 2);
}

#[test]
fn effect_tracks_multiple_signals() {
  let a = Signal::new(0);
  let b = Signal::new(0);
  let ac = a.clone();
  let bc = b.clone();
  let counter = Arc::new(AtomicI32::new(0));
  let c = counter.clone();
  let _effect = Effect::new(move || {
    let _ = ac.get();
    let _ = bc.get();
    c.fetch_add(1, Ordering::Relaxed);
  });
  assert_eq!(counter.load(Ordering::Relaxed), 1);
  a.set(1);
  assert_eq!(counter.load(Ordering::Relaxed), 2);
  b.set(1);
  assert_eq!(counter.load(Ordering::Relaxed), 3);
}

#[test]
fn effect_reads_current_value() {
  let s = Signal::new(0);
  let sc = s.clone();
  let seen = Arc::new(AtomicI32::new(-1));
  let v = seen.clone();
  let _effect = Effect::new(move || {
    v.store(sc.get(), Ordering::Relaxed);
  });
  assert_eq!(seen.load(Ordering::Relaxed), 0);
  s.set(42);
  assert_eq!(seen.load(Ordering::Relaxed), 42);
}

#[test]
fn dropping_effect_stops_reruns() {
  let s = Signal::new(0);
  let sc = s.clone();
  let counter = Arc::new(AtomicI32::new(0));
  let c = counter.clone();
  let effect = Effect::new(move || {
    let _ = sc.get();
    c.fetch_add(1, Ordering::Relaxed);
  });
  s.set(1);
  assert_eq!(counter.load(Ordering::Relaxed), 2);
  drop(effect);
  s.set(2);
  assert_eq!(counter.load(Ordering::Relaxed), 2);
}
