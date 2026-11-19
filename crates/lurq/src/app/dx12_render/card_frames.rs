//! The frames of a video that stay on the graphics card, as lurq's own Direct3D 12 renderer draws them.
//!
//! What decodes a video keeps each frame in one of a few textures that are shared by handle
//! ([`CardFrames`]). Here each handle is opened once on the renderer's own device: an NV12 texture's two
//! planes, or a texture of gray and one of color, are what the NV12 pipeline samples, as it samples the two
//! textures a frame from memory is sent to; a texture of red, green and blue is what the pipeline of pictures
//! samples. A frame is drawn from where the decoder put it: nothing of it is read or sent by the program.
//!
//! Which frame is drawn is chosen once for each image in each of the renderer's frames ([`Opened::pick`]): the
//! newest one that is all written, by the decoder's fence where it has one, and the one before it until then.
//! Nothing waits. The lease of a frame that is no longer drawn is given to the renderer, which keeps it until
//! the card has done the work that drew it.
//!
//! Where the renderer's device does not open the handles, for the decoder is on another card, the frames are
//! read by a device of that card ([`card_read`](crate::app::card_read)) and sent through memory into textures
//! of the renderer's own ([`Copied`]), and drawn the same way from them.

use windows::Win32::{
  Foundation::HANDLE,
  Graphics::{
    Direct3D12::{
      D3D12_CPU_PAGE_PROPERTY_UNKNOWN, D3D12_HEAP_FLAG_NONE, D3D12_HEAP_PROPERTIES, D3D12_HEAP_TYPE_DEFAULT,
      D3D12_MEMORY_POOL_UNKNOWN, D3D12_RESOURCE_DESC, D3D12_RESOURCE_DIMENSION_TEXTURE2D, D3D12_RESOURCE_FLAG_NONE,
      D3D12_RESOURCE_STATE_COPY_DEST, D3D12_TEXTURE_LAYOUT_UNKNOWN, ID3D12Device, ID3D12Fence, ID3D12Resource,
    },
    Dxgi::Common::{
      DXGI_FORMAT, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_FORMAT_NV12, DXGI_FORMAT_R8_UNORM, DXGI_FORMAT_R8G8_UNORM,
      DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_SAMPLE_DESC,
    },
  },
};

use crate::{
  app::card_read::{Read, Readers},
  images::{CardFrame, CardFrames, CardFramesWay, CardLease, CardSurface},
};

/// Says that `device` draws frames of the card, and on which card. What decodes a video asks
/// ([`card_frames_adapter`](crate::images::card_frames_adapter)) and decodes on that card; where nothing said
/// so, a frame goes through memory.
pub(super) fn announce(device: &ID3D12Device) {
  // SAFETY: the device is asked which card it is of.
  let card = unsafe { device.GetAdapterLuid() };
  crate::images::set_card_frames_adapter(Some((card.LowPart, card.HighPart)));
  crate::images::set_card_frames_nv12(true);
}

/// What the textures of a surface are.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
  /// One NV12 texture: its first plane is the gray, its second the color.
  Nv12,
  /// A texture of gray and one of color.
  Planes,
  /// One texture of red, green and blue, of this format.
  Rgba(DXGI_FORMAT),
}

/// One of the surfaces of a video's frames, opened on the renderer's device.
pub(super) struct Slot {
  /// The gray, or the NV12 texture, or the red, green and blue.
  pub first: ID3D12Resource,
  /// The color, where it is a texture of its own.
  pub second: Option<ID3D12Resource>,
  pub kind: Kind,
  /// The size of the texture, which may be larger than the picture.
  pub size: (u32, u32),
}

/// The texture behind `handle`, opened on `device`, with its format and size. `None` where the device does not
/// open it (another card), or it is not a texture of one of `formats`.
fn opened(
  device: &ID3D12Device,
  handle: isize,
  formats: &[DXGI_FORMAT],
) -> Option<(ID3D12Resource, DXGI_FORMAT, (u32, u32))> {
  let mut resource: Option<ID3D12Resource> = None;
  // SAFETY: the handle is one of a shared texture, kept open by the frames this is asked for.
  unsafe { device.OpenSharedHandle(HANDLE(handle as _), &mut resource) }.ok()?;
  let resource = resource?;
  // SAFETY: the resource was just opened; its description is read.
  let said = unsafe { resource.GetDesc() };
  if !formats.contains(&said.Format) {
    return None;
  }
  Some((resource, said.Format, (u32::try_from(said.Width).ok()?, said.Height)))
}

