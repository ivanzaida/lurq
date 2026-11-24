//! A path field: the input and the button as the board draws them, what the button tells, what a drop sets, the
//! keys, a field around it, its sizes, a narrow room, what a theme changes, and, with `mcp`, the system's dialog
//! answered by the broker of the tree: what is chosen set on the signal, and a dialog cancelled.

use std::{
  path::PathBuf,
  sync::{Arc, Mutex},
};

use lurq::{
  app::{
    App,
    events::{DraggedPaths, DroppedPath},
    theme::{PaletteColor, ThemePalette},
  },
  components::{Column, Row},
  core::Signal,
  layout::quad::QuadContent,
  node::{Element, color::Color},
  ui::{Field, PathField, PathFieldPart, PathFieldSize, PathKind, icons},
};

use super::stage::Stage;

/// The client an agent answers a dialog with, as the engine's own tests of its MCP server have it.
#[cfg(feature = "mcp")]
#[path = "../mcp_client/mod.rs"]
mod mcp_client;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// The character lurq's font has for an icon.
fn glyph(name: &str) -> String {
  icons::ALL
    .iter()
    .find_map(|(known, glyph)| (*known == name).then(|| glyph.to_string()))
    .unwrap_or_else(|| panic!("lurq's font has no icon {name:?}"))
}

/// The width and the color of the edge of the element with `id`.
fn edge(stage: &mut Stage, id: &str) -> (f32, Color) {
  let quad = stage.border(id).unwrap_or_else(|| panic!("#{id} should have an edge"));
  let side = quad.border.and_then(|border| border.top).expect("an edge on top");
  (side.width, side.color)
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

/// A window of 500 by 300 with `field` 8 in from its corner, in a row with the id `root` that is as wide as it.
fn page(field: impl Into<Element>) -> Element {
  Column::new()
    .size(500.0, 300.0)
    .padding(8.0)
    .child(Row::new().id("root").child(field))
    .into()
}

/// What a handler was told, in order.
fn log() -> (Arc<Mutex<Vec<String>>>, impl Fn(&str) + Send + Sync + Clone + 'static) {
  let heard = Arc::new(Mutex::new(Vec::new()));
  let told = heard.clone();
  (heard, move |value: &str| told.lock().unwrap().push(value.to_owned()))
}

#[test]
fn a_path_field_is_the_board_s_input_with_a_folder_and_a_button_8_after_it() {
  let mut stage = Stage::new(|_| page(PathField::new(Signal::new("H:\\projects\\lurq\\pencil".to_owned())).id("path")));
  // 440 wide: the input takes the room the button leaves, 8 before it, both 32 high.
  let root = stage.bounds("root");
  assert_eq!(root.2, 440.0);
  let (field, button) = (stage.bounds("path"), stage.bounds("path-browse"));
  assert_eq!((field.3, button.3), (32.0, 32.0));
  assert_eq!(field.0, root.0);
  assert!((button.0 - (field.0 + field.2) - 8.0).abs() < 0.01, "8 apart");
  assert!(
    (root.0 + root.2 - (button.0 + button.2)).abs() < 0.01,
    "the button ends it"
  );

  // The box: the background's fill in an edge of 1 in the input's color, 6 round.
  assert_eq!(stage.fill_color("path"), Some(light(PaletteColor::Background)));
  assert_eq!(edge(&mut stage, "path"), (1.0, light(PaletteColor::Input)));
  assert!(
    stage
      .border("path")
      .unwrap()
      .border_radius
      .is_some_and(|radius| radius.top_left == 6.0)
  );
  // A folder of 16 in the muted foreground 10 in, and the path in the mono type at 12, 8 after it.
  let folder = stage.text_quad(&glyph(icons::FOLDER)).unwrap();
  assert_eq!(
    stage.text(&glyph(icons::FOLDER)),
    (light(PaletteColor::MutedForeground), 16.0)
  );
  assert!((folder.x - field.0 - 10.0).abs() < 0.01);
  let path = stage
    .quads()
    .into_iter()
    .find(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text.starts_with("H:")))
    .expect("the path is painted");
  assert!((path.x - field.0 - 34.0).abs() < 0.01, "10, the icon's 16 and 8");
  let QuadContent::Text { style, .. } = &path.content else {
    unreachable!()
  };
  assert_eq!((style.color, style.font_size), (light(PaletteColor::Foreground), 12.0));
  assert!(
    style.font_family.to_string().to_lowercase().contains("mono"),
    "the mono type"
  );

  // The button: outlined, 12 in from its sides, saying "Browse…".
  assert_eq!(edge(&mut stage, "path-browse"), (1.0, light(PaletteColor::Input)));
  let label = stage.text_quad("Browse…").unwrap();
  assert_eq!(stage.text("Browse…"), (light(PaletteColor::Foreground), 13.0));
  assert!((label.x - button.0 - 12.0).abs() < 0.01);
  assert!((button.0 + button.2 - (label.x + label.width) - 12.0).abs() < 0.01);
}

