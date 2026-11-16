//! A seek bar: what is played, loaded and left on its track and the thumb at the position, over a picture and cut
//! into chapters; what a press, a drag and the keys tell; the time under the pointer and the preview over it; a
//! disabled one, a controlled one, the phone's size, and a theme's other look.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App,
    events::MouseButton,
    theme::{OpacityLevel, PaletteColor, ThemePalette},
  },
  components::{Column, Row},
  core::Signal,
  layout::quad::{Quad, QuadContent},
  node::{color::Color, dimension::Dimension},
  ui::{Case, Interaction, Opacity, SeekBar, SeekBarPart, SeekBarRenderer, SeekBarSize, SeekBarState, SeekBarView},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn color_of(quad: &Quad) -> Color {
  match quad.content {
    QuadContent::Rect { color, .. } => color,
    _ => panic!("a filled rectangle"),
  }
}

/// What tells what it was told, a line each.
#[derive(Clone, Default)]
struct Heard(Arc<Mutex<Vec<String>>>);

impl Heard {
  fn tell(&self, what: impl Into<String>) {
    self.0.lock().unwrap().push(what.into());
  }

  /// What was told since the last look.
  fn take(&self) -> Vec<String> {
    std::mem::take(&mut *self.0.lock().unwrap())
  }
}

/// A scene of a window's size, the bar 320 wide in it, 120 from the top: room for what stands over it.
fn scene(bar: impl Into<lurq::node::Element>) -> lurq::node::Element {
  Column::new()
    .size(480.0, 320.0)
    .padding_horizontal(40.0)
    .padding_top(160.0)
    .child(Column::new().width(320.0).child(bar))
    .into()
}

/// The stretches of the track of the bar `id`, from its start, merged where two side by side have one color:
/// where each starts and ends, from the bar's start, and its color.
fn track(stage: &mut Stage, id: &str) -> Vec<(f32, f32, Color)> {
  let (x, y, width, height) = stage.bounds(id);
  let mut quads: Vec<Quad> = stage
    .quads()
    .into_iter()
    .filter(|quad| {
      matches!(&quad.content, QuadContent::Rect { color, .. } if color.a() > 0)
        && (quad.height - 4.0).abs() < 0.01
        && quad.width > 0.01
        && quad.x >= x - 0.51
        && quad.x + quad.width <= x + width + 0.51
        && quad.y >= y
        && quad.y + quad.height <= y + height
    })
    .collect();
  quads.sort_by(|a, b| a.x.total_cmp(&b.x));
  let mut spans: Vec<(f32, f32, Color)> = Vec::new();
  for quad in quads {
    let (from, to, color) = (quad.x - x, quad.x + quad.width - x, color_of(&quad));
    match spans.last_mut() {
      Some(last) if last.2 == color && (from - last.1).abs() < 0.51 => last.1 = to,
      _ => spans.push((from, to, color)),
    }
  }
  spans
}

/// The thumb of the bar `id`: the filled square of `side` in its box.
fn thumb(stage: &mut Stage, id: &str, side: f32) -> Quad {
  let (x, y, width, height) = stage.bounds(id);
  stage
    .quads()
    .into_iter()
    .find(|quad| {
      matches!(&quad.content, QuadContent::Rect { color, .. } if color.a() > 0)
        && (quad.width, quad.height) == (side, side)
        && quad.x >= x - 0.51
        && quad.y >= y - 0.51
        && quad.x + quad.width <= x + width + 0.51
        && quad.y + quad.height <= y + height + 0.51
    })
    .unwrap_or_else(|| panic!("#{id} should have a thumb of {side}"))
}

fn near(a: f32, b: f32) -> bool {
  (a - b).abs() < 0.6
}

fn assert_spans(spans: &[(f32, f32, Color)], expected: &[(f32, f32, Color)]) {
  assert_eq!(spans.len(), expected.len(), "{spans:?}");
  for (span, want) in spans.iter().zip(expected) {
    assert!(
      near(span.0, want.0) && near(span.1, want.1) && span.2 == want.2,
      "{span:?} should be {want:?}"
    );
  }
}

