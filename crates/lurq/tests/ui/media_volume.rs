//! A volume control: the button and the slider as the board lays them out, the glyph of each level, muting that
//! keeps the volume, what a press, a key and the slider tell, the slider taking the sound back on, the ring of the
//! focus from the keyboard, a disabled one taking nothing, the white form over a picture, the collapsed form that
//! shows its slider under the pointer and with the focus, and a theme's rule.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App,
    theme::{PaletteColor, ThemePalette},
    window::Appearance,
  },
  components::{Column, Row},
  core::Signal,
  layout::quad::{Quad, QuadContent},
  node::color::Color,
  ui::{Button, Interaction, MediaVolume, MediaVolumePart, MediaVolumeSize, icons},
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

fn color_of(quad: &Quad) -> Color {
  match quad.content {
    QuadContent::Rect { color, .. } => color,
    _ => panic!("a filled rectangle"),
  }
}

/// The filled rectangles of a height painted inside the element with `id`.
fn filled(stage: &mut Stage, id: &str, height: f32) -> Vec<Quad> {
  let (x, y, width, outer) = stage.bounds(id);
  stage
    .quads()
    .into_iter()
    .filter(|quad| {
      matches!(&quad.content, QuadContent::Rect { color, .. } if color.a() > 0)
        && (quad.height - height).abs() < 0.01
        && quad.x >= x - 0.51
        && quad.y >= y - 0.51
        && quad.x + quad.width <= x + width + 0.51
        && quad.y + quad.height <= y + outer + 0.51
    })
    .collect()
}

/// A volume control in a row of its own with `id`, as wide and as high as the control.
fn holder(id: &str, control: impl Into<lurq::node::Element>) -> Row {
  Row::new().id(id).child(control)
}

#[test]
fn a_volume_control_is_a_ghost_button_and_a_slider_72_long_4_apart() {
  let volume = Signal::new(0.8_f32);
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .spacing(12.0)
      .child(holder(
        "md",
        MediaVolume::new(volume.clone())
          .id("mute")
          .build(&lurq::ui::current_theme()),
      ))
      .child(holder(
        "lg",
        MediaVolume::new(volume.clone())
          .size(MediaVolumeSize::Lg)
          .id("big")
          .build(&lurq::ui::current_theme()),
      ))
      .child(holder(
        "long",
        MediaVolume::new(volume.clone())
          .length(96.0)
          .build(&lurq::ui::current_theme()),
      ))
      .into()
  });
  let (x, _, width, height) = stage.bounds("mute");
  assert_eq!((width, height), (32.0, 32.0));
  let (hx, _, whole, _) = stage.bounds("md");
  assert_eq!(hx, x);
  assert_eq!(whole, 32.0 + 4.0 + 72.0, "the button, 4, and the slider");
  // Ghost: no fill at rest. The glyph is the loud one, 16, in the foreground.
  assert_eq!(stage.fill_color("mute"), None);
  assert_eq!(
    stage.text(&glyph(icons::VOLUME)),
    (light(PaletteColor::Foreground), 16.0)
  );

  let (.., width, height) = stage.bounds("big");
  assert_eq!((width, height), (36.0, 36.0), "lg: a button of 36");
  let (.., whole, _) = stage.bounds("lg");
  assert_eq!(whole, 36.0 + 4.0 + 72.0);
  let twenty = stage.quads().into_iter().any(|quad| {
    matches!(&quad.content, QuadContent::Text { text, style, .. } if *text == glyph(icons::VOLUME) && style.font_size == 20.0)
  });
  assert!(twenty, "lg: a glyph of 20");
  let (.., whole, _) = stage.bounds("long");
  assert_eq!(whole, 32.0 + 4.0 + 96.0, "a length of its own");

  // The slider's range is the primary color up to the volume, on a track of the secondary.
  let rails = filled(&mut stage, "md", 4.0);
  assert!(rails.iter().any(|quad| color_of(quad) == light(PaletteColor::Primary)));
  assert!(
    rails
      .iter()
      .any(|quad| color_of(quad) == light(PaletteColor::Secondary))
  );
}

