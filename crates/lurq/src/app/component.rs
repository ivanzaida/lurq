use std::fmt::{Debug, Display};

use crate::{
  app::ctx::Ctx,
  core::{Signal, SignalValue},
  node::Element,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComponentInfo {
  name: String,
  type_name: String,
  formatted_value: Option<String>,
  children: Vec<ComponentInfo>,
}

impl ComponentInfo {
  pub fn new(name: impl Into<String>, type_name: impl Into<String>) -> Self {
    Self {
      name: name.into(),
      type_name: type_name.into(),
      formatted_value: None,
      children: Vec::new(),
    }
  }

  pub fn with_value(name: impl Into<String>, type_name: impl Into<String>, formatted_value: impl Into<String>) -> Self {
    Self {
      name: name.into(),
      type_name: type_name.into(),
      formatted_value: Some(formatted_value.into()),
      children: Vec::new(),
    }
  }

  pub fn with_children(name: impl Into<String>, type_name: impl Into<String>, children: Vec<ComponentInfo>) -> Self {
    Self {
      name: name.into(),
      type_name: type_name.into(),
      formatted_value: None,
      children,
    }
  }

  pub fn name(&self) -> &str {
    &self.name
  }

  pub fn value(&self) -> &str {
    &self.type_name
  }

  pub fn type_name(&self) -> &str {
    &self.type_name
  }

  pub fn formatted_value(&self) -> Option<&str> {
    self.formatted_value.as_deref()
  }

  pub fn children(&self) -> &[ComponentInfo] {
    &self.children
  }

  #[cfg(feature = "devtools")]
  pub(crate) fn estimated_memory_bytes(&self) -> usize {
    self.name.capacity()
      + self.type_name.capacity()
      + self.formatted_value.as_ref().map(|value| value.capacity()).unwrap_or(0)
      + self.children.capacity() * std::mem::size_of::<ComponentInfo>()
      + self
        .children
        .iter()
        .map(ComponentInfo::estimated_memory_bytes)
        .sum::<usize>()
  }
}

pub struct DevtoolsFormatter<'a> {
  buffer: &'a mut Vec<ComponentInfo>,
}

impl<'a> DevtoolsFormatter<'a> {
  pub fn new(buffer: &'a mut Vec<ComponentInfo>) -> Self {
    Self { buffer }
  }

  pub fn buffer_mut(&mut self) -> &mut Vec<ComponentInfo> {
    self.buffer
  }

