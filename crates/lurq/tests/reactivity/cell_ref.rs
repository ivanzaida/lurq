use lurq::core::Ref;

#[test]
fn ref_get_returns_initial() {
  let r = Ref::new(42);
  assert_eq!(r.get(), 42);
}

#[test]
fn ref_set_updates() {
  let r = Ref::new(0);
  r.set(99);
  assert_eq!(r.get(), 99);
}

#[test]
fn ref_update_mutates() {
  let r = Ref::new(10);
  r.update(|v| *v += 5);
  assert_eq!(r.get(), 15);
}

#[test]
fn ref_with_reads() {
  let r = Ref::new(vec![1, 2, 3]);
  let len = r.with(|v| v.len());
  assert_eq!(len, 3);
}

#[test]
fn ref_clone_shares_state() {
  let r1 = Ref::new(0);
  let r2 = r1.clone();
  r1.set(42);
  assert_eq!(r2.get(), 42);
}

#[test]
fn ref_is_not_reactive() {
  let r = Ref::new(0);
  r.set(1);
  r.set(2);
  assert_eq!(r.get(), 2);
}

#[test]
fn ref_with_string() {
  let r = Ref::new(String::new());
  r.update(|s| s.push_str("hello"));
  r.update(|s| s.push_str(" world"));
  assert_eq!(r.get(), "hello world");
}
