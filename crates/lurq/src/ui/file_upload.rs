//! A file upload: what files to send are given by, and the list of what was given, each with how far it went.

use std::sync::Arc;

use super::{
  Button, ButtonVariant, DropZone, Progress, ProgressSize,
  drop_zone::{Hosted, Refocus, RejectedFile, Takes, TakesFiles, mount_files},
  icons, names,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::{fading, glyph, pressable},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    ctx::Ctx,
    events::{FileDragEvent, FileDropEvent},
    theme::{OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::{Column, Row},
  core::Signal,
  files::{DroppedFile, FileKind, format_size},
  layout::{Alignment, text_style::FontWeight},
  node::{CursorIcon, Element, border::Border as Edge, dimension::Dimension, node_kind::TextOverflow},
};

/// The parts of a file upload a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FileUploadPart {
  /// The upload: what files are given by, and the list under it.
  Root,
  /// The button that opens the system's dialog: "Choose files".
  Trigger,
  /// The list of the files given.
  List,
  /// One file of the list.
  Item,
  /// The icon of a file's kind.
  Icon,
  /// The column of a file's line, its progress and its error.
  Text,
  /// A file's name.
  Label,
  /// What is said after the name: its size, or how far it is sent.
  Meta,
  /// How far a file being sent is.
  Progress,
  /// What went wrong with a file that failed.
  Error,
  /// The mark of a file that was sent.
  Status,
  /// What takes a file out of the list.
  Remove,
  /// What sends a failed file again.
  Retry,
}

names!(FileUploadPart {
  Root => "root",
  Trigger => "trigger",
  List => "list",
  Item => "item",
  Icon => "icon",
  Text => "text",
  Label => "label",
  Meta => "meta",
  Progress => "progress",
  Error => "error",
  Status => "status",
  Remove => "remove",
  Retry => "retry",
});

/// The states of a file upload a recipe styles, besides what the pointer and the keyboard do to it. The states of
/// a file are resolved on its item's parts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FileUploadState {
  /// Files are given by a drop zone, in place of the button.
  Zone,
  /// What was given is wrong: a form says so.
  Invalid,
  /// It takes no files, and none is removed.
  Disabled,
  /// A file that waits to be sent.
  Waiting,
  /// A file being sent.
  Uploading,
  /// A file that was sent.
  Done,
  /// A file that failed.
  Failed,
  /// A file that is not removed nor sent again.
  Unavailable,
}

names!(FileUploadState {
  Zone => "zone",
  Invalid => "invalid",
  Disabled => "disabled",
  Waiting => "waiting",
  Uploading => "uploading",
  Done => "done",
  Failed => "failed",
  Unavailable => "unavailable",
});

/// Where a file of a file upload is: what the app says of it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum UploadState {
  /// Given, not sent yet.
  #[default]
  Waiting,
  /// Being sent: its progress says how far.
  Uploading,
  /// Sent.
  Done,
  /// Not sent: its error says why.
  Failed,
}

/// A file of a file upload: the file, where it is, how far it is sent, and what went wrong.
#[derive(Clone, Debug, PartialEq)]
pub struct UploadFile {
  pub file: DroppedFile,
  pub state: UploadState,
  /// How far it is sent, from 0 to 1, while it is uploading.
  pub progress: f32,
  /// What went wrong, while it failed.
  pub error: Option<Arc<str>>,
  /// What the app finds it by: its path, else its name, unless the app says.
  pub id: Arc<str>,
  pub key: Option<Arc<str>>,
  /// It is not removed nor sent again by a person.
  pub disabled: bool,
}

impl UploadFile {
  /// `file`, waiting to be sent.
  pub fn new(file: DroppedFile) -> Self {
    let id = match file.path() {
      Some(path) => Arc::from(path.to_string_lossy().as_ref()),
      None => Arc::from(file.name()),
    };
    Self {
      file,
      state: UploadState::Waiting,
      progress: 0.0,
      error: None,
      id,
      key: None,
      disabled: false,
    }
  }

  /// Being sent, `progress` of it (from 0 to 1).
  pub fn uploading(mut self, progress: f32) -> Self {
    self.state = UploadState::Uploading;
    self.progress = progress.clamp(0.0, 1.0);
    self.error = None;
    self
  }

