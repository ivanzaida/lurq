//! Layout of the rows of a table, whose cells are in the tracks of columns the
//! rows share (`layout/column_tracks.rs`): a row lays its cells out at the
//! widths of the tracks, and what holds the rows measures their cells first.

use std::sync::Arc;

use super::{ChildLayoutOverride, LayoutEngine};
use crate::{
  app::glyph_engine::GlyphEngine,
  layout::{
    Alignment, Constraints, Offset, Size,
    column_tracks::ColumnTracks,
    layout_kind::Position,
    layout_result::{ChildLayout, LayoutResult, SubtreeHolds},
  },
  node::node::Node,
};

/// How deep under what measures the tracks their rows are looked for: the rows of a table are in its body, which
/// may be in a box of its own beside the header.
const ROW_DEPTH: usize = 3;

/// Whether a child of a row takes a track: an overlay's declaration takes no room, and an absolutely positioned
/// child is placed over the row.
fn takes_track(child: &Node) -> bool {
  !child.is_overlay_declaration() && !matches!(child.position(), Position::Absolute { .. })
}

/// The rows in `node` that lay their cells out in `tracks`, and the nodes between `node` and them.
fn collect_rows<'a>(
  node: &'a Node,
  tracks: &ColumnTracks,
  depth: usize,
  rows: &mut Vec<&'a Node>,
  between: &mut Vec<&'a Node>,
) {
  for child in node.children() {
    if child.column_tracks() == Some(tracks) {
      rows.push(child);
    } else if depth < ROW_DEPTH && !child.children().is_empty() {
      let before = rows.len();
      collect_rows(child, tracks, depth + 1, rows, between);
      if rows.len() > before {
        between.push(child);
      }
    }
  }
}

fn empty_result() -> Arc<LayoutResult> {
  Arc::new(LayoutResult {
    size: Size::new(0.0, 0.0),
    children: Vec::new(),
    text_layout: None,
    dropped: false,
    holds: SubtreeHolds::UNKNOWN,
    generation: 0,
    sticky_shifts: None,
    reach: crate::layout::layout_result::Reach::UNKNOWN,
  })
}

impl LayoutEngine {
  /// Measures the cells of the rows in `node` whose tracks are sized by what is in them, and keeps in `tracks`
  /// the widest each has had. Where a track widened, the rows laid out at its old width are laid out again.
  pub(super) fn measure_column_tracks(&self, glyph_engine: &mut GlyphEngine, node: &Node, tracks: &ColumnTracks) {
    let count = tracks.len();
    let measured: Vec<bool> = (0..count).map(|index| tracks.measures(index)).collect();
    if !measured.contains(&true) {
      return;
    }
    let (mut rows, mut between) = (Vec::new(), Vec::new());
    collect_rows(node, tracks, 1, &mut rows, &mut between);
    let mut widths = vec![0.0_f32; count];
    for row in &rows {
      let mut track = 0;
      for cell in row.children().iter().filter(|child| takes_track(child)) {
        if track >= count {
          break;
        }
        let span = cell.track_span();
        // What spans tracks stands for cells that are not built: it measures none of them.
        if span == 1 && measured[track] {
          let natural = self.layout_node(glyph_engine, cell, Constraints::unbounded());
          widths[track] = widths[track].max(natural.size.width);
        }
        track += span;
      }
    }
    if tracks.record(&widths) {
      for laid_out in rows.iter().chain(&between) {
        laid_out.layout_cache.invalidate();
      }
    }
  }

