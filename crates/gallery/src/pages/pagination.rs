//! The page of the pagination: its playground, the pagination bound to a
//! signal, which pages it draws as the current one moves, the compact form
//! with a select of how many to a page, and what the keys do.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{Pagination, Select},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, keys, props_table, quiet},
  playground::{Playground, Spec, divider, flag, prop},
};

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .spacing(9.0)
    .child(Row::new().child(shown))
    .child(quiet(caption, 12.0))
}

/// What is on a stage, one under another.
fn stacked() -> Column {
  Column::new().width(FILL).spacing(20.0)
}

/// The numbers of pages and of siblings the playground offers.
const PAGES: [(&str, usize); 6] = [("12", 12), ("20", 20), ("100", 100), ("7", 7), ("1", 1), ("0", 0)];
const SIBLINGS: [(&str, usize); 3] = [("1", 1), ("0", 0), ("2", 2)];
/// How many rows to a page the compact form's select offers.
const SIZES: [(usize, &str); 3] = [(10, "10"), (20, "20"), (50, "50")];
/// What each key does while the focus is on a button of a pagination.
const KEYS: [(&str, &str); 5] = [
  (
    "Tab",
    "To the next button: each is a stop of its own. A disabled one is passed by.",
  ),
  (
    "Right  Left",
    "To the next button, or the one before; from the last on to the first.",
  ),
  ("Home  End", "To the first button that takes input, or the last."),
  ("Enter  Space", "To the page of the button."),
  ("Shift Tab", "Back to the button before."),
];

pub struct PaginationPage {
  frame: Frame,
  playground: Playground,
  page: Signal<usize>,
  pages: Signal<usize>,
  siblings: Signal<usize>,
  compact: Signal<bool>,
  disabled: Signal<bool>,
  results: Signal<usize>,
  board: Signal<usize>,
  rows: Signal<usize>,
  size: Signal<usize>,
}

impl PaginationPage {
  /// The playground: the pagination as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (page, pages, siblings) = (self.page.get(), self.pages.get(), self.siblings.get());
    let (compact, disabled) = (self.compact.get(), self.disabled.get());
    let plain = {
      let bound = self.page.clone();
      move || {
        Pagination::new(bound.clone(), pages)
          .siblings(siblings)
          .compact(compact)
          .disabled(disabled)
      }
    };
    let record = self.playground.recorder();
    let pagination = plain().on_change(move |to| record(format!("on_change({to})")));

    let mut code = vec![
      "// page: Signal<usize>, from 1".to_owned(),
      format!("Pagination::new(page.clone(), {pages})"),
    ];
    if siblings != 1 {
      code.push(format!("    .siblings({siblings})"));
    }
    if compact {
      code.push("    .compact(true)".to_owned());
    }
    if disabled {
      code.push("    .disabled(true)".to_owned());
    }

    let current = if pages == 0 { 0 } else { page.clamp(1, pages) };
    let mut tree = vec![
      format!("root (navigation)   {}", self.playground.measured()),
      "└─ nav              4 apart, Left and Right move along it".to_owned(),
      format!(
        "   ├─ previous      {}",
        if current <= 1 { "disabled" } else { "ghost" }
      ),
    ];
    if compact {
      tree.insert(1, format!("├─ summary         \"Page {current} of {pages}\""));
    } else {
      let shown = Pagination::pages_shown(current, pages, siblings);
      let drawn = shown
        .iter()
        .map(|item| match item {
          lurq::ui::PageItem::Page(page) if *page == current => format!("[{page}]"),
          lurq::ui::PageItem::Page(page) => page.to_string(),
          lurq::ui::PageItem::Ellipsis => "…".to_owned(),
        })
        .collect::<Vec<_>>()
        .join(" ");
      tree.push(format!("   ├─ pages         {drawn}"));
    }
    tree.push(format!(
      "   └─ next          {}",
      if current >= pages { "disabled" } else { "ghost" }
    ));

