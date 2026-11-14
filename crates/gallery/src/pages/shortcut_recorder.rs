//! The page of the shortcut recorder: its playground, the board's recorder, a combination the app refuses, an empty
//! one, its states, and a recorder in a field.

use std::sync::Arc;

use lurq::{
  app::{Accelerator, component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{Field, Interaction, KeyPlatform, ShortcutRecorder, ShortcutRecorderSize},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet, segmented_fill},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

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
    .overflow_visible()
}

fn shortcut(text: &str) -> Accelerator {
  Accelerator::parse(text).expect("a shortcut of the page")
}

/// What the page's command palette and quit are bound to: what a recorder of the page refuses.
fn taken(pressed: &Accelerator) -> Result<(), String> {
  match pressed.to_string().as_str() {
    "Ctrl+Shift+P" | "Meta+Shift+P" => Err("Already used by Command Palette".to_owned()),
    "Ctrl+Q" | "Meta+Q" => Err("Already used by Quit".to_owned()),
    _ => Ok(()),
  }
}

/// What a signal of a shortcut holds, as the page says it.
fn says(shortcut: &Option<Accelerator>) -> String {
  match shortcut {
    Some(shortcut) => format!("the signal has: Some({shortcut})"),
    None => "the signal has: None".to_owned(),
  }
}

pub struct ShortcutRecorderPage {
  frame: Frame,
  playground: Playground,
  shortcut: Signal<Option<Accelerator>>,
  conflicts: Signal<bool>,
  require_modifier: Signal<bool>,
  read_only: Signal<bool>,
  invalid: Signal<bool>,
  disabled: Signal<bool>,
  full_width: Signal<bool>,
  size: Signal<ShortcutRecorderSize>,
  mac: Signal<bool>,
  board: Signal<Option<Accelerator>>,
  conflict: Signal<Option<Accelerator>>,
  empty: Signal<Option<Accelerator>>,
  field: Signal<Option<Accelerator>>,
}

impl ShortcutRecorderPage {
  /// The playground: the recorder as the panel's props make it, bound to the page's signal.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (conflicts, require_modifier) = (self.conflicts.get(), self.require_modifier.get());
    let (read_only, invalid, disabled) = (self.read_only.get(), self.invalid.get(), self.disabled.get());
    let (full_width, size, mac) = (self.full_width.get(), self.size.get(), self.mac.get());
    let platform = if mac { KeyPlatform::Mac } else { KeyPlatform::Standard };
    let plain = move |held: Signal<Option<Accelerator>>| {
      let mut recorder = ShortcutRecorder::optional(held)
        .label("Command palette")
        .require_modifier(require_modifier)
        .read_only(read_only)
        .invalid(invalid)
        .disabled(disabled)
        .full_width(full_width)
        .size(size)
        .platform(platform);
      if conflicts {
        recorder = recorder.validate(taken);
      }
      recorder
    };
    let record = self.playground.recorder();
    let forced = self.playground.forced();
    let shown = {
      let (plain, held) = (plain.clone(), self.shortcut.clone());
      Shown::built(move |ctx| {
        plain(held)
          .force(forced)
          .on_change(move |shortcut: &Option<Accelerator>| {
            record(match shortcut {
              Some(shortcut) => format!("on_change(Some({shortcut}))"),
              None => "on_change(None)".to_owned(),
            })
          })
          .mount(ctx)
      })
    };

    let mut code = vec![
      "ShortcutRecorder::optional(shortcut)".to_owned(),
      "    .label(\"Command palette\")".to_owned(),
    ];
    for (on, call) in [
      (
        conflicts,
        "    .validate(|pressed| taken(pressed)) // Err(\"Already used by Command Palette\")".to_owned(),
      ),
      (!require_modifier, "    .require_modifier(false)".to_owned()),
      (read_only, "    .read_only(true)".to_owned()),
      (invalid, "    .invalid(true)".to_owned()),
      (disabled, "    .disabled(true)".to_owned()),
      (full_width, "    .full_width(true)".to_owned()),
      (
        size != ShortcutRecorderSize::Sm,
        format!("    .size(ShortcutRecorderSize::{size:?})"),
      ),
      (mac, "    .platform(KeyPlatform::Mac)".to_owned()),
    ] {
      if on {
        code.push(call);
      }
    }
    code.push("    .mount(ctx)".to_owned());

