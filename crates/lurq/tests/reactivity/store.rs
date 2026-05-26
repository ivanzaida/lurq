use lurq::core::Store;

#[derive(Clone, Debug, PartialEq)]
struct AppState {
  count: i32,
  name: String,
  items: Vec<i32>,
}

fn default_state() -> AppState {
  AppState {
    count: 0,
    name: "test".to_owned(),
    items: vec![1, 2, 3],
  }
}

#[test]
fn store_get_returns_initial() {
  let store = Store::new(default_state());
  assert_eq!(store.get().count, 0);
  assert_eq!(store.get().name, "test");
}

#[test]
fn store_set_replaces() {
  let store = Store::new(default_state());
  store.set(AppState {
    count: 42,
    name: "new".to_owned(),
    items: vec![],
  });
  assert_eq!(store.get().count, 42);
  assert_eq!(store.get().name, "new");
}

#[test]
fn store_update_mutates() {
  let store = Store::new(default_state());
  store.update(|s| s.count += 10);
  assert_eq!(store.get().count, 10);
}

#[test]
fn store_with_reads() {
  let store = Store::new(default_state());
  let len = store.with(|s| s.items.len());
  assert_eq!(len, 3);
}

#[test]
fn store_clone_shares() {
  let s1 = Store::new(default_state());
  let s2 = s1.clone();
  s1.update(|s| s.count = 99);
  assert_eq!(s2.get().count, 99);
}

// --- Lens tests ---

#[test]
fn lens_get_reads_field() {
  let store = Store::new(default_state());
  let count = store.lens(|s| s.count, |s, v| s.count = v);
  assert_eq!(count.get(), 0);
}

#[test]
fn lens_set_updates_field() {
  let store = Store::new(default_state());
  let count = store.lens(|s| s.count, |s, v| s.count = v);
  count.set(42);
  assert_eq!(store.get().count, 42);
}

#[test]
fn lens_update_mutates_field() {
  let store = Store::new(default_state());
  let count = store.lens(|s| s.count, |s, v| s.count = v);
  count.update(|v| *v += 10);
  assert_eq!(store.get().count, 10);
}

#[test]
fn lens_does_not_affect_other_fields() {
  let store = Store::new(default_state());
  let count = store.lens(|s| s.count, |s, v| s.count = v);
  count.set(99);
  assert_eq!(store.get().name, "test");
  assert_eq!(store.get().items, vec![1, 2, 3]);
}

#[test]
fn lens_on_string_field() {
  let store = Store::new(default_state());
  let name = store.lens(|s| s.name.clone(), |s, v| s.name = v);
  assert_eq!(name.get(), "test");
  name.set("updated".to_owned());
  assert_eq!(store.get().name, "updated");
}

#[test]
fn lens_on_vec_field() {
  let store = Store::new(default_state());
  let items = store.lens(|s| s.items.clone(), |s, v| s.items = v);
  items.update(|v| v.push(4));
  assert_eq!(store.get().items, vec![1, 2, 3, 4]);
}

#[test]
fn multiple_lenses_on_same_store() {
  let store = Store::new(default_state());
  let count = store.lens(|s| s.count, |s, v| s.count = v);
  let name = store.lens(|s| s.name.clone(), |s, v| s.name = v);
  count.set(50);
  name.set("hello".to_owned());
  let state = store.get();
  assert_eq!(state.count, 50);
  assert_eq!(state.name, "hello");
}

#[test]
fn lens_clone_shares_store() {
  let store = Store::new(default_state());
  let l1 = store.lens(|s| s.count, |s, v| s.count = v);
  let l2 = l1.clone();
  l1.set(77);
  assert_eq!(l2.get(), 77);
}
