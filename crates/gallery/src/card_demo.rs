//! What stands for a decoder that is not lurq's, on the Video Player page: a thread with a Direct3D 11 device of
//! its own, on the card lurq draws on, that writes a moving test picture into textures of that card and gives
//! them to lurq to draw (`lurq::images::CardFrames`). A real decoder writes its textures on the card; this one
//! sends each picture to its texture from memory, which is its own business: from the texture on, the frame is
//! lurq's as a real one is, and is never read back.
//!
//! Its textures are as large as what a codec codes (192 rows), and the picture is a part of them (180 rows): the
//! rows past the picture are bright pink, and none of it shows where the picture's part is drawn right. Each
//! frame is shown with the value its fence will have once it is written, and with a lease that gives its
//! texture back when lurq has finished with it; a frame for which no texture is free is skipped.
//!
//! One of the decoders is on another card than the one lurq draws on, as a decoder is on a machine with two:
//! the system's card of software stands for the other card. lurq's device opens nothing of it, and its frames
//! come through memory.

use std::{
  sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
  },
  time::Duration,
};

use lurq::images::{
  CardCrop, CardFrame, CardFrames, CardFramesWay, CardSurface, ImageData, NativeImageData, card_frames_adapter,
};
use windows::{
  Win32::{
    Foundation::{HMODULE, LUID},
    Graphics::{
      Direct3D::{D3D_DRIVER_TYPE_UNKNOWN, D3D_DRIVER_TYPE_WARP, D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_11_1},
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
  core::{Interface, PCWSTR, Result},
};

/// The picture's size, and the rows of the textures it is a part of.
pub const WIDE: u32 = 320;
pub const HIGH: u32 = 180;
const CODED: u32 = 192;
/// How many textures the decoder has to write in.
const SURFACES: usize = 4;
/// Between two frames: thirty a second.
const FRAME: Duration = Duration::from_millis(33);

/// How the decoder gives a frame.
#[derive(Clone, Copy)]
pub enum Kind {
  /// One NV12 texture.
  Nv12,
  /// A texture of gray and one of color.
  Planes,
  /// One texture of red, green and blue.
  Rgba,
  /// A texture of gray and one of color, of a decoder on another card than the one lurq draws on.
  OtherCard,
}

/// A decoder that runs, and the image of its frames. It stops when this is dropped.
pub struct Decoded {
  pub image: ImageData,
  frames: CardFrames,
  stop: Arc<AtomicBool>,
}

impl Decoded {
  /// How the frames come to the screen, once lurq has opened them.
  pub fn way(&self) -> Option<CardFramesWay> {
    self.frames.way()
  }
}

impl Drop for Decoded {
  fn drop(&mut self) {
    self.stop.store(true, Ordering::Release);
  }
}

/// Starts a decoder that gives its frames as `kind`. `None` where nothing draws frames of the card here (another
/// renderer, a card without NV12 textures), or the card does not make what it needs.
pub fn start(kind: Kind) -> Option<Decoded> {
  let adapter = card_frames_adapter()?;
  let stop = Arc::new(AtomicBool::new(false));
  let (told, made) = mpsc::channel();
  let stopped = stop.clone();
  std::thread::Builder::new()
    .name("gallery-card-decoder".to_owned())
    .spawn(move || {
      // SAFETY: the device, its textures and its fence are this thread's own, made and used here alone.
      let _ = unsafe { run(kind, adapter, &stopped, &told) };
    })
    .ok()?;
  let (image, frames) = made.recv_timeout(Duration::from_secs(5)).ok()?;
  Some(Decoded { image, frames, stop })
}

/// The textures of one surface: its only one, or the gray's and the color's.
struct Surface {
  first: ID3D11Texture2D,
  second: Option<ID3D11Texture2D>,
}

/// Gives a surface back to the decoder when lurq lets go of the frame that was in it.
struct Lease {
  free: Arc<[AtomicBool; SURFACES]>,
  surface: usize,
}

impl Drop for Lease {
  fn drop(&mut self) {
    self.free[self.surface].store(true, Ordering::Release);
  }
}

unsafe fn texture(
  device: &ID3D11Device,
  format: DXGI_FORMAT,
  width: u32,
  height: u32,
) -> Result<(ID3D11Texture2D, isize)> {
  unsafe {
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
    let mut made = None;
    device.CreateTexture2D(&like, None, Some(&mut made))?;
    let made = made.expect("the texture Direct3D made");
    let handle =
      made
        .cast::<IDXGIResource1>()?
        .CreateSharedHandle(None, DXGI_SHARED_RESOURCE_READ.0, PCWSTR::null())?;
    Ok((made, handle.0 as isize))
  }
}

unsafe fn run(
  kind: Kind,
  adapter: (u32, i32),
  stop: &AtomicBool,
  told: &mpsc::Sender<(ImageData, CardFrames)>,
) -> Result<()> {
  unsafe {
    // The card lurq draws on, or for the decoder on another card the system's card of software.
    let (card, driver) = if matches!(kind, Kind::OtherCard) {
      (None, D3D_DRIVER_TYPE_WARP)
    } else {
      let cards: IDXGIFactory4 = CreateDXGIFactory1()?;
      let card: IDXGIAdapter = cards.EnumAdapterByLuid(LUID {
        LowPart: adapter.0,
        HighPart: adapter.1,
      })?;
      (Some(card), D3D_DRIVER_TYPE_UNKNOWN)
    };
    let (mut device, mut context): (Option<ID3D11Device>, Option<ID3D11DeviceContext>) = (None, None);
    D3D11CreateDevice(
      card.as_ref(),
      driver,
      HMODULE::default(),
      D3D11_CREATE_DEVICE_BGRA_SUPPORT,
      Some(&[D3D_FEATURE_LEVEL_11_1, D3D_FEATURE_LEVEL_11_0]),
      D3D11_SDK_VERSION,
      Some(&mut device),
      None,
      Some(&mut context),
    )?;
    let (device, context) = (
      device.expect("the device Direct3D made"),
      context.expect("the context Direct3D made"),
    );
    let context = context.cast::<ID3D11DeviceContext4>()?;
    let mut fence: Option<ID3D11Fence> = None;
    device
      .cast::<ID3D11Device5>()?
      .CreateFence(0, D3D11_FENCE_FLAG_SHARED, &mut fence)?;
    let fence = fence.expect("the fence Direct3D made");
    const GENERIC_ALL: u32 = 0x1000_0000;
    let fence_handle = fence.CreateSharedHandle(None, GENERIC_ALL, PCWSTR::null())?.0 as isize;

    let (mut surfaces, mut shared) = (Vec::with_capacity(SURFACES), Vec::with_capacity(SURFACES));
    for _ in 0..SURFACES {
      match kind {
        Kind::Nv12 => {
          let (made, handle) = texture(&device, DXGI_FORMAT_NV12, WIDE, CODED)?;
          surfaces.push(Surface {
            first: made,
            second: None,
          });
          shared.push(CardSurface::Nv12(handle));
        }
        Kind::Planes | Kind::OtherCard => {
          let (gray, gray_handle) = texture(&device, DXGI_FORMAT_R8_UNORM, WIDE, CODED)?;
          let (color, color_handle) = texture(&device, DXGI_FORMAT_R8G8_UNORM, WIDE / 2, CODED / 2)?;
          surfaces.push(Surface {
            first: gray,
            second: Some(color),
          });
          shared.push(CardSurface::Planes {
            gray: gray_handle,
            color: color_handle,
          });
        }
        Kind::Rgba => {
          let (made, handle) = texture(&device, DXGI_FORMAT_R8G8B8A8_UNORM, WIDE, CODED)?;
          surfaces.push(Surface {
            first: made,
            second: None,
          });
          shared.push(CardSurface::Rgba(handle));
        }
      }
    }
    // SAFETY: the handles are of textures of one kind and size and of a fence, made to be shared, given away.
    let frames = CardFrames::with_fence(shared, fence_handle);
    let image = NativeImageData::from_card_frames(WIDE, HIGH, frames.clone()).image_data();
    if told.send((image, frames.clone())).is_err() {
      return Ok(());
    }

    let free: Arc<[AtomicBool; SURFACES]> = Arc::new(std::array::from_fn(|_| AtomicBool::new(true)));
    let crop = CardCrop {
      left: 0,
      top: 0,
      width: WIDE,
      height: HIGH,
    };
    let (mut number, mut signalled) = (0_u32, 0_u64);
    while !stop.load(Ordering::Acquire) {
      std::thread::sleep(FRAME);
      number = number.wrapping_add(1);
      // A texture lurq has let go of; with none, this frame is not shown.
      let Some(at) = (0..SURFACES).find(|at| free[*at].swap(false, Ordering::AcqRel)) else {
        continue;
      };
      let surface = &surfaces[at];
      match kind {
        Kind::Nv12 => {
          let mut bytes = gray(number);
          bytes.extend(color(number));
          context.UpdateSubresource(&surface.first, 0, None, bytes.as_ptr().cast(), WIDE, 0);
        }
        Kind::Planes | Kind::OtherCard => {
          let (bytes, tints) = (gray(number), color(number));
          context.UpdateSubresource(&surface.first, 0, None, bytes.as_ptr().cast(), WIDE, 0);
          if let Some(second) = &surface.second {
            context.UpdateSubresource(second, 0, None, tints.as_ptr().cast(), WIDE, 0);
          }
        }
        Kind::Rgba => {
          let bytes = colors(number);
          context.UpdateSubresource(&surface.first, 0, None, bytes.as_ptr().cast(), WIDE * 4, 0);
        }
      }
      signalled += 1;
      context.Signal(&fence, signalled)?;
      context.Flush();
      frames.show(CardFrame::of(at).crop(crop).ready_at(signalled).lease(Lease {
        free: free.clone(),
        surface: at,
      }));
    }
    Ok(())
  }
}

/// Where the bar that moves across the picture is, in frame `number`.
fn bar(number: u32) -> u32 {
  (number * 4) % WIDE
}

/// The gray of frame `number`: slanted bands that move, a bright bar that crosses, and white past the picture.
fn gray(number: u32) -> Vec<u8> {
  let at = bar(number);
  let mut bytes = Vec::with_capacity((WIDE * CODED) as usize);
  for y in 0..CODED {
    for x in 0..WIDE {
      bytes.push(if y >= HIGH {
        235
      } else if x.abs_diff(at) < 6 {
        230
      } else {
        40 + (((x + y + number * 2) % 128) as u8)
      });
    }
  }
  bytes
}

/// The color of frame `number`, one pair for two by two: from blue at the left to red at the right, and pink
/// past the picture.
fn color(_number: u32) -> Vec<u8> {
  let mut bytes = Vec::with_capacity((WIDE * CODED / 2) as usize);
  for y in 0..CODED / 2 {
    for x in 0..WIDE / 2 {
      if y >= HIGH / 2 {
        bytes.extend([200, 220]);
      } else {
        let across = (x * 255 / (WIDE / 2)) as u8;
        bytes.extend([255 - across, across]);
      }
    }
  }
  bytes
}

/// The red, green and blue of frame `number`: the same bands and bar, in colors, and pink past the picture.
fn colors(number: u32) -> Vec<u8> {
  let at = bar(number);
  let mut bytes = Vec::with_capacity((WIDE * CODED * 4) as usize);
  for y in 0..CODED {
    for x in 0..WIDE {
      bytes.extend(if y >= HIGH {
        [255, 0, 255, 255]
      } else if x.abs_diff(at) < 6 {
        [255, 255, 255, 255]
      } else {
        let band = 40 + (((x + y + number * 2) % 128) as u8);
        [band, (y * 255 / HIGH) as u8, 255 - (x * 255 / WIDE) as u8, 255]
      });
    }
  }
  bytes
}
