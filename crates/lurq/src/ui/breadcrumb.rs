//! A breadcrumb: the way to where one is, a step for each place on it. As a path control, the folders of a path in
//! a bar, as a file manager has them.

use std::{ops::Range, sync::Arc};

use parking_lot::Mutex;

use super::{
  icons,
  menu::{Menu, MenuItem},
  names,
  part_style::Length,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::{fading, glyph, opens_aside, pressable, tab_stop},
  text_box::edge,
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    component::Component,
    ctx::{Ctx, Placement},
    theme::{IconSize, OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::{Row, TextOverflow},
  core::{ElementRef, Signal},
  layout::{
    Alignment,
    layout_kind::{Justify, ShrinkLimit},
    text_style::FontWeight,
  },
  node::{CursorIcon, Element, EventHandler, IntoMouseEventHandler, color::Color},
};

/// The parts of a breadcrumb a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BreadcrumbPart {
  /// The row of the steps and what stands between them.
  Root,
  /// A step that is a link: what is pressed to go there, with its icon and its text.
  Link,
  /// The last step: the page one is on, which is text and not a link.
  Page,
  /// A step that is neither a link nor the page one is on: a place on the way that cannot be gone to
  /// ([`BreadcrumbStep::link(false)`](BreadcrumbStep::link)).
  Text,
  /// The icon of a step, before its text.
  Icon,
  /// What stands between two steps: an icon, or a text such as a slash.
  Separator,
  /// The icon of a separator.
  SeparatorIcon,
  /// What stands for the steps that are hidden: what is pressed to open the menu of them.
  Ellipsis,
  /// The icon of the ellipsis.
  EllipsisIcon,
}

names!(BreadcrumbPart {
  Root => "root",
  Link => "link",
  Page => "page",
  Text => "text",
  Icon => "icon",
  Separator => "separator",
  SeparatorIcon => "separator_icon",
  Ellipsis => "ellipsis",
  EllipsisIcon => "ellipsis_icon",
});

/// The states of a breadcrumb a recipe styles, besides what the pointer and
/// the keyboard do to its steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BreadcrumbState {
  /// Some steps are hidden behind the ellipsis.
  Collapsed,
  /// It takes no input, or, of a step, the step is not followed.
  Disabled,
}

names!(BreadcrumbState {
  Collapsed => "collapsed",
  Disabled => "disabled",
});

/// How a breadcrumb is drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BreadcrumbVariant {
  /// Links in a line of text, the page one is on after them: the way through an app's pages.
  #[default]
  Plain,
  /// Segments in a bar, the last in a box of its own: the path control of a file manager, the folders of a path.
  Path,
}

names!(BreadcrumbVariant {
  Plain => "plain",
  Path => "path",
});

/// What builds a breadcrumb's nodes: a theme's in place of the default one.
pub type BreadcrumbRenderer = Arc<dyn Fn(BreadcrumbView) -> Element + Send + Sync>;

/// What builds one part of a breadcrumb: a theme's, for
/// [`BreadcrumbPart::Separator`], built once for each place between two steps.
pub type BreadcrumbPartRenderer = Arc<dyn Fn(&BreadcrumbView) -> Element + Send + Sync>;

/// What stands between two steps of a breadcrumb.
#[derive(Clone, Debug, PartialEq)]
pub enum BreadcrumbSeparator {
  /// An icon of the theme's set, by its name: a chevron to the right, the design's.
  Icon(Name),
  /// A text: a slash.
  Text(Arc<str>),
}

impl BreadcrumbSeparator {
  /// A chevron to the right: the design's.
  pub fn chevron() -> Self {
    Self::Icon(Name::new(icons::CHEVRON_RIGHT))
  }

  /// A slash, in the type of the steps.
  pub fn slash() -> Self {
    Self::Text(Arc::from("/"))
  }
}

impl Default for BreadcrumbSeparator {
  fn default() -> Self {
    Self::chevron()
  }
}

type Handler = Arc<dyn Fn() + Send + Sync>;
type ValueHandler = Arc<dyn Fn(&str) + Send + Sync>;

