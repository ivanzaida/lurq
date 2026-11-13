//! `ui::Skeleton`: a line, a circle, a block and a paragraph at the design's
//! sizes in the muted fill, the app's sizes, the pulse and the shimmer, and a
//! skeleton standing still where motion is reduced.

use lurq::{
  app::{
    App,
    theme::{PaletteColor, ThemePalette},
  },
  components::{Column, Row},
  layout::quad::{Quad, QuadContent},
  node::color::Color,
  ui::{
    Length, SKELETON_PULSE_PERIOD, SKELETON_SHIMMER_PERIOD, Skeleton, SkeletonAnimation, SkeletonPart, SkeletonShape,
  },
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// The shapes painted in the muted fill, from the top.
fn bones(stage: &Stage) -> Vec<Quad> {
  let muted = light(PaletteColor::Muted);
  let mut found: Vec<Quad> = stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Rect { color, gradient: None } if *color == muted))
    .collect();
  found.sort_by(|a, b| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)));
  found
}

fn radius(quad: &Quad) -> f32 {
  quad.border_radius.map(|radius| radius.top_left).unwrap_or(0.0)
}

#[test]
fn a_line_a_circle_and_a_block_are_as_the_design_draws_them() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .width(300.0)
      .spacing(16.0)
      .child(Skeleton::line().id("line"))
      .child(Skeleton::circle(40.0).id("circle"))
      .child(Skeleton::block(200.0, 112.0).id("block"))
      .into()
  });
  assert_eq!(stage.bounds("line").2, 300.0, "a line is as wide as the room it has");
  assert_eq!(stage.bounds("line").3, 12.0);
  assert_eq!(stage.bounds("circle").2, 40.0);
  assert_eq!(stage.bounds("block").2, 200.0);
  let shapes = bones(&stage);
  assert_eq!(shapes.len(), 3);
  assert_eq!((shapes[0].width, shapes[0].height), (300.0, 12.0));
  assert_eq!(radius(&shapes[0]), 4.0, "radius-sm");
  assert_eq!((shapes[1].width, shapes[1].height), (40.0, 40.0));
  assert!(radius(&shapes[1]) >= 20.0, "round all the way");
  assert_eq!((shapes[2].width, shapes[2].height), (200.0, 112.0));
  assert_eq!(radius(&shapes[2]), 8.0, "radius-lg");
}

#[test]
fn a_paragraph_is_lines_10_apart_with_a_shorter_last_one() {
  let stage = Stage::new(|_| Column::new().width(300.0).child(Skeleton::paragraph(4)).into());
  let lines = bones(&stage);
  let widths: Vec<f32> = lines.iter().map(|line| line.width.round()).collect();
  assert_eq!(widths, [300.0, 300.0, 276.0, 180.0], "the design's");
  for pair in lines.windows(2) {
    assert_eq!(pair[1].y - pair[0].y, 22.0, "12 high, 10 apart");
  }
  assert!(lines.iter().all(|line| line.height == 12.0));
}

#[test]
fn the_app_sizes_a_skeleton() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .width(300.0)
      .spacing(16.0)
      .child(Skeleton::line().width(140.0).height(10.0).id("line"))
      .child(Skeleton::block(Length::Fill, 132.0).id("block"))
      .child(Skeleton::paragraph(2).width(200.0).height(16.0).id("paragraph"))
      .into()
  });
  let (_, _, width, height) = stage.bounds("line");
  assert_eq!((width, height), (140.0, 10.0));
  let (_, _, width, height) = stage.bounds("block");
  assert_eq!((width, height), (300.0, 132.0), "a block that fills");
  let (_, _, width, height) = stage.bounds("paragraph");
  assert_eq!((width, height), (200.0, 42.0));
  let lines: Vec<_> = bones(&stage).into_iter().filter(|line| line.height == 16.0).collect();
  let widths: Vec<f32> = lines.iter().map(|line| line.width.round()).collect();
  assert_eq!(widths, [200.0, 120.0]);
}

