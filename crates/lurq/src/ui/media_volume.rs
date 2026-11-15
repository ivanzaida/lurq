//! A volume control: a button that mutes, and a slider of how loud.
//!
//! This file is the agreed surface of the component (its props and events), drawn with a plain button and slider
//! until the component is given its own parts and recipe from the board (`Media/Volume`, `hRRVr`). The players are
//! built on these names: keep them.

use std::sync::Arc;

use super::{
  Button, ButtonSize, ButtonVariant, Slider, icons, names,
  recipe::{Interaction, Name},
  theme::current_theme,
};
use crate::{app::theme::Theme, components::Row, core::Signal, layout::Alignment, node::Element};

/// How large a volume control is: a button of 32 with an icon of 16, or of 36 with 20 in a video's bar.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MediaVolumeSize {
  #[default]
  Md,
  Lg,
}

names!(MediaVolumeSize {
  Md => "md",
  Lg => "lg",
});

/// A volume control.
///
/// ```
/// use lurq::{core::Signal, ui::MediaVolume};
///
/// let (volume, muted) = (Signal::new(0.6_f32), Signal::new(false));
/// let control = MediaVolume::new(volume).muted(muted).on_change(|volume| println!("{volume}"));
/// // The app holds both and is told what a person makes them.
/// let held = MediaVolume::controlled(0.6, |volume| println!("{volume}")).muted_controlled(false, |muted| println!("{muted}"));
/// ```
///
/// The volume runs from 0 to 1. Muting keeps it, so that the sound comes back as loud as it was. The glyph is the
/// loud one above a half, the low one under it, and the crossed one while muted or at nothing.
pub struct MediaVolume {
  volume: Signal<f32>,
  controlled: Option<f64>,
  muted: bool,
  on_change: Vec<Arc<dyn Fn(f64) + Send + Sync>>,
  on_change_end: Vec<Arc<dyn Fn(f64) + Send + Sync>>,
  on_muted_change: Vec<Arc<dyn Fn(bool) + Send + Sync>>,
  collapsed: bool,
  on_media: bool,
  size: Name,
  length: f32,
  label: Arc<str>,
  mute_label: Arc<str>,
  unmute_label: Arc<str>,
  disabled: bool,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl MediaVolume {
  /// A volume control bound to `volume`, from 0 to 1. What a person does sets the signal.
  pub fn new(volume: Signal<f32>) -> Self {
    Self {
      volume,
      controlled: None,
      muted: false,
      on_change: Vec::new(),
      on_change_end: Vec::new(),
      on_muted_change: Vec::new(),
      collapsed: false,
      on_media: false,
      size: MediaVolumeSize::Md.into(),
      length: 72.0,
      label: Arc::from("Volume"),
      mute_label: Arc::from("Mute"),
      unmute_label: Arc::from("Unmute"),
      disabled: false,
      forced: None,
      id: None,
      key: None,
    }
  }

  /// A volume control that shows `volume`, held by the app, and tells `on_change` what a person makes it.
  pub fn controlled(volume: impl Into<f64>, on_change: impl Fn(f64) + Send + Sync + 'static) -> Self {
    let volume = volume.into();
    let mut control = Self::new(Signal::new(volume as f32)).on_change(on_change);
    control.controlled = Some(volume);
    control
  }

  /// Muted while `muted` is true, which a press of the button turns.
  pub fn muted(mut self, muted: Signal<bool>) -> Self {
    self.muted = muted.get();
    self.on_muted_change.insert(0, Arc::new(move |next| muted.set(next)));
    self
  }

  /// Muted while `muted` is true, held by the app, which is told what a press makes it.
  pub fn muted_controlled(mut self, muted: bool, on_muted_change: impl Fn(bool) + Send + Sync + 'static) -> Self {
    self.muted = muted;
    self.on_muted_change(on_muted_change)
  }

  /// Called with the volume each time a person moves it. Each call adds a handler.
  pub fn on_change(mut self, handler: impl Fn(f64) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// Called with the volume when a drag is let go or a key's step is made. Each call adds a handler.
  pub fn on_change_end(mut self, handler: impl Fn(f64) + Send + Sync + 'static) -> Self {
    self.on_change_end.push(Arc::new(handler));
    self
  }

  /// Called on a press of the button, with whether it is to be muted. Each call adds a handler.
  pub fn on_muted_change(mut self, handler: impl Fn(bool) + Send + Sync + 'static) -> Self {
    self.on_muted_change.push(Arc::new(handler));
    self
  }

  /// Whether it is the button alone, the slider coming under the pointer and with the focus: where a bar has no
  /// room for the slider.
  pub fn collapsed(mut self, collapsed: bool) -> Self {
    self.collapsed = collapsed;
    self
  }

  /// Whether it stands over a picture: white, the same in a light and a dark theme.
  pub fn on_media(mut self, on_media: bool) -> Self {
    self.on_media = on_media;
    self
  }

  /// Its size, a [`MediaVolumeSize`].
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.size = size.into();
    self
  }

  /// How long the slider is, in pixels: 72 unless told.
  pub fn length(mut self, length: f32) -> Self {
    self.length = length;
    self
  }

  /// What the slider is called, to what reads the tree: "Volume" unless told.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Arc::from(label);
    self
  }

