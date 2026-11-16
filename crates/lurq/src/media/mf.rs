//! The thread of a media on Windows: Media Foundation's source reader decodes to NV12, and each frame is shown
//! when the media's clock comes to its time.

use std::{
  sync::Arc,
  time::{Duration, Instant},
};

use windows::{
  Win32::{
    Media::MediaFoundation::{
      IMFAttributes, IMFMediaType, IMFSample, IMFSourceReader, MF_MT_DEFAULT_STRIDE, MF_MT_FRAME_SIZE,
      MF_MT_MAJOR_TYPE, MF_MT_MINIMUM_DISPLAY_APERTURE, MF_MT_SUBTYPE, MF_PD_DURATION,
      MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS, MF_SOURCE_READER_ALL_STREAMS,
      MF_SOURCE_READER_ENABLE_ADVANCED_VIDEO_PROCESSING, MF_SOURCE_READER_FIRST_VIDEO_STREAM,
      MF_SOURCE_READER_MEDIASOURCE, MF_SOURCE_READERF_CURRENTMEDIATYPECHANGED, MF_SOURCE_READERF_ENDOFSTREAM,
      MF_VERSION, MFCreateAttributes, MFCreateMFByteStreamOnStream, MFCreateMediaType,
      MFCreateSourceReaderFromByteStream, MFCreateSourceReaderFromURL, MFMediaType_Video, MFSTARTUP_FULL, MFShutdown,
      MFStartup, MFVideoFormat_NV12,
    },
    System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize, StructuredStorage::PROPVARIANT},
    UI::Shell::SHCreateMemStream,
  },
  core::{GUID, HSTRING, Result},
};

use super::{Command, Opened, Shared};
use crate::images::ImageData;

/// The stream that is decoded: the first video of what is opened.
const VIDEO: u32 = MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32;
/// A second, in the units Media Foundation counts time in: 100 nanoseconds.
const TICKS: f64 = 10_000_000.0;
/// How long the thread waits for its next frame's time before it looks at what it was told.
const STEP: Duration = Duration::from_millis(10);
/// A frame this late is not shown: the next ones catch up with the clock.
const LATE: f64 = 0.2;

/// Runs a media to its end: opens `opened`, decodes, and does what it is told until nothing holds the media.
pub(super) fn run(shared: &Arc<Shared>, opened: Opened) {
  // SAFETY: COM and Media Foundation are started and stopped on this thread, around everything that uses them.
  unsafe {
    let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    match MFStartup(MF_VERSION, MFSTARTUP_FULL) {
      Ok(()) => {
        if let Err(error) = play(shared, opened) {
          shared.fail(error.message());
        }
        let _ = MFShutdown();
      }
      Err(error) => shared.fail(error.message()),
    }
    CoUninitialize();
  }
}

