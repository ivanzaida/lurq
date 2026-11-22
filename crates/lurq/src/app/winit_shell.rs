use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawWindowHandle};
#[cfg(target_os = "macos")]
use winit::platform::macos::EventLoopBuilderExtMacOS;
#[cfg(windows)]
use winit::platform::windows::{
  Color as WinitWindowsColor, CornerPreference as WinitCornerPreference, WindowAttributesExtWindows, WindowExtWindows,
};
use winit::{
  application::ApplicationHandler,
  dpi::{PhysicalPosition, Position},
  event::{ElementState, MouseScrollDelta, TouchPhase, WindowEvent},
  event_loop::{ActiveEventLoop, EventLoop},
  keyboard::{Key, KeyCode, ModifiersState, NamedKey, PhysicalKey},
  window::{
    CursorIcon as WinitCursorIcon, Fullscreen, Icon as WinitIcon, ResizeDirection as WinitResizeDirection, Window,
    WindowAttributes, WindowId,
  },
};

use crate::{
  app::{
    App, Tree,
    events::{MouseButton, ScrollPhase},
    runtime::{PassReport, SecondaryWindow, SecondaryWindowMetadata, finger},
    window::{
      CloseRequestSource, WindowBorderColor, WindowCommand, WindowCornerRadius, WindowIcon, WindowResizeDirection,
      resize::SizeReporter,
    },
  },
  node::{CursorIcon, color::Color},
  time::{Duration, Instant},
};

mod cursor;
mod file_drag;
mod frame_pacing;
#[cfg(test)]
mod frame_pacing_tests;
mod frame_source;
mod history_swipes;
mod ime;
#[cfg(target_os = "ios")]
mod ios;
#[cfg(target_os = "ios")]
mod ios_text;
mod loop_wait;
#[cfg(test)]
mod loop_wait_tests;
#[cfg(target_os = "macos")]
mod macos_controls;
mod native_resize;
mod notched_wheel;
#[cfg(test)]
mod touch_delivery_tests;
mod trackpad_gestures;
#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(target_arch = "wasm32")]
mod web_drop;
#[cfg(target_arch = "wasm32")]
mod web_text;
#[cfg(any(target_arch = "wasm32", test))]
mod web_wheel;

use file_drag::FileDragShell;
use frame_pacing::PresentedFrame;
use frame_source::FrameSource;
use loop_wait::{LoopNeeds, LoopWait};

const CONTINUOUS_REDRAW_GRACE: Duration = Duration::from_millis(500);
const WINIT_SLOW_SCOPE_THRESHOLD: Duration = Duration::from_millis(45);
const WINIT_BREAKDOWN_THRESHOLD: Duration = Duration::from_millis(45);
/// Floor between direct presents from the mouse-move stream (interactive
/// drags and hover restyles; vsync usually paces harder — this guards the
/// vsync-off case).
const INTERACTION_PRESENT_INTERVAL: Duration = Duration::from_millis(7);
/// The pixels a wheel scrolls for one line, which is what winit tells a notch as on a desktop.
const WHEEL_LINE: f32 = 40.0;

type TickFn = Box<dyn FnMut(&mut Tree, Duration)>;
type PaintFn = Box<dyn FnMut(&Tree, Duration, PassReport)>;
type PositionChangedFn = Box<dyn FnMut(i32, i32)>;
type SizeChangedFn = Box<dyn FnMut(u32, u32)>;

pub struct WinitWindow {
  app: App,
  // A tree is some 11 KB: boxed, the window's builders and the loop hand on a pointer, where a build without
  // optimizations kept a copy of the whole on the stack at each step.
  tree: Box<Tree>,
  attrs: WindowAttributes,
  start_without_focus: bool,
  corner_radius: Option<WindowCornerRadius>,
  on_tick: Option<TickFn>,
  on_paint: Option<PaintFn>,
  on_position_changed: Option<PositionChangedFn>,
  on_size_changed: Option<SizeChangedFn>,
}

impl WinitWindow {
  pub fn new(app: App, tree: Tree) -> Self {
    Self {
      app,
      tree: Box::new(tree),
      attrs: WindowAttributes::default(),
      start_without_focus: false,
      corner_radius: None,
      on_tick: None,
      on_paint: None,
      on_position_changed: None,
      on_size_changed: None,
    }
  }

  pub fn with_title(mut self, title: &str) -> Self {
    self.attrs = self.attrs.with_title(title);
    self
  }

  pub fn with_size(mut self, width: u32, height: u32) -> Self {
    self.attrs = self.attrs.with_inner_size(winit::dpi::LogicalSize::new(width, height));
    self
  }

  pub fn with_position(mut self, x: i32, y: i32) -> Self {
    self.attrs = self
      .attrs
      .with_position(Position::Physical(PhysicalPosition::new(x, y)));
    self
  }

  pub fn with_min_size(mut self, width: u32, height: u32) -> Self {
    self.attrs = self
      .attrs
      .with_min_inner_size(winit::dpi::LogicalSize::new(width, height));
    self
  }

  pub fn with_max_size(mut self, width: u32, height: u32) -> Self {
    self.attrs = self
      .attrs
      .with_max_inner_size(winit::dpi::LogicalSize::new(width, height));
    self
  }

  pub fn with_resizable(mut self, resizable: bool) -> Self {
    self.attrs = self.attrs.with_resizable(resizable);
    self
  }

  pub fn with_decorations(mut self, decorations: bool) -> Self {
    self.attrs = self.attrs.with_decorations(decorations);
    self.tree.window().set_decorated(decorations);
    self
  }

  pub fn with_title_bar_color(mut self, color: impl Into<Option<Color>>) -> Self {
    self.attrs = with_title_bar_color(self.attrs, color.into());
    self
  }

  /// The window's icon. On Windows it is both the small icon (title bar) and
  /// the big one (taskbar button, Alt+Tab); Windows scales the image to each
  /// size. On macOS this does nothing: the Dock and the app switcher show the
  /// application bundle's icon.
  pub fn with_icon(mut self, icon: impl Into<Option<WindowIcon>>) -> Self {
    let icon = icon.into();
    self.tree.window().set_icon(icon.clone());
    self.attrs = with_window_icons(self.attrs, icon.and_then(to_winit_icon));
    self
  }

  pub fn with_corner_radius(mut self, radius: WindowCornerRadius) -> Self {
    self.attrs = with_corner_radius(self.attrs, radius);
    self.corner_radius = Some(radius);
    self.tree.window().set_corner_radius(radius);
    self
  }

  pub fn with_rounded_corners(self, rounded: bool) -> Self {
    self.with_corner_radius(if rounded {
      WindowCornerRadius::Rounded
    } else {
      WindowCornerRadius::None
    })
  }

  pub fn with_transparent(mut self, transparent: bool) -> Self {
    self.attrs = self.attrs.with_transparent(transparent);
    self
  }

  /// Show the initial window without activating the application or taking keyboard focus.
  /// Intended for app instances driven through automation rather than local input.
  /// Winit supports this startup behavior on Windows and macOS.
  /// The ways a phone's app lets its content be turned: `Orientation`s joined with `|`. Without it the app
  /// turns every way its `Info.plist` names. It does nothing on a desktop.
  pub fn with_orientations(self, orientations: impl Into<crate::app::window::Orientations>) -> Self {
    let orientations = orientations.into();
    #[cfg(target_os = "ios")]
    ios::allow_orientations(orientations);
    #[cfg(not(target_os = "ios"))]
    let _ = orientations;
    self
  }

  pub fn with_start_without_focus(mut self, start_without_focus: bool) -> Self {
    self.start_without_focus = start_without_focus;
    self.attrs = self.attrs.with_active(!start_without_focus);
    self
  }

  /// Runs during event-loop ticking before paint and may mutate the tree.
  pub fn on_tick<F>(mut self, tick: F) -> Self
  where
    F: FnMut(&mut Tree, Duration) + 'static,
  {
    self.on_tick = Some(Box::new(tick));
    self
  }

  /// Runs after a frame is presented. Use this for paint profiling.
  pub fn on_paint<F>(mut self, paint: F) -> Self
  where
    F: FnMut(&Tree, Duration, PassReport) + 'static,
  {
    self.on_paint = Some(Box::new(paint));
    self
  }

  #[deprecated(note = "use on_paint for callbacks after a frame is presented")]
  pub fn on_frame<F>(self, frame: F) -> Self
  where
    F: FnMut(&Tree, Duration, PassReport) + 'static,
  {
    self.on_paint(frame)
  }

  pub fn on_position_changed<F>(mut self, callback: F) -> Self
  where
    F: FnMut(i32, i32) + 'static,
  {
    self.on_position_changed = Some(Box::new(callback));
    self
  }

  pub fn on_size_changed<F>(mut self, callback: F) -> Self
  where
    F: FnMut(u32, u32) + 'static,
  {
    self.on_size_changed = Some(Box::new(callback));
    self
  }

  pub fn run(self) {
    // UIKit makes the scene's delegate when the loop starts the app.
    #[cfg(target_os = "ios")]
    ios::register_scene_delegate();
    let mut event_loop_builder = EventLoop::builder();
    #[cfg(target_os = "macos")]
    if self.start_without_focus {
      event_loop_builder.with_activate_ignoring_other_apps(false);
    }
    let event_loop = event_loop_builder.build().unwrap();

    // Waker for commands pushed from other threads: without it a queued
    // command waits for the next OS event while the loop idles in
    // `ControlFlow::Wait`. The proxy wakes the loop; the resulting
    // `about_to_wait` drains the queues.
    let waker: crate::app::window::WindowWaker = {
      let proxy = std::sync::Mutex::new(event_loop.create_proxy());
      std::sync::Arc::new(move || {
        if let Ok(proxy) = proxy.lock() {
          let _ = proxy.send_event(());
        }
      })
    };

    self.app.shared.menu.set_waker(waker.clone());
    // A phone or a tablet emulated on a desktop: its size and safe area, and the mouse as its fingers.
    let device = if cfg!(any(target_os = "ios", target_os = "android")) {
      None
    } else {
      self.app.emulated().or_else(crate::app::Device::from_env)
    };
    let mut attrs = self.attrs;
    if let Some(device) = &device {
      if self.app.emulated().is_none() {
        self.app.emulate(device.clone());
      }
      attrs = attrs.with_inner_size(winit::dpi::LogicalSize::new(device.size.0, device.size.1));
      self.tree.window().set_safe_area(device.safe_area);
      tracing::info!(target: "lurq::emulate", "emulating a {} of {}x{}", device.name, device.size.0, device.size.1);
    }
    let tree = self.tree;
    tree.window().attach_shell(waker.clone());
    #[cfg(feature = "mcp")]
    tree.set_mcp_waker(waker.clone());
    // A paste's text comes from the browser a turn later; see `clipboard::web`.
    #[cfg(all(target_arch = "wasm32", feature = "clipboard"))]
    crate::clipboard::web::set_waker(waker.clone());
    let secondaries = (0..tree.secondary_window_count())
      .filter_map(|index| {
        tree
          .secondary_window(index)
          .map(|secondary| ManagedSecondaryWindow::new(index, secondary))
      })
      .collect();

    #[cfg_attr(target_arch = "wasm32", allow(unused_mut))]
    let mut handler = WinitHandler {
      app: self.app,
      main: ManagedWindow::new(
        tree,
        attrs,
        self.start_without_focus,
        self.corner_radius,
        self.on_tick,
        self.on_paint,
        self.on_position_changed,
        self.on_size_changed,
        true,
      )
      .emulating(device.is_some()),
      secondaries,
      loop_cadence: WinitLoopCadence::new(Instant::now()),
      window_waker: waker,
    };
    #[cfg(target_os = "macos")]
    let _native =
      crate::app::macos_menu::NativeMenu::install(handler.app.shared.menu.clone(), handler.main.tree.window().clone());
    #[cfg(not(target_arch = "wasm32"))]
    {
      event_loop.run_app(&mut handler).unwrap();
      #[cfg(feature = "mcp")]
      handler.main.tree.shutdown_mcp();
    }
    // A page's loop is the browser's: the app is handed to it and `run` returns at once, the window alive.
    #[cfg(target_arch = "wasm32")]
    {
      use winit::platform::web::EventLoopExtWebSys;
      event_loop.spawn_app(handler);
    }
  }
}

struct ManagedWindow {
  tree: Box<Tree>,
  window: Option<Window>,
  cursor_pos: (f64, f64),
  cursor: CursorIcon,
  modifiers: ModifiersState,
  attrs: Option<WindowAttributes>,
  start_without_focus: bool,
  corner_radius: Option<WindowCornerRadius>,
  on_tick: Option<TickFn>,
  on_paint: Option<PaintFn>,
  on_position_changed: Option<PositionChangedFn>,
  on_size_changed: Option<SizeChangedFn>,
  size_reporter: SizeReporter,
  close_exits: bool,
  last_tick: Instant,
  last_paint: Instant,
  frames: FrameSource,
  continuous_redraw_until: Option<Instant>,
  /// Last direct present from inside the mouse-move stream (interactive
  /// drags paint here — see the CursorMoved arm).
  last_interaction_present: Instant,
  ime: ime::ImeSync,
  /// The system's lights of the window, where the app's bar marks its controls.
  #[cfg(target_os = "macos")]
  native_controls: macos_controls::NativeControls,
  file_drag: FileDragShell,
  /// Where the wheel turns over the canvas, which winit does not say.
  #[cfg(target_arch = "wasm32")]
  wheel_pointer: Option<web::WheelPointer>,
  /// Whether the mouse's events are a pen's; see [`web::PenPointer`].
  #[cfg(target_arch = "wasm32")]
  pen_pointer: Option<web::PenPointer>,
  /// The textarea composed and inserted text comes to; see `web_text`.
  #[cfg(target_arch = "wasm32")]
  web_text: Option<web_text::WebText>,
  /// Files dragged onto the page; see `web_drop`.
  #[cfg(target_arch = "wasm32")]
  web_drop: Option<web_drop::WebDrop>,
  /// The phone or tablet the window emulates, with the mouse as its fingers; see [`App::emulate`].
  emulator: Option<crate::app::runtime::mouse_fingers::MouseFingers>,
  /// The status bar was last asked to be hidden.
  #[cfg(target_os = "ios")]
  status_bar_hidden: bool,
  /// The size the window had at the last frame.
  #[cfg(target_os = "ios")]
  presented_size: Option<winit::dpi::PhysicalSize<u32>>,
  #[cfg(target_os = "ios")]
  status_bar_tap: Option<ios::StatusBarTap>,
  /// The device's tilt, read while the window shows glass: the light on its rims leans with it.
  #[cfg(target_os = "ios")]
  tilt: Option<ios::Tilt>,
  /// How much of the window the keyboard covered at the last frame, in logical pixels.
  #[cfg(target_os = "ios")]
  keyboard_overlap: f32,
  /// The view the keyboard types into; see [`ios_text`].
  #[cfg(target_os = "ios")]
  text_input: Option<ios_text::TextInput>,
  /// An Apple Pencil's hover over the window; see `ios::PencilHover`.
  #[cfg(target_os = "ios")]
  pencil_hover: Option<ios::PencilHover>,
  #[cfg(target_os = "ios")]
  keyboard_probe: Option<ios::KeyboardProbe>,
  /// The scroll view whose finger takes the keyboard down with it; see `ios::KeyboardDrag`.
  #[cfg(target_os = "ios")]
  keyboard_drag: Option<ios::KeyboardDrag>,
  /// The move of the keyboard during which a finger last landed; see `ios::keyboard_move`.
  #[cfg(target_os = "ios")]
  touched_during_keyboard_move: Option<u64>,
}

