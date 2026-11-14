//! A sheet: a panel that slides in from an edge of the window over a veil,
//! which nothing under it gets past until it is closed.

use std::sync::Arc;

use super::{
  button::{Button, ButtonPart, ButtonSize, ButtonVariant},
  icons, names,
  part_style::{Length, PartAnimation, PartStyle},
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  separator::{Separator, SeparatorPart},
  shared::{FADE_IN, FADE_OUT},
  theme::{Themed, current_theme},
};
use crate::{
  animation::{AnimatableProperty, AnimatableValue, Keyframes, KeyframesId},
  app::{
    ctx::Ctx,
    events::MouseEvent,
    theme::{BorderSize, MotionDuration, MotionEasing, PaletteColor, RadiusSize, ShadowStyle, Theme, TypographyStyle},
  },
  components::{
    Column, Modal, Root, Row, ScrollVertical, Sheet as Draggable, SheetDetent, SheetNode, SheetProps, SheetStyle, Stack,
  },
  core::Signal,
  layout::{
    Alignment,
    layout_kind::{Justify, ScrollState},
    text_style::FontWeight,
  },
  node::{Element, border::Border as Edge, color::Color, dimension::Dimension},
};

/// A sheet sliding in from the window's start edge, its end edge, its top
/// and its bottom; and out to each of them. Each moves the panel by its own
/// width or height, whatever that is. The animation engine registers these
/// with the fades, so that every app has them.
pub const SHEET_IN_FROM_START: KeyframesId = KeyframesId::new(0xFF51);
pub const SHEET_IN_FROM_END: KeyframesId = KeyframesId::new(0xFF52);
pub const SHEET_IN_FROM_TOP: KeyframesId = KeyframesId::new(0xFF53);
pub const SHEET_IN_FROM_BOTTOM: KeyframesId = KeyframesId::new(0xFF54);
pub const SHEET_OUT_TO_START: KeyframesId = KeyframesId::new(0xFF55);
pub const SHEET_OUT_TO_END: KeyframesId = KeyframesId::new(0xFF56);
pub const SHEET_OUT_TO_TOP: KeyframesId = KeyframesId::new(0xFF57);
pub const SHEET_OUT_TO_BOTTOM: KeyframesId = KeyframesId::new(0xFF58);

/// The keyframes of a sheet coming in and going out: an offset in percent of
/// the panel's own size, from beyond its edge to where it stands, and back.
pub(crate) fn keyframes() -> [Keyframes; 8] {
  let slide = |id: KeyframesId, along: AnimatableProperty, from: f32, to: f32| {
    Keyframes::new(id)
      .frame(0.0, |frame| {
        frame.set(along, AnimatableValue::Percent(from));
      })
      .frame(1.0, |frame| {
        frame.set(along, AnimatableValue::Percent(to));
      })
  };
  use AnimatableProperty::{OffsetX, OffsetY};
  [
    slide(SHEET_IN_FROM_START, OffsetX, -100.0, 0.0),
    slide(SHEET_IN_FROM_END, OffsetX, 100.0, 0.0),
    slide(SHEET_IN_FROM_TOP, OffsetY, -100.0, 0.0),
    slide(SHEET_IN_FROM_BOTTOM, OffsetY, 100.0, 0.0),
    slide(SHEET_OUT_TO_START, OffsetX, 0.0, -100.0),
    slide(SHEET_OUT_TO_END, OffsetX, 0.0, 100.0),
    slide(SHEET_OUT_TO_TOP, OffsetY, 0.0, -100.0),
    slide(SHEET_OUT_TO_BOTTOM, OffsetY, 0.0, 100.0),
  ]
}

/// The parts of a sheet a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SheetPart {
  /// What covers the window behind the panel, in the scrim's color.
  Veil,
  /// The sheet itself: its color, the edge it shows the page, its corners
  /// and its shadow.
  Panel,
  /// The strip across the top of a bottom sheet that holds the handle.
  Grabber,
  /// The bar in the grabber that says the sheet can be dragged.
  Handle,
  /// The top of the panel: the heading, and the close button at its end.
  Header,
  /// The title and the description, one over the other.
  Heading,
  /// What the sheet is, in a word or a line.
  Title,
  /// More of it, under the title.
  Description,
  /// What the app puts in the sheet: a form, a list, a menu. It scrolls
  /// between the header and the footer.
  Body,
  /// The buttons at the bottom of the panel.
  Footer,
  /// The button that closes the sheet, in the header: an icon alone.
  Close,
  /// The line over the footer.
  Separator,
}

