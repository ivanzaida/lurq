//! The style of one part of a component: a patch in which every field is
//! optional.
//!
//! A recipe gives a part a patch for each case it styles (always, a variant,
//! a size, a state), and the patches that apply are laid one over another:
//! a field a later patch sets replaces the earlier one's, and a field it
//! leaves alone shows through. Every component uses this one type, so what
//! styles a button's root styles a switch's thumb.

use std::{sync::Arc, time::Duration};

use crate::{
  animation::{Animation, Easing, KeyframesId, Transition},
  app::theme::{ControlSize, Density, IconSize, MotionDuration, MotionEasing, OpacityLevel, Theme, TypographyStyle},
  layout::text_style::{FontStyle, FontWeight, TextStyle},
  node::{
    BackgroundColor, BoxShadow, BoxShadowValue, CursorIcon, Node, NodeUpdate, Outline, SpacingValue, Style, TextColor,
    border::{Border, ThemedBorderRadius},
    dimension::Dimension,
    radius_value::RadiusValue,
  },
};

/// The spread of the inset shadow a veil is: more than half of any box, so the shadow covers it.
const VEIL_SPREAD: f32 = 100_000.0;

/// A width or a height: a number of logical pixels, a step of one of the
/// theme's scales, or all the room there is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Length {
  Px(f32),
  /// The height of a control of a size.
  Control(ControlSize),
  /// The height of a row at a density.
  Density(Density),
  /// The side of an icon of a size.
  Icon(IconSize),
  /// All of the room the parent has.
  Fill,
  /// As large as the content is: undoes a size an earlier patch set.
  Auto,
}

impl Length {
  /// The length in `theme`: `None` for [`Self::Auto`], and for a step the theme does not have.
  pub(crate) fn resolve(self, theme: &Theme) -> Option<Dimension> {
    Some(match self {
      Self::Px(px) => Dimension::Px(px),
      Self::Control(size) => Dimension::Px(theme.control_sizes().try_get(size)?),
      Self::Density(density) => Dimension::Px(theme.densities().try_get(density)?),
      Self::Icon(size) => Dimension::Px(theme.icon_sizes().try_get(size)?),
      Self::Fill => Dimension::Pct(100.0),
      Self::Auto => return None,
    })
  }
}

impl From<f32> for Length {
  fn from(px: f32) -> Self {
    Self::Px(px)
  }
}

impl From<ControlSize> for Length {
  fn from(size: ControlSize) -> Self {
    Self::Control(size)
  }
}

impl From<Density> for Length {
  fn from(density: Density) -> Self {
    Self::Density(density)
  }
}

impl From<IconSize> for Length {
  fn from(size: IconSize) -> Self {
    Self::Icon(size)
  }
}

/// An opacity: a number from 0 to 1, or a level of the theme's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Opacity {
  Value(f32),
  Level(OpacityLevel),
}

impl From<f32> for Opacity {
  fn from(value: f32) -> Self {
    Self::Value(value)
  }
}

impl From<OpacityLevel> for Opacity {
  fn from(level: OpacityLevel) -> Self {
    Self::Level(level)
  }
}

/// The family a part's text is in: one of the theme's two, or one by name.
#[derive(Clone, Debug, PartialEq)]
pub enum FontFamily {
  /// The theme's family of interface text.
  Sans,
  /// The theme's family of code.
  Mono,
  /// A family name, an alias registered with `App::register_font`, or a
  /// CSS generic family.
  Named(Arc<str>),
}

impl From<&str> for FontFamily {
  fn from(name: &str) -> Self {
    Self::Named(Arc::from(name))
  }
}

/// The text of a part, each field optional: a typography role sets all of
/// them at once and any single one can then be set over it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextPatch {
  pub role: Option<TypographyStyle>,
  pub family: Option<FontFamily>,
  /// In logical pixels, before the person's text scale.
  pub size: Option<f32>,
  pub weight: Option<FontWeight>,
  pub style: Option<FontStyle>,
  pub line_height: Option<f32>,
  pub letter_spacing: Option<f32>,
}

