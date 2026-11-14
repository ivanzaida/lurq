//! The theme by hand: the design's and the one of lurq 0.61, light and dark, with every palette role, the type
//! scale, the radii, the elevations and the other scales drawn from the theme as it is.

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{
      BlurSize, ControlSize, Density, IconSize, MotionDuration, OpacityLevel, PaletteColor, RadiusSize, ShadowStyle,
      ThemeDefinition, ThemePalette, TypographyStyle,
    },
    window::Appearance,
  },
  components::{Column, Grid, Rect, Row, Text},
  core::Signal,
  layout::Alignment,
  node::Element,
};

use crate::engine_kit::{FILL, card, chip, controls, label, note, page};

const LOOKS: [&str; 3] = ["the system's", "light", "dark"];

pub(crate) struct ThemeDemo {
  /// Whether the design's theme is applied: the demo starts in the one of lurq 0.61.
  neutral: Signal<bool>,
  /// The index into `LOOKS`.
  look: Signal<usize>,
  inter: Signal<bool>,
}

/// A surface of the theme with a line on it in the color the theme draws on it.
fn surface(name: &str, fill: PaletteColor, ink: PaletteColor) -> Column {
  Column::new()
    .padding(12.0)
    .height(64.0)
    .background(fill)
    .foreground(ink)
    .rounded(RadiusSize::Md)
    .border_inside(1.0, PaletteColor::Border)
    .child(Text::new(name).variant(TypographyStyle::Label))
    .child(Text::new("on it").variant(TypographyStyle::Caption))
}

/// A swatch of one role with its name under it.
fn swatch(role: PaletteColor) -> Column {
  let name = role.as_str().to_owned();
  Column::new()
    .spacing(4.0)
    .child(
      Rect::new(FILL, 28.0)
        .background(role)
        .rounded(RadiusSize::Sm)
        .border_inside(1.0, PaletteColor::Border),
    )
    .child(Text::new(&name).variant(TypographyStyle::Caption))
}

impl ThemeDemo {
  fn switches(&self, ctx: &mut Ctx) -> Element {
    let theme = ctx.theme().clone();
    let neutral = self.neutral.clone();
    let apply = move |design: bool| {
      let (theme, neutral) = (theme.clone(), neutral.clone());
      move || {
        theme.apply(if design {
          ThemeDefinition::neutral()
        } else {
          ThemeDefinition::legacy()
        });
        neutral.set(design);
      }
    };
    let mut looks = Row::new().spacing(6.0);
    for (index, name) in LOOKS.into_iter().enumerate() {
      let (theme, look) = (ctx.theme().clone(), self.look.clone());
      looks = looks.child(chip(name, self.look.get() == index, move || {
        theme.set_preferred_appearance(match index {
          1 => Some(Appearance::Light),
          2 => Some(Appearance::Dark),
          _ => None,
        });
        look.set(index);
      }));
    }
    let (theme, inter) = (ctx.theme().clone(), self.inter.clone());
    let system_font = ctx.theme().definition().typography.sans.clone();
    let now = ctx.theme().appearance();
    card("Which theme")
      .child(controls([
        Element::from(label("Definition")),
        chip("the design's (neutral)", self.neutral.get(), apply(true)).into(),
        chip("lurq 0.61 (legacy)", !self.neutral.get(), apply(false)).into(),
      ]))
      .child(controls([
        Element::from(label("Appearance")),
        looks.into(),
        label(match now {
          Appearance::Light => "in effect: light",
          Appearance::Dark => "in effect: dark",
        })
        .into(),
      ]))
      .child(controls([
        Element::from(label("Interface font")),
        chip("Georgia, set once for every style", self.inter.get(), move || {
          let on = !inter.get();
          inter.set(on);
          // A font every system has, to see the change without one installed.
          theme.set_sans_family(if on { "Georgia".into() } else { system_font.clone() });
        })
        .into(),
      ]))
      .child(note(
        "The whole demo follows: its pages are styled for the theme of lurq 0.61, so under the design's their text \
         is smaller and their corners rounder. The legacy theme has one palette: it is light whatever the appearance \
         says. Turn the system's dark look on and off with \"the system's\" chosen.",
      ))
      .into()
  }

