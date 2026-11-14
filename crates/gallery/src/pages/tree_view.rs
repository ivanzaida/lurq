//! The page of the tree view: its playground, the board's file tree, many rows selected, children that come
//! later, guide lines, and its states.

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
  ui::{Badge, Button, ButtonSize, ButtonVariant, Interaction, TreeItem, TreeView, TreeViewPart, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{props_table, quiet},
  playground::{Playground, Spec, divider, flag},
};

type Id = &'static str;

/// The width of the board's tree.
const WIDE: f32 = 220.0;

/// How long the children of a lazy row take to come.
const LOADING: Duration = Duration::from_millis(1200);

/// What the playground's tree holds: `icons` with a folder's and a file's icons, `disabled` with the demo
/// crate's row disabled, `lazy` with a registry whose crates come when it is first opened, there once `loaded`,
/// `action` with a button to add a file at the end of each folder's row, shown while the row is pointed at.
#[derive(Clone, Copy)]
struct Shape {
  icons: bool,
  disabled: bool,
  lazy: bool,
  loaded: bool,
  action: bool,
}

/// The button at the end of a folder's row that adds a file to it.
fn new_file() -> Button {
  Button::new("New file")
    .icon(icons::PLUS)
    .icon_only(true)
    .variant(ButtonVariant::Ghost)
    .size(ButtonSize::Xs)
}

impl Shape {
  fn folder(self, id: Id, label: &str) -> TreeItem<Id> {
    let folder = if self.icons {
      TreeItem::folder(id, label)
    } else {
      TreeItem::new(id, label)
    };
    if self.action {
      folder.trailing(new_file()).trailing_on_hover(true)
    } else {
      folder
    }
  }

  fn file(self, id: Id, label: &str) -> TreeItem<Id> {
    if self.icons {
      TreeItem::file(id, label)
    } else {
      TreeItem::new(id, label)
    }
  }

  /// A workspace of crates, as an editor's side bar shows it.
  fn project(self) -> Vec<TreeItem<Id>> {
    let mut items = vec![
      self.folder("crates", "crates").children([
        self.folder("crates/lurq", "lurq").children([
          self.folder("crates/lurq/src", "src").children([
            self.file("crates/lurq/src/lib.rs", "lib.rs"),
            self.file("crates/lurq/src/theme.rs", "theme.rs"),
            self
              .folder("crates/lurq/src/ui", "ui")
              .child(self.file("crates/lurq/src/ui/tree_view.rs", "tree_view.rs"))
              .child(self.file("crates/lurq/src/ui/menu.rs", "menu.rs")),
          ]),
          self.file("crates/lurq/Cargo.toml", "Cargo.toml"),
        ]),
        self
          .folder("crates/gallery", "gallery")
          .child(self.file("crates/gallery/main.rs", "main.rs"))
          .child(self.file("crates/gallery/shell.rs", "shell.rs")),
        self
          .folder("crates/demo", "demo")
          .disabled(self.disabled)
          .child(self.file("crates/demo/main.rs", "main.rs")),
      ]),
      self
        .folder("docs", "docs")
        .child(self.file("docs/index.md", "index.md")),
    ];
    if self.lazy {
      let mut registry = self.folder("registry", "registry").lazy(true);
      if self.loaded {
        registry = registry.children([
          self.file("registry/wgpu", "wgpu"),
          self.file("registry/winit", "winit"),
          self.file("registry/cosmic-text", "cosmic-text"),
        ]);
      }
      items.push(registry);
    }
    items.push(self.file("Cargo.toml", "Cargo.toml"));
    items.push(self.file("README.md", "README.md"));
    items
  }
}

