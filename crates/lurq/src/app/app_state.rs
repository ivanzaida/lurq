use std::{
  path::{Path, PathBuf},
  sync::Arc,
};

use parking_lot::Mutex;

#[cfg(feature = "i18n")]
use crate::app::i18n::I18n;
use crate::app::{glyph_engine::GlyphEngine, theme::Theme};

/// A request to open a new OS window that renders its own component tree.
/// Created through [`WindowOpener::open`], drained by the runtime each loop
/// turn (the winit shell then creates the OS window and renders the tree with
/// the same render-engine factory as the main window).
pub struct SecondaryWindowRequest {
  pub(crate) title: String,
  pub(crate) width: u32,
  pub(crate) height: u32,
  /// `false` creates the OS window without native decorations, for trees that
  /// render their own `WindowChrome`.
  pub(crate) decorations: bool,
  /// `true` keeps the OS window over the other windows of the desktop, the
  /// ones of other apps among them: a picture in picture.
  pub(crate) always_on_top: bool,
  /// Optional stable machine name (titles aren't unique) surfaced to external
  /// tooling such as the MCP `lurq_windows` tool.
  pub(crate) name: Option<String>,
  // The type is read once, where it is used; an alias would only add a name.
  #[allow(clippy::type_complexity)]
  pub(crate) build: Box<dyn FnOnce(&mut App, &mut crate::app::Tree) + Send>,
}

/// Options for [`WindowOpener::open_with`] — the builder-style variant of
/// [`WindowOpener::open`] for windows that need a name or no decorations.
pub struct WindowOptions {
  title: String,
  width: u32,
  height: u32,
  decorations: bool,
  always_on_top: bool,
  name: Option<String>,
}

impl WindowOptions {
  pub fn new(title: impl Into<String>, width: u32, height: u32) -> Self {
    Self {
      title: title.into(),
      width,
      height,
      decorations: true,
      always_on_top: false,
      name: None,
    }
  }

  /// Keep the OS window over the other windows of the desktop, the ones of
  /// other apps among them, whether or not it has the focus: a picture in
  /// picture, a palette. Where the platform has no such level (a phone, a
  /// browser) the window is an ordinary one.
  pub fn always_on_top(mut self) -> Self {
    self.always_on_top = true;
    self
  }

  /// Create the OS window without native decorations (for trees that render
  /// their own `WindowChrome`).
  pub fn undecorated(mut self) -> Self {
    self.decorations = false;
    self
  }

  /// Stable machine name for external tooling to target this window by.
  pub fn window_name(mut self, name: impl Into<String>) -> Self {
    self.name = Some(name.into());
    self
  }
}

/// Cloneable handle for opening secondary windows from anywhere — including
/// event handlers, which don't have `Ctx` access. Obtain it via
/// `ctx.window_opener()` (or [`App::window_opener`]).
#[derive(Clone, Default)]
pub struct WindowOpener {
  queue: std::sync::Arc<std::sync::Mutex<Vec<SecondaryWindowRequest>>>,
}

impl WindowOpener {
  /// Queue a new secondary window with the given title and logical size.
  /// `build` mounts the window's root component into a fresh [`crate::app::Tree`]:
  ///
  /// ```ignore
  /// opener.open("Preview", 1100, 800, move |app, tree| {
  ///   tree.mount_root::<PreviewWindow>(app, props);
  /// });
  /// ```
  pub fn open<F>(&self, title: impl Into<String>, width: u32, height: u32, build: F)
  where
    F: FnOnce(&mut App, &mut crate::app::Tree) + Send + 'static,
  {
    self.request(title, width, height, true, build);
  }

  /// Like [`open`](Self::open), but the OS window is created without native
  /// decorations — for windows that render their own `WindowChrome`.
  pub fn open_undecorated<F>(&self, title: impl Into<String>, width: u32, height: u32, build: F)
  where
    F: FnOnce(&mut App, &mut crate::app::Tree) + Send + 'static,
  {
    self.request(title, width, height, false, build);
  }

