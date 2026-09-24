//! Focus shows one style whatever moved it there: click, Tab or a request.

use lurq::{
  app::{Tree, events::MouseButton, theme::BorderSize},
  components::{Button, Checkbox, Column, Slider},
  core::Signal,
  node::{CheckboxStyle, SliderPartStyle, Style, color::Color},
};

use crate::support::{RenderSnapshot, pointer_click, render_pass};

const FOCUS: &str = "#2563eb";

fn has_fill(snapshot: &RenderSnapshot, hex: &str) -> bool {
  snapshot.rects.iter().any(|rect| rect.color == Color::from_hex(hex))
}

fn has_stroke(snapshot: &RenderSnapshot, hex: &str) -> bool {
  snapshot
    .rects
    .iter()
    .any(|rect| rect.stroke_color == Color::from_hex(hex) && rect.stroke.iter().any(|width| *width > 0.0))
}

fn tab(tree: &mut Tree) {
  tree.key_down("Tab".to_owned(), "Tab".to_owned(), false, false, false);
}

fn click_center(tree: &mut Tree, id: &str) {
  let (x, y) = tree
    .get_element_by_id_mut(id)
    .and_then(|element| element.bounds())
    .expect("element should be laid out")
    .center();
  pointer_click(tree, x, y, MouseButton::Left);
}

fn button_screen() -> Column {
  Column::new().child(
    Button::new("Save")
      .id("save")
      .size(120.0, 32.0)
      .tab_index(0)
      .background("#e5e7eb")
      .focused_style(Style::new().background(FOCUS)),
  )
}

#[test]
fn button_shows_its_focused_style_when_tabbed_to() {
  let mut tree = Tree::new();
  tree.set_root(button_screen());
  assert!(!has_fill(&render_pass(&mut tree), FOCUS));

  tab(&mut tree);
  assert!(has_fill(&render_pass(&mut tree), FOCUS));
}

#[test]
fn button_shows_the_same_focused_style_when_clicked() {
  let mut tree = Tree::new();
  tree.set_root(button_screen());
  render_pass(&mut tree);

  click_center(&mut tree, "save");
  assert!(has_fill(&render_pass(&mut tree), FOCUS));
}

#[test]
fn checkbox_box_shows_its_focused_style() {
  let mut tree = Tree::new();
  tree.set_root(
    Column::new().child(
      Checkbox::new(Signal::new(false))
        .id("check")
        .tab_index(0)
        .box_style(
          CheckboxStyle::new()
            .size(16.0, 16.0)
            .border_inside(BorderSize::Sm, "#9ca3af"),
        )
        .box_focused_style(
          CheckboxStyle::new()
            .size(40.0, 40.0)
            .border_inside(BorderSize::Sm, FOCUS),
        ),
    ),
  );
  let unfocused = render_pass(&mut tree);
  assert!(!has_stroke(&unfocused, FOCUS));

  tab(&mut tree);
  let focused = render_pass(&mut tree);
  assert!(has_stroke(&focused, FOCUS));
  let box_width = |snapshot: &RenderSnapshot| {
    snapshot
      .rects
      .iter()
      .find(|rect| rect.stroke.iter().any(|width| *width > 0.0))
      .map(|rect| rect.width)
  };
  assert_eq!(
    box_width(&focused),
    box_width(&unfocused),
    "focus never resizes the box"
  );
}

#[test]
fn slider_thumb_shows_its_focused_style() {
  let mut tree = Tree::new();
  tree.set_root(
    Column::new().child(
      Slider::new(Signal::new(0))
        .range(0, 10)
        .width(200.0)
        .tab_index(0)
        .thumb_style(SliderPartStyle::new().size(16.0, 16.0).background("#111827"))
        .thumb_focused_style(SliderPartStyle::new().border_inside(BorderSize::Sm, FOCUS)),
    ),
  );
  assert!(!has_stroke(&render_pass(&mut tree), FOCUS));

  tab(&mut tree);
  assert!(has_stroke(&render_pass(&mut tree), FOCUS));
}
