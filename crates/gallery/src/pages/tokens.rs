//! The theme's tokens, drawn from the theme as it is: the palette, the type
//! scale, radii, elevations, control sizes and the other scales.

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{
      BlurSize, BorderSize, Breakpoint, ControlSize, Density, IconSize, MotionDuration, OpacityLevel, PaletteColor,
      RadiusSize, ShadowStyle, SpacingSize, ThemePalette, TypographyStyle,
    },
  },
  components::{Column, Grid, Rect, Row, Text},
  layout::Alignment,
  node::{Element, border::Border, dimension::Dimension},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, card, demo_card, muted},
};

pub struct Tokens {
  frame: Frame,
}

/// A surface of the theme with text on it in the color the theme draws on it.
fn surface(name: &str, fill: PaletteColor, ink: PaletteColor) -> Column {
  Column::new()
    .padding(12.0)
    .height(64.0)
    .spacing(2.0)
    .background(fill)
    .foreground(ink)
    .rounded(RadiusSize::Md)
    .border(Border::inside(BorderSize::Sm, PaletteColor::Border))
    .child(Text::new(name).variant(TypographyStyle::Label))
    .child(Text::new("text on it").variant(TypographyStyle::Caption))
}

/// A swatch of one role, with its name and its color in the palette the theme is in.
fn swatch(role: PaletteColor, palette: &ThemePalette) -> Column {
  let name = role.as_str().to_owned();
  let hex = palette.get(&role).to_hex();
  Column::new()
    .spacing(4.0)
    .child(
      Rect::new(FILL, 32.0)
        .background(role)
        .rounded(RadiusSize::Sm)
        .border(Border::inside(BorderSize::Sm, PaletteColor::Border)),
    )
    .child(Text::new(&name).variant(TypographyStyle::Caption))
    .child(muted(&hex, TypographyStyle::Caption))
}

/// A row of a scale: the name of a step and its value.
fn step(name: &str, value: String, sample: impl Into<Element>) -> Row {
  Row::new()
    .width(FILL)
    .height(44.0)
    .spacing(16.0)
    .align_items(Alignment::Center)
    .child(Text::new(name).variant(TypographyStyle::Mono).width(120.0))
    .child(muted(&value, TypographyStyle::Mono).width(90.0))
    .child(sample)
}

impl Tokens {
  fn palette(&self, ctx: &mut Ctx) -> Element {
    use PaletteColor::*;
    let pairs = [
      ("Background", Background, Foreground),
      ("Card", Card, CardForeground),
      ("Popover", Popover, PopoverForeground),
      ("Primary", Primary, PrimaryForeground),
      ("Secondary", Secondary, SecondaryForeground),
      ("Muted", Muted, MutedForeground),
      ("Highlight", Highlight, HighlightForeground),
      ("Destructive", Destructive, DestructiveForeground),
    ];
    // Fewer in a row where the window is narrow, so that a name fits its cell.
    let wide = ctx.breakpoint() >= Some(Breakpoint::Lg);
    let mut surfaces = Grid::with_columns(if wide { 4 } else { 2 }).gap(12.0).width(FILL);
    for (name, fill, ink) in pairs {
      surfaces = surfaces.child(surface(name, fill, ink));
    }
    let palette = ctx.theme().palette().clone();
    let mut swatches = Grid::with_columns(if wide { 6 } else { 3 }).gap(12.0).width(FILL);
    // The roles of the design: a deprecated one has the color of the role that replaces it.
    for role in ThemePalette::roles()
      .into_iter()
      .filter(|role| role.replacement().is_none())
    {
      swatches = swatches.child(swatch(role, &palette));
    }
    Column::new()
      .width(FILL)
      .spacing(16.0)
      .child(demo_card(
        surfaces,
        "PaletteColor::Card, PaletteColor::CardForeground, …",
      ))
      .child(demo_card(swatches, "theme.palette_color(PaletteColor::Primary)"))
      .into()
  }

  fn typography(&self, ctx: &mut Ctx) -> Element {
    let theme = ctx.theme().clone();
    let styles = [
      ("Heading", TypographyStyle::Heading),
      ("Title", TypographyStyle::Title),
      ("Body", TypographyStyle::Body),
      ("Description", TypographyStyle::Description),
      ("Label", TypographyStyle::Label),
      ("Button", TypographyStyle::Button),
      ("Caption", TypographyStyle::Caption),
      ("Mono", TypographyStyle::Mono),
    ];
    let mut scale = Column::new().width(FILL);
    for (name, role) in styles {
      let style = theme.typography_style(role);
      scale = scale.child(step(
        name,
        format!("{} / {}", style.font_size, style.weight.value()),
        Text::new("The quick brown fox jumps over the lazy dog").variant(role),
      ));
    }
    let families = format!(
      "Interface text is in \"{}\" and code in \"{}\": theme.set_sans_family(…) changes every style at once.",
      theme.sans_family(),
      theme.mono_family()
    );
    Column::new()
      .width(FILL)
      .spacing(12.0)
      .child(demo_card(scale, "Text::new(…).variant(TypographyStyle::Title)"))
      .child(muted(&families, TypographyStyle::Body).width(FILL))
      .into()
  }

