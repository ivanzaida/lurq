//! A color field: a color typed, or chosen in a color picker that opens under the field.

use std::{
  cell::RefCell,
  collections::{HashMap, hash_map::DefaultHasher},
  hash::{Hash, Hasher},
  panic::Location,
  sync::{Arc, Weak},
};

use super::{
  color_picker::{ColorFormat, ColorPicker},
  color_swatch::{ColorSwatch, ColorSwatchPart, ColorSwatchSize},
  field::{FieldControl, InField},
  icon::Icon,
  icons, names,
  part_style::{FontFamily, Length, PartStyle},
  popover::{Popover, PopoverPart},
  recipe::{Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::fading,
  text_box::{self, BoxStates, Judgment, TextBoxSize},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    ctx::Placement,
    events::{KeyboardEvent, MouseEvent},
    theme::{ControlSize, IconSize, PaletteColor, RadiusSize, Theme},
  },
  components::{self, Row, Stack},
  core::{ElementRef, Signal, SignalValue},
  eyedropper::EyeDropper,
  layout::{Alignment, StackAlignment, layout_kind::Justify, text_style::FontWeight},
  node::{CursorIcon, Element, HitTestBehavior, color::Color, dimension::Dimension},
};

/// The parts of a color field a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColorFieldPart {
  /// The field: its width, and what the picker opens under.
  Root,
  /// The box that is typed in: an input's box, its text in the code's type, room at its start for the swatch and at
  /// its end for the trigger.
  Control,
  /// What the box says while it holds no color.
  Placeholder,
  /// The swatch of the color at the box's start: what is laid over a [`ColorSwatch`]'s root. A press of it opens the
  /// picker.
  Swatch,
  /// What opens and closes the picker at the box's end.
  Trigger,
  /// The glyph of the trigger: an eyedropper.
  TriggerIcon,
  /// The popover's panel the picker is in.
  Panel,
}

names!(ColorFieldPart {
  Root => "root",
  Control => "control",
  Placeholder => "placeholder",
  Swatch => "swatch",
  Trigger => "trigger",
  TriggerIcon => "trigger_icon",
  Panel => "panel",
});

/// The states of a color field a recipe styles, besides what the pointer and the keyboard do to it: a box's, and
/// these.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColorFieldState {
  /// What it holds is wrong.
  Invalid,
  /// What it holds is allowed, with a caveat.
  Warning,
  /// What it holds was checked and is right.
  Valid,
  /// It can be read and copied, and not changed: the picker does not open.
  ReadOnly,
  /// It takes no input.
  Disabled,
  /// It is as wide as what holds it.
  FullWidth,
  /// Its picker is open.
  Open,
  /// It is in a fluid field ([`Field::fluid`](super::Field::fluid)): the field's label is inside its box, over
  /// the color.
  Fluid,
}

names!(ColorFieldState {
  Invalid => "invalid",
  Warning => "warning",
  Valid => "valid",
  ReadOnly => "read_only",
  Disabled => "disabled",
  FullWidth => "full_width",
  Open => "open",
  Fluid => "fluid",
});

/// The sizes of a color field: an input's.
pub type ColorFieldSize = TextBoxSize;

/// What builds a color field's nodes: a theme's in place of the default one.
pub type ColorFieldRenderer = Arc<dyn Fn(ColorFieldView) -> Element + Send + Sync>;

/// What a color field holds: a color ([`ColorField::new`]), or a color or none ([`ColorField::optional`]).
pub trait FieldColor: Copy + PartialEq + Send + Sync + SignalValue + 'static {
  /// The color held, where there is one.
  fn color(self) -> Option<Color>;
  /// What holds `color`.
  fn of(color: Color) -> Self;
  /// What holds no color, where it can be none.
  fn none() -> Option<Self>;
}

impl FieldColor for Color {
  fn color(self) -> Option<Color> {
    Some(self)
  }

  fn of(color: Color) -> Self {
    color
  }

  fn none() -> Option<Self> {
    None
  }
}

impl FieldColor for Option<Color> {
  fn color(self) -> Option<Color> {
    self
  }

