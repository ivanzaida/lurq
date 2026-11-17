//! The page of the command bar: its playground with a width to pick, an
//! editor's window under the board's bar, the bar narrowed, its commands in
//! each state, and the commands that are only behind its chevron.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{
    CommandBar, CommandBarCommand, CommandBarTab, Interaction, Toggle, ToolbarCommand, ToolbarGroup, ToolbarItem, icons,
  },
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

/// The widths the playground offers the bar: the board's, and narrower ones its groups overflow.
const WIDTHS: [(&str, Option<f32>); 5] = [
  ("Fill", None),
  ("860", Some(860.0)),
  ("600", Some(600.0)),
  ("420", Some(420.0)),
  ("300", Some(300.0)),
];

/// What the bar's commands are bound to, as the page holds it.
#[derive(Clone)]
struct Doc {
  tab: Signal<&'static str>,
  bold: Signal<bool>,
  italic: Signal<bool>,
  underline: Signal<bool>,
  strike: Signal<bool>,
  table: Signal<bool>,
  sidebar: Signal<bool>,
}

type Record = Arc<dyn Fn(String) + Send + Sync>;

impl Doc {
  /// A command that tells `record` it ran.
  fn command(record: &Record, label: &'static str, icon: &str) -> CommandBarCommand {
    let record = record.clone();
    CommandBarCommand::new(label)
      .icon(icon)
      .on_select(move || record(format!("{label}: on_select")))
  }

  /// A command that is on or off, bound to `on`.
  fn toggled(record: &Record, label: &'static str, icon: &str, on: &Signal<bool>) -> CommandBarCommand {
    let (record, set) = (record.clone(), on.clone());
    CommandBarCommand::new(label)
      .icon(icon)
      .large(true)
      .on(on.get())
      .on_change(move |on| {
        set.set(on);
        record(format!("{label}: on_change({on})"));
      })
  }

  /// A mark of the text, its icon alone, bound to `on`.
  fn mark(record: &Record, label: &'static str, icon: &str, on: &Signal<bool>) -> Toggle {
    let (record, set) = (record.clone(), on.clone());
    Toggle::new(label)
      .icon(icon)
      .icon_only(true)
      .on(on.get())
      .on_change(move |on| {
        set.set(on);
        record(format!("{label}: on_change({on})"));
      })
  }

  /// The board's tabs: Home with Clipboard, Format, Insert and View; Insert, View and Help with a group each.
  fn tabs(&self, record: &Record, secondary: bool) -> Vec<CommandBarTab<&'static str>> {
    let command = |label, icon| Self::command(record, label, icon);
    let clipboard = ToolbarGroup::new()
      .label("Clipboard")
      .item(command("Paste", icons::PASTE).large(true))
      .item(ToolbarItem::stack([
        command("Cut", icons::CUT),
        command("Copy", icons::COPY),
      ]));
    let format = ToolbarGroup::new()
      .label("Format")
      .item(ToolbarItem::from(Self::mark(record, "Bold", icons::BOLD, &self.bold)).shortcut("Ctrl+B"))
      .item(ToolbarItem::from(Self::mark(record, "Italic", icons::ITALIC, &self.italic)).shortcut("Ctrl+I"))
      .item(ToolbarItem::from(Self::mark(record, "Underline", icons::UNDERLINE, &self.underline)).shortcut("Ctrl+U"))
      .item(Self::mark(record, "Strikethrough", icons::STRIKETHROUGH, &self.strike));
    let insert = || {
      ToolbarGroup::new()
        .label("Insert")
        .item(command("Image", icons::IMAGE).large(true))
        .item(Self::toggled(record, "Table", icons::TABLE, &self.table))
        .item(command("Link", icons::LINK).large(true))
    };
    let view = || {
      ToolbarGroup::new()
        .label("View")
        .item(Self::toggled(record, "Sidebar", icons::PANEL_LEFT, &self.sidebar))
        .item(ToolbarItem::stack([
          command("Zoom in", icons::ZOOM_IN),
          command("Zoom out", icons::ZOOM_OUT),
        ]))
    };
    let mut home = CommandBarTab::new("home", "Home")
      .item(clipboard)
      .item(format)
      .item(insert())
      .item(view());
    if secondary {
      let (customize, keys) = (record.clone(), record.clone());
      home = home
        .secondary_command(
          ToolbarCommand::new("Customize")
            .icon(icons::SLIDERS)
            .on_select(move || customize("Customize: on_select".to_owned())),
        )
        .secondary_command(
          ToolbarCommand::new("Keyboard shortcuts")
            .icon(icons::KEYBOARD)
            .on_select(move || keys("Keyboard shortcuts: on_select".to_owned())),
        );
    }
    vec![
      home,
      CommandBarTab::new("insert", "Insert").item(insert()),
      CommandBarTab::new("view", "View").item(view()),
      CommandBarTab::new("help", "Help").item(
        ToolbarGroup::new()
          .label("Help")
          .item(command("Help", icons::HELP_CIRCLE).large(true))
          .item(command("About", icons::INFO).large(true)),
      ),
    ]
  }

  /// The board's bar, bound to the page's tab, telling `record` what it did.
  fn bar(&self, record: &Record, secondary: bool) -> CommandBar<&'static str> {
    let told = record.clone();
    CommandBar::new(self.tab.clone(), self.tabs(record, secondary))
      .on_change(move |tab| told(format!("on_change({tab:?})")))
  }
}

