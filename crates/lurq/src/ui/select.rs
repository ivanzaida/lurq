//! A select: one value chosen from a list that opens under a trigger, bound to a signal.

use std::sync::Arc;

use super::{
  icon::Icon,
  icons, names,
  part_style::{FontFamily, Length, Opacity, PartStyle},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  theme::{Themed, current_theme},
};
use crate::{
  app::theme::{
    BorderSize, ControlSize, IconSize, OpacityLevel, PaletteColor, RadiusSize, ShadowStyle, Theme, TypographyStyle,
  },
  components::{self, SelectOption},
  core::{Signal, SignalValue},
  layout::text_style::{FontWeight, TextStyle},
  node::{
    CursorIcon, Element, NodeUpdate, Outline, SelectCheckmarkPosition, SelectIcon, SelectPartStyle, SelectStyle,
    SpacingValue, TextColor,
    border::{Border, Borders},
    dimension::Dimension,
    padding::Padding,
  },
};

/// The parts of a select a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SelectPart {
  /// The box that is pressed and focused, and that the list opens under: its size, its edge, its fill, its ring, and
  /// the type of what it says. Its gap is the room between what it says and the chevron.
  Trigger,
  /// The label of the chosen option in the trigger: its type and its color, over the trigger's.
  Value,
  /// What the trigger says while nothing is chosen, over the value's type and color.
  Placeholder,
  /// The glyph at the end of the trigger: its side and its color.
  Chevron,
  /// The panel of the open list: its fill, edge, corners, shadow and padding. Its gap is the room between the trigger
  /// and it; its height is the most it is, past which it scrolls.
  Menu,
  /// An option of the open list: its height, padding, corners, type and color. Under the pointer or the keys it is
  /// `hovered`. Its gap is the room between its label and the check.
  Item,
  /// The mark at the end of the chosen item: its side and its color.
  Check,
  /// The second, smaller line of an option that has one ([`SelectOption::detail`]).
  Detail,
  /// The short text at the end of an option that has one ([`SelectOption::hint`]): "beta", "soon".
  Hint,
  /// The label over a group of options ([`SelectOption::group`]): its padding, type and color.
  Group,
  /// The line between two groups: its fill is the line's color, its height the line's, and its padding the room
  /// around it.
  Separator,
}

names!(SelectPart {
  Trigger => "trigger",
  Value => "value",
  Placeholder => "placeholder",
  Chevron => "chevron",
  Menu => "menu",
  Item => "item",
  Check => "check",
  Detail => "detail",
  Hint => "hint",
  Group => "group",
  Separator => "separator",
});

/// The sizes of a select: the heights of the theme's controls, as an input has them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SelectSize {
  /// 32 high, its text 13.
  Sm,
  /// 36 high, its text 14.
  #[default]
  Md,
  /// 40 high, its text 15.
  Lg,
}

names!(SelectSize {
  Sm => "sm",
  Md => "md",
  Lg => "lg",
});

/// The states of a select a recipe styles, besides what the pointer and the
/// keyboard do to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SelectState {
  /// Its list is open. The engine says when; the recipe's rules for it are what the trigger looks like then.
  Open,
  /// What is chosen is wrong, or nothing is and something has to be: its edge is in the destructive color.
  Invalid,
  /// What is chosen is allowed, with a caveat: its edge is in the warning color.
  Warning,
  /// What is chosen was checked and is right: its edge is in the success color.
  Valid,
  /// It takes no input.
  Disabled,
  /// It is as wide as what holds it.
  FullWidth,
  /// Of an item: it is the chosen one. Its rules are for `Item` and `Check`.
  Chosen,
  /// Of an item: it cannot be chosen ([`SelectOption::disabled`]).
  ItemDisabled,
}

names!(SelectState {
  Open => "open",
  Invalid => "invalid",
  Warning => "warning",
  Valid => "valid",
  Disabled => "disabled",
  FullWidth => "full_width",
  Chosen => "chosen",
  ItemDisabled => "item_disabled",
});

/// What builds a select's nodes: a theme's in place of the default one.
pub type SelectRenderer = Arc<dyn Fn(SelectView) -> Element + Send + Sync>;