  /// What the button is called while a press would mute.
  pub fn mute_label(mut self, label: &str) -> Self {
    self.mute_label = Arc::from(label);
    self
  }

  /// What the button is called while a press would bring the sound back.
  pub fn unmute_label(mut self, label: &str) -> Self {
    self.unmute_label = Arc::from(label);
    self
  }

  /// Whether it takes no input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Draws its button as it is under the pointer, held down or focused from the keyboard, with nothing pointing
  /// at it.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the button.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the volume control in `theme`.
  pub fn build(self, theme: &Theme) -> Element {
    let level = self.controlled.unwrap_or_else(|| f64::from(self.volume.get()));
    let silent = self.muted || level <= 0.0;
    let icon = if silent {
      icons::VOLUME_OFF
    } else if level < 0.5 {
      icons::VOLUME_LOW
    } else {
      icons::VOLUME
    };
    let size = if self.size == Name::from(MediaVolumeSize::Lg) {
      ButtonSize::Md
    } else {
      ButtonSize::Sm
    };
    let (muting, next) = (self.on_muted_change, !self.muted);
    let mut button = Button::new(if self.muted {
      &self.unmute_label
    } else {
      &self.mute_label
    })
    .icon(icon)
    .icon_only(true)
    .variant(ButtonVariant::Ghost)
    .size(size)
    .disabled(self.disabled)
    .force(self.forced)
    .on_click(move |_| {
      for handler in &muting {
        handler(next);
      }
    });
    if let Some(id) = self.id {
      button = button.id(id);
    }
    if let Some(key) = self.key {
      button = button.key(key);
    }
    let mut row = Row::new()
      .align_items(Alignment::Center)
      .spacing(4.0)
      .child(button.build(theme));
    if !self.collapsed {
      let (volume, held) = (self.volume.clone(), self.controlled);
      let (change, end) = (self.on_change, self.on_change_end);
      if self.muted {
        // Muted, the slider stands at nothing while the volume is kept.
        self.volume.set(0.0);
      }
      let slider = Slider::new(self.volume)
        .max(1.0)
        .step(0.05)
        .length(self.length)
        .label(&self.label)
        .disabled(self.disabled)
        .on_change(move |to| {
          for handler in &change {
            handler(to);
          }
          if let Some(held) = held {
            volume.set(held as f32);
          }
        })
        .on_change_end(move |to| {
          for handler in &end {
            handler(to);
          }
        });
      row = row.child(slider.build(theme));
    }
    row.into()
  }
}

impl From<MediaVolume> for Element {
  fn from(volume: MediaVolume) -> Self {
    volume.build(&current_theme())
  }
}
