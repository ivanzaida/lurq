//! The frames of a video that stay on the graphics card.
//!
//! What decodes a video, lurq's own media or an app's decoder, keeps its frames in a few textures of the card,
//! each shared by a handle any Direct3D device on the same card opens. It says which of them holds the frame to
//! show ([`CardFrames::show`]); what draws opens the handles once and draws that texture. A frame is never in
//! the memory of the program.
//!
//! A frame may say more than which texture it is in: the part of the texture that is the picture
//! ([`CardFrame::crop`]), the value of a fence that tells when it is all written ([`CardFrame::ready_at`]),
//! something to be let go of once the card has finished drawing it ([`CardFrame::lease`]), which is how a
//! decoder knows a texture is its own again, and how its gray and color are made red, green and blue
//! ([`CardFrame::colors`]).
//!
//! Where the card that draws is not the decoder's (a laptop with two), it cannot open the textures. They are
//! opened on the decoder's card then, and each frame is copied through memory: it is still shown, at the price
//! of a copy, and [`CardFrames::way`] says so.

use std::{
  any::Any,
  sync::{
    Arc,
    atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering},
  },
};

use parking_lot::{Mutex, RwLock};

use super::YuvColors;

/// A texture of the card that holds frames, by the handle it is shared by
/// (`IDXGIResource1::CreateSharedHandle`, or `ID3D12Device::CreateSharedHandle`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardSurface {
  /// One NV12 texture: the gray, and under it the color at half the size. Drawn where it is under
  /// Direct3D 12; under Vulkan on some cards alone, which [`card_frames_nv12`] says.
  Nv12(isize),
  /// Two textures: the gray (`R8`), and the color at half the size (`R8G8`), as a decoder gives them that
  /// cannot write one NV12 texture.
  Planes { gray: isize, color: isize },
  /// One texture of red, green, blue and alpha (`R8G8B8A8` or `B8G8R8A8`), already converted on the card.
  Rgba(isize),
}

impl CardSurface {
  /// Whether a surface of this kind is drawn as red, green and blue, and not converted from gray and color.
  pub fn is_rgba(&self) -> bool {
    matches!(self, Self::Rgba(_))
  }

  /// The handles of the surface: its only one, or the gray's and the color's.
  pub fn handles(&self) -> (isize, Option<isize>) {
    match *self {
      Self::Nv12(handle) | Self::Rgba(handle) => (handle, None),
      Self::Planes { gray, color } => (gray, Some(color)),
    }
  }

  fn same_kind(&self, other: &Self) -> bool {
    std::mem::discriminant(self) == std::mem::discriminant(other)
  }
}

/// The part of a texture that is the picture, in the texture's own pixels: a decoder's texture is as large as
/// what it codes (1920 by 1088), and the picture a part of it (1920 by 1080).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CardCrop {
  pub left: u32,
  pub top: u32,
  pub width: u32,
  pub height: u32,
}

