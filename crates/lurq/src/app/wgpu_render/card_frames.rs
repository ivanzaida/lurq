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

use std::sync::Arc;

use parking_lot::Mutex;
use windows::Win32::{
  Foundation::HANDLE,
  Graphics::{
    Direct3D12::{ID3D12Fence, ID3D12Resource},
    Dxgi::Common::{
      DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_FORMAT_NV12, DXGI_FORMAT_R8_UNORM, DXGI_FORMAT_R8G8_UNORM,
      DXGI_FORMAT_R8G8B8A8_UNORM,
    },
  },
};

use crate::images::{CardCrop, CardFrame, CardFrames, CardLease, CardSurface};

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
  size: (u32, u32),
  textures: Vec<wgpu::Texture>,
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
pub(super) fn announce(device: &wgpu::Device) {
  if !device.features().contains(wgpu::Features::TEXTURE_FORMAT_NV12) {
    return;
  }
  // SAFETY: the device's own handle is read and nothing of it is changed.
  let Some(hal) = (unsafe { device.as_hal::<wgpu::hal::api::Dx12>() }) else {
    return;
  };
  // SAFETY: as above.
  let card = unsafe { hal.raw_device().GetAdapterLuid() };
  crate::images::set_card_frames_adapter(Some((card.LowPart, card.HighPart)));
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
  let format = match said.Format {
    DXGI_FORMAT_NV12 => wgpu::TextureFormat::NV12,
    DXGI_FORMAT_R8_UNORM => wgpu::TextureFormat::R8Unorm,
    DXGI_FORMAT_R8G8_UNORM => wgpu::TextureFormat::Rg8Unorm,
    DXGI_FORMAT_R8G8B8A8_UNORM => wgpu::TextureFormat::Rgba8Unorm,
    DXGI_FORMAT_B8G8R8A8_UNORM => wgpu::TextureFormat::Bgra8Unorm,
    _ => return None,
  };
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
  let texture = unsafe {
    device.create_texture_from_hal::<wgpu::hal::api::Dx12>(
      texture,
      &wgpu::TextureDescriptor {
        label: Some("lurq_card_frame"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
      },
    )
  };
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

/// What is kept to draw an image whose frames stay on the card: each of its surfaces opened, the fence that
/// tells when a frame is written, and the frame that is drawn now.
pub(super) struct Opened {
  slots: Vec<Slot>,
  frames: CardFrames,
  fence: Option<ID3D12Fence>,
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
  /// the gray and the color, or the red, green and blue alone. `None` where a surface does not open or is not
  /// what it is said to be, and nothing is drawn of the frames.
  pub(super) fn open(
    device: &wgpu::Device,
    frames: &CardFrames,
    (width, height): (u32, u32),
    leases: &Leases,
    bind: impl Fn(&wgpu::TextureView, Option<&wgpu::TextureView>) -> wgpu::BindGroup,
  ) -> Option<Self> {
    let slots = frames
      .surfaces()
      .iter()
      .map(|surface| {
        let (first, second, size, textures) = match *surface {
          CardSurface::Nv12(handle) => {
            let (texture, size) = open(device, handle, &[wgpu::TextureFormat::NV12])?;
            let (gray, color) = planes(&texture);
            (gray, Some(color), size, vec![texture])
          }
          CardSurface::Planes { gray, color } => {
            let (gray, size) = open(device, gray, &[wgpu::TextureFormat::R8Unorm])?;
            let (color, _) = open(device, color, &[wgpu::TextureFormat::Rg8Unorm])?;
            (
              gray.create_view(&Default::default()),
              Some(color.create_view(&Default::default())),
              size,
              vec![gray, color],
            )
          }
          CardSurface::Rgba(handle) => {
            let (texture, size) = open(
              device,
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
      Some(handle) => Some(fence(device, handle)?),
      None => None,
    };
    Some(Self {
      slots,
      frames: frames.clone(),
      fence,
      drawn: None,
      leases: leases.clone(),
      width,
      height,
    })
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
  /// stays until then. The lease of a frame that is no longer drawn goes to be let go of.
  pub(super) fn pick(&mut self) {
    let Some((count, frame)) = self.frames.latest() else {
      return;
    };
    if self.drawn.as_ref().is_some_and(|(drawn, _)| *drawn == count) || frame.surface >= self.slots.len() {
      return;
    }
    let written = match (frame.ready, &self.fence) {
      // SAFETY: the fence is read.
      (Some(value), Some(fence)) => (unsafe { fence.GetCompletedValue() }) >= value,
      _ => true,
    };
    if !written {
      return;
    }
    if let Some((_, before)) = self.drawn.replace((count, frame)) {
      self.leases.keep(before.lease);
    }
  }

  /// The surface that is drawn now, by its number, where a frame was taken.
  pub(super) fn drawn(&self) -> Option<(usize, &Slot)> {
    let (_, frame) = self.drawn.as_ref()?;
    Some((frame.surface, self.slots.get(frame.surface)?))
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
