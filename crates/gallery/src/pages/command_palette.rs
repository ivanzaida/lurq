//! The page of the command palette: its playground, the palette over the window, groups with details and keywords,
//! and a palette that fetches what it shows.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  node::Element,
  ui::{Button, ButtonVariant, CommandItem, CommandPalette, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{props_table, quiet},
  playground::{Playground, Spec, divider, field, flag, prop},
};

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .spacing(9.0)
    .child(Row::new().child(shown))
    .child(quiet(caption, 12.0))
}

/// The board's suggestions, each telling `told` when it is run.
fn suggestions(told: &dyn Fn(&str) -> Box<dyn Fn() + Send + Sync>, shortcuts: bool) -> Vec<CommandItem> {
  [
    ("Toggle dark mode", icons::MOON, "Ctrl+D", "theme appearance light"),
    ("Open DevTools", icons::TERMINAL, "F12", "inspect debug console"),
    ("New window", icons::PLUS, "Ctrl+N", "open another"),
  ]
  .into_iter()
  .map(|(label, icon, keys, words)| {
    let mut command = CommandItem::new(label)
      .icon(icon)
      .keywords(words.split(' '))
      .on_select(told(label));
    if shortcuts {
      command = command.shortcut(keys);
    }
    command
  })
  .collect()
}

/// The files of a project, a few: what a palette of a code editor finds.
const FILES: [(&str, &str); 6] = [
  ("main.rs", "src"),
  ("command_palette.rs", "crates/lurq/src/ui"),
  ("menu.rs", "crates/lurq/src/ui"),
  ("shell.rs", "crates/gallery/src"),
  ("Cargo.toml", "crates/lurq"),
  ("README.md", ""),
];

pub struct CommandPalettePage {
  frame: Frame,
  playground: Playground,
  placeholder: Signal<String>,
  empty: Signal<String>,
  footer: Signal<bool>,
  loading: Signal<bool>,
  details: Signal<bool>,
  disabled: Signal<bool>,
  inline: Signal<bool>,
  play_open: Signal<bool>,
  // Over the window.
  window_open: Signal<bool>,
  window_ran: Signal<String>,
  // Groups, details and keywords, drawn in place.
  groups_open: Signal<bool>,
  groups_ran: Signal<String>,
  // A palette that fetches.
  fetch_open: Signal<bool>,
  fetch_query: Signal<String>,
}

impl CommandPalettePage {
  /// The playground: the board's palette as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (placeholder, empty) = (self.placeholder.get(), self.empty.get());
    let (footer, loading, details, disabled, inline) = (
      self.footer.get(),
      self.loading.get(),
      self.details.get(),
      self.disabled.get(),
      self.inline.get(),
    );
    let record = self.playground.recorder();
    let told = move |name: &str| -> Box<dyn Fn() + Send + Sync> {
      let (record, name) = (record.clone(), name.to_owned());
      Box::new(move || record(format!("on_select({name:?})")))
    };
    let palette = {
      let (placeholder, empty) = (placeholder.clone(), empty.clone());
      move |open: Signal<bool>, told: &dyn Fn(&str) -> Box<dyn Fn() + Send + Sync>| {
        let mut commands = suggestions(told, true);
        if details {
          commands[1] = commands[1].clone().detail("Developer");
          commands[2] = commands[2].clone().detail("Window");
        }
        if disabled {
          commands[2] = commands[2].clone().disabled(true);
        }
        CommandPalette::new(open)
          .placeholder(&placeholder)
          .empty(&empty)
          .footer(footer)
          .loading(loading)
          .inline(inline)
          .group("Suggestions", commands)
      }
    };
    let shown: Element = if inline {
      palette(self.play_open.clone(), &told).into()
    } else {
      let open = self.play_open.clone();
      Column::new()
        .child(
          Button::new("Open the palette")
            .variant(ButtonVariant::Outline)
            .icon(icons::SEARCH)
            .on_click(move |_| open.set(true)),
        )
        .child(palette(self.play_open.clone(), &told))
        .into()
    };
    let make = {
      let palette = palette.clone();
      move || -> Element {
        let quiet = |_: &str| -> Box<dyn Fn() + Send + Sync> { Box::new(|| {}) };
        let mut palette = palette(Signal::new(true), &quiet);
        palette = palette.inline(true);
        palette.into()
      }
    };

