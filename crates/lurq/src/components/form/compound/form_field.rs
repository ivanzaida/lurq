use std::{marker::PhantomData, sync::Arc};

use crate::{
  app::ctx::Ctx,
  components::{Column, Control, Text},
  core::SignalValue,
  node::Element,
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
    let visible_error = control.visible_error();
    let style = ctx.theme().form().field.clone();

    let mut field = Column::new().spacing(style.spacing);

    if let Some(label) = props.label.as_deref() {
      let label_style = if visible_error.is_some() {
        style.error.clone()
      } else {
        style.label.clone()
      };
      field = field.child(Text::styled(label, label_style));
    }

    for child in ctx.children() {
      field = field.child(child.clone());
    }

    if let Some(error) = visible_error {
      field = field.child(Text::styled(&error, style.error));
    } else if let Some(hint) = props.hint.as_deref() {
      field = field.child(Text::styled(hint, style.hint));
    }

    field
  }
}
