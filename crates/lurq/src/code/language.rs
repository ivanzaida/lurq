//! A language as a table: what lurq's own highlighters are, and what an app describes a language of its own with.

use std::{collections::HashMap, sync::Arc};

use super::{CodeToken, Highlighter, ScanState, TokenName, token};

/// What the scanner reads a language's code as.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CodeMode {
  /// A language of words, strings, numbers and marks: Rust, JSON, SQL.
  #[default]
  Code,
  /// Text with tags in it: HTML, XML. The tags have their names, attributes and values; the text between them is
  /// plain, but for its entities (`&amp;`) and comments.
  Markup,
  /// Markdown: headings, quotes and the markers of lists by the start of their lines, fenced code, and in a line
  /// code spans, strong text, emphasis and links.
  Markdown,
}

/// A comment from a mark to a mark, over lines: `/*` to `*/`.
#[derive(Clone, Debug, PartialEq)]
pub struct BlockComment {
  pub open: Arc<str>,
  pub close: Arc<str>,
  /// Whether one inside it is closed before it is (Rust's).
  pub nests: bool,
  /// Its token's name: [`token::COMMENT`], or [`token::COMMENT_DOC`] for one that documents.
  pub name: TokenName,
}

/// A form of string: what opens it, what closes it, and what is in it.
#[derive(Clone, Debug, PartialEq)]
pub struct StringForm {
  pub open: Arc<str>,
  pub close: Arc<str>,
  /// Letters that may stand right before the open mark, as part of the string (`b`, `r`, `f`): `""` for none.
  /// Matched in any case where the language's prefixes are ([`CodeLanguage::prefixes_any_case`]).
  pub prefixes: Vec<Arc<str>>,
  /// Whether a backslash escapes what follows it, each escape a token of [`token::STRING_ESCAPE`].
  pub escapes: bool,
  /// Whether the close mark twice is one of it, as `''` is in SQL.
  pub doubled: bool,
  /// Whether it goes on past the end of a line; one that does not ends with its line.
  pub multiline: bool,
  /// Whether `#`s between the prefix and the open mark are closed by as many after the close mark (Rust's raw
  /// strings, `r#"…"#`).
  pub hashes: bool,
  /// Its token's name: [`token::STRING`] unless told.
  pub name: TokenName,
}

impl StringForm {
  /// A string from `open` to `close`, with escapes, on one line.
  pub fn new(open: &str, close: &str) -> Self {
    Self {
      open: Arc::from(open),
      close: Arc::from(close),
      prefixes: vec![Arc::from("")],
      escapes: true,
      doubled: false,
      multiline: false,
      hashes: false,
      name: token::STRING.into(),
    }
  }

  /// The letters that may stand before it, `""` for none.
  pub fn prefixes(mut self, prefixes: &[&str]) -> Self {
    self.prefixes = prefixes.iter().map(|prefix| Arc::from(*prefix)).collect();
    self
  }

  pub fn escapes(mut self, escapes: bool) -> Self {
    self.escapes = escapes;
    self
  }

  pub fn doubled(mut self, doubled: bool) -> Self {
    self.doubled = doubled;
    self
  }

  pub fn multiline(mut self, multiline: bool) -> Self {
    self.multiline = multiline;
    self
  }

  pub fn hashes(mut self, hashes: bool) -> Self {
    self.hashes = hashes;
    self
  }

  pub fn name(mut self, name: impl Into<TokenName>) -> Self {
    self.name = name.into();
    self
  }
}

/// The forms of numbers a language writes.
#[derive(Clone, Debug, PartialEq)]
pub struct NumberForms {
  /// `0x1F`, `0o17`, `0b1010`.
  pub prefixed: bool,
  /// `1_000_000`.
  pub underscores: bool,
  /// A suffix or a unit right after it is part of it: `1u8`, `2.0f32`, `10px`, `50%`, `10n`.
  pub suffixes: bool,
  /// Dates and times of TOML: `1979-05-27T07:32:00Z`, `07:32:00`.
  pub dates: bool,
}

impl Default for NumberForms {
  fn default() -> Self {
    Self {
      prefixed: true,
      underscores: false,
      suffixes: false,
      dates: false,
    }
  }
}

/// Where a key or a property is: a name followed by its mark.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum KeyRule {
  /// None: a name is a name.
  #[default]
  None,
  /// A name, or a string, followed by `:` (JSON, YAML; a name followed by `:` and then a space or the end of the
  /// line, in YAML).
  Colon,
  /// A name, dotted or quoted, followed by `=`, at the start of a line or in braces (TOML).
  Equals,
  /// A name followed by `:` where no `{` follows on its line (CSS: `color: red`, not `a:hover {`); what follows is
  /// its value up to `;` or `}`.
  Declaration,
}

