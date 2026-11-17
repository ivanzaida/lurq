//! A signature pad: the board's 280 by 124 with its line and its words, a stroke of the mouse and of a finger or a
//! pen with its pressure, the stroke being drawn, Undo and Clear by the keys and the buttons and where the focus goes,
//! the app's signature drawn back and scaled, a picture of it, read only, disabled, invalid, a field, a long
//! placeholder, what tooling reads, and a theme's rules and renderer.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App,
    events::{MouseButton, TouchPhase},
    runtime::TouchRoute,
    theme::{PaletteColor, ThemePalette},
  },
  canvas::CanvasHandle,
  components::Column,
  core::Signal,
  layout::quad::QuadContent,
  node::color::Color,
  ui::{
    Field, Interaction, Signature, SignaturePad, SignaturePadPart, SignaturePadRenderer, SignaturePadState,
    SignaturePadView, SignaturePoint, SignatureStroke,
  },
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// A stage of `pad`, given the id "pad", 8 in from the window's corner, its canvas drawn on the CPU.
fn scene(pad: impl Fn() -> SignaturePad + Send + Sync + 'static) -> Stage {
  let mut stage = Stage::software(move |_| {
    Column::new()
      .size(400.0, 300.0)
      .padding(8.0)
      .child(pad().id("pad"))
      .into()
  });
  stage.tree.resize(400, 300);
  stage.pass();
  stage
}

/// A pad bound to `signature` that writes down every signature it tells, with what the signal held then.
fn told(signature: &Signal<Signature>) -> (Stage, Arc<Mutex<Vec<(Signature, Signature)>>>) {
  let heard = Arc::new(Mutex::new(Vec::new()));
  let (read, write) = (signature.clone(), heard.clone());
  let stage = scene(move || {
    let (seen, write) = (read.clone(), write.clone());
    SignaturePad::new(read.clone())
      .on_change(move |signature| write.lock().unwrap().push((signature.clone(), seen.get_untracked())))
  });
  (stage, heard)
}

/// Drags the mouse through `points`, from the top left of the pad, and lets it go at the last; one frame after each
/// move, as a window paints.
fn draw(stage: &mut Stage, points: &[(f32, f32)]) {
  let (x0, y0, ..) = stage.bounds("pad");
  let (x, y) = points[0];
  stage.tree.mouse_move(x0 + x, y0 + y);
  stage.tree.mouse_down(x0 + x, y0 + y, MouseButton::Left);
  stage.frame();
  for &(x, y) in &points[1..] {
    stage.tree.mouse_move(x0 + x, y0 + y);
    stage.frame();
  }
  let (x, y) = points[points.len() - 1];
  stage.tree.mouse_up(x0 + x, y0 + y, MouseButton::Left);
  stage.pass();
}

/// The pad's canvas.
fn canvas(stage: &mut Stage) -> CanvasHandle {
  stage
    .tree
    .get_element_by_id("pad-surface")
    .and_then(|element| element.as_canvas())
    .expect("the pad's surface is a canvas")
}

/// The pixel of the pad's canvas at `(x, y)` from its top left: red, green, blue and alpha.
fn pixel(stage: &mut Stage, x: f32, y: f32) -> [u8; 4] {
  let canvas = canvas(stage);
  let shot = canvas.snapshot().try_take().unwrap().unwrap();
  let (x, y) = (x.round() as usize, y.round() as usize);
  let at = (y * shot.width as usize + x) * 4;
  [shot.rgba[at], shot.rgba[at + 1], shot.rgba[at + 2], shot.rgba[at + 3]]
}

/// Whether the canvas has ink at `(x, y)`, or within a pixel of it.
fn inked(stage: &mut Stage, x: f32, y: f32) -> bool {
  [(0.0, 0.0), (1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)]
    .iter()
    .any(|(dx, dy)| pixel(stage, x + dx, y + dy)[3] > 100)
}