  fn shape(&self, ctx: &mut Ctx) -> Element {
    let theme = ctx.theme().clone();
    let mut radii = Row::new().spacing(20.0).align_items(Alignment::End);
    for size in [
      RadiusSize::Sm,
      RadiusSize::Md,
      RadiusSize::Lg,
      RadiusSize::Xl,
      RadiusSize::Full,
    ] {
      radii = radii.child(
        Column::new()
          .spacing(6.0)
          .child(Rect::new(84.0, 48.0).background(PaletteColor::Primary).rounded(size))
          .child(muted(
            &format!("{} · {}", size.as_str(), theme.radius_value(size)),
            TypographyStyle::Mono,
          )),
      );
    }
    let mut shadows = Row::new().width(FILL).spacing(32.0).padding(8.0);
    for (name, style) in [
      ("sm", ShadowStyle::Sm),
      ("md", ShadowStyle::Md),
      ("lg", ShadowStyle::Lg),
    ] {
      shadows = shadows.child(
        Column::new()
          .size(120.0, 64.0)
          .padding(12.0)
          .background(PaletteColor::Card)
          .foreground(PaletteColor::CardForeground)
          .rounded(RadiusSize::Lg)
          .box_shadow(style)
          .child(Text::new(name).variant(TypographyStyle::Mono)),
      );
    }
    Column::new()
      .width(FILL)
      .spacing(16.0)
      .child(demo_card(radii, ".rounded(RadiusSize::Md)"))
      .child(demo_card(shadows, ".box_shadow(ShadowStyle::Md)"))
      .into()
  }

  fn scales(&self, ctx: &mut Ctx) -> Element {
    let theme = ctx.theme().clone();
    let bar = |width: f32| {
      Rect::new(width, 12.0)
        .background(PaletteColor::Primary)
        .rounded(RadiusSize::Sm)
    };
    let px = |dimension: Dimension| match dimension {
      Dimension::Px(px) => px,
      _ => 0.0,
    };
    let mut rows = Column::new().width(FILL);
    for size in [
      SpacingSize::Xs,
      SpacingSize::Sm,
      SpacingSize::Md,
      SpacingSize::Lg,
      SpacingSize::Xl,
      SpacingSize::Section,
    ] {
      let value = px(theme.spacing_value(size));
      rows = rows.child(step(
        &format!("spacing {}", size.as_str()),
        format!("{value}"),
        bar(value),
      ));
    }
    for size in ControlSize::all() {
      let value = theme.control_size_value(*size);
      rows = rows.child(step(
        &format!("control {}", size.as_str()),
        format!("{value}"),
        bar(value),
      ));
    }
    for size in Density::all() {
      let value = theme.density_value(*size);
      rows = rows.child(step(
        &format!("density {}", size.as_str()),
        format!("{value}"),
        bar(value),
      ));
    }
    for size in IconSize::all() {
      let value = theme.icon_size_value(*size);
      rows = rows.child(step(
        &format!("icon {}", size.as_str()),
        format!("{value}"),
        Rect::new(value, value)
          .background(PaletteColor::Primary)
          .rounded(RadiusSize::Sm),
      ));
    }
    for size in BlurSize::all() {
      let value = theme.blur_value(*size);
      rows = rows.child(step(&format!("blur {}", size.as_str()), format!("{value}"), bar(value)));
    }
    for role in MotionDuration::all() {
      let millis = theme.duration_value(*role).as_millis();
      rows = rows.child(step(
        &format!("duration {}", role.as_str()),
        format!("{millis} ms"),
        bar(millis as f32 / 2.0),
      ));
    }
    for level in OpacityLevel::all() {
      let value = theme.opacity_value(*level);
      rows = rows.child(step(
        &format!("opacity {}", level.as_str()),
        format!("{value}"),
        Rect::new(84.0, 20.0)
          .background(PaletteColor::Primary)
          .rounded(RadiusSize::Sm)
          .opacity(value),
      ));
    }
    let ring = theme.focus_ring().clone();
    rows = rows.child(step(
      "focus ring",
      format!("{} / {}", ring.width, ring.offset),
      Rect::new(84.0, 24.0)
        .background(PaletteColor::Secondary)
        .rounded(RadiusSize::Md)
        .outline(ring.outline()),
    ));
    card().padding(20.0).child(rows).into()
  }
}

impl Component for Tokens {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self { frame: Frame::new(ctx) }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let sections = vec![
      Section::new(
        "Palette",
        "A surface comes with the color of what is drawn on it. Every color here is read from the theme: switch \
         the theme in the header and the values follow.",
        self.palette(ctx),
      ),
      Section::new(
        "Typography",
        "The type scale, in the theme's two families.",
        self.typography(ctx),
      ),
      Section::new("Shape", "Radii and elevations.", self.shape(ctx)),
      Section::new(
        "Scales",
        "Spacing, control and icon sizes, densities, blur, durations, opacity and the focus ring.",
        self.scales(ctx),
      ),
    ];
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Getting started", "Tokens"],
        title: "Tokens",
        description: "The palette and the scales every component is drawn from. A theme changes one, and every \
                      component that uses it follows.",
        import: Some("use lurq::app::theme::*;"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      sections,
    )
  }
}