  /// Sent.
  pub fn done(mut self) -> Self {
    self.state = UploadState::Done;
    self.progress = 1.0;
    self.error = None;
    self
  }

  /// Not sent, because of `error`.
  pub fn failed(mut self, error: &str) -> Self {
    self.state = UploadState::Failed;
    self.error = Some(Arc::from(error));
    self
  }

  /// Waiting to be sent again.
  pub fn waiting(mut self) -> Self {
    self.state = UploadState::Waiting;
    self.progress = 0.0;
    self.error = None;
    self
  }

  /// What the app finds it by, in place of its path or name.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = id.into();
    self
  }

  /// The key of its element.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Whether a person may not remove it nor send it again.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }
}

impl From<DroppedFile> for UploadFile {
  fn from(file: DroppedFile) -> Self {
    Self::new(file)
  }
}

impl crate::app::component::DevtoolsInspectable for UploadFile {
  fn inspect(&self, formatter: &mut crate::app::component::DevtoolsFormatter<'_>) {
    formatter.value_display(&format!("{} ({:?})", self.file.name(), self.state));
  }
}

/// What builds a file upload's nodes: a theme's in place of the default one.
pub type FileUploadRenderer = Arc<dyn Fn(FileUploadView) -> Element + Send + Sync>;

type ListHandler = Arc<dyn Fn(&[UploadFile]) + Send + Sync>;
type FileHandler = Arc<dyn Fn(&UploadFile) + Send + Sync>;
type FilesHandler = Arc<dyn Fn(&[DroppedFile]) + Send + Sync>;
type RejectHandler = Arc<dyn Fn(&[RejectedFile]) + Send + Sync>;
type SizeText = Arc<dyn Fn(u64) -> String + Send + Sync>;

/// Where the list is: the app's signal, a list the app holds and is told of, or a list shown.
#[derive(Clone)]
enum Files {
  Signal(Signal<Vec<UploadFile>>),
  Held(Vec<UploadFile>),
  Shown(Vec<UploadFile>),
}

impl Files {
  fn list(&self) -> Vec<UploadFile> {
    match self {
      Self::Signal(signal) => signal.get(),
      Self::Held(files) | Self::Shown(files) => files.clone(),
    }
  }

  fn count(&self) -> usize {
    match self {
      Self::Signal(signal) => signal.with_untracked(Vec::len),
      Self::Held(files) | Self::Shown(files) => files.len(),
    }
  }
}

/// A file upload: a button that opens the system's dialog, or a drop zone,
/// and the list of the files given, each with its size, how far it is sent,
/// whether it was or what went wrong. The app sends them: lurq sends nothing.
///
/// ```
/// use lurq::{core::Signal, files::DroppedFile, ui::{FileUpload, UploadFile}};
///
/// let files = Signal::new(vec![
///   UploadFile::new(DroppedFile::new("theme.json", Some(4096))).done(),
///   UploadFile::new(DroppedFile::new("screenshot.png", Some(812_000))).uploading(0.62),
/// ]);
/// let upload = FileUpload::new(files.clone())
///   .accept(["image/*", "json"])
///   .on_files(|given| println!("send {} files", given.len()))
///   .on_retry(|file| println!("send {} again", file.file.name()));
/// ```
///
/// Bound to a `Signal<Vec<UploadFile>>` ([`FileUpload::new`]), what is given
/// is added to the list as waiting and a removed file is taken out of it, and
/// then the handlers are told; held by the app
/// ([`FileUpload::controlled`]), it is only told. The app moves each file on
/// with [`UploadFile::uploading`], [`UploadFile::done`] and
/// [`UploadFile::failed`]. Files dropped anywhere on it are given as a drop
/// zone's are. Opening the dialog, and the zone showing a drag, are what the
/// upload mounted with [`FileUpload::mount`] does: converted into an element
/// on its own, its button opens nothing.
#[derive(Clone)]
pub struct FileUpload {
  files: Files,
  zone: DropZone,
  zone_shown: bool,
  choose_label: Arc<str>,
  remove_label: Arc<str>,
  retry_label: Arc<str>,
  size_text: Option<SizeText>,
  label: Option<Arc<str>>,
  invalid: bool,
  disabled: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  on_change: Vec<ListHandler>,
  on_files: Vec<FilesHandler>,
  on_reject: Vec<RejectHandler>,
  on_remove: Vec<FileHandler>,
  on_retry: Vec<FileHandler>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
  host: Option<Hosted>,
}

