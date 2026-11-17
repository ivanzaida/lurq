//! A path field: the path of a folder or a file, typed in or chosen with the button beside it.

use std::sync::Arc;

use parking_lot::Mutex;

use super::{
  button::{Button, ButtonPart, ButtonVariant},
  field::{FieldControl, InField},
  icons,
  input::{Input, InputPart, InputView},
  names,
  part_style::{FontFamily, Length, PartStyle},
  recipe::{Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::{IconChoice, shown_icon},
  text_box::TextBoxSize,
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    component::Component,
    ctx::{Ctx, FutureAction},
    events::{FileDropEvent, TextInputEvent},
    theme::{IconSize, PaletteColor, Theme},
  },
  components::Row,
  core::Signal,
  files::FileDialog,
  layout::Alignment,
  node::Element,
};

/// The parts of a path field a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PathFieldPart {
  /// The field and its button, side by side: its width, and the room between the two.
  Root,
  /// What the path field says of the root of the input it is made of.
  Field,
  /// What it says of the input's box: the type of a path.
  Control,
  /// What it says of the icon at the start of the box.
  Icon,
  /// What it says of the root of the button that browses.
  Button,
}

names!(PathFieldPart {
  Root => "root",
  Field => "field",
  Control => "control",
  Icon => "icon",
  Button => "button",
});

/// The states of a path field a recipe styles: an input's, which it shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PathFieldState {
  /// What it holds is wrong.
  Invalid,
  /// What it holds is allowed, with a caveat.
  Warning,
  /// What it holds was checked and is right.
  Valid,
  /// It can be read, selected and copied, and not changed: its button takes nothing.
  ReadOnly,
  /// It takes no input.
  Disabled,
  /// It is as wide as what holds it.
  FullWidth,
}

names!(PathFieldState {
  Invalid => "invalid",
  Warning => "warning",
  Valid => "valid",
  ReadOnly => "read_only",
  Disabled => "disabled",
  FullWidth => "full_width",
});

/// The sizes of a path field: an input's. The board's is the small one, which is the one it has unless told.
pub type PathFieldSize = TextBoxSize;

/// What a path field's path is the path of, and so what the system's dialog chooses when its button opens one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PathKind {
  /// A folder that is there.
  #[default]
  Folder,
  /// A file that is there, to open.
  File,
  /// A file to save: one that is there, or a new name in a folder.
  SaveFile,
}

/// What builds a path field's nodes: a theme's in place of the default one.
pub type PathFieldRenderer = Arc<dyn Fn(PathFieldView) -> Element + Send + Sync>;

type BrowseHandler = Arc<dyn Fn() + Send + Sync>;
type PickHandler = Arc<dyn Fn(&str) + Send + Sync>;

/// What a mounted path field gives the field it builds: what opens the system's dialog.
#[derive(Clone)]
struct Hosted {
  choose: Arc<dyn Fn(FileDialog, PathKind) + Send + Sync>,
}

/// A path field: an [`Input`] that holds the path of a folder or a file, with a button beside it that browses
/// for one. Where a project is kept, the file a report is written to, the program that opens a kind of file.
///
/// ```
/// use lurq::{core::Signal, files::FileDialog, ui::{PathField, PathKind}};
///
/// # fn build(ctx: &mut lurq::app::ctx::Ctx) {
/// let folder = Signal::new("H:\\projects\\lurq".to_owned());
/// // The app answers the button: it opens what it likes and sets the signal.
/// let told = PathField::new(folder.clone()).on_browse(|| println!("choose a folder"));
/// // Mounted and told what its path is of, the button opens the system's dialog and sets the path chosen.
/// let chosen = PathField::new(folder.clone())
///   .picks(PathKind::Folder)
///   .dialog(FileDialog::new().title("Project folder"))
///   .on_pick(|path| println!("the folder is {path}"))
///   .mount(ctx);
/// # }
/// ```
///
/// It is an input in every other way: its sizes, its judgments, read only, disabled, a field's label. What is
/// typed goes into the signal; Enter in it is told to [`on_submit`](Self::on_submit). The button is a stop of Tab
/// after the input, called by what it says, and tells [`on_browse`](Self::on_browse) of a press. A field that
/// [picks](Self::picks) and is [mounted](Self::mount) then opens the system's dialog for a folder, a file or a
/// place to save, starting where its path is, sets the path chosen on the signal and tells
/// [`on_pick`](Self::on_pick); a folder or a file dropped on it from the system is taken the same way. The dialog
/// is the system's with the feature `file_dialog`; see [`lurq::files`](crate::files) for what each platform
/// gives. Converted into an element on its own it is its still form, whose button tells `on_browse` and opens
/// nothing.
#[derive(Clone)]
pub struct PathField {
  input: Input,
  value: Signal<String>,
  picks: Option<PathKind>,
  dialog: FileDialog,
  icon: Option<IconChoice>,
  browse_label: Arc<str>,
  on_browse: Vec<BrowseHandler>,
  on_pick: Vec<PickHandler>,
  full_width: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  host: Option<Hosted>,
}

