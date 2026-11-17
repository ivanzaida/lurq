//! A tokenizer with state: what a line starts in, an edit of a text a tokenizer is told, and what lurq keeps of a
//! text's lines so that an edit re-tokenizes from the line it changed until the lines agree again.

use std::{any::Any, fmt, sync::Arc};

use super::{CodeToken, CodeTokenizer};

/// What a tokenizer needs to go on at a line's start: inside a block comment, a string over lines, a tag. Opaque
/// to lurq, which keeps the one each line starts in and compares two to know where an edit stops mattering.
///
/// It holds any value that is `Clone + PartialEq + Send + Sync`: a small enum for lurq's own scanner, syntect's
/// `ParseState` and `ScopeStack` for an adapter of it. A number would not do for a parser whose state is a stack,
/// and a value lurq cannot compare would let no edit stop early. [`LineState::start`] is the state of a text's first
/// line, which holds nothing.
#[derive(Clone, Default)]
pub struct LineState(Option<Arc<dyn StateValue>>);

trait StateValue: Any + Send + Sync {
  fn same(&self, other: &dyn StateValue) -> bool;
  fn as_any(&self) -> &dyn Any;
}

impl<T: Clone + PartialEq + Send + Sync + 'static> StateValue for T {
  fn same(&self, other: &dyn StateValue) -> bool {
    other.as_any().downcast_ref::<T>() == Some(self)
  }

  fn as_any(&self) -> &dyn Any {
    self
  }
}

impl LineState {
  /// The state of a text's first line: nothing open.
  pub fn start() -> Self {
    Self(None)
  }

  /// A state that holds `value`.
  pub fn new<T: Clone + PartialEq + Send + Sync + 'static>(value: T) -> Self {
    Self(Some(Arc::new(value)))
  }

  /// The value it holds, where it holds a `T`; `None` for [`LineState::start`].
  pub fn get<T: 'static>(&self) -> Option<&T> {
    self.0.as_ref().and_then(|value| value.as_any().downcast_ref::<T>())
  }
}

impl PartialEq for LineState {
  fn eq(&self, other: &Self) -> bool {
    match (&self.0, &other.0) {
      (None, None) => true,
      (Some(one), Some(other)) => one.same(other.as_ref()),
      _ => false,
    }
  }
}

impl fmt::Debug for LineState {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(if self.0.is_some() {
      "LineState(..)"
    } else {
      "LineState::start"
    })
  }
}

/// A text a tokenizer is told the edits of: an editable code field. A tokenizer that keeps a tree for each text (a
/// parser such as tree-sitter) keeps it by this.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BufferId(pub u64);

/// A place in a text: its line, from 0, and its byte in the line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct CodePoint {
  pub row: usize,
  pub column: usize,
}

/// An edit of a text, in the shape tree-sitter's `InputEdit` has: the bytes `start_byte..old_end_byte` of the text
/// were replaced by what is `start_byte..new_end_byte` of it now, and the same as places.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct CodeEdit {
  pub start_byte: usize,
  pub old_end_byte: usize,
  pub new_end_byte: usize,
  pub start_point: CodePoint,
  pub old_end_point: CodePoint,
  pub new_end_point: CodePoint,
}

impl CodeEdit {
  /// The edit that makes `old` into `new`: the part between what they start and end with alike, `None` where they
  /// are the same. An edit that adds or takes away a run of what stands beside it (a second `a` after an `a`) is
  /// told at the run's end.
  pub fn between(old: &str, new: &str) -> Option<Self> {
    if old == new {
      return None;
    }
    let mut start = old
      .bytes()
      .zip(new.bytes())
      .take_while(|(one, other)| one == other)
      .count();
    while !old.is_char_boundary(start) || !new.is_char_boundary(start) {
      start -= 1;
    }
    let most = old.len().min(new.len()) - start;
    let mut same_end = old
      .bytes()
      .rev()
      .zip(new.bytes().rev())
      .take(most)
      .take_while(|(one, other)| one == other)
      .count();
    while !old.is_char_boundary(old.len() - same_end) || !new.is_char_boundary(new.len() - same_end) {
      same_end -= 1;
    }
    let (old_end, new_end) = (old.len() - same_end, new.len() - same_end);
    Some(Self {
      start_byte: start,
      old_end_byte: old_end,
      new_end_byte: new_end,
      start_point: point(new, start),
      old_end_point: point(old, old_end),
      new_end_point: point(new, new_end),
    })
  }
}

/// The place of the byte `at` of `text`.
fn point(text: &str, at: usize) -> CodePoint {
  let before = &text[..at];
  let row = before.bytes().filter(|byte| *byte == b'\n').count();
  let column = before.rfind('\n').map_or(at, |newline| at - newline - 1);
  CodePoint { row, column }
}

/// How many lines a line-by-line tokenizer is given at a time.
const BATCH: usize = 32;

/// A line as kept: the state it starts in, and its tokens, in bytes of the line.
#[derive(Clone)]
struct KeptLine {
  start: LineState,
  tokens: Arc<[CodeToken]>,
}