/// A signature of one stroke across a pad of the board's size.
fn one_stroke() -> Signature {
  let mut signature = Signature::new(280.0, 124.0);
  signature.strokes.push(SignatureStroke::new([
    SignaturePoint::new(20.0, 50.0),
    SignaturePoint::new(60.0, 50.0),
    SignaturePoint::new(100.0, 50.0),
  ]));
  signature
}

#[test]
fn at_rest_it_is_the_boards_pad_with_its_line_and_its_words() {
  let mut stage = scene(|| SignaturePad::controlled(Signature::default(), |_| {}));
  let (x, y, width, height) = stage.bounds("pad");
  assert_eq!((width, height), (280.0, 124.0));
  let border = stage.border("pad").expect("an edge");
  let edge = border.border.unwrap().top.unwrap();
  assert_eq!((edge.width, edge.color), (1.0, light(PaletteColor::Input)));
  assert_eq!(stage.fill_color("pad"), Some(light(PaletteColor::Background)));
  // The words, mono 11 in the muted color, 12 above the bottom, 16 in.
  let (color, size) = stage.text("Sign above the line");
  assert_eq!((color, size), (light(PaletteColor::MutedForeground), 11.0));
  assert!(stage.font_of("Sign above the line").to_lowercase().contains("mono"));
  let words = stage.text_quad("Sign above the line").unwrap();
  assert!((words.x - (x + 16.0)).abs() < 0.5);
  assert!((words.y + words.height - (y + height - 12.0)).abs() < 1.0);
  // The line, 1 high in the input's color, across the pad 16 in from each side and 6 over the words.
  let line = stage
    .quads()
    .into_iter()
    .find(|quad| {
      quad.height == 1.0
        && matches!(quad.content, QuadContent::Rect { color, .. } if color == light(PaletteColor::Input))
    })
    .expect("the line");
  assert!((line.x - (x + 16.0)).abs() < 0.5 && (line.width - 248.0).abs() < 0.5);
  assert!((words.y - (line.y + 1.0 + 6.0)).abs() < 1.0, "{} {}", words.y, line.y);
  // Nothing is signed: no buttons.
  assert!(stage.tree.get_element_by_id("pad-undo").is_none());
  assert!(stage.tree.get_element_by_id("pad-clear").is_none());
}

#[test]
fn a_stroke_of_the_mouse_is_told_once_let_go_with_its_points_and_their_times() {
  let signature = Signal::new(Signature::default());
  let (mut stage, heard) = told(&signature);
  let (x0, y0, ..) = stage.bounds("pad");
  stage.tree.mouse_move(x0 + 30.0, y0 + 40.0);
  stage.tree.mouse_down(x0 + 30.0, y0 + 40.0, MouseButton::Left);
  stage.frame();
  for step in 1..=10 {
    stage
      .tree
      .mouse_move(x0 + 30.0 + step as f32 * 10.0, y0 + 40.0 + step as f32 * 2.0);
    stage.frame();
  }
  // While it is drawn the stroke is shown, and nothing is told.
  assert!(heard.lock().unwrap().is_empty());
  assert!(signature.get().is_empty());
  assert!(inked(&mut stage, 80.0, 50.0), "the stroke being drawn is shown");
  stage.tree.mouse_up(x0 + 130.0, y0 + 60.0, MouseButton::Left);
  stage.pass();
  let first = heard.lock().unwrap().clone();
  assert_eq!(first.len(), 1, "told once, when the stroke is let go");
  let (told, held) = &first[0];
  assert_eq!(told, held, "the signal is set before the handlers are told");
  assert_eq!(told.strokes.len(), 1);
  assert_eq!(
    (told.width, told.height),
    (280.0, 124.0),
    "the size of the pad it was signed on"
  );
  let points = &told.strokes[0].points;
  assert_eq!(points.len(), 11);
  assert_eq!((points[0].x, points[0].y), (30.0, 40.0), "from the pad's top left");
  assert_eq!((points[10].x, points[10].y), (130.0, 60.0));
  assert!(
    points.iter().all(|point| point.pressure.is_none()),
    "a mouse measures no pressure"
  );
  assert_eq!(points[0].time, 0);
  assert!(points.windows(2).all(|pair| pair[0].time <= pair[1].time));
  // Drawn as the signature, in the foreground's color.
  assert!(inked(&mut stage, 80.0, 50.0));
  let ink = pixel(&mut stage, 30.0, 40.0);
  let foreground = light(PaletteColor::Foreground);
  assert_eq!(
    (ink[0], ink[1], ink[2]),
    (foreground.r(), foreground.g(), foreground.b())
  );
  assert!(!inked(&mut stage, 200.0, 100.0));
  // The app setting the signal tells nobody, and is drawn.
  signature.set(Signature::default());
  stage.pass();
  assert_eq!(heard.lock().unwrap().len(), 1);
  assert!(!inked(&mut stage, 80.0, 50.0));
}

