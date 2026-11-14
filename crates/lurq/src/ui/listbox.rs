//! A listbox: a list in the page of which one option or any number are chosen, with the keys of a listbox.

use std::sync::Arc;

use super::{
  icons, names,
  part_style::{FontFamily, Length, PartStyle},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  selection::{ListSelection, Pick, select_all_key},
  shared::{fading, glyph, pressable},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    events::{KeyboardEvent, MouseEvent},
    theme::{BorderSize, Density, IconSize, OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::{self, Column, Row, ScrollVertical, SelectOption},
  core::{Signal, SignalValue},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{
    CursorIcon, Element, FocusGroup, SelectAdornmentState, border::Border, color::Color, dimension::Dimension,
    outline::Outline,
  },
};

/// The parts of a listbox a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ListboxPart {
  /// The box: its fill, its edge, its corners, its width, and the most it is high, past which its options scroll.
  Root,
  /// The options, one under the other, in the box: the room around them and between them.
  Content,
  /// The label over a group of options.
  Group,
  /// An option: what is pressed, chosen and focused.
  Option,
  /// What the app puts before an option's label: an icon, a mark.
  Leading,
  /// What an option says.
  Label,
  /// The second line under an option's label, where it has one.
  Detail,
  /// The short text at the end of an option that has one: "beta".
  Hint,
  /// What the app puts after an option's label.
  Trailing,
  /// The mark at the end of a chosen option.
  Check,
}

names!(ListboxPart {
  Root => "root",
  Content => "content",
  Group => "group",
  Option => "option",
  Leading => "leading",
  Label => "label",
  Detail => "detail",
  Hint => "hint",
  Trailing => "trailing",
  Check => "check",
});

/// The states of a listbox a recipe styles, besides what the pointer and the keyboard do to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ListboxState {
  /// Of an option: it is chosen, or one of those chosen.
  Chosen,
  /// Of an option: it cannot be chosen.
  OptionDisabled,
  /// The listbox takes no input.
  Disabled,
  /// What is chosen is wrong, or nothing is and something has to be: its edge is in the destructive color.
  Invalid,
  /// Any number of its options may be chosen.
  Multiple,
  /// It is as wide as what holds it.
  FullWidth,
}

names!(ListboxState {
  Chosen => "chosen",
  OptionDisabled => "option_disabled",
  Disabled => "disabled",
  Invalid => "invalid",
  Multiple => "multiple",
  FullWidth => "full_width",
});

/// What builds a listbox's nodes: a theme's in place of the default one.
pub type ListboxRenderer = Arc<dyn Fn(ListboxView) -> Element + Send + Sync>;

/// What a listbox reads and writes: a signal of one value, `Signal<T>`, of which one option is chosen, or of any
/// number, `Signal<Vec<T>>`, which [`Listbox::new`] takes either of, as [`Select`](super::Select) and
/// [`MultiSelect`](super::MultiSelect) take them.
pub struct ListboxValue<T> {
  read: Arc<dyn Fn() -> Vec<T> + Send + Sync>,
  write: Arc<dyn Fn(Vec<T>) + Send + Sync>,
  many: bool,
}

