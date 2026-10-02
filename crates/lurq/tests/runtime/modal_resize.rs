//! Layers sized against the window (modals, nested modals, anchored popups)
//! must follow the current window size when the component that declares them
//! re-renders after the window was resized back to an earlier size.
//!
//! Each layer node is rebuilt every pass and takes over the layout caches of
//! the previous pass's layer, so after 1440x960 -> 960x700 -> 1440x960 its
//! content holds results for both sizes. A re-render must lay the changed
//! content out under the constraints of the current size.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx},
  components::{Column, Modal, ModalTarget, Placement, Popup, Rect, Stack, Text},
  core::{ElementRef, Signal},
  node::{Element, dimension::Dimension},
};

#[derive(Clone, Copy)]
enum Scenario {
  RootModal,
  ParentModal,
  ElementModal,
  NestedModal,
  Popup,
}

struct Case {
  scenario: Scenario,
  tick: Mutex<Option<Signal<u32>>>,
  shown: Mutex<Option<Signal<bool>>>,
}

struct Shared(Arc<Case>);

#[cfg(feature = "devtools")]
impl lurq::app::component::DevtoolsInspectable for Shared {
  fn write_info(&self, _buffer: &mut Vec<lurq::app::component::ComponentInfo>) {}
}

impl Clone for Shared {
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }
}

impl PartialEq for Shared {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

fn full() -> Dimension {
  Dimension::Pct(100.0)
}

/// A window-filling column `id` with a full-width row `{id}-row` holding the
/// changing text: the row is a descendant the layer's cache hit skips.
fn full_layer(id: &str, tick: u32) -> Column {
  Column::new().id(id).width(full()).height(full()).child(row(id, tick))
}

fn row(id: &str, tick: u32) -> Column {
  Column::new()
    .id(format!("{id}-row"))
    .width(full())
    .child(Text::new(&tick.to_string()))
}

/// Declares the layer itself and re-renders on its own signal.
struct Owner {
  scenario: Scenario,
  tick: Signal<u32>,
  target: ElementRef,
}

impl Component for Owner {
  type Props = Shared;

  fn create(ctx: &mut Ctx) -> Self {
    let tick = ctx.signal(0);
    let case = ctx.props::<Self::Props>().0.clone();
    *case.tick.lock().unwrap() = Some(tick.clone());
    Self {
      scenario: case.scenario,
      tick,
      target: ElementRef::new(),
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    let tick = self.tick.get();
    match self.scenario {
      Scenario::RootModal => Element::from(Modal::new(full_layer("layer", tick)).target(ModalTarget::Root)),
      Scenario::ParentModal => Column::new()
        .width(full())
        .height(full())
        .child(Modal::new(full_layer("layer", tick)).target(ModalTarget::Parent))
        .into(),
      Scenario::ElementModal => Column::new()
        .width(full())
        .height(full())
        .child(
          Column::new()
            .width(full())
            .height(full())
            .ref_element(self.target.clone()),
        )
        .child(Modal::new(full_layer("layer", tick)).target(self.target.clone()))
        .into(),
      Scenario::NestedModal => Modal::new(
        Column::new()
          .width(full())
          .height(full())
          .child(Modal::new(full_layer("layer", tick)).target(ModalTarget::Root)),
      )
      .target(ModalTarget::Root)
      .into(),
      Scenario::Popup => Column::new()
        .width(full())
        .height(full())
        .child(Rect::new(120.0, 40.0).ref_element(self.target.clone()))
        .child(
          Popup::new(
            self.target.clone(),
            Stack::new()
              .id("layer")
              .width(full())
              .height(full())
              .child(Stack::new().width(full()).child(row("layer", tick))),
          )
          .open_when(true)
          .placement(Placement::BottomStart),
        )
        .into(),
    }
  }
}

/// Mounts the owner offstage-capable, so a test can take it off the tree and
/// bring it back.
struct Page;

impl Component for Page {
  type Props = Shared;

