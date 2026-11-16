//! `ui::BulletedList`: the design's list (`btfDc`), its numbers, icons and marks, and a list in an item.

use lurq::{
  app::{
    App,
    theme::{PaletteColor, ThemePalette},
  },
  components::{Column, Rect, Text},
  layout::quad::{Quad, QuadContent},
  node::color::Color,
  ui::{BulletedList, BulletedListItem, BulletedListMarker, BulletedListPart, icons},
};

use super::stage::Stage;

const ROLES: [&str; 3] = [
  "Semantic color roles",
  "Light and dark palettes",
  "Extra roles for app-specific needs",
];

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn staged(scene: impl Fn() -> BulletedList + Send + Sync + 'static) -> Stage {
  let app = App::new();
  app
    .theme()
    .set_preferred_appearance(Some(lurq::app::window::Appearance::Light));
  Stage::in_app(app, move |_| Column::new().width(380.0).child(scene()).into())
}

/// The filled boxes of 5 by 5: the dots.
fn dots(stage: &Stage) -> Vec<Quad> {
  stage
    .quads()
    .into_iter()
    .filter(|quad| {
      (quad.width, quad.height) == (5.0, 5.0)
        && matches!(&quad.content, QuadContent::Rect { color, .. } if color.a() > 0)
    })
    .collect()
}

/// The boxes of 5 by 5 with an edge and no fill: the rings.
fn rings(stage: &Stage) -> Vec<Quad> {
  stage
    .quads()
    .into_iter()
    .filter(|quad| (quad.width, quad.height) == (5.0, 5.0) && quad.border.is_some_and(|border| border.top.is_some()))
    .collect()
}

fn text_of(stage: &Stage, content: &str) -> Quad {
  stage
    .text_quad(content)
    .unwrap_or_else(|| panic!("{content:?} is painted"))
}

#[test]
fn the_list_is_the_boards() {
  let mut stage = staged(|| BulletedList::new(ROLES).id("list"));
  let (x, y, width, height) = stage.bounds("list");
  assert_eq!((x, y, width), (0.0, 0.0, 380.0), "it fills its room");
  // Three lines of 17 with 6 between them: 63, as the board's.
  assert!((height - 63.0).abs() < 1.5, "{height}");
  let dots = dots(&stage);
  assert_eq!(dots.len(), 3, "a dot for each item");
  for (dot, words) in dots.iter().zip(ROLES) {
    let text = text_of(&stage, words);
    assert_eq!(dot.x, 0.0, "the dot is at the list's start");
    assert_eq!(text.x, 15.0, "the text is 10 after a dot of 5");
    // The dot stands in the middle of its line.
    let middle = text.y + text.height / 2.0;
    assert!((dot.y + 2.5 - middle).abs() < 0.51, "{} against {middle}", dot.y + 2.5);
    assert!(
      matches!(&dot.content, QuadContent::Rect { color, .. } if *color == light(PaletteColor::MutedForeground)),
      "the dot is in MutedForeground"
    );
    assert_eq!(stage.text(words), (light(PaletteColor::Foreground), 14.0));
  }
  // The items are 6 apart.
  let (first, second) = (text_of(&stage, ROLES[0]), text_of(&stage, ROLES[1]));
  assert!((second.y - (first.y + first.height) - 6.0).abs() < 0.01);
}

#[test]
fn a_numbered_list_sets_its_numbers_at_the_end_of_their_room() {
  let stage = staged(|| BulletedList::new(["Install", "Configure", "Run"]).marker(BulletedListMarker::Number));
  assert!(dots(&stage).is_empty(), "a numbered list has no dots");
  for (number, words) in ["1.", "2.", "3."].into_iter().zip(["Install", "Configure", "Run"]) {
    let (mark, text) = (text_of(&stage, number), text_of(&stage, words));
    assert!((mark.x + mark.width - 18.0).abs() < 0.01, "the number ends at 18");
    assert_eq!(text.x, 26.0, "the text is 8 after the numbers' 18");
    assert_eq!(stage.text(number).0, light(PaletteColor::MutedForeground));
  }
  // It starts where it is told, and writes its numbers as it is told.
  let stage = staged(|| {
    BulletedList::new(["Fourth", "Fifth"])
      .marker(BulletedListMarker::Number)
      .start(4)
      .number_text(|number| format!("{number})"))
  });
  assert!(stage.paints("4)") && stage.paints("5)"));
}

