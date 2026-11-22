//! A node with a glass is a piece of glass over what is painted behind it: its backdrop is put back bent,
//! blurred and tinted (`RenderList::glasses`). The glasses under a container are one; a regular glass follows
//! what is behind it, its words with it; an interactive one answers a press.

use std::{
  sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
  },
  time::Duration,
};

use lurq::{
  app::{App, Tree, events::MouseButton},
  components::{Column, Rect, Row, Stack, Text},
  layout::glass::{GlassCmd, MAX_GLASS_SHAPES},
  node::{Glass, color::Color},
};

use crate::support::{RenderSnapshot, render_pass, render_pass_with_app};

const DARK: Color = Color::new(0x10, 0x10, 0x10, 255);
const LIGHT: Color = Color::new(0xf4, 0xf4, 0xf4, 255);

fn tree_of(root: impl Into<lurq::node::Element>, scale: f32) -> Tree {
  let mut tree = Tree::new();
  tree.set_scale_factor(scale);
  tree.resize((300.0 * scale) as u32, (200.0 * scale) as u32);
  tree.set_root(root);
  tree
}

fn render(root: impl Into<lurq::node::Element>, scale: f32) -> RenderSnapshot {
  render_pass(&mut tree_of(root, scale))
}

/// `content`, `inset` logical pixels into a 300 x 200 window of `fill`.
fn window(fill: Color, inset: f32, content: impl Into<lurq::node::Element>) -> Column {
  Column::new()
    .size(300.0, 200.0)
    .background(fill)
    .padding(inset)
    .child(content)
}

fn pill(width: f32, glass: Glass) -> Row {
  Row::new().size(width, 56.0).rounded(28.0).glass(glass)
}

/// The one glass of a frame.
fn only(snapshot: &RenderSnapshot) -> GlassCmd {
  assert_eq!(snapshot.glasses.len(), 1, "{:?}", snapshot.glasses);
  snapshot.glasses[0]
}

#[test]
fn a_glass_is_its_nodes_box_put_back_through_a_lens() {
  let snapshot = render(window(LIGHT, 40.0, pill(120.0, Glass::regular())), 2.0);
  assert_eq!(snapshot.backdrops.len(), 1, "{:?}", snapshot.backdrops);
  let backdrop = snapshot.backdrops[0];
  assert_eq!(backdrop.glass, Some(0));
  // One shape is the command's own box: an engine that draws no glass blurs inside it.
  assert_eq!(
    (backdrop.x, backdrop.y, backdrop.width, backdrop.height),
    (80.0, 80.0, 240.0, 112.0)
  );
  assert_eq!(backdrop.radii, [56.0; 4]);
  // A radius of 10.5 is a deviation of 5.25, in a window twice as dense.
  assert_eq!(backdrop.sigma, 10.5);

  let glass = only(&snapshot);
  assert_eq!(glass.count, 1);
  assert_eq!((glass.bend, glass.bezel), (90.0, 39.0));
  assert_eq!(glass.shapes[0].center, [200.0, 136.0]);
  assert_eq!(glass.shapes[0].half, [120.0, 56.0]);
  // The edge reads 90 inward, which is short of the far side of a pill 112 high; the shadow paints past it.
  assert_eq!(backdrop.reach, 0.0);
  assert!(backdrop.spread > 0.0);
  // The glass is under everything the node paints, and over the window's fill.
  let background = snapshot.rects.iter().find(|rect| rect.color == LIGHT).unwrap();
  assert!(background.order < backdrop.order);
}

#[test]
fn a_glass_that_is_not_there_paints_nothing() {
  let snapshot = render(window(LIGHT, 40.0, pill(120.0, Glass::regular().amount(0.0))), 1.0);
  assert!(snapshot.backdrops.is_empty() && snapshot.glasses.is_empty());
}

#[test]
fn the_glasses_under_a_container_are_one_and_the_rest_are_their_own() {
  let snapshot = render(
    window(
      LIGHT,
      20.0,
      Column::new()
        .spacing(10.0)
        .child(
          Row::new()
            .glass_container(24.0)
            .spacing(8.0)
            .child(pill(56.0, Glass::clear()))
            // What is not glass is no shape of it.
            .child(Rect::new(10.0, 10.0))
            .child(Column::new().child(pill(80.0, Glass::regular()))),
        )
        .child(pill(56.0, Glass::regular())),
    ),
    1.0,
  );
  assert_eq!(snapshot.backdrops.len(), 2, "{:?}", snapshot.backdrops);
  assert_eq!(snapshot.glasses.len(), 2);
  let joined = snapshot.glasses[0];
  assert_eq!((joined.count, joined.merge), (2, 24.0));
  // The first shape's glass is the material: clear has no shadow.
  assert_eq!(joined.shadow[2], 0.0);
  assert_eq!(joined.shapes[0].center, [48.0, 48.0]);
  assert_eq!(joined.shapes[1].center, [20.0 + 56.0 + 8.0 + 10.0 + 8.0 + 40.0, 48.0]);
  // Between the two it is glass, as far as they flow together.
  assert!(joined.distance(78.0, 48.0) < 2.0);
  // The command covers both, and reads the whole bend around them.
  let backdrop = snapshot.backdrops[0];
  assert_eq!((backdrop.x, backdrop.width), (20.0, 162.0));
  assert_eq!(backdrop.reach, 46.0);
  assert_eq!(snapshot.glasses[1].count, 1);
}

#[test]
fn a_container_holds_no_more_shapes_than_a_glass_has() {
  let mut row = Row::new().glass_container(4.0);
  for _ in 0..MAX_GLASS_SHAPES + 2 {
    row = row.child(Row::new().size(20.0, 20.0).glass(Glass::clear()));
  }
  let snapshot = render(window(LIGHT, 10.0, row), 1.0);
  assert_eq!(snapshot.glasses.len(), 3);
  assert_eq!(snapshot.glasses[0].count, MAX_GLASS_SHAPES);
  assert_eq!((snapshot.glasses[1].count, snapshot.glasses[2].count), (1, 1));
}

