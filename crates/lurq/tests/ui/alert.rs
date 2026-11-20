//! An alert, a banner and a callout: the box of each as the board draws it,
//! the colors and the icon of each variant, a text that wraps in the room it
//! has, the close control from the pointer and the keyboard, a banner the app
//! takes away and one that sticks to the top of what scrolls, a callout with
//! the bar of a document's note, what each is to tooling, and a theme's rule
//! and renderer.

use std::{
  sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
  },
  time::Duration,
};

use lurq::{
  app::{
    App,
    events::{MouseButton, ScrollPhase},
    theme::{PaletteColor, ThemePalette},
  },
  components::{Column, Row, ScrollVertical},
  core::Signal,
  layout::quad::{Quad, QuadContent},
  node::color::Color,
  ui::{
    Alert, AlertPart, AlertRenderer, AlertVariant, AlertView, Banner, BannerPart, Button, ButtonSize, ButtonVariant,
    Callout, CalloutPart, CalloutRenderer, CalloutView, Link, Name, Recipe, icons,
  },
};

use super::stage::Stage;

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

/// The edge of the element with `id`: its width and its color.
fn edge(stage: &mut Stage, id: &str) -> Option<(f32, Color)> {
  stage
    .border(id)
    .and_then(|quad| quad.border)
    .and_then(|border| border.top)
    .map(|side| (side.width, side.color))
}

/// The line along the start of the element with `id`: its width and its color.
fn start_line(stage: &mut Stage, id: &str) -> Option<(f32, Color)> {
  let (x, y, width, height) = stage.bounds(id);
  stage.quads().into_iter().find_map(|quad| {
    let at = (quad.x, quad.y, quad.width, quad.height) == (x, y, width, height);
    let side = quad
      .border
      .and_then(|border| border.left.filter(|_| border.top.is_none()))?;
    at.then_some((side.width, side.color))
  })
}

/// The radius of the top left and the top right corners of the fill of the element with `id`.
fn corners(stage: &mut Stage, id: &str) -> (f32, f32) {
  let radius = stage
    .fill(id)
    .and_then(|quad| quad.border_radius)
    .expect("a round fill");
  (radius.top_left, radius.top_right)
}

/// Clicks the middle of a painted quad.
fn click_on(stage: &mut Stage, quad: &Quad) {
  let (x, y) = (quad.x + quad.width / 2.0, quad.y + quad.height / 2.0);
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass();
}

/// The lines of text painted inside the box `(x, y, width, height)`.
fn lines_in(stage: &Stage, (x, y, width, height): (f32, f32, f32, f32)) -> Vec<Quad> {
  stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Text { .. }))
    .filter(|quad| quad.x >= x - 0.5 && quad.x < x + width && quad.y >= y - 0.5 && quad.y < y + height)
    .collect()
}

// --- Alert ---

#[test]
fn an_alert_is_the_boards_box_with_its_icon_title_and_description() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .width(376.0)
      .padding(8.0)
      .child(
        Alert::new("Update available")
          .description("lurq 0.61 is ready to install.")
          .id("alert"),
      )
      .into()
  });
  let (x, y, width, _) = stage.bounds("alert");
  assert_eq!(width, 360.0, "as wide as the room it is given");

  // The card's color, an edge of 1 in the border's color, corners of `radius-lg`.
  assert_eq!(stage.fill_color("alert"), Some(light(PaletteColor::Card)));
  assert_eq!(edge(&mut stage, "alert"), Some((1.0, light(PaletteColor::Border))));
  assert_eq!(corners(&mut stage, "alert"), (8.0, 8.0));

  // The information sign of 16, 16 inside, in the foreground's color.
  let info = glyph(icons::INFO);
  assert_eq!(stage.text(&info), (light(PaletteColor::Foreground), 16.0));
  let icon = stage.text_quad(&info).unwrap();
  assert!((icon.x - x - 16.0).abs() < 0.5 && (icon.y - y - 16.0).abs() < 0.5);

  // The title of 14 12 after the icon, the muted description of 13 under it.
  let title = stage.text_quad("Update available").unwrap();
  assert!(
    (title.x - x - 44.0).abs() < 0.5,
    "16, the icon of 16, 12: {}",
    title.x - x
  );
  assert!((title.y - y - 16.0).abs() < 2.0);
  assert_eq!(stage.text("Update available"), (light(PaletteColor::Foreground), 14.0));
  assert_eq!(
    stage.text("lurq 0.61 is ready to install."),
    (light(PaletteColor::MutedForeground), 13.0)
  );
  let description = stage.text_quad("lurq 0.61 is ready to install.").unwrap();
  assert!((description.x - title.x).abs() < 0.5);
  assert!(description.y >= title.y + title.height, "under the title");

  // Nothing to close it with unless it is given a handler.
  assert!(!stage.paints(&glyph(icons::CLOSE)));
}