#[test]
fn the_button_tells_on_browse_from_the_pointer_and_the_keyboard_and_typing_sets_the_signal() {
  let presses = Arc::new(Mutex::new(0));
  let (count, second) = (presses.clone(), presses.clone());
  let value = Signal::new(String::new());
  let held = value.clone();
  let (submitted, submit) = log();
  let mut stage = Stage::new(move |_| {
    let (count, second) = (count.clone(), second.clone());
    page(
      PathField::new(held.clone())
        .id("path")
        .on_browse(move || *count.lock().unwrap() += 1)
        .on_browse(move || *second.lock().unwrap() += 10)
        .on_submit(submit.clone()),
    )
  });
  stage.click("path-browse");
  assert_eq!(
    *presses.lock().unwrap(),
    11,
    "both handlers, the second after the first"
  );

  // Tab stops in the input, then on the button: what is typed is the signal's, Enter in the input submits it.
  stage.tree.mouse_move(490.0, 290.0);
  stage.click("path");
  assert_eq!(focused(&stage).as_deref(), Some("path"));
  stage.tree.text_input_replace(0..0, "/usr/local");
  stage.pass();
  assert_eq!(value.get(), "/usr/local");
  stage.key("Enter");
  assert_eq!(*submitted.lock().unwrap(), ["/usr/local"]);
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("path-browse"));
  stage.key("Enter");
  assert_eq!(*presses.lock().unwrap(), 22);
  stage.key("Space");
  assert_eq!(*presses.lock().unwrap(), 33);
}

#[test]
fn a_disabled_and_a_read_only_field_browse_nothing() {
  for (disabled, read_only) in [(true, false), (false, true)] {
    let presses = Arc::new(Mutex::new(0));
    let count = presses.clone();
    let value = Signal::new("/usr".to_owned());
    let held = value.clone();
    let mut stage = Stage::new(move |_| {
      let count = count.clone();
      page(
        PathField::new(held.clone())
          .id("path")
          .disabled(disabled)
          .read_only(read_only)
          .on_browse(move || *count.lock().unwrap() += 1),
      )
    });
    stage.click("path-browse");
    assert_eq!(
      *presses.lock().unwrap(),
      0,
      "disabled {disabled}, read only {read_only}"
    );
    // Nothing dropped on it is taken either.
    let (x, y) = stage.center("path");
    stage.tree.file_drop(
      DraggedPaths::new([DroppedPath::with_kind(PathBuf::from("/tmp"), true)]),
      x,
      y,
    );
    stage.pass();
    assert_eq!(value.get(), "/usr");
  }
}

