//! A video player: the board's numbers of its three forms, the form following its width from the first frame, each
//! state the board draws, what each control and each key tells the app, its menus, the controls fading out while it
//! plays, the app's layer with the controls and without them, and a theme's rules.

use std::{
  sync::{Arc, Mutex},
  time::{Duration, Instant},
};

use lurq::{
  app::{ctx::Ctx, events::MouseButton, theme::Theme, window::Appearance},
  components::Column,
  layout::quad::QuadContent,
  node::{CursorIcon, Element, color::Color, dimension::Dimension},
  ui::{Length, VIDEO_PLAYER_HIDE_AFTER, VideoPlayer, VideoPlayerMenu, VideoPlayerPart, VideoPlayerSize},
};

use super::stage::Stage;

const WHITE_20: Color = Color::new(255, 255, 255, 0x33);
const BLACK_60: Color = Color::new(0, 0, 0, 0x99);
const BLACK_70: Color = Color::new(0, 0, 0, 0xB3);

/// What the handlers of a player were told, in order.
#[derive(Clone, Default)]
struct Told(Arc<Mutex<Vec<String>>>);

impl Told {
  fn say(&self) -> impl Fn(String) + Send + Sync + Clone + 'static {
    let log = self.0.clone();
    move |what| log.lock().unwrap().push(what)
  }

  fn take(&self) -> Vec<String> {
    std::mem::take(&mut *self.0.lock().unwrap())
  }
}

/// A player of 4:10, paused at 1:24, held by the app, every handler telling `told`.
fn player(told: &Told) -> VideoPlayer {
  let say = told.say();
  let (a, b, c, d, e, f, g, h, i, j, k) = (
    say.clone(),
    say.clone(),
    say.clone(),
    say.clone(),
    say.clone(),
    say.clone(),
    say.clone(),
    say.clone(),
    say.clone(),
    say.clone(),
    say,
  );
  VideoPlayer::new()
    .id("v")
    .media(
      Column::new()
        .id("picture")
        .size(Dimension::Pct(100.0), Dimension::Pct(100.0)),
    )
    .duration(250.0)
    .buffered(140.0)
    .playing_controlled(false, move |to| a(format!("playing {to}")))
    .position_controlled(84.0, move |to| b(format!("position {to}")))
    .on_position_change_end(move |to| c(format!("position end {to}")))
    .volume_controlled(0.6, move |to| d(format!("volume {to}")))
    .muted_controlled(false, move |to| e(format!("muted {to}")))
    .rate_controlled(1.0, move |to| f(format!("rate {to}")))
    .captions_controlled(false, move |to| g(format!("captions {to}")))
    .fullscreen_controlled(false, move |to| h(format!("fullscreen {to}")))
    .picture_in_picture_controlled(false, move |to| i(format!("picture in picture {to}")))
    .qualities(&["1080p", "720p"])
    .quality_controlled(None, move |to| j(format!("quality {to:?}")))
    .on_retry(move || k("retry".to_owned()))
}

/// A window of 800 by 520 with `scene` at its top left, settled.
fn stage(scene: impl Fn(&mut Ctx) -> Element + Send + Sync + 'static) -> Stage {
  let mut stage = Stage::first_frame(800.0, 520.0, move |ctx| {
    Column::new().size(800.0, 520.0).child(scene(ctx)).into()
  });
  stage.pass();
  stage
}

/// The still form of what `make` builds.
fn still(make: impl Fn() -> VideoPlayer + Send + Sync + 'static) -> Stage {
  stage(move |_| make().build(&lurq::ui::current_theme()))
}

/// The mounted form of what `make` builds.
fn mounted(make: impl Fn() -> VideoPlayer + Send + Sync + 'static) -> Stage {
  stage(move |ctx| make().mount(ctx))
}

fn has(stage: &mut Stage, id: &str) -> bool {
  stage.tree.get_element_by_id_mut(id).is_some()
}

/// That the element `id` is said to be `is` under `what` to tooling, which keeps it with `mcp` or `devtools`.
#[cfg(any(feature = "devtools", feature = "mcp"))]
fn says(stage: &mut Stage, id: &str, what: &str, is: &str) {
  let said = stage
    .tree
    .get_element_by_id_mut(id)
    .and_then(|element| element.description(what));
  assert_eq!(said.as_deref(), Some(is), "{id} {what}");
}

#[cfg(not(any(feature = "devtools", feature = "mcp")))]
fn says(_: &mut Stage, _: &str, _: &str, _: &str) {}

/// The quads filled with `color` that are `width` by `height`.
fn rects(stage: &Stage, color: Color, width: f32, height: f32) -> usize {
  stage
    .quads()
    .into_iter()
    .filter(|quad| {
      matches!(&quad.content, QuadContent::Rect { color: fill, .. } if *fill == color)
        && (quad.width, quad.height) == (width, height)
    })
    .count()
}

/// The quad of the bar: its gradient and its opacity.
fn bar_quad(stage: &mut Stage) -> lurq::layout::quad::Quad {
  let (x, y, width, height) = stage.bounds("v-controls");
  stage
    .quads()
    .into_iter()
    .find(|quad| {
      matches!(&quad.content, QuadContent::Rect { gradient: Some(_), .. })
        && (quad.x, quad.y, quad.width, quad.height) == (x, y, width, height)
    })
    .expect("the bar is painted with its gradient")
}

/// How opaque the bar is as the window composites it: the opacity of the groups it is painted in, nothing where it
/// is not painted at all. A faded group is painted into a layer of its own at its opacity, its quads at their own.
fn bar_opacity(stage: &mut Stage) -> f32 {
  let (x, y, width, height) = stage.bounds("v-controls");
  let snapshot = crate::support::render_pass_with_app(&mut stage.tree, &mut stage.app);
  let Some(bar) = snapshot
    .rects
    .iter()
    .find(|rect| (rect.x, rect.y, rect.width, rect.height) == (x, y, width, height))
  else {
    return 0.0;
  };
  snapshot
    .layers
    .iter()
    .filter(|layer| (layer.start_order..layer.end_order).contains(&bar.order))
    .map(|layer| layer.opacity)
    .product()
}

/// A press at a point of the window.
fn press_at(stage: &mut Stage, x: f32, y: f32) {
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass();
}

#[test]
fn the_large_player_is_the_boards() {
  let told = Told::default();
  let made = told.clone();
  let mut stage = still(move || player(&made));
  assert_eq!(stage.bounds("v"), (0.0, 0.0, 640.0, 360.0), "640 by 360");
  // The bar: 88 high at the bottom, padded 12 at the sides and 8 at the bottom, a row of 36.
  assert_eq!(stage.bounds("v-controls"), (0.0, 272.0, 640.0, 88.0));
  assert_eq!(
    stage.bounds("v-play"),
    (12.0, 316.0, 36.0, 36.0),
    "play at the row's start"
  );
  let (x, y, width, height) = stage.bounds("v-fullscreen");
  assert_eq!(
    (x + width, y, width, height),
    (628.0, 316.0, 36.0, 36.0),
    "fullscreen at its end"
  );
  for (id, then) in [
    ("v-captions", "v-speed"),
    ("v-speed", "v-settings"),
    ("v-settings", "v-pip"),
    ("v-pip", "v-fullscreen"),
  ] {
    let (a, _, a_width, _) = stage.bounds(id);
    let (b, ..) = stage.bounds(then);
    assert_eq!(b - (a + a_width), 4.0, "4 between {id} and {then}");
  }
  // The speed is a text button, 36 high and padded 10.
  let (_, _, _, speed) = stage.bounds("v-speed");
  assert_eq!(speed, 36.0);
  assert!(stage.paints("1×"), "the speed reads 1×");
  // The seek bar runs over the row from padding to padding.
  let (seek_x, _, seek_width, _) = stage.bounds("v-seek");
  assert_eq!((seek_x, seek_width), (12.0, 616.0));
  // The gradient runs from nothing at the top to black at 70% at the bottom.
  let bar = bar_quad(&mut stage);
  let QuadContent::Rect {
    gradient: Some(gradient),
    ..
  } = &bar.content
  else {
    unreachable!()
  };
  let (first, last) = (gradient.stops.first().unwrap(), gradient.stops.last().unwrap());
  assert_eq!(first.color[3], 0.0, "clear at the top");
  assert!(
    (last.color[3] - 0.7).abs() < 0.01,
    "black at 70% at the bottom: {:?}",
    last.color
  );
  // Paused, the play disc of 64 stands in the middle.
  assert_eq!(stage.bounds("v-center"), (288.0, 148.0, 64.0, 64.0));
  assert!(told.take().is_empty(), "drawing it tells nothing");
}

