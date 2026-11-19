//! Files a person gives the app: dropped on it from the system, or chosen in
//! the system's file dialog.
//!
//! A drop reaches an element through its `on_file_drag_enter`,
//! `on_file_drag_over`, `on_file_drag_leave` and `on_file_drop` handlers
//! (see [`FileDragEvent`](crate::app::events::FileDragEvent)); a file of it is
//! a [`DroppedFile`] with [`DroppedFile::from_path`]. [`FileDialog`] opens the
//! system's dialog for files to open and gives what was chosen as the same
//! [`DroppedFile`]s, so an app handles a drop and a choice alike. The same
//! dialog chooses a folder ([`FileDialog::pick_folder`]) and where a file is
//! to be saved ([`FileDialog::pick_save`]), each given as its path.
//!
//! # What each platform gives
//!
//! - **Windows, macOS, Linux**: files dragged from Explorer, Finder or a file manager arrive with their paths; the
//!   dialog is the system's (`rfd`, with the feature `file_dialog`), and a chosen file has its path and size. On
//!   Windows the dialog is modal to the window it is opened over; on macOS and Linux it has no parent window yet, so it
//!   is not modal to the app's.
//! - **A browser**: files dragged onto the page from the system arrive with their names (while they hover, the browser
//!   tells only their kinds, so each is named `dragged/<n>.<extension of its kind>` until it is dropped) and, on the
//!   drop, their bytes: [`DroppedFile::from_path`] of a dropped path is the file with its bytes. A page reads no path.
//!   The dialog is the browser's file input, which asks the person to confirm first (a press must open it); a chosen
//!   file has its name, its size and its bytes, read as it is chosen ([`DroppedFile::read`]), and no path. A page is
//!   given no path, so there is no dialog for a folder or for where to save: [`FileDialog::pick_folder`] and
//!   [`FileDialog::pick_save`] give nothing, and [`FileDialog::save`] hands the bytes to the browser as a download.
//! - **iOS**: no drop; the dialog is the system's document picker (`UIDocumentPickerViewController`), with or without
//!   the feature `file_dialog`. A file chosen to open is a copy the system puts in the app's temporary folder, read by
//!   its path; a folder chosen is the app's to read and write while it runs; and where to save is chosen by putting an
//!   empty file of the dialog's file name where the person says, whose path there the app writes.
//!
//! Without the feature `file_dialog`, [`FileDialog::pick`] and the others give nothing on
//! every platform and say so through `tracing` once. Under an MCP server with
//! a [`FileDialogBroker`](crate::mcp::FileDialogBroker) registered, the
//! dialog of a window of that tree is the broker's: an agent answers it, and
//! no system dialog opens. A window asks the broker of its own tree only.

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
  bytes: Option<FileBytes>,
}

/// What a file holds, where the platform gave it with the file: a browser's. Printed as its length.
#[derive(Clone, PartialEq, Eq)]
struct FileBytes(Arc<[u8]>);

impl std::fmt::Debug for FileBytes {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{} bytes", self.0.len())
  }
}

