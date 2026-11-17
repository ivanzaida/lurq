//! The frames of a video that stay on the graphics card, as this renderer draws them on Direct3D 12.
//!
//! What decodes a video keeps each frame in one of a few textures that are shared by handle
//! ([`CardFrames`]). Here each handle is opened once on the renderer's own device and made a texture of wgpu's:
//! an NV12 texture's two planes, or a texture of gray and one of color, are what the NV12 pipeline samples, as
//! it samples the two textures a frame from memory is sent to; a texture of red, green and blue is what the
//! pipeline of pictures samples. A frame is drawn from where the decoder put it: nothing of it is read or sent
//! by the program.
//!
//! Which frame is drawn is chosen once in each of the renderer's frames ([`Opened::pick`]): the newest one that
//! is all written, by the decoder's fence where it has one, and the one before it until then. Nothing waits. The
//! lease of a frame that is no longer drawn is kept until the card has done all the work that drew it
//! ([`Leases`]).
//!
//! Under Vulkan the same holds: a texture Direct3D shares is given to Vulkan by its handle
//! (`VK_KHR_external_memory_win32`), and drawn where it is; one NV12 texture on some cards alone
//! ([`vulkan_takes_nv12`]). Vulkan cannot ask the texture what it is, nor a
//! fence of Direct3D how far it is: a Direct3D 11 device of the same card is asked both ([`Readers`]).
//!
//! Where the renderer's device does not open the handles, for the decoder is on another card, the frames are
//! copied through memory instead ([`Copied`]), and drawn the same way from textures of the renderer's own.

use std::{
  sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
  },
  time::{Duration, Instant},
};

use parking_lot::Mutex;
use windows::Win32::{
  Foundation::HANDLE,
  Graphics::{
    Direct3D11::ID3D11Fence,
    Direct3D12::{ID3D12Fence, ID3D12Resource},
    Dxgi::{
      Common::{
        DXGI_FORMAT, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_FORMAT_NV12, DXGI_FORMAT_R8_UNORM, DXGI_FORMAT_R8G8_UNORM,
        DXGI_FORMAT_R8G8B8A8_UNORM,
      },
      CreateDXGIFactory1, IDXGIFactory1,
    },
  },
};

use super::card_copies::{Copied, Readers};
use crate::images::{CardCrop, CardFrame, CardFrames, CardFramesWay, CardLease, CardSurface};

/// The leases of frames that are drawn no more, kept until the card has finished the work that drew them.
#[derive(Clone, Default)]
pub(super) struct Leases {
  kept: Arc<Mutex<Vec<CardLease>>>,
}

impl Leases {
  fn keep(&self, lease: Option<CardLease>) {
    if let Some(lease) = lease {
      self.kept.lock().push(lease);
    }
  }

  /// Lets go of what is kept once the card has done everything given to `queue` so far. Called after a frame
  /// is submitted: what was kept while it was made was drawn by an earlier frame at the latest.
  pub(super) fn let_go_after(&self, queue: &wgpu::Queue) {
    let kept = std::mem::take(&mut *self.kept.lock());
    if !kept.is_empty() {
      queue.on_submitted_work_done(move || drop(kept));
    }
  }
}

/// One of the surfaces of a video's frames, opened: its views, and what binds them for the pipeline.
pub(super) struct Slot {
  pub bind_group: wgpu::BindGroup,
  /// The gray, or the red, green and blue.
  pub first: wgpu::TextureView,
  /// The color, where the frame is gray and color.
  pub second: Option<wgpu::TextureView>,
  /// The size of the texture, which may be larger than the picture.
  pub(super) size: (u32, u32),
  pub(super) textures: Vec<wgpu::Texture>,
}

impl Slot {
  /// The textures behind the surface: its only one, or the gray's and the color's.
  #[cfg(test)]
  pub(super) fn textures(&self) -> &[wgpu::Texture] {
    &self.textures
  }
}

