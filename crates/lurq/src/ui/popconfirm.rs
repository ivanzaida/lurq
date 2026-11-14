//! A popconfirm: a question in a small panel hung from what was pressed, and two buttons that answer it.

use std::sync::Arc;

use super::{
  button::{Button, ButtonPart, ButtonSize, ButtonVariant},
  icons,
  message_box::autofocus,
  names,
  part_style::PartStyle,
  popover::{Popover, PopoverPart},
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::glyph,
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    ctx::Placement,
    events::KeyboardEvent,
    theme::{PaletteColor, Theme, TypographyStyle},
  },
  components::{Column, Row},
  core::{ElementRef, Signal},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{Element, SpacingValue, dimension::Dimension},
};

/// The parts of a popconfirm a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PopconfirmPart {
  /// The panel, laid over the popover's: its width and its padding.
  Panel,
  /// What the panel holds, laid over the popover's body.
  Body,
  /// What the panel holds: the message over the actions.
  Content,
  /// The icon beside the text.
  Message,
  /// The icon before the title.
  Icon,
  /// The title over the description.
  Text,
  /// The question, in a line.
  Title,
  /// More of it, under the title.
  Description,
  /// The two buttons at the end.
  Actions,
  /// The button that cancels, laid over the button's root.
  CancelButton,
  /// Its label, laid over the button's.
  CancelLabel,
  /// The button that confirms, laid over the button's root.
  ConfirmButton,
  /// Its label, laid over the button's.
  ConfirmLabel,
}

names!(PopconfirmPart {
  Panel => "panel",
  Body => "body",
  Content => "content",
  Message => "message",
  Icon => "icon",
  Text => "text",
  Title => "title",
  Description => "description",
  Actions => "actions",
  CancelButton => "cancel_button",
  CancelLabel => "cancel_label",
  ConfirmButton => "confirm_button",
  ConfirmLabel => "confirm_label",
});

/// The states of a popconfirm a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PopconfirmState {
  /// What it confirms removes something or cannot be undone: the confirm
  /// button and the icon are in the destructive color.
  Destructive,
}

names!(PopconfirmState {
  Destructive => "destructive",
});

/// What builds what a popconfirm's panel holds: a theme's in place of the
/// default one. Where the panel is shown, and when, stays the popover's.
pub type PopconfirmRenderer = Arc<dyn Fn(PopconfirmView) -> Element + Send + Sync>;

/// What a button of a popconfirm does when it is pressed.
type Handler = Arc<dyn Fn() + Send + Sync>;

