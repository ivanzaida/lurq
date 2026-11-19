//! A listbox: a list in the page of which one option or any number are chosen, with the keys of a listbox.

use std::{
  marker::PhantomData,
  sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
  },
};

use super::{
  icons, names,
  part_style::{FontFamily, Length, PartStyle},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  selection::{ListSelection, Pick, select_all_handler},
  shared::{ChangeHandler, fading, glyph, pressable},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    component::Component,
    ctx::Ctx,
    events::MouseEvent,
    theme::{BorderSize, Density, IconSize, OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::{self, Column, Row, ScrollVertical, SelectOption, VirtualizedList},
  core::{Signal, SignalValue},
  layout::{
    Alignment,
    layout_kind::{Justify, ScrollState},
    text_style::FontWeight,
  },
  node::{
    CursorIcon, Element, FocusGroup, FocusRows, SelectAdornmentState, SpacingValue, border::Border, color::Color,
    dimension::Dimension, node_kind::TextOverflow, outline::Outline,
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
  /// Of an option: it has a detail under its label, and is as high as its two lines need.
  Detail,
  /// Its options have a checkbox before their labels in place of the check at their ends
  /// ([`Listbox::checkboxes`]): a chosen one is not in the highlight.
  Checkboxes,
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
  Detail => "detail",
  Checkboxes => "checkboxes",
  Disabled => "disabled",
  Invalid => "invalid",
  Multiple => "multiple",
  FullWidth => "full_width",
});

/// What builds a listbox's nodes: a theme's in place of the default one.
pub type ListboxRenderer = Arc<dyn Fn(ListboxView) -> Element + Send + Sync>;

/// What tells a listbox's handlers the new choice, as the form it was made with has it: the value of a listbox of
/// one, one or none of an optional one, the whole set of one of any number.
type Tell<T, V> = fn(&[T], &[ChangeHandler<V>]);

/// What writes the next choice to where the listbox reads it: the app's signal, or nothing where the app holds it.
type Write<T> = Arc<dyn Fn(Vec<T>) + Send + Sync>;

fn tell_one<T>(next: &[T], handlers: &[ChangeHandler<T>]) {
  if let Some(value) = next.last() {
    for handler in handlers {
      handler(value);
    }
  }
}

fn tell_optional<T: Clone>(next: &[T], handlers: &[ChangeHandler<Option<T>>]) {
  let value = next.last().cloned();
  for handler in handlers {
    handler(&value);
  }
}

fn tell_many<T>(next: &[T], handlers: &[ChangeHandler<[T]>]) {
  for handler in handlers {
    handler(next);
  }
}

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
/// // Nothing chosen until an option is.
/// let theme = Signal::new(None);
/// let optional = Listbox::optional(theme, [("light", "Light"), ("dark", "Dark")]).on_change(|theme| println!("{theme:?}"));
/// // Any number.
/// let columns = Signal::new(vec!["name", "size"]);
/// let many = Listbox::multiple(columns, [("name", "Name"), ("size", "Size"), ("kind", "Kind")])
///   .label("Columns")
///   .on_change(|columns| println!("{} columns", columns.len()));
/// // Held by the app: it is told of a choice, and the listbox shows what it answers with.
/// let held = Listbox::controlled("wgpu", [("wgpu", "wgpu"), ("metal", "Metal")], |renderer| println!("{renderer}"));
/// ```
///
/// It takes what a [`Select`](super::Select) takes: options of [`SelectOption`] (or `(value, label)`, or
/// [`Choice`](super::Choice)), which may start a group with a label, be disabled, have a detail, a hint, and an
/// element before or after the label. Its forms are those of every component that holds a value: bound to the
/// app's signal, [`new`](Self::new) of one, [`optional`](Listbox::optional) of one or none and
/// [`multiple`](Listbox::multiple) of any number; held by the app, [`controlled`](Self::controlled),
/// [`controlled_optional`](Listbox::controlled_optional) and [`controlled_multiple`](Listbox::controlled_multiple).
/// `V` is what the listbox holds, `T`, `Option<T>` or `[T]`, and what [`on_change`](Self::on_change) is told.
///
/// It is one stop of Tab, at the chosen option or the first. Up and Down move through the options, Home and End to
/// the first and the last, PageUp and PageDown by what the box shows, a letter to the next option that starts
/// with it, and a disabled option is passed by. Of one, the choice follows the focus, as in a radio group, unless
/// [`follow_focus`](Self::follow_focus) is off: then Space and Enter choose. Of many, Space adds the focused option
/// or takes it out, Shift with the arrows, Home, End and the page keys chooses from where the choice started to
/// there, a press with Ctrl adds or takes out one, a press with Shift chooses a range, and Ctrl+A chooses them all.
///
/// A listbox of thousands of options is [mounted](Self::mount) in a virtual list, which builds only the options near
/// its view; the keys reach every one.
pub struct Listbox<T, V: ?Sized = T> {
  options: Vec<SelectOption<T>>,
  chosen: Vec<T>,
  write: Write<T>,
  many: bool,
  tell: Tell<T, V>,
  follow_focus: bool,
  checkboxes: bool,
  on_change: Vec<ChangeHandler<V>>,
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

impl<T: Clone + PartialEq + Send + Sync + 'static> Listbox<T, T> {
  /// A listbox of `options` bound to `value`: the option of its value is chosen, and choosing another sets it.
  pub fn new(value: Signal<T>, options: impl IntoIterator<Item = impl Into<SelectOption<T>>>) -> Self
  where
    T: SignalValue,
  {
    let chosen = vec![value.get()];
    let write = Arc::new(move |next: Vec<T>| {
      if let Some(next) = next.into_iter().last()
        && value.get_untracked() != next
      {
        value.set(next);
      }
    });
    Self::of(options, chosen, write, false, tell_one::<T>)
  }

  /// A listbox of one whose value the app holds: the option of `value` is chosen, and a press or a key that would
  /// choose another tells `on_change` the value it is to be. It shows what the app answers with the next time it is
  /// built: a choice the app does not take is not shown.
  pub fn controlled(
    value: T,
    options: impl IntoIterator<Item = impl Into<SelectOption<T>>>,
    on_change: impl Fn(&T) + Send + Sync + 'static,
  ) -> Self {
    Self::of(options, vec![value], Arc::new(|_| {}), false, tell_one::<T>).on_change(on_change)
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> Listbox<T, Option<T>> {
  /// A listbox of one bound to `value`, which may hold none: no option is chosen while it is `None`, and choosing
  /// one sets it to that option's value.
  pub fn optional(value: Signal<Option<T>>, options: impl IntoIterator<Item = impl Into<SelectOption<T>>>) -> Self
  where
    Option<T>: SignalValue,
  {
    let chosen = value.get().into_iter().collect();
    let write = Arc::new(move |next: Vec<T>| {
      let next = next.into_iter().last();
      if next.is_some() && value.get_untracked() != next {
        value.set(next);
      }
    });
    Self::of(options, chosen, write, false, tell_optional::<T>)
  }

  /// A listbox of one or none whose value the app holds: the option of `value` is chosen, none while it is `None`,
  /// and a choice is told to `on_change` as the value it is to be.
  pub fn controlled_optional(
    value: Option<T>,
    options: impl IntoIterator<Item = impl Into<SelectOption<T>>>,
    on_change: impl Fn(&Option<T>) + Send + Sync + 'static,
  ) -> Self {
    let chosen = value.into_iter().collect();
    Self::of(options, chosen, Arc::new(|_| {}), false, tell_optional::<T>).on_change(on_change)
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> Listbox<T, [T]> {
  /// A listbox of any number bound to `values`: the options of its values are chosen, and choosing one adds it or
  /// takes it out.
  pub fn multiple(values: Signal<Vec<T>>, options: impl IntoIterator<Item = impl Into<SelectOption<T>>>) -> Self
  where
    Vec<T>: SignalValue,
  {
    let chosen = values.get();
    let write = Arc::new(move |next: Vec<T>| values.set(next));
    Self::of(options, chosen, write, true, tell_many::<T>)
  }

  /// A listbox of any number whose values the app holds: the options of `values` are chosen, and a press or a key
  /// tells `on_change` the whole set it is to be.
  pub fn controlled_multiple(
    values: impl IntoIterator<Item = T>,
    options: impl IntoIterator<Item = impl Into<SelectOption<T>>>,
    on_change: impl Fn(&[T]) + Send + Sync + 'static,
  ) -> Self {
    let chosen = values.into_iter().collect();
    Self::of(options, chosen, Arc::new(|_| {}), true, tell_many::<T>).on_change(on_change)
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static, V: ?Sized + 'static> Listbox<T, V> {
  fn of(
    options: impl IntoIterator<Item = impl Into<SelectOption<T>>>,
    chosen: Vec<T>,
    write: Write<T>,
    many: bool,
    tell: Tell<T, V>,
  ) -> Self {
    Self {
      options: options.into_iter().map(Into::into).collect(),
      chosen,
      write,
      many,
      tell,
      follow_focus: true,
      checkboxes: false,
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

  /// Of a listbox of any number, whether each option has a checkbox before its label, ticked while it is chosen,
  /// in place of the check at its end and the highlight: a list of things to check, as a transfer's lists are. A
  /// press of an option then checks it or unchecks it and leaves the rest as they are, and a press with Shift, or
  /// Shift with the keys, checks the range from where the checking started and keeps what else is checked.
  pub fn checkboxes(mut self, checkboxes: bool) -> Self {
    self.checkboxes = checkboxes;
    self
  }

  /// Called with what is chosen after each press or key that changes it, after the signal is set: the value of a
  /// listbox of one, `&Option<T>` of an [`optional`](Listbox::optional) one, and the whole new set of one of any
  /// number. It is not called for what is already so (a press of the chosen option), nor when the app sets the
  /// signal. A second handler is called after the first. A handler of another type does not compile:
  ///
  /// ```compile_fail
  /// use lurq::{core::Signal, ui::Listbox};
  ///
  /// let renderer = Signal::new("wgpu");
  /// let listbox = Listbox::new(renderer, [("wgpu", "wgpu")]).on_change(|chosen: &[&str]| {});
  /// ```
  pub fn on_change(mut self, handler: impl Fn(&V) + Send + Sync + 'static) -> Self {
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
    let view = self.view(theme);
    match theme.renderer::<Self>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }

  /// The listbox as a renderer sees it, in `theme`: every option, in order.
  fn view(&self, theme: &Theme) -> ListboxView {
    let base = Case {
      variant: None,
      size: None,
      density: self.density.map(|density| Name::new(density.as_str())),
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(ListboxState::Multiple, self.many)
    .state(ListboxState::Checkboxes, self.checks())
    .state(ListboxState::Disabled, self.disabled)
    .state(ListboxState::Invalid, self.invalid)
    .state(ListboxState::FullWidth, self.full_width);
    let root = theme
      .resolve::<Self>(&base, Some(&self.rules))
      .owned_by(self.id.as_ref());
    let resolve = |chosen: bool, disabled: bool, detail: bool| {
      let case = base
        .clone()
        .state(ListboxState::Chosen, chosen)
        .state(ListboxState::OptionDisabled, disabled)
        .state(ListboxState::Detail, detail);
      theme.resolve::<Self>(&case, Some(&self.rules)).owned_like(&root)
    };
    // An option is in one of eight cases: each is resolved once, however many options there are.
    let cases = [false, true]
      .map(|chosen| [false, true].map(|disabled| [false, true].map(|detail| resolve(chosen, disabled, detail))));

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
      let (write, tell, on_change) = (self.write.clone(), self.tell, self.on_change.clone());
      Arc::new(move |next: Vec<T>| {
        write(next.clone());
        tell(&next, &on_change);
      }) as Write<T>
    };
    let checks = self.checks();
    let selection =
      Arc::new(ListSelection::new(ids, enabled.clone(), chosen.clone(), many, Some(write)).adding(checks));
    // The checkbox of an option is in one of four cases, each resolved once.
    let boxes = checks.then(|| {
      [false, true].map(|checked| {
        [false, true].map(|disabled| {
          theme.resolve::<super::Checkbox>(
            &Case::new()
              .state(super::CheckboxState::Checked, checked)
              .state(super::CheckboxState::Disabled, disabled)
              .breakpoint(super::current_breakpoint()),
            None,
          )
        })
      })
    });

    let mut position = 0;
    let options = self
      .options
      .iter()
      .enumerate()
      .map(|(index, option)| {
        let is_chosen = chosen.contains(option.value());
        let disabled = !enabled[index];
        // An option has an id of its own, which its parts are named after.
        let id: Option<Arc<str>> = self.id.as_ref().map(|id| Arc::from(format!("{id}-{index}")));
        let recipe = cases[usize::from(is_chosen)][usize::from(option.is_disabled())]
          [usize::from(option.detail_text().is_some())]
        .item(id.as_ref(), ListboxPart::Option);
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
          checkbox: boxes.as_ref().map(|boxes| {
            boxes[usize::from(is_chosen)][usize::from(disabled)]
              .clone()
              .owned_like(&recipe)
          }),
          recipe,
          forced: self.forced,
          pick,
          select_all,
          row: None,
          id,
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

    ListboxView {
      options,
      multiple: many,
      follow_focus: self.follow_focus && !many,
      disabled: self.disabled,
      label: self.label.clone(),
      description: self.description.clone(),
      recipe: root,
      id: self.id.clone(),
      key: self.key.clone(),
    }
  }

  /// Whether its options are drawn with checkboxes: of a listbox of any number that was asked to.
  fn checks(&self) -> bool {
    self.checkboxes && self.many
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static, V: ?Sized + 'static> From<Listbox<T, V>> for Element {
  fn from(listbox: Listbox<T, V>) -> Self {
    listbox.build(&current_theme())
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static, V: ?Sized + 'static> super::FieldControl for Listbox<T, V> {
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

  /// Told what the field says, as in `in_field`, and mounted: the same control as outside a field.
  fn mount_in_field(mut self, field: &super::InField, ctx: &mut Ctx) -> Element {
    if self.label.is_none() {
      self.label = Some(field.label.clone());
    }
    self.invalid |= field.judgment == Some(super::Judgment::Invalid);
    self.disabled |= field.disabled;
    if self.description.is_none() {
      self.description = field.description.clone();
    }
    self.mount(ctx)
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
    self.frame().child(scroll)
  }

  /// The box in the style of its root, with nothing in it yet: a listbox to what reads the tree. What
  /// [`root`](Self::root) puts its scroll in, and a [mounted](Listbox::mount) listbox its virtual list.
  pub fn frame(&self) -> Column {
    let mut root = Column::new()
      .align_items(Alignment::Stretch)
      .describe("a11y_role", "listbox")
      .part(&self.part(ListboxPart::Root));
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
#[derive(Clone)]
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
  /// The style of each part of the checkbox before its label, in its case: where the listbox has
  /// [`checkboxes`](Listbox::checkboxes).
  pub checkbox: Option<ResolvedRecipe>,
  /// The style of each part of this option, in its case.
  pub recipe: ResolvedRecipe,
  forced: Option<Interaction>,
  pick: Option<Arc<dyn Fn(Pick) + Send + Sync>>,
  select_all: Option<Arc<dyn Fn() + Send + Sync>>,
  /// Which row of the group's rows it is, in a listbox in a virtual list.
  row: Option<usize>,
  id: Option<Arc<str>>,
}

impl ListboxOption {
  /// The option as an option component of a virtual list builds it again, from render to render: its parts named
  /// after its id from naught each time, so that they are named alike.
  fn again(mut self) -> Self {
    self.recipe = self.recipe.item(self.id.as_ref(), ListboxPart::Option);
    self.checkbox = self.checkbox.map(|checkbox| checkbox.owned_like(&self.recipe));
    self
  }

  /// The style of a part, as this option is to be drawn.
  /// The style of a part, as this option is to be drawn: a disabled one as nothing points at it or presses it,
  /// whatever the listbox is forced to.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self
      .recipe
      .part(part)
      .forced(if self.disabled { None } else { self.forced })
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
    if let Some(row) = self.row {
      option = option.focus_row(row);
    }
    if self.disabled {
      option = option.describe("a11y_disabled", "true");
    }
    if let Some(detail) = &self.detail {
      option = option.describe("a11y_description", detail.clone());
    }
    if self.checkbox.is_some() {
      option = option.describe("a11y_checked", if self.chosen { "true" } else { "false" });
    }
    if let Some(pick) = self.pick.clone() {
      let (multiple, checks) = (self.multiple, self.checkbox.is_some());
      option = option.on_click(move |event: MouseEvent| {
        pick(if checks {
          Pick::of_check(&event)
        } else {
          Pick::of(&event, multiple)
        })
      });
    }
    if let Some(select_all) = self.select_all.clone() {
      option = option.on_select_all(select_all_handler(select_all));
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
        .child(label.text(&self.label).nowrap().text_overflow(TextOverflow::Elipsis)),
    );
    if let Some(detail) = &self.detail {
      let part = self.part(ListboxPart::Detail);
      text = text.child(
        Column::new()
          .width(fill)
          .min_width(Dimension::Px(0.0))
          .part(&part)
          .child(part.text(detail).nowrap().text_overflow(TextOverflow::Elipsis)),
      );
    }
    text
  }

  /// The mark at the end of a chosen option: `None` for one that is not chosen, and for an option of a listbox
  /// with checkboxes, whose checkbox says it.
  pub fn check(&self) -> Option<Element> {
    (self.chosen && self.checkbox.is_none())
      .then(|| glyph(&self.part(ListboxPart::Check), Name::new(icons::CHECK), false))
  }

  /// The checkbox before the label of an option of a listbox with [`checkboxes`](Listbox::checkboxes): the box of
  /// a [`Checkbox`](super::Checkbox), ticked while the option is chosen. It is drawn, not pressed: the option is.
  pub fn checkbox(&self) -> Option<Element> {
    let checkbox = self.checkbox.as_ref()?;
    let mut square = Row::new()
      .justify(Justify::Center)
      .align_items(Alignment::Center)
      .flex_shrink(0.0)
      .pointer_events_none()
      .part(&checkbox.part(super::CheckboxPart::Box));
    if self.chosen {
      square = square.child(glyph(
        &checkbox.part(super::CheckboxPart::Indicator),
        Name::new(icons::CHECK),
        false,
      ));
    }
    Some(square.into())
  }

  /// How the default renderer builds an option: its checkbox where the listbox has them, what the app put before its
  /// label, its label and detail, its hint, what the app put after it, and the mark of a chosen one, side by side.
  pub fn render(mut self) -> Element {
    let mut option = self.button();
    if let Some(checkbox) = self.checkbox() {
      option = option.child(checkbox);
    }
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

impl<T: Clone + PartialEq + Send + Sync + 'static, V: ?Sized + 'static> crate::ui::Styled for Listbox<T, V> {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl<T: Send + Sync + 'static, V: ?Sized + 'static> Themed for Listbox<T, V> {
  const NAME: &'static str = "listbox";
  type Renderer = ListboxRenderer;

  fn parts() -> crate::ui::Declared {
    ListboxPart::declared()
  }

  fn states() -> crate::ui::Declared {
    ListboxState::declared()
  }

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
    let detail = ListboxState::Detail;
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
      // With a detail under its label: 16 higher at every density, as a list's row with a description is.
      .state(detail, Option, |s| s.height(48.0))
      .when(Selector::new().density(Density::Compact).state(detail), Option, |s| s.height(40.0))
      .when(Selector::new().density(Density::Comfortable).state(detail), Option, |s| s.height(56.0))
      .when(Selector::new().density(Density::Touch).state(detail), Option, |s| s.height(60.0))
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
      // With checkboxes, the checkbox says what is chosen: no highlight and no heavier label.
      .when(Selector::new().state(Chosen).state(ListboxState::Checkboxes), Option, |s| {
        s.background(Color::new(0, 0, 0, 0))
      })
      .when(
        Selector::new().state(Chosen).state(ListboxState::Checkboxes).hovered(),
        Option,
        |s| s.background(Muted),
      )
      .when(Selector::new().state(Chosen).state(ListboxState::Checkboxes), Label, |s| {
        s.weight(FontWeight::Normal)
      })
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

impl<T: Clone + PartialEq + Send + Sync + 'static, V: ?Sized + 'static> Listbox<T, V> {
  /// The listbox in the engine's virtual list, for thousands of options: only the options near its view are built,
  /// and the keys reach every option, built or not. Up, Down, Home, End, PageUp, PageDown and typeahead step over
  /// every option that can be chosen (the engine's [`FocusRows`](crate::node::FocusRows)); the list builds and brings
  /// into view the one they go to, and the focus goes to it once it is built. The choice follows the focus there as
  /// it does in a listbox that builds every option, and Shift with the keys and Ctrl+A choose over every option.
  /// The list opens at the chosen option.
  ///
  /// The list fills the box, which is to be given a height: `.style(ListboxPart::Root, |s| s.height(..))`, or the
  /// room of what holds it. The options are drawn by [`ListboxOption::render`], each with its group's label over it;
  /// a theme's renderer of the whole listbox is not used, its rules are.
  ///
  /// ```
  /// use lurq::{app::ctx::Ctx, core::Signal, node::Element, ui::{Listbox, ListboxPart}};
  ///
  /// # fn zones(ctx: &mut Ctx, zone: Signal<u32>, names: Vec<String>) -> Element {
  /// Listbox::new(zone, names.iter().enumerate().map(|(index, name)| (index as u32, name.as_str())))
  ///   .label("Time zone")
  ///   .style(ListboxPart::Root, |s| s.height(320.0))
  ///   .mount(ctx)
  /// # }
  /// ```
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    ctx.mount::<ListboxHost<T, V>>(ListboxHostProps { listbox: self })
  }
}

struct ListboxHostProps<T, V: ?Sized> {
  listbox: Listbox<T, V>,
}

impl<T, V: ?Sized> PartialEq for ListboxHostProps<T, V> {
  /// A listbox built again is drawn again: what is chosen is read from the app's signal as it is built.
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

impl<T, V: ?Sized> crate::app::component::DevtoolsInspectable for ListboxHostProps<T, V> {
  fn inspect(&self, formatter: &mut crate::app::component::DevtoolsFormatter<'_>) {
    formatter.value("Listbox", format!("{} options", self.listbox.options.len()));
  }
}

/// The options a listbox in a virtual list built last, which its option components draw.
#[derive(Default)]
struct Shown {
  options: Vec<ListboxOption>,
  gap: SpacingValue,
}

/// The options of a listbox in a virtual list as the engine's focus group steps over them.
struct HostRows {
  data: Mutex<RowsData>,
  reveal_key: Signal<Option<String>>,
  reveal_request: Signal<u64>,
  scroll: ScrollState,
}

#[derive(Default)]
struct RowsData {
  labels: Vec<Arc<str>>,
  enabled: Vec<bool>,
  /// The height of an option and the room after it.
  step: f32,
}

impl FocusRows for HostRows {
  fn count(&self) -> usize {
    self.data.lock().unwrap().labels.len()
  }

  fn label(&self, row: usize) -> Option<String> {
    self.data.lock().unwrap().labels.get(row).map(|label| label.to_string())
  }

  fn enabled(&self, row: usize) -> bool {
    self.data.lock().unwrap().enabled.get(row).copied().unwrap_or(false)
  }

  fn page(&self) -> usize {
    let step = self.data.lock().unwrap().step.max(1.0);
    ((self.scroll.viewport_height() / step).floor() as usize).max(1)
  }

  fn reveal(&self, row: usize) {
    if row >= self.count() {
      return;
    }
    self.reveal_key.set(Some(row.to_string()));
    self.reveal_request.update(|request| *request = request.wrapping_add(1));
  }
}

/// A listbox in a virtual list: its box holds the options' column, the focus group, which holds the list, and the
/// list an option component for each option near its view.
struct ListboxHost<T, V: ?Sized> {
  shown: Arc<Mutex<Shown>>,
  rows: Arc<HostRows>,
  generation: AtomicU64,
  _values: PhantomData<fn(&V) -> T>,
}

impl<T: Clone + PartialEq + Send + Sync + 'static, V: ?Sized + 'static> Component for ListboxHost<T, V> {
  type Props = ListboxHostProps<T, V>;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      shown: Arc::default(),
      rows: Arc::new(HostRows {
        data: Mutex::default(),
        reveal_key: ctx.signal(None),
        reveal_request: ctx.signal(0),
        scroll: ScrollState::new(),
      }),
      generation: AtomicU64::new(0),
      _values: PhantomData,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let theme = current_theme();
    let mut view = ctx.props::<ListboxHostProps<T, V>>().listbox.view(&theme);
    for (index, option) in view.options.iter_mut().enumerate() {
      option.row = Some(index);
    }
    let (step, group) = option_steps(&view);
    let plain = px_height(&view.part(ListboxPart::Option), &theme).unwrap_or(32.0);
    *self.rows.data.lock().unwrap() = RowsData {
      labels: view.options.iter().map(|option| option.label.clone()).collect(),
      enabled: view.options.iter().map(|option| !option.disabled).collect(),
      step,
    };
    // Until the keys ask for an option, the list opens at the chosen one.
    let entry = view
      .options
      .iter()
      .position(|option| option.entry)
      .map(|index| index.to_string());
    // Every option component draws again from what this render built: what is chosen.
    let generation = self.generation.fetch_add(1, Ordering::Relaxed) + 1;
    let items: Arc<Vec<OptionItem>> = Arc::new(
      view
        .options
        .iter()
        .enumerate()
        .map(|(index, option)| OptionItem {
          index,
          grouped: option.group.as_ref().is_some_and(|label| !label.is_empty()),
          // An option with a detail is higher than one of one line.
          taller: px_height(&option.part(ListboxPart::Option), &theme).map_or(0.0, |height| (height - plain).max(0.0)),
          generation,
        })
        .collect(),
    );
    *self.shown.lock().unwrap() = Shown {
      options: std::mem::take(&mut view.options),
      gap: view.part(ListboxPart::Content).rest().gap.unwrap_or_default(),
    };
    let shown = self.shown.clone();
    let list = VirtualizedList::new_shared(ctx, items)
      .with_scroll_state(self.rows.scroll.clone())
      .reveal_key(self.rows.reveal_key.get().or(entry))
      .reveal_request(self.rows.reveal_request.get())
      .estimate_height(move |item: &OptionItem| step + item.taller + if item.grouped { group } else { 0.0 })
      .width(Dimension::Pct(100.0))
      .flex(1.0)
      .mount_keyed::<ListboxOptionHost, _, _, _>(
        |item: &OptionItem| item.index.to_string(),
        move |item: &OptionItem| ListboxOptionProps {
          index: item.index,
          generation: item.generation,
          shown: shown.clone(),
        },
      );
    let rows: Arc<dyn FocusRows> = self.rows.clone();
    let content = view
      .content()
      .flex(1.0)
      .min_height(Dimension::Px(0.0))
      .focus_rows(rows)
      .child(list);
    view.frame().child(content)
  }
}

/// The height of an option and the room after it, and the height of a group's label, as the recipe gives them:
/// what the list places options it has not built by, and what a page of options is.
fn option_steps(view: &ListboxView) -> (f32, f32) {
  let theme = view.theme();
  let spacing = theme.spacing();
  let px = |value: Option<&SpacingValue>| value.map(|value| value.resolve(&spacing, 0.0)).unwrap_or(0.0);
  let option = px_height(&view.part(ListboxPart::Option), theme).unwrap_or(32.0);
  let content = view.part(ListboxPart::Content).rest();
  let group = view.part(ListboxPart::Group).rest();
  // A line of its text, as the type sets it: about half as high again as the letters.
  let line = group.text.size.unwrap_or(11.0) * 1.5;
  (
    option + px(content.gap.as_ref()),
    px(group.padding_top.as_ref()) + px(group.padding_bottom.as_ref()) + line,
  )
}

/// The height a part's recipe gives it in pixels, where it gives one.
fn px_height(part: &ResolvedPart, theme: &Theme) -> Option<f32> {
  match part.rest().height?.resolve(theme)? {
    Dimension::Px(px) => Some(px),
    _ => None,
  }
}

/// An option of the virtual list: which of the options, from which render, whether a group's label is over it, and
/// how much higher than an option of one line it is.
#[derive(Clone, PartialEq)]
struct OptionItem {
  index: usize,
  grouped: bool,
  taller: f32,
  generation: u64,
}

struct ListboxOptionProps {
  index: usize,
  generation: u64,
  shown: Arc<Mutex<Shown>>,
}

impl PartialEq for ListboxOptionProps {
  fn eq(&self, other: &Self) -> bool {
    self.index == other.index && self.generation == other.generation && Arc::ptr_eq(&self.shown, &other.shown)
  }
}

impl crate::app::component::DevtoolsInspectable for ListboxOptionProps {
  fn inspect(&self, formatter: &mut crate::app::component::DevtoolsFormatter<'_>) {
    formatter.value("ListboxOption", format!("option {}", self.index));
  }
}

/// One option of a listbox in a virtual list, with its group's label over it where it starts one, and the room
/// after it that the options have between them.
struct ListboxOptionHost;

impl Component for ListboxOptionHost {
  type Props = ListboxOptionProps;

  fn create(_: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<ListboxOptionProps>();
    let (option, gap) = {
      let shown = props.shown.lock().unwrap();
      (shown.options.get(props.index).cloned(), shown.gap.clone())
    };
    let Some(option) = option.map(ListboxOption::again) else {
      return Column::new();
    };
    let mut column = Column::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Stretch);
    if let Some(group) = option.group_label() {
      column = column.child(group);
    }
    column.child(
      Column::new()
        .align_items(Alignment::Stretch)
        .spacing(gap)
        .child(option.render())
        // Nothing, after the room under the option.
        .child(Row::new()),
    )
  }
}
