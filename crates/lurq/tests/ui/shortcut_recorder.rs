//! A shortcut recorder: its box and keys as the board draws them, recording by a press and by Enter, the modifiers
//! held, a combination taken, refused for having no modifier or by the app, Escape, Backspace and the cross, the keys
//! of the window and of the app held back while it records, read only, disabled, a field, what tooling reads, and a
//! theme's rules.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    Accelerator, App,
    ctx::Ctx,
    events::KeyboardEvent,
    theme::{PaletteColor, ThemePalette},
  },
  components::{Button, Column, Row},
  core::Signal,
  layout::quad::QuadContent,
  node::{Element, color::Color},
  ui::{KeyPlatform, ShortcutRecorder, ShortcutRecorderPart, ShortcutRecorderSize, ShortcutValue},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn shortcut(text: &str) -> Accelerator {
  Accelerator::parse(text).unwrap()
}

/// A window of 400 by 300 with the recorder `make` builds, mounted, a button after it, and a handler of the app's
/// own on every key (`app`).
fn scene<V: ShortcutValue>(
  make: impl Fn() -> ShortcutRecorder<V> + Send + Sync + 'static,
  app_keys: &Arc<Mutex<Vec<String>>>,
) -> Stage {
  let heard = app_keys.clone();
  let mut stage = Stage::new(move |ctx: &mut Ctx| -> Element {
    let heard = heard.clone();
    Column::new()
      .size(400.0, 300.0)
      .padding(20.0)
      .spacing(12.0)
      .on_key_down(move |event: KeyboardEvent| heard.lock().unwrap().push(event.key.clone()))
      .child(make().platform(KeyPlatform::Standard).id("recorder").mount(ctx))
      .child(Row::new().child(Button::new("After").id("after").tab_index(0)))
      .into()
  });
  stage.tree.resize(400, 300);
  stage.pass();
  stage
}

fn chord(stage: &mut Stage, key: &str, code: &str, shift: bool, ctrl: bool) {
  stage.tree.key_down(key.to_owned(), code.to_owned(), shift, ctrl, false);
  stage.tree.key_up(key.to_owned(), code.to_owned(), shift, ctrl, false);
  stage.pass();
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

#[test]
fn at_rest_it_is_the_boards_box_with_its_keys_and_its_cross() {
  let keys = Arc::new(Mutex::new(Vec::new()));
  let mut stage = scene(
    || ShortcutRecorder::controlled_optional(Some(shortcut("Ctrl+Shift+P")), |_| {}),
    &keys,
  );
  let (_, _, width, height) = stage.bounds("recorder");
  assert_eq!((width, height), (230.0, 32.0));
  assert_eq!(stage.fill_color("recorder"), Some(light(PaletteColor::Background)));
  for key in ["Ctrl", "Shift", "P"] {
    assert!(stage.paints(key), "{key}");
  }
  // The keys 8 in from the box's start.
  let (x, ..) = stage.bounds("recorder");
  let ctrl = stage.text_quad("Ctrl").unwrap();
  assert!(ctrl.x - x > 8.0 && ctrl.x - x < 16.0, "{}", ctrl.x - x);
  // The cross at the end, in the muted foreground.
  let cross = lurq::ui::icons::ALL
    .iter()
    .find_map(|(name, glyph)| (*name == lurq::ui::icons::CLOSE).then(|| glyph.to_string()))
    .unwrap();
  assert_eq!(stage.text(&cross).0, light(PaletteColor::MutedForeground));
  assert!(!stage.paints("Record shortcut"));
}

#[test]
fn a_press_records_shows_the_modifiers_held_and_takes_the_combination() {
  let held = Signal::new(Some(shortcut("Ctrl+K")));
  let heard = Arc::new(Mutex::new(Vec::new()));
  let keys = Arc::new(Mutex::new(Vec::new()));
  let (read, told) = (held.clone(), heard.clone());
  let mut stage = scene(
    move || {
      let (seen, told) = (read.clone(), told.clone());
      ShortcutRecorder::optional(read.clone()).on_change(move |value: &Option<Accelerator>| {
        told.lock().unwrap().push((
          value.as_ref().map(ToString::to_string),
          seen.get_untracked().map(|s| s.to_string()),
        ))
      })
    },
    &keys,
  );
  stage.click("recorder");
  assert!(stage.paints("Press shortcut…"), "recording");
  assert!(stage.has_ring(light(PaletteColor::Ring)), "a line of the ring's color");
  // The modifiers as they are held.
  stage
    .tree
    .key_down("Control".to_owned(), "ControlLeft".to_owned(), false, true, false);
  stage.pass();
  assert!(stage.paints("Ctrl") && !stage.paints("Press shortcut…"));
  stage
    .tree
    .key_down("Shift".to_owned(), "ShiftLeft".to_owned(), true, true, false);
  stage.pass();
  assert!(stage.paints("Shift"));
  stage
    .tree
    .key_down("P".to_owned(), "KeyP".to_owned(), true, true, false);
  stage.pass();
  assert_eq!(held.get(), Some(shortcut("Ctrl+Shift+P")));
  assert_eq!(
    *heard.lock().unwrap(),
    [(Some("Ctrl+Shift+P".to_owned()), Some("Ctrl+Shift+P".to_owned()))],
    "told after the signal is set"
  );
  assert!(!stage.paints("Press shortcut…"), "no longer recording");
  assert!(
    keys.lock().unwrap().is_empty(),
    "the app's own handler heard none of it"
  );
  // The app setting the signal tells nobody.
  held.set(None);
  stage.pass();
  assert_eq!(heard.lock().unwrap().len(), 1);
  assert!(stage.paints("Record shortcut"), "the placeholder");
}

#[test]
fn while_it_records_tab_and_the_apps_keys_are_its_own() {
  let held = Signal::new(Some(shortcut("Ctrl+K")));
  let keys = Arc::new(Mutex::new(Vec::new()));
  let read = held.clone();
  let mut stage = scene(move || ShortcutRecorder::optional(read.clone()), &keys);
  // From the keyboard: Tab to it, Enter records.
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("recorder"));
  keys.lock().unwrap().clear();
  stage.key("Enter");
  assert!(stage.paints("Press shortcut…"));
  chord(&mut stage, "Tab", "Tab", false, true);
  assert_eq!(
    held.get(),
    Some(shortcut("Ctrl+Tab")),
    "Ctrl+Tab recorded, not followed"
  );
  assert_eq!(focused(&stage).as_deref(), Some("recorder"));
  assert_eq!(
    *keys.lock().unwrap(),
    ["Enter"],
    "the app heard the Enter that started it, and nothing after"
  );
}