#[test]
fn a_seek_bar_is_played_then_loaded_then_the_rest_with_the_sliders_thumb_at_the_position() {
  let mut stage = Stage::new(|_| {
    scene(
      SeekBar::new(Signal::new(100.0), 250.0)
        .buffered(150.0)
        .id("bar")
        .build(&lurq::ui::current_theme()),
    )
  });
  let (_, _, width, height) = stage.bounds("bar");
  assert_eq!((width, height), (320.0, 16.0), "as wide as it is given, 16 high");
  // The thumb's middle runs from 8 in to 8 from the end: 304 along. Played up to its middle, at 0.4 of the way;
  // loaded to 0.6 of it.
  let (played, loaded) = (8.0 + 304.0 * 0.4, 8.0 + 304.0 * 0.6);
  assert_spans(
    &track(&mut stage, "bar"),
    &[
      (0.0, played, light(PaletteColor::Primary)),
      (played, loaded, light(PaletteColor::MutedForeground)),
      (loaded, 320.0, light(PaletteColor::Secondary)),
    ],
  );
  let (x, y, ..) = stage.bounds("bar");
  let thumb = thumb(&mut stage, "bar", 16.0);
  assert!(near(thumb.x - x, played - 8.0) && near(thumb.y - y, 0.0));
  assert_eq!(color_of(&thumb), light(PaletteColor::Background));
  let edge = stage
    .quads()
    .into_iter()
    .find_map(|quad| {
      let side = quad.border?.top?;
      (near(quad.x, thumb.x) && quad.width == 16.0).then_some(side)
    })
    .expect("the thumb is edged");
  assert_eq!((edge.width, edge.color), (1.5, light(PaletteColor::Primary)));

  // What is loaded is the muted foreground at half its opacity; the track's ends are round by 2.
  let theme = stage.app.theme().clone();
  let recipe = theme.resolve::<SeekBar>(&Case::new().state(SeekBarState::Buffered, true), None);
  assert_eq!(
    recipe.part(SeekBarPart::Buffered).rest().opacity,
    Some(Opacity::Value(0.5))
  );
  assert!(recipe.part(SeekBarPart::Track).rest().radius.is_some());
}

#[test]
fn over_a_picture_it_is_white_with_a_solid_thumb_of_12() {
  let mut stage = Stage::new(|_| {
    scene(
      SeekBar::new(Signal::new(100.0), 250.0)
        .buffered(150.0)
        .on_media(true)
        .id("bar")
        .build(&lurq::ui::current_theme()),
    )
  });
  // The thumb is 12: its middle runs from 6 in, 308 along.
  let (played, loaded) = (6.0 + 308.0 * 0.4, 6.0 + 308.0 * 0.6);
  assert_spans(
    &track(&mut stage, "bar"),
    &[
      (0.0, played, Color::from_hex("#FFFFFF")),
      (played, loaded, Color::from_hex("#FFFFFF59")),
      (loaded, 320.0, Color::from_hex("#FFFFFF33")),
    ],
  );
  let (x, y, ..) = stage.bounds("bar");
  let thumb = thumb(&mut stage, "bar", 12.0);
  assert!(near(thumb.x - x, played - 6.0) && near(thumb.y - y, 2.0));
  assert_eq!(color_of(&thumb), Color::from_hex("#FFFFFF"));
  // The same in the dark theme: the whites are not the theme's.
  let theme = stage.app.theme().clone();
  let recipe = theme.resolve::<SeekBar>(&Case::new().state(SeekBarState::OnMedia, true), None);
  assert_eq!(
    recipe.part(SeekBarPart::Buffered).rest().opacity,
    Some(Opacity::Value(1.0))
  );
}

#[test]
fn chapters_cut_the_track_2_apart_where_each_starts() {
  let mut stage = Stage::new(|_| {
    scene(
      SeekBar::new(Signal::new(100.0), 250.0)
        .chapters(&[0.0, 50.0, 150.0])
        .id("bar")
        .build(&lurq::ui::current_theme()),
    )
  });
  let (first, second) = (8.0 + 304.0 * 0.2, 8.0 + 304.0 * 0.6);
  // The second chapter holds the thumb: played to its middle, which is half of what that chapter draws.
  let played = first + 1.0 + (second - first - 2.0) * 0.5;
  assert_spans(
    &track(&mut stage, "bar"),
    &[
      (0.0, first - 1.0, light(PaletteColor::Primary)),
      (first + 1.0, played, light(PaletteColor::Primary)),
      (played, second - 1.0, light(PaletteColor::Secondary)),
      (second + 1.0, 320.0, light(PaletteColor::Secondary)),
    ],
  );
}

