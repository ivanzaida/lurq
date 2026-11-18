//! A timeline: the board's items (the rail, the dot, the line and the texts), the last item, a description of five
//! lines and content under an item that the line runs along, the marks (an icon, media, a spinner), the tones, the
//! current and the upcoming item, an item that is pressed, what tooling reads, and a theme's rules and renderer; the
//! timeline's own `on_select` with its items one focus group, the small size and the density, the time column and
//! alternate sides with their fallback in a narrow window, the timeline across in a scroll area, and the item that
//! shows more.

use std::sync::{
  Arc, Mutex,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{
    App,
    theme::{Density, PaletteColor, ThemePalette},
  },
  components::{Column, Row, Text},
  layout::{
    quad::{Quad, QuadContent},
    text_style::FontWeight,
  },
  node::{color::Color, dimension::Dimension},
  ui::{
    Avatar, Button, Orientation, Timeline, TimelineAlign, TimelineItem, TimelinePart, TimelineRenderer, TimelineSize,
    TimelineVariant, TimelineView,
  },
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// The filled rectangles of `color` that are `width` wide.
fn rects(stage: &Stage, color: Color, width: f32) -> Vec<Quad> {
  stage
    .quads()
    .into_iter()
    .filter(|quad| {
      matches!(&quad.content, QuadContent::Rect { color: fill, .. } if *fill == color)
        && (quad.width - width).abs() < 0.01
    })
    .collect()
}

fn weight_of(stage: &Stage, content: &str) -> FontWeight {
  stage
    .quads()
    .iter()
    .find_map(|quad| match &quad.content {
      QuadContent::Text { text, style, .. } if text == content => Some(style.weight),
      _ => None,
    })
    .unwrap_or_else(|| panic!("no text {content:?} was painted"))
}

/// The board's three items.
fn board() -> Timeline {
  Timeline::new([
    TimelineItem::new("Released 0.60")
      .description("Blur and frosted glass land in the renderer.")
      .time("Nov 10")
      .id("a"),
    TimelineItem::new("Merged node-blur")
      .description("Blurs follow transforms and rounded clips.")
      .time("Nov 8")
      .id("b"),
    TimelineItem::new("Opened pull request")
      .description("Blur a node and its subtree as one layer.")
      .time("Nov 3")
      .upcoming(true)
      .id("c"),
  ])
  .label("History")
  .id("timeline")
}

/// A stage of `timeline` in a column of the board's 300.
fn stage_of(timeline: impl Fn() -> Timeline + Send + Sync + 'static) -> Stage {
  Stage::new(move |_| Column::new().width(316.0).padding(8.0).child(timeline()).into())
}

#[test]
fn an_item_is_a_rail_of_12_with_a_dot_of_10_and_a_line_beside_its_texts() {
  let mut stage = stage_of(board);
  let (x, y, width, height) = stage.bounds("a");
  assert!((width - 300.0).abs() < 0.5, "as wide as the timeline: {width}");

  // The dot: 10, in the primary color, 4 down, in the middle of the rail of 12.
  let dots = rects(&stage, light(PaletteColor::Primary), 10.0);
  assert_eq!(dots.len(), 2, "the upcoming item's dot is hollow");
  assert!((dots[0].x - (x + 1.0)).abs() < 0.5);
  assert!((dots[0].y - (y + 4.0)).abs() < 0.5);
  assert!((dots[0].height - 10.0).abs() < 0.01);

  // The line: 1 in the border's color, 4 under the dot, down to the bottom of the item.
  let lines = rects(&stage, light(PaletteColor::Border), 1.0);
  assert_eq!(lines.len(), 2, "the last item has no line");
  assert!((lines[0].x - (x + 5.5)).abs() < 0.5, "{}", lines[0].x - x);
  assert!((lines[0].y - (y + 18.0)).abs() < 0.5);
  assert!((lines[0].y + lines[0].height - (y + height)).abs() < 0.5);
  // The next item's dot is 4 under the end of the line.
  let (_, b_y, ..) = stage.bounds("b");
  assert!((b_y - (y + height)).abs() < 0.5, "items touch");
  assert!((dots[1].y - (b_y + 4.0)).abs() < 0.5);

  // The texts: 12 from the rail, the title 14 at 500, the description 13 and the time 11 in the muted color.
  let title = stage.text_quad("Released 0.60").unwrap();
  assert!((title.x - (x + 24.0)).abs() < 0.5);
  assert_eq!(stage.text("Released 0.60"), (light(PaletteColor::Foreground), 14.0));
  assert_eq!(weight_of(&stage, "Released 0.60"), FontWeight::Medium);
  assert_eq!(
    stage.text("Blur and frosted glass land in the renderer."),
    (light(PaletteColor::MutedForeground), 13.0)
  );
  assert_eq!(stage.text("Nov 10"), (light(PaletteColor::MutedForeground), 11.0));
  assert!(
    stage.font_of("Nov 10").to_lowercase().contains("mono"),
    "{}",
    stage.font_of("Nov 10")
  );
  // 20 under the time, but under the last item's.
  let time = stage.text_quad("Nov 10").unwrap();
  assert!(
    (y + height - (time.y + time.height) - 20.0).abs() < 1.0,
    "{}",
    y + height - time.y - time.height
  );
  let (_, c_y, _, c_height) = stage.bounds("c");
  let last = stage.text_quad("Nov 3").unwrap();
  assert!((c_y + c_height - (last.y + last.height)).abs() < 1.0);
}

#[test]
fn an_upcoming_item_has_a_hollow_dot() {
  let mut stage = stage_of(board);
  let (x, y, ..) = stage.bounds("c");
  let hollow = stage
    .quads()
    .into_iter()
    .find(|quad| (quad.x - (x + 1.0)).abs() < 0.5 && (quad.y - (y + 4.0)).abs() < 0.5 && quad.border.is_some())
    .expect("the hollow dot");
  assert!(matches!(hollow.content, QuadContent::Rect { color, .. } if color == light(PaletteColor::Background)));
  let edge = hollow.border.unwrap().top.unwrap();
  assert_eq!(edge.color, light(PaletteColor::Input));
  assert!((edge.width - 1.5).abs() < 0.01);
}

/// A description of five lines and content under an item: the line runs along all of it, to the next item.
#[test]
fn the_line_is_as_long_as_what_is_beside_it() {
  let long = "A description that is a good deal longer than the room beside the rail, so that it goes on to the next \
              line, and the one after it, and the one after that, until it fills five lines of the timeline's width \
              or more, in a narrow face as well as in a wide one: San Francisco sets more of it on a line than Segoe UI.";
  let mut stage = stage_of(move || {
    Timeline::new([
      TimelineItem::new("Wrote a long description").description(long).id("a"),
      TimelineItem::new("Attached files")
        .child(Column::new().height(120.0).width(200.0).id("files"))
        .time("Nov 8")
        .id("b"),
      TimelineItem::new("Started").id("c"),
    ])
  });
  let (_, a_y, _, a_height) = stage.bounds("a");
  let description = stage.text_quad(long).expect("the description");
  assert!(
    description.height > 13.0 * 1.4 * 4.5,
    "five lines: {}",
    description.height
  );
  assert!(
    description.width <= 300.0 - 24.0 + 0.5,
    "it wraps in the room the rail leaves"
  );
  let lines = rects(&stage, light(PaletteColor::Border), 1.0);
  assert_eq!(lines.len(), 2);
  assert!((lines[0].y + lines[0].height - (a_y + a_height)).abs() < 0.5);
  assert!(
    lines[0].height > description.height,
    "it runs along the whole description"
  );

  // Content under an item: 8 under the time, and the line runs past it.
  let (_, b_y, _, b_height) = stage.bounds("b");
  let (_, files_y, _, files_height) = stage.bounds("files");
  let time = stage.text_quad("Nov 8").unwrap();
  assert!(
    (files_y - (time.y + time.height) - 8.0).abs() < 1.0,
    "{}",
    files_y - time.y - time.height
  );
  assert!(lines[1].y + lines[1].height >= files_y + files_height);
  assert!((lines[1].y + lines[1].height - (b_y + b_height)).abs() < 0.5);
}

/// A long title wraps beside the action, which keeps its room at the end of the title.
#[test]
fn a_long_title_wraps_beside_the_action() {
  let title = "Reverted the change to the compositor that broke the frosted glass on the second monitor";
  let mut stage =
    stage_of(move || Timeline::new([TimelineItem::new(title).action(Button::new("Undo").id("undo")).id("a")]));
  let (x, _, width, _) = stage.bounds("a");
  let (undo_x, _, undo_width, _) = stage.bounds("undo");
  assert!(
    (undo_x + undo_width - (x + width)).abs() < 0.5,
    "the action is at the end"
  );
  let quad = stage.text_quad(title).unwrap();
  assert!(quad.x + quad.width <= undo_x, "the title stops before the action");
  assert!(quad.height > 30.0, "and wraps: {}", quad.height);
}

#[test]
fn a_tone_colors_the_dot_and_a_theme_variant_has_the_primary_one() {
  let stage = stage_of(|| {
    Timeline::new([
      TimelineItem::new("Built").variant(TimelineVariant::Success),
      TimelineItem::new("Failed").variant(TimelineVariant::Destructive),
      TimelineItem::new("Brand").variant("brand"),
    ])
  });
  assert_eq!(rects(&stage, light(PaletteColor::Success), 10.0).len(), 1);
  assert_eq!(rects(&stage, light(PaletteColor::Destructive), 10.0).len(), 1);
  assert_eq!(rects(&stage, light(PaletteColor::Primary), 10.0).len(), 1);
}

#[test]
fn the_current_item_has_a_ring_around_its_dot() {
  let stage = stage_of(|| Timeline::new([TimelineItem::new("Built"), TimelineItem::new("Deploying").current(true)]));
  assert!(stage.has_ring(light(PaletteColor::Primary)));
  let none = stage_of(|| Timeline::new([TimelineItem::new("Built")]));
  assert!(!none.has_ring(light(PaletteColor::Primary)));
}

/// A mark says what a dot says: the current item's is ringed, the well of an upcoming one hollow.
#[test]
fn a_mark_is_ringed_where_current_and_hollow_where_upcoming() {
  let mut stage = stage_of(|| {
    Timeline::new([
      TimelineItem::new("Deploying").icon("send").current(true),
      TimelineItem::new("Released").icon("star").upcoming(true).id("c"),
    ])
  });
  assert!(stage.has_ring(light(PaletteColor::Primary)), "the current mark's ring");
  let (x, y, ..) = stage.bounds("c");
  let at_well = |quad: &Quad| (quad.x - x).abs() < 0.5 && (quad.y - y).abs() < 0.5 && (quad.width - 24.0).abs() < 0.01;
  let quads = stage.quads();
  // With an icon in it, the well's edge is painted over the icon, apart from its fill.
  assert!(
    quads.iter().any(|quad| at_well(quad)
      && matches!(quad.content, QuadContent::Rect { color, .. } if color == light(PaletteColor::Background))),
    "the well is filled with the background"
  );
  let edge = quads
    .iter()
    .find_map(|quad| at_well(quad).then_some(quad.border).flatten())
    .expect("the well's edge");
  assert_eq!(edge.top.unwrap().color, light(PaletteColor::Input));
  assert!(
    !quads.iter().any(|quad| at_well(quad)
      && matches!(quad.content, QuadContent::Rect { color, .. } if color == light(PaletteColor::Muted))),
    "not the muted fill of a well at rest"
  );
}

/// An icon is in a well of 24 on the muted fill, and every rail of that timeline is 24, the dots in its middle.
#[test]
fn an_icon_stands_in_a_well_and_widens_every_rail() {
  let mut stage = stage_of(|| {
    Timeline::new([
      TimelineItem::new("Commented").icon("message").id("a"),
      TimelineItem::new("Merged").id("b"),
      TimelineItem::new("Reviewed")
        .media(Avatar::new("Ivan Zaida").size("xs"))
        .id("c"),
    ])
  });
  let (x, y, ..) = stage.bounds("a");
  let wells = rects(&stage, light(PaletteColor::Muted), 24.0);
  assert!(
    wells
      .iter()
      .any(|well| (well.x - x).abs() < 0.5 && (well.y - y).abs() < 0.5),
    "the well at the top of the rail"
  );
  let (b_x, ..) = stage.bounds("b");
  let dot = rects(&stage, light(PaletteColor::Primary), 10.0).remove(0);
  assert!(
    (dot.x - (b_x + 7.0)).abs() < 0.5,
    "the dot in the middle of a rail of 24"
  );
  let title = stage.text_quad("Merged").unwrap();
  assert!((title.x - (b_x + 36.0)).abs() < 0.5);
}

#[test]
fn a_pending_item_has_a_spinner_for_its_mark() {
  let still = stage_of(|| Timeline::new([TimelineItem::new("Deploying")]));
  let spinning = stage_of(|| Timeline::new([TimelineItem::new("Deploying").pending(true)]));
  assert!(
    rects(&spinning, light(PaletteColor::Primary), 10.0).is_empty(),
    "no dot"
  );
  // The spinner is a turning glyph: one text more than the title, 12 high in its mark.
  let glyphs = |stage: &Stage| -> Vec<Quad> {
    stage
      .quads()
      .into_iter()
      .filter(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text != "Deploying"))
      .collect()
  };
  assert!(glyphs(&still).is_empty());
  let spinner = glyphs(&spinning);
  assert_eq!(spinner.len(), 1, "the spinner is painted");
  assert!(spinner[0].height <= 12.5, "{}", spinner[0].height);
}

