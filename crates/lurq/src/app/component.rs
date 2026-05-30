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

pub trait ComponentProp {
  fn write_info(&self, buffer: &mut Vec<ComponentInfo>);
}

impl ComponentProp for () {
  fn write_info(&self, _buffer: &mut Vec<ComponentInfo>) {}
}

macro_rules! impl_scalar_component_prop {
  ($($ty:ty),* $(,)?) => {
    $(
      impl ComponentProp for $ty {
        fn write_info(&self, buffer: &mut Vec<ComponentInfo>) {
          buffer.push(ComponentInfo::new("value", std::any::type_name::<$ty>()));
        }
      }
    )*
  };
}

impl_scalar_component_prop!(
  bool, i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, f32, f64, String
);

macro_rules! impl_tuple_component_prop {
  ($($name:ident:$index:tt),+ $(,)?) => {
    impl<$($name),+> ComponentProp for ($($name,)+)
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

impl_tuple_component_prop!(A:0);
impl_tuple_component_prop!(A:0, B:1);
impl_tuple_component_prop!(A:0, B:1, C:2);
impl_tuple_component_prop!(A:0, B:1, C:2, D:3);
impl_tuple_component_prop!(A:0, B:1, C:2, D:3, E:4);
impl_tuple_component_prop!(A:0, B:1, C:2, D:3, E:4, F:5);

pub trait Component: Send + Sync + 'static {
  type Props: Send + PartialEq + ComponentProp + 'static;
  fn create(ctx: &mut Ctx) -> Self;
  fn render(&self, ctx: &mut Ctx) -> impl Into<Element>;
  fn on_mounted(&self) {}
  fn on_unmounted(&self) {}
}
