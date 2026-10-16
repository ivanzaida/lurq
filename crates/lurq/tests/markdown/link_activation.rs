//! Link activation with the input that made it: a left or middle click with
//! its modifiers, or Enter (`MarkdownProps::on_link_activate`).

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App, Tree,
    component::Component,
    ctx::Ctx,
    events::{MouseButton, MouseEvent, is_synthetic_input, with_synthetic_input},
  },
  components::{Column, Markdown, MarkdownLink, MarkdownProps},
  layout::quad::QuadContent,
  node::{Element, color::Color},
};

use super::links::{bounds_of, mount, pass, quads};
use crate::support::pointer_click;

/// One activation as the app saw it.
#[derive(Clone, Debug, PartialEq)]
struct Activation {
  destination: String,
  button: MouseButton,
  from_keyboard: bool,
  shift: bool,
  ctrl: bool,
  alt: bool,
  meta: bool,
  synthetic: bool,
}

impl Activation {
  fn pointer(destination: &str, button: MouseButton) -> Self {
    Self {
      destination: destination.to_owned(),
      button,
      from_keyboard: false,
      shift: false,
      ctrl: false,
      alt: false,
      meta: false,
      synthetic: false,
    }
  }
}

type Activations = Arc<Mutex<Vec<Activation>>>;

fn recorder() -> (Activations, impl Fn(&MarkdownLink, &MouseEvent) + Send + Sync) {
  let activations = Activations::default();
  let record = {
    let activations = activations.clone();
    move |link: &MarkdownLink, event: &MouseEvent| {
      activations.lock().unwrap().push(Activation {
        destination: link.destination().to_owned(),
        button: event.button,
        from_keyboard: event.from_keyboard,
        shift: event.shift,
        ctrl: event.ctrl,
        alt: event.alt,
        meta: event.meta,
        synthetic: is_synthetic_input(),
      })
    }
  };
  (activations, record)
}

fn taken(activations: &Activations) -> Vec<Activation> {
  std::mem::take(&mut *activations.lock().unwrap())
}

const TWO_LINKS: &str = "Open [first](/first) or [second](/second) here.";

#[test]
fn markdown_middle_click_activates_a_link_with_the_middle_button() {
  let (activations, record) = recorder();
  let clicks = Arc::new(Mutex::new(Vec::new()));
  let on_click = {
    let clicks = clicks.clone();
    move |link: &MarkdownLink| clicks.lock().unwrap().push(link.destination().to_owned())
  };
  let (_app, mut tree) = mount(
    MarkdownProps::new(TWO_LINKS)
      .on_link_activate(record)
      .on_link_click(on_click)
      .width(500.0),
  );
  let first = bounds_of(&mut tree, "first");

  pointer_click(&mut tree, first.x + 3.0, first.y + 3.0, MouseButton::Middle);
  assert_eq!(
    taken(&activations),
    [Activation::pointer("/first", MouseButton::Middle)]
  );
  assert!(
    clicks.lock().unwrap().is_empty(),
    "on_link_click keeps its meaning: left clicks and Enter"
  );

  pointer_click(&mut tree, first.x + 3.0, first.y + 3.0, MouseButton::Left);
  assert_eq!(taken(&activations), [Activation::pointer("/first", MouseButton::Left)]);
  assert_eq!(*clicks.lock().unwrap(), ["/first"]);

  pointer_click(&mut tree, first.x + 3.0, first.y + 3.0, MouseButton::Right);
  assert!(taken(&activations).is_empty(), "a right click is not an activation");
}

#[test]
fn markdown_link_activation_carries_ctrl_shift_alt_and_meta() {
  let (activations, record) = recorder();
  let (_app, mut tree) = mount(MarkdownProps::new(TWO_LINKS).on_link_activate(record).width(500.0));
  let second = bounds_of(&mut tree, "second");
  let (x, y) = (second.x + 3.0, second.y + 3.0);
  let click = |tree: &mut Tree, button, [shift, ctrl, alt, meta]: [bool; 4]| {
    tree.mouse_down_with_meta(x, y, button, shift, ctrl, alt, meta);
    tree.mouse_up_with_meta(x, y, button, shift, ctrl, alt, meta);
  };

  click(&mut tree, MouseButton::Left, [false, true, false, false]);
  click(&mut tree, MouseButton::Left, [true, true, false, false]);
  click(&mut tree, MouseButton::Left, [false, false, true, false]);
  click(&mut tree, MouseButton::Left, [false, false, false, true]);
  click(&mut tree, MouseButton::Middle, [true, false, false, true]);

  let pressed = |button, [shift, ctrl, alt, meta]: [bool; 4]| Activation {
    shift,
    ctrl,
    alt,
    meta,
    ..Activation::pointer("/second", button)
  };
  assert_eq!(
    taken(&activations),
    [
      pressed(MouseButton::Left, [false, true, false, false]),
      pressed(MouseButton::Left, [true, true, false, false]),
      pressed(MouseButton::Left, [false, false, true, false]),
      pressed(MouseButton::Left, [false, false, false, true]),
      pressed(MouseButton::Middle, [true, false, false, true]),
    ]
  );
}

