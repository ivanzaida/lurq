//! The page of the engine's theme: a `Theme` given a `ThemeDefinition` by hand (the design's, or lurq 0.61's), an
//! appearance and an interface font, and everything drawn from it as it is then. The theme is the stage's own: the
//! gallery around it keeps the theme it is in. Its playground chooses the definition, the appearance and the font;
//! its sections are the demo's Theme page: every role of the palette, the type scale, the radii, the elevations,
//! the control sizes and the other scales.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{
      BlurSize, Breakpoint, ControlSize, Density, IconSize, MotionDuration, OpacityLevel, PaletteColor, RadiusSize,
      ShadowStyle, Theme, ThemeDefinition, ThemePalette, TypographyStyle,
    },
    window::Appearance,
  },
  components::{Column, Grid, Rect, Row, Text},
  core::Signal,
  layout::{Alignment, layout_kind::Justify},
  node::{Element, border::Border},
};

use super::kit::named;
use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, mono, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

const DEFINITIONS: [(&str, usize); 2] = [("ThemeDefinition::neutral", 0), ("ThemeDefinition::legacy", 1)];
/// The appearance the stage's theme is in: the gallery's, whatever that is, or one of the two.
const LOOKS: [(&str, usize); 3] = [("the gallery's", 0), ("Light", 1), ("Dark", 2)];
/// The family the interface font is set to: a generic one, which every system and a browser has.
const SERIF: &str = "serif";

/// Keeps a theme of a stage in step with the page's: its appearance where `appearance` says, its text scale, bold
/// text and density. Each is set only where it differs, so that a render that calls it does not change again the
/// theme it reads.
pub(super) fn follow(stage: &Theme, page: &Theme, appearance: bool) {
  if appearance && stage.appearance() != page.appearance() {
    stage.set_preferred_appearance(Some(page.appearance()));
  }
  if stage.text_scale() != page.text_scale() {
    stage.set_text_scale(page.text_scale());
  }
  if stage.bold_text() != page.bold_text() {
    stage.set_bold_text(page.bold_text());
  }
  if stage.density() != page.density() {
    stage.set_density(page.density());
  }
}

fn definition(index: usize) -> ThemeDefinition {
  if index == 1 {
    ThemeDefinition::legacy()
  } else {
    ThemeDefinition::neutral()
  }
}

/// What the scenes of the page are drawn on: the theme's background, its text and its line, which are the stage
/// theme's where a scene is built in it.
fn panel() -> Column {
  Column::new()
    .width(FILL)
    .padding(20.0)
    .spacing(14.0)
    .rounded(RadiusSize::Lg)
    .background(PaletteColor::Background)
    .foreground(PaletteColor::Foreground)
    .border(Border::inside(1.0, PaletteColor::Border))
}

/// A button drawn from the theme alone: its control size, its radius, its type and its colors.
fn button(theme: &Theme, label: &str, fill: PaletteColor, ink: PaletteColor) -> Row {
  Row::new()
    .height(theme.control_size_value(ControlSize::Md))
    .padding_horizontal(14.0)
    .align_items(Alignment::Center)
    .rounded(RadiusSize::Md)
    .background(fill)
    .foreground(ink)
    .child(Text::new(label).variant(TypographyStyle::Button))
}

/// What the playground's stage shows: a card of an app drawn from the theme alone.
fn preview(theme: &Theme) -> Column {
  panel()
    .width(380.0)
    .max_width(FILL)
    .child(Text::new("Notifications").variant(TypographyStyle::Title))
    .child(
      Text::new("Choose what reaches you and when. Every color, size and corner here is the theme's.")
        .variant(TypographyStyle::Body)
        .width(FILL),
    )
    .child(
      Column::new()
        .width(FILL)
        .padding(12.0)
        .spacing(4.0)
        .rounded(RadiusSize::Md)
        .background(PaletteColor::Card)
        .foreground(PaletteColor::CardForeground)
        .border(Border::inside(1.0, PaletteColor::Border))
        .box_shadow(ShadowStyle::Sm)
        .child(Text::new("Weekly summary").variant(TypographyStyle::Label))
        .child(
          Text::new("Sent on Mondays")
            .variant(TypographyStyle::Caption)
            .color(PaletteColor::MutedForeground),
        ),
    )
    .child(
      Row::new()
        .width(FILL)
        .spacing(8.0)
        .wrap()
        .justify(Justify::End)
        .child(button(
          theme,
          "Cancel",
          PaletteColor::Secondary,
          PaletteColor::SecondaryForeground,
        ))
        .child(button(
          theme,
          "Save",
          PaletteColor::Primary,
          PaletteColor::PrimaryForeground,
        )),
    )
}