  pub fn debug_struct<'b>(&'b mut self, type_name: impl Into<String>) -> DevtoolsStruct<'b, 'a> {
    DevtoolsStruct {
      formatter: self,
      _type_name: type_name.into(),
    }
  }

  pub fn debug_tuple<'b>(&'b mut self, type_name: impl Into<String>) -> DevtoolsTuple<'b, 'a> {
    DevtoolsTuple {
      formatter: self,
      _type_name: type_name.into(),
      index: 0,
    }
  }

  pub fn debug_list<'b>(&'b mut self) -> DevtoolsList<'b, 'a> {
    DevtoolsList {
      formatter: self,
      index: 0,
    }
  }

  pub fn debug_set<'b>(&'b mut self) -> DevtoolsSet<'b, 'a> {
    DevtoolsSet {
      formatter: self,
      index: 0,
    }
  }

  pub fn debug_map<'b>(&'b mut self) -> DevtoolsMap<'b, 'a> {
    DevtoolsMap { formatter: self }
  }

  pub fn value_debug<T: Debug + ?Sized>(&mut self, value: &T) {
    self.value(std::any::type_name::<T>(), format!("{value:?}"));
  }

  pub fn value_display<T: Display + ?Sized>(&mut self, value: &T) {
    self.value(std::any::type_name::<T>(), value.to_string());
  }

  pub fn value(&mut self, type_name: impl Into<String>, formatted_value: impl Into<String>) {
    self
      .buffer
      .push(ComponentInfo::with_value("value", type_name, formatted_value));
  }

  pub fn field<T: DevtoolsInspectable + ?Sized>(&mut self, name: impl Into<String>, value: &T) {
    self.field_as(name, std::any::type_name::<T>(), value);
  }

  pub fn field_as<T: DevtoolsInspectable + ?Sized>(
    &mut self,
    name: impl Into<String>,
    type_name: impl Into<String>,
    value: &T,
  ) {
    let mut children = Vec::new();
    value.write_info(&mut children);
    self.field_children_or_value(name, type_name, children);
  }

  /// A field as the derive read it ([`probe`]): with what its type wrote of it, or with its type alone where
  /// the type does not write itself.
  #[doc(hidden)]
  pub fn field_probed(
    &mut self,
    name: impl Into<String>,
    type_name: impl Into<String>,
    written: Option<Vec<ComponentInfo>>,
  ) {
    match written {
      Some(children) => self.field_children_or_value(name, type_name, children),
      None => self.buffer.push(ComponentInfo::new(name, type_name)),
    }
  }

  pub fn field_debug<T: Debug + ?Sized>(&mut self, name: impl Into<String>, value: &T) {
    self.field_value(name, std::any::type_name::<T>(), format!("{value:?}"));
  }

  pub fn field_display<T: Display + ?Sized>(&mut self, name: impl Into<String>, value: &T) {
    self.field_value(name, std::any::type_name::<T>(), value.to_string());
  }

  pub fn field_value(
    &mut self,
    name: impl Into<String>,
    type_name: impl Into<String>,
    formatted_value: impl Into<String>,
  ) {
    self
      .buffer
      .push(ComponentInfo::with_value(name, type_name, formatted_value));
  }

  pub fn field_with(
    &mut self,
    name: impl Into<String>,
    type_name: impl Into<String>,
    inspect: impl FnOnce(&mut DevtoolsFormatter<'_>),
  ) {
    let mut children = Vec::new();
    {
      let mut formatter = DevtoolsFormatter::new(&mut children);
      inspect(&mut formatter);
    }
    self
      .buffer
      .push(ComponentInfo::with_children(name, type_name, children));
  }

  fn field_children_or_value(
    &mut self,
    name: impl Into<String>,
    type_name: impl Into<String>,
    children: Vec<ComponentInfo>,
  ) {
    if let Some(value) = collapsed_debug_value(&children) {
      self.buffer.push(ComponentInfo::with_value(name, type_name, value));
    } else {
      self
        .buffer
        .push(ComponentInfo::with_children(name, type_name, children));
    }
  }
}

pub struct DevtoolsStruct<'b, 'a> {
  formatter: &'b mut DevtoolsFormatter<'a>,
  _type_name: String,
}

impl DevtoolsStruct<'_, '_> {
  pub fn field<T: DevtoolsInspectable + ?Sized>(&mut self, name: impl Into<String>, value: &T) -> &mut Self {
    self.formatter.field(name, value);
    self
  }

  pub fn field_as<T: DevtoolsInspectable + ?Sized>(
    &mut self,
    name: impl Into<String>,
    type_name: impl Into<String>,
    value: &T,
  ) -> &mut Self {
    self.formatter.field_as(name, type_name, value);
    self
  }

  #[doc(hidden)]
  pub fn field_probed(
    &mut self,
    name: impl Into<String>,
    type_name: impl Into<String>,
    written: Option<Vec<ComponentInfo>>,
  ) -> &mut Self {
    self.formatter.field_probed(name, type_name, written);
    self
  }

  pub fn field_debug<T: Debug + ?Sized>(&mut self, name: impl Into<String>, value: &T) -> &mut Self {
    self.formatter.field_debug(name, value);
    self
  }

  pub fn field_display<T: Display + ?Sized>(&mut self, name: impl Into<String>, value: &T) -> &mut Self {
    self.formatter.field_display(name, value);
    self
  }

  pub fn field_value(
    &mut self,
    name: impl Into<String>,
    type_name: impl Into<String>,
    formatted_value: impl Into<String>,
  ) -> &mut Self {
    self.formatter.field_value(name, type_name, formatted_value);
    self
  }

  pub fn finish(&mut self) {}
}

pub struct DevtoolsTuple<'b, 'a> {
  formatter: &'b mut DevtoolsFormatter<'a>,
  _type_name: String,
  index: usize,
}

