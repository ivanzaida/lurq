use std::path::Path;
#[cfg(feature = "resources")]
use std::path::PathBuf;

#[cfg(feature = "i18n")]
use crate::app::i18n::I18n;
use crate::app::{glyph_engine::GlyphEngine, theme::Theme};

pub struct App {
  pub(crate) glyph_engine: GlyphEngine,
  pub(crate) theme: Theme,
  #[cfg(feature = "i18n")]
  pub(crate) i18n: I18n,
  pub(crate) scale_override: Option<f32>,
  #[cfg(feature = "tokio")]
  pub(crate) tokio_handle: Option<tokio::runtime::Handle>,
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
      #[cfg(feature = "i18n")]
      i18n: I18n::new(),
      scale_override: None,
      #[cfg(feature = "tokio")]
      tokio_handle: None,
      #[cfg(feature = "resources")]
      resource_loader: crate::resources::ResourceLoader::new(),
      #[cfg(all(feature = "image", feature = "resources"))]
      image_resource_cache: std::collections::HashMap::new(),
      #[cfg(all(feature = "svg", feature = "resources"))]
      svg_resource_cache: std::collections::HashMap::new(),
    }
  }

  pub fn set_scale_override(&mut self, scale: Option<f32>) {
    self.scale_override = scale;
    self.glyph_engine.clear_cache();
  }

  #[cfg(feature = "tokio")]
  pub fn with_tokio_handle(mut self, handle: tokio::runtime::Handle) -> Self {
    self.tokio_handle = Some(handle);
    self
  }

  #[cfg(feature = "tokio")]
  pub fn set_tokio_handle(&mut self, handle: tokio::runtime::Handle) {
    self.tokio_handle = Some(handle);
  }

  #[cfg(feature = "tokio")]
  pub fn clear_tokio_handle(&mut self) {
    self.tokio_handle = None;
  }

  #[cfg(feature = "tokio")]
  pub(crate) fn tokio_handle(&self) -> Option<tokio::runtime::Handle> {
    self.tokio_handle.clone()
  }

  pub fn theme(&self) -> &Theme {
    &self.theme
  }

  #[cfg(feature = "i18n")]
  pub fn i18n(&self) -> &I18n {
    &self.i18n
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
