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
    theme::{MotionDuration, MotionEasing, PaletteColor, ThemePalette},
  },
  components::{Column, Stack, Text},
  layout::quad::QuadContent,
  node::color::Color,
  ui::{
    ActionBar, ActionBarPart, ActionBarRenderer, ActionBarSide, ActionBarView, Button, FADE_IN, PartAnimation, icons,
  },
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

/// `bar` over a list that fills the window of 600 by 300, where the bar has room for all of it.
fn stage(make: impl Fn() -> ActionBar + Send + Sync + 'static) -> Stage {
  stage_in(App::new(), make)
}

/// `bar` over the list in an app whose bar fades in where it stands: the toast's keyframes it moves in with run on
/// the process's clock, so where it is while it moves depends on when a test reads it.
fn still(make: impl Fn() -> ActionBar + Send + Sync + 'static) -> Stage {
  let app = App::new();
  app.theme().patch_recipe::<ActionBar>(|recipe| {
    recipe
      .variant(ActionBarSide::Bottom, ActionBarPart::Bar, |s| {
        s.enter(PartAnimation::new(FADE_IN, MotionDuration::Base, MotionEasing::Enter))
      })
      .variant(ActionBarSide::Top, ActionBarPart::Bar, |s| {
        s.enter(PartAnimation::new(FADE_IN, MotionDuration::Base, MotionEasing::Enter))
      })
  });
  stage_in(app, make)
}

fn stage_in(app: App, make: impl Fn() -> ActionBar + Send + Sync + 'static) -> Stage {
  Stage::in_app(app, move |_| {
    Stack::new()
      .size(600.0, 300.0)
      .child(Column::new().id("list").size(600.0, 300.0).child(Text::new("Rows")))
      .child(make())
      .into()
  })
}

#[test]
fn the_bar_is_44_high_round_and_floating_16_over_the_bottom_in_the_middle() {
  let (cleared, moved) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
  let (c, m) = (cleared.clone(), moved.clone());
  let mut stage = still(move || bar(3, &c, &m));
  let (x, y, width, height) = stage.bounds("bar");
  assert_eq!(height, 44.0);
  assert!(close(y + height, 300.0 - 16.0), "16 over the bottom");
  assert!(close(x + width / 2.0, 300.0), "in the middle");
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
  let mut stage = still(move || bar(2, &cleared, &moved).side(ActionBarSide::Top));
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
  let mut files = stage(move || bar(4, &cleared, &moved).count_text("4 files"));
  assert!(files.paints("4 files"));
  assert!(!files.paints("4 selected"));
  assert!(files.bounds("bar").3 > 0.0);
}

#[test]
fn its_actions_act_its_clear_button_clears_and_escape_clears_where_the_app_says() {
  let (cleared, moved) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
  let (c, m) = (cleared.clone(), moved.clone());
  let mut stage = still(move || bar(3, &c, &m));
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
  let mut escaping = still(move || bar(3, &c, &m).escape_clears(true));
  escaping.key("Escape");
  assert_eq!(cleared.load(Ordering::SeqCst), 2);
}

