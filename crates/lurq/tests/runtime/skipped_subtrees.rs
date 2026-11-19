//! A frame walks only what has work for it: the refs it tells where their elements are, the layers it indexes, the
//! children a row drops, the transitions it ticks, what it paints. Each subtree knows whether it holds any of them,
//! and a subtree that holds none, or that ticked clean and has not changed, is passed by. These are the cases that
//! would break if a subtree's knowledge were stale: something of each kind appears deep in a subtree nothing touched
//! for frames, and is found, also after the subtree moved by a keyed reorder and after its parent was built again.

use std::{
  cell::RefCell,
  time::{Duration, Instant},
};

use lurq::{
  animation::{Easing, Transition},
  app::{App, Tree, component::Component, ctx::Ctx},
  components::{Button, Column, Popup, Rect, Row, ScrollVertical},
  core::{ElementRef, Signal},
  layout::{
    layout_kind::{ScrollState, ShrinkLimit},
    quad::QuadContent,
  },
  node::{Element, Style, color::Color},
};

use crate::support::render_pass_with_app;

const REST: &str = "#334155";
const LIT: &str = "#ef4444";
const HOVER: &str = "#22c55e";

/// What the deep part of the scene shows.
#[derive(Clone, Copy, PartialEq)]
enum Stage {
  Nothing,
  Ref,
  Layer,
  Dropping,
  Fades,
  /// A button whose fill heads to another on its own focus.
  Focused,
  /// An interaction group: the pointer on one of what it holds lights the `within` style of another.
  Within,
  /// An interaction group whose button takes the focus: the focus in the group lights the `within` style of what
  /// else it holds.
  FocusWithin,
}

#[derive(Clone)]
struct Scene {
  stage: Signal<u8>,
  /// The deep part after its sibling instead of before it: a keyed reorder.
  swapped: Signal<bool>,
  /// Builds the parent of the deep part again.
  rebuilt: Signal<u32>,
  lit: Signal<bool>,
  /// The deep part out of the window's view, where nothing paints it: only the walks tell its ref.
  far: bool,
  deep: ElementRef,
  anchor: ElementRef,
}

thread_local! {
  static SCENE: RefCell<Option<Scene>> = const { RefCell::new(None) };
}

fn scene() -> Scene {
  SCENE.with(|scene| scene.borrow().clone().expect("a scene is set"))
}

fn stage_of(value: u8) -> Stage {
  [
    Stage::Nothing,
    Stage::Ref,
    Stage::Layer,
    Stage::Dropping,
    Stage::Fades,
    Stage::Focused,
    Stage::Within,
    Stage::FocusWithin,
  ][usize::from(value)]
}

/// The deep part: it alone renders when the stage changes.
struct Deep;

impl Component for Deep {
  type Props = ();

  fn create(_: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, _: &mut Ctx) -> impl Into<Element> {
    let scene = scene();
    let element: Element = match stage_of(scene.stage.get()) {
      Stage::Nothing => Rect::new(10.0, 10.0).into(),
      Stage::Ref => Rect::new(20.0, 20.0).ref_element(scene.deep.clone()).into(),
      Stage::Layer => Column::new()
        .child(Rect::new(30.0, 10.0).ref_element(scene.anchor.clone()))
        .child(Popup::new(scene.anchor.clone(), Rect::new(50.0, 20.0).id("layer")).open(Signal::new(true)))
        .into(),
      Stage::Dropping => {
        let tool = |id: &str| {
          Rect::new(60.0, 20.0)
            .id(id)
            .tab_index(0)
            .flex_shrink(1.0)
            .shrink_limit(ShrinkLimit::Drop)
        };
        Row::new()
          .width(100.0)
          .child(tool("first"))
          .child(tool("second"))
          .child(tool("third"))
          .into()
      }
      Stage::Fades => Rect::new(100.0, 40.0)
        .id("fades")
        .background(if scene.lit.get() { LIT } else { REST })
        .transition(Transition::background_color().duration_ms(400).easing(Easing::Linear))
        .hovered(|style| style.background(HOVER))
        .into(),
      Stage::Focused => Button::new("Go")
        .id("go")
        .size(100.0, 40.0)
        .tab_index(0)
        .background(REST)
        .transition(Transition::background_color().duration_ms(400).easing(Easing::Linear))
        .focused(|style| style.background(HOVER))
        .into(),
      Stage::Within => Row::new()
        .interaction_group(true)
        .child(Rect::new(60.0, 40.0).id("hit").background(LIT))
        .child(fading_within("fades"))
        .into(),
      Stage::FocusWithin => Row::new()
        .interaction_group(true)
        .child(Button::new("Go").id("go").size(60.0, 40.0).tab_index(0))
        .child(fading_within("fades"))
        .into(),
    };
    element
  }
}