#[test]
fn the_compact_player_is_the_boards() {
  let told = Told::default();
  let mut stage = still(move || player(&told).size(VideoPlayerSize::Md));
  assert_eq!(stage.bounds("v"), (0.0, 0.0, 320.0, 180.0));
  assert_eq!(stage.bounds("v-controls"), (0.0, 108.0, 320.0, 72.0), "72 high");
  assert_eq!(
    stage.bounds("v-play"),
    (8.0, 146.0, 28.0, 28.0),
    "a row of 28, 6 over the bottom"
  );
  let (x, _, width, _) = stage.bounds("v-fullscreen");
  assert_eq!(x + width, 312.0, "fullscreen 8 from the end");
  let (more, _, more_width, _) = stage.bounds("v-more");
  assert_eq!(
    (x - (more + more_width), more_width),
    (2.0, 28.0),
    "more before it, 2 apart"
  );
  assert!(has(&mut stage, "v-mute"));
  for id in ["v-captions", "v-speed", "v-settings", "v-pip"] {
    assert!(!has(&mut stage, id), "{id} is in the menu");
  }
  // The play disc is 48 in the compact player.
  let (_, _, disc, _) = stage.bounds("v-center");
  assert_eq!(disc, 48.0);
}

#[test]
fn the_minimal_player_is_play_the_seek_bar_and_the_time() {
  let told = Told::default();
  let mut stage = still(move || player(&told).size(VideoPlayerSize::Sm).position_controlled(7.0, |_| {}));
  assert_eq!(stage.bounds("v"), (0.0, 0.0, 200.0, 113.0));
  assert_eq!(stage.bounds("v-controls"), (0.0, 65.0, 200.0, 48.0), "one row 48 high");
  assert_eq!(
    stage.bounds("v-play"),
    (4.0, 81.0, 28.0, 28.0),
    "padded 16 at the top and 4 at the start"
  );
  assert!(stage.paints("0:07"), "the elapsed time alone");
  let (seek, ..) = stage.bounds("v-seek");
  assert_eq!(seek, 4.0 + 28.0 + 6.0, "the seek bar 6 after play");
  for id in ["v-center", "v-mute", "v-fullscreen", "v-more"] {
    assert!(!has(&mut stage, id), "no {id} in the minimal player");
  }
}

#[test]
fn the_form_follows_the_width_from_the_first_frame() {
  for (width, form) in [
    (640.0, "large"),
    (432.0, "large"),
    (320.0, "compact"),
    (200.0, "minimal"),
  ] {
    let told = Told::default();
    let mut stage = Stage::first_frame(800.0, 520.0, move |ctx| {
      Column::new()
        .size(800.0, 520.0)
        .child(player(&told).width(width).height(width * 9.0 / 16.0).mount(ctx))
        .into()
    });
    let first = stage.picture();
    match form {
      "large" => assert!(has(&mut stage, "v-settings"), "{width} is the large player"),
      "compact" => assert!(
        has(&mut stage, "v-more") && !has(&mut stage, "v-settings"),
        "{width} is compact"
      ),
      _ => assert!(!has(&mut stage, "v-fullscreen"), "{width} is minimal"),
    }
    stage.frame();
    assert_eq!(stage.picture(), first, "the first frame at {width} is the settled one");
  }
}

#[test]
fn given_a_width_alone_it_is_sixteen_by_nine_from_the_first_frame() {
  let told = Told::default();
  let made = told.clone();
  let mut fixed = still(move || player(&made).width(320.0));
  assert_eq!(fixed.bounds("v"), (0.0, 0.0, 320.0, 180.0));
  let mut stage = Stage::first_frame(800.0, 520.0, move |ctx| {
    Column::new()
      .size(800.0, 520.0)
      .child(
        Column::new()
          .width(480.0)
          .child(player(&told).width(Length::Fill).mount(ctx)),
      )
      .into()
  });
  assert_eq!(
    stage.bounds("v"),
    (0.0, 0.0, 480.0, 270.0),
    "as high as 16 by 9 of the width it was given"
  );
  let first = stage.picture();
  stage.frame();
  assert_eq!(stage.picture(), first, "in the first frame");
}

#[test]
fn it_keeps_the_shape_it_is_told_at_the_width_it_has() {
  let told = Told::default();
  let made = told.clone();
  // An upright picture, as a feed's: 9 by 16.
  let mut upright = still(move || player(&made).width(270.0).aspect(9.0 / 16.0));
  assert_eq!(upright.bounds("v"), (0.0, 0.0, 270.0, 480.0));
  let made = told.clone();
  let mut stage = Stage::first_frame(800.0, 900.0, move |ctx| {
    Column::new()
      .size(800.0, 900.0)
      .child(
        Column::new()
          .width(360.0)
          .child(player(&made).width(Length::Fill).aspect(4.0 / 3.0).mount(ctx)),
      )
      .into()
  });
  assert_eq!(
    stage.bounds("v"),
    (0.0, 0.0, 360.0, 270.0),
    "4 by 3 of the width it has"
  );
  // What is not a shape is not taken: it stays 16 by 9.
  let made = told.clone();
  let mut none = still(move || player(&made).width(320.0).aspect(0.0).aspect(f32::NAN));
  assert_eq!(none.bounds("v"), (0.0, 0.0, 320.0, 180.0));
}

#[test]
fn its_width_is_the_most_it_takes_where_the_app_says_and_its_shape_holds() {
  let told = Told::default();
  for (room, wide, high) in [(800.0, 432.0, 243.0), (326.0, 326.0, 183.0), (256.0, 256.0, 144.0)] {
    let made = told.clone();
    let mut stage = Stage::first_frame(800.0, 520.0, move |ctx| {
      Column::new()
        .size(800.0, 520.0)
        .child(
          Column::new()
            .width(room)
            .child(player(&made).width(432.0).max_width(Length::Fill).mount(ctx)),
        )
        .into()
    });
    assert_eq!(
      stage.bounds("v"),
      (0.0, 0.0, wide, high),
      "in a room of {room}, from the first frame"
    );
    let first = stage.picture();
    stage.frame();
    assert_eq!(
      stage.picture(),
      first,
      "the first frame in a room of {room} is the settled one"
    );
  }
}

#[test]
fn the_volume_slider_folds_in_a_narrow_large_player() {
  let time_at = |width: f32| {
    let told = Told::default();
    let mut stage = stage(move |ctx| player(&told).width(width).height(360.0).mount(ctx));
    stage.bounds("v-time").0
  };
  let (narrow, wide) = (time_at(432.0), time_at(640.0));
  assert!(
    wide - narrow >= 60.0,
    "the slider of 64 folds under 480: the time at {narrow} and {wide}"
  );
}

