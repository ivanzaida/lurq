//! The page of how much of an element shows: `ElementRef::observe_visibility`, lurq's `IntersectionObserver`. Its
//! playground is a list whose rows say whether they are over the threshold its props give, against a viewport
//! grown by the margin they give; its sections are what the observer is for: the page of a feed that fills its
//! viewport, thumbnails that load near the viewport, and a mark at the end of a list that loads more.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{PaletteColor, RadiusSize},
  },
  components::{Column, Row, ScrollVertical, Spacer},
  core::{ElementRef, Signal, VisibilityObserver, VisibilityOptions},
  layout::{
    Alignment,
    layout_kind::{Justify, ScrollState},
    text_style::FontWeight,
  },
  node::Element,
  ui::{Button, ButtonSize, ButtonVariant},
};

use super::kit::{controls, samples};
use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, line, mono, props_table, quiet, text},
  playground::{Playground, Shown, Spec, divider, prop},
};

/// The shares the playground's observers are told of: each alone, as its one threshold.
const THRESHOLDS: [(&str, usize); 4] = [("0.0", 0), ("0.5", 1), ("0.99", 2), ("1.0", 3)];
const SHARES: [f32; 4] = [0.0, 0.5, 0.99, 1.0];
const MARGINS: [(&str, i32); 4] = [("0", 0), ("40", 40), ("120", 120), ("-40", -40)];
/// How many rows the playground's list has, and how high one is.
const ROWS: usize = 14;
const ROW_HEIGHT: f32 = 44.0;

/// A row of the playground's list: marked while it is over the threshold.
fn row(index: usize, over: bool) -> Row {
  Row::new()
    .width(FILL)
    .height(ROW_HEIGHT)
    .padding_horizontal(12.0)
    .align_items(Alignment::Center)
    .justify(Justify::SpaceBetween)
    .border_bottom(line())
    .background(if over {
      PaletteColor::Secondary
    } else {
      PaletteColor::Background
    })
    .child(text(&format!("Row {}", index + 1), 13.0, FontWeight::Medium))
    .child(quiet(if over { "over the threshold" } else { "" }, 12.0))
}

/// A list that scrolls down, 320 wide where there is room for it.
fn list_frame(content: impl Into<Element>, height: f32) -> ScrollVertical {
  ScrollVertical::new(content)
    .width(320.0)
    .max_width(FILL)
    .height(height)
    .rounded(RadiusSize::Md)
    .background(PaletteColor::Background)
    .border(line())
}

/// How many pages the feed has, and how large one is.
const PAGES: usize = 5;
const PAGE_WIDTH: f32 = 240.0;
const PAGE_HEIGHT: f32 = 150.0;

/// A feed that scrolls by pages, whose page that all but fills the viewport is the current one. A component of
/// its own: its refs and its observers live as long as it does.
struct Feed {
  state: ScrollState,
  pages: Vec<ElementRef>,
  current: Signal<usize>,
  /// How many times an observer was told.
  told: Signal<usize>,
  _watching: Vec<VisibilityObserver>,
}

impl Component for Feed {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let state = ScrollState::new();
    state.set_paging(true);
    let pages: Vec<ElementRef> = (0..PAGES).map(|_| ElementRef::new()).collect();
    let (current, told) = (ctx.signal(0), ctx.signal(0));
    let watching = pages
      .iter()
      .enumerate()
      .map(|(index, page)| {
        let (current, told) = (current.clone(), told.clone());
        page.observe_visibility(VisibilityOptions::new().thresholds([0.99]), move |seen| {
          told.update(|told| *told += 1);
          if seen.ratio >= 0.99 {
            current.set(index);
          }
        })
      })
      .collect();
    Self {
      state,
      pages,
      current,
      told,
      _watching: watching,
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    let current = self.current.get();
    let pages = Column::new().with_children(self.pages.iter().enumerate().map(|(index, page)| {
      Row::new()
        .size(PAGE_WIDTH, PAGE_HEIGHT)
        .align_items(Alignment::Center)
        .justify(Justify::Center)
        .background(if index.is_multiple_of(2) {
          PaletteColor::Muted
        } else {
          PaletteColor::Background
        })
        .ref_element(page.clone())
        .child(text(&format!("Page {index}"), 14.0, FontWeight::Bold))
    }));
    let next = {
      let state = self.state.clone();
      let to = (current + 1) % PAGES;
      Button::new("set_scroll: next page")
        .variant(ButtonVariant::Outline)
        .size(ButtonSize::Sm)
        .id("visibility-feed-next")
        .on_click(move |_| state.set_scroll(0.0, to as f32 * PAGE_HEIGHT))
    };
    Column::new()
      .width(FILL)
      .spacing(16.0)
      .child(controls().child(next))
      .child(
        samples()
          .child(
            ScrollVertical::new(pages)
              .with_scroll_state(self.state.clone())
              .id("visibility-feed")
              .size(PAGE_WIDTH, PAGE_HEIGHT)
              .rounded(RadiusSize::Md)
              .border(line()),
          )
          .child(
            Column::new()
              .spacing(6.0)
              .child(
                Row::new()
                  .spacing(8.0)
                  .child(quiet("the current page", 12.0))
                  .child(mono(&current.to_string(), 12.0).id("visibility-feed-current")),
              )
              .child(
                Row::new()
                  .spacing(8.0)
                  .child(quiet("observers told", 12.0))
                  .child(mono(&self.told.get().to_string(), 12.0).id("visibility-feed-told")),
              ),
          ),
      )
  }
}

