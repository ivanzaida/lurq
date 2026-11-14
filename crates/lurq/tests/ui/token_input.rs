//! A token input and mentions: the box, the tokens in its text with their fill, cross and looks, what the caret,
//! Backspace and a click on a cross do to them, and the list a trigger opens.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App,
    events::MouseButton,
    theme::{PaletteColor, RadiusSize, ThemePalette},
  },
  components::{Column, TextInputController},
  core::Signal,
  layout::quad::{Quad, QuadContent},
  node::{CursorIcon, color::Color},
  ui::{
    Avatar, Field, Mention, Mentions, MentionsController, TagVariant, Themed, Token, TokenInput, TokenInputPart,
    TokenInputRenderer, TokenInputSize, TokenInputView, icons,
  },
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// The fills of the tokens painted in `color`, left to right and top to bottom.
fn tokens(stage: &Stage, color: Color) -> Vec<Quad> {
  let mut fills: Vec<Quad> = stage
    .quads()
    .into_iter()
    .filter(|quad| {
      matches!(&quad.content, QuadContent::Rect { color: fill, .. } if *fill == color)
        && quad.border_radius.is_some()
        && quad.height <= 30.0
    })
    .collect();
  fills.sort_by(|a, b| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)));
  fills
}

/// The crosses painted at the ends of tokens: the theme's close glyph.
fn crosses(stage: &Stage) -> Vec<Quad> {
  let glyph = stage
    .app
    .theme()
    .icon(icons::CLOSE)
    .expect("the close icon")
    .glyph
    .to_string();
  let mut crosses: Vec<Quad> = stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if *text == glyph))
    .collect();
  crosses.sort_by(|a, b| a.x.total_cmp(&b.x));
  crosses
}

/// The width and the color of the edge of the element with `id`.
fn edge(stage: &mut Stage, id: &str) -> (f32, Color) {
  let quad = stage.border(id).unwrap_or_else(|| panic!("#{id} should have an edge"));
  let side = quad.border.and_then(|border| border.top).expect("an edge on top");
  (side.width, side.color)
}

fn press(stage: &mut Stage, (x, y): (f32, f32)) {
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass();
}

fn middle(quad: &Quad) -> (f32, f32) {
  (quad.x + quad.width / 2.0, quad.y + quad.height / 2.0)
}

fn tags(value: Signal<String>) -> TokenInput {
  TokenInput::new(value)
    .tokens(|text| Token::separated(text, ' '))
    .placeholder("Add tag")
    .id("tags")
}

#[test]
fn a_token_input_is_the_box_of_an_input_with_its_tokens_as_chips() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .child(tags(Signal::new("rust wgpu ".to_owned())))
      .into()
  });
  let (x, y, width, height) = stage.bounds("tags");
  assert_eq!((width, height), (260.0, 36.0));
  assert_eq!(stage.fill_color("tags"), Some(light(PaletteColor::Background)));
  assert_eq!(edge(&mut stage, "tags"), (1.0, light(PaletteColor::Input)));

  let chips = tokens(&stage, light(PaletteColor::Secondary));
  assert_eq!(chips.len(), 2, "rust and wgpu, each a chip");
  let radius = stage.app.theme().radius_value(RadiusSize::Sm);
  for chip in &chips {
    assert_eq!(chip.height, 24.0);
    assert!(
      (chip.y + 12.0 - (y + 18.0)).abs() < 2.0,
      "in the middle of the box: {}",
      chip.y - y
    );
    assert_eq!(chip.border_radius.unwrap().top_left, radius);
  }
  assert!(
    (chips[0].x - x - 6.0).abs() < 1.0,
    "6 from the start of the box: {}",
    chips[0].x - x
  );
  assert!(chips[1].x > chips[0].x + chips[0].width, "one after the other");

  // Each ends with its cross, 12 square in the muted foreground, 6 before the end of the chip.
  let crosses = crosses(&stage);
  assert_eq!(crosses.len(), 2);
  for (cross, chip) in crosses.iter().zip(&chips) {
    assert_eq!((cross.width, cross.height), (12.0, 12.0));
    let QuadContent::Text { style, .. } = &cross.content else {
      unreachable!()
    };
    assert_eq!(style.color, light(PaletteColor::MutedForeground));
    assert!((chip.x + chip.width - (cross.x + cross.width) - 6.0).abs() < 0.5);
  }
}

