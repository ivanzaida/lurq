//! What a menu item says besides its label: a detail under it, why it cannot
//! be chosen (with a lock), an element of the app's before it; and a menu and
//! a popover kept inside an element, not only inside the window.

use std::sync::{Arc, Mutex};

use lurq::{
  app::theme::{PaletteColor, ThemePalette},
  components::{Column, Rect, Row},
  core::{ElementRef, Signal},
  layout::quad::QuadContent,
  node::{Element, color::Color},
  ui::{Menu, MenuItem, Popover, icons},
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

/// A page with an anchor of 120 by 30 at 20, 20 and `menu` hung from it.
fn page(anchor: ElementRef, menu: Menu) -> Element {
  Column::new()
    .size(400.0, 300.0)
    .padding(20.0)
    .child(Rect::new(120.0, 30.0).id("anchor").ref_element(anchor))
    .child(menu)
    .into()
}

fn open_menu(items: impl Fn() -> Vec<MenuItem> + Send + Sync + 'static) -> Stage {
  let anchor = ElementRef::new();
  Stage::new(move |_| {
    page(
      anchor.clone(),
      Menu::controlled(true, anchor.clone(), |_| {}).id("menu").items(items()),
    )
  })
}

#[test]
fn a_detail_is_a_line_under_the_label_and_the_item_grows_to_hold_it() {
  let mut stage = open_menu(|| {
    vec![
      MenuItem::new("Builder").detail("gpt-5 · high").id("builder"),
      MenuItem::new("Reviewer").id("reviewer"),
    ]
  });
  let label = stage.text_quad("Builder").expect("the label shows");
  let detail = stage.text_quad("gpt-5 · high").expect("the detail shows");
  assert!(
    detail.y >= label.y + label.height + 1.5,
    "2 under the label: {:?} under {:?}",
    (detail.x, detail.y),
    (label.y, label.height)
  );
  assert!((detail.x - label.x).abs() < 0.5, "at the label's start");
  let (color, size) = stage.text("gpt-5 · high");
  assert_eq!(color, light(PaletteColor::MutedForeground));
  assert_eq!(size, 12.0);
  let (_, _, _, tall) = stage.bounds("builder");
  let (_, _, _, plain) = stage.bounds("reviewer");
  assert_eq!(plain, 32.0);
  assert!(tall > 40.0, "as high as its two lines and 6 above and below: {tall}");
  assert!(
    stage.ids().iter().any(|id| id == "builder-detail"),
    "the detail is a part"
  );
}

#[test]
fn an_unavailable_item_says_why_ends_in_a_lock_and_cannot_be_chosen() {
  let chosen = Arc::new(Mutex::new(0));
  let told = chosen.clone();
  let mut stage = open_menu(move || {
    let told = told.clone();
    vec![
      MenuItem::new("Merge")
        .unavailable("The run has not finished")
        .on_select(move || *told.lock().unwrap() += 1)
        .id("merge"),
    ]
  });
  assert!(stage.paints("The run has not finished"), "the reason under the label");
  assert!(stage.paints(&glyph(icons::LOCK)), "a lock at its end");
  assert!(stage.ids().iter().any(|id| id == "merge-lock"));
  stage.click("merge");
  assert_eq!(*chosen.lock().unwrap(), 0, "it is not chosen");
}

#[cfg(any(feature = "devtools", feature = "mcp"))]
#[test]
fn an_unavailable_items_reason_is_what_describes_it() {
  let mut stage = open_menu(|| {
    vec![
      MenuItem::new("Merge")
        .unavailable("The run has not finished")
        .id("merge"),
    ]
  });
  let described = stage
    .tree
    .get_element_by_id_mut("merge")
    .and_then(|element| element.description("a11y_description"));
  assert_eq!(described.as_deref(), Some("The run has not finished"));
}

#[test]
fn an_items_own_detail_stays_and_its_reason_is_still_read() {
  let stage = open_menu(|| {
    vec![
      MenuItem::new("Builder")
        .detail("gpt-5 · high")
        .unavailable("Not signed in")
        .id("builder"),
    ]
  });
  assert!(stage.paints("gpt-5 · high"));
  assert!(!stage.paints("Not signed in"), "its own detail is what it says");
  assert!(stage.paints(&glyph(icons::LOCK)));
}

#[test]
fn a_leading_element_stands_before_the_label_in_place_of_the_icon() {
  let mut stage = open_menu(|| {
    vec![
      MenuItem::new("Ada Lovelace")
        .icon(icons::USER)
        .leading(Rect::new(20.0, 20.0).id("avatar"))
        .id("ada"),
    ]
  });
  let (ax, _, aw, _) = stage.bounds("avatar");
  let label = stage.text_quad("Ada Lovelace").unwrap();
  assert!(ax + aw <= label.x, "before the label");
  assert!(!stage.paints(&glyph(icons::USER)), "in place of the icon");
  assert!(stage.ids().iter().any(|id| id == "ada-leading"));
}

/// A frame 300 wide at 20, 20 with an anchor at its 180, and what `overlay` hangs from it, kept inside the frame
/// or not.
fn framed(overlay: impl Fn(ElementRef, ElementRef) -> Element + Send + Sync + 'static) -> Stage {
  let (frame, anchor) = (ElementRef::new(), ElementRef::new());
  Stage::new(move |_| {
    Column::new()
      .size(400.0, 300.0)
      .padding(20.0)
      .child(
        Row::new()
          .width(300.0)
          .height(260.0)
          .id("frame")
          .ref_element(frame.clone())
          .padding_left(180.0)
          .child(Rect::new(40.0, 30.0).id("anchor").ref_element(anchor.clone()))
          .child(overlay(anchor.clone(), frame.clone())),
      )
      .into()
  })
}

#[test]
fn a_menu_within_an_element_is_moved_into_its_box() {
  let mut inside = framed(|anchor, frame| {
    Menu::controlled(true, anchor, |_| {})
      .id("menu")
      .within(frame)
      .item(MenuItem::new("Open"))
      .into()
  });
  let mut loose = framed(|anchor, _| {
    Menu::controlled(true, anchor, |_| {})
      .id("menu")
      .item(MenuItem::new("Open"))
      .into()
  });
  let (fx, _, fw, _) = inside.bounds("frame");
  let (x, _, width, _) = inside.bounds("menu");
  assert!(x + width <= fx + fw + 0.5, "its end inside the frame: {x} + {width}");
  let (loose_x, ..) = loose.bounds("menu");
  let (anchor_x, ..) = loose.bounds("anchor");
  assert_eq!(loose_x, anchor_x, "without it, at the anchor's start");
}

#[test]
fn a_popover_within_an_element_is_moved_into_its_box() {
  let mut stage = framed(|anchor, frame| {
    Popover::new(Signal::new(true), anchor)
      .id("popover")
      .width(200.0)
      .within(frame)
      .title("Sources")
      .into()
  });
  let (fx, _, fw, _) = stage.bounds("frame");
  let (x, _, width, _) = stage.bounds("popover");
  assert_eq!(width, 200.0);
  assert!((x + width - (fx + fw)).abs() < 0.5, "its end at the frame's: {x}");
}

#[test]
fn an_item_without_a_detail_or_a_reason_is_as_it_was() {
  let mut stage = open_menu(|| vec![MenuItem::new("Open").id("open")]);
  assert_eq!(stage.bounds("open").3, 32.0);
  assert!(
    !stage
      .quads()
      .iter()
      .any(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if *text == glyph(icons::LOCK)))
  );
}
