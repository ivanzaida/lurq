//! A token input over a list: what the app holds is the items, and what is typed and is no item yet is apart.
//!
//! The engine's input edits one text, so the list is kept as one: each item with a mark after it, then the draft.
//! The mark is a narrow space no keyboard types, which is the room between two tokens and where a row wraps. An
//! edit is settled before it is made: what it leaves of the items, what it makes of the draft, and whether the text
//! it would give is the one the list is kept as.

use std::{
  collections::HashMap,
  ops::Range,
  sync::{Arc, LazyLock, Mutex, Weak},
};

use crate::{components::TextInputController, core::Signal};

/// What ends an item in the text the list is kept as: a four-per-em space.
pub(crate) const END: char = '\u{2005}';

/// What a person changed a list of tokens to.
pub(crate) type ListHandler = Arc<dyn Fn(&[String]) + Send + Sync>;
/// What a person typed that is no token yet.
pub(crate) type DraftHandler = Arc<dyn Fn(&str) + Send + Sync>;

/// The text a list is kept as: every item with its mark, then the draft.
pub(crate) fn encode(items: &[String], draft: &str) -> String {
  let mut text =
    String::with_capacity(items.iter().map(|item| item.len() + END.len_utf8()).sum::<usize>() + draft.len());
  for item in items {
    text.push_str(item);
    text.push(END);
  }
  text.push_str(draft);
  text
}

/// The items and the draft of a text: what stands before each mark, and what stands after the last.
pub(crate) fn parse(text: &str) -> (Vec<String>, String) {
  let (items, draft) = match text.rfind(END) {
    Some(last) => (&text[..last], &text[last + END.len_utf8()..]),
    None => ("", text),
  };
  let items = items
    .split(END)
    .filter(|item| !item.is_empty())
    .map(str::to_owned)
    .collect();
  (items, draft.to_owned())
}

/// Where each item stands in a text, in its order.
pub(crate) fn ranges(text: &str) -> Vec<Range<usize>> {
  let mut ranges = Vec::new();
  let mut start = 0;
  for (at, character) in text.char_indices() {
    if character == END {
      if at > start {
        ranges.push(start..at);
      }
      start = at + character.len_utf8();
    }
  }
  ranges
}

/// What makes an item of what is typed.
#[derive(Clone)]
pub(crate) struct Rules {
  /// The characters that end an item as Enter does.
  pub(crate) separators: Vec<char>,
  /// Whether an item the list already has is not added again.
  pub(crate) unique: bool,
}

impl Rules {
  fn separates(&self, character: char) -> bool {
    character == '\n'
      || character == '\r'
      || character == '\t'
      || character == END
      || self.separators.contains(&character)
  }

  fn add(&self, items: &mut Vec<String>, item: &str) {
    let item = item.trim();
    if !item.is_empty() && !(self.unique && items.iter().any(|held| held == item)) {
      items.push(item.to_owned());
    }
  }

  /// The items and the draft `typed` makes after `items`: every part that has a separator after it is an item,
  /// and the last is the draft.
  fn typed(&self, mut items: Vec<String>, typed: &str) -> (Vec<String>, String) {
    let mut parts: Vec<&str> = typed.split(|character| self.separates(character)).collect();
    let draft = parts.pop().unwrap_or_default();
    let made = parts.len();
    for part in parts {
      self.add(&mut items, part);
    }
    // What follows a separator starts a new item: the room typed after a comma is not its start.
    let draft = if made > 0 { draft.trim_start() } else { draft };
    (items, draft.to_owned())
  }

  /// The list with its draft made an item: what Enter does.
  pub(crate) fn committed(&self, text: &str) -> Option<(Vec<String>, String)> {
    let (mut items, draft) = parse(text);
    if draft.trim().is_empty() {
      return None;
    }
    self.add(&mut items, &draft);
    Some((items, String::new()))
  }
}

/// What an edit comes to.
#[derive(Debug, PartialEq)]
pub(crate) struct Settled {
  pub(crate) items: Vec<String>,
  pub(crate) draft: String,
  /// The text the input is given in place of the edit's, where the edit's is not the one the list is kept as.
  pub(crate) text: Option<String>,
  /// The token the edit selects in place of changing anything: where it stands in the text before the edit.
  pub(crate) select: Option<Range<usize>>,
}

