//! A list item and a list: a row against the board (its height by density, what is before and after its text), a
//! row that is pressed and one that is not, a selected and a disabled row, the lines between rows and their inset,
//! the contained list with its header, a path in the router, and a theme's other anatomy.

use std::sync::{
  Arc, Mutex,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{
    App,
    events::{MouseButton, MouseEvent},
    theme::{Density, PaletteColor, Theme, ThemePalette},
  },
  components::{Column, Row, Text},
  core::Signal,
  layout::{
    quad::{Quad, QuadContent},
    text_style::FontWeight,
  },
  node::color::Color,
  ui::{
    Avatar, Case, LeadingAlign, List, ListItem, ListItemPart, ListItemRenderer, ListItemState, ListItemView, ListPart,
    Switch, Themed, icons,
  },
};

use super::stage::Stage;

const CHEVRON_RIGHT: &str = "\u{E01B}";
const CHECK: &str = "\u{E017}";

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

/// The quads of the text `content`.
fn texts(stage: &Stage, content: &str) -> Vec<Quad> {
  stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text == content))
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

/// A stage of `list` in a column that leaves it the board's 340, 8 in from each side.
fn stage_of(list: impl Fn() -> List + Send + Sync + 'static) -> Stage {
  Stage::new(move |_| Column::new().width(356.0).padding(8.0).child(list()).into())
}

/// The color of the veil over the element with `id`: an inset shadow over its whole box, as a part's veil is painted.
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

#[test]
fn a_row_with_a_description_is_52_high_with_its_avatar_its_text_and_its_time_12_apart() {
  let mut stage = stage_of(|| {
    List::new().item(
      ListItem::new("Ivan Zaida")
        .description("Pushed 3 commits to feat/node-blur")
        .leading(Avatar::new("Ivan Zaida"))
        .meta("2m")
        .id("row"),
    )
  });
  let (x, y, width, height) = stage.bounds("row");
  assert_eq!((width, height), (340.0, 52.0));
  // The avatar 12 in, the title 12 after its 32, the time 12 from the end.
  let title = stage.text_quad("Ivan Zaida").unwrap();
  assert!((title.x - (x + 12.0 + 32.0 + 12.0)).abs() < 0.5, "{}", title.x - x);
  assert_eq!(stage.text("Ivan Zaida"), (light(PaletteColor::Foreground), 14.0));
  assert_eq!(weight_of(&stage, "Ivan Zaida"), FontWeight::Medium);
  let description = stage.text_quad("Pushed 3 commits to feat/node-blur").unwrap();
  assert!(
    description.y > title.y + title.height - 0.5,
    "the description is under the title"
  );
  assert_eq!(
    stage.text("Pushed 3 commits to feat/node-blur"),
    (light(PaletteColor::MutedForeground), 12.0)
  );
  let time = stage.text_quad("2m").unwrap();
  assert!((time.x + time.width - (x + width - 12.0)).abs() < 0.5);
  assert_eq!(stage.text("2m").1, 11.0);
  assert!(
    stage.font_of("2m").to_lowercase().contains("mono"),
    "{}",
    stage.font_of("2m")
  );
  // Its text and its avatar are in the middle of its height.
  assert!(((title.y + description.y + description.height) / 2.0 - (y + 26.0)).abs() < 2.0);
  // A row that is not pressed has no fill.
  assert_eq!(stage.fill_color("row"), None);
}

#[test]
fn a_row_of_one_line_is_36_and_its_value_and_chevron_are_muted_at_its_end() {
  let mut stage = stage_of(|| {
    List::new().item(
      ListItem::new("Wi-Fi")
        .icon(icons::WIFI)
        .value("Home")
        .chevron(true)
        .id("row"),
    )
  });
  let (x, _, width, height) = stage.bounds("row");
  assert_eq!(height, 36.0);
  assert_eq!(stage.text("Wi-Fi").1, 14.0);
  assert_eq!(weight_of(&stage, "Wi-Fi"), FontWeight::Normal);
  assert_eq!(stage.text("Home"), (light(PaletteColor::MutedForeground), 13.0));
  let chevron = texts(&stage, CHEVRON_RIGHT).pop().expect("a chevron");
  assert!((chevron.x + chevron.width - (x + width - 12.0)).abs() < 1.0);
  let home = stage.text_quad("Home").unwrap();
  assert!(home.x + home.width <= chevron.x - 7.5, "the value 8 before the chevron");
}

