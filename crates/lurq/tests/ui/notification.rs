//! A notification: the board's card, the row of a list, what is unread, when, its buttons and what dismisses it,
//! a press of the whole from the pointer and the keyboard, a text longer than its room, the density of a row, what
//! it is to tooling, and a theme's rule and renderer.

use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{
    App,
    events::MouseButton,
    theme::{Density, PaletteColor, ShadowStyle, Theme, ThemePalette},
  },
  components::{Column, Row},
  layout::{
    quad::{Quad, QuadContent},
    text_style::FontWeight,
  },
  node::{BoxShadowValue, color::Color},
  ui::{
    Avatar, Case, Notification, NotificationPart, NotificationRenderer, NotificationVariant, NotificationView, icons,
  },
};

use super::stage::Stage;

const TITLE: &str = "Review requested";
const SAYS: &str = "Ivy asked you to review feat/node-blur.";

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// The character lurq's font has for an icon.
fn glyph(name: &str) -> String {
  icons::ALL
    .iter()
    .find_map(|(known, glyph)| (*known == name).then(|| glyph.to_string()))
    .unwrap_or_else(|| panic!("lurq's font has no icon {name:?}"))
}

/// Counts what is pressed.
fn counter() -> (Arc<AtomicUsize>, impl Fn() + Send + Sync + Clone + 'static) {
  let count = Arc::new(AtomicUsize::new(0));
  let add = count.clone();
  (count, move || {
    add.fetch_add(1, Ordering::SeqCst);
  })
}

fn close(a: f32, b: f32) -> bool {
  (a - b).abs() < 0.75
}

/// The weight the text `content` is painted in.
fn weight(stage: &Stage, content: &str) -> FontWeight {
  stage
    .quads()
    .iter()
    .find_map(|quad| match &quad.content {
      QuadContent::Text { text, style, .. } if text == content => Some(style.weight),
      _ => None,
    })
    .unwrap_or_else(|| panic!("no text {content:?} was painted"))
}

/// A filled dot `side` across in `color`.
fn dot(stage: &Stage, side: f32, color: Color) -> Option<Quad> {
  stage.quads().into_iter().find(|quad| {
    matches!(&quad.content, QuadContent::Rect { color: fill, .. } if *fill == color)
      && close(quad.width, side)
      && close(quad.height, side)
  })
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

/// The board's card.
fn review() -> Notification {
  Notification::new(TITLE).description(SAYS).id("it")
}

/// The board's first row.
fn ivy() -> Notification {
  Notification::new("Ivy Chen requested a review")
    .description("feat/node-blur")
    .media(Avatar::new("Ivy Chen").id("avatar"))
    .time("2m")
    .variant(NotificationVariant::Row)
    .id("it")
}

#[test]
fn a_notification_is_the_boards_card() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .size(400.0, 300.0)
      .padding(8.0)
      .child(review().button("Review", || ()).on_close(|| ()))
      .into()
  });
  let (x, y, width, height) = stage.bounds("it");
  assert_eq!(width, 340.0);
  assert_eq!(stage.fill_color("it"), Some(light(PaletteColor::Popover)));
  let edge = stage
    .border("it")
    .and_then(|quad| quad.border)
    .and_then(|b| b.top)
    .unwrap();
  assert_eq!((edge.width, edge.color), (1.0, light(PaletteColor::Border)));
  assert_eq!(
    stage.fill("it").and_then(|quad| quad.border_radius).unwrap().top_left,
    8.0
  );

  // The bell of 16, 16 inside, in the foreground's color; the title of 14 12 after it over the muted description.
  let bell = glyph(icons::BELL);
  assert_eq!(stage.text(&bell), (light(PaletteColor::Foreground), 16.0));
  let icon = stage.text_quad(&bell).unwrap();
  assert!(close(icon.x - x, 16.0) && close(icon.y - y, 16.0));
  let title = stage.text_quad(TITLE).unwrap();
  assert!(close(title.x - x, 44.0), "16, the bell of 16, 12: {}", title.x - x);
  assert_eq!(stage.text(TITLE), (light(PaletteColor::Foreground), 14.0));
  assert_eq!(weight(&stage, TITLE), FontWeight::Medium);
  assert_eq!(stage.text(SAYS), (light(PaletteColor::MutedForeground), 13.0));
  let says = stage.text_quad(SAYS).unwrap();
  assert!(close(says.x, title.x) && says.y >= title.y + title.height);

  // The buttons 10 under the text, 28 high, 8 apart: the first the primary one, what dismisses it a ghost.
  let (rx, ry, rw, rh) = stage.bounds("it-action-0");
  let (dx, dy, _, dh) = stage.bounds("it-close");
  assert_eq!((rh, dh), (28.0, 28.0));
  assert!(close(rx, title.x), "under the text");
  assert!(
    close(ry - (says.y + says.height), 10.0),
    "{}",
    ry - (says.y + says.height)
  );
  assert!(close(dx - (rx + rw), 8.0) && close(dy, ry));
  assert_eq!(stage.fill_color("it-action-0"), Some(light(PaletteColor::Primary)));
  assert_eq!(stage.fill_color("it-close"), None, "a ghost");
  assert_eq!(stage.text("Review"), (light(PaletteColor::PrimaryForeground), 13.0));
  assert_eq!(stage.text("Dismiss").1, 13.0);
  assert!(close(y + height - (ry + rh), 16.0), "16 under the buttons");

  // The medium shadow, and nothing of what the board does not draw.
  let rest = Theme::new()
    .resolve::<Notification>(&Case::new().variant(NotificationVariant::Card), None)
    .part(NotificationPart::Root)
    .rest();
  assert!(matches!(rest.shadow, Some(BoxShadowValue::Theme(ShadowStyle::Md))));
  assert!(dot(&stage, 7.0, light(PaletteColor::Info)).is_none(), "it was read");
  assert!(!stage.paints(&glyph(icons::CLOSE)));
}

