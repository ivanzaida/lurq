use std::sync::Arc;

use parking_lot::RwLock;

use super::{ImageData, ImagePixelFormat, NativeImageBackend, NativeImageData};

/// The devices that draw and the views that are given, counted in the order they come: a view given before
/// the device that draws now was made is of another device, the one that was lost.
static ORDER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// The next number in the order of devices and views.
pub(crate) fn next_in_order() -> u64 {
  ORDER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

thread_local! {
  /// The device whose engine is running an extension on this thread now, by its number.
  static RUNS_FOR: std::cell::Cell<Option<u64>> = const { std::cell::Cell::new(None) };
}

/// Runs `run` as an extension of the engine that draws with the device numbered `device`: a view given to a
/// slot within it is of that device.
pub(crate) fn for_device<R>(device: u64, run: impl FnOnce() -> R) -> R {
  struct Back(Option<u64>);
  impl Drop for Back {
    fn drop(&mut self) {
      RUNS_FOR.set(self.0);
    }
  }
  let _back = Back(RUNS_FOR.replace(Some(device)));
  run()
}

/// What a [`GpuViewport`](crate::components::GpuViewport) shows: a texture view the app renders into with its own
/// wgpu code on lurq's device, which lurq samples where the viewport is laid out, in the same frame and without a
/// copy.
///
/// The app makes one with [`new`](Self::new) and clones it into the UI (`GpuViewport::new(slot)`) and into its
/// renderer, a [`WgpuFrameExtension`](crate::app::wgpu_render::WgpuFrameExtension). The clones are the same slot:
/// a view given through one is shown by every viewport made with another. The renderer finds where the viewport is
/// in a frame by the slot's [`image_id`](Self::image_id), gives the slot a view of a texture once for each texture
/// ([`set_texture_view`](Self::set_texture_view)), and renders into the texture as often as it wants: what it
/// renders is shown without telling the slot again. A slot with no view draws nothing.
///
/// One slot may be shown by viewports in several windows of an app: they share one device, and each draws the
/// slot's one view stretched to its own box.
#[derive(Clone)]
pub struct WgpuExternalImageSlot {
  state: WgpuExternalImageState,
  native: NativeImageData,
}

#[derive(Clone)]
pub(crate) struct WgpuExternalImageState {
  inner: Arc<RwLock<WgpuExternalImageSlotState>>,
}

struct WgpuExternalImageSlotState {
  view: Option<wgpu::TextureView>,
  width: u32,
  height: u32,
  version: u64,
  /// The device the view is of, by its number, where an extension of its engine gave it.
  of: Option<u64>,
  /// When the view was given, in the order of devices and views.
  given: u64,
}

/// A slot's state as long as something else holds it: an engine keeps these of the slots it drew.
#[derive(Clone)]
pub(crate) struct WgpuExternalImageWeak {
  inner: std::sync::Weak<RwLock<WgpuExternalImageSlotState>>,
}

impl WgpuExternalImageWeak {
  pub(crate) fn upgrade(&self) -> Option<WgpuExternalImageState> {
    Some(WgpuExternalImageState {
      inner: self.inner.upgrade()?,
    })
  }

  pub(crate) fn is(&self, state: &WgpuExternalImageState) -> bool {
    std::ptr::eq(self.inner.as_ptr(), Arc::as_ptr(&state.inner))
  }
}

/// Whether a view given for the device `of`, where that is known, when `given` in the order of devices and
/// views, is of the device numbered `device`.
fn of_device(of: Option<u64>, given: u64, device: u64) -> bool {
  match of {
    Some(of) => of == device,
    None => given > device,
  }
}

#[derive(Clone)]
pub(crate) struct WgpuExternalImageSnapshot {
  pub view: wgpu::TextureView,
  pub width: u32,
  pub height: u32,
  pub version: u64,
  /// The device the view is of, where that is known, and when it was given.
  pub of: Option<u64>,
  pub given: u64,
}

impl WgpuExternalImageSnapshot {
  /// Whether the view is of the device numbered `device`: the one it was given for, or where it was given
  /// outside an extension, one that was there when it was given. A view of a device that was lost since is
  /// of another, and of another instance of wgpu, which takes nothing of it.
  pub(crate) fn is_of(&self, device: u64) -> bool {
    of_device(self.of, self.given, device)
  }
}

impl Default for WgpuExternalImageSlot {
  fn default() -> Self {
    Self::new()
  }
}

/// Why a slot did not take a texture view: lurq's image shader cannot sample the texture as color. The slot keeps
/// the view it had.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WgpuTextureError {
  /// The texture was made without `TextureUsages::TEXTURE_BINDING`, which a shader needs to sample it.
  NotSampled,
  /// The texture is not a 2D one.
  NotTwoDimensional(wgpu::TextureDimension),
  /// The texture is multisampled, with this many samples: render into a multisampled texture and resolve it into
  /// one of a single sample, which the slot is given.
  Multisampled(u32),
  /// The texture's format is not one lurq reads as filtered color: an integer, depth or stencil format, or one of
  /// 32-bit floats (`Rgba16Float` is filtered). An sRGB format and a linear one (`Rgba8Unorm`, `Bgra8Unorm`,
  /// `Rgba16Float`, `Rgb10a2Unorm`) are.
  Format(wgpu::TextureFormat),
}