#[test]
fn the_sizes_are_an_inputs_with_tokens_of_20_24_and_28() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .spacing(12.0)
      .child(tags(Signal::new("sm ".to_owned())).size(TokenInputSize::Sm).id("sm"))
      .child(tags(Signal::new("md ".to_owned())).id("md"))
      .child(tags(Signal::new("lg ".to_owned())).size(TokenInputSize::Lg).id("lg"))
      .into()
  });
  let chips = tokens(&stage, light(PaletteColor::Secondary));
  for ((id, box_height, chip_height), chip) in [("sm", 32.0, 20.0), ("md", 36.0, 24.0), ("lg", 40.0, 28.0)]
    .into_iter()
    .zip(chips)
  {
    assert_eq!(stage.bounds(id).3, box_height, "{id}");
    assert_eq!(chip.height, chip_height, "{id}");
  }
}

#[test]
fn a_token_of_a_variant_is_in_the_colors_of_a_tag_of_it() {
  let stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .child(
        TokenInput::new(Signal::new("passing failing".to_owned()))
          .tokens(|_| {
            vec![
              Token::new(0..7).variant(TagVariant::Success),
              Token::new(8..15).variant(TagVariant::Destructive),
            ]
          })
          .id("checks"),
      )
      .into()
  });
  assert_eq!(tokens(&stage, light(PaletteColor::SuccessMuted)).len(), 1);
  assert_eq!(tokens(&stage, light(PaletteColor::DestructiveMuted)).len(), 1);
  let colors: Vec<Color> = crosses(&stage)
    .into_iter()
    .map(|cross| match cross.content {
      QuadContent::Text { style, .. } => style.color,
      _ => unreachable!(),
    })
    .collect();
  assert_eq!(colors, [light(PaletteColor::Success), light(PaletteColor::Destructive)]);
}

#[test]
fn a_click_on_a_cross_takes_its_token_away_as_an_edit_that_undo_brings_back() {
  let value = Signal::new("rust wgpu naga ".to_owned());
  let edits = Arc::new(Mutex::new(Vec::new()));
  let (bound, told) = (value.clone(), edits.clone());
  let mut stage = Stage::new(move |_| {
    let told = told.clone();
    Column::new()
      .padding(8.0)
      .child(tags(bound.clone()).on_input(move |event| told.lock().unwrap().push(event.new_value.clone())))
      .into()
  });
  let cross = crosses(&stage).swap_remove(1);
  // The cross is pressed as a button is: the pointer is a hand over it, and the text cursor beside it.
  let (x, y) = middle(&cross);
  stage.tree.mouse_move(x, y);
  assert_eq!(stage.tree.cursor(), CursorIcon::Pointer);
  stage.tree.mouse_move(x - 20.0, y);
  assert_eq!(stage.tree.cursor(), CursorIcon::Text);
  press(&mut stage, (x, y));
  assert_eq!(value.get(), "rust naga ", "wgpu and the space after it");
  assert_eq!(edits.lock().unwrap().as_slice(), ["rust naga "]);
  assert_eq!(crosses(&stage).len(), 2);

  stage.tree.key_down("z".into(), "KeyZ".into(), false, true, false);
  stage.tree.key_up("z".into(), "KeyZ".into(), false, true, false);
  stage.pass();
  assert_eq!(value.get(), "rust wgpu naga ");
}

#[test]
fn the_caret_goes_past_a_token_whole_and_backspace_selects_it_before_it_deletes_it() {
  let value = Signal::new("rust wgpu x".to_owned());
  let controller = TextInputController::new();
  let (bound, held) = (value.clone(), controller.clone());
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .child(tags(bound.clone()).controller(held.clone()))
      .into()
  });
  stage.key("Tab");
  assert_eq!(
    stage
      .tree
      .focused_element()
      .and_then(|element| element.id().map(str::to_owned))
      .as_deref(),
    Some("tags")
  );
  // After "wgpu" and its space, Left goes over the space, then over the token at once.
  controller.select(10, 10).unwrap();
  stage.key("ArrowLeft");
  stage.key("ArrowLeft");
  assert_eq!(
    controller.selection().unwrap().caret,
    5,
    "from the end of wgpu to its start"
  );

  // Backspace right after a token selects it, drawn in the primary color; the next deletes it.
  controller.select(9, 9).unwrap();
  stage.pass();
  stage.key("Backspace");
  assert_eq!(controller.selection().unwrap().range(), 5..9);
  assert_eq!(value.get(), "rust wgpu x", "nothing deleted yet");
  assert_eq!(
    tokens(&stage, light(PaletteColor::Primary)).len(),
    1,
    "the selected token"
  );
  stage.key("Backspace");
  assert_eq!(value.get(), "rust  x");
}

