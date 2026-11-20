//! The page of the pager: `components::Pager`, pages one at a time of which only the one shown and the ones
//! beside it are built. Its playground is a pager whose pages tell when they are made and dropped, with its way,
//! its count and how many it builds beside; its sections are a strip of pictures across in a feed of albums
//! down, and its props.

use std::sync::Arc;

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{PaletteColor, RadiusSize},
  },
  components::{Column, Pager, Row},
  core::Signal,
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::Element,
  ui::{Button, ButtonSize, ButtonVariant},
};

use super::kit::samples;
use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, line, mono, props_table, quiet, text},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

const WAYS: [(&str, bool); 2] = [("vertical", false), ("horizontal", true)];
const COUNTS: [(&str, usize); 3] = [("5", 5), ("40", 40), ("1000", 1000)];
const BESIDE: [(&str, usize); 2] = [("1", 1), ("2", 2)];
/// How large the playground's pager is: upright where it goes down, wide where it goes across.
const UPRIGHT: (f32, f32) = (240.0, 300.0);
const WIDE: (f32, f32) = (240.0, 160.0);

/// What a page tells the playground's events.
type Tell = Arc<dyn Fn(String) + Send + Sync>;

/// A page of the playground's pager: which one, and what it tells when it is made and dropped.
struct LeafProps {
  index: usize,
  tell: Option<Tell>,
}

impl PartialEq for LeafProps {
  fn eq(&self, other: &Self) -> bool {
    self.index == other.index
  }
}

impl lurq::app::component::DevtoolsInspectable for LeafProps {}

/// A page with something of its own: a count of presses, which lives as long as the page is built.
struct Leaf {
  index: usize,
  pressed: Signal<u32>,
  tell: Option<Tell>,
}

impl Component for Leaf {
  type Props = LeafProps;

  fn create(ctx: &mut Ctx) -> Self {
    let (index, tell) = {
      let props = ctx.props::<LeafProps>();
      (props.index, props.tell.clone())
    };
    if let Some(tell) = &tell {
      tell(format!("page {index} made"));
    }
    Self {
      index,
      pressed: ctx.signal(0),
      tell,
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    let index = self.index;
    let pressed = self.pressed.get();
    let more = {
      let pressed = self.pressed.clone();
      Button::new("+1")
        .variant(ButtonVariant::Outline)
        .size(ButtonSize::Sm)
        .id(format!("pager-leaf-{index}-more"))
        .on_click(move |_| pressed.update(|pressed| *pressed += 1))
    };
    Column::new()
      .width(FILL)
      .flex(1.0)
      .spacing(8.0)
      .align_items(Alignment::Center)
      .justify(Justify::Center)
      .background(if index.is_multiple_of(2) {
        PaletteColor::Muted
      } else {
        PaletteColor::Background
      })
      .child(text(&format!("Page {index}"), 14.0, FontWeight::Bold))
      .child(quiet(&format!("pressed {pressed}"), 12.0).id(format!("pager-leaf-{index}-pressed")))
      .child(more)
  }

  fn on_unmounted(&self) {
    if let Some(tell) = &self.tell {
      tell(format!("page {} dropped", self.index));
    }
  }
}

/// A pager in the gallery's frame: a rounded box with a line about it.
fn framed(pager: impl Into<Element>) -> Column {
  Column::new()
    .max_width(FILL)
    .rounded(RadiusSize::Md)
    .border(line())
    .clip()
    .child(pager)
}

/// A line of what something is at: its name, and the value in a mono face with an id.
fn says(name: &str, value: &str, id: &str) -> Row {
  Row::new()
    .spacing(8.0)
    .child(quiet(name, 12.0))
    .child(mono(value, 12.0).id(id))
}

/// How many albums the feed has, how many pictures an album, and how large the feed and a strip are.
const ALBUMS: usize = 6;
const PICTURES: usize = 4;
const FEED: (f32, f32) = (240.0, 260.0);
const STRIP_HEIGHT: f32 = 180.0;

/// A feed of albums that goes down a page at a time, each with a strip of its pictures that goes across: a
/// finger that first goes across turns the strip, and one that first goes down the feed.
struct Albums {
  album: Signal<usize>,
  /// The picture each album's strip is at: the app's, so a strip that is built again is where it was.
  pictures: Vec<Signal<usize>>,
}

impl Component for Albums {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      album: ctx.signal(0),
      pictures: (0..ALBUMS).map(|_| ctx.signal(0)).collect(),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let album = self.album.get().min(ALBUMS - 1);
    let picture = self.pictures[album].get();
    let strips = self.pictures.clone();
    let feed = Pager::new(ALBUMS, move |ctx: &mut Ctx, album| {
      let strip = Pager::new(PICTURES, move |_: &mut Ctx, picture| {
        Row::new()
          .width(FILL)
          .flex(1.0)
          .align_items(Alignment::Center)
          .justify(Justify::Center)
          .background(if (album + picture).is_multiple_of(2) {
            PaletteColor::Muted
          } else {
            PaletteColor::Secondary
          })
          .child(text(&format!("{album} · {picture}"), 14.0, FontWeight::Bold))
      })
      .horizontal()
      .index(strips[album].clone())
      .height(STRIP_HEIGHT)
      .id(format!("albums-strip-{album}"))
      .mount(ctx);
      Column::new().width(FILL).flex(1.0).child(strip).child(
        Row::new()
          .width(FILL)
          .flex(1.0)
          .padding_horizontal(12.0)
          .align_items(Alignment::Center)
          .child(text(&format!("Album {album}"), 13.0, FontWeight::Medium)),
      )
    })
    .index(self.album.clone())
    .width(FEED.0)
    .height(FEED.1)
    .id("albums")
    .mount(ctx);
    samples().child(framed(feed)).child(
      Column::new()
        .spacing(6.0)
        .child(says("album", &album.to_string(), "albums-album"))
        .child(says("its picture", &picture.to_string(), "albums-picture")),
    )
  }
}

