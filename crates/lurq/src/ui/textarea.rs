//! A textarea: text of more than one line, typed into a box.

use std::sync::Arc;

use super::{
  names,
  part_style::Length,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::fading,
  theme::{Themed, current_theme},
};
use crate::{
  app::theme::{BorderSize, OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle},
  components::TextInput,
  core::Signal,
  layout::text_style::TextStyle,
  node::{CursorIcon, Element, Outline, TextColor, border},
};

/// The rows a textarea shows while it holds less, unless it is told otherwise.
pub const TEXTAREA_MIN_ROWS: usize = 3;
/// The rows a textarea grows to before its text scrolls, unless it is told otherwise.
pub const TEXTAREA_MAX_ROWS: usize = 8;

/// The parts of a textarea a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextareaPart {
  /// The box: what is typed into, focused and disabled, with the text in it.
  Root,
  /// The text it holds: its type and its color.
  Text,
  /// What it shows while it holds nothing: its type and its color.
  Placeholder,
}

names!(TextareaPart {
  Root => "root",
  Text => "text",
  Placeholder => "placeholder",
});

/// The states of a textarea a recipe styles, besides what the pointer and the
/// keyboard do to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextareaState {
  /// What it holds is not accepted: the form says why somewhere else.
  Invalid,
  /// It takes no input.
  Disabled,
}

names!(TextareaState {
  Invalid => "invalid",
  Disabled => "disabled",
});

/// What builds a textarea's nodes: a theme's in place of the default one.
pub type TextareaRenderer = Arc<dyn Fn(TextareaView) -> Element + Send + Sync>;