/// A part of an interaction group that fades to `HOVER` while the group is lit: under the pointer, or holding the
/// focus.
fn fading_within(id: &str) -> Rect {
  Rect::new(100.0, 40.0)
    .id(id.to_owned())
    .background(REST)
    .transition(Transition::background_color().duration_ms(400).easing(Easing::Linear))
    .within_style(Style::new().background(HOVER))
}

/// Whether `fills` holds a color halfway from `REST` to `HOVER`.
fn halfway_to_hover(fills: &[Color]) -> bool {
  let (rest, hover) = (Color::from_hex(REST), Color::from_hex(HOVER));
  fills
    .iter()
    .any(|color| *color != rest && *color != hover && color.g() > rest.g() && color.a() == 255)
}

/// A long untouched page with transitions of its own, and the deep part three columns down.
struct Page;

impl Component for Page {
  type Props = ();

  fn create(_: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let scene = scene();
    let _ = scene.rebuilt.get();
    let filler = Column::new().with_children((0..60).map(|row| {
      Rect::new(300.0, 4.0)
        .key(format!("row-{row}"))
        .background(REST)
        .transition(Transition::background_color().duration_ms(300))
    }));
    let deep = Column::new()
      .key("deep")
      .child(Column::new().child(Column::new().child(ctx.mount::<Deep>(()))));
    let sibling = Column::new().key("sibling").child(Rect::new(10.0, 10.0));
    let (first, second) = if scene.swapped.get() {
      (sibling, deep)
    } else {
      (deep, sibling)
    };
    let gap = if scene.far { 1000.0 } else { 0.0 };
    Column::new()
      .child(filler)
      .child(Rect::new(10.0, gap))
      .child(first)
      .child(second)
  }
}

struct Fixture {
  app: App,
  tree: Tree,
  scene: Scene,
  now: Instant,
}

impl Fixture {
  /// The page at `stage`, drawn for a few frames: every subtree ticked clean, every walk settled.
  fn new(stage: Stage) -> Self {
    Self::placed(stage, false)
  }

  /// As [`Self::new`], the deep part out of the window's view where `far`.
  fn placed(stage: Stage, far: bool) -> Self {
    let scene = Scene {
      far,
      stage: Signal::new(stage as u8),
      swapped: Signal::new(false),
      rebuilt: Signal::new(0),
      lit: Signal::new(false),
      deep: ElementRef::new(),
      anchor: ElementRef::new(),
    };
    SCENE.with(|slot| *slot.borrow_mut() = Some(scene.clone()));
    let mut app = App::new();
    let mut tree = Tree::new();
    tree.resize(400, 600);
    tree.mount_root::<Page>(&mut app, ());
    let mut fixture = Self {
      app,
      tree,
      scene,
      now: Instant::now(),
    };
    fixture.frames(5);
    fixture
  }

  /// `count` frames, a sixtieth of a second apart.
  fn frames(&mut self, count: usize) {
    for _ in 0..count {
      self.tree.set_animation_clock(Some(self.now));
      render_pass_with_app(&mut self.tree, &mut self.app);
      self.now += Duration::from_millis(16);
    }
  }

  /// What happens to the deep part before the case: nothing, a keyed reorder, its parent built again.
  fn prepare(&mut self, how: How) {
    match how {
      How::Untouched => {}
      How::Reordered => self.scene.swapped.set(true),
      How::Rebuilt => self.scene.rebuilt.update(|count| *count += 1),
    }
    self.frames(5);
  }

