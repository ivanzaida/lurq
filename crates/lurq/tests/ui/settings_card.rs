//! A settings card: the card of one setting and the card of several as the board draws them, a card that opens to
//! more settings, who keeps whether it is open, the control in its row, the keys, what tooling reads, the
//! density, a narrow room, and what a theme changes.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App,
    theme::{Density, OpacityLevel, PaletteColor, ThemePalette},
  },
  components::{Column, Row},
  core::Signal,
  layout::{quad::QuadContent, text_style::FontWeight},
  node::{Element, color::Color, dimension::Dimension},
  ui::{
    Button, Case, Interaction, ListItem, Opacity, SettingsCard, SettingsCardPart, SettingsCardRenderer,
    SettingsCardState, SettingsCardView, Switch, icons,
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

/// The weight of the text `content` as painted.
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

/// The color of the veil over the element with `id`: an inset shadow over its whole box.
fn veil_of(stage: &mut Stage, id: &str) -> Option<Color> {
  let (x, y, width, height) = stage.bounds(id);
  stage.quads().into_iter().find_map(|quad| match &quad.content {
    QuadContent::BoxShadow(shadow)
      if shadow.inset && (quad.x, quad.y, quad.width, quad.height) == (x, y, width, height) =>
    {
      Some(shadow.color)
    }
    _ => None,
  })
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

/// A window of 400 by 400 that leaves `card` the board's 340, 8 in from its corner.
fn page(card: impl Into<Element>) -> Element {
  Column::new()
    .size(400.0, 400.0)
    .child(Column::new().width(356.0).padding(8.0).child(card))
    .into()
}

/// The board's Settings Card: the theme of the app, with a control of the board's 110 by 30 at its end.
fn theme_row() -> ListItem {
  ListItem::new("App theme")
    .icon(icons::PALETTE)
    .description("Choose how lurq looks.")
    .trailing(Row::new().id("control").size(110.0, 30.0))
    .id("theme")
}

/// The rows the card of the theme opens to.
fn more() -> [ListItem; 2] {
  [
    ListItem::new("High contrast")
      .trailing(Switch::new("High contrast").label_hidden(true).id("contrast-switch"))
      .id("contrast"),
    ListItem::new("Accent color")
      .value("Blue")
      .chevron(true)
      .on_click(|_| {})
      .id("accent"),
  ]
}

/// What `on_open_change` was told, in order.
fn log() -> (Arc<Mutex<Vec<bool>>>, impl Fn(bool) + Send + Sync + Clone + 'static) {
  let heard = Arc::new(Mutex::new(Vec::new()));
  let told = heard.clone();
  (heard, move |open: bool| told.lock().unwrap().push(open))
}

#[test]
fn the_card_of_one_setting_is_the_board_s_61_with_its_icon_its_text_and_its_control() {
  let mut stage = Stage::new(|_| page(SettingsCard::new(theme_row()).id("card")));
  // The card: 340 by 61 on the card's fill, in an edge of 1 with the large radius.
  let (x, y, width, height) = stage.bounds("card");
  assert_eq!((width, height), (340.0, 61.0));
  assert_eq!(stage.fill_color("card"), Some(light(PaletteColor::Card)));
  let edge = stage.border("card").expect("the card has an edge");
  let side = edge.border.and_then(|border| border.top).unwrap();
  assert_eq!((side.width, side.color), (1.0, light(PaletteColor::Border)));
  assert!(
    edge.border_radius.is_some_and(|radius| radius.top_left == 8.0),
    "radius-lg"
  );

  // The icon: 20 in the foreground, 14 in, in the middle of the card's height.
  let icon = stage.text_quad(&glyph(icons::PALETTE)).unwrap();
  assert_eq!(
    stage.text(&glyph(icons::PALETTE)),
    (light(PaletteColor::Foreground), 20.0)
  );
  assert!((icon.x - x - 14.0).abs() < 0.01);
  assert!((icon.y + icon.height / 2.0 - (y + height / 2.0)).abs() < 0.51);

  // The text 12 after the icon: the title 13 at 500 over the description in 12 of the muted color, 2 apart.
  let title = stage.text_quad("App theme").unwrap();
  assert!((title.x - x - 46.0).abs() < 0.01, "14, the icon's 20 and 12");
  assert_eq!(stage.text("App theme"), (light(PaletteColor::Foreground), 13.0));
  assert_eq!(weight_of(&stage, "App theme"), FontWeight::Medium);
  let description = stage.text_quad("Choose how lurq looks.").unwrap();
  assert_eq!(
    stage.text("Choose how lurq looks."),
    (light(PaletteColor::MutedForeground), 12.0)
  );
  assert!((description.y - (title.y + title.height) - 2.0).abs() < 0.01);
  assert!(
    ((title.y + description.y + description.height) / 2.0 - (y + height / 2.0)).abs() < 0.51,
    "the text is in the middle of the card"
  );

  // The control 14 from the end, in the middle; a card with no rows under it has no chevron.
  let control = stage.bounds("control");
  assert!((x + width - (control.0 + control.2) - 14.0).abs() < 0.01);
  assert!((control.1 + 15.0 - (y + height / 2.0)).abs() < 0.51);
  assert!(!stage.paints(&glyph(icons::CHEVRON_DOWN)));
}

#[test]
fn a_card_that_opens_has_a_chevron_of_16_at_its_end_12_after_the_control() {
  let mut stage = Stage::new(|_| page(SettingsCard::new(theme_row()).items(more()).id("card")));
  let (x, _, width, height) = stage.bounds("card");
  assert_eq!(height, 61.0, "closed, it is its own row");
  let chevron = stage.text_quad(&glyph(icons::CHEVRON_DOWN)).unwrap();
  assert_eq!(
    stage.text(&glyph(icons::CHEVRON_DOWN)),
    (light(PaletteColor::MutedForeground), 16.0)
  );
  assert!((x + width - (chevron.x + 16.0) - 14.0).abs() < 0.01, "14 from the end");
  let control = stage.bounds("control");
  assert!(
    (chevron.x - (control.0 + control.2) - 12.0).abs() < 0.01,
    "12 after the control"
  );
  assert!(!stage.paints("High contrast"), "what it opens to is not there");
}

#[test]
fn a_card_of_several_settings_is_rows_of_57_with_a_line_between_each_two() {
  let mut stage = Stage::new(|_| {
    page(
      SettingsCard::group([
        ListItem::new("Open at login")
          .description("Start lurq when you sign in to this computer.")
          .trailing(Row::new().id("switch").size(36.0, 20.0))
          .id("login"),
        ListItem::new("Default project folder")
          .description("Where new projects are created.")
          .trailing(Row::new().id("choose").size(88.0, 28.0))
          .id("folder"),
      ])
      .label("Startup")
      .id("card"),
    )
  });
  let card = stage.bounds("card");
  assert_eq!((card.2, card.3), (340.0, 114.0), "two rows of 57");
  assert_eq!(stage.fill_color("card"), Some(light(PaletteColor::Card)));
  let (login, folder) = (stage.bounds("login"), stage.bounds("folder"));
  assert_eq!((login.2, login.3), (340.0, 57.0));
  assert_eq!(folder.1 - login.1, 57.0, "the rows touch");
  // A row: its text 14 in, the title 13 at 500, the control 14 from the end.
  let title = stage.text_quad("Open at login").unwrap();
  assert!((title.x - login.0 - 14.0).abs() < 0.01);
  assert_eq!(stage.text("Open at login"), (light(PaletteColor::Foreground), 13.0));
  assert_eq!(weight_of(&stage, "Open at login"), FontWeight::Medium);
  assert_eq!(
    stage.text("Start lurq when you sign in to this computer."),
    (light(PaletteColor::MutedForeground), 12.0)
  );
  let switch = stage.bounds("switch");
  assert!((login.0 + login.2 - (switch.0 + switch.2) - 14.0).abs() < 0.01);
  // The line: 1 high in the border's color, along the bottom of the first row, the width of the card.
  let line = stage
    .quads()
    .into_iter()
    .find(|quad| {
      quad.height == 1.0
        && quad.width == 340.0
        && matches!(&quad.content, QuadContent::Rect { color, .. } if *color == light(PaletteColor::Border))
    })
    .expect("a line between the two rows");
  assert_eq!(line.y, login.1 + 56.0, "inside the row over it");
}

#[test]
fn a_press_of_its_row_opens_the_card_and_tells_what_it_is_to_be() {
  let (heard, tell) = log();
  let open = Signal::new(false);
  let bound = open.clone();
  let mut stage = Stage::new(move |_| {
    page(
      SettingsCard::new(theme_row())
        .items(more())
        .bind(bound.clone())
        .on_open_change(tell.clone())
        .id("card"),
    )
  });
  stage.click("theme");
  assert!(open.get(), "the signal is set");
  assert_eq!(*heard.lock().unwrap(), [true]);
  // Open: the chevron points up, and the rows are under the card's own, under a line.
  assert!(stage.paints(&glyph(icons::CHEVRON_UP)) && !stage.paints(&glyph(icons::CHEVRON_DOWN)));
  assert!(stage.paints("High contrast") && stage.paints("Accent color"));
  let (card, theme, contrast) = (stage.bounds("card"), stage.bounds("theme"), stage.bounds("contrast"));
  assert_eq!(contrast.1, theme.1 + 61.0, "under the card's own row");
  assert_eq!(stage.bounds("accent").1, contrast.1 + contrast.3, "one under another");
  assert_eq!(card.3, 61.0 + contrast.3 + stage.bounds("accent").3);
  assert!(
    stage.quads().into_iter().any(|quad| quad.y == contrast.1
      && quad.width == 340.0
      && quad
        .border
        .and_then(|border| border.top)
        .is_some_and(|side| (side.width, side.color) == (1.0, light(PaletteColor::Border)))),
    "a line over the rows"
  );
  // A press again closes it; the app setting the signal tells nobody.
  stage.click("theme");
  assert!(!open.get());
  assert_eq!(*heard.lock().unwrap(), [true, false]);
  assert!(!stage.paints("High contrast"));
  open.set(true);
  stage.pass();
  assert!(stage.paints("High contrast"), "the app opened it");
  assert_eq!(heard.lock().unwrap().len(), 2, "and no handler heard of it");
}

#[test]
fn the_app_holds_whether_it_is_open_or_the_mounted_card_keeps_it() {
  // Told with `open`, the card opens nothing itself.
  let (heard, tell) = log();
  let mut stage = Stage::new(move |_| {
    page(
      SettingsCard::new(theme_row())
        .items(more())
        .open(false)
        .on_open_change(tell.clone())
        .id("card"),
    )
  });
  stage.click("theme");
  assert_eq!(*heard.lock().unwrap(), [true]);
  assert!(!stage.paints("High contrast"), "the app did not say it is open");

  // Not mounted and told nothing, it is its still form, as it starts.
  let mut still = Stage::new(|_| page(SettingsCard::new(theme_row()).items(more()).default_open(true)));
  assert!(still.paints("High contrast"));
  still.click("theme");
  assert!(still.paints("High contrast"), "nothing keeps what a press would change");

  // Mounted, it keeps it: from where it starts, and its handlers are told after it is so.
  let (heard, tell) = log();
  let mut stage = Stage::new(move |ctx| {
    page(
      SettingsCard::new(theme_row())
        .items(more())
        .default_open(true)
        .on_open_change(tell.clone())
        .id("card")
        .mount(ctx),
    )
  });
  assert!(stage.paints("High contrast"), "it starts open");
  stage.click("theme");
  assert!(!stage.paints("High contrast"));
  stage.click("theme");
  assert!(stage.paints("High contrast"));
  assert_eq!(*heard.lock().unwrap(), [false, true]);
}

/// A mounted card that opens, with a button in its own row: the stage, what `on_open_change` heard, and how often
/// the button was pressed.
fn with_a_button() -> (Stage, Arc<Mutex<Vec<bool>>>, Arc<Mutex<usize>>) {
  let (heard, tell) = log();
  let presses = Arc::new(Mutex::new(0));
  let count = presses.clone();
  let stage = Stage::new(move |ctx| {
    let count = count.clone();
    page(
      SettingsCard::new(
        ListItem::new("App theme")
          .description("Choose how lurq looks.")
          .trailing(
            Button::new("Reset")
              .id("reset")
              .on_click(move |_| *count.lock().unwrap() += 1),
          )
          .id("theme"),
      )
      .items(more())
      .on_open_change(tell.clone())
      .id("card")
      .mount(ctx),
    )
  });
  (stage, heard, presses)
}

#[test]
fn a_press_of_the_control_in_the_row_is_the_control_s_and_not_the_card_s() {
  let (mut stage, heard, presses) = with_a_button();
  stage.click("reset");
  assert_eq!(*presses.lock().unwrap(), 1);
  assert!(heard.lock().unwrap().is_empty(), "the card did not open");
  assert!(!stage.paints("High contrast"));

  // From the keyboard: Tab stops on the row, then on the control in it; Enter and Space on the row open and close.
  let (mut stage, heard, presses) = with_a_button();
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("theme"));
  stage.key("Enter");
  assert_eq!(*heard.lock().unwrap(), [true]);
  stage.key("Space");
  assert_eq!(*heard.lock().unwrap(), [true, false]);
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("reset"));
  stage.key("Enter");
  assert_eq!(*presses.lock().unwrap(), 1);
  assert_eq!(heard.lock().unwrap().len(), 2, "Enter on the control is the control's");
}

