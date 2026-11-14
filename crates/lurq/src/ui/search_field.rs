//! A search field: an input with the search icon before its text, a button that empties it, and the hint of the key
//! that reaches it.

use std::sync::Arc;

use super::{
  field::{FieldControl, InField},
  icon::Icon,
  icons,
  input::{Input, InputPart, InputView},
  names,
  part_style::{FontFamily, Length, PartStyle},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::fading,
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    events::{KeyboardEvent, TextInputEvent},
    theme::{BorderSize, IconSize, PaletteColor, RadiusSize, Theme},
  },
  components::{self, Row},
  core::{ElementRef, Signal},
  layout::Alignment,
  node::{CursorIcon, Element, border::Border, dimension::Dimension},
};

/// The parts of a search field a recipe styles. The box, its text, its placeholder and its icon are an
/// [`Input`]'s: what the field says of them is laid over the theme's recipe for an input.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SearchFieldPart {
  /// What the field says of the input's root: its width.
  Root,
  /// What the field says of the input's box: its padding.
  Control,
  /// What is at the end of the box: the clear button or the hint.
  End,
  /// The button that empties the field, while it holds something.
  Clear,
  /// The key that reaches the field, while it is empty.
  Shortcut,
}

names!(SearchFieldPart {
  Root => "root",
  Control => "control",
  End => "end",
  Clear => "clear",
  Shortcut => "shortcut",
});

/// The states of a search field a recipe styles, besides an input's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SearchFieldState {
  /// It holds something: the clear button is shown.
  Filled,
  /// It is as wide as what holds it.
  FullWidth,
}

names!(SearchFieldState {
  Filled => "filled",
  FullWidth => "full_width",
});

/// What builds a search field's nodes: a theme's in place of the default one.
pub type SearchFieldRenderer = Arc<dyn Fn(SearchFieldView) -> Element + Send + Sync>;

type SubmitHandler = Arc<dyn Fn(&str) + Send + Sync>;

/// A search field: an input with the search icon before its text, a button
/// at its end that empties it while it holds something, and the hint of the
/// key that reaches it while it is empty.
///
/// ```
/// use lurq::{core::Signal, ui::SearchField};
///
/// let query = Signal::new(String::new());
/// let search = SearchField::new(query.clone())
///   .placeholder("Search components")
///   .shortcut("/")
///   .on_submit(|query| println!("search for {query}"));
/// ```
///
/// What is typed goes into the signal, which the app filters by as it
/// changes. Escape empties the field while it holds something, and Enter
/// calls [`on_submit`](Self::on_submit) with what it holds. The clear button
/// is pressed with the pointer and puts the caret back in the field; from
/// the keyboard, Escape does what it does, so it is no stop of Tab. The hint
/// says the key: the app binds it, and focuses the field on it. To tooling
/// the field is a `searchbox`, and its button "Clear search".
pub struct SearchField {
  input: Input,
  value: Signal<String>,
  shortcut: Option<Arc<str>>,
  clear_label: Arc<str>,
  on_submit: Vec<SubmitHandler>,
  read_only: bool,
  disabled: bool,
  full_width: bool,
  states: Vec<Name>,
  rules: Recipe,
  input_rules: Recipe,
  forced: Option<Interaction>,
}

impl SearchField {
  /// A search field bound to `value`: it shows what the signal holds, and what is typed is set on it.
  pub fn new(value: Signal<String>) -> Self {
    let mut input = Input::new(value.clone()).icon(icons::SEARCH);
    input.boxed.role = Some("searchbox");
    Self {
      input,
      value,
      shortcut: None,
      clear_label: Arc::from("Clear search"),
      on_submit: Vec::new(),
      read_only: false,
      disabled: false,
      full_width: false,
      states: Vec::new(),
      rules: Recipe::new(),
      input_rules: Recipe::new(),
      forced: None,
    }
  }

  /// What the field says while it is empty.
  pub fn placeholder(mut self, placeholder: &str) -> Self {
    self.input = self.input.placeholder(placeholder);
    self
  }

  /// The key that reaches the field, shown at its end while it is empty: `/`, `Ctrl K`. The app binds the key.
  pub fn shortcut(mut self, key: &str) -> Self {
    self.shortcut = Some(Arc::from(key));
    self
  }

  /// What the clear button is called to tooling, in place of "Clear search": in another language.
  pub fn clear_label(mut self, label: &str) -> Self {
    self.clear_label = Arc::from(label);
    self
  }

  /// Called on Enter with what the field holds. The focus stays in the field.
  pub fn on_submit(mut self, handler: impl Fn(&str) + Send + Sync + 'static) -> Self {
    self.on_submit.push(Arc::new(handler));
    self
  }

  /// Called before an edit is made, with what the field will hold: a handler may refuse it.
  pub fn on_input(mut self, handler: impl Fn(&TextInputEvent) + Send + Sync + 'static) -> Self {
    self.input = self.input.on_input(handler);
    self
  }

