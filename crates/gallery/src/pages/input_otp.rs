//! The page of the one-time code: its playground, a code bound to a signal that says when it is complete, its
//! states, its sizes, a code of letters and digits, and a code in a field.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{Field, InputOtp, InputOtpKind, InputOtpSize, Interaction},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet, segmented_fill},
  playground::{Playground, Spec, divider, field, flag, prop},
};

/// The lengths the playground offers.
const LENGTHS: [usize; 3] = [4, 6, 8];

/// How the playground groups the cells: what the control says, and the size of each group.
const GROUPINGS: [(&str, usize); 3] = [("none", 0), ("halves", 2), ("pairs", 1)];

/// The sizes, as the board names them, how the code names them, and the side of a cell.
const SIZES: [(InputOtpSize, &str, &str, &str); 3] = [
  (InputOtpSize::Sm, "sm", "InputOtpSize::Sm", "32 × 36"),
  (InputOtpSize::Md, "md", "InputOtpSize::Md", "36 × 40"),
  (InputOtpSize::Lg, "lg", "InputOtpSize::Lg", "40 × 44"),
];

/// The groups of a code of `length` cells, grouped as `grouping` says: none, two halves, or pairs.
fn groups(length: usize, grouping: usize) -> Vec<usize> {
  match GROUPINGS[grouping.min(GROUPINGS.len() - 1)].1 {
    0 => Vec::new(),
    2 => vec![length / 2, length - length / 2],
    _ => vec![2; length / 2],
  }
}

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .spacing(9.0)
    .child(Row::new().overflow_visible().child(shown))
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

/// A code of six in two groups that holds `value` and is not typed in: for a stage that shows a state.
fn showing(value: &str) -> InputOtp {
  InputOtp::new(Signal::new(value.to_owned())).groups([3, 3])
}

pub struct InputOtpPage {
  frame: Frame,
  playground: Playground,
  value: Signal<String>,
  length: Signal<usize>,
  grouping: Signal<usize>,
  letters: Signal<bool>,
  uppercase: Signal<bool>,
  mask: Signal<bool>,
  size: Signal<usize>,
  invalid: Signal<bool>,
  disabled: Signal<bool>,
  code: Signal<String>,
  verified: Signal<String>,
  recovery: Signal<String>,
  pin: Signal<String>,
}

impl InputOtpPage {
  /// The playground: the code as the panel's props make it, bound to the page's signal.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let length = LENGTHS[self.length.get().min(LENGTHS.len() - 1)];
    let grouping = self.grouping.get().min(GROUPINGS.len() - 1);
    let grouped = groups(length, grouping);
    let letters = self.letters.get();
    let uppercase = letters && self.uppercase.get();
    let (size, size_name, size_code, side) = SIZES[self.size.get().min(SIZES.len() - 1)];
    let (mask, invalid, disabled) = (self.mask.get(), self.invalid.get(), self.disabled.get());
    let make = {
      let (value, grouped) = (self.value.clone(), grouped.clone());
      move || {
        InputOtp::new(value.clone())
          .length(length)
          .groups(grouped.clone())
          .kind(if letters {
            InputOtpKind::Alphanumeric
          } else {
            InputOtpKind::Digits
          })
          .uppercase(uppercase)
          .mask(mask)
          .size(size)
          .invalid(invalid)
          .disabled(disabled)
          .label("Verification code")
      }
    };
    let record = self.playground.recorder();
    let shown = make()
      .force(self.playground.forced())
      .on_complete(move |code| record(format!("on_complete({code:?})")));

