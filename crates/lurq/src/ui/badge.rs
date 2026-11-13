//! A badge: a short label of a state or a count, read and not operated.

use std::sync::Arc;

use super::{
  names,
  part_style::{Length, PartStyle},
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::glyph,
  theme::{Themed, current_theme},
};
use crate::{
  app::theme::{BorderSize, IconSize, PaletteColor, RadiusSize, Theme, TypographyStyle},
  components::{Row, Stack},
  layout::{Alignment, StackAlignment, layout_kind::Justify, text_style::FontWeight},
  node::{Element, border::Border as Line, dimension::Dimension, outline::Outline},
};

/// What a badge says, and in which colors.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BadgeVariant {
  /// In the primary color: what is new, what stands out.
  #[default]
  Default,
  /// Quiet, in the secondary fill: a version, a kind.
  Secondary,
  /// An edge and no fill.
  Outline,
  /// Something that went well: passing, online.
  Success,
  /// Something to look at: modified, away.
  Warning,
  /// Something going on: building, syncing.
  Info,
  /// Something that went wrong, and a count of what waits: failed, 3 unread.
  Destructive,
}

names!(BadgeVariant {
  Default => "default",
  Secondary => "secondary",
  Outline => "outline",
  Success => "success",
  Warning => "warning",
  Info => "info",
  Destructive => "destructive",
});

/// How tall a badge is: 18, 22 or 26.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BadgeSize {
  Sm,
  #[default]
  Md,
  Lg,
}

names!(BadgeSize {
  Sm => "sm",
  Md => "md",
  Lg => "lg",
});

/// The shape of a badge's corners.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BadgeShape {
  /// Rounded all the way.
  #[default]
  Pill,
  /// The theme's small radius: a version, a key.
  Square,
}

/// The parts of a badge a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BadgePart {
  /// The badge itself: its fill, its edge and its height. A dot alone is its root.
  Root,
  /// Its text, or its number.
  Label,
  /// An icon before the text.
  Icon,
  /// A dot before the text, in the color of the variant.
  Dot,
  /// What holds the element a badge is on, and the badge at its corner.
  Anchor,
  /// A square at the top end corner of the element a badge is on, which is not drawn: the round end of the
  /// badge is centred on its centre. Its width is all a renderer reads of it.
  Corner,
}

names!(BadgePart {
  Root => "root",
  Label => "label",
  Icon => "icon",
  Dot => "dot",
  Anchor => "anchor",
  Corner => "corner",
});

/// The states of a badge a recipe styles. It has nothing the pointer or the
/// keyboard does to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BadgeState {
  /// It has a dot before its text.
  Dot,
  /// It is a dot alone.
  DotOnly,
  /// It is a count.
  Count,
  /// It is on the corner of another element.
  OnCorner,
  /// Its corners have the theme's small radius ([`BadgeShape::Square`]).
  Square,
}

names!(BadgeState {
  Dot => "dot",
  DotOnly => "dot_only",
  Count => "count",
  OnCorner => "on_corner",
  Square => "square",
});

/// What builds a badge's nodes: a theme's in place of the default one.
pub type BadgeRenderer = Arc<dyn Fn(BadgeView) -> Element + Send + Sync>;

