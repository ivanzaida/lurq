//! An input: one line of text that a person types, bound to a signal.

use std::sync::Arc;

use super::{
  icon::Icon,
  names,
  part_style::{Length, PartStyle, PartTransition},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::fading,
  theme::{Themed, current_theme},
};
use crate::{
  animation::Transition,
  app::{
    events::TextInputEvent,
    theme::{
      BorderSize, ControlSize, IconSize, MotionDuration, OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle,
    },
  },
  components::{self, Row, Stack},
  core::Signal,
  layout::{Alignment, StackAlignment, text_style::TextStyle},
  node::{CursorIcon, Element, TextColor, border::Border, dimension::Dimension},
};

/// The parts of an input a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InputPart {
  /// The place the input takes: its width, and the room between its icon and its text.
  Root,
  /// The box that is typed in and focused: its edge, its fill, its height, and the type of its value.
  Control,
  /// What the box says while it is empty. It has the type of the control, in its own color.
  Placeholder,
  /// The glyph at the start of the box, where the input has one.
  Icon,
}

names!(InputPart {
  Root => "root",
  Control => "control",
  Placeholder => "placeholder",
  Icon => "icon",
});

/// The sizes of an input: the heights of the theme's controls.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum InputSize {
  /// 32 high, its text 13.
  Sm,
  /// 36 high, its text 14.
  #[default]
  Md,
  /// 40 high, its text 15.
  Lg,
}

names!(InputSize {
  Sm => "sm",
  Md => "md",
  Lg => "lg",
});

/// The states of an input a recipe styles, besides what the pointer and the
/// keyboard do to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InputState {
  /// What it holds is wrong: its edge is in the destructive color.
  Invalid,
  /// What it holds is allowed, with a caveat: its edge is in the warning color.
  Warning,
  /// What it holds was checked and is right: its edge is in the success color.
  Valid,
  /// It can be read, selected and copied, and not changed.
  ReadOnly,
  /// It takes no input.
  Disabled,
  /// It is as wide as what holds it.
  FullWidth,
  /// It has an icon at its start.
  WithIcon,
}

names!(InputState {
  Invalid => "invalid",
  Warning => "warning",
  Valid => "valid",
  ReadOnly => "read_only",
  Disabled => "disabled",
  FullWidth => "full_width",
  WithIcon => "with_icon",
});

/// What builds an input's nodes: a theme's in place of the default one.
pub type InputRenderer = Arc<dyn Fn(InputView) -> Element + Send + Sync>;

type InputHandler = Arc<dyn Fn(&TextInputEvent) + Send + Sync>;

/// An input: one line of text that a person types. A name, an email
/// address, a search.
///
/// ```
/// use lurq::{core::Signal, ui::{Input, icons}};
///
/// let name = Signal::new(String::new());
/// let input = Input::new(name.clone()).placeholder("Display name");
/// let search = Input::new(Signal::new(String::new())).placeholder("Search components").icon(icons::SEARCH);
/// let email = Input::new(Signal::new("ivan@".to_owned())).invalid(true);
/// ```
///
/// It is the engine's [`TextInput`](crate::components::TextInput) in the
/// design's dress: what is typed goes into the signal it is given, and what
/// the app sets on the signal is what it shows. Its edge says its state: the
/// ring's color under the pointer and, twice as thick, while it is focused;
/// the destructive, warning or success color where the app says what it
/// holds is wrong, doubtful or right.
pub struct Input {
  value: Signal<String>,
  placeholder: Option<Arc<str>>,
  icon: Option<Name>,
  size: Name,
  invalid: bool,
  warning: bool,
  valid: bool,
  read_only: bool,
  disabled: bool,
  full_width: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  on_input: Vec<InputHandler>,
  name: Option<Arc<str>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Input {
  /// An input bound to `value`: it shows what the signal holds, and what is typed is set on it.
  pub fn new(value: Signal<String>) -> Self {
    Self {
      value,
      placeholder: None,
      icon: None,
      size: InputSize::default().into(),
      invalid: false,
      warning: false,
      valid: false,
      read_only: false,
      disabled: false,
      full_width: false,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      on_input: Vec::new(),
      name: None,
      id: None,
      key: None,
    }
  }

  /// What the input says while it is empty.
  pub fn placeholder(mut self, placeholder: &str) -> Self {
    self.placeholder = Some(Arc::from(placeholder));
    self
  }

  /// A glyph of the theme's set at the start of the box: a search's magnifier.
  pub fn icon(mut self, name: impl Into<Name>) -> Self {
    self.icon = Some(name.into());
    self
  }

  /// An [`InputSize`], or a size the theme's recipe names.
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.size = size.into();
    self
  }

  /// Whether what the input holds is wrong. Of invalid, warning and valid, the first that is set is the one shown.
  pub fn invalid(mut self, invalid: bool) -> Self {
    self.invalid = invalid;
    self
  }

  /// Whether what the input holds is allowed with a caveat.
  pub fn warning(mut self, warning: bool) -> Self {
    self.warning = warning;
    self
  }

