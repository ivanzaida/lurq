use crate::layout::render_list::RenderList;

/// Cloneable handles to the WGPU objects owned by Lurq's renderer.
///
/// Frame extensions must use these handles instead of creating a second
/// instance, adapter, device, or queue. This keeps embedded renderers on the
/// same GPU timeline as Lurq and allows their texture views to be sampled
/// directly by the UI renderer.
#[derive(Clone)]
pub struct SharedWgpuContext {
  pub instance: wgpu::Instance,
  pub adapter: wgpu::Adapter,
  pub device: wgpu::Device,
  pub queue: wgpu::Queue,
  pub surface_format: wgpu::TextureFormat,
  /// The device's number in the order of devices and of the views given to slots: it tells a view of this
  /// device from one of a device that was lost.
  pub(crate) number: u64,
}

/// A rectangle of the window's surface, in physical pixels from its top left corner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WgpuViewportRect {
  pub x: f32,
  pub y: f32,
  pub width: f32,
  pub height: f32,
}

/// Read-only information for the Lurq frame an extension is preparing.
#[derive(Clone, Copy)]
pub struct WgpuFrameInfo<'a> {
  #[cfg_attr(not(feature = "raster"), allow(dead_code))]
  render_list: &'a RenderList,
  pub surface_width: u32,
  pub surface_height: u32,
  /// The window's physical pixels to one logical pixel: 1.5 at a scale of 150%. A rectangle of
  /// [`viewport`](Self::viewport) divided by it is the element's box in the layout's logical pixels, and a line
  /// one logical pixel wide is this many pixels of the texture.
  pub scale_factor: f32,
}

impl<'a> WgpuFrameInfo<'a> {
  pub(crate) fn new(render_list: &'a RenderList, surface_width: u32, surface_height: u32, scale_factor: f32) -> Self {
    Self {
      render_list,
      surface_width,
      surface_height,
      scale_factor,
    }
  }

  /// Where the viewport that shows the slot with `image_id` is in this frame: its whole box, in physical
  /// pixels, before its transform and whatever clips it. `None` when the frame does not show it.
  ///
  /// Pass [`WgpuExternalImageSlot::image_id`](crate::images::WgpuExternalImageSlot::image_id)
  /// to locate the corresponding `GpuViewport` in the current render list.
  #[cfg(feature = "raster")]
  pub fn viewport(&self, image_id: u64) -> Option<WgpuViewportRect> {
    self.image(image_id).map(|image| WgpuViewportRect {
      x: image.x,
      y: image.y,
      width: image.width,
      height: image.height,
    })
  }

  /// The part of the viewport with `image_id` that is shown in this frame, in physical pixels: its box, moved by
  /// its transform, and cut by the clips of its parents (a scroll it is half out of) and by the window's edges.
  /// `None` when none of it is shown, or the frame does not show it. A renderer that is expensive per pixel draws
  /// this part alone; the texture is still mapped to the whole box, [`viewport`](Self::viewport).
  #[cfg(feature = "raster")]
  pub fn visible_viewport(&self, image_id: u64) -> Option<WgpuViewportRect> {
    let image = self.image(image_id)?;
    // The box's corners, turned about its origin, and the rectangle around them.
    let [a, b, c, d] = image.transform;
    let [origin_x, origin_y] = image.transform_origin;
    let corners = [
      (0.0, 0.0),
      (image.width, 0.0),
      (0.0, image.height),
      (image.width, image.height),
    ]
    .map(|(x, y)| {
      let (x, y) = (x - origin_x, y - origin_y);
      (image.x + origin_x + a * x + c * y, image.y + origin_y + b * x + d * y)
    });
    let mut left = corners.iter().map(|corner| corner.0).fold(f32::INFINITY, f32::min);
    let mut top = corners.iter().map(|corner| corner.1).fold(f32::INFINITY, f32::min);
    let mut right = corners.iter().map(|corner| corner.0).fold(f32::NEG_INFINITY, f32::max);
    let mut bottom = corners.iter().map(|corner| corner.1).fold(f32::NEG_INFINITY, f32::max);
    if image.clip.active {
      left = left.max(image.clip.x);
      top = top.max(image.clip.y);
      right = right.min(image.clip.x + image.clip.width);
      bottom = bottom.min(image.clip.y + image.clip.height);
    }
    left = left.max(0.0);
    top = top.max(0.0);
    right = right.min(self.surface_width as f32);
    bottom = bottom.min(self.surface_height as f32);
    (right > left && bottom > top).then(|| WgpuViewportRect {
      x: left,
      y: top,
      width: right - left,
      height: bottom - top,
    })
  }

  #[cfg(feature = "raster")]
  fn image(&self, image_id: u64) -> Option<&crate::images::ImageCmd> {
    self.render_list.images.iter().find(|image| image.image_id == image_id)
  }
}

