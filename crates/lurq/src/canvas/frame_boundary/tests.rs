use super::*;

const A: CanvasId = CanvasId(1);
const B: CanvasId = CanvasId(2);

/// Replacements open on each canvas, as a renderer's `fronts` hold them.
type Fronts = HashMap<CanvasId, (u64, ())>;

fn open(fronts: &mut Fronts, id: CanvasId, token: u64) {
  fronts.insert(id, (token, ()));
}

#[test]
fn every_encode_ends_the_frame_while_no_replacement_is_open() {
  let mut boundary = FrameBoundary::default();
  let fronts = Fronts::new();
  for _ in 0..3 {
    boundary.begin_encode();
    assert!(boundary.end_encode(&fronts));
  }
}

#[test]
fn a_replacement_drawn_over_several_encodes_is_one_frame() {
  let mut boundary = FrameBoundary::default();
  let mut fronts = Fronts::new();
  // The encode that begins the replacement draws its first batch.
  boundary.begin_encode();
  open(&mut fronts, A, 1);
  assert!(!boundary.end_encode(&fronts), "the first batch does not end the frame");
  boundary.begin_encode();
  assert!(!boundary.end_encode(&fronts), "nor do the batches in between");
  // The last batch commits it.
  boundary.begin_encode();
  assert!(boundary.end_replacement(&mut fronts, A).is_some());
  assert!(boundary.end_encode(&fronts));
}

#[test]
fn a_replacement_begun_and_committed_in_one_encode_ends_it() {
  let mut boundary = FrameBoundary::default();
  let mut fronts = Fronts::new();
  boundary.begin_encode();
  open(&mut fronts, A, 1);
  boundary.end_replacement(&mut fronts, A);
  assert!(boundary.end_encode(&fronts));
}

#[test]
fn superseding_a_replacement_ends_the_frame_and_its_successor_starts_the_next() {
  let mut boundary = FrameBoundary::default();
  let mut fronts = Fronts::new();
  boundary.begin_encode();
  open(&mut fronts, A, 1);
  assert!(!boundary.end_encode(&fronts));
  // A new begin aborts the open replacement and opens its own.
  boundary.begin_encode();
  boundary.end_replacement(&mut fronts, A);
  open(&mut fronts, A, 2);
  assert!(boundary.end_encode(&fronts), "the superseded batches were a frame");
  boundary.begin_encode();
  assert!(!boundary.end_encode(&fronts), "the successor holds the next one open");
}

#[test]
fn any_canvas_holding_a_replacement_keeps_the_frame_open() {
  let mut boundary = FrameBoundary::default();
  let mut fronts = Fronts::new();
  boundary.begin_encode();
  open(&mut fronts, A, 1);
  assert!(!boundary.end_encode(&fronts));
  // B presents a whole replacement in every encode while A is still drawing.
  for token in 1..4 {
    boundary.begin_encode();
    open(&mut fronts, B, token);
    boundary.end_replacement(&mut fronts, B);
    assert!(!boundary.end_encode(&fronts), "A's page is not finished");
  }
  boundary.begin_encode();
  boundary.end_replacement(&mut fronts, A);
  assert!(boundary.end_encode(&fronts));
}

#[test]
fn an_encode_that_failed_part_way_ended_nothing() {
  let mut boundary = FrameBoundary::default();
  let mut fronts = Fronts::new();
  boundary.begin_encode();
  open(&mut fronts, A, 1);
  boundary.end_replacement(&mut fronts, A);
  open(&mut fronts, A, 2);
  // The encode returned an error before `end_encode`; the next one only
  // continues replacement 2.
  boundary.begin_encode();
  assert!(!boundary.end_encode(&fronts));
}
