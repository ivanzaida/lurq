//! The frames of a video that stay on the graphics card, as this renderer draws them on Direct3D 12.
//!
//! What decodes a video copies each frame, on the card, into one of a few NV12 textures that are shared by
//! handle ([`SharedNv12Frames`](crate::images::SharedNv12Frames)). Here each handle is opened once on the
//! renderer's own device and made a texture of wgpu's, whose two planes (the gray, and the color at half the
//! size) are what the NV12 pipeline samples, as it samples the two textures a frame from memory is sent to. A
//! frame is drawn from where the decoder's copy put it: nothing of it is read or sent by the program.

use windows::Win32::{Foundation::HANDLE, Graphics::Direct3D12::ID3D12Resource};

use super::CachedImageTexture;

/// One of the textures of a video's frames, opened: its planes, and what binds them for the pipeline.
pub(super) struct Slot {
  pub bind_group: wgpu::BindGroup,
  pub y_view: wgpu::TextureView,
  pub uv_view: wgpu::TextureView,
  _texture: wgpu::Texture,
}

/// Says that `device` draws shared frames, and on which card, where it can: on Direct3D 12 with NV12 textures.
/// What decodes a video asks ([`shared_nv12_adapter`](crate::images::shared_nv12_adapter)) and decodes on that
/// card; where nothing said so, a frame goes through memory, as before.
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
  crate::images::set_shared_nv12_adapter(Some((card.LowPart, card.HighPart)));
}

/// The texture behind `handle`, `width` by `height`, opened on `device`. `None` where the device is not one of
/// Direct3D 12, or does not open it (another card).
pub(super) fn open(device: &wgpu::Device, handle: isize, width: u32, height: u32) -> Option<wgpu::Texture> {
  let size = wgpu::Extent3d {
    width,
    height,
    depth_or_array_layers: 1,
  };
  let texture = {
    // SAFETY: the device's own handle is used to open a texture, which changes nothing of the device.
    let hal = unsafe { device.as_hal::<wgpu::hal::api::Dx12>() }?;
    let mut resource: Option<ID3D12Resource> = None;
    // SAFETY: the handle is one of a shared texture, kept open by the frames this is asked for.
    unsafe { hal.raw_device().OpenSharedHandle(HANDLE(handle as _), &mut resource) }.ok()?;
    // SAFETY: the resource is an NV12 texture of this size with one level and one sample, as it is said to be.
    unsafe {
      wgpu::hal::dx12::Device::texture_from_raw(
        resource?,
        wgpu::TextureFormat::NV12,
        wgpu::TextureDimension::D2,
        size,
        1,
        1,
      )
    }
  };
  // SAFETY: the texture was opened on this device, is as the descriptor says, and holds a frame or the black it
  // was made with: nothing reads what was never written.
  Some(unsafe {
    device.create_texture_from_hal::<wgpu::hal::api::Dx12>(
      texture,
      &wgpu::TextureDescriptor {
        label: Some("lurq_shared_nv12"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::NV12,
        usage: wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
      },
    )
  })
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

/// What is kept to draw `image`, whose frames stay on the card: each of its textures opened. `None` where one
/// does not open, and nothing is drawn of it.
pub(super) fn cached(
  device: &wgpu::Device,
  nv12_image_bgl: &wgpu::BindGroupLayout,
  image_sampler: &wgpu::Sampler,
  globals_buffer: &wgpu::Buffer,
  image: &crate::images::ImageCmd,
) -> Option<CachedImageTexture> {
  let frames = image
    .native
    .as_ref()?
    .payload::<crate::images::SharedNv12Frames>()?
    .clone();
  let slots = frames
    .handles()
    .iter()
    .map(|handle| {
      let texture = open(device, *handle, image.image_width, image.image_height)?;
      let (y_view, uv_view) = planes(&texture);
      let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("lurq_shared_nv12_img_bg"),
        layout: nv12_image_bgl,
        entries: &[
          wgpu::BindGroupEntry {
            binding: 0,
            resource: globals_buffer.as_entire_binding(),
          },
          wgpu::BindGroupEntry {
            binding: 1,
            resource: wgpu::BindingResource::TextureView(&y_view),
          },
          wgpu::BindGroupEntry {
            binding: 2,
            resource: wgpu::BindingResource::TextureView(&uv_view),
          },
          wgpu::BindGroupEntry {
            binding: 3,
            resource: wgpu::BindingResource::Sampler(image_sampler),
          },
        ],
      });
      Some(Slot {
        bind_group,
        y_view,
        uv_view,
        _texture: texture,
      })
    })
    .collect::<Option<Vec<_>>>()?;
  Some(CachedImageTexture::SharedNv12 {
    slots,
    frames,
    width: image.image_width,
    height: image.image_height,
  })
}