#[test]
fn the_keys_move_it_by_its_step_and_tell_the_change_and_its_end() {
  let position = Signal::new(100.0_f32);
  let heard = Heard::default();
  let (read, told, ended) = (position.clone(), heard.clone(), heard.clone());
  let mut stage = Stage::new(move |_| {
    let (told, ended) = (told.clone(), ended.clone());
    scene(
      SeekBar::new(read.clone(), 250.0)
        .on_change(move |to| told.tell(format!("change {to}")))
        .on_change_end(move |to| ended.tell(format!("end {to}")))
        .id("bar")
        .build(&lurq::ui::current_theme()),
    )
  });
  stage.key("Tab");
  assert!(
    stage.has_ring(light(PaletteColor::Ring)),
    "the ring of the focus is the thumb's"
  );
  stage.key("ArrowRight");
  assert_eq!(position.get(), 105.0);
  assert_eq!(heard.take(), ["change 105", "end 105"]);
  stage.key("ArrowLeft");
  stage.key("ArrowLeft");
  assert_eq!(position.get(), 95.0);
  heard.take();
  // A page is a tenth of the whole in whole steps: 25 seconds.
  stage.key("PageUp");
  assert_eq!(position.get(), 120.0);
  stage.key("End");
  assert_eq!(position.get(), 250.0);
  assert_eq!(heard.take(), ["change 120", "end 120", "change 250", "end 250"]);
  stage.key("ArrowRight");
  assert!(heard.take().is_empty(), "at the end, nothing moves and nothing is told");
  stage.key("Home");
  assert_eq!(position.get(), 0.0);
  // The thumb is where the keys put it, at the start.
  let (x, ..) = stage.bounds("bar");
  assert!(near(thumb(&mut stage, "bar", 16.0).x, x));
}

#[test]
fn a_press_on_the_track_seeks_there_and_a_drag_follows_the_pointer_to_where_it_is_let_go() {
  let position = Signal::new(0.0_f32);
  let heard = Heard::default();
  let (read, told, ended) = (position.clone(), heard.clone(), heard.clone());
  let mut stage = Stage::new(move |_| {
    let (told, ended) = (told.clone(), ended.clone());
    scene(
      SeekBar::new(read.clone(), 250.0)
        .on_change(move |to| told.tell(format!("change {to}")))
        .on_change_end(move |to| ended.tell(format!("end {to}")))
        .id("bar")
        .build(&lurq::ui::current_theme()),
    )
  });
  let (x, y, ..) = stage.bounds("bar");
  let along = |fraction: f32| (x + 8.0 + 304.0 * fraction, y + 8.0);
  stage.click("bar");
  assert_eq!(position.get(), 125.0, "the middle of the way the thumb's middle goes");
  assert_eq!(heard.take(), ["change 125", "end 125"]);

  let (from, to) = (along(0.5), along(0.75));
  stage.tree.mouse_move(from.0, from.1);
  stage.tree.mouse_down(from.0, from.1, MouseButton::Left);
  stage.tree.mouse_move(to.0, to.1);
  stage.pass();
  assert_eq!(position.get(), 187.5, "a drag is not snapped to the keys' step");
  stage.tree.mouse_up(to.0, to.1, MouseButton::Left);
  stage.pass();
  assert_eq!(heard.take().last().map(String::as_str), Some("end 187.5"));
  assert!(near(thumb(&mut stage, "bar", 16.0).x - x, 304.0 * 0.75));
}