names!(SheetPart {
  Veil => "veil",
  Panel => "panel",
  Grabber => "grabber",
  Handle => "handle",
  Header => "header",
  Heading => "heading",
  Title => "title",
  Description => "description",
  Body => "body",
  Footer => "footer",
  Close => "close",
  Separator => "separator",
});

/// The edge of the window a sheet comes from: the variant of its recipe.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SheetSide {
  /// The left edge, as high as the window: navigation, filters.
  Start,
  /// The right edge, as high as the window: details, a form to edit.
  #[default]
  End,
  /// The top, as wide as the window.
  Top,
  /// The bottom, as wide as the window, with a grabber: a choice or a short
  /// task, as a phone's apps show them.
  Bottom,
}

names!(SheetSide {
  Start => "start",
  End => "end",
  Top => "top",
  Bottom => "bottom",
});

impl SheetSide {
  /// Whether the sheet stands at the start or the end, as high as the window.
  pub fn is_side(self) -> bool {
    matches!(self, Self::Start | Self::End)
  }
}

/// The states of a sheet a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SheetState {
  /// Over the window, coming in and going out: not drawn where it is
  /// declared ([`Sheet::inline`]).
  Modal,
  /// Motion is reduced ([`Theme::reduced_motion`]): the panel fades in and
  /// out where it stands.
  ReducedMotion,
}

names!(SheetState {
  Modal => "modal",
  ReducedMotion => "reduced-motion",
});

/// How large a sheet is across from its edge, in the design's steps: its
/// width at the start or the end, its height at the top or the bottom. A
/// sheet given none is as wide as the board draws one, 320, or as high as
/// what is in it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SheetSize {
  /// 320 wide, or 240 high.
  Sm,
  /// 400 wide, or 360 high.
  Md,
  /// 540 wide, or 480 high.
  Lg,
  /// The whole window.
  Full,
}

names!(SheetSize {
  Sm => "sm",
  Md => "md",
  Lg => "lg",
  Full => "full",
});

/// What builds a sheet's nodes: a theme's in place of the default one.
pub type SheetRenderer = Arc<dyn Fn(SheetView) -> Element + Send + Sync>;

