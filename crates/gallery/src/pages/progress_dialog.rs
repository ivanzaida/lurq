//! The page of the progress dialog: its playground, a dialog the page closes
//! when its work is cancelled, the one the board draws as it is, and one whose
//! end is not known.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{Button, ButtonVariant, ProgressDialog, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet, segmented_fill},
  playground::{Playground, Spec, divider, field, flag, prop},
};

/// How far the work of the playground is, of 128 items; `None` while its end is not known.
const VALUES: [(&str, Option<f32>); 5] = [
  ("none", None),
  ("0", Some(0.0)),
  ("42", Some(42.0)),
  ("96", Some(96.0)),
  ("128", Some(128.0)),
];

const TITLE: &str = "Copying 128 items";
const MESSAGE: &str = "from Downloads to projects/lurq/assets";
const STATUS: &str = "about 2 min left";

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .spacing(9.0)
    .overflow_visible()
    .child(Row::new().overflow_visible().child(shown))
    .child(quiet(caption, 12.0))
}

/// What is on a stage, side by side; what does not fit goes on to the next line.
fn items() -> Row {
  Row::new()
    .width(FILL)
    .spacing(28.0)
    .wrap()
    .overflow_visible()
    .align_items(Alignment::Start)
}

/// "42 of 128 · 1.2 GB of 3.4 GB", as the board says it.
fn items_and_bytes(done: f32, all: f32) -> String {
  let gigabytes = |items: f32| items / 128.0 * 3.4;
  format!(
    "{done} of {all} · {:.1} GB of {:.1} GB",
    gigabytes(done),
    gigabytes(all)
  )
}

/// The board's progress dialog: copying 128 items.
fn copying(open: &Signal<bool>, detail: &Signal<bool>) -> ProgressDialog {
  let cancel = open.clone();
  ProgressDialog::new(open.clone())
    .icon(icons::COPY)
    .title(TITLE)
    .message(MESSAGE)
    .value(42.0)
    .max(128.0)
    .format(items_and_bytes)
    .status(STATUS)
    .detail("assets/icons/warning.svg", detail.clone())
    .action("Pause", || {})
    .on_cancel(move || cancel.set(false))
}

pub struct ProgressDialogPage {
  frame: Frame,
  playground: Playground,
  open: Signal<bool>,
  title: Signal<String>,
  message: Signal<String>,
  value: Signal<usize>,
  status: Signal<String>,
  format: Signal<bool>,
  detail: Signal<bool>,
  detail_open: Signal<bool>,
  pause: Signal<bool>,
  cancel: Signal<bool>,
  copy_open: Signal<bool>,
  copy_detail: Signal<bool>,
  board_detail: Signal<bool>,
}

impl ProgressDialogPage {
  /// The playground: a button on the stage that opens the dialog the panel's props make. Its cancel button
  /// closes it, as an app does once the work has stopped.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (title, message, status) = (self.title.get(), self.message.get(), self.status.get());
    let (value_name, value) = VALUES[self.value.get().min(VALUES.len() - 1)];
    let (format, detail, pause, cancel) = (
      self.format.get(),
      self.detail.get(),
      self.pause.get(),
      self.cancel.get(),
    );
    let record = self.playground.recorder();
    let detail_open = self.detail_open.clone();
    let make = {
      let (title, message, status) = (title.clone(), message.clone(), status.clone());
      move |open: Signal<bool>, record: Option<Arc<dyn Fn(String) + Send + Sync>>| {
        let mut dialog = ProgressDialog::new(open.clone()).icon(icons::COPY).max(128.0);
        if let Some(value) = value {
          dialog = dialog.value(value);
        }
        if !title.is_empty() {
          dialog = dialog.title(&title);
        }
        if !message.is_empty() {
          dialog = dialog.message(&message);
        }
        if !status.is_empty() {
          dialog = dialog.status(&status);
        }
        if format {
          dialog = dialog.format(items_and_bytes);
        }
        if detail {
          dialog = dialog.detail("assets/icons/warning.svg", detail_open.clone());
        }
        if pause {
          let record = record.clone();
          dialog = dialog.action("Pause", move || {
            if let Some(record) = &record {
              record("action(\"Pause\")".to_owned());
            }
          });
        }
        if cancel {
          dialog = dialog.on_cancel(move || {
            if let Some(record) = &record {
              record("on_cancel()".to_owned());
            }
            open.set(false);
          });
        }
        dialog
      }
    };
    let recorded: Arc<dyn Fn(String) + Send + Sync> = Arc::new(record);
    let opener = {
      let open = self.open.clone();
      Button::new("Copy items…")
        .variant(ButtonVariant::Outline)
        .on_click(move |_| open.set(true))
    };
    let shown = Row::new().child(opener).child(make(self.open.clone(), Some(recorded)));