#[test]
fn a_press_that_does_not_move_is_a_dot_and_jitter_is_no_point() {
  let signature = Signal::new(Signature::default());
  let (mut stage, _) = told(&signature);
  draw(&mut stage, &[(50.0, 50.0), (50.2, 50.3), (50.0, 50.0)]);
  let held = signature.get();
  assert_eq!(
    held.strokes[0].points.len(),
    1,
    "moves of less than a pixel are not points"
  );
  assert!(inked(&mut stage, 50.0, 50.0), "a dot");
}

#[test]
fn a_stroke_past_the_pad_s_edge_stays_on_it() {
  let signature = Signal::new(Signature::default());
  let (mut stage, _) = told(&signature);
  draw(&mut stage, &[(200.0, 60.0), (300.0, 60.0), (350.0, -20.0)]);
  let points = &signature.get().strokes[0].points;
  assert_eq!((points[1].x, points[1].y), (280.0, 60.0));
  assert_eq!((points[2].x, points[2].y), (280.0, 0.0));
}

#[test]
fn a_finger_or_a_pen_draws_with_its_pressure_and_is_taken_from_scrolling() {
  let signature = Signal::new(Signature::default());
  let (mut stage, heard) = told(&signature);
  let (x0, y0, ..) = stage.bounds("pad");
  let route = stage
    .tree
    .touch_event(7, TouchPhase::Start, x0 + 20.0, y0 + 30.0, Some(0.25));
  assert_eq!(route, TouchRoute::Taken, "the pad takes the finger");
  stage.frame();
  stage
    .tree
    .touch_event(7, TouchPhase::Move, x0 + 60.0, y0 + 40.0, Some(0.5));
  // A second finger meanwhile draws nothing.
  stage
    .tree
    .touch_event(8, TouchPhase::Start, x0 + 200.0, y0 + 80.0, Some(0.9));
  stage
    .tree
    .touch_event(8, TouchPhase::Move, x0 + 220.0, y0 + 90.0, Some(0.9));
  stage.tree.touch_event(8, TouchPhase::End, x0 + 220.0, y0 + 90.0, None);
  stage.frame();
  assert!(heard.lock().unwrap().is_empty());
  stage
    .tree
    .touch_event(7, TouchPhase::End, x0 + 60.0, y0 + 40.0, Some(0.5));
  stage.pass();
  let held = signature.get();
  assert_eq!(held.strokes.len(), 1);
  let pressures: Vec<Option<f32>> = held.strokes[0].points.iter().map(|point| point.pressure).collect();
  assert_eq!(pressures, [Some(0.25), Some(0.5)]);
  assert_eq!(heard.lock().unwrap().len(), 1);
  // A touch the system takes away signs nothing.
  stage.tree.touch_event(9, TouchPhase::Start, x0 + 20.0, y0 + 80.0, None);
  stage.tree.touch_event(9, TouchPhase::Move, x0 + 80.0, y0 + 80.0, None);
  stage
    .tree
    .touch_event(9, TouchPhase::Cancel, x0 + 80.0, y0 + 80.0, None);
  stage.pass();
  assert_eq!(signature.get().strokes.len(), 1);
  assert_eq!(heard.lock().unwrap().len(), 1);
}

