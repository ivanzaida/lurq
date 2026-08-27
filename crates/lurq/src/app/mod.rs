pub mod app_state;
pub mod component;
pub mod ctx;
#[cfg(feature = "devtools")]
pub mod devtools;
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
pub(crate) mod glyph_engine;
pub(crate) mod hit_test;
#[cfg(feature = "i18n")]
pub mod i18n;
pub(crate) mod profile_support;
pub(crate) mod profile_types;
#[cfg(feature = "perf_profile")]
pub mod profiler;
pub mod render_engine;
pub mod runtime;
pub mod synthetic_input;
pub mod theme;
#[cfg(feature = "wgpu")]
pub mod wgpu_render;
pub mod window;
#[cfg(feature = "winit")]
pub mod winit_shell;

pub use app_state::{App, SecondaryWindowRequest, WindowOpener, WindowOptions};
pub use runtime::{
  CheckboxHandle, ElementHandle, PassReasons, PassReport, SelectHandle, SliderHandle, TextInputHandle, Tree,
};
pub use synthetic_input::{SyntheticInput, SyntheticInputKind, SyntheticModifiers};
pub use window::{Window, WindowCornerRadius, WindowHandle, WindowIcon, WindowInfo, WindowResizeDirection};
