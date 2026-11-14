//! A spinner: something is being done, and how far it is is not known. And
//! the overlay that lays one over a part of the page while that part loads.

use std::{sync::Arc, time::Duration};

use super::{
  icon::Icon,
  icons, names,
  part_style::{Length, Opacity, PartAnimation},
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::{FADE_IN, FADE_OUT},
  theme::{Themed, current_theme},
};
use crate::{
  animation::{AnimatableProperty, Animation, Easing, Keyframes, KeyframesId},
  app::theme::{IconSize, MotionDuration, MotionEasing, PaletteColor, RadiusSize, Theme, TypographyStyle},
  components::{Column, Grid, Row},
  layout::{Alignment, grid::GridTrack, layout_kind::Justify},
  node::{BackgroundColor, Element, dimension::Dimension, transform::Decomposed},
};

/// The keyframes of a dot of [`SpinnerKind::Dots`], which every window has:
/// whole, then faint, then whole again. Each dot plays them a little after
/// the one before it, so that a wave runs along them.
pub const SPINNER_DOTS: KeyframesId = KeyframesId::new(0xFF31);

/// The keyframes of a bar of [`SpinnerKind::Bars`], which every window has:
/// its height, then less than half of it about its middle, then its height
/// again. Each bar plays them a little after the one before it.
pub const SPINNER_BARS: KeyframesId = KeyframesId::new(0xFF32);

/// How long the wave of [`SpinnerKind::Dots`] takes to come back to the first dot.
pub const SPINNER_DOTS_PERIOD: Duration = Duration::from_millis(1200);

/// How long a bar of [`SpinnerKind::Bars`] takes to shrink and grow back.
pub const SPINNER_BARS_PERIOD: Duration = Duration::from_millis(1000);

/// How much of each dot shows at rest, from the first: the design's, a wave caught at a moment.
const DOT_OPACITIES: [f32; 3] = [1.0, 0.6, 0.3];

/// How high each bar is at rest, from the first, as a share of the height of the bars: the design's 8, 18,
/// 12, 20 and 10 of 20.
const BAR_HEIGHTS: [f32; 5] = [0.4, 0.9, 0.6, 1.0, 0.5];

/// How faint a dot is at the faintest, and how short a bar is at the shortest.
const LEAST: f32 = 0.3;

/// The keyframes of the dots and the bars. They play in phase, so every spinner on screen moves as one.
pub(crate) fn keyframes() -> [Keyframes; 2] {
  let dot = Keyframes::new(SPINNER_DOTS)
    .frame(0.0, |frame| {
      frame.set(AnimatableProperty::Opacity, 1.0);
    })
    .frame(0.5, |frame| {
      frame.set(AnimatableProperty::Opacity, LEAST);
    })
    .frame(1.0, |frame| {
      frame.set(AnimatableProperty::Opacity, 1.0);
    });
  let bar = Keyframes::new(SPINNER_BARS)
    .frame(0.0, |frame| {
      frame.set(AnimatableProperty::Transform, Decomposed::IDENTITY);
    })
    .frame(0.5, |frame| {
      frame.set(
        AnimatableProperty::Transform,
        Decomposed::IDENTITY.with_scale(1.0, LEAST + 0.1),
      );
    })
    .frame(1.0, |frame| {
      frame.set(AnimatableProperty::Transform, Decomposed::IDENTITY);
    });
  [dot, bar]
}

/// What a spinner draws.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SpinnerKind {
  /// A ring with a gap that turns: the design's spinner, and what a button turns while it loads.
  #[default]
  Ring,
  /// Three dots that fade one after another.
  Dots,
  /// Five bars that shrink and grow one after another.
  Bars,
}

/// The color of a spinner.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SpinnerVariant {
  /// The color of the text around it: the ring, the dots and the bars are drawn in it, so a spinner in a
  /// button is the color of the button's label.
  #[default]
  Current,
  /// The muted foreground: a spinner that says less than the text beside it.
  Muted,
  /// The primary color.
  Primary,
}

names!(SpinnerVariant {
  Current => "current",
  Muted => "muted",
  Primary => "primary",
});

/// How large a spinner is: the design's 16, 24 and 32. A spinner is 16 where
/// nothing says otherwise, the side of an icon beside a line of text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SpinnerSize {
  #[default]
  Sm,
  Md,
  Lg,
}

names!(SpinnerSize {
  Sm => "sm",
  Md => "md",
  Lg => "lg",
});

