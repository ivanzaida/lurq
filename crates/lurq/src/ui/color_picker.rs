//! A color picker: a color chosen in an area of saturation and brightness, on a rail of hue and one of alpha, from
//! swatches, or typed as hex, RGB or HSL.

use std::{
  cell::RefCell,
  collections::{HashMap, hash_map::DefaultHasher},
  hash::{Hash, Hasher},
  panic::Location,
  sync::{Arc, Mutex, Weak},
};

use super::{
  color_swatch::{self, ColorSwatch, ColorSwatchPart, ColorSwatchShape, ColorSwatchSize},
  icon::Icon,
  icons, names,
  number_input::{NumberInput, NumberInputLayout},
  part_style::{FontFamily, Length, PartStyle},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::{fading, pressable},
  text_box::{self, BoxStates},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    events::{DragEvent, KeyboardEvent, MouseButton, MouseButtonMask, MouseEvent},
    theme::{ControlSize, IconSize, OpacityLevel, PaletteColor, RadiusSize, ShadowStyle, Theme},
  },
  components::{self, Column, Row, Stack},
  core::{ElementRef, Signal},
  eyedropper::EyeDropper,
  layout::{Alignment, StackAlignment, text_style::FontWeight},
  node::{
    BoxShadowValue, CursorIcon, Element, Gradient,
    border::Border,
    color::{Color, Hsl, Hsv},
    dimension::Dimension,
  },
};

/// The parts of a color picker a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColorPickerPart {
  /// The panel: its width, its fill, its edge, its corners, its shadow, the room inside it and between what it holds.
  Root,
  /// The area of saturation (across) and brightness (up): its size and its corners, and the ring of its focus.
  Area,
  /// The circle in the area at the color.
  AreaThumb,
  /// The rail of hue: its height and its corners.
  Hue,
  /// The rail of alpha, over a checkerboard: its height and its corners.
  Alpha,
  /// The circle on a rail at its value.
  Thumb,
  /// The light squares of the checkerboard under the rail of alpha.
  Checker,
  /// The dark squares of the checkerboard.
  CheckerCell,
  /// The row of swatches: the room between them.
  Swatches,
  /// A swatch of the row: what is laid over a [`ColorSwatch`]'s root.
  Swatch,
  /// The row of the fields of the value: the room between them.
  Fields,
  /// The field of the hex: an input's box, its text in the code's type.
  Input,
  /// What switches the fields between hex, RGB and HSL: a button with the name of the fields shown.
  Format,
  /// The letter over a field of a channel: R, G, B, A; H, S, L.
  FieldLabel,
  /// The button at the end of the fields that picks a color from the screen, where the platform can.
  Eyedropper,
  /// Its glyph.
  EyedropperIcon,
}

names!(ColorPickerPart {
  Root => "root",
  Area => "area",
  AreaThumb => "area_thumb",
  Hue => "hue",
  Alpha => "alpha",
  Thumb => "thumb",
  Checker => "checker",
  CheckerCell => "checker_cell",
  Swatches => "swatches",
  Swatch => "swatch",
  Fields => "fields",
  Input => "input",
  Format => "format",
  FieldLabel => "field_label",
  Eyedropper => "eyedropper",
  EyedropperIcon => "eyedropper_icon",
});

/// The states of a color picker a recipe styles, besides what the pointer and the keyboard do to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColorPickerState {
  /// It has a rail of alpha.
  Alpha,
  /// It is drawn without its panel: in a popover, which is the panel.
  Bare,
  /// It takes no input.
  Disabled,
}

names!(ColorPickerState {
  Alpha => "alpha",
  Bare => "bare",
  Disabled => "disabled",
});

/// The fields of a color picker's value: one of hex, or one of each channel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ColorFormat {
  /// One field, `#2563EB`: the board's.
  #[default]
  Hex,
  /// A field of red, green and blue, from 0 to 255.
  Rgb,
  /// A field of hue in degrees, saturation and lightness in percents.
  Hsl,
}

impl ColorFormat {
  /// The name the button that switches the fields shows.
  fn name(self) -> &'static str {
    match self {
      Self::Hex => "HEX",
      Self::Rgb => "RGB",
      Self::Hsl => "HSL",
    }
  }
}

/// What builds a color picker's nodes: a theme's in place of the default one.
pub type ColorPickerRenderer = Arc<dyn Fn(ColorPickerView) -> Element + Send + Sync>;

/// What is told a color.
type Told = Arc<dyn Fn(Color) + Send + Sync>;

/// The words a color picker writes for tooling, each of which the app can say otherwise.
#[derive(Clone)]
struct Words {
  saturation: Arc<str>,
  brightness: Arc<str>,
  hue: Arc<str>,
  alpha: Arc<str>,
  format: Arc<str>,
  eyedropper: Arc<str>,
}

impl Default for Words {
  fn default() -> Self {
    Self {
      saturation: Arc::from("Saturation"),
      brightness: Arc::from("Brightness"),
      hue: Arc::from("Hue"),
      alpha: Arc::from("Alpha"),
      format: Arc::from("Format"),
      eyedropper: Arc::from("Pick a color from the screen"),
    }
  }
}

/// The color a picker is bound to, or the one the app holds.
#[derive(Clone)]
enum Bound {
  Value(Signal<Color>),
  Controlled(Color),
}