/// A sheet: a panel that slides in from an edge of the window, over the
/// page, for what goes with the page without leaving it. Edit a profile,
/// filter a list, share a document.
///
/// ```
/// use lurq::{
///   core::Signal,
///   ui::{Button, ButtonVariant, Sheet, SheetSide},
/// };
///
/// let open = Signal::new(false);
/// let close = open.clone();
/// let sheet = Sheet::new(open.clone())
///   .side(SheetSide::End)
///   .title("Edit profile")
///   .description("Changes save when you close the panel.")
///   .close_button(true)
///   .footer(Button::new("Close").variant(ButtonVariant::Outline).on_click(move |_| close.set(false)))
///   .footer(Button::new("Save"));
/// ```
///
/// A sheet does not keep whether it is open: the app's signal does, and the
/// sheet is shown while it holds `true`. Escape, a press on the veil and the
/// close button set it to `false`. It slides in from its edge as the veil
/// fades in, and out again; with motion reduced it fades. As it opens the
/// focus goes to its first control, and as it closes back to what had it.
/// While it is open nothing under the veil takes the pointer, and Tab and
/// Shift+Tab go round the sheet's own controls. It is declared where the app
/// renders it, taking no room there, and covers the window under the
/// window's own title bar.
///
/// A sheet at the bottom that a finger drags, between heights and down to
/// close it, is mounted: [`Sheet::mount`].
pub struct Sheet {
  open: Signal<bool>,
  side: SheetSide,
  title: Option<Arc<str>>,
  description: Option<Arc<str>>,
  label: Option<Arc<str>>,
  body: Vec<Element>,
  footer: Vec<Element>,
  size: Option<Name>,
  close_button: bool,
  grabber: Option<bool>,
  dismiss_on_veil: bool,
  dismiss_on_escape: bool,
  detents: Vec<SheetDetent>,
  detent: Option<Signal<usize>>,
  scroll: Option<ScrollState>,
  inline: bool,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Sheet {
  /// A sheet shown while `open` holds `true`, at the window's end.
  pub fn new(open: Signal<bool>) -> Self {
    Self {
      open,
      side: SheetSide::default(),
      title: None,
      description: None,
      label: None,
      body: Vec::new(),
      footer: Vec::new(),
      size: None,
      close_button: false,
      grabber: None,
      dismiss_on_veil: true,
      dismiss_on_escape: true,
      detents: Vec::new(),
      detent: None,
      scroll: None,
      inline: false,
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// The edge of the window the sheet comes from.
  pub fn side(mut self, side: SheetSide) -> Self {
    self.side = side;
    self
  }

  /// What the sheet is: "Edit profile". It is also what the sheet is called
  /// to tooling that reads the tree.
  pub fn title(mut self, title: &str) -> Self {
    self.title = Some(Arc::from(title));
    self
  }

  /// More of it, under the title, in the muted color.
  pub fn description(mut self, description: &str) -> Self {
    self.description = Some(Arc::from(description));
    self
  }

  /// What the sheet is called to tooling that reads the tree, where it has no
  /// title: "Navigation".
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// Adds an element to the body, under the header.
  pub fn child(mut self, child: impl Into<Element>) -> Self {
    self.body.push(child.into());
    self
  }

  /// Adds elements to the body.
  pub fn children(mut self, children: impl IntoIterator<Item = impl Into<Element>>) -> Self {
    self.body.extend(children.into_iter().map(Into::into));
    self
  }

  /// Adds an element to the footer, after those already there: the least
  /// action first and the main one last. At the start, the end and the top
  /// the footer is a row at the panel's end, under a line; at the bottom it
  /// is a column of buttons as wide as the sheet, the main one on top. The
  /// app puts [`Button`]s there.
  pub fn footer(mut self, element: impl Into<Element>) -> Self {
    self.footer.push(element.into());
    self
  }

  /// A [`SheetSize`], or a size the theme's recipe names.
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.size = Some(size.into());
    self
  }

  /// Whether the header has a button that closes the sheet, at its end.
  pub fn close_button(mut self, close_button: bool) -> Self {
    self.close_button = close_button;
    self
  }

  /// Whether the panel has the grabber at its top. A sheet at the bottom has
  /// it unless told otherwise; a sheet at another edge has none.
  pub fn grabber(mut self, grabber: bool) -> Self {
    self.grabber = Some(grabber);
    self
  }

  /// Whether a press on the veil closes the sheet. On by default; off for a
  /// sheet that has to be answered. For a mounted sheet, also whether a
  /// finger drags it down to close it.
  pub fn dismiss_on_veil(mut self, dismiss: bool) -> Self {
    self.dismiss_on_veil = dismiss;
    self
  }

  /// Whether Escape closes the sheet. On by default.
  pub fn dismiss_on_escape(mut self, dismiss: bool) -> Self {
    self.dismiss_on_escape = dismiss;
    self
  }

  /// The heights a mounted sheet at the bottom stands at ([`Sheet::mount`]):
  /// it comes up at the first, and a finger moves it between them. Without
  /// them it stands at the height of its size, or of its content.
  pub fn detents(mut self, detents: impl IntoIterator<Item = SheetDetent>) -> Self {
    self.detents = detents.into_iter().collect();
    self
  }

  /// Which of the detents a mounted sheet is at, by its place in them: the
  /// sheet goes where the signal says, and says here where a finger put it.
  pub fn detent(mut self, detent: Signal<usize>) -> Self {
    self.detent = Some(detent);
    self
  }

  /// The state of the body's scrolling, which the app keeps. A mounted sheet
  /// gives a finger that goes down on the body to the body while it has
  /// something above to show, and to the sheet once it is at its top.
  pub fn scroll(mut self, scroll: ScrollState) -> Self {
    self.scroll = Some(scroll);
    self
  }

  /// Draws the panel where the sheet is declared, whether or not it is open,
  /// without the veil and without taking the window: for a gallery, a
  /// screenshot, a test.
  pub fn inline(mut self, inline: bool) -> Self {
    self.inline = inline;
    self
  }

  /// Puts the sheet in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this sheet over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this sheet.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// The id of the panel.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// The key of what the sheet lays over the window: two sheets of one
  /// component are told apart by it.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// The sheet as a renderer sees it, in `theme`.
  fn view(self, theme: &Theme) -> SheetView {
    let case = Case {
      variant: Some(self.side.into()),
      size: self.size,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(SheetState::Modal, !self.inline)
    .state(SheetState::ReducedMotion, theme.reduced_motion());
    SheetView {
      side: self.side,
      title: self.title,
      description: self.description,
      label: self.label,
      body: self.body,
      footer: self.footer,
      close_button: self.close_button,
      grabber: self.grabber.unwrap_or(self.side == SheetSide::Bottom),
      dismiss_on_veil: self.dismiss_on_veil,
      dismiss_on_escape: self.dismiss_on_escape,
      inline: self.inline,
      recipe: theme.resolve::<Sheet>(&case, Some(&self.rules)),
      open: self.open,
      detents: self.detents,
      detent: self.detent,
      scroll: self.scroll,
      id: self.id,
      key: self.key,
    }
  }

  /// Builds the sheet in `theme`. Converting a sheet into an element does
  /// this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let view = self.view(theme);
    match theme.renderer::<Sheet>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }

  /// The sheet mounted where it is declared. A sheet at the bottom is then
  /// the engine's [`Sheet`](crate::components::Sheet) in the look of this
  /// one: a finger drags it between its [`Sheet::detents`] and down to close
  /// it, it rises over the on-screen keyboard and keeps clear of a phone's
  /// edges, and it comes up and goes down with a spring. A sheet at another
  /// edge, or one drawn inline, is built as converting it would.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let theme = current_theme();
    if self.side != SheetSide::Bottom || self.inline {
      return self.build(&theme);
    }
    self.view(&theme).draggable(ctx)
  }
}

impl From<Sheet> for Element {
  fn from(sheet: Sheet) -> Self {
    sheet.build(&current_theme())
  }
}

/// A sheet as a renderer sees it: what it was given, the style of its parts,
/// and its behavior ready to be dressed.
pub struct SheetView {
  pub side: SheetSide,
  pub title: Option<Arc<str>>,
  pub description: Option<Arc<str>>,
  pub label: Option<Arc<str>>,
  /// What the app put in the body.
  pub body: Vec<Element>,
  /// What the app put in the footer.
  pub footer: Vec<Element>,
  pub close_button: bool,
  pub grabber: bool,
  pub dismiss_on_veil: bool,
  pub dismiss_on_escape: bool,
  pub inline: bool,
  /// The style of each part of this sheet.
  pub recipe: ResolvedRecipe,
  open: Signal<bool>,
  detents: Vec<SheetDetent>,
  detent: Option<Signal<usize>>,
  scroll: Option<ScrollState>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl SheetView {
  /// The style of a part, as this sheet is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The signal that holds whether the sheet is open.
  pub fn open(&self) -> &Signal<bool> {
    &self.open
  }

  /// What closes the sheet: its signal set to `false`.
  pub fn closer(&self) -> impl Fn() + Send + Sync + Clone + 'static {
    let open = self.open.clone();
    move || open.set(false)
  }

