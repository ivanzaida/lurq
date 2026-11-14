//! Files a person gives the app: dropped on it from the system, or chosen in
//! the system's file dialog.
//!
//! A drop reaches an element through its `on_file_drag_enter`,
//! `on_file_drag_over`, `on_file_drag_leave` and `on_file_drop` handlers
//! (see [`FileDragEvent`](crate::app::events::FileDragEvent)); a file of it is
//! a [`DroppedFile`] with [`DroppedFile::from_path`]. [`FileDialog`] opens the
//! system's dialog for files to open and gives what was chosen as the same
//! [`DroppedFile`]s, so an app handles a drop and a choice alike.
//!
//! # What each platform gives
//!
//! - **Windows, macOS, Linux**: files dragged from Explorer, Finder or a file manager arrive with their paths; the
//!   dialog is the system's (`rfd`, with the feature `file_dialog`), and a chosen file has its path and size. On
//!   Windows the dialog is modal to the window it is opened over; on macOS and Linux it has no parent window yet, so it
//!   is not modal to the app's.
//! - **A browser**: no drop from the system reaches the page (winit reports none), and a page reads no path. The dialog
//!   is the browser's file input, which asks the person to confirm first (a press must open it); a chosen file has its
//!   name and size and no path. Its bytes are not read through lurq yet.
//! - **iOS**: no drop and no dialog: `rfd` has no backend there, so [`FileDialog::pick`] gives nothing.
//!
//! Without the feature `file_dialog`, [`FileDialog::pick`] gives nothing on
//! every platform and says so through `tracing` once. Under an MCP server with
//! a [`FileDialogBroker`](crate::mcp::FileDialogBroker) registered, the
//! dialog is the broker's: an agent answers it, and no system dialog opens.

use std::{
  future::Future,
  path::{Path, PathBuf},
  pin::Pin,
  sync::{Arc, Mutex},
  task::{Context, Poll, Waker},
};

use crate::app::WindowHandle;

/// A file a person gave the app: dropped on it, or chosen in the file dialog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DroppedFile {
  name: Arc<str>,
  size: Option<u64>,
  path: Option<PathBuf>,
}

impl DroppedFile {
  /// The file at `path`: its name is the path's last part, and its size is
  /// read from the file system once, where it can be (`None` for a path that
  /// is not there, or a folder).
  pub fn from_path(path: impl Into<PathBuf>) -> Self {
    let path = path.into();
    let size = std::fs::metadata(&path)
      .ok()
      .filter(|metadata| metadata.is_file())
      .map(|metadata| metadata.len());
    let name = path
      .file_name()
      .map(|name| name.to_string_lossy().into_owned())
      .unwrap_or_else(|| path.to_string_lossy().into_owned());
    Self {
      name: Arc::from(name),
      size,
      path: Some(path),
    }
  }

  /// A file with no path: what a browser gives, or a test's.
  pub fn new(name: &str, size: Option<u64>) -> Self {
    Self {
      name: Arc::from(name),
      size,
      path: None,
    }
  }

  /// The same file with its size said, in place of what was read.
  pub fn with_size(mut self, size: Option<u64>) -> Self {
    self.size = size;
    self
  }

  /// Its name, with its extension.
  pub fn name(&self) -> &str {
    &self.name
  }

  /// Its size in bytes, where it is known.
  pub fn size(&self) -> Option<u64> {
    self.size
  }

  /// Where it is, where the platform says: not in a browser.
  pub fn path(&self) -> Option<&Path> {
    self.path.as_deref()
  }

  /// Its extension in lower case, without the dot: `"png"`.
  pub fn extension(&self) -> Option<String> {
    let (stem, extension) = self.name.rsplit_once('.')?;
    (!stem.is_empty() && !extension.is_empty()).then(|| extension.to_ascii_lowercase())
  }

  /// What kind of file its extension says it is.
  pub fn kind(&self) -> FileKind {
    self
      .extension()
      .map(|extension| FileKind::of_extension(&extension))
      .unwrap_or(FileKind::Other)
  }