    let mut code = vec![
      "// open: Signal<bool>".to_owned(),
      "ProgressDialog::new(open.clone())".to_owned(),
      "    .icon(icons::COPY)".to_owned(),
    ];
    if !title.is_empty() {
      code.push(format!("    .title({title:?})"));
    }
    if !message.is_empty() {
      code.push(format!("    .message({message:?})"));
    }
    if let Some(value) = value {
      code.push(format!("    .value({value:?})"));
    }
    code.push("    .max(128.0)".to_owned());
    if format {
      code.push("    .format(|done, all| format!(\"{done} of {all} · ..\"))".to_owned());
    }
    if !status.is_empty() {
      code.push(format!("    .status({status:?})"));
    }
    if detail {
      code.push("    .detail(\"assets/icons/warning.svg\", detail_open.clone())".to_owned());
    }
    if pause {
      code.push("    .action(\"Pause\", || ..)".to_owned());
    }
    if cancel {
      code.push("    .on_cancel(move || open.set(false))".to_owned());
    }

    let mut tree = vec![
      "dialog      veil · panel 420 px · neither Escape nor the veil close it".to_owned(),
      "├─ header   copy 20 · title 15 · message 12".to_owned(),
      format!(
        "├─ progress 6 px · {}",
        if value.is_some() { "a value" } else { "indeterminate" }
      ),
      "├─ meta     mono 11 · count · status".to_owned(),
    ];
    tree.push(format!(
      "└─ footer   {}{}{}",
      if detail { "Details · " } else { "" },
      if pause { "Pause · " } else { "" },
      if cancel { "Cancel" } else { "" }
    ));

    let texts = [
      (self.title.clone(), TITLE),
      (self.message.clone(), MESSAGE),
      (self.status.clone(), STATUS),
    ];
    let on = [self.format.clone(), self.pause.clone(), self.cancel.clone()];
    let (to_value, to_detail) = (self.value.clone(), self.detail.clone());
    let pick_value = self.value.clone();
    let values: Vec<(&str, usize)> = VALUES
      .iter()
      .enumerate()
      .map(|(index, value)| (value.0, index))
      .collect();
    self.playground.card(
      ctx,
      Spec {
        title: "Progress Dialog",
        make: Arc::new(move || make(Signal::new(true), None).inline(true).into()),
        shown: shown.into(),
        fill: false,
        forceable: false,
        facts: format!(
          "{} · {}",
          if value.is_some() {
            format!("{value_name} of 128")
          } else {
            "indeterminate".to_owned()
          },
          if self.open.get() { "open" } else { "closed" }
        ),
        event: "on_cancel / action",
        code,
        tree,
        controls: vec![
          prop("title", "&str", field(&self.title)),
          prop("message", "&str", field(&self.message)),
          prop("status", "&str", field(&self.status)),
          prop(
            "value",
            "f64 of 128",
            segmented_fill(&values, &self.value.get(), move |index| pick_value.set(index)),
          ),
          divider(),
          flag("format", &self.format),
          flag("detail", &self.detail),
          flag("action", &self.pause),
          flag("on_cancel", &self.cancel),
        ],
        changed: [
          title != TITLE,
          message != MESSAGE,
          status != STATUS,
          self.value.get() != 2,
          !format,
          detail,
          !pause,
          !cancel,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          for (signal, text) in &texts {
            signal.set((*text).to_owned());
          }
          to_value.set(2);
          for flag in &on {
            flag.set(true);
          }
          to_detail.set(false);
        }),
      },
    )
  }
}

