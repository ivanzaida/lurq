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
    Length, SKELETON_PULSE_PERIOD, SKELETON_SHIMMER_PERIOD, Skeleton, SkeletonAnimation, SkeletonMotion, SkeletonPart,
    SkeletonShape,
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

/// How many times the opacity of the bones painted changed over `span`, in passes 2 ms apart.
fn pulse_changes(stage: &mut Stage, span: std::time::Duration) -> usize {
  let opacities =
    |stage: &Stage| -> Vec<String> { bones(stage).iter().map(|bone| format!("{:.4}", bone.opacity)).collect() };
  let mut last = opacities(stage);
  let mut changed = 0;
  let mut at = std::time::Duration::ZERO;
  while at < span {
    at += std::time::Duration::from_millis(2);
    stage.pass_after(at);
    let now = opacities(stage);
    if now != last {
      changed += 1;
      last = now;
    }
  }
  changed
}

#[test]
fn stepped_skeletons_pulse_together_a_pose_a_step_and_ask_for_no_frame_between() {
  let motion = SkeletonMotion::stepped(20).period(std::time::Duration::from_millis(1600));
  let mut stage = Stage::new(move |_| {
    Column::new()
      .width(200.0)
      .spacing(8.0)
      .child(Skeleton::line().motion(motion))
      .child(Skeleton::circle(24.0).motion(motion))
      .into()
  });
  stage.pass_after(std::time::Duration::from_millis(3));
  let next = stage
    .next_change_after(std::time::Duration::from_millis(3))
    .expect("it pulses");
  assert!(
    next > std::time::Duration::ZERO && next <= std::time::Duration::from_millis(50),
    "the next frame is at the next step: {next:?}"
  );
  // Half a pulse (0.8 s) is 16 steps at 20 a second, and the two bones change together, in one phase.
  let changed = pulse_changes(&mut stage, std::time::Duration::from_millis(800));
  assert!((15..=17).contains(&changed), "{changed} changes in half a pulse");
  let opacities: Vec<f32> = bones(&stage).iter().map(|bone| bone.opacity).collect();
  assert_eq!(opacities[0], opacities[1], "in one phase");

  // Where motion is reduced it stands still, stepped or not.
  stage.app.theme().set_reduced_motion(Some(true));
  stage.pass();
  assert_eq!(
    stage.next_change_after(std::time::Duration::from_millis(3)),
    None,
    "nothing moves"
  );
}

#[test]
fn a_theme_gives_every_skeleton_its_motion() {
  let app = App::new();
  app.theme().set_skeleton_motion(SkeletonMotion::stepped(10));
  assert_eq!(app.theme().skeleton_motion().steps_per_second(), Some(10));
  let mut stage = Stage::in_app(app, |_| Column::new().width(200.0).child(Skeleton::line()).into());
  stage.pass_after(std::time::Duration::from_millis(3));
  let next = stage
    .next_change_after(std::time::Duration::from_millis(3))
    .expect("it pulses");
  assert!(
    next > std::time::Duration::from_millis(10),
    "in steps of a tenth of a second: {next:?}"
  );

  // A skeleton's own motion wins.
  let mut smooth = Stage::new(|_| {
    Column::new()
      .width(200.0)
      .child(Skeleton::line().motion(SkeletonMotion::SMOOTH))
      .into()
  });
  smooth.pass_after(std::time::Duration::from_millis(3));
  assert_eq!(
    smooth.next_change_after(std::time::Duration::from_millis(3)),
    Some(std::time::Duration::ZERO)
  );
}

#[test]
fn a_skeleton_takes_the_layout_an_app_chains_on_a_bone() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .width(300.0)
      .spacing(8.0)
      .child(
        Row::new()
          .width(300.0)
          .child(Skeleton::block(40.0, 20.0).radius(6.0).flex_shrink(0.0).id("tile"))
          .child(Skeleton::line().flex(1.0).min_width(0.0).id("rest")),
      )
      .child(Skeleton::line().max_width(120.0).id("capped"))
      .child(
        Skeleton::block(Length::Fill, 4.0)
          .min_height(10.0)
          .max_width(200.0)
          .id("bar"),
      )
      .into()
  });
  assert_eq!(stage.bounds("tile").2, 40.0);
  let (x, _, width, _) = stage.bounds("rest");
  assert_eq!((x, width), (40.0, 260.0), "a line that takes the room left in its row");
  assert_eq!(stage.bounds("capped").2, 120.0);
  let (_, _, width, height) = stage.bounds("bar");
  assert_eq!((width, height), (200.0, 10.0));
  let tile = bones(&stage)
    .into_iter()
    .find(|bone| bone.width == 40.0)
    .expect("the tile's bone");
  assert_eq!(radius(&tile), 6.0, "its own radius");
}

/// The least opacity the first bone painted takes over one pulse, sampled every 10 ms.
fn dimmest(stage: &mut Stage) -> f32 {
  (0..200)
    .map(|step| {
      stage.pass_after(std::time::Duration::from_millis(10) * step);
      bones(stage)[0].opacity
    })
    .fold(1.0, f32::min)
}