    let (set_pages, clamp) = (self.pages.clone(), self.page.clone());
    let set_siblings = self.siblings.clone();
    let (to_page, to_pages, to_siblings) = (self.page.clone(), self.pages.clone(), self.siblings.clone());
    let flags = [self.compact.clone(), self.disabled.clone()];
    self.playground.card(
      ctx,
      Spec {
        title: "Pagination",
        make: Arc::new(move || plain().into()),
        shown: pagination.into(),
        fill: false,
        forceable: false,
        facts: format!("page {current} of {pages}"),
        event: "on_change",
        code,
        tree,
        controls: vec![
          prop(
            "pages",
            "usize",
            self.playground.select(0, &PAGES, &pages, move |pages| {
              set_pages.set(pages);
              // The page the signal has stays one of the pages.
              let page = clamp.get_untracked().clamp(1, pages.max(1));
              clamp.set(page);
            }),
          ),
          prop(
            "siblings",
            "usize",
            self
              .playground
              .select(1, &SIBLINGS, &siblings, move |siblings| set_siblings.set(siblings)),
          ),
          divider(),
          flag("compact", &self.compact),
          flag("disabled", &self.disabled),
        ],
        changed: [pages != 12, siblings != 1, compact, disabled]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset: Arc::new(move || {
          to_page.set(1);
          to_pages.set(12);
          to_siblings.set(1);
          for flag in &flags {
            flag.set(false);
          }
        }),
      },
    )
  }
}

