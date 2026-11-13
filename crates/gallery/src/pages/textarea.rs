//! The page of the textarea: its playground, the textarea bound to a signal,
//! its states, its sizes, the rows it shows, and a field that holds it.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{Field, Interaction, TEXTAREA_MAX_ROWS, TEXTAREA_MIN_ROWS, Textarea, TextareaSize},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet, segmented_fill},
  playground::{Playground, Spec, divider, field, flag, prop},
};

/// How wide the board draws a textarea.
const WIDTH: f32 = 260.0;

/// The rows a textarea can be told to show at least, and at most: the selects of the playground.
const LEAST: [(&str, usize); 5] = [("1", 1), ("2", 2), ("3", 3), ("4", 4), ("5", 5)];
const MOST: [(&str, usize); 5] = [("3", 3), ("4", 4), ("6", 6), ("8", 8), ("12", 12)];

/// The sizes, as the board names an input's, how the code names them, and the type of each.
const SIZES: [(TextareaSize, &str, &str, f32); 3] = [
  (TextareaSize::Sm, "sm", "TextareaSize::Sm", 13.0),
  (TextareaSize::Md, "md", "TextareaSize::Md", 14.0),
  (TextareaSize::Lg, "lg", "TextareaSize::Lg", 15.0),
];

/// A textarea as wide as the board draws one, with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .width(WIDTH)
    .spacing(9.0)
    .child(shown)
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

pub struct TextareaPage {
  frame: Frame,
  playground: Playground,
  value: Signal<String>,
  placeholder: Signal<String>,
  label: Signal<String>,
  size: Signal<usize>,
  min_rows: Signal<usize>,
  max_rows: Signal<usize>,
  invalid: Signal<bool>,
  warning: Signal<bool>,
  valid: Signal<bool>,
  read_only: Signal<bool>,
  disabled: Signal<bool>,
  bio: Signal<String>,
  report: Signal<String>,
  story: Signal<String>,
}

impl TextareaPage {
  /// The playground: the textarea as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let placeholder = self.placeholder.get();
    let name = self.label.get();
    let (size, size_name, size_code, font_size) = SIZES[self.size.get().min(SIZES.len() - 1)];
    let (min_rows, max_rows) = (self.min_rows.get(), self.max_rows.get());
    let flags = [
      ("invalid", self.invalid.get()),
      ("warning", self.warning.get()),
      ("valid", self.valid.get()),
      ("read_only", self.read_only.get()),
      ("disabled", self.disabled.get()),
    ];
    let value = self.value.clone();
    let plain = {
      let (placeholder, value, name) = (placeholder.clone(), value.clone(), name.clone());
      let set: Vec<bool> = flags.iter().map(|(_, set)| *set).collect();
      move || {
        let mut textarea = Textarea::new(value.clone())
          .placeholder(&placeholder)
          .size(size)
          .rows(min_rows, max_rows)
          .invalid(set[0])
          .warning(set[1])
          .valid(set[2])
          .read_only(set[3])
          .disabled(set[4]);
        if !name.is_empty() {
          textarea = textarea.label(&name);
        }
        textarea
      }
    };
    let record = self.playground.recorder();
    let textarea = plain()
      .force(self.playground.forced())
      .on_change(move |text| record(format!("on_change({} characters)", text.chars().count())));

    let mut code = vec![
      "// value: Signal<String>".to_owned(),
      "Textarea::new(value.clone())".to_owned(),
      format!("    .placeholder({placeholder:?})"),
    ];
    if !name.is_empty() {
      code.push(format!("    .label({name:?})"));
    }
    if size != TextareaSize::Md {
      code.push(format!("    .size({size_code})"));
    }
    if (min_rows, max_rows) != (TEXTAREA_MIN_ROWS, TEXTAREA_MAX_ROWS) {
      code.push(format!("    .rows({min_rows}, {max_rows})"));
    }
    for (flag, set) in flags {
      if set {
        code.push(format!("    .{flag}(true)"));
      }
    }

