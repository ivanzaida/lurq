//! The page of the engine's text: a `components::Text` in a `TextStyle`, its size, weight, style and line height,
//! how it wraps and is cut, selected, colored and shadowed. Its playground is a text as its props say; its sections
//! are the demo's Typography page, with decoration, spacing and case beside them.

use std::sync::Arc;

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::{Column, Rect, Row, Text, TextOverflow},
  core::Signal,
  layout::{
    Alignment,
    text_style::{FontStyle, FontWeight, TextAlign, TextCase, TextShadow, TextStrikethrough, TextStyle, TextUnderline},
  },
  node::{Element, TextTransformMode, border::Border, transform::Transform2D},
};

use super::kit::{samples, well_line};
use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, fit, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

const SIZES: [(&str, i32); 7] = [
  ("12", 12),
  ("14", 14),
  ("16", 16),
  ("20", 20),
  ("24", 24),
  ("32", 32),
  ("48", 48),
];
const WEIGHTS: [(&str, FontWeight); 9] = [
  ("Thin", FontWeight::Thin),
  ("ExtraLight", FontWeight::ExtraLight),
  ("Light", FontWeight::Light),
  ("Normal", FontWeight::Normal),
  ("Medium", FontWeight::Medium),
  ("SemiBold", FontWeight::SemiBold),
  ("Bold", FontWeight::Bold),
  ("ExtraBold", FontWeight::ExtraBold),
  ("Black", FontWeight::Black),
];
/// The choices of the playground's select of a weight: an index into [`WEIGHTS`], which a signal keeps.
const WEIGHT_CHOICES: [(&str, usize); 9] = [
  ("Thin", 0),
  ("ExtraLight", 1),
  ("Light", 2),
  ("Normal", 3),
  ("Medium", 4),
  ("SemiBold", 5),
  ("Bold", 6),
  ("ExtraBold", 7),
  ("Black", 8),
];
const STYLES: [(&str, usize); 2] = [("Normal", 0), ("Italic", 1)];
/// Line heights in tenths: 12 is 1.2.
const LINE_HEIGHTS: [(&str, i32); 4] = [("1.0", 10), ("1.2", 12), ("1.6", 16), ("2.0", 20)];
const OVERFLOWS: [(&str, usize); 4] = [("wraps", 0), ("nowrap()", 1), ("Elipsis", 2), ("max_lines(2)", 3)];
/// Widths of the text; 0 is none of its own, -1 as wide as the stage.
const WIDTHS: [(&str, i32); 4] = [("none", 0), ("160", 160), ("260", 260), ("Pct(100.0)", -1)];
const ALIGNS: [(&str, usize); 3] = [("Left", 0), ("Center", 1), ("Right", 2)];
const ALIGN_VALUES: [TextAlign; 3] = [TextAlign::Left, TextAlign::Center, TextAlign::Right];
/// The colors of the playground's text: the first is none, which takes the foreground of what it is in.
const COLORS: [(&str, usize); 5] = [
  ("none", 0),
  ("MutedForeground", 1),
  ("Primary", 2),
  ("Destructive", 3),
  ("Success", 4),
];
const COLOR_ROLES: [PaletteColor; 5] = [
  PaletteColor::Foreground,
  PaletteColor::MutedForeground,
  PaletteColor::Primary,
  PaletteColor::Destructive,
  PaletteColor::Success,
];

const FOX: &str = "The quick brown fox";
const PLAYED: &str = "The quick brown fox jumps over the lazy dog, and the dog does not mind.";

/// The theme's body type with `change` made to it: what the page's texts are styled from.
fn body(theme: &Theme, change: impl FnOnce(&mut TextStyle)) -> TextStyle {
  let mut style = theme.typography_style(TypographyStyle::Body);
  change(&mut style);
  style
}

/// A text with what it is beside it, or above it where there is no room: a line of a list of samples.
fn labeled(label: &str, width: f32, shown: impl Into<Element>) -> Row {
  Row::new()
    .width(FILL)
    .spacing(16.0)
    .wrap()
    .align_items(Alignment::Center)
    .child(quiet(label, 12.0).width(width).flex_shrink(0.0))
    .child(Column::new().flex(1.0).min_width(160.0).max_width(FILL).child(shown))
}

