//! A status label: an icon and the word of a state, in the state's tone, with no box around them.

use std::sync::Arc;

use super::{
  icon::{Icon, SpinMotion},
  names,
  part_style::Length,
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::one_line,
  theme::{Themed, current_theme},
};
use crate::{
  app::theme::{IconSize, PaletteColor, Theme, TypographyStyle},
  components::Row,
  layout::{Alignment, text_style::FontWeight},
  node::Element,
};

/// The parts of a status label a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StatusLabelPart {
  /// The label itself: the color of its icon and its words, and the room between them.
  Root,
  /// The icon before the words: its side.
  Icon,
  /// The words.
  Label,
}

names!(StatusLabelPart {
  Root => "root",
  Icon => "icon",
  Label => "label",
});

/// The states of a status label a recipe styles. It has nothing the pointer or the keyboard does to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StatusLabelState {
  /// Its icon turns: what it says is under way.
  Spinning,
}

names!(StatusLabelState {
  Spinning => "spinning",
});

/// The tone a status label is in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum StatusLabelVariant {
  /// In the foreground: a state that is neither good nor bad, running.
  #[default]
  Neutral,
  /// In the muted foreground: what has not started or will not, a draft, queued, skipped.
  Muted,
  /// Something going on that is told: syncing, in review.
  Info,
  /// Something that went well: done, passed.
  Success,
  /// Something to look at: waiting for a person, paused.
  Warning,
  /// Something that went wrong: failed.
  Destructive,
}

names!(StatusLabelVariant {
  Neutral => "neutral",
  Muted => "muted",
  Info => "info",
  Success => "success",
  Warning => "warning",
  Destructive => "destructive",
});

/// How large a status label is: words of 12 beside an icon of 14, or of 11 beside one of 12.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum StatusLabelSize {
  Sm,
  #[default]
  Md,
}

names!(StatusLabelSize {
  Sm => "sm",
  Md => "md",
});

/// What builds a status label's nodes: a theme's in place of the default one.
pub type StatusLabelRenderer = Arc<dyn Fn(StatusLabelView) -> Element + Send + Sync>;

