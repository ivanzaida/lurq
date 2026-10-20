//! Files dragged into a winit window from the OS, turned into the tree's
//! file-drag session.
//!
//! winit reports a drag with one `HoveredFile` per file when it enters the
//! window, `HoveredFileCancelled` when it leaves, and one `DroppedFile` per
//! file on the drop, all in one burst from the OS callback (`IDropTarget` on
//! Windows, `draggingEntered:`/`performDragOperation:` on macOS). It reports
//! no position and no movement during the drag: the OS owns the pointer, so
//! no `CursorMoved` comes either. The burst is collected and delivered once
//! the events of the turn are handled, in the order they arrived, as one
//! enter or drop with all its paths, at the pointer position read from the
//! OS ([`cursor::position_in_window`]); while the drag hovers, the position
//! is read again every [`POLL_INTERVAL`] and the tree gets `file_drag_over`
//! when it moved. A hover that gets neither a drop nor a cancel within
//! [`HOVER_TIMEOUT`] is ended as a leave: its source is gone (a program that
//! died mid-drag, an event winit lost), and polling for it would never stop.
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
pub(super) const POLL_INTERVAL: Duration = Duration::from_millis(16);
/// A hover without a drop or a cancel for this long is ended; see the module
/// docs.
pub(super) const HOVER_TIMEOUT: Duration = Duration::from_secs(60);

/// One file event of winit, kept in arrival order.
#[derive(Debug, PartialEq, Eq)]
enum FileEvent {
  Hovered(PathBuf),
  Cancelled,
  Dropped(PathBuf),
}

pub(super) struct FileDragShell {
  /// The events of the turn not yet delivered.
  events: Vec<FileEvent>,
  /// A drag is over the window: its position is polled.
  hovering: bool,
  last_poll: Instant,
  /// When the last file event arrived, for the hover timeout.
  last_event: Instant,
  last_position: Option<(f32, f32)>,
}

impl Default for FileDragShell {
  fn default() -> Self {
    let now = Instant::now();
    Self {
      events: Vec::new(),
      hovering: false,
      last_poll: now,
      last_event: now,
      last_position: None,
    }
  }
}

impl FileDragShell {
  /// Takes a file event; `false` for any other event.
  pub(super) fn on_event(&mut self, event: &WindowEvent) -> bool {
    let event = match event {
      WindowEvent::HoveredFile(path) => FileEvent::Hovered(path.clone()),
      WindowEvent::DroppedFile(path) => FileEvent::Dropped(path.clone()),
      WindowEvent::HoveredFileCancelled => FileEvent::Cancelled,
      _ => return false,
    };
    self.events.push(event);
    true
  }

  /// Whether a file event waits to be delivered.
  pub(super) fn has_pending(&self) -> bool {
    !self.events.is_empty()
  }

  /// When the pointer position is next read, while a drag hovers.
  pub(super) fn next_poll(&self) -> Option<Instant> {
    self.hovering.then(|| self.last_poll + POLL_INTERVAL)
  }

  /// Delivers the collected events to the tree and polls the pointer while a
  /// drag hovers. `fallback` is the last pointer position the window saw,
  /// used where the OS position cannot be read.
  pub(super) fn flush(&mut self, tree: &mut Tree, window: Option<&Window>, fallback: (f64, f64)) {
    self.flush_at(
      tree,
      Instant::now(),
      &|| window.and_then(cursor::position_in_window),
      fallback,
    );
  }

  /// [`flush`](Self::flush) at `now`, reading the pointer with `read_position`.
  fn flush_at(
    &mut self,
    tree: &mut Tree,
    now: Instant,
    read_position: &dyn Fn() -> Option<(f32, f32)>,
    fallback: (f64, f64),
  ) {
    let events = std::mem::take(&mut self.events);
    if !events.is_empty() {
      self.last_event = now;
    }
    let mut events = events.into_iter().peekable();
    while let Some(event) = events.next() {
      match event {
        FileEvent::Cancelled => {
          self.hovering = false;
          tree.file_drag_leave();
        }
        FileEvent::Hovered(path) => {
          let mut paths = vec![path];
          while let Some(FileEvent::Hovered(_)) = events.peek() {
            if let Some(FileEvent::Hovered(path)) = events.next() {
              paths.push(path);
            }
          }
          let (x, y) = self.position(read_position, fallback);
          self.hovering = true;
          self.last_poll = now;
          tree.file_drag_enter(DraggedPaths::from_paths(paths), x, y);
        }
        FileEvent::Dropped(path) => {
          let mut paths = vec![path];
          while let Some(FileEvent::Dropped(_)) = events.peek() {
            if let Some(FileEvent::Dropped(path)) = events.next() {
              paths.push(path);
            }
          }
          let (x, y) = self.position(read_position, fallback);
          self.hovering = false;
          tree.file_drop(DraggedPaths::from_paths(paths), x, y);
        }
      }
    }
    if !self.hovering {
      return;
    }
    if now.duration_since(self.last_event) >= HOVER_TIMEOUT {
      self.hovering = false;
      tree.file_drag_leave();
      return;
    }
    if now.duration_since(self.last_poll) >= POLL_INTERVAL {
      self.last_poll = now;
      let moved_from = self.last_position;
      let position = self.position(read_position, fallback);
      if moved_from != Some(position) {
        tree.file_drag_over(position.0, position.1);
      }
    }
  }

