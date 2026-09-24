use std::sync::Arc;

use lurq::{
  app::theme::{
    BorderSize, RadiusSize, SpacingSize, ThemeBorderSizes, ThemeRadii, ThemeSpacing, ThemeTypography, TypographyStyle,
  },
  layout::text_style::{FontWeight, TextStyle},
  node::dimension::Dimension,
};

fn overline() -> TextStyle {
  TextStyle {
    font_size: 9.0,
    weight: FontWeight::SemiBold,
    ..TextStyle::default()
  }
}

#[test]
fn typography_extra_set_get_resolve() {
  let mut typography = ThemeTypography::new();
  typography.set("overline", overline());

  let style = typography.get(TypographyStyle::extra("overline"));
  assert_eq!(style.font_size, 9.0);
  assert_eq!(style.weight, FontWeight::SemiBold);
  assert!(typography.resolve("overline") == overline());
  assert!(typography.try_resolve(Arc::<str>::from("overline")) == Some(overline()));
  assert!(typography.extra.contains_key("overline"));
  // Named roles and their defaults are untouched.
  assert!(typography.get(TypographyStyle::Body) == ThemeTypography::default().body);
}

#[test]
fn typography_extra_missing_name_is_none_for_try_get() {
  let typography = ThemeTypography::default();
  assert!(typography.try_get("missing").is_none());
  assert!(typography.try_resolve(TypographyStyle::extra("missing")).is_none());
}

#[test]
#[should_panic(expected = "typography style not found: missing")]
fn typography_extra_missing_name_panics_for_get() {
  ThemeTypography::default().get("missing");
}

#[test]
fn radius_extra_set_get_resolve() {
  let mut radii = ThemeRadii::new();
  radii.set("card", 10.0);
  assert_eq!(radii.get(RadiusSize::extra("card")), 10.0);
  assert_eq!(radii.resolve("card"), 10.0);
  assert_eq!(radii.try_get(Arc::<str>::from("card")), Some(10.0));
  assert_eq!(radii.try_get("missing"), None);
  assert_eq!(radii.get(RadiusSize::Md), ThemeRadii::default().md);
}

#[test]
#[should_panic(expected = "radius size not found: missing")]
fn radius_extra_missing_name_panics_for_get() {
  ThemeRadii::default().get("missing");
}

#[test]
fn spacing_extra_set_get_resolve() {
  let mut spacing = ThemeSpacing::new();
  spacing.set("gutter", 20.0);
  assert_eq!(spacing.get(SpacingSize::extra("gutter")), Dimension::Px(20.0));
  assert_eq!(spacing.resolve("gutter"), Dimension::Px(20.0));
  assert_eq!(spacing.try_resolve("missing"), None);
  assert_eq!(spacing.get(SpacingSize::Section), ThemeSpacing::default().section);
}

#[test]
#[should_panic(expected = "spacing size not found: missing")]
fn spacing_extra_missing_name_panics_for_get() {
  ThemeSpacing::default().get("missing");
}

#[test]
fn border_size_extra_set_get_resolve() {
  let mut border_sizes = ThemeBorderSizes::new();
  border_sizes.set("hairline", 0.5);
  assert_eq!(border_sizes.get(BorderSize::extra("hairline")), 0.5);
  assert_eq!(border_sizes.resolve("hairline"), 0.5);
  assert_eq!(border_sizes.try_get("missing"), None);
  assert_eq!(border_sizes.get(BorderSize::Lg), ThemeBorderSizes::default().lg);
}

#[test]
#[should_panic(expected = "border size not found: missing")]
fn border_size_extra_missing_name_panics_for_get() {
  ThemeBorderSizes::default().get("missing");
}

#[test]
fn extra_roles_stay_copy_and_compare_by_name() {
  let owned = String::from("card");
  let radius = RadiusSize::from(owned.as_str());
  let copy = radius;
  assert_eq!(radius, copy);
  assert_eq!(radius, RadiusSize::extra("card"));
  assert_eq!(radius, RadiusSize::from(Arc::<str>::from("card")));
  assert_eq!(radius.as_str(), "card");
  assert_ne!(radius, RadiusSize::extra("panel"));

  let style = TypographyStyle::extra("overline");
  let copy = style;
  assert_eq!(style, copy);
  assert_eq!(style.as_str(), "overline");
  assert_eq!(SpacingSize::extra("gutter").as_str(), "gutter");
  assert_eq!(BorderSize::extra("hairline").as_str(), "hairline");
  // A dynamic name is interned once and shared by later conversions.
  assert!(std::ptr::eq(
    RadiusSize::extra(owned.clone()).as_str(),
    RadiusSize::extra(owned).as_str()
  ));
}
