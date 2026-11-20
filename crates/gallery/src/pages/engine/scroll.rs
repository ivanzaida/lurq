//! The page of the engine's scroll containers: `ScrollVertical`, `ScrollHorizontal` and `ScrollBoth`, their bars
//! (`ScrollBarStyle`), and scrolling to a place with a `ScrollState`. Its playground scrolls content as its props
//! say; its sections are the demo's Scroll page, the bars of the demo's shell, the ready-made bars, and the ways
//! to scroll to a place, and a strip that scrolls by pages with what it tells of its page and of its scroll. The
//! design system's container is `ui::ScrollArea`, on a page of its own.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    events::ScrollEvent,
    theme::{PaletteColor, RadiusSize},
  },
  components::{Column, Row, ScrollBoth, ScrollHorizontal, ScrollVertical},
  core::{ElementRef, Signal},
  layout::{
    Alignment,
    layout_kind::{Justify, ScrollState},
    scrollbar::{ScrollBarPlacement, ScrollBarStyle, ScrollBarVisibility},
    text_style::FontWeight,
  },
  node::Element,
  ui::{Button, ButtonSize, ButtonVariant},
};

use super::kit::{controls, sample, samples};
use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, line, mono, props_table, quiet, text},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

const AXES: [(&str, usize); 3] = [("ScrollVertical", 0), ("ScrollHorizontal", 1), ("ScrollBoth", 2)];
const VISIBILITIES: [(&str, ScrollBarVisibility); 5] = [
  ("Auto", ScrollBarVisibility::Auto),
  ("Always", ScrollBarVisibility::Always),
  ("OnHover", ScrollBarVisibility::OnHover),
  ("WhileScrolling", ScrollBarVisibility::WhileScrolling),
  ("Never", ScrollBarVisibility::Never),
];
/// The choices of the playground's select of when the bar shows: an index into [`VISIBILITIES`].
const VISIBILITY_CHOICES: [(&str, usize); 5] = [
  ("Auto", 0),
  ("Always", 1),
  ("OnHover", 2),
  ("WhileScrolling", 3),
  ("Never", 4),
];
const PLACEMENTS: [(&str, usize); 2] = [("Overlay", 0), ("Reserved", 1)];
const WIDTHS: [(&str, i32); 4] = [("4", 4), ("7", 7), ("8", 8), ("12", 12)];
const RADII: [(&str, i32); 3] = [("0", 0), ("2", 2), ("4", 4)];
/// The colors the playground offers a thumb, as roles of the theme; the first is none, which the theme's ring draws.
const THUMBS: [(&str, Option<PaletteColor>); 3] = [
  ("theme", None),
  ("Primary", Some(PaletteColor::Primary)),
  ("Chart2", Some(PaletteColor::Chart2)),
];
const THUMB_CHOICES: [(&str, usize); 3] = [("theme", 0), ("Primary", 1), ("Chart2", 2)];
const TRACKS: [(&str, usize); 2] = [("none", 0), ("Muted", 1)];

/// A role of the theme as it is drawn now: the color a bar's thumb or track is given.
fn role(color: PaletteColor) -> lurq::node::color::Color {
  lurq::ui::current_theme().palette_color(color)
}

/// A row of a list that scrolls down, in turn on the background and on a muted fill.
fn item(index: usize) -> Row {
  Row::new()
    .width(FILL)
    .height(32.0)
    .padding_horizontal(12.0)
    .align_items(Alignment::Center)
    .rounded(RadiusSize::Sm)
    .background(if index.is_multiple_of(2) {
      PaletteColor::Background
    } else {
      PaletteColor::Muted
    })
    .child(text(&format!("Item {index}"), 12.0, FontWeight::Medium))
}

fn items(count: usize) -> Column {
  Column::new()
    .width(FILL)
    .spacing(4.0)
    .with_children((1..=count).map(item))
}

/// A card of a list that scrolls sideways.
fn card(index: usize) -> Row {
  Row::new()
    .size(150.0, 80.0)
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .rounded(RadiusSize::Md)
    .background(PaletteColor::Background)
    .border(line())
    .child(text(&format!("Card {index}"), 12.0, FontWeight::Bold))
}

fn cards() -> Row {
  Row::new().spacing(12.0).with_children((1..=8).map(card))
}

/// Seven rows of twelve cells, 720 by 420: what scrolls both ways.
fn cells() -> Column {
  Column::new()
    .spacing(4.0)
    .padding(4.0)
    .with_children((1..=7).map(|row| {
      Row::new().spacing(4.0).with_children((1..=12).map(move |column| {
        Row::new()
          .size(56.0, 56.0)
          .align_items(Alignment::Center)
          .justify(Justify::Center)
          .rounded(RadiusSize::Sm)
          .background(if (row + column) % 2 == 0 {
            PaletteColor::Muted
          } else {
            PaletteColor::Background
          })
          .border(line())
          .child(mono(&format!("{row}:{column}"), 10.0).color(PaletteColor::MutedForeground))
      }))
    }))
}

