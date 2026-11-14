//! A token input: text that a person types, in which the app's tokens stand as chips.

use std::{ops::Range, sync::Arc};

use super::{
  field::{FieldControl, InField},
  icons, names,
  part_style::{Length, PartStyle},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  tag::TagVariant,
  text_box::{self, BoxStates, EditHandler, Judgment, TextBoxSize},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    events::TextInputEvent,
    theme::{ControlSize, IconSize, PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::{self, InputToken, InputTokenIcon, InputTokenStyle, InputTokens, Stack, TextInputController},
  core::Signal,
  layout::{StackAlignment, text_style::TextStyle},
  node::{BackgroundColor, Element, TextColor, color::Color, dimension::Dimension},
};

/// The parts of a token input a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TokenInputPart {
  /// The place the input takes: its width.
  Root,
  /// The box that is typed in and focused: its edge, its fill, its height, the room from its edge to the text,
  /// and the type of the text.
  Control,
  /// What the box says while it is empty. It has the type of the control, in its own color.
  Placeholder,
  /// A token: its fill, its edge, its corners, its height, the room before its text and after its icon, and
  /// the room between its text and its icon.
  Token,
  /// The text of a token: its type and its color.
  TokenLabel,
  /// The cross at the end of a token that takes it away: its side and its color.
  TokenIcon,
}

names!(TokenInputPart {
  Root => "root",
  Control => "control",
  Placeholder => "placeholder",
  Token => "token",
  TokenLabel => "token_label",
  TokenIcon => "token_icon",
});

/// The sizes of a token input: an input's, with tokens of 20, 24 and 28.
pub type TokenInputSize = TextBoxSize;

/// The states of a token input a recipe styles, besides what the pointer and the keyboard do to its box. The
/// two last are a token's own: its parts are resolved in them for the token the pointer is on and the tokens
/// the selection covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TokenInputState {
  /// What it holds is wrong: its edge is in the destructive color.
  Invalid,
  /// What it holds is allowed, with a caveat: its edge is in the warning color.
  Warning,
  /// What it holds was checked and is right: its edge is in the success color.
  Valid,
  /// It can be read, selected and copied, and not changed: its tokens have no cross.
  ReadOnly,
  /// It takes no input.
  Disabled,
  /// It is as wide as what holds it.
  FullWidth,
  /// Its text wraps onto as many rows as it needs, between the rows it is given.
  Multiline,
  /// The token is under the pointer.
  TokenHovered,
  /// The selection covers all of the token.
  TokenSelected,
}

names!(TokenInputState {
  Invalid => "invalid",
  Warning => "warning",
  Valid => "valid",
  ReadOnly => "read_only",
  Disabled => "disabled",
  FullWidth => "full_width",
  Multiline => "multiline",
  TokenHovered => "token_hovered",
  TokenSelected => "token_selected",
});

/// The variant of a token that a [`Mentions`](super::Mentions) chose: in the type of the text around it, in the
/// muted fill of the informative color, its text in the color.
pub const MENTION: &str = "mention";

/// What builds a token input's nodes: a theme's in place of the default one.
pub type TokenInputRenderer = Arc<dyn Fn(TokenInputView) -> Element + Send + Sync>;

/// A token in the text of a [`TokenInput`]: a range of the text that the app found, drawn as a chip.
///
/// ```
/// use lurq::ui::{TagVariant, Token};
///
/// let text = "rust wgpu ";
/// let rust = Token::new(0..4);
/// let wgpu = Token::new(5..9).variant(TagVariant::Info).removable(false);
/// assert_eq!(Token::separated(text, ' ').len(), 2);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Token {
  range: Range<usize>,
  variant: Name,
  removable: bool,
  atomic: bool,
}

impl Token {
  /// The token over `range` of the text, in UTF-8 bytes: one thing to the caret, with a cross that takes it away.
  pub fn new(range: Range<usize>) -> Self {
    Self {
      range,
      variant: TagVariant::default().into(),
      removable: true,
      atomic: true,
    }
  }

