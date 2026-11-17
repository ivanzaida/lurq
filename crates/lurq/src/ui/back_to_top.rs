//! A back to top button: it shows once a page is scrolled, and takes it to its top.
//!
//! It is a mounted component: it keeps whether the page is scrolled far
//! enough for it to show, which it reads after each layout, and a builder
//! reads nothing. [`BackToTop`] says what it is, and [`BackToTop::mount`]
//! puts it where it goes.

use std::sync::Arc;

use parking_lot::Mutex;

use super::{
  button::{Button, ButtonPart, ButtonShape, ButtonSize, ButtonVariant},
  icons, names,
  part_style::PartAnimation,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::{FADE_IN, FADE_OUT},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    component::Component,
    ctx::Ctx,
    events::MouseEvent,
    theme::{BorderSize, IconSize, MotionDuration, MotionEasing, PaletteColor, ShadowStyle, Theme},
  },
  components::Row,
  core::Signal,
  layout::layout_kind::ScrollState,
  node::{Element, EventHandler, IntoMouseEventHandler, border::Border},
};

/// How far down a page is scrolled before its back to top button shows, in logical pixels, unless
/// [`BackToTop::threshold`] says another distance.
pub const BACK_TO_TOP_THRESHOLD: f32 = 300.0;

/// The parts of a back to top button a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BackToTopPart {
  /// What holds the button, whoever draws it: how it comes as it shows and goes as it is taken out. It clips
  /// nothing of the button's shadow.
  Root,
  /// The button itself, laid over the theme's recipe for a button's root: its side, its edge, its shadow.
  Button,
  /// Its arrow, laid over the theme's recipe for a button's icon.
  Icon,
}

names!(BackToTopPart {
  Root => "root",
  Button => "button",
  Icon => "icon",
});

/// The states of a back to top button a recipe styles, besides what the
/// pointer and the keyboard do to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BackToTopState {
  /// It takes no input.
  Disabled,
}

names!(BackToTopState {
  Disabled => "disabled",
});

/// What builds a back to top button's nodes: a theme's in place of the
/// default one. When it shows, how it comes and goes, and what a press does
/// stay the button's: what a renderer returns is put in the root, which fades.
pub type BackToTopRenderer = Arc<dyn Fn(BackToTopView) -> Element + Send + Sync>;

