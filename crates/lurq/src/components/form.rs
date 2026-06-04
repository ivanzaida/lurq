use std::{any::Any, collections::BTreeMap, fmt, sync::Arc};

use parking_lot::Mutex;

pub use crate::node::FormData;
use crate::{
  app::{component::Component, ctx::Ctx},
  core::signal::{Signal, SignalValue},
  node::{Element, Node},
};

type SubmitCallback = Arc<dyn Fn(FormValues) + Send + Sync>;

#[derive(Clone, Debug, PartialEq)]
pub enum FormValue {
  String(Arc<str>),
  Number(f64),
  Bool(bool),
}

impl FormValue {
  pub fn as_str(&self) -> Option<&str> {
    match self {
      Self::String(value) => Some(value),
      _ => None,
    }
  }

  pub fn as_number(&self) -> Option<f64> {
    match self {
      Self::Number(value) => Some(*value),
      _ => None,
    }
  }

  pub fn as_bool(&self) -> Option<bool> {
    match self {
      Self::Bool(value) => Some(*value),
      _ => None,
    }
  }

  fn to_string_value(&self) -> String {
    match self {
      Self::String(value) => value.to_string(),
      Self::Number(value) => value.to_string(),
      Self::Bool(value) => value.to_string(),
    }
  }
}

impl From<&str> for FormValue {
  fn from(value: &str) -> Self {
    Self::String(Arc::from(value))
  }
}

impl From<String> for FormValue {
  fn from(value: String) -> Self {
    Self::String(Arc::from(value))
  }
}

impl From<Arc<str>> for FormValue {
  fn from(value: Arc<str>) -> Self {
    Self::String(value)
  }
}

impl From<bool> for FormValue {
  fn from(value: bool) -> Self {
    Self::Bool(value)
  }
}

impl From<i32> for FormValue {
  fn from(value: i32) -> Self {
    Self::Number(value as f64)
  }
}

impl From<f64> for FormValue {
  fn from(value: f64) -> Self {
    Self::Number(value)
  }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FormValues {
  fields: BTreeMap<Arc<str>, FormValue>,
}

impl FormValues {
  pub fn new() -> Self {
    <Self as Default>::default()
  }

  pub fn with(mut self, name: impl Into<Arc<str>>, value: impl Into<FormValue>) -> Self {
    self.set(name, value);
    self
  }

  pub fn set(&mut self, name: impl Into<Arc<str>>, value: impl Into<FormValue>) {
    self.fields.insert(name.into(), value.into());
  }

  pub fn get(&self, name: &str) -> Option<&FormValue> {
    self.fields.get(name)
  }

  pub fn get_string(&self, name: &str) -> Option<&str> {
    self.get(name).and_then(FormValue::as_str)
  }

  pub fn get_number(&self, name: &str) -> Option<f64> {
    self.get(name).and_then(FormValue::as_number)
  }

  pub fn get_bool(&self, name: &str) -> Option<bool> {
    self.get(name).and_then(FormValue::as_bool)
  }

  pub fn entries(&self) -> impl Iterator<Item = (&str, &FormValue)> {
    self.fields.iter().map(|(name, value)| (name.as_ref(), value))
  }

  pub fn len(&self) -> usize {
    self.fields.len()
  }

  pub fn is_empty(&self) -> bool {
    self.fields.is_empty()
  }
}

impl<const N: usize> From<[(&str, FormValue); N]> for FormValues {
  fn from(values: [(&str, FormValue); N]) -> Self {
    values
      .into_iter()
      .fold(Self::new(), |values, (name, value)| values.with(name, value))
  }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FormOptions {
  defaults: FormValues,
}

impl FormOptions {
  pub fn new() -> Self {
    <Self as Default>::default()
  }

  pub fn default(mut self, defaults: impl Into<FormValues>) -> Self {
    self.defaults = defaults.into();
    self
  }

  pub fn field(mut self, name: impl Into<Arc<str>>, value: impl Into<FormValue>) -> Self {
    self.defaults.set(name, value);
    self
  }
}

#[derive(Clone)]
pub struct FormHandle {
  inner: Arc<FormHandleInner>,
}

struct FormHandleInner {
  values: Mutex<FormValues>,
  strings: Mutex<BTreeMap<Arc<str>, Signal<String>>>,
  numbers: Mutex<BTreeMap<Arc<str>, Signal<f64>>>,
  bools: Mutex<BTreeMap<Arc<str>, Signal<bool>>>,
  on_submit: Mutex<Option<SubmitCallback>>,
  dirty_callback: Option<Arc<dyn Fn() + Send + Sync>>,
  watch_handles: Mutex<Vec<Box<dyn Any + Send + Sync>>>,
}

impl fmt::Debug for FormHandle {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_tuple("FormHandle").field(&Arc::as_ptr(&self.inner)).finish()
  }
}

impl PartialEq for FormHandle {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.inner, &other.inner)
  }
}

impl FormHandle {
  pub fn new(options: FormOptions) -> Self {
    Self::from_options(options, None)
  }