impl Component for ProgressDialogPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      open: ctx.signal(false),
      title: ctx.signal(TITLE.to_owned()),
      message: ctx.signal(MESSAGE.to_owned()),
      value: ctx.signal(2),
      status: ctx.signal(STATUS.to_owned()),
      format: ctx.signal(true),
      detail: ctx.signal(false),
      detail_open: ctx.signal(false),
      pause: ctx.signal(true),
      cancel: ctx.signal(true),
      copy_open: ctx.signal(false),
      copy_detail: ctx.signal(false),
      board_detail: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let opens = {
      let open = self.copy_open.clone();
      Row::new()
        .child(
          Button::new("Copy items…")
            .variant(ButtonVariant::Outline)
            .on_click(move |_| open.set(true)),
        )
        .child(copying(&self.copy_open, &self.copy_detail))
    };
    let closed = Signal::new(false);
    let drawn = items().child(item(
      copying(&closed, &self.board_detail).inline(true),
      "progress dialog",
    ));
    let unknown = items().child(item(
      ProgressDialog::new(closed.clone())
        .icon(icons::REFRESH)
        .title("Connecting to the registry")
        .message("crates.io")
        .on_cancel(|| {})
        .inline(true),
      "progress dialog · indeterminate",
    ));
    let rows = [
      (
        "open",
        "Signal<bool>",
        "",
        "Whether it is shown. The app's: it closes when the app sets it to false; Escape and the veil do not.",
      ),
      (
        "icon",
        "&str",
        "",
        "An icon of the set before the title: what kind of work it is.",
      ),
      (
        "title",
        "&str",
        "",
        "What is being done, 15/600. It is what the dialog is called.",
      ),
      ("message", "&str", "", "A line more of it, 12 in the muted color."),
      (
        "value",
        "f64",
        "",
        "How far the work is, a fraction of one unless max says otherwise. Without one its end is not known.",
      ),
      ("max", "f64", "1", "The whole of the work."),
      (
        "format",
        "Fn(f32, f32) -> String",
        "a percentage",
        "Says how much is done under the start of the bar: \"42 of 128\".",
      ),
      (
        "status",
        "&str",
        "",
        "What the app says of the rest, under the end of the bar.",
      ),
      (
        "detail",
        "&str, Signal<bool>",
        "",
        "A text behind \"Details\" at the start of the footer, open under it while the signal holds true.",
      ),
      (
        "action",
        "&str, Fn()",
        "",
        "A button before the cancel one: \"Pause\". It does not close the dialog.",
      ),
      (
        "on_cancel",
        "Fn()",
        "",
        "A cancel button, which has the focus as the dialog opens and which Escape presses. It does not close the dialog.",
      ),
      ("cancel_label", "&str", "\"Cancel\"", "The label of the cancel button."),
      ("width", "f32", "420 px", "A width of its own."),
      (
        "inline",
        "bool",
        "false",
        "The panel where it is declared, without the veil: for a gallery, a screenshot, a test.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Surfaces", "Progress Dialog"],
        title: "Progress Dialog",
        description: "A dialog over the window while a piece of work runs: what is being done, how far it is, and \
                      a way to stop it.",
        import: Some("use lurq::ui::ProgressDialog;"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of ProgressDialog is a control on the right. The button on the stage opens the dialog they \
           make; its cancel button closes it here, as an app does once the work has stopped.",
          playground,
        ),
        Section::new(
          "Progress dialog",
          "The dialog cannot be dismissed while the work runs: Escape presses Cancel, and the app closes it when \
           the work has stopped.",
          self.frame.demo(
            0,
            opens,
            "ProgressDialog::new(open).value(..).on_cancel(..)",
            &[
              "// open: Signal<bool>, detail: Signal<bool>",
              "let cancel = open.clone();",
              "ProgressDialog::new(open.clone())",
              "    .icon(icons::COPY)",
              "    .title(\"Copying 128 items\")",
              "    .message(\"from Downloads to projects/lurq/assets\")",
              "    .value(42.0)",
              "    .max(128.0)",
              "    .format(|done, all| format!(\"{done} of {all} · ..\"))",
              "    .status(\"about 2 min left\")",
              "    .detail(\"assets/icons/warning.svg\", detail.clone())",
              "    .action(\"Pause\", || {})",
              "    .on_cancel(move || cancel.set(false))",
            ],
          ),
        ),
        Section::new(
          "As the board draws it",
          "The panel alone, without the veil, where it is declared.",
          self.frame.demo(
            1,
            drawn,
            ".inline(true)",
            &[
              "ProgressDialog::new(open.clone())",
              "    .icon(icons::COPY)",
              "    .title(\"Copying 128 items\")",
              "    .message(\"from Downloads to projects/lurq/assets\")",
              "    .value(42.0)",
              "    .max(128.0)",
              "    .format(|done, all| format!(\"{done} of {all} · ..\"))",
              "    .status(\"about 2 min left\")",
              "    .detail(\"assets/icons/warning.svg\", detail.clone())",
              "    .action(\"Pause\", || {})",
              "    .on_cancel(|| {})",
              "    .inline(true)",
            ],
          ),
        ),
        Section::new(
          "Work whose end is not known",
          "Without a value the bar runs a segment across it, and nothing is said of how much is done.",
          self.frame.demo(
            2,
            unknown,
            "ProgressDialog::new(open)",
            &[
              "ProgressDialog::new(open.clone())",
              "    .icon(icons::REFRESH)",
              "    .title(\"Connecting to the registry\")",
              "    .message(\"crates.io\")",
              "    .on_cancel(|| {})",
              "    .inline(true)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a progress dialog is given.",
          props_table(&rows, &["open"], self.frame.room()),
        ),
      ],
    )
  }
}
