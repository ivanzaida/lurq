// A frame that stays on the graphics card, end to end: a real clip is decoded on the card, copied there, opened
// on wgpu's own device by the renderer's code, drawn, and read back; what is read is the frame the same decoder
// gives through memory, sample for sample.
//
// And the frames of a decoder that is not lurq's: a Direct3D 11 device of this test's stands for one. It keeps
// textures of each kind, writes them, signals its fence, and is given each texture back once the card has
// finished with it.
//
// And the frames of a decoder on another card than the one that draws: they are copied through memory. The copy
// is run on one card, which every machine has, and then with the system's card of software as the one that
// draws, which opens nothing of the real card's.

use std::{
  sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
  },
  time::{Duration, Instant},
};

use windows::{
  Win32::{
    Foundation::{HMODULE, LUID},
    Graphics::{
      Direct3D::{D3D_DRIVER_TYPE_UNKNOWN, D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_11_1},
      Direct3D11::{
        D3D11_BIND_SHADER_RESOURCE, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_FENCE_FLAG_SHARED,
        D3D11_RESOURCE_MISC_SHARED, D3D11_RESOURCE_MISC_SHARED_NTHANDLE, D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC,
        D3D11_USAGE_DEFAULT, D3D11CreateDevice, ID3D11Device, ID3D11Device5, ID3D11DeviceContext, ID3D11DeviceContext4,
        ID3D11Fence, ID3D11Texture2D,
      },
      Dxgi::{
        Common::{
          DXGI_FORMAT, DXGI_FORMAT_NV12, DXGI_FORMAT_R8_UNORM, DXGI_FORMAT_R8G8_UNORM, DXGI_FORMAT_R8G8B8A8_UNORM,
          DXGI_SAMPLE_DESC,
        },
        CreateDXGIFactory1, DXGI_SHARED_RESOURCE_READ, IDXGIAdapter, IDXGIFactory4, IDXGIResource1,
      },
    },
  },
  core::{Interface, PCWSTR},
};

use super::{card_copies, card_frames};
use crate::{
  images::{
    CardCrop, CardFrame, CardFrames, CardFramesWay, CardSurface, ImageData, ImagePixelFormat, NativeImageBackend,
    NativeImageData,
  },
  media::Media,
  resources::{Loader, Source},
};

/// The tests of a media on the card run one at a time: which card draws is said once for the whole program.
static ON_THE_CARD: parking_lot::Mutex<()> = parking_lot::Mutex::new(());

fn clip() -> std::path::PathBuf {
  std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../gallery/assets/media/sample.mp4")
}

/// A device on Direct3D 12 with NV12 textures, as the engine asks for one; `None` where the card has none.
fn device() -> Option<(wgpu::Device, wgpu::Queue)> {
  let instance = wgpu::Instance::new(super::instance_descriptor(wgpu::Backends::DX12));
  let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).ok()?;
  let found = pollster::block_on(adapter.request_device(&super::device_descriptor(
    &adapter,
    wgpu::Features::empty(),
    &wgpu::Limits::default(),
  )))
  .ok()?;
  found
    .0
    .features()
    .contains(wgpu::Features::TEXTURE_FORMAT_NV12)
    .then_some(found)
}

/// The clip, opened and silenced, paused at its start.
fn opened() -> Media {
  let media = Media::open(&Loader::new(), &Source::file(clip()));
  media.set_muted(true);
  media
}

/// The image `media` gives once its first frame is shown, in frames of the card (`on_card`) or of memory.
fn first_frame(media: &Media, on_card: bool) -> ImageData {
  let started = Instant::now();
  loop {
    assert!(
      started.elapsed() < Duration::from_secs(15),
      "no first frame: {:?}",
      media.state()
    );
    if let Some(image) = media.frame() {
      let native = image.native().map(|native| native.backend());
      if on_card && native == Some(NativeImageBackend::CardFrames) {
        return image;
      }
      // Memory's image is there from the start, black: its first frame is a later version of it.
      if !on_card && native.is_none() && image.version() > 0 {
        return image;
      }
    }
    assert_eq!(media.state().error, None);
    std::thread::sleep(Duration::from_millis(10));
  }
}

/// The frame `image` shows, of the card, drawn by the renderer's own code and read back.
fn drawn_from(device: &wgpu::Device, queue: &wgpu::Queue, image: &ImageData) -> Vec<u8> {
  let frames = image
    .native()
    .and_then(|native| native.payload::<CardFrames>())
    .expect("the frames of the card")
    .clone();
  let shown = frames.shown().expect("a frame is shown").surface();
  let (handle, _) = frames.surfaces()[shown].handles();
  let (texture, size) =
    card_frames::open(device, handle, &[wgpu::TextureFormat::NV12]).expect("the renderer's device opens the frame");
  assert_eq!(
    size,
    (image.width(), image.height()),
    "the media's texture is as large as its picture"
  );
  drawn(device, queue, &texture, image.width(), image.height())
}

/// How many pixels of `pixels` (a sample of gray and two of color each) are not `memory`'s frame, in gray and
/// in color, and the most one is off by.
fn differing(pixels: &[u8], memory: &[u8], wide: usize, high: usize) -> (usize, usize, u8) {
  let (mut gray, mut color, mut worst) = (0_usize, 0_usize, 0_u8);
  for y in 0..high {
    for x in 0..wide {
      let pixel = &pixels[(y * wide + x) * 4..][..4];
      let from = memory[y * wide + x];
      let at = wide * high + (y / 2) * wide + (x / 2) * 2;
      let (u, v) = (memory[at], memory[at + 1]);
      if pixel[0] != from {
        gray += 1;
        worst = worst.max(pixel[0].abs_diff(from));
      }
      if pixel[1] != u || pixel[2] != v {
        color += 1;
        worst = worst.max(pixel[1].abs_diff(u)).max(pixel[2].abs_diff(v));
      }
    }
  }
  (gray, color, worst)
}

