//! A router mounted offstage keeps its routed page's scroll offset.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx, events::ScrollPhase},
  components::{Column, Rect, Router, ScrollVertical},
  core::Signal,
  node::{Element, color::Color},
  router::{RouterHandle, Routes},
};

#[derive(Clone, lurq::DevtoolsInspectable)]
struct TabsProps {
  #[devtools_ignore]
  active: Signal<usize>,
  /// Receives tab a's router.
  #[devtools_ignore]
  router_a: Arc<Mutex<Option<RouterHandle>>>,
}

impl PartialEq for TabsProps {
  fn eq(&self, other: &Self) -> bool {
    self.active.id() == other.active.id() && Arc::ptr_eq(&self.router_a, &other.router_a)
  }
}

fn list(name: &'static str) -> Element {
  let mut rows = Column::new();
  for index in 0..20 {
    rows = rows.child(
      Rect::new(100.0, 50.0)
        .id(format!("{name}-row-{index}"))
        .background(Color::new(index as u8, 0, 0, 255)),
    );
  }
  ScrollVertical::new(rows)
    .id(format!("{name}-scroll"))
    .height(200.0)
    .into()
}

struct Tabs {
  routers: [RouterHandle; 2],
}

impl Component for Tabs {
  type Props = TabsProps;

  fn create(ctx: &mut Ctx) -> Self {
    let routers = ["a", "b"].map(|name| {
      let router = ctx.router(Routes::new().route("/list", move |_ctx| list(name)));
      router.push("/list");
      router
    });
    *ctx.props::<TabsProps>().router_a.lock().unwrap() = Some(routers[0].clone());
    Self { routers }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let active = ctx.props::<TabsProps>().active.get();
    let mut column = Column::new();
    for (index, router) in self.routers.iter().enumerate() {
      column = column.child(Router::mount_offstage(ctx, router.clone(), active == index));
    }
    column
  }
}

fn top(tree: &mut Tree, id: &str) -> f32 {
  tree
    .get_element_by_id_mut(id)
    .and_then(|element| element.bounds())
    .unwrap_or_else(|| panic!("#{id} should be laid out"))
    .y
}

fn pass(tree: &mut Tree, app: &mut App) {
  for _ in 0..2 {
    tree.request_redraw();
    tree.pass_headless(app);
  }
}

#[test]
fn routed_page_keeps_its_scroll_offset_across_tab_switches() {
  let active = Signal::new(0);
  let router_a = Arc::new(Mutex::new(None));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<Tabs>(
    &mut app,
    TabsProps {
      active: active.clone(),
      router_a: router_a.clone(),
    },
  );
  pass(&mut tree, &mut app);

  let scroll_top = top(&mut tree, "a-scroll");
  tree.scroll(10.0, scroll_top + 10.0, 0.0, -120.0, ScrollPhase::Scroll);
  pass(&mut tree, &mut app);
  assert_eq!(scroll_top - top(&mut tree, "a-row-0"), 120.0);
  // A query change re-renders the routed page with a new match while active.
  let router = router_a.lock().unwrap().clone().expect("tab a's router");
  router.replace("/list?sort=name");
  pass(&mut tree, &mut app);
  assert_eq!(scroll_top - top(&mut tree, "a-row-0"), 120.0);

  active.set(1);
  pass(&mut tree, &mut app);
  assert!(tree.get_element_by_id_mut("a-row-0").is_none(), "tab a is offstage");
  active.set(0);
  pass(&mut tree, &mut app);

  assert_eq!(top(&mut tree, "a-scroll") - top(&mut tree, "a-row-0"), 120.0);
}
