//! A multi select: several values chosen from a list that what is typed filters, held as tags in its box.

use std::{
  collections::HashMap,
  panic::Location,
  sync::{Arc, LazyLock, Mutex, Weak},
};

use super::{
  combobox::{self, Choices, ComboboxSize, ComboboxState, Engine, Filter, Finish, Many, ManyOf, apart},
  field::{FieldControl, InField},
  icon::Icon,
  icons, names,
  part_style::{Length, PartStyle},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  select::{self, engine_style},
  tag::{Tag, TagPart, TagSize},
  text_box::{self, BoxStates, EditHandler, Judgment, TextBoxSize},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    events::{KeyboardEvent, TextInputEvent},
    theme::{BorderSize, IconSize, PaletteColor, RadiusSize, Theme},
  },
  components::{self, Row, SelectOption, SelectTriggerState},
  core::{Signal, SignalValue},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{
    CursorIcon, Element, FocusGroup, SelectCheckmarkPosition, SelectIcon, SelectStyle, border::Border,
    dimension::Dimension,
  },
};

/// What a multi select keeps across its renders, one for each signal one is bound to while it is in the tree: what
/// was chosen at the last build, to tell a choice, and the tag the arrow keys are on. `alive` is held by the
/// input of the last one built.
struct Kept {
  chosen: Vec<usize>,
  picked: Signal<Option<usize>>,
  alive: Weak<()>,
}

static KEPT: LazyLock<Mutex<HashMap<usize, Kept>>> = LazyLock::new(Default::default);

/// The tag the arrow keys are on in the multi select bound to `binding`, what keeps it while the multi select is
/// in the tree, and whether a value was chosen since its last build: an option the last did not hold is held now.
fn kept(binding: usize, chosen: &[usize]) -> (Signal<Option<usize>>, Arc<()>, bool) {
  let mut all = KEPT.lock().unwrap();
  all.retain(|_, kept| kept.alive.strong_count() > 0);
  if let Some(kept) = all.get_mut(&binding)
    && let Some(alive) = kept.alive.upgrade()
  {
    let grew = chosen.iter().any(|index| !kept.chosen.contains(index));
    kept.chosen = chosen.to_vec();
    return (kept.picked.clone(), alive, grew);
  }
  let alive = Arc::new(());
  let picked = Signal::new(None);
  all.insert(
    binding,
    Kept {
      chosen: chosen.to_vec(),
      picked: picked.clone(),
      alive: Arc::downgrade(&alive),
    },
  );
  (picked, alive, false)
}

/// The parts of a multi select a recipe styles. Those it shares with a select and a combobox have their names, and
/// the engine draws its list as it draws theirs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MultiSelectPart {
  /// The box that holds the tags and is typed in, and that the list opens under: its width, its least height, its
  /// edge, its fill, its ring, its padding, and the type of what is typed. Its gap is the room between the tags and
  /// what is at its end.
  Trigger,
  /// What is typed, after the tags, over the trigger's type and color. Its least width is the room it keeps on the
  /// line of the tags before it goes on to the next.
  Value,
  /// What the box says while nothing is chosen and nothing is typed.
  Placeholder,
  /// The glyph at the end of the box.
  Chevron,
  /// The panel of the open list.
  Panel,
  /// An option of the open list. Its gap is the room between its box and its label.
  Item,
  /// The box before a chosen option: its side, corners, fill, edge, and the color of its check.
  Check,
  /// The check in that box: its side.
  CheckIcon,
  /// The box before an option that is not chosen.
  Unchecked,
  /// The second, smaller line of an option that has one.
  Detail,
  /// The short text at the end of an option that has one.
  Hint,
  /// The label over a group of options.
  Group,
  /// The line between two groups.
  Separator,
  /// The row after the options that cannot be chosen: "No results", or the spinner with "Loading results…".
  Status,
  /// The spinner of the status row.
  Spinner,
  /// The tags and the input after them, which go on to the next line where they do not fit: its gap is the room
  /// between them, across and down.
  Tags,
  /// A chosen value as a [`Tag`] in the box, and the tag of how many more there are past [`MultiSelect::max_shown`]:
  /// what is laid over the root of the tag.
  Tag,
  /// The text of a tag: what is laid over its label.
  TagLabel,
  /// What takes every chosen value away, at the end of the box ([`MultiSelect::clearable`]).
  Clear,
  /// The cross in it: its side.
  ClearIcon,
}