impl PathField {
  /// A path field bound to `value`: it shows the path the signal holds, and what is typed or chosen is set on it.
  pub fn new(value: Signal<String>) -> Self {
    Self {
      input: Input::new(value.clone()).size(PathFieldSize::Sm),
      value,
      picks: None,
      dialog: FileDialog::new(),
      icon: None,
      browse_label: super::current_locale().fields.browse.clone(),
      on_browse: Vec::new(),
      on_pick: Vec::new(),
      full_width: false,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      host: None,
    }
  }

  /// What the path is the path of: what the system's dialog chooses when the button of a mounted field opens it,
  /// and what the field takes of what is dropped on it (a folder, or a file). Without it the button opens nothing:
  /// it tells [`on_browse`](Self::on_browse), and what is dropped is taken whatever it is.
  pub fn picks(mut self, kind: PathKind) -> Self {
    self.picks = Some(kind);
    self
  }

  /// The system's dialog as the button opens it: its title, the kinds of file it shows, the name a file to save
  /// starts with. It opens where the field's path is unless it is given a folder of its own.
  pub fn dialog(mut self, dialog: FileDialog) -> Self {
    self.dialog = dialog;
    self
  }

  /// What the field says while it is empty.
  pub fn placeholder(mut self, placeholder: &str) -> Self {
    self.input = self.input.placeholder(placeholder);
    self
  }

  /// The glyph at the start of the box, by its name in the theme's set, in place of the folder (the file, for a
  /// field that picks one); or none, `icon(None)`.
  pub fn icon(mut self, icon: impl Into<IconChoice>) -> Self {
    self.icon = Some(icon.into());
    self
  }

  /// What the button says, and is called to tooling, in place of "Browse…".
  pub fn browse_label(mut self, label: &str) -> Self {
    self.browse_label = Arc::from(label);
    self
  }

  /// Called on a press of the button, before the system's dialog opens where the field opens one. A second
  /// handler is called after the first.
  pub fn on_browse(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_browse.push(Arc::new(handler));
    self
  }

  /// Called with the path a person chose in the system's dialog, or dropped on the field, once it is set on the
  /// signal. Not for what is typed ([`on_input`](Self::on_input)), nor for what the app sets. A second handler is
  /// called after the first.
  pub fn on_pick(mut self, handler: impl Fn(&str) + Send + Sync + 'static) -> Self {
    self.on_pick.push(Arc::new(handler));
    self
  }

