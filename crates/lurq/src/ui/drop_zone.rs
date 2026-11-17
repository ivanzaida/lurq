//! A drop zone: where files are dropped from the system, or chosen in its file dialog.

use std::sync::{Arc, Mutex};

use super::{
  icons, names,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::{IconChoice, fading, glyph, pressable, shown_icon},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    component::Component,
    ctx::{Ctx, FutureAction},
    events::{FileDragEvent, FileDropEvent},
    theme::{OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::Column,
  core::Signal,
  files::{DroppedFile, FileDialog},
  layout::{
    Alignment,
    layout_kind::Justify,
    text_style::{FontWeight, TextAlign},
  },
  node::{CursorIcon, Element, border::Border as Edge, dimension::Dimension},
};

/// The parts of a drop zone a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DropZonePart {
  /// The zone: what files are dropped on, and what is pressed to choose them.
  Root,
  /// The icon over the words.
  Icon,
  /// The column of the title and the description.
  Text,
  /// What it says: "Drop files here".
  Title,
  /// What it says under that: "or click to browse".
  Description,
}

names!(DropZonePart {
  Root => "root",
  Icon => "icon",
  Text => "text",
  Title => "title",
  Description => "description",
});

/// The states of a drop zone a recipe styles, besides what the pointer and the keyboard do to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DropZoneState {
  /// Files it takes are dragged over it.
  DragOver,
  /// Files it takes none of are dragged over it.
  Rejected,
  /// What it was given is wrong: a form says so.
  Invalid,
  /// It takes no files.
  Disabled,
  /// It has an icon.
  Icon,
  /// It has a description.
  Description,
}

names!(DropZoneState {
  DragOver => "drag_over",
  Rejected => "rejected",
  Invalid => "invalid",
  Disabled => "disabled",
  Icon => "icon",
  Description => "description",
});

/// Why a drop zone did not take a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RejectReason {
  /// It is not of the kinds or extensions the zone accepts.
  Kind,
  /// It is larger than the zone's greatest size.
  TooLarge,
  /// The zone had taken as many files as it takes.
  TooMany,
  /// It is a folder.
  Folder,
}

/// A file a drop zone did not take, and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RejectedFile {
  pub file: DroppedFile,
  pub reason: RejectReason,
}

/// What builds a drop zone's nodes: a theme's in place of the default one.
pub type DropZoneRenderer = Arc<dyn Fn(DropZoneView) -> Element + Send + Sync>;

type FilesHandler = Arc<dyn Fn(&[DroppedFile]) + Send + Sync>;
type RejectHandler = Arc<dyn Fn(&[RejectedFile]) + Send + Sync>;

/// What a drop zone takes: the files it accepts, how many, and how large.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Takes {
  pub(crate) accept: Vec<Arc<str>>,
  pub(crate) multiple: bool,
  pub(crate) max_size: Option<u64>,
  pub(crate) max_files: Option<usize>,
}

impl Takes {
  /// The files of `given` it takes, and those it does not with why, in their order.
  pub(crate) fn sort(
    &self,
    given: impl IntoIterator<Item = (DroppedFile, bool)>,
  ) -> (Vec<DroppedFile>, Vec<RejectedFile>) {
    let room = match (self.multiple, self.max_files) {
      (false, _) => 1,
      (true, Some(max)) => max,
      (true, None) => usize::MAX,
    };
    let (mut taken, mut rejected) = (Vec::new(), Vec::new());
    for (file, is_folder) in given {
      let reason = if is_folder {
        Some(RejectReason::Folder)
      } else if !file.is_accepted(&self.accept) {
        Some(RejectReason::Kind)
      } else if self
        .max_size
        .is_some_and(|max| file.size().is_some_and(|size| size > max))
      {
        Some(RejectReason::TooLarge)
      } else if taken.len() >= room {
        Some(RejectReason::TooMany)
      } else {
        None
      };
      match reason {
        Some(reason) => rejected.push(RejectedFile { file, reason }),
        None => taken.push(file),
      }
    }
    (taken, rejected)
  }

  /// The files of a drag as the zone sorts them.
  pub(crate) fn sort_dragged(&self, paths: &crate::app::events::DraggedPaths) -> (Vec<DroppedFile>, Vec<RejectedFile>) {
    self.sort(
      paths
        .items()
        .iter()
        .map(|item| (DroppedFile::from_path(item.path()), item.is_folder())),
    )
  }

