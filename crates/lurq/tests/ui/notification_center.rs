//! A notification center: the board's panel, its header's controls and what each tells, the rows and the lines
//! between them, the footer, the empty panel, a list that scrolls, the bell that opens it, the keys, where the
//! focus goes, a narrow room, what it is to tooling, and a theme's rule and renderer.

use std::sync::{
  Arc, Mutex,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{
    App,
    ctx::Ctx,
    theme::{PaletteColor, ShadowStyle, Theme, ThemePalette},
  },
  components::{Column, Row},
  core::{ElementRef, Signal},
  layout::quad::{Quad, QuadContent},
  node::{BoxShadowValue, Element, color::Color},
  ui::{
    Avatar, Badge, Button, ButtonVariant, Case, Notification, NotificationCenter, NotificationCenterPart,
    NotificationCenterRenderer, NotificationCenterView, icons,
  },
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn glyph(name: &str) -> String {
  icons::ALL
    .iter()
    .find_map(|(known, glyph)| (*known == name).then(|| glyph.to_string()))
    .unwrap_or_else(|| panic!("lurq's font has no icon {name:?}"))
}

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

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

/// The lines of 1 in the border's color along the bottom of something as wide as the box `(x, _, width, _)`:
/// where each is.
fn lines(stage: &Stage, (x, _, width, _): (f32, f32, f32, f32)) -> Vec<f32> {
  let edge = light(PaletteColor::Border);
  let mut tops: Vec<f32> = stage
    .quads()
    .into_iter()
    .filter(|quad| close(quad.x, x) && close(quad.width, width))
    .filter_map(|quad| {
      let border = quad.border.as_ref()?;
      let bottom = border.bottom.as_ref().filter(|_| border.top.is_none())?;
      (bottom.color == edge && bottom.width == 1.0).then_some(quad.y + quad.height - 1.0)
    })
    .collect();
  tops.sort_by(f32::total_cmp);
  tops.dedup_by(|a, b| close(*a, *b));
  tops
}

/// The board's three notifications: two that were not read and one that was.
fn board_items() -> Vec<Notification> {
  vec![
    Notification::new("Ivy Chen requested a review")
      .description("feat/node-blur")
      .media(Avatar::new("Ivy Chen"))
      .time("2m")
      .unread(true)
      .id("ivy"),
    Notification::new("Build passed")
      .description("412 tests in 38 seconds")
      .media(Avatar::new("C I"))
      .time("1h")
      .unread(true)
      .id("build"),
    Notification::new("Mark published 0.60")
      .description("Release notes are live")
      .media(Avatar::new("Mark Rowe"))
      .time("1d")
      .id("mark"),
  ]
}

/// The panel where it is declared, as `make` makes it.
fn inline(make: impl Fn(NotificationCenter) -> NotificationCenter + Send + Sync + 'static) -> Stage {
  Stage::first_frame(420.0, 600.0, move |_| {
    let center = NotificationCenter::new(Signal::new(true), ElementRef::new())
      .inline(true)
      .id("center");
    Column::new().size(420.0, 600.0).padding(8.0).child(make(center)).into()
  })
}

#[test]
fn a_notification_center_is_the_boards_panel() {
  let mut stage = inline(|center| center.items(board_items()).on_mark_all_read(|| ()).on_view_all(|| ()));
  stage.pass();
  let (x, y, width, height) = stage.bounds("center");
  assert_eq!(
    (width, height),
    (360.0, 256.0),
    "the header of 48, three rows of 56, the footer of 40"
  );
  assert_eq!(stage.fill_color("center"), Some(light(PaletteColor::Popover)));
  let edge = stage
    .border("center")
    .and_then(|quad| quad.border)
    .and_then(|b| b.top)
    .unwrap();
  assert_eq!((edge.width, edge.color), (1.0, light(PaletteColor::Border)));
  assert_eq!(
    stage
      .fill("center")
      .and_then(|quad| quad.border_radius)
      .unwrap()
      .top_left,
    12.0
  );
  let rest = Theme::new()
    .resolve::<NotificationCenter>(&Case::new(), None)
    .part(NotificationCenterPart::Panel)
    .rest();
  assert!(matches!(rest.shadow, Some(BoxShadowValue::Theme(ShadowStyle::Lg))));

  // The header: the title at 15 16 from the start, "Mark all read" at 13 in the info color 16 from the end.
  assert_eq!(stage.text("Notifications"), (light(PaletteColor::Foreground), 15.0));
  let title = stage.text_quad("Notifications").unwrap();
  assert!(close(title.x - x, 16.0));
  assert!(
    close(title.y + title.height / 2.0, y + 24.0),
    "in the middle of the header's 48"
  );
  assert_eq!(stage.text("Mark all read"), (light(PaletteColor::Info), 13.0));
  let mark = stage.text_quad("Mark all read").unwrap();
  assert!(close(x + width - (mark.x + mark.width), 16.0));

  // The rows, each 56, with a line under the header, between two rows and over the footer.
  for (id, top) in [("ivy", 48.0), ("build", 104.0), ("mark", 160.0)] {
    let (rx, ry, rw, rh) = stage.bounds(id);
    assert!(close(rx, x) && close(rw, width), "{id} is as wide as the panel");
    assert!(close(ry - y, top), "{id} at {}", ry - y);
    assert_eq!(rh, 56.0);
  }
  let tops: Vec<f32> = lines(&stage, (x, y, width, height)).iter().map(|top| top - y).collect();
  assert_eq!(tops.len(), 4, "{tops:?}");
  for (top, at) in tops.iter().zip([47.0, 103.0, 159.0, 215.0]) {
    assert!(close(*top, at), "{tops:?}");
  }

  // The footer of 40 with "View all" at 13 in its middle.
  let (fx, fy, fw, fh) = stage.bounds("center-view-all");
  assert_eq!((fw, fh), (360.0, 40.0));
  assert!(close(fx, x) && close(fy + fh, y + height));
  assert_eq!(stage.text("View all"), (light(PaletteColor::Foreground), 13.0));
  let words = stage.text_quad("View all").unwrap();
  assert!(close(words.x + words.width / 2.0, x + width / 2.0));
}

#[test]
fn the_first_frame_is_the_settled_one() {
  let mut stage = inline(|center| center.items(board_items()).on_view_all(|| ()));
  let first = stage.picture();
  stage.frame();
  assert_eq!(first, stage.picture());
}

#[test]
fn the_header_and_the_footer_tell_what_a_person_did() {
  let (marked, mark) = counter();
  let (cleared, clear) = counter();
  let (viewed, view) = counter();
  let mut stage = {
    let (mark, clear, view) = (mark.clone(), clear.clone(), view.clone());
    inline(move |center| {
      center
        .items(board_items())
        .on_mark_all_read(mark.clone())
        .on_mark_all_read(mark.clone())
        .on_clear(clear.clone())
        .on_view_all(view.clone())
        .clear_label("Clear")
    })
  };
  stage.pass();
  // Both controls of the header, 12 apart, at its end.
  let (mark_all, clear_all) = (
    stage.text_quad("Mark all read").unwrap(),
    stage.text_quad("Clear").unwrap(),
  );
  assert!(close(clear_all.x - (mark_all.x + mark_all.width), 12.0));
  stage.click("center-mark-all-read");
  assert_eq!(marked.load(Ordering::SeqCst), 2, "every handler, once each");
  stage.click("center-clear");
  assert_eq!(cleared.load(Ordering::SeqCst), 1);
  stage.click("center-view-all");
  assert_eq!(viewed.load(Ordering::SeqCst), 1);

  // The footer under the pointer is in the highlight; from the keyboard everything is reached and pressed.
  stage.hover("center-view-all");
  assert_eq!(
    stage.fill_color("center-view-all"),
    Some(light(PaletteColor::Highlight))
  );
}

#[test]
fn nothing_to_mark_and_nothing_to_clear_take_no_press() {
  let (marked, mark) = counter();
  let (cleared, clear) = counter();
  let mut stage = {
    let (mark, clear) = (mark.clone(), clear.clone());
    inline(move |center| {
      center
        .item(Notification::new("Mark published 0.60").id("mark"))
        .on_mark_all_read(mark.clone())
        .on_clear(clear.clone())
    })
  };
  stage.pass();
  stage.click("center-mark-all-read");
  assert_eq!(marked.load(Ordering::SeqCst), 0, "nothing is unread");
  stage.click("center-clear");
  assert_eq!(cleared.load(Ordering::SeqCst), 1, "there is something to clear");

  let mut empty = {
    let clear = clear.clone();
    inline(move |center| center.on_clear(clear.clone()))
  };
  empty.pass();
  empty.click("center-clear");
  assert_eq!(cleared.load(Ordering::SeqCst), 1, "an empty list is not cleared");
}

#[test]
fn an_empty_center_says_so_and_takes_the_apps_own_empty_state() {
  let mut stage = inline(|center| center.empty_text("You are all caught up"));
  stage.pass();
  let (x, y, width, height) = stage.bounds("center");
  assert_eq!(width, 360.0);
  assert_eq!(
    stage.text("You are all caught up"),
    (light(PaletteColor::MutedForeground), 13.0)
  );
  let inbox = glyph(icons::INBOX);
  assert_eq!(stage.text(&inbox), (light(PaletteColor::MutedForeground), 24.0));
  let (icon, words) = (
    stage.text_quad(&inbox).unwrap(),
    stage.text_quad("You are all caught up").unwrap(),
  );
  assert!(close(icon.x + icon.width / 2.0, x + width / 2.0), "in the middle");
  assert!(close(words.x + words.width / 2.0, x + width / 2.0));
  assert!(close(icon.y - (y + 48.0), 32.0), "32 under the header");
  assert!(words.y > icon.y + icon.height && words.y + words.height < y + height);

  let mut own = inline(|center| center.empty(Row::new().id("own").size(100.0, 40.0)));
  own.pass();
  assert!(!own.paints("No notifications"));
  assert_eq!(own.bounds("own").3, 40.0);
}

#[test]
fn a_long_list_scrolls_under_the_header_and_over_the_footer() {
  let mut stage = inline(|center| {
    center
      .items((0..12).map(|index| {
        Notification::new(&format!("Build {index} passed"))
          .description("412 tests in 38 seconds")
          .id(format!("row-{index}"))
      }))
      .on_view_all(|| ())
  });
  stage.pass();
  let (_, y, _, height) = stage.bounds("center");
  assert_eq!(height, 48.0 + 360.0 + 40.0, "the list is 360 at most");
  let (_, fy, _, fh) = stage.bounds("center-view-all");
  assert!(close(fy + fh, y + height), "the footer stays at the bottom");
  assert!(stage.paints("Build 0 passed"));
  assert!(!stage.paints("Build 11 passed"), "the last is under the fold");

  // A center with less room lets the list give way, and one told its own height keeps to it.
  let mut short = inline(|center| {
    center
      .items((0..12).map(|index| Notification::new(&format!("Build {index} passed"))))
      .max_height(120.0)
  });
  short.pass();
  assert_eq!(short.bounds("center").3, 48.0 + 120.0);
}

/// A bar with the bell at its end, and the center `make` makes of the board's notifications.
fn with_bell(
  open: &Signal<bool>,
  make: impl Fn(NotificationCenter) -> NotificationCenter + Send + Sync + 'static,
) -> Stage {
  let (open, bell) = (open.clone(), ElementRef::new());
  Stage::first_frame(600.0, 500.0, move |_: &mut Ctx| -> Element {
    let center = NotificationCenter::new(open.clone(), bell.clone())
      .trigger(
        Badge::count(2).label("unread notifications").on(
          Button::new("Notifications")
            .icon(icons::BELL)
            .icon_only(true)
            .variant(ButtonVariant::Ghost)
            .id("bell"),
        ),
      )
      .items(board_items().into_iter().map(|item| item.on_click(|_| ())))
      .id("center");
    Column::new()
      .size(600.0, 500.0)
      .padding(8.0)
      .child(
        Row::new()
          .width(584.0)
          .overflow_visible()
          .child(Button::new("Before").id("before"))
          .child(Row::new().flex(1.0))
          .child(make(center)),
      )
      .into()
  })
}

#[test]
fn the_bell_opens_it_under_itself_and_escape_and_a_press_outside_close_it() {
  let open = Signal::new(false);
  let (dismissed, dismiss) = counter();
  let mut stage = {
    let dismiss = dismiss.clone();
    with_bell(&open, move |center| center.on_dismiss(dismiss.clone()))
  };
  stage.pass();
  assert!(!stage.paints("Notifications"), "closed until the bell is pressed");
  assert!(stage.paints("2"), "the bell's count");

  stage.click("bell");
  stage.pass();
  assert!(open.get());
  let (bx, by, bw, bh) = stage.bounds("bell");
  let (x, y, width, _) = stage.bounds("center");
  assert_eq!(width, 360.0);
  assert!(
    close(x + width, bx + bw),
    "its end at the bell's end: {} and {}",
    x + width,
    bx + bw
  );
  assert!(close(y - (by + bh), 4.0), "4 under the bell: {}", y - (by + bh));

  stage.key("Escape");
  stage.pass();
  assert!(!open.get(), "Escape closes it");
  assert_eq!(dismissed.load(Ordering::SeqCst), 1);

  stage.click("bell");
  stage.pass();
  assert!(open.get());
  stage.click("before");
  stage.pass();
  assert!(!open.get(), "a press outside closes it");
  assert_eq!(dismissed.load(Ordering::SeqCst), 2);

  // The app's own change tells nothing.
  open.set(true);
  stage.pass();
  open.set(false);
  stage.pass();
  assert_eq!(dismissed.load(Ordering::SeqCst), 2);
}

#[test]
fn the_focus_goes_into_it_the_arrows_move_between_its_rows_and_it_goes_back_to_the_bell() {
  let open = Signal::new(false);
  let (viewed, view) = counter();
  let mut stage = {
    let view = view.clone();
    with_bell(&open, move |center| {
      center.on_mark_all_read(|| ()).on_view_all(view.clone())
    })
  };
  stage.pass();
  stage.key("Tab");
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("bell"));
  stage.key("Enter");
  stage.pass();
  assert!(open.get(), "Enter on the bell opens it");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("center-mark-all-read"),
    "the focus goes to the first thing in it"
  );
  // Tab stops in the rows once; the arrows move between them and do not leave them.
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("ivy"));
  stage.key("ArrowDown");
  assert_eq!(focused(&stage).as_deref(), Some("build"));
  stage.key("ArrowDown");
  stage.key("ArrowDown");
  assert_eq!(focused(&stage).as_deref(), Some("mark"), "no further than the last");
  stage.key("ArrowUp");
  assert_eq!(focused(&stage).as_deref(), Some("build"));
  stage.key("Tab");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("center-view-all"),
    "one stop for the rows"
  );

  // "View all" closes the panel, then tells, and the focus is back on the bell.
  stage.key("Enter");
  stage.pass();
  assert!(!open.get());
  assert_eq!(viewed.load(Ordering::SeqCst), 1);
  assert_eq!(focused(&stage).as_deref(), Some("bell"));
}