#[test]
fn a_disabled_bar_is_dimmed_shows_nothing_loaded_and_takes_nothing() {
  let (position, after) = (Signal::new(100.0_f32), Signal::new(0.0_f32));
  let heard = Heard::default();
  let (read, read_after, told) = (position.clone(), after.clone(), heard.clone());
  let mut stage = Stage::new(move |_| {
    let told = told.clone();
    Column::new()
      .size(480.0, 320.0)
      .padding(40.0)
      .spacing(24.0)
      .child(
        Column::new().width(320.0).child(
          SeekBar::new(read.clone(), 250.0)
            .buffered(150.0)
            .disabled(true)
            .on_change(move |to| told.tell(format!("change {to}")))
            .on_hover(|_| panic!("a disabled bar hears no pointer"))
            .id("disabled")
            .build(&lurq::ui::current_theme()),
        ),
      )
      .child(
        Column::new().width(320.0).child(
          SeekBar::new(read_after.clone(), 250.0)
            .id("after")
            .build(&lurq::ui::current_theme()),
        ),
      )
      .into()
  });
  assert!(
    track(&mut stage, "disabled")
      .iter()
      .all(|span| span.2 != light(PaletteColor::MutedForeground)),
    "nothing loaded is drawn"
  );
  stage.click("disabled");
  stage.hover("disabled");
  // Tab passes it by, to the next.
  stage.key("Tab");
  stage.key("ArrowRight");
  assert_eq!((position.get(), after.get()), (100.0, 5.0));
  assert!(heard.take().is_empty());

  let theme = stage.app.theme().clone();
  let root = theme
    .resolve::<SeekBar>(&Case::new().state(SeekBarState::Disabled, true), None)
    .part(SeekBarPart::Root)
    .rest();
  assert_eq!(root.opacity, Some(Opacity::Level(OpacityLevel::Disabled)));
  let enabled = theme
    .resolve::<SeekBar>(&Case::new(), None)
    .part(SeekBarPart::Root)
    .rest();
  assert_eq!(enabled.opacity, None);
}

#[test]
fn mounted_it_shows_the_time_under_the_pointer_in_a_chip_4_above_the_bar() {
  let heard = Heard::default();
  let told = heard.clone();
  let mut stage = Stage::new(move |ctx| {
    let told = told.clone();
    scene(
      SeekBar::new(Signal::new(20.0), 250.0)
        .on_hover(move |time| told.tell(format!("{time:?}")))
        .id("bar")
        .mount(ctx),
    )
  });
  let (x, y, ..) = stage.bounds("bar");
  let at = x + 8.0 + 304.0 * 0.4;
  stage.tree.mouse_move(at, y + 8.0);
  stage.pass();
  assert_eq!(heard.take(), ["Some(100.0)"]);
  let text = stage.text_quad("1:40").expect("the time under the pointer");
  assert_eq!(stage.text("1:40"), (light(PaletteColor::PrimaryForeground), 12.0));
  assert!(stage.font_of("1:40").to_lowercase().contains("mono"));
  let chip = stage
    .quads()
    .into_iter()
    .find(|quad| {
      matches!(&quad.content, QuadContent::Rect { color, .. } if *color == light(PaletteColor::Primary))
        && quad.x <= text.x
        && quad.x + quad.width >= text.x + text.width
        && quad.y + quad.height <= y
    })
    .expect("the chip");
  assert!(near(chip.y + chip.height, y - 4.0), "4 above the bar");
  assert!(near(chip.x + chip.width / 2.0, at), "centred on the pointer");
  assert!(near(text.x - chip.x, 10.0), "a tooltip's padding");

  stage.tree.mouse_move(at + 304.0 * 0.1, y + 8.0);
  stage.pass();
  assert!(stage.paints("2:05") && !stage.paints("1:40"), "it follows the pointer");
  stage.tree.mouse_move(x + 160.0, y + 200.0);
  stage.pass();
  assert_eq!(heard.take(), ["Some(125.0)", "None"]);
  assert!(!stage.paints("2:05"), "gone with the pointer");
}

#[test]
fn the_time_shows_wherever_the_pointer_comes_onto_the_bar_its_middle_too() {
  for share in [0.0_f32, 0.25, 0.5, 0.75, 1.0] {
    let mut stage = Stage::new(move |ctx| {
      scene(
        SeekBar::new(Signal::new(20.0), 200.0)
          .preview(|_| Column::new().id("frame").size(128.0, 72.0).into())
          .id("bar")
          .mount(ctx),
      )
    });
    let (x, y, ..) = stage.bounds("bar");
    stage.tree.mouse_move(x + 8.0 + 304.0 * share, y + 8.0);
    stage.pass();
    let time = lurq::ui::media_time_text(f64::from(200.0 * share), false);
    assert!(
      stage.paints(&time),
      "the chip reads {time} with the pointer at {share} of the bar"
    );
    assert!(
      stage.tree.get_element_by_id_mut("frame").is_some(),
      "and the frame is over it at {share}"
    );
  }
}