#[test]
fn each_density_has_its_height_and_under_a_finger_a_row_is_the_mobile_one() {
  for (density, one, two) in [
    (Density::Compact, 28.0, 44.0),
    (Density::Default, 36.0, 52.0),
    (Density::Comfortable, 44.0, 60.0),
    (Density::Touch, 56.0, 72.0),
  ] {
    let mut stage = stage_of(move || {
      List::new()
        .density(density)
        .item(ListItem::new("One").id("one"))
        .item(ListItem::new("Two").description("Under it").id("two"))
    });
    assert_eq!(stage.bounds("one").3, one, "{density:?}");
    assert_eq!(stage.bounds("two").3, two, "{density:?}");
  }
  let mut stage = stage_of(|| {
    List::new().density(Density::Touch).item(
      ListItem::new("Wi-Fi")
        .icon(icons::WIFI)
        .value("Home")
        .chevron(true)
        .id("row"),
    )
  });
  let (x, ..) = stage.bounds("row");
  assert_eq!(stage.text("Wi-Fi").1, 16.0);
  assert_eq!(stage.text("Home").1, 15.0);
  // The icon on its tile of 32, 16 in, on the muted fill: the title 12 after it.
  let title = stage.text_quad("Wi-Fi").unwrap();
  assert!((title.x - (x + 16.0 + 32.0 + 12.0)).abs() < 0.5, "{}", title.x - x);
  let muted = light(PaletteColor::Muted);
  assert!(stage.quads().iter().any(|quad| {
    matches!(&quad.content, QuadContent::Rect { color, .. } if *color == muted)
      && (quad.width, quad.height) == (32.0, 32.0)
      && (quad.x - (x + 16.0)).abs() < 0.5
  }));
}

#[test]
fn a_pressed_row_is_a_stop_of_tab_and_one_that_is_not_is_passed_by() {
  let presses = Arc::new(AtomicUsize::new(0));
  let counted = presses.clone();
  let mut stage = stage_of(move || {
    let counted = counted.clone();
    List::new()
      .item(ListItem::new("Plain").id("plain"))
      .item(ListItem::new("Pressed").id("pressed").on_click(move |_| {
        counted.fetch_add(1, Ordering::Relaxed);
      }))
      .item(ListItem::new("Dark mode").trailing(Switch::new("Dark mode").label_hidden(true).id("switch")))
  });
  stage.click("pressed");
  assert_eq!(presses.load(Ordering::Relaxed), 1);
  // A row that is not pressed presses nothing and takes no focus: Tab goes on from it.
  stage.click("plain");
  assert_eq!(presses.load(Ordering::Relaxed), 1);
  assert_eq!(focused(&stage), None, "the row that is not pressed takes no focus");
  stage.key("Tab");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("pressed"),
    "past the row that is not pressed"
  );
  // The focus from the keyboard is a ring inside the row; Enter and Space press it.
  assert!(stage.has_ring(light(PaletteColor::Ring)));
  stage.key("Enter");
  stage.key(" ");
  assert_eq!(presses.load(Ordering::Relaxed), 3);
  // A control in a row is pressed on its own.
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("switch"));
}

#[test]
fn the_arrows_move_between_the_rows_that_are_pressed() {
  let mut stage = stage_of(|| {
    List::new()
      .item(ListItem::new("Alpha").id("alpha").on_click(|_| {}))
      .item(ListItem::new("Beta").id("beta").on_click(|_| {}))
      .item(ListItem::new("Gamma").id("gamma").on_click(|_| {}).disabled(true))
      .item(ListItem::new("Delta").id("delta").on_click(|_| {}))
  });
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("alpha"));
  stage.key("ArrowDown");
  assert_eq!(focused(&stage).as_deref(), Some("beta"));
  stage.key("ArrowDown");
  assert_eq!(focused(&stage).as_deref(), Some("delta"), "past the disabled row");
  stage.key("a");
  assert_eq!(focused(&stage).as_deref(), Some("alpha"), "a letter");
}

