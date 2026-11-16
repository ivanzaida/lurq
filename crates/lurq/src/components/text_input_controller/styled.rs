//! Styled-text commands of a controller: what a rich editor's toolbar and
//! shortcuts call. Each returns `Ok(None)` or `Ok(false)` for a plain input.
use super::{InputEditError, TextInputController};
use crate::{
  app::events::KeyboardEvent,
  components::{ParagraphKind, RichText, SelectionStyle, SpanStyle, StyledText, TextMark},
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

  /// The text, its spans and the kinds of its paragraphs, as one value.
  pub fn rich_text(&self) -> Result<Option<RichText>, InputEditError> {
    let state = self.state()?;
    Ok(state.styled_text().map(|styled| {
      let kinds = state.current_paragraphs().map(|kinds| kinds.to_vec());
      RichText::new(styled.text, styled.spans, kinds.unwrap_or_default())
    }))
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
}
