// Headless WindowChrome coverage: styled Windows controls in the render list,
// hover/active state colors, full-size content under overlay resize handles,
// edge/corner resize hit zones resolved through real pointer hit testing, and
// the absence of lurq-painted outlines when the chrome has no border.

use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx, events::MouseButton, theme::PaletteColor},
  components::{
    ChromeBorderPolicy, ChromeTitleBar, Column, Rect, ResizeHandlePlacement, WindowChrome, WindowChromeMode,
    WindowControlContent, WindowControlKind, WindowControlStyle, WindowControls,
  },
  node::{CursorIcon, Element, TextColor, color::Color, dimension::Dimension},
};

use crate::support::{RenderSnapshot, render_pass_with_app};

const WIDTH: f32 = 1440.0;
const HEIGHT: f32 = 900.0;
const TITLE_HEIGHT: f32 = 36.0;
const BUTTON_WIDTH: f32 = 46.0;
const BUTTON_HEIGHT: f32 = 32.0;
const ICON_SIZE: f32 = 14.0;

fn foreground() -> Color {
  Color::from_hex("#aab4c3")
}

fn resting() -> Color {
  Color::from_hex("#141821")
}

fn hover() -> Color {
  Color::from_hex("#2a3140")
}

fn active() -> Color {
  Color::from_hex("#394255")
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Controls {
  Default,
  Styled,
}

#[derive(Clone)]
struct ChromeProps {
  controls: Controls,
  placement: ResizeHandlePlacement,
  // `ChromeBorderPolicy::Hidden` and no title-bar bottom border.
  borderless: bool,
  content_presses: Arc<AtomicUsize>,
}

impl PartialEq for ChromeProps {
  fn eq(&self, other: &Self) -> bool {
    self.controls == other.controls
      && self.placement == other.placement
      && self.borderless == other.borderless
      && Arc::ptr_eq(&self.content_presses, &other.content_presses)
  }
}

impl lurq::app::component::DevtoolsInspectable for ChromeProps {
  fn write_info(&self, _buffer: &mut Vec<lurq::app::component::ComponentInfo>) {}
}

struct ChromeHost;

impl Component for ChromeHost {
  type Props = ChromeProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<ChromeProps>().clone();
    let presses = props.content_presses.clone();
    let controls = match props.controls {
      Controls::Default => WindowControls::new().style(WindowControlStyle::Windows),
      Controls::Styled => styled_controls(),
    };
    let content = Column::new()
      .id("content")
      .width(Dimension::Pct(100.0))
      .height(Dimension::Pct(100.0))
      .on_mouse_down(move |_| {
        presses.fetch_add(1, Ordering::SeqCst);
      });

    let title_bar = ChromeTitleBar::new().height(TITLE_HEIGHT).controls(controls);
    let (border, title_bar) = if props.borderless {
      (ChromeBorderPolicy::Hidden, title_bar.border_bottom(None))
    } else {
      (ChromeBorderPolicy::PlatformDefault, title_bar)
    };

    WindowChrome::new()
      .mode(WindowChromeMode::AlwaysCustom)
      .resize_placement(props.placement)
      .border(border)
      .title_bar(title_bar)
      .content(content)
      .mount(ctx)
  }
}

// An icon-font-like control: a 14px element tinted with the supplied foreground,
// with one shared hover/active background (no red close hover).
fn styled_controls() -> WindowControls {
  let icon = || {
    WindowControlContent::element(|foreground: TextColor| {
      let color = foreground.as_color().expect("concrete foreground");
      Rect::new(ICON_SIZE, ICON_SIZE).background(color).into()
    })
  };
  let mut controls = WindowControls::new()
    .style(WindowControlStyle::Windows)
    .button_size(BUTTON_WIDTH, BUTTON_HEIGHT)
    .foreground(foreground())
    .background(resting())
    .hover_background(PaletteColor::extra("chrome_control_hover"))
    .active_background(active());
  for kind in WindowControlKind::ALL {
    controls = controls.content(kind, icon());
  }
  controls
}

fn mount(controls: Controls, placement: ResizeHandlePlacement) -> (App, Tree, Arc<AtomicUsize>) {
  mount_with(controls, placement, false)
}