names!(MultiSelectPart {
  Trigger => "trigger",
  Value => "value",
  Placeholder => "placeholder",
  Chevron => "chevron",
  Panel => "panel",
  Item => "item",
  Check => "check",
  CheckIcon => "check_icon",
  Unchecked => "unchecked",
  Detail => "detail",
  Hint => "hint",
  Group => "group",
  Separator => "separator",
  Status => "status",
  Spinner => "spinner",
  Tags => "tags",
  Tag => "tag",
  TagLabel => "tag_label",
  Clear => "clear",
  ClearIcon => "clear_icon",
});

/// The sizes of a multi select: a combobox's, the least heights of its box, with tags of 20, 24 and 28.
pub type MultiSelectSize = ComboboxSize;

/// The states of a multi select a recipe styles: a combobox's, by the same names.
pub type MultiSelectState = ComboboxState;

/// What builds a multi select's nodes: a theme's in place of the default one.
pub type MultiSelectRenderer = Arc<dyn Fn(MultiSelectView) -> Element + Send + Sync>;

/// A multi select: several values chosen from a list, which what is typed filters. Labels on an issue, the languages
/// of a project, the people a message goes to.
///
/// ```
/// use lurq::{core::Signal, ui::MultiSelect};
///
/// let languages = Signal::new(vec!["rust", "wgsl"]);
/// let select = MultiSelect::new(languages, [
///   ("rust", "Rust"),
///   ("wgsl", "WGSL"),
///   ("typescript", "TypeScript"),
///   ("swift", "Swift"),
/// ])
/// .placeholder("Select languages")
/// .max_shown(3);
/// // Held by the app: it is told the whole new set, and the box shows what it answers with.
/// let held = MultiSelect::controlled(vec!["rust"], [("rust", "Rust"), ("wgsl", "WGSL")], |languages| {
///   println!("{languages:?}")
/// });
/// ```
///
/// It is the engine's multiple [`Select`](crate::components::Select) as a combobox, in the design's dress: its box
/// holds what is chosen as tags, in the order they were chosen, and a text input after them, which keeps the focus
/// while the list is open. Typing opens the list and filters it; Down and Up move the highlight, Enter chooses or
/// takes back the highlighted option, and the list stays open while one chooses, with a checked box before each
/// option chosen. A tag's cross takes its value away, and so does Backspace in an empty input, the last one's.
/// Escape closes the list and a second Escape clears what was typed; Tab goes on, and what was typed is cleared
/// when the focus leaves. Past [`MultiSelect::max_shown`] tags the rest are one tag that counts them, "+3", and the
/// tags go on to the next line where they do not fit: the box grows with them.
pub struct MultiSelect<V = ()> {
  choices: Box<dyn ManyOf<V>>,
  placeholder: Option<Arc<str>>,
  boxed: BoxStates,
  full_width: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  filter: Filter,
  on_query: Vec<EditHandler>,
  loading: bool,
  empty_text: Arc<str>,
  loading_text: Arc<str>,
  max_shown: Option<usize>,
  clearable: bool,
  clear_label: Arc<str>,
  query: Option<Signal<String>>,
  name: Option<Arc<str>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl<T> MultiSelect<T>
where
  T: Clone + PartialEq + Send + Sync + SignalValue + 'static,
  Vec<T>: SignalValue,
{
  /// A multi select of `options` bound to `value`: the options whose values the signal has are chosen, and choosing
  /// one adds its value, or takes it out. An option is a `(value, label)` or a [`SelectOption`]. A multi select is
  /// the `multiple` of a [`Select`](super::Select) by what it is, so it keeps `new` and `controlled` of its own, where
  /// a component that holds one value or any number has `multiple` and `controlled_multiple`.
  pub fn new(value: Signal<Vec<T>>, options: impl IntoIterator<Item = impl Into<SelectOption<T>>>) -> Self {
    Self::of(Box::new(Many::bound(
      value,
      options.into_iter().map(Into::into).collect(),
    )))
  }

  /// A multi select of the app's `values`, which the app holds: a choice, a tag taken away and a clear are told to
  /// `on_change` with the whole new set, and the box shows what the app answers with. Two built at the one place in
  /// the app's code, in a loop, are told apart by their `id` or `key`.
  #[track_caller]
  pub fn controlled(
    values: Vec<T>,
    options: impl IntoIterator<Item = impl Into<SelectOption<T>>>,
    on_change: impl Fn(&[T]) + Send + Sync + 'static,
  ) -> Self {
    let options = options.into_iter().map(Into::into).collect();
    Self::of(Box::new(Many::controlled(values, options, Location::caller()))).on_change(on_change)
  }
}

impl<V: 'static> MultiSelect<V> {
  fn of(choices: Box<dyn ManyOf<V>>) -> Self {
    Self {
      choices,
      placeholder: None,
      boxed: BoxStates::default(),
      full_width: false,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      filter: Arc::new(combobox::contains),
      on_query: Vec::new(),
      loading: false,
      empty_text: super::current_locale().common.no_results.clone(),
      loading_text: super::current_locale().common.loading_results.clone(),
      max_shown: None,
      clearable: false,
      clear_label: super::current_locale().common.clear.clone(),
      query: None,
      name: None,
      id: None,
      key: None,
    }
  }