  /// A [`PathFieldSize`], or a size the theme's recipe names: the input's and the button's. Small unless told.
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.input = self.input.size(size);
    self
  }

  /// Whether the path is wrong. Of invalid, warning and valid, the first that is set is the one shown.
  pub fn invalid(mut self, invalid: bool) -> Self {
    self.input = self.input.invalid(invalid);
    self
  }

  /// Whether the path is allowed with a caveat.
  pub fn warning(mut self, warning: bool) -> Self {
    self.input = self.input.warning(warning);
    self
  }

  /// Whether the path was checked and is right.
  pub fn valid(mut self, valid: bool) -> Self {
    self.input = self.input.valid(valid);
    self
  }

  /// Whether the path can be read, selected and copied, and not changed: the button takes nothing either.
  pub fn read_only(mut self, read_only: bool) -> Self {
    self.input = self.input.read_only(read_only);
    self
  }

  /// Whether the field and its button take no input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.input = self.input.disabled(disabled);
    self
  }

  /// What the field is called, to what reads the tree. A [`Field`](super::Field) gives it its label.
  pub fn label(mut self, label: &str) -> Self {
    self.input = self.input.label(label);
    self
  }

  /// Whether the field is as wide as what holds it, in place of the design's 440.
  pub fn full_width(mut self, full_width: bool) -> Self {
    self.full_width = full_width;
    self
  }

  /// Called before an edit is made, with what the input will hold: a handler may refuse it.
  pub fn on_input(mut self, handler: impl Fn(&TextInputEvent) + Send + Sync + 'static) -> Self {
    self.input = self.input.on_input(handler);
    self
  }

  /// Called with the path when Enter is pressed in the field: a path typed, to go to.
  pub fn on_submit(mut self, handler: impl Fn(&str) + Send + Sync + 'static) -> Self {
    self.input = self.input.on_submit(handler);
    self
  }

  /// The name the path has in the form it is in.
  #[cfg(feature = "form")]
  pub fn name(mut self, name: impl Into<Arc<str>>) -> Self {
    self.input = self.input.name(name);
    self
  }

  /// Puts the path field in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this path field over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this path field.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the input, where Tab stops first, as it is under the pointer or focused, with nothing pointing at it:
  /// for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self.input = self.input.force(interaction);
    self
  }

  /// The id of the control: what is typed in and focused. The button has it with `-browse` after it.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    let id = id.into();
    self.input = self.input.id(id.clone());
    self.id = Some(id);
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.input = self.input.key(key);
    self
  }

  /// The path field mounted where it is declared: where it [picks](Self::picks), its button opens the system's
  /// dialog, which it keeps from render to render, and what is chosen is set on the signal.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let key = self.input.key.clone();
    let props = PathFieldProps { field: Arc::new(self) };
    match key {
      Some(key) => ctx.mount_keyed::<PathFieldHost>(&key, props),
      None => ctx.mount::<PathFieldHost>(props),
    }
  }

  /// Builds the path field in `theme`. Converting it into an element does this in the theme of what is being
  /// rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let (disabled, read_only) = (self.input.boxed.disabled, self.input.boxed.read_only);
    let case = self
      .input
      .boxed
      .case(self.states.clone())
      .state(PathFieldState::FullWidth, self.full_width);
    let recipe = theme.resolve::<PathField>(&case, Some(&self.rules));
    let default = match self.picks {
      Some(PathKind::File | PathKind::SaveFile) => icons::FILE,
      Some(PathKind::Folder) | None => icons::FOLDER,
    };
    // What the path field says of the input it is made of, over the theme's rules for an input.
    let dress = recipe
      .rules_for(PathFieldPart::Field, InputPart::Root)
      .extend(&recipe.rules_for(PathFieldPart::Control, InputPart::Control))
      .extend(&recipe.rules_for(PathFieldPart::Icon, InputPart::Icon));
    let size = self.input.boxed.size;
    let input = self
      .input
      .with_icon(shown_icon(self.icon, default))
      .full_width(true)
      .rules(&dress);
    let taken = Taken {
      value: self.value,
      on_pick: self.on_pick,
    };
    let view = PathFieldView {
      input: input.view(theme),
      browse_label: self.browse_label,
      size,
      picks: self.picks,
      disabled,
      read_only,
      recipe,
      forced: self.forced,
      on_browse: self.on_browse,
      dialog: self.dialog,
      taken,
      host: self.host,
      id: self.id,
    };
    match theme.renderer::<PathField>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl FieldControl for PathField {
  /// Called by the field's label unless it has a name of its own, judged as the field is unless the app judged
  /// the path itself, and disabled where the field is.
  fn in_field(mut self, field: &InField) -> Element {
    self.input.boxed.in_field(field);
    self.into()
  }

  /// Told what the field says, as in `in_field`, and mounted: the same control as outside a field.
  fn mount_in_field(mut self, field: &InField, ctx: &mut Ctx) -> Element {
    self.input.boxed.in_field(field);
    self.mount(ctx)
  }
}

impl From<PathField> for Element {
  fn from(field: PathField) -> Self {
    field.build(&current_theme())
  }
}

