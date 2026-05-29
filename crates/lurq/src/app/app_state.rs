use std::path::Path;
#[cfg(feature = "resources")]
use std::path::PathBuf;

use crate::app::{glyph_engine::GlyphEngine, theme::Theme};

pub struct App {
  pub(crate) glyph_engine: GlyphEngine,
  pub(crate) theme: Theme,
  pub(crate) profiling_enabled: bool,
  pub(crate) scale_override: Option<f32>,
  #[cfg(feature = "resources")]
  pub(crate) resource_loader: crate::resources::ResourceLoader,
  #[cfg(all(feature = "image", feature = "resources"))]
  pub(crate) image_resource_cache: std::collections::HashMap<std::sync::Arc<str>, crate::images::ImageData>,
  #[cfg(all(feature = "svg", feature = "resources"))]
  pub(crate) svg_resource_cache: std::collections::HashMap<std::sync::Arc<str>, crate::svg::SvgData>,
}

impl Default for App {
  fn default() -> Self {
    Self::new()
  }
}

impl App {
  pub fn new() -> Self {
    Self {
      glyph_engine: GlyphEngine::new(),
      theme: Theme::new(),
      profiling_enabled: false,
      scale_override: None,
      #[cfg(feature = "resources")]
      resource_loader: crate::resources::ResourceLoader::new(),
      #[cfg(all(feature = "image", feature = "resources"))]
      image_resource_cache: std::collections::HashMap::new(),
      #[cfg(all(feature = "svg", feature = "resources"))]
      svg_resource_cache: std::collections::HashMap::new(),
    }
  }

  pub fn profiling_enabled(&self) -> bool {
    self.profiling_enabled
  }

  pub fn set_profiling_enabled(&mut self, enabled: bool) {
    self.profiling_enabled = enabled;
  }

  pub fn set_scale_override(&mut self, scale: Option<f32>) {
    self.scale_override = scale;
    self.glyph_engine.clear_cache();
  }

  pub fn theme(&self) -> &Theme {
    &self.theme
  }

  pub fn load_font(&mut self, data: Vec<u8>) {
    self.glyph_engine.load_font(data);
  }

  pub fn load_font_file(&mut self, path: &Path) {
    self.glyph_engine.load_font_file(path);
  }

  pub fn load_fonts_dir(&mut self, path: &Path) {
    self.glyph_engine.load_fonts_dir(path);
  }

  pub fn register_font(&mut self, name: &str, family: &str) {
    self.glyph_engine.register_font(name, family);
  }

  #[cfg(feature = "resources")]
  pub fn set_resource_root(&mut self, root: PathBuf) {
    self.resource_loader.set_root(root);
  }
}
