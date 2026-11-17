//! A block of code: its lines in the family of code, colored by what each word is, under a bar with its name and a
//! button that copies it.

use std::{
  ops::Range,
  sync::{Arc, Mutex},
  time::Duration,
};

use super::{
  code_tokens::{TokenStyles, code_tokens},
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
    theme::{BorderSize, CodeScheme, IconSize, PaletteColor, RadiusSize, Theme, TypographyStyle},
    window::Window,
  },
  code::{CodeHighlighter, CodeToken},
  components::{Column, LineGutter, Row, TextInput},
  core::{ElementRef, Signal},
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
});

/// The names of the block's own parts, which are no token's.
const OWN_PARTS: &[&str] = &[
  "root",
  "header",
  "title",
  "copy",
  "copy_icon",
  "body",
  "code",
  "line_number",
  "marked",
];

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

/// How many lines a block of a greatest height may have and still lay all of them out: one of more lays out and
/// paints the lines in view, and a few screens of them around.
pub const CODE_BLOCK_WINDOWED_LINES: usize = 400;

/// How many lines a windowed block takes at a time.
const CHUNK: usize = 64;

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
/// Its code is in the family of code, each word in the color of what it is: the highlighter registered for its
/// language tells its tokens apart ([`crate::code`]: lurq's own for Rust, TOML, JSON, JavaScript and TypeScript,
/// shells, Python, CSS, HTML and XML, YAML, SQL and Markdown, or the app's), or the block's own
/// ([`highlighter`](Self::highlighter)); each token is in the style the recipe gives a part of its name
/// ([`token_style`](Self::token_style)) or the theme's code colors give it (`Theme::set_code_color`), dropping the
/// tail of a name neither has. Its lines are one line each that scrolls sideways (or wraps, with
/// [`wrap`](Self::wrap)). Its bar names it (its title, or its language) and copies it whole. A person selects its
/// text with the pointer, across its lines. To what reads the tree it is `code` with its language, called by its
/// label, its title or its language.
///
/// A block of a greatest height with more than [`CODE_BLOCK_WINDOWED_LINES`] lines lays out and paints the lines
/// in view and a few screens of them around, whatever its length; its tokens are kept, so a render that changes
/// nothing of its code does not tokenize it again. Such a block is a component, which `.child(block)` mounts.
#[derive(Clone)]
pub struct CodeBlock {
  code: Arc<str>,
  language: Option<Arc<str>>,
  highlighter: Option<CodeHighlighter>,
  scheme: Option<Arc<CodeScheme>>,
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
      highlighter: None,
      scheme: None,
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

  /// The language of the code (`rust`, `toml`): whose highlighter colors its words (`code::register_highlighter`
  /// gives a language one), what the bar shows where the block has no title, and what tooling is told. A language
  /// with no highlighter is plain text.
  pub fn language(mut self, language: &str) -> Self {
    self.language = Some(Arc::from(language));
    self
  }

  /// Tells the tokens of this block's code apart with `highlighter`, in place of its language's. Made once and
  /// cloned into each render, what it gave is kept and not asked again.
  ///
  /// ```
  /// use lurq::{code::{CodeHighlighter, CodeToken, token}, ui::CodeBlock};
  ///
  /// // Every line that starts with `>` is a prompt.
  /// let prompts = CodeHighlighter::new(|code: &str| {
  ///   let mut at = 0;
  ///   let mut found = Vec::new();
  ///   for line in code.split('\n') {
  ///     if line.starts_with('>') {
  ///       found.push(CodeToken::new(at..at + 1, token::KEYWORD));
  ///     }
  ///     at += line.len() + 1;
  ///   }
  ///   found
  /// });
  /// let session = CodeBlock::new("> help\nusage: ...").highlighter(prompts.clone());
  /// ```
  pub fn highlighter(mut self, highlighter: impl Into<CodeHighlighter>) -> Self {
    self.highlighter = Some(highlighter.into());
    self
  }