/// A step of a breadcrumb: a place on the way to where one is.
///
/// ```
/// use lurq::ui::{BreadcrumbStep, icons};
///
/// let home = BreadcrumbStep::new("Home").icon(icons::HOME).value("/").on_select(|| println!("home"));
/// let plain: BreadcrumbStep = "Settings".into();
/// let closed = BreadcrumbStep::new("Archive").disabled(true);
/// // In a state the theme's recipe names: a key in the family of code, a folder drawn as the steps before it.
/// let key = BreadcrumbStep::new("KON-12").state("mono", true);
/// ```
#[derive(Clone)]
pub struct BreadcrumbStep {
  label: Arc<str>,
  value: Arc<str>,
  icon: Option<Name>,
  disabled: bool,
  link: Option<bool>,
  states: Vec<Name>,
  on_select: Vec<Handler>,
  on_open_aside: Vec<EventHandler<crate::app::events::MouseEvent>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl BreadcrumbStep {
  /// A step called `label`, whose value is its label until it is given another.
  pub fn new(label: &str) -> Self {
    let label: Arc<str> = Arc::from(label);
    Self {
      value: label.clone(),
      label,
      icon: None,
      disabled: false,
      link: None,
      states: Vec::new(),
      on_select: Vec::new(),
      on_open_aside: Vec::new(),
      id: None,
      key: None,
    }
  }

  /// What the step stands for, which the breadcrumb's handler is told when it
  /// is followed: a route, a path, a key.
  pub fn value(mut self, value: &str) -> Self {
    self.value = Arc::from(value);
    self
  }

  /// An icon before the text, by its name in the theme's set.
  pub fn icon(mut self, name: impl Into<Name>) -> Self {
    self.icon = Some(name.into());
    self
  }

  /// Whether the step is not followed: it is dimmed, not a stop of Tab, and its
  /// item in the menu of the hidden steps is disabled. A place one may not go to
  /// from here.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Whether the step is a link, over what its place makes it: every step but the last is one unless told otherwise,
  /// and the last is the page one is on. `link(true)` on the last step makes it a link like the others, and the trail
  /// then has no current page: an editor's form whose trail ends with the record it edits. `link(false)` on a step
  /// before it makes it plain text ([`BreadcrumbPart::Text`]): a place on the way that cannot be gone to, which is no
  /// stop of Tab, is not followed, and is disabled in the menu of the hidden steps. `link(false)` on the last step
  /// leaves it the page one is on.
  ///
  /// ```
  /// use lurq::ui::{Breadcrumb, BreadcrumbStep};
  ///
  /// // Back to the task from its edit form: the trail ends with a link.
  /// let edit = Breadcrumb::new([BreadcrumbStep::new("Tasks"), BreadcrumbStep::new("KON-12").link(true)]);
  /// // A local folder that is no page of the app, before the pages that are.
  /// let sharing = Breadcrumb::new([
  ///   BreadcrumbStep::new("~/projects/kon").link(false),
  ///   BreadcrumbStep::new("Settings"),
  ///   BreadcrumbStep::new("Upload"),
  /// ]);
  /// ```
  pub fn link(mut self, link: bool) -> Self {
    self.link = Some(link);
    self
  }

  /// Puts this step alone in a state the theme's recipe names, or takes it out: the parts of the step (its link or
  /// its page, its icon) are styled in it, and the other steps and the separators are not. A key in the family of
  /// code, the page's title in a title's type, a last step drawn as the steps before it:
  /// `recipe.when(Selector::new().state("ancestor"), BreadcrumbPart::Page, |s| s.foreground(MutedForeground))`.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// The states the step is in, of its own.
  pub fn states(&self) -> &[Name] {
    &self.states
  }

  /// Called when the step is followed: a click of it, Enter or Space while it
  /// is focused, or choosing it in the menu of the hidden steps. The page one is
  /// on, and a step that is no [link](Self::link), are not followed.
  pub fn on_select(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_select.push(Arc::new(handler));
    self
  }

  /// Called when a person asks for the step's place beside what is open, in place of following the step: a press of
  /// its link with the middle button, or with Ctrl held (Cmd on macOS), and Enter with it while the link is focused.
  /// An app with tabs opens the place in a new one. Such a press is the handler's alone: the step is not followed,
  /// so neither its [`on_select`](Self::on_select) nor the breadcrumb's is called, and the link takes no focus. It
  /// is given the press, whose `shift` asks for a tab that is switched to. The last step, the page, has no link,
  /// and a hidden step's item in the menu is chosen as before. A second handler is called after the first.
  pub fn on_open_aside(mut self, handler: impl IntoMouseEventHandler) -> Self {
    self.on_open_aside.push(handler.into_event_handler());
    self
  }

  /// The id of what is pressed to follow the step, or of the page for the
  /// last one. A hidden step's item in the menu has it.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// What tells the step apart from the others where two have one label: the key of what is pressed.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// The steps of the path of a file or a folder, one for each part of it, the value of each the path as far as
  /// it: what [`Breadcrumb::path`] is made of. Parts are told apart by `/` and by `\`; a drive (`H:`), the root
  /// (`/`) and a share (`\\nas\media`) are the first step, with the icon of a drive, a `~` has the icon of a home,
  /// and every other part the icon of a folder. The last part is a folder too: a path to a file is given the
  /// file's icon by the app, `steps.pop().map(|file| file.icon(icons::FILE))`.
  ///
  /// ```
  /// use lurq::ui::BreadcrumbStep;
  ///
  /// let steps = BreadcrumbStep::path("H:\\projects\\lurq");
  /// let labels: Vec<_> = steps.iter().map(|step| step.label()).collect();
  /// let values: Vec<_> = steps.iter().map(|step| step.value_of()).collect();
  /// assert_eq!(labels, ["H:", "projects", "lurq"]);
  /// assert_eq!(values, ["H:\\", "H:\\projects", "H:\\projects\\lurq"]);
  /// ```
  pub fn path(path: &str) -> Vec<Self> {
    let separates = |c: char| c == '/' || c == '\\';
    let mut parts: Vec<(usize, usize)> = Vec::new();
    let mut start = None;
    for (at, c) in path.char_indices() {
      match (separates(c), start) {
        (true, Some(from)) => {
          parts.push((from, at));
          start = None;
        }
        (false, None) => start = Some(at),
        _ => {}
      }
    }
    if let Some(from) = start {
      parts.push((from, path.len()));
    }
    let folder = |(_, end): (usize, usize), label: &str| Self::new(label).value(&path[..end]).icon(icons::FOLDER);
    let mut leading = path.chars().take_while(|c| separates(*c));
    let (first, second) = (leading.next(), leading.next());
    let mut steps = Vec::with_capacity(parts.len() + 1);
    let mut rest = parts.as_slice();
    match (first, second, rest) {
      // A share: `\\server\share` is one place.
      (Some(_), Some(_), [_, share, ..]) => {
        steps.push(Self::new(&path[..share.1]).icon(icons::DRIVE));
        rest = &rest[2..];
      }
      // The root of a file system.
      (Some(root), None, _) => steps.push(Self::new(&root.to_string()).icon(icons::DRIVE)),
      // A drive: its root is the drive with a separator after it.
      (None, _, [drive, ..]) if path[drive.0..drive.1].ends_with(':') => {
        let name = &path[drive.0..drive.1];
        let separator = path[drive.1..].chars().next().unwrap_or('\\');
        steps.push(Self::new(name).value(&format!("{name}{separator}")).icon(icons::DRIVE));
        rest = &rest[1..];
      }
      (None, _, [home, ..]) if &path[home.0..home.1] == "~" => {
        steps.push(Self::new("~").icon(icons::HOME));
        rest = &rest[1..];
      }
      _ => {}
    }
    steps.extend(rest.iter().map(|part| folder(*part, &path[part.0..part.1])));
    steps
  }

  pub fn label(&self) -> &str {
    &self.label
  }

  pub fn value_of(&self) -> &str {
    &self.value
  }

  pub fn icon_name(&self) -> Option<&Name> {
    self.icon.as_ref()
  }
}

impl From<&str> for BreadcrumbStep {
  fn from(label: &str) -> Self {
    Self::new(label)
  }
}

/// A breadcrumb: the way to where one is, a step for each place on it in
/// order. Each step but the last is a link that a press follows; the last is
/// the page one is on.
///
/// ```
/// use lurq::{
///   core::{ElementRef, Signal},
///   ui::{Breadcrumb, BreadcrumbStep, icons},
/// };
///
/// # fn build(ctx: &mut lurq::app::ctx::Ctx, open: Signal<bool>, anchor: ElementRef) {
/// let way = Breadcrumb::new([
///   BreadcrumbStep::new("Home").icon(icons::HOME).value("/"),
///   BreadcrumbStep::new("Settings").value("/settings"),
///   BreadcrumbStep::new("Appearance").value("/settings/appearance"),
/// ])
/// .step("Theme")
/// .on_select(|route| println!("go to {route}"));
///
/// // With more than three steps, the first, an ellipsis and the last two; the ellipsis opens a menu of the
/// // others, which the mounted breadcrumb keeps.
/// let short = Breadcrumb::new(["Home", "Projects", "lurq", "crates", "ui", "breadcrumb.rs"])
///   .max_shown(3)
///   .mount(ctx);
/// // The same, with the menu's signal the app's, and what it hangs from.
/// let held = Breadcrumb::new(["Home", "Projects", "lurq", "crates", "ui", "breadcrumb.rs"])
///   .max_shown(3)
///   .menu(open, anchor);
/// # }
/// ```
///
/// A step is followed by a click, and by Enter or Space while it is focused:
/// the links are stops of Tab in their order, and the page is not. Following
/// a step calls its own handler and the breadcrumb's, told its value; where
/// it goes is the app's. The whole is a navigation called "Breadcrumb" to
/// what reads the tree ([`label`](Breadcrumb::label)), and the last step is
/// its current page.
///
/// With more steps than [`max_shown`](Breadcrumb::max_shown), the breadcrumb
/// shows the first, an ellipsis and the last ones, and the ellipsis opens a
/// [`Menu`] of the steps it hides. A mounted breadcrumb
/// ([`mount`](Breadcrumb::mount)) keeps whether the menu is open and what it
/// hangs from; one built on its own opens it only where the app gives it a
/// signal and an anchor ([`menu`](Breadcrumb::menu)).
///
/// In a room narrower than its steps a breadcrumb has two forms. The one that folds is
/// [`overflow(true)`](Breadcrumb::overflow) with [`mount`](Breadcrumb::mount): it hides the steps its room has no
/// place for in the same menu, the ones nearest the first first, and shows them again as it is given room. The
/// still one is every other breadcrumb, one built on its own ([`build`](Breadcrumb::build), `.into()`) and a
/// mounted one that does not overflow: it keeps nothing of what was measured and folds nothing. Its root is no
/// wider than its room and its last step is cut with an ellipsis, but the steps before it keep their widths, so
/// links wider together than the room run past its end. In a window a person can narrow, mount the one that
/// folds.
///
/// ```
/// use lurq::ui::{Breadcrumb, BreadcrumbStep, BreadcrumbVariant, icons};
///
/// # fn build(ctx: &mut lurq::app::ctx::Ctx) {
/// // A path control: the folders of a path in a bar, each told as the path as far as it. It folds to its room.
/// let path = Breadcrumb::path("H:\\projects\\lurq\\crates")
///   .on_select(|folder| println!("open {folder}"))
///   .mount(ctx);
/// // The same bar for steps of the app's own.
/// let places = Breadcrumb::new([
///   BreadcrumbStep::new("Macintosh HD").icon(icons::DRIVE).value("/"),
///   BreadcrumbStep::new("projects").icon(icons::FOLDER).value("/projects"),
/// ])
/// .variant(BreadcrumbVariant::Path);
/// # }
/// ```
#[derive(Clone)]
pub struct Breadcrumb {
  steps: Vec<BreadcrumbStep>,
  variant: Name,
  overflow: bool,
  current: bool,
  /// What a mounted breadcrumb keeps of the steps its row dropped.
  fold: Option<Fold>,
  max_shown: Option<usize>,
  separator: BreadcrumbSeparator,
  menu: Option<(ElementRef, Signal<bool>)>,
  label: Arc<str>,
  more_label: Arc<str>,
  disabled: bool,
  tab_stop: bool,
  cut_links: bool,
  /// The refs of the steps' names a mounted breadcrumb keeps, by the step's index: what a tooltip reads whether the
  /// name is cut from.
  name_refs: Vec<ElementRef>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  on_select: Vec<ValueHandler>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Default for Breadcrumb {
  fn default() -> Self {
    Self::new(Vec::<BreadcrumbStep>::new())
  }
}

impl Breadcrumb {
  /// A breadcrumb of `steps`, in order: the last is the page one is on. A step is a [`BreadcrumbStep`], or the
  /// text of one.
  pub fn new<S: Into<BreadcrumbStep>>(steps: impl IntoIterator<Item = S>) -> Self {
    Self {
      steps: steps.into_iter().map(Into::into).collect(),
      variant: BreadcrumbVariant::default().into(),
      overflow: false,
      current: true,
      fold: None,
      max_shown: None,
      separator: BreadcrumbSeparator::default(),
      menu: None,
      label: super::current_locale().navigation.breadcrumb.clone(),
      more_label: super::current_locale().navigation.show_hidden_steps.clone(),
      disabled: false,
      tab_stop: true,
      cut_links: false,
      name_refs: Vec::new(),
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      on_select: Vec::new(),
      id: None,
      key: None,
    }
  }

