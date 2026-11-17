//! A block of code: its lines in the family of code, colored by what each word is, under a bar with its name and a
//! button that copies it.

use std::{sync::Arc, time::Duration};

use super::{
  icons, names,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  scroll_area::{ScrollArea, ScrollAreaOrientation, ScrollAreaPart},
  shared::{fading, glyph, one_line, pressable},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    component::Component,
    ctx::{Ctx, Timeout},
    events::{MouseEvent, TextInputEvent},
    theme::{BorderSize, IconSize, PaletteColor, RadiusSize, Theme, TypographyStyle},
    window::Window,
  },
  code::{CodeTokenKind, tokens},
  components::{Column, InputToken, InputTokenStyle, InputTokens, LineGutter, Row, TextInput},
  core::Signal,
  layout::{
    Alignment,
    layout_kind::Justify,
    quad::RichTextSpan,
    text_style::{TextAlign, TextStyle},
  },
  node::{
    Element, Node, TextColor,
    border::Border,
    dimension::Dimension,
    gradient::{Gradient, GradientStop},
  },
  ui::{Length, PartStyle},
};

/// The parts of a code block a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CodeBlockPart {
  /// The block: its fill, its edge and its corners.
  Root,
  /// The bar over the code, with its title and the copy button.
  Header,
  /// The bar's text: the title, or the language.
  Title,
  /// The button that copies the code.
  Copy,
  /// Its glyph: a copy, or a check once it has copied.
  CopyIcon,
  /// The room the code scrolls in.
  Body,
  /// The code, in the color of what is none of the tokens below.
  Code,
  /// The numbers of the lines.
  LineNumber,
  /// The band behind a line that is picked out.
  Marked,
  /// The words of the language: `fn`, `let`.
  Keyword,
  /// Strings.
  String,
  /// Numbers.
  Number,
  /// Comments.
  Comment,
  /// The names of types, traits and modules: `App`, `lurq::`.
  Type,
  /// Brackets, operators and the other marks between names.
  Punctuation,
}

names!(CodeBlockPart {
  Root => "root",
  Header => "header",
  Title => "title",
  Copy => "copy",
  CopyIcon => "copy_icon",
  Body => "body",
  Code => "code",
  LineNumber => "line_number",
  Marked => "marked",
  Keyword => "keyword",
  String => "string",
  Number => "number",
  Comment => "comment",
  Type => "type",
  Punctuation => "punctuation",
});

/// The states of a code block a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CodeBlockState {
  /// It has its bar: a title, a language or a copy button.
  Header,
  /// Its lines are numbered.
  LineNumbers,
  /// Its lines wrap at its width, where they scroll sideways otherwise.
  Wrap,
  /// It has just copied its code: its button shows a check.
  Copied,
  /// It is drawn in a frame of its own: a fill, an edge and round corners.
  Framed,
}

names!(CodeBlockState {
  Header => "header",
  LineNumbers => "line_numbers",
  Wrap => "wrap",
  Copied => "copied",
  Framed => "framed",
});

/// What builds the nodes of a code block: a theme's in place of the default one.
pub type CodeBlockRenderer = Arc<dyn Fn(CodeBlockView) -> Element + Send + Sync>;

/// How long the copy button shows that it copied, mounted.
pub const CODE_BLOCK_COPIED_FOR: Duration = Duration::from_secs(2);

/// What hears the code copied.
type CopyHandler = Arc<dyn Fn(&str) + Send + Sync>;

