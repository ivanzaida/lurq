//! A tooltip: a short text beside an element while the pointer rests on it or
//! the keyboard's focus is on it.
//!
//! It is a mounted component, not a builder that converts into an element:
//! it keeps a timer for its delay, an anchor that is the same from one render
//! to the next, and whether the pointer has shown it, none of which a builder
//! can keep. [`Tooltip`] says what it is, and [`Tooltip::mount`] puts it on
//! the element it describes.

use std::{
  cell::{Cell, RefCell},
  f32::consts::SQRT_2,
  sync::Arc,
  time::{Duration, Instant},
};

use parking_lot::Mutex;

use super::{
  FontFamily, names,
  part_style::Length,
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    component::Component,
    ctx::{CollisionStrategy, Ctx, Overlay, Placement, Timeout},
    events::MouseEvent,
    theme::{PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::{Column, Row},
  core::{ElementRect, ElementRef, Signal},
  layout::{
    Alignment,
    layout_kind::{Justify, LayoutKind},
  },
  node::{Element, HitTestBehavior, Node, NodeUpdate, color::Color, dimension::Dimension, transform::Transform2D},
};

/// How long the pointer rests on an element before its tooltip shows, as a
/// Markdown link's does.
pub const TOOLTIP_DELAY: Duration = Duration::from_millis(500);

/// How soon after a tooltip went another shows with no delay: the pointer
/// moving along a toolbar shows each button's at once.
pub const TOOLTIP_SKIP_DELAY: Duration = Duration::from_millis(300);

thread_local! {
  /// Until when a tooltip that shows skips its delay: a tooltip went just
  /// before. A window's tooltips are all on its thread.
  static WARM_UNTIL: Cell<Option<Instant>> = const { Cell::new(None) };
}

/// Whether a tooltip went so lately that the next shows at once.
fn warm() -> bool {
  WARM_UNTIL.with(|until| until.get().is_some_and(|until| Instant::now() < until))
}

/// A tooltip went, or the pointer left the element of one that shows: the
/// next to show within the skip delay shows at once.
fn went() {
  WARM_UNTIL.with(|until| until.set(Some(Instant::now() + TOOLTIP_SKIP_DELAY)));
}

/// What puts away a tooltip the pointer showed.
type Hide = Arc<dyn Fn() + Send + Sync>;

thread_local! {
  /// The tooltip the pointer showed last, by its host, and what puts it away.
  static SHOWING: RefCell<Option<(usize, Hide)>> = const { RefCell::new(None) };
}

/// The pointer showed the tooltip of the host `id`: the one it showed before,
/// still there for its grace, goes, so that one tooltip shows at a time.
fn take_turn(id: usize, hide: &Hide) {
  let before = SHOWING.with(|showing| showing.borrow_mut().replace((id, hide.clone())));
  if let Some((before, hide)) = before
    && before != id
  {
    hide();
  }
}

/// Sets `signal` to `value` if it is not that already: a signal set tells
/// what reads it even of the same value.
fn put(signal: &Signal<bool>, value: bool) {
  if signal.get_untracked() != value {
    signal.set(value);
  }
}

/// The parts of a tooltip a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TooltipPart {
  /// The box and its arrow, and the room between them and what they describe.
  Root,
  /// The box: its text and its shortcut, on the primary color.
  Content,
  /// Its text.
  Label,
  /// The keys of a shortcut after the text, in a chip of their own.
  Shortcut,
  /// What the arrow is drawn in: the triangle between the box and what it
  /// describes, in the color of the box.
  Arrow,
}

names!(TooltipPart {
  Root => "root",
  Content => "content",
  Label => "label",
  Shortcut => "shortcut",
  Arrow => "arrow",
});

/// The side of what it describes a tooltip is on. Where there is no room on
/// that side, it is on the other one, its arrow turned with it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TooltipSide {
  /// Above it, its arrow pointing down.
  #[default]
  Top,
  Bottom,
  Left,
  Right,
}

