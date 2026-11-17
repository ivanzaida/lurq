//! The one scanner every [`CodeLanguage`] is read by: a line at a time, from the state the line starts in.

use std::sync::LazyLock;

use super::{
  CodeToken, TokenName,
  language::{CodeLanguage, CodeMode, KeyRule},
  token,
};

/// Where a line starts: inside a comment, a string or a tag an earlier line opened, or in none. The state after a
/// line is the next one's, so a long text can be tokenized from any line whose state is kept
/// ([`CodeLanguage::states`], [`CodeLanguage::line_tokens`]).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ScanState {
  inside: Inside,
}

impl ScanState {
  /// Whether the line starts in nothing an earlier line opened.
  pub fn is_clear(&self) -> bool {
    self.inside == Inside::Nothing
  }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
enum Inside {
  #[default]
  Nothing,
  /// A block comment of the language's form `form`, `depth` comments deep inside it.
  Comment { form: usize, depth: u32 },
  /// A string of the form `form`, closed with `hashes` `#`s.
  String { form: usize, hashes: u32 },
  /// A tag, between its name and its `>`.
  Tag,
  /// A value of an attribute of a tag, in the quote `quote`.
  TagValue { quote: char },
  /// A fenced block of Markdown, of `count` of `mark`.
  Fence { mark: char, count: usize },
  /// A block of text of YAML, under a key indented `indent`.
  BlockScalar { indent: usize },
}

/// The names of tokens, interned once.
struct Names {
  keyword: TokenName,
  string: TokenName,
  escape: TokenName,
  number: TokenName,
  type_: TokenName,
  function: TokenName,
  macro_: TokenName,
  variable: TokenName,
  constant: TokenName,
  property: TokenName,
  attribute: TokenName,
  operator: TokenName,
  punctuation: TokenName,
  tag: TokenName,
  regex: TokenName,
  namespace: TokenName,
  label: TokenName,
  heading: TokenName,
  bold: TokenName,
  italic: TokenName,
  code: TokenName,
  link: TokenName,
  link_url: TokenName,
  list: TokenName,
  quote: TokenName,
}

static NAMES: LazyLock<Names> = LazyLock::new(|| Names {
  keyword: token::KEYWORD.into(),
  string: token::STRING.into(),
  escape: token::STRING_ESCAPE.into(),
  number: token::NUMBER.into(),
  type_: token::TYPE.into(),
  function: token::FUNCTION.into(),
  macro_: token::FUNCTION_MACRO.into(),
  variable: token::VARIABLE.into(),
  constant: token::CONSTANT.into(),
  property: token::PROPERTY.into(),
  attribute: token::ATTRIBUTE.into(),
  operator: token::OPERATOR.into(),
  punctuation: token::PUNCTUATION.into(),
  tag: token::TAG.into(),
  regex: token::REGEX.into(),
  namespace: token::NAMESPACE.into(),
  label: token::LABEL.into(),
  heading: token::MARKUP_HEADING.into(),
  bold: token::MARKUP_BOLD.into(),
  italic: token::MARKUP_ITALIC.into(),
  code: token::MARKUP_CODE.into(),
  link: token::MARKUP_LINK.into(),
  link_url: "markup.link.url".into(),
  list: token::MARKUP_LIST.into(),
  quote: token::MARKUP_QUOTE.into(),
});

/// What came last on the line, which says whether a `/` starts a regular expression.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Prev {
  /// Nothing yet.
  Start,
  /// A value: a name, a number, a string, a closing bracket.
  Value,
  /// An operator or an opening mark, after which a value starts.
  Open,
  /// A word of the language.
  Keyword,
}

struct Scan<'a> {
  lang: &'a CodeLanguage,
  text: &'a str,
  base: usize,
  out: &'a mut Vec<CodeToken>,
  at: usize,
  prev: Prev,
  /// A shell's: the next word is a command.
  command: bool,
  /// CSS's: inside a declaration's value.
  value: bool,
  /// CSS's: the `:` that comes next starts a value.
  value_next: bool,
  /// The name the next word is declared as, after `fn` or `class`.
  declare: Option<TokenName>,
  /// Where a line comment starts, or the end of the line.
  code_end: usize,
}

impl CodeLanguage {
  /// Pushes the tokens of `line` (without its `\n`), its bytes starting at `base` in the code, from `state`.
  pub(super) fn scan_line(&self, line: &str, base: usize, state: &mut ScanState, out: &mut Vec<CodeToken>) {
    let text = line.strip_suffix('\r').unwrap_or(line);
    let mut scan = Scan {
      lang: self,
      text,
      base,
      out,
      at: 0,
      prev: Prev::Start,
      command: true,
      value: false,
      value_next: false,
      declare: None,
      code_end: text.len(),
    };
    match state.inside.clone() {
      Inside::Nothing | Inside::Tag => {}
      Inside::Comment { form, depth } => {
        scan.comment_from(form, depth, 0, state);
        if !state.is_clear() {
          return;
        }
      }
      Inside::String { form, hashes } => {
        scan.string_from(form, hashes, 0, 0, state);
        if !state.is_clear() {
          return;
        }
      }
      Inside::TagValue { quote } => {
        if !scan.tag_value(quote, 0, state) {
          return;
        }
      }
      Inside::Fence { mark, count } => {
        let trimmed = text.trim_start();
        let start = text.len() - trimmed.len();
        let run = trimmed.chars().take_while(|ch| *ch == mark).count();
        if run >= count && trimmed[run..].trim().is_empty() {
          state.inside = Inside::Nothing;
        }
        scan.push(start, text.len(), NAMES.code);
        return;
      }
      Inside::BlockScalar { indent } => {
        let trimmed = text.trim_start();
        let start = text.len() - trimmed.len();
        if trimmed.is_empty() {
          return;
        }
        if start > indent {
          scan.push(start, text.len(), NAMES.string);
          return;
        }
        state.inside = Inside::Nothing;
      }
    }
    match self.mode {
      CodeMode::Code => scan.code(state),
      CodeMode::Markup => scan.markup(state),
      CodeMode::Markdown => scan.markdown(state),
    }
  }
}

