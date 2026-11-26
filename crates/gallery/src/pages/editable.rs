//! The page of the editable: its playground, a title edited in place, its two forms as the board draws them, a
//! list of names each edited where it stands, and its states.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{Editable, Kbd},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .max_width(FILL)
    .spacing(9.0)
    .child(shown)
    .child(quiet(caption, 12.0))
}

/// What is on a stage, side by side; what does not fit goes on to the next line.
fn items() -> Row {
  Row::new().width(FILL).spacing(32.0).wrap().align_items(Alignment::End)
}

pub struct EditablePage {
  frame: Frame,
  playground: Playground,
  value: Signal<String>,
  placeholder: Signal<String>,
  label: Signal<String>,
  actions: Signal<bool>,
  submit_on_blur: Signal<bool>,
  invalid: Signal<bool>,
  read_only: Signal<bool>,
  disabled: Signal<bool>,
  full_width: Signal<bool>,
  title: Signal<String>,
  /// The board's two forms: one read, one being edited.
  read: Signal<String>,
  edited: Signal<String>,
  editing: Signal<bool>,
  /// The names of a list, each edited where it stands.
  files: [Signal<String>; 3],
  empty: Signal<String>,
  fixed: Signal<String>,
  /// The name of a page, edited with Enter's key cap at the end of its field.
  page: Signal<String>,
}

impl EditablePage {
  /// The playground: the editable as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (placeholder, name) = (self.placeholder.get(), self.label.get());
    let (actions, submit_on_blur) = (self.actions.get(), self.submit_on_blur.get());
    let flags = [
      ("invalid", self.invalid.get()),
      ("read_only", self.read_only.get()),
      ("disabled", self.disabled.get()),
      ("full_width", self.full_width.get()),
    ];
    let full_width = self.full_width.get();
    let plain = {
      let (value, placeholder, name) = (self.value.clone(), placeholder.clone(), name.clone());
      let set: Vec<bool> = flags.iter().map(|(_, set)| *set).collect();
      move || {
        let mut editable = Editable::new(value.clone())
          .placeholder(&placeholder)
          .actions(actions)
          .submit_on_blur(submit_on_blur)
          .invalid(set[0])
          .read_only(set[1])
          .disabled(set[2])
          .full_width(set[3]);
        if !name.is_empty() {
          editable = editable.label(&name);
        }
        editable
      }
    };
    let (changed, cancelled, turned) = (
      self.playground.recorder(),
      self.playground.recorder(),
      self.playground.recorder(),
    );
    let shown = self
      .playground
      .styled(plain())
      .force(self.playground.forced())
      .on_change(move |text| changed(format!("on_change({text:?})")))
      .on_cancel(move || cancelled("on_cancel()".to_owned()))
      .on_editing_change(move |editing| turned(format!("on_editing_change({editing})")));
    {
      let plain = plain.clone();
      self.playground.window_shows(move |ctx| plain().mount(ctx));
    }

    let mut code = vec![
      "// value: Signal<String>".to_owned(),
      "Editable::new(value.clone())".to_owned(),
      format!("    .placeholder({placeholder:?})"),
    ];
    if !name.is_empty() {
      code.push(format!("    .label({name:?})"));
    }
    if !actions {
      code.push("    .actions(false)".to_owned());
    }
    if !submit_on_blur {
      code.push("    .submit_on_blur(false)".to_owned());
    }
    for (flag, set) in flags {
      if set {
        code.push(format!("    .{flag}(true)"));
      }
    }
    code.push("    .mount(ctx)".to_owned());

    let held = self.value.get();
    let tree = vec![
      format!("root        {} · 32 high · gap 8", self.playground.measured()),
      "├─ label     16 semibold · foreground".to_owned(),
      "├─ icon      pencil · 14 · muted foreground".to_owned(),
      "└─ editing   field 160 × 32 · gap 6 · Save (outline) and Cancel (ghost), 32".to_owned(),
    ];

