use std::sync::{
  Arc, RwLock,
  atomic::{AtomicU64, Ordering},
};

use crate::node::dimension::Dimension;

static OVERRIDE_CLEAR_GENERATION: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Default)]
pub struct ElementRef {
  inner: Arc<RwLock<ElementRefInner>>,
}

#[derive(Clone, Default)]
pub struct ElementRefMut {
  inner: Arc<RwLock<ElementRefInner>>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ElementRect {
  pub x: f32,
  pub y: f32,
  pub relative_x: f32,
  pub relative_y: f32,
  pub width: f32,
  pub height: f32,
}

impl ElementRect {
  pub fn center(&self) -> (f32, f32) {
    (self.x + self.width / 2.0, self.y + self.height / 2.0)
  }
}

#[derive(Default)]
struct ElementRefInner {
  #[cfg(feature = "canvas")]
  canvas: Option<crate::canvas::CanvasWeak>,
  x: f32,
  y: f32,
  relative_x: f32,
  relative_y: f32,
  width: f32,
  height: f32,
  attached: bool,
  /// Whether the element is a text drawn in fewer letters than it has, as the last layout drew it
  /// ([`ElementRef::text_cut`]).
  text_cut: bool,
  hovered: bool,
  active: bool,
  focused: bool,
  focus_signal: Option<crate::core::Signal<bool>>,
  focus_visible: bool,
  focus_visible_signal: Option<crate::core::Signal<bool>>,
  focus_visible_within: bool,
  focus_visible_within_signal: Option<crate::core::Signal<bool>>,
  override_rect: Option<ElementRect>,
  /// The width and the height the element is laid out at in place of those it was built with
  /// ([`ElementRefMut::set_size`]).
  size_override: (Option<Dimension>, Option<Dimension>),
  /// What the element is said to be since it was built, by name ([`ElementRefMut::describe`]).
  descriptions: Vec<(Arc<str>, Arc<str>)>,
  /// Whether the element is out of the focus and the pointer ([`ElementRefMut::set_inert`]).
  inert: bool,
  override_clear_generation: u64,
  layout_dirty: bool,
  layout_subscribers: crate::core::layout_dirty::LayoutSubscribers,
  rect_observer: Option<Arc<dyn Fn(ElementRect) + Send + Sync>>,
  /// Whether what `rect_observer` changes is rendered and laid out again before the frame is painted
  /// ([`ElementRef::observe_rect_before_paint`]).
  before_paint: bool,
  /// The observers added beside the ref's own ([`ElementRef::add_rect_observer`]), each with its number and whether
  /// what it changes is shown before paint. Each is removed when its [`RectObserver`] is dropped.
  added_observers: Vec<(u64, RectCallback, bool)>,
  /// The number of the next added observer.
  next_observer: u64,
}

type RectCallback = Arc<dyn Fn(ElementRect) + Send + Sync>;

/// An observer of an element's rect added with [`ElementRef::add_rect_observer`] or
/// [`ElementRef::add_rect_observer_before_paint`]: it is called for as long as this is kept, and removed from the
/// ref when this is dropped. A component that adds one in each render keeps the last ones it added and drops those
/// of the render before, so that they do not pile up.
#[must_use = "the observer is removed when this is dropped"]
pub struct RectObserver {
  inner: std::sync::Weak<RwLock<ElementRefInner>>,
  id: u64,
}

impl Drop for RectObserver {
  fn drop(&mut self) {
    if let Some(inner) = self.inner.upgrade() {
      let mut inner = inner.write().unwrap_or_else(|poisoned| poisoned.into_inner());
      inner.added_observers.retain(|(id, ..)| *id != self.id);
    }
  }
}

impl std::fmt::Debug for RectObserver {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("RectObserver").field("id", &self.id).finish()
  }
}

