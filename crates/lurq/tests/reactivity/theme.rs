use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{
    App, Tree,
    component::Component,
    ctx::Ctx,
    theme::{
      PaletteId, RadiusId, SpacingId, Theme, ThemePalette, ThemeRadii, ThemeSpacing, ThemeTypography, TypographyId,
    },
  },
  layout::text_style::{FontWeight, TextStyle},
  node::{Element, color::Color, dimension::Dimension},
};

use crate::support::run_pass;

#[derive(lurq::DevtoolsInspectable)]
struct Shared<T>(Arc<T>);

impl<T> Clone for Shared<T> {
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }
}

impl<T> PartialEq for Shared<T> {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

impl<T> std::fmt::Debug for Shared<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_tuple("Shared").field(&(Arc::as_ptr(&self.0) as usize)).finish()
  }
}

#[test]
fn default_theme_has_palette() {
  let t = Theme::new();
  assert!(t.palette_color(PaletteId::new(0)).is_none());
}

#[test]
fn theme_token_apis_accept_raw_and_borrowed_ids() {
  let mut palette = ThemePalette::from_colors([(1u8, Color::from_hex("#123456"))]);
  let palette_id = PaletteId::new(1);
  palette.set(&palette_id, Color::from_hex("#abcdef"));
  assert_eq!(palette.get(&palette_id).unwrap().to_hex(), "#abcdef");
  assert_eq!(palette.resolve(1u8).to_hex(), "#abcdef");
  assert_eq!(palette.resolve(&palette_id).to_hex(), "#abcdef");

  let mut spacing = ThemeSpacing::from_values([(2u8, 12.0)]);
  let spacing_id = SpacingId::new(2);
  spacing.set(&spacing_id, Dimension::Pct(5.0));
  assert_eq!(spacing.get(&spacing_id), Some(Dimension::Pct(5.0)));
  assert_eq!(spacing.get(2u8), Some(Dimension::Pct(5.0)));

  let mut radii = ThemeRadii::from_values([(3u8, 4.0)]);
  let radius_id = RadiusId::new(3);
  radii.set(&radius_id, 8.0);
  assert_eq!(radii.get(&radius_id), Some(8.0));
  assert_eq!(radii.get(3u8), Some(8.0));

  let style = TextStyle {
    font_size: 22.0,
    ..TextStyle::default()
  };
  let mut typography = ThemeTypography::from_styles([(4u8, style.clone())]);
  let typography_id = TypographyId::new(4);
  typography.set(
    &typography_id,
    TextStyle {
      font_size: 24.0,
      ..style
    },
  );
  assert_eq!(typography.get(&typography_id).unwrap().font_size, 24.0);
  assert_eq!(typography.resolve(4u8).font_size, 24.0);
  assert_eq!(typography.resolve(&typography_id).font_size, 24.0);
}

#[test]
fn set_palette_color_updates_variant() {
  const BRAND: PaletteId = PaletteId::new(20);
  let t = Theme::new();
  t.set_palette_color(BRAND, Color::from_hex("#123456"));

  assert_eq!(t.palette_color(BRAND).unwrap().to_hex(), "#123456");
}

#[test]
fn register_palette_color_allocates_unique_ids() {
  let t = Theme::new();
  let brand = t.register_palette_color(Color::from_hex("#123456"));
  let accent = t.register_palette_color(Color::from_hex("#abcdef"));

  assert_eq!(brand, PaletteId::new(0));
  assert_eq!(accent, PaletteId::new(1));
  assert_ne!(brand, accent);
  assert_eq!(t.palette_color(brand).unwrap().to_hex(), "#123456");
  assert_eq!(t.palette_color(accent).unwrap().to_hex(), "#abcdef");
}

#[test]
fn set_palette_replaces_registry() {
  const BRAND: PaletteId = PaletteId::new(20);
  let t = Theme::new();
  let palette = ThemePalette::from_colors([(BRAND, Color::from_hex("#abcdef"))]);

  t.set_palette(palette);

  assert_eq!(t.palette_color(BRAND).unwrap().to_hex(), "#abcdef");
  assert!(t.palette_color(PaletteId::new(0)).is_none());
}

#[test]
fn default_theme_has_empty_spacing() {
  let t = Theme::new();
  assert!(t.spacing_value(SpacingId::new(0)).is_none());
}

#[test]
fn set_spacing_value_updates_token() {
  const GAP: SpacingId = SpacingId::new(9);
  let t = Theme::new();
  t.set_spacing_value(GAP, 12.0);

  assert_eq!(t.spacing_value(GAP), Some(Dimension::Px(12.0)));
}

#[test]
fn set_spacing_value_accepts_dimension() {
  const GAP: SpacingId = SpacingId::new(9);
  let t = Theme::new();
  t.set_spacing_value(GAP, Dimension::Pct(5.0));

  assert_eq!(t.spacing_value(GAP), Some(Dimension::Pct(5.0)));
}

#[test]
fn register_spacing_allocates_unique_ids() {
  let t = Theme::new();
  let small = t.register_spacing(4.0);
  let medium = t.register_spacing(8.0);

  assert_eq!(small, SpacingId::new(0));
  assert_eq!(medium, SpacingId::new(1));
  assert_ne!(small, medium);
  assert_eq!(t.spacing_value(small), Some(Dimension::Px(4.0)));
  assert_eq!(t.spacing_value(medium), Some(Dimension::Px(8.0)));
}

#[test]
fn set_spacing_replaces_registry() {
  const GAP: SpacingId = SpacingId::new(9);
  let t = Theme::new();
  let spacing = ThemeSpacing::from_values([(GAP, 12.0)]);

  t.set_spacing(spacing);

  assert_eq!(t.spacing_value(GAP), Some(Dimension::Px(12.0)));
  assert!(t.spacing_value(SpacingId::new(0)).is_none());
}

