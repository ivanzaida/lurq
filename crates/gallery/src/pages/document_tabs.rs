//! The page of document tabs: its playground, an editor's files the page holds, which its tabs show, close, move
//! and add to; the board's bar; and a narrow bar whose tabs go into the menu at its end.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::Column,
  core::Signal,
  node::{Element, dimension::Dimension},
  ui::{DocumentTab, DocumentTabs, MenuItem, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

/// The widths the playground offers the bar: the stage's, and narrower ones its tabs do not fit.
const WIDTHS: [(&str, Dimension); 4] = [
  ("the stage's", FILL),
  ("600", Dimension::Px(600.0)),
  ("420", Dimension::Px(420.0)),
  ("300", Dimension::Px(300.0)),
];

/// An open file, as the page holds it.
#[derive(Clone, Debug, PartialEq, lurq::DevtoolsInspectable)]
struct File {
  name: String,
  dirty: bool,
  pinned: bool,
  preview: bool,
}

impl File {
  fn new(name: &str) -> Self {
    Self {
      name: name.to_owned(),
      dirty: false,
      pinned: false,
      preview: false,
    }
  }

  /// Its tab: a file of code or of text by its name.
  fn tab(&self) -> DocumentTab<String> {
    let icon = if self.name.ends_with(".rs") {
      icons::FILE_CODE
    } else {
      icons::FILE_TEXT
    };
    DocumentTab::new(self.name.clone(), &self.name)
      .icon(icon)
      .dirty(self.dirty)
      .pinned(self.pinned)
      .preview(self.preview)
  }
}

/// The board's files: main.rs pinned, blur.rs, theme.rs not saved, canvas.rs, and README.md a preview.
fn board_files() -> Vec<File> {
  vec![
    File {
      pinned: true,
      ..File::new("main.rs")
    },
    File::new("blur.rs"),
    File {
      dirty: true,
      ..File::new("theme.rs")
    },
    File::new("canvas.rs"),
    File {
      preview: true,
      ..File::new("README.md")
    },
  ]
}

pub struct DocumentTabsPage {
  frame: Frame,
  playground: Playground,
  /// The files open in the playground, and the one shown.
  files: Signal<Vec<File>>,
  shown: Signal<String>,
  /// How many new files were made, for their names.
  made: Signal<usize>,
  width: Signal<usize>,
  add: Signal<bool>,
  menu: Signal<bool>,
  disabled: Signal<bool>,
  /// The files the narrow bar shows.
  narrow: Signal<String>,
}

impl DocumentTabsPage {
  /// The playground: the page's files as tabs, which do what an editor's do to them.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (width_name, width) = WIDTHS[self.width.get().min(WIDTHS.len() - 1)];
    let (add, menu, disabled) = (self.add.get(), self.menu.get(), self.disabled.get());
    let files = self.files.get();
    let record = self.playground.recorder();
    self
      .playground
      .watch(ctx, &self.shown, |name| format!("on_change(\"{name}\")"));
    let build = {
      let (files, shown, open, made, record) = (
        files.clone(),
        self.shown.clone(),
        self.files.clone(),
        self.made.clone(),
        record.clone(),
      );
      move || {
        let (closing, moving, keeping, adding) = (open.clone(), open.clone(), open.clone(), open.clone());
        let (closed, moved, kept, added) = (record.clone(), record.clone(), record.clone(), record.clone());
        let (shown_on_close, shown_on_add, made) = (shown.clone(), shown.clone(), made.clone());
        let mut tabs = DocumentTabs::new(shown.clone(), files.iter().map(File::tab))
          .label("Open files")
          .disabled(disabled)
          .on_close(move |name: &String| {
            closed(format!("on_close(\"{name}\")"));
            let mut files = closing.get_untracked();
            let Some(at) = files.iter().position(|file| file.name == *name) else {
              return;
            };
            files.remove(at);
            // The app says which is shown next: the tab beside the one closed.
            if shown_on_close.get_untracked() == *name
              && let Some(next) = files.get(at).or_else(|| files.last())
            {
              shown_on_close.set(next.name.clone());
            }
            closing.set(files);
          })
          .on_reorder(move |from, to| {
            moved(format!("on_reorder({from}, {to})"));
            let mut files = moving.get_untracked();
            let file = files.remove(from);
            files.insert(to, file);
            moving.set(files);
          })
          .on_keep(move |name: &String| {
            kept(format!("on_keep(\"{name}\")"));
            keeping.update(|files| {
              for file in files.iter_mut().filter(|file| file.name == *name) {
                file.preview = false;
              }
            });
          });
        if add {
          tabs = tabs.on_add(move || {
            made.update(|made| *made += 1);
            let name = format!("untitled-{}.rs", made.get_untracked());
            added(format!("on_add() → {name}"));
            adding.update(|files| files.push(File::new(&name)));
            shown_on_add.set(name);
          });
        }
        if menu {
          tabs = tabs.context_menu(|name: &String| {
            vec![
              MenuItem::new("Close Others").icon(icons::CLOSE),
              MenuItem::new("Copy Path").icon(icons::COPY),
              MenuItem::new(&format!("Reveal {name}")).icon(icons::FOLDER_OPEN),
            ]
          });
        }
        tabs
      }
    };
    let shown = {
      let (build, playground) = (build.clone(), &self.playground);
      Shown::built(move |ctx| {
        Column::new()
          .width(width)
          .overflow_visible()
          .child(playground.measure(build().mount(ctx)).width(FILL))
          .into()
      })
      .measured()
    };

    let mut code = vec![
      "// shown: Signal<String>; files: the app's list of what is open".to_owned(),
      "DocumentTabs::new(shown, files.iter().map(|file| {".to_owned(),
      "    DocumentTab::new(file.path.clone(), &file.name)".to_owned(),
      "        .icon(icons::FILE_CODE)".to_owned(),
      "        .dirty(file.dirty)".to_owned(),
      "        .pinned(file.pinned)".to_owned(),
      "        .preview(file.preview)".to_owned(),
      "}))".to_owned(),
      ".label(\"Open files\")".to_owned(),
      ".on_close(|path| close(path))".to_owned(),
      ".on_reorder(|from, to| files.insert(to, files.remove(from)))".to_owned(),
      ".on_keep(|path| keep(path))".to_owned(),
    ];
    if add {
      code.push(".on_add(|| new_file())".to_owned());
    }
    if menu {
      code.push(".context_menu(|path| vec![MenuItem::new(\"Close Others\"), MenuItem::new(\"Copy Path\")])".to_owned());
    }
    if disabled {
      code.push(".disabled(true)".to_owned());
    }
    code.push(".mount(ctx)".to_owned());
    let tree = vec![
      format!("root        {width_name} × 38 · muted · line under it"),
      "├─ list      one stop of Tab · arrows move · Space and Enter show".to_owned(),
      "│  ├─ tab    pinned · 38 square · pin".to_owned(),
      "│  ├─ tab    shown · background · 2 of primary on top · icon 14 · name 13 · close 20".to_owned(),
      "│  ├─ tab    not saved · a dot of 8 in place of the cross".to_owned(),
      "│  ├─ tab    preview · its name in italics".to_owned(),
      "│  └─ add    38 square · plus 16".to_owned(),
      "└─ more      the count of what did not fit · chevron · its menu".to_owned(),
    ];

    let widths: Vec<(&str, usize)> = WIDTHS
      .iter()
      .enumerate()
      .map(|(index, (name, _))| (*name, index))
      .collect();
    let set_width = self.width.clone();
    let (to_files, to_shown, to_made, to_width) = (
      self.files.clone(),
      self.shown.clone(),
      self.made.clone(),
      self.width.clone(),
    );
    let (to_add, to_menu, to_disabled) = (self.add.clone(), self.menu.clone(), self.disabled.clone());
    self.playground.card(
      ctx,
      Spec {
        title: "Document Tabs",
        make: Arc::new(move || {
          Column::new()
            .width(width)
            .child(build().build(&lurq::ui::current_theme()))
            .into()
        }),
        shown,
        fill: true,
        forceable: false,
        facts: format!("{} open · {width_name} wide", files.len()),
        event: "on_change / on_close / on_reorder / on_add",
        code,
        tree,
        controls: vec![
          prop(
            "width",
            "of what holds it",
            self
              .playground
              .select(0, &widths, &self.width.get(), move |index| set_width.set(index)),
          ),
          divider(),
          flag("on_add", &self.add),
          flag("context_menu", &self.menu),
          flag("disabled", &self.disabled),
        ],
        changed: [self.width.get() != 0, !add, !menu, disabled, files != board_files()]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset: Arc::new(move || {
          to_files.set(board_files());
          to_shown.set("blur.rs".to_owned());
          to_made.set(0);
          to_width.set(0);
          to_add.set(true);
          to_menu.set(true);
          to_disabled.set(false);
        }),
      },
    )
  }
}

