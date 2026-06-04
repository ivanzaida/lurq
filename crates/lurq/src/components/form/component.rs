use std::{fmt, sync::Arc};

use super::{FormContext, FormErrors, FormHandle, FormValues};
use crate::{
  app::{
    component::Component,
    ctx::{Ctx, FutureAction},
  },
  core::signal::SignalValue,
  node::{Element, FormData, Node},
};

type FormSubmitHandler = Arc<dyn Fn(FormData) + Send + Sync>;

#[derive(Clone, Default, crate::DevtoolsInspectable)]
pub struct FormProps {
  #[devtools_ignore]
  pub form: Option<FormHandle>,
  #[devtools_ignore]
  on_submit: Option<FormSubmitHandler>,
  #[devtools_ignore]
  child: Option<Element>,
}

impl fmt::Debug for FormProps {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("FormProps")
      .field("form", &self.form)
      .field("on_submit", &self.on_submit.as_ref().map(|_| "<submit handler>"))
      .field("child", &self.child.as_ref().map(|_| "<slot child>"))
      .finish()
  }
}

impl PartialEq for FormProps {
  fn eq(&self, other: &Self) -> bool {
    self.form == other.form && self.child.is_none() && other.child.is_none()
  }
}

impl FormProps {
  pub fn new(form: FormHandle) -> Self {
    Self {
      form: Some(form),
      on_submit: None,
      child: None,
    }
  }

  pub fn on_submit_data(mut self, on_submit: impl Fn(FormData) + Send + Sync + 'static) -> Self {
    self.on_submit = Some(Arc::new(on_submit));
    self
  }

  pub fn submit_action<T>(mut self, action: FutureAction<FormValues, T, FormErrors>) -> Self
  where
    T: SignalValue + Clone + PartialEq + Send + Sync + 'static,
  {
    if let Some(form) = self.form.clone() {
      self.on_submit = Some(Arc::new(move |data| {
        form.submit_action(data, &action);
      }));
    }
    self
  }
}

pub struct Form;

impl Form {
  pub fn mount(ctx: &mut Ctx, mut props: FormProps, child: impl Into<Element>) -> Element {
    props.child = Some(child.into());
    ctx.mount::<Self>(props)
  }

  pub fn element(props: FormProps, child: impl Into<Element>) -> Element {
    form_node(props, child.into())
  }
}

impl Component for Form {
  type Props = FormProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Self::Props>().clone();
    if let Some(form) = props.form.clone() {
      ctx.provide(FormContext::new(form));
    }
    let child = form_child(ctx, &props);
    form_node(props, child)
  }
}

fn form_node(props: FormProps, child: Element) -> Element {
  let mut node = Node::logical().with_tag_name("Form");

  if let Some(on_submit) = props.on_submit {
    node = node.form(move |data| on_submit(data));
  } else if let Some(form) = props.form {
    node = node.form(move |data| form.submit(data));
  }

  Element::from_node(node.child(child.node))
}

fn form_child(ctx: &Ctx, props: &FormProps) -> Element {
  if let Some(child) = props.child.clone() {
    assert!(
      ctx.children().is_empty(),
      "Form accepts either an explicit child via Form::mount or one slot child, not both"
    );
    return child;
  }

  match ctx.children() {
    [] => Element::new(),
    [child] => child.clone(),
    children => panic!(
      "Form accepts exactly one child; wrap multiple children in Column, Row, or Stack. Got {} children",
      children.len()
    ),
  }
}
