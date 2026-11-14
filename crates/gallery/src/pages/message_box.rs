//! The page of the message box: its playground, a question that is answered,
//! the boxes the board draws as they are, its kinds, and one with a detail.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{Button, ButtonVariant, MessageBox, MessageBoxKind, MessageBoxRole, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet, segmented_fill},
  playground::{Playground, Spec, divider, field, flag, prop},
};

/// The kinds, as the code names them.
const KINDS: [(&str, MessageBoxKind); 4] = [
  ("information", MessageBoxKind::Information),
  ("question", MessageBoxKind::Question),
  ("warning", MessageBoxKind::Warning),
  ("error", MessageBoxKind::Error),
];

const TITLE: &str = "Replace existing file?";
const MESSAGE: &str = "“theme.json” already exists in this folder. Replacing it overwrites its contents.";

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

/// A button that opens the message box `open` holds.
fn opener(label: &str, open: &Signal<bool>) -> Button {
  let open = open.clone();
  Button::new(label)
    .variant(ButtonVariant::Outline)
    .on_click(move |_| open.set(true))
}

/// The board's warning: replace an existing file, with "Don't ask again".
fn replace(open: &Signal<bool>, again: &Signal<bool>) -> MessageBox {
  MessageBox::new(open.clone())
    .kind(MessageBoxKind::Warning)
    .title(TITLE)
    .message(MESSAGE)
    .checkbox("Don't ask again", again.clone())
    .answer("Cancel", MessageBoxRole::Cancel, || {})
    .answer("Replace", MessageBoxRole::Default, || {})
}

/// The board's destructive question: delete projects.
fn delete(open: &Signal<bool>) -> MessageBox {
  MessageBox::new(open.clone())
    .kind(MessageBoxKind::Warning)
    .icon(icons::TRASH)
    .title("Delete 3 projects?")
    .message("The projects and their history are removed from this device. This cannot be undone.")
    .answer("Cancel", MessageBoxRole::Cancel, || {})
    .answer("Delete", MessageBoxRole::Destructive, || {})
}

/// The board's three choices: save the changes, do not, or go back.
fn unsaved(open: &Signal<bool>) -> MessageBox {
  MessageBox::new(open.clone())
    .kind(MessageBoxKind::Question)
    .icon(icons::BOX)
    .title("Do you want to save the changes to “blur.rs”?")
    .message("Your changes will be lost if you don't save them.")
    .width(440.0)
    .answer("Don't Save", MessageBoxRole::Plain, || {})
    .answer("Cancel", MessageBoxRole::Cancel, || {})
    .answer("Save", MessageBoxRole::Default, || {})
}

/// A failure with what went wrong behind "Details".
fn failed(open: &Signal<bool>, detail: &Signal<bool>) -> MessageBox {
  MessageBox::new(open.clone())
    .kind(MessageBoxKind::Error)
    .title("The project could not be saved.")
    .message("The disk is full. Free some space and try again.")
    .detail(
      "os error 28: no space left on device (writing projects/lurq/blur.rs)",
      detail.clone(),
    )
    .answer("OK", MessageBoxRole::Default, || {})
}

pub struct MessageBoxPage {
  frame: Frame,
  playground: Playground,
  open: Signal<bool>,
  title: Signal<String>,
  message: Signal<String>,
  kind: Signal<usize>,
  checkbox: Signal<bool>,
  checked: Signal<bool>,
  detail: Signal<bool>,
  detail_open: Signal<bool>,
  cancel: Signal<bool>,
  plain: Signal<bool>,
  destructive: Signal<bool>,
  replace_open: Signal<bool>,
  replace_again: Signal<bool>,
  delete_open: Signal<bool>,
  board_again: Signal<bool>,
  failed_detail: Signal<bool>,
}

