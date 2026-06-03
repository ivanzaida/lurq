use lurq::{
  app::theme::{PaletteId, Theme},
  node::color::Color,
};

#[test]
fn theme_lens_reads_sets_and_updates_focused_value() {
  const BRAND: PaletteId = PaletteId::new(8);
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
