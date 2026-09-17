//! The blend functions of W3C compositing and blending level 1, which are the
//! eighteen modes Pencil and Figma expose, plus the reference compositor the
//! software backend uses. The native backends run the same formulas in their
//! canvas shaders, so a pixel test can compare them.

/// Pencil's eighteen modes, in Pencil's own order. `Normal` is source-over.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BlendMode {
  #[default]
  Normal,
  Darken,
  Multiply,
  LinearBurn,
  ColorBurn,
  Lighten,
  Screen,
  LinearDodge,
  ColorDodge,
  Overlay,
  SoftLight,
  HardLight,
  Difference,
  Exclusion,
  Hue,
  Saturation,
  Color,
  Luminosity,
}

impl BlendMode {
  pub const ALL: [Self; 18] = [
    Self::Normal,
    Self::Darken,
    Self::Multiply,
    Self::LinearBurn,
    Self::ColorBurn,
    Self::Lighten,
    Self::Screen,
    Self::LinearDodge,
    Self::ColorDodge,
    Self::Overlay,
    Self::SoftLight,
    Self::HardLight,
    Self::Difference,
    Self::Exclusion,
    Self::Hue,
    Self::Saturation,
    Self::Color,
    Self::Luminosity,
  ];
  /// The value the canvas shaders switch on. Stable: it is written into a
  /// uniform, and the two native backends and this table must agree.
  pub fn index(self) -> u32 {
    Self::ALL.iter().position(|m| *m == self).unwrap_or(0) as u32
  }
  pub fn is_normal(self) -> bool {
    self == Self::Normal
  }
}

fn separable(mode: BlendMode, cb: f32, cs: f32) -> f32 {
  match mode {
    BlendMode::Normal => cs,
    BlendMode::Darken => cb.min(cs),
    BlendMode::Multiply => cb * cs,
    BlendMode::LinearBurn => (cb + cs - 1.0).max(0.0),
    BlendMode::ColorBurn => {
      if cb >= 1.0 {
        1.0
      } else if cs <= 0.0 {
        0.0
      } else {
        1.0 - ((1.0 - cb) / cs).min(1.0)
      }
    }
    BlendMode::Lighten => cb.max(cs),
    BlendMode::Screen => cb + cs - cb * cs,
    BlendMode::LinearDodge => (cb + cs).min(1.0),
    BlendMode::ColorDodge => {
      if cb <= 0.0 {
        0.0
      } else if cs >= 1.0 {
        1.0
      } else {
        (cb / (1.0 - cs)).min(1.0)
      }
    }
    BlendMode::Overlay => separable(BlendMode::HardLight, cs, cb),
    BlendMode::SoftLight => {
      let d = if cb <= 0.25 {
        ((16.0 * cb - 12.0) * cb + 4.0) * cb
      } else {
        cb.max(0.0).sqrt()
      };
      if cs <= 0.5 {
        cb - (1.0 - 2.0 * cs) * cb * (1.0 - cb)
      } else {
        cb + (2.0 * cs - 1.0) * (d - cb)
      }
    }
    BlendMode::HardLight => {
      if cs <= 0.5 {
        cb * (2.0 * cs)
      } else {
        let s = 2.0 * cs - 1.0;
        cb + s - cb * s
      }
    }
    BlendMode::Difference => (cb - cs).abs(),
    BlendMode::Exclusion => cb + cs - 2.0 * cb * cs,
    _ => cs,
  }
}

