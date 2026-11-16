//! The page of the column view: its playground, the board's file browser, children that come later, a room too
//! narrow for its columns, and its states.

use std::{sync::Arc, time::Duration};

use lurq::{
  app::{
    component::Component,
    ctx::{Ctx, Timeout},
  },
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{Badge, ColumnView, ColumnViewPart, Interaction, Length, TreeItem},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, flag},
};

type Id = &'static str;

/// How long the children of a lazy item take to come.
const LOADING: Duration = Duration::from_millis(1200);

/// What the playground's view holds: `icons` with a folder's and a file's icons, `disabled` with the demo crate's
/// item disabled, `lazy` with a registry whose crates come when it is first chosen, there once `loaded`, `counts`
/// with how many each crate holds at the end of its item.
#[derive(Clone, Copy)]
struct Shape {
  icons: bool,
  disabled: bool,
  lazy: bool,
  loaded: bool,
  counts: bool,
}

impl Shape {
  fn folder(self, id: Id, label: &str) -> TreeItem<Id> {
    if self.icons {
      TreeItem::folder(id, label)
    } else {
      TreeItem::new(id, label)
    }
  }

  fn file(self, id: Id, label: &str) -> TreeItem<Id> {
    if self.icons {
      TreeItem::file(id, label)
    } else {
      TreeItem::new(id, label)
    }
  }

  fn counted(self, item: TreeItem<Id>, count: &str) -> TreeItem<Id> {
    if self.counts {
      item.trailing(Badge::new(count))
    } else {
      item
    }
  }

  /// A workspace of crates, as a file browser shows it.
  fn project(self) -> Vec<TreeItem<Id>> {
    let mut items = vec![
      self.folder("crates", "crates").items([
        self.counted(self.folder("lurq", "lurq"), "3").items([
          self.folder("src", "src").items([
            self.file("lib.rs", "lib.rs"),
            self.file("theme.rs", "theme.rs"),
            self
              .folder("ui", "ui")
              .item(self.file("column_view.rs", "column_view.rs"))
              .item(self.file("tree_view.rs", "tree_view.rs")),
          ]),
          self
            .folder("tests", "tests")
            .item(self.file("ui_tests.rs", "ui_tests.rs")),
          self.file("lurq/Cargo.toml", "Cargo.toml"),
        ]),
        self
          .counted(self.folder("lurq_macros", "lurq_macros"), "1")
          .item(self.file("macros.rs", "lib.rs")),
        self
          .counted(self.folder("demo", "demo"), "1")
          .disabled(self.disabled)
          .item(self.file("main.rs", "main.rs")),
      ]),
      self.folder("docs", "docs").item(self.file("index.md", "index.md")),
      self
        .folder("pencil", "pencil")
        .item(self.file("board.pen", "lurq-design-system.pen")),
      self
        .folder("scripts", "scripts")
        .item(self.file("check.ps1", "check.ps1")),
    ];
    if self.lazy {
      let mut registry = self.folder("registry", "registry").lazy(true);
      if self.loaded {
        registry = registry.items([
          self.file("wgpu", "wgpu"),
          self.file("winit", "winit"),
          self.file("cosmic-text", "cosmic-text"),
        ]);
      }
      items.push(registry);
    }
    items
  }
}

const PLAIN: Shape = Shape {
  icons: true,
  disabled: false,
  lazy: false,
  loaded: false,
  counts: false,
};

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .max_width(FILL)
    .spacing(9.0)
    .child(Row::new().max_width(FILL).overflow_visible().child(shown))
    .child(quiet(caption, 12.0))
}

pub struct ColumnViewPage {
  frame: Frame,
  playground: Playground,
  icons: Signal<bool>,
  counts: Signal<bool>,
  lazy: Signal<bool>,
  disabled_item: Signal<bool>,
  disabled: Signal<bool>,
  played: Signal<Vec<Id>>,
  played_loaded: Signal<bool>,
  play_load: Timeout,
  path: Signal<Vec<Id>>,
  lazy_path: Signal<Vec<Id>>,
  loaded: Signal<bool>,
  load: Timeout,
  narrow: Signal<Vec<Id>>,
}

