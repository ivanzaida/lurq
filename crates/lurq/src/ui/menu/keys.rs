//! The keys of a menu that leaves the focus in the app's field: which item is the active one, and what the field's
//! keys do to it.

use std::{
  hash::{DefaultHasher, Hash, Hasher},
  sync::Arc,
};

use parking_lot::Mutex;

use crate::{
  app::events::{KeyboardEvent, MouseEvent},
  core::{ElementRef, Signal},
  ui::{shared::command_held_key, toolbar::keyboard_click},
};

/// What an item does when it is chosen, or asked for aside.
type Action = Arc<dyn Fn() + Send + Sync>;

/// What an item asked for aside does with the press that asked.
type Aside = Arc<dyn Fn(&MouseEvent) + Send + Sync>;

/// What the keys can do with one item of the menu: choose it, and ask for it aside.
#[derive(Clone, Default)]
struct Row {
  choose: Option<Action>,
  aside: Option<Aside>,
}

#[derive(Default)]
struct Held {
  /// Whether the menu was open as it was last built.
  open: bool,
  /// What the items were as the menu was last built: the active one goes back to the first that can be chosen when
  /// it changes.
  seen: Option<u64>,
  /// Which items can be chosen, in order.
  choosable: Vec<bool>,
  /// The item the keys are on, among the menu's items.
  active: Option<usize>,
  rows: Vec<Row>,
  /// Closes the menu as a person dismissing it does.
  dismiss: Option<Action>,
}

/// What a menu that leaves the focus in the app's field keeps from one render to the next, kept by the app (made once,
/// as a [`DateMemory`](super::super::DateMemory) is): the field the focus stays in, and the item the keys are on.
/// Given to [`Menu::keeps_focus`](super::Menu::keeps_focus).
///
/// ```
/// use lurq::{
///   components::TextInput,
///   core::{ElementRef, Signal},
///   ui::{Menu, MenuItem, MenuKeys},
/// };
///
/// # fn build(field: ElementRef, open: Signal<bool>, keys: &MenuKeys) {
/// // `keys` is `MenuKeys::new(field.clone())`, made once with the component.
/// let menu = Menu::new(open, field.clone())
///   .keeps_focus(keys)
///   .item(MenuItem::new("Summarize").on_select(|| println!("summarize")))
///   .item(MenuItem::new("Explain").on_select(|| println!("explain")));
/// // A field whose own Enter sends: the menu's keys first, then its own.
/// let forward = keys.clone();
/// let input = TextInput::new(Signal::new(String::new())).ref_element(field).on_key_down(move |event| {
///   if forward.key_down(&event) {
///     return;
///   }
///   // The field's own keys.
/// });
/// # }
/// ```
#[derive(Clone)]
pub struct MenuKeys {
  field: ElementRef,
  held: Arc<Mutex<Held>>,
  /// Changed where the active item moves: what renders the menu again.
  tick: Signal<u64>,
}

impl MenuKeys {
  /// The keys of a menu whose focus stays in `field`: while it has the focus, the menu reads its keys.
  pub fn new(field: impl Into<ElementRef>) -> Self {
    Self {
      field: field.into(),
      held: Arc::default(),
      tick: Signal::new(0),
    }
  }

  /// The element the focus stays in.
  pub fn field(&self) -> &ElementRef {
    &self.field
  }

  /// The item the keys are on, by its place among the menu's items (not its separators, headings or elements), as
  /// the menu was last built: `None` while it is closed or has no item that can be chosen.
  pub fn active(&self) -> Option<usize> {
    self.held.lock().active
  }

  /// What a key pressed in the field does to the menu, where the menu is open: Up and Down move the active item round
  /// the items that can be chosen, Enter chooses it, Ctrl+Enter (Cmd+Enter on macOS) asks for it aside where it has
  /// [`on_open_aside`](super::MenuItem::on_open_aside), and Escape closes the menu. Says whether it took the key, which
  /// it then keeps from the field (`prevent_default`); a key a handler before it took is left alone. A field whose own
  /// keys come first forwards each key here before them; the menu's panel reads them too, and takes a key once.
  pub fn key_down(&self, event: &KeyboardEvent) -> bool {
    if event.default_prevented() || event.composing || event.alt {
      return false;
    }
    let command = command_held_key(event);
    let plain = !event.ctrl && !event.meta;
    let taken = match event.key.as_str() {
      "ArrowDown" | "ArrowUp" if plain && !event.shift => self.step(event.key == "ArrowDown"),
      "Enter" if command => self.aside(event),
      "Enter" if plain && !event.shift => self.choose(),
      "Escape" if plain => self.dismiss(),
      _ => false,
    };
    if taken {
      event.prevent_default();
    }
    taken
  }

