//! Things in a styled text that are not text: a picture among its words. Each is one character of the text,
//! [`OBJECT_CHARACTER`], and a record beside it ([`InlineObject`]) that says what it is. The records live in a
//! signal beside the spans ([`crate::components::TextInput::objects`]); the input keeps them in step with its edits
//! and in its undo history, and draws each as the element the app gives for it.
use std::{ops::Range, sync::Arc};

use crate::app::component::{DevtoolsFormatter, DevtoolsInspectable};

/// The character that stands for an object in a text: U+FFFC, the object replacement character. One with no
/// [`InlineObject`] at its byte is a character like any other.
pub const OBJECT_CHARACTER: char = '\u{FFFC}';

/// An object in a styled text: the [`OBJECT_CHARACTER`] at the byte `at`, and what it is. The caret goes past it
/// as past one character; it is selected, deleted, copied and pasted with the words around it, and undo brings
/// it back as it was.
///
/// A document's Markdown writes it as an image, `![label](source)`, and with a width as
/// `<img src="source" alt="label" width="320">`.
///
/// ```
/// use lurq::components::{InlineObject, RichText};
///
/// let notes = RichText::from_markdown("The lake: ![at dusk](photos/lake.jpg)");
/// assert_eq!(notes.text, "The lake: \u{FFFC}");
/// assert_eq!(notes.objects, [InlineObject::new(10, "photos/lake.jpg").label("at dusk")]);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct InlineObject {
  /// The byte of the text its character starts at.
  pub at: usize,
  /// What it shows, as the app reads it: where a picture's pixels are, a path or an address.
  pub source: Arc<str>,
  /// What it is in words: a picture's alternative text, which stands for it in plain text.
  pub label: Arc<str>,
  /// Its width as the document holds it, in logical pixels; with none it is as large as it is by itself, and
  /// never wider than its line.
  pub width: Option<u32>,
}

impl InlineObject {
  /// An object at the byte `at` that shows `source`, with no label and no width of its own.
  pub fn new(at: usize, source: impl Into<Arc<str>>) -> Self {
    Self {
      at,
      source: source.into(),
      label: Arc::from(""),
      width: None,
    }
  }

  /// What it is in words.
  pub fn label(mut self, label: impl Into<Arc<str>>) -> Self {
    self.label = label.into();
    self
  }

  /// Its width in the document, in logical pixels.
  pub fn width(mut self, width: u32) -> Self {
    self.width = Some(width);
    self
  }

  /// The bytes of its character.
  pub fn range(&self) -> Range<usize> {
    self.at..self.at + OBJECT_CHARACTER.len_utf8()
  }

  /// Whether it shows the same thing as `other`, wherever each is and however wide: what tells the element
  /// built for one from the element built for another, and stays the same while one is made wider.
  pub(crate) fn same_thing(&self, other: &Self) -> bool {
    self.source == other.source && self.label == other.label
  }
}

impl DevtoolsInspectable for InlineObject {
  fn inspect(&self, formatter: &mut DevtoolsFormatter<'_>) {
    formatter.value_debug(self);
  }
}

/// `objects` as an input keeps them over `text`: in the order of the text, each at an [`OBJECT_CHARACTER`], one
/// for a character; of two at one byte the later is kept.
pub(crate) fn normalize_objects(text: &str, mut objects: Vec<InlineObject>) -> Vec<InlineObject> {
  let in_order = objects.windows(2).all(|pair| pair[0].at < pair[1].at);
  if !in_order {
    // A stable sort keeps two at one byte in the order they were given.
    objects.sort_by_key(|object| object.at);
  }
  let mut out: Vec<InlineObject> = Vec::with_capacity(objects.len());
  for object in objects {
    if !text
      .get(object.at..)
      .is_some_and(|rest| rest.starts_with(OBJECT_CHARACTER))
    {
      continue;
    }
    match out.last_mut() {
      Some(last) if last.at == object.at => *last = object,
      _ => out.push(object),
    }
  }
  out
}

/// The objects after an edit that replaced the bytes `range` with `inserted` bytes: those in what was replaced
/// are gone, those after it moved by the difference.
pub(crate) fn splice_objects(objects: &[InlineObject], range: Range<usize>, inserted: usize) -> Vec<InlineObject> {
  objects
    .iter()
    .filter(|object| object.at < range.start || object.at >= range.end)
    .map(|object| InlineObject {
      at: if object.at >= range.end {
        object.at - (range.end - range.start) + inserted
      } else {
        object.at
      },
      ..object.clone()
    })
    .collect()
}

/// The objects inside `range`, with their bytes counted from its start.
pub(crate) fn slice_objects(objects: &[InlineObject], range: Range<usize>) -> Vec<InlineObject> {
  objects
    .iter()
    .filter(|object| range.start <= object.at && object.at < range.end)
    .map(|object| InlineObject {
      at: object.at - range.start,
      ..object.clone()
    })
    .collect()
}

/// `objects` with `added`, whose bytes are counted from `start`, put among them.
pub(crate) fn place_objects(mut objects: Vec<InlineObject>, start: usize, added: &[InlineObject]) -> Vec<InlineObject> {
  objects.extend(added.iter().map(|object| InlineObject {
    at: object.at + start,
    ..object.clone()
  }));
  objects.sort_by_key(|object| object.at);
  objects
}

/// `text` with the label of each of `objects` in place of its character: what a plain text shows of them.
pub(crate) fn text_with_labels(text: &str, objects: &[InlineObject]) -> String {
  let mut out = String::with_capacity(text.len());
  let mut at = 0;
  for object in objects {
    let range = object.range();
    if range.start < at || !text.is_char_boundary(range.start) || range.end > text.len() {
      continue;
    }
    out.push_str(&text[at..range.start]);
    out.push_str(&object.label);
    at = range.end;
  }
  out.push_str(&text[at..]);
  out
}

#[cfg(test)]
#[path = "../../../tests/input/text_input/object_ops.rs"]
mod tests;
