//! Keeping text that one party wrote from reordering text another party
//! wrote around it.
//!
//! Unicode's explicit directional embeddings, overrides and isolates
//! (U+202A–U+202E, U+2066–U+2069) reorder everything after them up to the
//! end of the paragraph or their match. An unclosed U+202E RIGHT-TO-LEFT
//! OVERRIDE in a link's words, or before a link, would draw the host after it
//! backwards (`github.com` as `moc.buhtig`), so text from different sources
//! is stripped of these controls and each piece is wrapped in its own
//! isolate (U+2068 FIRST STRONG ISOLATE … U+2069 POP DIRECTIONAL ISOLATE):
//! nothing inside it reorders what is outside, and nothing outside it
//! reorders what is inside.

use std::borrow::Cow;

/// U+2068 FIRST STRONG ISOLATE: starts an isolate whose direction is that of
/// its first strong character.
pub(crate) const FIRST_STRONG_ISOLATE: char = '\u{2068}';
/// U+2069 POP DIRECTIONAL ISOLATE: ends the isolate.
pub(crate) const POP_DIRECTIONAL_ISOLATE: char = '\u{2069}';

/// Whether `ch` is an explicit directional embedding, override or isolate
/// control (U+202A–U+202E, U+2066–U+2069).
pub fn is_bidi_control(ch: char) -> bool {
  matches!(ch, '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
}

/// `text` without explicit directional embedding, override and isolate
/// controls; borrowed when it has none.
pub fn strip_bidi_controls(text: &str) -> Cow<'_, str> {
  if text.chars().any(is_bidi_control) {
    Cow::Owned(text.chars().filter(|ch| !is_bidi_control(*ch)).collect())
  } else {
    Cow::Borrowed(text)
  }
}

/// `text` stripped of directional controls and wrapped in its own isolate, to
/// put text from another source (a link's destination in a tooltip, say)
/// into a sentence without either reordering the other. The isolate marks
/// are invisible.
pub fn isolate_bidi(text: &str) -> String {
  let stripped = strip_bidi_controls(text);
  let mut isolated = String::with_capacity(stripped.len() + 6);
  isolated.push(FIRST_STRONG_ISOLATE);
  isolated.push_str(&stripped);
  isolated.push(POP_DIRECTIONAL_ISOLATE);
  isolated
}