    let held = self.shortcut.get();
    let tree = vec![
      format!("root       {}", self.playground.measured()),
      "└─ control  230 × 32 · radius-md · edge 1 input".to_owned(),
      match &held {
        Some(shortcut) => format!("   ├─ keys  {shortcut}, a Kbd each, 4 apart"),
        None => "   ├─ hint  \"Record shortcut\", muted".to_owned(),
      },
      "   └─ clear  the cross, 14, muted".to_owned(),
    ];

    let sizes = [
      ("Sm", ShortcutRecorderSize::Sm),
      ("Md", ShortcutRecorderSize::Md),
      ("Lg", ShortcutRecorderSize::Lg),
    ];
    let to_size = self.size.clone();
    let controls = vec![
      prop(
        "size",
        "ShortcutRecorderSize",
        segmented_fill(&sizes, &size, move |size| to_size.set(size)),
      ),
      divider(),
      flag("validate: taken", &self.conflicts),
      flag("require_modifier", &self.require_modifier),
      flag("read_only", &self.read_only),
      flag("invalid", &self.invalid),
      flag("disabled", &self.disabled),
      flag("full_width", &self.full_width),
      divider(),
      flag("a Mac's keys", &self.mac),
    ];
    let resets = (
      self.shortcut.clone(),
      self.size.clone(),
      [self.conflicts.clone(), self.require_modifier.clone()],
      [
        self.read_only.clone(),
        self.invalid.clone(),
        self.disabled.clone(),
        self.full_width.clone(),
        self.mac.clone(),
      ],
    );
    let make_held = Signal::new(held.clone());
    self.playground.card(
      ctx,
      Spec {
        title: "Shortcut Recorder",
        make: Arc::new(move || plain(make_held.clone()).into()),
        shown,
        fill: false,
        forceable: true,
        facts: match &held {
          Some(shortcut) => shortcut.to_string(),
          None => "none".to_owned(),
        },
        event: "on_change",
        code,
        tree,
        controls,
        changed: [
          size != ShortcutRecorderSize::Sm,
          !conflicts,
          !require_modifier,
          read_only,
          invalid,
          disabled,
          full_width,
          mac,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          resets.0.set(Some(shortcut("Ctrl+K")));
          resets.1.set(ShortcutRecorderSize::Sm);
          for flag in &resets.2 {
            flag.set(true);
          }
          for flag in &resets.3 {
            flag.set(false);
          }
        }),
      },
    )
  }
}

