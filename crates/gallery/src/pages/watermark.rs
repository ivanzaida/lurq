//! The page of the watermark: its playground, the board's card, a long page in a scroll, and a picture for a mark.

use std::sync::Arc;

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{PaletteColor, RadiusSize, TypographyStyle},
  },
  components::{Column, Row, ScrollVertical},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{Icon, Watermark},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, line, muted, props_table, segmented_fill, styled},
  playground::{Playground, Shown, Spec, field, flag, prop},
};

const ANGLES: [(&str, f32); 4] = [("-20", -20.0), ("-35", -35.0), ("0", 0.0), ("20", 20.0)];

const GAPS: [(&str, (f32, f32)); 3] = [
  ("27, 29", (27.0, 29.0)),
  ("12, 16", (12.0, 16.0)),
  ("48, 56", (48.0, 56.0)),
];

const MARK: &str = "lurq internal";

/// The board's card: 220 by 120 on the background, with an edge, that the watermark covers.
fn board_card() -> Column {
  Column::new()
    .size(220.0, 120.0)
    .rounded(RadiusSize::Lg)
    .background(PaletteColor::Background)
    .border(line())
}

/// A card of a report, with a few lines on it, that the marks lie over.
fn report(width: f32) -> Column {
  Column::new()
    .width(width)
    .max_width(FILL)
    .padding(20.0)
    .spacing(10.0)
    .rounded(RadiusSize::Lg)
    .background(PaletteColor::Background)
    .border(line())
    .child(styled("Quarterly report", TypographyStyle::Title, |_| {}))
    .child(muted(
      "Revenue grew 12% on the quarter before, led by the new plans. The figures are not final and are not to be \
       shared outside the team.",
      TypographyStyle::Body,
    ))
}

/// The logo the picture section repeats.
fn logo() -> Row {
  Row::new()
    .size(28.0, 28.0)
    .align_items(Alignment::Center)
    .child(Icon::new("shield").size(24.0))
}

pub struct WatermarkPage {
  frame: Frame,
  playground: Playground,
  text: Signal<String>,
  /// The index of the angle in [`ANGLES`].
  angle: Signal<usize>,
  /// The index of the gap in [`GAPS`].
  gap: Signal<usize>,
  media: Signal<bool>,
}

impl WatermarkPage {
  /// The watermark the panel's props make over the report, and the Rust that builds it.
  fn watermark(text: &str, angle: usize, gap: usize, media: bool) -> (Watermark, Vec<String>) {
    let mut watermark = Watermark::new(text);
    let mut call = vec![format!("Watermark::new({text:?})")];
    if angle != 0 {
      watermark = watermark.angle(ANGLES[angle].1);
      call.push(format!("    .angle({:.1})", ANGLES[angle].1));
    }
    if gap != 0 {
      let (x, y) = GAPS[gap].1;
      watermark = watermark.gap(x, y);
      call.push(format!("    .gap({x:.1}, {y:.1})"));
    }
    if media {
      watermark = watermark.media(logo());
      call.push("    .media(logo)".to_owned());
    }
    call.push("    .child(report)".to_owned());
    (watermark.child(report(420.0)), call)
  }

  fn play(&self, ctx: &mut Ctx) -> Element {
    let text = self.text.get();
    let (angle, gap, media) = (self.angle.get(), self.gap.get(), self.media.get());
    let made = {
      let text = text.clone();
      move || -> Element { Self::watermark(&text, angle, gap, media).0.into() }
    };
    let (watermark, call) = Self::watermark(&text, angle, gap, media);
    let (to_angle, to_gap) = (self.angle.clone(), self.gap.clone());
    let controls = vec![
      prop("text", "&str", field(&self.text)),
      prop(
        "angle",
        "f32",
        segmented_fill(
          &ANGLES
            .iter()
            .enumerate()
            .map(|(index, (name, _))| (*name, index))
            .collect::<Vec<_>>(),
          &angle,
          move |chosen| to_angle.set(chosen),
        ),
      ),
      prop(
        "gap",
        "(f32, f32)",
        segmented_fill(
          &GAPS
            .iter()
            .enumerate()
            .map(|(index, (name, _))| (*name, index))
            .collect::<Vec<_>>(),
          &gap,
          move |chosen| to_gap.set(chosen),
        ),
      ),
      flag("media", &self.media),
    ];
    let tree = vec![
      format!(
        "root      {}   what it covers, and the layer over it",
        self.playground.measured()
      ),
      "└─ layer   no pointer, nothing to tooling, repeated by the engine".to_owned(),
      "   └─ mark   caption, medium, foreground at 0.12, turned".to_owned(),
    ];
    let reset = {
      let (text, angle, gap, media) = (
        self.text.clone(),
        self.angle.clone(),
        self.gap.clone(),
        self.media.clone(),
      );
      Arc::new(move || {
        text.set(MARK.to_owned());
        angle.set(0);
        gap.set(0);
        media.set(false);
      })
    };
    self.playground.card(
      ctx,
      Spec {
        title: "Watermark",
        make: Arc::new(made),
        shown: Shown::of(watermark),
        fill: false,
        forceable: false,
        facts: format!("angle {}", ANGLES[angle].0),
        event: "",
        code: call,
        tree,
        controls,
        changed: [text != MARK, angle != 0, gap != 0, media]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset,
      },
    )
  }
}

