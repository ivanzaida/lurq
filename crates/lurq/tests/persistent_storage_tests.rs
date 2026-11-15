#![cfg(feature = "persistent_storage")]

use std::path::PathBuf;

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx},
  node::Element,
  persistent_storage::PersistentWrite,
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
fn app_bulk_roundtrips_values_in_key_order() {
  let app = App::new();

  app
    .write_bulk([("first", 1_i32), ("second", 2_i32), ("third", 3_i32)])
    .unwrap();

  let values = app
    .read_bulk_values::<i32, _, _>(["third", "missing", "first"])
    .unwrap();

  assert_eq!(values, vec![Some(3), None, Some(1)]);
}

#[test]
fn app_bulk_writes_mixed_value_types() {
  let app = App::new();

  app
    .write_bulk([
      PersistentWrite::new("name", "Ada"),
      PersistentWrite::new("launch_count", 12_u64),
      PersistentWrite::new("compact", true),
    ])
    .unwrap();

  assert_eq!(app.persistent_value::<String>("name"), Some("Ada".to_owned()));
  assert_eq!(app.persistent_value::<u64>("launch_count"), Some(12));
  assert_eq!(app.persistent_value::<bool>("compact"), Some(true));

  let batch = app.read_bulk(["name", "launch_count", "compact"]).unwrap();

  assert_eq!(batch.value::<String>("name"), Some("Ada".to_owned()));
  assert_eq!(batch.value::<u64>("launch_count"), Some(12));
  assert_eq!(batch.value::<bool>("compact"), Some(true));
}

#[derive(Debug, PartialEq, lurq::PersistentValue)]
struct UserPrefs {
  name: String,
  launch_count: u64,
  compact: bool,
}

#[test]
fn app_roundtrips_derived_persistent_value_struct() {
  let app = App::new();

  app
    .set_persistent_value(
      "prefs",
      UserPrefs {
        name: "Ada".to_owned(),
        launch_count: 12,
        compact: true,
      },
    )
    .unwrap();

  assert_eq!(
    app.persistent_value::<UserPrefs>("prefs"),
    Some(UserPrefs {
      name: "Ada".to_owned(),
      launch_count: 12,
      compact: true,
    })
  );
}

#[test]
fn app_snapshot_shows_derived_persistent_value_struct_fields() {
  let app = App::new();

  app
    .set_persistent_value(
      "prefs",
      UserPrefs {
        name: "Ada".to_owned(),
        launch_count: 12,
        compact: true,
      },
    )
    .unwrap();

  let snapshot = app.persistent_storage().snapshot().unwrap();
  let prefs = snapshot
    .iter()
    .find(|entry| entry.key == "prefs")
    .expect("prefs snapshot entry");

  assert_eq!(prefs.type_name, "UserPrefs");
  assert!(prefs.full_type_name.ends_with("UserPrefs"));
  assert_eq!(
    prefs.value,
    "UserPrefs { name: \"Ada\", launch_count: 12, compact: true }"
  );
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

#[test]
fn redb_bulk_storage_persists_across_app_instances() {
  let path = temp_storage_path("bulk_persists");

  {
    let mut app = App::new();
    app.set_persistent_storage_path(path.clone()).unwrap();
    app
      .write_bulk([("one", 1_u64), ("two", 2_u64), ("three", 3_u64)])
      .unwrap();
  }

  {
    let mut app = App::new();
    app.set_persistent_storage_path(path.clone()).unwrap();

    let values = app.read_bulk_values::<u64, _, _>(["three", "one", "missing"]).unwrap();

    assert_eq!(values, vec![Some(3), Some(1), None]);
  }

  let _ = std::fs::remove_file(path);
}

struct PersistentRoot;

impl Component for PersistentRoot {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    ctx
      .write_bulk([
        PersistentWrite::new("visits", 3_u64),
        PersistentWrite::new("opens", 5_u64),
        PersistentWrite::new("label", "ready"),
      ])
      .unwrap();
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let values = ctx.read_bulk(["visits", "opens", "label"]).unwrap();
    let visits = values.value::<u64>("visits").unwrap_or(0);
    let opens = values.value::<u64>("opens").unwrap_or(0);
    let label = values.value::<String>("label").unwrap_or_default();
    lurq::components::Text::new(&format!("{visits}/{opens}/{label}"))
  }
}