  /// Called with the whole new set when a person chooses an option, takes one back or away, or clears them: not when
  /// the app sets the signal. The handler takes the type of the values: `on_change(|languages: &[&str]| ..)` of a
  /// multi select of `&str`; a handler of another type does not compile.
  pub fn on_change(mut self, handler: impl Fn(&[V]) + Send + Sync + 'static) -> Self {
    self.choices.on_change(Arc::new(handler));
    self
  }

  /// What the box says while nothing is chosen and nothing is typed.
  pub fn placeholder(mut self, placeholder: &str) -> Self {
    self.placeholder = Some(Arc::from(placeholder));
    self
  }

  /// What the multi select is called, to what reads the tree. The field it is in shows its label.
  pub fn label(mut self, label: &str) -> Self {
    self.boxed.label = Some(Arc::from(label));
    self
  }

  /// A [`MultiSelectSize`], or a size the theme's recipe names: the least height of the box, and the size of its
  /// tags.
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.boxed.size = size.into();
    self
  }

  /// Whether what is chosen is wrong. Of invalid, warning and valid, the first that is set is the one shown.
  pub fn invalid(mut self, invalid: bool) -> Self {
    self.boxed.invalid = invalid;
    self
  }

  /// Whether what is chosen is allowed with a caveat.
  pub fn warning(mut self, warning: bool) -> Self {
    self.boxed.warning = warning;
    self
  }

  /// Whether what is chosen was checked and is right.
  pub fn valid(mut self, valid: bool) -> Self {
    self.boxed.valid = valid;
    self
  }

  /// Whether what is chosen can be read and not changed: the list does not open, what is typed is not taken, the
  /// tags have no cross, and there is no clear control. It keeps the focus and is read by what reads the tree.
  pub fn read_only(mut self, read_only: bool) -> Self {
    self.boxed.read_only = read_only;
    self
  }

  /// Whether the multi select takes no input: its tags cannot be taken away either.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.boxed.disabled = disabled;
    self
  }

  /// Whether the multi select is as wide as what holds it, in place of the design's 280.
  pub fn full_width(mut self, full_width: bool) -> Self {
    self.full_width = full_width;
    self
  }

  /// Which options are shown while something is typed: `filter(label, typed)`. By default those whose label
  /// contains what was typed, without regard to case.
  pub fn filter(mut self, filter: impl Fn(&str, &str) -> bool + Send + Sync + 'static) -> Self {
    self.filter = Arc::new(filter);
    self
  }

  /// Called with what the box will hold at each edit: for an app that fetches its options as a person types.
  pub fn on_query(mut self, handler: impl Fn(&str) + Send + Sync + 'static) -> Self {
    self
      .on_query
      .push(Arc::new(move |event: &TextInputEvent| handler(event.new_value())));
    self
  }

  /// Whether more options are loading: the list ends in a row with a spinner that says so.
  pub fn loading(mut self, loading: bool) -> Self {
    self.loading = loading;
    self
  }

  /// What the list says while what is typed leaves no option: "No results" unless it is told otherwise.
  pub fn empty_text(mut self, text: &str) -> Self {
    self.empty_text = Arc::from(text);
    self
  }

  /// What the row of [`MultiSelect::loading`] says: "Loading results…" unless it is told otherwise.
  pub fn loading_text(mut self, text: &str) -> Self {
    self.loading_text = Arc::from(text);
    self
  }

  /// How many tags the box shows at most: the rest are one tag that counts them, "+3".
  pub fn max_shown(mut self, max: usize) -> Self {
    self.max_shown = Some(max);
    self
  }

  /// Whether the box has a cross at its end, while something is chosen, that takes every chosen value away.
  pub fn clearable(mut self, clearable: bool) -> Self {
    self.clearable = clearable;
    self
  }

  /// What the clear control is called to what reads the tree: "Clear" unless it is told otherwise.
  pub fn clear_label(mut self, label: &str) -> Self {
    self.clear_label = Arc::from(label);
    self
  }

  /// The signal of what is typed, where the app keeps it. Without one the multi select keeps it itself while it is
  /// in the tree.
  pub fn query(mut self, query: Signal<String>) -> Self {
    self.query = Some(query);
    self
  }

  /// Puts the multi select in a state the theme's recipe names ([`MultiSelectState`]), or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// The name the multi select's value has in the form it is in.
  #[cfg(feature = "form")]
  pub fn name(mut self, name: impl Into<Arc<str>>) -> Self {
    self.name = Some(name.into());
    self
  }

  /// Styles one part of this multi select over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this multi select.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the box as it is under the pointer or focused, with nothing pointing at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the multi select's node: the box the list opens under. Its input has the id with `-input` after it.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the multi select in `theme`. Converting one into an element does this in the theme of what is being
  /// rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = self
      .boxed
      .case(self.states.clone())
      .state(ComboboxState::FullWidth, self.full_width);
    let mut choices = self.choices;
    choices.name(self.id.as_deref().or(self.key.as_deref()));
    let choices: Box<dyn Choices> = choices;
    let choices: Arc<dyn Choices> = Arc::from(choices);
    let labels = choices.labels();
    let chosen = choices.chosen();
    let (query, alive) = match self.query {
      Some(query) => (query, Arc::new(())),
      None => combobox::typed(choices.binding(), None),
    };
    let (picked, kept_alive, chose) = kept(choices.binding(), &chosen);
    // A choice clears what was typed: the list shows every option again for the next.
    if chose && !query.get_untracked().is_empty() {
      query.set(String::new());
    }
    let typed_now = query.get();
    let shown = combobox::filtered(&labels, &typed_now, None, &self.filter);
    let tags = chosen.len().min(self.max_shown.unwrap_or(usize::MAX));
    let picked_now = picked.get().filter(|&tag| tag < tags);
    let view = MultiSelectView {
      placeholder: self.placeholder,
      label: self.boxed.label.clone(),
      invalid: self.boxed.judgment() == Some(Judgment::Invalid),
      disabled: self.boxed.disabled,
      loading: self.loading,
      max_shown: self.max_shown,
      clearable: self.clearable,
      clear_label: self.clear_label,
      read_only: self.boxed.read_only,
      recipe: theme
        .resolve::<MultiSelect>(&case, Some(&self.rules))
        .owned_by(self.id.as_ref()),
      case,
      rules: self.rules,
      forced: self.forced,
      choices,
      labels,
      chosen,
      shown,
      query,
      alive: Arc::new((alive, kept_alive)),
      picked,
      picked_now,
      on_query: self.on_query,
      empty_text: self.empty_text,
      loading_text: self.loading_text,
      boxed: self.boxed,
      name: self.name,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<MultiSelect>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl<V: 'static> From<MultiSelect<V>> for Element {
  fn from(select: MultiSelect<V>) -> Self {
    select.build(&current_theme())
  }
}