/// Says that `device` draws frames of the card, and on which card, where it can: on Direct3D 12 with NV12
/// textures. What decodes a video asks ([`card_frames_adapter`](crate::images::card_frames_adapter)) and decodes
/// on that card; where nothing said so, a frame goes through memory, as before.
pub(super) fn announce(device: &wgpu::Device) -> bool {
  if !device.features().contains(wgpu::Features::TEXTURE_FORMAT_NV12) {
    return false;
  }
  // SAFETY: the device's own handle is read and nothing of it is changed.
  let Some(hal) = (unsafe { device.as_hal::<wgpu::hal::api::Dx12>() }) else {
    return false;
  };
  // SAFETY: as above.
  let card = unsafe { hal.raw_device().GetAdapterLuid() };
  crate::images::set_card_frames_adapter(Some((card.LowPart, card.HighPart)));
  crate::images::set_card_frames_nv12(true);
  true
}

/// AMD's, as a card says who made it.
const AMD: u32 = 0x1002;
/// A device lost this soon after Vulkan was given an NV12 texture is taken to be lost for it.
const LOST_FOR_NV12_WITHIN: Duration = Duration::from_secs(5);
/// When Vulkan was last given an NV12 texture.
static NV12_GIVEN: Mutex<Option<Instant>> = Mutex::new(None);
/// Whether a device of Vulkan was lost for an NV12 texture in this program: none is given one again.
static NV12_LOST: AtomicBool = AtomicBool::new(false);

/// Whether one NV12 texture is given to `device`, of Vulkan.
///
/// Vulkan has the format, and what a card does with such a texture of Direct3D is its driver's: of the two
/// this was run on, an RTX 5080 lost its device for it, at once and each time, and the Radeon of a
/// processor took it and drew it. So it is given to a card of AMD's and to no other, and to none once a
/// device was lost soon after it was given ([`a_device_is_lost`]): another driver of AMD's may lose its
/// device as the RTX did, and the program is not to lose one device after another for it.
fn vulkan_takes_nv12(device: &wgpu::Device) -> bool {
  device.features().contains(wgpu::Features::TEXTURE_FORMAT_NV12)
    && nv12_is_tried_on(device.adapter_info().vendor)
    && !NV12_LOST.load(Ordering::Acquire)
}

/// Whether a card of the maker `vendor` is one an NV12 texture is tried on.
pub(super) fn nv12_is_tried_on(vendor: u32) -> bool {
  vendor == AMD
}

/// A device is lost. Where Vulkan was given an NV12 texture a moment ago, it is taken to be lost for that,
/// and no device is given one again in this program: their frames come through memory, as on a card that
/// is known not to take them.
pub(super) fn a_device_is_lost() {
  if NV12_GIVEN
    .lock()
    .is_some_and(|given| given.elapsed() < LOST_FOR_NV12_WITHIN)
  {
    NV12_LOST.store(true, Ordering::Release);
    crate::images::set_card_frames_nv12(false);
  }
}

/// For a test: says when Vulkan was given an NV12 texture, or that it was not.
#[cfg(test)]
pub(super) fn nv12_was_given(at: Option<Instant>) {
  *NV12_GIVEN.lock() = at;
}

/// For a test: whether a device was lost for an NV12 texture.
#[cfg(test)]
pub(super) fn nv12_is_lost() -> bool {
  NV12_LOST.load(Ordering::Acquire)
}

/// For a test: nothing was lost for an NV12 texture, as when the program starts.
#[cfg(test)]
pub(super) fn forget_the_lost_nv12() {
  NV12_LOST.store(false, Ordering::Release);
  *NV12_GIVEN.lock() = None;
}

/// Says the same of a device of Vulkan that is given the textures Direct3D shares: on which card it draws,
/// found among the system's cards by what wgpu says of `adapter`. Whether it said so.
pub(super) fn announce_vulkan(device: &wgpu::Device, adapter: &wgpu::Adapter) -> bool {
  if !device.features().contains(wgpu::Features::VULKAN_EXTERNAL_MEMORY_WIN32) {
    return false;
  }
  let info = adapter.get_info();
  if info.backend != wgpu::Backend::Vulkan {
    return false;
  }
  // SAFETY: the system is asked for its cards, and what each says of itself is read.
  let card = unsafe {
    let Ok(cards) = CreateDXGIFactory1::<IDXGIFactory1>() else {
      return false;
    };
    (0..)
      .map_while(|index| cards.EnumAdapters1(index).ok())
      .filter_map(|card| card.GetDesc1().ok())
      .find(|said| said.VendorId == info.vendor && said.DeviceId == info.device)
      .map(|said| said.AdapterLuid)
  };
  let Some(card) = card else {
    return false;
  };
  crate::images::set_card_frames_adapter(Some((card.LowPart, card.HighPart)));
  // Where one NV12 texture is not given to Vulkan, what decodes into one gives red, green and blue instead.
  crate::images::set_card_frames_nv12(vulkan_takes_nv12(device));
  true
}

