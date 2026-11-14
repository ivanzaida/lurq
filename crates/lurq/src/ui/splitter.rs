//! A splitter: panes side by side or one over the other, that share its room
//! by the dividers between them, which a person drags.
//!
//! A divider is dragged with the pointer held: the drag follows the pointer
//! wherever it goes, in the window and out of it, with the cursor of the
//! divider until the button is let go. The panes are moved by sizes their
//! element refs are given ([`ElementRefMut::set_size`]): what is in them is
//! laid out again at its new size, and not built again.

use std::sync::Arc;

use parking_lot::Mutex;

use super::{
  icons, names,
  part_style::Length,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::glyph,
  text_box::edge,
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    component::Component,
    ctx::Ctx,
    events::{DragEvent, KeyboardEvent, MouseButtonMask, MouseEvent},
    theme::{PaletteColor, RadiusSize, Theme},
  },
  components::{Column, Row, Stack},
  core::{ElementRect, ElementRefMut, Signal},
  layout::Alignment,
  node::{
    BackgroundColor, CursorIcon, Element, HitTestBehavior, color::Color, dimension::Dimension, transform::Transform2D,
  },
};

/// The parts of a splitter a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SplitterPart {
  /// The splitter: its panes and the dividers between them.
  Root,
  /// A pane: what the app put in it, cut to its size.
  Pane,
  /// The room around a divider's line that takes the pointer and the focus: what is dragged, pressed twice and
  /// moved by the keys.
  Handle,
  /// The divider's line at rest, as thick as the room it keeps between two panes.
  Line,
  /// The wider line over it under the pointer, while it is dragged and while it is focused from the keyboard, in the
  /// color the handle's state gives it.
  Bar,
  /// The grip in the middle of the divider: what the board's Resizable has.
  Grip,
  /// The icon on the grip.
  GripIcon,
}

names!(SplitterPart {
  Root => "root",
  Pane => "pane",
  Handle => "handle",
  Line => "line",
  Bar => "bar",
  Grip => "grip",
  GripIcon => "grip-icon",
});

/// The states of a splitter a recipe styles, besides what the pointer and the
/// keyboard do to its dividers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SplitterState {
  /// Its panes are one over the other, its dividers lie across.
  Vertical,
  /// It is drawn as a box of its own: the board's specimen, with a fill, an edge and round corners.
  Framed,
  /// Its dividers take no input.
  Disabled,
}

names!(SplitterState {
  Vertical => "vertical",
  Framed => "framed",
  Disabled => "disabled",
});

/// Which way a splitter lays its panes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SplitterOrientation {
  /// Side by side, the dividers standing between them: a side bar beside an editor.
  #[default]
  Horizontal,
  /// One over the other, the dividers lying between them: an editor over a terminal.
  Vertical,
}

impl SplitterOrientation {
  /// The orientation of its dividers, as tooling is told it: a horizontal splitter's dividers stand upright.
  fn divider(self) -> &'static str {
    match self {
      Self::Horizontal => "vertical",
      Self::Vertical => "horizontal",
    }
  }
}

/// What builds a splitter's nodes: a theme's in place of the default one.
pub type SplitterRenderer = Arc<dyn Fn(SplitterView) -> Element + Send + Sync>;

/// What is told of the sizes of the panes.
type SizesHandler = Arc<dyn Fn(&[f32]) + Send + Sync>;

/// How large a pane is to begin with.
#[derive(Clone, Copy, Debug, PartialEq)]
enum PaneSize {
  /// Pixels it keeps as the splitter grows and shrinks.
  Px(f32),
  /// A share of the splitter, from 0 to 1, that it keeps as the splitter grows and shrinks.
  Share(f32),
}

/// A pane of a [`Splitter`]: what the app puts in it, how large it is to begin
/// with, and how small and how large a divider may make it.
///
/// ```
/// use lurq::{components::Text, ui::SplitterPane};
///
/// let files = SplitterPane::new(Text::new("Files")).size(240.0).min(160.0).max(480.0).collapsible(true);
/// let preview = SplitterPane::new(Text::new("Preview")).share(0.3).min(120.0);
/// // No size: it takes the rest of the room.
/// let editor = SplitterPane::new(Text::new("Editor")).min(200.0);
/// ```
#[derive(Clone)]
pub struct SplitterPane {
  content: Element,
  size: Option<PaneSize>,
  min: f32,
  max: f32,
  collapsible: bool,
  label: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl SplitterPane {
  /// A pane holding `content`, laid out in its whole width from its top: content with a height of
  /// `Dimension::Pct(100.0)` fills it. A pane given no size takes what the others leave.
  pub fn new(content: impl Into<Element>) -> Self {
    Self {
      content: content.into(),
      size: None,
      min: 0.0,
      max: f32::INFINITY,
      collapsible: false,
      label: None,
      key: None,
    }
  }

  /// How many pixels long the pane is along the splitter to begin with. It keeps them as the splitter grows and
  /// shrinks: a side bar.
  pub fn size(mut self, px: f32) -> Self {
    self.size = Some(PaneSize::Px(px.max(0.0)));
    self
  }

  /// What share of the splitter the pane takes to begin with, from 0 to 1. It keeps its share as the splitter
  /// grows and shrinks.
  pub fn share(mut self, share: f32) -> Self {
    self.size = Some(PaneSize::Share(share.clamp(0.0, 1.0)));
    self
  }

  /// The least the pane is, in pixels: no divider and no window makes it smaller while it is open.
  pub fn min(mut self, px: f32) -> Self {
    self.min = px.max(0.0);
    self
  }

  /// The most the pane is, in pixels.
  pub fn max(mut self, px: f32) -> Self {
    self.max = px.max(0.0);
    self
  }

  /// Whether the pane closes when a divider is dragged under half its least size, and opens again by a drag back,
  /// by a double press on the divider or by Enter on it. The pane that takes the rest of the room does not close.
  pub fn collapsible(mut self, collapsible: bool) -> Self {
    self.collapsible = collapsible;
    self
  }

  /// What the pane is called: the divider after it is called so too, to tooling that reads the tree.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// What tells the pane apart from the others where panes come and go.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }
}

/// How a pane gives way to the room: what it keeps as the splitter grows and shrinks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
  Px,
  Share,
  /// It takes what the others leave.
  Rest,
}