  /// The system's dialog for what it takes.
  pub(crate) fn dialog(&self, title: &str) -> FileDialog {
    let mut dialog = FileDialog::new()
      .title(title)
      .multiple(self.multiple && self.max_files != Some(1));
    // A kind is not an extension the dialog can filter by: with a kind among them, it shows every file.
    if !self.accept.is_empty() && self.accept.iter().all(|entry| !entry.ends_with("/*")) {
      let extensions: Vec<&str> = self.accept.iter().map(|entry| entry.trim_start_matches('.')).collect();
      dialog = dialog.filter(&extensions.join(", "), &extensions);
    }
    dialog
  }
}

/// A drop zone: where files are dropped from the system, or chosen in the
/// system's file dialog. Images for a gallery, a document to import.
///
/// ```
/// use lurq::ui::DropZone;
///
/// let zone = DropZone::new()
///   .accept(["image/*", "pdf"])
///   .max_size(5 * 1024 * 1024)
///   .description("PNG, JPG or PDF, up to 5 MB")
///   .on_files(|files| println!("{} files to upload", files.len()))
///   .on_reject(|refused| println!("{} not taken", refused.len()));
/// ```
///
/// It is a stop of Tab; a press of it, or Enter or Space on it, opens the
/// system's dialog for the files it takes. While files are dragged over it,
/// it shows whether it takes them: one it takes none of says so and is
/// refused. What it takes is told to [`DropZone::on_files`], what it does
/// not to [`DropZone::on_reject`]. Showing a drag and opening the dialog are
/// what the zone mounted with [`DropZone::mount`] does: converted into an
/// element on its own it is its still form, which takes a drop and opens no
/// dialog. The dialog is the system's with the feature `file_dialog`; see
/// [`lurq::files`](crate::files) for what each platform gives.
#[derive(Clone)]
pub struct DropZone {
  takes: Takes,
  icon: Option<IconChoice>,
  title: Arc<str>,
  description: Option<Arc<str>>,
  drop_text: Arc<str>,
  reject_text: Arc<str>,
  label: Option<Arc<str>>,
  invalid: bool,
  disabled: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  on_files: Vec<FilesHandler>,
  on_reject: Vec<RejectHandler>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
  /// What the mounted zone keeps: the drag over it, and the dialog it opens.
  host: Option<Hosted>,
}

/// What a mounted taker of files gives its still form: the drag over it, and what opens its dialog.
#[derive(Clone)]
pub(crate) struct Hosted {
  pub(crate) over: Signal<Option<bool>>,
  pub(crate) choose: Arc<dyn Fn(FileDialog) + Send + Sync>,
  /// Where the focus is to go at the next render, once what had it is gone: set by a handler.
  pub(crate) refocus: Arc<Mutex<Option<Refocus>>>,
  /// Where the focus goes in this render, and the ref the element there is to carry.
  pub(crate) focus: Option<(Refocus, crate::core::ElementRef)>,
}

/// Where the focus goes once what had it is gone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Refocus {
  /// The control of the file with this id.
  Control(Arc<str>),
  /// What files are given by: the button or the zone.
  Trigger,
}

impl Default for DropZone {
  fn default() -> Self {
    Self::new()
  }
}

impl DropZone {
  /// A zone that takes any number of files of any kind and size.
  pub fn new() -> Self {
    Self {
      takes: Takes {
        multiple: true,
        ..Takes::default()
      },
      icon: None,
      title: super::current_locale().fields.drop_files.clone(),
      description: Some(super::current_locale().fields.or_browse.clone()),
      drop_text: super::current_locale().fields.drop_to_add.clone(),
      reject_text: super::current_locale().fields.cannot_add.clone(),
      label: None,
      invalid: false,
      disabled: false,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      on_files: Vec::new(),
      on_reject: Vec::new(),
      id: None,
      key: None,
      host: None,
    }
  }