impl DevtoolsTuple<'_, '_> {
  pub fn field<T: DevtoolsInspectable + ?Sized>(&mut self, value: &T) -> &mut Self {
    let index = self.next_index();
    self.formatter.field(index, value);
    self
  }

  pub fn field_as<T: DevtoolsInspectable + ?Sized>(&mut self, type_name: impl Into<String>, value: &T) -> &mut Self {
    let index = self.next_index();
    self.formatter.field_as(index, type_name, value);
    self
  }

  #[doc(hidden)]
  pub fn field_probed(&mut self, type_name: impl Into<String>, written: Option<Vec<ComponentInfo>>) -> &mut Self {
    let index = self.next_index();
    self.formatter.field_probed(index, type_name, written);
    self
  }

  pub fn field_debug<T: Debug + ?Sized>(&mut self, value: &T) -> &mut Self {
    let index = self.next_index();
    self.formatter.field_debug(index, value);
    self
  }

  pub fn field_value(&mut self, type_name: impl Into<String>, formatted_value: impl Into<String>) -> &mut Self {
    let index = self.next_index();
    self.formatter.field_value(index, type_name, formatted_value);
    self
  }

  pub fn finish(&mut self) {}

  fn next_index(&mut self) -> String {
    let index = self.index;
    self.index += 1;
    index.to_string()
  }
}

pub struct DevtoolsList<'b, 'a> {
  formatter: &'b mut DevtoolsFormatter<'a>,
  index: usize,
}

impl DevtoolsList<'_, '_> {
  pub fn entry<T: DevtoolsInspectable + ?Sized>(&mut self, value: &T) -> &mut Self {
    let index = self.next_index();
    self.formatter.field(index, value);
    self
  }

  pub fn entry_debug<T: Debug + ?Sized>(&mut self, value: &T) -> &mut Self {
    let index = self.next_index();
    self.formatter.field_debug(index, value);
    self
  }

  pub fn finish(&mut self) {}

  fn next_index(&mut self) -> String {
    let index = self.index;
    self.index += 1;
    index.to_string()
  }
}

pub struct DevtoolsSet<'b, 'a> {
  formatter: &'b mut DevtoolsFormatter<'a>,
  index: usize,
}

impl DevtoolsSet<'_, '_> {
  pub fn entry<T: DevtoolsInspectable + ?Sized>(&mut self, value: &T) -> &mut Self {
    let index = self.next_index();
    self.formatter.field(index, value);
    self
  }

  pub fn entry_debug<T: Debug + ?Sized>(&mut self, value: &T) -> &mut Self {
    let index = self.next_index();
    self.formatter.field_debug(index, value);
    self
  }

  pub fn finish(&mut self) {}

  fn next_index(&mut self) -> String {
    let index = self.index;
    self.index += 1;
    index.to_string()
  }
}

pub struct DevtoolsMap<'b, 'a> {
  formatter: &'b mut DevtoolsFormatter<'a>,
}

impl DevtoolsMap<'_, '_> {
  pub fn entry<K: Debug + ?Sized, V: DevtoolsInspectable + ?Sized>(&mut self, key: &K, value: &V) -> &mut Self {
    self.formatter.field(format!("{key:?}"), value);
    self
  }

  pub fn entry_debug<K: Debug + ?Sized, V: Debug + ?Sized>(&mut self, key: &K, value: &V) -> &mut Self {
    self.formatter.field_debug(format!("{key:?}"), value);
    self
  }

  pub fn finish(&mut self) {}
}

fn collapsed_debug_value(fields: &[ComponentInfo]) -> Option<String> {
  if fields.len() != 1 {
    return None;
  }
  let info = &fields[0];
  if matches!(info.name(), "value" | "variant") {
    info.formatted_value().map(ToOwned::to_owned)
  } else {
    None
  }
}