impl<T: Clone + PartialEq + Send + Sync + SignalValue + 'static> From<Signal<T>> for ListboxValue<T> {
  fn from(signal: Signal<T>) -> Self {
    let read = signal.clone();
    Self {
      read: Arc::new(move || vec![read.get()]),
      write: Arc::new(move |chosen: Vec<T>| {
        if let Some(value) = chosen.into_iter().last()
          && signal.get() != value
        {
          signal.set(value);
        }
      }),
      many: false,
    }
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> From<Signal<Vec<T>>> for ListboxValue<T>
where
  Vec<T>: SignalValue,
{
  fn from(signal: Signal<Vec<T>>) -> Self {
    let read = signal.clone();
    Self {
      read: Arc::new(move || read.get()),
      write: Arc::new(move |chosen| signal.set(chosen)),
      many: true,
    }
  }
}

type Told<T> = Arc<dyn Fn(&[T]) + Send + Sync>;

/// A listbox: a list in the page of which one option is chosen, or any number. A renderer, a time zone, the
/// columns a table shows.
///
/// ```
/// use lurq::{components::SelectOption, core::Signal, ui::Listbox};
///
/// let renderer = Signal::new("wgpu");
/// let one = Listbox::new(renderer, [
///   SelectOption::new("wgpu", "wgpu").group("Renderers"),
///   SelectOption::new("dx12", "DirectX 12"),
///   SelectOption::new("metal", "Metal"),
///   SelectOption::new("gl", "OpenGL (legacy)").disabled(true),
/// ])
/// .label("Renderer");
/// // Any number: a `Signal<Vec<_>>`.
/// let columns = Signal::new(vec!["name", "size"]);
/// let many = Listbox::new(columns, [("name", "Name"), ("size", "Size"), ("kind", "Kind")]).label("Columns");
/// ```
///
/// It takes what a [`Select`](super::Select) takes: the app's signal of the value and options of
/// [`SelectOption`] (or `(value, label)`, or [`Choice`](super::Choice)), which may start a group with a label, be
/// disabled, have a detail, a hint, and an element before or after the label. What the signal holds is chosen:
/// with a `Signal<T>` the option of that value, with a `Signal<Vec<T>>` those of its values.
///
/// It is one stop of Tab, at the chosen option or the first. Up and Down move through the options, Home and End to
/// the first and the last, PageUp and PageDown by what the box shows, a letter to the next option that starts
/// with it, and a disabled option is passed by. Of one, the choice follows the focus, as in a radio group, unless
/// [`follow_focus`](Self::follow_focus) is off: then Space and Enter choose. Of many, Space adds the focused option
/// or takes it out, Shift with the arrows, Home, End and the page keys chooses from where the choice started to
/// there, a press with Ctrl adds or takes out one, a press with Shift chooses a range, and Ctrl+A chooses them all.
pub struct Listbox<T> {
  options: Vec<SelectOption<T>>,
  chosen: Vec<T>,
  write: Arc<dyn Fn(Vec<T>) + Send + Sync>,
  many: bool,
  follow_focus: bool,
  on_change: Vec<Told<T>>,
  label: Option<Arc<str>>,
  description: Option<Arc<str>>,
  invalid: bool,
  disabled: bool,
  full_width: bool,
  density: Option<Density>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl<T: Clone + PartialEq + Send + Sync + 'static> Listbox<T> {
  /// A listbox of `options` bound to `value`: a `Signal<T>` of which the option of its value is chosen, and
  /// choosing another sets it; or a `Signal<Vec<T>>` of which the options of its values are, and choosing one adds
  /// it or takes it out.
  pub fn new(value: impl Into<ListboxValue<T>>, options: impl IntoIterator<Item = impl Into<SelectOption<T>>>) -> Self {
    let value = value.into();
    Self {
      options: options.into_iter().map(Into::into).collect(),
      chosen: (value.read)(),
      write: value.write,
      many: value.many,
      follow_focus: true,
      on_change: Vec::new(),
      label: None,
      description: None,
      invalid: false,
      disabled: false,
      full_width: false,
      density: None,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// Of a listbox of one, whether the choice follows the focus: the arrows choose as they move, as in a radio
  /// group. It does unless told otherwise; without it the arrows move the focus alone, and Space, Enter or a press
  /// choose.
  pub fn follow_focus(mut self, follow: bool) -> Self {
    self.follow_focus = follow;
    self
  }

  /// Called with what is chosen after each press or key that changes it, after the signal is set.
  pub fn on_change(mut self, handler: impl Fn(&[T]) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// What the listbox is called, to what reads the tree. The field it is in shows its label.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// Whether what is chosen is wrong, or nothing is and something has to be: its edge is in the destructive color.
  pub fn invalid(mut self, invalid: bool) -> Self {
    self.invalid = invalid;
    self
  }

  /// Whether no option can be chosen.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Whether the listbox is as wide as what holds it, in place of the board's 220.
  pub fn full_width(mut self, full_width: bool) -> Self {
    self.full_width = full_width;
    self
  }

  /// The density its options are at, whatever the theme is at: how high each is.
  pub fn density(mut self, density: Density) -> Self {
    self.density = Some(density);
    self
  }

  /// Puts the listbox in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this listbox over what the theme gives it: `style(ListboxPart::Root, |s| s.max_height(240.0))`
  /// for one whose options scroll.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this listbox.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws every option as it is under the pointer, held down or focused from the keyboard, with nothing pointing
  /// at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the root. Each option's element has the id of the root followed by `-` and its place, from 0.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the listbox in `theme`. Converting a listbox into an element does this in the theme of what is being
  /// rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let base = Case {
      variant: None,
      size: None,
      density: self.density.map(|density| Name::new(density.as_str())),
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(ListboxState::Multiple, self.many)
    .state(ListboxState::Disabled, self.disabled)
    .state(ListboxState::Invalid, self.invalid)
    .state(ListboxState::FullWidth, self.full_width);
    let resolve = |chosen: bool, disabled: bool| {
      let case = base
        .clone()
        .state(ListboxState::Chosen, chosen)
        .state(ListboxState::OptionDisabled, disabled);
      theme.resolve::<Listbox<T>>(&case, Some(&self.rules))
    };
    // An option is in one of four cases: each is resolved once, however many options there are.
    let cases = [
      [resolve(false, false), resolve(false, true)],
      [resolve(true, false), resolve(true, true)],
    ];

    let many = self.many;
    let ids: Vec<T> = self.options.iter().map(|option| option.value().clone()).collect();
    let enabled: Vec<bool> = self
      .options
      .iter()
      .map(|option| !self.disabled && !option.is_disabled())
      .collect();
    // What is chosen of the options, in the app's order: a value no option has is not shown chosen.
    let chosen: Vec<T> = self
      .chosen
      .iter()
      .filter(|value| ids.contains(value))
      .cloned()
      .collect();
    let entry = (0..ids.len()).find(|index| enabled[*index] && chosen.contains(&ids[*index]));
    let write = {
      let (write, on_change) = (self.write.clone(), self.on_change.clone());
      Arc::new(move |next: Vec<T>| {
        write(next.clone());
        for handler in &on_change {
          handler(&next);
        }
      }) as Arc<dyn Fn(Vec<T>) + Send + Sync>
    };
    let selection = Arc::new(ListSelection::new(
      ids,
      enabled.clone(),
      chosen.clone(),
      many,
      Some(write),
    ));

    let mut position = 0;
    let options = self
      .options
      .into_iter()
      .enumerate()
      .map(|(index, option)| {
        let is_chosen = chosen.contains(option.value());
        let disabled = !enabled[index];
        let recipe = cases[usize::from(is_chosen)][usize::from(option.is_disabled())].clone();
        let state = SelectAdornmentState {
          selected: is_chosen,
          disabled,
          ..SelectAdornmentState::default()
        };
        let (leading, trailing) = option.adornments(&state);
        position += 1;
        let pick = (!disabled).then(|| {
          let selection = selection.clone();
          Arc::new(move |pick: Pick| selection.pick(index, pick)) as Arc<dyn Fn(Pick) + Send + Sync>
        });
        let select_all = (!disabled && many).then(|| {
          let selection = selection.clone();
          Arc::new(move || selection.select_all()) as Arc<dyn Fn() + Send + Sync>
        });
        ListboxOption {
          label: Arc::from(option.label()),
          group: option.starts_group().map(Arc::from),
          detail: option.detail_text().cloned(),
          hint: option.hint_text().cloned(),
          leading,
          trailing,
          chosen: is_chosen,
          disabled,
          entry: entry == Some(index),
          position: (position, 0),
          multiple: many,
          recipe,
          forced: self.forced,
          pick,
          select_all,
          id: self.id.as_ref().map(|id| Arc::from(format!("{id}-{index}"))),
        }
      })
      .collect::<Vec<_>>();
    let count = options.len();
    let options = options
      .into_iter()
      .map(|mut option| {
        option.position.1 = count;
        option
      })
      .collect();

    let view = ListboxView {
      options,
      multiple: many,
      follow_focus: self.follow_focus && !many,
      disabled: self.disabled,
      label: self.label,
      description: self.description,
      recipe: cases[0][0].clone(),
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Listbox<T>>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> From<Listbox<T>> for Element {
  fn from(listbox: Listbox<T>) -> Self {
    listbox.build(&current_theme())
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> super::FieldControl for Listbox<T> {
  /// Called by the field's label unless it has a name of its own, judged wrong where the field is unless the app
  /// judged it itself, disabled where the field is, and described by what is under the field.
  fn in_field(mut self, field: &super::InField) -> Element {
    if self.label.is_none() {
      self.label = Some(field.label.clone());
    }
    self.invalid |= field.judgment == Some(super::Judgment::Invalid);
    self.disabled |= field.disabled;
    if self.description.is_none() {
      self.description = field.description.clone();
    }
    self.into()
  }
}

/// A listbox as a renderer sees it: its options, and the style of its box.
pub struct ListboxView {
  /// The options, in order.
  pub options: Vec<ListboxOption>,
  /// Whether any number may be chosen.
  pub multiple: bool,
  /// Whether the choice follows the focus: a listbox of one, unless told otherwise.
  pub follow_focus: bool,
  pub disabled: bool,
  /// What the listbox is called, to what reads the tree.
  pub label: Option<Arc<str>>,
  /// What describes it to what reads the tree: what is under the field it is in.
  pub description: Option<Arc<str>>,
  /// The style of each part of the listbox, its options at rest.
  pub recipe: ResolvedRecipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl ListboxView {
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The box in the style of its root, with `content` in what scrolls in it: a listbox to what reads the tree.
  pub fn root(&self, content: Column) -> Column {
    let scroll = ScrollVertical::new(content)
      .width(Dimension::Pct(100.0))
      .flex_full(0.0, 1.0, None);
    let mut root = Column::new()
      .align_items(Alignment::Stretch)
      .describe("a11y_role", "listbox")
      .part(&self.part(ListboxPart::Root))
      .child(scroll);
    if let Some(label) = &self.label {
      root = root.describe("a11y_name", label.clone());
    }
    if let Some(description) = &self.description {
      root = root.describe("a11y_description", description.clone());
    }
    if self.multiple {
      root = root.describe("a11y_multiselectable", "true");
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
    root
  }

  /// The options' column in the style of its part, with the keys of a listbox: one stop of Tab at the chosen
  /// option or the first, the arrows, Home, End and the page keys through the options without going round, and
  /// typeahead; the choice follows the focus where it does, and Shift with the keys chooses a range of many.
  pub fn content(&self) -> Column {
    Column::new()
      .align_items(Alignment::Stretch)
      .width(Dimension::Pct(100.0))
      .focus_group(
        FocusGroup::vertical()
          .one_stop(true)
          .typeahead(true)
          .wrap(false)
          .pages(true)
          .activate(self.follow_focus)
          .shift_extends(self.multiple),
      )
      .part(&self.part(ListboxPart::Content))
  }

  /// How the default renderer builds a listbox: each group's label over its options, one under the other, in the
  /// box.
  pub fn render(mut self) -> Element {
    let mut content = self.content();
    for option in std::mem::take(&mut self.options) {
      if let Some(group) = option.group_label() {
        content = content.child(group);
      }
      content = content.child(option.render());
    }
    self.root(content).into()
  }
}

/// One option of a listbox as a renderer sees it: what it says, whether it is chosen, the style of its parts, and
/// its behavior ready to be dressed.
pub struct ListboxOption {
  pub label: Arc<str>,
  /// The label of the group it starts, where it starts one.
  pub group: Option<Arc<str>>,
  pub detail: Option<Arc<str>>,
  pub hint: Option<Arc<str>>,
  /// What the app put before its label.
  pub leading: Option<Element>,
  /// What the app put after its label.
  pub trailing: Option<Element>,
  pub chosen: bool,
  pub disabled: bool,
  /// Whether Tab enters the listbox at it: the first chosen option.
  pub entry: bool,
  /// Where it is among the options, from 1, and how many they are.
  pub position: (usize, usize),
  /// Whether any number may be chosen.
  pub multiple: bool,
  /// The style of each part of this option, in its case.
  pub recipe: ResolvedRecipe,
  forced: Option<Interaction>,
  pick: Option<Arc<dyn Fn(Pick) + Send + Sync>>,
  select_all: Option<Arc<dyn Fn() + Send + Sync>>,
  id: Option<Arc<str>>,
}

impl ListboxOption {
  /// The style of a part, as this option is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The label over the group this option starts, where it starts one with a label: a group to what reads the
  /// tree.
  pub fn group_label(&self) -> Option<Row> {
    let label = self.group.as_ref().filter(|label| !label.is_empty())?;
    let part = self.part(ListboxPart::Group);
    Some(
      Row::new()
        .align_items(Alignment::Center)
        .describe("a11y_role", "group")
        .describe("a11y_name", label.clone())
        .part(&part)
        .child(part.text(label).nowrap()),
    )
  }

  /// The option's behavior with the style of its part, with nothing in it yet: a press chooses it, with Ctrl or
  /// Shift where any number may be chosen; Space and Enter choose it, or add it or take it out; Ctrl+A chooses all.
  /// It is an option to what reads the tree, chosen or not.
  pub fn button(&self) -> components::Button {
    let mut option = pressable(&self.part(ListboxPart::Option), self.disabled)
      .justify(Justify::Start)
      .align_items(Alignment::Center)
      .describe("a11y_role", "option")
      .describe("a11y_name", self.label.clone())
      .describe("a11y_selected", if self.chosen { "true" } else { "false" })
      .describe("a11y_posinset", self.position.0.to_string())
      .describe("a11y_setsize", self.position.1.to_string())
      .focus_label(self.label.clone())
      .focus_entry(self.entry);
    if let Some(detail) = &self.detail {
      option = option.describe("a11y_description", detail.clone());
    }
    if let Some(pick) = self.pick.clone() {
      let multiple = self.multiple;
      option = option.on_click(move |event: MouseEvent| pick(Pick::of(&event, multiple)));
    }
    if let Some(select_all) = self.select_all.clone() {
      option = option.on_key_down(move |event: KeyboardEvent| select_all_key(&event, &*select_all));
    }
    if let Some(id) = &self.id {
      option = option.id(id.clone());
    }
    option
  }

  /// The label, with the detail under it where the option has one, as wide as the room the option leaves them.
  pub fn text(&self) -> Column {
    let fill = Dimension::Pct(100.0);
    let label = self.part(ListboxPart::Label);
    let mut text = Column::new().flex(1.0).min_width(Dimension::Px(0.0)).child(
      Column::new()
        .width(fill)
        .min_width(Dimension::Px(0.0))
        .part(&label)
        .child(label.text(&self.label).nowrap()),
    );
    if let Some(detail) = &self.detail {
      let part = self.part(ListboxPart::Detail);
      text = text.child(
        Column::new()
          .width(fill)
          .min_width(Dimension::Px(0.0))
          .part(&part)
          .child(part.text(detail).nowrap()),
      );
    }
    text
  }

  /// The mark at the end of a chosen option: `None` for one that is not chosen.
  pub fn check(&self) -> Option<Element> {
    self
      .chosen
      .then(|| glyph(&self.part(ListboxPart::Check), Name::new(icons::CHECK), false))
  }

  /// How the default renderer builds an option: what the app put before its label, its label and detail, its hint,
  /// what the app put after it, and the mark of a chosen one, side by side.
  pub fn render(mut self) -> Element {
    let mut option = self.button();
    if let Some(leading) = self.leading.take() {
      option = option.child(
        Row::new()
          .align_items(Alignment::Center)
          .part(&self.part(ListboxPart::Leading))
          .child(leading),
      );
    }
    option = option.child(self.text());
    if let Some(hint) = &self.hint {
      let part = self.part(ListboxPart::Hint);
      option = option.child(Row::new().part(&part).child(part.text(hint).nowrap()));
    }
    if let Some(trailing) = self.trailing.take() {
      option = option.child(
        Row::new()
          .align_items(Alignment::Center)
          .part(&self.part(ListboxPart::Trailing))
          .child(trailing),
      );
    }
    if let Some(check) = self.check() {
      option = option.child(check);
    }
    option.into()
  }
}

impl<T: Send + Sync + 'static> Themed for Listbox<T> {
  const NAME: &'static str = "listbox";
  type Renderer = ListboxRenderer;

  /// The design's listbox (`pencil/lurq-design-system.pen`, Listbox): a box of 220 in the background with an edge of
  /// 1 in the border's color and the large radius, its options 1 apart, 4 in from it. An option is 32 high, 8 in
  /// from its sides, with the small radius, its label in the body's type at 13 in the foreground. The chosen one is
  /// in the highlight (the board's `$accent`), its label at 500, with a check of 14 in the foreground at its end;
  /// the one under the pointer is on the muted fill; a disabled one is its label in the muted color, faded. A group's
  /// label is 11 at 600 in the muted color, 6 over it, 4 under it and 8 in. The board draws no option held down or
  /// focused: one held down has the veil of that state, and the focus from the keyboard is the ring inside it. At
  /// each density an option is as high as a sidebar's item is.
  fn recipe() -> Recipe {
    use ListboxPart::{Check, Content, Detail, Group, Hint, Label, Leading, Option, Root, Trailing};
    use ListboxState::{Chosen, Disabled, FullWidth, Invalid, OptionDisabled};
    use PaletteColor::{Background, Destructive, Foreground, Highlight, Muted, MutedForeground, Ring, StatePressed};
    let edge = |color: PaletteColor| Border::inside(BorderSize::Sm, color);
    Recipe::new()
      .base(Root, |s| {
        s.width(220.0)
          .background(Background)
          .border(edge(PaletteColor::Border))
          .radius(RadiusSize::Lg)
          .foreground(Foreground)
      })
      .state(FullWidth, Root, |s| s.width(Length::Fill))
      .state(Invalid, Root, |s| s.border(edge(Destructive)))
      .state(Disabled, Root, |s| s.opacity(OpacityLevel::Disabled))
      .base(Content, |s| s.padding_x(4.0).padding_y(4.0).gap(1.0))
      .base(Option, |s| {
        fading(
          s.height(32.0)
            .padding_x(8.0)
            .gap(8.0)
            .radius(RadiusSize::Sm)
            .cursor(CursorIcon::Pointer),
        )
      })
      .density(Density::Compact, Option, |s| s.height(24.0))
      .density(Density::Comfortable, Option, |s| s.height(40.0))
      .density(Density::Touch, Option, |s| s.height(44.0))
      .hovered(Option, |s| s.background(Muted))
      .pressed(Option, |s| s.veil(StatePressed))
      .focus_visible(Option, |s| s.outline(Outline::new(BorderSize::Md, Ring).offset(-2.0)))
      .state(Chosen, Option, |s| s.background(Highlight))
      .when(Selector::new().state(Chosen).hovered(), Option, |s| s.background(Highlight))
      .state(OptionDisabled, Option, |s| s.cursor(CursorIcon::Default))
      // A disabled option does not answer the pointer: no fill under it.
      .when(Selector::new().state(OptionDisabled).hovered(), Option, |s| {
        s.background(Color::new(0, 0, 0, 0))
      })
      .state(Disabled, Option, |s| s.cursor(CursorIcon::Default))
      .base(Leading, |s| s.gap(4.0).foreground(MutedForeground))
      .base(Label, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(13.0)
          .foreground(Foreground)
      })
      .state(Chosen, Label, |s| s.weight(FontWeight::Medium))
      .state(OptionDisabled, Label, |s| s.foreground(MutedForeground).opacity(0.6))
      .base(Detail, |s| s.typography(TypographyStyle::Caption).foreground(MutedForeground))
      .base(Hint, |s| {
        s.font_family(FontFamily::Mono)
          .font_size(11.0)
          .foreground(MutedForeground)
      })
      .base(Trailing, |s| s.gap(4.0).foreground(MutedForeground))
      .base(Check, |s| s.square(IconSize::Sm).foreground(Foreground))
      .base(Group, |s| {
        s.padding_x(8.0)
          .padding_top(6.0)
          .padding_bottom(4.0)
          .typography(TypographyStyle::Caption)
          .font_size(11.0)
          .weight(FontWeight::SemiBold)
          .foreground(MutedForeground)
      })
  }
}
