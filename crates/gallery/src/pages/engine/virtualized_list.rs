//! The page of the virtualized list: a list of any length that builds only the rows in and near its view, each a
//! component of the app's mounted by its key. Its playground is a list of up to a hundred thousand rows with a
//! count of the rows built; its sections go to a row the list has not built, and load more rows at the end.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{PaletteColor, RadiusSize},
  },
  components::{Column, Row, VirtualizedList},
  core::Signal,
  layout::{Alignment, text_style::FontWeight},
  node::Element,
  ui::{Button, ButtonSize, ButtonVariant},
};

use super::kit::controls;
use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, line, mono, props_table, quiet, text},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

const COUNTS: [(&str, usize); 4] = [("100", 100), ("1 000", 1_000), ("10 000", 10_000), ("100 000", 100_000)];
/// The heights a row may have; 0 is each of them in turn, so that the rows differ.
const HEIGHTS: [(&str, i32); 4] = [("28", 28), ("44", 44), ("72", 72), ("mixed", 0)];
const MIXED: [f32; 3] = [28.0, 72.0, 44.0];
const OVERSCANS: [(&str, i32); 3] = [("0", 0), ("256", 256), ("1024", 1024)];
const LIST_HEIGHTS: [(&str, i32); 2] = [("240", 240), ("400", 400)];
/// The rows the list of the section that goes to a row has, and those its buttons go to.
const REVEAL_ROWS: usize = 10_000;
const REVEALED: [usize; 4] = [1, 500, 5_000, 9_999];
/// How many rows the list that loads more starts with, and adds each time it reaches its end.
const PAGE: usize = 40;

/// A row of a list: its number, its height, and whether it counts itself in the playground's count.
#[derive(Clone, PartialEq, lurq::DevtoolsInspectable)]
pub struct Item {
  index: usize,
  height: f32,
  counted: bool,
}

/// The rows of a list, `count` of them, `height` high or of the mixed heights.
fn items(count: usize, height: i32, counted: bool) -> Arc<Vec<Item>> {
  Arc::new(
    (0..count)
      .map(|index| Item {
        index,
        height: if height > 0 {
          height as f32
        } else {
          MIXED[index % MIXED.len()]
        },
        counted,
      })
      .collect(),
  )
}

/// How many rows of the playground's list are built now, and have been since the page opened: what the page
/// provides to the rows, which count themselves as they are mounted and unmounted.
#[derive(Clone)]
struct Built {
  now: Signal<usize>,
  ever: Signal<usize>,
}

/// A row of the lists of the page: a component, which the list mounts when the row comes near the view and
/// unmounts when it goes far from it.
struct ListRow {
  built: Option<Built>,
}

impl Component for ListRow {
  type Props = Item;

  fn create(ctx: &mut Ctx) -> Self {
    let counted = ctx.props::<Item>().counted;
    Self {
      built: if counted { ctx.use_context::<Built>() } else { None },
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let item = ctx.props::<Item>();
    Row::new()
      .width(FILL)
      .height(item.height)
      .padding_horizontal(12.0)
      .spacing(12.0)
      .align_items(Alignment::Center)
      .background(if item.index.is_multiple_of(2) {
        PaletteColor::Card
      } else {
        PaletteColor::Muted
      })
      .child(
        text(&format!("Row {}", item.index + 1), 13.0, FontWeight::Medium)
          .flex(1.0)
          .min_width(0.0),
      )
      .child(quiet(&format!("{} high", item.height), 11.0).nowrap())
  }

  fn on_mounted(&self) {
    if let Some(built) = &self.built {
      built.now.update(|now| *now += 1);
      built.ever.update(|ever| *ever += 1);
    }
  }

  fn on_unmounted(&self) {
    if let Some(built) = &self.built {
      built.now.update(|now| *now = now.saturating_sub(1));
    }
  }
}

/// The frame every list of the page is in: its edge and its corners.
fn framed(list: Element) -> Column {
  Column::new()
    .width(320.0)
    .max_width(FILL)
    .clip()
    .rounded(RadiusSize::Md)
    .border(line())
    .child(list)
}

/// The list of the playground, as its props say.
#[derive(Clone, Copy)]
struct Listed {
  count: usize,
  height: i32,
  overscan: i32,
  list_height: i32,
  estimate: bool,
}

impl Listed {
  fn build(self, ctx: &mut Ctx, items: Arc<Vec<Item>>, record: Arc<dyn Fn(String) + Send + Sync>) -> Element {
    let (top, bottom) = (record.clone(), record);
    let mut list = VirtualizedList::new_shared(ctx, items)
      .width(FILL)
      .height(self.list_height as f32)
      .overscan_px(self.overscan as f32)
      .on_top_reached(move || top("on_top_reached".to_owned()))
      .on_bottom_reached(move || bottom("on_bottom_reached".to_owned()));
    if self.estimate {
      list = list.estimate_height(|item: &Item| item.height);
    }
    framed(list.mount_keyed::<ListRow, _, _, _>(|item| item.index, Item::clone)).into()
  }

