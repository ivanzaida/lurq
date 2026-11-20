//! The list of every open tab that the More button of the list mode opens ([`WindowTabsMenu::All`]): a search field
//! at its head once it holds more than eight tabs, then a row of each tab that matches, in the strip's order, with a
//! mark on the selected tab and on each tab not in the strip and a close button at its end. While it has the focus its
//! keys are its own (`keys_first`).

use std::sync::{
  Arc,
  atomic::{AtomicBool, Ordering},
};

use super::{LIST_INSET, MenuFocus, WindowTabEntry, WindowTabsPart, WindowTabsView};
use crate::{
  app::{
    ctx::Placement,
    events::{KeyboardEvent, MouseButton, MouseEvent},
  },
  components::{self, Column, Popup, Row, ScrollVertical},
  layout::{Alignment, layout_kind::Justify},
  node::{Element, FocusGroup, dimension::Dimension},
  ui::{
    icons,
    recipe::{Name, ResolvedPart},
    search_field::SearchField,
    shared::{glyph, one_line, pressable},
  },
};

/// A list of more than this many tabs leads with its search field.
const SEARCH_ABOVE: usize = 8;

/// How far under what it stands by the list is.
const LIST_OFFSET: f32 = 4.0;

/// The least height of the list, in a window too low for more.
const LEAST_HEIGHT: f32 = 120.0;

/// Whether a list of `count` tabs leads with its search field: it holds more than eight.
pub(super) fn searched(count: usize) -> bool {
  count > SEARCH_ABOVE
}

/// Whether `tab` matches the search `query`: its label, what its row says in place of it, or the quiet text after
/// it holds the query, in any case.
fn matches(query: &str, tab: &WindowTabEntry) -> bool {
  let query = query.trim().to_lowercase();
  let holds = |text: &str| text.to_lowercase().contains(&query);
  query.is_empty()
    || holds(&tab.label)
    || tab.list_label.as_deref().is_some_and(holds)
    || tab.list_detail.as_deref().is_some_and(holds)
}

/// The row the list keeps in view while it is open with `query` in its search field: that of the selected tab where
/// it matches, else the first that does.
pub(super) fn kept_in_view(tabs: &[WindowTabEntry], query: &str) -> Option<usize> {
  let mut listed = tabs.iter().enumerate().filter(|(_, tab)| matches(query, tab));
  let first = listed.clone().next().map(|(index, _)| index);
  listed.find(|(_, tab)| tab.selected).map(|(index, _)| index).or(first)
}

/// Whether the list takes `event` from the rest of the window while it has the focus: Ctrl+W (Cmd+W), and every key
/// pressed without Ctrl, Alt or Cmd.
fn the_lists(event: &KeyboardEvent) -> bool {
  closes(event) || !(event.ctrl || event.alt || event.meta)
}

/// Whether `event` is the window's close shortcut: Cmd+W on macOS, Ctrl+W elsewhere.
fn closes(event: &KeyboardEvent) -> bool {
  let held = if cfg!(target_os = "macos") {
    event.meta && !event.ctrl
  } else {
    event.ctrl && !event.meta
  };
  held && !event.shift && !event.alt && event.key.eq_ignore_ascii_case("w")
}

impl WindowTabsView {
  /// Whether the list leads with its search field: it holds more than eight tabs.
  fn searched(&self) -> bool {
    searched(self.tabs.len())
  }

  /// Where the focus goes as the list of every tab opens: its search field where it has one, else the row of the
  /// selected tab, or the first.
  pub(super) fn menu_entry(&self) -> MenuFocus {
    if self.searched() {
      return MenuFocus::Search;
    }
    MenuFocus::Item(self.tabs.iter().position(|tab| tab.selected).unwrap_or(0))
  }

  /// The style of a part of the row of the tab at `index`: in the states of that tab, its parts named after its id.
  fn item_part(&self, index: usize, part: WindowTabsPart) -> ResolvedPart {
    let tab = &self.tabs[index];
    let recipe = self.recipe(tab.selected, false, false, self.disabled || tab.disabled, false);
    let recipe = match (&tab.id, &tab.key) {
      (None, Some(key)) => recipe.keyed_item(key, WindowTabsPart::Item),
      (id, _) => recipe.item(id.as_ref(), WindowTabsPart::Item),
    };
    recipe.part(part)
  }

  /// The tabs the list shows, by index: those whose label matches what the search field holds.
  fn listed(&self, query: &str) -> Vec<usize> {
    (0..self.tabs.len())
      .filter(|index| matches(query, &self.tabs[*index]))
      .collect()
  }

