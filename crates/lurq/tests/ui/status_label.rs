//! `ui::StatusLabel`: an icon and toned words with no box, its icon turning while it spins.

use lurq::{
  app::theme::{PaletteColor, ThemePalette},
  components::Row,
  node::color::Color,
  ui::{StatusLabel, StatusLabelSize, StatusLabelVariant, icons},
};

use super::stage::Stage;

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

#[test]
fn a_status_label_is_an_icon_and_words_in_its_tone_with_no_box() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .child(
        StatusLabel::new("Completed")
          .icon(icons::CHECK_CIRCLE)
          .variant(StatusLabelVariant::Success)
          .id("done"),
      )
      .into()
  });
  assert_eq!(stage.text("Completed"), (light(PaletteColor::Success), 12.0));
  assert_eq!(
    stage.text(&glyph(icons::CHECK_CIRCLE)),
    (light(PaletteColor::Success), 14.0)
  );
  assert_eq!(stage.fill_color("done"), None, "no fill");
  assert!(stage.border("done").is_none(), "no edge");
  let (icon_x, _, icon_width, _) = stage.bounds("done-icon");
  let words = stage.text_quad("Completed").unwrap();
  assert!((words.x - (icon_x + icon_width) - 6.0).abs() < 0.01, "6 apart");
}

#[test]
fn each_tone_is_its_strong_color_and_small_is_smaller() {
  let stage = Stage::new(|_| {
    Row::new()
      .child(StatusLabel::new("Running"))
      .child(StatusLabel::new("Queued").variant(StatusLabelVariant::Muted))
      .child(StatusLabel::new("Paused").variant(StatusLabelVariant::Warning))
      .child(
        StatusLabel::new("Failed")
          .variant(StatusLabelVariant::Destructive)
          .size(StatusLabelSize::Sm),
      )
      .into()
  });
  assert_eq!(stage.text("Running").0, light(PaletteColor::Foreground));
  assert_eq!(stage.text("Queued").0, light(PaletteColor::MutedForeground));
  assert_eq!(stage.text("Paused").0, light(PaletteColor::Warning));
  assert_eq!(stage.text("Failed"), (light(PaletteColor::Destructive), 11.0));
}

#[test]
fn a_spinning_label_turns_its_icon_about_its_middle() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .child(StatusLabel::new("Running").icon(icons::SPINNER).spin(true).id("run"))
      .into()
  });
  let spinner = glyph(icons::SPINNER);
  stage.pass_after(lurq::ui::SPIN_PERIOD / 4);
  let quarter = stage.text_quad(&spinner).unwrap().transform;
  stage.pass_after(lurq::ui::SPIN_PERIOD / 2);
  let half = stage.text_quad(&spinner).unwrap().transform;
  assert_ne!(quarter, half, "the icon goes on turning");
}

#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn to_tooling_it_is_a_status_called_by_its_words() {
  let mut stage = Stage::new(|_| Row::new().child(StatusLabel::new("Failed").id("s")).into());
  let mut said = |key: &str| {
    stage
      .tree
      .get_element_by_id_mut("s")
      .and_then(|element| element.description(key))
  };
  assert_eq!(said("a11y_role").as_deref(), Some("status"));
  assert_eq!(said("a11y_name").as_deref(), Some("Failed"));
}
