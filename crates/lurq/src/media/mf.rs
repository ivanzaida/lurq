//! The thread of a media on Windows: Media Foundation's source reader decodes the picture to NV12 and the sound
//! to samples, each frame is shown when the media's clock comes to its time, and the sound is given to the
//! system's output, whose own clock the media's then follows.
//!
//! A frame stays on the graphics card where something draws there ([`OnCard`]): the decoder is given a device
//! of the card the renderer draws on, gives its frames as textures of it, and each is copied, on the card, into
//! a texture the renderer has opened. Where nothing draws on a card (a test, another renderer), or the decoder
//! gives memory all the same, a frame is copied through memory once, as it always was.

use std::{
  collections::VecDeque,
  sync::Arc,
  time::{Duration, Instant},
};

use windows::{
  Win32::{
    Foundation::{HMODULE, LUID},
    Graphics::{
      Direct3D::{D3D_DRIVER_TYPE_UNKNOWN, D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_11_1},
      Direct3D11::{
        D3D11_BIND_SHADER_RESOURCE, D3D11_BOX, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_CREATE_DEVICE_VIDEO_SUPPORT,
        D3D11_QUERY_DESC, D3D11_QUERY_EVENT, D3D11_RESOURCE_MISC_SHARED, D3D11_RESOURCE_MISC_SHARED_NTHANDLE,
        D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT, D3D11CreateDevice, ID3D11Device,
        ID3D11DeviceContext, ID3D11Multithread, ID3D11Query, ID3D11Texture2D,
      },
      Dxgi::{
        Common::{DXGI_FORMAT_NV12, DXGI_SAMPLE_DESC},
        CreateDXGIFactory1, DXGI_SHARED_RESOURCE_READ, IDXGIAdapter, IDXGIFactory4, IDXGIResource1,
      },
    },
    Media::MediaFoundation::{
      IMFAttributes, IMFDXGIBuffer, IMFDXGIDeviceManager, IMFMediaType, IMFSample, IMFSourceReader,
      MF_BYTESTREAM_ORIGIN_NAME, MF_E_END_OF_STREAM, MF_E_INVALID_FILE_FORMAT, MF_E_INVALID_FORMAT,
      MF_E_INVALIDMEDIATYPE, MF_E_TOPO_CODEC_NOT_FOUND, MF_E_UNSUPPORTED_BYTESTREAM_TYPE, MF_E_UNSUPPORTED_FORMAT,
      MF_E_UNSUPPORTED_SCHEME, MF_MT_AUDIO_BITS_PER_SAMPLE, MF_MT_AUDIO_NUM_CHANNELS, MF_MT_AUDIO_SAMPLES_PER_SECOND,
      MF_MT_DEFAULT_STRIDE, MF_MT_FRAME_SIZE, MF_MT_MAJOR_TYPE, MF_MT_MINIMUM_DISPLAY_APERTURE, MF_MT_SUBTYPE,
      MF_PD_DURATION, MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS, MF_SOURCE_READER_ALL_STREAMS,
      MF_SOURCE_READER_D3D_MANAGER, MF_SOURCE_READER_ENABLE_ADVANCED_VIDEO_PROCESSING,
      MF_SOURCE_READER_FIRST_AUDIO_STREAM, MF_SOURCE_READER_FIRST_VIDEO_STREAM, MF_SOURCE_READER_MEDIASOURCE,
      MF_SOURCE_READERF_CURRENTMEDIATYPECHANGED, MF_SOURCE_READERF_ENDOFSTREAM, MF_VERSION, MFAudioFormat_Float,
      MFAudioFormat_PCM, MFCreateAttributes, MFCreateDXGIDeviceManager, MFCreateMFByteStreamOnStream,
      MFCreateMediaType, MFCreateSourceReaderFromByteStream, MFCreateSourceReaderFromURL, MFMediaType_Audio,
      MFMediaType_Video, MFSTARTUP_FULL, MFShutdown, MFStartup, MFVideoFormat_NV12,
    },
    System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize, StructuredStorage::PROPVARIANT},
    UI::Shell::SHCreateMemStream,
  },
  core::{BOOL, GUID, HSTRING, Interface, PCWSTR, Result},
};

use super::{
  Command, MediaError, Opened, Shared,
  output::{CHANNELS, Output},
};
use crate::images::{ImageData, NativeImageData, SharedNv12Frames};

/// The streams that are decoded: the first video and the first sound of what is opened.
const VIDEO: u32 = MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32;
const AUDIO: u32 = MF_SOURCE_READER_FIRST_AUDIO_STREAM.0 as u32;
/// A second, in the units Media Foundation counts time in: 100 nanoseconds.
const TICKS: f64 = 10_000_000.0;
/// How long the thread waits for its next frame's time before it looks at what it was told.
const STEP: Duration = Duration::from_millis(10);
/// A frame this late is not shown: the next ones catch up with the clock.
const LATE: f64 = 0.2;
/// How much sound is decoded ahead of what the output holds, in seconds.
const AHEAD: f64 = 0.25;
/// The media's clock is set to the sound's where they are this far apart, in seconds: nearer than that it goes
/// on by itself, which is smoother than the steps the device counts in.
const APART: f64 = 0.03;

