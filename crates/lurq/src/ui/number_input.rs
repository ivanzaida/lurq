//! A number input: a number a person types, or steps with the keys and two buttons.

use std::sync::Arc;

use super::{
  field::{FieldControl, InField},
  icon::Icon,
  icons, names,
  part_style::{FontFamily, Length, PartStyle},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::fading,
  text_box::{self, BoxStates, Judgment, TextBoxSize},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    events::{KeyboardEvent, TextInputEvent},
    theme::{ControlSize, OpacityLevel, PaletteColor, RadiusSize, Theme},
  },
  components::{self, Column, Row, Stack},
  core::Signal,
  layout::{
    Alignment, StackAlignment,
    layout_kind::Justify,
    text_style::{FontWeight, TextAlign, TextStyle},
  },
  node::{CursorIcon, Element, HitTestBehavior, dimension::Dimension, node_kind::KeyboardKind},
};

/// The parts of a number input a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NumberInputPart {
  /// The place the input takes: its width.
  Root,
  /// The box that is typed in and focused: its edge, its fill, its height, and the type of the number.
  Control,
  /// What the box says while it is empty.
  Placeholder,
  /// What holds the buttons over the box: how far in from the box's edge they are, and the corners they are cut to.
  Buttons,
  /// The button that takes a step down.
  Decrease,
  /// The button that takes a step up.
  Increase,
  /// The line between a button and the number.
  Divider,
  /// The glyph of a button.
  Icon,
}

names!(NumberInputPart {
  Root => "root",
  Control => "control",
  Placeholder => "placeholder",
  Buttons => "buttons",
  Decrease => "decrease",
  Increase => "increase",
  Divider => "divider",
  Icon => "icon",
});

/// The sizes of a number input: the heights of the theme's controls, and the type of each.
pub type NumberInputSize = TextBoxSize;

/// Where a number input's buttons are: its variants.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum NumberInputLayout {
  /// One at each end, the number between them in the middle: a minus at the start, a plus at the end. The board's.
  #[default]
  Split,
  /// One over the other at the end, a chevron up over a chevron down, the number at the start.
  Stacked,
  /// None: the number is stepped by the keys alone.
  Bare,
}

names!(NumberInputLayout {
  Split => "split",
  Stacked => "stacked",
  Bare => "bare",
});

/// The states of a number input a recipe styles, besides what the pointer and the keyboard do to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NumberInputState {
  /// What it holds is wrong: its edge is in the destructive color.
  Invalid,
  /// What it holds is allowed, with a caveat: its edge is in the warning color.
  Warning,
  /// What it holds was checked and is right: its edge is in the success color.
  Valid,
  /// It can be read, selected and copied, and not changed: its buttons do nothing.
  ReadOnly,
  /// It takes no input.
  Disabled,
  /// It is as wide as what holds it.
  FullWidth,
  /// Its value is the least it can be: the button that steps down is disabled.
  AtMin,
  /// Its value is the greatest it can be: the button that steps up is disabled.
  AtMax,
}

names!(NumberInputState {
  Invalid => "invalid",
  Warning => "warning",
  Valid => "valid",
  ReadOnly => "read_only",
  Disabled => "disabled",
  FullWidth => "full_width",
  AtMin => "at_min",
  AtMax => "at_max",
});

/// What builds a number input's nodes: a theme's in place of the default one.
pub type NumberInputRenderer = Arc<dyn Fn(NumberInputView) -> Element + Send + Sync>;

type Formatter = Arc<dyn Fn(f64) -> String + Send + Sync>;
type Parser = Arc<dyn Fn(&str) -> Option<f64> + Send + Sync>;
type ChangeHandler = Arc<dyn Fn(Option<f64>) + Send + Sync>;

/// How a number input writes its number, and reads what is pasted into it: a prefix, a suffix, the separators of
/// decimals and of thousands, or the app's own formatter and parser.
///
/// ```
/// use lurq::ui::NumberFormat;
///
/// let price = NumberFormat::new().prefix("$").grouping(true);
/// let weight = NumberFormat::new().suffix(" kg");
/// // A locale that writes a comma for decimals and a dot between thousands.
/// let german = NumberFormat::new().separators(',', '.').grouping(true).suffix(" €");
/// // The app's own: a percentage of a fraction.
/// let percent = NumberFormat::custom(
///   |value| format!("{:.0}%", value * 100.0),
///   |text| text.trim().trim_end_matches('%').trim().parse::<f64>().ok().map(|n| n / 100.0),
/// );
/// ```
///
/// What it writes is shown while the input is not focused. While it is typed in, the input shows the number alone,
/// with the decimal separator and no prefix, suffix or grouping, and takes digits, a minus and that separator.
#[derive(Clone)]
pub struct NumberFormat {
  prefix: Arc<str>,
  suffix: Arc<str>,
  decimal: char,
  group: char,
  grouping: bool,
  format: Option<Formatter>,
  parse: Option<Parser>,
}