/// A reader of `opened` that decodes its first video to NV12, on the graphics card where the system can.
unsafe fn open(opened: Opened) -> Result<IMFSourceReader> {
  unsafe {
    let mut attributes: Option<IMFAttributes> = None;
    MFCreateAttributes(&mut attributes, 2)?;
    let attributes = attributes.expect("the attributes Media Foundation made");
    // Its own conversion to the format asked for, and the decoders of the hardware where there are some.
    attributes.SetUINT32(&MF_SOURCE_READER_ENABLE_ADVANCED_VIDEO_PROCESSING, 1)?;
    attributes.SetUINT32(&MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS, 1)?;
    let reader = match opened {
      Opened::File(path) => MFCreateSourceReaderFromURL(&HSTRING::from(path.as_os_str()), &attributes)?,
      Opened::Url(address) => MFCreateSourceReaderFromURL(&HSTRING::from(&*address), &attributes)?,
      Opened::Bytes(bytes) => {
        let stream = SHCreateMemStream(Some(&bytes)).ok_or_else(windows::core::Error::from_thread)?;
        let stream = MFCreateMFByteStreamOnStream(&stream)?;
        MFCreateSourceReaderFromByteStream(&stream, &attributes)?
      }
    };
    reader.SetStreamSelection(MF_SOURCE_READER_ALL_STREAMS.0 as u32, false)?;
    reader.SetStreamSelection(VIDEO, true)?;
    let wanted: IMFMediaType = MFCreateMediaType()?;
    wanted.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
    wanted.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_NV12)?;
    reader.SetCurrentMediaType(VIDEO, None, &wanted)?;
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

/// Sends `reader` to `seconds`: the next frame it gives is the key frame at or before that time.
unsafe fn go(reader: &IMFSourceReader, seconds: f64) -> Result<()> {
  unsafe { reader.SetCurrentPosition(&GUID::zeroed(), &PROPVARIANT::from((seconds * TICKS) as i64)) }
}

unsafe fn play(shared: &Shared, opened: Opened) -> Result<()> {
  unsafe {
    let reader = open(opened)?;
    let mut shape = Shape::of(&reader)?;
    let duration = reader
      .GetPresentationAttribute(MF_SOURCE_READER_MEDIASOURCE.0 as u32, &MF_PD_DURATION)
      .ok()
      .and_then(|ticks| u64::try_from(&ticks).ok())
      .map(|ticks| ticks as f64 / TICKS)
      .filter(|duration| *duration > 0.0);
    let mut image = ImageData::streaming_nv12(shape.black(), shape.width as u32, shape.height as u32);
    {
      let mut inner = shared.inner();
      inner.duration = duration;
      inner.size = Some((shape.width as u32, shape.height as u32));
      inner.frame = Some(image.clone());
      inner.waiting = false;
      inner.since = Instant::now();
    }
    shared.wake();
    // The frame decoded and not shown yet, with its time.
    let mut pending: Option<(f64, IMFSample)> = None;
    // A frame is owed whether it plays or not: the first one, and the one at a time it was sent to.
    let mut owed = true;
    // Frames before this time are passed over: the reader starts from the key frame before where it was sent.
    let mut from: Option<f64> = None;
    loop {
      let (playing, ended, now, rate) = {
        let inner = shared.inner();
        (inner.playing, inner.ended, inner.position(), inner.rate)
      };
      let wait = match &pending {
        _ if owed && !ended => None,
        Some((time, _)) if playing => {
          Some(Duration::from_secs_f64(((time - now) / rate).max(0.0)).min(STEP)).filter(|wait| !wait.is_zero())
        }
        None if playing && !ended => None,
        // Nothing to do until it is told something.
        _ => Some(Duration::from_secs(3600)),
      };
      for command in shared.take(wait) {
        match command {
          Command::Close => return Ok(()),
          Command::Play => {
            if shared.inner().ended {
              go(&reader, 0.0)?;
              (pending, from) = (None, None);
              let mut inner = shared.inner();
              inner.ended = false;
              inner.at = 0.0;
            }
            let mut inner = shared.inner();
            inner.since = Instant::now();
            inner.playing = true;
          }
          Command::Pause => {
            let mut inner = shared.inner();
            inner.hold();
            inner.playing = false;
          }
          Command::Seek(to) => {
            let to = duration.map_or(to, |duration| to.min(duration));
            go(&reader, to)?;
            (pending, from, owed) = (None, Some(to), true);
            let mut inner = shared.inner();
            inner.at = to;
            inner.since = Instant::now();
            inner.ended = false;
          }
          Command::Rate(rate) => {
            let mut inner = shared.inner();
            inner.hold();
            inner.rate = rate;
          }
        }
      }
      let (playing, ended) = {
        let inner = shared.inner();
        (inner.playing, inner.ended)
      };
      if pending.is_none() && (playing || owed) && !ended {
        let (mut flags, mut ticks, mut sample) = (0_u32, 0_i64, None);
        reader.ReadSample(VIDEO, 0, None, Some(&mut flags), Some(&mut ticks), Some(&mut sample))?;
        if flags & MF_SOURCE_READERF_CURRENTMEDIATYPECHANGED.0 as u32 != 0 {
          let next = Shape::of(&reader)?;
          if (next.width, next.height) != (shape.width, shape.height) {
            // Another size is another image: who draws the media takes it when it is told.
            image = ImageData::streaming_nv12(next.black(), next.width as u32, next.height as u32);
            let mut inner = shared.inner();
            inner.size = Some((next.width as u32, next.height as u32));
            inner.frame = Some(image.clone());
            drop(inner);
            shared.wake();
          }
          shape = next;
        }
        if flags & MF_SOURCE_READERF_ENDOFSTREAM.0 as u32 != 0 {
          {
            let mut inner = shared.inner();
            inner.hold();
            if let Some(duration) = inner.duration {
              inner.at = duration;
            }
            inner.ended = true;
            inner.playing = false;
          }
          owed = false;
          shared.wake();
          continue;
        }
        let Some(sample) = sample else {
          continue;
        };
        let time = ticks as f64 / TICKS;
        // Not there yet: the frames from the key frame to where it was sent are decoded and not shown.
        if from.is_some_and(|from| time + 0.001 < from) {
          continue;
        }
        from = None;
        pending = Some((time, sample));
      }
      let Some((time, sample)) = &pending else {
        continue;
      };
      if owed {
        show(&image, sample, &shape)?;
        (pending, owed) = (None, false);
        shared.wake();
        continue;
      }
      if playing {
        let now = shared.inner().position();
        // A file may hold frames a little past the length it says: at its end the clock stands, and they are due.
        if *time <= now + 0.001 || duration.is_some_and(|duration| now >= duration) {
          if now - time <= LATE {
            show(&image, sample, &shape)?;
          }
          pending = None;
        }
      }
    }
  }
}