  /// A [`TagVariant`]: what the token says by its color, as a tag does. Or a variant the theme's recipe names.
  pub fn variant(mut self, variant: impl Into<Name>) -> Self {
    self.variant = variant.into();
    self
  }

  /// Whether the token has the cross at its end, which a click takes it away by.
  pub fn removable(mut self, removable: bool) -> Self {
    self.removable = removable;
    self
  }

  /// Whether the token is one thing to the caret (the default): the arrow keys go past it whole, a click
  /// selects all of it, the first Backspace after it selects it and the next deletes it. One that is not is
  /// text drawn as a token, which the caret goes into.
  pub fn atomic(mut self, atomic: bool) -> Self {
    self.atomic = atomic;
    self
  }

  /// Where the token is in the text.
  pub fn range(&self) -> Range<usize> {
    self.range.clone()
  }

  /// The tokens of a list typed with `separator` between its items: every item that has a separator after it.
  /// The item being typed, with none after it yet, is text; typing the separator makes it a token.
  pub fn separated(text: &str, separator: char) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut start = 0;
    for (at, character) in text.char_indices() {
      if character == separator {
        let item = &text[start..at];
        let lead = item.len() - item.trim_start().len();
        let trimmed = item.trim();
        if !trimmed.is_empty() {
          tokens.push(Token::new(start + lead..start + lead + trimmed.len()));
        }
        start = at + character.len_utf8();
      }
    }
    tokens
  }
}

/// How the app finds the tokens in the text.
type FindTokens = Arc<dyn Fn(&str) -> Vec<Token> + Send + Sync>;

/// A token input: a box of text in which what the app recognizes stands as a token. Tags, recipients,
/// variables in a template.
///
/// ```
/// use lurq::{core::Signal, ui::{Token, TokenInput}};
///
/// // A list of tags: each word followed by a space is a token.
/// let tags = Signal::new("rust wgpu ".to_owned());
/// let input = TokenInput::new(tags).tokens(|text| Token::separated(text, ' ')).placeholder("Add tag");
///
/// // A template whose variables are tokens, on several rows.
/// let template = Signal::new("Hello {{name}}, your order shipped.".to_owned());
/// let prompt = TokenInput::new(template)
///   .rows(2, 6)
///   .tokens(|text| text.match_indices("{{name}}").map(|(at, name)| Token::new(at..at + name.len()).removable(false)).collect());
/// ```
///
/// What the input holds is text, in the signal it is given: a token is a range of that text that the
/// function given to [`TokenInput::tokens`] finds, and it is found again in every render, so an edit that
/// changes a token's text makes it what the function then says of it. The text is what is copied, pasted,
/// undone, sent with a form and read by tooling, which sees a `textbox` whose value is the text with each
/// token as its own words.
///
/// It is the box of an [`Input`](super::Input) (one row, 36 high) or, with [`TokenInput::rows`], of a
/// [`Textarea`](super::Textarea), with its sizes and judgments. A token is drawn by the recipe's
/// [`TokenInputPart::Token`], [`TokenInputPart::TokenLabel`] and [`TokenInputPart::TokenIcon`], in the colors
/// of a [`Tag`](super::Tag) of its variant; under the pointer and covered by the selection it takes the looks of
/// [`TokenInputState::TokenHovered`] and [`TokenInputState::TokenSelected`]. A click on a token's cross takes it
/// away, as an edit through [`TokenInput::on_input`] that undo brings back.
pub struct TokenInput {
  value: Signal<String>,
  find: Option<FindTokens>,
  rows: Option<(usize, usize)>,
  placeholder: Option<Arc<str>>,
  boxed: BoxStates,
  full_width: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  on_input: Vec<EditHandler>,
  controller: Option<TextInputController>,
  name: Option<Arc<str>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl TokenInput {
  /// A token input bound to `value`: it shows what the signal holds, and what is typed is set on it.
  pub fn new(value: Signal<String>) -> Self {
    Self {
      value,
      find: None,
      rows: None,
      placeholder: None,
      boxed: BoxStates::default(),
      full_width: false,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      on_input: Vec::new(),
      controller: None,
      name: None,
      id: None,
      key: None,
    }
  }

  /// How the app finds the tokens in the text: called with the text in every render. Tokens that overlap one
  /// before them, that hold a line break or that are not on the text's characters are left out.
  pub fn tokens(mut self, find: impl Fn(&str) -> Vec<Token> + Send + Sync + 'static) -> Self {
    self.find = Some(Arc::new(find));
    self
  }

  /// Text on several rows: the input is as high as `min_rows` while it holds less, grows to `max_rows`, and
  /// then scrolls. Tokens and text share each row and wrap as words do.
  pub fn rows(mut self, min_rows: usize, max_rows: usize) -> Self {
    let min_rows = min_rows.max(1);
    self.rows = Some((min_rows, max_rows.max(min_rows)));
    self
  }

  /// What the input says while it is empty.
  pub fn placeholder(mut self, placeholder: &str) -> Self {
    self.placeholder = Some(Arc::from(placeholder));
    self
  }

  /// A [`TokenInputSize`], or a size the theme's recipe names.
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

  /// Whether the input can be read, selected and copied, and not changed. Its tokens have no cross.
  pub fn read_only(mut self, read_only: bool) -> Self {
    self.boxed.read_only = read_only;
    self
  }

  /// Whether the input takes no input. Its tokens have no cross.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.boxed.disabled = disabled;
    self
  }