impl MessageBoxPage {
  /// The playground: a button on the stage that opens the message box the panel's props make.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (title, message) = (self.title.get(), self.message.get());
    let (kind_name, kind) = KINDS[self.kind.get().min(KINDS.len() - 1)];
    let (checkbox, detail) = (self.checkbox.get(), self.detail.get());
    let (cancel, plain, destructive) = (self.cancel.get(), self.plain.get(), self.destructive.get());
    let record = self.playground.recorder();
    let (checked, detail_open) = (self.checked.clone(), self.detail_open.clone());
    // What the panel's props make, its answers saying what they were.
    let make = {
      let (title, message) = (title.clone(), message.clone());
      move |open: Signal<bool>, record: Option<Arc<dyn Fn(String) + Send + Sync>>| {
        let mut message_box = MessageBox::new(open).kind(kind);
        if !title.is_empty() {
          message_box = message_box.title(&title);
        }
        if !message.is_empty() {
          message_box = message_box.message(&message);
        }
        if detail {
          message_box = message_box.detail("theme.json · 4.2 KB · changed today at 09:41", detail_open.clone());
        }
        if checkbox {
          message_box = message_box.checkbox("Don't ask again", checked.clone());
        }
        let answer = |label: &'static str| {
          let record = record.clone();
          move || {
            if let Some(record) = &record {
              record(format!("answer({label:?})"));
            }
          }
        };
        if plain {
          message_box = message_box.answer("Keep Both", MessageBoxRole::Plain, answer("Keep Both"));
        }
        if cancel {
          message_box = message_box.answer("Cancel", MessageBoxRole::Cancel, answer("Cancel"));
        }
        let role = if destructive {
          MessageBoxRole::Destructive
        } else {
          MessageBoxRole::Default
        };
        message_box.answer("Replace", role, answer("Replace"))
      }
    };
    let recorded: Arc<dyn Fn(String) + Send + Sync> = Arc::new(record);
    let shown = Row::new()
      .child(opener("Replace file…", &self.open))
      .child(make(self.open.clone(), Some(recorded)));

    let mut code = vec![
      "// open: Signal<bool>".to_owned(),
      "MessageBox::new(open.clone())".to_owned(),
      format!("    .kind(MessageBoxKind::{kind:?})"),
    ];
    if !title.is_empty() {
      code.push(format!("    .title({title:?})"));
    }
    if !message.is_empty() {
      code.push(format!("    .message({message:?})"));
    }
    if detail {
      code.push("    .detail(\"theme.json · 4.2 KB · changed today at 09:41\", detail_open.clone())".to_owned());
    }
    if checkbox {
      code.push("    .checkbox(\"Don't ask again\", checked.clone())".to_owned());
    }
    if plain {
      code.push("    .answer(\"Keep Both\", MessageBoxRole::Plain, || ..)".to_owned());
    }
    if cancel {
      code.push("    .answer(\"Cancel\", MessageBoxRole::Cancel, || ..)".to_owned());
    }
    code.push(format!(
      "    .answer(\"Replace\", MessageBoxRole::{}, || ..)",
      if destructive { "Destructive" } else { "Default" }
    ));

    let mut tree = vec![
      "dialog      veil · panel 400 px · alertdialog".to_owned(),
      format!("├─ body     well 36 px · {kind_name} · title 15 · message 13"),
    ];
    if detail {
      tree.push("│  └─ detail  Details · mono 11".to_owned());
    }
    tree.push(format!(
      "└─ footer   {}{}{}",
      if checkbox { "checkbox · " } else { "" },
      if plain { "Keep Both · " } else { "" },
      match (cancel, destructive) {
        (true, true) => "Cancel · Replace (destructive)",
        (true, false) => "Cancel · Replace",
        (false, true) => "Replace (destructive)",
        (false, false) => "Replace",
      }
    ));

    let to = (self.title.clone(), self.message.clone(), self.kind.clone());
    let off = [
      self.checkbox.clone(),
      self.detail.clone(),
      self.plain.clone(),
      self.destructive.clone(),
    ];
    let on = self.cancel.clone();
    let to_kind = self.kind.clone();
    let kinds: Vec<(&str, usize)> = KINDS.iter().enumerate().map(|(index, kind)| (kind.0, index)).collect();
    let (checked, detail_open) = (self.checked.clone(), self.detail_open.clone());
    self.playground.card(
      ctx,
      Spec {
        title: "Message Box",
        make: Arc::new(move || make(Signal::new(true), None).inline(true).into()),
        shown: shown.into(),
        fill: false,
        forceable: false,
        facts: format!(
          "{kind_name} · {}{}",
          if self.open.get() { "open" } else { "closed" },
          if checkbox && checked.get() { " · checked" } else { "" }
        ),
        event: "answer",
        code,
        tree,
        controls: vec![
          prop("title", "&str", field(&self.title)),
          prop("message", "&str", field(&self.message)),
          prop(
            "kind",
            "MessageBoxKind",
            segmented_fill(&kinds, &self.kind.get(), move |index| to_kind.set(index)),
          ),
          divider(),
          flag("checkbox", &self.checkbox),
          flag("detail", &self.detail),
          flag("cancel answer", &self.cancel),
          flag("plain answer", &self.plain),
          flag("destructive", &self.destructive),
        ],
        changed: [
          title != TITLE,
          message != MESSAGE,
          kind != MessageBoxKind::Warning,
          checkbox,
          detail,
          !cancel,
          plain,
          destructive,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          to.0.set(TITLE.to_owned());
          to.1.set(MESSAGE.to_owned());
          to.2.set(2);
          for flag in &off {
            flag.set(false);
          }
          on.set(true);
          checked.set(false);
          detail_open.set(false);
        }),
      },
    )
  }
}

