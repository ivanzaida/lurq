//! The page of the window's chrome: its playground, the bar of each platform, an app's menus in the bar, a window
//! that is not in front and one that is maximized.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  layout::{Alignment, layout_kind::Justify},
  node::Element,
  ui::{MenuEntry, MenuItem, Menubar, MenubarMenu, WindowChrome, WindowChromeState, WindowControls, WindowPlatform},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

const PLATFORMS: [(&str, WindowPlatform); 4] = [
  ("Windows", WindowPlatform::Windows),
  ("macOS", WindowPlatform::Macos),
  ("GNOME", WindowPlatform::Gnome),
  ("KDE", WindowPlatform::Kde),
];

/// What a window shows under its bar on this page: a line that says so, in the middle.
fn content(height: f32) -> Element {
  Column::new()
    .width(FILL)
    .height(height)
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .child(quiet("Window content", 11.0))
    .into()
}

/// A window of the board's width, 560, as the board draws it, or as wide as the room where that is less.
fn window(chrome: WindowChrome) -> Element {
  Column::new().width(FILL).max_width(560.0).child(chrome).into()
}

/// An app's menus: File, Edit and View.
fn menus(open: &Signal<Option<usize>>) -> Menubar {
  Menubar::new(open.clone())
    .label("Application")
    .menu(MenubarMenu::new(
      "File",
      [
        MenuItem::new("New").shortcut("Ctrl+N").into(),
        MenuItem::new("Open…").shortcut("Ctrl+O").into(),
        MenuEntry::Separator,
        MenuItem::new("Quit").shortcut("Ctrl+Q").into(),
      ],
    ))
    .menu(MenubarMenu::new("Edit", [MenuItem::new("Undo"), MenuItem::new("Redo")]))
    .menu(MenubarMenu::new(
      "View",
      [MenuItem::new("Zoom in"), MenuItem::new("Zoom out")],
    ))
}

pub struct WindowChromePage {
  frame: Frame,
  playground: Playground,
  platform: Signal<WindowPlatform>,
  title: Signal<String>,
  icon: Signal<bool>,
  menubar: Signal<bool>,
  inactive: Signal<bool>,
  maximized: Signal<bool>,
  minimizable: Signal<bool>,
  maximizable: Signal<bool>,
  /// The menu open in the playground's bar, and in the section's.
  play_menus: Signal<Option<usize>>,
  app_menus: Signal<Option<usize>>,
}

impl WindowChromePage {
  /// The playground: the chrome as the panel's props make it, in its still form.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let platform = self.platform.get();
    let title = self.title.get();
    let (icon, menubar) = (self.icon.get(), self.menubar.get());
    let (inactive, maximized) = (self.inactive.get(), self.maximized.get());
    let (minimizable, maximizable) = (self.minimizable.get(), self.maximizable.get());
    let record = self.playground.recorder();
    let forced = self.playground.forced();
    let menus_open = self.play_menus.clone();
    let make = Arc::new({
      let title = title.clone();
      let menus_open = menus_open.clone();
      move || -> WindowChrome {
        let mut chrome = WindowChrome::new()
          .platform(platform)
          .title(&title)
          .state(WindowChromeState::Inactive, inactive)
          .state(WindowChromeState::Maximized, maximized)
          .controls(WindowControls::new().minimizable(minimizable).maximizable(maximizable))
          .content(content(120.0));
        if !icon {
          chrome = chrome.icon(None);
        }
        if menubar {
          chrome = chrome.leading(menus(&menus_open));
        }
        chrome
      }
    });
    let shown = {
      let make = make.clone();
      make().force(forced).controls(
        WindowControls::new()
          .minimizable(minimizable)
          .maximizable(maximizable)
          .on_close(move || record("on_close()".to_owned())),
      )
    };

    let mut code = vec!["WindowChrome::new()".to_owned()];
    if platform != WindowPlatform::current() {
      code.push(format!("    .platform(WindowPlatform::{platform:?})"));
    }
    code.push(format!("    .title({title:?})"));
    if !icon {
      code.push("    .icon(None)".to_owned());
    }
    if menubar {
      code.push("    .leading(Menubar::new(open).menu(MenubarMenu::new(\"File\", [..])) /* Edit, View */)".to_owned());
    }
    if !minimizable || !maximizable {
      let mut controls = "    .controls(WindowControls::new()".to_owned();
      if !minimizable {
        controls.push_str(".minimizable(false)");
      }
      if !maximizable {
        controls.push_str(".maximizable(false)");
      }
      controls.push(')');
      code.push(controls);
    }
    code.push("    .content(content)".to_owned());
    code.push("    .mount(ctx)".to_owned());