pub trait DevtoolsInspectable {
  fn inspect(&self, _formatter: &mut DevtoolsFormatter<'_>) {}

  fn write_info(&self, buffer: &mut Vec<ComponentInfo>) {
    let mut formatter = DevtoolsFormatter::new(buffer);
    self.inspect(&mut formatter);
  }
}

impl DevtoolsInspectable for () {
  fn inspect(&self, _formatter: &mut DevtoolsFormatter<'_>) {}
}

impl DevtoolsInspectable for &'static str {
  fn inspect(&self, formatter: &mut DevtoolsFormatter<'_>) {
    formatter.value_debug(self);
  }
}

impl DevtoolsInspectable for std::sync::Arc<str> {
  fn inspect(&self, formatter: &mut DevtoolsFormatter<'_>) {
    formatter.value_debug(self);
  }
}

macro_rules! impl_scalar_devtools_inspectable {
  ($($ty:ty),* $(,)?) => {
    $(
      impl DevtoolsInspectable for $ty {
        fn inspect(&self, formatter: &mut DevtoolsFormatter<'_>) {
          formatter.value_debug(self);
        }
      }
    )*
  };
}

impl_scalar_devtools_inspectable!(
  bool,
  char,
  i8,
  i16,
  i32,
  i64,
  i128,
  isize,
  u8,
  u16,
  u32,
  u64,
  u128,
  usize,
  f32,
  f64,
  String,
  std::time::Duration,
  std::path::PathBuf,
);

/// How the derive reads a field whose type it cannot tell is inspectable: with what the type writes of it where
/// it writes itself, and with nothing where it does not. The first is found before the second for a `&Probe`,
/// so a struct with a field DevTools cannot read still derives, and that field shows its type alone.
#[doc(hidden)]
pub mod probe {
  use super::{ComponentInfo, DevtoolsInspectable};

  pub struct Probe<'a, T: ?Sized>(pub &'a T);

  pub trait Inspected {
    fn devtools_children(&self) -> Option<Vec<ComponentInfo>>;
  }

  impl<T: DevtoolsInspectable + ?Sized> Inspected for Probe<'_, T> {
    fn devtools_children(&self) -> Option<Vec<ComponentInfo>> {
      let mut children = Vec::new();
      self.0.write_info(&mut children);
      Some(children)
    }
  }

  pub trait Uninspected {
    fn devtools_children(&self) -> Option<Vec<ComponentInfo>>;
  }

  impl<T: ?Sized> Uninspected for &Probe<'_, T> {
    fn devtools_children(&self) -> Option<Vec<ComponentInfo>> {
      None
    }
  }
}

/// What reads a value of one type for DevTools, found by the type: `register_inspectable!` makes one, and the
/// derive calls it.
#[cfg(feature = "devtools")]
#[doc(hidden)]
pub struct RegisteredInspector {
  pub type_id: fn() -> std::any::TypeId,
  pub inspect: fn(&dyn std::any::Any, &mut Vec<ComponentInfo>),
}

#[cfg(feature = "devtools")]
inventory::collect!(RegisteredInspector);

/// What a value writes of itself for DevTools, by the inspector its type registered: `None` for a type that
/// registered none. This is how a value given with no bound on its type is read: what a context holds.
#[cfg(feature = "devtools")]
pub(crate) fn inspect_registered<T: 'static>(value: &T) -> Option<Vec<ComponentInfo>> {
  use std::{any::TypeId, collections::HashMap, sync::OnceLock};
  static INSPECTORS: OnceLock<HashMap<TypeId, fn(&dyn std::any::Any, &mut Vec<ComponentInfo>)>> = OnceLock::new();
  let inspectors = INSPECTORS.get_or_init(|| {
    inventory::iter::<RegisteredInspector>
      .into_iter()
      .map(|inspector| ((inspector.type_id)(), inspector.inspect))
      .collect()
  });
  let inspect = inspectors.get(&TypeId::of::<T>())?;
  let mut fields = Vec::new();
  inspect(value, &mut fields);
  Some(fields)
}

// The values an app provides as they are: a number, a text, a flag.
crate::register_inspectable!(
  bool,
  char,
  i8,
  i16,
  i32,
  i64,
  i128,
  isize,
  u8,
  u16,
  u32,
  u64,
  u128,
  usize,
  f32,
  f64,
  String,
  &'static str,
  std::sync::Arc<str>,
  std::time::Duration,
  std::path::PathBuf,
);