#[test]
fn a_selected_row_is_highlighted_a_pressed_one_veiled_and_a_disabled_one_takes_nothing() {
  let presses = Arc::new(AtomicUsize::new(0));
  let counted = presses.clone();
  let mut stage = stage_of(move || {
    let counted = counted.clone();
    List::new()
      .item(ListItem::new("Selected").selected(true).id("selected"))
      .item(
        ListItem::new("Disabled")
          .disabled(true)
          .id("disabled")
          .on_click(move |_| {
            counted.fetch_add(1, Ordering::Relaxed);
          }),
      )
      .item(ListItem::new("Hovered").on_click(|_| {}).id("hovered"))
  });
  assert_eq!(stage.fill_color("selected"), Some(light(PaletteColor::Highlight)));
  stage.click("disabled");
  assert_eq!(presses.load(Ordering::Relaxed), 0);
  assert_eq!(veil_of(&mut stage, "hovered"), None);
  stage.hover("hovered");
  // The veil is over the row, not its fill: a selected row under the pointer stays highlighted under it.
  assert_eq!(veil_of(&mut stage, "hovered"), Some(light(PaletteColor::StateHover)));
}

#[test]
fn a_checkbox_or_a_radio_before_the_text_is_the_row_s_own() {
  let checked = Signal::new(false);
  let held = checked.clone();
  let mut stage = stage_of(move || {
    let held = held.clone();
    let on = held.get();
    List::new().item(
      ListItem::new("Backups")
        .checkbox(on)
        .id("row")
        .on_click(move |_| held.set(!on)),
    )
  });
  assert!(texts(&stage, CHECK).is_empty());
  stage.click("row");
  assert!(checked.get());
  assert_eq!(texts(&stage, CHECK).len(), 1, "the check in the box");
}

#[test]
fn the_lines_between_rows_start_at_the_inset_and_none_is_under_the_last() {
  let mut stage = stage_of(|| {
    List::new()
      .separators(true)
      .inset(60.0)
      .id("list")
      .item(ListItem::new("Wi-Fi").id("a"))
      .item(ListItem::new("Bluetooth").id("b"))
      .item(ListItem::new("Airplane mode").id("c"))
  });
  let (x, ..) = stage.bounds("list");
  let (_, a_y, _, a_height) = stage.bounds("a");
  let (_, b_y, ..) = stage.bounds("b");
  let border = light(PaletteColor::Border);
  let lines: Vec<Quad> = stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Rect { color, .. } if *color == border) && quad.height == 1.0)
    .collect();
  assert_eq!(lines.len(), 2, "between each two rows");
  assert!((lines[0].x - (x + 60.0)).abs() < 0.5, "{}", lines[0].x - x);
  // The line is inside the row over it, along its bottom, as the board draws it: it adds no height.
  assert!(
    (lines[0].y + lines[0].height - (a_y + a_height)).abs() < 0.5,
    "{}",
    lines[0].y - a_y
  );
  assert!((b_y - (a_y + a_height)).abs() < 0.5, "rows touch each other");
}