  /// What the list hangs from, and where it stands by it: the More button, its start lined up with the button's; or,
  /// with no tab hidden and the button not laid out, the list of tabs, its end at theirs.
  fn list_anchor(&self) -> (crate::core::ElementRef, Placement) {
    if self.hidden_count() == 0 {
      (self.kept.list.clone(), Placement::BottomEnd)
    } else {
      (self.kept.anchor.clone(), Placement::BottomStart)
    }
  }

  /// The list of every open tab, under the More button (under the tabs where none is hidden and the button is not
  /// laid out), or under what the app told it to stand clear of, while it is open. The focus goes into it as it opens and back to the button as it closes; Escape empties the search field,
  /// and closes the list where it is empty; a press outside closes it. While it has the focus it hears keys before
  /// the rest of the window, which does not hear those it takes.
  pub(super) fn all_tabs(&self) -> Element {
    let query = self.kept.query.get();
    let listed: Arc<[usize]> = self.listed(&query).into();
    let mut panel = Column::new()
      .align_items(Alignment::Stretch)
      .part(&self.part(WindowTabsPart::Panel))
      .describe("a11y_role", "dialog")
      .describe("a11y_name", crate::ui::current_locale().navigation.open_tabs.clone());
    if let Some(id) = &self.id {
      panel = panel.id(format!("{id}-panel"));
    }
    // Escape empties the search wherever the focus is in the list, before it closes the list.
    let held = self.kept.query.clone();
    panel = panel.on_key_down(move |event: KeyboardEvent| {
      if event.focus_within && event.key == "Escape" && !event.composing && !held.get_untracked().is_empty() {
        event.prevent_default();
        held.set(String::new());
      }
    });
    if self.searched() {
      panel = panel.child(self.search(&listed));
    }
    panel = if listed.is_empty() {
      panel.child(self.no_match(&query))
    } else {
      panel.child(self.items(&listed))
    };
    if let Some(height) = self.list_height() {
      panel = panel.max_height(Dimension::Px(height));
    }
    // The list's keys are heard first, and those it takes go no further: after what is in it heard them, while the
    // focus is in it.
    let within = Arc::new(AtomicBool::new(false));
    let learned = within.clone();
    let content = Column::new()
      .keys_first(true)
      .on_key_down(move |event: KeyboardEvent| learned.store(event.focus_within, Ordering::Relaxed))
      .child(panel)
      .child(Row::new().on_key_down(move |event: KeyboardEvent| {
        if within.load(Ordering::Relaxed) && !event.composing && the_lists(&event) {
          event.stop_propagation();
        }
      }));
    let inset = self.list_inset();
    let told = self.open_told();
    let (anchor, placement) = self.list_anchor();
    let mut popup = Popup::new(anchor, content)
      .open(self.kept.open.clone())
      .placement(placement)
      .offset(0.0, LIST_OFFSET)
      .keep_clear(inset, inset, inset, inset)
      .takes_focus(true)
      .on_dismiss(move || told(false));
    if let Some(frame) = &self.list.clear_of {
      popup = popup.clear_of(frame.clone());
    }
    popup.into()
  }

  /// How far the list stays inside the window's edges.
  fn list_inset(&self) -> f32 {
    self.list.inset.unwrap_or(LIST_INSET)
  }

  /// The most the list is high: from its top, under what it stands by, to the window's bottom less its inset. None
  /// before the window's height is known.
  fn list_height(&self) -> Option<f32> {
    let window = (*self.kept.window_height.lock())?;
    let anchor = self.list_anchor().0;
    let frame = self
      .list
      .clear_of
      .as_ref()
      .filter(|frame| frame.is_attached())
      .unwrap_or(&anchor);
    let top = if frame.is_attached() {
      frame.y() + frame.height() + LIST_OFFSET
    } else {
      0.0
    };
    Some((window - top - self.list_inset()).max(LEAST_HEIGHT))
  }

