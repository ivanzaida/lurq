//! Text by touch, as a phone has it, where the platform does not draw it itself (iOS does: see
//! [`Tree::set_native_text_selection`]):
//!
//! - **The loupe.** A long press in a text input puts the caret under the finger and shows a magnified view of the text
//!   there over it; the caret goes with the finger until it lifts, and the menu of editing commands opens over it
//!   (Select all, Paste).
//! - **The handles.** While the app's pointer is a finger ([`Pointer::Coarse`](crate::app::Pointer)), a selection in
//!   the focused text input has a handle at each end, a knob on a stem in the theme's primary color. A finger that
//!   lands on one drags that end, and the menu of editing commands opens over the selection when it lifts.
//! - **The menu.** A selection a finger made (a double tap selects a word, a triple tap a line) opens the menu of
//!   editing commands over it: Cut, Copy, Paste, Select all ([`EditMenuRequest`](crate::app::window::EditMenuRequest)).

use super::Tree;
use crate::{
  app::events::{EditCommand, TouchPhase},
  layout::{
    quad::{ClipRect, RoundedClip},
    render_list::{GlyphCmd, RectCmd},
  },
  node::{border::BorderRadius, color::Color},
};

/// How far from a handle's knob a finger lands to take it, in logical pixels: a fingertip.
const HANDLE_REACH: f32 = 24.0;
/// A handle's knob: its radius, and the width of its stem, in logical pixels.
const KNOB_RADIUS: f32 = 5.0;
const STEM_WIDTH: f32 = 2.0;
/// The loupe: its size, how far above the line it stands, and how much it magnifies.
const LOUPE_WIDTH: f32 = 120.0;
const LOUPE_HEIGHT: f32 = 44.0;
const LOUPE_GAP: f32 = 12.0;
const LOUPE_ZOOM: f32 = 1.25;

/// An end of a selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum End {
  Start,
  End,
}

#[derive(Default)]
pub(super) struct TouchText {
  /// A finger drags a handle: its id, the end it holds, and how far below the finger the line it points at is.
  dragging: Option<(u64, End, f32)>,
  /// A long press placed the caret, and the finger moves it: where the finger is, in logical pixels.
  loupe: Option<(f32, f32)>,
}

/// The focused input's selection as handles see it: where each end's caret is (its top-left corner, in logical
/// pixels), the height of a line, and the box of the input.
struct Ends {
  start: (f32, f32),
  end: (f32, f32),
  line: f32,
  bounds: (f32, f32, f32, f32),
}

impl Tree {
  /// Whether lurq draws the caret's loupe and the selection's handles: the platform draws none, and the app's pointer
  /// is a finger.
  fn touch_text_shown(&self) -> bool {
    !self.layout_engine.native_text_selection() && self.pointer() == crate::app::Pointer::Coarse
  }

  /// The ends of the focused input's selection, where it has one and was painted.
  fn selection_ends(&self) -> Option<Ends> {
    let snapshot = self.text_input_snapshot()?;
    if snapshot.selection.is_empty() {
      return None;
    }
    let geometry = snapshot.geometry?;
    let at = |index: usize| {
      geometry
        .carets
        .iter()
        .find(|(caret, ..)| *caret == index)
        .map(|(_, x, y)| (*x, *y))
    };
    Some(Ends {
      start: at(snapshot.selection.start)?,
      end: at(snapshot.selection.end)?,
      line: geometry.line_height,
      bounds: geometry.bounds,
    })
  }

  /// The place of a caret in the focused input nearest to a point in logical pixels: on the line the point is on,
  /// or the nearest line.
  fn caret_near(&self, x: f32, y: f32) -> Option<usize> {
    let geometry = self.text_input_snapshot()?.geometry?;
    let line = geometry.line_height.max(1.0);
    let cost = |(_, cx, cy): &(usize, f32, f32)| {
      let lines = ((y - (cy + line / 2.0)) / line).abs().round();
      lines * 100_000.0 + (x - cx).abs()
    };
    geometry
      .carets
      .iter()
      .min_by(|a, b| cost(a).total_cmp(&cost(b)))
      .map(|(index, ..)| *index)
  }

