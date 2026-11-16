//! A component built again on its own costs the component and its way to the root, not the tree: its place is
//! found by the subtrees that hold components, the nodes that did not change keep their numbers, and the column it
//! is in is patched where it changed. These are the cases where what each shortcut knows would be stale: the
//! component moved by a keyed reorder, moved under another parent (a disabled one), and unmounted in the frame it
//! is built again. Each frame is checked against the same scene built fresh.

use std::{
  cell::RefCell,
  collections::HashSet,
  sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
  },
};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx, events::MouseButton},
  components::{Button, Column, Rect, Row},
  core::Signal,
  layout::quad::QuadContent,
  node::{Element, color::Color},
};

use crate::support::{pointer_click, render_pass_with_app};

const ITEMS: usize = 6;
const GREY: Color = Color::new(160, 160, 160, 255);

#[derive(Clone)]
struct Scene {
  /// The items of the main column, in order.
  order: Signal<Vec<usize>>,
  /// An item mounted in the side column, which is disabled, in place of the main one.
  moved: Signal<Option<usize>>,
  /// An item not mounted.
  gone: Signal<Option<usize>>,
  /// Each item's height: a change builds that item again, and only it.
  heights: Arc<Vec<Signal<f32>>>,
  clicks: Arc<Vec<AtomicUsize>>,
}

impl Scene {
  fn new() -> Self {
    Self {
      order: Signal::new((0..ITEMS).collect()),
      moved: Signal::new(None),
      gone: Signal::new(None),
      heights: Arc::new((0..ITEMS).map(|item| Signal::new(20.0 + item as f32)).collect()),
      clicks: Arc::new((0..ITEMS).map(|_| AtomicUsize::new(0)).collect()),
    }
  }
}

thread_local! {
  static SCENE: RefCell<Option<Scene>> = const { RefCell::new(None) };
}

fn scene() -> Scene {
  SCENE.with(|scene| scene.borrow().clone().expect("a scene is set"))
}

struct Item;

impl Component for Item {
  type Props = usize;

  fn create(_: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let item = *ctx.props::<usize>();
    let scene = scene();
    let height = scene.heights[item].get();
    let clicks = scene.clicks.clone();
    Column::new()
      .child(
        Rect::new(40.0, height)
          .id(format!("item-{item}"))
          .background(Color::new(30, 60, 90 + item as u8 * 20, 255))
          .when_disabled(|style| style.background(GREY)),
      )
      .child(
        Button::new("press")
          .id(format!("button-{item}"))
          .size(60.0, 20.0)
          .on_click(move |_| {
            clicks[item].fetch_add(1, Ordering::SeqCst);
          }),
      )
  }
}

struct Doc;

impl Component for Doc {
  type Props = ();

  fn create(_: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let scene = scene();
    let (moved, gone) = (scene.moved.get(), scene.gone.get());
    let mut main = Column::new();
    for item in scene.order.get() {
      if Some(item) != moved && Some(item) != gone {
        main = main.child(ctx.mount_keyed::<Item>(&format!("item-{item}"), item));
      }
    }
    let mut side = Column::new().disabled(true);
    if let Some(item) = moved.filter(|item| Some(*item) != gone) {
      side = side.child(ctx.mount_keyed::<Item>(&format!("item-{item}"), item));
    }
    Row::new().child(main).child(side)
  }
}

fn mount(scene: &Scene) -> (App, Tree) {
  SCENE.with(|slot| *slot.borrow_mut() = Some(scene.clone()));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.resize(400, 600);
  tree.mount_root::<Doc>(&mut app, ());
  render_pass_with_app(&mut tree, &mut app);
  (app, tree)
}

/// Where each item is drawn.
fn places(tree: &mut Tree) -> Vec<Option<(f32, f32, f32, f32)>> {
  (0..ITEMS)
    .map(|item| {
      tree
        .get_element_by_id_mut(&format!("item-{item}"))
        .and_then(|element| element.bounds())
        .map(|bounds| (bounds.x, bounds.y, bounds.width, bounds.height))
    })
    .collect()
}

/// The tree's nodes have a number each, and no two the same.
fn assert_numbered(tree: &mut Tree, what: &str) {
  let ids = RefCell::new(Vec::new());
  tree.find_element(|element| {
    ids.borrow_mut().push(element.node_id());
    false
  });
  let ids = ids.into_inner();
  assert!(ids.iter().all(|id| id.is_assigned()), "{what}: every node has a number");
  let unique: HashSet<_> = ids.iter().collect();
  assert_eq!(unique.len(), ids.len(), "{what}: no two nodes have the same number");
}

/// The scene as `tree` shows it, against the same scene built fresh.
fn assert_as_fresh(tree: &mut Tree, scene: &Scene, what: &str) {
  let shown = places(tree);
  let (_, mut fresh) = mount(scene);
  assert_eq!(
    shown,
    places(&mut fresh),
    "{what}: each item where a fresh build puts it"
  );
}