/// The engine's select bound to the value and its options, built with a style
/// and a placeholder: what does not depend on the type of the value is done to
/// the element it returns.
type Make = Box<dyn FnOnce(SelectStyle, Option<Arc<str>>) -> Element>;

/// A select: one value chosen from a list that opens under a trigger. A time
/// zone, a theme, a page size.
///
/// ```
/// use lurq::{components::SelectOption, core::Signal, ui::Select};
///
/// let theme = Signal::new("system");
/// let select = Select::new(theme, [("system", "System"), ("light", "Light"), ("dark", "Dark")]);
/// // Nothing chosen yet: a value that is no option's shows the placeholder.
/// let zone = Signal::new(String::new());
/// let zones = Select::new(zone, [
///   SelectOption::new("Europe/Kyiv".to_owned(), "Kyiv"),
///   SelectOption::new("Europe/Lisbon".to_owned(), "Lisbon").disabled(true),
/// ])
/// .placeholder("Select a time zone")
/// .invalid(true);
/// ```
///
/// It is the engine's [`Select`](crate::components::Select) in the design's
/// dress: a press or Enter, Space and the arrow keys open its list, the arrows
/// and the first letters move through it, and a press or Enter sets the
/// signal to the option's value and closes it. What the app sets on the signal
/// is what it shows. It stands beside an [`Input`](super::Input) in a form: the
/// same box, the same sizes, the same ring, and the same judgments of what it
/// holds (`invalid`, `warning`, `valid`). An option can start a group, which
/// the list draws with its label over it and a line above it.
pub struct Select {
  make: Make,
  placeholder: Option<Arc<str>>,
  label: Option<Arc<str>>,
  size: Name,
  invalid: bool,
  warning: bool,
  valid: bool,
  disabled: bool,
  full_width: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  name: Option<Arc<str>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Select {
  /// A select of `options` bound to `value`: the option whose value the
  /// signal has is chosen, and choosing another sets it. An option is a
  /// `(value, label)` or a [`SelectOption`], which can have a detail, an
  /// element before or after its label, or be disabled.
  pub fn new<T>(value: Signal<T>, options: impl IntoIterator<Item = impl Into<SelectOption<T>>>) -> Self
  where
    T: Clone + PartialEq + Send + Sync + SignalValue + 'static,
    Vec<T>: SignalValue,
  {
    let options: Vec<SelectOption<T>> = options.into_iter().map(Into::into).collect();
    let make: Make = Box::new(move |style, placeholder| {
      let mut select = components::Select::new(value).options(options).style(style);
      if let Some(placeholder) = placeholder {
        select = select.placeholder(placeholder);
      }
      select.into()
    });
    Self {
      make,
      placeholder: None,
      label: None,
      size: SelectSize::default().into(),
      invalid: false,
      warning: false,
      valid: false,
      disabled: false,
      full_width: false,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      name: None,
      id: None,
      key: None,
    }
  }

  /// What the trigger says while no option is chosen.
  pub fn placeholder(mut self, placeholder: &str) -> Self {
    self.placeholder = Some(Arc::from(placeholder));
    self
  }

  /// What the select is called, to what reads the tree. The field it is in shows its label.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// A [`SelectSize`], or a size the theme's recipe names.
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.size = size.into();
    self
  }

  /// Whether what is chosen is wrong, or nothing is and something has to be. Of invalid, warning and valid, the
  /// first that is set is the one shown, as an input shows them.
  pub fn invalid(mut self, invalid: bool) -> Self {
    self.invalid = invalid;
    self
  }

  /// Whether what is chosen is allowed with a caveat.
  pub fn warning(mut self, warning: bool) -> Self {
    self.warning = warning;
    self
  }

  /// Whether what is chosen was checked and is right.
  pub fn valid(mut self, valid: bool) -> Self {
    self.valid = valid;
    self
  }

  /// Whether the select takes no input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Whether the select is as wide as what holds it, in place of the design's 260.
  pub fn full_width(mut self, full_width: bool) -> Self {
    self.full_width = full_width;
    self
  }

  /// Puts the select in a state the theme's recipe names, or takes it out.
  /// `state(SelectState::Open, true)` draws the trigger as it is while the
  /// list is open, with the list closed: for a gallery or a test.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// The name the select's value has in the form it is in: the label of the chosen option.
  #[cfg(feature = "form")]
  pub fn name(mut self, name: impl Into<Arc<str>>) -> Self {
    self.name = Some(name.into());
    self
  }

