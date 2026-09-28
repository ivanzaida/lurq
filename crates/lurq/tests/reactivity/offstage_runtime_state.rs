//! Runtime state (scroll offsets, text-input carets) of an offstage component
//! survives a round trip offstage and back, whether or not the app holds it.

use lurq::{
  DevtoolsInspectable,
  app::{App, Tree, component::Component, ctx::Ctx, events::ScrollPhase},
  components::{Column, Rect, ScrollVertical, TextInput},
  core::Signal,
  layout::layout_kind::ScrollState,
  node::{Element, color::Color},
};

#[derive(Clone, PartialEq, DevtoolsInspectable)]
struct PageProps {
  name: &'static str,
  /// Changing it re-renders the page through its parent (`mount_*` with new
  /// props), not through its own dirty refresh.
  revision: u32,
  #[devtools_ignore]
  held: Option<HeldScroll>,
}

/// An app-held `ScrollState`. The fixture passes the same one on every
/// render, so all of them compare equal.
#[derive(Clone)]
struct HeldScroll(ScrollState);

impl PartialEq for HeldScroll {
  fn eq(&self, _other: &Self) -> bool {
    true
  }
}

struct Page {
  value: Signal<String>,
  /// Written by the page's own signal: a dirty refresh of the page alone.
  local: Signal<u32>,
}

impl Component for Page {
  type Props = PageProps;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      value: ctx.signal("AB".to_owned()),
      local: ctx.signal(0),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<PageProps>().clone();
    let _ = self.local.get();
    let mut rows = Column::new();
    for index in 0..20 {
      rows = rows.child(
        Rect::new(100.0, 50.0)
          .id(format!("{}-row-{index}", props.name))
          .background(Color::new(index as u8, 0, 0, 255)),
      );
    }
    let scroll = match props.held {
      Some(HeldScroll(state)) => ScrollVertical::new(rows).with_scroll_state(state),
      None => ScrollVertical::new(rows),
    };
    Column::new()
      .child(scroll.id(format!("{}-scroll", props.name)).height(200.0))
      .child(TextInput::new(self.value.clone()).id(format!("{}-input", props.name)))
  }
}

#[derive(Clone, DevtoolsInspectable)]
struct HostProps {
  #[devtools_ignore]
  active: Signal<&'static str>,
  #[devtools_ignore]
  revision: Signal<u32>,
  #[devtools_ignore]
  held: Option<HeldScroll>,
}

impl PartialEq for HostProps {
  fn eq(&self, other: &Self) -> bool {
    self.active.id() == other.active.id() && self.revision.id() == other.revision.id() && self.held == other.held
  }
}

struct Host;

impl Component for Host {
  type Props = HostProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<HostProps>().clone();
    let active = props.active.get();
    let revision = props.revision.get();
    let mut column = Column::new();
    for name in ["a", "b"] {
      let page = PageProps {
        name,
        revision,
        held: if name == "a" { props.held.clone() } else { None },
      };
      column = column.child(ctx.mount_keyed_offstage::<Page>(name, page, active == name));
    }
    column
  }
}

struct Fixture {
  tree: Tree,
  app: App,
  active: Signal<&'static str>,
  revision: Signal<u32>,
}

impl Fixture {
  fn new(held: Option<ScrollState>) -> Self {
    let active = Signal::new("a");
    let revision = Signal::new(0);
    let mut app = App::new();
    let mut tree = Tree::new();
    tree.mount_root::<Host>(
      &mut app,
      HostProps {
        active: active.clone(),
        revision: revision.clone(),
        held: held.map(HeldScroll),
      },
    );
    let mut fixture = Self {
      tree,
      app,
      active,
      revision,
    };
    fixture.pass();
    fixture.pass();
    fixture
  }

  fn pass(&mut self) {
    self.tree.request_redraw();
    self.tree.pass_headless(&mut self.app);
  }

  fn switch_to(&mut self, name: &'static str) {
    self.active.set(name);
    self.pass();
    self.pass();
  }

  fn top(&mut self, id: &str) -> f32 {
    self
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.bounds())
      .unwrap_or_else(|| panic!("#{id} should be laid out"))
      .y
  }

  /// How far page `a`'s scroll area is scrolled, from its first row's position.
  fn scroll_of_a(&mut self) -> f32 {
    self.top("a-scroll") - self.top("a-row-0")
  }

  fn scroll_a_by(&mut self, delta: f32) {
    let top = self.top("a-scroll");
    self.tree.scroll(10.0, top + 10.0, 0.0, -delta, ScrollPhase::Scroll);
    self.pass();
    self.pass();
  }

  fn key(&mut self, key: &str, code: &str) {
    self.tree.key_down(key.to_owned(), code.to_owned(), false, false, false);
  }
}

