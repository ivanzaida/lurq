//! A file upload: the board's button and list, what a drop adds, what a
//! person removes and retries, a file sent removed under a finger and a pointer, the forms of its list, and its states.

use std::{
  path::PathBuf,
  sync::{Arc, Mutex},
};

use lurq::{
  app::{
    App,
    events::{DraggedPaths, DroppedPath},
    theme::{PaletteColor, Theme, ThemePalette},
  },
  components::{Column, Row},
  core::Signal,
  files::DroppedFile,
  layout::quad::QuadContent,
  node::color::Color,
  ui::{
    Case, FileUpload, FileUploadPart, FileUploadRenderer, FileUploadView, Opacity, Recipe, RejectReason, RejectedFile,
    Themed, UploadFile, UploadState,
  },
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn edge(stage: &mut Stage, id: &str) -> Option<Color> {
  stage
    .border(id)
    .and_then(|quad| quad.border)
    .and_then(|border| border.top)
    .map(|side| side.color)
}

fn dragged(names: &[&str]) -> DraggedPaths {
  DraggedPaths::new(
    names
      .iter()
      .map(|name| DroppedPath::with_kind(PathBuf::from(name), false)),
  )
}

fn file(name: &str, size: u64) -> UploadFile {
  UploadFile::new(DroppedFile::new(name, Some(size)))
}

fn names(files: &[UploadFile]) -> Vec<String> {
  files.iter().map(|file| file.file.name().to_owned()).collect()
}

/// The board's two files: one sent, one 62% of the way.
fn boards() -> Vec<UploadFile> {
  vec![
    file("theme.json", 4096).done(),
    file("screenshot.png", 812_000).uploading(0.62),
  ]
}

#[test]
fn the_upload_is_the_boards_button_over_its_files() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .padding(8.0)
      .child(FileUpload::at(boards()).id("upload"))
      .into()
  });
  let (x, _, width, _) = stage.bounds("upload");
  assert_eq!(width, 300.0);
  let (bx, _, _, button_height) = stage.bounds("upload-trigger");
  assert_eq!(
    (bx, button_height),
    (x, 36.0),
    "the outline button of the board, at its start"
  );
  assert!(stage.paints("Choose files"));

  let (ix, iy, item_width, _) = stage.bounds("upload-theme.json");
  assert_eq!((ix, item_width), (x, 300.0), "a file as wide as the upload");
  let (_, by, ..) = stage.bounds("upload-trigger");
  assert_eq!(iy, by + 36.0 + 8.0, "8 under the button");
  assert_eq!(edge(&mut stage, "upload-theme.json"), Some(light(PaletteColor::Border)));
  assert_eq!(stage.text("theme.json"), (light(PaletteColor::Foreground), 13.0));
  assert_eq!(stage.text("4 KB"), (light(PaletteColor::MutedForeground), 11.0));
  assert_eq!(
    stage.text("62%").0,
    light(PaletteColor::MutedForeground),
    "how far it is sent, in place of its size"
  );
  let name = stage.text_quad("theme.json").unwrap();
  assert!(
    (name.x - (ix + 12.0 + 16.0 + 10.0)).abs() < 1.0,
    "after the padding, the icon and the gap"
  );
  let (_, sy, _, sh) = stage.bounds("upload-screenshot.png");
  assert!(
    sh > 40.0,
    "the file being sent is higher: its bar is under its line ({sh})"
  );
  assert_eq!(sy, iy + stage.bounds("upload-theme.json").3 + 8.0, "the files 8 apart");
}

