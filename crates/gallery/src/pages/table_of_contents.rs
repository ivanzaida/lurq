//! The page of the table of contents: its playground beside a small article that scrolls, the board's list, and
//! what the app's signal has of it.

use std::sync::Arc;

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{PaletteColor, RadiusSize, TypographyStyle},
  },
  components::{Column, Row, ScrollVertical, Text},
  core::{ElementRef, Signal},
  layout::Alignment,
  node::{Element, border::Border, dimension::Dimension},
  ui::{TableOfContents, TableOfContentsEntry},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, fit, props_table, quiet},
  playground::{Playground, Shown, Spec, field, flag, prop},
};

/// The board's entries (`i7m0J`), the level of each, and what its section says.
const SECTIONS: [(&str, usize, &str); 5] = [
  (
    "Runtime theme",
    1,
    "A window reads its theme while it runs: a change of the palette is drawn in the next frame, with no restart \
     and no rebuild of what is on it.",
  ),
  (
    "Palette",
    1,
    "Forty-two roles, light and dark: a component names a role and never a color, so a theme an app makes is \
     drawn by every component at once.",
  ),
  (
    "Extra roles",
    2,
    "An app adds roles of its own beside lurq's: a brand color, the series of a chart. A recipe reads them as it \
     reads the others.",
  ),
  (
    "Typography",
    1,
    "A type scale by role: a heading, a body, a caption, the mono face of code. A person's text scale multiplies \
     every one of them.",
  ),
  (
    "Font weight",
    2,
    "Regular for what is read, medium for what is pressed, semibold for what names a thing. A theme changes each \
     with its role.",
  ),
];

/// How high the article of a stage is: its sections scroll in it.
const ARTICLE: f32 = 260.0;

/// A small article: the sections of [`SECTIONS`], each with the ref of its entry on it, in a scroll of its own.
fn article(anchors: &[ElementRef], scroll: &ElementRef, key: &str) -> Element {
  let mut sections = Column::new().width(FILL).padding(16.0).spacing(20.0);
  for ((title, level, body), anchor) in SECTIONS.iter().zip(anchors) {
    sections = sections.child(
      Column::new()
        .width(FILL)
        .spacing(6.0)
        .ref_element(anchor.clone())
        .child(Text::new(title).variant(if *level == 1 {
          TypographyStyle::Heading3
        } else {
          TypographyStyle::Heading4
        }))
        .child(
          Text::new(body)
            .variant(TypographyStyle::Prose)
            .color(PaletteColor::MutedForeground)
            .width(FILL),
        )
        // The room a section of a real page has: what makes the article scroll.
        .child(
          Column::new()
            .width(FILL)
            .height(72.0)
            .rounded(RadiusSize::Md)
            .background(PaletteColor::Muted),
        ),
    );
  }
  ScrollVertical::new(sections)
    .key(key.to_owned())
    .flex(1.0)
    .min_width(Dimension::Px(0.0))
    .height(ARTICLE)
    .rounded(RadiusSize::Md)
    .border(Border::inside(1.0, PaletteColor::Border))
    .ref_element(scroll.clone())
    .into()
}

/// The entries of the board's list, on `anchors`.
fn entries(anchors: &[ElementRef], levels: bool, disabled: bool) -> Vec<TableOfContentsEntry> {
  SECTIONS
    .iter()
    .zip(anchors)
    .enumerate()
    .map(|(index, ((title, level, _), anchor))| {
      TableOfContentsEntry::new(title, anchor)
        .level(if levels { *level } else { 1 })
        .disabled(disabled && index == 3)
    })
    .collect()
}

/// Five refs, made once: the sections of one article.
fn anchors() -> Arc<Vec<ElementRef>> {
  Arc::new(SECTIONS.iter().map(|_| ElementRef::new()).collect())
}

pub struct TableOfContentsPage {
  frame: Frame,
  playground: Playground,
  /// The sections of the playground's article, and what scrolls them.
  play_anchors: Arc<Vec<ElementRef>>,
  play_scroll: ElementRef,
  /// The sections of the article of "The app's signal", and what scrolls them.
  bound_anchors: Arc<Vec<ElementRef>>,
  bound_scroll: ElementRef,
  /// The sections the still list of the board's section names: on nothing.
  still_anchors: Arc<Vec<ElementRef>>,
  title: Signal<String>,
  reading_line: Signal<String>,
  levels: Signal<bool>,
  disabled: Signal<bool>,
  current: Signal<usize>,
}