#[test]
fn the_preview_is_a_frame_of_128_by_72_over_the_chip() {
  let mut stage = Stage::new(move |ctx| {
    scene(
      SeekBar::new(Signal::new(20.0), 250.0)
        .preview(|time| {
          Row::new()
            .id(format!("frame-at-{}", time.round()))
            .width(Dimension::Pct(100.0))
            .height(Dimension::Pct(100.0))
            .background(PaletteColor::Destructive)
            .into()
        })
        .id("bar")
        .mount(ctx),
    )
  });
  let (x, y, ..) = stage.bounds("bar");
  stage.tree.mouse_move(x + 8.0 + 304.0 * 0.4, y + 8.0);
  stage.pass();
  let (_, frame_y, width, height) = stage.bounds("frame-at-100");
  assert_eq!((width, height), (128.0, 72.0));
  let chip = stage.text_quad("1:40").expect("the chip under the frame");
  // The chip is 6 under the frame, and the two 6 over the bar.
  assert!(frame_y + 72.0 < chip.y);
  let edge = stage
    .quads()
    .into_iter()
    .find_map(|quad| {
      let side = quad.border?.top?;
      ((quad.width, quad.height) == (128.0, 72.0)).then_some(side)
    })
    .expect("the frame is edged");
  assert_eq!((edge.width, edge.color), (2.0, Color::from_hex("#FFFFFF")));
  let theme = stage.app.theme().clone();
  let hover = theme
    .resolve::<SeekBar>(&Case::new().state(SeekBarState::Preview, true), None)
    .part(SeekBarPart::Hover)
    .rest();
  assert!(hover.gap.is_some() && hover.padding_bottom.is_some());
  assert!(
    y - (frame_y + 72.0) > 30.0,
    "the chip stands between the frame and the bar"
  );
}

#[test]
fn the_still_form_is_moved_and_reads_nothing_of_a_pointer_that_does_not_press_it() {
  let heard = Heard::default();
  let told = heard.clone();
  let position = Signal::new(20.0_f32);
  let read = position.clone();
  let mut stage = Stage::new(move |_| {
    let told = told.clone();
    scene(
      SeekBar::new(read.clone(), 250.0)
        .on_hover(move |time| told.tell(format!("{time:?}")))
        .id("bar")
        .build(&lurq::ui::current_theme()),
    )
  });
  let (x, y, ..) = stage.bounds("bar");
  stage.tree.mouse_move(x + 8.0 + 304.0 * 0.4, y + 8.0);
  stage.pass();
  assert!(heard.take().is_empty(), "the time under the pointer is a mounted bar's");
  assert!(!stage.paints("1:40"));
  // It is still dragged and moved by the keys.
  stage.click("bar");
  assert_eq!(position.get(), 125.0);
  stage.key("Tab");
  stage.key("ArrowRight");
  stage.key("ArrowRight");
  assert_eq!(
    position.get(),
    135.0,
    "the focus stays on it over the renders its changes make"
  );
}

#[test]
fn the_chip_goes_with_a_drag_to_the_time_it_is_dragged_to() {
  let heard = Heard::default();
  let told = heard.clone();
  let position = Signal::new(20.0_f32);
  let bound = position.clone();
  let mut stage = Stage::new(move |ctx| {
    let told = told.clone();
    scene(
      SeekBar::new(bound.clone(), 250.0)
        .on_hover(move |time| told.tell(format!("{time:?}")))
        .id("bar")
        .mount(ctx),
    )
  });
  let (x, y, ..) = stage.bounds("bar");
  let at = |share: f32| x + 8.0 + 304.0 * share;
  stage.tree.mouse_move(at(0.4), y + 8.0);
  stage.tree.mouse_down(at(0.4), y + 8.0, MouseButton::Left);
  stage.pass();
  assert!(stage.paints("1:40"), "under the pointer where it is pressed");
  // The runtime gives the moves of a drag to the slider alone: the bar hears of them as changes.
  stage.tree.mouse_move(at(0.8), y + 8.0);
  stage.pass();
  assert_eq!(position.get(), 200.0);
  assert!(stage.paints("3:20"), "the chip is over the time it is dragged to");
  assert!(!stage.paints("1:40"), "and not left where it was pressed");
  stage.tree.mouse_up(at(0.8), y + 8.0, MouseButton::Left);
  stage.pass();
  assert_eq!(heard.take(), ["Some(100.0)", "Some(200.0)"]);
  // Let go, the keys move the position and not the chip: the pointer has not moved.
  stage.key("ArrowRight");
  assert_eq!(position.get(), 205.0);
  assert!(stage.paints("3:20"));
  assert!(heard.take().is_empty());
}