impl<'a> Scan<'a> {
  fn push(&mut self, start: usize, end: usize, kind: TokenName) {
    if start >= end {
      return;
    }
    let range = self.base + start..self.base + end;
    if let Some(last) = self.out.last_mut()
      && last.kind == kind
      && last.range.end == range.start
    {
      last.range.end = range.end;
      return;
    }
    self.out.push(CodeToken { range, kind });
  }

  fn rest(&self) -> &'a str {
    &self.text[self.at..]
  }

  fn char_at(&self, at: usize) -> Option<char> {
    self.text.get(at..).and_then(|rest| rest.chars().next())
  }

  fn char_before(&self, at: usize) -> Option<char> {
    self.text[..at].chars().next_back()
  }

  fn is_name_char(&self, ch: char) -> bool {
    ch.is_alphanumeric() || self.lang.name_chars.contains(ch)
  }

  fn is_name_start(&self, ch: char) -> bool {
    ch.is_alphabetic() || self.lang.name_starts.contains(ch)
  }

  /// The end of the name that starts at `at`.
  fn name_end(&self, at: usize) -> usize {
    let mut end = at;
    for (index, ch) in self.text[at..].char_indices() {
      let fits = if index == 0 {
        self.is_name_start(ch) || ch.is_alphanumeric()
      } else {
        self.is_name_char(ch)
      };
      if !fits {
        break;
      }
      end = at + index + ch.len_utf8();
    }
    end
  }

  fn skip_spaces(&self, from: usize) -> usize {
    from + (self.text[from..].len() - self.text[from..].trim_start().len())
  }

  // The language of code.

  fn code(&mut self, state: &mut ScanState) {
    // A line that goes on from a comment or a string an earlier one opened has no start of its own.
    let fresh = self.at == 0;
    self.at = self.skip_spaces(self.at);
    let indent = self.at;
    if fresh && self.lang.sections && self.rest().starts_with('[') {
      // The rest of the line is read as any other.
      self.section();
    } else if fresh && self.lang.block_scalars && (self.rest().starts_with("---") || self.rest().starts_with("...")) {
      let run = self.rest().chars().take_while(|ch| matches!(ch, '-' | '.')).count();
      if self.rest()[run..].trim().is_empty() {
        self.push(self.at, self.at + run, NAMES.punctuation);
        return;
      }
    }
    while self.at < self.text.len() {
      let Some(ch) = self.char_at(self.at) else {
        break;
      };
      if ch.is_whitespace() {
        self.at += ch.len_utf8();
        continue;
      }
      if self.line_comment() {
        break;
      }
      if self.block_comment(state) {
        if !state.is_clear() {
          return;
        }
        continue;
      }
      if self.attribute() || self.string(state) {
        if !state.is_clear() {
          return;
        }
        continue;
      }
      if (self.lang.characters && ch == '\'' && self.character())
        || self.number()
        || (self.lang.regexes && ch == '/' && self.regex())
        || (self.lang.variables && ch == '$' && self.variable())
        || (self.lang.at_words && ch == '@' && self.at_word())
        || (self.lang.selectors && self.selector(ch))
      {
        continue;
      }
      if self.is_name_start(ch) {
        self.name();
        continue;
      }
      self.mark(ch);
    }
    if self.lang.block_scalars && self.opens_block_scalar() {
      state.inside = Inside::BlockScalar { indent };
    }
  }

  /// A section's head: `[name]` or `[[name.part]]` at the start of the line.
  fn section(&mut self) -> bool {
    let rest = self.rest();
    let open = if rest.starts_with("[[") { 2 } else { 1 };
    let Some(close) = rest.find(']') else {
      return false;
    };
    let start = self.at;
    self.push(start, start + open, NAMES.punctuation);
    let mut at = start + open;
    while at < start + close {
      let ch = self.char_at(at).unwrap_or(' ');
      if ch == '"' || ch == '\'' {
        let end = self.text[at + 1..start + close]
          .find(ch)
          .map_or(start + close, |found| at + 1 + found + 1);
        self.push(at, end, NAMES.namespace);
        at = end;
      } else if ch.is_whitespace() || ch == '.' {
        at += ch.len_utf8();
      } else {
        let end = self.text[at..start + close]
          .find(|ch: char| ch.is_whitespace() || ch == '.')
          .map_or(start + close, |found| at + found);
        self.push(at, end, NAMES.namespace);
        at = end;
      }
    }
    let after = start + close;
    let run = self.text[after..].chars().take_while(|ch| *ch == ']').count().min(open);
    self.push(after, after + run, NAMES.punctuation);
    self.at = after + run;
    true
  }

  fn line_comment(&mut self) -> bool {
    let rest = self.rest();
    let found = self
      .lang
      .line_comments
      .iter()
      .find(|(mark, _)| rest.starts_with(&**mark))
      .map(|(_, name)| *name);
    let Some(name) = found else {
      return false;
    };
    if self.lang.comment_after_space && self.char_before(self.at).is_some_and(|ch| !ch.is_whitespace()) {
      return false;
    }
    self.code_end = self.at;
    self.push(self.at, self.text.len(), name);
    self.at = self.text.len();
    true
  }

  fn block_comment(&mut self, state: &mut ScanState) -> bool {
    let rest = self.rest();
    let form = self.lang.block_comments.iter().position(|comment| {
      rest.starts_with(&*comment.open)
        // `/**/` and `/***` are no comments that document.
        && !(comment.name.as_str() == token::COMMENT_DOC
          && rest[comment.open.len()..].starts_with(['*', '/']))
    });
    let Some(form) = form else {
      return false;
    };
    let open = self.lang.block_comments[form].open.len();
    self.comment_from(form, 0, self.at + open, state);
    true
  }

  /// The comment of `form` from where it starts on this line (`self.at`) to its close, from `from`.
  fn comment_from(&mut self, form: usize, mut depth: u32, from: usize, state: &mut ScanState) {
    let lang = self.lang;
    let comment = &lang.block_comments[form];
    let mut at = from;
    while at < self.text.len() {
      let rest = &self.text[at..];
      if comment.nests && rest.starts_with(&*comment.open) {
        depth += 1;
        at += comment.open.len();
        continue;
      }
      if rest.starts_with(&*comment.close) {
        at += comment.close.len();
        if depth == 0 {
          self.push(self.at, at, comment.name);
          self.at = at;
          state.inside = Inside::Nothing;
          self.prev = Prev::Open;
          return;
        }
        depth -= 1;
        continue;
      }
      at += rest.chars().next().map_or(1, char::len_utf8);
    }
    self.push(self.at, self.text.len(), comment.name);
    self.at = self.text.len();
    state.inside = Inside::Comment { form, depth };
  }

  fn attribute(&mut self) -> bool {
    let rest = self.rest();
    let Some(mark) = self.lang.attributes.iter().find(|mark| rest.starts_with(&***mark)) else {
      return false;
    };
    let after = self.at + mark.len();
    if !self.char_at(after).is_some_and(|ch| self.is_name_start(ch)) {
      return false;
    }
    let mut end = self.name_end(after);
    // A path: `serde::rename`, `app.route`.
    loop {
      let rest = &self.text[end..];
      let step = if rest.starts_with("::") {
        2
      } else if rest.starts_with('.') {
        1
      } else {
        break;
      };
      let next = self.name_end(end + step);
      if next == end + step {
        break;
      }
      end = next;
    }
    self.push(self.at, end, NAMES.attribute);
    self.at = end;
    self.prev = Prev::Value;
    self.declare = None;
    true
  }

  fn string(&mut self, state: &mut ScanState) -> bool {
    let lang = self.lang;
    let before = self.char_before(self.at);
    for (index, form) in lang.strings.iter().enumerate() {
      for prefix in &form.prefixes {
        let rest = self.rest();
        if rest.len() < prefix.len() || !rest.is_char_boundary(prefix.len()) {
          continue;
        }
        let matches = if lang.prefixes_any_case {
          rest[..prefix.len()].eq_ignore_ascii_case(prefix)
        } else {
          rest.starts_with(&**prefix)
        };
        if !matches || (!prefix.is_empty() && before.is_some_and(|ch| self.is_name_char(ch))) {
          continue;
        }
        let mut at = self.at + prefix.len();
        let mut hashes = 0;
        if form.hashes {
          while self.text[at..].starts_with('#') {
            hashes += 1;
            at += 1;
          }
        }
        if !self.text[at..].starts_with(&*form.open) {
          continue;
        }
        let start = self.at;
        self.string_from(index, hashes, start, at + form.open.len(), state);
        if state.is_clear() && self.lang.keys != KeyRule::None && self.is_string_key() {
          self.rename_since(start, NAMES.property);
        }
        return true;
      }
    }
    false
  }

  /// The string of `form` that starts at `start` on this line, from `from` to its close.
  fn string_from(&mut self, index: usize, hashes: u32, start: usize, from: usize, state: &mut ScanState) {
    let lang = self.lang;
    let form = &lang.strings[index];
    let mut close = form.close.to_string();
    close.extend(std::iter::repeat_n('#', hashes as usize));
    let mut piece = start;
    let mut at = from;
    while at < self.text.len() {
      let rest = &self.text[at..];
      if form.escapes && rest.starts_with('\\') {
        let len = escape_len(rest);
        self.push(piece, at, form.name);
        self.push(at, at + len, NAMES.escape);
        at += len;
        piece = at;
        continue;
      }
      if rest.starts_with(&*close) {
        if form.doubled && rest[close.len()..].starts_with(&*close) {
          at += 2 * close.len();
          continue;
        }
        at += close.len();
        self.push(piece, at, form.name);
        self.at = at;
        self.prev = Prev::Value;
        self.declare = None;
        state.inside = Inside::Nothing;
        return;
      }
      at += rest.chars().next().map_or(1, char::len_utf8);
    }
    self.push(piece, self.text.len(), form.name);
    self.at = self.text.len();
    if form.multiline {
      state.inside = Inside::String { form: index, hashes };
    }
  }

  /// Whether the string just read is a key: a `:` follows it (JSON, YAML), or an `=` (TOML).
  fn is_string_key(&self) -> bool {
    let after = self.text[self.at..].trim_start();
    match self.lang.keys {
      KeyRule::Colon => after.starts_with(':'),
      KeyRule::Equals => self.is_dotted_key(self.at),
      _ => false,
    }
  }

  /// Gives the tokens pushed from `start` on this line the name `kind`.
  fn rename_since(&mut self, start: usize, kind: TokenName) {
    let from = self.base + start;
    let first = self
      .out
      .iter()
      .rposition(|token| token.range.start < from)
      .map_or(0, |index| index + 1);
    let range = self.out[first..]
      .first()
      .zip(self.out.last())
      .map(|(first, last)| first.range.start..last.range.end);
    if let Some(range) = range {
      self.out.truncate(first);
      self.push(range.start - self.base, range.end - self.base, kind);
    }
  }

  /// Rust's: a character (`'a'`, `'\n'`) is a string, a lifetime (`'a`, `'static`) a label.
  fn character(&mut self) -> bool {
    let start = self.at;
    let Some(first) = self.char_at(start + 1) else {
      return false;
    };
    if first == '\\' {
      let len = escape_len(&self.text[start + 1..]);
      let close = start + 1 + len;
      if self.text[close..].starts_with('\'') {
        self.push(start, start + 1, NAMES.string);
        self.push(start + 1, close, NAMES.escape);
        self.push(close, close + 1, NAMES.string);
        self.at = close + 1;
        self.prev = Prev::Value;
        return true;
      }
      return false;
    }
    let after = start + 1 + first.len_utf8();
    if self.text[after..].starts_with('\'') {
      self.push(start, after + 1, NAMES.string);
      self.at = after + 1;
      self.prev = Prev::Value;
      return true;
    }
    if self.is_name_start(first) {
      let end = self.name_end(start + 1);
      self.push(start, end, NAMES.label);
      self.at = end;
      self.prev = Prev::Value;
      return true;
    }
    false
  }

  fn number(&mut self) -> bool {
    let start = self.at;
    let Some(first) = self.char_at(start) else {
      return false;
    };
    let digit_after_point = first == '.' && self.char_at(start + 1).is_some_and(|ch| ch.is_ascii_digit());
    // CSS's `-10px`, which would be a name there.
    let negative = first == '-'
      && self.lang.selectors
      && self
        .char_at(start + 1)
        .is_some_and(|ch| ch.is_ascii_digit() || ch == '.');
    if !first.is_ascii_digit() && !digit_after_point && !negative {
      return false;
    }
    if self
      .char_before(start)
      .is_some_and(|ch| self.is_name_char(ch) || (digit_after_point && ch == '.'))
    {
      return false;
    }
    let forms = &self.lang.numbers;
    let bytes = self.text.as_bytes();
    let mut at = start + usize::from(negative);
    let digits = |at: &mut usize, hex: bool| {
      while *at < bytes.len() {
        let byte = bytes[*at];
        if byte.is_ascii_digit() || (hex && byte.is_ascii_hexdigit()) || (forms.underscores && byte == b'_') {
          *at += 1;
        } else {
          break;
        }
      }
    };
    let prefixed =
      forms.prefixed && first == '0' && matches!(bytes.get(start + 1), Some(b'x' | b'X' | b'o' | b'O' | b'b' | b'B'));
    if prefixed {
      at += 2;
      let hex = matches!(bytes[start + 1], b'x' | b'X');
      digits(&mut at, hex);
    } else {
      digits(&mut at, false);
      // A fraction, not a range (`1..2`) nor a method (`1.max(2)`).
      if bytes.get(at) == Some(&b'.') && bytes.get(at + 1).is_some_and(u8::is_ascii_digit) {
        at += 1;
        digits(&mut at, false);
      }
      if matches!(bytes.get(at), Some(b'e' | b'E')) {
        let sign = usize::from(matches!(bytes.get(at + 1), Some(b'+' | b'-')));
        if bytes.get(at + 1 + sign).is_some_and(u8::is_ascii_digit) {
          at += 1 + sign;
          digits(&mut at, false);
        }
      }
      if forms.dates && matches!(bytes.get(at), Some(b'-' | b':')) && bytes.get(at + 1).is_some_and(u8::is_ascii_digit)
      {
        while at < bytes.len() && (bytes[at].is_ascii_digit() || b"-:.TZ+".contains(&bytes[at])) {
          at += 1;
        }
      }
    }
    if forms.suffixes {
      while at < bytes.len() && (bytes[at].is_ascii_alphanumeric() || bytes[at] == b'_' || bytes[at] == b'%') {
        at += 1;
      }
    }
    self.push(start, at, NAMES.number);
    self.at = at;
    self.prev = Prev::Value;
    self.declare = None;
    true
  }

  /// JavaScript's `/…/flags`, where a value may start.
  fn regex(&mut self) -> bool {
    if self.prev == Prev::Value {
      return false;
    }
    let start = self.at;
    let mut at = start + 1;
    let mut class = false;
    while at < self.text.len() {
      let ch = self.char_at(at).unwrap_or(' ');
      match ch {
        '\\' => {
          at += 1 + self.char_at(at + 1).map_or(0, char::len_utf8);
          continue;
        }
        '[' => class = true,
        ']' => class = false,
        '/' if !class => {
          if at == start + 1 {
            return false;
          }
          at += 1;
          while self.char_at(at).is_some_and(|ch| ch.is_ascii_alphabetic()) {
            at += 1;
          }
          self.push(start, at, NAMES.regex);
          self.at = at;
          self.prev = Prev::Value;
          return true;
        }
        _ => {}
      }
      at += ch.len_utf8();
    }
    false
  }

  /// A shell's `$NAME`, `${…}`, `$1`, `$?`; `$(` opens a command.
  fn variable(&mut self) -> bool {
    let start = self.at;
    let end = match self.char_at(start + 1) {
      Some('{') => self.text[start..]
        .find('}')
        .map_or(self.text.len(), |close| start + close + 1),
      Some('(') => {
        self.push(start, start + 2, NAMES.punctuation);
        self.at = start + 2;
        self.command = true;
        self.prev = Prev::Open;
        return true;
      }
      Some(ch) if ch.is_ascii_digit() || "@*#?$!-".contains(ch) => start + 2,
      Some(ch) if self.is_name_start(ch) => {
        let mut end = start + 1;
        for (index, ch) in self.text[start + 1..].char_indices() {
          if !(ch.is_alphanumeric() || ch == '_') {
            break;
          }
          end = start + 1 + index + ch.len_utf8();
        }
        end
      }
      _ => return false,
    };
    self.push(start, end, NAMES.variable);
    self.at = end;
    self.prev = Prev::Value;
    self.command = false;
    true
  }

  /// CSS's `@media`.
  fn at_word(&mut self) -> bool {
    let end = self.name_end(self.at + 1);
    if end == self.at + 1 {
      return false;
    }
    self.push(self.at, end, NAMES.keyword);
    self.at = end;
    self.prev = Prev::Keyword;
    true
  }

  /// CSS's selectors outside a value (`.class`, `#id`, `:hover`), and in a value a color (`#fff`) and
  /// `!important`.
  fn selector(&mut self, ch: char) -> bool {
    let start = self.at;
    if self.value {
      if ch == '#' {
        let end = self.name_end(start + 1);
        let hex = &self.text[start + 1..end];
        if matches!(hex.len(), 3 | 4 | 6 | 8) && hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
          self.push(start, end, NAMES.constant);
          self.at = end;
          return true;
        }
      }
      if ch == '!' && self.text[start + 1..].starts_with("important") {
        self.push(start, start + 10, NAMES.keyword);
        self.at = start + 10;
        return true;
      }
      return false;
    }
    if self.value_next {
      return false;
    }
    let (mark, kind) = match ch {
      '.' | '#' => (1, NAMES.attribute),
      ':' if self.text[start..].starts_with("::") => (2, NAMES.keyword),
      ':' => (1, NAMES.keyword),
      _ => return false,
    };
    let end = self.name_end(start + mark);
    if end == start + mark {
      return false;
    }
    self.push(start, end, kind);
    self.at = end;
    true
  }

  fn name(&mut self) {
    let lang = self.lang;
    let start = self.at;
    let end = self.name_end(start);
    let word = &self.text[start..end];
    let lower;
    let lookup = if lang.any_case {
      lower = word.to_lowercase();
      lower.as_str()
    } else {
      word
    };
    let after = &self.text[end..];
    let mut end = end;
    let mut prev = Prev::Value;
    let mut command = false;
    let declared = self.declare.take();
    let kind = if let Some(&keyword) = lang.words.get(lookup)
      && !(lang.keys != KeyRule::None && self.is_key(end))
    {
      self.declare = lang.declares.get(lookup).copied();
      prev = Prev::Keyword;
      command = lang.commands_after.iter().any(|word| **word == *lookup);
      Some(keyword)
    } else if let Some(declared) = declared {
      Some(declared)
    } else if lang.keys != KeyRule::None && (self.is_key(end) || self.is_condition_key(start, end)) {
      if lang.keys == KeyRule::Declaration {
        self.value_next = true;
      }
      Some(NAMES.property)
    } else if lang.macros && after.starts_with('!') && !after.starts_with("!=") {
      end += 1;
      Some(NAMES.macro_)
    } else if let Some(path) = &lang.path
      && after.starts_with(&**path)
      && !(lang.capitals_are_types && starts_capital(word))
    {
      Some(NAMES.namespace)
    } else if lang.capitals_are_types && is_constant(word) {
      Some(NAMES.constant)
    } else if lang.capitals_are_types && starts_capital(word) {
      Some(NAMES.type_)
    } else if lang.calls && (after.starts_with('(') || after.starts_with("::<")) {
      Some(NAMES.function)
    } else if lang.commands && self.command {
      Some(NAMES.function)
    } else if lang.selectors && !self.value {
      if word.starts_with("--") {
        Some(NAMES.variable)
      } else {
        Some(NAMES.tag)
      }
    } else if lang.selectors && word.starts_with("--") {
      Some(NAMES.variable)
    } else {
      None
    };
    if let Some(kind) = kind {
      self.push(start, end, kind);
    }
    self.at = end;
    self.prev = prev;
    self.command = command;
  }

  /// Whether the name that ends at `end` is a key, by the language's rule.
  fn is_key(&self, end: usize) -> bool {
    let after = &self.text[end..];
    match self.lang.keys {
      KeyRule::None => false,
      KeyRule::Colon => {
        let after = after.trim_start();
        after.starts_with(':') && after[1..].chars().next().is_none_or(char::is_whitespace)
      }
      KeyRule::Equals => self.is_dotted_key(end),
      KeyRule::Declaration => {
        let after = after.trim_start();
        if !after.starts_with(':') || self.value {
          return false;
        }
        // `a:hover {` is a selector: a `{` follows before a `;`.
        let line = &after[1..];
        match (line.find('{'), line.find(';')) {
          (Some(brace), Some(semicolon)) => semicolon < brace,
          (Some(_), None) => false,
          _ => true,
        }
      }
    }
  }

  /// CSS's: a name in parentheses followed by `:`, the feature of a condition: `(max-width: 640px)`.
  fn is_condition_key(&self, start: usize, end: usize) -> bool {
    self.lang.keys == KeyRule::Declaration
      && self.text[..start].trim_end().ends_with('(')
      && self.text[end..].trim_start().starts_with(':')
  }

  /// TOML's: whether what follows `end` is the rest of a dotted key and its `=`.
  fn is_dotted_key(&self, end: usize) -> bool {
    let mut rest = self.text[end..].trim_start();
    while let Some(next) = rest.strip_prefix('.') {
      let next = next.trim_start();
      let skip = if let Some(quote) = next.chars().next().filter(|ch| *ch == '"' || *ch == '\'') {
        next[1..].find(quote).map_or(next.len(), |close| close + 2)
      } else {
        next
          .find(|ch: char| !(ch.is_alphanumeric() || ch == '_' || ch == '-'))
          .unwrap_or(next.len())
      };
      rest = next[skip..].trim_start();
    }
    rest.starts_with('=') && !rest.starts_with("==")
  }

  fn mark(&mut self, ch: char) {
    let start = self.at;
    let lang = self.lang;
    if let Some(path) = &lang.path
      && self.text[start..].starts_with(&**path)
    {
      self.push(start, start + path.len(), NAMES.punctuation);
      self.at = start + path.len();
      self.prev = Prev::Open;
      return;
    }
    if ch == '.' {
      self.at += 1;
      self.prev = Prev::Open;
      return;
    }
    if lang.operators.contains(ch) {
      let mut end = start;
      for (index, next) in self.text[start..].char_indices() {
        if index > 0 {
          // A comment, a regular expression or a value after an operator is not the operator's.
          let here = &self.text[start + index..];
          if !lang.operators.contains(next)
            || self.value_next
            || (lang.regexes && next == '/')
            || lang.line_comments.iter().any(|(mark, _)| here.starts_with(&**mark))
            || lang
              .block_comments
              .iter()
              .any(|comment| here.starts_with(&*comment.open))
          {
            break;
          }
        }
        end = start + index + next.len_utf8();
      }
      let operator = &self.text[start..end];
      self.push(start, end, NAMES.operator);
      if self.value_next && operator == ":" {
        self.value_next = false;
        self.value = true;
      }
      if lang.commands {
        self.command = matches!(operator, "|" | "||" | "&&" | "&" | "|&" | "!");
      }
      self.at = end;
      self.prev = Prev::Open;
      self.declare = None;
      return;
    }
    self.push(start, start + ch.len_utf8(), NAMES.punctuation);
    self.at = start + ch.len_utf8();
    self.prev = if matches!(ch, ')' | ']' | '}') {
      Prev::Value
    } else {
      Prev::Open
    };
    if lang.commands {
      self.command = matches!(ch, '(' | ';' | '{' | '`');
    }
    if lang.selectors && matches!(ch, ';' | '{' | '}') {
      self.value = false;
      self.value_next = false;
    }
    if !(ch == '<' || ch == '>') {
      self.declare = None;
    }
  }

  /// YAML's: whether the line ends in `|` or `>` after a key or a dash, which opens a block of text.
  fn opens_block_scalar(&self) -> bool {
    let code = self.text[..self.code_end].trim_end();
    let code = code.trim_end_matches(|ch: char| ch.is_ascii_digit() || ch == '+' || ch == '-');
    let Some(before) = code.strip_suffix('|').or_else(|| code.strip_suffix('>')) else {
      return false;
    };
    let before = before.trim_end();
    before.ends_with(':') || before.ends_with('-')
  }

  // Markup: HTML and XML.

  fn markup(&mut self, state: &mut ScanState) {
    let mut in_tag = state.inside == Inside::Tag;
    while self.at < self.text.len() {
      if in_tag {
        in_tag = self.tag(state);
        if !state.is_clear() && !in_tag {
          return;
        }
        continue;
      }
      if self.block_comment(state) || self.string(state) {
        if !state.is_clear() {
          return;
        }
        continue;
      }
      let rest = self.rest();
      if rest.starts_with('<') {
        let open = if rest.starts_with("</") || rest.starts_with("<?") || rest.starts_with("<!") {
          2
        } else {
          1
        };
        let name_end = self.name_end(self.at + open);
        if name_end > self.at + open {
          self.push(self.at, self.at + open, NAMES.punctuation);
          let kind = if rest.starts_with("<!") {
            NAMES.keyword
          } else {
            NAMES.tag
          };
          self.push(self.at + open, name_end, kind);
          self.at = name_end;
          state.inside = Inside::Tag;
          in_tag = true;
          continue;
        }
      }
      if rest.starts_with('&')
        && let Some(close) = rest[1..].find(';').filter(|close| *close > 0 && *close <= 32)
        && rest[1..=close].chars().all(|ch| ch.is_alphanumeric() || ch == '#')
      {
        self.push(self.at, self.at + close + 2, NAMES.constant);
        self.at += close + 2;
        continue;
      }
      // Plain text, to the next mark.
      let first = rest.chars().next().map_or(1, char::len_utf8);
      let skip = rest[first..].find(['<', '&']).map_or(rest.len(), |found| found + first);
      self.at += skip;
    }
  }

  /// Inside a tag, from its name to its end. Whether the line goes on inside it.
  fn tag(&mut self, state: &mut ScanState) -> bool {
    while self.at < self.text.len() {
      let Some(ch) = self.char_at(self.at) else {
        break;
      };
      let rest = self.rest();
      if ch.is_whitespace() {
        self.at += ch.len_utf8();
      } else if rest.starts_with("/>") || rest.starts_with("?>") {
        self.push(self.at, self.at + 2, NAMES.punctuation);
        self.at += 2;
        state.inside = Inside::Nothing;
        return false;
      } else if ch == '>' {
        self.push(self.at, self.at + 1, NAMES.punctuation);
        self.at += 1;
        state.inside = Inside::Nothing;
        return false;
      } else if ch == '=' {
        self.push(self.at, self.at + 1, NAMES.operator);
        self.at += 1;
      } else if ch == '"' || ch == '\'' {
        let start = self.at;
        self.at += 1;
        if !self.tag_value_from(ch, start, state) {
          return false;
        }
      } else if self.is_name_start(ch) {
        let end = self.name_end(self.at);
        self.push(self.at, end, NAMES.attribute);
        self.at = end;
      } else {
        // A value with no quotes, to a space or the end of the tag.
        let end = rest
          .find(|ch: char| ch.is_whitespace() || ch == '>')
          .map_or(self.text.len(), |found| self.at + found);
        let end = end.max(self.at + ch.len_utf8());
        self.push(self.at, end, NAMES.string);
        self.at = end;
      }
    }
    state.inside = Inside::Tag;
    true
  }

  /// A value in `quote` that a line before opened: whether it closed on this line.
  fn tag_value(&mut self, quote: char, from: usize, state: &mut ScanState) -> bool {
    self.at = from;
    if self.tag_value_from(quote, from, state) {
      state.inside = Inside::Tag;
      return true;
    }
    false
  }

  /// The rest of a value from `self.at`, from `start`: whether it closed on this line (the line ends inside it
  /// otherwise).
  fn tag_value_from(&mut self, quote: char, start: usize, state: &mut ScanState) -> bool {
    match self.text[self.at..].find(quote) {
      Some(close) => {
        let end = self.at + close + 1;
        self.push(start, end, NAMES.string);
        self.at = end;
        true
      }
      None => {
        self.push(start, self.text.len(), NAMES.string);
        self.at = self.text.len();
        state.inside = Inside::TagValue { quote };
        false
      }
    }
  }

  // Markdown.

  fn markdown(&mut self, state: &mut ScanState) {
    if self.at > 0 {
      self.inline(state);
      return;
    }
    let indent = self.skip_spaces(0);
    let line = &self.text[indent..];
    let end = self.text.len();
    for mark in ['`', '~'] {
      let run = line.chars().take_while(|ch| *ch == mark).count();
      if run >= 3 && !(mark == '`' && line[run..].contains('`')) {
        self.push(indent, end, NAMES.code);
        state.inside = Inside::Fence { mark, count: run };
        return;
      }
    }
    let hashes = line.chars().take_while(|ch| *ch == '#').count();
    if (1..=6).contains(&hashes) && line[hashes..].chars().next().is_none_or(char::is_whitespace) {
      self.push(indent, end, NAMES.heading);
      return;
    }
    if line.starts_with('>') {
      self.push(indent, end, NAMES.quote);
      return;
    }
    let trimmed = line.trim_end();
    if trimmed.len() >= 3
      && let Some(first) = trimmed.chars().next()
      && matches!(first, '-' | '=' | '*' | '_')
      && trimmed.chars().all(|ch| ch == first || ch == ' ')
    {
      self.push(indent, indent + trimmed.len(), NAMES.punctuation);
      return;
    }
    self.at = indent;
    let digits = line.chars().take_while(char::is_ascii_digit).count();
    let marker = if line.starts_with(['-', '*', '+']) && line[1..].starts_with(' ') {
      1
    } else if digits > 0 && line[digits..].starts_with(['.', ')']) && line[digits + 1..].starts_with(' ') {
      digits + 1
    } else {
      0
    };
    if marker > 0 {
      self.push(indent, indent + marker, NAMES.list);
      self.at = indent + marker;
    }
    self.inline(state);
  }

  /// The spans of a line of Markdown: code, strong text, emphasis, links.
  fn inline(&mut self, state: &mut ScanState) {
    while self.at < self.text.len() {
      let rest = self.rest();
      let Some(ch) = rest.chars().next() else {
        break;
      };
      if ch == '\\' {
        self.at += 1 + rest[1..].chars().next().map_or(0, char::len_utf8);
        continue;
      }
      if ch == '<' && self.block_comment(state) {
        if !state.is_clear() {
          return;
        }
        continue;
      }
      if ch == '`' {
        let run = rest.chars().take_while(|ch| *ch == '`').count();
        let fence = &rest[..run];
        match rest[run..].find(fence) {
          Some(close) => {
            let end = self.at + run + close + run;
            self.push(self.at, end, NAMES.code);
            self.at = end;
          }
          None => self.at += run,
        }
        continue;
      }
      if rest.starts_with("**") || rest.starts_with("__") {
        let mark = &rest[..2];
        if let Some(close) = rest[2..].find(mark).filter(|close| *close > 0) {
          let end = self.at + 2 + close + 2;
          self.push(self.at, end, NAMES.bold);
          self.at = end;
          continue;
        }
      }
      if (ch == '*' || ch == '_')
        && !(ch == '_' && self.char_before(self.at).is_some_and(char::is_alphanumeric))
        && rest[1..].chars().next().is_some_and(|next| !next.is_whitespace())
        && let Some(close) = rest[1..].find(ch).filter(|close| *close > 0)
      {
        let end = self.at + 1 + close + 1;
        self.push(self.at, end, NAMES.italic);
        self.at = end;
        continue;
      }
      if ch == '[' || (ch == '!' && rest[1..].starts_with('[')) {
        let open = if ch == '!' { 2 } else { 1 };
        if let Some(close) = rest[open..].find(']') {
          let text_end = self.at + open + close + 1;
          let after = &self.text[text_end..];
          let target = if after.starts_with('(') {
            after.find(')').map(|close| close + 1)
          } else if after.starts_with('[') {
            after.find(']').map(|close| close + 1)
          } else {
            None
          };
          if target.is_some() || after.starts_with(':') {
            self.push(self.at, text_end, NAMES.link);
            self.at = text_end;
            if let Some(target) = target {
              self.push(text_end, text_end + target, NAMES.link_url);
              self.at = text_end + target;
            }
            continue;
          }
        }
      }
      if ch == '<'
        && let Some(close) = rest.find('>')
        && (rest[1..].starts_with("http") || rest[1..close].contains('@'))
      {
        self.push(self.at, self.at + close + 1, NAMES.link);
        self.at += close + 1;
        continue;
      }
      self.at += ch.len_utf8();
    }
  }
}