impl TextPatch {
  fn merge(&mut self, over: &Self) {
    // A role sets every field: what an earlier patch set singly is under it.
    if over.role.is_some() {
      *self = Self {
        role: over.role,
        ..Self::default()
      };
    }
    macro_rules! take {
      ($($field:ident),*) => { $( if over.$field.is_some() { self.$field = over.$field.clone(); } )* };
    }
    take!(family, size, weight, style, line_height, letter_spacing);
  }

  fn is_empty(&self) -> bool {
    *self == Self::default()
  }

  /// The style of the part's text in `theme`: the role's, or the body
  /// style's where the patch names none, with the fields set over it. It
  /// names no color, so the text takes the foreground of the part it is in.
  pub fn resolve(&self, theme: &Theme) -> TextStyle {
    let mut style = theme.typography_style(self.role.unwrap_or(TypographyStyle::Body));
    match &self.family {
      Some(FontFamily::Sans) => style.font_family = theme.sans_family(),
      Some(FontFamily::Mono) => style.font_family = theme.mono_family(),
      Some(FontFamily::Named(name)) => style.font_family = name.clone(),
      None => {}
    }
    if let Some(size) = self.size {
      style.font_size = theme.scaled(size);
    }
    if let Some(weight) = self.weight {
      style.weight = weight;
    }
    if let Some(font_style) = self.style {
      style.style = font_style;
    }
    if let Some(line_height) = self.line_height {
      style.line_height = line_height;
    }
    if let Some(spacing) = self.letter_spacing {
      style.letter_spacing = spacing * theme.text_scale();
    }
    style
  }
}

/// How long a part's transition takes: a duration of the theme's, or a set one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TransitionDuration {
  Role(MotionDuration),
  Set(Duration),
}

/// How a part's transition moves: an easing of the theme's, or a set one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TransitionEasing {
  Role(MotionEasing),
  Set(Easing),
}

/// A transition of a part, with its duration and easing as the theme's roles
/// or as set values.
#[derive(Clone, Debug)]
pub struct PartTransition {
  /// The property that changes, as a `Transition` constructor gives it:
  /// `Transition::background_color()`. Its own duration and easing are replaced.
  pub what: Transition,
  pub duration: TransitionDuration,
  pub easing: TransitionEasing,
}

impl PartTransition {
  /// A transition of `what` that takes the theme's `duration` and moves by
  /// the theme's standard easing.
  pub fn new(what: Transition, duration: MotionDuration) -> Self {
    Self {
      what,
      duration: TransitionDuration::Role(duration),
      easing: TransitionEasing::Role(MotionEasing::Standard),
    }
  }

  pub fn easing(mut self, easing: MotionEasing) -> Self {
    self.easing = TransitionEasing::Role(easing);
    self
  }

  fn resolve(&self, theme: &Theme) -> Transition {
    let duration = match self.duration {
      TransitionDuration::Role(role) => theme.durations().try_get(role).unwrap_or_default(),
      TransitionDuration::Set(duration) => duration,
    };
    let easing = match self.easing {
      TransitionEasing::Role(role) => theme.easings().try_get(role).unwrap_or(Easing::Linear),
      TransitionEasing::Set(easing) => easing,
    };
    self.what.clone().duration(duration).easing(easing)
  }
}

/// A keyframe animation of a part, with its duration and easing as the
/// theme's roles or as set values: what the part plays as it appears
/// ([`PartStyle::enter`]) or as it leaves ([`PartStyle::exit`]).
#[derive(Clone, Debug)]
pub struct PartAnimation {
  /// The keyframes, which the window has: lurq's own (`ui::FADE_IN`) or the app's.
  pub keyframes: KeyframesId,
  pub duration: TransitionDuration,
  pub easing: TransitionEasing,
}

impl PartAnimation {
  /// `keyframes` played once over the theme's `duration`, moving by its `easing`.
  pub fn new(keyframes: KeyframesId, duration: MotionDuration, easing: MotionEasing) -> Self {
    Self {
      keyframes,
      duration: TransitionDuration::Role(duration),
      easing: TransitionEasing::Role(easing),
    }
  }

