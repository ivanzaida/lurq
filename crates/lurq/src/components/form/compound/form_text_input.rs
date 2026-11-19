use std::sync::Arc;

use super::form_field::form_field;
use crate::{
  app::ctx::Ctx,
  components::Control,
  node::{Element, dimension::Dimension},
  ui::{self, InputPart, Length, TextareaPart},
};

#[derive(Clone, Debug, PartialEq, crate::DevtoolsInspectable)]
pub struct FormTextInputProps {
  pub control: Control<String>,
  pub label: Option<Arc<str>>,
  pub hint: Option<Arc<str>>,
  pub placeholder: Option<Arc<str>>,
  #[devtools_ignore]
  pub height: Option<Dimension>,
  pub multiline: bool,
}

impl FormTextInputProps {
  pub fn new(control: Control<String>) -> Self {
    Self {
      control,
      label: None,
      hint: None,
      placeholder: None,
      height: None,
      multiline: false,
    }
  }

  pub fn label(mut self, label: impl Into<Arc<str>>) -> Self {
    self.label = Some(label.into());
    self
  }

  pub fn hint(mut self, hint: impl Into<Arc<str>>) -> Self {
    self.hint = Some(hint.into());
    self
  }

  pub fn placeholder(mut self, placeholder: impl Into<Arc<str>>) -> Self {
    self.placeholder = Some(placeholder.into());
    self
  }

  pub fn height(mut self, height: impl Into<Dimension>) -> Self {
    self.height = Some(height.into());
    self
  }

  pub fn multiline(mut self) -> Self {
    self.multiline = true;
    self
  }
}

/// A text field of a form: the design system's [`ui::Input`] in a [`ui::Field`], or its [`ui::Textarea`] where it is
/// [`multiline`](FormTextInputProps::multiline), bound to a string control. Its label is above the box and its hint
/// under it; the form's error takes the hint's place and makes the box invalid, and an edit clears it. The box is
/// as wide as the field, which is as wide as what holds it, and as high as the theme's control (36) unless it is
/// given a [`height`](FormTextInputProps::height). It is styled by the theme's recipes for `ui::Field`,
/// `ui::Input` and `ui::Textarea`.
pub struct FormTextInput;

impl crate::app::component::Component for FormTextInput {
  type Props = FormTextInputProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Self::Props>().clone();
    let control = ctx.form_control(&props.control);
    let value = control.value();

    let clear_error_form = props.control.form();
    let clear_error_name = Arc::<str>::from(props.control.name());
    ctx.watch(&value, move |_| {
      clear_error_form.clear_error(&clear_error_name);
    });

    let height = props.height.map(length);
    let (label, hint) = (props.label.as_deref(), props.hint.as_deref());
    if props.multiline {
      let mut area = ui::Textarea::new(value).name(control.name()).full_width(true);
      if let Some(placeholder) = props.placeholder.as_deref() {
        area = area.placeholder(placeholder);
      }
      if let Some(height) = height {
        area = area.style(TextareaPart::Control, |s| s.height(height));
      }
      form_field(&control, label, hint, area)
    } else {
      let mut input = ui::Input::new(value).name(control.name()).full_width(true);
      if let Some(placeholder) = props.placeholder.as_deref() {
        input = input.placeholder(placeholder);
      }
      if let Some(height) = height {
        input = input.style(InputPart::Control, |s| s.height(height));
      }
      form_field(&control, label, hint, input)
    }
  }
}

/// A height given as the engine's dimension, as a part's style takes it: a share of the room is all of it.
fn length(height: Dimension) -> Length {
  match height {
    Dimension::Px(px) => Length::Px(px),
    Dimension::Pct(_) => Length::Fill,
    Dimension::Auto => Length::Auto,
  }
}
