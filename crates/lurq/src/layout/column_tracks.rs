//! Columns that the rows of a table share: one list of tracks, whose widths
//! every row lays its cells out at, whatever is in a cell.
//!
//! **Rows.** A row given the tracks (`.column_tracks(&tracks)`) lays its
//! children out side by side, one to a track, each as wide as its track and
//! its gap apart, as a grid lays out one row. The header of a table and
//! each of its rows are such rows, each its own element, so that a row is
//! pressed, hovered and selected as one.
//!
//! **Widths.** A track is a length in pixels, a share of the room the others
//! and the gaps leave, or as wide as its content, each between a least and a
//! most. The content of a track is measured by what holds the rows
//! (`.measures_column_tracks(&tracks)`): when it is laid out it measures the
//! cells of the rows in it, and keeps the widest a track has had, so that a
//! column does not narrow as the rows in it change while they scroll. The
//! rows that are not built are not measured: a table of thousands of rows
//! builds those in view, and its columns are as wide as what has been seen.
//!
//! The tracks are kept by whatever builds the table from render to render: a
//! row whose tracks are another list than before is laid out again, so a
//! table whose columns change makes a new list.

use std::sync::Arc;

use parking_lot::Mutex;

/// How wide a track is, before its least and its most.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TrackWidth {
  /// A length in logical pixels.
  Px(f32),
  /// A share of the room the other tracks and the gaps leave: two tracks of
  /// `Share(1.0)` and `Share(2.0)` split it one to two. Where the room is not
  /// known (a row as wide as its content), as wide as its content.
  Share(f32),
  /// As wide as the widest of its cells that has been laid out.
  Content,
}

/// One track: how wide it is, and the least and the most it may be.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrackSpec {
  pub width: TrackWidth,
  pub min: f32,
  pub max: f32,
}

impl TrackSpec {
  pub fn new(width: TrackWidth) -> Self {
    Self {
      width,
      min: 0.0,
      max: f32::INFINITY,
    }
  }

  /// A track of `px` logical pixels.
  pub fn px(px: f32) -> Self {
    Self::new(TrackWidth::Px(px))
  }

  /// A track that takes `share` of the room left.
  pub fn share(share: f32) -> Self {
    Self::new(TrackWidth::Share(share))
  }

  /// A track as wide as its content.
  pub fn content() -> Self {
    Self::new(TrackWidth::Content)
  }

  /// The least it may be.
  pub fn min(mut self, min: f32) -> Self {
    self.min = min.max(0.0);
    self
  }

  /// The most it may be.
  pub fn max(mut self, max: f32) -> Self {
    self.max = max.max(0.0);
    self
  }

  /// Whether what is in it decides its width anywhere: a track in pixels never.
  fn measured(&self) -> bool {
    !matches!(self.width, TrackWidth::Px(_))
  }

  fn clamp(&self, width: f32) -> f32 {
    let width = if width.is_finite() { width } else { 0.0 };
    width.min(self.max).max(self.min)
  }
}

struct Inner {
  specs: Vec<TrackSpec>,
  /// The widest cell of each track that has been measured.
  content: Vec<f32>,
}

/// The tracks of a table's columns, shared by its rows and by what holds
/// them. A handle: clones are the same list, and two lists are equal only
/// where they are the same.
///
/// ```
/// use lurq::{
///   components::{Column, Row, Text},
///   layout::column_tracks::{ColumnTracks, TrackSpec},
/// };
///
/// let tracks = ColumnTracks::new([TrackSpec::share(1.0).min(120.0), TrackSpec::content(), TrackSpec::px(80.0)]);
/// let row = |name: &str, kind: &str, size: &str| {
///   Row::new()
///     .spacing(16.0)
///     .column_tracks(&tracks)
///     .child(Text::new(name))
///     .child(Text::new(kind))
///     .child(Text::new(size))
/// };
/// let table = Column::new()
///   .measures_column_tracks(&tracks)
///   .child(row("Name", "Kind", "Size"))
///   .child(row("blur.rs", "Rust", "12 KB"));
/// ```
#[derive(Clone)]
pub struct ColumnTracks(Arc<Mutex<Inner>>);

