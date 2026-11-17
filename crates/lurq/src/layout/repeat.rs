//! A pattern: what an element holds painted again and again across its box (`.repeat(Repeat)`).

/// How what an element holds is repeated across its box: a watermark, a pattern.
///
/// What the element holds is laid out once, where it is in the element; it is the first tile. Its tiles are the
/// first child's box with `gap` after it, across and down, both ways from the first, so that they cover the
/// element's box; every other row is moved `stagger` along. Only the tiles that can be seen are painted, under the
/// element's clip and the scroll it is in: a pattern over a page of any length costs the tiles in view, and
/// scrolling builds nothing. The copies are paint alone: no pointer hits them and tooling does not read them; what
/// the element holds is in the tree once.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Repeat {
  /// The room between one tile's box and the next, across.
  pub gap_x: f32,
  /// The room between one row of tiles and the next.
  pub gap_y: f32,
  /// How much further along every other row is.
  pub stagger: f32,
}

impl Repeat {
  /// Tiles that touch: the first child's box and no room after it.
  pub fn new() -> Self {
    Self::default()
  }

  /// The room between one tile and the next, across and down.
  pub fn gap(mut self, x: f32, y: f32) -> Self {
    self.gap_x = x.max(0.0);
    self.gap_y = y.max(0.0);
    self
  }

  /// How much further along every other row is: a brick's pattern, a watermark whose rows do not line up.
  pub fn stagger(mut self, stagger: f32) -> Self {
    self.stagger = stagger;
    self
  }
}