/// Where in a texture `wide` by `high` the part `min` to `max` of a picture is, both as parts of one: the picture
/// may be a part of its texture, the part `crop` says, or without it the part at the top left corner that is as
/// large as `picture`. `rgba` says the texture is of red, green and blue, and not of gray and color.
#[cfg(any(feature = "wgpu", feature = "dx12"))]
pub(crate) fn part_of(
  crop: Option<CardCrop>,
  picture: (u32, u32),
  (wide, high): (u32, u32),
  rgba: bool,
  min: [f32; 2],
  max: [f32; 2],
) -> ([f32; 2], [f32; 2]) {
  let crop = crop.unwrap_or(CardCrop {
    left: 0,
    top: 0,
    width: picture.0.min(wide),
    height: picture.1.min(high),
  });
  if (crop.left, crop.top, crop.width, crop.height) == (0, 0, wide, high) || wide == 0 || high == 0 {
    return (min, max);
  }
  // An edge of the picture inside the texture is drawn from a little within it: a sample at the very edge
  // would be mixed with what lies past it, which is no part of the picture. The color is at half the size,
  // and so a whole pixel is kept from.
  let within = if rgba { 0.5 } else { 1.0 };
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

/// What is let go of when the card has finished with a frame.
pub type CardLease = Arc<dyn Any + Send + Sync>;

/// A frame to show: the surface it is in, and what else is known of it.
#[derive(Clone)]
pub struct CardFrame {
  pub(crate) surface: usize,
  pub(crate) crop: Option<CardCrop>,
  pub(crate) ready: Option<u64>,
  pub(crate) lease: Option<CardLease>,
  pub(crate) colors: YuvColors,
}

impl CardFrame {
  /// The frame in the surface numbered `surface`, all of it the picture, written already.
  pub fn of(surface: usize) -> Self {
    Self {
      surface,
      crop: None,
      ready: None,
      lease: None,
      colors: YuvColors::BT709,
    }
  }

  /// The part of the surface that is the picture. Without it the picture is the part at the surface's top
  /// left corner that is as large as the image.
  pub fn crop(mut self, crop: CardCrop) -> Self {
    self.crop = Some(crop);
    self
  }

  /// The value the frames' fence has once this frame is all written. Until then what draws shows the frame
  /// before it, and waits for nothing. It takes a fence given to the frames ([`CardFrames::with_fence`]).
  pub fn ready_at(mut self, value: u64) -> Self {
    self.ready = Some(value);
    self
  }

  /// Something that is dropped once the card has finished with this frame: when another frame has taken its
  /// place and everything that drew it is done, or at once where it was never drawn. A decoder puts here what
  /// gives the surface back to it.
  pub fn lease(mut self, lease: impl Any + Send + Sync) -> Self {
    self.lease = Some(Arc::new(lease));
    self
  }

  /// How the frame's gray and color are red, green and blue: the matrix it was made with and the range of its
  /// samples, as the decoder says them (`MF_MT_YUV_MATRIX` and `MF_MT_VIDEO_NOMINAL_RANGE` of Media
  /// Foundation's). Without it a frame is BT.709 of the video range. A frame of red, green and blue
  /// ([`CardSurface::Rgba`]) has no use for it.
  pub fn colors(mut self, colors: YuvColors) -> Self {
    self.colors = colors;
    self
  }

  /// The number of the surface the frame is in.
  pub fn surface(&self) -> usize {
    self.surface
  }
}

/// How a video's frames come to the screen, as what draws them found when it opened them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardFramesWay {
  /// Drawn from where the decoder put them: nothing is copied.
  Card,
  /// The card that draws does not open the surfaces, and another does: the decoder is on another card. Each
  /// frame is copied through memory to the card that draws.
  Memory,
  /// No card opens the surfaces, or they are not what they are said to be: nothing is drawn.
  Nowhere,
}

/// The frames of a video that stay on the graphics card: a few textures, of which one holds the frame to show.
///
/// Made into an image with [`NativeImageData::from_card_frames`](super::NativeImageData::from_card_frames),
/// which is another picture with each frame shown: it is drawn again by itself.
#[derive(Clone)]
pub struct CardFrames {
  state: Arc<State>,
}

struct State {
  surfaces: Vec<CardSurface>,
  fence: Option<isize>,
  /// The frame to show, and how many were shown before it.
  shown: Mutex<Option<(u64, CardFrame)>>,
  count: AtomicU64,
  wakers: super::FrameWakers,
  /// The way the frames come to the screen: nothing before they are opened, and then a [`CardFramesWay`] and
  /// one.
  way: AtomicU8,
}

impl Drop for State {
  fn drop(&mut self) {
    let close = |handle: isize| {
      // SAFETY: the handles were given to `CardFrames::new` to keep and to close.
      unsafe {
        let _ = windows::Win32::Foundation::CloseHandle(windows::Win32::Foundation::HANDLE(handle as _));
      }
    };
    for surface in &self.surfaces {
      let (first, second) = surface.handles();
      close(first);
      if let Some(second) = second {
        close(second);
      }
    }
    if let Some(fence) = self.fence {
      close(fence);
    }
  }
}

impl CardFrames {
  /// The frames in `surfaces`, none of them shown yet.
  ///
  /// # Safety
  ///
  /// Each handle is an NT handle of a shared texture of the kind its surface says, all of one kind and one
  /// size. They are closed when the last of this is dropped.
  pub unsafe fn new(surfaces: Vec<CardSurface>) -> Self {
    Self::made(surfaces, None)
  }

  /// The frames in `surfaces`, with the fence that tells when a frame is all written
  /// ([`CardFrame::ready_at`]).
  ///
  /// # Safety
  ///
  /// As [`new`](Self::new), and `fence` is an NT handle of a shared fence (`ID3D11Fence::CreateSharedHandle`,
  /// or `ID3D12Device::CreateSharedHandle` of a fence), closed with the others.
  pub unsafe fn with_fence(surfaces: Vec<CardSurface>, fence: isize) -> Self {
    Self::made(surfaces, Some(fence))
  }

  fn made(surfaces: Vec<CardSurface>, fence: Option<isize>) -> Self {
    assert!(
      surfaces.windows(2).all(|pair| pair[0].same_kind(&pair[1])),
      "the surfaces of a video's frames are all of one kind"
    );
    Self {
      state: Arc::new(State {
        surfaces,
        fence,
        shown: Mutex::new(None),
        count: AtomicU64::new(0),
        wakers: super::FrameWakers::default(),
        way: AtomicU8::new(0),
      }),
    }
  }

