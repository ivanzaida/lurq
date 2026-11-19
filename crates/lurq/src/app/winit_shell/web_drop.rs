//! Files dragged onto the page from the system: winit reports none in a browser, so the shell listens to the
//! canvas's own drag events and gives the tree the same file drag a desktop's window gives it
//! ([`Tree::file_drag_enter`], `over`, `leave`, [`Tree::file_drop`]).
//!
//! While files hover, the browser tells only their kinds (`image/png`), not their names: each is named
//! `dragged/<n>.<extension of its kind>`, so that what accepts by extension can say whether it takes them. On the
//! drop the files are read, and then dropped under their own names as `dropped/<n>/<name>`, their bytes held for
//! [`DroppedFile::from_path`](crate::files::DroppedFile::from_path). A drag of text or of a link is not a file
//! drag, and is left to the browser.

use std::{cell::RefCell, path::PathBuf, rc::Rc, sync::Arc};

use wasm_bindgen::{JsCast, closure::Closure};
use wasm_bindgen_futures::JsFuture;
use winit::{platform::web::WindowExtWebSys, window::Window};

use crate::app::{
  Tree,
  events::{DraggedPaths, DroppedPath},
  window::WindowWaker,
};

/// What came of a drag, in order; positions in the canvas's pixels.
enum Came {
  Over(Option<DraggedPaths>, f32, f32),
  Leave,
  Drop(DraggedPaths, f32, f32),
}

pub(super) struct WebDrop {
  came: Rc<RefCell<Vec<Came>>>,
  _listeners: Vec<Closure<dyn FnMut(web_sys::DragEvent)>>,
}

impl WebDrop {
  pub(super) fn new(window: &Window, wake: WindowWaker) -> Option<Self> {
    let canvas = window.canvas()?;
    let came = Rc::new(RefCell::new(Vec::new()));
    let mut listeners = Vec::new();
    let mut listen = |kind: &str, handler: Box<dyn FnMut(web_sys::DragEvent)>| {
      let listener = Closure::<dyn FnMut(web_sys::DragEvent)>::wrap(handler);
      let _ = canvas.add_event_listener_with_callback(kind, listener.as_ref().unchecked_ref());
      listeners.push(listener);
    };
    for kind in ["dragenter", "dragover"] {
      let (came, wake, canvas) = (came.clone(), wake.clone(), canvas.clone());
      listen(
        kind,
        Box::new(move |event: web_sys::DragEvent| {
          let Some(kinds) = file_kinds(&event) else {
            return;
          };
          // The page takes the drop: without this the browser opens the file in place of the page.
          event.prevent_default();
          if let Some(transfer) = event.data_transfer() {
            transfer.set_drop_effect("copy");
          }
          let (x, y) = at(&canvas, &event);
          let paths = (kind == "dragenter").then(|| {
            DraggedPaths::new(kinds.iter().enumerate().map(|(index, kind)| {
              DroppedPath::with_kind(PathBuf::from(format!("dragged/{index}.{}", extension_of(kind))), false)
            }))
          });
          came.borrow_mut().push(Came::Over(paths, x, y));
          wake();
        }),
      );
    }
    {
      let (came, wake, canvas) = (came.clone(), wake.clone(), canvas.clone());
      listen(
        "dragleave",
        Box::new(move |event: web_sys::DragEvent| {
          // Leaving the canvas for an element over it is no leave of the window: only for the page's outside.
          let inside = event
            .related_target()
            .and_then(|target| target.dyn_into::<web_sys::Node>().ok());
          if inside.is_some_and(|node| canvas.contains(Some(&node))) {
            return;
          }
          came.borrow_mut().push(Came::Leave);
          wake();
        }),
      );
    }
    {
      let (came, canvas) = (came.clone(), canvas.clone());
      listen(
        "drop",
        Box::new(move |event: web_sys::DragEvent| {
          let Some(files) = event.data_transfer().and_then(|transfer| transfer.files()) else {
            return;
          };
          if files.length() == 0 {
            return;
          }
          event.prevent_default();
          let (x, y) = at(&canvas, &event);
          let files: Vec<web_sys::File> = (0..files.length()).filter_map(|index| files.get(index)).collect();
          let (came, wake) = (came.clone(), wake.clone());
          wasm_bindgen_futures::spawn_local(async move {
            static DROPS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let serial = DROPS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let mut held = Vec::with_capacity(files.len());
            for file in files {
              let Ok(buffer) = JsFuture::from(file.array_buffer()).await else {
                continue;
              };
              let bytes: Arc<[u8]> = Arc::from(js_sys::Uint8Array::new(&buffer).to_vec());
              held.push((PathBuf::from(format!("dropped/{serial}/{}", file.name())), bytes));
            }
            let paths = DraggedPaths::new(held.iter().map(|(path, _)| DroppedPath::with_kind(path.clone(), false)));
            crate::files::hold_dropped(held);
            came.borrow_mut().push(Came::Drop(paths, x, y));
            wake();
          });
        }),
      );
    }
    Some(Self {
      came,
      _listeners: listeners,
    })
  }

  /// Gives the tree what came since the last turn.
  pub(super) fn give(&self, tree: &mut Tree) -> bool {
    let came = std::mem::take(&mut *self.came.borrow_mut());
    let any = !came.is_empty();
    for event in came {
      match event {
        Came::Over(Some(paths), x, y) => tree.file_drag_enter(paths, x, y),
        Came::Over(None, x, y) => tree.file_drag_over(x, y),
        Came::Leave => tree.file_drag_leave(),
        Came::Drop(paths, x, y) => {
          tree.file_drop(paths, x, y);
        }
      }
    }
    any
  }
}

/// The kinds of the files a drag holds; `None` for a drag that holds no file.
fn file_kinds(event: &web_sys::DragEvent) -> Option<Vec<String>> {
  let items = event.data_transfer()?.items();
  let kinds: Vec<String> = (0..items.length())
    .filter_map(|index| items.get(index))
    .filter(|item| item.kind() == "file")
    .map(|item| item.type_())
    .collect();
  (!kinds.is_empty()).then_some(kinds)
}

/// Where the pointer is over the canvas, in its pixels.
fn at(canvas: &web_sys::HtmlCanvasElement, event: &web_sys::DragEvent) -> (f32, f32) {
  let rect = canvas.get_bounding_client_rect();
  let ratio = web_sys::window().map_or(1.0, |window| window.device_pixel_ratio());
  (
    ((f64::from(event.client_x()) - rect.left()) * ratio) as f32,
    ((f64::from(event.client_y()) - rect.top()) * ratio) as f32,
  )
}

/// An extension for a file of `kind` (a MIME type), so that what accepts by extension can judge it while it hovers.
fn extension_of(kind: &str) -> &str {
  match kind {
    "image/jpeg" => "jpg",
    "image/svg+xml" => "svg",
    "text/plain" => "txt",
    "text/markdown" => "md",
    "audio/mpeg" => "mp3",
    "application/zip" | "application/x-zip-compressed" => "zip",
    _ => kind.rsplit_once('/').map_or("", |(_, subtype)| subtype),
  }
}