  /// Moves the active item to the next that can be chosen, round the list.
  fn step(&self, forward: bool) -> bool {
    let mut held = self.held.lock();
    let count = held.choosable.len();
    if !held.open || !held.choosable.contains(&true) {
      return false;
    }
    let at = held.active.unwrap_or(if forward { count - 1 } else { 0 });
    let next = (1..=count)
      .map(|step| {
        if forward {
          (at + step) % count
        } else {
          (at + count - step) % count
        }
      })
      .find(|index| held.choosable[*index]);
    held.active = next;
    drop(held);
    self.redraw();
    true
  }

  /// Chooses the active item.
  fn choose(&self) -> bool {
    let choose = {
      let held = self.held.lock();
      let Some(at) = held.active.filter(|_| held.open) else {
        return false;
      };
      held.rows.get(at).and_then(|row| row.choose.clone())
    };
    let Some(choose) = choose else {
      return false;
    };
    choose();
    true
  }

  /// Asks for the active item aside, where it can be.
  fn aside(&self, event: &KeyboardEvent) -> bool {
    let aside = {
      let held = self.held.lock();
      let Some(at) = held.active.filter(|_| held.open) else {
        return false;
      };
      held.rows.get(at).and_then(|row| row.aside.clone())
    };
    let Some(aside) = aside else {
      return false;
    };
    let mut press = keyboard_click();
    (press.shift, press.ctrl, press.meta) = (event.shift, event.ctrl, event.meta);
    aside(&press);
    true
  }

  /// Closes the menu as a person dismissing it does.
  fn dismiss(&self) -> bool {
    let dismiss = {
      let held = self.held.lock();
      held.dismiss.clone().filter(|_| held.open)
    };
    let Some(dismiss) = dismiss else {
      return false;
    };
    dismiss();
    true
  }

  fn redraw(&self) {
    self.tick.update(|tick| *tick = tick.wrapping_add(1));
  }

  /// Makes the item at `index` the active one, where it can be chosen: the pointer came onto it.
  pub(crate) fn point(&self, index: usize) {
    let mut held = self.held.lock();
    if held.active == Some(index) || !held.choosable.get(index).copied().unwrap_or(false) {
      return;
    }
    held.active = Some(index);
    drop(held);
    self.redraw();
  }

  /// Starts a build of the menu, `open` or not, whose items are `items` (each one's label and whether it can be
  /// chosen), closed by a person with `dismiss`: says which item is the active one. It is the first that can be chosen
  /// as the menu opens and whenever what it holds changes; else it stays where the keys put it. Read so that a move of
  /// it renders the menu again.
  pub(crate) fn begin(&self, open: bool, items: &[(&str, bool)], dismiss: Action) -> Option<usize> {
    self.tick.get();
    let mut held = self.held.lock();
    held.rows = vec![Row::default(); items.len()];
    held.dismiss = Some(dismiss);
    if !open {
      (held.open, held.seen, held.active) = (false, None, None);
      held.choosable.clear();
      return None;
    }
    let mut hasher = DefaultHasher::new();
    items.hash(&mut hasher);
    let seen = hasher.finish();
    held.choosable = items.iter().map(|(_, choosable)| *choosable).collect();
    let first = held.choosable.iter().position(|choosable| *choosable);
    let kept = held
      .active
      .filter(|at| held.open && held.seen == Some(seen) && held.choosable.get(*at).copied().unwrap_or(false));
    held.active = kept.or(first);
    (held.open, held.seen) = (true, Some(seen));
    held.active
  }

  /// What choosing the item at `index`, and asking for it aside, do: the item tells them as it is built.
  pub(crate) fn register(&self, index: usize, choose: Option<Action>, aside: Option<Aside>) {
    if let Some(row) = self.held.lock().rows.get_mut(index) {
      *row = Row { choose, aside };
    }
  }
}

/// What a menu that keeps the focus in a field gives each of its items: the keys, its place among the items, and
/// whether it is the active one.
#[derive(Clone)]
pub(crate) struct ItemKeys {
  pub(crate) keys: MenuKeys,
  pub(crate) index: usize,
  pub(crate) active: bool,
}