/// The bar of the demo's Overlay and Reserved lists: always shown, 7 wide, a thumb in the primary color on a
/// muted track.
fn shown_bar(placement: ScrollBarPlacement) -> ScrollBarStyle {
  ScrollBarStyle {
    visible: ScrollBarVisibility::Always,
    placement,
    width: 7.0,
    min_thumb_length: 32.0,
    thumb_color: role(PaletteColor::Primary),
    thumb_radius: 4.0,
    track_color: role(PaletteColor::Muted),
    track_radius: 4.0,
    padding: 3.0,
    ..ScrollBarStyle::default()
  }
}

/// The Rust of [`shown_bar`].
const SHOWN_BAR: [&str; 11] = [
  "let bar = |placement| ScrollBarStyle {",
  "    visible: ScrollBarVisibility::Always,",
  "    placement,",
  "    width: 7.0,",
  "    min_thumb_length: 32.0,",
  "    thumb_color: theme.palette_color(PaletteColor::Primary),",
  "    thumb_radius: 4.0,",
  "    track_color: theme.palette_color(PaletteColor::Muted),",
  "    track_radius: 4.0,",
  "    padding: 3.0,",
  "    ..ScrollBarStyle::default() };",
];

/// A list of each placement of the bar, side by side.
fn placements(scroller: impl Fn(ScrollBarPlacement) -> Element) -> Row {
  samples()
    .child(sample(
      Column::new()
        .width(280.0)
        .max_width(FILL)
        .child(scroller(ScrollBarPlacement::Overlay)),
      "ScrollBarPlacement::Overlay",
    ))
    .child(sample(
      Column::new()
        .width(280.0)
        .max_width(FILL)
        .child(scroller(ScrollBarPlacement::Reserved)),
      "ScrollBarPlacement::Reserved",
    ))
}

/// What scrolls in the playground, as its props say.
#[derive(Clone, Copy)]
struct Scroller {
  axis: usize,
  visible: usize,
  reserved: bool,
  width: i32,
  thumb: usize,
  radius: i32,
  track: bool,
  hovered: bool,
}

impl Scroller {
  fn style(self) -> ScrollBarStyle {
    let mut style = ScrollBarStyle {
      visible: VISIBILITIES[self.visible.min(VISIBILITIES.len() - 1)].1,
      placement: if self.reserved {
        ScrollBarPlacement::Reserved
      } else {
        ScrollBarPlacement::Overlay
      },
      width: self.width as f32,
      thumb_radius: self.radius as f32,
      track_radius: self.radius as f32,
      ..ScrollBarStyle::default()
    };
    if let Some(color) = THUMBS[self.thumb.min(THUMBS.len() - 1)].1.clone() {
      style.thumb_color = role(color);
    }
    if self.track {
      style.track_color = role(PaletteColor::Muted);
    }
    style
  }

  /// The container; `record` hears it reach its top and its bottom.
  fn build(self, record: Arc<dyn Fn(String) + Send + Sync>) -> Element {
    let style = self.style();
    let hover = role(PaletteColor::Chart2);
    let hovered = self.hovered;
    let on_hover = move |style: ScrollBarStyle| {
      if hovered { style.with_thumb_color(hover) } else { style }
    };
    let (top, bottom) = (record.clone(), record);
    let reached_top = move |_: ScrollEvent| top("on_scroll_reach_top".to_owned());
    let reached_bottom = move |_: ScrollEvent| bottom("on_scroll_reach_bottom".to_owned());
    match self.axis {
      1 => ScrollHorizontal::new(cards())
        .scrollbar(style)
        .scrollbar_hovered(on_hover)
        .width(FILL)
        .height(116.0)
        .rounded(RadiusSize::Md)
        .background(PaletteColor::Muted)
        .into(),
      2 => ScrollBoth::new(cells())
        .scrollbar(style)
        .scrollbar_hovered(on_hover)
        .on_scroll_reach_top(reached_top)
        .on_scroll_reach_bottom(reached_bottom)
        .width(FILL)
        .height(200.0)
        .rounded(RadiusSize::Md)
        .background(PaletteColor::Background)
        .border(line())
        .into(),
      _ => ScrollVertical::new(items(20))
        .scrollbar(style)
        .scrollbar_hovered(on_hover)
        .on_scroll_reach_top(reached_top)
        .on_scroll_reach_bottom(reached_bottom)
        .width(FILL)
        .height(220.0)
        .rounded(RadiusSize::Md)
        .background(PaletteColor::Card)
        .border(line())
        .into(),
    }
  }

  fn code(self) -> Vec<String> {
    let (container, content) = match self.axis {
      1 => ("ScrollHorizontal", "cards"),
      2 => ("ScrollBoth", "cells"),
      _ => ("ScrollVertical", "items"),
    };
    let mut code = vec![
      format!("{container}::new({content})"),
      "    .scrollbar(ScrollBarStyle {".to_owned(),
    ];
    if self.visible != 0 {
      code.push(format!(
        "        visible: ScrollBarVisibility::{},",
        VISIBILITIES[self.visible].0
      ));
    }
    if self.reserved {
      code.push("        placement: ScrollBarPlacement::Reserved,".to_owned());
    }
    code.push(format!("        width: {}.0,", self.width));
    if let (name, Some(_)) = &THUMBS[self.thumb] {
      code.push(format!(
        "        thumb_color: theme.palette_color(PaletteColor::{name}),"
      ));
    }
    code.push(format!("        thumb_radius: {}.0,", self.radius));
    if self.track {
      code.push("        track_color: theme.palette_color(PaletteColor::Muted),".to_owned());
    }
    code.push("        ..ScrollBarStyle::default()".to_owned());
    code.push("    })".to_owned());
    if self.hovered {
      code.push(
        "    .scrollbar_hovered(move |style| style.with_thumb_color(theme.palette_color(PaletteColor::Chart2)))"
          .to_owned(),
      );
    }
    if self.axis != 1 {
      code.push("    .on_scroll_reach_bottom(|_| /* .. */)".to_owned());
    }
    code
  }
}