  /// The files it takes: extensions (`"png"` or `".png"`) and kinds (`"image/*"`, `"video/*"`, `"audio/*"`,
  /// `"text/*"`, `"archive/*"`), in any case. A second call adds to them. Nothing accepted takes every file.
  pub fn accept<S: AsRef<str>>(mut self, accepted: impl IntoIterator<Item = S>) -> Self {
    self
      .takes
      .accept
      .extend(accepted.into_iter().map(|entry| Arc::from(entry.as_ref())));
    self
  }

  /// Whether it takes more than one file at once: it does unless told.
  pub fn multiple(mut self, multiple: bool) -> Self {
    self.takes.multiple = multiple;
    self
  }

  /// The greatest size of a file it takes, in bytes. A file whose size is not known is taken.
  pub fn max_size(mut self, bytes: u64) -> Self {
    self.takes.max_size = Some(bytes);
    self
  }

  /// The most files it takes at once: those past it are refused as too many.
  pub fn max_files(mut self, max: usize) -> Self {
    self.takes.max_files = Some(max);
    self
  }

  /// The icon over its words in place of the upload icon, or `None` for none.
  pub fn icon(mut self, icon: impl Into<IconChoice>) -> Self {
    self.icon = Some(icon.into());
    self
  }

  /// What it says, in place of "Drop files here".
  pub fn title(mut self, title: &str) -> Self {
    self.title = Arc::from(title);
    self
  }

  /// What it says under its title, in place of "or click to browse": what it takes, say.
  pub fn description(mut self, description: &str) -> Self {
    self.description = Some(Arc::from(description));
    self
  }

  /// No words under its title.
  pub fn no_description(mut self) -> Self {
    self.description = None;
    self
  }

  /// What it says while files it takes are dragged over it, in place of "Drop to add".
  pub fn drop_text(mut self, text: &str) -> Self {
    self.drop_text = Arc::from(text);
    self
  }

  /// What it says while files it takes none of are dragged over it, in place of "These files can't be added".
  pub fn reject_text(mut self, text: &str) -> Self {
    self.reject_text = Arc::from(text);
    self
  }

  /// What it is called to what reads the tree, and the title of its dialog; its title where it has none.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// Whether what it was given is wrong: its edge in the destructive color.
  pub fn invalid(mut self, invalid: bool) -> Self {
    self.invalid = invalid;
    self
  }

  /// Whether it takes no files: no drop, no press, dimmed.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Called with the files it took from a drop or from its dialog, in their order. A second handler is called
  /// after the first.
  pub fn on_files(mut self, handler: impl Fn(&[DroppedFile]) + Send + Sync + 'static) -> Self {
    self.on_files.push(Arc::new(handler));
    self
  }

  /// Called with the files of a drop or of its dialog it did not take, and why. A second handler is called after
  /// the first.
  pub fn on_reject(mut self, handler: impl Fn(&[RejectedFile]) + Send + Sync + 'static) -> Self {
    self.on_reject.push(Arc::new(handler));
    self
  }

  /// Puts the zone in a state the theme's recipe names, or takes it out: `DragOver` and `Rejected` draw a drag
  /// with none, for a gallery or a test.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this zone over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this zone.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the zone as it is under the pointer, held down or focused from the keyboard, with nothing pointing at
  /// it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the zone's element.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// What tells the zone apart from those beside it.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// What it takes: for a component that holds a zone.
  pub(crate) fn takes(&self) -> &Takes {
    &self.takes
  }