#[test]
fn mounted_and_drawn_hovered_it_shows_its_position_in_the_chip_over_it() {
  let mut stage = Stage::new(|ctx| {
    scene(
      SeekBar::new(Signal::new(100.0), 250.0)
        .force(Some(Interaction::Hovered))
        .id("bar")
        .mount(ctx),
    )
  });
  stage.pass();
  let (x, ..) = stage.bounds("bar");
  let text = stage.text_quad("1:40").expect("the chip shows the position");
  assert!(
    near(text.x + text.width / 2.0, x + 8.0 + 304.0 * 0.4),
    "over the thumb, from the first frame"
  );
  // The still form shows no chip.
  let mut still = Stage::new(|_| {
    scene(
      SeekBar::new(Signal::new(100.0), 250.0)
        .force(Some(Interaction::Hovered))
        .id("bar")
        .build(&lurq::ui::current_theme()),
    )
  });
  still.pass();
  assert!(!still.paints("1:40"));
}

#[test]
fn a_controlled_bar_tells_where_it_is_moved_and_shows_what_the_app_answers() {
  let held = Signal::new(50.0_f32);
  let refuse = Arc::new(Mutex::new(false));
  let heard = Heard::default();
  let (value, no, told) = (held.clone(), refuse.clone(), heard.clone());
  let mut stage = Stage::new(move |_| {
    let (value, no, told) = (value.clone(), no.clone(), told.clone());
    scene(
      SeekBar::controlled(value.get(), 250.0, move |to| {
        told.tell(format!("change {to}"));
        if !*no.lock().unwrap() {
          value.set(to as f32);
        }
      })
      .id("bar")
      .build(&lurq::ui::current_theme()),
    )
  });
  stage.click("bar");
  assert_eq!(held.get(), 125.0);
  *refuse.lock().unwrap() = true;
  stage.key("Tab");
  stage.key("ArrowRight");
  assert_eq!(heard.take(), ["change 125", "change 130"]);
  assert_eq!(held.get(), 125.0, "the app kept its own");
  let (x, ..) = stage.bounds("bar");
  assert!(
    near(thumb(&mut stage, "bar", 16.0).x - x, 304.0 * 0.5),
    "the app's position is shown"
  );
}

#[test]
fn on_a_phone_it_is_28_high_with_a_thumb_of_28_and_16_over_a_picture() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .size(480.0, 320.0)
      .padding(40.0)
      .spacing(24.0)
      .child(
        Column::new().width(320.0).child(
          SeekBar::new(Signal::new(100.0), 250.0)
            .size(SeekBarSize::Lg)
            .id("surface")
            .build(&lurq::ui::current_theme()),
        ),
      )
      .child(
        Column::new().width(320.0).child(
          SeekBar::new(Signal::new(100.0), 250.0)
            .size(SeekBarSize::Lg)
            .on_media(true)
            .id("media")
            .build(&lurq::ui::current_theme()),
        ),
      )
      .into()
  });
  assert_eq!(stage.bounds("surface").3, 28.0);
  let thumb_quad = thumb(&mut stage, "surface", 28.0);
  assert_eq!(color_of(&thumb_quad), light(PaletteColor::Background));
  assert_eq!(stage.bounds("media").3, 16.0);
  thumb(&mut stage, "media", 16.0);
}

