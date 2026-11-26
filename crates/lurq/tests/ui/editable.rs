//! An editable: its two forms against the board, what starts an edit, what keeps it and what drops it, where
//! the focus goes on each way out, the bound and the held text, a text longer than its room, read only and
//! disabled, and what it is to tooling.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App,
    theme::{PaletteColor, ThemePalette},
  },
  components::{Column, Row},
  core::Signal,
  layout::quad::QuadContent,
  node::color::Color,
  ui::{Editable, EditablePart},
};

use super::stage::Stage;

const ID: &str = "title";
const FIELD: &str = "title-field";

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn close(a: f32, b: f32) -> bool {
  (a - b).abs() < 0.6
}

/// What the editable told the app, in order: "change:…", "cancel", "editing:true".
type Log = Arc<Mutex<Vec<String>>>;

fn note(log: &Log, what: impl Into<String>) {
  log.lock().unwrap().push(what.into());
}

/// An editable of "Project Alpha" between two buttons, the app's signal of its text, and what it told.
fn scene(adjust: impl Fn(Editable) -> Editable + Send + Sync + 'static) -> (Stage, Signal<String>, Log) {
  let value = Signal::new("Project Alpha".to_owned());
  let log = Log::default();
  let (held, told) = (value.clone(), log.clone());
  let stage = Stage::new(move |ctx| {
    let (changed, cancelled, turned) = (told.clone(), told.clone(), told.clone());
    let editable = Editable::new(held.clone())
      .id(ID)
      .on_change(move |text| note(&changed, format!("change:{text}")))
      .on_cancel(move || note(&cancelled, "cancel"))
      .on_editing_change(move |editing| note(&turned, format!("editing:{editing}")));
    Column::new()
      .size(400.0, 300.0)
      .child(lurq::components::Button::new("Before").id("before").tab_index(0))
      .child(Row::new().id("room").width(320.0).child(adjust(editable).mount(ctx)))
      .child(lurq::components::Button::new("After").id("after").tab_index(0))
      .into()
  });
  (stage, value, log)
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

fn exists(stage: &mut Stage, id: &str) -> bool {
  stage.tree.get_element_by_id_mut(id).is_some()
}

fn type_text(stage: &mut Stage, text: &str) {
  for character in text.chars() {
    let key = character.to_string();
    stage.tree.key_down(key.clone(), "Key".into(), false, false, false);
    stage.tree.key_up(key, "Key".into(), false, false, false);
  }
  stage.pass();
}

fn told(log: &Log) -> Vec<String> {
  log.lock().unwrap().clone()
}

#[test]
fn the_reading_form_is_the_boards_row_of_its_text_and_a_pencil() {
  let (mut stage, ..) = scene(|editable| editable);
  let (x, _, width, height) = stage.bounds(ID);
  assert_eq!(height, 32.0);
  let (color, size) = stage.text("Project Alpha");
  assert_eq!((color, size), (light(PaletteColor::Foreground), 16.0));
  let text = stage.text_quad("Project Alpha").unwrap();
  let QuadContent::Text { style, .. } = &text.content else {
    unreachable!()
  };
  assert_eq!(style.weight, lurq::layout::text_style::FontWeight::SemiBold);
  // The pencil: 14 across in the muted foreground, 8 after the text, at the end of the row.
  let pencil = stage
    .quads()
    .into_iter()
    .find(|quad| {
      matches!(&quad.content, QuadContent::Text { style, .. } if style.color == light(PaletteColor::MutedForeground))
    })
    .expect("the pencil");
  assert!(
    close(pencil.width, 14.0) && close(pencil.height, 14.0),
    "{}",
    pencil.width
  );
  assert!(close(pencil.x - (text.x + text.width), 8.0), "8 after the text");
  assert!(
    close(pencil.x + pencil.width, x + width),
    "the row is as wide as its text and its pencil"
  );
  assert!(!exists(&mut stage, FIELD), "no field until it is edited");
}

#[test]
fn a_press_puts_a_field_of_160_and_two_buttons_of_32_where_the_text_was() {
  let (mut stage, ..) = scene(|editable| editable);
  let (x, y, ..) = stage.bounds(ID);
  stage.click(ID);
  assert_eq!(focused(&stage).as_deref(), Some(FIELD), "the field has the focus");
  let (field_x, field_y, field_width, field_height) = stage.bounds(FIELD);
  assert_eq!((field_x, field_y), (x, y), "where the text was");
  assert_eq!((field_width, field_height), (160.0, 32.0));
  let (confirm_x, _, confirm_width, confirm_height) = stage.bounds("title-confirm");
  assert_eq!((confirm_width, confirm_height), (32.0, 32.0));
  assert!(close(confirm_x - (field_x + 160.0), 6.0), "6 after the field");
  let (cancel_x, _, cancel_width, _) = stage.bounds("title-cancel");
  assert!(close(cancel_x - (confirm_x + 32.0), 6.0) && cancel_width == 32.0);
  // The one that keeps the edit has the edge of an outline button; the one that drops it has none.
  assert!(stage.border("title-confirm").is_some());
  assert!(stage.border("title-cancel").is_none());
  // The text is selected: what is typed takes its place.
  type_text(&mut stage, "Beta");
  let field = stage.tree.get_element_by_id_mut(FIELD).unwrap();
  assert_eq!(field.as_text_input().unwrap().value(), "Beta");
}

#[test]
fn enter_keeps_the_edit_and_gives_the_focus_back_to_the_reading_form() {
  let (mut stage, value, log) = scene(|editable| editable);
  stage.tree.get_element_by_id_mut(ID).unwrap().focus();
  stage.key("Enter");
  assert_eq!(focused(&stage).as_deref(), Some(FIELD), "Enter starts an edit");
  type_text(&mut stage, "Project Beta");
  assert_eq!(value.get(), "Project Alpha", "nothing is kept while it is typed");
  stage.key("Enter");
  assert_eq!(value.get(), "Project Beta");
  assert_eq!(
    told(&log),
    ["editing:true", "change:Project Beta", "editing:false"],
    "the signal is set, then the change is told, then that the edit ended"
  );
  assert!(!exists(&mut stage, FIELD));
  assert_eq!(
    focused(&stage).as_deref(),
    Some(ID),
    "the focus is back on the reading form"
  );
  assert!(stage.paints("Project Beta"));
}

#[test]
fn f2_starts_an_edit_and_escape_drops_it() {
  let (mut stage, value, log) = scene(|editable| editable);
  stage.tree.get_element_by_id_mut(ID).unwrap().focus();
  stage.key("F2");
  assert_eq!(focused(&stage).as_deref(), Some(FIELD));
  type_text(&mut stage, "Thrown away");
  stage.key("Escape");
  assert_eq!(value.get(), "Project Alpha");
  assert_eq!(told(&log), ["editing:true", "cancel", "editing:false"]);
  assert_eq!(focused(&stage).as_deref(), Some(ID));
  assert!(stage.paints("Project Alpha"));
  // The next edit starts from the text again, not from what was thrown away.
  stage.key("F2");
  let field = stage.tree.get_element_by_id_mut(FIELD).unwrap();
  assert_eq!(field.as_text_input().unwrap().value(), "Project Alpha");
}

#[test]
fn leaving_the_field_keeps_the_edit_and_leaves_the_focus_where_it_went() {
  let (mut stage, value, log) = scene(|editable| editable);
  stage.click(ID);
  type_text(&mut stage, "Renamed");
  stage.click("after");
  assert_eq!(value.get(), "Renamed");
  assert_eq!(told(&log), ["editing:true", "change:Renamed", "editing:false"]);
  assert_eq!(focused(&stage).as_deref(), Some("after"), "the focus is not taken back");
  assert!(!exists(&mut stage, FIELD));
}

#[test]
fn leaving_the_field_drops_the_edit_where_the_editable_was_told_so() {
  let (mut stage, value, log) = scene(|editable| editable.submit_on_blur(false));
  stage.click(ID);
  type_text(&mut stage, "Renamed");
  stage.click("after");
  assert_eq!(value.get(), "Project Alpha");
  assert_eq!(told(&log), ["editing:true", "cancel", "editing:false"]);
}

#[test]
fn the_buttons_keep_and_drop_the_edit_with_the_focus_back_on_the_reading_form() {
  let (mut stage, value, log) = scene(|editable| editable);
  stage.click(ID);
  type_text(&mut stage, "Dropped");
  stage.click("title-cancel");
  assert_eq!(
    value.get(),
    "Project Alpha",
    "a press of Cancel does not keep the edit on its way"
  );
  assert_eq!(told(&log), ["editing:true", "cancel", "editing:false"]);
  assert_eq!(focused(&stage).as_deref(), Some(ID));
  log.lock().unwrap().clear();
  stage.click(ID);
  type_text(&mut stage, "Kept");
  stage.click("title-confirm");
  assert_eq!(value.get(), "Kept");
  assert_eq!(told(&log), ["editing:true", "change:Kept", "editing:false"]);
  assert_eq!(focused(&stage).as_deref(), Some(ID));
}

#[test]
fn an_edit_that_changes_nothing_tells_nothing() {
  let (mut stage, value, log) = scene(|editable| editable);
  stage.click(ID);
  stage.key("Enter");
  assert_eq!(value.get(), "Project Alpha");
  assert_eq!(told(&log), ["editing:true", "editing:false"]);
}

#[test]
fn the_app_sets_the_text_and_is_not_told() {
  let (mut stage, value, log) = scene(|editable| editable);
  value.set("Set by the app".to_owned());
  stage.pass();
  assert!(stage.paints("Set by the app"));
  assert!(told(&log).is_empty());
}

#[test]
fn a_held_text_is_told_the_edit_and_shows_what_the_app_answers_with() {
  let text = Signal::new("Held".to_owned());
  let log = Log::default();
  let (held, told_to) = (text.clone(), log.clone());
  let mut stage = Stage::new(move |ctx| {
    let (answer, heard) = (held.clone(), told_to.clone());
    // The app keeps the text in capitals, whatever was typed.
    let editable = Editable::controlled(&held.get(), move |typed| {
      note(&heard, format!("change:{typed}"));
      answer.set(typed.to_uppercase());
    })
    .id(ID);
    Column::new().size(400.0, 300.0).child(editable.mount(ctx)).into()
  });
  stage.click(ID);
  type_text(&mut stage, "typed");
  stage.key("Enter");
  assert_eq!(told(&log), ["change:typed"]);
  assert!(stage.paints("TYPED"), "what the app answered with");
}

#[test]
fn the_apps_signal_starts_and_ends_an_edit_without_a_report() {
  let editing = Signal::new(false);
  let bound = editing.clone();
  let (mut stage, _, log) = scene(move |editable| editable.editing(bound.clone()));
  assert!(!exists(&mut stage, FIELD));
  editing.set(true);
  stage.pass();
  assert_eq!(
    focused(&stage).as_deref(),
    Some(FIELD),
    "the field takes the focus as the app starts an edit"
  );
  let field = stage.tree.get_element_by_id_mut(FIELD).unwrap();
  assert_eq!(field.as_text_input().unwrap().value(), "Project Alpha");
  assert!(told(&log).is_empty());
  // A person ends it: the app's signal is set, and it is told.
  stage.key("Escape");
  assert!(!editing.get());
  assert_eq!(told(&log), ["cancel", "editing:false"]);
}

#[test]
fn an_editable_edited_from_the_start_shows_its_field_with_the_focus() {
  let (mut stage, ..) = scene(|editable| editable.default_editing(true));
  assert!(exists(&mut stage, FIELD));
  assert_eq!(focused(&stage).as_deref(), Some(FIELD));
}

#[test]
fn an_empty_editable_shows_its_placeholder_in_the_muted_foreground() {
  let (mut stage, value, _) = scene(|editable| editable.placeholder("Untitled"));
  value.set(String::new());
  stage.pass();
  assert_eq!(stage.text("Untitled").0, light(PaletteColor::MutedForeground));
  let (.., height) = stage.bounds(ID);
  assert_eq!(height, 32.0, "it keeps its row with nothing in it");
}

#[test]
fn a_text_longer_than_its_room_is_cut_and_the_pencil_keeps_its_place() {
  let long = "A project with a name much longer than the room the editable was given";
  let (mut stage, value, _) = scene(|editable| editable);
  value.set(long.to_owned());
  stage.pass();
  let (room_x, _, room_width, _) = stage.bounds("room");
  let (x, _, width, _) = stage.bounds(ID);
  assert!(
    x + width <= room_x + room_width + 0.5,
    "it is no wider than its room: {width}"
  );
  let cut = stage.cut_text_quad(long).expect("the text, cut");
  let QuadContent::Text { text, .. } = &cut.content else {
    unreachable!()
  };
  assert!(text.ends_with('…'), "cut with an ellipsis: {text:?}");
  let pencil = stage
    .quads()
    .into_iter()
    .find(|quad| {
      matches!(&quad.content, QuadContent::Text { style, .. } if style.color == light(PaletteColor::MutedForeground))
    })
    .expect("the pencil");
  assert!(close(pencil.width, 14.0) && pencil.x + pencil.width <= room_x + room_width + 0.5);
  // Edited in a room narrower than its field and buttons, the field gives way and the buttons keep theirs.
  stage.click(ID);
  let (cancel_x, _, cancel_width, _) = stage.bounds("title-cancel");
  assert!(cancel_x + cancel_width <= room_x + room_width + 0.5);
}

#[test]
fn a_full_width_editable_fills_its_room_in_both_forms() {
  let (mut stage, ..) = scene(|editable| editable.full_width(true));
  let (_, _, width, _) = stage.bounds(ID);
  assert_eq!(width, 320.0);
  stage.click(ID);
  let (field_x, _, field_width, _) = stage.bounds(FIELD);
  let (cancel_x, _, cancel_width, _) = stage.bounds("title-cancel");
  assert!(
    close(field_width, 320.0 - 2.0 * (32.0 + 6.0)),
    "the field takes what the buttons leave: {field_width}"
  );
  assert!(close(cancel_x + cancel_width, field_x + 320.0));
}

#[test]
fn without_its_actions_the_editing_form_is_the_field_alone() {
  let (mut stage, value, _) = scene(|editable| editable.actions(false));
  stage.click(ID);
  assert!(!exists(&mut stage, "title-confirm") && !exists(&mut stage, "title-cancel"));
  type_text(&mut stage, "By Enter");
  stage.key("Enter");
  assert_eq!(value.get(), "By Enter");
}

#[test]
fn a_read_only_editable_has_no_pencil_and_a_disabled_one_takes_nothing() {
  let (mut stage, _, log) = scene(|editable| editable.read_only(true));
  assert!(
    !stage.quads().iter().any(|quad| {
      matches!(&quad.content, QuadContent::Text { style, .. } if style.color == light(PaletteColor::MutedForeground))
    }),
    "no pencil"
  );
  stage.click(ID);
  assert!(!exists(&mut stage, FIELD));
  assert!(told(&log).is_empty());

  let (mut stage, _, log) = scene(|editable| editable.disabled(true));
  stage.click(ID);
  stage.key("F2");
  assert!(!exists(&mut stage, FIELD));
  assert!(told(&log).is_empty());
  let text = stage.text_quad("Project Alpha").unwrap();
  assert!(close(text.opacity, 0.5), "at half its opacity: {}", text.opacity);
}

#[test]
fn tab_stops_at_the_reading_form_once_and_shows_its_ring() {
  let (mut stage, ..) = scene(|editable| editable);
  stage.tree.get_element_by_id_mut("before").unwrap().focus();
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some(ID));
  assert!(
    stage.has_ring(light(PaletteColor::Ring)),
    "the ring of a focus from the keyboard"
  );
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("after"));
}

