//! What the gallery's own interface is built from, until the design system
//! has the component for it: each of these goes when its component lands.
//!
//! Everything here takes its colors, sizes and type from the theme, so the
//! gallery is light or dark with it.

use std::sync::{Arc, OnceLock};

use lurq::{
  app::theme::{BorderSize, PaletteColor, RadiusSize, ShadowStyle, TypographyStyle},
  components::{Column, Popup, Row, Text},
  core::{ElementRef, Signal},
  layout::{
    Alignment,
    layout_kind::Justify,
    text_style::{FontWeight, TextStyle},
  },
  node::{CursorIcon, Element, border::Border, dimension::Dimension},
};

pub const FILL: Dimension = Dimension::Pct(100.0);

/// A text in a typography style of the theme with something of it changed: a size, a weight. It names no color,
/// so it takes the foreground of what it is in.
pub fn styled(content: &str, role: TypographyStyle, change: impl FnOnce(&mut TextStyle)) -> Text {
  let mut style = lurq::ui::current_theme().typography_style(role);
  change(&mut style);
  Text::styled(content, style)
}

/// Text that says less: a caption, a hint.
pub fn muted(content: &str, style: TypographyStyle) -> Text {
  Text::new(content).variant(style).color(PaletteColor::MutedForeground)
}

/// The family of lurq's icon font, loaded when the gallery starts.
pub const ICON_FAMILY: &str = "lurq icons";

/// The icons of lurq's font by name, in the order of the font.
pub fn icons() -> &'static [(String, char)] {
  static ICONS: OnceLock<Vec<(String, char)>> = OnceLock::new();
  ICONS.get_or_init(|| {
    // A line of the table is `"name": "E000",`.
    include_str!("../../../design/icons/codepoints.json")
      .lines()
      .filter_map(|line| {
        let mut quoted = line.split('"');
        let (name, code) = (quoted.nth(1)?, quoted.nth(1)?);
        Some((name.to_owned(), char::from_u32(u32::from_str_radix(code, 16).ok()?)?))
      })
      .collect()
  })
}

/// An icon of lurq's font, `size` high. It names no color, so it takes the
/// foreground of what it is in.
///
/// The design system's `Icon` takes its place.
pub fn icon(name: &str, size: f32) -> Text {
  let glyph = icons()
    .iter()
    .find_map(|(known, glyph)| (known == name).then_some(*glyph))
    .unwrap_or_else(|| panic!("lurq's icon font has no icon {name:?}"));
  Text::styled(
    &glyph.to_string(),
    TextStyle {
      font_family: ICON_FAMILY.into(),
      font_size: size,
      line_height: 1.0,
      ..TextStyle::default()
    },
  )
}

/// A button that is an icon: the settings.
pub fn icon_button(name: &str) -> Row {
  let ring = lurq::ui::current_theme().focus_ring().outline();
  Row::new()
    .size(36.0, 36.0)
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .rounded(RadiusSize::Md)
    .cursor(CursorIcon::Pointer)
    .foreground(PaletteColor::MutedForeground)
    .hovered(|style| {
      style
        .background(PaletteColor::Highlight)
        .foreground(PaletteColor::HighlightForeground)
    })
    .focus_visible(|style| style.outline(ring))
    .tab_index(0)
    .child(icon(name, 18.0))
}

/// A key or a chord of keys, as it is on a keyboard: `Ctrl K`.
pub fn keys(chord: &str) -> Row {
  Row::new()
    .align_items(Alignment::Center)
    .height(20.0)
    .padding_horizontal(6.0)
    .rounded(RadiusSize::Sm)
    .background(PaletteColor::Muted)
    .border(Border::inside(BorderSize::Sm, PaletteColor::Border))
    .child(muted(chord, TypographyStyle::Caption))
}

/// The panel of a popup: a menu, the settings.
pub fn popover() -> Column {
  Column::new()
    .padding(4.0)
    .rounded(RadiusSize::Lg)
    .background(PaletteColor::Popover)
    .foreground(PaletteColor::PopoverForeground)
    .border(Border::inside(BorderSize::Sm, PaletteColor::Border))
    .box_shadow(ShadowStyle::Md)
}

