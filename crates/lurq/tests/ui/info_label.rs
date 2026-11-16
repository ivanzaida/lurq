//! An info label: the label and its mark against the board, a label with
//! nothing to explain, the tooltip the pointer and the keyboard show and
//! where it stands, a label longer than the room, what tooling reads, a
//! theme's rules and anatomy, and the same mark after a field's label.

use std::{
  sync::Arc,
  time::{Duration, Instant},
};

use lurq::{
  app::{
    App,
    ctx::Placement,
    theme::{PaletteColor, ThemePalette},
  },
  components::{Column, Row},
  core::Signal,
  layout::quad::{Quad, QuadContent},
  node::color::Color,
  ui::{Field, InfoLabel, InfoLabelPart, InfoLabelRenderer, InfoLabelView, Input, icons},
};

use super::stage::Stage;

const WORDS: &str = "Stored in the OS keychain";

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// The character the theme's icon font draws `name` with.
fn glyph(name: &str) -> String {
  icons::ALL
    .iter()
    .find(|(held, _)| *held == name)
    .map(|(_, glyph)| glyph.to_string())
    .unwrap_or_else(|| panic!("no icon {name}"))
}

fn api_key() -> InfoLabel {
  InfoLabel::new("API key").info(WORDS).id("key")
}

/// The label mounted 100 in from the window's corner, with room for its tooltip above it.
fn mounted(label: impl Fn() -> InfoLabel + Send + Sync + 'static) -> Stage {
  Stage::new(move |ctx| {
    Column::new()
      .size(400.0, 300.0)
      .padding(100.0)
      .child(Row::new().child(label().mount(ctx)))
      .into()
  })
}

/// The tooltip's box: the smallest fill around its text.
fn bubble(stage: &Stage, text: &str) -> Option<Quad> {
  let words = stage.text_quad(text)?;
  stage
    .quads()
    .into_iter()
    .filter(|quad| {
      matches!(&quad.content, QuadContent::Rect { color, .. } if color.a() > 0)
        && quad.x <= words.x + 0.01
        && quad.y <= words.y + 0.01
        && quad.x + quad.width >= words.x + words.width - 0.01
        && quad.y + quad.height >= words.y + words.height - 0.01
    })
    .min_by(|a, b| (a.width * a.height).total_cmp(&(b.width * b.height)))
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

#[test]
fn an_info_label_is_a_field_labels_text_with_an_i_of_14_6_after_it() {
  let mut stage = Stage::new(|_| Column::new().padding(24.0).child(Row::new().child(api_key())).into());
  assert_eq!(stage.text("API key"), (light(PaletteColor::Foreground), 13.0));
  let info = glyph(icons::INFO);
  assert_eq!(stage.text(&info), (light(PaletteColor::MutedForeground), 14.0));
  let label = stage.text_quad("API key").unwrap();
  let (x, y, width, height) = stage.bounds("key-mark");
  assert_eq!((width, height), (14.0, 14.0));
  assert!(
    (x - (label.x + label.width) - 6.0).abs() < 0.01,
    "6 after the text: {}",
    x - (label.x + label.width)
  );
  assert!(
    ((y + 7.0) - (label.y + label.height / 2.0)).abs() < 0.51,
    "level with the text"
  );
  assert!(!stage.paints(WORDS), "the still form shows nothing");
}

#[test]
fn a_label_with_nothing_to_explain_has_no_mark() {
  let mut stage = Stage::new(|ctx| {
    Column::new()
      .padding(24.0)
      .child(Row::new().child(InfoLabel::new("Region").id("region").mount(ctx)))
      .into()
  });
  assert!(stage.paints("Region"));
  assert!(!stage.paints(&glyph(icons::INFO)));
  assert!(stage.tree.get_element_by_id_mut("region-mark").is_none());
  stage.key("Tab");
  assert_eq!(focused(&stage), None, "nothing in it is a stop of Tab");
}

#[test]
fn the_pointer_resting_on_the_label_or_its_mark_shows_the_tooltip_after_its_delay() {
  for on in ["key", "key-mark"] {
    let mut stage = mounted(api_key);
    assert!(!stage.paints(WORDS), "nothing shows at rest");
    stage.hover(on);
    assert!(!stage.paints(WORDS), "{on}: not before a tooltip's delay");
    stage.tree.tick_timers_at(Instant::now() + Duration::from_millis(600));
    stage.pass();
    assert!(stage.paints(WORDS), "{on}: after it");
  }
}

#[test]
fn the_tooltip_stands_above_the_label_from_its_start() {
  let mut stage = mounted(|| api_key().force_open(Some(true)));
  let shown = bubble(&stage, WORDS).expect("the tooltip");
  let (x, y, ..) = stage.bounds("key");
  assert_eq!(
    stage.text(WORDS),
    (light(PaletteColor::PrimaryForeground), 12.0),
    "the theme's tooltip"
  );
  assert!(
    (shown.y + shown.height - (y - 6.0)).abs() < 0.5,
    "6 above the label: {} to {}, the label at {y}",
    shown.y,
    shown.y + shown.height
  );
  assert!((shown.x - x).abs() < 0.5, "from the label's start: {} and {x}", shown.x);
}

#[test]
fn a_placement_puts_the_tooltip_on_another_side() {
  let mut stage = mounted(|| api_key().placement(Placement::Bottom).force_open(Some(true)));
  let shown = bubble(&stage, WORDS).expect("the tooltip");
  let (x, y, width, height) = stage.bounds("key");
  assert!((shown.y - (y + height + 6.0)).abs() < 0.5, "6 under the label");
  assert!(
    (shown.x + shown.width / 2.0 - (x + width / 2.0)).abs() < 0.5,
    "centred on the label"
  );
}

#[test]
fn tab_stops_on_the_mark_which_shows_the_tooltip_at_once_and_escape_puts_it_away() {
  let mut stage = mounted(api_key);
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("key-mark"));
  assert!(
    stage.paints(WORDS),
    "the focus from the keyboard shows it with no delay"
  );
  assert!(stage.has_ring(light(PaletteColor::Ring)), "the mark has the ring");
  assert_eq!(
    stage.text(&glyph(icons::INFO)).0,
    light(PaletteColor::Foreground),
    "and is in the foreground while it is focused"
  );
  stage.key("Escape");
  assert!(!stage.paints(WORDS), "Escape puts it away");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("key-mark"),
    "the focus stays on the mark"
  );
}