  fn code(self) -> Vec<String> {
    let mut code = vec![
      format!("let items: Vec<Item> = (0..{}).map(Item::new).collect();", self.count),
      String::new(),
      "VirtualizedList::new(ctx, items)".to_owned(),
      format!("    .height({}.0)", self.list_height),
    ];
    if self.overscan != 256 {
      code.push(format!("    .overscan_px({}.0)", self.overscan));
    }
    if self.estimate {
      code.push("    .estimate_height(|item| item.height)".to_owned());
    }
    code.push("    .on_bottom_reached(|| /* .. */)".to_owned());
    code.push("    .mount_keyed::<ListRow, _, _, _>(|item| item.index, Item::clone)".to_owned());
    code
  }
}

pub struct VirtualizedListPage {
  frame: Frame,
  playground: Playground,
  count: Signal<usize>,
  height: Signal<i32>,
  overscan: Signal<i32>,
  list_height: Signal<i32>,
  estimate: Signal<bool>,
  built: Built,
  /// The rows of the playground's list, made again only when their count or their height changes: a list of a
  /// hundred thousand is not made at every render.
  items: Mutex<(usize, i32, Arc<Vec<Item>>)>,
  /// The rows of the list that goes to a row, the row it goes to, and how many times it was asked.
  reveal_items: Arc<Vec<Item>>,
  reveal: Signal<usize>,
  asked: Signal<usize>,
  /// How many rows the list that loads more has.
  loaded: Signal<usize>,
}

impl VirtualizedListPage {
  fn items(&self, count: usize, height: i32) -> Arc<Vec<Item>> {
    let mut kept = self.items.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if kept.0 != count || kept.1 != height {
      *kept = (count, height, items(count, height, true));
    }
    kept.2.clone()
  }