  /// Styles one part of this select over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this select: what the component
  /// it is in says of the select in this place.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the trigger as it is under the pointer or focused from the
  /// keyboard, with nothing pointing at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the select's node: what is pressed and focused.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the select in `theme`. Converting a select into an element does
  /// this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    // One judgment is shown: what is wrong over what is doubtful over what is right.
    let invalid = self.invalid;
    let warning = self.warning && !invalid;
    let valid = self.valid && !invalid && !warning;
    let case = Case {
      variant: None,
      size: Some(self.size),
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(SelectState::Invalid, invalid)
    .state(SelectState::Warning, warning)
    .state(SelectState::Valid, valid)
    .state(SelectState::Disabled, self.disabled)
    .state(SelectState::FullWidth, self.full_width);
    let view = SelectView {
      placeholder: self.placeholder,
      label: self.label,
      invalid,
      disabled: self.disabled,
      recipe: theme.resolve::<Select>(&case, Some(&self.rules)),
      case,
      rules: self.rules,
      forced: self.forced,
      make: self.make,
      name: self.name,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Select>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<Select> for Element {
  fn from(select: Select) -> Self {
    select.build(&current_theme())
  }
}

/// A select as a renderer sees it: what it was given, the style of its parts,
/// and its behavior ready to be dressed.
pub struct SelectView {
  pub placeholder: Option<Arc<str>>,
  /// What it is called, to what reads the tree.
  pub label: Option<Arc<str>>,
  /// Whether what is chosen is wrong, as the select shows it.
  pub invalid: bool,
  pub disabled: bool,
  /// The style of each part of this select.
  pub recipe: ResolvedRecipe,
  case: Case,
  rules: Recipe,
  forced: Option<Interaction>,
  make: Make,
  // Read only where the engine has forms.
  #[cfg_attr(not(feature = "form"), allow(dead_code))]
  name: Option<Arc<str>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl SelectView {
  /// The style of a part, as this select is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The recipe resolved with `state` on as well: what the trigger is while
  /// the list is open, or an item while it is chosen or disabled.
  pub fn in_state(&self, state: impl Into<Name>) -> ResolvedRecipe {
    let case = self.case.clone().state(state, true);
    self.theme().resolve::<Select>(&case, Some(&self.rules))
  }

  /// The type of a part's text as the engine paints it: its typography, in
  /// the color of its foreground, which is named.
  fn ink(&self, style: &PartStyle) -> TextStyle {
    let theme = self.theme();
    let mut text = style.text.resolve(theme);
    match &style.foreground {
      Some(TextColor::Color(color)) => text.color = *color,
      Some(TextColor::Palette(role)) => text.color = theme.palette_color(role.clone()),
      None => {}
    }
    text
  }

  /// The engine's style for the select, from the recipe: the trigger at rest,
  /// under the pointer, focused from the keyboard and open; its value, its
  /// placeholder and its chevron; the panel of the list; an item at rest,
  /// under the pointer or the keys, chosen and disabled, with its check and
  /// its detail.
  pub fn select_style(&self) -> SelectStyle {
    use SelectPart::{Check, Chevron, Detail, Group, Hint, Item, Menu, Placeholder, Separator, Trigger, Value};
    let theme = self.theme();
    let trigger = self.part(Trigger);
    let rest = trigger.rest();
    let value = rest.clone().merged(&self.part(Value).rest());
    let placeholder = value.clone().merged(&self.part(Placeholder).rest());
    let mut text = engine_part(&rest, theme).text(self.ink(&value));
    if let Some(color) = &value.foreground {
      text = text.text_color(color.clone());
    }
    let mut style = SelectStyle::unstyled()
      .trigger(text)
      .placeholder_text(self.ink(&placeholder));
    if let Some(gap) = rest.gap {
      style = style.trigger_gap(gap);
    }
    let states = trigger.styles();
    if !states.hovered.is_empty() {
      style = style.trigger_hovered(engine_part(&states.hovered, theme));
    }
    if !states.focus_visible.is_empty() {
      style = style.trigger_focused(engine_part(&states.focus_visible, theme));
    }
    // Merged over the trigger last while the list is open: the whole of the trigger as the open case has it.
    style = style.trigger_open(engine_part(
      &self.in_state(SelectState::Open).part(Trigger).rest(),
      theme,
    ));

    let chevron = self.part(Chevron).rest();
    style = style.chevron(icon(icons::CHEVRON_DOWN, &chevron, theme));
    if let Some(color) = &chevron.foreground {
      style = style.chevron_color(color.clone());
    }

    // The list is not forced: a forced interaction is the trigger's.
    let menu = self.recipe.part(Menu).rest();
    style = style
      // Its height is the most it is, which the engine says apart: not the least the panel is.
      .menu(engine_part(&PartStyle { height: None, ..menu.clone() }, theme))
      .menu_gap(spacing(menu.gap, theme).unwrap_or(4.0));
    if let Some(height) = px(menu.height, theme) {
      style = style.max_menu_height(height);
    }

    let item = self.recipe.part(Item);
    let chosen = self.in_state(SelectState::Chosen).part(Item);
    style = style
      .option(engine_part(&item.rest(), theme))
      .option_selected(engine_part(&chosen.rest(), theme))
      .option_disabled(engine_part(
        &self.in_state(SelectState::ItemDisabled).part(Item).rest(),
        theme,
      ))
      .option_detail(engine_part(&self.recipe.part(Detail).rest(), theme))
      .option_hint(engine_part(&self.recipe.part(Hint).rest(), theme))
      .group_label(engine_part(&self.recipe.part(Group).rest(), theme))
      .separator(engine_part(&self.recipe.part(Separator).rest(), theme));
    // The item under the keys looks as the one under the pointer does.
    if !item.styles().hovered.is_empty() {
      style = style.option_hovered(engine_part(&item.styles().hovered, theme));
    }
    if !chosen.styles().hovered.is_empty() {
      style = style.option_selected_hovered(engine_part(&chosen.styles().hovered, theme));
    }
    if let Some(gap) = item.rest().gap {
      style = style.checkmark_gap(gap).option_adornment_gap(gap);
    }

    let check = self.in_state(SelectState::Chosen).part(Check).rest();
    style = style
      .checkmark(icon(icons::CHECK, &check, theme))
      .checkmark_position(SelectCheckmarkPosition::Trailing)
      .single_checkmark(true);
    if let Some(side) = px(check.width, theme) {
      style = style.checkmark_size(side);
    }
    if let Some(color) = &check.foreground {
      style = style.checkmark_color(color.clone());
    }
    style
  }

  /// The engine's select bound to the value and its options, in `style`, with
  /// the trigger's size, opacity and cursor: what is pressed, opened, moved
  /// through with the keys, focused and disabled. It is a stop of Tab
  /// wherever it is, and a combobox to what reads the tree.
  pub fn select(self, style: SelectStyle) -> Element {
    let theme = self.theme().clone();
    let trigger = self.part(SelectPart::Trigger).rest();
    let mut node = (self.make)(style, self.placeholder.clone()).into_node();
    if let Some(width) = trigger.width.and_then(|width| width.resolve(&theme)) {
      NodeUpdate::width(&mut node, width);
    }
    if let Some(height) = trigger.height.and_then(|height| height.resolve(&theme)) {
      NodeUpdate::height(&mut node, height);
    }
    if let Some(opacity) = opacity(trigger.opacity, &theme) {
      NodeUpdate::opacity(&mut node, opacity);
    }
    if let Some(cursor) = trigger.cursor {
      NodeUpdate::cursor(&mut node, cursor);
    }
    // A select is a stop of Tab only in a form or with an index of its own; a control of the design system is one
    // wherever it is.
    NodeUpdate::tab_index(&mut node, 0);
    NodeUpdate::disabled(&mut node, self.disabled);
    if let Some(label) = &self.label {
      NodeUpdate::describe(&mut node, Arc::from("a11y_name"), label.clone());
    }
    #[cfg(feature = "form")]
    if let Some(name) = &self.name {
      NodeUpdate::name(&mut node, name.clone());
    }
    let mut select = Element::from_node(node);
    if let Some(id) = &self.id {
      select = select.id(id.clone());
    }
    if let Some(key) = &self.key {
      select = select.key(key.clone());
    }
    select
  }

  /// How the default renderer builds a select: the engine's, in the recipe's style.
  pub fn render(self) -> Element {
    let style = self.select_style();
    self.select(style)
  }
}

/// A length in pixels: `None` for one that is not.
fn px(length: Option<Length>, theme: &Theme) -> Option<f32> {
  match length?.resolve(theme)? {
    Dimension::Px(px) => Some(px),
    _ => None,
  }
}

fn spacing(value: Option<SpacingValue>, theme: &Theme) -> Option<f32> {
  value.map(|value| value.resolve(&theme.spacing(), 0.0))
}

fn opacity(opacity: Option<Opacity>, theme: &Theme) -> Option<f32> {
  match opacity? {
    Opacity::Value(value) => Some(value),
    Opacity::Level(level) => theme.opacities().try_get(level),
  }
}

/// A glyph of the theme's set in the side of `style`, in the color the engine
/// gives it: the chevron's or the check's.
fn icon(name: &'static str, style: &PartStyle, theme: &Theme) -> SelectIcon {
  let (side, theme) = (style.width.unwrap_or(Length::Icon(IconSize::Md)), theme.clone());
  SelectIcon::element(move |color| {
    let mut icon = Icon::new(name).size(side);
    if let Some(color) = color {
      icon = icon.color(color);
    }
    icon.build(&theme)
  })
}

/// A part's style as the engine's select paints a part: its fill, edge,
/// corners, shadow, ring, opacity, padding, height (at least), type and color.
fn engine_part(style: &PartStyle, theme: &Theme) -> SelectPartStyle {
  let mut part = SelectPartStyle::new();
  if let Some(color) = &style.background {
    part = part.background(color.clone());
  }
  if let Some(border) = &style.border {
    part = part.border_custom(Borders::all(border.clone()));
  }
  if let Some(radius) = style.radius {
    part = part.rounded(radius);
  }
  if let Some(shadow) = &style.shadow {
    part = part.box_shadow(shadow.clone());
  }
  let ring = match (&style.outline, style.focus_ring) {
    (Some(outline), _) => Some(outline.clone()),
    (None, Some(true)) => Some(theme.focus_ring().outline()),
    _ => None,
  };
  if let Some(ring) = ring {
    part = part.outline(ring);
  }
  if let Some(opacity) = opacity(style.opacity, theme) {
    part = part.opacity(opacity);
  }
  if style.padding_x.is_some() || style.padding_y.is_some() {
    let none = SpacingValue::from(0.0);
    part = part.padding(Padding::symmetric(
      style.padding_x.unwrap_or(none),
      style.padding_y.unwrap_or(none),
    ));
  }
  if let Some(height) = px(style.height, theme) {
    part = part.min_height(height);
  }
  if style.sets_text() {
    part = part.text(style.text.resolve(theme));
  }
  if let Some(color) = &style.foreground {
    part = part.text_color(color.clone());
  }
  part
}

impl Themed for Select {
  const NAME: &'static str = "select";
  type Renderer = SelectRenderer;

  /// The design's select (`pencil/lurq-design-system.pen`, Select, and Menu
  /// and Menu Item for its list): a trigger shaped like an input, 260 by 36,
  /// the background's fill in an edge of 1 in the color of an input's, 6
  /// round, its text 12 from its edge in the body's type, its placeholder and
  /// its chevron of 16 in the muted foreground. Under the pointer its edge is
  /// the ring's color; focused from the keyboard and open it has a line of 2
  /// inside the box in that color; the destructive, warning and success
  /// colors for what is chosen, as an input has them. A disabled one is half opaque on the muted fill. Small is 32 with
  /// text of 13, large 40 with text of 15. The list is 4 under it on the popover's
  /// fill, in an edge of 1 in the border's color, 8 round, with the theme's
  /// middle shadow and 4 of padding; an item is 32 high, 8 from the edge of
  /// the list, 4 round, in the body's type, on the highlight under the
  /// pointer or the keys; the chosen one has a check of 16 at its end, and a
  /// disabled one is dimmed. A hint at an item's end is 11 in the mono type,
  /// muted; a group's label is 11 and semibold, muted, over its items; a
  /// line of the border's color is between groups, 4 from each.
  fn recipe() -> Recipe {
    use PaletteColor::{
      Background, Destructive, Foreground, Highlight, Muted, MutedForeground, Popover, Ring, Success,
    };
    use SelectPart::{Check, Chevron, Detail, Group, Hint, Item, Menu, Placeholder, Separator, Trigger};
    let edge = |color: PaletteColor| Border::inside(BorderSize::Sm, color);
    // The focus is a line of 2 inside the box, over its edge, as an input's is: an outline, so that the edge under
    // the pointer does not take it away.
    let ring = |color: PaletteColor| Outline::new(2.0, color).offset(-2.0);
    let recipe = Recipe::new()
      .base(Trigger, |s| {
        s.width(260.0)
          .height(ControlSize::Md)
          .padding_x(12.0)
          .gap(8.0)
          .radius(RadiusSize::Md)
          .background(Background)
          .border(edge(PaletteColor::Input))
          .foreground(Foreground)
          .typography(TypographyStyle::Body)
          .cursor(CursorIcon::Pointer)
      })
      .base(Placeholder, |s| s.foreground(MutedForeground))
      .base(Chevron, |s| s.square(IconSize::Md).foreground(MutedForeground))
      .base(Menu, |s| {
        s.gap(4.0)
          .padding_x(4.0)
          .padding_y(4.0)
          .radius(RadiusSize::Lg)
          .background(Popover)
          .border(edge(PaletteColor::Border))
          .shadow(ShadowStyle::Md)
      })
      .base(Item, |s| {
        s.height(32.0)
          .padding_x(8.0)
          .gap(8.0)
          .radius(RadiusSize::Sm)
          .foreground(Foreground)
          .typography(TypographyStyle::Body)
      })
      .base(Check, |s| s.square(IconSize::Md).foreground(Foreground))
      .base(Detail, |s| {
        s.typography(TypographyStyle::Caption).foreground(MutedForeground)
      })
      .base(Hint, |s| {
        s.font_family(FontFamily::Mono)
          .font_size(11.0)
          .foreground(MutedForeground)
      })
      // The board pads a group's label 6 over and 4 under; a part's padding is the same over and under.
      .base(Group, |s| {
        s.padding_x(8.0)
          .padding_y(5.0)
          .typography(TypographyStyle::Caption)
          .font_size(11.0)
          .weight(FontWeight::SemiBold)
          .foreground(MutedForeground)
      })
      .base(Separator, |s| {
        s.height(1.0)
          .padding_y(4.0)
          .background(PaletteColor::Border)
      })
      .hovered(Trigger, |s| s.border(edge(Ring)))
      .focus_visible(Trigger, |s| s.outline(ring(Ring)))
      .state(SelectState::Open, Trigger, |s| s.outline(ring(Ring)))
      .hovered(Item, |s| s.background(Highlight))
      .state(SelectState::ItemDisabled, Item, |s| s.opacity(0.45))
      .size(SelectSize::Sm, Trigger, |s| s.height(ControlSize::Sm).font_size(13.0))
      .size(SelectSize::Lg, Trigger, |s| s.height(ControlSize::Lg).font_size(15.0))
      .state(SelectState::FullWidth, Trigger, |s| s.width(Length::Fill))
      .state(SelectState::Disabled, Trigger, |s| {
        s.background(Muted)
          .opacity(OpacityLevel::Disabled)
          .cursor(CursorIcon::Default)
      });
    // A judgment keeps its color under the pointer, in the focus and while open.
    [
      (SelectState::Invalid, Destructive),
      (SelectState::Warning, PaletteColor::Warning),
      (SelectState::Valid, Success),
    ]
    .into_iter()
    .fold(recipe, |recipe, (state, color)| {
      recipe
        .state(state, Trigger, |s| s.border(edge(color.clone())))
        .when(Selector::new().state(state).hovered(), Trigger, |s| {
          s.border(edge(color.clone()))
        })
        .when(Selector::new().state(state).focus_visible(), Trigger, |s| {
          s.outline(ring(color.clone()))
        })
        .when(Selector::new().state(state).state(SelectState::Open), Trigger, |s| {
          s.outline(ring(color))
        })
    })
  }
}
