//! The frame every page of the gallery is in: a header that says what the
//! page is, its sections one under another, and on the right the list of
//! them, which stays in view while the page scrolls.
//!
//! The sizes here are the design's (`pencil/lurq-design-system-demo.pen`).

#[cfg(not(target_arch = "wasm32"))]
use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    ctx::Ctx,
    theme::{Breakpoint, PaletteColor, RadiusSize, TypographyStyle},
  },
  components::{Column, Row, Text},
  core::{ElementRef, Signal},
  layout::{
    Alignment,
    layout_kind::{Justify, Sticky},
    text_style::FontWeight,
  },
  node::{Element, border::Border},
  ui::{Badge, BadgeVariant, Button, ButtonSize, ButtonVariant, Length, TableOfContents, TableOfContentsPart},
};

use crate::kit::{FILL, call_line, code_block, icon, icon_tool, line, mono, phone, stage_padding, styled, text, tool};

/// How wide the list of sections is.
const ON_THIS_PAGE: f32 = 216.0;
/// The widest the column of a page's content gets.
const CONTENT: f32 = 880.0;
/// The widest a page's name and description get, beside the page's actions.
const TITLE_BLOCK: f32 = 620.0;
/// How far under the top of the page's scroll a section's top has to be for the section to be the one that is
/// read: a few lines into the page.
const READING_LINE: f32 = 140.0;
/// The same from the top of the window, where nothing says what scrolls the page: under the title bar and the tabs.
const READING_LINE_IN_WINDOW: f32 = 240.0;

/// What scrolls the pages of a tab, said by the tab to the pages in it: the list of a page's sections follows it.
#[derive(Clone)]
pub struct PageScroll(pub ElementRef);

/// What a page says of itself above its sections.
pub struct Header {
  /// The way to the page: "Components", "Actions", "Button".
  pub breadcrumb: &'static [&'static str],
  pub title: &'static str,
  pub description: &'static str,
  /// The `use` line of what the page shows.
  pub import: Option<&'static str>,
  /// "Stable", "Preview".
  pub status: Option<&'static str>,
  /// The version of lurq what the page shows is in since: "0.62".
  pub since: Option<&'static str>,
  /// The file the page is written in, from the root of the repository: `file!()`.
  pub source: &'static str,
}

/// A section of a page: a heading, a line of guidance and what it shows.
pub struct Section {
  pub title: &'static str,
  pub guidance: &'static str,
  pub body: Element,
}

impl Section {
  pub fn new(title: &'static str, guidance: &'static str, body: impl Into<Element>) -> Self {
    Self {
      title,
      guidance,
      body: body.into(),
    }
  }
}

/// What a page keeps between renders for its frame: where its sections are, and how wide one of them is. The list
/// of the sections is a `ui::TableOfContents`, which keeps the one that is read itself.
#[derive(Clone)]
pub struct Frame {
  anchors: Arc<Mutex<Vec<ElementRef>>>,
  /// The demo cards of the page whose code is shown: a bit for each, by its number.
  shown_code: Signal<u32>,
  /// How wide a section of the page is, measured: the room a card has.
  room: Signal<f32>,
}

impl Frame {
  /// A frame for a page: call it where the page is created.
  pub fn new(ctx: &mut Ctx) -> Self {
    Self {
      anchors: Arc::default(),
      shown_code: ctx.signal(0),
      room: ctx.signal(0.0),
    }
  }

  /// How wide a section of the page is: the room a card of it has. Nothing until the page was laid out
  /// once. What cannot be narrower than some width asks, and scrolls sideways where there is less
  /// (`kit::fit`).
  pub fn room(&self) -> f32 {
    self.room.get()
  }

  /// How wide the stage of a demo card is: the card less its edges and the room around what it shows. Nothing
  /// until the page was laid out once. What a stage shows that cannot be narrower than some width asks, and
  /// scrolls sideways where there is less (`kit::fit`).
  pub fn stage_room(&self) -> f32 {
    let room = self.room.get();
    if room <= 0.0 {
      0.0
    } else {
      (room - 2.0 - 2.0 * stage_padding()).max(1.0)
    }
  }

