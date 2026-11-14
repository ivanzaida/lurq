//! Mentions: a token input that offers people or things as a trigger is typed, and holds what is chosen as tokens.

use std::sync::Arc;

use parking_lot::Mutex;

use super::{
  avatar::{Avatar, AvatarSize},
  field::{FieldControl, InField},
  icon::Icon,
  names,
  part_style::{Length, PartStyle},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  theme::{Themed, current_theme},
  token_input::{MENTION, Token, TokenInput, TokenInputPart, TokenInputView},
};
use crate::{
  app::{
    ctx::Placement,
    events::{KeyboardEvent, TextInputEvent},
    theme::{BorderSize, IconSize, PaletteColor, RadiusSize, ShadowStyle, Theme, TypographyStyle},
  },
  components::{Column, Popup, Rect, Row, TextInputController},
  core::{ElementRef, Signal},
  layout::Alignment,
  node::{CursorIcon, Element, border::Border},
};

/// The parts of a mentions field a recipe styles, besides those of the token input it is
/// ([`TokenInputPart`], whose tokens of the variant [`MENTION`] are its mentions).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MentionsPart {
  /// The place the field takes: its width.
  Root,
  /// The box that is typed in: what it lays over the token input's control, its room and its least height.
  Control,
  /// The list of what is offered, under the trigger: its panel.
  List,
  /// One offer in the list: what is pressed, and what is highlighted while it is the one Enter chooses.
  Option,
  /// The avatar of an offer, where it has one.
  OptionAvatar,
  /// The icon of an offer, where it has one and no avatar.
  OptionIcon,
  /// The text of an offer.
  OptionLabel,
}

names!(MentionsPart {
  Root => "root",
  Control => "control",
  List => "list",
  Option => "option",
  OptionAvatar => "option_avatar",
  OptionIcon => "option_icon",
  OptionLabel => "option_label",
});

/// The states of a mentions field a recipe styles, besides those of its box.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MentionsState {
  /// The list is open under a trigger.
  Open,
  /// The offer is the one Enter chooses: its own case.
  Active,
}

names!(MentionsState {
  Open => "open",
  Active => "active",
});

/// What builds a mentions field's nodes: a theme's in place of the default one.
pub type MentionsRenderer = Arc<dyn Fn(MentionsView) -> Element + Send + Sync>;

/// A person or a thing a [`Mentions`] offers after its trigger.
///
/// ```
/// use lurq::ui::{Avatar, Mention, icons};
///
/// let ivan = Mention::new("Ivan Zaida").avatar(Avatar::new("Ivan Zaida"));
/// // The text it puts in the field, in place of the trigger and its label.
/// let channel = Mention::new("design").text("#design").icon(icons::USERS);
/// ```
pub struct Mention {
  label: Arc<str>,
  text: Option<Arc<str>>,
  avatar: Option<Avatar>,
  icon: Option<Name>,
}

impl Mention {
  /// An offer called `label`, which puts the trigger and the label in the field.
  pub fn new(label: &str) -> Self {
    Self {
      label: Arc::from(label),
      text: None,
      avatar: None,
      icon: None,
    }
  }

  /// The text the offer puts in the field, the trigger included: the app's text for it, which the field's value
  /// holds and tooling reads.
  pub fn text(mut self, text: &str) -> Self {
    self.text = Some(Arc::from(text));
    self
  }

  /// An avatar before the label: a person's.
  pub fn avatar(mut self, avatar: Avatar) -> Self {
    self.avatar = Some(avatar);
    self
  }

  /// An icon before the label, by its name in the theme's set, where the offer has no avatar.
  pub fn icon(mut self, name: impl Into<Name>) -> Self {
    self.icon = Some(name.into());
    self
  }

  /// What the offer is called.
  pub fn label(&self) -> &str {
    &self.label
  }

