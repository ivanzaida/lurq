//! The page of what a box is painted with beside its layout: its fill, its corners, its shadows, a blur of itself or
//! of what is behind it, whether it cuts off what runs past it, and how opaque it is. Its playground paints one box
//! as its props say; its sections are the demo's Visual page without its pictures and its SVG, which have pages
//! of their own.

use std::sync::Arc;

use lurq::{
  animation::Transition,
  app::{
    component::Component,
    ctx::Ctx,
    theme::{PaletteColor, RadiusSize, ShadowStyle},
  },
  components::{Column, Rect, Row, Stack},
  core::Signal,
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{BoxShadow, CursorIcon, Element, RadiusValue, border::Border, transform::Transform2D},
};

use super::kit::{sample, samples, tint};
use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, fit, props_table, text},
  playground::{Playground, Shown, Spec, prop},
};

/// The fills the playground offers, each with the color the theme draws on it.
const FILLS: [(&str, PaletteColor, PaletteColor); 6] = [
  ("Primary", PaletteColor::Primary, PaletteColor::PrimaryForeground),
  ("Secondary", PaletteColor::Secondary, PaletteColor::SecondaryForeground),
  ("Card", PaletteColor::Card, PaletteColor::CardForeground),
  ("Muted", PaletteColor::Muted, PaletteColor::MutedForeground),
  (
    "Destructive",
    PaletteColor::Destructive,
    PaletteColor::DestructiveForeground,
  ),
  ("Chart2", PaletteColor::Chart2, PaletteColor::Background),
];
const FILL_CHOICES: [(&str, usize); 6] = [
  ("Primary", 0),
  ("Secondary", 1),
  ("Card", 2),
  ("Muted", 3),
  ("Destructive", 4),
  ("Chart2", 5),
];
/// The radii the playground offers: pixels, and the theme's Full, which is as round as the box allows.
const RADII: [(&str, usize); 5] = [("0", 0), ("8", 1), ("16", 2), ("40", 3), ("RadiusSize::Full", 4)];
const SHADOWS: [(&str, usize); 8] = [
  ("none", 0),
  ("ShadowStyle::Sm", 1),
  ("ShadowStyle::Md", 2),
  ("ShadowStyle::Lg", 3),
  ("layered", 4),
  ("hard offset", 5),
  ("glow", 6),
  ("inset", 7),
];
const BLURS: [(&str, i32); 4] = [("0", 0), ("2", 2), ("6", 6), ("16", 16)];
/// Opacities in tenths, which a signal keeps as a whole number.
const OPACITIES: [(&str, i32); 5] = [("1.0", 10), ("0.8", 8), ("0.6", 6), ("0.4", 4), ("0.2", 2)];
/// How large the playground's box is.
const BOX: (f32, f32) = (160.0, 96.0);

fn radius(index: usize) -> RadiusValue {
  match index {
    0 => 0.0.into(),
    1 => 8.0.into(),
    2 => 16.0.into(),
    3 => 40.0.into(),
    _ => RadiusSize::Full.into(),
  }
}

fn radius_code(index: usize) -> &'static str {
  match index {
    0 => "0.0",
    1 => "8.0",
    2 => "16.0",
    3 => "40.0",
    _ => "RadiusSize::Full",
  }
}

/// A shadow under two others' of the same color: a tight one at the edge and a wide one that a negative spread
/// keeps under the box.
fn layered() -> Vec<BoxShadow> {
  vec![
    BoxShadow::new(0.0, 1.0, 1.0, PaletteColor::Scrim),
    BoxShadow::new(0.0, 8.0, 24.0, PaletteColor::Scrim).spread(-4.0),
  ]
}

/// The box with the `index`th of [`SHADOWS`].
fn shadowed(shown: Column, index: usize) -> Column {
  match index {
    1 => shown.box_shadow(ShadowStyle::Sm),
    2 => shown.box_shadow(ShadowStyle::Md),
    3 => shown.box_shadow(ShadowStyle::Lg),
    4 => shown.box_shadow(layered()),
    5 => shown.box_shadow(BoxShadow::new(6.0, 6.0, 0.0, PaletteColor::Foreground)),
    6 => shown.box_shadow(BoxShadow::new(0.0, 0.0, 16.0, PaletteColor::Primary).spread(2.0)),
    7 => shown.box_shadow(BoxShadow::new(0.0, 2.0, 8.0, PaletteColor::Scrim).inset()),
    _ => shown,
  }
}

