//! A swatch picker: one color chosen among swatches laid out in a grid.

use std::sync::Arc;

use super::{
  color_swatch::{ColorSwatch, ColorSwatchPart, ColorSwatchSize, ColorSwatchState, color_over_checker},
  names,
  part_style::{Length, PartStyle},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::{ChangeHandler, Chosen, pressable},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    events::KeyboardEvent,
    theme::{BorderSize, PaletteColor, RadiusSize, ShadowStyle, Theme},
  },
  components::{Column, Row},
  core::{Signal, SignalValue},
  layout::layout_kind::Justify,
  node::{Element, FocusGroup, border::Border, color::Color, dimension::Dimension},
};

/// The parts of a swatch picker a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SwatchPickerPart {
  /// The panel: its fill, its edge, its corners, its shadow, its padding, and the room between its rows.
  Root,
  /// A row of swatches: the room between them.
  Row,
  /// What is laid over each swatch's root ([`ColorSwatchPart::Root`]): its side, its edge, its ring.
  Swatch,
}

names!(SwatchPickerPart {
  Root => "root",
  Row => "row",
  Swatch => "swatch",
});

/// The states of a swatch picker a recipe styles, besides what the pointer and the keyboard do to its swatches. A
/// swatch's own (`Selected`, `Disabled`) are [`ColorSwatchState`]'s, resolved on the swatch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SwatchPickerState {
  /// It is a panel of its own: a fill, an edge, a shadow, the padding. Out of it, the bare grid, to stand in a
  /// popover or a picker that frames it.
  Framed,
  /// What is chosen is wrong: a form says so.
  Invalid,
  /// It takes no input.
  Disabled,
}

names!(SwatchPickerState {
  Framed => "framed",
  Invalid => "invalid",
  Disabled => "disabled",
});

/// What builds a swatch picker's nodes: a theme's in place of the default one.
pub type SwatchPickerRenderer = Arc<dyn Fn(SwatchPickerView) -> Element + Send + Sync>;

/// One swatch of a [`SwatchPicker`]: a color, the value choosing it gives (the color itself, or the app's), what
/// it is called, and whether it can be chosen.
#[derive(Clone)]
pub struct Swatch<T = Color> {
  value: T,
  color: Color,
  label: Option<Arc<str>>,
  disabled: bool,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Swatch<Color> {
  /// A swatch of `color`, whose value is the color.
  pub fn new(color: impl Into<Color>) -> Self {
    let color = color.into();
    Self::of(color, color)
  }
}

impl<T> Swatch<T> {
  /// A swatch of `color` whose value is the app's `value`: a name of the theme's palette, a tag's color.
  pub fn of(value: T, color: impl Into<Color>) -> Self {
    Self {
      value,
      color: color.into(),
      label: None,
      disabled: false,
      id: None,
      key: None,
    }
  }

  /// What the swatch is called, to what reads the tree: "Red". Its color's hex where it is not given.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// Whether it cannot be chosen: dimmed, and the keys pass it by.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// The id of the swatch's node.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// What the swatch is told apart by among the swatches, in place of its place: what the keys move the focus to.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  fn map<U>(self, map: impl FnOnce(T) -> U) -> Swatch<U> {
    Swatch {
      value: map(self.value),
      color: self.color,
      label: self.label,
      disabled: self.disabled,
      id: self.id,
      key: self.key,
    }
  }
}

impl From<Color> for Swatch<Color> {
  fn from(color: Color) -> Self {
    Self::new(color)
  }
}

/// A swatch of the color a hex writes: `"#EF4444"`. It panics where the hex is no color, as [`Color::from_hex`]
/// does.
impl From<&str> for Swatch<Color> {
  fn from(hex: &str) -> Self {
    Self::new(Color::from_hex(hex))
  }
}