pub struct ScrollPage {
  frame: Frame,
  playground: Playground,
  /// Indexes into [`AXES`], [`VISIBILITIES`], [`PLACEMENTS`], [`THUMBS`] and [`TRACKS`].
  axis: Signal<usize>,
  visible: Signal<usize>,
  placement: Signal<usize>,
  width: Signal<i32>,
  thumb: Signal<usize>,
  radius: Signal<i32>,
  track: Signal<usize>,
  hovered: Signal<bool>,
}

impl ScrollPage {
  fn play(&self, ctx: &mut Ctx) -> Element {
    let scroller = Scroller {
      axis: self.axis.get(),
      visible: self.visible.get(),
      reserved: self.placement.get() == 1,
      width: self.width.get(),
      thumb: self.thumb.get(),
      radius: self.radius.get(),
      track: self.track.get() == 1,
      hovered: self.hovered.get(),
    };
    let record: Arc<dyn Fn(String) + Send + Sync> = Arc::new(self.playground.recorder());
    let (name, _) = AXES[scroller.axis.min(AXES.len() - 1)];
    let tree = vec![
      format!("{name}   {}", self.playground.measured()),
      format!(
        "├─ {}",
        match scroller.axis {
          1 => "row of 8 cards",
          2 => "7 rows of 12 cells",
          _ => "column of 20 items",
        }
      ),
      format!(
        "└─ bar   {} · {} wide",
        VISIBILITIES[scroller.visible].0, scroller.width
      ),
    ];
    let (set_axis, set_visible, set_placement, set_width, set_thumb, set_radius, set_track) = (
      self.axis.clone(),
      self.visible.clone(),
      self.placement.clone(),
      self.width.clone(),
      self.thumb.clone(),
      self.radius.clone(),
      self.track.clone(),
    );
    let controls = vec![
      prop(
        "container",
        "",
        self
          .playground
          .select(0, &AXES, &scroller.axis, move |value| set_axis.set(value)),
      ),
      prop(
        "visible",
        "ScrollBarVisibility",
        self
          .playground
          .select(1, &VISIBILITY_CHOICES, &scroller.visible, move |value| {
            set_visible.set(value)
          }),
      ),
      prop(
        "placement",
        "ScrollBarPlacement",
        self
          .playground
          .select(2, &PLACEMENTS, &self.placement.get(), move |value| {
            set_placement.set(value)
          }),
      ),
      prop(
        "width",
        "f32",
        self
          .playground
          .select(3, &WIDTHS, &scroller.width, move |value| set_width.set(value)),
      ),
      prop(
        "thumb_color",
        "Color",
        self
          .playground
          .select(4, &THUMB_CHOICES, &scroller.thumb, move |value| set_thumb.set(value)),
      ),
      prop(
        "thumb_radius",
        "f32",
        self
          .playground
          .select(5, &RADII, &scroller.radius, move |value| set_radius.set(value)),
      ),
      prop(
        "track_color",
        "Color",
        self
          .playground
          .select(6, &TRACKS, &self.track.get(), move |value| set_track.set(value)),
      ),
      divider(),
      flag("scrollbar_hovered: thumb in Chart2", &self.hovered),
    ];
    let reset = {
      let (axis, visible, placement, width, thumb, radius, track, hovered) = (
        self.axis.clone(),
        self.visible.clone(),
        self.placement.clone(),
        self.width.clone(),
        self.thumb.clone(),
        self.radius.clone(),
        self.track.clone(),
        self.hovered.clone(),
      );
      Arc::new(move || {
        axis.set(0);
        visible.set(0);
        placement.set(0);
        width.set(8);
        thumb.set(0);
        radius.set(4);
        track.set(0);
        hovered.set(false);
      })
    };
    let shown = record.clone();
    self.playground.card(
      ctx,
      Spec {
        title: "Scroll",
        make: Arc::new(move || scroller.build(Arc::new(|_| {}))),
        shown: Shown::built(move |_| scroller.build(shown)),
        fill: true,
        forceable: false,
        facts: format!("bar {}", VISIBILITIES[scroller.visible].0),
        event: "on_scroll_reach_top / on_scroll_reach_bottom",
        code: scroller.code(),
        tree,
        controls,
        changed: [
          scroller.axis != 0,
          scroller.visible != 0,
          scroller.reserved,
          scroller.width != 8,
          scroller.thumb != 0,
          scroller.radius != 4,
          scroller.track,
          scroller.hovered,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset,
      },
    )
  }
}

/// A list with the bar of the demo's shell: shown while the content overflows, 7 wide, a thumb in the primary
/// color that the pointer turns to another; and one that says the same with the roles of the theme.
fn shell() -> Row {
  let (primary, hover) = (role(PaletteColor::Primary), role(PaletteColor::Chart2));
  let by_closure = ScrollVertical::new(items(20))
    .scrollbar(ScrollBarStyle {
      visible: ScrollBarVisibility::Auto,
      width: 7.0,
      thumb_color: primary,
      thumb_radius: 4.0,
      ..ScrollBarStyle::default()
    })
    .scrollbar_hovered(move |style| style.with_thumb_color(hover))
    .width(FILL)
    .height(200.0)
    .rounded(RadiusSize::Md)
    .background(PaletteColor::Card)
    .border(line());
  let by_roles = ScrollVertical::new(items(20))
    .scrollbar(ScrollBarStyle {
      width: 7.0,
      thumb_color: primary,
      thumb_hovered_color: Some(PaletteColor::Chart2.into()),
      thumb_pressed_color: Some(PaletteColor::Chart4.into()),
      ..ScrollBarStyle::default()
    })
    .width(FILL)
    .height(200.0)
    .rounded(RadiusSize::Md)
    .background(PaletteColor::Card)
    .border(line());
  samples()
    .child(sample(
      Column::new().width(240.0).max_width(FILL).child(by_closure),
      "scrollbar_hovered",
    ))
    .child(sample(
      Column::new().width(240.0).max_width(FILL).child(by_roles),
      "thumb_hovered_color, thumb_pressed_color",
    ))
}

/// A list with each of the bars the engine has ready.
fn ready() -> Row {
  let mut all = samples();
  for (name, style) in [
    ("ScrollBarStyle::default()", ScrollBarStyle::default()),
    ("ScrollBarStyle::thin()", ScrollBarStyle::thin()),
    ("ScrollBarStyle::wide()", ScrollBarStyle::wide()),
    ("ScrollBarStyle::on_hover()", ScrollBarStyle::on_hover()),
    ("ScrollBarStyle::indicator()", ScrollBarStyle::indicator()),
    ("ScrollBarStyle::hidden()", ScrollBarStyle::hidden()),
  ] {
    all = all.child(sample(
      ScrollVertical::new(items(16))
        .scrollbar(style)
        .width(200.0)
        .max_width(FILL)
        .height(150.0)
        .rounded(RadiusSize::Md)
        .background(PaletteColor::Card)
        .border(line()),
      name,
    ));
  }
  all
}

/// How many pages the strip that scrolls by pages has, how wide and high one is, and how many lines of what it
/// told are kept.
const PAGES: usize = 5;
const PAGE_WIDTH: f32 = 240.0;
const PAGE_HEIGHT: f32 = 130.0;
const PAGES_TOLD: usize = 7;

/// A strip a finger scrolls a page at a time, with what its `ScrollState` says of its page beside it and what
/// its container told last. A component of its own: what it tells changes at every frame of a scroll.
struct Pages {
  state: ScrollState,
  /// What the container told, the newest last, and how many times in a row it told the last.
  told: Signal<Vec<(String, usize)>>,
  /// The page the strip is going to, and whether something moves it: read where the container tells.
  going: Signal<usize>,
  moving: Signal<bool>,
}

impl Pages {
  /// A handler's way to say a line, and to read where the strip is going with it.
  fn tell(&self) -> impl Fn(String) + Clone + Send + Sync + 'static {
    let (told, going, moving, state) = (
      self.told.clone(),
      self.going.clone(),
      self.moving.clone(),
      self.state.clone(),
    );
    move |line: String| {
      told.update(|told| match told.last_mut() {
        Some((last, times)) if *last == line => *times += 1,
        _ => {
          told.push((line, 1));
          if told.len() > PAGES_TOLD {
            told.remove(0);
          }
        }
      });
      let (to, _) = state.target_page();
      if going.get_untracked() != to {
        going.set(to);
      }
      if moving.get_untracked() != state.moving() {
        moving.set(state.moving());
      }
    }
  }
}