/// An error of the app's own wgpu calls, which lurq logged and gave to the extensions
/// ([`WgpuFrameExtension::error`]) instead of ending the program.
#[derive(Clone, Debug, PartialEq)]
pub struct WgpuAppError {
  /// What kind of error: a call wgpu found invalid, memory the card did not have, or a fault of the driver.
  pub kind: wgpu::ErrorFilter,
  /// What wgpu said of it, for a log: not for a person.
  pub message: String,
}

impl From<wgpu::Error> for WgpuAppError {
  fn from(error: wgpu::Error) -> Self {
    let kind = match &error {
      wgpu::Error::OutOfMemory { .. } => wgpu::ErrorFilter::OutOfMemory,
      wgpu::Error::Validation { .. } => wgpu::ErrorFilter::Validation,
      wgpu::Error::Internal { .. } => wgpu::ErrorFilter::Internal,
    };
    Self {
      kind,
      message: error.to_string(),
    }
  }
}

impl std::fmt::Display for WgpuAppError {
  fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(formatter, "{:?} error: {}", self.kind, self.message)
  }
}

impl std::error::Error for WgpuAppError {}

/// An in-process renderer that participates in Lurq's WGPU frame lifecycle.
///
/// `initialize` runs once after Lurq creates its device. `prepare` runs before
/// Lurq encodes the UI pass, so an embedded renderer can update external
/// textures that the UI samples in the same frame.
///
/// The engines of an app's windows share one device ([`WgpuRenderEngine::with_own_device`](
/// super::WgpuRenderEngine::with_own_device) keeps one apart), and each window's engine has extensions of its
/// own, made by the render engine factory: each is initialized and prepares the frames of its own window. lurq's
/// DevTools window runs none of them.
///
/// The device may be lost: its card was reset (a driver that hung or was updated) or taken away. Lurq finds
/// it, tells the extension ([`device_lost`](Self::device_lost)), and lets go of the engine and its
/// extensions; the engine that takes its place is made as the first one was, with extensions of its own,
/// which are initialized with another device. Lurq watches the device with wgpu's one callback for a loss
/// and its handler of uncaptured errors (`Device::set_device_lost_callback`, `Device::on_uncaptured_error`):
/// an extension sets neither.
///
/// A wgpu error of the extension's own calls ends nothing: lurq logs it and gives it to
/// [`error`](Self::error). An error of lurq's own work ends the program, as a fault of lurq's.
pub trait WgpuFrameExtension {
  fn initialize(&mut self, _gpu: &SharedWgpuContext) {}

  fn prepare(&mut self, _gpu: &SharedWgpuContext, _frame: WgpuFrameInfo<'_>) {}

  /// Keep the host event loop producing frames while the extension is active.
  fn wants_redraw(&self) -> bool {
    false
  }

  /// A wgpu error of the app's own calls: one this extension's `initialize`, `prepare`, `device_lost` or
  /// `shutdown` made, told right after that call, or one the app made on the device outside an error scope of
  /// its own and outside these calls (on a thread of its own), told to the extensions of the device's engines
  /// before the next frame they prepare. lurq has logged it already. What failed did nothing: an invalid
  /// pipeline or texture is one that draws nothing, and the app may make it again.
  fn error(&mut self, _error: &WgpuAppError) {}

  /// The device the extension was initialized with is lost. Everything made with it is gone, textures and
  /// the views of them given to a [`WgpuExternalImageSlot`](crate::images::WgpuExternalImageSlot) among
  /// them, and nothing more can be asked of it: what is asked fails, and is not waited for. Called once,
  /// on the thread that draws, as soon as Lurq finds the loss, which is before its next frame; `prepare` is
  /// not called again, and `shutdown` follows. An extension lets go of what it made, and tells whatever
  /// else of the app uses the device: while anything made with the lost device is held, the device is, and
  /// the card may give no whole device in its place. Lurq lets go of the views of the lost device that are
  /// in the slots it drew. The app's windows hear of it too, once their engines are replaced
  /// ([`AppEvent::GraphicsDeviceLost`](crate::app::window::AppEvent::GraphicsDeviceLost)).
  fn device_lost(&mut self) {}

  /// The engine is let go of. After [`device_lost`](Self::device_lost) there is nothing to ask of the device.
  fn shutdown(&mut self) {}
}

pub(crate) struct WgpuFrameExtensionEntry {
  pub extension: Box<dyn WgpuFrameExtension>,
  pub initialized: bool,
  /// Whether it was told that its device is lost.
  pub told_lost: bool,
}

impl WgpuFrameExtensionEntry {
  pub fn new(extension: Box<dyn WgpuFrameExtension>) -> Self {
    Self {
      extension,
      initialized: false,
      told_lost: false,
    }
  }
}