/// A color picker: a color chosen in an area of saturation and brightness, on a rail of hue (and one of alpha), from
/// swatches the app gives, or typed. The color of a label, of a brush, of a theme's accent.
///
/// ```
/// use lurq::{
///   core::Signal,
///   node::color::Color,
///   ui::{ColorFormat, ColorPicker},
/// };
///
/// let accent = Signal::new(Color::from_hex("#2563EB"));
/// let picker = ColorPicker::new(accent.clone())
///   .swatches(["#3B82F6", "#2DD4BF", "#F59E0B", "#A78BFA", "#F472B6", "#FAFAFA"].map(Color::from_hex))
///   .on_change_end(|color| println!("saved {}", color.to_hex_upper()));
/// // With a rail of alpha, and the fields of hex, RGB and HSL a button switches between.
/// let veil = ColorPicker::new(Signal::new(Color::from_hex("#00000080")))
///   .alpha(true)
///   .formats([ColorFormat::Hex, ColorFormat::Rgb, ColorFormat::Hsl]);
/// // The color held by the app, which is told of each change.
/// let held = ColorPicker::controlled(Color::from_hex("#22C55E"), |color| println!("{}", color.to_hex_upper()));
/// ```
///
/// The picker does not keep its color: the app's signal, or the app's own value ([`ColorPicker::controlled`]), does.
/// It keeps, while it is in the tree, the hue and saturation a grey or black does not say (so that moving through the
/// greys does not lose the hue), the fields shown, and what a drag began at.
///
/// The area is a slider of two axes: a press or a drag sets saturation across and brightness up, and with the focus in
/// it the arrows move them a step of 1%, ten with Shift, PageUp and PageDown move the brightness by 10%, Home and End
/// take the saturation to none and to the whole. The rails are sliders of hue (0° to 360°) and alpha (0% to 100%):
/// a press or a drag, the arrows by a degree or 1% and ten with Shift, PageUp and PageDown a tenth, Home and End to
/// the ends. Each is a stop of Tab and a `slider` to what reads the tree, saying its value in words: "Saturation 84%,
/// brightness 92%", "Hue 221°", "Alpha 50%". [`ColorPicker::on_change`] is told each change as it is made,
/// [`ColorPicker::on_change_end`] once a drag is let go, a key's step or a swatch's press made, or a field committed,
/// where the color then is another than it was as it began.
pub struct ColorPicker {
  bound: Bound,
  binding: usize,
  at: Option<&'static Location<'static>>,
  alpha: bool,
  formats: Vec<ColorFormat>,
  format: ColorFormat,
  swatches: Vec<Color>,
  label: Option<Arc<str>>,
  disabled: bool,
  bare: bool,
  eyedropper: bool,
  words: Words,
  on_change: Vec<Told>,
  on_change_end: Vec<Told>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl ColorPicker {
  /// A picker bound to `value`: what it shows, and what a person's change sets.
  pub fn new(value: Signal<Color>) -> Self {
    let binding = value.id();
    Self::of(Bound::Value(value), binding, None)
  }

  /// A picker of the app's `value`, which the app holds: each change is told to `on_change`, and the picker shows what
  /// the app answers with. Two built at the one place in the app's code, in a loop, are told apart by their `id` or
  /// `key`.
  #[track_caller]
  pub fn controlled(value: Color, on_change: impl Fn(Color) + Send + Sync + 'static) -> Self {
    let at = Location::caller();
    Self::of(Bound::Controlled(value), controlled_binding(at, None), Some(at)).on_change(on_change)
  }

  fn of(bound: Bound, binding: usize, at: Option<&'static Location<'static>>) -> Self {
    Self {
      bound,
      binding,
      at,
      alpha: false,
      formats: Vec::new(),
      format: ColorFormat::Hex,
      swatches: Vec::new(),
      label: None,
      disabled: false,
      bare: false,
      eyedropper: true,
      words: Words::default(),
      on_change: Vec::new(),
      on_change_end: Vec::new(),
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// Whether the button that picks a color from the screen stands at the end of the fields, where the platform can
  /// pick ([`EyeDropper::is_supported`](crate::eyedropper::EyeDropper::is_supported)); it is never shown where it
  /// cannot. On by default.
  pub fn eyedropper(mut self, eyedropper: bool) -> Self {
    self.eyedropper = eyedropper;
    self
  }

  /// What the eyedropper is called, to what reads the tree. "Pick a color from the screen".
  pub fn eyedropper_label(mut self, label: &str) -> Self {
    self.words.eyedropper = Arc::from(label);
    self
  }

  /// What a picker built by a component of its own is kept by across renders, in place of its signal or its place in
  /// the app's code: a color field's.
  pub(crate) fn kept_by(mut self, binding: usize) -> Self {
    self.binding = binding;
    self.at = None;
    self
  }

  /// Whether it has a rail of alpha, over a checkerboard, and the fields say the alpha. Without it the alpha of the
  /// color is kept as it is.
  pub fn alpha(mut self, alpha: bool) -> Self {
    self.alpha = alpha;
    self
  }

  /// The fields shown: one of hex (the default, the board's), or one of each channel of RGB or HSL.
  pub fn format(mut self, format: ColorFormat) -> Self {
    self.format = format;
    self
  }

  /// The fields a person switches between with a button before them, in order, starting at [`ColorPicker::format`]
  /// where it is among them. One or none: no button.
  pub fn formats(mut self, formats: impl IntoIterator<Item = ColorFormat>) -> Self {
    self.formats = formats.into_iter().collect();
    self
  }

  /// A swatch of `color` after those it has: a press of it makes the color it.
  pub fn swatch(mut self, color: impl Into<Color>) -> Self {
    self.swatches.push(color.into());
    self
  }

  /// Swatches after those it has, in a row between the rails and the fields.
  pub fn swatches(mut self, colors: impl IntoIterator<Item = impl Into<Color>>) -> Self {
    self.swatches.extend(colors.into_iter().map(Into::into));
    self
  }

  /// What the picker is called, to what reads the tree: a group of its sliders and fields.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// Whether it takes no input: dimmed, and nothing in it is pressed, dragged or typed in.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Whether it is drawn without its panel: the room inside it is kept, its fill, edge, corners and shadow are not.
  /// What a popover that holds it has.
  pub fn bare(mut self, bare: bool) -> Self {
    self.bare = bare;
    self
  }

  /// The word for saturation in what the area says to tooling. "Saturation".
  pub fn saturation_label(mut self, label: &str) -> Self {
    self.words.saturation = Arc::from(label);
    self
  }

  /// The word for brightness in what the area says to tooling. "Brightness".
  pub fn brightness_label(mut self, label: &str) -> Self {
    self.words.brightness = Arc::from(label);
    self
  }

  /// The name of the rail of hue. "Hue".
  pub fn hue_label(mut self, label: &str) -> Self {
    self.words.hue = Arc::from(label);
    self
  }

  /// The name of the rail of alpha. "Alpha".
  pub fn alpha_label(mut self, label: &str) -> Self {
    self.words.alpha = Arc::from(label);
    self
  }

  /// The name of the button that switches the fields. "Format".
  pub fn format_label(mut self, label: &str) -> Self {
    self.words.format = Arc::from(label);
    self
  }

  /// Called with the color each time a person changes it: as a thumb moves, as a key steps, as a swatch is pressed, as
  /// a field is committed. Not called for what is so already, nor when the app sets the color. Each call adds a
  /// handler.
  pub fn on_change(mut self, handler: impl Fn(Color) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// Called with the color when a change is over, where it is another than it was as the change began: a drag let go,
  /// a key's step, a swatch's press, a field committed. Each call adds a handler.
  pub fn on_change_end(mut self, handler: impl Fn(Color) + Send + Sync + 'static) -> Self {
    self.on_change_end.push(Arc::new(handler));
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

  /// Lays rules over the theme's recipe for this picker.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the area, the rails and the fields as they are under the pointer or focused from the keyboard, with nothing
  /// pointing at them: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the panel. The area is `{id}-area`, the rails `{id}-hue` and `{id}-alpha`, the hex field
  /// `{id}-hex`, the swatches `{id}-swatch-0` and on, the button of the fields `{id}-format`.
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
    let binding = match self.at {
      Some(at) => controlled_binding(at, self.id.as_deref().or(self.key.as_deref())),
      None => self.binding,
    };
    // Read so that what renders the picker renders it again when the app's value changes.
    let value = match &self.bound {
      Bound::Value(signal) => signal.get(),
      Bound::Controlled(value) => *value,
    };
    let kept = kept(binding, value, self.format);
    let format = kept.format.get();
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(ColorPickerState::Alpha, self.alpha)
    .state(ColorPickerState::Bare, self.bare)
    .state(ColorPickerState::Disabled, self.disabled);
    let model = Model {
      bound: self.bound,
      kept: kept.clone(),
      on_change: self.on_change,
      on_change_end: self.on_change_end,
    };
    // A color picked from the screen is a person's change: begun and over at once. Read so that a pick that ends
    // renders the picker again.
    let value = match kept.picked.get() {
      Some(color) => {
        kept.picked.set(None);
        if !self.disabled {
          model.step(|model| model.set_color(color));
        }
        model.current()
      }
      None => value,
    };
    let hsv = kept.shown(value);
    let view = ColorPickerView {
      color: value,
      hsv,
      alpha: self.alpha,
      format,
      formats: self.formats,
      swatches: self.swatches,
      label: self.label,
      disabled: self.disabled,
      eyedropper: self.eyedropper && EyeDropper::is_supported(),
      recipe: theme.resolve::<ColorPicker>(&case, Some(&self.rules)),
      words: self.words,
      model,
      forced: self.forced,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<ColorPicker>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<ColorPicker> for Element {
  fn from(picker: ColorPicker) -> Self {
    picker.build(&current_theme())
  }
}

/// What a picker built by a controlled form is across renders: where in the app's code it is built, and its id or
/// key, out of the range of the ids of signals.
fn controlled_binding(at: &'static Location<'static>, name: Option<&str>) -> usize {
  let mut hasher = DefaultHasher::new();
  (at.file(), at.line(), at.column(), name, "color_picker").hash(&mut hasher);
  (hasher.finish() as usize) | (1 << (usize::BITS - 1))
}

/// What a picker keeps while it is in the tree: the hue and saturation its color last had (which a grey does not
/// say), the color it shows, what a change began at, the fields shown and those the app gave last, and the elements
/// its pointer is read against.
struct Kept {
  hsv: Mutex<Hsv>,
  current: Mutex<Color>,
  start: Mutex<Option<Color>>,
  format: Signal<ColorFormat>,
  given: Mutex<ColorFormat>,
  area: ElementRef,
  hue: ElementRef,
  alpha: ElementRef,
  /// A color picked from the screen, given by the pick's thread and taken by the next build.
  picked: Signal<Option<Color>>,
}

impl Kept {
  /// The color `value` as the picker shows it: by the hue and saturation it had where `value` is the color they make,
  /// else by `value`'s own, keeping the hue of a grey and the saturation of black.
  fn shown(&self, value: Color) -> Hsv {
    let mut held = self.hsv.lock().unwrap();
    *self.current.lock().unwrap() = value;
    if Color::from(*held) == value {
      return *held;
    }
    let mut hsv = value.to_hsv();
    if hsv.s <= 0.0 || hsv.v <= 0.0 {
      hsv.h = held.h;
    }
    if hsv.v <= 0.0 {
      hsv.s = held.s;
    }
    *held = hsv;
    hsv
  }
}

thread_local! {
  /// What each picker in the tree keeps, by its binding. A window's pickers are all on its thread.
  static KEPT: RefCell<HashMap<usize, Weak<Kept>>> = RefCell::new(HashMap::new());
}

/// What the picker of `binding` keeps, made for `value` and `format` where it keeps nothing yet: the handlers of the
/// picker built hold it. A format the app gives anew is shown.
fn kept(binding: usize, value: Color, format: ColorFormat) -> Arc<Kept> {
  KEPT.with(|all| {
    let mut all = all.borrow_mut();
    if let Some(kept) = all.get(&binding).and_then(Weak::upgrade) {
      let mut given = kept.given.lock().unwrap();
      if *given != format {
        *given = format;
        kept.format.set(format);
      }
      drop(given);
      return kept;
    }
    all.retain(|_, kept| kept.strong_count() > 0);
    let kept = Arc::new(Kept {
      hsv: Mutex::new(value.to_hsv()),
      current: Mutex::new(value),
      start: Mutex::new(None),
      format: Signal::new(format),
      given: Mutex::new(format),
      area: ElementRef::new(),
      hue: ElementRef::new(),
      alpha: ElementRef::new(),
      picked: Signal::new(None),
    });
    all.insert(binding, Arc::downgrade(&kept));
    kept
  })
}

/// What the handlers of a picker know of it: where its color goes, and who is told.
#[derive(Clone)]
struct Model {
  bound: Bound,
  /// What the picker keeps: held by the handlers, so that it lives while the picker is in the tree.
  kept: Arc<Kept>,
  on_change: Vec<Told>,
  on_change_end: Vec<Told>,
}

impl Model {
  fn current(&self) -> Color {
    match &self.bound {
      Bound::Value(signal) => signal.get_untracked(),
      Bound::Controlled(_) => *self.kept.current.lock().unwrap(),
    }
  }

  fn hsv(&self) -> Hsv {
    *self.kept.hsv.lock().unwrap()
  }

  /// A change begins: what its end compares with.
  fn begin(&self) {
    let current = self.current();
    self.kept.start.lock().unwrap().get_or_insert(current);
  }

  /// Makes `hsv` the color, where it makes another than the picker shows, and tells the handlers.
  fn set(&self, hsv: Hsv) {
    *self.kept.hsv.lock().unwrap() = hsv;
    let color = Color::from(hsv);
    if color == self.current() {
      return;
    }
    *self.kept.current.lock().unwrap() = color;
    if let Bound::Value(signal) = &self.bound {
      signal.set(color);
    }
    for handler in &self.on_change {
      handler(color);
    }
  }

  /// Makes `color` the color, by its own hue where it says one.
  fn set_color(&self, color: Color) {
    let mut hsv = color.to_hsv();
    let held = self.hsv();
    if hsv.s <= 0.0 || hsv.v <= 0.0 {
      hsv.h = held.h;
    }
    self.set(hsv);
  }

  /// The change is over: told where the color is another than it was as it began.
  fn end(&self) {
    let Some(start) = self.kept.start.lock().unwrap().take() else {
      return;
    };
    let now = self.current();
    if now != start {
      for handler in &self.on_change_end {
        handler(now);
      }
    }
  }

  /// A change over at once: a key's step, a swatch's press, a field committed.
  fn step(&self, change: impl FnOnce(&Self)) {
    self.begin();
    change(self);
    self.end();
  }
}

/// Where a pointer at `(x, y)` is along `area`, from 0 to 1 across and from 1 at its top to 0 at its bottom.
fn place(area: &ElementRef, x: f32, y: f32) -> (f32, f32) {
  let bounds = area.bounds();
  let across = if bounds.width > 0.0 {
    ((x - bounds.x) / bounds.width).clamp(0.0, 1.0)
  } else {
    0.0
  };
  let up = if bounds.height > 0.0 {
    (1.0 - (y - bounds.y) / bounds.height).clamp(0.0, 1.0)
  } else {
    0.0
  };
  (across, up)
}

/// A number from 0 to 1 as a whole percent.
fn percent(value: f32) -> u32 {
  (value * 100.0).round() as u32
}

/// A slider of the picker: the area, or a rail. What a press and a drag at a place make of the color, and what a key
/// does.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Slider {
  Area,
  Hue,
  Alpha,
}

impl Slider {
  /// The color with the slider at `(across, up)`.
  fn at(self, hsv: Hsv, across: f32, up: f32) -> Hsv {
    match self {
      Self::Area => Hsv {
        s: across,
        v: up,
        ..hsv
      },
      Self::Hue => Hsv {
        h: across * 360.0,
        ..hsv
      },
      Self::Alpha => Hsv { a: across, ..hsv },
    }
  }

  /// The color after `key`, with Shift where `shift` says; `None` for a key the slider does not take.
  fn key(self, hsv: Hsv, key: &str, shift: bool) -> Option<Hsv> {
    let step = if shift { 10.0 } else { 1.0 };
    let clamp = |value: f32| value.clamp(0.0, 1.0);
    match self {
      Self::Area => {
        let (s, v) = (hsv.s, hsv.v);
        let (s, v) = match key {
          "ArrowRight" => (clamp(s + step / 100.0), v),
          "ArrowLeft" => (clamp(s - step / 100.0), v),
          "ArrowUp" => (s, clamp(v + step / 100.0)),
          "ArrowDown" => (s, clamp(v - step / 100.0)),
          "PageUp" => (s, clamp(v + 0.1)),
          "PageDown" => (s, clamp(v - 0.1)),
          "Home" => (0.0, v),
          "End" => (1.0, v),
          _ => return None,
        };
        Some(Hsv { s, v, ..hsv })
      }
      Self::Hue => {
        let h = hsv.h;
        let h = match key {
          "ArrowRight" | "ArrowUp" => h + step,
          "ArrowLeft" | "ArrowDown" => h - step,
          "PageUp" => h + 36.0,
          "PageDown" => h - 36.0,
          "Home" => 0.0,
          "End" => 360.0,
          _ => return None,
        };
        Some(Hsv {
          h: h.clamp(0.0, 360.0),
          ..hsv
        })
      }
      Self::Alpha => {
        let a = hsv.a;
        let a = match key {
          "ArrowRight" | "ArrowUp" => a + step / 100.0,
          "ArrowLeft" | "ArrowDown" => a - step / 100.0,
          "PageUp" => a + 0.1,
          "PageDown" => a - 0.1,
          "Home" => 0.0,
          "End" => 1.0,
          _ => return None,
        };
        Some(Hsv { a: clamp(a), ..hsv })
      }
    }
  }
}

/// A color picker as a renderer sees it: its color, what it holds, the style of its parts, and its behavior ready to
/// be dressed.
pub struct ColorPickerView {
  pub color: Color,
  /// The color as the area and the rails show it, with the hue a grey keeps.
  pub hsv: Hsv,
  pub alpha: bool,
  /// The fields shown.
  pub format: ColorFormat,
  /// The fields a person switches between.
  pub formats: Vec<ColorFormat>,
  pub swatches: Vec<Color>,
  pub label: Option<Arc<str>>,
  pub disabled: bool,
  /// Whether the eyedropper is shown: asked for, and the platform picks colors from the screen.
  pub eyedropper: bool,
  /// The style of each part of the picker.
  pub recipe: ResolvedRecipe,
  words: Words,
  model: Model,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl ColorPickerView {
  /// The style of a part, as the picker is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// A length of a part in pixels, where the recipe gives it so.
  fn px(&self, length: Option<Length>, or: f32) -> f32 {
    match length.and_then(|length| length.resolve(self.theme())) {
      Some(Dimension::Px(px)) => px,
      _ => or,
    }
  }

  /// The id of a part of the picker: the picker's with `suffix` after it.
  fn sub_id(&self, suffix: &str) -> Option<String> {
    self.id.as_ref().map(|id| format!("{id}-{suffix}"))
  }

  /// The width of the area and the rails.
  fn width(&self) -> f32 {
    self.px(self.part(ColorPickerPart::Area).rest().width, 208.0)
  }

  /// `inner` as a slider of the picker: a stop of Tab, a `slider` to tooling, pressed and dragged against
  /// `bounds`, and moved by its keys.
  fn slider(&self, inner: Stack, slider: Slider, bounds: ElementRef, name: Arc<str>, said: String) -> Stack {
    let mut slider_node = inner
      .ref_element(bounds.clone())
      .describe("a11y_role", "slider")
      .describe("a11y_name", name)
      .describe("a11y_valuetext", said);
    if self.disabled {
      return slider_node.describe("a11y_disabled", "true");
    }
    // A press of it focuses it, as a press of the engine's slider does: its keys go on from there.
    slider_node = slider_node.tab_index(0).focuses(&bounds);
    let (press, drag, ended, released, keys) = (
      (self.model.clone(), bounds.clone()),
      (self.model.clone(), bounds.clone()),
      self.model.clone(),
      self.model.clone(),
      self.model.clone(),
    );
    slider_node
      .start_drag_buttons(MouseButtonMask::LEFT)
      .on_mouse_down(move |event: MouseEvent| {
        if event.button != MouseButton::Left {
          return;
        }
        let (model, bounds) = &press;
        model.begin();
        let (across, up) = place(bounds, event.x, event.y);
        model.set(slider.at(model.hsv(), across, up));
      })
      .on_drag_move(move |event: DragEvent| {
        let (model, bounds) = &drag;
        let (across, up) = place(bounds, event.x, event.y);
        model.set(slider.at(model.hsv(), across, up));
      })
      .on_drag_end(move |_: DragEvent| ended.end())
      .on_mouse_up(move |_: MouseEvent| released.end())
      .on_key_down(move |event: KeyboardEvent| {
        if !event.focus_within || event.ctrl || event.alt || event.meta {
          return;
        }
        if let Some(next) = slider.key(keys.hsv(), &event.key, event.shift) {
          event.prevent_default();
          keys.step(|model| model.set(next));
        }
      })
  }

  /// The area of saturation across and brightness up: the hue, white over it from its start, black over that from its
  /// bottom, and its circle at the color.
  pub fn area(&self) -> Element {
    let part = self.part(ColorPickerPart::Area);
    let thumb = self.part(ColorPickerPart::AreaThumb);
    let (width, height) = (self.width(), self.px(part.rest().height, 120.0));
    let side = self.px(thumb.rest().width, 12.0);
    let hue = Color::from(Hsv {
      h: self.hsv.h,
      s: 1.0,
      v: 1.0,
      a: 1.0,
    });
    let white = Color::new(255, 255, 255, 255);
    let black = Color::new(0, 0, 0, 255);
    let field = Stack::new()
      .stack_align(StackAlignment::TopStart)
      .part(&part)
      .size(width, height)
      .child(Row::new().size(width, height).background(hue))
      .child(
        Row::new()
          .size(width, height)
          .background_gradient(Gradient::linear(90.0, [white, white.with_alpha(0)])),
      )
      .child(
        Row::new()
          .size(width, height)
          .background_gradient(Gradient::linear(180.0, [black.with_alpha(0), black])),
      );
    let circle = Row::new()
      .part(&thumb)
      .offset(
        self.hsv.s * width - side / 2.0,
        (1.0 - self.hsv.v) * height - side / 2.0,
      )
      .pointer_events_none();
    let said = format!(
      "{} {}%, {} {}%",
      self.words.saturation,
      percent(self.hsv.s),
      self.words.brightness.to_lowercase(),
      percent(self.hsv.v)
    );
    let name: Arc<str> = Arc::from(format!(
      "{} and {}",
      self.words.saturation,
      self.words.brightness.to_lowercase()
    ));
    let mut area = Stack::new()
      .stack_align(StackAlignment::TopStart)
      .part(&part)
      .overflow_visible()
      .size(width, height)
      .child(field)
      .child(circle)
      .describe("a11y_valuenow", percent(self.hsv.s).to_string());
    area = self.slider(area, Slider::Area, self.model.kept.area.clone(), name, said);
    if let Some(id) = self.sub_id("area") {
      area = area.id(id);
    }
    area.into()
  }

  /// A rail of `slider`, of `track` drawn by `fill`, with its thumb at `at` (from 0 to 1).
  fn rail(&self, slider: Slider, part: ColorPickerPart, fill: Element, at: f32, name: Arc<str>, said: String) -> Stack {
    let track = self.part(part);
    let thumb = self.part(ColorPickerPart::Thumb);
    let width = self.width();
    let height = self.px(track.rest().height, 10.0);
    let side = self.px(thumb.rest().width, 14.0);
    let bounds = match slider {
      Slider::Alpha => self.model.kept.alpha.clone(),
      _ => self.model.kept.hue.clone(),
    };
    let rail = Stack::new()
      .stack_align(StackAlignment::TopStart)
      .part(&track)
      .overflow_visible()
      .size(width, height)
      .child(
        Stack::new()
          .stack_align(StackAlignment::TopStart)
          .size(width, height)
          .part(&track)
          .child(fill),
      )
      .child(
        Row::new()
          .part(&thumb)
          .offset(at * width - side / 2.0, (height - side) / 2.0)
          .pointer_events_none(),
      );
    self.slider(rail, slider, bounds, name, said)
  }

  /// The rail of hue, every hue from red at its start round to red at its end, with its thumb at the hue.
  pub fn hue(&self) -> Element {
    let width = self.width();
    let height = self.px(self.part(ColorPickerPart::Hue).rest().height, 10.0);
    let stops: Vec<Color> = (0..=6)
      .map(|sixth| {
        Color::from(Hsv {
          h: sixth as f32 * 60.0,
          s: 1.0,
          v: 1.0,
          a: 1.0,
        })
      })
      .collect();
    let fill = Row::new()
      .size(width, height)
      .background_gradient(Gradient::linear(90.0, stops))
      .into();
    let degrees = self.hsv.h.round() as u32;
    let mut rail = self
      .rail(
        Slider::Hue,
        ColorPickerPart::Hue,
        fill,
        self.hsv.h / 360.0,
        self.words.hue.clone(),
        format!("{} {degrees}°", self.words.hue),
      )
      .describe("a11y_valuenow", degrees.to_string())
      .describe("a11y_valuemin", "0")
      .describe("a11y_valuemax", "360");
    if let Some(id) = self.sub_id("hue") {
      rail = rail.id(id);
    }
    rail.into()
  }

  /// The rail of alpha, where the picker has one: the color from none of it to all of it over a checkerboard, with its
  /// thumb at the alpha.
  pub fn alpha_rail(&self) -> Option<Element> {
    if !self.alpha {
      return None;
    }
    let width = self.width();
    let height = self.px(self.part(ColorPickerPart::Alpha).rest().height, 10.0);
    let opaque = self.color.with_alpha(255);
    let fill = Stack::new()
      .stack_align(StackAlignment::TopStart)
      .size(width, height)
      .child(color_swatch::checkerboard(
        width,
        height,
        &self.part(ColorPickerPart::Checker),
        &self.part(ColorPickerPart::CheckerCell),
      ))
      .child(
        Row::new()
          .size(width, height)
          .background_gradient(Gradient::linear(90.0, [opaque.with_alpha(0), opaque])),
      )
      .into();
    let alpha = percent(self.hsv.a);
    let mut rail = self
      .rail(
        Slider::Alpha,
        ColorPickerPart::Alpha,
        fill,
        self.hsv.a,
        self.words.alpha.clone(),
        format!("{} {alpha}%", self.words.alpha),
      )
      .describe("a11y_valuenow", alpha.to_string())
      .describe("a11y_valuemin", "0")
      .describe("a11y_valuemax", "100");
    if let Some(id) = self.sub_id("alpha") {
      rail = rail.id(id);
    }
    Some(rail.into())
  }

  /// The row of swatches, where the picker has any: a press of one makes the color it. The one of the color is
  /// selected.
  pub fn swatches(&self) -> Option<Element> {
    if self.swatches.is_empty() {
      return None;
    }
    let rules = self.recipe.rules_for(ColorPickerPart::Swatch, ColorSwatchPart::Root);
    let theme = self.theme().clone();
    let mut row = Row::new()
      .align_items(Alignment::Center)
      .overflow_visible()
      .part(&self.part(ColorPickerPart::Swatches));
    for (index, color) in self.swatches.iter().copied().enumerate() {
      let model = self.model.clone();
      let mut swatch = ColorSwatch::new(color)
        .size(ColorSwatchSize::Sm)
        .shape(ColorSwatchShape::Round)
        .selected(color == self.color)
        .disabled(self.disabled)
        .rules(&rules)
        .on_click(move |_| model.step(|model| model.set_color(color)));
      if let Some(id) = self.sub_id(&format!("swatch-{index}")) {
        swatch = swatch.id(id);
      }
      row = row.child(swatch.build(&theme));
    }
    Some(row.into())
  }

  /// The button before the fields that switches them to the next of the formats, where there are two or more.
  pub fn format_button(&self) -> Option<Element> {
    if self.formats.len() < 2 {
      return None;
    }
    let part = self.part(ColorPickerPart::Format);
    let (formats, current, format) = (self.formats.clone(), self.format, self.model.kept.format.clone());
    let mut button = pressable(&part, self.disabled)
      .flex_shrink(0.0)
      .describe("a11y_role", "button")
      .describe("a11y_name", format!("{}: {}", self.words.format, self.format.name()))
      .child(part.text(self.format.name()).nowrap().part(&part));
    if !self.disabled {
      button = button.on_click(move |_| {
        let at = formats.iter().position(|held| *held == current).map_or(0, |at| at + 1);
        format.set(formats[at % formats.len()]);
      });
    }
    if let Some(id) = self.sub_id("format") {
      button = button.id(id);
    }
    Some(button.into())
  }

  /// The button that picks a color from the screen, where the platform can and it is not hidden: the picker stays as
  /// it is while the person picks, and the color picked is a change of theirs (`on_change` and `on_change_end`).
  pub fn eyedropper(&self) -> Option<Element> {
    if !self.eyedropper {
      return None;
    }
    let part = self.part(ColorPickerPart::Eyedropper);
    let icon = self.part(ColorPickerPart::EyedropperIcon);
    let side = icon.rest().width.unwrap_or(Length::Icon(IconSize::Md));
    let mut button = pressable(&part, self.disabled)
      .flex_shrink(0.0)
      .align_items(Alignment::Center)
      .describe("a11y_role", "button")
      .describe("a11y_name", self.words.eyedropper.clone())
      .child(
        Row::new()
          .part(&icon)
          .child(Icon::new(icons::EYEDROPPER).size(side).build(self.theme())),
      );
    if !self.disabled {
      let (picked, theme) = (self.model.kept.picked.clone(), self.theme().clone());
      button = button.on_click(move |_| {
        let picked = picked.clone();
        EyeDropper::new().theme(&theme).pick_then(move |answer| {
          if let Ok(Some(color)) = answer {
            picked.set(Some(color));
          }
        });
      });
    }
    if let Some(id) = self.sub_id("eyedropper") {
      button = button.id(id);
    }
    Some(button.into())
  }

  /// The field of the hex: what is typed is read on Enter and as the focus leaves, in any form a color is written in;
  /// what is no color takes back the color's hex.
  pub fn hex_field(&self) -> Element {
    let part = self.part(ColorPickerPart::Input);
    let text = Signal::new(self.color.to_hex_upper());
    let control = components::TextInput::new(text.clone())
      .single_line()
      .text_style(text_box::ink(self.theme(), &part.rest()))
      .part(&part)
      .keep_text_while_focused(true);
    let states = BoxStates {
      disabled: self.disabled,
      label: Some(Arc::from("Hex")),
      ..BoxStates::default()
    };
    let mut control = text_box::text_box(control, &states, Vec::new());
    let commit = {
      let (model, text) = (self.model.clone(), text.clone());
      move || {
        let typed = text.get_untracked();
        match Color::parse(&typed) {
          Some(color) => model.step(|model| model.set_color(color)),
          None => {}
        }
        let shown = model.current().to_hex_upper();
        if text.get_untracked() != shown {
          text.set(shown);
        }
      }
    };
    let enter = commit.clone();
    control = control
      .on_key_down(move |event: KeyboardEvent| {
        if event.focus_within && event.key == "Enter" {
          enter();
        }
      })
      .on_blur(commit);
    if let Some(id) = self.sub_id("hex") {
      control = control.id(id);
    }
    control.into()
  }

  /// A field of a channel: `letter` over a number of `value` from 0 to `max` that `set` makes the color of.
  fn channel(
    &self,
    letter: &str,
    name: &str,
    value: f32,
    max: f32,
    set: impl Fn(&Model, f32) + Send + Sync + 'static,
  ) -> Element {
    let model = self.model.clone();
    let label = self.part(ColorPickerPart::FieldLabel);
    let field = NumberInput::controlled(value.round() as f64, move |typed| {
      if let Some(typed) = typed {
        model.step(|model| set(model, typed as f32));
      }
    })
    .min(0.0)
    .max(max as f64)
    .layout(NumberInputLayout::Bare)
    .label(name)
    .disabled(self.disabled)
    .full_width(true);
    let mut field = field;
    if let Some(id) = self.sub_id(&format!("{}", letter.to_lowercase())) {
      field = field.id(id);
    }
    Column::new()
      .align_items(Alignment::Stretch)
      .flex(1.0)
      .min_width(0.0)
      .spacing(2.0)
      .child(label.text(letter).nowrap().part(&label))
      .child(field.build(self.theme()))
      .into()
  }

  /// The fields of the value, in the format shown: the hex, or a number for each channel, and the button that switches
  /// them where there is one.
  pub fn fields(&self) -> Element {
    let mut row = Row::new()
      .align_items(Alignment::End)
      .width(Dimension::Pct(100.0))
      .part(&self.part(ColorPickerPart::Fields));
    if let Some(button) = self.format_button() {
      row = row.child(button);
    }
    match self.format {
      ColorFormat::Hex => row = row.child(Row::new().flex(1.0).min_width(0.0).child(self.hex_field())),
      ColorFormat::Rgb => {
        let [r, g, b] = [self.color.r(), self.color.g(), self.color.b()].map(f32::from);
        row = row
          .child(self.channel("R", "Red", r, 255.0, |model, red| {
            let c = model.current();
            model.set_color(Color::new(red as u8, c.g(), c.b(), c.a()))
          }))
          .child(self.channel("G", "Green", g, 255.0, |model, green| {
            let c = model.current();
            model.set_color(Color::new(c.r(), green as u8, c.b(), c.a()))
          }))
          .child(self.channel("B", "Blue", b, 255.0, |model, blue| {
            let c = model.current();
            model.set_color(Color::new(c.r(), c.g(), blue as u8, c.a()))
          }));
      }
      ColorFormat::Hsl => {
        let hsl = self.color.to_hsl_values();
        let hue = if hsl.s <= 0.0 { self.hsv.h } else { hsl.h };
        row = row
          .child(self.channel("H", "Hue", hue, 360.0, |model, h| {
            let hsl = model.current().to_hsl_values();
            model.set(with_hue(Hsl { h, ..hsl }, h))
          }))
          .child(self.channel("S", "Saturation", hsl.s * 100.0, 100.0, |model, s| {
            let hsl = model.current().to_hsl_values();
            let h = model.hsv().h;
            model.set(with_hue(Hsl { s: s / 100.0, ..hsl }, h))
          }))
          .child(self.channel("L", "Lightness", hsl.l * 100.0, 100.0, |model, l| {
            let hsl = model.current().to_hsl_values();
            let h = model.hsv().h;
            model.set(with_hue(Hsl { l: l / 100.0, ..hsl }, h))
          }));
      }
    }
    if self.alpha && self.format != ColorFormat::Hex {
      row = row.child(self.channel("A", "Alpha", self.hsv.a * 100.0, 100.0, |model, a| {
        let hsv = model.hsv();
        model.set(Hsv { a: a / 100.0, ..hsv })
      }));
    }
    if let Some(eyedropper) = self.eyedropper() {
      row = row.child(eyedropper);
    }
    row.into()
  }

  /// How the default renderer builds a picker: the panel, holding the area, the rails, the swatches and the fields one
  /// under the other.
  pub fn render(self) -> Element {
    let mut root = Column::new()
      .align_items(Alignment::Start)
      .part(&self.part(ColorPickerPart::Root))
      .describe("a11y_role", "group")
      .child(self.area())
      .child(self.hue());
    if let Some(label) = &self.label {
      root = root.describe("a11y_name", label.clone());
    }
    if let Some(alpha) = self.alpha_rail() {
      root = root.child(alpha);
    }
    if let Some(swatches) = self.swatches() {
      root = root.child(swatches);
    }
    root = root.child(self.fields());
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root.into()
  }
}

/// The color of `hsl` as the picker shows it, keeping the hue `h` where `hsl` is a grey.
fn with_hue(hsl: Hsl, h: f32) -> Hsv {
  let mut hsv = Color::from(hsl).to_hsv();
  if hsv.s <= 0.0 || hsv.v <= 0.0 {
    hsv.h = h;
  }
  hsv
}

impl Themed for ColorPicker {
  const NAME: &'static str = "color_picker";
  type Renderer = ColorPickerRenderer;

  /// The design's color picker (`pencil/lurq-design-system.pen`, Color Picker): a panel 232 wide in the popover's
  /// fill, with an edge of 1 in `Border`, the large radius and the medium shadow, 12 of padding and 12 between what it
  /// holds. The area of saturation and brightness is 208 by 120 with the medium radius: the hue, white over it from its
  /// start and black over that from its bottom; its circle is 12, a ring of 2 in white. The rail of hue is 10 high and
  /// round at its ends. The swatches are circles of 20, 8 apart. The field of the hex is an input, 36 high, its text in
  /// the code's type.
  ///
  /// The board draws no thumb on the rail, no rail of alpha and no field but the hex: a rail's thumb is a circle of
  /// 14 with a ring of 2 in white and a soft shadow, the rail of alpha is the rail of hue over the checkerboard of a
  /// swatch, a field of a channel is a bare number input under its letter of 11 in the muted foreground, and the
  /// button that switches the fields is a ghost button of 36 high in the code's type at 12. The focus of the area and
  /// of a rail is the theme's ring around it.
  fn recipe() -> Recipe {
    use ColorPickerPart::*;
    use PaletteColor::{
      Border as Edge, Foreground, Highlight, Muted, MutedForeground, Popover as Fill, PopoverForeground,
    };
    let white = Color::new(255, 255, 255, 255);
    let thumb = move |s: PartStyle, side: f32| {
      s.square(side)
        .radius(RadiusSize::Full)
        .border(Border::inside(2.0, white))
        .cursor(CursorIcon::Pointer)
    };
    let recipe = Recipe::new()
      .base(Root, |s| {
        s.width(232.0)
          .padding_x(12.0)
          .padding_y(12.0)
          .gap(12.0)
          .radius(RadiusSize::Lg)
          .background(Fill)
          .foreground(PopoverForeground)
          .border(Border::inside(1.0, Edge))
          .shadow(ShadowStyle::Md)
      })
      .base(Area, |s| {
        s.width(208.0)
          .height(120.0)
          .radius(RadiusSize::Md)
          .cursor(CursorIcon::Crosshair)
      })
      .base(AreaThumb, move |s| thumb(s, 12.0))
      .base(Hue, |s| {
        s.height(10.0).radius(RadiusSize::Full).cursor(CursorIcon::Pointer)
      })
      .base(Alpha, |s| {
        s.height(10.0).radius(RadiusSize::Full).cursor(CursorIcon::Pointer)
      })
      .base(Thumb, move |s| thumb(s, 14.0).shadow(ShadowStyle::Sm))
      .base(Checker, |s| s.background(PaletteColor::Background))
      .base(CheckerCell, |s| s.square(5.0).background(Muted))
      .base(Swatches, |s| s.gap(8.0))
      .base(Fields, |s| s.gap(6.0))
      .base(Input, |s| {
        text_box::box_style(s)
          .height(ControlSize::Md)
          .foreground(Foreground)
          .font_family(FontFamily::Mono)
          .font_size(14.0)
          .weight(FontWeight::Normal)
      })
      .base(Format, |s| {
        fading(s)
          .height(ControlSize::Md)
          .padding_x(8.0)
          .radius(RadiusSize::Md)
          .foreground(MutedForeground)
          .font_family(FontFamily::Mono)
          .font_size(12.0)
          .cursor(CursorIcon::Pointer)
      })
      .hovered(Format, |s| s.background(Highlight).foreground(Foreground))
      .focus_visible(Format, |s| s.focus_ring())
      .base(FieldLabel, |s| s.font_size(11.0).foreground(MutedForeground))
      .base(Eyedropper, |s| {
        fading(s)
          .square(ControlSize::Md)
          .radius(RadiusSize::Md)
          .foreground(MutedForeground)
          .cursor(CursorIcon::Pointer)
      })
      .base(EyedropperIcon, |s| s.square(Length::Icon(IconSize::Md)))
      .hovered(Eyedropper, |s| s.background(Highlight).foreground(Foreground))
      .focus_visible(Eyedropper, |s| s.focus_ring())
      .state(ColorPickerState::Disabled, Eyedropper, |s| {
        s.cursor(CursorIcon::Default)
      })
      .focus_visible(Area, |s| s.focus_ring())
      .focus_visible(Hue, |s| s.focus_ring())
      .focus_visible(Alpha, |s| s.focus_ring())
      .state(ColorPickerState::Bare, Root, |s| {
        s.background(Color::new(0, 0, 0, 0))
          .border(Border::inside(0.0, Color::new(0, 0, 0, 0)))
          .shadow(BoxShadowValue::none())
          .radius(0.0)
      })
      .state(ColorPickerState::Disabled, Root, |s| s.opacity(OpacityLevel::Disabled))
      .state(ColorPickerState::Disabled, Area, |s| s.cursor(CursorIcon::Default))
      .state(ColorPickerState::Disabled, Hue, |s| s.cursor(CursorIcon::Default))
      .state(ColorPickerState::Disabled, Alpha, |s| s.cursor(CursorIcon::Default))
      .state(ColorPickerState::Disabled, Format, |s| s.cursor(CursorIcon::Default));
    text_box::box_rules(recipe, Root, Input, FieldLabel)
  }
}
