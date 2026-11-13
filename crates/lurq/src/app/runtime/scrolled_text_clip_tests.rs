//! Text cut by the edge it scrolls under is drawn up to that edge and no
//! further, wherever a page scrolled by fractions of a pixel puts it: the
//! lines of a multiline text input scrolled under its border, the long line
//! of a single-line input, and a text taller than the parent that clips it.
//!
//! The render list says it: the clip each glyph is drawn under. A text cut
//! to its own box keeps a pixel of slop around it for its glyphs' rounding;
//! on a side where the clip cuts the text, that pixel, and the scissor's
//! rounding of it, drew the next row of the cut line over the input's border
//! and, at a scale of 1.5, past the box's edge.

use std::sync::{Arc, Mutex};

use raw_window_handle::{DisplayHandle, WindowHandle};

use super::Tree;
use crate::{
  app::{App, capture_test_engine::TestSurface, events::ScrollPhase, render_engine::RenderEngine},
  components::{Column, Row, ScrollVertical, Text, TextInput},
  core::Signal,
  layout::{
    quad::ClipRect,
    render_list::{GlyphCmd, RenderList},
  },
  node::{border::Border, padding::Padding},
};

const SCALE: f32 = 1.5;
const AREA: &str = "area";
const LINE: &str = "line";
const CLIPPED: &str = "clipped";
const PADDING_X: f32 = 12.0;
const BORDER: f32 = 1.0;
/// A clip edge is where layout put it, give or take float error.
const EPSILON: f32 = 0.01;

/// Keeps the glyphs of the last list it drew.
struct GlyphRecorder {
  glyphs: Arc<Mutex<Vec<GlyphCmd>>>,
}

impl RenderEngine for GlyphRecorder {
  fn resize(&mut self, _: u32, _: u32) {}

  fn render(&mut self, list: &RenderList, _: WindowHandle<'_>, _: DisplayHandle<'_>) -> bool {
    *self.glyphs.lock().unwrap() = list.glyphs.clone();
    true
  }
}

/// A box in physical pixels: left, top, right, bottom.
type Edges = [f32; 4];

struct Page {
  tree: Tree,
  app: App,
  glyphs: Arc<Mutex<Vec<GlyphCmd>>>,
}

impl Page {
  fn new() -> Self {
    let glyphs = Arc::new(Mutex::new(Vec::new()));
    let recorded = glyphs.clone();
    let mut tree = Tree::new();
    tree.set_render_engine_factory(move || {
      Box::new(GlyphRecorder {
        glyphs: recorded.clone(),
      }) as Box<dyn RenderEngine>
    });
    tree.set_scale_factor(SCALE);
    tree.resize(900, 600);
    let lines = (1..=12)
      .map(|line| format!("line {line} of the text, gjpqy"))
      .collect::<Vec<_>>()
      .join("\n");
    let boxed = |input: TextInput| {
      input
        .padding(Padding::symmetric(PADDING_X, 8.0))
        .border(Border::inside(BORDER, "#6b7280"))
        .rounded(6.0)
        .background("#0b0b0f")
    };
    let filler = |height: f32| Column::new().height(height).width(40.0).background("#1f2937");
    tree.set_root(ScrollVertical::new(
      Column::new()
        .padding(24.0)
        .spacing(16.0)
        .child(filler(200.0))
        .child(
          Row::new()
            .spacing(24.0)
            .child(
              boxed(TextInput::new(Signal::new(lines)).multiline().rows(3, 8))
                .id(AREA)
                .width(240.0),
            )
            .child(
              Column::new()
                .spacing(16.0)
                .child(
                  boxed(TextInput::new(Signal::new(
                    "A single-line input whose text runs well past its right edge".to_owned(),
                  )))
                  .id(LINE)
                  .width(160.0),
                )
                .child(
                  Column::new()
                    .id(CLIPPED)
                    .width(200.0)
                    .height(30.0)
                    .clip()
                    .child(Text::new("A text taller than\nthe parent that clips it\nthird line")),
                ),
            ),
        )
        .child(filler(900.0)),
    ));
    let mut page = Self {
      tree,
      app: App::new(),
      glyphs,
    };
    page.draw();
    page
  }

  fn draw(&mut self) {
    assert!(self.tree.pass(&mut self.app, &TestSurface).rendered, "the pass draws");
  }

