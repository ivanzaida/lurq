use lurq::core::Signal;

#[test]
fn get_returns_initial_value() {
  let s = Signal::new(42);
  assert_eq!(s.get(), 42);
}

#[test]
fn set_updates_value() {
  let s = Signal::new(0);
  s.set(99);
  assert_eq!(s.get(), 99);
}

#[test]
fn update_mutates_in_place() {
  let s = Signal::new(10);
  s.update(|v| *v += 5);
  assert_eq!(s.get(), 15);
}

#[test]
fn with_reads_without_cloning() {
  let s = Signal::new(vec![1, 2, 3]);
  let len = s.with(|v| v.len());
  assert_eq!(len, 3);
}

#[test]
fn get_untracked_returns_value() {
  let s = Signal::new(7);
  assert_eq!(s.get_untracked(), 7);
}

#[test]
fn clone_shares_state() {
  let s1 = Signal::new(0);
  let s2 = s1.clone();
  s1.set(42);
  assert_eq!(s2.get(), 42);
}

#[test]
fn from_creates_signal() {
  let s: Signal<i32> = 123.into();
  assert_eq!(s.get(), 123);
}

#[test]
fn multiple_sets_keep_last() {
  let s = Signal::new(0);
  s.set(1);
  s.set(2);
  s.set(3);
  assert_eq!(s.get(), 3);
}

#[test]
fn signal_with_string() {
  let s = Signal::new("hello".to_owned());
  s.update(|v| v.push_str(" world"));
  assert_eq!(s.get(), "hello world");
}

#[test]
fn signal_with_vec() {
  let s = Signal::new(vec![1]);
  s.update(|v| v.push(2));
  s.update(|v| v.push(3));
  assert_eq!(s.get(), vec![1, 2, 3]);
}

#[test]
fn with_untracked_reads() {
  let s = Signal::new(42);
  let v = s.with_untracked(|v| *v);
  assert_eq!(v, 42);
}

#[test]
fn update_complex_struct() {
  #[derive(Clone, Debug, PartialEq)]
  struct State { x: i32, y: String }
  let s = Signal::new(State { x: 0, y: "a".into() });
  s.update(|s| { s.x = 10; s.y = "b".into(); });
  let val = s.get();
  assert_eq!(val.x, 10);
  assert_eq!(val.y, "b");
}