#[test]
fn a_contained_list_is_a_box_with_a_header_and_rows_that_touch() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .child(
        List::new()
          .contained(true)
          .title("Recent")
          .action(Text::new("Clear"))
          .id("list")
          .style(ListPart::Root, |s| s.width(240.0))
          .item(ListItem::new("theme.md").chevron(true).id("a"))
          .item(ListItem::new("blur.md").chevron(true).id("b"))
          .item(ListItem::new("canvas.md").chevron(true).id("c")),
      )
      .into()
  });
  let (_, top, width, height) = stage.bounds("list");
  assert_eq!(width, 240.0);
  assert_eq!(
    height,
    32.0 + 36.0 * 3.0,
    "a header of 32 and rows of 36, the line between each two inside the row over it: the board's 140"
  );
  assert!(stage.border("list").is_some(), "an edge round it");
  assert!(stage.paints("RECENT"), "the header in capitals");
  assert!(stage.paints("Clear"));
  assert_eq!(stage.text("RECENT"), (light(PaletteColor::MutedForeground), 11.0));
  assert!((stage.bounds("a").1 - (top + 32.0)).abs() < 1.0);
  assert_eq!(stage.text("theme.md").1, 13.0);
  assert_eq!(texts(&stage, CHEVRON_RIGHT).len(), 3);
  // A contained row's text is 8 from its chevron, where a row of a plain list has 12.
  let theme = Theme::new();
  let gap = |contained: bool| {
    ListItem::recipe()
      .resolve(&Case::new().state(ListItemState::Contained, contained), &theme)
      .part(ListItemPart::Root)
      .rest()
      .gap
      .map(|gap| gap.resolve(&theme.spacing(), 0.0))
  };
  assert_eq!(gap(true), Some(8.0));
  assert_eq!(gap(false), Some(12.0));
}

#[cfg(feature = "router")]
#[test]
fn a_row_with_a_path_goes_there_in_the_router() {
  use lurq::router::{RouterHandle, Routes};

  let held: Arc<Mutex<Option<RouterHandle>>> = Arc::default();
  let shared = held.clone();
  let mut stage = Stage::new(move |ctx| {
    let router = shared
      .lock()
      .unwrap()
      .get_or_insert_with(|| {
        let router = ctx.router(
          Routes::new()
            .route("/", |_| {
              List::new()
                .item(ListItem::new("About").href("/about").chevron(true).id("about"))
                .into()
            })
            .route("/about", |_| Text::new("about-page").into()),
        );
        router.push("/");
        router
      })
      .clone();
    Column::new().child(lurq::components::Router::mount(ctx, router)).into()
  });
  stage.click("about");
  assert!(stage.paints("about-page"));
  assert_eq!(held.lock().unwrap().as_ref().unwrap().path().get(), "/about");
}

#[test]
fn a_renderer_sees_what_the_row_was_given() {
  let seen = Arc::new(Mutex::new(None));
  let told = seen.clone();
  let renderer: ListItemRenderer = Arc::new(move |view: ListItemView| {
    *told.lock().unwrap() = Some((
      view.title.to_string(),
      view.value.clone().map(|v| v.to_string()),
      view.pressable(),
    ));
    Row::new().id("other").size(10.0, 10.0).into()
  });
  let app = App::new();
  app.theme().set_renderer::<ListItem>(renderer);
  let mut stage = Stage::in_app(app, |_| {
    Column::new()
      .child(ListItem::new("Wi-Fi").value("Home").on_click(|_| {}))
      .into()
  });
  assert_eq!(stage.bounds("other").2, 10.0);
  assert_eq!(
    *seen.lock().unwrap(),
    Some(("Wi-Fi".to_owned(), Some("Home".to_owned()), true))
  );
}

#[test]
fn a_rule_of_the_app_restyles_a_part() {
  let mut stage = stage_of(|| {
    List::new().item(
      ListItem::new("Wi-Fi")
        .id("row")
        .style(ListItemPart::Root, |s| s.min_height(48.0))
        .style(ListItemPart::Title, |s| s.font_size(18.0)),
    )
  });
  assert_eq!(stage.bounds("row").3, 48.0);
  assert_eq!(stage.text("Wi-Fi").1, 18.0);
}