impl FileUpload {
  /// The upload bound to the app's list: what is given is added to it, and a removed file is taken out.
  pub fn new(files: Signal<Vec<UploadFile>>) -> Self {
    Self::with(Files::Signal(files))
  }

  /// The upload of a list the app holds: what is given is told to `on_change` as the whole new list, and the
  /// upload shows what the app answers with.
  pub fn controlled(files: Vec<UploadFile>, on_change: impl Fn(&[UploadFile]) + Send + Sync + 'static) -> Self {
    Self::with(Files::Held(files)).on_change(on_change)
  }

  /// The list shown and not operated: nothing is given, removed or sent again.
  pub fn at(files: Vec<UploadFile>) -> Self {
    Self::with(Files::Shown(files))
  }

  fn with(files: Files) -> Self {
    Self {
      files,
      zone: DropZone::new(),
      zone_shown: false,
      choose_label: Arc::from("Choose files"),
      remove_label: Arc::from("Remove"),
      retry_label: Arc::from("Retry"),
      size_text: None,
      label: None,
      invalid: false,
      disabled: false,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      on_change: Vec::new(),
      on_files: Vec::new(),
      on_reject: Vec::new(),
      on_remove: Vec::new(),
      on_retry: Vec::new(),
      id: None,
      key: None,
      host: None,
    }
  }

  /// The files it takes: as a drop zone's [`accept`](DropZone::accept).
  pub fn accept<S: AsRef<str>>(mut self, accepted: impl IntoIterator<Item = S>) -> Self {
    self.zone = self.zone.accept(accepted);
    self
  }

  /// Whether it takes more than one file: it does unless told.
  pub fn multiple(mut self, multiple: bool) -> Self {
    self.zone = self.zone.multiple(multiple);
    self
  }

  /// The greatest size of a file it takes, in bytes.
  pub fn max_size(mut self, bytes: u64) -> Self {
    self.zone = self.zone.max_size(bytes);
    self
  }

  /// The most files its list holds: what would be past it is refused as too many.
  pub fn max_files(mut self, max: usize) -> Self {
    self.zone = self.zone.max_files(max);
    self
  }

  /// Whether files are given by a drop zone over the list, in place of the button.
  pub fn zone(mut self, zone: bool) -> Self {
    self.zone_shown = zone;
    self
  }

  /// The zone's title, where it has a zone, in place of "Drop files here".
  pub fn title(mut self, title: &str) -> Self {
    self.zone = self.zone.title(title);
    self
  }

  /// The zone's description, where it has a zone, in place of "or click to browse".
  pub fn description(mut self, description: &str) -> Self {
    self.zone = self.zone.description(description);
    self
  }

  /// What the button says, in place of "Choose files".
  pub fn choose_label(mut self, label: &str) -> Self {
    self.choose_label = Arc::from(label);
    self
  }

  /// What a file's remove control is called before its name, in place of "Remove".
  pub fn remove_label(mut self, label: &str) -> Self {
    self.remove_label = Arc::from(label);
    self
  }

  /// What a failed file's retry control is called before its name, in place of "Retry".
  pub fn retry_label(mut self, label: &str) -> Self {
    self.retry_label = Arc::from(label);
    self
  }

  /// How a file's size is said, in place of "4 KB", "1.2 MB".
  pub fn size_text(mut self, text: impl Fn(u64) -> String + Send + Sync + 'static) -> Self {
    self.size_text = Some(Arc::new(text));
    self
  }

