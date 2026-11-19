//! A play button: the round control that starts and pauses what a player plays.

use std::sync::Arc;

use super::{
  icons, names,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::{fading, glyph, pressable},
  theme::{Themed, current_theme},
};
use crate::{
  app::theme::{OpacityLevel, PaletteColor, RadiusSize, Theme},
  components,
  node::{CursorIcon, Element, Outline, color::Color},
};

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

/// The parts of a play button a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlayButtonPart {
  /// The disc: what is pressed.
  Root,
  /// The glyph in the middle of it: play, pause, play again, or the spinner while it is loading.
  Icon,
}

names!(PlayButtonPart {
  Root => "root",
  Icon => "icon",
});

/// The states of a play button a recipe styles, besides what the pointer and the keyboard do to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlayButtonState {
  /// What it plays is playing: the glyph is the pause one.
  Playing,
  /// What it played has ended: the glyph is the one of playing again.
  Replay,
  /// What it would play is still being fetched: the spinner turns, and a press does nothing.
  Loading,
  /// It takes no input.
  Disabled,
}

names!(PlayButtonState {
  Playing => "playing",
  Replay => "replay",
  Loading => "loading",
  Disabled => "disabled",
});

/// How much of a play button shows while it is loading, in the design's recipe.
const LOADING_OPACITY: f32 = 0.75;

/// What builds a play button's nodes: a theme's in place of the default one.
pub type PlayButtonRenderer = Arc<dyn Fn(PlayButtonView) -> Element + Send + Sync>;

