//! The page of text editing: what every field of the design system draws while it is typed in. The selection
//! and the caret, the line under a composition, the wavy line under a word the app marks, the completion Tab
//! takes, and the menu of an edit.

use std::{ops::Range, sync::Arc};

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{ContextMenu, Input, Kbd, MenuItem, Textarea},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

/// How wide the board draws its card of marks.
const WIDTH: f32 = 352.0;

/// What the page's spelling checker knows to be wrong.
const MISSPELLED: [&str; 5] = ["mispeled", "teh", "recieve", "seperate", "occured"];

/// What the page's shell knows how to complete.
const COMMANDS: [&str; 6] = [
  "cargo run --release",
  "cargo build --release",
  "cargo test -p lurq",
  "cargo check",
  "git status",
  "git commit -m \"\"",
];

/// The words of `text` the page's checker marks: the bytes of each.
fn misspelled(text: &str) -> Vec<Range<usize>> {
  let lower = text.to_lowercase();
  let mut found = Vec::new();
  for word in MISSPELLED {
    let mut from = 0;
    while let Some(at) = lower[from..].find(word) {
      let start = from + at;
      found.push(start..start + word.len());
      from = start + word.len();
    }
  }
  found
}

/// What would complete `typed`: the rest of the first command that starts with it.
fn completion(typed: &str) -> &'static str {
  if typed.is_empty() {
    return "";
  }
  COMMANDS
    .iter()
    .find_map(|command| command.strip_prefix(typed))
    .unwrap_or("")
}

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .width(WIDTH)
    .max_width(FILL)
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

pub struct TextEditingPage {
  frame: Frame,
  playground: Playground,
  value: Signal<String>,
  offer: Signal<String>,
  marks: Signal<bool>,
  read_only: Signal<bool>,
  disabled: Signal<bool>,
  sentence: Signal<String>,
  composed: Signal<String>,
  spelled: Signal<String>,
  note: Signal<String>,
  command: Signal<String>,
  edited: Signal<String>,
  edit_open: Signal<bool>,
  edit_did: Signal<String>,
}

impl TextEditingPage {
  /// The playground: an input with the marks an app gives it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (offer, marks) = (self.offer.get(), self.marks.get());
    let (read_only, disabled) = (self.read_only.get(), self.disabled.get());
    let held = self.value.get();
    let ranges = if marks { misspelled(&held) } else { Vec::new() };
    let plain = {
      let (value, offer, ranges) = (self.value.clone(), offer.clone(), ranges.clone());
      move || {
        Input::new(value.clone())
          .completion(&offer)
          .squiggles(ranges.clone())
          .read_only(read_only)
          .disabled(disabled)
          .full_width(true)
      }
    };
    let record = self.playground.recorder();
    let shown = self
      .playground
      .styled(plain())
      .force(self.playground.forced())
      .on_input(move |event: &lurq::app::events::TextInputEvent| {
        record(format!("on_input({} characters)", event.new_value.chars().count()))
      });

    let mut code = vec![
      "// value: Signal<String>".to_owned(),
      "Input::new(value.clone())".to_owned(),
    ];
    if !offer.is_empty() {
      code.push(format!("    .completion({offer:?})"));
    }
    if marks {
      code.push("    // What the app's spelling checker found: the bytes of each word.".to_owned());
      code.push(format!("    .squiggles({ranges:?})"));
    }
    if read_only {
      code.push("    .read_only(true)".to_owned());
    }
    if disabled {
      code.push("    .disabled(true)".to_owned());
    }
    let tree = vec![
      format!("root         {}", self.playground.measured()),
      "├─ control    the text, its selection in Selection, its caret in Foreground".to_owned(),
      format!("├─ squiggle   {} ranges · Destructive", ranges.len()),
      format!("└─ completion {offer:?} · MutedForeground at 0.7 · Tab takes it"),
    ];