  /// The text it puts in the field after `trigger`.
  fn inserted(&self, trigger: char) -> Arc<str> {
    self
      .text
      .clone()
      .unwrap_or_else(|| Arc::from(format!("{trigger}{}", self.label)))
  }
}

/// What a trigger offers: a list, which the field narrows to the offers whose label or text has what was typed
/// after the trigger, or a function of what was typed.
pub enum MentionSource {
  List(Vec<Mention>),
  Find(Box<dyn Fn(&str) -> Vec<Mention> + Send + Sync>),
}

impl From<Vec<Mention>> for MentionSource {
  fn from(list: Vec<Mention>) -> Self {
    Self::List(list)
  }
}

impl<F: Fn(&str) -> Vec<Mention> + Send + Sync + 'static> From<F> for MentionSource {
  fn from(find: F) -> Self {
    Self::Find(Box::new(find))
  }
}

/// The most offers a list shows.
pub const MENTIONS_LIMIT: usize = 8;

impl MentionSource {
  /// The offers for `query`, typed after `trigger`.
  fn offers(self, trigger: char, query: &str) -> Vec<Mention> {
    let mut offers = match self {
      Self::Find(find) => find(query),
      Self::List(list) => {
        let query = query.to_lowercase();
        list
          .into_iter()
          .filter(|offer| {
            offer.label.to_lowercase().contains(&query) || offer.inserted(trigger).to_lowercase().contains(&query)
          })
          .collect()
      }
    };
    offers.truncate(MENTIONS_LIMIT);
    offers
  }
}

/// What a mentions field keeps from one render to the next, which the app keeps for it as it keeps its value: the
/// controller of its text, the offer Enter chooses, whether Escape put the list away, and the mentions chosen.
#[derive(Clone)]
pub struct MentionsController {
  input: TextInputController,
  inner: Arc<Mutex<Kept>>,
  /// Changed to render the field again where its text did not change: a key that moved the caret or the offer.
  tick: Signal<u64>,
  anchor: ElementRef,
}

#[derive(Default)]
struct Kept {
  active: usize,
  /// Where the trigger is and what was typed after it, as the list was last drawn.
  query: Option<(usize, String)>,
  /// The trigger Escape put the list away from.
  dismissed: Option<usize>,
  chosen: Vec<Arc<str>>,
}

impl Default for MentionsController {
  fn default() -> Self {
    Self::new()
  }
}

impl MentionsController {
  pub fn new() -> Self {
    Self {
      input: TextInputController::new(),
      inner: Arc::default(),
      tick: Signal::new(0),
      anchor: ElementRef::new(),
    }
  }

  /// The controller of the field's text: its selection, and text put at it.
  pub fn input(&self) -> TextInputController {
    self.input.clone()
  }

  /// Makes `text` a mention wherever the field's text has it as a word of its own: what a field loaded with a
  /// message that has mentions in it holds.
  pub fn mention(&self, text: &str) {
    let mut kept = self.inner.lock();
    if !kept.chosen.iter().any(|chosen| chosen.as_ref() == text) && !text.is_empty() {
      kept.chosen.push(Arc::from(text));
    }
    drop(kept);
    self.redraw();
  }

  /// The mentions chosen so far, as their text.
  pub fn mentioned(&self) -> Vec<Arc<str>> {
    self.inner.lock().chosen.clone()
  }

  fn redraw(&self) {
    self.tick.update(|tick| *tick = tick.wrapping_add(1));
  }

  /// Puts `text` and a space in place of the trigger at `trigger` and what was typed after it up to the caret,
  /// as one edit, and makes it a mention.
  fn choose(&self, trigger: usize, text: &str) {
    let Ok(selection) = self.input.selection() else {
      return;
    };
    if self.input.select(trigger, selection.caret.max(trigger)).is_err() {
      return;
    }
    if self.input.replace_selection(&format!("{text} ")).unwrap_or(false) {
      let mut kept = self.inner.lock();
      if !kept.chosen.iter().any(|chosen| chosen.as_ref() == text) {
        kept.chosen.push(Arc::from(text));
      }
      kept.active = 0;
      kept.query = None;
      kept.dismissed = None;
    }
    self.redraw();
  }
}