/// What a bar on the page is shown in: `width` wide, or as wide as the room.
fn sized(width: Option<f32>, bar: Element) -> Element {
  Column::new()
    .width(width.map_or(FILL, Into::into))
    .align_items(Alignment::Stretch)
    .overflow_visible()
    .child(bar)
    .into()
}

pub struct CommandBarPage {
  frame: Frame,
  playground: Playground,
  doc: Doc,
  width: Signal<usize>,
  label: Signal<String>,
  disabled: Signal<bool>,
  secondary: Signal<bool>,
}

impl CommandBarPage {
  /// The playground: the board's bar as the panel's props make it, as wide as the width picked.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (width_name, width) = WIDTHS[self.width.get().min(WIDTHS.len() - 1)];
    let (label, disabled, secondary) = (self.label.get(), self.disabled.get(), self.secondary.get());
    let record: Record = Arc::new(self.playground.recorder());
    let plain = {
      let (doc, label, record) = (self.doc.clone(), label.clone(), record.clone());
      move || doc.bar(&record, secondary).label(&label).disabled(disabled)
    };
    let bar = self
      .playground
      .styled(plain().force(self.playground.forced()))
      .mount(ctx);
    let measured = Column::new()
      .width(FILL)
      .align_items(Alignment::Stretch)
      .overflow_visible()
      .child(bar);
    let shown = sized(width, self.playground.measure(measured).width(FILL).into());

    let mut code = vec![
      "CommandBar::new(tab, [".to_owned(),
      "    CommandBarTab::new(\"home\", \"Home\")".to_owned(),
      "        .item(ToolbarGroup::new().label(\"Clipboard\")".to_owned(),
      "            .item(CommandBarCommand::new(\"Paste\").icon(icons::PASTE).large(true))".to_owned(),
      "            .item(ToolbarItem::stack([cut, copy])))".to_owned(),
      "        .item(ToolbarGroup::new().label(\"Format\").item(bold).item(italic))".to_owned(),
      "        .item(ToolbarGroup::new().label(\"Insert\").item(image).item(table.on(true)))".to_owned(),
    ];
    if secondary {
      code.push("        .secondary_command(ToolbarCommand::new(\"Customize\"))".to_owned());
    }
    code.push("        ,".to_owned());
    code.push("    CommandBarTab::new(\"insert\", \"Insert\").item(..),".to_owned());
    code.push("])".to_owned());
    if label != "Commands" {
      code.push(format!("    .label({label:?})"));
    }
    if disabled {
      code.push("    .disabled(true)".to_owned());
    }
    code.push("    .on_change(|tab| ..)".to_owned());
    code.push("    .mount(ctx)".to_owned());
    let tree = vec![
      format!(
        "root        {} · background · border · radius lg",
        self.playground.measured()
      ),
      "├─ tabs      32 · muted · line under · padding 8 · gap 2".to_owned(),
      "│  └─ tab    padding 12 · 13 · selected 600, underlined 2".to_owned(),
      "└─ bar       a toolbar · padding 8 / 4".to_owned(),
      "   ├─ group  padding 10 · line at its end · label 10 under".to_owned(),
      "   │  ├─ command  large 56 × 52 · icon 20 · label 11".to_owned(),
      "   │  └─ stack    small 24 · icon 14 · label 12 · gap 2".to_owned(),
      "   └─ more   chevron · menu of what does not fit".to_owned(),
    ];