/// Runs a media to its end: opens `opened`, decodes, and does what it is told until nothing holds the media.
/// `heard` is a media that is only heard: its picture, where it has one, is not decoded.
pub(super) fn run(shared: &Arc<Shared>, opened: Opened, heard: bool) {
  // SAFETY: COM and Media Foundation are started and stopped on this thread, around everything that uses them.
  unsafe {
    let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    match MFStartup(MF_VERSION, MFSTARTUP_FULL) {
      Ok(()) => {
        // What is only heard has no frames to keep anywhere.
        let mut card = if heard { None } else { OnCard::new() };
        let mut before = None;
        loop {
          match play(shared, opened.clone(), heard, card.take(), before.take()) {
            Ok(Next::Done) => break,
            // Something draws on a card now: it is opened again with that card's device, where it was.
            Ok(Next::Again(found, shown)) => (card, before) = (Some(found), Some(shown)),
            Err(error) => {
              shared.fail(failed(&error));
              break;
            }
          }
        }
        let _ = MFShutdown();
      }
      Err(error) => shared.fail(failed(&error)),
    }
    CoUninitialize();
  }
}

/// What the system's error is, as a media says it: what it does not play is one kind, a file that is not there
/// another, and the rest is kept in the system's words.
fn failed(error: &windows::core::Error) -> MediaError {
  // A file or a folder that is not there, as the system says it of a path.
  const NOT_FOUND: [u32; 2] = [0x8007_0002, 0x8007_0003];
  let code = error.code();
  if [
    MF_E_UNSUPPORTED_BYTESTREAM_TYPE,
    MF_E_UNSUPPORTED_FORMAT,
    MF_E_UNSUPPORTED_SCHEME,
    MF_E_INVALIDMEDIATYPE,
    MF_E_INVALID_FILE_FORMAT,
    MF_E_INVALID_FORMAT,
    MF_E_TOPO_CODEC_NOT_FOUND,
    // What is too short to be anything: the system reads to its end looking for what it is.
    MF_E_END_OF_STREAM,
  ]
  .contains(&code)
  {
    MediaError::Unsupported
  } else if NOT_FOUND.contains(&(code.0 as u32)) {
    MediaError::NotFound
  } else {
    MediaError::Other(Arc::from(error.message()))
  }
}

/// A reader of `opened`, with none of its streams chosen yet. It decodes on the graphics card where the system
/// can.
unsafe fn open(opened: Opened, card: Option<&OnCard>) -> Result<IMFSourceReader> {
  unsafe {
    let mut attributes: Option<IMFAttributes> = None;
    MFCreateAttributes(&mut attributes, 3)?;
    let attributes = attributes.expect("the attributes Media Foundation made");
    // Given a device of the card, the decoder gives its frames as textures of it.
    if let Some(card) = card {
      attributes.SetUnknown(&MF_SOURCE_READER_D3D_MANAGER, &card.manager)?;
    }
    // Its own conversion to the format asked for, and the decoders of the hardware where there are some.
    attributes.SetUINT32(&MF_SOURCE_READER_ENABLE_ADVANCED_VIDEO_PROCESSING, 1)?;
    attributes.SetUINT32(&MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS, 1)?;
    let reader = match opened {
      Opened::File(path) => MFCreateSourceReaderFromURL(&HSTRING::from(path.as_os_str()), &attributes)?,
      Opened::Url(address) => MFCreateSourceReaderFromURL(&HSTRING::from(&*address), &attributes)?,
      Opened::Bytes(bytes, name) => {
        let stream = SHCreateMemStream(Some(&bytes)).ok_or_else(windows::core::Error::from_thread)?;
        let stream = MFCreateMFByteStreamOnStream(&stream)?;
        // Where the bytes came by a name, its ending tells the system which of its readers they are for; with
        // none it tells from the bytes themselves.
        if let (Some(name), Ok(told)) = (name, stream.cast::<IMFAttributes>()) {
          let _ = told.SetString(&MF_BYTESTREAM_ORIGIN_NAME, &HSTRING::from(&*name));
        }
        MFCreateSourceReaderFromByteStream(&stream, &attributes)?
      }
    };
    reader.SetStreamSelection(MF_SOURCE_READER_ALL_STREAMS.0 as u32, false)?;
    Ok(reader)
  }
}

/// How a frame lies in the bytes the decoder gives, and the part of it that is the picture.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Shape {
  /// The bytes from one row to the next.
  stride: usize,
  /// Where the picture starts in the frame.
  left: usize,
  top: usize,
  /// The picture's size, both even: NV12 has one color for two by two.
  width: usize,
  height: usize,
}

impl Shape {
  /// The shape of the frames `reader` gives now.
  unsafe fn of(reader: &IMFSourceReader) -> Result<Self> {
    unsafe {
      let ty = reader.GetCurrentMediaType(VIDEO)?;
      let size = ty.GetUINT64(&MF_MT_FRAME_SIZE)?;
      let (mut width, mut height) = ((size >> 32) as usize, (size & 0xFFFF_FFFF) as usize);
      let (mut left, mut top) = (0, 0);
      // A decoder rounds its frames up to whole blocks and says which part of them is the picture.
      let mut area = [0_u8; 16];
      if ty.GetBlob(&MF_MT_MINIMUM_DISPLAY_APERTURE, &mut area, None).is_ok() {
        let read = |at: usize| i32::from_le_bytes([area[at], area[at + 1], area[at + 2], area[at + 3]]);
        let (x, y) = (
          i16::from_le_bytes([area[2], area[3]]),
          i16::from_le_bytes([area[6], area[7]]),
        );
        let (wide, high) = (read(8), read(12));
        if wide > 0 && high > 0 && x >= 0 && y >= 0 {
          (left, top) = (x as usize & !1, y as usize & !1);
          (width, height) = (
            (wide as usize).min(width.saturating_sub(left)),
            (high as usize).min(height.saturating_sub(top)),
          );
        }
      }
      let stride = match ty.GetUINT32(&MF_MT_DEFAULT_STRIDE) {
        Ok(stride) if stride as i32 != 0 => (stride as i32).unsigned_abs() as usize,
        _ => (size >> 32) as usize,
      };
      Ok(Self {
        stride,
        left,
        top,
        width: width & !1,
        height: height & !1,
      })
    }
  }