    let mut code = vec![
      "// value: Signal<String>".to_owned(),
      "InputOtp::new(value.clone())".to_owned(),
    ];
    if length != 6 {
      code.push(format!("    .length({length})"));
    }
    if !grouped.is_empty() {
      code.push(format!("    .groups({grouped:?})"));
    }
    if letters {
      code.push("    .kind(InputOtpKind::Alphanumeric)".to_owned());
    }
    if uppercase {
      code.push("    .uppercase(true)".to_owned());
    }
    if size != InputOtpSize::Md {
      code.push(format!("    .size({size_code})"));
    }
    for (name, set) in [("mask", mask), ("invalid", invalid), ("disabled", disabled)] {
      if set {
        code.push(format!("    .{name}(true)"));
      }
    }
    code.push("    .label(\"Verification code\")".to_owned());
    code.push("    .on_complete(|code| verify(code))".to_owned());

    let separators = grouped.len().saturating_sub(1);
    let mut tree = vec![
      format!("root        {}", self.playground.measured()),
      "├─ input     the code, one textbox under the cells".to_owned(),
      format!(
        "{} cell × {length}   {side} px   {:?}",
        if separators > 0 { "├─" } else { "└─" },
        self.value.get()
      ),
    ];
    if separators > 0 {
      tree.push(format!("└─ separator × {separators}   16 × 16 px   minus"));
    }

    let lengths: Vec<(&str, usize)> = vec![("4", 0), ("6", 1), ("8", 2)];
    let groupings: Vec<(&str, usize)> = GROUPINGS
      .iter()
      .enumerate()
      .map(|(index, grouping)| (grouping.0, index))
      .collect();
    let kinds: Vec<(&str, bool)> = vec![("digits", false), ("letters", true)];
    let sizes: Vec<(&str, usize)> = SIZES.iter().enumerate().map(|(index, size)| (size.1, index)).collect();
    let (to_length, to_grouping, to_letters, to_size) = (
      self.length.clone(),
      self.grouping.clone(),
      self.letters.clone(),
      self.size.clone(),
    );
    let reset = {
      let (value, length, grouping, letters, size) = (
        self.value.clone(),
        self.length.clone(),
        self.grouping.clone(),
        self.letters.clone(),
        self.size.clone(),
      );
      let flags = [
        self.uppercase.clone(),
        self.mask.clone(),
        self.invalid.clone(),
        self.disabled.clone(),
      ];
      move || {
        value.set(String::new());
        length.set(1);
        grouping.set(1);
        letters.set(false);
        size.set(1);
        for flag in &flags {
          flag.set(false);
        }
      }
    };
    let changed = usize::from(length != 6)
      + usize::from(grouping != 1)
      + usize::from(letters)
      + usize::from(uppercase)
      + usize::from(size != InputOtpSize::Md)
      + usize::from(mask)
      + usize::from(invalid)
      + usize::from(disabled)
      + usize::from(!self.value.get().is_empty());

    let mut controls = vec![
      prop("value", "Signal<String>", field(&self.value)),
      prop(
        "length",
        "usize",
        segmented_fill(&lengths, &self.length.get(), move |index| to_length.set(index)),
      ),
      prop(
        "groups",
        "impl IntoIterator<Item = usize>",
        segmented_fill(&groupings, &grouping, move |index| to_grouping.set(index)),
      ),
      prop(
        "kind",
        "InputOtpKind",
        segmented_fill(&kinds, &letters, move |letters| to_letters.set(letters)),
      ),
    ];
    // Capitals are a choice only of a code that has letters.
    if letters {
      controls.push(flag("uppercase", &self.uppercase));
    }
    controls.extend([
      prop(
        "size",
        "InputOtpSize",
        segmented_fill(&sizes, &self.size.get(), move |index| to_size.set(index)),
      ),
      divider(),
      flag("mask", &self.mask),
      flag("invalid", &self.invalid),
      flag("disabled", &self.disabled),
    ]);
    self.playground.card(
      ctx,
      Spec {
        title: "Input OTP",
        make: Arc::new(move || make().into()),
        shown: shown.into(),
        fill: false,
        forceable: true,
        facts: format!("{size_name} · cells {side} · radius 6"),
        event: "on_complete",
        code,
        tree,
        controls,
        changed,
        reset: Arc::new(reset),
      },
    )
  }
}

