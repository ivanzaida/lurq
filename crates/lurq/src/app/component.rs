use crate::{app::ctx::Ctx, node::Element};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComponentInfo {
  name: &'static str,
  value: &'static str,
}

impl ComponentInfo {
  pub const fn new(name: &'static str, value: &'static str) -> Self {
    Self { name, value }
  }

  pub fn name(&self) -> &'static str {
    self.name
  }

  pub fn value(&self) -> &'static str {
    self.value
  }
}

pub trait DevtoolsInspectable {
  fn write_info(&self, buffer: &mut Vec<ComponentInfo>);
}

impl DevtoolsInspectable for () {
  fn write_info(&self, _buffer: &mut Vec<ComponentInfo>) {}
}

macro_rules! impl_scalar_devtools_inspectable {
  ($($ty:ty),* $(,)?) => {
    $(
      impl DevtoolsInspectable for $ty {
        fn write_info(&self, buffer: &mut Vec<ComponentInfo>) {
          buffer.push(ComponentInfo::new("value", std::any::type_name::<$ty>()));
        }
      }
    )*
  };
}

impl_scalar_devtools_inspectable!(
  bool, i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, f32, f64, String
);

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
