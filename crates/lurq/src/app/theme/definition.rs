//! A theme as one value: everything a [`Theme`](super::Theme) holds that is
//! the look of an app and not what a person or the system set.

use std::sync::Arc;

#[cfg(feature = "form")]
use super::FormTheme;
use super::{
  Density, ThemeBlurs, ThemeBorderSizes, ThemeBreakpoints, ThemeCaret, ThemeControlSizes, ThemeDensities,
  ThemeDurations, ThemeEasings, ThemeFocusRing, ThemeIconSizes, ThemeInner, ThemeOpacities, ThemePalette, ThemeRadii,
  ThemeShadows, ThemeSpacing, ThemeTypography,
};
#[cfg(feature = "markdown")]
use super::{ThemeMarkdown, ThemeTextMarks};
use crate::{app::window::Appearance, layout::scrollbar::ScrollBarStyle};

/// The look of an app as one value: its palettes, its type, and every scale.
///
/// [`Theme::apply`](super::Theme::apply) installs one at once, and
/// [`Theme::definition`](super::Theme::definition) reads the theme back as
/// one. A definition is plain data: an app's theme is a built-in one with
/// some of its fields changed.
///
/// ```
/// use lurq::{app::theme::{RadiusSize, ThemeDefinition}, node::color::Color};
///
/// let mut brand = ThemeDefinition::neutral();
/// brand.light.primary = Color::from_hex("#7c3aed");
/// if let Some(dark) = brand.dark.as_mut() {
///   dark.primary = Color::from_hex("#a78bfa");
/// }
/// brand.radii.set(RadiusSize::Md, 10.0);
/// ```
///
/// What a person or the system set is not part of it and stays as it is when
/// one is applied: the appearance the theme is in, the text scale, bold text
/// and reduced motion.
#[derive(Clone)]
#[non_exhaustive]
pub struct ThemeDefinition {
  /// The palette of the light appearance, and of every appearance when
  /// there is no dark one.
  pub light: ThemePalette,
  /// The palette of the dark appearance. `None` for a theme with one look.
  pub dark: Option<ThemePalette>,
  pub typography: ThemeTypography,
  pub radii: ThemeRadii,
  pub spacing: ThemeSpacing,
  pub border_sizes: ThemeBorderSizes,
  pub shadows: ThemeShadows,
  pub breakpoints: ThemeBreakpoints,
  pub caret: ThemeCaret,
  pub scrollbar: ScrollBarStyle,
  pub control_sizes: ThemeControlSizes,
  pub icon_sizes: ThemeIconSizes,
  pub densities: ThemeDensities,
  /// The density the theme is at.
  pub density: Density,
  pub durations: ThemeDurations,
  pub easings: ThemeEasings,
  pub opacities: ThemeOpacities,
  pub blurs: ThemeBlurs,
  pub focus_ring: ThemeFocusRing,
  /// What the theme says about components: recipes in place of the design's,
  /// rules laid over them, renderers.
  pub ui: crate::ui::UiDefinition,
  #[cfg(feature = "markdown")]
  pub markdown: ThemeMarkdown,
  #[cfg(feature = "markdown")]
  pub text_marks: ThemeTextMarks,
  #[cfg(feature = "form")]
  pub form: FormTheme,
}

impl ThemeDefinition {
  /// The design system's theme (`pencil/lurq-design-system.pen`), light and
  /// dark: what an app has when it sets none.
  pub fn neutral() -> Self {
    Self {
      light: ThemePalette::light(),
      dark: Some(ThemePalette::dark()),
      typography: ThemeTypography::default(),
      radii: ThemeRadii::default(),
      spacing: ThemeSpacing::default(),
      border_sizes: ThemeBorderSizes::default(),
      shadows: ThemeShadows::default(),
      breakpoints: ThemeBreakpoints::default(),
      caret: ThemeCaret::default(),
      scrollbar: ScrollBarStyle::default(),
      control_sizes: ThemeControlSizes::default(),
      icon_sizes: ThemeIconSizes::default(),
      densities: ThemeDensities::default(),
      density: Density::Default,
      durations: ThemeDurations::default(),
      easings: ThemeEasings::default(),
      opacities: ThemeOpacities::default(),
      blurs: ThemeBlurs::default(),
      focus_ring: ThemeFocusRing::default(),
      ui: crate::ui::UiDefinition::new(),
      #[cfg(feature = "markdown")]
      markdown: ThemeMarkdown::default(),
      #[cfg(feature = "markdown")]
      text_marks: ThemeTextMarks::default(),
      #[cfg(feature = "form")]
      form: FormTheme::default(),
    }
  }

