//! An overlay closed and opened again while it leaves: the one leaving ends its leaving and is the one open, so that
//! there is one of it in the tree and on the screen, never one entering over one leaving. A popover, a dialog, a
//! sheet, a drawer, a menu, a tooltip and a hover card. The design gives a menu, a dialog and a tooltip no exit
//! animation, so that they go at once and nothing of them leaves; their tests give them one, as a theme may.

use std::{
  sync::atomic::{AtomicUsize, Ordering},
  time::{Duration, Instant},
};

use lurq::{
  app::{
    App,
    ctx::Ctx,
    theme::{MotionDuration, MotionEasing},
  },
  components::{Column, Rect, Row, TextInput},
  core::{ElementRef, Signal},
  layout::quad::QuadContent,
  node::Element,
  ui::{
    Button, ButtonVariant, Dialog, DialogPart, Drawer, FADE_IN, FADE_OUT, HoverCard, Menu, MenuItem, MenuPart,
    PartAnimation, Popover, Recipe, Sheet, SheetSide, Tooltip, TooltipPart,
  },
};

use super::stage::Stage;

/// A stage of 400 by 300 in `app`.
fn window(app: App, scene: impl Fn(&mut Ctx) -> Element + Send + Sync + 'static) -> Stage {
  let mut stage = Stage::in_app(app, scene);
  stage.tree.resize(400, 300);
  stage.pass();
  stage
}

/// `recipe` with `part` fading in as it comes and out as it goes, over the theme's base duration.
fn fading(recipe: Recipe, part: impl Into<lurq::ui::Name>) -> Recipe {
  recipe.base(part, |s| {
    s.enter(PartAnimation::new(FADE_IN, MotionDuration::Base, MotionEasing::Enter))
      .exit(PartAnimation::new(FADE_OUT, MotionDuration::Base, MotionEasing::Exit))
  })
}

/// How many times the text `content` is painted.
fn painted(stage: &Stage, content: &str) -> usize {
  stage
    .quads()
    .iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text == content))
    .count()
}

/// How many elements of the tree say `content`.
fn in_tree(stage: &mut Stage, content: &str) -> usize {
  let count = AtomicUsize::new(0);
  stage.tree.find_element(|element| {
    if element.text_content() == Some(content) {
      count.fetch_add(1, Ordering::SeqCst);
    }
    false
  });
  count.into_inner()
}

/// Closes what `open` holds open, opens it again while it leaves, and checks that the one leaving was there on its
/// way out and that one of it is there after: a frame after the second open, and once it has settled.
fn closed_and_opened_again(stage: &mut Stage, open: &Signal<bool>, content: &str) {
  stage.pass();
  assert_eq!(painted(stage, content), 1, "{content}: open");

  open.set(false);
  stage.pass_after(Duration::ZERO);
  stage.pass_after(Duration::from_millis(40));
  assert_eq!(painted(stage, content), 1, "{content}: leaving, it is still painted");

  open.set(true);
  stage.pass_after(Duration::from_millis(40));
  assert_eq!(
    painted(stage, content),
    1,
    "{content}: once, the frame after it opens again"
  );
  assert_eq!(in_tree(stage, content), 1, "{content}: one in the tree");
  stage.pass_after(Duration::from_millis(56));
  assert_eq!(painted(stage, content), 1, "{content}: once, the frame after");
  stage.pass();
  stage.pass();
  assert_eq!(painted(stage, content), 1, "{content}: once, settled");
  assert_eq!(in_tree(stage, content), 1, "{content}: one in the tree, settled");
}

/// A page of 400 by 300 with an anchor at 20, 20 and `layer` beside it.
fn page(anchor: &ElementRef, layer: impl Into<Element>) -> Element {
  Column::new()
    .size(400.0, 300.0)
    .padding(20.0)
    .child(
      Row::new()
        .child(Rect::new(120.0, 30.0).id("anchor").ref_element(anchor.clone()))
        .child(layer),
    )
    .into()
}

#[test]
fn a_popover_opened_again_while_it_leaves_is_there_once() {
  let (open, anchor) = (Signal::new(true), ElementRef::new());
  let scene = open.clone();
  let mut stage = window(App::new(), move |_| {
    page(&anchor, Popover::new(scene.clone(), anchor.clone()).title("Dimensions"))
  });
  closed_and_opened_again(&mut stage, &open, "Dimensions");
}

#[test]
fn a_menu_opened_again_while_it_leaves_is_there_once() {
  let (open, anchor) = (Signal::new(true), ElementRef::new());
  let scene = open.clone();
  let app = App::new();
  app
    .theme()
    .patch_recipe::<Menu>(|recipe| fading(recipe, MenuPart::Panel));
  let mut stage = window(app, move |_| {
    page(
      &anchor,
      Menu::new(scene.clone(), anchor.clone())
        .item(MenuItem::new("Rename"))
        .item(MenuItem::new("Duplicate")),
    )
  });
  closed_and_opened_again(&mut stage, &open, "Rename");
}

