#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CaretMode {
  Persistent,
  #[default]
  Blinking,
}

/// How high a text input's caret is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CaretHeight {
  /// A share of the font size of the text the caret stands in: `1.0` is as high as the type is large. It is
  /// centred in the line. Among the rows of a multiline input it is never higher than its row; in a single-line
  /// input it may be, up to the box the text is in.
  FontSize(f32),
  /// As high as the band of the text in its line: the font's ascent and descent, which a selection covers too.
  TextBand,
}

/// The caret of a text input, and the line under what an input method composes: how the caret blinks, how wide
/// and how high it is, and how thick the line is. Its color is the text's, or the input's `caret_color`.
///
/// The default is the design's: a caret 1.5 wide and 18 high in text of 14 (nine sevenths of the font size), and
/// a line of 1.5. [`ThemeCaret::legacy`] is what lurq 0.62 drew: 1 wide, as high as the font size, a line of 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThemeCaret {
  mode: CaretMode,
  width: f32,
  height: CaretHeight,
  composition_line: f32,
}

impl ThemeCaret {
  /// The design's caret, blinking as `mode` says. An app that keeps the caret of 0.62 changes the mode alone:
  /// `ThemeCaret::legacy().with_mode(mode)`, or `Theme::set_caret_mode`.
  pub fn new(mode: CaretMode) -> Self {
    Self {
      mode,
      ..Self::default()
    }
  }

  /// The caret of lurq 0.62: 1 wide, as high as the font size, with a line of 1 under a composition.
  pub fn legacy() -> Self {
    Self {
      mode: CaretMode::Blinking,
      width: 1.0,
      height: CaretHeight::FontSize(1.0),
      composition_line: 1.0,
    }
  }

  pub fn mode(&self) -> CaretMode {
    self.mode
  }

  pub fn set_mode(&mut self, mode: CaretMode) {
    self.mode = mode;
  }

  /// The caret, blinking as `mode` says: its size stays.
  pub fn with_mode(mut self, mode: CaretMode) -> Self {
    self.mode = mode;
    self
  }

  /// How wide the caret is, in logical pixels.
  pub fn width(&self) -> f32 {
    if self.width.is_finite() { self.width.max(0.0) } else { 1.0 }
  }

  pub fn set_width(&mut self, width: f32) {
    self.width = width;
  }

  /// The caret, `width` wide.
  pub fn with_width(mut self, width: f32) -> Self {
    self.width = width;
    self
  }

  /// How high the caret is.
  pub fn height(&self) -> CaretHeight {
    self.height
  }

  pub fn set_height(&mut self, height: CaretHeight) {
    self.height = height;
  }

  /// The caret, as high as `height` says.
  pub fn with_height(mut self, height: CaretHeight) -> Self {
    self.height = height;
    self
  }

  /// How thick the line under what an input method composes is, in logical pixels.
  pub fn composition_line(&self) -> f32 {
    if self.composition_line.is_finite() {
      self.composition_line.max(0.0)
    } else {
      1.0
    }
  }

  pub fn set_composition_line(&mut self, width: f32) {
    self.composition_line = width;
  }

  /// The caret, with a line `width` thick under a composition.
  pub fn with_composition_line(mut self, width: f32) -> Self {
    self.composition_line = width;
    self
  }
}

impl Default for ThemeCaret {
  fn default() -> Self {
    Self {
      mode: CaretMode::Blinking,
      width: 1.5,
      height: CaretHeight::FontSize(18.0 / 14.0),
      composition_line: 1.5,
    }
  }
}
