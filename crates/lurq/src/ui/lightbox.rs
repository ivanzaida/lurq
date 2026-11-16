//! A lightbox: pictures looked at one at a time over the window, larger and closer.

use std::sync::Arc;

use super::{
  FontFamily, Picture, icons, names,
  part_style::{Length, PartTransition},
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::{fading, glyph, pressable},
  theme::{Themed, current_theme},
};
use crate::{
  animation::Transition,
  app::{
    component::Component,
    ctx::Ctx,
    events::{KeyboardEvent, MouseEvent},
    theme::{MotionDuration, RadiusSize, Theme, TypographyStyle},
  },
  components::{Column, Modal, Root, Row, Stack, ZoomScroll, ZoomScrollProps},
  core::{ElementRef, Signal},
  layout::{
    Alignment, StackAlignment,
    layout_kind::{Justify, ScrollState},
    text_style::{FontWeight, TextAlign},
  },
  node::{
    CursorIcon, Element, HitTestBehavior, NodeUpdate, border::Border, color::Color, dimension::Dimension,
    outline::Outline,
  },
};

/// How much one press of Zoom in, or one notch of the wheel, brings the picture closer.
const ZOOM_STEP: f32 = 1.5;
/// How far an arrow key moves a picture that is zoomed in.
const PAN_STEP: f32 = 48.0;
/// The top row's padding at its two ends, the room a button of it takes with the gap after it, the level's, the
/// gap alone, and the width of a letter of the counter: what tells whether the row has room for its level.
const TOP_SIDES: f32 = 24.0;
const CONTROL_ROOM: f32 = 40.0;
const LEVEL_ROOM: f32 = 60.0;
const CONTROL_GAP: f32 = 4.0;
const COUNTER_LETTER: f32 = 8.0;

/// The room between the strip's small pictures, as the design has it, and the least there is at its ends:
/// what tells how many of them the strip has room for.
const STRIP_GAP: f32 = 6.0;
const STRIP_INSET: f32 = 8.0;

/// The parts of a lightbox a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LightboxPart {
  /// What covers the window under the lightbox.
  Veil,
  /// The lightbox: its fill, and its corners where it stands in a page.
  Root,
  /// The row along the top: the counter at its start, the buttons at its end.
  Top,
  /// Which picture is shown, in the counter.
  Counter,
  /// How many there are, after it.
  Total,
  /// A button of the top row.
  Control,
  /// The glyph of a button of the top row.
  ControlIcon,
  /// The button that says how close the picture is, and fits it again.
  Level,
  /// The room the picture is shown in.
  Stage,
  /// The round button over a side of the stage that turns to the picture before or after.
  Step,
  /// The glyph of that button.
  StepIcon,
  /// The side of the stage such a button lies over: its padding is how far in from the edge the button is.
  Side,
  /// The picture's title and description, over its lower edge. Its background is the color the gradient under
  /// them ends in, from nothing at its top.
  Caption,
  /// The picture's title.
  Title,
  /// The line under it.
  Description,
  /// The strip of small pictures along the bottom.
  Filmstrip,
  /// One of them.
  Thumb,
}

names!(LightboxPart {
  Veil => "veil",
  Root => "root",
  Top => "top",
  Counter => "counter",
  Total => "total",
  Control => "control",
  ControlIcon => "control_icon",
  Level => "level",
  Stage => "stage",
  Step => "step",
  StepIcon => "step_icon",
  Side => "side",
  Caption => "caption",
  Title => "title",
  Description => "description",
  Filmstrip => "filmstrip",
  Thumb => "thumb",
});

/// The states of a lightbox a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LightboxState {
  /// It stands in the page where it is declared, not over the window.
  Inline,
  /// The small picture of the strip that is the one shown.
  Current,
}

names!(LightboxState {
  Inline => "inline",
  Current => "current",
});

/// What builds a lightbox's nodes: a theme's in place of the default one.
pub type LightboxRenderer = Arc<dyn Fn(LightboxView) -> Element + Send + Sync>;

type Action = Arc<dyn Fn() + Send + Sync>;
type Told<T> = Arc<dyn Fn(T) + Send + Sync>;

/// The words a lightbox writes itself.
#[derive(Clone)]
struct Words {
  label: Arc<str>,
  close: Arc<str>,
  previous: Arc<str>,
  next: Arc<str>,
  zoom_in: Arc<str>,
  zoom_out: Arc<str>,
  fit: Arc<str>,
  download: Arc<str>,
}

impl Default for Words {
  fn default() -> Self {
    Self {
      label: Arc::from("Pictures"),
      close: Arc::from("Close"),
      previous: Arc::from("Previous picture"),
      next: Arc::from("Next picture"),
      zoom_in: Arc::from("Zoom in"),
      zoom_out: Arc::from("Zoom out"),
      fit: Arc::from("Fit to the window"),
      download: Arc::from("Download"),
    }
  }
}

