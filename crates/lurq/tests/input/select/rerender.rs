//! A select's open menu, keyboard highlight and focus stay with that select
//! when its parent re-renders: siblings inserted before it, other selects next
//! to it, its options or value changing while the menu is open.

use std::cell::RefCell;

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx, events::MouseButton},
  components::{Column, Select, Text},
  core::Signal,
  node::{Element, ElementRef},
};

use crate::support::{pointer_click, render_pass_with_app};

#[derive(Clone)]
struct Signals {
  banner: Signal<bool>,
  first: Signal<String>,
  second: Signal<String>,
  second_options: Signal<Vec<(String, String)>>,
  /// Puts each select in its own labelled column, like a form field.
  wrapped: bool,
}

thread_local! {
  static SIGNALS: RefCell<Option<Signals>> = const { RefCell::new(None) };
}

fn signals() -> Signals {
  SIGNALS.with(|signals| signals.borrow().clone().expect("fixture installed the signals"))
}

fn named_options(prefix: &str) -> Vec<(String, String)> {
  ["sm", "md", "lg"]
    .into_iter()
    .zip(["Small", "Medium", "Large"])
    .map(|(value, label)| (value.to_owned(), format!("{prefix} {label}")))
    .collect()
}

/// Two anonymous selects in a column; `banner` inserts a line before them,
/// the way a finished background check adds a note above a form.
struct TwoSelects;

impl Component for TwoSelects {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    let signals = signals();
    let mut column = Column::new().spacing(180.0);
    if signals.banner.get() {
      column = column.child(Text::new("Check finished"));
    }
    let first = Select::new(signals.first.clone())
      .options(named_options("First"))
      .width(200.0)
      .height(40.0);
    let second = Select::new(signals.second.clone())
      .options(signals.second_options.get())
      .width(200.0)
      .height(40.0);
    if signals.wrapped {
      column
        .child(Column::new().child(Text::new("First field")).child(first))
        .child(Column::new().child(Text::new("Second field")).child(second))
    } else {
      column.child(first).child(second)
    }
  }
}

struct Fixture {
  tree: Tree,
  app: App,
  signals: Signals,
}

impl Fixture {
  fn new() -> Self {
    Self::with_layout(false)
  }

  fn wrapped() -> Self {
    Self::with_layout(true)
  }

  fn with_layout(wrapped: bool) -> Self {
    let signals = Signals {
      banner: Signal::new(false),
      first: Signal::new("md".to_owned()),
      second: Signal::new("sm".to_owned()),
      second_options: Signal::new(named_options("Second")),
      wrapped,
    };
    SIGNALS.with(|slot| *slot.borrow_mut() = Some(signals.clone()));
    let mut app = App::new();
    let mut tree = Tree::new();
    tree.mount_root::<TwoSelects>(&mut app, ());
    let mut fixture = Self { tree, app, signals };
    fixture.pass();
    fixture
  }

  fn pass(&mut self) {
    render_pass_with_app(&mut self.tree, &mut self.app);
    render_pass_with_app(&mut self.tree, &mut self.app);
  }

  /// The centre of the `index`th select trigger, in tree order.
  fn trigger_center(&mut self, index: usize) -> (f32, f32) {
    let mut centers = Vec::new();
    for position in 0..=index {
      let element = self
        .tree
        .find_element(|el| el.tag_name() == "Select" && select_index(el) == Some(position))
        .expect("select present");
      centers.push(element.bounds().center());
    }
    centers[index]
  }

  fn open(&mut self, index: usize) {
    let (x, y) = self.trigger_center(index);
    pointer_click(&mut self.tree, x, y, MouseButton::Left);
    self.pass();
  }

  fn has_option(&mut self, label: &str) -> bool {
    self.tree.find_element(|el| el.text_content() == Some(label)).is_some()
  }

  fn press(&mut self, key: &str) {
    self.tree.key_down(key.to_owned(), key.to_owned(), false, false, false);
    self.pass();
  }

  /// The label of the focused select's trigger.
  fn focused_label(&mut self) -> Option<String> {
    let focused = self.tree.focused_element()?;
    (focused.tag_name() == "Select").then(|| trigger_label(&focused))
  }
}

/// Which select (0 = first) `el` is, by its trigger label.
fn select_index(el: ElementRef<'_>) -> Option<usize> {
  let label = trigger_label(&el);
  if label.starts_with("First") {
    Some(0)
  } else if label.starts_with("Second") {
    Some(1)
  } else {
    None
  }
}

fn trigger_label(el: &ElementRef<'_>) -> String {
  first_text(*el).unwrap_or_default()
}

fn first_text(el: ElementRef<'_>) -> Option<String> {
  el.text_content()
    .map(str::to_owned)
    .or_else(|| el.children().iter().find_map(first_text))
}

