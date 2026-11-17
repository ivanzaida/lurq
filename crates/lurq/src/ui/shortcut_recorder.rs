//! A shortcut recorder: a box that holds a key combination, which a person sets by pressing it.

use std::sync::Arc;

use super::{
  icons,
  kbd::{Kbd, KbdSize, KeyPlatform},
  names,
  part_style::{Length, PartStyle},
  recipe::{Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::{fading, glyph},
  text_box::{self, BoxStates, Judgment, TextBoxSize},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    Accelerator,
    component::Component,
    ctx::Ctx,
    events::KeyboardEvent,
    synthetic_input::SyntheticModifiers,
    theme::{ControlSize, IconSize, PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::{Column, Row},
  core::{ElementRef, Signal},
  layout::Alignment,
  node::{CursorIcon, Element},
};

/// The parts of a shortcut recorder a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShortcutRecorderPart {
  /// The box with what is said under it: as wide as the box.
  Root,
  /// The box: what is pressed and focused, and what takes the keys while it records.
  Control,
  /// The keys of the shortcut, or of the modifiers held while it records, in a row.
  Keys,
  /// The red dot that says it records.
  Dot,
  /// What the box says where it holds no shortcut: the placeholder, or "Press shortcut…" while it records.
  Hint,
  /// The cross at the end that empties it.
  Clear,
  /// What is said under the box of a shortcut that was not taken: why.
  Message,
}

names!(ShortcutRecorderPart {
  Root => "root",
  Control => "control",
  Keys => "keys",
  Dot => "dot",
  Hint => "hint",
  Clear => "clear",
  Message => "message",
});

/// The states of a shortcut recorder a recipe styles, besides what the pointer and the keyboard do to it. Those of
/// every box that is typed in (`Invalid`, `Warning`, `Valid`, `ReadOnly`, `Disabled`) are a box's
/// ([`TextBoxState`](super::TextBoxState)), by the same names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShortcutRecorderState {
  /// It takes the next combination pressed.
  Recording,
  /// It holds no shortcut.
  Empty,
  /// The last combination pressed was not taken: the box is edged as wrong and says why under it.
  Refused,
  /// What it holds is wrong: a form says so.
  Invalid,
  /// What it holds is allowed, with a caveat.
  Warning,
  /// What it holds was checked and is right.
  Valid,
  /// It is read and not changed.
  ReadOnly,
  /// It takes no input.
  Disabled,
  /// It is as wide as what holds it.
  FullWidth,
}

names!(ShortcutRecorderState {
  Recording => "recording",
  Empty => "empty",
  Refused => "refused",
  Invalid => "invalid",
  Warning => "warning",
  Valid => "valid",
  ReadOnly => "read_only",
  Disabled => "disabled",
  FullWidth => "full_width",
});

/// The sizes of a shortcut recorder: a box's.
pub type ShortcutRecorderSize = TextBoxSize;

/// The value a shortcut recorder holds: an [`Accelerator`], or `Option<Accelerator>` where that it holds none is a
/// state of its own, which Backspace and its cross give.
pub trait ShortcutValue: Clone + PartialEq + Send + Sync + 'static {
  /// The shortcut, where there is one.
  fn shortcut(&self) -> Option<&Accelerator>;
  /// The value of `shortcut`: `None` where the value cannot hold none.
  fn of(shortcut: Option<Accelerator>) -> Option<Self>;
  /// Whether it can hold none.
  const OPTIONAL: bool;
}

impl ShortcutValue for Accelerator {
  fn shortcut(&self) -> Option<&Accelerator> {
    Some(self)
  }

  fn of(shortcut: Option<Accelerator>) -> Option<Self> {
    shortcut
  }

  const OPTIONAL: bool = false;
}

impl ShortcutValue for Option<Accelerator> {
  fn shortcut(&self) -> Option<&Accelerator> {
    self.as_ref()
  }

  fn of(shortcut: Option<Accelerator>) -> Option<Self> {
    Some(shortcut)
  }

  const OPTIONAL: bool = true;
}

/// What the app says of a combination pressed: `Ok` takes it, `Err` refuses it with a sentence that says why.
pub type ShortcutCheck = Arc<dyn Fn(&Accelerator) -> Result<(), String> + Send + Sync>;