  /// Queue a secondary window described by [`WindowOptions`], for windows
  /// that need a stable name or no decorations.
  pub fn open_with<F>(&self, options: WindowOptions, build: F)
  where
    F: FnOnce(&mut App, &mut crate::app::Tree) + Send + 'static,
  {
    self.queue.lock().unwrap().push(SecondaryWindowRequest {
      title: options.title,
      width: options.width,
      height: options.height,
      decorations: options.decorations,
      always_on_top: options.always_on_top,
      name: options.name,
      build: Box::new(build),
    });
  }

  fn request<F>(&self, title: impl Into<String>, width: u32, height: u32, decorations: bool, build: F)
  where
    F: FnOnce(&mut App, &mut crate::app::Tree) + Send + 'static,
  {
    self.queue.lock().unwrap().push(SecondaryWindowRequest {
      title: title.into(),
      width,
      height,
      decorations,
      always_on_top: false,
      name: None,
      build: Box::new(build),
    });
  }

  pub(crate) fn take(&self) -> Vec<SecondaryWindowRequest> {
    std::mem::take(&mut self.queue.lock().unwrap())
  }
}

/// A handle to shared application services. Moving or cloning it does not move
/// the state used by mounted components; trees retain that state while needed.
#[derive(Clone)]
pub struct App {
  pub(crate) shared: Arc<AppShared>,
}

pub(crate) struct AppShared {
  pub(crate) menu: super::menu::MenuController,
  pub(crate) glyph_engine: Mutex<GlyphEngine>,
  pub(crate) theme: Theme,
  /// How dates are written and read, the app's choice: [`App::set_date_locale`].
  pub(crate) dates: crate::core::AppDateText,
  /// Every word lurq writes by itself, the app's choice: [`App::set_locale`].
  pub(crate) locale: crate::core::AppLocale,
  /// What the person points with: [`App::pointer`].
  pub(crate) pointer: super::pointer::AppPointer,
  pub(crate) window_opener: WindowOpener,
  #[cfg(feature = "i18n")]
  pub(crate) i18n: I18n,
  pub(crate) scale_override: Mutex<Option<f32>>,
  #[cfg(feature = "tokio")]
  pub(crate) tokio_handle: Mutex<Option<tokio::runtime::Handle>>,
  /// What reads the sources of everything that loads; see [`App::loader`].
  pub(crate) loader: crate::resources::Loader,
  #[cfg(feature = "http")]
  pub(crate) resource_loader: Mutex<crate::resources::ResourceLoader>,
  #[cfg(feature = "persistent_storage")]
  pub(crate) persistent_storage: Mutex<crate::persistent_storage::PersistentStorage>,
  #[cfg(all(feature = "image", feature = "http"))]
  pub(crate) image_resource_cache: Mutex<std::collections::HashMap<std::sync::Arc<str>, crate::images::ImageData>>,
  #[cfg(all(feature = "svg", feature = "http"))]
  pub(crate) svg_resource_cache: Mutex<std::collections::HashMap<std::sync::Arc<str>, crate::svg::SvgData>>,
}

impl Default for App {
  fn default() -> Self {
    Self::new()
  }
}

impl App {
  pub fn new() -> Self {
    Self {
      shared: Arc::new(AppShared {
        menu: super::menu::MenuController::default(),
        glyph_engine: Mutex::new(GlyphEngine::new()),
        theme: Theme::new(),
        dates: crate::core::AppDateText::new(),
        locale: crate::core::AppLocale::new(),
        pointer: super::pointer::AppPointer::new(),
        window_opener: WindowOpener::default(),
        #[cfg(feature = "i18n")]
        i18n: I18n::new(),
        scale_override: Mutex::new(None),
        #[cfg(feature = "tokio")]
        tokio_handle: Mutex::new(None),
        loader: crate::resources::Loader::new(),
        #[cfg(feature = "http")]
        resource_loader: Mutex::new(crate::resources::ResourceLoader::new()),
        #[cfg(feature = "persistent_storage")]
        persistent_storage: Mutex::new(crate::persistent_storage::PersistentStorage::memory()),
        #[cfg(all(feature = "image", feature = "http"))]
        image_resource_cache: Mutex::new(std::collections::HashMap::new()),
        #[cfg(all(feature = "svg", feature = "http"))]
        svg_resource_cache: Mutex::new(std::collections::HashMap::new()),
      }),
    }
  }

