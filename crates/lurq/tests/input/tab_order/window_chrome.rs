//! Tab in an app wrapped in `WindowChrome`. The chrome's title bar and resize
//! zones are a layer over the page; in 0.24.0 that layer was an open modal
//! without stops, so Tab did nothing anywhere in the window.

use std::sync::Arc;

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx},
  components::{Button, ChromeTitleBar, Column, Modal, Rect, Root, WindowChrome, WindowChromeMode},
  core::Signal,
  node::{Element, dimension::Dimension},
};

use super::{focused_id, shift_tab, tab};

/// Where the app declares its dialog: in the page (the dialog layer sits below
/// the chrome layer) or as a `WindowChrome::overlay` (it sits above it).
#[derive(Clone, Copy, PartialEq, Eq)]
enum DialogAt {
  Page,
  ChromeOverlay,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Dialog {
  Buttons,
  NoStops,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Setup {
  dialog_at: DialogAt,
  dialog: Dialog,
  /// A form between the page's two buttons.
  form: bool,
  /// A button with `tab_index(0)` in the title bar.
  title_stop: bool,
  open: bool,
}

const PAGE: Setup = Setup {
  dialog_at: DialogAt::Page,
  dialog: Dialog::Buttons,
  form: false,
  title_stop: false,
  open: false,
};

#[derive(Clone)]
struct ChromeProps {
  open: Arc<Signal<bool>>,
  setup: Setup,
}

impl PartialEq for ChromeProps {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.open, &other.open) && self.setup == other.setup
  }
}

impl lurq::app::component::DevtoolsInspectable for ChromeProps {
  fn write_info(&self, _buffer: &mut Vec<lurq::app::component::ComponentInfo>) {}
}

struct ChromeApp;

impl Component for ChromeApp {
  type Props = ChromeProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<ChromeProps>().clone();
    let setup = props.setup;
    let dialog = Modal::new(dialog(setup.dialog))
      .open((*props.open).clone())
      .target(Root);

    let mut page = Column::new()
      .width(Dimension::Pct(100.0))
      .height(Dimension::Pct(100.0))
      .child(Button::new("Toolbar").id("toolbar").tab_index(0));
    #[cfg(feature = "form")]
    if setup.form {
      page = page.child(form());
    }
    page = page.child(Button::new("Footer").id("footer").tab_index(0));

    let mut title_bar = ChromeTitleBar::new();
    if setup.title_stop {
      title_bar = title_bar.leading(Button::new("Menu").id("menu").tab_index(0));
    }
    let mut chrome = WindowChrome::new()
      .mode(WindowChromeMode::AlwaysCustom)
      .title_bar(title_bar);
    match setup.dialog_at {
      DialogAt::Page => page = page.child(dialog),
      DialogAt::ChromeOverlay => chrome = chrome.overlay(dialog),
    }
    chrome.content(page).mount(ctx)
  }
}

fn dialog(dialog: Dialog) -> Column {
  match dialog {
    Dialog::Buttons => Column::new()
      .child(Button::new("Cancel").id("cancel").tab_index(0))
      .child(Button::new("Confirm").id("confirm").tab_index(0)),
    Dialog::NoStops => Column::new().child(Rect::new(100.0, 40.0)),
  }
}

#[cfg(feature = "form")]
fn form() -> Element {
  use lurq::components::{Form, FormHandle, FormOptions, FormProps, TextInput};

  Form::element(
    FormProps::new(FormHandle::new(FormOptions::new())),
    Column::new()
      .child(TextInput::new(Signal::new(String::new())).id("form-name"))
      .child(Button::new("Save").id("form-save").submit()),
  )
}

struct Mounted {
  app: App,
  tree: Tree,
  open: Signal<bool>,
}

impl Mounted {
  fn new(setup: Setup) -> Self {
    let open = Signal::new(setup.open);
    let mut app = App::new();
    let mut tree = Tree::new();
    tree.mount_root::<ChromeApp>(
      &mut app,
      ChromeProps {
        open: Arc::new(open.clone()),
        setup,
      },
    );
    tree.pass_headless(&mut app);
    Self { app, tree, open }
  }

  fn set_open(&mut self, open: bool) {
    self.open.set(open);
    self.tree.pass_headless(&mut self.app);
  }

  fn tab_order(&mut self, count: usize, reverse: bool) -> Vec<String> {
    (0..count)
      .map(|_| {
        if reverse {
          shift_tab(&mut self.tree);
        } else {
          tab(&mut self.tree);
        }
        focused_id(&self.tree).unwrap_or_default()
      })
      .collect()
  }
}

#[test]
fn tab_moves_through_page_stops_under_window_chrome_and_wraps() {
  let mut app = Mounted::new(PAGE);

  assert_eq!(app.tab_order(3, false), ["toolbar", "footer", "toolbar"]);
  assert_eq!(app.tab_order(2, true), ["footer", "toolbar"]);
}

#[cfg(feature = "form")]
#[test]
fn page_form_joins_the_window_order_under_window_chrome() {
  let mut app = Mounted::new(Setup { form: true, ..PAGE });

  assert_eq!(
    app.tab_order(5, false),
    ["toolbar", "form-name", "form-save", "footer", "toolbar"]
  );
}

#[test]
fn title_bar_stops_follow_the_page_stops() {
  let mut app = Mounted::new(Setup {
    title_stop: true,
    ..PAGE
  });

  assert_eq!(app.tab_order(4, false), ["toolbar", "footer", "menu", "toolbar"]);
}

#[test]
fn modal_declared_in_the_page_traps_tab_under_window_chrome() {
  let mut app = Mounted::new(PAGE);
  assert_eq!(app.tab_order(1, false), ["toolbar"]);

  app.set_open(true);
  assert_eq!(app.tab_order(3, false), ["cancel", "confirm", "cancel"]);
  assert_eq!(app.tab_order(3, true), ["confirm", "cancel", "confirm"]);

  app.set_open(false);
  assert_eq!(
    app.tab_order(3, false),
    ["toolbar", "footer", "toolbar"],
    "closing the modal restores the window scope"
  );
}

#[test]
fn modal_declared_as_a_chrome_overlay_traps_tab() {
  let mut app = Mounted::new(Setup {
    dialog_at: DialogAt::ChromeOverlay,
    open: true,
    ..PAGE
  });

  assert_eq!(app.tab_order(3, false), ["cancel", "confirm", "cancel"]);
  assert_eq!(app.tab_order(2, true), ["confirm", "cancel"]);

  app.set_open(false);
  assert_eq!(app.tab_order(2, false), ["toolbar", "footer"]);
}

#[test]
fn modal_without_stops_under_window_chrome_still_keeps_tab_from_the_page() {
  let mut app = Mounted::new(Setup {
    dialog: Dialog::NoStops,
    open: true,
    ..PAGE
  });

  tab(&mut app.tree);
  assert_eq!(
    focused_id(&app.tree),
    None,
    "the toolbar behind the modal is not a stop"
  );
}

#[test]
fn chrome_layer_is_not_reported_as_a_modal() {
  let mut app = Mounted::new(PAGE);
  let tagged = |tree: &mut Tree, tag: &str| tree.find_element(|element| element.tag_name() == tag).is_some();

  assert!(tagged(&mut app.tree, "WindowChromeLayer"));
  assert!(!tagged(&mut app.tree, "Modal"), "no dialog is open");

  app.set_open(true);
  assert!(tagged(&mut app.tree, "Modal"));
}
