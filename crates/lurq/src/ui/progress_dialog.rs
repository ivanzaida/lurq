//! A progress dialog: a dialog that shows how far a piece of work is, and cannot be dismissed while it runs.

use std::sync::Arc;

use super::{
  button::{Button, ButtonPart, ButtonSize, ButtonVariant},
  dialog::{Dialog, DialogPart},
  message_box::{DisclosureParts, autofocus, disclosure, disclosure_recipe},
  names,
  part_style::{Length, PartStyle},
  progress::{Progress, ProgressFormat, ProgressPart, ProgressSize},
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::glyph,
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    events::KeyboardEvent,
    theme::{PaletteColor, Theme, TypographyStyle},
  },
  components::{Column, Row},
  core::Signal,
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{Element, dimension::Dimension},
};

/// The parts of a progress dialog a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ProgressDialogPart {
  /// What covers the window behind the panel, laid over the dialog's veil.
  Veil,
  /// The panel, laid over the dialog's: its width and its padding.
  Panel,
  /// What the panel holds, one thing under another.
  Content,
  /// The icon beside the title and the message.
  Header,
  /// The icon of the work, before the title.
  Icon,
  /// The title over the message.
  Text,
  /// What is being done: "Copying 128 items".
  Title,
  /// A line more of it: "from Downloads to projects/lurq/assets".
  Message,
  /// The bar, laid over the progress's root.
  Progress,
  /// The row under the bar: how much is done at its start, what the app says of the rest at its end.
  Meta,
  /// How much is done: "42 of 128", or a percentage.
  Count,
  /// What the app says of the rest: "about 2 min left".
  Status,
  /// The row at the bottom: what opens the detail at its start, the buttons at its end.
  Footer,
  /// What opens the detail: an arrow and its label.
  DetailTrigger,
  /// The arrow of the trigger, which points down while the detail is open.
  DetailIcon,
  /// The label of the trigger: "Details".
  DetailLabel,
  /// What opens and closes with the detail.
  DetailPanel,
  /// The detail itself.
  Detail,
  /// The buttons at the end of the footer.
  Actions,
  /// A button of the app's before the cancel one ("Pause"), laid over the button's root.
  ActionButton,
  /// Its label, laid over the button's.
  ActionLabel,
  /// The cancel button, laid over the button's root.
  CancelButton,
  /// Its label, laid over the button's.
  CancelLabel,
}

names!(ProgressDialogPart {
  Veil => "veil",
  Panel => "panel",
  Content => "content",
  Header => "header",
  Icon => "icon",
  Text => "text",
  Title => "title",
  Message => "message",
  Progress => "progress",
  Meta => "meta",
  Count => "count",
  Status => "status",
  Footer => "footer",
  DetailTrigger => "detail_trigger",
  DetailIcon => "detail_icon",
  DetailLabel => "detail_label",
  DetailPanel => "detail_panel",
  Detail => "detail",
  Actions => "actions",
  ActionButton => "action_button",
  ActionLabel => "action_label",
  CancelButton => "cancel_button",
  CancelLabel => "cancel_label",
});

/// The states of a progress dialog a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ProgressDialogState {
  /// How far the work is is not known.
  Indeterminate,
  /// Its detail is open.
  DetailOpen,
}

names!(ProgressDialogState {
  Indeterminate => "indeterminate",
  DetailOpen => "detail_open",
});

/// What builds a progress dialog's nodes: a theme's in place of the default one.
pub type ProgressDialogRenderer = Arc<dyn Fn(ProgressDialogView) -> Element + Send + Sync>;

/// What a button of a progress dialog does when it is pressed.
type Handler = Arc<dyn Fn() + Send + Sync>;