#[test]
fn an_icon_or_a_mark_of_the_apps_stands_for_the_bullet() {
  // An icon for every item: the name of one is a marker.
  let stage = staged(|| BulletedList::new(["Signed", "Notarized"]).marker(icons::CHECK));
  assert!(dots(&stage).is_empty());
  assert_eq!(text_of(&stage, "Signed").x, 24.0, "10 after an icon of 14");
  assert_eq!(text_of(&stage, "Notarized").x, 24.0);
  // One item's own icon and another's own element: every line starts after an icon's room, and the item with
  // neither keeps its dot.
  let mut stage = staged(|| {
    BulletedList::new([
      BulletedListItem::new("Plain"),
      BulletedListItem::new("Checked").icon(icons::CHECK),
      BulletedListItem::new("Marked").media(Rect::new(8.0, 8.0).id("mark")),
    ])
  });
  assert_eq!(dots(&stage).len(), 1);
  for words in ["Plain", "Checked", "Marked"] {
    assert_eq!(text_of(&stage, words).x, 24.0, "{words}");
  }
  let (mark_x, mark_y, ..) = stage.bounds("mark");
  let line = text_of(&stage, "Marked");
  assert_eq!(mark_x, 3.0, "the app's mark is in the middle of the room");
  assert!((mark_y + 4.0 - (line.y + line.height / 2.0)).abs() < 0.51);
  // With no marker the lines stand alone, at the list's start.
  let stage = staged(|| BulletedList::new(["Alone"]).marker(BulletedListMarker::None));
  assert!(dots(&stage).is_empty());
  assert_eq!(text_of(&stage, "Alone").x, 0.0);
}

#[test]
fn a_list_in_an_item_is_set_in_and_has_rings() {
  let stage = staged(|| {
    BulletedList::new(["Tokens"]).item(
      BulletedListItem::new("Components")
        .list(BulletedList::new(["Button"]).item(BulletedListItem::new("Switch").list(BulletedList::new(["Thumb"])))),
    )
  });
  assert_eq!(dots(&stage).len(), 2, "the outer list's dots");
  let rings = rings(&stage);
  assert_eq!(rings.len(), 3, "a ring for each item of the lists in it");
  assert_eq!(rings[0].x, 15.0, "a list in an item starts where the item's text does");
  assert_eq!(text_of(&stage, "Button").x, 30.0);
  assert_eq!(text_of(&stage, "Thumb").x, 45.0);
  assert_eq!(
    rings[0].border.unwrap().top.unwrap().color,
    light(PaletteColor::MutedForeground)
  );
  // The list in the item is 6 under the item's line.
  let (line, inner) = (text_of(&stage, "Components"), text_of(&stage, "Button"));
  assert!((inner.y - (line.y + line.height) - 6.0).abs() < 0.01);
  // A list in an item keeps the marker it was given.
  let stage = staged(|| {
    BulletedList::new([
      BulletedListItem::new("Steps").list(BulletedList::new(["Install", "Run"]).marker(BulletedListMarker::Number))
    ])
  });
  assert!(stage.paints("1.") && stage.paints("2."));
  assert_eq!(text_of(&stage, "Install").x, 15.0 + 26.0);
}