/// An item with `on_select` is a stop of Tab and pressed by the pointer, Enter and Space, every handler told; one
/// without is no stop; a disabled one takes nothing.
#[test]
fn an_item_with_on_select_is_pressed_and_a_stop_of_tab() {
  let told = Arc::new(AtomicUsize::new(0));
  let heard = told.clone();
  let mut stage = Stage::new(move |_| {
    let (first, second, third) = (heard.clone(), heard.clone(), heard.clone());
    Column::new()
      .width(316.0)
      .child(Timeline::new([
        TimelineItem::new("Read").id("read"),
        TimelineItem::new("Released 0.60")
          .on_select(move || {
            first.fetch_add(1, Ordering::Relaxed);
          })
          .on_select(move || {
            second.fetch_add(10, Ordering::Relaxed);
          })
          .id("released"),
        TimelineItem::new("Withdrawn")
          .on_select(move || {
            third.fetch_add(100, Ordering::Relaxed);
          })
          .disabled(true)
          .id("withdrawn"),
      ]))
      .child(Button::new("After").id("after"))
      .into()
  });
  let focused = |stage: &Stage| {
    stage
      .tree
      .focused_element()
      .and_then(|element| element.id().map(str::to_owned))
  };
  stage.key("Tab");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("released"),
    "the item read alone is no stop"
  );
  assert!(stage.has_ring(light(PaletteColor::Ring)));
  stage.key("Enter");
  assert_eq!(told.load(Ordering::Relaxed), 11, "both handlers, in order");
  stage.key("Tab");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("after"),
    "the disabled item is passed by"
  );

  stage.click("released");
  assert_eq!(told.load(Ordering::Relaxed), 22);
  stage.click("withdrawn");
  assert_eq!(told.load(Ordering::Relaxed), 22, "a disabled item takes nothing");
}