/// The Rust of the `index`th of [`SHADOWS`], on lines of a builder.
fn shadow_code(index: usize) -> Vec<String> {
  let line = match index {
    1 => "ShadowStyle::Sm",
    2 => "ShadowStyle::Md",
    3 => "ShadowStyle::Lg",
    4 => {
      return vec![
        "    .box_shadow(vec![".to_owned(),
        "        BoxShadow::new(0.0, 1.0, 1.0, PaletteColor::Scrim),".to_owned(),
        "        BoxShadow::new(0.0, 8.0, 24.0, PaletteColor::Scrim).spread(-4.0),".to_owned(),
        "    ])".to_owned(),
      ];
    }
    5 => "BoxShadow::new(6.0, 6.0, 0.0, PaletteColor::Foreground)",
    6 => "BoxShadow::new(0.0, 0.0, 16.0, PaletteColor::Primary).spread(2.0)",
    7 => "BoxShadow::new(0.0, 2.0, 8.0, PaletteColor::Scrim).inset()",
    _ => return Vec::new(),
  };
  vec![format!("    .box_shadow({line})")]
}

pub struct VisualPage {
  frame: Frame,
  playground: Playground,
  /// An index into [`FILLS`].
  fill: Signal<usize>,
  /// An index into [`RADII`].
  radius: Signal<usize>,
  /// An index into [`SHADOWS`].
  shadow: Signal<usize>,
  blur: Signal<i32>,
  /// In tenths.
  opacity: Signal<i32>,
}

impl VisualPage {
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (fill, corner, shadow) = (
      self.fill.get().min(FILLS.len() - 1),
      self.radius.get().min(RADII.len() - 1),
      self.shadow.get().min(SHADOWS.len() - 1),
    );
    let (blur, opacity) = (self.blur.get(), self.opacity.get());
    let (fill_name, fill_role, ink) = FILLS[fill].clone();
    let build = {
      let (fill_role, ink) = (fill_role.clone(), ink.clone());
      move || -> Element {
        let shown = Column::new()
          .size(BOX.0, BOX.1)
          .padding(12.0)
          .justify(Justify::End)
          .background(fill_role.clone())
          .foreground(ink.clone())
          .rounded(radius(corner))
          .child(text("A box", 14.0, FontWeight::SemiBold));
        let mut shown = shadowed(shown, shadow);
        if blur > 0 {
          shown = shown.blur(blur as f32);
        }
        if opacity < 10 {
          shown = shown.opacity(opacity as f32 / 10.0);
        }
        shown.into()
      }
    };

    let mut code = vec![
      "Column::new()".to_owned(),
      format!("    .size({}.0, {}.0)", BOX.0, BOX.1),
      format!("    .background(PaletteColor::{fill_name})"),
      format!("    .foreground(PaletteColor::{fill_name}Foreground)"),
      format!("    .rounded({})", radius_code(corner)),
    ];
    if fill_name == "Chart2" {
      code[3] = "    .foreground(PaletteColor::Background)".to_owned();
    }
    code.extend(shadow_code(shadow));
    if blur > 0 {
      code.push(format!("    .blur({blur}.0)"));
    }
    if opacity < 10 {
      code.push(format!("    .opacity({:?})", opacity as f32 / 10.0));
    }
    code.push("    .child(Text::new(\"A box\"))".to_owned());
    let tree = vec![
      format!("column   {}", self.playground.measured()),
      "└─ text \"A box\"".to_owned(),
    ];