#[test]
fn the_app_layer_is_drawn_with_the_controls_and_without_them() {
  for controls in [true, false] {
    let told = Told::default();
    let mut stage = still(move || {
      player(&told).controls(controls).child(
        Column::new()
          .id("app")
          .size(Dimension::Pct(100.0), Dimension::Pct(100.0)),
      )
    });
    assert_eq!(
      stage.bounds("v-overlay"),
      (0.0, 0.0, 640.0, 360.0),
      "the layer is the picture's size"
    );
    assert_eq!(
      stage.bounds("app"),
      (0.0, 0.0, 640.0, 360.0),
      "what the app put in it, over the picture"
    );
    assert_eq!(
      stage.bounds("picture"),
      (0.0, 0.0, 640.0, 360.0),
      "the picture fills the player"
    );
    for id in ["v-controls", "v-play", "v-seek", "v-time", "v-fullscreen", "v-center"] {
      assert_eq!(has(&mut stage, id), controls, "{id} is drawn only with the controls");
    }
  }
}

#[test]
fn the_overlay_takes_presses_without_the_controls() {
  let pressed = Arc::new(Mutex::new(0));
  let count = pressed.clone();
  let mut stage = still(move || {
    let count = count.clone();
    VideoPlayer::new().id("v").controls(false).child(
      Column::new()
        .id("like")
        .size(40.0, 40.0)
        .on_click(move |_| *count.lock().unwrap() += 1),
    )
  });
  stage.click("like");
  assert_eq!(
    *pressed.lock().unwrap(),
    1,
    "the app's own control is pressed through the player"
  );
}

#[test]
fn with_the_controls_the_overlay_and_the_bar_each_take_their_own_presses() {
  for mount in [false, true] {
    let told = Told::default();
    let pressed = Arc::new(Mutex::new(0));
    let (made, count) = (told.clone(), pressed.clone());
    let mut stage = stage(move |ctx| {
      let count = count.clone();
      let player = player(&made).width(640.0).child(
        Column::new()
          .id("like")
          .size(40.0, 40.0)
          .on_click(move |_| *count.lock().unwrap() += 1),
      );
      if mount {
        player.mount(ctx)
      } else {
        player.build(&lurq::ui::current_theme())
      }
    });
    stage.click("like");
    assert_eq!(
      *pressed.lock().unwrap(),
      1,
      "the app's own control is pressed (mounted: {mount})"
    );
    assert_eq!(
      told.take(),
      Vec::<String>::new(),
      "and the press of it is not a press of the picture (mounted: {mount})"
    );
    stage.click("v-play");
    assert_eq!(
      told.take(),
      ["playing true"],
      "the bar's play is still the player's (mounted: {mount})"
    );
  }
}

#[test]
fn a_play_button_in_the_middle_of_the_apps_layer_is_pressed_under_a_layer_that_draws_nothing_there() {
  use lurq::{
    components::{Row, Stack},
    layout::{Alignment, layout_kind::Justify},
    node::HitTestBehavior,
    ui::{PlayButton, PlayButtonVariant},
  };
  for mount in [false, true] {
    let heard = Arc::new(Mutex::new(Vec::new()));
    let tell = heard.clone();
    let mut stage = stage(move |ctx| {
      let tell = tell.clone();
      let center = Row::new()
        .size(Dimension::Pct(100.0), Dimension::Pct(100.0))
        .align_items(Alignment::Center)
        .justify(Justify::Center)
        .child(
          PlayButton::new()
            .id("own-play")
            .variant(PlayButtonVariant::Overlay)
            .on_change(move |to| tell.lock().unwrap().push(to)),
        );
      let over = Column::new()
        .size(Dimension::Pct(100.0), Dimension::Pct(100.0))
        .hit_test(HitTestBehavior::ContentOnly)
        .justify(Justify::SpaceBetween)
        .child(Row::new().id("top").width(Dimension::Pct(100.0)).height(40.0))
        .child(Row::new().id("bottom").width(Dimension::Pct(100.0)).height(40.0));
      let layer = Stack::new()
        .size(Dimension::Pct(100.0), Dimension::Pct(100.0))
        .child(center)
        .child(over);
      let player = VideoPlayer::new()
        .id("v")
        .width(243.0)
        .height(432.0)
        .controls(false)
        .playing_controlled(false, |_| {})
        .child(layer);
      if mount {
        player.mount(ctx)
      } else {
        player.build(&lurq::ui::current_theme())
      }
    });
    stage.click("own-play");
    assert_eq!(
      *heard.lock().unwrap(),
      [true],
      "the app's play button hears one press (mounted: {mount})"
    );
  }
}

#[test]
fn a_bound_player_is_dragged_through_the_renders_its_own_change_causes() {
  use lurq::core::Signal;
  for mount in [false, true] {
    let position = Signal::new(84.0_f32);
    let shown = position.clone();
    let mut stage = stage(move |ctx| {
      let player = VideoPlayer::new()
        .id("v")
        .width(640.0)
        .media(Column::new().size(Dimension::Pct(100.0), Dimension::Pct(100.0)))
        .duration(250.0)
        .position(shown.clone());
      if mount {
        player.mount(ctx)
      } else {
        player.build(&lurq::ui::current_theme())
      }
    });
    let (x, y, width, height) = stage.bounds("v-seek");
    let (at, middle) = (|share: f32| x + width * share, y + height / 2.0);
    stage.tree.mouse_move(at(0.34), middle);
    stage.tree.mouse_down(at(0.34), middle, MouseButton::Left);
    stage.pass();
    // What plays moves the position on by itself while the handle is held: the player is built again with
    // nothing of the pointer in it, and the drag goes on.
    position.set(position.get_untracked() + 0.25);
    stage.pass();
    stage.tree.mouse_move(at(0.5), middle);
    stage.pass();
    let first = position.get_untracked();
    // And again between two moves of the pointer, as a clock does four times a second.
    position.set(first + 0.25);
    stage.pass();
    position.set(first + 0.5);
    stage.pass();
    stage.tree.mouse_move(at(0.8), middle);
    stage.pass();
    let second = position.get_untracked();
    stage.tree.mouse_up(at(0.8), middle, MouseButton::Left);
    stage.pass();
    assert!(
      (110.0..140.0).contains(&first),
      "the handle follows the pointer to the middle (mounted: {mount}): {first}"
    );
    assert!(
      second > 185.0,
      "and goes on following it after the player was built again (mounted: {mount}): {first}, then {second}"
    );
    // The keys too: the focus stays in the bar from one render to the next.
    let before = position.get_untracked();
    stage.key("ArrowRight");
    stage.key("ArrowRight");
    let after = position.get_untracked();
    assert!(
      (after - before - 10.0).abs() < 0.01,
      "two arrow keys move it two steps of 5 (mounted: {mount}): from {before} to {after}"
    );
  }
}