#[test]
fn a_rule_and_a_renderer_of_the_theme_change_it() {
  let stage = stage_of(|| board().style(TimelinePart::Separator, |s| s.background(PaletteColor::Primary)));
  assert!(rects(&stage, light(PaletteColor::Border), 1.0).is_empty());
  assert_eq!(rects(&stage, light(PaletteColor::Primary), 1.0).len(), 2);

  let mut app = lurq::app::App::new();
  let renderer: TimelineRenderer = Arc::new(|view: TimelineView| {
    let titles: Vec<lurq::node::Element> = view
      .items
      .iter()
      .map(|item| Text::new(&format!("* {}", item.title)).into())
      .collect();
    Row::new().with_children(titles).into()
  });
  app.theme().set_renderer::<Timeline>(renderer);
  let stage = Stage::in_app(app, |_| board().into());
  assert!(stage.paints("* Released 0.60"));
  assert!(!stage.paints("Nov 10"));
}

/// A timeline is a list of list items to tooling, called by its label, the current one saying so, and an item that
/// is pressed a button in its item.
#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn a_timeline_tells_tooling_its_items() {
  let mut stage = stage_of(|| {
    Timeline::new([
      TimelineItem::new("Built").description("In 2 minutes.").id("built"),
      TimelineItem::new("Deploying")
        .current(true)
        .pending(true)
        .id("deploying"),
      TimelineItem::new("Released").on_select(|| {}).id("released"),
    ])
    .label("Deploy")
    .id("timeline")
  });
  let mut said = |id: &str, name: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(name))
  };
  assert_eq!(said("timeline", "a11y_role").as_deref(), Some("list"));
  assert_eq!(said("timeline", "a11y_name").as_deref(), Some("Deploy"));
  assert_eq!(said("built", "a11y_role").as_deref(), Some("listitem"));
  assert_eq!(said("built", "a11y_name").as_deref(), Some("Built"));
  assert_eq!(said("built", "a11y_description").as_deref(), Some("In 2 minutes."));
  assert_eq!(said("built", "a11y_current"), None);
  assert_eq!(said("deploying", "a11y_current").as_deref(), Some("true"));
  assert_eq!(said("deploying", "a11y_busy").as_deref(), Some("true"));
  assert_eq!(said("released", "a11y_role").as_deref(), Some("button"));
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