/// A trigger and what it offers.
struct Trigger {
  character: char,
  source: MentionSource,
}

/// A mentions field: text that a person types, in which a trigger (`@`, `#`, `/`) opens a list of what can be
/// mentioned, and what is chosen stands as a token. A comment, a message, a prompt.
///
/// ```
/// use lurq::{core::Signal, ui::{Avatar, Mention, Mentions, MentionsController}};
///
/// let message = Signal::new(String::new());
/// // Kept by the app, with the value, from one render to the next.
/// let mentions = MentionsController::new();
/// let people = vec![
///   Mention::new("Ivan Zaida").avatar(Avatar::new("Ivan Zaida")),
///   Mention::new("Ivy Chen").avatar(Avatar::new("Ivy Chen")),
/// ];
/// let field = Mentions::new(message, &mentions)
///   .trigger('@', people)
///   .trigger('#', |typed: &str| vec![Mention::new(&format!("{typed}-channel"))])
///   .placeholder("Write a comment");
/// ```
///
/// It is a [`TokenInput`] on rows. Typing a trigger at the start of a word opens the list under it, of the offers
/// whose label has what is typed after it (or those a function gives for it); Up and Down move the offer Enter
/// (or Tab) chooses, a click chooses one, and Escape puts the list away to go on typing plain text. A choice puts
/// the offer's text in place of the trigger and what was typed, and from then on that text is a mention: a token
/// of the variant [`MENTION`], one thing to the caret, which Backspace selects and then deletes. The value is the
/// text, mentions and all, as the app's text for each.
pub struct Mentions {
  input: TokenInput,
  controller: MentionsController,
  triggers: Vec<Trigger>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  on_mention: Vec<Arc<dyn Fn(&str) + Send + Sync>>,
  read_only: bool,
  disabled: bool,
  full_width: bool,
}

impl Mentions {
  /// A mentions field bound to `value`, keeping what it keeps in `controller`.
  pub fn new(value: Signal<String>, controller: &MentionsController) -> Self {
    Self {
      input: TokenInput::new(value).rows(2, 8),
      controller: controller.clone(),
      triggers: Vec::new(),
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      on_mention: Vec::new(),
      read_only: false,
      disabled: false,
      full_width: false,
    }
  }

  /// Offers `options` after `trigger`: a list of [`Mention`]s, narrowed to what is typed after the trigger, or a
  /// function of what is typed.
  pub fn trigger(mut self, trigger: char, options: impl Into<MentionSource>) -> Self {
    self.triggers.retain(|held| held.character != trigger);
    self.triggers.push(Trigger {
      character: trigger,
      source: options.into(),
    });
    self
  }

  /// Called with the text of an offer as it is chosen.
  pub fn on_mention(mut self, handler: impl Fn(&str) + Send + Sync + 'static) -> Self {
    self.on_mention.push(Arc::new(handler));
    self
  }

  /// Other tokens of the app's in the text, besides the mentions: how it finds them.
  pub fn tokens(mut self, find: impl Fn(&str) -> Vec<Token> + Send + Sync + 'static) -> Self {
    self.input = self.input.tokens(find);
    self
  }

  /// The rows the field shows while it holds less, and grows to before it scrolls: 2 and 8 unless told.
  pub fn rows(mut self, min_rows: usize, max_rows: usize) -> Self {
    self.input = self.input.rows(min_rows, max_rows);
    self
  }

  /// Whether the field is as wide as what holds it, in place of the design's 280.
  pub fn full_width(mut self, full_width: bool) -> Self {
    self.full_width = full_width;
    self.input = self.input.full_width(full_width);
    self
  }