/// A progress dialog: a dialog over the window while a piece of work runs,
/// which says what is being done and how far it is. Copying files,
/// exporting a project, installing an update.
///
/// ```
/// use lurq::{core::Signal, ui::{ProgressDialog, icons}};
///
/// let (open, details) = (Signal::new(true), Signal::new(false));
/// let copying = ProgressDialog::new(open)
///   .icon(icons::COPY)
///   .title("Copying 128 items")
///   .message("from Downloads to projects/lurq/assets")
///   .value(42.0)
///   .max(128.0)
///   .format(|done, all| format!("{done} of {all}"))
///   .status("about 2 min left")
///   .detail("assets/icons/warning.svg", details)
///   .action("Pause", || println!("paused"))
///   .on_cancel(|| println!("cancelled"));
/// ```
///
/// A progress dialog does not keep whether it is open, nor how far the work
/// is: the app's signal and the values it gives do, and it closes when the
/// app sets the signal to `false`. Escape and a press of the veil do not
/// close it. Its cancel button, there when the app gives it
/// [`ProgressDialog::on_cancel`], runs what the app gave it and does not close
/// it: the app closes it once the work has stopped. Escape presses it, and the
/// focus goes to it as the dialog opens. Without a value it shows work whose
/// end is not known. It is a [`Dialog`], with the veil, the focus and Tab of
/// one; to tooling that reads the tree it is called by its title and
/// described by its message.
pub struct ProgressDialog {
  open: Signal<bool>,
  icon: Option<Name>,
  title: Option<Arc<str>>,
  message: Option<Arc<str>>,
  value: Option<f32>,
  max: f32,
  format: Option<ProgressFormat>,
  status: Option<Arc<str>>,
  detail: Option<(Arc<str>, Signal<bool>)>,
  detail_label: Arc<str>,
  actions: Vec<(Arc<str>, Handler)>,
  cancel_label: Arc<str>,
  on_cancel: Option<Handler>,
  inline: bool,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl ProgressDialog {
  /// A progress dialog shown while `open` holds `true`.
  pub fn new(open: Signal<bool>) -> Self {
    Self {
      open,
      icon: None,
      title: None,
      message: None,
      value: None,
      max: 1.0,
      format: None,
      status: None,
      detail: None,
      detail_label: Arc::from("Details"),
      actions: Vec::new(),
      cancel_label: Arc::from("Cancel"),
      on_cancel: None,
      inline: false,
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// An icon of the theme's set before the title: what kind of work it is.
  pub fn icon(mut self, name: impl Into<Name>) -> Self {
    self.icon = Some(name.into());
    self
  }

  /// What is being done: "Copying 128 items". It is also what the dialog is
  /// called to tooling that reads the tree.
  pub fn title(mut self, title: &str) -> Self {
    self.title = Some(Arc::from(title));
    self
  }

  /// A line more of it, under the title: where from, where to, which file.
  pub fn message(mut self, message: &str) -> Self {
    self.message = Some(Arc::from(message));
    self
  }

  /// How far the work is, of the greatest value: a fraction of one unless
  /// [`ProgressDialog::max`] says otherwise. Without one the work's end is
  /// not known.
  pub fn value(mut self, value: impl Into<f64>) -> Self {
    self.value = Some(value.into() as f32);
    self
  }

  /// The greatest value: the whole of the work. 1 when not given.
  pub fn max(mut self, max: impl Into<f64>) -> Self {
    self.max = max.into() as f32;
    self
  }

  /// Says how much is done as the app does, given the value and the
  /// greatest: "42 of 128". A percentage when not given.
  pub fn format(mut self, format: impl Fn(f32, f32) -> String + Send + Sync + 'static) -> Self {
    self.format = Some(Arc::new(format));
    self
  }

  /// What the app says of the rest of the work, under the end of the bar: "about 2 min left".
  pub fn status(mut self, status: &str) -> Self {
    self.status = Some(Arc::from(status));
    self
  }

  /// A text a person opens to see more, under the footer: the file being
  /// copied, a log. It is open while `open` holds `true`, and its trigger at
  /// the start of the footer turns it.
  pub fn detail(mut self, detail: &str, open: Signal<bool>) -> Self {
    self.detail = Some((Arc::from(detail), open));
    self
  }

  /// The label of what opens the detail: "Details" when not given.
  pub fn detail_label(mut self, label: &str) -> Self {
    self.detail_label = Arc::from(label);
    self
  }

  /// A button of the app's before the cancel one: "Pause". It does not close the dialog.
  pub fn action(mut self, label: &str, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.actions.push((Arc::from(label), Arc::new(handler)));
    self
  }

  /// What the cancel button does: the dialog has one only with this. It does
  /// not close the dialog; the app does, once the work has stopped.
  pub fn on_cancel(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_cancel = Some(Arc::new(handler));
    self
  }

  /// The label of the cancel button: "Cancel" when not given.
  pub fn cancel_label(mut self, label: &str) -> Self {
    self.cancel_label = Arc::from(label);
    self
  }

  /// A width of its own, in logical pixels, over the board's 420.
  pub fn width(mut self, width: f32) -> Self {
    self.rules = self.rules.base(ProgressDialogPart::Panel, |s| s.width(width));
    self
  }

  /// Draws the panel where the dialog is declared, whether or not it is
  /// open, without the veil: for a gallery, a screenshot, a test.
  pub fn inline(mut self, inline: bool) -> Self {
    self.inline = inline;
    self
  }

  /// Puts the dialog in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this dialog over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this dialog.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// The id of the panel.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// The key of what the dialog lays over the window.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the dialog in `theme`. Converting a progress dialog into an
  /// element does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let detail_open = self.detail.as_ref().is_some_and(|(_, open)| open.get());
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states,
    }
    .state(ProgressDialogState::Indeterminate, self.value.is_none())
    .state(ProgressDialogState::DetailOpen, detail_open);
    let view = ProgressDialogView {
      icon: self.icon,
      title: self.title,
      message: self.message,
      value: self.value,
      max: self.max,
      status: self.status,
      detail: self.detail.as_ref().map(|(detail, _)| detail.clone()),
      detail_label: self.detail_label,
      detail_open,
      actions: self.actions.iter().map(|(label, _)| label.clone()).collect(),
      cancel_label: self.on_cancel.is_some().then_some(self.cancel_label),
      inline: self.inline,
      recipe: theme.resolve::<ProgressDialog>(&case, Some(&self.rules)),
      open: self.open,
      format: self.format,
      detail_signal: self.detail.map(|(_, open)| open),
      handlers: self.actions.into_iter().map(|(_, handler)| handler).collect(),
      on_cancel: self.on_cancel,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<ProgressDialog>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<ProgressDialog> for Element {
  fn from(dialog: ProgressDialog) -> Self {
    dialog.build(&current_theme())
  }
}

/// A progress dialog as a renderer sees it: what it was given, the style of
/// its parts, and its behavior ready to be dressed.
pub struct ProgressDialogView {
  pub icon: Option<Name>,
  pub title: Option<Arc<str>>,
  pub message: Option<Arc<str>>,
  /// How far the work is; `None` while it is not known.
  pub value: Option<f32>,
  pub max: f32,
  pub status: Option<Arc<str>>,
  pub detail: Option<Arc<str>>,
  pub detail_label: Arc<str>,
  pub detail_open: bool,
  /// The labels of the app's buttons, before the cancel one.
  pub actions: Vec<Arc<str>>,
  /// The label of the cancel button, where there is one.
  pub cancel_label: Option<Arc<str>>,
  pub inline: bool,
  /// The style of each part of this dialog.
  pub recipe: ResolvedRecipe,
  open: Signal<bool>,
  format: Option<ProgressFormat>,
  detail_signal: Option<Signal<bool>>,
  handlers: Vec<Handler>,
  on_cancel: Option<Handler>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl ProgressDialogView {
  /// The style of a part, as this dialog is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The signal that holds whether the dialog is open.
  pub fn open(&self) -> &Signal<bool> {
    &self.open
  }

  /// How much of the work is done, from 0 to 1; `None` while it is not known.
  pub fn fraction(&self) -> Option<f32> {
    let value = self.value?;
    Some(if self.max > 0.0 {
      value.clamp(0.0, self.max) / self.max
    } else {
      0.0
    })
  }

  /// How much is done as it is said: what the app's format says, or a
  /// percentage. `None` while it is not known.
  pub fn count(&self) -> Option<String> {
    let fraction = self.fraction()?;
    Some(match &self.format {
      Some(format) => format(self.value?.clamp(0.0, self.max.max(0.0)), self.max),
      None => format!("{}%", (fraction * 100.0).round() as i32),
    })
  }

  /// The icon, the title and the message.
  pub fn header(&self) -> Row {
    let mut text = Column::new()
      .flex(1.0)
      .align_items(Alignment::Stretch)
      .part(&self.part(ProgressDialogPart::Text));
    for (part, content) in [
      (ProgressDialogPart::Title, &self.title),
      (ProgressDialogPart::Message, &self.message),
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
    let mut header = Row::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Center)
      .part(&self.part(ProgressDialogPart::Header));
    if let Some(icon) = self.icon {
      header = header.child(glyph(&self.part(ProgressDialogPart::Icon), icon, false));
    }
    header.child(text)
  }

  /// The bar: a [`Progress`] with the dialog's rules for it laid over the theme's.
  pub fn progress(&self) -> Element {
    let progress = match self.value {
      Some(value) => Progress::new(value).max(self.max),
      None => Progress::indeterminate(),
    };
    progress
      .size(ProgressSize::Md)
      .rules(&self.recipe.rules_for(ProgressDialogPart::Progress, ProgressPart::Root))
      .build(self.theme())
  }

  /// How much is done at the start and what the app says of the rest at the end: `None` where there is neither.
  pub fn meta(&self) -> Option<Row> {
    let count = self.count();
    if count.is_none() && self.status.is_none() {
      return None;
    }
    let text = |part: ProgressDialogPart, content: &str| {
      let part = self.part(part);
      Row::new().part(&part).child(part.text(content).nowrap())
    };
    // What holds the count takes the room there is, so that the status stands at the end with no count too.
    let mut start = Row::new().flex(1.0);
    if let Some(count) = &count {
      start = start.child(text(ProgressDialogPart::Count, count));
    }
    let mut meta = Row::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Center)
      .part(&self.part(ProgressDialogPart::Meta))
      .child(start);
    if let Some(status) = &self.status {
      meta = meta.child(text(ProgressDialogPart::Status, status));
    }
    Some(meta)
  }

  /// What the cancel button and Escape do: what the app gave [`ProgressDialog::on_cancel`].
  pub fn canceller(&self) -> Option<impl Fn() + Send + Sync + Clone + 'static> {
    let handler = self.on_cancel.clone()?;
    Some(move || handler())
  }

  /// A button of the dialog's, with the dialog's rules for `parts` laid over the theme's for a button.
  fn button(&self, label: &str, parts: (ProgressDialogPart, ProgressDialogPart), handler: Handler) -> Element {
    let rules = self
      .recipe
      .rules_for(parts.0, ButtonPart::Root)
      .extend(&self.recipe.rules_for(parts.1, ButtonPart::Label));
    Button::new(label)
      .variant(ButtonVariant::Outline)
      .size(ButtonSize::Sm)
      .rules(&rules)
      .on_click(move |_| handler())
      .build(self.theme())
  }

  /// The app's buttons and the cancel one, which the focus goes to as the dialog opens.
  pub fn actions(&self) -> Row {
    let mut actions = Row::new()
      .align_items(Alignment::Center)
      .overflow_visible()
      .part(&self.part(ProgressDialogPart::Actions));
    for (label, handler) in self.actions.iter().zip(&self.handlers) {
      actions = actions.child(self.button(
        label,
        (ProgressDialogPart::ActionButton, ProgressDialogPart::ActionLabel),
        handler.clone(),
      ));
    }
    if let (Some(label), Some(handler)) = (&self.cancel_label, &self.on_cancel) {
      actions = actions.child(autofocus(self.button(
        label,
        (ProgressDialogPart::CancelButton, ProgressDialogPart::CancelLabel),
        handler.clone(),
      )));
    }
    actions
  }

  /// What opens the detail, and what opens and closes with it: `None` for a dialog with no detail.
  pub fn detail(&self) -> Option<(crate::components::Button, Column)> {
    let (detail, open) = (self.detail.as_ref()?, self.detail_signal.as_ref()?);
    Some(disclosure(
      &self.recipe,
      detail_parts(),
      &self.detail_label,
      detail,
      open,
    ))
  }

  /// The footer: what opens the detail at its start, the buttons at its end. `None` where there are neither.
  pub fn footer(&self, trigger: Option<crate::components::Button>) -> Option<Row> {
    let actions = (!self.actions.is_empty() || self.cancel_label.is_some()).then(|| self.actions());
    if trigger.is_none() && actions.is_none() {
      return None;
    }
    let mut footer = Row::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Center)
      .justify(if trigger.is_some() {
        Justify::SpaceBetween
      } else {
        Justify::End
      })
      .overflow_visible()
      .part(&self.part(ProgressDialogPart::Footer));
    if let Some(trigger) = trigger {
      footer = footer.child(trigger);
    }
    if let Some(actions) = actions {
      footer = footer.child(actions);
    }
    Some(footer)
  }

  /// What the panel holds, with nothing in it yet: Escape with the focus in it presses the cancel button.
  pub fn content(&self) -> Column {
    let mut content = Column::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Stretch)
      .overflow_visible()
      .part(&self.part(ProgressDialogPart::Content));
    if let Some(cancel) = self.canceller() {
      content = content.on_key_down(move |event: KeyboardEvent| {
        if event.focus_within && event.key == "Escape" && !event.default_prevented() {
          event.prevent_default();
          cancel();
        }
      });
    }
    content
  }

  /// `content` in a [`Dialog`] that neither Escape nor a press of its veil
  /// closes, its panel and its veil in the dialog's rules over the dialog's.
  pub fn dialog(&self, content: impl Into<Element>) -> Element {
    let rules = self
      .recipe
      .rules_for(ProgressDialogPart::Panel, DialogPart::Panel)
      .extend(&self.recipe.rules_for(ProgressDialogPart::Veil, DialogPart::Veil));
    let mut dialog = Dialog::new(self.open.clone())
      .rules(&rules)
      .inline(self.inline)
      .dismiss_on_escape(false)
      .dismiss_on_veil(false)
      .child(content);
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

  /// How the default renderer builds a progress dialog: the header, the bar,
  /// what is done, the footer and the detail under it, in a dialog.
  pub fn render(self) -> Element {
    let mut content = self.content().child(self.header()).child(self.progress());
    if let Some(meta) = self.meta() {
      content = content.child(meta);
    }
    let (trigger, panel) = match self.detail() {
      Some((trigger, panel)) => (Some(trigger), Some(panel)),
      None => (None, None),
    };
    // The detail opens under the footer, and is in one column with it, so that nothing of the content's gap is
    // left under the footer while the detail is closed.
    let mut bottom = Column::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Stretch)
      .overflow_visible();
    if let Some(footer) = self.footer(trigger) {
      bottom = bottom.child(footer);
    }
    if let Some(panel) = panel {
      bottom = bottom.child(panel);
    }
    self.dialog(content.child(bottom))
  }
}

/// The parts of the detail of a progress dialog.
fn detail_parts() -> DisclosureParts {
  use ProgressDialogPart::*;
  DisclosureParts {
    trigger: DetailTrigger.into(),
    icon: DetailIcon.into(),
    label: DetailLabel.into(),
    panel: DetailPanel.into(),
    content: Detail.into(),
  }
}

impl Themed for ProgressDialog {
  const NAME: &'static str = "progress_dialog";
  type Renderer = ProgressDialogRenderer;