/// What builds a shortcut recorder's nodes: a theme's in place of the default one.
pub type ShortcutRecorderRenderer = Arc<dyn Fn(ShortcutRecorderView) -> Element + Send + Sync>;

/// What a shortcut recorder is told a new shortcut with, or none.
type Change = Arc<dyn Fn(Option<Accelerator>) + Send + Sync>;

/// A shortcut recorder: a box that holds a key combination, which a person sets by pressing it. The shortcut of a
/// command in an app's settings, the key that shows a window.
///
/// ```
/// use lurq::{app::Accelerator, core::Signal, ui::ShortcutRecorder};
///
/// # fn recorders(ctx: &mut lurq::app::ctx::Ctx, palette: Signal<Option<Accelerator>>) {
/// // Bound to the app's signal; mounted, it records.
/// let shortcut = ShortcutRecorder::optional(palette)
///   .label("Command palette")
///   // The app refuses a combination with a sentence.
///   .validate(|shortcut| match shortcut.to_string().as_str() {
///     "Ctrl+Q" => Err("Already used by Quit".to_owned()),
///     _ => Ok(()),
///   })
///   .mount(ctx);
/// # }
/// ```
///
/// A press of the box, or Enter or Space while it has the focus, starts recording: the box takes every key of the
/// window, before the app's own shortcuts and its menu's accelerators (`captures_keys`), shows the modifiers as they
/// are held, and takes the combination when a key that is no modifier goes down. Escape gives up, and so does the
/// focus leaving it; Backspace and Delete with no modifier, or the cross at its end, empty an optional one. A
/// combination with no modifier is refused unless [`ShortcutRecorder::require_modifier`] says otherwise (a function
/// key is taken alone), and the app refuses others with [`ShortcutRecorder::validate`]: the box says why under it, and
/// keeps what it held. A recorder records where it is mounted ([`ShortcutRecorder::mount`]), which keeps whether it
/// records; converted into an element it shows its shortcut, and its cross empties it.
pub struct ShortcutRecorder<V = Accelerator> {
  value: V,
  set: Option<Arc<dyn Fn(&V) + Send + Sync>>,
  on_change: Vec<Arc<dyn Fn(&V) + Send + Sync>>,
  checks: Vec<ShortcutCheck>,
  require_modifier: bool,
  platform: KeyPlatform,
  boxed: BoxStates,
  full_width: bool,
  placeholder: Arc<str>,
  recording_text: Arc<str>,
  clear_label: Arc<str>,
  modifier_text: Arc<str>,
  #[cfg(feature = "form")]
  name: Option<Arc<str>>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl<V: ShortcutValue> ShortcutRecorder<V> {
  fn of(value: V, set: Option<Arc<dyn Fn(&V) + Send + Sync>>) -> Self {
    Self {
      value,
      set,
      on_change: Vec::new(),
      checks: Vec::new(),
      require_modifier: true,
      platform: KeyPlatform::current(),
      boxed: BoxStates {
        size: TextBoxSize::Sm.into(),
        ..BoxStates::default()
      },
      full_width: false,
      placeholder: super::current_locale().fields.record_shortcut.clone(),
      recording_text: super::current_locale().fields.press_shortcut.clone(),
      clear_label: super::current_locale().fields.clear_shortcut.clone(),
      modifier_text: super::current_locale().fields.shortcut_needs_modifier.clone(),
      #[cfg(feature = "form")]
      name: None,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }
}

impl ShortcutRecorder<Accelerator> {
  /// A recorder of the shortcut the signal holds; a combination pressed sets it.
  pub fn new(shortcut: Signal<Accelerator>) -> Self {
    Self::of(
      shortcut.get(),
      Some(Arc::new(move |value: &Accelerator| shortcut.set(value.clone()))),
    )
  }

  /// A recorder whose shortcut the app holds: `on_change` is told a combination pressed and taken, and what is shown
  /// is what the app says next.
  pub fn controlled(shortcut: Accelerator, on_change: impl Fn(&Accelerator) + Send + Sync + 'static) -> Self {
    Self::of(shortcut, None).on_change(on_change)
  }
}

impl ShortcutRecorder<Option<Accelerator>> {
  /// A recorder of the shortcut the signal holds, none while it holds `None`; a combination pressed sets it, and
  /// Backspace or the cross empty it.
  pub fn optional(shortcut: Signal<Option<Accelerator>>) -> Self {
    Self::of(
      shortcut.get(),
      Some(Arc::new(move |value: &Option<Accelerator>| shortcut.set(value.clone()))),
    )
  }

  /// A recorder whose shortcut the app holds, none while it is `None`: `on_change` is told what a combination
  /// pressed, Backspace or the cross ask for, and what is shown is what the app says next.
  pub fn controlled_optional(
    shortcut: Option<Accelerator>,
    on_change: impl Fn(&Option<Accelerator>) + Send + Sync + 'static,
  ) -> Self {
    Self::of(shortcut, None).on_change(on_change)
  }
}

impl<V: ShortcutValue> ShortcutRecorder<V> {
  /// Called with the value a combination pressed and taken sets, or an emptying, after the signal it is bound to
  /// is set. Not for a combination that is already its shortcut, nor when the app sets the signal. A second handler
  /// is called after the first.
  pub fn on_change(mut self, handler: impl Fn(&V) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// What the app says of a combination pressed: `Err` refuses it with a sentence the box says under it ("Already
  /// used by Command Palette"), and it keeps what it held. Each call adds a check; a combination is taken where
  /// every one takes it.
  pub fn validate(mut self, check: impl Fn(&Accelerator) -> Result<(), String> + Send + Sync + 'static) -> Self {
    self.checks.push(Arc::new(check));
    self
  }

  /// Whether a combination needs a modifier held with its key: it does unless told, but a function key (F1 to F24)
  /// is taken alone.
  pub fn require_modifier(mut self, require: bool) -> Self {
    self.require_modifier = require;
    self
  }

  /// Whose names the keys are drawn with: the platform the app runs on unless told.
  pub fn platform(mut self, platform: KeyPlatform) -> Self {
    self.platform = platform;
    self
  }

  /// What it is called to tooling: "Shortcut" unless told. It draws nothing: a field gives it its own label.
  pub fn label(mut self, label: &str) -> Self {
    self.boxed.label = Some(Arc::from(label));
    self
  }

  /// What the box says where it holds no shortcut: "Record shortcut" unless told.
  pub fn placeholder(mut self, placeholder: &str) -> Self {
    self.placeholder = Arc::from(placeholder);
    self
  }

  /// What the box says while it records and no modifier is held: "Press shortcut…" unless told.
  pub fn recording_text(mut self, text: &str) -> Self {
    self.recording_text = Arc::from(text);
    self
  }

  /// What the cross that empties it is called to tooling: "Clear shortcut" unless told.
  pub fn clear_label(mut self, label: &str) -> Self {
    self.clear_label = Arc::from(label);
    self
  }

  /// What the box says under it of a combination refused for having no modifier: "Hold Ctrl, Alt, Shift or the
  /// system key with it." unless told.
  pub fn modifier_text(mut self, text: &str) -> Self {
    self.modifier_text = Arc::from(text);
    self
  }

  /// A [`ShortcutRecorderSize`], or a size the theme's recipe names: `Sm`, 32 high, the board's, unless told.
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.boxed.size = size.into();
    self
  }

  /// Whether what it holds is wrong: its edge is the destructive color.
  pub fn invalid(mut self, invalid: bool) -> Self {
    self.boxed.invalid = invalid;
    self
  }

  /// Whether what it holds is allowed, with a caveat: its edge is the warning color.
  pub fn warning(mut self, warning: bool) -> Self {
    self.boxed.warning = warning;
    self
  }

  /// Whether what it holds was checked and is right: its edge is the success color.
  pub fn valid(mut self, valid: bool) -> Self {
    self.boxed.valid = valid;
    self
  }

  /// Whether it is read and not changed: it does not record, and has no cross.
  pub fn read_only(mut self, read_only: bool) -> Self {
    self.boxed.read_only = read_only;
    self
  }

  /// Whether it takes no input: dimmed, and no stop of Tab.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.boxed.disabled = disabled;
    self
  }

  /// Whether it is as wide as what holds it, in place of the board's 230.
  pub fn full_width(mut self, full_width: bool) -> Self {
    self.full_width = full_width;
    self
  }

  /// The name its shortcut has in the form it is in, written as [`Accelerator`] writes it: `Ctrl+Shift+P`.
  #[cfg(feature = "form")]
  pub fn name(mut self, name: impl Into<Arc<str>>) -> Self {
    self.name = Some(name.into());
    self
  }

  /// Puts it in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of it over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for it: what the component it is in says of it in this place.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the box as it is under the pointer or focused from the keyboard, with nothing pointing at it: for a
  /// gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the box: what is pressed and focused.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// What tells it apart from those beside it.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds it in `theme`, not recording: it shows its shortcut, and its cross empties it. Converting it into an
  /// element does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    self.core().build(theme, None)
  }