/// The one change that turns `old` into `new`: where it starts, what it takes out and what it puts in.
fn change<'a>(old: &'a str, new: &'a str) -> (usize, &'a str, &'a str) {
  let start = old
    .char_indices()
    .zip(new.chars())
    .find(|((_, before), after)| before != after)
    .map_or(old.len().min(new.len()), |((at, _), _)| at);
  let (old_rest, new_rest) = (&old[start..], &new[start..]);
  let same_end: usize = old_rest
    .chars()
    .rev()
    .zip(new_rest.chars().rev())
    .take_while(|(before, after)| before == after)
    .map(|(character, _)| character.len_utf8())
    .sum();
  (
    start,
    &old_rest[..old_rest.len() - same_end],
    &new_rest[..new_rest.len() - same_end],
  )
}

/// What the edit that turns `old` into `new` comes to. The tokens are whole things: an edit takes some away, or
/// leaves them. A Backspace over the room after a token selects that token, as one right after it does, so that
/// the next takes it away. An edit that would change a token's text is the caret among the tokens: the tokens it
/// covers go, and what it types goes to the draft.
pub(crate) fn settle(old: &str, new: &str, rules: &Rules) -> Settled {
  let (old_items, old_draft) = parse(old);
  let (kept, typed) = parse(new);
  let (at, taken, put) = change(old, new);
  if put.is_empty() && taken.chars().eq([END]) {
    let select = ranges(old).into_iter().find(|range| range.end == at);
    return Settled {
      items: old_items,
      draft: old_draft,
      text: Some(old.to_owned()),
      select,
    };
  }
  let (items, draft) = if kept.len() <= old_items.len() && kept.iter().any(|item| !old_items.contains(item)) {
    let covered = at..at + taken.len();
    let items = ranges(old)
      .into_iter()
      .zip(old_items)
      .filter(|(range, _)| {
        taken.is_empty() || covered.end <= range.start || covered.start >= range.end + END.len_utf8()
      })
      .map(|(_, item)| item)
      .collect();
    rules.typed(items, &format!("{old_draft}{put}"))
  } else {
    rules.typed(kept, &typed)
  };
  let text = encode(&items, &draft);
  Settled {
    items,
    draft,
    text: (text != new).then_some(text),
    select: None,
  }
}

/// What a token input over a list keeps between its builds: the text the engine's input edits, and the controller
/// that puts the caret at its end. One for each list a token input is bound to, while one is in the tree: `alive` is
/// held by the input last built, and when nothing holds it the input left the tree.
struct Kept {
  text: Signal<String>,
  controller: TextInputController,
  alive: Weak<()>,
}

static KEPT: LazyLock<Mutex<HashMap<usize, Kept>>> = LazyLock::new(Default::default);

/// The text and the controller of the token input bound to the signal `binding`, and what keeps them.
pub(crate) fn kept(binding: usize) -> (Signal<String>, TextInputController, Arc<()>) {
  let mut all = KEPT.lock().unwrap();
  all.retain(|_, kept| kept.alive.strong_count() > 0);
  if let Some(kept) = all.get(&binding)
    && let Some(alive) = kept.alive.upgrade()
  {
    return (kept.text.clone(), kept.controller.clone(), alive);
  }
  let alive = Arc::new(());
  let (text, controller) = (Signal::new(String::new()), TextInputController::new());
  all.insert(
    binding,
    Kept {
      text: text.clone(),
      controller: controller.clone(),
      alive: Arc::downgrade(&alive),
    },
  );
  (text, controller, alive)
}

#[cfg(test)]
mod tests {
  use super::*;

  fn rules(separators: &[char]) -> Rules {
    Rules {
      separators: separators.to_vec(),
      unique: false,
    }
  }

  fn list(items: &[&str], draft: &str) -> String {
    encode(&items.iter().map(|item| (*item).to_owned()).collect::<Vec<_>>(), draft)
  }

  #[test]
  fn a_text_is_its_items_and_its_draft() {
    let text = list(&["rust", "New York"], "wg");
    assert_eq!(
      parse(&text),
      (vec!["rust".to_owned(), "New York".to_owned()], "wg".to_owned())
    );
    assert_eq!(ranges(&text), vec![0..4, 4 + END.len_utf8()..12 + END.len_utf8()]);
  }

