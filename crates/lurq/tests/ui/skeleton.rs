//! `ui::Skeleton`: a line, a circle, a block and a paragraph at the design's
//! sizes in the muted fill, the app's sizes, the pulse and the shimmer, and a
//! skeleton standing still where motion is reduced.

use lurq::{
  app::{
    App,
    theme::{PaletteColor, ThemePalette},
  },
  components::{Column, Row},
  layout::{
    Constraints, Size,
    quad::{Quad, QuadContent},
  },
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
fn a_sheen_as_wide_as_the_skeleton_crosses_it_in_one_time_whatever_its_width() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .spacing(16.0)
      .child(
        Skeleton::block(120.0, 12.0)
          .animation(SkeletonAnimation::Shimmer)
          .id("narrow"),
      )
      .child(
        Skeleton::block(600.0, 12.0)
          .animation(SkeletonAnimation::Shimmer)
          .id("wide"),
      )
      .into()
  });
  // A window wide enough for the wide one, so that its sheen is seen to its end.
  stage
    .tree
    .set_layout_constraints_override(Some(Constraints::loose(Size::new(800.0, 300.0))));
  stage.pass();
  let narrow = stage.bounds("narrow");
  let wide = stage.bounds("wide");
  // The sheen painted over the skeleton at `bounds`, where it is: from the skeleton's start, and how wide.
  let sheen = |stage: &Stage, (x, y, ..): (f32, f32, f32, f32)| {
    stage
      .quads()
      .into_iter()
      .find(|quad| matches!(&quad.content, QuadContent::Rect { gradient: Some(_), .. }) && (quad.y - y).abs() < 0.01)
      .map(|quad| {
        assert_eq!(quad.clip.x, x, "clipped to the skeleton");
        (quad.x - x, quad.width)
      })
  };
  // Where in its period the sheen is now: it goes from one width before the start, -width, to the start of
  // the skeleton halfway, and to its end, +width, at the end. At the very end of a period it is wholly outside
  // and not painted, so it is read a quarter of a period on where it is not seen.
  let period = SKELETON_SHIMMER_PERIOD.as_secs_f32();
  let (offset, at, width) = (0..4)
    .find_map(|quarter| {
      let offset = quarter as f32 * period / 4.0;
      stage.pass_after(std::time::Duration::from_secs_f32(offset));
      sheen(&stage, narrow).map(|(at, width)| (offset, at, width))
    })
    .expect("a sheen, from the muted fill to a lighter one and back");
  assert!((width - 120.0).abs() < 0.01, "as wide as the skeleton: {width}");
  let phase = (at + 120.0) / 240.0 - offset / period;
  let next = (1.0 - phase.rem_euclid(1.0)) * period;
  for (share, place) in [(0.05, -0.9), (0.5, 0.0), (0.95, 0.9)] {
    stage.pass_after(std::time::Duration::from_secs_f32(next + share * period));
    for (bounds, side) in [(narrow, 120.0), (wide, 600.0)] {
      let (at, width) = sheen(&stage, bounds).unwrap_or_else(|| panic!("the sheen over {bounds:?} at {share}"));
      assert!((width - side).abs() < 0.01, "as wide as the skeleton: {width}");
      assert!(
        (at - place * side).abs() < side * 0.02,
        "at {share} of the period, the {side} one's sheen is at {at}, where {} was looked for",
        place * side
      );
    }
  }
  assert!(
    bones(&stage).iter().all(|bone| bone.opacity == 1.0),
    "and it does not pulse"
  );
  // Where motion is reduced it stands still: no sheen.
  stage.app.theme().set_reduced_motion(Some(true));
  stage.pass();
  assert!(sheen(&stage, narrow).is_none());
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