  /// Styles the tokens named `name` (`"keyword"`, `"string.escape"`, `"function.macro"`) over the theme's code
  /// colors: a part of that name in the block's recipe. A token with no style of its own name takes its less
  /// particular name's (`keyword.control.rust`, `keyword.control`, `keyword`).
  ///
  /// ```
  /// use lurq::{app::theme::PaletteColor, ui::CodeBlock};
  ///
  /// let block = CodeBlock::new("fn main() {} // the start")
  ///   .language("rust")
  ///   .token_style("comment", |s| s.foreground(PaletteColor::Success).italic())
  ///   .token_style("keyword", |s| s.bold());
  /// ```
  pub fn token_style(self, name: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.style(name, style)
  }

  /// Draws its tokens in the colors of `scheme` in place of the theme's: a block that looks apart from the rest of
  /// the app's code, as a terminal's. [`token_style`](Self::token_style) styles a name over it.
  ///
  /// ```
  /// use lurq::{app::theme::{CodeScheme, CodeStyle}, node::color::Color, ui::CodeBlock};
  ///
  /// let ember = CodeScheme::new()
  ///   .color("keyword", Color::from_hex("#f97316"))
  ///   .color("string", Color::from_hex("#84cc16"))
  ///   .style("comment", CodeStyle::new().foreground(Color::from_hex("#a8a29e")).italic());
  /// let block = CodeBlock::new("fn main() {} // the start").language("rust").code_scheme(ember);
  /// ```
  pub fn code_scheme(mut self, scheme: CodeScheme) -> Self {
    self.scheme = Some(Arc::new(scheme));
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
  /// [`CODE_BLOCK_COPIED_FOR`] after it copied, and a long block follows its scrolling with the lines in view.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    ctx.mount::<CodeBlockHost>(CodeBlockProps(Arc::new(self)))
  }

  /// Builds the still form in `theme`: it copies, and does not show that it did; a long block of a greatest height
  /// shows its first lines alone and does not follow its scrolling. Converting it into an element does this in the
  /// theme of what is being rendered, and mounts a long block of a greatest height as [`mount`](Self::mount) does.
  pub fn build(self, theme: &Theme) -> Element {
    self.build_with(theme, None)
  }

  /// Whether it lays out the lines in view alone: it has more lines than [`CODE_BLOCK_WINDOWED_LINES`], and a
  /// greatest height, or a host that follows where it is in the window (`mounted`).
  fn windowed(&self, mounted: bool) -> bool {
    (mounted || self.bounded()) && self.code.bytes().filter(|byte| *byte == b'\n').count() >= CODE_BLOCK_WINDOWED_LINES
  }

  /// Whether it has a room of its own that its code scrolls down in: a greatest height.
  fn bounded(&self) -> bool {
    matches!(self.max_height, Some(Length::Px(_)))
  }

  fn build_with(self, theme: &Theme, host: Option<&CodeBlockHost>) -> Element {
    let copied = host.map(|host| host.copied.clone());
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
    let tokens = code_tokens(&self.code, self.language.as_deref(), self.highlighter.as_ref());
    let shown = host.and_then(|host| host.follow.window.get());
    let window = self.windowed(host.is_some()).then(|| CodeWindow {
      lines: shown.map(|(start, end, _)| start..end),
      width: shown.map(|(.., width)| width as f32).filter(|width| *width > 0.0),
      follow: host.map(|host| host.follow.clone()),
      screen: host
        .and_then(|host| host.follow.screen_height())
        .filter(|height| *height > 0.0),
    });
    let view = CodeBlockView {
      code: self.code,
      language: self.language,
      highlighter: self.highlighter,
      scheme: self.scheme,
      tokens,
      lines_window: window,
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
    if block.windowed(true) {
      let key = block.key.clone();
      let props = CodeBlockProps(Arc::new(block));
      return match key {
        Some(key) => Element::component_keyed::<CodeBlockHost>(key, props),
        None => Element::component::<CodeBlockHost>(props),
      };
    }
    block.build(&current_theme())
  }
}

/// What a windowed block keeps: the lines it lays out, and what follows where they are.
struct CodeWindow {
  /// The lines laid out, where a layout has said which are in view.
  lines: Option<Range<usize>>,
  /// The width of the room the lines wrap in, where a layout has said it.
  width: Option<f32>,
  follow: Option<Follow>,
  /// How high the window is.
  screen: Option<f32>,
}

/// What a windowed block's host follows: where its lines are, the room they are seen in, and which it laid out.
#[derive(Clone)]
struct Follow {
  /// The lines laid out, and the width of the room they wrap in.
  window: Signal<Option<(usize, usize, u32)>>,
  geometry: Arc<Mutex<Option<Geometry>>>,
  /// The column of the lines, as high as all of them.
  content: ElementRef,
  /// The room the code scrolls in, where the block has a greatest height.
  room: ElementRef,
  /// The window, whose height bounds what of a block with no room of its own is in view.
  screen: Arc<Mutex<Option<Window>>>,
  /// Whether the whole code is selected, from the keyboard.
  all: Signal<bool>,
}

impl Follow {
  /// How high the window is now, where the block is in one that has a size.
  fn screen_height(&self) -> Option<f32> {
    self
      .screen
      .lock()
      .unwrap()
      .as_ref()
      .map(|screen| screen.info().logical_size().height)
      .filter(|height| *height > 0.0)
  }

