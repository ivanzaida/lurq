//! A rich text editor: formatted text typed into a box under a bar of its tools.

use std::sync::{
  Arc,
  atomic::{AtomicBool, Ordering},
};

use parking_lot::Mutex;

use super::{
  button::{Button, ButtonPart, ButtonSize, ButtonVariant},
  field::{FieldControl, InField},
  icons,
  input::{Input, InputSize},
  names,
  part_style::{FontFamily, Length, PartStyle},
  picture::Picture,
  popover::{Popover, PopoverPart},
  recipe::{Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  select::{Select, SelectPart},
  text_box::{self, BoxStates, Judgment, TextBoxState},
  theme::{Themed, current_theme},
  toggle::{Toggle, TogglePart},
  toolbar::{Toolbar, ToolbarItem, ToolbarPart},
};
use crate::{
  app::{
    component::Component,
    ctx::{Ctx, Placement},
    events::{ClipboardImage, KeyboardEvent, MouseEvent, PasteEvent},
    theme::{BorderSize, OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::{
    Column, InlineObject, LinkClick, MarkPresence, ParagraphKind, ParagraphMarker, ParagraphStyle, ParagraphStyles,
    RichText, Row, SelectionStyle, StyledChange, StyledSpan, StyledText, TextInput, TextInputController, TextMark,
  },
  core::{ElementRef, Signal},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{CursorIcon, Element, TextColor, border::Border},
};

/// The parts of a rich text editor a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RichTextEditorPart {
  /// The editor's box: its width, its fill, its edge and its corners.
  Root,
  /// What the editor says of the bar of its tools: laid over a toolbar's root.
  Toolbar,
  /// What the editor says of a tool: laid over the root of a toolbar's button or toggle.
  Tool,
  /// What the editor says of a tool that is on, after `Tool`: laid over a toggle's root while it is on.
  ToolOn,
  /// What the editor says of a tool's icon: laid over the icon of a button or a toggle.
  ToolIcon,
  /// What the editor says of the control that chooses a paragraph's style: laid over a select's trigger.
  Style,
  /// What the editor says of that control's chevron: laid over a select's chevron.
  StyleChevron,
  /// What the editor says of the list of styles: laid over a select's panel.
  StyleList,
  /// The box that is typed in: its padding, the type and color of body text, and the room between two paragraphs
  /// (its gap).
  Content,
  /// What the box shows while it holds nothing: laid over the content's type.
  Placeholder,
  /// A heading of the first level: its type, over the content's.
  Heading1,
  /// A heading of the second level.
  Heading2,
  /// A heading of the third level, and of every level under it.
  Heading3,
  /// An item of a bulleted list: the room before its text (its start padding).
  Bullet,
  /// The dot before an item: its width is the dot's, its gap the room to the text, its foreground the color.
  BulletMark,
  /// An item of a numbered list: the room before its text.
  Ordered,
  /// The number before an item: its gap the room to the text, its foreground the color.
  OrderedMark,
  /// A paragraph of a quote: the room before its text, and its color.
  Quote,
  /// The bar down the start of a quote: its width, and its foreground the color.
  QuoteMark,
  /// A line of a block of code: its type, the room before its text, its fill and the fill's radius.
  CodeBlock,
  /// The row under the box: what it says of the editor.
  Footer,
  /// A text of the footer.
  FooterText,
  /// What the editor says of the panel a link is given in: laid over a popover's panel.
  LinkPanel,
}

names!(RichTextEditorPart {
  Root => "root",
  Toolbar => "toolbar",
  Tool => "tool",
  ToolOn => "tool_on",
  ToolIcon => "tool_icon",
  Style => "style",
  StyleChevron => "style_chevron",
  StyleList => "style_list",
  Content => "content",
  Placeholder => "placeholder",
  Heading1 => "heading1",
  Heading2 => "heading2",
  Heading3 => "heading3",
  Bullet => "bullet",
  BulletMark => "bullet_mark",
  Ordered => "ordered",
  OrderedMark => "ordered_mark",
  Quote => "quote",
  QuoteMark => "quote_mark",
  CodeBlock => "code_block",
  Footer => "footer",
  FooterText => "footer_text",
  LinkPanel => "link_panel",
});

/// The states of a rich text editor a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RichTextEditorState {
  /// The keyboard's focus is in the box that is typed in.
  Focused,
  /// What it holds is wrong: its edge is in the destructive color.
  Invalid,
  /// What it holds is allowed, with a caveat: its edge is in the warning color.
  Warning,
  /// What it holds was checked and is right: its edge is in the success color.
  Valid,
  /// It can be read, selected and copied, and not changed: its tools take no input.
  ReadOnly,
  /// It takes no input.
  Disabled,
  /// It is as wide as what holds it.
  FullWidth,
}

names!(RichTextEditorState {
  Focused => "focused",
  Invalid => "invalid",
  Warning => "warning",
  Valid => "valid",
  ReadOnly => "read_only",
  Disabled => "disabled",
  FullWidth => "full_width",
});

/// A tool of a rich text editor's bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RichTextTool {
  /// The control that says what the paragraph is and makes it something else: a paragraph, a heading of a level
  /// from 1 to 3, a block of code.
  Style,
  Bold,
  Italic,
  Underline,
  Strikethrough,
  /// A bulleted list of the paragraphs the selection touches, or plain paragraphs of them again.
  Bullet,
  /// A numbered list.
  Ordered,
  /// A quote.
  Quote,
  /// Inline code.
  Code,
  /// Opens the panel a link's target is typed in.
  Link,
  /// Runs the app's [`RichTextEditor::on_image`], which asks for a picture and puts it in the text with
  /// [`RichTextController::insert_picture`].
  Image,
  Undo,
  Redo,
}

impl RichTextTool {
  /// The tools an editor has unless it is told: every one but the image's, in the board's order.
  pub const DEFAULT: [Self; 12] = [
    Self::Style,
    Self::Bold,
    Self::Italic,
    Self::Underline,
    Self::Strikethrough,
    Self::Bullet,
    Self::Ordered,
    Self::Quote,
    Self::Code,
    Self::Link,
    Self::Undo,
    Self::Redo,
  ];

  /// The group of the bar the tool stands in: a line stands between two groups, and the last is at the bar's end.
  fn group(self) -> u8 {
    match self {
      Self::Style => 0,
      Self::Bold | Self::Italic | Self::Underline | Self::Strikethrough => 1,
      Self::Bullet | Self::Ordered | Self::Quote | Self::Code => 2,
      Self::Link | Self::Image => 3,
      Self::Undo | Self::Redo => 4,
    }
  }
}

/// What builds a rich text editor's nodes: a theme's in place of the default one.
pub type RichTextEditorRenderer = Arc<dyn Fn(RichTextEditorView) -> Element + Send + Sync>;

type Run = Arc<dyn Fn() + Send + Sync>;
type ChangeHandler = Arc<dyn Fn(&RichText) + Send + Sync>;
type LinkHandler = Arc<dyn Fn(&str) + Send + Sync>;
type PictureBuilder = Arc<dyn Fn(&InlineObject) -> Element + Send + Sync>;
type ImageKeeper = Arc<dyn Fn(&ClipboardImage) -> Option<String> + Send + Sync>;
type CountText = Arc<dyn Fn(usize) -> String + Send + Sync>;
type HeadingText = Arc<dyn Fn(u8) -> String + Send + Sync>;

/// The value of an editor: the app's signal, or what the app holds itself.
#[derive(Clone)]
enum Held {
  Bound(Signal<RichText>),
  Controlled(RichText),
}

/// The words of the editor: the names of its tools and what it says.
#[derive(Clone)]
struct Words {
  label: Option<Arc<str>>,
  toolbar: Arc<str>,
  style: Arc<str>,
  paragraph: Arc<str>,
  heading: HeadingText,
  code_block: Arc<str>,
  bold: Arc<str>,
  italic: Arc<str>,
  underline: Arc<str>,
  strikethrough: Arc<str>,
  bullet: Arc<str>,
  ordered: Arc<str>,
  quote: Arc<str>,
  code: Arc<str>,
  link: Arc<str>,
  image: Arc<str>,
  undo: Arc<str>,
  redo: Arc<str>,
  more: Arc<str>,
  link_placeholder: Arc<str>,
  apply: Arc<str>,
  remove_link: Arc<str>,
  shortcuts: Arc<str>,
  count: CountText,
}

impl Default for Words {
  fn default() -> Self {
    let locale = super::current_locale();
    let words = &locale.editing;
    Self {
      label: None,
      toolbar: words.formatting.clone(),
      style: words.text_style.clone(),
      paragraph: words.paragraph.clone(),
      heading: {
        let heading = words.heading.clone();
        Arc::new(move |level| crate::core::fill(&heading, &[("level", &level.to_string())]))
      },
      code_block: words.code_block.clone(),
      bold: words.bold.clone(),
      italic: words.italic.clone(),
      underline: words.underline.clone(),
      strikethrough: words.strikethrough.clone(),
      bullet: words.bulleted_list.clone(),
      ordered: words.numbered_list.clone(),
      quote: words.quote.clone(),
      code: words.code.clone(),
      link: words.link.clone(),
      image: words.image.clone(),
      undo: words.undo.clone(),
      redo: words.redo.clone(),
      more: locale.common.more.clone(),
      link_placeholder: words.paste_link.clone(),
      apply: locale.common.apply.clone(),
      remove_link: words.remove_link.clone(),
      shortcuts: words.markdown_shortcuts.clone(),
      count: {
        let locale = locale.clone();
        Arc::new(move |count| locale.count(&locale.editing.words, count))
      },
    }
  }
}