impl Component for WatermarkPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      text: ctx.signal(MARK.to_owned()),
      angle: ctx.signal(0),
      gap: ctx.signal(0),
      media: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let board = Row::new()
      .width(FILL)
      .spacing(24.0)
      .wrap()
      .child(Watermark::new(MARK).child(board_card()));
    let paragraphs = (1..=60).map(|index| {
      muted(
        &format!(
          "{index}. A paragraph of a long document. The marks lie over every line of it, and only those in \
           view are painted as it scrolls."
        ),
        TypographyStyle::Body,
      )
      .width(FILL)
    });
    let long = ScrollVertical::new(
      Watermark::new("ivan@lurq.dev · 16 Nov 2026").child(
        Column::new()
          .width(FILL)
          .padding(20.0)
          .spacing(12.0)
          .with_children(paragraphs),
      ),
    )
    .width(FILL)
    .height(280.0)
    .rounded(RadiusSize::Lg)
    .background(PaletteColor::Background)
    .border(line());
    let picture = Row::new().width(FILL).spacing(24.0).wrap().child(
      Watermark::new("lurq")
        .media(logo())
        .gap(32.0, 32.0)
        .child(report(380.0)),
    );
    let rows = [
      (
        "new(text)",
        "Watermark",
        "",
        "The marks of a text, over what `child` gives it.",
      ),
      (
        "child, children",
        "impl Into<Element>",
        "",
        "What it covers. The watermark is as large as that, and the marks are cut to it.",
      ),
      (
        "media",
        "impl Into<Element>",
        "",
        "A picture repeated in place of the text, turned and faint.",
      ),
      (
        "gap(x, y)",
        "f32, f32",
        "27, 29",
        "The room between one mark's box and the next, across and down, before the marks are turned.",
      ),
      (
        "angle",
        "f32",
        "-20",
        "Degrees, clockwise where more than nothing: the board's marks rise to the right.",
      ),
      (
        "offset(x, y)",
        "f32, f32",
        "-20, 18",
        "Where the first mark's box is from the top left.",
      ),
      ("stagger", "f32", "40", "How much further along every other row is."),
      (
        "id, key, style, rules, state",
        "",
        "",
        "As every themed builder has them; the mark's type and faintness are \
       its part's.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Media", "Watermark"],
        title: "Watermark",
        description: "A text or a picture repeated across what it covers, turned and faint, over the content: it \
                      takes no pointer and tooling does not read it.",
        import: Some("use lurq::ui::Watermark;"),
        status: Some("Stable"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "The text, the angle, the gap, and a picture in place of the text, over a card of a report.",
          playground,
        ),
        Section::new(
          "On a card",
          "The board's: \"lurq internal\" over a card of 220 by 120.",
          self.frame.demo(
            0,
            board,
            "Watermark::new(\"lurq internal\").child(card)",
            &["Watermark::new(\"lurq internal\").child(card)"],
          ),
        ),
        Section::new(
          "Over a long page",
          "Inside a scroll the marks are the page's and move with it; only those in view are painted.",
          self.frame.demo(
            1,
            long,
            "ScrollVertical::new(Watermark::new(who).child(page))",
            &[
              "ScrollVertical::new(",
              "    Watermark::new(\"ivan@lurq.dev · 16 Nov 2026\").child(page),",
              ")",
            ],
          ),
        ),
        Section::new(
          "A picture",
          "A logo repeated in place of a text.",
          self.frame.demo(
            2,
            picture,
            "Watermark::new(\"lurq\").media(logo).gap(32.0, 32.0)",
            &["Watermark::new(\"lurq\").media(logo).gap(32.0, 32.0).child(report)"],
          ),
        ),
        Section::new(
          "Props",
          "What a watermark is made with and given.",
          props_table(&rows, &["new(text)"], self.frame.room()),
        ),
      ],
    )
  }
}