#[test]
fn a_click_on_a_token_selects_all_of_it_and_the_pointer_lights_it() {
  let value = Signal::new("rust wgpu ".to_owned());
  let controller = TextInputController::new();
  let (bound, held) = (value.clone(), controller.clone());
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .child(tags(bound.clone()).controller(held.clone()))
      .into()
  });
  let chip = tokens(&stage, light(PaletteColor::Secondary)).swap_remove(0);
  // Under the pointer it takes the hover of the secondary color.
  let (x, y) = (chip.x + 6.0, chip.y + chip.height / 2.0);
  stage.tree.mouse_move(x, y);
  stage.pass();
  assert_eq!(tokens(&stage, light(PaletteColor::SecondaryHover)).len(), 1);
  // A click on its text selects the whole of it.
  press(&mut stage, (x, y));
  assert_eq!(controller.selection().unwrap().range(), 0..4);
  assert_eq!(tokens(&stage, light(PaletteColor::Primary)).len(), 1);
}

#[test]
fn tokens_and_text_share_the_rows_of_a_token_input_on_rows() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .child(
        TokenInput::new(Signal::new(
          "rust wgpu naga winit cosmic taffy parley vello kurbo peniko ".to_owned(),
        ))
        .tokens(|text| Token::separated(text, ' '))
        .rows(2, 6)
        .id("tags"),
      )
      .into()
  });
  let chips = tokens(&stage, light(PaletteColor::Secondary));
  assert_eq!(chips.len(), 10);
  let rows: Vec<f32> = chips.iter().map(|chip| chip.y.round()).fold(Vec::new(), |mut rows, y| {
    if !rows.contains(&y) {
      rows.push(y);
    }
    rows
  });
  assert!(rows.len() >= 2, "they wrap: {rows:?}");
  assert!(
    rows[1] - rows[0] >= 26.0,
    "a row holds a token with room around it: {rows:?}"
  );
  let (_, _, _, height) = stage.bounds("tags");
  assert!(height >= 2.0 * 26.0, "as high as its rows: {height}");
}

#[test]
fn read_only_and_disabled_tokens_have_no_cross() {
  let stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .spacing(12.0)
      .child(tags(Signal::new("rust ".to_owned())).read_only(true).id("read"))
      .child(tags(Signal::new("wgpu ".to_owned())).disabled(true).id("off"))
      .child(
        TokenInput::new(Signal::new("naga".to_owned()))
          .tokens(|_| vec![Token::new(0..4).removable(false)])
          .id("fixed"),
      )
      .into()
  });
  assert_eq!(tokens(&stage, light(PaletteColor::Secondary)).len(), 3);
  assert!(crosses(&stage).is_empty());
}

#[test]
fn a_disabled_token_input_takes_nothing() {
  let value = Signal::new("rust ".to_owned());
  let bound = value.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .child(tags(bound.clone()).disabled(true))
      .into()
  });
  assert_eq!(stage.fill_color("tags"), Some(light(PaletteColor::Muted)));
  stage.click("tags");
  stage.key("Tab");
  assert!(stage.tree.focused_element().is_none());
  stage.tree.text_input_replace(0..0, "x");
  stage.pass();
  assert_eq!(value.get(), "rust ");
}

#[test]
fn a_field_judges_a_token_input() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .child(Field::new("Tags", tags(Signal::new("rust ".to_owned()))).error("Add at least two tags."))
      .into()
  });
  assert_eq!(edge(&mut stage, "tags"), (1.0, light(PaletteColor::Destructive)));
}