  /// What it is called to what reads the tree, and the title of its dialog.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self.zone = self.zone.label(label);
    self
  }

  /// Whether what was given is wrong: the button's or the zone's edge in the destructive color.
  pub fn invalid(mut self, invalid: bool) -> Self {
    self.invalid = invalid;
    self
  }

  /// Whether it takes no files, and no file is removed or sent again.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Called with the whole new list when a person gives or removes files; bound, after the signal is set. A second
  /// handler is called after the first.
  pub fn on_change(mut self, handler: impl Fn(&[UploadFile]) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// Called with the files given, after the list: those to send. A second handler is called after the first.
  pub fn on_files(mut self, handler: impl Fn(&[DroppedFile]) + Send + Sync + 'static) -> Self {
    self.on_files.push(Arc::new(handler));
    self
  }

  /// Called with the files it refused, and why. A second handler is called after the first.
  pub fn on_reject(mut self, handler: impl Fn(&[RejectedFile]) + Send + Sync + 'static) -> Self {
    self.on_reject.push(Arc::new(handler));
    self
  }

  /// Called with a file a person removed from the list, after the list: one being sent is to be stopped. A
  /// second handler is called after the first.
  pub fn on_remove(mut self, handler: impl Fn(&UploadFile) + Send + Sync + 'static) -> Self {
    self.on_remove.push(Arc::new(handler));
    self
  }

  /// Called with a failed file a person asked to send again; the app sets it uploading. A second handler is called
  /// after the first.
  pub fn on_retry(mut self, handler: impl Fn(&UploadFile) + Send + Sync + 'static) -> Self {
    self.on_retry.push(Arc::new(handler));
    self
  }

  /// Puts the upload in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this upload over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this upload.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the button or the zone as it is under the pointer, held down or focused from the keyboard, with nothing
  /// pointing at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the upload's element. Its button or zone has the id `<id>-trigger`, each file `<id>-<file's id>`.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// The upload mounted where it is declared: its button or its zone opens the system's dialog, and the zone shows
  /// a drag over it.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    mount_files(self, ctx)
  }

  fn operated(&self) -> bool {
    !matches!(self.files, Files::Shown(_)) && !self.disabled
  }

  /// What a person's change of the list does: the signal is set, then the handlers are told.
  fn change(&self) -> Change {
    Change {
      files: self.files.clone(),
      on_change: self.on_change.clone(),
      on_files: self.on_files.clone(),
      on_reject: self.on_reject.clone(),
      on_remove: self.on_remove.clone(),
    }
  }

  /// Builds the upload in `theme`. Converting an upload into an element does this in the theme of what is being
  /// rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(FileUploadState::Zone, self.zone_shown)
    .state(FileUploadState::Invalid, self.invalid)
    .state(FileUploadState::Disabled, self.disabled);
    let item_recipe = |file: &UploadFile| {
      let state = match file.state {
        UploadState::Waiting => FileUploadState::Waiting,
        UploadState::Uploading => FileUploadState::Uploading,
        UploadState::Done => FileUploadState::Done,
        UploadState::Failed => FileUploadState::Failed,
      };
      let case = case
        .clone()
        .state(state, true)
        .state(FileUploadState::Unavailable, file.disabled);
      theme.resolve::<FileUpload>(&case, Some(&self.rules))
    };
    let files = self.files.list();
    let items = files.iter().map(item_recipe).collect();
    let operated = self.operated();
    let view = FileUploadView {
      label: self.label.clone().unwrap_or_else(|| self.choose_label.clone()),
      choose_label: self.choose_label.clone(),
      remove_label: self.remove_label.clone(),
      retry_label: self.retry_label.clone(),
      size_text: self.size_text.clone(),
      zone_shown: self.zone_shown,
      invalid: self.invalid,
      disabled: self.disabled,
      operated,
      recipe: theme.resolve::<FileUpload>(&case, Some(&self.rules)),
      items,
      change: self.change(),
      takes: self.takes_now(),
      zone: self.zone.clone(),
      on_retry: self.on_retry.clone(),
      host: self.host.clone(),
      forced: self.forced,
      id: self.id.clone(),
      key: self.key.clone(),
      files,
    };
    match theme.renderer::<FileUpload>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<FileUpload> for Element {
  fn from(upload: FileUpload) -> Self {
    upload.build(&current_theme())
  }
}

impl TakesFiles for FileUpload {
  /// What it takes now: the zone's, with the room the list has left.
  fn takes_now(&self) -> Takes {
    let mut takes = self.zone.takes().clone();
    let held = self.files.count();
    takes.max_files = match (takes.multiple, takes.max_files) {
      (false, _) => Some(1usize.saturating_sub(held)),
      (true, Some(max)) => Some(max.saturating_sub(held)),
      (true, None) => None,
    };
    takes.multiple = true;
    takes
  }

  fn deliver(&self, taken: &[DroppedFile], rejected: &[RejectedFile]) {
    if self.operated() {
      self.change().give(taken, rejected);
    }
  }