names!(TooltipSide {
  Top => "top",
  Bottom => "bottom",
  Left => "left",
  Right => "right",
});

impl TooltipSide {
  /// The overlay's placement of this side: centered on what it describes.
  fn placement(self) -> Placement {
    match self {
      Self::Top => Placement::Top,
      Self::Bottom => Placement::Bottom,
      Self::Left => Placement::Left,
      Self::Right => Placement::Right,
    }
  }

  /// Whether the tooltip is above or below what it describes.
  fn vertical(self) -> bool {
    matches!(self, Self::Top | Self::Bottom)
  }

  fn opposite(self) -> Self {
    match self {
      Self::Top => Self::Bottom,
      Self::Bottom => Self::Top,
      Self::Left => Self::Right,
      Self::Right => Self::Left,
    }
  }
}

/// The states of a tooltip a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TooltipState {
  /// It has an arrow.
  Arrow,
  /// It shows a shortcut after its text.
  Shortcut,
  /// Its text wraps: it is longer than the box is wide. Known once the text
  /// is laid out, so a tooltip shown on an element; one anchored to nothing
  /// is never in it.
  Multiline,
}

names!(TooltipState {
  Arrow => "arrow",
  Shortcut => "shortcut",
  Multiline => "multiline",
});

/// What builds a tooltip's nodes: a theme's in place of the default one. It
/// builds what is shown, not when.
pub type TooltipRenderer = Arc<dyn Fn(TooltipView) -> Element + Send + Sync>;