#[test]
fn escape_gives_up_and_backspace_and_the_cross_empty_it() {
  let held = Signal::new(Some(shortcut("Ctrl+K")));
  let keys = Arc::new(Mutex::new(Vec::new()));
  let read = held.clone();
  let mut stage = scene(move || ShortcutRecorder::optional(read.clone()), &keys);
  stage.click("recorder");
  stage.key("Escape");
  assert!(!stage.paints("Press shortcut…"));
  assert_eq!(held.get(), Some(shortcut("Ctrl+K")), "kept");
  // Backspace while it records empties it.
  stage.click("recorder");
  stage.key("Backspace");
  assert_eq!(held.get(), None);
  // The cross empties it too.
  held.set(Some(shortcut("Ctrl+K")));
  stage.pass();
  let cross = lurq::ui::icons::ALL
    .iter()
    .find_map(|(name, glyph)| (*name == lurq::ui::icons::CLOSE).then(|| glyph.to_string()))
    .unwrap();
  let quad = stage.text_quad(&cross).unwrap();
  let (x, y) = (quad.x + quad.width / 2.0, quad.y + quad.height / 2.0);
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, lurq::app::events::MouseButton::Left);
  stage.tree.mouse_up(x, y, lurq::app::events::MouseButton::Left);
  stage.pass();
  assert_eq!(held.get(), None);
}

#[test]
fn a_combination_with_no_modifier_or_one_the_app_refuses_is_not_taken_and_says_why() {
  let held = Signal::new(shortcut("Ctrl+K"));
  let keys = Arc::new(Mutex::new(Vec::new()));
  let read = held.clone();
  let mut stage = scene(
    move || {
      ShortcutRecorder::new(read.clone()).validate(|shortcut| match shortcut.to_string().as_str() {
        "Ctrl+Shift+P" => Err("Already used by Command Palette".to_owned()),
        _ => Ok(()),
      })
    },
    &keys,
  );
  stage.click("recorder");
  chord(&mut stage, "a", "KeyA", false, false);
  assert_eq!(held.get(), shortcut("Ctrl+K"), "no modifier");
  assert!(stage.paints("Hold Ctrl, Alt, Shift or the system key with it."));
  stage.click("recorder");
  chord(&mut stage, "P", "KeyP", true, true);
  assert_eq!(held.get(), shortcut("Ctrl+K"));
  assert_eq!(
    stage.text("Already used by Command Palette").0,
    light(PaletteColor::Destructive)
  );
  // The box shows what was refused, edged as wrong.
  assert!(stage.paints("P"));
  let edge = stage
    .border("recorder")
    .and_then(|quad| quad.border)
    .and_then(|border| border.top)
    .map(|side| side.color);
  assert_eq!(edge, Some(light(PaletteColor::Destructive)));
  // A function key is taken alone.
  stage.click("recorder");
  assert!(
    !stage.paints("Already used by Command Palette"),
    "recording again forgets the refusal"
  );
  chord(&mut stage, "F5", "F5", false, false);
  assert_eq!(held.get(), shortcut("F5"));
}