impl ManagedWindow {
  fn new(
    tree: Box<Tree>,
    attrs: WindowAttributes,
    start_without_focus: bool,
    corner_radius: Option<WindowCornerRadius>,
    on_tick: Option<TickFn>,
    on_paint: Option<PaintFn>,
    on_position_changed: Option<PositionChangedFn>,
    on_size_changed: Option<SizeChangedFn>,
    close_exits: bool,
  ) -> Self {
    Self {
      tree,
      window: None,
      cursor_pos: (0.0, 0.0),
      cursor: CursorIcon::Default,
      modifiers: ModifiersState::empty(),
      attrs: Some(attrs),
      start_without_focus,
      corner_radius,
      on_tick,
      on_paint,
      on_position_changed,
      on_size_changed,
      size_reporter: SizeReporter::default(),
      close_exits,
      last_tick: Instant::now(),
      last_paint: Instant::now(),
      frames: FrameSource::default(),
      continuous_redraw_until: None,
      last_interaction_present: Instant::now(),
      ime: ime::ImeSync::default(),
      #[cfg(target_os = "macos")]
      native_controls: macos_controls::NativeControls::default(),
      file_drag: FileDragShell::default(),
      #[cfg(target_arch = "wasm32")]
      wheel_pointer: None,
      #[cfg(target_arch = "wasm32")]
      pen_pointer: None,
      #[cfg(target_arch = "wasm32")]
      web_text: None,
      #[cfg(target_arch = "wasm32")]
      web_drop: None,
      emulator: None,
      #[cfg(target_os = "ios")]
      status_bar_hidden: false,
      #[cfg(target_os = "ios")]
      presented_size: None,
      #[cfg(target_os = "ios")]
      #[cfg(target_os = "ios")]
      status_bar_tap: None,
      #[cfg(target_os = "ios")]
      tilt: None,
      #[cfg(target_os = "ios")]
      keyboard_overlap: 0.0,
      #[cfg(target_os = "ios")]
      text_input: None,
      #[cfg(target_os = "ios")]
      pencil_hover: None,
      #[cfg(target_os = "ios")]
      keyboard_probe: None,
      #[cfg(target_os = "ios")]
      keyboard_drag: None,
      #[cfg(target_os = "ios")]
      touched_during_keyboard_move: None,
    }
  }

  fn window_id(&self) -> Option<WindowId> {
    self.window.as_ref().map(Window::id)
  }

  /// Makes the mouse the fingers of an emulated phone, where `emulating`.
  fn emulating(mut self, emulating: bool) -> Self {
    self.emulator = emulating.then(crate::app::runtime::mouse_fingers::MouseFingers::default);
    self
  }

  /// Gives the mouse's event to the emulated phone's fingers, where the window emulates one: `true` when it took
  /// it, and nothing else is to be done with it. The wheel and the keyboard are not the fingers'.
  fn emulate_event(&mut self, event: &WindowEvent) -> bool {
    let Some(fingers) = &mut self.emulator else {
      return false;
    };
    match event {
      WindowEvent::CursorMoved { position, .. } => {
        self.cursor_pos = (position.x, position.y);
        fingers.moved(&mut self.tree, position.x as f32, position.y as f32);
      }
      WindowEvent::CursorLeft { .. } | WindowEvent::Focused(false) => {
        fingers.left(&mut self.tree);
        return matches!(event, WindowEvent::CursorLeft { .. });
      }
      WindowEvent::MouseInput {
        state,
        button: winit::event::MouseButton::Left,
        ..
      } => match state {
        ElementState::Pressed => fingers.pressed(&mut self.tree),
        ElementState::Released => fingers.released(&mut self.tree),
      },
      // The phone's back: the mouse's side buttons go back and forward, from the focus, as Android's back does.
      WindowEvent::MouseInput {
        state: ElementState::Released,
        button: button @ (winit::event::MouseButton::Back | winit::event::MouseButton::Forward),
        ..
      } => {
        if let Some(command) = crate::app::events::NavigationCommand::of_button(to_lurq_mouse_button(*button)) {
          self.tree.navigate_from_input(
            command,
            crate::app::events::NavigationSource::Key,
            crate::app::window::NavigationOrigin::Focus,
          );
        }
      }
      // A finger has no other buttons.
      WindowEvent::MouseInput { .. } => {}
      WindowEvent::ModifiersChanged(modifiers) => {
        let state = modifiers.state();
        fingers.modifiers(&mut self.tree, state.alt_key(), state.shift_key(), state.control_key());
        return false;
      }
      // With Alt+Shift+Ctrl the wheel pinches three fingers; otherwise it scrolls, as on a desktop.
      WindowEvent::MouseWheel { delta, .. } => {
        let delta_y = match delta {
          MouseScrollDelta::LineDelta(_, y) => y * WHEEL_LINE,
          MouseScrollDelta::PixelDelta(position) => position.y as f32 / self.tree.scale_factor(),
        };
        if !fingers.wheel(&mut self.tree, delta_y) {
          return false;
        }
      }
      _ => return false,
    }
    self.check_redraw();
    true
  }

  /// Whether the loop must poll for this window: `on_tick` runs every turn,
  /// or a file event waits for the turn's end.
  fn needs_polling(&self) -> bool {
    self.on_tick.is_some() || self.tree.needs_polling() || self.file_drag.has_pending()
  }

  /// When the pointer is next read for a file drag over the window.
  fn next_file_drag_poll(&self) -> Option<Instant> {
    self.file_drag.next_poll()
  }

  /// When a finger on the window next needs the loop: to press after resting, or for a step of momentum.
  fn next_touch_deadline(&self) -> Option<Instant> {
    #[cfg(target_os = "ios")]
    let keyboard_moving = ios::keyboard_moving();
    #[cfg(not(target_os = "ios"))]
    let keyboard_moving = false;
    self
      .tree
      .finger_deadline()
      .into_iter()
      .chain(keyboard_moving.then(|| Instant::now() + finger::SPRING_STEP))
      .min()
  }

  fn create_window(&mut self, event_loop: &ActiveEventLoop, app: &mut App) {
    if self.window.is_some() {
      return;
    }
    sync_system_motion(app);

    let attrs = self.attrs.take().unwrap_or_default();
    self.tree.window().record_initial_title(&attrs.title);
    // macOS set_visible(true) makes a window key even when it was created inactive.
    let show_after_first_present = attrs.visible && !self.start_without_focus;
    let attrs = if show_after_first_present {
      attrs.with_visible(false)
    } else {
      attrs
    };
    #[cfg(target_arch = "wasm32")]
    let attrs = web::window_attributes(attrs);
    let window = event_loop.create_window(attrs).unwrap();
    #[cfg(target_arch = "wasm32")]
    {
      self.wheel_pointer = web::WheelPointer::watch(&window);
      self.pen_pointer = web::PenPointer::watch(&window);
      self.web_text = web_text::WebText::new(&window, self.tree.window().loop_waker());
      self.web_drop = web_drop::WebDrop::new(&window, self.tree.window().loop_waker());
    }
    if let Some(radius) = self.corner_radius {
      set_corner_radius(&window, radius);
      self.tree.window().set_corner_radius(radius);
    }
    let size = surface_size(&window);
    self.tree.set_scale_factor(window.scale_factor() as f32);
    self.tree.resize(size.width, size.height);
    #[cfg(target_os = "ios")]
    {
      self
        .tree
        .set_double_click_distance(finger::DOUBLE_TAP_DISTANCE * window.scale_factor() as f32);
      self.tree.set_double_click_interval(finger::DOUBLE_TAP_INTERVAL);
      ios::watch_keyboard();
      self.frames.attach();
      if let Ok(RawWindowHandle::UiKit(handle)) = window.window_handle().map(|handle| handle.as_raw()) {
        ios::join_scene(handle.ui_view);
        ios::hear_hardware_keys(handle.ui_view);
        ios::apply_orientations(handle.ui_view);
        self.status_bar_tap = Some(ios::StatusBarTap::new(handle.ui_view));
        self.keyboard_drag = Some(ios::KeyboardDrag::new(handle.ui_view));
        self.text_input = Some(ios_text::TextInput::new(handle.ui_view));
        self.pencil_hover = ios::PencilHover::new(handle.ui_view);
        // The platform draws the focused input's caret and selection; see `ios_text`.
        self.tree.set_native_text_selection(true);
        self.keyboard_probe = Some(ios::KeyboardProbe::new(handle.ui_view));
      }
    }
    if let Ok(position) = window.outer_position() {
      self.tree.set_window_position(position.x, position.y);
      self.notify_position_changed(position.x, position.y);
    }
    // A dialog of the system opened over this window is modal to it; see `files::FileDialog`.
    #[cfg(windows)]
    self
      .tree
      .window()
      .set_parent(crate::app::window::parent::ParentWindow::of(&window));
    // The system is told where the window's controls are in a title bar the app draws itself.
    #[cfg(windows)]
    tell_window_controls(&window, &mut self.tree);
    // On macOS the system's own lights go where the app's controls are: see `macos_controls`.
    #[cfg(target_os = "macos")]
    self.tree.publish_window_controls();
    #[cfg(all(target_os = "macos", feature = "screenshot"))]
    macos_controls::draw_lights_in_screenshots(&window, &mut self.tree);
    self.window = Some(window);
    self.sync_window_state();
    let presented = show_after_first_present && self.present_now(app, true);
    if show_after_first_present {
      if let Some(window) = &self.window {
        window.set_visible(true);
      }
      self.tree.request_redraw();
      self.check_redraw();
    }
    if !presented {
      self.request_redraw();
    }
  }

  fn sync_window_state(&mut self) -> bool {
    let Some(window) = &self.window else {
      return false;
    };

    let size = surface_size(window);
    let minimized = window.is_minimized();
    let maximized = window.is_maximized();
    let full_screen = window.fullscreen().is_some();
    let focused = window.has_focus();
    let current = self.tree.window().info();
    let size_changed =
      current.resolved_width.round() as u32 != size.width || current.resolved_height.round() as u32 != size.height;

    if size_changed {
      self.tree.resize(size.width, size.height);
    }
    if let Some(minimized) = minimized {
      self.tree.window().set_minimized(minimized);
    }
    self.tree.window().set_maximized(maximized);
    self.tree.window().set_full_screen(full_screen);
    self.tree.window().set_focused(focused);
    self.report_size();

    size_changed
  }

  fn apply_window_commands(&mut self, event_loop: &ActiveEventLoop) -> bool {
    let commands = self.tree.window().take_shell_commands();
    if commands.is_empty() {
      return false;
    }

    let mut closed = false;
    for command in commands {
      match command {
        WindowCommand::RequestClose(_) => unreachable!("resolved by take_shell_commands"),
        WindowCommand::Close => {
          if self.close_exits {
            event_loop.exit();
          } else {
            self.window = None;
            #[cfg(windows)]
            self.tree.window().set_parent(None);
            self.frames.settle();
            closed = true;
          }
        }
        WindowCommand::SetMinimized(minimized) => {
          if let Some(window) = &self.window {
            window.set_minimized(minimized);
          }
          self.tree.window().set_minimized(minimized);
        }
        WindowCommand::SetMaximized(maximized) => {
          if let Some(window) = &self.window {
            window.set_maximized(maximized);
          }
          self.tree.window().set_maximized(maximized);
        }
        WindowCommand::SetFullScreen(full_screen) => {
          if let Some(window) = &self.window {
            window.set_fullscreen(full_screen.then_some(Fullscreen::Borderless(None)));
          }
          self.tree.window().set_full_screen(full_screen);
        }
        WindowCommand::SetDecorated(decorated) => {
          if let Some(window) = &self.window {
            window.set_decorations(decorated);
          }
          self.tree.window().set_decorated(decorated);
        }
        WindowCommand::Haptic(haptic) => {
          #[cfg(target_os = "ios")]
          ios::play_haptic(haptic);
          #[cfg(not(target_os = "ios"))]
          let _ = haptic;
        }
        WindowCommand::SetTitle(title) => {
          if let Some(window) = &self.window {
            window.set_title(&title);
          }
        }
        WindowCommand::SetTitleBarColor(color) => {
          if let Some(window) = &self.window {
            set_title_bar_color(window, color);
          }
        }
        WindowCommand::SetBorderColor(color) => {
          if let Some(window) = &self.window {
            set_border_color(window, color);
          }
          self.tree.window().set_border_color(color);
        }
        WindowCommand::SetIcon(icon) => {
          if let Some(window) = &self.window {
            set_window_icons(window, icon.and_then(to_winit_icon));
          }
        }
        WindowCommand::SetCornerRadius(radius) => {
          if let Some(window) = &self.window {
            set_corner_radius(window, radius);
          }
          self.tree.window().set_corner_radius(radius);
        }
        WindowCommand::Move { x, y } => {
          if let Some(window) = &self.window {
            window.set_outer_position(PhysicalPosition::new(x, y));
          }
          self.tree.set_window_position(x, y);
          self.notify_position_changed(x, y);
        }
        WindowCommand::Resize(request) => {
          // The size the window took reaches `on_size_changed` from `sync_window_state` below.
          self.tree.window().apply_resize(self.window.as_ref(), request);
        }
        WindowCommand::StartDrag => {
          if self.window.as_ref().is_some_and(start_native_window_drag) {
            let (x, y) = (self.cursor_pos.0 as f32, self.cursor_pos.1 as f32);
            self.tree.mouse_press_taken_by_os(x, y, MouseButton::Left);
          }
        }
        WindowCommand::StartResize(direction) => {
          if self
            .window
            .as_ref()
            .is_some_and(|window| start_native_window_resize(window, direction))
          {
            let (x, y) = (self.cursor_pos.0 as f32, self.cursor_pos.1 as f32);
            self.tree.mouse_press_taken_by_os(x, y, MouseButton::Left);
          }
        }
        WindowCommand::StopDrag => {}
        WindowCommand::ShowSystemMenu => {
          if let Some(window) = &self.window {
            let (x, y) = self.cursor_pos;
            window.show_window_menu(PhysicalPosition::new(x, y));
          }
        }
        WindowCommand::InjectInput(input) => {
          crate::app::synthetic_input::apply(&mut self.tree, &input);
        }
        #[cfg(feature = "clipboard")]
        WindowCommand::PasteFromClipboard { synthetic } => {
          self.tree.paste_from_os_clipboard(synthetic);
        }
        #[cfg(feature = "screenshot")]
        WindowCommand::Screenshot(output_path, region) => {
          self.tree.request_screenshot(output_path, region);
        }
        WindowCommand::OpenDevtools => {
          self.tree.open_devtools();
        }
        WindowCommand::CloseDevtools => {
          self.tree.close_devtools();
        }
        WindowCommand::ToggleDevtools => {
          self.tree.toggle_devtools();
        }
      }
    }
    if self.sync_window_state() {
      self.request_redraw();
    }
    closed
  }

  fn check_redraw(&mut self) {
    if self.tree.needs_redraw() {
      self.request_redraw();
    }
    if let Some(window) = &self.window {
      self.ime.apply(window, &self.tree);
      #[cfg(target_os = "macos")]
      self.native_controls.apply(window, &self.tree);
    }
  }

  fn request_redraw(&mut self) {
    if let Some(window) = &self.window {
      self.frames.request(window);
    }
  }

  /// Whether a frame is to be presented in this turn of the loop; see [`FrameSource::take_due`].
  fn take_frame_due(&mut self) -> bool {
    self.frames.take_due(self.tree.needs_redraw())
  }