/// A selected row tells tooling it is, a pressed one is a button, and one with a path a link.
#[cfg(feature = "mcp")]
#[test]
fn rows_tell_tooling_what_they_are() {
  let mut stage = stage_of(|| {
    List::new()
      .label("Settings")
      .id("list")
      .item(ListItem::new("Plain").id("plain"))
      .item(ListItem::new("Pressed").on_click(|_| {}).id("pressed"))
      .item(ListItem::new("Backups").checkbox(true).on_click(|_| {}).id("check"))
      .item(ListItem::new("Locked").disabled(true).on_click(|_| {}).id("locked"))
  });
  let mut said = |id: &str, name: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(name))
  };
  assert_eq!(said("list", "a11y_role").as_deref(), Some("list"));
  assert_eq!(said("list", "a11y_name").as_deref(), Some("Settings"));
  assert_eq!(said("plain", "a11y_role").as_deref(), Some("listitem"));
  assert_eq!(
    said("locked", "a11y_disabled").as_deref(),
    Some("true"),
    "a disabled row says so"
  );
  assert_eq!(said("pressed", "a11y_disabled"), None);
  assert_eq!(said("pressed", "a11y_role").as_deref(), Some("button"));
  assert_eq!(said("check", "a11y_role").as_deref(), Some("checkbox"));
  assert_eq!(said("check", "a11y_checked").as_deref(), Some("true"));
}

/// A row's `on_click` is given the press: what it was pressed with, and from where.
#[test]
fn a_rows_on_click_is_given_the_press() {
  let pressed: Arc<std::sync::Mutex<Vec<(bool, bool)>>> = Arc::default();
  let heard = pressed.clone();
  let mut stage = stage_of(move || {
    let heard = heard.clone();
    List::new().item(ListItem::new("Open").id("row").on_click(move |event: MouseEvent| {
      heard.lock().unwrap().push((event.shift, event.from_keyboard));
    }))
  });
  let (x, y) = stage.center("row");
  stage.tree.mouse_move(x, y);
  stage
    .tree
    .mouse_down_with_modifiers(x, y, MouseButton::Left, true, false, false);
  stage
    .tree
    .mouse_up_with_modifiers(x, y, MouseButton::Left, true, false, false);
  stage.pass();
  stage.key("Enter");
  assert_eq!(*pressed.lock().unwrap(), [(true, false), (false, true)]);
}

#[test]
fn in_a_pressed_row_the_control_at_its_end_keeps_its_own_presses_and_is_a_stop_of_tab_after_it() {
  let (rows, switches) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
  let (row, switch) = (rows.clone(), switches.clone());
  let mut stage = stage_of(move || {
    let (row, switch) = (row.clone(), switch.clone());
    List::new().item(
      ListItem::new("Dark mode")
        .id("row")
        .on_click(move |_| {
          row.fetch_add(1, Ordering::Relaxed);
        })
        .trailing(
          Switch::new("Dark mode")
            .label_hidden(true)
            .id("switch")
            .on_change(move |_| {
              switch.fetch_add(1, Ordering::Relaxed);
            }),
        ),
    )
  });
  stage.click("switch");
  assert_eq!(switches.load(Ordering::Relaxed), 1);
  assert_eq!(
    rows.load(Ordering::Relaxed),
    0,
    "a press of the switch is not the row's"
  );
  stage.click("row");
  assert_eq!(rows.load(Ordering::Relaxed), 1);
  assert_eq!(
    switches.load(Ordering::Relaxed),
    1,
    "nor a press of the row the switch's"
  );
}

#[test]
fn tab_stops_on_a_pressed_row_and_then_on_the_control_at_its_end() {
  let (rows, switches) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
  let (row, switch) = (rows.clone(), switches.clone());
  let mut stage = stage_of(move || {
    let (row, switch) = (row.clone(), switch.clone());
    List::new().item(
      ListItem::new("Dark mode")
        .id("row")
        .on_click(move |_| {
          row.fetch_add(1, Ordering::Relaxed);
        })
        .trailing(
          Switch::new("Dark mode")
            .label_hidden(true)
            .id("switch")
            .on_change(move |_| {
              switch.fetch_add(1, Ordering::Relaxed);
            }),
        ),
    )
  });
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("row"));
  stage.key("Enter");
  assert_eq!((rows.load(Ordering::Relaxed), switches.load(Ordering::Relaxed)), (1, 0));
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("switch"));
  stage.key(" ");
  assert_eq!(
    (rows.load(Ordering::Relaxed), switches.load(Ordering::Relaxed)),
    (1, 1),
    "a key on the switch is the switch's"
  );
}

