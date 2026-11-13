//! The frame every page of the gallery is in: a header that says what the
//! page is, its sections one under another, and on the right the list of
//! them, which stays in view while the page scrolls.
//!
//! The sizes here are the design's (`pencil/lurq-design-system-demo.pen`).

use std::{
  path::{Path, PathBuf},
  sync::{Arc, Mutex},
  time::{Duration, Instant},
};

use lurq::{
  app::{
    ctx::Ctx,
    theme::{Breakpoint, PaletteColor, RadiusSize, TypographyStyle},
  },
  components::{Column, Row, ScrollHorizontal, Text},
  core::{ElementRef, Signal},
  layout::{
    Alignment,
    layout_kind::{Justify, Sticky},
    text_style::FontWeight,
  },
  node::{CursorIcon, Element, border::Border},
  ui::{Button, ButtonSize, ButtonVariant},
};

use crate::kit::{FILL, code_block, icon, icon_tool, line, mono, overline, status_badge, styled, text, tool};

/// How wide the list of sections is.
const ON_THIS_PAGE: f32 = 216.0;
/// The widest the column of a page's content gets.
const CONTENT: f32 = 880.0;
/// The widest a page's name and description get, beside the page's actions.
const TITLE_BLOCK: f32 = 620.0;
/// How far down the window a section's top has to be for the section to be the one that is read: under the
/// title bar and the header, a few lines into the page.
const READING_LINE: f32 = 240.0;
/// How long the list keeps the section that was clicked while the page scrolls to it.
const HELD: Duration = Duration::from_millis(250);
/// The design system's board, from the root of the repository.
const BOARD: &str = "pencil/lurq-design-system.pen";

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

