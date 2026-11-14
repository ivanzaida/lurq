//! An action bar: its bar against the board, where it stands over what it
//! acts on, its actions and its clear button, Escape, its keys, how it comes
//! and goes, and a theme's rules and renderer.

use std::{
  sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
  },
  time::Duration,
};

use lurq::{
  app::{
    App,
    theme::{PaletteColor, ThemePalette},
  },
  components::{Column, Stack, Text},
  node::color::Color,
  ui::{ActionBar, ActionBarPart, ActionBarPlacement, ActionBarRenderer, ActionBarView, Button, icons},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn close(a: f32, b: f32) -> bool {
  (a - b).abs() < 0.5
}

/// The board's bar: Move, Duplicate and Delete, and the clear button, which counts its presses in `cleared`.
fn bar(count: usize, cleared: &Arc<AtomicUsize>, moved: &Arc<AtomicUsize>) -> ActionBar {
  let (cleared, moved) = (cleared.clone(), moved.clone());
  ActionBar::new(count)
    .id("bar")
    .action(
      Button::new("Move")
        .icon(icons::FOLDER_IMPORT)
        .id("move")
        .on_click(move |_| {
          moved.fetch_add(1, Ordering::SeqCst);
        }),
    )
    .action(Button::new("Duplicate").icon(icons::COPY).id("duplicate"))
    .destructive_action(Button::new("Delete").icon(icons::TRASH).id("delete"))
    .on_clear(move || {
      cleared.fetch_add(1, Ordering::SeqCst);
    })
}

/// `bar` over a list that fills the window of 400 by 300.
fn stage(make: impl Fn() -> ActionBar + Send + Sync + 'static) -> Stage {
  Stage::new(move |_| {
    Stack::new()
      .size(400.0, 300.0)
      .child(Column::new().id("list").size(400.0, 300.0).child(Text::new("Rows")))
      .child(make())
      .into()
  })
}

#[test]
fn the_bar_is_44_high_round_and_floating_16_over_the_bottom_in_the_middle() {
  let (cleared, moved) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
  let (c, m) = (cleared.clone(), moved.clone());
  let mut stage = stage(move || bar(3, &c, &m));
  let (x, y, width, height) = stage.bounds("bar");
  assert_eq!(height, 44.0);
  assert!(close(y + height, 300.0 - 16.0), "16 over the bottom");
  assert!(close(x + width / 2.0, 200.0), "in the middle");
  assert_eq!(stage.fill_color("bar"), Some(light(PaletteColor::Popover)));
  let edge = stage
    .border("bar")
    .and_then(|quad| quad.border)
    .and_then(|border| border.top);
  assert_eq!(
    edge.map(|side| (side.width, side.color)),
    Some((1.0, light(PaletteColor::Border)))
  );
  // The count, 13/600 in the foreground, 16 in from the start.
  let (color, size) = stage.text("3 selected");
  assert_eq!((color, size), (light(PaletteColor::Foreground), 13.0));
  let count = stage.text_quad("3 selected").unwrap();
  assert!(close(count.x - x, 16.0));
  // An action is 32 high and round; the one that destroys is in the destructive color.
  let (_, _, _, move_height) = stage.bounds("move");
  assert_eq!(move_height, 32.0);
  assert_eq!(stage.text("Move").0, light(PaletteColor::Foreground));
  assert_eq!(stage.text("Delete").0, light(PaletteColor::Destructive));
  // The clear button is a round 32, 6 in from the end.
  let (clear_x, _, clear_width, clear_height) = stage.bounds("bar-clear");
  assert_eq!((clear_width, clear_height), (32.0, 32.0));
  assert!(close(x + width - (clear_x + clear_width), 6.0));
}

#[test]
fn at_the_top_it_stands_16_under_the_top() {
  let (cleared, moved) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
  let mut stage = stage(move || bar(2, &cleared, &moved).placement(ActionBarPlacement::Top));
  let (_, y, ..) = stage.bounds("bar");
  assert!(close(y, 16.0));
  assert!(stage.paints("2 selected"));
}

#[test]
fn nothing_selected_shows_no_bar_and_what_it_says_can_be_another_text() {
  let (cleared, moved) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
  let (c, m) = (cleared.clone(), moved.clone());
  let mut empty = stage(move || bar(0, &c, &m));
  empty.pass_after(Duration::from_secs(2));
  assert!(!empty.paints("0 selected"));
  let mut files = stage(move || bar(4, &cleared, &moved).text("4 files"));
  assert!(files.paints("4 files"));
  assert!(!files.paints("4 selected"));
  assert!(files.bounds("bar").3 > 0.0);
}

#[test]
fn its_actions_act_its_clear_button_clears_and_escape_clears_where_the_app_says() {
  let (cleared, moved) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
  let (c, m) = (cleared.clone(), moved.clone());
  let mut stage = stage(move || bar(3, &c, &m));
  stage.click("move");
  assert_eq!(moved.load(Ordering::SeqCst), 1);
  stage.click("bar-clear");
  assert_eq!(cleared.load(Ordering::SeqCst), 1);
  stage.key("Escape");
  assert_eq!(
    cleared.load(Ordering::SeqCst),
    1,
    "Escape is not the bar's unless the app says so"
  );

  let (c, m) = (cleared.clone(), moved.clone());
  let mut escaping = self::stage(move || bar(3, &c, &m).escape_clears(true));
  escaping.key("Escape");
  assert_eq!(cleared.load(Ordering::SeqCst), 2);
}

#[test]
fn the_bar_is_one_stop_whose_arrows_move_between_its_buttons() {
  let (cleared, moved) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
  let (c, m) = (cleared.clone(), moved.clone());
  let mut stage = stage(move || bar(3, &c, &m));
  stage.key("Tab");
  let focused = |stage: &Stage| {
    stage
      .tree
      .focused_element()
      .and_then(|element| element.id().map(str::to_owned))
  };
  assert_eq!(focused(&stage).as_deref(), Some("move"));
  assert!(stage.has_ring(light(PaletteColor::Ring)));
  stage.key("ArrowRight");
  assert_eq!(focused(&stage).as_deref(), Some("duplicate"));
  stage.key("End");
  assert_eq!(focused(&stage).as_deref(), Some("bar-clear"));
  stage.key("Enter");
  assert_eq!(cleared.load(Ordering::SeqCst), 1);
  stage.key("Home");
  stage.key(" ");
  assert_eq!(moved.load(Ordering::SeqCst), 1);
}

#[test]
fn a_press_on_the_room_around_the_bar_reaches_the_list_under_it() {
  let pressed = Arc::new(AtomicUsize::new(0));
  let held = pressed.clone();
  let (cleared, moved) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
  let mut stage = Stage::new(move |_| {
    let held = held.clone();
    Stack::new()
      .size(400.0, 300.0)
      .child(
        lurq::components::Button::new("Row")
          .id("row")
          .size(400.0, 100.0)
          .on_click(move |_| {
            held.fetch_add(1, Ordering::SeqCst);
          }),
      )
      .child(bar(3, &cleared, &moved))
      .into()
  });
  stage.click("row");
  assert_eq!(pressed.load(Ordering::SeqCst), 1);
}

#[test]
fn a_rule_of_the_theme_changes_the_bar() {
  let app = App::new();
  app
    .theme()
    .patch_recipe::<ActionBar>(|recipe| recipe.base(ActionBarPart::Bar, |s| s.background(PaletteColor::Destructive)));
  let (cleared, moved) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
  let mut stage = Stage::in_app(app, move |_| {
    Stack::new().size(400.0, 300.0).child(bar(3, &cleared, &moved)).into()
  });
  assert_eq!(stage.fill_color("bar"), Some(light(PaletteColor::Destructive)));
}

#[test]
fn a_theme_gives_the_bar_another_anatomy() {
  let app = App::new();
  let renderer: ActionBarRenderer = Arc::new(|view: ActionBarView| {
    let shouted = view.text.to_uppercase();
    let mut column = Column::new().child(Text::new(&shouted));
    if let Some(clear) = view.clear() {
      column = column.child(clear);
    }
    column.into()
  });
  app.theme().set_renderer::<ActionBar>(renderer);
  let (cleared, moved) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
  let c = cleared.clone();
  let mut stage = Stage::in_app(app, move |_| {
    Stack::new().size(400.0, 300.0).child(bar(3, &c, &moved)).into()
  });
  assert!(stage.paints("3 SELECTED"));
  stage.click("bar-clear");
  assert_eq!(cleared.load(Ordering::SeqCst), 1);
}
