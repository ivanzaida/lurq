//! A speed menu: the menu's panel 168 wide under its anchor, the heading and the speeds with the one in use checked
//! at the end of its row, what a choice tells and what it does not, the keys, what closes it, what it is to tooling,
//! and a theme's rule.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    events::MouseButton,
    theme::{PaletteColor, ThemePalette},
  },
  components::{Column, Rect},
  core::{ElementRef, Signal},
  node::{Element, color::Color},
  ui::{MediaSpeedMenu, MediaSpeedMenuPart, icons},
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

/// A page of 400 by 360 with an anchor of 120 by 30 at 20, 20 and `menu` hung from it.
fn page(anchor: ElementRef, menu: MediaSpeedMenu) -> Element {
  Column::new()
    .size(400.0, 360.0)
    .padding(20.0)
    .child(Rect::new(120.0, 30.0).id("anchor").ref_element(anchor))
    .child(menu)
    .into()
}

/// What the menu told, in order, as lines.
fn log() -> (Arc<Mutex<Vec<String>>>, Arc<Mutex<Vec<String>>>) {
  let told = Arc::new(Mutex::new(Vec::new()));
  (told.clone(), told)
}

/// A press of the text `content` where it is painted.
fn press_text(stage: &mut Stage, content: &str) {
  let quad = stage
    .text_quad(content)
    .unwrap_or_else(|| panic!("{content:?} is painted"));
  let (x, y) = (quad.x + quad.width / 2.0, quad.y + quad.height / 2.0);
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass();
}

/// The speed menu, open, with the speed `rate` in use, telling `told` what it hears.
fn menu(open: &Signal<bool>, anchor: &ElementRef, rate: f32, told: &Arc<Mutex<Vec<String>>>) -> MediaSpeedMenu {
  let (change, dismiss, open_change) = (told.clone(), told.clone(), told.clone());
  MediaSpeedMenu::new(open.clone(), anchor.clone(), rate)
    .id("speed")
    .on_change(move |rate| change.lock().unwrap().push(format!("on_change({rate})")))
    .on_dismiss(move || dismiss.lock().unwrap().push("on_dismiss".to_owned()))
    .on_open_change(move |open| open_change.lock().unwrap().push(format!("on_open_change({open})")))
}

#[test]
fn the_speed_menu_is_a_menu_168_wide_with_the_heading_and_the_speed_in_use_checked() {
  let (anchor, open) = (ElementRef::new(), Signal::new(true));
  let (_, told) = log();
  let mut stage = Stage::new(move |_| page(anchor.clone(), menu(&open, &anchor, 1.0, &told)));
  let (x, y, width, _) = stage.bounds("speed");
  assert_eq!(width, 168.0);
  let (ax, ay, _, ah) = stage.bounds("anchor");
  assert_eq!((x, y), (ax, ay + ah + 4.0), "under its anchor, from its start, 4 away");
  assert_eq!(stage.fill_color("speed"), Some(light(PaletteColor::Popover)));

  // The heading is 11 in the muted foreground; the speeds are in the body's type, in the board's order.
  assert_eq!(
    stage.text("Playback speed"),
    (light(PaletteColor::MutedForeground), 11.0)
  );
  let speeds = ["0.5×", "0.75×", "Normal", "1.25×", "1.5×", "2×"];
  let mut last = y;
  for speed in speeds {
    let quad = stage.text_quad(speed).unwrap_or_else(|| panic!("{speed} is painted"));
    assert!(quad.y > last, "{speed} comes after the one before");
    last = quad.y;
  }
  // One check, at the end of the row of the speed in use: 8 in from the item's end, 4 more from the panel's.
  let check = stage.text_quad(&glyph(icons::CHECK)).expect("a check");
  let normal = stage.text_quad("Normal").unwrap();
  assert!(
    (check.y + check.height / 2.0 - (normal.y + normal.height / 2.0)).abs() < 2.0,
    "on Normal's row"
  );
  assert!(
    (check.x + check.width - (x + width - 12.0)).abs() < 0.51,
    "at the end of the row"
  );
  let checks = stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, lurq::layout::quad::QuadContent::Text { text, .. } if *text == glyph(icons::CHECK)))
    .count();
  assert_eq!(checks, 1);
}