impl Component for Pages {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let state = ScrollState::new();
    state.set_paging(true);
    Self {
      state,
      told: ctx.signal(Vec::new()),
      going: ctx.signal(0),
      moving: ctx.signal(false),
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    // Rendered again when the strip comes to rest on another page.
    let (rested, _) = self.state.page_signal().get();
    let pages = Row::new().with_children((0..PAGES).map(|page| {
      Row::new()
        .size(PAGE_WIDTH, PAGE_HEIGHT)
        .align_items(Alignment::Center)
        .justify(Justify::Center)
        .background(if page.is_multiple_of(2) {
          PaletteColor::Muted
        } else {
          PaletteColor::Background
        })
        .child(text(&format!("Page {page}"), 14.0, FontWeight::Bold))
    }));
    let (started, scrolled, ended, paged) = (self.tell(), self.tell(), self.tell(), self.tell());
    let strip = ScrollHorizontal::new(pages)
      .with_scroll_state(self.state.clone())
      .scrollbar(ScrollBarStyle::hidden())
      .on_scroll_start(move |_: ScrollEvent| started("on_scroll_start".to_owned()))
      .on_scroll(move |event: ScrollEvent| {
        // The events with no delta come after the content moved.
        if (event.delta_x, event.delta_y) == (0.0, 0.0) {
          scrolled(format!("on_scroll {}", event.source.as_str()));
        }
      })
      .on_scroll_end(move |event: ScrollEvent| ended(format!("on_scroll_end {}", event.source.as_str())))
      .on_page_change(move |page: usize| paged(format!("on_page_change {page}")))
      .id("scroll-pages")
      .size(PAGE_WIDTH, PAGE_HEIGHT)
      .rounded(RadiusSize::Md)
      .border(line());
    let button = |label: &str, id: &str, to: usize| {
      let state = self.state.clone();
      Button::new(label)
        .variant(ButtonVariant::Outline)
        .size(ButtonSize::Sm)
        .id(id)
        .on_click(move |_| state.set_scroll(to as f32 * PAGE_WIDTH, 0.0))
    };
    let says = |name: &str, value: String, id: &str| {
      Row::new()
        .spacing(8.0)
        .child(quiet(name, 12.0))
        .child(mono(&value, 12.0).id(id))
    };
    let told = self.told.get();
    let told = Column::new()
      .width(220.0)
      .max_width(FILL)
      .spacing(2.0)
      .id("scroll-pages-told")
      .with_children(told.iter().map(|(line, times)| {
        if *times > 1 {
          quiet(&format!("{line} ×{times}"), 12.0)
        } else {
          quiet(line, 12.0)
        }
      }));
    Column::new()
      .width(FILL)
      .spacing(16.0)
      .child(
        controls()
          .child(button("set_scroll: first", "scroll-pages-first", 0))
          .child(button(
            "set_scroll: next",
            "scroll-pages-next",
            (rested + 1).min(PAGES - 1),
          )),
      )
      .child(
        samples()
          .child(strip)
          .child(
            Column::new()
              .spacing(6.0)
              .child(says("page_signal()", rested.to_string(), "scroll-pages-rested"))
              .child(says(
                "target_page()",
                self.going.get().to_string(),
                "scroll-pages-going",
              ))
              .child(says("moving()", self.moving.get().to_string(), "scroll-pages-moving")),
          )
          .child(told),
      )
  }
}