/// A lightbox: pictures shown one at a time over the window.
///
/// ```
/// use lurq::{core::Signal, ui::{Lightbox, Picture}};
///
/// let open = Signal::new(false);
/// let viewer = Lightbox::new(open.clone(), [
///   Picture::from_file("lake.jpg").title("Lake at dusk"),
///   Picture::from_file("ridge.jpg").title("The ridge"),
/// ]);
/// open.set(true);
/// ```
///
/// It is open while its signal says so. The picture fills the room it has, whole; the counter says which one it
/// is; the buttons at the top bring it closer and further and fit it again, and close the lightbox; the round
/// buttons over its sides turn to the one before and after; the strip along the bottom shows them all. The
/// picture's [`title`](Picture::title) and [`description`](Picture::description) lie over its lower edge.
///
/// The wheel brings the picture closer about the pointer, as `+` and `-` do about its middle; `0` fits it
/// again and a double click goes in and back out. Closer than it fits, a drag and the arrow keys move it;
/// fitted, the left and right arrows turn to the picture before and after. Escape closes it.
///
/// A lightbox is a component: it keeps which picture is shown and how close, unless the app gives it signals
/// for them ([`index`](Self::index), [`zoom`](Self::zoom)).
#[derive(Clone)]
pub struct Lightbox {
  open: Signal<bool>,
  pictures: Vec<Picture>,
  index: Option<Signal<usize>>,
  controlled: Option<usize>,
  on_index_change: Vec<Told<usize>>,
  zoom: Option<Signal<f32>>,
  on_zoom_change: Vec<Told<f64>>,
  zoom_max: f32,
  preload: usize,
  filmstrip: Option<bool>,
  counter: Option<bool>,
  counter_text: Option<Arc<dyn Fn(usize, usize) -> String + Send + Sync>>,
  wrap: bool,
  on_download: Vec<Told<usize>>,
  actions: Vec<Element>,
  inline: bool,
  width: Option<Length>,
  height: Option<Length>,
  on_dismiss: Vec<Action>,
  dismiss_on_escape: bool,
  dismiss_on_veil: bool,
  close_button: bool,
  words: Words,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Lightbox {
  /// A lightbox of `pictures`, open while `open` says so.
  pub fn new(open: Signal<bool>, pictures: impl IntoIterator<Item = Picture>) -> Self {
    Self {
      open,
      pictures: pictures.into_iter().collect(),
      index: None,
      controlled: None,
      on_index_change: Vec::new(),
      zoom: None,
      on_zoom_change: Vec::new(),
      zoom_max: 4.0,
      preload: 1,
      filmstrip: None,
      counter: None,
      counter_text: None,
      wrap: false,
      on_download: Vec::new(),
      actions: Vec::new(),
      inline: false,
      width: None,
      height: None,
      on_dismiss: Vec::new(),
      dismiss_on_escape: true,
      dismiss_on_veil: true,
      close_button: true,
      words: Words::default(),
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// One more picture, after the others.
  pub fn picture(mut self, picture: Picture) -> Self {
    self.pictures.push(picture);
    self
  }

  /// More pictures, after the others.
  pub fn pictures(mut self, pictures: impl IntoIterator<Item = Picture>) -> Self {
    self.pictures.extend(pictures);
    self
  }

  /// Which picture is shown, counted from 0, to read and to set. Without it the lightbox keeps it, and starts
  /// at the first.
  pub fn index(mut self, index: Signal<usize>) -> Self {
    self.index = Some(index);
    self
  }

  /// Which picture is shown, as the app says it each time: the lightbox asks for another with `on_change` and
  /// shows what it is told.
  pub fn index_controlled(mut self, index: usize, on_change: impl Fn(usize) + Send + Sync + 'static) -> Self {
    self.controlled = Some(index);
    self.on_index_change.push(Arc::new(on_change));
    self
  }

  /// Called when another picture is turned to, with the one it is.
  pub fn on_index_change(mut self, handler: impl Fn(usize) + Send + Sync + 'static) -> Self {
    self.on_index_change.push(Arc::new(handler));
    self
  }

  /// How close the picture is, 1 where it fits, to read and to set. Without it the lightbox keeps it.
  pub fn zoom(mut self, zoom: Signal<f32>) -> Self {
    self.zoom = Some(zoom);
    self
  }

  /// Called when the picture is brought closer or further, with how close it is.
  pub fn on_zoom_change(mut self, handler: impl Fn(f64) + Send + Sync + 'static) -> Self {
    self.on_zoom_change.push(Arc::new(handler));
    self
  }

  /// How many pictures each way of the one shown are brought on once it is there, so that an arrow shows the
  /// next at once: 1 unless told, 0 for none.
  pub fn preload(mut self, preload: usize) -> Self {
    self.preload = preload;
    self
  }

  /// The closest the picture is brought, as a multiple of the size it fits at: 4 unless set.
  pub fn zoom_max(mut self, max: f32) -> Self {
    self.zoom_max = max.max(1.0);
    self
  }

  /// Whether the strip of small pictures shows: with more than one picture unless told.
  pub fn filmstrip(mut self, filmstrip: bool) -> Self {
    self.filmstrip = Some(filmstrip);
    self
  }

  /// Whether the counter shows: with more than one picture unless told.
  pub fn counter(mut self, counter: bool) -> Self {
    self.counter = Some(counter);
    self
  }

  /// What the counter says, from which picture is shown, counted from 1, and how many there are: "3 / 12"
  /// unless told.
  pub fn counter_text(mut self, text: impl Fn(usize, usize) -> String + Send + Sync + 'static) -> Self {
    self.counter_text = Some(Arc::new(text));
    self
  }

  /// Whether the first picture comes after the last, and the last before the first.
  pub fn wrap(mut self, wrap: bool) -> Self {
    self.wrap = wrap;
    self
  }

  /// Shows the Download button, and is called with the picture shown when it is pressed.
  pub fn on_download(mut self, handler: impl Fn(usize) + Send + Sync + 'static) -> Self {
    self.on_download.push(Arc::new(handler));
    self
  }

  /// A button of the app's in the top row, before Download.
  pub fn action(mut self, action: impl Into<Element>) -> Self {
    self.actions.push(action.into());
    self
  }

  /// Buttons of the app's in the top row.
  pub fn actions(mut self, actions: impl IntoIterator<Item = impl Into<Element>>) -> Self {
    self.actions.extend(actions.into_iter().map(Into::into));
    self
  }

  /// Stands in the page where it is declared, in place of over the window: open or not, it is there.
  pub fn inline(mut self, inline: bool) -> Self {
    self.inline = inline;
    self
  }

  /// How wide it is where it stands in a page: all the room unless told.
  pub fn width(mut self, width: impl Into<Length>) -> Self {
    self.width = Some(width.into());
    self
  }

  /// How high it is where it stands in a page: all the room unless told.
  pub fn height(mut self, height: impl Into<Length>) -> Self {
    self.height = Some(height.into());
    self
  }

  /// Called when it is dismissed: by Escape, by a press on what is around the picture, or by Close.
  pub fn on_dismiss(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_dismiss.push(Arc::new(handler));
    self
  }

  /// Whether Escape closes it: it does unless told.
  pub fn dismiss_on_escape(mut self, dismiss: bool) -> Self {
    self.dismiss_on_escape = dismiss;
    self
  }

  /// Whether a press on the room around the picture closes it: it does unless told.
  pub fn dismiss_on_veil(mut self, dismiss: bool) -> Self {
    self.dismiss_on_veil = dismiss;
    self
  }

  /// Whether it has a Close button: it has unless told.
  pub fn close_button(mut self, close_button: bool) -> Self {
    self.close_button = close_button;
    self
  }

  /// What it is called to tooling that reads the tree: "Pictures" unless told.
  pub fn label(mut self, label: &str) -> Self {
    self.words.label = Arc::from(label);
    self
  }

  pub fn close_label(mut self, label: &str) -> Self {
    self.words.close = Arc::from(label);
    self
  }

  pub fn previous_label(mut self, label: &str) -> Self {
    self.words.previous = Arc::from(label);
    self
  }

  pub fn next_label(mut self, label: &str) -> Self {
    self.words.next = Arc::from(label);
    self
  }

  pub fn zoom_in_label(mut self, label: &str) -> Self {
    self.words.zoom_in = Arc::from(label);
    self
  }

  pub fn zoom_out_label(mut self, label: &str) -> Self {
    self.words.zoom_out = Arc::from(label);
    self
  }

  pub fn fit_label(mut self, label: &str) -> Self {
    self.words.fit = Arc::from(label);
    self
  }

  pub fn download_label(mut self, label: &str) -> Self {
    self.words.download = Arc::from(label);
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

  /// The lightbox mounted in `ctx`: what converting it into an element does once the render that returns it
  /// has returned.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let key = self.key.clone();
    let props = LightboxProps(self);
    match key {
      Some(key) => ctx.mount_keyed::<LightboxHost>(&key, props),
      None => ctx.mount::<LightboxHost>(props),
    }
  }
}

/// A lightbox in a tree is its component, declared: the component that renders the tree mounts it.
impl From<Lightbox> for Element {
  fn from(lightbox: Lightbox) -> Self {
    let key = lightbox.key.clone();
    let props = LightboxProps(lightbox);
    match key {
      Some(key) => Element::component_keyed::<LightboxHost>(key, props),
      None => Element::component::<LightboxHost>(props),
    }
  }
}

/// A lightbox to mount.
#[derive(Clone)]
struct LightboxProps(Lightbox);

/// Its pictures and its handlers are made anew by every render of what holds it.
impl PartialEq for LightboxProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

#[cfg(feature = "devtools")]
impl crate::app::component::DevtoolsInspectable for LightboxProps {}

/// The component of a lightbox: what it keeps between renders.
struct LightboxHost {
  /// Which picture is shown, where the app names no signal for it.
  index: Signal<usize>,
  /// How close it is, where the app names no signal for it.
  zoom: Signal<f32>,
  /// Where a picture closer than it fits is moved to: the list that scrolls it.
  scroll: ScrollState,
  /// How close the picture was when the app was last told.
  told: Arc<parking_lot::Mutex<f32>>,
  /// What to tell the app once this render is laid out.
  news: Arc<parking_lot::Mutex<Vec<Action>>>,
  /// The stage as it is laid out: where it is in the window, and how large.
  stage: ElementRef,
  /// The stage's size the picture was last laid out for.
  room: Signal<(f32, f32)>,
  /// The picture whose pixels came, by its place.
  came: Signal<Option<usize>>,
  /// Whether it was open at the last render: one that opens again shows its picture fitted.
  was_open: std::sync::atomic::AtomicBool,
  /// The pictures about the one shown brought on ahead, by their places, with the loads held while they are
  /// on their way.
  ahead: parking_lot::Mutex<Vec<(usize, Option<crate::resources::Load>)>>,
  /// Moves on when a load ahead ends: what decodes what it brought.
  ahead_tick: Signal<u64>,
  /// What a load ahead calls when it ends: one waker, which a load keeps once.
  ahead_wake: Arc<dyn Fn() + Send + Sync>,
}

impl Component for LightboxHost {
  type Props = LightboxProps;

  fn create(ctx: &mut Ctx) -> Self {
    // A load ahead that ends renders the lightbox again; one that only came further does not.
    let ahead_tick = ctx.signal(0u64);
    let ahead_wake: Arc<dyn Fn() + Send + Sync> = {
      let tick = ahead_tick.clone();
      Arc::new(move || tick.set(tick.get_untracked().wrapping_add(1)))
    };
    Self {
      index: ctx.signal(0),
      zoom: ctx.signal(1.0),
      scroll: ScrollState::new(),
      told: Arc::new(parking_lot::Mutex::new(1.0)),
      news: Arc::default(),
      stage: ElementRef::new(),
      room: ctx.signal((0.0, 0.0)),
      came: ctx.signal(None),
      was_open: std::sync::atomic::AtomicBool::new(false),
      ahead: parking_lot::Mutex::new(Vec::new()),
      ahead_tick,
      ahead_wake,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let lightbox = ctx.props::<LightboxProps>().0.clone();
    let theme = current_theme();
    let count = lightbox.pictures.len();
    let index_signal = lightbox.index.clone().unwrap_or_else(|| self.index.clone());
    let index = lightbox
      .controlled
      .unwrap_or_else(|| index_signal.get())
      .min(count.saturating_sub(1));
    let zoom_signal = lightbox.zoom.clone().unwrap_or_else(|| self.zoom.clone());
    // Opened again, the picture is fitted, as it is the first time: how close it was is the lightbox's own, and
    // the app's where it gives a signal for it.
    let open = lightbox.inline || lightbox.open.get();
    let opened = open && !self.was_open.swap(open, std::sync::atomic::Ordering::Relaxed);
    if !open {
      self.was_open.store(false, std::sync::atomic::Ordering::Relaxed);
    }
    if opened && lightbox.zoom.is_none() && self.zoom.get_untracked() != 1.0 {
      self.zoom.set(1.0);
      self.scroll.set_scroll(0.0, 0.0);
    }
    let max = lightbox.zoom_max;
    let zoom = zoom_signal.get().clamp(1.0, max);
    let room = self.room.get();

    // The app hears of every change of how close the picture is, whatever made it: a button, a key, the wheel,
    // two fingers. Once the render that shows it is laid out.
    {
      let mut told = self.told.lock();
      if (*told - zoom).abs() > f32::EPSILON {
        *told = zoom;
        let handlers = lightbox.on_zoom_change.clone();
        self.news.lock().push(Arc::new(move || {
          handlers.iter().for_each(|tell| tell(f64::from(zoom)));
        }));
      }
    }
    // Brings the picture to a scale, about the middle of the stage: what a button and a key do. The wheel, a
    // double click and two fingers do it about where they are, in what shows the picture.
    let go: Arc<dyn Fn(f32) + Send + Sync> = {
      let zoom_signal = zoom_signal.clone();
      Arc::new(move |next: f32| {
        let next = next.clamp(1.0, max);
        if (next - zoom_signal.get_untracked()).abs() > f32::EPSILON {
          zoom_signal.set(next);
        }
      })
    };
    // Moves a picture closer than it fits, no further than its edges.
    let shift: Arc<dyn Fn(f32, f32) + Send + Sync> = {
      let scroll = self.scroll.clone();
      Arc::new(move |x: f32, y: f32| scroll.scroll_by(x, y))
    };
    // Turns to the picture `to`: fitted, as every picture is first seen.
    let turn: Told<usize> = {
      let (signal, controlled, told) = (
        index_signal.clone(),
        lightbox.controlled.is_some(),
        lightbox.on_index_change.clone(),
      );
      let (zoom_signal, scroll) = (zoom_signal.clone(), self.scroll.clone());
      Arc::new(move |to: usize| {
        if to >= count || to == index {
          return;
        }
        zoom_signal.set(1.0);
        scroll.set_scroll(0.0, 0.0);
        if !controlled {
          signal.set(to);
        }
        told.iter().for_each(|tell| tell(to));
      })
    };

    // The picture in the room it has: laid out as large as the stage, where it fits whole, and drawn closer
    // from there by what scrolls it (`ZoomScroll`): the wheel about the pointer, two fingers about their
    // middle, a double click, a drag. Until the stage is laid out it has no size to be laid out at.
    self.bring_on(ctx, &lightbox, index);
    let shown = lightbox.pictures.get(index).cloned().map(|picture| {
      let came = self.came.clone();
      let picture: Element = picture
        .in_viewer()
        .on_load(move || {
          if came.get_untracked() != Some(index) {
            came.set(Some(index));
          }
        })
        .key(format!("picture-{index}"))
        .into();
      if room.0 <= 0.0 || room.1 <= 0.0 {
        return picture;
      }
      let props = ZoomScrollProps::new(room.0, room.1)
        .range(1.0, max)
        .wheel_zoom(ZOOM_STEP)
        .drag_pan(true)
        // It pans by a drag and by the arrows: the board draws no bars over the picture's black.
        .scrollbars(false)
        .zoom(zoom_signal.clone())
        .scroll(self.scroll.clone());
      ZoomScroll::mount(ctx, props, picture)
    });

    let case = |current: bool| {
      Case {
        variant: None,
        size: None,
        density: None,
        breakpoint: super::current_breakpoint(),
        states: lightbox.states.clone(),
      }
      .state(LightboxState::Inline, lightbox.inline)
      .state(LightboxState::Current, current)
    };
    let view = LightboxView {
      recipe: theme.resolve::<Lightbox>(&case(false), Some(&lightbox.rules)),
      current: theme.resolve::<Lightbox>(&case(true), Some(&lightbox.rules)),
      index,
      zoom,
      shown,
      go,
      turn,
      stage: self.stage.clone(),
      shift,
      lightbox,
    };
    match theme.renderer::<Lightbox>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }

  /// The picture is laid out as large as the stage, which is known once it is laid out.
  fn after_layout(&self) {
    if !self.stage.is_attached() {
      return;
    }
    let room = (self.stage.width(), self.stage.height());
    let last = self.room.get_untracked();
    if (last.0 - room.0).abs() > 0.25 || (last.1 - room.1).abs() > 0.25 {
      self.room.set(room);
    }
    let news = std::mem::take(&mut *self.news.lock());
    news.iter().for_each(|tell| tell());
  }
}

impl LightboxHost {
  /// Brings on the pictures about the one shown, once its own pixels are there: as many each way as the
  /// lightbox says, round the ends where it wraps. A load that ends renders this again, which decodes what it
  /// brought.
  fn bring_on(&self, ctx: &Ctx, lightbox: &Lightbox, index: usize) {
    let _ = self.ahead_tick.get();
    let count = lightbox.pictures.len();
    let open = lightbox.inline || lightbox.open.get();
    if !open || lightbox.preload == 0 || self.came.get() != Some(index) {
      return;
    }
    let mut places = Vec::new();
    for step in 1..=lightbox.preload.min(count) {
      let after = index + step;
      let before = index.checked_sub(step);
      let after = if after < count {
        Some(after)
      } else {
        lightbox.wrap.then(|| after % count)
      };
      let before = before.or_else(|| lightbox.wrap.then(|| (index + count - step % count) % count));
      for at in [after, before].into_iter().flatten() {
        if at != index && !places.contains(&at) {
          places.push(at);
        }
      }
    }
    #[cfg(feature = "image")]
    {
      let loader = ctx.app_handle().map(|app| app.loader().clone());
      let mut ahead = self.ahead.lock();
      let held = std::mem::take(&mut *ahead);
      for at in places {
        // Asked once: a load that is on its way is held, and decoded here once it has ended.
        let load = match held.iter().find(|(place, _)| *place == at) {
          Some((_, None)) => None,
          Some((_, Some(load))) if load.state().is_loading() => Some(load.clone()),
          _ => lightbox.pictures[at].fetch(loader.as_ref()),
        };
        if let Some(load) = &load {
          load.wake_with(self.ahead_wake.clone());
        }
        ahead.push((at, load));
      }
    }
    #[cfg(not(feature = "image"))]
    let _ = (ctx, places);
  }
}

/// A lightbox as a renderer sees it: what it was given, which picture it shows and how close, and the style of
/// its parts.
pub struct LightboxView {
  /// The style of each part.
  pub recipe: ResolvedRecipe,
  /// The style of each part of the strip's small picture that is the one shown.
  current: ResolvedRecipe,
  /// Which picture is shown, counted from 0.
  pub index: usize,
  /// How close it is: 1 where it fits.
  pub zoom: f32,
  /// The picture shown, in what brings it closer.
  shown: Option<Element>,
  go: Arc<dyn Fn(f32) + Send + Sync>,
  turn: Told<usize>,
  stage: ElementRef,
  shift: Arc<dyn Fn(f32, f32) + Send + Sync>,
  lightbox: Lightbox,
}

impl LightboxView {
  /// The style of a part.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// How many pictures there are.
  pub fn count(&self) -> usize {
    self.lightbox.pictures.len()
  }

  /// Whether the picture is closer than it fits.
  pub fn zoomed(&self) -> bool {
    self.zoom > 1.0 + f32::EPSILON
  }

  fn id(&self, suffix: &str) -> Option<String> {
    self.lightbox.id.as_ref().map(|id| format!("{id}-{suffix}"))
  }

  /// What closes the lightbox and tells the app it was dismissed.
  pub fn dismisser(&self) -> impl Fn() + Send + Sync + Clone + 'static {
    let (open, told) = (self.lightbox.open.clone(), self.lightbox.on_dismiss.clone());
    move || {
      open.set(false);
      told.iter().for_each(|tell| tell());
    }
  }

  /// The picture before the one shown, and the one after, where there is one to turn to.
  fn neighbours(&self) -> (Option<usize>, Option<usize>) {
    let count = self.count();
    if count < 2 {
      return (None, None);
    }
    let wrap = self.lightbox.wrap;
    let before = match self.index {
      0 => wrap.then_some(count - 1),
      index => Some(index - 1),
    };
    let after = match self.index + 1 {
      next if next >= count => wrap.then_some(0),
      next => Some(next),
    };
    (before, after)
  }

  /// A button of the top row: its glyph and its name.
  fn control(&self, icon: &'static str, label: &Arc<str>, id: &str, press: Action) -> Element {
    let mut button = pressable(&self.part(LightboxPart::Control), false)
      .describe("a11y_role", "button")
      .describe("a11y_name", label.clone())
      .child(glyph(&self.part(LightboxPart::ControlIcon), Name::new(icon), false))
      .on_click(move |event: MouseEvent| {
        event.stop_propagation();
        press();
      });
    if let Some(id) = self.id(id) {
      button = button.id(id);
    }
    button.into()
  }

  /// The row along the top: the counter, and the buttons that bring the picture closer and further, fit it,
  /// download it and close the lightbox.
  pub fn top(&self) -> Element {
    let lightbox = &self.lightbox;
    let count = self.count();
    let mut top = Row::new()
      .part(&self.part(LightboxPart::Top))
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Center);
    // How wide the counter is, about: its letters are of one width.
    let mut said = 0.0;
    if lightbox.counter.unwrap_or(count > 1) {
      let part = self.part(LightboxPart::Counter);
      let letters = match &lightbox.counter_text {
        Some(text) => text(self.index + 1, count).chars().count(),
        None => format!("{} / {count}", self.index + 1).chars().count(),
      };
      said = letters as f32 * COUNTER_LETTER + CONTROL_GAP;
      let counter = match &lightbox.counter_text {
        Some(text) => Row::new().child(part.text(&text(self.index + 1, count)).nowrap().part(&part)),
        None => Row::new()
          .spacing(4.0)
          .child(part.text(&(self.index + 1).to_string()).nowrap().part(&part))
          .child({
            let total = self.part(LightboxPart::Total);
            total.text(&format!("/ {count}")).nowrap().part(&total)
          }),
      };
      top = top.child(counter);
    }
    top = top.child(Row::new().flex(1.0));

    let (go, zoom) = (self.go.clone(), self.zoom);
    let closer: Action = {
      let go = go.clone();
      Arc::new(move || go(zoom * ZOOM_STEP))
    };
    let further: Action = {
      let go = go.clone();
      Arc::new(move || go(zoom / ZOOM_STEP))
    };
    let fit: Action = Arc::new(move || go(1.0));
    top = top.child(self.control(icons::ZOOM_OUT, &lightbox.words.zoom_out, "zoom-out", further));
    // In a room too narrow for the whole row the level gives way: the buttons about it bring the picture closer
    // and further still, and a double click and `0` fit it.
    let buttons =
      2 + lightbox.actions.len() + usize::from(!lightbox.on_download.is_empty()) + usize::from(lightbox.close_button);
    let needed = TOP_SIDES + said + buttons as f32 * CONTROL_ROOM + LEVEL_ROOM;
    let room = self.stage.width();
    let level = self.part(LightboxPart::Level);
    let mut fitter = pressable(&level, false)
      .describe("a11y_role", "button")
      .describe("a11y_name", lightbox.words.fit.clone())
      .child(level.text(&format!("{}%", (self.zoom * 100.0).round())).nowrap())
      .on_click(move |event: MouseEvent| {
        event.stop_propagation();
        fit();
      });
    if let Some(id) = self.id("level") {
      fitter = fitter.id(id);
    }
    if room <= 0.0 || room >= needed {
      top = top.child(fitter);
    }
    top = top
      .child(self.control(icons::ZOOM_IN, &lightbox.words.zoom_in, "zoom-in", closer))
      .with_children(lightbox.actions.iter().cloned());
    if !lightbox.on_download.is_empty() {
      let (handlers, index) = (lightbox.on_download.clone(), self.index);
      let download: Action = Arc::new(move || handlers.iter().for_each(|handler| handler(index)));
      top = top.child(self.control(icons::DOWNLOAD, &lightbox.words.download, "download", download));
    }
    if lightbox.close_button {
      let close = self.dismisser();
      top = top.child(self.control(icons::CLOSE, &lightbox.words.close, "close", Arc::new(close)));
    }
    top.into()
  }

  /// A round button over a side of the stage, which turns to the picture `to`.
  fn step(&self, icon: &'static str, label: &Arc<str>, id: &str, to: usize) -> Element {
    let turn = self.turn.clone();
    let mut button = pressable(&self.part(LightboxPart::Step), false)
      .describe("a11y_role", "button")
      .describe("a11y_name", label.clone())
      .child(glyph(&self.part(LightboxPart::StepIcon), Name::new(icon), false))
      .on_click(move |event: MouseEvent| {
        event.stop_propagation();
        turn(to);
      });
    if let Some(id) = self.id(id) {
      button = button.id(id);
    }
    button.into()
  }

  /// The picture's title and description over its lower edge, where it has either and is not closer than it
  /// fits.
  pub fn caption(&self) -> Option<Element> {
    let (title, description) = self.lightbox.pictures.get(self.index)?.caption_of();
    if self.zoomed() || (title.is_none() && description.is_none()) {
      return None;
    }
    let part = self.part(LightboxPart::Caption);
    let end = part
      .rest()
      .background
      .clone()
      .unwrap_or(crate::node::BackgroundColor::Color(Color::new(0, 0, 0, 0xB3)));
    let clear = crate::node::BackgroundColor::Color(Color::new(0, 0, 0, 0));
    let mut caption = Column::new()
      .part(&part)
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Center)
      .background_gradient(crate::node::Gradient::linear(
        180.0,
        [(clear, 0.0), (end, 1.0)].map(|(color, at)| crate::node::gradient::GradientStop::at(color, at)),
      ));
    for (text, part) in [(title, LightboxPart::Title), (description, LightboxPart::Description)] {
      if let Some(text) = text {
        let part = self.part(part);
        caption = caption.child(
          part
            .text(&text)
            .text_align(TextAlign::Center)
            .width(Dimension::Pct(100.0))
            .part(&part),
        );
      }
    }
    Some(caption.into())
  }

  /// The room the picture is shown in: the picture, its caption over its lower edge, and the round buttons
  /// over its sides. The wheel brings the picture closer about the pointer, and a drag moves it.
  pub fn stage(&mut self) -> Element {
    let full = Dimension::Pct(100.0);
    let layer = || Stack::new().size(full, full).hit_test(HitTestBehavior::ContentOnly);
    // The stage is what the keys act on, and what takes the focus as the lightbox opens: the arrows, `+`, `-`, `0`
    // and Escape work at once, and Tab goes on to the buttons. Its ring shows where the keyboard put the focus.
    let name = self
      .lightbox
      .pictures
      .get(self.index)
      .and_then(Picture::label_of)
      .unwrap_or_else(|| Arc::from(format!("{} / {}", self.index + 1, self.count())));
    let mut room = Stack::new()
      .part(&self.part(LightboxPart::Stage))
      .width(full)
      .flex(1.0)
      .clip()
      .tab_index(0)
      .describe("a11y_role", "group")
      .describe("a11y_name", name)
      .ref_element(self.stage.clone());
    if let Some(shown) = self.shown.take() {
      room = room.child(shown);
    }
    if let Some(id) = self.id("stage") {
      room = room.id(id);
    }
    if let Some(caption) = self.caption() {
      // The caption is read, not pressed: the wheel and a drag over it are the picture's under it.
      room = room.child(
        layer()
          .stack_align(StackAlignment::BottomCenter)
          .pointer_events_none()
          .child(caption),
      );
    }
    let (before, after) = self.neighbours();
    if let Some(before) = before {
      let words = self.lightbox.words.previous.clone();
      room = room.child(
        layer()
          .stack_align(StackAlignment::CenterStart)
          .part(&self.part(LightboxPart::Side))
          .child(self.step(icons::CHEVRON_LEFT, &words, "previous", before)),
      );
    }
    if let Some(after) = after {
      let words = self.lightbox.words.next.clone();
      room = room.child(
        layer()
          .stack_align(StackAlignment::CenterEnd)
          .part(&self.part(LightboxPart::Side))
          .child(self.step(icons::CHEVRON_RIGHT, &words, "next", after)),
      );
    }
    let mut room: Element = room.into();
    if !self.lightbox.inline {
      NodeUpdate::autofocus(&mut *room.node, true);
    }
    room
  }

  /// The strip of small pictures along the bottom, the one shown marked, where it shows.
  pub fn filmstrip(&self) -> Option<Element> {
    let count = self.count();
    if !self.lightbox.filmstrip.unwrap_or(count > 1) {
      return None;
    }
    let mut strip = Row::new()
      .part(&self.part(LightboxPart::Filmstrip))
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Center)
      .justify(Justify::Center);
    // The strip shows as many as it has room for, about the one shown: a strip of a hundred pictures is not
    // cut at both ends with the one shown out of sight. Until the lightbox is laid out it shows them all.
    let (first, last) = {
      let (width, _) = thumb_size(&self.part(LightboxPart::Thumb));
      let room = self.stage.width();
      let fits = match room > 0.0 {
        true => (((room - 2.0 * STRIP_INSET + STRIP_GAP) / (width + STRIP_GAP)).floor() as usize).max(1),
        false => count,
      };
      let first = self.index.saturating_sub(fits / 2).min(count.saturating_sub(fits));
      (first, (first + fits).min(count))
    };
    for (at, picture) in self.lightbox.pictures.iter().enumerate().take(last).skip(first) {
      let recipe = if at == self.index { &self.current } else { &self.recipe };
      let part = recipe.part(LightboxPart::Thumb);
      let (width, height) = thumb_size(&part);
      let turn = self.turn.clone();
      let name = picture
        .label_of()
        .unwrap_or_else(|| Arc::from(format!("{} / {count}", at + 1)));
      let mut thumb = pressable(&part, false)
        .describe("a11y_role", "button")
        .describe("a11y_name", name)
        .describe("a11y_current", if at == self.index { "true" } else { "false" })
        .clip()
        .child(picture.clone().as_thumb(width, height).key(format!("thumb-{at}")))
        .on_click(move |event: MouseEvent| {
          event.stop_propagation();
          turn(at);
        });
      if let Some(id) = self.id(&format!("thumb-{at}")) {
        thumb = thumb.id(id);
      }
      strip = strip.child(thumb);
    }
    Some(strip.into())
  }

