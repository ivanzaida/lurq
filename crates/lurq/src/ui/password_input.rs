//! A password input: an input whose text is masked, with a button at its end that shows it and hides it again.

use std::sync::Arc;

use super::{
  field::{FieldControl, InField},
  icon::Icon,
  icons,
  input::{Input, InputView},
  names,
  part_style::{Length, PartStyle},
  recipe::{Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::{fading, pressable},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    events::TextInputEvent,
    theme::{IconSize, PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::{self, Column, Row},
  core::Signal,
  layout::Alignment,
  node::{CursorIcon, Element, dimension::Dimension, node_kind::ContentKind},
};

/// The parts of a password input a recipe styles. The box, its text and its placeholder are an
/// [`Input`]'s, styled by the theme's recipe for an input.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PasswordInputPart {
  /// The input, and the meter and its message under it.
  Root,
  /// The masked text: the room between its dots.
  Mask,
  /// The button at the end of the box that shows the password and hides it again.
  Toggle,
  /// The meter of the password's strength: its bars side by side.
  Strength,
  /// A bar of the meter the strength does not reach.
  Bar,
  /// A bar of the meter the strength reaches, in the strength's color.
  Filled,
  /// What the meter says of the strength, under it.
  Message,
}

names!(PasswordInputPart {
  Root => "root",
  Mask => "mask",
  Toggle => "toggle",
  Strength => "strength",
  Bar => "bar",
  Filled => "filled",
  Message => "message",
});

/// The states of a password input a recipe styles, besides an input's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PasswordInputState {
  /// The password is shown as it is typed.
  Revealed,
  /// It takes no input.
  Disabled,
  /// It is as wide as what holds it, with its meter.
  FullWidth,
}

names!(PasswordInputState {
  Revealed => "revealed",
  Disabled => "disabled",
  FullWidth => "full_width",
});

/// How strong the app judges a password to be: what the meter under the input shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PasswordStrength {
  /// One bar in the destructive color.
  Weak,
  /// Two bars in the warning color.
  Fair,
  /// Three bars in the success color.
  Good,
  /// All four bars in the success color.
  Strong,
}

names!(PasswordStrength {
  Weak => "weak",
  Fair => "fair",
  Good => "good",
  Strong => "strong",
});

impl PasswordStrength {
  /// How many of the meter's bars it fills.
  pub fn bars(self) -> usize {
    match self {
      Self::Weak => 1,
      Self::Fair => 2,
      Self::Good => 3,
      Self::Strong => 4,
    }
  }

  /// What the meter says of it unless the app says otherwise.
  pub fn message(self) -> &'static str {
    match self {
      Self::Weak => "Weak password",
      Self::Fair => "Fair password",
      Self::Good => "Good password",
      Self::Strong => "Strong password",
    }
  }
}

/// What builds a password input's nodes: a theme's in place of the default one.
pub type PasswordInputRenderer = Arc<dyn Fn(PasswordInputView) -> Element + Send + Sync>;

type RevealHandler = Arc<dyn Fn(bool) + Send + Sync>;

/// A password input: an [`Input`] whose text is masked, with a button at the
/// end of the box that shows the password and hides it again.
///
/// ```
/// use lurq::{core::Signal, ui::{PasswordInput, PasswordStrength}};
///
/// let password = Signal::new(String::new());
/// let shown = Signal::new(false);
/// let input = PasswordInput::new(password.clone())
///   .placeholder("Password")
///   .bind_reveal(shown.clone())
///   .strength(Some(PasswordStrength::Good));
/// ```
///
/// It is an input in every other way: its sizes, its judgments, read only,
/// disabled, a field's label. The reveal is the app's: a `bool` it says
/// with [`reveal`](Self::reveal) and hears of with
/// [`on_reveal`](Self::on_reveal), or a `Signal<bool>` it binds. The button
/// is drawn where something hears it, after the input in the order of Tab,
/// and is called "Show password" or "Hide password" to tooling. A masked
/// password is not copied, and tooling reads only its mask; shown, it is
/// what the screen shows, to tooling too. Under the input the app may show
/// a meter of the password's strength and what it says of it.
pub struct PasswordInput {
  input: Input,
  revealed: bool,
  on_reveal: Vec<RevealHandler>,
  labels: (Arc<str>, Arc<str>),
  strength: Option<PasswordStrength>,
  message: Option<Arc<str>>,
  new_password: bool,
  full_width: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
}