#[test]
fn a_rule_of_a_theme_changes_the_tokens_and_a_renderer_the_anatomy() {
  let app = App::new();
  app.theme().set_recipe::<TokenInput>(
    <TokenInput as Themed>::recipe().base(TokenInputPart::Token, |s| s.background(PaletteColor::Info).height(22.0)),
  );
  let rendered = Arc::new(Mutex::new(0));
  let told = rendered.clone();
  let renderer: TokenInputRenderer = Arc::new(move |view: TokenInputView| {
    *told.lock().unwrap() += view.tokens.len();
    view.render()
  });
  app.theme().set_renderer::<TokenInput>(renderer);
  let stage = Stage::in_app(app, |_| {
    Column::new()
      .padding(8.0)
      .child(tags(Signal::new("rust wgpu ".to_owned())))
      .into()
  });
  let chips = tokens(&stage, light(PaletteColor::Info));
  assert_eq!(chips.len(), 2);
  assert_eq!(chips[0].height, 22.0);
  assert!(*rendered.lock().unwrap() >= 2, "the theme's renderer saw both tokens");
}

fn people() -> Vec<Mention> {
  ["Ivan Zaida", "Ivy Chen", "Ivo Kovac", "Mara Lind"]
    .into_iter()
    .map(|name| Mention::new(name).avatar(Avatar::new(name)))
    .collect()
}

/// A mentions field in a window of its own size, so that its list is painted.
fn comment(value: Signal<String>, mentions: MentionsController) -> Stage {
  Stage::new(move |_| {
    Column::new()
      .size(400.0, 300.0)
      .padding(8.0)
      .child(
        Mentions::new(value.clone(), &mentions)
          .trigger('@', people())
          .trigger('#', |typed: &str| {
            vec![Mention::new(&format!("{typed}-team")).text(&format!("#{typed}-team"))]
          })
          .id("comment"),
      )
      .into()
  })
}

/// Types `text` at the end of the focused field.
fn type_at_end(stage: &mut Stage, value: &Signal<String>, text: &str) {
  let end = value.get().len();
  stage.tree.text_input_replace(end..end, text);
  stage.pass();
}

fn highlighted(stage: &Stage) -> Vec<Quad> {
  stage
    .quads()
    .into_iter()
    .filter(|quad| {
      matches!(&quad.content, QuadContent::Rect { color, .. } if *color == light(PaletteColor::Highlight))
        && quad.height == 36.0
    })
    .collect()
}

#[test]
fn mentions_is_a_box_of_280_and_a_trigger_opens_the_offers_under_it() {
  let value = Signal::new(String::new());
  let mut stage = comment(value.clone(), MentionsController::new());
  let (_, _, width, height) = stage.bounds("comment");
  assert_eq!(width, 280.0);
  assert!(height >= 64.0, "at least 64 high: {height}");
  assert!(!stage.paints("Ivan Zaida"), "no list before a trigger");

  stage.key("Tab");
  type_at_end(&mut stage, &value, "Thanks for the review @iv");
  for name in ["Ivan Zaida", "Ivy Chen", "Ivo Kovac"] {
    assert!(stage.paints(name), "{name} is offered");
  }
  assert!(!stage.paints("Mara Lind"), "what does not have what was typed is not");
  // The first offer is the one Enter chooses, filled with the highlight.
  let active = highlighted(&stage);
  assert_eq!(active.len(), 1);
  let ivan = stage.text_quad("Ivan Zaida").unwrap();
  assert!(ivan.y >= active[0].y && ivan.y < active[0].y + 36.0);
  // The list hangs under the text, inside the window.
  let (_, top, ..) = stage.bounds("comment");
  assert!(ivan.y > top + 8.0);
}

#[test]
fn down_and_enter_choose_an_offer_which_stands_as_a_mention() {
  let value = Signal::new(String::new());
  let mentions = MentionsController::new();
  let mut stage = comment(value.clone(), mentions.clone());
  stage.key("Tab");
  type_at_end(&mut stage, &value, "Thanks @iv");
  stage.key("ArrowDown");
  let chosen = highlighted(&stage);
  let ivy = stage.text_quad("Ivy Chen").unwrap();
  assert!(ivy.y >= chosen[0].y && ivy.y < chosen[0].y + 36.0, "Down moves to Ivy");
  stage.key("Enter");
  assert_eq!(value.get(), "Thanks @Ivy Chen ");
  assert!(!stage.paints("Ivan Zaida"), "the list is closed");
  assert_eq!(mentions.mentioned(), [Arc::from("@Ivy Chen")]);
  assert_eq!(
    tokens(&stage, light(PaletteColor::InfoMuted)).len(),
    1,
    "the mention's fill"
  );

  // Backspace deletes the space, then selects the mention, then deletes it.
  stage.key("Backspace");
  stage.key("Backspace");
  assert_eq!(value.get(), "Thanks @Ivy Chen");
  assert_eq!(mentions.input().selection().unwrap().range(), 7..16);
  stage.key("Backspace");
  assert_eq!(value.get(), "Thanks ");
}