    let (set_fill, set_radius, set_shadow, set_blur, set_opacity) = (
      self.fill.clone(),
      self.radius.clone(),
      self.shadow.clone(),
      self.blur.clone(),
      self.opacity.clone(),
    );
    let reset = {
      let (fill, radius, shadow, blur, opacity) = (
        self.fill.clone(),
        self.radius.clone(),
        self.shadow.clone(),
        self.blur.clone(),
        self.opacity.clone(),
      );
      Arc::new(move || {
        fill.set(2);
        radius.set(2);
        shadow.set(2);
        blur.set(0);
        opacity.set(10);
      })
    };
    let controls = vec![
      prop(
        "background",
        "impl Into<BackgroundColor>",
        self
          .playground
          .select(0, &FILL_CHOICES, &fill, move |value| set_fill.set(value)),
      ),
      prop(
        "rounded",
        "impl Into<RadiusValue>",
        self
          .playground
          .select(1, &RADII, &corner, move |value| set_radius.set(value)),
      ),
      prop(
        "box_shadow",
        "impl Into<BoxShadowValue>",
        self
          .playground
          .select(2, &SHADOWS, &shadow, move |value| set_shadow.set(value)),
      ),
      prop(
        "blur",
        "f32",
        self
          .playground
          .select(3, &BLURS, &blur, move |value| set_blur.set(value)),
      ),
      prop(
        "opacity",
        "f32",
        self
          .playground
          .select(4, &OPACITIES, &opacity, move |value| set_opacity.set(value)),
      ),
    ];
    self.playground.card(
      ctx,
      Spec {
        title: "Visual",
        make: Arc::new(build.clone()),
        shown: Shown::built(move |_| build()),
        fill: false,
        forceable: false,
        facts: format!("radius {}", RADII[corner].0),
        event: "",
        code,
        tree,
        controls,
        changed: [fill != 2, corner != 2, shadow != 2, blur != 0, opacity != 10]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset,
      },
    )
  }
}

/// A swatch of a role, with its name on it in the color the theme draws on it.
fn swatch(role: PaletteColor, ink: PaletteColor) -> Column {
  let name = role.as_str().to_owned();
  Column::new()
    .width(110.0)
    .max_width(FILL)
    .height(64.0)
    .padding(10.0)
    .justify(Justify::End)
    .rounded(RadiusSize::Md)
    .background(role)
    .foreground(ink)
    .child(text(&name, 12.0, FontWeight::SemiBold))
}

/// The roles of the theme that fill something, each in the color of its role: right in light and in dark.
fn colors() -> Row {
  use PaletteColor::*;
  let mut all = samples().spacing(12.0);
  for (role, ink) in [
    (Primary, PrimaryForeground),
    (Secondary, SecondaryForeground),
    (Muted, MutedForeground),
    (Highlight, HighlightForeground),
    (Destructive, DestructiveForeground),
    (Success, Background),
    (Warning, Background),
    (Info, Background),
    (Chart1, Background),
    (Chart2, Background),
    (Chart3, Background),
    (Chart4, Background),
    (Chart5, Background),
  ] {
    all = all.child(swatch(role, ink));
  }
  all
}

/// The same bar at four radii: square, a little rounded, rounded, a pill.
fn radii() -> Row {
  let mut all = samples();
  for (caption, corner) in [
    ("rounded(0.0): square", 0usize),
    ("rounded(8.0)", 1),
    ("rounded(16.0)", 2),
    ("rounded(40.0): a pill", 3),
  ] {
    all = all.child(sample(
      Rect::new(140.0, 50.0)
        .max_width(FILL)
        .background(PaletteColor::Primary)
        .rounded(radius(corner)),
      caption,
    ));
  }
  all
}

/// A card that casts the `index`th of [`SHADOWS`], with its name under it. What holds it lets the shadow out.
fn shadow_sample(caption: &str, index: usize) -> Column {
  let card = Column::new()
    .size(120.0, 64.0)
    .background(PaletteColor::Card)
    .rounded(10.0);
  sample(shadowed(card, index), caption).overflow_visible()
}

/// The theme's three elevations and a card that rises under the pointer; a layered shadow, a hard one, a glow and
/// a well, on a fill they show on.
fn shadows() -> Column {
  let rises = Column::new()
    .size(120.0, 64.0)
    .background(PaletteColor::Card)
    .rounded(10.0)
    .cursor(CursorIcon::Pointer)
    .box_shadow(ShadowStyle::Sm)
    .transition(Transition::box_shadow().duration_ms(200))
    .hovered(|style| style.box_shadow(ShadowStyle::Lg));
  Column::new()
    .width(FILL)
    .padding(32.0)
    .spacing(32.0)
    .rounded(RadiusSize::Md)
    .background(PaletteColor::Muted)
    .child(
      samples()
        .overflow_visible()
        .child(shadow_sample("ShadowStyle::Sm", 1))
        .child(shadow_sample("ShadowStyle::Md", 2))
        .child(shadow_sample("ShadowStyle::Lg", 3))
        .child(sample(rises, "hover: it rises").overflow_visible()),
    )
    .child(
      samples()
        .overflow_visible()
        .child(shadow_sample("layered", 4))
        .child(shadow_sample("hard offset", 5))
        .child(shadow_sample("glow", 6))
        .child(shadow_sample("inset", 7)),
    )
}

