//! The list mode of window tabs ([`WindowTabsOverflow::List`]): the tabs that do not fit leave the strip, and the
//! More button at the end of those that stay opens the menu of them.

use std::sync::atomic::Ordering;

use super::{Place, WindowTabsMenu, WindowTabsOverflow, WindowTabsPart, WindowTabsView};
use crate::{
  app::{
    ctx::Placement,
    events::{KeyboardEvent, MouseEvent},
  },
  components::Row,
  layout::{Alignment, layout_kind::ShrinkLimit},
  node::{Element, FocusGroup},
  ui::{
    icons,
    menu::{Menu, MenuItem},
    recipe::Name,
    shared::{glyph, pressable, tab_stop},
    tooltip::Tooltip,
  },
};

impl WindowTabsView {
  /// Whether `place` may leave the strip: a tab of mounted tabs in the list mode, but the selected one.
  pub(super) fn leaves(&self, place: Place) -> bool {
    let Place::Tab(index) = place else {
      return false;
    };
    self.overflow == WindowTabsOverflow::List && self.kept.revision.is_some() && !self.tabs[index].selected
  }

  /// The tab at `index`, `element`, with the separator before it, as one that leaves the strip whole where the
  /// list has no room for it, the last first. The layout says it left through the tab's ref, which renders the
  /// tabs again with it in the More button's menu before the frame is painted.
  pub(super) fn leaving(&self, index: usize, separator: Option<Element>, element: Element) -> Element {
    let kept = self.kept.clone();
    self.kept.tab_ref(index).observe_rect_before_paint(move |rect| {
      let gone = rect.width <= 0.0 && rect.height <= 0.0;
      let none = {
        let mut dropped = kept.dropped.lock();
        if dropped.len() <= index {
          dropped.resize(index + 1, false);
        }
        if dropped[index] == gone {
          return;
        }
        dropped[index] = gone;
        !dropped.contains(&true)
      };
      // With every tab back in the strip the More button is gone, and its menu with it.
      if none && kept.open.get_untracked() {
        kept.open.set(false);
      }
      kept.moved();
    });
    // The tab and the separator before it are as far apart as two tabs are.
    Row::new()
      .align_items(Alignment::Center)
      .spacing(self.list_gap())
      .flex_shrink(1.0)
      .shrink_limit(ShrinkLimit::Drop)
      .with_children(separator)
      .child(element)
      .into()
  }

  /// How many tabs left the strip for the More button's menu.
  pub fn hidden_count(&self) -> usize {
    self.hidden.iter().filter(|hidden| **hidden).count()
  }

  /// The More button of mounted tabs in the list mode, with its menu: laid out only while a tab has left the
  /// strip, after the tabs that stay. It shows how many left and a chevron, which points up while its menu is open,
  /// and is a stop of Tab of its own after the tabs (not one their arrows move to), called by the strip's
  /// `more_label` and told how many it holds to what reads the tree; its tooltip says how many, with the keys of the
  /// [`list_shortcut`](super::WindowTabs::list_shortcut). Its menu lists the tabs that left, in their order, or every
  /// tab ([`WindowTabsMenu::All`]); choosing one selects it, which brings it back into the strip, and the focus goes
  /// to it.
  pub fn more(&self) -> Option<Element> {
    if self.overflow != WindowTabsOverflow::List || self.kept.revision.is_none() {
      return None;
    }
    let count = self.hidden_count();
    let open = self.kept.open.get();
    let locale = crate::ui::current_locale();
    let label = self.part(WindowTabsPart::MoreLabel);
    let all = self.menu == WindowTabsMenu::All;
    let mut button = tab_stop(
      pressable(&self.part(WindowTabsPart::More), self.disabled),
      self.tab_stop,
    )
    .align_items(Alignment::Center)
    .ref_element(self.kept.anchor.clone())
    .describe("a11y_name", self.words.more.clone())
    .describe("a11y_description", locale.count(&locale.navigation.hidden_tabs, count))
    .describe("a11y_haspopup", if all { "dialog" } else { "menu" })
    .describe("a11y_expanded", if open { "true" } else { "false" })
    // The title bar around the strip drags the window from a press: not from a press on the button.
    .on_mouse_down(|event: MouseEvent| event.stop_propagation())
    .child(label.text(&count.to_string()).nowrap().part(&label))
    .child(glyph(
      &self.part(WindowTabsPart::MoreIcon),
      Name::new(if open { icons::CHEVRON_UP } else { icons::CHEVRON_DOWN }),
      false,
    ))
    .on_click({
      let toggle = self.toggle_menu();
      move |_| toggle()
    });
    if let Some(id) = &self.id {
      button = button.id(format!("{id}-more"));
    }
    let mut tooltip = Tooltip::new(&locale.count(&locale.navigation.hidden_tabs, count))
      .placement(Placement::Bottom)
      .disabled(open);
    if self.list_shortcut {
      tooltip = tooltip.shortcut(&list_keys());
    }
    let menu = if all { self.all_tabs() } else { self.hidden_menu() };
    Some(
      Row::new()
        .align_items(Alignment::Center)
        .overflow_visible()
        // What is beside the tabs is no tab: a stop of Tab of its own, and not one the arrows move to.
        .focus_group(FocusGroup::none())
        .drop_marker(true)
        // Known by its key: a tab closed before it does not hand it the node of a tab that left the strip, which
        // the last layout dropped and which would refuse the focus its menu takes.
        .key("more")
        .child(tooltip.declared(button))
        .child(menu)
        .into(),
    )
  }