    let held = value.get();
    let lines = held.split('\n').count();
    let shown = if held.is_empty() {
      "the placeholder".to_owned()
    } else {
      format!("{lines} line{}", if lines == 1 { "" } else { "s" })
    };
    let tree = vec![
      format!("root      {}", self.playground.measured()),
      format!("├─ text    {shown}, {font_size} px"),
      format!("└─ rows    {min_rows} at least, {max_rows} at most"),
    ];

    let (to_value, to_placeholder, to_label) = (self.value.clone(), self.placeholder.clone(), self.label.clone());
    let (to_min, to_max, to_size) = (self.min_rows.clone(), self.max_rows.clone(), self.size.clone());
    let flag_signals = [
      self.invalid.clone(),
      self.warning.clone(),
      self.valid.clone(),
      self.read_only.clone(),
      self.disabled.clone(),
    ];
    let (min, max, choose_size) = (self.min_rows.clone(), self.max_rows.clone(), self.size.clone());
    let sizes: Vec<(&str, usize)> = SIZES.iter().enumerate().map(|(index, size)| (size.1, index)).collect();
    self.playground.card(
      ctx,
      Spec {
        title: "Textarea",
        make: Arc::new(move || Column::new().width(360.0).child(plain()).into()),
        shown: Column::new().width(FILL).child(textarea).into(),
        fill: true,
        forceable: true,
        facts: format!("{size_name} · {} characters", held.chars().count()),
        event: "on_change",
        code,
        tree,
        controls: vec![
          prop("placeholder", "&str", field(&self.placeholder)),
          prop("label", "&str", field(&self.label)),
          prop(
            "size",
            "TextareaSize",
            segmented_fill(&sizes, &self.size.get(), move |index| choose_size.set(index)),
          ),
          prop(
            "rows: least",
            "usize",
            self.playground.select(0, &LEAST, &min_rows, move |rows| min.set(rows)),
          ),
          prop(
            "rows: most",
            "usize",
            self.playground.select(1, &MOST, &max_rows, move |rows| max.set(rows)),
          ),
          divider(),
          flag("invalid", &self.invalid),
          flag("warning", &self.warning),
          flag("valid", &self.valid),
          flag("read_only", &self.read_only),
          flag("disabled", &self.disabled),
        ],
        changed: [
          !held.is_empty(),
          placeholder != "Tell us what happened",
          !name.is_empty(),
          size != TextareaSize::Md,
          min_rows != TEXTAREA_MIN_ROWS,
          max_rows != TEXTAREA_MAX_ROWS,
        ]
        .into_iter()
        .chain(flags.iter().map(|(_, set)| *set))
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          to_value.set(String::new());
          to_placeholder.set("Tell us what happened".to_owned());
          to_label.set(String::new());
          to_size.set(1);
          to_min.set(TEXTAREA_MIN_ROWS);
          to_max.set(TEXTAREA_MAX_ROWS);
          for flag in &flag_signals {
            flag.set(false);
          }
        }),
      },
    )
  }
}