/// The same card blurred as one layer by growing radii, and once faded too.
fn blurred() -> Row {
  let card = |caption: &str, radius: f32, opacity: f32| {
    sample(
      Column::new()
        .size(120.0, 64.0)
        .padding(10.0)
        .spacing(6.0)
        .background(PaletteColor::Card)
        .foreground(PaletteColor::CardForeground)
        .border(Border::inside(1.0, PaletteColor::Border))
        .rounded(10.0)
        .child(text("Blurred", 14.0, FontWeight::Bold))
        .child(Rect::new(64.0, 10.0).background(PaletteColor::Primary).rounded(5.0))
        .opacity(opacity)
        .blur(radius),
      caption,
    )
    .overflow_visible()
  };
  samples()
    .overflow_visible()
    .child(card("blur(0.0)", 0.0, 1.0))
    .child(card("blur(2.0)", 2.0, 1.0))
    .child(card("blur(6.0)", 6.0, 1.0))
    .child(card("blur(16.0)", 16.0, 1.0))
    .child(card("blur(6.0).opacity(0.5)", 6.0, 0.5))
}

/// How wide the glass scene is: it cannot be narrower, and scrolls sideways in a card that is.
const GLASS: f32 = 480.0;

/// Frosted glass: panels that blur the stripes and the words behind them by growing radii, and one turned.
fn glass() -> Stack {
  let stripes = Row::new().with_children((0..12).map(|index| Rect::new(40.0, 96.0).background(tint(index % 2))));
  let pane = |caption: &str, radius: f32| {
    Column::new()
      .size(96.0, 64.0)
      .padding(10.0)
      .background(PaletteColor::GlassThin)
      .foreground(PaletteColor::Foreground)
      .border(Border::inside(1.0, PaletteColor::GlassRegular))
      .rounded(12.0)
      .backdrop_blur(radius)
      .child(text(caption, 12.0, FontWeight::Bold))
  };
  Stack::new()
    .size(GLASS, 96.0)
    .rounded(RadiusSize::Md)
    .clip()
    .child(stripes)
    .child(
      Column::new()
        .padding(16.0)
        .child(text("Behind the glass", 22.0, FontWeight::Bold).color(PaletteColor::Background)),
    )
    .child(
      Row::new()
        .padding(16.0)
        .spacing(16.0)
        .child(pane("blur 0", 0.0))
        .child(pane("blur 4", 4.0))
        .child(pane("blur 12", 12.0))
        .child(pane("turned", 12.0).transform(Transform2D::rotate_deg(-8.0))),
    )
}

/// A box with a child placed past its edge: cut off at the box, or painted past it.
fn clipped(caption: &str, clip: bool, round: bool) -> Column {
  let mut parent = Stack::new()
    .size(120.0, 80.0)
    .background(PaletteColor::Muted)
    .rounded(if round { 24.0 } else { 4.0 })
    .border(Border::inside(1.0, PaletteColor::Border))
    .child(
      Rect::new(80.0, 50.0)
        .background(PaletteColor::Warning)
        .opacity(0.7)
        .rounded(4.0)
        .absolute_position(60.0, 20.0),
    );
  parent = if clip { parent.clip() } else { parent.overflow_visible() };
  // What holds the box lets out what the box does.
  let held = Row::new()
    .padding_right(if clip { 0.0 } else { 24.0 })
    .overflow_visible()
    .child(parent);
  sample(held, caption).overflow_visible()
}

fn clipping() -> Row {
  samples()
    .spacing(48.0)
    .overflow_visible()
    .child(clipped("overflow_visible()", false, false))
    .child(clipped("clip(): the default", true, false))
    .child(clipped("clip(), rounded 24", true, true))
}