/// Each surface of `frames` opened on `device`. `None` where one does not open or is not what it is said to be.
fn slots(device: &ID3D12Device, frames: &CardFrames) -> Option<Vec<Slot>> {
  frames
    .surfaces()
    .iter()
    .map(|surface| {
      Some(match *surface {
        CardSurface::Nv12(handle) => {
          let (first, _, size) = opened(device, handle, &[DXGI_FORMAT_NV12])?;
          Slot {
            first,
            second: None,
            kind: Kind::Nv12,
            size,
          }
        }
        CardSurface::Planes { gray, color } => {
          let (first, _, size) = opened(device, gray, &[DXGI_FORMAT_R8_UNORM])?;
          let (second, ..) = opened(device, color, &[DXGI_FORMAT_R8G8_UNORM])?;
          Slot {
            first,
            second: Some(second),
            kind: Kind::Planes,
            size,
          }
        }
        CardSurface::Rgba(handle) => {
          let (first, format, size) = opened(
            device,
            handle,
            &[DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_FORMAT_B8G8R8A8_UNORM],
          )?;
          Slot {
            first,
            second: None,
            kind: Kind::Rgba(format),
            size,
          }
        }
      })
    })
    .collect()
}

/// The fence behind `handle`, opened on `device`.
fn fence(device: &ID3D12Device, handle: isize) -> Option<ID3D12Fence> {
  let mut fence: Option<ID3D12Fence> = None;
  // SAFETY: the handle is one of a shared fence, kept open by the frames this is asked for.
  unsafe { device.OpenSharedHandle(HANDLE(handle as _), &mut fence) }.ok()?;
  fence
}

/// A texture of the renderer's own, `wide` by `high` and of `format`, made to be copied into.
fn own_texture(device: &ID3D12Device, format: DXGI_FORMAT, wide: u32, high: u32) -> Option<ID3D12Resource> {
  let heap = D3D12_HEAP_PROPERTIES {
    Type: D3D12_HEAP_TYPE_DEFAULT,
    CPUPageProperty: D3D12_CPU_PAGE_PROPERTY_UNKNOWN,
    MemoryPoolPreference: D3D12_MEMORY_POOL_UNKNOWN,
    CreationNodeMask: 1,
    VisibleNodeMask: 1,
  };
  let said = D3D12_RESOURCE_DESC {
    Dimension: D3D12_RESOURCE_DIMENSION_TEXTURE2D,
    Alignment: 0,
    Width: u64::from(wide),
    Height: high,
    DepthOrArraySize: 1,
    MipLevels: 1,
    Format: format,
    SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
    Layout: D3D12_TEXTURE_LAYOUT_UNKNOWN,
    Flags: D3D12_RESOURCE_FLAG_NONE,
  };
  let mut texture: Option<ID3D12Resource> = None;
  // SAFETY: a texture of the device's own is made.
  unsafe {
    device.CreateCommittedResource(
      &heap,
      D3D12_HEAP_FLAG_NONE,
      &said,
      D3D12_RESOURCE_STATE_COPY_DEST,
      None,
      &mut texture,
    )
  }
  .ok()?;
  texture
}

/// How many bytes a pixel of `format` is, of the formats a frame is read as.
fn bytes_of(format: DXGI_FORMAT) -> usize {
  match format {
    DXGI_FORMAT_R8_UNORM => 1,
    DXGI_FORMAT_R8G8_UNORM => 2,
    _ => 4,
  }
}

/// A part of a frame that came through memory, for the renderer to send to its texture: the gray, the color,
/// or the red, green and blue.
pub(super) struct Part {
  pub texture: ID3D12Resource,
  pub format: DXGI_FORMAT,
  pub wide: u32,
  pub high: u32,
  /// The rows of the part, one after another with nothing between them.
  pub bytes: Vec<u8>,
}

/// A frame that came through memory, for the renderer to send to its textures.
pub(super) struct Arrived {
  pub parts: Vec<Part>,
  /// Whether the textures hold a frame already: they are as they were made until the first one is sent.
  pub written: bool,
}

/// What is kept to draw an image whose frames are on another card: the frames as a device of that card reads
/// them, and the textures of the renderer's own a frame is sent to.
struct Copied {
  read: Read,
  /// The renderer's own textures, which hold the frame that is drawn once one was sent.
  slot: Slot,
  written: bool,
  /// The memory a frame is read into, kept from one frame to the next.
  kept: Vec<Vec<u8>>,
}