impl PartialEq for ColumnTracks {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

impl std::fmt::Debug for ColumnTracks {
  fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let inner = self.0.lock();
    formatter
      .debug_struct("ColumnTracks")
      .field("specs", &inner.specs)
      .field("content", &inner.content)
      .finish()
  }
}

impl ColumnTracks {
  pub fn new(specs: impl IntoIterator<Item = TrackSpec>) -> Self {
    let specs: Vec<TrackSpec> = specs.into_iter().collect();
    let content = vec![0.0; specs.len()];
    Self(Arc::new(Mutex::new(Inner { specs, content })))
  }

  /// How many tracks there are.
  pub fn len(&self) -> usize {
    self.0.lock().specs.len()
  }

  pub fn is_empty(&self) -> bool {
    self.len() == 0
  }

  pub fn specs(&self) -> Vec<TrackSpec> {
    self.0.lock().specs.clone()
  }

  /// The widest cell of each track measured so far.
  pub fn content_widths(&self) -> Vec<f32> {
    self.0.lock().content.clone()
  }

  /// Whether the cells of track `index` are measured.
  pub(crate) fn measures(&self, index: usize) -> bool {
    self.0.lock().specs.get(index).is_some_and(TrackSpec::measured)
  }

  /// Takes in the widths of the cells measured in one layout, one for each
  /// track: a track keeps the widest it has had. Whether any track widened.
  pub(crate) fn record(&self, widths: &[f32]) -> bool {
    let mut inner = self.0.lock();
    let mut widened = false;
    for (held, width) in inner.content.iter_mut().zip(widths) {
      if width.is_finite() && *width > *held + 0.01 {
        *held = *width;
        widened = true;
      }
    }
    widened
  }

  /// The width of each track in a row whose cells have `available` between
  /// its paddings, where that is known, with `gap` between two tracks.
  pub fn widths(&self, available: Option<f32>, gap: f32) -> Vec<f32> {
    let inner = self.0.lock();
    resolve_tracks(&inner.specs, &inner.content, available, gap)
  }
}