  /// Whether it is one of `accepted`: extensions (`"png"` or `".png"`) and kinds (`"image/*"`, `"text/*"`), in
  /// any case. Nothing accepted takes every file.
  pub fn is_accepted(&self, accepted: &[impl AsRef<str>]) -> bool {
    if accepted.is_empty() {
      return true;
    }
    let extension = self.extension();
    let kind = self.kind();
    accepted.iter().any(|entry| {
      let entry = entry.as_ref().trim().to_ascii_lowercase();
      match entry.strip_suffix("/*") {
        Some(kind_name) => kind.name() == kind_name,
        None => extension.as_deref() == Some(entry.trim_start_matches('.')),
      }
    })
  }
}

impl crate::app::component::DevtoolsInspectable for DroppedFile {
  fn inspect(&self, formatter: &mut crate::app::component::DevtoolsFormatter<'_>) {
    formatter.value_display(&self.name);
  }
}

/// What kind of file a file is, by its extension: what decides its icon, and
/// what `"image/*"` and the others accept.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FileKind {
  Image,
  Video,
  Audio,
  /// Words: plain text, a document, a sheet, data, code.
  Text,
  Archive,
  #[default]
  Other,
}

impl FileKind {
  /// The kind of a file with `extension`, in lower case and without the dot.
  pub fn of_extension(extension: &str) -> Self {
    match extension {
      "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "svg" | "tif" | "tiff" | "ico" | "heic" | "avif" => Self::Image,
      "mp4" | "mov" | "webm" | "mkv" | "avi" | "m4v" => Self::Video,
      "mp3" | "wav" | "ogg" | "flac" | "m4a" | "aac" | "opus" => Self::Audio,
      "txt" | "md" | "json" | "csv" | "tsv" | "xml" | "yaml" | "yml" | "toml" | "pdf" | "doc" | "docx" | "rtf"
      | "odt" | "xls" | "xlsx" | "ods" | "html" | "htm" | "log" | "rs" | "js" | "ts" | "py" | "css" => Self::Text,
      "zip" | "tar" | "gz" | "tgz" | "7z" | "rar" | "xz" | "bz2" => Self::Archive,
      _ => Self::Other,
    }
  }

  /// The name `accept` says it by, before `/*`: `"image"`.
  pub fn name(self) -> &'static str {
    match self {
      Self::Image => "image",
      Self::Video => "video",
      Self::Audio => "audio",
      Self::Text => "text",
      Self::Archive => "archive",
      Self::Other => "other",
    }
  }
}

/// A size in bytes as a person reads it: `"512 B"`, `"4 KB"`, `"1.2 MB"`.
pub fn format_size(bytes: u64) -> String {
  const UNITS: [&str; 4] = ["KB", "MB", "GB", "TB"];
  if bytes < 1024 {
    return format!("{bytes} B");
  }
  let mut value = bytes as f64 / 1024.0;
  let mut unit = 0;
  while value >= 1024.0 && unit < UNITS.len() - 1 {
    value /= 1024.0;
    unit += 1;
  }
  // One decimal under ten, where it says something: 1.2 MB, but 4 KB.
  let tenths = (value * 10.0).round() / 10.0;
  if tenths < 10.0 && tenths.fract() != 0.0 {
    format!("{tenths:.1} {}", UNITS[unit])
  } else {
    format!("{} {}", value.round() as u64, UNITS[unit])
  }
}

/// The system's dialog for files to open.
///
/// ```no_run
/// use lurq::{app::ctx::Ctx, files::FileDialog};
///
/// # fn open(ctx: &mut Ctx) {
/// let window = ctx.window();
/// let choose = ctx.future_action(move |()| {
///   FileDialog::new().title("Add images").filter("Images", &["png", "jpg"]).multiple(true).pick_result(&window)
/// });
/// // In a handler: `choose.run(())`; `choose.state()` then holds what was chosen.
/// # }
/// ```
#[derive(Clone, Debug, Default)]
pub struct FileDialog {
  title: String,
  filters: Vec<(String, Vec<String>)>,
  multiple: bool,
  folder: Option<PathBuf>,
  /// The window it is modal to, where the platform's dialog takes one.
  parent: Option<crate::app::window::parent::ParentWindow>,
}

impl FileDialog {
  /// A dialog for one file, with every file shown.
  pub fn new() -> Self {
    Self::default()
  }

