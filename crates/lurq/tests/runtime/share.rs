//! Sharing an address (`app::Share`): a handler asks the window it holds for the system's share sheet, with
//! what is shared and the element it comes from. A stand-in is in place of the system here: it is given what
//! would have been shared, and no sheet is shown.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{App, Share, ShareError, Tree, component::Component, ctx::Ctx, events::MouseButton, share},
  components::{Column, Rect},
  core::ElementRef,
  node::Element,
};

use crate::support::{pointer_click, run_pass};

/// What a press shares, and what the window answered each press.
#[derive(Clone)]
struct Setup {
  url: &'static str,
  answers: Arc<Mutex<Vec<Result<(), ShareError>>>>,
}

impl PartialEq for Setup {
  fn eq(&self, other: &Self) -> bool {
    self.url == other.url
  }
}

impl lurq::app::component::DevtoolsInspectable for Setup {}

/// A page with a button, 40 from the left and 60 from the top, that shares when it is pressed.
struct Page {
  button: ElementRef,
}

impl Component for Page {
  type Props = Setup;

  fn create(_: &mut Ctx) -> Self {
    Self {
      button: ElementRef::new(),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let setup = ctx.props::<Setup>().clone();
    // A handler has no `Ctx`: it holds the window.
    let (window, button) = (ctx.window(), self.button.clone());
    Column::new().padding_left(40.0).padding_top(60.0).child(
      Rect::new(120.0, 30.0)
        .ref_element(self.button.clone())
        .on_click(move |_| {
          let share = Share::url(setup.url).title("An album").text("Look at this").at(&button);
          setup.answers.lock().unwrap().push(window.share(share));
        }),
    )
  }
}

fn stage(url: &'static str) -> (Tree, Arc<Mutex<Vec<Result<(), ShareError>>>>) {
  let answers = Arc::new(Mutex::new(Vec::new()));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.resize(400, 300);
  tree.mount_root::<Page>(
    &mut app,
    Setup {
      url,
      answers: answers.clone(),
    },
  );
  run_pass(&mut tree);
  (tree, answers)
}

#[test]
fn a_press_shares_an_address_with_its_title_its_text_and_the_place_of_what_was_pressed() {
  let shared = Arc::new(Mutex::new(Vec::<Share>::new()));
  let log = shared.clone();
  let _held = share::stand_in(move |share| log.lock().unwrap().push(share.clone()));
  let (mut tree, answers) = stage("  https://www.example.com/a/Ab12  ");
  pointer_click(&mut tree, 100.0, 75.0, MouseButton::Left);

  assert_eq!(*answers.lock().unwrap(), [Ok(())]);
  let shared = shared.lock().unwrap();
  assert_eq!(shared.len(), 1, "one press, one share");
  let share = &shared[0];
  assert_eq!(share.url, "https://www.example.com/a/Ab12", "the address, trimmed");
  assert_eq!(share.title.as_deref(), Some("An album"));
  assert_eq!(share.text.as_deref(), Some("Look at this"));
  let at = share.at.expect("the button is in the tree");
  assert_eq!((at.x, at.y, at.width, at.height), (40.0, 60.0, 120.0, 30.0));
}

#[test]
fn what_is_not_an_address_is_refused_and_nothing_is_shared() {
  let shared = Arc::new(Mutex::new(Vec::<Share>::new()));
  let log = shared.clone();
  let _held = share::stand_in(move |share| log.lock().unwrap().push(share.clone()));
  let (mut tree, answers) = stage("notes/album.txt");
  pointer_click(&mut tree, 100.0, 75.0, MouseButton::Left);

  assert_eq!(*answers.lock().unwrap(), [Err(ShareError::NotAnAddress)]);
  assert!(shared.lock().unwrap().is_empty());
}

/// With no stand-in the answer is the platform's: a sheet that is asked of the window's shell where the system
/// has one (a tree with no shell shows none), and `Unsupported`, said at once, where it has none.
#[test]
fn where_the_system_has_no_sheet_the_window_says_so_at_once() {
  let (mut tree, answers) = stage("https://www.example.com/a/Ab12");
  pointer_click(&mut tree, 100.0, 75.0, MouseButton::Left);
  let expected = if share::supported() {
    Ok(())
  } else {
    Err(ShareError::Unsupported)
  };
  assert_eq!(*answers.lock().unwrap(), [expected]);
}