#[test]
fn the_glyph_is_loud_from_a_half_low_under_it_and_crossed_while_muted_or_at_nothing() {
  for (volume, muted, name) in [
    (0.8, false, icons::VOLUME),
    (0.5, false, icons::VOLUME),
    (0.3, false, icons::VOLUME_LOW),
    (0.0, false, icons::VOLUME_OFF),
    (0.8, true, icons::VOLUME_OFF),
  ] {
    let stage = Stage::new(move |_| {
      Row::new()
        .padding(8.0)
        .child(
          MediaVolume::controlled(volume, |_| {})
            .muted_controlled(muted, |_| {})
            .build(&lurq::ui::current_theme()),
        )
        .into()
    });
    assert!(stage.paints(&glyph(name)), "{volume} {muted}: {name}");
  }
}

#[test]
fn muted_the_slider_stands_at_nothing_and_the_volume_is_kept() {
  let (volume, muted) = (Signal::new(0.8_f32), Signal::new(true));
  let (held, quiet) = (volume.clone(), muted.clone());
  let mut stage = Stage::new(move |_| {
    Row::new()
      .padding(8.0)
      .child(holder(
        "volume",
        MediaVolume::new(held.clone())
          .muted(quiet.clone())
          .id("mute")
          .build(&lurq::ui::current_theme()),
      ))
      .into()
  });
  // The thumb is at the start of the slider, and nothing of the range is painted.
  let (x, ..) = stage.bounds("mute");
  let slider_x = x + 32.0 + 4.0;
  let thumb = filled(&mut stage, "volume", 16.0)
    .into_iter()
    .find(|quad| quad.width == 16.0)
    .expect("a thumb");
  assert!((thumb.x - slider_x).abs() < 0.51, "at nothing: {} {slider_x}", thumb.x);
  assert_eq!(volume.get(), 0.8, "the app's volume is not touched");

  // Unmuted, it stands at the volume again.
  stage.click("mute");
  assert!(!muted.get());
  assert_eq!(volume.get(), 0.8);
  let thumb = filled(&mut stage, "volume", 16.0)
    .into_iter()
    .find(|quad| quad.width == 16.0)
    .expect("a thumb");
  assert!(thumb.x > slider_x + 40.0, "at 0.8 of 72: {}", thumb.x - slider_x);
}

#[test]
fn a_press_of_the_button_tells_the_other_state() {
  let told = Arc::new(Mutex::new(Vec::new()));
  let tell = told.clone();
  let muted = Signal::new(false);
  let held = muted.clone();
  let mut stage = Stage::new(move |_| {
    let tell = tell.clone();
    Row::new()
      .padding(8.0)
      .child(
        MediaVolume::new(Signal::new(0.6))
          .muted(held.clone())
          .on_muted_change(move |next| tell.lock().unwrap().push(next))
          .id("mute")
          .build(&lurq::ui::current_theme()),
      )
      .into()
  });
  stage.click("mute");
  assert!(muted.get(), "the signal is set first");
  stage.click("mute");
  assert!(!muted.get());
  assert_eq!(*told.lock().unwrap(), [true, false]);
}

/// The button is called by what a press would do, and the slider by the control's label.
#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn the_button_is_named_by_what_a_press_would_do() {
  let muted = Signal::new(false);
  let held = muted.clone();
  let mut stage = Stage::new(move |_| {
    Row::new()
      .padding(8.0)
      .child(
        MediaVolume::new(Signal::new(0.6))
          .muted(held.clone())
          .id("mute")
          .build(&lurq::ui::current_theme()),
      )
      .into()
  });
  let name = |stage: &mut Stage| {
    stage
      .tree
      .get_element_by_id_mut("mute")
      .and_then(|element| element.description("a11y_name"))
  };
  assert_eq!(name(&mut stage).as_deref(), Some("Mute"));
  stage.click("mute");
  assert_eq!(name(&mut stage).as_deref(), Some("Unmute"));
}