  /// What its title bar says, where the system draws one.
  pub fn title(mut self, title: &str) -> Self {
    self.title = title.to_owned();
    self
  }

  /// A choice of the files it shows: `name` and the extensions it holds, without the dot. A second filter is
  /// another choice.
  pub fn filter(mut self, name: &str, extensions: &[&str]) -> Self {
    self.filters.push((
      name.to_owned(),
      extensions
        .iter()
        .map(|extension| extension.trim_start_matches('.').to_owned())
        .collect(),
    ));
    self
  }

  /// Whether several files can be chosen at once.
  pub fn multiple(mut self, multiple: bool) -> Self {
    self.multiple = multiple;
    self
  }

  /// The folder it opens in, where the system follows it.
  pub fn folder(mut self, folder: impl Into<PathBuf>) -> Self {
    self.folder = Some(folder.into());
    self
  }

  /// Opens the dialog over `window` and gives what the person chose: nothing when they cancelled, or where there
  /// is no dialog (see the [module](self)). The future does not borrow the window: it is for
  /// [`Ctx::future_action`](crate::app::ctx::Ctx::future_action), whose result is `Result<Vec<DroppedFile>, ()>`
  /// with [`FileDialog::pick_result`].
  pub fn pick(self, window: &WindowHandle) -> impl Future<Output = Vec<DroppedFile>> + Send + 'static + use<> {
    let this = self.over(window);
    #[cfg(feature = "mcp")]
    if let Some(broker) = crate::mcp::FileDialogBroker::registered() {
      use crate::mcp::{FileDialogOperation, FileDialogRequest};
      let mut request = FileDialogRequest::new(if this.multiple {
        FileDialogOperation::OpenFiles
      } else {
        FileDialogOperation::OpenFile
      });
      request.title = this.title.clone();
      request.filters = this.filters.clone();
      match broker.request(window, request) {
        Ok(Some(pending)) => {
          return Picked::Broker(Box::pin(async move {
            match pending.await {
              Ok(selection) => selection.paths.into_iter().map(DroppedFile::from_path).collect(),
              Err(_) => Vec::new(),
            }
          }));
        }
        // Registered and refused: never a second, native dialog.
        Err(_) => return Picked::Ready(Some(Vec::new())),
        Ok(None) => {}
      }
    }
    Picked::Native(native::pick(this))
  }

  /// The dialog over `window`: modal to it where the platform's dialog takes a parent (Windows), and as it was
  /// elsewhere.
  pub(crate) fn over(mut self, window: &WindowHandle) -> Self {
    self.parent = if cfg!(windows) { window.parent() } else { None };
    self
  }

  /// The window it is modal to.
  #[cfg(test)]
  pub(crate) fn parent(&self) -> Option<crate::app::window::parent::ParentWindow> {
    self.parent
  }

  /// [`pick`](Self::pick) as the `Result` a future action holds.
  pub fn pick_result(
    self,
    window: &WindowHandle,
  ) -> impl Future<Output = Result<Vec<DroppedFile>, ()>> + Send + 'static + use<> {
    let picked = self.pick(window);
    async move { Ok(picked.await) }
  }
}

/// What [`FileDialog::pick`] waits on.
enum Picked {
  #[cfg(feature = "mcp")]
  Ready(Option<Vec<DroppedFile>>),
  #[cfg(feature = "mcp")]
  Broker(Pin<Box<dyn Future<Output = Vec<DroppedFile>> + Send>>),
  Native(Answer),
}

impl Future for Picked {
  type Output = Vec<DroppedFile>;

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    match self.get_mut() {
      #[cfg(feature = "mcp")]
      Self::Ready(files) => Poll::Ready(files.take().unwrap_or_default()),
      #[cfg(feature = "mcp")]
      Self::Broker(future) => future.as_mut().poll(cx),
      Self::Native(answer) => Pin::new(answer).poll(cx),
    }
  }
}

/// The answer of a dialog that runs off the loop: given once by whatever runs the dialog, waited on by the loop.
struct Answer(Arc<Mutex<AnswerSlot>>);