#[test]
fn a_card_that_was_not_read_has_its_time_and_its_dot_after_the_title() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .size(400.0, 300.0)
      .child(review().time("2m").unread(true))
      .into()
  });
  let (x, _, width, _) = stage.bounds("it");
  let title = stage.text_quad(TITLE).unwrap();
  assert_eq!(stage.text("2m"), (light(PaletteColor::MutedForeground), 11.0));
  assert_ne!(stage.font_of("2m"), stage.font_of(TITLE), "the time in the mono family");
  let time = stage.text_quad("2m").unwrap();
  let mark = dot(&stage, 7.0, light(PaletteColor::Info)).expect("the dot of what was not read");
  assert!(time.x > title.x + title.width, "after the title");
  assert!(
    close(mark.x - (time.x + time.width), 8.0),
    "{}",
    mark.x - (time.x + time.width)
  );
  assert!(
    close(x + width - (mark.x + mark.width), 16.0),
    "at the end of the card's room"
  );
  assert!(
    mark.y > title.y && mark.y + mark.height < title.y + title.height,
    "on the title's line"
  );
}

#[test]
fn a_row_is_the_notification_centers_item() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .size(400.0, 300.0)
      .child(
        Column::new().width(360.0).child(ivy().unread(true)).child(
          Notification::new("Mark published 0.60")
            .description("Release notes are live")
            .media(Avatar::new("Mark Rowe").id("read-avatar"))
            .time("1d")
            .variant(NotificationVariant::Row)
            .id("read"),
        ),
      )
      .into()
  });
  let (x, y, width, height) = stage.bounds("it");
  assert_eq!(
    (width, height),
    (360.0, 56.0),
    "as wide as its list, 12 over and under the avatar of 32"
  );
  assert_eq!(stage.fill_color("it"), None, "no box of its own");
  assert!(stage.border("it").is_none());

  // The dot of 7 in the info color 12 from the start, the avatar 10 after it, the text 10 after that.
  let mark = dot(&stage, 7.0, light(PaletteColor::Info)).expect("the dot");
  assert!(close(mark.x - x, 12.0));
  assert!(
    close(mark.y + mark.height / 2.0, y + height / 2.0),
    "in the middle of the row"
  );
  let (ax, ay, aw, ah) = stage.bounds("avatar");
  assert_eq!((aw, ah), (32.0, 32.0));
  assert!(close(ax - (mark.x + mark.width), 10.0) && close(ay - y, 12.0));
  let title = stage.text_quad("Ivy Chen requested a review").unwrap();
  assert!(close(title.x - (ax + aw), 10.0));
  assert_eq!(
    stage.text("Ivy Chen requested a review"),
    (light(PaletteColor::Foreground), 13.0)
  );
  assert_eq!(
    stage.text("feat/node-blur"),
    (light(PaletteColor::MutedForeground), 12.0)
  );
  // The time in mono 11, muted, 16 from the end, in the middle of the row.
  assert_eq!(stage.text("2m"), (light(PaletteColor::MutedForeground), 11.0));
  let time = stage.text_quad("2m").unwrap();
  assert!(close(x + width - (time.x + time.width), 16.0));
  assert!(close(time.y + time.height / 2.0, y + height / 2.0));

  // Unread is the heavier title; a row that was read keeps the dot's room, so the avatars line up.
  assert_eq!(weight(&stage, "Ivy Chen requested a review"), FontWeight::Medium);
  assert_eq!(weight(&stage, "Mark published 0.60"), FontWeight::Normal);
  assert!(close(stage.bounds("read-avatar").0, ax));
  let dots = stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Rect { color, .. } if *color == light(PaletteColor::Info)))
    .count();
  assert_eq!(dots, 1, "one dot, on the one that was not read");
}

