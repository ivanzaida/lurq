//! A waveform: the loudness of a recording as a row of bars, the played part of it marked.
//!
//! This file is the agreed surface of the component (its props and events), drawn plainly until the component is
//! given its own parts and recipe from the board (`Media/Waveform`, `Kq0Yo`). The players are built on these
//! names: keep them.

use std::sync::Arc;

use super::theme::current_theme;
use crate::{
  app::theme::{PaletteColor, Theme},
  components::{Rect, Row},
  layout::Alignment,
  node::Element,
};

/// A waveform.
///
/// ```
/// use lurq::ui::Waveform;
///
/// // A third of it played.
/// let wave = Waveform::at(0.33, &[0.2, 0.6, 1.0, 0.4, 0.7, 0.3]);
/// // The app holds how far it is and is told where a person presses or drags to.
/// let seek = Waveform::controlled(0.33, &[0.2, 0.6, 1.0, 0.4], |to| println!("{to}"));
/// ```
///
/// `peaks` are the heights of its bars, each from 0 to 1, one for each bar. How far it is played runs from 0 to 1.
pub struct Waveform {
  progress: f64,
  peaks: Vec<f32>,
  unheard: bool,
  label: Arc<str>,
  disabled: bool,
  on_change: Vec<Arc<dyn Fn(f64) + Send + Sync>>,
  on_change_end: Vec<Arc<dyn Fn(f64) + Send + Sync>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Waveform {
  /// A waveform shown and not operated, `progress` of it played.
  pub fn at(progress: impl Into<f64>, peaks: &[f32]) -> Self {
    Self {
      progress: progress.into(),
      peaks: peaks.to_vec(),
      unheard: false,
      label: Arc::from("Seek"),
      disabled: false,
      on_change: Vec::new(),
      on_change_end: Vec::new(),
      id: None,
      key: None,
    }
  }

  /// A waveform that shows `progress`, held by the app, and tells `on_change` where a person presses or drags
  /// to, from 0 to 1.
  pub fn controlled(progress: impl Into<f64>, peaks: &[f32], on_change: impl Fn(f64) + Send + Sync + 'static) -> Self {
    Self::at(progress, peaks).on_change(on_change)
  }

  /// Whether nothing of it was heard yet: every bar is then in the color of the part not played.
  pub fn unheard(mut self, unheard: bool) -> Self {
    self.unheard = unheard;
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

  /// Called with where a person moves it to, from 0 to 1. Each call adds a handler.
  pub fn on_change(mut self, handler: impl Fn(f64) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// Called with where it is when a drag is let go. Each call adds a handler.
  pub fn on_change_end(mut self, handler: impl Fn(f64) + Send + Sync + 'static) -> Self {
    self.on_change_end.push(Arc::new(handler));
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

  /// Builds the waveform in `theme`.
  pub fn build(self, _theme: &Theme) -> Element {
    let played = if self.unheard {
      0
    } else {
      (self.progress.clamp(0.0, 1.0) * self.peaks.len() as f64).round() as usize
    };
    let mut row = Row::new().align_items(Alignment::Center).spacing(2.0).height(24.0);
    for (index, peak) in self.peaks.iter().enumerate() {
      let color = if index < played {
        PaletteColor::Primary
      } else {
        PaletteColor::Input
      };
      row = row.child(
        Rect::new(3.0, (peak.clamp(0.0, 1.0) * 24.0).max(3.0))
          .background(color)
          .corner_radius(1.5),
      );
    }
    if let Some(id) = self.id {
      row = row.id(id);
    }
    if let Some(key) = self.key {
      row = row.key(key);
    }
    row.into()
  }
}

impl From<Waveform> for Element {
  fn from(wave: Waveform) -> Self {
    wave.build(&current_theme())
  }
}