#[test]
fn an_open_card_s_rows_are_stops_of_tab_and_a_closed_card_s_are_not() {
  let open = Signal::new(true);
  let bound = open.clone();
  let mut stage = Stage::new(move |_| {
    page(
      SettingsCard::new(
        ListItem::new("App theme")
          .description("Choose how lurq looks.")
          .id("theme"),
      )
      .items(more())
      .bind(bound.clone())
      .id("card"),
    )
  });
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("theme"));
  stage.key("Tab");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("contrast-switch"),
    "the switch of the first row"
  );
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("accent"), "the row that is pressed");
  // The focus is on the row and the ring is inside it.
  assert!(stage.has_ring(light(PaletteColor::Ring)));
  // Closed from its own row, the rows are gone: Tab from the row leaves the card.
  stage.click("theme");
  assert!(!open.get());
  stage.tree.mouse_move(390.0, 390.0);
  stage.key("Tab");
  assert_ne!(focused(&stage).as_deref(), Some("contrast-switch"));
  assert_ne!(focused(&stage).as_deref(), Some("accent"));
}

#[test]
fn the_row_of_a_card_that_opens_is_veiled_under_the_pointer_and_a_disabled_card_takes_nothing() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .size(400.0, 400.0)
      .padding(8.0)
      .spacing(8.0)
      .child(
        SettingsCard::new(ListItem::new("App theme").id("theme"))
          .items(more())
          .force(Some(Interaction::Hovered)),
      )
      .child(SettingsCard::new(ListItem::new("Language").id("still")))
      .into()
  });
  assert_eq!(veil_of(&mut stage, "theme"), Some(light(PaletteColor::StateHover)));
  stage.hover("still");
  assert_eq!(
    veil_of(&mut stage, "still"),
    None,
    "a row that opens nothing answers no pointer"
  );

  let (heard, tell) = log();
  let mut stage = Stage::new(move |ctx| {
    page(
      SettingsCard::new(theme_row())
        .items(more())
        .disabled(true)
        .on_open_change(tell.clone())
        .id("card")
        .mount(ctx),
    )
  });
  stage.click("theme");
  stage.key("Tab");
  stage.key("Enter");
  assert!(heard.lock().unwrap().is_empty());
  assert_eq!(focused(&stage), None, "nothing in it is a stop of Tab");
  let theme = stage.app.theme().clone();
  let case = Case::new().state(SettingsCardState::Disabled, true);
  assert_eq!(
    theme
      .resolve::<SettingsCard>(&case, None)
      .part(SettingsCardPart::Root)
      .rest()
      .opacity,
    Some(Opacity::Level(OpacityLevel::Disabled))
  );
}