  /// The surfaces, in the order of their numbers.
  pub fn surfaces(&self) -> &[CardSurface] {
    &self.state.surfaces
  }

  /// The handle of the fence, where one was given.
  pub fn fence(&self) -> Option<isize> {
    self.state.fence
  }

  /// Whether the frames are drawn as red, green and blue.
  pub fn is_rgba(&self) -> bool {
    self.state.surfaces.first().is_some_and(CardSurface::is_rgba)
  }

  /// Says `frame` is the one to show. The frame it takes the place of is let go of at once where nothing drew
  /// it.
  pub fn show(&self, frame: CardFrame) {
    let count = self.state.count.fetch_add(1, Ordering::AcqRel) + 1;
    let before = self.state.shown.lock().replace((count, frame));
    // Its lease may run code of the decoder's: not under the lock.
    drop(before);
    self.state.wakers.wake();
  }

  pub(crate) fn wakers(&self) -> &super::FrameWakers {
    &self.state.wakers
  }

  /// The frame to show, where one was shown.
  pub fn shown(&self) -> Option<CardFrame> {
    self.state.shown.lock().as_ref().map(|(_, frame)| frame.clone())
  }

  /// How many frames were shown: the version of an image of these frames.
  pub(crate) fn count(&self) -> u64 {
    self.state.count.load(Ordering::Acquire)
  }

  /// The frame to show and which one it is, counted from the first: what draws tells by the count that the
  /// frame is another one.
  pub(crate) fn latest(&self) -> Option<(u64, CardFrame)> {
    self.state.shown.lock().clone()
  }

  /// How the frames come to the screen, once what draws them has opened them: `None` before that. A decoder
  /// that is told [`Memory`](CardFramesWay::Memory) may decode on the card that draws
  /// ([`card_frames_adapter`]) to have the copy gone, and one that is told
  /// [`Nowhere`](CardFramesWay::Nowhere) gives its frames another way.
  pub fn way(&self) -> Option<CardFramesWay> {
    match self.state.way.load(Ordering::Acquire) {
      1 => Some(CardFramesWay::Card),
      2 => Some(CardFramesWay::Memory),
      3 => Some(CardFramesWay::Nowhere),
      _ => None,
    }
  }

  /// What draws the frames says how it found them.
  pub(crate) fn found(&self, way: CardFramesWay) {
    let way = match way {
      CardFramesWay::Card => 1,
      CardFramesWay::Memory => 2,
      CardFramesWay::Nowhere => 3,
    };
    self.state.way.store(way, Ordering::Release);
  }

  /// Whether `other` is these same frames.
  pub(crate) fn same(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.state, &other.state)
  }
}

/// The graphics card what draws frames of the card draws on, by its identifier: what decodes a video decodes on
/// that card, so that a frame is copied on it and nowhere else. `None` where nothing that draws them has
/// started: a frame then goes through memory.
static ADAPTER: RwLock<Option<(u32, i32)>> = RwLock::new(None);

/// Says which card draws [`CardFrames`], by the two halves of its identifier (its LUID), or that none does. A
/// renderer that draws them says so once it has its device.
pub fn set_card_frames_adapter(adapter: Option<(u32, i32)>) {
  *ADAPTER.write() = adapter;
}

/// The card that draws [`CardFrames`], where one does: a decoder makes its device on this card, and gives
/// memory where there is none.
pub fn card_frames_adapter() -> Option<(u32, i32)> {
  *ADAPTER.read()
}

/// Whether what draws [`CardFrames`] draws one NV12 texture where it is.
static NV12: AtomicBool = AtomicBool::new(true);

/// Says whether what draws [`CardFrames`] draws one NV12 texture ([`CardSurface::Nv12`]) where it is. A
/// renderer says so with the card it draws on.
pub fn set_card_frames_nv12(drawn: bool) {
  NV12.store(drawn, Ordering::Release);
}

/// Whether one NV12 texture ([`CardSurface::Nv12`]) is drawn where the decoder put it. Under Direct3D 12 it
/// is, on every card. Under Vulkan it is on a card of AMD's and on no other: on a card of NVIDIA's Vulkan
/// and Direct3D 11 do not agree where such a texture keeps its color, and the device is lost over it, so
/// its frames are copied through memory there ([`CardFramesWay::Memory`]). Where this says no, a decoder
/// gives gray and color apart ([`CardSurface::Planes`]) or red, green and blue ([`CardSurface::Rgba`]),
/// which are drawn where they are under Vulkan on every card, or the app draws through Direct3D 12.
pub fn card_frames_nv12() -> bool {
  NV12.load(Ordering::Acquire)
}