impl Default for NumberFormat {
  fn default() -> Self {
    Self::new()
  }
}

impl NumberFormat {
  /// The number alone, with a dot for decimals and no grouping: `1234.5`.
  pub fn new() -> Self {
    Self {
      prefix: Arc::from(""),
      suffix: Arc::from(""),
      decimal: '.',
      group: ',',
      grouping: false,
      format: None,
      parse: None,
    }
  }

  /// The app's own formatter and parser: `format` writes what is shown out of the focus, a value already rounded
  /// to the input's precision; `parse` reads what it wrote, or what is pasted in. What is typed in the focus is
  /// read as a number with the decimal separator ([`NumberFormat::separators`]).
  pub fn custom(
    format: impl Fn(f64) -> String + Send + Sync + 'static,
    parse: impl Fn(&str) -> Option<f64> + Send + Sync + 'static,
  ) -> Self {
    Self {
      format: Some(Arc::new(format)),
      parse: Some(Arc::new(parse)),
      ..Self::new()
    }
  }

  /// What is written before the number: a currency's sign.
  pub fn prefix(mut self, prefix: &str) -> Self {
    self.prefix = Arc::from(prefix);
    self
  }

  /// What is written after the number: a unit.
  pub fn suffix(mut self, suffix: &str) -> Self {
    self.suffix = Arc::from(suffix);
    self
  }

  /// Whether the thousands are set apart: `1,234,567`.
  pub fn grouping(mut self, grouping: bool) -> Self {
    self.grouping = grouping;
    self
  }

  /// The separator of decimals, the one that is typed, and the one between thousands: a locale's.
  pub fn separators(mut self, decimal: char, group: char) -> Self {
    self.decimal = decimal;
    self.group = group;
    self
  }

  /// The number as it is typed: `precision` decimals with the decimal separator, no prefix, suffix or grouping.
  fn plain(&self, value: f64, precision: usize) -> String {
    let text = format!("{value:.precision$}");
    // A value that rounds to nothing is written without a sign.
    let text = match text.strip_prefix('-') {
      Some(rest) if rest.chars().all(|c| c == '0' || c == '.') => rest.to_owned(),
      _ => text,
    };
    if self.decimal == '.' {
      text
    } else {
      text.replace('.', &self.decimal.to_string())
    }
  }

  /// The number as it is shown out of the focus.
  fn display(&self, value: f64, precision: usize) -> String {
    if let Some(format) = &self.format {
      return format(value);
    }
    let plain = self.plain(value, precision);
    let plain = if self.grouping {
      let (sign, rest) = match plain.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", plain.as_str()),
      };
      let (whole, fraction) = match rest.find(self.decimal) {
        Some(at) => rest.split_at(at),
        None => (rest, ""),
      };
      let mut grouped = String::new();
      for (index, digit) in whole.chars().enumerate() {
        if index > 0 && (whole.len() - index) % 3 == 0 {
          grouped.push(self.group);
        }
        grouped.push(digit);
      }
      format!("{sign}{grouped}{fraction}")
    } else {
      plain
    };
    format!("{}{plain}{}", self.prefix, self.suffix)
  }

  /// The number in `text`, as typed or as shown: the app's parser first, then the number with the prefix, the
  /// suffix and the grouping taken off.
  fn parse(&self, text: &str) -> Option<f64> {
    if let Some(parse) = &self.parse
      && let Some(value) = parse(text)
    {
      return value.is_finite().then_some(value);
    }
    let text = text.trim();
    let text = text.strip_prefix(self.prefix.trim()).unwrap_or(text).trim();
    let text = text.strip_suffix(self.suffix.trim()).unwrap_or(text).trim();
    let number: String = text
      .chars()
      .filter(|c| *c != self.group || self.group == self.decimal)
      .map(|c| if c == self.decimal { '.' } else { c })
      .collect();
    let value = number.parse::<f64>().ok()?;
    value.is_finite().then_some(value)
  }
}

/// What a number input is bound to.
#[derive(Clone)]
enum Bound {
  /// A number there always is: an emptied field takes back the number it had.
  Value(Signal<f64>),
  /// A number that may be none: an emptied field is none.
  Optional(Signal<Option<f64>>),
  /// The app's, told back by `on_change`.
  Controlled(Option<f64>),
}

/// What the input's handlers know of it: its value, its range, its step and how it writes it.
#[derive(Clone)]
struct Model {
  bound: Bound,
  on_change: Vec<ChangeHandler>,
  min: Option<f64>,
  max: Option<f64>,
  step: f64,
  precision: usize,
  format: NumberFormat,
}

impl Model {
  fn value(&self) -> Option<f64> {
    match &self.bound {
      Bound::Value(value) => Some(value.get_untracked()),
      Bound::Optional(value) => value.get_untracked(),
      Bound::Controlled(value) => *value,
    }
  }