/// The board's file tree.
fn board() -> Vec<TreeItem<Id>> {
  vec![
    TreeItem::folder("crates", "crates").children([
      TreeItem::folder("lurq", "lurq").children([
        TreeItem::file("lib.rs", "lib.rs"),
        TreeItem::file("theme.rs", "theme.rs"),
      ]),
      TreeItem::folder("demo", "demo").child(TreeItem::file("main.rs", "main.rs")),
    ]),
    TreeItem::file("Cargo.toml", "Cargo.toml"),
  ]
}

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .spacing(9.0)
    .child(Row::new().overflow_visible().child(shown))
    .child(quiet(caption, 12.0))
}

pub struct TreeViewPage {
  frame: Frame,
  playground: Playground,
  multiple: Signal<bool>,
  icons: Signal<bool>,
  guides: Signal<bool>,
  lazy: Signal<bool>,
  action: Signal<bool>,
  disabled: Signal<bool>,
  played_open: Signal<Vec<Id>>,
  played_selected: Signal<Vec<Id>>,
  played_loaded: Signal<bool>,
  play_load: Timeout,
  open: Signal<Vec<Id>>,
  selected: Signal<Option<Id>>,
  many_open: Signal<Vec<Id>>,
  many: Signal<Vec<Id>>,
  lazy_open: Signal<Vec<Id>>,
  loaded: Signal<bool>,
  load: Timeout,
  guide_open: Signal<Vec<Id>>,
  hover_open: Signal<Vec<Id>>,
}

impl TreeViewPage {
  /// The playground: the tree as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (multiple, guides) = (self.multiple.get(), self.guides.get());
    let shape = Shape {
      icons: self.icons.get(),
      disabled: self.disabled.get(),
      lazy: self.lazy.get(),
      loaded: self.played_loaded.get(),
      action: self.action.get(),
    };
    let (open, selected) = (self.played_open.get(), self.played_selected.get());
    // The app holds what is open and what is selected, so that each change is counted: `expanded` and
    // `selected` are this with the signal set for it.
    let record = self.playground.recorder();
    let (set_open, set_selected) = (self.played_open.clone(), self.played_selected.clone());
    let (toggled, changed, chosen, activated, expanded) =
      (record.clone(), record.clone(), record.clone(), record.clone(), record);
    let load = self.play_load.clone();
    let shown = TreeView::new(shape.project())
      .label("Files")
      .multiple(multiple)
      .guides(guides)
      .force(self.playground.forced())
      .expanded_ids(open.clone(), move |id, opened| {
        set_open.update(|held| {
          held.retain(|row| row != id);
          if opened {
            held.push(id);
          }
        });
        toggled(format!("toggle({id:?}, {opened})"));
      })
      .selected_ids(selected.clone(), move |next| {
        // One at a time, the page keeps the last, as a `Signal<Option<_>>` would.
        let next = if multiple {
          next
        } else {
          next.into_iter().last().into_iter().collect()
        };
        changed(format!("select({next:?})"));
        set_selected.set(next);
      })
      .on_select(move |id| chosen(format!("on_select({id:?})")))
      .on_activate(move |id| activated(format!("on_activate({id:?})")))
      .on_expand(move |id| {
        if *id == "registry" {
          load.restart();
        }
        expanded(format!("on_expand({id:?})"));
      })
      .style(TreeViewPart::Root, |s| s.width(260.0));

    let make = {
      let (open, selected) = (open.clone(), selected.clone());
      move || {
        TreeView::new(shape.project())
          .multiple(multiple)
          .guides(guides)
          .expanded_ids(open.clone(), |_, _| {})
          .selected_ids(selected.clone(), |_| {})
          .style(TreeViewPart::Root, |s| s.width(260.0))
          .into()
      }
    };

