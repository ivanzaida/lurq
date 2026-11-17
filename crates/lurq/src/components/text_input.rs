pub use crate::node::node_kind::{ContentKind, KeyboardKind, ReturnKey, TextInputOverflow, TextInputOverflowAnchor};
use crate::{
  app::theme::CaretMode,
  core::Signal,
  impl_into_node,
  layout::text_style::{FontFeatures, TextAlign, TextStyle},
};

impl_into_node!(TextInput);

/// The numbers of a multiline input's lines, and the bands behind the lines picked out, drawn beside its text where
/// it lays its lines out: a line's number stands at its first row, a wrapped line's band covers all its rows. What a
/// block of code numbers its lines with.
///
/// The numbers stand in the input's start padding, which the app makes as wide as they need, right-aligned `gap`
/// before the text; a band runs across the whole input. Neither is selected, copied or read to tooling.
///
/// ```
/// use lurq::{app::theme::PaletteColor, components::{LineGutter, TextInput}, core::Signal, layout::text_style::TextStyle};
///
/// let code = TextInput::new(Signal::new("fn main() {\n}".to_owned()))
///   .multiline()
///   .padding_left(40.0)
///   .line_gutter(LineGutter::new(TextStyle::default()).marked([2], PaletteColor::Highlight));
/// ```
#[derive(Clone, PartialEq)]
pub struct LineGutter {
  /// The number of the first line.
  pub first_line: usize,
  /// Whether the numbers are drawn; the bands are drawn either way.
  pub numbers: bool,
  /// The room between a number and the text.
  pub gap: f32,
  /// The numbers' type, and their color where `number_color` names none.
  pub style: TextStyle,
  /// The numbers' color.
  pub number_color: Option<crate::node::TextColor>,
  /// The numbers of the lines picked out.
  pub marked: Vec<usize>,
  /// The band behind a line picked out.
  pub marked_band: Option<crate::node::TextColor>,
  /// The number of a line picked out.
  pub marked_number: Option<crate::node::TextColor>,
}

impl LineGutter {
  /// Numbers from 1 in `style`, 16 before the text.
  pub fn new(style: TextStyle) -> Self {
    Self {
      first_line: 1,
      numbers: true,
      gap: 16.0,
      style,
      number_color: None,
      marked: Vec::new(),
      marked_band: None,
      marked_number: None,
    }
  }

  /// The number of the first line.
  pub fn first_line(mut self, number: usize) -> Self {
    self.first_line = number;
    self
  }

  /// Whether the numbers are drawn.
  pub fn numbers(mut self, numbers: bool) -> Self {
    self.numbers = numbers;
    self
  }

  /// The room between a number and the text.
  pub fn gap(mut self, gap: f32) -> Self {
    self.gap = gap;
    self
  }

  /// The numbers' color.
  pub fn color(mut self, color: impl Into<crate::node::TextColor>) -> Self {
    self.number_color = Some(color.into());
    self
  }

  /// Picks lines out by their numbers, with a band of `band` behind each.
  pub fn marked(mut self, lines: impl IntoIterator<Item = usize>, band: impl Into<crate::node::TextColor>) -> Self {
    self.marked.extend(lines);
    self.marked_band = Some(band.into());
    self
  }

  /// The color of a picked line's number.
  pub fn marked_color(mut self, color: impl Into<crate::node::TextColor>) -> Self {
    self.marked_number = Some(color.into());
    self
  }
}

/// Wavy lines under ranges of an input's text: what a spelling checker marks. The ranges are UTF-8 bytes of the
/// value as it is when the input is built; the app finds them again after an edit. They are drawn under the band
/// of the text, a row at a time, and are not part of the value: nothing of them is selected or copied.
///
/// ```
/// use lurq::{app::theme::PaletteColor, components::{InputSquiggles, TextInput}, core::Signal};
///
/// let text = Signal::new("A mispeled word".to_owned());
/// let input = TextInput::new(text).squiggles(InputSquiggles::new([2..10]).color(PaletteColor::Destructive));
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct InputSquiggles {
  pub(crate) ranges: std::sync::Arc<[std::ops::Range<usize>]>,
  pub(crate) color: crate::node::TextColor,
}

impl InputSquiggles {
  /// Lines under `ranges`, in the theme's destructive color.
  pub fn new(ranges: impl IntoIterator<Item = std::ops::Range<usize>>) -> Self {
    Self {
      ranges: ranges.into_iter().collect(),
      color: crate::node::TextColor::Palette(crate::app::theme::PaletteColor::Destructive),
    }
  }

  /// The color of the lines.
  pub fn color(mut self, color: impl Into<crate::node::TextColor>) -> Self {
    self.color = color.into();
    self
  }
}