/// How many thumbnails the grid has, how many in a line, and how far from the viewport one loads.
const THUMBS: usize = 48;
const THUMBS_ACROSS: usize = 4;
const THUMB: f32 = 48.0;
const THUMB_MARGIN: f32 = 80.0;

/// A grid of thumbnails, each loaded once it is within [`THUMB_MARGIN`] of the viewport, and kept.
struct Thumbs {
  tiles: Vec<ElementRef>,
  loaded: Signal<Vec<bool>>,
  _watching: Vec<VisibilityObserver>,
}

impl Component for Thumbs {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let tiles: Vec<ElementRef> = (0..THUMBS).map(|_| ElementRef::new()).collect();
    let loaded = ctx.signal(vec![false; THUMBS]);
    let watching = tiles
      .iter()
      .enumerate()
      .map(|(index, tile)| {
        let loaded = loaded.clone();
        tile.observe_visibility(VisibilityOptions::new().margin(THUMB_MARGIN), move |seen| {
          if seen.visible && !loaded.get_untracked()[index] {
            loaded.update(|loaded| loaded[index] = true);
          }
        })
      })
      .collect();
    Self {
      tiles,
      loaded,
      _watching: watching,
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    let loaded = self.loaded.get();
    let mut grid = Column::new().spacing(8.0).padding(8.0);
    for line_of in self.tiles.chunks(THUMBS_ACROSS).enumerate() {
      let (at, tiles) = line_of;
      grid = grid.child(
        Row::new()
          .spacing(8.0)
          .with_children(tiles.iter().enumerate().map(|(column, tile)| {
            let index = at * THUMBS_ACROSS + column;
            let mut tile = Row::new()
              .size(THUMB, THUMB)
              .align_items(Alignment::Center)
              .justify(Justify::Center)
              .rounded(RadiusSize::Sm)
              .border(line())
              .background(if loaded[index] {
                PaletteColor::Primary
              } else {
                PaletteColor::Muted
              })
              .ref_element(tile.clone());
            if loaded[index] {
              tile = tile.child(mono(&(index + 1).to_string(), 11.0).color(PaletteColor::PrimaryForeground));
            }
            tile
          })),
      );
    }
    let count = loaded.iter().filter(|loaded| **loaded).count();
    samples()
      .child(
        ScrollVertical::new(grid)
          .id("visibility-thumbs")
          .height(220.0)
          .rounded(RadiusSize::Md)
          .background(PaletteColor::Background)
          .border(line()),
      )
      .child(
        Row::new()
          .spacing(8.0)
          .child(quiet("loaded", 12.0))
          .child(mono(&format!("{count} of {THUMBS}"), 12.0).id("visibility-thumbs-loaded")),
      )
  }
}

/// How many rows the list that loads more starts with, how many it takes at a time, and the most it has.
const ENDLESS_START: usize = 12;
const ENDLESS_STEP: usize = 10;
const ENDLESS_MOST: usize = 62;

/// A list with a mark of no height at its end: when the mark comes into view, more rows come before it.
struct Endless {
  end: ElementRef,
  rows: Signal<usize>,
  _watching: VisibilityObserver,
}

impl Component for Endless {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let end = ElementRef::new();
    let rows = ctx.signal(ENDLESS_START);
    let watching = end.observe_visibility(VisibilityOptions::new(), {
      let rows = rows.clone();
      move |seen| {
        if seen.visible && rows.get_untracked() < ENDLESS_MOST {
          rows.update(|rows| *rows = (*rows + ENDLESS_STEP).min(ENDLESS_MOST));
        }
      }
    });
    Self {
      end,
      rows,
      _watching: watching,
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    let rows = self.rows.get();
    let list = Column::new()
      .width(FILL)
      .with_children((0..rows).map(|index| row(index, false)))
      .child(Spacer::new().width(FILL).height(0.0).ref_element(self.end.clone()));
    samples().child(list_frame(list, 200.0).id("visibility-endless")).child(
      Row::new()
        .spacing(8.0)
        .child(quiet("rows", 12.0))
        .child(mono(&format!("{rows} of {ENDLESS_MOST}"), 12.0).id("visibility-endless-rows")),
    )
  }
}