  /// The zone mounted where it is declared: it shows whether files dragged over it are taken, and a press opens
  /// the system's dialog.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    mount_files(self, ctx)
  }

  /// The zone with what a host keeps: for a component that holds a zone and is mounted itself.
  pub(crate) fn with_host(mut self, host: Option<Hosted>) -> Self {
    self.host = host;
    self
  }

  /// Builds the zone in `theme`. Converting a zone into an element does this in the theme of what is being
  /// rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let over = self.host.as_ref().and_then(|host| host.over.get());
    let icon = shown_icon(self.icon, icons::UPLOAD);
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(DropZoneState::DragOver, over == Some(true) && !self.disabled)
    .state(DropZoneState::Rejected, over == Some(false) && !self.disabled)
    .state(DropZoneState::Invalid, self.invalid)
    .state(DropZoneState::Disabled, self.disabled)
    .state(DropZoneState::Icon, icon.is_some())
    .state(DropZoneState::Description, self.description.is_some());
    let shown = |state: DropZoneState| case.states.contains(&Name::from(state));
    let (drag_over, rejected) = (shown(DropZoneState::DragOver), shown(DropZoneState::Rejected));
    let view = DropZoneView {
      title: if drag_over {
        self.drop_text.clone()
      } else if rejected {
        self.reject_text.clone()
      } else {
        self.title.clone()
      },
      // A drag says one thing: what the drop will do.
      description: if drag_over || rejected {
        None
      } else {
        self.description.clone()
      },
      label: self.label.unwrap_or_else(|| self.title.clone()),
      icon,
      drag_over,
      rejected,
      invalid: self.invalid,
      disabled: self.disabled,
      recipe: theme.resolve::<DropZone>(&case, Some(&self.rules)),
      forced: self.forced,
      takes: self.takes,
      on_files: self.on_files,
      on_reject: self.on_reject,
      host: self.host,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<DropZone>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<DropZone> for Element {
  fn from(zone: DropZone) -> Self {
    zone.build(&current_theme())
  }
}

/// A drop zone as a renderer sees it: what it was given, the style of its parts, and its behavior ready to be
/// dressed.
pub struct DropZoneView {
  /// What it says now: its title, or what a drag over it will do.
  pub title: Arc<str>,
  /// What it says under its title, where it says something; none while a drag is over it.
  pub description: Option<Arc<str>>,
  /// What it is called to what reads the tree.
  pub label: Arc<str>,
  pub icon: Option<Name>,
  /// Files it takes are dragged over it.
  pub drag_over: bool,
  /// Files it takes none of are dragged over it.
  pub rejected: bool,
  pub invalid: bool,
  pub disabled: bool,
  /// The style of each part of this zone.
  pub recipe: ResolvedRecipe,
  forced: Option<Interaction>,
  takes: Takes,
  on_files: Vec<FilesHandler>,
  on_reject: Vec<RejectHandler>,
  host: Option<Hosted>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl DropZoneView {
  /// The style of a part, as this zone is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The zone's behavior with the style of its root: the element files are dropped on and that is pressed,
  /// focused and disabled, with nothing in it yet. It is a button to what reads the tree, called by its label.
  pub fn zone(&self) -> crate::components::Button {
    let mut zone = pressable(&self.part(DropZonePart::Root), self.disabled)
      .justify(Justify::Center)
      .align_items(Alignment::Center)
      .describe("a11y_role", "button")
      .describe("a11y_name", self.label.clone());
    if !self.disabled {
      zone = self.hear_files(zone);
    }
    if let Some(id) = &self.id {
      zone = zone.id(id.clone());
    }
    if let Some(key) = &self.key {
      zone = zone.key(key.clone());
    }
    if let Some(Hosted {
      focus: Some((Refocus::Trigger, element)),
      ..
    }) = &self.host
    {
      zone = zone.ref_element(element.clone());
    }
    zone
  }

  /// What the zone does with a drag of files and a press.
  fn hear_files(&self, zone: crate::components::Button) -> crate::components::Button {
    let (takes, on_files, on_reject) = (self.takes.clone(), self.on_files.clone(), self.on_reject.clone());
    let over = self.host.as_ref().map(|host| host.over.clone());
    let mut zone = zone.on_file_drop({
      let over = over.clone();
      move |event: FileDropEvent| {
        if let Some(over) = &over {
          over.set(None);
        }
        let (taken, rejected) = takes.sort_dragged(&event.paths);
        deliver(&on_files, &on_reject, &taken, &rejected);
      }
    });
    if let Some(over) = over {
      let takes = self.takes.clone();
      zone = zone
        .on_file_drag_enter({
          let over = over.clone();
          move |event: FileDragEvent| {
            let (taken, _) = takes.sort_dragged(&event.paths);
            if taken.is_empty() {
              event.refuse();
            }
            over.set(Some(!taken.is_empty()));
          }
        })
        .on_file_drag_leave(move || over.set(None));
    }
    if let Some(host) = &self.host {
      let (choose, dialog) = (host.choose.clone(), self.takes.dialog(&self.label));
      zone = zone.on_click(move |_| choose(dialog.clone()));
    }
    zone
  }

  /// The icon over the words, in the style of its part.
  pub fn icon(&self) -> Option<Element> {
    let icon = self.icon?;
    Some(glyph(&self.part(DropZonePart::Icon), icon, false))
  }

  /// The title and the description, each wrapping where it is longer than the zone.
  pub fn text(&self) -> Column {
    let fill = Dimension::Pct(100.0);
    let line = |part: ResolvedPart, content: &str| {
      Column::new()
        .width(fill)
        .align_items(Alignment::Center)
        .part(&part)
        .child(part.text(content).width(fill).text_align(TextAlign::Center))
    };
    let mut text = Column::new()
      .width(fill)
      .align_items(Alignment::Center)
      .part(&self.part(DropZonePart::Text))
      .child(line(self.part(DropZonePart::Title), &self.title));
    if let Some(description) = &self.description {
      text = text.child(line(self.part(DropZonePart::Description), description));
    }
    text
  }

  /// How the default renderer builds a zone: the icon over the title and the description, in the middle of it,
  /// the root's gap apart.
  pub fn render(self) -> Element {
    let gap = self.part(DropZonePart::Root).rest().gap.unwrap_or_default();
    let mut column = Column::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Center)
      .spacing(gap.resolve(&self.theme().spacing(), 0.0));
    if let Some(icon) = self.icon() {
      column = column.child(icon);
    }
    self.zone().child(column.child(self.text())).into()
  }
}

/// Tells the handlers what was taken and what was not: each only where there is something to tell.
pub(crate) fn deliver(
  on_files: &[FilesHandler],
  on_reject: &[RejectHandler],
  taken: &[DroppedFile],
  rejected: &[RejectedFile],
) {
  if !taken.is_empty() {
    for handler in on_files {
      handler(taken);
    }
  }
  if !rejected.is_empty() {
    for handler in on_reject {
      handler(rejected);
    }
  }
}

impl Themed for DropZone {
  const NAME: &'static str = "drop_zone";
  type Renderer = DropZoneRenderer;

  /// The design's drop zone (`pencil/lurq-design-system.pen`, Drop Zone): 320 by 140 on the muted fill, with the
  /// large radius and an edge of 1.5 in the input's color inside it; in its middle the upload icon of 20 in the
  /// muted foreground, 8 over "Drop files here" in 14 medium and "or click to browse" in the caption's 12 in the
  /// muted foreground. The board draws it at rest only. A drag it takes is drawn as the board's Drop Target: the
  /// info's muted fill, an edge of 2 in the info color, the icon and the words in it; a drag it refuses the same in
  /// the destructive colors. Under the pointer its edge is the ring's color, focused from the keyboard it has the
  /// focus ring; invalid, its edge is destructive; disabled, it is at the disabled opacity.
  fn recipe() -> Recipe {
    use DropZonePart::{Description, Icon, Root, Text, Title};
    use PaletteColor::*;
    let edge = |width: f32, color: PaletteColor| Edge::inside(width, color);
    Recipe::new()
      .base(Root, |s| {
        fading(
          s.width(320.0)
            .max_width(super::Length::Fill)
            .min_height(140.0)
            .padding_x(24.0)
            .padding_y(16.0)
            .gap(8.0)
            .background(Muted)
            .radius(RadiusSize::Lg)
            .border(edge(1.5, Input))
            .cursor(CursorIcon::Pointer),
        )
      })
      .base(Icon, |s| s.square(20.0).foreground(MutedForeground))
      .base(Text, |s| s.gap(4.0))
      .base(Title, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(14.0)
          .weight(FontWeight::Medium)
          .foreground(Foreground)
      })
      .base(Description, |s| {
        s.typography(TypographyStyle::Caption).foreground(MutedForeground)
      })
      .hovered(Root, |s| s.border(edge(1.5, Ring)))
      .focus_visible(Root, |s| s.focus_ring())
      .state(DropZoneState::Invalid, Root, |s| s.border(edge(1.5, Destructive)))
      .state(DropZoneState::DragOver, Root, |s| {
        s.background(InfoMuted).border(edge(2.0, Info))
      })
      .state(DropZoneState::DragOver, Icon, |s| s.foreground(Info))
      .state(DropZoneState::DragOver, Title, |s| s.foreground(Info))
      .state(DropZoneState::Rejected, Root, |s| {
        s.background(DestructiveMuted).border(edge(2.0, Destructive))
      })
      .state(DropZoneState::Rejected, Icon, |s| s.foreground(Destructive))
      .state(DropZoneState::Rejected, Title, |s| s.foreground(Destructive))
      .state(DropZoneState::Disabled, Root, |s| {
        s.opacity(OpacityLevel::Disabled).cursor(CursorIcon::Default)
      })
  }
}