#[test]
fn the_keyboard_draws_the_ring_presses_the_button_and_moves_the_slider() {
  let (volume, muted) = (Signal::new(0.5_f32), Signal::new(false));
  let changes = Arc::new(Mutex::new(Vec::new()));
  let (held, quiet, tell) = (volume.clone(), muted.clone(), changes.clone());
  let mut stage = Stage::new(move |_| {
    let tell = tell.clone();
    Row::new()
      .padding(8.0)
      .child(
        MediaVolume::new(held.clone())
          .muted(quiet.clone())
          .on_change(move |to| tell.lock().unwrap().push(to))
          .build(&lurq::ui::current_theme()),
      )
      .into()
  });
  stage.key("Tab");
  assert!(stage.has_ring(light(PaletteColor::Ring)), "the button's ring");
  stage.key("Enter");
  assert!(muted.get());
  stage.key(" ");
  assert!(!muted.get());
  // Tab goes on to the slider; Right moves it a step of a twentieth.
  stage.key("Tab");
  stage.key("ArrowRight");
  assert!((volume.get() - 0.55).abs() < 1e-4, "{}", volume.get());
  assert_eq!(*changes.lock().unwrap(), [0.55]);
}

#[test]
fn moving_the_slider_up_from_nothing_while_muted_turns_the_sound_back_on() {
  let (volume, muted) = (Signal::new(0.7_f32), Signal::new(true));
  let told = Arc::new(Mutex::new(Vec::new()));
  let (held, quiet, tell) = (volume.clone(), muted.clone(), told.clone());
  let mut stage = Stage::new(move |_| {
    let (volume_told, muted_told) = (tell.clone(), tell.clone());
    Row::new()
      .padding(8.0)
      .child(
        MediaVolume::new(held.clone())
          .muted(quiet.clone())
          .on_change(move |to| volume_told.lock().unwrap().push(format!("on_change({to})")))
          .on_muted_change(move |next| muted_told.lock().unwrap().push(format!("on_muted_change({next})")))
          .build(&lurq::ui::current_theme()),
      )
      .into()
  });
  stage.key("Tab");
  stage.key("Tab");
  // The slider stands at nothing: Right takes it to the first step, which is the new volume, and the sound is back.
  stage.key("ArrowRight");
  assert!((volume.get() - 0.05).abs() < 1e-4, "{}", volume.get());
  assert!(!muted.get());
  assert_eq!(*told.lock().unwrap(), ["on_change(0.05)", "on_muted_change(false)"]);
  // Unmuted, it goes on from there.
  stage.key("ArrowRight");
  assert!((volume.get() - 0.1).abs() < 1e-4, "{}", volume.get());
  assert_eq!(told.lock().unwrap().len(), 3);
}

#[test]
fn a_disabled_control_takes_nothing() {
  let (volume, muted) = (Signal::new(0.5_f32), Signal::new(false));
  let (held, quiet) = (volume.clone(), muted.clone());
  let mut stage = Stage::new(move |_| {
    Row::new()
      .padding(8.0)
      .child(
        MediaVolume::new(held.clone())
          .muted(quiet.clone())
          .disabled(true)
          .id("mute")
          .build(&lurq::ui::current_theme()),
      )
      .child(Button::new("After").id("after"))
      .into()
  });
  stage.click("mute");
  assert!(!muted.get());
  // Tab passes the button and the slider by, to what comes after.
  stage.key("Tab");
  stage.key("ArrowRight");
  stage.key("ArrowLeft");
  assert_eq!(volume.get(), 0.5);
  assert!(
    stage.has_ring(light(PaletteColor::Ring)),
    "the ring is on what comes after"
  );
}