impl PasswordInput {
  /// A password input bound to `value`: what is typed is set on it.
  pub fn new(value: Signal<String>) -> Self {
    Self {
      input: Input::new(value),
      revealed: false,
      on_reveal: Vec::new(),
      labels: (Arc::from("Show password"), Arc::from("Hide password")),
      strength: None,
      message: None,
      new_password: false,
      full_width: false,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
    }
  }

  /// What the input says while it is empty.
  pub fn placeholder(mut self, placeholder: &str) -> Self {
    self.input = self.input.placeholder(placeholder);
    self
  }

  /// A glyph of the theme's set at the start of the box: a lock.
  pub fn icon(mut self, name: impl Into<Name>) -> Self {
    self.input = self.input.icon(name);
    self
  }

  /// An [`InputSize`](super::InputSize), or a size the theme's recipe names.
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.input = self.input.size(size);
    self
  }

  /// Whether the password is shown as it is typed, in place of its mask.
  pub fn reveal(mut self, revealed: bool) -> Self {
    self.revealed = revealed;
    self
  }

  /// Shown while `revealed` is true, which the button turns.
  pub fn bind_reveal(self, revealed: Signal<bool>) -> Self {
    let set = revealed.clone();
    self.reveal(revealed.get()).on_reveal(move |next| set.set(next))
  }

  /// Called on a press of the button with whether the password is to be shown.
  pub fn on_reveal(mut self, handler: impl Fn(bool) + Send + Sync + 'static) -> Self {
    self.on_reveal.push(Arc::new(handler));
    self
  }

  /// What the button is called to tooling while the password is hidden and while it is shown, in place of "Show
  /// password" and "Hide password": in another language.
  pub fn reveal_labels(mut self, show: &str, hide: &str) -> Self {
    self.labels = (Arc::from(show), Arc::from(hide));
    self
  }

  /// The strength the meter under the input shows, with what it says of it. `None` draws no meter.
  pub fn strength(mut self, strength: Option<PasswordStrength>) -> Self {
    self.strength = strength;
    self
  }

  /// What the meter says of the strength, in place of "Strong password" and the others.
  pub fn strength_message(mut self, message: &str) -> Self {
    self.message = Some(Arc::from(message));
    self
  }

  /// Whether the password is being chosen, not entered: the platform may offer a strong one.
  pub fn new_password(mut self, new_password: bool) -> Self {
    self.new_password = new_password;
    self
  }

  /// Whether what the input holds is wrong.
  pub fn invalid(mut self, invalid: bool) -> Self {
    self.input = self.input.invalid(invalid);
    self
  }

  /// Whether what the input holds is allowed with a caveat.
  pub fn warning(mut self, warning: bool) -> Self {
    self.input = self.input.warning(warning);
    self
  }

  /// Whether what the input holds was checked and is right.
  pub fn valid(mut self, valid: bool) -> Self {
    self.input = self.input.valid(valid);
    self
  }

  /// Whether the input can be read and not changed. A masked one cannot be copied either.
  pub fn read_only(mut self, read_only: bool) -> Self {
    self.input = self.input.read_only(read_only);
    self
  }

  /// Whether the input and its button take no input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.input = self.input.disabled(disabled);
    self
  }

  /// What the input is called, to what reads the tree. A field gives it its label.
  pub fn label(mut self, label: &str) -> Self {
    self.input = self.input.label(label);
    self
  }

  /// Whether the input is as wide as what holds it, in place of the design's 260.
  pub fn full_width(mut self, full_width: bool) -> Self {
    self.full_width = full_width;
    self.input = self.input.full_width(full_width);
    self
  }

  /// Called before an edit is made, with what the input will hold: a handler may refuse it.
  pub fn on_input(mut self, handler: impl Fn(&TextInputEvent) + Send + Sync + 'static) -> Self {
    self.input = self.input.on_input(handler);
    self
  }

  /// The name the password has in the form it is in.
  #[cfg(feature = "form")]
  pub fn name(mut self, name: impl Into<Arc<str>>) -> Self {
    self.input = self.input.name(name);
    self
  }

  /// Puts the password input in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this password input over what the theme gives it. The box is styled with
  /// [`input_rules`](Self::input_rules).
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this password input.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Lays rules over the theme's recipe for the input it is made of: its box, its text, its placeholder.
  pub fn input_rules(mut self, rules: &Recipe) -> Self {
    self.input = self.input.rules(rules);
    self
  }

  /// Draws the input as it is under the pointer or focused, with nothing pointing at it.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self.input = self.input.force(interaction);
    self
  }

  /// The id of the control: what is typed in and focused.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.input = self.input.id(id);
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.input = self.input.key(key);
    self
  }

  /// Builds the password input in `theme`. Converting it into an element does this in the theme of what is being
  /// rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let disabled = self.input.boxed.disabled;
    let mut case = self
      .input
      .boxed
      .case(self.states.clone())
      .state(PasswordInputState::Revealed, self.revealed)
      .state(PasswordInputState::Disabled, disabled)
      .state(PasswordInputState::FullWidth, self.full_width);
    if let Some(strength) = self.strength {
      case = case.state(strength, true);
    }
    let view = PasswordInputView {
      revealed: self.revealed,
      disabled,
      toggle_label: if self.revealed { self.labels.1 } else { self.labels.0 },
      strength: self.strength,
      message: self
        .message
        .or_else(|| self.strength.map(|strength| Arc::from(strength.message()))),
      new_password: self.new_password,
      recipe: theme.resolve::<PasswordInput>(&case, Some(&self.rules)),
      forced: self.forced,
      on_reveal: self.on_reveal,
      input: self.input.view(theme),
    };
    match theme.renderer::<PasswordInput>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl FieldControl for PasswordInput {
  /// Called by the field's label unless it has a name of its own, judged as the field is unless the app judged the
  /// input itself, and disabled where the field is.
  fn in_field(mut self, field: &InField) -> Element {
    self.input.boxed.in_field(field);
    self.into()
  }
}

