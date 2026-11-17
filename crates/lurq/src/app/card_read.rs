//! Reading the frames of a video whose decoder is on another card than the one that draws.
//!
//! A texture shared by handle is opened by a device of the card it is on, and by no other: on a machine with two
//! cards a renderer's device may be given handles it cannot open. Here they are opened by a Direct3D 11 device
//! of the card that does open them ([`Readers`]), and each frame is copied on that card into a texture that
//! memory reads ([`Read`]). A renderer sends what is read into textures of its own: the frame is shown, at the
//! price of a copy through memory, where it would not be shown at all.
//!
//! Nothing waits here: a frame is copied once its fence says it is written, and read once the card it is on has
//! copied it, which is asked without waiting in each of the renderer's frames; the frame before it is drawn
//! until then.

use windows::{
  Win32::{
    Foundation::{HANDLE, HMODULE},
    Graphics::{
      Direct3D::{D3D_DRIVER_TYPE_UNKNOWN, D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_11_1},
      Direct3D11::{
        D3D11_CPU_ACCESS_READ, D3D11_CREATE_DEVICE_FLAG, D3D11_MAP_FLAG_DO_NOT_WAIT, D3D11_MAP_READ,
        D3D11_MAPPED_SUBRESOURCE, D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING, D3D11CreateDevice,
        ID3D11Device, ID3D11Device1, ID3D11Device5, ID3D11DeviceContext, ID3D11Fence, ID3D11Texture2D,
      },
      Dxgi::{
        Common::{
          DXGI_FORMAT, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_FORMAT_NV12, DXGI_FORMAT_R8_UNORM, DXGI_FORMAT_R8G8_UNORM,
          DXGI_FORMAT_R8G8B8A8_UNORM,
        },
        CreateDXGIFactory1, DXGI_ERROR_WAS_STILL_DRAWING, IDXGIFactory1,
      },
    },
  },
  core::Interface,
};

use crate::images::{CardFrame, CardFrames, CardSurface};

/// The tests of frames on a card run one at a time, whichever renderer they are of: which card draws is said
/// once for the whole program, and a card has one Direct3D 12 device in it, which a test may remove.
#[cfg(test)]
pub(crate) static ON_THE_CARD: parking_lot::Mutex<()> = parking_lot::Mutex::new(());

/// A Direct3D 11 device of one of the machine's cards, which opens what a decoder on that card shares.
#[derive(Clone)]
struct Reader {
  device: ID3D11Device1,
  context: ID3D11DeviceContext,
}

impl Reader {
  /// The texture behind `handle`, where this card opens it.
  fn texture(&self, handle: isize) -> Option<ID3D11Texture2D> {
    // SAFETY: the handle is one of a shared texture, kept open by the frames this is asked for.
    unsafe { self.device.OpenSharedResource1(HANDLE(handle as _)) }.ok()
  }

  /// The fence behind `handle`, where this card opens it.
  fn fence(&self, handle: isize) -> Option<ID3D11Fence> {
    let mut fence: Option<ID3D11Fence> = None;
    let device = self.device.cast::<ID3D11Device5>().ok()?;
    // SAFETY: the handle is one of a shared fence, kept open by the frames this is asked for.
    unsafe { device.OpenSharedFence(HANDLE(handle as _), &mut fence) }.ok()?;
    fence
  }

  /// A texture of this device that memory reads, as large as `texture` and of its format.
  fn read_of(&self, texture: &ID3D11Texture2D) -> Option<ID3D11Texture2D> {
    let like = D3D11_TEXTURE2D_DESC {
      Usage: D3D11_USAGE_STAGING,
      BindFlags: 0,
      CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
      MiscFlags: 0,
      ..said(texture)
    };
    let mut made = None;
    // SAFETY: a texture of this device's own is made.
    unsafe { self.device.CreateTexture2D(&like, None, Some(&mut made)) }.ok()?;
    made
  }
}

/// What `texture` says of itself.
fn said(texture: &ID3D11Texture2D) -> D3D11_TEXTURE2D_DESC {
  let mut said = D3D11_TEXTURE2D_DESC::default();
  // SAFETY: the description is written into a place of this function's own.
  unsafe { texture.GetDesc(&mut said) };
  said
}

/// A device on each card of the machine, made the first time a renderer's own device opens nothing of a
/// video's frames.
#[derive(Default)]
pub(crate) struct Readers {
  looked: bool,
  cards: Vec<Reader>,
}

impl Readers {
  /// What the texture behind `handle` says of itself, its format and its size, as the card it is on tells.
  /// A device of Vulkan is given a texture by its handle with these said, and cannot ask them of it.
  #[cfg(feature = "wgpu")]
  pub(crate) fn said_of(&mut self, handle: isize) -> Option<(DXGI_FORMAT, u32, u32)> {
    let said = said(&self.that_opens(handle)?.texture(handle)?);
    Some((said.Format, said.Width, said.Height))
  }

