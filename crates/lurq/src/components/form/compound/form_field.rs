use std::{marker::PhantomData, sync::Arc};

use crate::{
  app::ctx::Ctx,
  components::{Column, Control, ResolvedControl},
  core::SignalValue,
  layout::Alignment,
  node::{Element, dimension::Dimension},
  ui::{self, FieldControl},
};

#[derive(Clone, Debug, PartialEq, crate::DevtoolsInspectable)]
pub struct FormFieldProps<T: SignalValue> {
  pub control: Control<T>,
  pub label: Option<Arc<str>>,
  pub hint: Option<Arc<str>>,
}

impl<T: SignalValue> FormFieldProps<T> {
  pub fn new(control: Control<T>) -> Self {
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

  pub fn maybe_label(mut self, label: Option<Arc<str>>) -> Self {
    self.label = label;
    self
  }

  pub fn maybe_hint(mut self, hint: Option<Arc<str>>) -> Self {
    self.hint = hint;
    self
  }
}

/// A field of a form around what it is mounted with: the design system's [`ui::Field`], told by the form what it
/// says of the control. Its label is above what it holds and its hint under it, or in the hint's place the error
/// the form shows ([`ResolvedControl::visible_error`]), in the destructive color; a required control
/// ([`Control::required`]) has the field's mark after its label, and a disabled one, or one of a form that is
/// submitting, is disabled. It is as wide as what holds it. It is styled by the theme's recipe for `ui::Field`.
pub struct FormField<T: SignalValue> {
  marker: PhantomData<T>,
}

impl<T> crate::app::component::Component for FormField<T>
where
  T: SignalValue + Clone + PartialEq + Send + Sync + 'static,
{
  type Props = FormFieldProps<T>;

  fn create(_ctx: &mut Ctx) -> Self {
    Self { marker: PhantomData }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Self::Props>().clone();
    let control = ctx.form_control(&props.control);
    let children = ctx.children().to_vec();
    // What it is mounted with is held as it is: one element, or several one under another.
    let held: Element = match <[Element; 1]>::try_from(children) {
      Ok([child]) => child,
      Err(children) => Column::new()
        .width(Dimension::Pct(100.0))
        .align_items(Alignment::Stretch)
        .spacing(6.0)
        .with_children(children)
        .into(),
    };
    form_field(&control, props.label.as_deref(), props.hint.as_deref(), held)
  }
}

/// The [`ui::Field`] a control of a form stands in, built: called `label` (none above the control where it has
/// none), with `hint` under the control, or the error the form shows of it in the hint's place; with the mark of a
/// required field where the control is required, and disabled where the control is (a form that is submitting
/// disables its controls). As wide as what holds it, as a form's field was.
pub(super) fn form_field<T>(
  control: &ResolvedControl<T>,
  label: Option<&str>,
  hint: Option<&str>,
  held: impl FieldControl + 'static,
) -> Element
where
  T: SignalValue + Clone + PartialEq + Send + Sync + 'static,
{
  let mut field = ui::Field::new(label.unwrap_or(""), held)
    .full_width(true)
    .required(control.is_required())
    .disabled(control.is_disabled());
  if let Some(hint) = hint {
    field = field.description(hint);
  }
  if let Some(error) = control.visible_error() {
    field = field.error(&error);
  }
  field.into()
}