  pub fn menu_bar_support(&self) -> super::menu::MenuBarSupport {
    if cfg!(all(feature = "winit", target_os = "macos")) {
      super::menu::MenuBarSupport::Native
    } else {
      super::menu::MenuBarSupport::Unavailable
    }
  }
  /// Retains the model everywhere for headless QA; installs a native bar only on macOS.
  pub fn set_menu_bar(&self, bar: super::menu::MenuBar) {
    self.shared.menu.set(bar);
  }
  pub fn on_menu_activate(&self, handler: impl Fn(&str) + Send + Sync + 'static) {
    self.shared.menu.on_activate(handler);
  }
  pub fn menu_controller(&self) -> super::menu::MenuController {
    self.shared.menu.clone()
  }

  pub fn set_scale_override(&mut self, scale: Option<f32>) {
    *self.shared.scale_override.lock() = scale;
    self.shared.glyph_engine.lock().clear_cache();
  }

  /// Gives the app its tokio runtime: the components' async tasks run on it, and so do the reads of the app's
  /// loader ([`Loader::tokio_handle`](crate::resources::Loader::tokio_handle)), which then starts no threads
  /// of its own.
  #[cfg(feature = "tokio")]
  pub fn with_tokio_handle(self, handle: tokio::runtime::Handle) -> Self {
    self.shared.loader.tokio_handle(handle.clone());
    *self.shared.tokio_handle.lock() = Some(handle);
    self
  }

  #[cfg(feature = "tokio")]
  pub fn set_tokio_handle(&mut self, handle: tokio::runtime::Handle) {
    self.shared.loader.tokio_handle(handle.clone());
    *self.shared.tokio_handle.lock() = Some(handle);
  }

  /// Takes the runtime away, before it is stopped: the app's loader reads on threads of its own again.
  #[cfg(feature = "tokio")]
  pub fn clear_tokio_handle(&mut self) {
    self.shared.loader.reset_tokio_handle();
    *self.shared.tokio_handle.lock() = None;
  }

  #[cfg(feature = "tokio")]
  pub(crate) fn tokio_handle(&self) -> Option<tokio::runtime::Handle> {
    self.shared.tokio_handle.lock().clone()
  }

  pub fn theme(&self) -> &Theme {
    &self.shared.theme
  }

  /// The app's locale: every word lurq writes by itself (the names of a player's buttons, what a media that cannot
  /// play says, a calendar's months), in the app's language. A [`Locale`](crate::core::Locale) of a tag lurq knows
  /// (`Locale::for_tag("uk")`), with whatever the app changed in it. English until it is set: the system's language
  /// and the i18n locale do not change it. What reads it in a render is rendered again when it changes.
  ///
  /// It sets how dates are written too ([`Locale::dates`](crate::core::Locale::dates)): an app that writes them
  /// its own way calls [`set_date_locale`](Self::set_date_locale) after it.
  ///
  /// ```
  /// use lurq::{app::App, core::Locale};
  ///
  /// let app = App::new();
  /// app.set_locale(Locale::for_tag("uk"));
  /// ```
  pub fn set_locale(&self, locale: crate::core::Locale) {
    self
      .shared
      .dates
      .set(crate::core::shared_date_text(locale.dates.clone()));
    self.shared.menu.set_words(locale.navigation.clone());
    self.shared.locale.set(locale);
  }