/// Draws the two planes of `texture` into a target of its size, a sample of gray and its two of color in each
/// pixel, and reads the target back.
fn drawn(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture, width: u32, height: u32) -> Vec<u8> {
  let (gray, color) = card_frames::planes(texture);
  let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
    label: Some("planes"),
    source: wgpu::ShaderSource::Wgsl(
      "@group(0) @binding(0) var gray: texture_2d<f32>;
       @group(0) @binding(1) var color: texture_2d<f32>;
       @vertex fn vs(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
         var corners = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
         return vec4(corners[index], 0.0, 1.0);
       }
       @fragment fn fs(@builtin(position) at: vec4<f32>) -> @location(0) vec4<f32> {
         let pixel = vec2<i32>(at.xy);
         return vec4(textureLoad(gray, pixel, 0).r, textureLoad(color, pixel / 2, 0).rg, 1.0);
       }"
      .into(),
    ),
  });
  let plane = |binding| wgpu::BindGroupLayoutEntry {
    binding,
    visibility: wgpu::ShaderStages::FRAGMENT,
    ty: wgpu::BindingType::Texture {
      sample_type: wgpu::TextureSampleType::Float { filterable: true },
      view_dimension: wgpu::TextureViewDimension::D2,
      multisampled: false,
    },
    count: None,
  };
  let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    label: None,
    entries: &[plane(0), plane(1)],
  });
  let bound = device.create_bind_group(&wgpu::BindGroupDescriptor {
    label: None,
    layout: &layout,
    entries: &[
      wgpu::BindGroupEntry {
        binding: 0,
        resource: wgpu::BindingResource::TextureView(&gray),
      },
      wgpu::BindGroupEntry {
        binding: 1,
        resource: wgpu::BindingResource::TextureView(&color),
      },
    ],
  });
  let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
    label: None,
    bind_group_layouts: &[Some(&layout)],
    immediate_size: 0,
  });
  let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: None,
    layout: Some(&pipeline_layout),
    vertex: wgpu::VertexState {
      module: &shader,
      entry_point: Some("vs"),
      buffers: &[],
      compilation_options: Default::default(),
    },
    fragment: Some(wgpu::FragmentState {
      module: &shader,
      entry_point: Some("fs"),
      targets: &[Some(wgpu::TextureFormat::Rgba8Unorm.into())],
      compilation_options: Default::default(),
    }),
    primitive: Default::default(),
    depth_stencil: None,
    multisample: Default::default(),
    multiview_mask: None,
    cache: None,
  });
  let target = device.create_texture(&wgpu::TextureDescriptor {
    label: None,
    size: wgpu::Extent3d {
      width,
      height,
      depth_or_array_layers: 1,
    },
    mip_level_count: 1,
    sample_count: 1,
    dimension: wgpu::TextureDimension::D2,
    format: wgpu::TextureFormat::Rgba8Unorm,
    usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    view_formats: &[],
  });
  let view = target.create_view(&Default::default());
  let row = (width * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
  let read = device.create_buffer(&wgpu::BufferDescriptor {
    label: None,
    size: u64::from(row * height),
    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
    mapped_at_creation: false,
  });
  let mut encoder = device.create_command_encoder(&Default::default());
  {
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
      label: None,
      color_attachments: &[Some(wgpu::RenderPassColorAttachment {
        view: &view,
        depth_slice: None,
        resolve_target: None,
        ops: wgpu::Operations {
          load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
          store: wgpu::StoreOp::Store,
        },
      })],
      depth_stencil_attachment: None,
      timestamp_writes: None,
      occlusion_query_set: None,
      multiview_mask: None,
    });
    pass.set_pipeline(&pipeline);
    pass.set_bind_group(0, &bound, &[]);
    pass.draw(0..3, 0..1);
  }
  encoder.copy_texture_to_buffer(
    target.as_image_copy(),
    wgpu::TexelCopyBufferInfo {
      buffer: &read,
      layout: wgpu::TexelCopyBufferLayout {
        offset: 0,
        bytes_per_row: Some(row),
        rows_per_image: Some(height),
      },
    },
    wgpu::Extent3d {
      width,
      height,
      depth_or_array_layers: 1,
    },
  );
  queue.submit([encoder.finish()]);
  read.slice(..).map_async(wgpu::MapMode::Read, |mapped| mapped.unwrap());
  device
    .poll(wgpu::PollType::wait_indefinitely())
    .expect("the card answers");
  let mapped = read.slice(..).get_mapped_range();
  let mut pixels = Vec::with_capacity((width * height * 4) as usize);
  for line in mapped.chunks(row as usize) {
    pixels.extend_from_slice(&line[..(width * 4) as usize]);
  }
  pixels
}

