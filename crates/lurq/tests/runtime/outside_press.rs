//! A left press outside an open popup closes it and, by default
//! (`OutsidePress::Consume`), does nothing else: the element under the
//! pointer is neither pressed, clicked nor focused.

use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx, events::MouseButton},
  components::{
    Button, ChromeTitleBar, Column, OutsidePress, Placement, Popup, Row, Select, Spacer, TextInput, WindowChrome,
    WindowChromeMode,
  },
  core::{ElementRef, Signal},
  node::Element,
};

use crate::support::pointer_click;

#[derive(Default)]
struct Counts {
  anchor_clicks: AtomicUsize,
  inside_clicks: AtomicUsize,
  behind_presses: AtomicUsize,
  behind_clicks: AtomicUsize,
  title_clicks: AtomicUsize,
}

impl Counts {
  fn get(counter: &AtomicUsize) -> usize {
    counter.load(Ordering::SeqCst)
  }
}

#[derive(Clone, Copy, PartialEq)]
struct Setup {
  outside_press: OutsidePress,
  dismiss: bool,
  /// The anchor and its popup sit in a `WindowChrome` title bar, whose layer
  /// lies over the page and under the popup.
  in_title_bar: bool,
}

const DEFAULT: Setup = Setup {
  outside_press: OutsidePress::Consume,
  dismiss: true,
  in_title_bar: false,
};

#[derive(Clone, lurq::DevtoolsInspectable)]
struct Props {
  #[devtools_ignore]
  open: Signal<bool>,
  #[devtools_ignore]
  choice: Signal<String>,
  #[devtools_ignore]
  counts: Arc<Counts>,
  #[devtools_ignore]
  setup: Setup,
}

impl PartialEq for Props {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.counts, &other.counts) && self.setup == other.setup
  }
}

struct Page {
  anchor: ElementRef,
}

fn counted(
  counter: impl Fn(&Counts) -> &AtomicUsize + Send + Sync + 'static,
  counts: Arc<Counts>,
) -> impl Fn() + Send + Sync + 'static {
  move || {
    counter(&counts).fetch_add(1, Ordering::SeqCst);
  }
}

impl Page {
  fn popup(&self, props: &Props) -> Popup {
    let inside = counted(|counts| &counts.inside_clicks, props.counts.clone());
    let content = Column::new()
      .child(Button::new("Inside").id("inside").on_click(move |_| inside()))
      .child(
        Select::new(props.choice.clone())
          .options([
            ("game".to_owned(), "Game"),
            ("studio".to_owned(), "Studio"),
            ("warm".to_owned(), "Warm"),
          ])
          .width(160.0)
          .height(30.0),
      );
    Popup::new(self.anchor.clone(), content)
      .open(props.open.clone())
      .placement(Placement::BottomStart)
      .dismiss_on_outside_click(props.setup.dismiss)
      .outside_press(props.setup.outside_press)
  }

  fn anchor_button(&self, props: &Props) -> Button {
    let open = props.open.clone();
    let clicked = counted(|counts| &counts.anchor_clicks, props.counts.clone());
    Button::new("Menu")
      .id("anchor")
      .ref_element(self.anchor.clone())
      .on_click(move |_| {
        clicked();
        open.update(|open| *open = !*open);
      })
  }
}

impl Component for Page {
  type Props = Props;

