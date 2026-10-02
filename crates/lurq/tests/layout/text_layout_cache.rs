//! Text layout writes per-constraint results into the node's text state (the
//! ellipsized string, whether the renderer wraps, the caret geometry that
//! selection uses). A layout served from a node's cache must leave that state
//! as the cached layout computed it, not as the last real layout left it.

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx, events::MouseButton},
  components::{Row, ScrollVertical, Text, TextInput, TextOverflow},
  core::Signal,
  layout::{
    Constraints, Size,
    layout_result::LayoutResult,
    quad::QuadContent,
    scrollbar::{ScrollBarPlacement, ScrollBarStyle, ScrollBarVisibility},
  },
  node::Element,
};

const LABEL: &str = "connect-to-production-cluster";
const WIDE: f32 = 400.0;
const NARROW: f32 = 100.0;
const LINE_HEIGHT: f32 = 20.0;
const PARAGRAPH: &str = "The quick brown fox jumps over the lazy dog and keeps running past the hills";
const INPUT_VALUE: &str = "aaaa bbbb cccc dddd";

/// One pass under tight constraints with a persistent `App`: a fresh `App`
/// changes the theme version, force-dirties every layout and bypasses the
/// cache under test.
fn layout_at(tree: &mut Tree, app: &mut App, constraints: Constraints) -> LayoutResult {
  tree.set_layout_constraints_override(Some(constraints));
  tree.pass_headless(app);
  tree.last_layout().expect("layout result").clone()
}

fn tight(width: f32, height: f32) -> Constraints {
  Constraints::tight(Size::new(width, height))
}

/// Width of the root's first child, or of the root when it has no children.
fn text_width(layout: &LayoutResult) -> f32 {
  layout
    .children
    .first()
    .map_or(layout.size.width, |child| child.result.size.width)
}

/// The text the renderer draws, and whether it wraps it.
fn drawn_text(tree: &Tree, layout: &LayoutResult) -> (String, bool) {
  tree
    .resolve_quads(layout)
    .into_iter()
    .find_map(|quad| match quad.content {
      QuadContent::Text { text, wrap, .. } => Some((text, wrap)),
      _ => None,
    })
    .expect("text quad")
}

fn ellipsized_label() -> Text {
  Text::new(LABEL).nowrap().text_overflow(TextOverflow::Elipsis)
}

/// One pass of [`drawn_labels`]: the available width, the text's laid-out
/// width and the string drawn for it.
#[derive(Debug, PartialEq)]
struct LabelPass {
  available: f32,
  width: f32,
  drawn: String,
}

impl LabelPass {
  fn assert_fits(&self) {
    if self.available >= WIDE {
      assert_eq!(self.drawn, LABEL, "a wide label is drawn in full: {self:?}");
    } else {
      assert!(self.width <= self.available, "the label fits its width: {self:?}");
      assert!(self.drawn.ends_with('…'), "a narrow label is ellipsized: {self:?}");
      assert!(self.drawn.len() < LABEL.len(), "a narrow label is shortened: {self:?}");
    }
  }
}

/// Lays `root` out at each available width in turn.
fn drawn_labels(root: impl Into<Element>, widths: &[f32]) -> Vec<LabelPass> {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_root(root);
  widths
    .iter()
    .map(|&available| {
      let layout = layout_at(&mut tree, &mut app, tight(available, LINE_HEIGHT));
      LabelPass {
        available,
        width: text_width(&layout),
        drawn: drawn_text(&tree, &layout).0,
      }
    })
    .collect()
}

#[test]
fn text_cache_root_ellipsis_follows_width_back_up() {
  let passes = drawn_labels(ellipsized_label(), &[WIDE, NARROW, WIDE]);
  passes.iter().for_each(LabelPass::assert_fits);
  assert_eq!(passes[0], passes[2], "the cached wide layout draws what it drew first");
}