/// How many albums the feed of strips has, and how many pictures an album.
const ALBUMS: usize = 3;
const PICTURES: usize = 3;

/// A feed of albums that scrolls down by pages, each album a strip of pictures that scrolls across by pages:
/// what a finger's gesture scrolls is one of the two, and both come to rest on a page.
struct Nested {
  feed: ScrollState,
  strips: Vec<ScrollState>,
}

impl Component for Nested {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    let paged = || {
      let state = ScrollState::new();
      state.set_paging(true);
      state
    };
    Self {
      feed: paged(),
      strips: (0..ALBUMS).map(|_| paged()).collect(),
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    // Rendered again when the feed or a strip comes to rest on another page.
    let (_, album) = self.feed.page_signal().get();
    let pictures: Vec<usize> = self.strips.iter().map(|strip| strip.page_signal().get().0).collect();
    let albums = Column::new().with_children(self.strips.iter().enumerate().map(|(album, strip)| {
      let strip_of = Row::new().with_children((0..PICTURES).map(|picture| {
        Row::new()
          .size(PAGE_WIDTH, PAGE_HEIGHT)
          .align_items(Alignment::Center)
          .justify(Justify::Center)
          .background(if (album + picture).is_multiple_of(2) {
            PaletteColor::Muted
          } else {
            PaletteColor::Background
          })
          .child(text(
            &format!("Album {album} · picture {picture}"),
            13.0,
            FontWeight::Bold,
          ))
      }));
      ScrollHorizontal::new(strip_of)
        .with_scroll_state(strip.clone())
        .scrollbar(ScrollBarStyle::hidden())
        .id(format!("scroll-nested-strip-{album}"))
        .size(PAGE_WIDTH, PAGE_HEIGHT)
    }));
    let says = |name: &str, value: String, id: &str| {
      Row::new()
        .spacing(8.0)
        .child(quiet(name, 12.0))
        .child(mono(&value, 12.0).id(id))
    };
    samples()
      .child(
        ScrollVertical::new(albums)
          .with_scroll_state(self.feed.clone())
          .scrollbar(ScrollBarStyle::hidden())
          .id("scroll-nested")
          .size(PAGE_WIDTH, PAGE_HEIGHT)
          .rounded(RadiusSize::Md)
          .border(line()),
      )
      .child(
        Column::new()
          .spacing(6.0)
          .child(says(
            "the feed rests on album",
            album.to_string(),
            "scroll-nested-album",
          ))
          .child(says(
            "its strip rests on picture",
            pictures.get(album).copied().unwrap_or(0).to_string(),
            "scroll-nested-picture",
          ))
          .child(says(
            "every strip",
            pictures.iter().map(usize::to_string).collect::<Vec<_>>().join(" "),
            "scroll-nested-pictures",
          )),
      )
  }
}

/// How many rows the list that is scrolled to a place has, and the one a button scrolls to.
const PLACES_ROWS: usize = 60;
const PLACES_ROW: usize = 40;

/// What the list of the section that scrolls to a place is asked: by the page, which has the context.
#[derive(Clone, Copy, PartialEq)]
enum Ask {
  Top,
  Row,
}

