//! The page of the textarea: its playground, the textarea bound to a signal,
//! its states, and the rows it shows.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{Interaction, TEXTAREA_MAX_ROWS, TEXTAREA_MIN_ROWS, Textarea},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Spec, divider, field, flag, prop},
};

/// How wide the board draws a textarea.
const WIDTH: f32 = 260.0;

/// The rows a textarea can be told to show at least, and at most: the selects of the playground.
const LEAST: [(&str, usize); 5] = [("1", 1), ("2", 2), ("3", 3), ("4", 4), ("5", 5)];
const MOST: [(&str, usize); 5] = [("3", 3), ("4", 4), ("6", 6), ("8", 8), ("12", 12)];

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
  min_rows: Signal<usize>,
  max_rows: Signal<usize>,
  invalid: Signal<bool>,
  disabled: Signal<bool>,
  bio: Signal<String>,
  report: Signal<String>,
}

impl TextareaPage {
  /// The playground: the textarea as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let placeholder = self.placeholder.get();
    let (min_rows, max_rows) = (self.min_rows.get(), self.max_rows.get());
    let (invalid, disabled) = (self.invalid.get(), self.disabled.get());
    let value = self.value.clone();
    let plain = {
      let (placeholder, value) = (placeholder.clone(), value.clone());
      move || {
        Textarea::new(value.clone())
          .placeholder(&placeholder)
          .rows(min_rows, max_rows)
          .invalid(invalid)
          .disabled(disabled)
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
    if (min_rows, max_rows) != (TEXTAREA_MIN_ROWS, TEXTAREA_MAX_ROWS) {
      code.push(format!("    .rows({min_rows}, {max_rows})"));
    }
    if invalid {
      code.push("    .invalid(true)".to_owned());
    }
    if disabled {
      code.push("    .disabled(true)".to_owned());
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
      format!("├─ text    {shown}"),
      format!("└─ rows    {min_rows} at least, {max_rows} at most"),
    ];

    let (to_value, to_placeholder) = (self.value.clone(), self.placeholder.clone());
    let (to_min, to_max) = (self.min_rows.clone(), self.max_rows.clone());
    let flags = [self.invalid.clone(), self.disabled.clone()];
    let (min, max) = (self.min_rows.clone(), self.max_rows.clone());
    self.playground.card(
      ctx,
      Spec {
        title: "Textarea",
        make: Arc::new(move || Column::new().width(360.0).child(plain()).into()),
        shown: Column::new().width(FILL).child(textarea).into(),
        fill: true,
        forceable: true,
        facts: format!("{} characters", held.chars().count()),
        event: "on_change",
        code,
        tree,
        controls: vec![
          prop("placeholder", "&str", field(&self.placeholder)),
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
          flag("disabled", &self.disabled),
        ],
        changed: [
          !held.is_empty(),
          placeholder != "Tell us what happened",
          min_rows != TEXTAREA_MIN_ROWS,
          max_rows != TEXTAREA_MAX_ROWS,
          invalid,
          disabled,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          to_value.set(String::new());
          to_placeholder.set("Tell us what happened".to_owned());
          to_min.set(TEXTAREA_MIN_ROWS);
          to_max.set(TEXTAREA_MAX_ROWS);
          for flag in &flags {
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
      min_rows: ctx.signal(TEXTAREA_MIN_ROWS),
      max_rows: ctx.signal(TEXTAREA_MAX_ROWS),
      invalid: ctx.signal(false),
      disabled: ctx.signal(false),
      bio: ctx.signal(String::new()),
      report: ctx.signal(
        "The export stopped at 80%.\nIt said the disk was full, but there were 40 GB free.\nIt happened twice \
         today.\nRestarting the app did not help.\nThe log is attached."
          .to_owned(),
      ),
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
      .child(item(
        Textarea::new(Signal::new(String::new()))
          .placeholder("Tell us what happened")
          .disabled(true),
        "disabled",
      ));
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
        "invalid",
        "bool",
        "false",
        "What it holds is not accepted: the edge and the ring are the destructive color.",
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
        "What it is called, to what reads the tree. It draws nothing: the field above it does.",
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
           while it is focused, the destructive color while what it holds is not accepted. A disabled one is \
           dimmed on the muted fill.",
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
              "Textarea::new(value.clone()).disabled(true)",
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
