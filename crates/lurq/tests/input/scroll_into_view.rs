//! Scrolling an element into view on request: `ElementHandle::scroll_into_view`, `Ctx::scroll_into_view`, and the
//! reveal that follows a focus request (`Ctx::focus`, `ElementHandle::focus`). A pointer click focuses without
//! scrolling. Tab is covered by `tab_order::scroll_into_view`.

use std::sync::Arc;

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx, events::MouseButton},
  components::{Button, Column, Rect, Row, ScrollHorizontal, ScrollVertical},
  core::{ElementRect, ElementRef, Signal},
  node::Element,
};

const ITEM: f32 = 40.0;
const VIEW: f32 = 100.0;

fn button(index: usize) -> Button {
  Button::new(&format!("Item {index}"))
    .id(format!("item-{index}"))
    .size(ITEM, ITEM)
    .tab_index(0)
}

fn items(target: Option<&ElementRef>) -> Column {
  let mut column = Column::new();
  for index in 0..10 {
    let mut item = button(index);
    if index == 7
      && let Some(target) = target
    {
      item = item.ref_element(target.clone());
    }
    column = column.child(item);
  }
  column
}

fn headless(root: impl Into<Element>) -> Tree {
  let mut tree = Tree::new();
  tree.set_root(root);
  tree.pass_headless(&mut App::new());
  tree
}

fn bounds(tree: &mut Tree, id: &str) -> ElementRect {
  tree
    .get_element_by_id_mut(id)
    .and_then(|element| element.bounds())
    .unwrap_or_else(|| panic!("#{id} should be laid out"))
}

fn assert_within(rect: ElementRect, start: f32, end: f32, label: &str) {
  assert!(
    rect.y >= start - 0.5 && rect.y + rect.height <= end + 0.5,
    "{label}: [{}..{}] is outside [{start}..{end}]",
    rect.y,
    rect.y + rect.height
  );
}

#[test]
fn element_handle_scrolls_an_element_deep_in_a_scroll_view_into_view() {
  let mut tree = headless(ScrollVertical::new(items(None)).height(VIEW));

  tree
    .get_element_by_id_mut("item-7")
    .expect("item-7 exists")
    .scroll_into_view();
  tree.pass_headless(&mut App::new());

  // Nearest: item 7 ([280..320]) ends at the viewport bottom.
  let rect = bounds(&mut tree, "item-7");
  assert!((rect.y - 60.0).abs() < 0.5, "item-7 top = {}", rect.y);
}

#[test]
fn scroll_into_view_leaves_a_visible_element_in_place() {
  let mut tree = headless(ScrollVertical::new(items(None)).height(VIEW));

  tree
    .get_element_by_id_mut("item-1")
    .expect("item-1 exists")
    .scroll_into_view();
  tree.pass_headless(&mut App::new());

  assert_eq!(bounds(&mut tree, "item-0").y, 0.0);
}

#[test]
fn scroll_into_view_scrolls_nested_containers_innermost_first() {
  let inner = ScrollVertical::new(items(None)).id("inner").height(VIEW);
  let content = Column::new()
    .child(Rect::new(ITEM, 300.0))
    .child(inner)
    .child(Rect::new(ITEM, 300.0));
  let mut tree = headless(ScrollVertical::new(content).height(200.0));

  tree
    .get_element_by_id_mut("item-7")
    .expect("item-7 exists")
    .scroll_into_view();
  tree.pass_headless(&mut App::new());

  let inner = bounds(&mut tree, "inner");
  assert_within(inner, 0.0, 200.0, "inner scroll view in the outer viewport");
  let item = bounds(&mut tree, "item-7");
  assert_within(item, inner.y, inner.y + VIEW, "item-7 in the inner viewport");
  assert_within(item, 0.0, 200.0, "item-7 in the outer viewport");
}

#[test]
fn scroll_into_view_scrolls_horizontally_in_a_horizontal_container() {
  let mut row = Row::new();
  for index in 0..10 {
    row = row.child(button(index));
  }
  let mut tree = headless(ScrollHorizontal::new(row).width(VIEW));

  tree
    .get_element_by_id_mut("item-7")
    .expect("item-7 exists")
    .scroll_into_view();
  tree.pass_headless(&mut App::new());

  let rect = bounds(&mut tree, "item-7");
  assert!((rect.x - 60.0).abs() < 0.5, "item-7 left = {}", rect.x);
}

#[derive(Clone, Copy, Debug, PartialEq, lurq::DevtoolsInspectable)]
enum Request {
  None,
  Scroll,
  Focus,
}

struct RequestState {
  target: ElementRef,
  request: Signal<Request>,
}

#[derive(Clone, lurq::DevtoolsInspectable)]
struct RequestProps(#[devtools_ignore] Arc<RequestState>);

impl PartialEq for RequestProps {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

/// A scroll view whose item 7 carries `target`; render issues the request
/// held by the signal.
struct RequestingList;

impl Component for RequestingList {
  type Props = RequestProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let state = ctx.props::<Self::Props>().0.clone();
    match state.request.get() {
      Request::None => {}
      Request::Scroll => ctx.scroll_into_view(&state.target),
      Request::Focus => ctx.focus(&state.target),
    }
    ScrollVertical::new(items(Some(&state.target))).height(VIEW)
  }
}

fn mount_requesting_list() -> (Tree, Arc<RequestState>) {
  let state = Arc::new(RequestState {
    target: ElementRef::new(),
    request: Signal::new(Request::None),
  });
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<RequestingList>(&mut app, RequestProps(state.clone()));
  tree.pass_headless(&mut app);
  (tree, state)
}

#[test]
fn ctx_scroll_into_view_reveals_the_element_in_the_next_frame() {
  let (mut tree, state) = mount_requesting_list();
  assert_eq!(bounds(&mut tree, "item-0").y, 0.0);

  state.request.set(Request::Scroll);
  tree.pass_headless(&mut App::new());

  let rect = bounds(&mut tree, "item-7");
  assert!((rect.y - 60.0).abs() < 0.5, "item-7 top = {}", rect.y);
}

#[test]
fn ctx_focus_scrolls_the_focused_element_into_view() {
  let (mut tree, state) = mount_requesting_list();

  state.request.set(Request::Focus);
  tree.pass_headless(&mut App::new());

  assert!(state.target.focused());
  let rect = bounds(&mut tree, "item-7");
  assert!((rect.y - 60.0).abs() < 0.5, "item-7 top = {}", rect.y);
}

#[test]
fn element_handle_focus_scrolls_the_focused_element_into_view() {
  let mut tree = headless(ScrollVertical::new(items(None)).height(VIEW));

  tree.get_element_by_id_mut("item-7").expect("item-7 exists").focus();
  tree.pass_headless(&mut App::new());

  let rect = bounds(&mut tree, "item-7");
  assert!((rect.y - 60.0).abs() < 0.5, "item-7 top = {}", rect.y);
}

#[test]
fn clicking_a_partly_visible_element_focuses_it_without_scrolling() {
  let mut tree = headless(ScrollVertical::new(items(None)).height(VIEW));

  // Item 2 spans [80..120]: only its top half is in the viewport.
  crate::support::pointer_click(&mut tree, 20.0, 90.0, MouseButton::Left);
  tree.pass_headless(&mut App::new());

  let focused = tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned));
  assert_eq!(focused.as_deref(), Some("item-2"));
  assert_eq!(bounds(&mut tree, "item-0").y, 0.0);
}