#[test]
fn a_folder_dropped_on_the_field_is_its_path_and_a_field_of_folders_takes_no_file() {
  let (picked, tell) = log();
  let value = Signal::new(String::new());
  let held = value.clone();
  let mut stage = Stage::new(move |_| {
    page(
      PathField::new(held.clone())
        .id("path")
        .picks(PathKind::Folder)
        .on_pick(tell.clone()),
    )
  });
  let (x, y) = stage.center("path");
  let file = || DroppedPath::with_kind(PathBuf::from("notes.md"), false);
  stage.tree.file_drop(DraggedPaths::new([file()]), x, y);
  stage.pass();
  assert_eq!(value.get(), "", "a file is not a folder");
  assert!(picked.lock().unwrap().is_empty());
  stage.tree.file_drop(
    DraggedPaths::new([file(), DroppedPath::with_kind(PathBuf::from("projects"), true)]),
    x,
    y,
  );
  stage.pass();
  assert_eq!(value.get(), "projects");
  assert_eq!(*picked.lock().unwrap(), ["projects"], "told once the signal is set");

  // A field of files takes the file, and one that says nothing of its path takes the first thing dropped.
  for (kind, taken) in [
    (Some(PathKind::File), "notes.md"),
    (Some(PathKind::SaveFile), "notes.md"),
    (None, "projects"),
  ] {
    let value = Signal::new(String::new());
    let held = value.clone();
    let mut stage = Stage::new(move |_| {
      let field = PathField::new(held.clone()).id("path");
      page(match kind {
        Some(kind) => field.picks(kind),
        None => field,
      })
    });
    let (x, y) = stage.center("path");
    stage.tree.file_drop(
      DraggedPaths::new([DroppedPath::with_kind(PathBuf::from("projects"), true), file()]),
      x,
      y,
    );
    stage.pass();
    assert_eq!(value.get(), taken, "{kind:?}");
  }
}

#[test]
fn a_mounted_field_that_picks_tells_on_browse_and_changes_nothing_where_nothing_is_chosen() {
  // The tests have no system dialog: it gives nothing, as a dialog a person cancelled does.
  let presses = Arc::new(Mutex::new(0));
  let count = presses.clone();
  let (picked, tell) = log();
  let value = Signal::new("H:\\projects".to_owned());
  let held = value.clone();
  let mut stage = Stage::new(move |ctx| {
    let count = count.clone();
    page(
      PathField::new(held.clone())
        .id("path")
        .picks(PathKind::Folder)
        .on_browse(move || *count.lock().unwrap() += 1)
        .on_pick(tell.clone())
        .mount(ctx),
    )
  });
  stage.click("path-browse");
  stage.pass();
  assert_eq!(*presses.lock().unwrap(), 1);
  assert_eq!(value.get(), "H:\\projects");
  assert!(picked.lock().unwrap().is_empty());
  // The app sets the signal: the field shows it, and no handler hears of it.
  value.set("H:\\other".to_owned());
  stage.pass();
  assert!(
    stage
      .quads()
      .iter()
      .any(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text == "H:\\other"))
  );
  assert!(picked.lock().unwrap().is_empty());
}

#[test]
fn its_icon_follows_what_it_picks_and_can_be_another_or_none() {
  let field = || PathField::new(Signal::new(String::new()));
  let stage = Stage::new(move |_| page(field().picks(PathKind::File)));
  assert!(stage.paints(&glyph(icons::FILE)) && !stage.paints(&glyph(icons::FOLDER)));
  let stage = Stage::new(move |_| page(field().picks(PathKind::SaveFile)));
  assert!(stage.paints(&glyph(icons::FILE)));
  let stage = Stage::new(move |_| page(field().icon(icons::DRIVE)));
  assert!(stage.paints(&glyph(icons::DRIVE)) && !stage.paints(&glyph(icons::FOLDER)));
  // With none, the path starts at the box's own 10.
  let mut stage = Stage::new(|_| page(PathField::new(Signal::new("/usr".to_owned())).icon(None).id("path")));
  assert!(!stage.paints(&glyph(icons::FOLDER)));
  let (x, ..) = stage.bounds("path");
  assert!((stage.text_quad("/usr").unwrap().x - x - 10.0).abs() < 0.01);
}

#[test]
fn its_sizes_are_the_input_s_and_the_button_s_and_full_width_fills_the_room() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .size(500.0, 300.0)
      .padding(8.0)
      .spacing(8.0)
      .child(
        PathField::new(Signal::new("/usr".to_owned()))
          .size(PathFieldSize::Md)
          .id("md"),
      )
      .child(
        PathField::new(Signal::new("/var".to_owned()))
          .size(PathFieldSize::Lg)
          .id("lg"),
      )
      .child(
        PathField::new(Signal::new("/etc".to_owned()))
          .full_width(true)
          .id("wide"),
      )
      .into()
  });
  assert_eq!((stage.bounds("md").3, stage.bounds("md-browse").3), (36.0, 36.0));
  assert_eq!(stage.text("/usr").1, 13.0);
  assert_eq!((stage.bounds("lg").3, stage.bounds("lg-browse").3), (40.0, 40.0));
  assert_eq!(stage.text("/var").1, 14.0);
  let (field, button) = (stage.bounds("wide"), stage.bounds("wide-browse"));
  assert_eq!(field.0, 8.0);
  assert!((button.0 + button.2 - 492.0).abs() < 0.01, "as wide as its room");
}