  fn len(&self) -> usize {
    self.width * self.height * 3 / 2
  }

  /// A frame of black: the picture before its first frame.
  fn black(&self) -> Vec<u8> {
    let mut frame = vec![128; self.len()];
    frame[..self.width * self.height].fill(16);
    frame
  }

  /// Copies the picture out of the decoder's `bytes` (its rows of gray, then its rows of color at half the
  /// height) into `frame`, row after row with nothing between. `false` where the bytes are not a frame of this
  /// shape.
  fn copy(&self, bytes: &[u8], frame: &mut Vec<u8>) -> bool {
    let (stride, width, height) = (self.stride, self.width, self.height);
    if stride == 0 || width == 0 || height == 0 || self.left + width > stride {
      return false;
    }
    // How many rows of gray the decoder's frame has: its color is half as many again, after them.
    let rows = bytes.len() * 2 / (3 * stride);
    if rows < self.top + height {
      return false;
    }
    frame.clear();
    frame.reserve(self.len());
    for row in 0..height {
      let from = (self.top + row) * stride + self.left;
      frame.extend_from_slice(&bytes[from..from + width]);
    }
    let color = rows * stride;
    for row in 0..height / 2 {
      let from = color + (self.top / 2 + row) * stride + self.left;
      match bytes.get(from..from + width) {
        Some(line) => frame.extend_from_slice(line),
        None => return false,
      }
    }
    true
  }
}

/// Puts the frame `sample` holds into `image`.
unsafe fn show(image: &ImageData, sample: &IMFSample, shape: &Shape) -> Result<()> {
  unsafe {
    let buffer = sample.ConvertToContiguousBuffer()?;
    let (mut data, mut len) = (std::ptr::null_mut(), 0_u32);
    buffer.Lock(&mut data, None, Some(&mut len))?;
    let mut frame = image.take_streaming_nv12_buffer().unwrap_or_default();
    // SAFETY: the buffer is locked, and holds `len` bytes at `data` until it is unlocked.
    let copied = !data.is_null() && shape.copy(std::slice::from_raw_parts(data, len as usize), &mut frame);
    buffer.Unlock()?;
    if copied {
      image.set_streaming_nv12(frame);
    }
    Ok(())
  }
}

/// How long a media whose frames go through memory looks for something that draws on a card, and how often: a
/// player on an app's first screen opens its video before the window has anything to draw with.
const LOOKS_FOR: Duration = Duration::from_secs(10);
const LOOKS_EVERY: Duration = Duration::from_millis(100);

/// How a turn at playing ended.
enum Next {
  /// Nothing holds the media, or it cannot play.
  Done,
  /// Something draws on a card now, and this media's frames went through memory: it is to be opened again with
  /// the device of that card, and the image it showed until then.
  Again(OnCard, Option<ImageData>),
}

/// How many textures hold the frames of a video on the card: the one that is shown, the one the next frame is
/// copied into, and one between them, which what draws may still be reading.
const FRAMES: usize = 3;
/// How long a copy on the card is waited for before its frame is shown all the same.
const COPIED_WITHIN: Duration = Duration::from_millis(100);

/// What decodes on the graphics card the renderer draws on: the device the system's decoder is given, and the
/// textures its frames are copied into, which the renderer opens by their handles.
struct OnCard {
  device: ID3D11Device,
  context: ID3D11DeviceContext,
  manager: IMFDXGIDeviceManager,
  /// What tells when the card has done what it was told: a copy is finished before its frame is shown.
  done: ID3D11Query,
  held: Option<Held>,
}

/// The textures of a video's frames, of one size, and the frames an image draws from.
struct Held {
  textures: Vec<ID3D11Texture2D>,
  frames: SharedNv12Frames,
  width: u32,
  height: u32,
}

impl OnCard {
  /// A device on the card that draws shared frames; `None` where nothing draws them, or the card has no such
  /// device, and a frame goes through memory.
  unsafe fn new() -> Option<Self> {
    unsafe {
      let (low, high) = crate::images::shared_nv12_adapter()?;
      let cards: IDXGIFactory4 = CreateDXGIFactory1().ok()?;
      let card: IDXGIAdapter = cards
        .EnumAdapterByLuid(LUID {
          LowPart: low,
          HighPart: high,
        })
        .ok()?;
      let (mut device, mut context): (Option<ID3D11Device>, Option<ID3D11DeviceContext>) = (None, None);
      D3D11CreateDevice(
        &card,
        D3D_DRIVER_TYPE_UNKNOWN,
        HMODULE::default(),
        D3D11_CREATE_DEVICE_VIDEO_SUPPORT | D3D11_CREATE_DEVICE_BGRA_SUPPORT,
        Some(&[D3D_FEATURE_LEVEL_11_1, D3D_FEATURE_LEVEL_11_0]),
        D3D11_SDK_VERSION,
        Some(&mut device),
        None,
        Some(&mut context),
      )
      .ok()?;
      let (device, context) = (device?, context?);
      // The decoder uses the device from threads of its own.
      let _ = device.cast::<ID3D11Multithread>().ok()?.SetMultithreadProtected(true);
      let (mut token, mut manager) = (0_u32, None);
      MFCreateDXGIDeviceManager(&mut token, &mut manager).ok()?;
      let manager = manager?;
      manager.ResetDevice(&device, token).ok()?;
      let mut done = None;
      device
        .CreateQuery(
          &D3D11_QUERY_DESC {
            Query: D3D11_QUERY_EVENT,
            MiscFlags: 0,
          },
          Some(&mut done),
        )
        .ok()?;
      Some(Self {
        device,
        context,
        manager,
        done: done?,
        held: None,
      })
    }
  }

