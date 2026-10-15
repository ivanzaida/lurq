//! The tooltip of a Markdown document's links: shown at the link the pointer
//! has rested on for the theme's `link_tooltip_delay`, or else at the link
//! with keyboard focus, at once. It lets the pointer through.
//!
//! It is a component of its own so that the pointer moving between links
//! renders only the tooltip, not the document.

use std::{collections::HashMap, sync::Arc, time::Duration};

use parking_lot::Mutex;

use super::{BlockWidth, apply_node_box};
use crate::{
  app::{
    component::Component,
    ctx::{CollisionStrategy, Ctx, Overlay, Placement, Timeout},
    theme::{MarkdownBlockStyle, MarkdownInlineStyle},
  },
  components::Text,
  core::{ElementRef, Signal},
  layout::text_style::TextStyle,
  node::{Element, HitTestBehavior},
};

/// The gap between a link and its tooltip.
const TOOLTIP_GAP: f32 = 4.0;

/// Which link a tooltip belongs to, the same from one render to the next
/// while the link stays: its destination, and how many links to that
/// destination come before it. A link added before it (a streamed reply
/// growing) does not move another link's tooltip or anchor.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct TooltipKey {
  destination: Arc<str>,
  occurrence: usize,
}

impl TooltipKey {
  pub(super) fn new(destination: Arc<str>, occurrence: usize) -> Self {
    Self {
      destination,
      occurrence,
    }
  }
}

#[cfg(feature = "devtools")]
impl crate::app::component::DevtoolsInspectable for TooltipKey {}

/// Which link's tooltip the pointer shows: the link it rests on, once the
/// delay has passed. It lives as long as the document, so the links keep
/// their anchors (and with them their node identity, focus and hover) from
/// one render to the next.
#[derive(Clone)]
pub(super) struct TooltipHover {
  /// A plain signal, not the document's state: setting it renders only the
  /// tooltip, which reads it.
  shown: Signal<Option<TooltipKey>>,
  /// The link the pointer is on while the delay runs.
  pending: Arc<Mutex<Option<TooltipKey>>>,
  timer: Timeout,
  delay: Arc<Mutex<Duration>>,
  anchors: Arc<Mutex<HashMap<TooltipKey, ElementRef>>>,
}

impl TooltipHover {
  /// A hover state whose timer `create_timeout` makes; call it once, in the
  /// document's `create`.
  pub(super) fn new(create_timeout: impl FnOnce(Box<dyn Fn() + Send + Sync>) -> Timeout) -> Self {
    let shown = Signal::new(None);
    let pending = Arc::new(Mutex::new(None::<TooltipKey>));
    let timer = {
      let (shown, pending) = (shown.clone(), pending.clone());
      create_timeout(Box::new(move || shown.set(pending.lock().clone())))
    };
    Self {
      shown,
      pending,
      timer,
      delay: Arc::new(Mutex::new(Duration::ZERO)),
      anchors: Arc::new(Mutex::new(HashMap::new())),
    }
  }

  /// The anchor of `key`'s link, the same in every render.
  pub(super) fn anchor(&self, key: &TooltipKey) -> ElementRef {
    self.anchors.lock().entry(key.clone()).or_default().clone()
  }

  /// Forgets the anchors of links that are gone.
  pub(super) fn keep_anchors(&self, tooltips: &[LinkTooltip]) {
    self
      .anchors
      .lock()
      .retain(|key, _| tooltips.iter().any(|tooltip| tooltip.key == *key));
  }

  pub(super) fn shown(&self) -> Signal<Option<TooltipKey>> {
    self.shown.clone()
  }

  /// How long the pointer rests on a link before its tooltip shows.
  pub(super) fn set_delay(&self, delay: Duration) {
    *self.delay.lock() = delay;
    self.timer.set_duration(delay);
  }

  /// The pointer came onto `key`'s link.
  pub(super) fn enter(&self, key: &TooltipKey) {
    if self.delay.lock().is_zero() {
      self.shown.set(Some(key.clone()));
      return;
    }
    *self.pending.lock() = Some(key.clone());
    self.timer.restart();
  }

  /// The pointer left `key`'s link.
  pub(super) fn leave(&self, key: &TooltipKey) {
    {
      let mut pending = self.pending.lock();
      if pending.as_ref() == Some(key) {
        *pending = None;
        self.timer.cancel();
      }
    }
    if self.shown.get_untracked().as_ref() == Some(key) {
      self.shown.set(None);
    }
  }
}

/// A link with a tooltip: where it shows and what it says.
#[derive(Clone)]
pub(super) struct LinkTooltip {
  key: TooltipKey,
  anchor: ElementRef,
  text: Arc<str>,
}

impl LinkTooltip {
  pub(super) fn new(key: TooltipKey, anchor: ElementRef, text: Arc<str>) -> Self {
    Self { key, anchor, text }
  }
}

#[derive(Clone)]
pub(super) struct LinkTooltipsProps {
  pub(super) tooltips: Arc<[LinkTooltip]>,
  pub(super) shown: Signal<Option<TooltipKey>>,
  pub(super) style: TextStyle,
  pub(super) text: MarkdownInlineStyle,
  pub(super) panel: MarkdownBlockStyle,
}

/// Every render of the document makes a new list, so props compare equal
/// only to themselves.
impl PartialEq for LinkTooltipsProps {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.tooltips, &other.tooltips)
      && self.style == other.style
      && self.text == other.text
      && self.panel == other.panel
  }
}

#[cfg(feature = "devtools")]
impl crate::app::component::DevtoolsInspectable for LinkTooltipsProps {}

pub(super) struct LinkTooltips;

impl Component for LinkTooltips {
  type Props = LinkTooltipsProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Self::Props>().clone();
    // Reading the hover and every anchor's focus-visible state subscribes,
    // so this renders again when either changes.
    let hovered = props
      .shown
      .get()
      .and_then(|key| props.tooltips.iter().find(|tooltip| tooltip.key == key));
    let focused = props.tooltips.iter().find(|tooltip| tooltip.anchor.focus_visible());
    let Some(tooltip) = hovered.or(focused) else {
      return Element::from(Overlay::new(Element::from_node(crate::node::Node::new())).open_when(false));
    };
    let words = Text::styled(&tooltip.text, props.text.apply(&props.style));
    let panel = BlockWidth::Fit
      .leaf(words)
      .map_node(|node| apply_node_box(node, &props.panel));
    Element::from(
      Overlay::new(panel)
        .anchor(tooltip.anchor.clone())
        .placement(Placement::BottomStart)
        .offset(0.0, TOOLTIP_GAP)
        .collision(CollisionStrategy::FlipThenClamp)
        .hit_test(HitTestBehavior::None),
    )
  }
}