/// The playground's props, as they are now.
#[derive(Clone)]
struct Played {
  content: String,
  size: i32,
  weight: usize,
  italic: bool,
  line_height: i32,
  overflow: usize,
  width: i32,
  align: usize,
  color: usize,
  selectable: bool,
  underline: bool,
  strikethrough: bool,
}

impl Played {
  fn text(&self, theme: &Theme) -> Text {
    let style = body(theme, |style| {
      style.font_size = self.size as f32;
      style.weight = WEIGHTS[self.weight.min(WEIGHTS.len() - 1)].1;
      style.style = if self.italic {
        FontStyle::Italic
      } else {
        FontStyle::Normal
      };
      style.line_height = self.line_height as f32 / 10.0;
    });
    let mut text = Text::styled(&self.content, style);
    text = match self.width {
      0 => text,
      -1 => text.width(FILL),
      width => text.width(width as f32).max_width(FILL),
    };
    text = match self.overflow {
      1 => text.nowrap(),
      2 => text.text_overflow(TextOverflow::Elipsis),
      3 => text.max_lines(2),
      _ => text,
    };
    if self.align != 0 {
      text = text.text_align(ALIGN_VALUES[self.align.min(ALIGN_VALUES.len() - 1)]);
    }
    if self.color != 0 {
      text = text.color(COLOR_ROLES[self.color.min(COLOR_ROLES.len() - 1)].clone());
    }
    if self.underline {
      text = text.underline(TextUnderline::new());
    }
    if self.strikethrough {
      text = text.strikethrough(TextStrikethrough::new());
    }
    text.selectable(self.selectable)
  }

  fn code(&self) -> Vec<String> {
    let mut code = vec![
      "// body: the theme's body type, theme.typography_style(TypographyStyle::Body)".to_owned(),
      "Text::styled(".to_owned(),
      format!("    {:?},", self.content),
      "    TextStyle {".to_owned(),
      format!("        font_size: {}.0,", self.size),
    ];
    if self.weight != 3 {
      code.push(format!(
        "        weight: FontWeight::{},",
        WEIGHTS[self.weight.min(WEIGHTS.len() - 1)].0
      ));
    }
    if self.italic {
      code.push("        style: FontStyle::Italic,".to_owned());
    }
    code.push(format!(
      "        line_height: {},",
      LINE_HEIGHTS
        .iter()
        .find(|(_, tenths)| *tenths == self.line_height)
        .map_or("1.2", |(name, _)| *name)
    ));
    code.push("        ..body".to_owned());
    code.push("    },".to_owned());
    code.push(")".to_owned());
    match self.width {
      0 => {}
      -1 => code.push(".width(Dimension::Pct(100.0))".to_owned()),
      width => code.push(format!(".width({width}.0)")),
    }
    match self.overflow {
      1 => code.push(".nowrap()".to_owned()),
      2 => code.push(".text_overflow(TextOverflow::Elipsis)".to_owned()),
      3 => code.push(".max_lines(2)".to_owned()),
      _ => {}
    }
    if self.align != 0 {
      code.push(format!(
        ".text_align(TextAlign::{})",
        ALIGNS[self.align.min(ALIGNS.len() - 1)].0
      ));
    }
    if self.color != 0 {
      code.push(format!(
        ".color(PaletteColor::{})",
        COLORS[self.color.min(COLORS.len() - 1)].0
      ));
    }
    if self.underline {
      code.push(".underline(TextUnderline::new())".to_owned());
    }
    if self.strikethrough {
      code.push(".strikethrough(TextStrikethrough::new())".to_owned());
    }
    if self.selectable {
      code.push(".selectable(true)".to_owned());
    }
    code
  }
}