    let mut code = vec!["// open: Signal<bool>, set by a shortcut or a button".to_owned()];
    code.push("CommandPalette::new(open)".to_owned());
    if placeholder != "Type a command or search" {
      code.push(format!("    .placeholder({placeholder:?})"));
    }
    if empty != "No results" {
      code.push(format!("    .empty({empty:?})"));
    }
    for (on, call) in [
      (footer, ".footer(true)"),
      (loading, ".loading(true)"),
      (inline, ".inline(true)"),
    ] {
      if on {
        code.push(format!("    {call}"));
      }
    }
    code.push("    .group(\"Suggestions\", [".to_owned());
    code.push("        CommandItem::new(\"Toggle dark mode\").icon(icons::MOON).shortcut(\"Ctrl+D\"),".to_owned());
    code.push(format!(
      "        CommandItem::new(\"Open DevTools\").icon(icons::TERMINAL).shortcut(\"F12\"){},",
      if details { ".detail(\"Developer\")" } else { "" }
    ));
    code.push(format!(
      "        CommandItem::new(\"New window\").icon(icons::PLUS).shortcut(\"Ctrl+N\"){}{},",
      if details { ".detail(\"Window\")" } else { "" },
      if disabled { ".disabled(true)" } else { "" }
    ));
    code.push("    ])".to_owned());

    let mut tree = vec![
      format!("panel        {}", self.playground.measured()),
      "├─ search     44 tall  icon 16 · input · key \"Esc\"".to_owned(),
      "├─ list       padded 4, scrolls past 320".to_owned(),
      "│  ├─ group    \"Suggestions\"  12/500".to_owned(),
      format!(
        "│  └─ rows     3 × 32 tall  icon 16 · label{} · shortcut",
        if details { " · detail" } else { "" }
      ),
    ];
    if loading {
      tree.push("│  └─ status   spinner 16 · \"Loading…\"".to_owned());
    }
    if footer {
      tree.push("└─ footer     36 tall  ↑ ↓ · ↵ · Esc".to_owned());
    }

    let (to_placeholder, to_empty) = (self.placeholder.clone(), self.empty.clone());
    let flags = [
      self.footer.clone(),
      self.loading.clone(),
      self.details.clone(),
      self.disabled.clone(),
      self.inline.clone(),
    ];
    let mut facts = vec!["420 wide".to_owned()];
    if inline {
      facts.push("inline".to_owned());
    }
    self.playground.card(
      ctx,
      Spec {
        title: "Command Palette",
        make: Arc::new(make),
        shown,
        fill: false,
        forceable: false,
        facts: facts.join(" · "),
        event: "on_select",
        code,
        tree,
        controls: vec![
          prop("placeholder", "&str", field(&self.placeholder)),
          prop("empty", "&str", field(&self.empty)),
          divider(),
          flag("footer", &self.footer),
          flag("loading", &self.loading),
          flag("detail", &self.details),
          flag("disabled", &self.disabled),
          flag("inline", &self.inline),
        ],
        changed: [
          placeholder != "Type a command or search",
          empty != "No results",
          !footer,
          loading,
          details,
          disabled,
          !inline,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          to_placeholder.set("Type a command or search".to_owned());
          to_empty.set("No results".to_owned());
          for (index, flag) in flags.iter().enumerate() {
            // The footer and inline are on to start with; the rest are off.
            flag.set(index == 0 || index == 4);
          }
        }),
      },
    )
  }

  /// What running a command on this page does: the signal says which it was.
  fn says(ran: &Signal<String>) -> impl Fn(&str) -> Box<dyn Fn() + Send + Sync> + use<> {
    let ran = ran.clone();
    move |name: &str| -> Box<dyn Fn() + Send + Sync> {
      let (ran, name) = (ran.clone(), name.to_owned());
      Box::new(move || ran.set(name.clone()))
    }
  }

  fn ran(signal: &Signal<String>) -> String {
    match signal.get() {
      name if name.is_empty() => "nothing run yet".to_owned(),
      name => format!("ran: {name}"),
    }
  }
}