  /// The value, read so that the render reading it is run again when it changes.
  fn tracked_value(&self) -> Option<f64> {
    match &self.bound {
      Bound::Value(value) => Some(value.get()),
      Bound::Optional(value) => value.get(),
      Bound::Controlled(value) => *value,
    }
  }

  /// Whether an emptied field is no value.
  fn optional(&self) -> bool {
    !matches!(self.bound, Bound::Value(_))
  }

  /// Sets the value, where it changed, and tells the handlers. A number there always is takes no `None`.
  fn write(&self, value: Option<f64>) {
    match &self.bound {
      Bound::Value(signal) => {
        let Some(value) = value else {
          return;
        };
        if signal.get_untracked() == value {
          return;
        }
        signal.set(value);
      }
      Bound::Optional(signal) => {
        if signal.get_untracked() == value {
          return;
        }
        signal.set(value);
      }
      Bound::Controlled(held) => {
        if *held == value {
          return;
        }
      }
    }
    for handler in &self.on_change {
      handler(value);
    }
  }

  fn round(&self, value: f64) -> f64 {
    let scale = 10f64.powi(self.precision as i32);
    (value * scale).round() / scale
  }

  fn clamp(&self, value: f64) -> f64 {
    let value = self.min.map_or(value, |min| value.max(min));
    self.max.map_or(value, |max| value.min(max))
  }

  /// `value` as the input keeps it: rounded to its precision and in its range.
  fn settle(&self, value: f64) -> f64 {
    self.clamp(self.round(self.clamp(value)))
  }

  fn display(&self, value: Option<f64>) -> String {
    value.map_or_else(String::new, |value| self.format.display(value, self.precision))
  }

  fn edit(&self, value: Option<f64>) -> String {
    value.map_or_else(String::new, |value| self.format.plain(value, self.precision))
  }

  /// Whether `text` is a number being typed: a minus first where the range has room below zero, digits, and one
  /// decimal separator where the precision keeps decimals. An empty field is one too.
  fn accepts(&self, text: &str) -> bool {
    let negative = self.min.is_none_or(|min| min < 0.0);
    let decimals = self.precision > 0;
    let mut separators = 0;
    text.chars().enumerate().all(|(index, c)| match c {
      '0'..='9' => true,
      '-' => index == 0 && negative,
      c if c == self.format.decimal => {
        separators += 1;
        decimals && separators == 1
      }
      _ => false,
    })
  }

  /// The value `steps` steps from `from` (none: nought, or the end of the range nearer to it), on the grid of the step
  /// from the least value: a value between two steps goes to the next one in the direction of the step.
  fn stepped(&self, from: Option<f64>, steps: i32) -> f64 {
    let base = self.min.unwrap_or(0.0);
    let Some(from) = from else {
      return self.settle(0.0);
    };
    let at = (from - base) / self.step;
    // A value a hair off the grid, by the error of a fraction, is on it.
    let on = if steps > 0 {
      (at + 1e-9).floor()
    } else {
      (at - 1e-9).ceil()
    };
    self.settle(base + (on + steps as f64) * self.step)
  }

  /// Whether `value` is the least the range allows.
  fn at_min(&self, value: Option<f64>) -> bool {
    matches!((value, self.min), (Some(value), Some(min)) if value <= min)
  }

  /// Whether `value` is the greatest the range allows.
  fn at_max(&self, value: Option<f64>) -> bool {
    matches!((value, self.max), (Some(value), Some(max)) if value >= max)
  }

  /// Takes the step of `steps` from what `text` holds, or from the value, writes it and shows it: as it is typed
  /// where the text was being typed (`editing`), else as it is shown.
  fn step(&self, text: &Signal<String>, steps: i32, editing: bool) {
    let held = text.get_untracked();
    let from = self.format.parse(&held).or_else(|| self.value());
    let value = self.stepped(from, steps);
    self.write(Some(value));
    let shown = if editing {
      self.edit(Some(value))
    } else {
      self.display(Some(value))
    };
    if held != shown {
      text.set(shown);
    }
  }

  /// Takes the value to the end of the range `value` is, where the range has that end.
  fn jump(&self, text: &Signal<String>, value: f64) {
    let value = self.settle(value);
    self.write(Some(value));
    text.set(self.edit(Some(value)));
  }

  /// Puts what `text` holds right: the number in it rounded, kept in the range and written again, an empty field no
  /// value (or the value it had, where there is always one), and anything else the value it had.
  fn commit(&self, text: &Signal<String>, editing: bool) {
    let held = text.get_untracked();
    let shown = |value: Option<f64>| if editing { self.edit(value) } else { self.display(value) };
    let next = if held.trim().is_empty() {
      if self.optional() {
        self.write(None);
        String::new()
      } else {
        shown(self.value())
      }
    } else {
      match self.format.parse(&held) {
        Some(value) => {
          let value = self.settle(value);
          self.write(Some(value));
          shown(Some(value))
        }
        None => shown(self.value()),
      }
    };
    if held != next {
      text.set(next);
    }
  }
}

