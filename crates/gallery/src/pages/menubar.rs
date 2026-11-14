//! The page of the menubar: its playground, an app's menus with checks, radios, shortcuts and a submenu, and a bar
//! with a disabled title.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::{ElementRef, Signal},
  node::Element,
  ui::{Menu, MenuEntry, MenuItem, Menubar, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{props_table, quiet},
  playground::{Playground, Spec, divider, field, flag, prop},
};

/// The zooms the View menu offers, one of which is chosen.
const ZOOMS: [&str; 3] = ["50%", "100%", "200%"];

pub struct MenubarPage {
  frame: Frame,
  playground: Playground,
  // The playground.
  title: Signal<String>,
  disabled: Signal<bool>,
  help_disabled: Signal<bool>,
  submenu: Signal<bool>,
  play_open: Signal<Option<usize>>,
  play_export: (ElementRef, Signal<bool>),
  // An app's menus.
  app_open: Signal<Option<usize>>,
  app_export: (ElementRef, Signal<bool>),
  chosen: Signal<String>,
  grid: Signal<bool>,
  zoom: Signal<usize>,
  // A bar with a disabled title.
  quiet_open: Signal<Option<usize>>,
}

impl MenubarPage {
  /// What a choice does on this page: the signal says which it was.
  fn says(chosen: &Signal<String>, name: &str) -> impl Fn() + Send + Sync + use<> {
    let (chosen, name) = (chosen.clone(), name.to_owned());
    move || chosen.set(name.clone())
  }

  /// The playground: a bar of four menus, the first called as the panel says.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let title = self.title.get();
    let (disabled, help_disabled, submenu) = (self.disabled.get(), self.help_disabled.get(), self.submenu.get());
    let record = self.playground.recorder();
    let forced = self.playground.forced();
    let first = if title.is_empty() {
      "File".to_owned()
    } else {
      title.clone()
    };
    let make = {
      let first = first.clone();
      move |open: Signal<Option<usize>>,
            export: (ElementRef, Signal<bool>),
            told: &dyn Fn(&str) -> Box<dyn Fn() + Send + Sync>| {
        let mut file: Vec<MenuEntry> = vec![
          MenuItem::new("New")
            .icon(icons::FILE)
            .shortcut("Ctrl+N")
            .on_select(told("New"))
            .into(),
          MenuItem::new("Open…")
            .shortcut("Ctrl+O")
            .on_select(told("Open…"))
            .into(),
          MenuEntry::Separator,
        ];
        if submenu {
          file.push(
            MenuItem::new("Export as")
              .submenu(
                Menu::new(export.0, export.1)
                  .items(["PNG", "SVG", "PDF"].map(|format| MenuItem::new(format).on_select(told(format)))),
              )
              .into(),
          );
        }
        file.push(MenuItem::new("Quit").shortcut("Ctrl+Q").on_select(told("Quit")).into());
        Menubar::new(open)
          .disabled(disabled)
          .force(forced)
          .menu(&first, file)
          .menu(
            "Edit",
            [
              MenuItem::new("Undo").shortcut("Ctrl+Z").on_select(told("Undo")),
              MenuItem::new("Redo").shortcut("Ctrl+Y").disabled(true),
            ],
          )
          .menu(
            "View",
            [MenuItem::new("Show grid").checked(true).on_select(told("Show grid"))],
          )
          .menu("Help", [MenuItem::new("About").on_select(told("About"))])
          .title_disabled("Help", help_disabled)
      }
    };
    let told = |name: &str| -> Box<dyn Fn() + Send + Sync> {
      let (record, name) = (record.clone(), name.to_owned());
      Box::new(move || record(format!("on_select({name:?})")))
    };
    let shown = make(self.play_open.clone(), self.play_export.clone(), &told);

