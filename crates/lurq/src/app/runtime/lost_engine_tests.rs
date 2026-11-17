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
  /// The scale each engine was told before its frames, in the order it was told.
  scales: RefCell<Vec<(usize, f32)>>,
  /// The engines that were told they draw lurq's own tooling.
  tooling: RefCell<Vec<usize>>,
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

  fn set_scale_factor(&mut self, scale_factor: f32) {
    self.told.scales.borrow_mut().push((self.number, scale_factor));
  }

  fn draws_tooling(&mut self) {
    self.told.tooling.borrow_mut().push(self.number);
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

#[test]
fn a_loss_that_another_window_finds_lets_go_of_this_windows_engine_too() {
  // The second engine, the other window's, is the one that says the device is lost: the main window's is not
  // drawn, and has not found it. They hold the same device all the same.
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
        loses: told.made.get() == 2,
      })
    }
  });
  tree.resize(300, 120);
  tree.set_root(Text::new("abc"));
  let mut app = App::new();
  tree.push_secondary_window(SecondaryWindow::new("other", 200, 100, Tree::new()));
  assert!(tree.pass(&mut app, &TestSurface).rendered);
  assert_eq!((told.made.get(), told.dropped.get()), (2, 0));

  // What the shell asks of the main window's tree before it draws the other window.
  told.lost.set(true);
  assert!(tree.render_engine.as_ref().is_some_and(|engine| !engine.lost()));
  tree.replace_lost_render_engine();
  assert_eq!(
    told.dropped.get(),
    2,
    "the main window's engine is let go of with the lost one: it holds the lost device"
  );
  assert_eq!(told.made.get(), 4, "and another is made for each window");
  assert!(tree.needs_redraw, "the main window is to be drawn again");
  let other = &tree.secondary_windows[0].tree;
  assert!(other.render_engine.as_ref().is_some_and(|engine| !engine.lost()));
  assert!(other.needs_redraw, "and the other window");
}

#[cfg(feature = "canvas")]
#[test]
fn a_canvas_is_drawn_again_by_its_painter_once_its_renderer_is_lost() {
  use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
  };

  let (mut tree, mut app, told) = tree(false);
  let painted = Arc::new(AtomicUsize::new(0));
  tree.set_root(crate::components::Canvas::new().width(100.0).height(50.0).paint(7, {
    let painted = painted.clone();
    move |_| {
      painted.fetch_add(1, Ordering::SeqCst);
    }
  }));
  assert!(tree.pass(&mut app, &TestSurface).rendered);
  assert_eq!(painted.load(Ordering::SeqCst), 1);
  // Drawn once: a frame for which nothing of it changed does not draw it again.
  tree.request_redraw();
  assert!(tree.pass(&mut app, &TestSurface).rendered);
  assert_eq!(painted.load(Ordering::SeqCst), 1);

  // What it held was on the device that is lost: it is drawn again for the engine that takes the place.
  told.lost.set(true);
  assert!(tree.pass(&mut app, &TestSurface).rendered);
  assert_eq!(told.made.get(), 2);
  assert_eq!(painted.load(Ordering::SeqCst), 2, "the painter draws the canvas again");
  tree.request_redraw();
  assert!(tree.pass(&mut app, &TestSurface).rendered);
  assert_eq!(painted.load(Ordering::SeqCst), 2);
}

/// What a window's handler of the app's events heard.
fn heard(tree: &Tree) -> std::sync::Arc<std::sync::Mutex<Vec<crate::app::window::AppEvent>>> {
  let heard = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
  tree.window().set_app_event_handler(Some(std::sync::Arc::new({
    let heard = heard.clone();
    move |event: &crate::app::window::AppEvent| heard.lock().unwrap().push(event.clone())
  })));
  heard
}

#[test]
fn each_window_hears_once_that_its_device_was_lost_after_its_engine_is_replaced() {
  use crate::app::window::AppEvent;

  let (mut tree, mut app, told) = tree(false);
  tree.push_secondary_window(SecondaryWindow::new("other", 200, 100, Tree::new()));
  let main = heard(&tree);
  let other = heard(&tree.secondary_windows[0].tree);
  assert!(tree.pass(&mut app, &TestSurface).rendered);
  assert!(main.lock().unwrap().is_empty(), "nothing is lost yet");

  told.lost.set(true);
  assert!(tree.pass(&mut app, &TestSurface).rendered);
  assert_eq!(told.made.get(), 4, "the engines of both windows were made again");
  assert_eq!(*main.lock().unwrap(), [AppEvent::GraphicsDeviceLost]);
  assert_eq!(*other.lock().unwrap(), [AppEvent::GraphicsDeviceLost]);

  // An engine that is not lost tells nothing.
  tree.request_redraw();
  assert!(tree.pass(&mut app, &TestSurface).rendered);
  assert_eq!(main.lock().unwrap().len(), 1);
  assert_eq!(other.lock().unwrap().len(), 1);
}

#[test]
fn an_engine_is_told_the_windows_scale_before_each_frame() {
  let (mut tree, mut app, told) = tree(false);
  tree.set_scale_factor(1.5);
  assert!(tree.pass(&mut app, &TestSurface).rendered);
  tree.set_scale_factor(2.0);
  assert!(tree.pass(&mut app, &TestSurface).rendered);
  assert_eq!(*told.scales.borrow(), [(1, 1.5), (1, 2.0)]);
  // And the one made after a loss, before its first frame.
  told.lost.set(true);
  assert!(tree.pass(&mut app, &TestSurface).rendered);
  assert_eq!(told.scales.borrow().last(), Some(&(2, 2.0)));
}

#[test]
fn the_engines_of_a_window_of_lurqs_own_tooling_are_told_so_and_no_others() {
  let (mut tree, mut app, told) = tree(false);
  let mut tooling = Tree::new();
  tooling.draws_tooling = true;
  tree.push_secondary_window(SecondaryWindow::new("tools", 200, 100, tooling));
  assert_eq!(told.made.get(), 2);
  assert_eq!(
    *told.tooling.borrow(),
    [2],
    "the tooling window's engine, and not the app's"
  );
  // As is the one made in its place after a loss, before the main window's.
  assert!(tree.pass(&mut app, &TestSurface).rendered);
  told.lost.set(true);
  assert!(tree.pass(&mut app, &TestSurface).rendered);
  assert_eq!(told.made.get(), 4);
  assert_eq!(*told.tooling.borrow(), [2, 3]);
}
