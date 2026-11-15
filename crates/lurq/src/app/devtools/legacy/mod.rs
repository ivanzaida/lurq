//! The tabs that are still the DevTools of before the design system: the Profiler, Signals and Storage, with the
//! colors and the icon font of their own (`style`). Each goes when its tab is rebuilt from the board
//! (`pencil/lurq-devtools.pen`) with the design system's components, as Components was.

#[cfg(feature = "persistent_storage")]
pub(super) mod persistent_storage_panel;
pub(super) mod profiler;
pub(super) mod profiler_analysis;
pub(super) mod profiler_capture;
pub(super) mod signals;
pub(super) mod style;

use super::{DevToolsNode, DevToolsSnapshot};
