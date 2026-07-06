//! Production repro (PW-studio hub cards): after a re-render chain changed a
//! text's content, the paint rastered the NEW (longer) text into the OLD
//! content's measured box — wrapping it at the stale width and overlapping
//! the sibling line below. Retained-diff chains transplanted the parent's
//! cached layout while laundering the pending dirty flags (a duplicate
//! re-render cleared what an earlier diff had marked).

use std::sync::Arc;

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx},
  components::{Column, Text},
  node::Element,
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