impl Component for MessageBoxPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      open: ctx.signal(false),
      title: ctx.signal(TITLE.to_owned()),
      message: ctx.signal(MESSAGE.to_owned()),
      kind: ctx.signal(2),
      checkbox: ctx.signal(false),
      checked: ctx.signal(false),
      detail: ctx.signal(false),
      detail_open: ctx.signal(false),
      cancel: ctx.signal(true),
      plain: ctx.signal(false),
      destructive: ctx.signal(false),
      replace_open: ctx.signal(false),
      replace_again: ctx.signal(false),
      delete_open: ctx.signal(false),
      board_again: ctx.signal(false),
      failed_detail: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let asks = Row::new()
      .spacing(12.0)
      .child(opener("Replace file…", &self.replace_open))
      .child(replace(&self.replace_open, &self.replace_again))
      .child(opener("Delete projects…", &self.delete_open))
      .child(delete(&self.delete_open));
    let closed = Signal::new(false);
    let drawn = items()
      .child(item(
        replace(&closed, &self.board_again).inline(true),
        "message box · warning",
      ))
      .child(item(delete(&closed).inline(true), "message box · destructive"))
      .child(item(unsaved(&closed).inline(true), "unsaved changes · three choices"));
    let mut kinds = items();
    for (name, kind) in KINDS {
      kinds = kinds.child(item(
        MessageBox::new(closed.clone())
          .kind(kind)
          .width(320.0)
          .title(&format!("A message box of the {name} kind"))
          .answer("OK", MessageBoxRole::Default, || {})
          .inline(true),
        &format!("MessageBoxKind::{kind:?}"),
      ));
    }
    let detail = items().child(item(
      failed(&closed, &self.failed_detail).inline(true),
      "error · detail",
    ));
    let rows = [
      (
        "open",
        "Signal<bool>",
        "",
        "Whether it is shown. The app's: every answer sets it to false before what the answer does runs.",
      ),
      (
        "kind",
        "MessageBoxKind",
        "Information",
        "The icon in its well and their colors: information, question, warning, error.",
      ),
      (
        "icon",
        "&str",
        "",
        "An icon of the set in place of the kind's: trash for a question that deletes.",
      ),
      (
        "title",
        "&str",
        "",
        "What it asks or says, 15/600. It is what the message box is called.",
      ),
      (
        "message",
        "&str",
        "",
        "More of it under the title, 13 in the muted color. It describes the box.",
      ),
      (
        "detail",
        "&str, Signal<bool>",
        "",
        "A text behind \"Details\" under the message, open while the signal holds true.",
      ),
      (
        "detail_label",
        "&str",
        "\"Details\"",
        "The label of what opens the detail.",
      ),
      (
        "checkbox",
        "&str, Signal<bool>",
        "",
        "A checkbox at the start of the footer, checked while the signal holds true. Enter on it gives the default answer.",
      ),
      (
        "answer",
        "&str, MessageBoxRole, Fn()",
        "",
        "An answer. Default: filled, Enter gives it. Cancel: Escape and a press of the veil give it. Destructive: in the destructive color, and the focus goes to cancel. Plain: at the start of the footer.",
      ),
      ("width", "f32", "400 px", "A width of its own."),
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
        breadcrumb: &["Components", "Surfaces", "Message Box"],
        title: "Message Box",
        description: "A dialog that asks one question or says one thing and waits for the answer: replace a file, \
                      save the changes, delete projects.",
        import: Some("use lurq::ui::{MessageBox, MessageBoxKind, MessageBoxRole};"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of MessageBox is a control on the right. The button on the stage opens the message box they \
           make; its answers, Enter, Escape and a press of the veil close it, and the Events tab says which answer \
           was given.",
          playground,
        ),
        Section::new(
          "Message box",
          "As it opens the focus goes to the default answer, or to Cancel where an answer is destructive. Enter \
           gives the default answer wherever the focus is but on another button; Escape and a press of the veil \
           give Cancel.",
          self.frame.demo(
            0,
            asks,
            "MessageBox::new(open.clone()).answer(..)",
            &[
              "// open: Signal<bool>, again: Signal<bool>",
              "MessageBox::new(open.clone())",
              "    .kind(MessageBoxKind::Warning)",
              "    .title(\"Replace existing file?\")",
              "    .message(\"“theme.json” already exists in this folder. Replacing it overwrites its contents.\")",
              "    .checkbox(\"Don't ask again\", again.clone())",
              "    .answer(\"Cancel\", MessageBoxRole::Cancel, || {})",
              "    .answer(\"Replace\", MessageBoxRole::Default, || {})",
              "",
              "MessageBox::new(open.clone())",
              "    .kind(MessageBoxKind::Warning)",
              "    .icon(icons::TRASH)",
              "    .title(\"Delete 3 projects?\")",
              "    .message(\"The projects and their history are removed from this device. This cannot be undone.\")",
              "    .answer(\"Cancel\", MessageBoxRole::Cancel, || {})",
              "    .answer(\"Delete\", MessageBoxRole::Destructive, || {})",
            ],
          ),
        ),
        Section::new(
          "As the board draws them",
          "The panel alone, without the veil, where it is declared. A plain answer stands at the start of the \
           footer; at its end Cancel comes first, whatever order the answers are given in.",
          self.frame.demo(
            1,
            drawn,
            ".inline(true)",
            &[
              "MessageBox::new(open.clone())",
              "    .kind(MessageBoxKind::Question)",
              "    .icon(icons::BOX)",
              "    .title(\"Do you want to save the changes to “blur.rs”?\")",
              "    .message(\"Your changes will be lost if you don't save them.\")",
              "    .width(440.0)",
              "    .answer(\"Don't Save\", MessageBoxRole::Plain, || {})",
              "    .answer(\"Cancel\", MessageBoxRole::Cancel, || {})",
              "    .answer(\"Save\", MessageBoxRole::Default, || {})",
              "    .inline(true)",
            ],
          ),
        ),
        Section::new(
          "Kinds",
          "The kind is the icon in the well and its colors. A message box with a destructive answer has its icon in \
           the destructive colors whatever its kind.",
          self.frame.demo(
            2,
            kinds,
            ".kind(MessageBoxKind::Error)",
            &[
              "MessageBox::new(open.clone())",
              "    .kind(MessageBoxKind::Error)",
              "    .width(320.0)",
              "    .title(\"A message box of the error kind\")",
              "    .answer(\"OK\", MessageBoxRole::Default, || {})",
              "    .inline(true)",
            ],
          ),
        ),
        Section::new(
          "Detail",
          "What went wrong, where a person can open it: \"Details\" under the message, which turns the app's signal.",
          self.frame.demo(
            3,
            detail,
            ".detail(text, open)",
            &[
              "// detail: Signal<bool>",
              "MessageBox::new(open.clone())",
              "    .kind(MessageBoxKind::Error)",
              "    .title(\"The project could not be saved.\")",
              "    .message(\"The disk is full. Free some space and try again.\")",
              "    .detail(\"os error 28: no space left on device (writing projects/lurq/blur.rs)\", detail.clone())",
              "    .answer(\"OK\", MessageBoxRole::Default, || {})",
              "    .inline(true)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a message box is given.",
          props_table(&rows, &["open"], self.frame.room()),
        ),
      ],
    )
  }
}
