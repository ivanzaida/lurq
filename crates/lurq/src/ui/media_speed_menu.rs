//! The menu a player's speed is chosen from.
//!
//! This file is the agreed surface of the component (its props and events), a plain menu until it is checked
//! against the board (`Media/Speed Menu`, `XUFEp`). The players are built on these names: keep them.

use std::sync::Arc;

use super::{MEDIA_RATES, Menu, MenuItem, media_rate_text, theme::current_theme};
use crate::{
  app::theme::Theme,
  core::{ElementRef, Signal},
  node::Element,
};

/// The menu a player's speed is chosen from: the speeds, the one in use checked.
///
/// ```
/// use lurq::{
///   core::{ElementRef, Signal},
///   ui::MediaSpeedMenu,
/// };
///
/// # fn menu(open: Signal<bool>, button: ElementRef) -> MediaSpeedMenu {
/// MediaSpeedMenu::new(open, button, 1.0).on_change(|rate| println!("{rate}"))
/// # }
/// ```
///
/// It is open while `open` is true, anchored to what opens it, and closes on a choice.
pub struct MediaSpeedMenu {
  open: Signal<bool>,
  anchor: ElementRef,
  rate: f32,
  rates: Vec<f32>,
  heading: Arc<str>,
  on_change: Vec<Arc<dyn Fn(f64) + Send + Sync>>,
}

impl MediaSpeedMenu {
  /// The menu of [`MEDIA_RATES`], open while `open` is true, anchored to `anchor`, with `rate` checked.
  pub fn new(open: Signal<bool>, anchor: impl Into<ElementRef>, rate: f32) -> Self {
    Self {
      open,
      anchor: anchor.into(),
      rate,
      rates: MEDIA_RATES.to_vec(),
      heading: Arc::from("Playback speed"),
      on_change: Vec::new(),
    }
  }

  /// The speeds it offers, in place of [`MEDIA_RATES`].
  pub fn rates(mut self, rates: &[f32]) -> Self {
    self.rates = rates.to_vec();
    self
  }

  /// The words over the speeds: "Playback speed" unless told.
  pub fn heading_text(mut self, heading: &str) -> Self {
    self.heading = Arc::from(heading);
    self
  }

  /// Called with the speed a person chooses, where it is another than the one in use. Each call adds a handler.
  pub fn on_change(mut self, handler: impl Fn(f64) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// Builds the menu in `theme`.
  pub fn build(self, theme: &Theme) -> Element {
    let (rate, handlers) = (self.rate, Arc::new(self.on_change));
    let items = self.rates.into_iter().map(|offered| {
      let (chosen, handlers) = ((offered - rate).abs() < f32::EPSILON, handlers.clone());
      MenuItem::new(&media_rate_text(offered))
        .checked(chosen)
        .on_select(move || {
          if !chosen {
            for handler in handlers.iter() {
              handler(f64::from(offered));
            }
          }
        })
    });
    Menu::new(self.open, self.anchor)
      .label(&self.heading)
      .heading(&self.heading)
      .items(items)
      .build(theme)
  }
}

impl From<MediaSpeedMenu> for Element {
  fn from(menu: MediaSpeedMenu) -> Self {
    menu.build(&current_theme())
  }
}
