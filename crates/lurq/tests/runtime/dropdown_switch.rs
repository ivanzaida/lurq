//! A press outside an open dropdown that lands on another dropdown's trigger
//! (a `Select`, or the anchor of a popup that dismisses on outside presses)
//! closes the open one and reaches that trigger, so one press switches
//! dropdowns, like a menu bar.

use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx, events::MouseButton},
  components::{Button, Column, Placement, Popup, Row, Select, Spacer},
  core::{ElementRef, Signal},
  node::Element,
};

use crate::support::pointer_click;

#[derive(Default)]
struct Counts {
  behind_presses: AtomicUsize,
  behind_clicks: AtomicUsize,
}

/// A select and its value, which stays across renders so the select keeps
/// its open menu when the page renders again.
#[derive(Clone)]
struct SelectField {
  value: Signal<String>,
  element: ElementRef,
}

impl SelectField {
  fn new() -> Self {
    Self {
      value: Signal::new("one".to_owned()),
      element: ElementRef::new(),
    }
  }

  fn render(&self, prefix: &str) -> Select<String> {
    Select::new(self.value.clone())
      .options([
        ("one".to_owned(), format!("{prefix} One")),
        ("two".to_owned(), format!("{prefix} Two")),
      ])
      .width(150.0)
      .height(30.0)
      .ref_element(self.element.clone())
  }
}

#[derive(Clone, lurq::DevtoolsInspectable)]
struct Props {
  #[devtools_ignore]
  popover_a: Signal<bool>,
  #[devtools_ignore]
  popover_b: Signal<bool>,
  #[devtools_ignore]
  select_a: SelectField,
  #[devtools_ignore]
  select_b: SelectField,
  #[devtools_ignore]
  counts: Arc<Counts>,
}

impl PartialEq for Props {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.counts, &other.counts)
  }
}

struct Page {
  anchor_a: ElementRef,
  anchor_b: ElementRef,
}

fn popover(id: &str, anchor: &ElementRef, open: &Signal<bool>) -> (Button, Popup) {
  let toggle = open.clone();
  let trigger = Button::new(id)
    .id(format!("anchor-{id}"))
    .width(150.0)
    .height(30.0)
    .ref_element(anchor.clone())
    .on_click(move |_| toggle.update(|open| *open = !*open));
  let content = Button::new("Inside").id(format!("inside-{id}"));
  let popup = Popup::new(anchor.clone(), content)
    .open(open.clone())
    .placement(Placement::BottomStart)
    .dismiss_on_outside_click(true);
  (trigger, popup)
}

impl Component for Page {
  type Props = Props;

  fn create(_ctx: &mut Ctx) -> Self {
    Self {
      anchor_a: ElementRef::new(),
      anchor_b: ElementRef::new(),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Props>().clone();
    let (trigger_a, popup_a) = popover("a", &self.anchor_a, &props.popover_a);
    let (trigger_b, popup_b) = popover("b", &self.anchor_b, &props.popover_b);
    let pressed = props.counts.clone();
    let clicked = props.counts.clone();
    let behind = Button::new("Behind")
      .id("behind")
      .width(200.0)
      .height(40.0)
      .on_mouse_down(move |_| {
        pressed.behind_presses.fetch_add(1, Ordering::SeqCst);
      })
      .on_click(move |_| {
        clicked.behind_clicks.fetch_add(1, Ordering::SeqCst);
      });
    Column::new()
      .child(
        Row::new()
          .spacing(40.0)
          .child(props.select_a.render("A"))
          .child(props.select_b.render("B"))
          .child(trigger_a)
          .child(popup_a)
          .child(trigger_b)
          .child(popup_b),
      )
      .child(Spacer::new().height(300.0))
      .child(behind)
  }
}

struct Fixture {
  app: App,
  tree: Tree,
  popover_a: Signal<bool>,
  popover_b: Signal<bool>,
  select_a: SelectField,
  select_b: SelectField,
  counts: Arc<Counts>,
}

impl Fixture {
  fn new() -> Self {
    let popover_a = Signal::new(false);
    let popover_b = Signal::new(false);
    let select_a = SelectField::new();
    let select_b = SelectField::new();
    let counts = Arc::new(Counts::default());
    let mut app = App::new();
    let mut tree = Tree::new();
    tree.mount_root::<Page>(
      &mut app,
      Props {
        popover_a: popover_a.clone(),
        popover_b: popover_b.clone(),
        select_a: select_a.clone(),
        select_b: select_b.clone(),
        counts: counts.clone(),
      },
    );
    let mut fixture = Self {
      app,
      tree,
      popover_a,
      popover_b,
      select_a,
      select_b,
      counts,
    };
    fixture.pass();
    fixture
  }