#[test]
fn undo_and_clear_by_the_buttons_and_the_focus_goes_to_the_pad_when_they_are_gone() {
  let signature = Signal::new(Signature::default());
  let (mut stage, heard) = told(&signature);
  draw(&mut stage, &[(20.0, 40.0), (60.0, 40.0)]);
  draw(&mut stage, &[(20.0, 70.0), (60.0, 70.0)]);
  draw(&mut stage, &[(120.0, 70.0), (160.0, 70.0)]);
  assert_eq!(signature.get().strokes.len(), 3);
  // The buttons are in the pad's top corner, 6 in, the last at its end.
  let (x, y, width, _) = stage.bounds("pad");
  let (cx, cy, cw, _) = stage.bounds("pad-clear");
  assert!((cx + cw - (x + width - 6.0)).abs() < 0.5 && (cy - (y + 6.0)).abs() < 0.5);
  stage.click("pad-undo");
  assert_eq!(signature.get().strokes.len(), 2);
  assert!(!inked(&mut stage, 140.0, 70.0), "the last stroke is gone");
  assert!(inked(&mut stage, 40.0, 70.0));
  stage.click("pad-clear");
  assert!(signature.get().is_empty());
  assert_eq!(heard.lock().unwrap().len(), 5, "three strokes, Undo and Clear");
  assert!(!inked(&mut stage, 40.0, 40.0));
  assert!(stage.tree.get_element_by_id("pad-clear").is_none());
  let focused = stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned));
  assert_eq!(focused.as_deref(), Some("pad"), "the pad has the focus its buttons had");
  // A press of a button draws nothing under it.
  draw(&mut stage, &[(20.0, 40.0), (60.0, 40.0)]);
  stage.click("pad-undo");
  assert!(signature.get().is_empty());
  assert_eq!(
    stage
      .tree
      .focused_element()
      .and_then(|element| element.id().map(str::to_owned))
      .as_deref(),
    Some("pad"),
    "Undo of the last stroke gives the focus to the pad"
  );
}

#[test]
fn the_pad_is_a_stop_of_tab_then_its_buttons_and_its_keys_take_strokes_back() {
  let mut three = one_stroke();
  three.strokes.push(one_stroke().strokes[0].clone());
  three.strokes.push(one_stroke().strokes[0].clone());
  let signature = Signal::new(three);
  let (mut stage, _) = told(&signature);
  stage.key("Tab");
  let focused = |stage: &Stage| {
    stage
      .tree
      .focused_element()
      .and_then(|element| element.id().map(str::to_owned))
  };
  assert_eq!(focused(&stage).as_deref(), Some("pad"));
  assert!(stage.has_ring(light(PaletteColor::Ring)), "the ring is around the pad");
  stage.key("Backspace");
  assert_eq!(signature.get().strokes.len(), 2);
  stage.tree.key_down("z".into(), "KeyZ".into(), false, true, false);
  stage.tree.key_up("z".into(), "KeyZ".into(), false, true, false);
  stage.pass();
  assert_eq!(signature.get().strokes.len(), 1, "Ctrl+Z takes back a stroke");
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("pad-undo"));
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("pad-clear"));
  stage.key("Enter");
  assert!(signature.get().is_empty());
  assert_eq!(focused(&stage).as_deref(), Some("pad"));
  draw(&mut stage, &[(20.0, 40.0), (60.0, 40.0)]);
  stage.key("Delete");
  assert!(signature.get().is_empty(), "Delete clears");
}

#[test]
fn the_app_s_signature_is_drawn_scaled_to_the_pad_and_a_stroke_joins_it_in_its_own_pixels() {
  // Signed on a pad twice as large: drawn at half its size.
  let mut large = Signature::new(560.0, 248.0);
  large.strokes.push(SignatureStroke::new([
    SignaturePoint::new(40.0, 100.0),
    SignaturePoint::new(200.0, 100.0),
  ]));
  let signature = Signal::new(large);
  let (mut stage, _) = told(&signature);
  assert!(inked(&mut stage, 60.0, 50.0), "half of (120, 100)");
  assert!(!inked(&mut stage, 150.0, 100.0));
  draw(&mut stage, &[(20.0, 20.0), (40.0, 20.0)]);
  let held = signature.get();
  assert_eq!((held.width, held.height), (560.0, 248.0), "it keeps its own size");
  assert_eq!((held.strokes[1].points[0].x, held.strokes[1].points[0].y), (40.0, 40.0));
}

