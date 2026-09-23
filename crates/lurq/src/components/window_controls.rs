use std::sync::Arc;

use crate::{
  app::{events::MouseEvent, theme::TypographyStyle, window::WindowHandle},
  components::{Row, Text},
  layout::{Alignment, layout_kind::Justify},
  node::{BackgroundColor, CursorIcon, Element, Style, TextColor, color::Color, dimension::Dimension},
};

const WINDOWS_CONTROL_WIDTH: f32 = 46.0;

/// Standard window controls rendered at the trailing edge of a [`ChromeTitleBar`](super::ChromeTitleBar).
///
/// Windows-style controls are styleable per control (content and colors) and share a button size.
/// macOS-style controls draw traffic lights colored by [`TrafficLightColors`].
#[derive(Clone)]
pub struct WindowControls {
  style: WindowControlStyle,
  on_close: Option<Arc<dyn Fn() + Send + Sync>>,
  button_width: f32,
  button_height: Option<f32>,
  minimize: WindowControlButton,
  maximize: WindowControlButton,
  restore: WindowControlButton,
  close: WindowControlButton,
  traffic_lights: TrafficLightColors,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowControlStyle {
  Platform,
  Windows,
  Macos,
  Hidden,
}

/// The action a Windows-style control performs. `Maximize` is shown while the window is restored and
/// `Restore` while it is maximized; both occupy the middle slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WindowControlKind {
  Minimize,
  Maximize,
  Restore,
  Close,
}

/// What a Windows-style control draws in its center.
#[derive(Clone)]
pub enum WindowControlContent {
  /// A text glyph in a theme typography role, e.g. an icon-font code point with a
  /// `TypographyStyle::extra("icon")` role that selects the icon font and size.
  Glyph {
    glyph: Arc<str>,
    typography: TypographyStyle,
  },
  /// App-built content such as an SVG or icon component. The supplier receives the control's
  /// configured foreground color.
  Element(Arc<dyn Fn(TextColor) -> Element + Send + Sync>),
}

/// Colors of one Windows-style control. Hover and active are background colors applied through the
/// node's hovered/active state styles; the foreground stays constant across states.
#[derive(Clone, Debug, PartialEq)]
pub struct WindowControlColors {
  pub foreground: TextColor,
  pub background: BackgroundColor,
  pub hover_background: BackgroundColor,
  pub active_background: BackgroundColor,
}

/// Traffic-light fills for macOS-style controls. Hover keeps the fill; active uses the `*_active` color.
///
/// Defaults are the macOS 11+ (Big Sur and later) values sampled from the system controls:
/// close `#FF5F57`, minimize `#FEBC2E`, zoom `#28C840`. Apple does not publish these values.
#[derive(Clone, Debug, PartialEq)]
pub struct TrafficLightColors {
  pub close: BackgroundColor,
  pub close_active: BackgroundColor,
  pub minimize: BackgroundColor,
  pub minimize_active: BackgroundColor,
  pub zoom: BackgroundColor,
  pub zoom_active: BackgroundColor,
}

#[derive(Clone)]
struct WindowControlButton {
  content: WindowControlContent,
  colors: WindowControlColors,
}

impl Default for WindowControls {
  fn default() -> Self {
    Self::new()
  }
}

impl WindowControls {
  pub fn new() -> Self {
    let default_colors = WindowControlColors::default();
    let button = |glyph: &str, colors: &WindowControlColors| WindowControlButton {
      content: WindowControlContent::glyph(glyph, TypographyStyle::Caption),
      colors: colors.clone(),
    };
    Self {
      style: WindowControlStyle::Platform,
      on_close: None,
      button_width: WINDOWS_CONTROL_WIDTH,
      button_height: None,
      minimize: button("-", &default_colors),
      maximize: button("□", &default_colors),
      restore: button("▢", &default_colors),
      close: button("x", &WindowControlColors::close()),
      traffic_lights: TrafficLightColors::default(),
    }
  }

  pub fn style(mut self, style: WindowControlStyle) -> Self {
    self.style = style;
    self
  }

  pub fn on_close(mut self, f: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_close = Some(Arc::new(f));
    self
  }

  /// Width of each Windows-style control. Defaults to 46.
  pub fn button_width(mut self, width: f32) -> Self {
    self.button_width = width.max(0.0);
    self
  }

  /// Height of each Windows-style control. Defaults to the title-bar height.
  pub fn button_height(mut self, height: f32) -> Self {
    self.button_height = Some(height.max(0.0));
    self
  }