/// A pane as the splitter lays it out.
#[derive(Clone, Debug, PartialEq)]
struct Spec {
  kind: Kind,
  /// Its pixels, or its share, to begin with; nothing for the pane that takes the rest.
  begin: f32,
  min: f32,
  max: f32,
  collapsible: bool,
  label: Option<Arc<str>>,
}

/// The panes as the splitter lays them out. The pane that takes the rest is the last one given no size, or the last
/// one where all have one; the others given none share equally what the given shares leave.
fn specs(panes: &[SplitterPane]) -> Vec<Spec> {
  let rest = panes
    .iter()
    .rposition(|pane| pane.size.is_none())
    .unwrap_or(panes.len().saturating_sub(1));
  let given: f32 = panes
    .iter()
    .filter_map(|pane| match pane.size {
      Some(PaneSize::Share(share)) => Some(share),
      _ => None,
    })
    .sum();
  let unsized_panes = panes.iter().filter(|pane| pane.size.is_none()).count().max(1);
  let even = (1.0 - given).max(0.0) / unsized_panes as f32;
  panes
    .iter()
    .enumerate()
    .map(|(index, pane)| {
      let (kind, begin) = if index == rest {
        (Kind::Rest, 0.0)
      } else {
        match pane.size {
          Some(PaneSize::Px(px)) => (Kind::Px, px),
          Some(PaneSize::Share(share)) => (Kind::Share, share),
          None => (Kind::Share, even),
        }
      };
      Spec {
        kind,
        begin,
        min: pane.min,
        max: pane.max.max(pane.min),
        collapsible: pane.collapsible && kind != Kind::Rest,
        label: pane.label.clone(),
      }
    })
    .collect()
}

/// A splitter: panes side by side or one over the other that share its room by
/// the dividers between them, which a person drags. The side bar, the editor
/// and the terminal of a window.
///
/// ```
/// use lurq::{
///   components::Text,
///   core::Signal,
///   ui::{Splitter, SplitterPane},
/// };
///
/// let layout = Signal::new(Vec::new());
/// let window = Splitter::horizontal()
///   .pane(SplitterPane::new(Text::new("Files")).size(240.0).min(160.0).collapsible(true).label("Files"))
///   .pane(SplitterPane::new(
///     Splitter::vertical()
///       .pane(SplitterPane::new(Text::new("Editor")).min(120.0))
///       .pane(SplitterPane::new(Text::new("Terminal")).size(160.0).min(80.0).collapsible(true)),
///   ))
///   .sizes(layout.clone())
///   .on_resize_end(|sizes| println!("panes now {sizes:?}"));
/// ```
///
/// A pane keeps what it was told to as the splitter grows and shrinks: its
/// pixels, or its share; the pane given no size takes the rest. No pane is
/// made smaller than its least size while there is room for all of them.
///
/// A divider is a stop of Tab. A drag moves it, with the pointer held until it
/// is let go; a double press puts it back where it began, or opens the pane
/// beside it that is closed. The arrow keys of its orientation move it by
/// [`Splitter::step`], with Shift by [`Splitter::large_step`]; Home and End
/// take the pane before it to its least and most size, and Enter closes and
/// opens the collapsible pane beside it.
///
/// The sizes are the splitter's own, or the app's in [`Splitter::sizes`]: a
/// signal the splitter writes as a divider moves, which the app saves and
/// restores. A splitter converted into an element keeps them while what holds
/// it does not render again, unless it is given the signal;
/// [`Splitter::mount`] keeps them, and shows the sizes the app sets in the
/// signal from outside.
#[derive(Clone)]
pub struct Splitter {
  orientation: SplitterOrientation,
  panes: Vec<SplitterPane>,
  sizes: Option<Signal<Vec<f32>>>,
  on_resize: Vec<SizesHandler>,
  on_resize_end: Vec<SizesHandler>,
  step: f32,
  large_step: f32,
  grip: bool,
  framed: bool,
  disabled: bool,
  label: Option<Arc<str>>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
  /// What the host of a mounted splitter is told: the sizes the splitter itself wrote last, and when a divider was
  /// let go.
  host: Option<Arc<HostLink>>,
}

impl Splitter {
  /// A splitter of panes side by side.
  pub fn horizontal() -> Self {
    Self::new(SplitterOrientation::Horizontal)
  }

  /// A splitter of panes one over the other.
  pub fn vertical() -> Self {
    Self::new(SplitterOrientation::Vertical)
  }