/// A tooltip: a short text that says what an element does, beside it while
/// the pointer rests on it or the keyboard's focus is on it. An icon button's
/// "Copy", a field's "Stored in the OS keychain".
///
/// ```no_run
/// use lurq::{app::ctx::Ctx, node::Element, ui::{Button, Tooltip, TooltipSide}};
///
/// fn toolbar(ctx: &mut Ctx) -> Element {
///   let copy = Button::new("Copy").icon("copy").icon_only(true);
///   Tooltip::new("Copy").shortcut("Ctrl+C").side(TooltipSide::Bottom).mount(ctx, copy)
/// }
/// ```
///
/// It shows after the pointer has rested on the element for its delay, at
/// once if another tooltip went a moment before, and at once when the focus
/// comes to the element, or to anything in it (a button in a row), from
/// the keyboard. It goes when the pointer has left both the element and
/// the tooltip for a moment (the pointer can be moved onto it to read it),
/// when the focus leaves, when the element is pressed, and on Escape, which
/// it takes before anything under it. One tooltip of the pointer shows at a
/// time. It takes no focus, and it stands beside the element, over it only
/// where the window has room on neither side, as it is kept in the window.
/// A disabled element shows its tooltip too. Its text is what tooling that
/// reads the tree is told the element is described by, and what the element
/// is called if nothing else names it.
///
/// Converted into an element on its own, a tooltip is its box as it shows,
/// anchored to nothing: for a gallery, a test, or a host of an app's own.
pub struct Tooltip {
  text: Arc<str>,
  shortcut: Option<Arc<str>>,
  side: TooltipSide,
  arrow: bool,
  delay: Duration,
  open: Option<bool>,
  disabled: bool,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Tooltip {
  /// A tooltip that says `text`, above what it describes, with no arrow.
  pub fn new(text: &str) -> Self {
    Self {
      text: Arc::from(text),
      shortcut: None,
      side: TooltipSide::Top,
      arrow: false,
      delay: TOOLTIP_DELAY,
      open: None,
      disabled: false,
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// The keys of a shortcut that does what the element does, after the text:
  /// "Ctrl+B".
  pub fn shortcut(mut self, keys: &str) -> Self {
    self.shortcut = (!keys.is_empty()).then(|| Arc::from(keys));
    self
  }

  /// The side of what it describes the tooltip is on, where there is room.
  pub fn side(mut self, side: TooltipSide) -> Self {
    self.side = side;
    self
  }

  /// Whether a triangle points from the box to what it describes.
  pub fn arrow(mut self, arrow: bool) -> Self {
    self.arrow = arrow;
    self
  }

  /// How long the pointer rests on the element before the tooltip shows;
  /// [`TOOLTIP_DELAY`] unless this says otherwise. The keyboard's focus shows
  /// it at once.
  pub fn delay(mut self, delay: Duration) -> Self {
    self.delay = delay;
    self
  }

  /// Shows the tooltip whatever the pointer and the focus do (`Some(true)`),
  /// or never (`Some(false)`); `None`, as it starts, leaves it to them. For a
  /// gallery, a test, a tour of the app.
  pub fn open(mut self, open: Option<bool>) -> Self {
    self.open = open;
    self
  }

  /// Whether the tooltip never shows: the element is as it would be without it.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Puts the tooltip in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this tooltip over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this tooltip: what the component
  /// it is in says of the tooltip in this place.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// The id of the tooltip's root: what shows.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Puts the tooltip on `element`, the element it describes, and returns
  /// that element. The pointer and the focus are the element's, or those of
  /// anything in it.
  pub fn mount(self, ctx: &mut Ctx, element: impl Into<Element>) -> Element {
    ctx.mount::<TooltipHost>(TooltipProps {
      tooltip: Arc::new(self),
      child: element.into(),
    })
  }

  /// The tooltip as it is drawn on `side`, in `theme`, its text wrapped or not.
  fn view(&self, theme: &Theme, side: TooltipSide, multiline: bool) -> TooltipView {
    let case = Case {
      variant: Some(side.into()),
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(TooltipState::Arrow, self.arrow)
    .state(TooltipState::Shortcut, self.shortcut.is_some())
    .state(TooltipState::Multiline, multiline);
    TooltipView {
      text: self.text.clone(),
      shortcut: self.shortcut.clone(),
      side,
      arrow: self.arrow,
      aim: 0.0,
      recipe: theme.resolve::<Tooltip>(&case, Some(&self.rules)),
      label_ref: None,
      id: self.id.clone(),
      key: self.key.clone(),
    }
  }

  /// Builds what `view` shows: the theme's renderer, or the default one.
  fn render_view(view: TooltipView) -> Element {
    match view.theme().renderer::<Tooltip>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }

  /// Builds the tooltip's box as it shows, anchored to nothing, in `theme`.
  /// Converting a tooltip into an element does this in the theme of what is
  /// being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    Self::render_view(self.view(theme, self.side, false))
  }
}

impl From<Tooltip> for Element {
  fn from(tooltip: Tooltip) -> Self {
    tooltip.build(&current_theme())
  }
}

/// A tooltip as a renderer sees it: what it says, the side it is on, and the
/// style of its parts.
pub struct TooltipView {
  pub text: Arc<str>,
  pub shortcut: Option<Arc<str>>,
  /// The side it is on: the side it was given, or the other where there was no room.
  pub side: TooltipSide,
  pub arrow: bool,
  /// How far along the box's edge from its middle the arrow is, toward the
  /// middle of what the tooltip describes: not 0 where the box was pushed
  /// along to stay in the window. Never so far that the arrow leaves the box.
  pub aim: f32,
  /// The style of each part of this tooltip.
  pub recipe: ResolvedRecipe,
  /// What the host reads the label's height by, to know whether it wraps.
  label_ref: Option<ElementRef>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl TooltipView {
  /// The style of a part, as this tooltip is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The box: the text, and the shortcut after it.
  pub fn content(&self) -> Row {
    let label = self.part(TooltipPart::Label);
    let mut content = Row::new()
      .align_items(Alignment::Center)
      .part(&self.part(TooltipPart::Content));
    // The text gives way to the box's greatest width, and wraps.
    let mut words = label.text(&self.text).flex_shrink(1.0);
    if let Some(label_ref) = &self.label_ref {
      words = words.ref_element(label_ref.clone());
    }
    content = content.child(words);
    if let Some(keys) = &self.shortcut {
      let part = self.part(TooltipPart::Shortcut);
      content = content.child(
        Row::new()
          .align_items(Alignment::Center)
          .flex_shrink(0.0)
          .part(&part)
          .child(part.text(keys).nowrap()),
      );
    }
    content
  }

  /// The arrow: a square in the color of the box, turned a quarter of a
  /// right angle, its middle on the edge of the arrow's part that meets the
  /// box, so that the part, which clips, shows the half that points away from
  /// it. `None` without an arrow, and where the recipe gives the arrow's part
  /// no size in pixels or the box no color.
  pub fn arrow(&self) -> Option<Element> {
    let (along, away) = self.arrow_size()?;
    let part = self.part(TooltipPart::Arrow);
    let fill = self.part(TooltipPart::Content).rest().background?;
    let vertical = self.side.vertical();
    let side = along / SQRT_2;
    // The arrow's part is on the side of the box toward what it describes: its edge that meets the box is the
    // near one above or left of what it describes, the far one below or right of it.
    let shift = match self.side {
      TooltipSide::Top | TooltipSide::Left => -away / 2.0,
      TooltipSide::Bottom | TooltipSide::Right => away / 2.0,
    };
    let (dx, dy) = if vertical { (0.0, shift) } else { (shift, 0.0) };
    let tip = Row::new()
      .width(side)
      .height(side)
      .flex_shrink(0.0)
      .background(fill)
      .offset(dx, dy)
      .transform(Transform2D::rotate_deg(45.0));
    // The part's long side is its main axis, so the square is not shrunk to its short one. It is moved along
    // the box by the aim, not laid out there: the box's size does not change with it.
    Some(if vertical {
      Row::new()
        .justify(Justify::Center)
        .align_items(Alignment::Center)
        .part(&part)
        .offset(self.aim, 0.0)
        .child(tip)
        .into()
    } else {
      Column::new()
        .justify(Justify::Center)
        .align_items(Alignment::Center)
        .part(&part)
        .offset(0.0, self.aim)
        .child(tip)
        .into()
    })
  }

  /// How long the arrow is along the box's edge, and how far out from it it
  /// goes: the size of the arrow's part. `None` without an arrow, and where
  /// the recipe gives the part no size in pixels.
  pub fn arrow_size(&self) -> Option<(f32, f32)> {
    if !self.arrow {
      return None;
    }
    let theme = self.theme();
    let px = |length: Option<Length>| match length?.resolve(theme)? {
      Dimension::Px(px) => Some(px),
      _ => None,
    };
    let rest = self.part(TooltipPart::Arrow).rest();
    let (width, height) = (px(rest.width)?, px(rest.height)?);
    Some(if self.side.vertical() {
      (width, height)
    } else {
      (height, width)
    })
  }

  /// How the default renderer builds a tooltip: the box, and the arrow on
  /// the side of it toward what it describes.
  pub fn render(self) -> Element {
    let (content, arrow) = (self.content(), self.arrow());
    let root = self.part(TooltipPart::Root);
    let toward_after = matches!(self.side, TooltipSide::Top | TooltipSide::Left);
    let mut children: Vec<Element> = vec![content.into()];
    if let Some(arrow) = arrow {
      if toward_after {
        children.push(arrow);
      } else {
        children.insert(0, arrow);
      }
    }
    let mut root: Element = if self.side.vertical() {
      Column::new()
        .align_items(Alignment::Center)
        .part(&root)
        .describe("a11y_role", "tooltip")
        .with_children(children)
        .into()
    } else {
      Row::new()
        .align_items(Alignment::Center)
        .part(&root)
        .describe("a11y_role", "tooltip")
        .with_children(children)
        .into()
    };
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root
  }
}

impl Themed for Tooltip {
  const NAME: &'static str = "tooltip";
  type Renderer = TooltipRenderer;

  /// The design's tooltip (`pencil/lurq-design-system.pen`, Tooltip and
  /// Tooltip/Arrow): a box in the primary color with a radius of 6, its text
  /// in the primary's foreground in 12, 10 from its sides and 6 from its top
  /// and bottom, and no wider than 200, where a longer text wraps. A shortcut
  /// is 8 after the text, in a chip of 16 high, 4 from its sides, with a
  /// radius of 3, in a white of 15%, its keys in the mono family in 10. The
  /// arrow is 12 along the box and 6 out from it. It stands 6 from what it
  /// describes. A text that wraps has 8 above and below it, in lines of 1.4.
  fn recipe() -> Recipe {
    use PaletteColor::{Primary, PrimaryForeground};
    use TooltipPart::{Arrow, Content, Label, Root, Shortcut};
    use TooltipSide::{Bottom, Left, Right, Top};
    Recipe::new()
      .base(Content, |s| {
        s.background(Primary)
          .foreground(PrimaryForeground)
          .radius(RadiusSize::Md)
          .padding_x(10.0)
          .padding_y(6.0)
          .gap(8.0)
          .max_width(200.0)
      })
      .base(Label, |s| s.typography(TypographyStyle::Caption))
      .base(Shortcut, |s| {
        s.height(16.0)
          .padding_x(4.0)
          .radius(3.0)
          .background(Color::from_hex("#FFFFFF26"))
          .font_family(FontFamily::Mono)
          .font_size(10.0)
      })
      .base(Arrow, |s| s.width(12.0).height(6.0))
      // A text that wraps: 8 above and below it, in lines of 1.4.
      .state(TooltipState::Multiline, Content, |s| s.padding_y(8.0))
      .state(TooltipState::Multiline, Label, |s| s.line_height(1.4))
      // The room between the tooltip and what it describes, on both sides of it so that it stays centered.
      .variant(Top, Root, |s| s.padding_y(6.0))
      .variant(Bottom, Root, |s| s.padding_y(6.0))
      .variant(Left, Root, |s| s.padding_x(6.0))
      .variant(Right, Root, |s| s.padding_x(6.0))
      .variant(Left, Arrow, |s| s.width(6.0).height(12.0))
      .variant(Right, Arrow, |s| s.width(6.0).height(12.0))
  }
}

/// A tooltip and the element it describes.
#[derive(Clone)]
struct TooltipProps {
  tooltip: Arc<Tooltip>,
  child: Element,
}

/// The element is built anew by every render of what holds it, so props are
/// never the same as before.
impl PartialEq for TooltipProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

#[cfg(feature = "devtools")]
impl crate::app::component::DevtoolsInspectable for TooltipProps {}

/// How long a tooltip stays after the pointer left the element or the
/// tooltip: time to cross the room between them, so that the pointer can be
/// moved onto the tooltip to read it, and off it, without it going.
const TOOLTIP_GRACE: Duration = Duration::from_millis(100);

/// Where the pointer is: on the element, on the tooltip.
#[derive(Default)]
struct Pointer {
  on_element: bool,
  on_tooltip: bool,
}

/// What shows a tooltip on the element it describes, and when.
struct TooltipHost {
  /// Whether the pointer has rested on the element for the delay, and not left both it and the tooltip since.
  hovered: Signal<bool>,
  /// Whether the tooltip may show: Escape (the runtime closes the top layer that takes it) or a press of the
  /// element turns it off, and it stays off until the pointer comes again, or until nothing asks for the
  /// tooltip (the pointer and the focus both gone), when a new one takes its place.
  allowed: Mutex<Signal<bool>>,
  /// Whether the tooltip was put on the other side, where there was no room on its own: its arrow turns with it.
  flipped: Signal<bool>,
  /// How far along the box the arrow is moved toward the element's middle, where the box was pushed along.
  aim: Signal<f32>,
  /// Whether the text wraps.
  multiline: Signal<bool>,
  /// What the readers of where the tooltip was put need from the last render: the side it was given, the
  /// element's anchor, the arrow's length along the box and the label's font size.
  placing: Arc<Mutex<Placing>>,
  pointer: Arc<Mutex<Pointer>>,
  /// The delay before the pointer shows it.
  delay: Timeout,
  /// The grace after the pointer left before it goes.
  grace: Timeout,
  /// The element's anchor where the element has none of its own.
  anchor: ElementRef,
  /// What shows, and its label.
  shown: ElementRef,
  label: ElementRef,
  /// Who this host is among the tooltips the pointer shows, and what puts its tooltip away.
  id: usize,
  hide: Hide,
}

/// See [`TooltipHost::placing`].
struct Placing {
  side: TooltipSide,
  anchor: ElementRef,
  along: f32,
  font_size: f32,
}

impl TooltipHost {
  /// Reads where the overlay put what shows: on which side of the element, and how far along from the
  /// element's middle; the overlay does not say.
  fn read_placement(placing: &Mutex<Placing>, rect: ElementRect, flipped: &Signal<bool>, aim: &Signal<f32>) {
    // Read and let go before a signal is set: what the signal tells may read it too.
    let (side, anchor, along) = {
      let placing = placing.lock();
      (placing.side, placing.anchor.clone(), placing.along)
    };
    let ((x, y), (anchor_x, anchor_y)) = (rect.center(), anchor.bounds().center());
    let other = match side {
      TooltipSide::Top => y > anchor_y,
      TooltipSide::Bottom => y < anchor_y,
      TooltipSide::Left => x > anchor_x,
      TooltipSide::Right => x < anchor_x,
    };
    put(flipped, other);
    // The root is as long as the box along the side's edge: its room is on the other axis.
    let (off, length) = if side.vertical() {
      (anchor_x - x, rect.width)
    } else {
      (anchor_y - y, rect.height)
    };
    let limit = (length / 2.0 - along).max(0.0);
    let off = (off.clamp(-limit, limit) * 2.0).round() / 2.0;
    if (aim.get_untracked() - off).abs() > 0.25 {
      aim.set(off);
    }
  }
}

impl Component for TooltipHost {
  type Props = TooltipProps;

  fn create(ctx: &mut Ctx) -> Self {
    let hovered = ctx.signal(false);
    let pointer = Arc::new(Mutex::new(Pointer::default()));
    let id = Arc::as_ptr(&pointer) as usize;
    let hide: Hide = {
      let hovered = hovered.clone();
      Arc::new(move || put(&hovered, false))
    };
    let delay = {
      let (hovered, hide) = (hovered.clone(), hide.clone());
      ctx.create_timeout(TOOLTIP_DELAY, move || {
        put(&hovered, true);
        take_turn(id, &hide);
      })
    };
    let grace = {
      let (hovered, pointer) = (hovered.clone(), pointer.clone());
      ctx.create_timeout(TOOLTIP_GRACE, move || {
        let pointer = pointer.lock();
        if !pointer.on_element && !pointer.on_tooltip && hovered.get_untracked() {
          hovered.set(false);
          went();
        }
      })
    };
    let (anchor, shown, label) = (ElementRef::new(), ElementRef::new(), ElementRef::new());
    let (flipped, aim, multiline) = (ctx.signal(false), ctx.signal(0.0_f32), ctx.signal(false));
    let placing = Arc::new(Mutex::new(Placing {
      side: TooltipSide::Top,
      anchor: anchor.clone(),
      along: 0.0,
      font_size: 0.0,
    }));
    {
      let (placing, flipped, aim) = (placing.clone(), flipped.clone(), aim.clone());
      shown.observe_rect(move |rect| Self::read_placement(&placing, rect, &flipped, &aim));
    }
    {
      // A label more than about a line and a half high wraps. The lines of a wrapped text are taller, so a text
      // that wraps stays wrapped, and one that does not is never as high.
      let (placing, multiline) = (placing.clone(), multiline.clone());
      label.observe_rect(move |rect| {
        let font_size = placing.lock().font_size;
        put(&multiline, font_size > 0.0 && rect.height > font_size * 1.6);
      });
    }
    Self {
      hovered,
      allowed: Mutex::new(ctx.signal(true)),
      flipped,
      aim,
      multiline,
      placing,
      pointer,
      delay,
      grace,
      anchor,
      shown,
      label,
      id,
      hide,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let TooltipProps { tooltip, child } = ctx.props::<TooltipProps>().clone();
    if tooltip.disabled {
      return child;
    }
    self.delay.set_duration(tooltip.delay);
    // A disabled element is not hovered, the elements it is in are: it is put in a row the pointer reaches.
    let child = if child.node.is_disabled() {
      hoverable(child)
    } else {
      holds_children(child)
    };
    // An element that has an anchor of its own keeps it, and the tooltip is anchored to it.
    let anchor = child.node.element_ref.clone().unwrap_or_else(|| self.anchor.clone());

    // Reading them subscribes: this renders again when the pointer, the focus or Escape changes them. The focus
    // is the element's or that of anything in it, a button in a row.
    let asked = self.hovered.get() || anchor.focus_visible_within();
    let allowed = {
      let mut allowed = self.allowed.lock();
      if !asked {
        // Nothing asks for it: whatever put it away is over, and the next time it is asked for it shows.
        *allowed = Signal::new(true);
      }
      allowed.clone()
    };
    let side = if self.flipped.get() {
      tooltip.side.opposite()
    } else {
      tooltip.side
    };
    let mut view = tooltip.view(&current_theme(), side, self.multiline.get());
    view.aim = self.aim.get();
    view.label_ref = Some(self.label.clone());
    {
      let mut placing = self.placing.lock();
      placing.side = tooltip.side;
      placing.anchor = anchor.clone();
      placing.along = view.arrow_size().map_or(0.0, |(along, _)| along);
      placing.font_size = view
        .part(TooltipPart::Label)
        .rest()
        .text
        .resolve(view.theme())
        .font_size;
    }
    let shows = !tooltip.text.is_empty() && tooltip.open.unwrap_or(asked);
    // A tooltip that went from under the pointer is told of no leave: the pointer is on it no longer.
    if !shows || (tooltip.open.is_none() && !allowed.get()) {
      self.pointer.lock().on_tooltip = false;
    }
    let overlay = if shows {
      let tooltip_enter = {
        let (pointer, grace) = (self.pointer.clone(), self.grace.clone());
        move || {
          pointer.lock().on_tooltip = true;
          grace.cancel();
        }
      };
      let tooltip_leave = {
        let (pointer, grace, hovered) = (self.pointer.clone(), self.grace.clone(), self.hovered.clone());
        move || {
          pointer.lock().on_tooltip = false;
          if hovered.get_untracked() {
            grace.restart();
          }
        }
      };
      let shown = Tooltip::render_view(view).map_node(|node| {
        node
          .ref_element(self.shown.clone())
          .on_mouse_enter(tooltip_enter)
          .on_mouse_leave(tooltip_leave)
      });
      let overlay = Overlay::new(shown)
        .anchor(anchor.clone())
        .placement(tooltip.side.placement())
        .collision(CollisionStrategy::FlipThenClamp)
        // The pointer can be moved onto the tooltip, its root and the room in it toward the element included (with
        // `ContentOnly` the root, which holds the handlers, would not be hit). It stands beside the element, so it
        // covers nothing of it.
        .hit_test(HitTestBehavior::Auto);
      match tooltip.open {
        // Shown by the app: Escape is not the tooltip's to take.
        Some(_) => overlay,
        // The runtime turns `allowed` off on an Escape while this is the top layer that takes it, so the key
        // closes the tooltip and not the dialog under it.
        None => overlay.open(allowed.clone()).dismiss_on_escape(true),
      }
    } else {
      Overlay::new(Element::new()).open_when(false)
    };
    // An element is not shared between threads as it is: the decoration, which is, holds it in a lock.
    let overlay = Arc::new(Mutex::new(Element::from(overlay)));

    let enter = {
      let (hovered, allowed, delay, grace, pointer, wait) = (
        self.hovered.clone(),
        allowed.clone(),
        self.delay.clone(),
        self.grace.clone(),
        self.pointer.clone(),
        tooltip.delay,
      );
      let (id, hide) = (self.id, self.hide.clone());
      move || {
        pointer.lock().on_element = true;
        grace.cancel();
        put(&allowed, true);
        if hovered.get_untracked() {
          return;
        }
        if wait.is_zero() || warm() {
          delay.cancel();
          hovered.set(true);
          take_turn(id, &hide);
        } else {
          delay.restart();
        }
      }
    };
    let leave = {
      let (hovered, allowed, delay, grace, pointer) = (
        self.hovered.clone(),
        allowed.clone(),
        self.delay.clone(),
        self.grace.clone(),
        self.pointer.clone(),
      );
      move || {
        pointer.lock().on_element = false;
        delay.cancel();
        if !hovered.get_untracked() {
          return;
        }
        if allowed.get_untracked() {
          // It stays for its grace, and the next tooltip along shows at once.
          grace.restart();
          went();
        } else {
          // Put away already: nothing is there to move onto.
          hovered.set(false);
        }
      }
    };
    // A press puts it away until the pointer comes again or the focus leaves.
    let press = {
      let (hovered, allowed, delay, grace) = (
        self.hovered.clone(),
        allowed.clone(),
        self.delay.clone(),
        self.grace.clone(),
      );
      move |_: MouseEvent| {
        delay.cancel();
        grace.cancel();
        if hovered.get_untracked() && allowed.get_untracked() {
          went();
        }
        put(&hovered, false);
        put(&allowed, false);
      }
    };

    let text = tooltip.text.clone();
    let describe = move |node: Node| {
      // What tooling is told is kept only where there is tooling to tell.
      #[cfg(any(feature = "devtools", feature = "mcp"))]
      let named = node.debug_attrs().iter().any(|(name, _)| &**name == "a11y_name");
      #[cfg(not(any(feature = "devtools", feature = "mcp")))]
      let named = true;
      let mut node = node
        .ref_element(anchor.clone())
        .on_mouse_enter(enter.clone())
        .on_mouse_leave(leave.clone())
        .on_mouse_down(press.clone());
      NodeUpdate::describe(&mut node, Arc::from("a11y_description"), text.clone());
      if !named {
        NodeUpdate::describe(&mut node, Arc::from("a11y_name"), text.clone());
      }
      // The overlay is declared in the element: a layer is found among the children of a container.
      NodeUpdate::child(&mut node, overlay.lock().clone().into_node());
      node
    };
    child.decorate_root(describe)
  }
}

/// A disabled `element` in a row of its own, which the pointer hovers: a disabled element is not hovered, the
/// elements it is in are. The row is as large as the element would be where it was: it takes the element's
/// width and height where they are shares of the room (`Fill`), and the element's flex.
fn hoverable(element: Element) -> Element {
  let (frame, flex) = (element.node.frame, element.node.flex);
  let share = |length: Option<Dimension>| length.filter(|length| matches!(length, Dimension::Pct(_)));
  let mut row: Element = Row::new().child(element).into();
  row.node.frame.width = share(frame.width);
  row.node.frame.height = share(frame.height);
  row.node.flex = flex;
  row
}

/// `element`, in a row of its own where its root lays out no children (a
/// text, a wrapper of one child): what declares the overlay must be a
/// container, or the overlay is never laid out.
fn holds_children(element: Element) -> Element {
  match element.node.layout_kind() {
    LayoutKind::Row { .. } | LayoutKind::Column { .. } | LayoutKind::Stack { .. } | LayoutKind::Grid(_) => element,
    _ => Row::new().child(element).into(),
  }
}