  /// Mounts it, which keeps whether it records, the modifiers held, and a combination it refused.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    ctx.mount::<RecorderHost>(RecorderProps { core: self.core() })
  }

  /// What is built, with the app's type left behind.
  fn core(self) -> Core {
    let (set, handlers) = (self.set, self.on_change);
    let change: Change = Arc::new(move |shortcut| {
      let Some(value) = V::of(shortcut) else {
        return;
      };
      if let Some(set) = &set {
        set(&value);
      }
      for handler in &handlers {
        handler(&value);
      }
    });
    Core {
      shortcut: self.value.shortcut().cloned(),
      optional: V::OPTIONAL,
      change,
      checks: self.checks,
      require_modifier: self.require_modifier,
      platform: self.platform,
      boxed: self.boxed,
      full_width: self.full_width,
      placeholder: self.placeholder,
      recording_text: self.recording_text,
      clear_label: self.clear_label,
      modifier_text: self.modifier_text,
      #[cfg(feature = "form")]
      name: self.name,
      states: self.states,
      rules: self.rules,
      forced: self.forced,
      id: self.id,
      key: self.key,
    }
  }
}

impl<V: ShortcutValue> From<ShortcutRecorder<V>> for Element {
  fn from(recorder: ShortcutRecorder<V>) -> Self {
    recorder.build(&current_theme())
  }
}