#[test]
fn a_title_longer_than_its_room_is_cut_with_an_ellipsis_and_what_is_at_the_end_keeps_its_room() {
  let title = "A setting whose name goes on for far longer than the row it is in has room for";
  let mut stage = stage_of(move || List::new().item(ListItem::new(title).value("Home").chevron(true).id("row")));
  let (x, _, width, _) = stage.bounds("row");
  assert_eq!(width, 340.0);
  assert!(!stage.paints(title), "the title has no room for all of itself");
  let cut = stage.cut_text_quad(title).expect("it is cut with an ellipsis");
  let value = stage.text_quad("Home").expect("the value is whole");
  assert!(cut.x + cut.width <= value.x + 0.5, "the title ends before the value");
  let chevron = texts(&stage, CHEVRON_RIGHT).pop().expect("the chevron");
  assert!(chevron.x + chevron.width <= x + width - 12.0 + 0.5);
}

/// The shapes painted in the muted fill: a skeleton's bones.
fn bones(stage: &Stage) -> Vec<lurq::layout::quad::Quad> {
  let muted = light(PaletteColor::Muted);
  stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Rect { color, gradient: None } if *color == muted))
    .collect()
}

#[test]
fn a_row_shows_bones_where_its_title_and_description_are_still_read_and_moves_nothing_when_they_come() {
  let read = Signal::new(false);
  let held = read.clone();
  let mut stage = stage_of(move || {
    let done = held.get();
    List::new()
      .item(
        ListItem::new("Backups")
          .description("Every night at 2:00")
          .title_pending(!done)
          .description_pending(!done)
          .on_click(|_| {})
          .id("backups"),
      )
      .item(ListItem::new("Sync").description_pending(!done).id("sync"))
  });
  assert!(
    !stage.paints("Backups") && !stage.paints("Every night at 2:00"),
    "no words while they are read"
  );
  let shapes = bones(&stage);
  assert_eq!(shapes.len(), 3, "a bone for each word still read");
  let ids = stage.ids();
  for id in [
    "backups-title",
    "backups-title_skeleton",
    "backups-description",
    "backups-description_skeleton",
    "sync-title",
    "sync-description_skeleton",
  ] {
    assert!(ids.contains(&id.to_owned()), "{id} in {ids:?}");
  }
  let (_, _, title_width, title_height) = stage.bounds("backups-title_skeleton");
  assert_eq!((title_width, title_height), (140.0, 12.0), "the recipe's title bone");
  let (_, _, width, height) = stage.bounds("backups-description_skeleton");
  assert_eq!((width, height), (220.0, 10.0), "the recipe's description bone");
  let pending = (stage.bounds("backups"), stage.bounds("sync"));
  assert!(
    pending.1.3 >= 52.0,
    "a row whose description is read is drawn as two lines"
  );
  #[cfg(feature = "mcp")]
  {
    let mut said = |id: &str, name: &str| {
      stage
        .tree
        .get_element_by_id_mut(id)
        .and_then(|element| element.description(name))
    };
    assert_eq!(said("backups", "a11y_busy").as_deref(), Some("true"));
    assert_eq!(
      said("backups", "a11y_name").as_deref(),
      Some("Backups"),
      "still called by its title"
    );
    assert_eq!(
      said("backups", "a11y_description"),
      None,
      "no description told while it is read"
    );
  }
  read.set(true);
  stage.pass();
  assert!(
    stage.paints("Backups") && stage.paints("Every night at 2:00"),
    "the words in their place"
  );
  assert!(bones(&stage).is_empty());
  assert_eq!(stage.bounds("backups"), pending.0, "the row did not move or grow");
}

