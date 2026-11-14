//! A key: its box against the board, a shortcut of several keys, a Mac's glyphs and their order, a sequence, the
//! sizes, the plain text a menu writes, that it takes no focus, a theme's rules and renderer; and inline code.

use std::sync::Arc;

use lurq::{
  app::{
    Accelerator, App,
    theme::{PaletteColor, ThemePalette},
  },
  components::{Column, Row},
  layout::quad::{Quad, QuadContent},
  node::{Element, color::Color},
  ui::{Button, InlineCode, Kbd, KbdPart, KbdRenderer, KbdSize, KbdVariant, KbdView, KeyPlatform},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn scene(children: Vec<Element>) -> Element {
  children
    .into_iter()
    .fold(Column::new().padding(24.0).spacing(16.0), |column, child| {
      column.child(Row::new().child(child))
    })
    .into()
}

/// The boxes filled with `color` inside the box of `id`, from the start.
fn boxes_in(stage: &mut Stage, id: &str, color: Color) -> Vec<Quad> {
  let (x, y, width, height) = stage.bounds(id);
  let mut boxes: Vec<Quad> = stage
    .quads()
    .into_iter()
    .filter(|quad| {
      matches!(&quad.content, QuadContent::Rect { color: fill, .. } if *fill == color)
        && quad.x >= x - 0.01
        && quad.y >= y - 0.01
        && quad.x + quad.width <= x + width + 0.01
        && quad.y + quad.height <= y + height + 0.01
    })
    .collect();
  boxes.sort_by(|a, b| a.x.total_cmp(&b.x));
  boxes
}

/// Where the texts are painted, from the start: they are drawn in that order.
fn order(stage: &Stage, texts: &[&str]) -> Vec<String> {
  let mut found: Vec<(f32, String)> = texts
    .iter()
    .map(|text| {
      let quad = stage
        .text_quad(text)
        .unwrap_or_else(|| panic!("{text:?} should be painted"));
      (quad.x, text.to_string())
    })
    .collect();
  found.sort_by(|a, b| a.0.total_cmp(&b.0));
  found.into_iter().map(|(_, text)| text).collect()
}

#[test]
fn a_key_is_20_high_on_the_muted_fill_with_an_edge_and_its_text_in_the_family_of_code() {
  let mut stage = Stage::new(|_| scene(vec![Kbd::new("Ctrl").id("ctrl").into()]));
  let (x, _, _, height) = stage.bounds("ctrl");
  assert_eq!(height, 20.0);
  assert_eq!(stage.fill_color("ctrl"), Some(light(PaletteColor::Muted)));
  let edge = stage.border("ctrl").expect("an edge");
  assert_eq!(
    edge.border.and_then(|border| border.top).map(|side| side.color),
    Some(light(PaletteColor::Border))
  );
  let fill = stage.fill("ctrl").unwrap();
  assert!(
    fill.border_radius.is_some_and(|radius| radius.top_left == 4.0),
    "the radius sm"
  );
  assert_eq!(stage.text("Ctrl"), (light(PaletteColor::MutedForeground), 11.0));
  assert_eq!(stage.font_of("Ctrl"), stage.app.theme().mono_family().to_string());
  let label = stage.text_quad("Ctrl").unwrap();
  assert!((label.x - x - 6.0).abs() < 0.01, "padded 6: {}", label.x - x);
}

#[test]
fn a_shortcut_draws_a_key_each_4_apart_and_a_letter_in_capitals() {
  let mut stage = Stage::new(|_| scene(vec![Kbd::shortcut("Ctrl+Shift+p").id("palette").into()]));
  let keys = boxes_in(&mut stage, "palette", light(PaletteColor::Muted));
  assert_eq!(keys.len(), 3, "a box for each key");
  for pair in keys.windows(2) {
    let gap = pair[1].x - (pair[0].x + pair[0].width);
    assert!((gap - 4.0).abs() < 0.01, "4 apart: {gap}");
  }
  assert_eq!(order(&stage, &["Ctrl", "Shift", "P"]), ["Ctrl", "Shift", "P"]);
  assert!(!stage.paints("+"), "nothing between the keys");
}

#[test]
fn keys_are_named_as_lurq_names_them_and_a_plus_after_a_plus_is_the_key() {
  let mut stage = Stage::new(|_| {
    scene(vec![
      Kbd::shortcut("Mod+ArrowUp").into(),
      Kbd::shortcut("Ctrl++").id("plus").into(),
      Kbd::new("Escape").into(),
      Kbd::new("Page Up").into(),
    ])
  });
  // `Mod` is Ctrl in words.
  assert_eq!(order(&stage, &["Ctrl", "↑"]), ["Ctrl", "↑"]);
  assert_eq!(boxes_in(&mut stage, "plus", light(PaletteColor::Muted)).len(), 2);
  assert!(stage.paints("+"));
  assert!(stage.paints("Esc"));
  assert!(stage.paints("Page Up"));
}

#[test]
fn on_a_mac_the_modifiers_are_glyphs_first_and_in_a_macs_order() {
  let mut stage = Stage::new(|_| {
    scene(vec![
      Kbd::shortcut("Mod+Shift+P")
        .platform(KeyPlatform::Mac)
        .id("palette")
        .into(),
      Kbd::shortcut("Alt+Backspace").platform(KeyPlatform::Mac).into(),
    ])
  });
  // ⇧ before ⌘, as a Mac's menus write them, though Mod was written first.
  assert_eq!(order(&stage, &["⇧", "⌘", "P"]), ["⇧", "⌘", "P"]);
  assert!(stage.paints("⌥") && stage.paints("⌫"));
  assert!(!stage.paints("Shift"));
  assert_eq!(boxes_in(&mut stage, "palette", light(PaletteColor::Muted)).len(), 3);
}

#[test]
fn an_accelerator_of_the_menu_is_drawn_with_its_modifiers_then_its_key() {
  let mut save = Accelerator::command("s");
  save.modifiers.shift = true;
  let mac = save.clone();
  let stage = Stage::new(move |_| {
    scene(vec![
      Kbd::accelerator(&mac).platform(KeyPlatform::Mac).into(),
      Kbd::accelerator(&save).id("words").into(),
    ])
  });
  // As `Accelerator::display` writes it: ⇧⌘S.
  assert_eq!(order(&stage, &["⇧", "⌘"]), ["⇧", "⌘"]);
  assert!(stage.paints("S"));
  assert!(stage.paints("Shift") && stage.paints("Meta"));
}

#[test]
fn a_sequence_says_then_between_its_steps() {
  let mut stage = Stage::new(|_| {
    scene(vec![
      Kbd::sequence(["G", "D"]).id("go").into(),
      Kbd::sequence(["Ctrl+K", "Ctrl+S"])
        .separator("and then")
        .id("chord")
        .into(),
    ])
  });
  assert_eq!(order(&stage, &["G", "then", "D"]), ["G", "then", "D"]);
  assert_eq!(stage.text("then").0, light(PaletteColor::MutedForeground));
  assert_eq!(boxes_in(&mut stage, "go", light(PaletteColor::Muted)).len(), 2);
  assert_eq!(boxes_in(&mut stage, "chord", light(PaletteColor::Muted)).len(), 4);
  assert!(stage.paints("and then"));
}

#[test]
fn the_sizes_are_16_20_and_24_high() {
  let mut stage = Stage::new(|_| {
    scene(vec![
      Kbd::new("K").size(KbdSize::Sm).id("sm").into(),
      Kbd::new("K").id("md").into(),
      Kbd::new("K").size(KbdSize::Lg).id("lg").into(),
      Kbd::shortcut("Ctrl+K").size(KbdSize::Lg).id("pair").into(),
    ])
  });
  for (id, height, padding) in [("sm", 16.0, 4.0), ("md", 20.0, 6.0), ("lg", 24.0, 7.0)] {
    let (x, _, width, measured) = stage.bounds(id);
    assert_eq!(measured, height, "{id}");
    let key = boxes_in(&mut stage, id, light(PaletteColor::Muted));
    assert_eq!(key.len(), 1, "{id}");
    assert!(
      (key[0].x - x).abs() < 0.01 && (key[0].width - width).abs() < 0.01,
      "{id}"
    );
    let label = stage
      .quads()
      .into_iter()
      .find(|quad| {
        matches!(&quad.content, QuadContent::Text { text, .. } if text == "K")
          && quad.x >= key[0].x
          && quad.x < key[0].x + key[0].width
          && quad.y >= key[0].y
          && quad.y < key[0].y + key[0].height
      })
      .unwrap_or_else(|| panic!("the K of {id}"));
    assert!(
      (label.x - x - padding).abs() < 0.01,
      "{id} padded {padding}: {}",
      label.x - x
    );
  }
  let sizes: Vec<f32> = stage
    .quads()
    .iter()
    .filter_map(|quad| match &quad.content {
      QuadContent::Text { text, style, .. } if text == "K" => Some(style.font_size),
      _ => None,
    })
    .collect();
  assert!(
    sizes.contains(&10.0) && sizes.contains(&11.0) && sizes.contains(&12.0),
    "{sizes:?}"
  );
  let keys = boxes_in(&mut stage, "pair", light(PaletteColor::Muted));
  let gap = keys[1].x - (keys[0].x + keys[0].width);
  assert!((gap - 6.0).abs() < 0.01, "large keys are 6 apart: {gap}");
}

#[test]
fn plain_keys_are_one_text_with_no_box_as_a_menu_writes_them() {
  let mut stage = Stage::new(|_| {
    scene(vec![
      Kbd::shortcut("Ctrl+K").variant(KbdVariant::Plain).id("words").into(),
      Kbd::shortcut("Mod+K")
        .variant(KbdVariant::Plain)
        .platform(KeyPlatform::Mac)
        .into(),
    ])
  });
  assert_eq!(stage.text("Ctrl+K"), (light(PaletteColor::MutedForeground), 11.0));
  assert!(stage.paints("⌘K"));
  assert!(boxes_in(&mut stage, "words", light(PaletteColor::Muted)).is_empty());
  assert!(stage.border("words").is_none());
}

#[test]
fn keys_take_no_focus_and_tab_passes_them_by() {
  let mut stage = Stage::new(|_| {
    scene(vec![
      Kbd::shortcut("Ctrl+K").id("keys").into(),
      Button::new("Search").id("search").into(),
    ])
  });
  stage.key("Tab");
  let focused = stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned));
  assert_eq!(
    focused.as_deref(),
    Some("search"),
    "Tab reaches the button, not the keys"
  );
}

