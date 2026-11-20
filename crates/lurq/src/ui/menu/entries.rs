//! What a menu holds, walked as one list: a section that scrolls is a place among the entries, and what it holds
//! follows it, so that the places of an item's page and of the keys run through the sections.

use super::MenuEntry;

/// Every entry of `entries` in order, a section that scrolls followed by what it holds.
pub(super) fn flat(entries: &[MenuEntry]) -> Vec<&MenuEntry> {
  let mut all = Vec::new();
  walk(entries, &mut all);
  all
}

fn walk<'a>(entries: &'a [MenuEntry], all: &mut Vec<&'a MenuEntry>) {
  for entry in entries {
    all.push(entry);
    if let MenuEntry::Scroll { entries, .. } = entry {
      walk(entries, all);
    }
  }
}

/// The entry at `place` of [`flat`]'s walk of `entries`, taken out of them.
pub(super) fn take(entries: Vec<MenuEntry>, place: usize) -> Option<MenuEntry> {
  let mut left = place;
  take_from(entries, &mut left)
}

fn take_from(entries: Vec<MenuEntry>, left: &mut usize) -> Option<MenuEntry> {
  for entry in entries {
    if *left == 0 {
      return Some(entry);
    }
    *left -= 1;
    if let MenuEntry::Scroll { entries, .. } = entry
      && let Some(found) = take_from(entries, left)
    {
      return Some(found);
    }
  }
  None
}

/// `entries` with the last item of [`flat`]'s walk that can be chosen made the one the focus goes to as the menu
/// opens: a menubar's menu opened with Up.
pub(crate) fn autofocus_last(entries: Vec<MenuEntry>) -> Vec<MenuEntry> {
  let last = flat(&entries)
    .iter()
    .rposition(|entry| matches!(entry, MenuEntry::Item(item) if !item.is_disabled()));
  match last {
    Some(last) => {
      let mut place = 0;
      mark(entries, last, &mut place)
    }
    None => entries,
  }
}

fn mark(entries: Vec<MenuEntry>, last: usize, place: &mut usize) -> Vec<MenuEntry> {
  entries
    .into_iter()
    .map(|entry| {
      let at = *place;
      *place += 1;
      match entry {
        MenuEntry::Item(item) if at == last => MenuEntry::Item(item.autofocus(true)),
        MenuEntry::Scroll { entries, max_height } => MenuEntry::Scroll {
          entries: mark(entries, last, place),
          max_height,
        },
        entry => entry,
      }
    })
    .collect()
}