impl<V: 'static> FieldControl for MultiSelect<V> {
  /// Called by the field's label unless it has a name of its own, judged as the field is unless the app judged it
  /// itself, and disabled where the field is.
  fn in_field(mut self, field: &InField) -> Element {
    self.boxed.in_field(field);
    self.into()
  }

  fn has_fluid_form(&self) -> bool {
    true
  }
}

/// A multi select as a renderer sees it: what it was given, the style of its parts, and its behavior ready to be
/// dressed.
pub struct MultiSelectView {
  pub placeholder: Option<Arc<str>>,
  /// What it is called, to what reads the tree.
  pub label: Option<Arc<str>>,
  /// Whether what is chosen is wrong, as the multi select shows it.
  pub invalid: bool,
  pub disabled: bool,
  /// Whether more options are loading.
  pub loading: bool,
  /// How many tags the box shows at most.
  pub max_shown: Option<usize>,
  /// Whether the box has the cross that takes every chosen value away.
  pub clearable: bool,
  /// Whether what is chosen can be read and not changed.
  pub read_only: bool,
  /// The style of each part of this multi select.
  pub recipe: ResolvedRecipe,
  case: Case,
  rules: Recipe,
  forced: Option<Interaction>,
  choices: Arc<dyn Choices>,
  labels: Vec<Arc<str>>,
  chosen: Vec<usize>,
  shown: Vec<usize>,
  query: Signal<String>,
  /// Held by the input: what keeps what was typed and the tag the arrows are on while the multi select is in the
  /// tree.
  alive: Arc<(Arc<()>, Arc<()>)>,
  /// The tag the arrow keys are on, by its place among the tags shown: Backspace takes it away.
  picked: Signal<Option<usize>>,
  picked_now: Option<usize>,
  on_query: Vec<EditHandler>,
  empty_text: Arc<str>,
  loading_text: Arc<str>,
  clear_label: Arc<str>,
  boxed: BoxStates,
  // Read only where the engine has forms.
  #[cfg_attr(not(feature = "form"), allow(dead_code))]
  name: Option<Arc<str>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl MultiSelectView {
  /// The style of a part, as this multi select is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The recipe resolved with `state` on as well.
  pub fn in_state(&self, state: impl Into<Name>) -> ResolvedRecipe {
    let case = self.case.clone().state(state, true);
    self.theme().resolve::<MultiSelect>(&case, Some(&self.rules))
  }

  /// What is typed: the signal the input is bound to.
  pub fn query(&self) -> Signal<String> {
    self.query.clone()
  }

  /// The labels of the chosen options, in the order they were chosen in.
  pub fn chosen(&self) -> Vec<Arc<str>> {
    self
      .chosen
      .iter()
      .filter_map(|index| self.labels.get(*index).cloned())
      .collect()
  }

  /// The labels of the options the list shows for what is typed.
  pub fn shown(&self) -> Vec<Arc<str>> {
    self
      .shown
      .iter()
      .filter_map(|index| self.labels.get(*index).cloned())
      .collect()
  }

  /// The engine's style for the box and its list, from the recipe, as a select's, with a box before each option:
  /// checked where it is chosen.
  pub fn select_style(&self) -> SelectStyle {
    let theme = self.theme().clone();
    // Drawn by the engine for each option, each time it draws the list: no node of them is named.
    let (check, icon, unchecked) = (
      self.recipe.part(MultiSelectPart::Check).unnamed(),
      self.recipe.part(MultiSelectPart::CheckIcon).unnamed(),
      self.recipe.part(MultiSelectPart::Unchecked).unnamed(),
    );
    let side = combobox::side(&check.rest(), &theme, 16.0);
    let checked = {
      let theme = theme.clone();
      SelectIcon::element(move |_| {
        let glyph = icon.rest().width.unwrap_or(Length::Icon(IconSize::Xs));
        mark(&check)
          .child(Icon::new(icons::CHECK).size(glyph).build(&theme))
          .into()
      })
    };
    let empty = SelectIcon::element(move |_| mark(&unchecked).into());
    engine_style(&self.recipe, self.forced, &|state| self.in_state(state))
      .checkmark(checked)
      .checkmark_unchecked(empty)
      .checkmark_position(SelectCheckmarkPosition::Leading)
      .checkmark_size(side)
  }

  /// The text input after the tags, in the type of the value and of the placeholder, with no box of its own: a stop
  /// of Tab, called by the multi select's label. Backspace in it while it is empty takes the last value away, and
  /// leaving it clears what was typed.
  pub fn input(&self) -> components::TextInput {
    let theme = self.theme();
    let value = self
      .part(MultiSelectPart::Trigger)
      .rest()
      .merged(&self.part(MultiSelectPart::Value).rest());
    let placeholder = value.clone().merged(&self.part(MultiSelectPart::Placeholder).rest());
    let least = select::px(value.min_width, theme).unwrap_or(0.0);
    let control = components::TextInput::new(self.query.clone())
      .single_line()
      .text_style(text_box::ink(theme, &value))
      .placeholder_style(text_box::ink(theme, &placeholder))
      // As long as its least room when the tags are laid on their lines, and as long as the rest of its line then.
      .min_width(least)
      .flex_full(1.0, 1.0, Some(least))
      // Where Tab enters the line of tags and input: see `render`.
      .focus_entry(true);
    let mut control = text_box::text_box(control, &self.boxed, self.on_query.clone());
    // The placeholder says what to do while nothing is chosen; beside tags it would say it again.
    if let Some(placeholder) = self.placeholder.as_ref().filter(|_| self.chosen.is_empty()) {
      control = control.placeholder(placeholder);
    }
    if let Some(id) = &self.id {
      control = control.id(format!("{id}-input"));
    }
    let (query, alive, picked) = (self.query.clone(), self.alive.clone(), self.picked.clone());
    control = control.on_blur(move || {
      // Held while the input is: what was typed is kept for the multi select while it is in the tree.
      let _ = &alive;
      if !query.get_untracked().is_empty() {
        query.set(String::new());
      }
      if picked.get_untracked().is_some() {
        picked.set(None);
      }
    });
    let tags = self.chosen.len().min(self.max_shown.unwrap_or(usize::MAX));
    let (choices, query, picked, chosen, disabled) = (
      self.choices.clone(),
      self.query.clone(),
      self.picked.clone(),
      self.chosen.clone(),
      self.disabled || self.read_only,
    );
    // The input is the one stop of Tab; the tags are reached from it, as a token input's tokens are: with what is
    // typed empty, Left goes onto the last tag and on to the one before, Right back towards the input, and
    // Backspace or Delete takes away the tag the arrows are on, or the last where they are on none.
    control.on_key_down(move |event: KeyboardEvent| {
      let plain = !event.ctrl && !event.alt && !event.meta && !event.shift && !event.composing;
      if !event.focus_within || disabled {
        return;
      }
      let on = picked.get_untracked().filter(|&tag| tag < tags);
      let empty = query.get_untracked().is_empty();
      let set = |tag: Option<usize>| {
        if picked.get_untracked() != tag {
          picked.set(tag);
        }
      };
      match event.key.as_str() {
        "ArrowLeft" if plain && empty && tags > 0 => {
          event.prevent_default();
          set(Some(on.map_or(tags - 1, |tag| tag.saturating_sub(1))));
        }
        "ArrowRight" if plain && on.is_some() => {
          event.prevent_default();
          set(on.and_then(|tag| (tag + 1 < tags).then_some(tag + 1)));
        }
        "Backspace" | "Delete" if plain && empty => {
          let gone = match on {
            Some(tag) => chosen.get(tag).copied(),
            None if event.key == "Backspace" => chosen.last().copied(),
            None => None,
          };
          if let Some(index) = gone {
            event.prevent_default();
            // The arrows stay on the tag that takes its place, or the one before where it was the last.
            set(on.and_then(|tag| (tags > 1).then(|| tag.min(tags - 2))));
            choices.unchoose(index);
          }
        }
        // Anything else is typing: the arrows leave the tags.
        _ if on.is_some() => set(None),
        _ => {}
      }
    })
  }

  /// The rules a tag of the box is given over the theme's for a tag: those of [`MultiSelectPart::Tag`] for its root
  /// and of [`MultiSelectPart::TagLabel`] for its label.
  pub fn tag_rules(&self) -> Recipe {
    self
      .recipe
      .rules_for(MultiSelectPart::Tag, TagPart::Root)
      .extend(&self.recipe.rules_for(MultiSelectPart::TagLabel, TagPart::Label))
  }

  /// The size of the tags in a box of this size.
  fn tag_size(&self) -> TagSize {
    if self.boxed.size == Name::from(TextBoxSize::Sm) {
      TagSize::Sm
    } else if self.boxed.size == Name::from(TextBoxSize::Lg) {
      TagSize::Lg
    } else {
      TagSize::Md
    }
  }

  /// The chosen values as tags, a pill each with a cross that takes it away, up to [`MultiSelect::max_shown`], and
  /// past it a tag that counts the rest.
  pub fn tags(&self) -> Vec<Element> {
    let (rules, size) = (self.tag_rules(), self.tag_size());
    let theme = self.theme().clone();
    let max = self.max_shown.unwrap_or(usize::MAX);
    let mut tags: Vec<Element> = self
      .chosen
      .iter()
      .take(max)
      .enumerate()
      .map(|(place, index)| {
        let (choices, index) = (self.choices.clone(), *index);
        // The tag the arrow keys are on is drawn as its button is focused from the keyboard.
        let on = self.picked_now == Some(place);
        let tag = Tag::new(&self.labels[index])
          .shape(super::TagShape::Pill)
          .size(size)
          .disabled(self.disabled)
          .rules(&rules)
          .force(on.then_some(Interaction::FocusVisible));
        // Read only, a tag has no cross.
        if self.read_only {
          return tag.build(&theme);
        }
        tag.on_remove(move || choices.unchoose(index)).build(&theme)
      })
      .map(apart)
      .collect();
    let more = self.chosen.len().saturating_sub(max);
    if more > 0 {
      tags.push(apart(
        Tag::new(&format!("+{more}"))
          .shape(super::TagShape::Pill)
          .size(size)
          .disabled(self.disabled)
          .rules(&rules)
          .build(&theme),
      ));
    }
    tags
  }

  /// The cross at the end of the box that takes every chosen value away, while something is chosen and the multi
  /// select is clearable and not read only. It is called "Clear" to what reads the tree, or its `clear_label`.
  pub fn clear(&self) -> Option<Element> {
    if !self.clearable || self.read_only || self.chosen.is_empty() {
      return None;
    }
    let choices = self.choices.clone();
    Some(combobox::clear_control(
      &self.part(MultiSelectPart::Clear),
      &self.part(MultiSelectPart::ClearIcon),
      self.theme(),
      self.disabled,
      &self.clear_label,
      move || choices.clear(),
    ))
  }

  /// The glyph at the end of the box, in the style of its part: a chevron down. The pointer passes through it.
  pub fn chevron(&self) -> Element {
    let part = self.part(MultiSelectPart::Chevron);
    let side = part.rest().width.unwrap_or(Length::Icon(IconSize::Md));
    Row::new()
      .part(&part)
      .flex_shrink(0.0)
      .pointer_events_none()
      .child(Icon::new(icons::CHEVRON_DOWN).size(side).build(self.theme()))
      .into()
  }

  /// The row after the options: the spinner and "Loading results…" while more are loading, "No results" while what is
  /// typed leaves none.
  pub fn status(&self) -> Option<Element> {
    combobox::status_row(
      &self.recipe,
      self.loading,
      self.shown.is_empty(),
      &self.loading_text,
      &self.empty_text,
    )
  }

  /// The engine's multiple select as a combobox around the input, of the options what is typed leaves, in `style`,
  /// at least as high as the box's part and as wide; `trigger` lays out what is in the box.
  pub fn select(
    self,
    style: SelectStyle,
    trigger: impl Fn(SelectTriggerState) -> Element + Send + Sync + 'static,
  ) -> Element {
    let engine = Engine {
      style,
      placeholder: self.placeholder.clone(),
      input: self.input(),
      trigger: Arc::new(trigger),
      status: self.status(),
      free_text: false,
      read_only: self.read_only,
    };
    let select = self.choices.clone().build(&self.shown, engine);
    combobox::finish(
      select,
      &self.part(MultiSelectPart::Trigger).rest(),
      self.theme(),
      Finish {
        fixed_height: false,
        disabled: self.disabled,
        label: self.label.as_ref(),
        name: self.name.as_ref(),
        id: self.id.as_ref(),
        key: self.key.as_ref(),
      },
    )
  }

  /// How the default renderer builds a multi select: the tags and the input after them, which go on to the next
  /// line where they do not fit, then the cross and the chevron.
  pub fn render(self) -> Element {
    let style = self.select_style();
    let gap = self.part(MultiSelectPart::Trigger).rest().gap;
    let tags_gap = self.part(MultiSelectPart::Tags).rest().gap;
    // The least the line of the tags is high, where the recipe says: in a fluid field, a tag's height, so that the
    // box is as high with no tag as with a line of them.
    let least = select::px(self.part(MultiSelectPart::Tags).rest().min_height, self.theme());
    let held = Mutex::new(Some((self.tags(), self.clear(), self.chevron())));
    self.select(style, move |state| {
      let Some((tags, clear, chevron)) = held.lock().unwrap().take() else {
        return Row::new().into();
      };
      let mut line = Row::new()
        .wrap()
        .align_items(Alignment::Center)
        .min_width(0.0)
        .flex(1.0);
      if let Some(gap) = tags_gap {
        line = line.spacing(gap);
      }
      line = line.with_children(tags);
      if let Some(input) = state.input {
        // The input is the one stop of Tab in the line: a tag's cross is pressed, or reached with the arrows from
        // the input, not stopped on.
        line = line.focus_group(FocusGroup::horizontal().one_stop(true)).child(input);
      }
      let mut row = Row::new()
        .width(Dimension::Pct(100.0))
        .align_items(Alignment::Center)
        .child(line);
      if let Some(gap) = gap {
        row = row.spacing(gap);
      }
      if let Some(least) = least {
        row = row.min_height(Dimension::Px(least));
      }
      if let Some(clear) = clear {
        row = row.child(clear);
      }
      row.child(chevron).into()
    })
  }
}

/// A box of the list of checkboxes, in the style of its part, with what it holds in the middle.
fn mark(part: &ResolvedPart) -> Row {
  Row::new()
    .part(part)
    .flex_shrink(0.0)
    .align_items(Alignment::Center)
    .justify(Justify::Center)
}

impl<V: 'static> crate::ui::Styled for MultiSelect<V> {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

/// Every multiselect has the one recipe and renderer, whatever its values are.
impl<V: 'static> Themed for MultiSelect<V> {
  const NAME: &'static str = "multi_select";
  /// Its id is on the trigger: the box that is pressed.
  const ROOT: &'static str = "trigger";
  type Renderer = MultiSelectRenderer;