pub struct VisibilityPage {
  frame: Frame,
  playground: Playground,
  /// An index into [`SHARES`].
  threshold: Signal<usize>,
  margin: Signal<i32>,
  /// The rows of the playground's list, and whether each was last told over the threshold.
  rows: Vec<ElementRef>,
  over: Signal<Vec<bool>>,
  /// The observers of the rows, and the threshold and the margin they were added with: a render with others
  /// drops them and adds new ones.
  watching: Mutex<Option<((usize, i32), Vec<VisibilityObserver>)>>,
}

impl VisibilityPage {
  /// Watches the rows with the threshold and the margin the props give, where the observers there are were
  /// added with others.
  fn watch(&self, threshold: usize, margin: i32) {
    let mut watching = self.watching.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if watching.as_ref().is_some_and(|(with, _)| *with == (threshold, margin)) {
      return;
    }
    // Those of the props before are removed first.
    *watching = None;
    let share = SHARES[threshold];
    let recorder = self.playground.recorder();
    let observers = self
      .rows
      .iter()
      .enumerate()
      .map(|(index, row)| {
        let (over, recorder) = (self.over.clone(), recorder.clone());
        let options = VisibilityOptions::new().thresholds([share]).margin(margin as f32);
        row.observe_visibility(options, move |seen| {
          let is_over = seen.visible && seen.ratio >= share;
          if over.get_untracked()[index] != is_over {
            over.update(|over| over[index] = is_over);
          }
          recorder(format!("row {}: ratio {:.2}", index + 1, seen.ratio));
        })
      })
      .collect();
    *watching = Some(((threshold, margin), observers));
  }

  fn play(&self, ctx: &mut Ctx) -> Element {
    let (threshold, margin) = (self.threshold.get(), self.margin.get());
    self.watch(threshold, margin);
    let over = self.over.get();
    let list = {
      let rows = self.rows.clone();
      let over = over.clone();
      move || -> Element {
        let column = Column::new().width(FILL).with_children(
          rows
            .iter()
            .enumerate()
            .map(|(index, at)| row(index, over[index]).ref_element(at.clone())),
        );
        list_frame(column, 230.0).id("visibility-list").into()
      }
    };
    // The window of its own shows the list with no refs: a ref is one element's.
    let plain = || -> Element {
      let column = Column::new()
        .width(FILL)
        .with_children((0..ROWS).map(|index| row(index, false)));
      list_frame(column, 230.0).into()
    };
    let tree = vec![
      format!("scroll vertical   {}", self.playground.measured()),
      "└─ column".to_owned(),
      format!("   └─ {ROWS} rows   each with a ref and an observer"),
    ];
    let (set_threshold, set_margin) = (self.threshold.clone(), self.margin.clone());
    let controls = vec![
      prop(
        "thresholds",
        "[f32]",
        self
          .playground
          .select(0, &THRESHOLDS, &threshold, move |value| set_threshold.set(value)),
      ),
      divider(),
      prop(
        "margin",
        "f32",
        self
          .playground
          .select(1, &MARGINS, &margin, move |value| set_margin.set(value)),
      ),
    ];
    let reset = {
      let (threshold, margin) = (self.threshold.clone(), self.margin.clone());
      Arc::new(move || {
        threshold.set(0);
        margin.set(0);
      })
    };
    let share = THRESHOLDS[threshold].0;
    let mut code = vec![
      "// In create: a ref and an observer for each row, kept on the component.".to_owned(),
      "let row = ElementRef::new();".to_owned(),
      "let watching = row.observe_visibility(".to_owned(),
      if margin == 0 {
        format!("    VisibilityOptions::new().thresholds([{share}]),")
      } else {
        format!("    VisibilityOptions::new().thresholds([{share}]).margin({margin}.0),")
      },
      format!("    move |seen| over.set(seen.visible && seen.ratio >= {share}),"),
      ");".to_owned(),
      String::new(),
      "// In render.".to_owned(),
      "ScrollVertical::new(Column::new().child(Row::new().ref_element(row.clone())) /* .. */)".to_owned(),
      "    .size(320.0, 230.0)".to_owned(),
    ];
    if threshold == 0 {
      code[3] = if margin == 0 {
        "    VisibilityOptions::new(),".to_owned()
      } else {
        format!("    VisibilityOptions::new().margin({margin}.0),")
      };
      code[4] = "    move |seen| over.set(seen.visible),".to_owned();
    }
    self.playground.card(
      ctx,
      Spec {
        title: "Visibility",
        make: Arc::new(plain),
        shown: Shown::built(move |_| list()),
        fill: true,
        forceable: false,
        facts: format!(
          "{} of {ROWS} rows over the threshold",
          over.iter().filter(|over| **over).count()
        ),
        event: "observe_visibility",
        code,
        tree,
        controls,
        changed: [threshold != 0, margin != 0]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset,
      },
    )
  }
}

