use lurq::{
  app::theme::{PaletteColor, Theme},
  node::color::Color,
};

#[test]
fn theme_lens_reads_sets_and_updates_focused_value() {
  let theme = Theme::new();
  let brand = theme.lens(
    |theme| theme.palette_color(PaletteColor::Accent),
    |theme, color| theme.set_palette_color(PaletteColor::Accent, color),
  );

  assert_eq!(brand.get().to_hex(), "#2563eb");

  brand.set(Color::from_hex("#123456"));
  assert_eq!(theme.palette_color(PaletteColor::Accent).to_hex(), "#123456");

  brand.update(|color| *color = Color::from_hex("#abcdef"));
  assert_eq!(brand.get().to_hex(), "#abcdef");
}