  /// What [`Self::set_locale`] set, English before: read in a render, it renders that again when it changes.
  pub fn locale(&self) -> std::sync::Arc<crate::core::Locale> {
    self.shared.locale.get()
  }

  /// What the person points with: a finger ([`Pointer::Coarse`](super::Pointer::Coarse)) or a mouse, a trackpad or
  /// a pen ([`Pointer::Fine`](super::Pointer::Fine)). It starts as the platform says
  /// ([`Pointer::platform_default`](super::Pointer::platform_default)) and, unless the app pins it, follows the
  /// last input: a finger makes it coarse, a mouse or a trackpad that moves makes it fine, a pen leaves it. Read in
  /// a render, it renders that again when it changes. A component dressed for a finger asks
  /// [`ui::touch_form`](crate::ui::touch_form).
  pub fn pointer(&self) -> super::Pointer {
    self.shared.pointer.get()
  }

  /// Pins what the person points with: input no longer moves it, and everything that reads it is rendered again,
  /// an open lightbox too. A kiosk with a touch screen pins [`Pointer::Coarse`](super::Pointer::Coarse); a test
  /// or a preview of the phone's forms on a desktop either. [`follow_pointer(true)`](Self::follow_pointer) lets go.
  ///
  /// ```
  /// use lurq::app::{App, Pointer};
  ///
  /// let app = App::new();
  /// app.set_pointer(Pointer::Coarse);
  /// assert_eq!(app.pointer(), Pointer::Coarse);
  /// ```
  pub fn set_pointer(&self, pointer: super::Pointer) {
    self.shared.pointer.tell(pointer, true);
  }

  /// Whether the last input moves the pointer: on unless told. `false` keeps it where it is; `true` follows input
  /// again, and lets go of what [`set_pointer`](Self::set_pointer) pinned.
  pub fn follow_pointer(&self, follows: bool) {
    self.shared.pointer.follow(follows);
  }

  /// How every calendar and date picker of the app writes and reads dates, and names its months, its days and its
  /// controls, where the component is given no `.locale(..)` of its own: a
  /// [`DateLocale`](crate::core::DateLocale) (`DateLocale::for_tag("uk")`, or one with a style or a format of the
  /// app's), or the app's own [`DateText`](crate::core::DateText), or an `Arc` of it. English until it is set: the
  /// system's language and the i18n locale do not change it. A mounted calendar or picker, and one built in a
  /// component's render, is built again when it changes.
  ///
  /// ```
  /// use lurq::{app::App, core::{DateLocale, DateStyle}};
  ///
  /// let app = App::new();
  /// app.set_date_locale(DateLocale::for_tag("en-GB").with_style(DateStyle::Long));
  /// ```
  ///
  /// An app that writes dates in its i18n locale sets it where it changes that locale:
  /// `app.set_date_locale(DateLocale::for_tag(tag))`.
  pub fn set_date_locale(&self, locale: impl crate::core::DateText + 'static) {
    self.shared.dates.set(crate::core::shared_date_text(locale));
  }

  /// What [`Self::set_date_locale`] set, English before: read in a render, it renders that again when it changes.
  pub fn date_locale(&self) -> std::sync::Arc<dyn crate::core::DateText> {
    self.shared.dates.get()
  }

  /// Handle for opening secondary OS windows (see [`WindowOpener`]).
  pub fn window_opener(&self) -> WindowOpener {
    self.shared.window_opener.clone()
  }

  #[cfg(feature = "i18n")]
  pub fn i18n(&self) -> &I18n {
    &self.shared.i18n
  }

  pub fn load_font(&mut self, data: Vec<u8>) {
    self.shared.glyph_engine.lock().load_font(data);
  }

  /// Loads a font whose glyphs are drawn grid-fitted: at the nearest whole pixel size
  /// and position, with the font's TrueType hints applied along both axes.
  ///
  /// This is for an icon font hinted for it. Text is drawn unhinted, so that a letter
  /// looks the same wherever on the line it falls; load a text font with
  /// [`Self::load_font`]. A glyph of a hinted font under a transform, or in text that
  /// is not snapped to pixels, is drawn unhinted like any other.
  pub fn load_hinted_font(&mut self, data: Vec<u8>) {
    self.shared.glyph_engine.lock().load_hinted_font(data);
  }

