//! A transfer: two lists side by side, and the items checked in one moved to the other by the buttons between them.

use std::sync::Arc;

use parking_lot::Mutex;

use super::{
  button::{Button, ButtonSize, ButtonState, ButtonVariant},
  checkbox::Checkbox,
  icons,
  listbox::{Listbox, ListboxPart},
  names,
  part_style::{FontFamily, Length, PartStyle},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::ChangeHandler,
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{BorderSize, ControlSize, Density, OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::{Column, Row, SelectOption},
  core::{ElementRef, Signal, SignalValue},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{Element, FocusGroup, border::Border, dimension::Dimension, node_kind::TextOverflow},
};

/// The parts of a transfer a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TransferPart {
  /// The two lists and the buttons between them: the room between them.
  Root,
  /// One of the two lists: its width, its fill, its edge and its corners.
  List,
  /// The row at the top of a list: its checkbox that checks all, its title and its count.
  Header,
  /// What a list is called, in its header.
  Title,
  /// How many of a list's items are checked, of how many, at the end of its header.
  Count,
  /// What holds a list's items: the room around them, and the most it is high, past which they scroll. What is laid
  /// over a listbox's box.
  Items,
  /// An item of a list: what is laid over an option of a listbox.
  Item,
  /// What an item says: what is laid over the label of a listbox's option.
  ItemLabel,
  /// What a list with no items says.
  Empty,
  /// The buttons between the lists, one under the other: the room between them.
  Actions,
}

names!(TransferPart {
  Root => "root",
  List => "list",
  Header => "header",
  Title => "title",
  Count => "count",
  Items => "items",
  Item => "item",
  ItemLabel => "item_label",
  Empty => "empty",
  Actions => "actions",
});

/// The states of a transfer a recipe styles, besides what the pointer and the keyboard do to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TransferState {
  /// It takes no input.
  Disabled,
  /// Its room is too narrow for the lists side by side: they are one over the other, the buttons in a row between
  /// them.
  Stacked,
}

names!(TransferState {
  Disabled => "disabled",
  Stacked => "stacked",
});

/// What builds a transfer's nodes: a theme's in place of the default one.
pub type TransferRenderer = Arc<dyn Fn(TransferView) -> Element + Send + Sync>;

type Write<T> = Arc<dyn Fn(Vec<T>) + Send + Sync>;
type CountText = Arc<dyn Fn(usize, usize) -> String + Send + Sync>;

/// How many items a list holds before a mounted transfer builds only those near its view.
const WINDOWED: usize = 200;