/// A surface of the theme with a line on it in the color the theme draws on it.
fn surface(name: &str, fill: PaletteColor, ink: PaletteColor) -> Column {
  Column::new()
    .padding(12.0)
    .height(64.0)
    .spacing(2.0)
    .background(fill)
    .foreground(ink)
    .rounded(RadiusSize::Md)
    .border(Border::inside(1.0, PaletteColor::Border))
    .child(Text::new(name).variant(TypographyStyle::Label))
    .child(Text::new("on it").variant(TypographyStyle::Caption))
}

/// A swatch of one role with its name under it, and for a role of lurq 0.61 the role that replaces it.
fn swatch(role: PaletteColor) -> Column {
  // A name breaks after an underscore where it does not fit its swatch, not inside a word.
  let name = role.as_str().replace('_', "_\u{200B}");
  Column::new()
    .spacing(4.0)
    .child(
      Rect::new(FILL, 28.0)
        .background(role.clone())
        .rounded(RadiusSize::Sm)
        .border(Border::inside(1.0, PaletteColor::Border)),
    )
    .child(Text::new(&name).variant(TypographyStyle::Caption))
}

/// The surfaces with their foregrounds, and every role of the palette.
fn palette(wide: bool) -> Column {
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
  let mut surfaces = Grid::with_columns(if wide { 4 } else { 2 }).gap(10.0).width(FILL);
  for (name, fill, ink) in pairs {
    surfaces = surfaces.child(surface(name, fill, ink));
  }
  let mut swatches = Grid::with_columns(if wide { 6 } else { 3 }).gap(10.0).width(FILL);
  for role in ThemePalette::roles() {
    swatches = swatches.child(swatch(role));
  }
  panel()
    .child(Text::new("Surfaces, each with the color of what is drawn on it").variant(TypographyStyle::Description))
    .child(surfaces)
    .child(Text::new("Every role").variant(TypographyStyle::Description))
    .child(swatches)
}

/// The theme's typography styles, each with its size.
fn type_scale(theme: &Theme) -> Column {
  let mut scale = panel();
  for (name, style) in [
    ("Heading", TypographyStyle::Heading),
    ("Title", TypographyStyle::Title),
    ("Body", TypographyStyle::Body),
    ("Description", TypographyStyle::Description),
    ("Label", TypographyStyle::Label),
    ("Button", TypographyStyle::Button),
    ("Caption", TypographyStyle::Caption),
    ("Mono", TypographyStyle::Mono),
  ] {
    let size = theme.typography_style(style).font_size;
    scale = scale.child(
      Text::new(&format!("{name}, {size} px: the quick brown fox"))
        .variant(style)
        .width(FILL),
    );
  }
  scale
}

/// The radii, the elevations and the control sizes, each drawn and with its value.
fn shapes(theme: &Theme) -> Column {
  let mut radii = Row::new().width(FILL).spacing(12.0).wrap().align_items(Alignment::End);
  for size in [
    RadiusSize::Sm,
    RadiusSize::Md,
    RadiusSize::Lg,
    RadiusSize::Xl,
    RadiusSize::Full,
  ] {
    let value = theme.radius_value(size.clone());
    radii = radii.child(
      Column::new()
        .spacing(4.0)
        .child(
          Rect::new(72.0, 44.0)
            .background(PaletteColor::Primary)
            .rounded(size.clone()),
        )
        .child(Text::new(&format!("{} {value}", size.as_str())).variant(TypographyStyle::Caption)),
    );
  }
  // Room round the cards: a shadow paints past them.
  let mut shadows = Row::new().width(FILL).spacing(28.0).padding(12.0).wrap();
  for (name, style) in [
    ("sm", ShadowStyle::Sm),
    ("md", ShadowStyle::Md),
    ("lg", ShadowStyle::Lg),
  ] {
    shadows = shadows.child(
      Column::new()
        .size(96.0, 56.0)
        .padding(10.0)
        .background(PaletteColor::Card)
        .foreground(PaletteColor::CardForeground)
        .rounded(RadiusSize::Md)
        .box_shadow(style)
        .child(Text::new(name).variant(TypographyStyle::Label)),
    );
  }
  let mut controls = Row::new().width(FILL).spacing(10.0).wrap().align_items(Alignment::End);
  for size in [ControlSize::Sm, ControlSize::Md, ControlSize::Lg] {
    let height = theme.control_size_value(size);
    controls = controls.child(
      Row::new()
        .height(height)
        .padding_horizontal(14.0)
        .align_items(Alignment::Center)
        .background(PaletteColor::Primary)
        .foreground(PaletteColor::PrimaryForeground)
        .rounded(RadiusSize::Md)
        .child(Text::new(&format!("{} {height}", size.as_str())).variant(TypographyStyle::Button)),
    );
  }
  panel()
    .child(Text::new("Radii").variant(TypographyStyle::Description))
    .child(radii)
    .child(Text::new("Elevations").variant(TypographyStyle::Description))
    .child(shadows)
    .child(Text::new("Control sizes").variant(TypographyStyle::Description))
    .child(controls)
}

