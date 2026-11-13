//! A toast: a short message in a corner of the window that leaves by
//! itself, and what shows toasts sent to it from anywhere.

use std::{
  sync::{
    Arc, Mutex, PoisonError,
    atomic::{AtomicU64, Ordering},
  },
  time::{Duration, Instant},
};

use super::{
  Button, Link,
  icon::Icon,
  icons, names,
  part_style::Length,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::{fading, glyph, pressable},
  theme::{Themed, current_breakpoint, current_theme},
};
use crate::{
  animation::{AnimatableProperty, Animation, Easing, Keyframes, KeyframesId, Transition},
  app::{
    component::{Component, DevtoolsInspectable},
    ctx::{Ctx, Modal, Root as Window, Timeout},
    events::{SwipeDirection, SwipeEvent},
    theme::{
      BorderSize, IconSize, MotionDuration, MotionEasing, PaletteColor, RadiusSize, ShadowStyle, Theme, TypographyStyle,
    },
  },
  components::{self, Column, Row, Stack},
  core::Signal,
  layout::{Alignment, StackAlignment, layout_kind::Justify, text_style::FontWeight},
  node::{CursorIcon, Element, HitTestBehavior, border::Border, dimension::Dimension, padding::Padding},
};

/// How long a toast is shown when it says nothing else.
pub const TOAST_DURATION: Duration = Duration::from_secs(5);

/// How many toasts are shown at once. The others wait, the oldest first, for
/// one of these to leave.
pub const TOASTS_SHOWN: usize = 3;

/// The parts of a toast a recipe styles, and of the corner the toasts are shown in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ToastPart {
  /// The panel: its width, fill, edge, corners and shadow.
  Root,
  /// What is in the panel above its timer, side by side.
  Content,
  /// The icon of its kind, at the start.
  Icon,
  /// The title and the description, one over the other.
  Text,
  /// What happened, in a few words.
  Title,
  /// More of it, under the title.
  Description,
  /// What holds the action: the button or the link it is given.
  Action,
  /// What closes it, at the end.
  Close,
  /// The rail along its bottom edge that says how long it has left.
  Timer,
  /// The part of the timer that is left.
  Remaining,
  /// The corner of the window the toasts are shown in: how far from its edges.
  Viewport,
  /// The toasts one over another while the pointer is on them.
  List,
  /// The edge of the toast behind the front one, while they are stacked.
  BehindNear,
  /// The edge of the one behind that.
  BehindFar,
}

names!(ToastPart {
  Root => "root",
  Content => "content",
  Icon => "icon",
  Text => "text",
  Title => "title",
  Description => "description",
  Action => "action",
  Close => "close",
  Timer => "timer",
  Remaining => "remaining",
  Viewport => "viewport",
  List => "list",
  BehindNear => "behind-near",
  BehindFar => "behind-far",
});

/// What a toast says it is: the color of its icon, and which icon it has.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ToastVariant {
  /// A message with no icon of its own: "Link copied".
  #[default]
  Neutral,
  /// Something was done: "Settings saved".
  Success,
  /// Something to know: "A new version is ready".
  Info,
  /// Something was done, with a catch: "Saved, but not synced".
  Warning,
  /// Something could not be done: "Could not connect".
  Error,
}

names!(ToastVariant {
  Neutral => "neutral",
  Success => "success",
  Info => "info",
  Warning => "warning",
  Error => "error",
});

impl ToastVariant {
  /// The icon of the variant in the theme's set: none for a neutral toast.
  pub fn icon(self) -> Option<&'static str> {
    match self {
      Self::Neutral => None,
      Self::Success => Some(icons::CHECK_CIRCLE),
      Self::Info => Some(icons::INFO),
      Self::Warning => Some(icons::WARNING),
      Self::Error => Some(icons::CLOSE_CIRCLE),
    }
  }
}

/// The states of a toast a recipe styles, besides what the pointer and the
/// keyboard do to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ToastState {
  /// It has an action: its content is centered on the button, and lower.
  Action,
}

names!(ToastState {
  Action => "action",
});

/// The corner, or the middle of an edge, of the window toasts are shown in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ToastPosition {
  TopLeft,
  Top,
  TopRight,
  BottomLeft,
  Bottom,
  #[default]
  BottomRight,
}

impl ToastPosition {
  /// Every one of them, in order.
  pub const ALL: [ToastPosition; 6] = [
    Self::TopLeft,
    Self::Top,
    Self::TopRight,
    Self::BottomLeft,
    Self::Bottom,
    Self::BottomRight,
  ];

  fn at_bottom(self) -> bool {
    matches!(self, Self::BottomLeft | Self::Bottom | Self::BottomRight)
  }

