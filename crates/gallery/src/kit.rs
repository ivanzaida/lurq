//! What the gallery's own interface is built from, until the design system
//! has the component for it: each of these goes when its component lands.
//!
//! Everything here takes its colors, sizes and type from the theme, so the
//! gallery is light or dark with it.

use std::sync::Arc;

use lurq::{
  app::theme::{BorderSize, PaletteColor, RadiusSize, ShadowStyle, TypographyStyle},
  components::{Column, Popup, Rect, Row, ScrollHorizontal, Text},
  core::{ElementRef, Signal},
  layout::{
    Alignment,
    layout_kind::Justify,
    text_style::{FontWeight, TextStyle},
  },
  node::{CursorIcon, Element, border::Border, dimension::Dimension},
  ui::{Button, ButtonSize, ButtonVariant, FontFamily, Icon, SegmentedControl, SegmentedControlPart},
};

pub const FILL: Dimension = Dimension::Pct(100.0);

/// A text in a typography style of the theme with something of it changed: a size, a weight. It names no color,
/// so it takes the foreground of what it is in.
pub fn styled(content: &str, role: TypographyStyle, change: impl FnOnce(&mut TextStyle)) -> Text {
  let mut style = lurq::ui::current_theme().typography_style(role);
  change(&mut style);
  Text::styled(content, style)
}

/// Interface text at a size of the design, in the theme's family and at its
/// text scale. It names no color, so it takes the foreground of what it is in.
pub fn text(content: &str, size: f32, weight: FontWeight) -> Text {
  let theme = lurq::ui::current_theme();
  let mut style = theme.typography_style(TypographyStyle::Body);
  style.font_size = theme.scaled(size);
  style.weight = weight;
  Text::styled(content, style)
}

/// Text in the theme's family of code at a size of the design: a label of the
/// interface, a count, a version.
pub fn mono(content: &str, size: f32) -> Text {
  let theme = lurq::ui::current_theme();
  let mut style = theme.typography_style(TypographyStyle::Mono);
  style.font_size = theme.scaled(size);
  Text::styled(content, style)
}

/// A label in capitals over a list: a group of the side bar, the list of a page's sections.
pub fn overline(content: &str) -> Text {
  let theme = lurq::ui::current_theme();
  let mut style = theme.typography_style(TypographyStyle::Mono);
  style.font_size = theme.scaled(11.0);
  style.letter_spacing = 1.0;
  Text::styled(&content.to_uppercase(), style).color(PaletteColor::MutedForeground)
}

/// Text that says less: a caption, a hint.
pub fn muted(content: &str, style: TypographyStyle) -> Text {
  Text::new(content).variant(style).color(PaletteColor::MutedForeground)
}

/// An icon of the theme's set, `size` high.
pub fn icon(name: &str, size: f32) -> Icon {
  Icon::new(name).size(size)
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
    .child(mono(chord, 11.0).color(PaletteColor::MutedForeground))
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

/// How high a row of the gallery's own lists is: the side bar, a menu. It is
/// 32 at the default density, as the design draws it, and moves with the
/// density by as much as a row of the theme does.
pub fn row_height(theme: &lurq::app::theme::Theme) -> f32 {
  use lurq::app::theme::Density;
  32.0 + theme.density_value(theme.density()) - theme.density_value(Density::Default)
}

/// One choice of a few, shown closed with the list of them in a popup: the density.
///
/// The design system's `Select` takes its place.
pub fn select<T: Clone + PartialEq + Send + Sync + 'static>(
  anchor: &ElementRef,
  open: &Signal<bool>,
  options: &[(&str, T)],
  chosen: &T,
  choose: impl Fn(T) + Send + Sync + 'static,
) -> Row {
  let theme = lurq::ui::current_theme();
  let ring = theme.focus_ring().outline();
  let row = row_height(&theme);
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
    .child(text(shown, 14.0, FontWeight::Normal))
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
        .child(text(name, 14.0, FontWeight::Normal))
        // The mark of the one that is chosen, and its room beside the others.
        .child(if on {
          Element::from(icon("check", 16.0))
        } else {
          Rect::new(16.0, 16.0).into()
        }),
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

/// One of a few choices in a row, of which one is chosen: the appearance.
pub fn segmented<T: Clone + PartialEq + Send + Sync + 'static>(
  options: &[(&str, T)],
  chosen: &T,
  choose: impl Fn(T) + Send + Sync + 'static,
) -> SegmentedControl {
  SegmentedControl::controlled(
    chosen.clone(),
    options.iter().map(|(label, value)| (value.clone(), *label)),
    choose,
  )
}