#[test]
fn the_volume_is_dragged_through_the_renders_its_own_change_causes() {
  use lurq::core::Signal;
  // Wide, the slider stands beside its button; under 480 it comes out under the pointer.
  for width in [640.0, 432.0] {
    let volume = Signal::new(0.5_f32);
    let shown = volume.clone();
    let mut stage = stage(move |ctx| {
      VideoPlayer::new()
        .id("v")
        .width(width)
        .media(Column::new().size(Dimension::Pct(100.0), Dimension::Pct(100.0)))
        .duration(250.0)
        .volume(shown.clone())
        .mount(ctx)
    });
    let button = stage.bounds("v-mute");
    stage.hover("v-mute");
    stage.pass();
    // The slider of 64 after the button and its gap of 4.
    let (from, middle) = (button.0 + button.2 + 4.0, button.1 + button.3 / 2.0);
    stage.tree.mouse_move(from + 32.0, middle);
    stage.tree.mouse_down(from + 32.0, middle, MouseButton::Left);
    stage.pass();
    stage.tree.mouse_move(from + 16.0, middle);
    stage.pass();
    let first = volume.get_untracked();
    stage.tree.mouse_move(from + 58.0, middle - 40.0);
    stage.pass();
    let second = volume.get_untracked();
    stage.tree.mouse_up(from + 58.0, middle - 40.0, MouseButton::Left);
    stage.pass();
    assert!(
      (0.1..0.4).contains(&first),
      "the thumb follows the pointer down (at {width}): {first}"
    );
    assert!(
      second > 0.8,
      "and goes on following it after the player was built again (at {width}): {first}, then {second}"
    );
  }
}

#[test]
fn playing_has_no_disc_and_waiting_a_spinner() {
  let told = Told::default();
  let made = told.clone();
  let mut playing = still(move || player(&made).playing_controlled(true, |_| {}));
  assert!(!has(&mut playing, "v-center"), "no disc while it plays");
  says(&mut playing, "v-play", "a11y_name", "Pause");
  let mut waiting = still(move || player(&told).playing_controlled(true, |_| {}).waiting(true));
  assert!(!has(&mut waiting, "v-center"), "a spinner, not the disc");
  assert!(has(&mut waiting, "v-controls"), "the bar stays while it waits");
}

#[test]
fn ended_has_the_veil_and_replay() {
  let told = Told::default();
  let made = told.clone();
  let mut stage = still(move || player(&made).ended(true));
  assert_eq!(rects(&stage, BLACK_60, 640.0, 360.0), 1, "the veil of black at 60%");
  assert!(has(&mut stage, "v-center"), "replay in the middle");
  says(&mut stage, "v-play", "a11y_name", "Replay");
  stage.click("v-play");
  assert_eq!(told.take(), ["playing true"], "replay asks to play");
}

#[test]
fn an_error_has_its_title_its_line_and_its_button_and_no_bar() {
  let told = Told::default();
  let made = told.clone();
  let mut stage = still(move || {
    player(&made)
      .error("This video can't be played")
      .error_description("Check the connection and try again.")
  });
  assert!(stage.paints("This video can't be played"));
  assert!(stage.paints("Check the connection and try again."));
  assert_eq!(rects(&stage, BLACK_60, 640.0, 360.0), 1, "the veil");
  assert!(!has(&mut stage, "v-controls"), "no bar");
  let (_, _, _, height) = stage.bounds("v-retry");
  assert_eq!(height, 32.0, "a button 32 high");
  assert_eq!(
    stage.fill_color("v-retry"),
    Some(Color::new(255, 255, 255, 255)),
    "white"
  );
  stage.click("v-retry");
  assert_eq!(told.take(), ["retry"]);
}

#[test]
fn live_has_its_badge_and_no_speed() {
  let told = Told::default();
  let mut stage = still(move || {
    VideoPlayer::new().id("v").live(true).playing_controlled(true, {
      let say = told.say();
      move |to| say(format!("playing {to}"))
    })
  });
  assert!(stage.paints("LIVE"), "the badge for the time");
  assert!(!has(&mut stage, "v-speed"), "no speed");
  assert!(!stage.paints("-:--"), "no time");
}

#[test]
fn the_poster_has_its_length_and_no_bar() {
  let told = Told::default();
  let mut stage = still(move || {
    player(&told).position_controlled(0.0, |_| {}).poster(
      Column::new()
        .id("poster")
        .size(Dimension::Pct(100.0), Dimension::Pct(100.0)),
    )
  });
  assert!(has(&mut stage, "poster"), "the poster over the picture");
  assert!(!has(&mut stage, "v-controls"), "no bar before the first play");
  assert!(has(&mut stage, "v-center"), "the disc to play it");
  let chip = stage.text_quad("4:10").expect("the length on the poster");
  assert!(640.0 - (chip.x + chip.width) >= 12.0, "12 and its padding from the end");
  assert_eq!(
    rects(&stage, BLACK_70, chip.width + 12.0, chip.height + 4.0),
    1,
    "in a box padded 2 6"
  );
}

#[test]
fn captions_stand_over_the_bar_and_their_button_is_on() {
  let told = Told::default();
  let mut stage = still(move || {
    player(&told)
      .captions_controlled(true, |_| {})
      .caption_text("The bar fades out while the video plays.")
  });
  let (x, y, width, height) = stage.bounds("v-caption");
  assert_eq!(y + height, 272.0 - 12.0, "12 over the bar");
  assert!(((x + width / 2.0) - 320.0).abs() < 1.0, "centered");
  assert_eq!(stage.fill_color("v-caption"), Some(BLACK_70));
  let (color, size) = stage.text("The bar fades out while the video plays.");
  assert_eq!((color, size), (Color::new(255, 255, 255, 255), 16.0));
  assert_eq!(stage.fill_color("v-captions"), Some(WHITE_20), "the button is on");
  says(&mut stage, "v-captions", "a11y_pressed", "true");
}

#[test]
fn muted_and_fullscreen_name_what_a_press_would_do() {
  let told = Told::default();
  let mut stage = still(move || {
    player(&told)
      .size(VideoPlayerSize::Md)
      .muted_controlled(true, |_| {})
      .fullscreen_controlled(true, |_| {})
  });
  says(&mut stage, "v-mute", "a11y_name", "Unmute");
  says(&mut stage, "v-fullscreen", "a11y_name", "Exit fullscreen");
}

#[test]
fn a_press_tells_what_it_is_to_be_and_changes_nothing() {
  let told = Told::default();
  let made = told.clone();
  let mut stage = still(move || player(&made));
  stage.click("v-play");
  assert_eq!(told.take(), ["playing true"]);
  assert!(has(&mut stage, "v-center"), "still paused until the app says");
  stage.click("v-center");
  assert_eq!(told.take(), ["playing true"]);
  stage.click("v-captions");
  assert_eq!(told.take(), ["captions true"]);
  stage.click("v-pip");
  assert_eq!(told.take(), ["picture in picture true"]);
  stage.click("v-fullscreen");
  assert_eq!(told.take(), ["fullscreen true"]);
  stage.click("v-mute");
  assert_eq!(told.take(), ["muted true"]);
}

#[test]
fn bound_the_signal_is_set_before_the_handler() {
  let playing = lurq::core::Signal::new(false);
  let (bound, seen) = (playing.clone(), Arc::new(Mutex::new(None)));
  let saw = seen.clone();
  let read = playing.clone();
  let mut stage = still(move || {
    let (saw, read) = (saw.clone(), read.clone());
    VideoPlayer::new()
      .id("v")
      .duration(10.0)
      .playing(bound.clone())
      .on_playing_change(move |_| *saw.lock().unwrap() = Some(read.get_untracked()))
  });
  stage.click("v-play");
  assert!(playing.get_untracked(), "the press set the signal");
  assert_eq!(*seen.lock().unwrap(), Some(true), "the handler saw it set");
}