  pub fn load_font_file(&mut self, path: &Path) {
    self.shared.glyph_engine.lock().load_font_file(path);
  }

  pub fn load_fonts_dir(&mut self, path: &Path) {
    self.shared.glyph_engine.lock().load_fonts_dir(path);
  }

  pub fn register_font(&mut self, name: &str, family: &str) {
    self.shared.glyph_engine.lock().register_font(name, family);
  }

  /// Names the font a generic family draws with, in place of the one chosen
  /// from the system's fonts when the app started. [`GenericFamily::SansSerif`]
  /// is also the font of text with no family.
  ///
  /// A browser has no system fonts a page can read: an app that runs in one
  /// loads its fonts ([`Self::load_font`]) and names them here, the
  /// sans-serif one at least. On a desktop it replaces the system's choice
  /// with a font the app ships.
  ///
  /// [`GenericFamily::SansSerif`]: crate::layout::text_style::GenericFamily::SansSerif
  pub fn set_generic_family(&mut self, generic: crate::layout::text_style::GenericFamily, family: &str) {
    self.shared.glyph_engine.lock().set_generic_family(generic, family);
  }

  /// Installs multiple font faces and aliases with one text-cache invalidation.
  /// Existing atlas entries remain valid because fontdb assigns stable IDs to
  /// loaded faces, so only newly requested glyphs need to be rasterized.
  pub fn install_fonts<I, A, N, F>(&mut self, fonts: I, aliases: A)
  where
    I: IntoIterator<Item = Vec<u8>>,
    A: IntoIterator<Item = (N, F)>,
    N: AsRef<str>,
    F: AsRef<str>,
  {
    self.shared.glyph_engine.lock().install_fonts(fonts, aliases);
  }

