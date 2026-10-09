//! Focus-visible: a focus ring for keyboard focus only, like CSS `:focus-visible`.

use lurq::{
  app::{Tree, events::MouseButton, theme::BorderSize},
  components::{Button, Checkbox, Column, Slider, TextInput},
  core::{ElementRef, Signal},
  node::{CheckboxStyle, SliderPartStyle, Style, color::Color, interaction_state::InteractionState},
};

use crate::support::{RenderSnapshot, pointer_click, render_pass};

const FOCUSED: &str = "#16a34a";
const RING: &str = "#2563eb";

fn has_fill(snapshot: &RenderSnapshot, hex: &str) -> bool {
  snapshot.rects.iter().any(|rect| rect.color == Color::from_hex(hex))
}

fn has_stroke(snapshot: &RenderSnapshot, hex: &str) -> bool {
  snapshot
    .rects
    .iter()
    .any(|rect| rect.stroke_color == Color::from_hex(hex) && rect.stroke.iter().any(|width| *width > 0.0))
}

fn key(tree: &mut Tree, key: &str, ctrl: bool) {
  tree.key_down(key.to_owned(), key.to_owned(), false, ctrl, false);
  tree.key_up(key.to_owned(), key.to_owned(), false, ctrl, false);
}

fn tab(tree: &mut Tree) {
  key(tree, "Tab", false);
}

fn click_center(tree: &mut Tree, id: &str) {
  let (x, y) = tree
    .get_element_by_id_mut(id)
    .and_then(|element| element.bounds())
    .expect("element should be laid out")
    .center();
  pointer_click(tree, x, y, MouseButton::Left);
}

fn focused_id(tree: &Tree) -> Option<String> {
  tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

struct Screen {
  tree: Tree,
  state: InteractionState,
  save: ElementRef,
}

/// Two buttons: `save` fills green on any focus and draws a blue ring only on
/// keyboard focus.
fn screen() -> Screen {
  let state = InteractionState::new();
  let save = ElementRef::new();
  let mut tree = Tree::new();
  tree.set_root(
    Column::new()
      .child(
        Button::new("Save")
          .id("save")
          .size(120.0, 32.0)
          .tab_index(0)
          .background("#e5e7eb")
          .interactive(state.clone())
          .ref_element(save.clone())
          .focused_style(Style::new().background(FOCUSED))
          .focus_visible(|style| style.border_inside(BorderSize::Sm, RING)),
      )
      .child(Button::new("Open").id("open").size(120.0, 32.0).tab_index(0)),
  );
  render_pass(&mut tree);
  Screen { tree, state, save }
}

#[test]
fn tab_shows_the_focus_visible_style() {
  let mut screen = screen();
  assert!(!has_stroke(&render_pass(&mut screen.tree), RING));

  tab(&mut screen.tree);
  let frame = render_pass(&mut screen.tree);
  assert!(has_stroke(&frame, RING), "keyboard focus draws the ring");
  assert!(has_fill(&frame, FOCUSED), "focused_style still applies");
  assert!(screen.tree.focus_visible());
  assert!(screen.state.is_focus_visible());
  assert!(screen.save.focused() && screen.save.focus_visible());
}

#[test]
fn click_focuses_without_the_focus_visible_style() {
  let mut screen = screen();
  click_center(&mut screen.tree, "save");
  let frame = render_pass(&mut screen.tree);

  assert_eq!(focused_id(&screen.tree).as_deref(), Some("save"));
  assert!(has_fill(&frame, FOCUSED), "focused_style shows on any focus");
  assert!(!has_stroke(&frame, RING), "a click draws no ring");
  assert!(!screen.tree.focus_visible());
  assert!(screen.state.is_focused() && !screen.state.is_focus_visible());
  assert!(!screen.save.focus_visible());
}

#[test]
fn a_key_press_after_a_click_shows_the_ring() {
  let mut screen = screen();
  click_center(&mut screen.tree, "save");
  render_pass(&mut screen.tree);

  key(&mut screen.tree, "Shift", false);
  key(&mut screen.tree, "c", true);
  assert!(
    !screen.tree.focus_visible(),
    "bare modifiers and shortcuts keep the pointer modality"
  );

  key(&mut screen.tree, "ArrowDown", false);
  assert!(screen.tree.focus_visible());
  assert!(has_stroke(&render_pass(&mut screen.tree), RING));
}

#[test]
fn a_pointer_press_after_keyboard_focus_hides_the_ring() {
  let mut screen = screen();
  tab(&mut screen.tree);
  assert!(screen.tree.focus_visible());

  click_center(&mut screen.tree, "save");
  let frame = render_pass(&mut screen.tree);
  assert_eq!(focused_id(&screen.tree).as_deref(), Some("save"));
  assert!(!screen.tree.focus_visible());
  assert!(!has_stroke(&frame, RING));
  assert!(has_fill(&frame, FOCUSED));
}

#[test]
fn the_focus_visible_signal_follows_the_modality() {
  let screen = screen();
  let mut tree = screen.tree;
  let visible = screen.save.focus_visible_signal();
  assert!(!visible.get());

  tab(&mut tree);
  assert!(visible.get());
  click_center(&mut tree, "save");
  assert!(!visible.get());
  key(&mut tree, "a", false);
  assert!(visible.get());
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("open"));
  assert!(!visible.get(), "losing focus loses focus-visible");
}