#[test]
fn a_recorder_the_app_holds_shows_what_the_app_answers() {
  let held = Signal::new(shortcut("Ctrl+K"));
  let keys = Arc::new(Mutex::new(Vec::new()));
  let asked = Arc::new(Mutex::new(Vec::new()));
  let (read, told) = (held.clone(), asked.clone());
  let mut stage = scene(
    move || {
      let (set, told) = (read.clone(), told.clone());
      // The app takes no shortcut with Alt in it.
      ShortcutRecorder::controlled(read.get(), move |shortcut: &Accelerator| {
        told.lock().unwrap().push(shortcut.to_string());
        if !shortcut.modifiers.alt {
          set.set(shortcut.clone());
        }
      })
    },
    &keys,
  );
  stage.click("recorder");
  stage
    .tree
    .key_down("x".to_owned(), "KeyX".to_owned(), false, false, true);
  stage.pass();
  assert_eq!(*asked.lock().unwrap(), ["Alt+X"]);
  assert!(stage.paints("K") && !stage.paints("X"), "what the app holds");
}

#[test]
fn read_only_and_disabled_do_not_record() {
  for disabled in [false, true] {
    let held = Signal::new(Some(shortcut("Ctrl+K")));
    let keys = Arc::new(Mutex::new(Vec::new()));
    let read = held.clone();
    let mut stage = scene(
      move || {
        ShortcutRecorder::optional(read.clone())
          .read_only(!disabled)
          .disabled(disabled)
      },
      &keys,
    );
    stage.click("recorder");
    assert!(!stage.paints("Press shortcut…"), "disabled: {disabled}");
    let cross = lurq::ui::icons::ALL
      .iter()
      .find_map(|(name, glyph)| (*name == lurq::ui::icons::CLOSE).then(|| glyph.to_string()))
      .unwrap();
    assert!(!stage.paints(&cross), "no cross");
  }
}

#[test]
fn the_sizes_and_full_width() {
  let keys = Arc::new(Mutex::new(Vec::new()));
  let mut stage = scene(
    || {
      ShortcutRecorder::controlled(shortcut("Ctrl+K"), |_| {})
        .size(ShortcutRecorderSize::Lg)
        .full_width(true)
    },
    &keys,
  );
  let (_, _, width, height) = stage.bounds("recorder");
  assert_eq!(height, 40.0);
  assert_eq!(width, 360.0, "as wide as what holds it");
}

#[test]
fn a_theme_restyles_its_parts() {
  const RED: Color = Color::new(200, 30, 30, 255);
  let app = App::new();
  app
    .theme()
    .patch_recipe::<ShortcutRecorder>(|rules| rules.base(ShortcutRecorderPart::Hint, |s| s.foreground(RED)));
  let mut stage = Stage::in_app(app, |ctx| {
    Column::new()
      .size(400.0, 300.0)
      .child(ShortcutRecorder::optional(Signal::new(None)).mount(ctx))
      .into()
  });
  stage.tree.resize(400, 300);
  stage.pass();
  assert_eq!(stage.text("Record shortcut").0, RED);
  let _ = QuadContent::None;
}

#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn tooling_reads_a_button_that_says_its_shortcut() {
  let held = Signal::new(Some(shortcut("Ctrl+Shift+P")));
  let keys = Arc::new(Mutex::new(Vec::new()));
  let read = held.clone();
  let mut stage = scene(
    move || ShortcutRecorder::optional(read.clone()).label("Command palette"),
    &keys,
  );
  let said = |stage: &mut Stage, name: &str| {
    stage
      .tree
      .get_element_by_id_mut("recorder")
      .and_then(|element| element.description(name))
  };
  assert_eq!(said(&mut stage, "a11y_role").as_deref(), Some("button"));
  assert_eq!(said(&mut stage, "a11y_name").as_deref(), Some("Command palette"));
  assert_eq!(said(&mut stage, "a11y_value").as_deref(), Some("Ctrl+Shift+P"));
  assert_eq!(said(&mut stage, "a11y_pressed").as_deref(), Some("false"));
  stage.click("recorder");
  assert_eq!(said(&mut stage, "a11y_pressed").as_deref(), Some("true"));
}