impl<T: DevtoolsInspectable> DevtoolsInspectable for Option<T> {
  fn inspect(&self, formatter: &mut DevtoolsFormatter<'_>) {
    match self {
      Some(value) => {
        formatter.field_with("Some", std::any::type_name::<Option<T>>(), |formatter| {
          value.write_info(formatter.buffer_mut());
        });
      }
      None => formatter.value(std::any::type_name::<Option<T>>(), "None"),
    }
  }
}

impl<T> DevtoolsInspectable for Vec<T> {
  fn inspect(&self, formatter: &mut DevtoolsFormatter<'_>) {
    formatter.value(std::any::type_name::<Vec<T>>(), format!("len={}", self.len()));
  }
}

impl<T> DevtoolsInspectable for [T] {
  fn inspect(&self, formatter: &mut DevtoolsFormatter<'_>) {
    formatter.value(std::any::type_name::<[T]>(), format!("len={}", self.len()));
  }
}

impl<T, const N: usize> DevtoolsInspectable for [T; N] {
  fn inspect(&self, formatter: &mut DevtoolsFormatter<'_>) {
    formatter.value(std::any::type_name::<[T; N]>(), format!("len={N}"));
  }
}

impl<T> DevtoolsInspectable for std::collections::VecDeque<T> {
  fn inspect(&self, formatter: &mut DevtoolsFormatter<'_>) {
    formatter.value(
      std::any::type_name::<std::collections::VecDeque<T>>(),
      format!("len={}", self.len()),
    );
  }
}

impl<T> DevtoolsInspectable for std::collections::BTreeSet<T> {
  fn inspect(&self, formatter: &mut DevtoolsFormatter<'_>) {
    formatter.value(
      std::any::type_name::<std::collections::BTreeSet<T>>(),
      format!("len={}", self.len()),
    );
  }
}

impl<K, V> DevtoolsInspectable for std::collections::BTreeMap<K, V> {
  fn inspect(&self, formatter: &mut DevtoolsFormatter<'_>) {
    formatter.value(
      std::any::type_name::<std::collections::BTreeMap<K, V>>(),
      format!("len={}", self.len()),
    );
  }
}

impl<T: DevtoolsInspectable, E: DevtoolsInspectable> DevtoolsInspectable for Result<T, E> {
  fn inspect(&self, formatter: &mut DevtoolsFormatter<'_>) {
    let type_name = std::any::type_name::<Result<T, E>>();
    match self {
      Ok(value) => formatter.field_with("Ok", type_name, |formatter| value.write_info(formatter.buffer_mut())),
      Err(error) => formatter.field_with("Err", type_name, |formatter| error.write_info(formatter.buffer_mut())),
    }
  }
}

/// A shared value is the value it shares.
impl<T: DevtoolsInspectable> DevtoolsInspectable for std::sync::Arc<T> {
  fn inspect(&self, formatter: &mut DevtoolsFormatter<'_>) {
    (**self).write_info(formatter.buffer_mut());
  }
}

impl<T: DevtoolsInspectable> DevtoolsInspectable for std::rc::Rc<T> {
  fn inspect(&self, formatter: &mut DevtoolsFormatter<'_>) {
    (**self).write_info(formatter.buffer_mut());
  }
}

impl DevtoolsInspectable for std::borrow::Cow<'_, str> {
  fn inspect(&self, formatter: &mut DevtoolsFormatter<'_>) {
    formatter.value_debug(&**self);
  }
}

impl DevtoolsInspectable for std::path::Path {
  fn inspect(&self, formatter: &mut DevtoolsFormatter<'_>) {
    formatter.value_debug(self);
  }
}

impl<T: Debug> DevtoolsInspectable for std::ops::Range<T> {
  fn inspect(&self, formatter: &mut DevtoolsFormatter<'_>) {
    formatter.value_debug(self);
  }
}

impl<T: Debug> DevtoolsInspectable for std::ops::RangeInclusive<T> {
  fn inspect(&self, formatter: &mut DevtoolsFormatter<'_>) {
    formatter.value_debug(self);
  }
}

