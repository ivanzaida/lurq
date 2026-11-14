//! A multi select: several values chosen from a list that what is typed filters, held as tags in its box.

use std::sync::{Arc, Mutex};

use super::{
  combobox::{self, Choices, Engine, Filter, Finish, Many},
  field::{FieldControl, InField},
  icon::Icon,
  icons, names,
  part_style::{Length, PartStyle},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  select::{self, engine_style},
  shared::{fading, pressable},
  tag::{Tag, TagPart, TagSize},
  text_box::{self, BoxStates, EditHandler, Judgment, TextBoxSize},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    events::{KeyboardEvent, MouseEvent, TextInputEvent},
    theme::{BorderSize, IconSize, PaletteColor, RadiusSize, Theme},
  },
  components::{self, Row, SelectOption, SelectTriggerState},
  core::{Signal, SignalValue},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{CursorIcon, Element, SelectCheckmarkPosition, SelectIcon, SelectStyle, border::Border, dimension::Dimension},
};

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
  Menu,
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
  Menu => "menu",
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
pub struct MultiSelect {
  choices: Arc<dyn Choices>,
  placeholder: Option<Arc<str>>,
  boxed: BoxStates,
  full_width: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  filter: Filter,
  on_input: Vec<EditHandler>,
  loading: bool,
  empty: Arc<str>,
  loading_text: Arc<str>,
  max_shown: Option<usize>,
  clearable: bool,
  query: Option<Signal<String>>,
  name: Option<Arc<str>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl MultiSelect {
  /// A multi select of `options` bound to `value`: the options whose values the signal has are chosen, and choosing
  /// one adds its value, or takes it out. An option is a `(value, label)` or a [`SelectOption`].
  pub fn new<T>(value: Signal<Vec<T>>, options: impl IntoIterator<Item = impl Into<SelectOption<T>>>) -> Self
  where
    T: Clone + PartialEq + Send + Sync + SignalValue + 'static,
    Vec<T>: SignalValue,
  {
    let options = options.into_iter().map(Into::into).collect();
    Self {
      choices: Arc::new(Many { value, options }),
      placeholder: None,
      boxed: BoxStates::default(),
      full_width: false,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      filter: Arc::new(combobox::contains),
      on_input: Vec::new(),
      loading: false,
      empty: Arc::from("No results"),
      loading_text: Arc::from("Loading results…"),
      max_shown: None,
      clearable: false,
      query: None,
      name: None,
      id: None,
      key: None,
    }
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

  /// A [`ComboboxSize`](super::ComboboxSize), or a size the theme's recipe names: the least height of the box, and
  /// the size of its tags.
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
  pub fn on_input(mut self, handler: impl Fn(&str) + Send + Sync + 'static) -> Self {
    self
      .on_input
      .push(Arc::new(move |event: &TextInputEvent| handler(event.new_value())));
    self
  }

  /// Whether more options are loading: the list ends in a row with a spinner that says so.
  pub fn loading(mut self, loading: bool) -> Self {
    self.loading = loading;
    self
  }

  /// What the list says while what is typed leaves no option: "No results" unless it is told otherwise.
  pub fn empty(mut self, empty: &str) -> Self {
    self.empty = Arc::from(empty);
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

  /// The signal of what is typed, where the app keeps it. Without one the multi select keeps it itself while it is
  /// in the tree.
  pub fn query(mut self, query: Signal<String>) -> Self {
    self.query = Some(query);
    self
  }

  /// Puts the multi select in a state the theme's recipe names ([`ComboboxState`](super::ComboboxState)), or takes
  /// it out.
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
      .state(super::ComboboxState::FullWidth, self.full_width);
    let labels = self.choices.labels();
    let chosen = self.choices.chosen();
    let (query, alive) = match self.query {
      Some(query) => (query, Arc::new(())),
      None => combobox::typed(self.choices.binding(), None),
    };
    let typed_now = query.get();
    let shown = combobox::filtered(&labels, &typed_now, None, &self.filter);
    let view = MultiSelectView {
      placeholder: self.placeholder,
      label: self.boxed.label.clone(),
      invalid: self.boxed.judgment() == Some(Judgment::Invalid),
      disabled: self.boxed.disabled,
      loading: self.loading,
      max_shown: self.max_shown,
      clearable: self.clearable,
      recipe: theme.resolve::<MultiSelect>(&case, Some(&self.rules)),
      case,
      rules: self.rules,
      forced: self.forced,
      choices: self.choices,
      labels,
      chosen,
      shown,
      query,
      alive,
      on_input: self.on_input,
      empty: self.empty,
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

impl From<MultiSelect> for Element {
  fn from(select: MultiSelect) -> Self {
    select.build(&current_theme())
  }
}

impl FieldControl for MultiSelect {
  /// Called by the field's label unless it has a name of its own, judged as the field is unless the app judged it
  /// itself, and disabled where the field is.
  fn in_field(mut self, field: &InField) -> Element {
    self.boxed.in_field(field);
    self.into()
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
  alive: Arc<()>,
  on_input: Vec<EditHandler>,
  empty: Arc<str>,
  loading_text: Arc<str>,
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
    let (check, icon, unchecked) = (
      self.recipe.part(MultiSelectPart::Check),
      self.recipe.part(MultiSelectPart::CheckIcon),
      self.recipe.part(MultiSelectPart::Unchecked),
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
      .min_width(least)
      .flex(1.0);
    let mut control = text_box::text_box(control, &self.boxed, self.on_input.clone());
    // The placeholder says what to do while nothing is chosen; beside tags it would say it again.
    if let Some(placeholder) = self.placeholder.as_ref().filter(|_| self.chosen.is_empty()) {
      control = control.placeholder(placeholder);
    }
    if let Some(id) = &self.id {
      control = control.id(format!("{id}-input"));
    }
    let (query, alive) = (self.query.clone(), self.alive.clone());
    control = control.on_blur(move || {
      // Held while the input is: what was typed is kept for the multi select while it is in the tree.
      let _ = &alive;
      if !query.get_untracked().is_empty() {
        query.set(String::new());
      }
    });
    let (choices, query, last, disabled) = (
      self.choices.clone(),
      self.query.clone(),
      self.chosen.last().copied(),
      self.disabled,
    );
    control.on_key_down(move |event: KeyboardEvent| {
      let plain = !event.ctrl && !event.alt && !event.meta && !event.shift && !event.composing;
      if event.focus_within
        && plain
        && !disabled
        && event.key == "Backspace"
        && query.get_untracked().is_empty()
        && let Some(index) = last
      {
        event.prevent_default();
        choices.unchoose(index);
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
      .map(|index| {
        let (choices, index) = (self.choices.clone(), *index);
        Tag::new(&self.labels[index])
          .pill(true)
          .size(size)
          .disabled(self.disabled)
          .rules(&rules)
          .on_remove(move || choices.unchoose(index))
          .build(&theme)
      })
      .map(apart)
      .collect();
    let more = self.chosen.len().saturating_sub(max);
    if more > 0 {
      tags.push(apart(
        Tag::new(&format!("+{more}"))
          .pill(true)
          .size(size)
          .disabled(self.disabled)
          .rules(&rules)
          .build(&theme),
      ));
    }
    tags
  }

  /// The cross at the end of the box that takes every chosen value away, while something is chosen and the multi
  /// select is clearable. It is called "Clear" to what reads the tree.
  pub fn clear(&self) -> Option<Element> {
    if !self.clearable || self.chosen.is_empty() {
      return None;
    }
    let part = self.part(MultiSelectPart::Clear);
    let side = self
      .part(MultiSelectPart::ClearIcon)
      .rest()
      .width
      .unwrap_or(Length::Icon(IconSize::Sm));
    let choices = self.choices.clone();
    let mut clear = pressable(&part, self.disabled)
      .describe("a11y_role", "button")
      .describe("a11y_name", "Clear")
      .flex_shrink(0.0)
      .child(Icon::new(icons::CLOSE).size(side).build(self.theme()));
    if !self.disabled {
      clear = clear.on_click(move |_| choices.clear());
    }
    Some(apart(clear.into()))
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
      &self.empty,
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
    };
    let select = self.choices.build(&self.shown, engine);
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
        line = line.child(input);
      }
      let mut row = Row::new()
        .width(Dimension::Pct(100.0))
        .align_items(Alignment::Center)
        .child(line);
      if let Some(gap) = gap {
        row = row.spacing(gap);
      }
      if let Some(clear) = clear {
        row = row.child(clear);
      }
      row.child(chevron).into()
    })
  }
}

/// What is pressed in the box without opening its list: a press of a tag's cross takes the tag away, and one of the
/// clear control clears, and neither reaches the select, which a press opens.
fn apart(element: Element) -> Element {
  Row::new()
    .flex_shrink(0.0)
    .overflow_visible()
    .on_mouse_down(|event: MouseEvent| event.stop_propagation())
    .child(element)
    .into()
}

/// A box of the list of checkboxes, in the style of its part, with what it holds in the middle.
fn mark(part: &ResolvedPart) -> Row {
  Row::new()
    .part(part)
    .flex_shrink(0.0)
    .align_items(Alignment::Center)
    .justify(Justify::Center)
}

impl Themed for MultiSelect {
  const NAME: &'static str = "multi_select";
  type Renderer = MultiSelectRenderer;

  /// The design's multi select (`pencil/lurq-design-system.pen`, Multi Select): a select's box 280 wide and at least
  /// 36 high, padded 6 at its start, 10 at its end and 6 over and under, its chosen values as tags 6 apart that go
  /// on to the next line where they do not fit, then a chevron down of 16 in the muted foreground. A tag is the
  /// design's, 24 high in the fill of what is secondary, 4 round, padded 8 before its text of 13 and 6 after its
  /// cross of 12 in the muted foreground. What is typed after the tags keeps 48 on their line. The edge, the ring
  /// and the judgments are a select's. The list is a select's, 280 wide, with a box of 16 before each option, 8
  /// before its label, 4 round: on the background's fill in an edge of the color of an input's, and, chosen, on the
  /// primary fill with a check of 12 in the primary's foreground. The cross that clears is 16, in the muted
  /// foreground, the foreground under the pointer.
  fn recipe() -> Recipe {
    use MultiSelectPart::{Check, CheckIcon, Clear, ClearIcon, Tag, TagLabel, Tags, Trigger, Unchecked, Value};
    use PaletteColor::{Foreground, MutedForeground, Primary, PrimaryForeground, Secondary, SecondaryForeground};
    let edge = |color: PaletteColor| Border::inside(BorderSize::Sm, color);
    let recipe = <select::Select as Themed>::recipe()
      .base(Trigger, |s| {
        s.width(280.0)
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
      .base(Clear, |s| {
        fading(
          s.square(16.0)
            .radius(3.0)
            .foreground(MutedForeground)
            .cursor(CursorIcon::Pointer),
        )
      })
      .base(ClearIcon, |s| s.square(12.0))
      .hovered(Clear, |s| s.foreground(Foreground))
      .focus_visible(Clear, |s| s.focus_ring())
      .size(TextBoxSize::Sm, TagLabel, |s| s.font_size(12.0))
      .size(TextBoxSize::Lg, TagLabel, |s| s.font_size(14.0));
    // The parts of the list a combobox adds to a select's, by the same names.
    combobox::status_rules(recipe)
  }
}