thread_local! {
  /// Whether an observer of [`ElementRef::observe_rect_before_paint`] ran since the runtime last asked.
  static OBSERVED_BEFORE_PAINT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Whether an observer of [`ElementRef::observe_rect_before_paint`] ran on this thread since the last call: the
/// runtime renders what it changed and lays the frame out again before painting it.
pub(crate) fn take_observed_before_paint() -> bool {
  OBSERVED_BEFORE_PAINT.with(|observed| observed.replace(false))
}

impl ElementRef {
  pub(crate) fn track_layout(&self, watch: &Arc<crate::core::layout_dirty::LayoutWatch>) {
    self.inner.read().unwrap().layout_subscribers.bind(watch);
  }
  /// Typed access to the attached canvas; `None` before layout or after detachment.
  #[cfg(feature = "canvas")]
  pub fn as_canvas(&self) -> Option<crate::canvas::CanvasHandle> {
    let canvas = self.inner.read().unwrap().canvas.as_ref()?.upgrade()?;
    canvas.is_attached().then_some(canvas)
  }

  #[cfg(feature = "canvas")]
  pub(crate) fn bind_canvas(&self, canvas: &crate::canvas::CanvasHandle) {
    let mut inner = self.inner.write().unwrap();
    if let Some(old) = inner.canvas.as_ref().and_then(crate::canvas::CanvasWeak::upgrade)
      && old.is_attached()
      && old.surface_id() != canvas.surface_id()
    {
      tracing::warn!("an element ref cannot bind to two live canvases");
      return;
    }
    inner.canvas = Some(canvas.downgrade());
  }

  #[cfg(feature = "canvas")]
  pub(crate) fn detach_canvas(&self, id: crate::canvas::CanvasId) {
    let mut inner = self.inner.write().unwrap();
    if inner
      .canvas
      .as_ref()
      .and_then(crate::canvas::CanvasWeak::upgrade)
      .is_some_and(|canvas| canvas.surface_id() == id)
    {
      inner.canvas = None;
      inner.attached = false;
    }
  }

  pub fn new() -> Self {
    Self::default()
  }

  pub fn mutable(&self) -> ElementRefMut {
    ElementRefMut {
      inner: self.inner.clone(),
    }
  }

  pub fn x(&self) -> f32 {
    self.inner.read().unwrap().x
  }

  pub fn y(&self) -> f32 {
    self.inner.read().unwrap().y
  }

  pub fn width(&self) -> f32 {
    self.inner.read().unwrap().width
  }

  pub fn height(&self) -> f32 {
    self.inner.read().unwrap().height
  }

  pub fn rect(&self) -> (f32, f32, f32, f32) {
    let inner = self.inner.read().unwrap();
    (inner.x, inner.y, inner.width, inner.height)
  }

  pub fn bounds(&self) -> ElementRect {
    let inner = self.inner.read().unwrap();
    ElementRect {
      x: inner.x,
      y: inner.y,
      relative_x: inner.relative_x,
      relative_y: inner.relative_y,
      width: inner.width,
      height: inner.height,
    }
  }

  pub fn is_attached(&self) -> bool {
    self.inner.read().unwrap().attached
  }

  /// Whether the element is a text that the last layout drew in fewer letters than it has: ended by an ellipsis
  /// where its room ran out (`TextOverflow::Elipsis`), or cut to a limit of lines. `false` for a text drawn whole,
  /// for an element that is no text, and before it is laid out. A label that says itself whole in a tooltip only
  /// while it is cut reads this ([`ui::Tooltip::when_cut`](crate::ui::Tooltip::when_cut)).
  pub fn text_cut(&self) -> bool {
    self.inner.read().unwrap().text_cut
  }

  /// What the layout drew of the element's text: cut or whole.
  pub(crate) fn set_text_cut(&self, cut: bool) {
    self.inner.write().unwrap().text_cut = cut;
  }

  pub fn hovered(&self) -> bool {
    self.inner.read().unwrap().hovered
  }

  pub fn active(&self) -> bool {
    self.inner.read().unwrap().active
  }

  pub fn focused(&self) -> bool {
    self.focus_signal().get()
  }

  /// Reactive focus state. Reading `focused()` during render subscribes too.
  pub fn focus_signal(&self) -> crate::core::Signal<bool> {
    let mut inner = self.inner.write().unwrap();
    let focused = inner.focused;
    inner
      .focus_signal
      .get_or_insert_with(|| crate::core::Signal::new(focused))
      .clone()
  }

  /// Focused and showing a focus ring, like CSS `:focus-visible`: the focus
  /// came from the keyboard (or a key was pressed since), or the element is a
  /// text input. Reading it during render subscribes, like [`Self::focused`].
  pub fn focus_visible(&self) -> bool {
    self.focus_visible_signal().get()
  }

  /// Reactive focus-visible state. Reading `focus_visible()` during render
  /// subscribes too.
  pub fn focus_visible_signal(&self) -> crate::core::Signal<bool> {
    let mut inner = self.inner.write().unwrap();
    let visible = inner.focus_visible;
    inner
      .focus_visible_signal
      .get_or_insert_with(|| crate::core::Signal::new(visible))
      .clone()
  }

  /// Focus-visible on the element or on something in it, like CSS
  /// `:focus-within` for a focus that shows its ring. Reading it during render
  /// subscribes, like [`Self::focus_visible`].
  pub fn focus_visible_within(&self) -> bool {
    self.focus_visible_within_signal().get()
  }

  /// Reactive focus-visible-within state. Reading `focus_visible_within()`
  /// during render subscribes too.
  pub fn focus_visible_within_signal(&self) -> crate::core::Signal<bool> {
    let mut inner = self.inner.write().unwrap();
    let within = inner.focus_visible_within;
    inner
      .focus_visible_within_signal
      .get_or_insert_with(|| crate::core::Signal::new(within))
      .clone()
  }

  pub(crate) fn set_focus_visible_within(&self, within: bool) {
    let signal = {
      let mut inner = self.inner.write().unwrap();
      if inner.focus_visible_within == within {
        return;
      }
      inner.focus_visible_within = within;
      inner.focus_visible_within_signal.clone()
    };
    if let Some(signal) = signal {
      signal.set(within);
    }
  }

  pub(crate) fn update(&self, x: f32, y: f32, relative_x: f32, relative_y: f32, width: f32, height: f32) {
    let observers = {
      let mut inner = self.inner.write().unwrap();
      let changed = !inner.attached
        || inner.x != x
        || inner.y != y
        || inner.relative_x != relative_x
        || inner.relative_y != relative_y
        || inner.width != width
        || inner.height != height;
      inner.x = x;
      inner.y = y;
      inner.relative_x = relative_x;
      inner.relative_y = relative_y;
      inner.width = width;
      inner.height = height;
      inner.attached = true;
      if changed {
        let before_paint = (inner.before_paint && inner.rect_observer.is_some())
          || inner.added_observers.iter().any(|(_, _, before_paint)| *before_paint);
        if before_paint {
          OBSERVED_BEFORE_PAINT.with(|observed| observed.set(true));
        }
        let mut observers: Vec<RectCallback> = inner.rect_observer.iter().cloned().collect();
        observers.extend(inner.added_observers.iter().map(|(_, observer, _)| observer.clone()));
        observers
      } else {
        Vec::new()
      }
    };
    // Outside the lock: an observer may read this ref (or write signals
    // whose subscribers do), or add and remove observers.
    let rect = ElementRect {
      x,
      y,
      relative_x,
      relative_y,
      width,
      height,
    };
    for observer in observers {
      observer(rect);
    }
  }

  /// Register a callback fired whenever the measured rect changes, including
  /// the first time the element is laid out. It runs during quad resolution
  /// (after layout, before present), so writing a [`crate::core::Signal`]
  /// from it marks the owning component dirty and the runtime schedules a
  /// follow-up pass — the measure-then-render handshake for components whose
  /// render depends on their own measured size.
  ///
  /// This is the ref's own observer: registering again replaces it, so the
  /// owner of the element registers it in every render. Code that watches a
  /// ref it was given, and must not take its owner's observer away (a list of
  /// a page's sections that follows their scroll), adds one of its own with
  /// [`add_rect_observer`](Self::add_rect_observer), beside this one.
  pub fn observe_rect(&self, observer: impl Fn(ElementRect) + Send + Sync + 'static) {
    let mut inner = self.inner.write().unwrap();
    inner.rect_observer = Some(Arc::new(observer));
    inner.before_paint = false;
  }

  /// As [`observe_rect`](Self::observe_rect), but what the observer changes is shown in the frame that laid the
  /// element out: where it writes a signal a component renders from, or gives an element a size through its ref
  /// ([`ElementRefMut::set_size`]), the runtime renders what is to be rendered and lays the frame out again before
  /// painting it. A component whose form follows its own room (a timeline that folds its two sides to one under a
  /// least width) shows the form that fits from its first frame, and as a panel is dragged across the width, with
  /// no frame of the other form; a splitter has its panes at the sizes that fit its room in its first frame.
  ///
  /// What is laid out again may be measured anew, so the frame is laid out again until nothing that is watched
  /// changes, four times at most. The observer should change what it writes only where the rect crosses what it is
  /// watching for: a value that changes on every rect lays every frame out again and again, and settles in none.
  ///
  /// This is the ref's own observer, as [`observe_rect`](Self::observe_rect)'s is, and replaces it; one added beside
  /// it is [`add_rect_observer_before_paint`](Self::add_rect_observer_before_paint).
  pub fn observe_rect_before_paint(&self, observer: impl Fn(ElementRect) + Send + Sync + 'static) {
    let mut inner = self.inner.write().unwrap();
    inner.rect_observer = Some(Arc::new(observer));
    inner.before_paint = true;
  }

  /// Adds an observer of the rect, called as [`observe_rect`](Self::observe_rect)'s is, beside the ref's own
  /// observer and any others added: none replaces another. It is called while the [`RectObserver`] this gives is
  /// kept, and removed when that is dropped. It is not called for the rect the element has now, only when that
  /// changes.
  pub fn add_rect_observer(&self, observer: impl Fn(ElementRect) + Send + Sync + 'static) -> RectObserver {
    self.add_observer(Arc::new(observer), false)
  }

  /// As [`add_rect_observer`](Self::add_rect_observer), and what the observer changes is shown in the frame that
  /// laid the element out, as with [`observe_rect_before_paint`](Self::observe_rect_before_paint).
  pub fn add_rect_observer_before_paint(&self, observer: impl Fn(ElementRect) + Send + Sync + 'static) -> RectObserver {
    self.add_observer(Arc::new(observer), true)
  }

  fn add_observer(&self, observer: RectCallback, before_paint: bool) -> RectObserver {
    let mut inner = self.inner.write().unwrap();
    let id = inner.next_observer;
    inner.next_observer += 1;
    inner.added_observers.push((id, observer, before_paint));
    RectObserver {
      inner: Arc::downgrade(&self.inner),
      id,
    }
  }

  pub(crate) fn same_handle(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.inner, &other.inner)
  }

  pub(crate) fn override_rect(&self) -> Option<ElementRect> {
    self.inner.read().unwrap().override_rect
  }

  /// The width and the height [`ElementRefMut::set_size`] gave the element, each `None` where it is laid out
  /// as it was built.
  pub(crate) fn size_override(&self) -> (Option<Dimension>, Option<Dimension>) {
    self.inner.read().unwrap().size_override
  }

  /// Whether [`ElementRefMut::set_inert`] made the element inert.
  pub(crate) fn is_inert(&self) -> bool {
    self.inner.read().unwrap().inert
  }

  /// What [`ElementRefMut::describe`] said of the element. Read only by
  /// tooling.
  #[cfg(any(feature = "devtools", feature = "mcp"))]
  pub(crate) fn descriptions(&self) -> Vec<(Arc<str>, Arc<str>)> {
    self.inner.read().unwrap().descriptions.clone()
  }

  pub(crate) fn override_clear_generation(&self) -> u64 {
    self.inner.read().unwrap().override_clear_generation
  }

  pub(crate) fn has_layout_dirty(&self) -> bool {
    self.inner.read().unwrap().layout_dirty
  }

  pub(crate) fn take_layout_dirty(&self) -> bool {
    let mut inner = self.inner.write().unwrap();
    let dirty = inner.layout_dirty;
    inner.layout_dirty = false;
    dirty
  }

  pub(crate) fn set_hovered(&self, hovered: bool) {
    self.inner.write().unwrap().hovered = hovered;
  }

  pub(crate) fn set_active(&self, active: bool) {
    self.inner.write().unwrap().active = active;
  }

  /// Losing focus also loses focus-visible.
  pub(crate) fn set_focused(&self, focused: bool) {
    if !focused {
      self.set_focus_visible(false);
    }
    let signal = {
      let mut inner = self.inner.write().unwrap();
      if inner.focused == focused {
        return;
      }
      inner.focused = focused;
      inner.focus_signal.clone()
    };
    if let Some(signal) = signal {
      signal.set(focused);
    }
  }

  pub(crate) fn set_focus_visible(&self, visible: bool) {
    let signal = {
      let mut inner = self.inner.write().unwrap();
      if inner.focus_visible == visible {
        return;
      }
      inner.focus_visible = visible;
      inner.focus_visible_signal.clone()
    };
    if let Some(signal) = signal {
      signal.set(visible);
    }
  }
}