/// One color chosen among swatches in a grid: a color of a label, of a highlight, of a calendar's event.
///
/// ```
/// use lurq::{core::Signal, node::color::Color, ui::{Swatch, SwatchPicker}};
///
/// # fn colors(chosen: Signal<Color>, tag: Signal<Option<&'static str>>) {
/// let picker = SwatchPicker::new(chosen, ["#EF4444", "#F97316", "#F59E0B", "#22C55E", "#3B82F6", "#A855F7"])
///   .columns(3)
///   .label("Label color");
/// // The app's values, each with its color; none chosen yet.
/// let named = SwatchPicker::optional(tag, [
///   Swatch::of("red", Color::from_hex("#EF4444")).label("Red"),
///   Swatch::of("blue", Color::from_hex("#3B82F6")).label("Blue"),
/// ]);
/// // Held by the app: it is told what is chosen, and shows what it answers with.
/// let held = SwatchPicker::controlled(Color::from_hex("#EF4444"), ["#EF4444", "#3B82F6"], |color| {
///   println!("{}", color.to_hex_upper())
/// });
/// # }
/// ```
///
/// Exactly one swatch is chosen, ringed in the foreground. The grid is one stop of Tab, at the chosen swatch (or the
/// first); the arrows move the focus in two directions, Home and End to the first and the last, and Enter, Space or
/// a click choose. A disabled swatch is passed by. [`on_change`](Self::on_change) is told the value a person chooses,
/// a `&T` the compiler checks. `T` is the color itself unless the swatches carry the app's values
/// ([`Swatch::of`]).
pub struct SwatchPicker<T = Color> {
  swatches: Vec<Swatch<T>>,
  chosen: Chosen<T>,
  on_change: Vec<ChangeHandler<T>>,
  columns: usize,
  framed: bool,
  size: Name,
  label: Option<Arc<str>>,
  invalid: bool,
  disabled: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl<T: Clone + PartialEq + Send + Sync + 'static> SwatchPicker<T> {
  fn of(chosen: Chosen<T>, swatches: impl IntoIterator<Item = impl Into<Swatch<T>>>) -> Self {
    Self {
      swatches: swatches.into_iter().map(Into::into).collect(),
      chosen,
      on_change: Vec::new(),
      columns: 6,
      framed: true,
      size: ColorSwatchSize::default().into(),
      label: None,
      invalid: false,
      disabled: false,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// A picker whose chosen swatch is the one with the signal's value. Choosing another sets the signal.
  pub fn new(value: Signal<T>, swatches: impl IntoIterator<Item = impl Into<Swatch<T>>>) -> Self
  where
    T: SignalValue,
  {
    Self::of(Chosen::one(value), swatches)
  }

  /// A picker whose value the app holds: the swatch of `value` is the chosen one, and choosing another tells
  /// `on_change` the value it is to be; the picker shows what the app then gives. It keeps nothing between renders,
  /// so two built at one place in a loop need nothing to be told apart.
  pub fn controlled(
    value: T,
    swatches: impl IntoIterator<Item = impl Into<Swatch<T>>>,
    on_change: impl Fn(&T) + Send + Sync + 'static,
  ) -> Self {
    Self::of(Chosen::held_one(value), swatches).on_change(on_change)
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> SwatchPicker<Option<T>> {
  /// A picker bound to `value`, which may hold none: then no swatch is chosen, and Tab enters at the first.
  /// Choosing one sets the signal to it.
  pub fn optional(value: Signal<Option<T>>, swatches: impl IntoIterator<Item = impl Into<Swatch<T>>>) -> Self
  where
    Option<T>: SignalValue,
  {
    Self::of(
      Chosen::one(value),
      swatches
        .into_iter()
        .map(|swatch| Into::<Swatch<T>>::into(swatch).map(Some)),
    )
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> SwatchPicker<T> {
  /// Called as a person chooses another swatch, with its value (`|color: &Color| ..`). Not called where the app
  /// sets the signal, nor for the swatch chosen already. Each call adds a handler.
  pub fn on_change(mut self, handler: impl Fn(&T) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// Swatches after those it has, one more row each `columns` of them.
  pub fn swatches(mut self, swatches: impl IntoIterator<Item = impl Into<Swatch<T>>>) -> Self {
    self.swatches.extend(swatches.into_iter().map(Into::into));
    self
  }

  /// A swatch after those it has.
  pub fn swatch(mut self, swatch: impl Into<Swatch<T>>) -> Self {
    self.swatches.push(swatch.into());
    self
  }

  /// How many swatches a row holds: 6, the board's, unless given. Up and Down move the focus by as many.
  pub fn columns(mut self, columns: usize) -> Self {
    self.columns = columns.max(1);
    self
  }

  /// Whether it is a panel of its own (the default): the popover's fill, an edge, the large radius, a shadow and a
  /// padding of 10. Out of it the grid alone, for a popover or a picker that frames it.
  pub fn framed(mut self, framed: bool) -> Self {
    self.framed = framed;
    self
  }

  /// The size of its swatches: a [`ColorSwatchSize`], `Md` 24 (the board's) unless given.
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.size = size.into();
    self
  }

  /// What the picker is called to what reads the tree: what its swatches are a choice of. It draws nothing.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// Whether what is chosen is wrong: the panel's edge is in the destructive color.
  pub fn invalid(mut self, invalid: bool) -> Self {
    self.invalid = invalid;
    self
  }

  /// Whether no swatch takes input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Puts the picker in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this picker over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this picker: what the component it is in says of it in this place.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the swatch Tab would stop on (the chosen one, else the first that is not disabled) as it is under the
  /// pointer, held down or focused from the keyboard, with nothing pointing at it: for a gallery or a test. The
  /// other swatches are drawn as they are.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the root. A swatch has an id of its own where it is given one.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the picker in `theme`. Converting a picker into an element does this in the theme of what is being
  /// rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let mut case = Case::new()
      .breakpoint(super::current_breakpoint())
      .state(SwatchPickerState::Framed, self.framed)
      .state(SwatchPickerState::Invalid, self.invalid)
      .state(SwatchPickerState::Disabled, self.disabled);
    for state in &self.states {
      case = case.state(state.clone(), true);
    }
    let recipe = theme
      .resolve::<SwatchPicker>(&case, Some(&self.rules))
      .owned_by(self.id.as_ref());
    let swatch_rules = recipe.rules_for(SwatchPickerPart::Swatch, ColorSwatchPart::Root);
    let handlers: Arc<[ChangeHandler<T>]> = self.on_change.into();
    let items: Vec<SwatchItem> = self
      .swatches
      .into_iter()
      .enumerate()
      .map(|(index, swatch)| {
        let selected = self.chosen.has(&swatch.value);
        let disabled = self.disabled || swatch.disabled;
        let swatch_case = Case::new()
          .size(self.size.clone())
          .breakpoint(super::current_breakpoint())
          .state(ColorSwatchState::Selected, selected)
          .state(ColorSwatchState::Pressable, true)
          .state(ColorSwatchState::Disabled, disabled);
        SwatchItem {
          color: swatch.color,
          label: swatch.label.unwrap_or_else(|| Arc::from(swatch.color.to_hex_upper())),
          selected,
          disabled,
          // A swatch the app gave an id names its parts after it; another is named after the part: `x-swatch-2`.
          recipe: theme
            .resolve::<ColorSwatch>(&swatch_case, Some(&swatch_rules))
            .owned_by(swatch.id.as_ref()),
          choose: self.chosen.picker(swatch.value, handlers.clone()),
          id: swatch.id,
          key: swatch.key.unwrap_or_else(|| Arc::from(format!("swatch-{index}"))),
        }
      })
      .collect();
    let view = SwatchPickerView {
      swatches: items,
      columns: self.columns,
      label: self.label,
      invalid: self.invalid,
      disabled: self.disabled,
      recipe,
      forced: self.forced,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<SwatchPicker>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> From<SwatchPicker<T>> for Element {
  fn from(picker: SwatchPicker<T>) -> Self {
    picker.build(&current_theme())
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> super::FieldControl for SwatchPicker<T> {
  /// Called by the field's label unless it has a name of its own, judged invalid where the field is, and disabled
  /// where the field is.
  fn in_field(mut self, field: &super::InField) -> Element {
    if self.label.is_none() {
      self.label = Some(field.label.clone());
    }
    self.invalid |= field.judgment == Some(super::text_box::Judgment::Invalid);
    self.disabled |= field.disabled;
    self.into()
  }
}

/// One swatch as a renderer sees it: its color, what it is called, whether it is chosen, the style of its parts
/// (a [`ColorSwatch`]'s), and what choosing it does.
pub struct SwatchItem {
  pub color: Color,
  /// What it is called to what reads the tree: its label, or its color's hex.
  pub label: Arc<str>,
  pub selected: bool,
  pub disabled: bool,
  /// The style of each part of the swatch, a [`ColorSwatch`]'s in its states.
  pub recipe: ResolvedRecipe,
  choose: Arc<dyn Fn() + Send + Sync>,
  id: Option<Arc<str>>,
  key: Arc<str>,
}

/// A swatch picker as a renderer sees it: its swatches, the style of its parts, and its behavior ready to be dressed.
pub struct SwatchPickerView {
  pub swatches: Vec<SwatchItem>,
  /// How many swatches a row holds.
  pub columns: usize,
  /// What it is called, to what reads the tree.
  pub label: Option<Arc<str>>,
  pub invalid: bool,
  pub disabled: bool,
  /// The style of each part of the picker.
  pub recipe: ResolvedRecipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl SwatchPickerView {
  /// The style of a part of the picker.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The swatch Tab enters the grid at: the chosen one, else the first that is not disabled.
  pub fn entry(&self) -> Option<usize> {
    self
      .swatches
      .iter()
      .position(|swatch| swatch.selected && !swatch.disabled)
      .or_else(|| self.swatches.iter().position(|swatch| !swatch.disabled))
  }

  /// The swatch Up (`-1`) or Down (`1`) takes the focus to from `index`: the one a row up or down, passing the
  /// disabled ones by row after row; none past the first or the last row.
  fn vertical(&self, index: usize, by: isize) -> Option<usize> {
    let mut at = index as isize;
    loop {
      at += by * self.columns as isize;
      let swatch = self.swatches.get(usize::try_from(at).ok()?)?;
      if !swatch.disabled {
        return Some(at as usize);
      }
    }
  }

  /// The swatch at `index`: a [`ColorSwatch`] in its states that a press, Enter or Space chooses, an `option` to what
  /// reads the tree. Up and Down move the focus a row; the group the picker is moves it along a row.
  pub fn swatch(&self, index: usize) -> Element {
    let item = &self.swatches[index];
    let forced = self.forced.filter(|_| self.entry() == Some(index));
    let part = |part: ColorSwatchPart| item.recipe.part(part).forced(forced);
    let root = part(ColorSwatchPart::Root);
    let side = |length: Option<Length>| match length.and_then(|length| length.resolve(self.theme())) {
      Some(Dimension::Px(px)) => px,
      _ => 24.0,
    };
    let (width, height) = (side(root.rest().width), side(root.rest().height));
    let mut swatch = pressable(&root, item.disabled)
      .justify(Justify::Start)
      .key(item.key.clone())
      .focus_entry(self.entry() == Some(index))
      .child(color_over_checker(
        item.color,
        width,
        height,
        &part(ColorSwatchPart::Checker),
        &part(ColorSwatchPart::CheckerCell),
      ))
      .describe("a11y_role", "option")
      .describe("a11y_name", item.label.clone())
      .describe("a11y_selected", if item.selected { "true" } else { "false" });
    if item.disabled {
      swatch = swatch.describe("a11y_disabled", "true");
    } else {
      let choose = item.choose.clone();
      swatch = swatch.on_click(move |_| choose());
    }
    let keys: Vec<Option<Arc<str>>> = [-1, 1]
      .into_iter()
      .map(|by| self.vertical(index, by).map(|to| self.swatches[to].key.clone()))
      .collect();
    let (up, down) = (keys[0].clone(), keys[1].clone());
    swatch = swatch.on_key_down(move |event: KeyboardEvent| {
      if !event.focus_within || event.ctrl || event.alt || event.meta || event.shift || event.composing {
        return;
      }
      let to = match event.key.as_str() {
        "ArrowUp" => &up,
        "ArrowDown" => &down,
        _ => return,
      };
      match to {
        Some(key) => event.focus_key(key.clone()),
        // Past the first or the last row the focus stays.
        None => event.prevent_default(),
      }
    });
    if let Some(id) = &item.id {
      swatch = swatch.id(id.clone());
    }
    swatch.into()
  }

  /// The grid in the style of the root, with nothing in it yet: one focus group, which Tab stops in once at
  /// [`Self::entry`] and whose Left, Right, Home and End move along the swatches; a `listbox` to what reads the
  /// tree.
  pub fn root(&self) -> Column {
    let mut root = Column::new()
      .part(&self.part(SwatchPickerPart::Root))
      .focus_group(FocusGroup::horizontal().one_stop(true).wrap(false))
      .describe("a11y_role", "listbox");
    if let Some(label) = &self.label {
      root = root.describe("a11y_name", label.clone());
    }
    if self.invalid {
      root = root.describe("a11y_invalid", "true");
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

  /// How the default renderer builds a picker: its root holding a row for each `columns` swatches.
  pub fn render(self) -> Element {
    let mut root = self.root();
    let count = self.swatches.len();
    let mut start = 0;
    while start < count {
      let end = (start + self.columns).min(count);
      let mut row = Row::new().part(&self.part(SwatchPickerPart::Row));
      for index in start..end {
        row = row.child(self.swatch(index));
      }
      root = root.child(row);
      start = end;
    }
    root.into()
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> crate::ui::Styled for SwatchPicker<T> {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl<T: Send + Sync + 'static> Themed for SwatchPicker<T> {
  const NAME: &'static str = "swatch_picker";
  type Renderer = SwatchPickerRenderer;

  fn parts() -> crate::ui::Declared {
    SwatchPickerPart::declared()
  }

  fn states() -> crate::ui::Declared {
    SwatchPickerState::declared()
  }

  /// The design's swatch picker (`pencil/lurq-design-system.pen`, Swatch Picker): a panel in the popover's color
  /// with an edge of 1 in `Border`, the large radius and the popover's shadow, padded 10, its rows 6 apart and its
  /// swatches of 24 6 apart, each with an edge of 1 in `StatePressed` (the board's black at 8%) and the chosen one
  /// ringed in `Foreground`. The board draws no swatch focused, disabled or wrong: the focus is the theme's ring
  /// on the swatch, a disabled one is dimmed, and a wrong choice puts the panel's edge in `Destructive`.
  fn recipe() -> Recipe {
    use PaletteColor::{Border as Edge, Destructive, Popover, PopoverForeground, StatePressed};
    use SwatchPickerPart::{Root, Row, Swatch};
    use SwatchPickerState::{Framed, Invalid};
    Recipe::new()
      .base(Root, |s| s.gap(6.0))
      .base(Row, |s| s.gap(6.0))
      .base(Swatch, |s| s.border(Border::inside(BorderSize::Sm, StatePressed)))
      .state(Framed, Root, |s| {
        s.padding_x(10.0)
          .padding_y(10.0)
          .radius(RadiusSize::Lg)
          .background(Popover)
          .foreground(PopoverForeground)
          .border(Border::inside(BorderSize::Sm, Edge))
          .shadow(ShadowStyle::Md)
      })
      .when(super::recipe::Selector::new().state(Framed).state(Invalid), Root, |s| {
        s.border(Border::inside(BorderSize::Sm, Destructive))
      })
  }
}
