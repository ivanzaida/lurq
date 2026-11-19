//! Where the sound of a media goes on Windows: the system's default device, shared with what else plays
//! (WASAPI). It is fed two channels of samples at the rate the decoder gives, and says how much of them was
//! heard, which is the clock of a media that has sound.

use std::collections::VecDeque;

use windows::{
  Win32::{
    Media::Audio::{
      AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM, AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY,
      IAudioClient, IAudioClock, IAudioRenderClient, IAudioStreamVolume, IMMDeviceEnumerator, MMDeviceEnumerator,
      WAVE_FORMAT_PCM, WAVEFORMATEX, eMultimedia, eRender,
    },
    System::Com::{CLSCTX_ALL, CoCreateInstance},
  },
  core::Result,
};

/// The channels an output is fed: what the decoder gives is brought to two.
pub(super) const CHANNELS: usize = 2;
/// How much sound the device holds ahead, in the units the system counts time in (100 nanoseconds): a fifth of a
/// second. A pause, the volume and the mute do not wait for it; a change of speed gives it back.
const AHEAD: i64 = 2_000_000;

/// An output that plays.
pub(super) struct Output {
  client: IAudioClient,
  render: IAudioRenderClient,
  clock: IAudioClock,
  level: Option<IAudioStreamVolume>,
  /// The units of the clock in a second.
  frequency: u64,
  /// How many frames the device holds.
  room: u32,
  /// How fast it plays: the device is told the samples are that many times as many in a second.
  speed: f64,
  running: bool,
  /// The time, in the media, of the first frame it was given since it was last emptied.
  base: f64,
  /// What it was last given, as much as the device holds: what was not heard yet is among it.
  recent: VecDeque<f32>,
}

impl Output {
  /// An output on the default device for samples at `rate` a second, played `speed` times as fast, at the
  /// loudness `level` (0 to 1).
  pub unsafe fn open(rate: u32, speed: f64, level: f32) -> Result<Self> {
    unsafe {
      let devices: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
      let device = devices.GetDefaultAudioEndpoint(eRender, eMultimedia)?;
      let client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;
      // Told that the samples are `speed` times as many in a second, the system plays them that much faster,
      // and brings them to what the device takes.
      let told = (f64::from(rate) * speed).round().clamp(1.0, f64::from(u32::MAX)) as u32;
      let frame = (CHANNELS * 2) as u16;
      let format = WAVEFORMATEX {
        wFormatTag: WAVE_FORMAT_PCM as u16,
        nChannels: CHANNELS as u16,
        nSamplesPerSec: told,
        nAvgBytesPerSec: told.saturating_mul(u32::from(frame)),
        nBlockAlign: frame,
        wBitsPerSample: 16,
        cbSize: 0,
      };
      client.Initialize(
        AUDCLNT_SHAREMODE_SHARED,
        AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM | AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY,
        AHEAD,
        0,
        &format,
        None,
      )?;
      let room = client.GetBufferSize()?;
      let render: IAudioRenderClient = client.GetService()?;
      let clock: IAudioClock = client.GetService()?;
      let frequency = clock.GetFrequency()?.max(1);
      let output = Self {
        level: client.GetService().ok(),
        client,
        render,
        clock,
        frequency,
        room,
        speed,
        running: false,
        base: 0.0,
        recent: VecDeque::new(),
      };
      output.set_level(level);
      Ok(output)
    }
  }

  /// How loud it is, from 0 to 1: of this output alone, and at once.
  pub fn set_level(&self, level: f32) {
    let Some(volume) = &self.level else {
      return;
    };
    // SAFETY: the interface is this output's own, used on the thread that made it.
    unsafe {
      let channels = volume.GetChannelCount().unwrap_or(0) as usize;
      let _ = volume.SetAllVolumes(&vec![level.clamp(0.0, 1.0); channels]);
    }
  }

  pub fn running(&self) -> bool {
    self.running
  }

  /// How many frames it holds that were not heard yet.
  pub fn pending(&self) -> u32 {
    // SAFETY: as in `set_level`.
    unsafe { self.client.GetCurrentPadding().unwrap_or(0) }
  }

  /// Gives it the frames at the front of `queue` (two samples each) that it has room for, and takes them out of
  /// the queue. The first frame given since it was emptied is at `at` in the media. Answers how many it took.
  pub unsafe fn feed(&mut self, queue: &mut VecDeque<f32>, at: f64) -> Result<usize> {
    unsafe {
      let free = self.room.saturating_sub(self.client.GetCurrentPadding()?) as usize;
      let frames = free.min(queue.len() / CHANNELS);
      if frames == 0 {
        return Ok(0);
      }
      if self.recent.is_empty() {
        self.base = at;
      }
      let data = self.render.GetBuffer(frames as u32)?;
      // SAFETY: the device gave room for `frames` frames of two 16-bit samples at `data`.
      let samples = std::slice::from_raw_parts_mut(data.cast::<i16>(), frames * CHANNELS);
      for slot in samples.iter_mut() {
        let sample = queue.pop_front().unwrap_or(0.0);
        *slot = (sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16;
        self.recent.push_back(sample);
      }
      self.render.ReleaseBuffer(frames as u32, 0)?;
      let kept = self.room as usize * CHANNELS;
      while self.recent.len() > kept {
        self.recent.pop_front();
      }
      Ok(frames)
    }
  }

  pub unsafe fn start(&mut self) -> Result<()> {
    if !self.running {
      unsafe { self.client.Start()? };
      self.running = true;
    }
    Ok(())
  }

  pub unsafe fn stop(&mut self) -> Result<()> {
    if self.running {
      unsafe { self.client.Stop()? };
      self.running = false;
    }
    Ok(())
  }

  /// Empties it: what it held is not heard, and its clock starts again with what it is given next.
  pub unsafe fn empty(&mut self) -> Result<()> {
    unsafe {
      self.stop()?;
      self.client.Reset()?;
    }
    self.recent.clear();
    Ok(())
  }

  /// Where in the media the sound that is heard now is; `None` before it was given anything.
  pub fn heard(&self) -> Option<f64> {
    if self.recent.is_empty() {
      return None;
    }
    let mut position = 0_u64;
    // SAFETY: as in `set_level`.
    unsafe { self.clock.GetPosition(&mut position, None).ok()? };
    Some(self.base + position as f64 / self.frequency as f64 * self.speed)
  }

  /// What it was given and has not played, to give to the output that takes its place: at another speed, or on
  /// another device.
  pub fn unheard(&self) -> Vec<f32> {
    let samples = (self.pending() as usize * CHANNELS).min(self.recent.len());
    self.recent.iter().skip(self.recent.len() - samples).copied().collect()
  }
}