pub struct PagerPage {
  frame: Frame,
  playground: Playground,
  across: Signal<bool>,
  count: Signal<usize>,
  beside: Signal<usize>,
  keys: Signal<bool>,
  /// The page the playground's pager is at: the app's.
  index: Signal<usize>,
}

impl PagerPage {
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (across, count, beside, keys) = (self.across.get(), self.count.get(), self.beside.get(), self.keys.get());
    let last = count - 1;
    let index = self.index.get().min(last);
    let recorder = self.playground.recorder();
    let pager = {
      let (at, recorder) = (self.index.clone(), recorder.clone());
      move |live: bool| {
        let tell: Option<Tell> = live.then(|| Arc::new(recorder.clone()) as Tell);
        let (width, height) = if across { WIDE } else { UPRIGHT };
        let mut pager = Pager::new(count, move |ctx: &mut Ctx, index| {
          ctx.mount::<Leaf>(LeafProps {
            index,
            tell: tell.clone(),
          })
        })
        .beside(beside)
        .keys(keys)
        .width(width)
        .height(height)
        // Another way is another pager: its pages are laid out anew.
        .key(if across { "across" } else { "down" });
        if across {
          pager = pager.horizontal();
        }
        if live {
          let told = recorder.clone();
          pager = pager
            .id("pager")
            .index(at.clone())
            .on_index_change(move |to| told(format!("on_index_change({to})")));
        }
        pager
      }
    };
    let shown = {
      let pager = pager.clone();
      move |ctx: &mut Ctx| -> Element { framed(pager(true).mount(ctx)).into() }
    };
    // The window of its own shows a pager that keeps where it is itself.
    let plain = move || -> Element { framed(pager(false)).into() };

    let to = |label: &str, id: &str, page: usize| {
      let at = self.index.clone();
      Button::new(label)
        .variant(ButtonVariant::Outline)
        .size(ButtonSize::Sm)
        .id(id)
        .on_click(move |_| at.set(page))
    };
    let (set_across, set_count, set_beside) = (self.across.clone(), self.count.clone(), self.beside.clone());
    let back = self.index.clone();
    let controls = vec![
      prop(
        "vertical / horizontal",
        "",
        self
          .playground
          .select(0, &WAYS, &across, move |value| set_across.set(value)),
      ),
      divider(),
      prop(
        "count",
        "usize",
        self.playground.select(1, &COUNTS, &count, move |value| {
          back.set(0);
          set_count.set(value);
        }),
      ),
      divider(),
      prop(
        "beside",
        "usize",
        self
          .playground
          .select(2, &BESIDE, &beside, move |value| set_beside.set(value)),
      ),
      divider(),
      flag("keys", &self.keys),
      divider(),
      prop(
        "index",
        "Signal<usize>",
        Row::new()
          .spacing(8.0)
          .wrap()
          .child(to("0", "pager-first", 0))
          .child(to("+1", "pager-next", (index + 1).min(last)))
          .child(to("+10", "pager-far", (index + 10).min(last))),
      ),
    ];
    let reset = {
      let (across, count, beside, keys, at) = (
        self.across.clone(),
        self.count.clone(),
        self.beside.clone(),
        self.keys.clone(),
        self.index.clone(),
      );
      Arc::new(move || {
        across.set(false);
        count.set(5);
        beside.set(1);
        keys.set(true);
        at.set(0);
      })
    };
    let mut code = vec![
      format!("Pager::new({count}, |ctx, index| ctx.mount::<Page>(index))"),
      "    .index(at.clone())".to_owned(),
      "    .on_index_change(|index| println!(\"at page {index}\"))".to_owned(),
    ];
    if across {
      code.push("    .horizontal()".to_owned());
    }
    if beside != 1 {
      code.push(format!("    .beside({beside})"));
    }
    if !keys {
      code.push("    .keys(false)".to_owned());
    }
    code.push("    .mount(ctx)".to_owned());
    let built = (index.saturating_sub(beside)..=(index + beside).min(last))
      .map(|page| page.to_string())
      .collect::<Vec<_>>()
      .join(", ");
    let tree = vec![
      format!("pager   {}", self.playground.measured()),
      "└─ pages   a list that scrolls by pages".to_owned(),
      format!("   └─ page {built}   of {count}: the one shown and {beside} on each side"),
    ];
    self.playground.card(
      ctx,
      Spec {
        title: "Pager",
        make: Arc::new(plain),
        shown: Shown::built(shown),
        fill: false,
        forceable: false,
        facts: format!("at page {index} of {count}"),
        event: "on_index_change",
        code,
        tree,
        controls,
        changed: [across, count != 5, beside != 1, !keys]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset,
      },
    )
  }
}

