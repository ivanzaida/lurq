//! The page of the alert: its playground, the board's alerts, each
//! severity, and an alert with an action, a close control and a body.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{Alert, AlertVariant, Button, ButtonSize, ButtonVariant, Progress, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet, segmented_fill},
  playground::{Playground, Spec, divider, field, flag, prop},
};

/// The variants, as the board names them and as the code does.
pub(crate) const VARIANTS: [(AlertVariant, &str, &str); 5] = [
  (AlertVariant::Neutral, "neutral", "AlertVariant::Neutral"),
  (AlertVariant::Info, "info", "AlertVariant::Info"),
  (AlertVariant::Success, "success", "AlertVariant::Success"),
  (AlertVariant::Warning, "warning", "AlertVariant::Warning"),
  (AlertVariant::Destructive, "destructive", "AlertVariant::Destructive"),
];

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .width(360.0)
    .spacing(10.0)
    .child(shown)
    .child(quiet(caption, 12.0))
}

/// What is on a stage, side by side; what does not fit goes on to the next line.
fn items() -> Row {
  Row::new()
    .width(FILL)
    .spacing(24.0)
    .wrap()
    .align_items(Alignment::Start)
}

pub struct AlertPage {
  frame: Frame,
  playground: Playground,
  title: Signal<String>,
  description: Signal<String>,
  variant: Signal<usize>,
  icon: Signal<bool>,
  action: Signal<bool>,
  closable: Signal<bool>,
  body: Signal<bool>,
  shown: Signal<bool>,
}

impl AlertPage {
  /// The playground: the alert as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (title, description) = (self.title.get(), self.description.get());
    let (variant, variant_name, variant_code) = VARIANTS[self.variant.get().min(VARIANTS.len() - 1)];
    let (icon, action, closable, body) = (self.icon.get(), self.action.get(), self.closable.get(), self.body.get());
    let plain = {
      let (title, description) = (title.clone(), description.clone());
      move || {
        let mut alert = Alert::new(&title).variant(variant);
        if !description.is_empty() {
          alert = alert.description(&description);
        }
        if !icon {
          alert = alert.icon(None);
        }
        if action {
          alert = alert.action(
            Button::new("Install")
              .variant(ButtonVariant::Outline)
              .size(ButtonSize::Sm),
          );
        }
        if body {
          alert = alert.child(Progress::new(0.6).label("Downloading"));
        }
        alert
      }
    };
    let record = self.playground.recorder();
    let mut alert = plain().force(self.playground.forced());
    if closable {
      alert = alert.on_close(move || record("on_close".to_owned()));
    }
    let make = {
      let plain = plain.clone();
      move || {
        let alert = plain();
        Column::new()
          .width(360.0)
          .child(if closable { alert.on_close(|| ()) } else { alert })
          .into()
      }
    };

    let mut code = vec![format!("Alert::new({title:?})")];
    if !description.is_empty() {
      code.push(format!("    .description({description:?})"));
    }
    if variant != AlertVariant::Neutral {
      code.push(format!("    .variant({variant_code})"));
    }
    if !icon {
      code.push("    .icon(None)".to_owned());
    }
    if action {
      code
        .push("    .action(Button::new(\"Install\").variant(ButtonVariant::Outline).size(ButtonSize::Sm))".to_owned());
    }
    if body {
      code.push("    .child(Progress::new(0.6).label(\"Downloading\"))".to_owned());
    }
    if closable {
      code.push("    .on_close(move || shown.set(false))".to_owned());
    }

    let mut tree = vec![format!(
      "root          {}   {variant_name} · radius 8 · padded 16",
      self.playground.measured()
    )];
    if icon {
      tree.push(format!("├─ icon        {} · 16", variant.icon()));
    }
    tree.push(format!(
      "{} text        title{}{}",
      if action || closable { "├─" } else { "└─" },
      if description.is_empty() { "" } else { " · description" },
      if body { " · body" } else { "" }
    ));
    if action {
      tree.push(format!("{} action      Install", if closable { "├─" } else { "└─" }));
    }
    if closable {
      tree.push("└─ close       16 · muted".to_owned());
    }

