#[derive(Clone, Copy, Debug, PartialEq, Eq, lurq_macros::DevtoolsInspectable)]
pub struct Color {
  r: u8,
  g: u8,
  b: u8,
  a: u8,
}

impl Color {
  pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
    Self { r, g, b, a }
  }

  pub fn r(&self) -> u8 {
    self.r
  }
  pub fn g(&self) -> u8 {
    self.g
  }
  pub fn b(&self) -> u8 {
    self.b
  }
  pub fn a(&self) -> u8 {
    self.a
  }

  pub fn from_rgb_str(value: &str) -> Self {
    let mut parts = value.split(',').map(str::trim);

    let r = parts
      .next()
      .and_then(|v| v.parse::<u8>().ok())
      .expect("invalid red channel");

    let g = parts
      .next()
      .and_then(|v| v.parse::<u8>().ok())
      .expect("invalid green channel");

    let b = parts
      .next()
      .and_then(|v| v.parse::<u8>().ok())
      .expect("invalid blue channel");

    assert!(parts.next().is_none(), "rgb string must have exactly 3 channels");

    Self { r, g, b, a: 255 }
  }

  pub fn from_rgba_str(value: &str) -> Self {
    let mut parts = value.split(',').map(str::trim);

    let r = parts
      .next()
      .and_then(|v| v.parse::<u8>().ok())
      .expect("invalid red channel");

    let g = parts
      .next()
      .and_then(|v| v.parse::<u8>().ok())
      .expect("invalid green channel");

    let b = parts
      .next()
      .and_then(|v| v.parse::<u8>().ok())
      .expect("invalid blue channel");

    let a = parts
      .next()
      .and_then(|v| v.parse::<u8>().ok())
      .expect("invalid alpha channel");

    assert!(parts.next().is_none(), "rgba string must have exactly 4 channels");

    Self { r, g, b, a }
  }

  pub fn to_f32_array(&self) -> [f32; 4] {
    [
      self.r as f32 / 255.0,
      self.g as f32 / 255.0,
      self.b as f32 / 255.0,
      self.a as f32 / 255.0,
    ]
  }

  pub fn to_linear_f32_array(&self) -> [f32; 4] {
    [
      srgb_to_linear(self.r),
      srgb_to_linear(self.g),
      srgb_to_linear(self.b),
      self.a as f32 / 255.0,
    ]
  }

  pub fn from_hex(hex: &str) -> Self {
    let hex = hex.strip_prefix('#').unwrap_or(hex);
    match hex.len() {
      3 => {
        let r = u8::from_str_radix(&hex[0..1], 16).unwrap();
        let g = u8::from_str_radix(&hex[1..2], 16).unwrap();
        let b = u8::from_str_radix(&hex[2..3], 16).unwrap();
        Self::new(r * 17, g * 17, b * 17, 255)
      }
      4 => {
        let r = u8::from_str_radix(&hex[0..1], 16).unwrap();
        let g = u8::from_str_radix(&hex[1..2], 16).unwrap();
        let b = u8::from_str_radix(&hex[2..3], 16).unwrap();
        let a = u8::from_str_radix(&hex[3..4], 16).unwrap();
        Self::new(r * 17, g * 17, b * 17, a * 17)
      }
      6 => {
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap();
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap();
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap();
        Self::new(r, g, b, 255)
      }
      8 => {
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap();
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap();
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap();
        let a = u8::from_str_radix(&hex[6..8], 16).unwrap();
        Self::new(r, g, b, a)
      }
      _ => panic!("invalid hex color: #{hex}"),
    }
  }

  pub fn to_hex(&self) -> String {
    if self.a == 255 {
      format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    } else {
      format!("#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, self.a)
    }
  }

  pub fn from_hsl(hsl: &str) -> Self {
    let inner = hsl
      .trim()
      .strip_prefix("hsla(")
      .or_else(|| hsl.trim().strip_prefix("hsl("))
      .and_then(|s| s.strip_suffix(')'))
      .expect("expected hsl(...) or hsla(...)");

    let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();

    let h: f64 = parts[0].parse().unwrap();
    let s: f64 = parts[1].strip_suffix('%').unwrap().parse::<f64>().unwrap() / 100.0;
    let l: f64 = parts[2].strip_suffix('%').unwrap().parse::<f64>().unwrap() / 100.0;
    let a: f64 = if parts.len() > 3 {
      parts[3].parse().unwrap()
    } else {
      1.0
    };

    let (r, g, b) = hsl_to_rgb(h, s, l);
    Self::new(r, g, b, (a * 255.0).round() as u8)
  }

  pub fn to_hsl(&self) -> String {
    let r = self.r as f64 / 255.0;
    let g = self.g as f64 / 255.0;
    let b = self.b as f64 / 255.0;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let l = (max + min) / 2.0;

    if delta < f64::EPSILON {
      return if self.a == 255 {
        format!("hsl(0, 0%, {}%)", (l * 100.0).round())
      } else {
        format!("hsla(0, 0%, {}%, {:.2})", (l * 100.0).round(), self.a as f64 / 255.0)
      };
    }

    let s = if l <= 0.5 {
      delta / (max + min)
    } else {
      delta / (2.0 - max - min)
    };

    let h = if (max - r).abs() < f64::EPSILON {
      60.0 * (((g - b) / delta) % 6.0)
    } else if (max - g).abs() < f64::EPSILON {
      60.0 * ((b - r) / delta + 2.0)
    } else {
      60.0 * ((r - g) / delta + 4.0)
    };
    let h = ((h % 360.0) + 360.0) % 360.0;

    if self.a == 255 {
      format!("hsl({}, {}%, {}%)", h.round(), (s * 100.0).round(), (l * 100.0).round())
    } else {
      format!(
        "hsla({}, {}%, {}%, {:.2})",
        h.round(),
        (s * 100.0).round(),
        (l * 100.0).round(),
        self.a as f64 / 255.0
      )
    }
  }

  pub fn with_alpha(mut self, alpha: u8) -> Self {
    self.a = alpha;
    self
  }

  pub fn with_opacity(self, opacity: f32) -> Self {
    self.with_alpha((opacity.clamp(0.0, 1.0) * 255.0).round() as u8)
  }
}

