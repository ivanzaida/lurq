//! A drop zone: the board's zone at rest, what a drag of files over it shows,
//! what a drop gives the app and what it refuses, its keys and its states.

use std::{
  path::PathBuf,
  sync::{Arc, Mutex},
};

use lurq::{
  app::{
    App,
    events::{DraggedPaths, DroppedPath},
    theme::{OpacityLevel, PaletteColor, ThemePalette},
  },
  components::{Column, Row},
  files::DroppedFile,
  node::color::Color,
  ui::{
    Case, DropZone, DropZonePart, DropZoneRenderer, DropZoneState, DropZoneView, Interaction, Opacity, RejectReason,
    RejectedFile,
  },
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// The color and the width of the line around the element with `id`.
fn edge(stage: &mut Stage, id: &str) -> Option<(Color, f32)> {
  stage
    .border(id)
    .and_then(|quad| quad.border)
    .and_then(|border| border.top)
    .map(|side| (side.color, side.width))
}

/// A drag of files that are not on the disk: their names are what the zone judges.
fn dragged(names: &[&str]) -> DraggedPaths {
  DraggedPaths::new(
    names
      .iter()
      .map(|name| DroppedPath::with_kind(PathBuf::from(name), false)),
  )
}

/// What the handlers of a zone were told.
#[derive(Default)]
struct Told {
  files: Vec<Vec<String>>,
  rejected: Vec<Vec<(String, RejectReason)>>,
}

fn hearing(told: &Arc<Mutex<Told>>, zone: DropZone) -> DropZone {
  let (files, rejected) = (told.clone(), told.clone());
  zone
    .on_files(move |given| {
      files
        .lock()
        .unwrap()
        .files
        .push(given.iter().map(|file| file.name().to_owned()).collect())
    })
    .on_reject(move |refused: &[RejectedFile]| {
      rejected.lock().unwrap().rejected.push(
        refused
          .iter()
          .map(|file| (file.file.name().to_owned(), file.reason))
          .collect(),
      )
    })
}

#[test]
fn a_zone_at_rest_is_the_boards_with_its_icon_title_and_description() {
  let mut stage = Stage::new(|_| Row::new().padding(8.0).child(DropZone::new().id("zone")).into());
  let (x, _, width, height) = stage.bounds("zone");
  assert_eq!((width, height), (320.0, 140.0));
  assert_eq!(stage.fill_color("zone"), Some(light(PaletteColor::Muted)));
  assert_eq!(edge(&mut stage, "zone"), Some((light(PaletteColor::Input), 1.5)));
  let rounded = stage.fill("zone").and_then(|quad| quad.border_radius).unwrap();
  assert_eq!(rounded.top_left, 8.0, "the large radius");

  assert_eq!(stage.text("Drop files here"), (light(PaletteColor::Foreground), 14.0));
  assert_eq!(
    stage.text("or click to browse"),
    (light(PaletteColor::MutedForeground), 12.0)
  );
  let (title, description) = (
    stage.text_quad("Drop files here").unwrap(),
    stage.text_quad("or click to browse").unwrap(),
  );
  let middle = x + width / 2.0;
  assert!(
    ((title.x + title.width / 2.0) - middle).abs() < 1.0,
    "the title in the middle"
  );
  assert!(((description.x + description.width / 2.0) - middle).abs() < 1.0);
  assert!(
    description.y > title.y + title.height - 1.0,
    "the description under the title"
  );
}

#[test]
fn a_drop_gives_the_files_it_takes_and_tells_what_it_refuses_and_why() {
  let told = Arc::new(Mutex::new(Told::default()));
  let heard = told.clone();
  let mut stage = Stage::new(move |_| {
    hearing(
      &heard,
      DropZone::new().accept(["image/*", ".PDF"]).max_files(2).id("zone"),
    )
    .into()
  });
  let (x, y) = stage.center("zone");
  let mut given = dragged(&["photo.png", "notes.txt", "scan.pdf", "map.jpg"])
    .items()
    .to_vec();
  given.push(DroppedPath::with_kind(PathBuf::from("folder"), true));
  stage.tree.file_drop(DraggedPaths::new(given), x, y);
  stage.pass();
  let told = told.lock().unwrap();
  assert_eq!(told.files, vec![vec!["photo.png".to_owned(), "scan.pdf".to_owned()]]);
  assert_eq!(
    told.rejected,
    vec![vec![
      ("notes.txt".to_owned(), RejectReason::Kind),
      ("map.jpg".to_owned(), RejectReason::TooMany),
      ("folder".to_owned(), RejectReason::Folder),
    ]]
  );
}

#[test]
fn a_file_larger_than_its_greatest_size_is_refused_as_too_large() {
  let dir = std::env::temp_dir().join(format!("lurq-drop-zone-{}", std::process::id()));
  std::fs::create_dir_all(&dir).unwrap();
  let (small, large) = (dir.join("small.txt"), dir.join("large.txt"));
  std::fs::write(&small, vec![b'a'; 10]).unwrap();
  std::fs::write(&large, vec![b'a'; 100]).unwrap();
  let told = Arc::new(Mutex::new(Told::default()));
  let heard = told.clone();
  let mut stage = Stage::new(move |_| hearing(&heard, DropZone::new().max_size(50).id("zone")).into());
  let (x, y) = stage.center("zone");
  stage.tree.file_drop(DraggedPaths::from_paths([small, large]), x, y);
  stage.pass();
  let told = told.lock().unwrap();
  assert_eq!(told.files, vec![vec!["small.txt".to_owned()]]);
  assert_eq!(
    told.rejected,
    vec![vec![("large.txt".to_owned(), RejectReason::TooLarge)]]
  );
  std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn one_that_takes_one_file_takes_the_first_and_refuses_the_rest() {
  let told = Arc::new(Mutex::new(Told::default()));
  let heard = told.clone();
  let mut stage = Stage::new(move |_| hearing(&heard, DropZone::new().multiple(false).id("zone")).into());
  let (x, y) = stage.center("zone");
  stage.tree.file_drop(dragged(&["a.txt", "b.txt"]), x, y);
  stage.pass();
  let told = told.lock().unwrap();
  assert_eq!(told.files, vec![vec!["a.txt".to_owned()]]);
  assert_eq!(told.rejected, vec![vec![("b.txt".to_owned(), RejectReason::TooMany)]]);
}

#[test]
fn a_mounted_zone_shows_a_drag_it_takes_and_one_it_refuses() {
  let mut stage = Stage::new(|ctx| {
    Row::new()
      .padding(8.0)
      .child(DropZone::new().accept(["png"]).id("zone").mount(ctx))
      .into()
  });
  let (x, y) = stage.center("zone");

  stage.tree.file_drag_enter(dragged(&["photo.png"]), x, y);
  stage.pass();
  assert_eq!(stage.tree.file_drag_accepted(), Some(true));
  assert_eq!(stage.fill_color("zone"), Some(light(PaletteColor::InfoMuted)));
  assert_eq!(edge(&mut stage, "zone"), Some((light(PaletteColor::Info), 2.0)));
  assert_eq!(stage.text("Drop to add").0, light(PaletteColor::Info));
  assert!(!stage.paints("or click to browse"), "a drag says one thing");

  stage.tree.file_drag_leave();
  stage.pass();
  assert_eq!(stage.fill_color("zone"), Some(light(PaletteColor::Muted)));
  assert!(stage.paints("Drop files here"));

  stage.tree.file_drag_enter(dragged(&["notes.txt"]), x, y);
  stage.pass();
  assert_eq!(stage.tree.file_drag_accepted(), Some(false), "the drag is refused");
  assert_eq!(stage.fill_color("zone"), Some(light(PaletteColor::DestructiveMuted)));
  assert_eq!(edge(&mut stage, "zone"), Some((light(PaletteColor::Destructive), 2.0)));
  assert_eq!(
    stage.text("These files can't be added").0,
    light(PaletteColor::Destructive)
  );

  stage.tree.file_drop(dragged(&["notes.txt"]), x, y);
  stage.pass();
  assert!(stage.paints("Drop files here"), "after the drop it is at rest again");
}

#[test]
fn the_still_zone_takes_a_drop_and_shows_no_drag() {
  let told = Arc::new(Mutex::new(Told::default()));
  let heard = told.clone();
  let mut stage = Stage::new(move |_| hearing(&heard, DropZone::new().id("zone")).into());
  let (x, y) = stage.center("zone");
  stage.tree.file_drag_enter(dragged(&["a.txt"]), x, y);
  stage.pass();
  assert!(stage.paints("Drop files here"));
  stage.tree.file_drop(dragged(&["a.txt"]), x, y);
  stage.pass();
  assert_eq!(told.lock().unwrap().files, vec![vec!["a.txt".to_owned()]]);
}

#[test]
fn a_disabled_zone_takes_no_drop_no_focus_and_is_dimmed() {
  let told = Arc::new(Mutex::new(Told::default()));
  let heard = told.clone();
  let mut stage = Stage::new(move |ctx| hearing(&heard, DropZone::new().disabled(true).id("zone")).mount(ctx));
  let (x, y) = stage.center("zone");
  stage.tree.file_drop(dragged(&["a.txt"]), x, y);
  stage.key("Tab");
  stage.pass();
  assert!(told.lock().unwrap().files.is_empty());
  assert!(!stage.has_ring(light(PaletteColor::Ring)));
  let theme = stage.app.theme().clone();
  let root = theme
    .resolve::<DropZone>(&Case::new().state(DropZoneState::Disabled, true), None)
    .part(DropZonePart::Root)
    .rest();
  assert_eq!(root.opacity, Some(Opacity::Level(OpacityLevel::Disabled)));
}

#[test]
fn tab_reaches_it_with_the_ring_and_invalid_and_forced_drag_states_draw() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .spacing(8.0)
      .child(DropZone::new().id("zone"))
      .child(DropZone::new().invalid(true).id("invalid"))
      .into()
  });
  stage.key("Tab");
  stage.pass();
  assert!(stage.has_ring(light(PaletteColor::Ring)), "the zone is a stop of Tab");
  assert_eq!(
    edge(&mut stage, "invalid"),
    Some((light(PaletteColor::Destructive), 1.5))
  );

  let mut stage = Stage::new(|_| {
    Column::new()
      .child(DropZone::new().state(DropZoneState::DragOver, true).id("over"))
      .child(
        DropZone::new()
          .state(DropZoneState::Rejected, true)
          .force(Some(Interaction::Hovered))
          .id("refused"),
      )
      .into()
  });
  assert_eq!(stage.fill_color("over"), Some(light(PaletteColor::InfoMuted)));
  assert_eq!(stage.fill_color("refused"), Some(light(PaletteColor::DestructiveMuted)));
  assert!(stage.paints("Drop to add") && stage.paints("These files can't be added"));
}

