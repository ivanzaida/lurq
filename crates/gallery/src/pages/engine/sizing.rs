//! The page of sizing: what a box's width and height are (pixels, a share of its parent, its content's), what
//! bounds them, the room inside a box that its padding takes, and a spacer that takes the room left in a row.
//! Its playground sizes one box as its props say; its sections are the demo's Sizing page with bounds and an
//! aspect ratio after them.

use std::sync::Arc;

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{PaletteColor, RadiusSize},
  },
  components::{Column, Row, Spacer},
  core::{ElementRef, Signal},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{Element, dimension::Dimension, padding::Padding},
};

use super::kit::{named, sample, samples, tile, tint, well, well_line};
use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, text},
  playground::{Playground, Shown, Spec, divider, prop},
};

/// The kinds of a length the playground offers: a number of pixels, a share of the parent, all of it, and the
/// content's.
const KINDS: [(&str, usize); 4] = [("Px", 0), ("Pct(50)", 1), ("Fill", 2), ("Auto", 3)];
/// The pixels of a width and of a height of the kind `Px`.
const PX_WIDTH: f32 = 180.0;
const PX_HEIGHT: f32 = 96.0;
/// The bounds the playground offers, 0 for none.
const MIN_WIDTHS: [(&str, i32); 3] = [("none", 0), ("120", 120), ("240", 240)];
const MAX_WIDTHS: [(&str, i32); 3] = [("none", 0), ("140", 140), ("320", 320)];
const MIN_HEIGHTS: [(&str, i32); 3] = [("none", 0), ("60", 60), ("140", 140)];
const MAX_HEIGHTS: [(&str, i32); 3] = [("none", 0), ("48", 48), ("120", 120)];
const PADDINGS: [(&str, i32); 4] = [("0", 0), ("8", 8), ("16", 16), ("32", 32)];
/// A select of a bound of the playground: the prop it sets, its choices, and the signal that keeps the choice.
type Bound<'a> = (&'a str, &'a [(&'a str, i32)], &'a Signal<i32>);
/// How high the well the playground's box stands in is.
const WELL: f32 = 240.0;

/// A length of the kind at `index` of [`KINDS`], `px` long where it is in pixels.
fn length(index: usize, px: f32) -> Dimension {
  match index {
    0 => Dimension::Px(px),
    1 => Dimension::Pct(50.0),
    2 => Dimension::Pct(100.0),
    _ => Dimension::Auto,
  }
}

/// The Rust of a length of the kind at `index`.
fn length_code(index: usize, px: f32) -> Option<String> {
  match index {
    0 => Some(format!("{px:.1}")),
    1 => Some("Dimension::Pct(50.0)".to_owned()),
    2 => Some("Dimension::Pct(100.0)".to_owned()),
    _ => None,
  }
}

/// What a box holds: a pill with a word in it, so that the room its padding takes shows around it in the box's
/// tint.
fn content(word: &str) -> Row {
  Row::new()
    .height(24.0)
    .padding_horizontal(10.0)
    .align_items(Alignment::Center)
    .rounded(RadiusSize::Sm)
    .background(PaletteColor::Background)
    .child(text(word, 12.0, FontWeight::Medium).nowrap())
}

/// The box of the playground, as its props say.
#[derive(Clone, Copy)]
struct Shape {
  width: usize,
  height: usize,
  min_width: i32,
  max_width: i32,
  min_height: i32,
  max_height: i32,
  padding: i32,
}

impl Shape {
  fn build(self, measured: Option<ElementRef>) -> Element {
    let mut boxed = Row::new()
      .width(length(self.width, PX_WIDTH))
      .height(length(self.height, PX_HEIGHT))
      .padding(self.padding as f32)
      .align_items(Alignment::Center)
      .justify(Justify::Center)
      .rounded(RadiusSize::Sm)
      .background(tint(0))
      .child(content("content"));
    if self.min_width > 0 {
      boxed = boxed.min_width(self.min_width as f32);
    }
    if self.max_width > 0 {
      boxed = boxed.max_width(self.max_width as f32);
    }
    if self.min_height > 0 {
      boxed = boxed.min_height(self.min_height as f32);
    }
    if self.max_height > 0 {
      boxed = boxed.max_height(self.max_height as f32);
    }
    if let Some(measured) = measured {
      boxed = boxed.ref_element(measured);
    }
    well().height(WELL).padding(16.0).child(boxed).into()
  }