#[cfg(any(feature = "devtools", feature = "mcp"))]
#[test]
fn to_tooling_the_card_is_a_group_and_its_row_says_whether_it_is_open() {
  let open = Signal::new(false);
  let bound = open.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .size(400.0, 400.0)
      .padding(8.0)
      .spacing(8.0)
      .child(
        SettingsCard::new(theme_row())
          .items(more())
          .bind(bound.clone())
          .id("card"),
      )
      .child(
        SettingsCard::group([ListItem::new("Open at login").id("login")])
          .label("Startup")
          .id("startup"),
      )
      .into()
  });
  let said = |stage: &mut Stage, id: &str, key: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(key))
  };
  assert_eq!(said(&mut stage, "card", "a11y_role").as_deref(), Some("group"));
  assert_eq!(
    said(&mut stage, "card", "a11y_name").as_deref(),
    Some("App theme"),
    "called by its own row"
  );
  assert_eq!(said(&mut stage, "theme", "a11y_role").as_deref(), Some("button"));
  assert_eq!(said(&mut stage, "theme", "a11y_expanded").as_deref(), Some("false"));
  assert_eq!(
    said(&mut stage, "theme", "a11y_description").as_deref(),
    Some("Choose how lurq looks.")
  );
  stage.click("theme");
  assert_eq!(said(&mut stage, "theme", "a11y_expanded").as_deref(), Some("true"));
  assert_eq!(said(&mut stage, "startup", "a11y_name").as_deref(), Some("Startup"));
  assert_eq!(said(&mut stage, "login", "a11y_role").as_deref(), Some("listitem"));
  assert_eq!(said(&mut stage, "login", "a11y_expanded"), None);
}