/// A few choices that share the width they are given, named as the code names them: a size, a shape.
pub fn segmented_fill<T: Clone + PartialEq + Send + Sync + 'static>(
  options: &[(&str, T)],
  chosen: &T,
  choose: impl Fn(T) + Send + Sync + 'static,
) -> SegmentedControl {
  segmented(options, chosen, choose)
    .full_width(true)
    .style(SegmentedControlPart::Label, |s| {
      s.font_family(FontFamily::Mono).font_size(12.0)
    })
}

/// A line under a row or beside a column.
pub fn line() -> Border {
  Border::inside(BorderSize::Sm, PaletteColor::Border)
}

/// A label in the family of code, quiet: the name of a control, a caption.
pub fn quiet(content: &str, size: f32) -> Text {
  mono(content, size).color(PaletteColor::MutedForeground)
}

/// A name as the code has it, in the weight of a name.
pub fn name(content: &str, size: f32) -> Text {
  let theme = lurq::ui::current_theme();
  let mut style = theme.typography_style(TypographyStyle::Mono);
  style.font_size = theme.scaled(size);
  style.weight = FontWeight::Medium;
  Text::styled(content, style)
}

/// The table of what a component is given: a prop, its type, what it is unless said, and what it does.
/// The props of `required` are marked so.
/// The least width of a table of props: its three fixed columns, and a description wide enough to read.
const PROPS_WIDTH: f32 = 780.0;

/// What needs `least` of width to be read, where there is `room` of it: as wide as what it is in, and in
/// less room than it needs as wide as that, scrolling sideways. An axis that scrolls has no width to fill,
/// so what is in it is given one. No room measured yet is room enough.
pub fn fit(key: &str, room: f32, least: f32, content: impl Into<Element>) -> Element {
  if room <= 0.0 || room >= least {
    return content.into();
  }
  // The bar of the scroll is drawn over the end of what scrolls: it has a strip of its own under it.
  ScrollHorizontal::new(Column::new().width(least).padding_bottom(12.0).child(content))
    .key(key)
    .width(FILL)
    .into()
}

/// The props of a component, a row for each. `room` is the width there is for the table:
/// `Frame::room`.
pub fn props_table(rows: &[(&str, &str, &str, &str)], required: &[&str], room: f32) -> Element {
  let head = |title: &str| text(title, 12.0, FontWeight::Medium).color(PaletteColor::MutedForeground);
  let mut table = Column::new().width(FILL).child(
    Row::new()
      .width(FILL)
      .height(40.0)
      .padding_horizontal(16.0)
      .align_items(Alignment::Center)
      .background(PaletteColor::Muted)
      .child(Row::new().width(150.0).child(head("Prop")))
      .child(Row::new().width(180.0).child(head("Type")))
      .child(Row::new().width(110.0).child(head("Default")))
      .child(Row::new().flex(1.0).child(head("Description"))),
  );
  for (prop, kind, default, about) in rows {
    let mut named = Row::new()
      .width(150.0)
      .spacing(8.0)
      .align_items(Alignment::Center)
      .child(name(prop, 13.0));
    if required.contains(prop) {
      named = named.child(
        Row::new()
          .height(18.0)
          .padding_horizontal(6.0)
          .align_items(Alignment::Center)
          .rounded(RadiusSize::Full)
          .background(PaletteColor::WarningMuted)
          .foreground(PaletteColor::Warning)
          .child(text("required", 11.0, FontWeight::Medium)),
      );
    }
    let mut typed = Row::new().width(180.0);
    if !kind.is_empty() {
      typed = typed.child(
        Row::new()
          .padding_horizontal(5.0)
          .padding_vertical(1.0)
          .rounded(RadiusSize::Sm)
          .background(PaletteColor::Muted)
          .child(mono(kind, 12.0)),
      );
    }
    table = table.child(
      Row::new()
        .width(FILL)
        .padding_horizontal(16.0)
        .padding_vertical(12.0)
        .align_items(Alignment::Center)
        .border_top(line())
        .child(named)
        .child(typed)
        .child(Row::new().width(110.0).child(if default.is_empty() {
          quiet("—", 13.0)
        } else {
          mono(default, 13.0)
        }))
        .child(
          styled(about, TypographyStyle::Body, |style| {
            style.font_size = 13.0;
            style.line_height = 1.5;
          })
          .color(PaletteColor::MutedForeground)
          .flex(1.0),
        ),
    );
  }
  Column::new()
    .width(FILL)
    .clip()
    .rounded(RadiusSize::Lg)
    .background(PaletteColor::Card)
    .foreground(PaletteColor::CardForeground)
    .border(line())
    // The columns keep the width they need: in a narrower card the table scrolls sideways, and a
    // description is never a few words wide.
    .child(fit("props", room, PROPS_WIDTH, table))
    .into()
}