/// A back to top button: a round button that shows once a page is scrolled
/// down, and takes it back to its top.
///
/// ```no_run
/// use lurq::{
///   app::ctx::Ctx,
///   components::{Column, ScrollVertical, Stack},
///   layout::{StackAlignment, layout_kind::ScrollState},
///   node::Element,
///   ui::BackToTop,
/// };
///
/// fn page(ctx: &mut Ctx, scroll: &ScrollState, article: Column) -> Element {
///   Stack::new()
///     .stack_align(StackAlignment::BottomEnd)
///     .child(ScrollVertical::new(article).with_scroll_state(scroll.clone()))
///     .child(Column::new().padding(16.0).overflow_visible().child(BackToTop::new(scroll).mount(ctx)))
///     .into()
/// }
/// ```
///
/// It is given the [`ScrollState`] of what it scrolls, which the app gives the
/// scroll container too. It shows once the content is scrolled further down
/// than its [`threshold`](Self::threshold), 300 unless told, fading in, and
/// goes as the content comes back above it. A press takes the content to its
/// top over the frames that follow, as a wheel's scroll goes, and at once
/// while motion is reduced ([`Ctx::scroll_to_top_of`]); a handler of
/// [`on_click`](Self::on_click) hears the press first, and one that prevents
/// its default keeps the content where it is. Where it floats is the app's to
/// say, as a [`Fab`](super::Fab)'s is: it is an element like any other, put
/// in a corner of a `Stack` over the scroll, in something that does not clip
/// its shadow.
///
/// It is a button to tooling, called "Back to top" or by its label, a stop of
/// Tab while it shows. Once the content is at its top the button is gone, and
/// the keyboard's focus with it: the next Tab goes on from where it stood.
///
/// Converted into an element on its own, it is its still form: the button,
/// always shown, whose press takes the content to its top at once.
#[derive(Clone)]
pub struct BackToTop {
  scroll: ScrollState,
  label: Arc<str>,
  icon: Name,
  threshold: f32,
  force_shown: Option<bool>,
  disabled: bool,
  on_click: Vec<EventHandler<MouseEvent>>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl BackToTop {
  /// A button that takes the container `scroll` is the state of back to its top.
  pub fn new(scroll: &ScrollState) -> Self {
    Self {
      scroll: scroll.clone(),
      label: super::current_locale().navigation.back_to_top.clone(),
      icon: Name::new(icons::ARROW_UP),
      threshold: BACK_TO_TOP_THRESHOLD,
      force_shown: None,
      disabled: false,
      on_click: Vec::new(),
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// What the button is called to tooling that reads the tree, in place of "Back to top".
  pub fn label(mut self, label: &str) -> Self {
    self.label = Arc::from(label);
    self
  }

  /// The icon of the button, by its name in the theme's set, in place of the arrow that points up.
  pub fn icon(mut self, name: impl Into<Name>) -> Self {
    self.icon = name.into();
    self
  }

  /// How far down the content is scrolled before the button shows, in logical pixels: 300
  /// ([`BACK_TO_TOP_THRESHOLD`]) unless told. At 0 it shows as soon as the content leaves its top.
  pub fn threshold(mut self, threshold: impl Into<f32>) -> Self {
    self.threshold = threshold.into().max(0.0);
    self
  }

  /// Shows the button wherever the content is scrolled to (`Some(true)`), or never (`Some(false)`); `None`, as
  /// it starts, leaves it to the scroll. For a gallery, a test.
  pub fn force_shown(mut self, shown: Option<bool>) -> Self {
    self.force_shown = shown;
    self
  }

  /// Whether the button takes no input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Called on a press, and on Enter and Space while the button is focused, before the content is scrolled: a
  /// handler that prevents the event's default keeps the content where it is. A second handler is called after
  /// the first.
  pub fn on_click(mut self, handler: impl IntoMouseEventHandler) -> Self {
    self.on_click.push(handler.into_event_handler());
    self
  }

  /// Puts the button in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this button over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this button: what the component it is in says of it in this place.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the button as it is under the pointer, held down or focused from the keyboard, with nothing pointing
  /// at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the button.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Puts the button where it goes: it shows once the content is scrolled past its threshold, and a press
  /// takes the content to its top.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    ctx.mount::<BackToTopHost>(BackToTopProps { button: self })
  }

  /// The button as it is drawn in `theme`, whose press calls `go` unless a handler of the app's refused it.
  fn view(&self, theme: &Theme, go: Arc<dyn Fn() + Send + Sync>) -> BackToTopView {
    let mut case = Case::new()
      .breakpoint(super::current_breakpoint())
      .state(BackToTopState::Disabled, self.disabled);
    for state in &self.states {
      case = case.state(state.clone(), true);
    }
    BackToTopView {
      label: self.label.clone(),
      icon: self.icon,
      disabled: self.disabled,
      recipe: theme.resolve::<BackToTop>(&case, Some(&self.rules)),
      on_click: self.on_click.clone(),
      go,
      forced: self.forced,
      id: self.id.clone(),
      key: self.key.clone(),
    }
  }

  /// Builds what `view` shows: the theme's renderer's, or the default one's, in the root that fades in as it
  /// shows and out as it is taken from what holds it. The root has a key, the app's or one of its own, so that
  /// what holds it keeps it while it goes; a renderer has no part in that.
  fn render_view(mut view: BackToTopView) -> Element {
    let root = Row::new()
      .key(view.key.take().unwrap_or_else(|| Arc::from("back-to-top")))
      .part(&view.part(BackToTopPart::Root))
      .overflow_visible();
    let drawn = match view.theme().renderer::<BackToTop>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    };
    root.child(drawn).into()
  }

  /// Builds the button's still form in `theme`: always shown, its press takes the content to its top at once.
  /// Converting it into an element does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let scroll = self.scroll.clone();
    Self::render_view(self.view(theme, Arc::new(move || scroll.set_scroll(scroll.scroll_x(), 0.0))))
  }
}

impl From<BackToTop> for Element {
  fn from(button: BackToTop) -> Self {
    button.build(&current_theme())
  }
}