#[test]
fn a_drop_adds_the_files_as_waiting_then_tells_the_handlers() {
  let files = Signal::new(boards());
  let told: Arc<Mutex<Vec<String>>> = Arc::default();
  let bound = files.clone();
  let heard = told.clone();
  let mut stage = Stage::new(move |_| {
    let (on_change, on_files, signal) = (heard.clone(), heard.clone(), bound.clone());
    FileUpload::new(bound.clone())
      .accept(["pdf", "png"])
      .on_change(move |list| {
        // The signal is set before a handler is told.
        assert_eq!(signal.get_untracked().len(), list.len());
        on_change.lock().unwrap().push(format!("change {}", list.len()))
      })
      .on_files(move |given| on_files.lock().unwrap().push(format!("files {}", given.len())))
      .id("upload")
      .into()
  });
  let (x, y) = stage.center("upload-trigger");
  stage.tree.file_drop(dragged(&["report.pdf", "notes.txt"]), x, y);
  stage.pass();
  let list = files.get();
  assert_eq!(names(&list), ["theme.json", "screenshot.png", "report.pdf"]);
  assert_eq!(list[2].state, UploadState::Waiting);
  assert_eq!(*told.lock().unwrap(), ["change 3", "files 1"]);
  assert!(stage.paints("report.pdf"), "the list shows what was added");
}

#[test]
fn the_remove_control_takes_a_file_out_and_retry_asks_the_app_to_send_it_again() {
  let files = Signal::new(vec![
    file("a.png", 10).uploading(0.3),
    file("b.png", 20).failed("The server refused it"),
  ]);
  let told: Arc<Mutex<Vec<String>>> = Arc::default();
  let (bound, heard) = (files.clone(), told.clone());
  let mut stage = Stage::new(move |_| {
    let (removed, retried) = (heard.clone(), heard.clone());
    FileUpload::new(bound.clone())
      .on_remove(move |file| removed.lock().unwrap().push(format!("remove {}", file.file.name())))
      .on_retry(move |file| retried.lock().unwrap().push(format!("retry {}", file.file.name())))
      .id("upload")
      .into()
  });
  assert_eq!(stage.text("The server refused it").0, light(PaletteColor::Destructive));
  assert_eq!(
    edge(&mut stage, "upload-b.png"),
    Some(light(PaletteColor::Destructive)),
    "a failed file's edge"
  );
  stage.click("upload-b.png-retry");
  assert_eq!(
    names(&files.get()),
    ["a.png", "b.png"],
    "retry changes nothing of the list"
  );
  stage.click("upload-a.png-remove");
  stage.pass();
  assert_eq!(names(&files.get()), ["b.png"]);
  assert_eq!(*told.lock().unwrap(), ["retry b.png", "remove a.png"]);
}

/// The id of the element the focus is on.
fn focused(stage: &mut Stage, _ids: &[&str]) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

#[test]
fn tab_reaches_the_button_then_each_control_and_a_removed_files_focus_goes_on() {
  let files = Signal::new(vec![file("a.png", 10), file("b.png", 20), file("c.png", 30)]);
  let bound = files.clone();
  let mut stage = Stage::new(move |ctx| {
    Row::new()
      .padding(8.0)
      .child(FileUpload::new(bound.clone()).id("upload").mount(ctx))
      .into()
  });
  let ids = [
    "upload-trigger",
    "upload-a.png-remove",
    "upload-b.png-remove",
    "upload-c.png-remove",
  ];
  stage.key("Tab");
  assert_eq!(focused(&mut stage, &ids).as_deref(), Some("upload-trigger"));
  stage.key("Tab");
  stage.key("Tab");
  assert_eq!(focused(&mut stage, &ids).as_deref(), Some("upload-b.png-remove"));
  stage.key("Enter");
  stage.pass();
  assert_eq!(names(&files.get()), ["a.png", "c.png"]);
  assert_eq!(
    focused(&mut stage, &ids).as_deref(),
    Some("upload-c.png-remove"),
    "on to the next file"
  );
  stage.key("Enter");
  stage.pass();
  assert_eq!(
    focused(&mut stage, &ids).as_deref(),
    Some("upload-a.png-remove"),
    "the last gone: the one before"
  );
  stage.key("Enter");
  stage.pass();
  assert!(files.get().is_empty());
  assert_eq!(
    focused(&mut stage, &ids).as_deref(),
    Some("upload-trigger"),
    "none left: the button"
  );
  assert!(stage.has_ring(light(PaletteColor::Ring)));
}