  /// Takes the lines in view now, where they are not the lines laid out: those of the column of the lines that are
  /// in the window, and in the block's own room where it has one.
  fn follow(&self) {
    let Some(geometry) = self.geometry.lock().unwrap().clone() else {
      return;
    };
    if !self.content.is_attached() {
      return;
    }
    let content = self.content.bounds();
    let screen = self.screen_height().unwrap_or(f32::INFINITY);
    let (mut top, mut bottom) = (0.0f32, screen);
    if geometry.bounded && self.room.is_attached() {
      let room = self.room.bounds();
      top = top.max(room.y);
      bottom = bottom.min(room.y + room.height);
    }
    let from = (top - content.y).max(0.0);
    let to = (bottom - content.y).min(content.height);
    let shown = self.window.get_untracked();
    let lines = if to > from {
      geometry.window(from, to - from)
    } else {
      // Out of view: the lines stay as they are.
      match shown {
        Some((start, end, _)) => start..end,
        None => return,
      }
    };
    // Wrapped lines take rows by the room's width: a room of another width counts them again.
    let width = if geometry.wraps {
      content.width.round() as u32
    } else {
      0
    };
    let next = Some((lines.start, lines.end, width));
    if shown != next {
      self.window.set(next);
    }
  }
}

/// How a windowed block's lines stand in the room it scrolls: what its host reads after a layout to find the lines in
/// view.
#[derive(Clone, Debug, PartialEq)]
struct Geometry {
  /// The room above the first line.
  top: f32,
  /// How high a row of a line is.
  pitch: f32,
  /// The rows of the lines before each line, and of them all at the end: a line that wraps takes more than one.
  rows_before: Arc<[u32]>,
  /// Whether its lines wrap, and take rows by the width of the room.
  wraps: bool,
  /// Whether the lines are seen in a room of the block's own.
  bounded: bool,
}

impl Geometry {
  /// The lines to lay out for a room `height` high scrolled `scrolled` down: those in view, and a chunk more on each
  /// side, by whole chunks.
  fn window(&self, scrolled: f32, height: f32) -> Range<usize> {
    let lines = self.rows_before.len().saturating_sub(1);
    let line_at = |y: f32| {
      let row = ((y - self.top) / self.pitch.max(1.0)).max(0.0) as u32;
      self
        .rows_before
        .partition_point(|before| *before <= row)
        .saturating_sub(1)
    };
    let first = line_at(scrolled);
    let last = line_at(scrolled + height.max(self.pitch));
    let start = (first / CHUNK).saturating_sub(1) * CHUNK;
    let end = ((last / CHUNK + 2) * CHUNK).min(lines);
    start.min(end)..end
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
  /// The block's own highlighter, where it has one in place of its language's.
  pub highlighter: Option<CodeHighlighter>,
  /// The block's own code colors, where it has them in place of the theme's.
  pub scheme: Option<Arc<CodeScheme>>,
  /// The tokens of the code, in bytes of it: by its highlighter, or its language's.
  pub tokens: Arc<[CodeToken]>,
  lines_window: Option<CodeWindow>,
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

  /// The styles of the code's tokens, as this block draws them.
  fn token_styles(&self) -> TokenStyles {
    TokenStyles::new(&self.recipe, OWN_PARTS, self.code_style(), self.scheme.clone())
  }

  /// The code's tokens in their styles, as the input draws them.
  fn input_tokens(&self) -> crate::components::InputTokens {
    self.token_styles().input_tokens(&self.code, &self.tokens)
  }

  fn color_of(&self, part: &ResolvedPart, otherwise: PaletteColor) -> crate::node::color::Color {
    match part.rest().foreground {
      Some(TextColor::Color(color)) => color,
      Some(TextColor::Palette(role)) => self.theme().palette_color(role),
      None => self.theme().palette_color(otherwise),
    }
  }

  /// `code` in spans of the styles of its tokens: the block's code or a line of it, or any code in the block's
  /// language.
  pub fn spans(&self, code: &str) -> Vec<RichTextSpan> {
    let tokens = if code == &*self.code {
      self.tokens.clone()
    } else {
      code_tokens(code, self.language.as_deref(), self.highlighter.as_ref())
    };
    self.token_styles().spans(code, 0..code.len(), &tokens)
  }

  /// The bytes of each line of the code, without its `\n`.
  fn line_ranges(&self) -> Vec<Range<usize>> {
    let mut start = 0;
    self
      .code
      .split('\n')
      .map(|line| {
        let range = start..start + line.len();
        start = range.end + 1;
        range
      })
      .collect()
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
    let ranges = self.line_ranges();
    let (_, _, top, bottom) = self.code_padding();
    self.unwrapped_lines(&self.token_styles(), &ranges, 0..ranges.len(), (top, bottom))
  }

  /// The lines `lines` that do not wrap, `pads` above and below them.
  fn unwrapped_lines(
    &self,
    styles: &TokenStyles,
    ranges: &[Range<usize>],
    lines: Range<usize>,
    (pad_top, pad_bottom): (f32, f32),
  ) -> Element {
    let code_part = self.part(CodeBlockPart::Code);
    if lines.is_empty() {
      return Column::new().into();
    }
    let whole = lines.len() == ranges.len();
    let bytes = ranges[lines.start].start..ranges[lines.end - 1].end;
    let mut code = Node::rich_text(styles.spans(&self.code, bytes, &self.tokens)).text_wrap(false);
    if self.selectable {
      code = code.selectable(true);
    }
    let mut code = Element::from_node(code);
    if whole && let Some(id) = &self.id {
      code = Row::new().id(format!("{id}-code")).child(code).into();
    }
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
      // color. The column is as wide as the block's longest number.
      if !whole {
        column = column.min_width(self.numbers_width());
      }
      let spans = lines
        .clone()
        .map(|index| {
          let mut text = self.number(index);
          if index + 1 < lines.end {
            text.push('\n');
          }
          RichTextSpan {
            text,
            style: self.number_style(Some(self.is_highlighted(index))),
          }
        })
        .collect::<Vec<_>>();
      let mut numbers = Node::rich_text(spans).text_wrap(false).text_align(TextAlign::Right);
      if !whole {
        // A chunk's numbers stand at the end of a column as wide as the block's longest, under each other's ends.
        numbers = numbers.width(Dimension::Pct(100.0));
      }
      column = column.child(Element::from_node(numbers));
      row = row.child(column);
    }
    row = row.child(code);
    if !lines.clone().any(|index| self.is_highlighted(index)) {
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
    for index in lines.clone().filter(|index| self.is_highlighted(*index)) {
      let at = index - lines.start;
      let (from, to) = (
        (top + pitch * at as f32) / height,
        (top + pitch * (at + 1) as f32) / height,
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

  /// How wide the column of the numbers is: the longest number in the family of code, whose figures are as wide as
  /// each other (about six tenths of its size each), and the room after it.
  fn numbers_width(&self) -> f32 {
    let lines = self.code.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let digits = self.number(lines.saturating_sub(1)).len() as f32;
    let number_part = self.part(CodeBlockPart::LineNumber);
    digits * self.code_style().font_size * 0.6
      + self.px(number_part.rest().padding_end.or(number_part.rest().padding_x))
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
    let ranges = self.line_ranges();
    let (_, _, top, bottom) = self.code_padding();
    let mut column = self.wrapped_lines(&self.token_styles(), &ranges, 0..ranges.len(), (top, bottom));
    if let Some(id) = &self.id {
      column = column.id(format!("{id}-code"));
    }
    column.into()
  }

  /// The lines `lines` that wrap, `pads` above and below them.
  fn wrapped_lines(
    &self,
    styles: &TokenStyles,
    ranges: &[Range<usize>],
    lines: Range<usize>,
    (pad_top, pad_bottom): (f32, f32),
  ) -> Column {
    let code_part = self.part(CodeBlockPart::Code);
    let marked = self.part(CodeBlockPart::Marked);
    let number_part = self.part(CodeBlockPart::LineNumber);
    let numbers_width = self.numbers_width();
    // The room's padding across is each line's, so that a picked line's band runs across the block.
    let rest = code_part.rest();
    let (start, end) = (
      self.px(rest.padding_start.or(rest.padding_x)),
      self.px(rest.padding_end.or(rest.padding_x)),
    );
    let mut column = Column::new()
      .width(Dimension::Pct(100.0))
      .part(&code_part)
      .padding_left(0.0)
      .padding_right(0.0)
      .padding_top(pad_top)
      .padding_bottom(pad_bottom);
    for index in lines {
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
      let mut text = Node::rich_text(styles.spans(&self.code, ranges[index].clone(), &self.tokens));
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
    column
  }

  /// The lines in view of a long block, and a few screens of them around: the rest stands as room above and below
  /// them, as high as its rows.
  fn windowed_lines(&self, window: &CodeWindow) -> Element {
    let ranges = self.line_ranges();
    let count = ranges.len();
    let styles = self.token_styles();
    let pitch = self.line_pitch();
    let (start, end, top, bottom) = self.code_padding();
    let advance = self.code_style().font_size * 0.6;
    let numbers = if self.line_numbers { self.numbers_width() } else { 0.0 };
    let width = window
      .width
      .or_else(|| window.follow.as_ref().map(|follow| follow.content.width()))
      .unwrap_or(0.0);
    let room = width - start - end - numbers;
    // A line that wraps takes as many rows as its characters fill at the room's width: the figures of the family of
    // code are as wide as each other.
    let chars: Vec<usize> = ranges
      .iter()
      .map(|range| self.code[range.clone()].chars().count())
      .collect();
    let mut rows_before = Vec::with_capacity(count + 1);
    let mut rows = 0u32;
    for chars in &chars {
      rows_before.push(rows);
      rows += if self.wrap && room > advance {
        ((*chars as f32 * advance) / room).ceil().max(1.0) as u32
      } else {
        1
      };
    }
    rows_before.push(rows);
    let geometry = Geometry {
      top,
      pitch,
      rows_before: rows_before.into(),
      wraps: self.wrap,
      bounded: self.max_height.is_some(),
    };
    // Before a layout has said which are in view: the first, as many as the room or the window shows.
    let lines = window.lines.clone().unwrap_or_else(|| {
      let height = if geometry.bounded {
        self.code_room_height()
      } else {
        window.screen.unwrap_or(1200.0)
      };
      geometry.window(0.0, height)
    });
    let lines = lines.start.min(count)..lines.end.min(count);
    let above = top + geometry.rows_before[lines.start] as f32 * pitch;
    let below = (geometry.rows_before[count] - geometry.rows_before[lines.end]) as f32 * pitch + bottom;
    if let Some(follow) = &window.follow {
      *follow.geometry.lock().unwrap() = Some(geometry);
    }
    let shown: Element = if self.wrap {
      self.wrapped_lines(&styles, &ranges, lines, (0.0, 0.0)).into()
    } else {
      self.unwrapped_lines(&styles, &ranges, lines, (0.0, 0.0))
    };
    let mut column = Column::new()
      .align_items(Alignment::Stretch)
      .child(Column::new().height(above))
      .child(shown)
      .child(Column::new().height(below));
    column = if self.wrap {
      column.width(Dimension::Pct(100.0))
    } else {
      // As wide as the longest line, which may be out of view: the lines in view do not change the width it scrolls.
      let longest = chars.iter().copied().max().unwrap_or(0) as f32;
      column
        .flex_full(1.0, 0.0, None)
        .min_width(longest * advance + start + end + numbers)
    };
    if let Some(follow) = &window.follow {
      column = column.ref_element(follow.content.clone());
      // Selected whole from the keyboard, the code is on the selection's color; a press in it starts another.
      if follow.all.get() {
        column = column.background(PaletteColor::Selection);
      }
      let all = follow.all.clone();
      column = column.on_mouse_down(move |_: MouseEvent| {
        if all.get_untracked() {
          all.set(false);
        }
      });
    }
    if let Some(id) = &self.id {
      column = column.id(format!("{id}-code"));
    }
    column.into()
  }

  /// How high the room the code scrolls in is at most: the block's greatest height, less its bar.
  fn code_room_height(&self) -> f32 {
    let header = match (self.has_header, self.part(CodeBlockPart::Header).rest().height) {
      (true, Some(Length::Px(height))) => height,
      _ => 0.0,
    };
    match self.max_height {
      Some(Length::Px(max)) => (max - header).max(0.0),
      _ => 0.0,
    }
  }

  /// The room the code scrolls in: sideways where it does not wrap, down past the greatest height.
  pub fn body(&self) -> Element {
    let content = match (&self.lines_window, self.selectable, self.wrap) {
      (Some(window), ..) => self.windowed_lines(window),
      (None, true, _) => self.code_input(),
      (None, false, true) => self.wrapped(),
      (None, false, false) => self.unwrapped(),
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
    if !self.selectable || self.lines_window.is_some() {
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
    if self.max_height.is_some()
      && let Some(follow) = self.lines_window.as_ref().and_then(|window| window.follow.as_ref())
    {
      body = body.ref_element(follow.room.clone());
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
    if let Some(follow) = self.lines_window.as_ref().and_then(|window| window.follow.as_ref()) {
      // Its lines are not all laid out: select all, from anywhere in the block, takes the code whole, and a copy of
      // it copies it whole.
      let (all, code) = (follow.all.clone(), self.code.clone());
      root = root
        .on_select_all({
          let all = all.clone();
          move |_: crate::app::events::EditEvent| all.set(true)
        })
        .on_copy(move |event: crate::app::events::CopyEvent| {
          if all.get_untracked() {
            event.set_text(code.to_string());
          }
        });
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

impl Themed for CodeBlock {
  const NAME: &'static str = "code_block";
  type Renderer = CodeBlockRenderer;

  /// The design's code block (`pencil/lurq-design-system.pen`, Code Block `kyzrV`, in Data and in the Markdown
  /// document): on `Background` in an edge of 1 in `Border` with the large radius; a bar of 36 on `Muted` with a line
  /// of 1 in `Border` under it, padded 14 at its start and 6 at its end, its title 11 in the family of code in
  /// `MutedForeground`, and a ghost button of 28 with a copy glyph of 16; the code 13 in the family of code in
  /// `Foreground`, padded 14, its lines 4 apart. Its tokens are in the theme's code colors (`CodeScheme::lurq`: the
  /// words of the language in `Info`, as the board draws them); a rule for a part named as a token styles it over
  /// them.
  fn recipe() -> Recipe {
    use CodeBlockPart::*;
    use PaletteColor::{
      Background, Border as Edge, Foreground, Highlight, HighlightForeground, Muted, MutedForeground, Success,
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

impl crate::app::component::DevtoolsInspectable for CodeBlockProps {}

/// What keeps a mounted block's check for a while after it copied, and a long block's lines in view.
struct CodeBlockHost {
  copied: Copied,
  follow: Follow,
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
      follow: Follow {
        window: Signal::new(None),
        geometry: Arc::default(),
        content: ElementRef::new(),
        room: ElementRef::new(),
        screen: Arc::default(),
        all: ctx.signal(false),
      },
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let block = (*ctx.props::<CodeBlockProps>().0).clone();
    *self.follow.screen.lock().unwrap() = super::theme::current_window();
    if !block.windowed(true) {
      *self.follow.geometry.lock().unwrap() = None;
    }
    let element = block.build_with(&current_theme(), Some(self));
    // Where the lines or their room move (a scroll of the block's own room or of the page round it, a window of
    // another size), the lines in view are taken before the frame is painted.
    for watched in [&self.follow.content, &self.follow.room] {
      let follow = self.follow.clone();
      watched.observe_rect_before_paint(move |_| follow.follow());
    }
    element
  }

  fn after_layout(&self) {
    self.follow.follow();
  }
}