/// The steps of an order, each told by its value: the third disabled.
fn steps() -> Vec<TimelineItem<u32>> {
  vec![
    TimelineItem::new("Ordered").value(1).id("s1"),
    TimelineItem::new("Packed").value(2).id("s2"),
    TimelineItem::new("Shipped").value(3).disabled(true).id("s3"),
    TimelineItem::new("Delivered").value(4).id("s4"),
  ]
}

/// The timeline's `on_select` makes every item pressed and is told the item's value, after its own handlers; the
/// items are one stop of Tab, the arrows move between them past a disabled one, and Tab leaves for what is after.
#[test]
fn the_timelines_on_select_is_told_the_value_and_the_items_are_one_stop() {
  let told: Arc<Mutex<Vec<String>>> = Arc::default();
  let heard = told.clone();
  let mut stage = Stage::new(move |_| {
    let (own, chosen) = (heard.clone(), heard.clone());
    let mut items = steps();
    items[0] = TimelineItem::new("Ordered")
      .value(1)
      .on_select(move || own.lock().unwrap().push("own".to_owned()))
      .id("s1");
    Column::new()
      .width(316.0)
      .child(Button::new("Before").id("before"))
      .child(Timeline::new(items).on_select(move |step: &u32| chosen.lock().unwrap().push(format!("step {step}"))))
      .child(Button::new("After").id("after"))
      .into()
  });
  stage.click("s2");
  assert_eq!(*told.lock().unwrap(), ["step 2"]);
  stage.click("s1");
  assert_eq!(told.lock().unwrap()[1..], ["own", "step 1"], "the item's own first");
  stage.click("s3");
  assert_eq!(told.lock().unwrap().len(), 3, "a disabled item takes nothing");

  stage.click("before");
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("s1"), "into the items at the first");
  stage.key("ArrowDown");
  assert_eq!(focused(&stage).as_deref(), Some("s2"));
  stage.key("ArrowDown");
  assert_eq!(focused(&stage).as_deref(), Some("s4"), "past the disabled one");
  stage.key("Enter");
  assert_eq!(told.lock().unwrap().last().map(String::as_str), Some("step 4"));
  stage.key("Home");
  assert_eq!(focused(&stage).as_deref(), Some("s1"));
  stage.key("d");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("s4"),
    "a letter goes to the title it starts"
  );
  stage.key("Tab");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("after"),
    "one stop: Tab leaves the items"
  );
}

