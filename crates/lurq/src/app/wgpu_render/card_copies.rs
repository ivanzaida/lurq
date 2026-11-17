//! The frames of a video whose decoder is on another card than the one that draws.
//!
//! A texture shared by handle is opened by a device of the card it is on, and by no other: on a machine with two
//! cards the renderer's device may be given handles it cannot open. They are opened by a Direct3D 11 device of
//! the card that does open them, and each frame is copied on that card into a texture that memory reads
//! ([`card_read`](crate::app::card_read)); here it is sent from memory into textures of the renderer's own
//! ([`Copied`]). The frame is shown, at the price of a copy through memory, where it would not be shown at all.
//!
//! Nothing waits here either: a frame is copied once its fence says it is written, and read once the card it is
//! on has copied it, which is asked without waiting in each of the renderer's frames; the frame before it is
//! drawn until then.

use windows::Win32::Graphics::Dxgi::Common::{
  DXGI_FORMAT, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_FORMAT_R8_UNORM, DXGI_FORMAT_R8G8_UNORM, DXGI_FORMAT_R8G8B8A8_UNORM,
};

use super::card_frames::Slot;
pub(super) use crate::app::card_read::Readers;
use crate::{
  app::card_read::Read,
  images::{CardFrame, CardFrames},
};

/// The format of wgpu's that a part of a frame read from another card is kept in.
fn kept_as(format: DXGI_FORMAT) -> Option<wgpu::TextureFormat> {
  Some(match format {
    DXGI_FORMAT_R8_UNORM => wgpu::TextureFormat::R8Unorm,
    DXGI_FORMAT_R8G8_UNORM => wgpu::TextureFormat::Rg8Unorm,
    DXGI_FORMAT_R8G8B8A8_UNORM => wgpu::TextureFormat::Rgba8Unorm,
    DXGI_FORMAT_B8G8R8A8_UNORM => wgpu::TextureFormat::Bgra8Unorm,
    _ => return None,
  })
}

/// What is kept to draw an image whose frames are on another card: the frames as a device of that card reads
/// them, and the textures of the renderer's own a frame is sent to.
pub(super) struct Copied {
  read: Read,
  /// The renderer's own textures, which hold the frame that is drawn.
  slot: Slot,
}

impl Copied {
  /// Opens each surface of `frames` on the card that opens them, and makes textures of `device` for the frame
  /// that is drawn, bound with `bind`. `None` where no card opens them, or a surface is not what it is said to
  /// be or not as the first one is.
  pub(super) fn open(
    device: &wgpu::Device,
    frames: &CardFrames,
    readers: &mut Readers,
    bind: &impl Fn(&wgpu::TextureView, Option<&wgpu::TextureView>) -> wgpu::BindGroup,
  ) -> Option<Self> {
    let read = Read::open(frames, readers)?;
    let (_, wide, high) = *read.parts().first()?;
    let textures = read
      .parts()
      .iter()
      .map(|&(format, width, height)| {
        Some(device.create_texture(&wgpu::TextureDescriptor {
          label: Some("lurq_card_frame_copied"),
          size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
          },
          mip_level_count: 1,
          sample_count: 1,
          dimension: wgpu::TextureDimension::D2,
          format: kept_as(format)?,
          usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::COPY_SRC,
          view_formats: &[],
        }))
      })
      .collect::<Option<Vec<wgpu::Texture>>>()?;
    let first_view = textures[0].create_view(&Default::default());
    let second_view = textures.get(1).map(|texture| texture.create_view(&Default::default()));
    Some(Self {
      read,
      slot: Slot {
        bind_group: bind(&first_view, second_view.as_ref()),
        first: first_view,
        second: second_view,
        size: (wide, high),
        textures,
      },
    })
  }

  /// The renderer's textures, which hold the frame that was read last.
  pub(super) fn slot(&self) -> &Slot {
    &self.slot
  }

  /// The value the decoder's fence has reached, where it has one.
  pub(super) fn reached(&self) -> Option<u64> {
    self.read.reached()
  }

  /// Whether a frame is being copied: another is not begun until it is read.
  pub(super) fn is_copying(&self) -> bool {
    self.read.is_copying()
  }

  /// Begins to copy `frame`, the one numbered `count` of those that were shown, on the card it is on. The frame
  /// is kept, and with it its lease, until it is read.
  pub(super) fn copy(&mut self, count: u64, frame: CardFrame) {
    self.read.copy(count, frame);
  }

  /// The frame that was being copied, once its card has copied it: it is sent to the renderer's textures, and is
  /// the one to draw. `None` while the card has not finished, for which nothing waits, and where no frame is
  /// being copied.
  pub(super) fn arrived(&mut self, queue: &wgpu::Queue) -> Option<(u64, CardFrame)> {
    let textures = &self.slot.textures;
    self.read.arrived(|part, rows, pitch| {
      if let Some(texture) = textures.get(part) {
        // SAFETY: the rows are of a part as large as this texture, which was made as the part is.
        unsafe { send(queue, texture, rows, pitch) };
      }
    })
  }
}

/// Sends the rows at `rows`, each `pitch` bytes after the one before it, to all of `texture`.
///
/// # Safety
///
/// `rows` is as many rows as the texture is high, each of them at least as long as a row of the texture.
unsafe fn send(queue: &wgpu::Queue, texture: &wgpu::Texture, rows: *const u8, pitch: u32) {
  let (wide, high) = (texture.width(), texture.height());
  let row = wide * texture.format().block_copy_size(None).unwrap_or(4);
  let bytes = pitch as usize * (high as usize - 1) + row as usize;
  // SAFETY: as the function says.
  let bytes = unsafe { std::slice::from_raw_parts(rows, bytes) };
  queue.write_texture(
    texture.as_image_copy(),
    bytes,
    wgpu::TexelCopyBufferLayout {
      offset: 0,
      bytes_per_row: Some(pitch),
      rows_per_image: None,
    },
    texture.size(),
  );
}