impl TableOfContentsPage {
  /// The playground: the list as the panel's props make it, beside an article that scrolls.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (title, levels, disabled) = (self.title.get(), self.levels.get(), self.disabled.get());
    let line_text = self.reading_line.get();
    let line = line_text.trim().parse::<f32>().unwrap_or(24.0);
    let forced = self.playground.forced();
    let record = self.playground.recorder();
    let (anchors, scroll) = (self.play_anchors.clone(), self.play_scroll.clone());
    let build = {
      let (title, anchors, scroll) = (title.clone(), anchors.clone(), scroll.clone());
      move || -> TableOfContents {
        TableOfContents::new(entries(&anchors, levels, disabled))
          .title(&title)
          .container(&scroll)
          .reading_line(line)
      }
    };
    // In a window of its own the list stands alone, still: there is no article beside it to follow.
    let make = {
      let build = build.clone();
      move || -> Element { build().into() }
    };
    let mut code = vec![
      "// theme, palette, roles, type, weight: ElementRef, each on its section; page on what scrolls them".to_owned(),
      "TableOfContents::new([(\"Runtime theme\", &theme), (\"Palette\", &palette)])".to_owned(),
    ];
    code.push(if levels {
      "    .entry(TableOfContentsEntry::new(\"Extra roles\", &roles).level(2))".to_owned()
    } else {
      "    .entry((\"Extra roles\", &roles))".to_owned()
    });
    code.push(if disabled {
      "    .entry(TableOfContentsEntry::new(\"Typography\", &type).disabled(true))".to_owned()
    } else {
      "    .entry((\"Typography\", &type))".to_owned()
    });
    code.push(if levels {
      "    .entry(TableOfContentsEntry::new(\"Font weight\", &weight).level(2))".to_owned()
    } else {
      "    .entry((\"Font weight\", &weight))".to_owned()
    });
    if title != "On this page" {
      code.push(format!("    .title({title:?})"));
    }
    code.push("    .container(&page)".to_owned());
    if line != 24.0 {
      code.push(format!("    .reading_line({line:?})"));
    }
    code.push("    .on_change(|index| ..)".to_owned());
    code.push("    .on_select(|index| ..)".to_owned());
    code.push("    .mount(ctx)".to_owned());
    let tree = vec![
      format!(
        "root       {}   entries 2 apart · one stop of Tab",
        self.playground.measured()
      ),
      "├─ title   mono 11 · capitals · MutedForeground · 12 in, 6 over the entries".to_owned(),
      "├─ entry   28 high · line 1 Border · label 13 MutedForeground, 12 in".to_owned(),
      "├─ entry   the one read: line 2 Primary · label Foreground medium".to_owned(),
      "└─ entry   of the second level: 12 further in".to_owned(),
    ];
    let reset = {
      let (title, reading_line) = (self.title.clone(), self.reading_line.clone());
      let (levels, disabled) = (self.levels.clone(), self.disabled.clone());
      Arc::new(move || {
        title.set("On this page".to_owned());
        reading_line.set("24".to_owned());
        levels.set(true);
        disabled.set(false);
      })
    };
    let shown = {
      let (build, playground) = (build.clone(), &self.playground);
      Shown::built(move |ctx| {
        let (changed, selected) = (record.clone(), record.clone());
        let contents = build()
          .force(forced)
          .on_change(move |index| changed(format!("on_change({index})")))
          .on_select(move |index| selected(format!("on_select({index})")))
          .mount(ctx);
        let scene = Row::new()
          .width(FILL)
          .spacing(24.0)
          .align_items(Alignment::Start)
          .child(article(&anchors, &scroll, "play-article"))
          .child(playground.measure(contents));
        // The article and the list beside it need 420: on a narrower stage the two scroll sideways.
        fit("contents-play", playground.stage_room(), 420.0, scene)
      })
      .measured()
    };
    self.playground.card(
      ctx,
      Spec {
        title: "Table of Contents",
        make: Arc::new(make),
        shown,
        fill: true,
        forceable: true,
        facts: "follows the article's scroll".to_owned(),
        event: "on_change",
        code,
        tree,
        controls: vec![
          prop("title", "&str", field(&self.title)),
          prop("reading_line", "f32", field(&self.reading_line)),
          flag("entry · level(2)", &self.levels),
          flag("entry · disabled", &self.disabled),
        ],
        changed: [title != "On this page", line != 24.0, !levels, disabled]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset,
      },
    )
  }
}