#[test]
fn a_row_that_says_something_below_is_held_with_it_in_the_item_part() {
  let mut stage = stage_of(|| {
    List::new()
      .item(
        ListItem::new("Certificate")
          .on_click(|_| {})
          .below(Row::new().size(80.0, 12.0).id("problem"))
          .id("cert"),
      )
      .item(ListItem::new("Plain").on_click(|_| {}).id("plain"))
  });
  let ids = stage.ids();
  assert!(ids.contains(&"cert-item".to_owned()), "{ids:?}");
  assert!(
    !ids.contains(&"plain-item".to_owned()),
    "a row with nothing below is the whole item: {ids:?}"
  );
  let (item, row, problem) = (stage.bounds("cert-item"), stage.bounds("cert"), stage.bounds("problem"));
  assert!(
    item.1 <= row.1 && item.1 + item.3 >= problem.1 + problem.3,
    "the item holds the row and what is below"
  );
  stage.key("Tab");
  assert_eq!(
    stage
      .tree
      .focused_element()
      .and_then(|element| element.id().map(str::to_owned))
      .as_deref(),
    Some("cert"),
    "the id of what is pressed is the row's"
  );
}

/// Whether `content` is painted whole: a text of it and nothing cut from it.
fn whole(stage: &Stage, content: &str) -> bool {
  stage
    .quads()
    .iter()
    .any(|quad| matches!(&quad.content, lurq::layout::quad::QuadContent::Text { text, .. } if text == content))
}

#[test]
fn a_cut_title_is_said_whole_in_a_tooltip_while_it_is_cut() {
  const TITLE: &str = "Rename the notes of every project that was moved last week";
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .spacing(8.0)
      .child(
        Column::new()
          .width(160.0)
          .child(ListItem::new(TITLE).id("cut").title_tooltip_when_cut(true)),
      )
      .child(
        Column::new()
          .width(360.0)
          .child(ListItem::new("Short").id("whole").title_tooltip_when_cut(true)),
      )
      .child(
        Column::new()
          .width(160.0)
          .child(ListItem::new(&TITLE.replace("notes", "files")).id("off")),
      )
      .into()
  });
  assert!(!whole(&stage, TITLE), "cut at rest");
  stage.own_timers();
  stage.hover("whole-title");
  stage.tick_timers(std::time::Duration::from_secs(2));
  stage.pass();
  assert_eq!(texts(&stage, "Short").len(), 1, "no tooltip over a title that fits");
  stage.hover("off-title");
  stage.tick_timers(std::time::Duration::from_secs(2));
  stage.pass();
  assert!(!whole(&stage, &TITLE.replace("notes", "files")), "none unless told");
  stage.hover("cut-title");
  stage.tick_timers(std::time::Duration::from_secs(4));
  stage.pass();
  assert!(whole(&stage, TITLE), "its tooltip says it whole");
}

#[test]
fn what_is_at_the_end_stands_at_the_top_of_a_row_that_grows_under_its_title_where_told() {
  let row = |at_top: bool, id: &'static str| {
    ListItem::new("GITHUB_TOKEN")
      .id(id)
      .description("Set 3 days ago")
      .detail(Column::new().height(80.0))
      .trailing_at_top(at_top)
      .trailing(Text::new("Replace").id(if at_top { "top-end" } else { "middle-end" }))
  };
  let mut stage = stage_of(move || List::new().item(row(true, "top")).item(row(false, "middle")));
  let (_, top_y, _, top_height) = stage.bounds("top");
  let (_, end_y, _, end_height) = stage.bounds("top-end");
  let (_, middle_y, _, middle_height) = stage.bounds("middle");
  let (_, centred_y, _, centred_height) = stage.bounds("middle-end");
  // Centred on its row by default; at the top of the row where told, by the title.
  assert!(
    ((centred_y + centred_height / 2.0) - (middle_y + middle_height / 2.0)).abs() < 1.0,
    "centred: {centred_y} in {middle_y} {middle_height}"
  );
  assert!(
    end_y < top_y + top_height / 2.0 - end_height,
    "at the top: {end_y} in {top_y} {top_height}"
  );
}

/// A row whose description wraps onto lines under its title, with a dot of 8 before its text aligned as `align` says.
fn dotted(id: &'static str, dot: &'static str, align: Option<LeadingAlign>) -> ListItem {
  let row = ListItem::new("Use the address from your folder")
    .id(id)
    .description(
      "The remote of the folder's repository, read from its configuration when the window opens, and kept until the \
       folder is moved or its remote is changed, long enough to wrap onto three lines",
    )
    .leading(Column::new().size(8.0, 8.0).id(dot));
  match align {
    Some(align) => row.leading_align(align),
    None => row,
  }
}