  /// A path control: the path of a file or a folder as a bar of its parts ([`BreadcrumbStep::path`]), each but the
  /// last pressed to go to that folder, which [`on_select`](Self::on_select) is told as the path as far as it. It
  /// is the [`BreadcrumbVariant::Path`] of these steps, called "Path", and it [overflows](Self::overflow): mounted,
  /// a path longer than its room folds its middle into the menu of the ellipsis.
  pub fn path(path: &str) -> Self {
    Self::new(BreadcrumbStep::path(path))
      .variant(BreadcrumbVariant::Path)
      .overflow(true)
      .label(&super::current_locale().navigation.path)
  }

  /// A [`BreadcrumbVariant`], or a variant the theme's recipe names.
  pub fn variant(mut self, variant: impl Into<Name>) -> Self {
    self.variant = variant.into();
    self
  }

  /// Whether the steps the breadcrumb has no room for go into the menu of the ellipsis: the ones nearest the first
  /// first, the first and the last never, and back as it is given room. It is what a mounted breadcrumb does
  /// ([`mount`](Self::mount)): it keeps which steps its row dropped. With [`max_shown`](Self::max_shown) too, no
  /// more than that many are shown where there is room for more. On a breadcrumb that is not mounted it does
  /// nothing: built on its own a breadcrumb is its still form, which cuts its last step and folds none.
  pub fn overflow(mut self, overflow: bool) -> Self {
    self.overflow = overflow;
    self
  }

  /// Whether the last step is the page one is on: it is unless told otherwise. `false` is a trail that ends with a
  /// link, as every step before it is (unless that step is told [`link(false)`](BreadcrumbStep::link)): the way to a
  /// page that is not in the trail itself. The same as [`BreadcrumbStep::link(true)`](BreadcrumbStep::link) on the
  /// last step.
  pub fn current(mut self, current: bool) -> Self {
    self.current = current;
    self
  }