/// A list kept on a `ScrollState`, with buttons that scroll it to a place and its offset beside them. A
/// component of its own, so that its offset, which changes at every frame of a scroll, renders it alone.
struct Places {
  state: ScrollState,
  row: ElementRef,
  offset: Signal<i32>,
  /// What the buttons asked for that needs the context of a render, and how many times: a render that sees more
  /// asks than it handled makes the request.
  asked: Signal<usize>,
  asks: Signal<usize>,
  handled: Mutex<usize>,
  /// How many rows were added at the end, and whether the list sticks to its end as they come.
  added: Signal<usize>,
  sticks: Signal<bool>,
}

impl Places {
  fn ask(&self, ask: Ask) -> impl Fn() + Send + Sync + 'static {
    let (asked, asks) = (self.asked.clone(), self.asks.clone());
    move || {
      asked.set(ask as usize);
      asks.update(|count| *count += 1);
    }
  }
}

impl Component for Places {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      state: ScrollState::new(),
      row: ElementRef::new(),
      offset: ctx.signal(0),
      asked: ctx.signal(0),
      asks: ctx.signal(0),
      handled: Mutex::new(0),
      added: ctx.signal(0),
      sticks: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let asks = self.asks.get();
    {
      let mut handled = self.handled.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
      if *handled != asks {
        *handled = asks;
        if self.asked.get_untracked() == Ask::Top as usize {
          ctx.scroll_to_top_of(&self.state);
        } else {
          ctx.scroll_into_view(&self.row);
        }
      }
    }
    let button = |label: &str, press: Box<dyn Fn() + Send + Sync>| {
      Button::new(label)
        .variant(ButtonVariant::Outline)
        .size(ButtonSize::Sm)
        .on_click(move |_| press())
    };
    let (by, bottom) = (self.state.clone(), self.state.clone());
    let offset = self.offset.clone();
    let state = self.state.clone();
    // A list at its end stays where it is as rows come below, unless it sticks to its end, as a chat's does.
    let (added, sticks) = (self.added.get(), self.sticks.get());
    self.state.set_sticks_to_end(sticks);
    let (more, stick) = (self.added.clone(), self.sticks.clone());
    let rows = Column::new()
      .width(FILL)
      .spacing(4.0)
      .with_children((1..=PLACES_ROWS + added).map(|index| {
        let row = item(index);
        if index == PLACES_ROW {
          row.background(PaletteColor::Chart2).ref_element(self.row.clone())
        } else {
          row
        }
      }));
    Column::new()
      .width(FILL)
      .spacing(16.0)
      .child(
        controls()
          .child(button("To the top", Box::new(self.ask(Ask::Top))))
          .child(button("Down 100", Box::new(move || by.scroll_by(0.0, 100.0))))
          .child(button(
            &format!("Into view: item {PLACES_ROW}"),
            Box::new(self.ask(Ask::Row)),
          ))
          .child(button(
            "To the bottom",
            Box::new(move || bottom.scroll_to_bottom_pending()),
          ))
          .child(button("Add 5 rows", Box::new(move || more.update(|count| *count += 5))))
          .child(button(
            if sticks {
              "Sticks to end: on"
            } else {
              "Sticks to end: off"
            },
            Box::new(move || stick.update(|on| *on = !*on)),
          ))
          .child(mono(&format!("scroll_y {}", self.offset.get()), 12.0).color(PaletteColor::MutedForeground)),
      )
      .child(
        ScrollVertical::new(rows)
          .with_scroll_state(self.state.clone())
          .on_scroll(move |_: ScrollEvent| {
            let at = state.scroll_y().round() as i32;
            if offset.get_untracked() != at {
              offset.set(at);
            }
          })
          .width(320.0)
          .max_width(FILL)
          .height(220.0)
          .rounded(RadiusSize::Md)
          .background(PaletteColor::Card)
          .border(line()),
      )
  }
}