/// The board's card, pressed as a whole, with a button and what dismisses it: what each was told, in order
/// (opened, reviewed, dismissed).
fn pressed_card() -> (Stage, impl Fn() -> (usize, usize, usize)) {
  let (opened, open) = counter();
  let (reviewed, review_it) = counter();
  let (closed, close_it) = counter();
  let stage = Stage::new(move |_| {
    let (open, review_it, close_it) = (open.clone(), review_it.clone(), close_it.clone());
    Column::new()
      .size(400.0, 300.0)
      .child(
        review()
          .button("Review", review_it)
          .on_close(close_it)
          .on_click(move |_| open()),
      )
      .into()
  });
  (stage, move || {
    (
      opened.load(Ordering::SeqCst),
      reviewed.load(Ordering::SeqCst),
      closed.load(Ordering::SeqCst),
    )
  })
}

#[test]
fn a_press_of_the_notification_opens_it_and_a_press_of_a_button_is_the_buttons_alone() {
  let (mut stage, counts) = pressed_card();
  // The title is the card's: a press of it opens.
  let title = stage.text_quad(TITLE).unwrap();
  let (tx, ty) = (title.x + 4.0, title.y + 4.0);
  stage.tree.mouse_move(tx, ty);
  stage.tree.mouse_down(tx, ty, MouseButton::Left);
  stage.tree.mouse_up(tx, ty, MouseButton::Left);
  stage.pass();
  assert_eq!(counts(), (1, 0, 0));
  stage.click("it-action-0");
  assert_eq!(counts(), (1, 1, 0), "the button's, and not the card's");
  stage.click("it-close");
  assert_eq!(counts(), (1, 1, 1), "what dismisses it does not open it");
}

#[test]
fn from_the_keyboard_the_card_and_then_its_buttons_are_stops_of_tab() {
  let (mut stage, counts) = pressed_card();
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("it"));
  assert!(stage.has_ring(light(PaletteColor::Ring)), "the ring from the keyboard");
  stage.key("Enter");
  assert_eq!(counts(), (1, 0, 0), "Enter opens");
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("it-action-0"));
  stage.key("Enter");
  assert_eq!(counts(), (1, 1, 0));
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("it-close"));
  stage.key("Enter");
  assert_eq!(counts(), (1, 1, 1));
}

