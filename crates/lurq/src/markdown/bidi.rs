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
//!
//! Bidi reorders each laid-out line on its own, and line and paragraph
//! separators start a new bidi paragraph: a link's words are kept to one bidi
//! paragraph ([`paragraph_breaks_as_spaces`]) and laid out so that a line
//! break anywhere in them leaves its trailing runs right after its words (see
//! `components::markdown::link_block`).

use std::borrow::Cow;

use unicode_bidi::{BidiClass, bidi_class};

/// U+200E LEFT-TO-RIGHT MARK: an invisible left-to-right letter.
pub(crate) const LEFT_TO_RIGHT_MARK: char = '\u{200E}';
/// U+200F RIGHT-TO-LEFT MARK: an invisible right-to-left letter.
pub(crate) const RIGHT_TO_LEFT_MARK: char = '\u{200F}';
/// U+2066 LEFT-TO-RIGHT ISOLATE: starts an isolate that runs left to right
/// whatever the direction around it.
pub(crate) const LEFT_TO_RIGHT_ISOLATE: char = '\u{2066}';
/// U+2067 RIGHT-TO-LEFT ISOLATE: starts an isolate that runs right to left
/// whatever the direction around it.
pub(crate) const RIGHT_TO_LEFT_ISOLATE: char = '\u{2067}';
/// U+2068 FIRST STRONG ISOLATE: starts an isolate whose direction is that of
/// its first strong character.
pub(crate) const FIRST_STRONG_ISOLATE: char = '\u{2068}';
/// U+2069 POP DIRECTIONAL ISOLATE: ends the isolate.
pub(crate) const POP_DIRECTIONAL_ISOLATE: char = '\u{2069}';
/// U+2060 WORD JOINER: an invisible character that no line breaks next to.
pub(crate) const WORD_JOINER: char = '\u{2060}';

/// Which way a bidi paragraph runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Direction {
  LeftToRight,
  RightToLeft,
}

impl Direction {
  /// The direction `ch` gives the paragraph it is the first letter of (bidi
  /// rule P2), if it is a letter with a direction.
  pub(crate) fn of_letter(ch: char) -> Option<Self> {
    match bidi_class(ch) {
      BidiClass::L => Some(Self::LeftToRight),
      BidiClass::R | BidiClass::AL => Some(Self::RightToLeft),
      _ => None,
    }
  }
}

/// Whether bidi would draw some of `text` right to left, or order it by the
/// text around it, in a left-to-right isolate: right-to-left letters and
/// Arabic digits. Text without them draws in its own order there.
pub(crate) fn has_right_to_left(text: &str) -> bool {
  text
    .chars()
    .any(|ch| matches!(bidi_class(ch), BidiClass::R | BidiClass::AL | BidiClass::AN))
}

/// Whether `ch` ends a line or a bidi paragraph: a line feed, a carriage
/// return, U+001C–U+001E, U+0085 NEXT LINE, U+2028 LINE SEPARATOR or U+2029
/// PARAGRAPH SEPARATOR.
pub(crate) fn is_paragraph_break(ch: char) -> bool {
  matches!(
    ch,
    '\n' | '\r' | '\u{1C}'..='\u{1E}' | '\u{85}' | '\u{2028}' | '\u{2029}'
  )
}

/// `text` with its separators other than a line feed or a carriage return
/// as line feeds, the hard break lurq lays out.
pub(crate) fn paragraph_breaks_as_line_feeds(text: &str) -> Cow<'_, str> {
  replace_chars(text, |ch| is_paragraph_break(ch) && !matches!(ch, '\n' | '\r'), '\n')
}

/// `text` with every line and paragraph separator as a space.
pub(crate) fn paragraph_breaks_as_spaces(text: &str) -> Cow<'_, str> {
  replace_chars(text, is_paragraph_break, ' ')
}

fn replace_chars(text: &str, replaced: impl Fn(char) -> bool, with: char) -> Cow<'_, str> {
  if text.chars().any(&replaced) {
    Cow::Owned(text.chars().map(|ch| if replaced(ch) { with } else { ch }).collect())
  } else {
    Cow::Borrowed(text)
  }
}

/// Whether `text` draws nothing: it is empty, or only whitespace, controls,
/// default-ignorable code points (U+200B ZERO WIDTH SPACE, U+2060 WORD
/// JOINER, U+FEFF, U+00AD SOFT HYPHEN, fillers, variation selectors, tags…)
/// or U+2800 BRAILLE PATTERN BLANK.
pub(crate) fn is_invisible(text: &str) -> bool {
  text
    .chars()
    .all(|ch| ch.is_whitespace() || ch.is_control() || is_default_ignorable(ch) || ch == '\u{2800}')
}

/// Unicode's `Default_Ignorable_Code_Point`: characters a font draws as
/// nothing.
fn is_default_ignorable(ch: char) -> bool {
  matches!(
    ch,
    '\u{AD}'
      | '\u{34F}'
      | '\u{61C}'
      | '\u{115F}'..='\u{1160}'
      | '\u{17B4}'..='\u{17B5}'
      | '\u{180B}'..='\u{180F}'
      | '\u{200B}'..='\u{200F}'
      | '\u{202A}'..='\u{202E}'
      | '\u{2060}'..='\u{206F}'
      | '\u{3164}'
      | '\u{FE00}'..='\u{FE0F}'
      | '\u{FEFF}'
      | '\u{FFA0}'
      | '\u{FFF0}'..='\u{FFF8}'
      | '\u{1BCA0}'..='\u{1BCA3}'
      | '\u{1D173}'..='\u{1D17A}'
      | '\u{E0000}'..='\u{E0FFF}'
  )
}

/// The byte offsets inside `text` where a line may break (Unicode line
/// breaking, as cosmic-text lays out), its end left out.
pub(crate) fn line_break_offsets(text: &str) -> impl Iterator<Item = usize> + '_ {
  unicode_linebreak::linebreaks(text)
    .map(|(offset, _)| offset)
    .filter(move |offset| *offset > 0 && *offset < text.len())
}

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