    let bar = match platform {
      WindowPlatform::Windows => "36 high · icon + title at the start · 3 controls 46 × 36 at the end",
      WindowPlatform::Macos => "36 high · lights 12, 8 apart, at the start · title in the middle",
      WindowPlatform::Gnome => "44 high · title in the middle · round controls 24, 8 apart, at the end",
      WindowPlatform::Kde => "32 high · icon at the start · title in the middle · round controls 22, 6 apart",
    };
    let tree = vec![
      format!("root      {}", self.playground.measured()),
      format!("├─ bar    {bar}"),
      "└─ content".to_owned(),
    ];

    let set_platform = self.platform.clone();
    let reset = {
      let (platform, title) = (self.platform.clone(), self.title.clone());
      let on = [self.icon.clone(), self.minimizable.clone(), self.maximizable.clone()];
      let off = [self.menubar.clone(), self.inactive.clone(), self.maximized.clone()];
      Arc::new(move || {
        platform.set(WindowPlatform::Windows);
        title.set("lurq demo".to_owned());
        for flag in &on {
          flag.set(true);
        }
        for flag in &off {
          flag.set(false);
        }
      })
    };
    let detached = {
      let make = make.clone();
      Arc::new(move || window(make()))
    };
    self.playground.card(
      ctx,
      Spec {
        title: "WindowChrome",
        make: detached,
        shown: Shown::of(window(shown)),
        fill: true,
        forceable: true,
        facts: format!("{platform:?}"),
        event: "on_close",
        code,
        tree,
        controls: vec![
          prop(
            "platform",
            "WindowPlatform",
            self
              .playground
              .select(0, &PLATFORMS, &platform, move |platform| set_platform.set(platform)),
          ),
          prop("title", "&str", field(&self.title)),
          divider(),
          flag("icon", &self.icon),
          flag("menubar", &self.menubar),
          flag("inactive", &self.inactive),
          flag("maximized", &self.maximized),
          flag("minimizable", &self.minimizable),
          flag("maximizable", &self.maximizable),
        ],
        changed: [
          platform != WindowPlatform::Windows,
          title != "lurq demo",
          !icon,
          menubar,
          inactive,
          maximized,
          !minimizable,
          !maximizable,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset,
      },
    )
  }

  /// The four platforms' bars, one over the other.
  fn platforms() -> Element {
    let mut column = Column::new().spacing(16.0);
    for (name, platform) in PLATFORMS {
      column = column.child(
        Column::new().spacing(8.0).child(quiet(name, 12.0)).child(window(
          WindowChrome::new()
            .platform(platform)
            .title("lurq demo")
            .content(content(80.0)),
        )),
      );
    }
    column.into()
  }

  /// The states: a window that is not in front, and one that is maximized, on each platform.
  fn states() -> Element {
    let mut column = Column::new().spacing(16.0);
    for (name, platform) in PLATFORMS {
      column = column.child(
        Row::new()
          .spacing(16.0)
          .wrap()
          .child(
            Column::new()
              .spacing(8.0)
              .child(quiet(&format!("{name} · not in front"), 12.0))
              .child(window(
                WindowChrome::new()
                  .platform(platform)
                  .title("lurq demo")
                  .state(WindowChromeState::Inactive, true),
              )),
          )
          .child(
            Column::new()
              .spacing(8.0)
              .child(quiet(&format!("{name} · maximized"), 12.0))
              .child(window(
                WindowChrome::new()
                  .platform(platform)
                  .title("lurq demo")
                  .state(WindowChromeState::Maximized, true),
              )),
          ),
      );
    }
    column.into()
  }
}

