//! A message box: a dialog that asks one question or says one thing, and waits for an answer.

use std::sync::Arc;

use super::{
  button::{Button, ButtonPart, ButtonSize, ButtonVariant},
  checkbox::{Checkbox, CheckboxPart},
  collapsible::opening,
  dialog::{Dialog, DialogPart},
  icons, names,
  part_style::PartStyle,
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::{glyph, pressable},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    events::KeyboardEvent,
    theme::{PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::{Column, Row},
  core::Signal,
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{Element, NodeUpdate, dimension::Dimension, node::Node},
};

/// What a message box says it is about, by the icon it shows and its color.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MessageBoxKind {
  /// It says something: an operation is done, a version is installed.
  #[default]
  Information,
  /// It asks something that has no wrong answer: open the file in a new window?
  Question,
  /// What is about to happen may lose something: replace an existing file?
  Warning,
  /// Something failed: the file could not be saved.
  Error,
}

names!(MessageBoxKind {
  Information => "information",
  Question => "question",
  Warning => "warning",
  Error => "error",
});

/// What an answer of a message box is: which button it is, and where it stands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MessageBoxRole {
  /// The answer Enter gives: the filled button at the end.
  Default,
  /// The answer that changes nothing, which Escape and a press of the veil
  /// give too: the outlined button before the others at the end.
  Cancel,
  /// An answer that removes something or cannot be undone: a button in the
  /// destructive color. The focus goes to the cancel button of a message box
  /// that has one.
  Destructive,
  /// Another answer: an outlined button at the start of the row ("Don't Save").
  #[default]
  Plain,
}

/// The parts of a message box a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MessageBoxPart {
  /// What covers the window behind the panel, laid over the dialog's veil.
  Veil,
  /// The panel, laid over the dialog's: its width and its padding.
  Panel,
  /// What the panel holds: the body over the footer.
  Content,
  /// The icon beside the text.
  Body,
  /// The round field the icon is in, in the kind's muted color.
  IconWell,
  /// The icon of the kind, in the kind's color.
  Icon,
  /// The title over the message, and the detail under them.
  Text,
  /// What the message box asks or says, in a line.
  Title,
  /// More of it, under the title.
  Message,
  /// What opens the detail: an arrow and its label.
  DetailTrigger,
  /// The arrow of the trigger, which points down while the detail is open.
  DetailIcon,
  /// The label of the trigger: "Details".
  DetailLabel,
  /// What opens and closes with the detail.
  DetailPanel,
  /// The detail itself: the text a person opens to see more.
  Detail,
  /// The row at the bottom: the checkbox and the plain answers at its start, the others at its end.
  Footer,
  /// The checkbox and the plain answers, at the start of the footer.
  Option,
  /// The checkbox, laid over the checkbox's root.
  Checkbox,
  /// The checkbox's label, laid over the checkbox's.
  CheckboxLabel,
  /// The answers at the end of the footer.
  Actions,
  /// The default answer, laid over the button's root.
  DefaultButton,
  /// The label of the default answer, laid over the button's.
  DefaultLabel,
  /// The cancel answer, laid over the button's root.
  CancelButton,
  /// The label of the cancel answer, laid over the button's.
  CancelLabel,
  /// A destructive answer, laid over the button's root.
  DestructiveButton,
  /// The label of a destructive answer, laid over the button's.
  DestructiveLabel,
  /// A plain answer, laid over the button's root.
  PlainButton,
  /// The label of a plain answer, laid over the button's.
  PlainLabel,
}