impl<V: ShortcutValue> super::FieldControl for ShortcutRecorder<V> {
  /// Called by the field's label unless it has a name of its own, judged as the field is unless the app judged it
  /// itself, and disabled where the field is. A field converted into an element holds it as it is built, which does
  /// not record: a field the app mounts ([`Field::mount`](super::Field::mount)) mounts it, and it records.
  fn in_field(mut self, field: &super::InField) -> Element {
    self.boxed.in_field(field);
    self.into()
  }

  /// Told what the field says, as in `in_field`, and mounted: the same control as outside a field.
  fn mount_in_field(mut self, field: &super::InField, ctx: &mut Ctx) -> Element {
    self.boxed.in_field(field);
    self.mount(ctx)
  }
}

/// A shortcut recorder with the app's type left behind.
#[derive(Clone)]
struct Core {
  shortcut: Option<Accelerator>,
  optional: bool,
  change: Change,
  checks: Vec<ShortcutCheck>,
  require_modifier: bool,
  platform: KeyPlatform,
  boxed: BoxStates,
  full_width: bool,
  placeholder: Arc<str>,
  recording_text: Arc<str>,
  clear_label: Arc<str>,
  modifier_text: Arc<str>,
  #[cfg(feature = "form")]
  name: Option<Arc<str>>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

/// What a mounted recorder keeps: whether it records, the modifiers held while it does, and the combination it last
/// refused with why.
#[derive(Clone)]
struct Live {
  recording: Signal<bool>,
  held: Signal<SyntheticModifiers>,
  refused: Signal<Option<(Accelerator, Arc<str>)>>,
}

impl Core {
  fn build(self, theme: &Theme, live: Option<Live>) -> Element {
    let interactive = !self.boxed.read_only && !self.boxed.disabled;
    let live = live.filter(|_| interactive);
    let recording = live.as_ref().is_some_and(|live| live.recording.get());
    let refused = live.as_ref().and_then(|live| live.refused.get());
    let held = live.as_ref().map(|live| live.held.get()).unwrap_or_default();
    let case = self
      .boxed
      .case(self.states.clone())
      .state(ShortcutRecorderState::Recording, recording)
      .state(ShortcutRecorderState::Empty, self.shortcut.is_none())
      .state(ShortcutRecorderState::Refused, refused.is_some())
      .state(ShortcutRecorderState::FullWidth, self.full_width);
    let recipe = theme.resolve::<ShortcutRecorder>(&case, Some(&self.rules));
    let view = ShortcutRecorderView {
      shortcut: self.shortcut.clone(),
      recording,
      held,
      refused,
      read_only: self.boxed.read_only,
      disabled: self.boxed.disabled,
      judgment: self.boxed.judgment(),
      label: self
        .boxed
        .label
        .clone()
        .unwrap_or_else(|| super::current_locale().fields.shortcut.clone()),
      size: self.boxed.size.clone(),
      recipe,
      core: self,
      live,
      control: ElementRef::new(),
    };
    match theme.renderer::<ShortcutRecorder>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

/// Whether `key` is a modifier, and which.
fn modifier_of(key: &str) -> Option<fn(&mut SyntheticModifiers, bool)> {
  Some(match key {
    "Shift" => |m, held| m.shift = held,
    "Control" => |m, held| m.ctrl = held,
    "Alt" | "AltGraph" => |m, held| m.alt = held,
    "Meta" | "OS" | "Super" => |m, held| m.meta = held,
    _ => return None,
  })
}

/// The modifiers an event says are held.
fn modifiers(event: &KeyboardEvent) -> SyntheticModifiers {
  SyntheticModifiers {
    shift: event.shift,
    ctrl: event.ctrl,
    alt: event.alt,
    meta: event.meta,
  }
}

/// Whether `key` is a function key, which is taken with no modifier.
fn function_key(key: &str) -> bool {
  key
    .strip_prefix('F')
    .and_then(|number| number.parse::<u32>().ok())
    .is_some_and(|number| (1..=24).contains(&number))
}

/// A shortcut in the words of `platform`, as a person hears it: "Ctrl+Shift+P", "Cmd+Shift+P".
fn spoken(shortcut: &Accelerator, platform: KeyPlatform) -> String {
  let mac = platform == KeyPlatform::Mac;
  let m = shortcut.modifiers;
  let locale = super::current_locale();
  let keys = &locale.keys;
  let mut words: Vec<&str> = Vec::new();
  for (held, word, mac_word) in [
    (m.ctrl, &keys.ctrl, &keys.control),
    (m.alt, &keys.alt, &keys.option),
    (m.shift, &keys.shift, &keys.shift),
    (m.meta, &keys.meta, &keys.cmd),
  ] {
    if held {
      words.push(if mac { mac_word } else { word });
    }
  }
  let written = shortcut.to_string();
  let key = written.rsplit('+').next().filter(|key| !key.is_empty()).unwrap_or("+");
  words.push(key);
  words.join("+")
}

/// A shortcut recorder as a renderer sees it: what it holds and shows, the style of its parts, and its behavior ready
/// to be dressed.
pub struct ShortcutRecorderView {
  /// The shortcut it holds, where it holds one.
  pub shortcut: Option<Accelerator>,
  /// Whether it records: it takes the next combination pressed.
  pub recording: bool,
  /// The modifiers held while it records.
  pub held: SyntheticModifiers,
  /// The combination it last refused, with why, until it records again.
  pub refused: Option<(Accelerator, Arc<str>)>,
  pub read_only: bool,
  pub disabled: bool,
  /// The judgment the app or its field gave it.
  pub judgment: Option<Judgment>,
  pub label: Arc<str>,
  pub size: Name,
  /// The style of each part of this recorder.
  pub recipe: ResolvedRecipe,
  core: Core,
  live: Option<Live>,
  control: ElementRef,
}

impl ShortcutRecorderView {
  /// The style of a part, as this recorder is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced())
  }

  fn forced(&self) -> Option<Interaction> {
    self.core.forced
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// Sets `shortcut`, or none, where it is another than the one held, and stops recording.
  fn setter(&self) -> impl Fn(Option<Accelerator>) + Send + Sync + Clone + 'static {
    let (change, held, live) = (self.core.change.clone(), self.shortcut.clone(), self.live.clone());
    move |shortcut: Option<Accelerator>| {
      if let Some(live) = &live {
        live.recording.set(false);
        live.held.set(SyntheticModifiers::default());
        live.refused.set(None);
      }
      if shortcut != held {
        change(shortcut);
      }
    }
  }

  /// What a combination pressed while it records does: taken, it is set; refused, the box says why and keeps what it
  /// held.
  fn taker(&self) -> impl Fn(Accelerator) + Send + Sync + Clone + 'static {
    let set = self.setter();
    let (checks, require, modifier_text, live) = (
      self.core.checks.clone(),
      self.core.require_modifier,
      self.core.modifier_text.clone(),
      self.live.clone(),
    );
    move |shortcut: Accelerator| {
      let refusal = if require && !shortcut.has_modifier() && !function_key(&shortcut.key) {
        Some(modifier_text.clone())
      } else {
        checks.iter().find_map(|check| check(&shortcut).err().map(Arc::from))
      };
      match refusal {
        Some(why) => {
          if let Some(live) = &live {
            live.recording.set(false);
            live.held.set(SyntheticModifiers::default());
            live.refused.set(Some((shortcut, why)));
          }
        }
        None => set(Some(shortcut)),
      }
    }
  }