/// The small size: a dot of 8, a title of 13 with its time beside it at the end, items 28 apart as rows of the
/// default density, and as rows of the others; the board's size does not move with the density.
#[test]
fn the_small_size_is_a_log_whose_rows_follow_the_density() {
  let log = || {
    Timeline::new([
      TimelineItem::new("Deployed").time("14:02").id("a"),
      TimelineItem::new("Built").time("13:58").id("b"),
      TimelineItem::new("Started").time("13:51").id("c"),
    ])
    .size(TimelineSize::Sm)
  };
  let mut stage = stage_of(log);
  let (x, y, width, _) = stage.bounds("a");
  assert_eq!(rects(&stage, light(PaletteColor::Primary), 8.0).len(), 3);
  assert_eq!(stage.text("Deployed").1, 13.0);
  let (title, time) = (stage.text_quad("Deployed").unwrap(), stage.text_quad("14:02").unwrap());
  assert!(
    (title.x - (x + 18.0)).abs() < 0.5,
    "a rail of 8 and 10 to the text: {}",
    title.x - x
  );
  assert!(
    (time.x + time.width - (x + width)).abs() < 1.0,
    "the time at the end of the title's row"
  );
  assert!(time.y < title.y + title.height, "beside the title, not under it");
  let (_, b_y, ..) = stage.bounds("b");
  assert!((b_y - y - 28.0).abs() < 0.5, "a row of 28: {}", b_y - y);

  for (density, pitch) in [
    (Density::Compact, 21.0),
    (Density::Comfortable, 36.0),
    (Density::Touch, 40.0),
  ] {
    let app = App::new();
    app.theme().set_density(density);
    let mut stage = Stage::in_app(app, move |_| Column::new().width(316.0).child(log()).into());
    let (_, a_y, ..) = stage.bounds("a");
    let (_, b_y, ..) = stage.bounds("b");
    assert!(
      (b_y - a_y - pitch).abs() < 1.5,
      "{density:?}: {} against {pitch}",
      b_y - a_y
    );
  }
  let app = App::new();
  app.theme().set_density(Density::Touch);
  let mut stage = Stage::in_app(app, |_| Column::new().width(316.0).child(board()).into());
  let (_, a_y, _, a_height) = stage.bounds("a");
  let mut plain = stage_of(board);
  assert_eq!(
    a_height,
    plain.bounds("a").3,
    "the board's size keeps its height: {a_y}"
  );
}