/// A fill at five opacities; a group whose label keeps its contrast; a group in a group.
fn faded() -> Column {
  let mut levels = samples().spacing(12.0);
  for (caption, opacity) in [("1.0", 1.0), ("0.8", 0.8), ("0.6", 0.6), ("0.4", 0.4), ("0.2", 0.2)] {
    levels = levels.child(sample(
      Rect::new(60.0, 32.0)
        .background(PaletteColor::Primary)
        .rounded(4.0)
        .opacity(opacity),
      caption,
    ));
  }
  let button = |opacity: f32| {
    Row::new()
      .size(100.0, 36.0)
      .align_items(Alignment::Center)
      .justify(Justify::Center)
      .rounded(6.0)
      .background(PaletteColor::Primary)
      .foreground(PaletteColor::PrimaryForeground)
      .border(Border::inside(1.0, PaletteColor::Primary))
      .opacity(opacity)
      .child(text("Save", 14.0, FontWeight::Medium))
  };
  let nested = Column::new()
    .padding(12.0)
    .rounded(8.0)
    .background(PaletteColor::Chart1)
    .opacity(0.5)
    .child(
      Rect::new(76.0, 24.0)
        .rounded(4.0)
        .background(PaletteColor::Chart3)
        .opacity(0.5),
    );
  Column::new().width(FILL).spacing(24.0).child(levels).child(
    samples()
      .child(sample(button(1.0), "a group at 1.0"))
      .child(sample(button(0.4), "the same at 0.4"))
      .child(sample(nested, "0.5 in 0.5")),
  )
}