  /// Prepares the fonts `styles` are drawn with, so that the first frame
  /// that draws them does not have to.
  ///
  /// The first text of each family, weight and slant makes the text engine
  /// score every font face on the system for it: 4 to 11 ms a combination on
  /// a Windows desktop, on the frame that first shows it. This does it now
  /// instead. Sizes, colours and spacing do not matter: a regular and a bold
  /// style of a family cover every size of them.
  ///
  /// It blocks for that long, so call it where that costs nothing: `App` is
  /// `Send`, and a clone of it on a thread of its own can warm the fonts
  /// while the window and the renderer are created. Call it after the fonts
  /// are loaded: loading a font forgets what was prepared.
  ///
  /// The theme's bold text (`Theme::bold_text`) draws every text two weights
  /// heavier, and the fonts are prepared at those weights when the theme has
  /// it. A shell that reads the setting from the system does so when its
  /// window starts: fonts prepared before that are the lighter ones.
  ///
  /// ```no_run
  /// # use lurq::{app::App, layout::text_style::{FontWeight, TextStyle}};
  /// let app = App::new();
  /// let warming = app.clone();
  /// std::thread::spawn(move || {
  ///   let bold = TextStyle {
  ///     weight: FontWeight::Bold,
  ///     ..TextStyle::default()
  ///   };
  ///   warming.warm_text_styles(&[TextStyle::default(), bold]);
  /// });
  /// ```
  pub fn warm_text_styles<'a>(&self, styles: impl IntoIterator<Item = &'a crate::layout::text_style::TextStyle>) {
    let bold_text = self.shared.theme.bold_text();
    for style in styles {
      // A lock per style: a frame that wants the engine gets in between.
      let mut engine = self.shared.glyph_engine.lock();
      // As a pass sets it, so the pass finds what is prepared here.
      engine.set_bold_text(bold_text);
      engine.warm_style(style);
    }
  }

  /// The app's loader: what reads the sources of its pictures, and of anything else the app loads through it.
  /// It holds the resource root and what the network may do.
  ///
  /// ```
  /// use std::time::Duration;
  ///
  /// let app = lurq::app::App::new();
  /// app.loader().root("assets").timeout(Duration::from_secs(10));
  /// ```
  pub fn loader(&self) -> &crate::resources::Loader {
    &self.shared.loader
  }

  /// Turns on the cache on the disk, in `folder`: what the app loads from an address (a sound, a picture) is
  /// kept there as a file, is there for the next run of the app, and is read from the folder and not from the
  /// network when it is asked for again. It is said where the app is made:
  ///
  /// ```no_run
  /// let app = lurq::app::App::new().enable_media_disk_cache("cache");
  /// // 512 MiB are kept unless told; over that, what was used longest ago is deleted.
  /// app.loader().media_disk_cache().budget(2 * 1024 * 1024 * 1024);
  /// ```
  ///
  /// Not called, nothing is written to the disk, and a sound loaded from an address is kept in memory for the
  /// run of the app (`media::sound_cache()`). The folder is the app's to choose, and lurq picks none: a cache
  /// folder of the app's own, which the system may empty. A browser keeps what it loads itself, and nothing is
  /// written there. The same as `app.loader().enable_media_disk_cache(folder)`; see
  /// [`MediaDiskCache`](crate::resources::MediaDiskCache).
  pub fn enable_media_disk_cache(self, folder: impl Into<PathBuf>) -> Self {
    self.shared.loader.enable_media_disk_cache(folder);
    // What the folder keeps of a sound is answered for by the sounds' cache.
    #[cfg(feature = "media")]
    crate::media::sound_cache().follow(self.shared.loader.media_disk_cache());
    self
  }

  /// The folder the app's resources are under: a path given as a resource is a path in it. The same as
  /// `app.loader().root(root)`.
  pub fn set_resource_root(&mut self, root: PathBuf) {
    self.shared.loader.root(root.clone());
    #[cfg(feature = "http")]
    self.shared.resource_loader.lock().set_root(root);
    #[cfg(not(feature = "http"))]
    let _ = root;
  }

  #[cfg(feature = "persistent_storage")]
  /// Returns a shared handle to the currently selected storage backend.
  pub fn persistent_storage(&self) -> crate::persistent_storage::PersistentStorage {
    self.shared.persistent_storage.lock().clone()
  }

  #[cfg(feature = "persistent_storage")]
  pub fn set_persistent_storage_path(
    &mut self,
    path: impl Into<PathBuf>,
  ) -> Result<(), crate::persistent_storage::PersistentStorageError> {
    *self.shared.persistent_storage.lock() = crate::persistent_storage::PersistentStorage::open(path.into())?;
    Ok(())
  }

  #[cfg(feature = "persistent_storage")]
  pub fn persistent_value<T: crate::persistent_storage::PersistentValue>(&self, key: &str) -> Option<T> {
    self.persistent_storage().value(key)
  }

  #[cfg(feature = "persistent_storage")]
  pub fn set_persistent_value<T: crate::persistent_storage::IntoPersistentValue>(
    &self,
    key: &str,
    value: T,
  ) -> Result<(), crate::persistent_storage::PersistentStorageError> {
    self.persistent_storage().set_value(key, value)
  }

  #[cfg(feature = "persistent_storage")]
  pub fn read_bulk<I, K>(
    &self,
    keys: I,
  ) -> Result<crate::persistent_storage::PersistentReadBatch, crate::persistent_storage::PersistentStorageError>
  where
    I: IntoIterator<Item = K>,
    K: AsRef<str>,
  {
    self.persistent_storage().read_bulk(keys)
  }

  #[cfg(feature = "persistent_storage")]
  pub fn read_bulk_values<T, I, K>(
    &self,
    keys: I,
  ) -> Result<Vec<Option<T>>, crate::persistent_storage::PersistentStorageError>
  where
    T: crate::persistent_storage::PersistentValue,
    I: IntoIterator<Item = K>,
    K: AsRef<str>,
  {
    self.persistent_storage().read_bulk_values(keys)
  }

  #[cfg(feature = "persistent_storage")]
  pub fn write_bulk<I, E>(&self, entries: I) -> Result<(), crate::persistent_storage::PersistentStorageError>
  where
    I: IntoIterator<Item = E>,
    E: crate::persistent_storage::IntoPersistentWrite,
  {
    self.persistent_storage().write_bulk(entries)
  }
}