#[test]
fn a_frame_decoded_and_drawn_on_the_card_is_the_frame_memory_gives() {
  let Some((device, queue)) = device() else {
    eprintln!("skipped: this card has no Direct3D 12 device with NV12 textures");
    return;
  };
  let _alone = ON_THE_CARD.lock();
  // Nothing draws on a card yet: the frame goes through memory, as it always did.
  crate::images::set_card_frames_adapter(None);
  let early = opened();
  let in_memory = first_frame(&early, false);
  let (width, height) = (in_memory.width(), in_memory.height());
  assert_eq!((width, height), (640, 360));
  let memory = in_memory.data_arc();
  assert_eq!(memory.len(), (width * height * 3 / 2) as usize);
  let (wide, high) = (width as usize, height as usize);

  // The renderer says which card it draws on: the decoder decodes there, and its frames stay there.
  card_frames::announce(&device);
  assert!(
    crate::images::card_frames_adapter().is_some(),
    "a device with NV12 on Direct3D 12 says so"
  );
  let late = opened();
  let on_card = first_frame(&late, true);
  assert_eq!((on_card.width(), on_card.height()), (width, height));
  let pixels = drawn_from(&device, &queue, &on_card);
  // Every sample of gray, and the two of color of every two by two, are memory's.
  assert_eq!(
    differing(&pixels, &memory, wide, high),
    (0, 0, 0),
    "the pixels that differ in gray and in color of {}, and the most one is off by",
    wide * high
  );
  // The picture is a picture: the clip's first frame is not one flat color.
  let first = pixels[0];
  assert!(
    pixels.chunks(4).any(|pixel| pixel[0].abs_diff(first) > 32),
    "the frame has light and dark in it"
  );

  // The media that opened before anything drew on a card (a player on an app's first screen) starts over on
  // the card by itself, where it was, and its frame there is the same one.
  let moved = first_frame(&early, true);
  assert_eq!(
    differing(&drawn_from(&device, &queue, &moved), &memory, wide, high),
    (0, 0, 0),
    "the frame of the media that started over on the card"
  );
  let state = early.state();
  assert_eq!((state.error, state.playing, state.position), (None, false, 0.0));

  // Played, its frames go on coming on the card, each in another of its textures, and it is another picture.
  let frames = moved
    .native()
    .and_then(|native| native.payload::<CardFrames>())
    .expect("the frames of the card")
    .clone();
  let shown = || frames.shown().expect("a frame is shown").surface();
  let (before, slot) = (moved.version(), shown());
  early.play();
  let started = Instant::now();
  let mut slots = std::collections::BTreeSet::from([slot]);
  while started.elapsed() < Duration::from_millis(600) {
    slots.insert(shown());
    std::thread::sleep(Duration::from_millis(5));
  }
  early.pause();
  let shown = moved.version() - before;
  assert!(
    (10..=22).contains(&shown),
    "about 18 frames of 30 a second in 0.6 s: {shown}"
  );
  assert_eq!(
    slots.len(),
    frames.surfaces().len(),
    "every texture took its turn: {slots:?}"
  );
  let later = drawn_from(&device, &queue, &moved);
  let (gray, ..) = differing(&later, &memory, wide, high);
  assert!(
    gray > wide * high / 100,
    "a frame half a second on is another picture: {gray} pixels differ"
  );
  assert_eq!(early.state().error, None);
  crate::images::set_card_frames_adapter(None);
}

/// What stands for a decoder that is not lurq's: a Direct3D 11 device on the card `device` draws on, with a
/// fence it signals as it writes.
struct Decoder {
  device: ID3D11Device,
  context: ID3D11DeviceContext,
  fence: ID3D11Fence,
  signalled: u64,
}

impl Decoder {
  fn on_the_card_of(device: &wgpu::Device) -> Self {
    // SAFETY: the card of wgpu's device is read, and a device of this test's own is made on it.
    unsafe {
      let luid: LUID = device
        .as_hal::<wgpu::hal::api::Dx12>()
        .expect("a device of Direct3D 12")
        .raw_device()
        .GetAdapterLuid();
      let cards: IDXGIFactory4 = CreateDXGIFactory1().unwrap();
      let card: IDXGIAdapter = cards.EnumAdapterByLuid(luid).unwrap();
      let (mut made, mut context): (Option<ID3D11Device>, Option<ID3D11DeviceContext>) = (None, None);
      D3D11CreateDevice(
        &card,
        D3D_DRIVER_TYPE_UNKNOWN,
        HMODULE::default(),
        D3D11_CREATE_DEVICE_BGRA_SUPPORT,
        Some(&[D3D_FEATURE_LEVEL_11_1, D3D_FEATURE_LEVEL_11_0]),
        D3D11_SDK_VERSION,
        Some(&mut made),
        None,
        Some(&mut context),
      )
      .unwrap();
      let (made, context) = (made.unwrap(), context.unwrap());
      let mut fence: Option<ID3D11Fence> = None;
      made
        .cast::<ID3D11Device5>()
        .unwrap()
        .CreateFence(0, D3D11_FENCE_FLAG_SHARED, &mut fence)
        .unwrap();
      Self {
        device: made,
        context,
        fence: fence.unwrap(),
        signalled: 0,
      }
    }
  }

  /// A handle of the fence, for the frames to keep and close.
  fn fence(&self) -> isize {
    const GENERIC_ALL: u32 = 0x1000_0000;
    // SAFETY: the fence was made to be shared.
    unsafe {
      self
        .fence
        .CreateSharedHandle(None, GENERIC_ALL, PCWSTR::null())
        .unwrap()
        .0 as isize
    }
  }

  /// A shared texture of `format`, and a handle of it for the frames to keep and close.
  fn texture(&self, format: DXGI_FORMAT, width: u32, height: u32) -> (ID3D11Texture2D, isize) {
    let like = D3D11_TEXTURE2D_DESC {
      Width: width,
      Height: height,
      MipLevels: 1,
      ArraySize: 1,
      Format: format,
      SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
      Usage: D3D11_USAGE_DEFAULT,
      BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
      CPUAccessFlags: 0,
      MiscFlags: (D3D11_RESOURCE_MISC_SHARED_NTHANDLE.0 | D3D11_RESOURCE_MISC_SHARED.0) as u32,
    };
    // SAFETY: a texture of this test's own is made, and a handle of it.
    unsafe {
      let mut texture = None;
      self.device.CreateTexture2D(&like, None, Some(&mut texture)).unwrap();
      let texture = texture.unwrap();
      let handle = texture
        .cast::<IDXGIResource1>()
        .unwrap()
        .CreateSharedHandle(None, DXGI_SHARED_RESOURCE_READ.0, PCWSTR::null())
        .unwrap();
      (texture, handle.0 as isize)
    }
  }