/// What an app keeps to work a rich text editor from outside it: the commands of its tools, and the text put at
/// its caret. Given to the editor with [`RichTextEditor::controller`]; without one the editor keeps its own.
///
/// ```
/// use lurq::{components::{ParagraphKind, TextMark}, ui::RichTextController};
///
/// // Kept by the app from one render to the next.
/// let editor = RichTextController::new();
/// // After the editor is on the screen:
/// editor.toggle_mark(TextMark::Bold);
/// editor.set_paragraphs(ParagraphKind::Heading(2));
/// editor.insert_link("the docs", "https://ivanzaida.github.io/lurq/");
/// ```
#[derive(Clone)]
pub struct RichTextController {
  input: TextInputController,
  /// Changed to have the editor drawn again where its text did not change: the caret moved, a tool was run.
  tick: Signal<u64>,
}

impl Default for RichTextController {
  fn default() -> Self {
    Self::new()
  }
}

impl RichTextController {
  pub fn new() -> Self {
    Self {
      input: TextInputController::new(),
      tick: Signal::new(0),
    }
  }

  /// The controller of the editor's text: its selection, and the engine's commands of a styled input.
  pub fn input(&self) -> TextInputController {
    self.input.clone()
  }

  fn redraw(&self) {
    self.tick.update(|tick| *tick = tick.wrapping_add(1));
  }

  /// Turns `mark` on over the selection unless all of it has it, then off; with nothing selected, for what is
  /// typed next. `false` where nothing changed, and while the editor is not on the screen.
  pub fn toggle_mark(&self, mark: TextMark) -> bool {
    let done = self.input.toggle_mark(mark).unwrap_or(false);
    self.redraw();
    done
  }

  /// Makes every paragraph the selection touches a `kind`.
  pub fn set_paragraphs(&self, kind: ParagraphKind) -> bool {
    let done = self.input.set_paragraphs(kind).unwrap_or(false);
    self.redraw();
    done
  }

  /// Makes the paragraphs the selection touches a `kind`, or body text again where they all are one already:
  /// what the tool of a list or a quote does.
  pub fn toggle_paragraphs(&self, kind: ParagraphKind) -> bool {
    let all = self
      .input
      .selected_paragraphs()
      .ok()
      .flatten()
      .is_some_and(|kinds| kinds.iter().all(|held| *held == kind));
    self.set_paragraphs(if all { ParagraphKind::Body } else { kind })
  }

  /// Links the selection to `target`, or unlinks it with `None`; with nothing selected, the link the caret is in.
  pub fn set_link(&self, target: Option<&str>) -> bool {
    let done = self.input.set_link(target).unwrap_or(false);
    self.redraw();
    done
  }

  /// Selects from `anchor` to `caret`, bytes of the text, or puts the caret at one place where they are the
  /// same. `false` where either is not at the start of a character.
  pub fn select(&self, anchor: usize, caret: usize) -> bool {
    let done = self.input.select(anchor, caret).is_ok();
    self.redraw();
    done
  }

  /// Puts `text` in place of the selection, in the style typing would give it.
  pub fn insert_text(&self, text: &str) -> bool {
    let done = self.input.replace_selection(text).unwrap_or(false);
    self.redraw();
    done
  }

  /// Puts a picture in place of the selection, or at the caret, which is left after it: `source` is where its
  /// pixels are, the path of a file or an address, and `label` what it shows, in words. What an app does with
  /// the file a person chose after the image tool ran its [`on_image`](RichTextEditor::on_image). The picture is
  /// a character of the document: it is selected, deleted, copied and pasted with the words around it, and one
  /// step of the undo takes it back. `false` in a line of a block of code, which holds no picture, in an editor
  /// that takes no change, and while the editor is not on the screen.
  ///
  /// ```
  /// use lurq::ui::RichTextController;
  ///
  /// let editor = RichTextController::new();
  /// // After the editor is on the screen, with the file a person chose:
  /// editor.insert_picture("photos/lake.jpg", "The lake at dusk");
  /// ```
  pub fn insert_picture(&self, source: &str, label: &str) -> bool {
    self.insert_object(&InlineObject::new(0, source).label(label))
  }

  /// Puts `object` in place of the selection as [`insert_picture`](Self::insert_picture) does: for a picture
  /// with a width of its own (`InlineObject::new(0, source).label(label).width(320)`). The byte it is put at is
  /// the caret's, whatever `object.at` says.
  pub fn insert_object(&self, object: &InlineObject) -> bool {
    let in_code = self
      .input
      .caret_paragraph()
      .ok()
      .flatten()
      .is_some_and(|(kind, _)| kind == ParagraphKind::Code);
    let done = !in_code && self.input.insert_object(object).unwrap_or(false);
    self.redraw();
    done
  }

  /// Puts `text` in place of the selection as a link to `target`.
  pub fn insert_link(&self, text: &str, target: &str) -> bool {
    let style = crate::components::SpanStyle::new().link(target);
    let linked = StyledText::new(text, vec![StyledSpan::new(0..text.len(), style)]);
    let done = self.input.replace_selection_styled(&linked).unwrap_or(false);
    self.redraw();
    done
  }

  /// Takes back the last edit.
  pub fn undo(&self) -> bool {
    let done = self.input.undo().unwrap_or(false);
    self.redraw();
    done
  }

  /// Makes again the edit last taken back.
  pub fn redo(&self) -> bool {
    let done = self.input.redo().unwrap_or(false);
    self.redraw();
    done
  }

  /// The document the editor holds now; `None` while it is not on the screen.
  pub fn value(&self) -> Option<RichText> {
    self.input.rich_text().ok().flatten()
  }
}