#[test]
fn text_cache_row_shrunk_ellipsis_follows_width_back_up() {
  let row = Row::new().child(ellipsized_label().flex_shrink(1.0));
  let passes = drawn_labels(row, &[WIDE, NARROW, WIDE]);
  passes.iter().for_each(LabelPass::assert_fits);
  assert_eq!(passes[0], passes[2], "the cached wide layout draws what it drew first");
}

/// The window width, as a component that re-renders on resize reads it.
#[derive(Clone)]
struct WindowWidth(Signal<f32>);

impl PartialEq for WindowWidth {
  fn eq(&self, other: &Self) -> bool {
    self.0.id() == other.0.id()
  }
}

#[cfg(feature = "devtools")]
impl lurq::app::component::DevtoolsInspectable for WindowWidth {}

struct ResizingToolbar;

impl Component for ResizingToolbar {
  type Props = WindowWidth;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    // Reading the width re-renders the toolbar on every resize; the output
    // keeps its shape, so the re-rendered nodes inherit the layout caches.
    ctx.props::<Self::Props>().0.get();
    Row::new().child(ellipsized_label().flex_shrink(1.0))
  }
}

#[test]
fn text_cache_rerendered_component_ellipsis_follows_width_back_up() {
  let mut app = App::new();
  let mut tree = Tree::new();
  let window_width = Signal::new(WIDE);
  tree.mount_root::<ResizingToolbar>(&mut app, WindowWidth(window_width.clone()));
  let passes: Vec<LabelPass> = [WIDE, NARROW, WIDE]
    .into_iter()
    .map(|available| {
      window_width.set(available);
      let layout = layout_at(&mut tree, &mut app, tight(available, LINE_HEIGHT));
      LabelPass {
        available,
        width: text_width(&layout),
        drawn: drawn_text(&tree, &layout).0,
      }
    })
    .collect();
  passes.iter().for_each(LabelPass::assert_fits);
  assert_eq!(passes[0], passes[2], "the cached wide layout draws what it drew first");
}

#[test]
fn text_cache_ellipsis_returns_after_narrow_wide_narrow() {
  let passes = drawn_labels(ellipsized_label(), &[NARROW, WIDE, NARROW]);
  passes.iter().for_each(LabelPass::assert_fits);
  assert_eq!(
    passes[0], passes[2],
    "the cached narrow layout draws what it drew first"
  );
}

/// A label in a vertical scroll area with a reserved gutter. Each real layout
/// of the area lays the label out at the full width, then inside the gutter.
fn label_in_gutter_scroll(label: &str) -> ScrollVertical {
  ScrollVertical::new(
    Text::new(label)
      .nowrap()
      .text_overflow(TextOverflow::Elipsis)
      .id("label"),
  )
  .scrollbar(ScrollBarStyle {
    visible: ScrollBarVisibility::Always,
    placement: ScrollBarPlacement::Reserved,
    width: 24.0,
    padding: 0.0,
    ..Default::default()
  })
}

fn drawn_in_gutter_scroll(tree: &mut Tree, app: &mut App) -> String {
  let layout = layout_at(tree, app, Constraints::loose(Size::new(NARROW * 2.0, NARROW)));
  drawn_text(tree, &layout).0
}

#[test]
fn text_cache_repaired_child_reused_by_parent_relayout_draws_its_own_layout() {
  const CHANGED: &str = "iiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiii";
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_root(label_in_gutter_scroll(LABEL));
  drawn_in_gutter_scroll(&mut tree, &mut app);

  // The scroll area repairs the changed label inside the gutter, then lays
  // itself out again because the label's size changed: the label is laid out
  // at the full width before the repaired layout is reused inside the gutter.
  tree
    .get_element_by_id_mut("label")
    .expect("label")
    .set_text_content(CHANGED);
  let drawn = drawn_in_gutter_scroll(&mut tree, &mut app);

  let mut fresh_app = App::new();
  let mut fresh = Tree::new();
  fresh.set_root(label_in_gutter_scroll(CHANGED));
  let expected = drawn_in_gutter_scroll(&mut fresh, &mut fresh_app);
  assert!(expected.ends_with('…'), "the label is ellipsized: {expected:?}");
  assert_eq!(drawn, expected, "the reused layout draws its own ellipsis");
}