  fn resolve(&self, theme: &Theme) -> Animation {
    let duration = match self.duration {
      TransitionDuration::Role(role) => theme.durations().try_get(role).unwrap_or_default(),
      TransitionDuration::Set(duration) => duration,
    };
    let easing = match self.easing {
      TransitionEasing::Role(role) => theme.easings().try_get(role).unwrap_or(Easing::Linear),
      TransitionEasing::Set(easing) => easing,
    };
    Animation::new(self.keyframes).duration(duration).easing(easing)
  }
}

/// The style of one part of a component, every field optional.
///
/// Colors, radii, spacings and shadows are roles of the theme or concrete
/// values, as everywhere in lurq. Sizes are [`Length`]s and the text a
/// [`TextPatch`].
#[derive(Clone, Debug, Default)]
pub struct PartStyle {
  pub background: Option<BackgroundColor>,
  /// The color of the text and the icons in the part and under it.
  pub foreground: Option<TextColor>,
  /// A border on every side.
  pub border: Option<Border>,
  /// A line along the bottom of the part: what a link has under the pointer.
  pub underline: Option<Border>,
  /// A line along the start side of the part: what marks the chosen tab of a list that runs down.
  pub start_line: Option<Border>,
  pub radius: Option<ThemedBorderRadius>,
  pub shadow: Option<BoxShadowValue>,
  /// A color laid over the part's background and under its content: the
  /// theme's `StateHover` and `StatePressed` darken or lighten any fill.
  pub veil: Option<BackgroundColor>,
  pub outline: Option<Outline>,
  /// Whether the part has the theme's focus ring as its outline.
  pub focus_ring: Option<bool>,
  pub opacity: Option<Opacity>,
  pub padding_x: Option<SpacingValue>,
  pub padding_y: Option<SpacingValue>,
  /// The padding of one side, over what `padding_x` or `padding_y` gives it.
  pub padding_top: Option<SpacingValue>,
  pub padding_bottom: Option<SpacingValue>,
  pub padding_start: Option<SpacingValue>,
  pub padding_end: Option<SpacingValue>,
  /// The room between the part's children.
  pub gap: Option<SpacingValue>,
  pub width: Option<Length>,
  pub height: Option<Length>,
  pub min_width: Option<Length>,
  pub min_height: Option<Length>,
  pub max_width: Option<Length>,
  pub text: TextPatch,
  /// The transitions of the part. A patch that sets them replaces the list.
  pub transitions: Option<Vec<PartTransition>>,
  /// What the part plays as it appears: a panel over the window fading in.
  pub enter: Option<PartAnimation>,
  /// What the part plays as it leaves, kept until it ends where what holds it keeps leaving parts: an
  /// overlay's layer, or a child of an element with `presence()` that has an id or a key.
  pub exit: Option<PartAnimation>,
  pub cursor: Option<CursorIcon>,
}

impl PartStyle {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn background(mut self, color: impl Into<BackgroundColor>) -> Self {
    self.background = Some(color.into());
    self
  }

  pub fn foreground(mut self, color: impl Into<TextColor>) -> Self {
    self.foreground = Some(color.into());
    self
  }

  /// A border on every side.
  pub fn border(mut self, border: Border) -> Self {
    self.border = Some(border);
    self
  }

  /// A line along the bottom of the part, in place of a border on every
  /// side: the underline of a link.
  pub fn underline(mut self, line: Border) -> Self {
    self.underline = Some(line);
    self
  }

  /// A line along the start side of the part, in place of a border on every
  /// side: the mark of the chosen tab of a list that runs down.
  pub fn start_line(mut self, line: Border) -> Self {
    self.start_line = Some(line);
    self
  }

  pub fn radius(mut self, radius: impl Into<RadiusValue>) -> Self {
    self.radius = Some(ThemedBorderRadius::all(radius));
    self
  }

