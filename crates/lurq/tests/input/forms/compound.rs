use lurq::{
  app::{
    App, Tree,
    component::Component,
    ctx::Ctx,
    events::MouseButton,
    theme::{FormFieldStyle, FormTheme},
  },
  components::{
    Column, ErrorVisibility, FormCheckboxInput, FormCheckboxInputProps, FormControlField, FormFieldProps, FormHandle,
    FormOptions, FormPrimaryButton, FormPrimaryButtonProps, FormSecondaryButton, FormSecondaryButtonProps,
    FormSliderInput, FormSliderInputProps, FormTextInput, FormTextInputProps, Text, ValidationResult,
  },
  layout::{quad::QuadContent, text_style::TextStyle},
  node::{Element, color::Color},
};

use crate::support::{pointer_click, render_pass, run_pass};

struct CompoundFieldHost {
  form: FormHandle,
  invalid: bool,
}

impl Component for CompoundFieldHost {
  type Props = bool;

  fn create(ctx: &mut Ctx) -> Self {
    let invalid = *ctx.props::<Self::Props>();
    let form = ctx.form(
      FormOptions::new()
        .field("email", "")
        .validate_string("email", |_value, _values| {
          ValidationResult::invalid("Email is required")
        }),
    );
    if invalid {
      form.validate_field("email");
    }
    Self { form, invalid }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let control = self.form.string_control("email").error_visibility(if self.invalid {
      ErrorVisibility::Always
    } else {
      ErrorVisibility::Never
    });
    ctx.form_view(self.form.clone(), |ctx| {
      Column::new().child(
        ctx.mount_with::<FormControlField<String>>(
          FormFieldProps::new(control)
            .label("Email")
            .hint("We'll never share it."),
          vec![Text::new("input-slot").into()],
        ),
      )
    })
  }
}

#[test]
fn form_control_field_renders_label_slot_child_and_hint() {
  let mut tree = Tree::new();
  tree.mount_root::<CompoundFieldHost>(&mut App::new(), false);
  run_pass(&mut tree);

  assert!(
    tree
      .find_element(|element| element.text_content() == Some("Email"))
      .is_some()
  );
  assert!(
    tree
      .find_element(|element| element.text_content() == Some("input-slot"))
      .is_some()
  );
  assert!(
    tree
      .find_element(|element| element.text_content() == Some("We'll never share it."))
      .is_some()
  );
}

#[test]
fn form_control_field_renders_visible_error_instead_of_hint() {
  let mut tree = Tree::new();
  tree.mount_root::<CompoundFieldHost>(&mut App::new(), true);
  run_pass(&mut tree);

  assert!(
    tree
      .find_element(|element| element.text_content() == Some("Email is required"))
      .is_some()
  );
  assert!(
    tree
      .find_element(|element| element.text_content() == Some("We'll never share it."))
      .is_none()
  );
}

#[test]
fn form_control_field_uses_theme_field_styles() {
  let mut app = App::new();
  let mut form_theme = FormTheme::default();
  form_theme.field = FormFieldStyle {
    label: TextStyle {
      color: Color::from_hex("#123456"),
      ..form_theme.field.label.clone()
    },
    ..form_theme.field
  };
  app.theme().set_form(form_theme);

  let mut tree = Tree::new();
  tree.mount_root::<CompoundFieldHost>(&mut app, false);
  run_pass(&mut tree);

  let label_style = text_quad_style(&tree, "Email").expect("label text quad should render");
  assert_eq!(label_style.color, Color::from_hex("#123456"));
}

struct BuiltinInputsHost {
  form: FormHandle,
}

impl Component for BuiltinInputsHost {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      form: ctx.form(
        FormOptions::new()
          .field("email", "")
          .field("enabled", false)
          .field("volume", 0.0),
      ),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let volume = self.form.number("volume").get();
    let volume_label = format!("volume={volume:.0}");
    ctx.form_view(self.form.clone(), |ctx| {
      Column::new()
        .width(200.0)
        .child(ctx.mount::<FormTextInput>(FormTextInputProps::new(self.form.string_control("email")).label("Email")))
        .child(
          ctx.mount::<FormCheckboxInput>(
            FormCheckboxInputProps::new(self.form.bool_control("enabled")).label("Enabled"),
          ),
        )
        .child(
          ctx.mount::<FormSliderInput>(
            FormSliderInputProps::new(self.form.number_control("volume"))
              .label("Volume")
              .range(0, 10),
          ),
        )
        .child(Text::new(&volume_label))
    })
  }
}

#[test]
fn builtin_form_inputs_render_field_labels_and_primitives() {
  let mut tree = Tree::new();
  tree.mount_root::<BuiltinInputsHost>(&mut App::new(), ());
  run_pass(&mut tree);

  for label in ["Email", "Enabled", "Volume"] {
    assert!(
      tree
        .find_element(|element| element.text_content() == Some(label))
        .is_some(),
      "{label} label should render"
    );
  }
  for tag in ["TextInput", "Checkbox", "Slider"] {
    assert!(
      tree.find_element(|element| element.tag_name() == tag).is_some(),
      "{tag} primitive should render"
    );
  }
}

