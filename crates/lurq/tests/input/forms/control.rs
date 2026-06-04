use std::sync::{Arc, Mutex};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx, events::MouseButton},
  components::{Button, Column, FormHandle, FormOptions, FormValues, Text, TextInput, validators},
  layout::text_style::TextStyle,
  node::{Element, color::Color, dimension::Dimension},
};

use crate::support::{RectSnapshot, pointer_click, render_pass, run_pass};

#[derive(Clone, Debug, PartialEq, lurq::DevtoolsInspectable)]
struct TextControlProps {
  name: Arc<str>,
  label: Arc<str>,
}

struct ReusableTextControl;

impl Component for ReusableTextControl {
  type Props = TextControlProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Self::Props>().clone();
    let control = ctx.string_control(props.name.clone());
    let error = control.visible_error();

    Column::new()
      .child(Text::new(&props.label))
      .child(Text::new(&format!("field={}", control.name())))
      .child(
        TextInput::new(control.value())
          .name(control.name())
          .single_line()
          .on_blur(control.on_blur()),
      )
      .child(match error {
        Some(error) => Text::new(&error),
        None => Text::new(""),
      })
  }
}

#[derive(Clone)]
struct Submitted(Arc<Mutex<Option<FormValues>>>);

#[cfg(feature = "devtools")]
impl lurq::app::component::DevtoolsInspectable for Submitted {
  fn write_info(&self, _buffer: &mut Vec<lurq::app::component::ComponentInfo>) {}
}

impl PartialEq for Submitted {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

struct NameResolvedFormRoot {
  form: FormHandle,
}

impl Component for NameResolvedFormRoot {
  type Props = Submitted;

  fn create(ctx: &mut Ctx) -> Self {
    let submitted = ctx.props::<Self::Props>().clone();
    let form = ctx
      .form(
        FormOptions::new()
          .field("email", "")
          .validate_string("email", validators::required("Email is required")),
      )
      .on_submit(move |values| {
        *submitted.0.lock().unwrap() = Some(values);
      });
    Self { form }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    ctx.form_view(self.form.clone(), |ctx| {
      Column::new()
        .child(ctx.mount::<ReusableTextControl>(TextControlProps {
          name: Arc::from("email"),
          label: Arc::from("Email"),
        }))
        .child(Button::new("Save").submit())
    })
  }
}

#[test]
fn reusable_control_resolves_name_from_nearest_form_context() {
  let submitted = Arc::new(Mutex::new(None));
  let mut tree = Tree::new();
  tree.mount_root::<NameResolvedFormRoot>(&mut App::new(), Submitted(submitted.clone()));
  run_pass(&mut tree);

  assert!(
    tree
      .find_element(|element| element.text_content() == Some("field=email"))
      .is_some()
  );

  let input = tree
    .find_element(|element| element.tag_name() == "TextInput")
    .expect("text input should render")
    .bounds();
  let (x, y) = input.center();
  pointer_click(&mut tree, x, y, MouseButton::Left);
  tree.key_down("A".to_owned(), "KeyA".to_owned(), false, false, false);
  run_pass(&mut tree);

  let button = tree
    .find_element(|element| element.text_content() == Some("Save"))
    .expect("submit button should render")
    .bounds();
  pointer_click(
    &mut tree,
    button.x + button.width / 2.0,
    button.y + button.height / 2.0,
    MouseButton::Left,
  );

  let values = submitted.lock().unwrap().clone().expect("form should submit");
  assert_eq!(values.get_string("email"), Some("A"));
}

#[test]
fn reusable_control_visible_error_tracks_submit_attempted_state() {
  let submitted = Arc::new(Mutex::new(None));
  let mut tree = Tree::new();
  tree.mount_root::<NameResolvedFormRoot>(&mut App::new(), Submitted(submitted));
  run_pass(&mut tree);

  assert!(
    tree
      .find_element(|element| element.text_content() == Some("Email is required"))
      .is_none()
  );

  let button = tree
    .find_element(|element| element.text_content() == Some("Save"))
    .expect("submit button should render")
    .bounds();
  pointer_click(
    &mut tree,
    button.x + button.width / 2.0,
    button.y + button.height / 2.0,
    MouseButton::Left,
  );
  run_pass(&mut tree);

  assert!(
    tree
      .find_element(|element| element.text_content() == Some("Email is required"))
      .is_some()
  );
}

struct StyledReusableTextControl;

impl Component for StyledReusableTextControl {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let control = ctx.string_control("email");
    let has_error = control.visible_error().is_some();
    let background = if has_error { "#33191b" } else { "#111827" };
    let border = if has_error { "#f05d5e" } else { "#30343a" };

    Column::new().width(240.0).child(
      TextInput::styled(
        control.value(),
        TextStyle {
          color: Color::from_hex("#f4f4f2"),
          font_size: 12.0,
          ..TextStyle::default()
        },
      )
      .name(control.name())
      .width(Dimension::Pct(100.0))
      .height(40.0)
      .padding_horizontal(10.0)
      .rounded(5.0)
      .background(background)
      .border_inside(1.0, Color::from_hex(border))
      .single_line(),
    )
  }
}

struct StyledControlFormRoot {
  form: FormHandle,
}

impl Component for StyledControlFormRoot {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      form: ctx.form(
        FormOptions::new()
          .field("email", "")
          .validate_string("email", validators::required("Email is required")),
      ),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    ctx.form_view(self.form.clone(), |ctx| {
      Column::new()
        .child(ctx.mount::<StyledReusableTextControl>(()))
        .child(Button::new("Save").submit())
    })
  }
}

#[test]
fn reusable_control_styled_input_visuals_update_after_submit_validation() {
  let mut tree = Tree::new();
  tree.mount_root::<StyledControlFormRoot>(&mut App::new(), ());

  let ok = render_pass(&mut tree);
  assert_input_visuals(&ok.rects, Color::from_hex("#111827"), Color::from_hex("#30343a"));

  let button = tree
    .find_element(|element| element.text_content() == Some("Save"))
    .expect("submit button should render")
    .bounds();
  pointer_click(
    &mut tree,
    button.x + button.width / 2.0,
    button.y + button.height / 2.0,
    MouseButton::Left,
  );

  let invalid = render_pass(&mut tree);
  assert_input_visuals(&invalid.rects, Color::from_hex("#33191b"), Color::from_hex("#f05d5e"));
}

fn assert_input_visuals(rects: &[RectSnapshot], fill: Color, border: Color) {
  let background = rects
    .iter()
    .find(|rect| rect.width == 240.0 && rect.height == 40.0 && rect.color == fill)
    .expect("expected rendered text input wrapper fill rect");
  assert_eq!(background.radii, [5.0; 4]);

  let border_rect = rects
    .iter()
    .find(|rect| {
      rect.width == 240.0
        && rect.height == 40.0
        && rect.color == Color::from_hex("#00000000")
        && rect.stroke == [1.0; 4]
    })
    .expect("expected rendered text input wrapper border rect");
  assert_eq!(border_rect.stroke_color, border);
}

#[test]
#[should_panic(expected = "form controls must be resolved inside a Form render context")]
fn name_based_control_requires_form_context() {
  struct OutsideForm;

  impl Component for OutsideForm {
    type Props = ();

    fn create(_ctx: &mut Ctx) -> Self {
      Self
    }

    fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
      let _control = ctx.string_control("email");
      Text::new("unreachable")
    }
  }

  let mut tree = Tree::new();
  tree.mount_root::<OutsideForm>(&mut App::new(), ());
}
