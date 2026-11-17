use std::sync::Arc;

use super::form_field::form_field;
use crate::{app::ctx::Ctx, components::Control, node::Element, ui};

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

/// A checkbox of a form: the design system's [`ui::Checkbox`] bound to a boolean control, with its label beside the
/// box and its hint under it in a [`ui::Field`]. The form's error takes the hint's place and makes the box invalid;
/// the form judges the field when the box loses the focus. A checked box gives the form `on` under the control's
/// name. It is styled by the theme's recipes for `ui::Checkbox` and `ui::Field`.
pub struct FormCheckboxInput;

impl crate::app::component::Component for FormCheckboxInput {
  type Props = FormCheckboxInputProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Self::Props>().clone();
    let control = ctx.form_control(&props.control);
    let blur_control = control.clone();

    // With no label the box stands alone, and the field names it nothing.
    let label = props.label.as_deref().unwrap_or("");
    let checkbox = ui::Checkbox::new(label)
      .label_hidden(label.is_empty())
      .bind(control.value())
      .name(control.name())
      .on_leave(move || {
        blur_control.mark_touched();
        blur_control.validate();
      });

    // The label is the checkbox's, beside its box: the field has none above it.
    form_field(&control, None, props.hint.as_deref(), checkbox)
  }
}
