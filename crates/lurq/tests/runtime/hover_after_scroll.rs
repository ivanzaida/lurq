//! Hover and anchored overlays under a pointer that holds still while the
//! content moves: a wheel scroll, or a relayout that pushes a row away. The
//! hover is hit-tested again after the layout, so a tooltip shown on hover
//! goes when its row leaves the pointer and follows the row while it stays
//! under it; an overlay whose anchor is scrolled under the header above the
//! list is not drawn over that header.

use std::{
  sync::{Arc, Mutex},
  time::Duration,
};

use lurq::{
  app::{
    App, Tree,
    component::Component,
    ctx::{CollisionStrategy, Ctx, Overlay, Placement},
    events::{InputSource, MouseButton, ScrollPhase, input_source, with_synthetic_input},
  },
  components::{Column, Rect, ScrollVertical},
  core::{ElementRect, ElementRef, Signal},
  node::{CursorIcon, Element, HitTestBehavior, color::Color},
};

use crate::support::TestSurface;

const HEADER: &str = "#1f2937";
const ROW: &str = "#22c55e";
const TOOLTIP: &str = "#ef4444";
const FILLER: &str = "#111827";
const PUSHED_IN: &str = "#334155";
const PUSHED_IN_HEIGHT: f32 = 100.0;
/// The header above the list is 40 px tall; the list's viewport starts there.
const HEADER_HEIGHT: f32 = 40.0;
/// The row sits 60 px into the list's content, at y 100 in the window.
const ROW_TOP: f32 = 60.0;
const GAP: f32 = 4.0;
/// Longer than the double-click interval, after which a held click is delivered.
const DOUBLE_CLICK_SETTLE: Duration = Duration::from_millis(550);

/// The source of the input each enter and leave of the row ran with.
type Sources = Arc<Mutex<Vec<(&'static str, InputSource)>>>;

/// What the test drives: the row's height, whether its tooltip shows only
/// while the row is hovered (or always, as a tooltip shown for keyboard focus
/// does), and whether the block above the row has grown.
#[derive(Clone, lurq::DevtoolsInspectable)]
struct Props {
  row_height: f32,
  on_hover_only: bool,
  pushed_in: Signal<bool>,
  hovered: Arc<Signal<bool>>,
  #[devtools_ignore]
  sources: Sources,
}

impl PartialEq for Props {
  fn eq(&self, other: &Self) -> bool {
    self.row_height == other.row_height && self.on_hover_only == other.on_hover_only
  }
}

impl std::fmt::Debug for Props {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Props").field("row_height", &self.row_height).finish()
  }
}

/// A header, and under it a scrolling list with one row whose tooltip is an
/// overlay anchored under it that lets the pointer through.
struct HeaderAndList {
  anchor: ElementRef,
}

impl Component for HeaderAndList {
  type Props = Props;

  fn create(_ctx: &mut Ctx) -> Self {
    Self {
      anchor: ElementRef::new(),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Props>().clone();
    let (enter, leave) = (props.hovered.clone(), props.hovered.clone());
    let (enter_sources, leave_sources) = (props.sources.clone(), props.sources.clone());
    // A double-click handler makes a click wait to see whether a second
    // one follows.
    let row = Rect::new(120.0, props.row_height)
      .background(ROW)
      .cursor(CursorIcon::Pointer)
      .ref_element(self.anchor.clone())
      .on_dblclick(|_| {})
      .on_mouse_enter(move || {
        enter_sources.lock().unwrap().push(("enter", input_source()));
        enter.set(true);
      })
      .on_mouse_leave(move || {
        leave_sources.lock().unwrap().push(("leave", input_source()));
        leave.set(false);
      });
    // Growing the block above the row moves the row without changing which
    // node is which.
    let above = if props.pushed_in.get() {
      Rect::new(300.0, ROW_TOP + PUSHED_IN_HEIGHT).background(PUSHED_IN)
    } else {
      Rect::new(300.0, ROW_TOP).background(FILLER)
    };
    let content = Column::new()
      .child(above)
      .child(row)
      .child(Rect::new(300.0, 600.0).background(FILLER));
    let open = !props.on_hover_only || props.hovered.get();
    Column::new()
      .child(Rect::new(300.0, HEADER_HEIGHT).background(HEADER))
      .child(ScrollVertical::new(content).size(300.0, 200.0))
      .child(
        Overlay::new(Rect::new(80.0, 16.0).background(TOOLTIP))
          .anchor(self.anchor.clone())
          .open_when(open)
          .placement(Placement::BottomStart)
          .offset(0.0, GAP)
          .collision(CollisionStrategy::FlipThenClamp)
          .hit_test(HitTestBehavior::None),
      )
  }
}

struct Fixture {
  app: App,
  tree: Tree,
  hovered: Arc<Signal<bool>>,
  pushed_in: Signal<bool>,
  sources: Sources,
}

impl Fixture {
  fn mount(row_height: f32, on_hover_only: bool) -> Self {
    let hovered = Arc::new(Signal::new(false));
    let pushed_in = Signal::new(false);
    let sources = Sources::default();
    let mut app = App::new();
    let mut tree = Tree::new();
    tree.mount_root::<HeaderAndList>(
      &mut app,
      Props {
        row_height,
        on_hover_only,
        pushed_in: pushed_in.clone(),
        hovered: hovered.clone(),
        sources: sources.clone(),
      },
    );
    let mut fixture = Self {
      app,
      tree,
      hovered,
      pushed_in,
      sources,
    };
    fixture.pass();
    fixture
  }

