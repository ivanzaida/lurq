//! A dialog: a panel over a veil that covers the window, which nothing under
//! it gets past until it is closed.

use std::sync::Arc;

use super::{
  button::{Button, ButtonPart, ButtonSize, ButtonVariant},
  icons, names,
  part_style::Length,
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    events::MouseEvent,
    theme::{BorderSize, PaletteColor, RadiusSize, ShadowStyle, Theme, TypographyStyle},
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
  /// The top of the panel: the heading, and the close button at its end.
  Header,
  /// The title and the description, one over the other.
  Heading,
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
}

names!(DialogPart {
  Veil => "veil",
  Panel => "panel",
  Header => "header",
  Heading => "heading",
  Title => "title",
  Description => "description",
  Body => "body",
  Footer => "footer",
  Close => "close",
  Separator => "separator",
});

/// The states of a dialog a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DialogState {
  /// Its body scrolls between a header and a footer that stay, each set off
  /// by a line.
  Scrollable,
}

names!(DialogState {
  Scrollable => "scrollable",
});

/// How wide a dialog is, in the design's steps. A dialog given none is as
/// wide as the board draws one, 420.
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
/// the close button set it to `false`. While it is open nothing under the
/// veil takes the pointer, and Tab and Shift+Tab go round the dialog's own
/// controls. It is declared where the app renders it, taking no room there,
/// and covers the whole window.
pub struct Dialog {
  open: Signal<bool>,
  title: Option<Arc<str>>,
  description: Option<Arc<str>>,
  body: Vec<Element>,
  footer: Vec<Element>,
  size: Option<Name>,
  scrollable: bool,
  close_button: bool,
  dismiss_on_veil: bool,
  dismiss_on_escape: bool,
  inline: bool,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Dialog {
  /// A dialog shown while `open` holds `true`.
  pub fn new(open: Signal<bool>) -> Self {
    Self {
      open,
      title: None,
      description: None,
      body: Vec::new(),
      footer: Vec::new(),
      size: None,
      scrollable: false,
      close_button: false,
      dismiss_on_veil: true,
      dismiss_on_escape: true,
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

  /// A width of its own, in logical pixels, over its size's. A dialog is
  /// never wider than the window.
  pub fn width(mut self, width: f32) -> Self {
    self.rules = self.rules.base(DialogPart::Panel, |s| s.width(width));
    self
  }

  /// Whether the body scrolls between a header and a footer that stay, each
  /// set off by a line: for terms, a long form, a list. The dialog is as
  /// high as its content while that fits in the window, and the body gives
  /// way when it does not.
  pub fn scrollable(mut self, scrollable: bool) -> Self {
    self.scrollable = scrollable;
    self
  }

  /// Whether the header has a button that closes the dialog, at its end.
  pub fn close_button(mut self, close_button: bool) -> Self {
    self.close_button = close_button;
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
    .state(DialogState::Scrollable, self.scrollable);
    let view = DialogView {
      title: self.title,
      description: self.description,
      body: self.body,
      footer: self.footer,
      scrollable: self.scrollable,
      close_button: self.close_button,
      dismiss_on_veil: self.dismiss_on_veil,
      dismiss_on_escape: self.dismiss_on_escape,
      inline: self.inline,
      recipe: theme.resolve::<Dialog>(&case, Some(&self.rules)),
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
  /// What the app put in the body.
  pub body: Vec<Element>,
  /// What the app put in the footer.
  pub footer: Vec<Element>,
  pub scrollable: bool,
  pub close_button: bool,
  pub dismiss_on_veil: bool,
  pub dismiss_on_escape: bool,
  pub inline: bool,
  /// The style of each part of this dialog.
  pub recipe: ResolvedRecipe,
  open: Signal<bool>,
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

  /// The panel in the style of its part, with nothing in it yet: a dialog to
  /// what reads the tree, called by its title. A press on it goes no further,
  /// so that it never reaches the veil.
  pub fn panel(&self) -> Column {
    let mut panel = Column::new()
      .part(&self.part(DialogPart::Panel))
      // Never more than the room the veil leaves it.
      .max_width(Dimension::Pct(100.0))
      .max_height(Dimension::Pct(100.0))
      .on_click(|event: MouseEvent| event.stop_propagation())
      .describe("a11y_role", "dialog");
    if let Some(title) = &self.title {
      panel = panel.describe("a11y_name", title.clone());
    }
    if let Some(id) = &self.id {
      panel = panel.id(id.clone());
    }
    panel
  }

  /// The title and the description, one over the other: `None` for a dialog
  /// that has neither.
  pub fn heading(&self) -> Option<Column> {
    if self.title.is_none() && self.description.is_none() {
      return None;
    }
    let mut heading = Column::new().part(&self.part(DialogPart::Heading)).flex(1.0);
    if let Some(title) = &self.title {
      let part = self.part(DialogPart::Title);
      heading = heading.child(Row::new().part(&part).child(part.text(title)));
    }
    if let Some(description) = &self.description {
      let part = self.part(DialogPart::Description);
      heading = heading.child(
        Row::new()
          .width(Dimension::Pct(100.0))
          .part(&part)
          .child(part.text(description)),
      );
    }
    Some(heading)
  }

  /// The button that closes the dialog: a ghost button that is its icon
  /// alone, styled over the theme's button by [`DialogPart::Close`].
  pub fn close(&self) -> Button {
    let close = self.closer();
    Button::new("Close")
      .variant(ButtonVariant::Ghost)
      .size(ButtonSize::Sm)
      .icon(icons::CLOSE)
      .icon_only(true)
      .rules(&self.recipe.rules_for(DialogPart::Close, ButtonPart::Root))
      .on_click(move |_| close())
  }

  /// The header: the heading, and the close button at its end. `None` for a
  /// dialog with neither.
  pub fn header(&self) -> Option<Row> {
    let heading = self.heading();
    if heading.is_none() && !self.close_button {
      return None;
    }
    let mut header = Row::new()
      .width(Dimension::Pct(100.0))
      .align_items(if self.scrollable {
        Alignment::Center
      } else {
        Alignment::Start
      })
      .part(&self.part(DialogPart::Header));
    if let Some(heading) = heading {
      header = header.child(heading);
    }
    if self.close_button {
      // Without a heading the button still stands at the end.
      header = header.justify(Justify::End).child(self.close());
    }
    Some(header)
  }

  /// A line across the panel, in the style of its part.
  pub fn separator(&self) -> Row {
    Row::new().part(&self.part(DialogPart::Separator))
  }

  /// The footer: a row at the end of the panel with what the app put there.
  /// `None` for a dialog with nothing in it.
  pub fn footer(&mut self) -> Option<Row> {
    if self.footer.is_empty() {
      return None;
    }
    Some(
      Row::new()
        .width(Dimension::Pct(100.0))
        .align_items(Alignment::Center)
        .justify(Justify::End)
        .part(&self.part(DialogPart::Footer))
        .with_children(std::mem::take(&mut self.footer)),
    )
  }

  /// The body: a column with what the app put there, which scrolls in a
  /// dialog that is scrollable. `None` for a dialog with nothing in it.
  pub fn body(&mut self) -> Option<Element> {
    if self.body.is_empty() {
      return None;
    }
    let body = Column::new()
      .width(Dimension::Pct(100.0))
      .part(&self.part(DialogPart::Body))
      .with_children(std::mem::take(&mut self.body));
    Some(if self.scrollable {
      // It takes the room the panel is given beyond the header and the footer, and gives way when its
      // content needs more than the window has.
      ScrollVertical::new(body)
        .width(Dimension::Pct(100.0))
        .flex_full(1.0, 1.0, None)
        .into()
    } else {
      body.into()
    })
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
      let close = self.closer();
      veil = veil.on_click(move |_| close());
    }
    if let Some(key) = &self.key {
      veil = veil.key(key.clone());
    }
    veil
  }

  /// The veil and the panel laid over the window while the dialog is open,
  /// as the engine's [`Modal`]: Escape closes it and Tab stays inside it.
  pub fn modal(&self, panel: impl Into<Element>) -> Element {
    Modal::new(self.veil(panel))
      .open(self.open.clone())
      .target(Root)
      .dismiss_on_escape(self.dismiss_on_escape)
      .into()
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

impl Themed for Dialog {
  const NAME: &'static str = "dialog";
  type Renderer = DialogRenderer;

  /// The design's dialog (`pencil/lurq-design-system.pen`, Dialog and
  /// Dialog/Scrollable): a panel of 420 in the popover's color with an edge
  /// of the border's color, corners of `radius-xl`, the large shadow, padded
  /// 24 with 20 between its title, its body and its buttons, over a veil of
  /// the scrim's color. Its title is 16/600 with the description 8 under it
  /// in 14 of the muted color; the buttons stand at the end, 8 apart. The
  /// one whose body scrolls has no padding of its own: a header of 56 and a
  /// footer of 60, each set off by a line, and the body padded 20 and 16
  /// between them. The close button is 28, its icon 16.
  fn recipe() -> Recipe {
    use DialogPart::{Body, Close, Description, Footer, Header, Heading, Panel, Separator, Title, Veil};
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
          .padding_x(24.0)
          .padding_y(24.0)
          .gap(20.0)
      })
      .base(Header, |s| s.gap(8.0))
      .base(Heading, |s| s.gap(8.0))
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
      .base(Body, |s| s.gap(10.0))
      .base(Footer, |s| s.gap(8.0))
      .base(Close, |s| s.square(28.0).radius(RadiusSize::Md))
      .base(Separator, |s| s.width(Length::Fill).height(1.0).background(PaletteColor::Border))
      .size(DialogSize::Sm, Panel, |s| s.width(360.0))
      .size(DialogSize::Md, Panel, |s| s.width(480.0))
      .size(DialogSize::Lg, Panel, |s| s.width(640.0))
      .size(DialogSize::Full, Panel, |s| s.width(Length::Fill))
      // The header, the body and the footer are padded each by itself, and the lines between them run from
      // edge to edge.
      .state(Scrollable, Panel, |s| s.padding_x(0.0).padding_y(0.0).gap(0.0))
      // The close button stands 12 from the edge and the title 20, as the body's text does.
      .state(Scrollable, Header, |s| s.height(56.0).padding_x(12.0))
      .state(Scrollable, Heading, |s| s.padding_x(8.0))
      .state(Scrollable, Body, |s| s.padding_x(20.0).padding_y(16.0))
      .state(Scrollable, Footer, |s| s.height(60.0).padding_x(20.0))
  }
}
