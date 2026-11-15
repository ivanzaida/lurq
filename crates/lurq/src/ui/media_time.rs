//! The time a player reads out: how far it is, and how long the whole is.

use std::sync::Arc;

use super::{
  Badge, BadgePart, BadgeVariant, FontFamily, names,
  part_style::PartStyle,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::pressable,
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    events::MouseEvent,
    theme::{PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::Row,
  layout::Alignment,
  node::{CursorIcon, Element, EventHandler, IntoMouseEventHandler, color::Color},
};

/// The speeds a player offers, slowest first; 1 is the speed a recording was made at.
pub const MEDIA_RATES: [f32; 6] = [0.5, 0.75, 1.0, 1.25, 1.5, 2.0];

/// A speed as a menu names it: "Normal" for 1, and "0.75×", "1.5×", "2×" for the others.
///
/// ```
/// use lurq::ui::media_rate_text;
///
/// assert_eq!(media_rate_text(1.0), "Normal");
/// assert_eq!(media_rate_text(1.5), "1.5×");
/// assert_eq!(media_rate_text(2.0), "2×");
/// ```
pub fn media_rate_text(rate: f32) -> String {
  if (rate - 1.0).abs() < f32::EPSILON {
    "Normal".to_string()
  } else {
    format!("{rate}×")
  }
}

/// A number of seconds as a player reads it out: `1:24`, and `1:02:45` with `hours` or from an hour up. A time
/// that is not a number, or below nothing, reads `0:00`.
///
/// ```
/// use lurq::ui::media_time_text;
///
/// assert_eq!(media_time_text(84.0, false), "1:24");
/// assert_eq!(media_time_text(3765.0, false), "1:02:45");
/// assert_eq!(media_time_text(84.0, true), "0:01:24");
/// ```
pub fn media_time_text(seconds: f64, hours: bool) -> String {
  let whole = if seconds.is_finite() && seconds > 0.0 {
    seconds.floor() as u64
  } else {
    0
  };
  let (h, m, s) = (whole / 3600, whole / 60 % 60, whole % 60);
  if hours || h > 0 {
    format!("{h}:{m:02}:{s:02}")
  } else {
    format!("{m}:{s:02}")
  }
}

/// What a time not known yet reads.
const UNKNOWN: &str = "-:--";

/// The parts of a time a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MediaTimePart {
  /// The row of the readings; what is pressed where the time is pressed.
  Root,
  /// How far it is: the elapsed time, or what is left with a minus before it.
  Elapsed,
  /// The "/" between the elapsed time and the whole.
  Divider,
  /// How long the whole is.
  Total,
  /// The badge that stands for the time of a live stream: what this recipe has for it is laid over the badge's.
  Live,
}

names!(MediaTimePart {
  Root => "root",
  Elapsed => "elapsed",
  Divider => "divider",
  Total => "total",
  Live => "live",
});

/// The states of a time a recipe styles, besides what the pointer and the keyboard do to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MediaTimeState {
  /// It stands over a picture: white, the same in a light and a dark theme.
  OnMedia,
  /// It reads what is left, alone.
  Remaining,
  /// Nothing is known of the length yet: it reads `-:-- / -:--`.
  Unknown,
  /// What plays is live: a badge says so in place of the time.
  Live,
  /// A press does something (a player turns the reading between elapsed and remaining).
  Pressable,
}

names!(MediaTimeState {
  OnMedia => "on_media",
  Remaining => "remaining",
  Unknown => "unknown",
  Live => "live",
  Pressable => "pressable",
});

/// The sizes of a time: the type of a compact player, of a player, and of a phone's player across.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MediaTimeSize {
  /// 11: a compact player's, an audio player's.
  Sm,
  /// 12.
  #[default]
  Md,
  /// 13: a phone's player held across.
  Lg,
}

names!(MediaTimeSize {
  Sm => "sm",
  Md => "md",
  Lg => "lg",
});

/// What builds a time's nodes: a theme's in place of the default one.
pub type MediaTimeRenderer = Arc<dyn Fn(MediaTimeView) -> Element + Send + Sync>;