impl std::fmt::Display for WgpuTextureError {
  fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::NotSampled => write!(formatter, "the texture was made without TextureUsages::TEXTURE_BINDING"),
      Self::NotTwoDimensional(dimension) => write!(formatter, "the texture is {dimension:?}, not D2"),
      Self::Multisampled(samples) => write!(formatter, "the texture has {samples} samples: resolve it into one of 1"),
      Self::Format(format) => write!(
        formatter,
        "lurq does not read {format:?} as filtered color: use an sRGB, unorm or 16-bit float format"
      ),
    }
  }
}

impl std::error::Error for WgpuTextureError {}

/// Whether lurq's image shader can sample `texture` as color.
fn sampled_as_color(texture: &wgpu::Texture) -> Result<(), WgpuTextureError> {
  if !texture.usage().contains(wgpu::TextureUsages::TEXTURE_BINDING) {
    return Err(WgpuTextureError::NotSampled);
  }
  if texture.dimension() != wgpu::TextureDimension::D2 {
    return Err(WgpuTextureError::NotTwoDimensional(texture.dimension()));
  }
  if texture.sample_count() != 1 {
    return Err(WgpuTextureError::Multisampled(texture.sample_count()));
  }
  // Without the device's features: a 32-bit float is filtered only where a feature says so, which a view does
  // not tell.
  match texture.format().sample_type(None, None) {
    Some(wgpu::TextureSampleType::Float { filterable: true }) => Ok(()),
    _ => Err(WgpuTextureError::Format(texture.format())),
  }
}

impl WgpuExternalImageSlot {
  /// An empty slot, with an identity of its own.
  pub fn new() -> Self {
    let state = WgpuExternalImageState {
      inner: Arc::new(RwLock::new(WgpuExternalImageSlotState {
        view: None,
        width: 1,
        height: 1,
        version: 0,
        of: None,
        given: 0,
      })),
    };
    let native = NativeImageData::new(
      1,
      1,
      ImagePixelFormat::Rgba8,
      NativeImageBackend::WgpuExternalRgba,
      state.clone(),
    );
    Self { state, native }
  }

  /// What finds the viewports that show this slot in a frame: `frame.viewport(slot.image_id())`
  /// ([`WgpuFrameInfo`](crate::app::wgpu_render::WgpuFrameInfo)). The clones of a slot have the same one.
  pub fn image_id(&self) -> u64 {
    self.native.id()
  }

  /// The slot as a picture of the layout, what a `GpuViewport` is made of.
  pub fn image_data(&self) -> ImageData {
    self.native.image_data()
  }

