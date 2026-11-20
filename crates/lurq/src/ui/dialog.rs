//! A dialog: a panel over a veil that covers the window, which nothing under
//! it gets past until it is closed.

use std::sync::Arc;

use super::{
  alert::wrapping,
  button::{Button, ButtonPart, ButtonSize, ButtonVariant},
  icons, names,
  part_style::Length,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::glyph,
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    events::{KeyboardEvent, MouseEvent},
    theme::{BorderSize, IconSize, PaletteColor, RadiusSize, ShadowStyle, Theme, TypographyStyle},
  },
  components::{Column, Modal, Root, Row, ScrollVertical},
  core::Signal,
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{Element, border::Border as Edge, dimension::Dimension},
};

/// The parts of a dialog a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DialogPart {
  /// What covers the window behind the panel, and holds the panel in its middle.
  Veil,
  /// The dialog itself: its edge, its corners, its shadow.
  Panel,
  /// The top of the panel: the text, and the close button at its end.
  Header,
  /// The title and the description, one over the other.
  Text,
  /// What the dialog asks or says, in a word or a line.
  Title,
  /// More of it, under the title.
  Description,
  /// What the app puts in the dialog: a form, a list, a few paragraphs.
  Body,
  /// The row of buttons at the bottom, at its end.
  Footer,
  /// The button that closes the dialog, in the header: an icon alone.
  Close,
  /// The line between the header and the body, and between the body and the
  /// footer, of a dialog whose body scrolls.
  Separator,
  /// The well before the title that holds the dialog's icon, or the app's element in its place.
  Mark,
  /// The icon in the mark.
  Icon,
  /// Quiet words at the start of the footer, before its buttons: why the action cannot be taken, what a key does.
  Note,
  /// The icon before the note's words.
  NoteIcon,
}

names!(DialogPart {
  Veil => "veil",
  Panel => "panel",
  Header => "header",
  Text => "text",
  Title => "title",
  Description => "description",
  Body => "body",
  Footer => "footer",
  Close => "close",
  Separator => "separator",
  Mark => "mark",
  Icon => "icon",
  Note => "note",
  NoteIcon => "note_icon",
});

/// The states of a dialog a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DialogState {
  /// Its body scrolls between a header and a footer that stay, each set off
  /// by a line.
  Scrollable,
  /// It has a mark before its title: an icon in its well, or the app's element.
  Mark,
  /// Its footer has a note at its start.
  Note,
}

names!(DialogState {
  Scrollable => "scrollable",
  Mark => "mark",
  Note => "note",
});

/// How wide a dialog is, in the design's steps. A dialog given none is as
/// wide as the board draws one, 420, which is none of these steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DialogSize {
  /// 360: a question and two buttons.
  Sm,
  /// 480: a short form.
  Md,
  /// 640: a form of two columns, a table.
  Lg,
  /// As wide as the window, less the veil's margin.
  Full,
}

names!(DialogSize {
  Sm => "sm",
  Md => "md",
  Lg => "lg",
  Full => "full",
});

/// What builds a dialog's nodes: a theme's in place of the default one.
pub type DialogRenderer = Arc<dyn Fn(DialogView) -> Element + Send + Sync>;