#[test]
fn each_severity_has_its_fill_its_edge_and_its_icon() {
  let mut stage = Stage::new(|_| {
    let mut column = Column::new().width(360.0).spacing(8.0);
    for (variant, title, id) in [
      (AlertVariant::Info, "New shortcut", "info"),
      (AlertVariant::Success, "Backup complete", "success"),
      (AlertVariant::Warning, "Low disk space", "warning"),
      (AlertVariant::Destructive, "Could not save", "destructive"),
    ] {
      column = column.child(
        Alert::new(title)
          .description(&format!("The {id} alert."))
          .variant(variant)
          .id(id),
      );
    }
    column.into()
  });
  use PaletteColor::*;
  for (id, fill, accent) in [
    ("info", InfoMuted, Info),
    ("success", SuccessMuted, Success),
    ("warning", WarningMuted, Warning),
  ] {
    assert_eq!(stage.fill_color(id), Some(light(fill)), "{id}");
    assert_eq!(edge(&mut stage, id), Some((1.0, light(accent))), "{id}");
    // The description in the foreground's color on a tinted fill.
    assert_eq!(stage.text(&format!("The {id} alert.")).0, light(Foreground), "{id}");
  }
  assert_eq!(stage.text(&glyph(icons::CHECK_CIRCLE)).0, light(Success));
  assert_eq!(stage.text(&glyph(icons::INFO)).0, light(Info));
  assert_eq!(stage.text(&glyph(icons::WARNING)).0, light(Warning));

  // What went wrong keeps the card's color, with an edge and a title of the destructive color and a muted
  // description.
  assert_eq!(stage.fill_color("destructive"), Some(light(Card)));
  assert_eq!(edge(&mut stage, "destructive"), Some((1.0, light(Destructive))));
  assert_eq!(stage.text("Could not save").0, light(Destructive));
  assert_eq!(stage.text("The destructive alert.").0, light(MutedForeground));
  let (_, y, ..) = stage.bounds("destructive");
  let sign = stage
    .quads()
    .into_iter()
    .find(|quad| {
      matches!(&quad.content, QuadContent::Text { text, .. } if *text == glyph(icons::WARNING)) && quad.y > y
    })
    .expect("the warning sign on the destructive alert");
  let QuadContent::Text { style, .. } = sign.content else {
    unreachable!()
  };
  assert_eq!(style.color, light(Destructive));
}

#[test]
fn an_icon_of_its_own_or_none() {
  let stage = Stage::new(|_| {
    Column::new()
      .width(360.0)
      .spacing(8.0)
      .child(Alert::new("Copied").icon(icons::COPY).id("copied"))
      .child(Alert::new("Plain").variant(AlertVariant::Info).icon(None).id("plain"))
      .into()
  });
  assert!(stage.paints(&glyph(icons::COPY)));
  assert!(!stage.paints(&glyph(icons::INFO)), "the variant's icon is not drawn");
  let mut stage = stage;
  let (x, ..) = stage.bounds("plain");
  let title = stage.text_quad("Plain").unwrap();
  assert!((title.x - x - 16.0).abs() < 0.5, "the title where the icon would be");
}

#[test]
fn a_long_description_wraps_beside_the_action_and_the_close_control() {
  const LONG: &str = "The settings file is read-only, so nothing you change here is kept after lurq closes.";
  let mut stage = Stage::new(|_| {
    Column::new()
      .width(360.0)
      .child(
        Alert::new("Could not save")
          .description(LONG)
          .variant(AlertVariant::Destructive)
          .action(
            Button::new("Retry")
              .variant(ButtonVariant::Outline)
              .size(ButtonSize::Sm)
              .id("retry"),
          )
          .on_close(|| ())
          .id("alert"),
      )
      .into()
  });
  let bounds = stage.bounds("alert");
  let (x, _, width, _) = bounds;
  let (retry_x, ..) = stage.bounds("retry");
  let mark = stage.text_quad(&glyph(icons::CLOSE)).unwrap();
  // The close control at the end, 16 from the edge, the action 12 before it.
  assert!((x + width - (mark.x + mark.width) - 16.0).abs() < 0.5);
  assert!(retry_x < mark.x);
  // The description goes on to more lines in the room the action leaves it, and none runs under the action. A
  // wrapped text is painted as one quad as high as its lines.
  let title = stage.text_quad("Could not save").unwrap();
  let description = stage.text_quad(LONG).unwrap();
  assert!(
    description.height > title.height * 2.0,
    "on more than one line: {} high",
    description.height
  );
  assert!((description.x - (x + 44.0)).abs() < 0.5);
  assert!(
    description.x + description.width <= retry_x - 11.5,
    "12 before the action: {} {retry_x}",
    description.x + description.width
  );
}

#[test]
fn a_body_of_the_apps_elements_stands_under_the_description() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .width(360.0)
      .child(
        Alert::new("Low disk space")
          .description("Less than 10% of the disk is free.")
          .variant(AlertVariant::Warning)
          .child(Row::new().size(120.0, 24.0).id("body"))
          .id("alert"),
      )
      .into()
  });
  let (x, ..) = stage.bounds("alert");
  let (body_x, body_y, ..) = stage.bounds("body");
  let description = stage.text_quad("Less than 10% of the disk is free.").unwrap();
  assert!((body_x - (x + 44.0)).abs() < 0.5, "under the text, after the icon");
  assert!(body_y >= description.y + description.height, "under the description");
}

fn small(label: &str, id: &str) -> Button {
  Button::new(label)
    .variant(ButtonVariant::Outline)
    .size(ButtonSize::Sm)
    .id(id.to_owned())
}

