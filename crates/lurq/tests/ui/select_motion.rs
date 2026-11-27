//! What a select's list plays as it opens and as it closes: the panel's `enter` and `exit`, on the side of the trigger
//! it opened on, as a menu's and a popover's panel play theirs. The design gives the list none; a theme may.

use std::time::Duration;

use lurq::{
  animation::{Keyframes, KeyframesId},
  app::{
    App,
    events::MouseButton,
    theme::{MotionDuration, MotionEasing},
  },
  components::Column,
  core::Signal,
  node::padding::Padding,
  ui::{FADE_OUT, PartAnimation, Select, SelectPart, TooltipSide},
};

use super::stage::Stage;

const DOWN: KeyframesId = KeyframesId::new(9);
const UP: KeyframesId = KeyframesId::new(10);

/// An app whose select's list comes down 4 to its place under its trigger and up 4 to it over its trigger, fading
/// out as it closes.
fn moving() -> App {
  let app = App::new();
  let theme = app.theme().clone();
  for (id, from) in [(DOWN, -4.0), (UP, 4.0)] {
    theme.set_keyframes(
      Keyframes::new(id)
        .frame(0.0, move |k| {
          k.translate(0.0, from);
        })
        .frame(1.0, |k| {
          k.translate(0.0, 0.0);
        }),
    );
  }
  theme.patch_recipe::<Select<String>>(|recipe| {
    recipe
      .base(SelectPart::Panel, |s| {
        s.exit(PartAnimation::new(FADE_OUT, MotionDuration::Base, MotionEasing::Exit))
      })
      .state(TooltipSide::Bottom, SelectPart::Panel, |s| {
        s.enter(PartAnimation::new(DOWN, MotionDuration::Base, MotionEasing::Enter))
      })
      .state(TooltipSide::Top, SelectPart::Panel, |s| {
        s.enter(PartAnimation::new(UP, MotionDuration::Base, MotionEasing::Enter))
      })
  });
  app
}

/// A page of 400 by 300 with a select `top` down it.
fn page(app: App, top: f32) -> Stage {
  let value = Signal::new("system".to_owned());
  let mut stage = Stage::in_app(app, move |_| {
    Column::new()
      .size(400.0, 300.0)
      .padding(Padding::new().top(top).left(8.0))
      .child(
        Select::new(
          value.clone(),
          [("system".to_owned(), "System"), ("dark".to_owned(), "Dark")],
        )
        .id("select"),
      )
      .into()
  });
  stage.tree.resize(400, 300);
  stage.pass();
  stage
}

/// Presses the select, and paints the first frame of its list.
fn open(stage: &mut Stage) {
  let (x, y) = stage.center("select");
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass_after(Duration::ZERO);
}

/// How far below its place at rest an option of the list is on the frame it opens on, with the select `top` down
/// the page.
fn opened_at(app: App, top: f32) -> f32 {
  let mut stage = page(app, top);
  open(&mut stage);
  let first = stage.text_quad("Dark").expect("the list is painted").y;
  stage.pass();
  let rest = stage.text_quad("Dark").expect("the list is painted").y;
  first - rest
}

fn near(a: f32, b: f32) -> bool {
  (a - b).abs() < 0.5
}

#[test]
fn a_select_list_moves_in_from_its_trigger_on_the_side_it_opened_on() {
  let under = opened_at(moving(), 8.0);
  assert!(near(under, -4.0), "under its trigger it comes down: {under}");
  let over = opened_at(moving(), 260.0);
  assert!(
    near(over, 4.0),
    "with no room under it, it flips over its trigger and comes up: {over}"
  );
}

#[test]
fn a_select_list_with_no_enter_is_in_place_at_once() {
  let moved = opened_at(App::new(), 8.0);
  assert!(near(moved, 0.0), "the design's list does not move: {moved}");
}

#[test]
fn a_select_list_is_painted_until_its_exit_ends() {
  let mut stage = page(moving(), 8.0);
  stage.click("select");
  assert!(stage.text_quad("Dark").is_some(), "open");

  let escape = || "Escape".to_owned();
  stage.tree.key_down(escape(), escape(), false, false, false);
  stage.tree.key_up(escape(), escape(), false, false, false);
  stage.pass_after(Duration::ZERO);
  stage.pass_after(Duration::from_millis(40));
  assert!(
    stage.text_quad("Dark").is_some(),
    "closing, it is still painted as it fades"
  );

  stage.pass();
  stage.pass();
  assert!(stage.text_quad("Dark").is_none(), "gone once its exit ends");
}

#[test]
fn a_select_list_plays_what_it_says_for_reduced_motion() {
  let app = moving();
  // Under reduced motion the list under its trigger comes up, not down: what it says for then, not its enter.
  app.theme().patch_recipe::<Select<String>>(|recipe| {
    recipe.state(TooltipSide::Bottom, SelectPart::Panel, |s| {
      s.reduced_enter(Some(PartAnimation::new(UP, MotionDuration::Base, MotionEasing::Enter)))
    })
  });
  app.theme().set_reduced_motion(Some(true));
  let moved = opened_at(app, 8.0);
  assert!(
    near(moved, 4.0),
    "with motion reduced it plays its reduced enter: {moved}"
  );
}

#[test]
fn a_select_list_opened_again_while_it_closes_is_there_once() {
  let mut stage = page(moving(), 8.0);
  stage.click("select");
  let (x, y) = stage.center("select");
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass_after(Duration::ZERO);
  stage.pass_after(Duration::from_millis(40));
  assert_eq!(painted(&stage, "Dark"), 1, "closing, it is still painted");

  open(&mut stage);
  stage.pass_after(Duration::from_millis(40));
  assert_eq!(painted(&stage, "Dark"), 1, "once, the frame after it opens again");
  stage.pass();
  stage.pass();
  assert_eq!(painted(&stage, "Dark"), 1, "once, settled");
}

/// How many times the text `content` is painted.
fn painted(stage: &Stage, content: &str) -> usize {
  stage
    .quads()
    .iter()
    .filter(|quad| matches!(&quad.content, lurq::layout::quad::QuadContent::Text { text, .. } if text == content))
    .count()
}
