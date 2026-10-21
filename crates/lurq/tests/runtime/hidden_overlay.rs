//! A popup whose anchor is scrolled out of view under the header above a
//! list stays open but is not drawn. It is still dismissed like a CSS popover
//! out of view: a press anywhere closes it and is not consumed, Escape closes
//! it, and keyboard focus inside it goes back to its trigger, or to the
//! nearest focusable ancestor of a trigger that takes no focus. A select's
//! menu is not drawn over the header either.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App, Tree,
    component::Component,
    ctx::Ctx,
    events::{MouseButton, ScrollPhase},
  },
  components::{Column, Popup, Rect, ScrollVertical, Select},
  core::{ElementRect, ElementRef, Signal},
  node::{Element, color::Color},
};

use crate::support::{TestSurface, pointer_click};

const HEADER_HEIGHT: f32 = 40.0;
const TRIGGER: &str = "#22c55e";
const OWNER: &str = "#0ea5e9";
const PANEL_BUTTON: &str = "#ef4444";
const BELOW: &str = "#a855f7";
/// Where the wheel turns: on the list, away from the trigger and the popup.
const WHEEL: (f32, f32) = (280.0, 200.0);

#[derive(Clone, lurq::DevtoolsInspectable)]
struct Props {
  open: Signal<bool>,
  /// Whether the trigger takes focus itself, or only the row around it does.
  focusable_trigger: bool,
  #[devtools_ignore]
  below_clicks: Arc<Mutex<u32>>,
}

impl PartialEq for Props {
  fn eq(&self, other: &Self) -> bool {
    self.focusable_trigger == other.focusable_trigger
  }
}

impl std::fmt::Debug for Props {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Props").finish()
  }
}

/// A header, a list with a row holding the popup's trigger, and a button
/// below the list.
struct PopupInList {
  anchor: ElementRef,
  value: Signal<String>,
}

impl Component for PopupInList {
  type Props = Props;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      anchor: ElementRef::new(),
      value: ctx.signal("one".to_owned()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Props>().clone();
    let mut trigger = Rect::new(120.0, 20.0)
      .background(TRIGGER)
      .ref_element(self.anchor.clone());
    if props.focusable_trigger {
      trigger = trigger.tab_index(0);
    }
    let row = Column::new().background(OWNER).padding(4.0).tab_index(0).child(trigger);
    let select = Select::new(self.value.clone())
      .options([("one".to_owned(), "One"), ("two".to_owned(), "Two")])
      .width(120.0)
      .height(24.0);
    let content = Column::new()
      .child(Rect::new(300.0, 60.0).background("#111827"))
      .child(row)
      .child(select)
      .child(Rect::new(300.0, 600.0).background("#111827"));
    let below_clicks = props.below_clicks.clone();
    Column::new()
      .child(Rect::new(300.0, HEADER_HEIGHT).background("#1f2937"))
      .child(ScrollVertical::new(content).size(300.0, 200.0))
      .child(
        Rect::new(300.0, 30.0)
          .background(BELOW)
          .on_click(move |_| *below_clicks.lock().unwrap() += 1),
      )
      .child(
        Popup::new(
          self.anchor.clone(),
          Rect::new(100.0, 24.0)
            .id("panel-button")
            .background(PANEL_BUTTON)
            .tab_index(0),
        )
        .open(props.open.clone()),
      )
  }
}

struct Fixture {
  app: App,
  tree: Tree,
  open: Signal<bool>,
  below_clicks: Arc<Mutex<u32>>,
}