  /// A splitter of panes laid `orientation`.
  pub fn new(orientation: SplitterOrientation) -> Self {
    Self {
      orientation,
      panes: Vec::new(),
      sizes: None,
      on_resize: Vec::new(),
      on_resize_end: Vec::new(),
      step: 10.0,
      large_step: 50.0,
      grip: false,
      framed: false,
      disabled: false,
      label: None,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
      host: None,
    }
  }

  /// Adds a pane after the others, with a divider before it.
  pub fn pane(mut self, pane: SplitterPane) -> Self {
    self.panes.push(pane);
    self
  }

  /// Adds panes after the others.
  pub fn panes(mut self, panes: impl IntoIterator<Item = SplitterPane>) -> Self {
    self.panes.extend(panes);
    self
  }

  /// The sizes of the panes, the app's: a pixel length along the splitter for each pane, as a divider last left
  /// them (0 for a closed pane), which the splitter writes as a divider moves. Empty to begin with the sizes the
  /// panes were given. Set from a saved layout, a pane of pixels takes its pixels, and one of a share takes the
  /// share its pixels were of the room they were saved in.
  pub fn sizes(mut self, sizes: Signal<Vec<f32>>) -> Self {
    self.sizes = Some(sizes);
    self
  }

  /// Called with the sizes of the panes each time a divider moves: while it is dragged, and for each key that moves
  /// it.
  pub fn on_resize(mut self, handler: impl Fn(&[f32]) + Send + Sync + 'static) -> Self {
    self.on_resize.push(Arc::new(handler));
    self
  }

  /// Called with the sizes of the panes once a divider has moved: when a drag that moved it is let go, and after a
  /// key, a double press or Enter moved it.
  pub fn on_resize_end(mut self, handler: impl Fn(&[f32]) + Send + Sync + 'static) -> Self {
    self.on_resize_end.push(Arc::new(handler));
    self
  }

  /// How many pixels an arrow key moves a divider: 10 when not given.
  pub fn step(mut self, px: f32) -> Self {
    self.step = px.max(1.0);
    self
  }

  /// How many pixels an arrow key moves a divider with Shift held: 50 when not given.
  pub fn large_step(mut self, px: f32) -> Self {
    self.large_step = px.max(1.0);
    self
  }

  /// Whether each divider has a grip in its middle: the board's Resizable.
  pub fn grip(mut self, grip: bool) -> Self {
    self.grip = grip;
    self
  }

  /// Whether the splitter is drawn as a box of its own, with a fill, an edge and round corners: the board's
  /// specimen. A splitter that is a window's layout has none.
  pub fn framed(mut self, framed: bool) -> Self {
    self.framed = framed;
    self
  }

  /// Whether the dividers take no input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// What a divider is called to tooling where the pane before it has no label.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// Puts the splitter in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this splitter over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this splitter: what the component it is in says of it in this place.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the dividers as they are under the pointer, dragged or focused from the keyboard, with nothing pointing
  /// at them: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the splitter's root. Its dividers are `{id}-divider-{n}`, from 0 at the first.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// The splitter as a renderer sees it in `theme`.
  pub fn view(self, theme: &Theme) -> SplitterView {
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(
      SplitterState::Vertical,
      self.orientation == SplitterOrientation::Vertical,
    )
    .state(SplitterState::Framed, self.framed)
    .state(SplitterState::Disabled, self.disabled);
    let specs = specs(&self.panes);
    let (contents, keys) = self.panes.into_iter().map(|pane| (pane.content, pane.key)).unzip();
    SplitterView {
      orientation: self.orientation,
      grip: self.grip,
      disabled: self.disabled,
      label: self.label,
      recipe: theme.resolve::<Splitter>(&case, Some(&self.rules)),
      specs,
      contents,
      keys,
      sizes: self.sizes.unwrap_or_else(|| Signal::new(Vec::new())),
      on_resize: self.on_resize,
      on_resize_end: self.on_resize_end,
      step: self.step,
      large_step: self.large_step,
      forced: self.forced,
      id: self.id,
      key: self.key,
      host: self.host,
    }
  }

  /// Builds the splitter in `theme`. Converting a splitter into an element does this in the theme of what is being
  /// rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let view = self.view(theme);
    match theme.renderer::<Splitter>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }

  /// The splitter mounted where it is declared: it keeps the sizes of its panes from render to render where the app
  /// gives it no signal for them, shows the sizes the app sets in [`Splitter::sizes`] from outside, and tells tooling
  /// the sizes a divider was let go at. Moving a divider renders nothing again while it moves; the splitter alone
  /// renders again when it is let go.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let key = self.key.clone();
    let props = SplitterProps {
      splitter: Arc::new(Mutex::new(self)),
    };
    match key {
      Some(key) => ctx.mount_keyed::<SplitterHost>(&key, props),
      None => ctx.mount::<SplitterHost>(props),
    }
  }
}

impl From<Splitter> for Element {
  fn from(splitter: Splitter) -> Self {
    splitter.build(&current_theme())
  }
}

/// Where a drag of a divider began.
struct DragStart {
  /// The sizes of the panes then, as they were laid out.
  sizes: Vec<f32>,
  /// Whether the divider has moved since.
  moved: bool,
}

/// What a splitter's handlers share: the panes, where they are, and what is told of their sizes.
struct Panes {
  orientation: SplitterOrientation,
  specs: Vec<Spec>,
  /// The thickness of a divider: the room between two panes.
  thickness: f32,
  sizes: Signal<Vec<f32>>,
  /// The panes, and their copies under the dividers' handles, which take the sizes the panes take.
  panes: Vec<ElementRefMut>,
  shadows: Vec<ElementRefMut>,
  /// The row of the panes: its length along the splitter is the room they share.
  room: ElementRefMut,
  on_resize: Vec<SizesHandler>,
  on_resize_end: Vec<SizesHandler>,
  drag: Mutex<Option<DragStart>>,
  host: Option<Arc<HostLink>>,
  /// What an arrow key moves a divider by, and with Shift.
  steps: (f32, f32),
}