  /// The textures of frames `width` by `height`, each shared by a handle.
  unsafe fn hold(&self, width: u32, height: u32) -> Result<Held> {
    unsafe {
      let like = D3D11_TEXTURE2D_DESC {
        Width: width,
        Height: height,
        MipLevels: 1,
        ArraySize: 1,
        Format: DXGI_FORMAT_NV12,
        SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
        Usage: D3D11_USAGE_DEFAULT,
        BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
        CPUAccessFlags: 0,
        MiscFlags: (D3D11_RESOURCE_MISC_SHARED_NTHANDLE.0 | D3D11_RESOURCE_MISC_SHARED.0) as u32,
      };
      let (mut textures, mut handles) = (Vec::with_capacity(FRAMES), Vec::with_capacity(FRAMES));
      for _ in 0..FRAMES {
        let mut texture = None;
        self.device.CreateTexture2D(&like, None, Some(&mut texture))?;
        let texture = texture.expect("the texture Direct3D made");
        let handle =
          texture
            .cast::<IDXGIResource1>()?
            .CreateSharedHandle(None, DXGI_SHARED_RESOURCE_READ.0, PCWSTR::null())?;
        handles.push(handle.0 as isize);
        textures.push(texture);
      }
      Ok(Held {
        textures,
        // SAFETY: the handles are of shared NV12 textures of one size, made here and given away to be closed.
        frames: SharedNv12Frames::new(handles),
        width,
        height,
      })
    }
  }

  /// Waits until the card has done what it was told so far, for as long as a copy may take.
  unsafe fn settle(&self) {
    unsafe {
      self.context.End(&self.done);
      self.context.Flush();
      let started = Instant::now();
      loop {
        let mut finished = BOOL(0);
        let asked = self.context.GetData(
          &self.done,
          Some((&raw mut finished).cast()),
          std::mem::size_of::<BOOL>() as u32,
          0,
        );
        if asked.is_err() || finished.as_bool() || started.elapsed() > COPIED_WITHIN {
          return;
        }
        std::thread::yield_now();
      }
    }
  }

  /// Copies the frame `sample` holds into the texture after the one that is shown, and shows it. `None` where
  /// the sample is no texture of the card: the decoder gave memory. The frames themselves where they were made
  /// anew (the first frame, another size), and an image is to be made of them.
  unsafe fn show(&mut self, sample: &IMFSample, shape: &Shape) -> Result<Option<Option<SharedNv12Frames>>> {
    unsafe {
      let Ok(buffer) = sample.GetBufferByIndex(0)?.cast::<IMFDXGIBuffer>() else {
        return Ok(None);
      };
      let mut decoded = std::ptr::null_mut();
      buffer.GetResource(&ID3D11Texture2D::IID, &mut decoded)?;
      // SAFETY: the buffer answered with a texture, counted for the caller.
      let decoded = ID3D11Texture2D::from_raw(decoded);
      let slice = buffer.GetSubresourceIndex()?;
      let (width, height) = (shape.width as u32, shape.height as u32);
      let made = self
        .held
        .as_ref()
        .is_none_or(|held| (held.width, held.height) != (width, height));
      if made {
        self.held = Some(self.hold(width, height)?);
      }
      let held = self.held.as_ref().expect("the frames just held");
      let slot = if made {
        0
      } else {
        (held.frames.shown() + 1) % held.textures.len()
      };
      // The picture's part of the decoder's frame, which is rounded up to whole blocks.
      let picture = D3D11_BOX {
        left: shape.left as u32,
        top: shape.top as u32,
        front: 0,
        right: shape.left as u32 + width,
        bottom: shape.top as u32 + height,
        back: 1,
      };
      self
        .context
        .CopySubresourceRegion(&held.textures[slot], 0, 0, 0, 0, &decoded, slice, Some(&picture));
      // Shown once it is all there: what draws never reads a frame half copied.
      self.settle();
      held.frames.show(slot);
      Ok(Some(made.then(|| held.frames.clone())))
    }
  }
}

/// The picture of a media: what its frames are like, the image they are put into, and the frame that waits.
struct Picture {
  shape: Shape,
  image: ImageData,
  /// The image's frames on the card, where they are there and not in memory.
  native: Option<NativeImageData>,
  /// The frame decoded and not shown yet, with its time.
  pending: Option<(f64, IMFSample)>,
  /// A frame is owed whether it plays or not: the first one, and the one at a time it was sent to.
  owed: bool,
  /// Frames before this time are passed over: the reader starts from the key frame before where it was sent.
  from: Option<f64>,
  /// The reader has no more frames.
  done: bool,
}