  fn palette(&self) -> Element {
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
    let mut surfaces = Grid::with_columns(4).gap(10.0).width(FILL);
    for (name, fill, ink) in pairs {
      surfaces = surfaces.child(surface(name, fill, ink));
    }
    let mut swatches = Grid::with_columns(6).gap(10.0).width(FILL);
    for role in ThemePalette::roles() {
      // The roles of lurq 0.61 are shown too: under the design's theme each has the color of its replacement.
      swatches = swatches.child(swatch(role));
    }
    Column::new()
      .spacing(14.0)
      .padding(20.0)
      .width(FILL)
      .background(Canvas)
      .foreground(Foreground)
      .rounded(RadiusSize::Lg)
      .border_inside(1.0, Border)
      .child(Text::new("Palette").variant(TypographyStyle::Title))
      .child(Text::new("Surfaces, each with the color of what is drawn on it").variant(TypographyStyle::Description))
      .child(surfaces)
      .child(Text::new("Every role, the deprecated ones last").variant(TypographyStyle::Description))
      .child(swatches)
      .into()
  }

  fn scales(&self, ctx: &mut Ctx) -> Element {
    use PaletteColor::*;
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
    let mut type_scale = Column::new().spacing(6.0);
    for (name, style) in styles {
      let size = theme.typography_style(style).font_size;
      type_scale = type_scale.child(Text::new(&format!("{name}, {size} px: the quick brown fox")).variant(style));
    }
    let mut radii = Row::new().spacing(12.0).align_items(Alignment::End);
    for size in [
      RadiusSize::Sm,
      RadiusSize::Md,
      RadiusSize::Lg,
      RadiusSize::Xl,
      RadiusSize::Full,
    ] {
      radii = radii.child(
        Column::new()
          .spacing(4.0)
          .child(Rect::new(72.0, 44.0).background(Primary).rounded(size))
          .child(
            Text::new(&format!("{} {}", size.as_str(), theme.radius_value(size))).variant(TypographyStyle::Caption),
          ),
      );
    }
    // As wide as the panel: a row as wide as its cards would cut the last card's shadow at its edge.
    let mut shadows = Row::new().spacing(28.0).padding(12.0).width(FILL);
    for (name, style) in [
      ("sm", ShadowStyle::Sm),
      ("md", ShadowStyle::Md),
      ("lg", ShadowStyle::Lg),
    ] {
      shadows = shadows.child(
        Column::new()
          .size(96.0, 56.0)
          .padding(10.0)
          .background(Card)
          .foreground(CardForeground)
          .rounded(RadiusSize::Md)
          .box_shadow(style)
          .child(Text::new(name).variant(TypographyStyle::Label)),
      );
    }
    let mut controls = Row::new().spacing(10.0).align_items(Alignment::End);
    for size in [ControlSize::Sm, ControlSize::Md, ControlSize::Lg] {
      let height = theme.control_size_value(size);
      controls = controls.child(
        Row::new()
          .height(height)
          .padding_horizontal(14.0)
          .align_items(Alignment::Center)
          .background(Primary)
          .foreground(PrimaryForeground)
          .rounded(RadiusSize::Md)
          .child(Text::new(&format!("{} {height}", size.as_str())).variant(TypographyStyle::Button)),
      );
    }
    let numbers = format!(
      "Icons {:?}. Densities {:?}. Durations {:?} ms. Opacity: disabled {}, muted {}. Blur {:?}. Focus ring {} wide, {} \
       off.",
      IconSize::all()
        .iter()
        .map(|size| theme.icon_size_value(*size))
        .collect::<Vec<_>>(),
      Density::all()
        .iter()
        .map(|size| theme.density_value(*size))
        .collect::<Vec<_>>(),
      MotionDuration::all()
        .iter()
        .map(|role| theme.duration_value(*role).as_millis())
        .collect::<Vec<_>>(),
      theme.opacity_value(OpacityLevel::Disabled),
      theme.opacity_value(OpacityLevel::Muted),
      BlurSize::all()
        .iter()
        .map(|size| theme.blur_value(*size))
        .collect::<Vec<_>>(),
      theme.focus_ring().width,
      theme.focus_ring().offset,
    );
    Column::new()
      .spacing(16.0)
      .padding(20.0)
      .width(FILL)
      .background(Background)
      .foreground(Foreground)
      .rounded(RadiusSize::Lg)
      .border_inside(1.0, Border)
      .child(Text::new("Type, radii, elevations, control sizes").variant(TypographyStyle::Title))
      .child(type_scale)
      .child(radii)
      .child(shadows)
      .child(controls)
      .child(Text::new(&numbers).variant(TypographyStyle::Description).width(FILL))
      .into()
  }
}

impl Component for ThemeDemo {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      neutral: ctx.signal(false),
      look: ctx.signal(0),
      inter: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    page(
      "Theme",
      "Everything in the two panels below is drawn from the theme: switch the definition and the appearance and it \
       follows.",
    )
    .child(self.switches(ctx))
    .child(self.palette())
    .child(self.scales(ctx))
  }
}