impl Panes {
  fn along(&self, rect: ElementRect) -> f32 {
    match self.orientation {
      SplitterOrientation::Horizontal => rect.width,
      SplitterOrientation::Vertical => rect.height,
    }
  }

  /// The room the panes share as last laid out.
  fn room(&self) -> f32 {
    self.along(self.room.bounds())
  }

  /// The room the dividers take.
  fn gaps(&self) -> f32 {
    self.thickness * self.specs.len().saturating_sub(1) as f32
  }

  /// The sizes of the panes as they were last laid out.
  fn measured(&self) -> Vec<f32> {
    self.panes.iter().map(|pane| self.along(pane.bounds())).collect()
  }

  /// The sizes of the panes now: as they were last set, in the room as it was last laid out, which holds before
  /// the panes are laid out again; as they were laid out where none were set yet.
  fn current(&self) -> Vec<f32> {
    let model = self.sizes.get_untracked();
    if model.len() == self.specs.len() {
      self.expected(&model, self.room())
    } else {
      self.measured()
    }
  }

  /// The sizes the panes have to begin with in a room of `room`.
  fn initial(&self, room: f32) -> Vec<f32> {
    let mut sizes: Vec<f32> = self
      .specs
      .iter()
      .map(|spec| match spec.kind {
        Kind::Px => spec.begin,
        Kind::Share => spec.begin * room,
        Kind::Rest => 0.0,
      })
      .collect();
    self.fill_rest(&mut sizes, room);
    sizes
  }

  /// The sizes the panes are laid out at in a room of `room`, by `model`, the sizes as they were last set.
  fn expected(&self, model: &[f32], room: f32) -> Vec<f32> {
    if model.len() != self.specs.len() {
      return self.initial(room);
    }
    let saved = model.iter().sum::<f32>() + self.gaps();
    let mut sizes: Vec<f32> = self
      .specs
      .iter()
      .zip(model)
      .map(|(spec, size)| match spec.kind {
        Kind::Px => *size,
        Kind::Share if saved > 0.0 => size / saved * room,
        _ => *size,
      })
      .collect();
    self.fill_rest(&mut sizes, room);
    sizes
  }

  /// Gives the pane that takes the rest what the others leave of `room`.
  fn fill_rest(&self, sizes: &mut [f32], room: f32) {
    let Some(rest) = self.specs.iter().position(|spec| spec.kind == Kind::Rest) else {
      return;
    };
    let others: f32 = sizes
      .iter()
      .enumerate()
      .filter(|(index, _)| *index != rest)
      .map(|(_, size)| size)
      .sum();
    sizes[rest] = (room - self.gaps() - others).max(0.0);
  }

  /// Keeps each open pane of `sizes` between its least and most size in a room of `room`, the pane that takes the
  /// rest too: what it lacks is taken from the panes nearest it, down to their least. Whether a size changed.
  fn fit(&self, sizes: &mut [f32], room: f32) -> bool {
    let before = sizes.to_vec();
    let open = |sizes: &[f32], index: usize| !(self.specs[index].collapsible && sizes[index] <= 0.0);
    for (index, spec) in self.specs.iter().enumerate() {
      if spec.kind != Kind::Rest && open(sizes, index) {
        sizes[index] = sizes[index].clamp(spec.min, spec.max);
      }
    }
    self.fill_rest(sizes, room);
    if let Some(rest) = self.specs.iter().position(|spec| spec.kind == Kind::Rest) {
      let mut lacks = self.specs[rest].min - sizes[rest];
      // The nearest first, on either side.
      let mut order: Vec<usize> = (0..self.specs.len()).filter(|index| *index != rest).collect();
      order.sort_by_key(|index| index.abs_diff(rest));
      for index in order {
        if lacks <= 0.0 {
          break;
        }
        if !open(sizes, index) {
          continue;
        }
        let gives = (sizes[index] - self.specs[index].min).max(0.0).min(lacks);
        sizes[index] -= gives;
        lacks -= gives;
      }
      self.fill_rest(sizes, room);
    }
    sizes.iter().zip(&before).any(|(now, was)| (now - was).abs() > 0.5)
  }

  /// The size a pane is laid out at: pixels, or a share of the room, or none for the pane that takes the rest.
  fn dimension(&self, index: usize, size: f32, room: f32) -> Option<Dimension> {
    let spec = &self.specs[index];
    if spec.collapsible && size <= 0.0 {
      return Some(Dimension::Px(0.0));
    }
    match spec.kind {
      Kind::Px => Some(Dimension::Px(size)),
      Kind::Share if room > 0.0 => Some(Dimension::Pct(size / room * 100.0)),
      Kind::Share => Some(Dimension::Px(size)),
      Kind::Rest => None,
    }
  }

  /// Lays the panes out at `sizes` without building anything again, and keeps them in the signal.
  fn put(&self, sizes: &[f32]) {
    let room = self.room();
    for (index, size) in sizes.iter().enumerate() {
      let dimension = self.dimension(index, *size, room);
      let (width, height) = match self.orientation {
        SplitterOrientation::Horizontal => (dimension, None),
        SplitterOrientation::Vertical => (None, dimension),
      };
      self.panes[index].set_size(width, height);
      self.shadows[index].set_size(width, height);
    }
    if let Some(host) = &self.host {
      *host.written.lock() = Some(sizes.to_vec());
    }
    self.sizes.set(sizes.to_vec());
  }