#[test]
fn every_notification_is_a_row_whatever_its_own_variant() {
  let mut stage = inline(|center| {
    center.item(
      Notification::new("Review requested")
        .description("Ivy asked you to review feat/node-blur.")
        .on_close(|| ())
        .id("card"),
    )
  });
  stage.pass();
  let (_, _, width, _) = stage.bounds("card");
  assert_eq!(width, 360.0, "as wide as the panel");
  assert_eq!(stage.fill_color("card"), None, "no card in the list");
  assert!(
    stage.paints(&glyph(icons::CLOSE)),
    "what dismisses it is the row's control"
  );
  assert!(!stage.paints("Dismiss"));
}

#[test]
fn it_is_no_wider_than_a_narrow_room_and_a_long_title_is_cut() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .size(400.0, 300.0)
      .child(
        Row::new().id("room").width(200.0).child(
          NotificationCenter::new(Signal::new(true), ElementRef::new())
            .inline(true)
            .title("Notifications of the whole workspace")
            .items(board_items())
            .on_mark_all_read(|| ())
            .on_view_all(|| ())
            .id("center"),
        ),
      )
      .into()
  });
  let (room, _, room_width, _) = stage.bounds("room");
  let (x, _, width, _) = stage.bounds("center");
  assert!(
    x >= room - 0.5 && x + width <= room + room_width + 0.5,
    "{width} wide in {room_width}"
  );
  for quad in stage.quads() {
    if let QuadContent::Text { text, .. } = &quad.content {
      assert!(
        quad.x + quad.width <= room + room_width + 0.5,
        "{text:?} runs past the room"
      );
    }
  }
  assert!(stage.paints("Mark all read"), "the control keeps its room");
  let cut: Option<Quad> = stage.cut_text_quad("Notifications of the whole workspace");
  assert!(
    cut.is_some() && !stage.paints("Notifications of the whole workspace"),
    "the title ends in an ellipsis"
  );
}