    let mut code = vec![
      "// open: Signal<Option<usize>>, the menu open now".to_owned(),
      "Menubar::new(open)".to_owned(),
    ];
    if disabled {
      code.push("    .disabled(true)".to_owned());
    }
    code.push(format!("    .menu({first:?}, ["));
    code.push("        MenuItem::new(\"New\").icon(icons::FILE).shortcut(\"Ctrl+N\").on_select(..).into(),".to_owned());
    code.push("        MenuItem::new(\"Open…\").shortcut(\"Ctrl+O\").on_select(..).into(),".to_owned());
    code.push("        MenuEntry::Separator,".to_owned());
    if submenu {
      code.push("        MenuItem::new(\"Export as\")".to_owned());
      code.push(
        "            .submenu(Menu::new(export, exporting).items([\"PNG\", \"SVG\", \"PDF\"].map(MenuItem::new)))"
          .to_owned(),
      );
      code.push("            .into(),".to_owned());
    }
    code.push("        MenuItem::new(\"Quit\").shortcut(\"Ctrl+Q\").on_select(..).into(),".to_owned());
    code.push("    ])".to_owned());
    code.push(
      "    .menu(\"Edit\", [MenuItem::new(\"Undo\").shortcut(\"Ctrl+Z\"), MenuItem::new(\"Redo\").disabled(true)])"
        .to_owned(),
    );
    code.push("    .menu(\"View\", [MenuItem::new(\"Show grid\").checked(true)])".to_owned());
    code.push("    .menu(\"Help\", [MenuItem::new(\"About\")])".to_owned());
    if help_disabled {
      code.push("    .title_disabled(\"Help\", true)".to_owned());
    }
    let tree = vec![
      format!("bar      {} · 36 tall · padding 4 · gap 2", self.playground.measured()),
      format!("├─ title  28 tall · padding 10 · {first:?} 13/500"),
      "├─ title  \"Edit\"".to_owned(),
      "├─ title  \"View\"".to_owned(),
      format!("└─ title  \"Help\"{}", if help_disabled { "  disabled" } else { "" }),
    ];
    let mut facts = vec![match self.play_open.get() {
      Some(index) => format!("open: {index}"),
      None => "closed".to_owned(),
    }];
    if disabled {
      facts.push("disabled".to_owned());
    }

    let (to_title, flags) = (
      self.title.clone(),
      [self.disabled.clone(), self.help_disabled.clone(), self.submenu.clone()],
    );
    self.playground.card(
      ctx,
      Spec {
        title: "Menubar",
        make: Arc::new(move || {
          let quiet = |_: &str| -> Box<dyn Fn() + Send + Sync> { Box::new(|| {}) };
          make(Signal::new(None), (ElementRef::new(), Signal::new(false)), &quiet).into()
        }),
        shown: shown.into(),
        fill: false,
        forceable: true,
        facts: facts.join(" · "),
        event: "on_select",
        code,
        tree,
        controls: vec![
          prop("menu", "&str", field(&self.title)),
          divider(),
          flag("disabled", &self.disabled),
          flag("Help disabled", &self.help_disabled),
          flag("submenu", &self.submenu),
        ],
        changed: [title != "File", disabled, help_disabled, !submenu]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset: Arc::new(move || {
          to_title.set("File".to_owned());
          for (index, flag) in flags.iter().enumerate() {
            // The submenu is on to start with; the rest are off.
            flag.set(index == 2);
          }
        }),
      },
    )
  }
}