#[test]
fn escape_puts_the_list_away_and_typing_goes_on_as_text() {
  let value = Signal::new(String::new());
  let mut stage = comment(value.clone(), MentionsController::new());
  stage.key("Tab");
  type_at_end(&mut stage, &value, "@iv");
  assert!(stage.paints("Ivan Zaida"));
  stage.key("Escape");
  assert!(!stage.paints("Ivan Zaida"));
  assert!(stage.tree.focused_element().is_some(), "the field keeps the focus");
  type_at_end(&mut stage, &value, "y");
  assert!(!stage.paints("Ivy Chen"), "still away for this trigger");
  assert_eq!(value.get(), "@ivy");
  stage.key("Enter");
  assert_eq!(value.get(), "@ivy\n", "Enter is a new line again");
}

#[test]
fn a_click_on_an_offer_chooses_it_and_a_function_offers_what_it_finds() {
  let value = Signal::new(String::new());
  let mut stage = comment(value.clone(), MentionsController::new());
  stage.key("Tab");
  type_at_end(&mut stage, &value, "ask #design");
  assert!(stage.paints("design-team"));
  let offer = stage.text_quad("design-team").unwrap();
  press(&mut stage, middle(&offer));
  assert_eq!(value.get(), "ask #design-team ");
  assert_eq!(
    stage
      .tree
      .focused_element()
      .and_then(|element| element.id().map(str::to_owned))
      .as_deref(),
    Some("comment"),
    "the press kept the focus in the field"
  );
}

/// The panel of the list of offers: the popover's fill, 220 wide.
fn list_panel(stage: &Stage) -> Option<Quad> {
  stage.quads().into_iter().find(|quad| {
    matches!(&quad.content, QuadContent::Rect { color, .. } if *color == light(PaletteColor::Popover))
      && quad.width == 220.0
  })
}

/// Whether the list stands under the trigger at byte `at` of the field, as the field was laid out last: its left at
/// the trigger's, its top 4 under the trigger's line.
fn stands_at_trigger(stage: &mut Stage, mentions: &MentionsController, at: usize) {
  let panel = list_panel(stage).expect("the list is painted");
  let (x, y, ..) = stage.bounds("comment");
  let point = mentions.input().caret_point(at).unwrap();
  // The box's padding is 12 across and 8 down.
  let (left, top) = (x + 12.0 + point.x, y + 8.0 + point.y + point.height + 4.0);
  assert!(point.x > 50.0, "the trigger is well into the line: {}", point.x);
  assert!(
    (panel.x - left).abs() < 1.0,
    "at the trigger's left: {} for {left}",
    panel.x
  );
  assert!(
    (panel.y - top).abs() < 1.0,
    "under the trigger's line: {} for {top}",
    panel.y
  );
}

#[test]
fn the_list_stands_at_a_trigger_that_arrives_in_a_paste_at_the_first_paint() {
  let value = Signal::new(String::new());
  let mentions = MentionsController::new();
  let mut stage = comment(value.clone(), mentions.clone());
  stage.key("Tab");
  // The whole line at once, as a paste puts it: the trigger is nowhere the text laid out before had.
  stage.tree.text_input_replace(0..0, "Thanks for the review @iv");
  stage.pass_after(std::time::Duration::ZERO);
  stands_at_trigger(&mut stage, &mentions, 22);
}

#[test]
fn the_list_stands_at_a_trigger_in_a_value_the_app_sets_at_the_first_paint() {
  let value = Signal::new(String::new());
  let mentions = MentionsController::new();
  let mut stage = comment(value.clone(), mentions.clone());
  stage.key("Tab");
  value.set("Ping the release owner @iv".to_owned());
  stage.pass_after(std::time::Duration::ZERO);
  assert!(stage.paints("Ivan Zaida"), "the caret was at the end, and stays there");
  stands_at_trigger(&mut stage, &mentions, 23);
}