/// A language as a table: its names, its words and the forms of its comments, strings, numbers, names and marks.
/// One scanner reads every language from its table, line by line, from a [`ScanState`] at the start of each
/// line.
///
/// lurq's own are [`CodeLanguage::rust`], [`toml`](Self::toml), [`json`](Self::json),
/// [`javascript`](Self::javascript), [`typescript`](Self::typescript), [`shell`](Self::shell),
/// [`python`](Self::python), [`css`](Self::css), [`html`](Self::html), [`xml`](Self::xml), [`yaml`](Self::yaml),
/// [`sql`](Self::sql) and [`markdown`](Self::markdown), registered for their names. An app describes one of its own
/// with the same table, or changes one of lurq's, and registers it:
///
/// ```
/// use lurq::code::{CodeLanguage, StringForm, register_highlighter, token};
///
/// let ini = CodeLanguage::new("ini")
///   .aliases(&["cfg"])
///   .line_comments(&[";", "#"])
///   .strings(vec![StringForm::new("\"", "\"")])
///   .words(token::CONSTANT_LANGUAGE, &["true", "false"])
///   .keys(lurq::code::KeyRule::Equals)
///   .sections(true);
/// register_highlighter("ini", ini);
///
/// // Rust, with one more word.
/// register_highlighter("rust", CodeLanguage::rust().words(token::KEYWORD, &["yeet"]));
/// ```
#[derive(Clone, Debug)]
pub struct CodeLanguage {
  pub(super) name: Arc<str>,
  pub(super) aliases: Vec<Arc<str>>,
  pub(super) mode: CodeMode,
  pub(super) words: HashMap<Box<str>, TokenName>,
  pub(super) any_case: bool,
  /// Each with its name, the longest first.
  pub(super) line_comments: Vec<(Arc<str>, TokenName)>,
  pub(super) comment_after_space: bool,
  pub(super) block_comments: Vec<BlockComment>,
  pub(super) strings: Vec<StringForm>,
  pub(super) prefixes_any_case: bool,
  pub(super) characters: bool,
  pub(super) numbers: NumberForms,
  pub(super) capitals_are_types: bool,
  pub(super) path: Option<Arc<str>>,
  pub(super) calls: bool,
  pub(super) macros: bool,
  pub(super) declares: HashMap<Box<str>, TokenName>,
  pub(super) attributes: Vec<Arc<str>>,
  pub(super) at_words: bool,
  pub(super) variables: bool,
  pub(super) name_chars: Arc<str>,
  pub(super) name_starts: Arc<str>,
  pub(super) keys: KeyRule,
  pub(super) sections: bool,
  pub(super) regexes: bool,
  pub(super) commands: bool,
  pub(super) commands_after: Vec<Box<str>>,
  pub(super) selectors: bool,
  pub(super) block_scalars: bool,
  pub(super) operators: Arc<str>,
}

impl CodeLanguage {
  /// A language called `name` with nothing in its table: its code is plain until it is told what to tell apart.
  pub fn new(name: &str) -> Self {
    Self {
      name: Arc::from(name),
      aliases: Vec::new(),
      mode: CodeMode::Code,
      words: HashMap::new(),
      any_case: false,
      line_comments: Vec::new(),
      comment_after_space: false,
      block_comments: Vec::new(),
      strings: Vec::new(),
      prefixes_any_case: false,
      characters: false,
      numbers: NumberForms::default(),
      capitals_are_types: false,
      path: None,
      calls: false,
      macros: false,
      declares: HashMap::new(),
      attributes: Vec::new(),
      at_words: false,
      variables: false,
      name_chars: Arc::from("_"),
      name_starts: Arc::from("_"),
      keys: KeyRule::None,
      sections: false,
      regexes: false,
      commands: false,
      commands_after: Vec::new(),
      selectors: false,
      block_scalars: false,
      operators: Arc::from("=+-*/%<>!&|^~?:"),
    }
  }

  /// The name it is registered by.
  pub fn name(&self) -> Arc<str> {
    self.name.clone()
  }

  /// Its other names.
  pub fn aliases_of(&self) -> &[Arc<str>] {
    &self.aliases
  }

  /// Its other names, which it is registered by too: `rs` for Rust.
  pub fn aliases(mut self, aliases: &[&str]) -> Self {
    self.aliases.extend(aliases.iter().map(|alias| Arc::from(*alias)));
    self
  }