  /// Whether the sheet lays out as the board's sheet at the bottom does: its
  /// heading in the middle, and its buttons one over the other as wide as it.
  fn stacked(&self) -> bool {
    self.side == SheetSide::Bottom
  }

  /// What the sheet is called to tooling that reads the tree.
  fn name(&self) -> Option<Arc<str>> {
    self.title.clone().or_else(|| self.label.clone())
  }

  /// The panel in the style of its part, with nothing in it yet: a dialog to
  /// what reads the tree, called by its title. A press on it goes no further,
  /// so that it never reaches the veil. It is never larger than the window.
  pub fn panel(&self) -> Column {
    let mut panel = Column::new()
      .part(&self.part(SheetPart::Panel))
      .max_width(Dimension::Pct(100.0))
      .max_height(Dimension::Pct(100.0))
      .on_click(|event: MouseEvent| event.stop_propagation())
      .describe("a11y_role", "dialog");
    if let Some(name) = self.name() {
      panel = panel.describe("a11y_name", name);
    }
    if let Some(id) = &self.id {
      panel = panel.id(id.clone());
    }
    panel
  }

  /// The grabber across the panel's top, with its handle in the middle:
  /// `None` for a sheet without one.
  pub fn grabber(&self) -> Option<Row> {
    if !self.grabber {
      return None;
    }
    Some(
      Row::new()
        .width(Dimension::Pct(100.0))
        .justify(Justify::Center)
        .part(&self.part(SheetPart::Grabber))
        .child(Row::new().part(&self.part(SheetPart::Handle))),
    )
  }