pub struct TextPage {
  frame: Frame,
  playground: Playground,
  content: Signal<String>,
  size: Signal<i32>,
  /// An index into [`WEIGHTS`].
  weight: Signal<usize>,
  /// 1 for italic.
  style: Signal<usize>,
  line_height: Signal<i32>,
  /// An index into [`OVERFLOWS`].
  overflow: Signal<usize>,
  width: Signal<i32>,
  /// An index into [`ALIGN_VALUES`].
  align: Signal<usize>,
  /// An index into [`COLOR_ROLES`].
  color: Signal<usize>,
  selectable: Signal<bool>,
  underline: Signal<bool>,
  strikethrough: Signal<bool>,
}

impl TextPage {
  fn played(&self) -> Played {
    Played {
      content: self.content.get(),
      size: self.size.get(),
      weight: self.weight.get(),
      italic: self.style.get() == 1,
      line_height: self.line_height.get(),
      overflow: self.overflow.get(),
      width: self.width.get(),
      align: self.align.get(),
      color: self.color.get(),
      selectable: self.selectable.get(),
      underline: self.underline.get(),
      strikethrough: self.strikethrough.get(),
    }
  }

  fn play(&self, ctx: &mut Ctx) -> Element {
    let played = self.played();
    let make = {
      let played = played.clone();
      move || -> Element { played.text(&lurq::ui::current_theme()).into() }
    };
    let shown = {
      let played = played.clone();
      Shown::built(move |ctx| played.text(ctx.theme()).into())
    };
    let tree = vec![format!(
      "text   {}   {} characters",
      self.playground.measured(),
      played.content.chars().count()
    )];
    let (set_size, set_weight, set_style, set_line_height) = (
      self.size.clone(),
      self.weight.clone(),
      self.style.clone(),
      self.line_height.clone(),
    );
    let (set_overflow, set_width, set_align, set_color) = (
      self.overflow.clone(),
      self.width.clone(),
      self.align.clone(),
      self.color.clone(),
    );
    let reset = {
      let (content, size, weight, style, line_height, overflow) = (
        self.content.clone(),
        self.size.clone(),
        self.weight.clone(),
        self.style.clone(),
        self.line_height.clone(),
        self.overflow.clone(),
      );
      let (width, align, color, flags) = (
        self.width.clone(),
        self.align.clone(),
        self.color.clone(),
        [
          self.selectable.clone(),
          self.underline.clone(),
          self.strikethrough.clone(),
        ],
      );
      Arc::new(move || {
        content.set(PLAYED.to_owned());
        size.set(20);
        weight.set(3);
        style.set(0);
        line_height.set(12);
        overflow.set(0);
        width.set(260);
        align.set(0);
        color.set(0);
        for flag in &flags {
          flag.set(false);
        }
      })
    };
    let (italic, chosen_style) = (played.italic, usize::from(played.italic));
    let controls = vec![
      prop("content", "&str", field(&self.content)),
      prop(
        "font_size",
        "f32",
        self
          .playground
          .select(0, &SIZES, &played.size, move |value| set_size.set(value)),
      ),
      prop(
        "weight",
        "FontWeight",
        self
          .playground
          .select(1, &WEIGHT_CHOICES, &played.weight, move |value| set_weight.set(value)),
      ),
      prop(
        "style",
        "FontStyle",
        self
          .playground
          .select(2, &STYLES, &chosen_style, move |value| set_style.set(value)),
      ),
      prop(
        "line_height",
        "f32",
        self
          .playground
          .select(3, &LINE_HEIGHTS, &played.line_height, move |value| {
            set_line_height.set(value)
          }),
      ),
      prop(
        "wrap / overflow",
        "TextOverflow",
        self
          .playground
          .select(4, &OVERFLOWS, &played.overflow, move |value| set_overflow.set(value)),
      ),
      prop(
        "width",
        "impl Into<Dimension>",
        self
          .playground
          .select(5, &WIDTHS, &played.width, move |value| set_width.set(value)),
      ),
      prop(
        "text_align",
        "TextAlign",
        self
          .playground
          .select(6, &ALIGNS, &played.align, move |value| set_align.set(value)),
      ),
      prop(
        "color",
        "impl Into<TextColor>",
        self
          .playground
          .select(7, &COLORS, &played.color, move |value| set_color.set(value)),
      ),
      divider(),
      flag("selectable", &self.selectable),
      flag("underline", &self.underline),
      flag("strikethrough", &self.strikethrough),
    ];
    let changed = [
      played.content != PLAYED,
      played.size != 20,
      played.weight != 3,
      italic,
      played.line_height != 12,
      played.overflow != 0,
      played.width != 260,
      played.align != 0,
      played.color != 0,
      played.selectable,
      played.underline,
      played.strikethrough,
    ]
    .into_iter()
    .filter(|changed| *changed)
    .count();
    self.playground.card(
      ctx,
      Spec {
        title: "Text",
        make: Arc::new(make),
        shown,
        fill: true,
        forceable: false,
        facts: format!("{} px", played.size),
        event: "",
        code: played.code(),
        tree,
        controls,
        changed,
        reset,
      },
    )
  }
}