  /// Writes `bytes`, in rows of `pitch`, into the texture.
  fn write(&self, texture: &ID3D11Texture2D, bytes: &[u8], pitch: u32) {
    // SAFETY: the bytes are as many as the texture's rows of this pitch.
    unsafe {
      self
        .context
        .UpdateSubresource(texture, 0, None, bytes.as_ptr().cast(), pitch, 0);
    }
  }

  /// Signals the fence after what was written so far, and answers with the value it will have then.
  fn signal(&mut self) -> u64 {
    self.signalled += 1;
    // SAFETY: the fence is this device's.
    unsafe {
      self
        .context
        .cast::<ID3D11DeviceContext4>()
        .unwrap()
        .Signal(&self.fence, self.signalled)
        .unwrap();
      self.context.Flush();
    }
    self.signalled
  }
}

/// What a decoder puts in a frame's lease: it says when it was let go of.
struct Returned(Arc<AtomicBool>);

impl Drop for Returned {
  fn drop(&mut self) {
    self.0.store(true, Ordering::SeqCst);
  }
}

fn lease() -> (Returned, Arc<AtomicBool>) {
  let returned = Arc::new(AtomicBool::new(false));
  (Returned(returned.clone()), returned)
}

/// Polls the device until `is` holds, for at most five seconds; whether it came to hold.
fn comes(device: &wgpu::Device, mut is: impl FnMut() -> bool) -> bool {
  let started = Instant::now();
  loop {
    let _ = device.poll(wgpu::PollType::Poll);
    if is() {
      return true;
    }
    if started.elapsed() > Duration::from_secs(5) {
      return false;
    }
    std::thread::sleep(Duration::from_millis(2));
  }
}

/// The bytes of a texture of one plane, `bytes` to a pixel, read back.
fn read(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture, bytes: u32) -> Vec<u8> {
  let (width, height) = (texture.width(), texture.height());
  let row = (width * bytes).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
  let buffer = device.create_buffer(&wgpu::BufferDescriptor {
    label: None,
    size: u64::from(row * height),
    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
    mapped_at_creation: false,
  });
  let mut encoder = device.create_command_encoder(&Default::default());
  encoder.copy_texture_to_buffer(
    texture.as_image_copy(),
    wgpu::TexelCopyBufferInfo {
      buffer: &buffer,
      layout: wgpu::TexelCopyBufferLayout {
        offset: 0,
        bytes_per_row: Some(row),
        rows_per_image: Some(height),
      },
    },
    wgpu::Extent3d {
      width,
      height,
      depth_or_array_layers: 1,
    },
  );
  queue.submit([encoder.finish()]);
  buffer
    .slice(..)
    .map_async(wgpu::MapMode::Read, |mapped| mapped.unwrap());
  device
    .poll(wgpu::PollType::wait_indefinitely())
    .expect("the card answers");
  let mapped = buffer.slice(..).get_mapped_range();
  let mut pixels = Vec::with_capacity((width * height * bytes) as usize);
  for line in mapped.chunks(row as usize) {
    pixels.extend_from_slice(&line[..(width * bytes) as usize]);
  }
  pixels
}

/// The frames opened as the renderer opens them, bound to nothing: the test draws them itself.
fn opened_frames(
  device: &wgpu::Device,
  frames: &CardFrames,
  picture: (u32, u32),
  leases: &card_frames::Leases,
) -> card_frames::Opened {
  let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    label: None,
    entries: &[],
  });
  let mut readers = card_copies::Readers::default();
  let opened = card_frames::Opened::open(device, frames, picture, leases, &mut readers, |_, _| {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
      label: None,
      layout: &layout,
      entries: &[],
    })
  });
  assert_eq!(
    frames.way(),
    Some(CardFramesWay::Card),
    "the renderer's device opens the decoder's surfaces"
  );
  opened
}

const WIDE: u32 = 256;
/// The texture is as high as what is coded, and the picture two rows less: 1088 and 1080, in small.
const HIGH: u32 = 8;
const PICTURE: u32 = 6;