impl From<PasswordInput> for Element {
  fn from(input: PasswordInput) -> Self {
    input.build(&current_theme())
  }
}

/// A password input as a renderer sees it: the input it is made of, the style of its own parts, and its behavior.
pub struct PasswordInputView {
  /// The input the password is typed in: its box, its text, its placeholder, its icon.
  pub input: InputView,
  pub revealed: bool,
  pub disabled: bool,
  /// What the button is called to tooling: "Show password" while the password is hidden.
  pub toggle_label: Arc<str>,
  pub strength: Option<PasswordStrength>,
  /// What the meter says of the strength, where it has one.
  pub message: Option<Arc<str>>,
  pub new_password: bool,
  /// The style of each of the password input's own parts.
  pub recipe: ResolvedRecipe,
  forced: Option<Interaction>,
  on_reveal: Vec<RevealHandler>,
}

impl PasswordInputView {
  /// The style of a part, as this password input is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The side of the button, in pixels.
  fn toggle_side(&self) -> f32 {
    match self
      .part(PasswordInputPart::Toggle)
      .rest()
      .width
      .and_then(|width| width.resolve(self.theme()))
    {
      Some(Dimension::Px(px)) => px,
      _ => 0.0,
    }
  }

  /// Whether the button is drawn: something hears a press of it.
  pub fn has_toggle(&self) -> bool {
    !self.on_reveal.is_empty()
  }

  /// The input's text input, masked unless the password is shown, and keeping room at its end for the button.
  /// Shown or not, it holds a password to the platform, which may offer to fill it in.
  pub fn control(&self) -> components::TextInput {
    let content = if self.new_password {
      ContentKind::NewPassword
    } else {
      ContentKind::Password
    };
    let mut control = self.input.control().content(content);
    control = if self.revealed {
      control.unmask()
    } else {
      let spacing = self.part(PasswordInputPart::Mask).rest().text.letter_spacing;
      let control = control.mask();
      match spacing {
        Some(spacing) => control.letter_spacing(spacing),
        None => control,
      }
    };
    if self.has_toggle() {
      control = control.padding_right(self.input.end_room(self.toggle_side()));
    }
    control
  }

  /// The button that shows the password and hides it again: the eye, a stop of Tab after the input.
  pub fn toggle(&self) -> Option<components::Button> {
    if !self.has_toggle() {
      return None;
    }
    let part = self.part(PasswordInputPart::Toggle);
    let side = part.rest().width.unwrap_or(Length::Icon(IconSize::Md));
    let mut toggle = pressable(&part, self.disabled)
      .describe("a11y_role", "button")
      .describe("a11y_name", self.toggle_label.clone())
      .describe("a11y_pressed", if self.revealed { "true" } else { "false" })
      .child(Icon::new(icons::EYE).size(side).build(self.theme()));
    if !self.disabled {
      let (handlers, next) = (self.on_reveal.clone(), !self.revealed);
      toggle = toggle.on_click(move |_| {
        for handler in &handlers {
          handler(next);
        }
      });
    }
    Some(toggle)
  }

