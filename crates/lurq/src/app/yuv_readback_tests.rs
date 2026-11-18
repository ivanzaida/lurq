// Frames of gray and color through the real render engines, read back from a hidden window of the test's own:
// a picture of three planes (I420) drawn whole, and the frames of a card in the matrix and the range their
// decoder says (`CardFrame::colors`), each against the colors the standards give.
//
// cargo test -p lurq --features wgpu,dx12,raster,screenshot --lib yuv_readback -- --ignored --test-threads=1

use std::sync::Arc;

use windows::{
  Win32::{
    Foundation::{HMODULE, LUID},
    Graphics::{
      Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_UNKNOWN, D3D_FEATURE_LEVEL_11_0},
      Direct3D11::{
        D3D11_BIND_SHADER_RESOURCE, D3D11_CREATE_DEVICE_FLAG, D3D11_QUERY_DESC, D3D11_QUERY_EVENT,
        D3D11_RESOURCE_MISC_SHARED, D3D11_RESOURCE_MISC_SHARED_NTHANDLE, D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC,
        D3D11_USAGE_DEFAULT, D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D,
      },
      Dxgi::{
        Common::{DXGI_FORMAT, DXGI_FORMAT_R8_UNORM, DXGI_FORMAT_R8G8_UNORM, DXGI_SAMPLE_DESC},
        CreateDXGIFactory1, DXGI_SHARED_RESOURCE_READ, IDXGIAdapter, IDXGIFactory4, IDXGIResource1,
      },
    },
  },
  core::{Interface, PCWSTR},
};

use super::readback_window;
use crate::{
  app::render_engine::{CapturedFrame, RenderEngine},
  images::{CardFrame, CardFrames, CardSurface, ImageCmd, ImagePixelFormat, NativeImageData, YuvColors, YuvMatrix},
  layout::{
    quad::ClipRect,
    render_list::{GlyphAtlas, RenderList},
  },
  node::color::Color,
};

/// Each picture is drawn in a square of this side.
const SIDE: u32 = 64;

/// R'G'B' of Y'CbCr samples of 0 to 255 by the standard's own weights of red and blue, `kr` and `kb`, in the
/// full range or the video one: what the engines must draw, as bytes of sRGB.
fn standard(samples: [u8; 3], colors: YuvColors) -> [u8; 3] {
  let (kr, kb) = match colors.matrix {
    YuvMatrix::Bt709 => (0.2126, 0.0722),
    YuvMatrix::Bt601 => (0.299, 0.114),
    YuvMatrix::Bt2020 => (0.2627, 0.0593),
  };
  let [y, cb, cr] = samples.map(|sample| f32::from(sample) / 255.0);
  let (y, cb, cr) = if colors.full_range {
    (y, cb - 0.5, cr - 0.5)
  } else {
    (
      (y - 16.0 / 255.0) * 255.0 / 219.0,
      (cb - 0.5) * 255.0 / 224.0,
      (cr - 0.5) * 255.0 / 224.0,
    )
  };
  let red = y + 2.0 * (1.0 - kr) * cr;
  let blue = y + 2.0 * (1.0 - kb) * cb;
  let green = (y - kr * red - kb * blue) / (1.0 - kr - kb);
  [red, green, blue].map(|channel| (channel.clamp(0.0, 1.0) * 255.0).round() as u8)
}

fn image(x: u32, y: u32, id: u64) -> ImageCmd {
  ImageCmd {
    order: 0,
    x: x as f32,
    y: y as f32,
    width: SIDE as f32,
    height: SIDE as f32,
    image_id: id,
    frame_index: 0,
    version: 1,
    data: Arc::new(Vec::new()),
    animation_frames: None,
    native: None,
    image_width: 0,
    image_height: 0,
    image_format: ImagePixelFormat::Rgba8,
    uv_min: [0.0, 0.0],
    uv_max: [1.0, 1.0],
    radii: [0.0; 4],
    transform: [1.0, 0.0, 0.0, 1.0],
    transform_origin: [0.0; 2],
    clip: ClipRect::default(),
    opacity: 1.0,
  }
}

fn list(images: Vec<ImageCmd>) -> RenderList {
  RenderList {
    clear_color: Color::new(0, 0, 0, 255),
    rects: Vec::new(),
    glyphs: Vec::new(),
    images,
    layers: Vec::new(),
    backdrops: Vec::new(),
    atlas: GlyphAtlas {
      data: Arc::from([0_u8; 4].as_slice()),
      width: 1,
      height: 1,
      version: 1,
      dirty_rects: Arc::from(Vec::new()),
      dirty_from_version: 0,
    },
  }
}

fn pixel(frame: &CapturedFrame, x: u32, y: u32) -> [u8; 3] {
  let at = ((y * frame.width + x) * 4) as usize;
  [frame.rgba[at], frame.rgba[at + 1], frame.rgba[at + 2]]
}

fn near(got: [u8; 3], wanted: [u8; 3]) -> bool {
  got.iter().zip(wanted).all(|(got, wanted)| got.abs_diff(wanted) <= 2)
}

