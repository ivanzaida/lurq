use super::*;
use crate::resources::{BytesReader, FileReader};

/// A box of `kind` holding `body`.
fn boxed(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
  let mut bytes = ((body.len() + 8) as u32).to_be_bytes().to_vec();
  bytes.extend_from_slice(kind);
  bytes.extend_from_slice(body);
  bytes
}

/// A full box: a version and flags, then `words`.
fn full(kind: &[u8; 4], words: &[u32]) -> Vec<u8> {
  let mut body = vec![0u8; 4];
  for word in words {
    body.extend_from_slice(&word.to_be_bytes());
  }
  boxed(kind, &body)
}

/// A track of `kind` (`vide`, `soun`), its time in `scale` units a second, its samples of `size` bytes each lasting
/// `delta` units, `per` of them in each chunk at `offsets`, and its key frames (the numbers of their samples).
fn track(
  kind: &[u8; 4],
  scale: u32,
  delta: u32,
  sizes: &[u32],
  per: u32,
  offsets: &[u32],
  sync: Option<&[u32]>,
) -> Vec<u8> {
  let mut mdhd = vec![0, 0, 0, 0];
  mdhd.extend_from_slice(&[0; 8]);
  mdhd.extend_from_slice(&scale.to_be_bytes());
  mdhd.extend_from_slice(&[0; 8]);
  let hdlr = {
    let mut body = vec![0u8; 8];
    body.extend_from_slice(kind);
    body.extend_from_slice(&[0; 12]);
    boxed(b"hdlr", &body)
  };
  let mut stbl = Vec::new();
  stbl.extend(full(b"stts", &[1, sizes.len() as u32, delta]));
  let mut stsz = vec![0, sizes.len() as u32];
  stsz.extend_from_slice(sizes);
  stbl.extend(full(b"stsz", &stsz));
  stbl.extend(full(b"stsc", &[1, 1, per, 1]));
  let mut stco = vec![offsets.len() as u32];
  stco.extend_from_slice(offsets);
  stbl.extend(full(b"stco", &stco));
  if let Some(sync) = sync {
    let mut stss = vec![sync.len() as u32];
    stss.extend_from_slice(sync);
    stbl.extend(full(b"stss", &stss));
  }
  let minf = boxed(b"minf", &boxed(b"stbl", &stbl));
  let mut mdia = boxed(b"mdhd", &mdhd);
  mdia.extend(hdlr);
  mdia.extend(minf);
  boxed(b"trak", &boxed(b"mdia", &mdia))
}

/// A picture of 4 frames a second, 2 to a chunk, and a sound of 2 samples a second, 1 to a chunk, laid out apart:
/// the picture's chunks first, at 1000 and 3000, and the sound's at 5000 and on.
fn index() -> Index {
  let mut moov = track(
    b"vide",
    4,
    1,
    &[400, 100, 100, 100, 400, 100],
    2,
    &[1000, 3000, 4000],
    Some(&[1, 5]),
  );
  moov.extend(track(b"soun", 2, 1, &[50, 50, 50], 1, &[5000, 5050, 5100], None));
  Index::parse(&moov).expect("an index")
}

#[test]
fn what_plays_until_a_time_needs_the_bytes_of_every_track_up_to_it() {
  let index = index();
  // At 0 the picture's first chunk (1000..1500) and the sound's first (5000..5050) play: the sound is further.
  assert_eq!(index.byte_at(0.0), 5050);
  assert_eq!(index.byte_at(0.4), 5050);
  // At 0.5 the picture's second chunk and the sound's second.
  assert_eq!(index.byte_at(0.5), 5100);
  assert_eq!(index.byte_at(1.0), 5150);
  assert_eq!(index.byte_at(60.0), 5150, "past its end, all of it");
  assert_eq!(index.byte_at(-1.0), 1500, "before its start, its first chunk");
}

#[test]
fn the_key_frames_are_those_of_the_picture_with_their_bytes() {
  assert_eq!(index().key_frames(), &[(0.0, 1000..1400), (1.0, 4000..4400)]);
}

#[test]
fn a_box_that_is_no_index_has_none() {
  assert_eq!(Index::parse(&boxed(b"mvhd", &[0; 100])), None);
  assert_eq!(Index::parse(&[0, 0, 0, 3, b'm']), None, "a box too short to be one");
}

#[test]
fn the_index_of_a_file_is_read_wherever_it_is() {
  let moov = boxed(b"moov", &track(b"vide", 1, 1, &[10, 10], 1, &[16, 26], Some(&[1])));
  let ftyp = boxed(b"ftyp", b"isom\0\0\0\0");
  let mdat = boxed(b"mdat", &[7; 20]);
  for file in [[&ftyp[..], &moov, &mdat].concat(), [&ftyp[..], &mdat, &moov].concat()] {
    let index = Index::read(&BytesReader::new(file)).expect("its index");
    assert_eq!(index.key_frames().len(), 1);
  }
  let none = [&ftyp[..], &mdat].concat();
  assert_eq!(Index::read(&BytesReader::new(none)), None);
}

#[test]
fn the_gallerys_clip_has_its_whole_length_and_its_key_frames_in_its_index() {
  let clip = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../gallery/assets/media/sample.mp4");
  let reader = FileReader::open(&clip).unwrap();
  let len = reader.len().unwrap();
  let index = Index::read(&reader).expect("an index");
  assert!(!index.key_frames().is_empty());
  assert!(index.key_frames().iter().all(|(_, bytes)| bytes.end <= len));
  // What plays grows with time, and all of it is before the end of the file.
  let mut last = 0;
  for tenth in 0..200 {
    let at = index.byte_at(f64::from(tenth) / 10.0);
    assert!(at >= last && at <= len, "{at} at {tenth}");
    last = at;
  }
  assert!(last > len / 2, "its end needs most of it: {last} of {len}");
}