/// A suggestion drawn after the end of an input's text, in the input's type: what a key would complete what is
/// typed with. It is drawn while the input has the focus and holds text, with the caret at the end and nothing
/// selected, and is no part of the value: the app puts it there when it is taken
/// ([`super::TextInputController::replace_selection`]).
///
/// ```
/// use lurq::{app::theme::PaletteColor, components::{InputGhost, TextInput}, core::Signal};
///
/// let command = Signal::new("cargo ru".to_owned());
/// let input = TextInput::new(command)
///   .single_line()
///   .ghost(InputGhost::new("n --release").color(PaletteColor::MutedForeground).opacity(0.7));
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct InputGhost {
  pub(crate) text: std::sync::Arc<str>,
  pub(crate) color: Option<crate::node::TextColor>,
  pub(crate) opacity: f32,
}

impl InputGhost {
  /// `text` after the end of what is typed, in the color of the placeholder.
  pub fn new(text: &str) -> Self {
    Self {
      text: text.into(),
      color: None,
      opacity: 1.0,
    }
  }

  /// The color of the suggestion.
  pub fn color(mut self, color: impl Into<crate::node::TextColor>) -> Self {
    self.color = Some(color.into());
    self
  }

  /// How opaque the suggestion is, from 0 to 1.
  pub fn opacity(mut self, opacity: f32) -> Self {
    self.opacity = opacity;
    self
  }
}

impl TextInput {
  /// Draws wavy lines under ranges of the text: see [`InputSquiggles`].
  pub fn squiggles(mut self, squiggles: InputSquiggles) -> Self {
    self.update_node(|node| {
      if let crate::node::node_kind::NodeKind::TextInput { state, .. } = node.node_kind() {
        state.set_squiggles(Some(squiggles));
      }
    });
    self
  }

  /// Draws a suggestion after the end of the text while the caret is there: see [`InputGhost`]. A plain input's:
  /// one with tokens or styled spans draws none.
  pub fn ghost(mut self, ghost: InputGhost) -> Self {
    self.update_node(|node| {
      if let crate::node::node_kind::NodeKind::TextInput { state, .. } = node.node_kind() {
        state.set_ghost(Some(ghost));
      }
    });
    self
  }

  /// Draw source-backed inline pills. Invalidated ranges and masked/IME text render normally.
  /// Use validated ranges for the exact current value; no display label replaces stored bytes.
  pub fn tokens(mut self, tokens: super::InputTokens) -> Self {
    self.update_node(|node| {
      if let crate::node::node_kind::NodeKind::TextInput { state, .. } = node.node_kind() {
        state.set_tokens(tokens);
      }
    });
    self
  }

  /// Makes this a styled input: `spans` holds the styled runs of the value
  /// as UTF-8 byte ranges ([`super::StyledSpan`]). The input draws them with
  /// the theme's text marks, moves them with every edit, keeps them in its
  /// undo history and writes them back to the signal; the app sets both
  /// signals to load a model. Typed text takes the style of the text before
  /// the caret, except that a link does not grow past its end. A styled
  /// input ignores [`Self::tokens`]; masked, it is plain.
  #[cfg(feature = "markdown")]
  pub fn spans(mut self, spans: Signal<Vec<super::StyledSpan>>) -> Self {
    self.update_node(|node| {
      if let crate::node::node_kind::NodeKind::TextInput { state, .. } = node.node_kind() {
        state.set_spans_signal(spans);
      }
    });
    self
  }

  /// Gives a styled input kinds of paragraphs: `kinds` holds what each hard
  /// paragraph of the value is ([`super::ParagraphKind`]: body text, a
  /// heading, an item of a list, a quote, a line of code), one for each, and
  /// `styles` says how each kind is drawn: its type, the room before its
  /// text and what stands there, and the room between paragraphs. The input
  /// keeps the kinds in step with every edit (a paragraph that is split
  /// gives its kind to both halves; paragraphs that are joined take the
  /// first's), in its undo history, and writes them back to the signal; the
  /// app sets the signal with the value and the spans to load a document.
  /// It needs [`Self::spans`] and [`Self::multiline`].
  #[cfg(feature = "markdown")]
  pub fn paragraphs(mut self, kinds: Signal<Vec<super::ParagraphKind>>, styles: super::ParagraphStyles) -> Self {
    self.update_node(|node| {
      if let crate::node::node_kind::NodeKind::TextInput { state, .. } = node.node_kind() {
        state.set_paragraphs(kinds, styles);
      }
    });
    self
  }

