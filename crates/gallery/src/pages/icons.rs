//! lurq's icon font: every icon in it, at the theme's sizes, in the color of
//! what it is in.

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{Breakpoint, IconSize, PaletteColor, RadiusSize, TypographyStyle},
  },
  components::{Column, Grid, Row},
  layout::{Alignment, layout_kind::Justify},
  node::Element,
  ui::{Icon, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, card, code, demo_card, icon, muted},
};

pub struct Icons {
  frame: Frame,
}

impl Icons {
  /// Every icon of the font with its name.
  fn set(&self) -> Element {
    let columns = match lurq::ui::current_breakpoint() {
      Some(Breakpoint::Xl) => 8,
      Some(Breakpoint::Lg) => 6,
      Some(Breakpoint::Md) => 5,
      _ => 3,
    };
    let mut grid = Grid::with_columns(columns).gap(8.0).width(FILL);
    for (name, _) in icons::ALL {
      grid = grid.child(
        Column::new()
          .width(FILL)
          .height(76.0)
          .spacing(8.0)
          .align_items(Alignment::Center)
          .justify(Justify::Center)
          .rounded(RadiusSize::Md)
          .hovered(|style| style.background(PaletteColor::Muted))
          .child(icon(name, 20.0))
          .child(muted(name, TypographyStyle::Caption)),
      );
    }
    Column::new()
      .width(FILL)
      .spacing(12.0)
      .child(card().padding(12.0).child(grid))
      .child(muted(
        &format!("{} icons, in the order of the font.", icons::ALL.len()),
        TypographyStyle::Body,
      ))
      .into()
  }

  /// One icon at each size of the theme's scale.
  fn sizes(&self, ctx: &mut Ctx) -> Element {
    let theme = ctx.theme().clone();
    let mut row = Row::new().spacing(32.0).align_items(Alignment::End);
    for size in IconSize::all() {
      let value = theme.icon_size_value(*size);
      row = row.child(
        Column::new()
          .spacing(8.0)
          .align_items(Alignment::Center)
          .child(icon("settings", value))
          .child(muted(&format!("{} · {value}", size.as_str()), TypographyStyle::Mono)),
      );
    }
    demo_card(row, "theme.icon_size_value(IconSize::Md)").into()
  }

  /// An icon names no color: it is drawn in the foreground of what it is in.
  fn color(&self) -> Element {
    let mut row = Row::new().spacing(16.0).align_items(Alignment::Center);
    for (fill, ink) in [
      (PaletteColor::Background, PaletteColor::Foreground),
      (PaletteColor::Background, PaletteColor::MutedForeground),
      (PaletteColor::Primary, PaletteColor::PrimaryForeground),
      (PaletteColor::Secondary, PaletteColor::SecondaryForeground),
      (PaletteColor::Destructive, PaletteColor::DestructiveForeground),
      (PaletteColor::SuccessMuted, PaletteColor::Success),
    ] {
      row = row.child(
        Row::new()
          .size(44.0, 44.0)
          .align_items(Alignment::Center)
          .justify(Justify::Center)
          .rounded(RadiusSize::Md)
          .background(fill)
          .foreground(ink)
          .child(icon("heart", 20.0)),
      );
    }
    demo_card(
      row,
      "Row::new().foreground(PaletteColor::PrimaryForeground).child(icon)",
    )
    .into()
  }

  /// A spinner is an icon that turns, and every one on screen turns with the others.
  fn spinning(&self) -> Element {
    let row = Row::new()
      .spacing(24.0)
      .align_items(Alignment::Center)
      .child(Icon::new(icons::SPINNER).spin(true))
      .child(Icon::new(icons::SPINNER_RAYS).spin(true).size(IconSize::Lg))
      .child(Icon::new(icons::REFRESH).spin(true).size(IconSize::Xl));
    demo_card(row, "Icon::new(icons::SPINNER).spin(true)").into()
  }

  /// How an app draws one, and how a theme changes what a name is drawn with.
  fn usage(&self) -> Element {
    let mut lines = card().padding(20.0).spacing(2.0);
    for line in [
      "use lurq::ui::{Icon, icons};",
      "",
      "Icon::new(icons::SETTINGS)                            // 16 high, in the foreground of what it is in",
      "Icon::new(icons::SETTINGS).size(IconSize::Lg)         // a step of the theme's icon sizes",
      "Icon::new(icons::CHECK).color(PaletteColor::Success)  // a color of its own",
      "",
      "// A theme draws a name with another glyph, and adds names of its own, from a font the app loads.",
      "theme.set_icon(icons::SEARCH, IconGlyph::new(\"brand icons\", '\\u{F002}'));",
      "theme.set_icon(\"rocket\", IconGlyph::new(\"brand icons\", '\\u{F135}'));",
      "Icon::new(\"rocket\")",
    ] {
      lines = lines.child(code(line).height(18.0));
    }
    lines.into()
  }
}

impl Component for Icons {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self { frame: Frame::new(ctx) }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let sections = vec![
      Section::new(
        "The set",
        "Drawn on a 24 by 24 grid with a stroke of 2, and hinted: at 16 and 20 pixels the strokes land on whole \
         pixels of the display.",
        self.set(),
      ),
      Section::new("Sizes", "The theme's scale of icon sizes.", self.sizes(ctx)),
      Section::new(
        "Color",
        "An icon is drawn in the foreground of what it is in, as text is: on a primary surface it is the color \
         the theme draws on primary.",
        self.color(),
      ),
      Section::new(
        "Spinning",
        "An icon that turns once a second. Spinners that appear at different moments turn together.",
        self.spinning(),
      ),
      Section::new(
        "In an app",
        "An icon is named, and the theme says what the name is drawn with: lurq's font, which every app has, \
         unless the theme gives the name another glyph.",
        self.usage(),
      ),
    ];
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Getting started", "Icons"],
        title: "Icons",
        description: "The icons of lurq's font, from the design system's board. This gallery draws its own with \
                      them: the title bar, the search, the side bar.",
        import: Some("use lurq::ui::{Icon, icons};"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      sections,
    )
  }
}