  fn pass(&mut self) {
    for _ in 0..2 {
      self.tree.request_redraw();
      self.tree.pass_headless(&mut self.app);
    }
  }

  fn click_at(&mut self, (x, y): (f32, f32)) {
    pointer_click(&mut self.tree, x, y, MouseButton::Left);
    self.pass();
  }

  fn click(&mut self, id: &str) {
    let point = self
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.bounds())
      .unwrap_or_else(|| panic!("#{id} should be laid out"))
      .center();
    self.click_at(point);
  }

  fn click_select_a(&mut self) {
    self.click_at(self.select_a.element.bounds().center());
  }

  fn click_select_b(&mut self) {
    self.click_at(self.select_b.element.bounds().center());
  }

  fn menu_open(&mut self, prefix: &str) -> bool {
    let label = format!("{prefix} Two");
    self
      .tree
      .find_element(|element| element.text_content() == Some(label.as_str()))
      .is_some()
  }

  fn popover_visible(&mut self, id: &str) -> bool {
    self.tree.get_element_by_id_mut(&format!("inside-{id}")).is_some()
  }

  fn focused(&self) -> Option<String> {
    self
      .tree
      .focused_element()
      .and_then(|element| element.id().map(str::to_owned))
  }

  fn behind(&self) -> (usize, usize) {
    (
      self.counts.behind_presses.load(Ordering::SeqCst),
      self.counts.behind_clicks.load(Ordering::SeqCst),
    )
  }
}

#[test]
fn a_press_on_another_select_opens_it() {
  let mut fixture = Fixture::new();
  fixture.click_select_a();
  assert!(fixture.menu_open("A"));

  fixture.click_select_b();

  assert!(!fixture.menu_open("A"), "the first menu closes");
  assert!(fixture.menu_open("B"), "the same press opens the second menu");
}

#[test]
fn a_press_on_a_select_closes_a_popover_and_opens_the_select() {
  let mut fixture = Fixture::new();
  fixture.click("anchor-a");
  assert!(fixture.popover_visible("a"));

  fixture.click_select_b();

  assert!(!fixture.popover_a.get());
  assert!(!fixture.popover_visible("a"));
  assert!(fixture.menu_open("B"));
}

#[test]
fn a_press_on_a_popover_anchor_closes_a_select_and_opens_the_popover() {
  let mut fixture = Fixture::new();
  fixture.click_select_a();
  assert!(fixture.menu_open("A"));

  fixture.click("anchor-b");

  assert!(!fixture.menu_open("A"));
  assert!(fixture.popover_b.get());
  assert!(fixture.popover_visible("b"));
}

#[test]
fn a_press_on_another_popover_anchor_switches_popovers() {
  let mut fixture = Fixture::new();
  fixture.click("anchor-a");
  assert!(fixture.popover_visible("a"));

  fixture.click("anchor-b");

  assert!(!fixture.popover_a.get());
  assert!(!fixture.popover_visible("a"));
  assert!(fixture.popover_b.get());
  assert!(fixture.popover_visible("b"));
}

#[test]
fn a_press_on_empty_space_only_closes_the_dropdown() {
  let mut fixture = Fixture::new();
  fixture.click("anchor-a");
  fixture.click("behind");
  assert!(!fixture.popover_a.get());
  assert_eq!(fixture.behind(), (0, 0), "the popover's closing press is consumed");

  fixture.click_select_a();
  fixture.click("behind");
  assert!(!fixture.menu_open("A"));
  assert_eq!(fixture.behind(), (0, 0), "the select's closing press is consumed");

  fixture.click("behind");
  assert_eq!(fixture.behind(), (1, 1), "the next press reaches it");
}

#[test]
fn a_press_on_the_open_dropdowns_own_trigger_closes_it() {
  let mut fixture = Fixture::new();
  fixture.click("anchor-a");
  fixture.click("anchor-a");
  assert!(!fixture.popover_a.get());
  assert!(!fixture.popover_visible("a"), "the popover does not reopen");

  fixture.click_select_a();
  fixture.click_select_a();
  assert!(!fixture.menu_open("A"), "the menu does not reopen");
}

#[test]
fn switching_dropdowns_moves_focus_to_the_second_trigger() {
  let mut fixture = Fixture::new();
  fixture.click_select_a();
  assert!(fixture.select_a.element.focused());

  fixture.click_select_b();
  assert!(fixture.select_b.element.focused(), "the second select takes focus");
  assert!(!fixture.select_a.element.focused());

  fixture.click("anchor-a");
  assert_eq!(fixture.focused().as_deref(), Some("anchor-a"));

  fixture.click_select_a();
  assert!(
    fixture.select_a.element.focused(),
    "the select takes focus from the popover's anchor"
  );
}
