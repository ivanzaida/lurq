//! A text input that loses the focus paints no caret, even where the render that moves the focus away also
//! builds it anew: the new node takes over the old one's state, and the old one's focus with it.

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx, events::KeyboardEvent},
  components::{Button, Column, TextInput},
  core::{ElementRef, Signal},
  node::{Element, color::Color},
};

use crate::support::{render_pass_with_app, run_pass};

const CARET: Color = Color::new(255, 0, 255, 255);

#[derive(Clone)]
struct Props {
  value: Signal<String>,
  moved: Signal<bool>,
}

impl PartialEq for Props {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

impl lurq::app::component::DevtoolsInspectable for Props {}

/// An input in a row; Escape in it gives the row the focus, by a ref the row carries from that render on.
struct Scene {
  button: ElementRef,
}

impl Component for Scene {
  type Props = Props;

  fn create(_: &mut Ctx) -> Self {
    Self {
      button: ElementRef::new(),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let Props { value, moved } = ctx.props::<Props>().clone();
    let away = moved.get();
    if away {
      ctx.focus(&self.button);
    }
    let escape = moved.clone();
    let input = TextInput::new(value)
      .caret_color(CARET)
      .id("input")
      .on_key_down(move |event: KeyboardEvent| {
        if event.key == "Escape" && event.focus_within {
          event.prevent_default();
          escape.set(true);
        }
      });
    // The row the input is in takes the focus by a ref it carries from this render on, as an inspector's row does
    // when Escape gives it the focus back: the render builds the row, and the input in it, anew.
    // A press of the row gives the input in it the focus, through a ref the render makes anew.
    let holder = ElementRef::new();
    let mut row = Button::empty()
      .tab_index(0)
      .id("row")
      .focuses(&holder)
      .child(Column::new().ref_element(holder).child(input));
    if away {
      row = row.ref_element(self.button.clone());
    }
    Column::new().child(row)
  }
}

fn carets(tree: &mut Tree, app: &mut App) -> usize {
  render_pass_with_app(tree, app)
    .rects
    .iter()
    .filter(|rect| rect.width == 1.0 && rect.color == CARET)
    .count()
}

#[test]
fn an_input_built_anew_in_the_render_that_moves_the_focus_away_paints_no_caret() {
  let mut app = App::new();
  app.theme().set_caret_mode(lurq::app::theme::CaretMode::Persistent);
  let mut tree = Tree::new();
  let props = Props {
    value: Signal::new("12".to_owned()),
    moved: Signal::new(false),
  };
  tree.mount_root::<Scene>(&mut app, props);
  run_pass(&mut tree);
  let input = tree
    .get_element_by_id_mut("input")
    .expect("the input")
    .bounds()
    .expect("laid out");
  let (x, y) = input.center();
  tree.mouse_down(x, y, lurq::app::events::MouseButton::Left);
  tree.mouse_up(x, y, lurq::app::events::MouseButton::Left);
  assert_eq!(carets(&mut tree, &mut app), 1, "the focused input's caret");

  tree.key_down("Escape".to_owned(), "Escape".to_owned(), false, false, false);
  tree.key_up("Escape".to_owned(), "Escape".to_owned(), false, false, false);
  carets(&mut tree, &mut app);
  assert_eq!(
    tree
      .focused_element()
      .and_then(|element| element.id().map(str::to_owned))
      .as_deref(),
    Some("row")
  );
  assert_eq!(
    carets(&mut tree, &mut app),
    0,
    "no caret once the focus has left the input"
  );
}