impl Component for DocumentTabsPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 1),
      files: ctx.signal(board_files()),
      shown: ctx.signal("blur.rs".to_owned()),
      made: ctx.signal(0),
      width: ctx.signal(0),
      add: ctx.signal(true),
      menu: ctx.signal(true),
      disabled: ctx.signal(false),
      narrow: ctx.signal("canvas.rs".to_owned()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let board = Column::new().width(FILL).child(
      DocumentTabs::controlled(
        "blur.rs".to_owned(),
        board_files().iter().map(File::tab),
        |_: &String| {},
      )
      .on_add(|| {})
      .build(&lurq::ui::current_theme()),
    );
    let narrow = Column::new()
      .spacing(8.0)
      .child(
        Column::new().width(420.0).child(
          DocumentTabs::new(self.narrow.clone(), board_files().iter().map(File::tab))
            .on_add(|| {})
            .mount(ctx),
        ),
      )
      .child(quiet("420 wide", 12.0));

    let rows = [
      (
        "new",
        "(Signal<T>, tabs)",
        "",
        "The tabs, of which the one whose value the signal has is shown. A press of another sets it.",
      ),
      (
        "optional",
        "(Signal<Option<T>>, tabs)",
        "",
        "The same where no document may be shown: an editor with none open.",
      ),
      (
        "controlled, controlled_optional",
        "(value, tabs, on_change)",
        "",
        "The app holds which is shown and is told what it is to be.",
      ),
      ("tab, tabs", "impl Into<DocumentTab<T>>", "", "Tabs after those it has."),
      (
        "mount",
        "(ctx)",
        "",
        "Puts the bar in the tree: it keeps what did not fit and its menu, a drag, and each context menu.",
      ),
      (
        "on_change",
        "Fn(&T)",
        "",
        "Told the tab a person shows, after the signal is set: not what the app sets.",
      ),
      (
        "on_close",
        "Fn(&T)",
        "",
        "Told the tab a person closes: its close control, a middle press, Ctrl+W. The app takes it out.",
      ),
      (
        "on_reorder",
        "Fn(usize, usize)",
        "",
        "Told a tab moved by a drag or Ctrl+Shift+PageUp and PageDown: tabs.insert(to, tabs.remove(from)).",
      ),
      (
        "on_add",
        "Fn()",
        "",
        "A press of the new tab button after the tabs, which is there once this is given.",
      ),
      (
        "on_keep",
        "Fn(&T)",
        "",
        "A double press of a preview: the app keeps it as a tab of its own.",
      ),
      (
        "context_menu",
        "Fn(&T) -> Vec<MenuItem>",
        "",
        "The commands of a tab's context menu, opened by a secondary press, Menu or Shift+F10.",
      ),
      ("label", "&str", "", "What the tabs are called to what reads the tree."),
      (
        "close_label, add_label, more_label",
        "&str",
        "\"Close\", \"New tab\", \"Show the hidden tabs\"",
        "What the close control (before the tab's name), the new tab button and More are called.",
      ),
      ("disabled", "bool", "false", "No tab takes input."),
      (
        "DocumentTab::icon",
        "impl Into<Name>",
        "",
        "An icon before the name, of the kind of the document.",
      ),
      (
        "DocumentTab::dirty",
        "bool",
        "false",
        "Changes not saved: a dot in place of the cross, which shows again under the pointer.",
      ),
      (
        "DocumentTab::pinned",
        "bool",
        "false",
        "A square of the pin at the start, with no close control, which never goes into the menu.",
      ),
      (
        "DocumentTab::preview",
        "bool",
        "false",
        "Its name in italics: the next file opened takes its place.",
      ),
      (
        "DocumentTab::on_close, disabled, id, key",
        "",
        "",
        "Its own handler, called before the bar's; dimmed; its id; the key the focus follows it by.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Desktop", "Document Tabs"],
        title: "Document Tabs",
        description: "The open documents of an editor, in a bar across its top: one of them shown, each closed by \
                      its control, moved by a drag, and those that do not fit in a menu at the end.",
        import: Some("use lurq::ui::{DocumentTab, DocumentTabs};"),
        status: Some("New"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "The page holds the files open, as an editor does: close a tab with its cross, a middle press or Ctrl+W, \
           drag one along the bar or move it with Ctrl+Shift+PageUp and PageDown, press the plus for a new file, \
           press a preview twice to keep it, and press a tab with the secondary button for its menu. Pick a narrow \
           width to see the tabs that do not fit go into the menu at the end.",
          playground,
        ),
        Section::new(
          "The board's bar",
          "A pinned tab, the tab shown on the background's color with a line of the primary color along its top, a \
           file not saved with a dot in place of its cross, and a preview in italics. A tab under the pointer shows \
           its cross whole.",
          self.frame.demo(
            0,
            board,
            "DocumentTabs::controlled(shown, tabs, on_change)",
            &[
              "DocumentTabs::controlled(shown, [",
              "    DocumentTab::new(\"main.rs\", \"main.rs\").pinned(true),",
              "    DocumentTab::new(\"blur.rs\", \"blur.rs\").icon(icons::FILE_CODE),",
              "    DocumentTab::new(\"theme.rs\", \"theme.rs\").icon(icons::FILE_CODE).dirty(true),",
              "    DocumentTab::new(\"canvas.rs\", \"canvas.rs\").icon(icons::FILE_CODE),",
              "    DocumentTab::new(\"README.md\", \"README.md\").icon(icons::FILE_TEXT).preview(true),",
              "], |path| show(path))",
              ".on_add(|| new_file())",
            ],
          ),
        ),
        Section::new(
          "What does not fit",
          "The tabs that do not fit leave the bar from its end, but the one shown and the pinned ones; the button at \
           the end says how many there are and opens a menu of them. A tab chosen there is shown, and in the bar.",
          self.frame.demo(
            1,
            narrow,
            "DocumentTabs::new(shown, tabs).mount(ctx)",
            &[
              "DocumentTabs::new(shown, tabs)",
              "    .on_add(|| new_file())",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What document tabs are given, and what each tab says of its document.",
          props_table(&rows, &["new", "tab, tabs"], self.frame.room()),
        ),
      ],
    )
  }
}