  fn present_now(&mut self, app: &mut App, request_followup_redraw: bool) -> bool {
    if let Some(w) = &self.window {
      let started_at = Instant::now();
      let setup_started_at = Instant::now();
      let size = surface_size(w);
      self.tree.set_scale_factor(w.scale_factor() as f32);
      self.tree.resize(size.width, size.height);
      #[cfg(target_os = "ios")]
      {
        self.presented_size = Some(size);
        // A phone held on its side shows no status bar, as the platform's apps do not; winit's view controller
        // would keep it.
        {
          use winit::platform::ios::WindowExtIOS;
          let on_its_side = size.width > size.height;
          if self.status_bar_hidden != on_its_side {
            self.status_bar_hidden = on_its_side;
            w.set_prefers_status_bar_hidden(on_its_side);
          }
        }
        let mut safe_area = safe_area_insets(w);
        // What the keyboard covers is not safe either; content padded by the safe area rises above it.
        let logical_height = f64::from(size.height) / w.scale_factor();
        let keyboard = self
          .keyboard_probe
          .as_ref()
          .map_or(0.0, |probe| probe.overlap(logical_height)) as f32;
        // With no keyboard its guide rests at the bottom safe area: what it covers over that is the keyboard's.
        safe_area.keyboard = (keyboard - safe_area.bottom).max(0.0);
        safe_area.bottom = safe_area.bottom.max(keyboard);
        if keyboard != self.keyboard_overlap {
          self.keyboard_overlap = keyboard;
          // It is coming up, or changed its height: the input being typed into stays in view, frame by frame as
          // the window shrinks. Unless a finger landed since the keyboard set off: the person scrolls where
          // they mean to, and bringing the input back each frame would hold their scroll until the keyboard
          // came to rest.
          if self.touched_during_keyboard_move != Some(ios::keyboard_move()) {
            self.tree.reveal_focused_input();
          }
        }
        self.tree.window().set_safe_area(safe_area);
        if let Some(status_bar_tap) = &mut self.status_bar_tap {
          status_bar_tap.set_height(f64::from(safe_area.top));
        }
      }
      self.frames.settle();
      #[cfg(feature = "perf_profile")]
      self
        .tree
        .set_profiled_refresh_interval(self.frames.refresh_interval(Some(w), setup_started_at));
      let setup = setup_started_at.elapsed();
      let pass_started_at = Instant::now();
      let report = self.tree.pass(app, w);
      let pass = pass_started_at.elapsed();
      // A pass hit-tests the hover again when content moved under a pointer
      // that held still, which can change the cursor without an input event.
      set_window_cursor(w, &mut self.cursor, self.tree.cursor());
      // The pass painted the focused input's caret where it now is.
      self.ime.apply(w, &self.tree);
      #[cfg(target_arch = "wasm32")]
      if let Some(web_text) = &mut self.web_text {
        web_text.follow(&self.tree);
      }
      // The pass laid the window's controls out: the system's lights go where the app's are.
      #[cfg(target_os = "macos")]
      self.native_controls.apply(w, &self.tree);
      // The pass laid the focused input's text out: the platform's caret goes where it now ends, with this
      // frame and not the one after.
      #[cfg(target_os = "ios")]
      if let Some(text_input) = &mut self.text_input {
        text_input.sync(&mut self.tree);
      }
      let presented = report.rendered;
      let mut paint_duration = Duration::ZERO;
      if presented {
        #[cfg(target_arch = "wasm32")]
        web::mark_first_frame();
        let now = Instant::now();
        self.frames.presented(PresentedFrame {
          started: pass_started_at,
          presented: now,
        });
        let delta = now.duration_since(self.last_paint);
        self.last_paint = now;
        if let Some(paint) = &mut self.on_paint {
          let paint_started_at = Instant::now();
          paint(&self.tree, delta, report);
          paint_duration = paint_started_at.elapsed();
        }
      }
      let followup_started_at = Instant::now();
      if request_followup_redraw {
        self.check_redraw();
      }
      let followup = followup_started_at.elapsed();
      let total = started_at.elapsed();
      if total >= WINIT_BREAKDOWN_THRESHOLD {
        crate::app::profile_support::video_log!(debug,
          target: "video::watch::lurq",
          "winit present_now breakdown total_ms={:.2} setup_ms={:.2} pass_ms={:.2} paint_ms={:.2} followup_ms={:.2} rendered={} cached={} request_followup_redraw={} reasons={:?}",
          duration_ms(total),
          duration_ms(setup),
          duration_ms(pass),
          duration_ms(paint_duration),
          duration_ms(followup),
          presented,
          report.used_cached_render_list,
          request_followup_redraw,
          report.reasons
        );
      }
      return presented;
    }
    false
  }

  fn has_continuous_redraw_video_recently(&mut self, now: Instant) -> bool {
    if self.tree.has_continuous_redraw_video() {
      self.continuous_redraw_until = now.checked_add(CONTINUOUS_REDRAW_GRACE);
      return true;
    }

    if self.continuous_redraw_until.is_some_and(|until| now <= until) {
      return true;
    }

    self.continuous_redraw_until = None;
    false
  }

  fn apply_cursor(&mut self) {
    match &self.window {
      Some(window) => set_window_cursor(window, &mut self.cursor, self.tree.cursor()),
      None => self.cursor = self.tree.cursor(),
    }
  }

  fn tick(&mut self) {
    let now = Instant::now();
    let delta = now.duration_since(self.last_tick);
    self.last_tick = now;

    if let Some(tick) = &mut self.on_tick {
      tick(&mut self.tree, delta);
    }
    self.tree.tick_timers();
    self.tree.tick_futures();
    self.tree.finger_tick(now);
    #[cfg(target_os = "ios")]
    {
      // An Apple Pencil hovering over the window hovers what is under it.
      let scale = self.tree.scale_factor() as f64;
      for (over, x, y) in ios::take_pencil_hovers() {
        if over {
          self.tree.pen_hover((x * scale) as f32, (y * scale) as f32);
        } else {
          self.tree.pen_hover_ended();
        }
      }
      // What the keyboard did goes to the focused input, and the keyboard follows the focus.
      if let Some(text_input) = &mut self.text_input {
        text_input.sync(&mut self.tree);
      }
      // The keyboard is on its way up or down: every frame lays the window out around where it is.
      if ios::keyboard_moving() {
        self.tree.request_redraw();
      }
      // The device was turned: the window takes its new size during the turn, with no event when it has it.
      // A frame drawn at a size the window had on the way would stay, half of the screen, until something
      // else asked for one.
      if let Some(window) = &self.window {
        let size = surface_size(window);
        if self.presented_size != Some(size) {
          self.tree.request_redraw();
        }
      }
      // The keys of a keyboard attached to the device.
      for key in ios::take_hardware_keys() {
        if key.down {
          self
            .tree
            .key_down_with_meta(key.key, key.code, key.shift, key.ctrl, key.alt, key.meta);
        } else {
          self
            .tree
            .key_up_with_meta(key.key, key.code, key.shift, key.ctrl, key.alt, key.meta);
        }
      }
      ios::watch_orientation();
      // What the scene said of the app goes to the window's handler; what the app was started with waits
      // until there is one.
      if self.tree.window().has_app_event_handler() {
        let events = ios::take_app_events();
        if !events.is_empty() {
          for event in &events {
            self.tree.window().dispatch_app_event(event);
          }
          self.tree.request_redraw();
        }
      }
      if self.status_bar_tap.as_mut().is_some_and(ios::StatusBarTap::take_tapped) {
        self.tree.touch_scroll_to_top(now);
      }
      // The light on glass leans as the device is tilted; the sensor is read only while there is glass.
      let has_glass = self.tree.has_glass();
      if has_glass || self.tilt.is_some() {
        let tilt = self.tilt.get_or_insert_with(ios::Tilt::new);
        tilt.set_running(has_glass);
        if let Some(light) = tilt.take() {
          self.tree.set_glass_light(light);
        }
      }
    }
    self
      .file_drag
      .flush(&mut self.tree, self.window.as_ref(), self.cursor_pos);
    let has_continuous_video = self.has_continuous_redraw_video_recently(now);
    if has_continuous_video {
      // Video streams are presented directly from about_to_wait. Requesting
      // winit redraw events for the same stream adds a second, irregular presentation
      // path that can reset last_paint and create visible cadence gaps.
      self.frames.see_window(&mut self.tree);
    } else {
      // Before the scheduled redraws are ticked: a hidden window drops them.
      let window = self.window.as_ref();
      self.frames.request_timeline_frame(&mut self.tree, window, now);
    }
    self.tree.tick_scheduled_redraw(now);
    if !has_continuous_video {
      self.check_redraw();
    }
  }

  fn notify_position_changed(&mut self, x: i32, y: i32) {
    if let Some(callback) = &mut self.on_position_changed {
      callback(x, y);
    }
  }

  /// Reports the window's size to `on_size_changed`, in logical pixels, unless the window is minimized (its client
  /// area is then not its size) or the size was already reported.
  fn report_size(&mut self) {
    let (Some(window), Some(callback)) = (&self.window, &mut self.on_size_changed) else {
      return;
    };
    let size = surface_size(window);
    let scale = self.tree.window().info().scale_factor.max(f32::EPSILON);
    let logical = (
      ((size.width as f32) / scale).round().max(1.0) as u32,
      ((size.height as f32) / scale).round().max(1.0) as u32,
    );
    if let Some((width, height)) = self.size_reporter.next(window.is_minimized() == Some(true), logical) {
      callback(width, height);
    }
  }

  fn handle_event(&mut self, app: &mut App, event_loop: &ActiveEventLoop, event: WindowEvent) -> bool {
    #[cfg(feature = "perf_profile")]
    stamp_profiled_input(&mut self.tree, &event);
    self.tree.set_app_ref(app);
    self.frames.handle_event(&event);
    if self.file_drag.on_event(&event) {
      // Delivered at the turn's end, once the burst of one file per event
      // is complete; see `file_drag`.
      return false;
    }
    // In the browser a pen's pointer events come as the mouse's: the tree is told which they are.
    #[cfg(target_arch = "wasm32")]
    if matches!(event, WindowEvent::CursorMoved { .. } | WindowEvent::MouseInput { .. })
      && let Some(pen) = &self.pen_pointer
    {
      self.tree.set_mouse_is_pen(pen.is_pen());
    }
    if self.emulate_event(&event) {
      self.apply_window_commands(event_loop);
      return false;
    }
    match event {
      WindowEvent::CloseRequested => {
        self.tree.window().dispatch_close_request(CloseRequestSource::Os);
        self.request_redraw();
      }
      WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
        self.tree.set_scale_factor(scale_factor as f32);
        #[cfg(target_os = "ios")]
        self
          .tree
          .set_double_click_distance(finger::DOUBLE_TAP_DISTANCE * scale_factor as f32);
        if let Some(w) = &self.window {
          let size = surface_size(w);
          self.tree.resize(size.width, size.height);
          self.request_redraw();
        }
      }
      WindowEvent::Resized(size) => {
        // On iOS the event's size is the safe area's, and the tree is as large as the screen. In a browser it can
        // be the canvas's CSS size, not its pixels' (`surface_size`).
        #[cfg(any(target_os = "ios", target_arch = "wasm32"))]
        let size = self.window.as_ref().map_or(size, surface_size);
        self.tree.resize(size.width, size.height);
        self.sync_window_state();
        // Paint the new size from inside the event: during a live edge drag Windows runs a modal sizing
        // loop that keeps delivering WM_SIZE, but WM_PAINT (and so `RedrawRequested`) only when the
        // queue is idle, and DWM stretches the previous frame until a new one is presented.
        // Only for the current size: size events queued while winit was busy arrive back to back, and
        // painting each stale one would replay the whole drag.
        let current = self.window.as_ref().is_some_and(|w| surface_size(w) == size);
        if current && size.width > 0 && size.height > 0 && self.present_now(app, false) {
          self.apply_window_commands(event_loop);
          return true;
        }
        self.request_redraw();
      }
      WindowEvent::Focused(focused) => {
        self.tree.window().set_focused(focused);
        // A window that takes focus may be coming back from the system's settings.
        if focused {
          sync_system_motion(app);
        }
        self.request_redraw();
      }
      WindowEvent::Moved(position) => {
        self.tree.set_window_position(position.x, position.y);
        self.notify_position_changed(position.x, position.y);
      }
      WindowEvent::CursorMoved { position, .. } => {
        self.cursor_pos = (position.x, position.y);
        self.tree.mouse_move_with_meta(
          position.x as f32,
          position.y as f32,
          self.modifiers.shift_key(),
          self.modifiers.control_key(),
          self.modifiers.alt_key(),
          self.modifiers.super_key(),
        );
        self.apply_cursor();
        self.check_redraw();
        // Any redraw this move produced (interactive drags — scrollbars,
        // sliders, text selection, on_drag_* sessions — but equally hover
        // enter/leave restyles) must paint from inside the event stream: a
        // high-rate mouse never lets the queue drain, so both
        // `about_to_wait` and WM_PAINT are starved for the whole gesture.
        // Gated on drags only, a drag repainted but a hover sweep skipped
        // every row between where the pointer started and where it paused.
        if self.tree.needs_redraw() && self.last_interaction_present.elapsed() >= INTERACTION_PRESENT_INTERVAL {
          self.last_interaction_present = Instant::now();
          let presented = self.present_now(app, false);
          if presented {
            self.apply_window_commands(event_loop);
            return true;
          }
        }
      }
      WindowEvent::CursorLeft { .. } => {
        self.tree.mouse_leave_window();
        self.apply_cursor();
        self.check_redraw();
      }
      WindowEvent::MouseInput { state, button, .. } => {
        let btn = to_lurq_mouse_button(button);
        let (x, y) = (self.cursor_pos.0 as f32, self.cursor_pos.1 as f32);
        match state {
          ElementState::Pressed => self.tree.mouse_down_with_meta(
            x,
            y,
            btn,
            self.modifiers.shift_key(),
            self.modifiers.control_key(),
            self.modifiers.alt_key(),
            self.modifiers.super_key(),
          ),
          ElementState::Released => self.tree.mouse_up_with_meta(
            x,
            y,
            btn,
            self.modifiers.shift_key(),
            self.modifiers.control_key(),
            self.modifiers.alt_key(),
            self.modifiers.super_key(),
          ),
        }
        self.apply_window_commands(event_loop);
        self.apply_cursor();
        self.check_redraw();
        return false;
      }
      WindowEvent::ModifiersChanged(modifiers) => {
        self.modifiers = modifiers.state();
      }
      WindowEvent::Ime(ime) => {
        self.tree.ime(ime::to_ime_event(ime));
        self.check_redraw();
      }
      WindowEvent::KeyboardInput { event, .. } => {
        if is_open_devtools_shortcut(&event, self.modifiers) {
          self.tree.open_devtools();
          self.apply_cursor();
          self.check_redraw();
          return false;
        }
        match event.state {
          ElementState::Pressed => self.tree.key_down_with_meta(
            key_to_string(&event),
            physical_key_to_string(&event.physical_key),
            self.modifiers.shift_key(),
            self.modifiers.control_key(),
            self.modifiers.alt_key(),
            self.modifiers.super_key(),
          ),
          ElementState::Released => self.tree.key_up_with_meta(
            key_to_string(&event),
            physical_key_to_string(&event.physical_key),
            self.modifiers.shift_key(),
            self.modifiers.control_key(),
            self.modifiers.alt_key(),
            self.modifiers.super_key(),
          ),
        }
        self.apply_cursor();
        self.check_redraw();
      }
      WindowEvent::MouseWheel { delta, phase, .. } => {
        #[cfg(not(target_arch = "wasm32"))]
        let (mut dx, mut dy) = match delta {
          MouseScrollDelta::LineDelta(x, y) => (x * WHEEL_LINE, y * WHEEL_LINE),
          MouseScrollDelta::PixelDelta(p) => (p.x as f32, p.y as f32),
        };
        #[cfg(not(target_arch = "wasm32"))]
        let notched = notched_wheel::is_notched(delta);
        // A browser tells a notch as the distance it would scroll a page, a mouse's as a touchpad's; see
        // `web_wheel`.
        #[cfg(target_arch = "wasm32")]
        let ((mut dx, mut dy), notched) = {
          let scale = f64::from(self.tree.scale_factor());
          let system_distance = web::scrolls_the_systems_distance();
          (
            web_wheel::scroll_distance(delta, scale, system_distance),
            web_wheel::is_notched(delta, scale, system_distance),
          )
        };
        if self.modifiers.shift_key() && dx == 0.0 {
          dx = dy;
          dy = 0.0;
        }
        let scroll_phase = match phase {
          TouchPhase::Started => ScrollPhase::Start,
          TouchPhase::Moved => ScrollPhase::Scroll,
          TouchPhase::Ended | TouchPhase::Cancelled => ScrollPhase::End,
        };
        if phase == TouchPhase::Started {
          self.tree.set_history_swipes(history_swipes::enabled());
        }
        // The wheel's own event says where the pointer is, also before it has moved; see `web::WheelPointer`.
        #[cfg(target_arch = "wasm32")]
        if let Some(position) = self
          .wheel_pointer
          .as_ref()
          .and_then(|pointer| pointer.take(f64::from(self.tree.scale_factor())))
        {
          self.cursor_pos = position;
        }
        let (x, y) = (self.cursor_pos.0 as f32, self.cursor_pos.1 as f32);
        // A mouse's notches are animated; a touchpad's travel moves the content with the fingers.
        // Ctrl goes with it: a touchpad's pinch on Windows, and in a browser, is the wheel with Ctrl held.
        let m = self.modifiers;
        let keys = (m.shift_key(), m.control_key(), m.alt_key(), m.super_key());
        if notched {
          self
            .tree
            .wheel_with_meta(x, y, dx, dy, scroll_phase, keys.0, keys.1, keys.2, keys.3);
        } else {
          self
            .tree
            .scroll_with_meta(x, y, dx, dy, scroll_phase, keys.0, keys.1, keys.2, keys.3);
        }
        self.apply_cursor();
        self.check_redraw();
      }
      // A Mac trackpad's pinch, rotation and smart zoom, at the pointer.
      WindowEvent::PinchGesture { .. } | WindowEvent::RotationGesture { .. } | WindowEvent::DoubleTapGesture { .. } => {
        trackpad_gestures::deliver(&mut self.tree, &event, self.cursor_pos);
        self.apply_cursor();
        self.check_redraw();
      }
      // winit tells a finger (and a pen) as a touch on iOS, Android, Windows, Linux and in the browser.
      WindowEvent::Touch(touch) => {
        #[cfg(target_os = "ios")]
        if touch.phase == TouchPhase::Started && ios::keyboard_moving() {
          self.touched_during_keyboard_move = Some(ios::keyboard_move());
        }
        // Whether the keyboard follows this finger down, said before the finger moves: a list under it asks.
        #[cfg(target_os = "ios")]
        if touch.phase == TouchPhase::Started
          && let Some(drag) = &mut self.keyboard_drag
        {
          let follows = self
            .tree
            .touch_keyboard_follows_at(touch.location.x as f32, touch.location.y as f32);
          drag.set_interactive(follows);
        }
        let route = deliver_touch(&mut self.tree, &mut self.cursor_pos, &touch, Instant::now());
        if route != crate::app::runtime::TouchRoute::Pointer {
          self.apply_window_commands(event_loop);
          self.check_redraw();
          return false;
        }
      }
      // UIKit draws a view from `drawRect:`, which is where winit sends this event from, and drops a
      // `setNeedsDisplay` made inside it: a redraw the pass asks for (a second layout pass, an animation's next
      // frame) would wait for an event that never comes, and with the frame still asked for nothing would ask again.
      // The loop takes another turn instead, and asks from there.
      #[cfg(target_os = "ios")]
      WindowEvent::RedrawRequested => {
        let presented = self.present_now(app, false);
        self.apply_window_commands(event_loop);
        if self.tree.needs_redraw() {
          event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
        }
        return presented;
      }
      #[cfg(not(target_os = "ios"))]
      WindowEvent::RedrawRequested => {
        let presented = self.present_now(app, true);
        self.apply_window_commands(event_loop);
        return presented;
      }
      _ => {}
    }
    self.apply_window_commands(event_loop);
    self.check_redraw();
    false
  }
}