impl Picture {
  /// The first video of `reader`, decoded to NV12; an error where it has none.
  unsafe fn of(reader: &IMFSourceReader) -> Result<Self> {
    unsafe {
      reader.SetStreamSelection(VIDEO, true)?;
      let wanted: IMFMediaType = MFCreateMediaType()?;
      wanted.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
      wanted.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_NV12)?;
      reader.SetCurrentMediaType(VIDEO, None, &wanted)?;
      let shape = Shape::of(reader)?;
      Ok(Self {
        shape,
        image: ImageData::streaming_nv12(shape.black(), shape.width as u32, shape.height as u32),
        native: None,
        pending: None,
        owed: true,
        from: None,
        done: false,
      })
    }
  }

  /// Shows the frame `sample` holds: copied on the card into the frames the image draws from, where the decoder
  /// gave it as a texture of that card, and through memory otherwise. Answers whether the picture's image is
  /// another one now, which who draws the media is to be told.
  unsafe fn show(&mut self, sample: &IMFSample, card: Option<&mut OnCard>) -> Result<bool> {
    unsafe {
      let (width, height) = (self.shape.width as u32, self.shape.height as u32);
      if let Some(card) = card
        && let Some(made) = card.show(sample, &self.shape)?
      {
        return Ok(match made {
          Some(frames) => {
            let native = NativeImageData::from_shared_nv12(width, height, frames);
            self.image = native.image_data();
            self.native = Some(native);
            true
          }
          None => {
            if let Some(native) = &self.native {
              native.bump_version();
            }
            false
          }
        });
      }
      // Through memory, into an image of memory: one whose frames were on the card is let go.
      let another = self.native.take().is_some();
      if another {
        self.image = ImageData::streaming_nv12(self.shape.black(), width, height);
      }
      show(&self.image, sample, &self.shape)?;
      Ok(another)
    }
  }

  /// Starts over from where the reader was sent: `to`, or its start.
  fn sent(&mut self, to: Option<f64>) {
    (self.pending, self.from, self.done) = (None, to, false);
    self.owed = self.owed || to.is_some();
  }
}

/// What the samples the decoder gives are like.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Samples {
  /// Frames in a second.
  rate: u32,
  channels: usize,
  /// The bytes of one sample.
  bytes: usize,
  float: bool,
}

impl Samples {
  /// What `reader` gives its sound as now.
  unsafe fn of(reader: &IMFSourceReader) -> Result<Self> {
    unsafe {
      let ty = reader.GetCurrentMediaType(AUDIO)?;
      Ok(Self {
        rate: ty.GetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND)?.max(1),
        channels: ty.GetUINT32(&MF_MT_AUDIO_NUM_CHANNELS)?.max(1) as usize,
        bytes: (ty.GetUINT32(&MF_MT_AUDIO_BITS_PER_SAMPLE)? as usize / 8).max(1),
        float: ty.GetGUID(&MF_MT_SUBTYPE)? == MFAudioFormat_Float,
      })
    }
  }

  /// One sample of `bytes`, from -1 to 1.
  fn sample(&self, bytes: &[u8]) -> f32 {
    match (self.float, bytes) {
      (true, [a, b, c, d]) => f32::from_le_bytes([*a, *b, *c, *d]),
      (true, [a, b, c, d, e, f, g, h]) => f64::from_le_bytes([*a, *b, *c, *d, *e, *f, *g, *h]) as f32,
      (false, [a]) => (f32::from(*a) - 128.0) / 128.0,
      (false, [a, b]) => f32::from(i16::from_le_bytes([*a, *b])) / 32_768.0,
      (false, [a, b, c]) => (i32::from_le_bytes([0, *a, *b, *c]) >> 8) as f32 / 8_388_608.0,
      (false, [a, b, c, d]) => i32::from_le_bytes([*a, *b, *c, *d]) as f32 / 2_147_483_648.0,
      _ => 0.0,
    }
  }

  /// The frames of `bytes` from the frame `skip` on, brought to two channels and put at the end of `queue`: one
  /// channel is heard on both sides, and of more than two the first two are, with the third (the middle one of
  /// the layouts that have five and more) on both.
  fn read(&self, bytes: &[u8], skip: usize, queue: &mut VecDeque<f32>) {
    let frame = self.bytes * self.channels;
    for frame in bytes.chunks_exact(frame).skip(skip) {
      let channel = |index: usize| self.sample(&frame[index * self.bytes..(index + 1) * self.bytes]);
      let (left, right) = match self.channels {
        1 => (channel(0), channel(0)),
        2 => (channel(0), channel(1)),
        _ => {
          let middle = channel(2) * std::f32::consts::FRAC_1_SQRT_2;
          (channel(0) + middle, channel(1) + middle)
        }
      };
      queue.push_back(left);
      queue.push_back(right);
    }
  }
}

/// The sound of a media: what the decoder gives, what waits to be heard, and where it goes.
struct Sound {
  samples: Samples,
  /// What is decoded and not given to the output yet: two samples a frame.
  queue: VecDeque<f32>,
  /// The time, in the media, of the frame the output is given next.
  at: f64,
  /// Nothing was decoded since the reader was last sent somewhere: the next sample says what time it is.
  fresh: bool,
  /// Sound before this time is passed over, as the frames of the picture are.
  from: Option<f64>,
  /// The reader has no more sound.
  done: bool,
  /// `None` where the system has nowhere to play it: the media goes on by its own clock, unheard.
  output: Option<Output>,
}

