//! A status bar: the bar and its items against the board, an item that is pressed, one that shows progress, the
//! tones of an item and of the bar, the keys of one stop of Tab, the items that leave a bar too narrow in the order
//! the app says, a text longer than its room, what tooling reads, and a theme's rules and renderer.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App,
    theme::{PaletteColor, ThemePalette},
  },
  components::{Column, Row, Text},
  node::color::Color,
  ui::{
    StatusBar, StatusBarPart, StatusBarRenderer, StatusBarVariant, StatusBarView, StatusItem, StatusItemVariant,
    StatusRegion, icons,
  },
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn close(a: f32, b: f32) -> bool {
  (a - b).abs() < 0.5
}

type Log = Arc<Mutex<Vec<String>>>;

/// The board's bar: the branch in the primary fill, sync, errors, warnings (pressed), indexing; the place at the end.
fn board(told: &Log) -> StatusBar {
  let (sync, problems) = (told.clone(), told.clone());
  StatusBar::new()
    .label("Editor status")
    .id("bar")
    .leading([
      StatusItem::new("feat/design-system")
        .icon(icons::BRANCH)
        .variant(StatusItemVariant::Primary)
        .id("branch"),
      StatusItem::new("2↓ 1↑")
        .icon(icons::REFRESH)
        .id("sync")
        .on_select(move || sync.lock().unwrap().push("sync".to_owned())),
      StatusItem::new("0").icon(icons::CLOSE_CIRCLE).id("errors"),
      StatusItem::new("3")
        .icon(icons::WARNING)
        .id("warnings")
        .on_select(move || problems.lock().unwrap().push("problems".to_owned())),
      StatusItem::new("Indexing").progress(0.55).id("indexing"),
    ])
    .trailing([
      StatusItem::new("Ln 42, Col 7").id("place"),
      StatusItem::new("UTF-8").id("encoding"),
      StatusItem::new("Rust").id("language"),
      StatusItem::new("Notifications")
        .icon(icons::BELL)
        .icon_only(true)
        .id("bell"),
    ])
}

fn stage(width: f32, told: &Log) -> Stage {
  let told = told.clone();
  Stage::new(move |ctx| Column::new().size(width, 300.0).child(board(&told).mount(ctx)).into())
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

#[test]
fn the_bar_and_its_items_are_the_boards() {
  let told = Arc::new(Mutex::new(Vec::new()));
  let mut stage = stage(860.0, &told);
  let (_, _, width, height) = stage.bounds("bar");
  assert_eq!((width, height), (860.0, 26.0), "a bar 26 high across its room");
  assert_eq!(stage.fill_color("bar"), Some(light(PaletteColor::Muted)));
  let (branch_x, _, _, branch_height) = stage.bounds("branch");
  assert_eq!(
    (branch_x, branch_height),
    (0.0, 26.0),
    "items as high as the bar, from its start"
  );
  assert_eq!(
    stage.fill_color("branch"),
    Some(light(PaletteColor::Primary)),
    "the branch in the primary fill"
  );
  assert_eq!(
    stage.text("feat/design-system"),
    (light(PaletteColor::PrimaryForeground), 12.0)
  );
  assert_eq!(stage.text("UTF-8"), (light(PaletteColor::Foreground), 12.0));
  // The place is at the end: the bell's end is the bar's.
  let (bell_x, _, bell_width, _) = stage.bounds("bell");
  assert!(close(bell_x + bell_width, 860.0));
  assert!(
    close(bell_width, 8.0 + 13.0 + 8.0),
    "an icon of 13 padded 8: {bell_width}"
  );
  assert!(!stage.paints("Notifications"), "an icon alone");
  // Progress: the text in the muted foreground and a rail 72 by 4 of the input's color.
  assert_eq!(stage.text("Indexing").0, light(PaletteColor::MutedForeground));
  let rail = stage.quads().into_iter().any(|quad| {
    matches!(&quad.content, lurq::layout::quad::QuadContent::Rect { color, .. } if *color == light(PaletteColor::Input))
      && close(quad.width, 72.0)
      && close(quad.height, 4.0)
  });
  assert!(rail, "a rail of 72 by 4");
  let done = stage.quads().into_iter().any(|quad| {
    matches!(&quad.content, lurq::layout::quad::QuadContent::Rect { color, .. } if *color == light(PaletteColor::Foreground))
      && close(quad.width, 72.0 * 0.55)
  });
  assert!(done, "its part done");
}

#[test]
fn a_pressed_item_calls_its_handler_and_the_pressed_items_are_one_stop() {
  let told = Arc::new(Mutex::new(Vec::new()));
  let mut stage = stage(860.0, &told);
  stage.click("warnings");
  assert_eq!(*told.lock().unwrap(), ["problems"]);
  stage.click("errors");
  assert_eq!(told.lock().unwrap().len(), 1, "an item with no handler is no button");
  stage.key("Tab");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("sync"),
    "Tab enters at the first pressed item"
  );
  stage.key("ArrowRight");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("warnings"),
    "the arrows pass what is not pressed by"
  );
  stage.key("Enter");
  assert_eq!(told.lock().unwrap()[1..], ["problems"]);
  stage.key("Tab");
  assert_ne!(focused(&stage).as_deref(), Some("sync"), "Tab leaves the bar");
}