  /// Lays the panes out at `sizes` and says so to `on_resize`.
  fn resize(&self, sizes: &[f32]) {
    self.put(sizes);
    for handler in &self.on_resize {
      handler(sizes);
    }
  }

  /// Says to `on_resize_end` that a divider has moved, and to the host.
  fn resized(&self) {
    let sizes = self.sizes.get_untracked();
    for handler in &self.on_resize_end {
      handler(&sizes);
    }
    if let Some(host) = &self.host {
      host.settled();
    }
  }

  /// A divider moved and let go at once: by a key, a double press, Enter.
  fn step_to(&self, sizes: Option<Vec<f32>>) {
    if let Some(sizes) = sizes {
      self.resize(&sizes);
      self.resized();
    }
  }

  /// The sizes of the panes from `start` with the divider after pane `index` moved by `by`: the two panes beside it
  /// change, within their least and most sizes. A collapsible pane dragged under half its least size closes, and a
  /// closed one dragged over half of it opens, where `closes`. `None` where the two cannot both fit.
  fn moved(&self, start: &[f32], index: usize, by: f32, closes: bool) -> Option<Vec<f32>> {
    let (first, second) = (&self.specs[index], &self.specs[index + 1]);
    let total = start[index] + start[index + 1];
    let shut = |spec: &Spec, size: f32| closes && spec.collapsible && size < spec.min / 2.0;
    let mut a = start[index] + by;
    a = if shut(first, a) {
      0.0
    } else {
      a.clamp(first.min, first.max)
    };
    a = a.clamp(0.0, total);
    let mut b = total - a;
    if shut(second, b) && total <= first.max {
      b = 0.0;
    } else if b < second.min {
      b = second.min;
    } else if b > second.max {
      b = second.max;
    }
    a = total - b;
    let a_fits = (a >= first.min - 0.01 && a <= first.max + 0.01) || (first.collapsible && a <= 0.0);
    let b_fits = (b >= second.min - 0.01 && b <= second.max + 0.01) || (second.collapsible && b <= 0.0);
    if !a_fits || !b_fits || a < 0.0 {
      return None;
    }
    let mut sizes = start.to_vec();
    sizes[index] = a;
    sizes[index + 1] = b;
    Some(sizes)
  }

  /// The sizes with the collapsible pane beside the divider after pane `index` closed, or opened to the size it
  /// began with where it is closed.
  fn toggled(&self, index: usize) -> Option<Vec<f32>> {
    let start = self.current();
    let pane = [index, index + 1]
      .into_iter()
      .find(|pane| self.specs[*pane].collapsible)?;
    let other = if pane == index { index + 1 } else { index };
    let total = start[index] + start[index + 1];
    let mut sizes = start.clone();
    if start[pane] > 0.0 {
      sizes[pane] = 0.0;
      sizes[other] = total;
      return Some(sizes);
    }
    let spec = &self.specs[pane];
    let room = total - self.specs[other].min;
    if room < spec.min {
      return None;
    }
    let size = self.initial(self.room())[pane].clamp(spec.min, spec.max).min(room);
    sizes[pane] = size;
    sizes[other] = total - size;
    Some(sizes)
  }

  /// The sizes with the divider after pane `index` back where it began, or the closed pane beside it opened.
  fn restored(&self, index: usize) -> Option<Vec<f32>> {
    let start = self.current();
    let closed = |pane: usize| self.specs[pane].collapsible && start[pane] <= 0.0;
    if closed(index) || closed(index + 1) {
      return self.toggled(index);
    }
    let began = self.initial(self.room())[index];
    self.moved(&start, index, began - start[index], false)
  }

  /// Puts the panes back between their least and most sizes in a room of `room`: the splitter was laid out in
  /// another room. The sizes are written where they were none yet, so that the app has them from the start.
  fn settle(&self, room: f32) {
    if room <= 0.0 || self.drag.lock().is_some() {
      return;
    }
    let model = self.sizes.get_untracked();
    let mut sizes = self.expected(&model, room);
    let changed = self.fit(&mut sizes, room);
    let first = model.len() != self.specs.len();
    if changed {
      self.put(&sizes);
    } else if first {
      if let Some(host) = &self.host {
        *host.written.lock() = Some(sizes.clone());
      }
      self.sizes.set(sizes);
    }
    if (changed || first)
      && let Some(host) = &self.host
    {
      host.settled();
    }
  }

  fn drag_start(&self) {
    *self.drag.lock() = Some(DragStart {
      sizes: self.current(),
      moved: false,
    });
  }

  fn drag_move(&self, index: usize, event: &DragEvent) {
    let by = match self.orientation {
      SplitterOrientation::Horizontal => event.total_delta_x,
      SplitterOrientation::Vertical => event.total_delta_y,
    };
    let sizes = {
      let mut drag = self.drag.lock();
      let Some(start) = drag.as_mut() else {
        return;
      };
      if by != 0.0 {
        start.moved = true;
      }
      self.moved(&start.sizes, index, by, true)
    };
    if let Some(sizes) = sizes
      && sizes != self.sizes.get_untracked()
    {
      self.resize(&sizes);
    }
  }

