use std::sync::Arc;

use crate::{
  app::ctx::Ctx,
  components::{Checkbox, Control, FormControlField, FormFieldProps},
  node::Element,
};

#[derive(Clone, Debug, PartialEq, crate::DevtoolsInspectable)]
pub struct FormCheckboxInputProps {
  pub control: Control<bool>,
  pub label: Option<Arc<str>>,
  pub hint: Option<Arc<str>>,
}

impl FormCheckboxInputProps {
  pub fn new(control: Control<bool>) -> Self {
    Self {
      control,
      label: None,
      hint: None,
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
}

pub struct FormCheckboxInput;

impl crate::app::component::Component for FormCheckboxInput {
  type Props = FormCheckboxInputProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Self::Props>().clone();
    let control = ctx.form_control(&props.control);
    let checkbox_style = ctx.theme().form().checkbox.clone();
    let blur_control = control.clone();

    let checkbox = Checkbox::new(control.value())
      .name(control.name())
      .box_style(checkbox_style.box_style)
      .checked_box_style(checkbox_style.checked_box_style)
      .box_hovered_style(checkbox_style.box_hovered_style)
      .checked_box_hovered_style(checkbox_style.checked_box_hovered_style)
      .on_blur(move || {
        blur_control.mark_touched();
        blur_control.validate();
      });

    ctx.mount_with::<FormControlField<bool>>(
      FormFieldProps::new(props.control)
        .maybe_label(props.label)
        .maybe_hint(props.hint),
      vec![checkbox.into()],
    )
  }
}