impl Copied {
  /// Opens each surface of `frames` on the card that opens them, and makes textures of `device` for the frame
  /// that is drawn. `None` where no card opens them, or a surface is not what it is said to be.
  fn open(device: &ID3D12Device, frames: &CardFrames, readers: &mut Readers) -> Option<Self> {
    let read = Read::open(frames, readers)?;
    let parts = read.parts();
    let &(format, wide, high) = parts.first()?;
    let first = own_texture(device, format, wide, high)?;
    let second = match parts.get(1) {
      Some(&(format, wide, high)) => Some(own_texture(device, format, wide, high)?),
      None => None,
    };
    let kind = if second.is_some() {
      Kind::Planes
    } else {
      Kind::Rgba(format)
    };
    Some(Self {
      read,
      slot: Slot {
        first,
        second,
        kind,
        size: (wide, high),
      },
      written: false,
      kept: Vec::new(),
    })
  }

  /// The frame that was being copied, once its card has copied it, and its parts as memory holds them.
  fn arrived(&mut self) -> Option<((u64, CardFrame), Arrived)> {
    let said: Vec<(DXGI_FORMAT, u32, u32)> = self.read.parts().to_vec();
    let mut kept = std::mem::take(&mut self.kept);
    kept.resize_with(said.len(), Vec::new);
    let frame = self.read.arrived(|part, rows, pitch| {
      let (Some(&(format, wide, high)), Some(bytes)) = (said.get(part), kept.get_mut(part)) else {
        return;
      };
      let row = wide as usize * bytes_of(format);
      bytes.clear();
      bytes.reserve(row * high as usize);
      for line in 0..high as usize {
        // SAFETY: the rows are as many as the part is high, each at least as long as a row of the part.
        bytes.extend_from_slice(unsafe { std::slice::from_raw_parts(rows.add(line * pitch as usize), row) });
      }
    });
    let Some(frame) = frame else {
      self.kept = kept;
      return None;
    };
    let textures = [Some(self.slot.first.clone()), self.slot.second.clone()];
    let parts = said
      .into_iter()
      .zip(kept)
      .zip(textures.into_iter().flatten())
      .map(|(((format, wide, high), bytes), texture)| Part {
        texture,
        format,
        wide,
        high,
        bytes,
      })
      .collect();
    Some((
      frame,
      Arrived {
        parts,
        written: self.written,
      },
    ))
  }
}

/// How the frames of an image come to the renderer.
enum Way {
  /// Each surface opened on the renderer's own device, and the fence that tells when a frame is written.
  Card {
    slots: Vec<Slot>,
    fence: Option<ID3D12Fence>,
  },
  /// The surfaces are another card's: each frame is copied through memory.
  Memory(Copied),
  /// No card opens them: nothing is drawn.
  Nowhere,
}

/// What a pick of a frame leaves the renderer to do.
#[derive(Default)]
pub(super) struct Picked {
  /// The lease of a frame that is no longer drawn, to be kept until the card has done the work that drew it.
  pub lease: Option<CardLease>,
  /// A frame that came through memory, to be sent to the renderer's textures.
  pub arrived: Option<Arrived>,
}

/// What is kept to draw an image whose frames are textures of a card: how they come to the renderer, and the
/// frame that is drawn now.
pub(super) struct Opened {
  way: Way,
  frames: CardFrames,
  /// The frame that is drawn, and which one it is of those that were shown.
  drawn: Option<(u64, CardFrame)>,
  /// The size of the picture.
  width: u32,
  height: u32,
}

impl Opened {
  /// Opens each surface of `frames`, whose picture is `width` by `height`. Where `device` does not open them,
  /// the card that does is found among `readers`, and each frame is copied through memory; where none does, or
  /// a surface is not what it is said to be, nothing is drawn of the frames. The frames are told which
  /// ([`CardFrames::way`]).
  pub(super) fn open(
    device: &ID3D12Device,
    frames: &CardFrames,
    (width, height): (u32, u32),
    readers: &mut Readers,
  ) -> Self {
    let on_card = slots(device, frames).and_then(|slots| {
      let fence = match frames.fence() {
        Some(handle) => Some(fence(device, handle)?),
        None => None,
      };
      Some(Way::Card { slots, fence })
    });
    let way = on_card
      .or_else(|| Copied::open(device, frames, readers).map(Way::Memory))
      .unwrap_or(Way::Nowhere);
    frames.found(match way {
      Way::Card { .. } => CardFramesWay::Card,
      Way::Memory(_) => CardFramesWay::Memory,
      Way::Nowhere => CardFramesWay::Nowhere,
    });
    Self {
      way,
      frames: frames.clone(),
      drawn: None,
      width,
      height,
    }
  }

