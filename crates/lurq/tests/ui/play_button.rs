//! A play button: its disc and glyph at each size as the board draws them, the glyph of each state, what a press
//! and a key tell, the ring of the focus from the keyboard, the overlay over a picture in both appearances, a
//! disabled and a loading one taking nothing, what it is to tooling, and a theme's rule and renderer.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App,
    theme::{OpacityLevel, PaletteColor, ThemePalette},
    window::Appearance,
  },
  components::{Column, Row},
  node::{Element, color::Color},
  ui::{
    Button, Case, Interaction, Opacity, PlayButton, PlayButtonPart, PlayButtonRenderer, PlayButtonSize,
    PlayButtonState, PlayButtonVariant, PlayButtonView, icons,
  },
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

/// What was told, in order, and a handler that tells it under `name`.
fn log() -> (
  Arc<Mutex<Vec<(&'static str, bool)>>>,
  impl Fn(&'static str) -> Box<dyn Fn(bool) + Send + Sync>,
) {
  let told = Arc::new(Mutex::new(Vec::new()));
  let tell = told.clone();
  (told, move |name| {
    let tell = tell.clone();
    Box::new(move |playing| tell.lock().unwrap().push((name, playing)))
  })
}

#[test]
fn a_play_button_is_a_round_primary_disc_of_each_size_with_its_glyph() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .padding(8.0)
      .spacing(12.0)
      .child(PlayButton::new().size(PlayButtonSize::Sm).id("sm"))
      .child(PlayButton::new().id("md"))
      .child(PlayButton::new().size(PlayButtonSize::Lg).id("lg"))
      .child(PlayButton::new().size(PlayButtonSize::Xl).id("xl"))
      .into()
  });
  let play = glyph(icons::PLAY);
  let glyphs: Vec<_> = stage
    .quads()
    .into_iter()
    .filter_map(|quad| match &quad.content {
      lurq::layout::quad::QuadContent::Text { text, style, .. } if *text == play => {
        Some((quad.x, style.font_size, style.color))
      }
      _ => None,
    })
    .collect();
  for (id, side, icon) in [
    ("sm", 32.0, 14.0),
    ("md", 40.0, 16.0),
    ("lg", 48.0, 20.0),
    ("xl", 64.0, 28.0),
  ] {
    let (x, _, width, height) = stage.bounds(id);
    assert_eq!((width, height), (side, side), "{id}");
    let fill = stage.fill(id).unwrap_or_else(|| panic!("#{id} is filled"));
    assert_eq!(stage.fill_color(id), Some(light(PaletteColor::Primary)), "{id}");
    assert!(
      fill
        .border_radius
        .is_some_and(|radius| radius.top_left >= side / 2.0 - 0.5),
      "{id}: round"
    );
    // The glyph stands in the middle of the disc, at the size the board gives it, in the primary's foreground.
    let (gx, size, color) = glyphs
      .iter()
      .copied()
      .find(|(gx, ..)| *gx >= x && *gx < x + width)
      .unwrap_or_else(|| panic!("#{id} has the play glyph"));
    assert_eq!(size, icon, "{id}");
    assert!((gx - (x + (side - icon) / 2.0)).abs() < 0.51, "{id}: in the middle");
    assert_eq!(color, light(PaletteColor::PrimaryForeground), "{id}");
  }
}

#[test]
fn each_state_has_its_glyph() {
  let stage = Stage::new(|_| {
    Row::new()
      .padding(8.0)
      .spacing(12.0)
      .child(PlayButton::new().id("paused"))
      .child(PlayButton::new().playing(true).id("playing"))
      .child(PlayButton::new().playing(true).replay(true).id("ended"))
      .child(PlayButton::new().loading(true).id("loading"))
      .child(
        PlayButton::new()
          .playing(true)
          .pause_label("Pause the song")
          .id("named"),
      )
      .into()
  });
  for name in [icons::PLAY, icons::PAUSE, icons::ROTATE_LEFT, icons::SPINNER] {
    assert!(stage.paints(&glyph(name)), "{name}");
  }
}

/// It is a button to tooling, called by what a press would do.
#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn it_is_a_button_called_by_what_a_press_would_do() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .padding(8.0)
      .spacing(12.0)
      .child(PlayButton::new().id("paused"))
      .child(PlayButton::new().playing(true).id("playing"))
      .child(PlayButton::new().playing(true).replay(true).id("ended"))
      .child(PlayButton::new().loading(true).id("loading"))
      .child(
        PlayButton::new()
          .playing(true)
          .pause_label("Pause the song")
          .id("named"),
      )
      .into()
  });
  let mut said = |id: &str, what: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(what))
  };
  assert_eq!(said("paused", "a11y_role").as_deref(), Some("button"));
  assert_eq!(said("paused", "a11y_name").as_deref(), Some("Play"));
  assert_eq!(said("playing", "a11y_name").as_deref(), Some("Pause"));
  assert_eq!(said("ended", "a11y_name").as_deref(), Some("Replay"));
  assert_eq!(said("named", "a11y_name").as_deref(), Some("Pause the song"));
  assert_eq!(said("loading", "a11y_busy").as_deref(), Some("true"));
}