#[test]
fn builtin_form_slider_updates_number_control() {
  let mut tree = Tree::new();
  tree.mount_root::<BuiltinInputsHost>(&mut App::new(), ());
  run_pass(&mut tree);
  let rect = tree
    .find_element(|element| element.tag_name() == "Slider")
    .expect("slider should render")
    .bounds();

  pointer_click(
    &mut tree,
    rect.x + rect.width,
    rect.y + rect.height / 2.0,
    MouseButton::Left,
  );
  run_pass(&mut tree);

  assert!(
    tree
      .find_element(|element| element.text_content() == Some("volume=10"))
      .is_some()
  );
}

#[test]
fn builtin_form_text_input_focus_style_renders_on_input_frame() {
  let mut tree = Tree::new();
  tree.mount_root::<BuiltinInputsHost>(&mut App::new(), ());
  run_pass(&mut tree);
  let rect = tree
    .find_element(|element| element.tag_name() == "TextInput")
    .expect("text input should render")
    .bounds();
  let (x, y) = rect.center();

  pointer_click(&mut tree, x, y, MouseButton::Left);
  let focused = render_pass(&mut tree);
  let focused_border = Color::new(13, 110, 253, 255);
  let focused_borders = focused
    .rects
    .iter()
    .filter(|rect| rect.stroke_color == focused_border && rect.stroke == [1.0; 4])
    .collect::<Vec<_>>();

  assert_eq!(focused_borders.len(), 1, "form input focused border should render once");
  assert_eq!(focused_borders[0].width, 200.0);
  assert_eq!(focused_borders[0].height, 36.0);
}

struct BuiltinButtonsHost {
  form: FormHandle,
  submitted: lurq::core::Signal<bool>,
  secondary_clicked: lurq::core::Signal<bool>,
}

impl Component for BuiltinButtonsHost {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let submitted = ctx.signal(false);
    let secondary_clicked = ctx.signal(false);
    let submitted_for_form = submitted.clone();
    let form = ctx.form(FormOptions::new()).on_submit(move |_| {
      submitted_for_form.set(true);
    });
    Self {
      form,
      submitted,
      secondary_clicked,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let submitted = self.submitted.get();
    let secondary_clicked = self.secondary_clicked.get();
    let submitted_label = format!("submitted={submitted}");
    let secondary_label = format!("secondary_clicked={secondary_clicked}");
    let secondary_clicked_for_button = self.secondary_clicked.clone();

    ctx.form_view(self.form.clone(), |ctx| {
      Column::new()
        .width(200.0)
        .child(ctx.mount::<FormPrimaryButton>(FormPrimaryButtonProps::new("Save")))
        .child(
          ctx.mount::<FormSecondaryButton>(FormSecondaryButtonProps::new("Cancel").on_click(move |_| {
            secondary_clicked_for_button.set(true);
          })),
        )
        .child(Text::new(&submitted_label))
        .child(Text::new(&secondary_label))
    })
  }
}

#[test]
fn builtin_form_buttons_render_labels() {
  let mut tree = Tree::new();
  tree.mount_root::<BuiltinButtonsHost>(&mut App::new(), ());
  run_pass(&mut tree);

  for label in ["Save", "Cancel"] {
    assert!(
      tree
        .find_element(|element| element.text_content() == Some(label))
        .is_some(),
      "{label} button label should render"
    );
  }
}

#[test]
fn primary_button_submits_by_default_and_secondary_button_does_not() {
  let mut tree = Tree::new();
  tree.mount_root::<BuiltinButtonsHost>(&mut App::new(), ());
  run_pass(&mut tree);
  let save = tree
    .find_element(|element| element.text_content() == Some("Save"))
    .expect("save label should render")
    .bounds();
  let cancel = tree
    .find_element(|element| element.text_content() == Some("Cancel"))
    .expect("cancel label should render")
    .bounds();

  pointer_click(&mut tree, cancel.center().0, cancel.center().1, MouseButton::Left);
  run_pass(&mut tree);

  assert!(
    tree
      .find_element(|element| element.text_content() == Some("secondary_clicked=true"))
      .is_some()
  );
  assert!(
    tree
      .find_element(|element| element.text_content() == Some("submitted=false"))
      .is_some()
  );

  pointer_click(&mut tree, save.center().0, save.center().1, MouseButton::Left);
  run_pass(&mut tree);

  assert!(
    tree
      .find_element(|element| element.text_content() == Some("submitted=true"))
      .is_some()
  );
}

fn text_quad_style(tree: &Tree, expected: &str) -> Option<TextStyle> {
  tree
    .resolve_quads(tree.last_layout()?)
    .into_iter()
    .find_map(|quad| match quad.content {
      QuadContent::Text { text, style, .. } if text == expected => Some(style),
      _ => None,
    })
}