  /// The lightbox itself: its top row, its stage and its strip, with its keys.
  pub fn panel(&mut self) -> Element {
    let lightbox = &self.lightbox;
    let theme = self.theme();
    let (go, turn, shift) = (self.go.clone(), self.turn.clone(), self.shift.clone());
    let (zoom, zoomed) = (self.zoom, self.zoomed());
    let (before, after) = self.neighbours();
    let mut panel = Column::new()
      .part(&self.part(LightboxPart::Root))
      .clip()
      .describe("a11y_role", "dialog")
      .describe("a11y_name", lightbox.words.label.clone())
      // A press in the lightbox is not one on what is around it.
      .on_click(|event: MouseEvent| event.stop_propagation())
      .on_key_down(move |event: KeyboardEvent| {
        // A key is told to every element: these are the lightbox's while the focus is in it.
        if !event.focus_within || event.ctrl || event.alt || event.meta {
          return;
        }
        let handled = match event.key.as_str() {
          "+" | "=" => {
            go(zoom * ZOOM_STEP);
            true
          }
          "-" | "_" => {
            go(zoom / ZOOM_STEP);
            true
          }
          "0" => {
            go(1.0);
            true
          }
          // Closer than it fits, the arrows move the picture; fitted, they turn to another.
          "ArrowLeft" if zoomed => {
            shift(-PAN_STEP, 0.0);
            true
          }
          "ArrowRight" if zoomed => {
            shift(PAN_STEP, 0.0);
            true
          }
          "ArrowUp" if zoomed => {
            shift(0.0, -PAN_STEP);
            true
          }
          "ArrowDown" if zoomed => {
            shift(0.0, PAN_STEP);
            true
          }
          "ArrowLeft" => before.map(|to| turn(to)).is_some(),
          "ArrowRight" => after.map(|to| turn(to)).is_some(),
          _ => false,
        };
        if handled {
          event.prevent_default();
          event.stop_propagation();
        }
      });
    let full = Dimension::Pct(100.0);
    let (width, height) = match lightbox.inline {
      true => (
        lightbox.width.and_then(|width| width.resolve(theme)).unwrap_or(full),
        lightbox.height.and_then(|height| height.resolve(theme)).unwrap_or(full),
      ),
      false => (full, full),
    };
    // The width said is the most the panel takes: in a narrower room it is as wide as the room.
    panel = panel.size(width, height).max_width(full);
    if let Some(id) = &lightbox.id {
      panel = panel.id(id.clone());
    }
    if let (Some(key), true) = (&lightbox.key, lightbox.inline) {
      panel = panel.key(key.clone());
    }
    let top = self.top();
    let stage = self.stage();
    panel = panel.child(top).child(stage);
    if let Some(strip) = self.filmstrip() {
      panel = panel.child(strip);
    }
    panel.into()
  }