/// One choice of a few, shown closed with the list of them in a popup: the density.
///
/// A row of the list is as high as a row is at the theme's density. The
/// design system's `Select` takes its place.
pub fn select<T: Clone + PartialEq + Send + Sync + 'static>(
  anchor: &ElementRef,
  open: &Signal<bool>,
  options: &[(&str, T)],
  chosen: &T,
  choose: impl Fn(T) + Send + Sync + 'static,
) -> Row {
  let theme = lurq::ui::current_theme();
  let ring = theme.focus_ring().outline();
  let row = theme.density_value(theme.density());
  let choose = Arc::new(choose);
  let shown = options
    .iter()
    .find_map(|(name, value)| (value == chosen).then_some(*name))
    .unwrap_or_default();
  let trigger = Row::new()
    .width(FILL)
    .height(36.0)
    .padding_horizontal(12.0)
    .spacing(8.0)
    .align_items(Alignment::Center)
    .justify(Justify::SpaceBetween)
    .rounded(RadiusSize::Md)
    .background(PaletteColor::Background)
    .border(Border::inside(BorderSize::Sm, PaletteColor::Input))
    .cursor(CursorIcon::Pointer)
    .focus_visible(|style| style.outline(ring))
    // Focusable, so that a press on it while another popup is open closes that one and still reaches it.
    .tab_index(0)
    .ref_element(anchor.clone())
    .on_click({
      let open = open.clone();
      move |_| open.update(|open| *open = !*open)
    })
    .child(Text::new(shown).variant(TypographyStyle::Body))
    .child(icon("chevron-down", 16.0).color(PaletteColor::MutedForeground));
  let mut list = popover().spacing(2.0);
  for (name, value) in options {
    let on = value == chosen;
    let (choose, value, open) = (choose.clone(), value.clone(), open.clone());
    list = list.child(
      Row::new()
        .width(FILL)
        .height(row)
        .padding_horizontal(8.0)
        .spacing(8.0)
        .align_items(Alignment::Center)
        .justify(Justify::SpaceBetween)
        .rounded(RadiusSize::Sm)
        .cursor(CursorIcon::Pointer)
        .hovered(|style| {
          style
            .background(PaletteColor::Highlight)
            .foreground(PaletteColor::HighlightForeground)
        })
        .on_click(move |_| {
          choose(value.clone());
          open.set(false);
        })
        .child(Text::new(name).variant(TypographyStyle::Body))
        .child(icon("check", 16.0).opacity(if on { 1.0 } else { 0.0 })),
    );
  }
  // The list is beside the trigger, not in it: a click on a row of it is not a click on the trigger.
  Row::new().child(trigger).child(
    Popup::new(anchor.clone(), list)
      .open(open.clone())
      .offset(0.0, 4.0)
      .match_anchor_width(true),
  )
}

/// A line of code, in the theme's family of code.
pub fn code(content: &str) -> Text {
  Text::new(content).variant(TypographyStyle::Mono)
}

/// A small label in a tinted box: a version, a status.
pub fn badge(content: &str) -> Row {
  Row::new()
    .align_items(Alignment::Center)
    .height(22.0)
    .padding_horizontal(8.0)
    .rounded(RadiusSize::Full)
    .background(PaletteColor::Secondary)
    .foreground(PaletteColor::SecondaryForeground)
    .child(styled(content, TypographyStyle::Caption, |style| {
      style.weight = FontWeight::Medium
    }))
}

/// A badge for something that went well: "Stable".
pub fn status_badge(content: &str) -> Row {
  badge(content)
    .background(PaletteColor::SuccessMuted)
    .foreground(PaletteColor::Success)
}

/// One of a few choices in a row, of which one is chosen: the appearance.
///
/// The design system's `SegmentedControl` takes its place.
pub fn segmented<T: Clone + PartialEq + Send + Sync + 'static>(
  options: &[(&str, T)],
  chosen: &T,
  choose: impl Fn(T) + Send + Sync + 'static,
) -> Row {
  let choose = Arc::new(choose);
  let mut control = Row::new()
    .align_items(Alignment::Center)
    .height(36.0)
    .padding(3.0)
    .rounded(RadiusSize::Md)
    .background(PaletteColor::Muted);
  for (name, value) in options {
    let on = value == chosen;
    let (choose, value) = (choose.clone(), value.clone());
    let mut segment = Row::new()
      .align_items(Alignment::Center)
      .height(30.0)
      .padding_horizontal(12.0)
      .rounded(RadiusSize::Sm)
      .cursor(CursorIcon::Pointer)
      .foreground(if on {
        PaletteColor::Foreground
      } else {
        PaletteColor::MutedForeground
      })
      .on_click(move |_| choose(value.clone()))
      .child(Text::new(name).variant(TypographyStyle::Label));
    if on {
      segment = segment.background(PaletteColor::Background).box_shadow(ShadowStyle::Sm);
    } else {
      segment = segment.hovered(|style| style.foreground(PaletteColor::Foreground));
    }
    control = control.child(segment);
  }
  control
}

/// A card: a surface with a line around it.
pub fn card() -> Column {
  Column::new()
    .width(FILL)
    .rounded(RadiusSize::Lg)
    .background(PaletteColor::Card)
    .foreground(PaletteColor::CardForeground)
    .border(Border::inside(BorderSize::Sm, PaletteColor::Border))
}

/// The stage of a demo card: where the live components stand, on the canvas.
pub fn stage(content: impl Into<Element>) -> Column {
  Column::new()
    .width(FILL)
    .padding(32.0)
    .background(PaletteColor::Canvas)
    .foreground(PaletteColor::Foreground)
    .child(content)
}

/// The footer of a demo card: the builder call that makes what the stage shows.
pub fn card_footer(call: &str) -> Row {
  Row::new()
    .width(FILL)
    .height(40.0)
    .padding_horizontal(16.0)
    .align_items(Alignment::Center)
    .border_top(Border::inside(BorderSize::Sm, PaletteColor::Border))
    .child(muted(call, TypographyStyle::Mono))
}

/// A demo card: a stage and the call that builds what is on it.
pub fn demo_card(content: impl Into<Element>, call: &str) -> Column {
  card().child(stage(content)).child(card_footer(call))
}