  /// Whether this is `frames` opened, for a picture of this size.
  pub(super) fn is_of(&self, frames: &CardFrames, (width, height): (u32, u32)) -> bool {
    self.frames.same(frames) && (self.width, self.height) == (width, height)
  }

  /// Takes the frame that was shown last as the one to draw, once it is all written; the one drawn before it
  /// stays until then. On the card, the lease of a frame that is no longer drawn is given, for the renderer to
  /// keep until the card has done the work that drew it. Through memory, a frame that is written is copied on
  /// its card, and once that is done it is given as memory holds it, for the renderer to send to its textures
  /// ([`sent`](Self::sent)); its lease is let go of then, for the decoder's texture is drawn by nothing.
  pub(super) fn pick(&mut self) -> Picked {
    let mut picked = Picked::default();
    if let Way::Memory(copied) = &mut self.way {
      if let Some(((count, mut frame), arrived)) = copied.arrived() {
        drop(frame.lease.take());
        self.drawn = Some((count, frame));
        picked.arrived = Some(arrived);
      }
      if copied.read.is_copying() {
        return picked;
      }
    }
    let Some((count, frame)) = self.frames.latest() else {
      return picked;
    };
    if self.drawn.as_ref().is_some_and(|(drawn, _)| *drawn == count) || frame.surface >= self.frames.surfaces().len() {
      return picked;
    }
    let reached = match &self.way {
      // SAFETY: the fence is read.
      Way::Card { fence, .. } => fence.as_ref().map(|fence| unsafe { fence.GetCompletedValue() }),
      Way::Memory(copied) => copied.read.reached(),
      Way::Nowhere => return picked,
    };
    let written = match (frame.ready, reached) {
      (Some(value), Some(reached)) => reached >= value,
      _ => true,
    };
    if !written {
      return picked;
    }
    match &mut self.way {
      Way::Card { .. } => {
        picked.lease = self.drawn.replace((count, frame)).and_then(|(_, before)| before.lease);
      }
      Way::Memory(copied) => copied.read.copy(count, frame),
      Way::Nowhere => {}
    }
    picked
  }

  /// The renderer sent a frame that came through memory to its textures, or could not (`written`): its memory
  /// is kept for the next one.
  pub(super) fn sent(&mut self, arrived: Arrived, written: bool) {
    if let Way::Memory(copied) = &mut self.way {
      copied.written |= written;
      copied.kept = arrived.parts.into_iter().map(|part| part.bytes).collect();
    }
  }

  /// Whether the textures that are drawn are another device's, shared with this one: such are read for a draw
  /// and given back after it. Through memory they are the renderer's own.
  pub(super) fn is_shared(&self) -> bool {
    matches!(self.way, Way::Card { .. })
  }

  /// The surface that is drawn now, where a frame was taken. Through memory there is one, which holds the
  /// frame that was read last, once one was sent to it.
  pub(super) fn drawn(&self) -> Option<&Slot> {
    let (_, frame) = self.drawn.as_ref()?;
    match &self.way {
      Way::Card { slots, .. } => slots.get(frame.surface),
      Way::Memory(copied) => copied.written.then_some(&copied.slot),
      Way::Nowhere => None,
    }
  }

  /// Where in the drawn surface the part `min` to `max` of the picture is, both as parts of one: the picture
  /// may be a part of its texture.
  pub(super) fn part(&self, min: [f32; 2], max: [f32; 2]) -> ([f32; 2], [f32; 2]) {
    let Some(((_, frame), slot)) = self.drawn.as_ref().zip(self.drawn()) else {
      return (min, max);
    };
    crate::images::part_of(
      frame.crop,
      (self.width, self.height),
      slot.size,
      self.frames.is_rgba(),
      min,
      max,
    )
  }

  /// How the drawn frame's gray and color are red, green and blue, as its decoder said: BT.709 of the video
  /// range before one is drawn.
  pub(super) fn colors(&self) -> crate::images::YuvColors {
    self
      .drawn
      .as_ref()
      .map_or(crate::images::YuvColors::BT709, |(_, frame)| frame.colors)
  }

  /// Ends the drawing of the frames: gives the lease of the frame that was drawn, which the card may still be
  /// drawing.
  pub(super) fn closed(mut self) -> Option<CardLease> {
    self.drawn.take().and_then(|(_, frame)| frame.lease)
  }
}