impl Component for MenubarPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      title: ctx.signal("File".to_owned()),
      disabled: ctx.signal(false),
      help_disabled: ctx.signal(false),
      submenu: ctx.signal(true),
      play_open: ctx.signal(None),
      play_export: (ElementRef::new(), ctx.signal(false)),
      app_open: ctx.signal(None),
      app_export: (ElementRef::new(), ctx.signal(false)),
      chosen: ctx.signal(String::new()),
      grid: ctx.signal(true),
      zoom: ctx.signal(1),
      quiet_open: ctx.signal(None),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    // An app's menus: shortcuts, a submenu, a check and the radios of a zoom.
    let says = |name: &str| Self::says(&self.chosen, name);
    let (grid, show_grid) = (self.grid.clone(), self.grid.get());
    let zoom = self.zoom.get();
    let mut view: Vec<MenuEntry> = vec![
      MenuItem::new("Show grid")
        .checked(show_grid)
        .on_select(move || grid.set(!show_grid))
        .into(),
      MenuEntry::Separator,
      MenuEntry::Label(Arc::from("Zoom")),
    ];
    for (index, name) in ZOOMS.iter().enumerate() {
      let pick = self.zoom.clone();
      view.push(
        MenuItem::new(name)
          .radio(zoom == index)
          .on_select(move || pick.set(index))
          .into(),
      );
    }
    let app = Menubar::new(self.app_open.clone())
      .menu(
        "File",
        [
          MenuItem::new("New")
            .icon(icons::FILE)
            .shortcut("Ctrl+N")
            .on_select(says("New"))
            .into(),
          MenuItem::new("Open…")
            .shortcut("Ctrl+O")
            .on_select(says("Open…"))
            .into(),
          MenuEntry::Separator,
          MenuItem::new("Export as")
            .submenu(
              Menu::new(self.app_export.0.clone(), self.app_export.1.clone())
                .items(["PNG", "SVG", "PDF"].map(|format| MenuItem::new(format).on_select(says(format)))),
            )
            .into(),
          MenuEntry::Separator,
          MenuItem::new("Quit").shortcut("Ctrl+Q").on_select(says("Quit")).into(),
        ],
      )
      .menu(
        "Edit",
        [
          MenuItem::new("Undo")
            .icon(icons::UNDO)
            .shortcut("Ctrl+Z")
            .on_select(says("Undo")),
          MenuItem::new("Redo").shortcut("Ctrl+Y").disabled(true),
          MenuItem::new("Cut").shortcut("Ctrl+X").on_select(says("Cut")),
          MenuItem::new("Copy").shortcut("Ctrl+C").on_select(says("Copy")),
          MenuItem::new("Paste").shortcut("Ctrl+V").on_select(says("Paste")),
        ],
      )
      .menu("View", view)
      .menu(
        "Help",
        [
          MenuItem::new("Keyboard shortcuts")
            .icon(icons::KEYBOARD)
            .on_select(says("Keyboard shortcuts")),
          MenuItem::new("About").on_select(says("About")),
        ],
      );
    let app = Column::new().spacing(9.0).child(Row::new().child(app)).child(quiet(
      &match self.chosen.get() {
        chosen if chosen.is_empty() => "nothing chosen yet".to_owned(),
        chosen => format!("chosen: {chosen}"),
      },
      12.0,
    ));

    let disabled = Row::new().child(
      Menubar::new(self.quiet_open.clone())
        .menu("File", [MenuItem::new("New"), MenuItem::new("Open…")])
        .menu("Edit", [MenuItem::new("Undo")])
        .menu("Tools", [MenuItem::new("Run")])
        .title_disabled("Tools", true),
    );

    let rows = [
      (
        "Menubar::new(open)",
        "Signal<Option<usize>>",
        "",
        "Which menu is open, by its index: the app's. A press of a title sets it; choosing an item, a press \
         outside and Escape set it to None.",
      ),
      (
        "menu",
        "&str, impl IntoIterator<Item = impl Into<MenuEntry>>",
        "",
        "A menu called by its title, after the bar's others: MenuItems, with MenuEntry::Separator and \
         MenuEntry::Label between them.",
      ),
      (
        "title_disabled",
        "&str, bool",
        "false",
        "The menu of that title cannot be opened: its title is dimmed and the arrows pass it by.",
      ),
      ("disabled", "bool", "false", "No menu of the bar can be opened."),
      (
        "id",
        "impl Into<Arc<str>>",
        "",
        "The bar's id; its titles are {id}-0, {id}-1 and on, each menu's panel {id}-0-menu.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Navigation", "Menubar"],
        title: "Menubar",
        description: "The menus of an app in a row, File, Edit, View, as a window's title bar holds them on Windows \
                      and Linux: each opens from its title, and the keyboard and the pointer go from one to the next.",
        import: Some("use lurq::ui::{MenuEntry, MenuItem, Menubar};"),
        status: Some("Beta"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "The first menu's title and the bar's props are controls on the right. A press of a title opens its \
           menu; with one open, the pointer on another title opens that one.",
          playground,
        ),
        Section::new(
          "An app's menus",
          "Shortcuts, a submenu, a check and the radios of a zoom, in the menus a Menu has. Tab stops on the bar \
           once; Left and Right move between the titles, Down, Enter and Space open one at its first item and Up at \
           its last. In an open menu Left and Right go to the menu beside it, Escape closes it with the focus back \
           on its title. Alt alone puts the focus on the bar and underlines each title's first letter; Alt with \
           that letter opens its menu.",
          self.frame.demo(
            0,
            app,
            "Menubar::new(open).menu(\"File\", [..])",
            &[
              "// open: Signal<Option<usize>>; export: ElementRef, exporting: Signal<bool>",
              "Menubar::new(open)",
              "    .menu(\"File\", [",
              "        MenuItem::new(\"New\").icon(icons::FILE).shortcut(\"Ctrl+N\").on_select(..).into(),",
              "        MenuItem::new(\"Open…\").shortcut(\"Ctrl+O\").on_select(..).into(),",
              "        MenuEntry::Separator,",
              "        MenuItem::new(\"Export as\")",
              "            .submenu(Menu::new(export, exporting).items([\"PNG\", \"SVG\", \"PDF\"].map(MenuItem::new)))",
              "            .into(),",
              "        MenuEntry::Separator,",
              "        MenuItem::new(\"Quit\").shortcut(\"Ctrl+Q\").on_select(..).into(),",
              "    ])",
              "    .menu(\"Edit\", [",
              "        MenuItem::new(\"Undo\").icon(icons::UNDO).shortcut(\"Ctrl+Z\").on_select(..),",
              "        MenuItem::new(\"Redo\").shortcut(\"Ctrl+Y\").disabled(true),",
              "        MenuItem::new(\"Cut\").shortcut(\"Ctrl+X\").on_select(..),",
              "        MenuItem::new(\"Copy\").shortcut(\"Ctrl+C\").on_select(..),",
              "        MenuItem::new(\"Paste\").shortcut(\"Ctrl+V\").on_select(..),",
              "    ])",
              "    .menu(\"View\", [",
              "        MenuItem::new(\"Show grid\").checked(grid.get()).on_select(..).into(),",
              "        MenuEntry::Separator,",
              "        MenuEntry::Label(\"Zoom\".into()),",
              "        MenuItem::new(\"50%\").radio(zoom.get() == 0).on_select(..).into(),",
              "        MenuItem::new(\"100%\").radio(zoom.get() == 1).on_select(..).into(),",
              "        MenuItem::new(\"200%\").radio(zoom.get() == 2).on_select(..).into(),",
              "    ])",
              "    .menu(\"Help\", [",
              "        MenuItem::new(\"Keyboard shortcuts\").icon(icons::KEYBOARD).on_select(..),",
              "        MenuItem::new(\"About\").on_select(..),",
              "    ])",
            ],
          ),
        ),
        Section::new(
          "A disabled title",
          "A menu that cannot be opened now: its title is dimmed, a press and the pointer open nothing, and the \
           arrows and Alt pass it by.",
          self.frame.demo(
            1,
            disabled,
            ".title_disabled(\"Tools\", true)",
            &[
              "// open: Signal<Option<usize>>",
              "Menubar::new(open)",
              "    .menu(\"File\", [MenuItem::new(\"New\"), MenuItem::new(\"Open…\")])",
              "    .menu(\"Edit\", [MenuItem::new(\"Undo\")])",
              "    .menu(\"Tools\", [MenuItem::new(\"Run\")])",
              "    .title_disabled(\"Tools\", true)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a menubar is given. Its menus' items are MenuItems, with all a Menu's items have.",
          props_table(&rows, &["Menubar::new(open)"], self.frame.room()),
        ),
      ],
    )
  }
}