  fn drag_end(&self) {
    let moved = self.drag.lock().take().is_some_and(|start| start.moved);
    if moved {
      self.resized();
    }
  }

  fn key(&self, index: usize, event: &KeyboardEvent) {
    if !event.focus_within || event.ctrl || event.alt || event.meta || event.composing {
      return;
    }
    let step = if event.shift {
      self.large_step()
    } else {
      self.small_step()
    };
    let (back, forth) = match self.orientation {
      SplitterOrientation::Horizontal => ("ArrowLeft", "ArrowRight"),
      SplitterOrientation::Vertical => ("ArrowUp", "ArrowDown"),
    };
    let start = self.current();
    let sizes = match event.key.as_str() {
      key if key == back => self.moved(&start, index, -step, false),
      key if key == forth => self.moved(&start, index, step, false),
      "Home" => self.moved(&start, index, self.specs[index].min - start[index], false),
      "End" => {
        let most = self.specs[index].max.min(start[index] + start[index + 1]);
        self.moved(&start, index, most - start[index], false)
      }
      "Enter" => self.toggled(index),
      _ => return,
    };
    event.prevent_default();
    self.step_to(sizes);
  }

  fn small_step(&self) -> f32 {
    self.steps.0
  }

  fn large_step(&self) -> f32 {
    self.steps.1
  }
}

/// A splitter as a renderer sees it: what it was given, the style of its
/// parts, and its behavior ready to be dressed.
pub struct SplitterView {
  pub orientation: SplitterOrientation,
  pub grip: bool,
  pub disabled: bool,
  pub label: Option<Arc<str>>,
  /// The style of each part of this splitter.
  pub recipe: ResolvedRecipe,
  specs: Vec<Spec>,
  contents: Vec<Element>,
  keys: Vec<Option<Arc<str>>>,
  sizes: Signal<Vec<f32>>,
  on_resize: Vec<SizesHandler>,
  on_resize_end: Vec<SizesHandler>,
  step: f32,
  large_step: f32,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
  host: Option<Arc<HostLink>>,
}

impl SplitterView {
  /// The style of a part, as this splitter is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// How many panes there are.
  pub fn len(&self) -> usize {
    self.specs.len()
  }

  pub fn is_empty(&self) -> bool {
    self.specs.is_empty()
  }

  /// The sizes of the panes as they were last set: empty until the splitter was first laid out. Read without
  /// subscribing: what renders the splitter is not rendered again as a divider moves.
  pub fn sizes(&self) -> Vec<f32> {
    self.sizes.get_untracked()
  }

  /// A length in pixels the recipe gives `part` along `along`: across a divider for the width of a horizontal
  /// splitter's.
  fn px(&self, part: SplitterPart, across: bool) -> Option<f32> {
    let style = self.part(part).rest();
    let length = match (self.orientation, across) {
      (SplitterOrientation::Horizontal, true) | (SplitterOrientation::Vertical, false) => style.width,
      _ => style.height,
    };
    match length?.resolve(self.theme())? {
      Dimension::Px(px) => Some(px),
      _ => None,
    }
  }

  /// The thickness of a divider: the room it keeps between two panes, the width of its line.
  pub fn thickness(&self) -> f32 {
    self.px(SplitterPart::Line, true).unwrap_or(1.0)
  }

  /// The pane at `index`, sized to `dimension` along the splitter, in the style of its part, with `content` in it.
  fn pane_box(&self, dimension: Option<Dimension>, element_ref: &ElementRefMut) -> Column {
    let mut pane = Column::new()
      .part(&self.part(SplitterPart::Pane))
      .align_items(Alignment::Stretch)
      .ref_element(element_ref.clone());
    pane = match (dimension, self.orientation) {
      (Some(dimension), SplitterOrientation::Horizontal) => pane.width(dimension),
      (Some(dimension), SplitterOrientation::Vertical) => pane.height(dimension),
      (None, _) => pane.flex(1.0),
    };
    pane
  }

  /// The grip in the middle of a divider, its icon along the divider.
  fn grip_part(&self) -> Element {
    let icon = glyph(
      &self.part(SplitterPart::GripIcon),
      Name::new(icons::GRIP_VERTICAL),
      false,
    );
    let icon: Element = match self.orientation {
      SplitterOrientation::Horizontal => icon,
      SplitterOrientation::Vertical => Row::new().transform(Transform2D::rotate_deg(90.0)).child(icon).into(),
    };
    Row::new()
      .part(&self.part(SplitterPart::Grip))
      .align_items(Alignment::Center)
      .justify(crate::layout::layout_kind::Justify::Center)
      .child(icon)
      .into()
  }