/// A back to top button as a renderer sees it: what it was given, the style
/// of its parts, and the button with its press attached.
pub struct BackToTopView {
  /// What the button is called to tooling.
  pub label: Arc<str>,
  /// The name of its icon.
  pub icon: Name,
  pub disabled: bool,
  /// The style of each part of this button.
  pub recipe: ResolvedRecipe,
  on_click: Vec<EventHandler<MouseEvent>>,
  go: Arc<dyn Fn() + Send + Sync>,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl BackToTopView {
  /// The style of a part, as this button is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The button: an outlined one that is its icon alone, round, with what the recipe says of its parts laid over
  /// the theme's button. A press tells the app's handlers, then takes the content to its top unless one of them
  /// refused.
  pub fn button(&self) -> Button {
    let rules = self
      .recipe
      .rules_for(BackToTopPart::Button, ButtonPart::Root)
      .extend(&self.recipe.rules_for(BackToTopPart::Icon, ButtonPart::Icon));
    let mut button = Button::new(&self.label)
      .variant(ButtonVariant::Outline)
      .size(ButtonSize::Lg)
      .shape(ButtonShape::Pill)
      .icon(self.icon)
      .icon_only(true)
      .disabled(self.disabled)
      .force(self.forced)
      .rules(&rules);
    for handler in &self.on_click {
      button = button.on_click(handler.clone());
    }
    let go = self.go.clone();
    button = button.on_click(move |event: MouseEvent| {
      if !event.default_prevented() {
        go();
      }
    });
    if let Some(id) = &self.id {
      button = button.id(id.clone());
    }
    button
  }

  /// How the default renderer builds a back to top button: the button. The component puts it in the root.
  pub fn render(self) -> Element {
    self.button().build(self.theme())
  }
}

impl Themed for BackToTop {
  const NAME: &'static str = "back_to_top";
  type Renderer = BackToTopRenderer;

  /// The design's back to top button (`pencil/lurq-design-system.pen`, Back
  /// To Top): a circle of 40 on `Background` in an edge of 1 in `Border`,
  /// with the medium shadow and an arrow of 16 in `Foreground`. Under the
  /// pointer, pressed and focused it is the outlined button it is made of.
  /// Its root fades in as it shows and out as it goes.
  fn recipe() -> Recipe {
    use BackToTopPart::{Button, Icon, Root};
    Recipe::new()
      .base(Root, |s| {
        s.enter(PartAnimation::new(FADE_IN, MotionDuration::Base, MotionEasing::Enter))
          .exit(PartAnimation::new(FADE_OUT, MotionDuration::Fast, MotionEasing::Exit))
      })
      .base(Button, |s| {
        s.square(40.0)
          .border(Border::inside(BorderSize::Sm, PaletteColor::Border))
          .shadow(ShadowStyle::Md)
      })
      .base(Icon, |s| s.square(IconSize::Md))
  }
}

/// A back to top button.
#[derive(Clone)]
struct BackToTopProps {
  button: BackToTop,
}

/// The button is built anew by every render of what holds it, so props are
/// never the same as before.
impl PartialEq for BackToTopProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

impl crate::app::component::DevtoolsInspectable for BackToTopProps {}

/// What the host reads of the scroll after each layout: its state, and how far down it shows the button.
struct Watched {
  scroll: ScrollState,
  threshold: f32,
}

/// What shows a back to top button once its content is scrolled, and takes the content to its top.
struct BackToTopHost {
  /// Whether the content is scrolled past the threshold: read after each layout.
  past: Signal<bool>,
  /// How many presses asked for the top, and how many of them a render has passed on to the window.
  asked: Signal<u32>,
  passed: Mutex<u32>,
  watched: Mutex<Option<Watched>>,
}

impl Component for BackToTopHost {
  type Props = BackToTopProps;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      past: ctx.signal(false),
      asked: ctx.signal(0),
      passed: Mutex::new(0),
      watched: Mutex::new(None),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let BackToTopProps { button } = ctx.props::<BackToTopProps>().clone();
    *self.watched.lock() = Some(Watched {
      scroll: button.scroll.clone(),
      threshold: button.threshold,
    });
    // Reading it subscribes: a press renders this again, with a context to ask the window by.
    let asked = self.asked.get();
    {
      let mut passed = self.passed.lock();
      if *passed != asked {
        *passed = asked;
        ctx.scroll_to_top_of(&button.scroll);
      }
    }
    // Where the content is when this is first built: the button of a page opened scrolled shows at once.
    let past = self.past.get() || button.scroll.scroll_y() > button.threshold;
    let shown = button.force_shown.unwrap_or(past);
    // What holds the button keeps its root while it fades out, and clips nothing of its shadow.
    let holder = Row::new().presence().overflow_visible();
    if !shown {
      return holder;
    }
    let go = {
      let asked = self.asked.clone();
      Arc::new(move || asked.update(|asked| *asked = asked.wrapping_add(1)))
    };
    holder.child(BackToTop::render_view(button.view(&current_theme(), go)))
  }

  fn after_layout(&self) {
    let Some(past) = self
      .watched
      .lock()
      .as_ref()
      .map(|watched| watched.scroll.scroll_y() > watched.threshold)
    else {
      return;
    };
    if self.past.get_untracked() != past {
      self.past.set(past);
    }
  }
}