/// Four quarters of a picture, each of its own samples of gray, blue and red: a saturated orange, a dark blue,
/// a green, and gray.
const QUARTERS: [[u8; 3]; 4] = [[150, 80, 190], [50, 180, 110], [140, 60, 50], [128, 128, 128]];

/// A picture of three planes, 8 by 8, of the four quarters: the gray, then the blue, then the red, each plane
/// whole before the next.
fn i420_quarters() -> Vec<u8> {
  let (wide, high) = (8_usize, 8_usize);
  let quarter =
    |x: usize, y: usize, of_w: usize, of_h: usize| usize::from(x >= of_w / 2) + 2 * usize::from(y >= of_h / 2);
  let mut bytes = Vec::with_capacity(wide * high * 3 / 2);
  for y in 0..high {
    for x in 0..wide {
      bytes.push(QUARTERS[quarter(x, y, wide, high)][0]);
    }
  }
  for channel in [1, 2] {
    for y in 0..high / 2 {
      for x in 0..wide / 2 {
        bytes.push(QUARTERS[quarter(x, y, wide / 2, high / 2)][channel]);
      }
    }
  }
  bytes
}

fn assert_draws_i420_whole(engine: &mut dyn RenderEngine, backend: &str) {
  let mut picture = image(0, 0, 1);
  picture.data = Arc::new(i420_quarters());
  picture.image_width = 8;
  picture.image_height = 8;
  picture.image_format = ImagePixelFormat::I420;
  // Twice: the second frame draws from the textures the first one made, and the third after a new frame.
  let mut next = picture.clone();
  next.version = 2;
  let lists = [list(vec![picture.clone()]), list(vec![picture]), list(vec![next])];
  let frames = readback_window::capture_each(engine, &lists.iter().collect::<Vec<_>>(), SIDE, SIDE);
  for frame in &frames {
    for (at, samples) in QUARTERS.into_iter().enumerate() {
      // The middle of each quarter, where neither plane is mixed with another quarter's samples.
      let (x, y) = (
        SIDE / 4 + SIDE / 2 * (at as u32 % 2),
        SIDE / 4 + SIDE / 2 * (at as u32 / 2),
      );
      let (got, wanted) = (pixel(frame, x, y), standard(samples, YuvColors::BT709));
      assert!(
        near(got, wanted),
        "{backend}: quarter {at} of an I420 picture, {samples:?}, is {got:?}; BT.709 of the video range gives \
         {wanted:?}"
      );
    }
  }
}

/// A device of Direct3D 11 on the card the engine draws frames of a card on, as a decoder's is.
fn decoder() -> (ID3D11Device, ID3D11DeviceContext) {
  // SAFETY: a device of this test's own is made, on the engine's card where it said one.
  unsafe {
    let card: Option<IDXGIAdapter> = crate::images::card_frames_adapter().map(|(low, high)| {
      let cards: IDXGIFactory4 = CreateDXGIFactory1().unwrap();
      cards
        .EnumAdapterByLuid(LUID {
          LowPart: low,
          HighPart: high,
        })
        .unwrap()
    });
    let (mut device, mut context) = (None, None);
    D3D11CreateDevice(
      card.as_ref(),
      if card.is_some() {
        D3D_DRIVER_TYPE_UNKNOWN
      } else {
        D3D_DRIVER_TYPE_HARDWARE
      },
      HMODULE::default(),
      D3D11_CREATE_DEVICE_FLAG(0),
      Some(&[D3D_FEATURE_LEVEL_11_0]),
      D3D11_SDK_VERSION,
      Some(&mut device),
      None,
      Some(&mut context),
    )
    .unwrap();
    (device.unwrap(), context.unwrap())
  }
}

/// A shared texture of `format`, `side` square, all of `sample`, and a handle of it for the frames to keep.
fn written(
  device: &ID3D11Device,
  context: &ID3D11DeviceContext,
  format: DXGI_FORMAT,
  side: u32,
  sample: &[u8],
) -> isize {
  let like = D3D11_TEXTURE2D_DESC {
    Width: side,
    Height: side,
    MipLevels: 1,
    ArraySize: 1,
    Format: format,
    SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
    Usage: D3D11_USAGE_DEFAULT,
    BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
    CPUAccessFlags: 0,
    MiscFlags: (D3D11_RESOURCE_MISC_SHARED_NTHANDLE.0 | D3D11_RESOURCE_MISC_SHARED.0) as u32,
  };
  let bytes = sample.repeat((side * side) as usize);
  // SAFETY: a texture of this test's own is made, written whole and shared.
  unsafe {
    let mut texture: Option<ID3D11Texture2D> = None;
    device.CreateTexture2D(&like, None, Some(&mut texture)).unwrap();
    let texture = texture.unwrap();
    context.UpdateSubresource(&texture, 0, None, bytes.as_ptr().cast(), side * sample.len() as u32, 0);
    texture
      .cast::<IDXGIResource1>()
      .unwrap()
      .CreateSharedHandle(None, DXGI_SHARED_RESOURCE_READ.0, PCWSTR::null())
      .unwrap()
      .0 as isize
  }
}