  fn of(color: Color) -> Self {
    Some(color)
  }

  fn none() -> Option<Self> {
    Some(None)
  }
}

/// The value a field is bound to, or the one the app holds.
#[derive(Clone)]
enum Bound<V: FieldColor> {
  Value(Signal<V>),
  Controlled(V),
}

/// The words a color field writes for tooling.
#[derive(Clone)]
struct Words {
  trigger: Arc<str>,
  swatch: Arc<str>,
  panel: Arc<str>,
}

/// A color field: a color typed in a box, or chosen in a [`ColorPicker`] that opens under it. The color of a label, of
/// a brand, of a chart's series.
///
/// ```
/// use lurq::{
///   core::Signal,
///   node::color::Color,
///   ui::{ColorField, Field},
/// };
///
/// let brand = Signal::new(Color::from_hex("#2563EB"));
/// let field = Field::new("Brand color", ColorField::new(brand.clone()));
/// // A color or none: an emptied field is none.
/// let tint = ColorField::optional(Signal::new(None)).placeholder("No tint").alpha(true);
/// // The color held by the app, which is told of each change.
/// let held = ColorField::controlled(Color::from_hex("#22C55E"), |color: Color| println!("{}", color.to_hex_upper()));
/// ```
///
/// The field does not keep its color: the app's signal does, or the app's own value ([`ColorField::controlled`]). What
/// is typed is read on Enter and as the focus leaves, in any form a color is written in (`#2563EB`, `2563eb`, `#25e`,
/// `rgb(37, 99, 235)`, `hsl(221 83% 53%)`); what is no color takes back the color's hex, and an emptied optional
/// field is none. The picker opens in a popover under the field from the swatch at its start or Down in the box, with
/// the focus in its area; Escape and a press outside close it, and the focus goes back to the box. It keeps, while it
/// is in the tree, whether its picker is open.
///
/// The eyedropper at the box's end picks a color from anywhere on the screen
/// ([`EyeDropper`](crate::eyedropper::EyeDropper)), where the platform can; where it cannot, there is none.
pub struct ColorField<V: FieldColor = Color> {
  bound: Bound<V>,
  binding: usize,
  at: Option<&'static Location<'static>>,
  placeholder: Option<Arc<str>>,
  boxed: BoxStates,
  full_width: bool,
  alpha: bool,
  format: ColorFormat,
  formats: Vec<ColorFormat>,
  swatches: Vec<Color>,
  placement: Placement,
  eyedropper: bool,
  words: Words,
  on_change: Vec<Arc<dyn Fn(V) + Send + Sync>>,
  name: Option<Arc<str>>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl ColorField<Color> {
  /// A field bound to `value`: what it shows, and what is typed or chosen sets.
  pub fn new(value: Signal<Color>) -> Self {
    let binding = value.id();
    Self::of(Bound::Value(value), binding, None)
  }
}

impl ColorField<Option<Color>> {
  /// A field bound to `value`, which may hold no color: `None` shows the placeholder, and emptying the field makes it
  /// none.
  pub fn optional(value: Signal<Option<Color>>) -> Self {
    let binding = value.id();
    Self::of(Bound::Value(value), binding, None)
  }
}

impl<V: FieldColor> ColorField<V> {
  /// A field of the app's `value`, which the app holds: each change is told to `on_change`, and the field shows what
  /// the app answers with. Two built at the one place in the app's code, in a loop, are told apart by their `id` or
  /// `key`.
  #[track_caller]
  pub fn controlled(value: V, on_change: impl Fn(V) + Send + Sync + 'static) -> Self {
    let at = Location::caller();
    Self::of(Bound::Controlled(value), controlled_binding(at, None), Some(at)).on_change(on_change)
  }