  /// A finger's event, in logical pixels, where it is the handles': it landed on one, and drags it. `true` when it
  /// was, and the finger is nothing else's.
  pub(super) fn touch_text_finger(&mut self, id: u64, phase: TouchPhase, x: f32, y: f32) -> bool {
    match phase {
      TouchPhase::Start => {
        if !self.touch_pointer.touches.is_empty() || !self.touch_text_shown() {
          return false;
        }
        self.rebuild_if_dirty();
        let Some(ends) = self.selection_ends() else {
          return false;
        };
        let takes = |caret: (f32, f32), knob_y: f32| {
          let on_stem = (x - caret.0).abs() <= HANDLE_REACH / 2.0 && y >= caret.1 && y <= caret.1 + ends.line;
          on_stem || (x - caret.0).hypot(y - knob_y) <= HANDLE_REACH
        };
        let end = if takes(ends.end, ends.end.1 + ends.line + KNOB_RADIUS) {
          End::End
        } else if takes(ends.start, ends.start.1 - KNOB_RADIUS) {
          End::Start
        } else {
          return false;
        };
        let caret = if end == End::Start { ends.start } else { ends.end };
        self.touch_text.dragging = Some((id, end, caret.1 + ends.line / 2.0 - y));
        self.close_edit_menu();
        self.needs_redraw = true;
        true
      }
      TouchPhase::Move => {
        let Some((held, end, below)) = self.touch_text.dragging else {
          return false;
        };
        if held != id {
          return false;
        }
        let (Some(snapshot), Some(index)) = (self.text_input_snapshot(), self.caret_near(x, y + below)) else {
          return true;
        };
        let (start, finish) = (snapshot.selection.start, snapshot.selection.end);
        // The end dragged past the other becomes the other: the selection is never empty.
        let (from, to, end) = match end {
          End::Start if index < finish => (index, finish, End::Start),
          End::Start => (finish, index.max(finish), End::End),
          End::End if index > start => (start, index, End::End),
          End::End => (index.min(start), start, End::Start),
        };
        self.touch_text.dragging = Some((held, end, below));
        if from < to {
          self.text_input_select(from..to);
        }
        true
      }
      TouchPhase::End | TouchPhase::Cancel => {
        let Some((held, ..)) = self.touch_text.dragging else {
          return false;
        };
        if held != id {
          return false;
        }
        self.touch_text.dragging = None;
        self.open_selection_menu();
        self.needs_redraw = true;
        true
      }
    }
  }

  /// A long press landed on a text input (`on_text_input`) and nothing took it: the caret goes under the finger,
  /// with the loupe over it, and follows the finger. `true` when it did.
  pub(super) fn touch_text_long_press(&mut self, x: f32, y: f32, on_text_input: bool) -> bool {
    if !on_text_input || self.layout_engine.native_text_selection() || !self.focused_text_input() {
      return false;
    }
    self.close_edit_menu();
    self.move_touch_caret(x, y);
    true
  }

  /// Whether a finger moves the caret with the loupe: then its moves move the caret and select nothing.
  pub(super) fn touch_caret_moving(&self) -> bool {
    self.touch_text.loupe.is_some()
  }

  /// The finger that moves the caret is at a point, in logical pixels.
  pub(super) fn move_touch_caret(&mut self, x: f32, y: f32) {
    self.touch_text.loupe = Some((x, y));
    if let Some(index) = self.caret_near(x, y) {
      self.text_input_select(index..index);
    }
    self.needs_redraw = true;
  }

  /// The finger that moved the caret lifted: the loupe goes, and the menu of editing commands opens over the caret.
  /// `true` when a finger was moving it.
  pub(super) fn end_touch_caret(&mut self) -> bool {
    if self.touch_text.loupe.take().is_none() {
      return false;
    }
    self.needs_redraw = true;
    if let Some(caret) = self.layout_engine.ime_cursor_area() {
      self.open_edit_menu(
        (caret.x + caret.width / 2.0, caret.y),
        vec![EditCommand::SelectAll, EditCommand::Paste],
      );
    }
    true
  }