impl Fixture {
  fn mount(focusable_trigger: bool) -> Self {
    let open = Signal::new(true);
    let below_clicks = Arc::new(Mutex::new(0));
    let mut app = App::new();
    let mut tree = Tree::new();
    tree.mount_root::<PopupInList>(
      &mut app,
      Props {
        open: open.clone(),
        focusable_trigger,
        below_clicks: below_clicks.clone(),
      },
    );
    let mut fixture = Self {
      app,
      tree,
      open,
      below_clicks,
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

  fn panel(&mut self) -> Option<ElementRect> {
    self.bounds_of(PANEL_BUTTON)
  }

  fn focused_color(&self) -> Option<Color> {
    self.tree.focused_element().and_then(|element| element.color())
  }

  /// Scrolls the list by `dy` with the wheel away from the popup, and draws.
  fn scroll_by(&mut self, dy: f32) {
    self.tree.scroll(WHEEL.0, WHEEL.1, 0.0, -dy, ScrollPhase::Scroll);
    self.pass();
  }

  /// Scrolls the trigger wholly under the header: the popup is not drawn.
  fn hide_trigger(&mut self) {
    assert!(self.panel().is_some(), "the popup is drawn while its trigger shows");
    self.scroll_by(120.0);
    let trigger = self.bounds_of(TRIGGER).expect("the trigger");
    assert!(trigger.y + trigger.height <= HEADER_HEIGHT, "{trigger:?}");
    assert_eq!(self.panel(), None, "not drawn over the header");
    assert!(self.open.get_untracked(), "still open");
  }

  fn focus_panel(&mut self) {
    self
      .tree
      .get_element_by_id_mut("panel-button")
      .expect("the popup's button")
      .focus();
    self.pass();
    assert_eq!(self.focused_color(), Some(Color::from_hex(PANEL_BUTTON)));
  }

  fn escape(&mut self) {
    self
      .tree
      .key_down("Escape".into(), "Escape".into(), false, false, false);
    self.tree.key_up("Escape".into(), "Escape".into(), false, false, false);
    self.pass();
  }
}

#[test]
fn hidden_popup_closes_on_a_press_elsewhere_which_still_reaches_what_it_lands_on() {
  let mut fixture = Fixture::mount(true);
  fixture.hide_trigger();

  let below = fixture.bounds_of(BELOW).expect("the button below the list");
  let (x, y) = below.center();
  pointer_click(&mut fixture.tree, x, y, MouseButton::Left);
  fixture.pass();

  assert!(!fixture.open.get_untracked(), "the press closed the hidden popup");
  assert_eq!(*fixture.below_clicks.lock().unwrap(), 1, "and was not consumed");

  fixture.scroll_by(-120.0);
  assert_eq!(fixture.panel(), None, "it stays closed when its trigger is back");
}

#[test]
fn hidden_popup_closes_on_escape() {
  let mut fixture = Fixture::mount(true);
  fixture.hide_trigger();

  fixture.escape();

  assert!(!fixture.open.get_untracked());
}

#[test]
fn shown_popup_still_consumes_the_press_that_closes_it() {
  let mut fixture = Fixture::mount(true);
  let below = fixture.bounds_of(BELOW).expect("the button below the list");
  let (x, y) = below.center();
  pointer_click(&mut fixture.tree, x, y, MouseButton::Left);
  fixture.pass();

  assert!(!fixture.open.get_untracked());
  assert_eq!(*fixture.below_clicks.lock().unwrap(), 0, "consumed, as before");
}

#[test]
fn focus_inside_a_popup_that_hides_goes_back_to_its_trigger() {
  let mut fixture = Fixture::mount(true);
  fixture.focus_panel();

  fixture.hide_trigger();

  assert_eq!(fixture.focused_color(), Some(Color::from_hex(TRIGGER)), "the trigger");
}

#[test]
fn focus_inside_a_popup_that_hides_goes_to_the_owner_of_a_trigger_that_takes_no_focus() {
  let mut fixture = Fixture::mount(false);
  fixture.focus_panel();

  fixture.hide_trigger();

  assert_eq!(
    fixture.focused_color(),
    Some(Color::from_hex(OWNER)),
    "the row around it"
  );
}

#[test]
fn select_menu_is_not_drawn_over_the_header_its_select_scrolled_under() {
  let mut fixture = Fixture::mount(true);
  fixture.open.set(false);
  fixture.pass();
  let select = fixture
    .tree
    .find_element(|element| element.tag_name() == "Select")
    .expect("the select")
    .bounds();
  let (x, y) = select.center();
  pointer_click(&mut fixture.tree, x, y, MouseButton::Left);
  fixture.pass();
  let has_menu = |tree: &mut Tree| {
    tree
      .find_element(|element| element.text_content() == Some("Two"))
      .is_some()
  };
  assert!(has_menu(&mut fixture.tree), "the menu opens");

  fixture.scroll_by(150.0);
  let select = fixture
    .tree
    .find_element(|element| element.tag_name() == "Select")
    .expect("the select")
    .bounds();
  assert!(select.y + select.height <= HEADER_HEIGHT, "{select:?}");
  assert!(!has_menu(&mut fixture.tree), "its menu is not drawn");

  fixture.scroll_by(-150.0);
  assert!(has_menu(&mut fixture.tree), "and comes back with it");
}