/// Waits until the device has done what it was told: what a decoder with no fence does before it shows a frame.
fn settle(device: &ID3D11Device, context: &ID3D11DeviceContext) {
  // SAFETY: a query of this test's own is made and asked.
  unsafe {
    let mut done = None;
    device
      .CreateQuery(
        &D3D11_QUERY_DESC {
          Query: D3D11_QUERY_EVENT,
          MiscFlags: 0,
        },
        Some(&mut done),
      )
      .unwrap();
    let done = done.unwrap();
    context.End(&done);
    context.Flush();
    let started = std::time::Instant::now();
    loop {
      let mut finished = windows::core::BOOL(0);
      let asked = context.GetData(
        &done,
        Some((&raw mut finished).cast()),
        std::mem::size_of::<windows::core::BOOL>() as u32,
        0,
      );
      if asked.is_err() || finished.as_bool() || started.elapsed().as_secs() > 2 {
        return;
      }
      std::thread::yield_now();
    }
  }
}

fn assert_draws_card_frames_in_their_colors(engine: &mut dyn RenderEngine, backend: &str) {
  // The engine starts and says on which card it draws frames of a card.
  let window = readback_window::Frames::new(engine, SIDE * 4, SIDE);
  window.capture(engine, &list(Vec::new()));
  let (device, context) = decoder();
  // One frame of each matrix and range, the same samples in each: what differs is what the decoder said.
  let samples = QUARTERS[0];
  let cases = [
    YuvColors::BT709,
    YuvColors::new(YuvMatrix::Bt601),
    YuvColors::new(YuvMatrix::Bt2020),
    YuvColors::new(YuvMatrix::Bt601).full_range(),
  ];
  let wanted = cases.map(|colors| standard(samples, colors));
  assert!(
    wanted.windows(2).all(|pair| !near(pair[0], pair[1])),
    "the cases are told apart: {wanted:?}"
  );
  let mut images = Vec::new();
  let mut kept = Vec::new();
  for (at, colors) in cases.into_iter().enumerate() {
    let gray = written(&device, &context, DXGI_FORMAT_R8_UNORM, 8, &samples[..1]);
    let color = written(&device, &context, DXGI_FORMAT_R8G8_UNORM, 4, &samples[1..]);
    // SAFETY: the handles are of shared textures of one kind and size, made here and given away.
    let frames = unsafe { CardFrames::new(vec![CardSurface::Planes { gray, color }]) };
    settle(&device, &context);
    frames.show(CardFrame::of(0).colors(colors));
    let native = NativeImageData::from_card_frames(8, 8, frames.clone());
    let mut drawn = image(SIDE * at as u32, 0, native.id());
    drawn.image_width = 8;
    drawn.image_height = 8;
    drawn.image_format = native.format();
    drawn.version = native.version();
    drawn.native = Some(native);
    images.push(drawn);
    kept.push(frames);
  }
  let scene = list(images);
  // The frames are opened at the first frame of the engine's, and drawn from the next one at the latest.
  let frames: Vec<_> = (0..3).map(|_| window.capture(engine, &scene)).collect();
  let frame = frames.last().unwrap();
  for (at, (colors, wanted)) in cases.into_iter().zip(wanted).enumerate() {
    let got = pixel(frame, SIDE * at as u32 + SIDE / 2, SIDE / 2);
    assert!(
      near(got, wanted),
      "{backend}: a frame of a card of {colors:?}, samples {samples:?}, is {got:?}; the standard gives {wanted:?}"
    );
  }
  drop(kept);
}

#[cfg(feature = "wgpu")]
#[test]
#[ignore = "requires a Windows desktop and a GPU adapter; creates a hidden window"]
fn wgpu_draws_a_picture_of_three_planes_whole() {
  let mut engine = crate::app::wgpu_render::WgpuRenderEngine::new();
  assert_draws_i420_whole(&mut engine, "wgpu");
}

#[cfg(feature = "dx12")]
#[test]
#[ignore = "requires a Windows desktop and a DX12 adapter; creates a hidden window"]
fn dx12_draws_a_picture_of_three_planes_whole() {
  let mut engine = crate::app::dx12_render::Dx12RenderEngine::new();
  assert_draws_i420_whole(&mut engine, "dx12");
}

#[cfg(feature = "wgpu")]
#[test]
#[ignore = "requires a Windows desktop and a GPU adapter; creates a hidden window"]
fn wgpu_draws_the_frames_of_a_card_in_the_colors_their_decoder_says() {
  let mut engine = crate::app::wgpu_render::WgpuRenderEngine::new();
  assert_draws_card_frames_in_their_colors(&mut engine, "wgpu");
}

#[cfg(feature = "dx12")]
#[test]
#[ignore = "requires a Windows desktop and a DX12 adapter; creates a hidden window"]
fn dx12_draws_the_frames_of_a_card_in_the_colors_their_decoder_says() {
  let mut engine = crate::app::dx12_render::Dx12RenderEngine::new();
  assert_draws_card_frames_in_their_colors(&mut engine, "dx12");
}