  fn hosted(mut self, host: Hosted) -> Element {
    self.host = Some(host);
    self.into()
  }

  fn host_key(&self) -> Option<Arc<str>> {
    self.key.clone()
  }
}

/// What a person's change of the list does.
#[derive(Clone)]
struct Change {
  files: Files,
  on_change: Vec<ListHandler>,
  on_files: Vec<FilesHandler>,
  on_reject: Vec<RejectHandler>,
  on_remove: Vec<FileHandler>,
}

impl Change {
  /// Files given: added to the list as waiting, and told.
  fn give(&self, taken: &[DroppedFile], rejected: &[RejectedFile]) {
    if !taken.is_empty() {
      let mut list = self.current();
      list.extend(taken.iter().cloned().map(UploadFile::new));
      self.set(&list);
      for handler in &self.on_files {
        handler(taken);
      }
    }
    if !rejected.is_empty() {
      for handler in &self.on_reject {
        handler(rejected);
      }
    }
  }

  /// A file removed: taken out of the list, and told.
  fn remove(&self, id: &str) {
    let mut list = self.current();
    let Some(index) = list.iter().position(|file| &*file.id == id) else {
      return;
    };
    let removed = list.remove(index);
    self.set(&list);
    for handler in &self.on_remove {
      handler(&removed);
    }
  }

  fn current(&self) -> Vec<UploadFile> {
    match &self.files {
      Files::Signal(signal) => signal.get_untracked(),
      Files::Held(files) | Files::Shown(files) => files.clone(),
    }
  }

  /// The new list: into the signal first, then to every handler.
  fn set(&self, list: &[UploadFile]) {
    if let Files::Signal(signal) = &self.files {
      signal.set(list.to_vec());
    }
    for handler in &self.on_change {
      handler(list);
    }
  }
}