    let (to_value, to_offer, to_marks) = (self.value.clone(), self.offer.clone(), self.marks.clone());
    let off = [self.read_only.clone(), self.disabled.clone()];
    let make = {
      let plain = plain.clone();
      move || Column::new().width(WIDTH).child(plain()).into()
    };
    self.playground.card(
      ctx,
      Spec {
        title: "Text Editing",
        make: Arc::new(make),
        shown: Shown::of(Column::new().width(WIDTH).max_width(FILL).child(shown)),
        fill: false,
        forceable: true,
        facts: format!("{} characters", held.chars().count()),
        event: "on_input",
        code,
        tree,
        controls: vec![
          prop("value", "Signal<String>", field(&self.value)),
          prop("completion", "&str", field(&self.offer)),
          flag("squiggles", &self.marks),
          divider(),
          flag("read_only", &self.read_only),
          flag("disabled", &self.disabled),
        ],
        changed: [
          held != "A mispeled word in teh sentence",
          offer != " twice",
          !marks,
          read_only,
          disabled,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          to_value.set("A mispeled word in teh sentence".to_owned());
          to_offer.set(" twice".to_owned());
          to_marks.set(true);
          for flag in &off {
            flag.set(false);
          }
        }),
      },
    )
  }
}

impl Component for TextEditingPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      value: ctx.signal("A mispeled word in teh sentence".to_owned()),
      offer: ctx.signal(" twice".to_owned()),
      marks: ctx.signal(true),
      read_only: ctx.signal(false),
      disabled: ctx.signal(false),
      sentence: ctx.signal("The quick brown fox jumps".to_owned()),
      composed: ctx.signal("Typing ".to_owned()),
      spelled: ctx.signal("A mispeled word".to_owned()),
      note: ctx.signal("We recieve the files in seperate folders.\nIt occured twice.".to_owned()),
      command: ctx.signal("cargo ru".to_owned()),
      edited: ctx.signal("Select some of this text".to_owned()),
      edit_open: ctx.signal(false),
      edit_did: ctx.signal(String::new()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let selection = items()
      .child(item(
        Input::new(self.sentence.clone()).full_width(true),
        "Shift and the arrows, a drag, a double click on a word",
      ))
      .child(item(
        Input::new(self.composed.clone()).full_width(true),
        "typed with an input method, what is composed is underlined",
      ));
    let spelled = self.spelled.get();
    let note = self.note.get();
    let spelling = items()
      .child(item(
        Input::new(self.spelled.clone())
          .squiggles(misspelled(&spelled))
          .full_width(true),
        &format!("{} marked", misspelled(&spelled).len()),
      ))
      .child(item(
        Textarea::new(self.note.clone())
          .squiggles(misspelled(&note))
          .rows(2, 5)
          .full_width(true),
        "a textarea's lines, a wave on each row a range is on",
      ));
    let command = self.command.get();
    let offered = completion(&command);
    let completing = items().child(item(
      Row::new()
        .width(FILL)
        .spacing(10.0)
        .align_items(Alignment::Center)
        .child(
          Column::new()
            .flex(1.0)
            .min_width(0.0)
            .child(Input::new(self.command.clone()).completion(offered).full_width(true)),
        )
        .child(Kbd::new("Tab")),
      if offered.is_empty() {
        "nothing to offer: type cargo, or git"
      } else {
        "Tab takes what is offered"
      },
    ));
    let did = self.edit_did.get();
    let says = |name: &'static str| {
      let did = self.edit_did.clone();
      move || did.set(name.to_owned())
    };
    let menu = items().child(item(
      ContextMenu::new(self.edit_open.clone(), Input::new(self.edited.clone()).full_width(true))
        .bar(true)
        .items(["Cut", "Copy", "Paste", "Select All"].map(|name| MenuItem::new(name).on_select(says(name)))),
      &if did.is_empty() {
        "the secondary button, or the menu key, on the field".to_owned()
      } else {
        format!("chosen: {did}")
      },
    ));
    let props = [
      (
        "selection",
        "PaletteColor::Selection",
        "",
        "The fill behind selected text, while the field has the focus. `TextInput::selection_color` for one field.",
      ),
      (
        "caret",
        "the text's color",
        "",
        "A line where the next character goes, as wide and as high as the theme's caret says: 1.5 by 18 in type \
         of 14. It blinks unless the theme's `CaretMode` is `Persistent`. `TextInput::caret_color` for one field.",
      ),
      (
        "composition",
        "the text's color",
        "",
        "A line under what an input method is composing, until it is committed, as thick as the theme's caret \
         says: 1.5.",
      ),
      (
        "squiggles",
        "impl IntoIterator<Item = Range<usize>>",
        "",
        "`Input::squiggles` and `Textarea::squiggles`: wavy lines in `Destructive` under ranges of the text, \
         bytes of it. The part `Squiggle` gives them another color.",
      ),
      (
        "completion",
        "&str",
        "",
        "`Input::completion`: what would complete what is typed, drawn after it in `MutedForeground` at 0.7 \
         while the caret is at the end. Tab puts it there. The part `Completion` gives it another look.",
      ),
      (
        "edit menu",
        "ContextMenu::bar(true)",
        "",
        "The commands of selected text in one row over a field: a context menu drawn as a bar.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Desktop", "Text Editing"],
        title: "Text Editing",
        description: "What every field of the design system draws while it is typed in: the selection and the \
                      caret, the line under a composition, the wavy line under a word the app marks, the \
                      completion Tab takes, and the menu of an edit.",
        import: Some("use lurq::ui::{Input, Textarea};"),
        status: Some("New"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "An input with the marks an app gives it. The page's own checker marks a few misspelled words, and \
           what `completion` holds is offered after what is typed: put the caret at the end and press Tab.",
          playground,
        ),
        Section::new(
          "Selection and caret",
          "The engine's text input draws them in the theme's roles in every field: the selection in Selection \
           behind the band of the text, the caret in the color of the text. A composition of an input method is \
           underlined until it is committed.",
          self.frame.demo(
            0,
            selection,
            "Input::new(value.clone())",
            &[
              "// Nothing to ask for: every field has them.",
              "Input::new(sentence.clone())",
              "",
              "// One field in colors of its own, on the engine's input.",
              "TextInput::new(value).selection_color(PaletteColor::InfoMuted).caret_color(PaletteColor::Info)",
            ],
          ),
        ),
        Section::new(
          "Spell check",
          "lurq has no spelling checker: the app finds what is wrong and gives the field the ranges, bytes of \
           its text, and the field draws a wavy line in the destructive color under each. The page marks five \
           words it knows to be wrong.",
          self.frame.demo(
            1,
            spelling,
            ".squiggles(ranges)",
            &[
              "// text: Signal<String>; misspelled(&str) -> Vec<Range<usize>> is the app's checker.",
              "Input::new(text.clone()).squiggles(misspelled(&text.get()))",
              "Textarea::new(note.clone()).squiggles(misspelled(&note.get()))",
            ],
          ),
        ),
        Section::new(
          "Ghost completion",
          "What would complete what is typed is drawn after it, faint, while the field has the focus with the \
           caret at its end. Tab puts it in the field as an edit; anywhere else Tab goes on to the next control. \
           The key beside the field is a Kbd of the page's.",
          self.frame.demo(
            2,
            completing,
            ".completion(rest)",
            &[
              "// command: Signal<String>; the app says what would complete it now.",
              "let rest = COMMANDS",
              "    .iter()",
              "    .find_map(|known| known.strip_prefix(command.get().as_str()))",
              "    .unwrap_or(\"\");",
              "Input::new(command.clone()).completion(rest)",
            ],
          ),
        ),
        Section::new(
          "Edit menu",
          "The commands of an edit in one row over the field: a context menu drawn as a bar. Left and Right \
           move between them.",
          self.frame.demo(
            3,
            menu,
            "ContextMenu::new(open, field).bar(true)",
            &[
              "ContextMenu::new(open.clone(), Input::new(text.clone()))",
              "    .bar(true)",
              "    .items([\"Cut\", \"Copy\", \"Paste\", \"Select All\"].map(|name| MenuItem::new(name)))",
            ],
          ),
        ),
        Section::new(
          "Marks",
          "What draws each mark, and what an app says to change it.",
          props_table(&props, &[], self.frame.room()),
        ),
      ],
    )
  }
}