#[test]
fn each_key_tells_what_it_does() {
  let told = Told::default();
  let made = told.clone();
  let mut stage = mounted(move || player(&made));
  // A press of the picture plays, and gives the player the focus.
  press_at(&mut stage, 100.0, 60.0);
  assert_eq!(told.take(), ["playing true"]);
  let keys: [(&str, &[&str]); 12] = [
    ("k", &["playing true"]),
    (" ", &["playing true"]),
    ("ArrowRight", &["position 89", "position end 89"]),
    ("ArrowLeft", &["position 79", "position end 79"]),
    ("l", &["position 94", "position end 94"]),
    ("j", &["position 74", "position end 74"]),
    ("Home", &["position 0", "position end 0"]),
    ("End", &["position 250", "position end 250"]),
    ("ArrowUp", &["volume 0.65"]),
    ("m", &["muted true"]),
    ("c", &["captions true"]),
    ("f", &["fullscreen true"]),
  ];
  for (key, then) in keys {
    stage.key(key);
    assert_eq!(told.take(), then, "{key:?}");
  }
}

#[test]
fn the_settings_menu_hangs_over_the_seek_bar_at_the_bars_end() {
  let told = Told::default();
  let made = told.clone();
  let mut stage = mounted(move || player(&made).captions_track("English"));
  stage.click("v-settings");
  for row in ["Quality", "Speed", "Captions"] {
    assert!(stage.paints(row), "the settings hold {row}");
  }
  for value in ["Auto", "Normal", "Off"] {
    assert!(stage.paints(value), "and what it is: {value}");
  }
  let (x, y, width, height) = stage.bounds("v-settings-menu");
  assert_eq!(width, 240.0, "240 wide");
  assert_eq!(x + width, 628.0, "its end on the bar's padding");
  assert_eq!(y + height, 316.0 - 24.0, "4 over the seek bar");
  assert_eq!(
    stage.fill_color("v-settings"),
    Some(WHITE_20),
    "its button is on while it is open"
  );
  assert!(told.take().is_empty(), "opening a menu tells nothing");
}

#[test]
fn a_speed_chosen_is_told() {
  let told = Told::default();
  let made = told.clone();
  let mut stage = mounted(move || player(&made));
  stage.click("v-speed");
  assert!(stage.paints("Playback speed"));
  let (x, y) = {
    let quad = stage.text_quad("1.5×").expect("the speeds");
    (quad.x + quad.width / 2.0, quad.y + quad.height / 2.0)
  };
  press_at(&mut stage, x, y);
  assert_eq!(told.take(), ["rate 1.5"]);
  assert!(!stage.paints("Playback speed"), "the menu closes on a choice");
}

#[test]
fn a_forced_menu_is_drawn_open() {
  let told = Told::default();
  let mut stage = still(move || {
    player(&told)
      .size(VideoPlayerSize::Md)
      .force_menu(Some(VideoPlayerMenu::More))
  });
  for row in ["Captions", "Speed", "Quality", "Picture in picture"] {
    assert!(stage.paints(row), "the compact menu holds {row}");
  }
  let (_, _, width, _) = stage.bounds("v-more-menu");
  assert_eq!(width, 200.0);
}

#[test]
fn playing_the_controls_fade_after_the_pointer_rests_and_come_back_when_it_moves() {
  let told = Told::default();
  let made = told.clone();
  let mut stage = mounted(move || {
    player(&made)
      .playing_controlled(true, |_| {})
      .captions_controlled(true, |_| {})
      .caption_text("A line")
  });
  stage.tree.mouse_move(100.0, 60.0);
  stage.pass();
  assert_eq!(bar_opacity(&mut stage), 1.0, "shown while the pointer moves");
  assert_eq!(
    stage.tree.cursor(),
    CursorIcon::Pointer,
    "the picture is pressed to play"
  );
  stage
    .tree
    .tick_timers_at(Instant::now() + VIDEO_PLAYER_HIDE_AFTER + Duration::from_millis(100));
  stage.pass();
  assert_eq!(bar_opacity(&mut stage), 0.0, "faded 3 s after the pointer rests");
  assert_eq!(stage.tree.cursor(), CursorIcon::None, "and the pointer hidden");
  let (_, y, _, height) = stage.bounds("v-caption");
  assert!(
    (y + height - (360.0 - 12.0)).abs() < 1.0,
    "the captions go down to 12 over the bottom"
  );
  stage.tree.mouse_move(120.0, 80.0);
  stage.pass();
  assert_eq!(bar_opacity(&mut stage), 1.0, "back as the pointer moves");
  assert_eq!(stage.tree.cursor(), CursorIcon::Pointer, "with the pointer");
}

#[test]
fn paused_the_controls_stay() {
  let told = Told::default();
  let made = told.clone();
  let mut stage = mounted(move || player(&made));
  stage.tree.mouse_move(100.0, 60.0);
  stage.pass();
  stage
    .tree
    .tick_timers_at(Instant::now() + VIDEO_PLAYER_HIDE_AFTER + Duration::from_secs(1));
  stage.pass();
  assert_eq!(bar_opacity(&mut stage), 1.0);
  assert_eq!(stage.tree.cursor(), CursorIcon::Pointer);
}

#[test]
fn hidden_by_force_the_captions_sit_at_the_bottom() {
  let told = Told::default();
  let mut stage = still(move || {
    player(&told)
      .playing_controlled(true, |_| {})
      .captions_controlled(true, |_| {})
      .caption_text("A line")
      .force_controls_shown(Some(false))
  });
  assert_eq!(bar_opacity(&mut stage), 0.0);
  let (_, y, _, height) = stage.bounds("v-caption");
  assert!((y + height - (360.0 - 12.0)).abs() < 1.0);
}

#[test]
fn over_the_picture_it_is_the_same_in_a_dark_theme() {
  let told = Told::default();
  let dark = Theme::new();
  dark.set_preferred_appearance(Some(Appearance::Dark));
  let mut stage = stage(move |ctx| {
    let player = player(&told).captions_controlled(true, |_| {}).caption_text("A line");
    ctx.in_theme(&dark, move |ctx| player.mount(ctx))
  });
  assert_eq!(stage.fill_color("v-caption"), Some(BLACK_70));
  assert_eq!(stage.fill_color("v-captions"), Some(WHITE_20));
}

#[test]
fn a_theme_rule_changes_it() {
  let told = Told::default();
  let mut stage = still(move || player(&told).style(VideoPlayerPart::Controls, |s| s.height(100.0)));
  let (_, y, _, height) = stage.bounds("v-controls");
  assert_eq!((y, height), (260.0, 100.0));
}

/// The point in the middle of the text `content` as painted.
fn on_text(stage: &Stage, content: &str) -> (f32, f32) {
  let quad = stage
    .text_quad(content)
    .unwrap_or_else(|| panic!("{content:?} is painted"));
  (quad.x + quad.width / 2.0, quad.y + quad.height / 2.0)
}

#[test]
fn a_row_of_the_settings_shows_its_choices_in_the_menus_place() {
  let told = Told::default();
  let made = told.clone();
  let mut stage = mounted(move || player(&made).captions_track("English"));
  stage.click("v-settings");
  let (x, y) = on_text(&stage, "Quality");
  press_at(&mut stage, x, y);
  // The page: a row back with the title, then the choices, in the same panel.
  for choice in ["Quality", "Auto", "1080p", "720p"] {
    assert!(stage.paints(choice), "the page holds {choice}");
  }
  assert!(!stage.paints("Speed"), "the menu's own rows give way to the page");
  let (_, _, width, _) = stage.bounds("v-settings-menu");
  assert_eq!(width, 240.0, "in the same panel");
  assert!(told.take().is_empty(), "opening a page tells nothing");
  // The row back returns to the menu's own rows.
  let (x, y) = on_text(&stage, "Quality");
  press_at(&mut stage, x, y);
  assert!(
    stage.paints("Speed") && stage.paints("Captions"),
    "back to the settings"
  );
  // A choice of a page is told, and closes the menu.
  let (x, y) = on_text(&stage, "Captions");
  press_at(&mut stage, x, y);
  let (x, y) = on_text(&stage, "English");
  press_at(&mut stage, x, y);
  assert_eq!(told.take(), ["captions true"]);
  assert!(!has(&mut stage, "v-settings-menu"), "the menu closes on a choice");
  // It opens again at its own rows.
  stage.click("v-settings");
  assert!(stage.paints("Speed"), "a closed menu opens at its own rows");
}

