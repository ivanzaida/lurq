//! A menu whose rows are the app's own elements (`MenuItem::content`), the app's elements among its entries
//! (`Menu::child`), and menus and popovers placed by another element than their anchor or at a point.

use std::sync::{Arc, Mutex};

use lurq::{
  app::theme::{PaletteColor, ThemePalette},
  components::{Column, Placement, Rect, Row, Text},
  core::{ElementRef, Signal},
  node::{Element, color::Color},
  ui::{Menu, MenuItem, Popover},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// What was chosen, in order, and a handler for each item that says so.
fn log() -> (
  Arc<Mutex<Vec<&'static str>>>,
  impl Fn(&'static str) -> Box<dyn Fn() + Send + Sync>,
) {
  let chosen = Arc::new(Mutex::new(Vec::new()));
  let told = chosen.clone();
  (chosen, move |name| {
    let told = told.clone();
    Box::new(move || told.lock().unwrap().push(name))
  })
}

/// A row of the app's: its label and, after it on the same line, a detail; 40 high.
fn row(label: &str, detail: &str) -> Element {
  Row::new()
    .height(40.0)
    .spacing(8.0)
    .child(Text::new(label))
    .child(Text::new(detail))
    .into()
}

/// A page of 400 by 300 with an anchor of 120 by 30 at 20, 20 and `menu` hung from it.
fn page(anchor: ElementRef, menu: Menu) -> Element {
  Column::new()
    .size(400.0, 300.0)
    .padding(20.0)
    .child(Rect::new(120.0, 30.0).id("anchor").ref_element(anchor))
    .child(menu)
    .into()
}

/// A menu of three rows of the app's, the second unavailable, and a foot of the app's under them.
fn app_rows(open: &Signal<bool>, tell: &impl Fn(&'static str) -> Box<dyn Fn() + Send + Sync>) -> Stage {
  let (anchor, held) = (ElementRef::new(), open.clone());
  let (opus, sonnet, haiku) = (tell("opus"), tell("sonnet"), tell("haiku"));
  let (opus, sonnet, haiku): (
    Arc<dyn Fn() + Send + Sync>,
    Arc<dyn Fn() + Send + Sync>,
    Arc<dyn Fn() + Send + Sync>,
  ) = (opus.into(), sonnet.into(), haiku.into());
  Stage::new(move |_| {
    let (opus, sonnet, haiku) = (opus.clone(), sonnet.clone(), haiku.clone());
    page(
      anchor.clone(),
      Menu::new(held.clone(), anchor.clone())
        .id("models")
        .item(
          MenuItem::new("Opus")
            .content(row("Opus", "the deepest"))
            .checked(true)
            .id("opus")
            .on_select(move || opus()),
        )
        .item(
          MenuItem::new("Sonnet")
            .content(row("Sonnet", "balanced"))
            .unavailable("Not on this plan")
            .id("sonnet")
            .on_select(move || sonnet()),
        )
        .item(
          MenuItem::new("Haiku")
            .content(row("Haiku", "the fastest"))
            .id("haiku")
            .on_select(move || haiku()),
        )
        .separator()
        .child(Text::new("Sonnet is not on this plan.").id("foot")),
    )
  })
}

#[test]
fn an_items_content_is_all_it_shows_and_it_is_as_high_as_that() {
  let (_, tell) = log();
  let mut stage = app_rows(&Signal::new(true), &tell);
  assert!(stage.paints("the deepest"), "the app's row");
  assert!(stage.paints("Sonnet is not on this plan."), "the app's foot");
  let (_, _, _, height) = stage.bounds("opus");
  assert_eq!(height, 40.0, "as high as the app's row, past the row's 32");
  // The item's own label is not drawn beside the app's: "Opus" is painted once.
  let painted = stage
    .quads()
    .iter()
    .filter(|quad| matches!(&quad.content, lurq::layout::quad::QuadContent::Text { text, .. } if text == "Opus"))
    .count();
  assert_eq!(painted, 1);
  // The foot is under the rows.
  let (_, haiku_y, ..) = stage.bounds("haiku");
  let (_, foot_y, ..) = stage.bounds("foot");
  assert!(foot_y > haiku_y);
}

#[test]
fn an_items_content_is_still_the_menus_item_to_the_keyboard_and_the_pointer() {
  let (chosen, tell) = log();
  let open = Signal::new(true);
  let mut stage = app_rows(&open, &tell);
  // The arrows pass the unavailable row by, and the foot that takes no focus.
  stage.key("Home");
  assert_eq!(stage.fill_color("opus"), Some(light(PaletteColor::Highlight)));
  stage.key("ArrowDown");
  assert_eq!(stage.fill_color("haiku"), Some(light(PaletteColor::Highlight)));
  stage.key("ArrowDown");
  assert_eq!(
    stage.fill_color("opus"),
    Some(light(PaletteColor::Highlight)),
    "round to the first"
  );
  // Typing a letter goes to the item whose label starts with it.
  stage.key("h");
  assert_eq!(stage.fill_color("haiku"), Some(light(PaletteColor::Highlight)));

  // A press of the unavailable row does nothing; Enter on Haiku chooses it and closes the menu.
  stage.click("sonnet");
  assert!(open.get());
  assert!(chosen.lock().unwrap().is_empty());
  stage.key("Enter");
  assert_eq!(*chosen.lock().unwrap(), ["haiku"]);
  assert!(!open.get());
}

#[cfg(any(feature = "devtools", feature = "mcp"))]
#[test]
fn an_items_content_says_what_the_item_is_to_tooling() {
  let (_, tell) = log();
  let mut stage = app_rows(&Signal::new(true), &tell);
  let said = |stage: &mut Stage, id: &str, name: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(name))
  };
  assert_eq!(
    said(&mut stage, "opus", "a11y_role").as_deref(),
    Some("menuitemcheckbox")
  );
  assert_eq!(said(&mut stage, "opus", "a11y_name").as_deref(), Some("Opus"));
  assert_eq!(said(&mut stage, "opus", "a11y_checked").as_deref(), Some("true"));
  assert_eq!(
    said(&mut stage, "sonnet", "a11y_description").as_deref(),
    Some("Not on this plan")
  );
}

#[test]
fn a_menu_placed_by_another_element_stands_by_its_box_and_is_opened_by_its_anchor() {
  let (held_anchor, held_frame) = (ElementRef::new(), ElementRef::new());
  let mut stage = Stage::new(move |_| {
    Column::new()
      .size(400.0, 300.0)
      .padding(20.0)
      .child(
        Row::new()
          .id("frame")
          .ref_element(held_frame.clone())
          .width(300.0)
          .height(40.0)
          .child(Rect::new(60.0, 30.0).id("anchor").ref_element(held_anchor.clone())),
      )
      .child(
        Menu::controlled(true, held_anchor.clone(), |_| {})
          .id("menu")
          .placement(Placement::BottomEnd)
          .placed_by(held_frame.clone())
          .item(MenuItem::new("Rename")),
      )
      .into()
  });
  let (fx, fy, fw, fh) = stage.bounds("frame");
  let (x, y, width, _) = stage.bounds("menu");
  assert_eq!(
    (x + width, y),
    (fx + fw, fy + fh + 4.0),
    "under the frame, at its end, 4 below"
  );
}

#[test]
fn a_menu_and_a_popover_stand_at_a_point_of_the_window() {
  let anchor = ElementRef::new();
  let held = anchor.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .size(400.0, 300.0)
      .padding(20.0)
      .child(Rect::new(60.0, 30.0).id("anchor").ref_element(held.clone()))
      .child(
        Menu::controlled(true, held.clone(), |_| {})
          .id("menu")
          .at(Some((150.0, 100.0)))
          .item(MenuItem::new("Rename")),
      )
      .child(
        Popover::new(Signal::new(true), held.clone())
          .id("popover")
          .offset(0.0)
          .at(Some((40.0, 200.0)))
          .title("Details"),
      )
      .into()
  });
  let (x, y, ..) = stage.bounds("menu");
  assert_eq!((x, y), (150.0, 100.0), "its corner at the point");
  let (px, py, _, ph) = stage.bounds("popover");
  // The window has no room under 200 for the popover: it opens up from the point.
  assert_eq!(px, 40.0);
  assert!(py == 200.0 || (py + ph - 200.0).abs() < 0.5, "at the point: {py}, {ph}");
}

#[test]
fn a_popover_placed_by_a_bar_stands_over_it_at_its_end() {
  let (held_anchor, held_bar) = (ElementRef::new(), ElementRef::new());
  let mut stage = Stage::new(move |_| {
    Column::new()
      .size(400.0, 300.0)
      .padding_top(240.0)
      .child(
        Row::new()
          .id("bar")
          .ref_element(held_bar.clone())
          .width(400.0)
          .height(24.0)
          .padding_left(40.0)
          .child(Rect::new(60.0, 24.0).id("item").ref_element(held_anchor.clone())),
      )
      .child(
        Popover::new(Signal::new(true), held_anchor.clone())
          .id("popover")
          .placement(Placement::TopEnd)
          .placed_by(held_bar.clone())
          .offset(8.0)
          .title("MCP server"),
      )
      .into()
  });
  let (bx, by, bw, _) = stage.bounds("bar");
  let (x, y, width, height) = stage.bounds("popover");
  assert_eq!(
    (x + width, y + height),
    (bx + bw, by - 8.0),
    "over the bar, 8 above it, at its end"
  );
}