/// Two lists side by side, and the buttons between them that move the items checked in one to the other: the
/// features a build has, the columns a table shows, the members of a group.
///
/// ```
/// use lurq::{core::Signal, ui::Transfer};
///
/// # fn render(ctx: &mut lurq::app::ctx::Ctx, enabled: Signal<Vec<&'static str>>) {
/// let features = [("canvas", "canvas"), ("svg", "svg"), ("router", "router"), ("wgpu", "wgpu"), ("winit", "winit")];
/// // What is on the right is the signal's: mounted, so that it keeps what is checked and finds its own narrow form.
/// let transfer = Transfer::new(enabled, features)
///   .source_title("Available")
///   .target_title("Enabled")
///   .label("Features")
///   .mount(ctx);
/// // Held by the app, which is told each new value.
/// let held = Transfer::controlled(["wgpu"], features, |enabled| println!("{enabled:?}"));
/// # }
/// ```
///
/// Its items are a [`Listbox`]'s options ([`SelectOption`], or `(value, label)`): one may be disabled, have a
/// detail, a hint, an element before or after its label. What it holds is the values of the items on the right, the
/// target, in the order they came there: the app's signal ([`new`](Self::new)) or what the app says with a handler it
/// is told by ([`controlled`](Self::controlled)). The left list, the source, has the rest, in the order of the items.
///
/// Each list is a listbox of any number with [`checkboxes`](Listbox::checkboxes): a press of an item checks it or
/// unchecks it, a press with Shift checks the range from the last one checked, and the keys are a listbox's: one stop
/// of Tab, Up and Down, Home and End, the page keys, a letter, Space to check, Shift with the keys to check a range,
/// Ctrl+A to check all. The checkbox in a list's header checks every item of it that takes input, or none, and is
/// half checked while some are; its count says how many are checked of how many there are. The button with the
/// chevron to the right moves what is checked on the left to the end of the right, the one with the chevron to the
/// left moves it back, and each is disabled while its list has nothing checked. A disabled item is not checked and
/// does not move.
///
/// What is checked is kept where the transfer is [mounted](Self::mount), or by the app ([`checked`](Self::checked));
/// mounted, or converted into an element, which declares the same live component, it measures its room before it is
/// painted and lays the lists one over the other, the buttons in a row between them and their chevrons turned, where
/// they do not fit side by side, and a list of more than two hundred items builds only those near its view.
/// [`build`](Self::build) is the still form of itself, side by side whatever its room.
#[derive(Clone)]
pub struct Transfer<T: 'static = ()> {
  items: Vec<SelectOption<T>>,
  value: Vec<T>,
  write: Option<Write<T>>,
  on_change: Vec<ChangeHandler<[T]>>,
  checked: Option<(Vec<T>, Option<Write<T>>)>,
  on_checked_change: Vec<ChangeHandler<[T]>>,
  source_title: Option<Arc<str>>,
  target_title: Option<Arc<str>>,
  count_text: CountText,
  select_all_label: Option<Arc<str>>,
  add_label: Option<Arc<str>>,
  remove_label: Option<Arc<str>>,
  empty_text: Option<Arc<str>>,
  /// The words it was told to speak in, in place of the app's.
  locale: Option<crate::core::InputWords>,
  label: Option<Arc<str>>,
  disabled: bool,
  density: Option<Density>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl<T: Clone + PartialEq + Send + Sync + 'static> Transfer<T> {
  /// A transfer of `items` bound to `value`: the items of its values are on the right, and a move sets it.
  pub fn new(value: Signal<Vec<T>>, items: impl IntoIterator<Item = impl Into<SelectOption<T>>>) -> Self
  where
    Vec<T>: SignalValue,
  {
    let mut transfer = Self::of(items);
    transfer.value = value.get();
    transfer.write = Some(Arc::new(move |next| value.set(next)));
    transfer
  }

  /// A transfer of `items` whose value the app holds: the items of `value` are on the right, and a move tells
  /// `on_change` the whole value it is to be; the transfer shows what the app then gives.
  pub fn controlled(
    value: impl IntoIterator<Item = T>,
    items: impl IntoIterator<Item = impl Into<SelectOption<T>>>,
    on_change: impl Fn(&[T]) + Send + Sync + 'static,
  ) -> Self {
    let mut transfer = Self::of(items);
    transfer.value = value.into_iter().collect();
    transfer.on_change(on_change)
  }

  fn of(items: impl IntoIterator<Item = impl Into<SelectOption<T>>>) -> Self {
    Self {
      items: items.into_iter().map(Into::into).collect(),
      value: Vec::new(),
      write: None,
      on_change: Vec::new(),
      checked: None,
      on_checked_change: Vec::new(),
      source_title: None,
      target_title: None,
      // A count and not a word: the same in every language.
      count_text: Arc::new(|checked, total| format!("{checked}/{total}")),
      select_all_label: None,
      add_label: None,
      remove_label: None,
      empty_text: None,
      locale: None,
      label: None,
      disabled: false,
      density: None,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// Called with the whole new value, the items on the right, after a move, after the signal is set. Not called
  /// when the app sets the signal. A second handler is called after the first.
  pub fn on_change(mut self, handler: impl Fn(&[T]) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// What is checked, in either list, where the app keeps it: the values the signal has. Without it a mounted
  /// transfer keeps it itself. A move takes what it moved out of it.
  pub fn checked(mut self, checked: Signal<Vec<T>>) -> Self
  where
    Vec<T>: SignalValue,
  {
    self.checked = Some((checked.get(), Some(Arc::new(move |next| checked.set(next)))));
    self
  }

  /// What is checked, as the app holds it: `checked`, and `on_checked_change` is told what a press or a key asks it
  /// to be.
  pub fn checked_controlled(
    mut self,
    checked: impl IntoIterator<Item = T>,
    on_checked_change: impl Fn(&[T]) + Send + Sync + 'static,
  ) -> Self {
    self.checked = Some((checked.into_iter().collect(), None));
    self.on_checked_change(on_checked_change)
  }

  /// Called with the whole of what is checked, in both lists, after each press, key or move that changes it.
  pub fn on_checked_change(mut self, handler: impl Fn(&[T]) + Send + Sync + 'static) -> Self {
    self.on_checked_change.push(Arc::new(handler));
    self
  }

  /// What the left list is called, in its header and to what reads the tree: the locale's "Available" unless it
  /// is told otherwise.
  pub fn source_title(mut self, title: &str) -> Self {
    self.source_title = Some(Arc::from(title));
    self
  }

  /// What the right list is called: the locale's "Selected" unless it is told otherwise.
  pub fn target_title(mut self, title: &str) -> Self {
    self.target_title = Some(Arc::from(title));
    self
  }

  /// What a list's header says of how many of its items are checked, of how many: "1/3" unless it is told
  /// otherwise.
  pub fn count_text(mut self, text: impl Fn(usize, usize) -> String + Send + Sync + 'static) -> Self {
    self.count_text = Arc::new(text);
    self
  }

  /// What the checkbox in a list's header is called to what reads the tree: the locale's "Select all" unless it
  /// is told otherwise.
  pub fn select_all_label(mut self, label: &str) -> Self {
    self.select_all_label = Some(Arc::from(label));
    self
  }

  /// What the button that moves to the right is called: the locale's "Add" unless it is told otherwise.
  pub fn add_label(mut self, label: &str) -> Self {
    self.add_label = Some(Arc::from(label));
    self
  }

  /// What the button that moves back to the left is called: the locale's "Remove" unless it is told otherwise.
  pub fn remove_label(mut self, label: &str) -> Self {
    self.remove_label = Some(Arc::from(label));
    self
  }

  /// What a list with no items says: the locale's "No items" unless it is told otherwise.
  pub fn empty_text(mut self, text: &str) -> Self {
    self.empty_text = Some(Arc::from(text));
    self
  }

  /// The words this transfer speaks in, in place of the app's ([`App::set_locale`](crate::app::App::set_locale)):
  /// its lists' titles, the names of its checkboxes and buttons, and what an empty list says. A word told by itself
  /// (`source_title`, `add_label` and the others) stays what it was told.
  pub fn locale(mut self, locale: &crate::core::Locale) -> Self {
    self.locale = Some(locale.input.clone());
    self
  }

  /// What the transfer is called, to what reads the tree. It draws nothing.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// Whether nothing in the transfer takes input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// The density its items are at, whatever the theme is at: how high each is.
  pub fn density(mut self, density: Density) -> Self {
    self.density = Some(density);
    self
  }

  /// Puts the transfer in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this transfer over what the theme gives it: `style(TransferPart::List, |s| s.width(240.0))`.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this transfer.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the item Tab would stop on in each list as it is under the pointer, held down or focused from the
  /// keyboard, with nothing pointing at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the root. The left list's box has this id with `-source` after it, its items `-source-0` and on, its
  /// header's checkbox `-source-all`; the right list's the same with `-target`; the buttons `-add` and `-remove`.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the still form of the transfer in `theme`, with nothing kept but what the app gives it: side by side
  /// whatever its room, wide only, with nothing checked unless the app holds what is. Converting a transfer into an
  /// element gives the live one in place of this, which keeps what is checked and stacks in a narrow room.
  pub fn build(self, theme: &Theme) -> Element {
    let kept = Kept {
      checked: Arc::default(),
      revision: None,
      stacked: false,
      need: None,
      buttons: None,
    };
    let (view, lists) = self.view(theme, kept);
    self.finish(view, lists.map(|list| list.map(|(listbox, _)| listbox.build(theme))))
  }

  /// The transfer mounted where it is declared: it keeps what is checked, measures its room before it is painted
  /// and finds its narrow form there, and builds only the items near the view of a long list.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let key = self.key.clone();
    let props = TransferProps { transfer: self };
    match key {
      Some(key) => ctx.mount_keyed::<TransferHost<T>>(&key, props).key(key),
      None => ctx.mount::<TransferHost<T>>(props),
    }
  }

  /// The view with its lists in it, drawn by the theme's renderer or the default one.
  fn finish(&self, mut view: TransferView, lists: [Option<Element>; 2]) -> Element {
    let [source, target] = lists;
    view.source.list = source;
    view.target.list = target;
    let theme = view.theme().clone();
    match theme.renderer::<Transfer<T>>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }

  fn case(&self, stacked: bool) -> Case {
    Case {
      variant: None,
      size: None,
      density: self.density.map(|density| Name::new(density.as_str())),
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(TransferState::Disabled, self.disabled)
    .state(TransferState::Stacked, stacked)
  }

  /// How wide the lists and the buttons are side by side, as the recipe gives them: under it, they are stacked.
  fn need(&self, theme: &Theme) -> f32 {
    let recipe = theme.resolve::<Transfer<T>>(&self.case(false), Some(&self.rules));
    let px = |length: Option<Length>| super::select::px(length, theme).unwrap_or(0.0);
    let gap = super::select::spacing(recipe.part(TransferPart::Root).rest().gap, theme).unwrap_or(0.0);
    2.0 * px(recipe.part(TransferPart::List).rest().width) + 2.0 * gap + theme.control_size_value(ControlSize::Sm)
  }

  /// The transfer as a renderer sees it, but for its lists, which are given apart, to be built or mounted: each a
  /// listbox, and whether it is long enough to build only the items near its view.
  fn view(&self, theme: &Theme, kept: Kept<T>) -> (TransferView, [Option<(Listbox<T, [T]>, bool)>; 2]) {
    let recipe = theme.resolve::<Transfer<T>>(&self.case(kept.stacked), Some(&self.rules));
    // The words it was not told: its own locale's, else the app's, read where it is built.
    let words = match &self.locale {
      Some(words) => words.clone(),
      None => super::current_locale().input.clone(),
    };
    let told = |word: &Option<Arc<str>>, default: &Arc<str>| word.clone().unwrap_or_else(|| default.clone());
    let ids: Vec<&T> = self.items.iter().map(SelectOption::value).collect();
    // What is on the right: the app's values that are items, in the app's order.
    let target: Vec<T> = self.value.iter().filter(|value| ids.contains(value)).cloned().collect();
    let enabled = |value: &T| {
      !self.disabled
        && self
          .items
          .iter()
          .any(|item| item.value() == value && !item.is_disabled())
    };
    let checked: Vec<T> = match &self.checked {
      Some((checked, _)) => checked.clone(),
      None => kept.checked.lock().clone(),
    };
    // What is checked that can be: an item no longer there, or disabled, is not.
    let checked: Vec<T> = checked.into_iter().filter(|value| enabled(value)).collect();

    let model = Arc::new(Model {
      order: self.items.iter().map(|item| item.value().clone()).collect(),
      value: target.clone(),
      checked: checked.clone(),
      write: self.write.clone(),
      on_change: self.on_change.clone(),
      write_checked: match &self.checked {
        Some((_, write)) => write.clone(),
        None => {
          let (held, revision) = (kept.checked.clone(), kept.revision.clone());
          Some(Arc::new(move |next| {
            *held.lock() = next;
            if let Some(revision) = &revision {
              revision.update(|revision| *revision += 1);
            }
          }))
        }
      },
      on_checked_change: self.on_checked_change.clone(),
    });

    let list_rules = Recipe::new()
      .extend(&recipe.rules_for(TransferPart::Items, ListboxPart::Root))
      .base(ListboxPart::Content, |s| s.padding_x(0.0).padding_y(0.0).gap(0.0))
      .extend(&recipe.rules_for(TransferPart::Item, ListboxPart::Option))
      .extend(&recipe.rules_for(TransferPart::ItemLabel, ListboxPart::Label));
    let most = super::select::px(recipe.part(TransferPart::Items).rest().max_height, theme);

    let side = |is_target: bool| {
      let items: Vec<SelectOption<T>> = if is_target {
        target
          .iter()
          .filter_map(|value| self.items.iter().find(|item| item.value() == value).cloned())
          .collect()
      } else {
        self
          .items
          .iter()
          .filter(|item| !target.contains(item.value()))
          .cloned()
          .collect()
      };
      let held: Vec<T> = items.iter().map(|item| item.value().clone()).collect();
      let on: Vec<T> = checked.iter().filter(|value| held.contains(value)).cloned().collect();
      let can: Vec<T> = held.iter().filter(|value| enabled(value)).cloned().collect();
      let title = if is_target {
        told(&self.target_title, &words.selected)
      } else {
        told(&self.source_title, &words.available)
      };
      let name = if is_target { "target" } else { "source" };
      let id = self.id.as_ref().map(|id| Arc::<str>::from(format!("{id}-{name}")));
      let count = items.len();
      let list = (count > 0).then(|| {
        let model = model.clone();
        let side_values = held.clone();
        let mut listbox = Listbox::controlled_multiple(on.clone(), items, move |next: &[T]| {
          model.check_in(&side_values, next.to_vec());
        })
        .checkboxes(true)
        .disabled(self.disabled)
        .label(&title)
        .force(self.forced)
        .rules(&list_rules);
        if let Some(density) = self.density {
          listbox = listbox.density(density);
        }
        if let Some(id) = &id {
          listbox = listbox.id(id.clone());
        }
        match most {
          // A long list builds only the items near its view: its box is as high as the most it is.
          Some(most) if count > WINDOWED => (listbox.style(ListboxPart::Root, |s| s.height(most)), true),
          _ => (listbox, false),
        }
      });
      let all = {
        let model = model.clone();
        let (side_values, can) = (held.clone(), can.clone());
        Arc::new(move |on: bool| model.check_in(&side_values, if on { can.clone() } else { Vec::new() }))
          as Arc<dyn Fn(bool) + Send + Sync>
      };
      let side = TransferList {
        title,
        count: (self.count_text)(on.len(), count),
        checked: on.len(),
        can: can.len(),
        all_checked: !can.is_empty() && can.iter().all(|value| on.contains(value)),
        list: None,
        empty_text: told(&self.empty_text, &words.no_items),
        select_all_label: told(&self.select_all_label, &words.select_all),
        disabled: self.disabled,
        all,
        recipe: recipe.clone(),
        id,
      };
      (side, list)
    };
    let (source, source_list) = side(false);
    let (target_list, target_items) = side(true);

    let moving = |to_target: bool| {
      let model = model.clone();
      Arc::new(move || model.move_checked(to_target)) as Arc<dyn Fn() + Send + Sync>
    };
    let view = TransferView {
      add: (source.checked > 0 && !self.disabled).then(|| moving(true)),
      remove: (target_list.checked > 0 && !self.disabled).then(|| moving(false)),
      source,
      target: target_list,
      add_label: told(&self.add_label, &words.add),
      remove_label: told(&self.remove_label, &words.remove),
      stacked: kept.stacked,
      label: self.label.clone(),
      disabled: self.disabled,
      recipe,
      buttons: kept.buttons.clone(),
      room: kept.need.map(|(room, need)| {
        *need.lock() = self.need(theme);
        room
      }),
      id: self.id.clone(),
      key: self.key.clone(),
    };
    (view, [source_list, target_items])
  }
}

/// The transfer declared where it is converted: the live component, as [`mount`](Transfer::mount) makes it, which
/// keeps what is checked and finds its narrow form by its own room. `.child(transfer)` stacks in a narrow room.
impl<T: Clone + PartialEq + Send + Sync + 'static> From<Transfer<T>> for Element {
  fn from(transfer: Transfer<T>) -> Self {
    let key = transfer.key.clone();
    let props = TransferProps { transfer };
    match key {
      Some(key) => Element::component_keyed::<TransferHost<T>>(key, props),
      None => Element::component::<TransferHost<T>>(props),
    }
  }
}

/// What a transfer keeps from one render to the next: what is checked, whether it is stacked, and, mounted, the
/// box it measures its room by and how wide it needs to be side by side.
struct Kept<T> {
  checked: Arc<Mutex<Vec<T>>>,
  /// Renders a mounted transfer again: what is checked changed.
  revision: Option<Signal<u64>>,
  stacked: bool,
  need: Option<(ElementRef, Arc<Mutex<f32>>)>,
  /// The two buttons, mounted: one its move disabled keeps the focus.
  buttons: Option<[ElementRef; 2]>,
}

/// The value and what is checked, which the lists, the checkboxes of their headers and the buttons change.
struct Model<T> {
  /// Every item's value, in the order of the items.
  order: Vec<T>,
  value: Vec<T>,
  checked: Vec<T>,
  write: Option<Write<T>>,
  on_change: Vec<ChangeHandler<[T]>>,
  write_checked: Option<Write<T>>,
  on_checked_change: Vec<ChangeHandler<[T]>>,
}

impl<T: Clone + PartialEq> Model<T> {
  /// Sets what is checked where it is another, then tells the handlers.
  fn set_checked(&self, next: Vec<T>) {
    if next == self.checked {
      return;
    }
    if let Some(write) = &self.write_checked {
      write(next.clone());
    }
    for handler in &self.on_checked_change {
      handler(&next);
    }
  }

  /// What is checked of the list of `side` is `next`; what is checked of the other stays.
  fn check_in(&self, side: &[T], next: Vec<T>) {
    let mut all: Vec<T> = self
      .checked
      .iter()
      .filter(|value| !side.contains(value))
      .cloned()
      .collect();
    all.extend(next);
    self.set_checked(all);
  }

  /// Moves what is checked on one side to the other: to the end of the right, or out of it. What moved is no longer
  /// checked. The value is set and told first, then what is checked.
  fn move_checked(&self, to_target: bool) {
    // What moves, in the order it is shown in: the items' on the left, the value's on the right.
    let from = if to_target { &self.order } else { &self.value };
    let moving: Vec<T> = from
      .iter()
      .filter(|value| self.checked.contains(value) && self.value.contains(value) != to_target)
      .cloned()
      .collect();
    if moving.is_empty() {
      return;
    }
    let next: Vec<T> = if to_target {
      self.value.iter().chain(&moving).cloned().collect()
    } else {
      self
        .value
        .iter()
        .filter(|value| !moving.contains(value))
        .cloned()
        .collect()
    };
    if let Some(write) = &self.write {
      write(next.clone());
    }
    for handler in &self.on_change {
      handler(&next);
    }
    self.set_checked(
      self
        .checked
        .iter()
        .filter(|value| !moving.contains(value))
        .cloned()
        .collect(),
    );
  }
}

/// A transfer as a renderer sees it: its two lists, its buttons' behavior, and the style of its parts.
pub struct TransferView {
  /// The left list, of what is not chosen.
  pub source: TransferList,
  /// The right list, of what is: the value.
  pub target: TransferList,
  /// Whether the lists are one over the other: the room is too narrow for them side by side.
  pub stacked: bool,
  /// What the transfer is called, to what reads the tree.
  pub label: Option<Arc<str>>,
  pub disabled: bool,
  /// The style of each part of the transfer.
  pub recipe: ResolvedRecipe,
  add: Option<Arc<dyn Fn() + Send + Sync>>,
  remove: Option<Arc<dyn Fn() + Send + Sync>>,
  add_label: Arc<str>,
  remove_label: Arc<str>,
  room: Option<ElementRef>,
  buttons: Option<[ElementRef; 2]>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl TransferView {
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The button that moves what is checked on the left to the right, or back (`to_target` false): an icon alone,
  /// the primary one and the outline one, disabled while its list has nothing checked. Stacked, its chevron points
  /// down, or up.
  pub fn button(&self, to_target: bool) -> Element {
    let (run, label, variant) = if to_target {
      (&self.add, &self.add_label, ButtonVariant::Primary)
    } else {
      (&self.remove, &self.remove_label, ButtonVariant::Outline)
    };
    let icon = match (to_target, self.stacked) {
      (true, false) => icons::CHEVRON_RIGHT,
      (true, true) => icons::CHEVRON_DOWN,
      (false, false) => icons::CHEVRON_LEFT,
      (false, true) => icons::CHEVRON_UP,
    };
    let element = self
      .buttons
      .as_ref()
      .map(|buttons| buttons[usize::from(!to_target)].clone());
    // A button whose move took away what it moves keeps the focus it has: it looks disabled and does nothing, and
    // is disabled once the focus leaves it.
    let held = run.is_none() && element.as_ref().is_some_and(ElementRef::focused);
    let mut button = Button::new(label)
      .variant(variant)
      .size(ButtonSize::Sm)
      .icon(icon)
      .icon_only(true)
      .disabled(run.is_none() && !held)
      .state(ButtonState::Disabled, held);
    if let Some(run) = run.clone() {
      button = button.on_click(move |_: crate::app::events::MouseEvent| run());
    }
    if let Some(element) = element {
      button = button.ref_element(element);
    }
    if let Some(id) = &self.id {
      button = button.id(format!("{id}-{}", if to_target { "add" } else { "remove" }));
    }
    let button = button.build(self.theme());
    if held {
      button.decorate_root(|mut node| {
        crate::node::NodeUpdate::describe(&mut node, Arc::from("a11y_disabled"), Arc::from("true"));
        node
      })
    } else {
      button
    }
  }

  /// The two buttons in the style of their part: one under the other, or side by side where the lists are stacked.
  pub fn actions(&self) -> Element {
    let part = self.part(TransferPart::Actions);
    let buttons = [self.button(true), self.button(false)];
    // A group the arrows move between, in which the focus stays on a button its move disabled.
    if self.stacked {
      Row::new()
        .justify(Justify::Center)
        .align_items(Alignment::Center)
        .focus_group(FocusGroup::horizontal().wrap(false))
        .part(&part)
        .with_children(buttons)
        .into()
    } else {
      Column::new()
        .align_items(Alignment::Center)
        .flex_shrink(0.0)
        .focus_group(FocusGroup::vertical().wrap(false))
        .part(&part)
        .with_children(buttons)
        .into()
    }
  }

  /// How the default renderer builds a transfer: the left list, the buttons and the right list, side by side, or one
  /// under the other where the room is narrow. It is a group to what reads the tree, called by its label.
  pub fn render(mut self) -> Element {
    let actions = self.actions();
    let (source, target) = (
      std::mem::replace(&mut self.source.list, None),
      std::mem::replace(&mut self.target.list, None),
    );
    let (source, target) = (self.source.render(source), self.target.render(target));
    let part = self.part(TransferPart::Root);
    let fill = Dimension::Pct(100.0);
    let mut root = if self.stacked {
      Column::new()
        .width(fill)
        .align_items(Alignment::Stretch)
        .part(&part)
        .child(source)
        .child(actions)
        .child(target)
    } else {
      Column::new().child(
        Row::new()
          .align_items(Alignment::Center)
          .part(&part)
          .child(source)
          .child(actions)
          .child(target),
      )
    }
    .describe("a11y_role", "group");
    if let Some(label) = &self.label {
      root = root.describe("a11y_name", label.clone());
    }
    if self.disabled {
      root = root.describe("a11y_disabled", "true");
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    match self.room {
      // Mounted: in a column as wide as the room, which it measures before the frame is painted.
      Some(room) => Column::new()
        .width(fill)
        .align_items(if self.stacked {
          Alignment::Stretch
        } else {
          Alignment::Start
        })
        .ref_element(room)
        .child(root)
        .into(),
      None => root.into(),
    }
  }
}

/// One list of a transfer as a renderer sees it: what its header says, its checkbox's state, and its items, built.
pub struct TransferList {
  /// What it is called, in its header.
  pub title: Arc<str>,
  /// What its header says of how many are checked, of how many.
  pub count: String,
  /// How many of its items are checked.
  pub checked: usize,
  /// How many of its items can be checked.
  pub can: usize,
  /// Whether every item that can be checked is.
  pub all_checked: bool,
  /// Its items: a listbox with checkboxes. `None` where it has none.
  pub list: Option<Element>,
  pub disabled: bool,
  empty_text: Arc<str>,
  select_all_label: Arc<str>,
  all: Arc<dyn Fn(bool) + Send + Sync>,
  /// The style of each part of the transfer.
  pub recipe: ResolvedRecipe,
  id: Option<Arc<str>>,
}

impl TransferList {
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The checkbox that checks every item that can be, or none: half checked while some are, disabled while none can
  /// be.
  pub fn select_all(&self) -> Element {
    let all = self.all.clone();
    let mut checkbox = Checkbox::new(&self.select_all_label)
      .label_hidden(true)
      .checked(self.all_checked)
      .indeterminate(self.checked > 0 && !self.all_checked)
      .disabled(self.disabled || self.can == 0)
      .on_change(move |on| all(on));
    if let Some(id) = &self.id {
      checkbox = checkbox.id(format!("{id}-all"));
    }
    checkbox.build(self.theme())
  }

  /// The header in the style of its part: the checkbox, the title, which ends in an ellipsis where it is longer than
  /// its room, and the count, which keeps its room.
  pub fn header(&self) -> Row {
    let (title, count) = (self.part(TransferPart::Title), self.part(TransferPart::Count));
    Row::new()
      .align_items(Alignment::Center)
      .part(&self.part(TransferPart::Header))
      .child(Row::new().flex_shrink(0.0).child(self.select_all()))
      .child(
        Row::new()
          .flex_full(1.0, 1.0, Some(0.0))
          .min_width(0.0)
          .part(&title)
          .child(title.text(&self.title).nowrap().text_overflow(TextOverflow::Elipsis)),
      )
      .child(
        Row::new()
          .flex_shrink(0.0)
          .part(&count)
          .child(count.text(&self.count).nowrap()),
      )
  }

  /// How the default renderer builds a list: its header over its items, or over what it says where it has none, in
  /// the box of its part.
  pub fn render(&self, list: Option<Element>) -> Element {
    let body: Element = match list {
      Some(list) => list,
      None => {
        let empty = self.part(TransferPart::Empty);
        Row::new().part(&empty).child(empty.text(&self.empty_text)).into()
      }
    };
    Column::new()
      .align_items(Alignment::Stretch)
      .part(&self.part(TransferPart::List))
      .child(self.header())
      .child(body)
      .into()
  }
}

/// Every transfer has the one recipe and renderer, whatever its values are.
impl<T: Send + Sync + 'static> Themed for Transfer<T> {
  const NAME: &'static str = "transfer";
  type Renderer = TransferRenderer;

  /// The design's transfer (`pencil/lurq-design-system.pen`, Transfer `ZAu2E`): two lists of 190 on the background in
  /// an edge of 1 in the border's color, 8 round, that cut what is in them, 12 from the buttons between them, in the
  /// middle of each other's height. A list's header is 36 high on the muted fill with a line of 1 in the border's
  /// color under it, 12 in from its sides, a checkbox, its title in the body's type at 13 and 500, and the count in
  /// the mono type at 11 in the muted foreground, 8 apart. Its items are 4 in from its top and bottom, each 32 high,
  /// 12 in from its sides, a checkbox and its label at 13 8 apart. The buttons are 32 square, 8 apart, the primary
  /// one that moves to the right and the outline one that moves back. The board draws no item under the pointer,
  /// focused or disabled, no long list and no empty one: under the pointer an item is on the muted fill, the focus
  /// from the keyboard is the ring inside it, a disabled item is a listbox's; a list scrolls past 264, eight items;
  /// an empty one says so at 13 in the muted foreground. Stacked, the lists are as wide as the room, 12 apart, and
  /// the buttons side by side between them. At each density an item is as high as a row of the density is.
  fn recipe() -> Recipe {
    use PaletteColor::{Background, Foreground, Muted, MutedForeground};
    use TransferPart::{Actions, Count, Empty, Header, Item, ItemLabel, Items, List, Root, Title};
    let edge = |width: f32| Border::inside(width, PaletteColor::Border);
    Recipe::new()
      .base(Root, |s| s.gap(12.0))
      .base(List, |s| {
        s.width(190.0)
          .max_width(Length::Fill)
          .background(Background)
          .foreground(Foreground)
          .border(Border::inside(BorderSize::Sm, PaletteColor::Border))
          .radius(RadiusSize::Lg)
      })
      .state(TransferState::Stacked, List, |s| s.width(Length::Fill))
      .base(Header, |s| {
        s.height(36.0)
          .padding_x(12.0)
          .gap(8.0)
          .background(Muted)
          .underline(edge(1.0))
      })
      .base(Title, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(13.0)
          .weight(FontWeight::Medium)
          .foreground(Foreground)
      })
      .base(Count, |s| {
        s.font_family(FontFamily::Mono)
          .font_size(11.0)
          .foreground(MutedForeground)
      })
      .base(Items, |s| {
        s.width(Length::Fill)
          .max_height(264.0)
          .padding_y(4.0)
          .radius(0.0)
          .border(edge(0.0))
          .background(Background)
      })
      .base(Item, |s| s.height(32.0).padding_x(12.0).gap(8.0).radius(0.0))
      .row_density(Selector::new(), Item, 32.0)
      .base(ItemLabel, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(13.0)
          .foreground(Foreground)
      })
      .base(Empty, |s| {
        s.padding_x(12.0)
          .padding_y(12.0)
          .font_size(13.0)
          .foreground(MutedForeground)
      })
      .base(Actions, |s| s.gap(8.0))
      .state(TransferState::Disabled, Root, |s| s.opacity(OpacityLevel::Disabled))
  }
}