  fn across(self) -> Alignment {
    match self {
      Self::TopLeft | Self::BottomLeft => Alignment::Start,
      Self::Top | Self::Bottom => Alignment::Center,
      Self::TopRight | Self::BottomRight => Alignment::End,
    }
  }
}

/// What a toast's action is: a button or a link.
#[derive(Clone)]
pub enum ToastAction {
  Button(Button),
  Link(Link),
}

impl ToastAction {
  /// The action with `press` called on a press of it too.
  fn also(self, press: impl Fn() + Send + Sync + 'static) -> Self {
    match self {
      Self::Button(button) => Self::Button(button.on_click(move |_| press())),
      Self::Link(link) => Self::Link(link.on_click(move |_| press())),
    }
  }
}

impl From<Button> for ToastAction {
  fn from(button: Button) -> Self {
    Self::Button(button)
  }
}

impl From<Link> for ToastAction {
  fn from(link: Link) -> Self {
    Self::Link(link)
  }
}

impl From<ToastAction> for Element {
  fn from(action: ToastAction) -> Self {
    match action {
      ToastAction::Button(button) => button.into(),
      ToastAction::Link(link) => link.into(),
    }
  }
}

/// How much of its time a toast has left, as its timer shows it.
#[derive(Clone, Copy, Debug, PartialEq)]
enum TimeLeft {
  /// All of it: nothing counts it down.
  All,
  /// A part of it, from 0 to 1, standing still.
  Part(f32),
  /// What is left runs out over this.
  RunningOut(Duration),
}

/// What builds a toast's nodes: a theme's in place of the default one.
pub type ToastRenderer = Arc<dyn Fn(ToastView) -> Element + Send + Sync>;