impl<T, S> DevtoolsInspectable for std::collections::HashSet<T, S> {
  fn inspect(&self, formatter: &mut DevtoolsFormatter<'_>) {
    formatter.value(
      std::any::type_name::<std::collections::HashSet<T, S>>(),
      format!("len={}", self.len()),
    );
  }
}

impl<K, V, S> DevtoolsInspectable for std::collections::HashMap<K, V, S> {
  fn inspect(&self, formatter: &mut DevtoolsFormatter<'_>) {
    formatter.value(
      std::any::type_name::<std::collections::HashMap<K, V, S>>(),
      format!("len={}", self.len()),
    );
  }
}

impl DevtoolsInspectable for str {
  fn inspect(&self, formatter: &mut DevtoolsFormatter<'_>) {
    formatter.value_debug(self);
  }
}

impl<T: DevtoolsInspectable + ?Sized> DevtoolsInspectable for Box<T> {
  fn inspect(&self, formatter: &mut DevtoolsFormatter<'_>) {
    (**self).write_info(formatter.buffer_mut());
  }
}

impl<T: SignalValue> DevtoolsInspectable for Signal<T> {
  fn inspect(&self, formatter: &mut DevtoolsFormatter<'_>) {
    formatter.value(std::any::type_name::<Signal<T>>(), format!("#{}", self.id()));
  }
}

impl DevtoolsInspectable for crate::core::NodeId {
  fn inspect(&self, formatter: &mut DevtoolsFormatter<'_>) {
    formatter.value(
      std::any::type_name::<crate::core::NodeId>(),
      format!("#{}", self.value()),
    );
  }
}

macro_rules! impl_tuple_devtools_inspectable {
  ($($name:ident:$index:tt),+ $(,)?) => {
    impl<$($name),+> DevtoolsInspectable for ($($name,)+)
    where
      $($name: Send + PartialEq + 'static),+
    {
      fn inspect(&self, formatter: &mut DevtoolsFormatter<'_>) {
        $(
          formatter.buffer_mut().push(ComponentInfo::new(stringify!($index), std::any::type_name::<$name>()));
        )+
      }
    }
  };
}

impl_tuple_devtools_inspectable!(A:0);
impl_tuple_devtools_inspectable!(A:0, B:1);
impl_tuple_devtools_inspectable!(A:0, B:1, C:2);
impl_tuple_devtools_inspectable!(A:0, B:1, C:2, D:3);
impl_tuple_devtools_inspectable!(A:0, B:1, C:2, D:3, E:4);
impl_tuple_devtools_inspectable!(A:0, B:1, C:2, D:3, E:4, F:5);

pub trait Component: Send + Sync + 'static {
  #[cfg(feature = "devtools")]
  type Props: Send + PartialEq + DevtoolsInspectable + 'static;
  #[cfg(not(feature = "devtools"))]
  type Props: Send + PartialEq + 'static;
  fn create(ctx: &mut Ctx) -> Self;
  fn render(&self, ctx: &mut Ctx) -> impl Into<Element>;
  fn after_layout(&self) {}
  fn on_mounted(&self) {}
  fn on_unmounted(&self) {}
}

#[cfg(test)]
mod tests {
  use super::{ComponentInfo, DevtoolsFormatter, DevtoolsInspectable};

  struct FormatterDebugValue {
    count: u32,
    label: &'static str,
  }

  impl DevtoolsInspectable for FormatterDebugValue {
    fn inspect(&self, formatter: &mut DevtoolsFormatter<'_>) {
      formatter
        .debug_struct("FormatterDebugValue")
        .field("count", &self.count)
        .field_debug("label", &self.label)
        .finish();
    }
  }