  fn pass(&mut self) {
    self.tree.request_redraw();
    self.tree.pass(&mut self.app, &TestSurface);
  }

  fn bounds_of(&mut self, color: &str) -> Option<ElementRect> {
    let color = Color::from_hex(color);
    self
      .tree
      .find_element(|element| element.color() == Some(color))
      .map(|element| element.bounds())
  }

  fn row(&mut self) -> ElementRect {
    self.bounds_of(ROW).expect("the row")
  }

  fn tooltip(&mut self) -> Option<ElementRect> {
    self.bounds_of(TOOLTIP)
  }

  /// Rests the pointer on the row, 10 px into it, and shows its tooltip.
  fn hover_row(&mut self) -> (f32, f32) {
    let row = self.row();
    let pointer = (row.x + 10.0, row.y + 10.0);
    self.tree.mouse_move(pointer.0, pointer.1);
    self.pass();
    assert!(self.hovered.get_untracked(), "the row is hovered");
    let tooltip = self.tooltip().expect("hovering the row shows its tooltip");
    assert_eq!(tooltip.y, row.y + row.height + GAP, "the tooltip sits under the row");
    pointer
  }

  /// Scrolls the list by `dy` with the wheel at `pointer`, which holds still,
  /// and draws one frame.
  fn scroll_by(&mut self, pointer: (f32, f32), dy: f32) {
    self.tree.scroll(pointer.0, pointer.1, 0.0, -dy, ScrollPhase::Scroll);
    self.pass();
  }
}

#[test]
fn scrolling_the_row_away_from_a_still_pointer_hides_its_tooltip_in_the_same_frame() {
  let mut fixture = Fixture::mount(20.0, true);
  let pointer = fixture.hover_row();

  fixture.scroll_by(pointer, 70.0);

  let row = fixture.row();
  assert!(
    row.y + row.height <= pointer.1,
    "the row moved up from under the pointer: {row:?}"
  );
  assert!(!fixture.hovered.get_untracked(), "the row got its mouse leave");
  assert_eq!(fixture.tooltip(), None, "its tooltip is gone");
}

#[test]
fn tooltip_follows_its_row_while_the_row_stays_under_the_still_pointer() {
  let mut fixture = Fixture::mount(60.0, true);
  let pointer = fixture.hover_row();

  fixture.scroll_by(pointer, 5.0);

  let row = fixture.row();
  assert!(
    row.y <= pointer.1 && pointer.1 <= row.y + row.height,
    "still under the pointer"
  );
  assert!(fixture.hovered.get_untracked(), "the row stays hovered");
  let tooltip = fixture.tooltip().expect("the tooltip stays");
  assert_eq!(tooltip.y, row.y + row.height + GAP, "and moves with the row");
}

#[test]
fn row_scrolled_under_the_header_takes_its_overlay_out_of_the_header() {
  // Shown regardless of the pointer, like a tooltip for keyboard focus.
  let mut fixture = Fixture::mount(20.0, false);
  let away = (250.0, 190.0);
  assert!(fixture.tooltip().is_some(), "shown while the row is in view");

  // Half the row shows under the header: the overlay stays, under the row
  // and inside the list.
  fixture.scroll_by(away, 70.0);
  let row = fixture.row();
  assert!(row.y < HEADER_HEIGHT && row.y + row.height > HEADER_HEIGHT, "{row:?}");
  let tooltip = fixture.tooltip().expect("shown while part of the row shows");
  assert!(tooltip.y >= HEADER_HEIGHT, "below the header: {tooltip:?}");

  // The whole row is under the header: the overlay is not drawn over it.
  fixture.scroll_by(away, 20.0);
  let row = fixture.row();
  assert!(row.y + row.height <= HEADER_HEIGHT, "{row:?}");
  assert_eq!(fixture.tooltip(), None, "not drawn over the header");

  fixture.scroll_by(away, -90.0);
  assert!(fixture.tooltip().is_some(), "back with the row");
}

#[test]
fn content_growing_above_the_row_moves_hover_off_it_without_pointer_movement() {
  let mut fixture = Fixture::mount(20.0, true);
  fixture.hover_row();

  fixture.pushed_in.set(true);
  fixture.pass();

  assert!(fixture.bounds_of(PUSHED_IN).is_some());
  assert!(!fixture.hovered.get_untracked(), "the row got its mouse leave");
  assert_eq!(fixture.tooltip(), None, "its tooltip is gone");
}

#[test]
fn pointer_that_left_the_window_hovers_nothing_the_content_moves_under_it() {
  let mut fixture = Fixture::mount(20.0, true);
  let pointer = fixture.hover_row();
  fixture.tree.mouse_leave_window();
  fixture.pass();
  assert!(!fixture.hovered.get_untracked());

  // The row goes away and comes back under where the pointer last was.
  fixture.pushed_in.set(true);
  fixture.pass();
  fixture.pushed_in.set(false);
  fixture.pass();

  let row = fixture.row();
  assert!(row.y <= pointer.1 && pointer.1 <= row.y + row.height);
  assert!(!fixture.hovered.get_untracked(), "nothing is hovered");
  assert_eq!(fixture.tooltip(), None);
}

#[test]
fn cursor_follows_the_row_that_scrolls_from_under_the_still_pointer() {
  let mut fixture = Fixture::mount(20.0, true);
  let pointer = fixture.hover_row();
  assert_eq!(fixture.tree.cursor(), CursorIcon::Pointer);

  fixture.scroll_by(pointer, 70.0);

  assert_eq!(fixture.tree.cursor(), CursorIcon::Default, "no mouse event needed");
}

#[test]
fn hover_moved_by_a_layout_keeps_the_source_of_the_input_that_put_the_pointer_there() {
  let mut fixture = Fixture::mount(20.0, true);
  let row = fixture.row();
  with_synthetic_input(|| fixture.tree.mouse_move(row.x + 10.0, row.y + 10.0));
  fixture.pass();

  fixture.pushed_in.set(true);
  fixture.pass();

  assert_eq!(
    *fixture.sources.lock().unwrap(),
    [("enter", InputSource::Synthetic), ("leave", InputSource::Synthetic)],
    "an agent's pointer stays synthetic"
  );
}

#[test]
fn held_click_delivered_after_the_pointer_left_does_not_put_the_pointer_back() {
  let mut fixture = Fixture::mount(20.0, true);
  let pointer = fixture.hover_row();
  fixture.tree.mouse_down(pointer.0, pointer.1, MouseButton::Left);
  fixture.tree.mouse_up(pointer.0, pointer.1, MouseButton::Left);
  fixture.tree.mouse_leave_window();
  fixture.pass();
  assert!(!fixture.hovered.get_untracked());

  std::thread::sleep(DOUBLE_CLICK_SETTLE);
  fixture.pass();
  assert!(!fixture.hovered.get_untracked(), "the held click left nothing hovered");

  // The row goes away and comes back under where the pointer was.
  fixture.pushed_in.set(true);
  fixture.pass();
  fixture.pushed_in.set(false);
  fixture.pass();
  assert!(
    !fixture.hovered.get_untracked(),
    "the pointer is still out of the window"
  );
}
