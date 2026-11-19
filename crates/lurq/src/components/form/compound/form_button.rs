use std::{fmt, sync::Arc};

use crate::{
  app::{ctx::Ctx, events::MouseEvent},
  node::Element,
  ui::{self, ButtonVariant},
};

type ClickCallback = Arc<dyn Fn(&MouseEvent) + Send + Sync>;

#[derive(Clone, crate::DevtoolsInspectable)]
pub struct FormPrimaryButtonProps {
  pub label: Arc<str>,
  pub submit: bool,
  #[devtools_ignore]
  pub on_click: Option<ClickCallback>,
}

#[derive(Clone, crate::DevtoolsInspectable)]
pub struct FormSecondaryButtonProps {
  pub label: Arc<str>,
  pub submit: bool,
  #[devtools_ignore]
  pub on_click: Option<ClickCallback>,
}

impl FormPrimaryButtonProps {
  pub fn new(label: impl Into<Arc<str>>) -> Self {
    Self {
      label: label.into(),
      submit: true,
      on_click: None,
    }
  }

  pub fn submit(mut self) -> Self {
    self.submit = true;
    self
  }

  pub fn button(mut self) -> Self {
    self.submit = false;
    self
  }

  pub fn on_click(mut self, f: impl Fn(&MouseEvent) + Send + Sync + 'static) -> Self {
    self.on_click = Some(Arc::new(f));
    self
  }
}

impl FormSecondaryButtonProps {
  pub fn new(label: impl Into<Arc<str>>) -> Self {
    Self {
      label: label.into(),
      submit: false,
      on_click: None,
    }
  }

  pub fn submit(mut self) -> Self {
    self.submit = true;
    self
  }

  pub fn button(mut self) -> Self {
    self.submit = false;
    self
  }

  pub fn on_click(mut self, f: impl Fn(&MouseEvent) + Send + Sync + 'static) -> Self {
    self.on_click = Some(Arc::new(f));
    self
  }
}

impl fmt::Debug for FormPrimaryButtonProps {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    debug_button_props(f, "FormPrimaryButtonProps", &self.label, self.submit, &self.on_click)
  }
}

impl fmt::Debug for FormSecondaryButtonProps {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    debug_button_props(f, "FormSecondaryButtonProps", &self.label, self.submit, &self.on_click)
  }
}

impl PartialEq for FormPrimaryButtonProps {
  fn eq(&self, other: &Self) -> bool {
    self.label == other.label && self.submit == other.submit && same_callback(&self.on_click, &other.on_click)
  }
}

impl PartialEq for FormSecondaryButtonProps {
  fn eq(&self, other: &Self) -> bool {
    self.label == other.label && self.submit == other.submit && same_callback(&self.on_click, &other.on_click)
  }
}

/// The button of a form's main action: the design system's [`ui::Button`] in its primary variant, as wide as what
/// holds it, which submits the form unless it is made a plain [`button`](FormPrimaryButtonProps::button). It is
/// styled by the theme's recipe for `ui::Button`.
pub struct FormPrimaryButton;

impl crate::app::component::Component for FormPrimaryButton {
  type Props = FormPrimaryButtonProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Self::Props>().clone();
    form_button(&props.label, props.submit, props.on_click, ButtonVariant::Primary)
  }
}

/// The button of a form's lesser action (cancel, go back): the design system's [`ui::Button`] in its outline
/// variant, as wide as what holds it, which submits nothing unless it is made a
/// [`submit`](FormSecondaryButtonProps::submit) button. It is styled by the theme's recipe for `ui::Button`.
pub struct FormSecondaryButton;

impl crate::app::component::Component for FormSecondaryButton {
  type Props = FormSecondaryButtonProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Self::Props>().clone();
    form_button(&props.label, props.submit, props.on_click, ButtonVariant::Outline)
  }
}

fn form_button(label: &str, submit: bool, on_click: Option<ClickCallback>, variant: ButtonVariant) -> Element {
  let mut button = ui::Button::new(label).variant(variant).full_width(true).submit(submit);
  if let Some(on_click) = on_click {
    button = button.on_click(move |event: MouseEvent| on_click(&event));
  }
  button.into()
}

fn debug_button_props(
  f: &mut fmt::Formatter<'_>,
  name: &str,
  label: &Arc<str>,
  submit: bool,
  on_click: &Option<ClickCallback>,
) -> fmt::Result {
  f.debug_struct(name)
    .field("label", label)
    .field("submit", &submit)
    .field("on_click", &on_click.as_ref().map(|_| "<callback>"))
    .finish()
}

fn same_callback(left: &Option<ClickCallback>, right: &Option<ClickCallback>) -> bool {
  match (left, right) {
    (Some(left), Some(right)) => Arc::ptr_eq(left, right),
    (None, None) => true,
    _ => false,
  }
}
