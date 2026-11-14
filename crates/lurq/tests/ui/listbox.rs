//! A listbox: its box and its options against the board, a group's label, the chosen option and a disabled one,
//! what a press and the keys of a listbox do with one chosen and with many, the page keys in a box that scrolls, a
//! disabled listbox, and a theme's other anatomy.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App,
    events::MouseButton,
    theme::{PaletteColor, ThemePalette},
  },
  components::{Column, Row, SelectOption},
  core::Signal,
  layout::{quad::QuadContent, text_style::FontWeight},
  node::color::Color,
  ui::{Choice, Listbox, ListboxPart, ListboxRenderer, ListboxView},
};

use super::stage::Stage;

const CHECK: &str = "\u{E017}";

type Id = &'static str;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
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

/// The board's renderers: wgpu, DirectX 12, Metal, and OpenGL disabled, under the label "Renderers".
fn renderers() -> Vec<SelectOption<Id>> {
  vec![
    SelectOption::new("wgpu", "wgpu").group("Renderers"),
    SelectOption::new("dx12", "DirectX 12"),
    SelectOption::new("metal", "Metal"),
    SelectOption::new("gl", "OpenGL (legacy)").disabled(true),
    SelectOption::new("vulkan", "Vulkan"),
  ]
}

/// A key with Shift, Ctrl or both.
fn key_with(stage: &mut Stage, key: &str, shift: bool, ctrl: bool) {
  stage.tree.key_down(key.to_owned(), key.to_owned(), shift, ctrl, false);
  stage.tree.key_up(key.to_owned(), key.to_owned(), shift, ctrl, false);
  stage.pass();
}

/// A press of the element `id` with Shift, Ctrl or both.
fn press_with(stage: &mut Stage, id: &str, shift: bool, ctrl: bool) {
  let (x, y) = stage.center(id);
  stage.tree.mouse_move(x, y);
  stage
    .tree
    .mouse_down_with_modifiers(x, y, MouseButton::Left, shift, ctrl, false);
  stage
    .tree
    .mouse_up_with_modifiers(x, y, MouseButton::Left, shift, ctrl, false);
  stage.pass();
}

/// A stage of the board's listbox of one, bound to `chosen`, telling `told` what it chose.
fn one(chosen: &Signal<Id>, told: &Arc<Mutex<Vec<Vec<Id>>>>, follow: bool) -> Stage {
  let (chosen, told) = (chosen.clone(), told.clone());
  Stage::new(move |_| {
    let told = told.clone();
    Column::new()
      .padding(8.0)
      .child(
        Listbox::new(chosen.clone(), renderers())
          .label("Renderer")
          .follow_focus(follow)
          .on_change(move |next| told.lock().unwrap().push(next.to_vec()))
          .id("box"),
      )
      .into()
  })
}

#[test]
fn the_box_is_220_with_options_of_32_one_apart_and_4_in() {
  let chosen = Signal::new("wgpu");
  let mut stage = one(&chosen, &Arc::default(), true);
  let (x, y, width, _) = stage.bounds("box");
  assert_eq!(width, 220.0);
  assert!(stage.border("box").is_some(), "an edge round it");
  // The group's label over the options: 6 over it, 4 under it, 8 in, 11 at 600 in the muted color.
  let label = stage.text_quad("Renderers").unwrap();
  assert!((label.x - (x + 4.0 + 8.0)).abs() < 0.5);
  assert_eq!(stage.text("Renderers"), (light(PaletteColor::MutedForeground), 11.0));
  assert_eq!(weight_of(&stage, "Renderers"), FontWeight::SemiBold);
  let (option_x, first, option_width, height) = stage.bounds("box-0");
  assert_eq!((option_x, option_width, height), (x + 4.0, 212.0, 32.0));
  let (_, second, ..) = stage.bounds("box-1");
  assert!((second - (first + 33.0)).abs() < 0.01, "1 apart");
  assert!(first > y + 4.0 + label.height, "under the group's label");
  // The text 8 in at 13.
  let wgpu = stage.text_quad("wgpu").unwrap();
  assert!((wgpu.x - (option_x + 8.0)).abs() < 0.5);
  assert_eq!(stage.text("DirectX 12"), (light(PaletteColor::Foreground), 13.0));
}