  /// What opens the More button's menu, or closes it. The list of every tab opens with its search empty, the focus
  /// on its search field where it has one, else on the row of the selected tab.
  pub(super) fn toggle_menu(&self) -> impl Fn() + Send + Sync + 'static {
    let kept = self.kept.clone();
    let focus = (self.menu == WindowTabsMenu::All).then(|| self.menu_entry());
    move || {
      let opening = !kept.open.get_untracked();
      if opening && let Some(focus) = focus {
        kept.query.set(String::new());
        kept.focus_menu(focus);
      }
      kept.open.set(opening);
    }
  }

  /// The handler of Ctrl+Shift+A (Cmd+Shift+A on macOS), where the strip has the list shortcut: it opens the More
  /// button's menu, or closes it. The menu of the hidden tabs opens only while a tab is hidden.
  pub(super) fn list_keys(&self) -> Option<impl Fn(KeyboardEvent) + Send + Sync + 'static> {
    if !self.list_shortcut || self.disabled || self.overflow != WindowTabsOverflow::List {
      return None;
    }
    let toggle = self.toggle_menu();
    let (open, can_open) = (
      self.kept.open.clone(),
      self.menu == WindowTabsMenu::All || self.hidden_count() > 0,
    );
    Some(move |event: KeyboardEvent| {
      let mac = cfg!(target_os = "macos");
      let held = if mac {
        event.meta && !event.ctrl
      } else {
        event.ctrl && !event.meta
      };
      if !held || !event.shift || event.alt || event.composing || !event.key.eq_ignore_ascii_case("a") {
        return;
      }
      if !can_open && !open.get_untracked() {
        return;
      }
      event.prevent_default();
      event.stop_propagation();
      toggle();
    })
  }

  /// The menu of the tabs that left the strip, in their order.
  fn hidden_menu(&self) -> Element {
    let mut menu = Menu::new(self.kept.open.clone(), self.kept.anchor.clone())
      .rules(&self.owner.lend("menu"))
      .placement(Placement::BottomEnd)
      .label(&self.words.more);
    for (index, entry) in self.tabs.iter().enumerate() {
      if !self.hidden[index] {
        continue;
      }
      let (select, refocus, follows) = (entry.select.clone(), self.kept.refocus.clone(), self.tab_stop);
      let mut item = MenuItem::new(&entry.label)
        .disabled(self.disabled || entry.disabled)
        .on_select(move || {
          // The tab chosen comes back into the strip as the selected one, and takes the focus the menu had.
          refocus.store(follows, Ordering::Relaxed);
          select();
        });
      if let Some(icon) = entry.icon {
        item = item.icon(icon);
      }
      if let Some(id) = &entry.id {
        item = item.id(format!("{id}-item"));
      }
      menu = menu.item(item);
    }
    menu.into()
  }
}

/// The keys of the list shortcut as the platform writes them: ⇧⌘A on a Mac, Ctrl+Shift+A elsewhere, in the
/// locale's names of the keys.
pub(super) fn list_keys() -> String {
  if cfg!(target_os = "macos") {
    "⇧⌘A".to_owned()
  } else {
    let keys = &crate::ui::current_locale().keys;
    format!("{}+{}+A", keys.ctrl, keys.shift)
  }
}