  /// Lays the children of a row of a table out side by side, one to a track of `tracks`, each as wide as its
  /// track and `gap` apart. The row is as tall as its tallest cell, and a cell sits in it as `align` says.
  #[allow(clippy::too_many_arguments)]
  pub(super) fn layout_track_row(
    &self,
    glyph_engine: &mut GlyphEngine,
    node: &Node,
    constraints: Constraints,
    tracks: &ColumnTracks,
    gap: f32,
    align: Alignment,
    child_overrides: Option<&[Option<ChildLayoutOverride>]>,
  ) -> LayoutResult {
    let children = node.children();
    let gap = if gap.is_finite() { gap.max(0.0) } else { 0.0 };
    let available = constraints.max_width.is_finite().then_some(constraints.max_width);
    let widths = tracks.widths(available, gap);
    let mut starts = Vec::with_capacity(widths.len());
    let mut length = 0.0;
    for (index, width) in widths.iter().enumerate() {
      if index > 0 {
        length += gap;
      }
      starts.push(length);
      length += width;
    }

    let cells: Vec<usize> = (0..children.len())
      .filter(|index| takes_track(&children[*index]))
      .collect();
    // The first track of each cell and how wide it is: a cell that spans tracks is as wide as they are, with the
    // gaps between them.
    let mut placed: Vec<(usize, f32)> = Vec::with_capacity(cells.len());
    let mut next = 0;
    for index in &cells {
      let span = children[*index].track_span().max(1);
      let end = (next + span).min(widths.len());
      let width = if next < end {
        widths[next..end].iter().sum::<f32>() + gap * (end - next - 1) as f32
      } else {
        0.0
      };
      placed.push((next, width));
      next += span;
    }
    let mut results: Vec<Option<Arc<LayoutResult>>> = vec![None; children.len()];
    let mut tallest = 0.0_f32;
    for (slot, index) in cells.iter().enumerate() {
      let width = placed[slot].1;
      let in_its_track = Constraints {
        min_width: width,
        max_width: width,
        min_height: 0.0,
        max_height: constraints.max_height,
      };
      let result = self.layout_child_node(glyph_engine, child_overrides, *index, &children[*index], in_its_track);
      tallest = tallest.max(result.size.height);
      results[*index] = Some(result);
    }
    let size = constraints.constrain(Size::new(length, tallest));

    // A cell stretched to the row is laid out as tall as it.
    if align == Alignment::Stretch {
      for (slot, index) in cells.iter().enumerate() {
        let laid_out = results[*index].as_ref().map_or(0.0, |result| result.size.height);
        if (laid_out - size.height).abs() < 0.01 {
          continue;
        }
        let width = placed[slot].1;
        let in_the_row = Constraints::tight(Size::new(width, size.height));
        results[*index] =
          Some(self.layout_child_node(glyph_engine, child_overrides, *index, &children[*index], in_the_row));
      }
    }

    // Children that take no track: overlays' declarations have no size, and absolutely positioned ones are laid
    // out in the row's room, as a stack lays them out.
    for (index, child) in children.iter().enumerate() {
      if results[index].is_some() {
        continue;
      }
      results[index] = Some(if child.is_overlay_declaration() {
        empty_result()
      } else {
        let room = Constraints::loose(Size::new(size.width, size.height));
        self.layout_child_node(glyph_engine, child_overrides, index, child, room)
      });
    }

    let mut track_of_child = vec![None; children.len()];
    for (slot, index) in cells.iter().enumerate() {
      track_of_child[*index] = Some(placed[slot].0);
    }
    let child_layouts = children
      .iter()
      .zip(results)
      .zip(track_of_child)
      .map(|((child, result), track)| {
        let result = result.expect("every child is laid out above");
        let offset = match (track, child.position()) {
          (Some(track), _) => Offset::new(
            starts.get(track).copied().unwrap_or(length),
            align.cross_offset(size.height, result.size.height),
          ),
          (None, Position::Absolute { x, y, .. }) => Offset::new(x, y),
          (None, _) => Offset::default(),
        };
        ChildLayout {
          offset: if child.is_overlay_declaration() {
            Offset::default()
          } else {
            Self::apply_relative_position(child, offset, result.size)
          },
          result,
        }
      })
      .collect();

    LayoutResult {
      size,
      children: child_layouts,
      text_layout: None,
      dropped: false,
      holds: SubtreeHolds::UNKNOWN,
      generation: 0,
      sticky_shifts: None,
      reach: crate::layout::layout_result::Reach::UNKNOWN,
    }
  }
}

#[cfg(test)]
mod tests {
  use crate::{
    components::{Column, Row, Text},
    layout::{
      Constraints,
      column_tracks::{ColumnTracks, TrackSpec},
      layout_engine::LayoutEngine,
    },
    node::Element,
  };

  fn lay_out(element: impl Into<Element>, width: f32) -> crate::layout::layout_result::LayoutResult {
    let mut glyph_engine = crate::app::glyph_engine::GlyphEngine::new();
    let engine = LayoutEngine::new();
    let node = element.into().into_node();
    let constraints = Constraints {
      min_width: width,
      max_width: width,
      min_height: 0.0,
      max_height: f32::INFINITY,
    };
    engine
      .layout_node(&mut glyph_engine, &node, constraints)
      .as_ref()
      .clone()
  }

  fn row(tracks: &ColumnTracks, cells: [&str; 3]) -> Row {
    Row::new()
      .spacing(16.0)
      .column_tracks(tracks)
      .with_children(cells.map(Text::new))
  }

