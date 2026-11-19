//! The ids a component given one gives the nodes of its parts.
//!
//! A component given `.id(x)` keeps `x` on its root and gives the node of every other part `x-<part>`, the part's
//! name as its `names!` table writes it: `x-label`, `x-dot`. A part on several nodes of one build is `x-<part>`,
//! `x-<part>-2`, `x-<part>-3`, in the order they are built. A node that has an id keeps it. A component built inside
//! another, and given no id of its own, takes the id of the part of the other it stands for (`x-close`), and so
//! prefixes its own parts with it. A component with no id gives its parts none. A part's name is written with
//! hyphens for its underscores (`x-trigger-icon`), and a part of which there are several that have a key of their own
//! (a day of a calendar, an item with a value) is told apart by it (`x-day-2026-11-20`), not by its place.

use std::{collections::HashMap, fmt, sync::Arc};

use parking_lot::Mutex;

use super::recipe::Name;

/// The id the parts of one component, or of one item of it, are named after, and how many nodes of each part have
/// been named in this build.
#[derive(Clone)]
pub(crate) struct Owner {
  id: Arc<str>,
  /// The part the id itself is on.
  root: Name,
  /// Whether the component puts the id on its root itself: one the app gave it. One it was lent is put on the
  /// first node of its root by the recipe.
  own: bool,
  /// How many nodes of each part have been named, shared by every clone of the recipe of one build.
  counts: Arc<Mutex<HashMap<Name, usize>>>,
}

impl Owner {
  pub(crate) fn new(id: Arc<str>, root: Name, own: bool) -> Self {
    Self {
      id,
      root,
      own,
      counts: Arc::default(),
    }
  }

  /// The same owner, its root `root`: an item whose parts are named after the component's id.
  pub(crate) fn rooted(&self, root: Name) -> Self {
    Self { root, ..self.clone() }
  }

  /// The id of the next node of `part`: `None` for the root, whose id is the owner's own, but the first node of a
  /// root the owner was lent.
  pub(crate) fn next(&self, part: Name) -> Option<Arc<str>> {
    let mut counts = self.counts.lock();
    let count = counts.entry(part).or_default();
    *count += 1;
    if part == self.root {
      return (!self.own && *count == 1).then(|| self.id.clone());
    }
    Some(match *count {
      1 => self.named(part, None),
      n => self.named(part, Some(&n.to_string())),
    })
  }

  /// The id of the node of `part` told apart from the others of it by `key`: `x-<part>-<key>`, whatever the order
  /// it is built in. `None` for the root.
  pub(crate) fn keyed(&self, part: Name, key: &str) -> Option<Arc<str>> {
    (part != self.root).then(|| self.named(part, Some(key)))
  }

  /// An owner of its own for an item of the component, `x-<part>-<key>`, which the item's node of `part` is given.
  pub(crate) fn item(&self, part: Name, key: &str) -> Self {
    Self::new(self.named(part, Some(key)), part, false)
  }

  /// `x-<part>`, or `x-<part>-<suffix>`, the underscores of the part's name made hyphens: `x-trigger-icon`.
  fn named(&self, part: Name, suffix: Option<&str>) -> Arc<str> {
    let part = part.as_str().replace('_', "-");
    match suffix {
      Some(suffix) => Arc::from(format!("{}-{part}-{suffix}", self.id)),
      None => Arc::from(format!("{}-{part}", self.id)),
    }
  }
}

impl fmt::Debug for Owner {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("Owner")
      .field("id", &self.id)
      .field("root", &self.root)
      .finish()
  }
}

/// A part of one component lent to another built inside it: what [`ResolvedRecipe::rules_for`] and
/// [`ResolvedRecipe::lend`] carry, so that the other, given no id of its own, is named after the part.
///
/// [`ResolvedRecipe::rules_for`]: super::ResolvedRecipe::rules_for
/// [`ResolvedRecipe::lend`]: super::ResolvedRecipe::lend
#[derive(Clone, Debug)]
pub(crate) struct Lent {
  owner: Owner,
  part: Name,
  /// Whether the part is lent to the other's root: the part the other stands for, over one lent to a part inside it.
  to_root: bool,
}

impl Lent {
  pub(crate) fn new(owner: Owner, part: Name, to_root: bool) -> Self {
    Self { owner, part, to_root }
  }

  /// Of two lent parts, the one the other component stands for: the first one lent to its root, or the first one.
  pub(crate) fn over(first: Option<Lent>, second: Option<&Lent>) -> Option<Lent> {
    match (first, second) {
      (Some(first), Some(second)) if !first.to_root && second.to_root => Some(second.clone()),
      (None, second) => second.cloned(),
      (first, _) => first,
    }
  }

  /// The owner the component lent the part is named by, rooted at `root`: the part's id, or, for the root of the
  /// component that lent it, that component's own id and its count, since what stands for a root is that root.
  pub(crate) fn owner(&self, root: Name) -> Option<Owner> {
    if self.part == self.owner.root {
      return Some(self.owner.rooted(root));
    }
    self.owner.next(self.part).map(|id| Owner::new(id, root, false))
  }
}
