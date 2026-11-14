use std::{
  ops::Deref,
  sync::{Arc, RwLock, RwLockReadGuard},
};

use crate::{
  core::Signal,
  layout::{scrollbar::ScrollBarStyle, text_style::TextStyle},
  node::{box_shadow::BoxShadow, color::Color, dimension::Dimension},
};

mod border;
mod breakpoints;
mod caret;
mod definition;
#[cfg(feature = "form")]
mod form;
mod icons;
#[cfg(feature = "markdown")]
mod markdown;
mod motion;
mod palette;
mod radius;
mod role_name;
mod shadow;
mod spacing;
#[cfg(feature = "markdown")]
mod text_marks;
mod tokens;
mod typography;

pub use border::{BorderSize, ThemeBorderSizes};
pub use breakpoints::{Breakpoint, ThemeBreakpoints};
pub use caret::{CaretMode, ThemeCaret};
pub use definition::ThemeDefinition;
#[cfg(feature = "form")]
pub use form::{
  FormButtonRole, FormButtonTheme, FormCheckboxStyle, FormFieldTheme, FormInputTheme, FormSliderStyle, FormTextRole,
  FormTheme,
};
pub use icons::{IconGlyph, ThemeIcons};
#[cfg(feature = "markdown")]
pub use markdown::{
  MarkdownBlockStyle, MarkdownFocusRing, MarkdownInlineStyle, MarkdownLinkState, MarkdownTextStyle, ThemeMarkdown,
};
#[cfg_attr(not(feature = "winit"), allow(unused_imports))]
pub(crate) use motion::system_reduced_motion;
pub use palette::{PaletteColor, ThemePalette};
pub use radius::{RadiusSize, ThemeRadii};
pub use role_name::RoleName;
pub use shadow::{ShadowStyle, ThemeShadows};
pub use spacing::{SpacingSize, ThemeSpacing};
#[cfg(feature = "markdown")]
pub use text_marks::{TextMarkStyle, ThemeTextMarks};
pub use tokens::{
  BlurSize, ControlSize, Density, IconSize, MotionDuration, MotionEasing, OpacityLevel, ThemeBlurs, ThemeControlSizes,
  ThemeDensities, ThemeDurations, ThemeEasings, ThemeFocusRing, ThemeIconSizes, ThemeOpacities,
};
pub use typography::{ThemeFonts, ThemeTypography, TypographyStyle};

#[derive(Clone)]
pub struct Theme {
  inner: Arc<RwLock<ThemeInner>>,
  version_signal: Signal<u64>,
  /// The component styles resolved at the theme's version: see `ui::theme`.
  resolved: Arc<std::sync::Mutex<crate::ui::theme::ResolvedCache>>,
}

struct ThemeInner {
  palette: ThemePalette,
  border_sizes: ThemeBorderSizes,
  spacing: ThemeSpacing,
  radii: ThemeRadii,
  shadows: Arc<ThemeShadows>,
  breakpoints: ThemeBreakpoints,
  caret: ThemeCaret,
  scrollbar: ScrollBarStyle,
  control_sizes: ThemeControlSizes,
  icon_sizes: ThemeIconSizes,
  densities: ThemeDensities,
  /// The density the theme is at.
  density: Density,
  durations: ThemeDurations,
  easings: ThemeEasings,
  opacities: ThemeOpacities,
  blurs: ThemeBlurs,
  focus_ring: ThemeFocusRing,
  /// What each name of an icon is drawn with.
  icons: ThemeIcons,
  /// What the theme says about components: recipes and renderers.
  ui: crate::ui::UiDefinition,
  /// The styles as they are used: `set_typography`'s, at `text_scale`.
  typography: ThemeTypography,
  base_typography: ThemeTypography,
  text_scale: f32,
  bold_text: bool,
  /// The palettes of a theme that has one for each appearance, and the appearance it is in.
  palettes: Option<(ThemePalette, ThemePalette)>,
  appearance: crate::app::window::Appearance,
  follows_system: bool,
  /// The appearance the app chose, whatever the system's is; `None` takes the system's.
  preferred_appearance: Option<crate::app::window::Appearance>,
  /// The system's appearance, as last told.
  system_appearance: crate::app::window::Appearance,
  #[cfg(feature = "markdown")]
  text_marks: Arc<ThemeTextMarks>,
  #[cfg(feature = "markdown")]
  markdown: ThemeMarkdown,
  #[cfg(feature = "form")]
  form: FormTheme,
  /// What the app decided about reduced motion; `None` follows the system.
  reduced_motion: Option<bool>,
  /// What the system asks for, as the shell last read it.
  system_reduced_motion: bool,
  version: u64,
}