  fn position(&mut self, read_position: &dyn Fn() -> Option<(f32, f32)>, fallback: (f64, f64)) -> (f32, f32) {
    let position = read_position().unwrap_or((fallback.0 as f32, fallback.1 as f32));
    self.last_position = Some(position);
    position
  }
}

#[cfg(test)]
mod tests {
  use std::sync::{Arc, Mutex};

  use super::*;
  use crate::{
    app::{
      App,
      events::{FileDragEvent, FileDropEvent},
    },
    components::Row,
  };

  type Log = Arc<Mutex<Vec<String>>>;

  /// A 200×200 window with a 100×100 target at its top-left corner, logging
  /// its file-drag events.
  fn target(log: &Log) -> Tree {
    let mut tree = Tree::new();
    tree.resize(200, 200);
    let (enter, over, leave, drop) = (log.clone(), log.clone(), log.clone(), log.clone());
    tree.set_root(
      Row::new()
        .width(100.0)
        .height(100.0)
        .on_file_drag_enter(move |event: FileDragEvent| {
          enter.lock().unwrap().push(format!("enter {}", event.paths.len()));
        })
        .on_file_drag_over(move |event: FileDragEvent| {
          over.lock().unwrap().push(format!("over {},{}", event.x, event.y));
        })
        .on_file_drag_leave(move || leave.lock().unwrap().push("leave".into()))
        .on_file_drop(move |event: FileDropEvent| {
          drop.lock().unwrap().push(format!("drop {}", event.paths.len()));
        }),
    );
    tree.pass_headless(&mut App::new());
    tree
  }

  fn hovered(shell: &mut FileDragShell, paths: &[&str]) {
    for path in paths {
      assert!(shell.on_event(&WindowEvent::HoveredFile(PathBuf::from(path))));
    }
  }

  fn dropped(shell: &mut FileDragShell, paths: &[&str]) {
    for path in paths {
      assert!(shell.on_event(&WindowEvent::DroppedFile(PathBuf::from(path))));
    }
  }

  #[test]
  fn a_burst_of_hovered_files_enters_once_and_the_pointer_is_polled_while_it_moves() {
    let log = Log::default();
    let mut tree = target(&log);
    let mut shell = FileDragShell::default();
    let position = Arc::new(Mutex::new(Some((10.0_f32, 10.0_f32))));
    let read = {
      let position = position.clone();
      move || *position.lock().unwrap()
    };
    let start = Instant::now();
    hovered(&mut shell, &["C:/a.png", "C:/b.png"]);
    assert!(shell.has_pending());

    shell.flush_at(&mut tree, start, &read, (0.0, 0.0));
    assert!(!shell.has_pending());
    assert_eq!(shell.next_poll(), Some(start + POLL_INTERVAL));
    assert_eq!(log.lock().unwrap().as_slice(), ["enter 2", "over 10,10"]);

    // Before the interval, and at the same position: nothing.
    shell.flush_at(&mut tree, start + POLL_INTERVAL / 2, &read, (0.0, 0.0));
    shell.flush_at(&mut tree, start + POLL_INTERVAL, &read, (0.0, 0.0));
    assert_eq!(log.lock().unwrap().len(), 2, "no over without a move");

    *position.lock().unwrap() = Some((20.0, 30.0));
    shell.flush_at(&mut tree, start + POLL_INTERVAL * 2, &read, (0.0, 0.0));
    assert_eq!(log.lock().unwrap().last().map(String::as_str), Some("over 20,30"));
    assert_eq!(shell.next_poll(), Some(start + POLL_INTERVAL * 3));
  }

