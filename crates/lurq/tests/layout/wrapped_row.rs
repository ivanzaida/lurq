use lurq::{
  app::Tree,
  components::{Button, Column, Row, Text},
  layout::{Constraints, Size, quad::QuadContent},
  node::dimension::Dimension,
};

use super::PassLayoutExt;

const CONTENT: &str = "This action is unavailable because the selected field belongs to a linked style. Edit the source style to preserve the complete original binding and its provenance.";

#[test]
fn wrapped_row_preserves_compact_buttons_and_line_spacing() {
  for scale in [1.0, 1.5, 2.0] {
    let button = || Button::new("Edit").padding_horizontal(9.0).padding_vertical(5.0);
    let mut plain = Tree::new();
    plain.set_scale_factor(scale);
    plain.set_root(Row::new().child(button()));
    let intrinsic = plain.pass_layout(Constraints::loose(Size::new(400.0, 720.0))).unwrap();
    let button_size = intrinsic.children[0].result.size;
    assert!(
      button_size.width > 18.0,
      "the ordinary button must contain measurable text"
    );

    let spacing = 7.0;
    let width = button_size.width * 2.0 + spacing + 1.0;
    let mut wrapped = Tree::new();
    wrapped.set_scale_factor(scale);
    wrapped.set_root(
      Row::new()
        .wrap()
        .spacing(spacing)
        .child(button())
        .child(button())
        .child(button()),
    );
    let result = wrapped
      .pass_layout(Constraints::tight(Size::new(width, 720.0)))
      .unwrap();

    for child in &result.children {
      assert_eq!(
        child.result.size, button_size,
        "wrapping must preserve intrinsic button size at DPI {scale}"
      );
    }
    assert_eq!(result.children[0].offset.x, 0.0);
    assert_eq!(result.children[1].offset.x, button_size.width + spacing);
    assert_eq!(result.children[0].offset.y, result.children[1].offset.y);
    assert_eq!(result.children[2].offset.x, 0.0);
    assert_eq!(result.children[2].offset.y, button_size.height + spacing);
  }
}

#[test]
fn wrapped_row_resolves_full_width_multiline_child_inside_padding() {
  for scale in [1.0, 1.5, 2.0] {
    let mut tree = Tree::new();
    tree.set_scale_factor(scale);
    tree.set_root(
      Row::new()
        .wrap()
        .spacing(4.0)
        .width(Dimension::Pct(100.0))
        .padding_horizontal(16.0)
        .child(
          Column::new()
            .width(Dimension::Pct(100.0))
            .spacing(4.0)
            .child(Button::new("Remove"))
            .child(Text::new(CONTENT).width(Dimension::Pct(100.0))),
        )
        .child(Button::new("Add")),
    );
    let result = tree.pass_layout(Constraints::loose(Size::new(264.0, 720.0))).unwrap();
    let notice = &result.children[0];
    let text = &notice.result.children[1].result;
    let button = &notice.result.children[0].result;

    assert_eq!(result.size.width, 264.0);
    assert_eq!(notice.offset.x, 16.0);
    assert_eq!(
      notice.result.size.width, 232.0,
      "percentage width must use the padded content constraint"
    );
    assert_eq!(text.size.width, 232.0);
    assert!(
      text.size.height > button.size.height,
      "the complete notice must wrap into multiple lines"
    );
    assert!(button.size.width < 232.0, "the ordinary action must remain compact");
    assert_eq!(result.children[1].offset.x, 16.0);
    assert_eq!(
      result.children[1].offset.y,
      notice.offset.y + notice.result.size.height + 4.0
    );

    let quads = tree.resolve_quads(&result);
    let quad = quads
      .iter()
      .find(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text == CONTENT))
      .expect("the entire original notice must reach rendering");
    assert!(matches!(&quad.content, QuadContent::Text { wrap: true, .. }));
    assert_eq!(quad.width, 232.0);
    assert!(quad.x + quad.width <= 248.0);
  }
}

#[test]
fn wrapped_row_resolves_percentage_max_width_without_forcing_compact_siblings() {
  let mut tree = Tree::new();
  tree.set_root(
    Row::new()
      .wrap()
      .spacing(4.0)
      .child(Text::new(CONTENT).max_width(Dimension::Pct(50.0)))
      .child(Button::new("Add")),
  );
  let result = tree.pass_layout(Constraints::loose(Size::new(232.0, 720.0))).unwrap();
  let text = &result.children[0].result;
  let button = &result.children[1].result;
  assert!(
    text.size.width > 0.0 && text.size.width <= 116.0,
    "percentage max-width must bound the measured text"
  );
  assert!(text.size.height > button.size.height);
  assert!(button.size.width < 116.0);
  assert_eq!(
    result.children[1].offset.y, 0.0,
    "the compact action should share the line when it fits"
  );
}