  /// What the input is called, to what reads the tree. A field gives it its label unless it has its own.
  pub fn label(mut self, label: &str) -> Self {
    self.boxed.label = Some(Arc::from(label));
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
  /// ([`TextInputEvent::prevent_default`]). Taking a token away by its cross is an edit too.
  pub fn on_input(mut self, handler: impl Fn(&TextInputEvent) + Send + Sync + 'static) -> Self {
    self.on_input.push(Arc::new(handler));
    self
  }

  /// The controller the app keeps for this input, which reads its selection and puts text at it: a token
  /// chosen from a picker of the app's.
  pub fn controller(mut self, controller: TextInputController) -> Self {
    self.controller = Some(controller);
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

  /// Lays rules over the theme's recipe for this input: what the component it is in says of the input in this
  /// place.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the box as it is under the pointer or focused, with nothing pointing at it: for a gallery or a test.
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

  /// Takes what the field it is in says, as [`FieldControl::in_field`] does, and stays a builder.
  pub(crate) fn in_field(mut self, field: &InField) -> Self {
    self.boxed.in_field(field);
    self
  }

  /// The view the input is drawn from in `theme`, for [`TokenInput::build`] and for what is made of a token
  /// input (a [`Mentions`](super::Mentions)), with the tokens `more` finds in the text besides the input's own.
  pub(crate) fn view(self, theme: &Theme, more: impl FnOnce(&str) -> Vec<Token>) -> TokenInputView {
    let multiline = self.rows.is_some();
    let case = self
      .boxed
      .case(self.states.clone())
      .state(TokenInputState::FullWidth, self.full_width)
      .state(TokenInputState::Multiline, multiline);
    // The text is read here, so that what renders the input renders it again as the text changes, and finds its
    // tokens anew.
    let text = self.value.get();
    let mut tokens = self.find.as_ref().map(|find| find(&text)).unwrap_or_default();
    tokens.extend(more(&text));
    TokenInputView {
      value: self.value,
      text: Arc::from(text),
      tokens: tidy(tokens),
      rows: self.rows,
      placeholder: self.placeholder,
      label: self.boxed.label.clone(),
      invalid: self.boxed.judgment() == Some(Judgment::Invalid),
      read_only: self.boxed.read_only,
      disabled: self.boxed.disabled,
      recipe: theme.resolve::<TokenInput>(&case, Some(&self.rules)),
      case,
      rules: self.rules,
      forced: self.forced,
      boxed: self.boxed,
      on_input: self.on_input,
      controller: self.controller,
      name: self.name,
      id: self.id,
      key: self.key,
    }
  }

  /// Builds the input in `theme`. Converting a token input into an element does this in the theme of what is
  /// being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let view = self.view(theme, |_| Vec::new());
    match theme.renderer::<TokenInput>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

/// The tokens in the order of the text, without those that overlap one before them or are empty.
fn tidy(mut tokens: Vec<Token>) -> Vec<Token> {
  tokens.sort_by_key(|token| (token.range.start, token.range.end));
  let mut end = 0;
  tokens.retain(|token| {
    let keep = token.range.start < token.range.end && token.range.start >= end;
    if keep {
      end = token.range.end;
    }
    keep
  });
  tokens
}

impl FieldControl for TokenInput {
  /// Called by the field's label unless it has a name of its own, judged as the field is unless the app judged the
  /// input itself, and disabled where the field is.
  fn in_field(self, field: &InField) -> Element {
    TokenInput::in_field(self, field).into()
  }
}

impl From<TokenInput> for Element {
  fn from(input: TokenInput) -> Self {
    input.build(&current_theme())
  }
}

/// A token input as a renderer sees it: what it was given, the tokens found in its text, the style of its parts,
/// and its behavior ready to be dressed.
pub struct TokenInputView {
  /// What the input shows and what is typed is set on.
  pub value: Signal<String>,
  /// The text the tokens were found in.
  pub text: Arc<str>,
  /// The tokens found in the text, in its order, none overlapping.
  pub tokens: Vec<Token>,
  /// The rows it shows and grows to, where its text wraps.
  pub rows: Option<(usize, usize)>,
  pub placeholder: Option<Arc<str>>,
  /// What it is called to what reads the tree: its own label, or its field's.
  pub label: Option<Arc<str>>,
  /// Whether what it holds is wrong, as the input shows it.
  pub invalid: bool,
  pub read_only: bool,
  pub disabled: bool,
  /// The style of each part of this input, as its box is drawn.
  pub recipe: ResolvedRecipe,
  case: Case,
  rules: Recipe,
  forced: Option<Interaction>,
  boxed: BoxStates,
  on_input: Vec<EditHandler>,
  controller: Option<TextInputController>,
  // Read only where the engine has forms.
  #[cfg_attr(not(feature = "form"), allow(dead_code))]
  name: Option<Arc<str>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl TokenInputView {
  /// The style of a part, as this input's box is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The type of the text: the control's.
  pub fn value_style(&self) -> TextStyle {
    text_box::ink(self.theme(), &self.part(TokenInputPart::Control).rest())
  }

  /// The type of the placeholder: the control's, with what the placeholder's part lays over it.
  pub fn placeholder_style(&self) -> TextStyle {
    let control = self.part(TokenInputPart::Control).rest();
    text_box::ink(
      self.theme(),
      &control.merged(&self.part(TokenInputPart::Placeholder).rest()),
    )
  }

  /// The recipe of a token of `variant`, in the states `states` of a token.
  pub fn token_recipe(&self, variant: Name, states: &[TokenInputState]) -> ResolvedRecipe {
    let mut case = self.case.clone();
    case.variant = Some(variant);
    let case = states.iter().fold(case, |case, state| case.state(*state, true));
    self.theme().resolve::<TokenInput>(&case, Some(&self.rules))
  }

  /// How the engine draws a token in the style of `recipe`'s token parts: its fill, edge, corners, height and
  /// room from the token part, its type and color from the label's, and, where `cross`, the theme's close icon
  /// in the icon part's side and color, which takes the token away.
  pub fn token_style(&self, recipe: &ResolvedRecipe, cross: bool) -> InputTokenStyle {
    let theme = recipe.theme();
    let token = recipe.part(TokenInputPart::Token).rest();
    let text = text_box::ink(
      theme,
      &token.clone().merged(&recipe.part(TokenInputPart::TokenLabel).rest()),
    );
    let spacing = theme.spacing();
    let space = |value: &Option<crate::node::SpacingValue>| value.as_ref().map(|value| value.resolve(&spacing, 0.0));
    let fill = match &token.background {
      Some(BackgroundColor::Color(color)) => *color,
      Some(BackgroundColor::Palette(role)) => theme.palette_color(role.clone()),
      Some(BackgroundColor::CurrentForeground) => text.color,
      None => Color::new(0, 0, 0, 0),
    };
    let start = space(&token.padding_start).or(space(&token.padding_x)).unwrap_or(0.0);
    let end = space(&token.padding_end).or(space(&token.padding_x)).unwrap_or(0.0);
    let radius = token
      .radius
      .as_ref()
      .map(|radius| radius.resolve(&theme.radii()).top_left)
      .unwrap_or(0.0);
    let mut style = InputTokenStyle::new(text.color, fill)
      .padding(start)
      .padding_end(end)
      .radius(radius)
      .text_style(text.clone());
    if let Some(Dimension::Px(height)) = token.height.and_then(|height| height.resolve(theme)) {
      style = style.height(height);
    }
    if let Some(edge) = token
      .border
      .as_ref()
      .and_then(|border| border.resolve(&theme.palette(), &theme.border_sizes()))
      .filter(|edge| edge.width > 0.0)
    {
      style = style.border(edge.width, edge.color);
    }
    if cross && let Some(glyph) = theme.icon(icons::CLOSE) {
      let icon = recipe.part(TokenInputPart::TokenIcon).rest();
      let side = match icon.width.unwrap_or(Length::Icon(IconSize::Xs)).resolve(theme) {
        Some(Dimension::Px(side)) => side,
        _ => theme.icon_size_value(IconSize::Xs),
      };
      let color = match &icon.foreground {
        Some(TextColor::Color(color)) => *color,
        Some(TextColor::Palette(role)) => theme.palette_color(role.clone()),
        None => text.color,
      };
      let gap = space(&token.gap).unwrap_or(0.0);
      style = style.icon(
        InputTokenIcon::new(glyph)
          .size(side)
          .color(color)
          .gap(gap)
          .removes(true),
      );
    }
    style
  }

  /// The engine's tokens for the text: each token in the style of its variant, with the looks it takes under the
  /// pointer and covered by the selection. A read-only or disabled input's tokens have no cross.
  pub fn input_tokens(&self) -> InputTokens {
    let editable = !self.read_only && !self.disabled;
    let mut looks: Vec<(Name, bool, InputTokenStyle)> = Vec::new();
    let tokens = self
      .tokens
      .iter()
      .filter(|token| {
        token.range.end <= self.text.len()
          && self.text.is_char_boundary(token.range.start)
          && self.text.is_char_boundary(token.range.end)
          && !self.text[token.range.clone()].contains(['\r', '\n'])
      })
      .map(|token| {
        let cross = token.removable && editable;
        let style = match looks
          .iter()
          .find(|(variant, with_cross, _)| *variant == token.variant && *with_cross == cross)
        {
          Some((_, _, style)) => style.clone(),
          None => {
            let rest = self.token_style(&self.token_recipe(token.variant, &[]), cross);
            let hovered = self.token_style(
              &self.token_recipe(token.variant, &[TokenInputState::TokenHovered]),
              cross,
            );
            let selected = self.token_style(
              &self.token_recipe(token.variant, &[TokenInputState::TokenSelected]),
              cross,
            );
            let style = rest.hovered(hovered).selected(selected);
            looks.push((token.variant, cross, style.clone()));
            style
          }
        };
        InputToken::new(token.range.clone(), style).atomic(token.atomic)
      })
      .collect();
    // The ranges were checked against the text above: what is left is valid.
    InputTokens::new(&self.text, tokens)
      .unwrap_or_else(|_| InputTokens::new(&self.text, Vec::new()).expect("no tokens are always valid"))
  }

  /// The input's behavior with the style of its box and its tokens: the engine's text input bound to the value,
  /// on one line or on its rows, with the tokens found in the text. It is a stop of Tab, a `textbox` to what reads
  /// the tree, called by its label, and says whether it is invalid; a read-only one refuses every edit.
  pub fn control(&self) -> components::TextInput {
    let mut control = components::TextInput::new(self.value.clone())
      .text_style(self.value_style())
      .placeholder_style(self.placeholder_style())
      .tokens(self.input_tokens());
    control = match self.rows {
      Some((min_rows, max_rows)) => control
        .multiline()
        .rows(min_rows, max_rows)
        .describe("a11y_multiline", "true"),
      None => control.single_line(),
    };
    let control = control.part(&self.part(TokenInputPart::Control));
    let mut control = text_box::text_box(control, &self.boxed, self.on_input.clone());
    if let Some(controller) = &self.controller {
      control = control.controller(controller.clone());
    }
    if let Some(placeholder) = &self.placeholder {
      control = control.placeholder(placeholder);
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

  /// The place the input takes, around `control`.
  pub fn root(&self, control: impl Into<Element>) -> Stack {
    Stack::new()
      .stack_align(StackAlignment::TopStart)
      .part(&self.part(TokenInputPart::Root))
      .child(control)
  }

  /// How the default renderer builds a token input: the control in its place.
  pub fn render(self) -> Element {
    self.root(self.control()).into()
  }
}

impl Themed for TokenInput {
  const NAME: &'static str = "token_input";
  type Renderer = TokenInputRenderer;

  /// The design's token input (`pencil/lurq-design-system.pen`, Token Input): the box of an input, 260 wide and
  /// 36 high, padded 6 at its sides, its text in the body's type and its placeholder in the muted foreground; its
  /// edge says its state as an input's does. A token is 24 high in the fill of what is secondary, with the small
  /// radius, 8 before its text and 6 after its cross, its text 13 in the secondary foreground and its cross of 12
  /// in the muted foreground, 4 after the text. Informative, success, warning and destructive tokens are in the
  /// muted fill of their color with their text in the color, as tags are. Under the pointer a token is in the
  /// secondary color's hover (a colored one keeps its fill) and its cross in the foreground; covered by the
  /// selection it is in the primary color with its text and cross in the primary foreground. Small, the box is 32
  /// and a token 20 with text of 12; large, 40 and 28 with text of 14. On rows, the box is padded 6 down too, and
  /// its lines are 28 high (twice the text), a token's text with them. A mention ([`MENTION`]) stands in a line of
  /// prose: in the informative color's muted fill and its text in the color, in the text's type with lines of 1.5,
  /// 20 high and padded 4.
  fn recipe() -> Recipe {
    use PaletteColor::*;
    use TokenInputPart::{Control, Placeholder, Root, Token, TokenIcon, TokenLabel};
    let recipe = Recipe::new()
      .base(Root, |s| s.width(260.0))
      .base(Control, |s| {
        text_box::box_style(s)
          .padding_x(6.0)
          .height(ControlSize::Md)
          .foreground(Foreground)
          .typography(TypographyStyle::Body)
      })
      .base(Token, |s| {
        s.height(24.0)
          .padding_start(8.0)
          .padding_end(6.0)
          .gap(4.0)
          .radius(RadiusSize::Sm)
          .background(Secondary)
      })
      .base(TokenLabel, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(13.0)
          .foreground(SecondaryForeground)
      })
      .base(TokenIcon, |s| s.square(12.0).foreground(MutedForeground))
      .state(TokenInputState::FullWidth, Root, |s| s.width(Length::Fill))
      .size(TextBoxSize::Sm, Token, |s| {
        s.height(20.0).padding_start(6.0).padding_end(4.0)
      })
      .size(TextBoxSize::Sm, TokenLabel, |s| s.font_size(12.0))
      .size(TextBoxSize::Sm, TokenIcon, |s| s.square(10.0))
      .size(TextBoxSize::Lg, Token, |s| {
        s.height(28.0).padding_start(10.0).padding_end(8.0)
      })
      .size(TextBoxSize::Lg, TokenLabel, |s| s.font_size(14.0))
      .size(TextBoxSize::Lg, TokenIcon, |s| s.square(IconSize::Sm));
    let recipe = text_box::box_sizes(recipe, Control, true);
    // On rows the box is as high as its rows, and each row holds a token with room above and below it.
    let recipe = [
      (None, 2.0, 26.0 / 13.0),
      (Some(TextBoxSize::Sm), 2.0, 24.0 / 12.0),
      (Some(TextBoxSize::Lg), 2.0, 32.0 / 14.0),
    ]
    .into_iter()
    .fold(recipe, |recipe, (size, line, token_line)| {
      let selector = match size {
        Some(size) => Selector::new().size(size).state(TokenInputState::Multiline),
        None => Selector::new().state(TokenInputState::Multiline),
      };
      recipe
        .when(selector.clone(), Control, |s| {
          s.height(Length::Auto).padding_y(6.0).line_height(line)
        })
        .when(selector, TokenLabel, |s| s.line_height(token_line))
    });
    let recipe = recipe
      .state(TokenInputState::TokenHovered, Token, |s| s.background(SecondaryHover))
      .state(TokenInputState::TokenHovered, TokenIcon, |s| s.foreground(Foreground))
      .state(TokenInputState::TokenSelected, Token, |s| s.background(Primary))
      .state(TokenInputState::TokenSelected, TokenLabel, |s| {
        s.foreground(PrimaryForeground)
      })
      .state(TokenInputState::TokenSelected, TokenIcon, |s| {
        s.foreground(PrimaryForeground)
      });
    // A token of a color keeps its fill and ink under the pointer; in a selection it is in the primary color, as
    // any token is.
    let colors = [
      (Name::from(TagVariant::Info), InfoMuted, Info),
      (Name::from(TagVariant::Success), SuccessMuted, Success),
      (Name::from(TagVariant::Warning), WarningMuted, Warning),
      (Name::from(TagVariant::Destructive), DestructiveMuted, Destructive),
      (Name::new(MENTION), InfoMuted, Info),
    ];
    let recipe = colors.into_iter().fold(recipe, |recipe, (variant, fill, ink)| {
      let hovered = Selector::new().variant(variant).state(TokenInputState::TokenHovered);
      recipe
        .variant(variant, Token, |s| s.background(fill.clone()))
        .variant(variant, TokenLabel, |s| s.foreground(ink.clone()))
        .variant(variant, TokenIcon, |s| s.foreground(ink.clone()))
        .when(hovered.clone(), Token, |s| s.background(fill))
        .when(hovered, TokenIcon, |s| s.foreground(ink))
    });
    // A mention stands in a line of prose: in the text's type, its fill 20 high and padded 4, with no room
    // around it that the line would not have.
    let mention = Selector::new().variant(MENTION);
    let recipe = recipe
      .when(mention.clone(), Token, |s| {
        s.height(20.0).padding_start(4.0).padding_end(4.0)
      })
      .when(mention.clone(), TokenLabel, |s| s.font_size(14.0))
      .when(mention.clone().size(TextBoxSize::Sm), TokenLabel, |s| s.font_size(13.0))
      .when(mention.clone().size(TextBoxSize::Lg), TokenLabel, |s| s.font_size(15.0))
      .when(mention.state(TokenInputState::Multiline), TokenLabel, |s| {
        s.line_height(1.5)
      });
    text_box::box_rules(recipe, Root, Control, Placeholder)
  }
}