/// What takes a path a person chose or dropped: the signal it is set on, and who is told.
#[derive(Clone)]
struct Taken {
  value: Signal<String>,
  on_pick: Vec<PickHandler>,
}

impl Taken {
  /// Sets `path` on the signal, then tells the handlers.
  fn take(&self, path: &str) {
    self.value.set(path.to_owned());
    for handler in &self.on_pick {
      handler(path);
    }
  }
}

/// A path field to mount.
#[derive(Clone)]
struct PathFieldProps {
  field: Arc<PathField>,
}

/// A path field is built anew by every render of what holds it, so props are never the same as before.
impl PartialEq for PathFieldProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

impl crate::app::component::DevtoolsInspectable for PathFieldProps {}

/// What a mounted path field keeps: the dialog it opened, and the field as it was rendered last, which the
/// dialog's answer is told to.
struct PathFieldHost {
  choose: FutureAction<(FileDialog, PathKind), Option<String>, ()>,
  latest: Arc<Mutex<Option<Taken>>>,
}

impl Component for PathFieldHost {
  type Props = PathFieldProps;

  fn create(ctx: &mut Ctx) -> Self {
    let window = ctx.window();
    let choose = ctx.future_action(move |(dialog, kind): (FileDialog, PathKind)| {
      let window = window.clone();
      async move {
        let chosen = match kind {
          PathKind::Folder => dialog.pick_folder(&window).await,
          PathKind::SaveFile => dialog.pick_save(&window).await,
          PathKind::File => dialog
            .multiple(false)
            .pick(&window)
            .await
            .into_iter()
            .next()
            .and_then(|file| file.path().map(std::path::Path::to_path_buf)),
        };
        Ok::<_, ()>(chosen.map(|path| path.to_string_lossy().into_owned()))
      }
    });
    let latest: Arc<Mutex<Option<Taken>>> = Arc::new(Mutex::new(None));
    ctx.watch(&choose.state(), {
      let latest = latest.clone();
      move |state| {
        let Some(Ok(Some(path))) = state.outcome() else {
          return;
        };
        let taken = latest.lock().clone();
        if let Some(taken) = taken {
          taken.take(path);
        }
      }
    });
    Self { choose, latest }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let mut field = (*ctx.props::<PathFieldProps>().field).clone();
    *self.latest.lock() = Some(Taken {
      value: field.value.clone(),
      on_pick: field.on_pick.clone(),
    });
    let choose = self.choose.clone();
    field.host = Some(Hosted {
      choose: Arc::new(move |dialog, kind| {
        // A second press while the dialog is open opens no second one.
        choose.run_if_idle((dialog, kind));
      }),
    });
    field.build(&current_theme())
  }
}

/// A path field as a renderer sees it: the input it is made of, the style of its own parts, and its behavior.
pub struct PathFieldView {
  /// The input the path is typed in, with what the path field says of it laid over its recipe.
  pub input: InputView,
  /// What the button says.
  pub browse_label: Arc<str>,
  /// The size of the field: the button's too.
  pub size: Name,
  /// What the path is of, where the field says.
  pub picks: Option<PathKind>,
  pub disabled: bool,
  pub read_only: bool,
  /// The style of each of the path field's own parts.
  pub recipe: ResolvedRecipe,
  forced: Option<Interaction>,
  on_browse: Vec<BrowseHandler>,
  dialog: FileDialog,
  taken: Taken,
  host: Option<Hosted>,
  id: Option<Arc<str>>,
}

impl PathFieldView {
  /// The style of a part, as this path field is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// Whether the path can be changed by the button and by a drop: the field is neither disabled nor read only.
  pub fn takes(&self) -> bool {
    !self.disabled && !self.read_only
  }