#[test]
fn a_label_longer_than_the_room_ends_in_an_ellipsis_and_the_mark_stays_after_it() {
  const LABEL: &str = "The key of the service account the nightly export signs its requests with";
  let mut stage = Stage::new(|ctx| {
    Column::new()
      .size(400.0, 300.0)
      .child(
        Row::new()
          .id("room")
          .width(150.0)
          .child(InfoLabel::new(LABEL).info(WORDS).id("key").mount(ctx)),
      )
      .into()
  });
  let (room, _, room_width, _) = stage.bounds("room");
  let (x, _, width, _) = stage.bounds("key");
  assert!(
    x >= room && x + width <= room + room_width + 0.5,
    "no wider than its room: {width}"
  );
  assert!(stage.text_quad(LABEL).is_none());
  let cut = stage.cut_text_quad(LABEL).expect("cut with an ellipsis");
  let (mark, _, mark_width, _) = stage.bounds("key-mark");
  assert_eq!(mark_width, 14.0, "the mark keeps its side");
  assert!(mark + mark_width <= room + room_width + 0.5, "and is in the room");
  assert!(cut.x + cut.width <= mark - 6.0 + 0.5, "6 after the text");
}

#[cfg(any(feature = "devtools", feature = "mcp"))]
#[test]
fn tooling_reads_the_mark_as_an_image_described_by_what_it_explains() {
  let mut stage = Stage::new(|ctx| {
    Column::new()
      .size(400.0, 300.0)
      .padding(100.0)
      .spacing(12.0)
      .child(Row::new().child(api_key().mount(ctx)))
      .child(
        Row::new().child(
          InfoLabel::new("Region")
            .info("Where the data is kept")
            .info_label("About the region")
            .id("region"),
        ),
      )
      .into()
  });
  let said = |stage: &mut Stage, id: &str, name: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(name))
  };
  assert_eq!(said(&mut stage, "key-mark", "a11y_role").as_deref(), Some("img"));
  assert_eq!(
    said(&mut stage, "key-mark", "a11y_name").as_deref(),
    Some("More information")
  );
  assert_eq!(said(&mut stage, "key-mark", "a11y_description").as_deref(), Some(WORDS));
  // The still form says as much.
  assert_eq!(
    said(&mut stage, "region-mark", "a11y_name").as_deref(),
    Some("About the region")
  );
  assert_eq!(
    said(&mut stage, "region-mark", "a11y_description").as_deref(),
    Some("Where the data is kept")
  );
}