/// The scales that are numbers: icons, densities, durations, opacities, blurs, the focus ring, the families.
fn numbers(theme: &Theme) -> Column {
  let list = |values: Vec<String>| values.join(", ");
  let line = |name: &str, value: String| named(name, Text::new(&value).variant(TypographyStyle::Mono).width(FILL));
  let ring = theme.focus_ring().clone();
  panel()
    .child(line(
      "icons",
      list(
        IconSize::all()
          .iter()
          .map(|size| theme.icon_size_value(*size).to_string())
          .collect(),
      ),
    ))
    .child(line(
      "densities",
      list(
        Density::all()
          .iter()
          .map(|size| theme.density_value(*size).to_string())
          .collect(),
      ),
    ))
    .child(line(
      "durations",
      list(
        MotionDuration::all()
          .iter()
          .map(|role| format!("{} ms", theme.duration_value(*role).as_millis()))
          .collect(),
      ),
    ))
    .child(line(
      "opacity",
      format!(
        "disabled {}, muted {}",
        theme.opacity_value(OpacityLevel::Disabled),
        theme.opacity_value(OpacityLevel::Muted)
      ),
    ))
    .child(line(
      "blur",
      list(
        BlurSize::all()
          .iter()
          .map(|size| theme.blur_value(*size).to_string())
          .collect(),
      ),
    ))
    .child(line("focus ring", format!("{} wide, {} off", ring.width, ring.offset)))
    .child(line(
      "families",
      format!("{}; code in {}", theme.sans_family(), theme.mono_family()),
    ))
}

pub struct ThemePage {
  frame: Frame,
  playground: Playground,
  /// The theme of the stage and of every scene of the page: what the controls change, and nothing else.
  stage: Theme,
  /// An index into [`DEFINITIONS`].
  definition: Signal<usize>,
  /// An index into [`LOOKS`].
  look: Signal<usize>,
  serif: Signal<bool>,
  /// The definition last applied to the stage's theme, so that it is applied once each time it is chosen.
  applied: Mutex<usize>,
  /// The interface family of each definition, which the font goes back to.
  families: [Arc<str>; 2],
}

impl ThemePage {
  /// Puts the stage's theme as the controls say, and in step with the page's in all they do not say. Only what
  /// differs is set: the render that follows a change finds the theme as it was left.
  fn sync(&self, page: &Theme) {
    let chosen = self.definition.get().min(1);
    {
      let mut applied = self.applied.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
      if *applied != chosen {
        self.stage.apply(definition(chosen));
        *applied = chosen;
      }
    }
    let family: Arc<str> = if self.serif.get() {
      SERIF.into()
    } else {
      self.families[chosen].clone()
    };
    if self.stage.sans_family() != family {
      self.stage.set_sans_family(family);
    }
    let look = match self.look.get() {
      1 => Appearance::Light,
      2 => Appearance::Dark,
      _ => page.appearance(),
    };
    if self.stage.appearance() != look {
      self.stage.set_preferred_appearance(Some(look));
    }
    follow(&self.stage, page, false);
  }