impl Component for WindowChromePage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 1),
      platform: ctx.signal(WindowPlatform::Windows),
      title: ctx.signal("lurq demo".to_owned()),
      icon: ctx.signal(true),
      menubar: ctx.signal(false),
      inactive: ctx.signal(false),
      maximized: ctx.signal(false),
      minimizable: ctx.signal(true),
      maximizable: ctx.signal(true),
      play_menus: ctx.signal(None),
      app_menus: ctx.signal(None),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let rows = [
      (
        "WindowChrome::new()",
        "",
        "",
        "The chrome of the platform the app runs on. mount(ctx) makes it the window's frame; converted into an \
         element it is the still form, a picture of the window.",
      ),
      (
        "platform",
        "WindowPlatform",
        "the app's",
        "Windows, Macos, Gnome or Kde: the bar's look and order.",
      ),
      (
        "title",
        "&str",
        "",
        "The window's title, in the bar and given to the window. It ends with an ellipsis where it has no room.",
      ),
      (
        "icon",
        "impl Into<IconChoice>",
        "box",
        "The app's icon where the platform shows one (Windows, KDE); None for none.",
      ),
      ("label", "&str", "the title", "The bar's name to what reads the tree."),
      (
        "leading",
        "impl Into<Element>",
        "",
        "The app's own at the start of the bar: a menubar. A press there is its own, not a drag.",
      ),
      (
        "trailing",
        "impl Into<Element>",
        "",
        "The app's own at the end of the bar, before the controls: a search, an account.",
      ),
      (
        "tabs",
        "impl Into<Element>",
        "",
        "WindowTabs in the bar: higher, with no title and no line under it, the tabs on its bottom edge.",
      ),
      (
        "controls",
        "WindowControls",
        "all three",
        "Which controls there are, their names (minimize_label, maximize_label, restore_label, close_label) and \
         on_close.",
      ),
      (
        "content",
        "impl Into<Element>",
        "",
        "What the window shows under the bar.",
      ),
      (
        "overlay",
        "impl Into<Element>",
        "",
        "A layer over the whole window, under the bar's inset.",
      ),
      (
        "mode, resize_handles, border",
        "",
        "the platform's",
        "The engine's chrome: when it is drawn, the edges that resize, the line around the window.",
      ),
      (
        "state",
        "WindowChromeState",
        "",
        "Inactive and Maximized for the still form; a mounted chrome reads them from the window.",
      ),
      (
        "force",
        "Option<Interaction>",
        "",
        "The close control as under the pointer or held, the lights' glyphs on macOS.",
      ),
      ("style, rules, id, key", "", "", "As every component has them."),
    ];
    let playground = self.play(ctx);
    let app = window(
      WindowChrome::new()
        .platform(WindowPlatform::Windows)
        .title("Untitled — Notes")
        .leading(menus(&self.app_menus))
        .content(content(120.0)),
    );
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Desktop", "Window Chrome"],
        title: "Window Chrome",
        description: "The title bar of a window the app draws itself, as Windows, macOS, GNOME and KDE draw theirs: \
                      the icon, the title, the app's own controls and the window's.",
        import: Some("use lurq::ui::{WindowChrome, WindowControls, WindowPlatform};"),
        status: Some("Beta"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop is a control on the right. On the stage the chrome is its still form, a picture of a window: \
           in an app, mount(ctx) makes it the window's own, which the bar drags, a double press maximizes, a right \
           press opens the system's menu of (on Windows) and the edges resize.",
          playground,
        ),
        Section::new(
          "Four platforms",
          "One component, four renderers: the look and the order of each desktop. An app draws the one it runs on, \
           or names another.",
          self.frame.demo(
            0,
            Self::platforms(),
            "WindowChrome::new().platform(platform)",
            &[
              "WindowChrome::new().platform(WindowPlatform::Windows).title(\"lurq demo\").content(content)",
              "WindowChrome::new().platform(WindowPlatform::Macos).title(\"lurq demo\").content(content)",
              "WindowChrome::new().platform(WindowPlatform::Gnome).title(\"lurq demo\").content(content)",
              "WindowChrome::new().platform(WindowPlatform::Kde).title(\"lurq demo\").content(content)",
            ],
          ),
        ),
        Section::new(
          "An app's menus in the bar",
          "A menubar in the bar's leading room, after the icon and before the title, as an editor has its menus: \
           its titles open their menus, and a press on them is theirs, not a drag of the window.",
          self.frame.demo(
            1,
            app,
            "WindowChrome::new().leading(Menubar::new(open)..)",
            &[
              "WindowChrome::new()",
              "    .title(\"Untitled — Notes\")",
              "    .leading(",
              "        Menubar::new(open)",
              "            .menu(MenubarMenu::new(\"File\", [..]))",
              "            .menu(MenubarMenu::new(\"Edit\", [..]))",
              "            .menu(MenubarMenu::new(\"View\", [..])),",
              "    )",
              "    .content(document)",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Not in front, and maximized",
          "A window that is not the one in front has its title and its glyphs muted, and grey lights on macOS; a \
           maximized one's middle control restores it. A mounted chrome reads both from the window.",
          self.frame.demo(
            2,
            Self::states(),
            ".state(WindowChromeState::Inactive, true)",
            &[
              "WindowChrome::new().title(\"lurq demo\").state(WindowChromeState::Inactive, true)",
              "WindowChrome::new().title(\"lurq demo\").state(WindowChromeState::Maximized, true)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a chrome is given. WindowControls says which of the window's controls there are and what they are \
           called.",
          props_table(&rows, &["WindowChrome::new()"], self.frame.room()),
        ),
      ],
    )
  }
}