  /// The meter of the strength: four bars, as many filled as the strength reaches.
  pub fn strength_meter(&self) -> Option<Row> {
    let filled = self.strength?.bars();
    let (bar, full) = (self.part(PasswordInputPart::Bar), self.part(PasswordInputPart::Filled));
    let mut meter = Row::new()
      .width(Dimension::Pct(100.0))
      .part(&self.part(PasswordInputPart::Strength));
    for index in 0..4 {
      meter = meter.child(Row::new().flex(1.0).part(if index < filled { &full } else { &bar }));
    }
    Some(meter)
  }

  /// What the meter says of the strength.
  pub fn message(&self) -> Option<Row> {
    let message = self.message.as_ref()?;
    let part = self.part(PasswordInputPart::Message);
    Some(Row::new().part(&part).child(part.text(message)))
  }

  /// How the default renderer builds a password input: the input with its button over the end of the box, and the
  /// meter and its message under it.
  pub fn render(self) -> Element {
    let end = self.toggle().map(|toggle| Row::new().child(toggle));
    let input = self.input.render_with_end(self.control(), end);
    let (meter, message) = (self.strength_meter(), self.message());
    if meter.is_none() && message.is_none() {
      return input.into();
    }
    // The input keeps its width; the meter and its message are as wide as it is.
    let mut root = Column::new()
      .align_items(Alignment::Stretch)
      .part(&self.part(PasswordInputPart::Root))
      .child(input);
    if let Some(meter) = meter {
      root = root.child(meter);
    }
    if let Some(message) = message {
      root = root.child(message);
    }
    root.into()
  }
}

impl Themed for PasswordInput {
  const NAME: &'static str = "password_input";
  type Renderer = PasswordInputRenderer;

  /// The design's password input (`pencil/lurq-design-system.pen`, Password Input): an input whose dots are 1
  /// apart, with an eye of 16 in the muted foreground at the end of the box, 12 from its edge and 8 after the text.
  /// Under it, 8 apart, a meter of four bars 4 high and 4 apart, fully round, on the input's color where the
  /// strength does not reach and in the strength's color where it does, and a message in a caption's type in that
  /// color: destructive for a weak password, warning for a fair one, success for a good or a strong one. The eye
  /// is in the foreground under the pointer and while the password is shown.
  fn recipe() -> Recipe {
    use PaletteColor::{Destructive, Foreground, Input, MutedForeground, Success, Warning};
    use PasswordInputPart::{Bar, Filled, Mask, Message, Root, Strength, Toggle};
    let recipe = Recipe::new()
      .base(Root, |s| s.gap(8.0))
      .state(PasswordInputState::FullWidth, Root, |s| s.width(Length::Fill))
      .base(Mask, |s| s.letter_spacing(1.0))
      .base(Toggle, |s| {
        fading(
          s.square(IconSize::Md)
            .radius(RadiusSize::Sm)
            .foreground(MutedForeground)
            .cursor(CursorIcon::Pointer),
        )
      })
      .hovered(Toggle, |s| s.foreground(Foreground))
      .focus_visible(Toggle, |s| s.focus_ring())
      .state(PasswordInputState::Revealed, Toggle, |s| s.foreground(Foreground))
      .state(PasswordInputState::Disabled, Toggle, |s| s.cursor(CursorIcon::Default))
      .base(Strength, |s| s.gap(4.0))
      .base(Bar, |s| s.height(4.0).radius(RadiusSize::Full).background(Input))
      .base(Filled, |s| s.height(4.0).radius(RadiusSize::Full).background(Success))
      .base(Message, |s| s.typography(TypographyStyle::Caption).foreground(Success));
    [(PasswordStrength::Weak, Destructive), (PasswordStrength::Fair, Warning)]
      .into_iter()
      .fold(recipe, |recipe, (strength, color)| {
        recipe
          .state(strength, Filled, |s| s.background(color.clone()))
          .state(strength, Message, |s| s.foreground(color))
      })
  }
}