impl DroppedFile {
  /// The file at `path`: its name is the path's last part, and its size is
  /// read from the file system once, where it can be (`None` for a path that
  /// is not there, or a folder).
  pub fn from_path(path: impl Into<PathBuf>) -> Self {
    let path = path.into();
    // A file dropped on a page: its path is a name the shell gave it, and its bytes are held for it.
    #[cfg(target_arch = "wasm32")]
    if let Some(bytes) = dropped_on_the_page(&path) {
      let name = path
        .file_name()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
      return Self::new(&name, None).with_bytes(bytes);
    }
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
      bytes: None,
    }
  }

  /// A file with no path: what a browser gives, or a test's.
  pub fn new(name: &str, size: Option<u64>) -> Self {
    Self {
      name: Arc::from(name),
      size,
      path: None,
      bytes: None,
    }
  }

  /// The same file holding `bytes`: what a browser gives, which has no path to read them from. Its size is theirs.
  pub fn with_bytes(mut self, bytes: impl Into<Arc<[u8]>>) -> Self {
    let bytes = bytes.into();
    self.size = Some(bytes.len() as u64);
    self.bytes = Some(FileBytes(bytes));
    self
  }

  /// What it holds: the bytes it came with (a browser's), or its file read from its path. An error where it has
  /// neither, or the file cannot be read.
  pub fn read(&self) -> std::io::Result<Arc<[u8]>> {
    if let Some(FileBytes(bytes)) = &self.bytes {
      return Ok(bytes.clone());
    }
    match &self.path {
      Some(path) => std::fs::read(path).map(Arc::from),
      None => Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "the file has no path and came with no bytes",
      )),
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
    [Self::Image, Self::Video, Self::Audio, Self::Text, Self::Archive]
      .into_iter()
      .find(|kind| kind.extensions().contains(&extension))
      .unwrap_or(Self::Other)
  }

  /// The extensions of files of this kind, in lower case and without the dot: what a dialog shows of it. None for
  /// [`Other`](Self::Other).
  pub fn extensions(self) -> &'static [&'static str] {
    match self {
      Self::Image => &[
        "png", "jpg", "jpeg", "gif", "webp", "bmp", "svg", "tif", "tiff", "ico", "heic", "avif",
      ],
      Self::Video => &["mp4", "mov", "webm", "mkv", "avi", "m4v"],
      Self::Audio => &["mp3", "wav", "ogg", "flac", "m4a", "aac", "opus"],
      Self::Text => &[
        "txt", "md", "json", "csv", "tsv", "xml", "yaml", "yml", "toml", "pdf", "doc", "docx", "rtf", "odt", "xls",
        "xlsx", "ods", "html", "htm", "log", "rs", "js", "ts", "py", "css",
      ],
      Self::Archive => &["zip", "tar", "gz", "tgz", "7z", "rar", "xz", "bz2"],
      Self::Other => &[],
    }
  }

  /// The kind `accept` names before `/*` (`"image"`), where it is one of these.
  pub fn named(name: &str) -> Option<Self> {
    [Self::Image, Self::Video, Self::Audio, Self::Text, Self::Archive]
      .into_iter()
      .find(|kind| kind.name() == name)
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

/// A size in bytes as a person reads it: `"512 B"`, `"4 KB"`, `"1.2 MB"`, in the units of the app's locale
/// ([`FieldWords::size`](crate::core::FieldWords::size)) where it is said in a render, in English outside one.
pub fn format_size(bytes: u64) -> String {
  crate::ui::current_locale().fields.size(bytes)
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
  /// The name a dialog for where to save starts with.
  file_name: Option<String>,
  /// What it chooses: set by the method that opens it.
  asks: Asks,
  /// The window it is modal to, where the platform's dialog takes one.
  parent: Option<crate::app::window::parent::ParentWindow>,
  /// Where the files are chosen from, as the app said: empty for what the filters take.
  sources: Vec<FileSource>,
  /// The words of the menu of sources on a phone, in the locale of the render it was made in.
  #[cfg_attr(not(target_os = "ios"), allow(dead_code))]
  words: Option<SourceWords>,
}

/// Where the files of a dialog to open are chosen from: what a phone offers in its menu before the dialog. Each
/// system offers of them what it has (see [`FileDialog::sources`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FileSource {
  /// The files of the system: Files on a phone, the dialog of files on a desktop.
  Files,
  /// The photo library: its pictures and videos, as the dialog's filters take them.
  PhotoLibrary,
  /// A picture taken with the camera, there and then. A phone's alone: there is none on a desktop.
  #[cfg(any(target_os = "ios", target_os = "android"))]
  Camera,
}

/// The words of a phone's menu of sources.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SourceWords {
  pub(crate) photo_library: Arc<str>,
  pub(crate) take_photo: Arc<str>,
  pub(crate) choose_file: Arc<str>,
  pub(crate) cancel: Arc<str>,
}

impl SourceWords {
  fn of(locale: &crate::core::Locale) -> Self {
    Self {
      photo_library: locale.fields.photo_library.clone(),
      take_photo: locale.fields.take_photo.clone(),
      choose_file: locale.fields.choose_file.clone(),
      cancel: locale.common.cancel.clone(),
    }
  }
}

/// What a dialog chooses.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Asks {
  /// Files to open.
  #[default]
  Files,
  /// A folder.
  Folder,
  /// Where a file is to be saved.
  Save,
}

impl FileDialog {
  /// A dialog for one file, with every file shown. Made in a render, it says what it says in the app's locale.
  pub fn new() -> Self {
    Self {
      words: Some(SourceWords::of(&crate::ui::current_locale())),
      ..Self::default()
    }
  }