  pub fn button_size(self, width: f32, height: f32) -> Self {
    self.button_width(width).button_height(height)
  }

  /// Replaces what one Windows-style control draws.
  pub fn content(mut self, kind: WindowControlKind, content: WindowControlContent) -> Self {
    self.button_mut(kind).content = content;
    self
  }

  /// Sets the colors of every Windows-style control, including close.
  pub fn colors(mut self, colors: WindowControlColors) -> Self {
    for kind in WindowControlKind::ALL {
      self.button_mut(kind).colors = colors.clone();
    }
    self
  }

  /// Sets the colors of one Windows-style control.
  pub fn control_colors(mut self, kind: WindowControlKind, colors: WindowControlColors) -> Self {
    self.button_mut(kind).colors = colors;
    self
  }

  /// Sets the foreground of every Windows-style control.
  pub fn foreground(mut self, color: impl Into<TextColor>) -> Self {
    let color = color.into();
    self.update_colors(|colors| colors.foreground = color.clone());
    self
  }

  /// Sets the resting background of every Windows-style control.
  pub fn background(mut self, color: impl Into<BackgroundColor>) -> Self {
    let color = color.into();
    self.update_colors(|colors| colors.background = color.clone());
    self
  }

  /// Sets the hover background of every Windows-style control, replacing the red close hover.
  pub fn hover_background(mut self, color: impl Into<BackgroundColor>) -> Self {
    let color = color.into();
    self.update_colors(|colors| colors.hover_background = color.clone());
    self
  }

  /// Sets the pressed background of every Windows-style control, replacing the red close press.
  pub fn active_background(mut self, color: impl Into<BackgroundColor>) -> Self {
    let color = color.into();
    self.update_colors(|colors| colors.active_background = color.clone());
    self
  }

  pub fn traffic_lights(mut self, colors: TrafficLightColors) -> Self {
    self.traffic_lights = colors;
    self
  }

  fn button_mut(&mut self, kind: WindowControlKind) -> &mut WindowControlButton {
    match kind {
      WindowControlKind::Minimize => &mut self.minimize,
      WindowControlKind::Maximize => &mut self.maximize,
      WindowControlKind::Restore => &mut self.restore,
      WindowControlKind::Close => &mut self.close,
    }
  }

  fn update_colors(&mut self, mut update: impl FnMut(&mut WindowControlColors)) {
    for kind in WindowControlKind::ALL {
      update(&mut self.button_mut(kind).colors);
    }
  }

  pub(crate) fn render(self, window: &WindowHandle, height: f32) -> Element {
    match self.resolved_style() {
      WindowControlStyle::Platform | WindowControlStyle::Windows => self.render_windows(window, height),
      WindowControlStyle::Macos => self.render_macos(window, height),
      WindowControlStyle::Hidden => Row::new().height(height).into(),
    }
  }

  fn resolved_style(&self) -> WindowControlStyle {
    match self.style {
      WindowControlStyle::Platform if cfg!(target_os = "macos") => WindowControlStyle::Macos,
      WindowControlStyle::Platform => WindowControlStyle::Windows,
      style => style,
    }
  }

  fn render_windows(self, window: &WindowHandle, height: f32) -> Element {
    let minimize_window = window.clone();
    let maximize_window = window.clone();
    let close_window = window.clone();
    let on_close = self.on_close.clone();
    let maximized = window.is_maximized;
    let width = self.button_width;
    let button_height = self.button_height.unwrap_or(height);
    let middle = if maximized { self.restore } else { self.maximize };

    Row::new()
      .height(height)
      .align_items(Alignment::Center)
      .child(
        control_button(self.minimize, width, button_height).on_click(move |event: MouseEvent| {
          minimize_window.set_minimized(true);
          event.prevent_default();
          event.stop_immediate_propagation();
        }),
      )
      .child(
        control_button(middle, width, button_height).on_click(move |event: MouseEvent| {
          maximize_window.set_maximized(!maximized);
          event.prevent_default();
          event.stop_immediate_propagation();
        }),
      )
      .child(
        control_button(self.close, width, button_height).on_click(move |event: MouseEvent| {
          if let Some(on_close) = &on_close {
            on_close();
          }
          close_window.close();
          event.prevent_default();
          event.stop_immediate_propagation();
        }),
      )
      .into()
  }