#[test]
fn a_focus_request_inherits_the_last_input_modality() {
  let mut screen = screen();
  screen.tree.get_element_by_id_mut("save").expect("save").focus();
  assert!(!screen.tree.focus_visible(), "before any input the modality is pointer");

  tab(&mut screen.tree);
  screen.tree.get_element_by_id_mut("save").expect("save").focus();
  assert_eq!(focused_id(&screen.tree).as_deref(), Some("save"));
  assert!(
    screen.tree.focus_visible(),
    "after keyboard input a request shows the ring"
  );

  click_center(&mut screen.tree, "open");
  screen.tree.get_element_by_id_mut("save").expect("save").focus();
  assert!(!screen.tree.focus_visible(), "after a click a request shows no ring");
}

#[test]
fn a_clicked_text_input_is_focus_visible() {
  let state = InteractionState::new();
  let mut tree = Tree::new();
  tree.set_root(
    Column::new().child(
      TextInput::new(Signal::new(String::new()))
        .id("name")
        .size(200.0, 32.0)
        .interactive(state.clone())
        .focus_visible(|style| style.border_inside(BorderSize::Sm, RING)),
    ),
  );
  render_pass(&mut tree);

  click_center(&mut tree, "name");
  assert_eq!(focused_id(&tree).as_deref(), Some("name"));
  assert!(tree.focus_visible());
  assert!(state.is_focus_visible());
  assert!(has_stroke(&render_pass(&mut tree), RING));
}

#[test]
fn checkbox_box_focused_style_is_keyboard_only() {
  let mut tree = Tree::new();
  tree.set_root(
    Column::new().child(
      Checkbox::new(Signal::new(false))
        .id("check")
        .size(16.0, 16.0)
        .tab_index(0)
        .box_style(CheckboxStyle::new().size(16.0, 16.0))
        .box_focused_style(CheckboxStyle::new().border_inside(BorderSize::Sm, RING)),
    ),
  );
  render_pass(&mut tree);

  click_center(&mut tree, "check");
  assert_eq!(focused_id(&tree).as_deref(), Some("check"));
  assert!(!has_stroke(&render_pass(&mut tree), RING), "a click draws no ring");

  key(&mut tree, "ArrowDown", false);
  assert!(has_stroke(&render_pass(&mut tree), RING));
}

#[test]
fn slider_thumb_focused_style_is_keyboard_only() {
  let mut tree = Tree::new();
  tree.set_root(
    Column::new().child(
      Slider::new(Signal::new(5))
        .id("volume")
        .range(0, 10)
        .size(200.0, 24.0)
        .tab_index(0)
        .thumb_style(SliderPartStyle::new().size(16.0, 16.0).background("#111827"))
        .thumb_focused_style(SliderPartStyle::new().border_inside(BorderSize::Sm, RING)),
    ),
  );
  render_pass(&mut tree);

  click_center(&mut tree, "volume");
  assert_eq!(focused_id(&tree).as_deref(), Some("volume"));
  assert!(!has_stroke(&render_pass(&mut tree), RING), "a click draws no ring");

  tab(&mut tree);
  assert!(has_stroke(&render_pass(&mut tree), RING));
}