  /// The keys of `shortcut` as a [`Kbd`] draws them, in the words of the recorder's platform.
  pub fn kbd(&self, shortcut: &Accelerator) -> Kbd {
    let size = if self.size == Name::from(TextBoxSize::Lg) {
      KbdSize::Lg
    } else {
      KbdSize::Md
    };
    Kbd::accelerator(shortcut).platform(self.core.platform).size(size)
  }

  /// The box with its behavior and the style of its part, with nothing in it yet: what is pressed, focused, and takes
  /// every key while it records. A button to tooling, called by its label, that says the shortcut it holds.
  pub fn control(&self) -> Row {
    let shortcut_text = match &self.shortcut {
      Some(shortcut) => spoken(shortcut, self.core.platform),
      None => self.core.placeholder.to_string(),
    };
    let mut control = Row::new()
      .align_items(Alignment::Center)
      .part(&self.part(ShortcutRecorderPart::Control))
      .ref_element(self.control.clone())
      .describe("a11y_role", "button")
      .describe("a11y_name", self.label.clone())
      .describe("a11y_value", shortcut_text)
      .describe("a11y_pressed", if self.recording { "true" } else { "false" });
    if self.judgment == Some(Judgment::Invalid) {
      control = control.describe("a11y_invalid", "true");
    }
    if let Some((_, why)) = &self.refused {
      control = control
        .describe("a11y_invalid", "true")
        .describe("a11y_description", why.clone());
    }
    if self.read_only {
      control = control.describe("a11y_readonly", "true");
    }
    if self.disabled {
      control = control.describe("a11y_disabled", "true").disabled(true);
    } else {
      control = control.tab_index(0);
    }
    #[cfg(feature = "form")]
    if let Some(name) = &self.core.name {
      control = control
        .name(name.clone())
        .form_value(self.shortcut.as_ref().map(|shortcut| Arc::from(shortcut.to_string())));
    }
    let interactive = !self.read_only && !self.disabled;
    if let Some(live) = self.live.clone().filter(|_| interactive) {
      let (set, take, optional) = (self.setter(), self.taker(), self.core.optional);
      if self.recording {
        let (down, up, stop) = (live.clone(), live.clone(), live.clone());
        control = control
          .captures_keys(true)
          .on_key_down(move |event: KeyboardEvent| {
            event.prevent_default();
            if let Some(hold) = modifier_of(&event.key) {
              let mut held = modifiers(&event);
              hold(&mut held, true);
              down.held.set(held);
              return;
            }
            let plain = !(event.ctrl || event.alt || event.meta || event.shift);
            match event.key.as_str() {
              "Escape" if plain => {
                down.recording.set(false);
                down.held.set(SyntheticModifiers::default());
              }
              "Backspace" | "Delete" if plain && optional => set(None),
              _ => {
                if let Some(shortcut) = Accelerator::from_key_event(&event) {
                  take(shortcut);
                }
              }
            }
          })
          .on_key_up(move |event: KeyboardEvent| {
            let mut held = modifiers(&event);
            if let Some(release) = modifier_of(&event.key) {
              release(&mut held, false);
            }
            up.held.set(held);
          })
          .on_blur(move || {
            stop.recording.set(false);
            stop.held.set(SyntheticModifiers::default());
          });
      } else {
        let (pressed, keyed) = (live.clone(), live);
        let clear = self.setter();
        control = control
          .focuses(&self.control)
          .on_click(move |_| {
            pressed.refused.set(None);
            pressed.recording.set(true);
          })
          .on_key_down(move |event: KeyboardEvent| {
            if !event.focus_within || event.ctrl || event.alt || event.meta || event.shift || event.composing {
              return;
            }
            match event.key.as_str() {
              "Enter" | " " => {
                event.prevent_default();
                keyed.refused.set(None);
                keyed.recording.set(true);
              }
              "Backspace" | "Delete" if optional => {
                event.prevent_default();
                clear(None);
              }
              _ => {}
            }
          });
      }
    }
    if let Some(id) = &self.core.id {
      control = control.id(id.clone());
    }
    if let Some(key) = &self.core.key {
      control = control.key(key.clone());
    }
    control
  }

