//! The page of sticky elements: what is held at an edge of the list it is in while the list scrolls, as CSS's
//! `position: sticky`. Its playground is a list whose section headers stick as its props say; its sections are the
//! demo's Sticky page: a footer that waits at the bottom for its place, and a table whose first row and first
//! column stay.

use std::sync::Arc;

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{PaletteColor, RadiusSize},
  },
  components::{Column, Row, ScrollBoth, ScrollVertical},
  core::Signal,
  layout::{Alignment, layout_kind::Sticky, text_style::FontWeight},
  node::{CursorIcon, Element},
  ui::Switch,
};

use super::kit::controls;
use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, line, props_table, quiet, text},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

const EDGES: [(&str, usize); 2] = [("top", 0), ("bottom", 1)];
const INSETS: [(&str, i32); 5] = [("0", 0), ("8", 8), ("16", 16), ("24", 24), ("40", 40)];
const SECTIONS: [(&str, [&str; 4]); 5] = [
  ("A", ["Ada", "Alan", "Alonzo", "Annie"]),
  ("B", ["Barbara", "Bjarne", "Brian", "Butler"]),
  ("C", ["Claude", "Clarence", "Cleve", "Corrado"]),
  ("D", ["Dana", "David", "Dennis", "Donald"]),
  ("E", ["Edsger", "Edgar", "Elizabeth", "Evelyn"]),
];
const NAMES: [&str; 12] = [
  "Ada",
  "Alan",
  "Barbara",
  "Bjarne",
  "Claude",
  "Dana",
  "Dennis",
  "Edsger",
  "Elizabeth",
  "Evelyn",
  "Frances",
  "Grace",
];
/// The columns of the table: the names, and nine weeks.
const WEEKS: usize = 9;

/// A list that scrolls, 320 wide where there is room for it.
fn list_frame(content: impl Into<Element>, height: f32) -> ScrollVertical {
  ScrollVertical::new(content)
    .width(320.0)
    .max_width(FILL)
    .height(height)
    .rounded(RadiusSize::Md)
    .background(PaletteColor::Background)
    .border(line())
}

/// The list of the playground, as its props say.
#[derive(Clone, Copy)]
struct Sections {
  bottom: bool,
  inset: i32,
  clear: bool,
}

impl Sections {
  /// The list; `record` hears a press of a header or of a row.
  fn build(self, record: Arc<dyn Fn(String) + Send + Sync>) -> Element {
    let sticky = if self.bottom {
      Sticky::bottom(self.inset as f32)
    } else {
      Sticky::top(self.inset as f32)
    };
    let mut list = Column::new().width(FILL);
    for (letter, names) in SECTIONS {
      let pressed = record.clone();
      let mut section = Column::new().width(FILL).child(
        Row::new()
          .width(FILL)
          .height(28.0)
          .padding_horizontal(12.0)
          .align_items(Alignment::Center)
          .background(PaletteColor::Muted)
          .border_bottom(line())
          .cursor(CursorIcon::Pointer)
          .sticky(sticky)
          .on_click(move |_| pressed(format!("on_click(header {letter})")))
          .child(text(letter, 12.0, FontWeight::Bold)),
      );
      for name in names {
        let pressed = record.clone();
        section = section.child(
          Row::new()
            .width(FILL)
            .height(36.0)
            .padding_horizontal(12.0)
            .align_items(Alignment::Center)
            .cursor(CursorIcon::Pointer)
            .hovered(|style| style.background(PaletteColor::StateHover))
            .on_click(move |_| pressed(format!("on_click({name})")))
            .child(text(name, 13.0, FontWeight::Normal)),
        );
      }
      list = list.child(section);
    }
    list_frame(list, 230.0).scrollbars_clear_of_sticky(self.clear).into()
  }

  fn code(self) -> Vec<String> {
    let sticky = if self.bottom {
      format!("Sticky::bottom({}.0)", self.inset)
    } else {
      format!("Sticky::top({}.0)", self.inset)
    };
    let mut code = vec![
      "let section = Column::new()".to_owned(),
      format!("    .child(Row::new().height(28.0).sticky({sticky}).child(Text::new(\"A\")))"),
      "    .child(Text::new(\"Ada\"))".to_owned(),
      "    // ..".to_owned(),
      String::new(),
      "ScrollVertical::new(Column::new().child(section) /* .. */)".to_owned(),
      "    .size(320.0, 230.0)".to_owned(),
    ];
    if self.clear {
      code.push("    .scrollbars_clear_of_sticky(true)".to_owned());
    }
    code
  }
}