/// The format of a texture of the card, as wgpu names it; `None` for one no frame is in.
fn format_of(said: DXGI_FORMAT) -> Option<wgpu::TextureFormat> {
  Some(match said {
    DXGI_FORMAT_NV12 => wgpu::TextureFormat::NV12,
    DXGI_FORMAT_R8_UNORM => wgpu::TextureFormat::R8Unorm,
    DXGI_FORMAT_R8G8_UNORM => wgpu::TextureFormat::Rg8Unorm,
    DXGI_FORMAT_R8G8B8A8_UNORM => wgpu::TextureFormat::Rgba8Unorm,
    DXGI_FORMAT_B8G8R8A8_UNORM => wgpu::TextureFormat::Bgra8Unorm,
    _ => return None,
  })
}

/// What a texture of the card is said to be to wgpu, whichever of its backends opened it.
fn described(format: wgpu::TextureFormat, size: wgpu::Extent3d) -> wgpu::TextureDescriptor<'static> {
  wgpu::TextureDescriptor {
    label: Some("lurq_card_frame"),
    size,
    mip_level_count: 1,
    sample_count: 1,
    dimension: wgpu::TextureDimension::D2,
    format,
    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
    view_formats: &[],
  }
}

/// The texture behind `handle`, given to a `device` of Vulkan, with its size. `None` where the device is not
/// one of Vulkan that takes textures Direct3D shares, no card opens the handle to say what it is, it is not
/// a texture of one of `formats`, it is an NV12 one and the card is not given those
/// ([`vulkan_takes_nv12`]), or Vulkan does not take it (another card).
fn open_in_vulkan(
  device: &wgpu::Device,
  readers: &mut Readers,
  handle: isize,
  formats: &[wgpu::TextureFormat],
) -> Option<(wgpu::Texture, (u32, u32))> {
  if !device.features().contains(wgpu::Features::VULKAN_EXTERNAL_MEMORY_WIN32) {
    return None;
  }
  // SAFETY: the device's own handle is used to take a texture, which changes nothing of the device.
  let hal = unsafe { device.as_hal::<wgpu::hal::api::Vulkan>() }?;
  let (said, width, height) = readers.said_of(handle)?;
  let format = format_of(said)?;
  // Gray and color apart, and red, green and blue, are taken by any card; one NV12 texture by some.
  let nv12 = format == wgpu::TextureFormat::NV12;
  if !formats.contains(&format) || (nv12 && !vulkan_takes_nv12(device)) {
    return None;
  }
  if nv12 {
    *NV12_GIVEN.lock() = Some(Instant::now());
  }
  let size = wgpu::Extent3d {
    width,
    height,
    depth_or_array_layers: 1,
  };
  // SAFETY: the handle is one of a shared texture of Direct3D 11 of this format and size, as the card it is
  // on said, kept open by the frames this is asked for.
  let texture = unsafe {
    hal.texture_from_d3d11_shared_handle(
      HANDLE(handle as _),
      &wgpu::hal::TextureDescriptor {
        label: Some("lurq_card_frame"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUses::RESOURCE | wgpu::TextureUses::COPY_SRC,
        memory_flags: wgpu::hal::MemoryFlags::empty(),
        view_formats: Vec::new(),
      },
    )
  }
  .ok()?;
  drop(hal);
  // SAFETY: the texture was taken by this device, is as the descriptor says, and holds a frame or what it
  // was made with: nothing reads what was never written.
  let texture = unsafe { device.create_texture_from_hal::<wgpu::hal::api::Vulkan>(texture, &described(format, size)) };
  Some((texture, (width, height)))
}

/// The texture behind `handle` as `device` draws it, with its size: opened by Direct3D 12, or given to
/// Vulkan.
fn opened(
  device: &wgpu::Device,
  readers: &mut Readers,
  handle: isize,
  formats: &[wgpu::TextureFormat],
) -> Option<(wgpu::Texture, (u32, u32))> {
  open(device, handle, formats).or_else(|| open_in_vulkan(device, readers, handle, formats))
}

/// What tells how far a decoder's fence is: the renderer's own device where it is of Direct3D 12, and a
/// Direct3D 11 device of the decoder's card where the renderer draws through Vulkan.
enum Fence {
  Own(ID3D12Fence),
  Read(ID3D11Fence),
}

impl Fence {
  /// The value the fence has reached.
  fn reached(&self) -> u64 {
    // SAFETY: the fence is read.
    match self {
      Self::Own(fence) => unsafe { fence.GetCompletedValue() },
      Self::Read(fence) => unsafe { fence.GetCompletedValue() },
    }
  }
}

/// The frames of `image`, where they are frames of the card.
pub(super) fn of(image: &crate::images::ImageCmd) -> Option<&CardFrames> {
  image.native.as_ref()?.payload::<CardFrames>()
}

/// The texture behind `handle`, opened on `device`, with its size. `None` where the device is not one of
/// Direct3D 12, does not open it (another card), or it is not a texture of one of `formats`.
pub(super) fn open(
  device: &wgpu::Device,
  handle: isize,
  formats: &[wgpu::TextureFormat],
) -> Option<(wgpu::Texture, (u32, u32))> {
  // SAFETY: the device's own handle is used to open a texture, which changes nothing of the device.
  let hal = unsafe { device.as_hal::<wgpu::hal::api::Dx12>() }?;
  let mut resource: Option<ID3D12Resource> = None;
  // SAFETY: the handle is one of a shared texture, kept open by the frames this is asked for.
  unsafe { hal.raw_device().OpenSharedHandle(HANDLE(handle as _), &mut resource) }.ok()?;
  let resource = resource?;
  // SAFETY: the resource was just opened; its description is read.
  let said = unsafe { resource.GetDesc() };
  let format = format_of(said.Format)?;
  if !formats.contains(&format) {
    return None;
  }
  let (width, height) = (u32::try_from(said.Width).ok()?, said.Height);
  let size = wgpu::Extent3d {
    width,
    height,
    depth_or_array_layers: 1,
  };
  // SAFETY: the resource is a texture of this format and size with one level and one sample, as it says itself.
  let texture =
    unsafe { wgpu::hal::dx12::Device::texture_from_raw(resource, format, wgpu::TextureDimension::D2, size, 1, 1) };
  drop(hal);
  // SAFETY: the texture was opened on this device, is as the descriptor says, and holds a frame or what it was
  // made with: nothing reads what was never written.
  let texture = unsafe { device.create_texture_from_hal::<wgpu::hal::api::Dx12>(texture, &described(format, size)) };
  Some((texture, (width, height)))
}

/// The two planes of an NV12 texture: the gray, and the color.
pub(super) fn planes(texture: &wgpu::Texture) -> (wgpu::TextureView, wgpu::TextureView) {
  let plane = |aspect, format| {
    texture.create_view(&wgpu::TextureViewDescriptor {
      aspect,
      format: Some(format),
      ..Default::default()
    })
  };
  (
    plane(wgpu::TextureAspect::Plane0, wgpu::TextureFormat::R8Unorm),
    plane(wgpu::TextureAspect::Plane1, wgpu::TextureFormat::Rg8Unorm),
  )
}

/// The fence behind `handle`, opened on `device`.
fn fence(device: &wgpu::Device, handle: isize) -> Option<ID3D12Fence> {
  // SAFETY: the device's own handle is used to open a fence, which changes nothing of the device.
  let hal = unsafe { device.as_hal::<wgpu::hal::api::Dx12>() }?;
  let mut fence: Option<ID3D12Fence> = None;
  // SAFETY: the handle is one of a shared fence, kept open by the frames this is asked for.
  unsafe { hal.raw_device().OpenSharedHandle(HANDLE(handle as _), &mut fence) }.ok()?;
  fence
}

/// How the frames of an image come to the renderer.
enum Way {
  /// Each surface opened on the renderer's own device, and the fence that tells when a frame is written.
  Card { slots: Vec<Slot>, fence: Option<Fence> },
  /// The surfaces are another card's: each frame is copied through memory.
  Memory(Copied),
  /// No card opens them: nothing is drawn.
  Nowhere,
}

/// Each surface of `frames` opened on `device`, and their fence. `None` where a surface does not open (another
/// card) or is not what it is said to be.
fn on_the_card(
  device: &wgpu::Device,
  frames: &CardFrames,
  readers: &mut Readers,
  bind: &impl Fn(&wgpu::TextureView, Option<&wgpu::TextureView>) -> wgpu::BindGroup,
) -> Option<Way> {
  let slots = frames
    .surfaces()
    .iter()
    .map(|surface| {
      let (first, second, size, textures) = match *surface {
        CardSurface::Nv12(handle) => {
          let (texture, size) = opened(device, readers, handle, &[wgpu::TextureFormat::NV12])?;
          let (gray, color) = planes(&texture);
          (gray, Some(color), size, vec![texture])
        }
        CardSurface::Planes { gray, color } => {
          let (gray, size) = opened(device, readers, gray, &[wgpu::TextureFormat::R8Unorm])?;
          let (color, _) = opened(device, readers, color, &[wgpu::TextureFormat::Rg8Unorm])?;
          (
            gray.create_view(&Default::default()),
            Some(color.create_view(&Default::default())),
            size,
            vec![gray, color],
          )
        }
        CardSurface::Rgba(handle) => {
          let (texture, size) = opened(
            device,
            readers,
            handle,
            &[wgpu::TextureFormat::Rgba8Unorm, wgpu::TextureFormat::Bgra8Unorm],
          )?;
          (texture.create_view(&Default::default()), None, size, vec![texture])
        }
      };
      Some(Slot {
        bind_group: bind(&first, second.as_ref()),
        first,
        second,
        size,
        textures,
      })
    })
    .collect::<Option<Vec<_>>>()?;
  let fence = match frames.fence() {
    Some(handle) => Some(match fence(device, handle) {
      Some(own) => Fence::Own(own),
      None => Fence::Read(readers.fence_of(frames.surfaces().first()?.handles().0, handle)?),
    }),
    None => None,
  };
  Some(Way::Card { slots, fence })
}

/// What is kept to draw an image whose frames are textures of a card: how they come to the renderer, and the
/// frame that is drawn now.
pub(super) struct Opened {
  way: Way,
  frames: CardFrames,
  /// The frame that is drawn, and which one it is of those that were shown.
  drawn: Option<(u64, CardFrame)>,
  leases: Leases,
  /// The size of the picture.
  width: u32,
  height: u32,
}

impl Drop for Opened {
  fn drop(&mut self) {
    // The card may still be drawing it.
    if let Some((_, frame)) = self.drawn.take() {
      self.leases.keep(frame.lease);
    }
  }
}

impl Opened {
  /// Opens each surface of `frames`, whose picture is `width` by `height`, and binds its views with `bind`:
  /// the gray and the color, or the red, green and blue alone. Where `device` does not open them, the card that
  /// does is found among `readers`, and each frame is copied through memory; where none does, or a surface is
  /// not what it is said to be, nothing is drawn of the frames. The frames are told which
  /// ([`CardFrames::way`]).
  pub(super) fn open(
    device: &wgpu::Device,
    frames: &CardFrames,
    picture: (u32, u32),
    leases: &Leases,
    readers: &mut Readers,
    bind: impl Fn(&wgpu::TextureView, Option<&wgpu::TextureView>) -> wgpu::BindGroup,
  ) -> Self {
    let way = on_the_card(device, frames, readers, &bind)
      .or_else(|| Copied::open(device, frames, readers, &bind).map(Way::Memory))
      .unwrap_or(Way::Nowhere);
    Self::by(way, frames, picture, leases)
  }

  /// As [`open`](Self::open), by a copy through memory though the frames be on the renderer's own card.
  #[cfg(test)]
  pub(super) fn through_memory(
    device: &wgpu::Device,
    frames: &CardFrames,
    picture: (u32, u32),
    leases: &Leases,
    readers: &mut Readers,
    bind: impl Fn(&wgpu::TextureView, Option<&wgpu::TextureView>) -> wgpu::BindGroup,
  ) -> Self {
    let way = Copied::open(device, frames, readers, &bind).map_or(Way::Nowhere, Way::Memory);
    Self::by(way, frames, picture, leases)
  }

  fn by(way: Way, frames: &CardFrames, (width, height): (u32, u32), leases: &Leases) -> Self {
    frames.found(match way {
      Way::Card { .. } => CardFramesWay::Card,
      Way::Memory(_) => CardFramesWay::Memory,
      Way::Nowhere => CardFramesWay::Nowhere,
    });
    Self {
      way,
      frames: frames.clone(),
      drawn: None,
      leases: leases.clone(),
      width,
      height,
    }
  }

  /// Whether this is `frames` opened, for a picture of this size.
  pub(super) fn is_of(&self, frames: &CardFrames, (width, height): (u32, u32)) -> bool {
    self.frames.same(frames) && (self.width, self.height) == (width, height)
  }

  /// Whether the frames are drawn as red, green and blue.
  pub(super) fn is_rgba(&self) -> bool {
    self.frames.is_rgba()
  }

  /// Takes the frame that was shown last as the one to draw, once it is all written; the one drawn before it
  /// stays until then. On the card, the lease of a frame that is no longer drawn goes to be let go of. Through
  /// memory, a frame that is written is copied on its card, and is the one to draw once that is done and it is
  /// sent to `queue`'s card; its lease is let go of then, for the decoder's texture is drawn by nothing.
  pub(super) fn pick(&mut self, queue: &wgpu::Queue) {
    if let Way::Memory(copied) = &mut self.way {
      if let Some((count, mut frame)) = copied.arrived(queue) {
        drop(frame.lease.take());
        self.drawn = Some((count, frame));
      }
      if copied.is_copying() {
        return;
      }
    }
    let Some((count, frame)) = self.frames.latest() else {
      return;
    };
    if self.drawn.as_ref().is_some_and(|(drawn, _)| *drawn == count) || frame.surface >= self.frames.surfaces().len() {
      return;
    }
    let reached = match &self.way {
      Way::Card { fence, .. } => fence.as_ref().map(Fence::reached),
      Way::Memory(copied) => copied.reached(),
      Way::Nowhere => return,
    };
    let written = match (frame.ready, reached) {
      (Some(value), Some(reached)) => reached >= value,
      _ => true,
    };
    if !written {
      return;
    }
    match &mut self.way {
      Way::Card { .. } => {
        if let Some((_, before)) = self.drawn.replace((count, frame)) {
          self.leases.keep(before.lease);
        }
      }
      Way::Memory(copied) => copied.copy(count, frame),
      Way::Nowhere => {}
    }
  }

  /// The surface that is drawn now, by its number, where a frame was taken. Through memory there is one, which
  /// holds the frame that was read last.
  pub(super) fn drawn(&self) -> Option<(usize, &Slot)> {
    let (_, frame) = self.drawn.as_ref()?;
    match &self.way {
      Way::Card { slots, .. } => Some((frame.surface, slots.get(frame.surface)?)),
      Way::Memory(copied) => Some((0, copied.slot())),
      Way::Nowhere => None,
    }
  }

  /// Where in the drawn surface the part `min` to `max` of the picture is, both as parts of one: the picture
  /// may be a part of its texture.
  pub(super) fn part(&self, min: [f32; 2], max: [f32; 2]) -> ([f32; 2], [f32; 2]) {
    let Some(((_, frame), (_, slot))) = self.drawn.as_ref().zip(self.drawn()) else {
      return (min, max);
    };
    let (wide, high) = slot.size;
    let crop = frame.crop.unwrap_or(CardCrop {
      left: 0,
      top: 0,
      width: self.width.min(wide),
      height: self.height.min(high),
    });
    if (crop.left, crop.top, crop.width, crop.height) == (0, 0, wide, high) || wide == 0 || high == 0 {
      return (min, max);
    }
    // An edge of the picture inside the texture is drawn from a little within it: a sample at the very edge
    // would be mixed with what lies past it, which is no part of the picture. The color is at half the size,
    // and so a whole pixel is kept from.
    let within = if self.is_rgba() { 0.5 } else { 1.0 };
    let edge = |at: u32, end: u32| if at > 0 && at < end { within } else { 0.0 };
    let (left, top) = (
      crop.left as f32 + edge(crop.left, wide),
      crop.top as f32 + edge(crop.top, high),
    );
    let (right, bottom) = (crop.left + crop.width, crop.top + crop.height);
    let (right, bottom) = (right as f32 - edge(right, wide), bottom as f32 - edge(bottom, high));
    let (from, to) = (
      [left / wide as f32, top / high as f32],
      [right / wide as f32, bottom / high as f32],
    );
    let at = |part: [f32; 2]| {
      [
        from[0] + part[0] * (to[0] - from[0]),
        from[1] + part[1] * (to[1] - from[1]),
      ]
    };
    (at(min), at(max))
  }
}
