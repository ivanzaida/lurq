//! A play button: the round control that starts and pauses what a player plays.
//!
//! This file is the agreed surface of the component (its props and events), drawn with a plain button until the
//! component is given its own parts and recipe from the board (`Media/Play Button`, `GN7mn`, and
//! `Media/Play Button/Overlay`, `PfeN2`). The players are built on these names: keep them.

use std::sync::Arc;

use super::{
  Button, ButtonShape, ButtonSize, ButtonVariant, icons, names,
  recipe::{Interaction, Name},
  theme::current_theme,
};
use crate::{app::theme::Theme, node::Element};

/// How large a play button is: 32 with an icon of 14, 40 with 16, 48 with 20, 64 with 28.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PlayButtonSize {
  Sm,
  #[default]
  Md,
  Lg,
  Xl,
}

names!(PlayButtonSize {
  Sm => "sm",
  Md => "md",
  Lg => "lg",
  Xl => "xl",
});

/// What a play button stands on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PlayButtonVariant {
  /// On a surface of the theme: the primary color.
  #[default]
  Primary,
  /// Over a picture: a dark disc with a white glyph, the same in a light and a dark theme.
  Overlay,
}

names!(PlayButtonVariant {
  Primary => "primary",
  Overlay => "overlay",
});

/// A play button.
///
/// ```
/// use lurq::ui::{PlayButton, PlayButtonSize};
///
/// let play = PlayButton::new().playing(false).size(PlayButtonSize::Lg).on_change(|playing| println!("{playing}"));
/// ```
///
/// It does not keep whether something plays: the app says so with [`PlayButton::playing`] and is told of a press
/// with [`PlayButton::on_change`], with what it is to be.
pub struct PlayButton {
  playing: bool,
  loading: bool,
  replay: bool,
  disabled: bool,
  size: Name,
  variant: Name,
  play_label: Arc<str>,
  pause_label: Arc<str>,
  replay_label: Arc<str>,
  on_change: Vec<Arc<dyn Fn(bool) + Send + Sync>>,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Default for PlayButton {
  fn default() -> Self {
    Self::new()
  }
}

impl PlayButton {
  /// A play button of nothing that plays yet.
  pub fn new() -> Self {
    Self {
      playing: false,
      loading: false,
      replay: false,
      disabled: false,
      size: PlayButtonSize::Md.into(),
      variant: PlayButtonVariant::Primary.into(),
      play_label: Arc::from("Play"),
      pause_label: Arc::from("Pause"),
      replay_label: Arc::from("Replay"),
      on_change: Vec::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// Whether it plays: the glyph is then the pause one.
  pub fn playing(mut self, playing: bool) -> Self {
    self.playing = playing;
    self
  }

  /// Whether what it would play is still being fetched: a spinner for the glyph, and a press does nothing.
  pub fn loading(mut self, loading: bool) -> Self {
    self.loading = loading;
    self
  }

  /// Whether what it played has ended: the glyph is then the one of playing again, and a press tells `true`.
  pub fn replay(mut self, replay: bool) -> Self {
    self.replay = replay;
    self
  }

  /// Whether it takes no input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Its size, a [`PlayButtonSize`].
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.size = size.into();
    self
  }

  /// What it stands on, a [`PlayButtonVariant`].
  pub fn variant(mut self, variant: impl Into<Name>) -> Self {
    self.variant = variant.into();
    self
  }

  /// What it is called while a press would play.
  pub fn play_label(mut self, label: &str) -> Self {
    self.play_label = Arc::from(label);
    self
  }

  /// What it is called while a press would pause.
  pub fn pause_label(mut self, label: &str) -> Self {
    self.pause_label = Arc::from(label);
    self
  }

  /// What it is called while a press would play again from the start.
  pub fn replay_label(mut self, label: &str) -> Self {
    self.replay_label = Arc::from(label);
    self
  }

  /// Called on a press, with what it is to be: playing if it was paused. Each call adds a handler.
  pub fn on_change(mut self, handler: impl Fn(bool) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// Draws it as it is under the pointer, held down or focused from the keyboard, with nothing pointing at it.
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

  /// Builds the play button in `theme`.
  pub fn build(self, theme: &Theme) -> Element {
    let (icon, label) = if self.replay {
      (icons::ROTATE_LEFT, self.replay_label)
    } else if self.playing {
      (icons::PAUSE, self.pause_label)
    } else {
      (icons::PLAY, self.play_label)
    };
    let size = if self.size == Name::from(PlayButtonSize::Sm) {
      ButtonSize::Sm
    } else if self.size == Name::from(PlayButtonSize::Lg) {
      ButtonSize::Lg
    } else if self.size == Name::from(PlayButtonSize::Xl) {
      ButtonSize::Xl
    } else {
      ButtonSize::Md
    };
    let variant = if self.variant == Name::from(PlayButtonVariant::Overlay) {
      ButtonVariant::Secondary
    } else {
      ButtonVariant::Primary
    };
    let mut button = Button::new(&label)
      .icon(icon)
      .icon_only(true)
      .shape(ButtonShape::Pill)
      .size(size)
      .variant(variant)
      .loading(self.loading)
      .disabled(self.disabled)
      .force(self.forced);
    if !self.on_change.is_empty() {
      let (handlers, next) = (self.on_change, self.replay || !self.playing);
      button = button.on_click(move |_| {
        for handler in &handlers {
          handler(next);
        }
      });
    }
    if let Some(id) = self.id {
      button = button.id(id);
    }
    if let Some(key) = self.key {
      button = button.key(key);
    }
    button.build(theme)
  }
}

impl From<PlayButton> for Element {
  fn from(button: PlayButton) -> Self {
    button.build(&current_theme())
  }
}
