//! A carousel: slides one after another in a room that shows one, or a few, at a time.

use std::{
  sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
  },
  time::Duration,
};

use parking_lot::Mutex;

use super::{
  Button, ButtonPart, ButtonShape, ButtonVariant, icons, names,
  part_style::Length,
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::{fading, pressable},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    component::Component,
    ctx::{Ctx, Interval, Timeout},
    events::{DragEvent, KeyboardEvent, MouseEvent, ScrollEvent},
    theme::{OpacityLevel, PaletteColor, RadiusSize, Theme},
  },
  components::{Column, Row, ScrollHorizontal},
  core::{ElementRef, Signal},
  layout::{Alignment, layout_kind::ScrollState, scrollbar::ScrollBarStyle},
  node::{CursorIcon, Element, FocusGroup, NodeUpdate, dimension::Dimension},
};

/// How far a drag of the mouse takes the slides, as a share of a slide, before letting go turns to the next one.
const DRAG_TURNS_AT: f32 = 0.2;

/// The parts of a carousel a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CarouselPart {
  /// The carousel: the row of the slides and its buttons, and the dots under it.
  Root,
  /// The row of the button before, the slides, and the button after.
  Track,
  /// The room the slides are shown in, which cuts off what is not shown.
  Viewport,
  /// The room of one slide.
  Slide,
  /// The buttons that turn to the slide before and after: a button of the design system's, in this style.
  Arrow,
  /// The row of dots, one for each place the carousel can be at.
  Dots,
  /// What a press of a dot is in: the dot and the room about it.
  Dot,
  /// The dot itself.
  DotMark,
}

names!(CarouselPart {
  Root => "root",
  Track => "track",
  Viewport => "viewport",
  Slide => "slide",
  Arrow => "arrow",
  Dots => "dots",
  Dot => "dot",
  DotMark => "dot_mark",
});

/// The states of a carousel a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CarouselState {
  /// The dot of the place the carousel is at.
  Current,
  /// It takes nothing.
  Disabled,
}

names!(CarouselState {
  Current => "current",
  Disabled => "disabled",
});

/// What builds a carousel's nodes: a theme's in place of the default one.
pub type CarouselRenderer = Arc<dyn Fn(CarouselView) -> Element + Send + Sync>;

type Action = Arc<dyn Fn() + Send + Sync>;
type Told<T> = Arc<dyn Fn(T) + Send + Sync>;
type Said = Arc<dyn Fn(usize, usize) -> String + Send + Sync>;

/// The words a carousel writes itself.
#[derive(Clone)]
struct Words {
  label: Arc<str>,
  previous: Arc<str>,
  next: Arc<str>,
}

impl Default for Words {
  fn default() -> Self {
    let locale = super::current_locale();
    Self {
      label: locale.navigation.carousel.clone(),
      previous: locale.navigation.previous_slide.clone(),
      next: locale.navigation.next_slide.clone(),
    }
  }
}