#[test]
fn a_theme_restyles_its_parts_and_gives_it_another_anatomy_that_still_seeks() {
  let position = Signal::new(100.0_f32);
  let read = position.clone();
  let app = App::new();
  // A bar that reads its position beside the track: other nodes, the same behavior.
  let renderer: SeekBarRenderer = Arc::new(|view: SeekBarView| {
    let time = view.time_text(view.position);
    Row::new()
      .width(Dimension::Pct(100.0))
      .spacing(8.0)
      .align_items(lurq::layout::Alignment::Center)
      .child(
        Row::new()
          .flex(1.0)
          .min_width(0.0)
          .height(16.0)
          .child(view.dressed(view.control())),
      )
      .child(view.part(SeekBarPart::Tip).text(&time).nowrap())
      .into()
  });
  app.theme().set_renderer::<SeekBar>(renderer);
  let mut stage = Stage::in_app(app, move |_| {
    scene(
      SeekBar::new(read.clone(), 250.0)
        .style(SeekBarPart::Thumb, |s| s.background(PaletteColor::Destructive))
        .id("bar")
        .build(&lurq::ui::current_theme()),
    )
  });
  assert!(stage.paints("1:40"));
  assert_eq!(
    color_of(&thumb(&mut stage, "bar", 16.0)),
    light(PaletteColor::Destructive)
  );
  stage.key("Tab");
  stage.key("ArrowRight");
  assert_eq!(position.get(), 105.0);
  assert!(stage.paints("1:45"));
}

#[test]
fn it_grows_to_the_room_of_a_row_beside_what_is_in_it_and_fills_a_column() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .size(480.0, 320.0)
      .padding(40.0)
      .spacing(24.0)
      .child(
        Row::new()
          .width(320.0)
          .spacing(10.0)
          .align_items(lurq::layout::Alignment::Center)
          .child(Row::new().size(32.0, 32.0).id("before"))
          .child(
            SeekBar::new(Signal::new(100.0), 250.0)
              .id("in-row")
              .build(&lurq::ui::current_theme()),
          )
          .child(lurq::ui::MediaTime::new(100.0, Some(250.0)).id("after")),
      )
      .child(
        Column::new().width(300.0).child(
          SeekBar::new(Signal::new(100.0), 250.0)
            .id("in-column")
            .build(&lurq::ui::current_theme()),
        ),
      )
      .into()
  });
  let (row_x, _, row_width, _) = stage.bounds("in-row");
  let (after_x, _, after_width, _) = stage.bounds("after");
  assert!(near(row_x, 40.0 + 32.0 + 10.0), "after what stands before it");
  assert!(
    near(after_x, row_x + row_width + 10.0),
    "the time keeps its place after it"
  );
  assert!(
    near(after_x + after_width, 40.0 + 320.0),
    "together they fill the row: {row_width}"
  );
  assert!(stage.text_quad("1:40").is_some(), "the time keeps its room");
  assert_eq!(stage.bounds("in-column").2, 300.0);
}

#[test]
fn empty_it_is_the_track_alone_and_takes_nothing() {
  let (position, after) = (Signal::new(100.0_f32), Signal::new(0.0_f32));
  let heard = Heard::default();
  let (read, read_after, told) = (position.clone(), after.clone(), heard.clone());
  let mut stage = Stage::new(move |_| {
    let told = told.clone();
    Column::new()
      .size(480.0, 320.0)
      .padding(40.0)
      .spacing(24.0)
      .child(
        Column::new().width(320.0).child(
          SeekBar::new(read.clone(), 250.0)
            .buffered(150.0)
            .empty(true)
            .on_change(move |to| told.tell(format!("change {to}")))
            .id("empty")
            .build(&lurq::ui::current_theme()),
        ),
      )
      .child(
        Column::new().width(320.0).child(
          SeekBar::new(read_after.clone(), 250.0)
            .id("after")
            .build(&lurq::ui::current_theme()),
        ),
      )
      .into()
  });
  assert_spans(
    &track(&mut stage, "empty"),
    &[(0.0, 320.0, light(PaletteColor::Secondary))],
  );
  let (x, y, width, height) = stage.bounds("empty");
  let thumbs = stage.quads().into_iter().any(|quad| {
    matches!(&quad.content, QuadContent::Rect { color, .. } if color.a() > 0)
      && quad.height > 4.0
      && quad.x >= x - 0.5
      && quad.x + quad.width <= x + width + 0.5
      && quad.y >= y - 0.5
      && quad.y + quad.height <= y + height + 0.5
  });
  assert!(!thumbs, "no thumb");
  stage.click("empty");
  stage.key("Tab");
  stage.key("ArrowRight");
  assert_eq!(
    (position.get(), after.get()),
    (100.0, 5.0),
    "not a stop of Tab: the next is"
  );
  assert!(heard.take().is_empty());
  // It is not dimmed: nothing is loaded, it is not disabled.
  let theme = stage.app.theme().clone();
  let root = theme
    .resolve::<SeekBar>(&Case::new().state(SeekBarState::Empty, true), None)
    .part(SeekBarPart::Root)
    .rest();
  assert_eq!(root.opacity, None);
}