impl Sound {
  /// The first sound of `reader`, decoded to plain samples, with an output for it; an error where it has none.
  unsafe fn of(reader: &IMFSourceReader, speed: f64, level: f32) -> Result<Self> {
    unsafe {
      reader.SetStreamSelection(AUDIO, true)?;
      let ask = |subtype: &GUID| -> Result<()> {
        let wanted: IMFMediaType = MFCreateMediaType()?;
        wanted.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Audio)?;
        wanted.SetGUID(&MF_MT_SUBTYPE, subtype)?;
        reader.SetCurrentMediaType(AUDIO, None, &wanted)
      };
      if let Err(error) = ask(&MFAudioFormat_PCM).or_else(|_| ask(&MFAudioFormat_Float)) {
        let _ = reader.SetStreamSelection(AUDIO, false);
        return Err(error);
      }
      let samples = Samples::of(reader)?;
      Ok(Self {
        samples,
        queue: VecDeque::new(),
        at: 0.0,
        fresh: true,
        from: None,
        done: false,
        output: Output::open(samples.rate, speed, level).ok(),
      })
    }
  }

  /// How many seconds of sound wait in the queue.
  fn waiting(&self) -> f64 {
    (self.queue.len() / CHANNELS) as f64 / f64::from(self.samples.rate)
  }

  /// Takes the sound `sample` holds, which is at `time` in the media.
  unsafe fn take(&mut self, sample: &IMFSample, mut time: f64) -> Result<()> {
    unsafe {
      let buffer = sample.ConvertToContiguousBuffer()?;
      let (mut data, mut len) = (std::ptr::null_mut(), 0_u32);
      buffer.Lock(&mut data, None, Some(&mut len))?;
      if !data.is_null() {
        // SAFETY: the buffer is locked, and holds `len` bytes at `data` until it is unlocked.
        let bytes = std::slice::from_raw_parts(data, len as usize);
        // Not there yet: what is before where the reader was sent is not heard.
        let skip = match self.from {
          Some(from) if from > time => ((from - time) * f64::from(self.samples.rate)).round() as usize,
          _ => 0,
        };
        let frames = bytes.len() / (self.samples.bytes * self.samples.channels);
        if skip < frames {
          self.from = None;
          time += skip as f64 / f64::from(self.samples.rate);
          if self.fresh {
            (self.at, self.fresh) = (time, false);
          }
          self.samples.read(bytes, skip, &mut self.queue);
        }
      }
      buffer.Unlock()
    }
  }

  /// Starts over from where the reader was sent: `to`, or its start. What waited is not heard.
  unsafe fn sent(&mut self, to: Option<f64>) {
    self.queue.clear();
    (self.fresh, self.from, self.done) = (true, to, false);
    self.at = to.unwrap_or(0.0);
    if let Some(output) = &mut self.output
      && unsafe { output.empty() }.is_err()
    {
      self.output = None;
    }
  }

  /// Another output in place of this one, at `speed` and `level`: for another speed, and where the device this
  /// one played on is gone. What the old one held and had not played is given to the new one.
  unsafe fn replace(&mut self, speed: f64, level: f32) {
    if let Some(mut old) = self.output.take() {
      let unheard = old.unheard();
      let _ = unsafe { old.stop() };
      self.at -= (unheard.len() / CHANNELS) as f64 / f64::from(self.samples.rate);
      for sample in unheard.into_iter().rev() {
        self.queue.push_front(sample);
      }
    }
    self.output = unsafe { Output::open(self.samples.rate, speed, level) }.ok();
  }

  /// Whether all of it was heard: decoded to its end, and nothing of it waits anywhere.
  fn over(&self) -> bool {
    match &self.output {
      Some(output) => self.done && self.queue.is_empty() && output.pending() == 0,
      None => true,
    }
  }
}

/// Sends `reader` to `seconds`: the next frame it gives is the key frame at or before that time.
unsafe fn go(reader: &IMFSourceReader, seconds: f64) -> Result<()> {
  unsafe { reader.SetCurrentPosition(&GUID::zeroed(), &PROPVARIANT::from((seconds * TICKS) as i64)) }
}