/// The time column: the times in a column of 80 before the line, at its end, the rest after the line.
#[test]
fn the_time_column_stands_before_the_line() {
  let mut stage = stage_of(|| board().align(TimelineAlign::TimeColumn));
  let (x, ..) = stage.bounds("a");
  let dot = rects(&stage, light(PaletteColor::Primary), 10.0).remove(0);
  assert!(
    (dot.x - (x + 80.0 + 12.0 + 1.0)).abs() < 0.5,
    "the rail after the column: {}",
    dot.x - x
  );
  let time = stage.text_quad("Nov 10").unwrap();
  assert!(
    (time.x + time.width - (x + 80.0)).abs() < 1.0,
    "the time at the column's end"
  );
  let title = stage.text_quad("Released 0.60").unwrap();
  assert!(
    (title.x - (dot.x - 1.0 + 12.0 + 12.0)).abs() < 0.5,
    "the title after the line"
  );
  assert!(time.y < title.y + title.height, "the time by the title");
}

/// Alternate sides: from the medium breakpoint up the second item's text is before the line, at its end, and its
/// time after it; in a narrower window every item's text is after the line.
#[test]
fn alternate_sides_fall_back_to_one_side_in_a_narrow_window() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .width(600.0)
      .child(board().align(TimelineAlign::Alternate))
      .into()
  });
  stage.tree.resize(900, 600);
  stage.pass();
  let (x, _, width, _) = stage.bounds("b");
  let middle = x + width / 2.0;
  let first = stage.text_quad("Released 0.60").unwrap();
  let second = stage.text_quad("Merged node-blur").unwrap();
  assert!(first.x > middle, "the first after the line");
  assert!(second.x + second.width < middle, "the second before it");
  assert!(
    (second.x + second.width - (middle - 6.0 - 12.0)).abs() < 1.5,
    "at the end of its side, 12 from the rail: {}",
    middle - second.x - second.width
  );
  let time = stage.text_quad("Nov 8").unwrap();
  assert!(time.x > middle, "its time on the other side");
  let dots = rects(&stage, light(PaletteColor::Primary), 10.0);
  assert!(
    dots.iter().all(|dot| (dot.x + 5.0 - middle).abs() < 0.5),
    "the line down the middle"
  );

  stage.tree.resize(500, 600);
  stage.pass();
  let (x, ..) = stage.bounds("b");
  let second = stage.text_quad("Merged node-blur").unwrap();
  assert!((second.x - (x + 24.0)).abs() < 0.5, "500 wide: after the line");
  let time = stage.text_quad("Nov 8").unwrap();
  assert!(time.y > second.y, "and its time under it");
}

