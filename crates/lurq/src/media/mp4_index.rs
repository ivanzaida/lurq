//! The index of an MP4 file, its `moov` box: where in its bytes each moment of it is, and where its key frames are.
//!
//! A media reads ahead by time, and the reader of its address fetches by bytes. Told by the share of its length
//! that a time is, the bytes a moment needs are as far off as its bitrate is uneven and its tracks are laid out
//! apart; the index says where each chunk of each track is and when it plays.

use std::ops::Range;

use crate::resources::Reader;

/// The largest index read: a few megabytes for an hour of video.
const MOST: u64 = 64 << 20;
/// How many boxes are looked through for the index, from the start of a file.
const BOXES: usize = 64;

/// What the index of an MP4 file says.
#[derive(Debug, PartialEq)]
pub(crate) struct Index {
  /// For each chunk of each track, in the order they play: when its first sample plays, and the end of the bytes
  /// all that plays until then needs, this chunk and every one that plays before it.
  reach: Vec<(f64, u64)>,
  /// The key frames of its picture: when each plays, and its bytes.
  key_frames: Vec<(f64, Range<u64>)>,
}

impl Index {
  /// The index of what `reader` reads, where it is an MP4 file whose index says where its samples are: read from
  /// its boxes, the index wherever it is (before the pictures or after them). `None` for what is not such a file:
  /// another kind, an MP4 whose samples are described in fragments, one that cannot be read.
  pub(crate) fn read(reader: &dyn Reader) -> Option<Self> {
    let len = reader.len()?;
    let mut at = 0;
    for _ in 0..BOXES {
      let head = read_exact(reader, at, 16.min(len.checked_sub(at)?))?;
      let (size, kind, header) = box_head(&head, len - at)?;
      if &kind == b"moov" {
        if size > MOST {
          return None;
        }
        let moov = read_exact(reader, at, size)?;
        return Self::parse(&moov[header as usize..]);
      }
      at = at.checked_add(size)?;
      if at >= len {
        return None;
      }
    }
    None
  }

  /// The index in the boxes of a `moov` box, `boxes` being what is in it.
  pub(crate) fn parse(boxes: &[u8]) -> Option<Self> {
    let mut chunks: Vec<(f64, u64)> = Vec::new();
    let mut key_frames = Vec::new();
    for (kind, trak) in children(boxes) {
      if &kind != b"trak" {
        continue;
      }
      let Some(track) = Track::of(trak) else {
        continue;
      };
      track.chunks(&mut chunks, track.picture.then_some(&mut key_frames));
    }
    if chunks.is_empty() {
      return None;
    }
    chunks.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut end = 0;
    let reach = chunks
      .into_iter()
      .map(|(time, chunk_end)| {
        end = end.max(chunk_end);
        (time, end)
      })
      .collect();
    key_frames.sort_by(|a: &(f64, Range<u64>), b| a.0.total_cmp(&b.0));
    Some(Self { reach, key_frames })
  }

  /// The end of the bytes all that plays until `seconds` needs: of the chunk that plays then, of every track, and of
  /// every chunk that plays before it.
  pub(crate) fn byte_at(&self, seconds: f64) -> u64 {
    let after = self.reach.partition_point(|(time, _)| *time <= seconds);
    match after.checked_sub(1) {
      Some(at) => self.reach[at].1,
      None => self.reach.first().map_or(0, |(_, end)| *end),
    }
  }

  /// The key frames of its picture, in the order they play: when each plays, and its bytes. Empty for what has no
  /// picture, and for a picture whose every frame is one.
  #[cfg_attr(not(test), allow(dead_code))]
  pub(crate) fn key_frames(&self) -> &[(f64, Range<u64>)] {
    &self.key_frames
  }
}

/// A track's tables: when its samples play, how large they are, which chunks hold them and where those are.
struct Track<'a> {
  picture: bool,
  scale: u32,
  times: &'a [u8],
  sizes: &'a [u8],
  chunked: &'a [u8],
  offsets: &'a [u8],
  wide: bool,
  sync: Option<&'a [u8]>,
}

impl<'a> Track<'a> {
  fn of(trak: &'a [u8]) -> Option<Self> {
    let mdia = child(trak, b"mdia")?;
    let mdhd = child(mdia, b"mdhd")?;
    let scale = match *mdhd.first()? {
      0 => u32_at(mdhd, 12)?,
      _ => u32_at(mdhd, 20)?,
    };
    let picture = child(mdia, b"hdlr").and_then(|hdlr| hdlr.get(8..12)) == Some(b"vide");
    let stbl = child(child(mdia, b"minf")?, b"stbl")?;
    let (offsets, wide) = match child(stbl, b"stco") {
      Some(stco) => (stco, false),
      None => (child(stbl, b"co64")?, true),
    };
    Some(Self {
      picture,
      scale: scale.max(1),
      times: child(stbl, b"stts")?,
      sizes: child(stbl, b"stsz")?,
      chunked: child(stbl, b"stsc")?,
      offsets,
      wide,
      sync: child(stbl, b"stss"),
    })
  }

