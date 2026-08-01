//! Production repro (PW-studio hub cards): after a re-render chain changed a
//! text's content, the paint rastered the NEW (longer) text into the OLD
//! content's measured box — wrapping it at the stale width and overlapping
//! the sibling line below. Retained-diff chains transplanted the parent's
//! cached layout while laundering the pending dirty flags (a duplicate
//! re-render cleared what an earlier diff had marked).

use std::sync::Arc;

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx},
  components::{Column, ScrollVertical, Text},
  layout::Alignment,
  node::{Element, color::Color, dimension::Dimension},
};

use crate::support::{RenderSnapshot, render_pass_with_app};

#[derive(Clone, PartialEq)]
struct CardProps {
  title: Arc<str>,
}

#[cfg(feature = "devtools")]
impl lurq::app::component::DevtoolsInspectable for CardProps {}

struct CardRoot;

impl Component for CardRoot {
  type Props = CardProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<CardProps>().clone();
    Column::new()
      .width(400.0)
      .spacing(4.0)
      .child(Text::new(&props.title))
      .child(Text::new("subtitle_marker"))
  }
}

/// Distinct glyph rows: title + subtitle = 2. A stale-width wrap adds a third
/// row (and overlaps the subtitle). Glyph tops within one line differ by a
/// few px (ascenders/descenders); line steps are ~a full line-height, so
/// cluster with a gap threshold in between.
fn baseline_rows(snapshot: &RenderSnapshot) -> usize {
  let mut ys: Vec<f32> = snapshot.glyphs.iter().map(|glyph| glyph.y).collect();
  ys.sort_by(|a, b| a.partial_cmp(b).unwrap());
  let mut rows = 0;
  let mut last = f32::MIN;
  for y in ys {
    if y - last > 9.0 {
      rows += 1;
    }
    last = y;
  }
  rows
}

fn title(text: &str) -> CardProps {
  CardProps { title: Arc::from(text) }
}

#[test]
fn changed_text_is_measured_fresh_not_wrapped_at_stale_width() {
  let mut tree = Tree::new();
  let mut app = App::new();
  tree.mount_root::<CardRoot>(&mut app, title("Награда за"));
  tree.resize(800, 600);

  let snapshot = render_pass_with_app(&mut tree, &mut app);
  assert_eq!(
    baseline_rows(&snapshot),
    2,
    "short title lays as one line plus subtitle"
  );

  // Content change followed by duplicate re-renders BEFORE any paint — the
  // production pattern (a window shell re-rendering per input event).
  tree.update_root_props::<CardRoot>(title("Награда за доставку"));
  tree.update_root_props::<CardRoot>(title("Награда за доставку"));
  tree.update_root_props::<CardRoot>(title("Награда за доставку"));

  let snapshot = render_pass_with_app(&mut tree, &mut app);
  assert_eq!(
    baseline_rows(&snapshot),
    2,
    "longer title must be re-measured (one line at 400px), not wrapped inside the previous title's box"
  );

  // And it must stay correct on subsequent paints.
  let snapshot = render_pass_with_app(&mut tree, &mut app);
  assert_eq!(baseline_rows(&snapshot), 2);
}

const DIALOGUE_CARD_COLOR: &str = "#19324c";

#[derive(Clone, PartialEq)]
struct DialogueProps {
  text: Arc<str>,
}

#[cfg(feature = "devtools")]
impl lurq::app::component::DevtoolsInspectable for DialogueProps {}

struct StretchingDialogue;

impl Component for StretchingDialogue {
  type Props = DialogueProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<DialogueProps>();
    let card = Column::new()
      .align_items(Alignment::Stretch)
      .padding(8.0)
      .background(DIALOGUE_CARD_COLOR)
      .child(Text::new("Window #1"))
      .child(Text::new(&props.text));
    let body = Column::new()
      .align_items(Alignment::Stretch)
      .width(Dimension::full())
      .padding(8.0)
      .child(card);

    Column::new()
      .width(320.0)
      .height(180.0)
      .align_items(Alignment::Stretch)
      .child(ScrollVertical::new(body).width(Dimension::full()).flex(1.0))
  }
}

fn dialogue(text: &str) -> DialogueProps {
  DialogueProps { text: Arc::from(text) }
}

fn dialogue_card_height(snapshot: &RenderSnapshot) -> f32 {
  let color = Color::from_hex(DIALOGUE_CARD_COLOR);
  snapshot
    .rects
    .iter()
    .find(|rect| rect.color == color)
    .expect("dialogue card background")
    .height
}

#[test]
fn stretched_wrapper_grows_and_shrinks_when_retained_text_reflows() {
  let mut tree = Tree::new();
  let mut app = App::new();
  tree.mount_root::<StretchingDialogue>(&mut app, dialogue("Short dialogue."));
  tree.resize(800, 600);

  let short = render_pass_with_app(&mut tree, &mut app);
  let short_height = dialogue_card_height(&short);

  tree.update_root_props::<StretchingDialogue>(dialogue(
    "A considerably longer dialogue line that must wrap across several rows inside the fixed-width card. \
     Its retained wrapper must grow instead of clipping the newly shaped glyphs.",
  ));
  let long = render_pass_with_app(&mut tree, &mut app);
  let long_height = dialogue_card_height(&long);
  assert!(
    long_height > short_height + 20.0,
    "stretched retained wrapper should grow for wrapped text: {long_height} <= {short_height}"
  );

  tree.update_root_props::<StretchingDialogue>(dialogue("Short dialogue."));
  let short_again = render_pass_with_app(&mut tree, &mut app);
  let short_again_height = dialogue_card_height(&short_again);
  assert!(
    (short_again_height - short_height).abs() < 1.0,
    "stretched retained wrapper should shrink back: {short_again_height} vs {short_height}"
  );
}