impl ColumnViewPage {
  /// The playground: the view as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let shape = Shape {
      icons: self.icons.get(),
      disabled: self.disabled_item.get(),
      lazy: self.lazy.get(),
      loaded: self.played_loaded.get(),
      counts: self.counts.get(),
    };
    let (disabled, path) = (self.disabled.get(), self.played.get());
    let record = self.playground.recorder();
    let (changed, activated) = (record.clone(), record);
    let (set, load) = (self.played.clone(), self.play_load.clone());
    let forced = self.playground.forced();
    // The page holds the path, so that each change is counted: `new(signal, ..)` is this with the signal set for
    // it.
    let dressed = move |path: Vec<Id>| {
      let (changed, activated, set, load) = (changed.clone(), activated.clone(), set.clone(), load.clone());
      ColumnView::controlled(path, shape.project(), move |next| {
        if next.last() == Some(&"registry") {
          load.restart();
        }
        changed(format!("on_change({next:?})"));
        set.set(next.to_vec());
      })
      .label("Files")
      .disabled(disabled)
      .force(forced)
      .on_activate(move |id| activated(format!("on_activate({id:?})")))
    };
    let shown = {
      let (dressed, path) = (dressed.clone(), path.clone());
      Shown::built(move |ctx| dressed(path).mount(ctx))
    };
    let make = {
      let path = path.clone();
      move || -> Element {
        ColumnView::controlled(path.clone(), shape.project(), |_| {})
          .disabled(disabled)
          .into()
      }
    };
    {
      let (dressed, path) = (dressed.clone(), self.played.clone());
      self.playground.window_shows(move |ctx| {
        dressed(path.get())
          .style(ColumnViewPart::Root, |s| s.width(Length::Fill))
          .mount(ctx)
      });
    }

    let (folder, file) = if shape.icons {
      ("TreeItem::folder", "TreeItem::file")
    } else {
      ("TreeItem::new", "TreeItem::new")
    };
    let mut code = vec![
      "// path: Signal<Vec<&str>>".to_owned(),
      "ColumnView::new(path, [".to_owned(),
      format!("    {folder}(\"crates\", \"crates\").items(["),
      format!(
        "        {folder}(\"lurq\", \"lurq\"){}.items([..]),",
        if shape.counts {
          ".trailing(Badge::new(\"3\"))"
        } else {
          ""
        }
      ),
      format!("        {folder}(\"lurq_macros\", \"lurq_macros\").items([..]),"),
      format!(
        "        {folder}(\"demo\", \"demo\"){}.items([..]),",
        if shape.disabled { ".disabled(true)" } else { "" }
      ),
      "    ]),".to_owned(),
      format!("    {folder}(\"docs\", \"docs\").item({file}(\"index.md\", \"index.md\")),"),
      "    ..".to_owned(),
    ];
    if shape.lazy {
      code.push(format!("    {folder}(\"registry\", \"registry\").lazy(true),"));
    }
    code.push("])".to_owned());
    code.push(".label(\"Files\")".to_owned());
    if disabled {
      code.push(".disabled(true)".to_owned());
    }
    if shape.lazy {
      code.push(".on_change(|path| /* bring what the last item of `path` holds */)".to_owned());
    }
    code.push(".on_activate(|id| /* open the file */)".to_owned());
    code.push(".mount(ctx)".to_owned());

    let tree = vec![
      format!("root        {}   an edge of 1, 8 round", self.playground.measured()),
      "└─ column   a share of the width, 160 at least; padded 4, a line of 1 at its end".to_owned(),
      "   └─ item     26 high, 6 at each side, 1 apart".to_owned(),
      format!(
        "      ├─ icon     14{}",
        if shape.icons {
          "   folder, open folder, file"
        } else {
          "   none"
        }
      ),
      "      ├─ label    body at 13, cut with an ellipsis".to_owned(),
      "      ├─ trailing what the app puts at the end".to_owned(),
      "      └─ chevron  12   where the item holds others".to_owned(),
    ];

    let flags = [
      self.counts.clone(),
      self.lazy.clone(),
      self.disabled_item.clone(),
      self.disabled.clone(),
    ];
    let (icons, reset_path, reset_loaded) = (self.icons.clone(), self.played.clone(), self.played_loaded.clone());
    self.playground.card(
      ctx,
      Spec {
        title: "Column View",
        make: Arc::new(make),
        shown,
        fill: false,
        forceable: true,
        facts: format!("path {path:?}"),
        event: "on_change",
        code,
        tree,
        controls: vec![
          flag("icons", &self.icons),
          flag("a count at the end", &self.counts),
          flag("lazy children", &self.lazy),
          divider(),
          flag("a disabled item", &self.disabled_item),
          flag("disabled", &self.disabled),
        ],
        changed: [!shape.icons, shape.counts, shape.lazy, shape.disabled, disabled]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset: Arc::new(move || {
          for flag in &flags {
            flag.set(false);
          }
          icons.set(true);
          reset_path.set(vec!["crates", "lurq"]);
          reset_loaded.set(false);
        }),
      },
    )
  }
}