  /// What the box shows: the keys of the shortcut, or of the modifiers held while it records with the dot before
  /// them, or the hint where there is neither; the combination refused, while the box says why.
  pub fn content(&self) -> Vec<Element> {
    let mut content = Vec::new();
    if self.recording {
      content.push(Row::new().part(&self.part(ShortcutRecorderPart::Dot)).into());
    }
    let shown = if self.recording {
      let held = self.held;
      (held.ctrl || held.alt || held.shift || held.meta).then(|| Accelerator::new("", held))
    } else {
      self
        .refused
        .as_ref()
        .map(|(shortcut, _)| shortcut.clone())
        .or_else(|| self.shortcut.clone())
    };
    match shown {
      Some(shortcut) => {
        let keys = if shortcut.key.is_empty() {
          // The modifiers held, with no key yet.
          let m = shortcut.modifiers;
          let words: Vec<&str> = [(m.ctrl, "Ctrl"), (m.alt, "Alt"), (m.shift, "Shift"), (m.meta, "Meta")]
            .into_iter()
            .filter_map(|(held, word)| held.then_some(word))
            .collect();
          Kbd::shortcut(&words.join("+"))
            .platform(self.core.platform)
            .size(KbdSize::Md)
        } else {
          self.kbd(&shortcut)
        };
        content.push(
          Row::new()
            .align_items(Alignment::Center)
            .part(&self.part(ShortcutRecorderPart::Keys))
            .child(keys.build(self.theme()))
            .into(),
        );
      }
      None => {
        let part = self.part(ShortcutRecorderPart::Hint);
        let text = if self.recording {
          &self.core.recording_text
        } else {
          &self.core.placeholder
        };
        content.push(Row::new().part(&part).child(part.text(text).nowrap()).into());
      }
    }
    content
  }

