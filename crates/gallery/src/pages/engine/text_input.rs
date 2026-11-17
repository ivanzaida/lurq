//! The page of the engine's text input: a field bound to a `Signal<String>`, on one line or on many, in the type it
//! is given; the controller that edits its selection from outside it; and what an app draws over its text, tokens,
//! a suggestion after it and wavy lines under it. Its sections are the Text Fields card of the demo's Inputs page.

use std::{ops::Range, sync::Arc};

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{CaretMode, PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::{
    Column, InputGhost, InputSquiggles, InputToken, InputTokenStyle, InputTokens, Row, TextInput, TextInputController,
  },
  core::Signal,
  layout::{
    Alignment,
    text_style::{FontWeight, TextAlign, TextStyle},
  },
  node::{CursorIcon, Element, border::Border},
  ui::{Button, ButtonSize, ButtonVariant, Switch},
};

use super::kit::{controls, named};
use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, mono, props_table, quiet, text},
  playground::{Playground, Shown, Spec, divider, field as text_field, flag, prop},
};

const MODES: [(&str, usize); 4] = [("single_line", 0), ("multiline", 1), ("textarea", 2), ("rows(3, 4)", 3)];
const SIZES: [(&str, i32); 4] = [("13", 13), ("14", 14), ("16", 16), ("20", 20)];
const ALIGNS: [(&str, usize); 3] = [("Left", 0), ("Center", 1), ("Right", 2)];
const ALIGN_VALUES: [TextAlign; 3] = [TextAlign::Left, TextAlign::Center, TextAlign::Right];

/// The variables the page's token parser knows: anything else between `{{` and `}}` is drawn as unknown.
const VARIABLES: [&str; 3] = ["task.title", "task.key", "project.name"];
/// What the page's spelling checker knows to be wrong.
const MISSPELLED: [&str; 4] = ["teh", "mispeled", "recieve", "seperate"];
/// What the page's shell knows how to complete.
const COMMANDS: [&str; 4] = ["cargo run --release", "cargo test -p lurq", "cargo check", "git status"];

/// What a field holds, as the demo's pills said it: on one line, cut after 42 characters, `<empty>` for nothing.
fn preview(value: &str) -> String {
  if value.is_empty() {
    return "<empty>".to_owned();
  }
  let line = value.replace(['\r', '\n'], " / ");
  if line.chars().count() <= 42 {
    return line;
  }
  let mut cut: String = line.chars().take(42).collect();
  cut.push_str("...");
  cut
}

/// What each signal of a section holds, a line each.
fn held(fields: &[(&str, &Signal<String>)]) -> Column {
  let mut list = Column::new().width(FILL).spacing(6.0);
  for (name, value) in fields {
    list = list.child(named(name, mono(&preview(&value.get()), 12.0).width(FILL)));
  }
  list
}

/// A field under its name.
fn labelled(label: &str, input: impl Into<Element>) -> Column {
  Column::new()
    .spacing(6.0)
    .child(text(label, 12.0, FontWeight::Medium).color(PaletteColor::MutedForeground))
    .child(input)
}

/// A text input in the theme's frame of a field: its background, an edge of the Input role, and an edge of the
/// Ring role while it has the focus.
fn framed(input: TextInput) -> TextInput {
  input
    .width(FILL)
    .padding_horizontal(12.0)
    .padding_vertical(6.0)
    .rounded(RadiusSize::Md)
    .background(PaletteColor::Background)
    .border(Border::inside(1.0, PaletteColor::Input))
    .focused(|style| style.border_inside(2.0, PaletteColor::Ring))
    .cursor(CursorIcon::Text)
}

/// The theme's body type at `size`.
fn body(theme: &Theme, size: f32) -> TextStyle {
  TextStyle {
    font_size: size,
    ..theme.typography_style(TypographyStyle::Body)
  }
}

/// The theme's body type at `size` in the muted color: what a placeholder is drawn in.
fn placeholder(theme: &Theme, size: f32) -> TextStyle {
  TextStyle {
    color: theme.palette_color(PaletteColor::MutedForeground),
    ..body(theme, size)
  }
}

/// The playground's input as its controls make it.
#[derive(Clone)]
struct Field {
  value: Signal<String>,
  placeholder: String,
  mode: usize,
  size: i32,
  align: usize,
  mask: bool,
  select_on_focus: bool,
  persistent: bool,
}