  /// Gives a styled input things in its text that are not text: `objects` holds what each
  /// [`OBJECT_CHARACTER`](super::OBJECT_CHARACTER) of the value is ([`super::InlineObject`]: a picture's source,
  /// its label and its width), and `element` builds what is drawn for one. The element is a child of the input,
  /// laid out in the room the text has and placed where the character stands: its size is the room it takes in
  /// its row, which is as high as it needs, and one wider than the row is made as wide as the row. It moves and
  /// is cut with the text as the input scrolls. To the caret an object is one character: the arrows go past it,
  /// a press on it selects it, Backspace after it selects it and the next deletes it. The pointer and the focus
  /// do not reach what the element holds.
  ///
  /// The input keeps the objects in step with every edit (one before an object moves it, one over it takes it
  /// away), in its undo history, and writes them back to the signal; the app sets the signal with the value to
  /// load a document. `element` is called as the input is built, for each object the signal holds then: a
  /// component that reads the signal builds its input again when it changes. It needs [`Self::spans`].
  ///
  /// ```
  /// use lurq::{
  ///   components::{Column, InlineObject, StyledSpan, TextInput},
  ///   core::Signal,
  /// };
  ///
  /// let value = Signal::new("A square: \u{FFFC}".to_owned());
  /// let objects = Signal::new(vec![InlineObject::new(10, "square").width(24)]);
  /// let input = TextInput::new(value)
  ///   .spans(Signal::new(Vec::<StyledSpan>::new()))
  ///   .objects(objects, |object| {
  ///     let side = object.width.unwrap_or(16) as f32;
  ///     Column::new().width(side).height(side).into()
  ///   })
  ///   .multiline();
  /// ```
  #[cfg(feature = "markdown")]
  pub fn objects(
    mut self,
    objects: Signal<Vec<super::InlineObject>>,
    mut element: impl FnMut(&super::InlineObject) -> crate::node::Element,
  ) -> Self {
    let held = objects.get();
    self.update_node(|node| {
      let crate::node::node_kind::NodeKind::TextInput { state, .. } = node.node_kind() else {
        return;
      };
      let (state, built) = (
        state.clone(),
        super::styled_text::normalize_objects(&state.value(), held),
      );
      node.clear_child_elements();
      for (index, object) in built.iter().enumerate() {
        // Two objects that show the same thing are told apart by which of them comes first.
        let nth = built[..index].iter().filter(|before| before.same_thing(object)).count();
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        // Its width is not part of what it is: the element of an object made wider is the same one.
        std::hash::Hash::hash(&(&object.source, &object.label, nth), &mut hasher);
        let key = format!("object-{:016x}", std::hash::Hasher::finish(&hasher));
        // What the element holds takes neither the pointer nor the focus: a press on it is a press on the text.
        let child = super::Stack::new().child(element(object)).inert(true).key(key);
        node.push_child_element(child.into());
      }
      state.set_objects(objects, built);
    });
    self
  }

  /// Whether an object selected alone ([`Self::objects`]) has a handle on its lower corner at the end, which a
  /// drag makes it wider or narrower by. The drag sets the object's `width`, from 16 to the width of the text,
  /// as it goes, so the element built for it follows; it is one undo step, reported once, when the handle is
  /// let go. Off unless told: an object that is not a picture has no width to give.
  #[cfg(feature = "markdown")]
  pub fn resizable_objects(mut self, resizable: bool) -> Self {
    self.update_node(|node| {
      if let crate::node::node_kind::NodeKind::TextInput { state, .. } = node.node_kind() {
        state.set_resizable_objects(resizable);
      }
    });
    self
  }

  /// Runs after every change a styled input makes to its text or spans
  /// (typing, deleting, pasting, styling, undo) with a [`super::StyledChange`]:
  /// the text and spans it now holds, shared rather than copied, and what the
  /// edit was. An app's own writes of the signals do not run it.
  #[cfg(feature = "markdown")]
  pub fn on_styled_change(mut self, f: impl Fn(&super::StyledChange) + Send + Sync + 'static) -> Self {
    self.update_node(|node| {
      if let crate::node::node_kind::NodeKind::TextInput { state, .. } = node.node_kind() {
        state.add_styled_change_handler(f);
      }
    });
    self
  }

  /// What copying or cutting from a styled input puts on the clipboard: `f`
  /// gets the selection with its spans (offsets from its start) and returns
  /// the text to copy, such as Markdown. Without it the plain text is copied.
  #[cfg(feature = "markdown")]
  pub fn copy_format(mut self, f: impl Fn(&super::StyledText) -> String + Send + Sync + 'static) -> Self {
    self.update_node(|node| {
      if let crate::node::node_kind::NodeKind::TextInput { state, .. } = node.node_kind() {
        state.set_copy_format(f);
      }
    });
    self
  }