  /// A step, after the steps the breadcrumb has already. The last is the page one is on.
  pub fn step(mut self, step: impl Into<BreadcrumbStep>) -> Self {
    self.steps.push(step.into());
    self
  }

  /// Steps, in order, after the steps the breadcrumb has already.
  pub fn steps<S: Into<BreadcrumbStep>>(mut self, steps: impl IntoIterator<Item = S>) -> Self {
    self.steps.extend(steps.into_iter().map(Into::into));
    self
  }

  /// The most steps shown: with more, the first, an ellipsis and the last
  /// `max - 1`. Never fewer than two, the first and the last; all of them
  /// without it.
  pub fn max_shown(mut self, max: usize) -> Self {
    self.max_shown = Some(max);
    self
  }

  /// What the breadcrumb is called to what reads the tree: "Breadcrumb" unless told otherwise.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Arc::from(label);
    self
  }

  /// What the ellipsis is called to what reads the tree: "Show the hidden steps" unless told otherwise.
  pub fn more_label(mut self, label: &str) -> Self {
    self.more_label = Arc::from(label);
    self
  }

  /// What stands between two steps: a chevron to the right unless it is given another.
  pub fn separator(mut self, separator: BreadcrumbSeparator) -> Self {
    self.separator = separator;
    self
  }

  /// Whether the menu of the hidden steps is open, and what it hangs from: the
  /// app's `Signal<bool>`, which a press of the ellipsis turns, and choosing a
  /// step, a press outside and Escape set to `false`. The anchor is the
  /// ellipsis's own: the app keeps it so that the menu stays the same menu
  /// while it is open. A mounted breadcrumb keeps its own where it is not
  /// given them; one built on its own without them opens nothing.
  pub fn menu(mut self, open: Signal<bool>, anchor: ElementRef) -> Self {
    self.menu = Some((anchor, open));
    self
  }

  /// Whether the breadcrumb takes no input: no step is followed.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Whether Tab stops at the links of the steps and at the ellipsis; it does unless told otherwise. `false` is a
  /// breadcrumb the pointer and the app's own shortcuts reach: Tab passes them by, and a press of one follows it
  /// and leaves the focus where it was. A link the app focuses is followed by Enter and Space as before, and they
  /// are the same to what reads the tree.
  pub fn tab_stop(mut self, stops: bool) -> Self {
    self.tab_stop = stops;
    self
  }

  /// The most a link is wide: a longer name is cut there with an ellipsis, and
  /// a tooltip says it whole while it is cut. A link keeps its room, so the
  /// page one is on gives way where the row is short. A number of logical
  /// pixels, or another [`Length`].
  pub fn link_max_width(mut self, width: impl Into<Length>) -> Self {
    let width = width.into();
    self.rules = self.rules.base(BreadcrumbPart::Link, |s| s.max_width(width));
    self.cut_links = true;
    self
  }

  /// Puts the breadcrumb in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Called when a step is followed, with its value, after the step's own handlers.
  pub fn on_select(mut self, handler: impl Fn(&str) + Send + Sync + 'static) -> Self {
    self.on_select.push(Arc::new(handler));
    self
  }

  /// Styles one part of this breadcrumb over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this breadcrumb.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the link Tab would stop on first (the first step's) as it is under
  /// the pointer, held down or focused from the keyboard, with nothing
  /// pointing at it: for a gallery or a test. The other links and the ellipsis
  /// are drawn as they are.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the root, and the prefix of the ids of its parts by the
  /// part-ids rule: the ellipsis `x-ellipsis`, the separators `x-separator`,
  /// `x-separator-2`, `x-separator-3` in their order (a repeated part with no
  /// key of its own is numbered from its second node, as in every component,
  /// so there is no `x-separator-1`). A step's own id is on the step.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Puts the breadcrumb where it goes, keeping whether the menu of the hidden
  /// steps is open and what it hangs from, where the app gives it neither
  /// ([`menu`](Breadcrumb::menu)), and the steps its row dropped where it
  /// [overflows](Breadcrumb::overflow).
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    ctx.mount::<BreadcrumbHost>(BreadcrumbProps { breadcrumb: self })
  }