impl ElementRefMut {
  #[cfg(feature = "canvas")]
  pub fn as_canvas(&self) -> Option<crate::canvas::CanvasHandle> {
    self.as_ref().as_canvas()
  }

  pub fn new() -> Self {
    Self::default()
  }

  pub fn as_ref(&self) -> ElementRef {
    ElementRef {
      inner: self.inner.clone(),
    }
  }

  pub fn x(&self) -> f32 {
    self.as_ref().x()
  }

  pub fn y(&self) -> f32 {
    self.as_ref().y()
  }

  pub fn width(&self) -> f32 {
    self.as_ref().width()
  }

  pub fn height(&self) -> f32 {
    self.as_ref().height()
  }

  pub fn rect(&self) -> (f32, f32, f32, f32) {
    self.as_ref().rect()
  }

  pub fn bounds(&self) -> ElementRect {
    self.as_ref().bounds()
  }

  /// See [`ElementRef::observe_rect`].
  pub fn observe_rect(&self, observer: impl Fn(ElementRect) + Send + Sync + 'static) {
    self.as_ref().observe_rect(observer);
  }

  /// As [`ElementRef::observe_rect_before_paint`].
  pub fn observe_rect_before_paint(&self, observer: impl Fn(ElementRect) + Send + Sync + 'static) {
    self.as_ref().observe_rect_before_paint(observer);
  }