/// A color by its hue, its saturation and its value (brightness): what a color picker's area and hue rail move.
/// The hue is in degrees from 0 to 360, the rest from 0 to 1. A color of no saturation or no value has no hue of its
/// own: [`Color::to_hsv`] gives it 0, and a picker keeps the hue it had.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hsv {
  pub h: f32,
  pub s: f32,
  pub v: f32,
  pub a: f32,
}

/// A color by its hue, its saturation and its lightness, as CSS's `hsl()` has it: the hue in degrees from 0 to 360,
/// the rest from 0 to 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hsl {
  pub h: f32,
  pub s: f32,
  pub l: f32,
  pub a: f32,
}

impl Color {
  /// The color `text` writes, or `None` where it writes none: `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa` (the `#` may
  /// be left out), `rgb(..)` and `rgba(..)` of three numbers from 0 to 255 or percents and an alpha from 0 to 1 or a
  /// percent, `hsl(..)` and `hsla(..)` of a hue in degrees and two percents, with commas or spaces and an alpha after
  /// a `/` or a fourth comma. Case and the space around are not read. What a person types into a color field.
  pub fn parse(text: &str) -> Option<Self> {
    let text = text.trim();
    let lower = text.to_ascii_lowercase();
    if let Some(inner) = function_args(&lower, &["rgba", "rgb"]) {
      let (channels, alpha) = split_args(inner)?;
      let [r, g, b] = channels;
      return Some(Self::new(
        channel(r)?,
        channel(g)?,
        channel(b)?,
        alpha.map_or(Some(255), alpha_channel)?,
      ));
    }
    if let Some(inner) = function_args(&lower, &["hsla", "hsl"]) {
      let (parts, alpha) = split_args(inner)?;
      let [h, s, l] = parts;
      let h: f32 = h.strip_suffix("deg").unwrap_or(h).parse().ok()?;
      let s = percent(s)?;
      let l = percent(l)?;
      let a = alpha.map_or(Some(255), alpha_channel)? as f32 / 255.0;
      return Some(Self::from(Hsl { h, s, l, a }));
    }
    let hex = lower.strip_prefix('#').unwrap_or(&lower);
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
      return None;
    }
    let digit = |i: usize| u8::from_str_radix(&hex[i..=i], 16).ok().map(|d| d * 17);
    let pair = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    match hex.len() {
      3 => Some(Self::new(digit(0)?, digit(1)?, digit(2)?, 255)),
      4 => Some(Self::new(digit(0)?, digit(1)?, digit(2)?, digit(3)?)),
      6 => Some(Self::new(pair(0)?, pair(2)?, pair(4)?, 255)),
      8 => Some(Self::new(pair(0)?, pair(2)?, pair(4)?, pair(6)?)),
      _ => None,
    }
  }

  /// `#RRGGBB`, or `#RRGGBBAA` where the color is not opaque, in capitals: as a color field shows it.
  pub fn to_hex_upper(&self) -> String {
    self.to_hex().to_ascii_uppercase()
  }

  /// `rgb(37, 99, 235)`, or `rgba(37, 99, 235, 0.5)` where the color is not opaque.
  pub fn to_rgb_string(&self) -> String {
    if self.a == 255 {
      format!("rgb({}, {}, {})", self.r, self.g, self.b)
    } else {
      format!("rgba({}, {}, {}, {})", self.r, self.g, self.b, alpha_text(self.a))
    }
  }

  /// The color by its hue, saturation and value.
  pub fn to_hsv(&self) -> Hsv {
    let [r, g, b, a] = self.to_f32_array();
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    Hsv {
      h: hue_of(r, g, b, max, delta),
      s: if max > 0.0 { delta / max } else { 0.0 },
      v: max,
      a,
    }
  }

  /// The color by its hue, saturation and lightness.
  pub fn to_hsl_values(&self) -> Hsl {
    let [r, g, b, a] = self.to_f32_array();
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let l = (max + min) / 2.0;
    let s = if delta <= f32::EPSILON {
      0.0
    } else {
      delta / (1.0 - (2.0 * l - 1.0).abs())
    };
    Hsl {
      h: hue_of(r, g, b, max, delta),
      s,
      l,
      a,
    }
  }
}