#[test]
fn a_press_tells_what_it_is_to_be_and_a_disabled_or_loading_one_takes_nothing() {
  let (told, tell) = log();
  let mut stage = Stage::new(move |_| {
    Row::new()
      .padding(8.0)
      .spacing(12.0)
      .child(PlayButton::new().id("paused").on_change(tell("paused")))
      .child(PlayButton::new().playing(true).id("playing").on_change(tell("playing")))
      .child(
        PlayButton::new()
          .playing(true)
          .replay(true)
          .id("ended")
          .on_change(tell("ended")),
      )
      .child(PlayButton::new().loading(true).id("loading").on_change(tell("loading")))
      .child(
        PlayButton::new()
          .disabled(true)
          .id("disabled")
          .on_change(tell("disabled")),
      )
      .into()
  });
  for id in ["paused", "playing", "ended", "loading", "disabled"] {
    stage.click(id);
  }
  assert_eq!(
    *told.lock().unwrap(),
    [("paused", true), ("playing", false), ("ended", true)]
  );
}

#[test]
fn tab_draws_the_ring_and_enter_and_space_press_the_button_it_is_on() {
  let (told, tell) = log();
  let mut stage = Stage::new(move |_| {
    Row::new()
      .padding(8.0)
      .spacing(12.0)
      .child(PlayButton::new().id("paused").on_change(tell("paused")))
      .child(
        PlayButton::new()
          .disabled(true)
          .id("disabled")
          .on_change(tell("disabled")),
      )
      .child(PlayButton::new().playing(true).id("playing").on_change(tell("playing")))
      .into()
  });
  assert!(!stage.has_ring(light(PaletteColor::Ring)), "no ring before a key");
  stage.key("Tab");
  assert!(stage.has_ring(light(PaletteColor::Ring)));
  stage.key("Enter");
  // The disabled one is passed by.
  stage.key("Tab");
  stage.key(" ");
  assert_eq!(*told.lock().unwrap(), [("paused", true), ("playing", false)]);
}

#[test]
fn under_the_pointer_it_is_the_primary_hover_color() {
  let mut stage = Stage::new(|_| Row::new().padding(8.0).child(PlayButton::new().id("play")).into());
  stage.hover("play");
  assert_eq!(stage.fill_color("play"), Some(light(PaletteColor::PrimaryHover)));
}

#[test]
fn over_a_picture_it_is_a_dark_disc_with_a_white_glyph_and_a_white_ring_in_both_appearances() {
  for appearance in [Appearance::Light, Appearance::Dark] {
    let app = App::new();
    app.theme().set_preferred_appearance(Some(appearance));
    let mut stage = Stage::in_app(app, |_| {
      Row::new()
        .padding(8.0)
        .spacing(12.0)
        .child(
          PlayButton::new()
            .variant(PlayButtonVariant::Overlay)
            .size(PlayButtonSize::Xl)
            .id("big"),
        )
        .child(
          PlayButton::new()
            .variant(PlayButtonVariant::Overlay)
            .size(PlayButtonSize::Lg)
            .playing(true)
            .force(Some(Interaction::FocusVisible))
            .id("compact"),
        )
        .into()
    });
    stage.pass();
    let white = Color::from_hex("#FFFFFF");
    for (id, side) in [("big", 64.0), ("compact", 48.0)] {
      let (.., width, height) = stage.bounds(id);
      assert_eq!((width, height), (side, side), "{appearance:?} {id}");
      assert_eq!(
        stage.fill_color(id),
        Some(Color::from_hex("#00000099")),
        "{appearance:?} {id}"
      );
    }
    assert_eq!(stage.text(&glyph(icons::PLAY)), (white, 28.0), "{appearance:?}");
    assert_eq!(stage.text(&glyph(icons::PAUSE)), (white, 20.0), "{appearance:?}");
    assert!(stage.has_ring(white), "{appearance:?}: the ring is white");
  }
}

#[test]
fn loading_it_is_at_three_quarters_and_disabled_at_half() {
  let theme = App::new().theme().clone();
  let opacity = |case: Case| {
    theme
      .resolve::<PlayButton>(&case, None)
      .part(PlayButtonPart::Root)
      .rest()
      .opacity
  };
  assert_eq!(opacity(Case::new()), None);
  assert_eq!(
    opacity(Case::new().state(PlayButtonState::Loading, true)),
    Some(Opacity::Value(0.75))
  );
  assert_eq!(
    opacity(Case::new().state(PlayButtonState::Disabled, true)),
    Some(Opacity::Level(OpacityLevel::Disabled))
  );
}

#[test]
fn a_rule_and_a_theme_renderer_change_it() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .padding(8.0)
      .child(
        PlayButton::new()
          .style(PlayButtonPart::Root, |s| s.background(PaletteColor::Destructive))
          .id("styled"),
      )
      .into()
  });
  assert_eq!(stage.fill_color("styled"), Some(light(PaletteColor::Destructive)));

  let app = App::new();
  let renderer: PlayButtonRenderer = Arc::new(|view: PlayButtonView| -> Element {
    let label = format!("{} now", view.label);
    Column::new().child(Button::new(&label).id("drawn")).into()
  });
  app.theme().set_renderer::<PlayButton>(renderer);
  let stage = Stage::in_app(app, |_| Row::new().child(PlayButton::new()).into());
  assert!(stage.paints("Play now"));
}
