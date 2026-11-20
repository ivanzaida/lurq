use crate::{
  app::ctx::{CollisionStrategy, OpenState, OutsidePress, Overlay, Placement},
  core::{ElementRef, Signal},
  node::{Element, HitTestBehavior},
};

/// The engine's floating layer anchored to an element, with no look of its own: it places its content beside
/// the anchor, flips and clamps it to the window, and closes on Escape and a press outside. The design system's
/// panels are built on it: an app normally wants [`ui::Popover`](crate::ui::Popover),
/// [`ui::Menu`](crate::ui::Menu) or [`ui::Tooltip`](crate::ui::Tooltip).
pub struct Popup {
  overlay: Overlay,
}

impl Popup {
  pub fn new(anchor: impl Into<ElementRef>, content: impl Into<Element>) -> Self {
    Self {
      overlay: Overlay::new(content)
        .anchor(anchor)
        .hit_test(HitTestBehavior::ContentOnly)
        .dismiss_on_outside_click(true)
        .dismiss_on_escape(true),
    }
  }

  pub fn anchor(mut self, anchor: impl Into<ElementRef>) -> Self {
    self.overlay = self.overlay.anchor(anchor);
    self
  }

  pub fn open(mut self, open: impl Into<OpenState>) -> Self {
    self.overlay = self.overlay.open(open);
    self
  }

  pub fn open_signal(self, open: Signal<bool>) -> Self {
    self.open(open)
  }

  pub fn open_when(mut self, open: bool) -> Self {
    self.overlay = self.overlay.open_when(open);
    self
  }

  pub fn placement(mut self, placement: Placement) -> Self {
    self.overlay = self.overlay.placement(placement);
    self
  }

  pub fn offset(mut self, x: f32, y: f32) -> Self {
    self.overlay = self.overlay.offset(x, y);
    self
  }

  pub fn match_anchor_width(mut self, match_anchor_width: bool) -> Self {
    self.overlay = self.overlay.match_anchor_width(match_anchor_width);
    self
  }

  /// Keeps the popup inside the box of `element` besides the window: see [`Overlay::within`].
  pub fn within(mut self, element: impl Into<ElementRef>) -> Self {
    self.overlay = self.overlay.within(element);
    self
  }

  pub fn collision(mut self, collision: CollisionStrategy) -> Self {
    self.overlay = self.overlay.collision(collision);
    self
  }

  pub fn hit_test(mut self, behavior: HitTestBehavior) -> Self {
    self.overlay = self.overlay.hit_test(behavior);
    self
  }

  pub fn dismiss_on_outside_click(mut self, dismiss: bool) -> Self {
    self.overlay = self.overlay.dismiss_on_outside_click(dismiss);
    self
  }

  /// Whether the press that closes the popup also reaches the element under
  /// the pointer. Defaults to [`OutsidePress::Consume`]: the press only
  /// closes it.
  pub fn outside_press(mut self, outside_press: OutsidePress) -> Self {
    self.overlay = self.overlay.outside_press(outside_press);
    self
  }

  /// Whether the keyboard focus goes into the popup as it opens and back as it
  /// closes, and Tab goes through it right after its anchor: see
  /// [`Overlay::takes_focus`](crate::app::ctx::Overlay::takes_focus). Off by default.
  pub fn takes_focus(mut self, takes: bool) -> Self {
    self.overlay = self.overlay.takes_focus(takes);
    self
  }

  /// Whether Tab goes through the popup right after its anchor while the
  /// focus stays where it is as it opens: see
  /// [`Overlay::tab_after_anchor`](crate::app::ctx::Overlay::tab_after_anchor). Off by default.
  pub fn tab_after_anchor(mut self, tab: bool) -> Self {
    self.overlay = self.overlay.tab_after_anchor(tab);
    self
  }

  /// Whether Tab stays in the popup while the focus is in it, going round from its last element to its first:
  /// see [`Overlay::traps_tab`](crate::app::ctx::Overlay::traps_tab). Off by default.
  pub fn traps_tab(mut self, traps: bool) -> Self {
    self.overlay = self.overlay.traps_tab(traps);
    self
  }

  pub fn dismiss_on_escape(mut self, dismiss: bool) -> Self {
    self.overlay = self.overlay.dismiss_on_escape(dismiss);
    self
  }

  /// What runs after a press outside the popup or Escape closed it: see
  /// [`Overlay::on_dismiss`](crate::app::ctx::Overlay::on_dismiss).
  pub fn on_dismiss(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.overlay = self.overlay.on_dismiss(handler);
    self
  }

  /// A point of the window, in logical pixels, that the popup stands at in
  /// place of its anchor, kept inside the window: see
  /// [`Overlay::at`](crate::app::ctx::Overlay::at). A context menu's.
  pub fn at(mut self, point: Option<(f32, f32)>) -> Self {
    self.overlay = self.overlay.at(point);
    self
  }

  /// [`Overlay::at_placed`](crate::app::ctx::Overlay::at_placed).
  pub(crate) fn at_placed(mut self, placement: Placement) -> Self {
    self.overlay = self.overlay.at_placed(placement);
    self
  }

  /// [`Overlay::keep_clear`](crate::app::ctx::Overlay::keep_clear).
  pub(crate) fn keep_clear(mut self, top: f32, right: f32, bottom: f32, left: f32) -> Self {
    self.overlay = self.overlay.keep_clear(top, right, bottom, left);
    self
  }

  /// Whether the anchor is this popup's trigger, which a left press opens.
  /// While another dropdown is open, a press on a trigger closes it and goes
  /// on to the trigger, so one press switches dropdowns. Defaults to `true`;
  /// set `false` when a left press on the anchor does something else, as for
  /// a context menu or an autocomplete list.
  pub fn switch_on_anchor_press(mut self, switch: bool) -> Self {
    self.overlay = self.overlay.switch_on_anchor_press(switch);
    self
  }
}

impl From<Popup> for Element {
  fn from(popup: Popup) -> Self {
    popup.overlay.into()
  }
}

pub type Popover = Popup;
