//! Where the focus goes as a dialog opens (`Button::autofocus`, `Dialog::initial_focus`), key caps in the app's
//! own words with ids of their own (`Dialog::note_key_worded`, part `note_key_cap`), and an element of the note
//! that wraps (`Dialog::note_child`).

use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::ctx::Ctx,
  components::{Column, Text},
  core::Signal,
  node::Element,
  ui::{Button, Dialog, DialogFocus},
};

use super::stage::Stage;

fn stage(scene: impl Fn(&mut Ctx) -> Element + Send + Sync + 'static) -> Stage {
  let mut stage = Stage::new(scene);
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

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

fn has(stage: &mut Stage, id: &str) -> bool {
  stage
    .tree
    .get_element_by_id_mut(id)
    .and_then(|element| element.bounds())
    .is_some()
}

/// A button that opens a dialog of Cancel and Delete, Delete marked to take the focus where `autofocus`, the dialog
/// given `focus`, and confirming with a count of its own.
fn delete_dialog(
  open: &Signal<bool>,
  autofocus: bool,
  focus: DialogFocus,
) -> (Stage, Arc<AtomicUsize>, Arc<AtomicUsize>, Arc<AtomicUsize>) {
  let ((cancels, cancel), (deletes, delete), (confirms, confirm)) = (counter(), counter(), counter());
  let held = open.clone();
  let stage = stage(move |_| {
    let (show, cancel, delete, confirm) = (held.clone(), cancel.clone(), delete.clone(), confirm.clone());
    Column::new()
      .size(400.0, 300.0)
      .child(Button::new("Delete run…").id("open").on_click(move |_| show.set(true)))
      .child(
        Dialog::new(held.clone())
          .title("Delete run?")
          .initial_focus(focus)
          .on_confirm(confirm)
          .footer(Button::new("Cancel").id("cancel").on_click(move |_| cancel()))
          .footer(
            Button::new("Delete")
              .id("delete")
              .autofocus(autofocus)
              .on_click(move |_| delete()),
          )
          .id("d"),
      )
      .into()
  });
  (stage, cancels, deletes, confirms)
}

#[test]
fn a_button_marked_autofocus_takes_the_focus_as_the_dialog_opens_and_enter_presses_it() {
  let open = Signal::new(false);
  let (mut stage, cancels, deletes, _) = delete_dialog(&open, true, DialogFocus::FirstControl);
  stage.click("open");
  assert!(open.get());
  stage.pass();
  assert_eq!(focused(&stage).as_deref(), Some("delete"), "the main action, not Cancel");
  stage.key("Enter");
  assert_eq!((count(&cancels), count(&deletes)), (0, 1));
}

#[test]
fn a_dialog_that_focuses_its_panel_confirms_with_enter_and_presses_no_button() {
  let open = Signal::new(false);
  let (mut stage, cancels, deletes, confirms) = delete_dialog(&open, false, DialogFocus::Panel);
  stage.click("open");
  stage.pass();
  assert_eq!(focused(&stage).as_deref(), Some("d"), "the panel has the focus");
  stage.key("Enter");
  assert_eq!(
    (count(&cancels), count(&deletes), count(&confirms)),
    (0, 0, 1),
    "Enter confirms"
  );
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("cancel"), "Tab goes on to the first control");
}

#[test]
fn by_default_the_first_control_takes_the_focus() {
  let open = Signal::new(false);
  let (mut stage, cancels, ..) = delete_dialog(&open, false, DialogFocus::FirstControl);
  stage.click("open");
  stage.pass();
  assert_eq!(focused(&stage).as_deref(), Some("cancel"));
  stage.key("Enter");
  assert_eq!(count(&cancels), 1);
}

#[test]
fn key_caps_say_the_apps_own_words_and_have_ids_of_their_own() {
  let mut stage = stage(|_| {
    Dialog::new(Signal::new(true))
      .title("Delete run?")
      .note_key_worded("Введення", "видаляє")
      .note_key("Esc", "cancels")
      .footer(Button::new("Delete").id("delete"))
      .inline(true)
      .id("d")
      .into()
  });
  assert!(stage.paints("Введення"), "the cap says the app's word as it is");
  assert!(has(&mut stage, "d-note_key_cap"), "the first cap");
  assert!(has(&mut stage, "d-note_key_cap-2"), "the second");
  let (cap_x, _, cap_width, _) = stage.bounds("d-note_key_cap");
  let words = stage.text_quad("видаляє").expect("the words after the cap");
  assert!(words.x >= cap_x + cap_width, "the words after their cap");
}

#[test]
fn an_element_of_the_note_wraps_where_the_note_is_short() {
  let reason = "Runs that are still going cannot be deleted until they stop or are cancelled by a person";
  let mut stage = stage(move |_| {
    Dialog::new(Signal::new(true))
      .title("Delete run?")
      .note_child(Text::new(reason).id("reason"))
      .footer(Button::new("Delete").id("delete"))
      .width(360.0)
      .inline(true)
      .id("d")
      .into()
  });
  let (note_x, _, note_width, _) = stage.bounds("d-note");
  let (reason_x, _, reason_width, reason_height) = stage.bounds("reason");
  let (delete_x, ..) = stage.bounds("delete");
  assert!(reason_x + reason_width <= note_x + note_width + 0.5, "inside the note");
  assert!(reason_x + reason_width <= delete_x, "before the buttons");
  let line = stage.text_quad("Delete run?").map_or(20.0, |title| title.height);
  assert!(reason_height > line * 1.5, "on several lines: {reason_height}");
}
