//! The frame every page of the gallery is in: a header that says what the
//! page is, its sections one under another, and on the right the list of
//! them, which stays in view while the page scrolls.

use std::{
  sync::{Arc, Mutex},
  time::{Duration, Instant},
};

use lurq::{
  app::{
    ctx::Ctx,
    theme::{BorderSize, Breakpoint, PaletteColor, TypographyStyle},
  },
  components::{Column, Row, Text},
  core::{ElementRef, Signal},
  layout::{Alignment, layout_kind::Sticky, text_style::FontWeight},
  node::{CursorIcon, Element, border::Border},
};

use crate::kit::{FILL, badge, code, muted, status_badge, styled};

/// How wide the list of sections is.
const ON_THIS_PAGE: f32 = 216.0;
/// How far down the window a section's top has to be for the section to be the one that is read: under the
/// title bar and the header, a few lines into the page.
const READING_LINE: f32 = 240.0;
/// How long the list keeps the section that was clicked while the page scrolls to it.
const HELD: Duration = Duration::from_millis(250);
/// The widest the column of a page's content gets.
const CONTENT: f32 = 880.0;

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
    }
  }

  /// The page: its header, its sections, and the list of them on the right
  /// where the window is wide enough for it.
  pub fn page(&self, ctx: &mut Ctx, header: Header, sections: Vec<Section>) -> Element {
    // The padding of the main column grows with the window: 32, 48, 64.
    let padding = match ctx.breakpoint() {
      Some(Breakpoint::Xl) => 64.0,
      Some(Breakpoint::Lg) => 48.0,
      _ => 32.0,
    };
    let with_list = ctx.breakpoint() >= Some(Breakpoint::Lg) && sections.len() > 1;

    let mut anchors = self.anchors.lock().unwrap();
    while anchors.len() < sections.len() {
      anchors.push(ctx.element_ref());
    }
    // The list marks the section that is being read: the last one whose top the page has scrolled past the
    // reading line. A section says where it is whenever that changes, which scrolling does.
    for (index, anchor) in anchors.iter().enumerate().take(sections.len()) {
      let (tops, held, current) = (self.tops.clone(), self.held.clone(), self.current.clone());
      anchor.observe_rect(move |rect| {
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

    let titles: Vec<&'static str> = sections.iter().map(|section| section.title).collect();
    let mut main = Column::new()
      .flex(1.0)
      .max_width(CONTENT + padding * 2.0)
      .padding(padding)
      .spacing(48.0)
      .child(page_header(header));
    for (index, section) in sections.into_iter().enumerate() {
      main = main.child(
        Column::new()
          .width(FILL)
          .spacing(16.0)
          .ref_element(anchors[index].clone())
          .child(
            Column::new()
              .spacing(6.0)
              .child(styled(section.title, TypographyStyle::Title, |_| {}))
              .child(muted(section.guidance, TypographyStyle::Body).width(FILL)),
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
      .padding_top(40.0)
      .padding_right(24.0)
      .sticky(Sticky::top(0.0))
      .child(
        styled("ON THIS PAGE", TypographyStyle::Caption, |style| {
          style.letter_spacing = 1.2;
          style.weight = FontWeight::Medium;
        })
        .color(PaletteColor::MutedForeground)
        .height(28.0),
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
          // The section that is being read is marked with an edge in the primary color.
          .border_left(Border::inside(
            BorderSize::Md,
            if on { PaletteColor::Primary } else { PaletteColor::Border },
          ))
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
          .child(Text::new(title).variant(TypographyStyle::Label)),
      );
    }
    list
  }
}

fn page_header(header: Header) -> Column {
  let mut way = Row::new().spacing(8.0).align_items(Alignment::Center);
  let last = header.breadcrumb.len().saturating_sub(1);
  for (index, step) in header.breadcrumb.iter().enumerate() {
    if index > 0 {
      way = way.child(muted("›", TypographyStyle::Body));
    }
    way = way.child(if index == last {
      Text::new(step).variant(TypographyStyle::Body)
    } else {
      muted(step, TypographyStyle::Body)
    });
  }
  let mut facts = Row::new().spacing(10.0).align_items(Alignment::Center);
  if let Some(import) = header.import {
    facts = facts.child(
      Row::new()
        .height(22.0)
        .padding_horizontal(6.0)
        .align_items(Alignment::Center)
        .rounded(lurq::app::theme::RadiusSize::Sm)
        .background(PaletteColor::Muted)
        .child(code(import)),
    );
  }
  if let Some(status) = header.status {
    facts = facts.child(status_badge(status));
  }
  facts = facts.child(badge(&format!("lurq {}", lurq::VERSION)));
  Column::new()
    .width(FILL)
    .spacing(14.0)
    .child(way)
    .child(styled(header.title, TypographyStyle::Heading, |style| {
      style.font_size = 32.0
    }))
    .child(
      muted(header.description, TypographyStyle::Body)
        .width(FILL)
        .max_width(620.0),
    )
    .child(facts)
}