  /// The cross that empties it, where it can hold none and holds a shortcut, and is neither read only nor disabled:
  /// a press empties it and stops recording. No stop of Tab: Backspace and Delete do as much from the keyboard.
  pub fn clear(&self) -> Option<Element> {
    let interactive = !self.read_only && !self.disabled;
    if !self.core.optional || self.shortcut.is_none() || !interactive {
      return None;
    }
    let set = self.setter();
    let part = self.part(ShortcutRecorderPart::Clear);
    Some(
      Row::new()
        .part(&part)
        .describe("a11y_role", "button")
        .describe("a11y_name", self.core.clear_label.clone())
        .on_click(move |_| set(None))
        .child(glyph(&part, Name::new(icons::CLOSE), false))
        .into(),
    )
  }

  /// What is said under the box of a combination refused: why.
  pub fn message(&self) -> Option<Element> {
    let (_, why) = self.refused.as_ref()?;
    let part = self.part(ShortcutRecorderPart::Message);
    Some(Row::new().part(&part).child(part.text(why)).into())
  }

  /// How the default renderer builds a recorder: the box with what it shows, room, and the cross at its end, and
  /// under it why a combination was refused.
  pub fn render(self) -> Element {
    let mut control = self.control().with_children(self.content()).child(Row::new().flex(1.0));
    if let Some(clear) = self.clear() {
      control = control.child(clear);
    }
    let mut root = Column::new()
      .align_items(Alignment::Stretch)
      .overflow_visible()
      .part(&self.part(ShortcutRecorderPart::Root))
      .child(control);
    if let Some(message) = self.message() {
      root = root.child(message);
    }
    root.into()
  }
}

/// What a mounted recorder is given: the recorder, built anew by every render of what holds it.
struct RecorderProps {
  core: Core,
}

impl PartialEq for RecorderProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

#[cfg(feature = "devtools")]
impl crate::app::component::DevtoolsInspectable for RecorderProps {}

/// What a mounted recorder keeps.
struct RecorderHost {
  live: Live,
}

impl Component for RecorderHost {
  type Props = RecorderProps;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      live: Live {
        recording: ctx.signal(false),
        held: ctx.signal(SyntheticModifiers::default()),
        refused: ctx.signal(None),
      },
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let core = ctx.props::<RecorderProps>().core.clone();
    core.build(&current_theme(), Some(self.live.clone()))
  }
}