impl ThemeInner {
  fn adjust_typography(&mut self) {
    self.typography = self.base_typography.scaled(self.text_scale);
  }
}

pub struct ThemeRef<'a, T> {
  inner: RwLockReadGuard<'a, ThemeInner>,
  value: fn(&ThemeInner) -> &T,
}

impl<T> Deref for ThemeRef<'_, T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    (self.value)(&self.inner)
  }
}

/// The design system's theme: [`ThemeDefinition::neutral`].
impl Default for Theme {
  fn default() -> Self {
    Self::from_definition(ThemeDefinition::neutral())
  }
}

impl Theme {
  /// The design system's theme, light and dark, following the system: [`ThemeDefinition::neutral`].
  pub fn new() -> Self {
    Self::default()
  }

  /// The theme of lurq 0.61: [`ThemeDefinition::legacy`]. It has one palette, so it is light whatever the
  /// system is in.
  pub fn legacy() -> Self {
    Self::from_definition(ThemeDefinition::legacy())
  }

  /// A theme with the look of `definition`, following the system.
  pub fn from_definition(definition: ThemeDefinition) -> Self {
    let appearance = crate::app::window::Appearance::Light;
    let palette = definition.palette(appearance).clone();
    let palettes = definition.dark.clone().map(|dark| (definition.light.clone(), dark));
    Self {
      inner: Arc::new(RwLock::new(ThemeInner {
        palette,
        border_sizes: definition.border_sizes,
        spacing: definition.spacing,
        radii: definition.radii,
        shadows: Arc::new(definition.shadows),
        breakpoints: definition.breakpoints,
        caret: definition.caret,
        scrollbar: definition.scrollbar,
        control_sizes: definition.control_sizes,
        icon_sizes: definition.icon_sizes,
        densities: definition.densities,
        density: definition.density,
        durations: definition.durations,
        easings: definition.easings,
        opacities: definition.opacities,
        blurs: definition.blurs,
        focus_ring: definition.focus_ring,
        icons: definition.icons,
        ui: definition.ui,
        typography: definition.typography.clone(),
        base_typography: definition.typography,
        text_scale: 1.0,
        bold_text: false,
        palettes,
        appearance,
        follows_system: true,
        preferred_appearance: None,
        system_appearance: appearance,
        #[cfg(feature = "markdown")]
        text_marks: Arc::new(definition.text_marks),
        #[cfg(feature = "markdown")]
        markdown: definition.markdown,
        #[cfg(feature = "form")]
        form: definition.form,
        reduced_motion: None,
        system_reduced_motion: false,
        version: 0,
      })),
      version_signal: Signal::new(0),
      resolved: Arc::default(),
    }
  }

  /// A theme with no styling for components: [`ThemeDefinition::bare`].
  pub fn bare() -> Self {
    Self::from_definition(ThemeDefinition::bare())
  }

  /// Reads what the theme says about components.
  pub(crate) fn with_ui<R>(&self, read: impl FnOnce(&crate::ui::UiDefinition) -> R) -> R {
    read(&self.inner.read().unwrap().ui)
  }

  /// Changes what the theme says about components: components that read the theme render again.
  pub(crate) fn update_ui(&self, update: impl FnOnce(&mut crate::ui::UiDefinition)) {
    self.mutate_inner(|inner| update(&mut inner.ui));
  }

  pub(crate) fn resolved_cache(&self) -> &std::sync::Mutex<crate::ui::theme::ResolvedCache> {
    &self.resolved
  }

  /// Gives the theme the look of `definition`, all of it at once: components that read the theme render
  /// once. What a person or the system set stays: the appearance the theme is in, the text scale, bold text
  /// and reduced motion.
  ///
  /// ```
  /// use lurq::app::theme::{Theme, ThemeDefinition};
  ///
  /// let theme = Theme::new();
  /// // An app written for lurq 0.61 that wants the look it had.
  /// theme.apply(ThemeDefinition::legacy());
  /// ```
  pub fn apply(&self, definition: ThemeDefinition) {
    self.mutate_inner(|inner| inner.apply(definition));
  }

  /// The look of the theme as it is now, as one value: to keep, to compare, or to change and apply again.
  pub fn definition(&self) -> ThemeDefinition {
    self.inner.read().unwrap().definition()
  }