fn field_input(theme: &Theme, field: &Field) -> TextInput {
  let size = field.size as f32;
  let mut input = TextInput::styled(field.value.clone(), body(theme, size))
    .key(format!("playground-{}", field.mode))
    .placeholder(&field.placeholder)
    .placeholder_style(placeholder(theme, size))
    .text_align(ALIGN_VALUES[field.align.min(2)]);
  input = match field.mode {
    1 => input.multiline(),
    2 => input.textarea(),
    3 => input.rows(3, 4),
    _ => input.single_line(),
  };
  if field.mask {
    input = input.mask();
  }
  if field.select_on_focus {
    input = input.select_on_focus(true);
  }
  if field.persistent {
    input = input.caret_mode(CaretMode::Persistent);
  }
  framed(input).max_width(360.0)
}

/// The byte ranges of every `{{...}}` in `source` that stays on one line.
fn expressions(source: &str) -> Vec<Range<usize>> {
  let mut found = Vec::new();
  let mut from = 0;
  while let Some(open) = source[from..].find("{{") {
    let start = from + open;
    let Some(close) = source[start + 2..].find("}}") else {
      break;
    };
    let end = start + 2 + close + 2;
    if !source[start..end].contains(['\n', '\r']) {
      found.push(start..end);
    }
    from = end;
  }
  found
}

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

pub struct TextInputPage {
  frame: Frame,
  playground: Playground,
  value: Signal<String>,
  placeholder: Signal<String>,
  /// An index into [`MODES`].
  mode: Signal<usize>,
  size: Signal<i32>,
  /// An index into [`ALIGN_VALUES`].
  align: Signal<usize>,
  mask: Signal<bool>,
  select_on_focus: Signal<bool>,
  persistent: Signal<bool>,
  name: Signal<String>,
  email: Signal<String>,
  hash: Signal<String>,
  styled_notes: Signal<String>,
  scroll: Signal<String>,
  notes: Signal<String>,
  limited: Signal<String>,
  prompt: Signal<String>,
  controller: TextInputController,
  /// What the controller said last: the selection it read, or what an edit did.
  said: Signal<String>,
  template: Signal<String>,
  atomic: Signal<bool>,
  command: Signal<String>,
  command_controller: TextInputController,
  sentence: Signal<String>,
}

impl TextInputPage {
  fn field(&self) -> Field {
    Field {
      value: self.value.clone(),
      placeholder: self.placeholder.get(),
      mode: self.mode.get(),
      size: self.size.get(),
      align: self.align.get(),
      mask: self.mask.get(),
      select_on_focus: self.select_on_focus.get(),
      persistent: self.persistent.get(),
    }
  }