  /// The design's progress dialog (`pencil/lurq-design-system.pen`, Progress
  /// Dialog): a dialog's panel 420 wide, padded 20 with 12 between its parts.
  /// The header is the icon of 20, 12 before the title of 15/600 over the
  /// message of 12 in the muted color, 2 apart; under it the bar of 6 across
  /// the panel, and under that what is done and what is left in the mono type
  /// of 11 in the muted color. The footer, 4 further down, has "Details" at
  /// its start and the outlined buttons of 32 padded 14 at its end, 8 apart.
  fn recipe() -> Recipe {
    use PaletteColor::{Foreground, MutedForeground};
    use ProgressDialogPart::*;
    let recipe = Recipe::new()
      // Of the 20 the panel is padded, 4 are the dialog's body's, which is room for the ring of a focused button.
      .base(Panel, |s| s.width(420.0).padding_x(16.0).padding_y(16.0))
      .base(Content, |s| s.gap(12.0))
      .base(Header, |s| s.gap(12.0))
      .base(Icon, |s| s.square(20.0).foreground(Foreground))
      .base(Text, |s| s.gap(2.0))
      .base(Title, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(15.0)
          .weight(FontWeight::SemiBold)
          .foreground(Foreground)
      })
      .base(Message, |s| {
        s.typography(TypographyStyle::Description)
          .font_size(12.0)
          .foreground(MutedForeground)
      })
      .base(Progress, |s| s.width(Length::Fill))
      .base(Meta, |s| s.gap(8.0))
      .base(Count, |s| {
        s.typography(TypographyStyle::Mono)
          .font_size(11.0)
          .foreground(MutedForeground)
      })
      .base(Status, |s| {
        s.typography(TypographyStyle::Mono)
          .font_size(11.0)
          .foreground(MutedForeground)
      })
      .base(Footer, |s| s.gap(8.0).padding_top(4.0))
      .base(Actions, |s| s.gap(8.0))
      .base(ActionButton, |s| s.padding_x(14.0))
      .base(CancelButton, |s| s.padding_x(14.0));
    disclosure_recipe(recipe, &detail_parts(), ProgressDialogState::DetailOpen.into())
  }
}