    let (to_width, to_label, to_disabled, to_secondary) = (
      self.width.clone(),
      self.label.clone(),
      self.disabled.clone(),
      self.secondary.clone(),
    );
    let set_width = self.width.clone();
    let widths: Vec<(&str, usize)> = WIDTHS
      .iter()
      .enumerate()
      .map(|(index, (name, _))| (*name, index))
      .collect();
    let make = {
      let plain = plain.clone();
      move || sized(width, plain().into())
    };
    self.playground.card(
      ctx,
      Spec {
        title: "Command Bar",
        make: Arc::new(make),
        shown: Shown::of(shown).measured(),
        fill: true,
        forceable: true,
        facts: match width {
          Some(_) => format!("{width_name} wide"),
          None => "as wide as the stage".to_owned(),
        },
        event: "on_change / on_select",
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
          prop("label", "&str", field(&self.label)),
          divider(),
          flag("disabled", &self.disabled),
          flag("secondary_command", &self.secondary),
        ],
        changed: [width.is_some(), label != "Commands", disabled, secondary]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset: Arc::new(move || {
          to_width.set(0);
          to_label.set("Commands".to_owned());
          to_disabled.set(false);
          to_secondary.set(false);
        }),
      },
    )
  }

  /// An editor's window: the bar over the page it acts on.
  fn window(&self, ctx: &mut Ctx) -> Element {
    let record: Record = Arc::new(|_| {});
    let bar = self.doc.bar(&record, true).label("Editor").mount(ctx);
    let mark = |on: bool, text: &str| if on { format!("[{text}]") } else { text.to_owned() };
    let words = format!(
      "{} · {} · {}",
      mark(self.doc.bold.get(), "bold"),
      mark(self.doc.italic.get(), "italic"),
      mark(self.doc.underline.get(), "underline"),
    );
    Column::new()
      .width(FILL)
      .align_items(Alignment::Stretch)
      .spacing(12.0)
      .overflow_visible()
      .child(bar)
      .child(
        Column::new()
          .padding_horizontal(16.0)
          .spacing(6.0)
          .child(quiet("Quarterly report.docx", 13.0))
          .child(quiet(&format!("The marks on: {words}"), 12.0)),
      )
      .into()
  }

  /// The board's bar at two widths: what does not fit goes behind the chevron, a group whole.
  fn narrow(&self, ctx: &mut Ctx) -> Element {
    let record: Record = Arc::new(|_| {});
    let mut row = Column::new().width(FILL).spacing(20.0).overflow_visible();
    let room = self.frame.stage_room();
    for width in [600.0_f32, 420.0] {
      // On a stage narrower than a bar the bar is as wide as the stage, and says so.
      let width = if room > 0.0 { width.min(room) } else { width };
      row = row.child(
        Column::new()
          .spacing(8.0)
          .overflow_visible()
          .child(sized(Some(width), self.doc.bar(&record, false).mount(ctx)))
          .child(quiet(&format!("{width} wide"), 12.0)),
      );
    }
    row.into()
  }

  /// A large and a small command, a toggled one and a disabled one, at rest and in each interaction.
  fn states() -> Element {
    let specimen = |interaction: Option<Interaction>, caption: &str| {
      Column::new()
        .spacing(10.0)
        .align_items(Alignment::Center)
        .child(
          Row::new()
            .spacing(6.0)
            .align_items(Alignment::Center)
            .child(
              CommandBarCommand::new("Paste")
                .icon(icons::PASTE)
                .large(true)
                .force(interaction),
            )
            .child(
              CommandBarCommand::new("Table")
                .icon(icons::TABLE)
                .large(true)
                .on(true)
                .force(interaction),
            )
            .child(
              Column::new()
                .spacing(2.0)
                .child(CommandBarCommand::new("Cut").icon(icons::CUT).force(interaction))
                .child(
                  // A disabled command takes no pointer and no focus: it is drawn at rest.
                  CommandBarCommand::new("Copy").icon(icons::COPY).disabled(true),
                ),
            ),
        )
        .child(quiet(caption, 12.0))
    };
    Row::new()
      .width(FILL)
      .spacing(28.0)
      .wrap()
      .child(specimen(None, "idle"))
      .child(specimen(Some(Interaction::Hovered), "hover"))
      .child(specimen(Some(Interaction::Pressed), "pressed"))
      .child(specimen(Some(Interaction::FocusVisible), "focus-visible"))
      .into()
  }
}