  fn play(&self, ctx: &mut Ctx) -> Element {
    let field = self.field();
    let (mode_name, _) = MODES[field.mode.min(3)];
    let (align_name, _) = ALIGNS[field.align.min(2)];
    let mut code = vec![
      "// value: Signal<String>".to_owned(),
      format!(
        "let style = TextStyle {{ font_size: {}.0, ..theme.typography_style(TypographyStyle::Body) }};",
        field.size
      ),
      "TextInput::styled(value, style)".to_owned(),
      format!("    .placeholder({:?})", field.placeholder),
      format!("    .{mode_name}()"),
    ];
    if field.align != 0 {
      code.push(format!("    .text_align(TextAlign::{align_name})"));
    }
    if field.mask {
      code.push("    .mask()".to_owned());
    }
    if field.select_on_focus {
      code.push("    .select_on_focus(true)".to_owned());
    }
    if field.persistent {
      code.push("    .caret_mode(CaretMode::Persistent)".to_owned());
    }
    code.extend([
      "    .padding_horizontal(12.0)".to_owned(),
      "    .padding_vertical(6.0)".to_owned(),
      "    .rounded(RadiusSize::Md)".to_owned(),
      "    .background(PaletteColor::Background)".to_owned(),
      "    .border(Border::inside(1.0, PaletteColor::Input))".to_owned(),
      "    .focused(|style| style.border_inside(2.0, PaletteColor::Ring))".to_owned(),
    ]);
    let value = self.value.get();
    let tree = vec![
      format!("text input   {}", self.playground.measured()),
      format!("  value   {:?}", preview(&value)),
      format!("  {} lines, {} bytes", value.split('\n').count(), value.len()),
    ];

    let (set_mode, set_size, set_align) = (self.mode.clone(), self.size.clone(), self.align.clone());
    let reset = {
      let (value, placeholder, mode, size, align) = (
        self.value.clone(),
        self.placeholder.clone(),
        self.mode.clone(),
        self.size.clone(),
        self.align.clone(),
      );
      let flags = [self.mask.clone(), self.select_on_focus.clone(), self.persistent.clone()];
      Arc::new(move || {
        value.set("Ada Lovelace".to_owned());
        placeholder.set("Display name".to_owned());
        mode.set(0);
        size.set(14);
        align.set(0);
        for flag in &flags {
          flag.set(false);
        }
      })
    };
    let (made, shown) = (field.clone(), field.clone());
    self.playground.card(
      ctx,
      Spec {
        title: "Text input",
        make: Arc::new(move || field_input(&lurq::ui::current_theme(), &made).into()),
        shown: Shown::built(move |ctx| field_input(ctx.theme(), &shown).into()),
        fill: true,
        forceable: false,
        facts: format!("{} bytes", value.len()),
        event: "on_input",
        code,
        tree,
        controls: vec![
          prop("placeholder", "&str", text_field(&self.placeholder)),
          prop(
            "single_line / multiline",
            "how it lays its lines out",
            self
              .playground
              .select(0, &MODES, &field.mode, move |value| set_mode.set(value)),
          ),
          prop(
            "font_size",
            "TextStyle",
            self
              .playground
              .select(1, &SIZES, &field.size, move |value| set_size.set(value)),
          ),
          prop(
            "text_align",
            "TextAlign",
            self
              .playground
              .select(2, &ALIGNS, &field.align, move |value| set_align.set(value)),
          ),
          divider(),
          flag("mask", &self.mask),
          flag("select_on_focus", &self.select_on_focus),
          flag("caret_mode(Persistent)", &self.persistent),
        ],
        changed: [
          field.placeholder != "Display name",
          field.mode != 0,
          field.size != 14,
          field.align != 0,
          field.mask,
          field.select_on_focus,
          field.persistent,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset,
      },
    )
  }

  /// Two fields on one line each: one that holds a name, one empty with what to type in it.
  fn single_line(&self, theme: &Theme) -> Column {
    let input = |value: &Signal<String>, hint: &str| {
      framed(
        TextInput::styled(value.clone(), body(theme, 14.0))
          .placeholder(hint)
          .placeholder_style(placeholder(theme, 14.0))
          .single_line(),
      )
    };
    Column::new()
      .width(FILL)
      .spacing(16.0)
      .child(
        Row::new()
          .width(FILL)
          .spacing(18.0)
          .wrap()
          .child(
            labelled("Name", input(&self.name, "Display name"))
              .flex(1.0)
              .min_width(200.0)
              .max_width(FILL),
          )
          .child(
            labelled("Email", input(&self.email, "name@example.com"))
              .flex(1.0)
              .min_width(200.0)
              .max_width(FILL),
          ),
      )
      .child(held(&[("name", &self.name), ("email", &self.email)]))
  }

  /// Fields in a type of their own: a hash on one line, and notes in a box of a fixed height.
  fn styled(&self, theme: &Theme) -> Column {
    let value_style = TextStyle {
      font_size: 13.0,
      weight: FontWeight::Medium,
      ..theme.typography_style(TypographyStyle::Mono)
    };
    let hint_style = TextStyle {
      color: theme.palette_color(PaletteColor::MutedForeground),
      ..value_style.clone()
    };
    let frame = |input: TextInput| {
      input
        .width(FILL)
        .padding_horizontal(10.0)
        .rounded(RadiusSize::Sm)
        .background(PaletteColor::Muted)
        .border(Border::inside(1.0, PaletteColor::Border))
        .caret_color(PaletteColor::Info)
        .focused(|style| style.border_inside(1.0, PaletteColor::Info))
        .cursor(CursorIcon::Text)
    };
    let hash = frame(
      TextInput::styled(self.hash.clone(), value_style.clone())
        .placeholder("a3f1b2c4d5e691cc...")
        .placeholder_style(hint_style.clone())
        .single_line(),
    )
    .height(40.0);
    let notes = frame(
      TextInput::styled(self.styled_notes.clone(), value_style)
        .placeholder("Fixed height notes")
        .placeholder_style(hint_style)
        .multiline(),
    )
    .height(82.0)
    .padding_vertical(10.0);
    Column::new()
      .width(FILL)
      .spacing(16.0)
      .child(labelled("Styled hash", hash).width(FILL))
      .child(labelled("Styled multiline", notes).width(FILL))
      .child(held(&[("hash", &self.hash), ("styled notes", &self.styled_notes)]))
  }

  /// A line that scrolls, a box that grows with its lines, and one that grows from three rows to four.
  fn growing(&self, theme: &Theme) -> Column {
    let scroll = framed(
      TextInput::styled(self.scroll.clone(), body(theme, 14.0))
        .placeholder("Single line")
        .placeholder_style(placeholder(theme, 14.0))
        .single_line(),
    )
    .height(38.0);
    let notes = framed(
      TextInput::styled(self.notes.clone(), body(theme, 14.0))
        .placeholder("Notes")
        .placeholder_style(placeholder(theme, 14.0))
        .textarea(),
    )
    .padding_vertical(8.0);
    let limited = framed(
      TextInput::styled(self.limited.clone(), body(theme, 14.0))
        .placeholder("Capped rows")
        .placeholder_style(placeholder(theme, 14.0))
        .rows(3, 4),
    )
    .padding_vertical(8.0);
    Column::new()
      .width(FILL)
      .spacing(16.0)
      .child(
        Row::new()
          .width(FILL)
          .spacing(18.0)
          .wrap()
          .align_items(Alignment::Start)
          .child(labelled("Scroll", scroll).width(220.0).max_width(FILL))
          .child(labelled("Multiline", notes).flex(1.0).min_width(200.0).max_width(FILL)),
      )
      .child(labelled("Rows 3-4", limited).width(FILL))
      .child(held(&[
        ("scroll", &self.scroll),
        ("notes", &self.notes),
        ("rows", &self.limited),
      ]))
  }

  /// An input a controller edits from the buttons beside it, and what the controller said last.
  fn controlled(&self, theme: &Theme) -> Column {
    let command = |label: &str, run: Arc<dyn Fn(&TextInputController) -> String + Send + Sync>| {
      let (controller, said) = (self.controller.clone(), self.said.clone());
      Button::new(label)
        .variant(ButtonVariant::Outline)
        .size(ButtonSize::Sm)
        .on_click(move |_| said.set(run(&controller)))
    };
    let told = |result: Result<bool, lurq::components::InputEditError>, done: &str| match result {
      Ok(true) => done.to_owned(),
      Ok(false) => "nothing to do".to_owned(),
      Err(error) => error.to_string(),
    };
    let prompt = self.prompt.clone();
    let bar = Row::new()
      .width(FILL)
      .spacing(6.0)
      .wrap()
      .child(command(
        "Insert {{task.title}}",
        Arc::new(move |controller: &TextInputController| {
          told(
            controller.replace_selection("{{task.title}}"),
            "replace_selection: inserted",
          )
        }),
      ))
      .child(command(
        "Select all",
        Arc::new(
          move |controller: &TextInputController| match controller.select(0, prompt.get_untracked().len()) {
            Ok(()) => "select: all of it".to_owned(),
            Err(error) => error.to_string(),
          },
        ),
      ))
      .child(
        command(
          "Undo",
          Arc::new(move |controller: &TextInputController| told(controller.undo(), "undo: taken back")),
        )
        .disabled(!self.controller.can_undo()),
      )
      .child(
        command(
          "Redo",
          Arc::new(move |controller: &TextInputController| told(controller.redo(), "redo: made again")),
        )
        .disabled(!self.controller.can_redo()),
      )
      .child(command(
        "Read the selection",
        Arc::new(|controller: &TextInputController| match controller.selection() {
          Ok(selection) => match selection.anchor {
            Some(anchor) if anchor != selection.caret => {
              format!("caret at byte {}, anchor at byte {}", selection.caret, anchor)
            }
            _ => format!("caret at byte {}, nothing selected", selection.caret),
          },
          Err(error) => error.to_string(),
        }),
      ));
    let input = framed(
      TextInput::styled(self.prompt.clone(), body(theme, 14.0))
        .key("controlled")
        .controller(self.controller.clone())
        .multiline(),
    )
    .padding_vertical(8.0);
    let said = self.said.get();
    Column::new()
      .width(FILL)
      .spacing(12.0)
      .child(bar)
      .child(input)
      .child(named(
        "controller",
        mono(if said.is_empty() { "nothing yet" } else { &said }, 12.0),
      ))
      .child(held(&[("value", &self.prompt)]))
  }

  /// An input whose `{{...}}` expressions are drawn as pills: known ones in one color, unknown ones in another.
  fn tokens(&self, theme: &Theme) -> Column {
    let source = self.template.get();
    let atomic = self.atomic.get();
    let known = InputTokenStyle::new(PaletteColor::Info, PaletteColor::InfoMuted)
      .padding(4.0)
      .radius(RadiusSize::Sm);
    let unknown = InputTokenStyle::new(PaletteColor::Destructive, PaletteColor::DestructiveMuted)
      .padding(4.0)
      .radius(RadiusSize::Sm);
    let tokens: Vec<InputToken> = expressions(&source)
      .into_iter()
      .map(|range| {
        let name = source[range.start + 2..range.end - 2].trim();
        let style = if VARIABLES.contains(&name) {
          known.clone()
        } else {
          unknown.clone()
        };
        InputToken::new(range, style).atomic(atomic)
      })
      .collect();
    let count = tokens.len();
    let mut input = TextInput::styled(self.template.clone(), body(theme, 14.0))
      .key("tokens")
      .multiline();
    // A token the parser found is over the source as it is now: the constructor checks it, and an error leaves
    // the text plain.
    if let Ok(tokens) = InputTokens::new(&source, tokens) {
      input = input.tokens(tokens);
    }
    let set = self.atomic.clone();
    Column::new()
      .width(FILL)
      .spacing(12.0)
      .child(
        controls().child(
          Row::new()
            .spacing(10.0)
            .align_items(Alignment::Center)
            .child(Switch::new("Atomic tokens").on(atomic).on_change(move |on| set.set(on))),
        ),
      )
      .child(framed(input).padding_vertical(8.0))
      .child(named("tokens", mono(&count.to_string(), 12.0)))
      .child(quiet(&format!("known: {}", VARIABLES.join(", ")), 12.0))
  }

  /// A command line with the rest of a command after what is typed, and a note with wavy lines under the words
  /// the page's checker does not know.
  fn marks(&self, theme: &Theme) -> Column {
    let typed = self.command.get();
    let rest = completion(&typed);
    let command = framed(
      TextInput::styled(self.command.clone(), theme.typography_style(TypographyStyle::Mono))
        .key("command")
        .controller(self.command_controller.clone())
        .placeholder("cargo")
        .placeholder_style(TextStyle {
          color: theme.palette_color(PaletteColor::MutedForeground),
          ..theme.typography_style(TypographyStyle::Mono)
        })
        .single_line()
        .ghost(InputGhost::new(rest).color(PaletteColor::MutedForeground).opacity(0.7)),
    )
    .height(38.0);
    let take = {
      let (controller, command) = (self.command_controller.clone(), self.command.clone());
      move |_| {
        let end = command.get_untracked().len();
        let rest = completion(&command.get_untracked());
        if controller.select(end, end).is_ok() {
          let _ = controller.replace_selection(rest);
        }
      }
    };
    let sentence = self.sentence.get();
    let note = framed(
      TextInput::styled(self.sentence.clone(), body(theme, 14.0))
        .multiline()
        .squiggles(InputSquiggles::new(misspelled(&sentence)).color(PaletteColor::Destructive)),
    )
    .padding_vertical(8.0);
    Column::new()
      .width(FILL)
      .spacing(16.0)
      .child(labelled("A suggestion", command).width(FILL))
      .child(
        controls().child(
          Button::new("Take the suggestion")
            .variant(ButtonVariant::Outline)
            .size(ButtonSize::Sm)
            .disabled(rest.is_empty())
            .on_click(take),
        ),
      )
      .child(labelled("Wavy lines", note).width(FILL))
      .child(quiet(&format!("marked: {}", MISSPELLED.join(", ")), 12.0))
  }
}

impl Component for TextInputPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let playground = Playground::new(ctx, 3);
    let value = ctx.signal("Ada Lovelace".to_owned());
    playground.watch(ctx, &value, |value| format!("on_input: {:?}", preview(value)));
    Self {
      frame: Frame::new(ctx),
      playground,
      value,
      placeholder: ctx.signal("Display name".to_owned()),
      mode: ctx.signal(0),
      size: ctx.signal(14),
      align: ctx.signal(0),
      mask: ctx.signal(false),
      select_on_focus: ctx.signal(false),
      persistent: ctx.signal(false),
      name: ctx.signal("Lol Kek".to_owned()),
      email: ctx.signal(String::new()),
      hash: ctx.signal(String::new()),
      styled_notes: ctx.signal(String::new()),
      scroll: ctx.signal("A long single-line value that should scroll inside a narrow input".to_owned()),
      notes: ctx.signal("Line one\nLine two".to_owned()),
      limited: ctx.signal("First row\nSecond row\nThird row\nFourth row\nFifth row".to_owned()),
      prompt: ctx.signal("Write a summary of the task: ".to_owned()),
      controller: TextInputController::new(),
      said: ctx.signal(String::new()),
      template: ctx.signal("Fix {{task.key}}: {{task.title}} in {{project.name}}, as {{owner}} asked.".to_owned()),
      atomic: ctx.signal(false),
      command: ctx.signal("cargo ru".to_owned()),
      command_controller: TextInputController::new(),
      sentence: ctx.signal("Teh word mispeled here is marked, and so is recieve.".to_owned()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let theme = ctx.theme().clone();
    let rows = [
      (
        "new / styled",
        "Signal<String> (, TextStyle)",
        "",
        "A field bound to a signal: what is typed is written to it, and what the app sets is shown. styled gives \
         it a type of its own.",
      ),
      (
        "single_line",
        "()",
        "",
        "One line that scrolls sideways to follow the caret. Enter does not break it.",
      ),
      (
        "multiline / textarea",
        "()",
        "",
        "Lines that wrap at its width, and grow it with them; Enter breaks a line.",
      ),
      (
        "rows",
        "(usize, usize)",
        "",
        "A multiline field at least as high as the first number of rows and at most the second, then scrolling.",
      ),
      (
        "placeholder / placeholder_style",
        "&str / TextStyle",
        "",
        "What is shown while it is empty, and the type it is shown in.",
      ),
      (
        "text_align",
        "TextAlign",
        "Left",
        "Where its lines stand across its width.",
      ),
      (
        "caret_color / caret_mode",
        "TextColor / CaretMode",
        "Blinking",
        "The caret's color, and whether it blinks.",
      ),
      (
        "mask",
        "()",
        "",
        "Each character drawn as a dot, as a password is; the value is the text typed.",
      ),
      (
        "select_on_focus",
        "bool",
        "false",
        "All of its text selected as it takes the focus, so that what is typed takes its place.",
      ),
      (
        "controller",
        "TextInputController",
        "",
        "What edits it from outside: reads and sets its selection, replaces it, undoes and redoes, as one step of \
         its history each.",
      ),
      (
        "tokens",
        "InputTokens",
        "",
        "Pills over ranges of its text, which the app finds again at each render. An atomic one is one thing to \
         the caret.",
      ),
      (
        "ghost",
        "InputGhost",
        "",
        "A suggestion after the end of its text while the caret is there. It is not part of the value.",
      ),
      (
        "squiggles",
        "InputSquiggles",
        "",
        "Wavy lines under ranges of its text, in the destructive color unless told.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Engine", "Text Input"],
        title: "Text Input",
        description: "The engine's field of text: bound to a Signal<String>, on one line or many, in the type and \
                      the frame it is given. The design system's Input and Textarea are built on it.",
        import: Some("use lurq::components::{TextInput, TextInputController};"),
        status: Some("Stable"),
        since: None,
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "A field bound to the page's signal, in the frame of the theme's fields. Type in it: each change of the \
           signal is counted. Choose how it lays its lines out, its size and where they stand.",
          playground,
        ),
        Section::new(
          "Single line",
          "Two fields on one line each, the first holding a name and the second empty with what to type in it. \
           Under them is what their signals hold.",
          self.frame.demo(
            0,
            self.single_line(&theme),
            "TextInput::new(name).single_line()",
            &[
              "// name, email: Signal<String>",
              "TextInput::new(name)",
              "    .placeholder(\"Display name\")",
              "    .single_line()",
              "    .width(Dimension::Pct(100.0))",
              "    .padding_horizontal(12.0)",
              "    .padding_vertical(6.0)",
              "    .background(PaletteColor::Background)",
              "    .border(Border::inside(1.0, PaletteColor::Input))",
              "    .focused(|style| style.border_inside(2.0, PaletteColor::Ring))",
              "",
              "TextInput::new(email).placeholder(\"name@example.com\").single_line()",
            ],
          ),
        ),
        Section::new(
          "In a type of its own",
          "A hash on one line and notes in a box of a fixed height, in the family of code at 13, with a caret and \
           a placeholder of their own colors. Notes longer than the box scroll in it.",
          self.frame.demo(
            1,
            self.styled(&theme),
            "TextInput::styled(hash, style).placeholder_style(hint)",
            &[
              "let style = TextStyle {",
              "    font_size: 13.0,",
              "    weight: FontWeight::Medium,",
              "    ..theme.typography_style(TypographyStyle::Mono)",
              "};",
              "let hint = TextStyle { color: theme.palette_color(PaletteColor::MutedForeground), ..style.clone() };",
              "TextInput::styled(hash, style.clone())",
              "    .placeholder(\"a3f1b2c4d5e691cc...\")",
              "    .placeholder_style(hint.clone())",
              "    .caret_color(PaletteColor::Info)",
              "    .single_line()",
              "    .height(40.0)",
              "",
              "TextInput::styled(notes, style).placeholder_style(hint).multiline().height(82.0)",
            ],
          ),
        ),
        Section::new(
          "Scrolling and growing",
          "A line longer than its field scrolls to follow the caret. A textarea grows with its lines, and one with \
           rows(3, 4) is three rows high at least and four at most, then scrolls.",
          self.frame.demo(
            2,
            self.growing(&theme),
            ".rows(3, 4)",
            &[
              "TextInput::new(scroll).single_line().width(220.0).height(38.0)",
              "TextInput::new(notes).placeholder(\"Notes\").textarea()",
              "TextInput::new(rows).placeholder(\"Capped rows\").rows(3, 4)",
            ],
          ),
        ),
        Section::new(
          "A controller",
          "A controller edits the field from outside it: it reads the selection, sets it, puts text in its place \
           and undoes, each edit one step of the field's history, as typing is. Put the caret somewhere, press \
           Insert, then Undo.",
          self.frame.demo(
            3,
            self.controlled(&theme),
            "controller.replace_selection(\"{{task.title}}\")",
            &[
              "let controller = TextInputController::new();",
              "TextInput::new(prompt)",
              "    .key(\"prompt\")",
              "    .controller(controller.clone())",
              "    .multiline()",
              "",
              "controller.replace_selection(\"{{task.title}}\")?;",
              "controller.select(0, prompt.get().len())?;",
              "if controller.can_undo() { controller.undo()?; }",
              "let selection = controller.selection()?; // caret and anchor, in bytes",
            ],
          ),
        ),
        Section::new(
          "Tokens",
          "The page finds the {{...}} expressions in the text at each render and draws them as pills: a variable it \
           knows in one color, any other in the destructive one. A token is text: the caret goes into it, unless \
           it is atomic.",
          self.frame.demo(
            4,
            self.tokens(&theme),
            "TextInput::new(source).tokens(InputTokens::new(&raw, tokens)?)",
            &[
              "let raw = source.get();",
              "let style = InputTokenStyle::new(PaletteColor::Info, PaletteColor::InfoMuted)",
              "    .padding(4.0)",
              "    .radius(RadiusSize::Sm);",
              "let tokens = expressions(&raw)",
              "    .map(|range| InputToken::new(range, style.clone()).atomic(atomic))",
              "    .collect();",
              "TextInput::new(source)",
              "    .tokens(InputTokens::new(&raw, tokens)?)",
              "    .multiline()",
            ],
          ),
        ),
        Section::new(
          "A suggestion and wavy lines",
          "What the app draws over a field's text that is no part of it: the rest of a command after what is \
           typed, which the app puts in when it is taken, and wavy lines under the words its checker marks.",
          self.frame.demo(
            5,
            self.marks(&theme),
            ".ghost(InputGhost::new(rest)).squiggles(InputSquiggles::new(ranges))",
            &[
              "TextInput::new(command)",
              "    .controller(controller.clone())",
              "    .single_line()",
              "    .ghost(InputGhost::new(completion(&command.get())).opacity(0.7))",
              "",
              "// Taking it: the caret to the end, and the rest in its place.",
              "controller.select(end, end)?;",
              "controller.replace_selection(rest)?;",
              "",
              "TextInput::new(note)",
              "    .multiline()",
              "    .squiggles(InputSquiggles::new(misspelled(&note.get())))",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a text input is given.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