  /// The fence behind `fence`, opened by the card that opens the texture behind `handle`: a device of
  /// Vulkan draws the textures and has no way to ask a fence of Direct3D how far it is.
  #[cfg(feature = "wgpu")]
  pub(crate) fn fence_of(&mut self, handle: isize, fence: isize) -> Option<ID3D11Fence> {
    self.that_opens(handle)?.fence(fence)
  }

  /// The card that opens the texture behind `handle`, where one does.
  fn that_opens(&mut self, handle: isize) -> Option<Reader> {
    if !self.looked {
      self.looked = true;
      self.cards = every_card();
    }
    self.cards.iter().find(|card| card.texture(handle).is_some()).cloned()
  }
}

fn every_card() -> Vec<Reader> {
  let mut cards = Vec::new();
  // SAFETY: the system is asked for its cards, and a device is made on each that makes one.
  unsafe {
    let Ok(all) = CreateDXGIFactory1::<IDXGIFactory1>() else {
      return cards;
    };
    for index in 0.. {
      let Ok(card) = all.EnumAdapters1(index) else {
        break;
      };
      let (mut device, mut context): (Option<ID3D11Device>, Option<ID3D11DeviceContext>) = (None, None);
      let made = D3D11CreateDevice(
        &card,
        D3D_DRIVER_TYPE_UNKNOWN,
        HMODULE::default(),
        D3D11_CREATE_DEVICE_FLAG(0),
        Some(&[D3D_FEATURE_LEVEL_11_1, D3D_FEATURE_LEVEL_11_0]),
        D3D11_SDK_VERSION,
        Some(&mut device),
        None,
        Some(&mut context),
      );
      let device = device.and_then(|device| device.cast::<ID3D11Device1>().ok());
      if let (Ok(()), Some(device), Some(context)) = (made, device, context) {
        cards.push(Reader { device, context });
      }
    }
  }
  cards
}

/// The frames of an image that are on another card, read: each of its surfaces opened by a device of that
/// card, and the textures a frame is copied into to be read by memory.
pub(crate) struct Read {
  reader: Reader,
  /// Each surface: its only texture, or the gray's and the color's.
  surfaces: Vec<(ID3D11Texture2D, Option<ID3D11Texture2D>)>,
  /// What memory reads a frame from, as a surface is.
  read: (ID3D11Texture2D, Option<ID3D11Texture2D>),
  fence: Option<ID3D11Fence>,
  /// Whether a surface is one NV12 texture: its gray and its color are read from one.
  nv12: bool,
  /// What a frame is read as: the format and the size of each of its parts.
  parts: Vec<(DXGI_FORMAT, u32, u32)>,
  /// The frame that is being copied, and which one it is of those that were shown.
  copying: Option<(u64, CardFrame)>,
}

impl Read {
  /// Opens each surface of `frames` on the card that opens them. `None` where no card opens them, or a surface
  /// is not what it is said to be or not as the first one is.
  pub(crate) fn open(frames: &CardFrames, readers: &mut Readers) -> Option<Self> {
    let kind = *frames.surfaces().first()?;
    let reader = readers.that_opens(kind.handles().0)?;
    let surfaces = frames
      .surfaces()
      .iter()
      .map(|surface| {
        let (first, second) = surface.handles();
        let second = match second {
          Some(second) => Some(reader.texture(second)?),
          None => None,
        };
        Some((reader.texture(first)?, second))
      })
      .collect::<Option<Vec<_>>>()?;
    let like = |texture: &ID3D11Texture2D| {
      let said = said(texture);
      (said.Format, said.Width, said.Height)
    };
    let (first, second) = surfaces.first()?;
    let (first_like, second_like) = (like(first), second.as_ref().map(like));
    if surfaces
      .iter()
      .any(|(first, second)| like(first) != first_like || second.as_ref().map(like) != second_like)
    {
      return None;
    }
    let (format, wide, high) = first_like;
    let is = |formats: &[DXGI_FORMAT]| formats.contains(&format);
    // What a renderer keeps a frame in: the gray and the color at half the size, or the red, green and blue.
    let parts: Vec<(DXGI_FORMAT, u32, u32)> = match (kind, second_like) {
      (CardSurface::Nv12(_), None) if is(&[DXGI_FORMAT_NV12]) => vec![
        (DXGI_FORMAT_R8_UNORM, wide, high),
        (DXGI_FORMAT_R8G8_UNORM, wide / 2, high / 2),
      ],
      (CardSurface::Planes { .. }, Some((DXGI_FORMAT_R8G8_UNORM, color_wide, color_high)))
        if is(&[DXGI_FORMAT_R8_UNORM]) =>
      {
        vec![
          (DXGI_FORMAT_R8_UNORM, wide, high),
          (DXGI_FORMAT_R8G8_UNORM, color_wide, color_high),
        ]
      }
      (CardSurface::Rgba(_), None) if is(&[DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_FORMAT_B8G8R8A8_UNORM]) => {
        vec![(format, wide, high)]
      }
      _ => return None,
    };
    if parts.iter().any(|&(_, wide, high)| wide == 0 || high == 0) {
      return None;
    }
    let read = (
      reader.read_of(first)?,
      match second {
        Some(second) => Some(reader.read_of(second)?),
        None => None,
      },
    );
    let fence = match frames.fence() {
      Some(handle) => Some(reader.fence(handle)?),
      None => None,
    };
    Some(Self {
      reader,
      surfaces,
      read,
      fence,
      nv12: matches!(kind, CardSurface::Nv12(_)),
      parts,
      copying: None,
    })
  }