#[test]
fn the_chosen_option_is_highlighted_with_a_check_and_a_disabled_one_is_muted() {
  let chosen = Signal::new("wgpu");
  let mut stage = one(&chosen, &Arc::default(), true);
  assert_eq!(stage.fill_color("box-0"), Some(light(PaletteColor::Highlight)));
  assert_eq!(weight_of(&stage, "wgpu"), FontWeight::Medium);
  assert_eq!(stage.fill_color("box-1"), None);
  assert_eq!(weight_of(&stage, "DirectX 12"), FontWeight::Normal);
  // One check, at the end of the chosen option, 8 from its end.
  let checks: Vec<_> = stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text == CHECK))
    .collect();
  assert_eq!(checks.len(), 1);
  let (x, y, width, height) = stage.bounds("box-0");
  assert!((checks[0].x + checks[0].width - (x + width - 8.0)).abs() < 1.0);
  assert!(checks[0].y >= y && checks[0].y + checks[0].height <= y + height);
  // A disabled option says its label in the muted color, faded.
  assert_ne!(stage.text("OpenGL (legacy)").0, light(PaletteColor::Foreground));
  // Under the pointer an option is on the muted fill.
  stage.hover("box-2");
  assert_eq!(stage.fill_color("box-2"), Some(light(PaletteColor::Muted)));
}

#[test]
fn of_one_a_press_chooses_and_the_choice_follows_the_keys() {
  let (chosen, told) = (Signal::new("dx12"), Arc::new(Mutex::new(Vec::new())));
  let mut stage = one(&chosen, &told, true);
  stage.click("box-2");
  assert_eq!(chosen.get(), "metal");
  assert_eq!(*told.lock().unwrap(), [vec!["metal"]]);
  stage.click("box-3");
  assert_eq!(chosen.get(), "metal", "a disabled option is not chosen");

  if let Some(id) = focused(&stage) {
    stage.tree.get_element_by_id_mut(&id).unwrap().blur();
  }
  stage.pass();
  stage.key("Tab");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("box-2"),
    "Tab enters at the chosen option"
  );
  stage.key("ArrowDown");
  assert_eq!(focused(&stage).as_deref(), Some("box-4"), "past the disabled one");
  assert_eq!(chosen.get(), "vulkan", "the choice follows the focus");
  stage.key("ArrowDown");
  assert_eq!(focused(&stage).as_deref(), Some("box-4"), "it does not go round");
  stage.key("Home");
  assert_eq!(chosen.get(), "wgpu");
  stage.key("m");
  assert_eq!(chosen.get(), "metal", "a letter chooses too");
  // Tab leaves it: it is one stop.
  stage.key("Tab");
  assert_ne!(focused(&stage).as_deref().map(|id| id.starts_with("box-")), Some(true));
}

#[test]
fn without_following_the_focus_space_and_enter_choose() {
  let chosen = Signal::new("wgpu");
  let mut stage = one(&chosen, &Arc::default(), false);
  stage.key("Tab");
  stage.key("ArrowDown");
  assert_eq!(focused(&stage).as_deref(), Some("box-1"));
  assert_eq!(chosen.get(), "wgpu", "the focus moves alone");
  // The focus and the choice are apart: the focused option is ringed, the chosen one highlighted.
  assert!(stage.has_ring(light(PaletteColor::Ring)));
  assert_eq!(stage.fill_color("box-0"), Some(light(PaletteColor::Highlight)));
  stage.key(" ");
  assert_eq!(chosen.get(), "dx12");
  stage.key("ArrowDown");
  stage.key("Enter");
  assert_eq!(chosen.get(), "metal");
}

/// A stage of a listbox of many, bound to `chosen`.
fn many(chosen: &Signal<Vec<Id>>) -> Stage {
  let chosen = chosen.clone();
  Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .child(Listbox::new(chosen.clone(), renderers()).label("Renderers").id("box"))
      .into()
  })
}

#[test]
fn of_many_ctrl_toggles_shift_chooses_a_range_and_space_and_ctrl_a_choose_too() {
  let chosen: Signal<Vec<Id>> = Signal::new(Vec::new());
  let mut stage = many(&chosen);
  stage.click("box-0");
  assert_eq!(chosen.get(), ["wgpu"]);
  press_with(&mut stage, "box-2", false, true);
  assert_eq!(chosen.get(), ["wgpu", "metal"], "Ctrl adds");
  press_with(&mut stage, "box-0", false, true);
  assert_eq!(chosen.get(), ["metal"], "and takes out");
  // Shift: from the last chosen to the option pressed, the disabled one passed by, the anchor kept last.
  press_with(&mut stage, "box-4", true, false);
  assert_eq!(chosen.get(), ["vulkan", "metal"]);
  // A press alone chooses that option alone.
  stage.click("box-1");
  assert_eq!(chosen.get(), ["dx12"]);
  // Each chosen option has its check.
  assert_eq!(stage.fill_color("box-1"), Some(light(PaletteColor::Highlight)));

  // Shift with the arrows moves the focus and chooses on the way.
  key_with(&mut stage, "ArrowDown", true, false);
  assert_eq!(focused(&stage).as_deref(), Some("box-2"));
  assert_eq!(chosen.get(), ["metal", "dx12"]);
  key_with(&mut stage, "End", true, false);
  assert_eq!(chosen.get(), ["metal", "vulkan", "dx12"]);
  // The arrows alone move the focus and choose nothing.
  key_with(&mut stage, "Home", false, false);
  assert_eq!(chosen.get(), ["metal", "vulkan", "dx12"]);
  // Space adds the focused option or takes it out.
  stage.key(" ");
  assert_eq!(chosen.get(), ["metal", "vulkan", "dx12", "wgpu"]);
  stage.key(" ");
  assert_eq!(chosen.get(), ["metal", "vulkan", "dx12"]);
  // Ctrl+A: every option that can be chosen.
  key_with(&mut stage, "a", false, true);
  assert_eq!(chosen.get().len(), 4);
}