#[test]
fn a_setting_follows_the_density_and_a_setting_of_one_line_is_44() {
  let heights = |density: Option<Density>| {
    let mut stage = Stage::new(move |_| {
      let (card, group) = (
        SettingsCard::new(theme_row()).id("card"),
        SettingsCard::group([
          ListItem::new("Open at login")
            .description("Start lurq when you sign in.")
            .id("two"),
          ListItem::new("Check for updates").id("one"),
        ]),
      );
      let (card, group) = match density {
        Some(density) => (card.density(density), group.density(density)),
        None => (card, group),
      };
      Column::new()
        .size(400.0, 400.0)
        .child(
          Column::new()
            .width(356.0)
            .padding(8.0)
            .spacing(8.0)
            .child(card)
            .child(group),
        )
        .into()
    });
    (stage.bounds("card").3, stage.bounds("two").3, stage.bounds("one").3)
  };
  assert_eq!(heights(None), (61.0, 57.0, 44.0));
  assert_eq!(heights(Some(Density::Compact)), (53.0, 49.0, 36.0), "8 lower");
  assert_eq!(heights(Some(Density::Comfortable)), (69.0, 65.0, 52.0), "8 higher");
  assert_eq!(heights(Some(Density::Touch)), (73.0, 69.0, 56.0), "12 higher");

  // The theme's density is the card's unless it says its own.
  let app = App::new();
  app.theme().set_density(Density::Compact);
  let mut stage = Stage::in_app(app, |_| page(SettingsCard::new(theme_row()).id("card")));
  assert_eq!(stage.bounds("card").3, 53.0);
}