  /// The title and the description, one over the other: `None` for a sheet
  /// that has neither.
  pub fn heading(&self) -> Option<Column> {
    if self.title.is_none() && self.description.is_none() {
      return None;
    }
    let (align, justify) = if self.stacked() {
      (Alignment::Center, Justify::Center)
    } else {
      (Alignment::Start, Justify::Start)
    };
    let mut heading = Column::new()
      .part(&self.part(SheetPart::Heading))
      .align_items(align)
      .flex(1.0);
    if let Some(title) = &self.title {
      let part = self.part(SheetPart::Title);
      heading = heading.child(Row::new().part(&part).child(part.text(title)));
    }
    if let Some(description) = &self.description {
      let part = self.part(SheetPart::Description);
      heading = heading.child(
        Row::new()
          .width(Dimension::Pct(100.0))
          .justify(justify)
          .part(&part)
          .child(part.text(description)),
      );
    }
    Some(heading)
  }

  /// The button that closes the sheet: a ghost button that is its icon
  /// alone, styled over the theme's button by [`SheetPart::Close`].
  pub fn close(&self) -> Button {
    let close = self.closer();
    Button::new("Close")
      .variant(ButtonVariant::Ghost)
      .size(ButtonSize::Sm)
      .icon(icons::CLOSE)
      .icon_only(true)
      .rules(&self.recipe.rules_for(SheetPart::Close, ButtonPart::Root))
      .on_click(move |_| close())
  }

  /// The header: the heading, and the close button at its end. `None` for a
  /// sheet with neither.
  pub fn header(&self) -> Option<Row> {
    let heading = self.heading();
    if heading.is_none() && !self.close_button {
      return None;
    }
    let mut header = Row::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Start)
      .part(&self.part(SheetPart::Header));
    if let Some(heading) = heading {
      header = header.child(heading);
    }
    if self.close_button {
      // Without a heading the button still stands at the end.
      header = header.justify(Justify::End).child(self.close());
    }
    Some(header)
  }

  /// The line over the footer: a [`Separator`], styled over the theme's by
  /// [`SheetPart::Separator`].
  pub fn separator(&self) -> Element {
    Separator::horizontal()
      .rules(&self.recipe.rules_for(SheetPart::Separator, SeparatorPart::Line))
      .into()
  }

  /// The body: a column with what the app put there, which scrolls between
  /// the header and the footer. At the start and the end it takes the room
  /// they leave, so that the footer stands at the panel's bottom; at the top
  /// and the bottom it is as high as what is in it while that fits, and
  /// gives way and scrolls when it does not. `None` for a sheet with nothing
  /// in it.
  pub fn body(&mut self) -> Option<Element> {
    if self.body.is_empty() {
      return None;
    }
    let body = Column::new()
      .width(Dimension::Pct(100.0))
      .part(&self.part(SheetPart::Body))
      .with_children(std::mem::take(&mut self.body));
    let mut scroll = ScrollVertical::new(body);
    if let Some(state) = &self.scroll {
      scroll = scroll.with_scroll_state(state.clone());
    }
    let grow = if self.side.is_side() { 1.0 } else { 0.0 };
    Some(scroll.width(Dimension::Pct(100.0)).flex_full(grow, 1.0, None).into())
  }

  /// The footer with what the app put there: a row at the panel's end, or at
  /// the bottom a column of buttons as wide as the sheet with the main one,
  /// the last given, on top. `None` for a sheet with nothing in it.
  pub fn footer(&mut self) -> Option<Element> {
    if self.footer.is_empty() {
      return None;
    }
    let part = self.part(SheetPart::Footer);
    let mut footer = std::mem::take(&mut self.footer);
    if self.stacked() {
      footer.reverse();
      return Some(
        Column::new()
          .width(Dimension::Pct(100.0))
          .align_items(Alignment::Stretch)
          .part(&part)
          .with_children(footer)
          .into(),
      );
    }
    Some(
      Row::new()
        .width(Dimension::Pct(100.0))
        .align_items(Alignment::Center)
        .justify(Justify::End)
        .part(&part)
        .with_children(footer)
        .into(),
    )
  }

  /// What the panel holds, in order: the grabber, the header, the body, and
  /// the footer, with the line over it at the edges that have one.
  pub fn contents(&mut self) -> Vec<Element> {
    let mut contents: Vec<Element> = Vec::new();
    if let Some(grabber) = self.grabber() {
      contents.push(grabber.into());
    }
    if let Some(header) = self.header() {
      contents.push(header.into());
    }
    if let Some(body) = self.body() {
      contents.push(body);
    }
    if let Some(footer) = self.footer() {
      if !self.stacked() {
        contents.push(self.separator());
      }
      contents.push(footer);
    }
    contents
  }