#[test]
fn a_themes_rule_changes_the_reading_form() {
  let app = App::new();
  app.theme().patch_recipe::<Editable>(|recipe| {
    recipe
      .base(EditablePart::Root, |s| s.height(40.0))
      .base(EditablePart::Label, |s| s.font_size(20.0))
  });
  let mut stage = Stage::in_app(app, |ctx| {
    Column::new()
      .size(400.0, 300.0)
      .child(Editable::new(Signal::new("Themed".to_owned())).id(ID).mount(ctx))
      .into()
  });
  assert_eq!(stage.bounds(ID).3, 40.0);
  assert_eq!(stage.text("Themed").1, 20.0);
}

#[test]
fn the_still_form_is_the_reading_form_with_nothing_kept() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .size(400.0, 300.0)
      .child(Editable::new(Signal::new("Still".to_owned())).id(ID))
      .into()
  });
  assert!(stage.paints("Still"));
  assert_eq!(stage.bounds(ID).3, 32.0);
}

#[cfg(any(feature = "devtools", feature = "mcp"))]
#[test]
fn it_is_a_button_called_by_its_label_and_its_field_a_textbox() {
  let (mut stage, ..) = scene(|editable| editable.label("Project name"));
  let describe = |stage: &mut Stage, id: &str, key: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(key))
  };
  assert_eq!(describe(&mut stage, ID, "a11y_name").as_deref(), Some("Project name"));
  assert_eq!(
    describe(&mut stage, ID, "a11y_description").as_deref(),
    Some("Project Alpha")
  );
  stage.click(ID);
  assert_eq!(describe(&mut stage, FIELD, "a11y_role").as_deref(), Some("textbox"));
  assert_eq!(
    describe(&mut stage, FIELD, "a11y_name").as_deref(),
    Some("Project name")
  );
  assert_eq!(
    describe(&mut stage, "title-confirm", "a11y_name").as_deref(),
    Some("Save")
  );
  assert_eq!(
    describe(&mut stage, "title-cancel", "a11y_name").as_deref(),
    Some("Cancel")
  );
}