/// The widths of `specs` whose cells measured `content`, in `available`
/// where it is known, `gap` apart. A track in pixels and one sized by its
/// content keep their widths; the tracks with a share split what is left
/// between their least and their most. Where the tracks do not fit, nothing
/// gives way below its least: the row is wider than its room.
pub(crate) fn resolve_tracks(specs: &[TrackSpec], content: &[f32], available: Option<f32>, gap: f32) -> Vec<f32> {
  let measured = |index: usize| content.get(index).copied().unwrap_or(0.0);
  let available = available.filter(|width| width.is_finite());
  let shares: Vec<Option<f32>> = specs
    .iter()
    .map(|spec| match spec.width {
      TrackWidth::Share(share) if available.is_some() && share.is_finite() && share > 0.0 => Some(share),
      _ => None,
    })
    .collect();
  let mut widths: Vec<f32> = specs
    .iter()
    .enumerate()
    .map(|(index, spec)| match spec.width {
      TrackWidth::Px(px) => spec.clamp(px),
      TrackWidth::Content | TrackWidth::Share(_) => spec.clamp(measured(index)),
    })
    .collect();
  let Some(available) = available else {
    return widths;
  };
  let gaps = gap.max(0.0) * specs.len().saturating_sub(1) as f32;
  // The shares split what the rest leave. One its least or its most holds keeps that, and the others split
  // what is then left, until none is held.
  let mut free: Vec<usize> = (0..specs.len()).filter(|index| shares[*index].is_some()).collect();
  loop {
    let taken: f32 = (0..specs.len())
      .filter(|index| !free.contains(index))
      .map(|index| widths[index])
      .sum();
    let room = (available - taken - gaps).max(0.0);
    let total: f32 = free.iter().filter_map(|index| shares[*index]).sum();
    if free.is_empty() || total <= 0.0 {
      break;
    }
    let mut held = None;
    for &index in &free {
      let wanted = room * shares[index].unwrap_or(0.0) / total;
      widths[index] = specs[index].clamp(wanted);
      if (widths[index] - wanted).abs() > 0.01 && held.is_none() {
        held = Some(index);
      }
    }
    match held {
      Some(index) => free.retain(|free| *free != index),
      None => break,
    }
  }
  widths
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn shares_split_what_pixels_content_and_gaps_leave() {
    let specs = [TrackSpec::share(1.0), TrackSpec::content(), TrackSpec::px(120.0)];
    let widths = resolve_tracks(&specs, &[300.0, 64.0, 0.0], Some(608.0), 16.0);

    // 608 less 64, 120 and two gaps of 16 is 392, all of it the share's, whatever its content measured.
    assert_eq!(widths, [392.0, 64.0, 120.0]);
  }

  #[test]
  fn shares_split_in_proportion_and_one_held_by_its_most_gives_the_rest_to_the_others() {
    let specs = [TrackSpec::share(1.0), TrackSpec::share(3.0)];
    assert_eq!(resolve_tracks(&specs, &[0.0, 0.0], Some(400.0), 0.0), [100.0, 300.0]);

    let held = [TrackSpec::share(1.0), TrackSpec::share(3.0).max(200.0)];
    assert_eq!(resolve_tracks(&held, &[0.0, 0.0], Some(400.0), 0.0), [200.0, 200.0]);
  }

  #[test]
  fn a_share_keeps_its_least_and_the_row_is_wider_than_its_room() {
    let specs = [TrackSpec::share(1.0).min(200.0), TrackSpec::px(300.0)];
    let widths = resolve_tracks(&specs, &[0.0, 0.0], Some(320.0), 16.0);

    assert_eq!(widths, [200.0, 300.0]);
  }

  #[test]
  fn where_the_room_is_not_known_a_share_is_as_wide_as_its_content() {
    let specs = [TrackSpec::share(1.0).min(80.0), TrackSpec::share(1.0)];
    let widths = resolve_tracks(&specs, &[40.0, 150.0], None, 16.0);

    assert_eq!(widths, [80.0, 150.0]);
  }

  #[test]
  fn content_is_held_between_its_least_and_its_most() {
    let specs = [TrackSpec::content().max(100.0), TrackSpec::content().min(48.0)];
    assert_eq!(resolve_tracks(&specs, &[180.0, 12.0], Some(500.0), 0.0), [100.0, 48.0]);
  }

  #[test]
  fn a_track_keeps_the_widest_cell_it_has_had() {
    let tracks = ColumnTracks::new([TrackSpec::content(), TrackSpec::px(40.0), TrackSpec::share(1.0)]);
    assert!(tracks.measures(0) && !tracks.measures(1) && tracks.measures(2));

    assert!(tracks.record(&[90.0, 0.0, 10.0]));
    assert!(!tracks.record(&[60.0, 0.0, 10.0]), "narrower cells widen nothing");
    assert!(tracks.record(&[120.0, 0.0, 0.0]));
    assert_eq!(tracks.content_widths(), [120.0, 0.0, 10.0]);
    assert_eq!(tracks.widths(Some(400.0), 0.0), [120.0, 40.0, 240.0]);
  }

  #[test]
  fn two_lists_are_equal_only_where_they_are_the_same() {
    let tracks = ColumnTracks::new([TrackSpec::content()]);
    assert_eq!(tracks, tracks.clone());
    assert_ne!(tracks, ColumnTracks::new([TrackSpec::content()]));
  }
}