/// A rich text editor: formatted text (headings, lists, quotes, code, and bold, italic, underlined, struck,
/// code and linked words) typed into a box, under a bar of the tools that format it. A comment, release
/// notes, the body of an article.
///
/// ```
/// use lurq::{app::ctx::Ctx, components::RichText, core::Signal, node::Element, ui::RichTextEditor};
///
/// # fn build(ctx: &mut Ctx, notes: Signal<RichText>) -> Element {
/// // Bound to the app's signal, which holds the document.
/// RichTextEditor::new(notes)
///   .placeholder("Write the release notes")
///   .on_change(|document| println!("{}", document.to_markdown()))
///   .mount(ctx)
/// # }
/// // Held by the app: it is told of each change, and the editor shows what the app answers with.
/// let held = RichTextEditor::controlled(RichText::from_markdown("## Notes"), |document| {
///   println!("{} words", document.word_count())
/// });
/// ```
///
/// Its value is a [`RichText`]: the text, the marks over it and what each of its paragraphs is, which reads
/// from Markdown and writes it ([`RichText::from_markdown`], [`RichText::to_markdown`]). The editor is the
/// engine's styled text input with kinds of paragraphs, so the caret, the selection, the clipboard and the
/// undo history are a text input's own; a copy keeps its formatting through the clipboard
/// ([`TextInput::rich_clipboard`](crate::components::TextInput::rich_clipboard)).
///
/// A document holds pictures too, each a character of its text ([`InlineObject`]): alone in a paragraph or
/// among words, as high as it is in a row that makes room for it. The caret goes past a picture as past one
/// character, a press selects it, Backspace after it selects it and the next deletes it, and it is copied and
/// pasted with the words around it, as the Markdown image it is written as. The image tool runs the app's
/// [`on_image`](Self::on_image), and the app puts the picture in with
/// [`RichTextController::insert_picture`]. A picture is drawn by a [`Picture`], which loads it and shows that it
/// is on its way or did not come; [`picture`](Self::picture) draws it another way.
///
/// A tool works on the selection: a mark over the selected words, or over what is typed next where nothing is
/// selected; a list, a quote or a style over every paragraph the selection touches. The bar shows what the
/// selection is. The keys: Ctrl+B, Ctrl+I, Ctrl+U (Cmd on a Mac), Ctrl+Shift+X to strike, Ctrl+E for code,
/// Ctrl+K for a link, Ctrl+Alt+0 to 3 for a paragraph and the headings, Ctrl+Shift+7, 8 and 9 for a numbered
/// list, a bulleted one and a quote, Ctrl+Z and Ctrl+Y. Enter in an empty item ends its list, Enter at the end
/// of a heading starts a paragraph, and Backspace at the start of anything but a paragraph makes it one. With
/// [`markdown_shortcuts`](Self::markdown_shortcuts), which are on unless told, `# `, `## `, `### `, `- `, `1. `,
/// `> ` and three backticks at the start of a paragraph make it what they say, and `**bold**`, `*italic*`,
/// `` `code` `` and `~~struck~~` become the marks as their last character is typed.
///
/// Mounted ([`mount`](Self::mount)), it keeps the text being edited, the selection its bar shows and the
/// panel of a link. Built without (`build`, or converted into an element), it is the still form of itself: the
/// document as it is drawn, with nothing kept between two renders.
#[derive(Clone)]
pub struct RichTextEditor {
  value: Held,
  on_change: Vec<ChangeHandler>,
  on_link_click: Vec<LinkHandler>,
  on_image: Vec<Run>,
  picture: Option<PictureBuilder>,
  on_paste_image: Option<ImageKeeper>,
  controller: Option<RichTextController>,
  placeholder: Option<Arc<str>>,
  min_rows: usize,
  max_rows: usize,
  tools: Vec<RichTextTool>,
  toolbar: bool,
  footer: bool,
  markdown_shortcuts: bool,
  boxed: BoxStates,
  full_width: bool,
  words: Words,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  name: Option<Arc<str>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

/// What an editor draws for a picture unless it is told ([`RichTextEditor::picture`]): a [`Picture`] of its
/// source, which is an address where it starts as one (`http://`, `https://`) and the path of a file otherwise,
/// called by its label, as wide as the document says it is and never wider than its line.
fn picture_of(object: &InlineObject) -> Element {
  let source = object.source.as_ref();
  let mut picture = if source.starts_with("http://") || source.starts_with("https://") {
    Picture::from_url(source)
  } else {
    Picture::from_file(source)
  };
  if !object.label.is_empty() {
    picture = picture.label(&object.label);
  }
  if let Some(width) = object.width {
    picture = picture.width(width as f32);
  }
  picture.into()
}

/// The rows of body text an editor shows while it holds less, unless it is told otherwise.
pub const RICH_TEXT_MIN_ROWS: usize = 6;
/// The rows of body text an editor grows to before its text scrolls, unless it is told otherwise.
pub const RICH_TEXT_MAX_ROWS: usize = 16;

macro_rules! words {
  ($($(#[$doc:meta])* $method:ident => $field:ident),* $(,)?) => {
    $(
      $(#[$doc])*
      pub fn $method(mut self, text: &str) -> Self {
        self.words.$field = Arc::from(text);
        self
      }
    )*
  };
}

impl RichTextEditor {
  fn of(value: Held) -> Self {
    Self {
      value,
      on_change: Vec::new(),
      on_link_click: Vec::new(),
      on_image: Vec::new(),
      picture: None,
      on_paste_image: None,
      controller: None,
      placeholder: None,
      min_rows: RICH_TEXT_MIN_ROWS,
      max_rows: RICH_TEXT_MAX_ROWS,
      tools: RichTextTool::DEFAULT.to_vec(),
      toolbar: true,
      footer: true,
      markdown_shortcuts: true,
      boxed: BoxStates::default(),
      full_width: false,
      words: Words::default(),
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      name: None,
      id: None,
      key: None,
    }
  }

  /// An editor of the document `value` holds: it writes each change into the signal, and shows what the app
  /// writes there.
  pub fn new(value: Signal<RichText>) -> Self {
    Self::of(Held::Bound(value))
  }

  /// An editor of a document the app holds: each change is told to `on_change`, and the editor shows what the
  /// app answers with. A change the app does not take is taken back.
  pub fn controlled(value: RichText, on_change: impl Fn(&RichText) + Send + Sync + 'static) -> Self {
    Self::of(Held::Controlled(value)).on_change(on_change)
  }

  /// Called with the document after each change a person makes: typing, a tool, a key, undo. Not when the app
  /// sets the signal. Bound, the signal is set first. A second handler is called after the first.
  pub fn on_change(mut self, handler: impl Fn(&RichText) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// Called with a link's target when it is clicked with Ctrl held (Cmd on a Mac): the app opens it. A plain
  /// click places the caret.
  pub fn on_link_click(mut self, handler: impl Fn(&str) + Send + Sync + 'static) -> Self {
    self.on_link_click.push(Arc::new(handler));
    self
  }

  /// What the image tool does: with it the bar has the tool ([`RichTextTool::Image`]) unless [`Self::tools`]
  /// leaves it out. The app asks for a picture (the system's dialog for a file, a panel of its own) and puts it
  /// in the text with [`RichTextController::insert_picture`].
  pub fn on_image(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    if self.on_image.is_empty() && !self.tools.contains(&RichTextTool::Image) {
      let after = self
        .tools
        .iter()
        .rposition(|tool| tool.group() <= RichTextTool::Image.group())
        .map_or(0, |at| at + 1);
      self.tools.insert(after, RichTextTool::Image);
    }
    self.on_image.push(Arc::new(handler));
    self
  }

  /// What is drawn for a picture of the document, in place of a [`Picture`] of its source: `element` is given
  /// the picture's record (its source, its label, its width) and builds the element. An app whose pictures are
  /// not files or addresses (bytes it holds, a resource, a key of its own store) draws them here. The element is
  /// laid out in the room the text has and takes the room it needs in its row; the pointer and the focus do
  /// not reach what it holds.
  ///
  /// ```
  /// use lurq::{components::RichText, core::Signal, ui::{Picture, RichTextEditor}};
  ///
  /// # fn build(notes: Signal<RichText>) -> RichTextEditor {
  /// // The document names its pictures by a path under the app's resources.
  /// RichTextEditor::new(notes).picture(|picture| {
  ///   Picture::from_resource(&picture.source).label(&picture.label).into()
  /// })
  /// # }
  /// ```
  pub fn picture(mut self, element: impl Fn(&InlineObject) -> Element + Send + Sync + 'static) -> Self {
    self.picture = Some(Arc::new(element));
    self
  }

  /// What is done with a picture pasted as pixels: a screenshot, an image copied in another app. `keep` is given
  /// the pixels, keeps them where the app keeps its pictures (a file it writes, a store of its own), and answers
  /// with the source the document is to name them by; the editor puts a picture of that source at the caret, as
  /// one step of the undo. `None` puts nothing in. Without it a paste of pixels does nothing: a document names
  /// its pictures and holds no pixels itself. A second call takes the place of the first.
  ///
  /// A clipboard that holds words is pasted as text, whatever else it holds. Files that are pictures, copied in
  /// a file manager, are put in by their paths with no handler at all, each called by its file's name.
  ///
  /// ```
  /// use lurq::{components::RichText, core::Signal, ui::RichTextEditor};
  ///
  /// # fn build(notes: Signal<RichText>) -> RichTextEditor {
  /// RichTextEditor::new(notes).on_paste_image(|image| {
  ///   // The app writes the pixels where it keeps its pictures, and says where.
  ///   let path = format!("attachments/pasted-{}x{}.png", image.width(), image.height());
  ///   std::fs::write(&path, image.to_png().ok()?).ok()?;
  ///   Some(path)
  /// })
  /// # }
  /// ```
  pub fn on_paste_image(mut self, keep: impl Fn(&ClipboardImage) -> Option<String> + Send + Sync + 'static) -> Self {
    self.on_paste_image = Some(Arc::new(keep));
    self
  }

  /// The app's handle on the editor: its tools' commands, run from outside it.
  pub fn controller(mut self, controller: &RichTextController) -> Self {
    self.controller = Some(controller.clone());
    self
  }

  /// What the box shows while it holds nothing.
  pub fn placeholder(mut self, placeholder: &str) -> Self {
    self.placeholder = Some(Arc::from(placeholder));
    self
  }

  /// What the editor is called, to what reads the tree. It draws nothing: the label a person reads is its
  /// field's, which gives the editor this name unless it has its own.
  pub fn label(mut self, label: &str) -> Self {
    self.words.label = Some(Arc::from(label));
    self.boxed.label = Some(Arc::from(label));
    self
  }

  /// The rows of body text the box shows at least, while it holds less, and at most, before its text scrolls:
  /// 6 and 16 unless told.
  pub fn rows(mut self, min: usize, max: usize) -> Self {
    self.min_rows = min.max(1);
    self.max_rows = max.max(self.min_rows);
    self
  }

  /// The tools of the bar, in the order given: a line stands between two of different groups (the style, the
  /// marks, the blocks, the link and the image, the history), and the history stands at the bar's end.
  pub fn tools(mut self, tools: impl IntoIterator<Item = RichTextTool>) -> Self {
    self.tools = tools.into_iter().collect();
    self
  }

  /// Whether the editor has its bar of tools: it has unless told. Without it the keys still format.
  pub fn toolbar(mut self, toolbar: bool) -> Self {
    self.toolbar = toolbar;
    self
  }

  /// Whether the editor has the row under its box, which says that Markdown shortcuts are on and counts the
  /// words: it has unless told.
  pub fn footer(mut self, footer: bool) -> Self {
    self.footer = footer;
    self
  }

  /// Whether what is typed as Markdown becomes what it says (`## ` a heading, `**bold**` bold): it does unless
  /// told.
  pub fn markdown_shortcuts(mut self, shortcuts: bool) -> Self {
    self.markdown_shortcuts = shortcuts;
    self
  }

  /// Whether what it holds is wrong. Of invalid, warning and valid, the first that is set is the one shown.
  pub fn invalid(mut self, invalid: bool) -> Self {
    self.boxed.invalid = invalid;
    self
  }

  /// Whether what it holds is allowed with a caveat.
  pub fn warning(mut self, warning: bool) -> Self {
    self.boxed.warning = warning;
    self
  }

  /// Whether what it holds was checked and is right.
  pub fn valid(mut self, valid: bool) -> Self {
    self.boxed.valid = valid;
    self
  }

  /// Whether it can be read, selected and copied, and not changed: its tools take no input.
  pub fn read_only(mut self, read_only: bool) -> Self {
    self.boxed.read_only = read_only;
    self
  }

  /// Whether it takes no input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.boxed.disabled = disabled;
    self
  }

  /// Whether the editor is as wide as what holds it, in place of the design's 560.
  pub fn full_width(mut self, full_width: bool) -> Self {
    self.full_width = full_width;
    self
  }

  words! {
    /// What the bar of tools is called, to what reads the tree: "Formatting" by default.
    toolbar_label => toolbar,
    /// What the control of a paragraph's style is called: "Text style" by default.
    style_label => style,
    /// What the style of a plain paragraph is called: "Paragraph" by default.
    paragraph_text => paragraph,
    /// What the style of a block of code is called: "Code block" by default.
    code_block_text => code_block,
    /// What the tool of bold is called: "Bold" by default.
    bold_label => bold,
    /// What the tool of italic is called: "Italic" by default.
    italic_label => italic,
    /// What the tool of an underline is called: "Underline" by default.
    underline_label => underline,
    /// What the tool that strikes text through is called: "Strikethrough" by default.
    strikethrough_label => strikethrough,
    /// What the tool of a bulleted list is called: "Bulleted list" by default.
    bullet_label => bullet,
    /// What the tool of a numbered list is called: "Numbered list" by default.
    ordered_label => ordered,
    /// What the tool of a quote is called: "Quote" by default.
    quote_label => quote,
    /// What the tool of inline code is called: "Code" by default.
    code_label => code,
    /// What the tool of a link is called: "Link" by default.
    link_label => link,
    /// What the tool of an image is called: "Image" by default.
    image_label => image,
    /// What the tool that takes an edit back is called: "Undo" by default.
    undo_label => undo,
    /// What the tool that makes an edit again is called: "Redo" by default.
    redo_label => redo,
    /// What the button of the tools that do not fit is called: "More" by default.
    more_label => more,
    /// What the field of a link's target shows while it is empty, and what it is called: "Paste a link" by default.
    link_placeholder => link_placeholder,
    /// What the button that gives the selection its link says: "Apply" by default.
    apply_label => apply,
    /// What the button that takes a link away says: "Remove link" by default.
    remove_link_label => remove_link,
    /// What the footer says while Markdown shortcuts are on: "Markdown shortcuts on" by default.
    shortcuts_text => shortcuts,
  }

  /// What the style of a heading is called, told its level: "Heading 2" by default.
  pub fn heading_text(mut self, text: impl Fn(u8) -> String + Send + Sync + 'static) -> Self {
    self.words.heading = Arc::new(text);
    self
  }

  /// What the footer says of the count of words, told the count: "18 words" by default.
  pub fn count_text(mut self, text: impl Fn(usize) -> String + Send + Sync + 'static) -> Self {
    self.words.count = Arc::new(text);
    self
  }

  /// The name the document has in the form the editor is in: its value there is the document as Markdown.
  #[cfg(feature = "form")]
  pub fn name(mut self, name: impl Into<Arc<str>>) -> Self {
    self.name = Some(name.into());
    self
  }

  /// Puts the editor in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this editor over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this editor.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the box as it is under the pointer or focused, with nothing pointing at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the editor. The box that is typed in has this id with `-content` after it, the bar `-toolbar`,
  /// a tool `-bold`, `-italic`, `-underline`, `-strikethrough`, `-bullet`, `-ordered`, `-quote`, `-code`, `-link`,
  /// `-image`, `-undo` and `-redo`, the style's control `-style`, the footer's count `-count`, and the field of
  /// a link `-link-field`.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Puts the editor in the tree of `ctx`'s component, in the theme of what is being rendered: it keeps the
  /// text being edited, what its bar shows of the selection, and the panel of a link.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let key = self.key.clone();
    let mut element = ctx.mount::<RichTextEditorHost>(RichTextEditorProps { editor: self });
    if let Some(key) = key {
      element = element.key(key);
    }
    element
  }

  /// Builds the editor in `theme` once, with nothing kept: the document as it is drawn. Converting an editor
  /// into an element does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let document = match &self.value {
      Held::Bound(signal) => signal.get(),
      Held::Controlled(value) => value.clone(),
    };
    let document = RichText::new(document.text, document.spans, document.paragraphs).with_objects(document.objects);
    let kept = Kept {
      text: Signal::new(document.text),
      spans: Signal::new(document.spans),
      kinds: Signal::new(document.paragraphs),
      objects: Signal::new(document.objects),
      controller: self.controller.clone().unwrap_or_default(),
      last: Arc::default(),
      link_open: Signal::new(false),
      link_target: Signal::new(String::new()),
      link_anchor: ElementRef::new(),
      link_field: ElementRef::new(),
      refocus: Arc::default(),
    };
    self.view(theme, kept, None)
  }

  fn view(self, theme: &Theme, kept: Kept, ctx: Option<&mut Ctx>) -> Element {
    let focused = kept.controller.input.element_ref().focus_signal().get();
    let case = self
      .boxed
      .case(self.states.clone())
      .state(RichTextEditorState::Focused, focused)
      .state(RichTextEditorState::FullWidth, self.full_width);
    let recipe = theme.resolve::<RichTextEditor>(&case, Some(&self.rules));
    let input = &kept.controller.input;
    let editable = !self.boxed.read_only && !self.boxed.disabled;
    let format = RichTextFormat {
      marks: input.selection_style().ok().flatten().unwrap_or_default(),
      paragraphs: input.selected_paragraphs().ok().flatten().unwrap_or_default(),
      can_undo: editable && input.can_undo(),
      can_redo: editable && input.can_redo(),
    };
    // A picture is no word, and stands between two.
    let words = kept.objects.with_untracked(|objects| {
      kept.text.with_untracked(|text| {
        if objects.is_empty() {
          text.split_whitespace().count()
        } else {
          RichText::plain(text.clone()).with_objects(objects.clone()).word_count()
        }
      })
    });
    let mut view = RichTextEditorView {
      placeholder: self.placeholder,
      min_rows: self.min_rows,
      max_rows: self.max_rows,
      tools: self.tools,
      has_toolbar: self.toolbar,
      has_footer: self.footer,
      markdown_shortcuts: self.markdown_shortcuts,
      read_only: self.boxed.read_only,
      disabled: self.boxed.disabled,
      invalid: self.boxed.judgment() == Some(Judgment::Invalid),
      focused,
      format,
      word_count: words,
      recipe,
      toolbar: None,
      held: self.value,
      on_change: self.on_change,
      on_link_click: self.on_link_click,
      on_image: self.on_image,
      picture: self.picture,
      on_paste_image: self.on_paste_image,
      boxed: self.boxed,
      words: self.words,
      forced: self.forced,
      kept,
      name: self.name,
      id: self.id,
      key: self.key,
    };
    if view.has_toolbar {
      let bar = view.bar();
      view.toolbar = Some(match ctx {
        Some(ctx) => bar.overflow(true).mount(ctx),
        None => bar.build(theme),
      });
    }
    match theme.renderer::<RichTextEditor>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<RichTextEditor> for Element {
  fn from(editor: RichTextEditor) -> Self {
    editor.build(&current_theme())
  }
}

impl FieldControl for RichTextEditor {
  /// Called by the field's label unless it has a name of its own, judged as the field is unless the app judged
  /// the editor itself, and disabled where the field is. A field converted into an element holds the still form
  /// of it; a field the app mounts ([`Field::mount`](super::Field::mount)) mounts it.
  fn in_field(mut self, field: &InField) -> Element {
    self.boxed.in_field(field);
    self.into()
  }

  fn mount_in_field(mut self, field: &InField, ctx: &mut Ctx) -> Element {
    self.boxed.in_field(field);
    self.mount(ctx)
  }
}

/// What an editor keeps from one render to the next: the signals the engine's input edits, its controller, what
/// it last wrote to the app or read from it, and the panel of a link.
#[derive(Clone)]
struct Kept {
  text: Signal<String>,
  spans: Signal<Vec<StyledSpan>>,
  kinds: Signal<Vec<ParagraphKind>>,
  /// The pictures of the document, each at its character of the text.
  objects: Signal<Vec<InlineObject>>,
  controller: RichTextController,
  /// The document the app's value and the signals last agreed on.
  last: Arc<Mutex<Option<RichText>>>,
  link_open: Signal<bool>,
  link_target: Signal<String>,
  /// Where the engine puts the start of the selection: what the panel of a link hangs from.
  link_anchor: ElementRef,
  link_field: ElementRef,
  /// Set by what takes the focus away from the text and gives it back: the style's control, the panel of a
  /// link, a tool run from the menu of those that did not fit.
  refocus: Arc<AtomicBool>,
}

#[derive(Clone)]
struct RichTextEditorProps {
  editor: RichTextEditor,
}

/// The editor is made anew by every render of what holds it, so props are never the same as before.
impl PartialEq for RichTextEditorProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

#[cfg(feature = "devtools")]
impl crate::app::component::DevtoolsInspectable for RichTextEditorProps {}

struct RichTextEditorHost {
  kept: Kept,
  /// Whether the panel of a link was open at the last render: it takes the focus as it opens.
  link_was_open: AtomicBool,
}

impl Component for RichTextEditorHost {
  type Props = RichTextEditorProps;

  fn create(_: &mut Ctx) -> Self {
    Self {
      kept: Kept {
        text: Signal::new(String::new()),
        spans: Signal::new(Vec::new()),
        kinds: Signal::new(Vec::new()),
        objects: Signal::new(Vec::new()),
        controller: RichTextController::new(),
        last: Arc::default(),
        link_open: Signal::new(false),
        link_target: Signal::new(String::new()),
        link_anchor: ElementRef::new(),
        link_field: ElementRef::new(),
        refocus: Arc::default(),
      },
      link_was_open: AtomicBool::new(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let RichTextEditorProps { editor } = ctx.props::<RichTextEditorProps>().clone();
    let mut kept = self.kept.clone();
    if let Some(controller) = &editor.controller {
      kept.controller = controller.clone();
    }
    // Read so that the editor is rendered again when the caret moves or a tool is run.
    kept.controller.tick.get();
    // What the app holds is what is shown: a value that is not what the editor last wrote is loaded.
    let held = match &editor.value {
      Held::Bound(signal) => signal.get(),
      Held::Controlled(value) => value.clone(),
    };
    let stale = kept.last.lock().as_ref() != Some(&held);
    if stale {
      let document = RichText::new(held.text.clone(), held.spans.clone(), held.paragraphs.clone())
        .with_objects(held.objects.clone());
      if kept.text.with_untracked(|text| *text != document.text) {
        kept.text.set(document.text);
      }
      if kept.objects.with_untracked(|objects| *objects != document.objects) {
        kept.objects.set(document.objects);
      }
      if kept.spans.with_untracked(|spans| *spans != document.spans) {
        kept.spans.set(document.spans);
      }
      if kept.kinds.with_untracked(|kinds| *kinds != document.paragraphs) {
        kept.kinds.set(document.paragraphs);
      }
      *kept.last.lock() = Some(held);
    }
    let link_open = kept.link_open.get();
    if link_open && !self.link_was_open.swap(link_open, Ordering::Relaxed) {
      ctx.focus(&kept.link_field);
    } else if !link_open {
      self.link_was_open.store(false, Ordering::Relaxed);
    }
    if kept.refocus.swap(false, Ordering::Relaxed) {
      kept.controller.input.focus(ctx);
    }
    editor.view(&current_theme(), kept, Some(ctx))
  }
}

/// What the selection is, as the bar shows it.
#[derive(Clone, Debug, Default)]
pub struct RichTextFormat {
  /// The marks over the selection, or of what is typed next.
  pub marks: SelectionStyle,
  /// What each paragraph the selection touches is.
  pub paragraphs: Vec<ParagraphKind>,
  pub can_undo: bool,
  pub can_redo: bool,
}

impl RichTextFormat {
  /// Whether every paragraph the selection touches is a `kind`.
  pub fn all(&self, kind: ParagraphKind) -> bool {
    !self.paragraphs.is_empty() && self.paragraphs.iter().all(|held| *held == kind)
  }

  /// The style the control of a paragraph's style shows: the level of a heading every paragraph is, 9 for a
  /// block of code, and 0 for anything else.
  fn style(&self) -> u8 {
    match self.paragraphs.first() {
      Some(ParagraphKind::Heading(level)) if self.all(ParagraphKind::Heading(*level)) => (*level).min(3),
      Some(ParagraphKind::Code) if self.all(ParagraphKind::Code) => CODE_STYLE,
      _ => 0,
    }
  }
}

/// The value the style's control has for a block of code.
const CODE_STYLE: u8 = 9;

fn style_kind(style: u8) -> ParagraphKind {
  match style {
    0 => ParagraphKind::Body,
    CODE_STYLE => ParagraphKind::Code,
    level => ParagraphKind::Heading(level),
  }
}

/// A rich text editor as a renderer sees it: what it was given, what its selection is, the style of its parts,
/// and its pieces ready to be laid out.
pub struct RichTextEditorView {
  pub placeholder: Option<Arc<str>>,
  pub min_rows: usize,
  pub max_rows: usize,
  /// The tools of the bar, in order.
  pub tools: Vec<RichTextTool>,
  pub has_toolbar: bool,
  pub has_footer: bool,
  pub markdown_shortcuts: bool,
  pub read_only: bool,
  pub disabled: bool,
  pub invalid: bool,
  /// Whether the keyboard's focus is in the box.
  pub focused: bool,
  /// What the selection is.
  pub format: RichTextFormat,
  /// How many words the document has.
  pub word_count: usize,
  /// The style of each part of this editor.
  pub recipe: ResolvedRecipe,
  /// The bar of tools, built: `None` where the editor has none.
  pub toolbar: Option<Element>,
  held: Held,
  on_change: Vec<ChangeHandler>,
  on_link_click: Vec<LinkHandler>,
  on_image: Vec<Run>,
  picture: Option<PictureBuilder>,
  on_paste_image: Option<ImageKeeper>,
  boxed: BoxStates,
  words: Words,
  forced: Option<Interaction>,
  kept: Kept,
  // Read only where the engine has forms.
  #[cfg_attr(not(feature = "form"), allow(dead_code))]
  name: Option<Arc<str>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl RichTextEditorView {
  /// The style of a part, as this editor is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  fn sub_id(&self, suffix: &str) -> Option<String> {
    self.id.as_ref().map(|id| format!("{id}-{suffix}"))
  }

  fn editable(&self) -> bool {
    !self.read_only && !self.disabled
  }

  /// What a tool does when it is run: the command, and then the editor drawn again with the focus in its text.
  fn run(&self, command: impl Fn(&RichTextController) + Send + Sync + 'static) -> Run {
    let (controller, refocus, editable) = (self.kept.controller.clone(), self.kept.refocus.clone(), self.editable());
    Arc::new(move || {
      if !editable {
        return;
      }
      command(&controller);
      refocus.store(true, Ordering::Relaxed);
      controller.redraw();
    })
  }

  /// What opens the panel of a link: its field shows the target of the link the selection is.
  fn open_link(&self) -> Run {
    let kept = self.kept.clone();
    let editable = self.editable();
    Arc::new(move || {
      if !editable {
        return;
      }
      let target = kept
        .controller
        .input
        .selection_style()
        .ok()
        .flatten()
        .and_then(|style| style.link)
        .map(|target| target.to_string())
        .unwrap_or_default();
      kept.link_target.set(target);
      kept.link_open.set(true);
    })
  }

  /// A tool that is on or off: a toggle of the bar, its icon alone, which leaves the focus in the text.
  fn toggle(&self, label: &str, icon: &'static str, suffix: &str, on: bool, run: Run) -> Toggle {
    let lit = self.part(RichTextEditorPart::ToolOn).rest();
    let rules = self
      .recipe
      .rules_for(RichTextEditorPart::ToolIcon, TogglePart::Icon)
      .state(super::toggle::ToggleState::On, TogglePart::Root, move |s| {
        s.merged(&lit)
      });
    let mut toggle = Toggle::new(label)
      .icon(icon)
      .icon_only(true)
      .on(on)
      .rules(&rules)
      .focuses(&self.kept.controller.input.element_ref())
      .on_change(move |_| run());
    if let Some(id) = self.sub_id(suffix) {
      toggle = toggle.id(id);
    }
    toggle
  }

  /// A tool that is run: a button of the bar, its icon alone, which leaves the focus in the text.
  fn button(&self, label: &str, icon: &'static str, suffix: &str, disabled: bool, run: Run) -> Button {
    let rules = self.recipe.rules_for(RichTextEditorPart::ToolIcon, ButtonPart::Icon);
    let mut button = Button::new(label)
      .icon(icon)
      .icon_only(true)
      .disabled(disabled)
      .rules(&rules)
      .focuses(&self.kept.controller.input.element_ref())
      .on_click(move |_| run());
    if let Some(id) = self.sub_id(suffix) {
      button = button.id(id);
    }
    button
  }

  /// The control that says what the paragraph is and makes it something else: a small select of the styles.
  pub fn style_select(&self) -> Element {
    let run = {
      let (controller, refocus, editable) = (self.kept.controller.clone(), self.kept.refocus.clone(), self.editable());
      move |style: &u8| {
        if editable {
          controller.set_paragraphs(style_kind(*style));
          refocus.store(true, Ordering::Relaxed);
          controller.redraw();
        }
      }
    };
    let heading = &self.words.heading;
    let options = [
      (0, self.words.paragraph.to_string()),
      (1, heading(1)),
      (2, heading(2)),
      (3, heading(3)),
      (CODE_STYLE, self.words.code_block.to_string()),
    ];
    let rules = self
      .recipe
      .rules_for(RichTextEditorPart::Style, SelectPart::Trigger)
      .extend(
        &self
          .recipe
          .rules_for(RichTextEditorPart::StyleChevron, SelectPart::Chevron),
      )
      .extend(&self.recipe.rules_for(RichTextEditorPart::StyleList, SelectPart::Panel));
    let mut select = Select::controlled(self.format.style(), options, run)
      .label(&self.words.style)
      .disabled(!self.editable())
      // The control is as wide as the one name it shows: its list is as wide as the longest of them.
      .fit_options(true)
      .rules(&rules);
    if let Some(id) = self.sub_id("style") {
      select = select.id(id);
    }
    select.build(self.theme())
  }

  /// One tool as the bar holds it.
  fn tool(&self, tool: RichTextTool) -> ToolbarItem {
    let all = |presence: MarkPresence| presence == MarkPresence::All;
    let marks = &self.format.marks;
    let mark = |mark: TextMark| {
      self.run(move |editor| {
        editor.toggle_mark(mark);
      })
    };
    let block = |kind: ParagraphKind| {
      self.run(move |editor| {
        editor.toggle_paragraphs(kind);
      })
    };
    let command = if cfg!(target_vendor = "apple") { "Cmd" } else { "Ctrl" };
    let keys = |keys: &str| format!("{command}+{keys}");
    let words = &self.words;
    match tool {
      RichTextTool::Style => ToolbarItem::element(self.style_select()),
      RichTextTool::Bold => {
        ToolbarItem::from(self.toggle(&words.bold, icons::BOLD, "bold", all(marks.bold), mark(TextMark::Bold)))
          .shortcut(&keys("B"))
      }
      RichTextTool::Italic => ToolbarItem::from(self.toggle(
        &words.italic,
        icons::ITALIC,
        "italic",
        all(marks.italic),
        mark(TextMark::Italic),
      ))
      .shortcut(&keys("I")),
      RichTextTool::Underline => ToolbarItem::from(self.toggle(
        &words.underline,
        icons::UNDERLINE,
        "underline",
        all(marks.underline),
        mark(TextMark::Underline),
      ))
      .shortcut(&keys("U")),
      RichTextTool::Strikethrough => ToolbarItem::from(self.toggle(
        &words.strikethrough,
        icons::STRIKETHROUGH,
        "strikethrough",
        all(marks.strikethrough),
        mark(TextMark::Strikethrough),
      ))
      .shortcut(&keys("Shift+X")),
      RichTextTool::Bullet => ToolbarItem::from(self.toggle(
        &words.bullet,
        icons::LIST,
        "bullet",
        self.format.all(ParagraphKind::Bullet),
        block(ParagraphKind::Bullet),
      ))
      .shortcut(&keys("Shift+8")),
      RichTextTool::Ordered => ToolbarItem::from(self.toggle(
        &words.ordered,
        icons::LIST_NUMBERED,
        "ordered",
        self.format.all(ParagraphKind::Ordered),
        block(ParagraphKind::Ordered),
      ))
      .shortcut(&keys("Shift+7")),
      RichTextTool::Quote => ToolbarItem::from(self.toggle(
        &words.quote,
        icons::QUOTE,
        "quote",
        self.format.all(ParagraphKind::Quote),
        block(ParagraphKind::Quote),
      ))
      .shortcut(&keys("Shift+9")),
      RichTextTool::Code => {
        ToolbarItem::from(self.toggle(&words.code, icons::CODE, "code", all(marks.code), mark(TextMark::Code)))
          .shortcut(&keys("E"))
      }
      RichTextTool::Link => {
        ToolbarItem::from(self.toggle(&words.link, icons::LINK, "link", marks.has_link, self.open_link()))
          .shortcut(&keys("K"))
      }
      RichTextTool::Image => {
        let handlers = self.on_image.clone();
        let run = self.run(move |_| {
          for handler in &handlers {
            handler();
          }
        });
        ToolbarItem::from(self.button(&words.image, icons::IMAGE, "image", false, run))
      }
      RichTextTool::Undo => ToolbarItem::from(self.button(
        &words.undo,
        icons::UNDO,
        "undo",
        !self.format.can_undo,
        self.run(|editor| {
          editor.undo();
        }),
      ))
      .shortcut(&keys("Z")),
      RichTextTool::Redo => ToolbarItem::from(self.button(
        &words.redo,
        icons::REDO,
        "redo",
        !self.format.can_redo,
        self.run(|editor| {
          editor.redo();
        }),
      ))
      .shortcut(&keys("Y")),
    }
  }

  /// The bar of tools: a toolbar in the editor's dress, a line between two groups of tools and the history at
  /// its end. Its tools take no input while the editor is read only or disabled.
  pub fn bar(&self) -> Toolbar {
    let rules = self
      .recipe
      .rules_for(RichTextEditorPart::Toolbar, ToolbarPart::Root)
      .extend(&self.recipe.rules_for(RichTextEditorPart::Tool, ToolbarPart::Item))
      .extend(&self.recipe.rules_for(RichTextEditorPart::Tool, ToolbarPart::MoreButton));
    let mut bar = Toolbar::new()
      .label(&self.words.toolbar)
      .more_label(&self.words.more)
      .size(ButtonSize::Sm)
      .disabled(!self.editable())
      .rules(&rules);
    let history = RichTextTool::Undo.group();
    let mut group = None;
    for tool in &self.tools {
      if tool.group() == history && group != Some(history) {
        bar = bar.item(Toolbar::spacer());
      } else if group.is_some_and(|before| before != tool.group()) {
        bar = bar.item(Toolbar::separator());
      }
      group = Some(tool.group());
      bar = bar.item(self.tool(*tool));
    }
    if let Some(id) = self.sub_id("toolbar") {
      bar = bar.id(id);
    }
    bar
  }

  /// How each kind of paragraph is drawn, from the parts of the recipe: the content's type with what each
  /// kind's part lays over it, the room before its text, its marker, and the room between two paragraphs.
  pub fn paragraph_styles(&self) -> ParagraphStyles {
    let theme = self.theme();
    let content = self.part(RichTextEditorPart::Content).rest();
    let px = |value: &Option<crate::node::spacing_value::SpacingValue>| {
      value.as_ref().map_or(0.0, |value| value.resolve(&theme.spacing(), 0.0))
    };
    let length = |length: Option<Length>| match length.and_then(|length| length.resolve(theme)) {
      Some(crate::node::dimension::Dimension::Px(px)) => px,
      _ => 0.0,
    };
    let ink =
      |part: &PartStyle, fallback: PaletteColor| part.foreground.clone().unwrap_or(TextColor::Palette(fallback));
    let kind = |part: RichTextEditorPart| {
      let over = self.part(part).rest();
      let text = content.clone().merged(&over).text.resolve(theme);
      let mut style = ParagraphStyle::new()
        .font_size(text.font_size)
        .weight(text.weight)
        .line_height(text.line_height)
        .font_family(text.font_family.clone())
        .indent(px(&over.padding_start));
      if let Some(color) = &over.foreground {
        style = style.color(color.clone());
      }
      (style, over)
    };
    let dot = self.part(RichTextEditorPart::BulletMark).rest();
    let number = self.part(RichTextEditorPart::OrderedMark).rest();
    let bar = self.part(RichTextEditorPart::QuoteMark).rest();
    let (code, code_part) = kind(RichTextEditorPart::CodeBlock);
    let fill = match &code_part.background {
      Some(crate::node::BackgroundColor::Palette(role)) => Some(TextColor::Palette(role.clone())),
      Some(crate::node::BackgroundColor::Color(color)) => Some(TextColor::Color(*color)),
      _ => None,
    };
    let code = match fill {
      Some(fill) => {
        let radius = code_part
          .radius
          .as_ref()
          .map_or(0.0, |radius| radius.resolve(&theme.radii()).top_left);
        code.background(fill, radius)
      }
      None => code,
    }
    .joined(true);
    ParagraphStyles::new()
      .gap(px(&content.gap))
      .with(ParagraphKind::Heading(1), kind(RichTextEditorPart::Heading1).0)
      .with(ParagraphKind::Heading(2), kind(RichTextEditorPart::Heading2).0)
      .with(ParagraphKind::Heading(3), kind(RichTextEditorPart::Heading3).0)
      .with(ParagraphKind::Heading(4), kind(RichTextEditorPart::Heading3).0)
      .with(ParagraphKind::Heading(5), kind(RichTextEditorPart::Heading3).0)
      .with(ParagraphKind::Heading(6), kind(RichTextEditorPart::Heading3).0)
      .with(
        ParagraphKind::Bullet,
        kind(RichTextEditorPart::Bullet).0.marker(ParagraphMarker::Dot {
          size: length(dot.width),
          gap: px(&dot.gap),
          color: ink(&dot, PaletteColor::MutedForeground),
        }),
      )
      .with(
        ParagraphKind::Ordered,
        kind(RichTextEditorPart::Ordered).0.marker(ParagraphMarker::Number {
          gap: px(&number.gap),
          color: ink(&number, PaletteColor::MutedForeground),
        }),
      )
      .with(
        ParagraphKind::Quote,
        kind(RichTextEditorPart::Quote).0.marker(ParagraphMarker::Bar {
          width: length(bar.width),
          color: ink(&bar, PaletteColor::Border),
        }),
      )
      .with(ParagraphKind::Code, code)
  }

  /// The box that is typed in: the engine's styled text input with kinds of paragraphs and objects, bound to
  /// what the editor keeps, with the editor's keys. Each change it makes is written to the app's signal and told
  /// to its handlers. It is a `textbox` of several lines to what reads the tree, and each picture in it a child
  /// of it.
  pub fn content(&self) -> TextInput {
    let part = self.part(RichTextEditorPart::Content).forced(self.forced);
    let text = text_box::ink(self.theme(), &part.rest());
    let placeholder = text_box::ink(
      self.theme(),
      &part.rest().merged(&self.part(RichTextEditorPart::Placeholder).rest()),
    );
    let kept = &self.kept;
    // The panel of a link hangs from where the selection starts: read before the controller is bound to the
    // input built here, which has no selection of its own yet.
    let start = kept
      .controller
      .input
      .selection()
      .map_or(0, |selection| selection.range().start);
    let picture = self.picture.clone();
    let input = TextInput::styled(kept.text.clone(), text)
      .placeholder_style(placeholder)
      .spans(kept.spans.clone())
      .paragraphs(kept.kinds.clone(), self.paragraph_styles())
      .objects(kept.objects.clone(), move |object| match &picture {
        Some(picture) => picture(object),
        None => picture_of(object),
      })
      // A selected picture is made wider or narrower by the handle on its corner.
      .resizable_objects(self.editable())
      // What is copied keeps its marks and kinds beside its text, and comes back with them.
      .rich_clipboard(true)
      .controller(kept.controller.input.clone())
      .multiline()
      .rows(self.min_rows, self.max_rows)
      .part(&part)
      .describe("a11y_multiline", "true");
    let mut input = text_box::text_box(input, &self.boxed, Vec::new());
    if let Some(placeholder) = &self.placeholder {
      input = input.placeholder(placeholder);
    }
    if let Some(id) = self.sub_id("content") {
      input = input.id(id);
    }
    input = input.anchor_at(kept.link_anchor.clone(), start);
    let changed = {
      let (held, last, handlers) = (self.held.clone(), kept.last.clone(), self.on_change.clone());
      let controller = kept.controller.clone();
      move |change: &StyledChange| {
        let document = change.to_rich_text();
        *last.lock() = Some(document.clone());
        if let Held::Bound(signal) = &held
          && signal.with_untracked(|held| *held != document)
        {
          signal.set(document.clone());
        }
        for handler in &handlers {
          handler(&document);
        }
        controller.redraw();
      }
    };
    let clicked = self.on_link_click.clone();
    let (moved, pressed) = (kept.controller.clone(), kept.controller.clone());
    let keys = self.keys();
    input
      .on_styled_change(changed)
      .on_link_click(move |link: &LinkClick| {
        for handler in &clicked {
          handler(&link.target);
        }
      })
      .on_key_down(move |event: KeyboardEvent| keys(&event))
      // The bar shows what the selection is: it is drawn again once a key or the pointer has moved the caret.
      .on_key_up(move |event: KeyboardEvent| {
        if event.focus_within {
          moved.redraw();
        }
      })
      .on_mouse_up(move |_: MouseEvent| pressed.redraw())
      .on_paste(self.paste_pictures())
  }

  /// What a paste that holds no words does: pixels are given to the app to keep and put in as the picture it
  /// answers with, and files that are pictures are put in by their paths. Anything else is left to the text.
  fn paste_pictures(&self) -> impl Fn(PasteEvent) + Send + Sync + 'static {
    let (keep, editor, editable) = (
      self.on_paste_image.clone(),
      self.kept.controller.clone(),
      self.editable(),
    );
    move |event: PasteEvent| {
      if !editable || event.default_prevented() || event.text().is_some_and(|text| !text.is_empty()) {
        return;
      }
      // Each picture to put in: its source, and what it is called.
      let mut pictures: Vec<(String, String)> = Vec::new();
      match (event.image(), &keep) {
        (Some(image), Some(keep)) => pictures.extend(keep(image).map(|source| (source, String::new()))),
        _ => {
          for path in event.files() {
            let extension = path
              .extension()
              .map(|extension| extension.to_string_lossy().to_lowercase());
            if extension.is_some_and(|extension| {
              crate::files::FileKind::of_extension(&extension) == crate::files::FileKind::Image
            }) {
              let name = path.file_stem().map(|name| name.to_string_lossy().into_owned());
              pictures.push((path.to_string_lossy().into_owned(), name.unwrap_or_default()));
            }
          }
        }
      }
      if pictures.is_empty() {
        return;
      }
      event.prevent_default();
      for (source, label) in pictures {
        editor.insert_picture(&source, &label);
      }
    }
  }

  /// What the keys do in the box, besides what they do in any text: the marks, the kinds of paragraphs, the
  /// link, and the Markdown shortcuts.
  fn keys(&self) -> impl Fn(&KeyboardEvent) + Send + Sync + 'static {
    let kept = self.kept.clone();
    let (editable, shortcuts) = (self.editable(), self.markdown_shortcuts);
    let open_link = self.open_link();
    move |event: &KeyboardEvent| {
      if !event.focus_within || event.default_prevented() || event.composing || !editable {
        return;
      }
      let editor = &kept.controller;
      let command = if cfg!(target_vendor = "apple") {
        event.meta
      } else {
        event.ctrl
      };
      let handled = if command {
        command_key(editor, event, &open_link)
      } else if event.alt || event.meta || event.ctrl {
        false
      } else {
        match event.key.as_str() {
          "Enter" if !event.shift => enter_key(&kept, shortcuts),
          "Backspace" => backspace_key(editor),
          " " if shortcuts => block_shortcut(&kept),
          "*" | "_" | "`" | "~" if shortcuts => mark_shortcut(&kept, &event.key),
          _ => false,
        }
      };
      if handled {
        event.prevent_default();
        editor.redraw();
      }
    }
  }

  /// The panel a link's target is typed in, hung from the start of the selection: Enter and its button give
  /// the selection the link, and a link the selection has can be taken away.
  pub fn link_panel(&self) -> Element {
    let kept = self.kept.clone();
    let close = {
      let kept = kept.clone();
      move || {
        kept.link_open.set(false);
        kept.refocus.store(true, Ordering::Relaxed);
        kept.controller.redraw();
      }
    };
    let apply: Run = {
      let (kept, close) = (kept.clone(), close.clone());
      Arc::new(move || {
        let target = kept.link_target.get_untracked();
        let target = target.trim();
        kept
          .controller
          .set_link(if target.is_empty() { None } else { Some(target) });
        close();
      })
    };
    let remove: Run = {
      let (kept, close) = (kept.clone(), close.clone());
      Arc::new(move || {
        kept.controller.set_link(None);
        close();
      })
    };
    let mut field = Input::new(kept.link_target.clone())
      .size(InputSize::Sm)
      .full_width(true)
      .placeholder(&self.words.link_placeholder)
      .label(&self.words.link_placeholder);
    if let Some(id) = self.sub_id("link-field") {
      field = field.id(id);
    }
    let view = field.view(self.theme());
    let on_enter = apply.clone();
    let control = view
      .control()
      .ref_element(kept.link_field.clone())
      .on_key_down(move |event: KeyboardEvent| {
        if event.focus_within && event.key == "Enter" && !event.composing && !(event.ctrl || event.alt || event.meta) {
          event.prevent_default();
          on_enter();
        }
      });
    let mut row = Row::new()
      .align_items(Alignment::Center)
      .spacing(6.0)
      .child(
        Row::new()
          .flex(1.0)
          .min_width(0.0)
          .align_items(Alignment::Stretch)
          .child(view.render_with_end(control, None)),
      )
      .child({
        let apply = apply.clone();
        Button::new(&self.words.apply)
          .size(ButtonSize::Sm)
          .on_click(move |_| apply())
          .build(self.theme())
      });
    if self.format.marks.has_link {
      row = row.child(
        Button::new(&self.words.remove_link)
          .variant(ButtonVariant::Ghost)
          .size(ButtonSize::Sm)
          .on_click(move |_| remove())
          .build(self.theme()),
      );
    }
    let dismissed = close.clone();
    Popover::new(kept.link_open.clone(), kept.link_anchor.clone())
      .label(&self.words.link)
      .placement(Placement::BottomStart)
      .rules(&self.recipe.rules_for(RichTextEditorPart::LinkPanel, PopoverPart::Panel))
      .on_dismiss(move || dismissed())
      .child(row)
      .build(self.theme())
  }

  /// The row under the box: that Markdown shortcuts are on, where they are, and the count of the words.
  pub fn footer(&self) -> Row {
    let part = self.part(RichTextEditorPart::Footer);
    let text = self.part(RichTextEditorPart::FooterText);
    let line = |words: &str| {
      Row::new().flex_shrink(1.0).min_width(0.0).child(
        text
          .text(words)
          .nowrap()
          .text_overflow(crate::node::node_kind::TextOverflow::Elipsis)
          .part(&text),
      )
    };
    let mut row = Row::new()
      .part(&part)
      .align_items(Alignment::Center)
      .justify(Justify::SpaceBetween);
    row = row.child(if self.markdown_shortcuts {
      line(&self.words.shortcuts)
    } else {
      Row::new()
    });
    let mut count = Row::new()
      .describe("a11y_live", "polite")
      .child(text.text(&(self.words.count)(self.word_count)).nowrap().part(&text));
    if let Some(id) = self.sub_id("count") {
      count = count.id(id);
    }
    row.child(count)
  }

  /// The editor's box, with nothing in it yet: a group to what reads the tree, called by its label.
  pub fn root(&self) -> Column {
    let mut root = Column::new()
      .align_items(Alignment::Stretch)
      .part(&self.part(RichTextEditorPart::Root))
      .describe("a11y_role", "group");
    if let Some(label) = self.boxed.label.as_ref().or(self.words.label.as_ref()) {
      root = root.describe("a11y_name", label.clone());
    }
    // In a form the editor's value is the document as Markdown, under its name.
    #[cfg(feature = "form")]
    if let Some(name) = &self.name {
      let kept = &self.kept;
      let document = RichText::new(
        kept.text.get_untracked(),
        kept.spans.get_untracked(),
        kept.kinds.get_untracked(),
      );
      root = root
        .name(name.clone())
        .form_value(Some(Arc::from(document.to_markdown())));
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root
  }

  /// How the default renderer builds an editor: the bar of tools, the box that is typed in, the footer, and
  /// the panel of a link, which is a layer of its own.
  pub fn render(mut self) -> Element {
    let mut root = self.root();
    if let Some(toolbar) = self.toolbar.take() {
      root = root.child(toolbar);
    }
    root = root.child(self.content());
    if self.has_footer {
      root = root.child(self.footer());
    }
    root.child(self.link_panel()).into()
  }
}

/// A key with Ctrl (Cmd on a Mac): the marks, the link, the kinds of paragraphs.
fn command_key(editor: &RichTextController, event: &KeyboardEvent, open_link: &Run) -> bool {
  let key = event.key.to_ascii_lowercase();
  let digit = event
    .code
    .strip_prefix("Digit")
    .and_then(|digit| digit.parse::<u8>().ok());
  match (key.as_str(), event.shift, event.alt) {
    ("b", false, false) => {
      editor.toggle_mark(TextMark::Bold);
    }
    ("i", false, false) => {
      editor.toggle_mark(TextMark::Italic);
    }
    ("u", false, false) => {
      editor.toggle_mark(TextMark::Underline);
    }
    ("e", false, false) => {
      editor.toggle_mark(TextMark::Code);
    }
    ("x", true, false) => {
      editor.toggle_mark(TextMark::Strikethrough);
    }
    ("k", false, false) => open_link(),
    (_, false, true) if digit.is_some_and(|digit| digit <= 3) => {
      {
        editor.set_paragraphs(style_kind(digit.unwrap_or(0)));
      };
    }
    (_, true, false) if digit == Some(7) => {
      editor.toggle_paragraphs(ParagraphKind::Ordered);
    }
    (_, true, false) if digit == Some(8) => {
      editor.toggle_paragraphs(ParagraphKind::Bullet);
    }
    (_, true, false) if digit == Some(9) => {
      editor.toggle_paragraphs(ParagraphKind::Quote);
    }
    _ => return false,
  }
  true
}

/// The paragraph the caret is in where nothing is selected: its kind, where its text is, and the caret.
fn caret_paragraph(input: &TextInputController) -> Option<(ParagraphKind, std::ops::Range<usize>, usize)> {
  let selection = input.selection().ok()?;
  if !selection.range().is_empty() {
    return None;
  }
  let (kind, range) = input.caret_paragraph().ok()??;
  Some((kind, range, selection.caret))
}

/// Enter: in an empty item or quote it ends the list, at the end of a heading it starts a paragraph, and after
/// three backticks it starts a block of code. Anywhere else it is the text input's own.
fn enter_key(kept: &Kept, shortcuts: bool) -> bool {
  let input = &kept.controller.input;
  // A picture selected alone is not what Enter takes the place of: the paragraph starts after it.
  if let Ok(selection) = input.selection() {
    let range = selection.range();
    let pictured = kept
      .objects
      .with_untracked(|objects| objects.iter().any(|object| object.range() == range));
    if pictured {
      let _ = input.select(range.end, range.end);
    }
  }
  let Some((kind, range, caret)) = caret_paragraph(input) else {
    return false;
  };
  match kind {
    ParagraphKind::Bullet | ParagraphKind::Ordered | ParagraphKind::Quote if range.is_empty() => {
      input.set_paragraphs(ParagraphKind::Body).unwrap_or(false)
    }
    ParagraphKind::Heading(_) if caret == range.end && !range.is_empty() => {
      input.replace_selection("\n").unwrap_or(false) && {
        let _ = input.set_paragraphs(ParagraphKind::Body);
        true
      }
    }
    ParagraphKind::Body if shortcuts && kept.text.with_untracked(|text| text.get(range.clone()) == Some("```")) => {
      input.select(range.start, range.end).is_ok() && input.replace_selection("").unwrap_or(false) && {
        let _ = input.set_paragraphs(ParagraphKind::Code);
        true
      }
    }
    _ => false,
  }
}

/// Backspace at the start of anything but a paragraph of body text makes it one.
fn backspace_key(editor: &RichTextController) -> bool {
  match caret_paragraph(&editor.input) {
    Some((kind, range, caret)) if kind != ParagraphKind::Body && caret == range.start => {
      editor.input.set_paragraphs(ParagraphKind::Body).unwrap_or(false)
    }
    _ => false,
  }
}

/// What a paragraph that starts with `prefix` and a space is made.
fn block_of(prefix: &str) -> Option<ParagraphKind> {
  let number = prefix.strip_suffix(['.', ')']);
  match prefix {
    "#" => Some(ParagraphKind::Heading(1)),
    "##" => Some(ParagraphKind::Heading(2)),
    "###" => Some(ParagraphKind::Heading(3)),
    "-" | "*" | "+" => Some(ParagraphKind::Bullet),
    ">" => Some(ParagraphKind::Quote),
    "```" => Some(ParagraphKind::Code),
    _ if number
      .is_some_and(|digits| !digits.is_empty() && digits.len() <= 3 && digits.bytes().all(|b| b.is_ascii_digit())) =>
    {
      Some(ParagraphKind::Ordered)
    }
    _ => None,
  }
}

/// A space after `#`, `-`, `1.`, `>` at the start of a paragraph of body text: the mark is taken away and the
/// paragraph is what it said.
fn block_shortcut(kept: &Kept) -> bool {
  let input = &kept.controller.input;
  let Some((ParagraphKind::Body, range, caret)) = caret_paragraph(input) else {
    return false;
  };
  let kind = kept
    .text
    .with_untracked(|text| text.get(range.start..caret).and_then(block_of));
  let Some(kind) = kind else {
    return false;
  };
  input.select(range.start, caret).is_ok() && input.replace_selection("").unwrap_or(false) && {
    let _ = input.set_paragraphs(kind);
    true
  }
}

/// The delimiters a mark is typed between, the longest first: what closes one that `key` is the last character of.
fn mark_delimiters(key: &str) -> &'static [(&'static str, TextMark)] {
  match key {
    "*" => &[("**", TextMark::Bold), ("*", TextMark::Italic)],
    "_" => &[("__", TextMark::Bold), ("_", TextMark::Italic)],
    "`" => &[("`", TextMark::Code)],
    "~" => &[("~~", TextMark::Strikethrough)],
    _ => &[],
  }
}

/// Where the text between a pair of `delimiter` ends at the end of `typed`: the start of the opening one. The
/// text is not empty, has no space at its edges, and has none of the delimiter's character in it.
fn delimited(typed: &str, delimiter: &str) -> Option<usize> {
  let inner = typed.strip_suffix(delimiter)?;
  let open = inner.rfind(delimiter)?;
  let text = &inner[open + delimiter.len()..];
  let mark = delimiter.chars().next()?;
  let clean = !text.is_empty()
    && !text.contains(mark)
    && !text.starts_with(char::is_whitespace)
    && !text.ends_with(char::is_whitespace);
  // A longer run of the character before the opening is another delimiter's: `**bold*` is not italic.
  let alone = !inner[..open].ends_with(mark);
  (clean && alone).then_some(open)
}

/// The last character of `**bold**`, `*italic*`, `` `code` `` or `~~struck~~`: the delimiters are taken away
/// and what was between them has the mark, as one edit. What is typed next has it no more.
fn mark_shortcut(kept: &Kept, key: &str) -> bool {
  let input = &kept.controller.input;
  let Some((kind, range, caret)) = caret_paragraph(input) else {
    return false;
  };
  if kind == ParagraphKind::Code {
    return false;
  }
  let Some(typed) = kept
    .text
    .with_untracked(|text| text.get(range.start..caret).map(|before| format!("{before}{key}")))
  else {
    return false;
  };
  let Some((delimiter, mark, open)) = mark_delimiters(key)
    .iter()
    .find_map(|(delimiter, mark)| delimited(&typed, delimiter).map(|open| (*delimiter, *mark, open)))
  else {
    return false;
  };
  let start = range.start + open;
  let inner = start + delimiter.len()..caret + key.len() - delimiter.len();
  if input.select(inner.start, inner.end).is_err() {
    return false;
  }
  let Ok(Some(between)) = input.selected_styled_text() else {
    return false;
  };
  let mut spans = Vec::new();
  let mut at = 0;
  let mut after = crate::components::SpanStyle::default();
  for (run, style) in between.runs() {
    after = style.clone().with(mark, false);
    spans.push(StyledSpan::new(at..at + run.len(), style.with(mark, true)));
    at += run.len();
  }
  let marked = StyledText::new(between.text.clone(), spans);
  if input.select(start, caret).is_err() || !input.replace_selection_styled(&marked).unwrap_or(false) {
    return false;
  }
  // The link of what was marked does not go on either: what follows is plain text of the marks before.
  after.link = None;
  let _ = input.set_pending_style(Some(after));
  true
}

impl Themed for RichTextEditor {
  const NAME: &'static str = "rich_text_editor";
  type Renderer = RichTextEditorRenderer;

  /// The design's rich text editor (`pencil/lurq-design-system.pen`, Rich Text Editor): a box 560 wide in the
  /// background's color, with an edge of 1 in the border's color and the large radius. Its bar is 40 high on
  /// the muted fill over a line of 1, padded 6, its tools 2 apart: each 28 square with the small radius and an
  /// icon of 15, one that is on filled with the background; the style's control is 28 high with the small
  /// radius, padded 10 before its text of 12 and 6 after its chevron of 12. The box is padded 18 across and 16
  /// down, its text the body's 14 on lines of 1.3, its paragraphs 10 apart. A heading of the second level is
  /// 22 semibold (the first 28 bold, the third 18 semibold) on lines of 1.2. An item's text starts 21 in, after
  /// a dot of 5 in the muted foreground and 10 of room; a numbered item's 24 in, 6 after its number; a quote's
  /// 14 in, in the muted foreground, after a bar of 3 in the border's color; a block of code is the mono
  /// family at 13 on the muted fill, 12 in. The footer is 28 high over a line of 1, padded 12, its texts the
  /// mono family at 11 in the muted foreground. Read only, the box is on the muted fill; disabled, the editor
  /// is at half its opacity; a judgment's color is its edge's.
  fn recipe() -> Recipe {
    use PaletteColor::{Background, Foreground, Muted, MutedForeground};
    use RichTextEditorPart::*;
    let edge = |color: PaletteColor| Border::inside(BorderSize::Sm, color);
    let recipe = Recipe::new()
      .base(Root, |s| {
        s.width(560.0)
          .max_width(Length::Fill)
          .background(Background)
          .border(edge(PaletteColor::Border))
          .radius(RadiusSize::Lg)
      })
      .state(RichTextEditorState::FullWidth, Root, |s| s.width(Length::Fill))
      .base(Toolbar, |s| {
        s.height(40.0)
          .padding_x(6.0)
          .padding_y(0.0)
          .gap(2.0)
          .radius(0.0)
          .background(Muted)
          .underline(edge(PaletteColor::Border))
      })
      .base(Tool, |s| s.square(28.0).padding_x(0.0).radius(RadiusSize::Sm))
      // The board fills a tool that is on with what is picked out, which is the bar's own fill in both palettes:
      // it is filled with the background, as the style's control beside it is, so that it shows.
      .base(ToolOn, |s| s.background(Background))
      .base(ToolIcon, |s| s.square(15.0))
      // As wide as the name of the style it shows, as the board's is.
      .base(Style, |s| {
        s.width(Length::Auto)
          .height(28.0)
          .padding_start(10.0)
          .padding_end(6.0)
          .gap(6.0)
          .radius(RadiusSize::Sm)
          .font_size(12.0)
      })
      .base(StyleChevron, |s| s.square(12.0))
      .base(Content, |s| {
        s.width(Length::Fill)
          .padding_x(18.0)
          .padding_y(16.0)
          .gap(10.0)
          .typography(TypographyStyle::Body)
          // Rows of 18 in type of 14, the caret's height: a line in pixels, whatever the font's own line is.
          .line_height(18.0 / 14.0)
          .foreground(Foreground)
          .cursor(CursorIcon::Text)
      })
      .base(Placeholder, |s| s.foreground(MutedForeground))
      .base(Heading1, |s| {
        s.font_size(28.0).weight(FontWeight::Bold).line_height(34.0 / 28.0)
      })
      .base(Heading2, |s| {
        s.font_size(22.0).weight(FontWeight::SemiBold).line_height(26.0 / 22.0)
      })
      .base(Heading3, |s| {
        s.font_size(18.0).weight(FontWeight::SemiBold).line_height(22.0 / 18.0)
      })
      .base(Bullet, |s| s.padding_start(21.0))
      .base(BulletMark, |s| s.width(5.0).gap(10.0).foreground(MutedForeground))
      .base(Ordered, |s| s.padding_start(24.0))
      .base(OrderedMark, |s| s.gap(6.0).foreground(MutedForeground))
      .base(Quote, |s| s.padding_start(14.0).foreground(MutedForeground))
      .base(QuoteMark, |s| s.width(3.0).foreground(PaletteColor::Border))
      .base(CodeBlock, |s| {
        s.padding_start(12.0)
          .font_family(FontFamily::Mono)
          .font_size(13.0)
          .line_height(21.0 / 13.0)
          .background(Muted)
          .radius(RadiusSize::Sm)
      })
      .base(Footer, |s| {
        s.height(28.0)
          .padding_x(12.0)
          .gap(12.0)
          .top_line(edge(PaletteColor::Border))
      })
      .base(FooterText, |s| {
        s.font_family(FontFamily::Mono)
          .font_size(11.0)
          .foreground(MutedForeground)
      })
      .base(LinkPanel, |s| {
        s.width(340.0).max_width(Length::Fill).padding_x(8.0).padding_y(8.0)
      })
      .state(TextBoxState::ReadOnly, Content, |s| s.background(Muted))
      .state(TextBoxState::Disabled, Root, |s| s.opacity(OpacityLevel::Disabled))
      .state(TextBoxState::Disabled, Content, |s| s.cursor(CursorIcon::Default));
    [Judgment::Invalid, Judgment::Warning, Judgment::Valid]
      .into_iter()
      .fold(recipe, |recipe, judgment| {
        recipe.state(TextBoxState::from(judgment), Root, |s| s.border(edge(judgment.color())))
      })
  }
}