#[cfg(test)]
mod tests {
  use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
  };

  use windows::{
    Win32::Graphics::{
      Direct3D::D3D_FEATURE_LEVEL_11_0,
      Direct3D12::{
        D3D12_FENCE_FLAG_SHARED, D3D12_HEAP_FLAG_SHARED, D3D12_RESOURCE_FLAG_ALLOW_SIMULTANEOUS_ACCESS,
        D3D12_RESOURCE_STATE_COMMON, D3D12CreateDevice, D3D12GetDebugInterface, ID3D12Debug,
      },
    },
    core::PCWSTR,
  };

  use super::*;
  use crate::images::CardCrop;

  /// A device of the card the system takes first.
  ///
  /// Direct3D's debug layer is turned on before the first one is made, where the system has it. wgpu turns it on
  /// when it starts on Direct3D 12 in a build like this one, and turning it on removes every device that was
  /// made without it (`ID3D12Debug::EnableDebugLayer` says so): a device of these tests made before a test of
  /// wgpu's started would be removed under it.
  pub(super) fn device() -> ID3D12Device {
    static DEBUG_LAYER: std::sync::Once = std::sync::Once::new();
    DEBUG_LAYER.call_once(|| {
      let mut debug: Option<ID3D12Debug> = None;
      // SAFETY: the debug layer is asked for, and turned on where the system has it, before a device is made.
      unsafe {
        if D3D12GetDebugInterface(&mut debug).is_ok()
          && let Some(debug) = debug
        {
          debug.EnableDebugLayer();
        }
      }
    });
    let mut device: Option<ID3D12Device> = None;
    // SAFETY: a device is asked of the system, of no card in particular.
    unsafe { D3D12CreateDevice(None, D3D_FEATURE_LEVEL_11_0, &mut device) }.expect("a Direct3D 12 device");
    device.expect("a Direct3D 12 device")
  }

  /// A texture of `format` made to be shared, by its handle. The handle keeps it.
  fn shared(device: &ID3D12Device, wide: u32, high: u32, format: DXGI_FORMAT) -> isize {
    const GENERIC_ALL: u32 = 0x1000_0000;
    let heap = D3D12_HEAP_PROPERTIES {
      Type: D3D12_HEAP_TYPE_DEFAULT,
      CPUPageProperty: D3D12_CPU_PAGE_PROPERTY_UNKNOWN,
      MemoryPoolPreference: D3D12_MEMORY_POOL_UNKNOWN,
      CreationNodeMask: 1,
      VisibleNodeMask: 1,
    };
    let said = D3D12_RESOURCE_DESC {
      Dimension: D3D12_RESOURCE_DIMENSION_TEXTURE2D,
      Alignment: 0,
      Width: u64::from(wide),
      Height: high,
      DepthOrArraySize: 1,
      MipLevels: 1,
      Format: format,
      SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
      Layout: D3D12_TEXTURE_LAYOUT_UNKNOWN,
      Flags: D3D12_RESOURCE_FLAG_ALLOW_SIMULTANEOUS_ACCESS,
    };
    let mut texture: Option<ID3D12Resource> = None;
    // SAFETY: a texture of a size and a format the card has is made, and a handle to it.
    unsafe {
      device
        .CreateCommittedResource(
          &heap,
          D3D12_HEAP_FLAG_SHARED,
          &said,
          D3D12_RESOURCE_STATE_COMMON,
          None,
          &mut texture,
        )
        .expect("a texture");
      let texture = texture.expect("a texture");
      device
        .CreateSharedHandle(&texture, None, GENERIC_ALL, PCWSTR::null())
        .expect("a handle of the texture")
        .0 as isize
    }
  }

  /// A fence made to be shared, and its handle.
  fn shared_fence(device: &ID3D12Device) -> (ID3D12Fence, isize) {
    const GENERIC_ALL: u32 = 0x1000_0000;
    // SAFETY: a fence is made on the device, and a handle to it.
    unsafe {
      let fence: ID3D12Fence = device.CreateFence(0, D3D12_FENCE_FLAG_SHARED).expect("a fence");
      let handle = device
        .CreateSharedHandle(&fence, None, GENERIC_ALL, PCWSTR::null())
        .expect("a handle of the fence");
      (fence, handle.0 as isize)
    }
  }

  /// What says it was let go of.
  struct Lease(Arc<AtomicBool>);

  impl Drop for Lease {
    fn drop(&mut self) {
      self.0.store(true, Ordering::Release);
    }
  }

  fn lease() -> (Lease, Arc<AtomicBool>) {
    let gone = Arc::new(AtomicBool::new(false));
    (Lease(gone.clone()), gone)
  }

  /// Whether a pick left the renderer nothing to do.
  fn nothing(picked: Picked) -> bool {
    picked.lease.is_none() && picked.arrived.is_none()
  }

  /// Whether two parts of a texture are the same, but for what the arithmetic rounds.
  fn near(part: ([f32; 2], [f32; 2]), other: ([f32; 2], [f32; 2])) -> bool {
    let close = |a: [f32; 2], b: [f32; 2]| (a[0] - b[0]).abs() < 1e-5 && (a[1] - b[1]).abs() < 1e-5;
    close(part.0, other.0) && close(part.1, other.1)
  }

  #[test]
  fn the_frame_drawn_is_the_newest_one_written_and_the_lease_of_the_one_before_is_given() {
    let _alone = crate::app::card_read::ON_THE_CARD.lock();
    let device = device();
    let planes = || CardSurface::Planes {
      gray: shared(&device, 64, 48, DXGI_FORMAT_R8_UNORM),
      color: shared(&device, 32, 24, DXGI_FORMAT_R8G8_UNORM),
    };
    let (fence, fence_handle) = shared_fence(&device);
    // SAFETY: the handles are of textures and of a fence made to be shared, given away.
    let frames = unsafe { CardFrames::with_fence(vec![planes(), planes()], fence_handle) };
    assert_eq!(frames.way(), None);
    let mut opened = Opened::open(&device, &frames, (64, 48), &mut Readers::default());
    assert_eq!(frames.way(), Some(CardFramesWay::Card));
    assert!(opened.is_of(&frames, (64, 48)) && !opened.is_of(&frames, (64, 40)));

    // Nothing was shown: nothing is drawn.
    assert!(nothing(opened.pick()) && opened.drawn().is_none());

    // A frame that is not all written is not drawn, and nothing waits for it.
    let (first, first_gone) = lease();
    frames.show(CardFrame::of(0).ready_at(1).lease(first));
    assert!(nothing(opened.pick()) && opened.drawn().is_none());
    // SAFETY: the fence is the test's own.
    unsafe { fence.Signal(1) }.expect("the fence is moved");
    assert!(nothing(opened.pick()), "no frame was drawn before the first");
    let slot = opened.drawn().expect("the first frame is drawn");
    assert_eq!(
      (slot.kind, slot.size, slot.second.is_some()),
      (Kind::Planes, (64, 48), true)
    );

    // The next one takes its place once it is written, and not before; the lease of the first is given then, and
    // is let go of by who was given it.
    let (second, second_gone) = lease();
    frames.show(CardFrame::of(1).ready_at(2).lease(second));
    assert!(nothing(opened.pick()));
    assert!(!first_gone.load(Ordering::Acquire));
    // SAFETY: as above.
    unsafe { fence.Signal(2) }.expect("the fence is moved");
    let before = opened
      .pick()
      .lease
      .expect("the lease of the frame that is drawn no more");
    assert!(
      !first_gone.load(Ordering::Acquire),
      "the card may still draw the first frame"
    );
    drop(before);
    assert!(first_gone.load(Ordering::Acquire));
    // The same frame again is taken once.
    assert!(nothing(opened.pick()));

    // When the image is drawn no more, the lease of its last frame is given too.
    let last = opened.closed().expect("the lease of the last frame");
    assert!(!second_gone.load(Ordering::Acquire));
    drop(last);
    drop(frames);
    assert!(second_gone.load(Ordering::Acquire));
  }

  #[test]
  fn frames_that_are_not_what_they_say_or_that_the_device_does_not_open_are_drawn_nowhere() {
    let _alone = crate::app::card_read::ON_THE_CARD.lock();
    let device = device();
    // A texture of gray said to be one of red, green and blue.
    // SAFETY: the handle is of a texture made to be shared, given away.
    let frames = unsafe { CardFrames::new(vec![CardSurface::Rgba(shared(&device, 64, 48, DXGI_FORMAT_R8_UNORM))]) };
    let mut opened = Opened::open(&device, &frames, (64, 48), &mut Readers::default());
    assert_eq!(frames.way(), Some(CardFramesWay::Nowhere));
    frames.show(CardFrame::of(0));
    assert!(nothing(opened.pick()) && opened.drawn().is_none());
    assert_eq!(opened.part([0.0, 0.0], [1.0, 1.0]), ([0.0, 0.0], [1.0, 1.0]));
  }

  #[test]
  fn a_texture_of_red_green_and_blue_is_drawn_as_it_is_and_the_picture_is_the_part_it_says() {
    let _alone = crate::app::card_read::ON_THE_CARD.lock();
    let device = device();
    // SAFETY: the handle is of a texture made to be shared, given away.
    let frames = unsafe {
      CardFrames::new(vec![CardSurface::Rgba(shared(
        &device,
        64,
        48,
        DXGI_FORMAT_B8G8R8A8_UNORM,
      ))])
    };
    // The picture is smaller than its texture: the part at the top left corner, kept half a pixel from the
    // edges inside the texture.
    let mut opened = Opened::open(&device, &frames, (60, 40), &mut Readers::default());
    assert_eq!(frames.way(), Some(CardFramesWay::Card));
    frames.show(CardFrame::of(0));
    assert!(nothing(opened.pick()));
    assert!(opened.is_shared());
    let slot = opened.drawn().expect("the frame is drawn");
    assert_eq!(
      (slot.kind, slot.size, slot.second.is_some()),
      (Kind::Rgba(DXGI_FORMAT_B8G8R8A8_UNORM), (64, 48), false)
    );
    assert!(near(
      opened.part([0.0, 0.0], [1.0, 1.0]),
      ([0.0, 0.0], [59.5 / 64.0, 39.5 / 48.0])
    ));
    // And the part a frame says.
    frames.show(CardFrame::of(0).crop(CardCrop {
      left: 4,
      top: 8,
      width: 32,
      height: 16,
    }));
    assert!(nothing(opened.pick()));
    assert!(near(
      opened.part([0.0, 0.0], [1.0, 1.0]),
      ([4.5 / 64.0, 8.5 / 48.0], [35.5 / 64.0, 23.5 / 48.0])
    ));
    // A part of the picture is that part of it.
    assert!(near(
      opened.part([0.5, 0.0], [1.0, 0.5]),
      ([20.0 / 64.0, 8.5 / 48.0], [35.5 / 64.0, 16.0 / 48.0])
    ));
  }
}