/// A toast: a short message that something happened, in a corner of the
/// window over everything, that leaves by itself after a while. Settings
/// saved, a file moved to the trash, a connection that failed.
///
/// ```
/// use lurq::ui::{Button, ButtonSize, ButtonVariant, Toast, ToastVariant};
///
/// let saved = Toast::new("Settings saved")
///   .description("Your theme applies to every open window.")
///   .variant(ToastVariant::Success);
/// // With an action, and a rail that says how long it has left.
/// let trashed = Toast::new("File moved to trash")
///   .description("theme-dark.json")
///   .variant(ToastVariant::Success)
///   .action(Button::new("Undo").variant(ButtonVariant::Outline).size(ButtonSize::Sm))
///   .timer(true);
/// ```
///
/// A toast is a panel like any other component, and can be put anywhere. To
/// show one from anywhere and have it leave by itself, send it to a
/// [`Toasts`]: `toasts.show(saved)`.
#[derive(Clone)]
pub struct Toast {
  title: Arc<str>,
  description: Option<Arc<str>>,
  variant: ToastVariant,
  icon: Option<Name>,
  icon_hidden: bool,
  action: Option<ToastAction>,
  closable: bool,
  timer: bool,
  duration: Duration,
  persistent: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  on_close: Vec<Arc<dyn Fn() + Send + Sync>>,
  /// How much of its time it has left, as what shows it counts it: what its timer shows.
  left: TimeLeft,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Toast {
  /// A toast that says `title`.
  pub fn new(title: &str) -> Self {
    Self {
      title: Arc::from(title),
      description: None,
      variant: ToastVariant::Neutral,
      icon: None,
      icon_hidden: false,
      action: None,
      closable: true,
      timer: false,
      duration: TOAST_DURATION,
      persistent: false,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      on_close: Vec::new(),
      left: TimeLeft::All,
      id: None,
      key: None,
    }
  }

  /// More of what happened, under the title.
  pub fn description(mut self, description: &str) -> Self {
    self.description = Some(Arc::from(description));
    self
  }

  pub fn variant(mut self, variant: ToastVariant) -> Self {
    self.variant = variant;
    self
  }

  /// An icon of the theme's set in place of the variant's: what a neutral toast has none of.
  pub fn icon(mut self, name: impl Into<Name>) -> Self {
    self.icon = Some(name.into());
    self
  }

  /// Whether the toast has no icon, whatever its variant.
  pub fn icon_hidden(mut self, hidden: bool) -> Self {
    self.icon_hidden = hidden;
    self
  }

  /// What can be done about it, before the close control: a [`Button`] or a [`Link`]. A toast shown by a
  /// [`Toasts`] leaves when it is pressed.
  pub fn action(mut self, action: impl Into<ToastAction>) -> Self {
    self.action = Some(action.into());
    self
  }

  /// Whether the toast has its close control. It has by default.
  pub fn closable(mut self, closable: bool) -> Self {
    self.closable = closable;
    self
  }

  /// Whether the toast has a rail along its bottom edge that says how long it has left.
  pub fn timer(mut self, timer: bool) -> Self {
    self.timer = timer;
    self
  }

  /// How long a [`Toasts`] shows it: [`TOAST_DURATION`] if it says nothing.
  pub fn duration(mut self, duration: Duration) -> Self {
    self.duration = duration;
    self
  }

  /// Whether it stays until it is closed, or its action is pressed.
  pub fn persistent(mut self, persistent: bool) -> Self {
    self.persistent = persistent;
    self
  }

  /// Puts the toast in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Called on a press of its close control. A toast shown by a [`Toasts`] leaves then too.
  pub fn on_close(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_close.push(Arc::new(handler));
    self
  }

  /// Styles one part of this toast over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this toast.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the close control as it is under the pointer, held down or
  /// focused from the keyboard, with nothing pointing at it: for a gallery
  /// or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the panel.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// How long a [`Toasts`] shows it: `None` while it stays until it is closed.
  fn lasting(&self) -> Option<Duration> {
    (!self.persistent).then_some(self.duration)
  }

  /// Builds the toast in `theme`. Converting a toast into an element does
  /// this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case {
      variant: Some(self.variant.into()),
      size: None,
      density: None,
      breakpoint: current_breakpoint(),
      states: self.states.clone(),
    }
    .state(ToastState::Action, self.action.is_some());
    let icon = match (self.icon_hidden, self.icon) {
      (true, _) => None,
      (false, Some(icon)) => Some(icon),
      (false, None) => self.variant.icon().map(Name::new),
    };
    let view = ToastView {
      title: self.title,
      description: self.description,
      variant: self.variant,
      icon,
      action: self.action,
      closable: self.closable,
      timer: self.timer,
      recipe: theme.resolve::<Toast>(&case, Some(&self.rules)),
      forced: self.forced,
      on_close: self.on_close,
      left: self.left,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Toast>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<Toast> for Element {
  fn from(toast: Toast) -> Self {
    toast.build(&current_theme())
  }
}

/// A toast as a renderer sees it: what it was given, the style of its parts,
/// and its behavior ready to be dressed.
pub struct ToastView {
  pub title: Arc<str>,
  pub description: Option<Arc<str>>,
  pub variant: ToastVariant,
  /// The icon it shows, by its name in the theme's set: the variant's, or the one it was given.
  pub icon: Option<Name>,
  pub action: Option<ToastAction>,
  pub closable: bool,
  pub timer: bool,
  /// The style of each part of this toast.
  pub recipe: ResolvedRecipe,
  forced: Option<Interaction>,
  on_close: Vec<Arc<dyn Fn() + Send + Sync>>,
  left: TimeLeft,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl ToastView {
  /// The style of a part, as this toast is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The icon in the style of its part, if the toast has one.
  pub fn icon(&self) -> Option<Element> {
    self.icon.map(|icon| glyph(&self.part(ToastPart::Icon), icon, false))
  }

  /// The title over the description.
  pub fn text(&self) -> Column {
    let title = self.part(ToastPart::Title);
    let mut text = Column::new()
      .flex(1.0)
      .part(&self.part(ToastPart::Text))
      .child(Row::new().part(&title).child(title.text(&self.title)));
    if let Some(description) = &self.description {
      let part = self.part(ToastPart::Description);
      text = text.child(Row::new().part(&part).child(part.text(description)));
    }
    text
  }

  /// The action it was given, in the style of its part.
  pub fn action(&self) -> Option<Row> {
    let action = self.action.clone()?;
    Some(Row::new().part(&self.part(ToastPart::Action)).child(action))
  }

  /// What closes it: a press of it tells every handler of `on_close`. It is
  /// pressed from the keyboard too, and called "Close" to what reads the tree.
  pub fn close(&self) -> components::Button {
    let part = self.part(ToastPart::Close);
    let side = part.rest().width.unwrap_or(Length::Icon(IconSize::Md));
    let mut close = pressable(&part, false)
      .describe("a11y_role", "button")
      .describe("a11y_name", "Close")
      .child(Icon::new(icons::CLOSE).size(side).build(self.theme()));
    if !self.on_close.is_empty() {
      let handlers = self.on_close.clone();
      close = close.on_click(move |_| {
        for handler in &handlers {
          handler();
        }
      });
    }
    close
  }

  /// How wide the part of the timer that is left is when all of it is: the
  /// panel's width. `None` where the recipe gives the panel no width in pixels.
  pub fn timer_width(&self) -> Option<f32> {
    match self.part(ToastPart::Root).rest().width?.resolve(self.theme())? {
      Dimension::Px(px) => Some(px),
      _ => None,
    }
  }

  /// The timer: its rail, and on it what is left. While the toast's time is
  /// counted down, what is left runs out; while it stands still, it stays
  /// where it is; a toast nothing counts down has all of it.
  pub fn timer(&self) -> Row {
    let mut remaining = Row::new().part(&self.part(ToastPart::Remaining));
    remaining = match (self.timer_width(), self.left) {
      (Some(full), TimeLeft::All) => remaining.width(Dimension::Px(full)),
      (Some(full), TimeLeft::Part(part)) => remaining.width(Dimension::Px(full * part.clamp(0.0, 1.0))),
      // It runs out from where it is to nothing, a transition from one width to the other.
      (Some(_), TimeLeft::RunningOut(left)) => remaining
        .width(Dimension::Px(0.0))
        .transition(Transition::width().duration(left).linear()),
      (None, _) => remaining.width(Dimension::Pct(100.0)),
    };
    Row::new().part(&self.part(ToastPart::Timer)).child(remaining)
  }

  /// How the default renderer builds a toast: its icon, its text, its action
  /// and its close control side by side, and its timer under them. It is a
  /// status to what reads the tree, called by its title.
  pub fn render(self) -> Element {
    let mut content = Row::new()
      .align_items(if self.action.is_some() {
        Alignment::Center
      } else {
        Alignment::Start
      })
      .part(&self.part(ToastPart::Content));
    if let Some(icon) = self.icon() {
      content = content.child(icon);
    }
    content = content.child(self.text());
    if let Some(action) = self.action() {
      content = content.child(action);
    }
    if self.closable {
      content = content.child(self.close());
    }
    let mut root = Column::new()
      .part(&self.part(ToastPart::Root))
      .describe("a11y_role", "status")
      .describe("a11y_name", self.title.clone())
      .child(content);
    if self.timer {
      root = root.child(self.timer());
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

impl Themed for Toast {
  const NAME: &'static str = "toast";
  type Renderer = ToastRenderer;

  /// The design's toast (`pencil/lurq-design-system.pen`, Toast and
  /// Toast/Rich): a panel 360 wide in the popover's colors, with an edge,
  /// large corners and the large shadow; 16 inside it the icon of 16 in the
  /// variant's color, 12 from the title (14, medium) over the description
  /// (13, muted), and the close control of 16 in the muted color. With an
  /// action it is 14 high inside, centered on the action, and its title is 2
  /// over its description. The timer is a rail of 2 in the secondary color,
  /// with what is left in the primary color. Toasts are 16 from the window's
  /// edges and 10 apart; stacked, the two behind show an edge of 8 each,
  /// 24 and 48 narrower, three quarters and half opaque.
  fn recipe() -> Recipe {
    use PaletteColor::{
      Destructive, Info, MutedForeground, Popover, PopoverForeground, Primary, Secondary, Success, Warning,
    };
    use ToastPart as P;
    let edge = || Border::inside(BorderSize::Sm, PaletteColor::Border);
    let behind = |s: super::PartStyle, width: f32, opacity: f32| {
      s.width(width)
        .height(8.0)
        .background(Popover)
        .border(edge())
        .radius(RadiusSize::Lg)
        .opacity(opacity)
    };
    Recipe::new()
      .base(P::Root, |s| {
        s.width(360.0)
          .background(Popover)
          .foreground(PopoverForeground)
          .border(edge())
          .radius(RadiusSize::Lg)
          .shadow(ShadowStyle::Lg)
      })
      .base(P::Content, |s| {
        s.width(Length::Fill).padding_x(16.0).padding_y(16.0).gap(12.0)
      })
      .base(P::Icon, |s| {
        s.square(Length::Icon(IconSize::Md)).foreground(PopoverForeground)
      })
      .base(P::Text, |s| s.gap(4.0))
      .base(P::Title, |s| {
        s.typography(TypographyStyle::Body)
          .weight(FontWeight::Medium)
          .foreground(PopoverForeground)
      })
      .base(P::Description, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(13.0)
          .line_height(1.5)
          .foreground(MutedForeground)
      })
      .base(P::Close, |s| {
        fading(
          s.square(Length::Icon(IconSize::Md))
            .radius(RadiusSize::Sm)
            .foreground(MutedForeground)
            .cursor(CursorIcon::Pointer),
        )
      })
      .hovered(P::Close, |s| s.foreground(PopoverForeground))
      .focus_visible(P::Close, |s| s.focus_ring())
      .base(P::Timer, |s| s.width(Length::Fill).height(2.0).background(Secondary))
      .base(P::Remaining, |s| s.height(2.0).background(Primary))
      .base(P::Viewport, |s| s.padding_x(16.0).padding_y(16.0))
      .base(P::List, |s| s.gap(10.0))
      .base(P::BehindNear, |s| behind(s, 336.0, 0.75))
      .base(P::BehindFar, |s| behind(s, 312.0, 0.5))
      .state(ToastState::Action, P::Content, |s| s.padding_y(14.0))
      .state(ToastState::Action, P::Text, |s| s.gap(2.0))
      .variant(ToastVariant::Success, P::Icon, |s| s.foreground(Success))
      .variant(ToastVariant::Info, P::Icon, |s| s.foreground(Info))
      .variant(ToastVariant::Warning, P::Icon, |s| s.foreground(Warning))
      .variant(ToastVariant::Error, P::Icon, |s| s.foreground(Destructive))
  }
}

/// Which toast of a [`Toasts`] it is, to take it away.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ToastId(u64);

/// A toast as a [`Toasts`] holds it. A toast is no value to share between threads, as an element is not: what
/// is in the signal is a toast behind a lock.
#[derive(Clone)]
struct Shown {
  id: ToastId,
  toast: Arc<Mutex<Toast>>,
}

impl Shown {
  fn toast(&self) -> Toast {
    self.toast.lock().unwrap_or_else(PoisonError::into_inner).clone()
  }
}

impl DevtoolsInspectable for Shown {}

/// The toasts an app shows: a handle it makes once, keeps in its root
/// component and clones into whatever shows a toast. Its host, which the
/// root renders once, draws them in a corner of the window over everything.
///
/// ```no_run
/// use lurq::{
///   app::{component::Component, ctx::Ctx},
///   components::{Button, Column},
///   node::Element,
///   ui::{Toast, ToastVariant, Toasts},
/// };
///
/// struct App {
///   toasts: Toasts,
/// }
///
/// impl Component for App {
///   type Props = ();
///
///   fn create(_ctx: &mut Ctx) -> Self {
///     Self { toasts: Toasts::new() }
///   }
///
///   fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
///     let toasts = self.toasts.clone();
///     Column::new()
///       .child(Button::new("Save").on_click(move |_| {
///         toasts.show(Toast::new("Settings saved").variant(ToastVariant::Success));
///       }))
///       // Last, so that its layer is over every other.
///       .child(self.toasts.host(ctx))
///   }
/// }
/// ```
///
/// A toast leaves after its duration, on a press of its close control or of
/// its action, and by [`Toasts::dismiss`]. While the pointer is on the toasts
/// their time stands still, and they are laid out one over another; when it
/// leaves, their time runs on from where it stood.
#[derive(Clone)]
pub struct Toasts {
  shown: Signal<Vec<Shown>>,
  /// Whether the pointer is on the toasts: their time stands still, and they are not stacked.
  paused: Signal<bool>,
  next: Arc<AtomicU64>,
}

impl Toasts {
  /// No toasts yet.
  pub fn new() -> Self {
    Self {
      shown: Signal::new(Vec::new()),
      paused: Signal::new(false),
      next: Arc::new(AtomicU64::new(1)),
    }
  }

  /// Shows `toast`, in front of those already shown.
  pub fn show(&self, toast: Toast) -> ToastId {
    let id = ToastId(self.next.fetch_add(1, Ordering::Relaxed));
    self.shown.update(|shown| {
      shown.push(Shown {
        id,
        toast: Arc::new(Mutex::new(toast)),
      })
    });
    id
  }

  /// Takes the toast `id` away, if it is still shown.
  pub fn dismiss(&self, id: ToastId) {
    if !self
      .shown
      .with_untracked(|shown| shown.iter().any(|held| held.id == id))
    {
      return;
    }
    self.shown.update(|shown| shown.retain(|held| held.id != id));
    // With nothing left under it, the pointer is on no toast: the next one's time runs.
    if self.shown.with_untracked(Vec::is_empty) && self.paused.get_untracked() {
      self.paused.set(false);
    }
  }

  /// Takes every toast away.
  pub fn clear(&self) {
    if self.shown.with_untracked(Vec::is_empty) {
      return;
    }
    self.shown.set(Vec::new());
    if self.paused.get_untracked() {
      self.paused.set(false);
    }
  }

  /// The toasts shown and waiting, the oldest first.
  pub fn ids(&self) -> Vec<ToastId> {
    self.shown.with(|shown| shown.iter().map(|held| held.id).collect())
  }

  pub fn len(&self) -> usize {
    self.shown.with(Vec::len)
  }

  pub fn is_empty(&self) -> bool {
    self.shown.with(Vec::is_empty)
  }

  /// What draws the toasts at the bottom right of the window. The app's root
  /// renders it once, as its last child, so that its layer is over every other.
  pub fn host(&self, ctx: &mut Ctx) -> Element {
    self.host_at(ctx, ToastPosition::default())
  }

  /// What draws the toasts at `position` in the window.
  pub fn host_at(&self, ctx: &mut Ctx, position: ToastPosition) -> Element {
    ctx.mount::<ToastHost>(ToastHostProps {
      toasts: self.clone(),
      position,
    })
  }

  fn same(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.next, &other.next)
  }
}

impl Default for Toasts {
  fn default() -> Self {
    Self::new()
  }
}

/// A toast coming in from under where it stands, and from over it; leaving
/// downward, and upward. The animation engine registers these with the
/// spinner's turn, so that every app has them.
const ENTER_FROM_BELOW: KeyframesId = KeyframesId::new(0xFF11);
const ENTER_FROM_ABOVE: KeyframesId = KeyframesId::new(0xFF12);
const LEAVE_DOWNWARD: KeyframesId = KeyframesId::new(0xFF13);
const LEAVE_UPWARD: KeyframesId = KeyframesId::new(0xFF14);

/// How far a toast moves as it comes in and as it leaves: the distance of the toasts from the window's edge.
const TRAVEL: f32 = 16.0;

/// The keyframes of a toast coming in and leaving: it fades in as it moves
/// to its place from the window's edge, and out as it moves back to it.
pub(crate) fn keyframes() -> [Keyframes; 4] {
  let slide = |id: KeyframesId, from: (f32, f32), to: (f32, f32)| {
    Keyframes::new(id)
      .frame(0.0, |frame| {
        frame.set(AnimatableProperty::OffsetY, from.0);
        frame.set(AnimatableProperty::Opacity, from.1);
      })
      .frame(1.0, |frame| {
        frame.set(AnimatableProperty::OffsetY, to.0);
        frame.set(AnimatableProperty::Opacity, to.1);
      })
  };
  [
    slide(ENTER_FROM_BELOW, (TRAVEL, 0.0), (0.0, 1.0)),
    slide(ENTER_FROM_ABOVE, (-TRAVEL, 0.0), (0.0, 1.0)),
    slide(LEAVE_DOWNWARD, (0.0, 1.0), (TRAVEL, 0.0)),
    slide(LEAVE_UPWARD, (0.0, 1.0), (-TRAVEL, 0.0)),
  ]
}

#[derive(Clone)]
struct ToastHostProps {
  toasts: Toasts,
  position: ToastPosition,
}

impl PartialEq for ToastHostProps {
  fn eq(&self, other: &Self) -> bool {
    self.position == other.position && self.toasts.same(&other.toasts)
  }
}

impl DevtoolsInspectable for ToastHostProps {}

/// What draws the toasts of a [`Toasts`] in a corner of the window, in a
/// layer over everything that takes no press beside them and keeps no Tab
/// from the page. A component, since each toast keeps a timer of its own.
struct ToastHost;

impl Component for ToastHost {
  type Props = ToastHostProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let ToastHostProps { toasts, position } = ctx.props::<ToastHostProps>().clone();
    // The newest first: those shown, and how long each lasts.
    let front: Vec<(ToastId, Option<Duration>)> = toasts.shown.with(|shown| {
      shown
        .iter()
        .rev()
        .take(TOASTS_SHOWN)
        .map(|held| (held.id, held.toast().lasting()))
        .collect()
    });
    if front.is_empty() {
      // Closed, the layer stays while the last toast leaves.
      return Element::from(Modal::new(Element::new()).open_when(false));
    }
    let paused = toasts.paused.get();
    let theme = ctx.theme().clone();
    let recipe = theme.resolve::<Toast>(
      &Case {
        breakpoint: current_breakpoint(),
        ..Case::default()
      },
      None,
    );
    let bottom = position.at_bottom();
    // A toast taken away leaves toward the window's edge: a keyframe animation, which reduced motion goes without.
    let leave = (!theme.reduced_motion()).then(|| {
      Animation::new(if bottom { LEAVE_DOWNWARD } else { LEAVE_UPWARD })
        .duration(theme.duration_value(MotionDuration::Base))
        .easing(theme.easings().try_get(MotionEasing::Exit).unwrap_or(Easing::Linear))
    });
    let mount = |ctx: &mut Ctx, (id, duration): (ToastId, Option<Duration>), shown: bool| {
      ctx.mount_keyed::<ToastItem>(
        &id.0.to_string(),
        ToastItemProps {
          toasts: toasts.clone(),
          id,
          paused,
          duration,
          shown,
          bottom,
        },
      )
    };
    // A toast in its place, by its id: the list keeps it there while it leaves.
    let place = |toast: Element, id: ToastId| {
      let place = Row::new().overflow_visible().key(id.0.to_string());
      match &leave {
        Some(leave) => place.exit_animation(leave.clone()),
        None => place,
      }
      .child(toast)
    };

    // One list in either form, so that a toast that goes from it leaves by its animation: one taken away, and
    // those behind the front one when the toasts are stacked again.
    let mut list = Column::new()
      .overflow_visible()
      .presence()
      .align_items(position.across())
      .part(&recipe.part(ToastPart::List));
    // The toasts behind the front one, while they are stacked: mounted, so that their time runs, and drawing
    // nothing.
    let mut waiting = Row::new().size(Dimension::Px(0.0), Dimension::Px(0.0));
    if paused || front.len() == 1 {
      // One over another, the newest nearest the window's edge.
      let order: Vec<_> = if bottom {
        front.into_iter().rev().collect()
      } else {
        front
      };
      for toast in order {
        let item = mount(ctx, toast, true);
        list = list.child(place(item, toast.0));
      }
    } else {
      // Stacked: the newest in front, and an edge of each of the others behind it, on the side away from the
      // window's edge.
      let near = recipe.part(ToastPart::BehindNear);
      let peek = match near.rest().height.and_then(|height| height.resolve(recipe.theme())) {
        Some(Dimension::Px(px)) => px,
        _ => 0.0,
      };
      let room = |before: f32| {
        let room = Row::new().overflow_visible();
        if bottom {
          room.padding_top(before)
        } else {
          room.padding_bottom(before)
        }
      };
      // An edge is drawn taller than it shows, its far side under the toast in front of it.
      let edge = |part: ToastPart, before: f32| {
        room(before).child(Row::new().part(&recipe.part(part)).height(Dimension::Px(peek * 3.0)))
      };
      let behind = front.len() - 1;
      let mut stack = Stack::new().overflow_visible().stack_align(if bottom {
        StackAlignment::TopCenter
      } else {
        StackAlignment::BottomCenter
      });
      if behind >= 2 {
        stack = stack.child(edge(ToastPart::BehindFar, 0.0));
      }
      stack = stack.child(edge(ToastPart::BehindNear, peek * (behind - 1) as f32));
      let item = mount(ctx, front[0], true);
      stack = stack.child(room(peek * behind as f32).child(item));
      list = list.child(place(stack.into(), front[0].0));
      for toast in &front[1..] {
        waiting = waiting.child(mount(ctx, *toast, false));
      }
    }

    // With the pointer on them their time stands still, and they are laid out one over another.
    let (enter, leave) = (toasts.paused.clone(), toasts.paused.clone());
    let shown = Row::new()
      .overflow_visible()
      .on_mouse_enter(move || enter.set(true))
      .on_mouse_leave(move || leave.set(false))
      .child(list);
    // Clear of what the window's edges hold: a phone's notch, its home indicator, its keyboard.
    let safe = ctx.safe_area();
    let viewport = Column::new()
      .size(Dimension::Pct(100.0), Dimension::Pct(100.0))
      .hit_test(HitTestBehavior::ContentOnly)
      .overflow_visible()
      .padding(
        Padding::new()
          .top(safe.top)
          .right(safe.right)
          .bottom(safe.bottom + safe.keyboard)
          .left(safe.left),
      )
      .child(
        Column::new()
          .size(Dimension::Pct(100.0), Dimension::Pct(100.0))
          .hit_test(HitTestBehavior::ContentOnly)
          .overflow_visible()
          .justify(if bottom { Justify::End } else { Justify::Start })
          .align_items(position.across())
          .part(&recipe.part(ToastPart::Viewport))
          .child(shown)
          .child(waiting),
      );
    // A layer like the window chrome's: over the page and its dialogs, taking a press only on what is in it,
    // and holding Tab to none of it, as a dialog's layer does.
    Element::from(
      Modal::new(viewport)
        .open_when(true)
        .target(Window)
        .dismiss_on_escape(false)
        .window_chrome_layer(),
    )
  }
}

#[derive(Clone)]
struct ToastItemProps {
  toasts: Toasts,
  id: ToastId,
  /// Whether the pointer is on the toasts: its time stands still.
  paused: bool,
  /// How long it lasts: `None` while it stays until it is closed.
  duration: Option<Duration>,
  /// Whether it is drawn: one behind the front one, while they are stacked, is not, and its time runs.
  shown: bool,
  /// Whether the toasts are at the bottom of the window: a toast comes in from under its place.
  bottom: bool,
}

impl PartialEq for ToastItemProps {
  fn eq(&self, other: &Self) -> bool {
    self.id == other.id
      && self.paused == other.paused
      && self.duration == other.duration
      && self.shown == other.shown
      && self.bottom == other.bottom
      && self.toasts.same(&other.toasts)
  }
}

impl DevtoolsInspectable for ToastItemProps {}

/// What a toast's timer has counted.
struct Clock {
  /// How long it had left when its time last stood still.
  left: Duration,
  /// Since when its time runs: `None` while it stands still.
  since: Option<Instant>,
  /// Whether it was drawn by the last render: its timer runs out from the next one on.
  drawn: bool,
  /// When it was first drawn: it comes in from then on.
  entered: Option<Instant>,
}

/// One toast a host shows, with its timer: it takes the toast away when the
/// toast's time is up, and keeps what is left of it while its time stands still.
struct ToastItem {
  timeout: Option<Timeout>,
  clock: Mutex<Clock>,
  /// Renders it once more right after it is first drawn: a transition runs from one width to another, so the
  /// timer is drawn where it is first and runs out from there.
  again: Timeout,
  renders: Signal<u64>,
}

impl Component for ToastItem {
  type Props = ToastItemProps;

  fn create(ctx: &mut Ctx) -> Self {
    let props = ctx.props::<ToastItemProps>().clone();
    let timeout = props.duration.map(|duration| {
      let (toasts, id) = (props.toasts.clone(), props.id);
      ctx.create_timeout(duration, move || toasts.dismiss(id))
    });
    let renders = ctx.signal(0_u64);
    let again = ctx.create_timeout(Duration::ZERO, {
      let renders = renders.clone();
      move || renders.update(|count| *count += 1)
    });
    Self {
      timeout,
      clock: Mutex::new(Clock {
        left: props.duration.unwrap_or_default(),
        since: None,
        drawn: false,
        entered: None,
      }),
      again,
      renders,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let _ = self.renders.get();
    let props = ctx.props::<ToastItemProps>().clone();
    let now = Instant::now();
    let mut clock = self.clock.lock().unwrap_or_else(PoisonError::into_inner);
    // Under the pointer its time stands still with what is left of it; let go, it runs on from there.
    if let Some(timeout) = &self.timeout {
      match (props.paused, clock.since) {
        (true, Some(since)) => {
          clock.left = clock.left.saturating_sub(now.saturating_duration_since(since));
          clock.since = None;
          timeout.cancel();
        }
        (false, None) => {
          clock.since = Some(now);
          timeout.set_duration(clock.left);
          timeout.start();
        }
        _ => {}
      }
    }
    if !props.shown {
      clock.drawn = false;
      return Element::new();
    }
    let toast = props
      .toasts
      .shown
      .with(|shown| shown.iter().find(|held| held.id == props.id).map(Shown::toast));
    let Some(mut toast) = toast else {
      return Element::new();
    };

    let left = match clock.since {
      Some(since) => clock.left.saturating_sub(now.saturating_duration_since(since)),
      None => clock.left,
    };
    toast.left = match props.duration {
      None => TimeLeft::All,
      Some(_) if clock.drawn && clock.since.is_some() => TimeLeft::RunningOut(left),
      Some(duration) => TimeLeft::Part(left.as_secs_f32() / duration.as_secs_f32().max(f32::EPSILON)),
    };
    if !clock.drawn {
      clock.drawn = true;
      self.again.restart();
    }
    // It comes in from the window's edge once, when it is first drawn: a keyframe animation, which reduced motion
    // goes without.
    let theme = ctx.theme().clone();
    let entered = *clock.entered.get_or_insert(now);
    let coming = theme.duration_value(MotionDuration::Slow);
    drop(clock);

    let dismiss = {
      let (toasts, id) = (props.toasts.clone(), props.id);
      move || toasts.dismiss(id)
    };
    toast.action = toast.action.map(|action| action.also(dismiss.clone()));
    // A finger throws it off: to a side, or toward the window's edge.
    let thrown = {
      let (dismiss, bottom) = (dismiss.clone(), props.bottom);
      move |swipe: SwipeEvent| {
        let away = match swipe.direction {
          SwipeDirection::Left | SwipeDirection::Right => true,
          SwipeDirection::Down => bottom,
          SwipeDirection::Up => !bottom,
        };
        if away {
          dismiss();
        }
      }
    };
    let mut item = Row::new().overflow_visible().on_swipe(thrown);
    if !theme.reduced_motion() && now.saturating_duration_since(entered) < coming {
      item = item.animation(
        Animation::new(if props.bottom {
          ENTER_FROM_BELOW
        } else {
          ENTER_FROM_ABOVE
        })
        .duration(coming)
        .easing(theme.easings().try_get(MotionEasing::Enter).unwrap_or(Easing::Linear)),
      );
    }
    Element::from(item.child(toast.on_close(dismiss)))
  }
}
