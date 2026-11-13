//! The page of the dialog: its playground, a dialog that asks one thing, the
//! two the board draws as they are, and one whose body scrolls.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx, theme::PaletteColor},
  components::{Column, Row},
  core::Signal,
  layout::{Alignment, text_style::FontWeight},
  node::Element,
  ui::{Button, ButtonVariant, Dialog, DialogSize},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet, segmented_fill, text},
  playground::{Playground, Spec, divider, field, flag, prop},
};

/// The sizes, as the board names them, how the code names them, and how wide each is.
const SIZES: [(Option<DialogSize>, &str, &str, f32); 5] = [
  (None, "none", "", 420.0),
  (Some(DialogSize::Sm), "sm", "DialogSize::Sm", 360.0),
  (Some(DialogSize::Md), "md", "DialogSize::Md", 480.0),
  (Some(DialogSize::Lg), "lg", "DialogSize::Lg", 640.0),
  (Some(DialogSize::Full), "full", "DialogSize::Full", 0.0),
];

/// The paragraphs of the terms, as the board writes them.
const TERMS: [&str; 6] = [
  "By installing lurq you agree to the MIT license and to the terms below.",
  "The software is provided as is, without warranty of any kind, express or implied.",
  "You may copy, modify and distribute it, provided the license notice stays intact.",
  "Contributions are accepted under the same terms.",
  "Nothing here limits what the license itself allows.",
  "These terms may change with a new release; the release notes say so.",
];

/// A paragraph of a dialog's body: 13, in the muted color.
fn paragraph(content: &str) -> lurq::components::Text {
  text(content, 13.0, FontWeight::Normal).color(PaletteColor::MutedForeground)
}

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .spacing(9.0)
    .child(Row::new().child(shown))
    .child(quiet(caption, 12.0))
}

/// What is on a stage, side by side; what does not fit goes on to the next line.
fn items() -> Row {
  Row::new()
    .width(FILL)
    .spacing(28.0)
    .wrap()
    .align_items(Alignment::Start)
}

/// A button that opens the dialog `open` holds.
fn opener(label: &str, open: &Signal<bool>) -> Button {
  let open = open.clone();
  Button::new(label)
    .variant(ButtonVariant::Outline)
    .on_click(move |_| open.set(true))
}

/// The board's dialog that asks one thing: delete a workspace.
fn delete_workspace(open: &Signal<bool>) -> Dialog {
  let (cancel, delete) = (open.clone(), open.clone());
  Dialog::new(open.clone())
    .title("Delete workspace?")
    .description("This removes every window layout and setting stored for this workspace. It cannot be undone.")
    .footer(
      Button::new("Cancel")
        .variant(ButtonVariant::Outline)
        .on_click(move |_| cancel.set(false)),
    )
    .footer(
      Button::new("Delete")
        .variant(ButtonVariant::Destructive)
        .on_click(move |_| delete.set(false)),
    )
}

/// The board's dialog whose body scrolls: the terms of use.
fn terms(open: &Signal<bool>) -> Dialog {
  let (decline, accept) = (open.clone(), open.clone());
  Dialog::new(open.clone())
    .scrollable(true)
    .close_button(true)
    .title("Terms of use")
    .children(TERMS.map(paragraph))
    .footer(
      Button::new("Decline")
        .variant(ButtonVariant::Outline)
        .on_click(move |_| decline.set(false)),
    )
    .footer(
      Button::new("Accept")
        .variant(ButtonVariant::Primary)
        .on_click(move |_| accept.set(false)),
    )
}

pub struct DialogPage {
  frame: Frame,
  playground: Playground,
  open: Signal<bool>,
  title: Signal<String>,
  description: Signal<String>,
  size: Signal<usize>,
  scrollable: Signal<bool>,
  close_button: Signal<bool>,
  dismiss_on_veil: Signal<bool>,
  dismiss_on_escape: Signal<bool>,
  delete_open: Signal<bool>,
  terms_open: Signal<bool>,
}