#[test]
fn a_notification_that_is_not_pressed_takes_no_focus_and_answers_no_pointer() {
  let mut stage = Stage::new(|_| Column::new().size(400.0, 300.0).child(review()).into());
  stage.key("Tab");
  assert_eq!(focused(&stage), None, "nothing in it is a stop of Tab");
  let before = stage.picture();
  stage.hover("it");
  assert_eq!(stage.picture(), before, "nothing under the pointer");
}

#[test]
fn a_row_under_the_pointer_is_in_the_highlight_and_its_close_control_dismisses_it() {
  let (opened, open) = counter();
  let (closed, close_it) = counter();
  let mut stage = Stage::new(move |_| {
    let (open, close_it) = (open.clone(), close_it.clone());
    Column::new()
      .size(400.0, 300.0)
      .child(
        Column::new()
          .width(360.0)
          .child(ivy().on_click(move |_| open()).on_close(close_it).close_label("Remove")),
      )
      .into()
  });
  assert_eq!(stage.fill_color("it"), None);
  stage.hover("it");
  assert_eq!(
    stage.fill_color("it"),
    Some(light(PaletteColor::Highlight)),
    "the board's row under the pointer"
  );
  // The close control of 16 at the end, after the time, muted.
  let mark = glyph(icons::CLOSE);
  assert_eq!(stage.text(&mark).1, 16.0);
  let (x, _, width, _) = stage.bounds("it");
  let (cx, _, cw, _) = stage.bounds("it-close");
  let time = stage.text_quad("2m").unwrap();
  assert!(close(x + width - (cx + cw), 16.0));
  assert!(close(cx - (time.x + time.width), 10.0));
  stage.click("it-close");
  assert_eq!(
    (opened.load(Ordering::SeqCst), closed.load(Ordering::SeqCst)),
    (0, 1),
    "dismissed, not opened"
  );
  assert!(!stage.paints("Remove"), "a row's is its icon alone");
  #[cfg(any(feature = "mcp", feature = "devtools"))]
  assert_eq!(
    stage
      .tree
      .get_element_by_id_mut("it-close")
      .and_then(|element| element.description("a11y_name"))
      .as_deref(),
    Some("Remove")
  );
}

#[test]
fn every_handler_is_told_in_the_order_it_was_given() {
  let said = Arc::new(std::sync::Mutex::new(Vec::new()));
  let mut stage = {
    let said = said.clone();
    Stage::new(move |_| {
      let say = |what: &'static str| {
        let said = said.clone();
        move || said.lock().unwrap().push(what)
      };
      let (first, second) = (say("click 1"), say("click 2"));
      Column::new()
        .size(400.0, 300.0)
        .child(
          review()
            .on_click(move |_| first())
            .on_click(move |_| second())
            .on_close(say("close 1"))
            .on_close(say("close 2")),
        )
        .into()
    })
  };
  stage.click("it-close");
  let title = stage.text_quad(TITLE).unwrap();
  let (x, y) = (title.x + 4.0, title.y + 4.0);
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass();
  assert_eq!(*said.lock().unwrap(), ["close 1", "close 2", "click 1", "click 2"]);
}

#[test]
fn an_icon_of_its_own_none_and_media_stand_before_the_text() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .size(400.0, 300.0)
      .spacing(8.0)
      .child(Notification::new("Build passed").icon(icons::CHECK_CIRCLE).id("icon"))
      .child(Notification::new("No icon").icon(None).id("none"))
      .child(
        Notification::new("From Ivy")
          .media(Avatar::new("Ivy Chen").id("avatar"))
          .id("media"),
      )
      .into()
  });
  assert!(stage.paints(&glyph(icons::CHECK_CIRCLE)));
  assert!(
    !stage.paints(&glyph(icons::BELL)),
    "the bell only where nothing else is said"
  );
  let (x, ..) = stage.bounds("none");
  assert!(
    close(stage.text_quad("No icon").unwrap().x - x, 16.0),
    "the text at the card's start"
  );
  let (mx, ..) = stage.bounds("media");
  let (ax, _, aw, _) = stage.bounds("avatar");
  assert!(close(ax - mx, 16.0));
  assert!(close(stage.text_quad("From Ivy").unwrap().x - (ax + aw), 12.0));
}