/// What a page keeps between renders for its frame: where its sections are,
/// and the one the list of sections was asked to show.
#[derive(Clone)]
pub struct Frame {
  anchors: Arc<Mutex<Vec<ElementRef>>>,
  wanted: Arc<Mutex<Option<usize>>>,
  /// Bumped when a section is asked for, so that the page renders and scrolls to it.
  asked: Signal<u32>,
  current: Signal<usize>,
  /// Where the top of each section is in the window, as it was last laid out.
  tops: Arc<Mutex<Vec<f32>>>,
  /// Until when the section that was clicked stays the marked one, wherever the page scrolls to.
  held: Arc<Mutex<Option<Instant>>>,
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
      wanted: Arc::default(),
      asked: ctx.signal(0),
      current: ctx.signal(0),
      tops: Arc::default(),
      held: Arc::default(),
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
      .child(Column::new().width(FILL).padding(32.0).child(shown));
    if open {
      let lines: Vec<String> = code.iter().map(|line| (*line).to_owned()).collect();
      card = card.child(
        Column::new()
          .width(FILL)
          .border_top(line())
          .background(PaletteColor::Canvas)
          .child(
            ScrollHorizontal::new(code_block(&lines))
              .key(format!("demo-code-{number}"))
              .width(FILL),
          ),
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
          // In a narrow card the call is cut where the tools begin: they are not pushed out of it.
          .child(
            Row::new()
              .flex(1.0)
              .clip()
              .child(mono(call, 12.0).nowrap().color(PaletteColor::MutedForeground)),
          )
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
    // The design is drawn for a desktop: 48 beside the content. A narrower window gets 32.
    let wide = ctx.breakpoint() >= Some(Breakpoint::Lg);
    let beside = if wide { 48.0 } else { 32.0 };
    let with_list = wide && sections.len() > 1;

    let mut anchors = self.anchors.lock().unwrap();
    while anchors.len() < sections.len() {
      anchors.push(ctx.element_ref());
    }
    // The list marks the section that is being read: the last one whose top the page has scrolled past the
    // reading line. A section says where it is whenever that changes, which scrolling does.
    for (index, anchor) in anchors.iter().enumerate().take(sections.len()) {
      let (tops, held, current) = (self.tops.clone(), self.held.clone(), self.current.clone());
      let room = self.room.clone();
      anchor.observe_rect(move |rect| {
        // Every section is as wide as the first.
        if index == 0 && (room.get() - rect.width).abs() >= 0.5 {
          room.set(rect.width);
        }
        let mut tops = tops.lock().unwrap();
        if tops.len() <= index {
          tops.resize(index + 1, f32::MAX);
        }
        tops[index] = rect.y;
        if held.lock().unwrap().is_some_and(|until| Instant::now() < until) {
          return;
        }
        let read = tops.iter().rposition(|top| *top <= READING_LINE).unwrap_or(0);
        if current.get() != read {
          current.set(read);
        }
      });
    }
    // Reading the signal is what renders the page when a section is asked for.
    let _ = self.asked.get();
    if let Some(wanted) = self.wanted.lock().unwrap().take()
      && let Some(anchor) = anchors.get(wanted)
    {
      ctx.scroll_into_view(anchor);
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
    drop(anchors);

    let mut page = Row::new().width(FILL).align_items(Alignment::Start).child(main);
    if with_list {
      page = page.child(self.on_this_page(&titles));
    }
    page.into()
  }

  /// The list of the page's sections. It stays at the top of the view while the page scrolls under it.
  fn on_this_page(&self, titles: &[&'static str]) -> Column {
    let mut list = Column::new()
      .width(ON_THIS_PAGE)
      .padding_top(36.0)
      .padding_right(16.0)
      .padding_bottom(32.0)
      .padding_left(8.0)
      .spacing(2.0)
      .sticky(Sticky::top(0.0))
      .child(
        Row::new()
          .width(FILL)
          .height(21.0)
          .padding_left(12.0)
          .padding_bottom(6.0)
          .align_items(Alignment::Center)
          .child(overline("On this page")),
      );
    let current = self.current.get();
    for (index, title) in titles.iter().enumerate() {
      let on = index == current;
      let (wanted, asked, chosen) = (self.wanted.clone(), self.asked.clone(), self.current.clone());
      let held = self.held.clone();
      list = list.child(
        Row::new()
          .width(FILL)
          .height(28.0)
          .padding_left(12.0)
          .align_items(Alignment::Center)
          .cursor(CursorIcon::Pointer)
          // The section that is being read is marked with a heavier edge in the primary color.
          .border_left(if on {
            Border::inside(2.0, PaletteColor::Primary)
          } else {
            Border::inside(1.0, PaletteColor::Border)
          })
          .foreground(if on {
            PaletteColor::Foreground
          } else {
            PaletteColor::MutedForeground
          })
          .hovered(|style| style.foreground(PaletteColor::Foreground))
          .on_click(move |_| {
            *wanted.lock().unwrap() = Some(index);
            // A section near the end of a page cannot come up to the reading line: it is the marked one
            // because it was asked for, until the page is scrolled again.
            *held.lock().unwrap() = Some(Instant::now() + HELD);
            chosen.set(index);
            asked.update(|count| *count += 1);
          })
          .child(text(
            title,
            13.0,
            if on { FontWeight::Medium } else { FontWeight::Normal },
          )),
      );
    }
    list
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

/// The root of the repository the gallery was built from.
fn repository() -> Option<PathBuf> {
  Some(Path::new(env!("CARGO_MANIFEST_DIR")).parent()?.parent()?.to_path_buf())
}

/// Opens a file in what the system opens it with: a source file in an editor, a board in Pencil.
fn open(path: &Path) {
  let program = if cfg!(target_os = "windows") {
    "explorer"
  } else if cfg!(target_os = "macos") {
    "open"
  } else {
    "xdg-open"
  };
  let _ = std::process::Command::new(program).arg(path).spawn();
}

/// What a page offers beside its name: its source, and the design it is built from. Each is there when the
/// gallery runs from the repository, where the file is.
fn page_actions(source: &'static str) -> Row {
  let mut actions = Row::new().spacing(8.0).align_items(Alignment::Center);
  let Some(root) = repository() else {
    return actions;
  };
  let action = |label: &str, glyph: &str, file: PathBuf| {
    Button::new(label)
      .variant(ButtonVariant::Outline)
      .size(ButtonSize::Sm)
      .icon(glyph)
      .on_click(move |_| open(&file))
  };
  let file = root.join(source);
  if file.is_file() {
    actions = actions.child(action("View source", "code", file));
  }
  let board = BOARD.split('/').fold(root, |path, part| path.join(part));
  if board.is_file() {
    actions = actions.child(action("Open in Pencil", "external-link", board));
  }
  actions
}

/// The least width of the page's column with the actions of its header beside the title.
const ROOMY: f32 = 760.0;

fn page_header(header: Header, wide: bool) -> Column {
  let mut way = Row::new().spacing(8.0).align_items(Alignment::Center);
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

  let mut facts = Row::new().spacing(10.0).align_items(Alignment::Center);
  if let Some(import) = header.import {
    facts = facts.child(
      Row::new()
        .padding_horizontal(5.0)
        .padding_vertical(1.0)
        .align_items(Alignment::Center)
        .rounded(RadiusSize::Sm)
        .background(PaletteColor::Card)
        .border(Border::inside(1.0, PaletteColor::Border))
        .child(mono(import, 13.0)),
    );
  }
  if let Some(status) = header.status {
    facts = facts.child(status_badge(status));
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
