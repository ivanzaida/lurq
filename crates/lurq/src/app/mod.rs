pub mod app_state;
#[cfg(all(test, windows, feature = "screenshot", any(feature = "wgpu", feature = "dx12")))]
mod backdrop_blur_readback_tests;
pub mod biometry;
#[cfg(all(
  test,
  windows,
  feature = "raster",
  feature = "screenshot",
  any(feature = "wgpu", feature = "dx12")
))]
mod blend_readback_tests;
#[cfg(all(test, windows, feature = "screenshot", any(feature = "wgpu", feature = "dx12")))]
mod box_shadow_readback_tests;
#[cfg(all(
  test,
  windows,
  feature = "canvas",
  feature = "perf_profile",
  feature = "screenshot",
  feature = "wgpu",
  feature = "dx12"
))]
mod canvas_frame_probe;
#[cfg(feature = "screenshot")]
pub(crate) mod capture_overlay;
#[cfg(feature = "screenshot")]
pub(crate) mod capture_redaction;
#[cfg(all(test, feature = "screenshot"))]
pub(crate) mod capture_test_engine;
#[cfg(all(windows, feature = "raster", any(feature = "wgpu", feature = "dx12")))]
pub(crate) mod card_read;
pub mod component;
pub mod ctx;
#[cfg(feature = "devtools")]
pub mod devtools;
#[cfg(all(test, windows, feature = "canvas", feature = "dx12", feature = "screenshot"))]
mod dx12_canvas_residency_tests;
#[cfg(all(feature = "dx12", target_os = "windows"))]
pub mod dx12_render;
pub mod events;
#[cfg(feature = "screenshot")]
// Without a render backend nothing performs captures; the helpers stay for
// the mcp feature's capture types.
#[cfg_attr(
  not(any(feature = "wgpu", all(feature = "dx12", target_os = "windows"))),
  allow(dead_code)
)]
pub(crate) mod frame_capture;
#[cfg(all(test, windows, feature = "screenshot", any(feature = "wgpu", feature = "dx12")))]
mod glyph_batch_readback_tests;
pub(crate) mod glyph_engine;
#[cfg(all(test, windows, feature = "screenshot", any(feature = "wgpu", feature = "dx12")))]
mod gradient_readback_tests;
pub(crate) mod hit_test;
pub mod i18n;
#[cfg(all(target_os = "ios", feature = "raster"))]
pub(crate) mod ios_symbol;
#[cfg(all(test, windows, feature = "screenshot", any(feature = "wgpu", feature = "dx12")))]
mod layer_blur_readback_tests;
#[cfg(all(test, windows, feature = "screenshot", any(feature = "wgpu", feature = "dx12")))]
mod nested_clip_readback_tests;
#[cfg(all(test, windows, feature = "screenshot", any(feature = "wgpu", feature = "dx12")))]
mod opacity_layer_readback_tests;
pub(crate) mod pointer;
pub(crate) mod profile_support;
pub(crate) mod profile_types;
pub mod profiler;
#[cfg(all(test, windows, feature = "screenshot", any(feature = "wgpu", feature = "dx12")))]
mod readback_window;
pub mod render_engine;
pub mod runtime;
#[cfg(all(test, windows, feature = "screenshot", any(feature = "wgpu", feature = "dx12")))]
mod stroke_readback_tests;
pub mod synthetic_input;
pub mod theme;
#[cfg(target_arch = "wasm32")]
#[cfg_attr(not(all(feature = "winit", feature = "wgpu")), allow(dead_code))]
pub(crate) mod web_canvas;
#[cfg(feature = "wgpu")]
pub mod wgpu_render;
pub mod window;
#[cfg(all(feature = "winit", windows))]
mod window_control_hits;
#[cfg(feature = "winit")]
pub mod winit_shell;
#[cfg(all(
  test,
  windows,
  feature = "raster",
  feature = "screenshot",
  any(feature = "wgpu", feature = "dx12")
))]
mod yuv_readback_tests;

pub use app_state::{App, SecondaryWindowRequest, WindowOpener, WindowOptions};
pub use pointer::Pointer;
pub use runtime::{
  CheckboxHandle, ClipboardReader, ClipboardWriter, ElementHandle, PassReasons, PassReport, SelectHandle, SliderHandle,
  TextInputHandle, Tree,
};
pub use synthetic_input::{SyntheticInput, SyntheticInputKind, SyntheticModifiers};
#[cfg(feature = "screenshot")]
pub use window::ScreenshotRegion;
pub use window::{
  CloseRequest, CloseRequestSource, EditMenuRequest, Window, WindowBorderColor, WindowCornerRadius, WindowHandle,
  WindowIcon, WindowInfo, WindowResizeDirection,
};

mod device;
pub use device::Device;
pub mod menu;
pub mod open_url;
pub use menu::{Accelerator, ApplicationMenu, Menu, MenuAction, MenuBar, MenuBarSupport, MenuController, MenuItem};
pub use open_url::{OpenUrlError, open_url};
pub mod system_appearance;
pub use system_appearance::system_appearance;
#[cfg(all(feature = "winit", target_os = "macos"))]
mod macos_menu;