  /// A finger's tap ended: where it left a selection in the focused input (a double tap selects a word), the menu
  /// of editing commands opens over it.
  pub(super) fn after_touch_tap(&mut self) {
    if !self.layout_engine.native_text_selection() && self.focused_text_input() {
      self.open_selection_menu();
    }
  }

  /// Opens the menu of editing commands over the focused input's selection, where it has one.
  fn open_selection_menu(&mut self) {
    self.rebuild_if_dirty();
    let Some(ends) = self.selection_ends() else {
      return;
    };
    let x = if (ends.start.1 - ends.end.1).abs() < 1.0 {
      (ends.start.0 + ends.end.0) / 2.0
    } else {
      ends.bounds.0 + ends.bounds.2 / 2.0
    };
    self.open_edit_menu(
      (x, ends.start.1 - KNOB_RADIUS * 2.0),
      vec![
        EditCommand::Cut,
        EditCommand::Copy,
        EditCommand::Paste,
        EditCommand::SelectAll,
      ],
    );
  }

  /// Paints the selection's handles and the caret's loupe over the frame: `rects` and `glyphs` are the frame's, in
  /// physical pixels, and the loupe magnifies what they draw under it.
  pub(super) fn push_touch_text_marks(
    &self,
    rects: &mut Vec<RectCmd>,
    glyphs: &mut Vec<GlyphCmd>,
    order: usize,
    scale: f32,
  ) {
    if !self.touch_text_shown() {
      return;
    }
    let (primary, background, border) = {
      let palette = self.layout_engine.palette();
      (palette.primary, palette.background, palette.border)
    };
    if self.touch_text.loupe.is_none()
      && let Some(ends) = self.selection_ends()
    {
      let (bx, by, bw, bh) = ends.bounds;
      let inside = |(x, y): (f32, f32)| x >= bx - 1.0 && x <= bx + bw + 1.0 && y >= by - ends.line && y <= by + bh;
      for (caret, end) in [(ends.start, End::Start), (ends.end, End::End)] {
        if !inside(caret) {
          continue;
        }
        rects.push(rounded(
          (caret.0 + STEM_WIDTH / 2.0, caret.1 + ends.line / 2.0),
          (STEM_WIDTH, ends.line),
          0.0,
          primary,
          order,
          scale,
        ));
        let knob_y = match end {
          End::Start => caret.1 - KNOB_RADIUS,
          End::End => caret.1 + ends.line + KNOB_RADIUS,
        };
        rects.push(rounded(
          (caret.0 + STEM_WIDTH / 2.0, knob_y),
          (KNOB_RADIUS * 2.0, KNOB_RADIUS * 2.0),
          KNOB_RADIUS,
          primary,
          order + 1,
          scale,
        ));
      }
    }
    let Some((fx, _)) = self.touch_text.loupe else {
      return;
    };
    let Some(caret) = self.layout_engine.ime_cursor_area() else {
      return;
    };
    // The loupe stands over the line the caret is on, centred on the finger, and shows what is around the caret
    // magnified.
    let source = (fx, caret.y + caret.height / 2.0);
    let center = (fx, (caret.y - LOUPE_GAP - LOUPE_HEIGHT / 2.0).max(LOUPE_HEIGHT / 2.0));
    let lens = (
      (center.0 - LOUPE_WIDTH / 2.0) * scale,
      (center.1 - LOUPE_HEIGHT / 2.0) * scale,
      LOUPE_WIDTH * scale,
      LOUPE_HEIGHT * scale,
    );
    let radius = LOUPE_HEIGHT / 2.0 * scale;
    let clip = ClipRect {
      x: lens.0,
      y: lens.1,
      width: lens.2,
      height: lens.3,
      active: true,
      rounded: [
        Some(RoundedClip {
          x: lens.0,
          y: lens.1,
          width: lens.2,
          height: lens.3,
          radius: BorderRadius::new(radius, radius, radius, radius),
        }),
        None,
      ],
    };
    let lens_rect = |color: Color, stroke: f32, stroke_color: Color, order: usize| RectCmd {
      order,
      x: lens.0,
      y: lens.1,
      width: lens.2,
      height: lens.3,
      color,
      radii: [radius; 4],
      stroke: [stroke * scale; 4],
      stroke_color,
      stroke_style: crate::layout::render_list::StrokeStyle::default(),
      transform: [1.0, 0.0, 0.0, 1.0],
      transform_origin: [0.0, 0.0],
      clip: ClipRect::default(),
      gradient: None,
      shadow: None,
    };
    let clear = Color::new(0, 0, 0, 0);
    // Above everything the frame draws: its orders are below `order`, and the copies keep theirs above `inner`.
    let base = order + 10;
    let inner = base + 1;
    let top = inner + order + 1;
    let face = lens_rect(background, 0.0, clear, base);
    let rim = lens_rect(clear, 1.0, border, top + 1);
    // What the frame draws around the caret, magnified about it into the lens.
    let (sx, sy) = (source.0 * scale, source.1 * scale);
    let (cx, cy) = (center.0 * scale, center.1 * scale);
    let reach = (lens.2 / LOUPE_ZOOM / 2.0, lens.3 / LOUPE_ZOOM / 2.0);
    let near = |x: f32, y: f32, w: f32, h: f32| {
      x < sx + reach.0 && x + w > sx - reach.0 && y < sy + reach.1 && y + h > sy - reach.1
    };
    let map = |x: f32, y: f32| (cx + (x - sx) * LOUPE_ZOOM, cy + (y - sy) * LOUPE_ZOOM);
    let copied_rects: Vec<RectCmd> = rects
      .iter()
      .filter(|rect| rect.order < order && rect.shadow.is_none() && near(rect.x, rect.y, rect.width, rect.height))
      .map(|rect| {
        let (x, y) = map(rect.x, rect.y);
        let origin = map(rect.transform_origin[0], rect.transform_origin[1]);
        RectCmd {
          order: inner + rect.order,
          x,
          y,
          width: rect.width * LOUPE_ZOOM,
          height: rect.height * LOUPE_ZOOM,
          radii: rect.radii.map(|radius| radius * LOUPE_ZOOM),
          stroke: rect.stroke.map(|stroke| stroke * LOUPE_ZOOM),
          transform_origin: [origin.0, origin.1],
          clip,
          ..rect.clone()
        }
      })
      .collect();
    let copied_glyphs: Vec<GlyphCmd> = glyphs
      .iter()
      .filter(|glyph| glyph.order < order && near(glyph.x, glyph.y, glyph.width, glyph.height))
      .map(|glyph| {
        let (x, y) = map(glyph.x, glyph.y);
        let origin = map(glyph.transform_origin[0], glyph.transform_origin[1]);
        GlyphCmd {
          order: inner + glyph.order,
          x,
          y,
          width: glyph.width * LOUPE_ZOOM,
          height: glyph.height * LOUPE_ZOOM,
          transform_origin: [origin.0, origin.1],
          clip,
          ..glyph.clone()
        }
      })
      .collect();
    rects.push(face);
    rects.extend(copied_rects);
    glyphs.extend(copied_glyphs);
    // The caret in the lens, over the copies.
    let mut caret_mark = rounded(center, (2.0, caret.height * LOUPE_ZOOM), 1.0, primary, top, scale);
    caret_mark.clip = clip;
    rects.push(caret_mark);
    rects.push(rim);
  }
}

/// A box of `size` centred on `center` with corners of `radius`, all in logical pixels, painted in `color`.
fn rounded(center: (f32, f32), size: (f32, f32), radius: f32, color: Color, order: usize, scale: f32) -> RectCmd {
  RectCmd {
    order,
    x: (center.0 - size.0 / 2.0) * scale,
    y: (center.1 - size.1 / 2.0) * scale,
    width: size.0 * scale,
    height: size.1 * scale,
    color,
    radii: [radius * scale; 4],
    stroke: [0.0; 4],
    stroke_color: Color::new(0, 0, 0, 0),
    stroke_style: crate::layout::render_list::StrokeStyle::default(),
    transform: [1.0, 0.0, 0.0, 1.0],
    transform_origin: [0.0, 0.0],
    clip: ClipRect::default(),
    gradient: None,
    shadow: None,
  }
}
