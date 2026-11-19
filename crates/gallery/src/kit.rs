//! What the gallery's own interface is built from, until the design system
//! has the component for it: each of these goes when its component lands.
//!
//! Everything here takes its colors, sizes and type from the theme, so the
//! gallery is light or dark with it.

use std::sync::{
  Arc,
  atomic::{AtomicU32, Ordering},
};

use lurq::{
  app::theme::{BorderSize, Breakpoint, PaletteColor, RadiusSize, ShadowStyle, TypographyStyle},
  components::{Column, Popup, Rect, Row, ScrollHorizontal, Text, TextOverflow},
  core::{ElementRef, Signal},
  layout::{
    Alignment,
    layout_kind::Justify,
    text_style::{FontWeight, TextStyle},
  },
  node::{CursorIcon, Element, border::Border, dimension::Dimension},
  ui::{Button, ButtonSize, ButtonVariant, CodeBlock, FontFamily, Icon, SegmentedControl, SegmentedControlPart},
};

pub const FILL: Dimension = Dimension::Pct(100.0);

/// The width under which a window is as narrow as a phone: the theme's first breakpoint.
pub const PHONE: f32 = 640.0;

/// Builds what `build` makes in a function of its own: a page builds each of its demos apart, so that what one
/// builds is gone from the stack before the next. In a build without optimizations the temporaries of every builder
/// of a long render are added up in its one frame: Data Grid's was 202 KB of the main thread's 1 MB.
pub fn apart<R>(build: impl FnOnce() -> R) -> R {
  build()
}

/// Whether the window is as narrow as a phone: under the theme's first breakpoint. The gallery is the desktop
/// system there too, with less room around what it shows.
pub fn phone() -> bool {
  !lurq::ui::current_breakpoint().is_some_and(|breakpoint| breakpoint >= Breakpoint::Sm)
}

/// The room around what a card shows: 32 as the design draws it, and half of it in a window as narrow as a
/// phone, where the card has little more than that to show it in.
pub fn stage_padding() -> f32 {
  if phone() { 16.0 } else { 32.0 }
}

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

/// Text that says less: a caption, a hint.
pub fn muted(content: &str, style: TypographyStyle) -> Text {
  Text::new(content).variant(style).color(PaletteColor::MutedForeground)
}

/// An icon of the theme's set, `size` high.
pub fn icon(name: &str, size: f32) -> Icon {
  Icon::new(name).size(size)
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
) -> SegmentedControl<T> {
  SegmentedControl::controlled(
    chosen.clone(),
    options.iter().map(|(label, value)| (value.clone(), *label)),
    move |value: &T| choose(value.clone()),
  )
}

/// The room a control of a playground's panel has beside the stage: the panel's width less its padding.
pub const PANEL_ROOM: f32 = 288.0;
/// The room a control of the panel has as the playground last laid it out, as the bits of an `f32`: under the
/// stage in a narrow window it is the card's, which is less than it has beside the stage.
static PANEL_ROOM_NOW: AtomicU32 = AtomicU32::new(0);

/// Says how much room a control of the panel has: the playground does, each time it is laid out.
pub fn set_panel_room(room: f32) {
  PANEL_ROOM_NOW.store(room.to_bits(), Ordering::Relaxed);
}

/// The room a control of the panel has: what the playground said last, and the panel's own until it has.
fn panel_room() -> f32 {
  let room = f32::from_bits(PANEL_ROOM_NOW.load(Ordering::Relaxed));
  if room > 0.0 { room } else { PANEL_ROOM }
}

