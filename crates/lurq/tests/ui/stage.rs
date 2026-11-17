//! A window that renders a scene with an app's theme, and what was painted in it.

use std::{
  sync::Arc,
  time::{Duration, Instant},
};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx, events::MouseButton},
  layout::{
    Constraints, Size,
    quad::{Quad, QuadContent},
  },
  node::{Element, color::Color},
};

use crate::support::TestSurface;

type Scene = Arc<dyn Fn(&mut Ctx) -> Element + Send + Sync>;

#[derive(Clone, lurq::DevtoolsInspectable)]
struct Props {
  #[devtools_ignore]
  scene: Scene,
}

impl PartialEq for Props {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.scene, &other.scene)
  }
}

struct Page;

impl Component for Page {
  type Props = Props;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let scene = ctx.props::<Props>().scene.clone();
    scene(ctx)
  }
}

/// A stage's caret stands still: its passes move the runtime's clock by whole seconds, which the caret's blink
/// follows, so a blinking caret would be shown or not by how many passes a test made. A test of the blink sets the
/// theme's caret mode back.
fn still_caret(app: &App) {
  app.theme().set_caret_mode(lurq::app::theme::CaretMode::Persistent);
}

pub struct Stage {
  pub app: App,
  pub tree: Tree,
  /// The clock the tree's transitions run on, which every pass moves on.
  now: Instant,
  /// Where [`presses_at`](Self::presses_at) counts the time of presses from.
  press_start: Instant,
  /// Whether the timers started on the stage count from its clock too ([`Stage::own_timers`]).
  timers: bool,
}

/// A stage that had its timers on its own clock gives the thread's back to the wall clock.
impl Drop for Stage {
  fn drop(&mut self) {
    if self.timers {
      self.tree.set_timer_clock(None);
    }
  }
}

impl Stage {
  /// A window of 400 by 300 that renders `scene` in the app's theme.
  pub fn new(scene: impl Fn(&mut Ctx) -> Element + Send + Sync + 'static) -> Self {
    Self::in_app(App::new(), scene)
  }

  /// A window as [`Stage::new`] makes it, whose canvases are drawn on the CPU, so that a test reads their pixels.
  #[cfg(feature = "canvas")]
  #[allow(dead_code)]
  pub fn software(scene: impl Fn(&mut Ctx) -> Element + Send + Sync + 'static) -> Self {
    Self::software_in(App::new(), scene)
  }

  /// As [`Stage::software`], in `app`: for a theme's recipe that changes what a canvas draws.
  #[cfg(feature = "canvas")]
  #[allow(dead_code)]
  pub fn software_in(app: App, scene: impl Fn(&mut Ctx) -> Element + Send + Sync + 'static) -> Self {
    let mut tree = Tree::new();
    tree.set_software_canvases(true);
    Self::with_tree(app, tree, scene)
  }

  pub fn in_app(app: App, scene: impl Fn(&mut Ctx) -> Element + Send + Sync + 'static) -> Self {
    Self::with_tree(app, Tree::new(), scene)
  }