#[test]
fn a_theme_restyles_keys_and_draws_them_another_way() {
  let app = App::new();
  app
    .theme()
    .patch_recipe::<Kbd>(|rules| rules.base(KbdPart::Key, |s| s.height(28.0).background(PaletteColor::Secondary)));
  let mut stage = Stage::in_app(app, |_| scene(vec![Kbd::new("Esc").id("esc").into()]));
  let (.., height) = stage.bounds("esc");
  assert_eq!(height, 28.0);
  assert_eq!(stage.fill_color("esc"), Some(light(PaletteColor::Secondary)));

  // A renderer that writes the keys of each step as one text in the root.
  let app = App::new();
  let renderer: KbdRenderer = Arc::new(|view: KbdView| {
    let text = view
      .steps
      .iter()
      .map(|step| view.written(step))
      .collect::<Vec<_>>()
      .join(", ");
    view.root().child(view.label(&text)).into()
  });
  app.theme().set_renderer::<Kbd>(renderer);
  let stage = Stage::in_app(app, |_| scene(vec![Kbd::sequence(["Ctrl+K", "S"]).into()]));
  assert!(stage.paints("Ctrl+K, S"));
  assert!(!stage.paints("then"));
}

#[test]
fn inline_code_is_13_in_the_family_of_code_on_the_muted_fill() {
  let mut stage = Stage::new(|_| scene(vec![InlineCode::new("ctx.theme()").id("code").into()]));
  assert_eq!(stage.fill_color("code"), Some(light(PaletteColor::Muted)));
  assert_eq!(stage.text("ctx.theme()"), (light(PaletteColor::Foreground), 13.0));
  assert_eq!(
    stage.font_of("ctx.theme()"),
    stage.app.theme().mono_family().to_string()
  );
  let (x, y, _, height) = stage.bounds("code");
  let text = stage.text_quad("ctx.theme()").unwrap();
  assert!((text.x - x - 5.0).abs() < 0.01, "padded 5 across: {}", text.x - x);
  assert!((text.y - y - 1.0).abs() < 0.01, "padded 1 above: {}", text.y - y);
  assert!((height - text.height - 2.0).abs() < 0.01, "and 1 below");
  assert!(stage.border("code").is_none(), "no edge");
}
