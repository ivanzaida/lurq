//! A textarea: text of more than one line, typed into a box.

use std::sync::Arc;

use super::{
  field::{FieldControl, InField},
  names,
  recipe::{Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  text_box::{self, BoxStates, EditHandler, Judgment, TextBoxSize},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    events::TextInputEvent,
    theme::{PaletteColor, Theme, TypographyStyle},
  },
  components::TextInput,
  core::Signal,
  layout::text_style::TextStyle,
  node::Element,
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

/// The sizes of a textarea: the type of an input's sizes, 13, 14 and 15. It is as high as its rows of that type.
pub type TextareaSize = TextBoxSize;

/// The states of a textarea a recipe styles, besides what the pointer and the
/// keyboard do to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextareaState {
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
}

names!(TextareaState {
  Invalid => "invalid",
  Warning => "warning",
  Valid => "valid",
  ReadOnly => "read_only",
  Disabled => "disabled",
});

/// What builds a textarea's nodes: a theme's in place of the default one.
pub type TextareaRenderer = Arc<dyn Fn(TextareaView) -> Element + Send + Sync>;

/// A textarea: text of more than one line, typed into a box. A bio, a note,
/// what happened.
///
/// ```
/// use lurq::{core::Signal, ui::{Textarea, TextareaSize}};
///
/// let bio = Signal::new(String::new());
/// let field = Textarea::new(bio.clone()).placeholder("Tell us what happened").rows(3, 8);
/// // Not accepted: the form says why under it.
/// let wrong = Textarea::new(bio.clone()).invalid(true);
/// let terms = Textarea::new(Signal::new("By using lurq you agree...".to_owned())).read_only(true);
/// let compact = Textarea::new(bio).size(TextareaSize::Sm);
/// ```
///
/// The textarea writes what is typed into its signal, and shows what the app
/// writes there. It is as high as its text, from the rows it shows at least
/// to the rows it shows at most ([`Textarea::rows`]); past that its text
/// scrolls inside it, with a scroll bar of its own. Its edge says its state,
/// as an input's does.
pub struct Textarea {
  value: Signal<String>,
  placeholder: Option<Arc<str>>,
  min_rows: usize,
  max_rows: usize,
  boxed: BoxStates,
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
      min_rows: TEXTAREA_MIN_ROWS,
      max_rows: TEXTAREA_MAX_ROWS,
      boxed: BoxStates::default(),
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
  /// a person reads is the field's, above it, which gives the textarea this
  /// name unless it has its own.
  pub fn label(mut self, label: &str) -> Self {
    self.boxed.label = Some(Arc::from(label));
    self
  }

  /// The rows it shows at least, while it holds less, and at most, before its
  /// text scrolls. Three and eight unless it is told.
  pub fn rows(mut self, min: usize, max: usize) -> Self {
    self.min_rows = min.max(1);
    self.max_rows = max.max(self.min_rows);
    self
  }