impl Component for TableOfContentsPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      play_anchors: anchors(),
      play_scroll: ElementRef::new(),
      bound_anchors: anchors(),
      bound_scroll: ElementRef::new(),
      still_anchors: anchors(),
      title: ctx.signal("On this page".to_owned()),
      reading_line: ctx.signal("24".to_owned()),
      levels: ctx.signal(true),
      disabled: ctx.signal(false),
      current: ctx.signal(0),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    // The board's list: still, with the second entry marked as the board draws it.
    let board = Column::new()
      .max_width(FILL)
      .spacing(10.0)
      .child(TableOfContents::new(entries(&self.still_anchors, true, false)).current(Signal::new(1)))
      .child(quiet("the board's list · still: converted with .into()", 12.0));
    // The app's signal has the entry: the line under the article says what it holds.
    let current = self.current.get();
    let bound = Column::new()
      .width(FILL)
      .spacing(12.0)
      .child(
        Row::new()
          .width(FILL)
          .spacing(24.0)
          .align_items(Alignment::Start)
          .child(article(&self.bound_anchors, &self.bound_scroll, "bound-article"))
          .child(
            TableOfContents::new(entries(&self.bound_anchors, true, false))
              .title("In this guide")
              .container(&self.bound_scroll)
              .current(self.current.clone())
              .mount(ctx),
          ),
      )
      .child(quiet(
        &format!("the signal has: {current} · {}", SECTIONS[current.min(4)].0),
        12.0,
      ));
    // The article and the list beside it need 420: in a narrower card the two scroll sideways.
    let bound = fit("contents-bound", self.frame.stage_room(), 420.0, bound);
    let rows = [
      (
        "new",
        "impl IntoIterator<Item = impl Into<TableOfContentsEntry>>",
        "",
        "Its entries: TableOfContentsEntry::new(label, &anchor), or a pair of a label and the ref of a section. \
         entry and entries add more.",
      ),
      (
        "mount",
        "&mut Ctx",
        "",
        "The list where it is declared, following the scroll and going to a section when its entry is pressed. \
         Converted with .into() it is still.",
      ),
      (
        "container",
        "&ElementRef",
        "",
        "What scrolls the sections: the reading line is under its top. With none, under the top of the window.",
      ),
      (
        "reading_line",
        "impl Into<f32>",
        "24",
        "How far under the top of the container a section's top is when the section becomes the one read.",
      ),
      (
        "current",
        "Signal<usize>",
        "",
        "The entry marked, the app's: set by the scroll and by a press; what the app sets is marked.",
      ),
      (
        "on_change",
        "Fn(usize)",
        "",
        "Another section is the one read, by the scroll or a press. Not told when the app sets the signal.",
      ),
      (
        "on_select",
        "Fn(usize)",
        "",
        "An entry was pressed, before the list goes to its section.",
      ),
      (
        "title",
        "&str",
        "On this page",
        "What the list is called over its entries, drawn in capitals. An empty one draws none.",
      ),
      (
        "label",
        "&str",
        "",
        "What it is called to tooling, in place of its title: a navigation of links.",
      ),
      (
        "TableOfContentsEntry::level",
        "usize",
        "1",
        "How deep the section is: an entry of the second level is 12 further in.",
      ),
      (
        "TableOfContentsEntry::disabled",
        "bool",
        "false",
        "The entry takes no input; the keys pass it by.",
      ),
    ];
    // The playground's article and its list need 420 too.
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Navigation", "Table of Contents"],
        title: "Table of Contents",
        description: "The list of a page's sections, with the one being read marked: it follows the scroll of the \
                      container the app names, and a press of an entry brings its section to the top. The list at \
                      the right of this page is one.",
        import: Some("use lurq::ui::{TableOfContents, TableOfContentsEntry};"),
        status: Some("Beta"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Scroll the article, or press an entry: the list marks the section whose top has come up to the reading \
           line, and the Events tab says each change.",
          playground,
        ),
        Section::new(
          "Table of Contents",
          "The board's list: a title in capitals, entries of 28 with a line at their start, the one read with a \
           line of 2 in the primary color and its label at medium weight, an entry of the second level set in.",
          self.frame.demo(
            0,
            board,
            "TableOfContents::new(..)",
            &[
              "TableOfContents::new([(\"Runtime theme\", &theme), (\"Palette\", &palette)])",
              "    .entry(TableOfContentsEntry::new(\"Extra roles\", &roles).level(2))",
              "    .entry((\"Typography\", &type))",
              "    .entry(TableOfContentsEntry::new(\"Font weight\", &weight).level(2))",
              "    .current(Signal::new(1))",
            ],
          ),
        ),
        Section::new(
          "The app's signal",
          "Given a signal, the list sets it as the article scrolls and as an entry is pressed, and marks what the \
           app sets: a breadcrumb or a title bar can say where the reader is.",
          self.frame.demo(
            1,
            bound,
            ".current(signal)",
            &[
              "// current: Signal<usize>; page: the ref of what scrolls the sections",
              "TableOfContents::new(entries)",
              "    .title(\"In this guide\")",
              "    .container(&page)",
              "    .current(current.clone())",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a table of contents and its entries are given.",
          props_table(&rows, &["new"], self.frame.room()),
        ),
      ],
    )
  }
}