#[test]
fn several_actions_stand_at_its_end_in_the_order_given_before_the_close_control() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .width(480.0)
      .child(
        Alert::new("Could not save")
          .description("The settings file is read-only.")
          .variant(AlertVariant::Destructive)
          .action(small("Details", "details"))
          .actions([small("Save as", "save-as"), small("Retry", "retry")])
          .on_close(|| ())
          .id("alert"),
      )
      .into()
  });
  let (details_x, details_y, details_width, details_height) = stage.bounds("details");
  let (save_x, save_y, save_width, _) = stage.bounds("save-as");
  let (retry_x, retry_y, retry_width, _) = stage.bounds("retry");
  // On one line, the least first and the main one last, 8 apart.
  assert!((save_x - (details_x + details_width) - 8.0).abs() < 0.5);
  assert!((retry_x - (save_x + save_width) - 8.0).abs() < 0.5);
  assert!((details_y - save_y).abs() < 0.5 && (save_y - retry_y).abs() < 0.5);
  // The close control after them, and the words before them.
  let mark = stage.text_quad(&glyph(icons::CLOSE)).unwrap();
  assert!(retry_x + retry_width <= mark.x);
  let title = stage.text_quad("Could not save").unwrap();
  assert!(title.x + title.width <= details_x);
  // Centered on the actions' height, and held by the part `action`, named after the alert.
  let (row_x, row_y, _, row_height) = stage.bounds("alert-action");
  assert!((row_x - details_x).abs() < 0.5);
  assert!((row_y + row_height / 2.0 - (details_y + details_height / 2.0)).abs() < 0.5);
}

#[test]
fn a_stacked_alert_has_its_actions_under_its_words_at_their_start() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .width(300.0)
      .child(
        Alert::new("Working offline")
          .description("Changes are kept on this computer.")
          .variant(AlertVariant::Warning)
          .stacked(true)
          .child(Row::new().size(120.0, 24.0).id("body"))
          .actions([
            small("Details", "details"),
            small("Reconnect now", "reconnect"),
            small("Work offline", "offline"),
          ])
          .on_close(|| ())
          .id("alert"),
      )
      .into()
  });
  let (x, y, width, height) = stage.bounds("alert");
  let title = stage.text_quad("Working offline").unwrap();
  let (_, body_y, _, body_height) = stage.bounds("body");
  let (details_x, details_y, details_width, _) = stage.bounds("details");
  let (reconnect_x, reconnect_y, ..) = stage.bounds("reconnect");
  let (offline_x, offline_y, _, offline_height) = stage.bounds("offline");
  // At the start of the words, 8 under the body.
  assert!((details_x - title.x).abs() < 0.5, "at the words' start");
  assert!(
    (details_y - (body_y + body_height) - 8.0).abs() < 0.5,
    "8 under the body"
  );
  assert!((reconnect_x - (details_x + details_width) - 8.0).abs() < 0.5);
  assert!((reconnect_y - details_y).abs() < 0.5);
  // What one line of the words' width does not hold goes on to the next, at the start again.
  assert!((offline_x - details_x).abs() < 0.5, "wrapped to the start: {offline_x}");
  assert!(offline_y > reconnect_y);
  // The title stays at the top, the close control at the end, and the box holds the actions.
  assert!((title.y - y - 16.0).abs() < 2.0, "not centered on the actions");
  let mark = stage.text_quad(&glyph(icons::CLOSE)).unwrap();
  assert!((x + width - (mark.x + mark.width) - 16.0).abs() < 0.5);
  assert!(mark.y < details_y, "the close control at the top");
  assert!(offline_y + offline_height <= y + height - 15.5);
}

#[test]
fn the_close_control_is_pressed_by_the_pointer_and_the_keyboard() {
  let (closed, close) = counter();
  let mut stage = Stage::new(move |_| {
    let close = close.clone();
    Column::new()
      .width(360.0)
      .child(Alert::new("New shortcut").on_close(move || close()))
      .into()
  });
  // From the keyboard: it is a stop of Tab, with the ring, and Enter presses it.
  stage.key("Tab");
  assert!(stage.has_ring(light(PaletteColor::Ring)));
  stage.key("Enter");
  assert_eq!(closed.load(Ordering::SeqCst), 1);
  // From the pointer, in the muted color at rest.
  let close = glyph(icons::CLOSE);
  assert_eq!(stage.text(&close), (light(PaletteColor::MutedForeground), 16.0));
  let mark = stage.text_quad(&close).unwrap();
  click_on(&mut stage, &mark);
  assert_eq!(closed.load(Ordering::SeqCst), 2);
}

#[test]
fn a_warning_and_what_went_wrong_are_alerts_to_tooling_and_the_rest_a_status() {
  let roles = Arc::new(Mutex::new(Vec::new()));
  let seen = roles.clone();
  let app = App::new();
  let renderer: AlertRenderer = Arc::new(move |view: AlertView| {
    seen
      .lock()
      .unwrap()
      .push((view.title.to_string(), view.role.to_string()));
    view.render()
  });
  app.theme().set_renderer::<Alert>(renderer);
  let _stage = Stage::in_app(app, |_| {
    let mut column = Column::new().width(360.0);
    for variant in AlertVariant::all() {
      column = column.child(Alert::new(variant.as_str()).variant(*variant));
    }
    column
      .child(Alert::new("urgent news").variant(AlertVariant::Info).role("alert"))
      .into()
  });
  let roles = roles.lock().unwrap();
  let role_of = |title: &str| {
    roles
      .iter()
      .rev()
      .find(|(said, _)| said == title)
      .map(|(_, role)| role.clone())
      .unwrap()
  };
  assert_eq!(role_of("neutral"), "status");
  assert_eq!(role_of("info"), "status");
  assert_eq!(role_of("success"), "status");
  assert_eq!(role_of("warning"), "alert");
  assert_eq!(role_of("destructive"), "alert");
  assert_eq!(role_of("urgent news"), "alert", "the app's role over the variant's");
}