  pub fn mode(mut self, mode: CodeMode) -> Self {
    self.mode = mode;
    self
  }

  /// Words that are tokens named `name`: its keywords (`token::KEYWORD`), the words that change the flow
  /// (`token::KEYWORD_CONTROL`), its constants (`token::CONSTANT_LANGUAGE`), its own types
  /// (`token::TYPE_BUILTIN`). A word told twice has the last name.
  pub fn words(mut self, name: impl Into<TokenName>, words: &[&str]) -> Self {
    let name = name.into();
    for word in words {
      let word = if self.any_case {
        word.to_lowercase()
      } else {
        (*word).to_owned()
      };
      self.words.insert(word.into_boxed_str(), name);
    }
    self
  }

  /// Whether its words are its words in any case (SQL's `SELECT` and `select`). Set before the words.
  pub fn words_any_case(mut self, any_case: bool) -> Self {
    self.any_case = any_case;
    self
  }

  /// What starts a comment to the end of its line: `//`, `#`, `--`.
  pub fn line_comments(mut self, marks: &[&str]) -> Self {
    for mark in marks {
      self.line_comments.push((Arc::from(*mark), token::COMMENT.into()));
    }
    self
      .line_comments
      .sort_by_key(|(mark, _)| std::cmp::Reverse(mark.len()));
    self
  }

  /// What starts a comment to the end of its line that documents what follows: `///`, `//!`.
  pub fn doc_comments(mut self, marks: &[&str]) -> Self {
    for mark in marks {
      self.line_comments.push((Arc::from(*mark), token::COMMENT_DOC.into()));
    }
    self
      .line_comments
      .sort_by_key(|(mark, _)| std::cmp::Reverse(mark.len()));
    self
  }

  /// Whether a line comment starts only at the start of a word (a shell's `#`, which `a#b` does not start).
  pub fn comment_after_space(mut self, after_space: bool) -> Self {
    self.comment_after_space = after_space;
    self
  }

  /// A comment from `open` to `close`, over lines, and whether one inside it is closed before it is.
  pub fn block_comment(mut self, open: &str, close: &str, nests: bool) -> Self {
    self.block_comments.push(BlockComment {
      open: Arc::from(open),
      close: Arc::from(close),
      nests,
      name: token::COMMENT.into(),
    });
    self
      .block_comments
      .sort_by_key(|comment| std::cmp::Reverse(comment.open.len()));
    self
  }

  /// A comment from `open` to `close` that documents what follows: `/**`.
  pub fn doc_block_comment(mut self, open: &str, close: &str) -> Self {
    self.block_comments.push(BlockComment {
      open: Arc::from(open),
      close: Arc::from(close),
      nests: false,
      name: token::COMMENT_DOC.into(),
    });
    self
      .block_comments
      .sort_by_key(|comment| std::cmp::Reverse(comment.open.len()));
    self
  }

  /// Its forms of strings, in the order they are tried: the longer open marks first (`"""` before `"`).
  pub fn strings(mut self, forms: Vec<StringForm>) -> Self {
    self.strings.extend(forms);
    self
  }

  /// Whether the prefixes of its strings are matched in any case (Python's `R"…"` and `r"…"`).
  pub fn prefixes_any_case(mut self, any_case: bool) -> Self {
    self.prefixes_any_case = any_case;
    self
  }

  /// Rust's characters and lifetimes: `'a'` and `'\n'` are strings, `'a` and `'static` labels.
  pub fn characters(mut self, characters: bool) -> Self {
    self.characters = characters;
    self
  }

  pub fn numbers(mut self, numbers: NumberForms) -> Self {
    self.numbers = numbers;
    self
  }

  /// Whether a name that starts with a capital is a type (`App`), and one in capitals alone a constant
  /// (`MAX_LEN`).
  pub fn capitals_are_types(mut self, capitals: bool) -> Self {
    self.capitals_are_types = capitals;
    self
  }

  /// The mark between the parts of a path, a name before which is a namespace: Rust's `::`.
  pub fn path(mut self, separator: &str) -> Self {
    self.path = Some(Arc::from(separator));
    self
  }

  /// Whether a name right before `(` is a function.
  pub fn calls(mut self, calls: bool) -> Self {
    self.calls = calls;
    self
  }

  /// Whether a name right before `!` is a macro of Rust, the `!` with it.
  pub fn macros(mut self, macros: bool) -> Self {
    self.macros = macros;
    self
  }