#[test]
fn a_decoder_of_the_apps_own_shows_frames_when_they_are_written_and_gets_its_textures_back() {
  let Some((device, queue)) = device() else {
    eprintln!("skipped: this card has no Direct3D 12 device with NV12 textures");
    return;
  };
  let mut decoder = Decoder::on_the_card_of(&device);
  let leases = card_frames::Leases::default();

  // Two surfaces of gray and color apart, as a decoder gives them that cannot write one NV12 texture.
  let surfaces: Vec<_> = (0..2)
    .map(|_| {
      let (gray, gray_handle) = decoder.texture(DXGI_FORMAT_R8_UNORM, WIDE, HIGH);
      let (color, color_handle) = decoder.texture(DXGI_FORMAT_R8G8_UNORM, WIDE / 2, HIGH / 2);
      (
        gray,
        color,
        CardSurface::Planes {
          gray: gray_handle,
          color: color_handle,
        },
      )
    })
    .collect();
  // SAFETY: the handles are of textures and of a fence made to be shared, of one kind and size, given away.
  let frames = unsafe { CardFrames::with_fence(surfaces.iter().map(|surface| surface.2).collect(), decoder.fence()) };
  let image = NativeImageData::from_card_frames(WIDE, PICTURE, frames.clone()).image_data();
  assert_eq!(image.format(), ImagePixelFormat::Nv12);
  let mut opened = opened_frames(&device, &frames, (WIDE, PICTURE), &leases);
  let write = |decoder: &mut Decoder, surface: usize, gray: u8, color: u8| {
    decoder.write(&surfaces[surface].0, &vec![gray; (WIDE * HIGH) as usize], WIDE);
    decoder.write(&surfaces[surface].1, &vec![color; (WIDE * HIGH / 2) as usize], WIDE);
    decoder.signal()
  };
  let grays = |opened: &card_frames::Opened| {
    let (_, slot) = opened.drawn().expect("a frame is drawn");
    let (gray, color) = (
      read(&device, &queue, &slot.textures()[0], 1),
      read(&device, &queue, &slot.textures()[1], 2),
    );
    (
      (*gray.iter().min().unwrap(), *gray.iter().max().unwrap()),
      (*color.iter().min().unwrap(), *color.iter().max().unwrap()),
    )
  };

  // Nothing was shown: nothing is drawn.
  opened.pick(&queue);
  assert!(opened.drawn().is_none());

  // The first frame: written, and shown with the value its fence has once it is.
  let (first, first_returned) = lease();
  let ready = write(&mut decoder, 0, 10, 20);
  frames.show(CardFrame::of(0).ready_at(ready).lease(first));
  assert!(
    comes(&device, || {
      opened.pick(&queue);
      opened.drawn().is_some()
    }),
    "the first frame is never drawn"
  );
  assert_eq!(opened.drawn().map(|(surface, _)| surface), Some(0));
  assert_eq!(grays(&opened), ((10, 10), (20, 20)));

  // The second is shown before it is written: the first stays, and nothing waits.
  let (second, second_returned) = lease();
  frames.show(CardFrame::of(1).ready_at(ready + 1).lease(second));
  for _ in 0..20 {
    opened.pick(&queue);
    std::thread::sleep(Duration::from_millis(5));
  }
  assert_eq!(
    opened.drawn().map(|(surface, _)| surface),
    Some(0),
    "a frame that is not written yet is not drawn"
  );
  assert!(!first_returned.load(Ordering::SeqCst));
  // Written, it is the one drawn.
  assert_eq!(write(&mut decoder, 1, 200, 90), ready + 1);
  assert!(
    comes(&device, || {
      opened.pick(&queue);
      opened.drawn().is_some_and(|(surface, _)| surface == 1)
    }),
    "the second frame is never drawn"
  );
  assert_eq!(grays(&opened), ((200, 200), (90, 90)));
  // The first is kept until the card has done what drew it, and then given back; the second is drawn still.
  assert!(
    !first_returned.load(Ordering::SeqCst),
    "the card may still be drawing the first"
  );
  leases.let_go_after(&queue);
  assert!(
    comes(&device, || first_returned.load(Ordering::SeqCst)),
    "the first is never given back"
  );
  assert!(!second_returned.load(Ordering::SeqCst));

  // A frame another takes the place of before anything drew it is given back at once.
  let ((third, third_returned), (fourth, fourth_returned)) = (lease(), lease());
  frames.show(CardFrame::of(0).ready_at(ready + 2).lease(third));
  frames.show(CardFrame::of(0).ready_at(ready + 3).lease(fourth));
  assert!(third_returned.load(Ordering::SeqCst));
  assert!(!fourth_returned.load(Ordering::SeqCst));

  // The picture is the top six rows of eight: it is drawn from a row within its lower edge, for the color.
  assert_eq!(opened.part([0.0, 0.0], [1.0, 1.0]), ([0.0, 0.0], [1.0, 5.0 / 8.0]));
  // A part the frame says itself, and half of the picture's width within it.
  let (cropped, _) = lease();
  let crop = CardCrop {
    left: 16,
    top: 2,
    width: 128,
    height: 4,
  };
  decoder.signal();
  decoder.signal();
  frames.show(CardFrame::of(0).crop(crop).lease(cropped));
  opened.pick(&queue);
  assert_eq!(opened.drawn().map(|(surface, _)| surface), Some(0));
  let (from, to) = opened.part([0.5, 0.0], [1.0, 1.0]);
  let (left, right) = (17.0 / 256.0, 143.0 / 256.0);
  assert!((from[0] - (left + right) / 2.0).abs() < 1e-6 && (to[0] - right).abs() < 1e-6);
  assert_eq!((from[1], to[1]), (3.0 / 8.0, 5.0 / 8.0));

  // No longer on the screen, what was drawn last is given back once the card has finished, and not before.
  assert!(fourth_returned.load(Ordering::SeqCst), "the fourth was never drawn");
  drop(opened);
  assert!(!second_returned.load(Ordering::SeqCst));
  leases.let_go_after(&queue);
  assert!(
    comes(&device, || second_returned.load(Ordering::SeqCst)),
    "the second is never given back"
  );

  // One NV12 texture: its two planes are drawn as the frames of lurq's own media are.
  let (nv12, handle) = decoder.texture(DXGI_FORMAT_NV12, WIDE, HIGH);
  let mut bytes = vec![77_u8; (WIDE * HIGH) as usize];
  bytes.extend((0..WIDE * HIGH / 4).flat_map(|_| [90_u8, 160]));
  decoder.write(&nv12, &bytes, WIDE);
  decoder.signal();
  // SAFETY: the handle is of an NV12 texture made to be shared, given away.
  let frames = unsafe { CardFrames::new(vec![CardSurface::Nv12(handle)]) };
  frames.show(CardFrame::of(0));
  let mut opened = opened_frames(&device, &frames, (WIDE, HIGH), &leases);
  opened.pick(&queue);
  let (_, slot) = opened.drawn().expect("the NV12 frame is drawn");
  let pixels = drawn(&device, &queue, &slot.textures()[0], WIDE, HIGH);
  assert!(
    pixels.chunks(4).all(|pixel| pixel[..3] == [77, 90, 160]),
    "the gray and the color of the NV12 frame: {:?}",
    &pixels[..4]
  );
  assert_eq!(opened.part([0.0, 0.0], [1.0, 1.0]), ([0.0, 0.0], [1.0, 1.0]));

  // One texture of red, green and blue, as a decoder gives that converts on the card.
  let (rgba, handle) = decoder.texture(DXGI_FORMAT_R8G8B8A8_UNORM, WIDE, HIGH);
  let bytes: Vec<u8> = (0..WIDE * HIGH).flat_map(|_| [10_u8, 20, 30, 255]).collect();
  decoder.write(&rgba, &bytes, WIDE * 4);
  decoder.signal();
  // SAFETY: the handle is of a texture of red, green, blue and alpha made to be shared, given away.
  let frames = unsafe { CardFrames::new(vec![CardSurface::Rgba(handle)]) };
  let image = NativeImageData::from_card_frames(WIDE, HIGH, frames.clone()).image_data();
  assert_eq!(image.format(), ImagePixelFormat::Rgba8);
  frames.show(CardFrame::of(0));
  let mut opened = opened_frames(&device, &frames, (WIDE, HIGH), &leases);
  opened.pick(&queue);
  assert!(opened.is_rgba());
  let (_, slot) = opened.drawn().expect("the frame of red, green and blue is drawn");
  assert!(slot.second.is_none());
  let pixels = read(&device, &queue, &slot.textures()[0], 4);
  assert!(
    pixels.chunks(4).all(|pixel| pixel == [10, 20, 30, 255]),
    "the red, green and blue of the frame: {:?}",
    &pixels[..4]
  );
}