pub struct StickyPage {
  frame: Frame,
  playground: Playground,
  /// An index into [`EDGES`].
  edge: Signal<usize>,
  inset: Signal<i32>,
  clear: Signal<bool>,
  /// How many presses the headers and the rows of the playground's list took.
  header_clicks: Signal<usize>,
  row_clicks: Signal<usize>,
  /// Whether the table's bars keep clear of its first row and column.
  table_clear: Signal<bool>,
}

impl StickyPage {
  fn play(&self, ctx: &mut Ctx) -> Element {
    let sections = Sections {
      bottom: self.edge.get() == 1,
      inset: self.inset.get(),
      clear: self.clear.get(),
    };
    let record: Arc<dyn Fn(String) + Send + Sync> = {
      let (recorder, headers, rows) = (
        self.playground.recorder(),
        self.header_clicks.clone(),
        self.row_clicks.clone(),
      );
      Arc::new(move |what: String| {
        if what.starts_with("on_click(header") {
          headers.update(|count| *count += 1);
        } else {
          rows.update(|count| *count += 1);
        }
        recorder(what);
      })
    };
    let tree = vec![
      format!("scroll vertical   {}", self.playground.measured()),
      "└─ column".to_owned(),
      "   ├─ section A".to_owned(),
      format!(
        "   │  ├─ header   sticky {} {}",
        if sections.bottom { "bottom" } else { "top" },
        sections.inset
      ),
      "   │  └─ 4 rows".to_owned(),
      "   └─ … 4 more sections".to_owned(),
    ];
    let (set_edge, set_inset) = (self.edge.clone(), self.inset.clone());
    let controls = vec![
      prop(
        "Sticky::",
        "the edge",
        self
          .playground
          .select(0, &EDGES, &self.edge.get(), move |value| set_edge.set(value)),
      ),
      prop(
        "inset",
        "f32",
        self
          .playground
          .select(1, &INSETS, &sections.inset, move |value| set_inset.set(value)),
      ),
      divider(),
      flag("scrollbars_clear_of_sticky", &self.clear),
    ];
    let reset = {
      let (edge, inset, clear, headers, rows) = (
        self.edge.clone(),
        self.inset.clone(),
        self.clear.clone(),
        self.header_clicks.clone(),
        self.row_clicks.clone(),
      );
      Arc::new(move || {
        edge.set(0);
        inset.set(0);
        clear.set(false);
        headers.set(0);
        rows.set(0);
      })
    };
    let shown = record.clone();
    self.playground.card(
      ctx,
      Spec {
        title: "Sticky",
        make: Arc::new(move || sections.build(Arc::new(|_| {}))),
        shown: Shown::built(move |_| sections.build(shown)),
        fill: true,
        forceable: false,
        facts: format!(
          "{} presses on a header · {} on a row",
          self.header_clicks.get(),
          self.row_clicks.get()
        ),
        event: "on_click",
        code: sections.code(),
        tree,
        controls,
        changed: [sections.bottom, sections.inset != 0, sections.clear]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset,
      },
    )
  }

  /// The table, with a switch over it for whether its bars keep clear of what stays.
  fn table(&self) -> Column {
    let cell = |content: &str, fill: PaletteColor, weight: FontWeight| {
      Row::new()
        .size(110.0, 34.0)
        .padding_horizontal(10.0)
        .align_items(Alignment::Center)
        .background(fill)
        .border_bottom(line())
        .child(text(content, 12.0, weight).nowrap())
    };
    let mut header = Row::new()
      .sticky(Sticky::top(0.0))
      .child(cell("Name", PaletteColor::Muted, FontWeight::Bold).sticky(Sticky::left(0.0)));
    for week in 1..=WEEKS {
      header = header.child(cell(&format!("Week {week}"), PaletteColor::Muted, FontWeight::Bold));
    }
    let mut table = Column::new().child(header);
    for (index, name) in NAMES.into_iter().enumerate() {
      let mut row = Row::new().child(cell(name, PaletteColor::Secondary, FontWeight::Medium).sticky(Sticky::left(0.0)));
      for week in 1..=WEEKS {
        row = row.child(cell(
          &((index * 7 + week * 3) % 40).to_string(),
          PaletteColor::Background,
          FontWeight::Normal,
        ));
      }
      table = table.child(row);
    }
    let clear = self.table_clear.clone();
    Column::new()
      .width(FILL)
      .spacing(16.0)
      .child(
        controls().child(
          Switch::new("Bars clear of what stays")
            .on(self.table_clear.get())
            .on_change(move |on| clear.set(on)),
        ),
      )
      .child(
        ScrollBoth::new(table)
          .scrollbars_clear_of_sticky(self.table_clear.get())
          .width(520.0)
          .max_width(FILL)
          .height(240.0)
          .rounded(RadiusSize::Md)
          .background(PaletteColor::Background)
          .border(line()),
      )
  }
}

