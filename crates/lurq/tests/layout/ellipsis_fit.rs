//! A single-line ellipsized text whose words fit the room its row gives it
//! is drawn in full. A Row sizes a growing child to the free room it has
//! left; summing the children back up in `f32` can come out a few
//! micro-pixels over the row, and that rounding must not pass for an
//! overflow that shrinks a text by them and cuts it.

use lurq::{
  app::{App, Tree},
  components::{Rect, Row, Spacer, Text, TextOverflow},
  layout::{Constraints, Size, layout_result::LayoutResult, quad::QuadContent},
  node::{Element, dimension::Dimension},
};

const LINE_HEIGHT: f32 = 20.0;
const SPACING: f32 = 8.0;
const ICON: f32 = 14.0;
const TITLE: &str = "Pausing run #4 of KON-10 · dismissed";
/// Widths of what follows the free room, with fractions as measured text has.
const STATES: [f32; 6] = [100.0, 103.059, 117.3, 129.872, 146.2, 196.981];

/// A card head as Orchester lays one out: an icon, the title, the free room,
/// and the state at the end.
fn head(state: f32) -> Row {
  Row::new()
    .spacing(SPACING)
    .child(Rect::new(ICON, LINE_HEIGHT))
    .child(
      Text::new(TITLE)
        .nowrap()
        .text_overflow(TextOverflow::Elipsis)
        .min_width(Dimension::Px(0.0))
        .flex_shrink(1.0),
    )
    .child(Spacer::new().flex(1.0))
    .child(Rect::new(state, LINE_HEIGHT))
}

/// One pass of the head at `width`: the title's box, the spacer's box and
/// the string drawn for the title.
struct HeadPass {
  title: f32,
  spacer: f32,
  drawn: String,
}

fn layout_head(tree: &mut Tree, app: &mut App, width: f32) -> HeadPass {
  tree.set_layout_constraints_override(Some(Constraints::tight(Size::new(width, LINE_HEIGHT))));
  tree.pass_headless(app);
  let layout: LayoutResult = tree.last_layout().expect("layout result").clone();
  let drawn = tree
    .resolve_quads(&layout)
    .into_iter()
    .find_map(|quad| match quad.content {
      QuadContent::Text { text, .. } => Some(text),
      _ => None,
    })
    .expect("the title is drawn");
  HeadPass {
    title: layout.children[1].result.size.width,
    spacer: layout.children[2].result.size.width,
    drawn,
  }
}

fn head_at(root: impl Into<Element>, width: f32, scale: f32) -> HeadPass {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_scale_factor(scale);
  tree.set_root(root);
  layout_head(&mut tree, &mut app, width)
}

/// Every width from 400 to 900 in steps of half a pixel, at every state
/// width, on one tree each: the title's box is its words' width and the
/// title is drawn whole wherever the spacer has room.
fn sweep(scale: f32) -> Vec<String> {
  let mut failures = Vec::new();
  for state in STATES {
    let mut app = App::new();
    let mut tree = Tree::new();
    tree.set_scale_factor(scale);
    tree.set_root(head(state));
    let natural = layout_head(&mut tree, &mut app, 2000.0).title;
    for step in 0..1000 {
      let width = 400.0 + step as f32 * 0.5;
      let pass = layout_head(&mut tree, &mut app, width);
      if pass.spacer < 1.0 {
        continue;
      }
      if pass.drawn != TITLE || pass.title != natural {
        failures.push(format!(
          "state {state} width {width}: {:?} in a box of {} (words {natural}), spacer {}",
          pass.drawn, pass.title, pass.spacer
        ));
      }
    }
  }
  failures
}

#[test]
fn ellipsis_fit_title_is_whole_wherever_the_spacer_has_room() {
  let failures = sweep(1.0);
  assert!(
    failures.is_empty(),
    "{} cut titles:\n{}",
    failures.len(),
    failures.join("\n")
  );
}

#[test]
fn ellipsis_fit_title_is_whole_wherever_the_spacer_has_room_at_scale_1_5() {
  let failures = sweep(1.5);
  assert!(
    failures.is_empty(),
    "{} cut titles:\n{}",
    failures.len(),
    failures.join("\n")
  );
}

/// The free room is taken by the spacer and given back on the way up: the
/// title is whole at a width it was laid out at before, after the row was
/// narrower and shrank it.
#[test]
fn ellipsis_fit_title_is_whole_again_after_a_narrower_width() {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_root(head(STATES[1]));
  let wide = layout_head(&mut tree, &mut app, 500.0);
  assert_eq!(wide.drawn, TITLE, "the title fits a wide row: box {}", wide.title);
  let narrow = layout_head(&mut tree, &mut app, 300.0);
  assert!(narrow.drawn.ends_with('…'), "a narrow row cuts the title: {narrow:?}");
  assert_eq!(narrow.spacer, 0.0, "the spacer gave up its room first");
  let again = layout_head(&mut tree, &mut app, 500.0);
  assert_eq!(again.drawn, TITLE, "the title is whole again: box {}", again.title);
  assert_eq!(again.title, wide.title);
}

/// A row too narrow for its words still ends the title in an ellipsis.
#[test]
fn ellipsis_fit_real_overflow_still_cuts_the_title() {
  let natural = head_at(head(STATES[0]), 2000.0, 1.0).title;
  let content = ICON + natural + STATES[0] + SPACING * 3.0;
  for overflow in [0.9, 20.0] {
    let pass = head_at(head(STATES[0]), content - overflow, 1.0);
    assert!(pass.drawn.ends_with('…'), "overflow {overflow}: {:?}", pass.drawn);
    assert!(
      pass.drawn.chars().count() < TITLE.chars().count(),
      "overflow {overflow}: {:?}",
      pass.drawn
    );
    assert_eq!(pass.spacer, 0.0, "overflow {overflow}: the spacer has no room");
    assert!(
      pass.title <= natural - overflow + 0.01,
      "overflow {overflow}: the box {} gave way",
      pass.title
    );
  }
}

impl std::fmt::Debug for HeadPass {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{:?} in a box of {}, spacer {}", self.drawn, self.title, self.spacer)
  }
}