  /// The veil over the window with the panel at its edge. A press beside the
  /// panel closes the sheet unless the sheet says otherwise; it takes the
  /// pointer either way. The veil fades in and out under the panel, which
  /// slides by itself.
  pub fn veil(&self, panel: impl Into<Element>) -> Stack {
    let full = Dimension::Pct(100.0);
    let veil = Column::new().size(full, full).part(&self.part(SheetPart::Veil));
    // What holds the panel at its edge: as high as the window at the start and the end, and in the middle of
    // the window's width at the top and the bottom, where a panel narrower than the window stands.
    let holder: Element = match self.side {
      SheetSide::Start | SheetSide::End => {
        let justify = if self.side == SheetSide::Start {
          Justify::Start
        } else {
          Justify::End
        };
        let mut holder = Row::new()
          .size(full, full)
          .align_items(Alignment::Stretch)
          .justify(justify)
          .child(panel);
        if self.dismiss_on_veil {
          let close = self.closer();
          holder = holder.on_click(move |_| close());
        }
        holder.into()
      }
      SheetSide::Top | SheetSide::Bottom => {
        let justify = if self.side == SheetSide::Top {
          Justify::Start
        } else {
          Justify::End
        };
        let mut holder = Column::new()
          .size(full, full)
          .align_items(Alignment::Center)
          .justify(justify)
          .child(panel);
        if self.dismiss_on_veil {
          let close = self.closer();
          holder = holder.on_click(move |_| close());
        }
        holder.into()
      }
    };
    let mut stack = Stack::new().size(full, full).child(veil).child(holder);
    if let Some(key) = &self.key {
      stack = stack.key(key.clone());
    }
    stack
  }

  /// The veil and the panel laid over the window while the sheet is open, as
  /// the engine's [`Modal`]: Escape closes it, Tab stays inside it, and the
  /// focus goes to its first control as it opens and back to what had it as
  /// it closes.
  pub fn modal(&self, panel: impl Into<Element>) -> Element {
    Modal::new(self.veil(panel))
      .open(self.open.clone())
      .target(Root)
      .dismiss_on_escape(self.dismiss_on_escape)
      .takes_focus(true)
      .into()
  }

  /// How the default renderer builds a sheet: the grabber, the header, the
  /// body and the footer in the panel, and the panel at its edge over the
  /// veil.
  pub fn render(mut self) -> Element {
    let contents = self.contents();
    let panel = self.panel().with_children(contents);
    if self.inline { panel.into() } else { self.modal(panel) }
  }

  /// A sheet at the bottom as the engine's draggable sheet, in this sheet's
  /// look: the engine lays out its panel and its backdrop, and the recipe
  /// dresses them; what the panel holds is this sheet's.
  fn draggable(mut self, ctx: &mut Ctx) -> Element {
    let panel = self.part(SheetPart::Panel);
    let (panel_look, veil_look) = (look(&panel), look(&self.part(SheetPart::Veil)));
    let handle_look = look(&self.part(SheetPart::Handle));
    let rest = panel.rest();
    let theme = self.theme().clone();
    let px = |length: Option<Length>| match length.and_then(|length| length.resolve(&theme)) {
      Some(Dimension::Px(px)) => Some(px),
      _ => None,
    };
    // Its heights: the app's, or the one its size gives the panel, or its content's.
    let detents = if !self.detents.is_empty() {
      std::mem::take(&mut self.detents)
    } else {
      match rest.height {
        Some(Length::Fill) => vec![SheetDetent::Large],
        height => match px(height) {
          Some(height) => vec![SheetDetent::Height(height)],
          None => vec![SheetDetent::Content],
        },
      }
    };
    let fits_content = detents.iter().all(|detent| *detent == SheetDetent::Content);
    let mut style = SheetStyle::new().grabber(false).dim(1.0).corner_radius(0.0);
    style.backdrop = Color::new(0, 0, 0, 0);
    if let Some(max_width) = px(rest.max_width) {
      style.max_width = max_width;
    }

    let contents = self.contents();
    // The panel's room and its spacing are this column's: the engine's panel reaches under the window's edge.
    let mut content = Column::new()
      .width(Dimension::Pct(100.0))
      .part(&spacing(&panel))
      .with_children(contents)
      .describe("a11y_role", "dialog");
    content = if fits_content {
      content.max_height(Dimension::Pct(100.0))
    } else {
      content.height(Dimension::Pct(100.0))
    };
    if let Some(name) = self.name() {
      content = content.describe("a11y_name", name);
    }
    if let Some(id) = &self.id {
      content = content.id(id.clone());
    }

    let mut props = SheetProps::new(self.open.clone())
      .detents(detents)
      .style(style)
      .dismissable(self.dismiss_on_veil)
      .dismiss_on_escape(self.dismiss_on_escape)
      .takes_focus(true)
      .dress(move |node, column| match node {
        SheetNode::Backdrop => column.part(&veil_look),
        SheetNode::Panel => column.part(&panel_look),
        // The engine draws no grabber of its own here: the sheet's is in what it holds.
        SheetNode::Grabber => column.part(&handle_look),
      });
    if let Some(detent) = self.detent.take() {
      props = props.detent(detent);
    }
    if let Some(scroll) = self.scroll.take() {
      props = props.scroll(scroll);
    }
    match &self.key {
      Some(key) => Draggable::mount_keyed(ctx, key, props, content),
      None => Draggable::mount(ctx, props, content),
    }
  }
}