#[test]
fn a_dialog_opened_again_while_it_leaves_is_there_once_with_its_focus_and_what_was_typed() {
  let (open, name) = (Signal::new(true), Signal::new(String::new()));
  let (scene, typed) = (open.clone(), name.clone());
  let app = App::new();
  app
    .theme()
    .patch_recipe::<Dialog>(|recipe| fading(recipe, DialogPart::Panel));
  let mut stage = window(app, move |_| {
    Column::new()
      .size(400.0, 300.0)
      .child(
        Dialog::new(scene.clone())
          .title("Rename workspace")
          .child(TextInput::new(typed.clone()).id("name")),
      )
      .into()
  });
  stage.pass();
  stage.click("name");
  stage.tree.text_input_replace(0..0, "Atlas");
  stage.pass();
  closed_and_opened_again(&mut stage, &open, "Rename workspace");
  // The focus is in its field again and what is typed goes on after what was.
  assert_eq!(
    stage
      .tree
      .focused_element()
      .and_then(|element| element.id().map(str::to_owned))
      .as_deref(),
    Some("name")
  );
  stage.tree.text_input_replace(5..5, " II");
  stage.pass();
  assert_eq!(name.get(), "Atlas II");
}

#[test]
fn a_sheet_opened_again_while_it_leaves_is_there_once() {
  let open = Signal::new(true);
  let scene = open.clone();
  let mut stage = window(App::new(), move |_| {
    Column::new()
      .size(400.0, 300.0)
      .child(Sheet::new(scene.clone()).side(SheetSide::End).title("Edit profile"))
      .into()
  });
  closed_and_opened_again(&mut stage, &open, "Edit profile");
}

#[test]
fn a_drawer_opened_again_while_it_leaves_is_there_once() {
  let open = Signal::new(true);
  let scene = open.clone();
  let mut stage = window(App::new(), move |_| {
    Row::new()
      .size(400.0, 300.0)
      .child(
        Drawer::new(scene.clone())
          .side(SheetSide::Start)
          .label("Navigation")
          .child(lurq::components::Text::new("Inbox")),
      )
      .child(lurq::components::Text::new("The page"))
      .into()
  });
  closed_and_opened_again(&mut stage, &open, "Inbox");
}

/// Lets the timers run `later` from now, and paints.
fn after(stage: &mut Stage, later: Duration) {
  stage.tree.tick_timers_at(Instant::now() + later);
  stage.pass_after(Duration::ZERO);
}

/// A page with `layer` mounted on a button and, far from it, somewhere for the pointer to go.
fn hovered(app: App, layer: impl Fn(&mut Ctx) -> Element + Send + Sync + 'static) -> Stage {
  window(app, move |ctx| {
    Column::new().size(400.0, 300.0).padding(100.0).child(layer(ctx)).into()
  })
}

/// Shows what the pointer on `id` shows, takes the pointer away until it begins to leave, and brings the pointer
/// back on the way: one of it is there after.
fn left_and_pointed_at_again(stage: &mut Stage, id: &str, content: &str) {
  stage.hover(id);
  after(stage, Duration::from_millis(1000));
  stage.pass();
  assert_eq!(painted(stage, content), 1, "{content}: shown");

  stage.tree.mouse_move(390.0, 290.0);
  stage.pass_after(Duration::ZERO);
  after(stage, Duration::from_millis(1000));
  stage.pass_after(Duration::from_millis(40));
  assert_eq!(painted(stage, content), 1, "{content}: leaving, it is still painted");

  let (x, y) = stage.center(id);
  stage.tree.mouse_move(x, y);
  stage.pass_after(Duration::from_millis(40));
  after(stage, Duration::from_millis(1000));
  stage.pass_after(Duration::from_millis(80));
  assert_eq!(painted(stage, content), 1, "{content}: once, as it shows again");
  assert_eq!(in_tree(stage, content), 1, "{content}: one in the tree");
  stage.pass();
  stage.pass();
  assert_eq!(painted(stage, content), 1, "{content}: once, settled");
}

#[test]
fn a_tooltip_shown_again_while_it_leaves_is_there_once() {
  let app = App::new();
  app
    .theme()
    .patch_recipe::<Tooltip>(|recipe| fading(recipe, TooltipPart::Panel));
  let mut stage = hovered(app, |ctx| {
    Tooltip::new("Copy link")
      .open_delay(Duration::ZERO)
      .mount(ctx, Button::new("Copy").variant(ButtonVariant::Outline).id("copy"))
  });
  left_and_pointed_at_again(&mut stage, "copy", "Copy link");
}

#[test]
fn a_hover_card_shown_again_while_it_leaves_is_there_once() {
  let mut stage = hovered(App::new(), |ctx| {
    HoverCard::new()
      .title("@lurq")
      .description("A Rust UI toolkit.")
      .mount(ctx, Button::new("Author").variant(ButtonVariant::Outline).id("author"))
  });
  left_and_pointed_at_again(&mut stage, "author", "@lurq");
}
