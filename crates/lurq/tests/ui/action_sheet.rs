//! The design system's action sheet: a phone's choice among a few actions, risen from the bottom of the screen.

use std::sync::{Arc, Mutex};

use lurq::{
  app::theme::{PaletteColor, ThemePalette},
  components::Column,
  core::Signal,
  node::color::Color,
  ui::{ActionSheet, MenuItem},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// A phone's screen of 390 by 844 with the board's sheet open on it: a title, three options, the last one that
/// destroys something.
fn on_a_phone(told: Arc<Mutex<Vec<String>>>) -> (Stage, Signal<bool>) {
  let open = Signal::new(true);
  let shown = open.clone();
  let mut stage = Stage::first_frame(390.0, 844.0, move |_| {
    let tell = |what: &'static str| {
      let told = told.clone();
      move || told.lock().unwrap().push(what.to_owned())
    };
    Column::new()
      .size(390.0, 844.0)
      .child(
        ActionSheet::new(
          shown.clone(),
          [
            MenuItem::new("Save to library").on_select(tell("save")),
            MenuItem::new("Duplicate").on_select(tell("duplicate")),
            MenuItem::new("Delete photo")
              .destructive(true)
              .on_select(tell("delete")),
          ],
        )
        .title("Photo options")
        .on_dismiss(tell("dismissed"))
        .id("sheet"),
      )
      .into()
  });
  stage.pass();
  (stage, open)
}

#[test]
fn it_is_the_boards_sheet_at_the_bottom_of_the_screen() {
  let (mut stage, _) = on_a_phone(Arc::default());
  // 8 in from the sides and the bottom; Cancel under the options, 8 apart.
  let (x, _, width, height) = stage.bounds("sheet-cancel");
  assert_eq!((x, width, height), (8.0, 374.0, 56.0));
  let (_, cancel_y, ..) = stage.bounds("sheet-cancel");
  assert_eq!(cancel_y + 56.0, 844.0 - 8.0);
  let (option_x, option_y, option_width, option_height) = stage.bounds("sheet-option-2");
  assert_eq!((option_x, option_width, option_height), (8.0, 374.0, 56.0));
  assert_eq!(option_y + 56.0 + 8.0, cancel_y);
  assert_eq!(stage.bounds("sheet-option-1").1 + 56.0, option_y);

  let (color, size) = stage.text("Photo options");
  assert_eq!((color, size), (light(PaletteColor::MutedForeground), 13.0));
  let (color, size) = stage.text("Save to library");
  assert_eq!((color, size), (light(PaletteColor::Foreground), 17.0));
  assert_eq!(stage.text("Delete photo").0, light(PaletteColor::Destructive));
  assert_eq!(stage.text("Cancel").1, 17.0);
}

#[test]
fn an_option_does_what_it_does_and_closes_the_sheet() {
  let told = Arc::new(Mutex::new(Vec::new()));
  let (mut stage, open) = on_a_phone(told.clone());
  stage.click("sheet-option-1");
  assert!(!open.get());
  assert_eq!(*told.lock().unwrap(), ["duplicate"]);
}

#[test]
fn cancel_the_scrim_and_escape_close_it_with_no_option() {
  let told = Arc::new(Mutex::new(Vec::new()));
  let (mut stage, open) = on_a_phone(told.clone());
  stage.click("sheet-cancel");
  assert!(!open.get());

  open.set(true);
  stage.pass();
  stage.tree.mouse_move(195.0, 100.0);
  stage
    .tree
    .mouse_down(195.0, 100.0, lurq::app::events::MouseButton::Left);
  stage.tree.mouse_up(195.0, 100.0, lurq::app::events::MouseButton::Left);
  stage.pass();
  assert!(!open.get(), "a press on the scrim");

  open.set(true);
  stage.pass();
  stage.key("Escape");
  assert!(!open.get());
  assert_eq!(told.lock().unwrap().len(), 3, "told it was dismissed each time");
  assert!(told.lock().unwrap().iter().all(|what| what == "dismissed"));
}