  #[test]
  fn enter_over_and_leave_follow_the_events_and_a_cancel_ends_the_polling() {
    let log = Log::default();
    let mut tree = target(&log);
    let mut shell = FileDragShell::default();
    let read = || Some((10.0_f32, 10.0_f32));
    let start = Instant::now();
    hovered(&mut shell, &["C:/a.png"]);
    shell.flush_at(&mut tree, start, &read, (0.0, 0.0));
    assert!(shell.next_poll().is_some());

    assert!(shell.on_event(&WindowEvent::HoveredFileCancelled));
    shell.flush_at(&mut tree, start + POLL_INTERVAL, &read, (0.0, 0.0));

    assert_eq!(log.lock().unwrap().as_slice(), ["enter 1", "over 10,10", "leave"]);
    assert_eq!(shell.next_poll(), None, "no polling after the cancel");
    assert!(!tree.has_file_drag());
  }

  #[test]
  fn a_drop_delivers_all_its_paths_and_ends_the_polling() {
    let log = Log::default();
    let mut tree = target(&log);
    let mut shell = FileDragShell::default();
    let read = || Some((10.0_f32, 10.0_f32));
    let start = Instant::now();
    hovered(&mut shell, &["C:/a.png", "C:/b.png"]);
    shell.flush_at(&mut tree, start, &read, (0.0, 0.0));
    dropped(&mut shell, &["C:/a.png", "C:/b.png"]);

    shell.flush_at(&mut tree, start + POLL_INTERVAL, &read, (0.0, 0.0));

    assert_eq!(log.lock().unwrap().as_slice(), ["enter 2", "over 10,10", "drop 2"]);
    assert_eq!(shell.next_poll(), None);
    assert!(!tree.has_file_drag());
  }

  /// A hover and a cancel in one turn (the drag crossed the window before
  /// the loop turned) end as not hovering, with no session left to poll; a
  /// cancel and a new hover end as hovering. Arrival order decides.
  #[test]
  fn events_of_one_turn_are_delivered_in_arrival_order() {
    let log = Log::default();
    let mut tree = target(&log);
    let mut shell = FileDragShell::default();
    let read = || Some((10.0_f32, 10.0_f32));
    let start = Instant::now();
    hovered(&mut shell, &["C:/a.png"]);
    assert!(shell.on_event(&WindowEvent::HoveredFileCancelled));
    shell.flush_at(&mut tree, start, &read, (0.0, 0.0));
    assert_eq!(log.lock().unwrap().as_slice(), ["enter 1", "over 10,10", "leave"]);
    assert_eq!(shell.next_poll(), None, "hover then cancel: not hovering");
    assert!(!tree.has_file_drag());

    log.lock().unwrap().clear();
    assert!(shell.on_event(&WindowEvent::HoveredFileCancelled));
    hovered(&mut shell, &["C:/b.png"]);
    shell.flush_at(&mut tree, start + POLL_INTERVAL, &read, (0.0, 0.0));
    assert_eq!(log.lock().unwrap().as_slice(), ["enter 1", "over 10,10"]);
    assert!(shell.next_poll().is_some(), "cancel then hover: hovering");
    assert!(tree.has_file_drag());
  }

  #[test]
  fn a_hover_without_a_drop_or_a_cancel_times_out_as_a_leave() {
    let log = Log::default();
    let mut tree = target(&log);
    let mut shell = FileDragShell::default();
    let read = || Some((10.0_f32, 10.0_f32));
    let start = Instant::now();
    hovered(&mut shell, &["C:/a.png"]);
    shell.flush_at(&mut tree, start, &read, (0.0, 0.0));

    shell.flush_at(&mut tree, start + HOVER_TIMEOUT - POLL_INTERVAL, &read, (0.0, 0.0));
    assert!(shell.next_poll().is_some(), "still hovering before the timeout");
    shell.flush_at(&mut tree, start + HOVER_TIMEOUT, &read, (0.0, 0.0));

    assert_eq!(log.lock().unwrap().last().map(String::as_str), Some("leave"));
    assert_eq!(shell.next_poll(), None);
    assert!(!tree.has_file_drag());

    // A drop after the timeout still arrives: it enters again at its point.
    dropped(&mut shell, &["C:/a.png"]);
    shell.flush_at(&mut tree, start + HOVER_TIMEOUT + POLL_INTERVAL, &read, (0.0, 0.0));
    assert_eq!(log.lock().unwrap().last().map(String::as_str), Some("drop 1"));
  }

  #[test]
  fn the_last_pointer_position_the_window_saw_stands_in_where_the_os_gives_none() {
    let log = Log::default();
    let mut tree = target(&log);
    let mut shell = FileDragShell::default();
    hovered(&mut shell, &["C:/a.png"]);

    shell.flush_at(&mut tree, Instant::now(), &|| None, (40.0, 50.0));

    assert_eq!(log.lock().unwrap().as_slice(), ["enter 1", "over 40,50"]);
  }
}