#[test]
fn a_text_longer_than_its_room_wraps_and_the_time_keeps_its_room() {
  const LONG: &str = "The nightly build of the design system branch finished with warnings";
  let mut stage = Stage::new(|_| {
    Column::new()
      .size(400.0, 300.0)
      .child(
        Row::new().id("room").width(200.0).child(
          Notification::new(LONG)
            .description("Forty two warnings were reported by the linter in the gallery crate.")
            .time("12m")
            .unread(true)
            .button("Open the report", || ())
            .on_close(|| ())
            .id("it"),
        ),
      )
      .into()
  });
  let (room, _, room_width, _) = stage.bounds("room");
  let (x, _, width, height) = stage.bounds("it");
  assert!(
    x >= room - 0.5 && x + width <= room + room_width + 0.5,
    "{width} wide in {room_width}"
  );
  assert!(height > 150.0, "its text goes on to the next lines: {height}");
  for quad in stage.quads() {
    if let QuadContent::Text { text, .. } = &quad.content {
      assert!(
        quad.x + quad.width <= room + room_width + 0.5,
        "{text:?} runs past the room"
      );
    }
  }
  assert!(stage.paints("12m"), "the time is whole");
  // The buttons that do not fit side by side go on to the next line.
  let (_, first, ..) = stage.bounds("it-action-0");
  let (_, second, ..) = stage.bounds("it-close");
  assert!(second > first);
}

#[test]
fn a_row_follows_the_density() {
  for (density, high) in [
    (Density::Compact, 48.0),
    (Density::Default, 56.0),
    (Density::Comfortable, 64.0),
    (Density::Touch, 68.0),
  ] {
    let app = App::new();
    app.theme().set_density(density);
    let mut stage = Stage::in_app(app, |_| {
      Column::new()
        .size(400.0, 300.0)
        .child(Column::new().width(360.0).child(ivy()))
        .into()
    });
    assert_eq!(stage.bounds("it").3, high, "{density:?}");
  }
}

#[test]
fn a_themes_rule_and_renderer_change_it() {
  let app = App::new();
  let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
  let renderer: NotificationRenderer = {
    let seen = seen.clone();
    Arc::new(move |view: NotificationView| {
      seen.lock().unwrap().push((
        view.title.to_string(),
        view.unread,
        view.row,
        view.close_button,
        view.icon,
      ));
      view.render()
    })
  };
  app.theme().set_renderer::<Notification>(renderer);
  let mut stage = Stage::in_app(app, |_| {
    Column::new()
      .size(400.0, 300.0)
      .child(
        review()
          .unread(true)
          .on_close(|| ())
          .style(NotificationPart::Root, |s| s.background(PaletteColor::Secondary)),
      )
      .into()
  });
  assert_eq!(stage.fill_color("it"), Some(light(PaletteColor::Secondary)));
  assert_eq!(
    seen.lock().unwrap().last().cloned(),
    Some((
      TITLE.to_owned(),
      true,
      false,
      true,
      Some(lurq::ui::Name::new(icons::BELL))
    ))
  );
}

#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn what_a_notification_is_to_tooling() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .size(400.0, 300.0)
      .spacing(8.0)
      .child(review().unread(true).unread_label("New"))
      .child(
        Notification::new("Build passed")
          .label("CI")
          .on_click(|_| ())
          .id("pressed"),
      )
      .into()
  });
  let mut said = |id: &str, what: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(what))
  };
  assert_eq!(said("it", "a11y_role").as_deref(), Some("article"));
  assert_eq!(said("it", "a11y_name").as_deref(), Some(TITLE));
  assert_eq!(said("it", "a11y_description").as_deref(), Some(SAYS));
  assert_eq!(said("pressed", "a11y_role").as_deref(), Some("button"));
  assert_eq!(said("pressed", "a11y_name").as_deref(), Some("CI"));
}
