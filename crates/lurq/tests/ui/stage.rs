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

pub struct Stage {
  pub app: App,
  pub tree: Tree,
  /// The clock the tree's transitions run on, which every pass moves on.
  now: Instant,
}

impl Stage {
  /// A window of 400 by 300 that renders `scene` in the app's theme.
  pub fn new(scene: impl Fn(&mut Ctx) -> Element + Send + Sync + 'static) -> Self {
    Self::in_app(App::new(), scene)
  }

  pub fn in_app(mut app: App, scene: impl Fn(&mut Ctx) -> Element + Send + Sync + 'static) -> Self {
    let mut tree = Tree::new();
    tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 300.0))));
    tree.mount_root::<Page>(&mut app, Props { scene: Arc::new(scene) });
    let mut stage = Self {
      app,
      tree,
      now: Instant::now(),
    };
    stage.pass();
    stage
  }

  /// Renders, lays out and paints what changed, then lets a second go by and
  /// paints again: a transition of a state style that the change started has
  /// run to its end.
  pub fn pass(&mut self) {
    for _ in 0..2 {
      self.tree.set_animation_clock(Some(self.now));
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

  pub fn key(&mut self, key: &str) {
    self.tree.key_down(key.to_owned(), key.to_owned(), false, false, false);
    self.tree.key_up(key.to_owned(), key.to_owned(), false, false, false);
    self.pass();
  }
}