  fn parts() -> crate::ui::Declared {
    MultiSelectPart::declared()
  }

  /// The design's multi select (`pencil/lurq-design-system.pen`, Multi Select): a select's box 280 wide and at least
  /// 36 high, padded 6 at its start, 10 at its end and 6 over and under, its chosen values as tags 6 apart that go
  /// on to the next line where they do not fit, then a chevron down of 16 in the muted foreground. A tag is the
  /// design's, 24 high in the fill of what is secondary, 4 round, padded 8 before its text of 13 and 6 after its
  /// cross of 12 in the muted foreground. What is typed after the tags keeps 48 on their line. The edge, the ring
  /// and the judgments are a select's. The list is a select's, 280 wide, with a box of 16 before each option, 8
  /// before its label, 4 round: on the background's fill in an edge of the color of an input's, and, chosen, on the
  /// primary fill with a check of 12 in the primary's foreground. The cross that clears is 16, in the muted
  /// foreground, the foreground under the pointer.
  ///
  /// In a fluid field, which the board does not draw for it, the box is a select's fluid one, at least 56 high and
  /// padded 12 at its start, where the field's label starts. Its tags are higher than a line of text, so it is
  /// padded 26 over them, 1 under the label's line, and 6 under them: one line of tags is the 56 of any fluid box, the
  /// tags, what is typed and the chevron in the middle of it about where an input's value stands, and each line more
  /// adds a tag and the 6 between lines.
  fn recipe() -> Recipe {
    use MultiSelectPart::{Check, CheckIcon, Tag, TagLabel, Tags, Trigger, Unchecked, Value};
    use PaletteColor::{Primary, PrimaryForeground, Secondary, SecondaryForeground};
    let edge = |color: PaletteColor| Border::inside(BorderSize::Sm, color);
    let recipe = <select::Select as Themed>::recipe()
      .base(Trigger, |s| {
        s.width(280.0)
          .max_width(Length::Fill)
          .padding_start(6.0)
          .padding_end(10.0)
          .padding_y(6.0)
          .gap(6.0)
          .cursor(CursorIcon::Text)
      })
      .base(Value, |s| s.min_width(48.0))
      .base(Tags, |s| s.gap(6.0))
      .base(Tag, |s| {
        s.padding_start(8.0)
          .padding_end(6.0)
          .gap(4.0)
          .radius(RadiusSize::Sm)
          .background(Secondary)
      })
      .base(TagLabel, |s| {
        s.font_size(13.0)
          .weight(FontWeight::Normal)
          .foreground(SecondaryForeground)
      })
      .base(Check, |s| {
        s.square(16.0)
          .radius(RadiusSize::Sm)
          .background(Primary)
          .border(edge(Primary))
          .foreground(PrimaryForeground)
      })
      .base(CheckIcon, |s| s.square(12.0))
      .base(Unchecked, |s| {
        s.square(16.0)
          .radius(RadiusSize::Sm)
          .background(PaletteColor::Background)
          .border(edge(PaletteColor::Input))
      })
      .size(TextBoxSize::Sm, TagLabel, |s| s.font_size(12.0))
      .size(TextBoxSize::Lg, TagLabel, |s| s.font_size(14.0))
      .state(MultiSelectState::Fluid, Trigger, |s| {
        // 26, a tag of 24 and 6 are the 56 of a fluid box.
        s.padding_start(12.0).padding_top(26.0).padding_bottom(6.0)
      })
      // The line of the tags is as high as a tag of the box's size with no tag in it too.
      .state(MultiSelectState::Fluid, Tags, |s| s.min_height(24.0))
      .when(
        Selector::new().size(TextBoxSize::Sm).state(MultiSelectState::Fluid),
        Tags,
        |s| s.min_height(20.0),
      )
      .when(
        Selector::new().size(TextBoxSize::Lg).state(MultiSelectState::Fluid),
        Tags,
        |s| s.min_height(28.0),
      );
    // The parts of the list and the clear control a combobox adds to a select's, by the same names.
    combobox::clear_rules(combobox::status_rules(recipe))
  }
}