/// The frames of a decoder on another card than the renderer's: a Direct3D 11 device of the system's software
/// card stands for the decoder.
#[cfg(test)]
mod other_card_tests {
  use std::time::{Duration, Instant};

  use windows::{
    Win32::{
      Foundation::HMODULE,
      Graphics::{
        Direct3D::{D3D_DRIVER_TYPE_WARP, D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_11_1},
        Direct3D11::{
          D3D11_BIND_SHADER_RESOURCE, D3D11_CREATE_DEVICE_FLAG, D3D11_RESOURCE_MISC_SHARED,
          D3D11_RESOURCE_MISC_SHARED_NTHANDLE, D3D11_SDK_VERSION, D3D11_SUBRESOURCE_DATA, D3D11_TEXTURE2D_DESC,
          D3D11_USAGE_DEFAULT, D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D,
        },
        Dxgi::{DXGI_SHARED_RESOURCE_READ, IDXGIResource1},
      },
    },
    core::{Interface, PCWSTR},
  };

  use super::*;

  /// A texture of the software card, `wide` by `high`, holding `bytes`, made to be shared: its handle, and the
  /// texture, which is kept while the handle is used.
  fn written(
    device: &ID3D11Device,
    format: DXGI_FORMAT,
    wide: u32,
    high: u32,
    bytes: &[u8],
  ) -> (isize, ID3D11Texture2D) {
    let said = D3D11_TEXTURE2D_DESC {
      Width: wide,
      Height: high,
      MipLevels: 1,
      ArraySize: 1,
      Format: format,
      SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
      Usage: D3D11_USAGE_DEFAULT,
      BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
      CPUAccessFlags: 0,
      MiscFlags: (D3D11_RESOURCE_MISC_SHARED_NTHANDLE.0 | D3D11_RESOURCE_MISC_SHARED.0) as u32,
    };
    let holds = D3D11_SUBRESOURCE_DATA {
      pSysMem: bytes.as_ptr().cast(),
      SysMemPitch: (bytes.len() / high as usize) as u32,
      SysMemSlicePitch: 0,
    };
    let mut texture: Option<ID3D11Texture2D> = None;
    // SAFETY: a texture is made holding `bytes`, which are as many rows as it is high, and a handle to it.
    unsafe {
      device
        .CreateTexture2D(&said, Some(&holds), Some(&mut texture))
        .expect("a texture of the software card");
      let texture = texture.expect("a texture of the software card");
      let handle = texture
        .cast::<IDXGIResource1>()
        .expect("a texture is shared")
        .CreateSharedHandle(None, DXGI_SHARED_RESOURCE_READ.0, PCWSTR::null())
        .expect("a handle of the texture");
      (handle.0 as isize, texture)
    }
  }