  fn render_macos(self, window: &WindowHandle, height: f32) -> Element {
    let close_window = window.clone();
    let minimize_window = window.clone();
    let maximize_window = window.clone();
    let on_close = self.on_close.clone();
    let maximized = window.is_maximized;
    let lights = self.traffic_lights;

    Row::new()
      .height(height)
      .align_items(Alignment::Center)
      .spacing(0.0)
      .padding_left(8.0)
      .child(
        macos_control_button(lights.close, lights.close_active).on_click(move |event: MouseEvent| {
          if let Some(on_close) = &on_close {
            on_close();
          }
          close_window.close();
          event.prevent_default();
          event.stop_immediate_propagation();
        }),
      )
      .child(
        macos_control_button(lights.minimize, lights.minimize_active).on_click(move |event: MouseEvent| {
          minimize_window.set_minimized(true);
          event.prevent_default();
          event.stop_immediate_propagation();
        }),
      )
      .child(
        macos_control_button(lights.zoom, lights.zoom_active).on_click(move |event: MouseEvent| {
          maximize_window.set_maximized(!maximized);
          event.prevent_default();
          event.stop_immediate_propagation();
        }),
      )
      .into()
  }
}

impl WindowControlKind {
  pub const ALL: [Self; 4] = [Self::Minimize, Self::Maximize, Self::Restore, Self::Close];
}

impl WindowControlContent {
  pub fn glyph(glyph: impl Into<Arc<str>>, typography: impl Into<TypographyStyle>) -> Self {
    Self::Glyph {
      glyph: glyph.into(),
      typography: typography.into(),
    }
  }

  pub fn element(supplier: impl Fn(TextColor) -> Element + Send + Sync + 'static) -> Self {
    Self::Element(Arc::new(supplier))
  }

  fn render(&self, foreground: TextColor) -> Element {
    match self {
      Self::Glyph { glyph, typography } => Text::new(glyph).variant(*typography).color(foreground).into(),
      Self::Element(supplier) => supplier(foreground),
    }
  }
}

impl Default for WindowControlColors {
  /// The minimize and maximize/restore defaults.
  fn default() -> Self {
    Self {
      foreground: TextColor::Color(Color::from_hex("#d9dee7")),
      background: BackgroundColor::Color(Color::new(0, 0, 0, 0)),
      hover_background: BackgroundColor::Color(Color::from_hex("#232934")),
      active_background: BackgroundColor::Color(Color::from_hex("#2d3440")),
    }
  }
}

impl WindowControlColors {
  pub fn new(
    foreground: impl Into<TextColor>,
    hover_background: impl Into<BackgroundColor>,
    active_background: impl Into<BackgroundColor>,
  ) -> Self {
    Self {
      foreground: foreground.into(),
      hover_background: hover_background.into(),
      active_background: active_background.into(),
      ..Self::default()
    }
  }

  /// The close-control default: red hover and press backgrounds.
  pub fn close() -> Self {
    Self {
      hover_background: BackgroundColor::Color(Color::from_hex("#c0392b")),
      active_background: BackgroundColor::Color(Color::from_hex("#922b21")),
      ..Self::default()
    }
  }

  pub fn background(mut self, color: impl Into<BackgroundColor>) -> Self {
    self.background = color.into();
    self
  }
}

impl Default for TrafficLightColors {
  fn default() -> Self {
    Self {
      close: Color::from_hex("#ff5f57").into(),
      close_active: Color::from_hex("#e2463f").into(),
      minimize: Color::from_hex("#febc2e").into(),
      minimize_active: Color::from_hex("#e0a11b").into(),
      zoom: Color::from_hex("#28c840").into(),
      zoom_active: Color::from_hex("#1ead34").into(),
    }
  }
}

fn control_button(button: WindowControlButton, width: f32, height: f32) -> Row {
  let colors = button.colors;
  Row::new()
    .width(width)
    .height(height)
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .background(colors.background)
    .cursor(CursorIcon::Pointer)
    .hovered_style(Style::new().background(colors.hover_background))
    .active_style(Style::new().background(colors.active_background))
    .on_mouse_down(|event: MouseEvent| {
      event.prevent_default();
      event.stop_immediate_propagation();
    })
    .child(button.content.render(colors.foreground))
}

fn macos_control_button(color: BackgroundColor, active_color: BackgroundColor) -> Row {
  Row::new()
    .width(20.0)
    .height(Dimension::Pct(100.0))
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .cursor(CursorIcon::Pointer)
    .on_mouse_down(|event: MouseEvent| {
      event.prevent_default();
      event.stop_immediate_propagation();
    })
    .child(
      Row::new()
        .width(12.0)
        .height(12.0)
        .rounded(6.0)
        .background(color.clone())
        .hovered_style(Style::new().background(color))
        .active_style(Style::new().background(active_color)),
    )
}