#[test]
fn a_rule_and_a_renderer_of_a_theme_change_an_alert() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .width(360.0)
      .child(
        Alert::new("Ruled")
          .id("alert")
          .style(AlertPart::Root, |s| s.background(PaletteColor::Secondary))
          .style(AlertPart::Title, |s| s.font_size(20.0)),
      )
      .into()
  });
  assert_eq!(stage.fill_color("alert"), Some(light(PaletteColor::Secondary)));
  assert_eq!(stage.text("Ruled").1, 20.0);

  // A renderer that draws the text alone: other nodes, the same recipe.
  let app = App::new();
  let renderer: AlertRenderer = Arc::new(|mut view: AlertView| {
    let text = view.text();
    Row::new()
      .id("alert")
      .part(&view.part(AlertPart::Root))
      .child(text)
      .into()
  });
  app.theme().set_renderer::<Alert>(renderer);
  let mut stage = Stage::in_app(app, |_| {
    Column::new()
      .width(360.0)
      .child(Alert::new("Text alone").variant(AlertVariant::Info))
      .into()
  });
  assert!(stage.paints("Text alone"));
  assert!(!stage.paints(&glyph(icons::INFO)));
  assert_eq!(stage.fill_color("alert"), Some(light(PaletteColor::InfoMuted)));
}

// --- Banner ---

#[test]
fn a_banner_is_the_boards_bar_with_its_icon_text_link_and_close_control() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .width(400.0)
      .child(
        Banner::new("A new version is out.")
          .variant(AlertVariant::Info)
          .link(Link::new("Update now"))
          .on_close(|| ())
          .id("banner"),
      )
      .into()
  });
  let (x, y, width, height) = stage.bounds("banner");
  assert_eq!(width, 400.0, "across the room it is given");
  assert_eq!(stage.fill_color("banner"), Some(light(PaletteColor::InfoMuted)));
  assert_eq!(corners(&mut stage, "banner"), (6.0, 6.0));
  assert_eq!(edge(&mut stage, "banner"), None, "no edge");

  // The megaphone of 16 in the color of information, 16 from the start, in the middle of the bar.
  let megaphone = glyph(icons::MEGAPHONE);
  assert_eq!(stage.text(&megaphone), (light(PaletteColor::Info), 16.0));
  let icon = stage.text_quad(&megaphone).unwrap();
  assert!((icon.x - x - 16.0).abs() < 0.5);
  assert!((icon.y + icon.height / 2.0 - (y + height / 2.0)).abs() < 1.0);

  // One line of 13 in the foreground's color 12 after the icon, padded 10 over and under it.
  assert_eq!(
    stage.text("A new version is out."),
    (light(PaletteColor::Foreground), 13.0)
  );
  let text = stage.text_quad("A new version is out.").unwrap();
  assert!((text.x - x - 44.0).abs() < 0.5);
  assert!(height < 44.0, "one line, padded 10: {height}");

  // The link in the color of information, 13 and semibold; the close control at the end in the foreground's.
  assert_eq!(stage.text("Update now"), (light(PaletteColor::Info), 13.0));
  let close = glyph(icons::CLOSE);
  assert_eq!(stage.text(&close), (light(PaletteColor::Foreground), 16.0));
  let mark = stage.text_quad(&close).unwrap();
  let link = stage.text_quad("Update now").unwrap();
  assert!((x + width - (mark.x + mark.width) - 16.0).abs() < 0.5);
  assert!(link.x + link.width <= mark.x - 11.5, "12 before the close control");
}

#[test]
fn each_variant_of_a_banner_has_its_fill_and_its_icon() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .width(400.0)
      .spacing(8.0)
      .child(Banner::new("Neutral").id("neutral"))
      .child(Banner::new("Warning").variant(AlertVariant::Warning).id("warning"))
      .child(Banner::new("Done").variant(AlertVariant::Success).id("success"))
      .into()
  });
  assert_eq!(stage.fill_color("neutral"), Some(light(PaletteColor::Muted)));
  assert_eq!(stage.text(&glyph(icons::MEGAPHONE)).0, light(PaletteColor::Foreground));
  assert_eq!(stage.fill_color("warning"), Some(light(PaletteColor::WarningMuted)));
  assert_eq!(stage.text(&glyph(icons::WARNING)).0, light(PaletteColor::Warning));
  assert_eq!(stage.fill_color("success"), Some(light(PaletteColor::SuccessMuted)));
  assert_eq!(stage.text(&glyph(icons::CHECK_CIRCLE)).0, light(PaletteColor::Success));
}