  fn create(ctx: &mut Ctx) -> Self {
    let shown = ctx.signal(true);
    *ctx.props::<Self::Props>().0.shown.lock().unwrap() = Some(shown);
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Self::Props>().clone();
    let shown = props.0.shown.lock().unwrap().as_ref().expect("page created").get();
    Column::new()
      .width(full())
      .height(full())
      .child(ctx.mount_offstage::<Owner>(props, shown))
  }
}

struct Harness {
  app: App,
  tree: Tree,
  case: Arc<Case>,
}

impl Harness {
  fn mount(scenario: Scenario) -> Self {
    let case = Arc::new(Case {
      scenario,
      tick: Mutex::new(None),
      shown: Mutex::new(None),
    });
    let mut app = App::new();
    let mut tree = Tree::new();
    tree.resize(1440, 960);
    tree.mount_root::<Page>(&mut app, Shared(case.clone()));
    tree.pass_headless(&mut app);
    Self { app, tree, case }
  }

  fn resize(&mut self, width: u32, height: u32) {
    self.tree.resize(width, height);
    self.tree.pass_headless(&mut self.app);
  }

  /// 1440x960 -> 960x700 -> 1440x960, a pass after each step.
  fn resize_back_and_forth(&mut self) {
    for (width, height) in [(1440, 960), (960, 700), (1440, 960)] {
      self.resize(width, height);
    }
  }

  fn tick(&mut self) {
    let tick = self.case.tick.lock().unwrap().clone().expect("owner created");
    tick.update(|value| *value += 1);
    self.tree.pass_headless(&mut self.app);
  }

  fn show(&mut self, shown: bool) {
    self
      .case
      .shown
      .lock()
      .unwrap()
      .as_ref()
      .expect("page created")
      .set(shown);
    self.tree.pass_headless(&mut self.app);
  }

  fn size(&mut self, id: &str) -> (f32, f32) {
    let bounds = self
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.bounds())
      .unwrap_or_else(|| panic!("{id} is laid out"));
    (bounds.width, bounds.height)
  }

  /// The layer spans the window and so does its row.
  fn assert_layer_fills(&mut self, width: f32, height: f32, step: &str) {
    assert_eq!(self.size("layer"), (width, height), "{step}: layer size");
    assert_eq!(self.size("layer-row").0, width, "{step}: row width");
  }
}

fn assert_follows_window_after_rerender(scenario: Scenario) {
  let mut harness = Harness::mount(scenario);
  harness.resize_back_and_forth();
  harness.assert_layer_fills(1440.0, 960.0, "after resizes");

  harness.tick();
  harness.assert_layer_fills(1440.0, 960.0, "after re-render");

  harness.resize(960, 700);
  harness.tick();
  harness.assert_layer_fills(960.0, 700.0, "after resize and re-render");
}

#[test]
fn root_modal_follows_the_window_after_its_owner_rerenders_past_two_resizes() {
  assert_follows_window_after_rerender(Scenario::RootModal);
}

#[test]
fn parent_modal_follows_the_window_after_its_owner_rerenders_past_two_resizes() {
  assert_follows_window_after_rerender(Scenario::ParentModal);
}

#[test]
fn element_modal_follows_the_window_after_its_owner_rerenders_past_two_resizes() {
  assert_follows_window_after_rerender(Scenario::ElementModal);
}

#[test]
fn nested_modal_follows_the_window_after_its_owner_rerenders_past_two_resizes() {
  assert_follows_window_after_rerender(Scenario::NestedModal);
}

#[test]
fn popup_follows_the_window_after_its_owner_rerenders_past_two_resizes() {
  assert_follows_window_after_rerender(Scenario::Popup);
}

#[test]
fn root_modal_of_an_offstage_owner_follows_the_window_it_returns_to() {
  let mut harness = Harness::mount(Scenario::RootModal);
  harness.resize(960, 700);
  harness.show(false);
  harness.resize(1440, 960);
  harness.show(true);
  harness.assert_layer_fills(1440.0, 960.0, "after returning");

  harness.tick();
  harness.assert_layer_fills(1440.0, 960.0, "after re-render");
}