  fn play(&self, ctx: &mut Ctx) -> Element {
    let listed = Listed {
      count: self.count.get(),
      height: self.height.get(),
      overscan: self.overscan.get(),
      list_height: self.list_height.get(),
      estimate: self.estimate.get(),
    };
    let items = self.items(listed.count, listed.height);
    let record: Arc<dyn Fn(String) + Send + Sync> = Arc::new(self.playground.recorder());
    {
      let items = items.clone();
      self
        .playground
        .window_shows(move |ctx| listed.build(ctx, items.clone(), Arc::new(|_| {})));
    }
    let (now, ever) = (self.built.now.get(), self.built.ever.get());
    let tree = vec![
      format!("virtualized list   {}", self.playground.measured()),
      "└─ scroll vertical".to_owned(),
      "   ├─ spacer   the rows before the view, estimated".to_owned(),
      format!("   ├─ {now} rows   in and near the view"),
      "   └─ spacer   the rows after it".to_owned(),
    ];
    let (set_count, set_height, set_overscan, set_list_height) = (
      self.count.clone(),
      self.height.clone(),
      self.overscan.clone(),
      self.list_height.clone(),
    );
    let controls = vec![
      prop(
        "items",
        "impl IntoIterator<Item = T>",
        self
          .playground
          .select(0, &COUNTS, &listed.count, move |value| set_count.set(value)),
      ),
      prop(
        "row height",
        "the row's own",
        self
          .playground
          .select(1, &HEIGHTS, &listed.height, move |value| set_height.set(value)),
      ),
      prop(
        "overscan_px",
        "f32",
        self
          .playground
          .select(2, &OVERSCANS, &listed.overscan, move |value| set_overscan.set(value)),
      ),
      prop(
        "height",
        "impl Into<Dimension>",
        self
          .playground
          .select(3, &LIST_HEIGHTS, &listed.list_height, move |value| {
            set_list_height.set(value)
          }),
      ),
      divider(),
      flag("estimate_height", &self.estimate),
    ];
    let reset = {
      let (count, height, overscan, list_height, estimate) = (
        self.count.clone(),
        self.height.clone(),
        self.overscan.clone(),
        self.list_height.clone(),
        self.estimate.clone(),
      );
      Arc::new(move || {
        count.set(10_000);
        height.set(44);
        overscan.set(256);
        list_height.set(240);
        estimate.set(false);
      })
    };
    self.playground.card(
      ctx,
      Spec {
        title: "Virtualized List",
        make: Arc::new(|| Column::new().into()),
        shown: Shown::built(move |ctx| listed.build(ctx, items, record)),
        fill: true,
        forceable: false,
        facts: format!(
          "{} rows · {now} built now · {ever} built since the page opened",
          listed.count
        ),
        event: "on_top_reached / on_bottom_reached",
        code: listed.code(),
        tree,
        controls,
        changed: [
          listed.count != 10_000,
          listed.height != 44,
          listed.overscan != 256,
          listed.list_height != 240,
          listed.estimate,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset,
      },
    )
  }

  /// A list of ten thousand rows, and buttons that bring one of them into view, built or not.
  fn reveal(&self, ctx: &mut Ctx) -> Column {
    let mut buttons = controls();
    for row in REVEALED {
      let (reveal, asked) = (self.reveal.clone(), self.asked.clone());
      buttons = buttons.child(
        Button::new(&format!("Row {row}"))
          .variant(ButtonVariant::Outline)
          .size(ButtonSize::Sm)
          .on_click(move |_| {
            reveal.set(row - 1);
            asked.update(|asked| *asked += 1);
          }),
      );
    }
    let list = VirtualizedList::new_shared(ctx, self.reveal_items.clone())
      .width(FILL)
      .height(240.0)
      .reveal_key(Some(self.reveal.get().to_string()))
      .reveal_request(self.asked.get() as u64)
      .mount_keyed::<ListRow, _, _, _>(|item| item.index, Item::clone);
    Column::new()
      .width(FILL)
      .spacing(16.0)
      .child(buttons)
      .child(framed(list))
  }

  /// A list that adds rows each time it reaches its end, as one that loads its pages does.
  fn more(&self, ctx: &mut Ctx) -> Column {
    let loaded = self.loaded.get();
    let more = self.loaded.clone();
    let list = VirtualizedList::new(
      ctx,
      (0..loaded).map(|index| Item {
        index,
        height: 36.0,
        counted: false,
      }),
    )
    .width(FILL)
    .height(240.0)
    .on_bottom_reached(move || more.update(|loaded| *loaded += PAGE))
    .mount_keyed::<ListRow, _, _, _>(|item| item.index, Item::clone);
    Column::new()
      .width(FILL)
      .spacing(12.0)
      .child(mono(&format!("{loaded} rows loaded"), 12.0).color(PaletteColor::MutedForeground))
      .child(framed(list))
  }
}

impl Component for VirtualizedListPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let built = Built {
      now: ctx.signal(0),
      ever: ctx.signal(0),
    };
    // Provided from here for the page's life: the rows of the playground's list count themselves into it.
    ctx.provide(built.clone());
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 4),
      count: ctx.signal(10_000),
      height: ctx.signal(44),
      overscan: ctx.signal(256),
      list_height: ctx.signal(240),
      estimate: ctx.signal(false),
      built,
      items: Mutex::new((0, 0, Arc::default())),
      reveal_items: items(REVEAL_ROWS, 36, false),
      reveal: ctx.signal(0),
      asked: ctx.signal(0),
      loaded: ctx.signal(PAGE),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let rows = [
      (
        "new / new_shared",
        "(&mut Ctx, items)",
        "",
        "The list of the items: new_shared takes an Arc the app keeps, so that a render of a long list copies \
         nothing.",
      ),
      (
        "mount_keyed",
        "(key of an item, props of an item)",
        "",
        "The component each row is, mounted with the key and the props of its item when the row comes near the \
         view, and unmounted when it goes far from it: what a row keeps in itself starts over when it comes back.",
      ),
      (
        "height / max_height / flex",
        "impl Into<Dimension>",
        "",
        "The view: a list with a height scrolls in it; one with a max_height alone is as high as its rows up to it.",
      ),
      (
        "overscan_px",
        "f32",
        "256",
        "How far past each edge of the view rows are built, so that a scroll shows built rows.",
      ),
      (
        "estimate_height",
        "Fn(&T) -> f32",
        "",
        "The height of a row not measured yet. Without it, the rows measured give their average.",
      ),
      (
        "bootstrap_rows",
        "usize",
        "16",
        "The rows built before the view is laid out, whose heights size the first window.",
      ),
      (
        "reveal_key / reveal_request",
        "Option<String> / u64",
        "",
        "Brings the row with the key into view, built or not, when the key or the request changes.",
      ),
      (
        "on_top_reached / on_bottom_reached",
        "Fn()",
        "",
        "Hears the list reach an end: where an app loads more.",
      ),
      (
        "with_scroll_state / scrollbar",
        "ScrollState / ScrollBarStyle",
        "",
        "As a scroll container's.",
      ),
    ];
    let playground = self.play(ctx);
    let reveal = self.reveal(ctx);
    let more = self.more(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Engine", "Virtualized List"],
        title: "Virtualized List",
        description: "A list that builds only the rows in and near its view, so that a list of any length costs \
                      about as much as a screen of rows. Each row is a component of the app's, mounted by its key.",
        import: Some("use lurq::components::VirtualizedList;"),
        status: Some("Stable"),
        since: None,
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Up to a hundred thousand rows. Scroll it and watch the count of rows built: it stays about what the view \
           and the overscan hold, while the count of those built since the page opened goes up. With mixed \
           heights the bar's thumb settles as rows are measured, unless each row's height is estimated.",
          playground,
        ),
        Section::new(
          "Going to a row",
          "Ten thousand rows. A button brings its row into view whether the list has built it or not: from the \
           heights it has measured and estimated, the list places it, and settles once the rows around it are \
           measured. A row in view already does not move.",
          self.frame.demo(
            0,
            reveal,
            ".reveal_key(Some(key))",
            &[
              "// reveal: Signal<usize>, asked: Signal<usize>; a button sets reveal and adds one to asked.",
              "VirtualizedList::new_shared(ctx, items.clone())",
              "    .height(240.0)",
              "    .reveal_key(Some(reveal.get().to_string()))",
              "    .reveal_request(asked.get() as u64)",
              "    .mount_keyed::<ListRow, _, _, _>(|item| item.index, Item::clone)",
            ],
          ),
        ),
        Section::new(
          "Loading more",
          "Forty rows, and forty more each time the list reaches its end, as a list that loads its pages does.",
          self.frame.demo(
            1,
            more,
            ".on_bottom_reached(|| ..)",
            &[
              "VirtualizedList::new(ctx, (0..loaded.get()).map(Item::new))",
              "    .height(240.0)",
              "    .on_bottom_reached(move || loaded.update(|loaded| *loaded += 40))",
              "    .mount_keyed::<ListRow, _, _, _>(|item| item.index, Item::clone)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a virtualized list is given.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