/// The time a player reads out.
///
/// ```
/// use lurq::ui::MediaTime;
///
/// let time = MediaTime::new(84.0, Some(250.0));
/// // What is left, alone: "-2:46".
/// let left = MediaTime::new(84.0, Some(250.0)).remaining(true);
/// // Nothing is known of the length yet: "-:-- / -:--".
/// let unknown = MediaTime::new(0.0, None);
/// ```
///
/// It reads the elapsed time in the foreground and the "/" and the whole in the muted foreground, in the mono type
/// so that it does not jump as it counts; from an hour up both read hours. Given [`MediaTime::on_click`] it is a
/// button, a stop of Tab with the ring of the focus.
#[derive(Clone)]
pub struct MediaTime {
  position: f64,
  duration: Option<f64>,
  remaining: bool,
  on_media: bool,
  live: bool,
  live_text: Arc<str>,
  size: Name,
  label: Option<Arc<str>>,
  on_click: Vec<EventHandler<MouseEvent>>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl MediaTime {
  /// The time at `position` seconds of `duration`; of no duration where the length is not known yet.
  pub fn new(position: impl Into<f64>, duration: Option<f64>) -> Self {
    Self {
      position: position.into(),
      duration,
      remaining: false,
      on_media: false,
      live: false,
      live_text: Arc::from("LIVE"),
      size: MediaTimeSize::default().into(),
      label: None,
      on_click: Vec::new(),
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// Whether it reads what is left, with a minus before it, and nothing else.
  pub fn remaining(mut self, remaining: bool) -> Self {
    self.remaining = remaining;
    self
  }

  /// Whether it stands over a picture: white, the same in a light and a dark theme.
  pub fn on_media(mut self, on_media: bool) -> Self {
    self.on_media = on_media;
    self
  }

  /// Whether what plays is live: a badge in the destructive color reads "LIVE" in place of the time.
  pub fn live(mut self, live: bool) -> Self {
    self.live = live;
    self
  }

  /// What the badge of a live stream reads: "LIVE" unless told.
  pub fn live_text(mut self, text: &str) -> Self {
    self.live_text = Arc::from(text);
    self
  }

  /// A [`MediaTimeSize`], or a size the theme's recipe names.
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.size = size.into();
    self
  }

  /// What it is called, to what reads the tree: what it reads unless told ("1:24 of 4:10").
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// Called on a press: a player turns the reading between elapsed and remaining. Each call adds a handler.
  pub fn on_click(mut self, handler: impl IntoMouseEventHandler) -> Self {
    self.on_click.push(handler.into_event_handler());
    self
  }

  /// Puts the time in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this time over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this time: what the component it is in says of it in this place.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws it as it is under the pointer, held down or focused from the keyboard, with nothing pointing at it:
  /// for a gallery or a test. It shows only where it is pressed.
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

  /// Builds the time in `theme`. Converting it into an element does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let known = |seconds: Option<f64>| seconds.filter(|seconds| seconds.is_finite() && *seconds >= 0.0);
    let duration = known(self.duration);
    let unknown = !self.live && duration.is_none();
    let case = Case {
      variant: None,
      size: Some(self.size.clone()),
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(MediaTimeState::OnMedia, self.on_media)
    .state(MediaTimeState::Remaining, self.remaining && !unknown && !self.live)
    .state(MediaTimeState::Unknown, unknown)
    .state(MediaTimeState::Live, self.live)
    .state(MediaTimeState::Pressable, !self.on_click.is_empty());
    let view = MediaTimeView {
      position: known(Some(self.position)).unwrap_or(0.0),
      duration,
      remaining: self.remaining,
      on_media: self.on_media,
      live: self.live,
      live_text: self.live_text,
      size: self.size,
      label: self.label,
      recipe: theme.resolve::<MediaTime>(&case, Some(&self.rules)),
      on_click: self.on_click,
      forced: self.forced,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<MediaTime>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<MediaTime> for Element {
  fn from(time: MediaTime) -> Self {
    time.build(&current_theme())
  }
}

/// A time as a renderer sees it: what it was given, what it reads, and the style of its parts.
pub struct MediaTimeView {
  /// How far it is, in seconds.
  pub position: f64,
  /// How long the whole is, in seconds; none where it is not known yet.
  pub duration: Option<f64>,
  pub remaining: bool,
  pub on_media: bool,
  pub live: bool,
  pub live_text: Arc<str>,
  pub size: Name,
  /// The name it was given; none for what it reads.
  pub label: Option<Arc<str>>,
  /// The style of each part of this time.
  pub recipe: ResolvedRecipe,
  on_click: Vec<EventHandler<MouseEvent>>,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl MediaTimeView {
  /// The style of a part, as this time is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self
      .recipe
      .part(part)
      .forced(self.forced.filter(|_| !self.on_click.is_empty()))
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// What it reads, part by part: the elapsed time (or what is left), and the whole where it reads it.
  pub fn readings(&self) -> (String, Option<String>) {
    let Some(duration) = self.duration else {
      return (UNKNOWN.to_owned(), Some(UNKNOWN.to_owned()));
    };
    let hours = duration >= 3600.0;
    if self.remaining {
      (
        format!("-{}", media_time_text((duration - self.position).max(0.0), hours)),
        None,
      )
    } else {
      (
        media_time_text(self.position.min(duration), hours),
        Some(media_time_text(duration, hours)),
      )
    }
  }

  /// What it is called, to what reads the tree.
  pub fn name(&self) -> Arc<str> {
    if let Some(label) = &self.label {
      return label.clone();
    }
    if self.live {
      return self.live_text.clone();
    }
    match self.readings() {
      (elapsed, Some(total)) => Arc::from(format!("{elapsed} of {total}")),
      (left, None) => Arc::from(format!("{} left", left.trim_start_matches('-'))),
    }
  }

  /// A reading in the style of its part, on one line.
  fn reading(&self, part: MediaTimePart, text: &str) -> Element {
    let part = self.part(part);
    part.text(text).nowrap().part(&part).into()
  }

  /// What it shows: the readings with the divider between them, or the badge of a live stream.
  pub fn content(&self) -> Vec<Element> {
    if self.live {
      return vec![
        Badge::new(&self.live_text)
          .variant(BadgeVariant::Destructive)
          .rules(&self.recipe.rules_for(MediaTimePart::Live, BadgePart::Root))
          .build(self.theme()),
      ];
    }
    let (elapsed, total) = self.readings();
    let mut content = vec![self.reading(MediaTimePart::Elapsed, &elapsed)];
    if let Some(total) = total {
      content.push(self.reading(MediaTimePart::Divider, "/"));
      content.push(self.reading(MediaTimePart::Total, &total));
    }
    content
  }

  /// How the default renderer builds a time: a row of its readings, or a button of them where a press does
  /// something.
  pub fn render(self) -> Element {
    let root = self.part(MediaTimePart::Root);
    let name = self.name();
    if self.on_click.is_empty() {
      let mut row = Row::new()
        .align_items(Alignment::Center)
        .flex_shrink(0.0)
        .part(&root)
        .with_children(self.content())
        .describe("a11y_name", name);
      if let Some(id) = &self.id {
        row = row.id(id.clone());
      }
      if let Some(key) = &self.key {
        row = row.key(key.clone());
      }
      return row.into();
    }
    let mut button = pressable(&root, false)
      .align_items(Alignment::Center)
      .flex_shrink(0.0)
      .with_children(self.content())
      .describe("a11y_role", "button")
      .describe("a11y_name", name);
    for handler in self.on_click.iter().cloned() {
      button = button.on_click(handler);
    }
    if let Some(id) = &self.id {
      button = button.id(id.clone());
    }
    if let Some(key) = &self.key {
      button = button.key(key.clone());
    }
    button.into()
  }
}

impl Themed for MediaTime {
  const NAME: &'static str = "media_time";
  type Renderer = MediaTimeRenderer;

  /// The design's time (`pencil/lurq-design-system.pen`, Media/Time): the mono type at 12, 4 between the
  /// readings, the elapsed time in the foreground and the "/" and the whole in the muted foreground. Not loaded,
  /// the elapsed `-:--` is muted too. Over a picture it is white whatever the theme, the "/" and the whole
  /// `#FFFFFF99`. A compact player's is 11 and a phone's held across 13. Live, a badge in the destructive color reads
  /// "LIVE" in place of it (a badge, filled). Pressed to turn the reading, it is a button whose ring is the theme's,
  /// around a radius sm.
  fn recipe() -> Recipe {
    use MediaTimePart::{Divider, Elapsed, Live, Root, Total};
    use PaletteColor::*;
    let mono = |s: PartStyle| s.typography(TypographyStyle::Caption).font_family(FontFamily::Mono);
    let sized = |recipe: Recipe, size: MediaTimeSize, font: f32| {
      [Elapsed, Divider, Total]
        .into_iter()
        .fold(recipe, |recipe, part| recipe.size(size, part, |s| s.font_size(font)))
    };
    let recipe = Recipe::new()
      .base(Root, |s| s.gap(4.0))
      .base(Elapsed, |s| mono(s).foreground(Foreground))
      .base(Divider, |s| mono(s).foreground(MutedForeground))
      .base(Total, |s| mono(s).foreground(MutedForeground))
      // The board's live badge is solid: the destructive color, its text in the destructive foreground.
      .base(Live, |s| s.background(Destructive).foreground(DestructiveForeground))
      .state(MediaTimeState::Unknown, Elapsed, |s| s.foreground(MutedForeground))
      .state(MediaTimeState::OnMedia, Elapsed, |s| s.foreground(Color::from_hex("#FFFFFF")))
      .state(MediaTimeState::OnMedia, Divider, |s| s.foreground(Color::from_hex("#FFFFFF99")))
      .state(MediaTimeState::OnMedia, Total, |s| s.foreground(Color::from_hex("#FFFFFF99")))
      .when(
        Selector::new().state(MediaTimeState::OnMedia).state(MediaTimeState::Unknown),
        Elapsed,
        |s| s.foreground(Color::from_hex("#FFFFFF99")),
      )
      .state(MediaTimeState::Pressable, Root, |s| {
        s.radius(RadiusSize::Sm).cursor(CursorIcon::Pointer)
      })
      .focus_visible(Root, |s| s.focus_ring());
    let recipe = sized(recipe, MediaTimeSize::Sm, 11.0);
    sized(recipe, MediaTimeSize::Lg, 13.0)
  }
}
