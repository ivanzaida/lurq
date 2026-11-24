//! Styled-text commands of a controller: what a rich editor's toolbar and
//! shortcuts call. Each returns `Ok(None)` or `Ok(false)` for a plain input.
use super::{InputEditError, TextInputController};
use crate::{
  app::events::KeyboardEvent,
  components::{InlineObject, ParagraphKind, RichText, SelectionStyle, SpanStyle, StyledText, TextMark},
  core::NodeId,
};

impl TextInputController {
  /// The text and all its spans.
  pub fn styled_text(&self) -> Result<Option<StyledText>, InputEditError> {
    Ok(self.state()?.styled_text())
  }

  /// The selection with its spans, offsets from its start; empty with
  /// nothing selected.
  pub fn selected_styled_text(&self) -> Result<Option<StyledText>, InputEditError> {
    Ok(self.state()?.selected_styled_text())
  }

  /// Which marks cover the selection, or the style the next typed text
  /// takes when nothing is selected: what a toolbar shows.
  pub fn selection_style(&self) -> Result<Option<SelectionStyle>, InputEditError> {
    Ok(self.state()?.selection_style())
  }

  /// Turns `mark` on over the selection unless all of it has it already,
  /// then off, as one undo step. With nothing selected it toggles the mark
  /// of the next typed text at the caret (Ctrl+B while typing).
  pub fn toggle_mark(&self, mark: TextMark) -> Result<bool, InputEditError> {
    Ok(self.state()?.toggle_mark(mark))
  }

  /// Sets `mark` on or off over the selection, as [`Self::toggle_mark`] does.
  pub fn set_mark(&self, mark: TextMark, on: bool) -> Result<bool, InputEditError> {
    Ok(self.state()?.set_mark(mark, on))
  }

  /// Links the selection to `target`, or unlinks it with `None`, as one undo
  /// step. With nothing selected it changes the whole link the caret is in,
  /// else the link of the next typed text.
  pub fn set_link(&self, target: Option<&str>) -> Result<bool, InputEditError> {
    Ok(self.state()?.set_link(target.map(Into::into)))
  }

  /// The style the next typed text at the caret takes, until the caret
  /// moves or text is typed; `None` returns to the style of the text before
  /// the caret.
  pub fn set_pending_style(&self, style: Option<SpanStyle>) -> Result<bool, InputEditError> {
    Ok(self.state()?.set_pending_style(style))
  }

  /// The text, its spans, the kinds of its paragraphs and its objects, as one value.
  pub fn rich_text(&self) -> Result<Option<RichText>, InputEditError> {
    let state = self.state()?;
    Ok(state.styled_text().map(|styled| {
      let kinds = state.current_paragraphs().map(|kinds| kinds.to_vec());
      let objects = state.current_objects().map(|objects| objects.to_vec());
      RichText::new(styled.text, styled.spans, kinds.unwrap_or_default()).with_objects(objects.unwrap_or_default())
    }))
  }

  /// The objects of the text, in its order. `None` for an input without objects
  /// ([`TextInput::objects`](crate::components::TextInput::objects)).
  pub fn objects(&self) -> Result<Option<Vec<InlineObject>>, InputEditError> {
    Ok(self.state()?.current_objects().map(|objects| objects.to_vec()))
  }

  /// Puts `object` in place of the selection, or at the caret, as one undo step: its character goes into the
  /// text and its record among the input's objects, at the byte the character is put (`object.at` is not read).
  /// The caret is left after it. `false` for an input without objects, and where an `on_input` handler refused
  /// the edit.
  pub fn insert_object(&self, object: &InlineObject) -> Result<bool, InputEditError> {
    let state = self.state()?;
    state.sync_external_value();
    let text = crate::components::OBJECT_CHARACTER.to_string();
    let keyboard = KeyboardEvent::new(&text, "InsertText", false, false, false, false, NodeId::UNASSIGNED);
    Ok(state.insert_object(object, &keyboard))
  }

  /// What each paragraph the selection touches is, in order: the one the
  /// caret is in with nothing selected. `None` for an input without kinds
  /// of paragraphs.
  pub fn selected_paragraphs(&self) -> Result<Option<Vec<ParagraphKind>>, InputEditError> {
    Ok(self.state()?.selected_paragraphs())
  }

  /// The paragraph the caret is in: its kind and the bytes of its text,
  /// without its line break.
  pub fn caret_paragraph(&self) -> Result<Option<(ParagraphKind, std::ops::Range<usize>)>, InputEditError> {
    Ok(self.state()?.caret_paragraph().map(|(_, kind, range)| (kind, range)))
  }

  /// Makes every paragraph the selection touches a `kind` (the one the
  /// caret is in with nothing selected), as one undo step. `false` where
  /// they all are already.
  pub fn set_paragraphs(&self, kind: ParagraphKind) -> Result<bool, InputEditError> {
    Ok(self.state()?.set_paragraph_kind(kind))
  }

  /// Checks the items of a checklist the selection touches (the one the caret is in with nothing selected), or
  /// unchecks them where they all are checked, as one undo step. `false` where it touches none.
  pub fn toggle_checks(&self) -> Result<bool, InputEditError> {
    Ok(self.state()?.toggle_selected_checks())
  }

  /// Puts a divider ([`ParagraphKind::Divider`]) after the paragraph the caret is in, or makes that paragraph
  /// one where it is empty body text, with a paragraph of body text after it that the caret is put in, as one
  /// undo step.
  pub fn insert_divider(&self) -> Result<bool, InputEditError> {
    let state = self.state()?;
    state.sync_external_value();
    let keyboard = KeyboardEvent::new("\n", "InsertText", false, false, false, false, NodeId::UNASSIGNED);
    Ok(state.insert_divider(&keyboard))
  }

  /// Replaces the selection, or inserts at the caret, text that brings its
  /// own spans (offsets from its start), as one undo step: what an app's
  /// paste handler calls with the Markdown it parsed. A plain input takes
  /// the text alone.
  pub fn replace_selection_styled(&self, text: &StyledText) -> Result<bool, InputEditError> {
    let state = self.state()?;
    state.sync_external_value();
    let keyboard = KeyboardEvent::new(&text.text, "InsertText", false, false, false, false, NodeId::UNASSIGNED);
    Ok(state.replace_selection_styled(text, &keyboard))
  }

  /// Replaces the selection, or inserts at the caret, text that brings its own spans, the kinds of its
  /// paragraphs and its objects, as one undo step: what a paste of formatted text does. A single paragraph goes into
  /// the line the caret is in and leaves it what it is, unless the line holds nothing else. Of several, the first
  /// goes on from the text before the caret and takes its own kind only where there is none, those between come with
  /// their kinds, and the last gives its kind to the text after the caret unless it is empty. An input without
  /// kinds of paragraphs takes the text and its spans, a plain one the text.
  pub fn replace_selection_rich(&self, text: &RichText) -> Result<bool, InputEditError> {
    let state = self.state()?;
    state.sync_external_value();
    let keyboard = KeyboardEvent::new(&text.text, "InsertText", false, false, false, false, NodeId::UNASSIGNED);
    Ok(state.replace_selection_rich(text, &keyboard))
  }
}