/// A number input: a number a person types, or steps with the arrow keys and two buttons. A quantity, a price, a
/// size in pixels.
///
/// ```
/// use lurq::{core::Signal, ui::{NumberFormat, NumberInput, NumberInputLayout}};
///
/// let quantity = Signal::new(1.0);
/// let input = NumberInput::new(quantity.clone()).min(1.0).max(99.0);
/// // A price in cents, its thousands set apart, the buttons stacked at the end.
/// let price = NumberInput::new(Signal::new(1250.0))
///   .min(0.0)
///   .step(0.5)
///   .precision(2)
///   .format(NumberFormat::new().prefix("$").grouping(true))
///   .layout(NumberInputLayout::Stacked);
/// // A number that may be left out: an emptied field is none.
/// let limit = NumberInput::optional(Signal::new(None)).placeholder("No limit");
/// ```
///
/// What is typed is kept as text while it is typed, and is the value whenever it is a number: a minus, digits and
/// one decimal separator are taken as they are typed, and nothing else. On Enter and when the focus leaves, the text
/// is put right: the number rounded to the precision, kept in the range and written in the format. Up and Down take
/// a step, Page Up and Page Down ten, Home and End go to the least and the greatest value where the range has them.
/// A wheel turned over it scrolls what it is in and does not change it. Its buttons take a step when pressed, and
/// another every twentieth of a second while held; they are not stops of Tab, and the one toward an end of the range
/// the value is at is disabled. To what reads the tree it is a `spinbutton` with its value, its least and its
/// greatest, and its buttons are "Increase" and "Decrease".
pub struct NumberInput {
  model: Model,
  layout: NumberInputLayout,
  placeholder: Option<Arc<str>>,
  boxed: BoxStates,
  full_width: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  precision: Option<usize>,
  name: Option<Arc<str>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl NumberInput {
  /// A number input bound to `value`: it shows what the signal holds, and what is typed or stepped is set on it.
  /// There is always a number: an emptied field takes back the one it had when it is left.
  pub fn new(value: Signal<f64>) -> Self {
    Self::bound(Bound::Value(value))
  }

  /// A number input bound to a number that may be none: an emptied field sets `None`, not nought.
  pub fn optional(value: Signal<Option<f64>>) -> Self {
    Self::bound(Bound::Optional(value))
  }

  /// A number input that shows `value` and tells each change to the handlers of [`NumberInput::on_change`]: the
  /// app sets what it shows. `None` is an empty field.
  pub fn controlled(value: impl Into<Option<f64>>) -> Self {
    Self::bound(Bound::Controlled(value.into()))
  }

  fn bound(bound: Bound) -> Self {
    Self {
      model: Model {
        bound,
        on_change: Vec::new(),
        min: None,
        max: None,
        step: 1.0,
        precision: 0,
        format: NumberFormat::new(),
      },
      layout: NumberInputLayout::default(),
      placeholder: None,
      boxed: BoxStates::default(),
      full_width: false,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      precision: None,
      name: None,
      id: None,
      key: None,
    }
  }

  /// The least value. None where it is not given: Home then moves the caret, and a minus can be typed.
  pub fn min(mut self, min: f64) -> Self {
    self.model.min = Some(min);
    self
  }

  /// The greatest value. None where it is not given: End then moves the caret.
  pub fn max(mut self, max: f64) -> Self {
    self.model.max = Some(max);
    self
  }

  /// How far apart the values a step takes it to are, from the least: 1 where it is not given. Page Up and Page
  /// Down take ten.
  pub fn step(mut self, step: f64) -> Self {
    if step.is_finite() && step > 0.0 {
      self.model.step = step;
    }
    self
  }

  /// How many decimals the value keeps and shows. Where it is not given, as many as the step has: none for whole
  /// steps, which then take no decimal separator.
  pub fn precision(mut self, precision: usize) -> Self {
    self.precision = Some(precision.min(12));
    self
  }

  /// How the number is written: see [`NumberFormat`].
  pub fn format(mut self, format: NumberFormat) -> Self {
    self.model.format = format;
    self
  }

  /// Where the buttons are, or that there are none.
  pub fn layout(mut self, layout: NumberInputLayout) -> Self {
    self.layout = layout;
    self
  }

  /// What the input says while it is empty.
  pub fn placeholder(mut self, placeholder: &str) -> Self {
    self.placeholder = Some(Arc::from(placeholder));
    self
  }

  /// A [`NumberInputSize`], or a size the theme's recipe names.
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.boxed.size = size.into();
    self
  }

  /// Whether what the input holds is wrong. Of invalid, warning and valid, the first that is set is the one shown.
  pub fn invalid(mut self, invalid: bool) -> Self {
    self.boxed.invalid = invalid;
    self
  }