#[test]
fn beside_a_circle_lines_fill_the_rest_of_the_row() {
  let stage = Stage::new(|_| {
    Row::new()
      .width(300.0)
      .spacing(12.0)
      .child(Skeleton::circle(40.0))
      .child(
        Column::new()
          .flex(1.0)
          .spacing(8.0)
          .child(Skeleton::line())
          .child(Skeleton::line().width(140.0).height(10.0)),
      )
      .into()
  });
  let shapes = bones(&stage);
  let line = shapes.iter().find(|shape| shape.height == 12.0).unwrap();
  assert_eq!(line.width, 248.0);
  assert_eq!(line.x, 52.0);
}

#[test]
fn a_skeleton_pulses_and_stands_still_where_motion_is_reduced() {
  let pulsing = |stage: &mut Stage| {
    (0..8).any(|step| {
      stage.pass_after(SKELETON_PULSE_PERIOD / 8 * step);
      let opacity = bones(stage)[0].opacity;
      assert!((0.49..=1.0).contains(&opacity));
      opacity < 0.9
    })
  };
  let mut stage = Stage::new(|_| Column::new().width(200.0).child(Skeleton::line()).into());
  assert!(pulsing(&mut stage));

  let app = App::new();
  app.theme().set_reduced_motion(Some(true));
  let mut still = Stage::in_app(app, |_| Column::new().width(200.0).child(Skeleton::line()).into());
  assert!(!pulsing(&mut still));

  let mut none = Stage::new(|_| {
    Column::new()
      .width(200.0)
      .child(Skeleton::line().animation(SkeletonAnimation::None))
      .into()
  });
  assert!(!pulsing(&mut none));
}

#[test]
fn a_sheen_crosses_a_skeleton_that_shimmers() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .width(300.0)
      .child(Skeleton::line().animation(SkeletonAnimation::Shimmer).id("line"))
      .into()
  });
  let (x, ..) = stage.bounds("line");
  // The sheen as it is painted, if it is over the line: the quad's position is where it was moved to.
  let sheen = |stage: &Stage| {
    stage
      .quads()
      .into_iter()
      .find(|quad| matches!(&quad.content, QuadContent::Rect { gradient: Some(_), .. }))
  };
  let first = sheen(&stage).expect("a sheen, from the muted fill to a lighter one and back");
  assert_eq!(first.width, 200.0, "the design's shimmer");
  // Over a period it goes from before the line's start to past its end, clipped to the line.
  let mut places = Vec::new();
  for step in 0..16 {
    stage.pass_after(SKELETON_SHIMMER_PERIOD / 16 * step);
    if let Some(quad) = sheen(&stage) {
      let at = quad.x - x;
      assert!((-200.5..=1200.5).contains(&at), "{at}");
      assert_eq!(quad.clip.width, 300.0, "clipped to the line");
      places.push(at);
    }
  }
  assert!(
    places.iter().any(|at| *at < 0.0),
    "it comes in from before the start: {places:?}"
  );
  assert!(places.iter().any(|at| *at > 0.0), "and goes on: {places:?}");
  assert!(
    bones(&stage).iter().all(|bone| bone.opacity == 1.0),
    "and does not pulse"
  );
}

#[test]
fn a_theme_rule_changes_every_skeleton() {
  let app = App::new();
  app.theme().patch_recipe::<Skeleton>(|rules| {
    rules
      .base(SkeletonPart::Bone, |s| s.background(PaletteColor::Secondary))
      .variant(SkeletonShape::Line, SkeletonPart::Bone, |s| s.height(16.0))
  });
  let mut stage = Stage::in_app(app, |_| {
    Column::new().width(200.0).child(Skeleton::line().id("line")).into()
  });
  assert_eq!(stage.bounds("line").3, 16.0);
  assert_eq!(stage.fill_color("line"), Some(light(PaletteColor::Secondary)));
}