/// The same words at each size.
fn sizes(theme: &Theme) -> Column {
  let mut list = Column::new().width(FILL).spacing(8.0);
  for size in [48.0, 32.0, 24.0, 20.0, 16.0, 14.0, 12.0] {
    list = list.child(labeled(
      &format!("{size}"),
      40.0,
      Text::styled(FOX, body(theme, |style| style.font_size = size)),
    ));
  }
  list
}

/// The same words in each weight.
fn weights(theme: &Theme) -> Column {
  let mut list = Column::new().width(FILL).spacing(8.0);
  for (name, weight) in WEIGHTS {
    list = list.child(labeled(
      &format!("{name} ({})", weight.value()),
      150.0,
      Text::styled(
        FOX,
        body(theme, |style| {
          style.font_size = 16.0;
          style.weight = weight;
        }),
      ),
    ));
  }
  list
}

/// The same words upright and slanted, in two weights.
fn styles(theme: &Theme) -> Column {
  let mut list = Column::new().width(FILL).spacing(8.0);
  for (name, weight, slant) in [
    ("Normal", FontWeight::Normal, FontStyle::Normal),
    ("Italic", FontWeight::Normal, FontStyle::Italic),
    ("Bold", FontWeight::Bold, FontStyle::Normal),
    ("Bold italic", FontWeight::Bold, FontStyle::Italic),
  ] {
    list = list.child(labeled(
      name,
      110.0,
      Text::styled(
        FOX,
        body(theme, |style| {
          style.font_size = 18.0;
          style.weight = weight;
          style.style = slant;
          style.line_height = 1.2;
        }),
      ),
    ));
  }
  list
}

/// A paragraph of a few lines at each line height.
fn line_heights(theme: &Theme) -> Column {
  let words = "Line height sets the rhythm of a text that wraps: how far one line is from the next.";
  let mut list = Column::new().width(FILL).spacing(16.0);
  for (name, tenths) in LINE_HEIGHTS {
    list = list.child(labeled(
      name,
      40.0,
      Text::styled(
        words,
        body(theme, |style| {
          style.font_size = 16.0;
          style.line_height = tenths as f32 / 10.0;
        }),
      )
      .width(360.0)
      .max_width(FILL),
    ));
  }
  list
}

/// A text 260 wide that wraps, that does not, that ends in an ellipsis, and that stops at two lines: each in a box
/// that shows the width it is given.
fn wrapping(theme: &Theme) -> Column {
  let words = "Wrapping keeps a long text inside the width it is given, a line under another.";
  let style = body(theme, |style| style.font_size = 16.0);
  let boxed = |text: Text| {
    Column::new()
      .width(260.0)
      .max_width(FILL)
      .border(well_line())
      .overflow_visible()
      .child(text.width(FILL))
  };
  let mut list = Column::new().width(FILL).spacing(16.0);
  let cases: [(&str, Text); 4] = [
    ("wraps", Text::styled(words, style.clone())),
    ("nowrap()", Text::styled(words, style.clone()).nowrap()),
    (
      "text_overflow(Elipsis)",
      Text::styled(words, style.clone()).text_overflow(TextOverflow::Elipsis),
    ),
    ("max_lines(2)", Text::styled(words, style).max_lines(2)),
  ];
  for (name, text) in cases {
    list = list.child(labeled(name, 170.0, boxed(text)));
  }
  list
}