#[test]
fn a_second_press_of_the_picture_asks_for_fullscreen() {
  let told = Told::default();
  let made = told.clone();
  let mut stage = mounted(move || player(&made));
  press_at(&mut stage, 100.0, 60.0);
  assert_eq!(told.take(), ["playing true"], "a press plays at once");
  press_at(&mut stage, 100.0, 60.0);
  assert_eq!(told.take(), ["playing true", "fullscreen true"]);
}

#[test]
fn the_seek_bar_shows_the_frame_the_app_gives_under_the_pointer() {
  let told = Told::default();
  let mut stage = mounted(move || {
    player(&told).preview(|_| {
      Column::new()
        .id("frame")
        .size(Dimension::Pct(100.0), Dimension::Pct(100.0))
        .into()
    })
  });
  let (x, y, width, height) = stage.bounds("v-seek");
  stage.tree.mouse_move(x + width * 0.4, y + height / 2.0);
  stage.pass();
  let (_, _, width, height) = stage.bounds("frame");
  assert_eq!((width, height), (128.0, 72.0), "the frame of 128 by 72 over the time");
}

#[test]
fn the_frame_shows_with_the_pointer_anywhere_on_the_seek_bar_its_middle_too() {
  for across in [640.0_f32, 432.0, 320.0] {
    for share in [0.1_f32, 0.5, 0.9] {
      let told = Told::default();
      let mut stage = mounted(move || {
        player(&told).width(across).preview(|_| {
          Column::new()
            .id("frame")
            .size(Dimension::Pct(100.0), Dimension::Pct(100.0))
            .into()
        })
      });
      let (x, y, width, height) = stage.bounds("v-seek");
      stage.tree.mouse_move(x + width * share, y + height / 2.0);
      stage.pass();
      assert!(
        has(&mut stage, "frame"),
        "the frame with the pointer at {share} of the bar of a player {across} wide"
      );
    }
  }
}

/// The controls of a bar by their ids, whichever of them the form in use has.
const BAR: [&str; 9] = [
  "v-play",
  "v-mute",
  "v-time",
  "v-captions",
  "v-speed",
  "v-more",
  "v-settings",
  "v-pip",
  "v-fullscreen",
];

#[test]
fn the_controls_of_the_large_bar_stand_40_apart() {
  let told = Told::default();
  let mut stage = still(move || player(&told).width(640.0));
  let x = |stage: &mut Stage, id: &str| stage.bounds(id).0;
  let (settings, pip, fullscreen) = (
    x(&mut stage, "v-settings"),
    x(&mut stage, "v-pip"),
    x(&mut stage, "v-fullscreen"),
  );
  assert_eq!(
    (pip - settings, fullscreen - pip),
    (40.0, 40.0),
    "a control of 36 and a gap of 4"
  );
  let (left, _, width, _) = stage.bounds("v");
  let (_, _, side, _) = stage.bounds("v-fullscreen");
  assert_eq!(
    left + width - (fullscreen + side),
    12.0,
    "the last control ends at the bar's padding of 12"
  );
}

#[test]
fn a_folded_volume_shows_its_slider_under_the_pointer_in_the_times_place() {
  let told = Told::default();
  let made = told.clone();
  let mut stage = mounted(move || player(&made).width(432.0));
  // The id is the button's: the slider comes out after it.
  let button = stage.bounds("v-mute");
  let time = stage.bounds("v-time");
  assert!(
    time.0 - (button.0 + button.2) < 20.0,
    "folded, the time stands right after the button: at {}",
    time.0
  );
  // The pointer over the button: the slider comes out beside it, and the time gives it its room.
  stage.hover("v-mute");
  stage.pass();
  assert!(
    !has(&mut stage, "v-time"),
    "the time is not drawn while the slider is out"
  );
  let (x, _, width, _) = stage.bounds("v");
  for id in ["v-captions", "v-speed", "v-settings", "v-pip", "v-fullscreen"] {
    let (at, _, wide, _) = stage.bounds(id);
    assert!(
      at >= button.0 + button.2 + 64.0 && at + wide <= x + width,
      "{id} at {at} is after the slider of 64 and in the player"
    );
  }
  // A drag of the slider tells the volume, and the slider stays out while it is dragged past the control.
  let (along, middle) = (button.0 + button.2 + 4.0 + 44.0, button.1 + button.3 / 2.0);
  stage.tree.mouse_move(along, middle);
  stage.tree.mouse_down(along, middle, MouseButton::Left);
  stage.pass();
  stage.tree.mouse_move(along - 30.0, middle - 60.0);
  stage.pass();
  assert!(
    told.take().iter().any(|what| what.starts_with("volume ")),
    "the drag tells the volume"
  );
  assert!(!has(&mut stage, "v-time"), "dragged, the slider stays out");
  stage.tree.mouse_up(along - 30.0, middle - 60.0, MouseButton::Left);
  stage.tree.mouse_move(along + 200.0, middle - 120.0);
  stage.pass();
  assert!(has(&mut stage, "v-time"), "let go and left, the time is back");
  assert_eq!(stage.bounds("v-time"), time);
}

#[test]
fn a_still_player_folds_its_volume_by_the_width_it_was_given() {
  let time_at = |width: f32| {
    let told = Told::default();
    let mut stage = still(move || player(&told).width(width));
    stage.bounds("v-time").0
  };
  let (narrow, wide) = (time_at(432.0), time_at(640.0));
  assert!(
    wide - narrow >= 60.0,
    "not mounted, a player 432 wide has no slider: the time at {narrow} and {wide}"
  );
}

#[test]
fn nothing_of_the_bar_is_cut_at_any_width_still_or_mounted() {
  for mount in [false, true] {
    for step in 0..=26 {
      let width = 200.0 + 20.0 * step as f32;
      let told = Told::default();
      let mut stage = stage(move |ctx| {
        let player = player(&told).width(width);
        if mount {
          player.mount(ctx)
        } else {
          player.build(&lurq::ui::current_theme())
        }
      });
      let (left, _, across, _) = stage.bounds("v");
      assert_eq!(across, width, "the player is as wide as it was told");
      let mut found = Vec::new();
      for id in BAR {
        if has(&mut stage, id) {
          let (x, _, w, _) = stage.bounds(id);
          found.push((id, x, w));
        }
      }
      // The minimal form has play alone of these, beside its seek bar.
      assert!(!found.is_empty(), "a bar at {width}");
      found.sort_by(|a, b| a.1.total_cmp(&b.1));
      for (id, x, w) in &found {
        assert!(
          *x >= left - 0.5 && x + w <= left + width + 0.5,
          "{id} is inside a player {width} wide (mounted: {mount}): from {x}, {w} wide"
        );
      }
      for pair in found.windows(2) {
        let ((before, x, w), (after, next, _)) = (pair[0], pair[1]);
        assert!(
          next >= x + w - 0.5,
          "{after} does not lie over {before} in a player {width} wide (mounted: {mount})"
        );
      }
    }
  }
}

