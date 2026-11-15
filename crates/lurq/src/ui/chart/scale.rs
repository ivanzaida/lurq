//! The scales of a chart: values onto a length, with ticks on round numbers, and categories onto bands.

/// Values from a domain onto a range of pixels, along a straight line. Built with [`LinearScale::nice`], its domain
/// starts and ends on ticks that are round numbers (1, 2 or 5 times a power of ten), and holds zero where the values
/// are all on one side of it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinearScale {
  /// The least and the most value it maps.
  pub domain: (f64, f64),
  /// Where they are drawn: the least value's place and the most's, which may run either way (a value axis runs up,
  /// from the plot's bottom).
  pub range: (f32, f32),
  /// The room between two ticks.
  pub step: f64,
}

impl LinearScale {
  /// A scale of `values` onto `range`, with about `count` ticks: its domain from the round number at or under the
  /// least of them (zero where none is under zero) to the round number at or over the most (zero where none is over
  /// it). No values, or values all the same, make a domain of a step around them.
  pub fn nice(values: impl IntoIterator<Item = f64>, range: (f32, f32), count: usize) -> Self {
    let (mut least, mut most) = values
      .into_iter()
      .filter(|value| value.is_finite())
      .fold((f64::INFINITY, f64::NEG_INFINITY), |(least, most), value| {
        (least.min(value), most.max(value))
      });
    if !least.is_finite() {
      least = 0.0;
      most = 1.0;
    }
    least = least.min(0.0);
    most = most.max(0.0);
    if most - least < f64::EPSILON {
      most = least + 1.0;
    }
    let step = nice_step((most - least) / count.max(1) as f64);
    Self {
      domain: ((least / step).floor() * step, (most / step).ceil() * step),
      range,
      step,
    }
  }

  /// The place `value` is drawn at.
  pub fn map(&self, value: f64) -> f32 {
    let (least, most) = self.domain;
    let t = if most > least {
      (value - least) / (most - least)
    } else {
      0.0
    };
    self.range.0 + (self.range.1 - self.range.0) * t as f32
  }

  /// The value drawn at `place`.
  pub fn invert(&self, place: f32) -> f64 {
    let (start, end) = self.range;
    let t = if (end - start).abs() > f32::EPSILON {
      ((place - start) / (end - start)) as f64
    } else {
      0.0
    };
    self.domain.0 + (self.domain.1 - self.domain.0) * t
  }

  /// The ticks from the least of the domain to the most, a step apart: round numbers.
  pub fn ticks(&self) -> Vec<f64> {
    let (least, most) = self.domain;
    if self.step <= 0.0 {
      return vec![least];
    }
    let count = ((most - least) / self.step).round() as usize;
    (0..=count)
      .map(|place| {
        let tick = least + self.step * place as f64;
        // What the steps add up to is written as the round number it is.
        (tick / self.step).round() * self.step
      })
      .collect()
  }
}

/// The round number, 1, 2 or 5 times a power of ten, nearest over `rough`.
fn nice_step(rough: f64) -> f64 {
  if !rough.is_finite() || rough <= 0.0 {
    return 1.0;
  }
  let power = 10f64.powf(rough.log10().floor());
  let fraction = rough / power;
  let nice = if fraction <= 1.0 {
    1.0
  } else if fraction <= 2.0 {
    2.0
  } else if fraction <= 5.0 {
    5.0
  } else {
    10.0
  };
  nice * power
}

/// Categories onto bands of equal width along a length, `gap` between two bands and none at the ends: a bar chart's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BandScale {
  pub count: usize,
  pub length: f32,
  pub gap: f32,
}

impl BandScale {
  pub fn new(count: usize, length: f32, gap: f32) -> Self {
    Self { count, length, gap }
  }

  /// How wide a band is.
  pub fn width(&self) -> f32 {
    if self.count == 0 {
      return 0.0;
    }
    ((self.length - self.gap * (self.count - 1) as f32) / self.count as f32).max(0.0)
  }

  /// Where the band of the category at `index` starts.
  pub fn start(&self, index: usize) -> f32 {
    index as f32 * (self.width() + self.gap)
  }

  /// The category whose band, or whose half of the gaps around it, is at `place`.
  pub fn index_at(&self, place: f32) -> Option<usize> {
    if self.count == 0 || place < 0.0 || place > self.length {
      return None;
    }
    let pitch = self.width() + self.gap;
    let index = ((place + self.gap / 2.0) / pitch).floor() as usize;
    Some(index.min(self.count - 1))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_scale_starts_at_zero_and_ends_on_a_round_number() {
    let scale = LinearScale::nice([0.55, 7.2, 12.3], (160.0, 0.0), 4);
    assert_eq!(scale.domain, (0.0, 15.0));
    assert_eq!(scale.ticks(), [0.0, 5.0, 10.0, 15.0]);
    assert_eq!(scale.map(0.0), 160.0);
    assert_eq!(scale.map(15.0), 0.0);
  }

  #[test]
  fn negative_values_take_the_domain_under_zero() {
    let scale = LinearScale::nice([-23.0, 41.0], (100.0, 0.0), 5);
    assert_eq!(scale.domain, (-40.0, 60.0));
    assert!(scale.ticks().contains(&0.0));
  }

  #[test]
  fn values_of_very_different_size_keep_round_ticks() {
    let scale = LinearScale::nice([0.002, 98_000.0], (0.0, 100.0), 4);
    assert_eq!(scale.ticks(), [0.0, 50_000.0, 100_000.0]);
    let small = LinearScale::nice([0.012, 0.031], (0.0, 100.0), 4);
    assert_eq!(small.step, 0.01);
    assert_eq!(small.ticks().last(), Some(&0.04));
  }

  #[test]
  fn bands_share_the_length_with_their_gaps() {
    let bands = BandScale::new(6, 372.0, 12.0);
    assert_eq!(bands.width(), 52.0);
    assert_eq!(bands.start(5), 320.0);
    assert_eq!(bands.index_at(330.0), Some(5));
    assert_eq!(
      bands.index_at(57.0),
      Some(0),
      "the half of a gap after a band is its own"
    );
    assert_eq!(bands.index_at(59.0), Some(1), "the other half the next band's");
  }
}
