//! A dialog's mark before its title, its note at the footer's start and its confirm on Enter; a field's error icon
//! and its control at the row's end.

use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{App, ctx::Ctx},
  components::{Column, Row, TextInput},
  core::Signal,
  node::Element,
  ui::{Button, Dialog, Field, Input, Switch, icons},
};

use super::stage::Stage;

fn stage(scene: impl Fn(&mut Ctx) -> Element + Send + Sync + 'static) -> Stage {
  let mut stage = Stage::in_app(App::new(), scene);
  stage.tree.resize(400, 300);
  stage.pass();
  stage
}

fn counter() -> (Arc<AtomicUsize>, impl Fn() + Send + Sync + Clone + 'static) {
  let count = Arc::new(AtomicUsize::new(0));
  let counted = count.clone();
  (count, move || {
    counted.fetch_add(1, Ordering::SeqCst);
  })
}

fn count(counter: &AtomicUsize) -> usize {
  counter.load(Ordering::SeqCst)
}

#[test]
fn a_dialog_has_a_mark_before_its_title_and_a_note_before_its_buttons() {
  let mut stage = stage(|_| {
    Column::new()
      .size(400.0, 300.0)
      .child(
        Dialog::new(Signal::new(true))
          .title("Delete connection?")
          .icon(icons::TRASH)
          .note("Nothing is deleted yet")
          .note_icon(icons::INFO)
          .footer(Button::new("Delete").id("delete"))
          .inline(true)
          .id("d"),
      )
      .into()
  });
  let ids = stage.ids();
  for id in ["d-mark", "d-icon", "d-note", "d-note_icon"] {
    assert!(ids.iter().any(|held| held == id), "{id} in {ids:?}");
  }
  let (mark_x, _, mark_width, _) = stage.bounds("d-mark");
  let title = stage.text_quad("Delete connection?").expect("the title");
  assert!(mark_x + mark_width <= title.x, "the mark before the title");
  let note = stage.text_quad("Nothing is deleted yet").expect("the note");
  let (delete_x, ..) = stage.bounds("delete");
  let (panel_x, ..) = stage.bounds("d");
  assert!(note.x + note.width <= delete_x, "the note before the buttons");
  assert!(
    note.x - panel_x < 60.0,
    "the note at the footer's start: {} from the panel",
    note.x - panel_x
  );
}

#[test]
fn a_dialog_given_media_draws_it_in_place_of_the_icon() {
  let stage = stage(|_| {
    Column::new()
      .size(400.0, 300.0)
      .child(
        Dialog::new(Signal::new(true))
          .title("Bind a server")
          .icon(icons::TRASH)
          .media(Row::new().size(20.0, 20.0).id("tile"))
          .inline(true)
          .id("d"),
      )
      .into()
  });
  let ids = stage.ids();
  assert!(
    ids.iter().any(|id| id == "tile") && !ids.iter().any(|id| id == "d-icon"),
    "{ids:?}"
  );
}

/// A dialog over the window that confirms on Enter, with a field of one line, one of several, and a button.
fn confirming() -> (Stage, Arc<AtomicUsize>, Arc<AtomicUsize>) {
  let (confirmed, confirm) = counter();
  let (kept, keep) = counter();
  let stage = stage(move |_| {
    let (confirm, keep) = (confirm.clone(), keep.clone());
    Column::new()
      .size(400.0, 300.0)
      .child(
        Dialog::new(Signal::new(true))
          .title("Rename")
          .child(TextInput::new(Signal::new(String::new())).id("name").width(200.0))
          .child(
            TextInput::new(Signal::new(String::new()))
              .multiline()
              .id("notes")
              .width(200.0),
          )
          .footer(Button::new("Keep").id("keep").on_click(move |_| keep()))
          .on_confirm(move || confirm())
          .id("d"),
      )
      .into()
  });
  (stage, confirmed, kept)
}

#[test]
fn enter_confirms_a_dialog_from_a_field_of_one_line() {
  let (mut stage, confirmed, kept) = confirming();
  stage.click("name");
  stage.key("Enter");
  assert_eq!((count(&confirmed), count(&kept)), (1, 0));
}

#[test]
fn enter_presses_a_focused_button_and_starts_a_line_in_a_field_of_several_and_does_not_confirm() {
  let (mut stage, confirmed, kept) = confirming();
  stage.click("keep");
  let pressed = count(&kept);
  stage.key("Enter");
  assert_eq!(count(&kept), pressed + 1, "Enter presses the focused button");
  stage.click("notes");
  stage.key("Enter");
  assert_eq!(count(&confirmed), 0, "neither confirms");
}