  #[test]
  fn formatter_debug_struct_adds_named_fields() {
    let value = FormatterDebugValue {
      count: 7,
      label: "ready",
    };
    let mut fields = Vec::new();

    value.write_info(&mut fields);

    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].name(), "count");
    assert_eq!(fields[0].type_name(), "u32");
    assert_eq!(fields[0].formatted_value(), Some("7"));
    assert_eq!(fields[1].name(), "label");
    assert_eq!(fields[1].formatted_value(), Some("\"ready\""));
  }

  #[test]
  fn formatter_debug_map_allows_dynamic_keys() {
    let mut fields = Vec::new();
    let mut formatter = DevtoolsFormatter::new(&mut fields);

    formatter.debug_map().entry_debug("theme", &42_u32).finish();

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].name(), "\"theme\"");
    assert_eq!(fields[0].formatted_value(), Some("42"));
  }

  #[test]
  fn legacy_write_info_impls_still_work() {
    struct LegacyValue;

    impl DevtoolsInspectable for LegacyValue {
      fn write_info(&self, buffer: &mut Vec<ComponentInfo>) {
        buffer.push(ComponentInfo::with_value("legacy", "LegacyValue", "ok"));
      }
    }

    let mut fields = Vec::new();
    LegacyValue.write_info(&mut fields);

    assert_eq!(fields[0].name(), "legacy");
    assert_eq!(fields[0].formatted_value(), Some("ok"));
  }

  #[test]
  fn derive_uses_formatter_for_tuple_structs() {
    #[derive(crate::DevtoolsInspectable)]
    struct TupleValue(u32, &'static str);

    let mut fields = Vec::new();
    TupleValue(9, "items").write_info(&mut fields);

    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].name(), "0");
    assert_eq!(fields[0].formatted_value(), Some("9"));
    assert_eq!(fields[1].name(), "1");
    assert_eq!(fields[1].formatted_value(), Some("\"items\""));
  }

  /// What a value writes where it writes one thing: a number, a text, a length.
  fn written<T: DevtoolsInspectable + ?Sized>(value: &T) -> Option<String> {
    let mut fields = Vec::new();
    value.write_info(&mut fields);
    match fields.as_slice() {
      [only] => only.formatted_value().map(str::to_owned),
      _ => None,
    }
  }

  #[test]
  fn the_types_of_std_write_themselves() {
    use std::{
      borrow::Cow,
      collections::{BTreeMap, BTreeSet, VecDeque},
      path::{Path, PathBuf},
      rc::Rc,
      sync::Arc,
      time::Duration,
    };

    // What is shared or boxed is the value it holds.
    assert_eq!(written(&Arc::new(5_u32)).as_deref(), Some("5"));
    assert_eq!(written(&Rc::new("lurq".to_owned())).as_deref(), Some("\"lurq\""));
    assert_eq!(written(&Cow::Borrowed("text")).as_deref(), Some("\"text\""));
    // What holds many says how many, whatever they are.
    struct Opaque;
    assert_eq!(written(&[1_u8, 2, 3]).as_deref(), Some("len=3"));
    assert_eq!(written(&[1_u8, 2][..]).as_deref(), Some("len=2"));
    assert_eq!(written(&vec![Opaque, Opaque]).as_deref(), Some("len=2"));
    assert_eq!(written(&VecDeque::from([1, 2])).as_deref(), Some("len=2"));
    assert_eq!(written(&BTreeSet::from([1, 2, 3])).as_deref(), Some("len=3"));
    assert_eq!(written(&BTreeMap::from([("a", 1)])).as_deref(), Some("len=1"));
    assert_eq!(written(&Duration::from_millis(1500)).as_deref(), Some("1.5s"));
    assert_eq!(written(&PathBuf::from("a/b.txt")).as_deref(), Some("\"a/b.txt\""));
    assert_eq!(written(Path::new("c")).as_deref(), Some("\"c\""));
    assert_eq!(written(&(1..4)).as_deref(), Some("1..4"));
    assert_eq!(written(&(1..=4)).as_deref(), Some("1..=4"));

    // A result says which it is, with what that holds under it.
    let mut fields = Vec::new();
    Ok::<u8, String>(7).write_info(&mut fields);
    assert_eq!(fields[0].name(), "Ok");
    assert_eq!(fields[0].children()[0].formatted_value(), Some("7"));
    let mut fields = Vec::new();
    Err::<u8, String>("no".to_owned()).write_info(&mut fields);
    assert_eq!(fields[0].name(), "Err");
    assert_eq!(fields[0].children()[0].formatted_value(), Some("\"no\""));
  }

  #[test]
  fn a_derive_takes_a_field_devtools_cannot_read_and_shows_its_type() {
    /// It does not write itself for DevTools.
    struct Opaque;

    #[derive(crate::DevtoolsInspectable)]
    struct Inner {
      enabled: bool,
    }

    #[derive(crate::DevtoolsInspectable)]
    struct Holder {
      count: u32,
      opaque: Opaque,
      inner: Inner,
      callback: fn() -> u8,
      many: Vec<Opaque>,
    }

    let mut fields = Vec::new();
    Holder {
      count: 3,
      opaque: Opaque,
      inner: Inner { enabled: true },
      callback: || 1,
      many: vec![Opaque],
    }
    .write_info(&mut fields);

    let names = fields.iter().map(ComponentInfo::name).collect::<Vec<_>>();
    assert_eq!(names, ["count", "opaque", "inner", "callback", "many"]);
    assert_eq!(fields[0].formatted_value(), Some("3"));
    // The field it cannot read is there with its type, and nothing of its value.
    assert!(fields[1].type_name().ends_with("Opaque"));
    assert_eq!(fields[1].formatted_value(), None);
    assert!(fields[1].children().is_empty());
    // One it can read is read as before.
    assert_eq!(fields[2].children()[0].name(), "enabled");
    assert_eq!(fields[2].children()[0].formatted_value(), Some("true"));
    assert_eq!(fields[3].formatted_value(), None);
    assert_eq!(fields[4].formatted_value(), Some("len=1"));

    // A tuple struct's too.
    #[derive(crate::DevtoolsInspectable)]
    struct Pair(Opaque, u8);
    let mut fields = Vec::new();
    Pair(Opaque, 2).write_info(&mut fields);
    assert_eq!(fields[0].formatted_value(), None);
    assert_eq!(fields[1].formatted_value(), Some("2"));
  }

  #[test]
  fn a_field_of_a_type_parameter_is_read_where_the_parameter_is_bound() {
    #[derive(crate::DevtoolsInspectable)]
    struct Bound<T: DevtoolsInspectable>(T);

    #[derive(crate::DevtoolsInspectable)]
    struct Free<T>(T);

    let mut fields = Vec::new();
    Bound(4_u8).write_info(&mut fields);
    assert_eq!(fields[0].formatted_value(), Some("4"));

    // With no bound, what the parameter is is not known where the struct is written.
    let mut fields = Vec::new();
    Free(4_u8).write_info(&mut fields);
    assert_eq!(fields[0].formatted_value(), None);
    assert_eq!(fields[0].type_name(), "u8");
  }

  #[cfg(feature = "devtools")]
  #[test]
  fn a_value_is_read_by_the_inspector_its_type_registered() {
    use super::inspect_registered;

    #[derive(crate::DevtoolsInspectable)]
    struct Derived {
      count: u32,
    }

    struct ByHand(u8);

    impl DevtoolsInspectable for ByHand {
      fn inspect(&self, formatter: &mut DevtoolsFormatter<'_>) {
        formatter.value("ByHand", self.0.to_string());
      }
    }
    crate::register_inspectable!(ByHand);

    struct Unregistered;

    #[derive(crate::DevtoolsInspectable)]
    struct Generic<T>(T);

    let value = |fields: Option<Vec<ComponentInfo>>| {
      fields.and_then(|fields| {
        fields
          .first()
          .and_then(|field| field.formatted_value().map(str::to_owned))
      })
    };
    // A derive registers the type it derives for, and a type written by hand is registered by hand.
    assert_eq!(value(inspect_registered(&Derived { count: 3 })).as_deref(), Some("3"));
    assert_eq!(value(inspect_registered(&ByHand(9))).as_deref(), Some("9"));
    // The values an app provides as they are.
    assert_eq!(value(inspect_registered(&7_i32)).as_deref(), Some("7"));
    assert_eq!(
      value(inspect_registered(&"text".to_owned())).as_deref(),
      Some("\"text\"")
    );
    assert_eq!(value(inspect_registered(&"static")).as_deref(), Some("\"static\""));
    // A type that registered none has none: so has a type with parameters, which is a type for each.
    assert!(inspect_registered(&Unregistered).is_none());
    assert!(inspect_registered(&Generic(1_u8)).is_none());
  }
}