struct ManagedSecondaryWindow {
  index: usize,
  window: Option<Window>,
  cursor_pos: (f64, f64),
  cursor: CursorIcon,
  modifiers: ModifiersState,
  attrs: Option<WindowAttributes>,
  close_requested: bool,
  frames: FrameSource,
  ime: ime::ImeSync,
  /// The system's lights of the window, where the app's bar marks its controls.
  #[cfg(target_os = "macos")]
  native_controls: macos_controls::NativeControls,
  file_drag: FileDragShell,
}

/// What a further window of the app is on a tablet or on a display plugged in: a window in a scene of its
/// own, which UIKit makes and the person or the system may close.
#[cfg(target_os = "ios")]
impl ManagedSecondaryWindow {
  /// winit's view in the window.
  fn ui_view(&self) -> Option<std::ptr::NonNull<std::ffi::c_void>> {
    match self.window.as_ref()?.window_handle().ok()?.as_raw() {
      RawWindowHandle::UiKit(handle) => Some(handle.ui_view),
      _ => None,
    }
  }

  /// Whether the scene the window was shown in is gone, by the person's doing or the system's.
  fn scene_closed(&self) -> bool {
    self.ui_view().is_some_and(ios::other_scene_closed)
  }

  /// The app closes the window: its scene goes with it.
  fn leave_scene(&self) {
    if let Some(view) = self.ui_view() {
      ios::leave_other_scene(view);
    }
  }
}

impl ManagedSecondaryWindow {
  /// When the finger on the window, or what it left moving, next needs the loop.
  fn next_touch_deadline(&self, tree: &Tree) -> Option<Instant> {
    let _ = self;
    tree.finger_deadline()
  }

  /// A touch on the window: the first finger is its pointer, as on the app's first window.
  fn handle_touch(&mut self, tree: &mut Tree, touch: winit::event::Touch) {
    deliver_touch(tree, &mut self.cursor_pos, &touch, Instant::now());
  }

  /// What time brings a finger on the window: its press, its momentum, and content springing back.
  fn tick_touch(&mut self, tree: &mut Tree, now: Instant) {
    let _ = self;
    tree.finger_tick(now);
  }
}

impl ManagedSecondaryWindow {
  fn new(index: usize, secondary: &SecondaryWindow) -> Self {
    Self {
      index,
      window: None,
      cursor_pos: (0.0, 0.0),
      cursor: CursorIcon::Default,
      modifiers: ModifiersState::empty(),
      attrs: Some(
        WindowAttributes::default()
          .with_title(secondary.title())
          .with_decorations(secondary.decorations())
          .with_window_level(if secondary.always_on_top() {
            winit::window::WindowLevel::AlwaysOnTop
          } else {
            winit::window::WindowLevel::Normal
          })
          .with_inner_size(winit::dpi::LogicalSize::new(secondary.width(), secondary.height())),
      ),
      close_requested: false,
      frames: FrameSource::default(),
      ime: ime::ImeSync::default(),
      #[cfg(target_os = "macos")]
      native_controls: macos_controls::NativeControls::default(),
      file_drag: FileDragShell::default(),
    }
  }

  fn index(&self) -> usize {
    self.index
  }

  fn needs_polling(&self) -> bool {
    self.file_drag.has_pending()
  }

  fn next_file_drag_poll(&self) -> Option<Instant> {
    self.file_drag.next_poll()
  }

  fn window_id(&self) -> Option<WindowId> {
    self.window.as_ref().map(Window::id)
  }

  fn request_close(&mut self) {
    self.frames.settle();
    self.close_requested = true;
  }

  fn close_requested(&self) -> bool {
    self.close_requested
  }

  fn create_window(
    &mut self,
    event_loop: &ActiveEventLoop,
    app: &mut App,
    tree: &mut Tree,
  ) -> Option<SecondaryWindowMetadata> {
    if self.window.is_some() {
      return None;
    }

    let attrs = self.attrs.take().unwrap_or_default();
    let show_after_first_present = attrs.visible;
    let attrs = if show_after_first_present {
      attrs.with_visible(false)
    } else {
      attrs
    };
    let window = event_loop.create_window(attrs).unwrap();
    let size = surface_size(&window);
    let metadata = secondary_window_metadata(&window);
    tree.set_scale_factor(window.scale_factor() as f32);
    tree.resize(size.width, size.height);
    if let Ok(position) = window.outer_position() {
      tree.set_window_position(position.x, position.y);
    }
    #[cfg(windows)]
    tree
      .window()
      .set_parent(crate::app::window::parent::ParentWindow::of(&window));
    #[cfg(windows)]
    tell_window_controls(&window, tree);
    #[cfg(target_os = "macos")]
    tree.publish_window_controls();
    #[cfg(all(target_os = "macos", feature = "screenshot"))]
    macos_controls::draw_lights_in_screenshots(&window, tree);
    self.window = Some(window);
    // A window of UIKit's is seen in a scene: one the system has ready for it, or one it is asked for now.
    #[cfg(target_os = "ios")]
    if let Some(view) = self.ui_view() {
      ios::join_other_scene(view);
    }
    self.sync_window_state(tree);
    let presented = show_after_first_present && self.present_now(app, tree);
    if show_after_first_present && let Some(window) = &self.window {
      window.set_visible(true);
    }
    if !presented {
      self.request_redraw();
    }
    Some(metadata)
  }

  fn sync_window_state(&mut self, tree: &mut Tree) {
    if let Some(window) = &self.window {
      if let Some(minimized) = window.is_minimized() {
        tree.window().set_minimized(minimized);
      }
      tree.window().set_maximized(window.is_maximized());
      tree.window().set_full_screen(window.fullscreen().is_some());
      tree.window().set_decorated(window.is_decorated());
      tree.window().set_focused(window.has_focus());
    }
  }

  fn apply_window_commands(&mut self, tree: &mut Tree) -> bool {
    let commands = tree.window().take_shell_commands();
    if commands.is_empty() {
      return false;
    }

    let mut closed = false;
    for command in commands {
      match command {
        WindowCommand::RequestClose(_) => unreachable!("resolved by take_shell_commands"),
        WindowCommand::Close => {
          self.request_close();
          closed = true;
        }
        WindowCommand::SetMinimized(minimized) => {
          if let Some(window) = &self.window {
            window.set_minimized(minimized);
          }
          tree.window().set_minimized(minimized);
        }
        WindowCommand::SetMaximized(maximized) => {
          if let Some(window) = &self.window {
            window.set_maximized(maximized);
          }
          tree.window().set_maximized(maximized);
        }
        WindowCommand::SetFullScreen(full_screen) => {
          if let Some(window) = &self.window {
            window.set_fullscreen(full_screen.then_some(Fullscreen::Borderless(None)));
          }
          tree.window().set_full_screen(full_screen);
        }
        WindowCommand::SetDecorated(decorated) => {
          if let Some(window) = &self.window {
            window.set_decorations(decorated);
          }
          tree.window().set_decorated(decorated);
        }
        WindowCommand::Haptic(haptic) => {
          #[cfg(target_os = "ios")]
          ios::play_haptic(haptic);
          #[cfg(not(target_os = "ios"))]
          let _ = haptic;
        }
        WindowCommand::SetTitle(title) => {
          if let Some(window) = &self.window {
            window.set_title(&title);
          }
        }
        WindowCommand::SetTitleBarColor(color) => {
          if let Some(window) = &self.window {
            set_title_bar_color(window, color);
          }
        }
        WindowCommand::SetBorderColor(color) => {
          if let Some(window) = &self.window {
            set_border_color(window, color);
          }
          tree.window().set_border_color(color);
        }
        WindowCommand::SetIcon(icon) => {
          if let Some(window) = &self.window {
            set_window_icons(window, icon.and_then(to_winit_icon));
          }
        }
        WindowCommand::SetCornerRadius(radius) => {
          if let Some(window) = &self.window {
            set_corner_radius(window, radius);
          }
          tree.window().set_corner_radius(radius);
        }
        WindowCommand::Move { x, y } => {
          if let Some(window) = &self.window {
            window.set_outer_position(PhysicalPosition::new(x, y));
          }
          tree.set_window_position(x, y);
        }
        WindowCommand::Resize(request) => {
          tree.window().apply_resize(self.window.as_ref(), request);
        }
        WindowCommand::StartDrag => {
          if self.window.as_ref().is_some_and(start_native_window_drag) {
            let (x, y) = (self.cursor_pos.0 as f32, self.cursor_pos.1 as f32);
            tree.mouse_press_taken_by_os(x, y, MouseButton::Left);
          }
        }
        WindowCommand::StartResize(direction) => {
          if self
            .window
            .as_ref()
            .is_some_and(|window| start_native_window_resize(window, direction))
          {
            let (x, y) = (self.cursor_pos.0 as f32, self.cursor_pos.1 as f32);
            tree.mouse_press_taken_by_os(x, y, MouseButton::Left);
          }
        }
        WindowCommand::StopDrag => {}
        WindowCommand::ShowSystemMenu => {
          if let Some(window) = &self.window {
            let (x, y) = self.cursor_pos;
            window.show_window_menu(PhysicalPosition::new(x, y));
          }
        }
        WindowCommand::InjectInput(input) => {
          crate::app::synthetic_input::apply(tree, &input);
        }
        #[cfg(feature = "clipboard")]
        WindowCommand::PasteFromClipboard { synthetic } => {
          tree.paste_from_os_clipboard(synthetic);
        }
        #[cfg(feature = "screenshot")]
        WindowCommand::Screenshot(output_path, region) => {
          tree.request_screenshot(output_path, region);
        }
        WindowCommand::OpenDevtools => {
          tree.open_devtools();
        }
        WindowCommand::CloseDevtools => {
          tree.close_devtools();
        }
        WindowCommand::ToggleDevtools => {
          tree.toggle_devtools();
        }
      }
    }
    self.sync_window_state(tree);
    closed
  }

  fn check_redraw(&mut self, tree: &Tree) {
    if tree.needs_redraw() {
      self.request_redraw();
    }
    if let Some(window) = &self.window {
      self.ime.apply(window, tree);
      #[cfg(target_os = "macos")]
      self.native_controls.apply(window, tree);
    }
  }

  fn request_redraw(&mut self) {
    if let Some(window) = &self.window {
      self.frames.request(window);
    }
  }

  fn present_now(&mut self, app: &mut App, tree: &mut Tree) -> bool {
    if let Some(w) = &self.window {
      let size = surface_size(w);
      tree.set_scale_factor(w.scale_factor() as f32);
      tree.resize(size.width, size.height);
      // The tree is as large as the window, and is told what of that is clear of the screen's edges: the
      // window's own, as it is in a scene of its own. No keyboard comes up over it: nothing in it is typed
      // into.
      #[cfg(target_os = "ios")]
      if let Ok(RawWindowHandle::UiKit(handle)) = w.window_handle().map(|handle| handle.as_raw()) {
        let (top, right, bottom, left) = ios::own_safe_area(handle.ui_view);
        tree.window().set_safe_area(crate::app::window::SafeAreaInsets {
          top,
          right,
          bottom,
          left,
          keyboard: 0.0,
        });
      }
      self.frames.settle();
      let started = Instant::now();
      #[cfg(feature = "perf_profile")]
      tree.set_profiled_refresh_interval(self.frames.refresh_interval(Some(w), started));
      let presented = tree.pass(app, w).rendered;
      // See the main window's `present_now`.
      set_window_cursor(w, &mut self.cursor, tree.cursor());
      if presented {
        self.frames.presented(PresentedFrame {
          started,
          presented: Instant::now(),
        });
      }
      self.check_redraw(tree);
      return presented;
    }
    false
  }

  fn apply_cursor(&mut self, tree: &Tree) {
    match &self.window {
      Some(window) => set_window_cursor(window, &mut self.cursor, tree.cursor()),
      None => self.cursor = tree.cursor(),
    }
  }

  fn tick(&mut self, tree: &mut Tree) {
    let now = Instant::now();
    tree.tick_timers();
    tree.tick_futures();
    self.tick_touch(tree, now);
    self.file_drag.flush(tree, self.window.as_ref(), self.cursor_pos);
    let window = self.window.as_ref();
    self.frames.request_timeline_frame(tree, window, now);
    tree.tick_scheduled_redraw(now);
    self.check_redraw(tree);
  }

