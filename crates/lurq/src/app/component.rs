use crate::{
  app::ctx::Ctx,
  core::{Signal, SignalValue},
  node::Element,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComponentInfo {
  name: &'static str,
  type_name: &'static str,
  formatted_value: Option<String>,
  children: Vec<ComponentInfo>,
}

impl ComponentInfo {
  pub const fn new(name: &'static str, type_name: &'static str) -> Self {
    Self {
      name,
      type_name,
      formatted_value: None,
      children: Vec::new(),
    }
  }

  pub fn with_value(name: &'static str, type_name: &'static str, formatted_value: impl Into<String>) -> Self {
    Self {
      name,
      type_name,
      formatted_value: Some(formatted_value.into()),
      children: Vec::new(),
    }
  }

  pub fn with_children(name: &'static str, type_name: &'static str, children: Vec<ComponentInfo>) -> Self {
    Self {
      name,
      type_name,
      formatted_value: None,
      children,
    }
  }

  pub fn name(&self) -> &'static str {
    self.name
  }

  pub fn value(&self) -> &'static str {
    self.type_name
  }

  pub fn type_name(&self) -> &'static str {
    self.type_name
  }

  pub fn formatted_value(&self) -> Option<&str> {
    self.formatted_value.as_deref()
  }

  pub fn children(&self) -> &[ComponentInfo] {
    &self.children
  }

  #[cfg(feature = "devtools")]
  pub(crate) fn estimated_memory_bytes(&self) -> usize {
    self.formatted_value.as_ref().map(|value| value.capacity()).unwrap_or(0)
      + self.children.capacity() * std::mem::size_of::<ComponentInfo>()
      + self
        .children
        .iter()
        .map(ComponentInfo::estimated_memory_bytes)
        .sum::<usize>()
  }
}

pub trait DevtoolsInspectable {
  fn write_info(&self, buffer: &mut Vec<ComponentInfo>);
}

impl DevtoolsInspectable for () {
  fn write_info(&self, _buffer: &mut Vec<ComponentInfo>) {}
}

impl DevtoolsInspectable for &'static str {
  fn write_info(&self, buffer: &mut Vec<ComponentInfo>) {
    buffer.push(ComponentInfo::with_value(
      "value",
      std::any::type_name::<&'static str>(),
      format!("{self:?}"),
    ));
  }
}

macro_rules! impl_scalar_devtools_inspectable {
  ($($ty:ty),* $(,)?) => {
    $(
      impl DevtoolsInspectable for $ty {
        fn write_info(&self, buffer: &mut Vec<ComponentInfo>) {
          buffer.push(ComponentInfo::with_value("value", std::any::type_name::<$ty>(), format!("{self:?}")));
        }
      }
    )*
  };
}

impl_scalar_devtools_inspectable!(
  bool, char, i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64, String
);

impl<T: DevtoolsInspectable> DevtoolsInspectable for Option<T> {
  fn write_info(&self, buffer: &mut Vec<ComponentInfo>) {
    match self {
      Some(value) => {
        let mut children = Vec::new();
        value.write_info(&mut children);
        buffer.push(ComponentInfo::with_children(
          "Some",
          std::any::type_name::<Option<T>>(),
          children,
        ));
      }
      None => buffer.push(ComponentInfo::with_value(
        "value",
        std::any::type_name::<Option<T>>(),
        "None",
      )),
    }
  }
}

impl<T: DevtoolsInspectable> DevtoolsInspectable for Vec<T> {
  fn write_info(&self, buffer: &mut Vec<ComponentInfo>) {
    buffer.push(ComponentInfo::with_value(
      "value",
      std::any::type_name::<Vec<T>>(),
      format!("len={}", self.len()),
    ));
  }
}

impl<T: DevtoolsInspectable + ?Sized> DevtoolsInspectable for Box<T> {
  fn write_info(&self, buffer: &mut Vec<ComponentInfo>) {
    (**self).write_info(buffer);
  }
}

impl<T: SignalValue> DevtoolsInspectable for Signal<T> {
  fn write_info(&self, buffer: &mut Vec<ComponentInfo>) {
    buffer.push(ComponentInfo::with_value(
      "value",
      std::any::type_name::<Signal<T>>(),
      format!("#{}", self.id()),
    ));
  }
}

impl DevtoolsInspectable for crate::core::NodeId {
  fn write_info(&self, buffer: &mut Vec<ComponentInfo>) {
    buffer.push(ComponentInfo::with_value(
      "value",
      std::any::type_name::<crate::core::NodeId>(),
      format!("#{}", self.value()),
    ));
  }
}

macro_rules! impl_tuple_devtools_inspectable {
  ($($name:ident:$index:tt),+ $(,)?) => {
    impl<$($name),+> DevtoolsInspectable for ($($name,)+)
    where
      $($name: Send + PartialEq + 'static),+
    {
      fn write_info(&self, buffer: &mut Vec<ComponentInfo>) {
        $(
          buffer.push(ComponentInfo::new(stringify!($index), std::any::type_name::<$name>()));
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
  fn on_mounted(&self) {}
  fn on_unmounted(&self) {}
}