impl Component for VisualPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 5),
      fill: ctx.signal(2),
      radius: ctx.signal(2),
      shadow: ctx.signal(2),
      blur: ctx.signal(0),
      opacity: ctx.signal(10),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let rows = [
      (
        "background",
        "impl Into<BackgroundColor>",
        "none",
        "The fill: a color, or a role of the theme's palette, which follows light and dark.",
      ),
      (
        "rounded",
        "impl Into<RadiusValue>",
        "0",
        "The radius of every corner: pixels, or a RadiusSize of the theme. What is in the box is cut at the corners.",
      ),
      (
        "box_shadow",
        "impl Into<BoxShadowValue>",
        "none",
        "A ShadowStyle of the theme, a BoxShadow, or a list of them, the first on top. Paints outside the box, or \
         inside it with inset(); changes no layout.",
      ),
      (
        "blur",
        "f32",
        "0",
        "Blurs the element with all that is in it, as one layer: the radius in pixels.",
      ),
      (
        "backdrop_blur",
        "f32",
        "0",
        "Blurs what is painted behind the element, inside its box: frosted glass, with a fill that lets it through.",
      ),
      (
        "clip",
        "()",
        "",
        "Cuts off what is in the element at its box and its corners. A container does unless told otherwise.",
      ),
      (
        "overflow_visible",
        "()",
        "",
        "Lets what is in the element paint past its box: a shadow, a ring, a child placed outside it.",
      ),
      (
        "opacity",
        "f32",
        "1",
        "Fades the element and all that is in it as one group, blended once over what is behind it.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Engine", "Visual"],
        title: "Visual",
        description: "What a box is painted with beside its layout: a fill, rounded corners, shadows, a blur of \
                      itself or of what is behind it, whether it cuts off what runs past it, and how opaque it is. \
                      None of it changes a size or a place.",
        import: Some("use lurq::{app::theme::ShadowStyle, node::BoxShadow};"),
        status: Some("Stable"),
        since: None,
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "One box, painted as its props say. Give it a fill of the theme, its corners, a shadow, a blur and an \
           opacity, and turn the stage to the other appearance: the roles follow it.",
          playground,
        ),
        Section::new(
          "Colors of the theme",
          "A fill is a role of the theme's palette, never a hex of the page's: each is right in light and in dark, \
           and the text on it takes the role drawn on it.",
          self.frame.demo(
            0,
            colors(),
            ".background(PaletteColor::Primary)",
            &[
              "Column::new()",
              "    .background(PaletteColor::Primary)",
              "    .foreground(PaletteColor::PrimaryForeground)",
              "    .child(Text::new(\"primary\"))",
            ],
          ),
        ),
        Section::new(
          "Corners",
          "The same bar square, a little rounded, rounded, and as round as its height allows.",
          self.frame.demo(
            1,
            radii(),
            ".rounded(16.0)",
            &[
              "Rect::new(140.0, 50.0).background(PaletteColor::Primary).rounded(0.0)",
              "Rect::new(140.0, 50.0).background(PaletteColor::Primary).rounded(8.0)",
              "Rect::new(140.0, 50.0).background(PaletteColor::Primary).rounded(16.0)",
              "Rect::new(140.0, 50.0).background(PaletteColor::Primary).rounded(40.0)",
            ],
          ),
        ),
        Section::new(
          "Box shadows",
          "The theme's three elevations, and a card that rises under the pointer; then a layered shadow, a hard \
           offset, a glow and an inset well. A shadow paints outside its element, so what holds it lets it out.",
          self.frame.demo(
            2,
            shadows(),
            ".box_shadow(ShadowStyle::Md)",
            &[
              "Column::new().box_shadow(ShadowStyle::Md)",
              "",
              "// Rises under the pointer.",
              "Column::new()",
              "    .box_shadow(ShadowStyle::Sm)",
              "    .transition(Transition::box_shadow().duration_ms(200))",
              "    .hovered(|style| style.box_shadow(ShadowStyle::Lg))",
              "",
              "Column::new().box_shadow(vec![",
              "    BoxShadow::new(0.0, 1.0, 1.0, PaletteColor::Scrim),",
              "    BoxShadow::new(0.0, 8.0, 24.0, PaletteColor::Scrim).spread(-4.0),",
              "])",
              "Column::new().box_shadow(BoxShadow::new(6.0, 6.0, 0.0, PaletteColor::Foreground))",
              "Column::new().box_shadow(BoxShadow::new(0.0, 0.0, 16.0, PaletteColor::Primary).spread(2.0))",
              "Column::new().box_shadow(BoxShadow::new(0.0, 2.0, 8.0, PaletteColor::Scrim).inset())",
            ],
          ),
        ),
        Section::new(
          "Layer blur",
          "The same card blurred with all that is in it, as one layer, by growing radii, and once faded too.",
          self.frame.demo(
            3,
            blurred(),
            ".blur(6.0)",
            &[
              "Column::new()",
              "    .size(120.0, 64.0)",
              "    .background(PaletteColor::Card)",
              "    .child(Text::new(\"Blurred\"))",
              "    .opacity(0.5)",
              "    .blur(6.0)",
            ],
          ),
        ),
        Section::new(
          "Backdrop blur",
          "Frosted glass: panes with a fill that lets light through blur the stripes and the words behind them, by \
           growing radii. The last is turned, and its blur turns with it.",
          self.frame.demo(
            4,
            fit("glass", self.frame.stage_room(), GLASS, glass()),
            ".backdrop_blur(12.0)",
            &[
              "Column::new()",
              "    .size(96.0, 64.0)",
              "    .background(PaletteColor::GlassThin)",
              "    .border(Border::inside(1.0, PaletteColor::GlassRegular))",
              "    .rounded(12.0)",
              "    .backdrop_blur(12.0)",
            ],
          ),
        ),
        Section::new(
          "Clipping",
          "A box with a child placed past its edge. A container cuts off what is in it at its box, and at its \
           corners where they are rounded; overflow_visible lets it paint past them.",
          self.frame.demo(
            5,
            clipping(),
            ".overflow_visible()",
            &[
              "Stack::new()",
              "    .size(120.0, 80.0)",
              "    .overflow_visible()",
              "    .child(Rect::new(80.0, 50.0).absolute_position(60.0, 20.0))",
              "",
              "Stack::new().size(120.0, 80.0).rounded(24.0).clip().child(..)",
            ],
          ),
        ),
        Section::new(
          "Opacity",
          "A fill at five opacities. An opacity fades a group as one: the label of a button at 0.4 keeps its \
           contrast on its fill, and its border adds no lighter ring. A group at 0.5 in a group at 0.5 is at a \
           quarter.",
          self.frame.demo(
            6,
            faded(),
            ".opacity(0.4)",
            &[
              "Rect::new(60.0, 32.0).background(PaletteColor::Primary).opacity(0.6)",
              "",
              "Row::new()",
              "    .background(PaletteColor::Primary)",
              "    .border(Border::inside(1.0, PaletteColor::Primary))",
              "    .opacity(0.4)",
              "    .child(Text::new(\"Save\").color(PaletteColor::PrimaryForeground))",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a box is given to be painted with.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