  fn handle_event(
    &mut self,
    app: &mut App,
    _event_loop: &ActiveEventLoop,
    event: WindowEvent,
    tree: &mut Tree,
  ) -> bool {
    #[cfg(feature = "perf_profile")]
    stamp_profiled_input(tree, &event);
    tree.set_app_ref(app);
    self.frames.handle_event(&event);
    if self.file_drag.on_event(&event) {
      return false;
    }
    match event {
      WindowEvent::CloseRequested => {
        tree.window().dispatch_close_request(CloseRequestSource::Os);
        self.request_redraw();
      }
      WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
        tree.set_scale_factor(scale_factor as f32);
        if let Some(w) = &self.window {
          let size = surface_size(w);
          tree.resize(size.width, size.height);
          self.request_redraw();
        }
      }
      WindowEvent::Resized(size) => {
        // On iOS the event's size is the safe area's, and the tree is as large as the window.
        #[cfg(target_os = "ios")]
        let size = self.window.as_ref().map_or(size, surface_size);
        tree.resize(size.width, size.height);
        self.sync_window_state(tree);
        // Paint inside the event, as the main window does, so a live resize is not stretched.
        let current = self.window.as_ref().is_some_and(|w| surface_size(w) == size);
        if !current || size.width == 0 || size.height == 0 || !self.present_now(app, tree) {
          self.request_redraw();
        }
      }
      WindowEvent::Focused(focused) => {
        tree.window().set_focused(focused);
        if focused {
          sync_system_motion(app);
        }
        self.request_redraw();
      }
      WindowEvent::Moved(position) => {
        tree.set_window_position(position.x, position.y);
      }
      WindowEvent::CursorMoved { position, .. } => {
        self.cursor_pos = (position.x, position.y);
        tree.mouse_move_with_meta(
          position.x as f32,
          position.y as f32,
          self.modifiers.shift_key(),
          self.modifiers.control_key(),
          self.modifiers.alt_key(),
          self.modifiers.super_key(),
        );
        self.apply_cursor(tree);
        self.check_redraw(tree);
      }
      WindowEvent::CursorLeft { .. } => {
        tree.mouse_leave_window();
        self.apply_cursor(tree);
        self.check_redraw(tree);
      }
      WindowEvent::MouseInput { state, button, .. } => {
        let btn = to_lurq_mouse_button(button);
        let (x, y) = (self.cursor_pos.0 as f32, self.cursor_pos.1 as f32);
        match state {
          ElementState::Pressed => tree.mouse_down_with_meta(
            x,
            y,
            btn,
            self.modifiers.shift_key(),
            self.modifiers.control_key(),
            self.modifiers.alt_key(),
            self.modifiers.super_key(),
          ),
          ElementState::Released => tree.mouse_up_with_meta(
            x,
            y,
            btn,
            self.modifiers.shift_key(),
            self.modifiers.control_key(),
            self.modifiers.alt_key(),
            self.modifiers.super_key(),
          ),
        }
        self.apply_window_commands(tree);
        self.apply_cursor(tree);
        self.check_redraw(tree);
        return false;
      }
      WindowEvent::ModifiersChanged(modifiers) => {
        self.modifiers = modifiers.state();
      }
      WindowEvent::Ime(ime) => {
        tree.ime(ime::to_ime_event(ime));
        self.check_redraw(tree);
      }
      WindowEvent::KeyboardInput { event, .. } => {
        match event.state {
          ElementState::Pressed => tree.key_down_with_meta(
            key_to_string(&event),
            physical_key_to_string(&event.physical_key),
            self.modifiers.shift_key(),
            self.modifiers.control_key(),
            self.modifiers.alt_key(),
            self.modifiers.super_key(),
          ),
          ElementState::Released => tree.key_up_with_meta(
            key_to_string(&event),
            physical_key_to_string(&event.physical_key),
            self.modifiers.shift_key(),
            self.modifiers.control_key(),
            self.modifiers.alt_key(),
            self.modifiers.super_key(),
          ),
        }
        self.apply_cursor(tree);
        self.check_redraw(tree);
      }
      WindowEvent::MouseWheel { delta, phase, .. } => {
        let (mut dx, mut dy) = match delta {
          MouseScrollDelta::LineDelta(x, y) => (x * WHEEL_LINE, y * WHEEL_LINE),
          MouseScrollDelta::PixelDelta(p) => (p.x as f32, p.y as f32),
        };
        if self.modifiers.shift_key() && dx == 0.0 {
          dx = dy;
          dy = 0.0;
        }
        let scroll_phase = match phase {
          TouchPhase::Started => ScrollPhase::Start,
          TouchPhase::Moved => ScrollPhase::Scroll,
          TouchPhase::Ended | TouchPhase::Cancelled => ScrollPhase::End,
        };
        if phase == TouchPhase::Started {
          tree.set_history_swipes(history_swipes::enabled());
        }
        let (x, y) = (self.cursor_pos.0 as f32, self.cursor_pos.1 as f32);
        // A mouse's notches are animated; a touchpad's travel moves the content with the fingers.
        let m = self.modifiers;
        let keys = (m.shift_key(), m.control_key(), m.alt_key(), m.super_key());
        if notched_wheel::is_notched(delta) {
          tree.wheel_with_meta(x, y, dx, dy, scroll_phase, keys.0, keys.1, keys.2, keys.3);
        } else {
          tree.scroll_with_meta(x, y, dx, dy, scroll_phase, keys.0, keys.1, keys.2, keys.3);
        }
        self.apply_cursor(tree);
        self.check_redraw(tree);
      }
      WindowEvent::PinchGesture { .. } | WindowEvent::RotationGesture { .. } | WindowEvent::DoubleTapGesture { .. } => {
        trackpad_gestures::deliver(tree, &event, self.cursor_pos);
        self.apply_cursor(tree);
        self.check_redraw(tree);
      }
      WindowEvent::Touch(touch) => self.handle_touch(tree, touch),
      WindowEvent::RedrawRequested => {
        let presented = self.present_now(app, tree);
        self.apply_window_commands(tree);
        return presented;
      }
      _ => {}
    }
    self.apply_window_commands(tree);
    self.check_redraw(tree);
    false
  }
}

/// A touch on a window, as winit gives it, to the tree's model of a finger ([`Tree::finger`]): the finger's own
/// events first, to what it landed on; then, where it is the tree's, the pointer it stands for. Tells whose the
/// finger was.
fn deliver_touch(
  tree: &mut Tree,
  cursor_pos: &mut (f64, f64),
  touch: &winit::event::Touch,
  now: Instant,
) -> crate::app::runtime::TouchRoute {
  let phase = match touch.phase {
    TouchPhase::Started => crate::app::events::TouchPhase::Start,
    TouchPhase::Moved => crate::app::events::TouchPhase::Move,
    TouchPhase::Ended => crate::app::events::TouchPhase::End,
    TouchPhase::Cancelled => crate::app::events::TouchPhase::Cancel,
  };
  let force = touch.force.map(|force| force.normalized() as f32);
  let (x, y) = (touch.location.x, touch.location.y);
  // A pen in range that touches nothing: Windows tells its moves as a touch's that never started. It hovers.
  if phase == crate::app::events::TouchPhase::Move && is_pen(touch) && !tree.touch_is_down(touch.id) {
    tree.pen_hover(x as f32, y as f32);
    *cursor_pos = (x, y);
    return crate::app::runtime::TouchRoute::Taken;
  }
  let route = tree.finger_at(touch.id, phase, x as f32, y as f32, force, is_pen(touch), now);
  if route == crate::app::runtime::TouchRoute::Pointer {
    *cursor_pos = (x, y);
  }
  route
}

/// Whether a touch is a pen's, where winit lets it be told:
///
/// - on iOS an Apple Pencil's touch has the angle it is held at, a finger's none;
/// - on Windows a finger comes as `WM_TOUCH`, with no force, and a pen as pointer input with its pressure
///   (`Force::Normalized`), since winit registers the window for touch;
/// - elsewhere winit tells nothing of it (Android, Linux; the browser gives a pen as the mouse), and a touch is a
///   finger's.
fn is_pen(touch: &winit::event::Touch) -> bool {
  pen_by_force(touch.force)
}

#[cfg(target_os = "ios")]
fn pen_by_force(force: Option<winit::event::Force>) -> bool {
  matches!(
    force,
    Some(winit::event::Force::Calibrated {
      altitude_angle: Some(_),
      ..
    })
  )
}

#[cfg(windows)]
fn pen_by_force(force: Option<winit::event::Force>) -> bool {
  matches!(force, Some(winit::event::Force::Normalized(_)))
}

#[cfg(not(any(target_os = "ios", windows)))]
fn pen_by_force(_force: Option<winit::event::Force>) -> bool {
  false
}

/// Whether a mouse event is one Windows made from a touch: it promotes a finger's `WM_TOUCH` to mouse messages,
/// which winit passes on as the mouse's, marked with `MI_WP_SIGNATURE` in the message's extra information. The
/// finger already reached the tree as a touch, so the mouse's copy would press and click it a second time.
fn promoted_from_touch(event: &WindowEvent) -> bool {
  if !matches!(event, WindowEvent::CursorMoved { .. } | WindowEvent::MouseInput { .. }) {
    return false;
  }
  promoted_message()
}

#[cfg(windows)]
fn promoted_message() -> bool {
  /// `MI_WP_SIGNATURE`, in the high three bytes of what `GetMessageExtraInfo` gives.
  const SIGNATURE: u32 = 0xFF51_5700;
  let extra = unsafe { windows::Win32::UI::WindowsAndMessaging::GetMessageExtraInfo() };
  (extra.0 as u32) & 0xFFFF_FF00 == SIGNATURE
}

#[cfg(not(windows))]
fn promoted_message() -> bool {
  false
}

fn secondary_window_metadata(window: &Window) -> SecondaryWindowMetadata {
  let raw_window = window.window_handle().ok().map(|handle| handle.as_raw());
  let raw_display = window.display_handle().ok().map(|handle| handle.as_raw());
  let hwnd = raw_window.and_then(|handle| match handle {
    RawWindowHandle::Win32(handle) => Some(handle.hwnd.get()),
    _ => None,
  });

  SecondaryWindowMetadata {
    window_id: Some(format!("{:?}", window.id())),
    raw_window_handle: raw_window.map(|handle| format!("{handle:?}")),
    raw_display_handle: raw_display.map(|handle| format!("{handle:?}")),
    hwnd,
  }
}

struct WinitHandler {
  app: App,
  main: ManagedWindow,
  secondaries: Vec<ManagedSecondaryWindow>,
  loop_cadence: WinitLoopCadence,
  /// Cloned into every tree's `Window` so cross-thread command pushes wake
  /// the loop.
  window_waker: crate::app::window::WindowWaker,
}

struct WinitLoopCadence {
  started_at: Instant,
  ticks: u32,
  direct_presents: u32,
  redraw_requests: u32,
  poll_requests: u32,
  wait_until_requests: u32,
  wait_requests: u32,
  continuous_ticks: u32,
  redraw_pending_ticks: u32,
  last_tick_at: Option<Instant>,
  max_tick_delta: Duration,
}

fn duration_ms(duration: Duration) -> f64 {
  duration.as_secs_f64() * 1000.0
}

impl WinitLoopCadence {
  fn new(now: Instant) -> Self {
    Self {
      started_at: now,
      ticks: 0,
      direct_presents: 0,
      redraw_requests: 0,
      poll_requests: 0,
      wait_until_requests: 0,
      wait_requests: 0,
      continuous_ticks: 0,
      redraw_pending_ticks: 0,
      last_tick_at: None,
      max_tick_delta: Duration::ZERO,
    }
  }

  fn record(
    &mut self,
    now: Instant,
    has_continuous_redraw_image: bool,
    redraw_pending: bool,
    direct_presented: bool,
    redraw_requested: bool,
    control_flow: &'static str,
  ) {
    if let Some(last_tick_at) = self.last_tick_at {
      self.max_tick_delta = self.max_tick_delta.max(now.saturating_duration_since(last_tick_at));
    }
    self.last_tick_at = Some(now);
    self.ticks += 1;
    self.direct_presents += u32::from(direct_presented);
    self.redraw_requests += u32::from(redraw_requested);
    self.continuous_ticks += u32::from(has_continuous_redraw_image);
    self.redraw_pending_ticks += u32::from(redraw_pending);
    match control_flow {
      "poll" => self.poll_requests += 1,
      "wait_until" => self.wait_until_requests += 1,
      _ => self.wait_requests += 1,
    }

    if now.saturating_duration_since(self.started_at) < Duration::from_secs(1) {
      return;
    }

    crate::app::profile_support::video_log!(debug,
      target: "video::watch::lurq",
      "winit loop cadence ticks={} max_tick_delta_ms={:.2} continuous_ticks={} redraw_pending_ticks={} direct_presents={} redraw_requests={} poll={} wait_until={} wait={}",
      self.ticks,
      duration_ms(self.max_tick_delta),
      self.continuous_ticks,
      self.redraw_pending_ticks,
      self.direct_presents,
      self.redraw_requests,
      self.poll_requests,
      self.wait_until_requests,
      self.wait_requests
    );

    *self = Self::new(now);
  }
}

struct WinitSlowScope {
  name: &'static str,
  detail: &'static str,
  started_at: Instant,
}

impl WinitSlowScope {
  fn new(name: &'static str, detail: &'static str) -> Self {
    Self {
      name,
      detail,
      started_at: Instant::now(),
    }
  }
}

impl Drop for WinitSlowScope {
  fn drop(&mut self) {
    let elapsed = self.started_at.elapsed();
    if elapsed < WINIT_SLOW_SCOPE_THRESHOLD {
      return;
    }

    crate::app::profile_support::video_log!(debug,
      target: "video::watch::lurq",
      "winit slow scope name={} detail={} elapsed_ms={:.2}",
      self.name,
      self.detail,
      duration_ms(elapsed)
    );
  }
}

impl WinitHandler {
  /// Tells the window and the theme which appearance the system is in.
  fn follow_appearance(&mut self, theme: winit::window::Theme) {
    let mut system = self.main.tree.window().info().system;
    system.appearance = match theme {
      winit::window::Theme::Light => crate::app::window::Appearance::Light,
      winit::window::Theme::Dark => crate::app::window::Appearance::Dark,
    };
    self.main.tree.window().set_system_settings(system);
    self.app.theme().apply_system(&system);
    self.main.tree.request_redraw();
  }

  fn close_secondary_at_position(&mut self, position: usize) {
    let index = self.secondaries[position].index();
    #[cfg(target_os = "ios")]
    self.secondaries[position].leave_scene();
    #[cfg(windows)]
    if let Some(secondary) = self.main.tree.secondary_window(index) {
      secondary.tree().window().set_parent(None);
    }
    if self.main.tree.close_secondary_window(index) {
      self.main.request_redraw();
    }
    self.secondaries.remove(position);
  }

  fn sync_secondary_windows(&mut self, event_loop: &ActiveEventLoop) {
    self
      .secondaries
      .retain(|secondary| self.main.tree.secondary_window(secondary.index()).is_some());

    let count = self.main.tree.secondary_window_count();
    for index in 0..count {
      if self.secondaries.iter().any(|secondary| secondary.index() == index) {
        continue;
      }

      let Some(secondary) = self.main.tree.secondary_window(index) else {
        continue;
      };
      secondary.tree().window().attach_shell(self.window_waker.clone());
      let mut managed = ManagedSecondaryWindow::new(index, secondary);
      self.main.tree.ensure_secondary_window_render_engine(index);
      let metadata = self
        .main
        .tree
        .secondary_window_mut(index)
        .and_then(|secondary| managed.create_window(event_loop, &mut self.app, secondary.tree_mut()));
      if let Some(metadata) = metadata {
        self.main.tree.set_secondary_window_metadata(index, metadata);
      }
      self.secondaries.push(managed);
    }
  }

