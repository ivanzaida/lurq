#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
  fn achromatic() {
    let c = Color::new(128, 128, 128, 255);
    let hsl = c.to_hsl();
    assert!(hsl.contains("0%"), "saturation should be 0 for gray: {hsl}");
  }
}
