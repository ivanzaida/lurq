//! The tooltip of a Markdown document's links: shown at the link the pointer
//! has rested on for the theme's `link_tooltip_delay`, or else at the link
//! with keyboard focus, at once. It lets the pointer through.
//!
//! It is a component of its own so that the pointer moving between links
//! renders only the tooltip, not the document.

use std::{sync::Arc, time::Duration};

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

/// Which link's tooltip the pointer shows: the link it rests on, once the
/// delay has passed. It lives as long as the document, so the links keep
/// their anchors (and with them their node identity, focus and hover) from
/// one render to the next.
#[derive(Clone)]
pub(super) struct TooltipHover {
  /// A plain signal, not the document's state: setting it renders only the
  /// tooltip, which reads it.
  shown: Signal<Option<usize>>,
  /// The link the pointer is on while the delay runs.
  pending: Arc<Mutex<Option<usize>>>,
  timer: Timeout,
  delay: Arc<Mutex<Duration>>,
  anchors: Arc<Mutex<Vec<ElementRef>>>,
}

impl TooltipHover {
  /// A hover state whose timer `create_timeout` makes; call it once, in the
  /// document's `create`.
  pub(super) fn new(create_timeout: impl FnOnce(Box<dyn Fn() + Send + Sync>) -> Timeout) -> Self {
    let shown = Signal::new(None);
    let pending = Arc::new(Mutex::new(None));
    let timer = {
      let (shown, pending) = (shown.clone(), pending.clone());
      create_timeout(Box::new(move || shown.set(*pending.lock())))
    };
    Self {
      shown,
      pending,
      timer,
      delay: Arc::new(Mutex::new(Duration::ZERO)),
      anchors: Arc::new(Mutex::new(Vec::new())),
    }
  }

  /// The anchor of tooltip `slot`'s link, the same in every render.
  pub(super) fn anchor(&self, slot: usize) -> ElementRef {
    let mut anchors = self.anchors.lock();
    while anchors.len() <= slot {
      anchors.push(ElementRef::new());
    }
    anchors[slot].clone()
  }

  pub(super) fn shown(&self) -> Signal<Option<usize>> {
    self.shown.clone()
  }

  /// How long the pointer rests on a link before its tooltip shows.
  pub(super) fn set_delay(&self, delay: Duration) {
    *self.delay.lock() = delay;
    self.timer.set_duration(delay);
  }

  /// The pointer came onto tooltip `slot`'s link.
  pub(super) fn enter(&self, slot: usize) {
    if self.delay.lock().is_zero() {
      self.shown.set(Some(slot));
      return;
    }
    *self.pending.lock() = Some(slot);
    self.timer.restart();
  }

  /// The pointer left tooltip `slot`'s link.
  pub(super) fn leave(&self, slot: usize) {
    {
      let mut pending = self.pending.lock();
      if *pending == Some(slot) {
        *pending = None;
        self.timer.cancel();
      }
    }
    if self.shown.get_untracked() == Some(slot) {
      self.shown.set(None);
    }
  }
}

/// A link with a tooltip: where it shows and what it says.
#[derive(Clone)]
pub(super) struct LinkTooltip {
  anchor: ElementRef,
  text: Arc<str>,
}

impl LinkTooltip {
  pub(super) fn new(anchor: ElementRef, text: Arc<str>) -> Self {
    Self { anchor, text }
  }
}

#[derive(Clone)]
pub(super) struct LinkTooltipsProps {
  pub(super) tooltips: Arc<[LinkTooltip]>,
  pub(super) shown: Signal<Option<usize>>,
  pub(super) style: TextStyle,
  pub(super) text: MarkdownInlineStyle,
  pub(super) panel: MarkdownBlockStyle,
}

/// Every render of the document makes new anchors, so props compare equal
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
    let hovered = props.shown.get().filter(|slot| *slot < props.tooltips.len());
    let focused = props.tooltips.iter().position(|tooltip| tooltip.anchor.focus_visible());
    let Some(tooltip) = hovered.or(focused).map(|slot| &props.tooltips[slot]) else {
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
