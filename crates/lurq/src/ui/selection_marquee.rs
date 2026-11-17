//! A selection marquee: the rectangle a person drags over a surface to select what it touches.

use std::sync::Arc;

use parking_lot::Mutex;

use super::{
  names,
  part_style::PartStyle,
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    component::Component,
    ctx::Ctx,
    events::{DragEvent, KeyboardEvent, MouseButton, MouseButtonMask, MouseEvent},
    theme::{BorderSize, PaletteColor, Theme},
  },
  components::{Row, Stack},
  core::{ElementRef, Signal},
  layout::layout_kind::ScrollState,
  node::{Element, border::Border},
};

/// How far a press on the surface moves before it is a marquee: the distance past which the engine clicks nothing
/// on the release either.
const DRAG_START: f32 = 4.0;

/// The parts of a selection marquee a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SelectionMarqueePart {
  /// The surface: what holds the app's content, and the rectangle over it.
  Root,
  /// The rectangle that is dragged: its fill, its edge and how far it is seen through.
  Marquee,
}

names!(SelectionMarqueePart {
  Root => "root",
  Marquee => "marquee",
});

/// The states of a selection marquee a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SelectionMarqueeState {
  /// A rectangle is being dragged.
  Active,
  /// The surface takes no drag.
  Disabled,
}

names!(SelectionMarqueeState {
  Active => "active",
  Disabled => "disabled",
});

/// What builds a selection marquee's nodes: a theme's in place of the default one.
pub type SelectionMarqueeRenderer = Arc<dyn Fn(SelectionMarqueeView) -> Element + Send + Sync>;

/// What a marquee does to what was selected before it, as the keys held at its press say.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MarqueeMode {
  /// What it touches is the selection: no key held.
  #[default]
  Replace,
  /// What it touches is added to the selection: Shift held.
  Add,
  /// What it touches goes into the selection, or out of it where it was in: Ctrl held, Cmd on a Mac.
  Toggle,
}

names!(MarqueeMode {
  Replace => "replace",
  Add => "add",
  Toggle => "toggle",
});

/// The rectangle of a marquee as the app is told it, in the surface's own coordinates: from the top left corner of
/// what the marquee wraps, whatever it is scrolled to.
///
/// ```
/// use lurq::ui::{Marquee, MarqueeMode};
///
/// # fn told(marquee: &Marquee, before: &[u32]) -> Vec<u32> {
/// // The tiles of a grid of 72 by 50, three across, that the rectangle touches.
/// let touched = (0..6_u32).filter(|tile| {
///   let (column, row) = ((tile % 3) as f32, (tile / 3) as f32);
///   marquee.touches(14.0 + column * 72.0, 14.0 + row * 50.0, 58.0, 38.0)
/// });
/// // What is selected now: the touched, added to or taken out of what was selected before as the keys say.
/// marquee.select(before, touched)
/// # }
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Marquee {
  /// How far its start is from the surface's.
  pub x: f32,
  /// How far its top is under the surface's.
  pub y: f32,
  pub width: f32,
  pub height: f32,
  /// What it does to what was selected before it.
  pub mode: MarqueeMode,
  /// Where the surface's top left corner was laid out in the window.
  surface: (f32, f32),
}

impl Marquee {
  /// Whether the rectangle touches the box at `x`, `y` of the surface, `width` by `height`: they share some room.
  /// A box of no size is touched by nothing.
  pub fn touches(&self, x: f32, y: f32, width: f32, height: f32) -> bool {
    width > 0.0
      && height > 0.0
      && x < self.x + self.width
      && self.x < x + width
      && y < self.y + self.height
      && self.y < y + height
  }

  /// Whether the box at `x`, `y` of the surface, `width` by `height`, is wholly in the rectangle.
  pub fn contains(&self, x: f32, y: f32, width: f32, height: f32) -> bool {
    x >= self.x && y >= self.y && x + width <= self.x + self.width && y + height <= self.y + self.height
  }

  /// Whether the rectangle touches `element`, something on the surface, where it was last laid out: for a surface
  /// whose items the app keeps a ref of each, and not their places.
  pub fn touches_element(&self, element: &ElementRef) -> bool {
    let (x, y, width, height) = element.rect();
    self.touches(x - self.surface.0, y - self.surface.1, width, height)
  }