#[test]
fn markdown_enter_on_a_link_reports_keyboard_activation_with_its_modifiers() {
  let (activations, record) = recorder();
  let (mut app, mut tree) = mount(MarkdownProps::new(TWO_LINKS).on_link_activate(record).width(500.0));

  tree.key_down("Tab".into(), "Tab".into(), false, false, false);
  pass(&mut app, &mut tree);
  tree.key_down("Enter".into(), "Enter".into(), false, false, false);
  tree.key_down_with_meta("Enter".into(), "Enter".into(), true, true, false, true);

  let keyboard = |[shift, ctrl, meta]: [bool; 3]| Activation {
    from_keyboard: true,
    shift,
    ctrl,
    meta,
    ..Activation::pointer("/first", MouseButton::Left)
  };
  assert_eq!(
    taken(&activations),
    [keyboard([false, false, false]), keyboard([true, true, true])]
  );
}

#[test]
fn markdown_middle_press_on_one_link_released_elsewhere_activates_nothing() {
  let (activations, record) = recorder();
  let (_app, mut tree) = mount(MarkdownProps::new(TWO_LINKS).on_link_activate(record).width(500.0));
  let first = bounds_of(&mut tree, "first");
  let second = bounds_of(&mut tree, "second");

  tree.mouse_down(first.x + 3.0, first.y + 3.0, MouseButton::Middle);
  tree.mouse_move(second.x + 3.0, second.y + 3.0);
  tree.mouse_up(second.x + 3.0, second.y + 3.0, MouseButton::Middle);
  assert!(
    taken(&activations).is_empty(),
    "pressed on one link, released on another"
  );

  tree.mouse_down(second.x + 3.0, second.y + 3.0, MouseButton::Middle);
  tree.mouse_move(second.x + 300.0, second.y + 3.0);
  tree.mouse_up(second.x + 300.0, second.y + 3.0, MouseButton::Middle);
  assert!(
    taken(&activations).is_empty(),
    "pressed on a link, released past the text"
  );

  tree.mouse_down(second.x + 300.0, second.y + 3.0, MouseButton::Middle);
  tree.mouse_move(second.x + 3.0, second.y + 3.0);
  tree.mouse_up(second.x + 3.0, second.y + 3.0, MouseButton::Middle);
  assert!(taken(&activations).is_empty(), "pressed elsewhere, released on a link");
}

/// Where the selection highlight is drawn.
fn highlight(tree: &Tree) -> Vec<(f32, f32, f32, f32)> {
  quads(tree)
    .iter()
    .filter(|quad| matches!(quad.content, QuadContent::Rect { color, .. } if color == Color::from_hex("#bfdbfe")))
    .map(|quad| (quad.x, quad.y, quad.width, quad.height))
    .collect()
}

#[test]
fn markdown_middle_click_on_a_link_keeps_the_selection_and_a_middle_drag_selects_nothing() {
  let (activations, record) = recorder();
  let (mut app, mut tree) = mount(
    MarkdownProps::new("Read [the docs](/docs) and then the rest of this paragraph.")
      .selectable(true)
      .on_link_activate(record)
      .width(600.0),
  );
  let link = bounds_of(&mut tree, "the docs");
  let y = link.y + link.height * 0.5;

  tree.mouse_down(link.x + link.width + 20.0, y, MouseButton::Left);
  tree.mouse_move(link.x + link.width + 200.0, y);
  tree.mouse_up(link.x + link.width + 200.0, y, MouseButton::Left);
  pass(&mut app, &mut tree);
  let selected = highlight(&tree);
  assert!(!selected.is_empty(), "a left drag selects");

  pointer_click(&mut tree, link.x + 3.0, y, MouseButton::Middle);
  pass(&mut app, &mut tree);
  assert_eq!(taken(&activations), [Activation::pointer("/docs", MouseButton::Middle)]);
  assert_eq!(highlight(&tree), selected, "the selection stays where it was");

  tree.mouse_down(link.x + 3.0, y, MouseButton::Middle);
  tree.mouse_move(link.x + 250.0, y);
  tree.mouse_up(link.x + 250.0, y, MouseButton::Middle);
  pass(&mut app, &mut tree);
  assert!(taken(&activations).is_empty());
  assert_eq!(highlight(&tree), selected, "a middle drag selects nothing");
}

