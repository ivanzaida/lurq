//! A timeline: the board's items (the rail, the dot, the line and the texts), the last item, a description of five
//! lines and content under an item that the line runs along, the marks (an icon, media, a spinner), the tones, the
//! current and the upcoming item, an item that is pressed, what tooling reads, and a theme's rules and renderer.

use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::theme::{PaletteColor, ThemePalette},
  components::{Column, Row, Text},
  layout::{
    quad::{Quad, QuadContent},
    text_style::FontWeight,
  },
  node::color::Color,
  ui::{Avatar, Button, Timeline, TimelineItem, TimelinePart, TimelineRenderer, TimelineVariant, TimelineView},
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
              line, and the one after it, and the one after that, until it fills five lines of the timeline's width.";
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