  /// Where the files are chosen from, in place of what the filters take. On a phone the menu before the dialog has
  /// these, of what the device has (the camera where it has one and the app's Info.plist says why it uses it), and
  /// one alone opens at once. On a desktop, which has no camera to choose from ([`FileSource::Camera`] is a phone's
  /// alone): on macOS the photo library of Photos where it is the one source, on Windows the dialog of files opened
  /// in the user's Pictures where the photo library is among them; the dialog of files otherwise.
  pub fn sources(mut self, sources: impl IntoIterator<Item = FileSource>) -> Self {
    self.sources = sources.into_iter().collect();
    self
  }

  /// The sources the dialog offers, of `has`, the ones the system has: the app's, or what the filters take.
  #[cfg_attr(not(any(target_os = "ios", target_os = "macos", windows)), allow(dead_code))]
  pub(crate) fn offered(&self, has: &[FileSource]) -> Vec<FileSource> {
    let wanted = if self.sources.is_empty() {
      let extensions: Vec<&str> = self
        .filters
        .iter()
        .flat_map(|(_, extensions)| extensions.iter())
        .map(String::as_str)
        .collect();
      let takes = |kind: FileKind| {
        extensions.is_empty()
          || extensions
            .iter()
            .any(|extension| FileKind::of_extension(extension) == kind)
      };
      let mut wanted = Vec::new();
      if takes(FileKind::Image) || takes(FileKind::Video) {
        wanted.push(FileSource::PhotoLibrary);
      }
      #[cfg(any(target_os = "ios", target_os = "android"))]
      if takes(FileKind::Image) {
        wanted.push(FileSource::Camera);
      }
      wanted.push(FileSource::Files);
      wanted
    } else {
      self.sources.clone()
    };
    let mut offered: Vec<FileSource> = Vec::new();
    for source in wanted {
      if has.contains(&source) && !offered.contains(&source) {
        offered.push(source);
      }
    }
    if offered.is_empty() {
      offered.push(FileSource::Files);
    }
    offered
  }