/// What `item` is filled with: in a disabled element, its disabled look.
fn fill(tree: &mut Tree, item: usize) -> Option<Color> {
  let bounds = tree
    .get_element_by_id_mut(&format!("item-{item}"))
    .and_then(|element| element.bounds())?;
  let layout = tree.last_layout()?.clone();
  tree
    .resolve_quads(&layout)
    .into_iter()
    .find_map(|quad| match quad.content {
      QuadContent::Rect { color, .. }
        if (quad.x - bounds.x).abs() < 0.5
          && (quad.y - bounds.y).abs() < 0.5
          && (quad.width - bounds.width).abs() < 0.5
          && (quad.height - bounds.height).abs() < 0.5 =>
      {
        Some(color)
      }
      _ => None,
    })
}

/// Whether a press on `item`'s button reaches it: not where it is in the disabled column.
fn presses(tree: &mut Tree, scene: &Scene, item: usize) -> bool {
  let before = scene.clicks[item].load(Ordering::SeqCst);
  let (x, y) = tree
    .get_element_by_id_mut(&format!("button-{item}"))
    .and_then(|element| element.bounds())
    .expect("the button is laid out")
    .center();
  pointer_click(tree, x, y, MouseButton::Left);
  // The pointer away again: what it hovered does not stay lit.
  tree.mouse_move(399.0, 599.0);
  scene.clicks[item].load(Ordering::SeqCst) > before
}

#[test]
fn a_component_built_again_after_a_keyed_reorder_is_found_numbered_and_placed_where_it_moved() {
  let scene = Scene::new();
  let (mut app, mut tree) = mount(&scene);
  scene.order.set((0..ITEMS).rev().collect());
  render_pass_with_app(&mut tree, &mut app);
  assert_as_fresh(&mut tree, &scene, "reordered");

  // The item that went from first to last grows: it alone is built again, at its new place.
  scene.heights[0].set(55.0);
  render_pass_with_app(&mut tree, &mut app);
  assert_eq!(
    places(&mut tree)[0].map(|place| place.3),
    Some(55.0),
    "the item built again shows its change"
  );
  assert_as_fresh(&mut tree, &scene, "an item built again after the reorder");
  assert_numbered(&mut tree, "an item built again after the reorder");

  // One in the middle, which the reorder moved past the one that grew.
  scene.heights[3].set(8.0);
  render_pass_with_app(&mut tree, &mut app);
  assert_as_fresh(&mut tree, &scene, "a second item built again");
  assert_numbered(&mut tree, "a second item built again");
}

#[test]
fn a_component_built_again_after_it_moved_under_another_parent_is_found_there_and_is_disabled_with_it() {
  let scene = Scene::new();
  let (mut app, mut tree) = mount(&scene);
  assert!(
    presses(&mut tree, &scene, 2),
    "in the main column its button takes a press"
  );

  scene.moved.set(Some(2));
  render_pass_with_app(&mut tree, &mut app);
  assert_as_fresh(&mut tree, &scene, "moved");

  // Built again where it moved: found under its new parent, and disabled as what is in it is.
  scene.heights[2].set(48.0);
  render_pass_with_app(&mut tree, &mut app);
  assert_eq!(
    places(&mut tree)[2].map(|place| place.3),
    Some(48.0),
    "built again where it moved"
  );
  assert_as_fresh(&mut tree, &scene, "built again under its new parent");
  assert_numbered(&mut tree, "built again under its new parent");
  assert_eq!(
    fill(&mut tree, 2),
    Some(GREY),
    "built again in the disabled column, it has the disabled look"
  );
  assert_ne!(fill(&mut tree, 1), Some(GREY), "one in the main column does not");
  assert!(
    !presses(&mut tree, &scene, 2),
    "in the disabled column its button takes no press"
  );
  assert!(presses(&mut tree, &scene, 1), "one left in the main column still does");

  // Back in the main column, and built again there.
  scene.moved.set(None);
  render_pass_with_app(&mut tree, &mut app);
  scene.heights[2].set(30.0);
  render_pass_with_app(&mut tree, &mut app);
  assert_as_fresh(&mut tree, &scene, "built again back in the main column");
  assert_ne!(
    fill(&mut tree, 2),
    Some(GREY),
    "back in the main column, it is not disabled"
  );
  assert_numbered(&mut tree, "built again back in the main column");
  assert!(
    presses(&mut tree, &scene, 2),
    "back in the main column its button takes a press again"
  );
}

#[test]
fn a_component_unmounted_in_the_frame_it_is_built_again_leaves_no_trace() {
  let scene = Scene::new();
  let (mut app, mut tree) = mount(&scene);
  // Both in one frame: the item is to be built again, and its parent drops it.
  scene.heights[4].set(70.0);
  scene.gone.set(Some(4));
  render_pass_with_app(&mut tree, &mut app);
  assert_eq!(places(&mut tree)[4], None, "the item is gone");
  assert_as_fresh(&mut tree, &scene, "unmounted while built again");
  assert_numbered(&mut tree, "unmounted while built again");

  // The one after it, built again next: found where the gone one's place closed up.
  scene.heights[5].set(12.0);
  render_pass_with_app(&mut tree, &mut app);
  assert_as_fresh(&mut tree, &scene, "the next item built again");

  // Mounted again, it is new: built at its height of now.
  scene.gone.set(None);
  render_pass_with_app(&mut tree, &mut app);
  assert_eq!(places(&mut tree)[4].map(|place| place.3), Some(70.0));
  assert_as_fresh(&mut tree, &scene, "mounted again");
  assert_numbered(&mut tree, "mounted again");
}