impl Component for ScrollPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 7),
      axis: ctx.signal(0),
      visible: ctx.signal(0),
      placement: ctx.signal(0),
      width: ctx.signal(8),
      thumb: ctx.signal(0),
      radius: ctx.signal(4),
      track: ctx.signal(0),
      hovered: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let rows = [
      (
        "ScrollVertical / ScrollHorizontal / ScrollBoth::new",
        "impl Into<Element>",
        "",
        "What scrolls: the child is given as much room as it wants along the axes that scroll. Without a size \
         there, the container is as large as its content, up to its max_height.",
      ),
      (
        "scrollbar",
        "ScrollBarStyle",
        "the theme's",
        "The bars: when they show (visible), over the content or beside it (placement), width, thumb and track \
         colors and radii, padding, and how a bar that comes and goes fades.",
      ),
      (
        "scrollbar_hovered",
        "Fn(ScrollBarStyle) -> ScrollBarStyle",
        "",
        "The bar while the pointer is over it, made from the bar at rest. thumb_hovered_color and \
         thumb_pressed_color in the style say it with the theme's roles.",
      ),
      (
        "with_scroll_state",
        "ScrollState",
        "",
        "The state the app keeps to read the offset (scroll_y) and to scroll: set_scroll, scroll_by, \
         scroll_to_bottom_pending, and ctx.scroll_to_top_of.",
      ),
      (
        "on_scroll / on_scroll_reach_top / on_scroll_reach_bottom",
        "Fn(ScrollEvent)",
        "",
        "Hears the content move, and reach an end.",
      ),
      (
        "on_scroll_start / on_scroll_end",
        "Fn(ScrollEvent)",
        "",
        "Hears the content start to move, and come to rest: after a finger's momentum, the way to a page and the \
         spring back from past an end, not at the finger's lift. event.source is what moved it.",
      ),
      (
        "on_page_change",
        "Fn(usize)",
        "",
        "Hears the page the content came to rest on, a page being as long as the view, whatever took it there. \
         ScrollState::page_signal() is the same for a render, and target_page() where it is going.",
      ),
      (
        "scrollbars_clear_of_sticky",
        "bool",
        "false",
        "The bars run beside what sticks at the edges and not over it.",
      ),
      (
        "fill_width",
        "bool",
        "false",
        "On ScrollHorizontal and ScrollBoth: the content is at least as wide as the view.",
      ),
      (
        "wheel_across",
        "bool",
        "false",
        "On ScrollHorizontal and ScrollBoth: a wheel turned up and down scrolls sideways.",
      ),
      (
        "middle_button_autoscroll",
        "bool",
        "false",
        "A press of the middle button scrolls toward the pointer, the faster the farther it is.",
      ),
    ];
    let playground = self.play(ctx);
    let places = ctx.mount::<Places>(());
    let pages = ctx.mount::<Pages>(());
    let nested = ctx.mount::<Nested>(());
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Engine", "Scroll"],
        title: "Scroll",
        description: "ScrollVertical, ScrollHorizontal and ScrollBoth: the engine's containers that scroll what is \
                      larger than them, with bars an app styles itself. The design system's is the Scroll Area.",
        import: Some(
          "use lurq::{components::{ScrollBoth, ScrollHorizontal, ScrollVertical}, layout::scrollbar::ScrollBarStyle};",
        ),
        status: Some("Stable"),
        since: None,
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "A list that scrolls down, a row of cards that scrolls sideways, or cells that scroll both ways, with a \
           bar styled by the panel. An overlay bar lies over the content; a reserved one has a lane of its own.",
          playground,
        ),
        Section::new(
          "Down",
          "Twenty items in a view of about six, with the bar over the content and with a lane of its own.",
          self.frame.demo(
            0,
            placements(|placement| {
              ScrollVertical::new(items(20))
                .scrollbar(shown_bar(placement))
                .width(FILL)
                .height(220.0)
                .rounded(RadiusSize::Md)
                .background(PaletteColor::Card)
                .border(line())
                .into()
            }),
            "ScrollVertical::new(items)",
            &[
              &SHOWN_BAR[..],
              &[
                "",
                "ScrollVertical::new(items)",
                "    .scrollbar(bar(ScrollBarPlacement::Overlay))",
                "    .height(220.0)",
              ],
            ]
            .concat(),
          ),
        ),
        Section::new(
          "Sideways",
          "Eight cards of 150 in a row that scrolls sideways.",
          self.frame.demo(
            1,
            placements(|placement| {
              ScrollHorizontal::new(cards())
                .scrollbar(shown_bar(placement))
                .width(FILL)
                .height(116.0)
                .rounded(RadiusSize::Md)
                .background(PaletteColor::Muted)
                .into()
            }),
            "ScrollHorizontal::new(cards)",
            &[
              &SHOWN_BAR[..],
              &[
                "",
                "ScrollHorizontal::new(Row::new().spacing(12.0).with_children(cards))",
                "    .scrollbar(bar(ScrollBarPlacement::Reserved))",
                "    .height(116.0)",
              ],
            ]
            .concat(),
          ),
        ),
        Section::new(
          "Both ways",
          "Cells 720 by 420 in a view smaller both ways: a bar on each axis.",
          self.frame.demo(
            2,
            placements(|placement| {
              ScrollBoth::new(cells())
                .scrollbar(shown_bar(placement))
                .width(FILL)
                .height(180.0)
                .rounded(RadiusSize::Md)
                .background(PaletteColor::Background)
                .border(line())
                .into()
            }),
            "ScrollBoth::new(cells)",
            &[
              &SHOWN_BAR[..],
              &[
                "",
                "ScrollBoth::new(cells)",
                "    .scrollbar(bar(ScrollBarPlacement::Overlay))",
                "    .height(180.0)",
              ],
            ]
            .concat(),
          ),
        ),
        Section::new(
          "A bar that answers the pointer",
          "Bars as a window of an app may draw them: shown while the content overflows, a thumb in the primary color \
           that turns to another under the pointer, by a closure. The second says the same with roles of the \
           theme, and has another color while the thumb is dragged.",
          self.frame.demo(
            3,
            shell(),
            ".scrollbar_hovered(|style| style.with_thumb_color(hover))",
            &[
              "ScrollVertical::new(content)",
              "    .scrollbar(ScrollBarStyle {",
              "        visible: ScrollBarVisibility::Auto,",
              "        width: 7.0,",
              "        thumb_color: primary,",
              "        thumb_radius: 4.0,",
              "        ..ScrollBarStyle::default()",
              "    })",
              "    .scrollbar_hovered(move |style| style.with_thumb_color(hover))",
              "",
              "ScrollBarStyle {",
              "    thumb_hovered_color: Some(PaletteColor::Chart2.into()),",
              "    thumb_pressed_color: Some(PaletteColor::Chart4.into()),",
              "    ..ScrollBarStyle::default()",
              "}",
            ],
          ),
        ),
        Section::new(
          "Ready-made bars",
          "The bars the engine has ready: the theme's, thin, wide, one that shows while the pointer is over the \
           list, a phone's indicator that shows while the list scrolls, and none.",
          self.frame.demo(
            4,
            ready(),
            ".scrollbar(ScrollBarStyle::thin())",
            &[
              "ScrollVertical::new(items).scrollbar(ScrollBarStyle::thin())",
              "ScrollVertical::new(items).scrollbar(ScrollBarStyle::wide())",
              "ScrollVertical::new(items).scrollbar(ScrollBarStyle::on_hover())",
              "ScrollVertical::new(items).scrollbar(ScrollBarStyle::indicator())",
              "ScrollVertical::new(items).scrollbar(ScrollBarStyle::hidden())",
            ],
          ),
        ),
        Section::new(
          "Scrolling to a place",
          "A list kept on a ScrollState, which the app reads and moves: back to the top over a few frames, down by \
           100 at once, an element brought into view, and the bottom after the next layout. The offset is read in \
           on_scroll.",
          self.frame.demo(
            5,
            places,
            "ctx.scroll_to_top_of(&state)",
            &[
              "// Kept by the component, from create.",
              "let state = ScrollState::new();",
              "",
              "ScrollVertical::new(rows)",
              "    .with_scroll_state(state.clone())",
              "    .on_scroll(move |_| offset.set(state.scroll_y()))",
              "",
              "ctx.scroll_to_top_of(&state);         // in a render, over a few frames",
              "state.scroll_by(0.0, 100.0);          // anywhere, at once",
              "ctx.scroll_into_view(&row);           // the row's ElementRef",
              "state.scroll_to_bottom_pending();     // after the next layout",
              "state.set_sticks_to_end(true);        // a chat: at its end, it follows what is added",
            ],
          ),
        ),
        Section::new(
          "By pages",
          "A strip whose ScrollState pages: a finger scrolls it a page at a time (run the gallery with --device \
           phone, and the mouse is a finger; a touchpad scrolls it sideways as any strip). Beside it, the page it \
           came to rest on, the one it is going to, known as the finger lifts, and whether it moves; and what its \
           container told: one start, an on_scroll for every step with what moved it, the page once the strip is \
           within a fiftieth of a page of it, and one end when it is at rest, a quarter of a second after that. \
           An offset the app sets changes the page with no scroll told.",
          self.frame.demo(
            6,
            pages,
            "state.set_paging(true)",
            &[
              "// Kept by the component, from create.",
              "let state = ScrollState::new();",
              "state.set_paging(true);",
              "",
              "// In a render: rendered again when the strip rests on another page.",
              "let (page, _) = state.page_signal().get();",
              "",
              "ScrollHorizontal::new(pages)",
              "    .with_scroll_state(state.clone())",
              "    .on_scroll_start(|_| println!(\"it moves\"))",
              "    .on_scroll(|event| {",
              "        // No delta: the content is where this step took it.",
              "        if (event.delta_x, event.delta_y) == (0.0, 0.0) {",
              "            println!(\"{} moved it\", event.source.as_str());",
              "        }",
              "    })",
              "    .on_scroll_end(|event| println!(\"at rest after {}\", event.source.as_str()))",
              "    .on_page_change(|page| println!(\"page {page}\"))",
              "",
              "let (going, _) = state.target_page();  // known at the lift",
              "state.set_scroll(2.0 * 240.0, 0.0);    // the page follows, no scroll is told",
            ],
          ),
        ),
        Section::new(
          "A strip in a feed",
          "A feed of albums that scrolls down by pages, and in each album a strip of pictures that scrolls across \
           by pages (run the gallery with --device phone, and the mouse is a finger). A gesture is one list's: the \
           one along the way the finger went in its first points. A drag to the left that turns downward stays the \
           strip's, a slanted drag down is the feed's and leaves the strip where it is, and whatever ends the \
           gesture both come to rest on a page.",
          self.frame.demo(
            7,
            nested,
            "ScrollVertical::new(strips) // each a ScrollHorizontal",
            &[
              "// Kept by the component: the feed's state and one for each strip, all paging.",
              "let feed = ScrollState::new();",
              "feed.set_paging(true);",
              "",
              "let albums = Column::new().with_children(strips.iter().map(|strip| {",
              "    ScrollHorizontal::new(pictures())",
              "        .with_scroll_state(strip.clone())   // strip.set_paging(true)",
              "        .size(240.0, 130.0)",
              "}));",
              "ScrollVertical::new(albums)",
              "    .with_scroll_state(feed.clone())",
              "    .size(240.0, 130.0)",
              "",
              "// In a render: the album the feed rests on, and the picture its strip rests on.",
              "let (_, album) = feed.page_signal().get();",
              "let (picture, _) = strips[album].page_signal().get();",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a scroll container is given.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