fn mount_with(controls: Controls, placement: ResizeHandlePlacement, borderless: bool) -> (App, Tree, Arc<AtomicUsize>) {
  let mut app = App::new();
  app
    .theme()
    .set_palette_color(PaletteColor::extra("chrome_control_hover"), hover());
  let presses = Arc::new(AtomicUsize::new(0));
  let mut tree = Tree::new();
  tree.resize(WIDTH as u32, HEIGHT as u32);
  tree.mount_root::<ChromeHost>(
    &mut app,
    ChromeProps {
      controls,
      placement,
      borderless,
      content_presses: presses.clone(),
    },
  );
  render_pass_with_app(&mut tree, &mut app);
  (app, tree, presses)
}

// Buttons sit flush against the right window edge: minimize, maximize, close.
fn button_x(slot: usize) -> f32 {
  WIDTH - BUTTON_WIDTH * (3 - slot) as f32
}

fn button_center(slot: usize) -> (f32, f32) {
  (button_x(slot) + BUTTON_WIDTH / 2.0, TITLE_HEIGHT / 2.0)
}

fn button_color(snapshot: &RenderSnapshot, slot: usize) -> Option<Color> {
  snapshot
    .rects
    .iter()
    .filter(|rect| {
      (rect.x - button_x(slot)).abs() < 0.5
        && (rect.width - BUTTON_WIDTH).abs() < 0.5
        && (rect.height - BUTTON_HEIGHT).abs() < 0.5
    })
    .map(|rect| rect.color)
    .next_back()
}

#[test]
fn styled_controls_render_configured_size_content_and_colors() {
  let (mut app, mut tree, _) = mount(Controls::Styled, ResizeHandlePlacement::Overlay);
  let snapshot = render_pass_with_app(&mut tree, &mut app);

  for slot in 0..3 {
    let button = snapshot
      .rects
      .iter()
      .find(|rect| (rect.x - button_x(slot)).abs() < 0.5 && (rect.width - BUTTON_WIDTH).abs() < 0.5)
      .expect("control button background rendered");
    assert_eq!(button.height, BUTTON_HEIGHT);
    assert_eq!(button.y, (TITLE_HEIGHT - BUTTON_HEIGHT) / 2.0);
    assert_eq!(button.color, resting());

    let icon = snapshot
      .rects
      .iter()
      .find(|rect| rect.width == ICON_SIZE && rect.x > button.x && rect.x < button.x + BUTTON_WIDTH)
      .expect("icon element rendered inside the button");
    assert_eq!(icon.color, foreground());
    assert_eq!(icon.x, button.x + (BUTTON_WIDTH - ICON_SIZE) / 2.0);
    assert_eq!(icon.y, (TITLE_HEIGHT - ICON_SIZE) / 2.0);
  }
  assert_eq!(snapshot.glyph_count, 0, "element content replaces the default glyphs");
}

#[test]
fn styled_controls_share_hover_and_active_backgrounds() {
  let (mut app, mut tree, _) = mount(Controls::Styled, ResizeHandlePlacement::Overlay);

  for slot in 0..3 {
    let (x, y) = button_center(slot);
    tree.mouse_move(x, y);
    let hovered = render_pass_with_app(&mut tree, &mut app);
    assert_eq!(
      button_color(&hovered, slot),
      Some(hover()),
      "palette hover on slot {slot}"
    );
    for other in (0..3).filter(|other| *other != slot) {
      assert_eq!(button_color(&hovered, other), Some(resting()));
    }
  }

  // Hold the minimize button: pressing close would close the test window.
  let (x, y) = button_center(0);
  tree.mouse_move(x, y);
  tree.mouse_down(x, y, MouseButton::Left);
  let pressed = render_pass_with_app(&mut tree, &mut app);
  assert_eq!(button_color(&pressed, 0), Some(active()));
}

#[test]
fn default_controls_keep_the_red_close_hover() {
  let (mut app, mut tree, _) = mount(Controls::Default, ResizeHandlePlacement::Overlay);
  let default_hover = Color::from_hex("#232934");
  let close_hover = Color::from_hex("#c0392b");
  let default_height = TITLE_HEIGHT;
  let color_at = |snapshot: &RenderSnapshot, slot: usize| {
    snapshot
      .rects
      .iter()
      .filter(|rect| (rect.x - button_x(slot)).abs() < 0.5 && rect.height == default_height)
      .map(|rect| rect.color)
      .next_back()
  };

  let (x, y) = button_center(0);
  tree.mouse_move(x, y);
  assert_eq!(
    color_at(&render_pass_with_app(&mut tree, &mut app), 0),
    Some(default_hover)
  );

  let (x, y) = button_center(2);
  tree.mouse_move(x, y);
  let snapshot = render_pass_with_app(&mut tree, &mut app);
  assert_eq!(color_at(&snapshot, 2), Some(close_hover));
  assert!(snapshot.glyph_count > 0, "default controls draw text glyphs");
}