/// A few choices that share the width they are given, named as the code names them: a size, a shape. Where
/// their names do not fit one row of the panel they are on as many rows as they need, each choice as wide as
/// the others, so that no name is cut.
pub fn segmented_fill<T: Clone + PartialEq + Send + Sync + 'static>(
  options: &[(&str, T)],
  chosen: &T,
  choose: impl Fn(T) + Send + Sync + 'static,
) -> Element {
  let row = |options: &[(&str, T)], choose: Arc<dyn Fn(T) + Send + Sync>| {
    segmented(options, chosen, move |value| choose(value))
      .full_width(true)
      .style(SegmentedControlPart::Label, |s| {
        s.font_family(FontFamily::Mono).font_size(12.0)
      })
  };
  // A name in the family of code at 12 is 7.2 a character, in a segment padded 12 at each side.
  let longest = options
    .iter()
    .map(|(label, _)| label.chars().count())
    .max()
    .unwrap_or(0);
  let segment = longest as f32 * 7.2 + 24.0;
  let fit = ((panel_room() - 8.0) / segment).floor().max(1.0) as usize;
  let choose: Arc<dyn Fn(T) + Send + Sync> = Arc::new(choose);
  if options.len() <= fit {
    return row(options, choose).into();
  }
  // As many rows as it takes, with as many choices in each as the others have.
  let rows = options.len().div_ceil(fit);
  let each = options.len().div_ceil(rows);
  let mut column = Column::new().width(FILL).spacing(6.0);
  for chunk in options.chunks(each) {
    column = column.child(row(chunk, choose.clone()));
  }
  column.into()
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
  // A column is as wide as the longest word in it, and no narrower than the design draws it: a word does not
  // break, and one wider than its column would be drawn under the column after it. What is more than a word
  // ("invalid / warning / valid", a type with spaces in it) goes on to more lines in its column, so the
  // description keeps its room.
  let theme = lurq::ui::current_theme();
  let wide = |size: f32, longest: usize| longest as f32 * theme.scaled(size) * 0.62;
  let word = |cell: &str| {
    cell
      .split_whitespace()
      .map(|word| word.chars().count())
      .max()
      .unwrap_or(0)
  };
  let longest = |of: &dyn Fn(&(&str, &str, &str, &str)) -> usize| rows.iter().map(of).max().unwrap_or(0);
  let badge = theme.scaled(11.0) * 0.62 * 8.0 + 12.0 + 8.0;
  let prop_width = rows
    .iter()
    .map(|row| wide(13.0, word(row.0)) + if required.contains(&row.0) { badge } else { 0.0 })
    .fold(0.0, f32::max);
  let prop_width = (prop_width + 16.0).max(150.0);
  let type_width = (wide(12.0, longest(&|row| word(row.1))) + 10.0 + 16.0).max(180.0);
  let default_width = (wide(13.0, longest(&|row| word(row.2))) + 16.0).max(110.0);
  let least = PROPS_WIDTH + (prop_width - 150.0) + (type_width - 180.0) + (default_width - 110.0);
  let head = |title: &str| text(title, 12.0, FontWeight::Medium).color(PaletteColor::MutedForeground);
  let mut table = Column::new().width(FILL).child(
    Row::new()
      .width(FILL)
      .height(40.0)
      .padding_horizontal(16.0)
      .align_items(Alignment::Center)
      .background(PaletteColor::Bar)
      .child(Row::new().width(prop_width).flex_shrink(0.0).child(head("Prop")))
      .child(Row::new().width(type_width).flex_shrink(0.0).child(head("Type")))
      .child(Row::new().width(default_width).flex_shrink(0.0).child(head("Default")))
      .child(Row::new().flex(1.0).child(head("Description"))),
  );
  for (prop, kind, default, about) in rows {
    // A column in each cell gives its text the cell's width to wrap in: a row lays a text out on one line.
    let mut named = Row::new()
      .width(prop_width)
      .flex_shrink(0.0)
      .padding_right(16.0)
      .spacing(8.0)
      .align_items(Alignment::Center)
      .child(Column::new().flex(1.0).min_width(0.0).child(name(prop, 13.0)));
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
    let mut typed = Column::new()
      .width(type_width)
      .flex_shrink(0.0)
      .padding_right(16.0)
      .align_items(Alignment::Start);
    if !kind.is_empty() {
      typed = typed.child(
        Column::new()
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
        .child(
          Column::new()
            .width(default_width)
            .flex_shrink(0.0)
            .padding_right(16.0)
            .align_items(Alignment::Start)
            .child(if default.is_empty() {
              quiet("—", 13.0)
            } else {
              mono(default, 13.0)
            }),
        )
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
    .child(fit("props", room, least, table))
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

/// How high a line of code or of a log is in the gallery's cards: a little more than a code block's own.
pub const CODE_ROW: f32 = 24.0;

/// Rust as a page shows it: a code block with its lines numbered, on what holds it, with no copy button of its
/// own (the card it is in has Copy).
pub fn code_block(lines: &[String]) -> CodeBlock {
  CodeBlock::new(&lines.join(
    "
",
  ))
  .language("rust")
  .label("Code")
  .line_numbers(true)
  .header(false)
  .copy_button(false)
  .framed(false)
  .row_height(CODE_ROW)
}

/// Lines of plain text in the family of code, selected and copied as one text: a log, a tree.
pub fn plain_block(lines: &[String]) -> CodeBlock {
  CodeBlock::new(&lines.join(
    "
",
  ))
  .label("Text")
  .header(false)
  .copy_button(false)
  .framed(false)
  .row_height(CODE_ROW)
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
  Column::new().width(FILL).padding(stage_padding()).child(content)
}

/// The call under a demo card, on one line: where the card is narrower than the call, it ends in an ellipsis
/// before the tools beside it, which keep their room.
pub fn call_line(call: &str) -> Row {
  Row::new().flex(1.0).min_width(0.0).child(
    mono(call, 12.0)
      .nowrap()
      .text_overflow(TextOverflow::Elipsis)
      .color(PaletteColor::MutedForeground),
  )
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
    .spacing(12.0)
    .child(call_line(call))
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