fn luminosity(c: [f32; 3]) -> f32 {
  0.3 * c[0] + 0.59 * c[1] + 0.11 * c[2]
}
fn clip_color(mut c: [f32; 3]) -> [f32; 3] {
  let l = luminosity(c);
  let n = c[0].min(c[1]).min(c[2]);
  let x = c[0].max(c[1]).max(c[2]);
  if n < 0.0 {
    for v in &mut c {
      *v = l + (*v - l) * l / (l - n).max(f32::MIN_POSITIVE);
    }
  }
  if x > 1.0 {
    for v in &mut c {
      *v = l + (*v - l) * (1.0 - l) / (x - l).max(f32::MIN_POSITIVE);
    }
  }
  c
}
fn set_luminosity(c: [f32; 3], l: f32) -> [f32; 3] {
  let d = l - luminosity(c);
  clip_color([c[0] + d, c[1] + d, c[2] + d])
}
fn saturation(c: [f32; 3]) -> f32 {
  c[0].max(c[1]).max(c[2]) - c[0].min(c[1]).min(c[2])
}
fn set_saturation(c: [f32; 3], s: f32) -> [f32; 3] {
  let (min, max) = (c[0].min(c[1]).min(c[2]), c[0].max(c[1]).max(c[2]));
  let mut result = [0.0; 3];
  if max > min {
    for i in 0..3 {
      result[i] = (c[i] - min) / (max - min) * s;
    }
  }
  result
}

/// `B(Cb, Cs)` on straight, non-premultiplied colours, as the specification
/// defines it. Non-separable modes take the whole triple.
pub(crate) fn blend(mode: BlendMode, cb: [f32; 3], cs: [f32; 3]) -> [f32; 3] {
  match mode {
    BlendMode::Hue => set_luminosity(set_saturation(cs, saturation(cb)), luminosity(cb)),
    BlendMode::Saturation => set_luminosity(set_saturation(cb, saturation(cs)), luminosity(cb)),
    BlendMode::Color => set_luminosity(cs, luminosity(cb)),
    BlendMode::Luminosity => set_luminosity(cb, luminosity(cs)),
    _ => [0, 1, 2].map(|i| separable(mode, cb[i], cs[i])),
  }
}

/// One premultiplied source over one premultiplied backdrop, blended and then
/// composited source-over, which is the whole of what a layer or a blended draw
/// does to a pixel. Inputs and the result are premultiplied 0..=1.
pub(crate) fn composite_pixel(mode: BlendMode, backdrop: [f32; 4], source: [f32; 4]) -> [f32; 4] {
  let (ab, ags) = (backdrop[3], source[3]);
  if ags <= 0.0 {
    return backdrop;
  }
  let straight = |c: [f32; 4]| {
    if c[3] <= 0.0 {
      [0.0; 3]
    } else {
      [c[0] / c[3], c[1] / c[3], c[2] / c[3]]
    }
  };
  let (cb, cs) = (straight(backdrop), straight(source));
  let blended = blend(mode, cb, cs);
  // Cr = (1 - ab) * Cs + ab * B(Cb, Cs); the source-over that follows is written
  // premultiplied, so the backdrop keeps its own premultiplied channels.
  let mut out = [0.0; 4];
  for i in 0..3 {
    let cr = (1.0 - ab) * cs[i] + ab * blended[i];
    out[i] = ags * cr + backdrop[i] * (1.0 - ags);
  }
  out[3] = ags + ab * (1.0 - ags);
  out
}

/// Composites `source` over `destination` in place. Both are premultiplied sRGB
/// RGBA8 of the same length; `alpha` scales the whole source, which is how an
/// isolated layer's opacity applies once rather than per draw.
pub(crate) fn composite_premultiplied(destination: &mut [u8], source: &[u8], alpha: f32, mode: BlendMode) {
  let alpha = alpha.clamp(0.0, 1.0);
  if alpha <= 0.0 {
    return;
  }
  for (d, s) in destination.chunks_exact_mut(4).zip(source.chunks_exact(4)) {
    let source = [0, 1, 2, 3].map(|i| f32::from(s[i]) / 255.0 * alpha);
    if source[3] <= 0.0 && mode.is_normal() {
      continue;
    }
    let backdrop = [0, 1, 2, 3].map(|i| f32::from(d[i]) / 255.0);
    let out = composite_pixel(mode, backdrop, source);
    for i in 0..4 {
      d[i] = (out[i].clamp(0.0, 1.0) * 255.0).round() as u8;
    }
  }
}
