use lurq::{
  app::theme::{Theme, ThemeColors, ThemeValue},
  node::color::Color,
};

#[test]
fn default_theme_has_colors() {
  let t = Theme::new();
  let colors = t.colors();
  assert_eq!(colors.primary.r(), 59);
}

#[test]
fn set_colors_updates() {
  let t = Theme::new();
  t.set_colors(ThemeColors {
    primary: Color::from_hex("#ff0000"),
    ..ThemeColors::default()
  });
  assert_eq!(t.colors().primary.r(), 255);
}

#[test]
fn default_theme_has_fonts() {
  let t = Theme::new();
  let fonts = t.fonts();
  assert_eq!(fonts.body.font_size, 16.0);
  assert_eq!(fonts.heading.font_size, 24.0);
}

#[test]
fn set_fonts_updates() {
  let t = Theme::new();
  let mut fonts = t.fonts();
  fonts.body.font_size = 18.0;
  t.set_fonts(fonts);
  assert_eq!(t.fonts().body.font_size, 18.0);
}

#[test]
fn custom_prop_string() {
  let t = Theme::new();
  t.set("app_name", "demo");
  let val = t.get("app_name").unwrap();
  assert_eq!(val.as_str(), Some("demo"));
}

#[test]
fn custom_prop_u32() {
  let t = Theme::new();
  t.set("max_items", 100_u32);
  assert_eq!(t.get("max_items").unwrap().as_u32(), Some(100));
}

#[test]
fn custom_prop_f32() {
  let t = Theme::new();
  t.set("scale", 1.5_f32);
  assert_eq!(t.get("scale").unwrap().as_f32(), Some(1.5));
}

#[test]
fn custom_prop_missing_returns_none() {
  let t = Theme::new();
  assert!(t.get("nonexistent").is_none());
}

#[test]
fn custom_prop_overwrite() {
  let t = Theme::new();
  t.set("key", "old");
  t.set("key", "new");
  assert_eq!(t.get("key").unwrap().as_str(), Some("new"));
}

#[test]
fn theme_clone_shares_state() {
  let t1 = Theme::new();
  let t2 = t1.clone();
  t1.set("shared", 42_u32);
  assert_eq!(t2.get("shared").unwrap().as_u32(), Some(42));
}

#[test]
fn theme_value_from_str() {
  let v: ThemeValue = "hello".into();
  assert_eq!(v.as_str(), Some("hello"));
}

#[test]
fn theme_value_from_string() {
  let v: ThemeValue = "world".to_owned().into();
  assert_eq!(v.as_str(), Some("world"));
}

#[test]
fn theme_value_from_u32() {
  let v: ThemeValue = 42_u32.into();
  assert_eq!(v.as_u32(), Some(42));
}

#[test]
fn theme_value_from_f32() {
  let v: ThemeValue = 33.14_f32.into();
  assert_eq!(v.as_f32(), Some(33.14));
}

#[test]
fn theme_value_wrong_type_returns_none() {
  let v: ThemeValue = 42_u32.into();
  assert!(v.as_str().is_none());
  assert!(v.as_f32().is_none());
}