impl Component for VisibilityPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 2),
      threshold: ctx.signal(0),
      margin: ctx.signal(0),
      rows: (0..ROWS).map(|_| ElementRef::new()).collect(),
      over: ctx.signal(vec![false; ROWS]),
      watching: Mutex::new(None),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let rows = [
      (
        "ElementRef::observe_visibility",
        "VisibilityOptions, Fn(Visibility)",
        "",
        "Tells the observer how much of the element shows: at the first layout, and each time the share crosses a \
         threshold. Gives a VisibilityObserver; the observer is removed when that is dropped.",
      ),
      (
        "thresholds",
        "[f32]",
        "[0.0]",
        "The shares, 0 to 1, the observer is told of when the share that shows crosses one, either way. With \
         none: when the element comes into view, and when it leaves.",
      ),
      (
        "margin / margins",
        "f32 / f32, f32",
        "0",
        "Grows the root on every side, or across and down apart, before the element is measured: what is near \
         the viewport counts as showing. A negative one shrinks it.",
      ),
      (
        "root",
        "&ElementRef",
        "",
        "Measures against the box of an element around it. Without one: the scroll container nearest around the \
         element, as much of it as the window shows, or the window.",
      ),
      (
        "Visibility::ratio / visible",
        "f32 / bool",
        "",
        "The share of the element's box inside the root, and whether any of it is. An element of no size has 1 \
         where it lies inside the root.",
      ),
      (
        "Visibility::visible_rect / rect / root_rect",
        "ElementRect",
        "",
        "The part that shows, the element's box and the root it was measured against, in the window's logical \
         pixels.",
      ),
    ];
    let playground = self.play(ctx);
    let (feed, thumbs, endless) = (ctx.mount::<Feed>(()), ctx.mount::<Thumbs>(()), ctx.mount::<Endless>(()));
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Engine", "Visibility"],
        title: "Visibility",
        description: "How much of an element shows in the list it scrolls in, told when it crosses a share the app \
                      names: lurq's IntersectionObserver. Reckoned from the layout, so it is right whatever moved \
                      the content.",
        import: Some("use lurq::core::{ElementRef, Visibility, VisibilityObserver, VisibilityOptions};"),
        status: Some("New"),
        since: None,
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Scroll the list: a row is marked while the share of it that shows is over the threshold, measured \
           against the list's viewport grown by the margin. The events are what the observers were told: nothing \
           between two crossings.",
          playground,
        ),
        Section::new(
          "The page that shows",
          "A feed that scrolls by pages. Each page has an observer with a threshold of 0.99: the one that all but \
           fills the viewport is the current one, whether a finger brought it (run the gallery with --device phone), \
           the wheel, or the button, which sets the offset.",
          self.frame.demo(
            0,
            feed,
            "VisibilityOptions::new().thresholds([0.99])",
            &[
              "// In create, for each page.",
              "let page = ElementRef::new();",
              "let watching = page.observe_visibility(",
              "    VisibilityOptions::new().thresholds([0.99]),",
              "    move |seen| {",
              "        if seen.ratio >= 0.99 {",
              "            current.set(index);",
              "        }",
              "    },",
              ");",
              "",
              "// In render.",
              "Row::new().size(240.0, 150.0).ref_element(page.clone())",
            ],
          ),
        ),
        Section::new(
          "Near the viewport",
          "Forty-eight thumbnails, each loaded once it comes within 80 of the list's viewport, and kept. Those far \
           under it stay as they are until the list is scrolled toward them.",
          self.frame.demo(
            1,
            thumbs,
            "VisibilityOptions::new().margin(80.0)",
            &[
              "let watching = tile.observe_visibility(",
              "    VisibilityOptions::new().margin(80.0),",
              "    move |seen| {",
              "        if seen.visible {",
              "            load(index);",
              "        }",
              "    },",
              ");",
            ],
          ),
        ),
        Section::new(
          "A mark at the end",
          "The last child of the list is a mark of no height. When it comes into view ten more rows come before \
           it, which takes it out of view again, until the list has all its rows.",
          self.frame.demo(
            2,
            endless,
            "Spacer::new().height(0.0).ref_element(end.clone())",
            &[
              "// In create.",
              "let end = ElementRef::new();",
              "let watching = end.observe_visibility(VisibilityOptions::new(), move |seen| {",
              "    if seen.visible {",
              "        rows.update(|rows| *rows += 10);",
              "    }",
              "});",
              "",
              "// In render: the mark is the last child of what scrolls.",
              "ScrollVertical::new(",
              "    Column::new()",
              "        .with_children(rows)",
              "        .child(Spacer::new().height(0.0).ref_element(end.clone())),",
              ")",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What an observer is given, and what it is told.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
