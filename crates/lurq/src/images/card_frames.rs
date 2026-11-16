//! The frames of a video that stay on the graphics card.
//!
//! What decodes a video, lurq's own media or an app's decoder, keeps its frames in a few textures of the card,
//! each shared by a handle any Direct3D device on the same card opens. It says which of them holds the frame to
//! show ([`CardFrames::show`]); what draws opens the handles once and draws that texture. A frame is never in
//! the memory of the program.
//!
//! A frame may say more than which texture it is in: the part of the texture that is the picture
//! ([`CardFrame::crop`]), the value of a fence that tells when it is all written ([`CardFrame::ready_at`]),
//! and something to be let go of once the card has finished drawing it ([`CardFrame::lease`]), which is how a
//! decoder knows a texture is its own again.

use std::{
  any::Any,
  sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
  },
};

use parking_lot::{Mutex, RwLock};

/// A texture of the card that holds frames, by the handle it is shared by
/// (`IDXGIResource1::CreateSharedHandle`, or `ID3D12Device::CreateSharedHandle`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardSurface {
  /// One NV12 texture: the gray, and under it the color at half the size.
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

/// What is let go of when the card has finished with a frame.
pub type CardLease = Arc<dyn Any + Send + Sync>;

/// A frame to show: the surface it is in, and what else is known of it.
#[derive(Clone)]
pub struct CardFrame {
  pub(crate) surface: usize,
  pub(crate) crop: Option<CardCrop>,
  pub(crate) ready: Option<u64>,
  pub(crate) lease: Option<CardLease>,
}

impl CardFrame {
  /// The frame in the surface numbered `surface`, all of it the picture, written already.
  pub fn of(surface: usize) -> Self {
    Self {
      surface,
      crop: None,
      ready: None,
      lease: None,
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

  /// The number of the surface the frame is in.
  pub fn surface(&self) -> usize {
    self.surface
  }
}

/// The frames of a video that stay on the graphics card: a few textures, of which one holds the frame to show.
///
/// Made into an image with [`NativeImageData::from_card_frames`](super::NativeImageData::from_card_frames).
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
  }

  /// The frame to show, where one was shown.
  pub fn shown(&self) -> Option<CardFrame> {
    self.state.shown.lock().as_ref().map(|(_, frame)| frame.clone())
  }

  /// The frame to show and which one it is, counted from the first: what draws tells by the count that the
  /// frame is another one.
  pub(crate) fn latest(&self) -> Option<(u64, CardFrame)> {
    self.state.shown.lock().clone()
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