  /// The element's box in physical pixels, let in by `inset` logical pixels across and down.
  fn edges(&mut self, id: &str, inset: (f32, f32)) -> Edges {
    let bounds = self
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.bounds())
      .expect("laid out");
    [
      (bounds.x + inset.0) * SCALE,
      (bounds.y + inset.1) * SCALE,
      (bounds.x + bounds.width - inset.0) * SCALE,
      (bounds.y + bounds.height - inset.1) * SCALE,
    ]
  }

  /// A wheel notch at a point in logical pixels.
  fn wheel(&mut self, (x, y): (f32, f32), delta_y: f32) {
    let (x, y) = (x * SCALE, y * SCALE);
    self.tree.mouse_move(x, y);
    self.tree.scroll(x, y, 0.0, 0.0, ScrollPhase::Start);
    self.tree.scroll(x, y, 0.0, delta_y, ScrollPhase::Scroll);
    self.tree.scroll(x, y, 0.0, 0.0, ScrollPhase::End);
    self.draw();
  }

  /// The glyphs of the texts clipped inside the element `id`: those whose clip lies in its box, give or take the
  /// two pixels a clip might be let out by.
  fn glyphs_in(&mut self, id: &str) -> Vec<GlyphCmd> {
    let bounds = self.edges(id, (0.0, 0.0));
    self
      .glyphs
      .lock()
      .unwrap()
      .iter()
      .filter(|glyph| {
        glyph.clip.active && {
          let clip = clip_edges(glyph.clip);
          clip[0] >= bounds[0] - 2.0
            && clip[1] >= bounds[1] - 2.0
            && clip[2] <= bounds[2] + 2.0
            && clip[3] <= bounds[3] + 2.0
        }
      })
      .cloned()
      .collect()
  }
}

fn clip_edges(clip: ClipRect) -> Edges {
  assert!(clip.active, "a cut text is drawn under a clip");
  [clip.x, clip.y, clip.x + clip.width, clip.y + clip.height]
}

/// Where a glyph reaches past `edges`, its clip cuts it at them: not a pixel further. Returns how many glyphs
/// reached past the top, the bottom and the right.
fn assert_cut_at(glyphs: &[GlyphCmd], edges: Edges, what: &str) -> [usize; 3] {
  let mut cut = [0; 3];
  for glyph in glyphs {
    let clip = clip_edges(glyph.clip);
    if glyph.y < edges[1] {
      cut[0] += 1;
      assert!(
        clip[1] >= edges[1] - EPSILON,
        "{what}: a line cut at the top is drawn above it: clip top {} over edge {}",
        clip[1],
        edges[1]
      );
    }
    if glyph.y + glyph.height > edges[3] {
      cut[1] += 1;
      assert!(
        clip[3] <= edges[3] + EPSILON,
        "{what}: a line cut at the bottom is drawn below it: clip bottom {} under edge {}",
        clip[3],
        edges[3]
      );
    }
    if glyph.x + glyph.width > edges[2] {
      cut[2] += 1;
      assert!(
        clip[2] <= edges[2] + EPSILON,
        "{what}: a line cut at the right is drawn past it: clip right {} past edge {}",
        clip[2],
        edges[2]
      );
    }
  }
  cut
}

#[test]
fn text_scrolled_under_an_edge_is_cut_at_it_as_the_page_scrolls() {
  let mut page = Page::new();
  let (mut top, mut bottom, mut right, mut below) = (0, 0, 0, 0);
  // The page moves by fractions of a physical pixel, the pointer beside the inputs.
  for delta in [-7.3, -11.1, -5.6, -13.9, -3.3, -9.7, -6.6, -12.4] {
    // The input, shown at its end, scrolls back a little each time, so its lines are cut at its top and at its bottom
    // where they fall.
    let area = page.edges(AREA, (0.0, 0.0));
    page.wheel(
      ((area[0] + area[2]) / 2.0 / SCALE, (area[1] + area[3]) / 2.0 / SCALE),
      4.7,
    );
    let area = page.edges(AREA, (0.0, 0.0));
    page.wheel((area[2] / SCALE + 300.0, area[1] / SCALE + 10.0), delta);

    // A multiline input shows its text from its top border to its bottom border.
    let inside_border = page.edges(AREA, (BORDER, BORDER));
    let cut = assert_cut_at(&page.glyphs_in(AREA), inside_border, "multiline input");
    top += cut[0];
    bottom += cut[1];

    // A single-line input shows its text inside its padding.
    let content = page.edges(LINE, (PADDING_X, 0.0));
    right += assert_cut_at(&page.glyphs_in(LINE), content, "single-line input")[2];

    let clipped = page.edges(CLIPPED, (0.0, 0.0));
    below += assert_cut_at(&page.glyphs_in(CLIPPED), clipped, "clipped text")[1];
  }

  assert!(
    top > 0 && bottom > 0,
    "the multiline input's text is cut at both its edges"
  );
  assert!(right > 0, "the single-line input's text is cut at its right");
  assert!(below > 0, "the clipped text is cut at its parent's bottom");
}