  /// What a frame is read as, for a renderer to keep it in textures of its own: the format and the size of the
  /// gray and of the color, or of the red, green and blue alone.
  pub(crate) fn parts(&self) -> &[(DXGI_FORMAT, u32, u32)] {
    &self.parts
  }

  /// The value the decoder's fence has reached, where it has one.
  pub(crate) fn reached(&self) -> Option<u64> {
    // SAFETY: the fence is read.
    self.fence.as_ref().map(|fence| unsafe { fence.GetCompletedValue() })
  }

  /// Whether a frame is being copied: another is not begun until it is read.
  pub(crate) fn is_copying(&self) -> bool {
    self.copying.is_some()
  }

  /// Begins to copy `frame`, the one numbered `count` of those that were shown, on the card it is on. The frame
  /// is kept, and with it its lease, until it is read.
  pub(crate) fn copy(&mut self, count: u64, frame: CardFrame) {
    let Some((first, second)) = self.surfaces.get(frame.surface) else {
      return;
    };
    // SAFETY: the textures are of one device, and each pair of one format and size.
    unsafe {
      self.reader.context.CopyResource(&self.read.0, first);
      if let (Some(into), Some(second)) = (&self.read.1, second) {
        self.reader.context.CopyResource(into, second);
      }
      self.reader.context.Flush();
    }
    self.copying = Some((count, frame));
  }

  /// The frame that was being copied, once its card has copied it: each of its parts is given to `send`, by its
  /// number among [`parts`](Self::parts), as rows in memory and how far one row is from the next, and it is the
  /// frame to draw. `None` while the card has not finished, for which nothing waits, and where no frame is being
  /// copied.
  ///
  /// The rows `send` is given are as many as the part is high, each at least as long as a row of the part, and
  /// are gone when it returns.
  pub(crate) fn arrived(&mut self, mut send: impl FnMut(usize, *const u8, u32)) -> Option<(u64, CardFrame)> {
    self.copying.as_ref()?;
    let context = &self.reader.context;
    let mut mapped: Vec<(&ID3D11Texture2D, D3D11_MAPPED_SUBRESOURCE)> = Vec::with_capacity(2);
    let mut failed = None;
    for texture in [Some(&self.read.0), self.read.1.as_ref()].into_iter().flatten() {
      let mut memory = D3D11_MAPPED_SUBRESOURCE::default();
      // SAFETY: the texture is one that memory reads; where the card has not finished it is not mapped.
      match unsafe {
        context.Map(
          texture,
          0,
          D3D11_MAP_READ,
          D3D11_MAP_FLAG_DO_NOT_WAIT.0 as u32,
          Some(&mut memory),
        )
      } {
        Ok(()) => mapped.push((texture, memory)),
        Err(error) => {
          failed = Some(error.code() != DXGI_ERROR_WAS_STILL_DRAWING);
          break;
        }
      }
    }
    if failed.is_none() {
      let (_, first) = &mapped[0];
      send(0, first.pData.cast(), first.RowPitch);
      if self.nv12 {
        // Under the rows of the gray, in an NV12 texture, are the rows of its color.
        let under = first.RowPitch as usize * self.parts[0].2 as usize;
        // SAFETY: the texture is mapped, and its color begins where its gray ends.
        send(1, unsafe { first.pData.cast::<u8>().add(under) }, first.RowPitch);
      } else if let Some((_, second)) = mapped.get(1) {
        send(1, second.pData.cast(), second.RowPitch);
      }
    }
    for (texture, _) in &mapped {
      // SAFETY: the texture was mapped above.
      unsafe { context.Unmap(*texture, 0) };
    }
    match failed {
      None => self.copying.take(),
      // The card is lost, or the texture: the frame is let go of, and the one before it stays.
      Some(true) => {
        self.copying = None;
        None
      }
      Some(false) => None,
    }
  }
}