/// A device on the system's card of software, which every Windows has beside its real ones; `None` where wgpu
/// makes none on it.
fn software_device() -> Option<(wgpu::Device, wgpu::Queue)> {
  let instance = wgpu::Instance::new(super::instance_descriptor(wgpu::Backends::DX12));
  let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
    force_fallback_adapter: true,
    ..Default::default()
  }))
  .ok()?;
  pollster::block_on(adapter.request_device(&super::device_descriptor(
    &adapter,
    wgpu::Features::empty(),
    &wgpu::Limits::default(),
  )))
  .ok()
}

/// The card `device` is on.
fn card_of(device: &wgpu::Device) -> (u32, i32) {
  // SAFETY: the card of wgpu's device is read.
  let card = unsafe {
    device
      .as_hal::<wgpu::hal::api::Dx12>()
      .expect("a device of Direct3D 12")
      .raw_device()
      .GetAdapterLuid()
  };
  (card.LowPart, card.HighPart)
}

/// The frames opened to be copied through memory though `device` could open them itself, bound to nothing.
fn copied_frames(
  device: &wgpu::Device,
  frames: &CardFrames,
  picture: (u32, u32),
  leases: &card_frames::Leases,
  readers: &mut card_copies::Readers,
) -> card_frames::Opened {
  let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    label: None,
    entries: &[],
  });
  card_frames::Opened::through_memory(device, frames, picture, leases, readers, |_, _| {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
      label: None,
      layout: &layout,
      entries: &[],
    })
  })
}

/// The least and the greatest byte of the gray that is drawn, and of the color.
fn drawn_grays(
  device: &wgpu::Device,
  queue: &wgpu::Queue,
  opened: &card_frames::Opened,
) -> Option<((u8, u8), (u8, u8))> {
  let (_, slot) = opened.drawn()?;
  let (gray, color) = (
    read(device, queue, &slot.textures()[0], 1),
    read(device, queue, &slot.textures()[1], 2),
  );
  Some((
    (*gray.iter().min().unwrap(), *gray.iter().max().unwrap()),
    (*color.iter().min().unwrap(), *color.iter().max().unwrap()),
  ))
}

