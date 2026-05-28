use std::f32::consts::PI;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform2D {
  pub a: f32,
  pub b: f32,
  pub c: f32,
  pub d: f32,
  pub tx: f32,
  pub ty: f32,
}

impl Transform2D {
  pub const IDENTITY: Self = Self {
    a: 1.0,
    b: 0.0,
    c: 0.0,
    d: 1.0,
    tx: 0.0,
    ty: 0.0,
  };

  pub fn translate(tx: f32, ty: f32) -> Self {
    Self {
      tx,
      ty,
      ..Self::IDENTITY
    }
  }

  pub fn scale(sx: f32, sy: f32) -> Self {
    Self {
      a: sx,
      d: sy,
      ..Self::IDENTITY
    }
  }

  pub fn scale_uniform(s: f32) -> Self {
    Self::scale(s, s)
  }

  pub fn rotate(radians: f32) -> Self {
    let (sin, cos) = radians.sin_cos();
    Self {
      a: cos,
      b: sin,
      c: -sin,
      d: cos,
      tx: 0.0,
      ty: 0.0,
    }
  }

  pub fn rotate_deg(degrees: f32) -> Self {
    Self::rotate(degrees * PI / 180.0)
  }

  pub fn skew(ax: f32, ay: f32) -> Self {
    Self {
      a: 1.0,
      b: ay.tan(),
      c: ax.tan(),
      d: 1.0,
      tx: 0.0,
      ty: 0.0,
    }
  }

  pub fn then(&self, other: &Self) -> Self {
    Self {
      a: self.a * other.a + self.c * other.b,
      b: self.b * other.a + self.d * other.b,
      c: self.a * other.c + self.c * other.d,
      d: self.b * other.c + self.d * other.d,
      tx: self.a * other.tx + self.c * other.ty + self.tx,
      ty: self.b * other.tx + self.d * other.ty + self.ty,
    }
  }

  pub fn matrix_2x2(&self) -> [f32; 4] {
    [self.a, self.b, self.c, self.d]
  }

  pub fn is_identity(&self) -> bool {
    *self == Self::IDENTITY
  }
}

impl Default for Transform2D {
  fn default() -> Self {
    Self::IDENTITY
  }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Decomposed {
  pub translate_x: f32,
  pub translate_y: f32,
  pub scale_x: f32,
  pub scale_y: f32,
  pub rotate: f32,
  pub skew_x: f32,
}

impl Decomposed {
  pub const IDENTITY: Self = Self {
    translate_x: 0.0,
    translate_y: 0.0,
    scale_x: 1.0,
    scale_y: 1.0,
    rotate: 0.0,
    skew_x: 0.0,
  };

  pub fn with_rotate(mut self, radians: f32) -> Self {
    self.rotate = radians;
    self
  }

  pub fn with_rotate_deg(mut self, degrees: f32) -> Self {
    self.rotate = degrees * PI / 180.0;
    self
  }

  pub fn with_scale(mut self, sx: f32, sy: f32) -> Self {
    self.scale_x = sx;
    self.scale_y = sy;
    self
  }

  pub fn with_translate(mut self, tx: f32, ty: f32) -> Self {
    self.translate_x = tx;
    self.translate_y = ty;
    self
  }

  pub fn with_skew(mut self, radians: f32) -> Self {
    self.skew_x = radians;
    self
  }

  pub fn to_matrix(&self) -> Transform2D {
    recompose(self)
  }

  pub fn lerp(&self, to: &Decomposed, t: f32) -> Decomposed {
    let l = |a: f32, b: f32| a + (b - a) * t;
    Decomposed {
      translate_x: l(self.translate_x, to.translate_x),
      translate_y: l(self.translate_y, to.translate_y),
      scale_x: l(self.scale_x, to.scale_x),
      scale_y: l(self.scale_y, to.scale_y),
      rotate: l(self.rotate, to.rotate),
      skew_x: l(self.skew_x, to.skew_x),
    }
  }
}

impl Default for Decomposed {
  fn default() -> Self {
    Self::IDENTITY
  }
}

pub fn decompose(t: &Transform2D) -> Option<Decomposed> {
  let mut a = t.a;
  let mut b = t.b;
  let mut c = t.c;
  let mut d = t.d;

  let mut scale_x = (a * a + b * b).sqrt();
  if scale_x < 1e-12 {
    return None;
  }
  a /= scale_x;
  b /= scale_x;

  let skew = c * a + d * b;
  c -= skew * a;
  d -= skew * b;

  let scale_y = (c * c + d * d).sqrt();
  if scale_y < 1e-12 {
    return None;
  }
  c /= scale_y;
  d /= scale_y;

  let skew_x = (skew / scale_y).atan();

  let det = a * d - b * c;
  if det < 0.0 {
    scale_x = -scale_x;
    a = -a;
    b = -b;
  }

  let rotate = b.atan2(a);

  Some(Decomposed {
    translate_x: t.tx,
    translate_y: t.ty,
    scale_x,
    scale_y,
    rotate,
    skew_x,
  })
}

pub fn recompose(d: &Decomposed) -> Transform2D {
  let (sin, cos) = d.rotate.sin_cos();
  let tan_skew = d.skew_x.tan();

  Transform2D {
    a: cos * d.scale_x,
    b: sin * d.scale_x,
    c: (-sin + cos * tan_skew) * d.scale_y,
    d: (cos + sin * tan_skew) * d.scale_y,
    tx: d.translate_x,
    ty: d.translate_y,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn identity_roundtrip() {
    let d = decompose(&Transform2D::IDENTITY).unwrap();
    let r = recompose(&d);
    assert!((r.a - 1.0).abs() < 1e-6);
    assert!((r.d - 1.0).abs() < 1e-6);
    assert!(r.b.abs() < 1e-6);
    assert!(r.c.abs() < 1e-6);
  }

  #[test]
  fn rotation_roundtrip() {
    let t = Transform2D::rotate(0.5);
    let d = decompose(&t).unwrap();
    let r = recompose(&d);
    assert!((r.a - t.a).abs() < 1e-5);
    assert!((r.b - t.b).abs() < 1e-5);
    assert!((r.c - t.c).abs() < 1e-5);
    assert!((r.d - t.d).abs() < 1e-5);
  }

  #[test]
  fn decomposed_lerp_preserves_full_rotation() {
    let from = decompose(&Transform2D::rotate(0.0)).unwrap();
    let mut to = from;
    to.rotate = std::f32::consts::TAU;
    let mid = from.lerp(&to, 0.25);
    let expected = Transform2D::rotate(std::f32::consts::TAU * 0.25);
    let got = mid.to_matrix();
    assert!((got.a - expected.a).abs() < 0.05);
    assert!((got.b - expected.b).abs() < 0.05);
  }

  #[test]
  fn then_combines_transforms() {
    let a = Transform2D::translate(10.0, 0.0);
    let b = Transform2D::scale(2.0, 2.0);
    let c = a.then(&b);
    assert_eq!(c.a, 2.0);
    assert_eq!(c.tx, 10.0);
  }
}
