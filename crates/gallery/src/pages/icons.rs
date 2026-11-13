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
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, ICON_FAMILY, card, code, demo_card, icon, icons, muted},
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
    for (name, _) in icons() {
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
        &format!("{} icons, in the order of the font.", icons().len()),
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

  /// How an app draws one today.
  fn usage(&self) -> Element {
    let mut lines = card().padding(20.0).spacing(2.0);
    for line in [
      "// Once, when the app starts: the font, hinted to the pixels of the display.",
      "app.load_hinted_font(include_bytes!(\"lurq-icons.ttf\").to_vec());",
      "",
      "// An icon is a text in that family. It names no color, so it takes the foreground above it.",
      "Text::styled(\"\\u{E06F}\", TextStyle {",
      &format!("  font_family: \"{ICON_FAMILY}\".into(),"),
      "  font_size: theme.icon_size_value(IconSize::Md),",
      "  line_height: 1.0,",
      "  ..TextStyle::default()",
      "})",
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
        "In an app",
        "The font is in the crate's assets, and an icon is a text in it. The design system's Icon, which takes a \
         name and a size of the scale, is not built yet: this is what it will wrap.",
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
        import: None,
        status: None,
        since: Some("0.62"),
        source: file!(),
      },
      sections,
    )
  }
}