/// A component that takes files from a drop and from the system's dialog, and keeps, mounted, the drag over it
/// and the dialog it opened: a drop zone, a file upload.
pub(crate) trait TakesFiles: Clone + Send + Sync + 'static {
  /// What it takes now.
  fn takes_now(&self) -> Takes;
  /// Tells it what it took and what it did not.
  fn deliver(&self, taken: &[DroppedFile], rejected: &[RejectedFile]);
  /// The element it is, with what its host keeps.
  fn hosted(self, host: Hosted) -> Element;
  fn host_key(&self) -> Option<Arc<str>>;
}

impl TakesFiles for DropZone {
  fn takes_now(&self) -> Takes {
    self.takes.clone()
  }

  fn deliver(&self, taken: &[DroppedFile], rejected: &[RejectedFile]) {
    deliver(&self.on_files, &self.on_reject, taken, rejected);
  }

  fn hosted(mut self, host: Hosted) -> Element {
    self.host = Some(host);
    self.into()
  }

  fn host_key(&self) -> Option<Arc<str>> {
    self.key.clone()
  }
}

/// Mounts `taker` where it is declared; see [`DropZone::mount`].
pub(crate) fn mount_files<C: TakesFiles>(taker: C, ctx: &mut Ctx) -> Element {
  let key = taker.host_key();
  let props = FilesProps { taker: Arc::new(taker) };
  match key {
    Some(key) => ctx.mount_keyed::<FilesHost<C>>(&key, props),
    None => ctx.mount::<FilesHost<C>>(props),
  }
}