  /// An [`InputSize`](super::InputSize), or a size the theme's recipe names.
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.input = self.input.size(size);
    self
  }

  /// Whether what the field holds is wrong.
  pub fn invalid(mut self, invalid: bool) -> Self {
    self.input = self.input.invalid(invalid);
    self
  }

  /// Whether what the field holds is allowed with a caveat.
  pub fn warning(mut self, warning: bool) -> Self {
    self.input = self.input.warning(warning);
    self
  }

  /// Whether what the field holds was checked and is right.
  pub fn valid(mut self, valid: bool) -> Self {
    self.input = self.input.valid(valid);
    self
  }

  /// Whether the field can be read and not changed: it has no clear button, and Escape does not empty it.
  pub fn read_only(mut self, read_only: bool) -> Self {
    self.read_only = read_only;
    self.input = self.input.read_only(read_only);
    self
  }

  /// Whether the field takes no input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self.input = self.input.disabled(disabled);
    self
  }

  /// What the field is called, to what reads the tree. A field gives it its label.
  pub fn label(mut self, label: &str) -> Self {
    self.input = self.input.label(label);
    self
  }

  /// Whether the field is as wide as what holds it, in place of the design's 280.
  pub fn full_width(mut self, full_width: bool) -> Self {
    self.full_width = full_width;
    self.input = self.input.full_width(full_width);
    self
  }

  /// The name the query has in the form it is in.
  #[cfg(feature = "form")]
  pub fn name(mut self, name: impl Into<Arc<str>>) -> Self {
    self.input = self.input.name(name);
    self
  }

  /// Puts the search field in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this search field over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this search field.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Lays rules over the theme's recipe for the input it is made of, and over what the field says of it.
  pub fn input_rules(mut self, rules: &Recipe) -> Self {
    self.input_rules = self.input_rules.extend(rules);
    self
  }

  /// Draws the field as it is under the pointer or focused, with nothing pointing at it.
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

  /// Builds the search field in `theme`. Converting it into an element does this in the theme of what is being
  /// rendered, which renders again as the field fills and empties.
  pub fn build(self, theme: &Theme) -> Element {
    let filled = !self.value.get().is_empty();
    let case: Case = self
      .input
      .boxed
      .case(self.states.clone())
      .state(SearchFieldState::Filled, filled)
      .state(SearchFieldState::FullWidth, self.full_width);
    let recipe = theme.resolve::<SearchField>(&case, Some(&self.rules));
    let of_input = recipe
      .rules_for(SearchFieldPart::Root, InputPart::Root)
      .extend(&recipe.rules_for(SearchFieldPart::Control, InputPart::Control));
    let input = self.input.rules(&of_input).rules(&self.input_rules);
    let view = SearchFieldView {
      filled,
      shortcut: self.shortcut,
      clear_label: self.clear_label,
      editable: !self.read_only && !self.disabled,
      recipe,
      forced: self.forced,
      on_submit: self.on_submit,
      focus: ElementRef::new(),
      input: input.view(theme),
    };
    match theme.renderer::<SearchField>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl FieldControl for SearchField {
  /// Called by the field's label unless it has a name of its own, judged as the field is unless the app judged the
  /// search field itself, and disabled where the field is.
  fn in_field(mut self, field: &InField) -> Element {
    self.input.boxed.in_field(field);
    self.disabled = self.input.boxed.disabled;
    self.into()
  }
}

impl From<SearchField> for Element {
  fn from(field: SearchField) -> Self {
    field.build(&current_theme())
  }
}

/// A search field as a renderer sees it: the input it is made of, the style of its own parts, and its behavior.
pub struct SearchFieldView {
  /// The input the query is typed in: its box, its text, its placeholder, its icon.
  pub input: InputView,
  /// Whether the field holds something: its clear button is shown in place of the hint.
  pub filled: bool,
  pub shortcut: Option<Arc<str>>,
  pub clear_label: Arc<str>,
  /// Whether what the field holds can be changed: it is neither read only nor disabled.
  pub editable: bool,
  /// The style of each of the search field's own parts.
  pub recipe: ResolvedRecipe,
  forced: Option<Interaction>,
  on_submit: Vec<SubmitHandler>,
  /// What a press of the clear button focuses: the control.
  focus: ElementRef,
}

impl SearchFieldView {
  /// The style of a part, as this search field is to be drawn.
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

  /// Whether the clear button is shown: the field holds something that can be taken away.
  fn clears(&self) -> bool {
    self.filled && self.editable
  }

  /// How wide the hint is: its padding and a cell of the mono type for each of its characters. It is laid over
  /// the end of the box before it is laid out, so the box keeps room for it by this.
  fn shortcut_width(&self, key: &str) -> f32 {
    // A cell of a mono type is about 0.6 of its size.
    const CELL: f32 = 0.6;
    let style = self.part(SearchFieldPart::Shortcut).rest();
    let padding = style
      .padding_x
      .map(|padding| padding.resolve(&self.theme().spacing(), 0.0))
      .unwrap_or(0.0);
    let size = style.text.resolve(self.theme()).font_size;
    2.0 * padding + key.chars().count() as f32 * size * CELL
  }

  /// The input's text input: a stop of Tab, a `searchbox` to tooling, where Escape empties what it holds and Enter
  /// submits it, keeping room at its end for the clear button or the hint.
  pub fn control(&self) -> components::TextInput {
    let mut control = self.input.control().ref_element(self.focus.clone());
    let end = if self.clears() {
      Some(self.px(self.part(SearchFieldPart::Clear).rest().width))
    } else if let Some(key) = self.shortcut.as_deref().filter(|_| !self.filled) {
      Some(self.shortcut_width(key))
    } else {
      None
    };
    if let Some(end) = end {
      control = control.padding_right(self.input.end_room(end));
    }
    let (value, submit, editable) = (self.input.value.clone(), self.on_submit.clone(), self.editable);
    // A key reaches every node: these act only while the focus is in the field, and leave a composition's keys to it.
    control.on_key_down(move |event: KeyboardEvent| {
      if !event.focus_within || event.composing || event.ctrl || event.alt || event.meta {
        return;
      }
      match event.key.as_str() {
        "Escape" if editable && !value.get_untracked().is_empty() => {
          event.prevent_default();
          value.set(String::new());
        }
        "Enter" if !submit.is_empty() => {
          event.prevent_default();
          let query = value.get_untracked();
          for handler in &submit {
            handler(&query);
          }
        }
        _ => {}
      }
    })
  }

  /// The button that empties the field and puts the caret back in it, while it holds something.
  pub fn clear(&self) -> Option<Row> {
    if !self.clears() {
      return None;
    }
    let part = self.part(SearchFieldPart::Clear);
    let side = part.rest().width.unwrap_or(Length::Icon(IconSize::Md));
    let value = self.input.value.clone();
    Some(
      Row::new()
        .part(&part)
        .describe("a11y_role", "button")
        .describe("a11y_name", self.clear_label.clone())
        .focuses(&self.focus)
        .on_click(move |_| value.set(String::new()))
        .child(Icon::new(icons::CLOSE_CIRCLE).size(side).build(self.theme())),
    )
  }

  /// The hint of the key that reaches the field, while it is empty. The pointer passes through it.
  pub fn shortcut(&self) -> Option<Row> {
    let key = self.shortcut.as_ref().filter(|_| !self.filled)?;
    let part = self.part(SearchFieldPart::Shortcut);
    Some(
      Row::new()
        .align_items(Alignment::Center)
        .pointer_events_none()
        .part(&part)
        .child(part.text(key).nowrap()),
    )
  }

  /// How the default renderer builds a search field: the input, with the clear button or the hint over the end
  /// of its box.
  pub fn render(self) -> Element {
    let end = match (self.clear(), self.shortcut()) {
      (Some(clear), _) => Some(clear),
      (None, Some(shortcut)) => Some(shortcut),
      (None, None) => None,
    }
    .map(|item| Row::new().part(&self.part(SearchFieldPart::End)).child(item));
    self.input.render_with_end(self.control(), end).into()
  }
}

impl Themed for SearchField {
  const NAME: &'static str = "search_field";
  type Renderer = SearchFieldRenderer;

  /// The design's search field (`pencil/lurq-design-system.pen`, Search Field): an input 280 wide whose box is
  /// padded 10 at its start and 8 at its end, the search icon of 16 in the muted foreground 8 before the text.
  /// While it holds something a clear button of 16 in the muted foreground is at its end, in the foreground under
  /// the pointer; while it is empty, the hint of its key: 20 high, padded 6, 4 round on the muted fill in an edge
  /// of 1 in the border's color, in the mono type of 11 in the muted foreground.
  fn recipe() -> Recipe {
    use PaletteColor::{Foreground, Muted, MutedForeground};
    use SearchFieldPart::{Clear, Control, End, Root, Shortcut};
    Recipe::new()
      .base(Root, |s| s.width(280.0))
      .state(SearchFieldState::FullWidth, Root, |s| s.width(Length::Fill))
      .base(Control, |s| s.padding_start(10.0).padding_end(8.0))
      .base(End, |s| s.gap(8.0))
      .base(Clear, |s| {
        fading(
          s.square(IconSize::Md)
            .radius(RadiusSize::Sm)
            .foreground(MutedForeground)
            .cursor(CursorIcon::Pointer),
        )
      })
      .hovered(Clear, |s| s.foreground(Foreground))
      .base(Shortcut, |s| {
        s.height(20.0)
          .padding_x(6.0)
          .radius(RadiusSize::Sm)
          .background(Muted)
          .border(Border::inside(BorderSize::Sm, PaletteColor::Border))
          .font_family(FontFamily::Mono)
          .font_size(11.0)
          .foreground(MutedForeground)
      })
  }
}