  pub fn lens<T: Clone + Send + Sync + 'static>(
    &self,
    getter: impl Fn(&Theme) -> T + Send + Sync + 'static,
    setter: impl Fn(&Theme, T) + Send + Sync + 'static,
  ) -> ThemeLens<T> {
    ThemeLens {
      theme: self.clone(),
      getter: Arc::new(getter),
      setter: Arc::new(setter),
    }
  }

  pub fn palette(&self) -> ThemeRef<'_, ThemePalette> {
    ThemeRef {
      inner: self.inner.read().unwrap(),
      value: |inner| &inner.palette,
    }
  }

  /// Sets the palette of the appearance the theme is in now. A theme with a palette for each appearance
  /// keeps the other one as it is: [`Theme::set_palettes`] sets both.
  pub fn set_palette(&self, palette: ThemePalette) {
    self.mutate_inner(|inner| {
      if let Some((light, dark)) = &mut inner.palettes {
        match inner.appearance {
          crate::app::window::Appearance::Light => *light = palette.clone(),
          crate::app::window::Appearance::Dark => *dark = palette.clone(),
        }
      }
      inner.palette = palette;
    });
  }

  /// Sets the color of one role, in every appearance: a brand color set once is the brand color in the
  /// light look and in the dark one. [`Theme::set_palette_color_for`] sets it in one.
  pub fn set_palette_color(&self, palette_color: impl Into<PaletteColor>, color: Color) {
    let role = palette_color.into();
    self.mutate_inner(|inner| {
      if let Some((light, dark)) = &mut inner.palettes {
        light.set(&role, color);
        dark.set(&role, color);
      }
      inner.palette.set(role, color);
    });
  }

  /// Sets the color of one role in the palette of one appearance. On a theme with one palette it sets that
  /// palette's, whatever `appearance` is.
  pub fn set_palette_color_for(
    &self,
    appearance: crate::app::window::Appearance,
    palette_color: impl Into<PaletteColor>,
    color: Color,
  ) {
    let role = palette_color.into();
    self.mutate_inner(|inner| {
      let Some((light, dark)) = &mut inner.palettes else {
        inner.palette.set(role, color);
        return;
      };
      match appearance {
        crate::app::window::Appearance::Light => light.set(&role, color),
        crate::app::window::Appearance::Dark => dark.set(&role, color),
      }
      if inner.appearance == appearance {
        inner.palette.set(role, color);
      }
    });
  }

  pub fn palette_color(&self, palette_color: impl Into<PaletteColor>) -> Color {
    self.inner.read().unwrap().palette.get(palette_color)
  }

  pub fn border_sizes(&self) -> ThemeRef<'_, ThemeBorderSizes> {
    ThemeRef {
      inner: self.inner.read().unwrap(),
      value: |inner| &inner.border_sizes,
    }
  }

  pub fn set_border_sizes(&self, border_sizes: ThemeBorderSizes) {
    self.mutate_inner(|inner| inner.border_sizes = border_sizes);
  }

  pub fn set_border_size_value(&self, size: impl Into<BorderSize>, value: f32) {
    self.mutate_inner(|inner| inner.border_sizes.set(size, value));
  }

  pub fn border_size_value(&self, size: impl Into<BorderSize>) -> f32 {
    self.inner.read().unwrap().border_sizes.get(size)
  }

  pub fn spacing(&self) -> ThemeRef<'_, ThemeSpacing> {
    ThemeRef {
      inner: self.inner.read().unwrap(),
      value: |inner| &inner.spacing,
    }
  }

  pub fn set_spacing(&self, spacing: ThemeSpacing) {
    self.mutate_inner(|inner| inner.spacing = spacing);
  }

  pub fn set_spacing_value(&self, size: impl Into<SpacingSize>, value: impl Into<Dimension>) {
    self.mutate_inner(|inner| inner.spacing.set(size, value));
  }

  pub fn spacing_value(&self, size: impl Into<SpacingSize>) -> Dimension {
    self.inner.read().unwrap().spacing.get(size)
  }

  pub fn radii(&self) -> ThemeRef<'_, ThemeRadii> {
    ThemeRef {
      inner: self.inner.read().unwrap(),
      value: |inner| &inner.radii,
    }
  }

  pub fn set_radii(&self, radii: ThemeRadii) {
    self.mutate_inner(|inner| inner.radii = radii);
  }

  pub fn set_radius_value(&self, size: impl Into<RadiusSize>, value: f32) {
    self.mutate_inner(|inner| inner.radii.set(size, value));
  }

  pub fn radius_value(&self, size: impl Into<RadiusSize>) -> f32 {
    self.inner.read().unwrap().radii.get(size)
  }

  pub fn shadows(&self) -> ThemeRef<'_, ThemeShadows> {
    ThemeRef {
      inner: self.inner.read().unwrap(),
      value: |inner| &inner.shadows,
    }
  }

  pub fn set_shadows(&self, shadows: ThemeShadows) {
    self.mutate_inner(|inner| inner.shadows = Arc::new(shadows));
  }

  /// Sets the shadows of one role; an empty list gives the role no shadow.
  pub fn set_shadow_style(&self, style: impl Into<ShadowStyle>, shadows: impl Into<Arc<[BoxShadow]>>) {
    self.mutate_inner(|inner| Arc::make_mut(&mut inner.shadows).set(style, shadows));
  }

  /// Panics when a [`ShadowStyle::Extra`] name is not in the table; see [`ThemeShadows::try_get`].
  pub fn shadow_style(&self, style: impl Into<ShadowStyle>) -> Arc<[BoxShadow]> {
    self.inner.read().unwrap().shadows.get(style)
  }

  /// The shared shadow table, for painting without copying it.
  pub(crate) fn shared_shadows(&self) -> Arc<ThemeShadows> {
    self.inner.read().unwrap().shadows.clone()
  }

  pub fn breakpoints(&self) -> ThemeRef<'_, ThemeBreakpoints> {
    ThemeRef {
      inner: self.inner.read().unwrap(),
      value: |inner| &inner.breakpoints,
    }
  }

  pub fn set_breakpoints(&self, breakpoints: ThemeBreakpoints) {
    self.mutate_inner(|inner| inner.breakpoints = breakpoints);
  }

  pub fn set_breakpoint_value(&self, breakpoint: Breakpoint, value: f32) {
    self.mutate_inner(|inner| inner.breakpoints.set(breakpoint, value));
  }

  pub fn breakpoint_value(&self, breakpoint: Breakpoint) -> f32 {
    self.inner.read().unwrap().breakpoints.get(breakpoint)
  }

  pub fn caret(&self) -> ThemeRef<'_, ThemeCaret> {
    ThemeRef {
      inner: self.inner.read().unwrap(),
      value: |inner| &inner.caret,
    }
  }

  pub fn set_caret(&self, caret: ThemeCaret) {
    self.mutate_inner(|inner| inner.caret = caret);
  }

  pub fn set_caret_mode(&self, mode: CaretMode) {
    self.mutate_inner(|inner| inner.caret.set_mode(mode));
  }

  pub fn caret_mode(&self) -> CaretMode {
    self.inner.read().unwrap().caret.mode()
  }

  pub fn scrollbar(&self) -> ThemeRef<'_, ScrollBarStyle> {
    ThemeRef {
      inner: self.inner.read().unwrap(),
      value: |inner| &inner.scrollbar,
    }
  }

  pub fn set_scrollbar(&self, scrollbar: ScrollBarStyle) {
    self.mutate_inner(|inner| inner.scrollbar = scrollbar);
  }

  pub fn control_sizes(&self) -> ThemeRef<'_, ThemeControlSizes> {
    ThemeRef {
      inner: self.inner.read().unwrap(),
      value: |inner| &inner.control_sizes,
    }
  }

  pub fn set_control_sizes(&self, control_sizes: ThemeControlSizes) {
    self.mutate_inner(|inner| inner.control_sizes = control_sizes);
  }

  /// The height of a control of a size, in logical pixels. Panics when an `Extra` name is not in the table.
  pub fn control_size_value(&self, role: impl Into<ControlSize>) -> f32 {
    self.inner.read().unwrap().control_sizes.get(role)
  }

  pub fn set_control_size_value(&self, role: impl Into<ControlSize>, value: f32) {
    self.mutate_inner(|inner| inner.control_sizes.set(role, value));
  }

  pub fn icon_sizes(&self) -> ThemeRef<'_, ThemeIconSizes> {
    ThemeRef {
      inner: self.inner.read().unwrap(),
      value: |inner| &inner.icon_sizes,
    }
  }

  pub fn set_icon_sizes(&self, icon_sizes: ThemeIconSizes) {
    self.mutate_inner(|inner| inner.icon_sizes = icon_sizes);
  }

  /// The side of an icon of a size, in logical pixels. Panics when an `Extra` name is not in the table.
  pub fn icon_size_value(&self, role: impl Into<IconSize>) -> f32 {
    self.inner.read().unwrap().icon_sizes.get(role)
  }

  pub fn set_icon_size_value(&self, role: impl Into<IconSize>, value: f32) {
    self.mutate_inner(|inner| inner.icon_sizes.set(role, value));
  }

  pub fn densities(&self) -> ThemeRef<'_, ThemeDensities> {
    ThemeRef {
      inner: self.inner.read().unwrap(),
      value: |inner| &inner.densities,
    }
  }

  pub fn set_densities(&self, densities: ThemeDensities) {
    self.mutate_inner(|inner| inner.densities = densities);
  }

  /// The height of a row at a density, in logical pixels. Panics when an `Extra` name is not in the table.
  pub fn density_value(&self, role: impl Into<Density>) -> f32 {
    self.inner.read().unwrap().densities.get(role)
  }

  pub fn set_density_value(&self, role: impl Into<Density>, value: f32) {
    self.mutate_inner(|inner| inner.densities.set(role, value));
  }

  /// The density the theme is at: [`Density::Default`] unless set. The rules
  /// of a recipe for a density ([`Recipe::density`](crate::ui::Recipe::density))
  /// hold while the theme is at it, and what draws rows of its own reads it:
  /// `theme.density_value(theme.density())` is the height of a row.
  pub fn density(&self) -> Density {
    self.track_access();
    self.inner.read().unwrap().density
  }

  /// Puts the theme at a density: a tool with much to show at
  /// [`Density::Compact`], an app for a finger at [`Density::Touch`].
  /// Everything drawn from the theme is built again.
  pub fn set_density(&self, density: impl Into<Density>) {
    let density = density.into();
    if self.inner.read().unwrap().density != density {
      self.mutate_inner(|inner| inner.density = density);
    }
  }

  pub fn durations(&self) -> ThemeRef<'_, ThemeDurations> {
    ThemeRef {
      inner: self.inner.read().unwrap(),
      value: |inner| &inner.durations,
    }
  }

  pub fn set_durations(&self, durations: ThemeDurations) {
    self.mutate_inner(|inner| inner.durations = durations);
  }

  /// How long a duration is. Panics when an `Extra` name is not in the table.
  pub fn duration_value(&self, role: impl Into<MotionDuration>) -> std::time::Duration {
    self.inner.read().unwrap().durations.get(role)
  }

  pub fn set_duration_value(&self, role: impl Into<MotionDuration>, value: std::time::Duration) {
    self.mutate_inner(|inner| inner.durations.set(role, value));
  }

  pub fn easings(&self) -> ThemeRef<'_, ThemeEasings> {
    ThemeRef {
      inner: self.inner.read().unwrap(),
      value: |inner| &inner.easings,
    }
  }

  pub fn set_easings(&self, easings: ThemeEasings) {
    self.mutate_inner(|inner| inner.easings = easings);
  }

  /// The curve of an easing. Panics when an `Extra` name is not in the table.
  pub fn easing_value(&self, role: impl Into<MotionEasing>) -> crate::animation::Easing {
    self.inner.read().unwrap().easings.get(role)
  }

  pub fn set_easing_value(&self, role: impl Into<MotionEasing>, value: crate::animation::Easing) {
    self.mutate_inner(|inner| inner.easings.set(role, value));
  }

  pub fn opacities(&self) -> ThemeRef<'_, ThemeOpacities> {
    ThemeRef {
      inner: self.inner.read().unwrap(),
      value: |inner| &inner.opacities,
    }
  }

  pub fn set_opacities(&self, opacities: ThemeOpacities) {
    self.mutate_inner(|inner| inner.opacities = opacities);
  }

  /// The opacity of a level, from 0 to 1. Panics when an `Extra` name is not in the table.
  pub fn opacity_value(&self, role: impl Into<OpacityLevel>) -> f32 {
    self.inner.read().unwrap().opacities.get(role)
  }

  pub fn set_opacity_value(&self, role: impl Into<OpacityLevel>, value: f32) {
    self.mutate_inner(|inner| inner.opacities.set(role, value));
  }

  pub fn blurs(&self) -> ThemeRef<'_, ThemeBlurs> {
    ThemeRef {
      inner: self.inner.read().unwrap(),
      value: |inner| &inner.blurs,
    }
  }

  pub fn set_blurs(&self, blurs: ThemeBlurs) {
    self.mutate_inner(|inner| inner.blurs = blurs);
  }

  /// The radius of a blur, in logical pixels. Panics when an `Extra` name is not in the table.
  pub fn blur_value(&self, role: impl Into<BlurSize>) -> f32 {
    self.inner.read().unwrap().blurs.get(role)
  }

  pub fn set_blur_value(&self, role: impl Into<BlurSize>, value: f32) {
    self.mutate_inner(|inner| inner.blurs.set(role, value));
  }

  /// The ring around the control the keyboard is on.
  pub fn focus_ring(&self) -> ThemeRef<'_, ThemeFocusRing> {
    ThemeRef {
      inner: self.inner.read().unwrap(),
      value: |inner| &inner.focus_ring,
    }
  }

  pub fn set_focus_ring(&self, focus_ring: ThemeFocusRing) {
    self.mutate_inner(|inner| inner.focus_ring = focus_ring);
  }

  /// What each name of an icon is drawn with: lurq's own set unless the theme says otherwise.
  pub fn icons(&self) -> ThemeRef<'_, ThemeIcons> {
    self.track_access();
    ThemeRef {
      inner: self.inner.read().unwrap(),
      value: |inner| &inner.icons,
    }
  }

  pub fn set_icons(&self, icons: ThemeIcons) {
    self.mutate_inner(|inner| inner.icons = icons);
  }

  /// The glyph the theme draws the icon `name` with, if it has one.
  pub fn icon(&self, name: impl Into<RoleName>) -> Option<IconGlyph> {
    self.track_access();
    self.inner.read().unwrap().icons.get(name).cloned()
  }

  /// Gives the icon `name` a glyph: a new icon, or another drawing of one there is. Every
  /// [`Icon`](crate::ui::Icon) of that name on screen is drawn again.
  pub fn set_icon(&self, name: impl Into<RoleName>, glyph: IconGlyph) {
    let name = name.into();
    self.mutate_inner(|inner| inner.icons.set(name, glyph));
  }

  pub fn typography(&self) -> ThemeRef<'_, ThemeTypography> {
    ThemeRef {
      inner: self.inner.read().unwrap(),
      value: |inner| &inner.typography,
    }
  }

  pub fn set_typography(&self, typography: ThemeTypography) {
    self.mutate_inner(|inner| {
      inner.base_typography = typography;
      inner.adjust_typography();
    });
  }

  pub fn set_typography_style(&self, typography_style: impl Into<TypographyStyle>, style: TextStyle) {
    self.mutate_inner(|inner| {
      inner.base_typography.set(typography_style, style);
      inner.adjust_typography();
    });
  }

  /// How much larger than set the typography's styles are: what a person asked the system for (a phone's
  /// Dynamic Type), 1 where they asked for nothing. A size that is not one of the typography's is scaled
  /// with [`Theme::scaled`].
  pub fn text_scale(&self) -> f32 {
    self.inner.read().unwrap().text_scale
  }

  pub fn set_text_scale(&self, scale: f32) {
    let scale = if scale.is_finite() && scale > 0.0 { scale } else { 1.0 };
    if self.text_scale() == scale {
      return;
    }
    self.mutate_inner(|inner| {
      inner.text_scale = scale;
      inner.adjust_typography();
    });
  }

  /// A size in logical pixels at the theme's text scale.
  pub fn scaled(&self, size: f32) -> f32 {
    size * self.text_scale()
  }

  /// Every text is drawn two weights heavier, whatever set its weight: a person asked the system for bold
  /// text.
  pub fn bold_text(&self) -> bool {
    self.inner.read().unwrap().bold_text
  }

  pub fn set_bold_text(&self, bold: bool) {
    if self.bold_text() == bold {
      return;
    }
    self.mutate_inner(|inner| inner.bold_text = bold);
  }

  /// Gives the theme a palette for each appearance: the one for the appearance it is in is its palette, now
  /// and when the appearance changes.
  pub fn set_palettes(&self, light: ThemePalette, dark: ThemePalette) {
    self.mutate_inner(|inner| {
      inner.palette = match inner.appearance {
        crate::app::window::Appearance::Light => light.clone(),
        crate::app::window::Appearance::Dark => dark.clone(),
      };
      inner.palettes = Some((light, dark));
    });
  }

  /// Light or dark: the system's where the theme follows it, and what [`Theme::set_appearance`] said.
  pub fn appearance(&self) -> crate::app::window::Appearance {
    self.inner.read().unwrap().appearance
  }

  pub fn set_appearance(&self, appearance: crate::app::window::Appearance) {
    if self.appearance() == appearance {
      return;
    }
    self.mutate_inner(|inner| {
      inner.appearance = appearance;
      if let Some((light, dark)) = &inner.palettes {
        inner.palette = match appearance {
          crate::app::window::Appearance::Light => light.clone(),
          crate::app::window::Appearance::Dark => dark.clone(),
        };
      }
    });
  }

  /// Whether the theme takes what a person set for the whole system: its appearance, its text size and bold
  /// text. It does unless told not to; an app with a look of its own, or its own setting for these, says no
  /// and sets them itself.
  pub fn follows_system(&self) -> bool {
    self.inner.read().unwrap().follows_system
  }

  pub fn set_follows_system(&self, follows: bool) {
    self.inner.write().unwrap().follows_system = follows;
  }

  /// The appearance the app chose with [`Theme::set_preferred_appearance`], or `None` where it takes the
  /// system's.
  pub fn preferred_appearance(&self) -> Option<crate::app::window::Appearance> {
    self.inner.read().unwrap().preferred_appearance
  }

  /// Chooses light or dark for the app whatever the system is in: the setting an app gives a person as
  /// "Light, Dark, System". `None`, the default, is "System". Unlike [`Theme::set_follows_system`]`(false)`,
  /// the theme goes on taking the system's text size and bold text.
  pub fn set_preferred_appearance(&self, appearance: Option<crate::app::window::Appearance>) {
    let system = {
      let mut inner = self.inner.write().unwrap();
      inner.preferred_appearance = appearance;
      inner.system_appearance
    };
    match appearance {
      Some(appearance) => self.set_appearance(appearance),
      None if self.follows_system() => self.set_appearance(system),
      None => {}
    }
  }

  /// Takes the system's settings, if the theme follows them. A shell calls it when they change.
  pub fn apply_system(&self, system: &crate::app::window::SystemSettings) {
    self.inner.write().unwrap().system_appearance = system.appearance;
    if !self.follows_system() {
      return;
    }
    self.set_appearance(self.preferred_appearance().unwrap_or(system.appearance));
    self.set_text_scale(system.text_scale);
    self.set_bold_text(system.bold_text);
  }

  /// The family of interface text: [`ThemeTypography::sans`].
  pub fn sans_family(&self) -> Arc<str> {
    self.inner.read().unwrap().base_typography.sans.clone()
  }

  /// Sets the font of the whole interface: every typography style in the family it was in is in `family`
  /// now. A family name, an alias registered with `App::register_font`, or a CSS generic family.
  pub fn set_sans_family(&self, family: impl Into<Arc<str>>) {
    self.mutate_inner(|inner| {
      inner.base_typography.set_sans(family);
      inner.adjust_typography();
    });
  }

  /// The family of code: [`ThemeTypography::mono_family`].
  pub fn mono_family(&self) -> Arc<str> {
    self.inner.read().unwrap().base_typography.mono_family.clone()
  }

  /// Sets the font of code: every typography style in the family it was in is in `family` now.
  pub fn set_mono_family(&self, family: impl Into<Arc<str>>) {
    self.mutate_inner(|inner| {
      inner.base_typography.set_mono(family);
      inner.adjust_typography();
    });
  }

  pub fn default_text_style(&self) -> TextStyle {
    self.inner.read().unwrap().typography.default_style().clone()
  }

  pub fn set_default_text_style(&self, style: TextStyle) {
    self.mutate_inner(|inner| {
      inner.base_typography.set_default_style(style);
      inner.adjust_typography();
    });
  }

  pub fn typography_style(&self, typography_style: impl Into<TypographyStyle>) -> TextStyle {
    self.inner.read().unwrap().typography.get(typography_style)
  }

  #[cfg(feature = "markdown")]
  pub fn markdown(&self) -> ThemeRef<'_, ThemeMarkdown> {
    ThemeRef {
      inner: self.inner.read().unwrap(),
      value: |inner| &inner.markdown,
    }
  }

  #[cfg(feature = "markdown")]
  pub fn set_markdown(&self, markdown: ThemeMarkdown) {
    self.mutate_inner(|inner| inner.markdown = markdown);
  }

  /// How the marks of styled text inputs look.
  #[cfg(feature = "markdown")]
  pub fn text_marks(&self) -> ThemeRef<'_, ThemeTextMarks> {
    ThemeRef {
      inner: self.inner.read().unwrap_or_else(std::sync::PoisonError::into_inner),
      value: |inner| &inner.text_marks,
    }
  }

  #[cfg(feature = "markdown")]
  pub fn set_text_marks(&self, text_marks: ThemeTextMarks) {
    self.mutate_inner(|inner| inner.text_marks = Arc::new(text_marks));
  }

  /// The shared text marks, for layout without copying them.
  #[cfg(feature = "markdown")]
  pub(crate) fn shared_text_marks(&self) -> Arc<ThemeTextMarks> {
    self
      .inner
      .read()
      .unwrap_or_else(std::sync::PoisonError::into_inner)
      .text_marks
      .clone()
  }

  pub fn fonts(&self) -> ThemeFonts {
    self.inner.read().unwrap().typography.clone().into()
  }

  pub fn set_fonts(&self, fonts: ThemeFonts) {
    self.set_typography(fonts.into());
  }

  #[cfg(feature = "form")]
  pub fn form(&self) -> ThemeRef<'_, FormTheme> {
    ThemeRef {
      inner: self.inner.read().unwrap(),
      value: |inner| &inner.form,
    }
  }

  #[cfg(feature = "form")]
  pub fn set_form(&self, form: FormTheme) {
    self.mutate_inner(|inner| inner.form = form);
  }

  /// Whether motion is to be reduced: what [`Self::set_reduced_motion`] set,
  /// or else what the user asked of every app in the system's accessibility
  /// settings, on the platforms lurq reads it on (Windows, macOS, iOS).
  ///
  /// While it is, a transition of where an element is or how large it is
  /// (its transform, offset, width and height) ends at once; a fade or a
  /// change of color still runs. Keyframe animations are the app's: read
  /// this, or `Ctx::reduced_motion`, to hold back one that moves.
  pub fn reduced_motion(&self) -> bool {
    let inner = self.inner.read().unwrap();
    inner.reduced_motion.unwrap_or(inner.system_reduced_motion)
  }

  /// Decides whether motion is reduced, for the app: `Some(true)` or
  /// `Some(false)`. `None`, the default, follows the system's setting.
  pub fn set_reduced_motion(&self, reduced: Option<bool>) {
    if self.inner.read().unwrap().reduced_motion != reduced {
      self.mutate_inner(|inner| inner.reduced_motion = reduced);
    }
  }

  /// The system's setting, from the shell that read it.
  #[cfg_attr(not(feature = "winit"), allow(dead_code))]
  pub(crate) fn set_system_reduced_motion(&self, reduced: bool) {
    if self.inner.read().unwrap().system_reduced_motion != reduced {
      self.mutate_inner(|inner| inner.system_reduced_motion = reduced);
    }
  }

  pub(crate) fn version(&self) -> u64 {
    self.inner.read().unwrap().version
  }

  /// Whether `other` is this theme or a clone of it: the same values, which
  /// two separate themes at one version need not be.
  pub(crate) fn same_theme(&self, other: &Theme) -> bool {
    Arc::ptr_eq(&self.inner, &other.inner)
  }

  pub(crate) fn track_access(&self) {
    let _ = self.version_signal.get();
  }

  fn mutate_inner(&self, f: impl FnOnce(&mut ThemeInner)) {
    let version = {
      let mut inner = self.inner.write().unwrap();
      f(&mut inner);
      inner.version = inner.version.wrapping_add(1);
      inner.version
    };
    self.version_signal.set(version);
  }
}

pub struct ThemeLens<T> {
  theme: Theme,
  getter: Arc<dyn Fn(&Theme) -> T + Send + Sync>,
  // Reads as the pair of the getter above it.
  #[allow(clippy::type_complexity)]
  setter: Arc<dyn Fn(&Theme, T) + Send + Sync>,
}

impl<T: Clone + Send + Sync + 'static> ThemeLens<T> {
  pub fn get(&self) -> T {
    (self.getter)(&self.theme)
  }

  pub fn set(&self, value: T) {
    (self.setter)(&self.theme, value);
  }

  pub fn update(&self, f: impl FnOnce(&mut T)) {
    let mut value = self.get();
    f(&mut value);
    self.set(value);
  }
}

impl<T> Clone for ThemeLens<T> {
  fn clone(&self) -> Self {
    Self {
      theme: self.theme.clone(),
      getter: self.getter.clone(),
      setter: self.setter.clone(),
    }
  }
}
