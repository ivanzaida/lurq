//! Keeping text that one party wrote from reordering text another party
//! wrote around it.
//!
//! Unicode's explicit directional embeddings, overrides and isolates
//! (U+202A–U+202E, U+2066–U+2069) reorder everything after them up to the
//! end of the paragraph or their match. An unclosed U+202E RIGHT-TO-LEFT
//! OVERRIDE in a link's words, or before a link, would draw the host after it
//! backwards (`github.com` as `moc.buhtig`). The directional marks (U+200E
//! LEFT-TO-RIGHT MARK, U+200F RIGHT-TO-LEFT MARK, U+061C ARABIC LETTER MARK)
//! are invisible letters with a direction: one at the start turns a
//! paragraph right to left, and marks between two pieces decide which way
//! they run. So text from different sources is stripped of all of these, and
//! each piece is wrapped in its own isolate (U+2068 FIRST STRONG ISOLATE …
//! U+2069 POP DIRECTIONAL ISOLATE): nothing inside it reorders what is
//! outside, and nothing outside it reorders what is inside.

use std::borrow::Cow;

/// U+2066 LEFT-TO-RIGHT ISOLATE: starts an isolate that runs left to right
/// whatever the direction around it.
pub(crate) const LEFT_TO_RIGHT_ISOLATE: char = '\u{2066}';
/// U+2068 FIRST STRONG ISOLATE: starts an isolate whose direction is that of
/// its first strong character.
pub(crate) const FIRST_STRONG_ISOLATE: char = '\u{2068}';
/// U+2069 POP DIRECTIONAL ISOLATE: ends the isolate.
pub(crate) const POP_DIRECTIONAL_ISOLATE: char = '\u{2069}';

/// Whether `ch` is an explicit directional embedding, override or isolate
/// control (U+202A–U+202E, U+2066–U+2069) or a directional mark (U+200E,
/// U+200F, U+061C).
pub fn is_bidi_control(ch: char) -> bool {
  matches!(
    ch,
    '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' | '\u{200E}' | '\u{200F}' | '\u{061C}'
  )
}

/// `text` without directional controls and marks ([`is_bidi_control`]);
/// borrowed when it has none.
pub fn strip_bidi_controls(text: &str) -> Cow<'_, str> {
  if text.chars().any(is_bidi_control) {
    Cow::Owned(text.chars().filter(|ch| !is_bidi_control(*ch)).collect())
  } else {
    Cow::Borrowed(text)
  }
}

/// `text` stripped of directional controls and marks and wrapped in its own
/// isolate, to put text from another source (a link's destination in a
/// tooltip, say) into a sentence without either reordering the other. The
/// isolate marks are invisible.
pub fn isolate_bidi(text: &str) -> String {
  let stripped = strip_bidi_controls(text);
  let mut isolated = String::with_capacity(stripped.len() + 6);
  isolated.push(FIRST_STRONG_ISOLATE);
  isolated.push_str(&stripped);
  isolated.push(POP_DIRECTIONAL_ISOLATE);
  isolated
}