/// A file upload as a renderer sees it: what it was given, the style of its parts, and its behavior ready to be
/// dressed.
pub struct FileUploadView {
  /// What it is called to what reads the tree.
  pub label: Arc<str>,
  pub choose_label: Arc<str>,
  pub remove_label: Arc<str>,
  pub retry_label: Arc<str>,
  /// The files of the list, in their order.
  pub files: Vec<UploadFile>,
  /// Whether files are given by a drop zone in place of the button.
  pub zone_shown: bool,
  pub invalid: bool,
  pub disabled: bool,
  /// Whether a person gives, removes and retries files: not where the list is only shown, or it is disabled.
  pub operated: bool,
  /// The style of each part of the upload, as a whole.
  pub recipe: ResolvedRecipe,
  /// The style of each part for each file, in the file's state.
  items: Vec<ResolvedRecipe>,
  size_text: Option<SizeText>,
  change: Change,
  takes: Takes,
  zone: DropZone,
  on_retry: Vec<FileHandler>,
  host: Option<Hosted>,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl FileUploadView {
  /// The style of a part of the upload as a whole.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  /// The style of a part of the file at `index`, in its state.
  pub fn item_part(&self, index: usize, part: impl Into<Name>) -> ResolvedPart {
    self.items.get(index).unwrap_or(&self.recipe).part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  fn child_id(&self, suffix: &str) -> Option<Arc<str>> {
    self.id.as_ref().map(|id| Arc::from(format!("{id}-{suffix}")))
  }

  /// What files are given by: the zone, or the button that opens the dialog, in the style of the trigger.
  pub fn trigger(&self) -> Element {
    if self.zone_shown {
      let change = self.change.clone();
      let operated = self.operated;
      let mut zone = self
        .zone
        .clone()
        .invalid(self.invalid)
        // A list only shown draws its zone at rest: it takes nothing all the same.
        .disabled(self.disabled)
        .force(self.forced)
        .with_host(self.host.clone());
      // The zone's own count of files does not know the list: it takes what the upload takes.
      zone = zone
        .max_files(self.takes.max_files.unwrap_or(usize::MAX))
        .multiple(true);
      zone = zone
        .on_files({
          let change = change.clone();
          move |taken| {
            if operated {
              change.give(taken, &[]);
            }
          }
        })
        .on_reject(move |rejected| {
          if operated {
            change.give(&[], rejected);
          }
        });
      if let Some(id) = self.child_id("trigger") {
        zone = zone.id(id);
      }
      return zone.into();
    }
    let mut button = Button::new(&self.choose_label)
      .variant(ButtonVariant::Outline)
      .icon(icons::UPLOAD)
      .disabled(self.disabled)
      .rules(&self.recipe.rules_for(FileUploadPart::Trigger, super::ButtonPart::Root))
      .force(self.forced);
    if let (true, Some(host)) = (self.operated, &self.host) {
      let (choose, dialog) = (host.choose.clone(), self.takes.dialog(&self.label));
      button = button.on_click(move |_| choose(dialog.clone()));
    }
    if let Some(id) = self.child_id("trigger") {
      button = button.id(id);
    }
    if let Some(element) = self.focus_ref(&Refocus::Trigger) {
      button = button.ref_element(element);
    }
    button.into()
  }

  /// The line of the file at `index`: its kind's icon, its name, its size or how far it is sent, its progress, its
  /// error, and its mark or its controls.
  pub fn item(&self, index: usize) -> Option<Element> {
    let file = self.files.get(index)?;
    let part = |name: FileUploadPart| self.item_part(index, name);
    let icon = match file.file.kind() {
      FileKind::Image => icons::IMAGE,
      FileKind::Text => icons::FILE_TEXT,
      FileKind::Archive => icons::ARCHIVE,
      _ => icons::FILE,
    };
    let meta = match file.state {
      UploadState::Uploading => format!("{}%", (file.progress * 100.0).round() as u32),
      _ => file
        .file
        .size()
        .map(|size| match &self.size_text {
          Some(text) => text(size),
          None => format_size(size),
        })
        .unwrap_or_default(),
    };
    let fill = Dimension::Pct(100.0);
    let name = part(FileUploadPart::Label);
    let mut line = Row::new()
      .width(fill)
      .align_items(Alignment::Center)
      .spacing(8.0)
      .child(
        Row::new()
          .flex(1.0)
          .min_width(crate::node::dimension::Dimension::Px(0.0))
          .part(&name)
          .child(
            name
              .text(file.file.name())
              .nowrap()
              .text_overflow(TextOverflow::Elipsis)
              .width(fill),
          ),
      );
    if !meta.is_empty() {
      let meta_part = part(FileUploadPart::Meta);
      line = line.child(Row::new().part(&meta_part).child(meta_part.text(&meta).nowrap()));
    }
    let mut text = Column::new()
      .flex(1.0)
      .width(fill)
      .part(&part(FileUploadPart::Text))
      .child(line);
    if file.state == UploadState::Uploading {
      let progress = Progress::new(file.progress)
        .size(ProgressSize::Sm)
        .label(file.file.name())
        .label_hidden(true);
      text = text.child(
        Row::new()
          .width(fill)
          .part(&part(FileUploadPart::Progress))
          .child(progress),
      );
    }
    if let (UploadState::Failed, Some(error)) = (file.state, &file.error) {
      let error_part = part(FileUploadPart::Error);
      text = text.child(
        Column::new()
          .width(fill)
          .part(&error_part)
          .child(error_part.text(error).width(fill)),
      );
    }
    let mut item = Row::new()
      .width(fill)
      .align_items(Alignment::Center)
      .part(&part(FileUploadPart::Item))
      .child(glyph(&part(FileUploadPart::Icon), Name::from(icon), false))
      .child(text);
    let able = self.operated && !file.disabled;
    match file.state {
      UploadState::Done => {
        item = item.child(glyph(
          &part(FileUploadPart::Status),
          Name::from(icons::CHECK_CIRCLE),
          false,
        ));
      }
      UploadState::Failed => {
        let retry = {
          let (handlers, file) = (self.on_retry.clone(), file.clone());
          move || {
            for handler in &handlers {
              handler(&file);
            }
          }
        };
        item = item
          .child(self.control(
            index,
            FileUploadPart::Retry,
            icons::REFRESH,
            &self.retry_label,
            able,
            retry,
          ))
          .child(self.remove(index, able));
      }
      UploadState::Waiting | UploadState::Uploading => item = item.child(self.remove(index, able)),
    }
    if let Some(id) = self.child_id(&file.id) {
      item = item.id(id);
    }
    if let Some(key) = &file.key {
      item = item.key(key.clone());
    }
    Some(item.into())
  }

  fn remove(&self, index: usize, able: bool) -> Element {
    let (change, id) = (self.change.clone(), self.files[index].id.clone());
    // The control goes with its file: the focus goes on to the next file's control, else the one before, else
    // to what gives files.
    let next = self
      .has_control(index + 1..self.files.len())
      .or_else(|| self.has_control_back(index));
    let target = next
      .map(|other| Refocus::Control(self.files[other].id.clone()))
      .unwrap_or(Refocus::Trigger);
    let refocus = self.host.as_ref().map(|host| host.refocus.clone());
    self.control(
      index,
      FileUploadPart::Remove,
      icons::CLOSE,
      &self.remove_label,
      able,
      move || {
        if let Some(refocus) = &refocus {
          *refocus.lock().unwrap() = Some(target.clone());
        }
        change.remove(&id)
      },
    )
  }

  /// The first file of `range` with a control a person presses.
  fn has_control(&self, mut range: std::ops::Range<usize>) -> Option<usize> {
    range.find(|&index| self.pressed(index))
  }

  /// The last file before `index` with a control a person presses.
  fn has_control_back(&self, index: usize) -> Option<usize> {
    (0..index).rev().find(|&index| self.pressed(index))
  }

  fn pressed(&self, index: usize) -> bool {
    let file = &self.files[index];
    self.operated && !file.disabled && file.state != UploadState::Done
  }

  /// The ref the element at `target` carries in this render, where the focus is to go there.
  fn focus_ref(&self, target: &Refocus) -> Option<crate::core::ElementRef> {
    let (to, element) = self.host.as_ref()?.focus.as_ref()?;
    (to == target).then(|| element.clone())
  }

  /// A control of a file: an icon pressed, called by `label` and the file's name.
  fn control(
    &self,
    index: usize,
    part: FileUploadPart,
    icon: &'static str,
    label: &str,
    able: bool,
    action: impl Fn() + Send + Sync + 'static,
  ) -> Element {
    let style = self.item_part(index, part);
    let name = format!("{label} {}", self.files[index].file.name());
    if !able {
      return Row::new()
        .part(&style)
        .describe("a11y_name", name)
        .child(glyph(&style, Name::from(icon), false))
        .into();
    }
    let mut control = pressable(&style, false)
      .describe("a11y_role", "button")
      .describe("a11y_name", name)
      .on_click(move |_| action())
      .child(glyph(&style, Name::from(icon), false));
    // A failed file's first control is its retry: the focus that comes to the file comes there.
    let first = part == FileUploadPart::Retry || self.files[index].state != UploadState::Failed;
    if first && let Some(element) = self.focus_ref(&Refocus::Control(self.files[index].id.clone())) {
      control = control.ref_element(element);
    }
    if let Some(id) = self.child_id(&format!("{}-{}", self.files[index].id, part_name(part))) {
      control = control.id(id);
    }
    control.into()
  }

  /// The list of the files, one under another.
  pub fn list(&self) -> Option<Column> {
    if self.files.is_empty() {
      return None;
    }
    let mut list = Column::new()
      .width(Dimension::Pct(100.0))
      .part(&self.part(FileUploadPart::List))
      .describe("a11y_role", "list");
    for index in 0..self.files.len() {
      if let Some(item) = self.item(index) {
        list = list.child(item);
      }
    }
    Some(list)
  }

  /// How the default renderer builds an upload: the button or the zone, and the list under it. Files dropped
  /// anywhere on it are given.
  pub fn render(self) -> Element {
    let mut root = Column::new()
      .align_items(Alignment::Start)
      .part(&self.part(FileUploadPart::Root))
      .describe("a11y_name", self.label.clone())
      .child(self.trigger());
    if let Some(list) = self.list() {
      root = root.child(list);
    }
    if self.operated && !self.zone_shown {
      let (change, takes) = (self.change.clone(), self.takes.clone());
      root = root
        .on_file_drag_enter({
          let takes = takes.clone();
          move |event: FileDragEvent| {
            if takes.sort_dragged(&event.paths).0.is_empty() {
              event.refuse();
            }
          }
        })
        .on_file_drop(move |event: FileDropEvent| {
          let (taken, rejected) = takes.sort_dragged(&event.paths);
          change.give(&taken, &rejected);
        });
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root.into()
  }
}

fn part_name(part: FileUploadPart) -> &'static str {
  match part {
    FileUploadPart::Retry => "retry",
    _ => "remove",
  }
}

impl super::FieldControl for FileUpload {
  /// Called by the field's label unless it has its own, invalid where the field is, and disabled where it is.
  fn in_field(mut self, field: &super::InField) -> Element {
    if self.label.is_none() {
      self = self.label(&field.label);
    }
    self.invalid |= field.judgment == Some(super::Judgment::Invalid);
    self.disabled |= field.disabled;
    self.into()
  }
}

impl super::FieldControl for DropZone {
  /// Called by the field's label unless it has its own, invalid where the field is, and disabled where it is.
  fn in_field(self, field: &super::InField) -> Element {
    self
      .label(&field.label)
      .invalid(field.judgment == Some(super::Judgment::Invalid))
      .disabled(field.disabled)
      .into()
  }
}

impl Themed for FileUpload {
  const NAME: &'static str = "file_upload";
  type Renderer = FileUploadRenderer;

  /// The design's file upload (`pencil/lurq-design-system.pen`, File Upload): 300 wide, the outline button "Choose
  /// files" with the upload icon, and 8 under it a list of the files 8 apart. A file is a row with an edge of 1 in
  /// the border's color inside it, the medium radius, padded 12 across and 10 down, its parts 10 apart: the icon of
  /// its kind, 16 in the muted foreground; its name in 13 medium, cut with an ellipsis, and 8 after it its size or
  /// how far it is sent in 11 mono in the muted foreground; while it is sent a bar of 4, 6 under the line; at the
  /// end the check in the success color once it was sent, or the remove control of 16 in the muted foreground. The
  /// board draws no failed file: its edge is destructive, its error in 12 in the destructive color under its line,
  /// with a retry control before the remove control. A drop zone in place of the button is as wide as the list.
  fn recipe() -> Recipe {
    use FileUploadPart::*;
    use PaletteColor::{Border as BorderColor, Destructive, Foreground, MutedForeground, Success};
    let edge = |color: PaletteColor| Edge::inside(1.0, color);
    let control = |s: super::PartStyle| {
      fading(
        s.square(16.0)
          .radius(RadiusSize::Sm)
          .foreground(MutedForeground)
          .cursor(CursorIcon::Pointer),
      )
    };
    Recipe::new()
      .base(Root, |s| s.width(300.0).gap(8.0))
      .base(List, |s| s.gap(8.0))
      .base(Item, |s| {
        s.border(edge(BorderColor))
          .radius(RadiusSize::Md)
          .padding_x(12.0)
          .padding_y(10.0)
          .gap(10.0)
      })
      .base(Icon, |s| s.square(16.0).foreground(MutedForeground))
      .base(Text, |s| s.gap(6.0))
      .base(Label, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(13.0)
          .weight(FontWeight::Medium)
          .foreground(Foreground)
      })
      .base(Meta, |s| {
        s.typography(TypographyStyle::Mono)
          .font_size(11.0)
          .foreground(MutedForeground)
      })
      .base(Error, |s| {
        s.typography(TypographyStyle::Caption).foreground(Destructive)
      })
      .base(Status, |s| s.square(16.0).foreground(Success))
      .base(Remove, control)
      .base(Retry, control)
      .hovered(Remove, |s| s.foreground(Foreground))
      .hovered(Retry, |s| s.foreground(Foreground))
      .focus_visible(Remove, |s| s.focus_ring())
      .focus_visible(Retry, |s| s.focus_ring())
      .state(FileUploadState::Zone, Root, |s| s.width(320.0))
      .state(FileUploadState::Failed, Item, |s| s.border(edge(Destructive)))
      .state(FileUploadState::Failed, Icon, |s| s.foreground(Destructive))
      .state(FileUploadState::Invalid, Trigger, |s| s.border(edge(Destructive)))
      .state(FileUploadState::Unavailable, Remove, |s| {
        s.opacity(OpacityLevel::Disabled).cursor(CursorIcon::Default)
      })
      .state(FileUploadState::Unavailable, Retry, |s| {
        s.opacity(OpacityLevel::Disabled).cursor(CursorIcon::Default)
      })
      .state(FileUploadState::Disabled, List, |s| s.opacity(OpacityLevel::Disabled))
  }
}
