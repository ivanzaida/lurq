//! A line laid out narrower and then wider again, through the layout cache,
//! gives every child back its natural size and text.

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx},
  components::{Rect, Row, ScrollVertical},
  core::Signal,
  layout::{
    Constraints, Size,
    layout_kind::ShrinkLimit,
    layout_result::LayoutResult,
    scrollbar::{ScrollBarPlacement, ScrollBarStyle, ScrollBarVisibility},
  },
  node::Element,
};

use super::{
  COUNT, DETAIL, LINE_HEIGHT, RUN, SPACING, TIME, assert_close, child_widths, drawn_texts, label, layout_at, tight,
};

const WIDE: f32 = 900.0;
const NARROW: f32 = 150.0;
const GUTTER: f32 = 24.0;

/// A status bar: an item whose icon and count stay whole, a run label, a time
/// that drops first, all giving way in order.
fn status_bar() -> Row {
  let item = Row::new()
    .spacing(SPACING)
    .child(Rect::new(14.0, 14.0))
    .child(label(COUNT))
    .child(label(DETAIL).flex_shrink(1.0))
    .flex_shrink(1.0)
    .shrink_order(2)
    .shrink_limit(ShrinkLimit::Content);
  Row::new()
    .spacing(SPACING)
    .child(item)
    .child(label(RUN).flex_shrink(1.0).shrink_order(3))
    .child(
      label(TIME)
        .flex_shrink(1.0)
        .shrink_order(1)
        .shrink_limit(ShrinkLimit::Drop),
    )
}

/// What one pass lays out and draws.
#[derive(Debug, PartialEq)]
struct Pass {
  widths: Vec<f32>,
  item_widths: Vec<f32>,
  drawn: Vec<String>,
}

fn pass(tree: &Tree, layout: &LayoutResult) -> Pass {
  Pass {
    widths: child_widths(layout),
    item_widths: child_widths(&layout.children[0].result),
    drawn: drawn_texts(tree, layout),
  }
}

fn assert_natural(pass: &Pass) {
  assert_eq!(
    pass.drawn,
    [COUNT, DETAIL, RUN, TIME],
    "a wide bar draws every text whole"
  );
  assert!(
    pass.widths.iter().all(|width| *width > 0.0),
    "nothing is dropped: {pass:?}"
  );
}

fn assert_narrow(pass: &Pass) {
  assert_eq!(pass.widths[2], 0.0, "the time is dropped: {pass:?}");
  assert_eq!(pass.drawn[0], COUNT, "the count stays whole: {pass:?}");
  assert!(!pass.drawn.iter().any(|text| text == TIME), "the time is not drawn");
}

#[test]
fn give_way_cache_narrow_then_wide_restores_natural_widths() {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_root(status_bar());
  let passes: Vec<Pass> = [WIDE, NARROW, WIDE, NARROW]
    .into_iter()
    .map(|width| {
      let layout = layout_at(&mut tree, &mut app, tight(width, LINE_HEIGHT));
      pass(&tree, &layout)
    })
    .collect();

  assert_natural(&passes[0]);
  assert_narrow(&passes[1]);
  assert_eq!(passes[2], passes[0], "the wide bar is restored");
  assert_eq!(passes[3], passes[1], "the narrow bar gives way the same again");
}

/// The window width, read by the bar so that it re-renders on every resize.
#[derive(Clone)]
struct WindowWidth(Signal<f32>);

impl PartialEq for WindowWidth {
  fn eq(&self, other: &Self) -> bool {
    self.0.id() == other.0.id()
  }
}

#[cfg(feature = "devtools")]
impl lurq::app::component::DevtoolsInspectable for WindowWidth {}

struct ResizingStatusBar;

impl Component for ResizingStatusBar {
  type Props = WindowWidth;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    ctx.props::<Self::Props>().0.get();
    status_bar()
  }
}

#[test]
fn give_way_cache_rerendered_bar_restores_natural_widths() {
  let mut app = App::new();
  let mut tree = Tree::new();
  let window_width = Signal::new(WIDE);
  tree.mount_root::<ResizingStatusBar>(&mut app, WindowWidth(window_width.clone()));
  let passes: Vec<Pass> = [WIDE, NARROW, WIDE]
    .into_iter()
    .map(|width| {
      window_width.set(width);
      let layout = layout_at(&mut tree, &mut app, tight(width, LINE_HEIGHT));
      pass(&tree, &layout)
    })
    .collect();

  assert_natural(&passes[0]);
  assert_narrow(&passes[1]);
  assert_eq!(passes[2], passes[0], "the wide bar is restored");
}

/// The bar in a vertical scroll area with a reserved gutter: each real layout
/// of the area lays the bar out at the full width, then inside the gutter.
fn bar_in_gutter_scroll() -> ScrollVertical {
  ScrollVertical::new(status_bar()).scrollbar(ScrollBarStyle {
    visible: ScrollBarVisibility::Always,
    placement: ScrollBarPlacement::Reserved,
    width: GUTTER,
    padding: 0.0,
    ..Default::default()
  })
}

/// One pass of the scroll area: the bar's widths and every text drawn.
fn scroll_pass(tree: &mut Tree, app: &mut App, width: f32) -> (f32, Pass) {
  let layout = layout_at(tree, app, Constraints::loose(Size::new(width, LINE_HEIGHT * 4.0)));
  let bar = &layout.children[0].result;
  let pass = Pass {
    widths: child_widths(bar),
    item_widths: child_widths(&bar.children[0].result),
    drawn: drawn_texts(tree, &layout),
  };
  (bar.size.width, pass)
}

#[test]
fn give_way_cache_reserved_gutter_gives_way_inside_the_viewport() {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_root(bar_in_gutter_scroll());

  let (natural_bar, natural) = scroll_pass(&mut tree, &mut app, WIDE);
  assert_natural(&natural);

  // Room for the whole bar but not for the gutter as well: only the time
  // (the first order) gives way.
  let width = natural_bar + GUTTER - 1.0;
  let (fitted_bar, fitted) = scroll_pass(&mut tree, &mut app, width);
  assert!(fitted_bar <= width - GUTTER, "the bar fits the viewport: {fitted_bar}");
  assert_eq!(fitted.widths[2], 0.0, "the time is dropped");
  assert_eq!(
    fitted.widths[..2],
    natural.widths[..2],
    "the item and the run label keep their widths"
  );
  assert_eq!(fitted.drawn, [COUNT, DETAIL, RUN]);

  let (again_bar, again) = scroll_pass(&mut tree, &mut app, WIDE);
  assert_eq!(again, natural, "the wide bar is restored");
  assert_close(again_bar, natural_bar, "the bar has its natural width again");
}