#[test]
fn ctx_persistent_value_reads_and_writes_app_storage() {
  let mut app = App::new();
  let mut tree = Tree::new();

  tree.mount_root::<PersistentRoot>(&mut app, ());

  assert_eq!(tree.root().unwrap().text_content(), Some("3/5/ready"));
  assert_eq!(app.persistent_value::<u64>("visits"), Some(3));
  assert_eq!(app.persistent_value::<u64>("opens"), Some(5));
  assert_eq!(app.persistent_value::<String>("label"), Some("ready".to_owned()));
}

#[test]
fn dock_state_roundtrips_through_storage() {
  use lurq::{
    persistent_storage::PersistentStorage,
    ui::{DockNode, DockPlace, DockSide, DockState},
  };

  let mut state = DockState::new(
    DockNode::row([
      DockNode::group(["explorer"]),
      DockNode::column([
        DockNode::documents(DockNode::group(["blur.rs", "theme.rs"])),
        DockNode::group(["terminal", "problems", "output"]),
      ])
      .shares([0.7, 0.3]),
    ])
    .shares([0.25, 0.75]),
  );
  assert!(state.show("problems", DockPlace::Hidden(DockSide::Bottom)));
  assert!(state.show(
    "outline",
    DockPlace::Float {
      position: (10.0, 20.0),
      size: Some((300.0, 200.0)),
    }
  ));
  assert!(state.show("say \"hi\"", DockPlace::Tab { of: "explorer".into() }));
  assert!(state.select("theme.rs"));
  assert!(state.maximize("terminal"));

  let storage = PersistentStorage::memory();
  storage.set_value("dock", state.clone()).unwrap();

  assert_eq!(storage.value::<DockState>("dock"), Some(state));
  assert_eq!(storage.value::<DockState>("missing"), None);
}

#[test]
fn a_data_grids_arrangement_sort_and_groups_open_roundtrip_through_storage() {
  use lurq::{
    persistent_storage::PersistentStorage,
    ui::{ColumnLayout, ColumnPin, ColumnPlace, GroupKey, TableSort},
  };

  let place = |id: &str, hidden: bool, width: Option<f32>, fit: bool, pin: ColumnPin| ColumnPlace {
    id: id.into(),
    hidden,
    width,
    fit,
    pin,
  };
  let layout = ColumnLayout {
    columns: vec![
      place("size", false, Some(150.5), false, ColumnPin::None),
      place("na\"me", false, None, true, ColumnPin::Start),
      place("status", true, None, false, ColumnPin::End),
    ],
  };
  let sort = Some(TableSort::descending("size"));
  let open = vec![
    GroupKey::new(["Passing"]),
    GroupKey::new(["Passing", r"Ada \ Lovelace"]),
  ];

  let storage = PersistentStorage::memory();
  storage.set_value("columns", layout.clone()).unwrap();
  storage.set_value("sort", sort.clone()).unwrap();
  storage.set_value("unsorted", None::<TableSort>).unwrap();
  let several = vec![TableSort::ascending("status"), TableSort::descending("size")];
  storage.set_value("several", several.clone()).unwrap();
  storage.set_value("none", Vec::<TableSort>::new()).unwrap();
  storage.set_value("open", open.clone()).unwrap();

  assert_eq!(storage.value::<ColumnLayout>("columns"), Some(layout.clone()));
  assert_eq!(storage.value::<Option<TableSort>>("sort"), Some(sort));
  assert_eq!(storage.value::<Option<TableSort>>("unsorted"), Some(None));
  assert_eq!(storage.value::<Vec<TableSort>>("several"), Some(several.clone()));
  assert_eq!(storage.value::<Vec<TableSort>>("none"), Some(Vec::new()));
  assert_eq!(TableSort::parse_list(&TableSort::list_to_text(&several)), Some(several));
  assert_eq!(storage.value::<Vec<GroupKey>>("open"), Some(open));
  assert_eq!(layout.to_string().parse::<ColumnLayout>(), Ok(layout.clone()));

  // Read back for the columns the app has now: one it dropped is left out, a new one takes its place.
  let valid = layout.valid(["na\"me", "kind", "size"]);
  let ids: Vec<&str> = valid.columns.iter().map(|place| &*place.id).collect();
  assert_eq!(
    ids,
    ["size", "na\"me", "kind"],
    "kind after the column before it among the app's"
  );
  assert_eq!(valid.place("size").and_then(|place| place.width), Some(150.5));
}
