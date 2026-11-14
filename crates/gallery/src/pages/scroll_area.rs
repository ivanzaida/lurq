//! The page of the scroll area: its playground, the long body of a dialog, a list with its bar beside it, a
//! table's two bars, and when bars show.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx, theme::PaletteColor},
  components::{Column, Row},
  core::Signal,
  layout::text_style::FontWeight,
  node::Element,
  ui::{ScrollArea, ScrollAreaOrientation, ScrollAreaPart, ScrollAreaScrollbars},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, text},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

/// The ways it scrolls, as the code names them.
const ORIENTATIONS: [(ScrollAreaOrientation, &str, &str); 3] = [
  (ScrollAreaOrientation::Vertical, "vertical", "Vertical"),
  (ScrollAreaOrientation::Horizontal, "horizontal", "Horizontal"),
  (ScrollAreaOrientation::Both, "both", "Both"),
];

/// When the bars show, as the code names them.
const SCROLLBARS: [(ScrollAreaScrollbars, &str); 5] = [
  (ScrollAreaScrollbars::Auto, "Auto"),
  (ScrollAreaScrollbars::Always, "Always"),
  (ScrollAreaScrollbars::Hover, "Hover"),
  (ScrollAreaScrollbars::Scrolling, "Scrolling"),
  (ScrollAreaScrollbars::Never, "Never"),
];

/// The terms of the board's dialog with a scrolling body, paragraph by paragraph.
const TERMS: [&str; 6] = [
  "By installing lurq you agree to these terms. Read them through before you go on.",
  "The software is provided as it is, without a warranty of any kind, to the extent the law allows.",
  "You may copy, modify and distribute the software, in source or compiled, under the terms of its license.",
  "Contributions are accepted under the same license as the rest of the project, and are credited.",
  "The names of the project and of its authors may not be used to promote what is built with it.",
  "These terms are read under the law of the place the project is published from.",
];

/// The paragraphs of the terms, each as wide as the room, 10 apart as the board has them.
fn terms() -> Column {
  TERMS
    .iter()
    .fold(Column::new().width(FILL).spacing(10.0), |column, line| {
      column.child(
        text(line, 13.0, FontWeight::Normal)
          .color(PaletteColor::MutedForeground)
          .width(FILL),
      )
    })
}

/// A row of a list: a title over a line of detail, 52 high as the board's.
fn row(title: &str, detail: &str) -> Column {
  Column::new()
    .width(FILL)
    .height(52.0)
    .padding_horizontal(12.0)
    .padding_vertical(8.0)
    .spacing(2.0)
    .child(text(title, 14.0, FontWeight::Medium))
    .child(text(detail, 12.0, FontWeight::Normal).color(PaletteColor::MutedForeground))
}

/// A grid wider and higher than its room: a table's cells.
fn grid() -> Column {
  let mut grid = Column::new().spacing(4.0);
  for line in 0..14 {
    let mut cells = Row::new().spacing(4.0);
    for column in 0..9 {
      cells = cells.child(
        Column::new()
          .width(88.0)
          .height(28.0)
          .padding_horizontal(8.0)
          .background(PaletteColor::Muted)
          .child(text(
            &format!("R{} C{}", line + 1, column + 1),
            12.0,
            FontWeight::Normal,
          )),
      );
    }
    grid = grid.child(cells);
  }
  grid
}

pub struct ScrollAreaPage {
  frame: Frame,
  playground: Playground,
  orientation: Signal<usize>,
  scrollbars: Signal<usize>,
  gutter: Signal<bool>,
  label: Signal<String>,
}

impl ScrollAreaPage {
  /// The playground: the area as the panel's props make it, over the terms or over a grid.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (orientation, name, variant) = ORIENTATIONS[self.orientation.get().min(ORIENTATIONS.len() - 1)];
    let (scrollbars, bars) = SCROLLBARS[self.scrollbars.get().min(SCROLLBARS.len() - 1)];
    let (gutter, label) = (self.gutter.get(), self.label.get());
    let plain = {
      let label = label.clone();
      move || {
        let mut area = ScrollArea::new()
          .width(320.0)
          .height(160.0)
          .orientation(orientation)
          .scrollbars(scrollbars)
          .gutter(gutter);
        if !label.is_empty() {
          area = area.label(&label);
        }
        match orientation {
          // The text keeps clear of the bar, as the board's body has it.
          ScrollAreaOrientation::Vertical => area.style(ScrollAreaPart::Root, |s| s.padding_end(18.0)).child(terms()),
          ScrollAreaOrientation::Horizontal => area.child(
            TERMS
              .iter()
              .map(|line| text(line, 13.0, FontWeight::Normal).width(220.0))
              .fold(Row::new().spacing(12.0), Row::child),
          ),
          ScrollAreaOrientation::Both => area.child(grid()),
        }
      }
    };