  /// How the default renderer builds a splitter: the panes in a row (or a column), the room of a divider between
  /// each two, and over them a copy of the row whose panes take no pointer and whose dividers' handles do, in a room
  /// wider than the line, over the edges of the panes beside them.
  pub fn render(self) -> Element {
    let count = self.specs.len();
    let thickness = self.thickness();
    let handle_across = self.px(SplitterPart::Handle, true).unwrap_or(thickness);
    let model = self.sizes.get_untracked();
    let panes: Vec<ElementRefMut> = (0..count).map(|_| ElementRefMut::new()).collect();
    let shadows: Vec<ElementRefMut> = (0..count).map(|_| ElementRefMut::new()).collect();
    let room = ElementRefMut::new();
    let shared = Arc::new(Panes {
      orientation: self.orientation,
      specs: self.specs.clone(),
      thickness,
      sizes: self.sizes.clone(),
      panes: panes.clone(),
      shadows: shadows.clone(),
      room: room.clone(),
      on_resize: self.on_resize.clone(),
      on_resize_end: self.on_resize_end.clone(),
      drag: Mutex::new(None),
      host: self.host.clone(),
      steps: (self.step, self.large_step),
    });
    {
      let shared = shared.clone();
      room.observe_rect(move |rect| {
        let along = shared.along(rect);
        shared.settle(along);
      });
    }

    // How the panes are laid out to begin with: by the sizes last set, each as it keeps them, or by what they were
    // given.
    let saved = model.iter().sum::<f32>() + thickness * count.saturating_sub(1) as f32;
    let dimensions: Vec<Option<Dimension>> = self
      .specs
      .iter()
      .enumerate()
      .map(|(index, spec)| {
        if model.len() == count {
          shared.dimension(index, model[index], saved)
        } else {
          match spec.kind {
            Kind::Px => Some(Dimension::Px(spec.begin)),
            Kind::Share => Some(Dimension::Pct(spec.begin * 100.0)),
            Kind::Rest => None,
          }
        }
      })
      .collect();

    let horizontal = self.orientation == SplitterOrientation::Horizontal;
    let fill = Dimension::Pct(100.0);
    let mut below: Vec<Element> = Vec::with_capacity(count);
    let mut above: Vec<Element> = Vec::with_capacity(count * 2);
    for (index, content) in self.contents.iter().enumerate() {
      let mut pane = self.pane_box(dimensions[index], &panes[index]).child(content.clone());
      if let Some(key) = &self.keys[index] {
        pane = pane.key(key.clone());
      }
      below.push(pane.into());
      above.push(
        self
          .pane_box(dimensions[index], &shadows[index])
          .pointer_events_none()
          .into(),
      );
      if index + 1 < count {
        above.push(self.divider(index, &shared, thickness, handle_across, &model));
      }
    }
    let (below, above): (Element, Element) = if horizontal {
      (
        Row::new()
          .size(fill, fill)
          .spacing(thickness)
          .align_items(Alignment::Stretch)
          .ref_element(room)
          .with_children(below)
          .into(),
        Row::new()
          .size(fill, fill)
          .align_items(Alignment::Stretch)
          .hit_test(HitTestBehavior::ContentOnly)
          .with_children(above)
          .into(),
      )
    } else {
      (
        Column::new()
          .size(fill, fill)
          .spacing(thickness)
          .align_items(Alignment::Stretch)
          .ref_element(room)
          .with_children(below)
          .into(),
        Column::new()
          .size(fill, fill)
          .align_items(Alignment::Stretch)
          .hit_test(HitTestBehavior::ContentOnly)
          .with_children(above)
          .into(),
      )
    };
    let mut root = Stack::new()
      .part(&self.part(SplitterPart::Root))
      .child(below)
      .child(above);
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root.into()
  }

  /// The divider after pane `index`: a slot as thick as the room between the panes, holding the handle, which is
  /// wider and centered on it, with the line, the bar and the grip in it.
  fn divider(&self, index: usize, shared: &Arc<Panes>, thickness: f32, handle_across: f32, model: &[f32]) -> Element {
    let horizontal = self.orientation == SplitterOrientation::Horizontal;
    let back = -(handle_across - thickness) / 2.0;
    let mut handle = Stack::new()
      .part(&self.part(SplitterPart::Handle))
      .key(format!("divider-{index}"))
      .child(Row::new().part(&self.part(SplitterPart::Line)))
      .child(Row::new().part(&self.part(SplitterPart::Bar)))
      .describe("a11y_role", "separator")
      .describe("a11y_orientation", self.orientation.divider());
    if horizontal {
      handle = handle.offset(back, 0.0);
    } else {
      handle = handle.offset(0.0, back);
    }
    if self.grip {
      handle = handle.child(self.grip_part());
    }
    let spec = &self.specs[index];
    if let Some(label) = spec.label.clone().or_else(|| self.label.clone()) {
      handle = handle.describe("a11y_name", label);
    }
    if let Some(size) = model.get(index) {
      handle = handle.describe("a11y_valuenow", format!("{}", size.round()));
    }
    handle = handle.describe("a11y_valuemin", format!("{}", spec.min.round()));
    if spec.max.is_finite() {
      handle = handle.describe("a11y_valuemax", format!("{}", spec.max.round()));
    }
    if let Some(id) = &self.id {
      handle = handle.id(format!("{id}-divider-{index}"));
    }
    if self.disabled {
      handle = handle.disabled(true);
    } else {
      // A divider is a stop of Tab wherever it is, in the order of the tree.
      handle = handle
        .tab_index(0)
        .start_drag_buttons(MouseButtonMask::LEFT)
        .on_drag_start({
          let shared = shared.clone();
          move |_: DragEvent| shared.drag_start()
        })
        .on_drag_move({
          let shared = shared.clone();
          move |event: DragEvent| shared.drag_move(index, &event)
        })
        .on_drag_end({
          let shared = shared.clone();
          move |_: DragEvent| shared.drag_end()
        })
        .on_dblclick({
          let shared = shared.clone();
          move |_: MouseEvent| shared.step_to(shared.restored(index))
        })
        .on_key_down({
          let shared = shared.clone();
          move |event: KeyboardEvent| shared.key(index, &event)
        });
    }
    // A row lays its child out as long as it is along it, a column as high: the handle stands out of the slot on
    // both sides of the line.
    if horizontal {
      Row::new()
        .width(thickness)
        .hit_test(HitTestBehavior::ContentOnly)
        .overflow_visible()
        .child(handle)
        .into()
    } else {
      Column::new()
        .height(thickness)
        .hit_test(HitTestBehavior::ContentOnly)
        .overflow_visible()
        .child(handle)
        .into()
    }
  }
}