  /// A demo card: what is shown, live, and under it the call that makes one of it. "Show code" opens the
  /// Rust that builds what the stage shows, and Copy puts it on the clipboard. `number` tells the cards of a
  /// page apart: each keeps whether its code is shown.
  pub fn demo(&self, number: u32, shown: impl Into<Element>, call: &str, code: &[&str]) -> Element {
    let bit = 1u32 << number;
    let open = self.shown_code.get() & bit != 0;
    let toggle = self.shown_code.clone();
    let copied = code.join("\n");
    let mut card = Column::new()
      .width(FILL)
      .clip()
      .rounded(RadiusSize::Lg)
      .background(PaletteColor::Card)
      .foreground(PaletteColor::CardForeground)
      .border(line())
      .child(Column::new().width(FILL).padding(stage_padding()).child(shown));
    if open {
      let lines: Vec<String> = code.iter().map(|line| (*line).to_owned()).collect();
      card = card.child(
        Column::new()
          .width(FILL)
          .border_top(line())
          .background(PaletteColor::Canvas)
          .child(code_block(&lines).key(format!("demo-code-{number}"))),
      );
    }
    card
      .child(
        Row::new()
          .width(FILL)
          .height(40.0)
          .padding_left(16.0)
          .padding_right(8.0)
          .align_items(Alignment::Center)
          .justify(Justify::SpaceBetween)
          .border_top(line())
          .spacing(12.0)
          // In a narrow card the call ends in an ellipsis where the tools begin: they are not pushed out of it.
          .child(call_line(call))
          .child(
            Row::new()
              .spacing(2.0)
              .align_items(Alignment::Center)
              .child(tool(if open { "Hide code" } else { "Show code" }, "code", move || {
                toggle.update(|shown| *shown ^= bit)
              }))
              .child(icon_tool("Copy", "copy", move || {
                lurq::clipboard::copy_to_clipboard(&copied);
              })),
          ),
      )
      .into()
  }

  /// The page: its header, its sections, and the list of them on the right
  /// where the window is wide enough for it.
  pub fn page(&self, ctx: &mut Ctx, header: Header, sections: Vec<Section>) -> Element {
    // The design is drawn for a desktop: 48 beside the content. A narrower window gets 32, and one as narrow
    // as a phone 16: what is left for the page there is what its cards need.
    let wide = ctx.breakpoint() >= Some(Breakpoint::Lg);
    let beside = if wide {
      48.0
    } else if phone() {
      16.0
    } else {
      32.0
    };
    let with_list = wide && sections.len() > 1;

    let mut anchors = self.anchors.lock().unwrap();
    while anchors.len() < sections.len() {
      anchors.push(ctx.element_ref());
    }
    // Every section is as wide as the first, which says how wide. The list of the sections watches their refs
    // beside this observer.
    if let Some(first) = anchors.first() {
      let room = self.room.clone();
      first.observe_rect(move |rect| {
        if (room.get() - rect.width).abs() >= 0.5 {
          room.set(rect.width);
        }
      });
    }

    // A wide window with a narrow column for the page has no room for the actions beside the title.
    let room = self.room.get();
    let roomy = room <= 0.0 || room >= ROOMY;
    let titles: Vec<&'static str> = sections.iter().map(|section| section.title).collect();
    let mut main = Column::new()
      .flex(1.0)
      .padding_top(32.0)
      .padding_horizontal(beside)
      .padding_bottom(64.0)
      .spacing(48.0)
      .child(page_header(header, wide && roomy).max_width(CONTENT));
    for (index, section) in sections.into_iter().enumerate() {
      main = main.child(
        Column::new()
          .width(FILL)
          .max_width(CONTENT)
          .spacing(16.0)
          .ref_element(anchors[index].clone())
          .child(
            Column::new()
              .width(FILL)
              .spacing(6.0)
              .child(styled(section.title, TypographyStyle::Title, |_| {}))
              .child(prose(section.guidance, 14.0).width(FILL)),
          )
          .child(section.body),
      );
    }

    let mut page = Row::new().width(FILL).align_items(Alignment::Start).child(main);
    if with_list {
      page = page.child(self.on_this_page(ctx, &titles, &anchors));
    }
    drop(anchors);
    page.into()
  }