  fn check_secondary_redraw(&mut self) {
    for secondary_window in &mut self.secondaries {
      let index = secondary_window.index();
      if let Some(secondary) = self.main.tree.secondary_window(index) {
        secondary_window.check_redraw(secondary.tree());
      }
    }
  }

  /// Presents each window an MCP call delivered held input to (a press, a move while held), as a real mouse-move
  /// stream presents from inside its events, so each call lands in exactly one pass of its own.
  #[cfg(feature = "mcp")]
  fn present_mcp_input(&mut self, event_loop: &ActiveEventLoop) {
    if self.main.tree.take_mcp_input_present()
      && self.main.tree.needs_redraw()
      && self.main.present_now(&mut self.app, false)
    {
      self.main.apply_window_commands(event_loop);
    }
    for position in 0..self.secondaries.len() {
      let index = self.secondaries[position].index();
      let Some(secondary) = self.main.tree.secondary_window_mut(index) else {
        continue;
      };
      let tree = secondary.tree_mut();
      if tree.take_mcp_input_present() && tree.needs_redraw() {
        self.secondaries[position].present_now(&mut self.app, tree);
      }
    }
  }

  fn present_dirty_secondaries(&mut self) {
    // A window that is drawn while the main one is not may be the one that finds the device lost.
    self.main.tree.replace_lost_render_engine();
    for position in 0..self.secondaries.len() {
      let index = self.secondaries[position].index();
      let should_present = self
        .main
        .tree
        .secondary_window(index)
        .is_some_and(|secondary| secondary.tree().needs_redraw());
      if !should_present {
        continue;
      }

      let secondary_window = &mut self.secondaries[position];
      if let Some(secondary) = self.main.tree.secondary_window_mut(index) {
        secondary_window.present_now(&mut self.app, secondary.tree_mut());
      }
    }
  }

  fn apply_secondary_window_requests(&mut self, event_loop: &ActiveEventLoop) {
    if self.main.tree.apply_secondary_window_requests(&mut self.app) {
      self.main.request_redraw();
    }
    self.sync_secondary_windows(event_loop);
    self.check_secondary_redraw();
  }

  fn handle_secondary_pick_event(&mut self, event: &WindowEvent) -> bool {
    if !self.main.tree.secondary_pick_mode() {
      return false;
    }

    match event {
      WindowEvent::CursorMoved { position, .. } => {
        self.main.cursor_pos = (position.x, position.y);
        true
      }
      WindowEvent::MouseInput { state, button, .. } => {
        if *button != winit::event::MouseButton::Left {
          return true;
        }

        if matches!(state, ElementState::Pressed) {
          return true;
        }

        let (x, y) = (self.main.cursor_pos.0 as f32, self.main.cursor_pos.1 as f32);
        self.main.tree.pick_secondary_node_at(x, y);
        self.main.request_redraw();
        self.check_secondary_redraw();
        true
      }
      WindowEvent::KeyboardInput { event, .. }
        if matches!(event.state, ElementState::Pressed)
          && matches!(event.logical_key, Key::Named(NamedKey::Escape)) =>
      {
        self.main.tree.cancel_secondary_pick();
        self.check_secondary_redraw();
        true
      }
      _ => false,
    }
  }
}

/// Stamps an input event the tree's dispatch profiles as received now, so its
/// sample has a queue delay; any other event clears the stamp. winit gives no
/// OS timestamp, so this is when the shell's handler got the event.
#[cfg(feature = "perf_profile")]
fn stamp_profiled_input(tree: &mut Tree, event: &WindowEvent) {
  let input = matches!(
    event,
    WindowEvent::CursorMoved { .. }
      | WindowEvent::CursorLeft { .. }
      | WindowEvent::MouseInput { .. }
      | WindowEvent::KeyboardInput { .. }
      | WindowEvent::MouseWheel { .. }
  );
  tree.stamp_profiled_input(input.then(Instant::now));
}

/// Gives the theme the system's reduce-motion setting, where the platform has one lurq reads.
fn sync_system_motion(app: &App) {
  if let Some(reduced) = crate::app::theme::system_reduced_motion() {
    app.theme().set_system_reduced_motion(reduced);
  }
}

fn window_event_name(event: &WindowEvent) -> &'static str {
  match event {
    WindowEvent::ActivationTokenDone { .. } => "ActivationTokenDone",
    WindowEvent::Resized(_) => "Resized",
    WindowEvent::Moved(_) => "Moved",
    WindowEvent::CloseRequested => "CloseRequested",
    WindowEvent::Destroyed => "Destroyed",
    WindowEvent::DroppedFile(_) => "DroppedFile",
    WindowEvent::HoveredFile(_) => "HoveredFile",
    WindowEvent::HoveredFileCancelled => "HoveredFileCancelled",
    WindowEvent::Focused(_) => "Focused",
    WindowEvent::KeyboardInput { .. } => "KeyboardInput",
    WindowEvent::ModifiersChanged(_) => "ModifiersChanged",
    WindowEvent::Ime(_) => "Ime",
    WindowEvent::CursorMoved { .. } => "CursorMoved",
    WindowEvent::CursorEntered { .. } => "CursorEntered",
    WindowEvent::CursorLeft { .. } => "CursorLeft",
    WindowEvent::MouseWheel { .. } => "MouseWheel",
    WindowEvent::MouseInput { .. } => "MouseInput",
    WindowEvent::PinchGesture { .. } => "PinchGesture",
    WindowEvent::PanGesture { .. } => "PanGesture",
    WindowEvent::DoubleTapGesture { .. } => "DoubleTapGesture",
    WindowEvent::RotationGesture { .. } => "RotationGesture",
    WindowEvent::TouchpadPressure { .. } => "TouchpadPressure",
    WindowEvent::AxisMotion { .. } => "AxisMotion",
    WindowEvent::Touch(_) => "Touch",
    WindowEvent::ScaleFactorChanged { .. } => "ScaleFactorChanged",
    WindowEvent::ThemeChanged(_) => "ThemeChanged",
    WindowEvent::Occluded(_) => "Occluded",
    WindowEvent::RedrawRequested => "RedrawRequested",
  }
}

impl ApplicationHandler for WinitHandler {
  // A page calls into the app between the loop's turns as well as in them; see `Window::off_loop_waker`.
  #[cfg(target_arch = "wasm32")]
  fn new_events(&mut self, _event_loop: &ActiveEventLoop, _cause: winit::event::StartCause) {
    crate::app::window::set_in_loop_turn(true);
  }

  fn resumed(&mut self, event_loop: &ActiveEventLoop) {
    self.main.create_window(event_loop, &mut self.app);
    #[cfg(not(target_os = "ios"))]
    if let Some(theme) = self.main.window.as_ref().and_then(|window| window.theme()) {
      self.follow_appearance(theme);
    }
    #[cfg(target_os = "macos")]
    crate::app::macos_menu::sync();
    self.sync_secondary_windows(event_loop);
  }

  fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
    // A finger's touch reached the tree as a touch: the mouse's copy of it Windows makes is dropped.
    if promoted_from_touch(&event) {
      return;
    }
    let event_name = window_event_name(&event);
    let _slow_scope = WinitSlowScope::new("window_event", event_name);
    if self.main.window_id() == Some(id) {
      // The system's appearance, where winit tells it: a desktop.
      if let WindowEvent::ThemeChanged(theme) = &event {
        self.follow_appearance(*theme);
      }
      if matches!(event, WindowEvent::RedrawRequested) && self.main.has_continuous_redraw_video_recently(Instant::now())
      {
        self.main.frames.settle();
        return;
      }
      let started_at = Instant::now();
      let pick_started_at = Instant::now();
      if self.handle_secondary_pick_event(&event) {
        let pick = pick_started_at.elapsed();
        let total = started_at.elapsed();
        if total >= WINIT_BREAKDOWN_THRESHOLD {
          crate::app::profile_support::video_log!(debug,
            target: "video::watch::lurq",
            "winit window_event breakdown event={} total_ms={:.2} pick_ms={:.2} handle_ms=0.00 secondary_requests_ms=0.00 post_present_ms=0.00 presented=false picked=true",
            event_name,
            duration_ms(total),
            duration_ms(pick)
          );
        }
        return;
      }
      let pick = pick_started_at.elapsed();
      let handle_started_at = Instant::now();
      let presented = self.main.handle_event(&mut self.app, event_loop, event);
      let handle = handle_started_at.elapsed();
      let requests_started_at = Instant::now();
      self.apply_secondary_window_requests(event_loop);
      let requests = requests_started_at.elapsed();
      let post_present_started_at = Instant::now();
      if presented {
        self.check_secondary_redraw();
        self.present_dirty_secondaries();
      }
      let post_present = post_present_started_at.elapsed();
      let total = started_at.elapsed();
      if total >= WINIT_BREAKDOWN_THRESHOLD {
        crate::app::profile_support::video_log!(debug,
          target: "video::watch::lurq",
          "winit window_event breakdown event={} total_ms={:.2} pick_ms={:.2} handle_ms={:.2} secondary_requests_ms={:.2} post_present_ms={:.2} presented={} picked=false",
          event_name,
          duration_ms(total),
          duration_ms(pick),
          duration_ms(handle),
          duration_ms(requests),
          duration_ms(post_present),
          presented
        );
      }
      return;
    }

    if let Some(position) = self
      .secondaries
      .iter()
      .position(|secondary| secondary.window_id() == Some(id))
    {
      if is_open_devtools_window_event(&event, self.secondaries[position].modifiers) {
        self.main.tree.open_devtools();
        self.apply_secondary_window_requests(event_loop);
        return;
      }
      let index = self.secondaries[position].index();
      let mut closed = false;
      #[cfg(target_os = "ios")]
      let drawn = matches!(event, WindowEvent::RedrawRequested);
      // As in `present_dirty_secondaries`: the window may be asked to draw by the system.
      if matches!(event, WindowEvent::RedrawRequested) {
        self.main.tree.replace_lost_render_engine();
      }
      if let Some(secondary) = self.main.tree.secondary_window_mut(index) {
        self.secondaries[position].handle_event(&mut self.app, event_loop, event, secondary.tree_mut());
        closed = self.secondaries[position].close_requested();
      }
      if closed {
        self.close_secondary_at_position(position);
      } else {
        self.apply_secondary_window_requests(event_loop);
        // UIKit draws a view from `drawRect:`, which is where winit sends the redraw from, and drops a redraw
        // asked for inside it: the frame the pass asked for is taken as not coming, and the loop takes another
        // turn and asks from there, as for the main window.
        #[cfg(target_os = "ios")]
        if drawn
          && let Some(secondary_window) = self.secondaries.iter_mut().find(|secondary| secondary.index() == index)
        {
          secondary_window.frames.settle();
          if self
            .main
            .tree
            .secondary_window(index)
            .is_some_and(|secondary| secondary.tree().needs_redraw())
          {
            event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
          }
        }
      }
    }
  }

  fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
    #[cfg(target_arch = "wasm32")]
    let _turn = web::TurnEnd;
    let _slow_scope = WinitSlowScope::new("about_to_wait", "main");
    let started_at = Instant::now();
    let stage_started_at = Instant::now();
    #[cfg(target_os = "macos")]
    crate::app::macos_menu::sync();
    // An item of the menu is the window's in front: an item of editing runs on its focus.
    let in_front = self
      .secondaries
      .iter()
      .filter_map(|secondary| self.main.tree.secondary_window(secondary.index()))
      .map(|secondary| secondary.tree().window().clone())
      .find(|window| window.info().is_focused)
      .unwrap_or_else(|| self.main.tree.window().clone());
    self.app.shared.menu.drain_to(self.main.tree.window(), &in_front);
    // What an item asked for (an edit, a way back) runs in the window's next pass: the pass is asked for now, not
    // left to the caret's next blink.
    if in_front.has_requests() {
      self.main.check_redraw();
      self.check_secondary_redraw();
    }
    // What a paste the browser has answered holds; see `clipboard::web`.
    #[cfg(all(target_arch = "wasm32", feature = "clipboard"))]
    if let Some(pasted) = crate::clipboard::web::take_pasted() {
      // What this page copied with its Markdown, where the clipboard still holds what it wrote: the plain text,
      // and the Markdown beside it.
      use crate::app::events::ClipboardContent;
      let mut content = match pasted.text {
        Some(text) => match crate::app::runtime::copied_rich(&text) {
          Some((plain, markdown)) => ClipboardContent::text(plain).with_markdown(markdown),
          None => ClipboardContent::text(text),
        },
        None => ClipboardContent::empty(),
      };
      if let Some(image) = pasted.image {
        content = content.with_image(image);
      }
      self.main.tree.paste(content);
    }
    // What came to the page as text and not as keys: see `web_text`.
    #[cfg(target_arch = "wasm32")]
    if let Some(typed) = self.main.web_text.as_ref().map(web_text::WebText::take)
      && !typed.is_empty()
    {
      web_text::give(&mut self.main.tree, typed);
      self.main.check_redraw();
    }
    // Files dragged onto the page: see `web_drop`.
    #[cfg(target_arch = "wasm32")]
    if let Some(web_drop) = &self.main.web_drop
      && web_drop.give(&mut self.main.tree)
    {
      self.main.check_redraw();
    }
    self.main.tick();
    // What the person set for the whole system, read again each turn: it is a few calls, and the loop turns
    // when the app comes back from the settings or from the control centre, where it is changed.
    #[cfg(target_os = "ios")]
    if let Some(window) = self.main.window.as_ref()
      && let Ok(RawWindowHandle::UiKit(handle)) = window.window_handle().map(|handle| handle.as_raw())
    {
      let system = ios::system_settings(handle.ui_view);
      if self.main.tree.window().info().system != system {
        self.main.tree.window().set_system_settings(system);
        self.app.theme().apply_system(&system);
        self.main.tree.request_redraw();
      }
    }
    let tick = stage_started_at.elapsed();
    let stage_started_at = Instant::now();
    self.main.apply_window_commands(event_loop);
    #[cfg(feature = "mcp")]
    if self.main.tree.drain_mcp_requests(&mut self.app) {
      // Tool handlers ran against the trees; pick up any redraws or window
      // commands they produced.
      self.main.apply_window_commands(event_loop);
      // Before any redraw is requested: a requested redraw would paint the same input a second time.
      self.present_mcp_input(event_loop);
      self.main.check_redraw();
      self.check_secondary_redraw();
    }
    let main_commands = stage_started_at.elapsed();
    let stage_started_at = Instant::now();
    self.apply_secondary_window_requests(event_loop);
    let secondary_requests = stage_started_at.elapsed();
    let mut direct_presented = false;
    let stage_started_at = Instant::now();
    let now = Instant::now();
    let main_has_continuous_video = self.main.has_continuous_redraw_video_recently(now);
    // Interactive drags (scrollbars, sliders, text selection, on_drag_*
    // sessions) present directly too: on Windows the WM_MOUSEMOVE stream
    // starves WM_PAINT, so redraws requested from drag handlers would only
    // land once the mouse pauses. Vsync in present caps the rate.
    let main_has_interactive_drag = self.main.tree.has_active_input_interaction() && self.main.tree.needs_redraw();
    let continuous_check = stage_started_at.elapsed();
    let mut present = Duration::ZERO;
    let mut post_present = Duration::ZERO;
    let main_frame_due = self.main.take_frame_due();
    if main_has_continuous_video || main_has_interactive_drag || main_frame_due {
      let stage_started_at = Instant::now();
      let presented = self.main.present_now(&mut self.app, false);
      present = stage_started_at.elapsed();
      direct_presented = presented;
      let stage_started_at = Instant::now();
      self.apply_secondary_window_requests(event_loop);
      if presented {
        self.check_secondary_redraw();
        self.present_dirty_secondaries();
      }
      post_present = stage_started_at.elapsed();
    }
    let stage_started_at = Instant::now();
    let mut position = 0;
    while position < self.secondaries.len() {
      let secondary_index = self.secondaries[position].index();
      let mut closed = false;
      if let Some(secondary) = self.main.tree.secondary_window_mut(secondary_index) {
        let secondary_window = &mut self.secondaries[position];
        secondary_window.tick(secondary.tree_mut());
        closed = secondary_window.apply_window_commands(secondary.tree_mut());
        // The person closed the window's scene, or its display was unplugged.
        #[cfg(target_os = "ios")]
        {
          closed |= secondary_window.scene_closed();
        }
      }

      if closed {
        self.close_secondary_at_position(position);
      } else {
        position += 1;
      }
    }
    let secondary_tick = stage_started_at.elapsed();
    let stage_started_at = Instant::now();
    self.sync_secondary_windows(event_loop);
    let sync_secondary = stage_started_at.elapsed();

    let stage_started_at = Instant::now();
    let secondary_needs_polling = self.secondaries.iter().any(|secondary_window| {
      self
        .main
        .tree
        .secondary_window(secondary_window.index())
        .is_some_and(|secondary| secondary.tree().needs_polling())
        || secondary_window.needs_polling()
    });
    let secondary_has_continuous_video = self.secondaries.iter().any(|secondary_window| {
      self
        .main
        .tree
        .secondary_window(secondary_window.index())
        .is_some_and(|secondary| secondary.tree().has_continuous_redraw_video())
    });
    let has_continuous_video = main_has_continuous_video || secondary_has_continuous_video;
    let redraw_pending =
      self.main.frames.pending() || self.secondaries.iter().any(|secondary| secondary.frames.pending());
    let redraw_requested = self.main.tree.needs_redraw()
      || self.secondaries.iter().any(|secondary_window| {
        self
          .main
          .tree
          .secondary_window(secondary_window.index())
          .is_some_and(|secondary| secondary.tree().needs_redraw())
      });
    let trees = || {
      self
        .secondaries
        .iter()
        .filter_map(|secondary_window| self.main.tree.secondary_window(secondary_window.index()))
        .map(|secondary| secondary.tree())
        .chain(std::iter::once(&*self.main.tree))
    };
    let next_redraw = trees().filter_map(Tree::next_scheduled_redraw).min();
    // A file drag hovering over a window reads the pointer at its poll
    // interval, like a timer.
    let next_file_drag_poll = self
      .secondaries
      .iter()
      .filter_map(ManagedSecondaryWindow::next_file_drag_poll)
      .chain(self.main.next_file_drag_poll())
      .min();
    let next_secondary_touch = self
      .secondaries
      .iter()
      .filter_map(|secondary_window| {
        let secondary = self.main.tree.secondary_window(secondary_window.index())?;
        secondary_window.next_touch_deadline(secondary.tree())
      })
      .min();
    let next_timer = trees()
      .filter_map(Tree::next_timer_at)
      .chain(next_file_drag_poll)
      .chain(self.main.next_touch_deadline())
      .chain(next_secondary_touch)
      .min();

    let wait = LoopWait::of(
      LoopNeeds {
        continuous_video: has_continuous_video,
        redraw_pending,
        polling: self.main.needs_polling() || secondary_needs_polling,
        next_redraw,
        next_timer,
      },
      now,
    );
    event_loop.set_control_flow(wait.control_flow());
    let control_flow = wait.name();
    self.loop_cadence.record(
      now,
      has_continuous_video,
      redraw_pending,
      direct_presented,
      redraw_requested,
      control_flow,
    );
    let control_flow_duration = stage_started_at.elapsed();
    let total = started_at.elapsed();
    if total >= WINIT_BREAKDOWN_THRESHOLD {
      crate::app::profile_support::video_log!(debug,
        target: "video::watch::lurq",
        "winit about_to_wait breakdown total_ms={:.2} tick_ms={:.2} main_commands_ms={:.2} secondary_requests_ms={:.2} continuous_check_ms={:.2} present_ms={:.2} post_present_ms={:.2} secondary_tick_ms={:.2} sync_secondary_ms={:.2} control_flow_ms={:.2} main_video={} secondary_video={} direct_presented={} redraw_pending={} redraw_requested={} control_flow={}",
        duration_ms(total),
        duration_ms(tick),
        duration_ms(main_commands),
        duration_ms(secondary_requests),
        duration_ms(continuous_check),
        duration_ms(present),
        duration_ms(post_present),
        duration_ms(secondary_tick),
        duration_ms(sync_secondary),
        duration_ms(control_flow_duration),
        main_has_continuous_video,
        secondary_has_continuous_video,
        direct_presented,
        redraw_pending,
        redraw_requested,
        control_flow
      );
    }
  }
}