/// The document inside a column that can be dragged with any button.
struct DraggableRoot;

#[derive(Clone)]
struct DraggableProps {
  markdown: MarkdownProps,
  drags: Arc<Mutex<Vec<MouseButton>>>,
}

impl PartialEq for DraggableProps {
  fn eq(&self, other: &Self) -> bool {
    self.markdown == other.markdown && Arc::ptr_eq(&self.drags, &other.drags)
  }
}

#[cfg(feature = "devtools")]
impl lurq::app::component::DevtoolsInspectable for DraggableProps {}

impl Component for DraggableRoot {
  type Props = DraggableProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Self::Props>().clone();
    let drags = props.drags.clone();
    Column::new()
      .on_drag_start(move |event: lurq::app::events::DragEvent| drags.lock().unwrap().push(event.button))
      .child(Markdown::mount(ctx, props.markdown))
  }
}

#[test]
fn markdown_middle_press_on_a_link_starts_no_drag_of_what_it_is_in() {
  let (activations, record) = recorder();
  let drags = Arc::new(Mutex::new(Vec::new()));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<DraggableRoot>(
    &mut app,
    DraggableProps {
      markdown: MarkdownProps::new(TWO_LINKS).on_link_activate(record).width(500.0),
      drags: drags.clone(),
    },
  );
  pass(&mut app, &mut tree);
  let first = bounds_of(&mut tree, "first");

  pointer_click(&mut tree, first.x + 3.0, first.y + 3.0, MouseButton::Middle);
  assert_eq!(
    taken(&activations),
    [Activation::pointer("/first", MouseButton::Middle)]
  );
  assert!(drags.lock().unwrap().is_empty(), "the press was the link's");

  pointer_click(&mut tree, first.x + 300.0, first.y + 3.0, MouseButton::Middle);
  assert_eq!(
    *drags.lock().unwrap(),
    [MouseButton::Middle],
    "a middle press off the links still drags"
  );
}

#[test]
fn markdown_synthetic_middle_click_activates_a_link_as_synthetic_input() {
  let (activations, record) = recorder();
  let (_app, mut tree) = mount(MarkdownProps::new(TWO_LINKS).on_link_activate(record).width(500.0));
  let second = bounds_of(&mut tree, "second");
  let (x, y) = (second.x + 3.0, second.y + 3.0);

  let input = lurq::app::synthetic_input::SyntheticInput::click_button(x, y, MouseButton::Middle)
    .with_modifiers(lurq::app::synthetic_input::SyntheticModifiers::default().ctrl().meta());
  lurq::app::synthetic_input::apply(&mut tree, &input);
  with_synthetic_input(|| pointer_click(&mut tree, x, y, MouseButton::Left));

  assert_eq!(
    taken(&activations),
    [
      Activation {
        ctrl: true,
        meta: true,
        synthetic: true,
        ..Activation::pointer("/second", MouseButton::Middle)
      },
      Activation {
        synthetic: true,
        ..Activation::pointer("/second", MouseButton::Left)
      },
    ]
  );
}

#[test]
fn markdown_middle_click_without_on_link_activate_does_nothing() {
  let clicks = Arc::new(Mutex::new(0));
  let on_click = {
    let clicks = clicks.clone();
    move |_: &MarkdownLink| *clicks.lock().unwrap() += 1
  };
  let (_app, mut tree) = mount(MarkdownProps::new(TWO_LINKS).on_link_click(on_click).width(500.0));
  let first = bounds_of(&mut tree, "first");

  pointer_click(&mut tree, first.x + 3.0, first.y + 3.0, MouseButton::Middle);
  assert_eq!(*clicks.lock().unwrap(), 0);
  pointer_click(&mut tree, first.x + 3.0, first.y + 3.0, MouseButton::Left);
  assert_eq!(*clicks.lock().unwrap(), 1);
}