/// A dialog: a panel over the window that asks one thing and waits for the
/// answer. Delete a workspace, accept the terms, name a new file.
///
/// ```
/// use lurq::{
///   core::Signal,
///   ui::{Button, ButtonVariant, Dialog},
/// };
///
/// let open = Signal::new(false);
/// let cancel = open.clone();
/// let dialog = Dialog::new(open.clone())
///   .title("Delete workspace?")
///   .description("This removes every window layout and setting stored for this workspace.")
///   .footer(Button::new("Cancel").variant(ButtonVariant::Outline).on_click(move |_| cancel.set(false)))
///   .footer(Button::new("Delete").variant(ButtonVariant::Destructive));
/// ```
///
/// A dialog does not keep whether it is open: the app's signal does, and the
/// dialog is shown while it holds `true`. Escape, a press on the veil and
/// the close button set it to `false`. As it opens the focus goes to its
/// first control, and as it closes back to what had it. While it is open
/// nothing under the veil takes the pointer, and Tab and Shift+Tab go round
/// the dialog's own controls. It is declared where the app renders it,
/// taking no room there, and covers the whole window.
///
/// A dialog may have a mark before its title ([`icon`](Self::icon), or the app's [`media`](Self::media)), quiet
/// words at its footer's start ([`note`](Self::note)), and an action Enter runs ([`on_confirm`](Self::on_confirm)):
///
/// ```
/// use lurq::{core::Signal, ui::{Button, ButtonVariant, Dialog, icons}};
///
/// let open = Signal::new(true);
/// let close = open.clone();
/// let dialog = Dialog::new(open.clone())
///   .title("Delete connection?")
///   .icon(icons::TRASH)
///   .note("Esc cancels")
///   .on_confirm(move || close.set(false))
///   .footer(Button::new("Delete").variant(ButtonVariant::Destructive));
/// ```
pub struct Dialog {
  open: Signal<bool>,
  title: Option<Arc<str>>,
  description: Option<Arc<str>>,
  icon: Option<Name>,
  media: Option<Element>,
  note: Option<Arc<str>>,
  note_icon: Option<Name>,
  on_confirm: Vec<Confirm>,
  body: Vec<Element>,
  footer: Vec<Element>,
  size: Option<Name>,
  scrollable: bool,
  close_button: bool,
  dismiss_on_veil: bool,
  dismiss_on_escape: bool,
  on_dismiss: Vec<Dismiss>,
  alert: bool,
  label: Option<Arc<str>>,
  described: Option<Arc<str>>,
  close_label: Arc<str>,
  forced: Option<Interaction>,
  inline: bool,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

/// What runs as the dialog is dismissed.
type Dismiss = Arc<dyn Fn() + Send + Sync>;

/// What runs as Enter confirms the dialog.
type Confirm = Arc<dyn Fn() + Send + Sync>;

impl Dialog {
  /// A dialog shown while `open` holds `true`.
  pub fn new(open: Signal<bool>) -> Self {
    Self {
      open,
      title: None,
      description: None,
      icon: None,
      media: None,
      note: None,
      note_icon: None,
      on_confirm: Vec::new(),
      body: Vec::new(),
      footer: Vec::new(),
      size: None,
      scrollable: false,
      close_button: false,
      dismiss_on_veil: true,
      dismiss_on_escape: true,
      on_dismiss: Vec::new(),
      alert: false,
      label: None,
      described: None,
      close_label: super::current_locale().common.close.clone(),
      forced: None,
      inline: false,
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// What the dialog asks or says: "Delete workspace?". It is also what the
  /// dialog is called to tooling that reads the tree.
  pub fn title(mut self, title: &str) -> Self {
    self.title = Some(Arc::from(title));
    self
  }

  /// More of it, under the title, in the muted color.
  pub fn description(mut self, description: &str) -> Self {
    self.description = Some(Arc::from(description));
    self
  }

  /// An icon before the title, in its well (the part `mark`), by its name in the theme's set: what the dialog
  /// is about, a warning, a deletion.
  pub fn icon(mut self, name: impl Into<Name>) -> Self {
    self.icon = Some(name.into());
    self
  }

  /// An element of the app's before the title, in the well in place of the icon: a tile of its own, an avatar.
  pub fn media(mut self, media: impl Into<Element>) -> Self {
    self.media = Some(media.into());
    self
  }

  /// Quiet words at the start of the footer, the buttons at its end: why the action cannot be taken yet, what a
  /// key does. An empty text is none.
  pub fn note(mut self, note: &str) -> Self {
    self.note = (!note.is_empty()).then(|| Arc::from(note));
    self
  }

  /// An icon before the note's words, by its name in the theme's set.
  pub fn note_icon(mut self, name: impl Into<Name>) -> Self {
    self.note_icon = Some(name.into());
    self
  }

  /// Called when Enter is pressed while the dialog is open: its main action, which the app runs (and closes the
  /// dialog after, where it should). Not where the focus is on something Enter does a thing of its own with (a
  /// button, which Enter presses, a link, a select, a text input of several lines), nor for an Enter an input
  /// method composes, nor with Ctrl, Alt or Cmd held. A second handler is called after the first.
  pub fn on_confirm(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_confirm.push(Arc::new(handler));
    self
  }

  /// Adds an element to the body, under the title and the description.
  pub fn child(mut self, child: impl Into<Element>) -> Self {
    self.body.push(child.into());
    self
  }

  /// Adds elements to the body.
  pub fn children(mut self, children: impl IntoIterator<Item = impl Into<Element>>) -> Self {
    self.body.extend(children.into_iter().map(Into::into));
    self
  }

  /// Adds an element to the footer, after those already there: the footer is
  /// a row at the end of the dialog, its least action first and its main
  /// one last. The app puts [`Button`]s there.
  pub fn footer(mut self, element: impl Into<Element>) -> Self {
    self.footer.push(element.into());
    self
  }

  /// A [`DialogSize`], or a size the theme's recipe names.
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.size = Some(size.into());
    self
  }

  /// A width of its own over its size's: logical pixels, or any [`Length`].
  /// A dialog is never wider than the window.
  pub fn width(mut self, width: impl Into<Length>) -> Self {
    let width = width.into();
    self.rules = self.rules.base(DialogPart::Panel, |s| s.width(width));
    self
  }

  /// Whether the dialog is laid out for a body that scrolls: a header and a
  /// footer of their own heights, each set off by a line, with the body
  /// between them. For terms, a long form, a list. Any dialog is as high as
  /// its content while that fits in the window, and its body gives way and
  /// scrolls when it does not.
  pub fn scrollable(mut self, scrollable: bool) -> Self {
    self.scrollable = scrollable;
    self
  }

  /// Whether the header has a button that closes the dialog, at its end.
  pub fn close_button(mut self, close_button: bool) -> Self {
    self.close_button = close_button;
    self
  }

  /// What the close button is called to tooling that reads the tree:
  /// "Close" when not given.
  pub fn close_label(mut self, label: &str) -> Self {
    self.close_label = Arc::from(label);
    self
  }

  /// Draws the close button as it is under the pointer, held down or
  /// focused from the keyboard, with nothing pointing at it: for a gallery
  /// or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// Whether a press on the veil closes the dialog. On by default; off for
  /// a dialog that has to be answered.
  pub fn dismiss_on_veil(mut self, dismiss: bool) -> Self {
    self.dismiss_on_veil = dismiss;
    self
  }

  /// Whether Escape closes the dialog. On by default.
  pub fn dismiss_on_escape(mut self, dismiss: bool) -> Self {
    self.dismiss_on_escape = dismiss;
    self
  }

  /// What runs when the dialog is dismissed, after its signal is set to
  /// `false`: by a press on the veil, by its close button, or by Escape,
  /// wherever the focus is. A button of the app's that closes it, or the app
  /// setting the signal, does not run it.
  /// A second handler runs after the first.
  pub fn on_dismiss(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_dismiss.push(Arc::new(handler));
    self
  }

  /// Whether the dialog asks something that has to be answered before
  /// anything else: an `alertdialog`, and not a `dialog`, to tooling that
  /// reads the tree.
  pub fn alert(mut self, alert: bool) -> Self {
    self.alert = alert;
    self
  }

  /// What the dialog is called to tooling that reads the tree, over its
  /// title: for one whose title is drawn by what it holds and not given to
  /// [`Dialog::title`].
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// What the dialog is described by to tooling that reads the tree: the
  /// message under its title.
  pub fn described(mut self, description: &str) -> Self {
    self.described = Some(Arc::from(description));
    self
  }

  /// Draws the panel where the dialog is declared, whether or not it is open,
  /// without the veil and without taking the window: for a gallery, a
  /// screenshot, a test.
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
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
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

  /// The key of what the dialog lays over the window: two dialogs of one
  /// component are told apart by it.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the dialog in `theme`. Converting a dialog into an element does
  /// this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case {
      variant: None,
      size: self.size,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(DialogState::Scrollable, self.scrollable)
    .state(DialogState::Mark, self.icon.is_some() || self.media.is_some())
    .state(DialogState::Note, self.note.is_some());
    let view = DialogView {
      title: self.title,
      description: self.description,
      icon: self.icon,
      media: self.media,
      note: self.note,
      note_icon: self.note_icon,
      on_confirm: self.on_confirm,
      body: self.body,
      footer: self.footer,
      scrollable: self.scrollable,
      close_button: self.close_button,
      dismiss_on_veil: self.dismiss_on_veil,
      dismiss_on_escape: self.dismiss_on_escape,
      inline: self.inline,
      recipe: theme
        .resolve::<Dialog>(&case, Some(&self.rules))
        .owned_by(self.id.as_ref()),
      on_dismiss: self.on_dismiss,
      alert: self.alert,
      label: self.label,
      described: self.described,
      close_label: self.close_label,
      forced: self.forced,
      open: self.open,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Dialog>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<Dialog> for Element {
  fn from(dialog: Dialog) -> Self {
    dialog.build(&current_theme())
  }
}

/// A dialog as a renderer sees it: what it was given, the style of its
/// parts, and its behavior ready to be dressed.
pub struct DialogView {
  pub title: Option<Arc<str>>,
  pub description: Option<Arc<str>>,
  /// The icon in the mark before the title.
  pub icon: Option<Name>,
  /// The app's element in the mark, in place of the icon.
  pub media: Option<Element>,
  /// The words at the start of the footer.
  pub note: Option<Arc<str>>,
  /// The icon before the note's words.
  pub note_icon: Option<Name>,
  on_confirm: Vec<Confirm>,
  /// What the app put in the body.
  pub body: Vec<Element>,
  /// What the app put in the footer.
  pub footer: Vec<Element>,
  pub scrollable: bool,
  pub close_button: bool,
  pub dismiss_on_veil: bool,
  pub dismiss_on_escape: bool,
  pub inline: bool,
  /// What the close button is called: "Close" unless the app said otherwise.
  pub close_label: Arc<str>,
  /// How the close button is drawn whatever points at it.
  pub forced: Option<Interaction>,
  /// The style of each part of this dialog.
  pub recipe: ResolvedRecipe,
  open: Signal<bool>,
  on_dismiss: Vec<Dismiss>,
  alert: bool,
  label: Option<Arc<str>>,
  described: Option<Arc<str>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl DialogView {
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

  /// What closes the dialog: its signal set to `false`.
  pub fn closer(&self) -> impl Fn() + Send + Sync + Clone + 'static {
    let open = self.open.clone();
    move || open.set(false)
  }

  /// What dismisses the dialog: it closes, and what the app gave
  /// [`Dialog::on_dismiss`] runs, in the order it was given. The veil, the
  /// close button and Escape do this.
  pub fn dismisser(&self) -> impl Fn() + Send + Sync + Clone + 'static {
    let (close, on_dismiss) = (self.closer(), self.on_dismiss.clone());
    move || {
      close();
      for handler in &on_dismiss {
        handler();
      }
    }
  }

  /// What Enter runs, as [`Dialog::on_confirm`] says: `None` for a dialog given no handler.
  pub fn confirmer(&self) -> Option<impl Fn(KeyboardEvent) + Send + Sync + Clone + 'static> {
    if self.on_confirm.is_empty() {
      return None;
    }
    let (handlers, modal) = (self.on_confirm.clone(), !self.inline);
    Some(move |event: KeyboardEvent| {
      let enter = event.key == "Enter" || event.code == "Enter" || event.code == "NumpadEnter";
      // A dialog over the window has every key while it is open; one drawn in place only those of its own focus.
      let ours = modal || event.focus_within;
      if !enter || !ours || event.composing || event.focus_takes_enter || event.ctrl || event.alt || event.meta {
        return;
      }
      event.prevent_default();
      for handler in &handlers {
        handler();
      }
    })
  }

  /// The panel in the style of its part, with nothing in it yet: a dialog to
  /// what reads the tree, called by its title. A press on it goes no further,
  /// so that it never reaches the veil. Enter runs what the dialog confirms with, where it has one.
  pub fn panel(&self) -> Column {
    let mut panel = Column::new()
      .part(&self.part(DialogPart::Panel))
      // Never more than the room the veil leaves it.
      .max_width(Dimension::Pct(100.0))
      .max_height(Dimension::Pct(100.0))
      .on_click(|event: MouseEvent| event.stop_propagation())
      .describe("a11y_role", if self.alert { "alertdialog" } else { "dialog" });
    if let Some(name) = self.label.as_ref().or(self.title.as_ref()) {
      panel = panel.describe("a11y_name", name.clone());
    }
    if let Some(description) = &self.described {
      panel = panel.describe("a11y_description", description.clone());
    }
    if let Some(id) = &self.id {
      panel = panel.id(id.clone());
    }
    if let Some(confirm) = self.confirmer() {
      panel = panel.on_key_down(confirm);
    }
    panel
  }

  /// The mark before the title: the app's element as it is (the well's look is not laid on it), or the icon in its
  /// well. `None` for a dialog with neither.
  pub fn mark(&self) -> Option<Row> {
    if let Some(media) = &self.media {
      let mut holder = Row::new()
        .align_items(Alignment::Center)
        .flex_shrink(0.0)
        .child(media.clone());
      if let Some(id) = self.recipe.part_id(DialogPart::Mark) {
        holder = holder.id(id);
      }
      return Some(holder);
    }
    let part = self.part(DialogPart::Mark);
    let name = self.icon?;
    Some(
      Row::new()
        .align_items(Alignment::Center)
        .justify(Justify::Center)
        .flex_shrink(0.0)
        .part(&part)
        .child(glyph(&self.part(DialogPart::Icon), name, false)),
    )
  }

  /// The note at the start of the footer: its icon and its words, which take the room the buttons leave and go on
  /// to the next line in it. `None` for a dialog with none.
  pub fn note(&self) -> Option<Row> {
    let note = self.note.as_ref()?;
    let part = self.part(DialogPart::Note);
    let mut row = Row::new()
      .align_items(Alignment::Center)
      .flex_full(1.0, 1.0, Some(0.0))
      .min_width(Dimension::Px(0.0))
      .part(&part);
    if let Some(name) = self.note_icon {
      row = row.child(glyph(&self.part(DialogPart::NoteIcon), name, false));
    }
    Some(
      row.child(
        part
          .text(note)
          .flex_full(1.0, 1.0, Some(0.0))
          .min_width(Dimension::Px(0.0)),
      ),
    )
  }

  /// The title and the description, one over the other: `None` for a dialog
  /// that has neither.
  pub fn text(&self) -> Option<Column> {
    if self.title.is_none() && self.description.is_none() {
      return None;
    }
    // It takes the room the close button leaves and no more: a longer text goes on to the next line.
    let mut text = Column::new()
      .part(&self.part(DialogPart::Text))
      .flex_full(1.0, 1.0, Some(0.0))
      .min_width(Dimension::Px(0.0));
    if let Some(title) = &self.title {
      text = text.child(wrapping(&self.part(DialogPart::Title), title));
    }
    if let Some(description) = &self.description {
      text = text.child(wrapping(&self.part(DialogPart::Description), description));
    }
    Some(text)
  }

  /// The button that closes the dialog: a ghost button that is its icon
  /// alone, styled over the theme's button by [`DialogPart::Close`].
  pub fn close(&self) -> Button {
    let close = self.dismisser();
    Button::new(&self.close_label)
      .variant(ButtonVariant::Ghost)
      .size(ButtonSize::Sm)
      .icon(icons::CLOSE)
      .icon_only(true)
      .force(self.forced)
      .rules(&self.recipe.rules_for(DialogPart::Close, ButtonPart::Root))
      .on_click(move |_| close())
  }

  /// The header: the text, and the close button at its end. `None` for a
  /// dialog with neither.
  pub fn header(&self) -> Option<Row> {
    let mark = self.mark();
    let text = self.text();
    if mark.is_none() && text.is_none() && !self.close_button {
      return None;
    }
    let mut header = Row::new()
      .width(Dimension::Pct(100.0))
      // A title beside a mark stands level with its middle.
      .align_items(if self.scrollable || mark.is_some() {
        Alignment::Center
      } else {
        Alignment::Start
      })
      .part(&self.part(DialogPart::Header));
    if let Some(mark) = mark {
      header = header.child(mark);
    }
    if let Some(text) = text {
      header = header.child(text);
    }
    if self.close_button {
      // Without a text the button still stands at the end.
      header = header.justify(Justify::End).child(self.close());
    }
    Some(header)
  }

  /// A line across the panel, in the style of its part.
  pub fn separator(&self) -> Row {
    Row::new().part(&self.part(DialogPart::Separator))
  }

  /// The footer: a row at the end of the panel with the note at its start, where there is one, and what the app
  /// put there at its end. `None` for a dialog with neither.
  pub fn footer(&mut self) -> Option<Row> {
    let note = self.note();
    if self.footer.is_empty() && note.is_none() {
      return None;
    }
    Some(
      Row::new()
        .width(Dimension::Pct(100.0))
        .align_items(Alignment::Center)
        .justify(Justify::End)
        .part(&self.part(DialogPart::Footer))
        .with_children(note)
        .with_children(std::mem::take(&mut self.footer)),
    )
  }

  /// The body: a column with what the app put there, which scrolls when the
  /// window is too low for all of the dialog. `None` for a dialog with
  /// nothing in it.
  pub fn body(&mut self) -> Option<Element> {
    if self.body.is_empty() {
      return None;
    }
    let body = Column::new()
      .width(Dimension::Pct(100.0))
      .part(&self.part(DialogPart::Body))
      .with_children(std::mem::take(&mut self.body));
    // It is as high as what is in it, and gives way and scrolls when that needs more than the window has
    // beyond the header and the footer. It does not grow: a panel with room to spare is not made higher by it.
    Some(
      ScrollVertical::new(body)
        .width(Dimension::Pct(100.0))
        .flex_full(0.0, 1.0, None)
        .into(),
    )
  }

  /// The veil over the window with the panel in its middle. A press on it
  /// closes the dialog unless the dialog says otherwise; it takes the
  /// pointer either way.
  pub fn veil(&self, panel: impl Into<Element>) -> Column {
    let mut veil = Column::new()
      .size(Dimension::Pct(100.0), Dimension::Pct(100.0))
      .align_items(Alignment::Center)
      .justify(Justify::Center)
      .part(&self.part(DialogPart::Veil))
      .child(panel);
    if self.dismiss_on_veil {
      let close = self.dismisser();
      veil = veil.on_click(move |_| close());
    }
    if let Some(key) = &self.key {
      veil = veil.key(key.clone());
    }
    veil
  }

  /// The veil and the panel laid over the window while the dialog is open,
  /// as the engine's [`Modal`]: Escape closes it wherever the focus is and
  /// runs what the app gave `on_dismiss`, Tab stays inside it, and the focus
  /// goes to its first control as it opens and back to what had it as it
  /// closes.
  pub fn modal(&self, panel: impl Into<Element>) -> Element {
    let mut modal = Modal::new(self.veil(panel))
      .open(self.open.clone())
      .target(Root)
      .dismiss_on_escape(self.dismiss_on_escape)
      .takes_focus(true);
    for handler in &self.on_dismiss {
      let handler = handler.clone();
      modal = modal.on_dismiss(move || handler());
    }
    modal.into()
  }

  /// How the default renderer builds a dialog: the header, the body and the
  /// footer in the panel, with lines between them where the body scrolls,
  /// and the panel over the veil.
  pub fn render(mut self) -> Element {
    let header = self.header();
    let body = self.body();
    let footer = self.footer();
    let mut panel = self.panel();
    if let Some(header) = header {
      panel = panel.child(header);
      if self.scrollable {
        panel = panel.child(self.separator());
      }
    }
    if let Some(body) = body {
      panel = panel.child(body);
    }
    if let Some(footer) = footer {
      if self.scrollable {
        panel = panel.child(self.separator());
      }
      panel = panel.child(footer);
    }
    if self.inline { panel.into() } else { self.modal(panel) }
  }
}

impl crate::ui::Styled for Dialog {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for Dialog {
  const NAME: &'static str = "dialog";
  /// The id is the panel's.
  const ROOT: &'static str = "panel";
  type Renderer = DialogRenderer;