#[test]
fn a_controlled_pad_shows_what_the_app_answers() {
  let held = Signal::new(Signature::default());
  let asked = Arc::new(Mutex::new(0));
  let (read, count) = (held.clone(), asked.clone());
  // The app takes the first stroke and no more.
  let mut stage = scene(move || {
    let (set, count) = (read.clone(), count.clone());
    SignaturePad::controlled(read.get(), move |signature| {
      *count.lock().unwrap() += 1;
      if signature.strokes.len() <= 1 {
        set.set(signature.clone());
      }
    })
  });
  draw(&mut stage, &[(20.0, 40.0), (60.0, 40.0)]);
  draw(&mut stage, &[(20.0, 90.0), (60.0, 90.0)]);
  assert_eq!(*asked.lock().unwrap(), 2);
  assert_eq!(held.get().strokes.len(), 1);
  assert!(inked(&mut stage, 40.0, 40.0));
  assert!(!inked(&mut stage, 40.0, 90.0), "what the app holds, not what was drawn");
}

#[test]
fn read_only_and_disabled_take_nothing_and_have_no_buttons() {
  for (read_only, disabled) in [(true, false), (false, true)] {
    let signature = Signal::new(one_stroke());
    let read = signature.clone();
    let mut stage = scene(move || SignaturePad::new(read.clone()).read_only(read_only).disabled(disabled));
    assert!(inked(&mut stage, 60.0, 50.0), "the signature is shown");
    assert!(stage.tree.get_element_by_id("pad-undo").is_none());
    draw(&mut stage, &[(20.0, 90.0), (100.0, 90.0)]);
    stage.key("Tab");
    stage.key("Delete");
    assert_eq!(
      signature.get(),
      one_stroke(),
      "read only {read_only}, disabled {disabled}"
    );
  }
  let stage = scene(|| SignaturePad::controlled(Signature::default(), |_| {}).disabled(true));
  let theme = stage.app.theme().clone();
  let root = theme
    .resolve::<SignaturePad>(&lurq::ui::Case::new().state(SignaturePadState::Disabled, true), None)
    .part(SignaturePadPart::Root)
    .rest();
  assert_eq!(
    root.opacity,
    Some(lurq::ui::Opacity::Level(lurq::app::theme::OpacityLevel::Disabled))
  );
}

#[test]
fn invalid_and_in_a_field_with_an_error_its_edge_is_destructive() {
  let mut stage = scene(|| SignaturePad::controlled(Signature::default(), |_| {}).invalid(true));
  let edge = stage.border("pad").unwrap().border.unwrap().top.unwrap();
  assert_eq!(edge.color, light(PaletteColor::Destructive));
  let stage = Stage::software(|_| {
    Column::new()
      .padding(8.0)
      .child(
        Field::new(
          "Signature",
          SignaturePad::controlled(Signature::default(), |_| {}).id("pad"),
        )
        .error("Sign to accept."),
      )
      .into()
  });
  let destructive = stage.quads().iter().any(|quad| {
    quad
      .border
      .and_then(|border| border.top)
      .is_some_and(|side| side.color == light(PaletteColor::Destructive) && side.width == 1.0)
      && quad.width == 280.0
  });
  assert!(destructive);
}

#[test]
fn a_placeholder_longer_than_the_pad_ends_in_an_ellipsis() {
  let long = "Sign above the line with the name you use on every document you have ever signed";
  let stage = scene(move || SignaturePad::controlled(Signature::default(), |_| {}).placeholder(long));
  let words = stage.cut_text_quad(long).expect("the words, cut");
  assert!(words.x + words.width <= 8.0 + 280.0 - 16.0 + 0.5);
}

