use crate::{
  app::ctx::{CollisionStrategy, OpenState, Overlay, Placement},
  core::{ElementRef, Signal},
  node::{Element, HitTestBehavior},
};

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

  pub fn dismiss_on_escape(mut self, dismiss: bool) -> Self {
    self.overlay = self.overlay.dismiss_on_escape(dismiss);
    self
  }
}

impl From<Popup> for Element {
  fn from(popup: Popup) -> Self {
    popup.overlay.into()
  }
}

pub type Popover = Popup;
