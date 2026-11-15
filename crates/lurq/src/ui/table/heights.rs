//! How high the lines of a mounted table are: each is taken to be as high as a row of its recipe until it is built
//! and measured. Where every line is that high, it holds nothing per line; once one measures otherwise, it keeps
//! the differences in a tree of sums, so that where a line is and which line is at a place are reckoned in steps of
//! the order of the logarithm of how many there are, for a million lines as for ten.

use std::collections::HashMap;

/// The heights of a table's lines: the estimate, and what the lines built measured.
#[derive(Clone, Debug, Default)]
pub(super) struct Heights {
  estimate: f32,
  count: usize,
  /// What each line measured, where it was built.
  measured: HashMap<usize, f32>,
  /// The sums of the differences from the estimate: a Fenwick tree, made once a line measures otherwise.
  sums: Option<Vec<f64>>,
}

impl Heights {
  pub fn new(estimate: f32, count: usize) -> Self {
    Self {
      estimate,
      count,
      ..Self::default()
    }
  }

  /// Whether it was made for lines of `estimate` and `count`.
  pub fn is_for(&self, estimate: f32, count: usize) -> bool {
    self.estimate == estimate && self.count == count
  }

  pub fn estimate(&self) -> f32 {
    self.estimate
  }

  pub fn count(&self) -> usize {
    self.count
  }

  /// How high the line at `position` is: what it measured, or the estimate.
  pub fn height(&self, position: usize) -> f32 {
    self.measured.get(&position).copied().unwrap_or(self.estimate)
  }

  /// Where the line at `position` starts, from the top of the first: for `count`, the height of them all.
  pub fn top(&self, position: usize) -> f32 {
    let position = position.min(self.count);
    (position as f64 * self.estimate as f64 + self.differences_before(position)) as f32
  }

  /// The height of every line.
  pub fn total(&self) -> f32 {
    self.top(self.count)
  }

  /// The line at `y` from the top of the first: the last whose top is at or above it.
  pub fn position_at(&self, y: f32) -> usize {
    if self.count == 0 || y <= 0.0 {
      return 0;
    }
    if self.sums.is_none() && self.estimate > 0.0 {
      return ((y / self.estimate).floor() as usize).min(self.count - 1);
    }
    // The tops grow with the position: the last line whose top is not past `y`.
    let (mut low, mut high) = (0, self.count - 1);
    while low < high {
      let middle = low + (high - low).div_ceil(2);
      if self.top(middle) <= y {
        low = middle;
      } else {
        high = middle - 1;
      }
    }
    low
  }

  /// Keeps what the line at `position` measured: how much higher it is than it was taken to be, 0 where it is as
  /// high.
  pub fn measure(&mut self, position: usize, height: f32) -> f32 {
    if position >= self.count {
      return 0.0;
    }
    let before = self.height(position);
    if (height - before).abs() < 0.5 {
      return 0.0;
    }
    self.measured.insert(position, height);
    let count = self.count;
    let sums = self.sums.get_or_insert_with(|| vec![0.0; count + 1]);
    let change = (height - before) as f64;
    let mut at = position + 1;
    while at <= count {
      sums[at] += change;
      at += at & at.wrapping_neg();
    }
    height - before
  }

  /// The sum of the differences from the estimate of the lines before `position`.
  fn differences_before(&self, position: usize) -> f64 {
    let Some(sums) = &self.sums else {
      return 0.0;
    };
    let (mut at, mut sum) = (position, 0.0);
    while at > 0 {
      sum += sums[at];
      at &= at - 1;
    }
    sum
  }
}

#[cfg(test)]
mod tests {
  use super::Heights;

  #[test]
  fn lines_of_one_height_hold_nothing_and_are_reckoned_by_it() {
    let heights = Heights::new(44.0, 1_000_000);
    assert_eq!(heights.top(10), 440.0);
    assert_eq!(heights.total(), 44_000_000.0);
    assert_eq!(heights.position_at(441.0), 10);
    assert_eq!(heights.position_at(1e12), 999_999);
    assert!(heights.sums.is_none());
  }

  #[test]
  fn a_line_measured_higher_moves_every_line_after_it() {
    let mut heights = Heights::new(44.0, 100);
    assert_eq!(heights.measure(3, 88.0), 44.0);
    assert_eq!(
      heights.measure(3, 88.2),
      0.0,
      "a measure as high as it was changes nothing"
    );
    assert_eq!(heights.top(3), 132.0);
    assert_eq!(heights.top(4), 220.0);
    assert_eq!(heights.top(100), 4444.0);
    assert_eq!(heights.position_at(150.0), 3, "within the high line");
    assert_eq!(heights.position_at(219.0), 3);
    assert_eq!(heights.position_at(220.0), 4);
    heights.measure(3, 44.0);
    assert_eq!(heights.top(4), 176.0);
  }
}