  fn bounds(&mut self, id: &str) -> Option<(f32, f32)> {
    self
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.bounds())
      .map(|rect| (rect.width, rect.height))
  }

  fn tab(&mut self) -> Option<String> {
    self
      .tree
      .key_down("Tab".to_owned(), "Tab".to_owned(), false, false, false);
    self
      .tree
      .key_up("Tab".to_owned(), "Tab".to_owned(), false, false, false);
    self.frames(1);
    self
      .tree
      .focused_element()
      .and_then(|element| element.id().map(str::to_owned))
  }

  fn fills(&mut self) -> Vec<Color> {
    self.tree.set_animation_clock(Some(self.now));
    let frame = render_pass_with_app(&mut self.tree, &mut self.app);
    frame.rects.iter().map(|rect| rect.color).collect()
  }
}

#[derive(Clone, Copy, Debug)]
enum How {
  Untouched,
  Reordered,
  Rebuilt,
}

const HOWS: [How; 3] = [How::Untouched, How::Reordered, How::Rebuilt];

#[test]
fn a_ref_added_deep_in_an_untouched_subtree_is_told_where_its_element_is() {
  for how in HOWS {
    let mut fixture = Fixture::placed(Stage::Nothing, true);
    fixture.prepare(how);
    fixture.scene.stage.set(Stage::Ref as u8);
    fixture.frames(2);
    let bounds = fixture.scene.deep.bounds();
    assert!(
      fixture.scene.deep.is_attached() && bounds.width == 20.0 && bounds.height == 20.0,
      "{how:?}: the ref has its element's box: {bounds:?}"
    );
  }
}

#[test]
fn a_layer_opened_deep_in_an_untouched_subtree_is_indexed_and_drawn() {
  for how in HOWS {
    let mut fixture = Fixture::new(Stage::Nothing);
    fixture.prepare(how);
    fixture.scene.stage.set(Stage::Layer as u8);
    fixture.frames(3);
    assert_eq!(
      fixture.bounds("layer"),
      Some((50.0, 20.0)),
      "{how:?}: the popup is laid out over the tree"
    );
  }
}

#[test]
fn a_child_dropped_deep_in_an_untouched_subtree_is_known_to_tab() {
  for how in HOWS {
    let mut fixture = Fixture::new(Stage::Nothing);
    fixture.prepare(how);
    fixture.scene.stage.set(Stage::Dropping as u8);
    fixture.frames(3);
    assert_eq!(
      fixture.bounds("third"),
      Some((0.0, 0.0)),
      "{how:?}: the row drops what does not fit"
    );
    assert_eq!(fixture.tab().as_deref(), Some("first"), "{how:?}");
    let next = fixture.tab();
    assert!(
      next.as_deref() != Some("second") && next.as_deref() != Some("third"),
      "{how:?}: Tab passes the dropped by: {next:?}"
    );
  }
}

#[test]
fn a_transition_started_deep_in_a_subtree_ticked_clean_runs() {
  for how in HOWS {
    let mut fixture = Fixture::new(Stage::Fades);
    fixture.prepare(how);
    // Clean for frames: then its color changes by its state, with no layout.
    fixture.scene.lit.set(true);
    fixture.frames(1);
    fixture.now += Duration::from_millis(200);
    let halfway = fixture.fills();
    let (rest, lit) = (Color::from_hex(REST), Color::from_hex(LIT));
    assert!(
      !halfway.contains(&rest) || !halfway.contains(&lit),
      "{how:?}: the fill is between its colors halfway"
    );
    assert!(
      halfway
        .iter()
        .any(|color| *color != rest && *color != lit && color.a() == 255 && color.r() > rest.r()),
      "{how:?}: a color between {REST} and {LIT}: {halfway:?}"
    );
  }
}

#[test]
fn a_hover_transition_deep_in_a_subtree_ticked_clean_runs() {
  for how in HOWS {
    let mut fixture = Fixture::new(Stage::Fades);
    fixture.prepare(how);
    let center = fixture
      .tree
      .get_element_by_id_mut("fades")
      .and_then(|element| element.bounds())
      .expect("laid out")
      .center();
    fixture.tree.mouse_move(center.0, center.1);
    fixture.frames(1);
    fixture.now += Duration::from_millis(200);
    let halfway = fixture.fills();
    let (rest, hover) = (Color::from_hex(REST), Color::from_hex(HOVER));
    assert!(
      halfway
        .iter()
        .any(|color| *color != rest && *color != hover && color.g() > rest.g() && color.a() == 255),
      "{how:?}: a color between {REST} and {HOVER} halfway: {halfway:?}"
    );
  }
}