  /// What the field says while it is empty.
  pub fn placeholder(mut self, placeholder: &str) -> Self {
    self.input = self.input.placeholder(placeholder);
    self
  }

  /// A [`TextBoxSize`]: the type of its text and mentions, 13, 14 or 15.
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

  /// Whether the field can be read and not changed: a trigger opens nothing.
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

  /// What the field is called, to what reads the tree.
  pub fn label(mut self, label: &str) -> Self {
    self.input = self.input.label(label);
    self
  }

  /// Called before an edit is made, with what the field will hold; a handler may refuse it. Choosing an offer is
  /// an edit too.
  pub fn on_input(mut self, handler: impl Fn(&TextInputEvent) + Send + Sync + 'static) -> Self {
    self.input = self.input.on_input(handler);
    self
  }

  /// The name the field's value has in the form it is in.
  #[cfg(feature = "form")]
  pub fn name(mut self, name: impl Into<Arc<str>>) -> Self {
    self.input = self.input.name(name);
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

  /// Lays rules over the token input the field is: its box and its tokens.
  pub fn input_rules(mut self, rules: &Recipe) -> Self {
    self.input = self.input.rules(rules);
    self
  }

  /// Draws the box as it is under the pointer or focused, and the active offer highlighted: for a gallery or a
  /// test.
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

  /// Builds the field in `theme`. Converting a mentions field into an element does this in the theme of what is
  /// being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let controller = self.controller.clone();
    // Read so that what renders the field renders it again when a key moves the offer or the caret, and when the
    // focus comes and goes.
    controller.tick.get();
    let focused = controller.input.element_ref().focus_signal().get();
    let base = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    };
    let own = theme.resolve::<Mentions>(&base, Some(&self.rules));
    // The box's room and least height are the field's own, over the token input's.
    let control = own.part(MentionsPart::Control).rest();
    let root = own.part(MentionsPart::Root).rest();
    let mut box_rules = Recipe::new().base(TokenInputPart::Control, move |s| s.merged(&control));
    if !self.full_width {
      box_rules = box_rules.base(TokenInputPart::Root, move |s| s.merged(&root));
    }
    let chosen = controller.inner.lock().chosen.clone();
    let input = self
      .input
      .controller(controller.input.clone())
      .rules(&box_rules)
      .view(theme, move |text| mentions_in(text, &chosen));
    let text = input.text.clone();
    let caret = controller
      .input
      .selection()
      .ok()
      .filter(|selection| selection.range().is_empty());
    let mut triggers = self.triggers;
    let characters: Vec<char> = triggers.iter().map(|trigger| trigger.character).collect();
    let pending = caret
      .filter(|_| focused && !self.read_only && !self.disabled)
      .and_then(|selection| pending_trigger(&text, selection.caret, &characters, &input.tokens));
    let mut kept = controller.inner.lock();
    // Escape puts the list away while the caret is in that trigger's word; elsewhere it is forgotten.
    if pending.as_ref().map(|(at, ..)| *at) != kept.dismissed {
      kept.dismissed = None;
    }
    let pending = pending.filter(|(at, ..)| kept.dismissed != Some(*at));
    let offers = match &pending {
      Some((at, character, query)) => {
        let index = triggers.iter().position(|trigger| trigger.character == *character);
        let source = index.map(|index| triggers.swap_remove(index).source);
        let offers = source
          .map(|source| source.offers(*character, query))
          .unwrap_or_default();
        // A new query starts at the first offer.
        if kept.query.as_ref() != Some(&(*at, query.clone())) {
          kept.query = Some((*at, query.clone()));
          kept.active = 0;
        }
        offers
      }
      None => {
        kept.query = None;
        Vec::new()
      }
    };
    let active = kept.active.min(offers.len().saturating_sub(1));
    kept.active = active;
    drop(kept);
    let open = !offers.is_empty();
    let trigger = pending.as_ref().map(|(at, character, _)| (*at, *character));
    let case = base.state(MentionsState::Open, open);
    let view = MentionsView {
      input,
      offers,
      active,
      trigger,
      open,
      recipe: theme.resolve::<Mentions>(&case, Some(&self.rules)),
      case,
      rules: self.rules,
      forced: self.forced,
      controller,
      on_mention: self.on_mention,
    };
    match theme.renderer::<Mentions>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

/// The mentions in `text`: every place a chosen mention's text stands as a word of its own.
fn mentions_in(text: &str, chosen: &[Arc<str>]) -> Vec<Token> {
  let mut tokens = Vec::new();
  for mention in chosen {
    for (at, found) in text.match_indices(mention.as_ref()) {
      let end = at + found.len();
      let starts = text[..at].chars().next_back().is_none_or(char::is_whitespace);
      let ends = text[end..]
        .chars()
        .next()
        .is_none_or(|next| next.is_whitespace() || next.is_ascii_punctuation());
      if starts && ends {
        tokens.push(Token::new(at..end).variant(MENTION).removable(false));
      }
    }
  }
  tokens
}

/// The trigger the caret is in the word of: where it is, which it is, and what is typed after it up to the caret.
/// The word starts with the trigger, at the start of the text or after a space, and is no token.
fn pending_trigger(text: &str, caret: usize, triggers: &[char], tokens: &[Token]) -> Option<(usize, char, String)> {
  let before = text.get(..caret)?;
  let start = before
    .char_indices()
    .rev()
    .find(|(_, character)| character.is_whitespace())
    .map_or(0, |(at, character)| at + character.len_utf8());
  let word = &before[start..];
  let character = word.chars().next()?;
  if !triggers.contains(&character) {
    return None;
  }
  if tokens
    .iter()
    .any(|token| token.range().start <= start && start < token.range().end)
  {
    return None;
  }
  Some((start, character, word[character.len_utf8()..].to_owned()))
}

impl FieldControl for Mentions {
  /// Named, judged and disabled by the field as a token input is.
  fn in_field(mut self, field: &InField) -> Element {
    self.disabled |= field.disabled;
    self.input = self.input.in_field(field);
    self.into()
  }
}

impl From<Mentions> for Element {
  fn from(mentions: Mentions) -> Self {
    mentions.build(&current_theme())
  }
}

/// A mentions field as a renderer sees it: the token input it is, the offers for the trigger being typed, and the
/// style of its own parts.
pub struct MentionsView {
  /// The token input the field is, its mentions among its tokens.
  pub input: TokenInputView,
  /// What the list offers, in its order: none while it is closed.
  pub offers: Vec<Mention>,
  /// The offer Enter chooses.
  pub active: usize,
  /// The trigger being typed: where it is in the text, and which it is.
  pub trigger: Option<(usize, char)>,
  /// Whether the list is open.
  pub open: bool,
  /// The style of each of its own parts.
  pub recipe: ResolvedRecipe,
  case: Case,
  rules: Recipe,
  forced: Option<Interaction>,
  controller: MentionsController,
  on_mention: Vec<Arc<dyn Fn(&str) + Send + Sync>>,
}

impl MentionsView {
  /// The style of a part, as this field is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// What choosing the offer at `index` does: its text in place of the trigger and what was typed, as a mention.
  fn chooser(&self, index: usize) -> Option<Arc<dyn Fn() + Send + Sync>> {
    let (at, character) = self.trigger?;
    let text = self.offers.get(index)?.inserted(character);
    let (controller, handlers) = (self.controller.clone(), self.on_mention.clone());
    Some(Arc::new(move || {
      controller.choose(at, &text);
      for handler in &handlers {
        handler(&text);
      }
    }))
  }