/// Two cards with a dot, a title and a description that wraps in the card's width.
fn meta_cards(theme: &Theme) -> Row {
  let card = |title: &str, description: &str, dot: PaletteColor| {
    Column::new()
      .width(420.0)
      .max_width(FILL)
      .spacing(8.0)
      .padding(12.0)
      .rounded(RadiusSize::Md)
      .background(PaletteColor::Muted)
      .border(Border::inside(1.0, PaletteColor::Border))
      .child(Rect::new(12.0, 12.0).rounded(RadiusSize::Full).background(dot))
      .child(Text::styled(
        title,
        body(theme, |style| {
          style.font_size = 15.0;
          style.weight = FontWeight::Bold;
        }),
      ))
      .child(
        Text::styled(description, body(theme, |style| style.font_size = 14.0))
          .color(PaletteColor::MutedForeground)
          .width(FILL),
      )
  };
  samples()
    .spacing(16.0)
    .child(card(
      "Advanced path",
      "Prefer the seed restore unless you exported a private key and checked the recovery package before you \
       moved anything.",
      PaletteColor::Destructive,
    ))
    .child(card(
      "Recovery note",
      "Keep the account's phrases apart from what unlocks the device.",
      PaletteColor::Warning,
    ))
}

/// Texts to select: a line, a paragraph of lines, and a paragraph in a turned parent.
fn selection(theme: &Theme, room: f32) -> Column {
  let style = body(theme, |style| style.font_size = 16.0);
  let single = "Selectable text takes a drag, a double click for a word and a triple click for a line.";
  let lines =
    "Selectable text that wraps keeps each selected row lit.\nDrag across the lines to see the range of each.";
  let turned = "This selectable text is in a turned parent.\nIts selection is turned with it.";
  let item = |label: &str, shown: Element| {
    Column::new()
      .width(FILL)
      .spacing(8.0)
      .overflow_visible()
      .child(quiet(label, 12.0))
      .child(shown)
  };
  Column::new()
    .width(FILL)
    .spacing(24.0)
    .overflow_visible()
    .child(item(
      "One line, nowrap()",
      fit(
        "text-selection-line",
        room,
        660.0,
        Text::styled(single, style.clone()).selectable(true).nowrap(),
      ),
    ))
    .child(item(
      "Lines",
      Text::styled(lines, style.clone())
        .selectable(true)
        .width(420.0)
        .max_width(FILL)
        .into(),
    ))
    .child(item(
      "A turned parent",
      Column::new()
        .width(480.0)
        .max_width(FILL)
        .padding(14.0)
        .rounded(RadiusSize::Md)
        .background(PaletteColor::Muted)
        .border(Border::inside(1.0, PaletteColor::Border))
        .transform(Transform2D::rotate_deg(-2.0).then(&Transform2D::scale(1.02, 1.02)))
        .overflow_visible()
        .child(
          Text::styled(turned, style)
            .selectable(true)
            .text_transform_mode(TextTransformMode::Rasterized)
            .width(FILL),
        )
        .into(),
    ))
}

/// A line in each of six roles of the theme.
fn colors(theme: &Theme) -> Column {
  let mut list = Column::new().width(FILL).spacing(6.0);
  for (name, role) in [
    ("Foreground", PaletteColor::Foreground),
    ("Primary", PaletteColor::Primary),
    ("Success", PaletteColor::Success),
    ("Warning", PaletteColor::Warning),
    ("Destructive", PaletteColor::Destructive),
    ("MutedForeground", PaletteColor::MutedForeground),
  ] {
    list = list.child(
      Text::styled(
        &format!("PaletteColor::{name}"),
        body(theme, |style| style.font_size = 15.0),
      )
      .color(role)
      .width(FILL),
    );
  }
  list
}