#[test]
fn it_shows_what_a_track_of_captions_says_at_its_position() {
  use lurq::ui::MediaCaptions;
  let track =
    MediaCaptions::parse("WEBVTT\n\n01:20.000 --> 01:30.000\nThe bar fades out.\n\n02:00.000 --> 02:05.000\nLater.\n");
  let told = Told::default();
  // The player is at 1:24.
  let (made, cues) = (told.clone(), track.clone());
  let mut on = still(move || player(&made).captions_controlled(true, |_| {}).caption_cues(&cues));
  assert!(on.paints("The bar fades out."), "the cue of its position");
  assert!(!on.paints("Later."));
  let (made, cues) = (told.clone(), track.clone());
  let mut off = still(move || player(&made).caption_cues(&cues));
  assert!(!off.paints("The bar fades out."), "off, nothing is said");
  let (made, cues) = (told.clone(), track.clone());
  let mut between = still(move || {
    player(&made)
      .captions_controlled(true, |_| {})
      .position_controlled(100.0, |_| {})
      .caption_cues(&cues)
  });
  assert!(!has(&mut between, "v-caption"), "between two cues there is no line");
  let mut own = still(move || {
    player(&told)
      .captions_controlled(true, |_| {})
      .caption_cues(&track)
      .caption_text("The app's own.")
  });
  assert!(
    own.paints("The app's own.") && !own.paints("The bar fades out."),
    "a line the app gives is shown in the track's place"
  );
}

#[test]
fn in_fullscreen_it_fills_its_window_and_keeps_its_room_where_it_stood() {
  use lurq::{core::Signal, ui::VideoFullscreen};
  let full = Signal::new(false);
  let shown = full.clone();
  let mut stage = stage(move |ctx| {
    Column::new()
      .child(
        VideoPlayer::new()
          .id("v")
          .width(320.0)
          .media(Column::new().size(Dimension::Pct(100.0), Dimension::Pct(100.0)))
          .duration(250.0)
          .fullscreen(shown.clone())
          .mount(ctx),
      )
      .child(Column::new().id("after").size(10.0, 10.0))
      .into()
  });
  assert_eq!(stage.bounds("v"), (0.0, 0.0, 320.0, 180.0));
  assert!(!has(&mut stage, "v-captions"), "compact at 320");
  let after = stage.bounds("after");
  stage.click("v-fullscreen");
  stage.pass();
  assert!(full.get(), "the button asks for it");
  assert_eq!(
    stage.bounds("v"),
    (0.0, 0.0, 800.0, 520.0),
    "it is as large as its window"
  );
  assert_eq!(stage.bounds("after"), after, "what stands after it has not moved");
  assert!(has(&mut stage, "v-captions"), "in the form of the width it has now");
  // Escape leaves, and it is back where it stood, as large as it was.
  stage.key("Escape");
  stage.pass();
  assert!(!full.get(), "Escape asks to leave");
  assert_eq!(stage.bounds("v"), (0.0, 0.0, 320.0, 180.0));
  assert!(!has(&mut stage, "v-captions"));
  assert_eq!(stage.bounds("after"), after);

  // An app that lays fullscreen out itself: the player is drawn where it stands, its button saying "exit".
  let mut own = mounted(|| {
    VideoPlayer::new()
      .id("v")
      .width(432.0)
      .duration(250.0)
      .fullscreen_controlled(true, |_| {})
      .fullscreen_fills(VideoFullscreen::App)
  });
  assert_eq!(own.bounds("v"), (0.0, 0.0, 432.0, 243.0));
  says(&mut own, "v-fullscreen", "a11y_name", "Exit fullscreen");
  // Filling its window alone, from its first frame.
  let mut window = mounted(|| {
    VideoPlayer::new()
      .id("v")
      .width(432.0)
      .duration(250.0)
      .fullscreen_controlled(true, |_| {})
      .fullscreen_fills(VideoFullscreen::Window)
  });
  assert_eq!(window.bounds("v"), (0.0, 0.0, 800.0, 520.0));
}

#[test]
fn in_picture_in_picture_it_says_so_where_it_stood_and_a_button_brings_it_back() {
  let told = Told::default();
  let made = told.clone();
  let mut away = mounted(move || {
    // The handler of `player` goes on telling what is asked of it.
    player(&made).width(432.0).picture_in_picture_controlled(true, |_| {})
  });
  assert_eq!(away.bounds("v"), (0.0, 0.0, 432.0, 243.0), "it keeps its room");
  assert!(away.paints("Playing in picture in picture"));
  assert!(
    !has(&mut away, "v-play") && !has(&mut away, "v-controls"),
    "its controls went with it"
  );
  assert!(!has(&mut away, "picture"), "and its picture");
  told.take();
  away.click("v-back");
  assert_eq!(told.take(), ["picture in picture false"]);
  // An app that makes the window itself: the player is drawn where it stands, its button on.
  let made = told.clone();
  let mut own = mounted(move || {
    player(&made)
      .width(640.0)
      .picture_in_picture_controlled(true, |_| {})
      .picture_in_picture_window(false)
  });
  assert!(has(&mut own, "v-play") && has(&mut own, "picture"));
  assert!(!own.paints("Playing in picture in picture"));
  says(&mut own, "v-pip", "a11y_pressed", "true");
  // Not mounted, it has no window to open.
  let mut still_one = still(move || player(&told).picture_in_picture_controlled(true, |_| {}));
  assert!(has(&mut still_one, "v-play"));
}

/// Whether two measures are the same to half a pixel.
fn near(one: f32, other: f32) -> bool {
  (one - other).abs() <= 0.5
}

/// The touch form of [`player`], as wide as the board's phone.
fn touch(told: &Told) -> VideoPlayer {
  player(told).width(360.0).touch(true)
}

#[test]
fn the_touch_form_is_the_boards() {
  let told = Told::default();
  let made = told.clone();
  let mut stage = mounted(move || touch(&made));
  assert_eq!(stage.bounds("v"), (0.0, 0.0, 360.0, 203.0));
  // The top row: the captions and the settings, 44 each, 4 from the end. Nothing leaves it: the app does not listen.
  assert_eq!(stage.bounds("v-settings"), (312.0, 0.0, 44.0, 44.0));
  assert_eq!(stage.bounds("v-captions"), (268.0, 0.0, 44.0, 44.0));
  assert!(!has(&mut stage, "v-back"));
  // The middle: a play of 56 with ten seconds back and on, 44, 32 from it.
  let (disc, rewind, forward) = (
    stage.bounds("v-play"),
    stage.bounds("v-rewind"),
    stage.bounds("v-forward"),
  );
  assert_eq!((disc.2, disc.3), (56.0, 56.0));
  assert!(
    near(disc.0 + 28.0, 180.0) && near(disc.1 + 28.0, 101.5),
    "in the middle: {disc:?}"
  );
  assert_eq!((rewind.2, rewind.3, forward.2, forward.3), (44.0, 44.0, 44.0, 44.0));
  assert!(near(rewind.0 + 44.0 + 32.0, disc.0) && near(disc.0 + 56.0 + 32.0, forward.0));
  // The bottom: the time 16 from the start, fullscreen 44 at the end, and the seek bar 16 from the sides, 6 up.
  assert_eq!(stage.bounds("v-fullscreen").2, 44.0);
  assert!(near(stage.bounds("v-fullscreen").0, 312.0));
  let seek = stage.bounds("v-seek");
  assert!(
    near(seek.0, 16.0) && near(seek.2, 328.0) && near(seek.1 + seek.3 + 6.0, 203.0),
    "{seek:?}"
  );
  assert!(!has(&mut stage, "v-mute"), "a phone has keys for the volume");
  // What they tell.
  told.take();
  stage.click("v-rewind");
  stage.click("v-forward");
  stage.click("v-play");
  assert_eq!(
    told.take(),
    [
      "position 74",
      "position end 74",
      "position 94",
      "position end 94",
      "playing true"
    ]
  );
}