  /// The search field at the head of the list: it says how many tabs it searches, Enter chooses the first that
  /// matches, and Down goes to it.
  fn search(&self, listed: &Arc<[usize]>) -> Element {
    let locale = crate::ui::current_locale();
    let placeholder = locale.count(&locale.navigation.search_tabs, self.tabs.len());
    let first = listed.first().copied();
    let choose = first.map(|index| self.chooser(index));
    let mut field = SearchField::new(self.kept.query.clone())
      .placeholder(&placeholder)
      .label(&placeholder)
      .full_width(true)
      .ref_element(self.kept.search.clone())
      .on_submit(move |_| {
        if let Some(choose) = &choose {
          choose();
        }
      });
    if self.list_shortcut {
      // While it is empty, the field says the keys that open and close the list.
      field = field.shortcut(&super::list::list_keys());
    }
    if let Some(id) = &self.id {
      field = field.id(format!("{id}-search"));
    }
    let kept = self.kept.clone();
    Row::new()
      .part(&self.part(WindowTabsPart::Search))
      // A key reaches every node: Down goes to the first row only while the focus is in the field.
      .on_key_down(move |event: KeyboardEvent| {
        let plain = !(event.ctrl || event.alt || event.meta || event.shift);
        if event.focus_within
          && plain
          && !event.composing
          && event.key == "ArrowDown"
          && let Some(first) = first
        {
          event.prevent_default();
          kept.focus_menu(MenuFocus::Item(first));
        }
      })
      .child(field)
      .into()
  }

  /// What the list says where no tab matches the search: "No open tab matches “…”".
  fn no_match(&self, query: &str) -> Element {
    let part = self.part(WindowTabsPart::Empty);
    let words = crate::core::fill(
      &crate::ui::current_locale().navigation.no_tab_matches,
      &[("query", query.trim())],
    );
    Row::new().part(&part).child(part.text(&words)).into()
  }

  /// The rows of `listed`, one stop of Tab whose Up and Down move from row to row, scrolled where the window is
  /// too low for them all.
  fn items(&self, listed: &Arc<[usize]>) -> Element {
    let mut rows = Column::new()
      .align_items(Alignment::Stretch)
      .part(&self.part(WindowTabsPart::Items))
      .focus_group(FocusGroup::vertical().one_stop(true))
      .describe("a11y_role", "listbox")
      .describe("a11y_name", crate::ui::current_locale().navigation.open_tabs.clone());
    for (at, index) in listed.iter().enumerate() {
      rows = rows.child(self.item(*index, at, listed));
    }
    // As high as the rows, and giving way to the list's most height, past which they scroll.
    ScrollVertical::new(rows).flex_shrink(1.0).min_height(0.0).into()
  }