/// The size the tree is laid out and drawn at. On iOS that is the whole window, which covers the screen: winit's
/// `inner_size` is the safe area there, smaller than the view the surface fills, and a frame of that size was
/// stretched over the screen. The app keeps its content inside the safe area with
/// [`WindowInfo::safe_area`](crate::app::window::WindowInfo::safe_area).
fn surface_size(window: &Window) -> winit::dpi::PhysicalSize<u32> {
  #[cfg(target_os = "ios")]
  return window.outer_size();
  // A canvas's size reaches winit from an observer: until it has run, the size is read from the page. The
  // observer's is taken when it agrees with the page's: it is the exact count of the canvas's pixels, where the
  // page's is its CSS size times the ratio, rounded. It does not always agree: Chromium's device emulation tells
  // the CSS size as the pixels', and a surface that size is stretched over the canvas, everything 1.5 times too
  // large at a ratio of 1.5.
  #[cfg(target_arch = "wasm32")]
  {
    let size = window.inner_size();
    return match web::canvas_size(window) {
      Some(page) if web::size_disagrees(size, page) => page,
      Some(page) if size.width == 0 || size.height == 0 => page,
      _ => size,
    };
  }
  #[cfg(not(any(target_os = "ios", target_arch = "wasm32")))]
  window.inner_size()
}

/// The safe area's insets in logical pixels, as UIKit has them for the window's view.
#[cfg(target_os = "ios")]
fn safe_area_insets(window: &Window) -> crate::app::window::SafeAreaInsets {
  let Ok(RawWindowHandle::UiKit(handle)) = window.window_handle().map(|handle| handle.as_raw()) else {
    return Default::default();
  };
  let (top, right, bottom, left) = ios::safe_area(handle.ui_view);
  crate::app::window::SafeAreaInsets {
    top,
    right,
    bottom,
    left,
    keyboard: 0.0,
  }
}

fn to_lurq_mouse_button(button: winit::event::MouseButton) -> MouseButton {
  match button {
    winit::event::MouseButton::Left => MouseButton::Left,
    winit::event::MouseButton::Right => MouseButton::Right,
    winit::event::MouseButton::Middle => MouseButton::Middle,
    winit::event::MouseButton::Back => MouseButton::Back,
    winit::event::MouseButton::Forward => MouseButton::Forward,
    winit::event::MouseButton::Other(id) => MouseButton::Other(id.min(u16::from(u8::MAX)) as u8),
  }
}

fn key_to_string(event: &winit::event::KeyEvent) -> String {
  match &event.logical_key {
    Key::Character(text) => event
      .text
      .as_ref()
      .map(ToString::to_string)
      .unwrap_or_else(|| text.to_string()),
    Key::Named(named) => named_key_to_string(*named).to_owned(),
    Key::Dead(Some(ch)) => ch.to_string(),
    Key::Dead(None) => String::new(),
    Key::Unidentified(_) => event.text.as_ref().map(ToString::to_string).unwrap_or_default(),
  }
}

fn is_open_devtools_shortcut(event: &winit::event::KeyEvent, modifiers: ModifiersState) -> bool {
  event.state == ElementState::Pressed
    && modifiers.control_key()
    && modifiers.shift_key()
    && !modifiers.alt_key()
    && !modifiers.super_key()
    && matches!(event.physical_key, PhysicalKey::Code(KeyCode::F12))
}

fn is_open_devtools_window_event(event: &WindowEvent, modifiers: ModifiersState) -> bool {
  matches!(event, WindowEvent::KeyboardInput { event, .. } if is_open_devtools_shortcut(event, modifiers))
}

fn named_key_to_string(key: NamedKey) -> &'static str {
  match key {
    NamedKey::Backspace => "Backspace",
    NamedKey::Delete => "Delete",
    NamedKey::ArrowLeft => "ArrowLeft",
    NamedKey::ArrowRight => "ArrowRight",
    NamedKey::ArrowUp => "ArrowUp",
    NamedKey::ArrowDown => "ArrowDown",
    NamedKey::Home => "Home",
    NamedKey::End => "End",
    NamedKey::Insert => "Insert",
    NamedKey::Enter => "Enter",
    NamedKey::Space => " ",
    // A key the input method takes (Windows `VK_PROCESSKEY`); see
    // `KeyboardEvent::composing`. Its code is the physical key, such as
    // `Enter` for the one that confirms a composition.
    NamedKey::Process => "Process",
    // An unmapped named key used to deliver `key == ""`, which silently
    // broke every app-side `event.key == "Escape"`-style match.
    NamedKey::Escape => "Escape",
    NamedKey::Tab => "Tab",
    NamedKey::PageUp => "PageUp",
    NamedKey::PageDown => "PageDown",
    NamedKey::Control => "Control",
    NamedKey::Shift => "Shift",
    NamedKey::Alt => "Alt",
    NamedKey::Super => "Super",
    NamedKey::CapsLock => "CapsLock",
    NamedKey::ContextMenu => "ContextMenu",
    NamedKey::F1 => "F1",
    NamedKey::F2 => "F2",
    NamedKey::F3 => "F3",
    NamedKey::F4 => "F4",
    NamedKey::F5 => "F5",
    NamedKey::F6 => "F6",
    NamedKey::F7 => "F7",
    NamedKey::F8 => "F8",
    NamedKey::F9 => "F9",
    NamedKey::F10 => "F10",
    NamedKey::F11 => "F11",
    NamedKey::F12 => "F12",
    // A keyboard's own keys for going back and forward, and Android's back, which winit names as a browser's:
    // `KeyboardEvent::navigation_command` reads them.
    NamedKey::BrowserBack => "BrowserBack",
    NamedKey::BrowserForward => "BrowserForward",
    _ => "",
  }
}

#[cfg(test)]
mod key_name_tests {
  use super::*;

  #[test]
  fn named_keys_apps_match_on_are_not_empty() {
    // Escape regression: an empty string here breaks every app-side
    // `event.key == "Escape"` shortcut without any visible error.
    for (key, expected) in [
      (NamedKey::Escape, "Escape"),
      (NamedKey::Tab, "Tab"),
      (NamedKey::Enter, "Enter"),
      (NamedKey::PageUp, "PageUp"),
      (NamedKey::PageDown, "PageDown"),
      (NamedKey::F5, "F5"),
      (NamedKey::Control, "Control"),
      (NamedKey::Process, "Process"),
    ] {
      assert_eq!(named_key_to_string(key), expected);
    }
  }

  #[test]
  fn androids_back_is_the_command_to_go_back() {
    use crate::{
      app::events::{KeyboardEvent, NavigationCommand},
      core::NodeId,
    };
    // winit gives Android's back (`Keycode::Back`) as the named key `BrowserBack`, with no physical key.
    let key = |named| {
      KeyboardEvent::new(
        named_key_to_string(named),
        "",
        false,
        false,
        false,
        false,
        NodeId::UNASSIGNED,
      )
    };
    assert_eq!(
      key(NamedKey::BrowserBack).navigation_command(),
      Some(NavigationCommand::Back)
    );
    assert_eq!(
      key(NamedKey::BrowserForward).navigation_command(),
      Some(NavigationCommand::Forward)
    );
  }

  #[test]
  fn the_side_buttons_are_the_mouses_back_and_forward() {
    use crate::app::events::NavigationCommand;
    assert_eq!(
      NavigationCommand::of_button(to_lurq_mouse_button(winit::event::MouseButton::Back)),
      Some(NavigationCommand::Back)
    );
    assert_eq!(
      NavigationCommand::of_button(to_lurq_mouse_button(winit::event::MouseButton::Forward)),
      Some(NavigationCommand::Forward)
    );
    assert_eq!(
      to_lurq_mouse_button(winit::event::MouseButton::Other(8)),
      MouseButton::Other(8)
    );
  }
}

fn physical_key_to_string(key: &PhysicalKey) -> String {
  match key {
    PhysicalKey::Code(code) => format!("{code:?}"),
    PhysicalKey::Unidentified(_) => String::new(),
  }
}

/// Shows `cursor` on `window` unless it is the `applied` one already.
fn set_window_cursor(window: &Window, applied: &mut CursorIcon, cursor: CursorIcon) {
  if cursor == *applied {
    return;
  }
  // No cursor hides the pointer; any other shows it again.
  if cursor == CursorIcon::None {
    window.set_cursor_visible(false);
  } else {
    if *applied == CursorIcon::None {
      window.set_cursor_visible(true);
    }
    window.set_cursor(to_winit_cursor(cursor));
  }
  *applied = cursor;
}

fn to_winit_cursor(cursor: CursorIcon) -> WinitCursorIcon {
  match cursor {
    CursorIcon::Default => WinitCursorIcon::Default,
    CursorIcon::ContextMenu => WinitCursorIcon::ContextMenu,
    CursorIcon::Help => WinitCursorIcon::Help,
    CursorIcon::Pointer => WinitCursorIcon::Pointer,
    CursorIcon::Progress => WinitCursorIcon::Progress,
    CursorIcon::Wait => WinitCursorIcon::Wait,
    CursorIcon::Cell => WinitCursorIcon::Cell,
    CursorIcon::Crosshair => WinitCursorIcon::Crosshair,
    CursorIcon::Text => WinitCursorIcon::Text,
    CursorIcon::VerticalText => WinitCursorIcon::VerticalText,
    CursorIcon::Alias => WinitCursorIcon::Alias,
    CursorIcon::Copy => WinitCursorIcon::Copy,
    CursorIcon::Move => WinitCursorIcon::Move,
    CursorIcon::NoDrop => WinitCursorIcon::NoDrop,
    CursorIcon::NotAllowed => WinitCursorIcon::NotAllowed,
    CursorIcon::Grab => WinitCursorIcon::Grab,
    CursorIcon::Grabbing => WinitCursorIcon::Grabbing,
    CursorIcon::EResize => WinitCursorIcon::EResize,
    CursorIcon::NResize => WinitCursorIcon::NResize,
    CursorIcon::NeResize => WinitCursorIcon::NeResize,
    CursorIcon::NwResize => WinitCursorIcon::NwResize,
    CursorIcon::SResize => WinitCursorIcon::SResize,
    CursorIcon::SeResize => WinitCursorIcon::SeResize,
    CursorIcon::SwResize => WinitCursorIcon::SwResize,
    CursorIcon::WResize => WinitCursorIcon::WResize,
    CursorIcon::EwResize => WinitCursorIcon::EwResize,
    CursorIcon::NsResize => WinitCursorIcon::NsResize,
    CursorIcon::NeswResize => WinitCursorIcon::NeswResize,
    CursorIcon::NwseResize => WinitCursorIcon::NwseResize,
    CursorIcon::ColResize => WinitCursorIcon::ColResize,
    CursorIcon::RowResize => WinitCursorIcon::RowResize,
    CursorIcon::AllScroll => WinitCursorIcon::AllScroll,
    CursorIcon::ZoomIn => WinitCursorIcon::ZoomIn,
    CursorIcon::ZoomOut => WinitCursorIcon::ZoomOut,
    CursorIcon::DndAsk => WinitCursorIcon::DndAsk,
    CursorIcon::AllResize => WinitCursorIcon::AllResize,
    // Hidden by `set_window_cursor`: the icon it would have is never shown.
    CursorIcon::None => WinitCursorIcon::Default,
  }
}

fn to_winit_icon(icon: WindowIcon) -> Option<WinitIcon> {
  let (rgba, width, height) = icon.into_rgba();
  WinitIcon::from_rgba(rgba, width, height).ok()
}