    let mut code = vec!["ScrollArea::new()".to_owned(), "    .height(160.0)".to_owned()];
    if orientation != ScrollAreaOrientation::Vertical {
      code.push(format!("    .orientation(ScrollAreaOrientation::{variant})"));
    }
    if scrollbars != ScrollAreaScrollbars::Auto {
      code.push(format!("    .scrollbars(ScrollAreaScrollbars::{bars})"));
    }
    if gutter {
      code.push("    .gutter(true)".to_owned());
    }
    if !label.is_empty() {
      code.push(format!("    .label({label:?})"));
    }
    code.push("    .child(paragraphs)".to_owned());
    let tree = vec![
      format!("root      {}   scrolls {name}", self.playground.measured()),
      "├─ content".to_owned(),
      "└─ bar     lane 8 · thumb 6 · ring · 2 from the edge".to_owned(),
    ];

    let (to_orientation, to_scrollbars, to_gutter, to_label) = (
      self.orientation.clone(),
      self.scrollbars.clone(),
      self.gutter.clone(),
      self.label.clone(),
    );
    let orientations: Vec<(&str, usize)> = ORIENTATIONS
      .iter()
      .enumerate()
      .map(|(index, orientation)| (orientation.1, index))
      .collect();
    let kinds: Vec<(&str, usize)> = SCROLLBARS
      .iter()
      .enumerate()
      .map(|(index, kind)| (kind.1, index))
      .collect();
    let (choose, pick) = (self.orientation.clone(), self.scrollbars.clone());
    self.playground.card(
      ctx,
      Spec {
        title: "Scroll Area",
        make: Arc::new({
          let plain = plain.clone();
          move || plain().into()
        }),
        shown: Shown::of(plain()),
        fill: false,
        forceable: false,
        facts: format!("{name} · {}", bars.to_lowercase()),
        event: "",
        code,
        tree,
        controls: vec![
          prop(
            "orientation",
            "ScrollAreaOrientation",
            crate::kit::segmented_fill(&orientations, &self.orientation.get(), move |index| choose.set(index)),
          ),
          prop(
            "scrollbars",
            "ScrollAreaScrollbars",
            self
              .playground
              .select(0, &kinds, &self.scrollbars.get(), move |index| pick.set(index)),
          ),
          prop("label", "&str", field(&self.label)),
          divider(),
          flag("gutter", &self.gutter),
        ],
        changed: [
          orientation != ScrollAreaOrientation::Vertical,
          scrollbars != ScrollAreaScrollbars::Auto,
          gutter,
          !label.is_empty(),
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          to_orientation.set(0);
          to_scrollbars.set(0);
          to_gutter.set(false);
          to_label.set(String::new());
        }),
      },
    )
  }
}