    let (to_value, to_placeholder, to_label) = (self.value.clone(), self.placeholder.clone(), self.label.clone());
    let on = [self.actions.clone(), self.submit_on_blur.clone()];
    let off = [
      self.invalid.clone(),
      self.read_only.clone(),
      self.disabled.clone(),
      self.full_width.clone(),
    ];
    let make = {
      let plain = plain.clone();
      move || plain().into()
    };
    self.playground.card(
      ctx,
      Spec {
        title: "Editable",
        make: Arc::new(make),
        shown: Shown::built(move |ctx| shown.mount(ctx)),
        fill: full_width,
        forceable: true,
        facts: format!("{} characters", held.chars().count()),
        event: "on_change",
        code,
        tree,
        controls: vec![
          prop("value", "Signal<String>", field(&self.value)),
          prop("placeholder", "&str", field(&self.placeholder)),
          prop("label", "&str", field(&self.label)),
          divider(),
          flag("actions", &self.actions),
          flag("submit_on_blur", &self.submit_on_blur),
          divider(),
          flag("invalid", &self.invalid),
          flag("read_only", &self.read_only),
          flag("disabled", &self.disabled),
          flag("full_width", &self.full_width),
        ],
        changed: [
          held != "Project Alpha",
          placeholder != "Untitled",
          !name.is_empty(),
          !actions,
          !submit_on_blur,
        ]
        .into_iter()
        .chain(flags.iter().map(|(_, set)| *set))
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          to_value.set("Project Alpha".to_owned());
          to_placeholder.set("Untitled".to_owned());
          to_label.set(String::new());
          for flag in &on {
            flag.set(true);
          }
          for flag in &off {
            flag.set(false);
          }
        }),
      },
    )
  }
}

