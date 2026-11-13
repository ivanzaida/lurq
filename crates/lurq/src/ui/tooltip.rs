//! A tooltip: a short text beside an element while the pointer rests on it or
//! the keyboard's focus is on it.
//!
//! It is a mounted component, not a builder that converts into an element:
//! it keeps a timer for its delay, an anchor that is the same from one render
//! to the next, and whether the pointer has shown it, none of which a builder
//! can keep. [`Tooltip`] says what it is, and [`Tooltip::mount`] puts it on
//! the element it describes.

use std::{
  cell::Cell,
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
    events::{KeyboardEvent, MouseEvent},
    theme::{PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::{Column, Row},
  core::{ElementRef, Signal},
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

/// A tooltip went: the next to show within the skip delay shows at once.
fn went() {
  WARM_UNTIL.with(|until| until.set(Some(Instant::now() + TOOLTIP_SKIP_DELAY)));
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
}

names!(TooltipState {
  Arrow => "arrow",
  Shortcut => "shortcut",
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
/// comes to the element from the keyboard. It goes when the pointer leaves,
/// the focus leaves, the element is pressed or Escape is pressed. It takes
/// neither the pointer nor the focus: the pointer goes through it. Its text is
/// what tooling that reads the tree is told the element is described by, and
/// what the element is called if nothing else names it.
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
  /// that element. The pointer and the focus are the element's root's: give
  /// it the control itself (a button), not a row the control is in.
  pub fn mount(self, ctx: &mut Ctx, element: impl Into<Element>) -> Element {
    ctx.mount::<TooltipHost>(TooltipProps {
      tooltip: Arc::new(self),
      child: element.into(),
    })
  }

  /// The tooltip as it is drawn on `side`, in `theme`.
  fn view(&self, theme: &Theme, side: TooltipSide) -> TooltipView {
    let case = Case {
      variant: Some(side.into()),
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(TooltipState::Arrow, self.arrow)
    .state(TooltipState::Shortcut, self.shortcut.is_some());
    TooltipView {
      text: self.text.clone(),
      shortcut: self.shortcut.clone(),
      side,
      arrow: self.arrow,
      recipe: theme.resolve::<Tooltip>(&case, Some(&self.rules)),
      id: self.id.clone(),
      key: self.key.clone(),
    }
  }

  /// Builds what the tooltip shows on `side`, in `theme`.
  fn shown(&self, theme: &Theme, side: TooltipSide) -> Element {
    let view = self.view(theme, side);
    match theme.renderer::<Tooltip>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }

  /// Builds the tooltip's box as it shows, anchored to nothing, in `theme`.
  /// Converting a tooltip into an element does this in the theme of what is
  /// being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    self.shown(theme, self.side)
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
  /// The style of each part of this tooltip.
  pub recipe: ResolvedRecipe,
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
      .part(&self.part(TooltipPart::Content))
      .child(label.text(&self.text));
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
    if !self.arrow {
      return None;
    }
    let theme = self.theme();
    let px = |length: Option<Length>| match length?.resolve(theme)? {
      Dimension::Px(px) => Some(px),
      _ => None,
    };
    let part = self.part(TooltipPart::Arrow);
    let rest = part.rest();
    let (width, height) = (px(rest.width)?, px(rest.height)?);
    let fill = self.part(TooltipPart::Content).rest().background?;
    let vertical = self.side.vertical();
    // Along the box's edge, and away from it.
    let (along, away) = if vertical { (width, height) } else { (height, width) };
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
    // The part's long side is its main axis, so the square is not shrunk to its short one.
    Some(if vertical {
      Row::new()
        .justify(Justify::Center)
        .align_items(Alignment::Center)
        .part(&part)
        .child(tip)
        .into()
    } else {
      Column::new()
        .justify(Justify::Center)
        .align_items(Alignment::Center)
        .part(&part)
        .child(tip)
        .into()
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
  /// describes.
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

/// What shows a tooltip on the element it describes, and when.
struct TooltipHost {
  /// Whether the pointer has rested on the element for the delay.
  hovered: Signal<bool>,
  /// Whether Escape or a press put the tooltip away: it stays away until the
  /// pointer comes again or the focus leaves.
  dismissed: Signal<bool>,
  /// Whether the tooltip was put on the other side, where there was no room
  /// on its own: its arrow turns with it.
  flipped: Signal<bool>,
  /// The side the tooltip was given, for what reads where it was put.
  side: Arc<Mutex<TooltipSide>>,
  timer: Timeout,
  /// The element it describes.
  anchor: ElementRef,
  /// What shows.
  shown: ElementRef,
}

impl Component for TooltipHost {
  type Props = TooltipProps;

  fn create(ctx: &mut Ctx) -> Self {
    let hovered = ctx.signal(false);
    let timer = {
      let hovered = hovered.clone();
      ctx.create_timeout(TOOLTIP_DELAY, move || put(&hovered, true))
    };
    let (anchor, shown) = (ElementRef::new(), ElementRef::new());
    let flipped = ctx.signal(false);
    let side = Arc::new(Mutex::new(TooltipSide::Top));
    {
      // The overlay does not say which side it put the tooltip on: where it was laid out says.
      let (anchor, flipped, side) = (anchor.clone(), flipped.clone(), side.clone());
      shown.observe_rect(move |rect| {
        let ((x, y), (anchor_x, anchor_y)) = (rect.center(), anchor.bounds().center());
        let other = match *side.lock() {
          TooltipSide::Top => y > anchor_y,
          TooltipSide::Bottom => y < anchor_y,
          TooltipSide::Left => x > anchor_x,
          TooltipSide::Right => x < anchor_x,
        };
        put(&flipped, other);
      });
    }
    Self {
      hovered,
      dismissed: ctx.signal(false),
      flipped,
      side,
      timer,
      anchor,
      shown,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let TooltipProps { tooltip, child } = ctx.props::<TooltipProps>().clone();
    if tooltip.disabled {
      return child;
    }
    self.timer.set_duration(tooltip.delay);
    *self.side.lock() = tooltip.side;
    // Reading them subscribes: this renders again when the pointer, the focus or Escape changes them.
    let asked = (self.hovered.get() || self.anchor.focus_visible()) && !self.dismissed.get();
    let open = !tooltip.text.is_empty() && tooltip.open.unwrap_or(asked);
    let overlay = if open {
      let side = if self.flipped.get() {
        tooltip.side.opposite()
      } else {
        tooltip.side
      };
      let shown = tooltip
        .shown(&current_theme(), side)
        .map_node(|node| node.ref_element(self.shown.clone()));
      Overlay::new(shown)
        .anchor(self.anchor.clone())
        .placement(tooltip.side.placement())
        .collision(CollisionStrategy::FlipThenClamp)
        .hit_test(HitTestBehavior::None)
    } else {
      Overlay::new(Element::new()).open_when(false)
    };
    // An element is not shared between threads as it is: the decoration, which is, holds it in a lock.
    let overlay = Arc::new(Mutex::new(Element::from(overlay)));

    let enter = {
      let (hovered, dismissed, timer, delay) = (
        self.hovered.clone(),
        self.dismissed.clone(),
        self.timer.clone(),
        tooltip.delay,
      );
      move || {
        put(&dismissed, false);
        if delay.is_zero() || warm() {
          timer.cancel();
          put(&hovered, true);
        } else {
          timer.restart();
        }
      }
    };
    let leave = {
      let (hovered, dismissed, timer) = (self.hovered.clone(), self.dismissed.clone(), self.timer.clone());
      move || {
        timer.cancel();
        if hovered.get_untracked() {
          hovered.set(false);
          went();
        }
        put(&dismissed, false);
      }
    };
    // A press, or Escape while it shows, puts it away until the pointer comes again or the focus leaves.
    let away = {
      let (hovered, dismissed, timer, anchor) = (
        self.hovered.clone(),
        self.dismissed.clone(),
        self.timer.clone(),
        self.anchor.clone(),
      );
      move || {
        timer.cancel();
        let showing = hovered.get_untracked() || anchor.focus_visible_signal().get_untracked();
        if showing && !dismissed.get_untracked() {
          went();
        }
        put(&hovered, false);
        put(&dismissed, true);
      }
    };
    let press = {
      let away = away.clone();
      move |_: MouseEvent| away()
    };
    let escape = move |event: KeyboardEvent| {
      if event.key == "Escape" {
        away();
      }
    };
    let blur = {
      let dismissed = self.dismissed.clone();
      move || put(&dismissed, false)
    };

    let (anchor, text) = (self.anchor.clone(), tooltip.text.clone());
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
        .on_mouse_down(press.clone())
        .on_key_down(escape.clone())
        .on_blur(blur.clone());
      NodeUpdate::describe(&mut node, Arc::from("a11y_description"), text.clone());
      if !named {
        NodeUpdate::describe(&mut node, Arc::from("a11y_name"), text.clone());
      }
      // The overlay is declared in the element: a layer is found among the children of a container.
      NodeUpdate::child(&mut node, overlay.lock().clone().into_node());
      node
    };
    holds_children(child).decorate_root(describe)
  }
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