#[test]
fn a_long_banner_goes_on_to_more_lines_before_its_close_control() {
  const LONG: &str = "lurq 0.61 is available with faster text shaping, a new icon font and fewer redraws.";
  let mut stage = Stage::new(|_| {
    Column::new()
      .width(320.0)
      .spacing(8.0)
      .child(Banner::new("Short").on_close(|| ()).id("short"))
      .child(Banner::new(LONG).on_close(|| ()).id("long"))
      .into()
  });
  let (_, _, _, short) = stage.bounds("short");
  let bounds = stage.bounds("long");
  assert!(bounds.3 > short + 10.0, "on more lines: {} {short}", bounds.3);
  let marks: Vec<Quad> = stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if *text == glyph(icons::CLOSE)))
    .collect();
  let mark = marks.last().unwrap();
  for line in lines_in(&stage, bounds)
    .into_iter()
    .filter(|quad| quad.x < mark.x - 1.0)
  {
    assert!(line.x + line.width <= mark.x + 0.5, "before the close control");
  }
}

#[test]
fn a_centered_banner_has_its_text_in_its_middle() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .width(400.0)
      .child(
        Banner::new("No connection. Changes will sync later.")
          .variant(AlertVariant::Warning)
          .icon(None)
          .centered(true)
          .id("banner"),
      )
      .into()
  });
  let (x, _, width, _) = stage.bounds("banner");
  let text = stage.text_quad("No connection. Changes will sync later.").unwrap();
  assert!(
    (text.x + text.width / 2.0 - (x + width / 2.0)).abs() < 1.0,
    "in the middle: {} {}",
    text.x + text.width / 2.0,
    x + width / 2.0
  );
}

#[test]
fn the_app_takes_a_banner_away_from_its_close_control_and_the_page_moves_up() {
  let open = Signal::new(true);
  let shown = open.clone();
  let mut stage = Stage::new(move |_| {
    let close = shown.clone();
    Column::new()
      .width(400.0)
      .child(
        Banner::new("Trial ends in 3 days.")
          .link(Link::new("Upgrade").id("upgrade"))
          .open(shown.get())
          .on_close(move || close.set(false)),
      )
      .child(Row::new().size(400.0, 40.0).id("page"))
      .into()
  });
  let (_, below, ..) = stage.bounds("page");
  assert!(below > 30.0, "under the banner");

  // Tab goes to its link, then to its close control: Enter presses it, and the app takes it away.
  stage.key("Tab");
  stage.key("Tab");
  stage.key("Enter");
  assert!(!open.get_untracked());
  stage.pass_after(Duration::from_secs(2));
  stage.pass();
  assert!(!stage.paints("Trial ends in 3 days."), "gone once it has faded out");
  let (_, top, ..) = stage.bounds("page");
  assert!(top < 1.0, "the page moves up: {top}");

  // Shown again, it is back over the page.
  open.set(true);
  stage.pass();
  assert!(stage.paints("Trial ends in 3 days."));
}

#[test]
fn a_banner_shown_again_while_it_fades_out_is_there_once() {
  let open = Signal::new(true);
  let shown = open.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .width(400.0)
      .child(Banner::new("Trial ends in 3 days.").open(shown.get()))
      .child(Row::new().size(400.0, 40.0).id("page"))
      .into()
  });
  let (_, below, ..) = stage.bounds("page");
  let count = |stage: &Stage| {
    stage
      .quads()
      .iter()
      .filter(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text == "Trial ends in 3 days."))
      .count()
  };

  // Taken away, it fades out where it was.
  open.set(false);
  stage.pass_after(Duration::ZERO);
  stage.pass_after(Duration::from_millis(40));
  assert_eq!(count(&stage), 1, "leaving, it is still painted");

  // Shown again on the way: the one leaving goes and the one shown is there alone, the page under one banner.
  open.set(true);
  stage.pass_after(Duration::from_millis(40));
  assert_eq!(count(&stage), 1, "once");
  assert!(
    (stage.bounds("page").1 - below).abs() < 0.5,
    "the page under one banner"
  );
  stage.pass_after(Duration::from_secs(2));
  stage.pass();
  assert_eq!(count(&stage), 1);
  assert!((stage.bounds("page").1 - below).abs() < 0.5);
}

#[test]
fn a_banner_fades_in_and_out_by_its_recipe() {
  let theme = App::new().theme().clone();
  let panel = theme
    .resolve::<Banner>(&lurq::ui::Case::new(), None)
    .part(BannerPart::Panel)
    .rest();
  assert_eq!(panel.enter.map(|enter| enter.keyframes), Some(lurq::ui::FADE_IN));
  assert_eq!(panel.exit.map(|exit| exit.keyframes), Some(lurq::ui::FADE_OUT));
}

#[test]
fn a_sticky_banner_stays_at_the_top_of_what_scrolls() {
  let mut stage = Stage::new(|_| {
    ScrollVertical::new(
      Column::new()
        .width(300.0)
        .child(
          Banner::new("Read only")
            .variant(AlertVariant::Info)
            .sticky(true)
            .id("banner"),
        )
        .child(Row::new().size(300.0, 600.0)),
    )
    .size(300.0, 200.0)
    .into()
  });
  let (_, top, ..) = stage.bounds("banner");
  stage.tree.scroll(150.0, 100.0, 0.0, -120.0, ScrollPhase::Scroll);
  stage.pass();
  let fill = light(PaletteColor::InfoMuted);
  let bar = stage
    .quads()
    .into_iter()
    .find(|quad| matches!(quad.content, QuadContent::Rect { color, .. } if color == fill))
    .expect("the banner is painted");
  assert!((bar.y - top).abs() < 1.0, "at the top still: {} {top}", bar.y);
}