  /// See [`ElementRef::add_rect_observer`].
  pub fn add_rect_observer(&self, observer: impl Fn(ElementRect) + Send + Sync + 'static) -> RectObserver {
    self.as_ref().add_rect_observer(observer)
  }

  /// See [`ElementRef::add_rect_observer_before_paint`].
  pub fn add_rect_observer_before_paint(&self, observer: impl Fn(ElementRect) + Send + Sync + 'static) -> RectObserver {
    self.as_ref().add_rect_observer_before_paint(observer)
  }

  pub fn set_bounds(&self, rect: ElementRect) {
    let mut inner = self.inner.write().unwrap();
    inner.x = rect.x;
    inner.y = rect.y;
    inner.relative_x = rect.relative_x;
    inner.relative_y = rect.relative_y;
    inner.width = rect.width;
    inner.height = rect.height;
    inner.override_rect = Some(rect);
    inner.layout_dirty = true;
    inner.layout_subscribers.mark();
  }

  pub fn set_relative_bounds(&self, relative_x: f32, relative_y: f32, width: f32, height: f32) {
    let current = self.bounds();
    let parent_x = current.x - current.relative_x;
    let parent_y = current.y - current.relative_y;
    self.set_bounds(ElementRect {
      x: parent_x + relative_x,
      y: parent_y + relative_y,
      relative_x,
      relative_y,
      width,
      height,
    });
  }