/// A popconfirm: a question in a small panel hung from what was pressed,
/// answered by one of two buttons. Delete this file? Leave the channel?
///
/// ```
/// use lurq::{
///   core::{ElementRef, Signal},
///   ui::{Button, ButtonVariant, Popconfirm},
/// };
///
/// let (anchor, open) = (ElementRef::new(), Signal::new(false));
/// let delete = Popconfirm::new(anchor, open)
///   .trigger(Button::new("Delete").variant(ButtonVariant::Outline))
///   .title("Delete this file?")
///   .description("It will be moved to the trash.")
///   .destructive(true)
///   .confirm("Delete", || println!("deleted"))
///   .cancel("Keep", || {});
/// ```
///
/// A popconfirm is a [`Popover`]: it does not keep whether it is open, its
/// trigger turns the app's signal, and it is kept in the window. Either
/// button closes it and then runs what the app gave it. As it opens the focus
/// goes to the cancel button, so that Enter or Space does not confirm what
/// was not read. Escape with the focus in it cancels, as the cancel button
/// does; a press outside closes it, confirming nothing. To tooling that reads
/// the tree it is an `alertdialog`, called by its title and described by its
/// description.
pub struct Popconfirm {
  anchor: ElementRef,
  open: Signal<bool>,
  trigger: Option<Element>,
  icon: Name,
  title: Option<Arc<str>>,
  description: Option<Arc<str>>,
  confirm: (Arc<str>, Option<Handler>),
  cancel: (Arc<str>, Option<Handler>),
  destructive: bool,
  placement: Placement,
  offset: Option<SpacingValue>,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Popconfirm {
  /// A popconfirm hung from `anchor`, shown while `open` is `true`.
  pub fn new(anchor: impl Into<ElementRef>, open: Signal<bool>) -> Self {
    Self {
      anchor: anchor.into(),
      open,
      trigger: None,
      icon: icons::WARNING.into(),
      title: None,
      description: None,
      confirm: (Arc::from("Yes"), None),
      cancel: (Arc::from("No"), None),
      destructive: false,
      placement: Placement::BottomStart,
      offset: None,
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// What is pressed to ask: a button. It is put before the popconfirm, is
  /// its anchor, and a press of it turns the signal.
  pub fn trigger(mut self, trigger: impl Into<Element>) -> Self {
    self.trigger = Some(trigger.into());
    self
  }

  /// An icon of the theme's set before the title, in place of the warning sign.
  pub fn icon(mut self, name: impl Into<Name>) -> Self {
    self.icon = name.into();
    self
  }

  /// The question: "Delete this file?".
  pub fn title(mut self, title: &str) -> Self {
    self.title = Some(Arc::from(title));
    self
  }

  /// More of it, under the title.
  pub fn description(mut self, description: &str) -> Self {
    self.description = Some(Arc::from(description));
    self
  }

  /// The button that confirms: its label ("Yes" when not given) and what it
  /// does after it closes the popconfirm.
  pub fn confirm(mut self, label: &str, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.confirm = (Arc::from(label), Some(Arc::new(handler)));
    self
  }

  /// The button that cancels: its label ("No" when not given) and what it
  /// does after it closes the popconfirm. Escape runs it too.
  pub fn cancel(mut self, label: &str, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.cancel = (Arc::from(label), Some(Arc::new(handler)));
    self
  }

  /// Whether what it confirms removes something or cannot be undone: the
  /// confirm button is destructive.
  pub fn destructive(mut self, destructive: bool) -> Self {
    self.destructive = destructive;
    self
  }

  /// Which side of the anchor it is on: under it at its start by default, as a popover is.
  pub fn placement(mut self, placement: Placement) -> Self {
    self.placement = placement;
    self
  }

  /// How far it is from its anchor: the popover's by default.
  pub fn offset(mut self, offset: impl Into<SpacingValue>) -> Self {
    self.offset = Some(offset.into());
    self
  }

  /// Puts the popconfirm in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this popconfirm over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this popconfirm.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// The id of the panel.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the popconfirm in `theme`. Converting a popconfirm into an
  /// element does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states,
    }
    .state(PopconfirmState::Destructive, self.destructive);
    let recipe = theme.resolve::<Popconfirm>(&case, Some(&self.rules));
    let rules = recipe
      .rules_for(PopconfirmPart::Panel, PopoverPart::Root)
      .extend(&recipe.rules_for(PopconfirmPart::Body, PopoverPart::Body));
    let view = PopconfirmView {
      icon: self.icon,
      title: self.title.clone(),
      description: self.description.clone(),
      confirm_label: self.confirm.0,
      cancel_label: self.cancel.0,
      destructive: self.destructive,
      recipe,
      open: self.open.clone(),
      on_confirm: self.confirm.1,
      on_cancel: self.cancel.1,
    };
    let content = match theme.renderer::<Popconfirm>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    };
    let mut popover = Popover::new(self.anchor, self.open)
      .alert(true)
      .placement(self.placement)
      .rules(&rules)
      .child(content);
    if let Some(trigger) = self.trigger {
      popover = popover.trigger(trigger);
    }
    if let Some(offset) = self.offset {
      popover = popover.offset(offset);
    }
    if let Some(title) = &self.title {
      popover = popover.named(title);
    }
    if let Some(description) = &self.description {
      popover = popover.described(description);
    }
    if let Some(id) = self.id {
      popover = popover.id(id);
    }
    if let Some(key) = self.key {
      popover = popover.key(key);
    }
    popover.build(theme)
  }
}

impl From<Popconfirm> for Element {
  fn from(popconfirm: Popconfirm) -> Self {
    popconfirm.build(&current_theme())
  }
}

/// What a popconfirm's panel holds as a renderer sees it: what it was given,
/// the style of its parts, and its behavior ready to be dressed.
pub struct PopconfirmView {
  pub icon: Name,
  pub title: Option<Arc<str>>,
  pub description: Option<Arc<str>>,
  pub confirm_label: Arc<str>,
  pub cancel_label: Arc<str>,
  pub destructive: bool,
  /// The style of each part of this popconfirm.
  pub recipe: ResolvedRecipe,
  open: Signal<bool>,
  on_confirm: Option<Handler>,
  on_cancel: Option<Handler>,
}