  /// Has the slot show `view`, of a texture `width` by `height` pixels, without copying its pixels, and returns
  /// the slot's version. Given once for each texture: what is rendered into it later is shown as it is.
  ///
  /// The texture is made with lurq's device (the `SharedWgpuContext` an extension is given), and is a 2D texture
  /// of one sample with `TextureUsages::TEXTURE_BINDING`, of a format lurq reads as filtered color. An sRGB format
  /// holds color encoded as a picture's is, and is decoded by the card; a linear one (`Rgba8Unorm`,
  /// `Rgba16Float`) holds linear color, which is what a shader writes into it; each is shown as the colors it
  /// holds. A texture lurq cannot sample is refused with a [`WgpuTextureError`] and a line in the log, and the
  /// slot keeps the view it had. The slot holds the view, and so the texture, until another is given or
  /// [`clear`](Self::clear) is called.
  ///
  /// A view given within an extension's `initialize` or `prepare` is known to be of the device of that
  /// extension's engine; given from elsewhere, it is taken to be of the device that draws when it is given,
  /// which a view given late by another thread may not be:
  /// [`set_texture_view_of`](Self::set_texture_view_of) is for that. When the device is lost and another
  /// takes its place, the view is drawn no more, until one of the device that draws now is given.
  pub fn set_texture_view(&self, view: wgpu::TextureView, width: u32, height: u32) -> Result<u64, WgpuTextureError> {
    self.give(view, width, height, RUNS_FOR.get())
  }

  /// As [`set_texture_view`](Self::set_texture_view), for a view given from anywhere: another thread of the
  /// app's renderer, a callback. `gpu` is the context the extension was given, with whose device the
  /// texture was made. A view given after its device was lost is then known for what it is and is not
  /// drawn, where one given without its context would be taken for a view of the device that draws now.
  pub fn set_texture_view_of(
    &self,
    gpu: &crate::app::wgpu_render::SharedWgpuContext,
    view: wgpu::TextureView,
    width: u32,
    height: u32,
  ) -> Result<u64, WgpuTextureError> {
    self.give(view, width, height, Some(gpu.number))
  }

  fn give(&self, view: wgpu::TextureView, width: u32, height: u32, of: Option<u64>) -> Result<u64, WgpuTextureError> {
    if let Err(error) = sampled_as_color(view.texture()) {
      tracing::error!("a GpuViewport's slot refused a texture view: {error}");
      return Err(error);
    }
    let mut state = self.state.inner.write();
    state.version = state.version.wrapping_add(1);
    state.view = Some(view);
    state.of = of;
    state.given = next_in_order();
    state.width = width.max(1);
    state.height = height.max(1);
    Ok(state.version)
  }

  /// Empties the slot and returns its version: the viewports that show it draw nothing, and what is under them
  /// shows through. The slot lets go of the view it held, and so of the texture where nothing else holds it.
  pub fn clear(&self) -> u64 {
    let mut state = self.state.inner.write();
    state.version = state.version.wrapping_add(1);
    state.view = None;
    state.version
  }

  /// How many times the slot was given a view or emptied: a number that changes whenever what it shows does.
  pub fn version(&self) -> u64 {
    self.state.version()
  }
}

impl WgpuExternalImageState {
  pub(crate) fn version(&self) -> u64 {
    self.inner.read().version
  }

  pub(crate) fn downgrade(&self) -> WgpuExternalImageWeak {
    WgpuExternalImageWeak {
      inner: Arc::downgrade(&self.inner),
    }
  }

  /// Lets go of the view where it is of the device numbered `device`, which is lost: the view is gone with
  /// the device, and while anything holds it the lost device is held with it, which the card may answer
  /// by giving no whole device in its place. The slot is empty until the app gives a view again.
  pub(crate) fn forget_view_of(&self, device: u64) {
    let mut state = self.inner.write();
    if state.view.is_some() && of_device(state.of, state.given, device) {
      state.view = None;
      state.version = state.version.wrapping_add(1);
    }
  }

  pub(crate) fn snapshot(&self) -> Option<WgpuExternalImageSnapshot> {
    let state = self.inner.read();
    Some(WgpuExternalImageSnapshot {
      view: state.view.clone()?,
      width: state.width,
      height: state.height,
      version: state.version,
      of: state.of,
      given: state.given,
    })
  }
}

#[cfg(test)]
mod tests {
  use super::WgpuExternalImageSlot;

  #[test]
  fn slot_has_stable_image_identity_and_version() {
    let slot = WgpuExternalImageSlot::new();
    let clone = slot.clone();

    assert_eq!(slot.image_id(), clone.image_id());
    assert_eq!(slot.image_data().id(), slot.image_id());
    assert_eq!(slot.version(), 0);
    assert_eq!(clone.clear(), 1);
    assert_eq!(slot.version(), 1);
  }