/// Plays `opened` until nothing holds the media. `card` is the device of the card that draws, where one does;
/// `before` is there where the media was playing already and is opened again, with the image it showed.
unsafe fn play(
  shared: &Shared,
  opened: Opened,
  heard: bool,
  mut card: Option<OnCard>,
  before: Option<Option<ImageData>>,
) -> Result<Next> {
  unsafe {
    let reader = open(opened, card.as_ref())?;
    let (mut volume, mut muted) = {
      let inner = shared.inner();
      (inner.volume, inner.muted)
    };
    let level = |volume: f32, muted: bool| if muted { 0.0 } else { volume };
    let mut picture = if heard { None } else { Picture::of(&reader).ok() };
    let mut sound = Sound::of(&reader, shared.inner().rate, level(volume, muted)).ok();
    if picture.is_none() && sound.is_none() {
      // What the system opened and found nothing in to play: a kind of picture or sound it has no decoder for.
      shared.fail(MediaError::Unsupported);
      return Ok(Next::Done);
    }
    // Opened again, it goes on from where it was, and shows what it showed until its frame there is decoded.
    if let Some(shown) = before {
      let at = {
        let mut inner = shared.inner();
        inner.hold();
        inner.at
      };
      if at > 0.0 {
        go(&reader, at)?;
        if let Some(sound) = &mut sound {
          sound.sent(Some(at));
        }
      }
      if let Some(picture) = &mut picture {
        picture.sent(Some(at));
        if let Some(shown) = shown
          && (shown.width(), shown.height()) == (picture.shape.width as u32, picture.shape.height as u32)
          && !shown.is_native()
        {
          picture.image = shown;
        }
      }
    }
    let looks_since = Instant::now();
    let duration = reader
      .GetPresentationAttribute(MF_SOURCE_READER_MEDIASOURCE.0 as u32, &MF_PD_DURATION)
      .ok()
      .and_then(|ticks| u64::try_from(&ticks).ok())
      .map(|ticks| ticks as f64 / TICKS)
      .filter(|duration| *duration > 0.0);
    {
      let mut inner = shared.inner();
      inner.duration = duration;
      inner.size = picture
        .as_ref()
        .map(|picture| (picture.shape.width as u32, picture.shape.height as u32));
      inner.frame = picture.as_ref().map(|picture| picture.image.clone());
      inner.sound = sound.is_some();
      inner.waiting = false;
      inner.since = Instant::now();
    }
    shared.wake();
    // Something was done in the last turn of the loop: there may be more to do at once.
    let mut worked = true;
    // How many of the times it was sent somewhere it has done.
    let mut gone = 0_u64;
    loop {
      let (playing, ended, now, rate) = {
        let inner = shared.inner();
        (inner.playing, inner.ended, inner.position(), inner.rate)
      };
      // Its frames go through memory, and something may draw on a card by now: a window that had nothing to
      // draw with when this opened. It starts over on that card.
      let looks = card.is_none() && picture.is_some() && looks_since.elapsed() < LOOKS_FOR;
      if looks && let Some(found) = OnCard::new() {
        return Ok(Next::Again(found, picture.map(|picture| picture.image)));
      }
      let wait = if worked {
        None
      } else if playing && !ended {
        let due = picture
          .as_ref()
          .and_then(|picture| picture.pending.as_ref())
          .map_or(STEP, |(time, _)| {
            Duration::from_secs_f64(((time - now) / rate).max(0.0))
          });
        Some(due.min(STEP)).filter(|wait| !wait.is_zero())
      } else if looks {
        Some(LOOKS_EVERY)
      } else {
        // Nothing to do until it is told something.
        Some(Duration::from_secs(3600))
      };
      let commands = shared.take(wait);
      worked = !commands.is_empty();
      // Sent to many times while a seek bar is dragged, it goes to the last of them.
      let last_seek = commands.iter().rposition(|command| matches!(command, Command::Seek(_)));
      for (index, command) in commands.into_iter().enumerate() {
        if matches!(command, Command::Seek(_)) {
          gone += 1;
          if Some(index) != last_seek {
            continue;
          }
        }
        match command {
          Command::Close => return Ok(Next::Done),
          Command::Play => {
            if shared.inner().ended {
              go(&reader, 0.0)?;
              if let Some(picture) = &mut picture {
                picture.sent(None);
              }
              if let Some(sound) = &mut sound {
                sound.sent(None);
              }
              let mut inner = shared.inner();
              inner.ended = false;
              inner.at = 0.0;
            }
            {
              let mut inner = shared.inner();
              inner.since = Instant::now();
              inner.playing = true;
            }
            // Who draws it may have asked whether it plays before this thread heard the command.
            shared.wake();
          }
          Command::Pause => {
            let mut inner = shared.inner();
            inner.hold();
            inner.playing = false;
          }
          Command::Seek(to) => {
            let to = duration.map_or(to, |duration| to.min(duration));
            go(&reader, to)?;
            if let Some(picture) = &mut picture {
              picture.sent(Some(to));
            }
            if let Some(sound) = &mut sound {
              sound.sent(Some(to));
            }
            let mut inner = shared.inner();
            inner.at = to;
            inner.since = Instant::now();
            inner.ended = false;
          }
          Command::Rate(rate) => {
            {
              let mut inner = shared.inner();
              inner.hold();
              inner.rate = rate;
            }
            if let Some(sound) = &mut sound {
              sound.replace(rate, level(volume, muted));
            }
          }
          Command::Volume(to) => {
            volume = to;
            if let Some(output) = sound.as_ref().and_then(|sound| sound.output.as_ref()) {
              output.set_level(level(volume, muted));
            }
          }
          Command::Muted(to) => {
            muted = to;
            if let Some(output) = sound.as_ref().and_then(|sound| sound.output.as_ref()) {
              output.set_level(level(volume, muted));
            }
          }
        }
      }
      let (playing, ended, rate) = {
        let inner = shared.inner();
        (inner.playing, inner.ended, inner.rate)
      };

      // The next frame of the picture.
      if let Some(picture) = &mut picture
        && picture.pending.is_none()
        && (playing || picture.owed)
        && !picture.done
        && !ended
      {
        worked = true;
        let (mut flags, mut ticks, mut sample) = (0_u32, 0_i64, None);
        reader.ReadSample(VIDEO, 0, None, Some(&mut flags), Some(&mut ticks), Some(&mut sample))?;
        if flags & MF_SOURCE_READERF_CURRENTMEDIATYPECHANGED.0 as u32 != 0 {
          let next = Shape::of(&reader)?;
          if (next.width, next.height) != (picture.shape.width, picture.shape.height) {
            // Another size is another image: who draws the media takes it when it is told.
            picture.image = ImageData::streaming_nv12(next.black(), next.width as u32, next.height as u32);
            picture.native = None;
            let mut inner = shared.inner();
            inner.size = Some((next.width as u32, next.height as u32));
            inner.frame = Some(picture.image.clone());
            drop(inner);
            shared.wake();
          }
          picture.shape = next;
        }
        if flags & MF_SOURCE_READERF_ENDOFSTREAM.0 as u32 != 0 {
          (picture.done, picture.owed) = (true, false);
        } else if let Some(sample) = sample {
          let time = ticks as f64 / TICKS;
          // Not there yet: the frames from the key frame to where it was sent are decoded and not shown.
          if !picture.from.is_some_and(|from| time + 0.001 < from) {
            picture.from = None;
            picture.pending = Some((time, sample));
          }
        }
      }

      // More of the sound, while less than it keeps ahead waits.
      if let Some(sound) = &mut sound
        && sound.output.is_some()
        && !sound.done
        && !ended
        && sound.waiting() < AHEAD
      {
        worked = true;
        let (mut flags, mut ticks, mut sample) = (0_u32, 0_i64, None);
        reader.ReadSample(AUDIO, 0, None, Some(&mut flags), Some(&mut ticks), Some(&mut sample))?;
        if flags & MF_SOURCE_READERF_CURRENTMEDIATYPECHANGED.0 as u32 != 0 {
          let next = Samples::of(&reader)?;
          if next.rate != sound.samples.rate {
            // What waited was at the old rate: it is let go with the output that played it.
            sound.queue.clear();
            sound.samples = next;
            sound.replace(rate, level(volume, muted));
            sound.fresh = true;
          }
          sound.samples = next;
        }
        if flags & MF_SOURCE_READERF_ENDOFSTREAM.0 as u32 != 0 {
          sound.done = true;
        } else if let Some(sample) = sample {
          sound.take(&sample, ticks as f64 / TICKS)?;
        }
      }

      // The sound goes to the output while it plays, once the picture it was sent to is shown: the two start
      // together.
      let shown = picture.as_ref().is_none_or(|picture| !picture.owed);
      if let Some(sound) = &mut sound
        && sound.output.is_some()
      {
        let at = sound.at;
        let output = sound.output.as_mut().expect("the output that was just there");
        let fed = if playing && !ended && shown {
          output.feed(&mut sound.queue, at).and_then(|frames| {
            if frames > 0 || output.pending() > 0 {
              output.start()?;
            }
            Ok(frames)
          })
        } else {
          output.stop().map(|()| 0)
        };
        match fed {
          Ok(frames) => {
            sound.at += frames as f64 / f64::from(sound.samples.rate);
            worked = worked || frames > 0;
            // The media's clock is the sound's: what is heard is where it is.
            if output.running()
              && output.pending() > 0
              && let Some(heard) = output.heard()
            {
              let mut inner = shared.inner();
              if inner.playing && inner.sent == gone && (heard - inner.position()).abs() > APART {
                inner.at = heard;
                inner.since = Instant::now();
              }
            }
          }
          // The device is gone, or was changed: the sound goes on on the one the system has now.
          Err(_) => sound.replace(rate, level(volume, muted)),
        }
      }

      // The frame that waits is shown when it is owed, and when the clock comes to its time.
      if let Some(picture) = &mut picture
        && let Some(time) = picture.pending.as_ref().map(|(time, _)| *time)
      {
        // Whether it is taken now, and whether it is shown: one that is too late is passed over.
        let taken = if picture.owed {
          Some(true)
        } else {
          // Whether it plays is read now, and not as the turn began: a pause is in the media's state at once,
          // and a frame decoded before it is not shown after it.
          let (plays, now) = {
            let inner = shared.inner();
            (inner.playing, inner.position())
          };
          // A file may hold frames a little past the length it says: at its end the clock stands, and they are
          // due.
          (plays && (time <= now + 0.001 || duration.is_some_and(|duration| now >= duration)))
            .then_some(now - time <= LATE)
        };
        if let Some(shown) = taken {
          let owed = picture.owed;
          if let Some((_, sample)) = picture.pending.take()
            && shown
            && picture.show(&sample, card.as_mut())?
          {
            // Another image holds the frames now: who draws the media takes it when it is told.
            shared.inner().frame = Some(picture.image.clone());
            shared.wake();
          }
          picture.owed = false;
          worked = true;
          if owed {
            shared.wake();
          }
        }
      }

      // It ended once all of its picture was shown and all of its sound was heard. What has neither (a sound
      // with nowhere to play) ends when its clock comes to its length.
      let seen = picture
        .as_ref()
        .is_none_or(|picture| picture.done && picture.pending.is_none());
      let listened = sound.as_ref().is_none_or(Sound::over);
      let timed = picture.is_some()
        || sound.as_ref().is_some_and(|sound| sound.output.is_some())
        || (playing && duration.is_none_or(|duration| shared.inner().position() >= duration));
      if !ended && seen && listened && timed {
        {
          let mut inner = shared.inner();
          inner.hold();
          if let Some(duration) = inner.duration {
            inner.at = duration;
          }
          inner.ended = true;
          inner.playing = false;
        }
        if let Some(output) = sound.as_mut().and_then(|sound| sound.output.as_mut()) {
          let _ = output.stop();
        }
        worked = true;
        shared.wake();
      }
    }
  }
}