#[test]
fn open_menu_stays_on_its_select_when_a_sibling_is_inserted_before_it() {
  let mut fixture = Fixture::new();
  fixture.open(1);
  assert!(fixture.has_option("Second Large"), "the second select's menu opened");

  fixture.signals.banner.set(true);
  fixture.pass();

  assert!(
    fixture.has_option("Second Large"),
    "the second select's menu stays open after the insertion"
  );
  assert!(
    !fixture.has_option("First Large"),
    "the first select must not take over the open menu"
  );
  assert_eq!(fixture.focused_label().as_deref(), Some("Second Small"));
}

#[test]
fn escape_after_a_relayout_closes_the_select_that_was_open() {
  let mut fixture = Fixture::new();
  fixture.open(1);
  fixture.signals.banner.set(true);
  fixture.pass();

  fixture.press("Escape");

  assert!(!fixture.has_option("Second Large"), "Escape closed the open menu");
  assert!(!fixture.has_option("First Large"), "no other select is left open");
  assert_eq!(fixture.focused_label().as_deref(), Some("Second Small"));
}

#[test]
fn keyboard_highlight_stays_with_its_select_across_a_sibling_insertion() {
  let mut fixture = Fixture::new();
  fixture.open(1);
  fixture.press("ArrowDown");
  fixture.press("ArrowDown");
  fixture.signals.banner.set(true);
  fixture.pass();

  fixture.press("Enter");

  assert_eq!(
    fixture.signals.second.get(),
    "md",
    "Enter commits the second select's highlight"
  );
  assert_eq!(fixture.signals.first.get(), "md", "the first select is untouched");
}

#[test]
fn removing_a_sibling_before_an_open_select_keeps_it_open() {
  let mut fixture = Fixture::new();
  fixture.signals.banner.set(true);
  fixture.pass();
  fixture.open(1);

  fixture.signals.banner.set(false);
  fixture.pass();

  assert!(fixture.has_option("Second Large"));
  assert!(!fixture.has_option("First Large"));
}

#[test]
fn pointer_move_over_the_menu_after_a_rerender_selects_nothing() {
  let mut fixture = Fixture::new();
  fixture.open(1);
  fixture.signals.banner.set(true);
  fixture.pass();

  let (x, y) = fixture
    .tree
    .find_element(|el| el.text_content() == Some("Second Large"))
    .expect("menu still open")
    .bounds()
    .center();
  fixture.tree.mouse_move(x, y);
  fixture.pass();
  fixture.tree.mouse_move(x, y + 1.0);
  fixture.pass();

  assert_eq!(fixture.signals.first.get(), "md");
  assert_eq!(fixture.signals.second.get(), "sm");
  assert!(fixture.has_option("Second Large"), "hovering keeps the menu open");
}

#[test]
fn replacing_the_options_of_an_open_select_keeps_it_open_and_selects_nothing() {
  let mut fixture = Fixture::new();
  fixture.open(1);
  fixture.press("ArrowDown");
  fixture.press("ArrowDown");

  fixture.signals.second_options.set(named_options("Other")[..2].to_vec());
  fixture.pass();

  assert!(fixture.has_option("Other Medium"), "the menu shows the new options");
  assert_eq!(fixture.signals.second.get(), "sm", "replacing options commits nothing");
  assert_eq!(fixture.signals.first.get(), "md");

  fixture.press("Enter");
  assert_eq!(
    fixture.signals.second.get(),
    "sm",
    "the highlight named a replaced option, so Enter commits nothing"
  );
  assert!(
    !fixture.has_option("Other Medium"),
    "Enter without a highlight closes the menu"
  );
}

#[test]
fn highlight_survives_a_rerender_that_keeps_the_options() {
  let mut fixture = Fixture::new();
  fixture.open(1);
  fixture.press("ArrowDown");
  fixture.press("ArrowDown");

  fixture.signals.second_options.set(named_options("Second"));
  fixture.pass();
  fixture.press("Enter");

  assert_eq!(fixture.signals.second.get(), "md");
}

#[test]
fn an_external_value_change_while_open_keeps_the_menu_and_commits_nothing() {
  let mut fixture = Fixture::new();
  fixture.open(1);

  fixture.signals.second.set("lg".to_owned());
  fixture.pass();

  assert!(fixture.has_option("Second Medium"), "the menu stays open");
  assert_eq!(fixture.signals.second.get(), "lg");
  assert_eq!(fixture.signals.first.get(), "md");
  assert_eq!(fixture.focused_label().as_deref(), Some("Second Large"));
}

#[test]
fn open_menu_in_a_field_wrapper_stays_with_its_select_across_an_insertion() {
  let mut fixture = Fixture::wrapped();
  fixture.open(1);
  assert!(fixture.has_option("Second Large"));

  fixture.signals.banner.set(true);
  fixture.pass();

  assert!(!fixture.has_option("First Large"), "the first select must not open");
  assert_ne!(fixture.focused_label().as_deref(), Some("First Medium"));

  fixture.press("Escape");
  assert!(!fixture.has_option("First Large"), "Escape leaves no select open");
  assert!(!fixture.has_option("Second Large"), "Escape leaves no select open");
  assert_eq!(fixture.signals.first.get(), "md");
  assert_eq!(fixture.signals.second.get(), "sm");
}