#[test]
fn default_theme_has_empty_radii() {
  let t = Theme::new();
  assert!(t.radius_value(RadiusId::new(0)).is_none());
}

#[test]
fn set_radius_value_updates_token() {
  const CARD: RadiusId = RadiusId::new(9);
  let t = Theme::new();
  t.set_radius_value(CARD, 6.0);

  assert_eq!(t.radius_value(CARD), Some(6.0));
}

#[test]
fn register_radius_allocates_unique_ids() {
  let t = Theme::new();
  let small = t.register_radius(4.0);
  let medium = t.register_radius(8.0);

  assert_eq!(small, RadiusId::new(0));
  assert_eq!(medium, RadiusId::new(1));
  assert_ne!(small, medium);
  assert_eq!(t.radius_value(small), Some(4.0));
  assert_eq!(t.radius_value(medium), Some(8.0));
}

#[test]
fn set_radii_replaces_registry() {
  const CARD: RadiusId = RadiusId::new(9);
  let t = Theme::new();
  let radii = ThemeRadii::from_values([(CARD, 6.0)]);

  t.set_radii(radii);

  assert_eq!(t.radius_value(CARD), Some(6.0));
  assert!(t.radius_value(RadiusId::new(0)).is_none());
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
fn default_theme_has_typography() {
  let t = Theme::new();
  assert_eq!(t.default_text_style().font_size, 16.0);
  assert!(t.typography_style(TypographyId::new(0)).is_none());
}

#[test]
fn set_typography_style_updates_variant() {
  const DISPLAY: TypographyId = TypographyId::new(10);
  let t = Theme::new();
  t.set_typography_style(
    DISPLAY,
    TextStyle {
      font_size: 32.0,
      weight: FontWeight::Bold,
      ..TextStyle::default()
    },
  );

  let style = t.typography_style(DISPLAY).unwrap();
  assert_eq!(style.font_size, 32.0);
  assert!(style.weight == FontWeight::Bold);
}

#[test]
fn register_typography_style_allocates_unique_ids() {
  let t = Theme::new();
  let label = t.register_typography_style(TextStyle {
    font_size: 13.0,
    ..TextStyle::default()
  });
  let display = t.register_typography_style(TextStyle {
    font_size: 32.0,
    ..TextStyle::default()
  });

  assert_eq!(label, TypographyId::new(0));
  assert_eq!(display, TypographyId::new(1));
  assert_ne!(label, display);
  assert_eq!(t.typography_style(label).unwrap().font_size, 13.0);
  assert_eq!(t.typography_style(display).unwrap().font_size, 32.0);
}

#[test]
fn set_typography_replaces_registry() {
  const CAPTION: TypographyId = TypographyId::new(11);
  let t = Theme::new();
  let typography = ThemeTypography::from_styles([(
    CAPTION,
    TextStyle {
      font_size: 11.0,
      ..TextStyle::default()
    },
  )]);

  t.set_typography(typography);

  assert_eq!(t.typography_style(CAPTION).unwrap().font_size, 11.0);
  assert!(t.typography_style(TypographyId::new(0)).is_none());
}

#[test]
fn theme_lens_reads_sets_and_updates_value() {
  const BRAND: PaletteId = PaletteId::new(4);
  let theme = Theme::new();
  let brand = theme.lens(
    |theme| theme.palette_color(BRAND).unwrap_or(Color::from_hex("#000000")),
    |theme, color| theme.set_palette_color(BRAND, color),
  );

  assert_eq!(brand.get().to_hex(), "#000000");

  brand.set(Color::from_hex("#123456"));
  assert_eq!(theme.palette_color(BRAND).unwrap().to_hex(), "#123456");

  brand.update(|color| *color = Color::from_hex("#abcdef"));
  assert_eq!(brand.get().to_hex(), "#abcdef");
}

struct ThemeSubscriberRoot;

struct ThemeSubscriberChild {
  renders: Arc<AtomicUsize>,
}

impl Component for ThemeSubscriberRoot {
  type Props = Shared<AtomicUsize>;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    ctx.mount::<ThemeSubscriberChild>(ctx.props::<Self::Props>().clone())
  }
}

impl Component for ThemeSubscriberChild {
  type Props = Shared<AtomicUsize>;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      renders: ctx.props::<Self::Props>().0.clone(),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    self.renders.fetch_add(1, Ordering::Relaxed);
    let font_size = ctx.theme().default_text_style().font_size;
    lurq::components::Text::new(&format!("font={font_size}"))
  }
}

#[test]
fn theme_change_rerenders_subscriber_components() {
  let renders = Arc::new(AtomicUsize::new(0));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<ThemeSubscriberRoot>(&mut app, Shared(renders.clone()));

  run_pass(&mut tree);
  assert_eq!(renders.load(Ordering::Relaxed), 1);

  app.theme().set_default_text_style(TextStyle {
    font_size: 22.0,
    ..TextStyle::default()
  });
  run_pass(&mut tree);

  assert_eq!(renders.load(Ordering::Relaxed), 2);
  assert!(tree.find_element(|el| el.text_content() == Some("font=22")).is_some());
}

#[test]
fn theme_clone_shares_state() {
  let t1 = Theme::new();
  let t2 = t1.clone();
  t1.set_palette_color(PaletteId::new(7), Color::from_hex("#123456"));
  assert_eq!(t2.palette_color(PaletteId::new(7)).unwrap().to_hex(), "#123456");
}