  /// Lays the element out `width` wide and `height` high in place of the width and the height it was built
  /// with, until this is called again: `None` leaves that side as it was built. The least and the most size
  /// it was built with still hold, and its parent places it as it would any element of that size. Nothing is
  /// rendered again: the element and what holds it are laid out again at the next pass, which this asks for.
  /// A splitter moves its panes so while a divider is dragged, without building what is in them again.
  pub fn set_size(&self, width: Option<Dimension>, height: Option<Dimension>) {
    let mut inner = self.inner.write().unwrap();
    if inner.size_override == (width, height) {
      return;
    }
    inner.size_override = (width, height);
    inner.layout_dirty = true;
    inner.layout_subscribers.mark();
  }

  /// Says `value` of the element under `name` to tooling that reads the tree (the DevTools, the MCP server), in
  /// place of what its `describe(name, ..)` said when it was built, until it is said again. Nothing is rendered
  /// again: a splitter tells where its divider is after a drag so.
  pub fn describe(&self, name: impl Into<Arc<str>>, value: impl Into<Arc<str>>) {
    let (name, value) = (name.into(), value.into());
    let mut inner = self.inner.write().unwrap();
    match inner.descriptions.iter_mut().find(|(said, _)| *said == name) {
      Some((_, held)) => *held = value,
      None => inner.descriptions.push((name, value)),
    }
  }