#[test]
fn a_field_shows_its_error_icon_only_while_invalid() {
  let mut stage = stage(|_| {
    Column::new()
      .size(400.0, 300.0)
      .child(
        Field::new("Email", Input::new(Signal::new(String::new())))
          .error("Enter a full email address.")
          .error_icon(icons::ALERT_CIRCLE)
          .id("email"),
      )
      .child(
        Field::new("Name", Input::new(Signal::new(String::new())))
          .description("Shown in the title bar.")
          .error_icon(icons::ALERT_CIRCLE)
          .id("name"),
      )
      .into()
  });
  let ids = stage.ids();
  assert!(ids.iter().any(|id| id == "email-icon"), "{ids:?}");
  assert!(!ids.iter().any(|id| id == "name-icon"), "{ids:?}");
  let (icon_x, _, icon_width, _) = stage.bounds("email-icon");
  let words = stage.text_quad("Enter a full email address.").expect("the error");
  assert!(icon_x + icon_width <= words.x, "the icon before the words");
}

#[test]
fn a_field_with_its_control_at_the_end_has_its_words_on_the_left() {
  let mut stage = stage(|_| {
    Column::new()
      .size(400.0, 300.0)
      .child(
        Field::new("Sounds", Switch::new("Sounds").label_hidden(true).id("sounds-switch"))
          .description("Play a sound when a run ends.")
          .control_end(true)
          .full_width(true)
          .id("sounds"),
      )
      .into()
  });
  let label = stage.text_quad("Sounds").expect("the label");
  let description = stage
    .text_quad("Play a sound when a run ends.")
    .expect("the description");
  let (switch_x, switch_y, switch_width, switch_height) = stage.bounds("sounds-switch");
  let (field_x, field_y, field_width, field_height) = stage.bounds("sounds");
  assert!((label.x - description.x).abs() < 0.5, "one over the other");
  assert!(description.y > label.y, "the description under the label");
  assert!(
    description.x + description.width <= switch_x,
    "the words before the control"
  );
  assert!(
    ((switch_x + switch_width) - (field_x + field_width)).abs() < 0.5,
    "the control at the row's end"
  );
  let middle = |at: f32, height: f32| at + height / 2.0;
  assert!(
    (middle(switch_y, switch_height) - middle(field_y, field_height)).abs() < 1.0,
    "the control in the middle of the row's height"
  );
}

#[test]
fn a_dialog_puts_its_mark_on_the_title_row_and_the_description_under_both_where_told() {
  let shown = |with_title: bool| {
    let mut stage = stage(move |_| {
      Column::new()
        .size(400.0, 300.0)
        .child(
          Dialog::new(Signal::new(true))
            .title("Run git init?")
            .description("The folder is not a repository yet, and the project needs one.")
            .icon(icons::FOLDER)
            .close_button(true)
            .mark_with_title(with_title)
            .inline(true)
            .id("d"),
        )
        .into()
    });
    let (mark_x, mark_y, _, mark_height) = stage.bounds("d-mark");
    let title = stage.text_quad("Run git init?").expect("the title");
    let description = stage
      .text_quad("The folder is not a repository yet, and the project needs one.")
      .or_else(|| stage.text_quad("The folder is not a repository yet, and the project"))
      .expect("the description");
    (mark_x, mark_y, mark_height, title, description)
  };
  // Beside both, unless told.
  let (mark_x, _, _, title, description) = shown(false);
  assert!(description.x > mark_x + 32.0, "the description after the mark");
  assert!((description.x - title.x).abs() < 0.5);
  // On the title's row: the title level with the mark's middle, the description under them from the mark's start.
  let (mark_x, mark_y, mark_height, title, description) = shown(true);
  let middle = mark_y + mark_height / 2.0;
  assert!(
    (title.y + title.height / 2.0 - middle).abs() < 2.0,
    "level with the mark: {} and {middle}",
    title.y + title.height / 2.0
  );
  assert!(
    (description.x - mark_x).abs() < 0.5,
    "from the mark's start: {}",
    description.x
  );
  assert!(description.y >= mark_y + mark_height, "under the mark");
}

#[test]
fn a_field_puts_an_action_on_its_descriptions_line_after_its_words() {
  use lurq::ui::Link;
  let (pressed, press) = counter();
  let mut stage = stage(move |_| {
    let press = press.clone();
    Column::new()
      .size(400.0, 300.0)
      .padding(8.0)
      .child(
        Field::new("Key", Input::new(Signal::new("KON".to_owned())))
          .warning(true)
          .description("Taken by Kontur ·")
          .description_action(Link::new("Use KON2").on_click(move |_| press()).id("use"))
          .id("key"),
      )
      .into()
  });
  let words = stage.text_quad("Taken by Kontur ·").expect("the words");
  let (link_x, link_y, _, link_height) = stage.bounds("use");
  assert!(
    (link_x - (words.x + words.width) - 6.0).abs() < 0.5,
    "6 after the words: {} and {}",
    link_x,
    words.x + words.width
  );
  assert!(
    (link_y + link_height / 2.0 - (words.y + words.height / 2.0)).abs() < 2.0,
    "on their line"
  );
  stage.click("use");
  assert_eq!(count(&pressed), 1, "and it is pressed");
}
