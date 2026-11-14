use lurq::{
  app::theme::{PaletteColor, Theme},
  node::color::Color,
};

#[test]
fn theme_lens_reads_sets_and_updates_focused_value() {
  let theme = Theme::new();
  let brand = theme.lens(
    |theme| theme.palette_color(PaletteColor::Primary),
    |theme, color| theme.set_palette_color(PaletteColor::Primary, color),
  );

  assert_eq!(brand.get().to_hex(), "#18181b");

  brand.set(Color::from_hex("#123456"));
  assert_eq!(theme.palette_color(PaletteColor::Primary).to_hex(), "#123456");

  brand.update(|color| *color = Color::from_hex("#abcdef"));
  assert_eq!(brand.get().to_hex(), "#abcdef");
}