#[derive(Clone)]
struct TransferProps<T: 'static> {
  transfer: Transfer<T>,
}

/// A transfer built again is drawn again: its value is read from the app's signal as it is built.
impl<T> PartialEq for TransferProps<T> {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

impl<T> crate::app::component::DevtoolsInspectable for TransferProps<T> {
  fn inspect(&self, formatter: &mut crate::app::component::DevtoolsFormatter<'_>) {
    formatter.value("Transfer", format!("{} items", self.transfer.items.len()));
  }
}

/// What a mounted transfer keeps between renders.
struct TransferHost<T> {
  buttons: [ElementRef; 2],
  checked: Arc<Mutex<Vec<T>>>,
  revision: Signal<u64>,
  stacked: Signal<bool>,
  room: ElementRef,
  need: Arc<Mutex<f32>>,
}

impl<T: Clone + PartialEq + Send + Sync + 'static> Component for TransferHost<T> {
  type Props = TransferProps<T>;

  fn create(ctx: &mut Ctx) -> Self {
    let (room, stacked, need) = (ElementRef::new(), ctx.signal(false), Arc::new(Mutex::new(0.0_f32)));
    // Written only where the room crosses what the lists need side by side.
    let (watch, needed) = (stacked.clone(), need.clone());
    room.observe_rect_before_paint(move |rect| {
      let now = rect.width < *needed.lock() - 0.5;
      if watch.get_untracked() != now {
        watch.set(now);
      }
    });
    Self {
      buttons: [ElementRef::new(), ElementRef::new()],
      checked: Arc::default(),
      revision: ctx.signal(0_u64),
      stacked,
      room,
      need,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    // Read so that a check renders the transfer again, and so does the focus leaving a button.
    self.revision.get();
    for button in &self.buttons {
      button.focus_signal().get();
    }
    let kept = Kept {
      checked: self.checked.clone(),
      revision: Some(self.revision.clone()),
      stacked: self.stacked.get(),
      need: Some((self.room.clone(), self.need.clone())),
      buttons: Some(self.buttons.clone()),
    };
    let theme = current_theme();
    let (view, lists) = ctx.props::<TransferProps<T>>().transfer.view(&theme, kept);
    // A long list is mounted, to build only the items near its view.
    let lists = lists.map(|list| {
      list.map(|(listbox, long)| {
        if long {
          listbox.mount(ctx)
        } else {
          listbox.build(&theme)
        }
      })
    });
    ctx.props::<TransferProps<T>>().transfer.finish(view, lists)
  }
}