  fn of(bound: Bound<V>, binding: usize, at: Option<&'static Location<'static>>) -> Self {
    Self {
      bound,
      binding,
      at,
      placeholder: None,
      boxed: BoxStates::default(),
      full_width: false,
      alpha: false,
      format: ColorFormat::Hex,
      formats: Vec::new(),
      swatches: Vec::new(),
      placement: Placement::BottomStart,
      eyedropper: true,
      words: Words {
        trigger: super::current_locale().fields.pick_from_screen.clone(),
        swatch: super::current_locale().fields.choose_color.clone(),
        panel: super::current_locale().fields.color.clone(),
      },
      on_change: Vec::new(),
      name: None,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// What the box says while it holds no color.
  pub fn placeholder(mut self, placeholder: &str) -> Self {
    self.placeholder = Some(Arc::from(placeholder));
    self
  }

  /// What the field is called, to what reads the tree. The [`Field`](super::Field) it is in gives it its label.
  pub fn label(mut self, label: &str) -> Self {
    self.boxed.label = Some(Arc::from(label));
    self
  }

  /// A [`ColorFieldSize`]: an input's, `Sm` 32, `Md` 36 (the default), `Lg` 40, with a swatch of 20, 24 and 32.
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.boxed.size = size.into();
    self
  }

  /// Whether what it holds is wrong: its edge in the destructive color.
  pub fn invalid(mut self, invalid: bool) -> Self {
    self.boxed.invalid = invalid;
    self
  }

  /// Whether what it holds is allowed, with a caveat: its edge in the warning color.
  pub fn warning(mut self, warning: bool) -> Self {
    self.boxed.warning = warning;
    self
  }

  /// Whether what it holds was checked and is right: its edge in the success color.
  pub fn valid(mut self, valid: bool) -> Self {
    self.boxed.valid = valid;
    self
  }

  /// Whether it can be read and copied, and not changed: nothing typed is taken, and the picker does not open.
  pub fn read_only(mut self, read_only: bool) -> Self {
    self.boxed.read_only = read_only;
    self
  }

  /// Whether it takes no input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.boxed.disabled = disabled;
    self
  }

  /// Whether it is as wide as what holds it, in place of 180.
  pub fn full_width(mut self, full_width: bool) -> Self {
    self.full_width = full_width;
    self
  }

  /// Whether the picker has a rail of alpha, and the box writes the alpha.
  pub fn alpha(mut self, alpha: bool) -> Self {
    self.alpha = alpha;
    self
  }

  /// The fields of the picker: see [`ColorPicker::format`].
  pub fn format(mut self, format: ColorFormat) -> Self {
    self.format = format;
    self
  }

  /// The fields of the picker a person switches between: see [`ColorPicker::formats`].
  pub fn formats(mut self, formats: impl IntoIterator<Item = ColorFormat>) -> Self {
    self.formats = formats.into_iter().collect();
    self
  }

  /// A swatch of the picker after those it has.
  pub fn swatch(mut self, color: impl Into<Color>) -> Self {
    self.swatches.push(color.into());
    self
  }

  /// Swatches of the picker after those it has.
  pub fn swatches(mut self, colors: impl IntoIterator<Item = impl Into<Color>>) -> Self {
    self.swatches.extend(colors.into_iter().map(Into::into));
    self
  }

  /// Where the picker opens beside the field: under it at its start where there is room (the default).
  pub fn placement(mut self, placement: Placement) -> Self {
    self.placement = placement;
    self
  }

  /// Whether the eyedropper stands at the box's end, where the platform picks colors from the screen
  /// ([`EyeDropper::is_supported`](crate::eyedropper::EyeDropper::is_supported)); it is never shown where it cannot.
  /// On by default. The swatch at the box's start and Down open the picker either way.
  pub fn eyedropper(mut self, eyedropper: bool) -> Self {
    self.eyedropper = eyedropper;
    self
  }

  /// What the eyedropper at the box's end is called, to what reads the tree. "Pick a color from the screen".
  pub fn trigger_label(mut self, label: &str) -> Self {
    self.words.trigger = Arc::from(label);
    self
  }

  /// What the swatch at the box's start, which opens the picker, is called to what reads the tree. "Choose color".
  pub fn swatch_label(mut self, label: &str) -> Self {
    self.words.swatch = Arc::from(label);
    self
  }

