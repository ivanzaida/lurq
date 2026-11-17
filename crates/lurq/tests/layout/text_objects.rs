//! Objects of a read-only rich text (`Text::object`): an element in a line of text, as wide as it is, wrapping
//! with the words, on the middle of a line that keeps its height, and an element like any other to a press and to
//! the focus. The weight-probe face gives exact advances at 10px: "a" is 5px and the space 2.5px.

use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{App, Tree, events::MouseButton},
  components::{Column, Rect, Text},
  layout::{Constraints, Size, quad::RichTextSpan, text_style::TextStyle},
  node::{Element, color::Color},
};

use crate::support::{RectSnapshot, pointer_click, render_pass_with_app};

const FAMILY: &str = "Lurq Weight Probe";
const REGULAR: &[u8] = include_bytes!("../assets/weight_probe/LurqWeightProbe-Regular.ttf");
const RED: Color = Color::new(200, 30, 60, 255);
const GREEN: Color = Color::new(30, 160, 60, 255);
/// The line of the probe's type: 10 at 2.
const LINE: f32 = 20.0;

fn probe_app() -> App {
  let mut app = App::new();
  app.install_fonts([REGULAR.to_vec()], std::iter::empty::<(&str, &str)>());
  app
}

fn style() -> TextStyle {
  TextStyle {
    font_family: FAMILY.into(),
    font_size: 10.0,
    line_height: 2.0,
    ..TextStyle::default()
  }
}

/// "aa ", an object, " aa", with `object` standing on it.
fn sentence(object: impl Into<Element>) -> Text {
  Text::rich([
    RichTextSpan::new("aa ", style()),
    RichTextSpan::object(style()),
    RichTextSpan::new(" aa", style()),
  ])
  .object(object)
}

struct Scene {
  app: App,
  tree: Tree,
}

impl Scene {
  fn new(root: impl Into<Element>, width: f32) -> Self {
    let app = probe_app();
    let mut tree = Tree::new();
    tree.resize(400, 300);
    tree.set_root(root);
    tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(width, 300.0))));
    let mut scene = Self { app, tree };
    scene.render();
    scene
  }

  fn render(&mut self) -> Vec<RectSnapshot> {
    render_pass_with_app(&mut self.tree, &mut self.app).rects
  }

  fn size(&self) -> Size {
    self.tree.last_layout().expect("a layout").size
  }

  fn the_box(&mut self, color: Color) -> RectSnapshot {
    let boxes: Vec<_> = self.render().into_iter().filter(|rect| rect.color == color).collect();
    assert_eq!(boxes.len(), 1, "one box of {color:?}");
    boxes[0].clone()
  }
}

fn close(actual: f32, expected: f32, what: &str) {
  assert!(
    (actual - expected).abs() < 0.6,
    "{what}: {actual} where {expected} was expected"
  );
}

#[test]
fn an_object_takes_the_width_of_its_element_in_the_line_and_keeps_the_lines_height() {
  let mut scene = Scene::new(sentence(Rect::new(30.0, 8.0).background(RED)), 400.0);
  // "aa " is 12.5, the object 30, " aa" 12.5.
  close(scene.size().width, 55.0, "the paragraph's width");
  close(scene.size().height, LINE, "the paragraph's height: the line's");
  let object = scene.the_box(RED);
  close(object.x, 12.5, "the object stands after the words before it");
  close(object.width, 30.0, "as wide as its element");
  close(object.y + object.height / 2.0, LINE / 2.0, "on the middle of its line");

  // An element taller than the line stands over and under it, and the line keeps its height.
  let mut tall = Scene::new(sentence(Rect::new(30.0, 32.0).background(RED)), 400.0);
  close(tall.size().height, LINE, "the line keeps its height");
  let object = tall.the_box(RED);
  close(object.height, 32.0, "drawn whole");
  close(object.y, (LINE - 32.0) / 2.0, "over the top of the line");
}

#[test]
fn an_object_wraps_with_the_words_and_is_no_wider_than_the_paragraph() {
  // 55 do not fit in 45: the words after the object go to the next line.
  let mut scene = Scene::new(sentence(Rect::new(30.0, 8.0).background(RED)), 45.0);
  let height = scene.size().height;
  assert!(height > 1.5 * LINE && height < 2.5 * LINE, "two lines: {height}");
  let object = scene.the_box(RED);
  close(object.x, 12.5, "on the first line, after its words");
  close(
    object.y + object.height / 2.0,
    LINE / 2.0,
    "on the middle of the first line",
  );

  // Narrower still, the object goes to a line of its own, and starts it.
  let mut narrow = Scene::new(sentence(Rect::new(30.0, 8.0).background(RED)), 32.0);
  let object = narrow.the_box(RED);
  close(object.x, 0.0, "at the start of its line");
  close(object.y + object.height / 2.0, LINE * 1.5, "on the second line");

  // An element wider than the paragraph takes no more than the paragraph's room in its line.
  let scene = Scene::new(sentence(Rect::new(60.0, 8.0).background(GREEN)), 40.0);
  assert!(
    scene.size().width <= 40.5,
    "no wider than the room: {}",
    scene.size().width
  );
}