impl Component for InputOtpPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      value: ctx.signal(String::new()),
      length: ctx.signal(1),
      grouping: ctx.signal(1),
      letters: ctx.signal(false),
      uppercase: ctx.signal(false),
      mask: ctx.signal(false),
      size: ctx.signal(1),
      invalid: ctx.signal(false),
      disabled: ctx.signal(false),
      code: ctx.signal(String::new()),
      verified: ctx.signal(String::new()),
      recovery: ctx.signal(String::new()),
      pin: ctx.signal(String::new()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let verified = self.verified.clone();
    let told = if self.verified.get().is_empty() {
      format!("the signal has: {:?}", self.code.get())
    } else {
      format!("on_complete: {:?}", self.verified.get())
    };
    let live = items().child(item(
      InputOtp::new(self.code.clone())
        .groups([3, 3])
        .label("Verification code")
        .on_complete(move |code| verified.set(code.to_owned())),
      &told,
    ));

    let states = items()
      .child(item(showing(""), "empty"))
      .child(item(showing("427193"), "filled"))
      .child(item(showing("42").force(Some(Interaction::FocusVisible)), "focus"))
      .child(item(showing("427193").invalid(true), "invalid"))
      .child(item(showing("427193").valid(true), "valid"))
      .child(item(showing("427193").mask(true), "masked"))
      .child(item(showing("427193").read_only(true), "read only"))
      .child(item(showing("").disabled(true), "disabled"));

    let mut sizes = items();
    for (size, name, _, side) in SIZES {
      sizes = sizes.child(item(showing("4271").size(size), &format!("{name} · {side}")));
    }

    let kinds = items()
      .child(item(
        InputOtp::new(self.recovery.clone())
          .length(8)
          .groups([4, 4])
          .kind(InputOtpKind::Alphanumeric)
          .uppercase(true)
          .label("Recovery code"),
        "letters and digits, in capitals",
      ))
      .child(item(
        InputOtp::new(self.pin.clone()).length(4).mask(true).label("PIN"),
        "a PIN of four, masked",
      ));

    let fields = items()
      .child(
        Field::new(
          "Verification code",
          InputOtp::new(Signal::new(String::new())).groups([3, 3]),
        )
        .description("We sent a code to ivan@lurq.dev."),
      )
      .child(
        Field::new(
          "Verification code",
          InputOtp::new(Signal::new("427193".to_owned())).groups([3, 3]),
        )
        .error("That code has expired. Send a new one."),
      );

    let rows = [
      (
        "value",
        "Signal<String>",
        "",
        "The code: what the cells show, and what is typed is set on it.",
      ),
      ("length", "usize", "6", "How many cells it has."),
      (
        "groups",
        "impl IntoIterator<Item = usize>",
        "",
        "The sizes of the groups of cells, from the first; a dash between groups.",
      ),
      (
        "kind",
        "InputOtpKind",
        "Digits",
        "Digits, or letters and digits. Anything else typed or pasted is left out.",
      ),
      ("uppercase", "bool", "false", "A letter is taken as a capital."),
      ("mask", "bool", "false", "A dot for each character, as a PIN is shown."),
      (
        "on_complete",
        "impl Fn(&str)",
        "",
        "Called with the code when an edit fills its last cell.",
      ),
      (
        "size",
        "InputOtpSize",
        "Md",
        "Cells of 32 × 36, 36 × 40, 40 × 44; type of 14, 16, 18.",
      ),
      (
        "invalid / warning / valid",
        "bool",
        "false",
        "The color of every edge, and of the ring of the cell that has the focus.",
      ),
      (
        "read_only",
        "bool",
        "false",
        "It can be focused, read and copied, and not changed.",
      ),
      (
        "disabled",
        "bool",
        "false",
        "Half the opacity on the muted fill; it takes no input.",
      ),
      (
        "label",
        "&str",
        "",
        "What it is called, to what reads the tree. A field gives it its own label.",
      ),
      (
        "name",
        "impl Into<Arc<str>>",
        "",
        "The name of its value, the whole code, in the form it is in.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Inputs", "Input OTP"],
        title: "Input OTP",
        description: "A one-time code: a few characters typed one to a cell, the code a message or an authenticator \
                      app gives. To a person it is one field; to a form and to tooling, one text input.",
        import: Some("use lurq::ui::{InputOtp, InputOtpKind};"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of InputOtp is a control on the right. The code on the stage is live: type in it, paste a \
           code, press Backspace and the arrows.",
          playground,
        ),
        Section::new(
          "Input OTP",
          "Typing fills the cell that has the focus and goes on to the next; Backspace empties it and goes back; a \
           paste of a whole code fills every cell from the first, its spaces and dashes dropped. When the last cell \
           is filled, on_complete is called with the code.",
          self.frame.demo(
            0,
            live,
            ".on_complete(|code| verify(code))",
            &[
              "// code: Signal<String>",
              "InputOtp::new(code.clone())",
              "    .groups([3, 3])",
              "    .label(\"Verification code\")",
              "    .on_complete(|code| verify(code))",
            ],
          ),
        ),
        Section::new(
          "States",
          "The cell the focus is on has the ring, and while it is empty a caret. A judgment colors every edge. A \
           masked code shows a dot for each character.",
          self.frame.demo(
            1,
            states,
            ".invalid(true)",
            &[
              "InputOtp::new(code).groups([3, 3])",
              "",
              "// Drawn as it is focused, with nothing focusing it: for a gallery, a test.",
              "InputOtp::new(code).groups([3, 3]).force(Some(Interaction::FocusVisible))",
              "",
              "InputOtp::new(code).groups([3, 3]).invalid(true)",
              "InputOtp::new(code).groups([3, 3]).valid(true)",
              "InputOtp::new(code).groups([3, 3]).mask(true)",
              "InputOtp::new(code).groups([3, 3]).read_only(true)",
              "InputOtp::new(code).groups([3, 3]).disabled(true)",
            ],
          ),
        ),
        Section::new(
          "Sizes",
          "The cells a step smaller or larger, with their type; the room between them stays 6.",
          self.frame.demo(
            2,
            sizes,
            ".size(InputOtpSize::Sm)",
            &[
              "InputOtp::new(code).groups([3, 3]).size(InputOtpSize::Sm)",
              "InputOtp::new(code).groups([3, 3])",
              "InputOtp::new(code).groups([3, 3]).size(InputOtpSize::Lg)",
            ],
          ),
        ),
        Section::new(
          "Kinds",
          "A recovery code of letters and digits, taken as capitals however it is typed; a PIN of four, masked.",
          self.frame.demo(
            3,
            kinds,
            ".kind(InputOtpKind::Alphanumeric)",
            &[
              "InputOtp::new(recovery.clone())",
              "    .length(8)",
              "    .groups([4, 4])",
              "    .kind(InputOtpKind::Alphanumeric)",
              "    .uppercase(true)",
              "    .label(\"Recovery code\")",
              "",
              "InputOtp::new(pin.clone()).length(4).mask(true).label(\"PIN\")",
            ],
          ),
        ),
        Section::new(
          "In a field",
          "A field names the code with its label, judges it with its error, and its label focuses it.",
          self.frame.demo(
            4,
            fields,
            "Field::new(\"Verification code\", otp)",
            &[
              "Field::new(\"Verification code\", InputOtp::new(code.clone()).groups([3, 3]))",
              "    .description(\"We sent a code to ivan@lurq.dev.\")",
              "",
              "Field::new(\"Verification code\", InputOtp::new(code.clone()).groups([3, 3]))",
              "    .error(\"That code has expired. Send a new one.\")",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a code is given.",
          props_table(&rows, &["value"], self.frame.room()),
        ),
      ],
    )
  }
}