#[cfg(test)]
mod tests {
  use super::App;
  use crate::layout::text_style::{FontWeight, TextStyle};

  fn bold() -> TextStyle {
    TextStyle {
      weight: FontWeight::Bold,
      ..TextStyle::default()
    }
  }

  #[test]
  fn fonts_warmed_on_another_thread_measure_as_they_would() {
    let untouched = App::new();
    let warmed = App::new();
    let warming = warmed.clone();
    std::thread::spawn(move || warming.warm_text_styles(&[TextStyle::default(), bold()]))
      .join()
      .expect("the warming thread");

    for style in [TextStyle::default(), bold()] {
      for text in ["Ag", "Voice and text for the team", "wrapped\nlines"] {
        let expected = untouched
          .shared
          .glyph_engine
          .lock()
          .measure_text(text, &style, f32::MAX);
        let measured = warmed.shared.glyph_engine.lock().measure_text(text, &style, f32::MAX);
        assert_eq!(
          (measured.width, measured.height),
          (expected.width, expected.height),
          "{text:?}"
        );
      }
    }
  }

  #[test]
  fn fonts_are_warmed_at_the_weights_bold_text_draws() {
    let app = App::new();
    app.shared.theme.set_bold_text(true);
    app.warm_text_styles(&[TextStyle::default()]);

    let mut engine = app.shared.glyph_engine.lock();
    // What a pass does first. The engine has the setting already, so nothing
    // prepared is dropped, and the sample measured to warm the font is found.
    engine.set_bold_text(true);
    engine.measure_text("Ag", &TextStyle::default(), f32::MAX);
    assert_eq!((engine.measure_hits, engine.measure_misses), (1, 1));
  }

  #[test]
  fn warming_no_styles_does_nothing() {
    let app = App::new();
    app.warm_text_styles(&[]);
    let engine = app.shared.glyph_engine.lock();
    assert_eq!((engine.measure_hits, engine.measure_misses), (0, 0));
  }

  #[cfg(feature = "tokio")]
  #[test]
  fn the_runtime_the_app_is_given_is_its_loaders_too_until_it_is_taken_away() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
      .worker_threads(1)
      .build()
      .expect("a runtime");
    assert!(!App::new().loader().reads_on_a_runtime());
    let mut app = App::new().with_tokio_handle(runtime.handle().clone());
    assert!(app.loader().reads_on_a_runtime());
    app.clear_tokio_handle();
    assert!(!app.loader().reads_on_a_runtime());
    app.set_tokio_handle(runtime.handle().clone());
    assert!(app.loader().reads_on_a_runtime());
  }

  #[cfg(not(target_arch = "wasm32"))]
  #[test]
  fn the_cache_on_the_disk_is_off_until_the_app_gives_its_folder() {
    let folder = std::env::temp_dir().join(format!("lurq-app-cache-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    let app = App::new();
    assert!(!app.loader().media_disk_cache().is_on());
    assert!(!folder.exists(), "lurq picks no folder, and makes none");
    let app = app.enable_media_disk_cache(&folder);
    let kept = app.loader().media_disk_cache();
    assert!(kept.is_on() && kept.is_empty());
    assert_eq!(kept.folder().as_deref(), Some(folder.as_path()));
    assert!(folder.is_dir(), "the folder is made");
    let _ = std::fs::remove_dir_all(&folder);
  }
}