  /// Puts each of its chunks in `chunks`, as when it plays and where its bytes end, and each of its key frames in
  /// `key_frames` where it is given.
  fn chunks(&self, chunks: &mut Vec<(f64, u64)>, mut key_frames: Option<&mut Vec<(f64, Range<u64>)>>) {
    let Some(count) = u32_at(self.offsets, 4) else {
      return;
    };
    let uniform = u32_at(self.sizes, 4).unwrap_or(0);
    let samples = u32_at(self.sizes, 8).unwrap_or(0);
    let runs = u32_at(self.chunked, 4).unwrap_or(0);
    let mut clock = Clock::new(self.times);
    let mut sync = self.sync.map(|stss| (u32_at(stss, 4).unwrap_or(0), 0u32, stss));
    let mut sample = 0u32;
    let mut run = 0u32;
    for chunk in 0..count {
      let Some(mut offset) = (if self.wide {
        u64_at(self.offsets, 8 + 8 * chunk as usize)
      } else {
        u32_at(self.offsets, 8 + 4 * chunk as usize).map(u64::from)
      }) else {
        return;
      };
      // The run of chunks this one is of: each run says from which chunk on its chunks hold so many samples.
      while run + 1 < runs && u32_at(self.chunked, 8 + 12 * (run as usize + 1)).is_some_and(|first| first <= chunk + 1)
      {
        run += 1;
      }
      let Some(per) = u32_at(self.chunked, 12 + 12 * run as usize) else {
        return;
      };
      let starts = clock.at(sample) as f64 / f64::from(self.scale);
      for _ in 0..per {
        if sample >= samples {
          break;
        }
        let size = u64::from(if uniform > 0 {
          uniform
        } else {
          u32_at(self.sizes, 12 + 4 * sample as usize).unwrap_or(0)
        });
        if let (Some(key_frames), Some((count, next, stss))) = (key_frames.as_deref_mut(), sync.as_mut())
          && *next < *count
          && u32_at(stss, 8 + 4 * *next as usize) == Some(sample + 1)
        {
          *next += 1;
          key_frames.push((clock.at(sample) as f64 / f64::from(self.scale), offset..offset + size));
        }
        offset += size;
        sample += 1;
      }
      if per > 0 {
        chunks.push((starts, offset));
      }
    }
  }
}

/// When the samples of a track play, read from its `stts` table in order.
struct Clock<'a> {
  table: &'a [u8],
  entries: u32,
  entry: u32,
  /// The first sample of the entry, and when it plays.
  first: u32,
  time: u64,
}

impl<'a> Clock<'a> {
  fn new(table: &'a [u8]) -> Self {
    Self {
      table,
      entries: u32_at(table, 4).unwrap_or(0),
      entry: 0,
      first: 0,
      time: 0,
    }
  }

  /// When `sample` plays, in the track's units: samples are asked for in order.
  fn at(&mut self, sample: u32) -> u64 {
    while self.entry < self.entries {
      let at = 8 + 8 * self.entry as usize;
      let (Some(count), Some(delta)) = (u32_at(self.table, at), u32_at(self.table, at + 4)) else {
        break;
      };
      if sample < self.first + count {
        return self.time + u64::from(sample - self.first) * u64::from(delta);
      }
      self.time += u64::from(count) * u64::from(delta);
      self.first += count;
      self.entry += 1;
    }
    self.time
  }
}

/// The size, the kind and the size of the header of the box `head` starts, `left` bytes before its file ends.
fn box_head(head: &[u8], left: u64) -> Option<(u64, [u8; 4], u64)> {
  let kind: [u8; 4] = head.get(4..8)?.try_into().ok()?;
  match u32_at(head, 0)? {
    0 => Some((left, kind, 8)),
    1 => Some((u64_at(head, 8)?, kind, 16)),
    size if size >= 8 => Some((u64::from(size), kind, 8)),
    _ => None,
  }
}

/// The boxes in `boxes`, each its kind and what is in it.
fn children(mut boxes: &[u8]) -> impl Iterator<Item = ([u8; 4], &[u8])> {
  std::iter::from_fn(move || {
    let (size, kind, header) = box_head(boxes, boxes.len() as u64)?;
    let size = usize::try_from(size).ok()?;
    let body = boxes.get(header as usize..size)?;
    boxes = &boxes[size..];
    Some((kind, body))
  })
}

/// What is in the first box of `kind` in `boxes`.
fn child<'a>(boxes: &'a [u8], kind: &[u8; 4]) -> Option<&'a [u8]> {
  children(boxes).find(|(found, _)| found == kind).map(|(_, body)| body)
}

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
  Some(u32::from_be_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

fn u64_at(bytes: &[u8], at: usize) -> Option<u64> {
  Some(u64::from_be_bytes(bytes.get(at..at + 8)?.try_into().ok()?))
}

/// `len` bytes of `reader` from `at`, waiting for them: `None` where they are not all there.
fn read_exact(reader: &dyn Reader, at: u64, len: u64) -> Option<Vec<u8>> {
  let mut bytes = vec![0; usize::try_from(len).ok()?];
  let mut filled = 0;
  while filled < bytes.len() {
    match reader.read_at(at + filled as u64, &mut bytes[filled..]) {
      Ok(0) | Err(_) => return None,
      Ok(read) => filled += read,
    }
  }
  Some(bytes)
}

#[cfg(test)]
mod tests;