#[test]
fn the_bar_is_one_stop_whose_arrows_move_between_its_buttons() {
  let (cleared, moved) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
  let (c, m) = (cleared.clone(), moved.clone());
  let mut stage = still(move || bar(3, &c, &m));
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

/// `make` over a list `width` wide, in an app whose bar fades in where it stands, at the theme's text size `scale`.
fn narrow(width: f32, scale: f32, make: impl Fn() -> ActionBar + Send + Sync + 'static) -> Stage {
  let app = App::new();
  app.theme().set_text_scale(scale);
  app.theme().patch_recipe::<ActionBar>(|recipe| {
    recipe.variant(ActionBarSide::Bottom, ActionBarPart::Bar, |s| {
      s.enter(PartAnimation::new(FADE_IN, MotionDuration::Base, MotionEasing::Enter))
    })
  });
  Stage::in_app(app, move |_| {
    Stack::new()
      .size(width, 300.0)
      .child(Column::new().id("list").size(width, 300.0).child(Text::new("Rows")))
      .child(make())
      .into()
  })
}

/// Whether the clear button is whole, inside the bar, 6 from its end, and after the last action.
fn clear_is_whole(stage: &mut Stage, room: f32) {
  let (x, _, width, _) = stage.bounds("bar");
  assert!(
    x >= 16.0 - 0.5 && x + width <= room - 16.0 + 0.5,
    "the bar is in its room"
  );
  let (clear_x, _, clear_width, clear_height) = stage.bounds("bar-clear");
  assert_eq!((clear_width, clear_height), (32.0, 32.0), "the clear button is whole");
  assert!(close(x + width - (clear_x + clear_width), 6.0), "6 from the end");
  let (delete_x, _, delete_width, _) = stage.bounds("delete");
  assert!(delete_x + delete_width <= clear_x, "after the actions");
}

#[test]
fn a_long_text_gives_way_and_the_actions_and_the_clear_button_stay() {
  let (cleared, moved) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
  let text = "3 files selected in the folder of the quarterly reports of the year before last";
  let mut stage = still(move || bar(3, &cleared, &moved).count_text(text));
  clear_is_whole(&mut stage, 600.0);
  // The text is cut where the line after it starts.
  let (move_x, ..) = stage.bounds("move");
  let count = stage
    .quads()
    .into_iter()
    .find_map(|quad| match &quad.content {
      QuadContent::Text { text, .. } if text.starts_with("3 files") => Some((quad.x, quad.width, text.clone())),
      _ => None,
    })
    .expect("the text is painted");
  assert!(count.2.ends_with('…'), "cut with an ellipsis: {}", count.2);
  assert!(count.0 + count.1 < move_x, "the text ends before the actions");
  let (_, _, move_width, _) = stage.bounds("move");
  assert!(move_width > 32.0, "Move keeps its label");
}

#[test]
fn at_a_larger_text_size_the_clear_button_is_whole() {
  let (cleared, moved) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
  let mut stage = narrow(420.0, 1.3, move || bar(3, &cleared, &moved));
  clear_is_whole(&mut stage, 420.0);
}

#[test]
fn cleared_from_the_bar_the_focus_goes_where_the_app_says() {
  let selected = lurq::core::Signal::new(3usize);
  let row = lurq::core::ElementRef::new();
  let (held, target) = (selected.clone(), row.clone());
  let still = App::new();
  still.theme().patch_recipe::<ActionBar>(|recipe| {
    recipe.variant(ActionBarSide::Bottom, ActionBarPart::Bar, |s| {
      s.enter(PartAnimation::new(FADE_IN, MotionDuration::Base, MotionEasing::Enter))
    })
  });
  let mut stage = Stage::in_app(still, move |ctx| {
    let clear = held.clone();
    Stack::new()
      .size(600.0, 300.0)
      .child(
        Column::new().size(600.0, 300.0).child(
          lurq::components::Button::new("Row")
            .id("row")
            .ref_element(target.clone()),
        ),
      )
      .child(
        ActionBar::new(held.get())
          .id("bar")
          .action(Button::new("Move").icon(icons::FOLDER_IMPORT).id("move"))
          .on_clear(move || clear.set(0))
          .escape_clears(true)
          .return_focus(target.clone())
          .mount(ctx),
      )
      .into()
  });
  let focused = |stage: &Stage| {
    stage
      .tree
      .focused_element()
      .and_then(|element| element.id().map(str::to_owned))
  };
  // Escape in the bar clears the selection, and the focus goes back to the row.
  stage.key("Tab");
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("move"));
  stage.key("Escape");
  assert_eq!(selected.get(), 0);
  stage.pass_after(Duration::from_secs(1));
  assert_eq!(focused(&stage).as_deref(), Some("row"));
  // Enter on the clear button does the same.
  selected.set(2);
  stage.pass_after(Duration::from_secs(1));
  stage.key("Tab");
  stage.key("End");
  assert_eq!(focused(&stage).as_deref(), Some("bar-clear"));
  stage.key("Enter");
  assert_eq!(selected.get(), 0);
  stage.pass_after(Duration::from_secs(1));
  assert_eq!(focused(&stage).as_deref(), Some("row"));
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
    let shouted = view.count_text.to_uppercase();
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

#[test]
fn every_clear_handler_is_called_in_order_and_actions_add_several() {
  let told = Arc::new(std::sync::Mutex::new(Vec::new()));
  let (first, second) = (told.clone(), told.clone());
  let mut stage = still(move || {
    let (first, second) = (first.clone(), second.clone());
    ActionBar::new(3)
      .id("bar")
      .actions([Button::new("Move").id("move"), Button::new("Copy").id("copy")])
      .destructive_actions([Button::new("Delete").id("delete")])
      .on_clear(move || first.lock().unwrap().push("first"))
      .on_clear(move || second.lock().unwrap().push("second"))
      .force(Some(lurq::ui::Interaction::Hovered))
  });
  let (move_x, ..) = stage.bounds("move");
  let (copy_x, ..) = stage.bounds("copy");
  let (delete_x, ..) = stage.bounds("delete");
  assert!(move_x < copy_x && copy_x < delete_x, "in the order they were added");
  assert_eq!(stage.text("Delete").0, light(PaletteColor::Destructive));
  assert_eq!(
    stage.fill_color("bar-clear"),
    Some(light(PaletteColor::Highlight)),
    "the clear button as under the pointer"
  );
  stage.click("bar-clear");
  assert_eq!(*told.lock().unwrap(), ["first", "second"]);
}

#[cfg(any(feature = "devtools", feature = "mcp"))]
#[test]
fn the_bar_is_called_by_its_label_or_what_it_says() {
  let (cleared, moved) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
  let (c, m) = (cleared.clone(), moved.clone());
  let mut plain = still(move || bar(3, &c, &m));
  let mut named = still(move || bar(3, &cleared, &moved).label("Actions on 3 files"));
  let name = |stage: &mut Stage| {
    stage
      .tree
      .get_element_by_id_mut("bar")
      .and_then(|element| element.description("a11y_name"))
  };
  assert_eq!(name(&mut plain).as_deref(), Some("3 selected"));
  assert_eq!(name(&mut named).as_deref(), Some("Actions on 3 files"));
}

#[test]
fn the_first_frame_at_a_width_is_the_settled_one() {
  let (cleared, moved) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
  for width in [600.0, 420.0, 300.0, 200.0] {
    // The bar fades in where it stands, so a frame differs from the next only where it settles.
    let app = App::new();
    app.theme().patch_recipe::<ActionBar>(|recipe| {
      recipe
        .variant(ActionBarSide::Bottom, ActionBarPart::Bar, |s| {
          s.enter(PartAnimation::new(FADE_IN, MotionDuration::Base, MotionEasing::Enter))
        })
        .variant(ActionBarSide::Top, ActionBarPart::Bar, |s| {
          s.enter(PartAnimation::new(FADE_IN, MotionDuration::Base, MotionEasing::Enter))
        })
    });
    let (cleared, moved) = (cleared.clone(), moved.clone());
    let mut stage = Stage::first_frame_in(app, width, 300.0, move |ctx| {
      Column::new()
        .size(width, 300.0)
        .child(bar(3, &cleared, &moved).mount(ctx))
        .into()
    });
    let first = stage.picture();
    stage.frame();
    assert_eq!(first, stage.picture(), "at {width}: the first frame is the next one");
  }
}

/// The board's bar, mounted over a list in a room `width` wide whose id is `room`, in an app whose bar fades in
/// where it stands, with one frame painted.
fn roomed(width: &lurq::core::Signal<f32>, moved: &Arc<AtomicUsize>, plain: bool) -> Stage {
  let app = App::new();
  app.theme().patch_recipe::<ActionBar>(|recipe| {
    recipe.variant(ActionBarSide::Bottom, ActionBarPart::Bar, |s| {
      s.enter(PartAnimation::new(FADE_IN, MotionDuration::Base, MotionEasing::Enter))
    })
  });
  let (width, moved) = (width.clone(), moved.clone());
  Stage::first_frame_in(app, 700.0, 300.0, move |ctx| {
    let cleared = Arc::new(AtomicUsize::new(0));
    let mut actions = bar(3, &cleared, &moved);
    if plain {
      // An action with no icon: its label is all it has.
      actions = actions.action(Button::new("Tag").id("tag"));
    }
    Column::new()
      .size(700.0, 300.0)
      .child(
        Stack::new()
          .id("room")
          .size(width.get(), 300.0)
          .child(
            Column::new()
              .id("list")
              .size(width.get(), 300.0)
              .child(Text::new("Rows")),
          )
          .child(actions.mount(ctx)),
      )
      .into()
  })
}

/// Where the bar and each of its buttons are, and how wide.
fn laid(stage: &mut Stage) -> Vec<(f32, f32)> {
  ["bar", "move", "duplicate", "delete", "bar-clear"]
    .into_iter()
    .map(|id| {
      let (x, _, width, _) = stage.bounds(id);
      (x.round(), width.round())
    })
    .collect()
}

#[test]
fn a_mounted_bar_wider_than_its_room_draws_its_actions_as_icons_before_its_text_gives_way() {
  let moved = Arc::new(AtomicUsize::new(0));
  let width = lurq::core::Signal::new(330.0f32);
  let mut stage = roomed(&width, &moved, false);
  // 330 has no room for the three labels: each action is its icon alone, and what the bar says is whole.
  for (id, label) in [("move", "Move"), ("duplicate", "Duplicate"), ("delete", "Delete")] {
    let (.., action, height) = stage.bounds(id);
    assert!(
      action <= 36.0 && height == 32.0,
      "{id} is its icon alone: {action} wide"
    );
    assert!(!stage.paints(label), "{label} is not written");
  }
  assert!(stage.paints("3 selected"), "the text is whole");
  clear_is_whole(&mut stage, 330.0);
  let first = stage.picture();
  stage.frame();
  assert_eq!(stage.picture(), first, "settled in its first frame");
  // An icon alone still acts.
  stage.click("move");
  assert_eq!(moved.load(Ordering::SeqCst), 1);
}

#[test]
fn a_mounted_bar_follows_its_room_its_labels_back_when_there_is_room() {
  let moved = Arc::new(AtomicUsize::new(0));
  let width = lurq::core::Signal::new(600.0f32);
  let mut stage = roomed(&width, &moved, false);
  // With room for all of it the bar is as it was: every action with its label.
  assert!(stage.paints("Move") && stage.paints("Duplicate") && stage.paints("Delete"));
  assert!(stage.bounds("move").2 > 40.0);
  let wide = laid(&mut stage);

  width.set(330.0);
  stage.frame();
  assert!(!stage.paints("Duplicate"), "in the frame the room narrowed in");
  let narrow = laid(&mut stage);
  let fresh = laid(&mut roomed(&lurq::core::Signal::new(330.0), &moved, false));
  assert_eq!(narrow, fresh, "as a bar laid out at 330 first");

  width.set(600.0);
  stage.frame();
  assert!(stage.paints("Duplicate"), "in the frame the room grew in");
  assert_eq!(laid(&mut stage), wide, "as a bar laid out at 600 first");
}

#[test]
fn an_action_with_no_icon_keeps_its_label_and_actions_with_no_room_go_into_the_menu_of_more() {
  let moved = Arc::new(AtomicUsize::new(0));
  let width = lurq::core::Signal::new(380.0f32);
  let mut stage = roomed(&width, &moved, true);
  assert!(stage.bounds("move").2 <= 36.0, "an action with an icon is its icon");
  assert!(stage.paints("Tag"), "one with none keeps its label");
  assert!(stage.paints("3 selected"));
  // Narrower than the icons too, the actions go into the menu of More, the last first: what the bar says and
  // the clear button stay, and nothing of the bar is past it.
  width.set(250.0);
  stage.frame();
  let (x, _, bar_width, _) = stage.bounds("bar");
  assert!(
    x >= 16.0 - 0.5 && x + bar_width <= 250.0 - 16.0 + 0.5,
    "the bar is in its room"
  );
  assert!(stage.paints("3 selected"), "the text is whole");
  let (clear, _, clear_width, _) = stage.bounds("bar-clear");
  assert_eq!(clear_width, 32.0);
  assert!(
    close(x + bar_width - (clear + clear_width), 6.0),
    "the clear button 6 from the bar's end"
  );
  assert_eq!(stage.bounds("tag").2, 0.0, "the last action went");
  let (more, _, more_width, _) = stage.bounds("bar-more");
  assert!(
    more_width > 0.0 && more + more_width <= clear,
    "More before the clear button"
  );
  let settled = stage.picture();
  stage.frame();
  assert_eq!(stage.picture(), settled, "settled in the frame the room narrowed in");
  stage.click("bar-more");
  stage.pass();
  assert!(stage.paints("Tag") && stage.paints("Move"), "its menu holds what went");
  stage.click("move-item");
  assert_eq!(moved.load(Ordering::SeqCst), 1, "chosen there, an action acts");
  // Narrower than what it says with More and the clear button, the text gives way at last.
  width.set(170.0);
  stage.frame();
  let (x, _, bar_width, _) = stage.bounds("bar");
  assert!(
    x >= 16.0 - 0.5 && x + bar_width <= 170.0 - 16.0 + 0.5,
    "the bar is in its room"
  );
  assert!(!stage.paints("3 selected"), "the text is cut");
  assert_eq!(stage.bounds("bar-clear").2, 32.0);
  // With room again every action is back in the bar, and More is gone.
  width.set(380.0);
  stage.frame();
  stage.frame();
  assert!(stage.bounds("tag").2 > 0.0 && stage.bounds("bar-more").2 == 0.0);
  assert!(stage.paints("3 selected"));
}