#[test]
fn held_by_the_app_it_tells_the_new_list_and_shows_what_the_app_answers_with() {
  let told: Arc<Mutex<Vec<Vec<String>>>> = Arc::default();
  let heard = told.clone();
  let mut stage = Stage::new(move |_| {
    let heard = heard.clone();
    FileUpload::controlled(vec![file("a.png", 10)], move |list| {
      heard.lock().unwrap().push(names(list))
    })
    .id("upload")
    .into()
  });
  let (x, y) = stage.center("upload-trigger");
  stage.tree.file_drop(dragged(&["b.png"]), x, y);
  stage.click("upload-a.png-remove");
  stage.pass();
  assert_eq!(
    *told.lock().unwrap(),
    vec![vec!["a.png".to_owned(), "b.png".to_owned()], vec![]]
  );
  assert!(
    stage.paints("a.png") && !stage.paints("b.png"),
    "the app answered nothing: the list is as it was"
  );
}

#[test]
fn shown_and_not_operated_nothing_is_added_or_removed() {
  let mut stage = Stage::new(|_| FileUpload::at(boards()).id("upload").into());
  let (x, y) = stage.center("upload-trigger");
  stage.tree.file_drop(dragged(&["b.png"]), x, y);
  stage.pass();
  assert!(!stage.paints("b.png"));
  assert!(
    stage
      .tree
      .get_element_by_id("upload-screenshot.png-remove")
      .is_none_or(|element| element.tag_name() != "Button"),
    "no control is pressed"
  );
}

#[test]
fn its_most_files_count_those_it_holds() {
  let files = Signal::new(vec![file("a.png", 10)]);
  let refused: Arc<Mutex<Vec<(String, RejectReason)>>> = Arc::default();
  let (bound, heard) = (files.clone(), refused.clone());
  let mut stage = Stage::new(move |_| {
    let heard = heard.clone();
    FileUpload::new(bound.clone())
      .max_files(2)
      .on_reject(move |refused: &[RejectedFile]| {
        heard
          .lock()
          .unwrap()
          .extend(refused.iter().map(|file| (file.file.name().to_owned(), file.reason)))
      })
      .id("upload")
      .into()
  });
  let (x, y) = stage.center("upload-trigger");
  stage.tree.file_drop(dragged(&["b.png", "c.png"]), x, y);
  stage.pass();
  assert_eq!(names(&files.get()), ["a.png", "b.png"]);
  assert_eq!(*refused.lock().unwrap(), [("c.png".to_owned(), RejectReason::TooMany)]);
}

#[test]
fn a_long_name_is_cut_and_its_size_keeps_its_room() {
  let long = "the-photographs-of-the-whole-trip-to-the-mountains-in-the-summer.png";
  let mut stage = Stage::new(move |_| FileUpload::at(vec![file(long, 4096)]).id("upload").into());
  let (x, _, width, _) = stage.bounds("upload");
  assert!(!stage.paints(long), "the name is cut");
  let name = stage.cut_text_quad(long).expect("the name, cut with an ellipsis");
  let size = stage.text_quad("4 KB").expect("the size, whole");
  assert!(name.x + name.width <= size.x, "the name ends before the size");
  assert!(
    size.x + size.width <= x + width - 12.0 + 0.5,
    "the size inside the padding"
  );
}