    let mut code = vec![
      "// open: Signal<Vec<&str>>".to_owned(),
      if multiple {
        "// selected: Signal<Vec<&str>>".to_owned()
      } else {
        "// selected: Signal<Option<&str>>".to_owned()
      },
      "TreeView::new([".to_owned(),
    ];
    let (folder, file) = if shape.icons {
      ("TreeItem::folder", "TreeItem::file")
    } else {
      ("TreeItem::new", "TreeItem::new")
    };
    code.push(format!("    {folder}(\"crates\", \"crates\").children(["));
    code.push(format!("        {folder}(\"crates/lurq\", \"lurq\").children([..]),"));
    code.push(format!(
      "        {folder}(\"crates/gallery\", \"gallery\").children([..]),"
    ));
    code.push(format!(
      "        {folder}(\"crates/demo\", \"demo\"){}.children([..]),",
      if shape.disabled { ".disabled(true)" } else { "" }
    ));
    code.push("    ]),".to_owned());
    code.push(format!("    {folder}(\"docs\", \"docs\").children([..]),"));
    if shape.lazy {
      code.push(format!("    {folder}(\"registry\", \"registry\").lazy(true),"));
    }
    code.push(format!("    {file}(\"Cargo.toml\", \"Cargo.toml\"),"));
    code.push(format!("    {file}(\"README.md\", \"README.md\"),"));
    code.push("])".to_owned());
    code.push(".label(\"Files\")".to_owned());
    code.push(".expanded(open)".to_owned());
    code.push(".selected(selected)".to_owned());
    if multiple {
      code.push(".multiple(true)".to_owned());
    }
    if guides {
      code.push(".guides(true)".to_owned());
    }
    if shape.lazy {
      code.push(".on_expand(|id| /* bring the children of `id` */)".to_owned());
    }
    if shape.action {
      code.push("// each folder: .trailing(new_file_button).trailing_on_hover(true)".to_owned());
    }
    code.push(".on_activate(|id| /* open the file */)".to_owned());

    let tree = vec![
      format!("root        {}   rows 2 apart", self.playground.measured()),
      "└─ row      28 high, 8 at each side, 16 in for each level".to_owned(),
      format!(
        "   ├─ indent   a step of 16 for each level{}",
        if guides { ", a guide of 1 in each" } else { "" }
      ),
      "   └─ content  6 apart".to_owned(),
      "      ├─ chevron  14   right closed, down open, a spinner while loading".to_owned(),
      format!(
        "      ├─ icon     16{}",
        if shape.icons {
          "   folder, open folder, file"
        } else {
          "   none"
        }
      ),
      "      ├─ label    body at 13, 500 while selected".to_owned(),
      "      └─ trailing what the app puts at the end".to_owned(),
    ];

    let flags = [
      self.multiple.clone(),
      self.guides.clone(),
      self.lazy.clone(),
      self.disabled.clone(),
      self.action.clone(),
    ];
    let (icons, reset_open, reset_selected, reset_loaded) = (
      self.icons.clone(),
      self.played_open.clone(),
      self.played_selected.clone(),
      self.played_loaded.clone(),
    );
    self.playground.card(
      ctx,
      Spec {
        title: "Tree View",
        make: Arc::new(make),
        shown: shown.into(),
        fill: false,
        forceable: true,
        facts: format!("{} open, {} selected", open.len(), selected.len()),
        event: "select",
        code,
        tree,
        controls: vec![
          flag("multiple", &self.multiple),
          divider(),
          flag("icons", &self.icons),
          flag("guides", &self.guides),
          flag("lazy children", &self.lazy),
          flag("a disabled row", &self.disabled),
          flag("an action on hover", &self.action),
        ],
        changed: [multiple, !shape.icons, guides, shape.lazy, shape.disabled, shape.action]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset: Arc::new(move || {
          for flag in &flags {
            flag.set(false);
          }
          icons.set(true);
          reset_open.set(vec!["crates", "crates/lurq"]);
          reset_selected.set(Vec::new());
          reset_loaded.set(false);
        }),
      },
    )
  }
}