/// A block of code: a snippet in a page of docs, a command to run, a file's lines.
///
/// ```
/// use lurq::ui::CodeBlock;
///
/// let main = CodeBlock::new("fn main() {\n  let app = App::new();\n}")
///   .language("rust")
///   .title("main.rs")
///   .line_numbers(true)
///   .highlight_lines([2])
///   .on_copy(|code| println!("copied {} bytes", code.len()));
/// ```
///
/// Its code is in the family of code, each word in the color of what it is where the block knows the language
/// ([`crate::code`]: Rust), on one line each that scrolls sideways (or wraps, with [`wrap`](Self::wrap)). Its bar
/// names it (its title, or its language) and copies it whole. A person selects its text with the pointer, across its
/// lines. To what reads the tree it is `code` with its language, called by its label, its title or its language.
#[derive(Clone)]
pub struct CodeBlock {
  code: Arc<str>,
  language: Option<Arc<str>>,
  title: Option<Arc<str>>,
  label: Option<Arc<str>>,
  line_numbers: bool,
  first_line: usize,
  highlighted: Vec<usize>,
  wrap: bool,
  min_height: Option<Length>,
  max_height: Option<Length>,
  copy_button: bool,
  copy_label: Arc<str>,
  copied_text: Arc<str>,
  on_copy: Vec<CopyHandler>,
  selectable: bool,
  framed: bool,
  row_height: f32,
  header: Option<bool>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

/// How high a line of code is unless told: 17, the family of code's own line at 13, and 4 between two, as the
/// board draws code.
pub const CODE_BLOCK_ROW_HEIGHT: f32 = 21.0;

impl CodeBlock {
  /// A block that shows `code`; a newline at its end is not a line of its own.
  pub fn new(code: &str) -> Self {
    Self {
      code: Arc::from(code.strip_suffix('\n').unwrap_or(code)),
      language: None,
      title: None,
      label: None,
      line_numbers: false,
      first_line: 1,
      highlighted: Vec::new(),
      wrap: false,
      min_height: None,
      max_height: None,
      copy_button: true,
      copy_label: super::current_locale().editing.copy_code.clone(),
      copied_text: super::current_locale().editing.copied.clone(),
      on_copy: Vec::new(),
      selectable: true,
      framed: true,
      row_height: CODE_BLOCK_ROW_HEIGHT,
      header: None,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// The language of the code (`rust`, `toml`): what colors its words, where the block knows it, what the bar
  /// shows where the block has no title, and what tooling is told.
  pub fn language(mut self, language: &str) -> Self {
    self.language = Some(Arc::from(language));
    self
  }

  /// What the bar shows in place of the language: a file's name. On one line, ending in an ellipsis where the bar
  /// has no room.
  pub fn title(mut self, title: &str) -> Self {
    self.title = Some(Arc::from(title));
    self
  }

  /// Its name to what reads the tree: its title, or its language, unless told.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// Numbers the lines, in a column at their start that a selection does not take. Off by default.
  pub fn line_numbers(mut self, line_numbers: bool) -> Self {
    self.line_numbers = line_numbers;
    self
  }

  /// The number of the first line, where the code is a part of a file: 1 by default.
  pub fn first_line(mut self, number: usize) -> Self {
    self.first_line = number;
    self
  }

  /// Picks lines out, by their numbers (from [`first_line`](Self::first_line)): a band behind each across the
  /// block, its number in the color of the code.
  pub fn highlight_lines(mut self, lines: impl IntoIterator<Item = usize>) -> Self {
    self.highlighted.extend(lines);
    self
  }

  /// Wraps the lines at the block's width, where they are one line each that scrolls sideways otherwise.
  pub fn wrap(mut self, wrap: bool) -> Self {
    self.wrap = wrap;
    self
  }

  /// The least height of the block, its bar with it: a short snippet stands in a room this high.
  pub fn min_height(mut self, height: impl Into<Length>) -> Self {
    self.min_height = Some(height.into());
    self
  }

  /// The greatest height of the block, its bar with it: the code scrolls down past it.
  pub fn max_height(mut self, height: impl Into<Length>) -> Self {
    self.max_height = Some(height.into());
    self
  }

  /// Whether the bar has the button that copies the code: shown unless told otherwise.
  pub fn copy_button(mut self, copy_button: bool) -> Self {
    self.copy_button = copy_button;
    self
  }

  /// The copy button's name ("Copy code").
  pub fn copy_label(mut self, label: &str) -> Self {
    self.copy_label = Arc::from(label);
    self
  }

  /// The copy button's name while it shows that it copied ("Copied"), mounted.
  pub fn copied_text(mut self, text: &str) -> Self {
    self.copied_text = Arc::from(text);
    self
  }

  /// Told the code a press of the copy button put on the clipboard.
  pub fn on_copy(mut self, handler: impl Fn(&str) + Send + Sync + 'static) -> Self {
    self.on_copy.push(Arc::new(handler));
    self
  }

  /// Whether a person selects the code: on unless told otherwise. Selectable, the code is a stop of Tab, read
  /// only, whose arrows move a caret through it, Shift with them selecting, Ctrl+A taking it all and Ctrl+C copying
  /// what is selected; the pointer selects it across its lines. Off, it is text that what holds it may make
  /// selectable (a Markdown document's own selection).
  pub fn selectable(mut self, selectable: bool) -> Self {
    self.selectable = selectable;
    self
  }

  /// Whether it is drawn in a frame of its own, as it is unless told otherwise: a fill, an edge and round corners.
  /// Off, the bar and the code stand on what holds them (a card of the app's that frames them).
  pub fn framed(mut self, framed: bool) -> Self {
    self.framed = framed;
    self
  }

  /// Whether it has its bar over the code. Unless told, it has it where it has a title, a language or a copy
  /// button; told not to, the language still colors the code and is told to tooling.
  pub fn header(mut self, header: bool) -> Self {
    self.header = Some(header);
    self
  }

  /// How high each line of the code is, in pixels: [`CODE_BLOCK_ROW_HEIGHT`] unless told. The code keeps its
  /// padding of the board around the lines.
  pub fn row_height(mut self, height: f32) -> Self {
    self.row_height = height.max(1.0);
    self
  }

  /// Puts it in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this block.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the copy button as it is under the pointer or held down, with nothing pointing at it: for a gallery or
  /// a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// Its id. Its bar's title is `{id}-title`, its copy button `{id}-copy`, its code `{id}-code`.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Mounted where it is declared: the copy button shows a check, named [`copied_text`](Self::copied_text), for
  /// [`CODE_BLOCK_COPIED_FOR`] after it copied.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    ctx.mount::<CodeBlockHost>(CodeBlockProps(Arc::new(self)))
  }

  /// Builds the still form in `theme`: it copies, and does not show that it did. Converting it into an element does
  /// this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    self.build_with(theme, None)
  }

