//! Going back and forward with the focus where nothing on its way out hears the command: on a button of the title bar
//! the window's chrome draws over the content, or beside the element that has the handler. The command goes on to the
//! window's own handlers, as with nothing focused (#264).

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App, Tree,
    component::Component,
    ctx::Ctx,
    events::{HistoryEvent, NavigationCommand},
  },
  components::{Button, ChromeTitleBar, Column, Rect, Row, WindowChrome, WindowChromeMode},
  node::{Element, dimension::Dimension},
};

use crate::support::run_pass;

type Log = Arc<Mutex<Vec<String>>>;

fn seen(log: &Log) -> Vec<String> {
  std::mem::take(&mut *log.lock().unwrap())
}

fn hears(log: &Log, name: &'static str) -> impl Fn(HistoryEvent) + Send + Sync + 'static {
  let log = log.clone();
  move |event: HistoryEvent| log.lock().unwrap().push(format!("{name} {:?}", event.command()))
}

#[derive(Clone)]
struct Props {
  log: Log,
}

impl PartialEq for Props {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.log, &other.log)
  }
}

impl lurq::app::component::DevtoolsInspectable for Props {
  fn write_info(&self, _buffer: &mut Vec<lurq::app::component::ComponentInfo>) {}
}

/// A window in its chrome, with a button of its own in the title bar and a page that goes back and forward.
struct Chromed;

impl Component for Chromed {
  type Props = Props;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let log = ctx.props::<Props>().log.clone();
    let page = Column::new()
      .width(Dimension::Pct(100.0))
      .height(Dimension::Pct(100.0))
      .on_history_back(hears(&log, "page"))
      .on_history_forward(hears(&log, "page"))
      .child(Button::new("Toolbar").id("toolbar").tab_index(0));
    WindowChrome::new()
      .mode(WindowChromeMode::AlwaysCustom)
      .title_bar(ChromeTitleBar::new().leading(Button::new("Menu").id("menu").tab_index(0)))
      .content(page)
      .mount(ctx)
  }
}

#[test]
fn back_and_forward_with_the_focus_in_the_title_bar_reach_the_page() {
  let log = Log::default();
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<Chromed>(&mut app, Props { log: log.clone() });
  tree.pass_headless(&mut app);

  tree
    .get_element_by_id_mut("menu")
    .expect("the title bar's button")
    .focus();
  tree.pass_headless(&mut app);
  assert_eq!(
    tree
      .focused_element()
      .and_then(|element| element.id().map(str::to_owned)),
    Some("menu".to_owned())
  );

  assert!(tree.run_navigation(NavigationCommand::Back), "something took it");
  assert_eq!(seen(&log), ["page Back"]);
  assert!(tree.run_navigation(NavigationCommand::Forward));
  assert_eq!(seen(&log), ["page Forward"]);
}

#[test]
fn a_focus_beside_the_handler_reaches_it_when_nothing_around_the_focus_hears() {
  let log = Log::default();
  let mut tree = Tree::new();
  tree.resize(400, 200);
  tree.set_root(
    Row::new()
      .size(400.0, 200.0)
      .child(Rect::new(100.0, 40.0).id("field").focusable(true))
      .child(Column::new().size(200.0, 200.0).on_history_back(hears(&log, "content"))),
  );
  run_pass(&mut tree);
  tree.get_element_by_id_mut("field").unwrap().focus();
  run_pass(&mut tree);

  assert!(tree.run_navigation(NavigationCommand::Back));
  assert_eq!(seen(&log), ["content Back"]);
}