  /// Words the name after which is declared as `name`: `fn` declares a function, `struct` a type.
  pub fn declares(mut self, name: impl Into<TokenName>, words: &[&str]) -> Self {
    let name = name.into();
    for word in words {
      self.declares.insert((*word).into(), name);
    }
    self
  }

  /// What starts an attribute or a decorator, with the name after it: `#[`, `#![`, `@`.
  pub fn attributes(mut self, marks: &[&str]) -> Self {
    self.attributes.extend(marks.iter().map(|mark| Arc::from(*mark)));
    self.attributes.sort_by_key(|mark| std::cmp::Reverse(mark.len()));
    self
  }

  /// Whether `@` and a name is a keyword (CSS's `@media`).
  pub fn at_words(mut self, at_words: bool) -> Self {
    self.at_words = at_words;
    self
  }

  /// Whether `$` and a name, `${…}`, `$1` or `$?` is a variable (a shell's).
  pub fn variables(mut self, variables: bool) -> Self {
    self.variables = variables;
    self
  }

  /// The characters a name has besides letters and digits (`_` unless told; `-` in CSS, `$` in JavaScript), and
  /// those it may start with.
  pub fn name_chars(mut self, inside: &str, start: &str) -> Self {
    self.name_chars = Arc::from(inside);
    self.name_starts = Arc::from(start);
    self
  }

  /// Where its keys are: a name or a string followed by its mark is a [`token::PROPERTY`].
  pub fn keys(mut self, keys: KeyRule) -> Self {
    self.keys = keys;
    self
  }

  /// Whether a line that starts with `[` names a section: `[dependencies]`, `[[bin]]`, its name a
  /// [`token::NAMESPACE`].
  pub fn sections(mut self, sections: bool) -> Self {
    self.sections = sections;
    self
  }

  /// Whether `/…/` where a value may start is a regular expression (JavaScript's).
  pub fn regexes(mut self, regexes: bool) -> Self {
    self.regexes = regexes;
    self
  }

  /// Whether the first word of a command is a function: at the start of a line, and after `|`, `&&`, `;` or `(`
  /// (a shell's).
  pub fn commands(mut self, commands: bool) -> Self {
    self.commands = commands;
    self
  }

  /// The words a command follows, as it follows `|`: a shell's `then`, `do`, `else`.
  pub fn commands_after(mut self, words: &[&str]) -> Self {
    self.commands_after.extend(words.iter().map(|word| Box::from(*word)));
    self
  }

  /// Whether names outside a declaration's value are a selector's (CSS): an element a [`token::TAG`], `.class`
  /// and `#id` an [`token::ATTRIBUTE`], `:hover` a [`token::KEYWORD`].
  pub fn selectors(mut self, selectors: bool) -> Self {
    self.selectors = selectors;
    self
  }

  /// Whether a line that ends in `|` or `>` after a key starts a block of text, the lines more indented than the
  /// key's (YAML's).
  pub fn block_scalars(mut self, block_scalars: bool) -> Self {
    self.block_scalars = block_scalars;
    self
  }

  /// The characters that are operators; every other mark is punctuation, and `.` is plain.
  pub fn operators(mut self, operators: &str) -> Self {
    self.operators = Arc::from(operators);
    self
  }

  /// The tokens of `code`, line by line from its first.
  pub fn tokens(&self, code: &str) -> Vec<CodeToken> {
    let mut tokens = Vec::new();
    let mut state = ScanState::default();
    let mut start = 0;
    for line in code.split('\n') {
      self.scan_line(line, start, &mut state, &mut tokens);
      start += line.len() + 1;
    }
    tokens
  }

  /// The tokens of one line, in bytes of the line, from the state the line starts in: `state` is the next line's
  /// after it. A long text is tokenized from any line whose state is known ([`Self::states`]).
  pub fn line_tokens(&self, line: &str, state: &mut ScanState) -> Vec<CodeToken> {
    let mut tokens = Vec::new();
    self.scan_line(line.trim_end_matches('\n'), 0, state, &mut tokens);
    tokens
  }

  /// The state each line of `code` starts in: inside a comment or a string that an earlier line opened, or not.
  pub fn states(&self, code: &str) -> Vec<ScanState> {
    let mut states = Vec::new();
    let mut state = ScanState::default();
    let mut scratch = Vec::new();
    for line in code.split('\n') {
      states.push(state.clone());
      scratch.clear();
      self.scan_line(line, 0, &mut state, &mut scratch);
    }
    states
  }
}

impl Highlighter for CodeLanguage {
  fn tokens(&self, code: &str) -> Vec<CodeToken> {
    CodeLanguage::tokens(self, code)
  }
}