#[test]
fn what_is_before_the_text_stands_in_the_middle_at_the_top_or_on_the_title_line_as_told() {
  let mut stage = stage_of(|| {
    List::new()
      .item(dotted("middle", "middle-dot", None))
      .item(dotted("top", "top-dot", Some(LeadingAlign::Top)))
      .item(dotted("line", "line-dot", Some(LeadingAlign::FirstLine)))
  });
  let title = |stage: &mut Stage, id: &str| {
    let row = stage.bounds(id);
    let quads: Vec<Quad> = stage
      .quads()
      .into_iter()
      .filter(|quad| {
        matches!(&quad.content, QuadContent::Text { text, .. } if text == "Use the address from your folder")
          && quad.y >= row.1
          && quad.y < row.1 + row.3
      })
      .collect();
    let quad = quads.first().expect("the title is painted").clone();
    (quad.y, quad.height)
  };

  let (_, middle_y, _, middle_height) = stage.bounds("middle");
  let (_, dot_y, ..) = stage.bounds("middle-dot");
  assert!(middle_height > 70.0, "the description wraps: {middle_height}");
  assert!(
    ((dot_y + 4.0) - (middle_y + middle_height / 2.0)).abs() < 1.5,
    "in the middle by default: {dot_y} in {middle_y} {middle_height}"
  );

  let (title_y, _) = title(&mut stage, "top");
  let (_, dot_y, ..) = stage.bounds("top-dot");
  assert!(
    (dot_y - title_y).abs() < 1.5,
    "at the title's top: {dot_y} by {title_y}"
  );

  let (title_y, title_height) = title(&mut stage, "line");
  let (_, dot_y, ..) = stage.bounds("line-dot");
  assert!(
    ((dot_y + 4.0) - (title_y + title_height / 2.0)).abs() < 1.5,
    "in the middle of the title's line: {dot_y} by {title_y} {title_height}"
  );
}

#[test]
fn what_is_before_the_title_on_its_line_has_what_is_under_it_start_under_it() {
  let mut stage = stage_of(|| {
    List::new()
      .item(
        ListItem::new("Use the address from your folder")
          .id("title-dot")
          .title_leading(Column::new().size(8.0, 8.0).id("dot"))
          .subtitle(Text::new("git@github.com:team/kontur.git").id("path")),
      )
      .item(
        ListItem::new("Use the address from your folder")
          .id("leading-dot")
          .leading(Column::new().size(8.0, 8.0).id("lead"))
          .subtitle(Text::new("git@github.com:team/kontur.git").id("under-title")),
      )
  });
  let (dot_x, dot_y, dot_width, _) = stage.bounds("dot");
  let (path_x, path_y, ..) = stage.bounds("path");
  assert!(
    (path_x - dot_x).abs() < 0.5,
    "the subtitle starts under the dot: {path_x} {dot_x}"
  );
  assert!(path_y > dot_y, "under it");
  // The title is 8 after the dot, on its line.
  let title = stage
    .quads()
    .into_iter()
    .filter(
      |quad| matches!(&quad.content, QuadContent::Text { text, .. } if text == "Use the address from your folder"),
    )
    .map(|quad| (quad.x, quad.y, quad.height))
    .next()
    .expect("the title");
  assert!(
    (title.0 - (dot_x + dot_width + 8.0)).abs() < 0.5,
    "8 after the dot: {title:?}"
  );
  assert!(
    dot_y > title.1 && dot_y + 8.0 < title.1 + title.2 + 0.5,
    "on its line: {dot_y} {title:?}"
  );

  // Where the dot leads the row, what is under the title starts under the title.
  let (lead_x, _, lead_width, _) = stage.bounds("lead");
  let (under_x, ..) = stage.bounds("under-title");
  assert!(
    under_x >= lead_x + lead_width + 8.0,
    "under the title: {under_x} {lead_x}"
  );
}