/// The look of a part alone, its colors, its edges, its corners and its
/// shadow, with nothing of its room or its motion: what dresses a node the
/// engine lays out and moves itself.
fn look(part: &ResolvedPart) -> ResolvedPart {
  let rest = part.rest();
  let look = PartStyle {
    background: rest.background,
    foreground: rest.foreground,
    border: rest.border,
    underline: rest.underline,
    start_line: rest.start_line,
    end_line: rest.end_line,
    top_line: rest.top_line,
    radius: rest.radius,
    shadow: rest.shadow,
    ..PartStyle::default()
  };
  only(part.theme(), look)
}

/// The padding of a part and the room between its children alone.
fn spacing(part: &ResolvedPart) -> ResolvedPart {
  let rest = part.rest();
  let spacing = PartStyle {
    padding_x: rest.padding_x,
    padding_y: rest.padding_y,
    padding_top: rest.padding_top,
    padding_bottom: rest.padding_bottom,
    padding_start: rest.padding_start,
    padding_end: rest.padding_end,
    gap: rest.gap,
    ..PartStyle::default()
  };
  only(part.theme(), spacing)
}

/// A part that is `style` and nothing else.
fn only(theme: &Theme, style: PartStyle) -> ResolvedPart {
  let name = Name::new("part");
  Recipe::new()
    .base(name, |_| style)
    .resolve(&Case::default(), theme)
    .part(name)
}

impl Themed for Sheet {
  const NAME: &'static str = "sheet";
  type Renderer = SheetRenderer;