/// The parts of a spinner a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SpinnerPart {
  /// The spinner and its label, side by side.
  Root,
  /// What moves: the ring, as a square of its side; the row of the dots or of the bars.
  Indicator,
  /// A dot of [`SpinnerKind::Dots`].
  Dot,
  /// A bar of [`SpinnerKind::Bars`]: its width, and its height is a share of the indicator's.
  Bar,
  /// The text beside it, when it is drawn.
  Label,
}

names!(SpinnerPart {
  Root => "root",
  Indicator => "indicator",
  Dot => "dot",
  Bar => "bar",
  Label => "label",
});

/// The states of a spinner a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SpinnerState {
  /// It is [`SpinnerKind::Dots`].
  Dots,
  /// It is [`SpinnerKind::Bars`].
  Bars,
}

names!(SpinnerState {
  Dots => "dots",
  Bars => "bars",
});

/// What builds a spinner's nodes: a theme's in place of the default one.
pub type SpinnerRenderer = Arc<dyn Fn(SpinnerView) -> Element + Send + Sync>;

/// What a spinner is called to tooling when the app gives it no label.
const LOADING: &str = "Loading";

/// A spinner: something is being done, and how far it is is not known.
///
/// ```
/// use lurq::ui::{Spinner, SpinnerKind, SpinnerSize};
///
/// let plain = Spinner::new();
/// // Beside the text that says what is being done.
/// let inline = Spinner::new().label("Loading results…");
/// let large = Spinner::new().size(SpinnerSize::Lg);
/// let dots = Spinner::new().kind(SpinnerKind::Dots);
/// ```
///
/// It is a status to tooling that reads the tree, called by its label, or
/// "Loading" when it has none. It is drawn in the color of the text around
/// it, the ring, the dots and the bars alike, so a spinner in a button is the
/// color of the button's label.
/// Spinners move whether or not motion is reduced: one that stood still would
/// say nothing.
#[derive(Clone)]
pub struct Spinner {
  kind: SpinnerKind,
  variant: Name,
  size: Name,
  label: Option<Arc<str>>,
  label_hidden: bool,
  decorative: bool,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Spinner {
  /// A ring of 16 in the color of the text around it, called "Loading".
  pub fn new() -> Self {
    Self {
      kind: SpinnerKind::default(),
      variant: SpinnerVariant::default().into(),
      size: SpinnerSize::default().into(),
      label: None,
      label_hidden: false,
      decorative: false,
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// What the spinner draws: a ring, dots or bars.
  pub fn kind(mut self, kind: SpinnerKind) -> Self {
    self.kind = kind;
    self
  }

  /// A [`SpinnerVariant`], or a variant the theme's recipe names.
  pub fn variant(mut self, variant: impl Into<Name>) -> Self {
    self.variant = variant.into();
    self
  }

  /// A [`SpinnerSize`], or a size the theme's recipe names.
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.size = size.into();
    self
  }

  /// What is being done, drawn beside the spinner and what it is called:
  /// "Loading results…".
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// Whether the label is not drawn. It stays what the spinner is called.
  pub fn label_hidden(mut self, hidden: bool) -> Self {
    self.label_hidden = hidden;
    self
  }

  /// Whether the spinner is no status of its own, where what it is in says
  /// already what is being done: a button that is loading.
  pub fn decorative(mut self, decorative: bool) -> Self {
    self.decorative = decorative;
    self
  }

  /// Puts the spinner in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this spinner over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this spinner: what the component
  /// it is in says of the spinner in this place.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// The id of the root: the spinner with its label.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the spinner in `theme`. Converting a spinner into an element does
  /// this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case {
      variant: Some(self.variant),
      size: Some(self.size),
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(SpinnerState::Dots, self.kind == SpinnerKind::Dots)
    .state(SpinnerState::Bars, self.kind == SpinnerKind::Bars);
    let view = SpinnerView {
      kind: self.kind,
      variant: self.variant,
      size: self.size,
      label: self.label,
      label_hidden: self.label_hidden,
      decorative: self.decorative,
      recipe: theme.resolve::<Spinner>(&case, Some(&self.rules)),
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Spinner>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl Default for Spinner {
  fn default() -> Self {
    Self::new()
  }
}

impl From<Spinner> for Element {
  fn from(spinner: Spinner) -> Self {
    spinner.build(&current_theme())
  }
}

/// A spinner as a renderer sees it: what it was given and the style of its parts.
pub struct SpinnerView {
  pub kind: SpinnerKind,
  pub variant: Name,
  pub size: Name,
  /// The label the app gave it, if any.
  pub label: Option<Arc<str>>,
  pub label_hidden: bool,
  /// Whether it is no status of its own.
  pub decorative: bool,
  /// The style of each part of this spinner.
  pub recipe: ResolvedRecipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl SpinnerView {
  /// The style of a part, as this spinner is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// What the spinner is called: its label, or "Loading".
  pub fn name(&self) -> Arc<str> {
    self.label.clone().unwrap_or_else(|| Arc::from(LOADING))
  }

  /// A length of the recipe in logical pixels, where it is one.
  fn px(&self, length: Option<Length>) -> Option<f32> {
    match length?.resolve(self.theme())? {
      Dimension::Px(px) => Some(px),
      _ => None,
    }
  }

  /// An animation of the dots or the bars: once in `period`, without end, the `index`th a step after the one
  /// before it.
  fn wave(keyframes: KeyframesId, period: Duration, index: usize, count: usize) -> Animation {
    Animation::new(keyframes)
      .duration(period)
      .delay(period / (2 * count as u32) * index as u32)
      .easing(Easing::EASE_IN_OUT)
      .infinite()
  }

  /// What moves, in the style of its part: the ring, the dots or the bars.
  pub fn indicator(&self) -> Element {
    let part = self.part(SpinnerPart::Indicator);
    match self.kind {
      SpinnerKind::Ring => {
        let side = part.rest().width.unwrap_or(Length::Icon(IconSize::Md));
        Row::new()
          .part(&part)
          .child(Icon::new(icons::SPINNER).size(side).spin(true).build(self.theme()))
          .into()
      }
      SpinnerKind::Dots => {
        let dot = self.part(SpinnerPart::Dot);
        let count = DOT_OPACITIES.len();
        let mut row = Row::new().part(&part).align_items(Alignment::Center);
        for (index, opacity) in DOT_OPACITIES.into_iter().enumerate() {
          row = row.child(Row::new().part(&dot).opacity(opacity).animation(Self::wave(
            SPINNER_DOTS,
            SPINNER_DOTS_PERIOD,
            index,
            count,
          )));
        }
        row.into()
      }
      SpinnerKind::Bars => {
        let bar = self.part(SpinnerPart::Bar);
        let height = self.px(part.rest().height).unwrap_or(20.0);
        let count = BAR_HEIGHTS.len();
        let mut row = Row::new().part(&part).align_items(Alignment::Center);
        for (index, share) in BAR_HEIGHTS.into_iter().enumerate() {
          row = row.child(Row::new().part(&bar).height(height * share).animation(Self::wave(
            SPINNER_BARS,
            SPINNER_BARS_PERIOD,
            index,
            count,
          )));
        }
        row.into()
      }
    }
  }

  /// The label beside the spinner, in its part's typography, if it is drawn.
  pub fn label(&self) -> Option<Element> {
    let label = self.label.as_ref().filter(|_| !self.label_hidden)?;
    let part = self.part(SpinnerPart::Label);
    Some(Row::new().part(&part).child(part.text(label).nowrap()).into())
  }

  /// How the default renderer builds a spinner: what moves, and its label beside it. The root is a status to
  /// tooling, called by the label, unless the spinner is decorative.
  pub fn render(self) -> Element {
    let mut root = Row::new()
      .align_items(Alignment::Center)
      .part(&self.part(SpinnerPart::Root))
      .child(self.indicator());
    if let Some(label) = self.label() {
      root = root.child(label);
    }
    if !self.decorative {
      root = root.describe("a11y_role", "status").describe("a11y_name", self.name());
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root.into()
  }
}

impl Themed for Spinner {
  const NAME: &'static str = "spinner";
  type Renderer = SpinnerRenderer;

  /// The design's spinner (`pencil/lurq-design-system.pen`, Spinner,
  /// Loader/Dots, Loader/Bars, Loader/Inline): a ring of 16, 24 or 32;
  /// three dots of 8, 6 apart, in a row 16 high; five bars 3 wide and round,
  /// 3 apart, in a row 20 high; all in the color of the text around them,
  /// the foreground where nothing names one; and a label
  /// 8 beside it, 13 in the muted foreground. The dots and the bars grow with
  /// the ring's sizes, by a half and by twice.
  fn recipe() -> Recipe {
    use PaletteColor::{MutedForeground, Primary};
    use SpinnerPart::{Bar, Dot, Indicator, Label, Root};
    let recipe = Recipe::new()
      .base(Root, |s| s.gap(8.0))
      .base(Indicator, |s| s.square(IconSize::Md))
      .base(Label, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(13.0)
          .foreground(MutedForeground)
      })
      .base(Dot, |s| {
        s.square(8.0)
          .radius(RadiusSize::Full)
          .background(BackgroundColor::CurrentForeground)
      })
      .base(Bar, |s| {
        s.width(3.0)
          .radius(RadiusSize::Full)
          .background(BackgroundColor::CurrentForeground)
      })
      .size(SpinnerSize::Md, Indicator, |s| s.square(IconSize::Xl))
      .size(SpinnerSize::Lg, Indicator, |s| s.square(32.0))
      .state(SpinnerState::Dots, Indicator, |s| {
        s.width(Length::Auto).height(16.0).gap(6.0)
      })
      .state(SpinnerState::Bars, Indicator, |s| {
        s.width(Length::Auto).height(20.0).gap(3.0)
      });
    // The dots and the bars of a size: the height of their row, the room between them, and the side of a dot
    // or the width of a bar.
    let sized = |recipe: Recipe, size: SpinnerSize, scale: f32| {
      recipe
        .when(Selector::new().size(size).state(SpinnerState::Dots), Indicator, |s| {
          s.width(Length::Auto).height(16.0 * scale).gap(6.0 * scale)
        })
        .when(Selector::new().size(size).state(SpinnerState::Dots), Dot, |s| {
          s.square(8.0 * scale)
        })
        .when(Selector::new().size(size).state(SpinnerState::Bars), Indicator, |s| {
          s.width(Length::Auto).height(20.0 * scale).gap(3.0 * scale)
        })
        .when(Selector::new().size(size).state(SpinnerState::Bars), Bar, |s| {
          s.width(3.0 * scale)
        })
    };
    let recipe = sized(recipe, SpinnerSize::Md, 1.5);
    let recipe = sized(recipe, SpinnerSize::Lg, 2.0);
    let colored = |recipe: Recipe, variant: SpinnerVariant, color: PaletteColor| {
      recipe
        .variant(variant, Indicator, |s| s.foreground(color.clone()))
        .variant(variant, Dot, |s| s.background(color.clone()))
        .variant(variant, Bar, |s| s.background(color))
    };
    let recipe = colored(recipe, SpinnerVariant::Muted, MutedForeground);
    colored(recipe, SpinnerVariant::Primary, Primary)
  }
}

/// The parts of a loading overlay a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LoadingOverlayPart {
  /// The content and the veil over it, in one cell: as large as the content. It clips the veil to its corners.
  Root,
  /// What holds the content: out of input while it loads.
  Content,
  /// What lies over the content while it loads, with the spinner in its middle: it fades in and out.
  Veil,
  /// The fill of the veil, through which the content shows faintly: its background and how much of it shows
  /// (its opacity), which the veil is filled with.
  Scrim,
}

names!(LoadingOverlayPart {
  Root => "root",
  Content => "content",
  Veil => "veil",
  Scrim => "scrim",
});

/// The states of a loading overlay a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LoadingOverlayState {
  /// The content is loading: the veil is over it.
  Loading,
}

names!(LoadingOverlayState {
  Loading => "loading",
});

/// What builds a loading overlay's nodes: a theme's in place of the default one.
pub type LoadingOverlayRenderer = Arc<dyn Fn(LoadingOverlayView) -> Element + Send + Sync>;

/// A veil with a spinner over a part of the page while it loads: a panel,
/// a table, a card.
///
/// ```
/// use lurq::{components::Text, ui::{LoadingOverlay, Spinner}};
///
/// let loading = true;
/// let panel = LoadingOverlay::new(loading, Text::new("Recent activity"));
/// // With a spinner of its own, which says what is being done.
/// let saving = LoadingOverlay::new(loading, Text::new("Draft")).spinner(Spinner::new().label("Saving…"));
/// ```
///
/// While it is loading, the content takes no input: no press reaches it and
/// Tab passes it by. The veil fades in as the loading starts and out as it
/// ends. The overlay is as large as its content; the veil is clipped to the
/// root's corners, which a recipe rounds where the content is rounded.
pub struct LoadingOverlay {
  loading: bool,
  content: Element,
  spinner: Option<Spinner>,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl LoadingOverlay {
  /// `content`, with the veil over it while `loading` is true.
  pub fn new(loading: bool, content: impl Into<Element>) -> Self {
    Self {
      loading,
      content: content.into(),
      spinner: None,
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// The spinner in the middle of the veil, in place of a plain one.
  pub fn spinner(mut self, spinner: Spinner) -> Self {
    self.spinner = Some(spinner);
    self
  }

  /// Puts the overlay in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this overlay over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this overlay.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// The id of the root.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the overlay in `theme`. Converting an overlay into an element
  /// does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(LoadingOverlayState::Loading, self.loading);
    let view = LoadingOverlayView {
      loading: self.loading,
      content: Some(self.content),
      spinner: self.spinner,
      recipe: theme.resolve::<LoadingOverlay>(&case, Some(&self.rules)),
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<LoadingOverlay>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<LoadingOverlay> for Element {
  fn from(overlay: LoadingOverlay) -> Self {
    overlay.build(&current_theme())
  }
}

/// A loading overlay as a renderer sees it.
pub struct LoadingOverlayView {
  pub loading: bool,
  /// The content, until a renderer takes it.
  pub content: Option<Element>,
  /// The spinner the app gave it, if any.
  pub spinner: Option<Spinner>,
  /// The style of each part of this overlay.
  pub recipe: ResolvedRecipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl LoadingOverlayView {
  /// The style of a part, as this overlay is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The fill of the veil: the scrim's color with as much of it showing as the scrim's opacity says. It is a
  /// color and not an opacity of the veil, which its fade from nothing to whole would undo once it ends.
  fn scrim(&self) -> Option<BackgroundColor> {
    let scrim = self.part(LoadingOverlayPart::Scrim).rest();
    let theme = self.theme();
    let color = scrim.background?.resolve(&theme.palette())?;
    let shown = match scrim.opacity {
      Some(Opacity::Value(value)) => value,
      Some(Opacity::Level(level)) => theme.opacity_value(level),
      None => 1.0,
    };
    let alpha = (f32::from(color.a()) * shown.clamp(0.0, 1.0)).round() as u8;
    Some(BackgroundColor::Color(color.with_alpha(alpha)))
  }

  /// The veil, while the content is loading: the scrim's fill with the spinner in the middle of it. It has a
  /// key, so that the root, which keeps leaving children, keeps it while it fades out.
  pub fn veil(&self) -> Option<Element> {
    if !self.loading {
      return None;
    }
    let spinner = self.spinner.clone().unwrap_or_default();
    // As large as the spinner where the root measures what is in its cell, so that the veil does not widen
    // it, and stretched to the cell after.
    let mut veil = Row::new()
      .key("veil")
      .part(&self.part(LoadingOverlayPart::Veil))
      .align_items(Alignment::Center)
      .justify(Justify::Center)
      .grid_column(0)
      .grid_row(0)
      .child(spinner.build(self.theme()));
    if let Some(fill) = self.scrim() {
      veil = veil.background(fill);
    }
    Some(veil.into())
  }

  /// How the default renderer builds the overlay: the content, out of input while it loads, and the veil
  /// over it, both in one cell of the root.
  pub fn render(mut self) -> Element {
    let content = Column::new()
      .part(&self.part(LoadingOverlayPart::Content))
      .disabled(self.loading)
      .grid_column(0)
      .grid_row(0)
      .child(self.content.take().unwrap_or_else(|| Row::new().into()));
    // A grid of one cell that the content and the veil are both in, stretched to it. The cell is as large as
    // the content, whatever room the root is given: the veil covers the content and nothing beside it, and a
    // content that fills the room makes it as wide as the room.
    let mut root = Grid::new()
      .columns([GridTrack::Auto])
      .rows([GridTrack::Auto])
      .presence()
      .part(&self.part(LoadingOverlayPart::Root))
      .child(content);
    if let Some(veil) = self.veil() {
      root = root.child(veil);
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root.into()
  }
}

impl Themed for LoadingOverlay {
  const NAME: &'static str = "loading_overlay";
  type Renderer = LoadingOverlayRenderer;

  /// The design's loading overlay (`pencil/lurq-design-system.pen`, Loading
  /// Overlay): a veil in the background's color at 85 hundredths over the
  /// content, a spinner of 16 in its middle. It fades in and out over the
  /// theme's base duration.
  fn recipe() -> Recipe {
    use LoadingOverlayPart::{Scrim, Veil};
    Recipe::new()
      .base(Veil, |s| {
        s.enter(PartAnimation::new(
          FADE_IN,
          MotionDuration::Base,
          MotionEasing::Standard,
        ))
        .exit(PartAnimation::new(
          FADE_OUT,
          MotionDuration::Base,
          MotionEasing::Standard,
        ))
      })
      .base(Scrim, |s| s.background(PaletteColor::Background).opacity(0.85))
  }
}