#[test]
fn a_themes_rule_and_renderer_change_it() {
  let app = App::new();
  let seen = Arc::new(Mutex::new(Vec::new()));
  let renderer: NotificationCenterRenderer = {
    let seen = seen.clone();
    Arc::new(move |view: NotificationCenterView| {
      seen
        .lock()
        .unwrap()
        .push((view.title.to_string(), view.unread, view.items.len()));
      view.render()
    })
  };
  app.theme().set_renderer::<NotificationCenter>(renderer);
  let mut stage = Stage::in_app(app, |_| {
    Column::new()
      .size(420.0, 400.0)
      .child(
        NotificationCenter::new(Signal::new(true), ElementRef::new())
          .inline(true)
          .title("Inbox")
          .items(board_items())
          .width(300.0)
          .style(NotificationCenterPart::Panel, |s| s.background(PaletteColor::Secondary))
          .id("center"),
      )
      .into()
  });
  assert_eq!(stage.fill_color("center"), Some(light(PaletteColor::Secondary)));
  assert_eq!(stage.bounds("center").2, 300.0);
  assert_eq!(seen.lock().unwrap().last().cloned(), Some(("Inbox".to_owned(), 2, 3)));
}

#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn what_a_notification_center_is_to_tooling() {
  let open = Signal::new(false);
  let mut stage = with_bell(&open, |center| center.label("Inbox"));
  stage.pass();
  let said = |stage: &mut Stage, id: &str, what: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(what))
  };
  stage.click("bell");
  stage.pass();
  assert_eq!(said(&mut stage, "center", "a11y_role").as_deref(), Some("dialog"));
  assert_eq!(said(&mut stage, "center", "a11y_name").as_deref(), Some("Inbox"));
  assert_eq!(said(&mut stage, "ivy", "a11y_role").as_deref(), Some("button"));
  assert_eq!(
    said(&mut stage, "ivy", "a11y_name").as_deref(),
    Some("Ivy Chen requested a review")
  );
}