  fn with_tree(mut app: App, mut tree: Tree, scene: impl Fn(&mut Ctx) -> Element + Send + Sync + 'static) -> Self {
    still_caret(&app);
    tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 300.0))));
    tree.mount_root::<Page>(&mut app, Props { scene: Arc::new(scene) });
    let mut stage = Self {
      app,
      tree,
      now: Instant::now(),
      press_start: Instant::now(),
      timers: false,
    };
    stage.pass();
    stage
  }

  /// A window of `width` by `height` that renders `scene`, with one frame painted and no more: what the first frame
  /// shows, before anything settles in a second one.
  pub fn first_frame(width: f32, height: f32, scene: impl Fn(&mut Ctx) -> Element + Send + Sync + 'static) -> Self {
    Self::first_frame_in(App::new(), width, height, scene)
  }

  /// As [`first_frame`](Self::first_frame), in `app`.
  pub fn first_frame_in(
    mut app: App,
    width: f32,
    height: f32,
    scene: impl Fn(&mut Ctx) -> Element + Send + Sync + 'static,
  ) -> Self {
    still_caret(&app);
    let mut tree = Tree::new();
    tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(width, height))));
    tree.resize(width as u32, height as u32);
    tree.mount_root::<Page>(&mut app, Props { scene: Arc::new(scene) });
    let mut stage = Self {
      app,
      tree,
      now: Instant::now(),
      press_start: Instant::now(),
      timers: false,
    };
    stage.frame();
    stage
  }

  /// What the last frame painted, quad by quad: where each is, what it is and the words it draws, to tell one frame
  /// from another.
  pub fn picture(&self) -> Vec<String> {
    self
      .quads()
      .into_iter()
      .map(|quad| {
        let what = match &quad.content {
          QuadContent::Text { text, .. } => format!("text {text:?}"),
          QuadContent::RichText { spans, .. } => {
            format!(
              "rich {:?}",
              spans.iter().map(|span| span.text.as_str()).collect::<String>()
            )
          }
          QuadContent::Rect { color, .. } => format!("rect {color:?}"),
          _ => "other".to_owned(),
        };
        // The width of each side of its border: a line between two groups is one side of one.
        let sides = quad.border.as_ref().map(|border| {
          [&border.top, &border.right, &border.bottom, &border.left]
            .map(|side| side.as_ref().map_or(0.0, |side| side.width))
        });
        format!(
          "{:.1},{:.1} {:.1}x{:.1} {what} border {sides:?}",
          quad.x, quad.y, quad.width, quad.height
        )
      })
      .collect()
  }

  /// Puts the timers started from now on (a `Timeout`, an `Interval`) on the stage's clock: each counts its
  /// delay from the clock of the frame before it started, and [`tick_timers`](Self::tick_timers) fires those
  /// that have run out by a moment of that clock. Which of two timers has run out then does not depend on how
  /// long the machine took between them: for a test that ticks one timer and not another.
  #[allow(dead_code)]
  pub fn own_timers(&mut self) {
    self.timers = true;
    self.tree.set_timer_clock(Some(self.now));
  }

  /// Fires the timers that have run out `later` after the stage's clock: see [`own_timers`](Self::own_timers).
  #[allow(dead_code)]
  pub fn tick_timers(&mut self, later: Duration) {
    self.tree.tick_timers_at(self.now + later);
  }

  /// The clock of a frame, which the timers started after it count from where they are the stage's.
  fn clock(&mut self, now: Instant) {
    self.tree.set_animation_clock(Some(now));
    if self.timers {
      self.tree.set_timer_clock(Some(now));
    }
  }

  /// Renders, lays out and paints one frame, a sixtieth of a second after the last.
  pub fn frame(&mut self) {
    self.clock(self.now);
    self.tree.request_redraw();
    self.tree.pass(&mut self.app, &TestSurface);
    self.now += Duration::from_millis(16);
  }

  /// Renders, lays out and paints what changed, then lets a second go by and
  /// paints again: a transition of a state style that the change started has
  /// run to its end.
  pub fn pass(&mut self) {
    for _ in 0..2 {
      self.clock(self.now);
      self.tree.request_redraw();
      self.tree.pass(&mut self.app, &TestSurface);
      self.now += Duration::from_secs(1);
    }
  }

  /// The quad that draws the border of the element with `id`: with content
  /// in the element, its border is painted over the content, apart from its fill.
  pub fn border(&mut self, id: &str) -> Option<Quad> {
    let (x, y, width, height) = self.bounds(id);
    self
      .quads()
      .into_iter()
      .find(|quad| quad.border.is_some() && (quad.x, quad.y, quad.width, quad.height) == (x, y, width, height))
  }

  pub fn quads(&self) -> Vec<Quad> {
    self.tree.resolve_quads(self.tree.last_layout().unwrap())
  }

  /// The box of the element with `id`: x, y, width, height.
  pub fn bounds(&mut self, id: &str) -> (f32, f32, f32, f32) {
    let rect = self
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.bounds())
      .unwrap_or_else(|| panic!("#{id} should be laid out"));
    (rect.x, rect.y, rect.width, rect.height)
  }

  /// The filled rectangle at the box of the element with `id`.
  pub fn fill(&mut self, id: &str) -> Option<Quad> {
    let (x, y, width, height) = self.bounds(id);
    self.quads().into_iter().find(|quad| {
      matches!(&quad.content, QuadContent::Rect { color, .. } if color.a() > 0)
        && (quad.x, quad.y, quad.width, quad.height) == (x, y, width, height)
    })
  }

  /// The color the element with `id` is filled with.
  pub fn fill_color(&mut self, id: &str) -> Option<Color> {
    self.fill(id).and_then(|quad| match quad.content {
      QuadContent::Rect { color, .. } => Some(color),
      _ => None,
    })
  }

  /// The color and the size of the text `content` as painted.
  pub fn text(&self, content: &str) -> (Color, f32) {
    self
      .quads()
      .iter()
      .find_map(|quad| match &quad.content {
        QuadContent::Text { text, style, .. } if text == content => Some((style.color, style.font_size)),
        _ => None,
      })
      .unwrap_or_else(|| panic!("no text {content:?} was painted"))
  }

  /// The font family of the text `content` as painted.
  pub fn font_of(&self, content: &str) -> String {
    self
      .quads()
      .iter()
      .find_map(|quad| match &quad.content {
        QuadContent::Text { text, style, .. } if text == content => Some(style.font_family.to_string()),
        _ => None,
      })
      .unwrap_or_else(|| panic!("no text {content:?} was painted"))
  }

  /// Whether a line of `color` is painted around something: an outline.
  pub fn has_ring(&self, color: Color) -> bool {
    self.quads().iter().any(|quad| {
      matches!(&quad.content, QuadContent::Rect { color: fill, gradient: None } if fill.a() == 0)
        && quad.border.and_then(|border| border.top).map(|side| side.color) == Some(color)
    })
  }

  pub fn center(&mut self, id: &str) -> (f32, f32) {
    let (x, y, width, height) = self.bounds(id);
    (x + width / 2.0, y + height / 2.0)
  }

  pub fn hover(&mut self, id: &str) {
    let (x, y) = self.center(id);
    self.tree.mouse_move(x, y);
    self.pass();
  }

  pub fn click(&mut self, id: &str) {
    let (x, y) = self.center(id);
    self.tree.mouse_move(x, y);
    self.tree.mouse_down(x, y, MouseButton::Left);
    self.tree.mouse_up(x, y, MouseButton::Left);
    self.pass();
  }

  /// Gives the presses from here on at `at` after the stage's start, on the tree's clock of presses and not the
  /// wall clock's, until called again: two presses at one time are a double press however slow the machine, and two
  /// a double press's while apart are two presses however fast. Presses before the first call are on the wall clock.
  pub fn presses_at(&mut self, at: Duration) {
    self.tree.set_press_clock(Some(self.press_start + at));
  }

  pub fn key(&mut self, key: &str) {
    self.tree.key_down(key.to_owned(), key.to_owned(), false, false, false);
    self.tree.key_up(key.to_owned(), key.to_owned(), false, false, false);
    self.pass();
  }
}

impl Stage {
  /// The quad of the text `content` as painted.
  pub fn text_quad(&self, content: &str) -> Option<Quad> {
    self
      .quads()
      .into_iter()
      .find(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text == content))
  }

  /// The quad of the text `content` as painted whole, or cut with an
  /// ellipsis: its start and the ellipsis.
  pub fn cut_text_quad(&self, content: &str) -> Option<Quad> {
    self.quads().into_iter().find(|quad| {
      matches!(&quad.content, QuadContent::Text { text, .. }
        if text == content || text.strip_suffix('…').is_some_and(|start| !start.trim_end().is_empty() && content.starts_with(start.trim_end())))
    })
  }

  /// Whether the text `content` is painted.
  pub fn paints(&self, content: &str) -> bool {
    self.text_quad(content).is_some()
  }

  /// Paints once more, `later` after the last pass: for what moves on its own.
  pub fn pass_after(&mut self, later: Duration) {
    self.clock(self.now + later);
    self.tree.request_redraw();
    self.tree.pass(&mut self.app, &TestSurface);
  }
}