/// The length of the escape `rest` starts with: `\n`, `\x41`, `\u{1F600}`, `é`, `\101`.
fn escape_len(rest: &str) -> usize {
  let mut chars = rest.char_indices().skip(1);
  let Some((at, ch)) = chars.next() else {
    return rest.len();
  };
  let hex = |from: usize, most: usize| {
    rest[from..]
      .char_indices()
      .take(most)
      .take_while(|(_, ch)| ch.is_ascii_hexdigit())
      .count()
  };
  match ch {
    'x' => 2 + hex(2, 2),
    'u' if rest[2..].starts_with('{') => rest.find('}').map_or(2, |close| close + 1),
    'u' => 2 + hex(2, 4),
    'U' => 2 + hex(2, 8),
    '0'..='7' => {
      1 + rest[1..]
        .char_indices()
        .take(3)
        .take_while(|(_, ch)| ('0'..='7').contains(ch))
        .count()
    }
    _ => at + ch.len_utf8(),
  }
}

fn starts_capital(word: &str) -> bool {
  word.trim_start_matches('_').starts_with(|ch: char| ch.is_uppercase())
}

/// A name in capitals alone, of more than one letter: `MAX_LEN`, `HTTP2`.
fn is_constant(word: &str) -> bool {
  word.chars().filter(|ch| ch.is_alphabetic()).count() > 1
    && word
      .chars()
      .all(|ch| ch.is_uppercase() || ch.is_ascii_digit() || ch == '_')
}