// --- Callout ---

#[test]
fn a_callout_is_the_boards_box_with_its_icon_and_text() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .width(380.0)
      .child(
        Callout::new("Theme changes rerender only the components that read the theme.")
          .variant(AlertVariant::Info)
          .id("callout"),
      )
      .into()
  });
  let bounds = stage.bounds("callout");
  let (x, y, width, _) = bounds;
  assert_eq!(width, 380.0);
  assert_eq!(stage.fill_color("callout"), Some(light(PaletteColor::InfoMuted)));
  assert_eq!(corners(&mut stage, "callout"), (6.0, 6.0));
  assert_eq!(start_line(&mut stage, "callout"), None, "no bar");

  // The icon of 16 in the color of information, 14 from the start and 12 from the top; the text 10 after it,
  // 13 in the foreground's color, on two lines.
  let info = glyph(icons::INFO);
  assert_eq!(stage.text(&info), (light(PaletteColor::Info), 16.0));
  let icon = stage.text_quad(&info).unwrap();
  assert!((icon.x - x - 14.0).abs() < 0.5 && (icon.y - y - 12.0).abs() < 0.5);
  let lines: Vec<Quad> = lines_in(&stage, bounds)
    .into_iter()
    .filter(|quad| quad.x > icon.x + 1.0)
    .collect();
  let first = lines.iter().min_by(|a, b| a.y.total_cmp(&b.y)).unwrap();
  assert!(
    (first.x - x - 40.0).abs() < 0.5,
    "14, the icon of 16, 10: {}",
    first.x - x
  );
  let QuadContent::Text { style, .. } = &first.content else {
    unreachable!()
  };
  assert_eq!((style.color, style.font_size), (light(PaletteColor::Foreground), 13.0));
}

#[test]
fn a_callout_with_a_bar_is_a_documents_note() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .width(420.0)
      .spacing(8.0)
      .child(
        Callout::new("Large radii cost more on integrated GPUs.")
          .title("Warning")
          .variant(AlertVariant::Warning)
          .bar(true)
          .id("note"),
      )
      .child(
        Callout::new("Blurring a video node disables direct scanout.")
          .title("Caution")
          .variant(AlertVariant::Destructive)
          .icon(icons::ALERT_OCTAGON)
          .bar(true)
          .id("caution"),
      )
      .into()
  });
  let (x, y, ..) = stage.bounds("note");
  assert_eq!(stage.fill_color("note"), Some(light(PaletteColor::WarningMuted)));
  assert_eq!(
    start_line(&mut stage, "note"),
    Some((3.0, light(PaletteColor::Warning)))
  );
  assert_eq!(corners(&mut stage, "note"), (0.0, 6.0), "square on the side of the bar");

  // The icon before the title of 14 in the variant's color, the text of 14 under them.
  let sign = stage.text_quad(&glyph(icons::WARNING)).unwrap();
  let title = stage.text_quad("Warning").unwrap();
  assert_eq!(stage.text("Warning"), (light(PaletteColor::Warning), 14.0));
  assert!((sign.x - x - 14.0).abs() < 4.0, "at the start, after the bar");
  assert!((title.x - (sign.x + 16.0 + 8.0)).abs() < 0.5, "8 after the icon");
  assert!((sign.y - y - 12.0).abs() < 3.0);
  let text = stage.text_quad("Large radii cost more on integrated GPUs.").unwrap();
  assert_eq!(
    stage.text("Large radii cost more on integrated GPUs."),
    (light(PaletteColor::Foreground), 14.0)
  );
  assert!(text.y >= title.y + title.height, "under the title");
  assert!((text.x - sign.x).abs() < 0.5, "from the start, under the icon");

  // An icon of its own, and the destructive color.
  assert_eq!(
    stage.text(&glyph(icons::ALERT_OCTAGON)).0,
    light(PaletteColor::Destructive)
  );
  assert_eq!(stage.fill_color("caution"), Some(light(PaletteColor::DestructiveMuted)));
  assert_eq!(
    start_line(&mut stage, "caution"),
    Some((3.0, light(PaletteColor::Destructive)))
  );
}

#[test]
fn a_callout_holds_a_title_and_the_apps_elements_beside_its_icon() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .width(380.0)
      .child(
        Callout::new("Use a lighter shadow in dark mode.")
          .title("Tip")
          .child(Row::new().size(100.0, 20.0).id("more"))
          .id("callout"),
      )
      .into()
  });
  let (x, ..) = stage.bounds("callout");
  assert_eq!(stage.fill_color("callout"), Some(light(PaletteColor::Muted)));
  let title = stage.text_quad("Tip").unwrap();
  let text = stage.text_quad("Use a lighter shadow in dark mode.").unwrap();
  let (more_x, more_y, ..) = stage.bounds("more");
  assert!((title.x - x - 40.0).abs() < 0.5, "beside the icon");
  assert!(text.y >= title.y + title.height);
  assert!(more_y >= text.y + text.height);
  assert!((more_x - title.x).abs() < 0.5);
}

