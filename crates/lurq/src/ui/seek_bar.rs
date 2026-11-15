//! A seek bar: where a player is in what it plays, how much of it is loaded, and the handle that moves it.
//!
//! This file is the agreed surface of the component (its props and events), drawn with a plain slider until the
//! component is given its own parts and recipe from the board (`Media/Seek Bar`, `uniHD`; over a picture
//! `Media/Seek Bar/On Media`, `XyxOi`; with chapters `HfHQq`; the frame over the pointer `Media/Seek Preview`,
//! `b4rqjJ`). The players are built on these names: keep them.

use std::sync::Arc;

use super::{Slider, recipe::Interaction, theme::current_theme};
use crate::{app::theme::Theme, core::Signal, node::Element};

/// A seek bar.
///
/// ```
/// use lurq::{core::Signal, ui::SeekBar};
///
/// // Bound to the app's position, in seconds, of a recording 250 seconds long.
/// let position = Signal::new(84.0_f32);
/// let bar = SeekBar::new(position, 250.0).buffered(140.0).on_change_end(|to| println!("seek to {to}"));
/// // The app holds the position and is told where a person moves it.
/// let held = SeekBar::controlled(84.0, 250.0, |to| println!("{to}"));
/// ```
///
/// [`SeekBar::on_change`] hears the position as a drag moves it, [`SeekBar::on_change_end`] where it is let go:
/// a player that seeks only when the handle is let go listens to the second.
pub struct SeekBar {
  position: Signal<f32>,
  controlled: Option<f64>,
  duration: f64,
  buffered: Option<f64>,
  chapters: Vec<f64>,
  step: f64,
  on_media: bool,
  label: Arc<str>,
  disabled: bool,
  on_change: Vec<Arc<dyn Fn(f64) + Send + Sync>>,
  on_change_end: Vec<Arc<dyn Fn(f64) + Send + Sync>>,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl SeekBar {
  /// A seek bar bound to `position`, in seconds, of something `duration` seconds long. What a person does sets
  /// the signal.
  pub fn new(position: Signal<f32>, duration: impl Into<f64>) -> Self {
    Self {
      position,
      controlled: None,
      duration: duration.into(),
      buffered: None,
      chapters: Vec::new(),
      step: 5.0,
      on_media: false,
      label: Arc::from("Seek"),
      disabled: false,
      on_change: Vec::new(),
      on_change_end: Vec::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// A seek bar that shows `position`, held by the app, and tells `on_change` where a person moves it.
  pub fn controlled(
    position: impl Into<f64>,
    duration: impl Into<f64>,
    on_change: impl Fn(f64) + Send + Sync + 'static,
  ) -> Self {
    let position = position.into();
    let mut bar = Self::new(Signal::new(position as f32), duration).on_change(on_change);
    bar.controlled = Some(position);
    bar
  }

  /// How far what is played is loaded, in seconds from its start: drawn between what is played and the rest.
  pub fn buffered(mut self, buffered: impl Into<f64>) -> Self {
    self.buffered = Some(buffered.into());
    self
  }

  /// Where the chapters start, in seconds: the bar is cut there.
  pub fn chapters(mut self, starts: &[f64]) -> Self {
    self.chapters = starts.to_vec();
    self
  }

  /// How many seconds an arrow key moves it: 5 unless told.
  pub fn step(mut self, step: impl Into<f64>) -> Self {
    self.step = step.into();
    self
  }

  /// Whether it stands over a picture: white, the same in a light and a dark theme.
  pub fn on_media(mut self, on_media: bool) -> Self {
    self.on_media = on_media;
    self
  }

  /// What it is called, to what reads the tree: "Seek" unless told.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Arc::from(label);
    self
  }

  /// Whether it takes no input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Called with the position, in seconds, each time a person moves it. Each call adds a handler.
  pub fn on_change(mut self, handler: impl Fn(f64) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// Called with the position, in seconds, when a drag is let go or a key's step is made. Each call adds a
  /// handler.
  pub fn on_change_end(mut self, handler: impl Fn(f64) + Send + Sync + 'static) -> Self {
    self.on_change_end.push(Arc::new(handler));
    self
  }

  /// Draws it as it is under the pointer or focused from the keyboard, with nothing pointing at it.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the seek bar in `theme`.
  pub fn build(self, theme: &Theme) -> Element {
    let (position, held) = (self.position.clone(), self.controlled);
    let (change, end) = (self.on_change, self.on_change_end);
    let mut slider = Slider::new(self.position)
      .max(self.duration.max(0.0))
      .step(self.step)
      .label(&self.label)
      .disabled(self.disabled)
      .force(self.forced)
      .on_change(move |to| {
        for handler in &change {
          handler(to);
        }
        // A controlled bar goes back to what the app holds until the app answers.
        if let Some(held) = held {
          position.set(held as f32);
        }
      })
      .on_change_end(move |to| {
        for handler in &end {
          handler(to);
        }
      });
    if let Some(id) = self.id {
      slider = slider.id(id);
    }
    if let Some(key) = self.key {
      slider = slider.key(key);
    }
    slider.build(theme)
  }
}

impl From<SeekBar> for Element {
  fn from(bar: SeekBar) -> Self {
    bar.build(&current_theme())
  }
}