impl Component for PaginationPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 2),
      page: ctx.signal(1),
      pages: ctx.signal(12),
      siblings: ctx.signal(1),
      compact: ctx.signal(false),
      disabled: ctx.signal(false),
      results: ctx.signal(1),
      board: ctx.signal(1),
      rows: ctx.signal(1),
      size: ctx.signal(20),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let says = |page: &Signal<usize>| format!("the signal has: {}", page.get());
    let live = stacked()
      .child(item(Pagination::new(self.results.clone(), 20), &says(&self.results)))
      .child(item(
        Pagination::new(self.board.clone(), 12).siblings(0),
        &format!("no siblings, as the board draws it · {}", says(&self.board)),
      ));
    // The same pagination on five pages of twenty: the row is as long on each.
    let moving = [1, 4, 5, 16, 20].into_iter().fold(stacked(), |column, current| {
      column.child(item(
        Pagination::controlled(current, 20, |_| {}),
        &format!("on page {current} of 20"),
      ))
    });
    let edges = stacked()
      .child(item(
        Pagination::controlled(1, 7, |_| {}),
        "7 pages: every one is drawn",
      ))
      .child(item(
        Pagination::controlled(1, 1, |_| {}),
        "1 page: previous and next are disabled",
      ))
      .child(item(Pagination::controlled(0, 0, |_| {}), "no pages"))
      .child(item(Pagination::controlled(4, 12, |_| {}).disabled(true), "disabled"));
    // The board's compact form, as wide as the board draws it, over the rows of a table.
    let size = self.size.get();
    let compact = stacked()
      .child(item(
        Column::new().width(440.0).child(
          Pagination::for_items(self.rows.clone(), 284, size)
            .compact(true)
            .page_size("Rows per page", Select::new(self.size.clone(), SIZES)),
        ),
        &format!("{size} to a page · {}", says(&self.rows)),
      ))
      .child(item(
        Pagination::new(self.results.clone(), 20).compact(true),
        "of pages and not of items",
      ));
    let keyboard = KEYS
      .into_iter()
      .fold(Column::new().width(FILL).spacing(10.0), |list, (key, does)| {
        let chord = key
          .split("  ")
          .fold(Row::new().width(140.0).spacing(6.0), |chord, key| {
            chord.child(keys(key))
          });
        list.child(
          Row::new()
            .width(FILL)
            .align_items(Alignment::Center)
            .child(chord)
            .child(quiet(does, 13.0)),
        )
      });
    let rows = [
      (
        "page",
        "Signal<usize>",
        "",
        "The current page, counted from 1. A press of another page sets it. `controlled(current, pages, go)` \
         takes the page and a handler in its place.",
      ),
      (
        "pages",
        "usize",
        "",
        "How many pages there are. 0 draws previous and next, disabled.",
      ),
      (
        "items",
        "(usize, usize)",
        "",
        "The pages of so many items, so many to a page, in place of a number of pages; `for_items` takes them \
         at once. The compact form then says 1–20 of 284.",
      ),
      (
        "siblings",
        "usize",
        "1",
        "How many pages are drawn on each side of the current one. The row has 2 × siblings + 5 things once \
         there are more pages than that.",
      ),
      (
        "compact",
        "bool",
        "false",
        "Previous and next with where the list is, and no pages.",
      ),
      (
        "page_size",
        "(&str, Select)",
        "",
        "A select of how many to a page, after its label, at the start; the pagination is then as wide as what \
         holds it.",
      ),
      (
        "summary",
        "&str",
        "",
        "What is said of where the list is, in place of its own: in another language.",
      ),
      (
        "label",
        "&str",
        "\"Pagination\"",
        "What the whole is called to what reads the tree.",
      ),
      ("on_change", "impl Fn(usize)", "", "Called with the page of each press."),
      ("disabled", "bool", "false", "No button takes input."),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Navigation", "Pagination"],
        title: "Pagination",
        description: "The pages of a list, one of them the current one: the results of a search, the rows of a \
                      table, the posts of a feed.",
        import: Some("use lurq::ui::Pagination;"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Pagination is a control on the right. The pagination on the stage is live: press a page, \
           previous or next, or Tab to it and use the arrow keys.",
          playground,
        ),
        Section::new(
          "Pagination",
          "The page holds the current page in a signal, counted from 1. The first and the last page are always \
           drawn, the current one with a page on each side, and an ellipsis where pages are left out.",
          self.frame.demo(
            0,
            live,
            "Pagination::new(page.clone(), 20)",
            &[
              "// page, board: Signal<usize>",
              "Pagination::new(page.clone(), 20)",
              "",
              "Pagination::new(board.clone(), 12).siblings(0)",
            ],
          ),
        ),
        Section::new(
          "As the current page moves",
          "The row has as many things wherever the current page is, so that it does not change its length: near \
           an end the pages run on to it, and an ellipsis always stands for two pages or more.",
          self.frame.demo(
            1,
            moving,
            "Pagination::controlled(5, 20, go)",
            &[
              "// The app holds the page: go is told the page of each press.",
              "Pagination::controlled(1, 20, go)",
              "Pagination::controlled(4, 20, go)",
              "Pagination::controlled(5, 20, go)",
              "Pagination::controlled(16, 20, go)",
              "Pagination::controlled(20, 20, go)",
            ],
          ),
        ),
        Section::new(
          "Few pages, none, disabled",
          "Previous is disabled on the first page and next on the last. With no pages both are.",
          self.frame.demo(
            2,
            edges,
            ".disabled(true)",
            &[
              "Pagination::controlled(1, 7, go)",
              "Pagination::controlled(1, 1, go)",
              "Pagination::controlled(0, 0, go)",
              "Pagination::controlled(4, 12, go).disabled(true)",
            ],
          ),
        ),
        Section::new(
          "Compact",
          "Previous and next, edged, with where the list is: the footer of a table. Given a select of how many to a \
           page it is as wide as what holds it, with the select at one end.",
          self.frame.demo(
            3,
            compact,
            ".compact(true)",
            &[
              "// rows, size: Signal<usize>",
              "Column::new().width(440.0).child(",
              "    Pagination::for_items(rows.clone(), 284, size.get())",
              "        .compact(true)",
              "        .page_size(",
              "            \"Rows per page\",",
              "            Select::new(size.clone(), [(10, \"10\"), (20, \"20\"), (50, \"50\")]),",
              "        ),",
              ")",
              "",
              "Pagination::new(page.clone(), 20).compact(true)",
            ],
          ),
        ),
        Section::new(
          "Keyboard",
          "Each button is a stop of Tab, and the arrow keys move along the row. To what reads the tree the whole \
           is a navigation, each page is called \"Page 3\", and the current one says it is.",
          keyboard,
        ),
        Section::new(
          "Props",
          "What a pagination is given.",
          props_table(&rows, &["page", "pages"], self.frame.room()),
        ),
      ],
    )
  }
}