  /// The token input's control, with the keys of the list: Up and Down move the offer Enter and Tab choose,
  /// Escape puts the list away. Any other key, and a press, has the field look again for a trigger at the caret.
  pub fn control(&self) -> crate::components::TextInput {
    let controller = self.controller.clone();
    let choosers: Vec<_> = (0..self.offers.len()).filter_map(|index| self.chooser(index)).collect();
    let (open, trigger) = (self.open, self.trigger);
    let mut control = self.input.control().on_key_down(move |event: KeyboardEvent| {
      if !event.focus_within {
        return;
      }
      let plain = !event.ctrl && !event.alt && !event.meta;
      if open && plain && !choosers.is_empty() {
        let count = choosers.len();
        let mut kept = controller.inner.lock();
        match event.key.as_str() {
          "ArrowDown" if !event.shift => {
            event.prevent_default();
            kept.active = (kept.active + 1) % count;
          }
          "ArrowUp" if !event.shift => {
            event.prevent_default();
            kept.active = (kept.active + count - 1) % count;
          }
          "Enter" | "Tab" if !event.shift => {
            event.prevent_default();
            let chosen = choosers[kept.active.min(count - 1)].clone();
            drop(kept);
            chosen();
            return;
          }
          "Escape" => {
            event.prevent_default();
            kept.dismissed = trigger.map(|(at, _)| at);
          }
          _ => {}
        }
      }
      controller.redraw();
    });
    let controller = self.controller.clone();
    control = control
      .on_mouse_down(move |_| controller.redraw())
      .describe("a11y_haspopup", "listbox")
      .describe("a11y_expanded", if self.open { "true" } else { "false" });
    control
  }