    let to = (self.title.clone(), self.description.clone(), self.variant.clone());
    let flags = [
      self.icon.clone(),
      self.action.clone(),
      self.closable.clone(),
      self.body.clone(),
    ];
    let choose = self.variant.clone();
    let variants: Vec<(&str, usize)> = VARIANTS.iter().enumerate().map(|(index, v)| (v.1, index)).collect();
    self.playground.card(
      ctx,
      Spec {
        title: "Alert",
        make: Arc::new(make),
        shown: Column::new().width(360.0).child(alert).into(),
        fill: false,
        forceable: closable,
        facts: format!("{variant_name} · {}", variant.role()),
        event: "on_close",
        code,
        tree,
        controls: vec![
          prop("title", "&str", field(&self.title)),
          prop("description", "&str", field(&self.description)),
          prop(
            "variant",
            "AlertVariant",
            segmented_fill(&variants, &self.variant.get(), move |index| choose.set(index)),
          ),
          divider(),
          flag("icon", &self.icon),
          flag("action", &self.action),
          flag("on_close", &self.closable),
          flag("child", &self.body),
        ],
        changed: [
          title != "Update available",
          description != "lurq 0.61 is ready to install.",
          variant != AlertVariant::Neutral,
          !icon,
          action,
          closable,
          body,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          to.0.set("Update available".to_owned());
          to.1.set("lurq 0.61 is ready to install.".to_owned());
          to.2.set(0);
          for flag in &flags {
            flag.set(false);
          }
          flags[0].set(true);
        }),
      },
    )
  }
}