  fn code(self) -> Vec<String> {
    let mut code = vec!["Row::new()".to_owned()];
    if let Some(width) = length_code(self.width, PX_WIDTH) {
      code.push(format!("    .width({width})"));
    }
    if let Some(height) = length_code(self.height, PX_HEIGHT) {
      code.push(format!("    .height({height})"));
    }
    for (name, value) in [
      ("min_width", self.min_width),
      ("max_width", self.max_width),
      ("min_height", self.min_height),
      ("max_height", self.max_height),
      ("padding", self.padding),
    ] {
      if value > 0 {
        code.push(format!("    .{name}({value}.0)"));
      }
    }
    code.push("    .child(content)".to_owned());
    code
  }
}

pub struct SizingPage {
  frame: Frame,
  playground: Playground,
  /// Indexes into [`KINDS`].
  width: Signal<usize>,
  height: Signal<usize>,
  min_width: Signal<i32>,
  max_width: Signal<i32>,
  min_height: Signal<i32>,
  max_height: Signal<i32>,
  padding: Signal<i32>,
  /// The box of the playground, and its size as it was last laid out: the stage measures the well it stands in.
  boxed: ElementRef,
  boxed_size: Signal<String>,
}

impl SizingPage {
  fn play(&self, ctx: &mut Ctx) -> Element {
    let sized = Shape {
      width: self.width.get(),
      height: self.height.get(),
      min_width: self.min_width.get(),
      max_width: self.max_width.get(),
      min_height: self.min_height.get(),
      max_height: self.max_height.get(),
      padding: self.padding.get(),
    };
    let size = self.boxed_size.clone();
    self.boxed.observe_rect(move |rect| {
      let said = format!("{} × {}", rect.width.round(), rect.height.round());
      if size.get_untracked() != said {
        size.set(said);
      }
    });
    let boxed = self.boxed.clone();
    let tree = vec![
      format!("well   {}   padding 16", self.playground.measured()),
      format!("└─ box   {}   padding {}", self.boxed_size.get(), sized.padding),
      "   └─ content".to_owned(),
    ];

    let selects: [(&str, &str, &Signal<usize>); 2] = [
      ("width", "impl Into<Dimension>", &self.width),
      ("height", "impl Into<Dimension>", &self.height),
    ];
    let mut controls = Vec::new();
    for (index, (name, kind, signal)) in selects.into_iter().enumerate() {
      let set = signal.clone();
      controls.push(prop(
        name,
        kind,
        self
          .playground
          .select(index, &KINDS, &signal.get(), move |value| set.set(value)),
      ));
    }
    controls.push(divider());
    let bounds: [Bound<'_>; 5] = [
      ("min_width", &MIN_WIDTHS, &self.min_width),
      ("max_width", &MAX_WIDTHS, &self.max_width),
      ("min_height", &MIN_HEIGHTS, &self.min_height),
      ("max_height", &MAX_HEIGHTS, &self.max_height),
      ("padding", &PADDINGS, &self.padding),
    ];
    for (index, (name, options, signal)) in bounds.into_iter().enumerate() {
      let set = signal.clone();
      controls.push(prop(
        name,
        if name == "padding" {
          "impl Into<Padding>"
        } else {
          "impl Into<Dimension>"
        },
        self
          .playground
          .select(index + 2, options, &signal.get(), move |value| set.set(value)),
      ));
    }

    let reset = {
      let (width, height, min_width, max_width, min_height, max_height, padding) = (
        self.width.clone(),
        self.height.clone(),
        self.min_width.clone(),
        self.max_width.clone(),
        self.min_height.clone(),
        self.max_height.clone(),
        self.padding.clone(),
      );
      Arc::new(move || {
        width.set(0);
        height.set(0);
        min_width.set(0);
        max_width.set(0);
        min_height.set(0);
        max_height.set(0);
        padding.set(16);
      })
    };
    self.playground.card(
      ctx,
      Spec {
        title: "Sizing",
        make: Arc::new(move || sized.build(None)),
        shown: Shown::built(move |_| sized.build(Some(boxed))),
        fill: true,
        forceable: false,
        facts: format!("box {} px", self.boxed_size.get()),
        event: "",
        code: sized.code(),
        tree,
        controls,
        changed: [
          sized.width != 0,
          sized.height != 0,
          sized.min_width != 0,
          sized.max_width != 0,
          sized.min_height != 0,
          sized.max_height != 0,
          sized.padding != 16,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset,
      },
    )
  }
}

/// A box 120 wide, one 80 in a hundred of its row, and one as wide as its words.
fn kinds() -> Column {
  Column::new()
    .width(FILL)
    .spacing(12.0)
    .child(named("Px(120.0)", tile("120 px", 0, 120.0, 28.0)))
    .child(named(
      "Pct(80.0)",
      Row::new()
        .width(FILL)
        .child(tile("80% of the row", 1, Dimension::Pct(80.0), 28.0)),
    ))
    .child(named(
      "Auto",
      Row::new().child(tile("as wide as this", 2, Dimension::Auto, 28.0).padding_horizontal(18.0)),
    ))
}

/// A box whose padding is tinted around what it holds.
fn padded(caption: &str, index: usize, padding: Padding, height: f32) -> Row {
  Row::new()
    .width(FILL)
    .height(height)
    .padding(padding)
    .align_items(Alignment::Center)
    .rounded(RadiusSize::Md)
    .background(tint(index))
    .child(
      Row::new()
        .width(FILL)
        .height(FILL)
        .align_items(Alignment::Center)
        .justify(Justify::Center)
        .rounded(RadiusSize::Sm)
        .background(PaletteColor::Background)
        .child(text(caption, 13.0, FontWeight::Normal)),
    )
}

fn paddings() -> Column {
  Column::new()
    .width(FILL)
    .spacing(16.0)
    .child(sample(
      padded("all 20", 0, Padding::all(20.0), 80.0),
      "Padding::all(20.0)",
    ))
    .child(sample(
      padded("40 and 10", 4, Padding::symmetric(40.0, 10.0), 60.0),
      "Padding::symmetric(40.0, 10.0)",
    ))
}

/// Two boxes at the ends of a row, a spacer that grows between them, its bounds dashed.
fn spaced() -> Column {
  well().padding(12.0).child(
    Row::new()
      .width(FILL)
      .align_items(Alignment::Center)
      .child(tile("Left", 0, 80.0, 48.0))
      .child(
        Spacer::new()
          .height(30.0)
          .flex(1.0)
          .border(well_line())
          .rounded(RadiusSize::Sm),
      )
      .child(tile("Right", 1, 80.0, 48.0)),
  )
}

/// A box that fills its row up to a most, and one as wide as its words but no narrower than a least.
fn bounded() -> Column {
  Column::new()
    .width(FILL)
    .spacing(12.0)
    .child(sample(
      well()
        .padding(8.0)
        .child(tile("fills, up to 240", 2, FILL, 32.0).max_width(240.0)),
      ".width(Dimension::Pct(100.0)).max_width(240.0)",
    ))
    .child(sample(
      well().padding(8.0).child(
        Row::new().child(
          tile("short", 3, Dimension::Auto, 32.0)
            .padding_horizontal(12.0)
            .min_width(160.0),
        ),
      ),
      ".min_width(160.0)",
    ))
}

/// Boxes whose height their width gives.
fn ratios() -> Row {
  samples()
    .child(sample(
      tile("16 : 9", 0, 192.0, Dimension::Auto).aspect_ratio(16.0 / 9.0),
      "16.0 / 9.0",
    ))
    .child(sample(tile("1 : 1", 1, 96.0, Dimension::Auto).aspect_ratio(1.0), "1.0"))
    .child(sample(
      tile("3 : 4", 2, 96.0, Dimension::Auto).aspect_ratio(3.0 / 4.0),
      "3.0 / 4.0",
    ))
}

impl Component for SizingPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 7),
      width: ctx.signal(0),
      height: ctx.signal(0),
      min_width: ctx.signal(0),
      max_width: ctx.signal(0),
      min_height: ctx.signal(0),
      max_height: ctx.signal(0),
      padding: ctx.signal(16),
      boxed: ElementRef::new(),
      boxed_size: ctx.signal(String::new()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let rows = [
      (
        "width / height",
        "impl Into<Dimension>",
        "Auto",
        "A number of pixels (a plain f32 is Px), a share of the parent's room inside its padding (Pct), or \
         Auto: as large as what the box holds.",
      ),
      (
        "size",
        "(Dimension, Dimension)",
        "",
        "The width and the height at once.",
      ),
      (
        "min_width / max_width",
        "impl Into<Dimension>",
        "",
        "The least and the most the width can be, whatever the box is told or holds. A percent is of the parent.",
      ),
      (
        "min_height / max_height",
        "impl Into<Dimension>",
        "",
        "The same for the height.",
      ),
      (
        "padding",
        "impl Into<Padding>",
        "0",
        "The room inside the box around what it holds: all sides, or Padding::symmetric, or padding_left and the \
         others one side each.",
      ),
      (
        "aspect_ratio",
        "f32",
        "",
        "The width over the height: the axis that is Auto follows from the one that is known.",
      ),
      (
        "Spacer::new().flex(1.0)",
        "Spacer",
        "",
        "A box with nothing in it: with a flex factor it takes the room left in its row or column.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Engine", "Sizing"],
        title: "Sizing",
        description: "How large a box is: a number of pixels, a share of its parent, or as large as what it holds, \
                      between a least and a most, with padding inside it.",
        import: Some("use lurq::{components::Spacer, node::{dimension::Dimension, padding::Padding}};"),
        status: Some("Stable"),
        since: None,
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "A box in a well 240 high whose bounds are dashed. Choose its width and height, bound them, and give it \
           padding: the tint around the pill is the padding. A share is of the well inside its padding of 16.",
          playground,
        ),
        Section::new(
          "Kinds of length",
          "A width in pixels, a share of the row the box is in, and none at all, where the box is as wide as what \
           it holds and its padding.",
          self.frame.demo(
            0,
            kinds(),
            "Dimension::Pct(80.0)",
            &[
              "Row::new().size(120.0, 28.0)",
              "",
              "Row::new()",
              "    .width(Dimension::Pct(100.0))",
              "    .child(Row::new().width(Dimension::Pct(80.0)).height(28.0))",
              "",
              "// Auto: as wide as its text and its padding.",
              "Row::new().height(28.0).padding_horizontal(18.0).child(Text::new(\"as wide as this\"))",
            ],
          ),
        ),
        Section::new(
          "Padding",
          "Padding is room inside a box, between its edge and what it holds: the same on every side, or one for \
           the sides and one for above and below.",
          self.frame.demo(
            1,
            paddings(),
            "Padding::symmetric(40.0, 10.0)",
            &[
              "Row::new()",
              "    .width(Dimension::Pct(100.0))",
              "    .height(80.0)",
              "    .padding(Padding::all(20.0))",
              "    .child(content)",
              "",
              "Row::new()",
              "    .height(60.0)",
              "    .padding(Padding::symmetric(40.0, 10.0))",
              "    .child(content)",
            ],
          ),
        ),
        Section::new(
          "Spacer",
          "A spacer with a flex factor takes the room its row leaves, and pushes what comes after it to the end. \
           Its bounds are dashed here; it draws nothing of its own.",
          self.frame.demo(
            2,
            spaced(),
            "Spacer::new().flex(1.0)",
            &[
              "Row::new()",
              "    .width(Dimension::Pct(100.0))",
              "    .child(Rect::new(80.0, 48.0))",
              "    .child(Spacer::new().flex(1.0))",
              "    .child(Rect::new(80.0, 48.0))",
            ],
          ),
        ),
        Section::new(
          "Least and most",
          "A box that fills its row stops at its most; one as wide as its words is no narrower than its least. \
           Make the window narrower: the first gives way, down to the room there is.",
          self.frame.demo(
            3,
            bounded(),
            ".max_width(240.0)",
            &[
              "Row::new().width(Dimension::Pct(100.0)).max_width(240.0)",
              "",
              "Row::new().padding_horizontal(12.0).min_width(160.0).child(Text::new(\"short\"))",
            ],
          ),
        ),
        Section::new(
          "Aspect ratio",
          "A box told its width and a ratio is as high as the ratio makes it.",
          self.frame.demo(
            4,
            ratios(),
            ".aspect_ratio(16.0 / 9.0)",
            &[
              "Row::new().width(192.0).aspect_ratio(16.0 / 9.0)",
              "Row::new().width(96.0).aspect_ratio(1.0)",
              "Row::new().width(96.0).aspect_ratio(3.0 / 4.0)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What any box is given to size it.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