  #[test]
  fn a_view_is_of_the_device_it_was_given_for_and_of_no_device_made_after_it() {
    use super::{WgpuExternalImageSnapshot, for_device, next_in_order};
    let Some(gpu) = any_device() else {
      eprintln!("skipped: no card to make a texture on");
      return;
    };
    let view = any_view_of(&gpu);
    let snapshot = |slot: &WgpuExternalImageSlot| -> WgpuExternalImageSnapshot {
      slot.state.snapshot().expect("the view that was given")
    };
    let (first, slot) = (next_in_order(), WgpuExternalImageSlot::new());
    // Given by an extension of the first device's engine: it is that device's, whatever is made later.
    for_device(first, || slot.set_texture_view(view.clone(), 4, 4)).unwrap();
    let second = next_in_order();
    assert!(snapshot(&slot).is_of(first));
    assert!(
      !snapshot(&slot).is_of(second),
      "a view of the device before is not the next one's"
    );
    // Given by an extension of the second's: the second's, and not the first's.
    for_device(second, || slot.set_texture_view(view.clone(), 4, 4)).unwrap();
    assert!(snapshot(&slot).is_of(second) && !snapshot(&slot).is_of(first));
    // Given outside any extension: of the devices that were there, and of none made after it.
    slot.set_texture_view(view.clone(), 4, 4).unwrap();
    let third = next_in_order();
    assert!(snapshot(&slot).is_of(second));
    assert!(!snapshot(&slot).is_of(third));
    // A view of a device that is lost is let go of; one of another device stays.
    let view = snapshot(&slot).view;
    for_device(second, || slot.set_texture_view(view, 4, 4)).unwrap();
    slot.state.forget_view_of(first);
    assert!(slot.state.snapshot().is_some(), "the view is of the second, and stays");
    let version = slot.version();
    slot.state.forget_view_of(second);
    assert!(slot.state.snapshot().is_none());
    assert_eq!(slot.version(), version + 1, "an empty slot is another picture");
    let weak = slot.state.downgrade();
    assert!(weak.is(&slot.state) && weak.upgrade().is_some());
    // Given with its context, from anywhere: of that context's device, whichever draws when it is given.
    let fourth = next_in_order();
    let view = any_view_of(&gpu);
    let lost = crate::app::wgpu_render::SharedWgpuContext { number: first, ..gpu };
    slot.set_texture_view_of(&lost, view, 4, 4).unwrap();
    assert!(snapshot(&slot).is_of(first));
    assert!(
      !snapshot(&slot).is_of(fourth),
      "a view of a lost device, given late, is not the next one's"
    );
    // One extension's run within another's leaves the outer one's device as it was.
    for_device(first, || {
      for_device(second, || {});
      slot.clear();
    });
  }

  /// The context of a device, for what does not look at which device a view is of. One for the whole test, of
  /// the backend lurq takes by itself: devices of Vulkan made and let go of one after another, with a view of
  /// each left alive, ended the test program now and then (an access violation, not in lurq's code).
  fn any_device() -> Option<crate::app::wgpu_render::SharedWgpuContext> {
    let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    descriptor.backends = if cfg!(windows) {
      wgpu::Backends::DX12
    } else {
      wgpu::Backends::all()
    };
    let instance = wgpu::Instance::new(descriptor);
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).ok()?;
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()?;
    Some(crate::app::wgpu_render::SharedWgpuContext {
      instance,
      adapter,
      device,
      queue,
      surface_format: wgpu::TextureFormat::Rgba8Unorm,
      number: 0,
    })
  }

  /// A view of a texture of `gpu`'s device.
  fn any_view_of(gpu: &crate::app::wgpu_render::SharedWgpuContext) -> wgpu::TextureView {
    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
      label: None,
      size: wgpu::Extent3d {
        width: 4,
        height: 4,
        depth_or_array_layers: 1,
      },
      mip_level_count: 1,
      sample_count: 1,
      dimension: wgpu::TextureDimension::D2,
      format: wgpu::TextureFormat::Rgba8Unorm,
      usage: wgpu::TextureUsages::TEXTURE_BINDING,
      view_formats: &[],
    });
    texture.create_view(&Default::default())
  }
}
