//! A switch: a setting that is on or off, and takes effect at once.

use std::sync::Arc;

use super::{
  names,
  part_style::{Length, PartTransition},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::{fading, pressable},
  theme::{Themed, current_theme},
};
use crate::{
  animation::Transition,
  app::theme::{MotionDuration, OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle},
  components::{self, Row},
  layout::{Alignment, layout_kind::Justify},
  node::{CursorIcon, Element, dimension::Dimension},
};

/// The parts of a switch a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SwitchPart {
  /// The switch and its label, side by side.
  Root,
  /// What is pressed: the rail the thumb moves on.
  Track,
  /// What moves: at the start of the track while the switch is off, at its end while it is on.
  Thumb,
  /// Its text, beside the track.
  Label,
}

names!(SwitchPart {
  Root => "root",
  Track => "track",
  Thumb => "thumb",
  Label => "label",
});

/// The states of a switch a recipe styles, besides what the pointer and the
/// keyboard do to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SwitchState {
  /// It is on.
  On,
  /// It takes no input.
  Disabled,
}

names!(SwitchState {
  On => "on",
  Disabled => "disabled",
});

/// What builds a switch's nodes: a theme's in place of the default one.
pub type SwitchRenderer = Arc<dyn Fn(SwitchView) -> Element + Send + Sync>;