  /// A radius for each corner, from the top left clockwise: a part that is round at one end and square at the
  /// other, as the fill of a bar that starts at the window's edge is.
  pub fn corners(
    mut self,
    top_left: impl Into<RadiusValue>,
    top_right: impl Into<RadiusValue>,
    bottom_right: impl Into<RadiusValue>,
    bottom_left: impl Into<RadiusValue>,
  ) -> Self {
    self.radius = Some(ThemedBorderRadius::new(top_left, top_right, bottom_right, bottom_left));
    self
  }

  pub fn shadow(mut self, shadow: impl Into<BoxShadowValue>) -> Self {
    self.shadow = Some(shadow.into());
    self
  }

  /// A color laid over the part's background, under its content.
  pub fn veil(mut self, color: impl Into<BackgroundColor>) -> Self {
    self.veil = Some(color.into());
    self
  }

  pub fn outline(mut self, outline: Outline) -> Self {
    self.outline = Some(outline);
    self
  }

  /// The theme's focus ring as the part's outline: its width, offset and color.
  pub fn focus_ring(mut self) -> Self {
    self.focus_ring = Some(true);
    self
  }

  pub fn opacity(mut self, opacity: impl Into<Opacity>) -> Self {
    self.opacity = Some(opacity.into());
    self
  }

  pub fn padding_x(mut self, padding: impl Into<SpacingValue>) -> Self {
    self.padding_x = Some(padding.into());
    self
  }

  pub fn padding_y(mut self, padding: impl Into<SpacingValue>) -> Self {
    self.padding_y = Some(padding.into());
    self
  }

  /// The padding over the part's content alone, where it is not what is under it: a label that sits a
  /// pixel low in its row. It is laid over `padding_y`.
  pub fn padding_top(mut self, padding: impl Into<SpacingValue>) -> Self {
    self.padding_top = Some(padding.into());
    self
  }

  /// The padding under the part's content alone. It is laid over `padding_y`.
  pub fn padding_bottom(mut self, padding: impl Into<SpacingValue>) -> Self {
    self.padding_bottom = Some(padding.into());
    self
  }

  /// The padding before the part's content alone, at its left. It is laid over `padding_x`.
  pub fn padding_start(mut self, padding: impl Into<SpacingValue>) -> Self {
    self.padding_start = Some(padding.into());
    self
  }

  /// The padding after the part's content alone, at its right. It is laid over `padding_x`.
  pub fn padding_end(mut self, padding: impl Into<SpacingValue>) -> Self {
    self.padding_end = Some(padding.into());
    self
  }

  /// The padding of each side as the style gives it: top, end, bottom, start. A side of its own over the
  /// axis it is on; `None` where neither says.
  pub fn paddings(&self) -> [Option<SpacingValue>; 4] {
    [
      self.padding_top.or(self.padding_y),
      self.padding_end.or(self.padding_x),
      self.padding_bottom.or(self.padding_y),
      self.padding_start.or(self.padding_x),
    ]
  }

  pub fn gap(mut self, gap: impl Into<SpacingValue>) -> Self {
    self.gap = Some(gap.into());
    self
  }

  pub fn width(mut self, width: impl Into<Length>) -> Self {
    self.width = Some(width.into());
    self
  }

  pub fn height(mut self, height: impl Into<Length>) -> Self {
    self.height = Some(height.into());
    self
  }

  pub fn min_width(mut self, width: impl Into<Length>) -> Self {
    self.min_width = Some(width.into());
    self
  }

  /// The least the part is high: it is higher for what is in it.
  pub fn min_height(mut self, height: impl Into<Length>) -> Self {
    self.min_height = Some(height.into());
    self
  }

  pub fn max_width(mut self, width: impl Into<Length>) -> Self {
    self.max_width = Some(width.into());
    self
  }

  /// A width and a height that are the same: an icon, a square button.
  pub fn square(self, side: impl Into<Length>) -> Self {
    let side = side.into();
    self.width(side).height(side)
  }

  /// The typography role of the part's text: every field of its text at once.
  pub fn typography(mut self, role: impl Into<TypographyStyle>) -> Self {
    self.text.role = Some(role.into());
    self
  }

  pub fn font_family(mut self, family: impl Into<FontFamily>) -> Self {
    self.text.family = Some(family.into());
    self
  }

