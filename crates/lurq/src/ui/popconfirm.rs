//! A popconfirm: a question in a small panel hung from what was pressed, and two buttons that answer it.

use std::sync::Arc;

use super::{
  button::{Button, ButtonPart, ButtonSize, ButtonVariant},
  icons,
  message_box::{autofocus, wrapping},
  names,
  part_style::{Length, PartStyle},
  popover::{Popover, PopoverPart},
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::{IconChoice, glyph, shown_icon},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    ctx::{CollisionStrategy, Placement},
    events::KeyboardEvent,
    theme::{PaletteColor, Theme, TypographyStyle},
  },
  components::{Column, Row},
  core::{ElementRef, Signal},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{Element, dimension::Dimension},
};

/// The parts of a popconfirm a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PopconfirmPart {
  /// The panel, laid over the popover's: its width and its padding.
  Panel,
  /// What the panel holds, laid over the popover's body.
  Body,
  /// What the panel holds: the header over the footer.
  Content,
  /// The row of the icon and the text, at the top.
  Header,
  /// The icon before the title.
  Icon,
  /// The column of the title over the description.
  Text,
  /// The question, in a line.
  Title,
  /// More of it, under the title.
  Description,
  /// The row of the two buttons, at the end.
  Footer,
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
  Header => "header",
  Icon => "icon",
  Text => "text",
  Title => "title",
  Description => "description",
  Footer => "footer",
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
/// let delete = Popconfirm::new(open, anchor)
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
/// goes to the confirm button, or to the cancel button where what it
/// confirms is destructive, so that Enter or Space does not remove what was
/// not read. Escape and a press outside cancel, as the cancel button does,
/// and then run [`Popconfirm::on_dismiss`]. To tooling that reads the tree
/// it is an `alertdialog`, called by its title and described by its
/// description.
pub struct Popconfirm {
  open: Signal<bool>,
  anchor: ElementRef,
  trigger: Option<Element>,
  icon: Option<IconChoice>,
  title: Option<Arc<str>>,
  description: Option<Arc<str>>,
  confirm: (Arc<str>, Option<Handler>),
  cancel: (Arc<str>, Option<Handler>),
  destructive: bool,
  placement: Placement,
  offset: Option<(f32, f32)>,
  collision: Option<CollisionStrategy>,
  width: Option<Length>,
  label: Option<Arc<str>>,
  inline: bool,
  dismiss_on_escape: bool,
  dismiss_on_outside_press: bool,
  on_dismiss: Vec<Handler>,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Popconfirm {
  /// A popconfirm hung from `anchor`, shown while `open` is `true`.
  pub fn new(open: Signal<bool>, anchor: impl Into<ElementRef>) -> Self {
    Self {
      open,
      anchor: anchor.into(),
      trigger: None,
      icon: None,
      title: None,
      description: None,
      confirm: (Arc::from("Yes"), None),
      cancel: (Arc::from("No"), None),
      destructive: false,
      placement: Placement::BottomStart,
      offset: None,
      collision: None,
      width: None,
      label: None,
      inline: false,
      dismiss_on_escape: true,
      dismiss_on_outside_press: true,
      on_dismiss: Vec::new(),
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

  /// The icon before the title: one of the theme's set in place of the
  /// warning sign, or `None` for no icon.
  pub fn icon(mut self, icon: impl Into<IconChoice>) -> Self {
    self.icon = Some(icon.into());
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
  /// does after it closes the popconfirm. Escape and a press outside run it too.
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

  /// How far it is moved from where its placement puts it, across (`x`) and
  /// down (`y`), as [`Popover::offset`]: the popover's by default.
  pub fn offset(mut self, x: f32, y: f32) -> Self {
    self.offset = Some((x, y));
    self
  }

  /// What it does where there is no room for it on its side, as
  /// [`Popover::collision`]: the popover's by default.
  pub fn collision(mut self, collision: CollisionStrategy) -> Self {
    self.collision = Some(collision);
    self
  }

  /// How wide the panel is, in place of what the theme gives it.
  pub fn width(mut self, width: impl Into<Length>) -> Self {
    self.width = Some(width.into());
    self
  }

  /// What the popconfirm is called to tooling that reads the tree, in place
  /// of its title.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// What runs after a person closed the popconfirm by Escape or a press
  /// outside it, after what the cancel button runs; not on a press of either
  /// button, and not when the app sets the signal. A second handler is
  /// called after the first.
  pub fn on_dismiss(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_dismiss.push(Arc::new(handler));
    self
  }

  /// Whether Escape closes the popconfirm. `true` by default.
  pub fn dismiss_on_escape(mut self, dismiss: bool) -> Self {
    self.dismiss_on_escape = dismiss;
    self
  }

  /// Whether a press outside the popconfirm and its anchor closes it. `true` by default.
  pub fn dismiss_on_outside_press(mut self, dismiss: bool) -> Self {
    self.dismiss_on_outside_press = dismiss;
    self
  }

  /// Draws the panel where the popconfirm is declared, whether or not it is
  /// open, and not over the window beside its anchor: for a gallery, a
  /// screenshot, a test. Its trigger is not drawn; its buttons still answer
  /// and set the signal to `false`.
  pub fn inline(mut self, inline: bool) -> Self {
    self.inline = inline;
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
    // A width given here is under what `style` and `rules` say of the panel.
    let mut rules = Recipe::new();
    if let Some(width) = self.width {
      rules = rules.base(PopconfirmPart::Panel, |s| s.width(width));
    }
    let rules = rules.extend(&self.rules);
    let recipe = theme.resolve::<Popconfirm>(&case, Some(&rules));
    let rules = recipe
      .rules_for(PopconfirmPart::Panel, PopoverPart::Panel)
      .extend(&recipe.rules_for(PopconfirmPart::Body, PopoverPart::Body));
    let view = PopconfirmView {
      icon: shown_icon(self.icon, icons::WARNING),
      title: self.title.clone(),
      description: self.description.clone(),
      confirm_label: self.confirm.0,
      cancel_label: self.cancel.0,
      destructive: self.destructive,
      recipe,
      open: self.open.clone(),
      on_confirm: self.confirm.1,
      on_cancel: self.cancel.1.clone(),
      on_dismiss: self.on_dismiss.clone(),
      dismiss_on_escape: self.dismiss_on_escape,
    };
    let content = match theme.renderer::<Popconfirm>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    };
    let mut popover = Popover::new(self.open, self.anchor)
      .alert(true)
      .placement(self.placement)
      .inline(self.inline)
      .dismiss_on_escape(self.dismiss_on_escape)
      .dismiss_on_outside_press(self.dismiss_on_outside_press)
      .rules(&rules)
      .child(content);
    // A press outside, and Escape where the panel does not have the focus, cancel as the cancel button does, and
    // then run what `on_dismiss` was given. The popover has closed by then.
    let (on_cancel, on_dismiss) = (self.cancel.1, self.on_dismiss);
    if on_cancel.is_some() || !on_dismiss.is_empty() {
      popover = popover.on_dismiss(move || dismissed(on_cancel.as_ref(), &on_dismiss));
    }
    if let Some(trigger) = self.trigger {
      popover = popover.trigger(trigger);
    }
    if let Some((x, y)) = self.offset {
      popover = popover.offset(x, y);
    }
    if let Some(collision) = self.collision {
      popover = popover.collision(collision);
    }
    if let Some(name) = self.label.as_ref().or(self.title.as_ref()) {
      popover = popover.label(name);
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
  /// The icon before the title: the warning sign, the one it was given, or none.
  pub icon: Option<Name>,
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
  on_dismiss: Vec<Handler>,
  dismiss_on_escape: bool,
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

  /// What a press of the cancel button does: the popconfirm closes, and what the app gave it runs.
  pub fn canceller(&self) -> impl Fn() + Send + Sync + Clone + 'static {
    answer(self.open.clone(), self.on_cancel.clone())
  }

  /// What Escape and a press outside do: the popconfirm closes, what the cancel button runs runs, and then what
  /// [`Popconfirm::on_dismiss`] was given.
  pub fn dismisser(&self) -> impl Fn() + Send + Sync + Clone + 'static {
    let (open, on_cancel, on_dismiss) = (self.open.clone(), self.on_cancel.clone(), self.on_dismiss.clone());
    move || {
      open.set(false);
      dismissed(on_cancel.as_ref(), &on_dismiss);
    }
  }

  /// Whether the focus goes to the confirm button as it opens: unless what it confirms is destructive, when it
  /// goes to the cancel button.
  fn confirm_takes_focus(&self) -> bool {
    !self.destructive
  }

  /// The row of the icon and the title over the description.
  pub fn header(&self) -> Row {
    let mut text = Column::new()
      .flex(1.0)
      .align_items(Alignment::Stretch)
      .part(&self.part(PopconfirmPart::Text));
    for (part, content) in [
      (PopconfirmPart::Title, &self.title),
      (PopconfirmPart::Description, &self.description),
    ] {
      if let Some(content) = content {
        text = text.child(wrapping(&self.part(part), content));
      }
    }
    let mut header = Row::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Start)
      .part(&self.part(PopconfirmPart::Header));
    if let Some(icon) = self.icon {
      header = header.child(glyph(&self.part(PopconfirmPart::Icon), icon, false));
    }
    header.child(text)
  }

  /// The cancel button: an outlined one, which the focus goes to as the popconfirm opens where what it confirms
  /// is destructive.
  pub fn cancel(&self) -> Element {
    let rules = self
      .recipe
      .rules_for(PopconfirmPart::CancelButton, ButtonPart::Root)
      .extend(&self.recipe.rules_for(PopconfirmPart::CancelLabel, ButtonPart::Label));
    let cancel = self.canceller();
    let button = Button::new(&self.cancel_label)
      .variant(ButtonVariant::Outline)
      .size(ButtonSize::Xs)
      .rules(&rules)
      .on_click(move |_| cancel())
      .build(self.theme());
    if self.confirm_takes_focus() {
      button
    } else {
      autofocus(button)
    }
  }

  /// The confirm button: a primary one, which the focus goes to as the popconfirm opens, or a destructive one.
  pub fn confirm(&self) -> Element {
    let rules = self
      .recipe
      .rules_for(PopconfirmPart::ConfirmButton, ButtonPart::Root)
      .extend(&self.recipe.rules_for(PopconfirmPart::ConfirmLabel, ButtonPart::Label));
    let confirm = self.confirmer();
    let button = Button::new(&self.confirm_label)
      .variant(if self.destructive {
        ButtonVariant::Destructive
      } else {
        ButtonVariant::Primary
      })
      .size(ButtonSize::Xs)
      .rules(&rules)
      .on_click(move |_| confirm())
      .build(self.theme());
    if self.confirm_takes_focus() {
      autofocus(button)
    } else {
      button
    }
  }

  /// The row of the two buttons at the end, cancel first. Where they do not
  /// fit side by side, the confirm button goes under the cancel button.
  pub fn footer(&self) -> Row {
    Row::new()
      .width(Dimension::Pct(100.0))
      .wrap()
      .justify(Justify::End)
      .align_items(Alignment::Center)
      .overflow_visible()
      .part(&self.part(PopconfirmPart::Footer))
      .child(self.cancel())
      .child(self.confirm())
  }

  /// What the panel holds, with nothing in it yet: Escape with the focus in it cancels, unless the popconfirm was
  /// told not to close on Escape.
  pub fn content(&self) -> Column {
    let content = Column::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Stretch)
      .overflow_visible()
      .part(&self.part(PopconfirmPart::Content));
    if !self.dismiss_on_escape {
      return content;
    }
    let dismiss = self.dismisser();
    content.on_key_down(move |event: KeyboardEvent| {
      if event.focus_within && event.key == "Escape" && !event.default_prevented() {
        event.prevent_default();
        dismiss();
      }
    })
  }

  /// How the default renderer builds what the panel holds: the header over the footer.
  pub fn render(self) -> Element {
    self.content().child(self.header()).child(self.footer()).into()
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

/// What runs once a person closed the popconfirm by Escape or a press outside: the cancel button's handler, then
/// each of `on_dismiss`.
fn dismissed(on_cancel: Option<&Handler>, on_dismiss: &[Handler]) {
  if let Some(on_cancel) = on_cancel {
    on_cancel();
  }
  for handler in on_dismiss {
    handler();
  }
}

impl Themed for Popconfirm {
  const NAME: &'static str = "popconfirm";
  type Renderer = PopconfirmRenderer;

  /// The design's popconfirm (`pencil/lurq-design-system.pen`, Popconfirm):
  /// a popover's panel 250 wide, padded 12 with 12 between its header and
  /// its buttons. The header is the warning icon of 16 in the warning color,
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
      .base(Header, |s| s.gap(8.0))
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
      .base(Footer, |s| s.gap(8.0))
      .base(CancelLabel, |s| s.font_size(13.0))
      .base(ConfirmLabel, |s| s.font_size(13.0))
      .state(PopconfirmState::Destructive, Icon, |s| s.foreground(Destructive))
  }
}
