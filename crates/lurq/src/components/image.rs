use crate::{images::ImageData, impl_into_node, node::BackgroundSize};

impl_into_node!(Image);

impl Image {
  pub fn new(data: ImageData) -> Self {
    Self::from_node(crate::node::Node::image(data))
  }

  #[cfg(feature = "image")]
  pub fn from_resource(path: &str) -> Self {
    Self::from_node(crate::node::Node::resource_image(path))
  }

  /// An image decoded at runtime from encoded bytes (PNG, JPEG, the first
  /// frame of a GIF or WebP, BMP, TIFF), off the UI thread: the node shows
  /// its [`placeholder`](Self::placeholder) (and its own background) until
  /// the pixels are decoded, then the image; a broken file shows the
  /// [`error_view`](Self::error_view). The bytes are hashed for the cache
  /// key unless [`cache_key`](Self::cache_key) gives one, and the decoded
  /// pixels are kept in the process-wide
  /// [`decoded_images`](crate::images::decoded_images) cache, so building
  /// the same image again (a chat laid out again, a `VirtualizedList` row
  /// scrolled back in) decodes nothing.
  ///
  /// ```
  /// use lurq::{components::Image, images::ImageKey, node::BackgroundSize};
  ///
  /// let png: Vec<u8> = std::fs::read("screenshot.png").unwrap_or_default();
  /// let chip = Image::from_bytes(png.clone())
  ///   .cache_key(ImageKey::of_bytes(&png))
  ///   .downscale_to(256, 256)
  ///   .fit(BackgroundSize::Cover)
  ///   .size(96.0, 96.0);
  /// ```
  #[cfg(feature = "image")]
  pub fn from_bytes(bytes: impl Into<std::sync::Arc<[u8]>>) -> Self {
    let bytes: std::sync::Arc<[u8]> = bytes.into();
    let key = crate::images::ImageKey::of_bytes(&bytes);
    Self::from_source(crate::images::ImageSource::Bytes(bytes), key)
  }

  /// An image decoded at runtime from a file, off the UI thread; see
  /// [`from_bytes`](Self::from_bytes). The key is the path with the file's
  /// size and modification time ([`ImageKey::of_path`](crate::images::ImageKey::of_path)).
  #[cfg(feature = "image")]
  pub fn from_file(path: impl Into<std::path::PathBuf>) -> Self {
    let path = path.into();
    let key = crate::images::ImageKey::of_path(&path);
    Self::from_source(crate::images::ImageSource::File(path), key)
  }

  /// An image decoded at runtime from `source`, cached under `key`; see
  /// [`from_bytes`](Self::from_bytes).
  #[cfg(feature = "image")]
  pub fn from_source(source: crate::images::ImageSource, key: crate::images::ImageKey) -> Self {
    Self::from_node(crate::node::Node::loading_image(crate::node::node_kind::LoadingImage {
      key,
      cap: None,
      source,
      fit: BackgroundSize::Stretch,
      error_view: None,
      failed: false,
    }))
  }

  /// The cache key of a runtime image: the same key means the same pixels.
  /// `from_bytes` has already hashed the bytes by the time this runs; an app
  /// that stores a hash of its own and wants neither that hashing nor a
  /// copy of the bytes builds the node with
  /// `Image::from_source(ImageSource::Bytes(arc), ImageKey::new(hash))`.
  #[cfg(feature = "image")]
  pub fn cache_key(mut self, key: crate::images::ImageKey) -> Self {
    self.update_node(|node| node.update_loading_image(|request| request.key = key));
    self
  }

  /// Downscales a runtime image to fit within `width` × `height` pixels,
  /// keeping its aspect ratio (never upscaling), so a thumbnail of a 4K
  /// screenshot does not keep its 30 MB of RGBA. Cached separately from the
  /// full-size decode of the same key.
  #[cfg(feature = "image")]
  pub fn downscale_to(mut self, width: u32, height: u32) -> Self {
    let cap = crate::images::ImageSizeCap::new(width, height);
    self.update_node(|node| node.update_loading_image(|request| request.cap = Some(cap)));
    self
  }

  /// What the node shows while a runtime image decodes: a spinner, a grey
  /// box. The node's own background shows too, under the placeholder.
  #[cfg(feature = "image")]
  pub fn placeholder(mut self, placeholder: impl Into<crate::node::Element>) -> Self {
    let placeholder = placeholder.into().into_node();
    self.update_node(|node| {
      if matches!(node.node_kind(), crate::node::node_kind::NodeKind::LoadingImage { .. }) {
        crate::node::NodeUpdate::child(node, placeholder);
      }
    });
    self
  }

  /// What the node shows once a runtime image failed to decode (a broken
  /// file, an unsupported format): built from the error, in place of the
  /// placeholder. Without it the node shows only its own background.
  #[cfg(feature = "image")]
  pub fn error_view<E: Into<crate::node::Element>>(
    mut self,
    view: impl Fn(&crate::images::ImageDecodeError) -> E + Send + Sync + 'static,
  ) -> Self {
    let view: crate::node::node_kind::ErrorViewCallback = std::sync::Arc::new(move |error| view(error).into());
    self.update_node(|node| node.update_loading_image(|request| request.error_view = Some(view)));
    self
  }

  /// How the pixels fill the node's box: `Stretch` (the default) fills it as
  /// the box is sized, `Contain` fits the whole image in it, centered, and
  /// `Cover` fills it and crops the overflow, centered.
  pub fn fit(mut self, fit: BackgroundSize) -> Self {
    self.update_node(|node| node.set_image_fit(fit));
    self
  }

  pub fn stretch(self) -> Self {
    self.fit(BackgroundSize::Stretch)
  }

  pub fn cover(self) -> Self {
    self.fit(BackgroundSize::Cover)
  }

  pub fn contain(self) -> Self {
    self.fit(BackgroundSize::Contain)
  }
}