/// Across: the marks on a line, each item's text under its mark, the items sharing the width and wrapping their
/// text; more than the room scrolls sideways, and the keys bring the focused item into view.
#[test]
fn across_the_items_share_the_width_and_scroll_to_the_focused_one() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .width(600.0)
      .child(
        Timeline::new([
          TimelineItem::new("Ordered").time("Nov 1").id("a"),
          TimelineItem::new("Packed and handed to the courier at the warehouse in Lviv").id("b"),
          TimelineItem::new("Delivered").upcoming(true).id("c"),
        ])
        .orientation(Orientation::Horizontal)
        .id("order"),
      )
      .into()
  });
  let (a_x, a_y, a_width, _) = stage.bounds("a");
  let (b_x, b_y, b_width, _) = stage.bounds("b");
  assert!(
    (a_width - 200.0).abs() < 0.5 && (b_width - 200.0).abs() < 0.5,
    "{a_width} {b_width}"
  );
  assert!((b_x - (a_x + 200.0)).abs() < 0.5 && (a_y - b_y).abs() < 0.5);
  let dots = rects(&stage, light(PaletteColor::Primary), 10.0);
  assert_eq!(dots.len(), 2);
  assert!((dots[0].y - dots[1].y).abs() < 0.5, "the marks on one line");
  assert!((dots[0].x - a_x).abs() < 0.5);
  let lines: Vec<Quad> = stage
    .quads()
    .into_iter()
    .filter(|quad| {
      matches!(&quad.content, QuadContent::Rect { color, .. } if *color == light(PaletteColor::Border))
        && (quad.height - 1.0).abs() < 0.01
    })
    .collect();
  assert_eq!(lines.len(), 2, "a line across from each mark but the last");
  assert!((lines[0].x - (a_x + 14.0)).abs() < 0.5, "4 after the mark");
  assert!(
    (lines[0].x + lines[0].width - (a_x + 196.0)).abs() < 0.5,
    "4 short of the next"
  );
  let title = stage.text_quad("Ordered").unwrap();
  let running = stage.text_quad("Delivered").unwrap();
  assert!(
    (running.y - title.y).abs() < 0.5,
    "every title on one line, the hollow mark's too"
  );
  assert!(
    (title.y - (dots[0].y - 1.0 + 12.0 + 8.0)).abs() < 1.0,
    "8 under the rail of 12, the dot in its middle: {}",
    title.y - dots[0].y
  );
  let long = stage
    .text_quad("Packed and handed to the courier at the warehouse in Lviv")
    .unwrap();
  assert!(
    long.width <= 200.0 - 12.0 + 0.5 && long.height > 30.0,
    "it wraps in its item"
  );

  // Ten items: each at its least, 160, the row wider than the room, and the focused one brought into view.
  let mut stage = Stage::new(|_| {
    Column::new()
      .width(600.0)
      .child(
        Timeline::new((0..10u32).map(|index| {
          TimelineItem::new(&if index == 3 {
            "Step 3 waited at the warehouse for a courier".to_owned()
          } else {
            format!("Step {index}")
          })
          .value(index)
          .id(format!("s{index}"))
        }))
        .orientation(Orientation::Horizontal)
        .on_select(|_| {}),
      )
      .into()
  });
  assert!((stage.bounds("s0").2 - 160.0).abs() < 0.5);
  // A pressed item's long title wraps in it, short of the next.
  let long = stage.text_quad("Step 3 waited at the warehouse for a courier").unwrap();
  let (s4_x, ..) = stage.bounds("s4");
  assert!(
    long.x + long.width <= s4_x && long.height > 30.0,
    "{} against {s4_x}",
    long.x + long.width
  );
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("s0"));
  stage.key("ArrowRight");
  assert_eq!(focused(&stage).as_deref(), Some("s1"), "the arrows go across");
  stage.key("End");
  stage.pass();
  assert_eq!(focused(&stage).as_deref(), Some("s9"));
  let last = stage.text_quad("Step 9").unwrap();
  assert!(
    last.x + last.width <= 600.5 && last.x >= 0.0,
    "scrolled into view: {}",
    last.x
  );
}

/// With `on_load_more` the line runs on to an item that shows more: its button calls the handler; while loading
/// its mark is a spinner and the button says so and takes no press.
#[test]
fn the_item_that_shows_more_loads_the_next_page() {
  let calls = Arc::new(AtomicUsize::new(0));
  let loading = Arc::new(std::sync::atomic::AtomicBool::new(false));
  let (count, busy) = (calls.clone(), loading.clone());
  let mut stage = stage_of(move || {
    let count = count.clone();
    board()
      .on_load_more(move || {
        count.fetch_add(1, Ordering::Relaxed);
      })
      .loading(busy.load(Ordering::Relaxed))
  });
  assert_eq!(
    rects(&stage, light(PaletteColor::Border), 1.0).len(),
    3,
    "the last item's line runs on"
  );
  assert!(stage.paints("Show more"));
  let (x, y, ..) = stage.bounds("c");
  let (_, c_y, _, c_height) = stage.bounds("c");
  assert!(c_height > 0.0 && c_y >= y && x >= 0.0);
  let button = stage.text_quad("Show more").unwrap();
  let center = (button.x + button.width / 2.0, button.y + button.height / 2.0);
  stage.tree.mouse_move(center.0, center.1);
  stage
    .tree
    .mouse_down(center.0, center.1, lurq::app::events::MouseButton::Left);
  stage
    .tree
    .mouse_up(center.0, center.1, lurq::app::events::MouseButton::Left);
  stage.pass();
  assert_eq!(calls.load(Ordering::Relaxed), 1);

  loading.store(true, Ordering::Relaxed);
  let mut stage = {
    let busy = loading.clone();
    stage_of(move || {
      board()
        .on_load_more(|| {})
        .loading(busy.load(Ordering::Relaxed))
        .more_label("Older")
    })
  };
  assert!(stage.paints("Loading") && !stage.paints("Older"));
  let glyphs = stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text.chars().all(|c| ('\u{E000}'..='\u{F8FF}').contains(&c))))
    .count();
  assert!(glyphs >= 1, "a spinner for its mark");
  let _ = stage.bounds("c");
}