#[test]
fn a_title_longer_than_its_room_is_cut_a_description_wraps_and_the_control_keeps_its_room() {
  let title = "The folder every new project of this workspace is created in";
  let description = "New projects are created here unless the dialog that makes one is told another place.";
  let mut stage = Stage::new(move |_| {
    Column::new()
      .size(400.0, 400.0)
      .child(
        Column::new().width(236.0).padding(8.0).child(
          SettingsCard::group([ListItem::new(title)
            .description(description)
            .trailing(Row::new().id("control").size(88.0, 28.0))
            .id("row")])
          .id("card"),
        ),
      )
      .into()
  });
  let (x, _, width, height) = stage.bounds("card");
  assert_eq!(width, 220.0, "as wide as its room");
  let control = stage.bounds("control");
  assert_eq!(control.2, 88.0, "the control keeps its width");
  assert!((x + width - (control.0 + control.2) - 14.0).abs() < 0.01);
  assert!(!stage.paints(title), "the title has no room for all of itself");
  assert!(stage.cut_text_quad(title).is_some(), "it is cut with an ellipsis");
  let wrapped = stage.text_quad(description).expect("the description is painted");
  assert!(
    wrapped.height > 40.0,
    "the description goes on to the next lines: {}",
    wrapped.height
  );
  assert!(height > 57.0, "and the row grows with it: {height}");
  for quad in stage.quads() {
    if matches!(&quad.content, QuadContent::Text { .. }) {
      assert!(quad.x + quad.width <= control.0 + 0.5, "no text runs under the control");
    }
  }
}

