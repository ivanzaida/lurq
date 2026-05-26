use lurq::core::{Memo, Signal};

#[test]
fn memo_computes_initial_value() {
  let s = Signal::new(3);
  let m = Memo::new(move || s.get() * 2);
  assert_eq!(m.get(), 6);
}

#[test]
fn memo_recomputes_on_dependency_change() {
  let s = Signal::new(5);
  let sc = s.clone();
  let m = Memo::new(move || sc.get() * 10);
  assert_eq!(m.get(), 50);
  s.set(3);
  assert_eq!(m.get(), 30);
}

#[test]
fn memo_with_accesses_value() {
  let s = Signal::new(7);
  let sc = s.clone();
  let m = Memo::new(move || sc.get() + 1);
  let result = m.with(|v| *v);
  assert_eq!(result, 8);
}

#[test]
fn memo_does_not_propagate_unchanged_value() {
  let s = Signal::new(5);
  let sc = s.clone();
  let m = Memo::new(move || {
    let v = sc.get();
    if v > 10 {
      "big"
    } else {
      "small"
    }
  });
  assert_eq!(m.get(), "small");
  s.set(3);
  assert_eq!(m.get(), "small");
}

#[test]
fn memo_propagates_on_actual_change() {
  let s = Signal::new(5);
  let sc = s.clone();
  let m = Memo::new(move || {
    let v = sc.get();
    if v > 10 {
      "big"
    } else {
      "small"
    }
  });
  assert_eq!(m.get(), "small");
  s.set(20);
  assert_eq!(m.get(), "big");
}

#[test]
fn memo_clone_shares_value() {
  let s = Signal::new(1);
  let sc = s.clone();
  let m1 = Memo::new(move || sc.get() * 100);
  let m2 = m1.clone();
  assert_eq!(m2.get(), 100);
  s.set(2);
  assert_eq!(m1.get(), 200);
  assert_eq!(m2.get(), 200);
}

#[test]
fn memo_with_multiple_dependencies() {
  let a = Signal::new(2);
  let b = Signal::new(3);
  let ac = a.clone();
  let bc = b.clone();
  let m = Memo::new(move || ac.get() + bc.get());
  assert_eq!(m.get(), 5);
  a.set(10);
  assert_eq!(m.get(), 13);
  b.set(20);
  assert_eq!(m.get(), 30);
}

#[test]
fn memo_chains() {
  let s = Signal::new(1);
  let sc = s.clone();
  let m1 = Memo::new(move || sc.get() * 2);
  let m1c = m1.clone();
  let m2 = Memo::new(move || m1c.get() + 10);
  assert_eq!(m2.get(), 12);
  s.set(5);
  assert_eq!(m1.get(), 10);
  assert_eq!(m2.get(), 20);
}