#[test]
fn live_it_is_full_with_the_thumb_at_its_end_and_tells_nothing() {
  let heard = Heard::default();
  let (told, ended) = (heard.clone(), heard.clone());
  let mut stage = Stage::new(move |_| {
    let (told, ended) = (told.clone(), ended.clone());
    Column::new()
      .size(480.0, 320.0)
      .padding(40.0)
      .spacing(24.0)
      .child(
        Column::new().width(320.0).child(
          SeekBar::controlled(0.0, 0.0, move |to| told.tell(format!("change {to}")))
            .on_change_end(move |to| ended.tell(format!("end {to}")))
            .on_media(true)
            .live(true)
            .id("live")
            .build(&lurq::ui::current_theme()),
        ),
      )
      .child(
        Column::new().width(320.0).child(
          SeekBar::new(Signal::new(100.0), 250.0)
            .live(true)
            .id("themed")
            .build(&lurq::ui::current_theme()),
        ),
      )
      .into()
  });
  assert_spans(&track(&mut stage, "live"), &[(0.0, 320.0, Color::from_hex("#FFFFFF"))]);
  let (x, ..) = stage.bounds("live");
  assert!(
    near(thumb(&mut stage, "live", 12.0).x - x, 308.0),
    "the thumb at the end"
  );
  assert_spans(
    &track(&mut stage, "themed"),
    &[(0.0, 320.0, light(PaletteColor::Primary))],
  );
  let (x, ..) = stage.bounds("themed");
  assert!(near(thumb(&mut stage, "themed", 16.0).x - x, 304.0));

  stage.click("live");
  let (x, y, ..) = stage.bounds("live");
  stage.tree.mouse_move(x + 20.0, y + 8.0);
  stage.tree.mouse_down(x + 20.0, y + 8.0, MouseButton::Left);
  stage.tree.mouse_move(x + 100.0, y + 8.0);
  stage.tree.mouse_up(x + 100.0, y + 8.0, MouseButton::Left);
  stage.pass();
  stage.key("Tab");
  stage.key("Home");
  assert!(heard.take().is_empty(), "a press, a drag and the keys tell nothing");
  assert!(
    near(thumb(&mut stage, "live", 12.0).x - x, 308.0),
    "and it stays at the end"
  );
}

#[test]
fn given_to_a_tree_as_it_is_it_is_its_component_with_the_chip_and_a_drag_that_holds() {
  let heard = Heard::default();
  let told = heard.clone();
  let position = Signal::new(20.0_f32);
  let bound = position.clone();
  // No `mount` and no `Ctx`: the bar is a child as any element is.
  let mut stage = Stage::new(move |_| {
    let told = told.clone();
    scene(
      SeekBar::new(bound.clone(), 250.0)
        .on_hover(move |time| told.tell(format!("{time:?}")))
        .id("bar"),
    )
  });
  let (x, y, ..) = stage.bounds("bar");
  let at = |share: f32| x + 8.0 + 304.0 * share;
  stage.tree.mouse_move(at(0.4), y + 8.0);
  stage.pass();
  assert_eq!(heard.take(), ["Some(100.0)"]);
  assert!(stage.paints("1:40"), "the time under the pointer is kept between renders");

  // A drag holds over the renders its own changes make.
  stage.tree.mouse_down(at(0.4), y + 8.0, MouseButton::Left);
  stage.pass();
  stage.tree.mouse_move(at(0.6), y + 8.0);
  stage.pass();
  assert_eq!(position.get(), 150.0);
  stage.tree.mouse_move(at(0.8), y + 8.0);
  stage.pass();
  assert_eq!(position.get(), 200.0, "the second move of the drag is still the bar's");
  assert!(stage.paints("3:20"), "and the chip is over the time it is dragged to");
  stage.tree.mouse_up(at(0.8), y + 8.0, MouseButton::Left);
  stage.pass();
  stage.key("ArrowRight");
  assert_eq!(position.get(), 205.0, "the focus stayed on it");
}