impl Component for ColumnViewPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let played_loaded = ctx.signal(false);
    let set = played_loaded.clone();
    let play_load = ctx.create_timeout(LOADING, move || set.set(true));
    let loaded = ctx.signal(false);
    let set = loaded.clone();
    let load = ctx.create_timeout(LOADING, move || set.set(true));
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      icons: ctx.signal(true),
      counts: ctx.signal(false),
      lazy: ctx.signal(false),
      disabled_item: ctx.signal(false),
      disabled: ctx.signal(false),
      played: ctx.signal(vec!["crates", "lurq"]),
      played_loaded,
      play_load,
      path: ctx.signal(vec!["crates", "lurq"]),
      lazy_path: ctx.signal(Vec::new()),
      loaded,
      load,
      narrow: ctx.signal(vec!["crates", "lurq", "src"]),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let files = ColumnView::new(self.path.clone(), PLAIN.project())
      .label("Files")
      .mount(ctx);
    let load = self.load.clone();
    let lazy = ColumnView::new(
      self.lazy_path.clone(),
      Shape {
        lazy: true,
        loaded: self.loaded.get(),
        ..PLAIN
      }
      .project(),
    )
    .label("Files")
    .on_change(move |path| {
      if path.last() == Some(&"registry") {
        load.restart();
      }
    })
    .key("lazy")
    .mount(ctx);
    let narrow = ColumnView::new(self.narrow.clone(), PLAIN.project())
      .label("Files")
      .style(ColumnViewPart::Root, |s| s.width(340.0))
      .key("narrow")
      .mount(ctx);
    let state = |forced: Option<Interaction>, path: &[Id], disabled: bool| {
      ColumnView::controlled(
        path.to_vec(),
        [
          TreeItem::folder("src", "src").item(TreeItem::file("lib.rs", "lib.rs")),
          TreeItem::file("Cargo.toml", "Cargo.toml").trailing(Badge::new("2")),
          TreeItem::file("README.md", "README.md").disabled(true),
        ],
        |_| {},
      )
      .force(forced)
      .disabled(disabled)
      .style(ColumnViewPart::Root, |s| s.width(200.0).height(96.0))
    };
    let states = Row::new()
      .spacing(28.0)
      .wrap()
      .align_items(Alignment::Start)
      .child(item(
        state(None, &[], false),
        "at rest; a disabled item; a badge at the end",
      ))
      .child(item(state(None, &["Cargo.toml"], false), "the end of the path"))
      .child(item(state(Some(Interaction::Hovered), &[], false), "under the pointer"))
      .child(item(
        state(Some(Interaction::FocusVisible), &[], false),
        "focused from the keyboard",
      ))
      .child(item(state(None, &["Cargo.toml"], true), "disabled"));

    let rows = [
      (
        "path",
        "Signal<Vec<T>>",
        "",
        "The values from an item at the top down to the item chosen last, the first argument of `new`. \
         `controlled(path, items, on_change)` is the same held by the app.",
      ),
      (
        "items",
        "impl IntoIterator<Item = TreeItem<T>>",
        "",
        "What is at the top, each with what it holds: a tree's items, with `icon`, `open_icon`, `trailing`, \
         `trailing_on_hover`, `disabled`, `lazy`, `id` and `key`.",
      ),
      (
        "on_change",
        "impl Fn(&[T])",
        "",
        "Told the whole new path after a press or a key changes it, after the signal is set.",
      ),
      (
        "on_activate",
        "impl Fn(&T)",
        "",
        "Told of the item a double press or Enter activates.",
      ),
      (
        "disabled",
        "bool",
        "false",
        "No item takes input. An item has its own too.",
      ),
      (
        "label",
        "&str",
        "",
        "What the view is called, to what reads the window's tree.",
      ),
      (
        "loading_label",
        "&str",
        "\"Loading\"",
        "What the spinner of a column whose items are on their way is called.",
      ),
      (
        "mount",
        "&mut Ctx",
        "",
        "Keeps where the columns are scrolled to, starts at the end of the path and brings the last column into \
         sight as the path grows.",
      ),
      (
        "style / rules",
        "ColumnViewPart",
        "",
        "Root, Column, Item, Icon, Label, Trailing, Reveal, Chevron, Status, Spinner.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Desktop", "Column View"],
        title: "Column View",
        description: "A hierarchy as columns side by side: choosing an item of one shows what it holds in the \
                      next, as a file browser's columns do.",
        import: Some("use lurq::ui::{ColumnView, TreeItem};"),
        status: Some("Preview"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of ColumnView is a control on the right. The view on the stage is live: a press chooses an \
           item, the arrows go along a column and across the columns, and a double press or Enter activates.",
          playground,
        ),
        Section::new(
          "A file browser",
          "The board's view, bound to a `Signal<Vec<&str>>` of the path. Tab reaches the item the path ends at; Up \
           and Down choose along its column, Right goes into what it holds and Left back to the item the column is \
           of, Home and End go to the first and the last, and a letter to the next item that starts with it.",
          self.frame.demo(
            0,
            files,
            "ColumnView::new(path, items).mount(ctx)",
            &[
              "// path: Signal<Vec<&str>>, here [\"crates\", \"lurq\"]",
              "ColumnView::new(path, [",
              "    TreeItem::folder(\"crates\", \"crates\").items([",
              "        TreeItem::folder(\"lurq\", \"lurq\").items([",
              "            TreeItem::folder(\"src\", \"src\").items([..]),",
              "            TreeItem::folder(\"tests\", \"tests\").items([..]),",
              "            TreeItem::file(\"Cargo.toml\", \"Cargo.toml\"),",
              "        ]),",
              "        ..",
              "    ]),",
              "    TreeItem::folder(\"docs\", \"docs\").items([..]),",
              "    ..",
              "])",
              ".label(\"Files\")",
              ".mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Children that come later",
          "A lazy item has a chevron and, chosen, a column with a spinner until the app gives it its items: here \
           a second after `on_change` is told a path that ends at the registry.",
          self.frame.demo(
            1,
            lazy,
            "TreeItem::folder(id, label).lazy(true)",
            &[
              "// loaded: Signal<bool>, set when the crates of the registry come",
              "let mut registry = TreeItem::folder(\"registry\", \"registry\").lazy(true);",
              "if loaded.get() {",
              "    registry = registry.items(crates);",
              "}",
              "ColumnView::new(path, [.., registry])",
              "    .on_change(|path| if path.last() == Some(&\"registry\") { /* fetch them */ })",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "More columns than the room",
          "The columns share the view's width, none narrower than 160, and scroll sideways where they do not fit. \
           Mounted, the view starts at the end of its path and brings the last column into sight as the path \
           grows; the keys keep the item they go to in sight.",
          self.frame.demo(
            2,
            narrow,
            ".style(ColumnViewPart::Root, |s| s.width(340.0))",
            &[
              "ColumnView::new(path, items)",
              "    .style(ColumnViewPart::Root, |s| s.width(340.0))",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "States",
          "The board draws an item at rest, one on the path and the one the path ends at. Under the pointer and \
           held down an item is veiled, the focus from the keyboard is a ring inside it, and a disabled item is \
           dimmed and passed by.",
          self.frame.demo(
            3,
            states,
            ".force(Some(Interaction::FocusVisible))",
            &[
              "ColumnView::controlled([\"Cargo.toml\"], [",
              "    TreeItem::folder(\"src\", \"src\").item(TreeItem::file(\"lib.rs\", \"lib.rs\")),",
              "    TreeItem::file(\"Cargo.toml\", \"Cargo.toml\").trailing(Badge::new(\"2\")),",
              "    TreeItem::file(\"README.md\", \"README.md\").disabled(true),",
              "], |path| { /* the app's path */ })",
              "",
              "// Drawn as under the pointer or focused, with nothing pointing at it: for a gallery, a test.",
              ".force(Some(Interaction::FocusVisible))",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a column view is given.",
          props_table(&rows, &["path", "items"], self.frame.room()),
        ),
      ],
    )
  }
}