#[test]
fn a_full_width_pad_and_its_size_props() {
  let mut stage = Stage::software(|_| {
    Column::new()
      .width(500.0)
      .child(
        SignaturePad::controlled(Signature::default(), |_| {})
          .full_width(true)
          .id("wide"),
      )
      .child(
        SignaturePad::controlled(Signature::default(), |_| {})
          .width(320.0)
          .height(160.0)
          .id("sized"),
      )
      .into()
  });
  assert_eq!(stage.bounds("wide").2, 500.0);
  let (_, _, width, height) = stage.bounds("sized");
  assert_eq!((width, height), (320.0, 160.0));
}

#[test]
fn a_signature_is_a_picture_of_its_own_size_drawn_on_nothing() {
  let signature = one_stroke();
  let image = signature
    .to_image(2.0, Color::new(0, 0, 255, 255), 2.0)
    .expect("a picture");
  assert_eq!((image.width(), image.height()), (560, 248));
  let at = |x: usize, y: usize| image.data()[(y * 560 + x) * 4..][..4].to_vec();
  assert_eq!(at(120, 100), [0, 0, 255, 255], "the stroke, at twice its place");
  assert_eq!(at(400, 200)[3], 0, "and nothing under it");
  assert!(
    Signature::default()
      .to_image(1.0, Color::new(0, 0, 0, 255), 2.0)
      .is_none()
  );
}

#[test]
fn a_signature_is_a_png_of_straight_rgba_that_reads_back() {
  // Ink at half its opacity: a PNG holds the color as it is and its alpha beside it, not the color times the alpha.
  let bytes = one_stroke()
    .to_png(2.0, Color::new(0, 0, 255, 128), 2.0)
    .expect("a PNG");
  assert_eq!(
    &bytes[..8],
    [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a],
    "the signature of a PNG"
  );
  let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
    .read_info()
    .expect("a PNG that reads back");
  let mut pixels = vec![0; reader.output_buffer_size().expect("a size")];
  let info = reader.next_frame(&mut pixels).expect("its pixels");
  assert_eq!((info.width, info.height), (560, 248));
  assert_eq!(
    (info.color_type, info.bit_depth),
    (png::ColorType::Rgba, png::BitDepth::Eight)
  );
  let at = |x: usize, y: usize| pixels[(y * 560 + x) * 4..][..4].to_vec();
  let inked = at(120, 100);
  assert_eq!(
    &inked[..3],
    [0, 0, 255],
    "the ink's own color, not darkened by its alpha"
  );
  assert!((i32::from(inked[3]) - 128).abs() <= 1, "{inked:?}");
  assert_eq!(at(400, 200)[3], 0, "and nothing under it");
  assert!(
    Signature::default()
      .to_png(1.0, Color::new(0, 0, 0, 255), 2.0)
      .is_none()
  );
}

#[test]
fn a_forced_focus_draws_the_ring_with_nothing_pointing_at_it() {
  let stage = scene(|| SignaturePad::controlled(Signature::default(), |_| {}).force(Some(Interaction::FocusVisible)));
  assert!(stage.has_ring(light(PaletteColor::Ring)));
}

#[test]
fn a_theme_restyles_it_and_gives_it_another_anatomy() {
  const RED: Color = Color::new(200, 30, 30, 255);
  let app = App::new();
  app
    .theme()
    .patch_recipe::<SignaturePad>(|rules| rules.base(SignaturePadPart::Ink, |s| s.background(RED)));
  let mut stage = Stage::software_in(app, |_| {
    Column::new()
      .padding(8.0)
      .child(SignaturePad::controlled(one_stroke(), |_| {}).id("pad"))
      .into()
  });
  let ink = pixel(&mut stage, 60.0, 50.0);
  assert_eq!((ink[0], ink[1], ink[2]), (200, 30, 30));

  let app = App::new();
  let renderer: SignaturePadRenderer = Arc::new(|view: SignaturePadView| {
    let words = format!("{} strokes", view.signature.strokes.len());
    view
      .root()
      .child(view.part(SignaturePadPart::Placeholder).text(&words))
      .into()
  });
  app.theme().set_renderer::<SignaturePad>(renderer);
  let stage = Stage::in_app(app, |_| {
    Column::new()
      .padding(8.0)
      .child(SignaturePad::controlled(one_stroke(), |_| {}))
      .into()
  });
  assert!(stage.paints("1 strokes"));
}