impl Component for TreeViewPage {
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
      multiple: ctx.signal(false),
      icons: ctx.signal(true),
      guides: ctx.signal(false),
      lazy: ctx.signal(false),
      action: ctx.signal(false),
      disabled: ctx.signal(false),
      played_open: ctx.signal(vec!["crates", "crates/lurq"]),
      played_selected: ctx.signal(Vec::new()),
      played_loaded,
      play_load,
      open: ctx.signal(vec!["crates", "lurq"]),
      selected: ctx.signal(Some("lib.rs")),
      many_open: ctx.signal(vec!["crates", "crates/lurq", "crates/lurq/src"]),
      many: ctx.signal(vec!["crates/lurq/src/lib.rs", "crates/lurq/src/theme.rs"]),
      lazy_open: ctx.signal(Vec::new()),
      loaded,
      load,
      guide_open: ctx.signal(vec!["crates", "crates/lurq", "crates/lurq/src", "crates/lurq/src/ui"]),
      hover_open: ctx.signal(vec!["crates", "crates/lurq"]),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let plain = Shape {
      icons: true,
      disabled: false,
      lazy: false,
      loaded: false,
      action: false,
    };
    let files = TreeView::new(board())
      .label("Files")
      .expanded(self.open.clone())
      .selected(self.selected.clone())
      .style(TreeViewPart::Root, |s| s.width(WIDE));
    let many = TreeView::new(plain.project())
      .label("Files")
      .expanded(self.many_open.clone())
      .selected(self.many.clone())
      .multiple(true)
      .style(TreeViewPart::Root, |s| s.width(260.0));
    let load = self.load.clone();
    let lazy = TreeView::new(
      Shape {
        lazy: true,
        loaded: self.loaded.get(),
        ..plain
      }
      .project(),
    )
    .label("Files")
    .expanded(self.lazy_open.clone())
    .on_expand(move |id| {
      if *id == "registry" {
        load.restart();
      }
    })
    .style(TreeViewPart::Root, |s| s.width(260.0));
    let big = ctx.mount::<BigTree>(());
    let hover = TreeView::new(Shape { action: true, ..plain }.project())
      .label("Files")
      .expanded(self.hover_open.clone())
      .style(TreeViewPart::Root, |s| s.width(260.0));
    let guides = TreeView::new(plain.project())
      .label("Files")
      .expanded(self.guide_open.clone())
      .guides(true)
      .style(TreeViewPart::Root, |s| s.width(260.0));
    let state = |forced: Option<Interaction>, selected: Option<Id>| {
      TreeView::new([
        TreeItem::folder("src", "src"),
        TreeItem::file("lib.rs", "lib.rs").trailing(Badge::new("2")),
        TreeItem::file("main.rs", "main.rs").disabled(true),
      ])
      .selected_ids(selected, |_| {})
      .force(forced)
      .style(TreeViewPart::Root, |s| s.width(180.0))
    };
    let states = Row::new()
      .spacing(28.0)
      .wrap()
      .align_items(Alignment::Start)
      .child(item(state(None, None), "at rest; a disabled row; a badge at the end"))
      .child(item(state(None, Some("lib.rs")), "selected"))
      .child(item(state(Some(Interaction::Hovered), None), "under the pointer"))
      .child(item(
        state(Some(Interaction::FocusVisible), None),
        "focused from the keyboard",
      ));