impl DialogPage {
  /// The playground: a button on the stage that opens the dialog the panel's props make.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (title, description) = (self.title.get(), self.description.get());
    let (size, size_name, size_code, width) = SIZES[self.size.get().min(SIZES.len() - 1)];
    let (scrollable, close_button) = (self.scrollable.get(), self.close_button.get());
    let (on_veil, on_escape) = (self.dismiss_on_veil.get(), self.dismiss_on_escape.get());
    // What the panel's props make, open or not: `inline` draws it in a window of its own.
    let make = {
      let (title, description) = (title.clone(), description.clone());
      move |open: Signal<bool>| {
        let mut dialog = Dialog::new(open)
          .scrollable(scrollable)
          .close_button(close_button)
          .dismiss_on_veil(on_veil)
          .dismiss_on_escape(on_escape);
        if !title.is_empty() {
          dialog = dialog.title(&title);
        }
        if !description.is_empty() {
          dialog = dialog.description(&description);
        }
        if let Some(size) = size {
          dialog = dialog.size(size);
        }
        if scrollable {
          dialog = dialog.children(TERMS.map(paragraph));
        }
        dialog
      }
    };
    let record = self.playground.recorder();
    let footer = |label: &'static str, variant: ButtonVariant| {
      let (open, record) = (self.open.clone(), record.clone());
      Button::new(label).variant(variant).on_click(move |_| {
        open.set(false);
        record(format!("on_click({label:?})"));
      })
    };
    let dialog = make(self.open.clone())
      .footer(footer("Cancel", ButtonVariant::Outline))
      .footer(footer("Confirm", ButtonVariant::Primary));
    let shown = Row::new().child(opener("Open dialog", &self.open)).child(dialog);

    let mut code = vec![
      "// open: Signal<bool>".to_owned(),
      "Dialog::new(open.clone())".to_owned(),
    ];
    if !title.is_empty() {
      code.push(format!("    .title({title:?})"));
    }
    if !description.is_empty() {
      code.push(format!("    .description({description:?})"));
    }
    if size.is_some() {
      code.push(format!("    .size({size_code})"));
    }
    for (name, set, default) in [
      ("scrollable", scrollable, false),
      ("close_button", close_button, false),
      ("dismiss_on_veil", on_veil, true),
      ("dismiss_on_escape", on_escape, true),
    ] {
      if set != default {
        code.push(format!("    .{name}({set})"));
      }
    }
    if scrollable {
      code.push("    .children(paragraphs)".to_owned());
    }
    code.push("    .footer(Button::new(\"Cancel\").variant(ButtonVariant::Outline))".to_owned());
    code.push("    .footer(Button::new(\"Confirm\"))".to_owned());

    let width = if width > 0.0 {
      format!("{width} px")
    } else {
      "the window".to_owned()
    };
    let mut tree = vec![
      "veil        the window   scrim".to_owned(),
      format!("└─ panel    {width}   popover · radius 12 · shadow lg"),
      format!(
        "   ├─ header   {}",
        if close_button { "title · close 28 px" } else { "title" }
      ),
    ];
    if scrollable {
      tree.push("   ├─ body     scrolls between the lines".to_owned());
    } else {
      tree.push("   ├─ body     scrolls where the window is low".to_owned());
    }
    tree.push("   └─ footer   Cancel · Confirm".to_owned());

    let to = (self.title.clone(), self.description.clone(), self.size.clone());
    let flags = [self.scrollable.clone(), self.close_button.clone()];
    let dismissals = [self.dismiss_on_veil.clone(), self.dismiss_on_escape.clone()];
    let to_size = self.size.clone();
    let sizes: Vec<(&str, usize)> = SIZES.iter().enumerate().map(|(index, size)| (size.1, index)).collect();
    self.playground.card(
      ctx,
      Spec {
        title: "Dialog",
        make: Arc::new(move || {
          make(Signal::new(true))
            .inline(true)
            .footer(Button::new("Cancel").variant(ButtonVariant::Outline))
            .footer(Button::new("Confirm"))
            .into()
        }),
        shown: shown.into(),
        fill: false,
        forceable: false,
        facts: format!("{size_name} · {}", if self.open.get() { "open" } else { "closed" }),
        event: "on_click",
        code,
        tree,
        controls: vec![
          prop("title", "&str", field(&self.title)),
          prop("description", "&str", field(&self.description)),
          prop(
            "size",
            "DialogSize",
            segmented_fill(&sizes, &self.size.get(), move |index| to_size.set(index)),
          ),
          divider(),
          flag("scrollable", &self.scrollable),
          flag("close_button", &self.close_button),
          flag("dismiss_on_veil", &self.dismiss_on_veil),
          flag("dismiss_on_escape", &self.dismiss_on_escape),
        ],
        changed: [
          title != "Delete workspace?",
          description != "It cannot be undone.",
          size.is_some(),
          scrollable,
          close_button,
          !on_veil,
          !on_escape,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          to.0.set("Delete workspace?".to_owned());
          to.1.set("It cannot be undone.".to_owned());
          to.2.set(0);
          for flag in &flags {
            flag.set(false);
          }
          for flag in &dismissals {
            flag.set(true);
          }
        }),
      },
    )
  }
}

