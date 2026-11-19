//! The icons the theme starts with are the icons of the font.
//!
//! `design/icons/codepoints.json` is what `scripts/build-icon-font.py` builds
//! the font from, and `ui::icons` is the module it writes for the names. This
//! holds the two together: a glyph added to the board with the font built and
//! the module left as it was fails here.

use std::path::Path;

use lurq::{
  app::theme::{Theme, ThemeDefinition, ThemeIcons},
  ui::icons,
};

/// The table of the font: each name with its character.
fn codepoints() -> Vec<(String, char)> {
  let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../design/icons/codepoints.json");
  let text = std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
  // A line of the table is `"name": "E000",`.
  text
    .lines()
    .filter_map(|line| {
      let mut quoted = line.split('"');
      let (name, code) = (quoted.nth(1)?, quoted.nth(1)?);
      let glyph = char::from_u32(u32::from_str_radix(code, 16).ok()?)?;
      Some((name.to_owned(), glyph))
    })
    .collect()
}

#[test]
fn the_names_of_the_module_are_the_names_of_the_font() {
  let font = codepoints();
  assert!(font.len() > 100, "the table of the font was read");
  let module: Vec<(String, char)> = icons::ALL
    .iter()
    .map(|(name, glyph)| ((*name).to_owned(), *glyph))
    .collect();
  let mut sorted = font.clone();
  sorted.sort_by_key(|(_, glyph)| *glyph);
  assert_eq!(
    module, sorted,
    "run `python scripts/build-icon-font.py`: it writes `ui/icons.rs` from the table"
  );
}

#[test]
fn a_theme_starts_with_the_icons_of_the_font() {
  let theme = Theme::new();
  assert_eq!(theme.icons().len(), icons::ALL.len());
  for (name, glyph) in icons::ALL {
    let drawn = theme.icon(*name).unwrap_or_else(|| panic!("the theme has {name}"));
    assert_eq!((&*drawn.family, drawn.glyph), (icons::FAMILY, *glyph));
  }
  assert_eq!(ThemeDefinition::neutral().icons, ThemeIcons::lurq());
  assert_eq!(
    ThemeDefinition::bare().icons,
    ThemeIcons::lurq(),
    "a bare theme has no recipe, and has its icons"
  );
}