#[test]
fn a_judgment_is_the_edge_of_the_box_and_a_field_names_and_judges_it() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .size(500.0, 300.0)
      .padding(8.0)
      .spacing(8.0)
      .child(
        PathField::new(Signal::new("/nowhere".to_owned()))
          .invalid(true)
          .id("wrong"),
      )
      .child(
        Field::new("Project folder", PathField::new(Signal::new(String::new())).id("named"))
          .error("Choose a folder that exists"),
      )
      .into()
  });
  assert_eq!(edge(&mut stage, "wrong"), (1.0, light(PaletteColor::Destructive)));
  assert_eq!(
    edge(&mut stage, "named"),
    (1.0, light(PaletteColor::Destructive)),
    "the field's error is the path's"
  );
  #[cfg(any(feature = "devtools", feature = "mcp"))]
  {
    let said = |stage: &mut Stage, id: &str, key: &str| {
      stage
        .tree
        .get_element_by_id_mut(id)
        .and_then(|element| element.description(key))
    };
    assert_eq!(
      said(&mut stage, "named", "a11y_name").as_deref(),
      Some("Project folder")
    );
    assert_eq!(said(&mut stage, "named", "a11y_role").as_deref(), Some("textbox"));
    assert_eq!(said(&mut stage, "named", "a11y_invalid").as_deref(), Some("true"));
  }
}

#[cfg(any(feature = "devtools", feature = "mcp"))]
#[test]
fn the_input_is_called_by_its_label_and_the_button_by_the_words_it_draws() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .size(500.0, 300.0)
      .padding(8.0)
      .spacing(8.0)
      .child(PathField::new(Signal::new(String::new())).label("Folder").id("plain"))
      .child(
        PathField::new(Signal::new(String::new()))
          .browse_label("Durchsuchen…")
          .id("said"),
      )
      .into()
  });
  let said = |stage: &mut Stage, id: &str, key: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(key))
  };
  assert_eq!(said(&mut stage, "plain", "a11y_name").as_deref(), Some("Folder"));
  assert_eq!(said(&mut stage, "plain", "a11y_role").as_deref(), Some("textbox"));
  // The button is a button of the design system, which is called by the words it draws.
  assert!(stage.paints("Browse…") && stage.paints("Durchsuchen…"));
  assert_eq!(
    said(&mut stage, "said-browse", "a11y_name"),
    None,
    "its text is its name"
  );
}

#[test]
fn a_path_longer_than_the_box_stays_in_it_and_the_button_keeps_its_room() {
  let long = "H:\\projects\\lurq\\.claude\\worktrees\\b12-desktop\\crates\\lurq\\src\\ui\\path_field.rs";
  let mut stage = Stage::new(move |_| {
    Column::new()
      .size(400.0, 300.0)
      .child(
        Row::new()
          .id("room")
          .width(240.0)
          .child(PathField::new(Signal::new(long.to_owned())).id("path")),
      )
      .into()
  });
  let (field, button) = (stage.bounds("path"), stage.bounds("path-browse"));
  assert!(
    (button.0 + button.2 - 240.0).abs() < 0.5,
    "the button ends at the room's end"
  );
  assert!(button.2 > 60.0, "the button keeps its width: {}", button.2);
  assert!((button.0 - (field.0 + field.2) - 8.0).abs() < 0.01);
  assert!(field.2 > 100.0 && field.0 == 0.0, "the input has the rest");
  assert!(stage.paints("Browse…"), "the button's word is whole");
}