  /// What the popover of the picker is called, to what reads the tree. "Color".
  pub fn panel_label(mut self, label: &str) -> Self {
    self.words.panel = Arc::from(label);
    self
  }

  /// Called with the value each time a person changes it: typed and committed, chosen in the picker, emptied. Not
  /// called for what is so already, nor when the app sets it. Each call adds a handler.
  pub fn on_change(mut self, handler: impl Fn(V) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// The name of its value in the form it is in.
  pub fn name(mut self, name: impl Into<Arc<str>>) -> Self {
    self.name = Some(name.into());
    self
  }

  /// Puts the field in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this field over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this field.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the box as it is under the pointer or focused from the keyboard, with nothing pointing at it: for a gallery
  /// or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the box that is typed in. The swatch is `{id}-swatch`, the eyedropper `{id}-trigger`, the popover
  /// `{id}-panel` and the picker in it `{id}-picker`.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the field in `theme`. Converting a field into an element does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let binding = match self.at {
      Some(at) => controlled_binding(at, self.id.as_deref().or(self.key.as_deref())),
      None => self.binding,
    };
    let kept = kept(binding);
    // Read so that what renders the field renders it again when the app's value or the picker's opening changes.
    let value = match &self.bound {
      Bound::Value(signal) => signal.get(),
      Bound::Controlled(value) => *value,
    };
    let open = kept.open.get() && !self.boxed.disabled && !self.boxed.read_only;
    let bound = self.bound.clone();
    let actions = Model {
      bound: self.bound,
      on_change: self.on_change,
    }
    .actions();
    // A color picked from the screen is a person's change. Read so that a pick that ends renders the field again.
    let value = match kept.picked.get() {
      Some(color) => {
        kept.picked.set(None);
        if !self.boxed.disabled && !self.boxed.read_only {
          (actions.choose)(color);
        }
        match &bound {
          Bound::Value(signal) => signal.get_untracked(),
          Bound::Controlled(value) => *value,
        }
      }
      None => value,
    };
    let case = self
      .boxed
      .case(self.states.clone())
      .state(ColorFieldState::FullWidth, self.full_width)
      .state(ColorFieldState::Open, open);
    let view = ColorFieldView {
      value: value.color(),
      placeholder: self.placeholder,
      label: self.boxed.label.clone(),
      invalid: self.boxed.judgment() == Some(Judgment::Invalid),
      read_only: self.boxed.read_only,
      disabled: self.boxed.disabled,
      open,
      alpha: self.alpha,
      format: self.format,
      formats: self.formats,
      swatches: self.swatches,
      placement: self.placement,
      eyedropper: self.eyedropper && EyeDropper::is_supported(),
      recipe: theme.resolve::<ColorField>(&case, Some(&self.rules)),
      words: self.words,
      text: Signal::new(value.color().map(display).unwrap_or_default()),
      actions,
      kept,
      binding,
      boxed: self.boxed,
      name: self.name,
      forced: self.forced,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<ColorField>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl<V: FieldColor> From<ColorField<V>> for Element {
  fn from(field: ColorField<V>) -> Self {
    field.build(&current_theme())
  }
}

impl<V: FieldColor> FieldControl for ColorField<V> {
  /// Called by the field's label unless it has a name of its own, judged as the field is unless the app judged the
  /// color field itself, and disabled where the field is.
  fn in_field(mut self, field: &InField) -> Element {
    self.boxed.in_field(field);
    self.into()
  }

  fn has_fluid_form(&self) -> bool {
    true
  }
}

/// The color as the box writes it: its hex, with the alpha where the color is not opaque.
fn display(color: Color) -> String {
  color.to_hex_upper()
}

/// What a field held by the app is across renders: where in its code it is built, and its id or key.
fn controlled_binding(at: &'static Location<'static>, name: Option<&str>) -> usize {
  let mut hasher = DefaultHasher::new();
  (at.file(), at.line(), at.column(), name, "color_field").hash(&mut hasher);
  (hasher.finish() as usize) | (1 << (usize::BITS - 1))
}

/// What a field keeps while it is in the tree: whether its picker is open, and the box the picker opens under and
/// gives the focus back to.
struct Kept {
  open: Signal<bool>,
  anchor: ElementRef,
  /// A color picked from the screen, given by the pick's thread and taken by the next build.
  picked: Signal<Option<Color>>,
}

thread_local! {
  /// What each color field in the tree keeps, by its binding. A window's fields are all on its thread.
  static KEPT: RefCell<HashMap<usize, Weak<Kept>>> = RefCell::new(HashMap::new());
}

/// What the field of `binding` keeps: the handlers of the field built hold it.
fn kept(binding: usize) -> Arc<Kept> {
  KEPT.with(|all| {
    let mut all = all.borrow_mut();
    if let Some(kept) = all.get(&binding).and_then(Weak::upgrade) {
      return kept;
    }
    all.retain(|_, kept| kept.strong_count() > 0);
    let kept = Arc::new(Kept {
      open: Signal::new(false),
      anchor: ElementRef::new(),
      picked: Signal::new(None),
    });
    all.insert(binding, Arc::downgrade(&kept));
    kept
  })
}

/// Where a field's value goes, and who is told.
#[derive(Clone)]
struct Model<V: FieldColor> {
  bound: Bound<V>,
  on_change: Vec<Arc<dyn Fn(V) + Send + Sync>>,
}

/// What the handlers of a field do to its value, with the type of the value out of sight: read what is typed, and
/// take a color the picker chose.
#[derive(Clone)]
struct Actions {
  commit: Arc<dyn Fn(&Signal<String>) + Send + Sync>,
  choose: Arc<dyn Fn(Color) + Send + Sync>,
}

impl<V: FieldColor> Model<V> {
  fn current(&self) -> V {
    match &self.bound {
      Bound::Value(signal) => signal.get_untracked(),
      Bound::Controlled(value) => *value,
    }
  }

  /// Makes `value` the value, where it is another, and tells the handlers; a controlled field only tells them.
  fn write(&self, value: V) {
    if self.current() == value {
      return;
    }
    if let Bound::Value(signal) = &self.bound {
      signal.set(value);
    }
    for handler in &self.on_change {
      handler(value);
    }
  }

  /// What the handlers of the field do with this model.
  fn actions(self) -> Actions {
    let chooser = self.clone();
    Actions {
      commit: Arc::new(move |text| self.commit(text)),
      choose: Arc::new(move |color| chooser.write(V::of(color))),
    }
  }

  /// Reads what `text` holds: a color is the value, an emptied optional field none, and anything else takes back the
  /// value's text.
  fn commit(&self, text: &Signal<String>) {
    let typed = text.get_untracked();
    if typed.trim().is_empty() {
      if let Some(none) = V::none() {
        self.write(none);
      }
    } else if let Some(color) = Color::parse(&typed) {
      self.write(V::of(color));
    }
    let shown = self.current().color().map(display).unwrap_or_default();
    if text.get_untracked() != shown {
      text.set(shown);
    }
  }
}

impl Kept {
  fn set_open(&self, open: bool) {
    if self.open.get_untracked() != open {
      self.open.set(open);
    }
  }

  fn toggle(&self) {
    self.set_open(!self.open.get_untracked());
  }
}

/// A color field as a renderer sees it: its color, the style of its parts, and its behavior ready to be dressed.
pub struct ColorFieldView {
  /// The color it holds, where it holds one.
  pub value: Option<Color>,
  pub placeholder: Option<Arc<str>>,
  /// What it is called to what reads the tree: its own label, or its field's.
  pub label: Option<Arc<str>>,
  pub invalid: bool,
  pub read_only: bool,
  pub disabled: bool,
  /// Whether its picker is open.
  pub open: bool,
  pub alpha: bool,
  pub format: ColorFormat,
  pub formats: Vec<ColorFormat>,
  pub swatches: Vec<Color>,
  pub placement: Placement,
  /// Whether the eyedropper is shown: asked for, and the platform picks colors from the screen.
  pub eyedropper: bool,
  /// The style of each part of the field.
  pub recipe: ResolvedRecipe,
  words: Words,
  /// What the box holds as it is typed in.
  text: Signal<String>,
  actions: Actions,
  /// What the field keeps: held by the handlers, so that it lives while the field is in the tree.
  kept: Arc<Kept>,
  binding: usize,
  boxed: BoxStates,
  // Read only where the engine has forms.
  #[cfg_attr(not(feature = "form"), allow(dead_code))]
  name: Option<Arc<str>>,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl ColorFieldView {
  /// The style of a part, as the field is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  fn px(&self, length: Option<Length>) -> f32 {
    match length.and_then(|length| length.resolve(self.theme())) {
      Some(Dimension::Px(px)) => px,
      _ => 0.0,
    }
  }

  fn sub_id(&self, suffix: &str) -> Option<String> {
    self.id.as_ref().map(|id| format!("{id}-{suffix}"))
  }

  /// The size of the swatch: one for each size of the box.
  fn swatch_size(&self) -> ColorSwatchSize {
    if self.boxed.size == Name::from(TextBoxSize::Sm) {
      ColorSwatchSize::Sm
    } else if self.boxed.size == Name::from(TextBoxSize::Lg) {
      ColorSwatchSize::Lg
    } else {
      ColorSwatchSize::Md
    }
  }

  /// The box: an input's, typed in and focused, a `combobox` to what reads the tree that says it opens a dialog and
  /// whether it is open. Down opens the picker; Enter and the focus leaving read what is typed.
  pub fn control(&self) -> components::TextInput {
    let part = self.part(ColorFieldPart::Control);
    let placeholder = self.part(ColorFieldPart::Placeholder);
    let control = components::TextInput::new(self.text.clone())
      .single_line()
      .text_style(text_box::ink(self.theme(), &part.rest()))
      .placeholder_style(text_box::ink(self.theme(), &part.rest().merged(&placeholder.rest())))
      .part(&part)
      // Each build makes the text anew: the box keeps what is half typed while it has the focus.
      .keep_text_while_focused(true);
    let states = BoxStates {
      role: Some("combobox"),
      ..self.boxed.clone()
    };
    let mut control = text_box::text_box_as(control, &states, Vec::new(), "combobox")
      .describe("a11y_haspopup", "dialog")
      .describe("a11y_expanded", if self.open { "true" } else { "false" });
    if let Some(placeholder) = &self.placeholder {
      control = control.placeholder(placeholder);
    }
    if !self.read_only && !self.disabled {
      let (actions, kept, text) = (self.actions.clone(), self.kept.clone(), self.text.clone());
      control = control.on_key_down(move |event: KeyboardEvent| {
        if !event.focus_within || event.ctrl || event.meta || event.composing {
          return;
        }
        match event.key.as_str() {
          "ArrowDown" if !kept.open.get_untracked() => {
            event.prevent_default();
            kept.set_open(true);
          }
          // What is typed is read, and Enter goes on to what it does besides: it sends the form.
          "Enter" => (actions.commit)(&text),
          _ => {}
        }
      });
      let (blur, text) = (self.actions.clone(), self.text.clone());
      control = control.on_blur(move || (blur.commit)(&text));
    }
    #[cfg(feature = "form")]
    if let Some(name) = &self.name {
      control = control.name(name.clone());
    }
    if let Some(id) = &self.id {
      control = control.id(id.clone());
    }
    control
  }

  /// The swatch of the color at the box's start, or of none over the checkerboard: a press of it opens the picker. It
  /// takes no focus, since Down in the box opens the picker.
  pub fn swatch(&self) -> Element {
    let rules = self.recipe.rules_for(ColorFieldPart::Swatch, ColorSwatchPart::Root);
    let color = self.value.unwrap_or(Color::new(0, 0, 0, 0));
    let inner = ColorSwatch::new(color)
      .size(self.swatch_size())
      .rules(&rules)
      .build(self.theme());
    let kept = self.kept.clone();
    let mut swatch = components::Button::empty()
      .focusable(false)
      .disabled(self.disabled || self.read_only)
      .describe("a11y_name", self.words.swatch.clone())
      .describe("a11y_haspopup", "dialog")
      .child(inner)
      .on_click(move |_: MouseEvent| kept.toggle());
    if let Some(id) = self.sub_id("swatch") {
      swatch = swatch.id(id);
    }
    swatch.into()
  }

  /// The eyedropper at the box's end, where the platform picks colors from the screen and it is not hidden: it starts a
  /// pick, and the color picked is the field's, as a person's change. It takes no focus; the picker, open or not,
  /// stays as it is while the person picks.
  pub fn trigger(&self) -> Option<Element> {
    if !self.eyedropper {
      return None;
    }
    let part = self.part(ColorFieldPart::Trigger);
    let icon = self.part(ColorFieldPart::TriggerIcon);
    let side = icon.rest().width.unwrap_or(Length::Icon(IconSize::Md));
    let (picked, theme) = (self.kept.picked.clone(), self.theme().clone());
    let mut trigger = components::Button::empty()
      .justify(Justify::Center)
      .align_items(Alignment::Center)
      .part(&part)
      .focusable(false)
      .disabled(self.disabled || self.read_only)
      .describe("a11y_name", self.words.trigger.clone())
      .child(
        Row::new()
          .part(&icon)
          .child(Icon::new(icons::EYEDROPPER).size(side).build(self.theme())),
      )
      .on_click(move |_: MouseEvent| {
        let picked = picked.clone();
        EyeDropper::new().theme(&theme).pick_then(move |answer| {
          if let Ok(Some(color)) = answer {
            picked.set(Some(color));
          }
        });
      });
    if let Some(id) = self.sub_id("trigger") {
      trigger = trigger.id(id);
    }
    Some(trigger.into())
  }

  /// The field: the box, the swatch over its start and the eyedropper over its end. What the picker opens under. In
  /// a fluid field the swatch and the eyedropper are on the line of the color, under the field's label.
  pub fn field(&self) -> Stack {
    let root = self.part(ColorFieldPart::Root);
    let swatch_side = match self.swatch_size() {
      ColorSwatchSize::Sm => 20.0,
      ColorSwatchSize::Md => 24.0,
      ColorSwatchSize::Lg => 32.0,
    };
    let trigger = self.trigger();
    let trigger_side = if trigger.is_some() {
      self.px(self.part(ColorFieldPart::Trigger).rest().width)
    } else {
      0.0
    };
    // The swatch and the eyedropper stand 6 from the box's edges, the text 8 after the swatch.
    let edge = 6.0;
    Stack::new()
      .stack_align(StackAlignment::CenterStart)
      .part(&root)
      .ref_element(self.kept.anchor.clone())
      .child(
        self
          .control()
          .padding_left(edge + swatch_side + 8.0)
          .padding_right(edge + trigger_side),
      )
      .child(
        Row::new()
          .width(Dimension::Pct(100.0))
          .padding_left(edge)
          .padding_right(if trigger_side > 0.0 {
            edge - (trigger_side - 16.0) / 2.0
          } else {
            edge
          })
          .padding_top(text_box::value_line(
            &self.part(ColorFieldPart::Control).rest(),
            self.theme(),
          ))
          .align_items(Alignment::Center)
          .justify(Justify::SpaceBetween)
          .hit_test(HitTestBehavior::ContentOnly)
          .child(self.swatch())
          .with_children(trigger),
      )
  }

  /// The picker in its popover under the field, where it is open: the field's color, which a change in it sets.
  pub fn panel(&self) -> Element {
    let choose = self.actions.choose.clone();
    let color = self.value.unwrap_or(Color::new(0, 0, 0, 255));
    let mut picker = ColorPicker::controlled(color, move |color| choose(color))
      .kept_by(self.binding ^ 0x5bd1_e995)
      .bare(true)
      .alpha(self.alpha)
      .format(self.format)
      .formats(self.formats.clone())
      .swatches(self.swatches.clone())
      // The field's own eyedropper is beside the picker.
      .eyedropper(false)
      .disabled(self.disabled);
    if let Some(id) = self.sub_id("picker") {
      picker = picker.id(id);
    }
    let rules = self.recipe.rules_for(ColorFieldPart::Panel, PopoverPart::Panel);
    let mut popover = Popover::new(self.kept.open.clone(), self.kept.anchor.clone())
      .placement(self.placement)
      .label(&self.words.panel)
      .rules(&rules)
      .child(picker.build(self.theme()));
    if let Some(id) = self.sub_id("panel") {
      popover = popover.id(id);
    }
    popover.into()
  }

  /// How the default renderer builds a field: the field, and the panel that opens under it.
  pub fn render(self) -> Element {
    // No wider than the room it has: the field in it is as wide as its recipe says at most. In a fluid field it is
    // as wide as the field.
    let mut root = Row::new()
      .max_width(Dimension::Pct(100.0))
      .child(self.field())
      .child(self.panel());
    if self.boxed.fluid {
      root = root.width(Dimension::Pct(100.0));
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root.into()
  }
}

impl crate::ui::Styled for ColorField {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for ColorField {
  const NAME: &'static str = "color_field";
  type Renderer = ColorFieldRenderer;

  fn parts() -> crate::ui::Declared {
    ColorFieldPart::declared()
  }

  fn states() -> crate::ui::Declared {
    ColorFieldState::declared()
  }

  /// The design's color field (`pencil/lurq-design-system.pen`, Color Field): an input's box 180 wide and 36 high (the
  /// background's fill in an edge of 1 in the color of an input's, 6 round), with the swatch of its color (24, the
  /// board's) 6 from its start, its hex 8 after it in the code's type at 13 in the foreground, and an eyedropper of 16
  /// in the muted foreground 6 from its end. The board draws it at rest only: under the pointer, focused, judged, read
  /// only and disabled it is an input; the eyedropper is in a button of 24 with the small radius, on the highlight
  /// under the pointer. The picker opens 4 under it in a popover whose panel is the picker's: 232 wide, no padding of
  /// its own. In a fluid field, which the board does not draw for it, the box is the fluid one, as wide as the field,
  /// the swatch and the eyedropper on the line of the hex; its line is in the ring's color while the picker is open.
  fn recipe() -> Recipe {
    use ColorFieldPart::{Control, Panel, Placeholder, Root, Trigger, TriggerIcon};
    use PaletteColor::{Foreground, Highlight, MutedForeground};
    let recipe = Recipe::new()
      .base(Root, |s| s.width(180.0))
      .base(Control, |s| {
        text_box::box_style(s)
          .height(ControlSize::Md)
          .foreground(Foreground)
          .font_family(FontFamily::Mono)
          .font_size(13.0)
          .weight(FontWeight::Normal)
      })
      .base(Trigger, |s| {
        fading(
          s.square(24.0)
            .radius(RadiusSize::Sm)
            .foreground(MutedForeground)
            .cursor(CursorIcon::Pointer),
        )
      })
      .hovered(Trigger, |s| s.background(Highlight))
      .base(TriggerIcon, |s| s.square(IconSize::Md))
      .base(Panel, |s| s.width(Length::Auto).padding_x(0.0).padding_y(0.0).gap(0.0))
      .state(ColorFieldState::FullWidth, Root, |s| s.width(Length::Fill))
      .state(ColorFieldState::Disabled, Trigger, |s| s.cursor(CursorIcon::Default))
      .state(ColorFieldState::ReadOnly, Trigger, |s| s.cursor(CursorIcon::Default))
      .when(Selector::new().state(ColorFieldState::Open), Control, |s| {
        s.border(text_box::edge(PaletteColor::Ring))
      });
    let recipe = text_box::box_sizes(recipe, Control, true);
    let recipe = text_box::box_rules(recipe, Root, Control, Placeholder);
    // Open, a box has its edge in the ring's color: a fluid one its line.
    let open = Selector::new()
      .state(ColorFieldState::Fluid)
      .state(ColorFieldState::Open);
    text_box::fluid_rules(recipe, Root, Control).when(open, Control, |s| text_box::fluid_line(s, PaletteColor::Ring))
  }
}