/// What lurq keeps of one text for one tokenizer: the text, the state each of its lines starts in and its tokens.
/// The next text is re-tokenized from its first changed line, until a line starts in the state the kept line it
/// stands for started in; the lines after are taken as they were.
#[derive(Default)]
pub(crate) struct LineCache {
  /// The tokenizer and the generation it was at.
  tokenizer: Option<(u64, u64)>,
  code: Arc<str>,
  /// The lines, where the tokenizer goes line by line; empty for one that does not.
  lines: Vec<KeptLine>,
  all: Option<Arc<[CodeToken]>>,
  /// How many lines were tokenized in all: what a test counts.
  #[cfg(test)]
  pub(crate) tokenized: usize,
}

impl LineCache {
  /// The tokens of `code` by `tokenizer`, in bytes of the code, from what was kept of the text before.
  pub(crate) fn tokens(&mut self, tokenizer: &CodeTokenizer, code: &str) -> Arc<[CodeToken]> {
    self.tokens_with(tokenizer, code, |tokenizer, code| tokenizer.tokens(code).to_vec())
  }

  /// As [`tokens`](Self::tokens), the code of a tokenizer that does not go line by line given by `whole`.
  pub(crate) fn tokens_with(
    &mut self,
    tokenizer: &CodeTokenizer,
    code: &str,
    whole: impl FnOnce(&CodeTokenizer, &str) -> Vec<CodeToken>,
  ) -> Arc<[CodeToken]> {
    let mut whole = Some(whole);
    let key = (tokenizer.id(), tokenizer.generation());
    if self.tokenizer != Some(key) {
      *self = Self {
        tokenizer: Some(key),
        #[cfg(test)]
        tokenized: self.tokenized,
        ..Self::default()
      };
    } else if let Some(all) = &self.all
      && &*self.code == code
    {
      return all.clone();
    }
    let new: Vec<&str> = code.split('\n').collect();
    // From the first line the edit changed; the lines at the end it left as they were stand for the old ones.
    let (first, tail) = if self.lines.is_empty() || self.all.is_none() {
      (0, 0)
    } else {
      let start = self
        .code
        .bytes()
        .zip(code.bytes())
        .take_while(|(one, other)| one == other)
        .count();
      let most = self.code.len().min(code.len()) - start;
      let same_end = self
        .code
        .bytes()
        .rev()
        .zip(code.bytes().rev())
        .take(most)
        .take_while(|(one, other)| one == other)
        .count();
      let first = code[..start].bytes().filter(|byte| *byte == b'\n').count();
      let tail = code[code.len() - same_end..]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count();
      (first, tail.min(new.len() - first - 1).min(self.lines.len() - first - 1))
    };
    let mut lines: Vec<KeptLine> = self.lines[..first.min(self.lines.len())].to_vec();
    let mut state = self
      .lines
      .get(first)
      .map_or_else(LineState::start, |line| line.start.clone());
    let mut at = first;
    let mut resumed = None;
    'lines: while at < new.len() {
      let batch = &new[at..(at + BATCH).min(new.len())];
      let Some(found) = tokenizer.tokens_from(batch, &state) else {
        return self.whole(code, whole.take().map_or_else(Vec::new, |whole| whole(tokenizer, code)));
      };
      if found.len() != batch.len() {
        return self.whole(code, whole.take().map_or_else(Vec::new, |whole| whole(tokenizer, code)));
      }
      #[cfg(test)]
      {
        self.tokenized += batch.len();
      }
      for (line, (tokens, end)) in batch.iter().zip(found) {
        lines.push(KeptLine {
          start: state,
          tokens: super::normalized(line, tokens).into(),
        });
        state = end;
        at += 1;
        // Past the edit, a line that starts as the kept one it stands for did: the rest is as it was.
        if tail > 0 && at >= new.len() - tail {
          let old = at + self.lines.len() - new.len();
          if old < self.lines.len() && self.lines[old].start == state {
            resumed = Some(old);
            break 'lines;
          }
        }
      }
    }
    if let Some(old) = resumed {
      lines.extend_from_slice(&self.lines[old..]);
    }
    // The tokens of every line, in bytes of the code.
    let mut all = Vec::new();
    let mut offset = 0;
    for (line, kept) in new.iter().zip(&lines) {
      all.extend(kept.tokens.iter().map(|token| CodeToken {
        range: token.range.start + offset..token.range.end + offset,
        kind: token.kind,
      }));
      offset += line.len() + 1;
    }
    let all: Arc<[CodeToken]> = all.into();
    self.lines = lines;
    self.code = Arc::from(code);
    self.all = Some(all.clone());
    all
  }

  /// The tokens of a tokenizer that does not go line by line: all of them, kept by the code.
  fn whole(&mut self, code: &str, all: Vec<CodeToken>) -> Arc<[CodeToken]> {
    let all: Arc<[CodeToken]> = super::normalized(code, all).into();
    self.lines.clear();
    self.code = Arc::from(code);
    self.all = Some(all.clone());
    all
  }
}
