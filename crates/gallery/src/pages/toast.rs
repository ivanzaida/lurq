//! The page of the toast: its playground, its variants, the toast with an
//! action and a timer, and toasts shown from anywhere by a `Toasts`.

use std::{sync::Arc, time::Duration};

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{Button, ButtonSize, ButtonVariant, Toast, ToastPosition, ToastVariant, Toasts},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet, segmented_fill},
  playground::{Playground, Spec, divider, field, flag, prop},
};

/// The variants, by the names the code gives them.
const VARIANTS: [(&str, ToastVariant); 5] = [
  ("Neutral", ToastVariant::Neutral),
  ("Success", ToastVariant::Success),
  ("Info", ToastVariant::Info),
  ("Warning", ToastVariant::Warning),
  ("Error", ToastVariant::Error),
];

/// How long the playground's toast is shown, in seconds: none for one that stays.
const DURATIONS: [(&str, Option<u64>); 4] = [("3 s", Some(3)), ("5 s", Some(5)), ("10 s", Some(10)), ("stays", None)];

/// Where the page's host shows its toasts.
const POSITIONS: [(&str, ToastPosition); 6] = [
  ("TopLeft", ToastPosition::TopLeft),
  ("Top", ToastPosition::Top),
  ("TopRight", ToastPosition::TopRight),
  ("BottomLeft", ToastPosition::BottomLeft),
  ("Bottom", ToastPosition::Bottom),
  ("BottomRight", ToastPosition::BottomRight),
];

fn named<T: PartialEq + Copy>(table: &[(&'static str, T)], value: T) -> &'static str {
  table
    .iter()
    .find(|(_, held)| *held == value)
    .map_or("", |(name, _)| name)
}

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .spacing(9.0)
    .child(Row::new().overflow_visible().child(shown))
    .child(quiet(caption, 12.0))
}

/// The action of the board's toast: a small outlined button.
fn undo() -> Button {
  Button::new("Undo").variant(ButtonVariant::Outline).size(ButtonSize::Sm)
}

pub struct ToastPage {
  frame: Frame,
  playground: Playground,
  /// The toasts the page shows: its host is in the last section.
  toasts: Toasts,
  title: Signal<String>,
  description: Signal<String>,
  variant: Signal<ToastVariant>,
  action: Signal<bool>,
  timer: Signal<bool>,
  closable: Signal<bool>,
  duration: Signal<Option<u64>>,
  position: Signal<ToastPosition>,
}

impl ToastPage {
  /// The playground: the toast as the panel's props make it, and a button that shows it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (title, description, variant) = (self.title.get(), self.description.get(), self.variant.get());
    let (action, timer, closable) = (self.action.get(), self.timer.get(), self.closable.get());
    let (duration, position) = (self.duration.get(), self.position.get());
    let record = self.playground.recorder();
    let plain = {
      let (title, description) = (title.clone(), description.clone());
      move || {
        let mut toast = Toast::new(&title).variant(variant).timer(timer).closable(closable);
        if !description.is_empty() {
          toast = toast.description(&description);
        }
        if action {
          toast = toast.action(undo());
        }
        match duration {
          Some(seconds) => toast.duration(Duration::from_secs(seconds)),
          None => toast.persistent(true),
        }
      }
    };
    let panel = plain().force(self.playground.forced());
    let show = {
      let (toasts, plain, record) = (self.toasts.clone(), plain.clone(), record.clone());
      Button::new("Show toast").on_click(move |_| {
        let id = toasts.show(plain());
        record(format!("show() -> {id:?}"));
      })
    };
    let stage = Column::new()
      .overflow_visible()
      .spacing(24.0)
      .align_items(Alignment::Center)
      .child(Row::new().overflow_visible().child(panel))
      .child(show);

    let mut code = vec![format!("Toast::new({title:?})")];
    if !description.is_empty() {
      code.push(format!("    .description({description:?})"));
    }
    if variant != ToastVariant::Neutral {
      code.push(format!("    .variant(ToastVariant::{})", named(&VARIANTS, variant)));
    }
    if action {
      code.push("    .action(Button::new(\"Undo\").variant(ButtonVariant::Outline).size(ButtonSize::Sm))".to_owned());
    }
    if timer {
      code.push("    .timer(true)".to_owned());
    }
    if !closable {
      code.push("    .closable(false)".to_owned());
    }
    match duration {
      Some(5) => {}
      Some(seconds) => code.push(format!("    .duration(Duration::from_secs({seconds}))")),
      None => code.push("    .persistent(true)".to_owned()),
    }
    code.push("// toasts: Toasts, whose host the app's root renders last".to_owned());
    code.push("toasts.show(toast);".to_owned());

    let mut tree = vec![format!("root            {}", self.playground.measured())];
    tree.push(format!("{} content", if timer { "├─" } else { "└─" }));
    let bar = if timer { "│ " } else { "  " };
    if let Some(icon) = variant.icon() {
      tree.push(format!("{bar}  ├─ icon         16 × 16 px   {icon}"));
    }
    tree.push(format!("{bar}  ├─ text"));
    tree.push(format!(
      "{bar}  │  {} title    {title:?}",
      if description.is_empty() { "└─" } else { "├─" }
    ));
    if !description.is_empty() {
      tree.push(format!("{bar}  │  └─ description"));
    }
    if action {
      tree.push(format!("{bar}  ├─ action       Undo"));
    }
    if closable {
      tree.push(format!("{bar}  └─ close        16 × 16 px"));
    }
    if timer {
      tree.push("└─ timer          2 px   └─ remaining".to_owned());
    }

    let (to_title, to_description, to_variant) = (self.title.clone(), self.description.clone(), self.variant.clone());
    let flags = [self.action.clone(), self.timer.clone()];
    let (to_closable, to_duration, to_position) = (self.closable.clone(), self.duration.clone(), self.position.clone());
    let (set_variant, set_position) = (self.variant.clone(), self.position.clone());
    let set_duration = self.duration.clone();
    self.playground.card(
      ctx,
      Spec {
        title: "Toast",
        make: Arc::new(move || plain().into()),
        shown: stage.into(),
        fill: false,
        forceable: true,
        facts: format!("{} · {}", named(&VARIANTS, variant), named(&POSITIONS, position)),
        event: "show",
        code,
        tree,
        controls: vec![
          prop("title", "&str", field(&self.title)),
          prop("description", "&str", field(&self.description)),
          prop(
            "variant",
            "ToastVariant",
            self
              .playground
              .select(0, &VARIANTS, &variant, move |chosen| set_variant.set(chosen)),
          ),
          prop(
            "duration",
            "Duration",
            segmented_fill(&DURATIONS, &duration, move |chosen| set_duration.set(chosen)),
          ),
          divider(),
          flag("action", &self.action),
          flag("timer", &self.timer),
          flag("closable", &self.closable),
          divider(),
          prop(
            "host_at",
            "ToastPosition",
            self
              .playground
              .select(1, &POSITIONS, &position, move |chosen| set_position.set(chosen)),
          ),
        ],
        changed: [
          variant != ToastVariant::Success,
          action,
          timer,
          !closable,
          duration != Some(5),
          position != ToastPosition::BottomRight,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          to_title.set("Settings saved".to_owned());
          to_description.set("Your theme applies to every open window.".to_owned());
          to_variant.set(ToastVariant::Success);
          for flag in &flags {
            flag.set(false);
          }
          to_closable.set(true);
          to_duration.set(Some(5));
          to_position.set(ToastPosition::BottomRight);
        }),
      },
    )
  }

  /// A button of the last section that shows `toast`.
  fn shows(&self, label: &str, toast: impl Fn() -> Toast + Send + Sync + 'static) -> Button {
    let toasts = self.toasts.clone();
    Button::new(label).variant(ButtonVariant::Outline).on_click(move |_| {
      toasts.show(toast());
    })
  }
}