#[test]
fn an_editable_the_apps_signal_shows_edited_from_the_start_holds_its_text_and_leaves_the_focus() {
  let editing = Signal::new(true);
  let bound = editing.clone();
  let (mut stage, ..) = scene(move |editable| editable.editing(bound.clone()));
  let field = stage.tree.get_element_by_id_mut(FIELD).unwrap();
  assert_eq!(field.as_text_input().unwrap().value(), "Project Alpha");
  assert_ne!(
    focused(&stage).as_deref(),
    Some(FIELD),
    "the focus is left where it was"
  );
}

#[test]
fn tab_in_the_field_keeps_the_edit_and_goes_on_to_the_control_after_it() {
  let (mut stage, value, log) = scene(|editable| editable);
  stage.click(ID);
  type_text(&mut stage, "By Tab");
  stage.key("Tab");
  assert_eq!(value.get(), "By Tab");
  assert_eq!(told(&log), ["editing:true", "change:By Tab", "editing:false"]);
  assert_eq!(
    focused(&stage).as_deref(),
    Some("after"),
    "the buttons of the editing form are no stops of Tab"
  );
}

#[test]
fn what_the_app_puts_at_the_fields_end_stands_over_it_and_the_text_keeps_clear_of_it() {
  let (mut stage, ..) = scene(|editable| {
    editable
      .actions(false)
      .end(lurq::components::Text::new("Enter").id("enter"))
  });
  assert!(!exists(&mut stage, "enter"), "only while it is edited");
  stage.click(ID);
  let (field_x, field_y, field_width, field_height) = stage.bounds(FIELD);
  let (x, y, width, height) = stage.bounds("enter");
  assert!(
    x >= field_x + field_width - 40.0 - 12.0 && x + width <= field_x + field_width + 0.5,
    "at the field's end: {x} {width} in {field_x} {field_width}"
  );
  assert!(
    y >= field_y && y + height <= field_y + field_height,
    "in the field's box"
  );
  // What is typed goes into the field, and Enter keeps it.
  type_text(&mut stage, "Pages");
  stage.key("Enter");
  assert!(!exists(&mut stage, "enter"));
}