/// What takes files, to mount.
struct FilesProps<C> {
  taker: Arc<C>,
}

impl<C> Clone for FilesProps<C> {
  fn clone(&self) -> Self {
    Self {
      taker: self.taker.clone(),
    }
  }
}

impl<C> PartialEq for FilesProps<C> {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

impl<C> crate::app::component::DevtoolsInspectable for FilesProps<C> {}

/// What a mounted taker of files keeps: the drag over it, and the dialog it opened.
struct FilesHost<C> {
  over: Signal<Option<bool>>,
  refocus: Arc<Mutex<Option<Refocus>>>,
  choose: FutureAction<FileDialog, Vec<DroppedFile>, ()>,
  /// The taker as it was rendered last, which the dialog's answer is told to.
  latest: Arc<Mutex<Option<C>>>,
}

impl<C: TakesFiles> Component for FilesHost<C> {
  type Props = FilesProps<C>;

  fn create(ctx: &mut Ctx) -> Self {
    let window = ctx.window();
    let choose = ctx.future_action(move |dialog: FileDialog| dialog.pick_result(&window));
    let latest: Arc<Mutex<Option<C>>> = Arc::new(Mutex::new(None));
    ctx.watch(&choose.state(), {
      let latest = latest.clone();
      move |state| {
        let Some(Ok(chosen)) = state.outcome() else {
          return;
        };
        let Some(taker) = latest.lock().unwrap().clone() else {
          return;
        };
        let (taken, rejected) = taker.takes_now().sort(chosen.iter().map(|file| (file.clone(), false)));
        taker.deliver(&taken, &rejected);
      }
    });
    Self {
      over: ctx.signal(None),
      refocus: Arc::default(),
      choose,
      latest,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let taker = (*ctx.props::<FilesProps<C>>().taker).clone();
    *self.latest.lock().unwrap() = Some(taker.clone());
    let choose = self.choose.clone();
    let focus = self.refocus.lock().unwrap().take().map(|target| {
      let element = crate::core::ElementRef::new();
      ctx.focus(&element);
      (target, element)
    });
    taker.hosted(Hosted {
      over: self.over.clone(),
      refocus: self.refocus.clone(),
      focus,
      choose: Arc::new(move |dialog| {
        // A second press while the dialog is open opens no second one.
        choose.run_if_idle(dialog);
      }),
    })
  }
}