impl Component for ToastPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 2),
      toasts: Toasts::new(),
      title: ctx.signal("Settings saved".to_owned()),
      description: ctx.signal("Your theme applies to every open window.".to_owned()),
      variant: ctx.signal(ToastVariant::Success),
      action: ctx.signal(false),
      timer: ctx.signal(false),
      closable: ctx.signal(true),
      duration: ctx.signal(Some(5)),
      position: ctx.signal(ToastPosition::BottomRight),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let variants = VARIANTS.iter().fold(
      Column::new().overflow_visible().spacing(20.0),
      |column, (name, variant)| {
        let toast = Toast::new(match variant {
          ToastVariant::Neutral => "Link copied",
          ToastVariant::Success => "Settings saved",
          ToastVariant::Info => "A new version is ready",
          ToastVariant::Warning => "Saved, but not synced",
          ToastVariant::Error => "Could not connect",
        })
        .description(match variant {
          ToastVariant::Neutral => "The link to this page is on the clipboard.",
          ToastVariant::Success => "Your theme applies to every open window.",
          ToastVariant::Info => "It is installed when the app starts again.",
          ToastVariant::Warning => "The changes are sent when you are online.",
          ToastVariant::Error => "The MCP server refused the connection.",
        })
        .variant(*variant);
        column.child(item(toast, &format!("ToastVariant::{name}")))
      },
    );
    let rich = Row::new().overflow_visible().child(
      Toast::new("File moved to trash")
        .description("theme-dark.json")
        .variant(ToastVariant::Success)
        .action(undo())
        .timer(true),
    );
    let anywhere = Row::new()
      .width(FILL)
      .wrap()
      .spacing(12.0)
      .child(self.shows("Settings saved", || {
        Toast::new("Settings saved")
          .description("Your theme applies to every open window.")
          .variant(ToastVariant::Success)
      }))
      .child(self.shows("File moved to trash", || {
        Toast::new("File moved to trash")
          .description("theme-dark.json")
          .variant(ToastVariant::Success)
          .action(undo())
          .timer(true)
      }))
      .child(self.shows("Could not connect", || {
        Toast::new("Could not connect")
          .description("The MCP server refused the connection.")
          .variant(ToastVariant::Error)
          .persistent(true)
      }))
      .child({
        let toasts = self.toasts.clone();
        Button::new("Clear all")
          .variant(ButtonVariant::Ghost)
          .on_click(move |_| toasts.clear())
      });
    // The page's own host: what draws the toasts it shows, over the whole window.
    let host = self.toasts.host_at(ctx, self.position.get());
    let rows = [
      ("title", "&str", "", "What happened, in a few words."),
      ("description", "&str", "", "More of it, under the title."),
      (
        "variant",
        "ToastVariant",
        "Neutral",
        "Neutral, Success, Info, Warning or Error: the icon, and its color. A neutral toast has none.",
      ),
      (
        "icon",
        "Name",
        "",
        "An icon of the theme's set in place of the variant's.",
      ),
      ("icon_hidden", "bool", "false", "No icon, whatever the variant."),
      (
        "action",
        "Button | Link",
        "",
        "What can be done about it. The toast is centered on it; shown by a Toasts, it leaves when it is pressed.",
      ),
      ("closable", "bool", "true", "Whether it has its close control."),
      ("on_close", "impl Fn()", "", "Called on a press of the close control."),
      (
        "timer",
        "bool",
        "false",
        "A rail along its bottom edge that says how long it has left.",
      ),
      (
        "duration",
        "Duration",
        "5 s",
        "How long a Toasts shows it. The time stands still while the pointer is on the toasts.",
      ),
      (
        "persistent",
        "bool",
        "false",
        "It stays until it is closed, or its action is pressed.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Surfaces & overlays", "Toast"],
        title: "Toast",
        description: "A short message that something happened, in a corner of the window over everything, that \
                      leaves by itself: settings saved, a file moved to the trash, a connection that failed.",
        import: Some("use lurq::ui::{Toast, Toasts};"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Toast is a control on the right. The panel on the stage is the toast as they make it; Show \
           toast sends it to the page's Toasts, which shows it in the corner the last control names.",
          playground,
        ),
        Section::new(
          "Variants",
          "The variant is the icon and its color. A neutral toast has no icon, unless it is given one.",
          self.frame.demo(
            0,
            variants,
            ".variant(ToastVariant::Success)",
            &[
              "Toast::new(\"Settings saved\")",
              "    .description(\"Your theme applies to every open window.\")",
              "    .variant(ToastVariant::Success)",
              "",
              "Toast::new(\"Could not connect\")",
              "    .description(\"The MCP server refused the connection.\")",
              "    .variant(ToastVariant::Error)",
            ],
          ),
        ),
        Section::new(
          "With an action and a timer",
          "An action is a Button or a Link before the close control; the toast is centered on it. The timer is a rail \
           along the bottom edge: shown by a Toasts, what is left runs out over the toast's duration.",
          self.frame.demo(
            1,
            rich,
            ".action(..).timer(true)",
            &[
              "Toast::new(\"File moved to trash\")",
              "    .description(\"theme-dark.json\")",
              "    .variant(ToastVariant::Success)",
              "    .action(",
              "        Button::new(\"Undo\")",
              "            .variant(ButtonVariant::Outline)",
              "            .size(ButtonSize::Sm)",
              "            .on_click(move |_| restore()),",
              "    )",
              "    .timer(true)",
            ],
          ),
        ),
        Section::new(
          "Shown from anywhere",
          "A Toasts is made once, at the app's root, and cloned into whatever shows a toast. Its host, which the root \
           renders last, draws them in a corner of the window over everything. Several are stacked; with the pointer \
           on them they are laid out one over another and their time stands still.",
          Column::new()
            .width(FILL)
            .child(self.frame.demo(
              2,
              anywhere,
              "toasts.show(Toast::new(..))",
              &[
                "// In the root component's create: once.",
                "let toasts = Toasts::new();",
                "",
                "// In its render, last, so that its layer is over every other.",
                "column.child(self.toasts.host(ctx))",
                "",
                "// Anywhere, with a clone of it.",
                "toasts.show(Toast::new(\"Settings saved\").variant(ToastVariant::Success));",
              ],
            ))
            .child(host),
        ),
        Section::new(
          "Props",
          "What a toast is given.",
          props_table(&rows, &["title"], self.frame.room()),
        ),
      ],
    )
  }
}
