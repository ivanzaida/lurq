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
    Self { tx, ty, ..Self::IDENTITY }
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

#[derive(Clone, Copy, Debug)]
pub(crate) struct Decomposed {
  pub translate_x: f32,
  pub translate_y: f32,
  pub scale_x: f32,
  pub scale_y: f32,
  pub rotate: f32,
  pub skew_x: f32,
}

pub(crate) fn decompose(t: &Transform2D) -> Option<Decomposed> {
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

pub(crate) fn recompose(d: &Decomposed) -> Transform2D {
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

pub(crate) fn lerp_transform(from: &Transform2D, to: &Transform2D, t: f32) -> Option<Transform2D> {
  let da = decompose(from)?;
  let db = decompose(to)?;

  let mut rot_diff = db.rotate - da.rotate;
  if rot_diff > PI {
    rot_diff -= 2.0 * PI;
  }
  if rot_diff < -PI {
    rot_diff += 2.0 * PI;
  }

  let lerp = |a: f32, b: f32| a + (b - a) * t;

  Some(recompose(&Decomposed {
    translate_x: lerp(da.translate_x, db.translate_x),
    translate_y: lerp(da.translate_y, db.translate_y),
    scale_x: lerp(da.scale_x, db.scale_x),
    scale_y: lerp(da.scale_y, db.scale_y),
    rotate: da.rotate + rot_diff * t,
    skew_x: lerp(da.skew_x, db.skew_x),
  }))
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
  fn lerp_identity_to_rotation() {
    let from = Transform2D::IDENTITY;
    let to = Transform2D::rotate(PI / 2.0);
    let mid = lerp_transform(&from, &to, 0.5).unwrap();
    let d = decompose(&mid).unwrap();
    assert!((d.rotate - PI / 4.0).abs() < 0.01);
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