#[test]
fn a_callout_is_a_note_to_tooling_and_a_theme_changes_it() {
  let roles = Arc::new(Mutex::new(Vec::new()));
  let seen = roles.clone();
  let app = App::new();
  let renderer: CalloutRenderer = Arc::new(move |view: CalloutView| {
    seen.lock().unwrap().push(view.role.to_string());
    view.render()
  });
  app.theme().set_renderer::<Callout>(renderer);
  let _stage = Stage::in_app(app, |_| Column::new().width(300.0).child(Callout::new("A note")).into());
  assert_eq!(roles.lock().unwrap().last().map(String::as_str), Some("note"));

  let mut stage = Stage::new(|_| {
    Column::new()
      .width(300.0)
      .child(
        Callout::new("Ruled")
          .id("callout")
          .style(CalloutPart::Root, |s| s.background(PaletteColor::Secondary))
          .style(CalloutPart::Label, |s| s.font_size(18.0)),
      )
      .into()
  });
  assert_eq!(stage.fill_color("callout"), Some(light(PaletteColor::Secondary)));
  assert_eq!(stage.text("Ruled").1, 18.0);
}

/// A variant is a name: one a theme adds reaches the recipe, and an alert, a banner and a callout of it have the
/// neutral one's icon (and an alert and a banner its role).
#[test]
fn a_variant_a_theme_adds_reaches_the_recipe_with_the_neutral_icon() {
  let brand = Recipe::new().variant("brand", AlertPart::Root, |s| s.background(PaletteColor::Secondary));
  let roles = Arc::new(Mutex::new(Vec::new()));
  let seen = roles.clone();
  let app = App::new();
  let renderer: AlertRenderer = Arc::new(move |view: AlertView| {
    seen
      .lock()
      .unwrap()
      .push((view.variant, view.role.to_string(), view.icon));
    view.render()
  });
  app.theme().set_renderer::<Alert>(renderer);
  let mut stage = Stage::in_app(app, move |_| {
    Column::new()
      .width(360.0)
      .child(Alert::new("Brand").variant("brand").rules(&brand).id("alert"))
      .child(Banner::new("Brand banner").variant("brand").id("banner"))
      .child(Callout::new("Brand callout").variant("brand").id("callout"))
      .into()
  });
  assert_eq!(stage.fill_color("alert"), Some(light(PaletteColor::Secondary)));
  let (variant, role, icon) = roles.lock().unwrap().last().cloned().unwrap();
  assert_eq!(variant, Name::new("brand"));
  assert_eq!(role, "status");
  assert_eq!(icon, Some(Name::new(icons::INFO)), "the neutral one's icon");
  // The banner's neutral icon is the megaphone, the callout's the information sign.
  assert!(stage.paints(&glyph(icons::MEGAPHONE)));
  assert_eq!(
    stage
      .quads()
      .iter()
      .filter(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if *text == glyph(icons::INFO)))
      .count(),
    2,
    "the alert's and the callout's"
  );
}

/// An alert and a banner have no close control until they are given `on_close`, and one once they are.
#[test]
fn an_alert_and_a_banner_have_a_close_control_once_they_are_given_on_close() {
  let closed = Arc::new(AtomicUsize::new(0));
  let (alert_closed, banner_closed) = (closed.clone(), closed.clone());
  let plain = Stage::new(|_| {
    Column::new()
      .width(360.0)
      .child(Alert::new("Saved"))
      .child(Banner::new("Saved"))
      .into()
  });
  assert!(!plain.paints(&glyph(icons::CLOSE)));
  let mut closing = Stage::new(move |_| {
    let (alert_closed, banner_closed) = (alert_closed.clone(), banner_closed.clone());
    Column::new()
      .width(360.0)
      .child(Alert::new("Saved").on_close(move || {
        alert_closed.fetch_add(1, Ordering::Relaxed);
      }))
      .child(Banner::new("Saved").on_close(move || {
        banner_closed.fetch_add(10, Ordering::Relaxed);
      }))
      .into()
  });
  let closes: Vec<Quad> = closing
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if *text == glyph(icons::CLOSE)))
    .collect();
  assert_eq!(closes.len(), 2);
  for close in &closes {
    let (x, y) = (close.x + close.width / 2.0, close.y + close.height / 2.0);
    closing.tree.mouse_move(x, y);
    closing.tree.mouse_down(x, y, MouseButton::Left);
    closing.tree.mouse_up(x, y, MouseButton::Left);
    closing.pass();
  }
  assert_eq!(closed.load(Ordering::Relaxed), 11);
}

/// `label` is what an alert, a banner and a callout are called to tooling, in place of their title or text.
#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn a_label_is_what_a_message_is_called_to_tooling() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .width(360.0)
      .child(Alert::new("Could not save").label("Save failed").id("alert"))
      .child(Alert::new("Saved").id("plain"))
      .child(Banner::new("lurq 0.61 is available").label("Update").id("banner"))
      .child(Callout::new("Use blur.").title("Tip").label("Blur tip").id("callout"))
      .into()
  });
  let mut said = |id: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description("a11y_name"))
  };
  assert_eq!(said("alert").as_deref(), Some("Save failed"));
  assert_eq!(said("plain").as_deref(), Some("Saved"));
  assert_eq!(said("banner").as_deref(), Some("Update"));
  assert_eq!(said("callout").as_deref(), Some("Blur tip"));
}