  /// Whether the filters take pictures, and videos: what the photo library shows.
  #[cfg_attr(
    not(any(target_os = "ios", all(target_os = "macos", feature = "file_dialog"))),
    allow(dead_code)
  )]
  pub(crate) fn takes_media(&self) -> (bool, bool) {
    let extensions: Vec<&str> = self
      .filters
      .iter()
      .flat_map(|(_, extensions)| extensions.iter())
      .map(String::as_str)
      .collect();
    let takes = |kind: FileKind| {
      extensions.is_empty()
        || extensions
          .iter()
          .any(|extension| FileKind::of_extension(extension) == kind)
    };
    (takes(FileKind::Image), takes(FileKind::Video))
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

  /// The dialog opening in `folder`, where it was given no folder of its own.
  pub(crate) fn folder_unless_given(mut self, folder: impl Into<PathBuf>) -> Self {
    if self.folder.is_none() {
      self.folder = Some(folder.into());
    }
    self
  }

  /// The name a dialog for where to save ([`pick_save`](Self::pick_save)) starts with, where the system follows
  /// it.
  pub fn file_name(mut self, name: &str) -> Self {
    self.file_name = Some(name.to_owned());
    self
  }

  /// Opens the dialog over `window` and gives what the person chose: nothing when they cancelled, or where there
  /// is no dialog (see the [module](self)). The future does not borrow the window: it is for
  /// [`Ctx::future_action`](crate::app::ctx::Ctx::future_action), whose result is `Result<Vec<DroppedFile>, ()>`
  /// with [`FileDialog::pick_result`].
  pub fn pick(mut self, window: &WindowHandle) -> impl Future<Output = Vec<DroppedFile>> + Send + 'static + use<> {
    self.asks = Asks::Files;
    self.ask(window)
  }

  /// Opens the dialog for a folder over `window` and gives the folder the person chose: `None` when they
  /// cancelled, or where there is no such dialog (a browser, a build without `file_dialog` outside iOS). Its title and
  /// the [folder](Self::folder) it opens in are the dialog's; its filters say nothing of folders.
  ///
  /// ```no_run
  /// use lurq::{app::ctx::Ctx, files::FileDialog};
  ///
  /// # fn open(ctx: &mut Ctx) {
  /// let window = ctx.window();
  /// let choose = ctx.future_action(move |()| {
  ///   let folder = FileDialog::new().title("Project folder").pick_folder(&window);
  ///   async move { Ok::<_, ()>(folder.await) }
  /// });
  /// // In a handler: `choose.run(())`; `choose.state()` then holds the folder, or `None`.
  /// # }
  /// ```
  pub fn pick_folder(
    mut self,
    window: &WindowHandle,
  ) -> impl Future<Output = Option<PathBuf>> + Send + 'static + use<> {
    self.asks = Asks::Folder;
    let asked = self.ask(window);
    async move { path_of(asked.await) }
  }

  /// Opens the dialog for where to save a file over `window` and gives the path the person chose: `None` when
  /// they cancelled, or where there is no such dialog. It starts in its [folder](Self::folder) with its
  /// [file name](Self::file_name), and its filters are the kinds the file can be saved as. The system asks before
  /// a file that is there is chosen; nothing is written by lurq.
  pub fn pick_save(mut self, window: &WindowHandle) -> impl Future<Output = Option<PathBuf>> + Send + 'static + use<> {
    self.asks = Asks::Save;
    let asked = self.ask(window);
    async move { path_of(asked.await) }
  }

  /// Saves `bytes` as a file where the person says. On a desktop and on iOS it opens the dialog for where to save
  /// ([`pick_save`](Self::pick_save)) and writes the file there; in a browser the bytes are a download of the
  /// dialog's [file name](Self::file_name), which the browser saves where it saves downloads, or asks where, as the
  /// person set it. `true` once the file is written, or handed to the browser; `false` when the person cancelled or
  /// it could not be written.
  pub fn save(self, window: &WindowHandle, bytes: Arc<[u8]>) -> impl Future<Output = bool> + Send + 'static + use<> {
    #[cfg(target_arch = "wasm32")]
    {
      let _ = window;
      let name = self.file_name.clone().unwrap_or_else(|| "download".to_owned());
      let handed = download(&name, &bytes);
      return std::future::ready(handed);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
      let chosen = self.pick_save(window);
      async move {
        match chosen.await {
          Some(path) => std::fs::write(path, &bytes).is_ok(),
          None => false,
        }
      }
    }
  }

  /// Opens the dialog for what it asks, over `window`: the broker's where the window's tree registered one
  /// with its MCP server, the system's otherwise. A folder and a path to save at come back as one file with that path.
  fn ask(self, window: &WindowHandle) -> Picked {
    let this = self.over(window);
    #[cfg(feature = "mcp")]
    if let Some(broker) = window.file_dialog_broker() {
      use crate::mcp::{FileDialogOperation, FileDialogRequest};
      let mut request = FileDialogRequest::new(match this.asks {
        Asks::Files if this.multiple => FileDialogOperation::OpenFiles,
        Asks::Files => FileDialogOperation::OpenFile,
        Asks::Folder => FileDialogOperation::OpenFolder,
        Asks::Save => FileDialogOperation::SaveFile,
      });
      request.title = this.title.clone();
      request.filters = this.filters.clone();
      request.suggested_name = this.file_name.clone().filter(|_| this.asks == Asks::Save);
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

/// The path of the one thing a dialog for a folder or for where to save chose.
#[cfg(target_arch = "wasm32")]
thread_local! {
  /// The files of the last drop on the page, by the paths the shell gave them (`dropped/<n>/<name>`).
  static DROPPED: std::cell::RefCell<Vec<(PathBuf, Arc<[u8]>)>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Holds the files of a drop on the page, in place of the last drop's: what [`DroppedFile::from_path`] finds.
#[cfg(target_arch = "wasm32")]
pub(crate) fn hold_dropped(files: Vec<(PathBuf, Arc<[u8]>)>) {
  DROPPED.with(|dropped| *dropped.borrow_mut() = files);
}

#[cfg(target_arch = "wasm32")]
fn dropped_on_the_page(path: &Path) -> Option<Arc<[u8]>> {
  DROPPED.with(|dropped| {
    dropped
      .borrow()
      .iter()
      .find(|(held, _)| held == path)
      .map(|(_, bytes)| bytes.clone())
  })
}

/// Hands `bytes` to the browser as a download named `name`: a link to a blob of them, pressed and let go.
#[cfg(target_arch = "wasm32")]
fn download(name: &str, bytes: &[u8]) -> bool {
  use wasm_bindgen::JsCast;
  let parts = js_sys::Array::of1(&js_sys::Uint8Array::from(bytes));
  let Ok(blob) = web_sys::Blob::new_with_u8_array_sequence(&parts) else {
    return false;
  };
  let Ok(address) = web_sys::Url::create_object_url_with_blob(&blob) else {
    return false;
  };
  let handed = web_sys::window()
    .and_then(|window| window.document())
    .and_then(|document| document.create_element("a").ok())
    .and_then(|link| link.dyn_into::<web_sys::HtmlAnchorElement>().ok())
    .map(|link| {
      link.set_href(&address);
      link.set_download(name);
      link.click();
    })
    .is_some();
  // The browser has taken the blob by the time the press returns: it is let go a moment later all the same.
  let later = wasm_bindgen::closure::Closure::once_into_js(move || {
    let _ = web_sys::Url::revoke_object_url(&address);
  });
  if let Some(window) = web_sys::window() {
    let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(later.unchecked_ref(), 10_000);
  }
  handed
}

fn path_of(chosen: Vec<DroppedFile>) -> Option<PathBuf> {
  chosen.into_iter().next().and_then(|file| file.path)
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

  #[cfg_attr(
    any(target_os = "ios", all(feature = "file_dialog", not(target_arch = "wasm32"))),
    allow(dead_code)
  )]
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
  use super::{Answer, Asks, DroppedFile, FileDialog};

  /// The system's dialog, run on a thread of its own so the loop goes on drawing; on macOS `rfd` hands it to the
  /// main thread, which the loop keeps turning. On macOS the photo library of Photos, where it is the one source.
  pub(super) fn pick(dialog: FileDialog) -> Answer {
    #[cfg(target_os = "macos")]
    if dialog.asks == Asks::Files
      && dialog.offered(&[super::FileSource::PhotoLibrary, super::FileSource::Files])
        == [super::FileSource::PhotoLibrary]
    {
      return super::photos::pick(dialog);
    }
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
    } else if let Some(pictures) = pictures_for(dialog) {
      native = native.set_directory(pictures);
    }
    // Over its window the dialog is modal: the window takes no input until it closes.
    if let Some(parent) = &dialog.parent {
      native = native.set_parent(parent);
    }
    let handles = match dialog.asks {
      Asks::Files if multiple => native.pick_files().await.unwrap_or_default(),
      Asks::Files => native.pick_file().await.into_iter().collect(),
      Asks::Folder => native.pick_folder().await.into_iter().collect(),
      Asks::Save => {
        if let Some(name) = &dialog.file_name {
          native = native.set_file_name(name);
        }
        native.save_file().await.into_iter().collect()
      }
    };
    handles
      .into_iter()
      .map(|handle| DroppedFile::from_path(handle.path()))
      .collect()
  }

  /// On Windows, the user's Pictures, where files to open come from the photo library, which a desktop has as a
  /// folder.
  fn pictures_for(dialog: &FileDialog) -> Option<std::path::PathBuf> {
    if !cfg!(windows) || dialog.asks != Asks::Files {
      return None;
    }
    let offered = dialog.offered(&[super::FileSource::PhotoLibrary, super::FileSource::Files]);
    if !offered.contains(&super::FileSource::PhotoLibrary) {
      return None;
    }
    let folder = std::path::PathBuf::from(std::env::var_os("USERPROFILE")?).join("Pictures");
    folder.is_dir().then_some(folder)
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
  use super::{Answer, Asks, DroppedFile, FileDialog};

  /// The browser's file input, run by the page's own executor; the files cross with their names and bytes, read as
  /// they are chosen. A page is given no path: there is no dialog for a folder or for where to save.
  pub(super) fn pick(dialog: FileDialog) -> Answer {
    if dialog.asks != Asks::Files {
      return Answer::ready(Vec::new());
    }
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
      let mut files = Vec::with_capacity(handles.len());
      for handle in handles {
        let name = handle.file_name();
        files.push(DroppedFile::new(&name, None).with_bytes(handle.read().await));
      }
      answering.give(files);
    });
    answer
  }
}