  /// Builds the breadcrumb in `theme`. Converting a breadcrumb into an
  /// element does this in the theme of what is being rendered. Its ellipsis
  /// opens the menu of the hidden steps where it is given a signal and an
  /// anchor ([`menu`](Breadcrumb::menu)), or mounted. Built so it is the still
  /// form: it folds no step into the menu for want of room, whatever
  /// [`overflow`](Breadcrumb::overflow) says, and only its last step is cut.
  pub fn build(self, theme: &Theme) -> Element {
    let count = self.steps.len();
    let counted = hidden(count, self.max_shown);
    let fold = self.fold.filter(|_| self.overflow);
    // The steps the row dropped are those after the first, as far as the last of them.
    let folded = fold.as_ref().and_then(|fold| {
      (1..count.saturating_sub(1))
        .rev()
        .find(|index| fold.read.get(*index).copied().unwrap_or(false))
        .map(|last| last + 1)
    });
    let hidden = match (&counted, folded) {
      (Some(counted), Some(folded)) => Some(1..counted.end.max(folded)),
      (Some(counted), None) => Some(counted.clone()),
      (None, Some(folded)) => Some(1..folded),
      (None, None) => None,
    };
    let case = Case {
      variant: Some(self.variant),
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(BreadcrumbState::Collapsed, hidden.is_some());
    // A disabled step is drawn in the state of a disabled breadcrumb, alone.
    let disabled = case.clone().state(BreadcrumbState::Disabled, true);
    let case = case.state(BreadcrumbState::Disabled, self.disabled);
    let recipe = theme
      .resolve::<Breadcrumb>(&case, Some(&self.rules))
      .owned_by(self.id.as_ref());
    let disabled_recipe = theme
      .resolve::<Breadcrumb>(&disabled, Some(&self.rules))
      .owned_like(&recipe);
    // A step in states of its own is resolved in them, as the whole and disabled where it is.
    let step_recipes = self
      .steps
      .iter()
      .map(|step| {
        (!step.states.is_empty()).then(|| {
          let base = if self.disabled || step.disabled {
            &disabled
          } else {
            &case
          };
          let case = step
            .states
            .iter()
            .fold(base.clone(), |case, state| case.state(*state, true));
          theme
            .resolve::<Breadcrumb>(&case, Some(&self.rules))
            .owned_like(&recipe)
        })
      })
      .collect();
    let view = BreadcrumbView {
      steps: self.steps,
      hidden,
      counted,
      fold,
      separator: self.separator,
      disabled: self.disabled,
      current: self.current,
      tab_stop: self.tab_stop,
      cut_links: self.cut_links,
      name_refs: self.name_refs,
      recipe,
      disabled_recipe,
      step_recipes,
      label: self.label,
      more_label: self.more_label,
      forced: self.forced,
      menu: self.menu,
      on_select: self.on_select,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Breadcrumb>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<Breadcrumb> for Element {
  fn from(breadcrumb: Breadcrumb) -> Self {
    breadcrumb.build(&current_theme())
  }
}

/// A breadcrumb, mounted.
#[derive(Clone)]
struct BreadcrumbProps {
  breadcrumb: Breadcrumb,
}

/// The breadcrumb is built anew by every render of what holds it, so props are never the same as before.
impl PartialEq for BreadcrumbProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

impl crate::app::component::DevtoolsInspectable for BreadcrumbProps {}

/// What a mounted breadcrumb that overflows gives the breadcrumb it builds: where each step was laid out, and which
/// of them its row dropped.
#[derive(Clone)]
struct Fold {
  /// Which steps the last layout dropped, by index: set as the layout drops one and takes one back.
  dropped: Signal<Vec<bool>>,
  /// What `dropped` held when this render began.
  read: Vec<bool>,
  /// A ref for each step, which says whether it is dropped: a dropped one has no size.
  refs: Vec<ElementRef>,
}

/// What a mounted breadcrumb keeps between renders: whether the menu of the hidden steps is open and what it hangs
/// from, and which steps its row dropped.
struct BreadcrumbHost {
  anchor: ElementRef,
  open: Signal<bool>,
  dropped: Signal<Vec<bool>>,
  refs: Mutex<Vec<ElementRef>>,
  /// The refs of the steps' names, kept from render to render so that a tooltip reads what the last layout cut.
  name_refs: Mutex<Vec<ElementRef>>,
}

impl Component for BreadcrumbHost {
  type Props = BreadcrumbProps;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      anchor: ElementRef::new(),
      open: ctx.signal(false),
      dropped: ctx.signal(Vec::new()),
      refs: Mutex::new(Vec::new()),
      name_refs: Mutex::new(Vec::new()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let mut breadcrumb = ctx.props::<BreadcrumbProps>().breadcrumb.clone();
    if breadcrumb.menu.is_none() {
      breadcrumb.menu = Some((self.anchor.clone(), self.open.clone()));
    }
    // The page's name is cut where the row is short, and the links' at their most width.
    breadcrumb.name_refs = kept(&self.name_refs, breadcrumb.steps.len());
    if breadcrumb.overflow {
      let count = breadcrumb.steps.len();
      let refs = kept(&self.refs, count);
      breadcrumb.fold = Some(Fold {
        read: self.dropped.get(),
        dropped: self.dropped.clone(),
        refs,
      });
    }
    breadcrumb.build(&current_theme())
  }
}

/// The first `count` of the refs `held` keeps, made where it has fewer.
fn kept(held: &Mutex<Vec<ElementRef>>, count: usize) -> Vec<ElementRef> {
  let mut refs = held.lock();
  while refs.len() < count {
    refs.push(ElementRef::new());
  }
  refs[..count].to_vec()
}

/// The steps the ellipsis stands for among `count`, where no more than `max`
/// are shown: all but the first and the last `max - 1`.
fn hidden(count: usize, max: Option<usize>) -> Option<Range<usize>> {
  let max = max?.max(2);
  (count > max).then(|| 1..count - (max - 1))
}

/// A breadcrumb as a renderer sees it: its steps, which of them are hidden,
/// the style of its parts, and what following a step does.
pub struct BreadcrumbView {
  /// Every step, in order, the hidden ones too.
  pub steps: Vec<BreadcrumbStep>,
  /// The steps the ellipsis stands for, if some are hidden: those over [`Breadcrumb::max_shown`], and those the row
  /// of a breadcrumb that overflows dropped.
  pub hidden: Option<Range<usize>>,
  /// The steps hidden for being over the most shown: they are not laid out.
  counted: Option<Range<usize>>,
  /// What a mounted breadcrumb that overflows keeps.
  fold: Option<Fold>,
  pub separator: BreadcrumbSeparator,
  pub disabled: bool,
  /// Whether the last step is the page one is on, unless it is told to be a link ([`Breadcrumb::current`]).
  current: bool,
  /// Whether Tab stops at the links and the ellipsis.
  tab_stop: bool,
  /// Whether a link's name is cut at its most width, with a tooltip of it whole while it is cut
  /// ([`Breadcrumb::link_max_width`]).
  pub cut_links: bool,
  name_refs: Vec<ElementRef>,
  /// The style of each part of this breadcrumb.
  pub recipe: ResolvedRecipe,
  /// The style of the parts of a step that is disabled.
  disabled_recipe: ResolvedRecipe,
  /// The style of the parts of each step in states of its own, by its index: `None` for a step in none.
  step_recipes: Vec<Option<ResolvedRecipe>>,
  /// What the whole is called.
  pub label: Arc<str>,
  /// What the ellipsis is called.
  pub more_label: Arc<str>,
  forced: Option<Interaction>,
  menu: Option<(ElementRef, Signal<bool>)>,
  on_select: Vec<ValueHandler>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl BreadcrumbView {
  /// The style of a part, as this breadcrumb is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  /// The style of a part of the step at `index`: disabled where the step is, in the step's own states where it has
  /// some, and in the interaction forced where it is the link Tab stops on first. Its parts are named after the
  /// step's id, or after its value.
  pub fn step_part(&self, index: usize, part: impl Into<Name>) -> ResolvedPart {
    let forced = self.forced.filter(|_| self.entry() == Some(index));
    let recipe = match self.step_recipes.get(index) {
      Some(Some(own)) => own,
      _ if self.is_disabled(index) => &self.disabled_recipe,
      _ => &self.recipe,
    };
    let step = &self.steps[index];
    let root = if self.is_current(index) {
      BreadcrumbPart::Page
    } else if self.is_link(index) {
      BreadcrumbPart::Link
    } else {
      BreadcrumbPart::Text
    };
    let recipe = match &step.id {
      Some(id) => recipe.item(Some(id), root),
      None => recipe.keyed_item(&step.value, root),
    };
    recipe.part(part).forced(forced)
  }

  /// Whether the step at `index` takes no input: it is disabled, or the whole breadcrumb is.
  pub fn is_disabled(&self, index: usize) -> bool {
    self.disabled || self.steps.get(index).is_some_and(|step| step.disabled)
  }

  /// The step whose link Tab stops on first: the first shown, unless it is no link (the page one is on, or plain
  /// text) or disabled. None while the breadcrumb is disabled.
  pub fn entry(&self) -> Option<usize> {
    if self.disabled {
      return None;
    }
    self
      .shown()
      .first()
      .copied()
      .filter(|&index| self.is_link(index) && !self.is_disabled(index))
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// Whether the step at `index` is the page one is on: the last, unless the breadcrumb ends with a link
  /// ([`Breadcrumb::current`], [`BreadcrumbStep::link`]).
  pub fn is_current(&self, index: usize) -> bool {
    index + 1 == self.steps.len() && self.current && self.steps[index].link != Some(true)
  }

  /// Whether the step at `index` is a link that a press follows: it is not the page one is on, and not told
  /// [`link(false)`](BreadcrumbStep::link).
  pub fn is_link(&self, index: usize) -> bool {
    index < self.steps.len() && !self.is_current(index) && self.steps[index].link != Some(false)
  }

  /// Whether the step at `index` is the last: the one a breadcrumb that overflows never drops, and the one that gives
  /// way where the row is short.
  fn is_last(&self, index: usize) -> bool {
    index + 1 == self.steps.len()
  }

  /// The steps laid out, in order, by their index: all of them but those over the most shown. In a breadcrumb that
  /// overflows, the steps its row dropped are among them: they are laid out at no size until there is room.
  pub fn shown(&self) -> Vec<usize> {
    (0..self.steps.len())
      .filter(|index| !self.counted.as_ref().is_some_and(|counted| counted.contains(index)))
      .collect()
  }

  /// Whether the breadcrumb hides the steps it has no room for: it overflows, and is mounted.
  pub fn overflows(&self) -> bool {
    self.fold.is_some()
  }

  /// What following the step at `index` does: its own handlers, then the
  /// breadcrumb's with its value. `None` for the page and a step that is no link, and while the step or the
  /// whole is disabled.
  pub fn follow(&self, index: usize) -> Option<impl Fn() + Send + Sync + Clone + 'static> {
    let step = self.steps.get(index)?;
    if self.is_disabled(index) || !self.is_link(index) {
      return None;
    }
    let (own, told, value) = (step.on_select.clone(), self.on_select.clone(), step.value.clone());
    Some(move || {
      for handler in &own {
        handler();
      }
      for handler in &told {
        handler(&value);
      }
    })
  }

  /// The root in the style of its part, with nothing in it yet: a row that
  /// is a navigation called by its label to what reads the tree.
  pub fn root(&self) -> Row {
    // The ring of a focused link stands out of its box: the row does not clip it.
    let mut root = Row::new()
      .align_items(Alignment::Center)
      .overflow_visible()
      .min_width(0.0)
      .part(&self.part(BreadcrumbPart::Root))
      .describe("a11y_role", "navigation")
      .describe("a11y_name", self.label.clone());
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root
  }

  /// The icon of the step at `index`, if it has one.
  fn icon(&self, index: usize) -> Option<Element> {
    let name = self.steps.get(index)?.icon.clone()?;
    Some(glyph(&self.step_part(index, BreadcrumbPart::Icon), name, false))
  }

  /// The step at `index`: a link that a press follows, the page one is on
  /// for the last, or plain text for a step that is no link, in the style of its part, with its icon and its text.
  /// The page's name, a link's cut at its most width, and the last step's, say themselves whole in
  /// a tooltip while they are cut.
  pub fn step(&self, index: usize) -> Element {
    let step = &self.steps[index];
    if !self.is_current(index) && !self.is_link(index) {
      return self.plain(index);
    }
    if self.is_current(index) {
      let part = self.step_part(index, BreadcrumbPart::Page);
      let mut page = Row::new()
        .align_items(Alignment::Center)
        .part(&part)
        .describe("a11y_name", step.label.clone())
        .describe("a11y_current", "page");
      if let Some(icon) = self.icon(index) {
        page = page.child(icon);
      }
      // The page gives way last, after every step that can go into the menu, and is cut then: what is before it
      // keeps its room. Its name is said whole in a tooltip while it is cut, as a cut link's is.
      let text = self.name_refs.get(index).cloned().unwrap_or_default();
      page = page
        .flex_shrink(1.0)
        .min_width(0.0)
        .shrink_order(self.steps.len() as i32)
        .shrink_limit(ShrinkLimit::Content)
        .child(
          Row::new().flex_shrink(1.0).min_width(0.0).child(
            part
              .text(&step.label)
              .nowrap()
              .text_overflow(TextOverflow::Elipsis)
              .ref_element(text.clone()),
          ),
        );
      if let Some(id) = &step.id {
        page = page.id(id.clone());
      }
      return super::Tooltip::new(&step.label).when_cut(&text).declared(page);
    }
    let part = self.step_part(index, BreadcrumbPart::Link);
    let pressed = tab_stop(pressable(&part, self.is_disabled(index)), self.tab_stop);
    // Ahead of following the step, which a press that asks for it aside does not reach.
    let aside = if self.is_disabled(index) {
      &[][..]
    } else {
      &step.on_open_aside[..]
    };
    let mut link = opens_aside(pressed, aside)
      .align_items(Alignment::Center)
      .describe("a11y_role", "link")
      .describe("a11y_name", step.label.clone());
    if let Some(follow) = self.follow(index) {
      link = link.on_click(move |_| follow());
    }
    if let Some(icon) = self.icon(index) {
      link = link.child(icon);
    }
    if let Some(id) = &step.id {
      link = link.id(id.clone());
    }
    if let Some(key) = &step.key {
      link = link.key(key.clone());
    }
    let last = self.is_last(index);
    if !self.cut_links && !last {
      return link.child(part.text(&step.label).nowrap()).into();
    }
    // Cut at the link's most width, which it keeps: the page gives way first. A link that ends the trail gives way
    // as the page would. The name is said whole while it is cut.
    let text = self.name_refs.get(index).cloned().unwrap_or_default();
    let link = if last {
      link
        .flex_shrink(1.0)
        .min_width(0.0)
        .shrink_order(self.steps.len() as i32)
        .shrink_limit(ShrinkLimit::Content)
    } else {
      link.flex_shrink(0.0)
    };
    let link = link.child(
      Row::new().flex_shrink(1.0).min_width(0.0).child(
        part
          .text(&step.label)
          .nowrap()
          .text_overflow(TextOverflow::Elipsis)
          .ref_element(text.clone()),
      ),
    );
    super::Tooltip::new(&step.label).when_cut(&text).declared(link)
  }

  /// A step that is no link and not the page one is on: its icon and its text in the part [`BreadcrumbPart::Text`],
  /// called by its label to what reads the tree, and nothing a press or Tab reaches.
  fn plain(&self, index: usize) -> Element {
    let step = &self.steps[index];
    let part = self.step_part(index, BreadcrumbPart::Text);
    let mut plain = Row::new()
      .align_items(Alignment::Center)
      .part(&part)
      .describe("a11y_name", step.label.clone());
    if let Some(icon) = self.icon(index) {
      plain = plain.child(icon);
    }
    plain = plain.child(part.text(&step.label).nowrap());
    if let Some(id) = &step.id {
      plain = plain.id(id.clone());
    }
    if let Some(key) = &step.key {
      plain = plain.key(key.clone());
    }
    plain.into()
  }

  /// What stands between two steps. A theme's renderer for
  /// [`BreadcrumbPart::Separator`] builds it in place of the default one.
  pub fn separator(&self) -> Element {
    if let Some(renderer) = self
      .theme()
      .part_renderer::<Breadcrumb, BreadcrumbPartRenderer>(BreadcrumbPart::Separator)
    {
      return renderer(self);
    }
    let part = self.part(BreadcrumbPart::Separator);
    let room = Row::new().align_items(Alignment::Center).part(&part);
    match &self.separator {
      BreadcrumbSeparator::Icon(name) => {
        room.child(glyph(&self.part(BreadcrumbPart::SeparatorIcon), name.clone(), false))
      }
      BreadcrumbSeparator::Text(text) => room.child(part.text(text).nowrap()),
    }
    .into()
  }

  /// The menu of the hidden steps: an item for each, which follows it.
  fn hidden_menu(&self, anchor: ElementRef, open: Signal<bool>) -> Menu {
    let mut menu = Menu::new(open, anchor)
      .placement(Placement::BottomStart)
      .rules(&self.recipe.lend("menu"));
    for index in self.hidden.clone().unwrap_or(0..0) {
      let step = &self.steps[index];
      // A step that is no link goes nowhere from the menu either.
      let mut item = MenuItem::new(&step.label).disabled(self.is_disabled(index) || !self.is_link(index));
      if let Some(icon) = &step.icon {
        item = item.icon(icon.clone());
      }
      if let Some(follow) = self.follow(index) {
        item = item.on_select(follow);
      }
      if let Some(id) = &step.id {
        item = item.id(id.clone());
      }
      menu = menu.item(item);
    }
    menu
  }

  /// The ellipsis, with the menu of the hidden steps it opens, if some are
  /// hidden: a button that turns the menu's open signal.
  pub fn ellipsis(&self) -> Option<Element> {
    if self.hidden.is_none() && self.fold.is_none() {
      return None;
    }
    let part = self.part(BreadcrumbPart::Ellipsis);
    let (anchor, open) = match &self.menu {
      Some((anchor, open)) => (anchor.clone(), Some(open.clone())),
      None => (ElementRef::new(), None),
    };
    let expanded = open.as_ref().is_some_and(|open| open.get());
    let mut button = tab_stop(pressable(&part, self.disabled), self.tab_stop)
      .justify(Justify::Center)
      .align_items(Alignment::Center)
      .ref_element(anchor.clone())
      .describe("a11y_name", self.more_label.clone())
      .describe("a11y_haspopup", "menu")
      .describe("a11y_expanded", if expanded { "true" } else { "false" })
      .child(glyph(
        &self.part(BreadcrumbPart::EllipsisIcon),
        Name::new(icons::MORE_HORIZONTAL),
        false,
      ));
    if let Some(id) = &self.id {
      button = button.id(format!("{id}-ellipsis"));
    }
    // The menu is an overlay: a child of a row, beside what it hangs from.
    let mut holder = Row::new().align_items(Alignment::Center).overflow_visible();
    match open {
      Some(open) => {
        if !self.disabled {
          let toggle = open.clone();
          button = button.on_click(move |_| toggle.update(|open| *open = !*open));
        }
        holder = holder.child(button).child(self.hidden_menu(anchor, open));
      }
      None => holder = holder.child(button),
    }
    Some(holder.into())
  }

  /// The room between two things of the root, in pixels: what a separator and its step are apart where they are
  /// held together.
  fn gap(&self) -> f32 {
    self
      .part(BreadcrumbPart::Root)
      .rest()
      .gap
      .map(|gap| gap.resolve(&self.theme().spacing(), 0.0))
      .unwrap_or(0.0)
  }

  /// A separator and what stands after it, held together: what a row drops as one.
  fn after_separator(&self, element: Element) -> Row {
    Row::new()
      .align_items(Alignment::Center)
      .overflow_visible()
      .spacing(self.gap())
      .child(self.separator())
      .child(element)
  }

  /// The step at `index` of a breadcrumb that overflows, with the separator before it: its row drops them
  /// together where it has no room, the steps nearest the first first, and its ref says so before the frame is
  /// painted.
  fn droppable(&self, fold: &Fold, index: usize) -> Element {
    let reference = fold.refs[index].clone();
    let (dropped, count) = (fold.dropped.clone(), self.steps.len());
    let open = self.menu.as_ref().map(|(_, open)| open.clone());
    reference.observe_rect_before_paint(move |rect| {
      let now = rect.width <= 0.0 && rect.height <= 0.0;
      if dropped.get_untracked().get(index).copied().unwrap_or(false) == now {
        return;
      }
      dropped.update(|dropped| {
        dropped.resize(count.max(dropped.len()), false);
        dropped[index] = now;
      });
      // With every step back in the row the ellipsis is gone, and its menu with it.
      if let Some(open) = &open
        && open.get_untracked()
        && !dropped.get_untracked().contains(&true)
      {
        open.set(false);
      }
    });
    self
      .after_separator(self.step(index))
      .flex_shrink(1.0)
      .shrink_limit(ShrinkLimit::Drop)
      .shrink_order(index as i32)
      .ref_element(reference)
      .into()
  }

  /// How the default renderer builds a breadcrumb: the steps shown, a
  /// separator between each two, and the ellipsis in place of the hidden ones.
  /// One that overflows holds each step after the first with its separator, for
  /// its row to drop them together, and its ellipsis is there while one is dropped.
  pub fn render(self) -> Element {
    let mut root = self.root();
    let shown = self.shown();
    for (place, index) in shown.iter().enumerate() {
      match &self.fold {
        Some(fold) if place > 0 && !self.is_last(*index) => root = root.child(self.droppable(fold, *index)),
        _ => {
          if place > 0 {
            root = root.child(self.separator());
          }
          root = root.child(self.step(*index));
        }
      }
      // The ellipsis stands after the first step, where the hidden ones would be.
      if place == 0
        && let Some(ellipsis) = self.ellipsis()
      {
        root = match self.counted {
          // Steps over the most shown are hidden whatever the room: so is the ellipsis there.
          Some(_) => root.child(self.separator()).child(ellipsis),
          None if self.fold.is_some() => root.child(self.after_separator(ellipsis).drop_marker(true)),
          None => root.child(self.separator()).child(ellipsis),
        };
      }
    }
    root.into()
  }
}

impl crate::ui::Styled for Breadcrumb {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for Breadcrumb {
  const NAME: &'static str = "breadcrumb";
  type Renderer = BreadcrumbRenderer;

  fn parts() -> crate::ui::Declared {
    BreadcrumbPart::declared()
  }

  fn states() -> crate::ui::Declared {
    BreadcrumbState::declared()
  }

  /// The design's breadcrumb (`pencil/lurq-design-system.pen`, Breadcrumb and
  /// Breadcrumb/Collapsed): steps 8 apart in the body's type, the links in
  /// the muted foreground and the page in the foreground at 500, a chevron of
  /// 14 in the muted foreground between each two. The ellipsis is a square of
  /// 24 filled with the highlight (the board's `$accent`), with the small
  /// radius and an icon of 16 in it. Under the pointer a link is in the
  /// foreground. Where the board is silent: a step that is no link is drawn as
  /// a link at rest, and is the same under the pointer.
  ///
  /// As a path control (Path Control) it is a bar 28 high on the muted fill
  /// with the medium radius, 4 in from its sides, its segments touching: each
  /// 22 high with the small radius, 6 in, an icon of 13 in the muted
  /// foreground 5 before a text of 12 in the foreground, a chevron of 12 in
  /// the muted foreground between each two, and the last on the background's
  /// fill in an edge of 1. The board draws no segment under the pointer and no
  /// ellipsis in the bar: a segment has the veils of a pressed row, and the
  /// ellipsis is a segment of its own, a square of 22 with an icon of 13.
  fn recipe() -> Recipe {
    use BreadcrumbPart::*;
    use BreadcrumbVariant::Path;
    use PaletteColor::{Background, Foreground, Highlight, Muted, MutedForeground, StateHover, StatePressed};
    let path = || Selector::new().variant(Path);
    Recipe::new()
      .base(Root, |s| s.gap(8.0).max_width(Length::Fill))
      .base(Link, |s| {
        fading(
          s.gap(4.0)
            .radius(RadiusSize::Sm)
            .foreground(MutedForeground)
            .typography(TypographyStyle::Body)
            .cursor(CursorIcon::Pointer),
        )
      })
      .hovered(Link, |s| s.foreground(Foreground))
      .focus_visible(Link, |s| s.focus_ring())
      .base(Page, |s| {
        s.gap(4.0)
          .foreground(Foreground)
          .typography(TypographyStyle::Body)
          .weight(FontWeight::Medium)
      })
      .base(Text, |s| {
        s.gap(4.0)
          .foreground(MutedForeground)
          .typography(TypographyStyle::Body)
      })
      .base(Icon, |s| s.square(IconSize::Sm))
      .base(Separator, |s| {
        s.foreground(MutedForeground).typography(TypographyStyle::Body)
      })
      .base(SeparatorIcon, |s| s.square(IconSize::Sm))
      .base(Ellipsis, |s| {
        fading(
          s.square(24.0)
            .radius(RadiusSize::Sm)
            .background(Highlight)
            .foreground(Foreground)
            .cursor(CursorIcon::Pointer),
        )
      })
      .hovered(Ellipsis, |s| s.veil(StateHover))
      .pressed(Ellipsis, |s| s.veil(StatePressed))
      .focus_visible(Ellipsis, |s| s.focus_ring())
      .base(EllipsisIcon, |s| s.square(IconSize::Md))
      // The path control: a bar of segments.
      .variant(Path, Root, |s| {
        s.height(28.0)
          .padding_x(4.0)
          .gap(0.0)
          .radius(RadiusSize::Md)
          .background(Muted)
      })
      .variant(Path, Link, |s| {
        s.height(22.0)
          .padding_x(6.0)
          .gap(5.0)
          .font_size(12.0)
          .foreground(Foreground)
      })
      .when(path().hovered(), Link, |s| s.veil(StateHover))
      .when(path().pressed(), Link, |s| s.veil(StatePressed))
      .variant(Path, Page, |s| {
        s.height(22.0)
          .padding_x(6.0)
          .gap(5.0)
          .radius(RadiusSize::Sm)
          .font_size(12.0)
          .weight(FontWeight::Normal)
          .background(Background)
          .border(edge(PaletteColor::Border))
      })
      .variant(Path, Text, |s| {
        s.height(22.0)
          .padding_x(6.0)
          .gap(5.0)
          .font_size(12.0)
          .foreground(Foreground)
      })
      .variant(Path, Icon, |s| s.square(13.0).foreground(MutedForeground))
      .variant(Path, Separator, |s| s.font_size(12.0))
      .variant(Path, SeparatorIcon, |s| s.square(12.0))
      .variant(Path, Ellipsis, |s| {
        s.square(22.0)
          .background(Color::new(0, 0, 0, 0))
          .foreground(MutedForeground)
      })
      .variant(Path, EllipsisIcon, |s| s.square(13.0))
      // Dimmed part by part, so that a disabled step is dimmed alone: what a disabled breadcrumb shows is the same.
      .state(BreadcrumbState::Disabled, Link, |s| {
        s.opacity(OpacityLevel::Disabled).cursor(CursorIcon::Default)
      })
      .state(BreadcrumbState::Disabled, Page, |s| s.opacity(OpacityLevel::Disabled))
      .state(BreadcrumbState::Disabled, Text, |s| s.opacity(OpacityLevel::Disabled))
      .state(BreadcrumbState::Disabled, Separator, |s| s.opacity(OpacityLevel::Disabled))
      .state(BreadcrumbState::Disabled, Ellipsis, |s| {
        s.opacity(OpacityLevel::Disabled).cursor(CursorIcon::Default)
      })
  }
}
