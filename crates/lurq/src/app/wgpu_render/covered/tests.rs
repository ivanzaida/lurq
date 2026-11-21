use super::*;
use crate::{
  layout::{
    quad::{ClipRect, RoundedClip},
    render_list::{RectShadow, RenderGradient, RenderGradientStop, StrokeStyle},
  },
  node::color::Color,
};

const WIDTH: f32 = 400.0;
const HEIGHT: f32 = 300.0;

fn rect([x, y, width, height]: [f32; 4], color: Color) -> RectCmd {
  RectCmd {
    order: 0,
    x,
    y,
    width,
    height,
    color,
    radii: [0.0; 4],
    stroke: [0.0; 4],
    stroke_color: Color::new(0, 0, 0, 0),
    stroke_style: StrokeStyle::default(),
    transform: [1.0, 0.0, 0.0, 1.0],
    transform_origin: [width * 0.5, height * 0.5],
    clip: ClipRect::default(),
    gradient: None,
    shadow: None,
  }
}

fn window(color: Color) -> RectCmd {
  rect([0.0, 0.0, WIDTH, HEIGHT], color)
}

const BLACK: Color = Color::new(0, 0, 0, 255);
const NONE: Color = Color::new(0, 0, 0, 0);

#[test]
fn a_clear_rect_paints_nothing_and_a_tinted_one_does() {
  assert!(paints_nothing(&window(NONE)));
  assert!(!paints_nothing(&window(Color::new(0, 0, 0, 77))));
}

#[test]
fn a_clear_rect_with_a_gradient_or_a_stroke_paints() {
  let mut gradient = window(NONE);
  gradient.gradient = Some(RenderGradient {
    kind: 0,
    flags: 0,
    dir: [1.0, 0.0],
    center: [0.5; 2],
    from_angle: 0.0,
    stops: vec![RenderGradientStop {
      color: [1.0; 4],
      position: 0.0,
    }],
  });
  assert!(!paints_nothing(&gradient));
  let mut stroke = window(NONE);
  stroke.stroke = [1.0; 4];
  stroke.stroke_color = BLACK;
  assert!(!paints_nothing(&stroke));
  stroke.stroke_color = NONE;
  assert!(paints_nothing(&stroke));
}

#[test]
fn an_opaque_rect_over_the_whole_target_covers_it() {
  assert!(covers(&window(BLACK), WIDTH, HEIGHT));
  assert!(covers(
    &rect([-10.0, -10.0, WIDTH + 20.0, HEIGHT + 20.0], BLACK),
    WIDTH,
    HEIGHT
  ));
}

#[test]
fn a_rect_that_leaves_a_pixel_shows_what_is_under_it() {
  assert!(!covers(&rect([0.0, 1.0, WIDTH, HEIGHT], BLACK), WIDTH, HEIGHT));
  assert!(!covers(&rect([0.0, 0.0, WIDTH - 1.0, HEIGHT], BLACK), WIDTH, HEIGHT));
  assert!(!covers(&window(Color::new(0, 0, 0, 254)), WIDTH, HEIGHT));
  let mut rounded = window(BLACK);
  rounded.radii = [8.0; 4];
  assert!(!covers(&rounded, WIDTH, HEIGHT));
  let mut turned = window(BLACK);
  turned.transform = [0.9, 0.1, -0.1, 0.9];
  assert!(!covers(&turned, WIDTH, HEIGHT));
  let mut shadow = window(BLACK);
  shadow.shadow = Some(RectShadow {
    inset: false,
    offset: [0.0; 2],
    spread: 0.0,
    sigma: 4.0,
    shape_radii: [0.0; 4],
  });
  assert!(!covers(&shadow, WIDTH, HEIGHT));
}

#[test]
fn a_rect_under_a_clip_covers_only_what_the_clip_leaves_whole() {
  let mut cut = window(BLACK);
  cut.clip = ClipRect {
    x: 0.0,
    y: 20.0,
    width: WIDTH,
    height: HEIGHT - 20.0,
    active: true,
    ..ClipRect::default()
  };
  assert!(!covers(&cut, WIDTH, HEIGHT));
  let mut wide = window(BLACK);
  wide.clip = ClipRect {
    x: 0.0,
    y: 0.0,
    width: WIDTH,
    height: HEIGHT,
    active: true,
    ..ClipRect::default()
  };
  assert!(covers(&wide, WIDTH, HEIGHT));
  wide.clip.rounded[0] = Some(RoundedClip {
    x: 0.0,
    y: 0.0,
    width: WIDTH,
    height: HEIGHT,
    radius: Default::default(),
  });
  assert!(!covers(&wide, WIDTH, HEIGHT));
}

#[test]
fn the_last_cover_hides_the_draws_before_it_and_the_layers_between() {
  let steps = [
    LayerStep::Draws(0..3),
    LayerStep::Composite(1),
    LayerStep::Draws(3..6),
    LayerStep::Draws(6..8),
  ];
  let covered = Covered::of(&steps, |draw| draw == 1 || draw == 4).unwrap();
  assert!(covered.hides_draw(0, 0) && covered.hides_draw(0, 2));
  assert!(covered.hides_step(1));
  assert!(covered.hides_draw(2, 3));
  assert!(!covered.hides_draw(2, 4), "the cover itself is painted");
  assert!(!covered.hides_draw(2, 5) && !covered.hides_draw(3, 6));
  assert!(!covered.hides_step(3));
}

#[test]
fn what_a_backdrop_blur_reads_before_a_cover_is_painted() {
  let steps = [
    LayerStep::Draws(0..2),
    LayerStep::Backdrop(0),
    LayerStep::Composite(1),
    LayerStep::Draws(2..5),
  ];
  let covered = Covered::of(&steps, |draw| draw == 3).unwrap();
  assert!(!covered.hides_draw(0, 0) && !covered.hides_draw(0, 1));
  assert!(covered.hides_step(2));
  assert!(covered.hides_draw(3, 2));
  assert!(!covered.hides_draw(3, 3) && !covered.hides_draw(3, 4));
}

#[test]
fn a_target_with_no_cover_hides_nothing() {
  assert_eq!(Covered::of(&[LayerStep::Draws(0..4)], |_| false), None);
}