#[test]
fn a_theme_restyles_its_parts() {
  let app = App::new();
  app.theme().patch_recipe::<PathField>(|rules| {
    rules
      .base(PathFieldPart::Root, |s| s.gap(16.0).width(300.0))
      .base(PathFieldPart::Control, |s| s.font_size(15.0))
  });
  let mut stage = Stage::in_app(app, |_| page(PathField::new(Signal::new("/usr".to_owned())).id("path")));
  assert_eq!(stage.bounds("root").2, 300.0);
  let (field, button) = (stage.bounds("path"), stage.bounds("path-browse"));
  assert!((button.0 - (field.0 + field.2) - 16.0).abs() < 0.01);
  assert_eq!(stage.text("/usr").1, 15.0);
}

/// What a press of Browse came to, with the dialog answered by the broker of the stage's tree in place of the
/// system: as an agent answers it, over the tree's MCP server.
#[cfg(feature = "mcp")]
struct Browsed {
  /// What the field's signal holds afterwards.
  value: String,
  /// How often `on_browse` was called.
  browsed: usize,
  /// What `on_pick` was told, in order.
  picked: Vec<String>,
  /// The dialog as the broker was asked for it.
  request: serde_json::Value,
}

/// Mounts a path field that holds `start` and picks `kind` with `dialog`, presses its button, and answers the
/// dialog it asks for: with `chosen`, or as a person who cancelled it.
#[cfg(feature = "mcp")]
fn browse(kind: PathKind, dialog: lurq::files::FileDialog, start: &str, chosen: Option<PathBuf>) -> Browsed {
  use std::time::{Duration, Instant};

  use lurq::mcp::{FileDialogBroker, McpConfig, Scope};
  use serde_json::json;

  let value = Signal::new(start.to_owned());
  let (picked, tell) = log();
  let browsed = Arc::new(Mutex::new(0_usize));
  let (held, count) = (value.clone(), browsed.clone());
  let mut stage = Stage::new(move |ctx| {
    let count = count.clone();
    page(
      PathField::new(held.clone())
        .id("path")
        .picks(kind)
        .dialog(dialog.clone())
        .on_browse(move || *count.lock().unwrap() += 1)
        .on_pick(tell.clone())
        .mount(ctx),
    )
  });
  let mcp = stage.tree.enable_mcp(
    McpConfig::new()
      .app_name("lurq-path-field-test")
      .scope(Scope::custom("file_dialogs"))
      .file_dialogs(FileDialogBroker::new()),
  );
  let (port, token) = (mcp.port(), mcp.token().to_owned());
  stage.click("path-browse");

  let answer = chosen.map(|path| path.to_string_lossy().into_owned());
  let agent = std::thread::spawn(move || {
    let mut client = mcp_client::Client::new(port, token);
    client.initialize();
    // The dialog is asked for once the press is handled; the agent waits for it.
    let deadline = Instant::now() + Duration::from_secs(30);
    let request = loop {
      let listed = client.tool_json("lurq_file_dialogs", json!({}));
      if let Some(request) = listed["requests"].as_array().and_then(|requests| requests.first()) {
        break request.clone();
      }
      assert!(Instant::now() < deadline, "no dialog was asked for: {listed}");
      std::thread::sleep(Duration::from_millis(20));
    };
    let mut reply = json!({
      "request_id": request["request_id"], "window": request["window"], "operation": request["operation"],
    });
    match answer {
      Some(path) => {
        reply["action"] = json!("select");
        reply["paths"] = json!([path]);
        if request["operation"] == "save_file" {
          reply["overwrite"] = json!(false);
        }
      }
      None => reply["action"] = json!("cancel"),
    }
    let answered = client.tool_json("lurq_file_dialog_respond", reply);
    assert!(answered.get("error").is_none(), "{answered}");
    request
  });
  let deadline = Instant::now() + Duration::from_secs(60);
  while !agent.is_finished() {
    assert!(Instant::now() < deadline, "the agent did not finish");
    // As the shell's loop does: the requests, the futures woken, a pass.
    stage.tree.drain_mcp_requests(&mut stage.app);
    stage.tree.tick_futures();
    stage.pass();
    std::thread::sleep(Duration::from_millis(2));
  }
  let request = agent.join().expect("the agent's thread");
  // The answer reaches the field's future on the passes after the agent is done.
  for _ in 0..20 {
    stage.tree.tick_futures();
    stage.pass();
    std::thread::sleep(Duration::from_millis(2));
  }
  stage.tree.shutdown_mcp();
  Browsed {
    value: value.get(),
    browsed: *browsed.lock().unwrap(),
    picked: picked.lock().unwrap().clone(),
    request,
  }
}