  /// The list of the page's sections, a `ui::TableOfContents`: it marks the section that is read as the page
  /// scrolls, and a press of an entry goes to its section. It stays at the top of the view while the page scrolls
  /// under it.
  fn on_this_page(&self, ctx: &mut Ctx, titles: &[&'static str], anchors: &[ElementRef]) -> Column {
    let mut contents = TableOfContents::new(titles.iter().copied().zip(anchors))
      // As wide as the column it is in.
      .style(TableOfContentsPart::Root, |s| s.width(Length::Fill));
    // The tab the page is in says what scrolls it: the reading line is under the top of that.
    contents = match ctx.use_context::<PageScroll>() {
      Some(PageScroll(scroll)) => contents.container(&scroll).reading_line(READING_LINE),
      None => contents.reading_line(READING_LINE_IN_WINDOW),
    };
    Column::new()
      .width(ON_THIS_PAGE)
      .padding_top(36.0)
      .padding_right(16.0)
      .padding_bottom(32.0)
      .padding_left(8.0)
      .sticky(Sticky::top(0.0))
      .child(contents.mount(ctx))
  }
}

/// A paragraph that explains: a description, a line of guidance. Its lines are set apart for reading.
fn prose(content: &str, size: f32) -> Text {
  let size = lurq::ui::current_theme().scaled(size);
  styled(content, TypographyStyle::Body, |style| {
    style.font_size = size;
    style.line_height = 1.6;
  })
  .color(PaletteColor::MutedForeground)
}

/// Where the repository's files are shown on the web: master, the branch the gallery is published from.
const REPOSITORY_FILES: &str = "https://github.com/ivanzaida/lurq/blob/master";

/// What a page offers beside its name: its source, on GitHub, in the browser. Every build alike, a phone's too.
fn page_actions(source: &'static str) -> Row {
  let address = format!("{REPOSITORY_FILES}/{}", source.replace('\\', "/"));
  Row::new().spacing(8.0).align_items(Alignment::Center).child(
    Button::new("View source")
      .variant(ButtonVariant::Outline)
      .size(ButtonSize::Sm)
      .icon("code")
      .on_click(move |_| {
        let _ = lurq::app::open_url(&address);
      }),
  )
}

/// The least width of the page's column with the actions of its header beside the title.
const ROOMY: f32 = 760.0;

fn page_header(header: Header, wide: bool) -> Column {
  // A way longer than the page goes on to a second line between its steps.
  let mut way = Row::new()
    .width(FILL)
    .spacing(8.0)
    .wrap()
    .align_items(Alignment::Center);
  let last = header.breadcrumb.len().saturating_sub(1);
  for (index, step) in header.breadcrumb.iter().enumerate() {
    if index > 0 {
      way = way.child(icon("chevron-right", 14.0).color(PaletteColor::MutedForeground));
    }
    way = way.child(if index == last {
      text(step, 14.0, FontWeight::Medium)
    } else {
      text(step, 14.0, FontWeight::Normal).color(PaletteColor::MutedForeground)
    });
  }

  let title_size = lurq::ui::current_theme().scaled(32.0);
  let block = Column::new()
    .spacing(10.0)
    .child(styled(header.title, TypographyStyle::Heading, |style| {
      style.font_size = title_size
    }))
    .child(prose(header.description, 15.0).width(FILL));
  let actions = page_actions(header.source);
  // The actions stand at the end of the row, on the last line of the description; where there is no room
  // beside it, under it.
  let title: Element = if wide {
    Row::new()
      .width(FILL)
      .spacing(16.0)
      .align_items(Alignment::End)
      .justify(Justify::SpaceBetween)
      .child(block.flex(1.0).max_width(TITLE_BLOCK))
      .child(actions)
      .into()
  } else {
    Column::new()
      .width(FILL)
      .spacing(16.0)
      .child(block.width(FILL))
      .child(actions)
      .into()
  };

  // The facts go on to a second line where the page is narrower than they are, and a `use` longer than the
  // page breaks between its names: a column gives it the page's width to wrap in.
  let mut facts = Row::new()
    .width(FILL)
    .spacing(10.0)
    .wrap()
    .align_items(Alignment::Center);
  if let Some(import) = header.import {
    facts = facts.child(
      Column::new()
        .max_width(FILL)
        .padding_horizontal(5.0)
        .padding_vertical(1.0)
        .rounded(RadiusSize::Sm)
        .background(PaletteColor::Card)
        .border(Border::inside(1.0, PaletteColor::Border))
        .child(mono(import, 13.0)),
    );
  }
  if let Some(status) = header.status {
    facts = facts.child(Badge::new(status).variant(BadgeVariant::Success));
  }
  if let Some(since) = header.since {
    facts = facts.child(mono(&format!("since {since}"), 12.0).color(PaletteColor::MutedForeground));
  }
  let mut column = Column::new().width(FILL).spacing(16.0).child(way).child(title);
  if header.import.is_some() || header.status.is_some() || header.since.is_some() {
    column = column.child(facts);
  }
  column
}