#[test]
fn a_pulse_dims_to_half_unless_a_theme_or_a_skeleton_says_how_far() {
  let mut half = Stage::new(|_| Column::new().width(200.0).child(Skeleton::line()).into());
  let least = dimmest(&mut half);
  assert!((least - 0.5).abs() < 0.02, "to half: {least}");

  let app = App::new();
  app.theme().set_skeleton_motion(SkeletonMotion::SMOOTH.dim_to(0.55));
  assert_eq!(app.theme().skeleton_motion().dimmed(), 0.55);
  let mut themed = Stage::in_app(app, |_| Column::new().width(200.0).child(Skeleton::line()).into());
  let least = dimmest(&mut themed);
  assert!((least - 0.55).abs() < 0.02, "to the theme's 0.55: {least}");

  let mut own = Stage::new(|_| {
    Column::new()
      .width(200.0)
      .child(Skeleton::line().motion(SkeletonMotion::SMOOTH.dim_to(0.8)))
      .into()
  });
  let least = dimmest(&mut own);
  assert!((least - 0.8).abs() < 0.02, "to its own 0.8: {least}");
  assert_eq!(SkeletonMotion::SMOOTH.dim_to(1.7).dimmed(), 1.0, "no more than whole");
  assert_eq!(SkeletonMotion::SMOOTH.dim_to(-1.0).dimmed(), 0.0);
  assert_eq!(SkeletonMotion::SMOOTH.dimmed(), 0.5);
}

#[test]
fn a_theme_has_every_skeleton_shimmer_unless_one_says_otherwise() {
  let app = App::new();
  app.theme().set_skeleton_animation(SkeletonAnimation::Shimmer);
  assert_eq!(app.theme().skeleton_animation(), SkeletonAnimation::Shimmer);
  let mut stage = Stage::in_app(app, |_| {
    Column::new()
      .width(200.0)
      .spacing(16.0)
      .child(Skeleton::line().id("themed"))
      .child(Skeleton::line().animation(SkeletonAnimation::Pulse).id("own"))
      .into()
  });
  let sheens = |stage: &mut Stage| {
    (0..4)
      .map(|quarter| {
        stage.pass_after(SKELETON_SHIMMER_PERIOD / 4 * quarter);
        stage
          .quads()
          .into_iter()
          .filter(|quad| matches!(&quad.content, QuadContent::Rect { gradient: Some(_), .. }))
          .count()
      })
      .max()
      .unwrap_or(0)
  };
  assert_eq!(
    sheens(&mut stage),
    1,
    "the themed one shimmers, the one told to pulse does not"
  );
}

#[test]
fn an_enter_not_to_play_with_its_parent_plays_only_where_its_part_comes_into_what_was_there() {
  use lurq::{
    app::theme::{MotionDuration, MotionEasing},
    core::Signal,
    ui::{FADE_IN, PartAnimation, PartStyle, Recipe},
  };
  let shown = Signal::new(0u8);
  let held = shown.clone();
  let mut stage = Stage::first_frame(400.0, 300.0, move |_| {
    // A panel keyed by what is shown, which fades in as it comes, and not with the page.
    let panel = Recipe::new().base("panel", |s: PartStyle| {
      s.enter(PartAnimation::new(FADE_IN, MotionDuration::Slow, MotionEasing::Enter))
        .enter_with_parent(false)
    });
    let theme = lurq::ui::current_theme();
    let part = theme
      .resolve::<Skeleton>(&lurq::ui::Case::new(), Some(&panel))
      .part("panel");
    let which = held.get();
    Column::new()
      .size(400.0, 300.0)
      .child(
        Row::new()
          .size(200.0, 100.0)
          .background(light(PaletteColor::Primary))
          .part(&part)
          .key(format!("panel-{which}"))
          .id("panel"),
      )
      .into()
  });
  let opacity = |stage: &Stage| {
    stage
      .quads()
      .into_iter()
      .find(|quad| matches!(&quad.content, QuadContent::Rect { color, .. } if *color == light(PaletteColor::Primary)))
      .map(|quad| quad.opacity)
      .expect("the panel")
  };
  assert_eq!(opacity(&stage), 1.0, "whole on the page's first paint");
  shown.set(1);
  stage.pass_after(std::time::Duration::ZERO);
  stage.pass_after(std::time::Duration::from_millis(50));
  let fading = opacity(&stage);
  assert!(fading < 0.99, "the panel that came in its place fades in: {fading}");
}

#[test]
fn an_enter_plays_with_its_parent_unless_told() {
  use lurq::{
    app::theme::{MotionDuration, MotionEasing},
    ui::{FADE_IN, PartAnimation, PartStyle, Recipe},
  };
  let stage = Stage::first_frame(400.0, 300.0, move |_| {
    let panel = Recipe::new().base("panel", |s: PartStyle| {
      s.enter(PartAnimation::new(FADE_IN, MotionDuration::Slow, MotionEasing::Enter))
    });
    let part = lurq::ui::current_theme()
      .resolve::<Skeleton>(&lurq::ui::Case::new(), Some(&panel))
      .part("panel");
    Column::new()
      .size(400.0, 300.0)
      .child(
        Row::new()
          .size(200.0, 100.0)
          .background(light(PaletteColor::Primary))
          .part(&part),
      )
      .into()
  });
  let opacity = stage
    .quads()
    .into_iter()
    .find(|quad| matches!(&quad.content, QuadContent::Rect { color, .. } if *color == light(PaletteColor::Primary)))
    .map(|quad| quad.opacity);
  assert!(
    opacity.is_none_or(|opacity| opacity < 0.99),
    "it fades in with the page: {opacity:?}"
  );
}