#[test]
fn the_frames_of_a_decoder_on_another_card_come_through_memory() {
  let Some((device, queue)) = device() else {
    eprintln!("skipped: this card has no Direct3D 12 device with NV12 textures");
    return;
  };
  let mut decoder = Decoder::on_the_card_of(&device);
  let leases = card_frames::Leases::default();
  let mut readers = card_copies::Readers::default();

  // Two surfaces of gray and color apart, with a fence, copied through memory.
  let surfaces: Vec<_> = (0..2)
    .map(|_| {
      let (gray, gray_handle) = decoder.texture(DXGI_FORMAT_R8_UNORM, WIDE, HIGH);
      let (color, color_handle) = decoder.texture(DXGI_FORMAT_R8G8_UNORM, WIDE / 2, HIGH / 2);
      (
        gray,
        color,
        CardSurface::Planes {
          gray: gray_handle,
          color: color_handle,
        },
      )
    })
    .collect();
  // SAFETY: the handles are of textures and of a fence made to be shared, of one kind and size, given away.
  let frames = unsafe { CardFrames::with_fence(surfaces.iter().map(|surface| surface.2).collect(), decoder.fence()) };
  assert_eq!(frames.way(), None, "nothing has opened the frames yet");
  let mut opened = copied_frames(&device, &frames, (WIDE, PICTURE), &leases, &mut readers);
  assert_eq!(frames.way(), Some(CardFramesWay::Memory));
  let write = |decoder: &mut Decoder, surface: usize, gray: u8, color: u8| {
    decoder.write(&surfaces[surface].0, &vec![gray; (WIDE * HIGH) as usize], WIDE);
    decoder.write(&surfaces[surface].1, &vec![color; (WIDE * HIGH / 2) as usize], WIDE);
    decoder.signal()
  };

  // Nothing was shown: nothing is drawn.
  opened.pick(&queue);
  assert!(opened.drawn().is_none());

  // The first frame is read from its card and is what the renderer's textures hold.
  let (first, first_returned) = lease();
  let ready = write(&mut decoder, 0, 10, 20);
  frames.show(CardFrame::of(0).ready_at(ready).lease(first));
  assert!(
    comes(&device, || {
      opened.pick(&queue);
      opened.drawn().is_some()
    }),
    "the first frame never comes"
  );
  assert_eq!(drawn_grays(&device, &queue, &opened), Some(((10, 10), (20, 20))));
  assert!(!first_returned.load(Ordering::SeqCst), "it is the frame to show still");

  // The second is shown before it is written. The first is on the screen from the renderer's own textures, and
  // nothing draws the decoder's: it has its texture back at once, with no card to wait for.
  let (second, second_returned) = lease();
  frames.show(CardFrame::of(1).ready_at(ready + 1).lease(second));
  assert!(
    first_returned.load(Ordering::SeqCst),
    "the first is read: its texture is the decoder's again"
  );
  for _ in 0..20 {
    opened.pick(&queue);
    std::thread::sleep(Duration::from_millis(5));
  }
  assert_eq!(
    drawn_grays(&device, &queue, &opened),
    Some(((10, 10), (20, 20))),
    "a frame that is not written yet is not copied"
  );
  // Written, it comes.
  assert_eq!(write(&mut decoder, 1, 200, 90), ready + 1);
  assert!(
    comes(&device, || {
      opened.pick(&queue);
      drawn_grays(&device, &queue, &opened) == Some(((200, 200), (90, 90)))
    }),
    "the second frame never comes"
  );
  assert!(!second_returned.load(Ordering::SeqCst));
  // The picture is a part of its texture as it is on the card.
  assert_eq!(opened.part([0.0, 0.0], [1.0, 1.0]), ([0.0, 0.0], [1.0, 5.0 / 8.0]));
  // A frame another takes the place of is let go of: at once where its copy was not begun, and once it is read
  // where it was.
  let (third, third_returned) = lease();
  let ready = write(&mut decoder, 0, 30, 40);
  frames.show(CardFrame::of(0).ready_at(ready).lease(third));
  assert!(second_returned.load(Ordering::SeqCst));
  opened.pick(&queue);
  frames.show(CardFrame::of(1).ready_at(ready));
  assert!(
    comes(&device, || {
      opened.pick(&queue);
      third_returned.load(Ordering::SeqCst)
    }),
    "the third is never given back"
  );
  drop(opened);

  // One NV12 texture: its gray and its color are read from the one texture.
  let (nv12, handle) = decoder.texture(DXGI_FORMAT_NV12, WIDE, HIGH);
  let mut bytes = vec![77_u8; (WIDE * HIGH) as usize];
  bytes.extend((0..WIDE * HIGH / 4).flat_map(|_| [90_u8, 160]));
  decoder.write(&nv12, &bytes, WIDE);
  decoder.signal();
  // SAFETY: the handle is of an NV12 texture made to be shared, given away.
  let frames = unsafe { CardFrames::new(vec![CardSurface::Nv12(handle)]) };
  frames.show(CardFrame::of(0));
  let mut opened = copied_frames(&device, &frames, (WIDE, HIGH), &leases, &mut readers);
  assert_eq!(frames.way(), Some(CardFramesWay::Memory));
  assert!(
    comes(&device, || {
      opened.pick(&queue);
      opened.drawn().is_some()
    }),
    "the NV12 frame never comes"
  );
  assert_eq!(drawn_grays(&device, &queue, &opened), Some(((77, 77), (90, 160))));
  let (_, slot) = opened.drawn().unwrap();
  let color = read(&device, &queue, &slot.textures()[1], 2);
  assert!(
    color.chunks(2).all(|pair| pair == [90, 160]),
    "the color: {:?}",
    &color[..4]
  );
  drop(opened);

  // One texture of red, green and blue.
  let (rgba, handle) = decoder.texture(DXGI_FORMAT_R8G8B8A8_UNORM, WIDE, HIGH);
  let bytes: Vec<u8> = (0..WIDE * HIGH).flat_map(|_| [10_u8, 20, 30, 255]).collect();
  decoder.write(&rgba, &bytes, WIDE * 4);
  decoder.signal();
  // SAFETY: the handle is of a texture of red, green, blue and alpha made to be shared, given away.
  let frames = unsafe { CardFrames::new(vec![CardSurface::Rgba(handle)]) };
  frames.show(CardFrame::of(0));
  let mut opened = copied_frames(&device, &frames, (WIDE, HIGH), &leases, &mut readers);
  assert!(
    comes(&device, || {
      opened.pick(&queue);
      opened.drawn().is_some()
    }),
    "the frame of red, green and blue never comes"
  );
  let (_, slot) = opened.drawn().unwrap();
  assert!(slot.second.is_none());
  let pixels = read(&device, &queue, &slot.textures()[0], 4);
  assert!(
    pixels.chunks(4).all(|pixel| pixel == [10, 20, 30, 255]),
    "the red, green and blue of the frame: {:?}",
    &pixels[..4]
  );
  drop(opened);

  // A handle that is no texture is opened by no card: nothing is drawn, and the frames say so.
  // SAFETY: the handle is one made to be shared and given away; that it is a fence's is what is tried.
  let frames = unsafe { CardFrames::new(vec![CardSurface::Nv12(decoder.fence())]) };
  frames.show(CardFrame::of(0));
  let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    label: None,
    entries: &[],
  });
  let nothing = |device: &wgpu::Device| {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
      label: None,
      layout: &layout,
      entries: &[],
    })
  };
  let mut opened = card_frames::Opened::open(&device, &frames, (WIDE, HIGH), &leases, &mut readers, |_, _| {
    nothing(&device)
  });
  assert_eq!(frames.way(), Some(CardFramesWay::Nowhere));
  opened.pick(&queue);
  assert!(opened.drawn().is_none());
  drop(opened);

  // Really another card: the system's card of software draws, and the decoder is on the real one. The device
  // that draws opens nothing of the decoder's; the real card's does, and the frame comes through memory.
  let Some((soft, soft_queue)) = software_device() else {
    eprintln!("not run: no device on the system's card of software");
    return;
  };
  if card_of(&soft) == card_of(&device) {
    eprintln!("not run: the only card here is the card of software");
    return;
  }
  let (gray, gray_handle) = decoder.texture(DXGI_FORMAT_R8_UNORM, WIDE, HIGH);
  let (color, color_handle) = decoder.texture(DXGI_FORMAT_R8G8_UNORM, WIDE / 2, HIGH / 2);
  // SAFETY: the handles are of textures and of a fence made to be shared, given away.
  let frames = unsafe {
    CardFrames::with_fence(
      vec![CardSurface::Planes {
        gray: gray_handle,
        color: color_handle,
      }],
      decoder.fence(),
    )
  };
  let soft_layout = soft.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    label: None,
    entries: &[],
  });
  let mut readers = card_copies::Readers::default();
  let mut opened = card_frames::Opened::open(&soft, &frames, (WIDE, PICTURE), &leases, &mut readers, |_, _| {
    soft.create_bind_group(&wgpu::BindGroupDescriptor {
      label: None,
      layout: &soft_layout,
      entries: &[],
    })
  });
  assert_eq!(
    frames.way(),
    Some(CardFramesWay::Memory),
    "the card that draws opens nothing of another card's, and the other card does"
  );
  decoder.write(&gray, &vec![123; (WIDE * HIGH) as usize], WIDE);
  decoder.write(&color, &vec![45; (WIDE * HIGH / 2) as usize], WIDE);
  let ready = decoder.signal();
  let (other, other_returned) = lease();
  frames.show(CardFrame::of(0).ready_at(ready).lease(other));
  assert!(
    comes(&soft, || {
      opened.pick(&soft_queue);
      opened.drawn().is_some()
    }),
    "the frame of the other card never comes"
  );
  assert_eq!(drawn_grays(&soft, &soft_queue, &opened), Some(((123, 123), (45, 45))));
  frames.show(CardFrame::of(0).ready_at(ready + 1));
  assert!(other_returned.load(Ordering::SeqCst));
}