impl Component for CommandBarPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 1),
      doc: Doc {
        tab: ctx.signal("home"),
        bold: ctx.signal(true),
        italic: ctx.signal(false),
        underline: ctx.signal(false),
        strike: ctx.signal(false),
        table: ctx.signal(true),
        sidebar: ctx.signal(false),
      },
      width: ctx.signal(0),
      label: ctx.signal("Commands".to_owned()),
      disabled: ctx.signal(false),
      secondary: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let rows = [
      (
        "new",
        "(Signal<T>, tabs)",
        "",
        "A bar whose selected tab is the signal's value; selecting another sets it.",
      ),
      (
        "controlled",
        "(T, tabs, on_change)",
        "",
        "A bar whose selected tab the app holds, told the value of each other one selected.",
      ),
      (
        "mount",
        "(ctx)",
        "",
        "Puts the bar in the tree: its tabs and its toolbar keep what did not fit, the menu and where Tab enters.",
      ),
      (
        "on_change",
        "impl Fn(&T)",
        "",
        "Called when a person selects another tab, after the signal is set.",
      ),
      (
        "label",
        "&str",
        "\"Commands\"",
        "What the bar is called to tooling; its row of commands is called by the selected tab.",
      ),
      ("disabled", "bool", "false", "Its tabs and its commands take no input."),
      (
        "more_label",
        "&str",
        "\"More commands\"",
        "What the chevron at the end is called, in its tooltip and to tooling.",
      ),
      (
        "CommandBarTab::new",
        "(T, &str)",
        "",
        "A tab of a value and a label, with no commands yet.",
      ),
      (
        "CommandBarTab::item, items",
        "impl Into<ToolbarItem>",
        "",
        "What the tab's bar holds, as a toolbar has it: labelled ToolbarGroups, ToolbarItem::stack, commands, \
         toggles, a select.",
      ),
      (
        "CommandBarTab::secondary_command(s)",
        "ToolbarCommand",
        "",
        "A command only in the menu behind the chevron, which is then always there.",
      ),
      (
        "CommandBarCommand::new",
        "(&str)",
        "",
        "A command: an icon and a label, 24 high.",
      ),
      (
        "CommandBarCommand::large",
        "bool",
        "false",
        "Its icon of 20 over its label of 11, 56 by 52.",
      ),
      (
        "CommandBarCommand::on",
        "bool",
        "",
        "It is a toggle, picked out while on; on_change is told the other value.",
      ),
      (
        "CommandBarCommand::on_select / on_change",
        "impl Fn() / impl Fn(bool)",
        "",
        "What running it does, by a press, a key or its item in the menu.",
      ),
      (
        "style, rules, state, force, id, key",
        "",
        "",
        "As every themed builder has them, on the bar and on a command.",
      ),
    ];
    let playground = self.play(ctx);
    let window = self.window(ctx);
    let narrow = self.narrow(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Desktop", "Command Bar"],
        title: "Command Bar",
        description: "The tabs of a window's commands over the commands of the selected one, in labelled groups: \
                      the ribbon of an editor or a file manager. A toolbar under tabs, whose groups go behind its \
                      chevron as it narrows.",
        import: Some("use lurq::ui::{CommandBar, CommandBarCommand, CommandBarTab, ToolbarGroup, ToolbarItem};"),
        status: Some("Beta"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop is a control on the right. Pick a width: the groups that do not fit go behind the chevron, \
           the last first. Select a tab, run a command, turn a toggle on: the log under the stage says what \
           the bar told.",
          playground,
        ),
        Section::new(
          "An editor's window",
          "The board's bar over the document it acts on. Its tabs are one stop of Tab and its commands another; \
           Left and Right move in each. Bold, Italic and Underline are toggles, Table and Sidebar are large commands \
           that are on or off, and Cut over Copy is a stack. Customize and Keyboard shortcuts are only behind the \
           chevron.",
          self.frame.demo(
            0,
            window,
            "CommandBar::new(tab, tabs)",
            &[
              "CommandBar::new(tab, [home, insert, view, help])",
              "    .label(\"Editor\")",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "What does not fit",
          "The same bar at two widths. The groups that do not fit leave the bar the last first, each whole, and the \
           chevron opens a menu of them under their labels, a toggle checked.",
          self.frame.demo(
            1,
            narrow,
            "a group whole, under its label",
            &["CommandBar::new(tab, tabs).mount(ctx)"],
          ),
        ),
        Section::new(
          "Commands",
          "A large command, a large one that is on, and a stack of two small ones, the second disabled: at rest, \
           under the pointer, held down and focused from the keyboard.",
          self.frame.demo(
            2,
            Self::states(),
            "CommandBarCommand::new(\"Paste\").large(true)",
            &[
              "CommandBarCommand::new(\"Paste\").icon(icons::PASTE).large(true)",
              "CommandBarCommand::new(\"Table\").icon(icons::TABLE).large(true).on(true)",
              "ToolbarItem::stack([",
              "    CommandBarCommand::new(\"Cut\").icon(icons::CUT),",
              "    CommandBarCommand::new(\"Copy\").icon(icons::COPY).disabled(true),",
              "])",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a command bar, its tabs and its commands are given.",
          props_table(&rows, &["new", "mount"], self.frame.room()),
        ),
      ],
    )
  }
}