#[cfg(feature = "mcp")]
#[test]
fn the_folder_chosen_in_the_dialog_is_set_on_the_signal_and_told_once() {
  let folder = std::env::temp_dir().join("lurq-path-field-project");
  let came = browse(
    PathKind::Folder,
    lurq::files::FileDialog::new().title("Project folder"),
    "H:\\projects",
    Some(folder.clone()),
  );
  let chosen = folder.to_string_lossy().into_owned();
  assert_eq!(
    came.request["operation"], "open_folder",
    "a field of folders asks for a folder"
  );
  assert_eq!(
    came.request["title"], "Project folder",
    "with the dialog the app gave it"
  );
  assert_eq!(came.value, chosen, "what was chosen is the signal's");
  assert_eq!(came.picked, [chosen], "and on_pick is told it, once");
  assert_eq!(came.browsed, 1, "on_browse was told of the press");
}

#[cfg(feature = "mcp")]
#[test]
fn a_dialog_the_person_cancelled_changes_nothing_and_tells_nobody() {
  let came = browse(PathKind::Folder, lurq::files::FileDialog::new(), "H:\\projects", None);
  assert_eq!(came.request["operation"], "open_folder");
  assert_eq!(came.value, "H:\\projects", "the path is as it was");
  assert!(came.picked.is_empty(), "on_pick heard nothing");
  assert_eq!(came.browsed, 1, "the press was still told to on_browse");
}

#[cfg(feature = "mcp")]
#[test]
fn a_field_of_a_file_to_save_asks_for_one_with_the_name_it_starts_with() {
  let report = std::env::temp_dir().join("lurq-path-field-report.md");
  let came = browse(
    PathKind::SaveFile,
    lurq::files::FileDialog::new()
      .title("Save the report")
      .filter("Markdown", &["md"])
      .file_name("report.md"),
    "",
    Some(report.clone()),
  );
  let chosen = report.to_string_lossy().into_owned();
  assert_eq!(came.request["operation"], "save_file");
  assert_eq!(came.request["suggested_name"], "report.md");
  assert_eq!(came.request["filters"], serde_json::json!([["Markdown", ["md"]]]));
  assert_eq!(came.value, chosen);
  assert_eq!(came.picked, [chosen]);
  assert_eq!(came.browsed, 1);
}

#[cfg(feature = "mcp")]
#[test]
fn a_field_of_a_file_to_open_asks_for_one_file() {
  let notes = std::env::temp_dir().join("lurq-path-field-notes.txt");
  let came = browse(
    PathKind::File,
    lurq::files::FileDialog::new().multiple(true),
    "",
    Some(notes.clone()),
  );
  assert_eq!(
    came.request["operation"], "open_file",
    "one file, whatever the dialog was told"
  );
  assert_eq!(came.value, notes.to_string_lossy());
  assert_eq!(came.picked.len(), 1);
}

#[test]
fn the_button_is_of_the_variant_the_field_is_told_at_rest_and_under_the_pointer() {
  use lurq::ui::{Button, ButtonVariant};
  let mut stage = Stage::new(|_| {
    Column::new()
      .size(500.0, 300.0)
      .padding(8.0)
      .child(
        PathField::new(Signal::new(String::new()))
          .button_variant(ButtonVariant::Secondary)
          .id("path"),
      )
      .child(Button::new("Browse…").variant(ButtonVariant::Secondary).id("plain"))
      .into()
  });
  let rest = stage.fill_color("plain");
  assert_eq!(rest, Some(light(PaletteColor::Secondary)));
  assert_eq!(stage.fill_color("path-browse"), rest, "at rest");
  stage.hover("plain");
  let hovered = stage.fill_color("plain");
  assert_ne!(hovered, rest);
  stage.hover("path-browse");
  assert_eq!(stage.fill_color("path-browse"), hovered, "under the pointer");
}