impl From<Hsv> for Color {
  fn from(hsv: Hsv) -> Self {
    let h = hsv.h.rem_euclid(360.0);
    let (s, v) = (hsv.s.clamp(0.0, 1.0), hsv.v.clamp(0.0, 1.0));
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = sector(h, c, x);
    Self::new(unit(r + m), unit(g + m), unit(b + m), unit(hsv.a))
  }
}

impl From<Hsl> for Color {
  fn from(hsl: Hsl) -> Self {
    let h = hsl.h.rem_euclid(360.0);
    let (s, l) = (hsl.s.clamp(0.0, 1.0), hsl.l.clamp(0.0, 1.0));
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r, g, b) = sector(h, c, x);
    Self::new(unit(r + m), unit(g + m), unit(b + m), unit(hsl.a))
  }
}

impl From<Color> for Hsv {
  fn from(color: Color) -> Self {
    color.to_hsv()
  }
}

impl From<Color> for Hsl {
  fn from(color: Color) -> Self {
    color.to_hsl_values()
  }
}

/// The hue of `r`, `g` and `b` (from 0 to 1) in degrees, of which `max` is the greatest and `delta` the greatest less
/// the least: 0 for a grey.
fn hue_of(r: f32, g: f32, b: f32, max: f32, delta: f32) -> f32 {
  if delta <= f32::EPSILON {
    return 0.0;
  }
  let h = if max == r {
    60.0 * ((g - b) / delta).rem_euclid(6.0)
  } else if max == g {
    60.0 * ((b - r) / delta + 2.0)
  } else {
    60.0 * ((r - g) / delta + 4.0)
  };
  h.rem_euclid(360.0)
}

/// The red, green and blue of a hue `h` before the lightness is added, of chroma `c` and second part `x`.
fn sector(h: f32, c: f32, x: f32) -> (f32, f32, f32) {
  match h {
    h if h < 60.0 => (c, x, 0.0),
    h if h < 120.0 => (x, c, 0.0),
    h if h < 180.0 => (0.0, c, x),
    h if h < 240.0 => (0.0, x, c),
    h if h < 300.0 => (x, 0.0, c),
    _ => (c, 0.0, x),
  }
}

/// A part from 0 to 1 as a channel from 0 to 255.
fn unit(value: f32) -> u8 {
  (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// What is between the parentheses of `text` where it is a call of one of `names`.
fn function_args<'a>(text: &'a str, names: &[&str]) -> Option<&'a str> {
  names.iter().find_map(|name| {
    text
      .strip_prefix(name)?
      .trim_start()
      .strip_prefix('(')?
      .strip_suffix(')')
  })
}

/// Three arguments and an alpha, if there is one: separated by commas, or by spaces with the alpha after a `/`.
fn split_args(inner: &str) -> Option<([&str; 3], Option<&str>)> {
  let (main, slash_alpha) = match inner.split_once('/') {
    Some((main, alpha)) => (main, Some(alpha.trim())),
    None => (inner, None),
  };
  let parts: Vec<&str> = if main.contains(',') {
    main.split(',').map(str::trim).filter(|part| !part.is_empty()).collect()
  } else {
    main.split_whitespace().collect()
  };
  match (parts.as_slice(), slash_alpha) {
    ([a, b, c], alpha) => Some(([a, b, c], alpha)),
    ([a, b, c, alpha], None) => Some(([a, b, c], Some(*alpha))),
    _ => None,
  }
}