impl Component for ShortcutRecorderPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      shortcut: ctx.signal(Some(shortcut("Ctrl+K"))),
      conflicts: ctx.signal(true),
      require_modifier: ctx.signal(true),
      read_only: ctx.signal(false),
      invalid: ctx.signal(false),
      disabled: ctx.signal(false),
      full_width: ctx.signal(false),
      size: ctx.signal(ShortcutRecorderSize::Sm),
      mac: ctx.signal(false),
      board: ctx.signal(Some(shortcut("Ctrl+Shift+P"))),
      conflict: ctx.signal(Some(shortcut("Ctrl+K"))),
      empty: ctx.signal(None),
      field: ctx.signal(None),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let playground = self.play(ctx);
    let board = items().child(item(
      ShortcutRecorder::optional(self.board.clone())
        .label("Command palette")
        .platform(KeyPlatform::Standard)
        .mount(ctx),
      &says(&self.board.get()),
    ));
    let conflict = items().child(item(
      ShortcutRecorder::optional(self.conflict.clone())
        .label("Toggle sidebar")
        .platform(KeyPlatform::Standard)
        .validate(taken)
        .mount(ctx),
      "press it, then Ctrl+Shift+P",
    ));
    let empty = items().child(item(
      ShortcutRecorder::optional(self.empty.clone())
        .label("Show window")
        .platform(KeyPlatform::Standard)
        .mount(ctx),
      &says(&self.empty.get()),
    ));
    let states = items()
      .child(item(
        ShortcutRecorder::controlled(shortcut("Ctrl+K"), |_| {})
          .platform(KeyPlatform::Standard)
          .force(Some(Interaction::FocusVisible)),
        "focus-visible",
      ))
      .child(item(
        ShortcutRecorder::controlled(shortcut("Ctrl+K"), |_| {}).platform(KeyPlatform::Mac),
        "a Mac's keys",
      ))
      .child(item(
        ShortcutRecorder::controlled(shortcut("Ctrl+K"), |_| {})
          .platform(KeyPlatform::Standard)
          .read_only(true),
        "read only",
      ))
      .child(item(
        ShortcutRecorder::controlled(shortcut("Ctrl+K"), |_| {})
          .platform(KeyPlatform::Standard)
          .invalid(true),
        "invalid",
      ))
      .child(item(
        ShortcutRecorder::controlled(shortcut("Ctrl+K"), |_| {})
          .platform(KeyPlatform::Standard)
          .disabled(true),
        "disabled",
      ));
    let given = self.field.get();
    let field = items().child(item(
      Column::new().width(280.0).child({
        let recorder = ShortcutRecorder::optional(self.field.clone())
          .label("Quick open")
          .platform(KeyPlatform::Standard)
          .invalid(given.is_none())
          .mount(ctx);
        let field = Field::new("Quick open", recorder);
        if given.is_none() {
          field.error("Choose a shortcut to open files with.")
        } else {
          field.description("Opens the file finder from anywhere.")
        }
      }),
      &says(&given),
    ));
    let rows = [
      (
        "new",
        "Signal<Accelerator>",
        "",
        "The shortcut the signal holds; a combination pressed sets it. optional(Signal<Option<Accelerator>>) holds \
         none too, which Backspace and the cross give.",
      ),
      (
        "controlled",
        "Accelerator, impl Fn(&Accelerator)",
        "",
        "In place of a signal: the app holds the shortcut and is told a combination taken. controlled_optional for \
         Option<Accelerator>.",
      ),
      (
        "on_change",
        "impl Fn(&V)",
        "",
        "Called with the value a combination taken or an emptying sets, after the signal. Not for the shortcut it \
         holds already.",
      ),
      (
        "validate",
        "Fn(&Accelerator) -> Result<(), String>",
        "",
        "The app refuses a combination with a sentence, said under the box. Each call adds a check.",
      ),
      (
        "require_modifier",
        "bool",
        "true",
        "A combination needs a modifier; a function key is taken alone.",
      ),
      (
        "platform",
        "KeyPlatform",
        "the app's",
        "Whose names the keys are drawn with.",
      ),
      (
        "label",
        "&str",
        "\"Shortcut\"",
        "What it is called to tooling. A field's label where it is given the field's.",
      ),
      (
        "placeholder",
        "&str",
        "\"Record shortcut\"",
        "What it says where it holds none.",
      ),
      (
        "recording_text",
        "&str",
        "\"Press shortcut…\"",
        "What it says while it records.",
      ),
      (
        "clear_label",
        "&str",
        "\"Clear shortcut\"",
        "What the cross is called to tooling.",
      ),
      (
        "modifier_text",
        "&str",
        "\"Hold Ctrl, Alt, …\"",
        "What it says of a combination refused for having no modifier.",
      ),
      (
        "size",
        "ShortcutRecorderSize",
        "Sm",
        "Sm 32, Md 36, Lg 40 high: a box's sizes.",
      ),
      (
        "invalid, warning, valid",
        "bool",
        "false",
        "What the app judged of what it holds: the edge in that color.",
      ),
      (
        "read_only",
        "bool",
        "false",
        "Shown and not changed: it does not record, and has no cross.",
      ),
      ("disabled", "bool", "false", "Dimmed; no stop of Tab."),
      (
        "full_width",
        "bool",
        "false",
        "As wide as what holds it, in place of 230.",
      ),
      (
        "name",
        "&str",
        "",
        "The name of its shortcut in a form, written as Ctrl+Shift+P.",
      ),
    ];
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Inputs", "Shortcut Recorder"],
        title: "Shortcut Recorder",
        description: "A box that holds a key combination, which a person sets by pressing it: the shortcut of a \
                      command in an app's settings.",
        import: Some("use lurq::{app::Accelerator, ui::ShortcutRecorder};"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of ShortcutRecorder is a control on the right. The recorder on the stage is live: press it, \
           or Enter while it has the focus, and press a combination. While it records it takes every key of the \
           window, the app's own shortcuts too; Escape gives up, Backspace empties it.",
          playground,
        ),
        Section::new(
          "Shortcut recorder",
          "The board's: its keys as Kbd draws them, the cross that empties it at its end. Recording, its edge is the \
           ring's color and a red dot stands before \"Press shortcut…\".",
          self.frame.demo(
            0,
            board,
            "ShortcutRecorder::optional(palette).mount(ctx)",
            &[
              "// palette: Signal<Option<Accelerator>>",
              "ShortcutRecorder::optional(palette)",
              "    .label(\"Command palette\")",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "A combination that is taken",
          "The app refuses a combination with a sentence: the box shows what was pressed edged as wrong, says why \
           under it, and keeps what it held.",
          self.frame.demo(
            1,
            conflict,
            ".validate(|pressed| taken(pressed))",
            &[
              "ShortcutRecorder::optional(sidebar)",
              "    .label(\"Toggle sidebar\")",
              "    .validate(|pressed| match pressed.to_string().as_str() {",
              "        \"Ctrl+Shift+P\" => Err(\"Already used by Command Palette\".to_owned()),",
              "        _ => Ok(()),",
              "    })",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Empty",
          "A recorder bound to a Signal<Option<Accelerator>> that holds None says its placeholder.",
          self.frame.demo(
            2,
            empty,
            "ShortcutRecorder::optional(show_window)",
            &["ShortcutRecorder::optional(show_window).label(\"Show window\").mount(ctx)"],
          ),
        ),
        Section::new(
          "States",
          "The ring of the focus is inside the box; the keys are drawn in the names of the platform; read only and \
           disabled do not record.",
          self.frame.demo(
            3,
            states,
            ".platform(KeyPlatform::Mac)",
            &[
              "ShortcutRecorder::controlled(shortcut, on_change).force(Some(Interaction::FocusVisible))",
              "ShortcutRecorder::controlled(shortcut, on_change).platform(KeyPlatform::Mac)",
              "ShortcutRecorder::controlled(shortcut, on_change).read_only(true)",
              "ShortcutRecorder::controlled(shortcut, on_change).invalid(true)",
              "ShortcutRecorder::controlled(shortcut, on_change).disabled(true)",
            ],
          ),
        ),
        Section::new(
          "In a field",
          "A field names the recorder and says what is wrong. The recorder is mounted to record, so it is given the \
           field's label and judgment itself.",
          self.frame.demo(
            4,
            field,
            "Field::new(\"Quick open\", recorder)",
            &[
              "// quick_open: Signal<Option<Accelerator>>",
              "let recorder = ShortcutRecorder::optional(quick_open.clone())",
              "    .label(\"Quick open\")",
              "    .invalid(quick_open.get().is_none())",
              "    .mount(ctx);",
              "Field::new(\"Quick open\", recorder).error(\"Choose a shortcut to open files with.\")",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a recorder is given.",
          props_table(&rows, &["new"], self.frame.room()),
        ),
      ],
    )
  }
}