/// winit's window icon is only Windows' small icon (`ICON_SMALL`, the title
/// bar); the taskbar button and Alt+Tab show the big one (`ICON_BIG`), which
/// would otherwise stay the executable's icon. Both get the same image.
#[cfg(windows)]
fn with_window_icons(attrs: WindowAttributes, icon: Option<WinitIcon>) -> WindowAttributes {
  attrs.with_window_icon(icon.clone()).with_taskbar_icon(icon)
}

/// macOS ignores window icons (the bundle's icon stands for the app); X11
/// takes this one for the title bar and the task switcher.
#[cfg(not(windows))]
fn with_window_icons(attrs: WindowAttributes, icon: Option<WinitIcon>) -> WindowAttributes {
  attrs.with_window_icon(icon)
}

/// See [`with_window_icons`].
#[cfg(windows)]
fn set_window_icons(window: &Window, icon: Option<WinitIcon>) {
  window.set_taskbar_icon(icon.clone());
  window.set_window_icon(icon);
}

/// See [`with_window_icons`].
#[cfg(not(windows))]
fn set_window_icons(window: &Window, icon: Option<WinitIcon>) {
  window.set_window_icon(icon);
}

#[cfg(windows)]
fn with_title_bar_color(attrs: WindowAttributes, color: Option<Color>) -> WindowAttributes {
  attrs.with_title_background_color(color.map(to_winit_windows_color))
}

#[cfg(not(windows))]
fn with_title_bar_color(attrs: WindowAttributes, color: Option<Color>) -> WindowAttributes {
  let _ = color;
  attrs
}

#[cfg(windows)]
fn set_title_bar_color(window: &Window, color: Option<Color>) {
  window.set_title_background_color(color.map(to_winit_windows_color));
}

#[cfg(not(windows))]
fn set_title_bar_color(window: &Window, color: Option<Color>) {
  let _ = (window, color);
}

#[cfg(windows)]
fn set_border_color(window: &Window, color: WindowBorderColor) {
  window.set_border_color(match color {
    WindowBorderColor::Default => Some(WinitWindowsColor::SYSTEM_DEFAULT),
    WindowBorderColor::None => None,
    WindowBorderColor::Color(color) => Some(to_winit_windows_color(color)),
  });
}

#[cfg(not(windows))]
fn set_border_color(window: &Window, color: WindowBorderColor) {
  let _ = (window, color);
}

#[cfg(windows)]
fn to_winit_windows_color(color: Color) -> WinitWindowsColor {
  WinitWindowsColor::from_rgb(color.r(), color.g(), color.b())
}

#[cfg(windows)]
fn with_corner_radius(attrs: WindowAttributes, radius: WindowCornerRadius) -> WindowAttributes {
  attrs.with_corner_preference(to_winit_corner_preference(radius))
}

#[cfg(not(windows))]
fn with_corner_radius(attrs: WindowAttributes, radius: WindowCornerRadius) -> WindowAttributes {
  let _ = radius;
  attrs
}

#[cfg(windows)]
fn set_corner_radius(window: &Window, radius: WindowCornerRadius) {
  window.set_corner_preference(to_winit_corner_preference(radius));
}

#[cfg(target_os = "macos")]
fn set_corner_radius(window: &Window, radius: WindowCornerRadius) {
  let Ok(handle) = window.window_handle() else {
    return;
  };

  let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
    return;
  };

  let radius = to_macos_corner_radius(radius);
  unsafe {
    use objc2::{msg_send, runtime::AnyObject};

    let view = handle.ns_view.as_ptr().cast::<AnyObject>();
    let wants_layer: bool = msg_send![view, wantsLayer];
    if !wants_layer {
      if radius.is_none() {
        return;
      }
      let _: () = msg_send![view, setWantsLayer: true];
    }

    let layer: *mut AnyObject = msg_send![view, layer];
    if layer.is_null() {
      return;
    }

    let _: () = msg_send![layer, setCornerRadius: radius.unwrap_or(0.0)];
    let _: () = msg_send![layer, setMasksToBounds: radius.is_some()];
  }
}

#[cfg(all(not(windows), not(target_os = "macos")))]
fn set_corner_radius(window: &Window, radius: WindowCornerRadius) {
  let _ = (window, radius);
}

#[cfg(windows)]
fn to_winit_corner_preference(radius: WindowCornerRadius) -> WinitCornerPreference {
  match radius {
    WindowCornerRadius::Default => WinitCornerPreference::Default,
    WindowCornerRadius::None => WinitCornerPreference::DoNotRound,
    WindowCornerRadius::Rounded => WinitCornerPreference::Round,
    WindowCornerRadius::RoundedSmall => WinitCornerPreference::RoundSmall,
  }
}

#[cfg(target_os = "macos")]
fn to_macos_corner_radius(radius: WindowCornerRadius) -> Option<f64> {
  match radius {
    WindowCornerRadius::Default => None,
    WindowCornerRadius::None => Some(0.0),
    WindowCornerRadius::Rounded => Some(10.0),
    WindowCornerRadius::RoundedSmall => Some(4.0),
  }
}

fn to_winit_resize_direction(direction: WindowResizeDirection) -> WinitResizeDirection {
  match direction {
    WindowResizeDirection::East => WinitResizeDirection::East,
    WindowResizeDirection::North => WinitResizeDirection::North,
    WindowResizeDirection::NorthEast => WinitResizeDirection::NorthEast,
    WindowResizeDirection::NorthWest => WinitResizeDirection::NorthWest,
    WindowResizeDirection::South => WinitResizeDirection::South,
    WindowResizeDirection::SouthEast => WinitResizeDirection::SouthEast,
    WindowResizeDirection::SouthWest => WinitResizeDirection::SouthWest,
    WindowResizeDirection::West => WinitResizeDirection::West,
  }
}

/// Starts the native window move loop. True once the OS owns the left-button press: the loop consumes
/// its release, so lurq never receives it (the caller ends the press with
/// [`Tree::mouse_press_taken_by_os`]). This holds for the Windows loop, `performWindowDragWithEvent` on
/// macOS, and the X11 and Wayland compositor moves behind `drag_window`.
fn start_native_window_drag(window: &Window) -> bool {
  begin_native_window_drag(window) || window.drag_window().is_ok()
}

/// Starts the native window resize loop; see [`start_native_window_drag`]. macOS has none, so a resize
/// handle there keeps its press and gets the real release.
fn start_native_window_resize(window: &Window, direction: WindowResizeDirection) -> bool {
  begin_native_window_resize(window, direction)
    || window.drag_resize_window(to_winit_resize_direction(direction)).is_ok()
}

/// Has `tree` leave where the window's controls are (`.window_control(..)`) after each layout, and lays the
/// shell's procedure over `window`, which answers the system's hit test from it; see `app::window_control_hits`.
#[cfg(windows)]
fn tell_window_controls(window: &Window, tree: &mut Tree) {
  let Ok(handle) = window.window_handle() else {
    return;
  };
  if let RawWindowHandle::Win32(handle) = handle.as_raw() {
    tree.publish_window_controls();
    crate::app::window_control_hits::lay_over(handle.hwnd.get(), tree.window().clone());
  }
}

#[cfg(windows)]
fn begin_native_window_drag(window: &Window) -> bool {
  use windows::Win32::UI::WindowsAndMessaging::HTCAPTION;

  send_native_non_client_mouse_down(window, HTCAPTION)
}

#[cfg(not(windows))]
fn begin_native_window_drag(window: &Window) -> bool {
  let _ = window;
  false
}

#[cfg(windows)]
fn begin_native_window_resize(window: &Window, direction: WindowResizeDirection) -> bool {
  use windows::Win32::UI::WindowsAndMessaging::{
    HTBOTTOM, HTBOTTOMLEFT, HTBOTTOMRIGHT, HTLEFT, HTRIGHT, HTTOP, HTTOPLEFT, HTTOPRIGHT,
  };

  let hit_test = match direction {
    WindowResizeDirection::North => HTTOP,
    WindowResizeDirection::South => HTBOTTOM,
    WindowResizeDirection::West => HTLEFT,
    WindowResizeDirection::East => HTRIGHT,
    WindowResizeDirection::NorthWest => HTTOPLEFT,
    WindowResizeDirection::NorthEast => HTTOPRIGHT,
    WindowResizeDirection::SouthWest => HTBOTTOMLEFT,
    WindowResizeDirection::SouthEast => HTBOTTOMRIGHT,
  };
  send_native_non_client_mouse_down(window, hit_test)
}

#[cfg(not(windows))]
fn begin_native_window_resize(window: &Window, direction: WindowResizeDirection) -> bool {
  let _ = (window, direction);
  false
}

/// Starts the native move/size loop the way a press on a real frame would.
///
/// The message is posted, not sent: `SendMessageW` ran the whole modal loop inside the lurq event
/// handler that issued the command, where winit buffers every event. No frame was drawn for the
/// entire drag (DWM stretched the last one) and the buffered size events were replayed afterwards.
/// Posted, the loop runs from winit's own message pump, which delivers `Resized` and paints during
/// the drag. The start point is the cursor position in screen coordinates, which the loop uses as
/// the drag origin (for example to place a maximized window restored by dragging its caption).
#[cfg(windows)]
fn send_native_non_client_mouse_down(window: &Window, hit_test: u32) -> bool {
  use windows::Win32::{
    Foundation::{HWND, LPARAM, POINT, WPARAM},
    UI::{
      Input::KeyboardAndMouse::ReleaseCapture,
      WindowsAndMessaging::{GetCursorPos, PostMessageW, WM_NCLBUTTONDOWN},
    },
  };

  let Ok(handle) = window.window_handle() else {
    return false;
  };
  let RawWindowHandle::Win32(handle) = handle.as_raw() else {
    return false;
  };
  let hwnd = HWND(handle.hwnd.get() as *mut _);
  if hwnd.is_invalid() {
    return false;
  }

  let mut cursor = POINT::default();
  unsafe {
    if GetCursorPos(&mut cursor).is_err() {
      return false;
    }
    let _ = ReleaseCapture();
    PostMessageW(
      Some(hwnd),
      WM_NCLBUTTONDOWN,
      WPARAM(hit_test as usize),
      LPARAM(screen_point_lparam(cursor.x, cursor.y)),
    )
    .is_ok()
  }
}

/// `MAKELPARAM` of a screen point, as `WM_NC*` mouse messages carry it: signed 16-bit x and y.
#[cfg(windows)]
fn screen_point_lparam(x: i32, y: i32) -> isize {
  let x = x as i16 as u16 as u32;
  let y = y as i16 as u16 as u32;
  ((y << 16) | x) as i32 as isize
}

#[cfg(all(test, windows))]
mod native_drag_tests {
  use super::screen_point_lparam;

  #[test]
  fn screen_point_lparam_packs_signed_coordinates() {
    assert_eq!(screen_point_lparam(0, 0), 0);
    assert_eq!(screen_point_lparam(1440, 900), (900 << 16) | 1440);
    // Monitors left of or above the primary have negative screen coordinates.
    let packed = screen_point_lparam(-100, -20) as u32;
    assert_eq!((packed & 0xffff) as u16 as i16, -100);
    assert_eq!((packed >> 16) as u16 as i16, -20);
  }
}

#[cfg(test)]
mod close_runtime_tests {
  use super::*;
  #[test]
  fn secondary_veto_delayed_acceptance_and_unconditional_close() {
    let mut app = App::new();
    let mut root = Tree::new();
    app.window_opener().open("Preferences", 200, 200, |_, _| {});
    root.apply_secondary_window_requests(&mut app);
    let secondary = root.secondary_window_mut(0).unwrap();
    let mut managed = ManagedSecondaryWindow::new(0, secondary);
    let tree = secondary.tree_mut();
    tree.window().handle().on_close_requested(|r| r.cancel());
    tree.window().dispatch_close_request(CloseRequestSource::Os);
    assert!(!managed.apply_window_commands(tree));
    assert!(!managed.close_requested());
    let pending = std::sync::Arc::new(std::sync::Mutex::new(None));
    let save = pending.clone();
    tree
      .window()
      .handle()
      .on_close_requested(move |r| *save.lock().unwrap() = Some(r));
    tree.window().handle().request_close();
    assert!(!managed.apply_window_commands(tree));
    pending.lock().unwrap().take().unwrap().proceed();
    assert!(managed.apply_window_commands(tree));
    assert!(managed.close_requested());
    tree
      .window()
      .handle()
      .on_close_requested(|_| panic!("unconditional close called handler"));
    tree.window().handle().close();
    assert!(managed.apply_window_commands(tree));
  }
}

#[cfg(all(test, windows))]
mod native_window_tests {
  use windows::Win32::{
    Foundation::{HWND, LPARAM, WPARAM},
    UI::WindowsAndMessaging::{ICON_BIG, ICON_SMALL, SendMessageW, WM_GETICON},
  };
  use winit::platform::windows::EventLoopBuilderExtWindows;

  use super::*;

  /// The window's `ICON_SMALL` or `ICON_BIG` handle, 0 when unset.
  fn icon_handle(window: &Window, kind: u32) -> isize {
    let RawWindowHandle::Win32(handle) = window.window_handle().expect("window handle").as_raw() else {
      panic!("not a Win32 window");
    };
    let hwnd = HWND(handle.hwnd.get() as *mut std::ffi::c_void);
    // SAFETY: `hwnd` is the test's own live window; WM_GETICON only reads it.
    unsafe { SendMessageW(hwnd, WM_GETICON, Some(WPARAM(kind as usize)), Some(LPARAM(0))).0 }
  }

  /// One test: winit allows one event loop per process.
  #[test]
  fn a_hidden_window_takes_icons_and_runtime_titles() {
    // The test's only event loop: winit allows one per process.
    let event_loop = EventLoop::builder().with_any_thread(true).build().expect("event loop");
    let icon = || to_winit_icon(WindowIcon::from_rgba(vec![200; 32 * 32 * 4], 32, 32));
    let attributes = with_window_icons(WindowAttributes::default().with_visible(false), icon());
    // A hidden window of the test's own, outside `run_app`.
    #[allow(deprecated)]
    let window = event_loop.create_window(attributes).expect("hidden test window");
    assert_ne!(icon_handle(&window, ICON_SMALL), 0, "title bar icon");
    assert_ne!(icon_handle(&window, ICON_BIG), 0, "taskbar and Alt+Tab icon");

    set_window_icons(&window, None);
    assert_eq!(icon_handle(&window, ICON_SMALL), 0);
    assert_eq!(icon_handle(&window, ICON_BIG), 0);

    set_window_icons(&window, icon());
    assert_ne!(icon_handle(&window, ICON_SMALL), 0);
    assert_ne!(icon_handle(&window, ICON_BIG), 0);

    runtime_title_reaches_the_os_window(window);
  }

  /// `WindowHandle::set_title` is applied by the shell to the live window.
  fn runtime_title_reaches_the_os_window(window: Window) {
    let mut app = App::new();
    let mut root = Tree::new();
    app.window_opener().open("Preferences", 200, 200, |_, _| {});
    root.apply_secondary_window_requests(&mut app);
    let secondary = root.secondary_window_mut(0).expect("opened secondary window");
    let mut managed = ManagedSecondaryWindow::new(0, secondary);
    let tree = secondary.tree_mut();
    assert_eq!(tree.window().handle().title().as_deref(), Some("Preferences"));
    managed.window = Some(window);

    tree.window().handle().set_title("Tasks - Orchester");
    managed.apply_window_commands(tree);
    let os_title = |managed: &ManagedSecondaryWindow| managed.window.as_ref().expect("test window").title();
    assert_eq!(os_title(&managed), "Tasks - Orchester");
    assert_eq!(tree.window().handle().title().as_deref(), Some("Tasks - Orchester"));

    tree.window().handle().set_title("Run 12 - Orchester");
    managed.apply_window_commands(tree);
    assert_eq!(os_title(&managed), "Run 12 - Orchester");
  }
}