  /// What selects the tab at `index` from the list, and closes the list: the tab comes back into the strip, and
  /// takes the focus where the strip is a stop of Tab.
  fn chooser(&self, index: usize) -> impl Fn() + Send + Sync + 'static {
    let (select, kept, follows) = (self.tabs[index].select.clone(), self.kept.clone(), self.tab_stop);
    let told = self.open_told();
    move || {
      kept.refocus.store(follows, Ordering::Relaxed);
      kept.open.set(false);
      told(false);
      select();
    }
  }

  /// The row of the tab at `index`, the `at`th of `listed`: its icon, its label (or what the row says in place of it)
  /// and the quiet text after it, a mark where it is not in the strip, the app's elements, a mark where it is the
  /// selected tab, and its close button. A press, Enter and Space choose it; the middle
  /// button and Ctrl+W (Cmd+W on macOS) close its tab, and the keys go on to the next row; Up on the first row goes
  /// back to the search field.
  fn item(&self, index: usize, at: usize, listed: &Arc<[usize]>) -> Element {
    let entry = &self.tabs[index];
    let part = |part: WindowTabsPart| self.item_part(index, part);
    let disabled = self.disabled || entry.disabled;
    let choose = self.chooser(index);
    let label = entry.list_label.as_ref().unwrap_or(&entry.label);
    let mut row = pressable(&part(WindowTabsPart::Item), disabled)
      .justify(Justify::Start)
      .align_items(Alignment::Center)
      .interaction_group(true)
      .focus_entry(entry.selected)
      .ref_element(self.kept.item_ref(index))
      .describe("a11y_role", "option")
      .describe("a11y_name", label.clone())
      .describe("a11y_selected", if entry.selected { "true" } else { "false" })
      .on_click(move |_| choose());
    if self.hidden[index] {
      row = row.describe(
        "a11y_description",
        crate::ui::current_locale().navigation.not_in_strip.clone(),
      );
    }
    if !disabled {
      row = row.on_key_down(self.item_keys(index, at, listed));
      if self.closable(index) {
        let close = entry.close.clone();
        row = row.on_mouse_click(MouseButton::Middle, move |event: MouseEvent| {
          event.stop_propagation();
          close();
        });
      }
    }
    if let Some(icon) = entry.icon {
      row = row.child(glyph(&part(WindowTabsPart::ItemIcon), icon, false));
    }
    // With a quiet text after it, the label is as wide as it is and gives way first; the text takes what it leaves.
    let (grow, basis) = if entry.list_detail.is_some() {
      (0.0, None)
    } else {
      (1.0, Some(0.0))
    };
    row = row.child(
      Row::new()
        .flex_full(grow, 1.0, basis)
        .min_width(0.0)
        .child(one_line(&part(WindowTabsPart::ItemLabel), label)),
    );
    if let Some(detail) = &entry.list_detail {
      row = row.child(
        Row::new()
          .flex_full(1.0, 1.0, Some(0.0))
          .min_width(0.0)
          .child(one_line(&part(WindowTabsPart::ItemDetail), detail)),
      );
    }
    let mark = |what: WindowTabsPart, name: &str, suffix: &str| {
      let mut mark = Row::new().child(glyph(&part(what), Name::new(name), false));
      if let Some(id) = &entry.id {
        mark = mark.id(format!("{id}-item-{suffix}"));
      }
      mark
    };
    if self.hidden[index] {
      row = row.child(mark(WindowTabsPart::ItemHidden, icons::EYE_OFF, "hidden"));
    }
    if !entry.list_trailing.is_empty() {
      row = row.child(
        Row::new()
          .part(&part(WindowTabsPart::ItemMarks))
          .align_items(Alignment::Center)
          .flex_shrink(0.0)
          .with_children(entry.list_trailing.iter().cloned()),
      );
    }
    if entry.selected {
      row = row.child(mark(WindowTabsPart::ItemCurrent, icons::CHECK, "current"));
    }
    row = row.child(self.item_close(index));
    if let Some(id) = &entry.id {
      row = row.id(format!("{id}-item"));
    }
    // A row is told from the others by its tab's key or id, so that the focus on it does not pass to the row of
    // the next tab when a tab before it is closed.
    if let Some(key) = entry.key.as_ref().or(entry.id.as_ref()) {
      row = row.key(key.clone());
    }
    row.into()
  }

  /// The close button at the end of the row of the tab at `index`, its x shown while the row is under the pointer
  /// or has the focus: for the pointer, called "Close" and the tab's label. A row whose tab cannot be closed keeps
  /// the room of the button empty.
  fn item_close(&self, index: usize) -> Element {
    let entry = &self.tabs[index];
    let part = self.item_part(index, WindowTabsPart::ItemClose);
    if !self.closable(index) {
      return Row::new().part(&part.clone().unnamed()).into();
    }
    let close = entry.close.clone();
    let mut button = components::Button::empty()
      .focusable(false)
      .part(&part)
      .justify(Justify::Center)
      .align_items(Alignment::Center)
      .describe("a11y_name", format!("{} {}", self.words.close, entry.label))
      // A press of it is its own: it does not choose the row.
      .on_mouse_down(|event: MouseEvent| event.stop_propagation())
      .on_click(move |event: MouseEvent| {
        event.stop_propagation();
        close();
      })
      .child(glyph(
        &self.item_part(index, WindowTabsPart::ItemCloseIcon),
        Name::new(icons::CLOSE),
        false,
      ));
    if let Some(id) = &entry.id {
      button = button.id(format!("{id}-item-close"));
    }
    button.into()
  }

  /// What the keys do on the row of the tab at `index`, the `at`th of `listed`, while it has the focus: Ctrl+W
  /// (Cmd+W) closes its tab and the focus goes on to the next row, or the one before at the end; Up on the first
  /// row goes back to the search field.
  fn item_keys(&self, index: usize, at: usize, listed: &Arc<[usize]>) -> impl Fn(KeyboardEvent) + Send + Sync + 'static {
    let (kept, listed) = (self.kept.clone(), listed.clone());
    let close = self.closable(index).then(|| self.tabs[index].close.clone());
    let (searched, count) = (self.searched(), self.tabs.len());
    move |event: KeyboardEvent| {
      if !event.focus_within || event.composing {
        return;
      }
      if closes(&event) {
        // The window's Ctrl+W is the row's while the list is open, whether its tab closes or not.
        event.prevent_default();
        event.stop_propagation();
        let Some(close) = &close else {
          return;
        };
        close();
        // The tabs after the one closed move one place back.
        let next = listed
          .get(at + 1)
          .map(|next| next - 1)
          .or_else(|| at.checked_sub(1).and_then(|before| listed.get(before).copied()));
        if let Some(next) = next {
          kept.focus_menu_with(MenuFocus::Item(next), count - 1);
        }
        return;
      }
      let plain = !(event.ctrl || event.alt || event.meta || event.shift);
      if plain && at == 0 && searched && event.key == "ArrowUp" {
        event.prevent_default();
        kept.focus_menu(MenuFocus::Search);
      }
    }
  }
}