#[test]
fn a_theme_restyles_an_info_label_or_gives_it_another_anatomy() {
  let app = App::new();
  app.theme().patch_recipe::<InfoLabel>(|rules| {
    rules
      .base(InfoLabelPart::Root, |s| s.gap(12.0))
      .base(InfoLabelPart::Mark, |s| s.square(18.0).foreground(PaletteColor::Info))
  });
  let mut stage = Stage::in_app(app, |_| {
    Column::new().padding(24.0).child(Row::new().child(api_key())).into()
  });
  let label = stage.text_quad("API key").unwrap();
  let (x, _, width, _) = stage.bounds("key-mark");
  assert_eq!(width, 18.0);
  assert!((x - (label.x + label.width) - 12.0).abs() < 0.01);
  assert_eq!(stage.text(&glyph(icons::INFO)), (light(PaletteColor::Info), 18.0));

  let app = App::new();
  let renderer: InfoLabelRenderer = Arc::new(|view: InfoLabelView| {
    // The mark before the text.
    let part = view.part(InfoLabelPart::Label);
    let mut root = view.root();
    if let Some(mark) = view.mark() {
      root = root.child(mark);
    }
    root.child(part.text(&view.label)).into()
  });
  app.theme().set_renderer::<InfoLabel>(renderer);
  let mut stage = Stage::in_app(app, |ctx| {
    Column::new()
      .size(400.0, 300.0)
      .padding(100.0)
      .child(Row::new().child(api_key().mount(ctx)))
      .into()
  });
  let label = stage.text_quad("API key").unwrap();
  let (x, ..) = stage.bounds("key-mark");
  assert!(x < label.x, "the theme's label has its mark first");
  stage.key("Tab");
  assert!(stage.paints(WORDS), "and still shows its tooltip when it should");
}

#[test]
fn a_field_draws_the_same_mark_after_its_label_and_a_mounted_one_shows_its_tooltip() {
  let value = Signal::new(String::new());
  let mut stage = Stage::new(move |ctx| {
    Column::new()
      .size(400.0, 300.0)
      .padding(100.0)
      .child(
        Field::new("API key", Input::new(value.clone()).id("input"))
          .info(WORDS)
          .required(true)
          .id("field")
          .mount(ctx),
      )
      .into()
  });
  let info = glyph(icons::INFO);
  assert_eq!(stage.text(&info), (light(PaletteColor::MutedForeground), 14.0));
  let star = stage.text_quad("*").expect("the mark of a required field");
  let mark = stage.text_quad(&info).unwrap();
  assert!(
    (mark.x - (star.x + star.width) - 6.0).abs() < 0.01,
    "6 after what is before it: {}",
    mark.x - (star.x + star.width)
  );
  assert!(!stage.paints(WORDS));
  // The mark is the first stop of Tab in the field, before its control, and the focus shows its tooltip.
  stage.key("Tab");
  assert!(stage.paints(WORDS));
  let shown = bubble(&stage, WORDS).unwrap();
  assert!(
    (shown.x + shown.width / 2.0 - (mark.x + 7.0)).abs() < 0.5,
    "centred over the mark"
  );
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("input"));
  assert!(!stage.paints(WORDS), "it goes as the focus leaves the mark");

  // Converted into an element, the mark is there and nothing shows on it.
  let value = Signal::new(String::new());
  let mut stage = Stage::new(move |_| {
    Column::new()
      .size(400.0, 300.0)
      .padding(100.0)
      .child(Field::new("Region", Input::new(value.clone())).info(WORDS))
      .into()
  });
  assert!(stage.paints(&info));
  stage.key("Tab");
  assert!(!stage.paints(WORDS));
}