  fn parts() -> crate::ui::Declared {
    DialogPart::declared()
  }

  fn states() -> crate::ui::Declared {
    DialogState::declared()
  }

  /// The design's dialog (`pencil/lurq-design-system.pen`, Dialog and
  /// Dialog/Scrollable): a panel of 420 in the popover's color with an edge
  /// of the border's color, corners of `radius-xl`, the large shadow, padded
  /// 24 with 20 between its title, its body and its buttons, over a veil of
  /// the scrim's color. Of the 24 and the 20, 4 are the header's, the body's
  /// and the footer's own: room for the ring of a focused control, which the
  /// body would clip where it scrolls. Its title is 16/600 with the description 8 under it
  /// in 14 of the muted color; the buttons stand at the end, 8 apart. The
  /// one whose body scrolls has no padding of its own: a header of 56 and a
  /// footer of 60, each set off by a line, and the body padded 20 and 16
  /// between them. The close button is 28, its icon 16.
  fn recipe() -> Recipe {
    use DialogPart::{
      Body, Close, Description, Footer, Header, Icon, Mark, Note, NoteIcon, Panel, Separator, Text, Title, Veil,
    };
    use DialogState::Scrollable;
    use PaletteColor::*;
    Recipe::new()
      // The margin keeps the panel off the window's edges.
      .base(Veil, |s| s.background(Scrim).padding_x(16.0).padding_y(16.0))
      .base(Panel, |s| {
        s.width(420.0)
          .background(Popover)
          .foreground(PopoverForeground)
          .border(Edge::inside(BorderSize::Sm, PaletteColor::Border))
          .radius(RadiusSize::Xl)
          .shadow(ShadowStyle::Lg)
          .padding_x(20.0)
          .padding_y(20.0)
          .gap(12.0)
      })
      .base(Header, |s| s.gap(8.0).padding_x(4.0).padding_y(4.0))
      .base(Text, |s| s.gap(8.0))
      .base(Title, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(16.0)
          .weight(FontWeight::SemiBold)
      })
      .base(Description, |s| {
        s.typography(TypographyStyle::Description)
          .line_height(1.5)
          .foreground(MutedForeground)
      })
      .base(Body, |s| s.gap(10.0).padding_x(4.0).padding_y(4.0))
      .base(Footer, |s| s.gap(8.0).padding_x(4.0).padding_y(4.0))
      .base(Close, |s| s.square(28.0).radius(RadiusSize::Md))
      .base(Separator, |s| s.width(Length::Fill).height(1.0).background(PaletteColor::Border))
      // A mark (lurq's, the board has none): a well of 32 in the muted fill, its icon 16 in the foreground, 12 before
      // the title, which stands level with its middle.
      .base(Mark, |s| s.square(32.0).radius(RadiusSize::Lg).background(Muted).foreground(Foreground))
      .base(Icon, |s| s.square(IconSize::Md))
      .state(DialogState::Mark, Header, |s| s.gap(12.0))
      // A note: 12 in the muted foreground at the footer's start, its icon 14, 6 apart; 12 before the buttons.
      .base(Note, |s| {
        s.gap(6.0)
          .padding_end(4.0)
          .typography(TypographyStyle::Caption)
          .foreground(MutedForeground)
      })
      .base(NoteIcon, |s| s.square(IconSize::Sm))
      .size(DialogSize::Sm, Panel, |s| s.width(360.0))
      .size(DialogSize::Md, Panel, |s| s.width(480.0))
      .size(DialogSize::Lg, Panel, |s| s.width(640.0))
      .size(DialogSize::Full, Panel, |s| s.width(Length::Fill))
      // The header, the body and the footer are padded each by itself, and the lines between them run from
      // edge to edge.
      .state(Scrollable, Panel, |s| s.padding_x(0.0).padding_y(0.0).gap(0.0))
      // The close button stands 12 from the edge and the title 20, as the body's text does. A header of 56 for a
      // title, and higher for a title with a description under it.
      .state(Scrollable, Header, |s| s.min_height(56.0).padding_x(12.0).padding_y(12.0))
      .state(Scrollable, Text, |s| s.padding_x(8.0))
      .state(Scrollable, Body, |s| s.padding_x(20.0).padding_y(16.0))
      .state(Scrollable, Footer, |s| s.height(60.0).padding_x(20.0).padding_y(0.0))
  }
}