  /// How the default renderer builds a lightbox: its panel where it stands in a page, or over the window, on
  /// what covers it, while it is open.
  pub fn render(mut self) -> Element {
    let panel = self.panel();
    if self.lightbox.inline {
      return panel;
    }
    let mut veil = Column::new()
      .size(Dimension::Pct(100.0), Dimension::Pct(100.0))
      .part(&self.part(LightboxPart::Veil))
      .child(panel);
    if self.lightbox.dismiss_on_veil {
      let close = self.dismisser();
      veil = veil.on_click(move |_: MouseEvent| close());
    }
    if let Some(key) = &self.lightbox.key {
      veil = veil.key(key.clone());
    }
    let mut modal = Modal::new(veil)
      .open(self.lightbox.open.clone())
      .target(Root)
      .dismiss_on_escape(self.lightbox.dismiss_on_escape)
      .takes_focus(true);
    for handler in &self.lightbox.on_dismiss {
      let handler = handler.clone();
      modal = modal.on_dismiss(move || handler());
    }
    modal.into()
  }
}

/// How large a small picture of the strip is, from the style of its part.
fn thumb_size(part: &ResolvedPart) -> (f32, f32) {
  let px = |length: Option<Length>, or: f32| match length.and_then(|length| length.resolve(part.theme())) {
    Some(Dimension::Px(px)) => px,
    _ => or,
  };
  let rest = part.rest();
  (px(rest.width, 44.0), px(rest.height, 32.0))
}