#[derive(Default)]
struct AnswerSlot {
  files: Option<Vec<DroppedFile>>,
  waker: Option<Waker>,
}

impl Answer {
  fn new() -> (Self, Answering) {
    let slot = Arc::new(Mutex::new(AnswerSlot::default()));
    (Self(slot.clone()), Answering(slot))
  }

  #[cfg_attr(all(feature = "file_dialog", not(target_os = "ios")), allow(dead_code))]
  fn ready(files: Vec<DroppedFile>) -> Self {
    let (answer, answering) = Self::new();
    answering.give(files);
    answer
  }
}

impl Future for Answer {
  type Output = Vec<DroppedFile>;

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    let mut slot = self.0.lock().unwrap();
    match slot.files.take() {
      Some(files) => Poll::Ready(files),
      None => {
        slot.waker = Some(cx.waker().clone());
        Poll::Pending
      }
    }
  }
}

/// What gives an [`Answer`].
struct Answering(Arc<Mutex<AnswerSlot>>);

impl Answering {
  fn give(self, files: Vec<DroppedFile>) {
    let waker = {
      let mut slot = self.0.lock().unwrap();
      slot.files = Some(files);
      slot.waker.take()
    };
    if let Some(waker) = waker {
      waker.wake();
    }
  }
}

#[cfg(all(feature = "file_dialog", not(target_os = "ios"), not(target_arch = "wasm32")))]
mod native {
  use super::{Answer, DroppedFile, FileDialog};

  /// The system's dialog, run on a thread of its own so the loop goes on drawing; on macOS `rfd` hands it to the
  /// main thread, which the loop keeps turning.
  pub(super) fn pick(dialog: FileDialog) -> Answer {
    let (answer, answering) = Answer::new();
    std::thread::spawn(move || {
      let files = block_on(dialog_of(&dialog, dialog.multiple));
      answering.give(files);
    });
    answer
  }

  async fn dialog_of(dialog: &FileDialog, multiple: bool) -> Vec<DroppedFile> {
    let mut native = rfd::AsyncFileDialog::new();
    if !dialog.title.is_empty() {
      native = native.set_title(&dialog.title);
    }
    for (name, extensions) in &dialog.filters {
      native = native.add_filter(name, extensions);
    }
    if let Some(folder) = &dialog.folder {
      native = native.set_directory(folder);
    }
    // Over its window the dialog is modal: the window takes no input until it closes.
    if let Some(parent) = &dialog.parent {
      native = native.set_parent(parent);
    }
    let handles = if multiple {
      native.pick_files().await.unwrap_or_default()
    } else {
      native.pick_file().await.into_iter().collect()
    };
    handles
      .into_iter()
      .map(|handle| DroppedFile::from_path(handle.path()))
      .collect()
  }

  /// Runs a future to its end on this thread.
  fn block_on<F: std::future::Future>(future: F) -> F::Output {
    use std::{
      pin::pin,
      sync::Arc,
      task::{Context, Poll, Wake, Waker},
      thread::Thread,
    };
    struct Unpark(Thread);
    impl Wake for Unpark {
      fn wake(self: Arc<Self>) {
        self.0.unpark();
      }
    }
    let waker = Waker::from(Arc::new(Unpark(std::thread::current())));
    let mut cx = Context::from_waker(&waker);
    let mut future = pin!(future);
    loop {
      if let Poll::Ready(output) = future.as_mut().poll(&mut cx) {
        return output;
      }
      std::thread::park();
    }
  }
}

#[cfg(all(feature = "file_dialog", target_arch = "wasm32"))]
mod native {
  use super::{Answer, DroppedFile, FileDialog};

  /// The browser's file input, run by the page's own executor; the files cross as their names and sizes.
  pub(super) fn pick(dialog: FileDialog) -> Answer {
    let (answer, answering) = Answer::new();
    wasm_bindgen_futures::spawn_local(async move {
      let mut native = rfd::AsyncFileDialog::new();
      if !dialog.title.is_empty() {
        native = native.set_title(&dialog.title);
      }
      for (name, extensions) in &dialog.filters {
        native = native.add_filter(name, extensions);
      }
      let handles = if dialog.multiple {
        native.pick_files().await.unwrap_or_default()
      } else {
        native.pick_file().await.into_iter().collect()
      };
      let files = handles
        .into_iter()
        .map(|handle| DroppedFile::new(&handle.file_name(), Some(handle.inner().size() as u64)))
        .collect();
      answering.give(files);
    });
    answer
  }
}

