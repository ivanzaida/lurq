#[cfg(feature = "image")]
pub(crate) mod rasterize;
mod render;
mod svg_data;
#[allow(dead_code)]
pub(crate) mod tessellate;

pub(crate) use render::SvgCmd;
pub use svg_data::SvgData;