impl Component for EditablePage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      value: ctx.signal("Project Alpha".to_owned()),
      placeholder: ctx.signal("Untitled".to_owned()),
      label: ctx.signal(String::new()),
      actions: ctx.signal(true),
      submit_on_blur: ctx.signal(true),
      invalid: ctx.signal(false),
      read_only: ctx.signal(false),
      disabled: ctx.signal(false),
      full_width: ctx.signal(false),
      title: ctx.signal("Quarterly report".to_owned()),
      read: ctx.signal("Project Alpha".to_owned()),
      edited: ctx.signal("Project Alpha".to_owned()),
      editing: ctx.signal(true),
      files: [
        ctx.signal("README.md".to_owned()),
        ctx.signal("design-system.pen".to_owned()),
        ctx.signal("release-notes-0.63.md".to_owned()),
      ],
      empty: ctx.signal(String::new()),
      fixed: ctx.signal("Project Alpha".to_owned()),
      page: ctx.signal("Untitled".to_owned()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let title = self.title.get();
    let live = items().child(item(
      Editable::new(self.title.clone())
        .label("Report title")
        .placeholder("Untitled report")
        .key("title")
        .mount(ctx),
      &format!("the signal holds {title:?}"),
    ));
    let forms = items()
      .child(item(
        Editable::new(self.read.clone()).key("read").mount(ctx),
        "editable",
      ))
      .child(item(
        // The app keeps whether it is edited: this one starts being edited, as the board draws it.
        Editable::new(self.edited.clone())
          .editing(self.editing.clone())
          .key("edited")
          .mount(ctx),
        if self.editing.get() {
          "editable editing"
        } else {
          "press it to edit again"
        },
      ));
    let mut list = Column::new().width(320.0).max_width(FILL).spacing(4.0);
    for (index, file) in self.files.iter().enumerate() {
      list = list.child(
        Editable::new(file.clone())
          .actions(false)
          .full_width(true)
          .label("File name")
          .key(format!("file-{index}"))
          .mount(ctx),
      );
    }
    let names = items().child(item(
      list,
      "Enter keeps a name, Escape drops the edit, Tab goes on to the next",
    ));
    let named = items().child(item(
      Column::new().width(280.0).max_width(FILL).child(
        Editable::new(self.page.clone())
          .actions(false)
          .full_width(true)
          .label("Page name")
          .end(Kbd::new("Enter"))
          .key("page")
          .mount(ctx),
      ),
      "press it: Enter's key cap stands at the field's end",
    ));
    let states = items()
      .child(item(
        Editable::new(self.empty.clone())
          .placeholder("Untitled")
          .key("empty")
          .mount(ctx),
        "empty: its placeholder",
      ))
      .child(item(
        Editable::new(self.fixed.clone())
          .read_only(true)
          .key("read-only")
          .mount(ctx),
        "read only",
      ))
      .child(item(
        Editable::new(self.fixed.clone())
          .disabled(true)
          .key("disabled")
          .mount(ctx),
        "disabled",
      ));
    let props = [
      (
        "value",
        "Signal<String>",
        "",
        "The text. An edit that is kept is written into it. `controlled(value, on_change)` for a text the app \
         holds.",
      ),
      (
        "on_change",
        "impl Fn(&str)",
        "",
        "Called with the new text when an edit that changed it is kept: by Enter, the button, or leaving the field.",
      ),
      (
        "on_cancel",
        "impl Fn()",
        "",
        "Called when an edit is dropped: by Escape or the button.",
      ),
      (
        "editing",
        "Signal<bool>",
        "",
        "Whether it is being edited, in the app's signal: the app starts an edit by setting it. \
         `default_editing(true)` for one that keeps this itself and starts edited.",
      ),
      (
        "on_editing_change",
        "impl Fn(bool)",
        "",
        "Called when a person starts an edit and when one ends, after `on_change` or `on_cancel`.",
      ),
      (
        "placeholder",
        "&str",
        "",
        "What the reading form shows while the text is empty, and the field while nothing is typed.",
      ),
      (
        "actions",
        "bool",
        "true",
        "Whether the editing form has the buttons that keep the edit and drop it.",
      ),
      (
        "submit_on_blur",
        "bool",
        "true",
        "Whether leaving the field keeps what was typed. Without it, leaving drops it.",
      ),
      (
        "invalid",
        "bool",
        "false",
        "What is typed is wrong: the field's edge says so.",
      ),
      (
        "read_only",
        "bool",
        "false",
        "The text is read and cannot be edited: no pencil.",
      ),
      ("disabled", "bool", "false", "Half the opacity; it takes no input."),
      (
        "full_width",
        "bool",
        "false",
        "As wide as what holds it: its text is cut where it is longer, its field fills the room.",
      ),
      (
        "label",
        "&str",
        "",
        "What the text is, to what reads the tree. Its buttons are `confirm_label` and `cancel_label`.",
      ),
      (
        "end",
        "impl Into<Element>",
        "none",
        "What stands at the end of the field while it is edited: a key cap. The text keeps clear of the part \
         `end`'s width, 40.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Inputs", "Editable"],
        title: "Editable",
        description: "A text that is read where it stands and edited in place: a title, a name, a cell. A press, \
                      Enter or F2 puts a field where the text was; Enter keeps what was typed, and so does leaving \
                      the field; Escape drops it.",
        import: Some("use lurq::ui::Editable;"),
        status: Some("New"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Editable is a control on the right. The editable on the stage is live: press it, or Tab \
           to it and press Enter or F2, type, and press Enter or Escape.",
          playground,
        ),
        Section::new(
          "Editable",
          "The page holds the title in a signal. It is set when an edit is kept, not while it is typed.",
          self.frame.demo(
            0,
            live,
            "Editable::new(title.clone()).mount(ctx)",
            &[
              "// title: Signal<String>",
              "Editable::new(title.clone())",
              "    .label(\"Report title\")",
              "    .placeholder(\"Untitled report\")",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Two forms",
          "Read, it is its text and a pencil. Edited, a field with the text selected and two buttons: the outline \
           one keeps the edit, the ghost one drops it. When an edit ends by a key or a button the focus is back on \
           the reading form.",
          self.frame.demo(
            1,
            forms,
            ".editing(editing.clone())",
            &[
              "Editable::new(name.clone()).mount(ctx)",
              "",
              "// editing: Signal<bool>, the app's: it starts an edit by setting it.",
              "Editable::new(name.clone()).editing(editing.clone()).mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Without its buttons",
          "A list of names, each edited where it stands, with Enter and Escape alone. Full width, a name is cut \
           where it is longer than its row.",
          self.frame.demo(
            2,
            names,
            ".actions(false).full_width(true)",
            &[
              "for file in &files {",
              "    list = list.child(",
              "        Editable::new(file.clone())",
              "            .actions(false)",
              "            .full_width(true)",
              "            .label(\"File name\")",
              "            .mount(ctx),",
              "    );",
              "}",
            ],
          ),
        ),
        Section::new(
          "Something at the field's end",
          "What the app puts at the end of the field while it is edited stands over the end of its box, as a \
           password's eye does, and the text keeps clear of it: a key cap that says Enter keeps the name.",
          self.frame.demo(
            4,
            named,
            ".end(Kbd::new(\"Enter\"))",
            &[
              "Editable::new(name.clone())",
              "    .actions(false)",
              "    .full_width(true)",
              "    .end(Kbd::new(\"Enter\"))",
              "    .mount(ctx)",
              "",
              "// Wider than 40, its part is styled wider: the room the text keeps clear.",
              ".style(EditablePart::End, |s| s.width(64.0))",
            ],
          ),
        ),
        Section::new(
          "States",
          "Empty, the reading form shows the placeholder in the muted foreground. Read only, it is its text alone \
           and takes no press. Disabled, it is dimmed.",
          self.frame.demo(
            3,
            states,
            ".read_only(true)",
            &[
              "Editable::new(empty.clone()).placeholder(\"Untitled\").mount(ctx)",
              "Editable::new(name.clone()).read_only(true).mount(ctx)",
              "Editable::new(name.clone()).disabled(true).mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What an editable is given.",
          props_table(&props, &["value"], self.frame.room()),
        ),
      ],
    )
  }
}
