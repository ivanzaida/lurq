#![cfg(feature = "persistent_storage")]

use std::path::PathBuf;

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx},
  node::Element,
};

fn temp_storage_path(name: &str) -> PathBuf {
  let unique = format!(
    "lurq_persistent_storage_{}_{}_{}.redb",
    name,
    std::process::id(),
    std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .unwrap()
      .as_nanos()
  );
  std::env::temp_dir().join(unique)
}

#[test]
fn app_persistent_value_roundtrips_primitives() {
  let app = App::new();

  app.set_persistent_value("enabled", true).unwrap();
  app.set_persistent_value("count", 42_i32).unwrap();
  app.set_persistent_value("ratio", 1.5_f64).unwrap();
  app.set_persistent_value("name", "Ada").unwrap();

  assert_eq!(app.persistent_value::<bool>("enabled"), Some(true));
  assert_eq!(app.persistent_value::<i32>("count"), Some(42));
  assert_eq!(app.persistent_value::<f64>("ratio"), Some(1.5));
  assert_eq!(app.persistent_value::<String>("name"), Some("Ada".to_owned()));
}

#[test]
fn persistent_value_type_mismatch_returns_none() {
  let app = App::new();

  app.set_persistent_value("count", 42_i32).unwrap();

  assert_eq!(app.persistent_value::<String>("count"), None);
}

#[test]
fn redb_storage_persists_across_app_instances() {
  let path = temp_storage_path("persists");

  {
    let mut app = App::new();
    app.set_persistent_storage_path(path.clone()).unwrap();
    app.set_persistent_value("count", 7_u32).unwrap();
    app.set_persistent_value("name", String::from("Lurq")).unwrap();
  }

  {
    let mut app = App::new();
    app.set_persistent_storage_path(path.clone()).unwrap();

    assert_eq!(app.persistent_value::<u32>("count"), Some(7));
    assert_eq!(app.persistent_value::<String>("name"), Some("Lurq".to_owned()));
  }

  let _ = std::fs::remove_file(path);
}

struct PersistentRoot;

impl Component for PersistentRoot {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    ctx.set_persistent_value("visits", 3_u64).unwrap();
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let visits = ctx.persistent_value::<u64>("visits").unwrap_or(0);
    lurq::components::Text::new(&format!("{visits}"))
  }
}

#[test]
fn ctx_persistent_value_reads_and_writes_app_storage() {
  let mut app = App::new();
  let mut tree = Tree::new();

  tree.mount_root::<PersistentRoot>(&mut app, ());

  assert_eq!(tree.root().unwrap().text_content(), Some("3"));
  assert_eq!(app.persistent_value::<u64>("visits"), Some(3));
}