  /// The size of the part's text, in logical pixels.
  pub fn font_size(mut self, size: f32) -> Self {
    self.text.size = Some(size);
    self
  }

  pub fn weight(mut self, weight: FontWeight) -> Self {
    self.text.weight = Some(weight);
    self
  }

  pub fn font_style(mut self, style: FontStyle) -> Self {
    self.text.style = Some(style);
    self
  }

  pub fn line_height(mut self, line_height: f32) -> Self {
    self.text.line_height = Some(line_height);
    self
  }

  pub fn letter_spacing(mut self, spacing: f32) -> Self {
    self.text.letter_spacing = Some(spacing);
    self
  }

  /// Adds a transition of the part.
  pub fn transition(mut self, transition: PartTransition) -> Self {
    self.transitions.get_or_insert_with(Vec::new).push(transition);
    self
  }

  /// What the part plays as it appears.
  pub fn enter(mut self, animation: PartAnimation) -> Self {
    self.enter = Some(animation);
    self
  }

  /// What the part plays as it leaves.
  pub fn exit(mut self, animation: PartAnimation) -> Self {
    self.exit = Some(animation);
    self
  }

  pub fn cursor(mut self, cursor: CursorIcon) -> Self {
    self.cursor = Some(cursor);
    self
  }

  /// Whether the style sets nothing.
  pub fn is_empty(&self) -> bool {
    self.background.is_none()
      && self.foreground.is_none()
      && self.border.is_none()
      && self.underline.is_none()
      && self.start_line.is_none()
      && self.radius.is_none()
      && self.shadow.is_none()
      && self.veil.is_none()
      && self.outline.is_none()
      && self.focus_ring.is_none()
      && self.opacity.is_none()
      && self.padding_x.is_none()
      && self.padding_y.is_none()
      && self.padding_top.is_none()
      && self.padding_bottom.is_none()
      && self.padding_start.is_none()
      && self.padding_end.is_none()
      && self.gap.is_none()
      && self.width.is_none()
      && self.height.is_none()
      && self.min_width.is_none()
      && self.min_height.is_none()
      && self.max_width.is_none()
      && self.transitions.is_none()
      && self.enter.is_none()
      && self.exit.is_none()
      && self.cursor.is_none()
      && self.text.is_empty()
  }

  /// Lays `over` on this style: every field it sets replaces this one's.
  pub fn merge(&mut self, over: &Self) {
    macro_rules! take {
      ($($field:ident),*) => { $( if over.$field.is_some() { self.$field = over.$field.clone(); } )* };
    }
    take!(
      background,
      foreground,
      border,
      underline,
      start_line,
      radius,
      shadow,
      veil,
      outline,
      focus_ring,
      opacity,
      padding_x,
      padding_y,
      padding_top,
      padding_bottom,
      padding_start,
      padding_end,
      gap,
      width,
      height,
      min_width,
      min_height,
      max_width,
      transitions,
      enter,
      exit,
      cursor
    );
    self.text.merge(&over.text);
  }

  /// This style with `over` laid on it.
  pub fn merged(mut self, over: &Self) -> Self {
    self.merge(over);
    self
  }

