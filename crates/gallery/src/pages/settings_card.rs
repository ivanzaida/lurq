//! The page of the settings card: its playground, the board's card of one setting and the one that opens to more,
//! the cards of rows of a preferences window, rows that lead somewhere, and its states.

use std::sync::Arc;

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{Density, PaletteColor},
  },
  components::{Column, Row},
  core::Signal,
  layout::{Alignment, text_style::FontWeight},
  node::Element,
  ui::{Button, ButtonSize, ButtonVariant, Interaction, ListItem, Select, SelectSize, SettingsCard, Switch, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet, text},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

/// The densities the playground offers, and how the code names them.
const DENSITIES: [(&str, Option<Density>, &str); 5] = [
  ("theme's", None, ""),
  ("compact", Some(Density::Compact), "Density::Compact"),
  ("default", Some(Density::Default), "Density::Default"),
  ("comfortable", Some(Density::Comfortable), "Density::Comfortable"),
  ("touch", Some(Density::Touch), "Density::Touch"),
];

/// The appearances the board's card chooses between.
fn themes() -> [(String, &'static str); 3] {
  [
    ("system".to_owned(), "System"),
    ("light".to_owned(), "Light"),
    ("dark".to_owned(), "Dark"),
  ]
}

/// The channels updates come by.
fn channels() -> [(String, &'static str); 2] {
  [("stable".to_owned(), "Stable"), ("beta".to_owned(), "Beta")]
}

/// A select at the end of a row, 120 wide: a select keeps a width of its own, which would leave the text beside
/// it none.
fn choice(value: &Signal<String>, options: impl IntoIterator<Item = (String, &'static str)>, name: &str) -> Row {
  Row::new().width(120.0).flex_shrink(0.0).child(
    Select::new(value.clone(), options)
      .size(SelectSize::Sm)
      .full_width(true)
      .label(name),
  )
}

/// A switch at the end of a row, called by the row's title and bound to `on`.
fn toggle(title: &str, on: &Signal<bool>) -> Switch {
  Switch::new(title).label_hidden(true).bind(on.clone())
}

/// The heading of a group of settings, as the board's preferences window writes it over a card.
fn heading(content: &str) -> Element {
  text(content, 12.0, FontWeight::SemiBold)
    .color(PaletteColor::MutedForeground)
    .into()
}

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .width(FILL)
    .spacing(9.0)
    .child(Column::new().width(FILL).child(shown))
    .child(quiet(caption, 12.0))
}

/// What is on a stage, one under the other.
fn items() -> Column {
  Column::new().width(FILL).spacing(24.0).align_items(Alignment::Start)
}

/// A column no wider than `width`, and as wide as its room where that is less: what gives a card its width.
fn held(width: f32, shown: impl Into<Element>) -> Column {
  Column::new().width(width).max_width(FILL).child(shown)
}

pub struct SettingsCardPage {
  frame: Frame,
  playground: Playground,
  title: Signal<String>,
  description: Signal<bool>,
  icon: Signal<bool>,
  rows: Signal<bool>,
  open: Signal<bool>,
  disabled: Signal<bool>,
  fill: Signal<bool>,
  density: Signal<usize>,
  /// What the settings of the page are set to.
  theme: Signal<String>,
  contrast: Signal<bool>,
  motion: Signal<bool>,
  login: Signal<bool>,
  channel: Signal<String>,
  install: Signal<bool>,
  /// What a row that leads somewhere was pressed for.
  went: Signal<String>,
}

/// The rows a card of the theme opens to, their switches bound to `contrast` and `motion`.
fn more_of(contrast: &Signal<bool>, motion: &Signal<bool>) -> [ListItem; 2] {
  [
    ListItem::new("High contrast")
      .description("Stronger edges and text.")
      .trailing(toggle("High contrast", contrast)),
    ListItem::new("Reduce motion")
      .description("Fewer animations.")
      .trailing(toggle("Reduce motion", motion)),
  ]
}

impl SettingsCardPage {
  /// The rows a card of the theme opens to.
  fn more(&self) -> [ListItem; 2] {
    more_of(&self.contrast, &self.motion)
  }

  /// The playground: the card of one setting as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let title = self.title.get();
    let (description, icon, rows) = (self.description.get(), self.icon.get(), self.rows.get());
    let (open, disabled, fill) = (self.open.get(), self.disabled.get(), self.fill.get());
    let (density_name, density, density_code) = DENSITIES[self.density.get().min(DENSITIES.len() - 1)];
    let make = {
      let (title, theme, bound) = (title.clone(), self.theme.clone(), self.open.clone());
      let (contrast, motion) = (self.contrast.clone(), self.motion.clone());
      move || {
        let mut row = ListItem::new(&title).trailing(choice(&theme, themes(), &title));
        if icon {
          row = row.icon(icons::PALETTE);
        }
        if description {
          row = row.description("Choose how lurq looks.");
        }
        let mut card = SettingsCard::new(row).disabled(disabled);
        if rows {
          card = card.items(more_of(&contrast, &motion)).bind(bound.clone());
        }
        if let Some(density) = density {
          card = card.density(density);
        }
        if !fill {
          card = card.width(360.0);
        }
        card
      }
    };
    let record = self.playground.recorder();
    let shown = make()
      .force(self.playground.forced())
      .on_open_change(move |open| record(format!("on_open_change({open})")));

    let mut code = vec!["SettingsCard::new(".to_owned(), format!("    ListItem::new({title:?})")];
    if icon {
      code.push("        .icon(icons::PALETTE)".to_owned());
    }
    if description {
      code.push("        .description(\"Choose how lurq looks.\")".to_owned());
    }
    code.push("        .trailing(Select::new(theme.clone(), themes).size(SelectSize::Sm)),".to_owned());
    code.push(")".to_owned());
    if rows {
      code.push(
        ".item(ListItem::new(\"High contrast\").trailing(Switch::new(\"High contrast\").label_hidden(true)))"
          .to_owned(),
      );
      code.push(
        ".item(ListItem::new(\"Reduce motion\").trailing(Switch::new(\"Reduce motion\").label_hidden(true)))"
          .to_owned(),
      );
      code.push("// open: Signal<bool>, which a press of the card's row turns".to_owned());
      code.push(".bind(open.clone())".to_owned());
    }
    if let Some(_density) = density {
      code.push(format!(".density({density_code})"));
    }
    if disabled {
      code.push(".disabled(true)".to_owned());
    }
    if !fill {
      code.push(".width(360.0)".to_owned());
    }

    let mut tree = vec![
      format!(
        "root (group)   {}   card · edge 1, border · radius-lg",
        self.playground.measured()
      ),
      format!(
        "{} header     a setting: 14 in, 14 over and under{}",
        if rows { "├─" } else { "└─" },
        if rows { " · pressed" } else { "" }
      ),
    ];
    let bar = if rows { "│ " } else { "  " };
    if icon {
      tree.push(format!("{bar} ├─ icon      20, foreground"));
    }
    tree.push(format!("{bar} ├─ title     13, medium"));
    if description {
      tree.push(format!("{bar} ├─ description  12, muted"));
    }
    tree.push(format!(
      "{bar} {} trailing  the select{}",
      if rows { "├─" } else { "└─" },
      if rows { ", 12 before the chevron" } else { "" }
    ));
    if rows {
      tree.push(format!(
        "│  └─ chevron   16, muted · {}",
        if open { "up" } else { "down" }
      ));
      tree.push(format!(
        "└─ panel      {}",
        if open {
          "2 settings under a line"
        } else {
          "closed: nothing in it"
        }
      ));
    }

    let densities: Vec<(&str, usize)> = DENSITIES
      .iter()
      .enumerate()
      .map(|(index, density)| (density.0, index))
      .collect();
    let to_density = self.density.clone();
    let reset = {
      let (title, density) = (self.title.clone(), self.density.clone());
      let on = [self.description.clone(), self.icon.clone(), self.rows.clone()];
      let off = [self.open.clone(), self.disabled.clone(), self.fill.clone()];
      move || {
        title.set("App theme".to_owned());
        density.set(0);
        for flag in &on {
          flag.set(true);
        }
        for flag in &off {
          flag.set(false);
        }
      }
    };
    let changed = [
      title != "App theme",
      !description,
      !icon,
      !rows,
      rows && open,
      disabled,
      fill,
      density.is_some(),
    ]
    .into_iter()
    .filter(|changed| *changed)
    .count();
    let mut controls = vec![
      prop("title", "&str", field(&self.title)),
      prop(
        "density",
        "Density",
        self
          .playground
          .select(0, &densities, &self.density.get(), move |index| to_density.set(index)),
      ),
      divider(),
      flag("icon", &self.icon),
      flag("description", &self.description),
      flag("items", &self.rows),
    ];
    // Whether it is open is said only of a card that opens.
    if rows {
      controls.push(flag("open", &self.open));
    }
    controls.push(flag("disabled", &self.disabled));
    controls.push(flag("fill its room", &self.fill));
    self.playground.card(
      ctx,
      Spec {
        title: "Settings Card",
        make: Arc::new(move || make().into()),
        shown: Shown::of(shown),
        fill,
        forceable: rows,
        facts: format!(
          "{} · density {density_name}",
          match (rows, open) {
            (false, _) => "one setting",
            (true, false) => "closed",
            (true, true) => "open",
          }
        ),
        event: if rows { "on_open_change" } else { "" },
        code,
        tree,
        controls,
        changed,
        reset: Arc::new(reset),
      },
    )
  }
}

impl Component for SettingsCardPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 1),
      title: ctx.signal("App theme".to_owned()),
      description: ctx.signal(true),
      icon: ctx.signal(true),
      rows: ctx.signal(true),
      open: ctx.signal(false),
      disabled: ctx.signal(false),
      fill: ctx.signal(false),
      density: ctx.signal(0),
      theme: ctx.signal("system".to_owned()),
      contrast: ctx.signal(false),
      motion: ctx.signal(false),
      login: ctx.signal(true),
      channel: ctx.signal("stable".to_owned()),
      install: ctx.signal(false),
      went: ctx.signal(String::new()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    // The board's card of one setting, and the same that opens to more: mounted, it keeps whether it is open.
    let theme_row = || {
      ListItem::new("App theme")
        .icon(icons::PALETTE)
        .description("Choose how lurq looks.")
        .trailing(choice(&self.theme, themes(), "App theme"))
    };
    let cards = items()
      .child(item(
        held(360.0, SettingsCard::new(theme_row())),
        &format!("one setting · the signal has {:?}", self.theme.get()),
      ))
      .child(item(
        held(360.0, SettingsCard::new(theme_row()).items(self.more()).mount(ctx)),
        "with rows under it, it opens: press its row, or Enter on it",
      ));

    // The board's preferences: a card of rows under each heading.
    let groups = held(
      520.0,
      Column::new()
        .width(FILL)
        .spacing(16.0)
        .child(
          SettingsCard::group([
            ListItem::new("Open at login")
              .description("Start lurq when you sign in to this computer.")
              .trailing(toggle("Open at login", &self.login)),
            ListItem::new("Default project folder")
              .description("Where new projects are created.")
              .trailing(
                Button::new("Choose…")
                  .variant(ButtonVariant::Outline)
                  .size(ButtonSize::Sm),
              ),
          ])
          .label("Startup"),
        )
        .child(heading("Updates"))
        .child(
          SettingsCard::group([
            ListItem::new("Update channel")
              .description("Stable gets fewer, tested releases.")
              .trailing(choice(&self.channel, channels(), "Update channel")),
            ListItem::new("Install automatically")
              .description("Download and install in the background.")
              .trailing(toggle("Install automatically", &self.install)),
          ])
          .label("Updates"),
        ),
    );

    // Rows that lead to a page of their own: pressed, with a value and a chevron.
    let (to_keys, to_alerts) = (self.went.clone(), self.went.clone());
    let went = self.went.get();
    let leading = items().child(item(
      held(
        420.0,
        SettingsCard::group([
          ListItem::new("Keyboard shortcuts")
            .icon(icons::KEYBOARD)
            .description("The keys that run commands.")
            .chevron(true)
            .on_click(move |_| to_keys.set("Keyboard shortcuts".to_owned())),
          ListItem::new("Notifications")
            .icon(icons::BELL)
            .value("Banners")
            .chevron(true)
            .on_click(move |_| to_alerts.set("Notifications".to_owned())),
        ])
        .label("More"),
      ),
      &if went.is_empty() {
        "a pressed row is a stop of Tab; Up and Down move between them".to_owned()
      } else {
        format!("went to: {went}")
      },
    ));

    let more = || {
      [
        ListItem::new("High contrast").trailing(Switch::new("High contrast").label_hidden(true)),
        ListItem::new("Reduce motion").trailing(Switch::new("Reduce motion").label_hidden(true).on(true)),
      ]
    };
    let still = || {
      ListItem::new("App theme")
        .icon(icons::PALETTE)
        .description("Choose how lurq looks.")
    };
    let state = |card: SettingsCard, caption: &str| item(held(360.0, card), caption);
    let states = items()
      .child(state(SettingsCard::new(still()).items(more()), "closed"))
      .child(state(
        SettingsCard::new(still())
          .items(more())
          .force(Some(Interaction::Hovered)),
        "hover: its row under a veil",
      ))
      .child(state(
        SettingsCard::new(still())
          .items(more())
          .force(Some(Interaction::FocusVisible)),
        "focus-visible: the ring inside its row",
      ))
      .child(state(SettingsCard::new(still()).items(more()).open(true), "open"))
      .child(state(
        SettingsCard::new(still()).items(more()).open(true).disabled(true),
        "disabled: nothing in it is pressed",
      ));

    let card_rows = [
      (
        "setting",
        "ListItem",
        "",
        "SettingsCard::new: the card's own row, the setting it is the card of.",
      ),
      (
        "SettingsCard::group",
        "impl IntoIterator<ListItem>",
        "",
        "A card of several settings, a row each, with a line between each two.",
      ),
      (
        "item / items",
        "ListItem",
        "",
        "Rows, after those it holds: the settings of a group, or what the card of one setting opens to.",
      ),
      (
        "open",
        "bool",
        "",
        "Whether the rows under the card's own row are shown, where the app holds it.",
      ),
      (
        "bind",
        "Signal<bool>",
        "",
        "Open while the signal is true, which a press of the card's row turns.",
      ),
      (
        "default_open",
        "bool",
        "false",
        "Whether a mounted card that keeps it itself starts open.",
      ),
      (
        "on_open_change",
        "impl Fn(bool)",
        "",
        "Called on a press of the card's row with what the card is to be. Not for what the app sets.",
      ),
      (
        "mount",
        "&mut Ctx",
        "",
        "Puts it where it goes, keeping whether it is open where the app says neither open nor bind.",
      ),
      (
        "label",
        "&str",
        "its row's title",
        "What the card is called to what reads the tree. It draws nothing.",
      ),
      (
        "density",
        "Density",
        "the theme's",
        "How high its rows are: 8 lower compact, 8 higher comfortable, 12 higher under a finger.",
      ),
      ("width", "impl Into<Length>", "its room", "A width of its own."),
      (
        "disabled",
        "bool",
        "false",
        "Nothing in it is pressed or focused, and it is dimmed.",
      ),
    ];
    let row_rows = [
      (
        "ListItem::new",
        "&str",
        "",
        "A row: what the setting is called, 13 at 500.",
      ),
      (
        "description",
        "&str",
        "",
        "What it does, under the title in 12 of the muted color. It wraps.",
      ),
      (
        "icon / leading",
        "Name / Element",
        "",
        "Before the text: an icon of 20 in the foreground, or what the app puts there.",
      ),
      (
        "trailing",
        "impl Into<Element>",
        "",
        "The control that sets it, at the row's end: a switch, a select, a button. Its presses are its own.",
      ),
      (
        "value / chevron",
        "&str / bool",
        "",
        "What it is set to, and a chevron, for a row that leads to a page of its own.",
      ),
      (
        "on_click / href",
        "handler / &str",
        "",
        "Makes the row pressed: a stop of Tab, with the veils of a pressed row and the ring inside it.",
      ),
      ("disabled", "bool", "false", "That row is dimmed and takes nothing."),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Desktop", "Settings Card"],
        title: "Settings Card",
        description: "A card of a settings page: one setting with its icon, its title, its description and the \
                      control that sets it, which may open to more; or the rows of several settings in one card.",
        import: Some("use lurq::ui::{ListItem, SettingsCard};"),
        status: Some("New"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of SettingsCard is a control on the right. The card on the stage is live: its select \
           chooses, and with items its row opens it.",
          playground,
        ),
        Section::new(
          "Settings card",
          "The board's: one setting in a card of its own, its control at its end. Given rows, the card opens to \
           them: its own row is pressed and shows a chevron, and a press of the control in it stays the \
           control's.",
          self.frame.demo(
            0,
            cards,
            "SettingsCard::new(setting).items(more).mount(ctx)",
            &[
              "let setting = ListItem::new(\"App theme\")",
              "    .icon(icons::PALETTE)",
              "    .description(\"Choose how lurq looks.\")",
              "    .trailing(Select::new(theme.clone(), themes).size(SelectSize::Sm));",
              "",
              "SettingsCard::new(setting)",
              "",
              "// With rows it opens to them. Mounted, it keeps whether it is open.",
              "SettingsCard::new(setting)",
              "    .item(ListItem::new(\"High contrast\").trailing(Switch::new(\"High contrast\").label_hidden(true)))",
              "    .item(ListItem::new(\"Reduce motion\").trailing(Switch::new(\"Reduce motion\").label_hidden(true)))",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Settings rows",
          "The board's Settings Row is a ListItem in a card: SettingsCard::group holds several, with a line \
           between each two, as the groups of the board's preferences window. A heading over a card is the \
           page's own text; the card's label names it to what reads the tree.",
          self.frame.demo(
            1,
            groups,
            "SettingsCard::group([row, row])",
            &[
              "SettingsCard::group([",
              "    ListItem::new(\"Open at login\")",
              "        .description(\"Start lurq when you sign in to this computer.\")",
              "        .trailing(Switch::new(\"Open at login\").label_hidden(true).bind(login)),",
              "    ListItem::new(\"Default project folder\")",
              "        .description(\"Where new projects are created.\")",
              "        .trailing(Button::new(\"Choose…\").variant(ButtonVariant::Outline).size(ButtonSize::Sm)),",
              "])",
              ".label(\"Startup\")",
            ],
          ),
        ),
        Section::new(
          "Rows that lead somewhere",
          "A row given on_click or href is pressed as a whole: a setting with a page of its own, with what it is \
           set to and a chevron at its end.",
          self.frame.demo(
            2,
            leading,
            ".value(\"Banners\").chevron(true).on_click(..)",
            &[
              "SettingsCard::group([",
              "    ListItem::new(\"Keyboard shortcuts\")",
              "        .icon(icons::KEYBOARD)",
              "        .description(\"The keys that run commands.\")",
              "        .chevron(true)",
              "        .on_click(|_| go_to(\"/settings/keys\")),",
              "    ListItem::new(\"Notifications\")",
              "        .icon(icons::BELL)",
              "        .value(\"Banners\")",
              "        .chevron(true)",
              "        .href(\"/settings/notifications\"),",
              "])",
            ],
          ),
        ),
        Section::new(
          "States",
          "The board draws the card at rest. A card that opens has the veil of a pressed row on its own row under \
           the pointer and the ring of the focus inside it; open, its rows are under a line; disabled, it is \
           dimmed with all that is in it.",
          self.frame.demo(
            3,
            states,
            ".open(true)",
            &[
              "let card = || SettingsCard::new(setting()).items(more());",
              "card()",
              "card().force(Some(Interaction::Hovered))",
              "card().force(Some(Interaction::FocusVisible))",
              "card().open(true)",
              "card().open(true).disabled(true)",
            ],
          ),
        ),
        Section::new(
          "SettingsCard props",
          "What a settings card is given.",
          props_table(&card_rows, &["setting"], self.frame.room()),
        ),
        Section::new(
          "A row's props",
          "A row is a ListItem: what a settings card uses of it.",
          props_table(&row_rows, &["ListItem::new"], self.frame.room()),
        ),
      ],
    )
  }
}