/// It is a menu to tooling, called by its heading or by its label.
#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn it_is_a_menu_called_by_its_heading_or_its_label() {
  let (anchor, open) = (ElementRef::new(), Signal::new(true));
  let (_, told) = log();
  let (first, second) = (anchor.clone(), open.clone());
  let mut stage = Stage::new(move |_| page(first.clone(), menu(&second, &first, 1.0, &told)));
  let mut said = |stage: &mut Stage, what: &str| {
    stage
      .tree
      .get_element_by_id_mut("speed")
      .and_then(|element| element.description(what))
  };
  assert_eq!(said(&mut stage, "a11y_role").as_deref(), Some("menu"));
  assert_eq!(said(&mut stage, "a11y_name").as_deref(), Some("Playback speed"));
  let mut named = Stage::new(move |_| {
    page(
      anchor.clone(),
      MediaSpeedMenu::new(open.clone(), anchor.clone(), 1.0)
        .label("Speed of the lecture")
        .id("speed"),
    )
  });
  assert_eq!(said(&mut named, "a11y_name").as_deref(), Some("Speed of the lecture"));
}

#[test]
fn choosing_another_speed_tells_it_and_the_speed_in_use_only_closes() {
  let (anchor, open) = (ElementRef::new(), Signal::new(true));
  let (told, tell) = log();
  let (held, at) = (open.clone(), anchor.clone());
  let mut stage = Stage::new(move |_| page(at.clone(), menu(&held, &at, 1.0, &tell)));
  press_text(&mut stage, "1.5×");
  assert!(!open.get(), "closed");
  assert_eq!(*told.lock().unwrap(), ["on_open_change(false)", "on_change(1.5)"]);

  told.lock().unwrap().clear();
  open.set(true);
  stage.pass();
  press_text(&mut stage, "Normal");
  assert!(!open.get(), "closed");
  assert_eq!(*told.lock().unwrap(), ["on_open_change(false)"], "nothing changed");
}

#[test]
fn the_arrows_move_between_the_speeds_and_enter_chooses() {
  let (anchor, open) = (ElementRef::new(), Signal::new(true));
  let (told, tell) = log();
  let (held, at) = (open.clone(), anchor.clone());
  let mut stage = Stage::new(move |_| page(at.clone(), menu(&held, &at, 1.0, &tell)));
  // The menu put the focus on its first speed as it opened; Down goes to the next, Enter chooses it.
  stage.key("Home");
  stage.key("ArrowDown");
  stage.key("Enter");
  assert!(!open.get());
  assert_eq!(told.lock().unwrap().last().map(String::as_str), Some("on_change(0.75)"));
}

#[test]
fn escape_dismisses_it_and_tells_no_speed() {
  let (anchor, open) = (ElementRef::new(), Signal::new(true));
  let (told, tell) = log();
  let (held, at) = (open.clone(), anchor.clone());
  let mut stage = Stage::new(move |_| page(at.clone(), menu(&held, &at, 1.5, &tell)));
  assert!(stage.paints("Normal"));
  stage.key("Escape");
  assert!(!open.get());
  assert_eq!(*told.lock().unwrap(), ["on_dismiss", "on_open_change(false)"]);
  stage.pass();
  assert!(!stage.paints("Normal"), "closed, it paints nothing");
}

#[test]
fn its_own_speeds_and_heading_and_a_rule_over_the_recipe() {
  let (anchor, open) = (ElementRef::new(), Signal::new(true));
  let mut stage = Stage::new(move |_| {
    page(
      anchor.clone(),
      MediaSpeedMenu::new(open.clone(), anchor.clone(), 0.8)
        .rates(&[0.8, 1.0, 1.2])
        .heading_text("Speed")
        .style(MediaSpeedMenuPart::Panel, |s| {
          s.width(200.0).min_width(200.0).max_width(200.0)
        })
        .id("speed"),
    )
  });
  assert_eq!(stage.bounds("speed").2, 200.0);
  for text in ["Speed", "0.8×", "Normal", "1.2×"] {
    assert!(stage.paints(text), "{text}");
  }
  assert!(!stage.paints("0.5×"));
}