  /// What is selected with this rectangle over `touched`, where `before` was selected when it began: the touched
  /// alone, `before` and the touched, or `before` with each of the touched put in or taken out, as its
  /// [`mode`](Self::mode) says.
  pub fn select<T: Clone + PartialEq>(&self, before: &[T], touched: impl IntoIterator<Item = T>) -> Vec<T> {
    let touched: Vec<T> = touched.into_iter().collect();
    match self.mode {
      MarqueeMode::Replace => touched,
      MarqueeMode::Add => {
        let mut selected = before.to_vec();
        for item in touched {
          if !selected.contains(&item) {
            selected.push(item);
          }
        }
        selected
      }
      MarqueeMode::Toggle => {
        let mut selected: Vec<T> = before.iter().filter(|item| !touched.contains(item)).cloned().collect();
        selected.extend(touched.into_iter().filter(|item| !before.contains(item)));
        selected
      }
    }
  }
}

type Told = Arc<dyn Fn(&Marquee) + Send + Sync>;

/// The rectangle a person drags over a surface to select what it touches: the tiles of a grid, the files of a
/// folder, the nodes of a canvas.
///
/// ```
/// use lurq::{components::Row, core::Signal, ui::SelectionMarquee};
///
/// # fn render(ctx: &mut lurq::app::ctx::Ctx, selected: Signal<Vec<u32>>, before: Signal<Vec<u32>>) {
/// # let tiles = Row::new();
/// # let touched = |_: &lurq::ui::Marquee| -> Vec<u32> { Vec::new() };
/// let (held, pick, back) = (selected.clone(), selected.clone(), selected.clone());
/// let (kept, read, undo) = (before.clone(), before.clone(), before.clone());
/// let surface = SelectionMarquee::new(tiles)
///   // What was selected when the drag began: what Shift adds to, and what Escape puts back.
///   .on_start(move |_| kept.set(held.get_untracked()))
///   .on_change(move |marquee| pick.set(marquee.select(&read.get_untracked(), touched(marquee))))
///   .on_cancel(move || back.set(undo.get_untracked()))
///   // Mounted, so that the rectangle is drawn.
///   .mount(ctx);
/// # }
/// ```
///
/// It wraps the app's surface and draws nothing of its own until a press on it moves past a few pixels. From then
/// the rectangle runs from where the button went down to the pointer, and the app is told it:
/// [`on_start`](Self::on_start) once, [`on_change`](Self::on_change) as it changes and
/// [`on_change_end`](Self::on_change_end) when the button comes up. The rectangle is in the surface's own
/// coordinates, and says what the keys held at the press ask for ([`Marquee::mode`]): Shift adds to what was
/// selected, Ctrl (Cmd) puts in and takes out. Escape ends it and tells [`on_cancel`](Self::on_cancel); the release
/// that follows tells nothing. What is selected is the app's: it reckons what the rectangle touches
/// ([`Marquee::touches`], [`Marquee::touches_element`]) and keeps the selection ([`Marquee::select`]).
///
/// A press on something of the surface that is dragged itself is that thing's drag, and no marquee. In a scroll
/// container the surface is what scrolls, the marquee around the content: at the container's edge the drag scrolls
/// it (the engine's `drag_autoscroll`), and with the container's state ([`scroll`](Self::scroll)) the rectangle
/// keeps its start on the content as it scrolls. The rectangle is drawn where the marquee is
/// [mounted](Self::mount); converted into an element on its own it still tells the app, with nothing drawn.
pub struct SelectionMarquee {
  child: Element,
  scroll: Option<ScrollState>,
  rect: Option<(f32, f32, f32, f32)>,
  disabled: bool,
  label: Option<Arc<str>>,
  on_start: Vec<Told>,
  on_change: Vec<Told>,
  on_change_end: Vec<Told>,
  on_cancel: Vec<Arc<dyn Fn() + Send + Sync>>,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl SelectionMarquee {
  /// A marquee over `child`, the app's surface.
  pub fn new(child: impl Into<Element>) -> Self {
    Self {
      child: child.into(),
      scroll: None,
      rect: None,
      disabled: false,
      label: None,
      on_start: Vec::new(),
      on_change: Vec::new(),
      on_change_end: Vec::new(),
      on_cancel: Vec::new(),
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// The state of the scroll container the surface is in: the rectangle keeps its start on the content while the
  /// container scrolls under the drag, by the wheel or at its edge.
  pub fn scroll(mut self, state: &ScrollState) -> Self {
    self.scroll = Some(state.clone());
    self
  }

  /// Called once with the rectangle as a drag becomes a marquee: where the app keeps what was selected before
  /// it. A second handler is called after the first.
  pub fn on_start(mut self, handler: impl Fn(&Marquee) + Send + Sync + 'static) -> Self {
    self.on_start.push(Arc::new(handler));
    self
  }

  /// Called with the rectangle each time it changes, after `on_start`: as the pointer moves, and as the surface
  /// scrolls under it.
  pub fn on_change(mut self, handler: impl Fn(&Marquee) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// Called with the last rectangle when the button comes up. Not called for a marquee Escape ended.
  pub fn on_change_end(mut self, handler: impl Fn(&Marquee) + Send + Sync + 'static) -> Self {
    self.on_change_end.push(Arc::new(handler));
    self
  }

  /// Called when Escape ends a marquee: where the app puts back what was selected before it.
  pub fn on_cancel(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_cancel.push(Arc::new(handler));
    self
  }

  /// Whether the surface takes no drag.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// What the surface is called, to what reads the tree. It draws nothing.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// Draws the rectangle at `x`, `y` of the surface, `width` by `height`, with nothing dragging it: for a gallery,
  /// a screenshot, a test.
  pub fn rect(mut self, x: f32, y: f32, width: f32, height: f32) -> Self {
    self.rect = Some((x, y, width, height));
    self
  }

  /// Puts the marquee in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this marquee over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this marquee.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// The id of the surface. The rectangle has this id with `-marquee` after it.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the marquee in `theme`, with nothing kept: a drag still tells the app its rectangle, but none is
  /// drawn. Converting a marquee into an element does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    self.build_with(theme, Kept::new(None))
  }

  /// The marquee mounted where it is declared, where it keeps a drag from one render to the next: the rectangle is
  /// drawn as it is dragged.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let key = self.key.clone();
    let props = SelectionMarqueeProps { marquee: self };
    match key {
      Some(key) => ctx.mount_keyed::<SelectionMarqueeHost>(&key, props).key(key),
      None => ctx.mount::<SelectionMarqueeHost>(props),
    }
  }

  fn build_with(&self, theme: &Theme, kept: Kept) -> Element {
    let drawn = self.rect.or_else(|| kept.drawn().map(|drag| drag.rect()));
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(SelectionMarqueeState::Active, drawn.is_some())
    .state(SelectionMarqueeState::Disabled, self.disabled);
    let view = SelectionMarqueeView {
      child: Some(self.child.clone()),
      rect: drawn,
      disabled: self.disabled,
      label: self.label.clone(),
      recipe: theme.resolve::<SelectionMarquee>(&case, Some(&self.rules)),
      model: Arc::new(Model {
        scroll: self.scroll.clone(),
        on_start: self.on_start.clone(),
        on_change: self.on_change.clone(),
        on_change_end: self.on_change_end.clone(),
        on_cancel: self.on_cancel.clone(),
        kept,
      }),
      id: self.id.clone(),
      key: self.key.clone(),
    };
    match theme.renderer::<SelectionMarquee>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<SelectionMarquee> for Element {
  fn from(marquee: SelectionMarquee) -> Self {
    marquee.build(&current_theme())
  }
}

/// A marquee under way, from the press to the release.
#[derive(Clone, Copy, Debug)]
struct Drag {
  /// Where the button went down, in the surface's coordinates.
  anchor: (f32, f32),
  /// Where the pointer is, in the surface's coordinates, kept on the surface.
  point: (f32, f32),
  /// Where the surface's top left corner was in the window at the press, and what its scroll container was
  /// scrolled to.
  origin: (f32, f32),
  scrolled: (f32, f32),
  mode: MarqueeMode,
  /// Whether the pointer has moved far enough for this to be a marquee and not a press.
  begun: bool,
}

impl Drag {
  /// The rectangle from where the button went down to the pointer: x, y, width, height.
  fn rect(&self) -> (f32, f32, f32, f32) {
    let (x, y) = (self.anchor.0.min(self.point.0), self.anchor.1.min(self.point.1));
    (
      x,
      y,
      (self.anchor.0 - self.point.0).abs(),
      (self.anchor.1 - self.point.1).abs(),
    )
  }
}

/// What a selection marquee keeps from one render to the next. A mounted one keeps it and is rendered again as the
/// rectangle changes; one converted into an element on its own starts again at each build, and draws nothing.
#[derive(Clone)]
struct Kept {
  drag: Arc<Mutex<Option<Drag>>>,
  /// Renders a mounted marquee again: its rectangle changed.
  revision: Option<Signal<u64>>,
  /// The keys held at the last press on the surface.
  mode: Arc<Mutex<MarqueeMode>>,
  surface: ElementRef,
}

impl Kept {
  fn new(revision: Option<Signal<u64>>) -> Self {
    Self {
      drag: Arc::default(),
      revision,
      mode: Arc::default(),
      surface: ElementRef::new(),
    }
  }

  fn moved(&self) {
    if let Some(revision) = &self.revision {
      revision.update(|revision| *revision += 1);
    }
  }

  /// The marquee drawn now: one that has begun, of a mounted surface.
  fn drawn(&self) -> Option<Drag> {
    self.revision.as_ref()?;
    (*self.drag.lock()).filter(|drag| drag.begun)
  }
}

/// What the handlers of a surface know of its marquee.
struct Model {
  scroll: Option<ScrollState>,
  on_start: Vec<Told>,
  on_change: Vec<Told>,
  on_change_end: Vec<Told>,
  on_cancel: Vec<Arc<dyn Fn() + Send + Sync>>,
  kept: Kept,
}

impl Model {
  /// What the scroll container the surface is in is scrolled to.
  fn scrolled(&self) -> (f32, f32) {
    self
      .scroll
      .as_ref()
      .map_or((0.0, 0.0), |state| (state.scroll_x(), state.scroll_y()))
  }

  /// The rectangle of `drag` as the app is told it.
  fn told(&self, drag: &Drag) -> Marquee {
    let (x, y, width, height) = drag.rect();
    Marquee {
      x,
      y,
      width,
      height,
      mode: drag.mode,
      surface: (self.kept.surface.x(), self.kept.surface.y()),
    }
  }

  fn press(&self, event: &MouseEvent) {
    if event.button != MouseButton::Left {
      return;
    }
    *self.kept.mode.lock() = if event.shift {
      MarqueeMode::Add
    } else if event.ctrl || event.meta {
      MarqueeMode::Toggle
    } else {
      MarqueeMode::Replace
    };
  }

  fn start(&self, event: &DragEvent) {
    let origin = (self.kept.surface.x(), self.kept.surface.y());
    let anchor = (event.x - origin.0, event.y - origin.1);
    *self.kept.drag.lock() = Some(Drag {
      anchor,
      point: anchor,
      origin,
      scrolled: self.scrolled(),
      mode: *self.kept.mode.lock(),
      begun: false,
    });
  }

  /// Where the pointer of `event` is on the surface, kept on it: with the state of its scroll container, from
  /// where the surface was at the press and how far it has scrolled since; without, from where it was last laid
  /// out.
  fn point(&self, drag: &Drag, event: &DragEvent) -> (f32, f32) {
    let (x, y) = match &self.scroll {
      Some(_) => {
        let now = self.scrolled();
        (
          event.x - drag.origin.0 + now.0 - drag.scrolled.0,
          event.y - drag.origin.1 + now.1 - drag.scrolled.1,
        )
      }
      None => (event.x - self.kept.surface.x(), event.y - self.kept.surface.y()),
    };
    let (width, height) = (self.kept.surface.width(), self.kept.surface.height());
    (x.clamp(0.0, width.max(0.0)), y.clamp(0.0, height.max(0.0)))
  }

  /// A move of the pointer, or of the surface under it. Gives whether there is a marquee.
  fn moved(&self, event: &DragEvent) -> bool {
    let (told, begins) = {
      let mut held = self.kept.drag.lock();
      let Some(drag) = held.as_mut() else {
        return false;
      };
      let point = self.point(drag, event);
      let begins = !drag.begun && event.total_delta_x.hypot(event.total_delta_y) > DRAG_START;
      if begins {
        drag.begun = true;
      }
      if !drag.begun || (!begins && point == drag.point) {
        return drag.begun;
      }
      drag.point = point;
      (self.told(drag), begins)
    };
    for handler in if begins { &self.on_start } else { &self.on_change } {
      handler(&told);
    }
    // The first rectangle is told as a change too: an app that only follows the changes misses none.
    if begins {
      for handler in &self.on_change {
        handler(&told);
      }
    }
    self.kept.moved();
    true
  }

  fn end(&self, event: &DragEvent) {
    // The last place of the pointer is the release's.
    self.moved(event);
    let Some(drag) = self.kept.drag.lock().take() else {
      return;
    };
    if !drag.begun {
      return;
    }
    let told = self.told(&drag);
    for handler in &self.on_change_end {
      handler(&told);
    }
    self.kept.moved();
  }

  /// Escape: ends a marquee under way. Gives whether there was one.
  fn cancel(&self) -> bool {
    let mut held = self.kept.drag.lock();
    if !held.as_ref().is_some_and(|drag| drag.begun) {
      return false;
    }
    *held = None;
    drop(held);
    for handler in &self.on_cancel {
      handler();
    }
    self.kept.moved();
    true
  }
}

/// A selection marquee as a renderer sees it: the app's surface, the rectangle to draw, and the style of its parts.
pub struct SelectionMarqueeView {
  child: Option<Element>,
  /// The rectangle to draw, in the surface's coordinates: x, y, width, height. `None` while none is dragged.
  pub rect: Option<(f32, f32, f32, f32)>,
  pub disabled: bool,
  /// What the surface is called, to what reads the tree.
  pub label: Option<Arc<str>>,
  /// The style of each part of this marquee.
  pub recipe: ResolvedRecipe,
  model: Arc<Model>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl SelectionMarqueeView {
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The surface in the style of the root, with nothing in it yet: a press on it that moves is a marquee, which
  /// scrolls the container it is in at that container's edge, and Escape ends it.
  pub fn root(&self) -> Stack {
    let mut root = Stack::new()
      .ref_element(self.model.kept.surface.clone())
      .part(&self.part(SelectionMarqueePart::Root));
    if !self.disabled {
      let (press, start, moved, end, keys) = (
        self.model.clone(),
        self.model.clone(),
        self.model.clone(),
        self.model.clone(),
        self.model.clone(),
      );
      root = root
        .on_mouse_down(move |event: MouseEvent| press.press(&event))
        .start_drag_buttons(MouseButtonMask::LEFT)
        .drag_autoscroll(true)
        .on_drag_start(move |event: DragEvent| start.start(&event))
        .on_drag_move(move |event: DragEvent| {
          moved.moved(&event);
        })
        .on_drag_end(move |event: DragEvent| end.end(&event))
        .on_key_down(move |event: KeyboardEvent| {
          if event.key == "Escape" && keys.cancel() {
            event.prevent_default();
          }
        });
    }
    if let Some(label) = &self.label {
      root = root.describe("a11y_name", label.clone());
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root
  }

  /// The rectangle in the style of its part, where it is on the surface: `None` while none is dragged. The pointer
  /// passes through it.
  pub fn marquee(&self) -> Option<Element> {
    let (x, y, width, height) = self.rect?;
    let mut marquee = Row::new()
      .pointer_events_none()
      .absolute(x, y, width, height)
      .part(&self.part(SelectionMarqueePart::Marquee));
    if let Some(id) = &self.id {
      marquee = marquee.id(format!("{id}-marquee"));
    }
    Some(marquee.into())
  }

  /// How the default renderer builds a marquee: the app's surface, and the rectangle over it.
  pub fn render(mut self) -> Element {
    let child = self.child.take();
    self.root().with_children(child).with_children(self.marquee()).into()
  }
}

impl Themed for SelectionMarquee {
  const NAME: &'static str = "selection_marquee";
  type Renderer = SelectionMarqueeRenderer;

  /// The design's selection marquee (`pencil/lurq-design-system.pen`, Selection Marquee): a rectangle on the muted
  /// info fill in an edge of 1 in the info color, the whole of it at 0.55, drawn over the surface from where the
  /// button went down to the pointer. The surface itself is the app's, and the root gives it nothing.
  fn recipe() -> Recipe {
    Recipe::new().base(SelectionMarqueePart::Marquee, |s| {
      s.background(PaletteColor::InfoMuted)
        .border(Border::inside(BorderSize::Sm, PaletteColor::Info))
        .opacity(0.55)
    })
  }
}

struct SelectionMarqueeProps {
  marquee: SelectionMarquee,
}

/// A marquee built again is drawn again: its surface is the app's as it is built.
impl PartialEq for SelectionMarqueeProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

impl crate::app::component::DevtoolsInspectable for SelectionMarqueeProps {}

/// What a mounted selection marquee keeps between renders.
struct SelectionMarqueeHost {
  kept: Kept,
}

impl Component for SelectionMarqueeHost {
  type Props = SelectionMarqueeProps;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      kept: Kept::new(Some(ctx.signal(0_u64))),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    if let Some(revision) = &self.kept.revision {
      // Read so that a rectangle that changes renders the marquee again.
      revision.get();
    }
    ctx
      .props::<SelectionMarqueeProps>()
      .marquee
      .build_with(&current_theme(), self.kept.clone())
  }
}
