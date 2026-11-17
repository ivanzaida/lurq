//! The tokenizer of each language: lurq's own for the languages it knows, an app's where it registered one.

use std::{
  collections::HashMap,
  fmt,
  sync::{
    Arc, LazyLock,
    atomic::{AtomicU64, Ordering},
  },
};

use parking_lot::RwLock;

use super::{CodeToken, cache};
use crate::core::Signal;

/// What tells the tokens of a language's code apart.
///
/// Any `Fn(&str) -> Vec<CodeToken>` is one, and so is a [`CodeLanguage`](super::CodeLanguage), the table lurq's own
/// are written in. The tokens it gives are in bytes of the code; they need not cover it (what none covers is plain),
/// and those that are empty, out of the code, off a character's boundary or over another are left out.
pub trait Tokenizer: Send + Sync + 'static {
  /// The tokens of `code`.
  fn tokens(&self, code: &str) -> Vec<CodeToken>;
}

impl<F> Tokenizer for F
where
  F: Fn(&str) -> Vec<CodeToken> + Send + Sync + 'static,
{
  fn tokens(&self, code: &str) -> Vec<CodeToken> {
    self(code)
  }
}

static NEXT_TOKENIZER: AtomicU64 = AtomicU64::new(1);

/// A tokenizer as lurq keeps it: shared, and told from every other one, so that what it gave for a code is kept
/// and not asked again.
///
/// Made once and cloned, it is one tokenizer: a block given a clone in every render takes its tokens from what
/// the first render kept. Made anew in each render, each is a new one and the code is tokenized again.
#[derive(Clone)]
pub struct CodeTokenizer {
  id: u64,
  tokenizer: Arc<dyn Tokenizer>,
}

impl CodeTokenizer {
  pub fn new(tokenizer: impl Tokenizer) -> Self {
    Self {
      id: NEXT_TOKENIZER.fetch_add(1, Ordering::Relaxed),
      tokenizer: Arc::new(tokenizer),
    }
  }

  /// The tokens of `code`, in order and none over another: what this tokenizer gave for the same code before,
  /// where it is still kept.
  pub fn tokens(&self, code: &str) -> Arc<[CodeToken]> {
    cache::tokens(self.id, code, || super::normalized(code, self.tokenizer.tokens(code)))
  }

  /// What tells this tokenizer from every other: the same for its clones.
  pub fn id(&self) -> u64 {
    self.id
  }
}

impl<H: Tokenizer> From<H> for CodeTokenizer {
  fn from(tokenizer: H) -> Self {
    Self::new(tokenizer)
  }
}

impl PartialEq for CodeTokenizer {
  fn eq(&self, other: &Self) -> bool {
    self.id == other.id
  }
}

impl fmt::Debug for CodeTokenizer {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_tuple("CodeTokenizer").field(&self.id).finish()
  }
}

impl crate::app::component::DevtoolsInspectable for CodeTokenizer {}

/// A language with a tokenizer: the name it was registered by, and its other names.
struct Language {
  name: Arc<str>,
  aliases: Vec<Arc<str>>,
  tokenizer: CodeTokenizer,
}

#[derive(Default)]
struct Registry {
  /// Each language by the name it was registered by.
  languages: HashMap<Arc<str>, Language>,
  /// The language each name and alias is, in lowercase.
  names: HashMap<Arc<str>, Arc<str>>,
  /// What a render that read a name's tokenizer follows: set when the tokenizer of that name changes. A name
  /// with none has one too, so that what showed its code plain shows it in colors once one is registered.
  changes: HashMap<Arc<str>, Signal<u64>>,
}

static REGISTRY: LazyLock<RwLock<Registry>> = LazyLock::new(|| {
  let mut registry = Registry::default();
  for language in super::languages::all() {
    let name = language.name();
    let aliases: Vec<Arc<str>> = language.aliases_of().to_vec();
    registry.insert(&name, &aliases, CodeTokenizer::new(language));
  }
  RwLock::new(registry)
});

/// The name a language is looked up by: `Rust` and `rust ignore` are `rust`, `{.toml}` is `toml`.
fn key(language: &str) -> Arc<str> {
  let language = language.trim().trim_start_matches(['{', '.']);
  let end = language
    .find(|ch: char| ch.is_whitespace() || matches!(ch, ',' | '}' | '{'))
    .unwrap_or(language.len());
  Arc::from(language[..end].to_lowercase())
}