#[cfg(any(not(feature = "file_dialog"), target_os = "ios"))]
mod native {
  use super::{Answer, FileDialog};

  /// No dialog here: nothing is chosen.
  pub(super) fn pick(_dialog: FileDialog) -> Answer {
    static SAID: std::sync::Once = std::sync::Once::new();
    SAID.call_once(|| {
      if cfg!(target_os = "ios") {
        tracing::warn!("lurq: there is no file dialog on iOS; FileDialog::pick gives nothing");
      } else {
        tracing::warn!("lurq: FileDialog::pick needs the feature `file_dialog`; it gives nothing without it");
      }
    });
    Answer::ready(Vec::new())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_file_is_accepted_by_its_extension_or_its_kind_in_any_case() {
    let photo = DroppedFile::new("Holiday.PNG", Some(2048));
    assert_eq!(photo.extension().as_deref(), Some("png"));
    assert_eq!(photo.kind(), FileKind::Image);
    assert!(photo.is_accepted(&["png"]));
    assert!(photo.is_accepted(&[".PNG"]));
    assert!(photo.is_accepted(&["image/*"]));
    assert!(!photo.is_accepted(&["text/*", "pdf"]));
    assert!(photo.is_accepted(&[] as &[&str]), "nothing accepted takes every file");
    let dotfile = DroppedFile::new(".gitignore", None);
    assert_eq!(
      dotfile.extension(),
      None,
      "a name that starts with its dot has no extension"
    );
    assert_eq!(dotfile.kind(), FileKind::Other);
  }

  #[cfg(windows)]
  #[test]
  fn on_windows_the_dialog_is_given_its_window_as_its_parent() {
    use raw_window_handle::{RawDisplayHandle, RawWindowHandle, Win32WindowHandle, WindowsDisplayHandle};

    use crate::app::{Tree, window::parent::ParentWindow};
    let tree = Tree::new();
    let window = tree.window().handle();
    assert_eq!(
      FileDialog::new().over(&window).parent(),
      None,
      "a window the shell did not make has none"
    );
    let hwnd = Win32WindowHandle::new(std::num::NonZeroIsize::new(0x1234).unwrap());
    let parent = ParentWindow::new(
      RawWindowHandle::Win32(hwnd),
      RawDisplayHandle::Windows(WindowsDisplayHandle::new()),
    );
    tree.window().set_parent(Some(parent));
    let dialog = FileDialog::new().title("Add files").over(&window);
    assert_eq!(
      dialog.parent().map(|parent| parent.raw().0),
      Some(RawWindowHandle::Win32(hwnd))
    );
    tree.window().set_parent(None);
    assert_eq!(
      FileDialog::new().over(&window).parent(),
      None,
      "the window gone, the dialog has none"
    );
  }

  #[test]
  fn a_size_is_said_in_the_unit_a_person_reads() {
    assert_eq!(format_size(512), "512 B");
    assert_eq!(format_size(4 * 1024), "4 KB");
    assert_eq!(format_size(40 * 1024), "40 KB");
    assert_eq!(format_size(1_258_291), "1.2 MB");
    assert_eq!(format_size(3 * 1024 * 1024 * 1024), "3 GB");
  }

  #[test]
  fn a_file_from_a_path_has_its_name_and_the_size_on_disk() {
    let dir = std::env::temp_dir().join(format!("lurq-files-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("notes.txt");
    std::fs::write(&path, b"twelve bytes").unwrap();
    let file = DroppedFile::from_path(&path);
    assert_eq!(file.name(), "notes.txt");
    assert_eq!(file.size(), Some(12));
    assert_eq!(file.path(), Some(path.as_path()));
    assert_eq!(DroppedFile::from_path(dir.join("gone.txt")).size(), None);
    assert_eq!(DroppedFile::from_path(&dir).size(), None, "a folder has no size");
    std::fs::remove_dir_all(&dir).unwrap();
  }
}