#[test]
fn over_a_picture_it_is_white_in_both_appearances() {
  for appearance in [Appearance::Light, Appearance::Dark] {
    let app = App::new();
    app.theme().set_preferred_appearance(Some(appearance));
    let mut stage = Stage::in_app(app, |_| {
      Column::new()
        .padding(8.0)
        .spacing(12.0)
        .child(holder(
          "volume",
          MediaVolume::new(Signal::new(0.5))
            .on_media(true)
            .id("mute")
            .build(&lurq::ui::current_theme()),
        ))
        .child(
          MediaVolume::new(Signal::new(0.5))
            .on_media(true)
            .force(Some(Interaction::FocusVisible))
            .build(&lurq::ui::current_theme()),
        )
        .into()
    });
    let white = Color::from_hex("#FFFFFF");
    assert_eq!(stage.text(&glyph(icons::VOLUME)).0, white, "{appearance:?}: the glyph");
    let rails = filled(&mut stage, "volume", 4.0);
    assert!(
      rails.iter().any(|quad| color_of(quad) == white),
      "{appearance:?}: the range"
    );
    assert!(
      rails.iter().any(|quad| color_of(quad) == Color::from_hex("#FFFFFF33")),
      "{appearance:?}: the track"
    );
    let thumb = filled(&mut stage, "volume", 12.0)
      .into_iter()
      .find(|quad| quad.width == 12.0)
      .unwrap_or_else(|| panic!("{appearance:?}: a thumb of 12"));
    assert_eq!(color_of(&thumb), white);
    assert!(stage.has_ring(white), "{appearance:?}: a white ring");

    stage.hover("mute");
    assert_eq!(
      stage.fill_color("mute"),
      Some(Color::from_hex("#FFFFFF1F")),
      "{appearance:?}: hover"
    );
  }
}

#[test]
fn collapsed_it_is_the_button_alone_and_the_slider_comes_with_the_pointer_and_the_focus() {
  let volume = Signal::new(0.5_f32);
  let mut stage = Stage::new(move |ctx| {
    let control = MediaVolume::new(volume.clone()).collapsed(true).id("mute").mount(ctx);
    Row::new()
      .padding(8.0)
      .spacing(40.0)
      .child(Row::new().id("volume").child(control))
      .child(Button::new("After").id("after"))
      .into()
  });
  let width = |stage: &mut Stage| stage.bounds("volume").2;
  assert_eq!(width(&mut stage), 32.0, "the button alone");

  stage.hover("mute");
  assert_eq!(width(&mut stage), 32.0 + 4.0 + 72.0, "under the pointer");
  stage.tree.mouse_move(600.0, 300.0);
  stage.pass();
  assert_eq!(width(&mut stage), 32.0, "the pointer has gone");

  // The focus on the button shows the slider, Tab goes on into it, and out of the control it goes again.
  stage.key("Tab");
  assert_eq!(width(&mut stage), 108.0, "the focus on the button");
  stage.key("Tab");
  assert_eq!(width(&mut stage), 108.0, "the focus on the slider");
  stage.key("Tab");
  assert_eq!(width(&mut stage), 32.0, "the focus has gone on");

  // Converted into an element on its own it is the still form: the button alone.
  let mut still = Stage::new(|_| {
    Row::new()
      .child(
        Row::new().id("volume").child(
          MediaVolume::new(Signal::new(0.5))
            .collapsed(true)
            .build(&lurq::ui::current_theme()),
        ),
      )
      .into()
  });
  assert_eq!(still.bounds("volume").2, 32.0);
}

#[test]
fn a_rule_lays_over_the_recipe() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .padding(8.0)
      .child(holder(
        "volume",
        MediaVolume::new(Signal::new(0.5))
          .style(MediaVolumePart::Slider, |s| s.width(120.0))
          .build(&lurq::ui::current_theme()),
      ))
      .into()
  });
  assert_eq!(stage.bounds("volume").2, 32.0 + 4.0 + 120.0);
}