impl Component for TextareaPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 2),
      value: ctx.signal(String::new()),
      placeholder: ctx.signal("Tell us what happened".to_owned()),
      label: ctx.signal(String::new()),
      size: ctx.signal(1),
      min_rows: ctx.signal(TEXTAREA_MIN_ROWS),
      max_rows: ctx.signal(TEXTAREA_MAX_ROWS),
      invalid: ctx.signal(false),
      warning: ctx.signal(false),
      valid: ctx.signal(false),
      read_only: ctx.signal(false),
      disabled: ctx.signal(false),
      bio: ctx.signal(String::new()),
      report: ctx.signal(
        "The export stopped at 80%.\nIt said the disk was full, but there were 40 GB free.\nIt happened twice \
         today.\nRestarting the app did not help.\nThe log is attached."
          .to_owned(),
      ),
      story: ctx.signal(String::new()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let bio = self.bio.get();
    let live = items().child(item(
      Textarea::new(self.bio.clone()).placeholder("A few words about you"),
      &format!("the signal has {} characters", bio.chars().count()),
    ));
    let filled = || Signal::new("The export stopped at 80%.".to_owned());
    let states = items()
      .child(item(
        Textarea::new(Signal::new(String::new())).placeholder("Tell us what happened"),
        "default",
      ))
      .child(item(Textarea::new(filled()), "filled"))
      .child(item(
        Textarea::new(Signal::new(String::new()))
          .placeholder("Tell us what happened")
          .force(Some(Interaction::Hovered)),
        "hover",
      ))
      .child(item(
        Textarea::new(filled()).force(Some(Interaction::FocusVisible)),
        "focus",
      ))
      .child(item(Textarea::new(filled()).invalid(true), "invalid"))
      .child(item(Textarea::new(filled()).warning(true), "warning"))
      .child(item(Textarea::new(filled()).valid(true), "valid"))
      .child(item(Textarea::new(filled()).read_only(true), "read only"))
      .child(item(
        Textarea::new(Signal::new(String::new()))
          .placeholder("Tell us what happened")
          .disabled(true),
        "disabled",
      ));
    let mut sizes = items();
    for (size, name, _, font_size) in SIZES {
      sizes = sizes.child(item(
        Textarea::new(Signal::new(String::new()))
          .placeholder("Tell us what happened")
          .size(size),
        &format!("{name} · text {font_size}"),
      ));
    }
    let story = self.story.get();
    let told = story.chars().count();
    let in_field = {
      let field = Field::new(
        "What happened",
        Textarea::new(self.story.clone()).placeholder("The steps, what you expected, what you saw"),
      )
      .required(true)
      .full_width(true);
      // The field's judgment is the textarea's: it is not told.
      if told < 20 {
        field.error(&format!("At least 20 characters: {told} so far."))
      } else {
        field.description(&format!("{told} characters.")).valid(true)
      }
    };
    let in_field = items().child(Column::new().width(360.0).child(in_field));
    let rows = items()
      .child(item(
        Textarea::new(Signal::new(String::new()))
          .placeholder("Two rows, then it grows")
          .rows(2, 4),
        "rows(2, 4): empty",
      ))
      .child(item(
        Textarea::new(self.report.clone()).rows(2, 4),
        "rows(2, 4): five lines, the last scroll",
      ));
    let props = [
      (
        "value",
        "Signal<String>",
        "",
        "What it holds. What is typed is written into it, and what the app writes there shows.",
      ),
      (
        "placeholder",
        "&str",
        "",
        "What it shows while it holds nothing, in the muted foreground.",
      ),
      (
        "rows",
        "(usize, usize)",
        "3, 8",
        "The rows it shows at least, and at most before its text scrolls inside it.",
      ),
      (
        "size",
        "TextareaSize",
        "Md",
        "The type of an input's sizes, 13, 14 and 15, and rows of that type.",
      ),
      (
        "invalid",
        "bool",
        "false",
        "What it holds is wrong: the edge and the ring are the destructive color.",
      ),
      (
        "warning",
        "bool",
        "false",
        "What it holds is allowed with a caveat: the warning color.",
      ),
      (
        "valid",
        "bool",
        "false",
        "What it holds was checked and is right: the success color.",
      ),
      (
        "read_only",
        "bool",
        "false",
        "It can be focused, selected and copied, and not changed; on the muted fill.",
      ),
      (
        "disabled",
        "bool",
        "false",
        "Half the opacity on the muted fill; it takes no input and Tab passes it by.",
      ),
      (
        "on_change",
        "impl Fn(&str)",
        "",
        "Called on every edit with the text it holds once the edit applies.",
      ),
      (
        "label",
        "&str",
        "",
        "What it is called, to what reads the tree. It draws nothing: a field gives it its own label.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Inputs", "Textarea"],
        title: "Textarea",
        description: "Text of more than one line, typed into a box: a bio, a note, what happened. It grows with its \
                      text, and past the rows it shows at most its text scrolls inside it.",
        import: Some("use lurq::ui::Textarea;"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Textarea is a control on the right. The textarea on the stage is live: type into it, and \
           Enter starts a new line.",
          playground,
        ),
        Section::new(
          "Textarea",
          "The page holds the text in a signal. Tab reaches the box, and what is typed goes into the signal.",
          self.frame.demo(
            0,
            live,
            "Textarea::new(bio.clone()).placeholder(..)",
            &[
              "// bio: Signal<String>",
              "Textarea::new(bio.clone())",
              "    .placeholder(\"A few words about you\")",
            ],
          ),
        ),
        Section::new(
          "States",
          "The edge is the only sign of a state: the ring's color under the pointer, a ring of 2 inside the box \
           while it is focused, the destructive, warning and success colors for what it holds. Read only is on the \
           muted fill in the color of a border, and refuses every edit; a disabled one is dimmed on the muted fill.",
          self.frame.demo(
            1,
            states,
            ".invalid(true)",
            &[
              "Textarea::new(value.clone()).placeholder(\"Tell us what happened\")",
              "Textarea::new(filled.clone())",
              "",
              "// Drawn under the pointer, and focused, with nothing pointing at it: for a gallery, a test.",
              "Textarea::new(value.clone()).force(Some(Interaction::Hovered))",
              "Textarea::new(filled.clone()).force(Some(Interaction::FocusVisible))",
              "",
              "Textarea::new(filled.clone()).invalid(true)",
              "Textarea::new(filled.clone()).warning(true)",
              "Textarea::new(filled.clone()).valid(true)",
              "Textarea::new(filled.clone()).read_only(true)",
              "Textarea::new(value.clone()).disabled(true)",
            ],
          ),
        ),
        Section::new(
          "Sizes",
          "The type of an input's sizes, 13, 14 and 15, with a line of 1.5: a row is that high. The padding stays.",
          self.frame.demo(
            3,
            sizes,
            ".size(TextareaSize::Sm)",
            &[
              "Textarea::new(value.clone()).size(TextareaSize::Sm)",
              "Textarea::new(value.clone())",
              "Textarea::new(value.clone()).size(TextareaSize::Lg)",
            ],
          ),
        ),
        Section::new(
          "In a field",
          "A field calls the textarea by its label and gives it its judgment: here the error makes it invalid until \
           it holds 20 characters. A press of the label focuses it.",
          self.frame.demo(
            4,
            in_field,
            "Field::new(\"What happened\", textarea).error(..)",
            &[
              "// story: Signal<String>",
              "let told = story.get().chars().count();",
              "let field = Field::new(",
              "    \"What happened\",",
              "    Textarea::new(story.clone()).placeholder(\"The steps, what you expected, what you saw\"),",
              ")",
              ".required(true)",
              ".full_width(true);",
              "if told < 20 {",
              "    field.error(&format!(\"At least 20 characters: {told} so far.\"))",
              "} else {",
              "    field.description(&format!(\"{told} characters.\")).valid(true)",
              "}",
            ],
          ),
        ),
        Section::new(
          "Rows",
          "A textarea is as high as its text, from the rows it shows at least to the rows it shows at most. Past \
           those its text scrolls inside it, with a scroll bar of its own.",
          self.frame.demo(
            2,
            rows,
            ".rows(2, 4)",
            &[
              "Textarea::new(value.clone())",
              "    .placeholder(\"Two rows, then it grows\")",
              "    .rows(2, 4)",
              "",
              "// report: Signal<String>, five lines",
              "Textarea::new(report.clone()).rows(2, 4)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a textarea is given.",
          props_table(&props, &["value"], self.frame.room()),
        ),
      ],
    )
  }
}
