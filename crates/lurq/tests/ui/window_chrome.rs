//! A window's chrome: the board's bar on Windows, macOS, GNOME and KDE, its controls in each platform's order and
//! look, a bar with tabs and its line under them, a window that is not in front or is maximized, a title too long for
//! its room, what tooling reads, and a theme's renderer.

use std::sync::Arc;

use lurq::{
  app::{
    App,
    theme::{PaletteColor, ThemePalette},
  },
  components::{Column, Rect},
  layout::{Constraints, Size, quad::QuadContent},
  node::{Element, color::Color},
  ui::{
    Interaction, WindowChrome, WindowChromePart, WindowChromeRenderer, WindowChromeState, WindowChromeView,
    WindowPlatform, icons,
  },
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// A stage of 640 by 300 with `chrome` 560 wide in it, as the board draws it.
fn stage(chrome: impl Fn() -> WindowChrome + Send + Sync + 'static) -> Stage {
  stage_in(App::new(), chrome)
}

fn stage_in(app: App, chrome: impl Fn() -> WindowChrome + Send + Sync + 'static) -> Stage {
  let mut stage = Stage::in_app(app, move |_| -> Element {
    Column::new().width(560.0).child(chrome().id("bar")).into()
  });
  stage.tree.resize(640, 300);
  stage
    .tree
    .set_layout_constraints_override(Some(Constraints::loose(Size::new(640.0, 300.0))));
  stage.pass();
  stage
}

fn on(platform: WindowPlatform) -> WindowChrome {
  WindowChrome::new().platform(platform).title("lurq demo")
}

/// The text quad of `content`, as painted.
fn text_at(stage: &Stage, content: &str) -> (f32, f32) {
  let quad = stage
    .text_quad(content)
    .unwrap_or_else(|| panic!("{content:?} is painted"));
  (quad.x, quad.width)
}

#[test]
fn on_windows_the_icon_and_the_title_are_at_the_start_and_three_square_controls_at_the_end() {
  let mut stage = stage(|| on(WindowPlatform::Windows).content(Rect::new(560.0, 100.0)));
  let (bx, by, bw, bh) = stage.bounds("bar");
  assert_eq!((bx, by, bw, bh), (0.0, 0.0, 560.0, 36.0));
  assert_eq!(stage.fill_color("bar"), Some(light(PaletteColor::Bar)));
  // Minimize, maximize and close, 46 by 36, flush with the end and with one another.
  let controls = ["bar-minimize", "bar-maximize", "bar-close"].map(|id| stage.bounds(id));
  for (index, (x, y, width, height)) in controls.into_iter().enumerate() {
    assert_eq!(
      (x, y, width, height),
      (560.0 - 46.0 * (3 - index) as f32, 0.0, 46.0, 36.0)
    );
  }
  // The title after the icon, 12 + 16 + 8 from the start.
  let (title_x, _) = text_at(&stage, "lurq demo");
  assert!((title_x - 36.0).abs() < 1.0, "{title_x}");
  let (color, size) = stage.text("lurq demo");
  assert_eq!((color, size), (light(PaletteColor::Foreground), 13.0));
}

#[test]
fn on_macos_the_lights_are_at_the_start_and_the_title_in_the_middle() {
  let mut stage = stage(|| on(WindowPlatform::Macos));
  assert_eq!(stage.bounds("bar").3, 36.0);
  // Close, minimize and zoom, 12 across and 8 apart, 12 in from the start, in the system's colors.
  let lights = ["bar-close", "bar-minimize", "bar-maximize"].map(|id| stage.bounds(id));
  for (index, (x, y, width, height)) in lights.into_iter().enumerate() {
    assert_eq!((x, y, width, height), (12.0 + 20.0 * index as f32, 12.0, 12.0, 12.0));
  }
  assert_eq!(stage.fill_color("bar-close"), Some(Color::from_hex("#FF5F57")));
  assert_eq!(stage.fill_color("bar-minimize"), Some(Color::from_hex("#FEBC2E")));
  assert_eq!(stage.fill_color("bar-maximize"), Some(Color::from_hex("#28C840")));
  // The title is in the middle of the bar.
  let (x, width) = text_at(&stage, "lurq demo");
  assert!((x + width / 2.0 - 280.0).abs() < 1.0, "{x} {width}");
}

#[test]
fn on_gnome_the_header_bar_is_44_high_with_round_filled_controls_at_the_end() {
  let mut stage = stage(|| on(WindowPlatform::Gnome));
  assert_eq!(stage.bounds("bar").3, 44.0);
  let controls = ["bar-minimize", "bar-maximize", "bar-close"].map(|id| stage.bounds(id));
  for (index, (x, y, width, height)) in controls.into_iter().enumerate() {
    assert_eq!(
      (x, y, width, height),
      (560.0 - 10.0 - 88.0 + 32.0 * index as f32, 10.0, 24.0, 24.0)
    );
  }
  assert_eq!(
    stage.fill_color("bar-minimize"),
    Some(light(PaletteColor::SecondaryHover))
  );
  let (x, width) = text_at(&stage, "lurq demo");
  assert!((x + width / 2.0 - 280.0).abs() < 1.0, "{x} {width}");
}

#[test]
fn on_kde_the_icon_is_at_the_start_the_title_in_the_middle_and_round_controls_at_the_end() {
  let mut stage = stage(|| on(WindowPlatform::Kde));
  assert_eq!(stage.bounds("bar").3, 32.0);
  let controls = ["bar-minimize", "bar-maximize", "bar-close"].map(|id| stage.bounds(id));
  for (index, (x, y, width, height)) in controls.into_iter().enumerate() {
    assert_eq!(
      (x, y, width, height),
      (560.0 - 6.0 - 78.0 + 28.0 * index as f32, 5.0, 22.0, 22.0)
    );
  }
  assert_eq!(stage.fill_color("bar-minimize"), None, "nothing under them at rest");
  // In the middle of the room between the bar's paddings, 8 at the start and 6 at the end.
  let (x, width) = text_at(&stage, "lurq demo");
  assert!((x + width / 2.0 - 281.0).abs() < 1.0, "{x} {width}");
}

#[test]
fn the_close_control_is_red_with_a_white_cross_under_the_pointer() {
  for platform in [WindowPlatform::Windows, WindowPlatform::Gnome, WindowPlatform::Kde] {
    let mut stage = stage(move || on(platform).force(Some(Interaction::Hovered)));
    assert_eq!(
      stage.fill_color("bar-close"),
      Some(light(PaletteColor::Destructive)),
      "{platform:?}"
    );
    let cross = icons::ALL
      .iter()
      .find_map(|(name, glyph)| (*name == icons::CLOSE).then(|| glyph.to_string()))
      .expect("the font has a cross");
    assert_eq!(
      stage.text(&cross).0,
      light(PaletteColor::DestructiveForeground),
      "{platform:?}: the cross is white"
    );
  }
}

#[test]
fn a_window_that_is_not_in_front_has_grey_lights_on_macos_and_a_muted_title() {
  let mut stage = stage(|| on(WindowPlatform::Macos).state(WindowChromeState::Inactive, true));
  for id in ["bar-close", "bar-minimize", "bar-maximize"] {
    assert_eq!(stage.fill_color(id), Some(light(PaletteColor::Input)), "{id}");
  }
  assert_eq!(stage.text("lurq demo").0, light(PaletteColor::MutedForeground));
}

#[test]
fn a_bar_with_tabs_is_higher_has_its_title_before_the_tabs_and_its_line_under_them() {
  for (platform, height) in [
    (WindowPlatform::Windows, 40.0),
    (WindowPlatform::Macos, 40.0),
    (WindowPlatform::Gnome, 44.0),
    (WindowPlatform::Kde, 38.0),
  ] {
    let tab = light(PaletteColor::Background);
    let mut stage = stage(move || {
      on(platform)
        .tabs(Rect::new(200.0, 32.0).background(tab).id("tabs"))
        .content(Rect::new(560.0, 100.0))
    });
    assert_eq!(stage.bounds("bar").3, height, "{platform:?}");
    // The title stands before the tabs, on one line, centred on the bar's height.
    let title = stage
      .text_quad("lurq demo")
      .unwrap_or_else(|| panic!("{platform:?}: the title"));
    let (tabs_x, y, _, tab_height) = stage.bounds("tabs");
    assert!(
      title.x + title.width <= tabs_x,
      "{platform:?}: the title before the tabs"
    );
    assert!(
      ((title.y + title.height / 2.0) - height / 2.0).abs() <= 2.0,
      "{platform:?}: centred on the bar"
    );
    // The tabs stand on the bar's bottom edge.
    assert_eq!(y + tab_height, height, "{platform:?}");
    // The bar's line is under the tabs, along its bottom edge, and drawn before them: the selected tab covers it.
    let border = light(PaletteColor::Border);
    let quads = stage.quads();
    let lines: Vec<usize> = quads
      .iter()
      .enumerate()
      .filter(|(_, quad)| quad.height <= 1.0 && quad.width >= 500.0 && (quad.y - (height - 1.0)).abs() < 0.01)
      .filter(|(_, quad)| matches!(&quad.content, QuadContent::Rect { color, .. } if *color == border))
      .map(|(at, _)| at)
      .collect();
    assert_eq!(lines.len(), 1, "{platform:?}: one line under the tabs");
    let drawn = quads
      .iter()
      .position(|quad| {
        (quad.x, quad.width, quad.height) == (tabs_x, 200.0, 32.0)
          && matches!(&quad.content, QuadContent::Rect { color, .. } if *color == tab)
      })
      .unwrap_or_else(|| panic!("{platform:?}: the tabs' fill"));
    assert!(lines[0] < drawn, "{platform:?}: the line is drawn before the tabs");
  }
  // Windows keeps its controls at the top, 36 high.
  let mut stage = stage(|| on(WindowPlatform::Windows).tabs(Rect::new(200.0, 32.0)));
  assert_eq!(stage.bounds("bar-close"), (514.0, 0.0, 46.0, 36.0));
}

#[test]
fn a_title_longer_than_its_room_ends_with_an_ellipsis_and_the_controls_keep_theirs() {
  const LONG: &str = "A title far longer than the room the bar leaves for it between its icon and the controls";
  let mut stage = stage(|| WindowChrome::new().platform(WindowPlatform::Windows).title(LONG));
  let title = stage
    .quads()
    .into_iter()
    .find_map(|quad| match &quad.content {
      QuadContent::Text { text, .. } if text.starts_with("A title") => Some((text.to_string(), quad.x + quad.width)),
      _ => None,
    })
    .expect("the title");
  assert!(title.0.ends_with('…'), "{:?}", title.0);
  assert!(
    title.1 <= 560.0 - 138.0 + 0.5,
    "it ends before the controls: {}",
    title.1
  );
  assert_eq!(stage.bounds("bar-close").2, 46.0);
}

#[test]
fn what_the_app_leaves_out_is_not_there() {
  let stage = stage(|| {
    WindowChrome::new()
      .platform(WindowPlatform::Windows)
      .icon(None)
      .controls(lurq::ui::WindowControls::new().maximizable(false).minimizable(false))
  });
  let mut stage = stage;
  assert!(stage.tree.get_element_by_id_mut("bar-minimize").is_none());
  assert!(stage.tree.get_element_by_id_mut("bar-maximize").is_none());
  assert_eq!(stage.bounds("bar-close").0, 514.0);
}

#[test]
fn the_app_puts_its_own_controls_at_the_start_and_the_end() {
  let mut stage = stage(|| {
    on(WindowPlatform::Windows)
      .leading(Rect::new(120.0, 24.0).id("menus"))
      .trailing(Rect::new(80.0, 24.0).id("search"))
  });
  // The menus after the icon, before the title; the search before the controls, 8 from them.
  let (menus_x, ..) = stage.bounds("menus");
  assert!((menus_x - 36.0).abs() < 0.5, "{menus_x}");
  let (title_x, _) = text_at(&stage, "lurq demo");
  assert!(title_x >= menus_x + 120.0 + 8.0 - 0.5, "{title_x}");
  let (search_x, _, search_width, _) = stage.bounds("search");
  assert!(
    (search_x + search_width - (560.0 - 138.0 - 8.0)).abs() < 0.5,
    "{search_x}"
  );
}

#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn tooling_reads_a_named_bar_and_controls_named_by_their_labels() {
  let mut stage = stage(|| {
    on(WindowPlatform::Windows)
      .state(WindowChromeState::Maximized, true)
      .controls(lurq::ui::WindowControls::new().close_label("Quit"))
  });
  let mut said = |id: &str, name: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(name))
  };
  assert_eq!(said("bar", "a11y_name").as_deref(), Some("lurq demo"));
  assert_eq!(said("bar-minimize", "a11y_name").as_deref(), Some("Minimize"));
  assert_eq!(
    said("bar-maximize", "a11y_name").as_deref(),
    Some("Restore"),
    "maximized, it restores"
  );
  assert_eq!(said("bar-close", "a11y_name").as_deref(), Some("Quit"));
  assert_eq!(said("bar-close", "a11y_role").as_deref(), Some("button"));
}

#[test]
fn a_rule_restyles_a_part_and_a_theme_draws_another_bar() {
  let mut stage = stage(|| on(WindowPlatform::Windows).style(WindowChromePart::Bar, |s| s.height(48.0)));
  assert_eq!(stage.bounds("bar").3, 48.0);

  let app = App::new();
  let renderer: WindowChromeRenderer = Arc::new(|view: WindowChromeView| {
    // Every platform drawn as KDE draws it.
    view.kde()
  });
  app.theme().set_renderer::<WindowChrome>(renderer);
  let mut stage = stage_in(app, || on(WindowPlatform::Windows));
  assert_eq!(stage.bounds("bar-close").2, 46.0, "the Windows recipe still sizes it");
  let (minimize, ..) = stage.bounds("bar-minimize");
  let (close, ..) = stage.bounds("bar-close");
  assert!(minimize < close);
}