/// A carousel: slides one after another, of which the room shows one, or as many as it is told, at a time.
///
/// ```
/// use lurq::ui::{Carousel, Picture};
///
/// let photos = Carousel::new([
///   Picture::from_file("lake.jpg").aspect_ratio(16.0 / 9.0),
///   Picture::from_file("ridge.jpg").aspect_ratio(16.0 / 9.0),
///   Picture::from_file("shore.jpg").aspect_ratio(16.0 / 9.0),
/// ])
/// .wrap(true);
/// ```
///
/// The round buttons beside the slides turn to the one before and after, the dots under them to any, and the
/// left and right arrow keys on a button turn as it does. A finger swipes from one to the next, a drag of the
/// mouse does the same, and a sideways scroll of a trackpad comes to rest on a slide.
///
/// A carousel is a component: it keeps which slide it is at, unless the app gives it a signal for it
/// ([`index`](Self::index)).
#[derive(Clone)]
pub struct Carousel {
  slides: Vec<Element>,
  index: Option<Signal<usize>>,
  controlled: Option<usize>,
  on_index_change: Vec<Told<usize>>,
  per_view: usize,
  gap: f32,
  wrap: bool,
  arrows: bool,
  dots: bool,
  autoplay: Option<Duration>,
  width: Option<Length>,
  disabled: bool,
  words: Words,
  slide_label: Option<Said>,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Carousel {
  /// A carousel of `slides`, at the first.
  pub fn new(slides: impl IntoIterator<Item = impl Into<Element>>) -> Self {
    Self {
      slides: slides.into_iter().map(Into::into).collect(),
      index: None,
      controlled: None,
      on_index_change: Vec::new(),
      per_view: 1,
      gap: 12.0,
      wrap: false,
      arrows: true,
      dots: true,
      autoplay: None,
      width: None,
      disabled: false,
      words: Words::default(),
      slide_label: None,
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// One more slide, after the others.
  pub fn slide(mut self, slide: impl Into<Element>) -> Self {
    self.slides.push(slide.into());
    self
  }

  /// More slides, after the others.
  pub fn slides(mut self, slides: impl IntoIterator<Item = impl Into<Element>>) -> Self {
    self.slides.extend(slides.into_iter().map(Into::into));
    self
  }

  /// Which slide the carousel is at, counted from 0, to read and to set: the first one shown, where it shows
  /// several. Without it the carousel keeps it, and starts at the first.
  pub fn index(mut self, index: Signal<usize>) -> Self {
    self.index = Some(index);
    self
  }

  /// Which slide the carousel is at, as the app says it each time: the carousel asks for another with
  /// `on_change` and shows what it is told.
  pub fn index_controlled(mut self, index: usize, on_change: impl Fn(usize) + Send + Sync + 'static) -> Self {
    self.controlled = Some(index);
    self.on_index_change.push(Arc::new(on_change));
    self
  }

  /// Called when the carousel comes to another slide, with the one it is.
  pub fn on_index_change(mut self, handler: impl Fn(usize) + Send + Sync + 'static) -> Self {
    self.on_index_change.push(Arc::new(handler));
    self
  }

  /// How many slides the room shows at once, each as wide as its share: 1 unless told. A turn moves by one.
  pub fn per_view(mut self, per_view: usize) -> Self {
    self.per_view = per_view.max(1);
    self
  }

  /// The room between two slides: 12 unless told.
  pub fn gap(mut self, gap: f32) -> Self {
    self.gap = gap.max(0.0);
    self
  }

  /// Whether the first slide comes after the last, and the last before the first.
  pub fn wrap(mut self, wrap: bool) -> Self {
    self.wrap = wrap;
    self
  }

  /// Whether the round buttons beside the slides show: they do unless told.
  pub fn arrows(mut self, arrows: bool) -> Self {
    self.arrows = arrows;
    self
  }

  /// Whether the dots under the slides show: they do unless told, where there is more than one place to be at.
  pub fn dots(mut self, dots: bool) -> Self {
    self.dots = dots;
    self
  }

  /// Turns to the next slide by itself every `every`, from the last to the first. It waits while the pointer
  /// is over the carousel and while the focus is in it from the keyboard, and stops for good once a person
  /// turns it.
  pub fn autoplay(mut self, every: Duration) -> Self {
    self.autoplay = Some(every);
    self
  }

  /// How wide the carousel is, its buttons with it: all the room there is unless told, and never wider than
  /// its room. The slides share what the buttons leave.
  pub fn width(mut self, width: impl Into<Length>) -> Self {
    self.width = Some(width.into());
    self
  }

  /// Half the opacity; it takes nothing.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// What it is called to tooling that reads the tree: "Carousel" unless told.
  pub fn label(mut self, label: &str) -> Self {
    self.words.label = Arc::from(label);
    self
  }

  /// What the button before is called: "Previous slide" unless told.
  pub fn previous_label(mut self, label: &str) -> Self {
    self.words.previous = Arc::from(label);
    self
  }

  /// What the button after is called: "Next slide" unless told.
  pub fn next_label(mut self, label: &str) -> Self {
    self.words.next = Arc::from(label);
    self
  }

  /// What a slide and its dot are called, from which it is, counted from 1, and how many there are: "Slide 2
  /// of 4" unless told.
  pub fn slide_label(mut self, label: impl Fn(usize, usize) -> String + Send + Sync + 'static) -> Self {
    self.slide_label = Some(Arc::new(label));
    self
  }

  /// Turns a state a recipe's rules name on or off.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// The style of one part, over the theme's.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Rules over the theme's.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// The carousel mounted in `ctx`: what converting it into an element does once the render that returns it
  /// has returned.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let key = self.key.clone();
    let props = CarouselProps(self);
    match key {
      Some(key) => ctx.mount_keyed::<CarouselHost>(&key, props),
      None => ctx.mount::<CarouselHost>(props),
    }
  }

  /// What slide `at` of `count`, counted from 1, is called.
  fn name_of(&self, at: usize, count: usize) -> String {
    match &self.slide_label {
      Some(label) => label(at, count),
      None => crate::core::fill(
        &super::current_locale().navigation.slide_of,
        &[("at", &at.to_string()), ("total", &count.to_string())],
      ),
    }
  }
}

/// A carousel in a tree is its component, declared: the component that renders the tree mounts it.
impl From<Carousel> for Element {
  fn from(carousel: Carousel) -> Self {
    let key = carousel.key.clone();
    let props = CarouselProps(carousel);
    match key {
      Some(key) => Element::component_keyed::<CarouselHost>(key, props),
      None => Element::component::<CarouselHost>(props),
    }
  }
}

/// A carousel to mount.
#[derive(Clone)]
struct CarouselProps(Carousel);

/// Its slides and its handlers are made anew by every render of what holds it.
impl PartialEq for CarouselProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

impl crate::app::component::DevtoolsInspectable for CarouselProps {}

/// The component of a carousel: what it keeps between renders.
struct CarouselHost {
  /// Which slide it is at, where the app names no signal for it.
  index: Signal<usize>,
  /// The list the slides scroll in.
  scroll: ScrollState,
  /// The room the slides are shown in, as it is laid out.
  viewport: ElementRef,
  /// The carousel as it is laid out: what the pointer is over and the focus in.
  root: ElementRef,
  /// The width of the room the slides were last laid out for.
  room: Signal<f32>,
  /// Where the slides are, or are going: the slide the list was last sent to or came to, and how far apart
  /// two slides were then. Nothing until they are first placed.
  placed: Arc<Mutex<Option<(usize, f32)>>>,
  /// Asks for the slides to come to rest where the carousel is at: a drag let go, a scroll that ended.
  settle: Signal<u64>,
  settle_wanted: Arc<AtomicBool>,
  /// A person turned the carousel: it turns by itself no more.
  stopped: Arc<AtomicBool>,
  /// What turns it by itself, once it is asked to.
  timer: Mutex<Option<Interval>>,
  /// What the timer does: turn to the next slide, as this render has it.
  advance: Arc<Mutex<Option<Action>>>,
  /// The slide a drag of the mouse started at.
  dragged_from: Arc<Mutex<usize>>,
  /// Which slide it is at, as the last render had it: what a drag that outlives renders reads.
  current: Arc<AtomicUsize>,
  /// The buttons before and after: a button that turned to the end takes nothing more, and gives its focus to
  /// the other.
  arrows: [ElementRef; 2],
  /// A moment after the list last scrolled, it comes to rest on a slide: a mouse's wheel tells of no end.
  rest: Timeout,
  /// What coming to rest does, as this render has it.
  resting: Arc<Mutex<Option<Action>>>,
}

/// How long after the list last scrolled it comes to rest on a slide.
const REST_AFTER: Duration = Duration::from_millis(180);

impl Component for CarouselHost {
  type Props = CarouselProps;

  fn create(ctx: &mut Ctx) -> Self {
    let scroll = ScrollState::new();
    // A finger goes a page at a time, and a page is as wide as the list: as many slides as it shows.
    scroll.set_paging(true);
    let resting: Arc<Mutex<Option<Action>>> = Arc::default();
    let rest = {
      let resting = resting.clone();
      ctx.create_timeout(REST_AFTER, move || {
        let rest = resting.lock().clone();
        if let Some(rest) = rest {
          rest();
        }
      })
    };
    Self {
      index: ctx.signal(0),
      scroll,
      viewport: ElementRef::new(),
      root: ElementRef::new(),
      room: ctx.signal(0.0),
      placed: Arc::default(),
      settle: ctx.signal(0),
      settle_wanted: Arc::default(),
      stopped: Arc::default(),
      timer: Mutex::new(None),
      advance: Arc::default(),
      dragged_from: Arc::default(),
      current: Arc::default(),
      arrows: [ElementRef::new(), ElementRef::new()],
      rest,
      resting,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let carousel = ctx.props::<CarouselProps>().0.clone();
    let theme = current_theme();
    let count = carousel.slides.len();
    let shown = carousel.per_view.min(count.max(1));
    // The places it can be at: the first slide shown is any but the last few, which the room shows together.
    let places = count.saturating_sub(shown) + 1;
    let index_signal = carousel.index.clone().unwrap_or_else(|| self.index.clone());
    let index = carousel
      .controlled
      .unwrap_or_else(|| index_signal.get())
      .min(places - 1);
    let _ = self.settle.get();
    self.current.store(index, Ordering::Relaxed);

    // A slide's room is its share of the list, the gap after it with it: the list is as wide as the room the
    // slides are shown in and one gap more, which that room cuts off. A page of the list is then as many
    // slides as the room shows, and a finger's page comes to rest on a slide.
    let room = self.room.get();
    let step = if room > 0.0 {
      (room + carousel.gap) / shown as f32
    } else {
      0.0
    };

    // The slides go where the carousel is at: at once the first time and when the room changed, over a few
    // frames when it was turned, and not at all when they got there themselves (a finger, a trackpad).
    if step > 0.0 {
      let mut placed = self.placed.lock();
      let settle = self.settle_wanted.swap(false, Ordering::Relaxed);
      let to = index as f32 * step;
      match *placed {
        Some((at, was)) if (was - step).abs() < 0.25 => {
          if at != index || settle {
            ctx.scroll_state_to(&self.scroll, to, 0.0);
          }
        }
        _ => self.scroll.set_scroll_pending(to, 0.0),
      }
      *placed = Some((index, step));
    }

    let tell = {
      let (signal, controlled, told) = (
        index_signal.clone(),
        carousel.controlled.is_some(),
        carousel.on_index_change.clone(),
      );
      move |to: usize| {
        if !controlled {
          signal.set(to);
        }
        told.iter().for_each(|tell| tell(to));
      }
    };
    // A person's turn: to the slide `to`.
    let turn: Told<usize> = {
      let (tell, stopped, disabled) = (tell.clone(), self.stopped.clone(), carousel.disabled);
      Arc::new(move |to: usize| {
        stopped.store(true, Ordering::Relaxed);
        if disabled || to >= places || to == index {
          return;
        }
        tell(to);
      })
    };
    *self.advance.lock() = (places > 1).then(|| {
      let tell = tell.clone();
      let next = if index + 1 < places { index + 1 } else { 0 };
      Arc::new(move || tell(next)) as Action
    });
    self.autoplay(ctx, &carousel, places);

    // What the list tells: where it came to, a finger's or a trackpad's way, which the dots follow; and that it
    // stopped, where it is to come to rest on a slide.
    let scrolled: Arc<dyn Fn() + Send + Sync> = {
      let (scroll, placed, tell) = (self.scroll.clone(), self.placed.clone(), tell.clone());
      Arc::new(move || {
        let mut placed = placed.lock();
        let Some((at, step)) = *placed else {
          return;
        };
        if step <= 0.0 {
          return;
        }
        let (going, _) = scroll.scroll_target();
        let near = ((going / step).round().max(0.0) as usize).min(places - 1);
        if near != at {
          *placed = Some((near, step));
          drop(placed);
          tell(near);
        }
      })
    };
    let settle: Action = {
      let (wanted, signal) = (self.settle_wanted.clone(), self.settle.clone());
      Arc::new(move || {
        wanted.store(true, Ordering::Relaxed);
        signal.set(signal.get_untracked().wrapping_add(1));
      })
    };
    // Where the list stopped, the slide nearest is the one it is at, and it goes there. A scroll's handlers
    // hear of it before the list moves: the slide it came to is known once it stopped.
    let rest: Action = {
      let (scrolled, settle) = (scrolled.clone(), settle.clone());
      Arc::new(move || {
        scrolled();
        settle();
      })
    };
    *self.resting.lock() = Some(rest.clone());
    // A scroll that is not the slides' own way to a slide comes to rest a moment after it stops.
    let scrolled: Action = {
      let (scrolled, rest, scroll) = (scrolled.clone(), self.rest.clone(), self.scroll.clone());
      Arc::new(move || {
        scrolled();
        let (going, _) = scroll.scroll_target();
        if (going - scroll.scroll_x()).abs() < 0.5 {
          rest.restart();
        }
      })
    };
    // A drag of the mouse carries the slides; let go, they go on to the next one where it went far enough.
    let dragged: Dragged = {
      let (from, placed, scroll, stopped, current) = (
        self.dragged_from.clone(),
        self.placed.clone(),
        self.scroll.clone(),
        self.stopped.clone(),
        self.current.clone(),
      );
      let (tell, settle) = (tell.clone(), settle.clone());
      Dragged {
        start: Arc::new({
          let (from, current) = (from.clone(), current.clone());
          move || {
            stopped.store(true, Ordering::Relaxed);
            *from.lock() = current.load(Ordering::Relaxed);
          }
        }),
        moved: Arc::new(move |by: f32| scroll.scroll_by(-by, 0.0)),
        end: Arc::new(move |by: f32| {
          let start = *from.lock();
          let step = placed.lock().map_or(0.0, |(_, step)| step);
          let far = step > 0.0 && by.abs() > step * DRAG_TURNS_AT;
          let to = match (far, by < 0.0) {
            (true, true) => (start + 1).min(places - 1),
            (true, false) => start.saturating_sub(1),
            (false, _) => start,
          };
          // Where the list is going is not where it is to rest: the slides are sent there anew.
          if let Some(placed) = placed.lock().as_mut() {
            placed.0 = usize::MAX;
          }
          if to != current.load(Ordering::Relaxed) {
            tell(to);
          }
          settle();
        }),
      }
    };

    // A button that has nothing more to turn to takes no focus: the focus it has goes to the other one, where
    // the keys turn on.
    if !carousel.wrap && places > 1 {
      let [before, after] = &self.arrows;
      if index == 0 && before.focused() {
        ctx.focus(after);
      } else if index == places - 1 && after.focused() {
        ctx.focus(before);
      }
    }

    let case = |current: bool| {
      Case {
        variant: None,
        size: None,
        density: None,
        breakpoint: super::current_breakpoint(),
        states: carousel.states.clone(),
      }
      .state(CarouselState::Disabled, carousel.disabled)
      .state(CarouselState::Current, current)
    };
    let view = CarouselView {
      recipe: theme.resolve::<Carousel>(&case(false), Some(&carousel.rules)),
      current: theme.resolve::<Carousel>(&case(true), Some(&carousel.rules)),
      index,
      places,
      shown,
      step,
      turn,
      scrolled,
      settle: rest,
      dragged,
      scroll: self.scroll.clone(),
      viewport: self.viewport.clone(),
      root: self.root.clone(),
      arrows: self.arrows.clone(),
      carousel,
    };
    match theme.renderer::<Carousel>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }

  /// The slides are as wide as their share of the room they are shown in, which is known once it is laid out.
  fn after_layout(&self) {
    if !self.viewport.is_attached() {
      return;
    }
    let room = self.viewport.width();
    if (self.room.get_untracked() - room).abs() > 0.25 {
      self.room.set(room);
    }
  }
}

impl CarouselHost {
  /// Starts the timer that turns the carousel by itself, once it is asked to, and stops it once it is not.
  fn autoplay(&self, ctx: &mut Ctx, carousel: &Carousel, places: usize) {
    let mut timer = self.timer.lock();
    match carousel.autoplay {
      Some(every) if places > 1 && !carousel.disabled => {
        if timer.is_none() {
          let (stopped, root, advance) = (self.stopped.clone(), self.root.clone(), self.advance.clone());
          let interval = ctx.create_interval(every, move || {
            if stopped.load(Ordering::Relaxed) || root.hovered() || root.focus_visible_within() {
              return;
            }
            let advance = advance.lock().clone();
            if let Some(advance) = advance {
              advance();
            }
          });
          interval.start();
          *timer = Some(interval);
        }
      }
      _ => {
        if let Some(interval) = timer.take() {
          interval.stop();
        }
      }
    }
  }
}

/// What a drag of the mouse over the slides does: where it starts, by how much it moves them, and how far it
/// went in all when it is let go.
#[derive(Clone)]
struct Dragged {
  start: Action,
  moved: Told<f32>,
  end: Told<f32>,
}

/// A carousel as a renderer sees it: what it was given, which slide it is at, and the style of its parts.
pub struct CarouselView {
  /// The style of each part.
  pub recipe: ResolvedRecipe,
  /// The style of each part of the dot of the place the carousel is at.
  current: ResolvedRecipe,
  /// Which slide it is at, counted from 0: the first one shown.
  pub index: usize,
  /// How many places it can be at: one for each slide but the last few, which are shown together.
  pub places: usize,
  /// How many slides the room shows at once.
  pub shown: usize,
  /// How far apart two slides are, the gap with them: nothing until the room is laid out.
  pub step: f32,
  turn: Told<usize>,
  scrolled: Action,
  settle: Action,
  dragged: Dragged,
  scroll: ScrollState,
  viewport: ElementRef,
  root: ElementRef,
  arrows: [ElementRef; 2],
  carousel: Carousel,
}

impl CarouselView {
  /// The style of a part.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The id of a part of it, from the carousel's.
  fn id(&self, part: &str) -> Option<String> {
    self.carousel.id.as_ref().map(|id| format!("{id}-{part}"))
  }

  /// The slides before and after the one it is at: the last before the first, and the first after the last,
  /// where it wraps. Nothing where there is none.
  fn neighbours(&self) -> (Option<usize>, Option<usize>) {
    let (index, last, wrap) = (self.index, self.places - 1, self.carousel.wrap && self.places > 1);
    let before = match index {
      0 if wrap => Some(last),
      0 => None,
      _ => Some(index - 1),
    };
    let after = match index == last {
      true if wrap => Some(0),
      true => None,
      false => Some(index + 1),
    };
    (before, after)
  }

  /// A round button beside the slides that turns to `to`: before it with the left arrow key, after it with
  /// the right one.
  fn arrow(&self, icon: &'static str, label: &Arc<str>, part: &str, to: Option<usize>, held: &ElementRef) -> Element {
    let (before, after) = self.neighbours();
    let rules = self.recipe.rules_for(CarouselPart::Arrow, ButtonPart::Root);
    let turn = self.turn.clone();
    let mut button = Button::new(label.as_ref())
      .icon(icon)
      .icon_only(true)
      .variant(ButtonVariant::Outline)
      .shape(ButtonShape::Pill)
      .ref_element(held.clone())
      .rules(&rules)
      .disabled(self.carousel.disabled || to.is_none())
      .on_click({
        let turn = turn.clone();
        move |_: MouseEvent| {
          if let Some(to) = to {
            turn(to);
          }
        }
      });
    if let Some(id) = self.id(part) {
      button = button.id(id);
    }
    Row::new()
      .on_key_down(move |event: KeyboardEvent| {
        // A key is told to every element; this one is the button's that has the focus.
        if !event.focus_within {
          return;
        }
        let to = match event.key.as_str() {
          "ArrowLeft" => before,
          "ArrowRight" => after,
          _ => return,
        };
        event.prevent_default();
        event.stop_propagation();
        if let Some(to) = to {
          turn(to);
        }
      })
      .child(button)
      .into()
  }

  /// The button before the slides, where it shows.
  pub fn previous(&self) -> Option<Element> {
    let (before, _) = self.neighbours();
    self.carousel.arrows.then(|| {
      self.arrow(
        icons::ARROW_LEFT,
        &self.carousel.words.previous,
        "previous",
        before,
        &self.arrows[0],
      )
    })
  }

  /// The button after the slides, where it shows.
  pub fn next(&self) -> Option<Element> {
    let (_, after) = self.neighbours();
    self.carousel.arrows.then(|| {
      self.arrow(
        icons::ARROW_RIGHT,
        &self.carousel.words.next,
        "next",
        after,
        &self.arrows[1],
      )
    })
  }

  /// The room the slides are shown in: the list of them, which a finger swipes, a drag of the mouse carries and
  /// a trackpad scrolls sideways. Only the slides shown take the pointer and the focus.
  pub fn viewport(&self) -> Element {
    let carousel = &self.carousel;
    let count = carousel.slides.len();
    let full = Dimension::Pct(100.0);
    let mut list = Row::new().align_items(Alignment::Stretch);
    for (at, slide) in carousel.slides.iter().enumerate() {
      let mut room = Column::new()
        .part(&self.part(CarouselPart::Slide))
        .align_items(Alignment::Stretch)
        .padding_right(carousel.gap)
        .describe("a11y_role", "group")
        .describe(
          "a11y_roledescription",
          super::current_locale().navigation.slide_role.clone(),
        )
        .describe("a11y_name", carousel.name_of(at + 1, count))
        .child(slide.clone());
      if self.step > 0.0 {
        room = room.width(self.step);
      } else {
        room = room.width(full);
      }
      if let Some(id) = self.id(&format!("slide-{at}")) {
        room = room.id(id);
      }
      let mut room: Element = room.into();
      if !(self.index..self.index + self.shown).contains(&at) {
        NodeUpdate::inert(&mut *room.node, true);
      }
      list = list.child(room);
    }
    // `settle` is where a scroll that ended comes to rest.
    let (scrolled, rest) = (self.scrolled.clone(), self.settle.clone());
    let mut slides = ScrollHorizontal::new(list)
      .with_scroll_state(self.scroll.clone())
      .scrollbar(ScrollBarStyle::hidden())
      .on_scroll(move |_: ScrollEvent| scrolled())
      .on_scroll_end(move |_: ScrollEvent| rest());
    if !carousel.disabled && self.places > 1 {
      let Dragged { start, moved, end } = self.dragged.clone();
      slides = slides
        .on_drag_start(move |_: DragEvent| start())
        .on_drag_move(move |event: DragEvent| moved(event.delta_x))
        .on_drag_end(move |event: DragEvent| end(event.total_delta_x));
    }
    // One gap wider than the room, which cuts it off: see `step`.
    slides = match self.step > 0.0 {
      true => slides.width(self.step * self.shown as f32),
      false => slides.width(full),
    };
    let mut viewport = Row::new()
      .part(&self.part(CarouselPart::Viewport))
      .flex(1.0)
      .min_width(0.0)
      .clip()
      .ref_element(self.viewport.clone())
      .child(slides);
    if let Some(id) = self.id("viewport") {
      viewport = viewport.id(id);
    }
    let mut viewport: Element = viewport.into();
    if carousel.disabled {
      NodeUpdate::inert(&mut *viewport.node, true);
    }
    viewport
  }

  /// The dots under the slides, one for each place it can be at, the current one marked, where they show. The
  /// arrow keys move among them and Enter turns to the one with the focus.
  pub fn dots(&self) -> Option<Element> {
    let carousel = &self.carousel;
    if !carousel.dots || self.places < 2 {
      return None;
    }
    let mut dots = Row::new()
      .part(&self.part(CarouselPart::Dots))
      .align_items(Alignment::Center)
      .focus_group(FocusGroup::horizontal().one_stop(true))
      .describe("a11y_role", "tablist")
      .describe("a11y_name", carousel.words.label.clone());
    for at in 0..self.places {
      let current = at == self.index;
      let recipe = if current { &self.current } else { &self.recipe };
      let turn = self.turn.clone();
      let mut dot = pressable(&recipe.part(CarouselPart::Dot), carousel.disabled)
        .describe("a11y_role", "tab")
        .describe("a11y_name", carousel.name_of(at + 1, carousel.slides.len()))
        .describe("a11y_selected", if current { "true" } else { "false" })
        .child({
          let mark = Row::new().part(&recipe.part(CarouselPart::DotMark));
          match self.id(&format!("dot-{at}-mark")) {
            Some(id) => mark.id(id),
            None => mark,
          }
        })
        .on_click(move |_: MouseEvent| turn(at));
      if let Some(id) = self.id(&format!("dot-{at}")) {
        dot = dot.id(id);
      }
      dots = dots.child(dot);
    }
    Some(dots.into())
  }

  /// How the default renderer builds a carousel: the slides between the two buttons, and the dots under them.
  pub fn render(self) -> Element {
    let carousel = &self.carousel;
    let full = Dimension::Pct(100.0);
    let width = carousel
      .width
      .and_then(|width| width.resolve(self.theme()))
      .unwrap_or(full);
    let track = Row::new()
      .part(&self.part(CarouselPart::Track))
      .width(full)
      .align_items(Alignment::Center)
      .with_children(self.previous())
      .child(self.viewport())
      .with_children(self.next());
    let mut root = Column::new()
      .part(&self.part(CarouselPart::Root))
      .width(width)
      .max_width(full)
      .align_items(Alignment::Center)
      .ref_element(self.root.clone())
      .describe("a11y_role", "region")
      .describe(
        "a11y_roledescription",
        super::current_locale().navigation.carousel_role.clone(),
      )
      .describe("a11y_name", carousel.words.label.clone())
      .child(track)
      .with_children(self.dots());
    if let Some(id) = &carousel.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &carousel.key {
      root = root.key(key.clone());
    }
    root.into()
  }
}

impl crate::ui::Styled for Carousel {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for Carousel {
  const NAME: &'static str = "carousel";
  type Renderer = CarouselRenderer;

  fn parts() -> crate::ui::Declared {
    CarouselPart::declared()
  }

  fn states() -> crate::ui::Declared {
    CarouselState::declared()
  }

  /// The design's carousel (`pencil/lurq-design-system.pen`, Carousel): the slides between two round outline
  /// buttons of 36 with arrows of 16, 12 from them, and under them, 12 down, dots of 6 that are 6 apart, in
  /// `Input` and the current one in `Primary`. A dot is pressed in a room 3 larger on every side.
  fn recipe() -> Recipe {
    use CarouselPart::*;
    use PaletteColor::{Input, Primary};
    Recipe::new()
      .base(Root, |s| s.gap(12.0))
      .base(Track, |s| s.gap(12.0))
      .base(Dot, |s| {
        s.padding_x(3.0)
          .padding_y(3.0)
          .radius(RadiusSize::Full)
          .cursor(CursorIcon::Pointer)
      })
      .focus_visible(Dot, |s| s.focus_ring())
      .base(DotMark, |s| {
        fading(s.square(6.0).radius(RadiusSize::Full).background(Input))
      })
      .state(CarouselState::Current, DotMark, |s| s.background(Primary))
      .state(CarouselState::Disabled, Root, |s| s.opacity(OpacityLevel::Disabled))
      .state(CarouselState::Disabled, Dot, |s| s.cursor(CursorIcon::Default))
  }
}