  /// The theme of lurq 0.61: its palette color for color, with no dark one,
  /// its typography, radii and shadows. An app written for 0.61 that applies it looks as it did.
  pub fn legacy() -> Self {
    Self {
      light: ThemePalette::legacy(),
      dark: None,
      typography: ThemeTypography::legacy(),
      radii: ThemeRadii::legacy(),
      shadows: ThemeShadows::legacy(),
      ..Self::neutral()
    }
  }

  /// The design's tokens with no styling for components: every scale and both
  /// palettes of [`Self::neutral`], and no recipe, so every part of every
  /// component comes out unstyled and every component still works. The base
  /// for a look that shares nothing with the design's.
  pub fn bare() -> Self {
    Self {
      ui: crate::ui::UiDefinition::bare(),
      ..Self::neutral()
    }
  }

  /// The palette of `appearance`: the light one where there is no dark one.
  pub fn palette(&self, appearance: Appearance) -> &ThemePalette {
    match (appearance, &self.dark) {
      (Appearance::Dark, Some(dark)) => dark,
      _ => &self.light,
    }
  }
}

impl Default for ThemeDefinition {
  fn default() -> Self {
    Self::neutral()
  }
}

impl ThemeInner {
  /// Takes every value of `definition`. What a person or the system set stays.
  pub(super) fn apply(&mut self, definition: ThemeDefinition) {
    self.palette = definition.palette(self.appearance).clone();
    self.palettes = definition.dark.map(|dark| (definition.light.clone(), dark));
    if self.palettes.is_none() {
      self.palette = definition.light;
    }
    self.base_typography = definition.typography;
    self.adjust_typography();
    self.radii = definition.radii;
    self.spacing = definition.spacing;
    self.border_sizes = definition.border_sizes;
    self.shadows = Arc::new(definition.shadows);
    self.breakpoints = definition.breakpoints;
    self.caret = definition.caret;
    self.scrollbar = definition.scrollbar;
    self.control_sizes = definition.control_sizes;
    self.icon_sizes = definition.icon_sizes;
    self.densities = definition.densities;
    self.density = definition.density;
    self.durations = definition.durations;
    self.easings = definition.easings;
    self.opacities = definition.opacities;
    self.blurs = definition.blurs;
    self.focus_ring = definition.focus_ring;
    self.ui = definition.ui;
    #[cfg(feature = "markdown")]
    {
      self.markdown = definition.markdown;
      self.text_marks = Arc::new(definition.text_marks);
    }
    #[cfg(feature = "form")]
    {
      self.form = definition.form;
    }
  }

  /// The theme as it is now, as a definition.
  pub(super) fn definition(&self) -> ThemeDefinition {
    let (light, dark) = match &self.palettes {
      Some((light, dark)) => (light.clone(), Some(dark.clone())),
      None => (self.palette.clone(), None),
    };
    ThemeDefinition {
      light,
      dark,
      typography: self.base_typography.clone(),
      radii: self.radii.clone(),
      spacing: self.spacing.clone(),
      border_sizes: self.border_sizes.clone(),
      shadows: (*self.shadows).clone(),
      breakpoints: self.breakpoints,
      caret: self.caret,
      scrollbar: self.scrollbar.clone(),
      control_sizes: self.control_sizes.clone(),
      icon_sizes: self.icon_sizes.clone(),
      densities: self.densities.clone(),
      density: self.density,
      durations: self.durations.clone(),
      easings: self.easings.clone(),
      opacities: self.opacities.clone(),
      blurs: self.blurs.clone(),
      focus_ring: self.focus_ring.clone(),
      ui: self.ui.clone(),
      #[cfg(feature = "markdown")]
      markdown: self.markdown.clone(),
      #[cfg(feature = "markdown")]
      text_marks: (*self.text_marks).clone(),
      #[cfg(feature = "form")]
      form: self.form.clone(),
    }
  }
}