/// Words long enough to take several lines of an alert 300 wide.
const LONG: &str = "The token this project signs in with expires in three days, and a run that starts after that fails.";

#[test]
fn an_alert_aligned_to_the_top_has_its_icon_and_actions_on_its_first_line() {
  let alert = |top: bool, id: &str| {
    Alert::new("Token expires")
      .description(LONG)
      .variant(AlertVariant::Warning)
      .align_top(top)
      .action(small("Rotate", &format!("{id}-rotate")))
      .id(id.to_owned())
  };
  let mut stage = Stage::new(move |_| {
    Column::new()
      .width(300.0)
      .child(alert(false, "middle"))
      .child(alert(true, "top"))
      .into()
  });
  let (_, middle_y, _, middle_height) = stage.bounds("middle");
  let (_, rotate_y, _, rotate_height) = stage.bounds("middle-rotate");
  assert!(
    ((rotate_y + rotate_height / 2.0) - (middle_y + middle_height / 2.0)).abs() < 1.0,
    "centered unless told"
  );
  let (_, y, ..) = stage.bounds("top");
  let (_, rotate_y, ..) = stage.bounds("top-rotate");
  assert!((rotate_y - y - 16.0).abs() < 0.5, "the action at the top: {}", rotate_y - y);
  let (_, icon_y, ..) = stage.bounds("top-icon");
  assert!((icon_y - y - 16.0).abs() < 2.0, "the icon at the top too");
}

#[test]
fn a_note_comes_after_the_actions() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .width(300.0)
      .child(
        Alert::new("Rotate the token?")
          .stacked(true)
          .action(small("Rotate", "rotate"))
          .note("A new token is shown once.")
          .id("stacked"),
      )
      .child(
        Alert::new("Signed out")
          .description("Sign in to sync.")
          .action(small("Sign in", "sign-in"))
          .note("Nothing is lost.")
          .id("end"),
      )
      .into()
  });
  let (_, rotate_y, _, rotate_height) = stage.bounds("rotate");
  let (note_x, note_y, ..) = stage.bounds("stacked-note");
  let title = stage.text_quad("Rotate the token?").unwrap();
  assert!(note_y >= rotate_y + rotate_height, "under the actions");
  assert!((note_x - title.x).abs() < 0.5, "at the words' start");
  let (_, note_y, ..) = stage.bounds("end-note");
  let (_, description_y, _, description_height) = stage.bounds("end-description");
  assert!(note_y >= description_y + description_height, "under the words where the actions are at the end");
  assert_eq!(
    stage.text("Nothing is lost.").0,
    light(PaletteColor::MutedForeground),
    "in the muted color"
  );
}

#[test]
fn an_untitled_alert_draws_no_title() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .width(300.0)
      .child(Alert::untitled().description("The token expires in 3 days.").id("alert"))
      .into()
  });
  assert!(!stage.ids().contains(&"alert-title".to_owned()), "no title part");
  let (_, y, ..) = stage.bounds("alert");
  let (_, description_y, ..) = stage.bounds("alert-description");
  assert!((description_y - y - 16.0).abs() < 0.5, "the words at the top");
}

#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn an_untitled_alert_is_called_by_its_description() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .width(300.0)
      .child(Alert::untitled().description("The token expires in 3 days.").id("alert"))
      .into()
  });
  let name = stage
    .tree
    .get_element_by_id_mut("alert")
    .and_then(|element| element.description("a11y_name"));
  assert_eq!(name.as_deref(), Some("The token expires in 3 days."));
}

#[test]
fn a_compact_alert_has_its_icon_beside_the_title_alone_and_its_actions_at_the_end() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .width(300.0)
      .child(
        Alert::new("Not synced")
          .description(LONG)
          .variant(AlertVariant::Info)
          .compact(true)
          .actions_end(true)
          .action(small("Push", "push"))
          .id("alert"),
      )
      .into()
  });
  let (x, y, width, height) = stage.bounds("alert");
  let (icon_x, icon_y, ..) = stage.bounds("alert-icon");
  let title = stage.text_quad("Not synced").unwrap();
  let (description_x, description_y, ..) = stage.bounds("alert-description");
  assert!(title.y - y < 20.0 && (icon_y - y - 16.0).abs() < 2.0, "the icon and the title on the first line");
  assert!(title.x > icon_x + 16.0, "the title beside the icon");
  assert!((description_x - icon_x).abs() < 0.5, "the words under them from the icon's start: {description_x} {x}");
  assert!(description_y > title.y + title.height - 1.0, "the words under the title");
  let (push_x, push_y, push_width, push_height) = stage.bounds("push");
  assert!((x + width - 16.0 - (push_x + push_width)).abs() < 0.5, "the action at the far end");
  assert!(push_y > description_y, "under the words");
  assert!((y + height - 16.0 - (push_y + push_height)).abs() < 0.5, "and the box holds it");
  assert!(stage.ids().contains(&"alert-head".to_owned()));
}