/// The same words under four shadows: a shadow takes a color, which is read from the theme as the page is drawn.
fn shadows(theme: &Theme) -> Column {
  let color = |role: PaletteColor| theme.palette_color(role);
  let variants = [
    (
      "Hard offset",
      TextShadow::new(2.0, 2.0, 0.0, color(PaletteColor::Scrim)),
    ),
    ("Soft drop", TextShadow::new(2.0, 3.0, 6.0, color(PaletteColor::Scrim))),
    ("Glow", TextShadow::new(0.0, 0.0, 10.0, color(PaletteColor::Primary))),
    (
      "Colored offset",
      TextShadow::new(-3.0, 2.0, 4.0, color(PaletteColor::Destructive)),
    ),
  ];
  let mut list = Column::new().width(FILL).spacing(12.0).overflow_visible();
  for (name, shadow) in variants {
    list = list.child(
      labeled(
        name,
        110.0,
        Text::styled(
          FOX,
          body(theme, |style| {
            style.font_size = 24.0;
            style.weight = FontWeight::Bold;
          }),
        )
        .text_shadow(shadow),
      )
      .overflow_visible(),
    );
  }
  list
}

/// Lines under and through, letters spaced, and a text drawn in capitals.
fn decoration(theme: &Theme) -> Column {
  let style = body(theme, |style| style.font_size = 16.0);
  let mut list = Column::new().width(FILL).spacing(10.0);
  let cases: [(&str, Text); 6] = [
    (
      "underline",
      Text::styled(FOX, style.clone()).underline(TextUnderline::new()),
    ),
    (
      "strikethrough",
      Text::styled(FOX, style.clone()).strikethrough(TextStrikethrough::new()),
    ),
    (
      "letter_spacing(-0.5)",
      Text::styled(FOX, style.clone()).letter_spacing(-0.5),
    ),
    (
      "letter_spacing(2.0)",
      Text::styled(FOX, style.clone()).letter_spacing(2.0),
    ),
    (
      "TextCase::Upper",
      Text::styled(
        FOX,
        TextStyle {
          case: TextCase::Upper,
          letter_spacing: 1.0,
          ..style.clone()
        },
      ),
    ),
    ("Mono", Text::new(FOX).variant(TypographyStyle::Mono)),
  ];
  for (name, text) in cases {
    list = list.child(labeled(name, 170.0, text));
  }
  list
}