#[test]
fn the_page_keys_move_by_what_the_box_shows() {
  let chosen = Signal::new(0usize);
  let scene = chosen.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .size(400.0, 300.0)
      .child(
        Listbox::new(scene.clone(), (0..30).map(|index| (index, format!("Option {index}"))))
          .id("box")
          // 4 and 4 around eight options of 32, 1 apart.
          .style(ListboxPart::Root, |s| s.max_height(8.0 + 32.0 * 8.0 + 7.0)),
      )
      .into()
  });
  let (_, _, _, height) = stage.bounds("box");
  assert!(height < 280.0, "the box scrolls: {height}");
  stage.key("Tab");
  stage.key("PageDown");
  let at = chosen.get();
  assert!((6..=8).contains(&at), "a page on: {at}");
  stage.key("PageDown");
  assert!(chosen.get() >= at + 6);
  stage.key("End");
  assert_eq!(chosen.get(), 29);
  stage.key("PageUp");
  assert!((21..=23).contains(&chosen.get()), "{}", chosen.get());
}

#[test]
fn a_disabled_listbox_takes_nothing() {
  let chosen = Signal::new("wgpu");
  let scene = chosen.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .child(Listbox::new(scene.clone(), renderers()).disabled(true).id("box"))
      .into()
  });
  stage.click("box-1");
  assert_eq!(chosen.get(), "wgpu");
  stage.key("Tab");
  assert_eq!(focused(&stage), None);
}

#[test]
fn a_choice_is_an_option() {
  let chosen = Signal::new(1u8);
  let scene = chosen.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .child(
        Listbox::new(
          scene.clone(),
          [Choice::new(1u8, "Day"), Choice::new(2u8, "Week").disabled(true)],
        )
        .id("box"),
      )
      .into()
  });
  assert!(stage.paints("Day"));
  stage.click("box-1");
  assert_eq!(chosen.get(), 1);
}

#[test]
fn a_renderer_sees_the_options_and_what_is_chosen() {
  let seen = Arc::new(Mutex::new(Vec::new()));
  let told = seen.clone();
  let renderer: ListboxRenderer = Arc::new(move |view: ListboxView| {
    *told.lock().unwrap() = view
      .options
      .iter()
      .map(|option| {
        (
          option.label.to_string(),
          option.chosen,
          option.disabled,
          option.position,
        )
      })
      .collect();
    Row::new().id("other").size(10.0, 10.0).into()
  });
  let app = App::new();
  app.theme().set_renderer::<Listbox<Id>>(renderer);
  let mut stage = Stage::in_app(app, |_| {
    Column::new()
      .child(Listbox::new(Signal::new("metal"), renderers()))
      .into()
  });
  assert_eq!(stage.bounds("other").2, 10.0);
  let seen = seen.lock().unwrap();
  assert_eq!(seen.len(), 5);
  assert_eq!(seen[2], ("Metal".to_owned(), true, false, (3, 5)));
  assert_eq!(seen[3], ("OpenGL (legacy)".to_owned(), false, true, (4, 5)));
}

/// A listbox is a listbox of options to tooling, each saying whether it is chosen.
#[cfg(feature = "mcp")]
#[test]
fn a_listbox_tells_tooling_what_is_chosen() {
  let chosen: Signal<Vec<Id>> = Signal::new(vec!["metal"]);
  let mut stage = many(&chosen);
  let mut said = |id: &str, name: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(name))
  };
  assert_eq!(said("box", "a11y_role").as_deref(), Some("listbox"));
  assert_eq!(said("box", "a11y_name").as_deref(), Some("Renderers"));
  assert_eq!(said("box", "a11y_multiselectable").as_deref(), Some("true"));
  assert_eq!(said("box-2", "a11y_role").as_deref(), Some("option"));
  assert_eq!(said("box-2", "a11y_selected").as_deref(), Some("true"));
  assert_eq!(said("box-1", "a11y_selected").as_deref(), Some("false"));
}