impl Component for PagerPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 3),
      across: ctx.signal(false),
      count: ctx.signal(5),
      beside: ctx.signal(1),
      keys: ctx.signal(true),
      index: ctx.signal(0),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let rows = [
      (
        "Pager::new",
        "usize, Fn(&mut Ctx, usize) -> Element",
        "",
        "How many pages there are, and what builds the page at an index: called for the page shown and the ones \
         beside it, each in a component of its own, which is dropped when the page is further away.",
      ),
      (
        "index",
        "Signal<usize>",
        "",
        "The page it is at, held by the app. The pager goes to the page it says when the app sets it, and sets it \
         to the page a person brought it to once it is at rest there. Without one it keeps where it is itself.",
      ),
      (
        "on_index_change",
        "Fn(usize)",
        "",
        "Called with the page a finger, a wheel or a key brought it to, once it is at rest there. Not for a page \
         the app set.",
      ),
      (
        "vertical / horizontal",
        "",
        "vertical",
        "Pages one under another, or one beside another. A finger turns it along its own way alone: a strip \
         across in a feed that goes down takes the finger that first went across.",
      ),
      (
        "beside",
        "usize",
        "1",
        "How many pages are built on each side of the one shown. Never none: the page a finger drags in is there.",
      ),
      (
        "keys",
        "bool",
        "true",
        "The arrow keys along its way turn a page, and Page Up, Page Down, Home and End one that goes down: where \
         the focus is in it, and where most of it shows and no text input has the focus.",
      ),
      (
        "width / height",
        "Dimension",
        "100%",
        "How large it is: all of its room unless told. A page is as large as the pager.",
      ),
      (
        "id / key",
        "&str",
        "",
        "The id of its box; the list its pages scroll in is `{id}-pages` and a page `{id}-page-{index}`. A key \
         tells it from the pagers beside it.",
      ),
    ];
    let playground = self.play(ctx);
    let albums = ctx.mount::<Albums>(());
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Engine", "Pager"],
        title: "Pager",
        description: "Pages one at a time, each as large as the pager, down or across. Which page it is at is an \
                      index the app holds; only the page shown and the ones beside it are built, so a feed of a \
                      thousand videos holds three.",
        import: Some("use lurq::components::Pager;"),
        status: Some("New"),
        since: None,
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Drag a page with a finger (run the gallery with --device phone), turn the wheel over it, or press an \
           arrow key: it goes one page, and the index changes when it is at rest there. The buttons set the index, \
           as an app does. The events say when a page is made and when it is dropped; a page's count of presses is \
           its own, and is gone with it.",
          playground,
        ),
        Section::new(
          "A strip in a feed",
          "A feed of albums that goes down, each with a strip of its pictures that goes across. The feed and \
           every strip are pagers; the picture each strip is at is the app's, so a strip that is built again is \
           where it was left.",
          self.frame.demo(
            0,
            albums,
            "Pager::new(pictures, ..).horizontal()",
            &[
              "Pager::new(albums.len(), move |ctx, album| {",
              "    Column::new()",
              "        .child(",
              "            Pager::new(albums[album].pictures.len(), move |_, picture| tile(album, picture))",
              "                .horizontal()",
              "                .index(pictures[album].clone())",
              "                .height(180.0)",
              "                .mount(ctx),",
              "        )",
              "        .child(caption(album))",
              "})",
              ".index(album.clone())",
              ".mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a pager is given.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