/// A channel written as a number from 0 to 255 or a percent.
fn channel(text: &str) -> Option<u8> {
  match text.strip_suffix('%') {
    Some(percent) => Some(unit(percent.trim().parse::<f32>().ok()? / 100.0)),
    None => {
      let value: f32 = text.parse().ok()?;
      (0.0..=255.0).contains(&value).then(|| value.round() as u8)
    }
  }
}

/// An alpha written as a number from 0 to 1 or a percent, as a channel.
fn alpha_channel(text: &str) -> Option<u8> {
  let value = match text.strip_suffix('%') {
    Some(percent) => percent.trim().parse::<f32>().ok()? / 100.0,
    None => text.parse::<f32>().ok()?,
  };
  (0.0..=1.0).contains(&value).then(|| unit(value))
}

/// A part written as a percent, from 0 to 1.
fn percent(text: &str) -> Option<f32> {
  let value: f32 = text.strip_suffix('%').unwrap_or(text).trim().parse().ok()?;
  (0.0..=100.0).contains(&value).then_some(value / 100.0)
}

/// An alpha channel written from 0 to 1, with no more figures than it needs: `0.5`, `0.25`, `1`.
fn alpha_text(alpha: u8) -> String {
  let text = format!("{:.2}", alpha as f32 / 255.0);
  text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

impl From<&str> for Color {
  fn from(s: &str) -> Self {
    if s.trim().starts_with("hsl") {
      Self::from_hsl(s)
    } else if s.trim().starts_with('#') {
      Self::from_hex(s)
    } else if s.trim().starts_with("rgba") {
      Self::from_rgba_str(s)
    } else if s.trim().starts_with("rgb") {
      Self::from_rgb_str(s)
    } else {
      panic!("unsupported color format: {s}");
    }
  }
}

fn srgb_to_linear(channel: u8) -> f32 {
  let c = channel as f32 / 255.0;
  if c <= 0.04045 {
    c / 12.92
  } else {
    ((c + 0.055) / 1.055).powf(2.4)
  }
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (u8, u8, u8) {
  let h = ((h % 360.0) + 360.0) % 360.0;
  let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
  let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
  let m = l - c / 2.0;

  let (r1, g1, b1) = if h < 60.0 {
    (c, x, 0.0)
  } else if h < 120.0 {
    (x, c, 0.0)
  } else if h < 180.0 {
    (0.0, c, x)
  } else if h < 240.0 {
    (0.0, x, c)
  } else if h < 300.0 {
    (x, 0.0, c)
  } else {
    (c, 0.0, x)
  };

  (
    ((r1 + m) * 255.0).round() as u8,
    ((g1 + m) * 255.0).round() as u8,
    ((b1 + m) * 255.0).round() as u8,
  )
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn hex_roundtrip() {
    let c = Color::from_hex("#ff8040");
    assert_eq!(c.to_hex(), "#ff8040");
  }

  #[test]
  fn hex_short() {
    let c = Color::from_hex("#fab");
    assert_eq!(c.r, 0xff);
    assert_eq!(c.g, 0xaa);
    assert_eq!(c.b, 0xbb);
    assert_eq!(c.a, 255);
  }

  #[test]
  fn hex_with_alpha() {
    let c = Color::from_hex("#ff804080");
    assert_eq!(c.to_hex(), "#ff804080");
  }

  #[test]
  fn hsl_pure_red() {
    let c = Color::from_hsl("hsl(0, 100%, 50%)");
    assert_eq!(c.r, 255);
    assert_eq!(c.g, 0);
    assert_eq!(c.b, 0);
  }

  #[test]
  fn hsl_roundtrip() {
    let c = Color::new(255, 0, 0, 255);
    let hsl = c.to_hsl();
    let c2 = Color::from_hsl(&hsl);
    assert_eq!(c2.r, c.r);
    assert_eq!(c2.g, c.g);
    assert_eq!(c2.b, c.b);
  }

  #[test]
  fn hsla_alpha() {
    let c = Color::from_hsl("hsla(120, 50%, 50%, 0.5)");
    assert_eq!(c.a, 128);
    assert!(c.to_hsl().starts_with("hsla("));
  }

  #[test]
  fn parse_reads_hex_rgb_and_hsl_and_refuses_what_is_no_color() {
    assert_eq!(Color::parse("#2563EB"), Some(Color::new(0x25, 0x63, 0xeb, 255)));
    assert_eq!(Color::parse(" 2563eb "), Some(Color::new(0x25, 0x63, 0xeb, 255)));
    assert_eq!(Color::parse("#2563EB80"), Some(Color::new(0x25, 0x63, 0xeb, 0x80)));
    assert_eq!(Color::parse("#fab"), Some(Color::new(0xff, 0xaa, 0xbb, 255)));
    assert_eq!(Color::parse("#fab8"), Some(Color::new(0xff, 0xaa, 0xbb, 0x88)));
    assert_eq!(Color::parse("rgb(37, 99, 235)"), Some(Color::new(37, 99, 235, 255)));
    assert_eq!(Color::parse("RGB(37 99 235 / 50%)"), Some(Color::new(37, 99, 235, 128)));
    assert_eq!(
      Color::parse("rgba(37, 99, 235, 0.5)"),
      Some(Color::new(37, 99, 235, 128))
    );
    assert_eq!(Color::parse("rgb(100%, 0%, 0%)"), Some(Color::new(255, 0, 0, 255)));
    assert_eq!(Color::parse("hsl(0, 100%, 50%)"), Some(Color::new(255, 0, 0, 255)));
    assert_eq!(
      Color::parse("hsl(120deg 100% 25% / 0.5)"),
      Some(Color::new(0, 128, 0, 128))
    );
    for wrong in [
      "",
      "#",
      "#12",
      "#12345",
      "#gggggg",
      "blue",
      "rgb(1, 2)",
      "rgb(300, 0, 0)",
      "hsl(0, 120%, 50%)",
      "rgba(1, 2, 3, 2)",
    ] {
      assert_eq!(Color::parse(wrong), None, "{wrong:?} is no color");
    }
  }

  #[test]
  fn hex_and_rgb_are_written_with_alpha_where_it_is_not_opaque() {
    let blue = Color::new(0x25, 0x63, 0xeb, 255);
    assert_eq!(blue.to_hex_upper(), "#2563EB");
    assert_eq!(blue.with_alpha(0x80).to_hex_upper(), "#2563EB80");
    assert_eq!(blue.to_rgb_string(), "rgb(37, 99, 235)");
    assert_eq!(blue.with_alpha(128).to_rgb_string(), "rgba(37, 99, 235, 0.5)");
    assert_eq!(blue.with_alpha(64).to_rgb_string(), "rgba(37, 99, 235, 0.25)");
  }

  #[test]
  fn hsv_goes_there_and_back() {
    let blue = Color::new(0x25, 0x63, 0xeb, 255);
    let hsv = blue.to_hsv();
    assert!((hsv.h - 221.2).abs() < 0.1, "{hsv:?}");
    assert!(
      (hsv.s - 0.843).abs() < 0.001 && (hsv.v - 0.922).abs() < 0.001,
      "{hsv:?}"
    );
    assert_eq!(Color::from(hsv), blue);
    for color in [
      Color::new(255, 0, 0, 255),
      Color::new(0, 0, 0, 255),
      Color::new(255, 255, 255, 0),
      Color::new(12, 200, 99, 77),
      Color::new(128, 128, 128, 255),
    ] {
      assert_eq!(Color::from(color.to_hsv()), color);
      assert_eq!(Color::from(color.to_hsl_values()), color);
    }
    // A grey has no hue: 0, and full red, green and blue at its value.
    let grey = Color::new(128, 128, 128, 255).to_hsv();
    assert_eq!((grey.h, grey.s), (0.0, 0.0));
    assert_eq!(
      Color::from(Hsv {
        h: 120.0,
        s: 1.0,
        v: 1.0,
        a: 1.0
      }),
      Color::new(0, 255, 0, 255)
    );
    assert_eq!(
      Color::from(Hsv {
        h: 360.0,
        s: 1.0,
        v: 1.0,
        a: 1.0
      }),
      Color::new(255, 0, 0, 255)
    );
  }

  #[test]
  fn hsl_values_are_css_hsl() {
    let hsl = Color::new(0, 128, 0, 255).to_hsl_values();
    assert!(
      (hsl.h - 120.0).abs() < 0.01 && (hsl.s - 1.0).abs() < 0.01 && (hsl.l - 0.251).abs() < 0.001,
      "{hsl:?}"
    );
  }

  #[test]
  fn achromatic() {
    let c = Color::new(128, 128, 128, 255);
    let hsl = c.to_hsl();
    assert!(hsl.contains("0%"), "saturation should be 0 for gray: {hsl}");
  }
}