  #[test]
  fn typing_changes_the_draft_and_no_item() {
    let old = list(&["rust"], "wg");
    let new = list(&["rust"], "wgp");
    assert_eq!(
      settle(&old, &new, &rules(&[','])),
      Settled {
        items: vec!["rust".to_owned()],
        draft: "wgp".to_owned(),
        text: None,
        select: None,
      }
    );
  }

  #[test]
  fn a_separator_makes_the_draft_an_item() {
    let old = list(&["rust"], "wgpu");
    let new = format!("{old},");
    let settled = settle(&old, &new, &rules(&[',']));
    assert_eq!(settled.items, ["rust", "wgpu"]);
    assert_eq!(settled.draft, "");
    assert_eq!(settled.text, Some(list(&["rust", "wgpu"], "")));
  }

  #[test]
  fn a_pasted_list_is_its_items_and_the_last_is_the_draft() {
    let old = list(&["rust"], "");
    let new = format!("{old}wgpu, vello,  taffy");
    let settled = settle(&old, &new, &rules(&[',']));
    assert_eq!(settled.items, ["rust", "wgpu", "vello"]);
    assert_eq!(settled.draft, "taffy");
  }

  #[test]
  fn a_token_taken_away_leaves_the_others() {
    let old = list(&["rust", "wgpu", "vello"], "ta");
    // The engine deletes the token's text and leaves its mark.
    let new = old.replacen("wgpu", "", 1);
    let settled = settle(&old, &new, &rules(&[',']));
    assert_eq!(settled.items, ["rust", "vello"]);
    assert_eq!(settled.draft, "ta");
    assert_eq!(settled.text, Some(list(&["rust", "vello"], "ta")));
  }

  #[test]
  fn what_is_typed_among_the_tokens_goes_to_the_draft() {
    let old = list(&["rust", "wgpu"], "");
    let at = "rust".len() + END.len_utf8();
    let new = format!("{}x{}", &old[..at], &old[at..]);
    let settled = settle(&old, &new, &rules(&[',']));
    assert_eq!(settled.items, ["rust", "wgpu"]);
    assert_eq!(settled.draft, "x");
  }

  #[test]
  fn a_backspace_over_the_room_after_a_token_selects_the_token() {
    let old = list(&["rust", "wgpu"], "ve");
    let at = "rust".len();
    let new = format!("{}{}", &old[..at], &old[at + END.len_utf8()..]);
    let settled = settle(&old, &new, &rules(&[',']));
    assert_eq!(settled.items, ["rust", "wgpu"]);
    assert_eq!(settled.draft, "ve");
    assert_eq!(settled.text.as_deref(), Some(old.as_str()));
    assert_eq!(settled.select, Some(0..4));

    // The room after the last token, with the draft right behind it.
    let at = old.rfind(END).unwrap();
    let new = format!("{}{}", &old[..at], &old[at + END.len_utf8()..]);
    let settled = settle(&old, &new, &rules(&[',']));
    assert_eq!(settled.items, ["rust", "wgpu"]);
    assert_eq!(settled.select, Some(at - 4..at));
  }

  #[test]
  fn typing_over_a_selected_token_takes_it_away_and_types_in_the_draft() {
    let old = list(&["rust", "wgpu"], "");
    let new = old.replacen("wgpu", "x", 1);
    let settled = settle(&old, &new, &rules(&[',']));
    assert_eq!(settled.items, ["rust"]);
    assert_eq!(settled.draft, "x");
  }

  #[test]
  fn an_item_the_list_has_is_not_added_again_where_items_are_unique() {
    let rules = Rules {
      separators: vec![','],
      unique: true,
    };
    let old = list(&["rust"], "rust");
    let settled = settle(&old, &format!("{old},"), &rules);
    assert_eq!(settled.items, ["rust"]);
    assert_eq!(settled.draft, "");
  }

  #[test]
  fn enter_makes_the_draft_an_item() {
    let rules = rules(&[]);
    assert_eq!(
      rules.committed(&list(&["rust"], " New York ")),
      Some((vec!["rust".to_owned(), "New York".to_owned()], String::new()))
    );
    assert_eq!(rules.committed(&list(&["rust"], "  ")), None);
  }
}