  /// Gives a node the style, in `theme`: what the part looks like at rest.
  pub(crate) fn apply(&self, node: &mut Node, theme: &Theme) {
    if let Some(color) = &self.background {
      NodeUpdate::background(node, color.clone());
    }
    if let Some(color) = &self.foreground {
      NodeUpdate::foreground(node, color.clone());
    }
    if let Some(border) = &self.border {
      NodeUpdate::border(node, border.clone());
    }
    if let Some(line) = &self.underline {
      NodeUpdate::border_bottom(node, line.clone());
    }
    if let Some(line) = &self.start_line {
      NodeUpdate::border_left(node, line.clone());
    }
    if let Some(radius) = self.radius {
      NodeUpdate::corner_radius_top_left(node, radius.top_left);
      NodeUpdate::corner_radius_top_right(node, radius.top_right);
      NodeUpdate::corner_radius_bottom_right(node, radius.bottom_right);
      NodeUpdate::corner_radius_bottom_left(node, radius.bottom_left);
    }
    if let Some(shadow) = self.resolved_shadow(theme) {
      NodeUpdate::box_shadow(node, shadow);
    }
    if let Some(outline) = self.resolved_outline(theme) {
      NodeUpdate::outline(node, outline);
    }
    if let Some(opacity) = self.resolved_opacity(theme) {
      NodeUpdate::opacity(node, opacity);
    }
    if let Some(padding) = &self.padding_x {
      NodeUpdate::padding_horizontal(node, *padding);
    }
    if let Some(padding) = &self.padding_y {
      NodeUpdate::padding_vertical(node, *padding);
    }
    // A side of its own is laid over the axis it is on.
    if let Some(padding) = &self.padding_top {
      NodeUpdate::padding_top(node, *padding);
    }
    if let Some(padding) = &self.padding_bottom {
      NodeUpdate::padding_bottom(node, *padding);
    }
    if let Some(padding) = &self.padding_start {
      NodeUpdate::padding_left(node, *padding);
    }
    if let Some(padding) = &self.padding_end {
      NodeUpdate::padding_right(node, *padding);
    }
    if let Some(gap) = &self.gap {
      NodeUpdate::spacing(node, *gap);
    }
    if let Some(width) = self.width.and_then(|width| width.resolve(theme)) {
      NodeUpdate::width(node, width);
    }
    if let Some(height) = self.height.and_then(|height| height.resolve(theme)) {
      NodeUpdate::height(node, height);
    }
    if let Some(width) = self.min_width.and_then(|width| width.resolve(theme)) {
      NodeUpdate::min_width(node, width);
    }
    if let Some(height) = self.min_height.and_then(|height| height.resolve(theme)) {
      NodeUpdate::min_height(node, height);
    }
    if let Some(width) = self.max_width.and_then(|width| width.resolve(theme)) {
      NodeUpdate::max_width(node, width);
    }
    for transition in self.transitions.iter().flatten() {
      NodeUpdate::transition(node, transition.resolve(theme));
    }
    if let Some(animation) = &self.enter {
      NodeUpdate::animation(node, animation.resolve(theme));
    }
    if let Some(animation) = &self.exit {
      NodeUpdate::exit_animation(node, animation.resolve(theme));
    }
    if let Some(cursor) = self.cursor {
      NodeUpdate::cursor(node, cursor);
    }
  }

  /// The shadows of the part with its veil: a veil is an inset shadow that
  /// covers the whole box, painted over the background and under the content.
  fn resolved_shadow(&self, theme: &Theme) -> Option<BoxShadowValue> {
    let Some(veil) = &self.veil else {
      return self.shadow.clone();
    };
    let veil = BoxShadow::new(0.0, 0.0, 0.0, veil.clone()).spread(VEIL_SPREAD).inset();
    let under: Arc<[BoxShadow]> = match &self.shadow {
      Some(BoxShadowValue::Shadows(shadows)) => shadows.clone(),
      Some(BoxShadowValue::Theme(role)) => theme.shadows().try_get(*role).unwrap_or_else(|| Arc::from([])),
      None => Arc::from([]),
    };
    Some(BoxShadowValue::Shadows(
      std::iter::once(veil).chain(under.iter().cloned()).collect(),
    ))
  }

  fn resolved_outline(&self, theme: &Theme) -> Option<Outline> {
    match (&self.outline, self.focus_ring) {
      (Some(outline), _) => Some(outline.clone()),
      (None, Some(true)) => Some(theme.focus_ring().outline()),
      _ => None,
    }
  }

  fn resolved_opacity(&self, theme: &Theme) -> Option<f32> {
    match self.opacity? {
      Opacity::Value(value) => Some(value),
      Opacity::Level(level) => theme.opacities().try_get(level),
    }
  }