  /// One offer: its avatar or icon and its label, in a row that a click chooses by and that keeps the focus in the
  /// field.
  pub fn option(&self, index: usize, offer: Mention) -> Element {
    let active = index == self.active;
    let case = self.case.clone().state(MentionsState::Active, active);
    let recipe = self.theme().resolve::<Mentions>(&case, Some(&self.rules));
    let part = recipe.part(MentionsPart::Option).forced(self.forced.filter(|_| active));
    let mut row = Row::new()
      .align_items(Alignment::Center)
      .focusable(false)
      .describe("a11y_role", "option")
      .describe("a11y_name", offer.label.clone())
      .describe("a11y_selected", if active { "true" } else { "false" })
      .part(&part);
    if let Some(avatar) = offer.avatar {
      row = row.child(
        Row::new()
          .part(&recipe.part(MentionsPart::OptionAvatar))
          .child(avatar.size(AvatarSize::Sm).build(self.theme())),
      );
    } else if let Some(name) = offer.icon {
      let icon = recipe.part(MentionsPart::OptionIcon);
      let side = icon.rest().width.unwrap_or(Length::Icon(IconSize::Md));
      row = row.child(
        Row::new()
          .part(&icon)
          .child(Icon::new(name).size(side).build(self.theme())),
      );
    }
    let label = recipe.part(MentionsPart::OptionLabel);
    row = row.child(Row::new().part(&label).child(label.text(&offer.label).nowrap()));
    if let Some(choose) = self.chooser(index) {
      row = row.on_click(move |_| choose());
    }
    let controller = self.controller.clone();
    row
      .on_mouse_enter(move || {
        let mut kept = controller.inner.lock();
        if kept.active != index {
          kept.active = index;
          drop(kept);
          controller.redraw();
        }
      })
      .into()
  }

  /// The list of offers, hung under the trigger: a panel in the style of its part, open while there are offers.
  /// It takes no focus: the keys stay in the field.
  pub fn list(&mut self) -> Element {
    let offers = std::mem::take(&mut self.offers);
    let mut panel = Column::new()
      .align_items(Alignment::Stretch)
      .part(&self.part(MentionsPart::List))
      .describe("a11y_role", "listbox");
    for (index, offer) in offers.into_iter().enumerate() {
      panel = panel.child(self.option(index, offer));
    }
    Popup::new(self.controller.anchor.clone(), panel)
      .open_when(self.open)
      .placement(Placement::BottomStart)
      .offset(0.0, 4.0)
      .takes_focus(false)
      .dismiss_on_outside_click(false)
      .dismiss_on_escape(false)
      .switch_on_anchor_press(false)
      .into()
  }