    let rows = [
      (
        "items",
        "impl IntoIterator<Item = TreeItem<T>>",
        "",
        "The rows at the top, each with what it holds: `TreeItem::new(id, label)`, or `folder` and `file` with \
         their icons; `icon`, `open_icon`, `trailing`, `disabled`, `children`, `lazy`.",
      ),
      (
        "expanded",
        "Signal<Vec<T>>",
        "none",
        "The rows that are open. Opening a row adds it, closing one takes it out. `expanded_ids(ids, toggle)` is \
         the same held by the app.",
      ),
      (
        "selected",
        "Signal<Option<T>> / Signal<Vec<T>>",
        "none",
        "What is selected. `selected_ids(ids, change)` is the same held by the app.",
      ),
      (
        "multiple",
        "bool",
        "false",
        "Any number selected, with Ctrl and Shift as a list of files has them, Space and Ctrl+A.",
      ),
      (
        "guides",
        "bool",
        "false",
        "A line down each level, under the chevron of the row it is in.",
      ),
      (
        "disabled",
        "bool",
        "false",
        "No row takes input. A row has its own too.",
      ),
      (
        "label",
        "&str",
        "",
        "What the tree is called, to what reads the window's tree.",
      ),
      (
        "on_select",
        "impl Fn(&T)",
        "",
        "Told of the row a press or Space selects.",
      ),
      (
        "on_activate",
        "impl Fn(&T)",
        "",
        "Told of the row a double press or Enter activates. A row that opens opens or closes as well.",
      ),
      (
        "on_expand",
        "impl Fn(&T)",
        "",
        "Told of each row as it opens: where a lazy row's children are brought.",
      ),
      (
        "style / rules",
        "TreeViewPart",
        "",
        "Root, Row, Indent, Guide, Content, Chevron, Icon, Label, Trailing.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Navigation", "Tree View"],
        title: "Tree View",
        description: "Things inside things, one level under the other: the files of a project in their folders, \
                      the layers of a scene, settings in their groups.",
        import: Some("use lurq::ui::{TreeItem, TreeView};"),
        status: Some("Preview"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of TreeView is a control on the right. The tree on the stage is live: a press selects a row, \
           a press of its chevron opens it, a double press activates it, and the keys are a tree's.",
          playground,
        ),
        Section::new(
          "A file tree",
          "The board's tree. The page holds what is open in a `Signal<Vec<&str>>` and what is selected in a \
           `Signal<Option<&str>>`. Tab reaches the selected row; Up and Down move through the rows shown, Right \
           opens a folder and goes into it, Left closes it and goes out, `*` opens every folder beside the focused \
           one, a letter goes to the next row that starts with it, Space selects and Enter activates.",
          self.frame.demo(
            0,
            files,
            "TreeView::new(items).expanded(open).selected(selected)",
            &[
              "// open: Signal<Vec<&str>>, selected: Signal<Option<&str>>",
              "TreeView::new([",
              "    TreeItem::folder(\"crates\", \"crates\").children([",
              "        TreeItem::folder(\"lurq\", \"lurq\").children([",
              "            TreeItem::file(\"lib.rs\", \"lib.rs\"),",
              "            TreeItem::file(\"theme.rs\", \"theme.rs\"),",
              "        ]),",
              "        TreeItem::folder(\"demo\", \"demo\").child(TreeItem::file(\"main.rs\", \"main.rs\")),",
              "    ]),",
              "    TreeItem::file(\"Cargo.toml\", \"Cargo.toml\"),",
              "])",
              ".label(\"Files\")",
              ".expanded(open)",
              ".selected(selected)",
            ],
          ),
        ),
        Section::new(
          "Many selected",
          "With `multiple` and a `Signal<Vec<&str>>`: a press with Ctrl adds a row or takes it out, one with Shift \
           selects the rows from the last selected to it, Shift with the arrows selects on the way, Space adds or \
           takes out the focused row, and Ctrl+A selects every row shown.",
          self.frame.demo(
            1,
            many,
            ".selected(many).multiple(true)",
            &[
              "// open, many: Signal<Vec<&str>>",
              "TreeView::new(project)",
              "    .expanded(open)",
              "    .selected(many)",
              "    .multiple(true)",
            ],
          ),
        ),
        Section::new(
          "Children that come later",
          "A lazy row opens as a folder does, with a spinner in place of its chevron until the app gives it its \
           children: here a second after `on_expand` is told.",
          self.frame.demo(
            2,
            lazy,
            "TreeItem::folder(id, label).lazy(true)",
            &[
              "// loaded: Signal<bool>, set when the crates of the registry come",
              "let mut registry = TreeItem::folder(\"registry\", \"registry\").lazy(true);",
              "if loaded.get() {",
              "    registry = registry.children(crates);",
              "}",
              "TreeView::new([.., registry])",
              "    .expanded(open)",
              "    .on_expand(|id| if *id == \"registry\" { /* fetch them */ })",
            ],
          ),
        ),
        Section::new(
          "Guide lines",
          "A line down each level, under the chevron of the row it is in: for a deep tree, where the indent alone \
           is hard to follow.",
          self.frame.demo(
            3,
            guides,
            ".guides(true)",
            &["TreeView::new(project).expanded(open).guides(true)"],
          ),
        ),
        Section::new(
          "Ten thousand rows",
          "A hundred folders of a hundred files, every folder open: `mount(ctx)` puts the rows in the engine's            virtual list, which builds those near the view. The keys reach every row, built or not: End goes to the            last, a letter to the next row it starts, PageDown a view down, Left from a file to its folder however            far up, Shift with them and Ctrl+A select over all of them.",
          self.frame.demo(
            5,
            big,
            "TreeView::new(items).mount(ctx)",
            &[
              "// open, selected: Signal<Vec<u32>>",
              "TreeView::new((0..100).map(|i| {",
              "    TreeItem::folder(i * 1000, &format!(\"Folder {i}\"))",
              "        .children((0..100).map(|j| TreeItem::file(i * 1000 + j + 1, &format!(\"file {i}.{j}\"))))",
              "}))",
              ".expanded(open)",
              ".selected(selected)",
              ".multiple(true)",
              ".style(TreeViewPart::Root, |s| s.width(320.0).height(360.0))",
              ".mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "An action on hover",
          "What an item marks with `trailing_on_hover` stands at the row's end only while the pointer is on the row            or the focus is in it, and always under a finger: the button that adds a file to a folder.",
          self.frame.demo(
            6,
            hover,
            ".trailing(button).trailing_on_hover(true)",
            &[
              "TreeItem::folder(\"src\", \"src\")",
              "    .trailing(",
              "        Button::new(\"New file\")",
              "            .icon(icons::PLUS)",
              "            .icon_only(true)",
              "            .variant(ButtonVariant::Ghost)",
              "            .size(ButtonSize::Xs),",
              "    )",
              "    .trailing_on_hover(true)",
            ],
          ),
        ),
        Section::new(
          "States",
          "The board draws a row at rest and a selected one. Under the pointer and held down a row is veiled, the \
           focus from the keyboard is a ring inside the row, and a disabled row is dimmed and passed by. What the \
           app puts at a row's end stands there in the muted foreground.",
          self.frame.demo(
            4,
            states,
            ".force(Some(Interaction::FocusVisible))",
            &[
              "TreeView::new([",
              "    TreeItem::folder(\"src\", \"src\"),",
              "    TreeItem::file(\"lib.rs\", \"lib.rs\").trailing(Badge::new(\"2\")),",
              "    TreeItem::file(\"main.rs\", \"main.rs\").disabled(true),",
              "])",
              "",
              "// Drawn as under the pointer or focused, with nothing pointing at it: for a gallery, a test.",
              ".force(Some(Interaction::FocusVisible))",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a tree is given.",
          props_table(&rows, &["items"], self.frame.room()),
        ),
      ],
    )
  }
}

/// A hundred folders of a hundred files, in a component of its own so that the page's other signals do not build
/// its ten thousand items again.
struct BigTree {
  open: Signal<Vec<u32>>,
  selected: Signal<Vec<u32>>,
}

impl Component for BigTree {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      open: ctx.signal((0..100).map(|i| i * 1000).collect()),
      selected: ctx.signal(Vec::new()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    TreeView::new((0..100u32).map(|i| {
      TreeItem::folder(i * 1000, &format!("Folder {i}"))
        .children((0..100u32).map(move |j| TreeItem::file(i * 1000 + j + 1, &format!("file {i}.{j}"))))
    }))
    .label("Ten thousand files")
    .expanded(self.open.clone())
    .selected(self.selected.clone())
    .multiple(true)
    .style(TreeViewPart::Root, |s| s.width(320.0).height(360.0))
    .mount(ctx)
  }
}