#[test]
fn offstage_round_trip_keeps_the_scroll_offset() {
  let mut fixture = Fixture::new(None);
  fixture.scroll_a_by(150.0);
  assert_eq!(fixture.scroll_of_a(), 150.0);

  fixture.switch_to("b");
  assert!(
    fixture.tree.get_element_by_id_mut("a-row-0").is_none(),
    "page a is offstage"
  );
  fixture.switch_to("a");

  assert_eq!(fixture.scroll_of_a(), 150.0);
}

#[test]
fn scroll_offset_survives_parent_rerenders_before_going_offstage() {
  let mut fixture = Fixture::new(None);
  fixture.scroll_a_by(150.0);
  // The parent re-renders the active page with new props; the retained tree
  // keeps the offset while the page stays active.
  fixture.revision.set(1);
  fixture.pass();
  assert_eq!(fixture.scroll_of_a(), 150.0);
  fixture.scroll_a_by(50.0);
  fixture.revision.set(2);
  fixture.pass();
  assert_eq!(fixture.scroll_of_a(), 200.0);

  fixture.switch_to("b");
  fixture.switch_to("a");

  assert_eq!(fixture.scroll_of_a(), 200.0);
}

#[test]
fn scroll_offset_survives_repeated_round_trips_and_offstage_prop_changes() {
  let mut fixture = Fixture::new(None);
  fixture.scroll_a_by(100.0);
  for (round, delta) in [(1, 50.0), (2, 100.0), (3, -200.0)] {
    let expected = fixture.scroll_of_a();
    fixture.switch_to("b");
    // Props change while the page is offstage: it re-renders on resume.
    fixture.revision.set(round);
    fixture.pass();
    fixture.switch_to("a");
    assert_eq!(fixture.scroll_of_a(), expected, "round {round}");
    fixture.scroll_a_by(delta);
  }
  assert_eq!(fixture.scroll_of_a(), 50.0);
}

#[test]
fn scroll_offset_survives_the_pages_own_rerenders_before_going_offstage() {
  let mut fixture = Fixture::new(None);
  fixture.scroll_a_by(150.0);
  fixture.revision.set(1);
  fixture.pass();
  // A dirty refresh of the page alone, after a parent-driven render.
  let top = fixture.top("a-input");
  let (x, y) = (10.0, top + 5.0);
  crate::support::pointer_click(&mut fixture.tree, x, y, lurq::app::events::MouseButton::Left);
  fixture.key("C", "KeyC");
  fixture.pass();
  assert_eq!(fixture.scroll_of_a(), 150.0);

  fixture.switch_to("b");
  fixture.switch_to("a");

  assert_eq!(fixture.scroll_of_a(), 150.0);
}

#[test]
fn an_app_held_scroll_state_keeps_its_offset_offstage() {
  let held = ScrollState::new();
  let mut fixture = Fixture::new(Some(held.clone()));
  fixture.scroll_a_by(150.0);
  fixture.revision.set(1);
  fixture.pass();
  assert_eq!(held.scroll_y(), 150.0);

  fixture.switch_to("b");
  fixture.switch_to("a");

  assert_eq!(fixture.scroll_of_a(), 150.0);
  assert_eq!(held.scroll_y(), 150.0);
  fixture.scroll_a_by(30.0);
  assert_eq!(held.scroll_y(), 180.0, "the held state stays the live one");
}

#[test]
fn offstage_round_trip_keeps_a_text_input_caret() {
  let mut fixture = Fixture::new(None);
  fixture.revision.set(1);
  fixture.pass();
  fixture.tree.get_element_by_id_mut("a-input").expect("input").focus();
  fixture.key("End", "End");
  fixture.key("ArrowLeft", "ArrowLeft");

  fixture.switch_to("b");
  fixture.switch_to("a");

  fixture.tree.get_element_by_id_mut("a-input").expect("input").focus();
  fixture.key("C", "KeyC");
  let value = fixture
    .tree
    .get_element_by_id_mut("a-input")
    .and_then(|element| element.as_text_input().map(|input| input.value()))
    .expect("text input");
  assert_eq!(value, "ACB");
}