  /// Keeps the formatting of a styled input with kinds of paragraphs ([`Self::paragraphs`]) through the
  /// clipboard. A copy or a cut puts the selected text on the clipboard as before and keeps the Markdown of it
  /// beside it, its marks and the kinds of the paragraphs it touches. A paste whose content has Markdown beside
  /// its text ([`ClipboardContent::markdown_content`](crate::app::events::ClipboardContent::markdown_content)),
  /// or whose text reads as Markdown
  /// ([`RichText::reads_as_markdown`](super::RichText::reads_as_markdown)), is inserted with its marks and kinds
  /// as one undo step; any other is plain text, and so is every paste into a line of code. An `on_paste` handler
  /// that prevents the default still takes the paste itself. An input without kinds of paragraphs is not
  /// changed by it.
  ///
  /// The operating system's clipboard holds one text: a copy of a selection with a mark or a paragraph that is
  /// not body text gives it the Markdown, which another app or another lurq process reads as it is written (and
  /// such an input there pastes with its formatting). This process keeps the plain text beside it: while the
  /// clipboard still holds what was written, an input of the app without this option pastes the plain text.
  #[cfg(feature = "markdown")]
  pub fn rich_clipboard(mut self, on: bool) -> Self {
    self.update_node(|node| {
      if let crate::node::node_kind::NodeKind::TextInput { state, .. } = node.node_kind() {
        state.set_rich_clipboard(on);
      }
    });
    self
  }

  /// Runs when a person clicks a link of a styled input with the platform's
  /// shortcut modifier held (Ctrl, or Cmd on macOS); the app opens the
  /// target. A plain click places the caret, as in any text.
  #[cfg(feature = "markdown")]
  pub fn on_link_click(mut self, f: impl Fn(&super::LinkClick) + Send + Sync + 'static) -> Self {
    self.update_node(|node| {
      if let crate::node::node_kind::NodeKind::TextInput { state, .. } = node.node_kind() {
        state.add_link_click_handler(f);
      }
    });
    self
  }

  /// Retained selection/edit control for this input; use one controller per keyed value.
  pub fn controller(mut self, controller: super::TextInputController) -> Self {
    self.update_node(|node| {
      if let crate::node::node_kind::NodeKind::TextInput { state, .. } = node.node_kind() {
        controller.bind(state);
      }
    });
    self.ref_element(controller.reference())
  }

  pub fn new(value: Signal<String>) -> Self {
    Self::from_node(crate::node::Node::text_input(value))
  }

  pub fn styled(value: Signal<String>, style: TextStyle) -> Self {
    Self::from_node(crate::node::Node::text_input_styled(value, style))
  }