/// A status label: what a thing is doing, as an icon and a word in one tone, with no fill and no edge. The state
/// of a run in a row of a table, of a step in a page's header.
///
/// ```
/// use lurq::ui::{StatusLabel, StatusLabelVariant, icons};
///
/// let done = StatusLabel::new("Completed").icon(icons::CHECK_CIRCLE).variant(StatusLabelVariant::Success);
/// // Its icon turns while what it says is under way, as a spinner does.
/// let running = StatusLabel::new("Running · 03 Implement").icon(icons::SPINNER).spin(true);
/// let failed = StatusLabel::new("Failed").icon(icons::CLOSE_CIRCLE).variant(StatusLabelVariant::Destructive);
/// ```
///
/// A status label is read and not operated: it takes no press, no focus and no stop of Tab. Its words are one line,
/// cut with an ellipsis where its room is narrower than they are. What reads the tree is told it is a `status`,
/// called by its words.
pub struct StatusLabel {
  label: Arc<str>,
  icon: Option<Name>,
  spin: bool,
  motion: Option<SpinMotion>,
  variant: Name,
  size: Name,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl StatusLabel {
  /// A status label that says `label`.
  pub fn new(label: &str) -> Self {
    Self {
      label: Arc::from(label),
      icon: None,
      spin: false,
      motion: None,
      variant: StatusLabelVariant::default().into(),
      size: StatusLabelSize::default().into(),
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// The icon before the words, by its name in the theme's set.
  pub fn icon(mut self, name: impl Into<Name>) -> Self {
    self.icon = Some(name.into());
    self
  }

  /// Whether the icon turns, as a spinner does: what the label says is under way. It turns as the theme's
  /// [`SpinMotion`] says, or its own ([`spin_motion`](Self::spin_motion)).
  pub fn spin(mut self, spin: bool) -> Self {
    self.spin = spin;
    self
  }

  /// How the icon turns where it spins, in place of the theme's motion.
  pub fn spin_motion(mut self, motion: SpinMotion) -> Self {
    self.motion = Some(motion);
    self
  }

  /// A [`StatusLabelVariant`], or a tone the theme's recipe names.
  pub fn variant(mut self, variant: impl Into<Name>) -> Self {
    self.variant = variant.into();
    self
  }

  /// A [`StatusLabelSize`], or a size the theme's recipe names.
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.size = size.into();
    self
  }

  /// Puts the label in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this label over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this label: what the component it is in says of it in this place.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// The id of the label's root. Its icon is `<id>-icon`, its words `<id>-label`.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the label in `theme`. Converting a label into an element does this in the theme of what is being
  /// rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case {
      variant: Some(self.variant),
      size: Some(self.size),
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(StatusLabelState::Spinning, self.spin);
    let view = StatusLabelView {
      label: self.label,
      icon: self.icon,
      spin: self.spin,
      motion: self.motion.unwrap_or_else(|| theme.spin_motion()),
      variant: self.variant,
      size: self.size,
      recipe: theme
        .resolve::<StatusLabel>(&case, Some(&self.rules))
        .owned_by(self.id.as_ref()),
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<StatusLabel>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<StatusLabel> for Element {
  fn from(label: StatusLabel) -> Self {
    label.build(&current_theme())
  }
}

/// A status label as a renderer sees it: what it was given, and the style of its parts.
pub struct StatusLabelView {
  pub label: Arc<str>,
  /// The name of its icon, if it has one.
  pub icon: Option<Name>,
  /// Whether its icon turns.
  pub spin: bool,
  /// How its icon turns: its own motion, or the theme's.
  pub motion: SpinMotion,
  pub variant: Name,
  pub size: Name,
  /// The style of each part of this label.
  pub recipe: ResolvedRecipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl StatusLabelView {
  /// The style of a part, as this label is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The root in its style, with nothing in it yet: a `status` to what reads the tree, called by the words.
  pub fn root(&self) -> Row {
    let mut root = Row::new()
      .align_items(Alignment::Center)
      .min_width(0.0)
      .part(&self.part(StatusLabelPart::Root))
      .describe("a11y_role", "status")
      .describe("a11y_name", self.label.clone());
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root
  }

  /// The icon in the style of its part, turning where the label spins: `None` where the label has none.
  pub fn icon(&self) -> Option<Element> {
    let name = self.icon?;
    let part = self.part(StatusLabelPart::Icon);
    let side = part.rest().width.unwrap_or(Length::Icon(IconSize::Sm));
    let icon = Icon::new(name)
      .size(side)
      .spin(self.spin)
      .spin_motion(self.motion)
      .build(part.theme());
    Some(Row::new().flex_shrink(0.0).part(&part).child(icon).into())
  }

  /// The words, on one line cut with an ellipsis where the room is short.
  pub fn words(&self) -> Element {
    one_line(&self.part(StatusLabelPart::Label), &self.label)
  }

  /// How the default renderer builds a label: its icon and its words, side by side.
  pub fn render(self) -> Element {
    let mut root = self.root();
    if let Some(icon) = self.icon() {
      root = root.child(icon);
    }
    root.child(self.words()).into()
  }
}

impl crate::ui::Styled for StatusLabel {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for StatusLabel {
  const NAME: &'static str = "status_label";
  type Renderer = StatusLabelRenderer;

  fn parts() -> crate::ui::Declared {
    StatusLabelPart::declared()
  }

  fn states() -> crate::ui::Declared {
    StatusLabelState::declared()
  }

  /// A status label as the badge's words are set, with no box: an icon of 14 and words of 12 at medium weight, 6
  /// apart, in the tone's strong color (`Foreground`, `MutedForeground`, `Info`, `Success`, `Warning`,
  /// `Destructive`). Small is an icon of 12 and words of 11, 4 apart.
  fn recipe() -> Recipe {
    use PaletteColor::{Destructive, Foreground, Info, MutedForeground, Success, Warning};
    use StatusLabelPart::{Icon, Label, Root};
    let mut recipe = Recipe::new()
      .base(Root, |s| s.gap(6.0).foreground(Foreground))
      .base(Icon, |s| s.square(IconSize::Sm))
      .base(Label, |s| {
        s.typography(TypographyStyle::Caption).weight(FontWeight::Medium)
      })
      .size(StatusLabelSize::Sm, Root, |s| s.gap(4.0))
      .size(StatusLabelSize::Sm, Icon, |s| s.square(IconSize::Xs))
      .size(StatusLabelSize::Sm, Label, |s| s.font_size(11.0));
    for (variant, color) in [
      (StatusLabelVariant::Muted, MutedForeground),
      (StatusLabelVariant::Info, Info),
      (StatusLabelVariant::Success, Success),
      (StatusLabelVariant::Warning, Warning),
      (StatusLabelVariant::Destructive, Destructive),
    ] {
      recipe = recipe.variant(variant, Root, |s| s.foreground(color));
    }
    recipe
  }
}
