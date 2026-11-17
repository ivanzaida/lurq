//! A theme for a subtree (`Ctx::in_theme`) whose root is a mounted component: the scope is on the component's root,
//! which the component's own renders replace. What the component and the components in it paint takes the scope's
//! palette at its first render, after it renders again by itself, and after the component around it does; also
//! where the component's root is a logical wrapper with no box of its own (a form's is).

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App, Tree,
    component::Component,
    ctx::Ctx,
    theme::{PaletteColor, Theme},
  },
  components::{Column, Rect},
  core::Signal,
  layout::{Constraints, Size, quad::QuadContent},
  node::{Element, color::Color},
};

use crate::support::TestSurface;

const PRIMARY: &str = "#ff0000";
const MUTED: &str = "#00ff00";

/// The signals the test turns to render a component again: the panel's own, and the host's.
#[derive(Clone, Default)]
struct Handles {
  /// The panel's root is a form's, a logical wrapper with no box around its column.
  logical: bool,
  panel: Arc<Mutex<Option<Signal<u32>>>>,
  host: Arc<Mutex<Option<Signal<u32>>>>,
}

#[cfg(feature = "devtools")]
impl lurq::app::component::DevtoolsInspectable for Handles {
  fn write_info(&self, _buffer: &mut Vec<lurq::app::component::ComponentInfo>) {}
}

impl PartialEq for Handles {
  fn eq(&self, other: &Self) -> bool {
    self.logical == other.logical && Arc::ptr_eq(&self.panel, &other.panel)
  }
}

/// A component in the panel, with a role of its own.
struct Swatch;

impl Component for Swatch {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    Rect::new(10.0, 10.0).id("nested").background(PaletteColor::Primary)
  }
}

/// The scope's root: a panel in `Muted` holding a rect in `Primary` and a component.
struct Panel {
  renders: Signal<u32>,
}

impl Component for Panel {
  type Props = Handles;

  fn create(ctx: &mut Ctx) -> Self {
    let renders = ctx.signal(0);
    *ctx.props::<Handles>().panel.lock().unwrap() = Some(renders.clone());
    Self { renders }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    // Read, so that a change of it renders the panel again, and the panel alone.
    let renders = self.renders.get();
    let column = Column::new()
      .id("panel")
      .padding(4.0 + (renders % 2) as f32)
      .background(PaletteColor::Muted)
      .child(Rect::new(20.0, 10.0).id("inside").background(PaletteColor::Primary))
      .child(ctx.mount::<Swatch>(()));
    #[cfg(feature = "form")]
    if ctx.props::<Handles>().logical {
      let form = ctx.form(lurq::components::FormOptions::new());
      return ctx.form_view(form, |_| column);
    }
    column.into()
  }
}

struct Host {
  scoped: Theme,
  renders: Signal<u32>,
}

impl Component for Host {
  type Props = Handles;

  fn create(ctx: &mut Ctx) -> Self {
    let scoped = Theme::new();
    scoped.set_palette_color(PaletteColor::Primary, Color::from_hex(PRIMARY));
    scoped.set_palette_color(PaletteColor::Muted, Color::from_hex(MUTED));
    let renders = ctx.signal(0);
    *ctx.props::<Handles>().host.lock().unwrap() = Some(renders.clone());
    Self { scoped, renders }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let _ = self.renders.get();
    let handles = ctx.props::<Handles>().clone();
    let panel = ctx.in_theme(&self.scoped, |ctx| ctx.mount::<Panel>(handles));
    Column::new()
      .child(panel)
      .child(Rect::new(20.0, 10.0).id("outside").background(PaletteColor::Primary))
  }
}

fn pass(tree: &mut Tree, app: &mut App) {
  tree.request_redraw();
  tree.pass(app, &TestSurface);
}

/// The color the element with `id` is filled with.
fn fill(tree: &mut Tree, id: &str) -> Color {
  let rect = tree
    .get_element_by_id_mut(id)
    .and_then(|element| element.bounds())
    .unwrap_or_else(|| panic!("#{id} is laid out"));
  tree
    .resolve_quads(tree.last_layout().unwrap())
    .into_iter()
    .find_map(|quad| match quad.content {
      QuadContent::Rect { color, .. }
        if color.a() > 0 && (quad.x, quad.y, quad.width, quad.height) == (rect.x, rect.y, rect.width, rect.height) =>
      {
        Some(color)
      }
      _ => None,
    })
    .unwrap_or_else(|| panic!("#{id} is filled"))
}

fn assert_scoped(tree: &mut Tree, when: &str) {
  assert_eq!(fill(tree, "panel").to_hex(), MUTED, "the component's root, {when}");
  assert_eq!(fill(tree, "inside").to_hex(), PRIMARY, "what it holds, {when}");
  assert_eq!(fill(tree, "nested").to_hex(), PRIMARY, "a component in it, {when}");
  assert_ne!(
    fill(tree, "outside").to_hex(),
    PRIMARY,
    "what is outside the scope, {when}"
  );
}

fn renders_in_the_scope(logical: bool) {
  let mut app = App::new();
  let handles = Handles {
    logical,
    ..Handles::default()
  };
  let mut tree = Tree::new();
  tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(200.0, 200.0))));
  tree.mount_root::<Host>(&mut app, handles.clone());
  pass(&mut tree, &mut app);
  assert_scoped(&mut tree, "at its first render");

  let panel = handles.panel.lock().unwrap().clone().unwrap();
  panel.update(|renders| *renders += 1);
  pass(&mut tree, &mut app);
  assert_scoped(&mut tree, "after it rendered again by itself");

  let host = handles.host.lock().unwrap().clone().unwrap();
  host.update(|renders| *renders += 1);
  pass(&mut tree, &mut app);
  assert_scoped(&mut tree, "after the host rendered again");

  panel.update(|renders| *renders += 1);
  pass(&mut tree, &mut app);
  assert_scoped(&mut tree, "after it rendered again by itself once more");
}

#[test]
fn a_mounted_component_at_the_root_of_a_scope_paints_in_the_scopes_theme() {
  renders_in_the_scope(false);
}

#[cfg(feature = "form")]
#[test]
fn a_mounted_component_whose_root_is_a_logical_wrapper_paints_in_the_scopes_theme() {
  renders_in_the_scope(true);
}