/// A column far down a scroll view whose child reaches out of its box: the column's box is out of view, what its
/// child draws past it is not.
struct Overflowing;

thread_local! {
  static SCROLL: RefCell<Option<ScrollState>> = const { RefCell::new(None) };
}

impl Component for Overflowing {
  type Props = ();

  fn create(_: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, _: &mut Ctx) -> impl Into<Element> {
    let state = SCROLL.with(|slot| slot.borrow().clone().expect("a scroll state"));
    let mut content = Column::new();
    for index in 0..200 {
      content = content.child(Rect::new(300.0, 20.0).background(REST).key(format!("row-{index}")));
    }
    // A column 10 high whose child is 200 high and visible past it.
    content = content.child(
      Column::new()
        .height(10.0)
        .overflow_visible()
        .child(Rect::new(300.0, 200.0).background(LIT)),
    );
    for index in 0..200 {
      content = content.child(Rect::new(300.0, 20.0).background(REST).key(format!("after-{index}")));
    }
    ScrollVertical::new(content).with_scroll_state(state).size(400.0, 300.0)
  }
}

#[test]
fn what_a_subtree_draws_past_its_box_is_drawn_while_its_box_is_out_of_view() {
  let state = ScrollState::new();
  SCROLL.with(|slot| *slot.borrow_mut() = Some(state.clone()));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.resize(400, 300);
  tree.mount_root::<Overflowing>(&mut app, ());
  render_pass_with_app(&mut tree, &mut app);
  // The column's box is at 4,000 to 4,010; its child runs to 4,200. Scrolled to 4,100, only the child is in view.
  state.set_scroll(0.0, 4100.0);
  tree.request_redraw();
  let frame = render_pass_with_app(&mut tree, &mut app);
  assert!(
    frame.rects.iter().any(|rect| rect.color == Color::from_hex(LIT)),
    "the child past its column's box is drawn"
  );
  let quads = tree.resolve_quads(tree.last_layout().expect("laid out"));
  let drawn = quads
    .iter()
    .filter(|quad| matches!(quad.content, QuadContent::Rect { .. }))
    .count();
  assert!(
    drawn < 40,
    "the rest of the 400 rows out of view are culled: {drawn} drawn"
  );
}

// A change of a state of interaction marks the way to what changed, not the tree: what styles itself by it heads to
// its new look, also deep in a page ticked clean for frames and after a keyed reorder or a rebuild of its parent.

#[test]
fn a_focus_transition_deep_in_a_subtree_ticked_clean_runs() {
  for how in HOWS {
    let mut fixture = Fixture::new(Stage::Focused);
    fixture.prepare(how);
    assert_eq!(
      fixture.tab().as_deref(),
      Some("go"),
      "{how:?}: Tab takes the focus to the button"
    );
    fixture.now += Duration::from_millis(200);
    let halfway = fixture.fills();
    assert!(halfway_to_hover(&halfway), "{how:?}: its own focus: {halfway:?}");
  }
}

#[test]
fn a_within_transition_lit_by_the_pointer_on_its_group_runs() {
  for how in HOWS {
    let mut fixture = Fixture::new(Stage::Within);
    fixture.prepare(how);
    let center = fixture
      .tree
      .get_element_by_id_mut("hit")
      .and_then(|element| element.bounds())
      .expect("laid out")
      .center();
    // On what else the group holds: the part that fades is lit by its group, not by its own state.
    fixture.tree.mouse_move(center.0, center.1);
    fixture.frames(1);
    fixture.now += Duration::from_millis(200);
    let halfway = fixture.fills();
    assert!(halfway_to_hover(&halfway), "{how:?}: lit by its group: {halfway:?}");
  }
}

#[test]
fn a_within_transition_lit_by_the_focus_in_its_group_runs() {
  for how in HOWS {
    let mut fixture = Fixture::new(Stage::FocusWithin);
    fixture.prepare(how);
    assert_eq!(
      fixture.tab().as_deref(),
      Some("go"),
      "{how:?}: Tab takes the focus into the group"
    );
    fixture.now += Duration::from_millis(200);
    let halfway = fixture.fills();
    assert!(
      halfway_to_hover(&halfway),
      "{how:?}: lit by the focus in its group: {halfway:?}"
    );
  }
}