#[test]
fn overlay_resize_handles_keep_full_content_size() {
  let (_app, mut tree, _) = mount(Controls::Default, ResizeHandlePlacement::Overlay);
  let bounds = tree.get_element_by_id_mut("content").unwrap().bounds().unwrap();

  assert_eq!((bounds.x, bounds.y), (0.0, TITLE_HEIGHT));
  assert_eq!((bounds.width, bounds.height), (WIDTH, HEIGHT - TITLE_HEIGHT));
}

#[test]
fn inset_resize_handles_reserve_a_gutter() {
  let (_app, mut tree, _) = mount(Controls::Default, ResizeHandlePlacement::Inset);
  let bounds = tree.get_element_by_id_mut("content").unwrap().bounds().unwrap();

  assert_eq!((bounds.x, bounds.y), (3.0, TITLE_HEIGHT));
  assert_eq!(
    (bounds.width, bounds.height),
    (WIDTH - 6.0, HEIGHT - TITLE_HEIGHT - 3.0)
  );
}

#[test]
fn overlay_resize_zones_resolve_at_edges_and_corners() {
  let (mut app, mut tree, presses) = mount(Controls::Default, ResizeHandlePlacement::Overlay);
  let right = WIDTH - 1.0;
  let bottom = HEIGHT - 1.0;
  let cases = [
    ((1.0, 450.0), CursorIcon::WResize),
    ((right, 450.0), CursorIcon::EResize),
    ((720.0, 1.0), CursorIcon::NResize),
    ((720.0, bottom), CursorIcon::SResize),
    ((1.0, 1.0), CursorIcon::NwResize),
    ((right, 1.0), CursorIcon::NeResize),
    ((1.0, bottom), CursorIcon::SwResize),
    ((right, bottom), CursorIcon::SeResize),
  ];

  for ((x, y), cursor) in cases {
    tree.mouse_move(x, y);
    render_pass_with_app(&mut tree, &mut app);
    assert_eq!(tree.cursor(), cursor, "cursor at ({x}, {y})");
    tree.mouse_down(x, y, MouseButton::Left);
    tree.mouse_up(x, y, MouseButton::Left);
  }
  assert_eq!(presses.load(Ordering::SeqCst), 0, "resize zones capture edge presses");

  // Just inside the handle strip the content receives the press.
  tree.mouse_move(10.0, 450.0);
  tree.mouse_down(10.0, 450.0, MouseButton::Left);
  tree.mouse_up(10.0, 450.0, MouseButton::Left);
  assert_eq!(presses.load(Ordering::SeqCst), 1);
}

// Filled rects no thicker than a border line; lurq paints borders this way.
fn thin_lines(snapshot: &RenderSnapshot) -> Vec<(f32, f32, f32, f32, Color)> {
  snapshot
    .rects
    .iter()
    .filter(|rect| rect.color.a() > 0 && (rect.width <= 1.5 || rect.height <= 1.5))
    .map(|rect| (rect.x, rect.y, rect.width, rect.height, rect.color))
    .collect()
}

#[test]
fn borderless_chrome_paints_no_outline() {
  for placement in [ResizeHandlePlacement::Overlay, ResizeHandlePlacement::Inset] {
    let (mut app, mut tree, _) = mount_with(Controls::Default, placement, true);
    let snapshot = render_pass_with_app(&mut tree, &mut app);

    assert_eq!(thin_lines(&snapshot), vec![], "{placement:?}");
    assert!(
      snapshot.rects.iter().all(|rect| rect.stroke == [0.0; 4]),
      "{placement:?}"
    );
  }
}

#[test]
fn default_title_bar_paints_its_bottom_border() {
  let (mut app, mut tree, _) = mount(Controls::Default, ResizeHandlePlacement::Overlay);
  let snapshot = render_pass_with_app(&mut tree, &mut app);
  let title_border = (0.0, TITLE_HEIGHT - 1.0, WIDTH, 1.0, Color::from_hex("#252a32"));

  assert!(thin_lines(&snapshot).contains(&title_border));
}