  /// The style as what a node shows in a state: only what is painted. A
  /// state changes no size, padding or text, so nothing moves under the
  /// pointer.
  pub(crate) fn state_style(&self, theme: &Theme) -> Option<Style> {
    let mut style = Style::new();
    let mut any = false;
    if let Some(color) = &self.background {
      style = style.background(color.clone());
      any = true;
    }
    if let Some(color) = &self.foreground {
      style = style.foreground(color.clone());
      any = true;
    }
    if let Some(border) = &self.border {
      style = style.border(border.clone());
      any = true;
    }
    if let Some(line) = &self.underline {
      style = style.border_bottom(line.clone());
      any = true;
    }
    if let Some(line) = &self.start_line {
      style = style.border_left(line.clone());
      any = true;
    }
    if let Some(radius) = self.radius {
      style.border_radius = Some(radius);
      any = true;
    }
    if let Some(shadow) = self.resolved_shadow(theme) {
      style = style.box_shadow(shadow);
      any = true;
    }
    if let Some(outline) = self.resolved_outline(theme) {
      style = style.outline(outline);
      any = true;
    }
    if let Some(opacity) = self.resolved_opacity(theme) {
      style = style.opacity(opacity);
      any = true;
    }
    if let Some(cursor) = self.cursor {
      style = style.cursor(cursor);
      any = true;
    }
    any.then_some(style)
  }

  /// Whether the style sets text: a family, a size, a role.
  pub fn sets_text(&self) -> bool {
    !self.text.is_empty()
  }
}

#[cfg(test)]
mod tests {
  use super::{Length, PartStyle};
  use crate::{
    app::theme::{ControlSize, PaletteColor, RadiusSize, Theme, TypographyStyle},
    layout::text_style::FontWeight,
    node::BackgroundColor,
  };

  #[test]
  fn a_later_patch_replaces_what_it_sets_and_leaves_the_rest() {
    let base = PartStyle::new()
      .background(PaletteColor::Primary)
      .radius(RadiusSize::Md)
      .height(ControlSize::Md);
    let over = PartStyle::new().background(PaletteColor::PrimaryHover);
    let merged = base.merged(&over);
    assert_eq!(
      merged.background,
      Some(BackgroundColor::Palette(PaletteColor::PrimaryHover))
    );
    assert_eq!(
      merged.radius,
      Some(crate::node::border::ThemedBorderRadius::all(RadiusSize::Md))
    );
    assert_eq!(merged.height, Some(Length::Control(ControlSize::Md)));
  }

  #[test]
  fn a_typography_role_sets_every_text_field_and_a_field_can_be_set_over_it() {
    let theme = Theme::new();
    let label = PartStyle::new()
      .typography(TypographyStyle::Button)
      .weight(FontWeight::Bold);
    let style = label.text.resolve(&theme);
    assert_eq!(
      style.font_size,
      theme.typography_style(TypographyStyle::Button).font_size
    );
    assert_eq!(style.weight, FontWeight::Bold);

    // A role laid over single fields starts the text again from that role.
    let again = label.merged(&PartStyle::new().typography(TypographyStyle::Caption));
    assert_eq!(
      again.text.resolve(&theme).weight,
      theme.typography_style(TypographyStyle::Caption).weight
    );
  }

  #[test]
  fn text_is_in_one_of_the_themes_two_families_or_in_one_by_name() {
    let theme = Theme::new();
    theme.set_sans_family("Inter");
    theme.set_mono_family("JetBrains Mono");
    let family = |style: PartStyle| style.text.resolve(&theme).font_family;
    assert_eq!(
      &*family(PartStyle::new().font_family(super::FontFamily::Mono)),
      "JetBrains Mono"
    );
    assert_eq!(&*family(PartStyle::new().font_family(super::FontFamily::Sans)), "Inter");
    assert_eq!(&*family(PartStyle::new().font_family("Georgia")), "Georgia");
  }

  #[test]
  fn a_state_style_holds_only_what_is_painted() {
    let theme = Theme::new();
    assert!(
      PartStyle::new()
        .height(40.0)
        .padding_x(8.0)
        .state_style(&theme)
        .is_none()
    );
    assert!(
      PartStyle::new()
        .background(PaletteColor::PrimaryHover)
        .state_style(&theme)
        .is_some()
    );
  }
}
