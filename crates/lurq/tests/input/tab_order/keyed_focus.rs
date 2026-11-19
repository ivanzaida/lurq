//! A key's or a press's handler moving the focus within its group to an element by its key, once the render the
//! event causes is done: a page of numbered items that Right moves through and onto the next page, and whose last
//! item, pressed, turns the page.

use std::sync::Arc;

use lurq::{
  app::{
    App, Tree,
    component::Component,
    ctx::Ctx,
    events::{KeyboardEvent, MouseButton, MouseEvent},
  },
  components::{Button, Column, Row},
  core::Signal,
  node::{Element, FocusGroup},
};

use super::{focused_id, tab};
use crate::support::pointer_click;

#[derive(Clone, lurq::DevtoolsInspectable)]
struct PageSignal(Arc<Signal<u32>>);

impl PartialEq for PageSignal {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

impl std::fmt::Debug for PageSignal {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str("PageSignal")
  }
}

/// Three items a page, numbered on from page to page. Right goes to the next number, by its key, turning the page
/// where it is on the next; Down asks for a number that is nowhere; `x` asks for the first item of the group around
/// the other one.
struct Pages {
  page: Signal<u32>,
}

impl Component for Pages {
  type Props = PageSignal;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      page: (*ctx.props::<Self::Props>().0).clone(),
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    let page = self.page.get();
    let item = |number: u32| {
      let (turn, press) = (self.page.clone(), self.page.clone());
      Button::new(&number.to_string())
        .id(format!("item-{number}"))
        .key(format!("item-{number}"))
        .tab_index(0)
        .on_key_down(move |event: KeyboardEvent| {
          if !event.focus_within {
            return;
          }
          match event.key.as_str() {
            "ArrowRight" => {
              let next = number + 1;
              if next / 3 != number / 3 {
                turn.set(next / 3);
              }
              event.focus_key(format!("item-{next}"));
            }
            "ArrowDown" => event.focus_key("item-1000"),
            "x" => event.focus_key("other-1"),
            _ => {}
          }
        })
        // The last item of a page, pressed, turns it and asks for the first of the next, which the press renders.
        .on_click(move |event: MouseEvent| {
          if number % 3 == 2 {
            press.set(number / 3 + 1);
            event.focus_key(format!("item-{}", number + 1));
          }
        })
    };
    Column::new()
      .child(
        Row::new()
          .focus_group(FocusGroup::horizontal().one_stop(true))
          .with_children((page * 3..page * 3 + 3).map(item)),
      )
      .child(
        Row::new()
          .focus_group(FocusGroup::horizontal())
          .child(Button::new("Other").id("other-1").key("other-1").tab_index(0)),
      )
  }
}

fn pages() -> (Tree, App, Signal<u32>) {
  let page = Signal::new(0);
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<Pages>(&mut app, PageSignal(Arc::new(page.clone())));
  tree.pass_headless(&mut app);
  (tree, app, page)
}

fn key(tree: &mut Tree, app: &mut App, key: &str) {
  tree.key_down(key.to_owned(), key.to_owned(), false, false, false);
  tree.key_up(key.to_owned(), key.to_owned(), false, false, false);
  tree.pass_headless(app);
}

#[test]
fn the_focus_goes_to_the_element_of_the_key_in_the_group() {
  let (mut tree, mut app, _) = pages();
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("item-0"));
  key(&mut tree, &mut app, "ArrowRight");
  assert_eq!(focused_id(&tree).as_deref(), Some("item-1"));
  assert!(tree.focus_visible(), "moved from the keyboard, it shows its ring");
}

#[test]
fn the_element_may_be_one_the_key_renders() {
  let (mut tree, mut app, page) = pages();
  tab(&mut tree);
  key(&mut tree, &mut app, "ArrowRight");
  key(&mut tree, &mut app, "ArrowRight");
  assert_eq!(focused_id(&tree).as_deref(), Some("item-2"));
  // Right from the last of the page turns it: the item it asks for is on the page the key renders.
  key(&mut tree, &mut app, "ArrowRight");
  assert_eq!(page.get_untracked(), 1);
  assert_eq!(focused_id(&tree).as_deref(), Some("item-3"));
}

#[test]
fn a_key_that_is_nowhere_moves_nothing_and_the_group_does_not_act() {
  let (mut tree, mut app, _) = pages();
  tab(&mut tree);
  key(&mut tree, &mut app, "ArrowRight");
  // The request prevents the key's default: Down is not the horizontal group's anyway, and no element has the key.
  key(&mut tree, &mut app, "ArrowDown");
  assert_eq!(focused_id(&tree).as_deref(), Some("item-1"));
}

#[test]
fn an_element_of_another_group_is_not_reached() {
  let (mut tree, mut app, _) = pages();
  tab(&mut tree);
  key(&mut tree, &mut app, "x");
  assert_eq!(
    focused_id(&tree).as_deref(),
    Some("item-0"),
    "the key is looked for in the group around the focus"
  );
}

fn click(tree: &mut Tree, app: &mut App, id: &str) {
  let (x, y) = tree
    .get_element_by_id_mut(id)
    .and_then(|element| element.bounds())
    .expect("element should be laid out")
    .center();
  pointer_click(tree, x, y, MouseButton::Left);
  tree.pass_headless(app);
}

#[test]
fn a_press_asks_for_an_element_the_press_renders_with_no_ring() {
  let (mut tree, mut app, page) = pages();
  click(&mut tree, &mut app, "item-2");
  assert_eq!(page.get_untracked(), 1, "the item pressed is gone with its page");
  assert_eq!(focused_id(&tree).as_deref(), Some("item-3"));
  assert!(!tree.focus_visible(), "moved by the pointer, it shows no ring");
}