/// A switch: a setting that is on or off and takes effect at once. Dark
/// mode, sync in the background, notifications paused.
///
/// ```
/// use lurq::ui::Switch;
///
/// let dark = Switch::new("Dark mode").on(true).on_change(|on| println!("dark: {on}"));
/// // The track alone, where a row of settings has the text already: its label is still what it is called.
/// let sync = Switch::new("Sync in background").label_hidden(true);
/// ```
///
/// A switch does not keep whether it is on: the app says so with
/// [`Switch::on`], and is told of a press with [`Switch::on_change`]. A press
/// of its label is a press of the switch.
pub struct Switch {
  label: Arc<str>,
  label_hidden: bool,
  on: bool,
  disabled: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  on_change: Vec<Arc<dyn Fn(bool) + Send + Sync>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Switch {
  /// A switch called `label`, with the label beside it.
  pub fn new(label: &str) -> Self {
    Self {
      label: Arc::from(label),
      label_hidden: false,
      on: false,
      disabled: false,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      on_change: Vec::new(),
      id: None,
      key: None,
    }
  }

  /// Whether the switch is its track alone. Its label stays what it is called.
  pub fn label_hidden(mut self, hidden: bool) -> Self {
    self.label_hidden = hidden;
    self
  }

  /// Whether the switch is on.
  pub fn on(mut self, on: bool) -> Self {
    self.on = on;
    self
  }

  /// Whether the switch takes no input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Puts the switch in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Called on a press, with what the switch is to be: on if it was off.
  pub fn on_change(mut self, handler: impl Fn(bool) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// Styles one part of this switch over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this switch: what the component
  /// it is in says of the switch in this place.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the switch as it is under the pointer, held down or focused from
  /// the keyboard, with nothing pointing at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the track: what is pressed.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the switch in `theme`. Converting a switch into an element does
  /// this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(SwitchState::On, self.on)
    .state(SwitchState::Disabled, self.disabled);
    let view = SwitchView {
      label: self.label,
      label_hidden: self.label_hidden,
      on: self.on,
      disabled: self.disabled,
      recipe: theme.resolve::<Switch>(&case, Some(&self.rules)),
      forced: self.forced,
      on_change: self.on_change,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Switch>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<Switch> for Element {
  fn from(switch: Switch) -> Self {
    switch.build(&current_theme())
  }
}

/// A switch as a renderer sees it: what it was given, the style of its
/// parts, and its behavior ready to be dressed.
pub struct SwitchView {
  pub label: Arc<str>,
  pub label_hidden: bool,
  pub on: bool,
  pub disabled: bool,
  /// The style of each part of this switch.
  pub recipe: ResolvedRecipe,
  forced: Option<Interaction>,
  on_change: Vec<Arc<dyn Fn(bool) + Send + Sync>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl SwitchView {
  /// The style of a part, as this switch is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// What a press does: every handler is told what the switch is to be.
  fn press(&self) -> Option<impl Fn() + Send + Sync + Clone + 'static> {
    let (handlers, next) = (self.on_change.clone(), !self.on);
    (!handlers.is_empty() && !self.disabled).then(|| {
      move || {
        for handler in &handlers {
          handler(next);
        }
      }
    })
  }

  /// The switch's behavior with the style of its track: the element that is
  /// pressed, focused and disabled, with nothing in it yet. It is a switch to
  /// what reads the tree, called by its label, and says whether it is on.
  pub fn track(&self) -> components::Button {
    let mut track = pressable(&self.part(SwitchPart::Track), self.disabled)
      .justify(Justify::Start)
      .describe("a11y_role", "switch")
      .describe("a11y_name", self.label.clone())
      .describe("a11y_checked", if self.on { "true" } else { "false" });
    if let Some(press) = self.press() {
      track = track.on_click(move |_| press());
    }
    if let Some(id) = &self.id {
      track = track.id(id.clone());
    }
    if let Some(key) = &self.key {
      track = track.key(key.clone());
    }
    track
  }

  /// How far the thumb moves from off to on: the room of the track that the
  /// thumb does not fill. `None` where the recipe gives the track or the
  /// thumb no width in pixels.
  pub fn travel(&self) -> Option<f32> {
    let theme = self.theme();
    let px = |length: Option<Length>| match length?.resolve(theme)? {
      Dimension::Px(px) => Some(px),
      _ => None,
    };
    let (track, thumb) = (self.part(SwitchPart::Track).rest(), self.part(SwitchPart::Thumb).rest());
    let padding = track
      .padding_x
      .map(|padding| padding.resolve(&theme.spacing(), 0.0))
      .unwrap_or(0.0);
    Some((px(track.width)? - 2.0 * padding - px(thumb.width)?).max(0.0))
  }

  /// The thumb in the style of its part, moved to the end of the track while
  /// the switch is on. It is moved, not laid out at the other end, so that a
  /// transition the recipe gives its offset slides it.
  pub fn thumb(&self) -> Row {
    let thumb = Row::new().part(&self.part(SwitchPart::Thumb));
    match self.travel() {
      // Off it has an offset too, of nothing: a transition runs from one offset to another.
      Some(travel) => thumb.offset(if self.on { travel } else { 0.0 }, 0.0),
      None => thumb,
    }
  }

  /// How the default renderer builds a switch: the track with its thumb, and the label beside it.
  pub fn render(self) -> Element {
    let mut track = self.track().child(self.thumb());
    // A track whose travel the recipe does not say holds its thumb at the end it belongs at.
    if self.on && self.travel().is_none() {
      track = track.justify(Justify::End);
    }
    let mut root = Row::new()
      .align_items(Alignment::Center)
      .part(&self.part(SwitchPart::Root))
      .child(track);
    if !self.label_hidden {
      let part = self.part(SwitchPart::Label);
      let mut label = Row::new().part(&part).child(part.text(&self.label).nowrap());
      if let Some(press) = self.press() {
        label = label.on_click(move |_| press());
      }
      root = root.child(label);
    }
    root.into()
  }
}

impl Themed for Switch {
  const NAME: &'static str = "switch";
  type Renderer = SwitchRenderer;

  /// The design's switch (`pencil/lurq-design-system.pen`, Switch): a track
  /// of 36 by 20, round, in the color of an input's edge, with a thumb of 16
  /// in the background's color 2 from its edge; the track is in the primary
  /// color while it is on. Its label is 8 beside it, in the body's type.
  fn recipe() -> Recipe {
    use PaletteColor::*;
    use SwitchPart::{Label, Root, Thumb, Track};
    Recipe::new()
      .base(Root, |s| s.gap(8.0).foreground(Foreground))
      .base(Track, |s| {
        fading(
          s.width(36.0)
            .height(20.0)
            .padding_x(2.0)
            .radius(RadiusSize::Full)
            .background(Input)
            .cursor(CursorIcon::Pointer),
        )
      })
      .base(Thumb, |s| {
        s.square(16.0)
          .radius(RadiusSize::Full)
          .background(Background)
          .transition(PartTransition::new(Transition::offset_x(), MotionDuration::Fast))
      })
      .base(Label, |s| {
        s.typography(TypographyStyle::Body).cursor(CursorIcon::Pointer)
      })
      .focus_visible(Track, |s| s.focus_ring())
      .state(SwitchState::On, Track, |s| s.background(Primary))
      .state(SwitchState::Disabled, Root, |s| s.opacity(OpacityLevel::Disabled))
      .state(SwitchState::Disabled, Track, |s| s.cursor(CursorIcon::Default))
      .state(SwitchState::Disabled, Label, |s| s.cursor(CursorIcon::Default))
  }
}

impl super::FieldControl for Switch {
  /// Called by the field's label where its own is hidden, since the label a person reads is then the field's, and
  /// disabled where the field is. A switch has no judgment.
  fn in_field(mut self, field: &super::InField) -> Element {
    if self.label_hidden {
      self.label = field.label.clone();
    }
    self.disabled |= field.disabled;
    self.into()
  }
}
