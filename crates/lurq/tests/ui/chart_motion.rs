//! What every chart's motion does, as the bar chart's test has it: the plot moves in as it first shows and to a
//! change of its data, drawn at each frame while it moves and not at rest, and stands still where motion is reduced
//! or the chart is told not to move. The pie's, the radar's and the heatmap's tests call it with their chart.

use std::time::Duration;

use lurq::{
  app::ctx::Ctx,
  components::Column,
  core::Signal,
  layout::{Constraints, Size},
  node::{Element, color::Color},
};

use super::stage::Stage;

/// About half of a chart's motion: where a test reads a plot on its way.
pub const HALF_WAY: Duration = Duration::from_millis(120);

/// A chart, mounted with the id `c` from its values and whether it moves, that moves as every chart does.
pub fn moves_as_every_chart(chart: impl Fn(&mut Ctx, Vec<f64>, bool) -> Element + Send + Sync + 'static) {
  let (shown, values, moves) = (Signal::new(false), Signal::new(vec![1.0, 2.0, 3.0]), Signal::new(true));
  let (on, bound, moving) = (shown.clone(), values.clone(), moves.clone());
  let mut stage = Stage::new(move |ctx| {
    let mut column = Column::new().size(420.0, 600.0);
    if on.get() {
      column = column.child(chart(ctx, bound.get(), moving.get()));
    }
    column.into()
  });
  stage
    .tree
    .set_layout_constraints_override(Some(Constraints::loose(Size::new(800.0, 700.0))));
  stage.pass();
  let revision = |stage: &mut Stage| {
    stage
      .tree
      .get_element_by_id_mut("c-plot")
      .and_then(|element| element.canvas())
      .unwrap()
      .status()
      .content_revision
  };

  shown.set(true);
  stage.pass_after(Duration::ZERO);
  assert!(stage.tree.has_moving_canvas(), "a chart first shown moves in");
  let entering = revision(&mut stage);
  stage.pass_after(Duration::from_millis(100));
  assert!(
    revision(&mut stage) > entering,
    "drawn again at the next frame on its way"
  );
  stage.pass();
  assert!(!stage.tree.has_moving_canvas(), "and comes to rest in a moment");
  let rest = revision(&mut stage);
  stage.pass();
  stage.pass_after(Duration::from_millis(16));
  assert_eq!(
    revision(&mut stage),
    rest,
    "at rest, frames and renders that change nothing draw nothing"
  );

  values.set(vec![3.0, 1.0, 2.0]);
  stage.pass_after(Duration::ZERO);
  assert!(stage.tree.has_moving_canvas(), "a change of the data moves it");
  stage.pass();
  assert!(!stage.tree.has_moving_canvas());

  stage.app.theme().set_reduced_motion(Some(true));
  stage.pass();
  values.set(vec![2.0, 3.0, 1.0]);
  stage.pass_after(Duration::ZERO);
  assert!(
    !stage.tree.has_moving_canvas(),
    "where motion is reduced, it goes to its place at once"
  );
  stage.app.theme().set_reduced_motion(None);

  moves.set(false);
  stage.pass();
  values.set(vec![1.0, 1.0, 4.0]);
  stage.pass_after(Duration::ZERO);
  assert!(
    !stage.tree.has_moving_canvas(),
    "nor where the chart is told not to move"
  );
}

/// The color the plot with the id `c-plot` drew at `(x, y)` of its box, where its canvases are drawn on the CPU.
pub fn pixel(stage: &mut Stage, x: f32, y: f32) -> Color {
  let canvas = stage
    .tree
    .get_element_by_id_mut("c-plot")
    .and_then(|element| element.canvas())
    .expect("the plot is a canvas");
  let scale = canvas.scale_factor().max(1.0);
  let shot = canvas
    .snapshot()
    .try_take()
    .expect("drawn in software")
    .expect("a snapshot");
  let (px, py) = ((x * scale) as u32, (y * scale) as u32);
  let at = ((py.min(shot.height - 1) * shot.width + px.min(shot.width - 1)) * 4) as usize;
  Color::new(shot.rgba[at], shot.rgba[at + 1], shot.rgba[at + 2], shot.rgba[at + 3])
}

/// Whether `a` and `b` are the same color within a few of each channel.
pub fn near(a: Color, b: Color) -> bool {
  [(a.r(), b.r()), (a.g(), b.g()), (a.b(), b.b()), (a.a(), b.a())]
    .into_iter()
    .all(|(a, b)| (a as i32 - b as i32).abs() <= 3)
}

/// A stage whose canvases are drawn on the CPU, its chart 420 wide.
pub fn software(chart: impl Fn(&mut Ctx) -> Element + Send + Sync + 'static) -> Stage {
  let mut stage = Stage::software(move |ctx| Column::new().size(420.0, 600.0).child(chart(ctx)).into());
  stage
    .tree
    .set_layout_constraints_override(Some(Constraints::loose(Size::new(800.0, 700.0))));
  stage.pass();
  stage
}