#[test]
fn in_a_room_too_narrow_for_its_text_beside_the_control_the_control_goes_under_the_text() {
  // 256 leaves the text 46 beside the icon, the control of 110 and the chevron: under the least of 80, so the row
  // is two lines.
  let mut stage = Stage::new(|_| {
    Column::new()
      .size(400.0, 400.0)
      .child(
        Column::new().width(272.0).padding(8.0).child(
          // As a page holds it: a width of its own, no wider than its room.
          Column::new()
            .width(360.0)
            .max_width(Dimension::Pct(100.0))
            .child(SettingsCard::new(theme_row()).items(more()).id("card")),
        ),
      )
      .into()
  });
  let (x, y, width, height) = stage.bounds("card");
  assert_eq!(width, 256.0);
  let title = stage.text_quad("App theme").expect("the title is whole");
  let description = stage
    .text_quad("Choose how lurq looks.")
    .expect("the description is painted");
  assert!(
    (title.x - x - 46.0).abs() < 0.01,
    "the text keeps its place after the icon"
  );
  assert!(
    description.width >= 60.0,
    "the description has its least room and is not broken in its words: {}",
    description.width
  );
  let control = stage.bounds("control");
  assert_eq!(control.2, 110.0, "the control keeps its width");
  assert!(
    control.1 >= description.y + description.height,
    "the control is under the text: {control:?} {}",
    description.y + description.height
  );
  assert!((control.0 - x - 14.0).abs() < 0.01, "at the row's start: {control:?}");
  let chevron = stage.text_quad(&glyph(icons::CHEVRON_DOWN)).unwrap();
  assert!(
    chevron.x + chevron.width <= x + width - 14.0 + 0.01,
    "the chevron stays in the card"
  );
  assert!(
    control.1 + control.3 <= y + height - 12.0 + 0.01,
    "the card holds both lines: {height}"
  );
  for quad in stage.quads() {
    assert!(
      quad.x + quad.width <= x + width + 0.5,
      "nothing runs past the card: {} {}",
      quad.x,
      quad.width
    );
  }
}

#[test]
fn a_card_with_a_width_of_its_own_keeps_it_and_is_no_wider_than_a_narrow_room() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .size(400.0, 400.0)
      .spacing(8.0)
      .child(SettingsCard::new(theme_row()).width(300.0).id("fixed"))
      .child(
        Row::new().id("room").width(200.0).child(
          SettingsCard::new(ListItem::new("Language").id("language"))
            .width(340.0)
            .id("narrow"),
        ),
      )
      .into()
  });
  assert_eq!(stage.bounds("fixed").2, 300.0);
  assert!(stage.bounds("narrow").2 <= 200.5, "its width is the most it takes");
}

#[test]
fn a_theme_restyles_the_card_and_a_renderer_builds_another_with_the_same_behavior() {
  let app = App::new();
  app.theme().patch_recipe::<SettingsCard>(|rules| {
    rules
      .base(SettingsCardPart::Root, |s| s.background(PaletteColor::Muted))
      .base(SettingsCardPart::Header, |s| s.padding_y(20.0))
  });
  let mut stage = Stage::in_app(app, |_| page(SettingsCard::new(theme_row()).id("card")));
  assert_eq!(stage.fill_color("card"), Some(light(PaletteColor::Muted)));
  assert_eq!(stage.bounds("card").3, 72.0, "20 over and under the row's text of 32");

  let (heard, tell) = log();
  let app = App::new();
  let renderer: SettingsCardRenderer = Arc::new(|mut view: SettingsCardView| {
    // The rows over the card's own row: other nodes, the same behavior.
    let header = view.header();
    let panel = view.panel();
    let mut root = view.root().child(panel);
    if let Some(header) = header {
      root = root.child(header);
    }
    root.into()
  });
  app.theme().set_renderer::<SettingsCard>(renderer);
  let mut stage = Stage::in_app(app, move |ctx| {
    page(
      SettingsCard::new(theme_row())
        .items(more())
        .on_open_change(tell.clone())
        .id("card")
        .mount(ctx),
    )
  });
  stage.click("theme");
  assert_eq!(*heard.lock().unwrap(), [true]);
  assert!(
    stage.bounds("contrast").1 < stage.bounds("theme").1,
    "the rows are over the row"
  );
}