#[test]
fn text_cache_render_wrap_follows_bounded_width_back() {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_root(Text::new(PARAGRAPH));
  let bounded = Constraints::loose(Size::new(NARROW, 400.0));
  let unbounded = Constraints::loose(Size::new(f32::INFINITY, 400.0));

  let first = layout_at(&mut tree, &mut app, bounded);
  assert!(drawn_text(&tree, &first).1, "text wraps inside a bounded width");
  let wide = layout_at(&mut tree, &mut app, unbounded);
  assert!(!drawn_text(&tree, &wide).1, "text does not wrap without a width bound");
  let again = layout_at(&mut tree, &mut app, bounded);

  assert_eq!(again.size, first.size, "the bounded layout is served again");
  assert!(drawn_text(&tree, &again).1, "the cached bounded layout still wraps");
}

/// Selection rectangles (y, width) of a selectable text, from its caret
/// geometry.
fn selection_rects(tree: &Tree, layout: &LayoutResult) -> Vec<(f32, f32)> {
  tree
    .resolve_quads(layout)
    .into_iter()
    .filter(|quad| matches!(quad.content, QuadContent::Rect { .. }) && quad.width > 1.0)
    .map(|quad| (quad.y, quad.width))
    .collect()
}

#[test]
fn text_cache_wrapped_selection_follows_width_back_up() {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_root(Text::new(PARAGRAPH).selectable(true));
  let wide = Constraints::loose(Size::new(WIDE * 2.0, 400.0));
  let narrow = Constraints::loose(Size::new(NARROW, 400.0));

  let first = layout_at(&mut tree, &mut app, wide);
  let y = first.size.height / 2.0;
  tree.mouse_down(0.0, y, MouseButton::Left);
  tree.mouse_move(first.size.width, y);
  tree.mouse_up(first.size.width, y, MouseButton::Left);
  let first = layout_at(&mut tree, &mut app, wide);
  let selected = selection_rects(&tree, &first);
  assert_eq!(selected.len(), 1, "one selected line at the wide width: {selected:?}");

  let narrowed = layout_at(&mut tree, &mut app, narrow);
  assert!(
    selection_rects(&tree, &narrowed).len() > 1,
    "the selection follows the wrapped lines"
  );
  let again = layout_at(&mut tree, &mut app, wide);

  assert_eq!(again.size, first.size, "the wide layout is served again");
  assert_eq!(
    selection_rects(&tree, &again),
    selected,
    "the cached wide layout selects what it selected first"
  );
}

/// Where a click at the end of the first visual row puts the caret in a
/// multiline input that was laid out at `widths` in turn.
fn caret_after_click_past_first_row(widths: &[f32]) -> usize {
  let value = Signal::new(INPUT_VALUE.to_owned());
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_root(TextInput::new(value.clone()).multiline());
  for &width in widths {
    layout_at(&mut tree, &mut app, tight(width, 200.0));
  }
  let last = *widths.last().expect("at least one width");
  tree.mouse_down(last - 2.0, 4.0, MouseButton::Left);
  tree.mouse_up(last - 2.0, 4.0, MouseButton::Left);
  tree.key_down("X".to_owned(), "KeyX".to_owned(), false, false, false);
  value.get().find('X').expect("marker is inserted")
}

#[test]
fn text_cache_multiline_input_caret_follows_width_back_up() {
  let expected = caret_after_click_past_first_row(&[WIDE]);
  assert_eq!(expected, INPUT_VALUE.len(), "the value fits on one wide row");
  assert_eq!(
    caret_after_click_past_first_row(&[WIDE, NARROW / 2.0, WIDE]),
    expected,
    "the cached wide layout places the caret where the first wide layout did"
  );
}