#[test]
fn a_tap_shows_and_hides_the_touch_forms_controls_and_two_on_a_side_seek() {
  let told = Told::default();
  let made = told.clone();
  let mut stage = mounted(move || touch(&made));
  told.take();
  // A tap on the picture, in its middle third and over no control: the controls go, and come back with the next.
  press_at(&mut stage, 180.0, 60.0);
  assert!(!has(&mut stage, "v-play") && !has(&mut stage, "v-settings") && !has(&mut stage, "v-seek"));
  press_at(&mut stage, 180.0, 60.0);
  assert!(has(&mut stage, "v-play") && has(&mut stage, "v-settings"));
  assert!(told.take().is_empty(), "a tap in the middle tells nothing");
  // Two on the right third: ten seconds on, the mark, and no control.
  press_at(&mut stage, 330.0, 60.0);
  press_at(&mut stage, 330.0, 60.0);
  assert_eq!(told.take(), ["position 94", "position end 94"]);
  assert!(stage.paints("10 seconds"));
  assert!(!has(&mut stage, "v-play"), "nothing but the mark shows");
  // A third adds up.
  press_at(&mut stage, 330.0, 60.0);
  assert!(stage.paints("20 seconds"));
  told.take();
  // The other side starts again.
  press_at(&mut stage, 30.0, 60.0);
  press_at(&mut stage, 30.0, 60.0);
  assert_eq!(told.take(), ["position 74", "position end 74"]);
  assert!(stage.paints("10 seconds") && !stage.paints("20 seconds"));
}

#[test]
fn on_its_side_the_touch_form_has_its_title_its_speed_and_a_way_back() {
  use lurq::ui::VideoFullscreen;
  let told = Told::default();
  let made = told.clone();
  let side = move |made: &Told| {
    touch(made)
      .width(780.0)
      .aspect(780.0 / 360.0)
      .title("Night Drive")
      .description("Aurora Lane")
      .fullscreen_controlled(true, {
        let say = made.say();
        move |to| say(format!("fullscreen {to}"))
      })
      .fullscreen_fills(VideoFullscreen::App)
  };
  let mut stage = mounted({
    let side = side.clone();
    move || side(&made)
  });
  assert_eq!(stage.bounds("v"), (0.0, 0.0, 780.0, 360.0));
  assert!(stage.paints("Night Drive") && stage.paints("Aurora Lane"));
  // 44 at each side for the notch.
  assert!(near(stage.bounds("v-back").0, 44.0));
  assert!(near(stage.bounds("v-settings").0 + 44.0, 780.0 - 44.0));
  let (disc, rewind) = (stage.bounds("v-play"), stage.bounds("v-rewind"));
  assert_eq!((disc.2, rewind.2), (72.0, 48.0));
  assert!(near(rewind.0 + 48.0 + 56.0, disc.0));
  assert!(has(&mut stage, "v-speed"), "the speed is in its bottom row");
  let seek = stage.bounds("v-seek");
  assert!(near(seek.0, 56.0) && near(seek.2, 780.0 - 112.0));
  // The way back leaves fullscreen where the app does not listen for it.
  told.take();
  stage.click("v-back");
  assert!(told.take().iter().any(|what| what == "fullscreen false"));
  let heard = told.clone();
  let mut own = mounted(move || {
    let say = heard.say();
    side(&heard).on_back(move || say("back".to_owned()))
  });
  told.take();
  own.click("v-back");
  assert_eq!(told.take(), ["back"]);
}

/// Renders `stage` again until `is` holds, for at most ten seconds: a media works on a thread of its own.
#[cfg(all(windows, feature = "media"))]
#[track_caller]
fn until(stage: &mut Stage, what: &str, mut is: impl FnMut(&mut Stage) -> bool) {
  let started = std::time::Instant::now();
  loop {
    // The player's clock is a timer of its window, which a window with no loop does not run by itself.
    stage.tree.tick_timers_at(std::time::Instant::now());
    stage.pass();
    if is(stage) {
      return;
    }
    assert!(
      started.elapsed() < std::time::Duration::from_secs(10),
      "it never came to: {what}"
    );
    std::thread::sleep(std::time::Duration::from_millis(20));
  }
}

#[cfg(all(windows, feature = "media"))]
#[test]
fn given_a_source_it_plays_the_file_and_its_controls_move_the_video() {
  use lurq::resources::Source;
  let clip = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../gallery/assets/media/sample.mp4");
  let told = Told::default();
  let said = told.clone();
  let mut stage = mounted(move || {
    let say = said.say();
    // Silenced: the clip has a tone, and a run of the tests is not heard.
    VideoPlayer::new()
      .id("v")
      .width(640.0)
      .source(&Source::file(&clip))
      .muted_controlled(true, |_| {})
      .on_playing_change(move |to| say(format!("playing {to}")))
  });
  // It opened: its length is the file's, and its picture is the media's.
  until(&mut stage, "its length", |stage| stage.paints("0:15"));
  assert!(stage.paints("0:00"));
  says(&mut stage, "v-play", "a11y_name", "Play");
  // Play: the app hears of it, and the video goes, by its own clock.
  stage.click("v-play");
  assert_eq!(told.take(), ["playing true"]);
  until(&mut stage, "playing", |stage| {
    has(stage, "v-play") && stage.paints("0:01")
  });
  says(&mut stage, "v-play", "a11y_name", "Pause");
  // Pause: it stands where it is.
  stage.click("v-play");
  assert_eq!(told.take(), ["playing false"]);
  stage.pass();
  says(&mut stage, "v-play", "a11y_name", "Play");
  std::thread::sleep(std::time::Duration::from_millis(1200));
  stage.pass();
  assert!(stage.paints("0:01"), "paused, the time stands");
  // What is no video says so, with the reason under it and nothing to play.
  let mut broken = mounted(|| {
    VideoPlayer::new()
      .id("v")
      .width(640.0)
      .source(&Source::bytes(b"not a video".to_vec()))
  });
  until(&mut broken, "the error", |stage| {
    stage.paints("This video can't be played")
  });
  assert!(!has(&mut broken, "v-play"));
}

#[test]
fn given_to_a_tree_as_it_is_it_is_its_component_with_the_form_of_its_width_and_its_menus() {
  for (width, form) in [(640.0, "large"), (320.0, "compact"), (200.0, "minimal")] {
    let told = Told::default();
    // No `mount` and no `Ctx`: the player is a child as any element is.
    let mut stage = Stage::first_frame(800.0, 520.0, move |_| {
      Column::new()
        .size(800.0, 520.0)
        .child(player(&told).width(width).height(width * 9.0 / 16.0))
        .into()
    });
    let first = stage.picture();
    match form {
      "large" => assert!(has(&mut stage, "v-settings"), "{width} is the large player"),
      "compact" => assert!(
        has(&mut stage, "v-more") && !has(&mut stage, "v-settings"),
        "{width} is compact"
      ),
      _ => assert!(!has(&mut stage, "v-fullscreen"), "{width} is minimal"),
    }
    stage.frame();
    assert_eq!(stage.picture(), first, "the first frame at {width} is the settled one");
  }

  // Its menus are kept between renders: a press of settings opens its panel.
  let told = Told::default();
  let mut stage = stage(move |_| player(&told).width(640.0).height(360.0).into());
  assert!(!has(&mut stage, "v-settings-menu"));
  stage.click("v-settings");
  assert!(has(&mut stage, "v-settings-menu"), "the menu a mounted player keeps");
}