/// The colour of the words of a frame.
fn ink(snapshot: &RenderSnapshot) -> [f32; 4] {
  snapshot.glyphs.first().expect("words").color
}

#[test]
fn a_regular_glass_follows_what_is_behind_it_and_its_words_with_it() {
  let scene = |fill: Color| window(fill, 40.0, pill(120.0, Glass::regular()).child(Text::new("Glass")));
  let mut app = App::new();

  let mut tree = tree_of(scene(LIGHT), 1.0);
  let light = render_pass_with_app(&mut tree, &mut app);
  // White, half there.
  assert!(
    only(&light).tint[0] > 0.45 && only(&light).tint[3] > 0.45,
    "{:?}",
    only(&light).tint
  );
  let black = ink(&light);
  assert!(black[..3] == [0.0; 3] && (black[3] - 0.85).abs() < 0.01, "{black:?}");

  // Over a dark window it is the dark glass from its first frame, and its words are light from the next.
  let mut tree = tree_of(scene(DARK), 1.0);
  let dark = render_pass_with_app(&mut tree, &mut app);
  assert_eq!(only(&dark).tint[..3], [0.0; 3]);
  assert!(only(&dark).tint[3] > 0.3);
  let dark = render_pass_with_app(&mut tree, &mut app);
  assert_eq!(ink(&dark), [1.0; 4]);
}

#[test]
fn a_glass_turns_in_time_when_what_is_behind_it_changes_and_tells_who_asked() {
  let heard = Arc::new(AtomicU32::new(0));
  let scene = |fill: Color| {
    let heard = heard.clone();
    window(
      fill,
      40.0,
      pill(120.0, Glass::regular()).on_glass_look(move |dark| {
        heard.fetch_add(if dark { 100 } else { 1 }, Ordering::SeqCst);
      }),
    )
  };
  let mut app = App::new();
  let mut tree = tree_of(scene(LIGHT), 1.0);
  let start = lurq::time::Instant::now();
  tree.set_animation_clock(Some(start));
  let light = only(&render_pass_with_app(&mut tree, &mut app)).tint;
  assert_eq!(heard.load(Ordering::SeqCst), 1, "told its first look");

  tree.set_root(scene(DARK));
  let turning = only(&render_pass_with_app(&mut tree, &mut app)).tint;
  assert_eq!(turning, light, "the turn starts from where it was");
  assert_eq!(heard.load(Ordering::SeqCst), 101, "told as it turns");
  tree.set_animation_clock(Some(start + Duration::from_millis(140)));
  let half = only(&render_pass_with_app(&mut tree, &mut app)).tint;
  assert!(half[0] < light[0] && half[0] > 0.0, "{half:?}");
  tree.set_animation_clock(Some(start + Duration::from_millis(600)));
  let dark = only(&render_pass_with_app(&mut tree, &mut app)).tint;
  assert_eq!(dark[..3], [0.0; 3]);
  assert_eq!(heard.load(Ordering::SeqCst), 101, "and not again");
}

#[test]
fn a_coloured_glass_and_a_clear_one_follow_nothing() {
  let blue = Color::new(0, 122, 255, 204);
  for fill in [LIGHT, DARK] {
    let tinted = only(&render(
      window(fill, 40.0, pill(120.0, Glass::regular().tint(blue))),
      1.0,
    ));
    assert!(
      (tinted.tint[2] - 0.8).abs() < 0.01 && tinted.tint[0] == 0.0,
      "{:?}",
      tinted.tint
    );
    let clear = only(&render(window(fill, 40.0, pill(120.0, Glass::clear())), 1.0));
    assert_eq!(clear.tint, [0.0; 4]);
  }
}

#[test]
fn an_interactive_glass_grows_under_a_press_is_lit_there_and_springs_back() {
  let scene = Stack::new()
    .size(300.0, 200.0)
    .background(LIGHT)
    .child(pill(120.0, Glass::regular().interactive(true)).absolute_position(40.0, 40.0));
  let mut app = App::new();
  let mut tree = tree_of(scene, 1.0);
  let start = lurq::time::Instant::now();
  tree.set_animation_clock(Some(start));
  let rest = only(&render_pass_with_app(&mut tree, &mut app));
  assert_eq!((rest.touch[3], rest.shapes[0].scale), (0.0, 1.0));

  tree.mouse_move(70.0, 60.0);
  tree.mouse_down(70.0, 60.0, MouseButton::Left);
  render_pass_with_app(&mut tree, &mut app);
  tree.set_animation_clock(Some(start + Duration::from_millis(400)));
  render_pass_with_app(&mut tree, &mut app);
  tree.set_animation_clock(Some(start + Duration::from_millis(900)));
  let held = only(&render_pass_with_app(&mut tree, &mut app));
  assert!(held.touch[3] > 0.9, "{:?}", held.touch);
  // Lit from where the pointer is.
  assert_eq!(held.touch[..2], [70.0, 60.0]);
  assert!(held.shapes[0].scale > 1.05, "{}", held.shapes[0].scale);

  tree.mouse_up(70.0, 60.0, MouseButton::Left);
  let mut at = 900;
  let mut last = held;
  for _ in 0..80 {
    at += 16;
    tree.set_animation_clock(Some(start + Duration::from_millis(at)));
    last = only(&render_pass_with_app(&mut tree, &mut app));
  }
  assert_eq!((last.touch[3], last.shapes[0].scale), (0.0, 1.0), "at rest again");
}