  /// Whether what the input holds was checked and is right.
  pub fn valid(mut self, valid: bool) -> Self {
    self.valid = valid;
    self
  }

  /// Whether the input can be read, selected and copied, and not changed. It is still a stop of Tab.
  pub fn read_only(mut self, read_only: bool) -> Self {
    self.read_only = read_only;
    self
  }

  /// Whether the input takes no input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Whether the input is as wide as what holds it, in place of the design's 260.
  pub fn full_width(mut self, full_width: bool) -> Self {
    self.full_width = full_width;
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

  /// Called before an edit is made, with what the input will hold: a handler may refuse it
  /// ([`TextInputEvent::prevent_default`]).
  pub fn on_input(mut self, handler: impl Fn(&TextInputEvent) + Send + Sync + 'static) -> Self {
    self.on_input.push(Arc::new(handler));
    self
  }

  /// The name the input's value has in the form it is in.
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

  /// Lays rules over the theme's recipe for this input: what the component
  /// it is in says of the input in this place.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the input as it is under the pointer or focused, with nothing
  /// pointing at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the control: what is typed in and focused.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the input in `theme`. Converting an input into an element does
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
    .state(InputState::Invalid, invalid)
    .state(InputState::Warning, warning)
    .state(InputState::Valid, valid)
    .state(InputState::ReadOnly, self.read_only)
    .state(InputState::Disabled, self.disabled)
    .state(InputState::FullWidth, self.full_width)
    .state(InputState::WithIcon, self.icon.is_some());
    let view = InputView {
      value: self.value,
      placeholder: self.placeholder,
      icon: self.icon,
      invalid,
      read_only: self.read_only,
      disabled: self.disabled,
      recipe: theme.resolve::<Input>(&case, Some(&self.rules)),
      forced: self.forced,
      on_input: self.on_input,
      name: self.name,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Input>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<Input> for Element {
  fn from(input: Input) -> Self {
    input.build(&current_theme())
  }
}

/// An input as a renderer sees it: what it was given, the style of its
/// parts, and its behavior ready to be dressed.
pub struct InputView {
  /// What the input shows and what is typed is set on.
  pub value: Signal<String>,
  pub placeholder: Option<Arc<str>>,
  pub icon: Option<Name>,
  /// Whether what it holds is wrong, as the input shows it.
  pub invalid: bool,
  pub read_only: bool,
  pub disabled: bool,
  /// The style of each part of this input.
  pub recipe: ResolvedRecipe,
  forced: Option<Interaction>,
  on_input: Vec<InputHandler>,
  // Read only where the engine has forms.
  #[cfg_attr(not(feature = "form"), allow(dead_code))]
  name: Option<Arc<str>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl InputView {
  /// The style of a part, as this input is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The text of the part as a text input paints it: the part's type, in the
  /// color of its foreground. A text input does not take the foreground of
  /// its node, so the color is named.
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

  /// The type of the value: the control's.
  pub fn value_style(&self) -> TextStyle {
    self.ink(&self.part(InputPart::Control).rest())
  }

  /// The type of the placeholder: the control's, with what the placeholder's part lays over it.
  pub fn placeholder_style(&self) -> TextStyle {
    let control = self.part(InputPart::Control).rest();
    self.ink(&control.merged(&self.part(InputPart::Placeholder).rest()))
  }

  /// How far the text starts from the start of the box: the control's own
  /// padding, and past the icon and the room after it where the input has
  /// one. `None` where the recipe gives the icon no width in pixels.
  pub fn text_start(&self) -> Option<f32> {
    let theme = self.theme();
    let spacing = theme.spacing();
    let padding = self
      .part(InputPart::Control)
      .rest()
      .padding_x
      .map(|padding| padding.resolve(&spacing, 0.0))
      .unwrap_or(0.0);
    if self.icon.is_none() {
      return Some(padding);
    }
    let icon = match self
      .part(InputPart::Icon)
      .rest()
      .width
      .unwrap_or(Length::Icon(IconSize::Md))
      .resolve(theme)?
    {
      Dimension::Px(px) => px,
      _ => return None,
    };
    let gap = self
      .part(InputPart::Root)
      .rest()
      .gap
      .map(|gap| gap.resolve(&spacing, 0.0))
      .unwrap_or(0.0);
    Some(padding + icon + gap)
  }

  /// The input's behavior with the style of its control: the engine's text
  /// input bound to the value, which is typed in, focused and disabled. A
  /// read-only one refuses every edit.
  pub fn control(&self) -> components::TextInput {
    let mut control = components::TextInput::new(self.value.clone())
      .single_line()
      .text_style(self.value_style())
      .placeholder_style(self.placeholder_style())
      // A text input is a stop of Tab only in a form or with an index of its own; a control of the design system is
      // one wherever it is.
      .tab_index(0)
      .disabled(self.disabled)
      .part(&self.part(InputPart::Control))
      .describe("a11y_role", "textbox");
    if let Some(placeholder) = &self.placeholder {
      control = control.placeholder(placeholder);
    }
    if let Some(start) = self.text_start().filter(|_| self.icon.is_some()) {
      control = control.padding_left(start);
    }
    let (handlers, read_only) = (self.on_input.clone(), self.read_only);
    if read_only || !handlers.is_empty() {
      control = control.on_input(move |event: TextInputEvent| {
        if read_only {
          event.prevent_default();
          return;
        }
        for handler in &handlers {
          handler(&event);
        }
      });
    }
    #[cfg(feature = "form")]
    if let Some(name) = &self.name {
      control = control.name(name.clone());
    }
    if let Some(id) = &self.id {
      control = control.id(id.clone());
    }
    if let Some(key) = &self.key {
      control = control.key(key.clone());
    }
    control
  }

  /// The icon in the style of its part, at the start of the box. The pointer
  /// passes through it to the control under it.
  pub fn icon(&self) -> Option<Row> {
    let name = self.icon?;
    let part = self.part(InputPart::Icon);
    let side = part.rest().width.unwrap_or(Length::Icon(IconSize::Md));
    let padding = self
      .part(InputPart::Control)
      .rest()
      .padding_x
      .map(|padding| padding.resolve(&self.theme().spacing(), 0.0))
      .unwrap_or(0.0);
    Some(
      Row::new()
        .padding_left(padding)
        .align_items(Alignment::Center)
        .pointer_events_none()
        .child(
          Row::new()
            .part(&part)
            .child(Icon::new(name).size(side).build(self.theme())),
        ),
    )
  }

  /// How the default renderer builds an input: the control, and its icon laid over its start.
  pub fn render(self) -> Element {
    // The control is the box, so that the ring of its focus is its own edge; the icon lies over it.
    let mut root = Stack::new()
      .stack_align(StackAlignment::CenterStart)
      .part(&self.part(InputPart::Root))
      .child(self.control());
    if let Some(icon) = self.icon() {
      root = root.child(icon);
    }
    root.into()
  }
}

impl Themed for Input {
  const NAME: &'static str = "input";
  type Renderer = InputRenderer;

  /// The design's input (`pencil/lurq-design-system.pen`, Input): a box 260
  /// wide and 36 high, the background's fill in an edge of 1 in the color of
  /// an input's, 6 round, its text 12 from its edge in the body's type and
  /// its placeholder in the muted foreground. Its edge is the only signal of
  /// its state: the ring's color under the pointer and 2 thick while it is
  /// focused; the destructive, warning and success colors for what it holds.
  /// A disabled one is half opaque on the muted fill; a read-only one is on
  /// the muted fill in the color of a border. Its icon is 16, 8 before the
  /// text. Small is 32 with text of 13, large 40 with text of 15.
  fn recipe() -> Recipe {
    use InputPart::{Control, Icon, Placeholder, Root};
    use PaletteColor::{Background, Destructive, Foreground, Muted, MutedForeground, Ring, Success};
    let edge = |color: PaletteColor| Border::inside(BorderSize::Sm, color);
    let ring = |color: PaletteColor| Border::inside(BorderSize::Md, color);
    let recipe = Recipe::new()
      .base(Root, |s| s.width(260.0).gap(8.0))
      .base(Control, |s| {
        fading(
          s.width(Length::Fill)
            .height(ControlSize::Md)
            .padding_x(12.0)
            .radius(RadiusSize::Md)
            .background(Background)
            .border(edge(PaletteColor::Input))
            .foreground(Foreground)
            .typography(TypographyStyle::Body)
            .cursor(CursorIcon::Text),
        )
        .transition(PartTransition::new(Transition::border_color(), MotionDuration::Fast))
      })
      .base(Placeholder, |s| s.foreground(MutedForeground))
      .base(Icon, |s| s.square(IconSize::Md).foreground(MutedForeground))
      .hovered(Control, |s| s.border(edge(Ring)))
      .focus_visible(Control, |s| s.border(ring(Ring)))
      .size(InputSize::Sm, Control, |s| s.height(ControlSize::Sm).font_size(13.0))
      .size(InputSize::Lg, Control, |s| s.height(ControlSize::Lg).font_size(15.0))
      .state(InputState::FullWidth, Root, |s| s.width(Length::Fill))
      .state(InputState::ReadOnly, Control, |s| {
        s.background(Muted).border(edge(PaletteColor::Border))
      })
      .state(InputState::Disabled, Root, |s| s.opacity(OpacityLevel::Disabled))
      .state(InputState::Disabled, Control, |s| {
        s.background(Muted).cursor(CursorIcon::Default)
      })
      .state(InputState::Valid, Icon, |s| s.foreground(Success));
    // A judgment keeps its color under the pointer and in the focus, where the focus is thicker.
    [
      (InputState::Invalid, Destructive),
      (InputState::Warning, PaletteColor::Warning),
      (InputState::Valid, Success),
    ]
    .into_iter()
    .fold(recipe, |recipe, (state, color)| {
      recipe
        .state(state, Control, |s| s.border(edge(color.clone())))
        .when(Selector::new().state(state).hovered(), Control, |s| {
          s.border(edge(color.clone()))
        })
        .when(Selector::new().state(state).focus_visible(), Control, |s| {
          s.border(ring(color))
        })
    })
  }
}