/// Fourteen lines and a bar after them that waits at the bottom of the view for its place, then the rest of the
/// list.
fn footer() -> ScrollVertical {
  let mut section = Column::new().width(FILL);
  for index in 1..=14 {
    section = section.child(
      Row::new()
        .height(32.0)
        .padding_horizontal(12.0)
        .align_items(Alignment::Center)
        .child(text(&format!("Line {index}"), 13.0, FontWeight::Normal)),
    );
  }
  let section = section.child(
    Row::new()
      .width(FILL)
      .height(34.0)
      .padding_horizontal(12.0)
      .align_items(Alignment::Center)
      .background(PaletteColor::Primary)
      .sticky(Sticky::bottom(0.0))
      .child(text("3 selected", 12.0, FontWeight::Bold).color(PaletteColor::PrimaryForeground)),
  );
  let after = Column::new()
    .height(220.0)
    .padding(12.0)
    .child(quiet("After the section", 13.0));
  list_frame(Column::new().width(FILL).child(section).child(after), 200.0)
}

impl Component for StickyPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 2),
      edge: ctx.signal(0),
      inset: ctx.signal(0),
      clear: ctx.signal(false),
      header_clicks: ctx.signal(0),
      row_clicks: ctx.signal(0),
      table_clear: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let rows = [
      (
        "sticky",
        "Sticky",
        "",
        "Holds the element at an edge of the scroll container it is in while that scrolls. It keeps its room in \
         the flow and never leaves its parent's box.",
      ),
      (
        "Sticky::top / bottom / left / right",
        "f32",
        "",
        "The edge, and how far from it the element is held.",
      ),
      (
        "with_top / with_left / ..",
        "f32",
        "",
        "Another edge too: Sticky::top(0.0).with_left(0.0) holds the corner of a table that scrolls both ways.",
      ),
      (
        "scrollbars_clear_of_sticky",
        "bool",
        "false",
        "On the scroll container: its bars run beside what sticks and not over it.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Engine", "Sticky"],
        title: "Sticky",
        description: "An element held at an edge of the list it is in while the list scrolls, as CSS's position: \
                      sticky. It sticks in the scroll container it is in, not in the page's.",
        import: Some("use lurq::layout::layout_kind::Sticky;"),
        status: Some("Stable"),
        since: None,
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Scroll the list: a header stays at its edge until its section ends, and the next one pushes it out. \
           Press a stuck header where a row is under it: the header takes the press, the row does not.",
          playground,
        ),
        Section::new(
          "A footer",
          "The bar is the last line of its section. It is held at the bottom of the view while its place is below, \
           and is in its place once that scrolls in.",
          self.frame.demo(
            0,
            footer(),
            "Sticky::bottom(0.0)",
            &[
              "let section = Column::new()",
              "    .child(Text::new(\"Line 1\"))",
              "    // .. 13 more",
              "    .child(Row::new().height(34.0).sticky(Sticky::bottom(0.0)).child(Text::new(\"3 selected\")));",
              "",
              "ScrollVertical::new(Column::new().child(section).child(after)).size(320.0, 200.0)",
            ],
          ),
        ),
        Section::new(
          "A table",
          "Scroll down and sideways: the names stay on the left and the weeks on top, and the corner cell stays in \
           the corner, over both. With the switch on, the bars run beside them.",
          self.frame.demo(
            1,
            self.table(),
            "Sticky::top(0.0)",
            &[
              "let header = Row::new()",
              "    .sticky(Sticky::top(0.0))",
              "    .child(cell(\"Name\").sticky(Sticky::left(0.0)))",
              "    .child(cell(\"Week 1\"))",
              "    // ..",
              "let row = Row::new()",
              "    .child(cell(\"Ada\").sticky(Sticky::left(0.0)))",
              "    .child(cell(\"3\"))",
              "    // ..",
              "",
              "ScrollBoth::new(Column::new().child(header).child(row) /* .. */)",
              "    .scrollbars_clear_of_sticky(true)",
              "    .size(520.0, 240.0)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What makes an element stick, and what the container it sticks in is told.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