impl Component for CommandPalettePage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      placeholder: ctx.signal("Type a command or search".to_owned()),
      empty: ctx.signal("No results".to_owned()),
      footer: ctx.signal(true),
      loading: ctx.signal(false),
      details: ctx.signal(false),
      disabled: ctx.signal(false),
      inline: ctx.signal(true),
      play_open: ctx.signal(true),
      window_open: ctx.signal(false),
      window_ran: ctx.signal(String::new()),
      groups_open: ctx.signal(true),
      groups_ran: ctx.signal(String::new()),
      fetch_open: ctx.signal(true),
      fetch_query: ctx.signal(String::new()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    // Over the window, from a button.
    let open = self.window_open.clone();
    let says = Self::says(&self.window_ran);
    let window = item(
      Column::new()
        .child(
          Button::new("Search commands")
            .variant(ButtonVariant::Outline)
            .icon(icons::SEARCH)
            .on_click(move |_| open.set(true)),
        )
        .child(
          CommandPalette::new(self.window_open.clone())
            .group("Suggestions", suggestions(&says, true))
            .footer(true),
        ),
      &Self::ran(&self.window_ran),
    );

    // Groups, a detail on each file, and keywords that find a command by another word.
    let says = Self::says(&self.groups_ran);
    let files: Vec<CommandItem> = FILES
      .iter()
      .map(|(name, folder)| {
        let file = CommandItem::new(name).icon(icons::FILE).on_select(says(name));
        if folder.is_empty() { file } else { file.detail(folder) }
      })
      .collect();
    let groups = item(
      CommandPalette::new(self.groups_open.clone())
        .inline(true)
        .placeholder("Search files and commands")
        .group("Files", files)
        .group(
          "Commands",
          [
            CommandItem::new("Go to line")
              .icon(icons::ARROW_RIGHT)
              .shortcut("Ctrl+G")
              .keywords(["jump", "number"])
              .on_select(says("Go to line")),
            CommandItem::new("Format document")
              .icon(icons::FILE_CODE)
              .shortcut("Shift+Alt+F")
              .keywords(["prettify", "rustfmt"])
              .on_select(says("Format document")),
            CommandItem::new("Publish crate").icon(icons::UPLOAD).disabled(true),
          ],
        ),
      &Self::ran(&self.groups_ran),
    );

    // A palette whose app finds its commands: it says what is typed, and gives back what it found.
    let query = self.fetch_query.get();
    let wanted = query.trim().to_lowercase();
    let found: Vec<CommandItem> = if wanted.is_empty() {
      Vec::new()
    } else {
      ["Lisbon", "London", "Los Angeles", "Lyon", "Oslo", "Lima"]
        .into_iter()
        .filter(|city| city.to_lowercase().starts_with(&wanted))
        .map(|city| CommandItem::new(city).icon(icons::PIN))
        .collect()
    };
    let fetch = item(
      CommandPalette::new(self.fetch_open.clone())
        .inline(true)
        .placeholder("Find a city")
        .query(self.fetch_query.clone())
        .filter(|_, _| true)
        .empty(if wanted.is_empty() {
          "Type a city's first letters"
        } else {
          "No city starts so"
        })
        .group("Cities", found),
      "the page filters the cities itself, as an app that fetches does",
    );

    let rows = [
      (
        "CommandPalette::new(open)",
        "Signal<bool>",
        "",
        "Whether it is open: the app's. Running a command, Escape on an empty input and a press on the veil set \
         it to false.",
      ),
      (
        "placeholder",
        "&str",
        "Type a command or search",
        "What the input says while nothing is typed.",
      ),
      (
        "item",
        "CommandItem",
        "",
        "A command with no group. items takes several.",
      ),
      (
        "group",
        "&str, [CommandItem]",
        "",
        "Commands under a label; a group with none shown is not shown.",
      ),
      (
        "filter",
        "Fn(&CommandItem, &str) -> bool",
        "every word",
        "Which commands are shown for what is typed. By default those whose label, detail or keywords hold \
         every word, in the app's order.",
      ),
      (
        "on_input",
        "Fn(&str)",
        "",
        "Called with what the input will hold: an app that fetches asks here.",
      ),
      ("query", "Signal<String>", "", "The app's own signal for what is typed."),
      (
        "loading",
        "bool",
        "false",
        "The list ends with a spinner while more are on their way.",
      ),
      (
        "empty",
        "&str",
        "No results",
        "What the list says when nothing is found.",
      ),
      (
        "footer",
        "bool",
        "false",
        "A row of the keys that work in it, at the bottom.",
      ),
      ("inline", "bool", "false", "The panel drawn in place, without the veil."),
      ("CommandItem::new(label)", "&str", "", "A command, called by its label."),
      ("icon", "impl Into<Name>", "", "An icon of 16 before the label, muted."),
      ("detail", "&str", "", "A second text after the label, muted."),
      (
        "shortcut",
        "&str",
        "",
        "The keys that do the same, at the end. The app binds them.",
      ),
      ("keywords", "[&str]", "", "Words that find it and are not shown."),
      (
        "disabled",
        "bool",
        "false",
        "Dimmed; the keys and the pointer pass it by.",
      ),
      (
        "on_select",
        "impl Fn()",
        "",
        "Called when it is run, before the palette closes.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Surfaces & overlays", "Command Palette"],
        title: "Command Palette",
        description: "A search over the window that finds a command or a place by typing: every command of an \
                      editor, the pages of an app, the files of a project.",
        import: Some("use lurq::ui::{CommandItem, CommandPalette};"),
        status: Some("Beta"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "The props are controls on the right. Inline, the panel stands on the stage; turn inline off and the button \
           opens it over the window. Type to filter, Down and Up to move, Enter to run, Escape to clear and close.",
          playground,
        ),
        Section::new(
          "Over the window",
          "The page holds whether it is open, in a signal the button sets. It stands near the top of the window over \
           the veil, the focus in its input, and gives the focus back as it closes. The gallery's own search, Ctrl+K, \
           is one too.",
          self.frame.demo(
            0,
            window,
            "CommandPalette::new(open).group(\"Suggestions\", commands)",
            &[
              "// open: Signal<bool>",
              "Button::new(\"Search commands\")",
              "    .variant(ButtonVariant::Outline)",
              "    .icon(icons::SEARCH)",
              "    .on_click(move |_| open.set(true))",
              "",
              "CommandPalette::new(open)",
              "    .group(\"Suggestions\", [",
              "        CommandItem::new(\"Toggle dark mode\").icon(icons::MOON).shortcut(\"Ctrl+D\").on_select(..),",
              "        CommandItem::new(\"Open DevTools\").icon(icons::TERMINAL).shortcut(\"F12\").on_select(..),",
              "        CommandItem::new(\"New window\").icon(icons::PLUS).shortcut(\"Ctrl+N\").on_select(..),",
              "    ])",
              "    .footer(true)",
            ],
          ),
        ),
        Section::new(
          "Groups, details and keywords",
          "Each group has its label over it, and the highlight runs across them. A file says its folder after its \
           name; \"rustfmt\" finds Format document by a keyword it does not show. A disabled command is dimmed and \
           the keys pass it by.",
          self.frame.demo(
            1,
            groups,
            ".group(\"Files\", files).group(\"Commands\", commands)",
            &[
              "CommandPalette::new(open)",
              "    .inline(true)",
              "    .placeholder(\"Search files and commands\")",
              "    .group(\"Files\", [",
              "        CommandItem::new(\"main.rs\").icon(icons::FILE).detail(\"src\"),",
              "        CommandItem::new(\"menu.rs\").icon(icons::FILE).detail(\"crates/lurq/src/ui\"),",
              "    ])",
              "    .group(\"Commands\", [",
              "        CommandItem::new(\"Go to line\").icon(icons::ARROW_RIGHT).shortcut(\"Ctrl+G\")",
              "            .keywords([\"jump\", \"number\"]),",
              "        CommandItem::new(\"Format document\").icon(icons::FILE_CODE).shortcut(\"Shift+Alt+F\")",
              "            .keywords([\"prettify\", \"rustfmt\"]),",
              "        CommandItem::new(\"Publish crate\").icon(icons::UPLOAD).disabled(true),",
              "    ])",
            ],
          ),
        ),
        Section::new(
          "An app that finds its own",
          "The app holds what is typed and filters, or fetches, the commands itself: the palette shows every one it \
           is given. What the list says while nothing is found is the app's too.",
          self.frame.demo(
            2,
            fetch,
            ".query(query).filter(|_, _| true)",
            &[
              "// query: Signal<String>; found: the commands the app found for it",
              "CommandPalette::new(open)",
              "    .inline(true)",
              "    .placeholder(\"Find a city\")",
              "    .query(query)",
              "    .filter(|_, _| true)",
              "    .empty(\"No city starts so\")",
              "    .group(\"Cities\", found)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a palette and its commands are given.",
          props_table(
            &rows,
            &["CommandPalette::new(open)", "CommandItem::new(label)"],
            self.frame.room(),
          ),
        ),
      ],
    )
  }
}