impl Component for DialogPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      open: ctx.signal(false),
      title: ctx.signal("Delete workspace?".to_owned()),
      description: ctx.signal("It cannot be undone.".to_owned()),
      size: ctx.signal(0),
      scrollable: ctx.signal(false),
      close_button: ctx.signal(false),
      dismiss_on_veil: ctx.signal(true),
      dismiss_on_escape: ctx.signal(true),
      delete_open: ctx.signal(false),
      terms_open: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let asks = Row::new()
      .child(opener("Delete workspace…", &self.delete_open))
      .child(delete_workspace(&self.delete_open));
    let drawn = items()
      .child(item(delete_workspace(&Signal::new(false)).inline(true), "dialog"))
      .child(item(
        terms(&Signal::new(false))
          .inline(true)
          .style(lurq::ui::DialogPart::Panel, |s| s.height(300.0)),
        "dialog · scrolling body",
      ));
    let scrolls = Row::new()
      .child(opener("Terms of use…", &self.terms_open))
      .child(terms(&self.terms_open));
    let rows = [
      (
        "open",
        "Signal<bool>",
        "",
        "Whether it is shown. The app's: Escape, a press on the veil and the close button set it to false.",
      ),
      (
        "title",
        "&str",
        "",
        "What it asks or says, 16/600. It is what the dialog is called.",
      ),
      (
        "description",
        "&str",
        "",
        "More of it under the title, 14 in the muted color.",
      ),
      (
        "child",
        "impl Into<Element>",
        "",
        "Adds an element to the body: a form, a list, paragraphs.",
      ),
      (
        "footer",
        "impl Into<Element>",
        "",
        "Adds an element to the row of buttons at its end: the least action first, the main one last.",
      ),
      (
        "size",
        "DialogSize",
        "420 px",
        "Sm 360, Md 480, Lg 640, Full the window. Never wider than the window.",
      ),
      ("width", "f32", "", "A width of its own, over its size's."),
      (
        "scrollable",
        "bool",
        "false",
        "A header of 56 and a footer of 60, each set off by a line, with the body between them. Any body scrolls where the window is too low.",
      ),
      (
        "close_button",
        "bool",
        "false",
        "A ghost button with the close icon at the end of the header.",
      ),
      (
        "dismiss_on_veil",
        "bool",
        "true",
        "Whether a press on the veil closes it.",
      ),
      ("dismiss_on_escape", "bool", "true", "Whether Escape closes it."),
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
        breadcrumb: &["Components", "Surfaces", "Dialog"],
        title: "Dialog",
        description: "A panel over the window that asks one thing and waits for the answer: delete a workspace, \
                      accept the terms, name a new file.",
        import: Some("use lurq::ui::Dialog;"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Dialog is a control on the right. The button on the stage opens the dialog they make; its \
           buttons, Escape and a press on the veil close it.",
          playground,
        ),
        Section::new(
          "Dialog",
          "The page holds whether it is open, in a signal. As it opens the focus goes to its first button, and \
           back to the button that opened it as it closes; while it is open nothing under the veil takes the \
           pointer, and Tab goes round the dialog's buttons.",
          self.frame.demo(
            0,
            asks,
            "Dialog::new(open.clone()).title(..).footer(..)",
            &[
              "// open: Signal<bool>",
              "let (cancel, delete) = (open.clone(), open.clone());",
              "Dialog::new(open.clone())",
              "    .title(\"Delete workspace?\")",
              "    .description(\"This removes every window layout and setting stored for this workspace. It \
               cannot be undone.\")",
              "    .footer(Button::new(\"Cancel\").variant(ButtonVariant::Outline).on_click(move |_| \
               cancel.set(false)))",
              "    .footer(Button::new(\"Delete\").variant(ButtonVariant::Destructive).on_click(move |_| \
               delete.set(false)))",
            ],
          ),
        ),
        Section::new(
          "As the board draws them",
          "The panel alone, without the veil, where it is declared: what a gallery and a screenshot show.",
          self.frame.demo(
            1,
            drawn,
            ".inline(true)",
            &[
              "Dialog::new(open.clone())",
              "    .title(\"Delete workspace?\")",
              "    .description(\"..\")",
              "    .footer(..)",
              "    .inline(true)",
              "",
              "Dialog::new(open.clone())",
              "    .scrollable(true)",
              "    .close_button(true)",
              "    .title(\"Terms of use\")",
              "    .children(paragraphs)",
              "    .footer(..)",
              "    .style(DialogPart::Panel, |s| s.height(300.0))",
              "    .inline(true)",
            ],
          ),
        ),
        Section::new(
          "Scrolling body",
          "A header of 56 with the close button and a footer of 60, each set off by a line; the body scrolls \
           between them where the window is too low for all of it.",
          self.frame.demo(
            2,
            scrolls,
            ".scrollable(true).close_button(true)",
            &[
              "// open: Signal<bool>",
              "Dialog::new(open.clone())",
              "    .scrollable(true)",
              "    .close_button(true)",
              "    .title(\"Terms of use\")",
              "    .children(paragraphs)",
              "    .footer(Button::new(\"Decline\").variant(ButtonVariant::Outline))",
              "    .footer(Button::new(\"Accept\"))",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a dialog is given.",
          props_table(&rows, &["open"], self.frame.room()),
        ),
      ],
    )
  }
}