  /// A [`TextareaSize`], or a size the theme's recipe names: the type of its text, and so the height of its rows.
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.boxed.size = size.into();
    self
  }

  /// Whether what it holds is wrong. Of invalid, warning and valid, the first that is set is the one shown.
  pub fn invalid(mut self, invalid: bool) -> Self {
    self.boxed.invalid = invalid;
    self
  }

  /// Whether what it holds is allowed with a caveat.
  pub fn warning(mut self, warning: bool) -> Self {
    self.boxed.warning = warning;
    self
  }

  /// Whether what it holds was checked and is right.
  pub fn valid(mut self, valid: bool) -> Self {
    self.boxed.valid = valid;
    self
  }

  /// Whether it can be read, selected and copied, and not changed. It is still a stop of Tab.
  pub fn read_only(mut self, read_only: bool) -> Self {
    self.boxed.read_only = read_only;
    self
  }

  /// Whether it takes no input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.boxed.disabled = disabled;
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
  /// The signal is written either way. A read-only textarea makes no edit.
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
    let case = self.boxed.case(self.states.clone());
    let view = TextareaView {
      value: self.value,
      placeholder: self.placeholder,
      label: self.boxed.label.clone(),
      min_rows: self.min_rows,
      max_rows: self.max_rows,
      invalid: self.boxed.judgment() == Some(Judgment::Invalid),
      read_only: self.boxed.read_only,
      disabled: self.boxed.disabled,
      recipe: theme.resolve::<Textarea>(&case, Some(&self.rules)),
      forced: self.forced,
      boxed: self.boxed,
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

impl FieldControl for Textarea {
  /// Called by the field's label unless it has a name of its own, judged as the field is unless the app judged the
  /// textarea itself, and disabled where the field is.
  fn in_field(mut self, field: &InField) -> Element {
    self.boxed.in_field(field);
    self.into()
  }
}

/// A textarea as a renderer sees it: what it was given, the style of its
/// parts, and its behavior ready to be dressed.
pub struct TextareaView {
  pub value: Signal<String>,
  pub placeholder: Option<Arc<str>>,
  /// What it is called to what reads the tree: its own label, or its field's.
  pub label: Option<Arc<str>>,
  pub min_rows: usize,
  pub max_rows: usize,
  /// Whether what it holds is wrong, as the textarea shows it.
  pub invalid: bool,
  pub read_only: bool,
  pub disabled: bool,
  /// The style of each part of this textarea.
  pub recipe: ResolvedRecipe,
  forced: Option<Interaction>,
  boxed: BoxStates,
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
    text_box::ink(self.theme(), &self.part(part).rest())
  }

  /// The type of the placeholder: the text's, with what the placeholder's part lays over it.
  pub fn placeholder_style(&self) -> TextStyle {
    let text = self.part(TextareaPart::Text).rest();
    text_box::ink(self.theme(), &text.merged(&self.part(TextareaPart::Placeholder).rest()))
  }

  /// The textarea's behavior with the style of its box and its text: the
  /// engine's multiline text input, bound to the signal, as many rows high as
  /// the view says, a stop of Tab. It is a `textbox` to what reads the tree,
  /// called by its label, and says whether it is invalid. A read-only one
  /// refuses every edit.
  pub fn input(&self) -> TextInput {
    let input = TextInput::styled(self.value.clone(), self.text_style(TextareaPart::Text))
      .placeholder_style(self.placeholder_style())
      .multiline()
      .rows(self.min_rows, self.max_rows)
      .part(&self.part(TextareaPart::Root))
      .describe("a11y_multiline", "true");
    let handlers: Vec<EditHandler> = self
      .on_change
      .iter()
      .map(|handler| {
        let handler = handler.clone();
        Arc::new(move |event: &TextInputEvent| handler(&event.new_value)) as EditHandler
      })
      .collect();
    let mut input = text_box::text_box(input, &self.boxed, handlers);
    if let Some(placeholder) = &self.placeholder {
      input = input.placeholder(placeholder);
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
  /// with a line of 1.5, in the foreground, and its placeholder in the muted
  /// foreground. Under the pointer the edge is the ring's color; focused, a
  /// ring of 2 is drawn inside the box. Invalid, warning and valid, the edge
  /// and the ring are the destructive, warning and success colors. Read only,
  /// it is on the muted fill in the color of a border. Disabled, it is at half
  /// its opacity on the muted fill. Small, its text is 13; large, 15.
  fn recipe() -> Recipe {
    use TextareaPart::{Placeholder, Root, Text};
    let recipe = Recipe::new()
      .base(Root, |s| text_box::box_style(s).padding_y(8.0))
      .base(Text, |s| {
        s.typography(TypographyStyle::Body)
          .line_height(1.5)
          .foreground(PaletteColor::Foreground)
      });
    let recipe = text_box::box_sizes(recipe, Text, false);
    text_box::box_rules(recipe, Root, Root, Placeholder)
  }
}