  /// Makes the element inert, or not, as `.inert(..)` does where it was built, with nothing rendered again: Tab and
  /// a focus group's keys pass by everything in it, no pointer reaches it, and the focus in it leaves at the next
  /// pass, which this asks for. Built inert, it stays inert whatever this says. A splitter makes a pane it closes
  /// inert so while a divider is dragged.
  pub fn set_inert(&self, inert: bool) {
    let mut inner = self.inner.write().unwrap();
    if inner.inert == inert {
      return;
    }
    inner.inert = inert;
    // A pass, after whose layout the focus leaves what became inert.
    inner.layout_dirty = true;
    inner.layout_subscribers.mark();
  }

  /// The width and the height [`Self::set_size`] gave the element.
  pub fn size(&self) -> (Option<Dimension>, Option<Dimension>) {
    self.as_ref().size_override()
  }

  pub fn clear_bounds_override(&self) {
    let mut inner = self.inner.write().unwrap();
    if inner.override_rect.take().is_some() {
      // Each node consuming this handle must discard its cached offset.
      // A shared flag would let the first tree hide the clear from the rest.
      inner.override_clear_generation = OVERRIDE_CLEAR_GENERATION.fetch_add(1, Ordering::Relaxed);
    }
    inner.layout_dirty = true;
    inner.layout_subscribers.mark();
  }

  pub fn is_attached(&self) -> bool {
    self.as_ref().is_attached()
  }

  pub fn hovered(&self) -> bool {
    self.as_ref().hovered()
  }

  pub fn active(&self) -> bool {
    self.as_ref().active()
  }

  pub fn focused(&self) -> bool {
    self.as_ref().focused()
  }

  pub fn focus_signal(&self) -> crate::core::Signal<bool> {
    self.as_ref().focus_signal()
  }

  /// See [`ElementRef::focus_visible`].
  pub fn focus_visible(&self) -> bool {
    self.as_ref().focus_visible()
  }

  /// See [`ElementRef::focus_visible_signal`].
  pub fn focus_visible_signal(&self) -> crate::core::Signal<bool> {
    self.as_ref().focus_visible_signal()
  }
}

impl From<ElementRefMut> for ElementRef {
  fn from(value: ElementRefMut) -> Self {
    value.as_ref()
  }
}

#[cfg(all(test, any(feature = "devtools", feature = "mcp")))]
mod tests {
  use std::sync::Arc;

  use super::ElementRefMut;
  use crate::node::{NodeUpdate, node::Node};

  fn attr(node: &Node, name: &str) -> Option<String> {
    node
      .debug_attrs()
      .iter()
      .find(|(said, _)| said.as_ref() == name)
      .map(|(_, value)| value.to_string())
  }

  #[test]
  fn what_a_ref_describes_stands_for_what_the_element_was_built_with() {
    let handle = ElementRefMut::new();
    let mut node = Node::new().ref_element(handle.clone());
    NodeUpdate::describe(&mut node, Arc::from("a11y_role"), Arc::from("separator"));
    NodeUpdate::describe(&mut node, Arc::from("a11y_valuenow"), Arc::from("120"));
    assert_eq!(attr(&node, "a11y_valuenow").as_deref(), Some("120"));
    handle.describe("a11y_valuenow", "180");
    handle.describe("a11y_valuemax", "240");
    assert_eq!(attr(&node, "a11y_valuenow").as_deref(), Some("180"));
    assert_eq!(attr(&node, "a11y_valuemax").as_deref(), Some("240"));
    assert_eq!(attr(&node, "a11y_role").as_deref(), Some("separator"));
    assert_eq!(node.debug_attrs().len(), 3, "said once each");
    handle.describe("a11y_valuenow", "200");
    assert_eq!(attr(&node, "a11y_valuenow").as_deref(), Some("200"));
  }
}
