//! An engine whose graphics device is lost is replaced by another, made by what made it: the window is drawn
//! again, and the app goes on.

use std::{
  cell::{Cell, RefCell},
  rc::Rc,
};

use raw_window_handle::{DisplayHandle, WindowHandle};

use super::{SecondaryWindow, Tree};
use crate::{
  app::{App, capture_test_engine::TestSurface, render_engine::RenderEngine},
  components::Text,
  layout::render_list::RenderList,
};

/// What the engines of a test did, in the order they did it.
#[derive(Default)]
struct Told {
  made: Cell<usize>,
  /// Whether the device was lost: the engines made before it are lost with it.
  lost: Cell<bool>,
  /// How many engines were let go of.
  dropped: Cell<usize>,
  /// Each frame drawn: the engine that drew it, and the size it was told.
  drawn: RefCell<Vec<(usize, (u32, u32))>>,
}

struct Engine {
  number: usize,
  size: (u32, u32),
  told: Rc<Told>,
  /// Whether this one is lost when the device is: one made after the loss has a device of its own.
  loses: bool,
}

impl Drop for Engine {
  fn drop(&mut self) {
    self.told.dropped.set(self.told.dropped.get() + 1);
  }
}

impl RenderEngine for Engine {
  fn resize(&mut self, width: u32, height: u32) {
    self.size = (width, height);
  }

  fn render(&mut self, _list: &RenderList, _window: WindowHandle<'_>, _display: DisplayHandle<'_>) -> bool {
    if self.lost() {
      return false;
    }
    self.told.drawn.borrow_mut().push((self.number, self.size));
    true
  }

  fn lost(&self) -> bool {
    self.loses && self.told.lost.get()
  }
}

/// A tree whose engines are made by a factory that counts them; `every` says whether each of them is lost
/// once the test says so, or those alone that were made before it.
fn tree(every: bool) -> (Tree, App, Rc<Told>) {
  let told = Rc::new(Told::default());
  let mut tree = Tree::new();
  tree.set_render_engine_factory({
    let told = told.clone();
    move || {
      told.made.set(told.made.get() + 1);
      Box::new(Engine {
        number: told.made.get(),
        size: (0, 0),
        told: told.clone(),
        loses: every || !told.lost.get(),
      })
    }
  });
  tree.resize(300, 120);
  tree.set_root(Text::new("abc"));
  (tree, App::new(), told)
}

#[test]
fn an_engine_that_is_lost_is_replaced_and_the_window_is_drawn_again() {
  let (mut tree, mut app, told) = tree(false);
  assert!(tree.pass(&mut app, &TestSurface).rendered);
  assert_eq!(told.made.get(), 1);
  assert_eq!(*told.drawn.borrow(), [(1, (300, 120))]);

  // Nothing changed and nothing asks for a frame: the loss itself has the window drawn again, by another
  // engine, which is told how large the window is.
  told.lost.set(true);
  assert!(tree.pass(&mut app, &TestSurface).rendered, "the window is drawn again");
  assert_eq!(
    told.made.get(),
    2,
    "another engine is made in the place of the lost one"
  );
  assert_eq!(told.drawn.borrow().last(), Some(&(2, (300, 120))));

  // And it stays: an engine that is not lost is not replaced.
  tree.request_redraw();
  assert!(tree.pass(&mut app, &TestSurface).rendered);
  assert_eq!(told.made.get(), 2);
}

#[test]
fn an_engine_lost_as_soon_as_it_is_made_is_not_made_in_every_frame() {
  let (mut tree, mut app, told) = tree(true);
  assert!(tree.pass(&mut app, &TestSurface).rendered);
  told.lost.set(true);
  for _ in 0..5 {
    tree.request_redraw();
    assert!(!tree.pass(&mut app, &TestSurface).rendered);
  }
  assert_eq!(told.made.get(), 2, "one is made, and no more within the second");
}

#[test]
fn a_tree_with_nothing_that_makes_engines_keeps_the_one_it_has() {
  let told = Rc::new(Told::default());
  let mut tree = Tree::new();
  tree.render_engine = Some(Box::new(Engine {
    number: 1,
    size: (0, 0),
    told: told.clone(),
    loses: true,
  }));
  tree.resize(300, 120);
  tree.set_root(Text::new("abc"));
  let mut app = App::new();
  told.lost.set(true);
  assert!(!tree.pass(&mut app, &TestSurface).rendered);
  assert!(tree.render_engine.is_some());
}

#[test]
fn the_engines_of_the_trees_other_windows_are_let_go_of_with_its_own() {
  let (mut tree, mut app, told) = tree(false);
  tree.push_secondary_window(SecondaryWindow::new("other", 200, 100, Tree::new()));
  assert_eq!(told.made.get(), 2, "the other window has an engine of its own");
  assert!(tree.pass(&mut app, &TestSurface).rendered);

  // One device is behind both, and it is lost. The other window is not drawn now, and holds the lost device
  // all the same: its engine is let go of in the pass that finds the loss, and another made for it.
  told.lost.set(true);
  assert!(tree.pass(&mut app, &TestSurface).rendered);
  assert_eq!(told.dropped.get(), 2, "both lost engines are let go of");
  assert_eq!(told.made.get(), 4);
  let other = &tree.secondary_windows[0].tree;
  assert!(other.render_engine.as_ref().is_some_and(|engine| !engine.lost()));
  assert!(other.needs_redraw, "the other window is to be drawn again");
}