impl Registry {
  /// Gives `name`'s language `tokenizer`, with `aliases` as other names of it. The names whose tokenizer
  /// changed, with the one it had.
  fn insert(
    &mut self,
    name: &str,
    aliases: &[Arc<str>],
    tokenizer: CodeTokenizer,
  ) -> (Vec<Arc<str>>, Option<CodeTokenizer>) {
    let name = key(name);
    let own = self.names.get(&name).cloned().unwrap_or_else(|| name.clone());
    let language = self.languages.entry(own.clone()).or_insert_with(|| Language {
      name: own.clone(),
      aliases: Vec::new(),
      tokenizer: tokenizer.clone(),
    });
    let replaced = (language.tokenizer != tokenizer).then(|| std::mem::replace(&mut language.tokenizer, tokenizer));
    let mut moved = Vec::new();
    for alias in std::iter::once(&name).chain(aliases) {
      let alias = key(alias);
      if alias != language.name && !language.aliases.contains(&alias) {
        language.aliases.push(alias.clone());
      }
      moved.push(alias);
    }
    let mut changed: Vec<Arc<str>> = std::iter::once(language.name.clone())
      .chain(language.aliases.iter().cloned())
      .collect();
    // An alias taken from another language leaves it.
    for alias in moved {
      if let Some(other) = self.names.insert(alias.clone(), own.clone())
        && other != own
        && let Some(other) = self.languages.get_mut(&other)
      {
        other.aliases.retain(|held| *held != alias);
      }
      if !changed.contains(&alias) {
        changed.push(alias);
      }
    }
    self.names.insert(own.clone(), own);
    (changed, replaced)
  }
}

/// Bumps what the renders that read `names` follow, once the registry is let go.
fn tell(names: Vec<Arc<str>>) {
  let signals: Vec<Signal<u64>> = {
    let registry = REGISTRY.read();
    names
      .iter()
      .filter_map(|name| registry.changes.get(name).cloned())
      .collect()
  };
  for signal in signals {
    signal.update(|version| *version = version.wrapping_add(1));
  }
}

/// Registers `tokenizer` for `language` (`"sql"`, case aside): the code of that language, in every code block,
/// Markdown document, inline code and code field of the app, is told apart by it from now on, and what shows it now is
/// drawn again. A language that has a tokenizer, lurq's own among them, has this one in its place, under each of
/// its names (`rs` is Rust's too).
///
/// The registry is the process's: an app has one. `App::register_tokenizer` says the same.
pub fn register_tokenizer(language: &str, tokenizer: impl Into<CodeTokenizer>) {
  register_tokenizer_with_aliases(language, &[], tokenizer);
}

/// As [`register_tokenizer`], and `aliases` are other names of the language (`"psql"`, `"postgres"`): a name
/// that was another language's is this one's now.
pub fn register_tokenizer_with_aliases(language: &str, aliases: &[&str], tokenizer: impl Into<CodeTokenizer>) {
  let aliases: Vec<Arc<str>> = aliases.iter().map(|alias| Arc::from(*alias)).collect();
  let (changed, replaced) = REGISTRY.write().insert(language, &aliases, tokenizer.into());
  if let Some(replaced) = replaced {
    cache::forget(replaced.id());
  }
  tell(changed);
}

/// Takes the tokenizer of `language` away: its code is plain text from now on. Whether it had one.
pub fn remove_tokenizer(language: &str) -> bool {
  let name = key(language);
  let removed = {
    let mut registry = REGISTRY.write();
    let Some(own) = registry.names.get(&name).cloned() else {
      return false;
    };
    let Some(language) = registry.languages.remove(&own) else {
      return false;
    };
    registry.names.retain(|_, language| *language != own);
    language
  };
  cache::forget(removed.tokenizer.id());
  tell(std::iter::once(removed.name).chain(removed.aliases).collect());
  true
}

/// The tokenizer of `language`, by its name or an alias, case aside; `None` for one with none, whose code is
/// plain. Read in a render, it renders that again when the language's tokenizer changes.
pub fn tokenizer(language: &str) -> Option<CodeTokenizer> {
  let name = key(language);
  let (found, change) = {
    let registry = REGISTRY.read();
    let found = registry
      .names
      .get(&name)
      .and_then(|own| registry.languages.get(own))
      .map(|language| language.tokenizer.clone());
    (found, registry.changes.get(&name).cloned())
  };
  let change = change.unwrap_or_else(|| {
    REGISTRY
      .write()
      .changes
      .entry(name)
      .or_insert_with(|| Signal::new(0))
      .clone()
  });
  let _ = change.get();
  found
}

/// Whether `language` has a tokenizer: lurq's own or the app's.
pub fn knows(language: &str) -> bool {
  let name = key(language);
  REGISTRY.read().names.contains_key(&name)
}

/// Every language with a tokenizer, by the name it was registered by, with its other names, in the order of
/// their names.
pub fn languages() -> Vec<(Arc<str>, Vec<Arc<str>>)> {
  let registry = REGISTRY.read();
  let mut all: Vec<_> = registry
    .languages
    .values()
    .map(|language| (language.name.clone(), language.aliases.clone()))
    .collect();
  all.sort_by(|a, b| a.0.cmp(&b.0));
  all
}

/// The tokens of `code` in `language`, by the language's tokenizer; none for a language with none. What the
/// tokenizer gave for the same code before is not asked again.
pub fn tokens(code: &str, language: &str) -> Arc<[CodeToken]> {
  match tokenizer(language) {
    Some(tokenizer) if !code.is_empty() => tokenizer.tokens(code),
    _ => Arc::from([]),
  }
}