impl Component for AlertPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      title: ctx.signal("Update available".to_owned()),
      description: ctx.signal("lurq 0.61 is ready to install.".to_owned()),
      variant: ctx.signal(0),
      icon: ctx.signal(true),
      action: ctx.signal(false),
      closable: ctx.signal(false),
      body: ctx.signal(false),
      shown: ctx.signal(true),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let board = items()
      .child(item(
        Alert::new("Update available").description("lurq 0.61 is ready to install."),
        "alert",
      ))
      .child(item(
        Alert::new("Could not save")
          .description("The settings file is read-only.")
          .variant(AlertVariant::Destructive),
        "alert destructive",
      ));
    let severities = items()
      .child(item(
        Alert::new("New shortcut")
          .description("Press Ctrl+K to open the command palette.")
          .variant(AlertVariant::Info),
        "alert info",
      ))
      .child(item(
        Alert::new("Backup complete")
          .description("All 128 files were copied.")
          .variant(AlertVariant::Success),
        "alert success",
      ))
      .child(item(
        Alert::new("Low disk space")
          .description("Less than 10% of the disk is free.")
          .variant(AlertVariant::Warning),
        "alert warning",
      ))
      .child(item(
        Alert::new("Title alone").variant(AlertVariant::Info),
        "title alone",
      ));
    let shown = self.shown.clone();
    let dismissable: Element = if self.shown.get() {
      let hide = shown.clone();
      Alert::new("Could not sync")
        .description("Three changes are waiting for a connection.")
        .variant(AlertVariant::Warning)
        .action(
          Button::new("Retry")
            .variant(ButtonVariant::Outline)
            .size(ButtonSize::Sm),
        )
        .on_close(move || hide.set(false))
        .into()
    } else {
      Button::new("Show the alert again")
        .variant(ButtonVariant::Outline)
        .on_click(move |_| shown.set(true))
        .into()
    };
    let composed = items().child(item(dismissable, "action · on_close")).child(item(
      Alert::new("Downloading lurq 0.61")
        .icon(icons::DOWNLOAD)
        .variant(AlertVariant::Info)
        .child(Progress::new(0.6).label("Downloading")),
      "icon · child",
    ));
    let rows = [
      (
        "new",
        "&str",
        "",
        "The title: what it is about, 14 and medium. It is what the alert is called.",
      ),
      (
        "description",
        "&str",
        "",
        "More of it under the title, 13, muted or in the foreground's color.",
      ),
      (
        "variant",
        "AlertVariant",
        "Neutral",
        "Neutral, Info, Success, Warning, Destructive: the fill, the edge and the icon.",
      ),
      (
        "icon",
        "&str | None",
        "the variant's",
        "An icon of the theme's set in place of the variant's, or none.",
      ),
      (
        "action",
        "impl Into<Element>",
        "",
        "At the end, before the close control: a button, a link.",
      ),
      (
        "child / children",
        "impl Into<Element>",
        "",
        "Elements under the description, for more than text.",
      ),
      (
        "on_close",
        "impl Fn()",
        "",
        "Draws the close control and is called on a press of it. The app takes the alert away.",
      ),
      (
        "role",
        "&str",
        "alert · status",
        "What it is to tooling: an alert for Warning and Destructive, a status for the others.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Data", "Alert"],
        title: "Alert",
        description: "A message in the flow of a page about what is on it: a file that could not be saved, a disk \
                      that is nearly full, a shortcut to know. It is read where it stands, as wide as the room it \
                      is given, and is not an overlay.",
        import: Some("use lurq::ui::{Alert, AlertVariant};"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Alert is a control on the right. With on_close on, the alert has its close control, and \
           each press is counted.",
          playground,
        ),
        Section::new(
          "Alert",
          "The board's alerts: a neutral one in the card's color, and one that says what went wrong, with an edge, \
           an icon and a title of the destructive color.",
          self.frame.demo(
            0,
            board,
            "Alert::new(..).description(..)",
            &[
              "Alert::new(\"Update available\").description(\"lurq 0.61 is ready to install.\")",
              "",
              "Alert::new(\"Could not save\")",
              "    .description(\"The settings file is read-only.\")",
              "    .variant(AlertVariant::Destructive)",
            ],
          ),
        ),
        Section::new(
          "Severities",
          "An informative, a successful and a warning alert are in the muted form of their color, with an edge and \
           an icon of that color. A title alone is enough.",
          self.frame.demo(
            1,
            severities,
            ".variant(AlertVariant::Success)",
            &[
              "Alert::new(\"New shortcut\")",
              "    .description(\"Press Ctrl+K to open the command palette.\")",
              "    .variant(AlertVariant::Info)",
              "Alert::new(\"Backup complete\").description(\"All 128 files were copied.\").variant(AlertVariant::Success)",
              "Alert::new(\"Low disk space\").description(\"Less than 10% of the disk is free.\").variant(AlertVariant::Warning)",
              "Alert::new(\"Title alone\").variant(AlertVariant::Info)",
            ],
          ),
        ),
        Section::new(
          "Action, close control and body",
          "An action stands at the end, before the close control; the app takes the alert away when it is closed. \
           What the app puts in it stands under the description.",
          self.frame.demo(
            2,
            composed,
            ".action(..).on_close(..).child(..)",
            &[
              "// shown: Signal<bool>",
              "Alert::new(\"Could not sync\")",
              "    .description(\"Three changes are waiting for a connection.\")",
              "    .variant(AlertVariant::Warning)",
              "    .action(Button::new(\"Retry\").variant(ButtonVariant::Outline).size(ButtonSize::Sm))",
              "    .on_close(move || shown.set(false))",
              "",
              "Alert::new(\"Downloading lurq 0.61\")",
              "    .icon(icons::DOWNLOAD)",
              "    .variant(AlertVariant::Info)",
              "    .child(Progress::new(0.6).label(\"Downloading\"))",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What an alert is given. Its title is all it needs.",
          props_table(&rows, &["new"], self.frame.room()),
        ),
      ],
    )
  }
}