/// A badge: a short label of a state or a count. Beta, passing, 3 unread.
///
/// ```
/// use lurq::ui::{Badge, BadgeSize, BadgeVariant, Button, icons};
///
/// let beta = Badge::new("Beta");
/// let passing = Badge::new("Passing").variant(BadgeVariant::Success).icon(icons::CHECK);
/// let online = Badge::new("Online").variant(BadgeVariant::Success).dot(true);
/// // A dot alone: what it says is what it is called.
/// let unread = Badge::new("Unread").variant(BadgeVariant::Info).dot_only(true);
/// // A count, "99+" past 99, nothing at all at 0.
/// let inbox = Badge::count(120).label("unread messages");
/// // On the top end corner of what it counts.
/// let bell = Badge::count(3).label("notifications").on(Button::new("Notifications").icon(icons::BELL).icon_only(true));
/// ```
///
/// A badge is read and not operated: it takes no press, no focus and no stop
/// of Tab. What reads the tree is told it is a `status`, called by its text.
pub struct Badge {
  label: Arc<str>,
  variant: Name,
  size: Name,
  shape: BadgeShape,
  icon: Option<Name>,
  dot: bool,
  dot_only: bool,
  count: Option<usize>,
  max: usize,
  show_zero: bool,
  on: Option<Element>,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Badge {
  /// A badge that says `label`.
  pub fn new(label: &str) -> Self {
    Self {
      label: Arc::from(label),
      variant: BadgeVariant::default().into(),
      size: BadgeSize::default().into(),
      shape: BadgeShape::default(),
      icon: None,
      dot: false,
      dot_only: false,
      count: None,
      max: 99,
      show_zero: false,
      on: None,
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// A count: `n` in a round badge, in [`BadgeVariant::Destructive`] and [`BadgeSize::Sm`] as the design
  /// draws it, "99+" past its [`max`](Self::max), and nothing at all at 0 unless [`show_zero`](Self::show_zero)
  /// says so.
  pub fn count(n: usize) -> Self {
    Self {
      variant: BadgeVariant::Destructive.into(),
      size: BadgeSize::Sm.into(),
      count: Some(n),
      ..Self::new("")
    }
  }

  /// A [`BadgeVariant`], or a variant the theme's recipe names.
  pub fn variant(mut self, variant: impl Into<Name>) -> Self {
    self.variant = variant.into();
    self
  }

  /// A [`BadgeSize`], or a size the theme's recipe names.
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.size = size.into();
    self
  }

  /// The shape of its corners.
  pub fn shape(mut self, shape: BadgeShape) -> Self {
    self.shape = shape;
    self
  }

  /// An icon before the text, by its name in the theme's set.
  pub fn icon(mut self, name: impl Into<Name>) -> Self {
    self.icon = Some(name.into());
    self
  }

  /// Whether a dot in the variant's color stands before the text: online, away. The badge is then drawn with an
  /// edge and no fill, and the variant colors the dot.
  pub fn dot(mut self, dot: bool) -> Self {
    self.dot = dot;
    self
  }

  /// Whether the badge is a dot alone, in the variant's color. Its text is not drawn, and is still what it is
  /// called.
  pub fn dot_only(mut self, dot_only: bool) -> Self {
    self.dot_only = dot_only;
    self
  }

  /// The most a count shows: past it the count is `max` and a plus, "99+". 99 unless told.
  pub fn max(mut self, max: usize) -> Self {
    self.max = max;
    self
  }

  /// Whether a count of 0 is drawn. It is not, unless told.
  pub fn show_zero(mut self, show: bool) -> Self {
    self.show_zero = show;
    self
  }

  /// What a count counts, which what reads the tree is told with the number: "3 unread messages". On a badge
  /// that is not a count, its text, as [`Badge::new`] has it.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Arc::from(label);
    self
  }

  /// Puts the badge on the top end corner of `element`, the round end of the badge centred a little in from the
  /// corner, with a ring of the background around it: a dot or a count on what it is about. The two are one
  /// element, as large as `element`; the badge reaches past it, so what holds them gives it room or does not
  /// clip. A count that is not drawn leaves `element` alone.
  pub fn on(mut self, element: impl Into<Element>) -> Self {
    self.on = Some(element.into());
    self
  }

  /// Puts the badge in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this badge over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this badge: what the component
  /// it is in says of the badge in this place.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// The id of the badge itself (its root), not of what it is on.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the badge in `theme`. Converting a badge into an element does
  /// this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case {
      variant: Some(self.variant),
      size: Some(self.size),
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(BadgeState::Dot, self.dot && !self.dot_only)
    .state(BadgeState::DotOnly, self.dot_only)
    .state(BadgeState::Count, self.count.is_some())
    .state(BadgeState::OnCorner, self.on.is_some())
    .state(BadgeState::Square, self.shape == BadgeShape::Square);
    let text = match self.count {
      Some(n) if n > self.max => Arc::from(format!("{}+", self.max)),
      Some(n) => Arc::from(n.to_string()),
      None => self.label.clone(),
    };
    let view = BadgeView {
      text,
      label: self.label,
      variant: self.variant,
      size: self.size,
      shape: self.shape,
      icon: self.icon,
      dot: self.dot,
      dot_only: self.dot_only,
      count: self.count,
      shown: self.count.is_none_or(|n| n > 0 || self.show_zero),
      recipe: theme.resolve::<Badge>(&case, Some(&self.rules)),
      on: self.on,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Badge>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<Badge> for Element {
  fn from(badge: Badge) -> Self {
    badge.build(&current_theme())
  }
}

/// A badge as a renderer sees it: what it was given, and the style of its parts.
pub struct BadgeView {
  /// What the badge draws: its text, or its count as it is shown ("99+").
  pub text: Arc<str>,
  /// Its text as it was given, or what a count counts.
  pub label: Arc<str>,
  pub variant: Name,
  pub size: Name,
  pub shape: BadgeShape,
  /// The name of its icon, if it has one.
  pub icon: Option<Name>,
  pub dot: bool,
  pub dot_only: bool,
  /// The number of a count.
  pub count: Option<usize>,
  /// Whether the badge is drawn: a count of 0 is not, unless told.
  pub shown: bool,
  /// The style of each part of this badge.
  pub recipe: ResolvedRecipe,
  /// The element the badge is on, if it is on one.
  pub on: Option<Element>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl BadgeView {
  /// The style of a part, as this badge is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// What the badge is called to what reads the tree: its text, or a count's number with what it counts.
  pub fn name(&self) -> Arc<str> {
    match self.count {
      Some(n) if self.label.is_empty() => Arc::from(n.to_string()),
      Some(n) => Arc::from(format!("{n} {}", self.label)),
      None => self.label.clone(),
    }
  }

  /// The badge's root in its style, with nothing in it yet: a `status` to what reads the tree, called by
  /// [`name`](Self::name).
  pub fn root(&self) -> Row {
    let mut root = Row::new()
      .align_items(Alignment::Center)
      .justify(Justify::Center)
      .part(&self.part(BadgePart::Root))
      .describe("a11y_role", "status")
      .describe("a11y_name", self.name());
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root
  }

  /// The badge as the default renderer draws it, on no element: a dot alone, or the dot, the icon and the text
  /// it has, in its root.
  pub fn badge(&self) -> Row {
    let mut root = self.root();
    if self.dot_only {
      return root;
    }
    if self.dot {
      root = root.child(Row::new().part(&self.part(BadgePart::Dot)));
    }
    if let Some(name) = self.icon {
      root = root.child(glyph(&self.part(BadgePart::Icon), name, false));
    }
    let part = self.part(BadgePart::Label);
    root.child(Row::new().part(&part).child(part.text(&self.text).nowrap()))
  }

  /// How far the badge is moved from the top end corner of the element it is on, right and down, so that the
  /// round end of the badge is centred on the centre of the [`Corner`](BadgePart::Corner): half the badge's
  /// height less half the corner's side. `None` where the recipe gives the badge or the corner no height or width
  /// in pixels.
  pub fn corner_offset(&self) -> Option<(f32, f32)> {
    let theme = self.theme();
    let px = |length: Option<Length>| match length?.resolve(theme)? {
      Dimension::Px(px) => Some(px),
      _ => None,
    };
    let height = px(self.part(BadgePart::Root).rest().height)?;
    let corner = px(self.part(BadgePart::Corner).rest().width)?;
    let shift = (height - corner) / 2.0;
    Some((shift, -shift))
  }

  /// `element` with `badge` on its top end corner, moved by [`corner_offset`](Self::corner_offset). The anchor
  /// does not clip, so the badge is drawn where it reaches past the element.
  pub fn place(&self, element: Element, badge: Row) -> Element {
    let badge = match self.corner_offset() {
      Some((x, y)) => badge.offset(x, y),
      None => badge,
    };
    Stack::new()
      .stack_align(StackAlignment::TopEnd)
      .part(&self.part(BadgePart::Anchor))
      .overflow_visible()
      .child(element)
      .child(badge)
      .into()
  }

  /// How the default renderer builds a badge: the badge, on the element it is on where it is on one. A count
  /// that is not drawn is nothing, or the element alone.
  pub fn render(mut self) -> Element {
    let on = self.on.take();
    match (self.shown, on) {
      (true, Some(element)) => {
        let badge = self.badge();
        self.place(element, badge)
      }
      (true, None) => self.badge().into(),
      (false, Some(element)) => element,
      (false, None) => Element::new(),
    }
  }
}

impl Themed for Badge {
  const NAME: &'static str = "badge";
  type Renderer = BadgeRenderer;

  /// The design's badges (`pencil/lurq-design-system.pen`: Badge, Badge/Dot,
  /// Counter Badge, Indicator). A badge is 22 high and round, padded 8, its
  /// text 12/500, 4 between an icon and the text: in `Primary` with
  /// `PrimaryForeground`, in `Secondary`, with an edge of `Border` and no
  /// fill, or a status's muted fill with its strong text. Small is 18, padded
  /// 6, at 11; large is 26, padded 10, at 13. Square has the radius `Sm`. With
  /// a dot it has an edge and no fill, a dot of 7 in the variant's color 6
  /// before the text, padded 8 before and 10 after. A count is a small badge
  /// at 600, in `Destructive` with `DestructiveForeground`, at least as wide as
  /// it is high. On a corner a badge has a ring of 2 in `Background`, and a
  /// dot alone is 12 there; the round end of the badge is centred 3 in from
  /// the corner.
  fn recipe() -> Recipe {
    use BadgePart::{Corner, Dot, Icon, Label, Root};
    use BadgeState::{Count, DotOnly, OnCorner, Square};
    use PaletteColor::*;
    let edge = || Line::inside(BorderSize::Sm, PaletteColor::Border);
    let mut recipe = Recipe::new()
      .base(Root, |s| {
        s.height(22.0)
          .padding_x(8.0)
          .gap(4.0)
          .radius(RadiusSize::Full)
          .background(Primary)
          .foreground(PrimaryForeground)
      })
      .base(Label, |s| {
        s.typography(TypographyStyle::Caption).weight(FontWeight::Medium)
      })
      .base(Icon, |s| s.square(IconSize::Xs))
      .base(Dot, |s| s.square(7.0).radius(RadiusSize::Full).background(Primary))
      .base(Corner, |s| s.square(6.0))
      .variant(BadgeVariant::Secondary, Root, |s| {
        s.background(Secondary).foreground(SecondaryForeground)
      })
      .variant(BadgeVariant::Outline, Root, |s| {
        s.background(Background).foreground(Foreground).border(edge())
      });
    // A status: its muted fill with its strong text; a dot, and a dot alone, in the strong color.
    for (variant, strong, muted) in [
      (BadgeVariant::Success, Success, SuccessMuted),
      (BadgeVariant::Warning, Warning, WarningMuted),
      (BadgeVariant::Info, Info, InfoMuted),
      (BadgeVariant::Destructive, Destructive, DestructiveMuted),
    ] {
      recipe = recipe.variant(variant, Root, |s| s.background(muted).foreground(strong));
    }
    let strong = [
      (BadgeVariant::Default, Primary),
      (BadgeVariant::Secondary, SecondaryForeground),
      (BadgeVariant::Outline, Foreground),
      (BadgeVariant::Success, Success),
      (BadgeVariant::Warning, Warning),
      (BadgeVariant::Info, Info),
      (BadgeVariant::Destructive, Destructive),
    ];
    for (variant, color) in &strong {
      let color = color.clone();
      recipe = recipe.variant(*variant, Dot, |s| s.background(color));
    }

    // The sizes: a height, a padding, a size of text and of icon; with a dot, its padding.
    for (size, height, padding, font, icon) in [
      (BadgeSize::Sm, 18.0, 6.0, 11.0, Length::Px(10.0)),
      (BadgeSize::Lg, 26.0, 10.0, 13.0, Length::Icon(IconSize::Sm)),
    ] {
      recipe = recipe
        .size(size, Root, |s| s.height(height).padding_x(padding))
        .size(size, Label, |s| s.font_size(font))
        .size(size, Icon, |s| s.square(icon));
    }

    // With a dot before its text: an edge and no fill, the dot 6 before the text, 2 more after it than before.
    recipe = recipe.state(BadgeState::Dot, Root, |s| {
      s.background(Background)
        .foreground(Foreground)
        .border(edge())
        .gap(6.0)
        .padding_start(8.0)
        .padding_end(10.0)
    });
    for (size, start) in [(BadgeSize::Sm, 6.0), (BadgeSize::Lg, 10.0)] {
      recipe = recipe.when(Selector::new().size(size).state(BadgeState::Dot), Root, |s| {
        s.padding_start(start).padding_end(start + 2.0)
      });
    }

    // A count: a small badge at 600, solid where it is destructive, as round as it is high.
    recipe = recipe
      .state(Count, Label, |s| s.weight(FontWeight::SemiBold))
      .state(Count, Root, |s| s.min_width(22.0))
      .when(
        Selector::new().state(Count).variant(BadgeVariant::Destructive),
        Root,
        |s| s.background(Destructive).foreground(DestructiveForeground),
      );
    for (size, side) in [(BadgeSize::Sm, 18.0), (BadgeSize::Lg, 26.0)] {
      recipe = recipe.when(Selector::new().size(size).state(Count), Root, |s| s.min_width(side));
    }

    // A dot alone: a circle in the variant's strong color, 8 (6 small, 10 large), and 12 on a corner. A count
    // drawn as a dot alone is as wide as it is high too.
    recipe = recipe
      .state(DotOnly, Root, |s| s.square(8.0).min_width(0.0).padding_x(0.0))
      .when(Selector::new().state(DotOnly).state(OnCorner), Root, |s| s.square(12.0));
    for (size, side, on_corner) in [(BadgeSize::Sm, 6.0, 10.0), (BadgeSize::Lg, 10.0, 14.0)] {
      recipe = recipe
        .when(Selector::new().size(size).state(DotOnly), Root, |s| s.square(side))
        .when(Selector::new().size(size).state(DotOnly).state(OnCorner), Root, |s| {
          s.square(on_corner)
        });
    }
    for (variant, color) in strong {
      recipe = recipe.when(Selector::new().variant(variant).state(DotOnly), Root, |s| {
        s.background(color)
      });
    }

    recipe
      // The outline's edge, on a dot alone, is the dot's own color.
      .when(
        Selector::new().variant(BadgeVariant::Outline).state(DotOnly),
        Root,
        |s| s.border(Line::inside(BorderSize::Sm, Foreground)),
      )
      .state(Square, Root, |s| s.radius(RadiusSize::Sm))
      // On a corner, a ring of the background parts the badge from what it is on.
      .state(OnCorner, Root, |s| s.outline(Outline::new(2.0, Background)))
  }
}