#[test]
fn a_title_longer_than_the_zone_wraps_inside_it_and_its_words_can_be_its_own() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .padding(8.0)
      .child(
        DropZone::new()
          .title("Drop the photographs of the whole trip here, every one of them")
          .description("PNG or JPG, up to 5 MB")
          .icon(None)
          .id("zone"),
      )
      .into()
  });
  let (x, _, width, height) = stage.bounds("zone");
  let title = stage
    .text_quad("Drop the photographs of the whole trip here, every one of them")
    .expect("the title, whole");
  assert!(title.x >= x && title.x + title.width <= x + width, "inside the zone");
  assert!(title.height > 30.0, "on more than one line: {}", title.height);
  assert!(
    height >= 140.0,
    "the zone grows with it, at least as high as the board's"
  );
  assert!(stage.paints("PNG or JPG, up to 5 MB"));
}

#[test]
fn a_theme_renderer_gives_the_zone_another_anatomy_and_it_still_takes_files() {
  let told = Arc::new(Mutex::new(Told::default()));
  let heard = told.clone();
  let app = App::new();
  let renderer: DropZoneRenderer = Arc::new(|view: DropZoneView| {
    let title = view.title.clone();
    view
      .zone()
      .child(lurq::components::Text::new(&format!("> {title}")))
      .into()
  });
  app.theme().set_renderer::<DropZone>(renderer);
  let mut stage = Stage::in_app(app, move |_| hearing(&heard, DropZone::new().id("zone")).into());
  assert!(stage.paints("> Drop files here"));
  let (x, y) = stage.center("zone");
  stage.tree.file_drop(dragged(&["a.txt"]), x, y);
  stage.pass();
  assert_eq!(told.lock().unwrap().files, vec![vec!["a.txt".to_owned()]]);
}

#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn to_tooling_it_is_a_button_called_by_its_label_or_its_title() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .child(DropZone::new().id("zone"))
      .child(DropZone::new().label("Attachments").id("named"))
      .into()
  });
  for (id, name) in [("zone", "Drop files here"), ("named", "Attachments")] {
    let element = stage.tree.get_element_by_id_mut(id).unwrap();
    assert_eq!(element.description("a11y_role").as_deref(), Some("button"));
    assert_eq!(element.description("a11y_name").as_deref(), Some(name));
  }
}

#[test]
fn a_dropped_file_of_a_path_reads_its_name() {
  assert_eq!(DroppedFile::from_path("C:/gone/report.pdf").name(), "report.pdf");
}