  pub fn text_style(mut self, style: TextStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_input_style(node, style));
    self
  }

  pub fn placeholder_style(mut self, style: TextStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_input_placeholder_style(node, style));
    self
  }

  pub fn text_align(mut self, align: impl Into<TextAlign>) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_input_align(node, align));
    self
  }

  /// Extra space after every glyph of the value and placeholder, in logical
  /// pixels (negative tightens). Like `text_align`, a later `text_style` or
  /// `placeholder_style` replaces it.
  pub fn letter_spacing(mut self, letter_spacing: f32) -> Self {
    self.update_node(|node| node.set_text_input_letter_spacing(letter_spacing));
    self
  }

  /// OpenType feature settings of the value and placeholder. Like
  /// `letter_spacing`, a later `text_style` or `placeholder_style` replaces them.
  pub fn font_features(mut self, font_features: impl Into<FontFeatures>) -> Self {
    let font_features = font_features.into();
    self.update_node(|node| node.set_text_input_font_features(font_features));
    self
  }

  pub fn placeholder(mut self, placeholder: &str) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::placeholder(node, placeholder));
    self
  }

  pub fn on_input(mut self, f: impl crate::node::IntoTextInputEventHandler) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::on_input(node, f));
    self
  }

  pub fn off_input(mut self, f: impl crate::node::IntoTextInputEventHandler) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::off_input(node, f));
    self
  }

  pub fn caret_color(mut self, color: impl Into<crate::node::TextColor>) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::caret_color(node, color));
    self
  }

  pub fn selection_color(mut self, color: impl Into<crate::node::TextColor>) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::selection_color(node, color));
    self
  }

  pub fn caret_mode(mut self, mode: CaretMode) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_input_caret_mode(node, mode));
    self
  }

  pub fn overflow(mut self, overflow: TextInputOverflow) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_input_overflow(node, overflow));
    self
  }

  /// Controls which edge of overflowing single-line text is visible while
  /// the input is not focused. Focused inputs always scroll to follow the
  /// caret.
  pub fn unfocused_overflow_anchor(mut self, anchor: TextInputOverflowAnchor) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_input_unfocused_overflow_anchor(node, anchor));
    self
  }

  /// Masks each value character with `•` (U+2022).
  pub fn mask(mut self) -> Self {
    self.update_node(crate::node::NodeUpdate::text_input_mask);
    self
  }

  /// Uses a custom mask, such as `'\u{25cf}'` for a WinUI-style heavy dot.
  pub fn mask_char(mut self, mask: char) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_input_mask_char(node, mask));
    self
  }

  /// Whether the input keeps what it shows while it has the focus, when a render binds it to another signal: the
  /// text being typed is written into the new signal in place of what that signal held. For an input whose text is
  /// made anew from a value in each render, as a number's is: a render while it is typed in, which the value's
  /// change itself may cause, does not take back what is half typed (`1.`, `-`). Out of the focus it shows what its
  /// signal holds. The input needs a key or an id, so that a render finds it again with another signal.
  pub fn keep_text_while_focused(mut self, keeps: bool) -> Self {
    self.update_node(|node| {
      if let crate::node::node_kind::NodeKind::TextInput { state, .. } = node.node_kind() {
        state.set_keeps_text_while_focused(keeps);
      }
    });
    self
  }

  /// Whether all the input's text is selected as it takes the focus, so that what is typed takes its place: a
  /// name being renamed, a cell being edited, a text edited where it stands. A press that brings the focus puts
  /// the caret where it is, as in any input.
  pub fn select_on_focus(mut self, selects: bool) -> Self {
    self.update_node(|node| {
      if let crate::node::node_kind::NodeKind::TextInput { state, .. } = node.node_kind() {
        state.set_select_on_focus(selects);
      }
    });
    self
  }

  /// The on-screen keyboard the input asks for: digits for a number, an `@` key for an email address. It
  /// changes nothing where there is no such keyboard.
  pub fn keyboard(mut self, keyboard: crate::node::node_kind::KeyboardKind) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_input_keyboard(node, keyboard));
    self
  }

  /// What the on-screen keyboard's Return key says. The key sends Enter whatever it says.
  pub fn return_key(mut self, return_key: crate::node::node_kind::ReturnKey) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_input_return_key(node, return_key));
    self
  }

  /// What the input holds, for a platform that fills inputs in: a phone offers the person's email addresses,
  /// a saved password, a code that just came. See [`ContentKind`] for what an input says without it.
  pub fn content(mut self, content: ContentKind) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_input_content(node, content));
    self
  }

  pub fn unmask(mut self) -> Self {
    self.update_node(crate::node::NodeUpdate::text_input_unmask);
    self
  }

  pub fn single_line(mut self) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_input_overflow(node, TextInputOverflow::Scroll));
    self
  }

  /// Whether a multiline input's lines wrap at its width: they do unless told otherwise. One whose lines do not
  /// wrap is as wide as its longest line, for what holds it to scroll sideways: a block of code.
  pub fn wrap(mut self, wrap: bool) -> Self {
    self.update_node(|node| {
      if let crate::node::node_kind::NodeKind::TextInput { state, .. } = node.node_kind() {
        state.set_line_wrap(wrap);
      }
    });
    self
  }

  /// Draws the numbers of a multiline input's lines, and bands behind the lines picked out, where it lays them
  /// out: see [`LineGutter`].
  pub fn line_gutter(mut self, gutter: LineGutter) -> Self {
    self.update_node(|node| {
      if let crate::node::node_kind::NodeKind::TextInput { state, .. } = node.node_kind() {
        state.set_gutter(Some(std::sync::Arc::new(gutter)));
      }
    });
    self
  }

  pub fn multiline(mut self) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_input_overflow(node, TextInputOverflow::Multiline));
    self
  }

  pub fn textarea(mut self) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_input_rows(node, 2, 6));
    self
  }

  pub fn rows(mut self, min_rows: usize, max_rows: usize) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_input_rows(node, min_rows, max_rows));
    self
  }

  pub fn min_rows(mut self, min_rows: usize) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_input_min_rows(node, min_rows));
    self
  }

  pub fn max_rows(mut self, max_rows: usize) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_input_max_rows(node, max_rows));
    self
  }

  pub fn rows_exact(mut self, rows: usize) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_input_rows_exact(node, rows));
    self
  }
}

impl Default for TextInput {
  fn default() -> Self {
    Self::new(Signal::new(String::new()))
  }
}