#[test]
fn a_tone_colors_an_item_and_the_whole_bar() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .size(860.0, 300.0)
      .child(
        StatusBar::new()
          .id("bar")
          .leading([StatusItem::new("2 errors")
            .variant(StatusItemVariant::Destructive)
            .id("errors")])
          .trailing([StatusItem::new("Rust").id("language")]),
      )
      .child(
        StatusBar::new()
          .id("debugging")
          .variant(StatusBarVariant::Warning)
          .leading([StatusItem::new("Debugging").id("debug")])
          .trailing([StatusItem::new("Plain").variant(StatusItemVariant::Neutral).id("plain")]),
      )
      .into()
  });
  assert_eq!(stage.fill_color("errors"), Some(light(PaletteColor::DestructiveMuted)));
  assert_eq!(stage.text("2 errors").0, light(PaletteColor::Destructive));
  assert_eq!(stage.text("Rust").0, light(PaletteColor::Foreground));
  assert_eq!(stage.fill_color("debugging"), Some(light(PaletteColor::WarningMuted)));
  assert_eq!(
    stage.text("Debugging").0,
    light(PaletteColor::Warning),
    "an item is in the bar's tone"
  );
  assert_eq!(
    stage.text("Plain").0,
    light(PaletteColor::Foreground),
    "unless it has its own"
  );
}

#[test]
fn a_bar_too_narrow_drops_its_items_from_the_end_it_says_and_by_priority() {
  let bar = |first: StatusRegion| {
    StatusBar::new()
      .drops_first(first)
      .leading([
        StatusItem::new("feat/design-system").id("branch").priority(1),
        StatusItem::new("Indexing").id("indexing"),
      ])
      .trailing([
        StatusItem::new("Ln 42, Col 7").id("place"),
        StatusItem::new("UTF-8").id("encoding"),
      ])
  };
  let mut stage = Stage::new(move |_| {
    Column::new()
      .size(860.0, 300.0)
      .child(Row::new().width(260.0).child(bar(StatusRegion::Trailing)))
      .into()
  });
  assert!(stage.bounds("branch").2 > 0.0, "the item of a higher priority stays");
  assert_eq!(
    stage.bounds("encoding").2,
    0.0,
    "the trailing end gives way first, from its end"
  );
  let (place_x, _, place_width, _) = stage.bounds("place");
  assert!(
    close(place_x + place_width, 260.0),
    "what stays at the end is at the end: {}",
    place_x + place_width
  );
  let mut stage = Stage::new(move |_| {
    Column::new()
      .size(860.0, 300.0)
      .child(Row::new().width(260.0).child(bar(StatusRegion::Leading)))
      .into()
  });
  assert_eq!(stage.bounds("indexing").2, 0.0, "the leading end first");
  assert!(stage.bounds("place").2 > 0.0);
  assert!(stage.bounds("branch").2 > 0.0);
}

#[test]
fn a_text_longer_than_the_bar_ends_in_an_ellipsis_and_a_dropped_item_is_no_stop() {
  let long = "Indexing every file of a workspace far larger than this bar is wide";
  let bar = move |width: f32| {
    Stage::new(move |_| {
      Column::new()
        .size(860.0, 300.0)
        .child(
          Row::new().width(width).child(
            StatusBar::new()
              .leading([StatusItem::new(long).id("long").priority(5)])
              .trailing([StatusItem::new("Rust").id("rust").on_select(|| {})]),
          ),
        )
        .into()
    })
  };
  // A text of 240 at most, cut with an ellipsis.
  let mut stage = bar(860.0);
  let width = stage.bounds("long").2;
  assert!(width > 240.0 && width <= 8.0 + 240.0 + 8.0, "{width}");
  assert!(!stage.paints(long) && stage.cut_text_quad(long).is_some());
  // Too narrow for both, the item of the lower priority leaves.
  let mut stage = bar(270.0);
  assert!(stage.bounds("long").2 > 0.0);
  assert_eq!(stage.bounds("rust").2, 0.0);
  stage.key("Tab");
  assert_ne!(
    focused(&stage).as_deref(),
    Some("rust"),
    "a dropped item takes no focus"
  );
}

#[cfg(any(feature = "devtools", feature = "mcp"))]
#[test]
fn tooling_reads_a_status_and_its_items_by_their_names() {
  let told = Arc::new(Mutex::new(Vec::new()));
  let mut stage = stage(860.0, &told);
  let mut say = |id: &str, name: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(name))
  };
  assert_eq!(say("bar", "a11y_role").as_deref(), Some("status"));
  assert_eq!(say("bar", "a11y_name").as_deref(), Some("Editor status"));
  assert_eq!(
    say("bell", "a11y_name").as_deref(),
    Some("Notifications"),
    "an icon alone is called by its text"
  );
}

#[test]
fn a_themes_rules_and_renderer_change_it() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .size(860.0, 300.0)
      .child(
        StatusBar::new()
          .id("bar")
          .leading(["Ready"])
          .style(StatusBarPart::Root, |s| s.height(30.0)),
      )
      .into()
  });
  assert_eq!(stage.bounds("bar").3, 30.0);
  let app = App::new();
  let renderer: StatusBarRenderer = Arc::new(|view: StatusBarView| {
    Row::new()
      .child(Text::new(&format!("{} items", view.entries.len())))
      .into()
  });
  app.theme().set_renderer::<StatusBar>(renderer);
  let stage = Stage::in_app(app, |_| {
    Column::new()
      .child(StatusBar::new().leading(["Ready"]).trailing(["Rust"]))
      .into()
  });
  assert!(stage.paints("2 items"));
}