/// The item that shows more stands on the line where the items' marks are: after the time column, or in the middle.
#[test]
fn the_item_that_shows_more_stands_on_the_line() {
  for align in [TimelineAlign::TimeColumn, TimelineAlign::Alternate] {
    let mut stage = Stage::new(move |_| {
      Column::new()
        .width(600.0)
        .child(board().align(align).on_load_more(|| {}))
        .into()
    });
    stage.tree.resize(900, 600);
    stage.pass();
    let dot = rects(&stage, light(PaletteColor::Primary), 10.0).remove(0);
    let edges: Vec<Quad> = stage
      .quads()
      .into_iter()
      .filter(|quad| {
        (quad.width - 10.0).abs() < 0.01
          && quad.border.and_then(|border| border.top).map(|side| side.color) == Some(light(PaletteColor::Input))
      })
      .collect();
    let more = edges.last().expect("the hollow dot of the item that shows more");
    assert!((more.x - dot.x).abs() < 0.5, "{align:?}: {} against {}", more.x, dot.x);
  }
}

/// One frame: renders, lays out and paints what changed, once.
fn frame(stage: &mut Stage) {
  stage.tree.request_redraw();
  stage.tree.pass(&mut stage.app, &crate::support::TestSurface);
}

/// Whether the board's timeline in the stage has its second item's text before the line: alternate sides.
fn alternating(stage: &mut Stage) -> bool {
  let (x, _, width, _) = stage.bounds("b");
  let second = stage.text_quad("Merged node-blur").unwrap();
  second.x + second.width < x + width / 2.0
}

/// Mounted, a timeline's alternate sides follow its own room, not the window: in a pane of a splitter, a pane
/// narrower than the least width folds them to one side, and the frame that lays the pane out already shows the
/// form that fits as the divider is dragged across that width.
#[test]
fn a_mounted_timeline_folds_by_its_own_room_in_the_frame_a_divider_moves_it() {
  let mut stage = Stage::new(move |ctx| {
    lurq::ui::Splitter::new()
      .id("split")
      .pane(
        lurq::ui::SplitterPane::new(
          Column::new()
            .size(
              lurq::node::dimension::Dimension::Pct(100.0),
              lurq::node::dimension::Dimension::Pct(100.0),
            )
            .child(board().align(TimelineAlign::Alternate).mount(ctx)),
        )
        .length(600.0)
        .min(100.0),
      )
      .pane(
        lurq::ui::SplitterPane::new(
          Column::new()
            .id("other")
            .size(Dimension::Pct(100.0), Dimension::Pct(100.0)),
        )
        .min(40.0),
      )
      .into()
  });
  // A window of 1000 by 400, wide past the md breakpoint: what folds the sides is the pane's room alone.
  stage
    .tree
    .set_layout_constraints_override(Some(lurq::layout::Constraints::tight(lurq::layout::Size::new(
      1000.0, 400.0,
    ))));
  stage.tree.resize(1000, 400);
  stage.pass();
  assert!(alternating(&mut stage), "a wide pane: two sides");

  // Dragged by the pointer, across the least width and back, a frame at each place.
  let (x, ..) = stage.bounds("other");
  let divider = x - 0.5;
  stage.tree.mouse_move(divider, 200.0);
  stage
    .tree
    .mouse_down(divider, 200.0, lurq::app::events::MouseButton::Left);
  frame(&mut stage);
  for (to, wide) in [(560.0, true), (430.0, false), (500.0, true), (350.0, false)] {
    stage.tree.mouse_move(to, 200.0);
    frame(&mut stage);
    assert_eq!(alternating(&mut stage), wide, "dragged to {to}, in that frame");
  }
  stage.tree.mouse_up(350.0, 200.0, lurq::app::events::MouseButton::Left);
  frame(&mut stage);
  assert!(!alternating(&mut stage));
}

/// Built as an element of its own, a timeline has no room to read: its sides follow the window's breakpoint.
#[test]
fn a_timeline_built_as_an_element_folds_by_the_window() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .width(300.0)
      .child(board().align(TimelineAlign::Alternate))
      .into()
  });
  stage.tree.resize(900, 600);
  stage.pass();
  assert!(alternating(&mut stage), "a wide window: two sides, in a column of 300");
}