#[test]
fn with_its_zone_a_drag_shows_on_the_zone_and_a_drop_adds() {
  let files = Signal::new(Vec::new());
  let bound = files.clone();
  let mut stage = Stage::new(move |ctx| {
    Row::new()
      .padding(8.0)
      .child(FileUpload::new(bound.clone()).zone(true).id("upload").mount(ctx))
      .into()
  });
  assert!(stage.paints("Drop files here"));
  let (_, _, width, _) = stage.bounds("upload-trigger");
  assert_eq!(width, 320.0, "the board's zone");
  let (x, y) = stage.center("upload-trigger");
  stage.tree.file_drag_enter(dragged(&["a.png"]), x, y);
  stage.pass();
  assert_eq!(stage.fill_color("upload-trigger"), Some(light(PaletteColor::InfoMuted)));
  stage.tree.file_drop(dragged(&["a.png"]), x, y);
  stage.pass();
  assert_eq!(names(&files.get()), ["a.png"]);
  assert!(stage.paints("Drop files here"), "at rest again");
}

#[test]
fn a_disabled_upload_takes_no_drop_and_removes_nothing() {
  let files = Signal::new(vec![file("a.png", 10)]);
  let bound = files.clone();
  let mut stage = Stage::new(move |_| FileUpload::new(bound.clone()).disabled(true).id("upload").into());
  let (x, y) = stage.center("upload-trigger");
  stage.tree.file_drop(dragged(&["b.png"]), x, y);
  stage.pass();
  assert_eq!(names(&files.get()), ["a.png"]);
  assert!(
    stage
      .tree
      .get_element_by_id("upload-a.png-remove")
      .is_none_or(|element| element.tag_name() != "Button"),
    "no control is pressed"
  );
}

#[test]
fn a_theme_renderer_gives_another_anatomy_with_the_same_files() {
  let app = App::new();
  let renderer: FileUploadRenderer = Arc::new(|view: FileUploadView| {
    let mut column = Column::new().child(view.trigger());
    for (index, file) in view.files.iter().enumerate() {
      column = column.child(lurq::components::Text::new(&format!("{index}: {}", file.file.name())));
    }
    column.into()
  });
  app.theme().set_renderer::<FileUpload>(renderer);
  let stage = Stage::in_app(app, |_| FileUpload::at(boards()).into());
  assert!(stage.paints("0: theme.json") && stage.paints("1: screenshot.png"));
}

#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn to_tooling_a_files_controls_are_called_by_what_they_do_and_its_name() {
  let files = Signal::new(vec![file("b.png", 20).failed("Refused")]);
  let mut stage = Stage::new(move |_| FileUpload::new(files.clone()).id("upload").into());
  let mut said = |id: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description("a11y_name"))
  };
  assert_eq!(said("upload-b.png-retry").as_deref(), Some("Retry b.png"));
  assert_eq!(said("upload-b.png-remove").as_deref(), Some("Remove b.png"));
}

#[test]
fn a_file_given_again_is_refused_as_a_duplicate() {
  // As a drop gives it: of no size the test's paths have, as files that are not there.
  let files = Signal::new(vec![UploadFile::new(DroppedFile::new("a.png", None))]);
  let refused: Arc<Mutex<Vec<(String, RejectReason)>>> = Arc::default();
  let (bound, heard) = (files.clone(), refused.clone());
  let mut stage = Stage::new(move |_| {
    let heard = heard.clone();
    FileUpload::new(bound.clone())
      .multiple(true)
      .on_reject(move |refused: &[RejectedFile]| {
        heard
          .lock()
          .unwrap()
          .extend(refused.iter().map(|file| (file.file.name().to_owned(), file.reason)))
      })
      .id("upload")
      .into()
  });
  let (x, y) = stage.center("upload-trigger");
  // The one it holds, and one given twice at once.
  stage.tree.file_drop(dragged(&["a.png", "b.png", "b.png"]), x, y);
  stage.pass();
  assert_eq!(names(&files.get()), ["a.png", "b.png"]);
  assert_eq!(
    *refused.lock().unwrap(),
    [
      ("a.png".to_owned(), RejectReason::Duplicate),
      ("b.png".to_owned(), RejectReason::Duplicate)
    ]
  );
}

