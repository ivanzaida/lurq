//! A pager: pages one at a time, each as large as the pager's room, of which only the one it is at and the ones
//! beside it are built. A feed of videos that goes down a video at a time, and a strip of an album's pictures
//! that goes across in a page of it.
//!
//! Which page it is at is an index, and the index is the truth: the app holds it in a signal, the pager shows
//! the page it says, and a person's move changes it once the move is over and the page is at rest.
//!
//! ```no_run
//! use lurq::{
//!   app::ctx::Ctx,
//!   components::{Column, Pager},
//!   core::Signal,
//!   node::Element,
//! };
//!
//! fn feed(ctx: &mut Ctx, clips: usize, at: Signal<usize>) -> Element {
//!   Pager::new(clips, |_ctx, index| Column::new().id(format!("clip-{index}")))
//!     .index(at)
//!     .on_index_change(|index| println!("at clip {index}"))
//!     .mount(ctx)
//! }
//! ```
//!
//! - **A page is a component of its own**, made when it comes within a page of the one shown and dropped when it is
//!   further: what a page mounts (`ctx.mount`), and what it keeps, is its own and lives as long as it is built. A feed
//!   of a thousand videos holds three.
//! - **A finger** drags the pages and lets go: the pager goes on to the page beside or back, never further than one
//!   page for one gesture, along its own way alone (a strip across in a feed that goes down takes the finger that first
//!   went across, and the feed the one that first went down).
//! - **A wheel** turns a page for a notch, and a touchpad's swipe turns one.
//! - **The arrow keys** turn a page along the pager's way; Page Up, Page Down, Home and End turn one that goes down.
//!   Where the focus is in the pager, and also where it is not while most of the pager shows, unless a text input has
//!   the focus.

use std::sync::{
  Arc,
  atomic::{AtomicBool, Ordering},
};

use parking_lot::Mutex;

use crate::{
  app::{
    component::Component,
    ctx::Ctx,
    events::{KeyboardEvent, ScrollEvent, ScrollSource},
  },
  components::{Column, Row, ScrollHorizontal, ScrollVertical},
  core::{ElementRef, Signal, Visibility, VisibilityObserver, VisibilityOptions},
  layout::{layout_kind::ScrollState, scrollbar::ScrollBarStyle},
  node::{Element, NodeUpdate, dimension::Dimension},
  time::{Duration, Instant},
};

/// What builds a page.
type Page = Arc<dyn Fn(&mut Ctx, usize) -> Element + Send + Sync>;
/// What is told the page it came to.
type Told = Arc<dyn Fn(usize) + Send + Sync>;

/// A wheel's events closer together than this are one turn of it: a touchpad's swipe, and what it leaves
/// rolling.
const ONE_SWIPE: Duration = Duration::from_millis(140);
/// A wheel that goes on turning turns a page again after this long.
const TURNS_AGAIN_AFTER: Duration = Duration::from_millis(650);
/// And only by a step this long: a notch of a mouse's wheel, and not the tail of a touchpad's swipe.
const A_NOTCH: f32 = 30.0;