  fn play(&self, ctx: &mut Ctx) -> Element {
    let (chosen, look, serif) = (self.definition.get().min(1), self.look.get().min(2), self.serif.get());
    let stage = self.stage.clone();
    let build = {
      let stage = stage.clone();
      move || -> Element { lurq::ui::in_theme(&stage, || preview(&stage).theme(&stage).into()) }
    };
    let shown = {
      let stage = stage.clone();
      Shown::built(move |ctx| ctx.in_theme(&stage, |ctx| preview(&ctx.theme().clone())))
    };

    let mut code = vec![if chosen == 1 {
      "let theme = Theme::legacy();".to_owned()
    } else {
      "let theme = Theme::new();".to_owned()
    }];
    match look {
      1 => code.push("theme.set_preferred_appearance(Some(Appearance::Light));".to_owned()),
      2 => code.push("theme.set_preferred_appearance(Some(Appearance::Dark));".to_owned()),
      _ => {}
    }
    if serif {
      code.push(format!("theme.set_sans_family(\"{SERIF}\");"));
    }
    code.push("ctx.in_theme(&theme, |ctx| card(ctx))".to_owned());
    let in_effect = match stage.appearance() {
      Appearance::Light => "light",
      Appearance::Dark => "dark",
    };
    let named = if chosen == 1 { "legacy" } else { "neutral" };
    let tree = vec![
      format!("column   {}", self.playground.measured()),
      format!("│  theme: {named}, {in_effect}, {}", stage.sans_family()),
      "├─ text   Title".to_owned(),
      "├─ text   Body".to_owned(),
      "├─ column card · ShadowStyle::Sm".to_owned(),
      "└─ row    buttons · ControlSize::Md".to_owned(),
    ];

    let (set_definition, set_look) = (self.definition.clone(), self.look.clone());
    let reset = {
      let (definition, look, serif) = (self.definition.clone(), self.look.clone(), self.serif.clone());
      Arc::new(move || {
        definition.set(0);
        look.set(0);
        serif.set(false);
      })
    };
    let controls = vec![
      prop(
        "apply",
        "ThemeDefinition",
        self
          .playground
          .select(0, &DEFINITIONS, &chosen, move |value| set_definition.set(value)),
      ),
      prop(
        "set_preferred_appearance",
        "Option<Appearance>",
        self
          .playground
          .select(1, &LOOKS, &look, move |value| set_look.set(value)),
      ),
      divider(),
      flag("set_sans_family(\"serif\")", &self.serif),
    ];
    self.playground.card(
      ctx,
      Spec {
        title: "Theme",
        make: Arc::new(build),
        shown,
        fill: true,
        forceable: false,
        facts: format!("{named} · {in_effect}"),
        event: "",
        code,
        tree,
        controls,
        changed: [chosen != 0, look != 0, serif]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset,
      },
    )
  }
}