#[cfg(target_os = "ios")]
#[path = "files_ios.rs"]
mod native;

/// The photo library as a source of files: iOS's, and macOS's with the system's dialogs (`file_dialog`).
#[cfg(any(target_os = "ios", all(target_os = "macos", feature = "file_dialog")))]
#[path = "files_photos.rs"]
mod photos;

/// Runs `work` on the main thread, as the main queue gets to it: where AppKit's and UIKit's pickers are shown.
#[cfg(any(target_os = "ios", all(target_os = "macos", feature = "file_dialog")))]
fn on_main(work: impl FnOnce(objc2::MainThreadMarker) + Send + 'static) {
  use std::ffi::c_void;
  unsafe extern "C" {
    /// The main queue of libdispatch, which `dispatch_get_main_queue()` returns.
    static _dispatch_main_q: c_void;
    fn dispatch_async_f(queue: *const c_void, context: *mut c_void, work: extern "C" fn(*mut c_void));
  }
  type Work = Box<dyn FnOnce(objc2::MainThreadMarker) + Send>;
  extern "C" fn run(context: *mut c_void) {
    // SAFETY: `context` is the box made below, given to this call once.
    let work = unsafe { Box::from_raw(context.cast::<Work>()) };
    if let Some(main) = objc2::MainThreadMarker::new() {
      work(main);
    }
  }
  let context = Box::into_raw(Box::new(Box::new(work) as Work)).cast::<c_void>();
  // SAFETY: the main queue lives as long as the process; `run` takes the box back exactly once.
  unsafe { dispatch_async_f(&raw const _dispatch_main_q, context, run) };
}

