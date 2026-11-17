//! What the GPU Viewport page shows: a picture the app makes on the GPU itself, as an app with a renderer of its
//! own does, and gives to lurq through a slot. It is a frame extension of the gallery's wgpu renderer
//! (`WgpuFrameExtension`): before each frame it finds where the page's viewport is, makes a texture of that size
//! with the device lurq draws with, writes rings into it and gives its view to the slot, which lurq then draws
//! where the viewport is, with no copy through the CPU.
//!
//! The page says which colors and whether the rings turn; nothing is made while no viewport of the slot is shown.

use std::sync::{
  OnceLock,
  atomic::{AtomicBool, AtomicUsize, Ordering},
};

use lurq::{
  app::wgpu_render::{SharedWgpuContext, WgpuFrameExtension, WgpuFrameInfo},
  images::WgpuExternalImageSlot,
};

/// The largest side of the texture: a viewport larger than this is drawn from a picture this big, scaled. The
/// picture is made on the CPU at half the viewport's size, which is quick enough for a frame of a debug build.
const LARGEST: u32 = 640;

/// The slot the page's viewport shows and the extension writes to: one for the whole gallery.
pub fn slot() -> &'static WgpuExternalImageSlot {
  static SLOT: OnceLock<WgpuExternalImageSlot> = OnceLock::new();
  SLOT.get_or_init(WgpuExternalImageSlot::new)
}

/// Which of [`COLORS`] the rings are in: the page sets it.
pub static COLORS_AT: AtomicUsize = AtomicUsize::new(0);
/// Whether the rings turn, a step a frame, with frames asked for while they do: the page sets it, and puts it
/// back when it goes.
pub static TURNING: AtomicBool = AtomicBool::new(false);

/// The pairs of colors of the rings, inner and outer, as linear RGB.
pub const COLORS: [(&str, [f32; 3], [f32; 3]); 3] = [
  ("Ocean", [0.02, 0.25, 0.65], [0.0, 0.75, 0.7]),
  ("Ember", [0.7, 0.08, 0.02], [0.98, 0.65, 0.05]),
  ("Moss", [0.05, 0.3, 0.08], [0.6, 0.85, 0.2]),
];

/// The extension: the texture it made last and what it drew into it.
#[derive(Default)]
pub struct Scene {
  texture: Option<(wgpu::Texture, u32, u32)>,
  /// The colors and the step of the picture in the texture, to draw only what changed.
  drawn: Option<(usize, u32)>,
  step: u32,
}

impl Scene {
  /// The rings at `step`, `width` by `height`, in the colors `at`: RGBA, a row after another.
  fn picture(width: u32, height: u32, at: usize, step: u32) -> Vec<u8> {
    let (_, inner, outer) = COLORS[at % COLORS.len()];
    let (cx, cy) = (width as f32 / 2.0, height as f32 / 2.0);
    let reach = cx.hypot(cy).max(1.0);
    let phase = step as f32 * 0.05;
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
      for x in 0..width {
        let (dx, dy) = (x as f32 - cx, y as f32 - cy);
        let distance = dx.hypot(dy) / reach;
        let angle = dy.atan2(dx);
        let wave = 0.5 + 0.5 * (distance * 28.0 - phase * 4.0 + (angle * 3.0 + phase).sin() * 0.8).sin();
        let mix = (distance * 0.7 + wave * 0.3).clamp(0.0, 1.0);
        for channel in 0..3 {
          let value = inner[channel] + (outer[channel] - inner[channel]) * mix;
          let lit = value * (0.55 + 0.45 * wave);
          pixels.push((lit.clamp(0.0, 1.0).powf(1.0 / 2.2) * 255.0) as u8);
        }
        pixels.push(255);
      }
    }
    pixels
  }
}

impl WgpuFrameExtension for Scene {
  fn prepare(&mut self, gpu: &SharedWgpuContext, frame: WgpuFrameInfo<'_>) {
    let Some(rect) = frame.viewport(slot().image_id()) else {
      return;
    };
    let width = ((rect.width / 2.0).round() as u32).clamp(1, LARGEST);
    let height = ((rect.height / 2.0).round() as u32).clamp(1, LARGEST);
    let turning = TURNING.load(Ordering::Relaxed);
    if turning {
      self.step = self.step.wrapping_add(1);
    }
    let at = COLORS_AT.load(Ordering::Relaxed);
    let made = self
      .texture
      .as_ref()
      .is_some_and(|(_, w, h)| (*w, *h) == (width, height));
    if made && self.drawn == Some((at, self.step)) {
      return;
    }
    if !made {
      let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("gallery GPU viewport scene"),
        size: wgpu::Extent3d {
          width,
          height,
          depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
      });
      self.texture = Some((texture, width, height));
    }
    let Some((texture, ..)) = &self.texture else {
      return;
    };
    gpu.queue.write_texture(
      wgpu::TexelCopyTextureInfo {
        texture,
        mip_level: 0,
        origin: wgpu::Origin3d::ZERO,
        aspect: wgpu::TextureAspect::All,
      },
      &Self::picture(width, height, at, self.step),
      wgpu::TexelCopyBufferLayout {
        offset: 0,
        bytes_per_row: Some(width * 4),
        rows_per_image: Some(height),
      },
      wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
      },
    );
    if !made {
      slot().set_texture_view(
        texture.create_view(&wgpu::TextureViewDescriptor::default()),
        width,
        height,
      );
    }
    self.drawn = Some((at, self.step));
  }

  fn wants_redraw(&self) -> bool {
    TURNING.load(Ordering::Relaxed)
  }

  fn device_lost(&mut self) {
    self.texture = None;
    self.drawn = None;
  }
}