  fn create(_ctx: &mut Ctx) -> Self {
    Self {
      anchor: ElementRef::new(),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Props>().clone();
    let pressed = counted(|counts| &counts.behind_presses, props.counts.clone());
    let clicked = counted(|counts| &counts.behind_clicks, props.counts.clone());
    let behind = Button::new("Behind")
      .id("behind")
      .width(200.0)
      .height(40.0)
      .on_mouse_down(move |_| pressed())
      .on_click(move |_| clicked());
    let field = TextInput::new(Signal::new(String::new())).id("field").width(200.0);
    let menu = Row::new().child(self.anchor_button(&props)).child(self.popup(&props));
    if !props.setup.in_title_bar {
      return Column::new()
        .child(menu)
        .child(Spacer::new().height(300.0))
        .child(behind)
        .child(field)
        .into();
    }
    let title_clicked = counted(|counts| &counts.title_clicks, props.counts.clone());
    let title_bar = ChromeTitleBar::new().leading(
      Row::new()
        .child(menu)
        .child(Spacer::new().width(300.0))
        .child(Button::new("Tab").id("title-button").on_click(move |_| title_clicked())),
    );
    WindowChrome::new()
      .mode(WindowChromeMode::AlwaysCustom)
      .title_bar(title_bar)
      .content(
        Column::new()
          .child(Spacer::new().height(300.0))
          .child(behind)
          .child(field),
      )
      .mount(ctx)
  }
}

struct Fixture {
  app: App,
  tree: Tree,
  open: Signal<bool>,
  choice: Signal<String>,
  counts: Arc<Counts>,
}

impl Fixture {
  fn new(setup: Setup) -> Self {
    let open = Signal::new(true);
    let choice = Signal::new("studio".to_owned());
    let counts = Arc::new(Counts::default());
    let mut app = App::new();
    let mut tree = Tree::new();
    tree.mount_root::<Page>(
      &mut app,
      Props {
        open: open.clone(),
        choice: choice.clone(),
        counts: counts.clone(),
        setup,
      },
    );
    let mut fixture = Self {
      app,
      tree,
      open,
      choice,
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

  fn center(&mut self, id: &str) -> (f32, f32) {
    self
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.bounds())
      .unwrap_or_else(|| panic!("#{id} should be laid out"))
      .center()
  }

  fn click(&mut self, (x, y): (f32, f32)) {
    pointer_click(&mut self.tree, x, y, MouseButton::Left);
    self.pass();
  }

  fn click_id(&mut self, id: &str) {
    let point = self.center(id);
    self.click(point);
  }

  fn popup_visible(&mut self) -> bool {
    self.tree.get_element_by_id_mut("inside").is_some()
  }
}

#[test]
fn a_press_outside_only_closes_the_popup() {
  let mut fixture = Fixture::new(DEFAULT);
  assert!(fixture.popup_visible());

  fixture.click_id("behind");

  assert!(!fixture.open.get());
  assert!(!fixture.popup_visible());
  assert_eq!(Counts::get(&fixture.counts.behind_presses), 0, "no press reaches it");
  assert_eq!(Counts::get(&fixture.counts.behind_clicks), 0, "no click reaches it");

  fixture.click_id("behind");
  assert_eq!(Counts::get(&fixture.counts.behind_presses), 1);
  assert_eq!(
    Counts::get(&fixture.counts.behind_clicks),
    1,
    "the next press reaches it"
  );
}

#[test]
fn a_consumed_press_does_not_move_focus() {
  let mut fixture = Fixture::new(DEFAULT);
  fixture.open.set(false);
  fixture.pass();
  fixture.tree.get_element_by_id_mut("field").expect("field").focus();
  fixture.click_id("anchor");
  assert!(fixture.open.get());
  let focused = |fixture: &Fixture| {
    fixture
      .tree
      .focused_element()
      .and_then(|element| element.id().map(str::to_owned))
  };
  let before = focused(&fixture);

  fixture.click_id("behind");

  assert!(!fixture.open.get());
  assert_eq!(focused(&fixture), before);
}

#[test]
fn a_pass_through_popup_delivers_the_closing_press() {
  let mut fixture = Fixture::new(Setup {
    outside_press: OutsidePress::PassThrough,
    ..DEFAULT
  });

  fixture.click_id("behind");

  assert!(!fixture.open.get());
  assert_eq!(Counts::get(&fixture.counts.behind_presses), 1);
  assert_eq!(Counts::get(&fixture.counts.behind_clicks), 1);
}

#[test]
fn presses_on_the_anchor_and_the_popup_are_delivered() {
  let mut fixture = Fixture::new(DEFAULT);

  fixture.click_id("inside");
  assert_eq!(Counts::get(&fixture.counts.inside_clicks), 1);
  assert!(fixture.open.get());

  fixture.click_id("anchor");
  assert_eq!(Counts::get(&fixture.counts.anchor_clicks), 1, "the anchor toggles it");
  assert!(!fixture.open.get());
}

#[test]
fn a_popup_that_does_not_dismiss_lets_presses_through() {
  let mut fixture = Fixture::new(Setup {
    dismiss: false,
    ..DEFAULT
  });

  fixture.click_id("behind");

  assert!(fixture.open.get());
  assert_eq!(Counts::get(&fixture.counts.behind_clicks), 1);
}

#[test]
fn other_buttons_pass_through_and_keep_the_popup_open() {
  let mut fixture = Fixture::new(DEFAULT);
  let (x, y) = fixture.center("behind");

  fixture.tree.mouse_down(x, y, MouseButton::Right);
  fixture.tree.mouse_up(x, y, MouseButton::Right);
  fixture.pass();

  assert!(fixture.open.get());
  assert_eq!(Counts::get(&fixture.counts.behind_presses), 1);
}

#[test]
fn a_select_menu_opened_from_the_popup_is_not_outside_it() {
  let mut fixture = Fixture::new(DEFAULT);
  let popup_bottom = {
    let bounds = fixture
      .tree
      .find_element(|element| element.tag_name() == "Select")
      .expect("select in the popup")
      .bounds();
    bounds.y + bounds.height
  };
  let (x, y) = fixture
    .tree
    .find_element(|element| element.tag_name() == "Select")
    .expect("select in the popup")
    .bounds()
    .center();
  fixture.click((x, y));
  let option = fixture
    .tree
    .find_element(|element| element.text_content() == Some("Warm"))
    .expect("select menu open")
    .bounds();
  assert!(option.y >= popup_bottom, "the option lies below the popup: {option:?}");

  fixture.click(option.center());

  assert_eq!(fixture.choice.get(), "warm");
  assert!(fixture.open.get(), "choosing in the select menu keeps the popup open");
}

#[test]
fn an_open_select_menu_in_the_popup_consumes_a_press_on_the_popup() {
  let mut fixture = Fixture::new(DEFAULT);
  let (x, y) = fixture
    .tree
    .find_element(|element| element.tag_name() == "Select")
    .expect("select in the popup")
    .bounds()
    .center();
  fixture.click((x, y));
  assert!(
    fixture
      .tree
      .find_element(|element| element.text_content() == Some("Warm"))
      .is_some()
  );

  fixture.click_id("inside");

  assert!(
    fixture
      .tree
      .find_element(|element| element.text_content() == Some("Warm"))
      .is_none(),
    "the select menu closes"
  );
  assert_eq!(Counts::get(&fixture.counts.inside_clicks), 0);
  assert!(fixture.open.get(), "the press was inside the popup");
}

#[test]
fn a_press_on_the_title_bar_under_a_title_bar_popup_is_consumed() {
  let mut fixture = Fixture::new(Setup {
    in_title_bar: true,
    ..DEFAULT
  });
  assert!(fixture.popup_visible());

  fixture.click_id("title-button");

  assert!(!fixture.open.get());
  assert_eq!(Counts::get(&fixture.counts.title_clicks), 0);
  fixture.click_id("title-button");
  assert_eq!(Counts::get(&fixture.counts.title_clicks), 1);

  fixture.click_id("anchor");
  assert!(fixture.open.get());
  fixture.click_id("behind");
  assert!(!fixture.open.get());
  assert_eq!(
    Counts::get(&fixture.counts.behind_clicks),
    0,
    "the page under the chrome too"
  );
}