impl PopconfirmView {
  /// The style of a part, as this popconfirm is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// What a press of the confirm button does: the popconfirm closes, and what the app gave it runs.
  pub fn confirmer(&self) -> impl Fn() + Send + Sync + Clone + 'static {
    answer(self.open.clone(), self.on_confirm.clone())
  }

  /// What a press of the cancel button and Escape do: the popconfirm closes, and what the app gave it runs.
  pub fn canceller(&self) -> impl Fn() + Send + Sync + Clone + 'static {
    answer(self.open.clone(), self.on_cancel.clone())
  }

  /// The icon and the title over the description.
  pub fn message(&self) -> Row {
    let mut text = Column::new()
      .flex(1.0)
      .align_items(Alignment::Stretch)
      .part(&self.part(PopconfirmPart::Text));
    for (part, content) in [
      (PopconfirmPart::Title, &self.title),
      (PopconfirmPart::Description, &self.description),
    ] {
      if let Some(content) = content {
        let part = self.part(part);
        text = text.child(
          Row::new()
            .width(Dimension::Pct(100.0))
            .part(&part)
            .child(part.text(content)),
        );
      }
    }
    Row::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Start)
      .part(&self.part(PopconfirmPart::Message))
      .child(glyph(&self.part(PopconfirmPart::Icon), self.icon, false))
      .child(text)
  }

  /// The cancel button: an outlined one, which the focus goes to as the popconfirm opens.
  pub fn cancel(&self) -> Element {
    let rules = self
      .recipe
      .rules_for(PopconfirmPart::CancelButton, ButtonPart::Root)
      .extend(&self.recipe.rules_for(PopconfirmPart::CancelLabel, ButtonPart::Label));
    let cancel = self.canceller();
    autofocus(
      Button::new(&self.cancel_label)
        .variant(ButtonVariant::Outline)
        .size(ButtonSize::Xs)
        .rules(&rules)
        .on_click(move |_| cancel())
        .build(self.theme()),
    )
  }

  /// The confirm button: a primary one, or a destructive one.
  pub fn confirm(&self) -> Element {
    let rules = self
      .recipe
      .rules_for(PopconfirmPart::ConfirmButton, ButtonPart::Root)
      .extend(&self.recipe.rules_for(PopconfirmPart::ConfirmLabel, ButtonPart::Label));
    let confirm = self.confirmer();
    Button::new(&self.confirm_label)
      .variant(if self.destructive {
        ButtonVariant::Destructive
      } else {
        ButtonVariant::Primary
      })
      .size(ButtonSize::Xs)
      .rules(&rules)
      .on_click(move |_| confirm())
      .build(self.theme())
  }

  /// The two buttons at the end, cancel first.
  pub fn actions(&self) -> Row {
    Row::new()
      .width(Dimension::Pct(100.0))
      .justify(Justify::End)
      .align_items(Alignment::Center)
      .overflow_visible()
      .part(&self.part(PopconfirmPart::Actions))
      .child(self.cancel())
      .child(self.confirm())
  }

  /// What the panel holds, with nothing in it yet: Escape with the focus in it cancels.
  pub fn content(&self) -> Column {
    let cancel = self.canceller();
    Column::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Stretch)
      .overflow_visible()
      .part(&self.part(PopconfirmPart::Content))
      .on_key_down(move |event: KeyboardEvent| {
        if event.focus_within && event.key == "Escape" && !event.default_prevented() {
          event.prevent_default();
          cancel();
        }
      })
  }

  /// How the default renderer builds what the panel holds: the message over the actions.
  pub fn render(self) -> Element {
    self.content().child(self.message()).child(self.actions()).into()
  }
}

/// An answer: `open` set to `false`, then `handler`.
fn answer(open: Signal<bool>, handler: Option<Handler>) -> impl Fn() + Send + Sync + Clone + 'static {
  move || {
    open.set(false);
    if let Some(handler) = &handler {
      handler();
    }
  }
}

impl Themed for Popconfirm {
  const NAME: &'static str = "popconfirm";
  type Renderer = PopconfirmRenderer;

  /// The design's popconfirm (`pencil/lurq-design-system.pen`, Popconfirm):
  /// a popover's panel 250 wide, padded 12 with 12 between its message and
  /// its buttons. The message is the warning icon of 16 in the warning color,
  /// 8 before the title of 13/500 over the description of 12 in the muted
  /// color, 2 apart. The buttons are 28 high, padded 10, their labels 13, 8
  /// apart at the end. A destructive one has its icon in the destructive color.
  fn recipe() -> Recipe {
    use PaletteColor::{Destructive, Foreground, MutedForeground, Warning};
    use PopconfirmPart::*;
    Recipe::new()
      // Of the 12 the panel is padded, 4 are the body's: room for the ring of a focused button, which the body
      // would clip.
      .base(Panel, |s| s.width(250.0).padding_x(8.0).padding_y(8.0))
      .base(Body, |s| s.padding_x(4.0).padding_y(4.0))
      .base(Content, |s| s.gap(12.0))
      .base(Message, |s| s.gap(8.0))
      .base(Icon, |s| s.square(16.0).foreground(Warning))
      .base(Text, |s| s.gap(2.0))
      .base(Title, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(13.0)
          .weight(FontWeight::Medium)
          .foreground(Foreground)
      })
      .base(Description, |s| {
        s.typography(TypographyStyle::Description)
          .font_size(12.0)
          .foreground(MutedForeground)
      })
      .base(Actions, |s| s.gap(8.0))
      .base(CancelLabel, |s| s.font_size(13.0))
      .base(ConfirmLabel, |s| s.font_size(13.0))
      .state(PopconfirmState::Destructive, Icon, |s| s.foreground(Destructive))
  }
}