impl Themed for Lightbox {
  const NAME: &'static str = "lightbox";
  type Renderer = LightboxRenderer;

  /// The design's lightbox (`pencil/lurq-design-system.pen`, Lightbox): near black at 95% in both themes, and
  /// white on it. A top row of 52, 16 before the counter and 8 around the buttons, 4 between them: the counter
  /// in mono 12, how many at 60% white; buttons of 36 with glyphs of 20, the pointer's fill on them at 12%
  /// white; the level a button of 56 by 36 with 13 medium. The stage takes the rest. Over its sides, 12 in,
  /// round buttons of 36 on 40% black with a chevron of 20. The caption lies over the picture's lower edge on
  /// a gradient to 70% black: 20 above, 16 beside and 8 under a title of 14 medium and a line of 13 at 60%
  /// white, 2 apart. The strip is 52 high, its pictures 44 by 32 with small corners, 6 apart, at 60%; the one
  /// shown at full, with a line of 2 in white inside it. In a page it has the corners of a surface.
  fn recipe() -> Recipe {
    use LightboxPart::*;
    use LightboxState::{Current, Inline};
    let white = Color::new(255, 255, 255, 255);
    let white_60 = Color::new(255, 255, 255, 0x99);
    let white_12 = Color::new(255, 255, 255, 0x1F);
    let black_40 = Color::new(0, 0, 0, 0x66);
    let black_60 = Color::new(0, 0, 0, 0x99);
    let black_70 = Color::new(0, 0, 0, 0xB3);
    Recipe::new()
      .base(Root, |s| s.background(Color::new(0x09, 0x09, 0x0B, 0xF2)))
      .state(Inline, Root, |s| s.radius(RadiusSize::Lg))
      .base(Top, |s| {
        s.height(52.0)
          .padding_top(8.0)
          .padding_bottom(8.0)
          .padding_start(16.0)
          .padding_end(8.0)
          .gap(4.0)
      })
      .base(Counter, |s| {
        s.typography(TypographyStyle::Caption)
          .font_family(FontFamily::Mono)
          .font_size(12.0)
          .foreground(white)
      })
      .base(Total, |s| {
        s.typography(TypographyStyle::Caption)
          .font_family(FontFamily::Mono)
          .font_size(12.0)
          .foreground(white_60)
      })
      .base(Control, |s| {
        fading(
          s.square(36.0)
            .radius(RadiusSize::Md)
            .foreground(white)
            .cursor(CursorIcon::Pointer),
        )
      })
      .hovered(Control, |s| s.background(white_12))
      .focus_visible(Control, |s| s.outline(Outline::new(2.0, white)))
      // Inside the stage, which reaches the panel's edges.
      .focus_visible(Stage, |s| s.outline(Outline::new(2.0, white).offset(-2.0)))
      .base(ControlIcon, |s| s.square(20.0))
      .base(Level, |s| {
        fading(
          s.width(56.0)
            .height(36.0)
            .radius(RadiusSize::Md)
            .typography(TypographyStyle::Body)
            .font_size(13.0)
            .weight(FontWeight::Medium)
            .foreground(white)
            .cursor(CursorIcon::Pointer),
        )
      })
      .hovered(Level, |s| s.background(white_12))
      .focus_visible(Level, |s| s.outline(Outline::new(2.0, white)))
      .base(Step, |s| {
        fading(
          s.square(36.0)
            .radius(RadiusSize::Full)
            .background(black_40)
            .foreground(white)
            .cursor(CursorIcon::Pointer),
        )
      })
      .hovered(Step, |s| s.background(black_60))
      .focus_visible(Step, |s| s.outline(Outline::new(2.0, white)))
      .base(StepIcon, |s| s.square(20.0))
      .base(Side, |s| s.padding_x(12.0))
      .base(Caption, |s| {
        s.background(black_70)
          .padding_top(20.0)
          .padding_bottom(8.0)
          .padding_x(16.0)
          .gap(2.0)
      })
      .base(Title, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(14.0)
          .weight(FontWeight::Medium)
          .foreground(white)
      })
      .base(Description, |s| {
        s.typography(TypographyStyle::Body).font_size(13.0).foreground(white_60)
      })
      .base(Filmstrip, |s| s.height(52.0).gap(6.0))
      .base(Thumb, |s| {
        s.width(44.0)
          .height(32.0)
          .radius(RadiusSize::Sm)
          .opacity(0.6)
          .cursor(CursorIcon::Pointer)
          .transition(PartTransition::new(Transition::opacity(), MotionDuration::Fast))
      })
      .hovered(Thumb, |s| s.opacity(1.0))
      .focus_visible(Thumb, |s| s.outline(Outline::new(2.0, white)))
      .state(Current, Thumb, |s| s.opacity(1.0).border(Border::inside(2.0, white)))
  }
}