  /// What a press of the button does: the app's handlers, then the system's dialog where the field picks and is
  /// mounted, opening where the path is.
  pub fn browse(&self) -> impl Fn() + Send + Sync + Clone + 'static {
    let handlers = self.on_browse.clone();
    let open = self.host.clone().zip(self.picks);
    let (dialog, value) = (self.dialog.clone(), self.taken.value.clone());
    move || {
      for handler in &handlers {
        handler();
      }
      if let Some((host, kind)) = &open {
        (host.choose)(starting_at(dialog.clone(), &value.get_untracked(), *kind), *kind);
      }
    }
  }

  /// The input in its place: it takes the room the button leaves, and a folder or a file dropped on it from the
  /// system is its path.
  pub fn field(&self) -> Row {
    let mut field = Row::new()
      .flex(1.0)
      .min_width(0.0)
      // The ring of the focus is the control's own edge; a judgment's is drawn in it too.
      .overflow_visible()
      .child(self.input.render_with_end(self.input.control(), None));
    if self.takes() {
      let (taken, picks) = (self.taken.clone(), self.picks);
      field = field.on_file_drop(move |event: FileDropEvent| {
        let dropped = match picks {
          Some(PathKind::Folder) => event.paths.folders().next(),
          Some(PathKind::File | PathKind::SaveFile) => event.paths.files().next(),
          None => event.paths.paths().next(),
        };
        if let Some(path) = dropped {
          taken.take(&path.to_string_lossy());
        }
      });
    }
    field
  }

  /// The button that browses: an outlined [`Button`] of the field's size, a stop of Tab after the input.
  pub fn button(&self) -> Element {
    let rules = self.recipe.rules_for(PathFieldPart::Button, ButtonPart::Root);
    let browse = self.browse();
    let mut button = Button::new(&self.browse_label)
      .variant(ButtonVariant::Outline)
      .size(self.size)
      .disabled(!self.takes())
      .rules(&rules)
      .on_click(move |_| browse());
    if let Some(id) = &self.id {
      button = button.id(format!("{id}-browse"));
    }
    button.build(self.theme())
  }

  /// The root in the style of its part, with nothing in it yet.
  pub fn root(&self) -> Row {
    Row::new()
      .align_items(Alignment::Center)
      .overflow_visible()
      .part(&self.part(PathFieldPart::Root))
  }

  /// How the default renderer builds a path field: the input, and the button after it.
  pub fn render(self) -> Element {
    self.root().child(self.field()).child(self.button()).into()
  }
}

/// `dialog` opening where `path` is, unless it has a folder of its own: in the folder itself for a dialog that
/// chooses one, in the folder of the file for the others.
fn starting_at(dialog: FileDialog, path: &str, kind: PathKind) -> FileDialog {
  let path = std::path::Path::new(path.trim());
  let folder = match kind {
    PathKind::Folder => Some(path),
    PathKind::File | PathKind::SaveFile => path.parent(),
  };
  match folder.filter(|folder| !folder.as_os_str().is_empty()) {
    Some(folder) => dialog.folder_unless_given(folder),
    None => dialog,
  }
}

impl crate::ui::Styled for PathField {
  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for PathField {
  const NAME: &'static str = "path_field";
  type Renderer = PathFieldRenderer;

  fn parts() -> crate::ui::Declared {
    PathFieldPart::declared()
  }

  fn states() -> crate::ui::Declared {
    PathFieldState::declared()
  }

  /// The design's path field (`pencil/lurq-design-system.pen`, Path Field): 440 wide, an input and an outlined
  /// button 8 apart, both 32 high. The input's path is in the mono type at 12, 10 from its edge, after a folder of
  /// 16 in the muted foreground; the button is 12 in from its sides. The board draws the small size, which is the
  /// one a path field has unless told: a medium one is 36 with a path of 13, a large one 40 with a path of 14.
  fn recipe() -> Recipe {
    use PathFieldPart::{Control, Field, Icon, Root};
    Recipe::new()
      .base(Root, |s| s.width(440.0).max_width(Length::Fill).gap(8.0))
      .state(PathFieldState::FullWidth, Root, |s| s.width(Length::Fill))
      .base(Field, |s| s.width(Length::Fill))
      .base(Control, |s| {
        s.font_family(FontFamily::Mono).font_size(12.0).padding_x(10.0)
      })
      .size(PathFieldSize::Md, Control, |s| s.font_size(13.0))
      .size(PathFieldSize::Lg, Control, |s| s.font_size(14.0))
      .base(Icon, |s| {
        s.square(IconSize::Md).foreground(PaletteColor::MutedForeground)
      })
  }
}