/// A pager: see the [module](self).
#[derive(Clone)]
pub struct Pager {
  count: usize,
  page: Page,
  index: Option<Signal<usize>>,
  told: Vec<Told>,
  horizontal: bool,
  beside: usize,
  keys: bool,
  width: Dimension,
  height: Dimension,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Pager {
  /// A pager of `count` pages that goes down, where `page` builds the page at an index: called for the page
  /// shown and the ones beside it, in a component of the page's own.
  pub fn new<E: Into<Element>>(count: usize, page: impl Fn(&mut Ctx, usize) -> E + Send + Sync + 'static) -> Self {
    Self {
      count,
      page: Arc::new(move |ctx, index| page(ctx, index).into()),
      index: None,
      told: Vec::new(),
      horizontal: false,
      beside: 1,
      keys: true,
      width: Dimension::Pct(100.0),
      height: Dimension::Pct(100.0),
      id: None,
      key: None,
    }
  }

  /// The page it is at, held by the app: the pager shows the page `index` says and goes there when it changes
  /// (over a few frames to a page that is built, at once to one further away), and sets it to the page a
  /// person brought it to, once it is at rest there. A pager given none starts at its first page and keeps
  /// where it is itself.
  pub fn index(mut self, index: Signal<usize>) -> Self {
    self.index = Some(index);
    self
  }

  /// Called with the page a person brought the pager to (a finger, a wheel, a key), once it is at rest there
  /// and after its `index` was set. Not called for a page the app set itself. Each call adds a handler.
  pub fn on_index_change(mut self, handler: impl Fn(usize) + Send + Sync + 'static) -> Self {
    self.told.push(Arc::new(handler));
    self
  }

  /// Pages one under another, which is what it does unless told.
  pub fn vertical(mut self) -> Self {
    self.horizontal = false;
    self
  }

  /// Pages one beside another.
  pub fn horizontal(mut self) -> Self {
    self.horizontal = true;
    self
  }

  /// How many pages are built on each side of the one shown: 1 unless told, and never none, since the page a
  /// finger drags in must be there. More keeps what is further away alive at the cost of what it holds.
  pub fn beside(mut self, pages: usize) -> Self {
    self.beside = pages.max(1);
    self
  }

  /// Whether the keys turn it: they do unless told.
  pub fn keys(mut self, keys: bool) -> Self {
    self.keys = keys;
    self
  }

  /// How wide it is: all of its room unless told.
  pub fn width(mut self, width: impl Into<Dimension>) -> Self {
    self.width = width.into();
    self
  }

  /// How high it is: all of its room unless told.
  pub fn height(mut self, height: impl Into<Dimension>) -> Self {
    self.height = height.into();
    self
  }

  /// The id of its box. The list the pages scroll in is `{id}-pages`, and a page `{id}-page-{index}`.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// What tells it from the pagers beside it, where what holds it builds several in turn.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Mounts it in `ctx`.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let key = self.key.clone();
    let props = PagerProps(self);
    match key {
      Some(key) => ctx.mount_keyed::<PagerHost>(&key, props),
      None => ctx.mount::<PagerHost>(props),
    }
  }
}

/// A pager in a tree is its component, declared: the component that renders the tree mounts it.
impl From<Pager> for Element {
  fn from(pager: Pager) -> Self {
    let key = pager.key.clone();
    let props = PagerProps(pager);
    match key {
      Some(key) => Element::component_keyed::<PagerHost>(key, props),
      None => Element::component::<PagerHost>(props),
    }
  }
}

/// A pager to mount.
#[derive(Clone)]
struct PagerProps(Pager);

/// What builds its pages and its handlers are made anew by every render of what holds it.
impl PartialEq for PagerProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

impl crate::app::component::DevtoolsInspectable for PagerProps {}

/// A wheel's turns, as the pager counts them.
#[derive(Default)]
struct Wheel {
  /// When it last heard the wheel, and when the wheel last turned a page.
  heard: Option<Instant>,
  turned: Option<Instant>,
}

impl Wheel {
  /// Whether a wheel's step of `delta` at `now` turns a page: the first of a swipe does, and a wheel that
  /// goes on turning does again after a while.
  fn turns(&mut self, delta: f32, now: Instant) -> bool {
    let new = self.heard.is_none_or(|heard| now.duration_since(heard) > ONE_SWIPE);
    let again = delta.abs() >= A_NOTCH
      && self
        .turned
        .is_none_or(|turned| now.duration_since(turned) > TURNS_AGAIN_AFTER);
    self.heard = Some(now);
    if new || again {
      self.turned = Some(now);
    }
    new || again
  }
}

