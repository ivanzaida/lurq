//! A drag that holds the pointer: its moves reach it wherever the pointer goes,
//! in the window and out of it, and the window shows the cursor it began with
//! until it is let go.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx, events::MouseButton},
  components::{Rect, Row},
  layout::{Constraints, Size},
  node::{CursorIcon, Element},
};

use crate::support::run_pass;

#[derive(Clone, Debug, lurq::DevtoolsInspectable)]
struct Moves(Arc<Mutex<Vec<f32>>>);

impl PartialEq for Moves {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

/// A handle that resizes, and beside it a text's room, each with a cursor of its own.
struct Held;

impl Component for Held {
  type Props = Moves;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let moves = ctx.props::<Moves>().0.clone();
    Row::new()
      .child(
        Rect::new(20.0, 100.0)
          .cursor(CursorIcon::ColResize)
          .on_drag_move(move |event: lurq::app::events::DragEvent| moves.lock().unwrap().push(event.total_delta_x)),
      )
      .child(Rect::new(200.0, 100.0).cursor(CursorIcon::Text))
  }
}

fn tree() -> (Tree, Arc<Mutex<Vec<f32>>>) {
  let moves = Arc::new(Mutex::new(Vec::new()));
  let mut tree = Tree::new();
  tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 300.0))));
  tree.mount_root::<Held>(&mut App::new(), Moves(moves.clone()));
  run_pass(&mut tree);
  (tree, moves)
}

#[test]
fn the_cursor_a_drag_began_with_stays_until_it_is_let_go() {
  let (mut tree, _) = tree();
  tree.mouse_move(10.0, 50.0);
  assert_eq!(tree.cursor(), CursorIcon::ColResize);
  tree.mouse_down(10.0, 50.0, MouseButton::Left);
  // Over the text, whose cursor is another.
  tree.mouse_move(120.0, 50.0);
  assert_eq!(tree.cursor(), CursorIcon::ColResize);
  // Out of the window: the shell tells the tree the pointer left, and goes on passing its moves.
  tree.mouse_leave_window();
  assert_eq!(tree.cursor(), CursorIcon::ColResize);
  tree.mouse_move(900.0, 50.0);
  assert_eq!(tree.cursor(), CursorIcon::ColResize);
  tree.mouse_up(900.0, 50.0, MouseButton::Left);
  tree.mouse_move(120.0, 50.0);
  assert_eq!(
    tree.cursor(),
    CursorIcon::Text,
    "let go, the cursor is what is under the pointer"
  );
}

#[test]
fn a_drag_hears_the_pointer_out_of_the_window() {
  let (mut tree, moves) = tree();
  tree.mouse_down(10.0, 50.0, MouseButton::Left);
  tree.mouse_move(120.0, 50.0);
  tree.mouse_leave_window();
  tree.mouse_move(-40.0, 500.0);
  tree.mouse_up(-40.0, 500.0, MouseButton::Left);
  assert_eq!(*moves.lock().unwrap(), vec![110.0, -50.0]);
}