/// A textarea: text of more than one line, typed into a box. A bio, a note,
/// what happened.
///
/// ```
/// use lurq::{core::Signal, ui::Textarea};
///
/// let bio = Signal::new(String::new());
/// let field = Textarea::new(bio.clone()).placeholder("Tell us what happened").rows(3, 8);
/// // Not accepted: the form says why under it.
/// let wrong = Textarea::new(bio).invalid(true);
/// ```
///
/// The textarea writes what is typed into its signal, and shows what the app
/// writes there. It is as high as its text, from the rows it shows at least
/// to the rows it shows at most ([`Textarea::rows`]); past that its text
/// scrolls inside it, with a scroll bar of its own.
pub struct Textarea {
  value: Signal<String>,
  placeholder: Option<Arc<str>>,
  label: Option<Arc<str>>,
  min_rows: usize,
  max_rows: usize,
  invalid: bool,
  disabled: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  on_change: Vec<Arc<dyn Fn(&str) + Send + Sync>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Textarea {
  /// A textarea that holds what `value` holds, and writes what is typed into it.
  pub fn new(value: Signal<String>) -> Self {
    Self {
      value,
      placeholder: None,
      label: None,
      min_rows: TEXTAREA_MIN_ROWS,
      max_rows: TEXTAREA_MAX_ROWS,
      invalid: false,
      disabled: false,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      on_change: Vec::new(),
      id: None,
      key: None,
    }
  }

  /// What it shows while it holds nothing.
  pub fn placeholder(mut self, placeholder: &str) -> Self {
    self.placeholder = Some(Arc::from(placeholder));
    self
  }

  /// What it is called, to what reads the tree. It draws nothing: the label
  /// a person reads is the field's, above it.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// The rows it shows at least, while it holds less, and at most, before its
  /// text scrolls. Three and eight unless it is told.
  pub fn rows(mut self, min: usize, max: usize) -> Self {
    self.min_rows = min.max(1);
    self.max_rows = max.max(self.min_rows);
    self
  }

  /// Whether what it holds is not accepted.
  pub fn invalid(mut self, invalid: bool) -> Self {
    self.invalid = invalid;
    self
  }

  /// Whether it takes no input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Puts the textarea in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Called on every edit, with the text the textarea holds once it applies.
  /// The signal is written either way.
  pub fn on_change(mut self, handler: impl Fn(&str) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// Styles one part of this textarea over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this textarea: what the component
  /// it is in says of it in this place.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the textarea as it is under the pointer or focused, with nothing
  /// pointing at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the box: what is typed into.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the textarea in `theme`. Converting a textarea into an element
  /// does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(TextareaState::Invalid, self.invalid)
    .state(TextareaState::Disabled, self.disabled);
    let view = TextareaView {
      value: self.value,
      placeholder: self.placeholder,
      label: self.label,
      min_rows: self.min_rows,
      max_rows: self.max_rows,
      invalid: self.invalid,
      disabled: self.disabled,
      recipe: theme.resolve::<Textarea>(&case, Some(&self.rules)),
      forced: self.forced,
      on_change: self.on_change,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Textarea>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<Textarea> for Element {
  fn from(textarea: Textarea) -> Self {
    textarea.build(&current_theme())
  }
}

/// A textarea as a renderer sees it: what it was given, the style of its
/// parts, and its behavior ready to be dressed.
pub struct TextareaView {
  pub value: Signal<String>,
  pub placeholder: Option<Arc<str>>,
  pub label: Option<Arc<str>>,
  pub min_rows: usize,
  pub max_rows: usize,
  pub invalid: bool,
  pub disabled: bool,
  /// The style of each part of this textarea.
  pub recipe: ResolvedRecipe,
  forced: Option<Interaction>,
  on_change: Vec<Arc<dyn Fn(&str) + Send + Sync>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl TextareaView {
  /// The style of a part, as this textarea is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The text style of a part: its type, and its foreground as the color of
  /// the text. A text input draws in its own style and does not take the
  /// foreground of what it is in, so the color is resolved here; a part with
  /// no foreground keeps the default color, which the theme's `Foreground` is.
  pub fn text_style(&self, part: impl Into<Name>) -> TextStyle {
    let part = self.part(part);
    let rest = part.rest();
    let mut style = rest.text.resolve(self.theme());
    match rest.foreground {
      Some(TextColor::Color(color)) => style.color = color,
      Some(TextColor::Palette(role)) => style.color = self.theme().palette().resolve(role),
      None => {}
    }
    style
  }

  /// The textarea's behavior with the style of its box and its text: the
  /// engine's multiline text input, bound to the signal, as many rows high as
  /// the view says, a stop of Tab. It is a `textbox` to what reads the tree,
  /// called by its label, and says whether it is invalid.
  pub fn input(&self) -> TextInput {
    let mut input = TextInput::styled(self.value.clone(), self.text_style(TextareaPart::Text))
      .placeholder_style(self.text_style(TextareaPart::Placeholder))
      .multiline()
      .rows(self.min_rows, self.max_rows)
      // A text input is a stop of Tab only in a form or with an index of its own; a control of the design
      // system is one wherever it is.
      .tab_index(0)
      .disabled(self.disabled)
      .part(&self.part(TextareaPart::Root))
      .describe("a11y_role", "textbox")
      .describe("a11y_multiline", "true")
      .describe("a11y_invalid", if self.invalid { "true" } else { "false" });
    if let Some(placeholder) = &self.placeholder {
      input = input.placeholder(placeholder);
    }
    if let Some(label) = &self.label {
      input = input.describe("a11y_name", label.clone());
    }
    if !self.on_change.is_empty() {
      let handlers = self.on_change.clone();
      input = input.on_input(move |event: crate::app::events::TextInputEvent| {
        for handler in &handlers {
          handler(&event.new_value);
        }
      });
    }
    if let Some(id) = &self.id {
      input = input.id(id.clone());
    }
    if let Some(key) = &self.key {
      input = input.key(key.clone());
    }
    input
  }

  /// How the default renderer builds a textarea: the box alone, which is the text input.
  pub fn render(self) -> Element {
    self.input().into()
  }
}

impl Themed for Textarea {
  const NAME: &'static str = "textarea";
  type Renderer = TextareaRenderer;

  /// The design's textarea (`pencil/lurq-design-system.pen`, Textarea, and the
  /// states the board draws for an input): a box as wide as its room, in the
  /// background's color with an edge of 1 in the input's color, the medium
  /// radius, padded 12 across and 8 down; its text in the body's type (14)
  /// with a line of 1.5,
  /// in the foreground, and its placeholder in the muted foreground. Under the
  /// pointer the edge is the ring's color; focused, a ring of 2 is drawn
  /// inside the box. Invalid, the edge and the ring are the destructive color.
  /// Disabled, it is at half its opacity on the muted fill.
  fn recipe() -> Recipe {
    use PaletteColor::*;
    use TextareaPart::{Placeholder, Root, Text};
    use TextareaState::{Disabled, Invalid};
    let edge = |color: PaletteColor| border::Border::inside(BorderSize::Sm, color);
    // The board draws the ring of the focus as a line of 2 inside the box, over its edge.
    let ring = |color: PaletteColor| Outline::new(2.0, color).offset(-2.0);
    Recipe::new()
      .base(Root, |s| {
        fading(
          s.width(Length::Fill)
            .padding_x(12.0)
            .padding_y(8.0)
            .radius(RadiusSize::Md)
            .background(Background)
            .border(edge(Input))
            .cursor(CursorIcon::Text),
        )
      })
      .base(Text, |s| {
        s.typography(TypographyStyle::Body)
          .line_height(1.5)
          .foreground(Foreground)
      })
      .base(Placeholder, |s| {
        s.typography(TypographyStyle::Body)
          .line_height(1.5)
          .foreground(MutedForeground)
      })
      .hovered(Root, |s| s.border(edge(Ring)))
      .focus_visible(Root, |s| s.outline(ring(Ring)))
      .state(Invalid, Root, |s| s.border(edge(Destructive)))
      .when(Selector::new().state(Invalid).hovered(), Root, |s| {
        s.border(edge(Destructive))
      })
      .when(Selector::new().state(Invalid).focus_visible(), Root, |s| {
        s.outline(ring(Destructive))
      })
      .state(Disabled, Root, |s| {
        s.background(Muted)
          .opacity(OpacityLevel::Disabled)
          .cursor(CursorIcon::Default)
      })
  }
}