  #[test]
  fn the_frames_of_a_decoder_on_another_card_come_through_memory_as_they_were_written() {
    let _alone = crate::app::card_read::ON_THE_CARD.lock();
    // The renderer's device is of the card the system takes first; the decoder's is the software card's.
    let device = super::tests::device();
    let (mut decoder, mut context): (Option<ID3D11Device>, Option<ID3D11DeviceContext>) = (None, None);
    // SAFETY: a device of the software card is asked of the system.
    unsafe {
      D3D11CreateDevice(
        None,
        D3D_DRIVER_TYPE_WARP,
        HMODULE::default(),
        D3D11_CREATE_DEVICE_FLAG(0),
        Some(&[D3D_FEATURE_LEVEL_11_1, D3D_FEATURE_LEVEL_11_0]),
        D3D11_SDK_VERSION,
        Some(&mut decoder),
        None,
        Some(&mut context),
      )
    }
    .expect("a device of the software card");
    let (decoder, context) = (decoder.expect("the device"), context.expect("its context"));

    // A frame of gray and of color, each sample its own.
    const WIDE: u32 = 64;
    const HIGH: u32 = 48;
    let gray: Vec<u8> = (0..WIDE * HIGH).map(|at| (at % 251) as u8).collect();
    let color: Vec<u8> = (0..WIDE * HIGH / 2).map(|at| (at % 241) as u8).collect();
    let (gray_handle, _gray) = written(&decoder, DXGI_FORMAT_R8_UNORM, WIDE, HIGH, &gray);
    let (color_handle, _color) = written(&decoder, DXGI_FORMAT_R8G8_UNORM, WIDE / 2, HIGH / 2, &color);
    // SAFETY: the software card is told to do what it was given.
    unsafe { context.Flush() };
    // SAFETY: the handles are of textures made to be shared, given away.
    let frames = unsafe {
      CardFrames::new(vec![CardSurface::Planes {
        gray: gray_handle,
        color: color_handle,
      }])
    };
    let mut readers = Readers::default();
    let mut opened = Opened::open(&device, &frames, (WIDE, HIGH), &mut readers);
    if frames.way() == Some(CardFramesWay::Card) {
      // The system's first card is its software card: there is no other card here to read from.
      eprintln!("the renderer's card opens the software card's textures: nothing comes through memory here");
      return;
    }
    assert_eq!(frames.way(), Some(CardFramesWay::Memory));
    assert!(!opened.is_shared());

    frames.show(CardFrame::of(0));
    // The frame is copied on its card, and comes when that is done: nothing waits for it, and nothing is drawn
    // until it was sent to the renderer's textures.
    let started = Instant::now();
    let arrived = loop {
      let picked = opened.pick();
      assert!(picked.lease.is_none());
      if let Some(arrived) = picked.arrived {
        break arrived;
      }
      assert!(opened.drawn().is_none());
      assert!(
        started.elapsed() < Duration::from_secs(5),
        "the frame never comes through memory"
      );
      std::thread::sleep(Duration::from_millis(2));
    };
    assert!(!arrived.written, "the renderer's textures hold no frame yet");
    assert_eq!(arrived.parts.len(), 2);
    let (first, second) = (&arrived.parts[0], &arrived.parts[1]);
    assert_eq!(
      (first.format, first.wide, first.high),
      (DXGI_FORMAT_R8_UNORM, WIDE, HIGH)
    );
    assert_eq!(
      (second.format, second.wide, second.high),
      (DXGI_FORMAT_R8G8_UNORM, WIDE / 2, HIGH / 2)
    );
    assert!(first.bytes == gray, "the gray is the gray that was written");
    assert!(second.bytes == color, "the color is the color that was written");
    assert!(
      opened.drawn().is_none(),
      "nothing is drawn of textures nothing was sent to"
    );

    // Once the renderer sent it, the frame is drawn from the renderer's own textures.
    opened.sent(arrived, true);
    let slot = opened.drawn().expect("the frame is drawn");
    assert_eq!(
      (slot.kind, slot.size, slot.second.is_some()),
      (Kind::Planes, (WIDE, HIGH), true)
    );

    // The next frame comes the same way, into the memory the first was read into, and says the textures hold a
    // frame already.
    frames.show(CardFrame::of(0));
    let started = Instant::now();
    let arrived = loop {
      if let Some(arrived) = opened.pick().arrived {
        break arrived;
      }
      assert!(
        started.elapsed() < Duration::from_secs(5),
        "the second frame never comes through memory"
      );
      std::thread::sleep(Duration::from_millis(2));
    };
    assert!(arrived.written);
    assert!(arrived.parts[0].bytes == gray && arrived.parts[1].bytes == color);
  }
}