/// To tooling the pad is a picture called by its label that says whether it is signed.
#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn tooling_reads_a_picture_named_by_its_label_signed_or_not() {
  let signature = Signal::new(Signature::default());
  let read = signature.clone();
  let mut stage = scene(move || {
    SignaturePad::new(read.clone())
      .label("Signature of the recipient")
      .signed_text("Signed by the recipient")
  });
  let said = |stage: &mut Stage, id: &str, name: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(name))
  };
  assert_eq!(said(&mut stage, "pad", "a11y_role").as_deref(), Some("img"));
  assert_eq!(
    said(&mut stage, "pad", "a11y_name").as_deref(),
    Some("Signature of the recipient")
  );
  assert_eq!(
    said(&mut stage, "pad", "a11y_description").as_deref(),
    Some("Not signed")
  );
  signature.set(one_stroke());
  stage.pass();
  assert_eq!(
    said(&mut stage, "pad", "a11y_description").as_deref(),
    Some("Signed by the recipient")
  );
  assert_eq!(said(&mut stage, "pad-undo", "a11y_name").as_deref(), Some("Undo"));
  assert_eq!(said(&mut stage, "pad-clear", "a11y_name").as_deref(), Some("Clear"));
  let mut shown = scene(|| SignaturePad::controlled(one_stroke(), |_| {}).read_only(true));
  assert_eq!(said(&mut shown, "pad", "a11y_readonly").as_deref(), Some("true"));
}

/// Its words are the app's locale's, read again when the app changes it; a locale of its own is above the app's, and
/// a word told by itself is above both.
#[test]
fn its_words_are_the_app_s_locale_s_and_a_word_told_by_itself_stays() {
  use lurq::core::Locale;
  let pad = || SignaturePad::controlled(one_stroke(), |_| {}).id("pad");
  let app = App::new();
  app.set_locale(Locale::for_tag("uk"));
  let handle = app.clone();
  let mut stage = Stage::software_in(app, move |_| Column::new().padding(8.0).child(pad()).into());
  assert!(stage.paints("Підпишіть над лінією"));
  handle.set_locale(Locale::for_tag("de"));
  stage.pass();
  assert!(stage.paints("Über der Linie unterschreiben") && !stage.paints("Підпишіть над лінією"));

  // A word told by itself stays in the app's locale.
  let app = App::new();
  app.set_locale(Locale::for_tag("uk"));
  let told = Stage::software_in(app, move |_| {
    Column::new().padding(8.0).child(pad().placeholder("Sign here")).into()
  });
  assert!(told.paints("Sign here") && !told.paints("Підпишіть над лінією"));

  // A locale of its own, in an app of none; the word told stays whichever comes first.
  let mut own = Stage::software(move |_| {
    Column::new()
      .padding(8.0)
      .child(pad().undo_label("Wstecz").locale(&Locale::for_tag("pl")))
      .into()
  });
  assert!(own.paints("Podpisz nad linią"));
  #[cfg(any(feature = "mcp", feature = "devtools"))]
  {
    let mut said = |id: &str, what: &str| {
      own
        .tree
        .get_element_by_id_mut(id)
        .and_then(|element| element.description(what))
    };
    assert_eq!(said("pad", "a11y_name").as_deref(), Some("Podpis"));
    assert_eq!(said("pad", "a11y_description").as_deref(), Some("Podpisano"));
    assert_eq!(said("pad-undo", "a11y_name").as_deref(), Some("Wstecz"));
    assert_eq!(said("pad-clear", "a11y_name").as_deref(), Some("Wyczyść"));
  }
  // With no locale it is English.
  let plain = scene(|| SignaturePad::controlled(Signature::default(), |_| {}));
  assert!(plain.paints("Sign above the line"));
}
