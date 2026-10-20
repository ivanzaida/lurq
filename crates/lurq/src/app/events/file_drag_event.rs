//! Files dragged into the window from the OS (Explorer, Finder, another
//! program), delivered to the element under the pointer.
//!
//! The paths are given once for the whole drag, however many files it holds.
//! Only files and folders arrive: winit reports nothing else, so text or an
//! image dragged from a browser is ignored.

use std::{
  path::{Path, PathBuf},
  sync::Arc,
};

use super::EventControl;
use crate::core::NodeId;

/// One path of a drag: a file, or a folder (which an app may refuse).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DroppedPath {
  path: PathBuf,
  is_folder: bool,
}

impl DroppedPath {
  /// Looks the path up once (`Path::is_dir`, on the calling thread: the
  /// shell's, when the drag enters the window) so a folder is reported as
  /// one. A path on a slow network drive can make that look-up slow.
  pub fn new(path: PathBuf) -> Self {
    let is_folder = path.is_dir();
    Self { path, is_folder }
  }

  /// A path with its kind given, for a synthetic drag of paths that do not
  /// exist.
  pub fn with_kind(path: PathBuf, is_folder: bool) -> Self {
    Self { path, is_folder }
  }

  pub fn path(&self) -> &Path {
    &self.path
  }

  pub fn into_path(self) -> PathBuf {
    self.path
  }

  pub fn is_folder(&self) -> bool {
    self.is_folder
  }

  pub fn is_file(&self) -> bool {
    !self.is_folder
  }
}

/// The paths of one drag, shared by its events.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DraggedPaths {
  items: Arc<[DroppedPath]>,
}

impl DraggedPaths {
  pub fn new(items: impl IntoIterator<Item = DroppedPath>) -> Self {
    Self {
      items: items.into_iter().collect(),
    }
  }

  /// Looks every path up once; see [`DroppedPath::new`].
  pub fn from_paths(paths: impl IntoIterator<Item = PathBuf>) -> Self {
    Self::new(paths.into_iter().map(DroppedPath::new))
  }

  pub fn items(&self) -> &[DroppedPath] {
    &self.items
  }

  pub fn paths(&self) -> impl Iterator<Item = &Path> {
    self.items.iter().map(DroppedPath::path)
  }

  pub fn files(&self) -> impl Iterator<Item = &Path> {
    self.items.iter().filter(|item| item.is_file()).map(DroppedPath::path)
  }

  pub fn folders(&self) -> impl Iterator<Item = &Path> {
    self.items.iter().filter(|item| item.is_folder()).map(DroppedPath::path)
  }

  pub fn has_folders(&self) -> bool {
    self.items.iter().any(DroppedPath::is_folder)
  }

  pub fn len(&self) -> usize {
    self.items.len()
  }

  pub fn is_empty(&self) -> bool {
    self.items.is_empty()
  }
}

/// A drag of files entering, or moving over, an element: `on_file_drag_enter`
/// and `on_file_drag_over`. `x` and `y` are logical pixels in the window, as
/// on a [`MouseEvent`](super::MouseEvent).
#[derive(Clone, Debug)]
pub struct FileDragEvent {
  pub x: f32,
  pub y: f32,
  pub paths: DraggedPaths,
  /// The node whose handlers run now: the element under the pointer, or an
  /// ancestor the event bubbled to.
  pub target_id: NodeId,
  pub(crate) control: EventControl,
  pub(crate) accepted: Arc<std::sync::atomic::AtomicBool>,
}

impl FileDragEvent {
  pub(crate) fn new(x: f32, y: f32, paths: DraggedPaths, target_id: NodeId) -> Self {
    Self {
      x,
      y,
      paths,
      target_id,
      control: EventControl::new(),
      accepted: Arc::new(std::sync::atomic::AtomicBool::new(true)),
    }
  }

  /// The event as `target_id`'s handlers see it: the propagation state is
  /// shared, the acceptance is this node's own, so a refusal is the node's.
  pub(crate) fn at(&self, target_id: NodeId) -> Self {
    Self {
      target_id,
      paths: self.paths.clone(),
      control: self.control.clone(),
      accepted: Arc::new(std::sync::atomic::AtomicBool::new(true)),
      ..*self
    }
  }

  /// Refuses the drag at this element for the rest of the drag session:
  /// while the drag is over it, no element shows the `drag_over` style and
  /// [`Tree::file_drag_accepted`](crate::app::Tree::file_drag_accepted)
  /// reports `Some(false)`; the drag leaving the element and entering it
  /// again asks again. The OS cursor is not changed: winit answers the OS
  /// before lurq sees the drag, so it shows a copy cursor over the whole
  /// window. The drop is still delivered to `on_file_drop`, whose handler
  /// ignores what it refused.
  pub fn refuse(&self) {
    self.accepted.store(false, std::sync::atomic::Ordering::Relaxed);
    self.control.prevent_default();
  }

  /// Same as [`refuse`](Self::refuse).
  pub fn prevent_default(&self) {
    self.refuse();
  }

  pub fn default_prevented(&self) -> bool {
    self.control.default_prevented()
  }

  pub fn is_accepted(&self) -> bool {
    self.accepted.load(std::sync::atomic::Ordering::Relaxed)
  }

  pub fn stop_propagation(&self) {
    self.control.stop_propagation();
  }

  pub fn propagation_stopped(&self) -> bool {
    self.control.propagation_stopped()
  }

  pub fn stop_immediate_propagation(&self) {
    self.control.stop_immediate_propagation();
  }

  pub fn immediate_propagation_stopped(&self) -> bool {
    self.control.immediate_propagation_stopped()
  }
}

/// Files dropped on an element: `on_file_drop`. `x` and `y` are logical
/// pixels in the window.
#[derive(Clone, Debug)]
pub struct FileDropEvent {
  pub x: f32,
  pub y: f32,
  pub paths: DraggedPaths,
  /// The node whose handlers run now: the element under the pointer, or an
  /// ancestor the event bubbled to.
  pub target_id: NodeId,
  pub(crate) control: EventControl,
}

impl FileDropEvent {
  pub(crate) fn new(x: f32, y: f32, paths: DraggedPaths, target_id: NodeId) -> Self {
    Self {
      x,
      y,
      paths,
      target_id,
      control: EventControl::new(),
    }
  }

  pub(crate) fn at(&self, target_id: NodeId) -> Self {
    Self {
      target_id,
      paths: self.paths.clone(),
      control: self.control.clone(),
      ..*self
    }
  }

  pub fn prevent_default(&self) {
    self.control.prevent_default();
  }

  pub fn default_prevented(&self) -> bool {
    self.control.default_prevented()
  }

  pub fn stop_propagation(&self) {
    self.control.stop_propagation();
  }

  pub fn propagation_stopped(&self) -> bool {
    self.control.propagation_stopped()
  }

  pub fn stop_immediate_propagation(&self) {
    self.control.stop_immediate_propagation();
  }

  pub fn immediate_propagation_stopped(&self) -> bool {
    self.control.immediate_propagation_stopped()
  }
}