  /// The design's sheet (`pencil/lurq-design-system.pen`, Sheet and Drawer).
  /// At the start or the end: a panel 320 wide and as high as the window, in
  /// the background's colors, with a line of the border's color along the
  /// edge it shows the page and the large shadow. Its header is padded 20
  /// at the top and the sides, the title 16/600 over the description in 13
  /// of the muted color, 4 apart, the close button's icon of 16 at its end,
  /// 12 from them; the body is padded 20 with 16 between what it holds; the
  /// footer is padded 16 and 20, its buttons 8 apart at the end, under a
  /// line. At the top it is as wide as the window, its line along its
  /// bottom. At the bottom (Drawer) it has the line all round and corners
  /// of `radius-xl` at its top, a grabber with a handle 40 by 4 in the muted
  /// color 8 under its top edge, its heading in the middle 20 under that,
  /// and its buttons as wide as it, 8 apart, 20 from the edges. It comes in
  /// over the theme's slow duration in the entering curve as the veil of the
  /// scrim's color fades in, and goes out over the base duration in the
  /// leaving curve (Foundations, Motion: a dialog, a sheet and a drawer take
  /// the slow duration, and exits are quicker than entrances). Of the
  /// header's 20, 4 are the heading's own, so that the close button of 24
  /// has its icon where the board draws it.
  fn recipe() -> Recipe {
    use PaletteColor::*;
    use SheetPart::{Body, Close, Description, Footer, Grabber, Handle, Header, Heading, Panel, Title, Veil};
    use SheetSide::{Bottom, End, Start, Top};
    use SheetSize::{Full, Lg, Md, Sm};
    use SheetState::{Modal, ReducedMotion};
    let line = || Edge::inside(BorderSize::Sm, PaletteColor::Border);
    let entering = |keyframes| PartAnimation::new(keyframes, MotionDuration::Slow, MotionEasing::Enter);
    let leaving = |keyframes| PartAnimation::new(keyframes, MotionDuration::Base, MotionEasing::Exit);
    let moving = |side: SheetSide| Selector::new().variant(side).state(Modal);
    let sized = |side: SheetSide, size: SheetSize| Selector::new().variant(side).size(size);
    let mut recipe = Recipe::new()
      .base(Veil, |s| {
        s.background(Scrim)
          .enter(entering(FADE_IN))
          .exit(leaving(FADE_OUT))
      })
      .base(Panel, |s| {
        s.background(Background)
          .foreground(Foreground)
          .shadow(ShadowStyle::Lg)
      })
      .base(Header, |s| s.gap(8.0).padding_x(16.0).padding_top(16.0))
      .base(Heading, |s| s.gap(4.0).padding_start(4.0).padding_top(4.0))
      .base(Title, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(16.0)
          .weight(FontWeight::SemiBold)
      })
      .base(Description, |s| {
        s.typography(TypographyStyle::Description)
          .font_size(13.0)
          .foreground(MutedForeground)
      })
      .base(Close, |s| {
        s.square(24.0)
          .radius(RadiusSize::Md)
          .foreground(MutedForeground)
      })
      .base(Body, |s| s.gap(16.0).padding_x(20.0).padding_y(20.0))
      .base(Footer, |s| s.gap(8.0).padding_x(20.0).padding_y(16.0))
      // At the start and the end, as high as the window.
      .variant(Start, Panel, |s| s.width(320.0).height(Length::Fill).end_line(line()))
      .variant(End, Panel, |s| s.width(320.0).height(Length::Fill).start_line(line()))
      // At the top, as wide as the window.
      .variant(Top, Panel, |s| s.width(Length::Fill).underline(line()))
      // At the bottom, the board's drawer.
      .variant(Bottom, Panel, |s| {
        s.width(Length::Fill)
          .border(line())
          .corners(RadiusSize::Xl, RadiusSize::Xl, 0.0, 0.0)
          .padding_bottom(20.0)
      })
      .variant(Bottom, Grabber, |s| s.padding_top(8.0))
      .base(Handle, |s| {
        s.width(40.0)
          .height(4.0)
          .radius(RadiusSize::Full)
          .background(Muted)
      })
      .variant(Bottom, Header, |s| s.padding_x(20.0).padding_top(20.0))
      .variant(Bottom, Heading, |s| s.padding_start(0.0).padding_top(0.0))
      .variant(Bottom, Body, |s| s.padding_y(0.0).padding_top(20.0))
      .variant(Bottom, Footer, |s| s.padding_y(0.0).padding_top(20.0));
    for (size, width, height) in [(Sm, 320.0, 240.0), (Md, 400.0, 360.0), (Lg, 540.0, 480.0)] {
      recipe = recipe
        .when(sized(Start, size), Panel, |s| s.width(width))
        .when(sized(End, size), Panel, |s| s.width(width))
        .when(sized(Top, size), Panel, |s| s.height(height))
        .when(sized(Bottom, size), Panel, |s| s.height(height));
    }
    recipe = recipe
      .when(sized(Start, Full), Panel, |s| s.width(Length::Fill))
      .when(sized(End, Full), Panel, |s| s.width(Length::Fill))
      .when(sized(Top, Full), Panel, |s| s.height(Length::Fill))
      .when(sized(Bottom, Full), Panel, |s| s.height(Length::Fill));
    // Over the window it slides in from its edge and out to it, and with motion reduced it fades where it stands.
    for (side, coming, going) in [
      (Start, SHEET_IN_FROM_START, SHEET_OUT_TO_START),
      (End, SHEET_IN_FROM_END, SHEET_OUT_TO_END),
      (Top, SHEET_IN_FROM_TOP, SHEET_OUT_TO_TOP),
      (Bottom, SHEET_IN_FROM_BOTTOM, SHEET_OUT_TO_BOTTOM),
    ] {
      recipe = recipe.when(moving(side), Panel, |s| s.enter(entering(coming)).exit(leaving(going)));
    }
    recipe.when(Selector::new().state(Modal).state(ReducedMotion), Panel, |s| {
      s.enter(entering(FADE_IN)).exit(leaving(FADE_OUT))
    })
  }
}