impl Themed for ShortcutRecorder {
  const NAME: &'static str = "shortcut_recorder";
  type Renderer = ShortcutRecorderRenderer;

  /// The design's shortcut recorder (`pencil/lurq-design-system.pen`, Shortcut Recorder): a box 230 wide and 32 high,
  /// a box's fill and edge with the medium radius, 8 in at its start and 6 at its end, its keys 4 apart as
  /// [`Kbd`]s draw them, 6 from the cross of 14 in the muted foreground at its end. Recording, its edge is a line of
  /// 2 in the ring's color, and a dot of 8 in the destructive color stands before "Press shortcut…" in the muted
  /// foreground at 13. A combination refused edges it in the destructive color, and says why 6 under it at 12 in
  /// that color. The rest is a box's: the ring's color under the pointer, a judgment's color, read only, disabled.
  fn recipe() -> Recipe {
    use PaletteColor::{Destructive, Foreground, MutedForeground, Ring};
    use ShortcutRecorderPart::{Clear, Control, Dot, Hint, Keys, Message, Root};
    use ShortcutRecorderState::{FullWidth, Recording, Refused};
    let recipe = Recipe::new()
      .base(Root, |s| s.width(230.0).gap(6.0))
      .base(Control, |s| {
        text_box::box_style(s)
          .height(ControlSize::Md)
          .padding_x(0.0)
          .padding_start(8.0)
          .padding_end(6.0)
          .gap(6.0)
          .foreground(Foreground)
          .typography(TypographyStyle::Body)
          .cursor(CursorIcon::Pointer)
      })
      .base(Keys, |s| s.gap(4.0))
      .base(Dot, |s| s.square(8.0).radius(RadiusSize::Full).background(Destructive))
      .base(Hint, |s| s.foreground(MutedForeground))
      .base(Clear, |s| {
        fading(
          s.square(IconSize::Sm)
            .foreground(MutedForeground)
            .cursor(CursorIcon::Pointer),
        )
      })
      .hovered(Clear, |s| s.foreground(Foreground))
      .base(Message, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(12.0)
          .foreground(Destructive)
      })
      .state(FullWidth, Root, |s| s.width(Length::Fill))
      .state(Recording, Control, |s| s.outline(text_box::ring(Ring)));
    let recipe = text_box::box_sizes(recipe, Control, true);
    text_box::box_rules(recipe, Root, Control, Hint)
      // A refusal keeps its edge under the pointer, as a judgment does.
      .state(Refused, Control, |s| s.border(text_box::edge(Destructive)))
      .when(Selector::new().state(Refused).hovered(), Control, |s| {
        s.border(text_box::edge(Destructive))
      })
      .state(super::TextBoxState::ReadOnly, Control, |s| s.cursor(CursorIcon::Default))
  }
}