impl Themed for Splitter {
  const NAME: &'static str = "splitter";
  type Renderer = SplitterRenderer;

  /// The design's splitter (`pencil/lurq-design-system.pen`, Splitter Handle and Resizable): panes with a line of 1
  /// between them in the color of an edge; under the pointer the line is 3 wide in the ring's color, and dragged in
  /// the primary color. The Resizable's grip is 12 by 20 in the middle of the line, round by the small radius, in
  /// the color of an edge, with a grip icon of 10 on it. Framed, the splitter is the board's specimen: the
  /// background's fill, an edge of 1 and the large radius. The room that takes the pointer is 12 across the line.
  fn recipe() -> Recipe {
    use PaletteColor::*;
    use SplitterPart::{Bar, Grip, GripIcon, Handle, Line, Root};
    let clear = Color::new(0, 0, 0, 0);
    let disabled = || Selector::new().state(SplitterState::Disabled);
    Recipe::new()
      .base(Root, |s| s.width(Length::Fill).height(Length::Fill))
      .base(Handle, |s| {
        s.width(12.0)
          .height(Length::Fill)
          .foreground(clear)
          .cursor(CursorIcon::ColResize)
      })
      .base(Line, |s| {
        s.width(1.0).height(Length::Fill).background(PaletteColor::Border)
      })
      .base(Bar, |s| {
        s.width(3.0)
          .height(Length::Fill)
          .background(BackgroundColor::CurrentForeground)
      })
      .base(Grip, |s| {
        s.width(12.0)
          .height(20.0)
          .radius(RadiusSize::Sm)
          .background(PaletteColor::Border)
      })
      .base(GripIcon, |s| s.width(10.0).foreground(Foreground))
      .hovered(Handle, |s| s.foreground(Ring))
      .pressed(Handle, |s| s.foreground(Primary))
      .focus_visible(Handle, |s| s.foreground(Ring))
      .state(SplitterState::Vertical, Handle, |s| {
        s.width(Length::Fill).height(12.0).cursor(CursorIcon::RowResize)
      })
      .state(SplitterState::Vertical, Line, |s| s.width(Length::Fill).height(1.0))
      .state(SplitterState::Vertical, Bar, |s| s.width(Length::Fill).height(3.0))
      .state(SplitterState::Vertical, Grip, |s| s.width(20.0).height(12.0))
      .state(SplitterState::Framed, Root, |s| {
        s.background(Background)
          .border(edge(PaletteColor::Border))
          .radius(RadiusSize::Lg)
      })
      .state(SplitterState::Disabled, Handle, |s| s.cursor(CursorIcon::Default))
      .when(disabled().hovered(), Handle, |s| s.foreground(clear))
      .when(disabled().pressed(), Handle, |s| s.foreground(clear))
  }
}

/// What a mounted splitter's host and the splitter it builds share.
struct HostLink {
  /// The sizes the splitter wrote last: a write of other sizes is the app's.
  written: Mutex<Option<Vec<f32>>>,
  /// Renders the host again: once a divider is let go, so that tooling reads the sizes it was let go at.
  revision: Signal<u64>,
}

impl HostLink {
  fn settled(&self) {
    self.revision.update(|revision| *revision += 1);
  }
}

/// A mounted splitter: the splitter it is given, built anew by every render of what holds it.
#[derive(Clone)]
struct SplitterProps {
  splitter: Arc<Mutex<Splitter>>,
}

/// A splitter is built anew by every render of what holds it, so props are never the same as before.
impl PartialEq for SplitterProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

#[cfg(feature = "devtools")]
impl crate::app::component::DevtoolsInspectable for SplitterProps {}

/// What keeps a mounted splitter's sizes, and renders it again when the app sets them.
struct SplitterHost {
  /// The sizes where the app gives no signal for them.
  own: Signal<Vec<f32>>,
  link: Arc<HostLink>,
  /// The signal of the sizes the host listens to, by its id.
  watched: Mutex<Option<usize>>,
}

impl SplitterHost {
  /// Listens to `sizes`, where it is another signal than before: a write that is not the splitter's renders the
  /// host again.
  fn watch(&self, ctx: &mut Ctx, sizes: &Signal<Vec<f32>>) {
    let mut watched = self.watched.lock();
    if *watched == Some(sizes.id()) {
      return;
    }
    *watched = Some(sizes.id());
    let link = self.link.clone();
    ctx.watch(sizes, move |value: &Vec<f32>| {
      let ours = link.written.lock().as_ref() == Some(value);
      if !ours {
        link.settled();
      }
    });
  }
}

impl Component for SplitterHost {
  type Props = SplitterProps;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      own: Signal::new(Vec::new()),
      link: Arc::new(HostLink {
        written: Mutex::new(None),
        revision: ctx.signal(0_u64),
      }),
      watched: Mutex::new(None),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let mut splitter = ctx.props::<SplitterProps>().splitter.lock().clone();
    let sizes = splitter.sizes.clone().unwrap_or_else(|| self.own.clone());
    self.watch(ctx, &sizes);
    splitter.sizes = Some(sizes);
    splitter.host = Some(self.link.clone());
    splitter.build(&current_theme())
  }
}