impl Component for ScrollAreaPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 1),
      orientation: ctx.signal(0),
      scrollbars: ctx.signal(0),
      gutter: ctx.signal(false),
      label: ctx.signal(String::new()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    // The board's dialog with a scrolling body: the terms, the bar in the body's padding.
    let body = Column::new()
      .width(440.0)
      .padding_top(16.0)
      .padding_left(20.0)
      .padding_right(6.0)
      .child(
        ScrollArea::new()
          .height(184.0)
          .label("Terms of use")
          .style(ScrollAreaPart::Root, |s| s.padding_end(18.0))
          .child(terms()),
      );
    // The board's list: its rows, and the bar in a lane of its own beside them.
    let mut rows = ScrollArea::new().width(340.0).height(160.0).gutter(true);
    for (title, detail) in [
      ("Quarterly report.pdf", "Edited 2 hours ago"),
      ("Roadmap.md", "Edited yesterday"),
      ("Logo.svg", "Edited 3 days ago"),
      ("Invoices.xlsx", "Edited last week"),
      ("Notes.txt", "Edited last month"),
    ] {
      rows = rows.child(row(title, detail));
    }
    let both = ScrollArea::new()
      .id("both-ways")
      .width(480.0)
      .height(200.0)
      .orientation(ScrollAreaOrientation::Both)
      .child(grid());
    let hover = Row::new()
      .spacing(16.0)
      .child(
        ScrollArea::new()
          .width(200.0)
          .height(120.0)
          .scrollbars(ScrollAreaScrollbars::Hover)
          .child(terms()),
      )
      .child(
        ScrollArea::new()
          .width(200.0)
          .height(120.0)
          .scrollbars(ScrollAreaScrollbars::Scrolling)
          .child(terms()),
      );
    let rows_table = [
      ("new", "", "", "An empty area that scrolls down."),
      (
        "child, children",
        "impl Into<Element>",
        "",
        "What scrolls: one under another, or side by side across.",
      ),
      (
        "orientation",
        "ScrollAreaOrientation",
        "Vertical",
        "Which way it scrolls: Vertical, Horizontal, or Both, with both bars.",
      ),
      (
        "scrollbars",
        "ScrollAreaScrollbars",
        "Auto",
        "When its bars show: while it overflows, Always, under the pointer (Hover), only while it scrolls \
         (Scrolling), or Never.",
      ),
      (
        "gutter",
        "bool",
        "false",
        "Its bars in a lane of their own beside the content, not over its edge.",
      ),
      (
        "label",
        "&str",
        "",
        "A region of that name to tooling, and a stop of Tab whose arrows, page keys, Home and End scroll it.",
      ),
      (
        "scroll_state",
        "ScrollState",
        "",
        "The app's: where it is scrolled to, read and set.",
      ),
      (
        "width, height, max_height",
        "impl Into<Length>",
        "",
        "Its size; it is as large as its room without one.",
      ),
      (
        "state, style, rules, force, id, key",
        "",
        "",
        "As every component has them. Its bar is the theme's ScrollBarStyle.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Data", "Scroll Area"],
        title: "Scroll Area",
        description: "Content that scrolls in a room of its own, with the design's bar: a lane of 8 with a round \
                      thumb of 6 in the ring color, darker under the pointer.",
        import: Some("use lurq::ui::{ScrollArea, ScrollAreaOrientation, ScrollAreaScrollbars};"),
        status: Some("New"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of ScrollArea is a control on the right. Scroll the stage with the wheel, drag the thumb, or \
           give it a label, Tab to it and use the arrow keys.",
          playground,
        ),
        Section::new(
          "A long body",
          "The board's dialog with a scrolling body: the terms scroll in the room they are given, and the bar stands \
           in the body's padding. With a label it is a region Tab reaches, whose keys scroll it.",
          self.frame.demo(
            0,
            body,
            "ScrollArea::new().height(184.0).label(\"Terms of use\")",
            &[
              "ScrollArea::new()",
              "    .height(184.0)",
              "    .label(\"Terms of use\")",
              "    .child(paragraphs)",
            ],
          ),
        ),
        Section::new(
          "A list beside its bar",
          "The board's list: with a gutter the bar has a lane of its own, and the rows end before it.",
          self.frame.demo(
            1,
            rows,
            ".gutter(true)",
            &[
              "ScrollArea::new()",
              "    .width(340.0)",
              "    .height(160.0)",
              "    .gutter(true)",
              "    .children(rows)",
            ],
          ),
        ),
        Section::new(
          "Both ways",
          "A table's cells, wider and higher than their room: a bar at the end and one at the bottom, each with its \
           own state under the pointer.",
          self.frame.demo(
            2,
            both,
            ".orientation(ScrollAreaOrientation::Both)",
            &[
              "ScrollArea::new()",
              "    .height(200.0)",
              "    .orientation(ScrollAreaOrientation::Both)",
              "    .child(cells)",
            ],
          ),
        ),
        Section::new(
          "When bars show",
          "Hover: under the pointer and while it scrolls, then they fade. Scrolling: only while it scrolls, as a \
           system set to show scroll bars only when scrolling has them.",
          self.frame.demo(
            3,
            hover,
            ".scrollbars(ScrollAreaScrollbars::Hover)",
            &[
              "ScrollArea::new().scrollbars(ScrollAreaScrollbars::Hover)",
              "ScrollArea::new().scrollbars(ScrollAreaScrollbars::Scrolling)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a scroll area is given.",
          props_table(&rows_table, &[], self.frame.room()),
        ),
      ],
    )
  }
}