/// A play button.
///
/// ```
/// use lurq::ui::{PlayButton, PlayButtonSize, PlayButtonVariant};
///
/// let play = PlayButton::new().playing(false).size(PlayButtonSize::Lg).on_change(|playing| println!("{playing}"));
/// // Over a picture, in the middle of a paused video.
/// let big = PlayButton::new().variant(PlayButtonVariant::Overlay).size(PlayButtonSize::Xl);
/// ```
///
/// It does not keep whether something plays: the app says so with [`PlayButton::playing`] and is told of a press
/// with [`PlayButton::on_change`], with what it is to be. It is a button to what reads the tree, called by the label
/// of what a press would do ("Play", "Pause", "Replay"), and a click, Enter and Space press it.
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
  states: Vec<Name>,
  rules: Recipe,
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
      play_label: super::current_locale().media.play.clone(),
      pause_label: super::current_locale().media.pause.clone(),
      replay_label: super::current_locale().media.replay.clone(),
      on_change: Vec::new(),
      states: Vec::new(),
      rules: Recipe::new(),
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

  /// Whether what it played has ended: the glyph is the one of playing again, and a press tells `true`.
  pub fn replay(mut self, replay: bool) -> Self {
    self.replay = replay;
    self
  }

  /// Whether it takes no input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Its size, a [`PlayButtonSize`] or one the theme's recipe names.
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.size = size.into();
    self
  }

  /// What it stands on, a [`PlayButtonVariant`] or one the theme's recipe names.
  pub fn variant(mut self, variant: impl Into<Name>) -> Self {
    self.variant = variant.into();
    self
  }

  /// What it is called while a press would play: "Play" unless told.
  pub fn play_label(mut self, label: &str) -> Self {
    self.play_label = Arc::from(label);
    self
  }

  /// What it is called while a press would pause: "Pause" unless told.
  pub fn pause_label(mut self, label: &str) -> Self {
    self.pause_label = Arc::from(label);
    self
  }

  /// What it is called while a press would play again from the start: "Replay" unless told.
  pub fn replay_label(mut self, label: &str) -> Self {
    self.replay_label = Arc::from(label);
    self
  }

  /// Called on a press, with what it is to be: playing if it was paused or had ended. A second handler is called
  /// after the first. Not called while it is loading or disabled.
  pub fn on_change(mut self, handler: impl Fn(bool) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// Puts the play button in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this play button over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this play button: what the component it is in says of it in this place.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws it as it is under the pointer, held down or focused from the keyboard, with nothing pointing at it: for
  /// a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the disc: what is pressed.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// What tells it apart from those beside it.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the play button in `theme`. Converting it into an element does this in the theme of what is being
  /// rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case {
      variant: Some(self.variant),
      size: Some(self.size),
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(PlayButtonState::Playing, self.playing && !self.replay)
    .state(PlayButtonState::Replay, self.replay)
    .state(PlayButtonState::Loading, self.loading)
    .state(PlayButtonState::Disabled, self.disabled);
    let label = if self.replay {
      self.replay_label
    } else if self.playing {
      self.pause_label
    } else {
      self.play_label
    };
    let view = PlayButtonView {
      label,
      playing: self.playing,
      loading: self.loading,
      replay: self.replay,
      disabled: self.disabled,
      variant: self.variant,
      size: self.size,
      recipe: theme
        .resolve::<PlayButton>(&case, Some(&self.rules))
        .owned_by(self.id.as_ref()),
      forced: self.forced,
      on_change: self.on_change,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<PlayButton>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<PlayButton> for Element {
  fn from(button: PlayButton) -> Self {
    button.build(&current_theme())
  }
}

/// A play button as a renderer sees it: what it was given, the style of its parts, and its behavior ready to be
/// dressed.
pub struct PlayButtonView {
  /// What it is called in the state it is in: the label of what a press would do.
  pub label: Arc<str>,
  pub playing: bool,
  pub loading: bool,
  pub replay: bool,
  pub disabled: bool,
  pub variant: Name,
  pub size: Name,
  /// The style of each part of this play button.
  pub recipe: ResolvedRecipe,
  forced: Option<Interaction>,
  on_change: Vec<Arc<dyn Fn(bool) + Send + Sync>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl PlayButtonView {
  /// The style of a part, as this play button is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The glyph's name in the state it is in: the spinner, play again, pause, or play.
  pub fn icon_name(&self) -> &'static str {
    if self.loading {
      icons::SPINNER
    } else if self.replay {
      icons::ROTATE_LEFT
    } else if self.playing {
      icons::PAUSE
    } else {
      icons::PLAY
    }
  }

  /// The play button's behavior with the style of its root: the element that is pressed, focused and disabled,
  /// with nothing in it yet. It is a button to what reads the tree, called by [`PlayButtonView::label`]; a press
  /// tells every handler what it is to be, unless it is loading or disabled.
  pub fn root(&self) -> components::Button {
    let mut root = pressable(&self.part(PlayButtonPart::Root), self.disabled)
      .describe("a11y_role", "button")
      .describe("a11y_name", self.label.clone());
    if !self.loading && !self.disabled && !self.on_change.is_empty() {
      let (handlers, next) = (self.on_change.clone(), self.replay || !self.playing);
      root = root.on_click(move |_| {
        for handler in &handlers {
          handler(next);
        }
      });
    }
    if self.loading {
      root = root.describe("a11y_busy", "true");
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root
  }

  /// The glyph in the style of its part, turning while it is loading.
  pub fn icon(&self) -> Element {
    glyph(
      &self.part(PlayButtonPart::Icon),
      Name::new(self.icon_name()),
      self.loading,
    )
  }

  /// How the default renderer builds a play button: the disc with its glyph in the middle.
  pub fn render(self) -> Element {
    self.root().child(self.icon()).into()
  }
}

impl crate::ui::Styled for PlayButton {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for PlayButton {
  const NAME: &'static str = "play_button";
  type Renderer = PlayButtonRenderer;

  fn parts() -> crate::ui::Declared {
    PlayButtonPart::declared()
  }

  fn states() -> crate::ui::Declared {
    PlayButtonState::declared()
  }

  /// The design's play button (`pencil/lurq-design-system.pen`, `Media/Play Button`, `GN7mn`): a disc of 40 in the
  /// primary color with a glyph of 16 in its foreground, the primary hover color under the pointer and the ring of
  /// the focus from the keyboard; 32 with 14, 48 with 20 and 64 with 28 for the other sizes. Loading it is at 75%
  /// and the spinner turns in the glyph's place; disabled it is at half. Over a picture (`Media/Play Button/Overlay`,
  /// `PfeN2`) the disc is black at 60% and the glyph and the ring white, the same in a light and a dark theme: the
  /// picture is not themed. The board draws no overlay under the pointer: it is black at 70%, the end of the
  /// board's bar gradient.
  fn recipe() -> Recipe {
    use PaletteColor::*;
    use PlayButtonPart::{Icon, Root};
    let white = Color::from_hex("#FFFFFF");
    let overlay = PlayButtonVariant::Overlay;
    let sized = |recipe: Recipe, size: PlayButtonSize, side: f32, icon: f32| {
      recipe
        .size(size, Root, |s| s.square(side))
        .size(size, Icon, |s| s.square(icon))
    };
    let recipe = Recipe::new()
      .base(Root, |s| {
        fading(
          s.square(40.0)
            .radius(RadiusSize::Full)
            .background(Primary)
            .foreground(PrimaryForeground)
            .cursor(CursorIcon::Pointer),
        )
      })
      .base(Icon, |s| s.square(16.0))
      .hovered(Root, |s| s.background(PrimaryHover))
      // Pressed it goes back to its fill under the pressed veil, as the primary button does, so that a press is seen.
      .pressed(Root, |s| s.background(Primary).veil(StatePressed))
      .focus_visible(Root, |s| s.focus_ring())
      .variant(overlay, Root, |s| s.background(Color::from_hex("#00000099")).foreground(white))
      .when(Selector::new().variant(overlay).hovered(), Root, |s| {
        s.background(Color::from_hex("#000000B3"))
      })
      .when(Selector::new().variant(overlay).pressed(), Root, |s| {
        s.background(Color::from_hex("#000000B3"))
      })
      // The theme's ring is drawn in the ring's color, which is lost on a picture: white, as wide and as far out.
      .when(Selector::new().variant(overlay).focus_visible(), Root, |s| {
        s.outline(Outline::new(2.0, white).offset(2.0))
      })
      .state(PlayButtonState::Loading, Root, |s| {
        s.opacity(LOADING_OPACITY).cursor(CursorIcon::Default)
      })
      .state(PlayButtonState::Disabled, Root, |s| {
        s.opacity(OpacityLevel::Disabled).cursor(CursorIcon::Default)
      });
    let recipe = sized(recipe, PlayButtonSize::Sm, 32.0, 14.0);
    let recipe = sized(recipe, PlayButtonSize::Lg, 48.0, 20.0);
    sized(recipe, PlayButtonSize::Xl, 64.0, 28.0)
  }
}