/// Waits until `is` holds, for at most ten seconds; whether it came to hold.
fn becomes(mut is: impl FnMut() -> bool) -> bool {
  let started = Instant::now();
  while started.elapsed() < Duration::from_secs(10) {
    if is() {
      return true;
    }
    std::thread::sleep(Duration::from_millis(10));
  }
  false
}

#[test]
fn a_media_whose_card_is_reset_is_opened_again_where_it_was() {
  let Some((device, queue)) = device() else {
    eprintln!("skipped: this card has no Direct3D 12 device with NV12 textures");
    return;
  };
  let _alone = ON_THE_CARD.lock();
  card_frames::announce(&device);
  let media = opened();
  let first = first_frame(&media, true);
  let frames_of = |image: &ImageData| {
    image
      .native()
      .and_then(|native| native.payload::<CardFrames>())
      .expect("the frames of the card")
      .clone()
  };
  let before = frames_of(&first);
  // Sent on, paused: the frame there is shown.
  media.seek(5.0);
  assert!(
    becomes(|| before.count() >= 2),
    "the frame it was sent to is never shown"
  );
  let there = drawn_from(&device, &queue, &first);

  // The card is reset: the device the media decodes on is removed, which the next frame it shows finds.
  crate::media::lose_card_for_test();
  media.seek(5.0);
  // It is opened again on another device, with other textures, and shows the frame where it was.
  assert!(
    becomes(|| {
      media.frame().is_some_and(|image| {
        image.native().map(|native| native.backend()) == Some(NativeImageBackend::CardFrames)
          && !frames_of(&image).same(&before)
          && frames_of(&image).shown().is_some()
      })
    }),
    "the media is not opened again: {:?}",
    media.state()
  );
  assert_eq!(media.state().error, None);
  let state = media.state();
  assert!(
    (state.position - 5.0).abs() < 0.2 && !state.playing,
    "it is where it was, paused as it was: {state:?}"
  );
  let again = drawn_from(
    &device,
    &queue,
    &media.frame().expect("the image of the media opened again"),
  );
  let differ = again.iter().zip(&there).filter(|(now, was)| now != was).count();
  assert_eq!(
    (again.len(), differ),
    (there.len(), 0),
    "the frame shown after the reset is the frame that was shown before it"
  );
  // And it plays on.
  let shown = frames_of(&media.frame().unwrap());
  let count = shown.count();
  media.play();
  assert!(
    becomes(|| shown.count() > count + 5),
    "it does not play after the reset"
  );
  media.pause();
  assert_eq!(media.state().error, None);
  crate::images::set_card_frames_adapter(None);
}