impl Component for ThemePage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 2),
      stage: Theme::new(),
      definition: ctx.signal(0),
      look: ctx.signal(0),
      serif: ctx.signal(false),
      applied: Mutex::new(0),
      families: [
        ThemeDefinition::neutral().typography.sans,
        ThemeDefinition::legacy().typography.sans,
      ],
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let page = ctx.theme().clone();
    self.sync(&page);
    let wide = ctx.breakpoint() >= Some(Breakpoint::Lg);
    let stage = self.stage.clone();
    let playground = self.play(ctx);
    let palette = ctx.in_theme(&stage, |_| palette(wide));
    let type_scale = ctx.in_theme(&stage, |ctx| type_scale(&ctx.theme().clone()));
    let shapes = ctx.in_theme(&stage, |ctx| shapes(&ctx.theme().clone()));
    let numbers = ctx.in_theme(&stage, |ctx| numbers(&ctx.theme().clone()));
    let rows = [
      (
        "Theme::new",
        "",
        "",
        "The design's theme, light and dark, following the system: ThemeDefinition::neutral. Theme::legacy is \
         lurq 0.61's.",
      ),
      (
        "apply",
        "ThemeDefinition",
        "",
        "Gives the theme the look of a definition, all at once. What a person or the system set stays: the \
         appearance, the text scale, bold text, reduced motion.",
      ),
      (
        "set_preferred_appearance",
        "Option<Appearance>",
        "None",
        "Light or dark whatever the system is in; None takes the system's. The legacy theme has one palette: it \
         is light whatever this says.",
      ),
      (
        "set_sans_family",
        "impl Into<Arc<str>>",
        "",
        "The family of every typography style that was in the interface's: a family, an alias registered with \
         the app, or a generic family.",
      ),
      (
        "set_palette_color",
        "(PaletteColor, Color)",
        "",
        "The color of one role in every appearance: a brand color set once.",
      ),
      (
        "in_theme",
        "(&Theme, build)",
        "",
        "On Ctx: builds an element in a theme of its own, which it is laid out and painted in. How this page \
         keeps the gallery in its theme.",
      ),
    ];
    // What the system says with no window, and the look the gallery's root was created in.
    let system = {
      // The name over the value: a line of the name beside it does not fit a window as narrow as a phone.
      let says = |name: &str, value: String, id: &str| {
        Column::new()
          .width(FILL)
          .spacing(2.0)
          .child(quiet(name, 12.0).width(FILL))
          .child(mono(&value, 12.0).id(id))
      };
      let look = |appearance: Option<Appearance>| match appearance {
        Some(Appearance::Light) => "Light",
        Some(Appearance::Dark) => "Dark",
        None => "not said",
      };
      let created = crate::created_in();
      Column::new()
        .width(FILL)
        .spacing(10.0)
        .child(says(
          "the system, asked now with no window",
          look(lurq::app::system_appearance()).to_owned(),
          "theme-system-asked",
        ))
        .child(says(
          "the theme the gallery's root was created in",
          look(created.map(|(theme, _)| theme)).to_owned(),
          "theme-root-created-in",
        ))
        .child(says(
          "what its window said of the system then",
          look(created.map(|(_, window)| window)).to_owned(),
          "theme-root-window-said",
        ))
        .child(says(
          "what the window says now",
          look(Some(ctx.window().info().system.appearance)).to_owned(),
          "theme-window-says",
        ))
    };
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Engine", "Theme"],
        title: "Theme",
        description: "The engine's theme by hand: a Theme given a ThemeDefinition, the design's or lurq 0.61's, an \
                      appearance and an interface font. Everything on this page below the header is drawn from it; \
                      the theme is the page's own, and the gallery around it keeps its own.",
        import: Some("use lurq::app::theme::{Theme, ThemeDefinition};"),
        status: Some("Stable"),
        since: None,
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Apply the design's definition or lurq 0.61's, choose light or dark, and set the interface font once for \
           every style: the card, and every scene under it, follow. The legacy definition has one palette: it is \
           light whatever the appearance says.",
          playground,
        ),
        Section::new(
          "Palette",
          "The surfaces, each with the color of what is drawn on it, and every role of the palette. The roles of \
           lurq 0.61 are last, each with the role that replaces it: under the design's definition each has its \
           replacement's color.",
          self.frame.demo(
            0,
            palette,
            "theme.palette_color(PaletteColor::Primary)",
            &[
              "Column::new()",
              "    .background(PaletteColor::Card)",
              "    .foreground(PaletteColor::CardForeground)",
              "    .child(Text::new(\"Card\").variant(TypographyStyle::Label))",
              "",
              "for role in ThemePalette::roles() {",
              "    Rect::new(Dimension::Pct(100.0), 28.0).background(role);",
              "}",
            ],
          ),
        ),
        Section::new(
          "Type scale",
          "The typography styles of the theme, each at its size. The interface font set in the playground changes \
           all of them but Mono at once.",
          self.frame.demo(
            1,
            type_scale,
            "Text::new(..).variant(TypographyStyle::Title)",
            &[
              "let size = theme.typography_style(TypographyStyle::Title).font_size;",
              "Text::new(\"The quick brown fox\").variant(TypographyStyle::Title)",
            ],
          ),
        ),
        Section::new(
          "Radii, elevations, control sizes",
          "The theme's radii, its three shadows and the heights of its controls, each drawn from it and with its \
           value: the two definitions differ in all of them.",
          self.frame.demo(
            2,
            shapes,
            ".rounded(RadiusSize::Md).box_shadow(ShadowStyle::Md)",
            &[
              "Rect::new(72.0, 44.0).rounded(RadiusSize::Lg)",
              "Column::new().box_shadow(ShadowStyle::Md)",
              "Row::new().height(theme.control_size_value(ControlSize::Md))",
            ],
          ),
        ),
        Section::new(
          "Other scales",
          "What the theme says in numbers: the sizes of icons, the densities, the durations of motion, the \
           opacities, the blurs, the focus ring, and the families of text.",
          self.frame.demo(
            3,
            numbers,
            "theme.duration_value(MotionDuration::Base)",
            &[
              "theme.icon_size_value(IconSize::Md)",
              "theme.density_value(Density::Default)",
              "theme.duration_value(MotionDuration::Base)",
              "theme.opacity_value(OpacityLevel::Disabled)",
              "theme.blur_value(BlurSize::Thin)",
              "theme.focus_ring().width",
            ],
          ),
        ),
        Section::new(
          "The system's look, before the window",
          "A shell tells the theme whether the system is light or dark once it has made its window, and the root            is mounted before that: on a dark system it was created in the light palette. The gallery asks the            system first, with no window, so its root is created in the look the window will have.",
          self.frame.demo(
            4,
            system,
            "app.read_system_appearance()",
            &[
              "let mut app = App::new();",
              "// Before the root is mounted: the theme is in the system's look from the root's create.",
              "app.read_system_appearance();",
              "tree.mount_root::<Root>(&mut app, props);",
              "",
              "// The reading alone: Some(Appearance::Dark), or None where the system cannot be asked.",
              "let look = lurq::app::system_appearance();",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a theme is made from and how it is changed by hand.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