#[test]
fn objects_are_given_in_the_order_of_the_text_and_a_span_left_without_one_is_text() {
  let spans = [
    RichTextSpan::object(style()),
    RichTextSpan::new("a", style()),
    RichTextSpan::object(style()),
    RichTextSpan::new("a", style()),
    RichTextSpan::object(style()),
  ];
  let mut scene = Scene::new(
    Text::rich(spans)
      .object(Rect::new(20.0, 6.0).background(RED))
      .object(Rect::new(10.0, 6.0).background(GREEN)),
    400.0,
  );
  close(scene.the_box(RED).x, 0.0, "the first object");
  close(scene.the_box(GREEN).x, 25.0, "the second, after 20 and an a");
  // The third object character has no element: it is a character of the text, wider than nothing.
  assert!(scene.size().width >= 39.5, "{}", scene.size().width);
}

#[test]
fn a_press_on_an_object_is_its_elements() {
  let pressed = Arc::new(AtomicUsize::new(0));
  let counted = pressed.clone();
  let button = Column::new().size(30.0, 12.0).background(RED).on_click(move |_| {
    counted.fetch_add(1, Ordering::SeqCst);
  });
  let mut scene = Scene::new(sentence(button), 400.0);
  let object = scene.the_box(RED);
  pointer_click(
    &mut scene.tree,
    object.x + object.width / 2.0,
    object.y + object.height / 2.0,
    MouseButton::Left,
  );
  scene.render();
  assert_eq!(pressed.load(Ordering::SeqCst), 1);
  // A press on the words beside it is not.
  pointer_click(&mut scene.tree, 3.0, LINE / 2.0, MouseButton::Left);
  scene.render();
  assert_eq!(pressed.load(Ordering::SeqCst), 1);
}

#[test]
fn an_element_that_changes_its_size_lays_its_line_out_again() {
  let width = lurq::core::Signal::new(30.0f32);
  let held = width.clone();
  #[derive(Clone)]
  struct Props(lurq::core::Signal<f32>);
  impl PartialEq for Props {
    fn eq(&self, _: &Self) -> bool {
      false
    }
  }
  #[cfg(feature = "devtools")]
  impl lurq::app::component::DevtoolsInspectable for Props {}
  struct Grows;
  impl lurq::app::component::Component for Grows {
    type Props = Props;
    fn create(_: &mut lurq::app::ctx::Ctx) -> Self {
      Self
    }
    fn render(&self, ctx: &mut lurq::app::ctx::Ctx) -> impl Into<Element> {
      let width = ctx.props::<Props>().0.get();
      Rect::new(width, 8.0).background(RED)
    }
  }
  let mut app = probe_app();
  let mut tree = Tree::new();
  tree.resize(400, 300);
  struct Root;
  impl lurq::app::component::Component for Root {
    type Props = Props;
    fn create(_: &mut lurq::app::ctx::Ctx) -> Self {
      Self
    }
    fn render(&self, ctx: &mut lurq::app::ctx::Ctx) -> impl Into<Element> {
      let props = ctx.props::<Props>().clone();
      Column::new()
        .child(sentence(Element::component::<Grows>(props)))
        .child(Rect::new(4.0, 4.0).background(GREEN))
    }
  }
  tree.mount_root::<Root>(&mut app, Props(held));
  let rects = render_pass_with_app(&mut tree, &mut app).rects;
  let red = rects.iter().find(|rect| rect.color == RED).expect("the object").clone();
  close(red.width, 30.0, "at first");
  width.set(50.0);
  let rects = render_pass_with_app(&mut tree, &mut app).rects;
  let red = rects.iter().find(|rect| rect.color == RED).expect("the object").clone();
  close(red.width, 50.0, "grown");
}

#[test]
fn a_highlight_past_an_object_is_on_the_words_it_covers() {
  use lurq::components::TextHighlight;
  // " aa" after the object: its "aa" is bytes 3 + 3 + 1 to the end, drawn from 12.5 + 30 + 2.5.
  let text = sentence(Rect::new(30.0, 8.0).background(RED)).highlights(vec![TextHighlight::new(7..9, GREEN)]);
  let mut scene = Scene::new(text, 400.0);
  let mark = scene.the_box(GREEN);
  close(mark.x, 45.0, "the highlight starts where its words are drawn");
  close(mark.width, 10.0, "and is as wide as they are");
}
