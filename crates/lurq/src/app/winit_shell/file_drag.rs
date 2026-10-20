//! Files dragged into a winit window from the OS, turned into the tree's
//! file-drag session.
//!
//! winit reports a drag with one `HoveredFile` per file when it enters the
//! window, `HoveredFileCancelled` when it leaves, and one `DroppedFile` per
//! file on the drop, all in one burst from the OS callback (`IDropTarget` on
//! Windows, `draggingEntered:`/`performDragOperation:` on macOS). It reports
//! no position and no movement during the drag: the OS owns the pointer, so
//! no `CursorMoved` comes either. The burst is collected and delivered once
//! the events of the turn are handled, as one enter or drop with all its
//! paths, at the pointer position read from the OS
//! ([`cursor::position_in_window`]); while the drag hovers, the position is
//! read again every [`POLL_INTERVAL`] and the tree gets `file_drag_over`.
//!
//! winit answers the OS about the drop effect before lurq sees the drag (a
//! copy cursor over the whole window), so an element's refusal does not
//! change the cursor.

use std::{
  path::PathBuf,
  time::{Duration, Instant},
};

use winit::{event::WindowEvent, window::Window};

use super::cursor;
use crate::app::{Tree, events::DraggedPaths};

/// How often the pointer position is read while files hover over the window.
const POLL_INTERVAL: Duration = Duration::from_millis(16);

pub(super) struct FileDragShell {
  /// Paths of the burst of `HoveredFile` events not yet delivered.
  hovered: Vec<PathBuf>,
  /// Paths of the burst of `DroppedFile` events not yet delivered.
  dropped: Vec<PathBuf>,
  /// A `HoveredFileCancelled` not yet delivered.
  cancelled: bool,
  /// A drag is over the window: its position is polled.
  hovering: bool,
  last_poll: Instant,
  last_position: Option<(f32, f32)>,
}

impl Default for FileDragShell {
  fn default() -> Self {
    Self {
      hovered: Vec::new(),
      dropped: Vec::new(),
      cancelled: false,
      hovering: false,
      last_poll: Instant::now(),
      last_position: None,
    }
  }
}

impl FileDragShell {
  /// Takes a file event; `false` for any other event.
  pub(super) fn on_event(&mut self, event: &WindowEvent) -> bool {
    match event {
      WindowEvent::HoveredFile(path) => self.hovered.push(path.clone()),
      WindowEvent::DroppedFile(path) => self.dropped.push(path.clone()),
      WindowEvent::HoveredFileCancelled => self.cancelled = true,
      _ => return false,
    }
    true
  }

  /// Whether a file event waits to be delivered.
  pub(super) fn has_pending(&self) -> bool {
    !self.hovered.is_empty() || !self.dropped.is_empty() || self.cancelled
  }

  /// When the pointer position is next read, while a drag hovers.
  pub(super) fn next_poll(&self) -> Option<Instant> {
    self.hovering.then(|| self.last_poll + POLL_INTERVAL)
  }

  /// Delivers the collected events to the tree and polls the pointer while a
  /// drag hovers. `fallback` is the last pointer position the window saw,
  /// used where the OS position cannot be read.
  pub(super) fn flush(&mut self, tree: &mut Tree, window: Option<&Window>, fallback: (f64, f64)) {
    let now = Instant::now();
    if self.cancelled {
      self.cancelled = false;
      self.hovering = false;
      self.hovered.clear();
      tree.file_drag_leave();
    }
    if !self.hovered.is_empty() {
      let paths = DraggedPaths::from_paths(std::mem::take(&mut self.hovered));
      let (x, y) = self.position(window, fallback);
      self.hovering = true;
      self.last_poll = now;
      tree.file_drag_enter(paths, x, y);
    }
    if !self.dropped.is_empty() {
      let paths = DraggedPaths::from_paths(std::mem::take(&mut self.dropped));
      let (x, y) = self.position(window, fallback);
      self.hovering = false;
      tree.file_drop(paths, x, y);
      return;
    }
    if self.hovering && now.duration_since(self.last_poll) >= POLL_INTERVAL {
      self.last_poll = now;
      let position = self.position(window, fallback);
      if self.last_position != Some(position) {
        tree.file_drag_over(position.0, position.1);
      }
    }
  }

  fn position(&mut self, window: Option<&Window>, fallback: (f64, f64)) -> (f32, f32) {
    let position = window
      .and_then(cursor::position_in_window)
      .unwrap_or((fallback.0 as f32, fallback.1 as f32));
    self.last_position = Some(position);
    position
  }
}