names!(MessageBoxPart {
  Veil => "veil",
  Panel => "panel",
  Content => "content",
  Body => "body",
  IconWell => "icon_well",
  Icon => "icon",
  Text => "text",
  Title => "title",
  Message => "message",
  DetailTrigger => "detail_trigger",
  DetailIcon => "detail_icon",
  DetailLabel => "detail_label",
  DetailPanel => "detail_panel",
  Detail => "detail",
  Footer => "footer",
  Option => "option",
  Checkbox => "checkbox",
  CheckboxLabel => "checkbox_label",
  Actions => "actions",
  DefaultButton => "default_button",
  DefaultLabel => "default_label",
  CancelButton => "cancel_button",
  CancelLabel => "cancel_label",
  DestructiveButton => "destructive_button",
  DestructiveLabel => "destructive_label",
  PlainButton => "plain_button",
  PlainLabel => "plain_label",
});

/// The states of a message box a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MessageBoxState {
  /// One of its answers is destructive: its icon is in the destructive colors, whatever its kind.
  Destructive,
  /// Its detail is open.
  DetailOpen,
}

names!(MessageBoxState {
  Destructive => "destructive",
  DetailOpen => "detail_open",
});

/// What builds a message box's nodes: a theme's in place of the default one.
pub type MessageBoxRenderer = Arc<dyn Fn(MessageBoxView) -> Element + Send + Sync>;

/// What an answer does when it is given.
type Handler = Arc<dyn Fn() + Send + Sync>;

/// One answer of a message box: its label, its role and what it does.
#[derive(Clone)]
pub struct MessageBoxAnswer {
  pub label: Arc<str>,
  pub role: MessageBoxRole,
  handler: Handler,
}