  /// Whether what the input holds is allowed with a caveat.
  pub fn warning(mut self, warning: bool) -> Self {
    self.boxed.warning = warning;
    self
  }

  /// Whether what the input holds was checked and is right.
  pub fn valid(mut self, valid: bool) -> Self {
    self.boxed.valid = valid;
    self
  }

  /// Whether the input can be read, selected and copied, and not changed: typing, the keys and the buttons do
  /// nothing. It is still a stop of Tab.
  pub fn read_only(mut self, read_only: bool) -> Self {
    self.boxed.read_only = read_only;
    self
  }

  /// Whether the input takes no input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.boxed.disabled = disabled;
    self
  }

  /// What the input is called, to what reads the tree. A field gives it its label.
  pub fn label(mut self, label: &str) -> Self {
    self.boxed.label = Some(Arc::from(label));
    self
  }

  /// Whether the input is as wide as what holds it, in place of the design's 132.
  pub fn full_width(mut self, full_width: bool) -> Self {
    self.full_width = full_width;
    self
  }

  /// Called with the value each time it changes: as a number is typed, stepped, put right or emptied.
  pub fn on_change(mut self, handler: impl Fn(Option<f64>) + Send + Sync + 'static) -> Self {
    self.model.on_change.push(Arc::new(handler));
    self
  }

  /// Puts the input in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// The name the input's text has in the form it is in. A form's number (`FormHandle::number`) bound with
  /// [`NumberInput::new`] holds its value.
  #[cfg(feature = "form")]
  pub fn name(mut self, name: impl Into<Arc<str>>) -> Self {
    self.name = Some(name.into());
    self
  }

  /// Styles one part of this input over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this input.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the input as it is under the pointer or focused, with nothing pointing at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the control: what is typed in and focused.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// The key of the control. Each build makes the control's text anew, so the control is found again by its key:
  /// one is given where the app gives none.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the input in `theme`. Converting a number input into an element does this in the theme of what is
  /// being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let mut model = self.model;
    model.precision = self.precision.unwrap_or_else(|| decimals_of(model.step));
    if let (Some(min), Some(max)) = (model.min, model.max)
      && max < min
    {
      model.max = Some(min);
    }
    // Read so that what renders the input renders it again when the app's value changes: the text is made from it.
    let value = model.tracked_value();
    let (at_min, at_max) = (model.at_min(value), model.at_max(value));
    let case = self
      .boxed
      .case(self.states.clone())
      .state(NumberInputState::FullWidth, self.full_width)
      .state(NumberInputState::AtMin, at_min)
      .state(NumberInputState::AtMax, at_max);
    let case = Case {
      variant: Some(self.layout.into()),
      ..case
    };
    let view = NumberInputView {
      text: Signal::new(model.display(value)),
      value,
      min: model.min,
      max: model.max,
      step: model.step,
      precision: model.precision,
      layout: self.layout,
      placeholder: self.placeholder,
      label: self.boxed.label.clone(),
      invalid: self.boxed.judgment() == Some(Judgment::Invalid),
      read_only: self.boxed.read_only,
      disabled: self.boxed.disabled,
      at_min,
      at_max,
      recipe: theme.resolve::<NumberInput>(&case, Some(&self.rules)),
      forced: self.forced,
      model,
      boxed: self.boxed,
      name: self.name,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<NumberInput>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

/// How many decimals `step` has, as it would be written: 0.25 has two.
fn decimals_of(step: f64) -> usize {
  (0..=12)
    .find(|places| {
      let scale = 10f64.powi(*places);
      ((step * scale).round() - step * scale).abs() < 1e-9 * scale.max(1.0)
    })
    .unwrap_or(12) as usize
}

impl FieldControl for NumberInput {
  /// Called by the field's label unless it has a name of its own, judged as the field is unless the app judged the
  /// input itself, and disabled where the field is.
  fn in_field(mut self, field: &InField) -> Element {
    self.boxed.in_field(field);
    self.into()
  }
}

impl From<NumberInput> for Element {
  fn from(input: NumberInput) -> Self {
    input.build(&current_theme())
  }
}

/// A number input as a renderer sees it: what it was given, the style of its parts, and its behavior ready to be
/// dressed.
pub struct NumberInputView {
  /// What the control shows, made anew in each build from the value: the control keeps what is typed in it while
  /// it has the focus.
  pub text: Signal<String>,
  /// The value as the input was built with it.
  pub value: Option<f64>,
  pub min: Option<f64>,
  pub max: Option<f64>,
  pub step: f64,
  /// How many decimals the value keeps and shows: the one given, or the step's.
  pub precision: usize,
  pub layout: NumberInputLayout,
  pub placeholder: Option<Arc<str>>,
  /// What it is called to what reads the tree: its own label, or its field's.
  pub label: Option<Arc<str>>,
  /// Whether what it holds is wrong, as the input shows it.
  pub invalid: bool,
  pub read_only: bool,
  pub disabled: bool,
  /// Whether the value is the least the range allows: the button that steps down is disabled.
  pub at_min: bool,
  /// Whether the value is the greatest the range allows: the button that steps up is disabled.
  pub at_max: bool,
  /// The style of each part of this input.
  pub recipe: ResolvedRecipe,
  forced: Option<Interaction>,
  model: Model,
  boxed: BoxStates,
  // Read only where the engine has forms.
  #[cfg_attr(not(feature = "form"), allow(dead_code))]
  name: Option<Arc<str>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl NumberInputView {
  /// The style of a part, as this input is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The type of the number: the control's.
  pub fn value_style(&self) -> TextStyle {
    text_box::ink(self.theme(), &self.part(NumberInputPart::Control).rest())
  }

  /// The type of the placeholder: the control's, with what the placeholder's part lays over it.
  pub fn placeholder_style(&self) -> TextStyle {
    let control = self.part(NumberInputPart::Control).rest();
    text_box::ink(
      self.theme(),
      &control.merged(&self.part(NumberInputPart::Placeholder).rest()),
    )
  }

  /// A length of a part's style in pixels: 0 where it has none, or none in pixels.
  fn px(&self, length: Option<Length>) -> f32 {
    match length.and_then(|length| length.resolve(self.theme())) {
      Some(Dimension::Px(px)) => px,
      _ => 0.0,
    }
  }

  /// A part's padding along the row, at its start and its end.
  fn padding_x(&self, part: NumberInputPart) -> (f32, f32) {
    let spacing = self.theme().spacing();
    let style = self.part(part).rest();
    let [_, end, _, start] = style.paddings();
    (
      start.map_or(0.0, |padding| padding.resolve(&spacing, 0.0)),
      end.map_or(0.0, |padding| padding.resolve(&spacing, 0.0)),
    )
  }

  /// How far the buttons reach into the box from its start and from its end: the room the number leaves them.
  pub fn buttons_reach(&self) -> (f32, f32) {
    let (inset_start, inset_end) = self.padding_x(NumberInputPart::Buttons);
    let divider = self.px(self.part(NumberInputPart::Divider).rest().width);
    let decrease = self.px(self.part(NumberInputPart::Decrease).rest().width);
    let increase = self.px(self.part(NumberInputPart::Increase).rest().width);
    match self.layout {
      NumberInputLayout::Split => (inset_start + decrease + divider, increase + divider + inset_end),
      NumberInputLayout::Stacked => (0.0, increase.max(decrease) + divider + inset_end),
      NumberInputLayout::Bare => (0.0, 0.0),
    }
  }

  /// The input's behavior with the style of its control: the engine's text input, which is typed in, focused and
  /// disabled, and which takes what a number is typed with, steps with the keys and is put right on Enter and when
  /// the focus leaves it.
  pub fn control(&self) -> components::TextInput {
    let model = self.model.clone();
    let read_only = self.boxed.read_only;
    // What is typed: taken where it is a number being typed and then the value where it is a number; else refused,
    // unless it is a whole number as it is written, pasted in, which is then typed in its place.
    let typed = {
      let model = model.clone();
      Arc::new(move |event: &TextInputEvent| {
        let text = event.new_value();
        if model.accepts(text) {
          if text.is_empty() {
            if model.optional() {
              model.write(None);
            }
          } else if let Some(value) = model.format.parse(text) {
            model.write(Some(value));
          }
          return;
        }
        event.prevent_default();
        // A number written with what is around it, pasted: typed as it is, where it is one that could be typed.
        if let Some(value) = model.format.parse(text).map(|value| model.round(value))
          && model.accepts(&model.edit(Some(value)))
        {
          model.write(Some(value));
          event.value.set(model.edit(Some(value)));
        }
      }) as text_box::EditHandler
    };
    let control_part = self.part(NumberInputPart::Control);
    let (gap_start, gap_end) = self.padding_x(NumberInputPart::Control);
    let (reach_start, reach_end) = self.buttons_reach();
    let align = match self.layout {
      NumberInputLayout::Split => TextAlign::Center,
      NumberInputLayout::Stacked | NumberInputLayout::Bare => TextAlign::Left,
    };
    let keyboard = if model.precision > 0 || model.min.is_none_or(|min| min < 0.0) {
      KeyboardKind::Decimal
    } else {
      KeyboardKind::Number
    };
    let control = components::TextInput::new(self.text.clone())
      .single_line()
      .text_style(self.value_style())
      .placeholder_style(self.placeholder_style())
      .text_align(align)
      .part(&control_part)
      .padding_left(reach_start + gap_start)
      .padding_right(reach_end + gap_end)
      .keyboard(keyboard)
      // Each build makes the text anew: the control keeps what is half typed while it has the focus.
      .keep_text_while_focused(true);
    let mut control = text_box::text_box_as(control, &self.boxed, vec![typed], "spinbutton");
    if let Some(value) = self.value {
      control = control
        .describe("a11y_valuenow", value.to_string())
        .describe("a11y_valuetext", self.model.display(Some(value)));
    }
    if let Some(min) = self.min {
      control = control.describe("a11y_valuemin", min.to_string());
    }
    if let Some(max) = self.max {
      control = control.describe("a11y_valuemax", max.to_string());
    }
    if let Some(placeholder) = &self.placeholder {
      control = control.placeholder(placeholder);
    }
    if !read_only {
      let text = self.text.clone();
      let keys = model.clone();
      control = control.on_key_down(move |event: KeyboardEvent| {
        if !event.focus_within || event.ctrl || event.alt || event.meta || event.composing {
          return;
        }
        let steps = match event.key.as_str() {
          "ArrowUp" => Some(1),
          "ArrowDown" => Some(-1),
          "PageUp" => Some(10),
          "PageDown" => Some(-10),
          _ => None,
        };
        if let Some(steps) = steps {
          event.prevent_default();
          keys.step(&text, steps, true);
          return;
        }
        match (event.key.as_str(), keys.min, keys.max) {
          ("Home", Some(min), _) => {
            event.prevent_default();
            keys.jump(&text, min);
          }
          ("End", _, Some(max)) => {
            event.prevent_default();
            keys.jump(&text, max);
          }
          // The text is put right and Enter goes on to what it does besides: it sends the form.
          ("Enter", ..) => keys.commit(&text, true),
          _ => {}
        }
      });
    }
    // In the focus the number is typed as it is, without what is written around it; out of it, it is written.
    let (focus_text, focus_model) = (self.text.clone(), model.clone());
    let (blur_text, blur_model) = (self.text.clone(), model);
    control = control
      .on_focus(move || {
        let held = focus_text.get_untracked();
        if let Some(value) = focus_model.format.parse(&held) {
          let typed = focus_model.edit(Some(value));
          if held != typed {
            focus_text.set(typed);
          }
        }
      })
      .on_blur(move || {
        if !read_only {
          blur_model.commit(&blur_text, false);
        }
      });
    #[cfg(feature = "form")]
    if let Some(name) = &self.name {
      control = control.name(name.clone());
    }
    if let Some(id) = &self.id {
      control = control.id(id.clone());
    }
    control.key(self.key.clone().unwrap_or_else(|| Arc::from("number-input")))
  }

  /// A button that takes `steps` steps, in the style of `part`, with `icon`: pressed and held by the pointer, not a
  /// stop of Tab and not taking the focus, disabled where the input is, where it is read only, and at the end of the
  /// range it steps toward.
  pub fn button(&self, part: NumberInputPart, icon: &'static str, steps: i32) -> Row {
    let style = self.part(part);
    let icon_part = self.part(NumberInputPart::Icon);
    let side = icon_part.rest().width.unwrap_or(Length::Px(16.0));
    let at_end = if steps > 0 { self.at_max } else { self.at_min };
    let disabled = self.disabled || self.read_only || at_end;
    let (text, model) = (self.text.clone(), self.model.clone());
    let name = if steps > 0 { "Increase" } else { "Decrease" };
    Row::new()
      .part(&style)
      .align_items(Alignment::Center)
      .justify(Justify::Center)
      // A press of it leaves the focus where it is: on the number being typed, or nowhere.
      .focusable(false)
      .disabled(disabled)
      .repeat_press(true)
      .on_click(move |_| {
        let editing = model.accepts(&text.get_untracked());
        model.step(&text, steps, editing);
      })
      .describe("a11y_role", "button")
      .describe("a11y_name", name)
      .describe("a11y_disabled", if disabled { "true" } else { "false" })
      .child(
        Row::new()
          .part(&icon_part)
          .pointer_events_none()
          .child(Icon::new(icon).size(side).build(self.theme())),
      )
  }

  /// A line between a button and the number. The pointer passes through it.
  pub fn divider(&self) -> Row {
    Row::new()
      .part(&self.part(NumberInputPart::Divider))
      .pointer_events_none()
  }

  /// The buttons laid over the box, as the layout has them; none for a bare one. Only the buttons take the pointer:
  /// elsewhere it reaches the box under them.
  pub fn buttons(&self) -> Option<Row> {
    let height = self.px(self.part(NumberInputPart::Control).rest().height);
    let room = || Row::new().flex(1.0).pointer_events_none();
    let row = Row::new()
      .part(&self.part(NumberInputPart::Buttons))
      .width(Dimension::Pct(100.0))
      .height(height)
      .align_items(Alignment::Stretch)
      .hit_test(HitTestBehavior::ContentOnly);
    match self.layout {
      NumberInputLayout::Split => Some(
        row
          .child(self.button(NumberInputPart::Decrease, icons::MINUS, -1))
          .child(self.divider())
          .child(room())
          .child(self.divider())
          .child(self.button(NumberInputPart::Increase, icons::PLUS, 1)),
      ),
      NumberInputLayout::Stacked => Some(
        row.child(room()).child(self.divider()).child(
          Column::new()
            .align_items(Alignment::Stretch)
            .child(self.button(NumberInputPart::Increase, icons::CHEVRON_UP, 1).flex(1.0))
            .child(
              self
                .button(NumberInputPart::Decrease, icons::CHEVRON_DOWN, -1)
                .flex(1.0),
            ),
        ),
      ),
      NumberInputLayout::Bare => None,
    }
  }

  /// How the default renderer builds a number input: the control, which is the box and holds the ring of the
  /// focus, and its buttons laid over its ends.
  pub fn render(self) -> Element {
    let mut root = Stack::new()
      .stack_align(StackAlignment::CenterStart)
      .part(&self.part(NumberInputPart::Root))
      .child(self.control());
    if let Some(buttons) = self.buttons() {
      root = root.child(buttons);
    }
    root.into()
  }
}

impl Themed for NumberInput {
  const NAME: &'static str = "number_input";
  type Renderer = NumberInputRenderer;

  /// The design's number input (`pencil/lurq-design-system.pen`, Number Input): a box 132 wide and 36 high, an
  /// input's box (the background's fill in an edge of 1 in the color of an input's, 6 round, the ring's color under
  /// the pointer and a line of 2 inside it in the focus), its number in the middle in the code's type at 14 in the
  /// foreground; a minus at its start and a plus at its end, each 36 wide with a line of 1 in the color of an
  /// input's between it and the number, their glyphs 16 in the foreground. A button under the pointer is in the
  /// accent's fill; the one toward the end of the range the value is at is half opaque. Small is 32 with type of 13,
  /// large 40 with type of 15, the buttons as wide as the box is high. Stacked, the buttons are 24 wide at the end, a
  /// chevron of 14 up over one down with a line between them, and the number is at the start, 12 in.
  fn recipe() -> Recipe {
    use NumberInputLayout::{Split, Stacked};
    use NumberInputPart::{Buttons, Control, Decrease, Divider, Icon, Increase, Placeholder, Root};
    use PaletteColor::{Foreground, Highlight, HighlightForeground, Input};
    let button = |s: PartStyle| fading(s).width(34.0).foreground(Foreground).cursor(CursorIcon::Pointer);
    let mut recipe = Recipe::new()
      .base(Root, |s| s.width(132.0))
      .base(Control, |s| {
        text_box::box_style(s)
          .height(ControlSize::Md)
          .foreground(Foreground)
          .font_family(FontFamily::Mono)
          .font_size(14.0)
          .weight(FontWeight::Normal)
      })
      // The number in the middle stands clear of the buttons by 4.
      .variant(Split, Control, |s| s.padding_x(4.0))
      .base(Buttons, |s| s.padding_x(1.0).padding_y(1.0).radius(RadiusSize::Md))
      .base(Decrease, button)
      .base(Increase, button)
      .base(Divider, |s| s.width(1.0).background(Input))
      .base(Icon, |s| s.square(16.0))
      .hovered(Decrease, |s| s.background(Highlight).foreground(HighlightForeground))
      .hovered(Increase, |s| s.background(Highlight).foreground(HighlightForeground))
      .variant(Stacked, Decrease, |s| s.width(24.0))
      .variant(Stacked, Increase, |s| s.width(24.0).underline(text_box::edge(Input)))
      .variant(Stacked, Icon, |s| s.square(14.0))
      .state(NumberInputState::FullWidth, Root, |s| s.width(Length::Fill))
      .state(NumberInputState::AtMin, Decrease, |s| {
        s.opacity(OpacityLevel::Disabled).cursor(CursorIcon::Default)
      })
      .state(NumberInputState::AtMax, Increase, |s| {
        s.opacity(OpacityLevel::Disabled).cursor(CursorIcon::Default)
      })
      .state(NumberInputState::ReadOnly, Decrease, |s| s.cursor(CursorIcon::Default))
      .state(NumberInputState::ReadOnly, Increase, |s| s.cursor(CursorIcon::Default))
      .state(NumberInputState::Disabled, Decrease, |s| s.cursor(CursorIcon::Default))
      .state(NumberInputState::Disabled, Increase, |s| s.cursor(CursorIcon::Default));
    // A button of the board's layout is as wide as the box is high, less the box's edge and the line beside it: 36
    // with its line, at the medium size.
    for (size, width) in [(TextBoxSize::Sm, 30.0), (TextBoxSize::Lg, 38.0)] {
      let side = move |s: PartStyle| s.width(width);
      recipe = recipe
        .when(Selector::new().variant(Split).size(size), Decrease, side)
        .when(Selector::new().variant(Split).size(size), Increase, side);
    }
    let recipe = text_box::box_sizes(recipe, Control, true);
    text_box::box_rules(recipe, Root, Control, Placeholder)
  }
}