  pub(crate) fn with_dirty(options: FormOptions, dirty_callback: Arc<dyn Fn() + Send + Sync>) -> Self {
    Self::from_options(options, Some(dirty_callback))
  }

  fn from_options(options: FormOptions, dirty_callback: Option<Arc<dyn Fn() + Send + Sync>>) -> Self {
    Self {
      inner: Arc::new(FormHandleInner {
        values: Mutex::new(options.defaults),
        strings: Mutex::new(BTreeMap::new()),
        numbers: Mutex::new(BTreeMap::new()),
        bools: Mutex::new(BTreeMap::new()),
        on_submit: Mutex::new(None),
        dirty_callback,
        watch_handles: Mutex::new(Vec::new()),
      }),
    }
  }

  pub fn on_submit(self, on_submit: impl Fn(FormValues) + Send + Sync + 'static) -> Self {
    *self.inner.on_submit.lock() = Some(Arc::new(on_submit));
    self
  }

  pub fn string(&self, name: impl Into<Arc<str>>) -> Signal<String> {
    let name = name.into();
    if let Some(signal) = self.inner.strings.lock().get(&name).cloned() {
      return signal;
    }

    let initial = self
      .inner
      .values
      .lock()
      .get(&name)
      .map(FormValue::to_string_value)
      .unwrap_or_default();
    let signal = Signal::new(initial);
    self.watch_for_dirty(&signal);
    self.inner.strings.lock().insert(name, signal.clone());
    signal
  }

  pub fn number(&self, name: impl Into<Arc<str>>) -> Signal<f64> {
    let name = name.into();
    if let Some(signal) = self.inner.numbers.lock().get(&name).cloned() {
      return signal;
    }

    let initial = self
      .inner
      .values
      .lock()
      .get(&name)
      .and_then(FormValue::as_number)
      .unwrap_or_default();
    let signal = Signal::new(initial);
    self.watch_for_dirty(&signal);
    self.inner.numbers.lock().insert(name, signal.clone());
    signal
  }

  pub fn bool(&self, name: impl Into<Arc<str>>) -> Signal<bool> {
    let name = name.into();
    if let Some(signal) = self.inner.bools.lock().get(&name).cloned() {
      return signal;
    }

    let initial = self
      .inner
      .values
      .lock()
      .get(&name)
      .and_then(FormValue::as_bool)
      .unwrap_or_default();
    let signal = Signal::new(initial);
    self.watch_for_dirty(&signal);
    self.inner.bools.lock().insert(name, signal.clone());
    signal
  }

  pub fn values(&self) -> FormValues {
    self.inner.values.lock().clone()
  }

  pub fn submit(&self, data: FormData) {
    let strings = self
      .inner
      .strings
      .lock()
      .iter()
      .map(|(name, signal)| (name.clone(), signal.clone()))
      .collect::<Vec<_>>();
    let numbers = self
      .inner
      .numbers
      .lock()
      .iter()
      .map(|(name, signal)| (name.clone(), signal.clone()))
      .collect::<Vec<_>>();
    let bools = self
      .inner
      .bools
      .lock()
      .iter()
      .map(|(name, signal)| (name.clone(), signal.clone()))
      .collect::<Vec<_>>();

    let mut values = self.inner.values.lock().clone();
    for (name, value) in data.entries() {
      values.set(name.as_str(), value.to_owned());
    }

    for (name, signal) in strings {
      if let Some(value) = data.get(&name) {
        let value = value.to_owned();
        signal.set(value.clone());
        values.set(name, value);
      }
    }

    for (name, signal) in numbers {
      if let Some(value) = data.get(&name).and_then(|value| value.parse::<f64>().ok()) {
        signal.set(value);
        values.set(name, value);
      }
    }

    for (name, signal) in bools {
      let value = data.get(&name).is_some();
      signal.set(value);
      values.set(name, value);
    }

    *self.inner.values.lock() = values.clone();
    let on_submit = self.inner.on_submit.lock().clone();
    if let Some(on_submit) = on_submit {
      on_submit(values);
    }
  }

  fn watch_for_dirty<T>(&self, signal: &Signal<T>)
  where
    T: SignalValue + Send + Sync + 'static,
  {
    let Some(dirty_callback) = self.inner.dirty_callback.clone() else {
      return;
    };

    let handle = signal.watch(move || dirty_callback());
    self.inner.watch_handles.lock().push(Box::new(handle));
  }
}

#[derive(Clone, Default, crate::DevtoolsInspectable)]
pub struct FormProps {
  #[devtools_ignore]
  pub form: Option<FormHandle>,
  #[devtools_ignore]
  child: Option<Element>,
}

impl fmt::Debug for FormProps {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("FormProps")
      .field("form", &self.form)
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
      child: None,
    }
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
    let child = form_child(ctx, &props);
    form_node(props, child)
  }
}

fn form_node(props: FormProps, child: Element) -> Element {
  let mut node = Node::logical().with_tag_name("Form");

  if let Some(form) = props.form {
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