#[cfg(all(not(feature = "file_dialog"), not(target_os = "ios")))]
mod native {
  use super::{Answer, FileDialog};

  /// No dialog here: nothing is chosen.
  pub(super) fn pick(_dialog: FileDialog) -> Answer {
    static SAID: std::sync::Once = std::sync::Once::new();
    SAID.call_once(|| {
      tracing::warn!("lurq: FileDialog::pick needs the feature `file_dialog`; it gives nothing without it");
    });
    Answer::ready(Vec::new())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn the_sources_offered_are_what_the_filters_take_or_what_the_app_said_of_what_the_system_has() {
    use FileSource::{Files, PhotoLibrary};
    let has = [PhotoLibrary, Files];
    let images = FileDialog::new().filter("Images", &["png", "heic"]);
    assert_eq!(
      images.offered(&has),
      [PhotoLibrary, Files],
      "pictures: the photo library too"
    );
    assert_eq!(
      FileDialog::new().offered(&has),
      [PhotoLibrary, Files],
      "anything: the photo library too"
    );
    let documents = FileDialog::new().filter("PDF", &["pdf"]);
    assert_eq!(documents.offered(&has), [Files], "no picture nor video: Files alone");
    assert_eq!(
      images.clone().sources([PhotoLibrary]).offered(&has),
      [PhotoLibrary],
      "the app's"
    );
    assert_eq!(
      images.sources([PhotoLibrary]).offered(&[Files]),
      [Files],
      "what the system has not: Files"
    );
    let videos = FileDialog::new().filter("Videos", &["mov"]);
    assert_eq!(videos.takes_media(), (false, true));
  }

  #[test]
  fn a_drop_zones_dialog_shows_the_extensions_of_the_kinds_it_takes() {
    let dialog = crate::ui::DropZone::new()
      .accept(["image/*", ".PDF"])
      .takes()
      .dialog("Choose");
    let [(name, extensions)] = dialog.filters.as_slice() else {
      panic!("one filter: {:?}", dialog.filters);
    };
    assert_eq!(name, "image/*, .PDF");
    assert!(
      extensions.iter().any(|extension| extension == "heic"),
      "the kind's extensions"
    );
    assert!(
      extensions.iter().any(|extension| extension == "pdf"),
      "an extension, in lower case without its dot"
    );
    assert!(!extensions.iter().any(|extension| extension == "mp4"));
    // A kind lurq knows none of shows every file.
    let every = crate::ui::DropZone::new().accept(["model/*"]).takes().dialog("Choose");
    assert!(every.filters.is_empty());
    assert_eq!(FileKind::of_extension("heic"), FileKind::Image);
    assert_eq!(FileKind::named("audio"), Some(FileKind::Audio));
  }

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

  #[test]
  fn a_folder_or_a_place_to_save_is_the_path_of_the_one_thing_chosen() {
    assert_eq!(path_of(Vec::new()), None, "cancelled");
    assert_eq!(
      path_of(vec![DroppedFile::new("notes.md", None)]),
      None,
      "a browser's file has no path"
    );
    let chosen = path_of(vec![DroppedFile::from_path("/projects/lurq")]);
    assert_eq!(chosen.as_deref(), Some(Path::new("/projects/lurq")));
  }

  #[cfg(not(feature = "file_dialog"))]
  #[test]
  fn without_the_feature_a_folder_and_a_place_to_save_are_not_chosen() {
    use std::{pin::pin, task::Waker};

    let tree = crate::app::Tree::new();
    let window = tree.window().handle();
    let mut cx = Context::from_waker(Waker::noop());
    let folder = pin!(FileDialog::new().title("Project folder").pick_folder(&window));
    assert_eq!(folder.poll(&mut cx), Poll::Ready(None));
    let save = pin!(FileDialog::new().file_name("notes.md").pick_save(&window));
    assert_eq!(save.poll(&mut cx), Poll::Ready(None));
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