/// An app a finger points in, as an app on a phone starts: pinned, so that the stage's own clicks leave it.
fn for_a_finger() -> App {
  let app = App::new();
  app.set_pointer(lurq::app::Pointer::Coarse);
  app
}

#[test]
fn under_a_finger_a_file_sent_has_its_remove_control_beside_its_check() {
  let files = Signal::new(vec![file("a.png", 10).done(), file("b.png", 20)]);
  let bound = files.clone();
  let mut stage = Stage::in_app(for_a_finger(), move |_| {
    Row::new()
      .padding(8.0)
      .child(FileUpload::new(bound.clone()).id("upload"))
      .into()
  });
  let (ix, _, iw, _) = stage.bounds("upload-a.png");
  let (rx, _, rw, _) = stage.bounds("upload-a.png-remove");
  assert_eq!(rx + rw, ix + iw - 12.0, "at the row's end, inside its padding");
  let (wx, ..) = stage.bounds("upload-b.png-remove");
  assert_eq!(wx, rx, "at the end, as a waiting file's");
  stage.tap("upload-a.png-remove");
  stage.pass();
  assert_eq!(names(&files.get()), ["b.png"]);
}

#[test]
fn under_a_pointer_a_file_sents_check_gives_way_to_its_remove_control_while_its_row_is_pointed_at_or_focused() {
  // The design: the control hidden at rest over the check, shown in the row's light, and the check hidden then.
  let theme = Theme::new();
  let recipe = FileUpload::recipe().resolve(&Case::new(), &theme);
  assert_eq!(
    recipe.part(FileUploadPart::Reveal).styles().rest.opacity,
    Some(Opacity::Value(0.0))
  );
  assert_eq!(
    recipe.part(FileUploadPart::Reveal).styles().within.opacity,
    Some(Opacity::Value(1.0))
  );
  assert_eq!(
    recipe.part(FileUploadPart::Status).styles().within.opacity,
    Some(Opacity::Value(0.0))
  );

  // Live: the part is lit with the row. A rule of the test's paints it while it is.
  let lit = light(PaletteColor::Primary);
  let files = Signal::new(vec![file("a.png", 10).done(), file("b.png", 20)]);
  let bound = files.clone();
  let mut stage = Stage::new(move |ctx| {
    Row::new()
      .padding(8.0)
      .child(
        FileUpload::new(bound.clone())
          .rules(&Recipe::new().within(FileUploadPart::Reveal, |s| s.background(PaletteColor::Primary)))
          .id("upload")
          .mount(ctx),
      )
      .into()
  });
  let shown = |stage: &Stage| {
    stage
      .quads()
      .iter()
      .any(|quad| matches!(&quad.content, QuadContent::Rect { color, .. } if *color == lit))
  };
  assert!(!shown(&stage), "at rest: the check");
  stage.hover("upload-a.png");
  assert!(shown(&stage), "the row under the pointer: the remove control");
  stage.hover("upload-b.png");
  assert!(!shown(&stage), "another row under the pointer");
  stage.key("Tab");
  stage.key("Tab");
  assert_eq!(
    focused(&mut stage, &[]).as_deref(),
    Some("upload-a.png-remove"),
    "the keyboard reaches it"
  );
  assert!(shown(&stage), "the control with the focus");
  stage.key("Enter");
  stage.pass();
  assert_eq!(names(&files.get()), ["b.png"]);
}

#[test]
fn a_file_sent_that_is_not_removed_has_its_check_alone() {
  let mut stage = Stage::new(|_| {
    FileUpload::new(Signal::new(vec![file("a.png", 10).done().disabled(true)]))
      .id("upload")
      .into()
  });
  assert!(
    stage
      .tree
      .get_element_by_id("upload-a.png-remove")
      .is_none_or(|element| element.tag_name() != "Button"),
    "no control is pressed"
  );
}