impl Component for TextPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 8),
      content: ctx.signal(PLAYED.to_owned()),
      size: ctx.signal(20),
      weight: ctx.signal(3),
      style: ctx.signal(0),
      line_height: ctx.signal(12),
      overflow: ctx.signal(0),
      width: ctx.signal(260),
      align: ctx.signal(0),
      color: ctx.signal(0),
      selectable: ctx.signal(false),
      underline: ctx.signal(false),
      strikethrough: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let theme = ctx.theme().clone();
    let rows = [
      (
        "Text::styled",
        "(&str, TextStyle)",
        "",
        "A text in a style: its family, size, weight, slant, line height, color and the rest. Text::new takes the \
         default style, and variant a typography style of the theme.",
      ),
      (
        "font_size",
        "f32",
        "14",
        "In the style: the size of the type in logical pixels.",
      ),
      (
        "weight",
        "FontWeight",
        "Normal",
        "In the style: from Thin (100) to Black (900), or a number.",
      ),
      ("style", "FontStyle", "Normal", "In the style: upright, or Italic."),
      (
        "line_height",
        "f32",
        "1.2",
        "In the style: how high a line is, as a multiple of the size.",
      ),
      (
        "nowrap",
        "()",
        "",
        "The text stays on one line, past its width if it is longer.",
      ),
      (
        "text_overflow",
        "TextOverflow",
        "Clip",
        "Elipsis keeps the text on one line and ends it in an ellipsis where its width ends.",
      ),
      (
        "max_lines",
        "u32",
        "0",
        "At most this many lines, the last ending in an ellipsis; 0 is no limit.",
      ),
      (
        "text_align",
        "impl Into<TextAlign>",
        "Left",
        "Where each line stands in the text's width.",
      ),
      (
        "color",
        "impl Into<TextColor>",
        "",
        "A role of the theme or a color. A text with none takes the foreground of what it is in.",
      ),
      (
        "text_shadow",
        "TextShadow",
        "",
        "A shadow under the glyphs: an offset, a blur and a color.",
      ),
      (
        "underline",
        "TextUnderline",
        "",
        "A line under the text; strikethrough draws one through it.",
      ),
      (
        "letter_spacing",
        "f32",
        "0",
        "Room after every glyph, in logical pixels; less than 0 draws them closer.",
      ),
      (
        "selectable",
        "bool",
        "false",
        "Whether a drag selects the text, a double click a word and a triple click a line, and a copy takes it.",
      ),
      (
        "text_transform_mode",
        "TextTransformMode",
        "Bitmap",
        "How a turned text is drawn: Rasterized draws its glyphs at the angle, sharper for a text that stays turned.",
      ),
    ];
    let playground = self.play(ctx);
    let room = self.frame.stage_room();
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Engine", "Text"],
        title: "Text",
        description: "A text in a style: its size, weight, slant and line height; how it wraps in its width or is \
                      cut; whether it can be selected; its color, its shadow and the lines on it.",
        import: Some("use lurq::{components::Text, layout::text_style::TextStyle};"),
        status: Some("Stable"),
        since: None,
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "A text in the theme's body type with the props on the right: what it says, its size, weight, slant and \
           line height, how it wraps or is cut in its width, where its lines stand, and its color.",
          playground,
        ),
        Section::new(
          "Sizes",
          "The same words from 48 down to 12. A text is as high as its lines: its size times its line height.",
          self.frame.demo(
            0,
            sizes(&theme),
            "TextStyle { font_size: 48.0, ..body }",
            &[
              "let body = theme.typography_style(TypographyStyle::Body);",
              "Text::styled(\"The quick brown fox\", TextStyle { font_size: 48.0, ..body.clone() })",
              "Text::styled(\"The quick brown fox\", TextStyle { font_size: 32.0, ..body.clone() })",
              "// ..",
            ],
          ),
        ),
        Section::new(
          "Weights",
          "The nine weights of a font, from Thin to Black. A family without one draws the nearest it has.",
          self.frame.demo(
            1,
            weights(&theme),
            "TextStyle { weight: FontWeight::Bold, ..body }",
            &[
              "Text::styled(\"The quick brown fox\", TextStyle {",
              "    font_size: 16.0,",
              "    weight: FontWeight::Bold,",
              "    ..body",
              "})",
            ],
          ),
        ),
        Section::new(
          "Styles",
          "Upright and italic, in the normal weight and in bold.",
          self.frame.demo(
            2,
            styles(&theme),
            "TextStyle { style: FontStyle::Italic, ..body }",
            &[
              "Text::styled(\"The quick brown fox\", TextStyle {",
              "    font_size: 18.0,",
              "    weight: FontWeight::Bold,",
              "    style: FontStyle::Italic,",
              "    line_height: 1.2,",
              "    ..body",
              "})",
            ],
          ),
        ),
        Section::new(
          "Line height",
          "The same paragraph at four line heights: how far one line of a text that wraps is from the next.",
          self.frame.demo(
            3,
            line_heights(&theme),
            "TextStyle { line_height: 1.6, ..body }",
            &[
              "Text::styled(paragraph, TextStyle { font_size: 16.0, line_height: 1.6, ..body })",
              "    .width(360.0)",
            ],
          ),
        ),
        Section::new(
          "Wrapping and overflow",
          "A text 260 wide, in a box that shows its width: it wraps under itself, stays on one line cut at its edge, \
           ends in an ellipsis, or stops after two lines.",
          self.frame.demo(
            4,
            wrapping(&theme),
            ".text_overflow(TextOverflow::Elipsis)",
            &[
              "Text::styled(words, style.clone()).width(260.0)",
              "Text::styled(words, style.clone()).width(260.0).nowrap()",
              "Text::styled(words, style.clone()).width(260.0).text_overflow(TextOverflow::Elipsis)",
              "Text::styled(words, style).width(260.0).max_lines(2)",
            ],
          ),
        ),
        Section::new(
          "In a card",
          "A title and a description in a card of a fixed width: the description wraps in the card's width, and the \
           card is as high as it needs.",
          self.frame.demo(
            5,
            meta_cards(&theme),
            "Column::new().width(420.0).child(Text::styled(..).width(FILL))",
            &[
              "Column::new()",
              "    .width(420.0)",
              "    .spacing(8.0)",
              "    .padding(12.0)",
              "    .child(Rect::new(12.0, 12.0).rounded(RadiusSize::Full).background(PaletteColor::Destructive))",
              "    .child(Text::styled(\"Advanced path\", TextStyle { font_size: 15.0, weight: FontWeight::Bold, \
               ..body.clone() }))",
              "    .child(",
              "        Text::styled(description, TextStyle { font_size: 14.0, ..body })",
              "            .color(PaletteColor::MutedForeground)",
              "            .width(Dimension::Pct(100.0)),",
              "    )",
            ],
          ),
        ),
        Section::new(
          "Selection",
          "Drag over a text to select it, double click a word, triple click a line, and copy what is selected. A \
           text in a turned parent is selected where it is drawn.",
          self.frame.demo(
            6,
            selection(&theme, room),
            ".selectable(true)",
            &[
              "Text::styled(line, style.clone()).selectable(true).nowrap()",
              "Text::styled(lines, style.clone()).selectable(true).width(420.0)",
              "",
              "Column::new()",
              "    .width(480.0)",
              "    .transform(Transform2D::rotate_deg(-2.0).then(&Transform2D::scale(1.02, 1.02)))",
              "    .child(",
              "        Text::styled(turned, style)",
              "            .selectable(true)",
              "            .text_transform_mode(TextTransformMode::Rasterized),",
              "    )",
            ],
          ),
        ),
        Section::new(
          "Colors",
          "A text in six roles of the theme: each is the role's color in light and in dark.",
          self.frame.demo(
            7,
            colors(&theme),
            ".color(PaletteColor::Primary)",
            &[
              "Text::styled(\"PaletteColor::Primary\", TextStyle { font_size: 15.0, ..body })",
              "    .color(PaletteColor::Primary)",
            ],
          ),
        ),
        Section::new(
          "Shadow",
          "A shadow under the glyphs: one with no blur, a soft one, a glow with no offset, and one in a color.",
          self.frame.demo(
            8,
            shadows(&theme),
            ".text_shadow(TextShadow::new(2.0, 3.0, 6.0, color))",
            &[
              "let color = |role| theme.palette_color(role);",
              "Text::styled(\"The quick brown fox\", TextStyle { font_size: 24.0, weight: FontWeight::Bold, ..body })",
              "    .text_shadow(TextShadow::new(2.0, 3.0, 6.0, color(PaletteColor::Scrim)))",
              "",
              "// A glow: no offset, a blur.",
              ".text_shadow(TextShadow::new(0.0, 0.0, 10.0, color(PaletteColor::Primary)))",
            ],
          ),
        ),
        Section::new(
          "Lines, spacing and case",
          "A line under the text and through it, letters set closer and farther apart, a text drawn in capitals, and \
           the theme's family of code. A text drawn in capitals is still copied as it is written.",
          self.frame.demo(
            9,
            decoration(&theme),
            ".underline(TextUnderline::new())",
            &[
              "Text::styled(\"The quick brown fox\", style.clone()).underline(TextUnderline::new())",
              "Text::styled(\"The quick brown fox\", style.clone()).strikethrough(TextStrikethrough::new())",
              "Text::styled(\"The quick brown fox\", style.clone()).letter_spacing(2.0)",
              "Text::styled(\"The quick brown fox\", TextStyle { case: TextCase::Upper, ..style })",
              "Text::new(\"The quick brown fox\").variant(TypographyStyle::Mono)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a text is given, in its style and on itself.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