/// A message box: a dialog that asks one question or says one thing and
/// waits for the answer. Replace a file, save the changes, delete projects.
///
/// ```
/// use lurq::{
///   core::Signal,
///   ui::{MessageBox, MessageBoxKind, MessageBoxRole},
/// };
///
/// let (open, again) = (Signal::new(false), Signal::new(false));
/// let replace = MessageBox::new(open)
///   .kind(MessageBoxKind::Warning)
///   .title("Replace existing file?")
///   .message("“theme.json” already exists in this folder. Replacing it overwrites its contents.")
///   .checkbox("Don't ask again", again)
///   .answer("Cancel", MessageBoxRole::Cancel, || {})
///   .answer("Replace", MessageBoxRole::Default, || println!("replaced"));
/// ```
///
/// A message box does not keep whether it is open: the app's signal does, and
/// any answer sets it to `false` before what the answer does runs. It is a
/// [`Dialog`]: the veil, the focus going in as it opens and back as it
/// closes, and Tab staying inside are the dialog's. Enter gives the default
/// answer wherever the focus is, unless it is on another button; Escape and a
/// press of the veil give the cancel answer where there is one, and do
/// nothing where there is none. As it opens the focus goes to the default
/// answer, or to the cancel answer of a box with a destructive one. To
/// tooling that reads the tree it is an `alertdialog`, called by its title and
/// described by its message.
///
/// Its answers are [`Button`]s styled by the message box's recipe:
/// [`MessageBoxPart::DefaultButton`] is laid over the root of the default
/// one, [`MessageBoxPart::DefaultLabel`] over its label, and so for each
/// role, so that a theme gives the buttons of a message box a fill or a font
/// of their own without touching buttons elsewhere.
pub struct MessageBox {
  open: Signal<bool>,
  kind: Name,
  icon: Option<Name>,
  title: Option<Arc<str>>,
  message: Option<Arc<str>>,
  detail: Option<(Arc<str>, Signal<bool>)>,
  detail_label: Arc<str>,
  checkbox: Option<(Arc<str>, Signal<bool>)>,
  answers: Vec<MessageBoxAnswer>,
  inline: bool,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl MessageBox {
  /// A message box shown while `open` holds `true`.
  pub fn new(open: Signal<bool>) -> Self {
    Self {
      open,
      kind: MessageBoxKind::default().into(),
      icon: None,
      title: None,
      message: None,
      detail: None,
      detail_label: Arc::from("Details"),
      checkbox: None,
      answers: Vec::new(),
      inline: false,
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// A [`MessageBoxKind`], or a kind the theme's recipe names: the icon and its colors.
  pub fn kind(mut self, kind: impl Into<Name>) -> Self {
    self.kind = kind.into();
    self
  }

  /// An icon of the theme's set in place of the kind's: `trash` for a question that deletes.
  pub fn icon(mut self, name: impl Into<Name>) -> Self {
    self.icon = Some(name.into());
    self
  }

  /// What the message box asks or says: "Replace existing file?". It is also
  /// what the message box is called to tooling that reads the tree.
  pub fn title(mut self, title: &str) -> Self {
    self.title = Some(Arc::from(title));
    self
  }

  /// More of it, under the title, in the muted color.
  pub fn message(mut self, message: &str) -> Self {
    self.message = Some(Arc::from(message));
    self
  }

  /// A text a person opens to see more, under the message: what failed and
  /// where. It is open while `open` holds `true`, and its trigger turns it.
  pub fn detail(mut self, detail: &str, open: Signal<bool>) -> Self {
    self.detail = Some((Arc::from(detail), open));
    self
  }

  /// The label of what opens the detail: "Details" when not given.
  pub fn detail_label(mut self, label: &str) -> Self {
    self.detail_label = Arc::from(label);
    self
  }

  /// A checkbox at the start of the footer, checked while `checked` holds
  /// `true`: "Don't ask again".
  pub fn checkbox(mut self, label: &str, checked: Signal<bool>) -> Self {
    self.checkbox = Some((Arc::from(label), checked));
    self
  }

  /// An answer: a button with `label`, which closes the message box and then
  /// runs `handler`. The plain answers stand at the start of the footer in
  /// the order they are given; at its end the cancel answer stands first and
  /// the others after it in their order.
  pub fn answer(mut self, label: &str, role: MessageBoxRole, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.answers.push(MessageBoxAnswer {
      label: Arc::from(label),
      role,
      handler: Arc::new(handler),
    });
    self
  }

  /// A width of its own, in logical pixels, over the board's 400.
  pub fn width(mut self, width: f32) -> Self {
    self.rules = self.rules.base(MessageBoxPart::Panel, |s| s.width(width));
    self
  }

  /// Draws the panel where the message box is declared, whether or not it is
  /// open, without the veil: for a gallery, a screenshot, a test.
  pub fn inline(mut self, inline: bool) -> Self {
    self.inline = inline;
    self
  }

  /// Puts the message box in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this message box over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this message box.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// The id of the panel.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// The key of what the message box lays over the window.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the message box in `theme`. Converting a message box into an
  /// element does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let destructive = self
      .answers
      .iter()
      .any(|answer| answer.role == MessageBoxRole::Destructive);
    let detail_open = self.detail.as_ref().is_some_and(|(_, open)| open.get());
    let case = Case {
      variant: Some(self.kind),
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states,
    }
    .state(MessageBoxState::Destructive, destructive)
    .state(MessageBoxState::DetailOpen, detail_open);
    let view = MessageBoxView {
      kind: self.kind,
      icon: self.icon.unwrap_or_else(|| kind_icon(self.kind)),
      title: self.title,
      message: self.message,
      detail: self.detail.as_ref().map(|(detail, _)| detail.clone()),
      detail_label: self.detail_label,
      detail_open,
      checkbox_label: self.checkbox.as_ref().map(|(label, _)| label.clone()),
      checked: self.checkbox.as_ref().is_some_and(|(_, checked)| checked.get()),
      answers: self.answers,
      inline: self.inline,
      recipe: theme.resolve::<MessageBox>(&case, Some(&self.rules)),
      open: self.open,
      detail_signal: self.detail.map(|(_, open)| open),
      checkbox_signal: self.checkbox.map(|(_, checked)| checked),
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<MessageBox>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<MessageBox> for Element {
  fn from(message_box: MessageBox) -> Self {
    message_box.build(&current_theme())
  }
}

/// The icon of a kind: an information sign for one the theme names.
fn kind_icon(kind: Name) -> Name {
  if kind == Name::from(MessageBoxKind::Warning) {
    icons::WARNING.into()
  } else if kind == Name::from(MessageBoxKind::Error) {
    icons::ALERT_CIRCLE.into()
  } else {
    // The icon set has no question mark yet: a question shows the information sign.
    icons::INFO.into()
  }
}

/// A message box as a renderer sees it: what it was given, the style of its
/// parts, and its behavior ready to be dressed.
pub struct MessageBoxView {
  pub kind: Name,
  /// The icon it shows: the kind's, or the one it was given.
  pub icon: Name,
  pub title: Option<Arc<str>>,
  pub message: Option<Arc<str>>,
  pub detail: Option<Arc<str>>,
  pub detail_label: Arc<str>,
  pub detail_open: bool,
  /// The label of the checkbox, if it has one.
  pub checkbox_label: Option<Arc<str>>,
  pub checked: bool,
  /// Its answers, in the order they were given.
  pub answers: Vec<MessageBoxAnswer>,
  pub inline: bool,
  /// The style of each part of this message box.
  pub recipe: ResolvedRecipe,
  open: Signal<bool>,
  detail_signal: Option<Signal<bool>>,
  checkbox_signal: Option<Signal<bool>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl MessageBoxView {
  /// The style of a part, as this message box is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// What giving the answer at `index` does: the message box closes, and
  /// then the answer's handler runs.
  pub fn press(&self, index: usize) -> impl Fn() + Send + Sync + Clone + 'static {
    let open = self.open.clone();
    let handler = self.answers.get(index).map(|answer| answer.handler.clone());
    move || {
      open.set(false);
      if let Some(handler) = &handler {
        handler();
      }
    }
  }

  /// The index of the first answer of `role`.
  pub fn find(&self, role: MessageBoxRole) -> Option<usize> {
    self.answers.iter().position(|answer| answer.role == role)
  }

  /// The answer the focus goes to as the message box opens: the cancel one
  /// where an answer is destructive, else the default one.
  pub fn focused_first(&self) -> Option<usize> {
    if self.find(MessageBoxRole::Destructive).is_some() {
      self.find(MessageBoxRole::Cancel)
    } else {
      self.find(MessageBoxRole::Default)
    }
  }

  /// The parts of the button and of its label for an answer of `role`.
  pub fn button_parts(role: MessageBoxRole) -> (MessageBoxPart, MessageBoxPart) {
    match role {
      MessageBoxRole::Default => (MessageBoxPart::DefaultButton, MessageBoxPart::DefaultLabel),
      MessageBoxRole::Cancel => (MessageBoxPart::CancelButton, MessageBoxPart::CancelLabel),
      MessageBoxRole::Destructive => (MessageBoxPart::DestructiveButton, MessageBoxPart::DestructiveLabel),
      MessageBoxRole::Plain => (MessageBoxPart::PlainButton, MessageBoxPart::PlainLabel),
    }
  }

  /// The button of the answer at `index`: a [`Button`] of the role's variant,
  /// with the message box's rules for the role laid over the theme's for a
  /// button. The focus goes to it as the message box opens where it is
  /// [`MessageBoxView::focused_first`].
  pub fn button(&self, index: usize) -> Element {
    let answer = &self.answers[index];
    let variant = match answer.role {
      MessageBoxRole::Default => ButtonVariant::Primary,
      MessageBoxRole::Destructive => ButtonVariant::Destructive,
      MessageBoxRole::Cancel | MessageBoxRole::Plain => ButtonVariant::Outline,
    };
    let (root, label) = Self::button_parts(answer.role);
    let rules = self
      .recipe
      .rules_for(root, ButtonPart::Root)
      .extend(&self.recipe.rules_for(label, ButtonPart::Label));
    let press = self.press(index);
    let button = Button::new(&answer.label)
      .variant(variant)
      .size(ButtonSize::Sm)
      .rules(&rules)
      .on_click(move |_| press())
      .build(self.theme());
    if self.focused_first() == Some(index) {
      autofocus(button)
    } else {
      button
    }
  }

  /// The round field with the kind's icon in it.
  pub fn icon(&self) -> Row {
    Row::new()
      .justify(Justify::Center)
      .align_items(Alignment::Center)
      .flex_shrink(0.0)
      .part(&self.part(MessageBoxPart::IconWell))
      .child(glyph(&self.part(MessageBoxPart::Icon), self.icon, false))
  }

  /// The title in the style of its part, where there is one.
  pub fn title(&self) -> Option<Element> {
    let title = self.title.as_ref()?;
    let part = self.part(MessageBoxPart::Title);
    Some(
      Row::new()
        .width(Dimension::Pct(100.0))
        .part(&part)
        .child(part.text(title))
        .into(),
    )
  }

  /// The message in the style of its part, where there is one.
  pub fn message(&self) -> Option<Element> {
    let message = self.message.as_ref()?;
    let part = self.part(MessageBoxPart::Message);
    Some(
      Row::new()
        .width(Dimension::Pct(100.0))
        .part(&part)
        .child(part.text(message))
        .into(),
    )
  }

  /// What opens the detail, with what opens and closes under it: `None` for a message box with no detail.
  pub fn detail(&self) -> Option<Element> {
    let (detail, open) = (self.detail.as_ref()?, self.detail_signal.as_ref()?);
    let (trigger, panel) = disclosure(
      &self.recipe,
      DisclosureParts {
        trigger: MessageBoxPart::DetailTrigger.into(),
        icon: MessageBoxPart::DetailIcon.into(),
        label: MessageBoxPart::DetailLabel.into(),
        panel: MessageBoxPart::DetailPanel.into(),
        content: MessageBoxPart::Detail.into(),
      },
      &self.detail_label,
      detail,
      open,
    );
    Some(
      Column::new()
        .width(Dimension::Pct(100.0))
        .align_items(Alignment::Start)
        .overflow_visible()
        .child(trigger)
        .child(panel)
        .into(),
    )
  }

  /// The title, the message and the detail, one over another.
  pub fn text(&self) -> Column {
    let mut text = Column::new()
      .flex(1.0)
      .align_items(Alignment::Stretch)
      .overflow_visible()
      .part(&self.part(MessageBoxPart::Text));
    for element in [self.title(), self.message(), self.detail()].into_iter().flatten() {
      text = text.child(element);
    }
    text
  }

  /// The icon beside the text.
  pub fn body(&self) -> Row {
    Row::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Start)
      .overflow_visible()
      .part(&self.part(MessageBoxPart::Body))
      .child(self.icon())
      .child(self.text())
  }

  /// The checkbox, where there is one: Enter on it gives the default answer and does not turn it.
  pub fn checkbox(&self) -> Option<Element> {
    let (label, checked) = (self.checkbox_label.as_ref()?, self.checkbox_signal.clone()?);
    let rules = self
      .recipe
      .rules_for(MessageBoxPart::Checkbox, CheckboxPart::Root)
      .extend(
        &self
          .recipe
          .rules_for(MessageBoxPart::CheckboxLabel, CheckboxPart::Label),
      );
    let checkbox = Checkbox::new(label)
      .checked(self.checked)
      .rules(&rules)
      .on_change(move |on| checked.set(on))
      .build(self.theme());
    let mut row = Row::new().overflow_visible().child(checkbox);
    if let Some(default) = self.find(MessageBoxRole::Default) {
      let press = self.press(default);
      row = row.on_key_down(move |event: KeyboardEvent| {
        if event.focus_within && enter(&event) {
          event.prevent_default();
          press();
        }
      });
    }
    Some(row.into())
  }

  /// The checkbox and the plain answers, at the start of the footer: `None` where there are neither.
  pub fn option(&self) -> Option<Row> {
    let plain: Vec<usize> = (0..self.answers.len())
      .filter(|index| self.answers[*index].role == MessageBoxRole::Plain)
      .collect();
    let checkbox = self.checkbox();
    if checkbox.is_none() && plain.is_empty() {
      return None;
    }
    let mut option = Row::new()
      .align_items(Alignment::Center)
      .overflow_visible()
      .part(&self.part(MessageBoxPart::Option));
    if let Some(checkbox) = checkbox {
      option = option.child(checkbox);
    }
    for index in plain {
      option = option.child(self.button(index));
    }
    Some(option)
  }

  /// The answers at the end of the footer: the cancel one, and after it the others but the plain ones.
  pub fn actions(&self) -> Row {
    let cancel = self.find(MessageBoxRole::Cancel);
    let order = cancel.into_iter().chain(
      (0..self.answers.len())
        .filter(|index| Some(*index) != cancel && self.answers[*index].role != MessageBoxRole::Plain),
    );
    let mut actions = Row::new()
      .align_items(Alignment::Center)
      .overflow_visible()
      .part(&self.part(MessageBoxPart::Actions));
    for index in order {
      actions = actions.child(self.button(index));
    }
    actions
  }

  /// The footer: the checkbox and the plain answers at its start, the others at its end.
  pub fn footer(&self) -> Row {
    let mut footer = Row::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Center)
      .overflow_visible()
      .part(&self.part(MessageBoxPart::Footer));
    match self.option() {
      Some(option) => {
        footer = footer.justify(Justify::SpaceBetween).child(option);
      }
      None => footer = footer.justify(Justify::End),
    }
    footer.child(self.actions())
  }

  /// What the panel holds, with nothing in it yet: Enter gives the default
  /// answer while the focus is nowhere in it, as Escape gives the cancel one
  /// (the dialog's, with what [`Dialog::on_dismiss`] runs).
  pub fn content(&self) -> Column {
    let mut content = Column::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Stretch)
      .overflow_visible()
      .part(&self.part(MessageBoxPart::Content));
    // Drawn where it is declared, it is no dialog over the window, and the Enter of the page is not its own.
    if !self.inline
      && let Some(default) = self.find(MessageBoxRole::Default)
    {
      let press = self.press(default);
      content = content.on_key_down(move |event: KeyboardEvent| {
        if !event.focus_within && enter(&event) {
          event.prevent_default();
          press();
        }
      });
    }
    content
  }

  /// `content` in a [`Dialog`]: its panel and its veil in the message box's
  /// rules over the dialog's, its Escape and a press of its veil giving the
  /// cancel answer where there is one, and it an `alertdialog` called by the
  /// title and described by the message.
  pub fn dialog(&self, content: impl Into<Element>) -> Element {
    let rules = self
      .recipe
      .rules_for(MessageBoxPart::Panel, DialogPart::Panel)
      .extend(&self.recipe.rules_for(MessageBoxPart::Veil, DialogPart::Veil));
    let cancel = self.find(MessageBoxRole::Cancel);
    let mut dialog = Dialog::new(self.open.clone())
      .alert(true)
      .rules(&rules)
      .inline(self.inline)
      .dismiss_on_escape(cancel.is_some())
      .dismiss_on_veil(cancel.is_some())
      .child(content);
    if let Some(cancel) = cancel
      && let Some(answer) = self.answers.get(cancel)
    {
      let handler = answer.handler.clone();
      dialog = dialog.on_dismiss(move || handler());
    }
    if let Some(title) = &self.title {
      dialog = dialog.named(title);
    }
    if let Some(message) = &self.message {
      dialog = dialog.described(message);
    }
    if let Some(id) = &self.id {
      dialog = dialog.id(id.clone());
    }
    if let Some(key) = &self.key {
      dialog = dialog.key(key.clone());
    }
    dialog.build(self.theme())
  }

  /// How the default renderer builds a message box: the body over the
  /// footer, in a dialog.
  pub fn render(self) -> Element {
    let content = self.content().child(self.body()).child(self.footer());
    self.dialog(content)
  }
}

/// Whether a key event is a plain Enter that nothing took yet.
pub(crate) fn enter(event: &KeyboardEvent) -> bool {
  event.key == "Enter" && !event.composing && !event.default_prevented() && !(event.ctrl || event.alt || event.meta)
}

/// `element` with the first node in it that takes the focus marked to take it as its dialog opens.
pub(crate) fn autofocus(mut element: Element) -> Element {
  fn mark(node: &mut Node) -> bool {
    if node.is_focusable() {
      NodeUpdate::autofocus(node, true);
      return true;
    }
    node.children.iter_mut().any(mark)
  }
  mark(&mut element.node);
  element
}

/// The parts of a component that draw what opens a detail and the detail.
pub(crate) struct DisclosureParts {
  pub trigger: Name,
  pub icon: Name,
  pub label: Name,
  pub panel: Name,
  pub content: Name,
}

/// What opens a text and what opens and closes with the text, open while
/// `open` holds `true`: the board's "Details", an arrow and its label, the
/// arrow pointing down while it is open. The trigger is a button to what
/// reads the tree, and says whether it is open; the panel is as wide as what
/// it is put in.
pub(crate) fn disclosure(
  recipe: &ResolvedRecipe,
  parts: DisclosureParts,
  label: &str,
  text: &str,
  open: &Signal<bool>,
) -> (crate::components::Button, Column) {
  let is_open = open.get();
  let turn = open.clone();
  let label_part = recipe.part(parts.label);
  let trigger = pressable(&recipe.part(parts.trigger), false)
    .justify(Justify::Start)
    .align_items(Alignment::Center)
    .describe("a11y_role", "button")
    .describe("a11y_name", label)
    .describe("a11y_expanded", if is_open { "true" } else { "false" })
    .on_click(move |_| turn.update(|open| *open = !*open))
    .child(glyph(
      &recipe.part(parts.icon),
      if is_open {
        icons::CHEVRON_DOWN
      } else {
        icons::CHEVRON_RIGHT
      }
      .into(),
      false,
    ))
    .child(Row::new().part(&label_part).child(label_part.text(label)));
  let mut panel = Column::new()
    .align_items(Alignment::Stretch)
    .clip()
    .presence()
    .part(&recipe.part(parts.panel));
  if is_open {
    let part = recipe.part(parts.content);
    // Keyed, so that what the panel held is kept as it closes over it.
    panel = panel.child(
      Column::new()
        .key("detail")
        .align_items(Alignment::Stretch)
        .part(&part)
        .child(part.text(text)),
    );
  }
  (trigger, panel.width(Dimension::Pct(100.0)))
}

/// The style of what opens a detail and of the detail, in the board's
/// "Details" of a progress dialog: a chevron of 14 in the muted color, 4
/// before its label of 13, and the detail 8 under it in the mono type of 11
/// on the muted fill. It opens with its height over the theme's base duration.
pub(crate) fn disclosure_recipe(recipe: Recipe, parts: &DisclosureParts, open: Name) -> Recipe {
  use PaletteColor::*;
  let recipe = recipe
    .base(parts.trigger, |s| {
      super::shared::fading(s.gap(4.0).radius(RadiusSize::Sm).cursor(crate::node::CursorIcon::Pointer))
    })
    .focus_visible(parts.trigger, |s| s.focus_ring())
    .base(parts.icon, |s| s.square(14.0).foreground(MutedForeground))
    .base(parts.label, |s| {
      s.typography(TypographyStyle::Body).font_size(13.0).foreground(Foreground)
    })
    .base(parts.content, |s| {
      s.typography(TypographyStyle::Mono)
        .font_size(11.0)
        .line_height(1.5)
        .foreground(MutedForeground)
        .background(Muted)
        .radius(RadiusSize::Md)
        .padding_x(10.0)
        .padding_y(8.0)
    })
    // The 8 between the trigger and the detail is the panel's, so that nothing of it is left while it is closed.
    .state(open, parts.panel, |s| s.padding_top(8.0));
  opening(recipe, open, parts.panel, parts.content)
}

impl Themed for MessageBox {
  const NAME: &'static str = "message_box";
  type Renderer = MessageBoxRenderer;

  /// The design's message box (`pencil/lurq-design-system.pen`, Message Box
  /// and Desktop Dialogs): a dialog's panel 400 wide, padded 20 with 16
  /// between its body and its footer. The body is a round well of 36 in the
  /// kind's muted color with its icon of 18 in the kind's color, 14 before the
  /// title of 15/600 over the message of 13 in the muted color, 4 apart. The
  /// footer has the checkbox, its label of 13, at its start and the buttons
  /// of 32 padded 14 at its end, 8 apart. A destructive question has its icon
  /// in the destructive colors.
  fn recipe() -> Recipe {
    use MessageBoxPart::*;
    use PaletteColor::{
      Destructive as Danger, DestructiveMuted, Foreground, Info, InfoMuted, MutedForeground, Secondary,
      Warning as Caution, WarningMuted,
    };
    let kind = |recipe: Recipe, kind: MessageBoxKind, well: PaletteColor, ink: PaletteColor| {
      recipe
        .variant(kind, IconWell, |s| s.background(well))
        .variant(kind, Icon, |s| s.foreground(ink))
    };
    // Of the 20 the panel is padded, 4 are the dialog's body's, which is room for the ring of a focused button.
    let recipe = Recipe::new()
      .base(Panel, |s| s.width(400.0).padding_x(16.0).padding_y(16.0))
      .base(Content, |s| s.gap(16.0))
      .base(Body, |s| s.gap(14.0))
      .base(IconWell, |s| {
        s.square(36.0).radius(RadiusSize::Full).background(InfoMuted)
      })
      .base(Icon, |s| s.square(18.0).foreground(Info))
      .base(Text, |s| s.gap(4.0))
      .base(Title, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(15.0)
          .weight(FontWeight::SemiBold)
          .foreground(Foreground)
      })
      .base(Message, |s| {
        s.typography(TypographyStyle::Description)
          .font_size(13.0)
          .line_height(1.5)
          .foreground(MutedForeground)
      })
      .base(Footer, |s| s.gap(8.0))
      .base(Option, |s| s.gap(8.0))
      .base(CheckboxLabel, |s| s.font_size(13.0))
      .base(Actions, |s| s.gap(8.0))
      .base(DefaultButton, |s| s.padding_x(14.0))
      .base(CancelButton, |s| s.padding_x(14.0))
      .base(DestructiveButton, |s| s.padding_x(14.0))
      .base(PlainButton, |s| s.padding_x(14.0));
    let recipe = kind(recipe, MessageBoxKind::Question, Secondary, Foreground);
    let recipe = kind(recipe, MessageBoxKind::Warning, WarningMuted, Caution);
    let recipe = kind(recipe, MessageBoxKind::Error, DestructiveMuted, Danger);
    let recipe = recipe
      .state(MessageBoxState::Destructive, IconWell, |s| {
        s.background(DestructiveMuted)
      })
      .state(MessageBoxState::Destructive, Icon, |s| s.foreground(Danger));
    disclosure_recipe(
      recipe,
      &DisclosureParts {
        trigger: DetailTrigger.into(),
        icon: DetailIcon.into(),
        label: DetailLabel.into(),
        panel: DetailPanel.into(),
        content: Detail.into(),
      },
      MessageBoxState::DetailOpen.into(),
    )
  }
}