#[test]
fn a_long_item_wraps_and_keeps_its_dot_on_its_first_line() {
  const LONG: &str = "Extra roles for app-specific needs, which a theme names and a recipe reads as it reads the \
                      roles lurq has, in light and in dark";
  let mut stage = staged(|| {
    BulletedList::new([
      BulletedListItem::new(LONG).id("long"),
      BulletedListItem::new("Short").id("short"),
    ])
    .id("list")
  });
  let (_, _, list_width, _) = stage.bounds("list");
  let (_, long_y, long_width, long_height) = stage.bounds("long");
  let (_, short_y, _, short_height) = stage.bounds("short");
  assert_eq!(
    (list_width, long_width),
    (380.0, 380.0),
    "it does not run past its room"
  );
  assert!(
    long_height > short_height * 1.9,
    "{long_height}: it goes on to a second line"
  );
  assert!((short_y - (long_y + long_height) - 6.0).abs() < 0.01);
  // The dot is in the middle of the first line, not of the item.
  let dot = dots(&stage).remove(0);
  assert!((dot.y + 2.5 - short_height / 2.0).abs() < 0.51, "{}", dot.y);
}

#[test]
fn an_item_holds_elements_under_its_line_or_alone() {
  let mut stage = staged(|| {
    BulletedList::new([
      BulletedListItem::new("With a note").child(Text::new("The note.").id("note")),
      BulletedListItem::new("").child(Text::new("Alone.").id("alone")),
    ])
  });
  let line = text_of(&stage, "With a note");
  let (note_x, note_y, ..) = stage.bounds("note");
  assert_eq!(note_x, 15.0);
  assert!((note_y - (line.y + line.height) - 6.0).abs() < 0.01, "6 under the line");
  let (alone_x, ..) = stage.bounds("alone");
  assert_eq!(alone_x, 15.0);
  assert_eq!(dots(&stage).len(), 2);
}

#[test]
fn a_theme_draws_it_otherwise() {
  let stage = staged(|| {
    BulletedList::new(ROLES)
      .style(BulletedListPart::Bullet, |s| s.background(PaletteColor::Primary))
      .style(BulletedListPart::Root, |s| s.gap(12.0))
  });
  let dots = dots(&stage);
  assert!(matches!(&dots[0].content, QuadContent::Rect { color, .. } if *color == light(PaletteColor::Primary)));
  let (first, second) = (text_of(&stage, ROLES[0]), text_of(&stage, ROLES[1]));
  assert!((second.y - (first.y + first.height) - 12.0).abs() < 0.01);
  // What is said of a list is said of the lists in its items.
  let stage = staged(|| {
    BulletedList::new([BulletedListItem::new("Outer").list(BulletedList::new(["Inner"]))])
      .style(BulletedListPart::Label, |s| s.foreground(PaletteColor::Primary))
  });
  assert_eq!(stage.text("Inner").0, light(PaletteColor::Primary));
}

/// The Markdown document's lists start from the same numbers: one source for the two.
#[cfg(feature = "markdown")]
#[test]
fn a_markdown_documents_lists_have_its_numbers() {
  let markdown = lurq::app::theme::ThemeMarkdown::default();
  let stage = staged(|| BulletedList::new(ROLES));
  let (first, second) = (text_of(&stage, ROLES[0]), text_of(&stage, ROLES[1]));
  assert_eq!(second.y - (first.y + first.height), markdown.list_item_spacing);
  let dot = dots(&stage).remove(0);
  assert_eq!(dot.width, markdown.bullet_size);
  assert_eq!(first.x - (dot.x + dot.width), markdown.bullet_gap);
  let stage = staged(|| BulletedList::new(["Install"]).marker(BulletedListMarker::Number));
  let (number, text) = (text_of(&stage, "1."), text_of(&stage, "Install"));
  assert_eq!(number.x + number.width, markdown.ordered_list_marker_width);
  assert_eq!(text.x - (number.x + number.width), markdown.list_marker_gap);
}

#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn to_tooling_it_is_a_list_of_items_called_by_its_label() {
  let mut stage = staged(|| {
    BulletedList::new([BulletedListItem::new("Semantic color roles").id("item")])
      .label("What a theme has")
      .id("list")
  });
  let mut said = |id: &str, key: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(key))
  };
  assert_eq!(said("list", "a11y_role").as_deref(), Some("list"));
  assert_eq!(said("list", "a11y_name").as_deref(), Some("What a theme has"));
  assert_eq!(said("item", "a11y_role").as_deref(), Some("listitem"));
}