  #[test]
  fn every_row_puts_its_cells_at_the_widths_of_the_tracks_whatever_is_in_them() {
    let tracks = ColumnTracks::new([TrackSpec::share(1.0), TrackSpec::content(), TrackSpec::px(90.0)]);
    let table = Column::new()
      .measures_column_tracks(&tracks)
      .child(row(&tracks, ["Name", "Status", "Updated"]))
      .child(row(&tracks, ["crates/lurq", "A much longer status", "2 min ago"]))
      .child(row(&tracks, ["crates/lurq_macros", "Ok", "just now"]));
    let result = lay_out(table, 640.0);

    let starts = |row: usize| -> Vec<f32> {
      result.children[row]
        .result
        .children
        .iter()
        .map(|cell| cell.offset.x)
        .collect()
    };
    let widths = |row: usize| -> Vec<f32> {
      result.children[row]
        .result
        .children
        .iter()
        .map(|cell| cell.result.size.width)
        .collect()
    };
    assert_eq!(starts(0), starts(1));
    assert_eq!(starts(1), starts(2));
    assert_eq!(widths(0), widths(2));
    // The column sized by its content is as wide as its widest cell, the longer status.
    let status = tracks.content_widths()[1];
    assert!(status > 60.0, "the widest status was measured: {status}");
    assert!((widths(0)[1] - status).abs() < 0.5);
    // The share takes the rest: 640 less the status, 90 and two gaps.
    assert!((widths(0)[0] - (640.0 - status - 90.0 - 32.0)).abs() < 0.5);
    assert!((result.children[0].result.size.width - 640.0).abs() < 0.5);
  }

  #[test]
  fn a_child_that_spans_tracks_is_as_wide_as_they_are_and_what_comes_after_it_is_in_its_own_track() {
    let tracks = ColumnTracks::new([
      TrackSpec::px(50.0),
      TrackSpec::px(60.0),
      TrackSpec::px(70.0),
      TrackSpec::content(),
    ]);
    // The row builds its first and last cells, and stands one room for the two between them.
    let spanned = Row::new()
      .spacing(10.0)
      .column_tracks(&tracks)
      .child(Text::new("a"))
      .child(Row::new().track_span(2))
      .child(Text::new("a long last cell"));
    let full = row4(&tracks);
    let table = Column::new().measures_column_tracks(&tracks).child(full).child(spanned);
    let result = lay_out(table, 400.0);

    let full = &result.children[0].result.children;
    let spanned = &result.children[1].result.children;
    assert_eq!(
      spanned[1].offset.x, full[1].offset.x,
      "the room starts where the second track does"
    );
    assert_eq!(
      spanned[1].result.size.width,
      60.0 + 10.0 + 70.0,
      "the two tracks and the gap between them"
    );
    assert_eq!(spanned[2].offset.x, full[3].offset.x, "the last cell in its own track");
    assert!(
      tracks.content_widths()[3] > 60.0,
      "a cell after a room is measured in its track"
    );
  }

  fn row4(tracks: &ColumnTracks) -> Row {
    Row::new()
      .spacing(10.0)
      .column_tracks(tracks)
      .with_children(["a", "b", "c", "d"].map(Text::new))
  }

  #[test]
  fn a_track_keeps_its_width_when_its_widest_cell_goes() {
    let tracks = ColumnTracks::new([TrackSpec::content(), TrackSpec::share(1.0), TrackSpec::px(40.0)]);
    let wide = Column::new()
      .measures_column_tracks(&tracks)
      .child(row(&tracks, ["A rather long name", "b", "c"]));
    lay_out(wide, 500.0);
    let measured = tracks.content_widths()[0];

    let narrow = Column::new()
      .measures_column_tracks(&tracks)
      .child(row(&tracks, ["A", "b", "c"]));
    let result = lay_out(narrow, 500.0);

    assert!((result.children[0].result.children[0].result.size.width - measured).abs() < 0.5);
  }

  #[test]
  fn rows_in_a_box_of_their_own_are_measured_too() {
    let tracks = ColumnTracks::new([TrackSpec::content(), TrackSpec::share(1.0)]);
    let table = Column::new()
      .measures_column_tracks(&tracks)
      .child(
        Row::new()
          .column_tracks(&tracks)
          .child(Text::new("Name"))
          .child(Text::new("")),
      )
      .child(
        Column::new().child(
          Row::new()
            .column_tracks(&tracks)
            .child(Text::new("A longer name"))
            .child(Text::new("")),
        ),
      );
    let result = lay_out(table, 400.0);

    let header = result.children[0].result.children[0].result.size.width;
    let body = result.children[1].result.children[0].result.children[0]
      .result
      .size
      .width;
    assert!((header - body).abs() < 0.5);
    assert!((header - tracks.content_widths()[0]).abs() < 0.5);
  }
}