  fn build_with(self, theme: &Theme, copied: Option<Copied>) -> Element {
    let has_header = self
      .header
      .unwrap_or(self.copy_button || self.title.is_some() || self.language.is_some());
    let is_copied = copied.as_ref().is_some_and(|copied| copied.on.get());
    let mut case = Case::new()
      .breakpoint(super::current_breakpoint())
      .state(CodeBlockState::Header, has_header)
      .state(CodeBlockState::LineNumbers, self.line_numbers)
      .state(CodeBlockState::Wrap, self.wrap)
      .state(CodeBlockState::Copied, is_copied)
      .state(CodeBlockState::Framed, self.framed);
    for state in &self.states {
      case = case.state(state.clone(), true);
    }
    let view = CodeBlockView {
      code: self.code,
      language: self.language,
      title: self.title,
      label: self.label,
      line_numbers: self.line_numbers,
      first_line: self.first_line,
      highlighted: self.highlighted,
      wrap: self.wrap,
      min_height: self.min_height,
      max_height: self.max_height,
      copy_button: self.copy_button,
      copy_label: self.copy_label,
      copied_text: self.copied_text,
      copied: is_copied,
      on_copy: self.on_copy,
      selectable: self.selectable,
      framed: self.framed,
      row_height: self.row_height,
      has_header,
      recipe: theme.resolve::<CodeBlock>(&case, Some(&self.rules)),
      forced: self.forced,
      window: super::theme::current_window(),
      shown_copied: copied,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<CodeBlock>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<CodeBlock> for Element {
  fn from(block: CodeBlock) -> Self {
    block.build(&current_theme())
  }
}

/// What a mounted block keeps: whether it has just copied, and what turns that off.
#[derive(Clone)]
struct Copied {
  on: Signal<bool>,
  off: Timeout,
}

/// A code block as a renderer sees it: its code, how it is shown, and the style of its parts.
pub struct CodeBlockView {
  pub code: Arc<str>,
  pub language: Option<Arc<str>>,
  pub title: Option<Arc<str>>,
  pub label: Option<Arc<str>>,
  pub line_numbers: bool,
  /// The number of the first line.
  pub first_line: usize,
  /// The numbers of the lines picked out.
  pub highlighted: Vec<usize>,
  pub wrap: bool,
  pub min_height: Option<Length>,
  pub max_height: Option<Length>,
  pub copy_button: bool,
  pub copy_label: Arc<str>,
  pub copied_text: Arc<str>,
  /// Whether it has just copied, mounted.
  pub copied: bool,
  on_copy: Vec<CopyHandler>,
  pub selectable: bool,
  pub framed: bool,
  /// How high each line of the code is.
  pub row_height: f32,
  /// Whether it has its bar over the code.
  pub has_header: bool,
  /// The style of each part of this block.
  pub recipe: ResolvedRecipe,
  forced: Option<Interaction>,
  /// The window that puts the code on the clipboard: the one it is built in.
  window: Option<Window>,
  shown_copied: Option<Copied>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl CodeBlockView {
  /// The style of a part, as this block is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// Its name to what reads the tree: its label, its title or its language, or "Code".
  pub fn name(&self) -> Arc<str> {
    self
      .label
      .clone()
      .or_else(|| self.title.clone())
      .or_else(|| self.language.clone())
      .unwrap_or_else(|| super::current_locale().editing.code.clone())
  }

  /// What the bar shows: the title, or the language.
  pub fn heading(&self) -> Option<Arc<str>> {
    self.title.clone().or_else(|| self.language.clone())
  }

  /// The lines of the code.
  pub fn lines(&self) -> Vec<&str> {
    self.code.split('\n').collect()
  }

  /// The style of the code's text, and the color of each kind of token in it, as the recipe gives them.
  fn code_style(&self) -> TextStyle {
    let part = self.part(CodeBlockPart::Code);
    let mut style = part.rest().text.resolve(self.theme());
    style.color = self.color_of(&part, PaletteColor::Foreground);
    style.line_height = self.row_height / style.font_size.max(1.0);
    style
  }

  /// The padding around the lines: the recipe's, less above and below by what a row takes past the family's own
  /// line, so that the first line's glyphs stand where they would on a line of its own.
  fn code_padding(&self) -> (f32, f32, f32, f32) {
    let rest = self.part(CodeBlockPart::Code).rest();
    let style = self.code_style();
    let natural = style.font_size * self.theme().typography_style(TypographyStyle::Mono).line_height;
    let leading = ((self.row_height - natural) / 2.0).max(0.0);
    let start = self.px(rest.padding_start.or(rest.padding_x));
    let end = self.px(rest.padding_end.or(rest.padding_x));
    let top = (self.px(rest.padding_top.or(rest.padding_y)) - leading).max(0.0);
    let bottom = (self.px(rest.padding_bottom.or(rest.padding_y)) - leading).max(0.0);
    (start, end, top, bottom)
  }

  /// The code as one text a person selects with the keyboard and the pointer: a read-only input of many lines, its
  /// words in their colors, its numbers and its bands in its gutter where it lays its lines out.
  fn code_input(&self) -> Element {
    let style = self.code_style();
    let (start, end, top, bottom) = self.code_padding();
    let number_part = self.part(CodeBlockPart::LineNumber);
    let number_rest = number_part.rest();
    let gap = self.px(number_rest.padding_end.or(number_rest.padding_x));
    let digits = self.number(self.lines().len().saturating_sub(1)).len() as f32;
    // The figures of the family of code are as wide as each other: about six tenths of its size each.
    let numbers = if self.line_numbers {
      digits * style.font_size * 0.6 + gap
    } else {
      0.0
    };
    let mut number_style = number_part.rest().text.resolve(self.theme());
    number_style.line_height = style.line_height;
    let mut gutter = LineGutter::new(number_style)
      .first_line(self.first_line)
      .numbers(self.line_numbers)
      .gap(gap)
      .color(number_rest.foreground.unwrap_or(PaletteColor::MutedForeground.into()))
      .marked_color(
        self
          .part(CodeBlockPart::Code)
          .rest()
          .foreground
          .unwrap_or(PaletteColor::Foreground.into()),
      );
    if !self.highlighted.is_empty() {
      let band = match self.part(CodeBlockPart::Marked).rest().background {
        Some(crate::node::BackgroundColor::Palette(role)) => TextColor::Palette(role),
        Some(crate::node::BackgroundColor::Color(color)) => TextColor::Color(color),
        _ => TextColor::Palette(PaletteColor::Highlight),
      };
      gutter = gutter.marked(self.highlighted.iter().copied(), band);
    }
    let mut input = TextInput::styled(Signal::new(self.code.to_string()), style)
      .multiline()
      .wrap(self.wrap)
      .tokens(self.input_tokens())
      .line_gutter(gutter)
      .padding_left(start + numbers)
      .padding_right(end)
      .padding_top(top)
      .padding_bottom(bottom)
      .on_input(|event: TextInputEvent| event.prevent_default())
      // A stop of Tab wherever it is, as a control of the design system is.
      .tab_index(0)
      .describe("a11y_role", "code")
      .describe("a11y_name", self.name())
      .describe("a11y_readonly", "true");
    if let Some(language) = &self.language {
      input = input.describe("a11y_language", language.clone());
    }
    input = if self.wrap {
      input.width(Dimension::Pct(100.0))
    } else {
      // As wide as the room the code scrolls in, or as its longest line where that is wider.
      input.flex_full(1.0, 0.0, None)
    };
    if let Some(id) = &self.id {
      input = input.id(format!("{id}-code"));
    }
    input.into()
  }

  /// The code's words in the colors of their kinds, as the input draws them: a run of one kind on one line each.
  fn input_tokens(&self) -> InputTokens {
    let language = self.language.as_deref().unwrap_or("");
    let mut runs = Vec::new();
    for token in tokens(&self.code, language) {
      let Some(part) = token_part(token.kind) else {
        continue;
      };
      let color = self
        .part(part)
        .rest()
        .foreground
        .unwrap_or(PaletteColor::Foreground.into());
      let style = InputTokenStyle::new(color, crate::node::color::Color::new(0, 0, 0, 0));
      // A token of the input stands on one line: one of many lines is one run a line.
      let mut at = token.range.start;
      for piece in self.code[token.range.clone()].split_inclusive('\n') {
        let text = piece.trim_end_matches('\n');
        if !text.is_empty() {
          runs.push(InputToken::new(at..at + text.len(), style.clone()));
        }
        at += piece.len();
      }
    }
    InputTokens::new(&self.code, runs)
      .unwrap_or_else(|_| InputTokens::new(&self.code, Vec::new()).expect("no tokens are always valid"))
  }

  fn color_of(&self, part: &ResolvedPart, otherwise: PaletteColor) -> crate::node::color::Color {
    match part.rest().foreground {
      Some(TextColor::Color(color)) => color,
      Some(TextColor::Palette(role)) => self.theme().palette_color(role),
      None => self.theme().palette_color(otherwise),
    }
  }

  /// The code in spans of the colors of its tokens: of `line` alone, or of the whole code.
  pub fn spans(&self, code: &str) -> Vec<RichTextSpan> {
    let base = self.code_style();
    let styled = |part: CodeBlockPart| TextStyle {
      color: self.color_of(&self.part(part), PaletteColor::Foreground),
      ..base.clone()
    };
    let kinds = [
      (CodeTokenKind::Keyword, styled(CodeBlockPart::Keyword)),
      (CodeTokenKind::Type, styled(CodeBlockPart::Type)),
      (CodeTokenKind::String, styled(CodeBlockPart::String)),
      (CodeTokenKind::Number, styled(CodeBlockPart::Number)),
      (CodeTokenKind::Comment, styled(CodeBlockPart::Comment)),
      (CodeTokenKind::Punctuation, styled(CodeBlockPart::Punctuation)),
    ];
    let language = self.language.as_deref().unwrap_or("");
    let mut spans: Vec<RichTextSpan> = tokens(code, language)
      .into_iter()
      .map(|token| RichTextSpan {
        text: code[token.range].to_owned(),
        style: kinds
          .iter()
          .find(|(kind, _)| *kind == token.kind)
          .map_or_else(|| base.clone(), |(_, style)| style.clone()),
      })
      .collect();
    if spans.is_empty() {
      spans.push(RichTextSpan {
        text: String::new(),
        style: base,
      });
    }
    spans
  }

  /// What a press of the copy button does: the code on the clipboard, the app told, and the check shown.
  fn copy(&self) -> impl Fn(MouseEvent) + Send + Sync + 'static {
    let (code, window, handlers, copied) = (
      self.code.clone(),
      self.window.clone(),
      self.on_copy.clone(),
      self.shown_copied.clone(),
    );
    move |_| {
      if let Some(window) = &window {
        window.request_copy(code.to_string());
      }
      for handler in &handlers {
        handler(&code);
      }
      if let Some(copied) = &copied {
        copied.on.set(true);
        copied.off.restart();
      }
    }
  }

  /// The bar: the title, or the language, at its start and the copy button at its end.
  pub fn header(&self) -> Option<Element> {
    let heading = self.heading();
    if !self.has_header {
      return None;
    }
    let mut bar = Row::new()
      .align_items(Alignment::Center)
      .justify(Justify::SpaceBetween)
      .part(&self.part(CodeBlockPart::Header));
    let mut title = Row::new().flex_shrink(1.0).min_width(0.0);
    if let Some(heading) = &heading {
      title = title.child(one_line(&self.part(CodeBlockPart::Title), heading));
    }
    if let Some(id) = &self.id {
      title = title.id(format!("{id}-title"));
    }
    bar = bar.child(title);
    if self.copy_button {
      bar = bar.child(self.copy_control());
    }
    Some(bar.into())
  }

  /// The copy button: a copy glyph, a check once it copied, named by `copy_label` or `copied_text`.
  pub fn copy_control(&self) -> Element {
    let part = self.part(CodeBlockPart::Copy).forced(self.forced);
    let name = if self.copied {
      self.copied_text.clone()
    } else {
      self.copy_label.clone()
    };
    let icon = if self.copied { icons::CHECK } else { icons::COPY };
    let mut button = pressable(&part, false)
      .describe("a11y_role", "button")
      .describe("a11y_name", name)
      .child(glyph(&self.part(CodeBlockPart::CopyIcon), Name::from(icon), false))
      .on_click(self.copy());
    if let Some(id) = &self.id {
      button = button.id(format!("{id}-copy"));
    }
    button.into()
  }

  /// The pitch of a line: the code's size on its line.
  fn line_pitch(&self) -> f32 {
    let style = self.code_style();
    style.font_size * style.line_height
  }

  /// The number of the line at `index`, as text.
  fn number(&self, index: usize) -> String {
    (self.first_line + index).to_string()
  }

  fn is_highlighted(&self, index: usize) -> bool {
    self.highlighted.contains(&(self.first_line + index))
  }

  /// The code that does not wrap: one text of all its lines, which a selection runs across, beside a column of
  /// their numbers, over the bands of the lines picked out.
  fn unwrapped(&self) -> Element {
    let lines = self.lines();
    let code_part = self.part(CodeBlockPart::Code);
    let mut code = Node::rich_text(self.spans(&self.code)).text_wrap(false);
    if self.selectable {
      code = code.selectable(true);
    }
    let mut code = Element::from_node(code);
    if let Some(id) = &self.id {
      code = Row::new().id(format!("{id}-code")).child(code).into();
    }
    let (_, _, pad_top, pad_bottom) = self.code_padding();
    let mut row = Row::new()
      .align_items(Alignment::Start)
      .part(&code_part)
      .padding_top(pad_top)
      .padding_bottom(pad_bottom);
    if self.line_numbers {
      let mut column = Column::new()
        .align_items(Alignment::End)
        .part(&self.part(CodeBlockPart::LineNumber));
      // One text of the numbers, so that they stand on the code's lines; a picked line's number is in the code's
      // color.
      let spans = (0..lines.len())
        .map(|index| {
          let mut text = self.number(index);
          if index + 1 < lines.len() {
            text.push('\n');
          }
          RichTextSpan {
            text,
            style: self.number_style(Some(self.is_highlighted(index))),
          }
        })
        .collect::<Vec<_>>();
      column = column.child(Element::from_node(
        Node::rich_text(spans).text_wrap(false).text_align(TextAlign::Right),
      ));
      row = row.child(column);
    }
    row = row.child(code);
    if self.highlighted.is_empty() {
      return row.into();
    }
    // The bands behind the lines picked out: hard stops of a gradient down the room the lines are in, which runs
    // across the room the code scrolls in, or across the code where it is wider.
    let pitch = self.line_pitch();
    let (top, bottom) = (pad_top, pad_bottom);
    let height = top + bottom + pitch * lines.len() as f32;
    let band = self
      .part(CodeBlockPart::Marked)
      .rest()
      .background
      .unwrap_or(PaletteColor::Highlight.into());
    let clear = crate::node::BackgroundColor::Color(crate::node::color::Color::new(0, 0, 0, 0));
    let mut stops = Vec::new();
    for index in (0..lines.len()).filter(|index| self.is_highlighted(*index)) {
      let (from, to) = (
        (top + pitch * index as f32) / height,
        (top + pitch * (index + 1) as f32) / height,
      );
      stops.extend([
        GradientStop::at(clear.clone(), from),
        GradientStop::at(band.clone(), from),
        GradientStop::at(band.clone(), to),
        GradientStop::at(clear.clone(), to),
      ]);
    }
    row
      .flex_full(1.0, 0.0, None)
      .background_gradient(Gradient::linear(180.0, stops))
      .into()
  }

  /// A spacing of the recipe, in pixels.
  fn px(&self, spacing: Option<crate::node::SpacingValue>) -> f32 {
    match spacing {
      Some(crate::node::SpacingValue::Dimension(Dimension::Px(px))) => px,
      Some(crate::node::SpacingValue::Theme(size)) => match self.theme().spacing_value(size) {
        Dimension::Px(px) => px,
        _ => 0.0,
      },
      _ => 0.0,
    }
  }

  /// The style of a line's number: the code's own color where the line is picked out.
  fn number_style(&self, highlighted: Option<bool>) -> TextStyle {
    let part = self.part(CodeBlockPart::LineNumber);
    let mut style = part.rest().text.resolve(self.theme());
    style.line_height = self.code_style().line_height;
    style.color = if highlighted == Some(true) {
      self.color_of(&self.part(CodeBlockPart::Code), PaletteColor::Foreground)
    } else {
      self.color_of(&part, PaletteColor::MutedForeground)
    };
    style
  }

  /// The code that wraps: a row for each line, its number at its start and its text wrapping beside it.
  fn wrapped(&self) -> Element {
    let lines = self.lines();
    let code_part = self.part(CodeBlockPart::Code);
    let marked = self.part(CodeBlockPart::Marked);
    // The column of the numbers is as wide as the longest number in the family of code, whose figures are as wide
    // as each other: about six tenths of its size each.
    let digits = self.number(lines.len().saturating_sub(1)).len() as f32;
    let number_part = self.part(CodeBlockPart::LineNumber);
    let numbers_width = digits * self.code_style().font_size * 0.6
      + self.px(number_part.rest().padding_end.or(number_part.rest().padding_x));
    // The room's padding across is each line's, so that a picked line's band runs across the block.
    let rest = code_part.rest();
    let (start, end) = (
      self.px(rest.padding_start.or(rest.padding_x)),
      self.px(rest.padding_end.or(rest.padding_x)),
    );
    let (_, _, pad_top, pad_bottom) = self.code_padding();
    let mut column = Column::new()
      .width(Dimension::Pct(100.0))
      .part(&code_part)
      .padding_left(0.0)
      .padding_right(0.0)
      .padding_top(pad_top)
      .padding_bottom(pad_bottom);
    for (index, line) in lines.iter().enumerate() {
      let mut row = Row::new()
        .width(Dimension::Pct(100.0))
        .align_items(Alignment::Start)
        .padding_left(start)
        .padding_right(end);
      if self.is_highlighted(index) {
        row = row.part(&marked);
      }
      if self.line_numbers {
        let number = Element::from_node(
          Node::rich_text(vec![RichTextSpan {
            text: self.number(index),
            style: self.number_style(Some(self.is_highlighted(index))),
          }])
          .text_wrap(false)
          .text_align(TextAlign::Right),
        );
        row = row.child(
          Column::new()
            .width(numbers_width)
            .flex_shrink(0.0)
            .align_items(Alignment::End)
            .part(&number_part)
            .child(number),
        );
      }
      let mut text = Node::rich_text(self.spans(line));
      if self.selectable {
        text = text.selectable(true);
      }
      row = row.child(
        Column::new()
          .flex_full(1.0, 1.0, Some(0.0))
          .min_width(0.0)
          .child(Element::from_node(text).map_node(|node| node.width(Dimension::Pct(100.0)))),
      );
      column = column.child(row);
    }
    if let Some(id) = &self.id {
      column = column.id(format!("{id}-code"));
    }
    column.into()
  }

  /// The room the code scrolls in: sideways where it does not wrap, down past the greatest height.
  pub fn body(&self) -> Element {
    let content = match (self.selectable, self.wrap) {
      (true, _) => self.code_input(),
      (false, true) => self.wrapped(),
      (false, false) => self.unwrapped(),
    };
    let orientation = match (self.wrap, self.max_height.is_some()) {
      (true, _) => ScrollAreaOrientation::Vertical,
      (false, false) => ScrollAreaOrientation::Horizontal,
      (false, true) => ScrollAreaOrientation::Both,
    };
    let header = if self.has_header {
      self.part(CodeBlockPart::Header).rest().height
    } else {
      None
    };
    let header = match header {
      Some(Length::Px(height)) => height,
      _ => 0.0,
    };
    let mut area = ScrollArea::new()
      .width(Length::Fill)
      .orientation(orientation)
      .fill_width(true)
      .child(content);
    // The code that takes the focus itself is the stop of Tab; text that does not is reached by the area's keys.
    if !self.selectable {
      area = area.label(&self.name());
    }
    if let Some(Length::Px(max)) = self.max_height {
      area = area.max_height((max - header).max(0.0));
    }
    let mut body = Column::new()
      .width(Dimension::Pct(100.0))
      .part(&self.part(CodeBlockPart::Body));
    if let Some(Length::Px(min)) = self.min_height {
      // The room the code scrolls in is as high as the block's least height too, and not only the body round it:
      // a short snippet's sideways bar is at the block's end, not under its last line with empty room below.
      let least = (min - header).max(0.0);
      area = area.style(ScrollAreaPart::Root, move |style| style.min_height(least));
      body = body.min_height(least);
    }
    body.child(area).into()
  }

  /// The block, with nothing in it yet: `code` to what reads the tree, with its language.
  pub fn root(&self) -> Column {
    let mut root = Column::new()
      .align_items(Alignment::Stretch)
      .part(&self.part(CodeBlockPart::Root))
      .describe("a11y_role", "code")
      .describe("a11y_name", self.name());
    if let Some(language) = &self.language {
      root = root.describe("a11y_language", language.clone());
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root
  }

  /// How the default renderer builds a code block: its bar over its code.
  pub fn render(self) -> Element {
    let mut root = self.root();
    if let Some(header) = self.header() {
      root = root.child(header);
    }
    root.child(self.body()).into()
  }
}

/// The part whose foreground colors a kind of token, or none for plain code.
fn token_part(kind: CodeTokenKind) -> Option<CodeBlockPart> {
  match kind {
    CodeTokenKind::Plain => None,
    CodeTokenKind::Keyword => Some(CodeBlockPart::Keyword),
    CodeTokenKind::Type => Some(CodeBlockPart::Type),
    CodeTokenKind::String => Some(CodeBlockPart::String),
    CodeTokenKind::Number => Some(CodeBlockPart::Number),
    CodeTokenKind::Comment => Some(CodeBlockPart::Comment),
    CodeTokenKind::Punctuation => Some(CodeBlockPart::Punctuation),
  }
}

impl Themed for CodeBlock {
  const NAME: &'static str = "code_block";
  type Renderer = CodeBlockRenderer;

  /// The design's code block (`pencil/lurq-design-system.pen`, Code Block `kyzrV`, in Data and in the Markdown
  /// document): on `Background` in an edge of 1 in `Border` with the large radius; a bar of 36 on `Muted` with a line
  /// of 1 in `Border` under it, padded 14 at its start and 6 at its end, its title 11 in the family of code in
  /// `MutedForeground`, and a ghost button of 28 with a copy glyph of 16; the code 13 in the family of code in
  /// `Foreground`, padded 14, its lines 4 apart, the words of the language in `Info`.
  fn recipe() -> Recipe {
    use CodeBlockPart::*;
    use PaletteColor::{
      Background, Border as Edge, Foreground, Highlight, HighlightForeground, Info, Muted, MutedForeground, Success,
      Warning,
    };
    // A line is `row_height` high, 21 unless told: the code's own line of 17 and 4 between two. The room around the
    // lines is 14, less above and below by what a row takes past the code's own line.
    Recipe::new()
      .base(Root, |s| s.width(Length::Fill).foreground(Foreground))
      .state(CodeBlockState::Framed, Root, |s| {
        s.background(Background)
          .border(Border::inside(BorderSize::Sm, Edge))
          .radius(RadiusSize::Lg)
      })
      .base(Header, |s| {
        s.height(36.0)
          .background(Muted)
          .underline(Border::inside(BorderSize::Sm, Edge))
          .padding_start(14.0)
          .padding_end(6.0)
          .gap(8.0)
      })
      .base(Title, |s| {
        s.typography(TypographyStyle::Mono)
          .font_size(11.0)
          .foreground(MutedForeground)
      })
      .base(Copy, |s| {
        fading(
          s.square(28.0)
            .radius(RadiusSize::Md)
            .foreground(Foreground)
            .cursor(crate::node::CursorIcon::Pointer),
        )
      })
      .hovered(Copy, |s| s.background(Highlight).foreground(HighlightForeground))
      .pressed(Copy, |s| s.veil(PaletteColor::StatePressed))
      .focus_visible(Copy, |s| s.focus_ring())
      .base(CopyIcon, |s| s.square(Length::Icon(IconSize::Md)))
      .state(CodeBlockState::Copied, CopyIcon, |s| s.foreground(Success))
      .base(Code, |s| {
        s.typography(TypographyStyle::Mono)
          .font_size(13.0)
          .foreground(Foreground)
          .padding_x(14.0)
          .padding_y(14.0)
      })
      .base(LineNumber, |s| {
        s.typography(TypographyStyle::Mono)
          .font_size(13.0)
          .foreground(MutedForeground)
          .padding_end(16.0)
      })
      .base(Marked, |s| s.background(Highlight))
      .base(Keyword, |s| s.foreground(Info))
      .base(String, |s| s.foreground(Success))
      .base(Number, |s| s.foreground(Warning))
      .base(Comment, |s| s.foreground(MutedForeground))
      // Where the board is silent, as the gallery's code had them: names of types in `Info`, marks quiet.
      .base(Type, |s| s.foreground(Info))
      .base(Punctuation, |s| s.foreground(MutedForeground))
  }
}

/// A code block mounted where it is declared.
#[derive(Clone)]
struct CodeBlockProps(Arc<CodeBlock>);

/// The block is built anew by every render of what holds it, so props are never the same as before.
impl PartialEq for CodeBlockProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

#[cfg(feature = "devtools")]
impl crate::app::component::DevtoolsInspectable for CodeBlockProps {}

/// What keeps a mounted block's check for a while after it copied.
struct CodeBlockHost {
  copied: Copied,
}

impl Component for CodeBlockHost {
  type Props = CodeBlockProps;

  fn create(ctx: &mut Ctx) -> Self {
    let on = ctx.signal(false);
    let off = ctx.create_timeout(CODE_BLOCK_COPIED_FOR, {
      let on = on.clone();
      move || on.set(false)
    });
    Self {
      copied: Copied { on, off },
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let block = (*ctx.props::<CodeBlockProps>().0).clone();
    block.build_with(&current_theme(), Some(self.copied.clone()))
  }
}