/// The component of a pager: what it keeps between renders.
struct PagerHost {
  /// Which page it is at, where the app names no signal for it.
  index: Signal<usize>,
  /// The list the pages scroll in, a page at a time.
  scroll: ScrollState,
  /// Its box as it is laid out, and how large that was: a page is as large.
  root: ElementRef,
  wide: Signal<f32>,
  high: Signal<f32>,
  /// The page nearest where the pages are now, which is not the index while they move: its neighbours are
  /// built too.
  near: Signal<usize>,
  /// The index the pages were last put at, and how long a page was then. Nothing until they are first put.
  placed: Arc<Mutex<Option<(usize, f32)>>>,
  /// The page a wheel or a key asked for, which the next render sends the pages to.
  asked: Arc<Mutex<Option<usize>>>,
  again: Signal<u64>,
  wheel: Arc<Mutex<Wheel>>,
  /// Whether most of it shows: its keys are its own then, with the focus elsewhere.
  shows: Arc<AtomicBool>,
  _watching: Arc<VisibilityObserver>,
}

impl Component for PagerHost {
  type Props = PagerProps;

  fn create(ctx: &mut Ctx) -> Self {
    let scroll = ScrollState::new();
    scroll.set_paging(true);
    let root = ElementRef::new();
    let shows = Arc::new(AtomicBool::new(false));
    let watching = root.observe_visibility(VisibilityOptions::new().thresholds([0.5]), {
      let shows = shows.clone();
      move |seen: Visibility| shows.store(seen.ratio >= 0.5, Ordering::Relaxed)
    });
    Self {
      index: ctx.signal(0),
      scroll,
      root,
      wide: ctx.signal(0.0),
      high: ctx.signal(0.0),
      near: ctx.signal(0),
      placed: Arc::default(),
      asked: Arc::default(),
      again: ctx.signal(0),
      wheel: Arc::default(),
      shows,
      _watching: Arc::new(watching),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let pager = ctx.props::<PagerProps>().0.clone();
    let horizontal = pager.horizontal;
    let count = pager.count;
    let last = count.saturating_sub(1);
    let index_signal = pager.index.clone().unwrap_or_else(|| self.index.clone());
    let index = index_signal.get().min(last);
    let _ = self.again.get();
    let (wide, high) = (self.wide.get(), self.high.get());
    let length = if horizontal { wide } else { high };
    let laid = length > 0.0;
    // Until the pages were first put where the index says, the page nearest their offset is not the one shown.
    let near = if self.placed.lock().is_some() {
      self.near.get().min(last)
    } else {
      let _ = self.near.get();
      index
    };
    let along = move |page: (usize, usize)| if horizontal { page.0 } else { page.1 };

    // The pages go where the index says: at once the first time and when the room changed; over a few frames
    // when the app set it to a page that is built, and at once to one further away, past pages that are not
    // there to be seen; and not at all when a person brought them there. A wheel and a key ask for a page, and
    // the pages go there the same way: the index follows once they are at rest.
    let asked = self.asked.lock().take();
    // The page the pages are sent to by this render, which is built before they come to it.
    let mut aim = index;
    if laid && count > 0 {
      let offset = |page: usize| {
        let to = page as f32 * length;
        if horizontal { (to, 0.0) } else { (0.0, to) }
      };
      let mut placed = self.placed.lock();
      match *placed {
        Some((seen, was)) if (was - length).abs() < 0.25 => {
          let going = along(self.scroll.target_page()).min(last);
          aim = going;
          let to = if seen != index { Some(index) } else { asked };
          if let Some(to) = to.map(|to| to.min(last)).filter(|to| *to != going) {
            let (x, y) = offset(to);
            if to.abs_diff(going) <= pager.beside {
              ctx.scroll_state_to(&self.scroll, x, y);
            } else {
              self.scroll.set_scroll_pending(x, y);
            }
            aim = to;
          }
        }
        _ => {
          let (x, y) = offset(index);
          self.scroll.set_scroll_pending(x, y);
          if self.near.get_untracked() != index {
            self.near.set(index);
          }
        }
      }
      *placed = Some((index, length));
    }

    // The page a person brought it to: the index is set, and then the app is told.
    let tell = {
      let (signal, told) = (index_signal.clone(), pager.told.clone());
      move |to: usize| {
        if signal.get_untracked() != to {
          signal.set(to);
          told.iter().for_each(|tell| tell(to));
        }
      }
    };
    // What a wheel and a key do: ask for the page beside the one the pages are at, or going to.
    let ask = {
      let (asked, again) = (self.asked.clone(), self.again.clone());
      move |to: usize| {
        *asked.lock() = Some(to);
        again.set(again.get_untracked().wrapping_add(1));
      }
    };

    // The pages built: the one shown, the one nearest where a finger has the pages, the one they are sent to,
    // and the ones beside each. Before its room is known the one shown alone, which is then all there is to see.
    let mut built: Vec<usize> = Vec::new();
    if count > 0 {
      let centers = if laid { vec![index, near, aim] } else { vec![index] };
      let beside = if laid { pager.beside } else { 0 };
      for center in centers {
        for page in center.saturating_sub(beside)..=(center + beside).min(last) {
          if !built.contains(&page) {
            built.push(page);
          }
        }
      }
      built.sort_unstable();
    }
    let full = Dimension::Pct(100.0);
    // The room of the pages that are not built, `pages` of them.
    let gap = |pages: usize| {
      let long = pages as f32 * length;
      if horizontal {
        Column::new().width(long).height(high).flex_shrink(0.0)
      } else {
        Column::new().width(wide).height(long).flex_shrink(0.0)
      }
    };
    let mut slots: Vec<Element> = Vec::new();
    let mut next = 0;
    for page in built {
      if laid && page > next {
        slots.push(gap(page - next).into());
      }
      let made = ctx.mount_keyed::<PageHost>(
        &format!("page-{page}"),
        PageProps {
          index: page,
          page: pager.page.clone(),
        },
      );
      let mut slot = Column::new()
        .key(format!("page-{page}"))
        .flex_shrink(0.0)
        .clip()
        .child(made);
      slot = if laid {
        slot.width(wide).height(high)
      } else {
        slot.width(full).height(full)
      };
      if let Some(id) = &pager.id {
        slot = slot.id(format!("{id}-page-{page}"));
      }
      let mut slot: Element = slot.into();
      // Only the page shown takes the pointer and the focus: a Tab does not go into a page that is not there.
      if page != index {
        NodeUpdate::inert(&mut *slot.node, true);
      }
      slots.push(slot);
      next = page + 1;
    }
    if laid && next < count {
      slots.push(gap(count - next).into());
    }

    let scrolled = {
      let (scroll, near, wheel, ask) = (self.scroll.clone(), self.near.clone(), self.wheel.clone(), ask.clone());
      move |event: ScrollEvent| {
        let turned = if horizontal { event.delta_x } else { event.delta_y };
        if (event.delta_x, event.delta_y) == (0.0, 0.0) {
          // A finger moved the pages, or what it left did: the page nearest them, whose neighbours are built.
          // Pages that are sent somewhere pass the ones on the way, which are not made for that.
          if matches!(
            event.source,
            ScrollSource::Finger | ScrollSource::Momentum | ScrollSource::Snap
          ) {
            let page = along(scroll.page()).min(last);
            if near.get_untracked() != page {
              near.set(page);
            }
          }
          return;
        }
        if event.source != ScrollSource::Wheel {
          return;
        }
        // A wheel does not scroll the pages by what it turned: it turns a page.
        event.prevent_default();
        if turned == 0.0 || !wheel.lock().turns(turned, Instant::now()) {
          return;
        }
        let going = along(scroll.target_page());
        let to = if turned < 0.0 {
          going + 1
        } else {
          going.saturating_sub(1)
        };
        if to <= last && to != going {
          ask(to);
        }
      }
    };
    let paged = {
      let near = self.near.clone();
      move |page: usize| {
        let page = page.min(last);
        if near.get_untracked() != page {
          near.set(page);
        }
        tell(page);
      }
    };
    let list: Element = if horizontal {
      Row::new().with_children(slots).into()
    } else {
      Column::new().with_children(slots).into()
    };
    let pages_id = pager.id.as_ref().map(|id| format!("{id}-pages"));
    macro_rules! pages {
      ($list:expr) => {{
        let mut pages = $list
          .with_scroll_state(self.scroll.clone())
          .scrollbar(ScrollBarStyle::hidden())
          .on_scroll(scrolled)
          .on_page_change(paged)
          .width(full)
          .height(full);
        if let Some(id) = pages_id {
          pages = pages.id(id);
        }
        Element::from(pages)
      }};
    }
    let pages = if horizontal {
      pages!(ScrollHorizontal::new(list))
    } else {
      pages!(ScrollVertical::new(list))
    };

    let mut root = Column::new()
      .width(pager.width)
      .height(pager.height)
      .min_width(0.0)
      .min_height(0.0)
      .clip()
      .ref_element(self.root.clone())
      .child(pages);
    if pager.keys && count > 1 {
      let (scroll, shows) = (self.scroll.clone(), self.shows.clone());
      root = root.on_key_down(move |event: KeyboardEvent| {
        if event.text_input_focused || event.composing || event.ctrl || event.alt || event.meta {
          return;
        }
        // A key is told to every element: these are the pager's where the focus is in it, and where most of it
        // shows, which a strip in a page that is not shown does not.
        if !(event.focus_within || shows.load(Ordering::Relaxed)) {
          return;
        }
        let going = along(scroll.target_page());
        let to = match (event.key.as_str(), horizontal) {
          ("ArrowDown" | "PageDown", false) | ("ArrowRight", true) => going + 1,
          ("ArrowUp" | "PageUp", false) | ("ArrowLeft", true) => match going.checked_sub(1) {
            Some(to) => to,
            None => return,
          },
          ("Home", false) => 0,
          ("End", false) => last,
          _ => return,
        };
        if to > last || to == going {
          return;
        }
        event.prevent_default();
        event.stop_propagation();
        ask(to);
      });
    }
    if let Some(id) = &pager.id {
      root = root.id(id.clone());
    }
    root
  }

  /// A page is as large as the pager's box, which is known once it is laid out.
  fn after_layout(&self) {
    if !self.root.is_attached() {
      return;
    }
    let (wide, high) = (self.root.width(), self.root.height());
    if (self.wide.get_untracked() - wide).abs() > 0.25 {
      self.wide.set(wide);
    }
    if (self.high.get_untracked() - high).abs() > 0.25 {
      self.high.set(high);
    }
  }
}

/// A page to mount: which one, and what builds it.
struct PageProps {
  index: usize,
  page: Page,
}

/// What builds it is made anew by every render of what holds the pager.
impl PartialEq for PageProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

impl crate::app::component::DevtoolsInspectable for PageProps {}

/// The component of one page: what the page mounts and keeps is in it, and goes with it.
struct PageHost;

impl Component for PageHost {
  type Props = PageProps;

  fn create(_: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let (index, page) = {
      let props = ctx.props::<PageProps>();
      (props.index, props.page.clone())
    };
    page(ctx, index)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_swipe_of_a_touchpad_turns_one_page_and_a_wheel_that_goes_on_turns_again() {
    let start = Instant::now();
    let at = |millis: u64| start + Duration::from_millis(millis);
    let mut wheel = Wheel::default();
    // A swipe: many small steps close together, and its tail.
    assert!(wheel.turns(-4.0, at(0)));
    for step in 1..60 {
      assert!(!wheel.turns(-12.0, at(step * 16)), "step {step} of one swipe");
    }
    // Another swipe, after a rest.
    assert!(wheel.turns(-4.0, at(60 * 16 + 300)));

    // A mouse's wheel turned without a rest: a page for the first notch, and one more after a while.
    let mut wheel = Wheel::default();
    assert!(wheel.turns(-120.0, at(0)));
    for millis in (100..=600).step_by(100) {
      assert!(!wheel.turns(-120.0, at(millis)), "the notch at {millis} ms");
    }
    assert!(wheel.turns(-120.0, at(700)));
  }
}