/// A small button with no fill, for the edge of a card: Copy, Reset.
pub fn tool(label: &str, glyph: &str, press: impl Fn() + Send + Sync + 'static) -> Button {
  Button::new(label)
    .variant(ButtonVariant::Ghost)
    .size(ButtonSize::Xs)
    .icon(glyph)
    .on_click(move |_| press())
}

/// A small button that is an icon alone, for the edge of a card.
pub fn icon_tool(label: &str, glyph: &str, press: impl Fn() + Send + Sync + 'static) -> Button {
  tool(label, glyph, press).icon_only(true)
}

/// Rust as a page shows it: a line number, and each piece in the color of what it is. A name of a type is the
/// color of information, a string the color of success, punctuation is quiet.
pub fn code_block(lines: &[String]) -> Column {
  let mut block = Column::new()
    .width(FILL)
    .padding_horizontal(16.0)
    .padding_top(16.0)
    .padding_bottom(18.0)
    .spacing(4.0);
  for (index, line) in lines.iter().enumerate() {
    let mut row = Row::new().height(20.0).align_items(Alignment::Center).child(
      mono(&(index + 1).to_string(), 13.0)
        .color(PaletteColor::Input)
        .width(28.0),
    );
    for (piece, role) in pieces(line) {
      row = row.child(mono(&piece, 13.0).color(role));
    }
    block = block.child(row);
  }
  block
}

/// The pieces of a line of Rust, each with the color it is drawn in.
fn pieces(line: &str) -> Vec<(String, PaletteColor)> {
  let chars: Vec<char> = line.chars().collect();
  let mut pieces: Vec<(String, PaletteColor)> = Vec::new();
  let mut push = |piece: String, role: PaletteColor| match pieces.last_mut() {
    // What is beside a piece of its color is drawn with it, as one text.
    Some((last, held)) if *held == role => last.push_str(&piece),
    _ => pieces.push((piece, role)),
  };
  let mut at = 0;
  while at < chars.len() {
    let start = at;
    let first = chars[at];
    if first == '/' && chars.get(at + 1) == Some(&'/') {
      push(chars[at..].iter().collect(), PaletteColor::MutedForeground);
      break;
    }
    if first == '"' {
      at += 1;
      while at < chars.len() && chars[at] != '"' {
        at += 1;
      }
      at = (at + 1).min(chars.len());
      push(chars[start..at].iter().collect(), PaletteColor::Success);
    } else if first.is_alphanumeric() || first == '_' {
      while at < chars.len() && (chars[at].is_alphanumeric() || chars[at] == '_') {
        at += 1;
      }
      let word: String = chars[start..at].iter().collect();
      // A name with a capital that leads somewhere is a type or a module: `ButtonVariant::`.
      let leads = chars.get(at) == Some(&':') && chars.get(at + 1) == Some(&':');
      let role = if leads || word == "_" {
        if word == "_" {
          PaletteColor::MutedForeground
        } else {
          PaletteColor::Info
        }
      } else {
        PaletteColor::Foreground
      };
      push(word, role);
    } else if first == '.' || first == ' ' {
      at += 1;
      push(first.to_string(), PaletteColor::Foreground);
    } else {
      at += 1;
      push(first.to_string(), PaletteColor::MutedForeground);
    }
  }
  pieces
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

/// The stage of a demo card: where the live components stand. It is the card's own surface: the page
/// around the card is the canvas, and the card stands out from it whole.
pub fn stage(content: impl Into<Element>) -> Column {
  Column::new().width(FILL).padding(32.0).child(content)
}

/// The footer of a demo card: the builder call that makes what the stage shows, and a button that copies it.
pub fn card_footer(call: &str) -> Row {
  let copied = call.to_owned();
  Row::new()
    .width(FILL)
    .height(40.0)
    .padding_left(16.0)
    .padding_right(8.0)
    .align_items(Alignment::Center)
    .justify(Justify::SpaceBetween)
    .border_top(Border::inside(BorderSize::Sm, PaletteColor::Border))
    .child(mono(call, 12.0).color(PaletteColor::MutedForeground))
    .child(icon_tool("Copy", "copy", move || {
      lurq::clipboard::copy_to_clipboard(&copied);
    }))
}

/// A demo card: a stage and the call that builds what is on it.
pub fn demo_card(content: impl Into<Element>, call: &str) -> Column {
  Column::new()
    .width(FILL)
    .clip()
    .rounded(RadiusSize::Lg)
    .background(PaletteColor::Card)
    .foreground(PaletteColor::CardForeground)
    .border(Border::inside(BorderSize::Sm, PaletteColor::Border))
    .child(stage(content))
    .child(card_footer(call))
}