  /// A mark of no width at the trigger, which the list hangs from: as high as the text's line there, where the
  /// input last laid it out, inside the box's padding.
  pub fn anchor(&self) -> Element {
    let point = self
      .trigger
      .and_then(|(at, _)| self.controller.input.caret_point(at).ok());
    let control = self.input.part(TokenInputPart::Control).rest();
    let spacing = self.theme().spacing();
    let pad = |side: &Option<crate::node::SpacingValue>, both: &Option<crate::node::SpacingValue>| {
      side
        .as_ref()
        .or(both.as_ref())
        .map_or(0.0, |value| value.resolve(&spacing, 0.0))
    };
    let (left, top) = (
      pad(&control.padding_start, &control.padding_x),
      pad(&control.padding_top, &control.padding_y),
    );
    let (x, y, height) = point.map_or((0.0, 0.0, 1.0), |point| (point.x, point.y, point.height));
    Row::new()
      .padding_left((left + x).max(0.0))
      .padding_top((top + y).max(0.0))
      .pointer_events_none()
      .child(Rect::new(1.0, height.max(1.0)).ref_element(self.controller.anchor.clone()))
      .into()
  }

  /// How the default renderer builds a mentions field: the token input's control, the mark the list hangs from,
  /// and the list.
  pub fn render(mut self) -> Element {
    let control = self.control();
    let anchor = self.anchor();
    let list = self.list();
    self
      .input
      .root(control)
      .overflow_visible()
      .child(anchor)
      .child(list)
      .into()
  }
}

impl Themed for Mentions {
  const NAME: &'static str = "mentions";
  type Renderer = MentionsRenderer;

  /// The design's mentions (`pencil/lurq-design-system.pen`, Mentions): a box 280 wide and at least 64 high,
  /// padded 12 across and 8 down, its text in the body's type with lines of 1.5. The list hangs 4 under the
  /// trigger: 220 wide, in the popover's color with the large radius, an edge of 1 in the border's color, the
  /// medium shadow and 4 of padding. An offer is 36 high, padded 8, with the small radius and 8 between its avatar
  /// of 24 (or icon of 16, muted) and its label in the body's type; the active one is filled with the highlight,
  /// as an offer under the pointer is.
  fn recipe() -> Recipe {
    use MentionsPart::{Control, List, Option, OptionAvatar, OptionIcon, OptionLabel, Root};
    use PaletteColor::{Foreground, Highlight, MutedForeground, Popover, PopoverForeground};
    Recipe::new()
      .base(Root, |s| s.width(280.0))
      .base(Control, |s| {
        s.padding_x(12.0).padding_y(8.0).min_height(64.0).line_height(1.5)
      })
      .base(List, |s| {
        s.width(220.0)
          .padding_x(4.0)
          .padding_y(4.0)
          .radius(RadiusSize::Lg)
          .background(Popover)
          .foreground(PopoverForeground)
          .border(Border::inside(BorderSize::Sm, PaletteColor::Border))
          .shadow(ShadowStyle::Md)
      })
      .base(Option, |s| {
        s.height(36.0)
          .padding_x(8.0)
          .gap(8.0)
          .radius(RadiusSize::Sm)
          .cursor(CursorIcon::Pointer)
      })
      .base(OptionAvatar, |s| s.square(24.0))
      .base(OptionIcon, |s| s.square(IconSize::Md).foreground(MutedForeground))
      .base(OptionLabel, |s| {
        s.typography(TypographyStyle::Body)
          .foreground(Foreground)
          .width(Length::Fill)
      })
      .hovered(Option, |s| s.background(Highlight))
      .state(MentionsState::Active, Option, |s| s.background(Highlight))
  }
}
