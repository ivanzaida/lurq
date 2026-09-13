#![cfg(feature = "query")]

use std::sync::atomic::{AtomicUsize, Ordering};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx},
  components::{Column, Text},
  core::Signal,
  node::Element,
  query::{QueryClient, QueryClientOptions},
};

mod support;

static CALLS: AtomicUsize = AtomicUsize::new(0);

#[lurq::query(stale_time = "30s")]
async fn amount(id: u64) -> Result<u64, String> {
  CALLS.fetch_add(1, Ordering::Relaxed);
  Ok(id)
}

#[derive(Clone)]
struct Props {
  client: QueryClient,
  visible: Signal<u64>,
}
impl PartialEq for Props {
  fn eq(&self, other: &Self) -> bool {
    self.client == other.client && self.visible.id() == other.visible.id()
  }
}
#[cfg(feature = "devtools")]
impl lurq::app::component::DevtoolsInspectable for Props {
  fn write_info(&self, _: &mut Vec<lurq::app::component::ComponentInfo>) {}
}

struct Reader;
impl Component for Reader {
  type Props = ();
  fn create(_: &mut Ctx) -> Self {
    Self
  }
  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let query = ctx.query(amount(42));
    Text::new(&query.data().map_or("loading".to_owned(), |value| value.to_string()))
  }
}

// Intentionally provide in a nested component to exercise provider retention
// when its parent rerenders and inherited contexts are refreshed.
struct Provider;
impl Component for Provider {
  type Props = Props;
  fn create(ctx: &mut Ctx) -> Self {
    ctx.provide(ctx.props::<Props>().client.clone());
    Self
  }
  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let count = ctx.props::<Props>().visible.get();
    let mut column = Column::new();
    for index in 0..count {
      column = column.child(ctx.mount_keyed::<Reader>(&index.to_string(), ()));
    }
    column
  }
}

struct Root;
impl Component for Root {
  type Props = Props;
  fn create(_: &mut Ctx) -> Self {
    Self
  }
  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Props>().clone();
    props.visible.get(); // Also rerender the parent to refresh inherited contexts.
    ctx.mount::<Provider>(props)
  }
}

#[test]
fn mounted_components_share_requests_and_keep_the_provider_after_navigation() {
  CALLS.store(0, Ordering::Relaxed);
  let client = QueryClient::new();
  let visible = Signal::new(2);
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<Root>(
    &mut app,
    Props {
      client: client.clone(),
      visible: visible.clone(),
    },
  );
  assert_eq!(CALLS.load(Ordering::Relaxed), 0);
  assert_eq!(client.inspect()[0].observers, 2);
  tree.tick_futures();
  assert_eq!(CALLS.load(Ordering::Relaxed), 1);
  for child in tree.root().unwrap().children() {
    assert_eq!(child.text_content(), Some("42"));
  }

  visible.set(1);
  tree.request_redraw();
  tree.pass(&mut app, &support::TestSurface);
  assert_eq!(client.inspect()[0].observers, 1);
  client.invalidate(amount::all());
  tree.tick_futures();
  assert_eq!(CALLS.load(Ordering::Relaxed), 2);

  visible.set(0);
  tree.request_redraw();
  tree.pass(&mut app, &support::TestSurface);
  assert_eq!(client.inspect()[0].observers, 0);
  client.invalidate(amount(42));
  tree.tick_futures();
  assert_eq!(CALLS.load(Ordering::Relaxed), 2);

  visible.set(1);
  tree.request_redraw();
  tree.pass(&mut app, &support::TestSurface);
  assert_eq!(client.inspect()[0].observers, 1);
  tree.tick_futures();
  assert_eq!(CALLS.load(Ordering::Relaxed), 3);
  assert_eq!(
    tree.root().unwrap().children().iter().next().unwrap().text_content(),
    Some("42")
  );
}

// No Clone/PartialEq/DevtoolsInspectable bounds on fetched values or errors.
pub struct Payload(pub u64);
pub struct Failure;

mod definitions {
  #[lurq::query(gc_time = "0s")]
  pub async fn sum(mut left: u64, right: u64) -> Result<super::Payload, super::Failure> {
    left += right;
    Ok(super::Payload(left))
  }

  #[lurq::query]
  pub async fn version() -> Result<super::Payload, super::Failure> {
    Ok(super::Payload(1))
  }

  #[lurq::query]
  #[cfg(any())]
  async fn excluded() -> Result<MissingType, MissingType> {
    todo!()
  }
}

struct MacroReader;
impl Component for MacroReader {
  type Props = ();
  fn create(ctx: &mut Ctx) -> Self {
    ctx.provide(QueryClient::with_options(QueryClientOptions::default()));
    Self
  }
  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let sum = ctx.query(definitions::sum(3, 4));
    let version = ctx.query(definitions::version());
    Text::new(&format!(
      "{}:{}",
      sum.data().map_or(0, |v| v.0),
      version.data().map_or(0, |v| v.0)
    ))
  }
}

#[test]
fn macro_preserves_visibility_module_paths_and_supports_owned_nonclone_results() {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<MacroReader>(&mut app, ());
  tree.tick_futures();
  assert_eq!(tree.root().unwrap().text_content(), Some("7:1"));
  let client = QueryClient::new();
  client.invalidate(definitions::sum::all());
  client.invalidate(definitions::version::all());
}

static SHARED_CALLS: AtomicUsize = AtomicUsize::new(0);

#[lurq::query]
async fn shared_amount() -> Result<usize, String> {
  Ok(SHARED_CALLS.fetch_add(1, Ordering::Relaxed) + 1)
}

struct SharedRoot;
impl Component for SharedRoot {
  type Props = QueryClient;
  fn create(ctx: &mut Ctx) -> Self {
    ctx.provide(ctx.props::<QueryClient>().clone());
    Self
  }
  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let query = ctx.query(shared_amount());
    Text::new(&query.data().map_or("loading".to_owned(), |value| value.to_string()))
  }
}

#[test]
fn cloning_shares_cache_across_trees_and_new_clients_remain_isolated() {
  SHARED_CALLS.store(0, Ordering::Relaxed);
  let client = QueryClient::new();
  let mut app = App::new();
  let mut first = Tree::new();
  let mut second = Tree::new();
  first.mount_root::<SharedRoot>(&mut app, client.clone());
  second.mount_root::<SharedRoot>(&mut app, client.clone());
  first.tick_futures();
  assert_eq!(first.root().unwrap().text_content(), Some("1"));
  assert_eq!(second.root().unwrap().text_content(), Some("loading"));
  second.tick_futures();
  assert_eq!(second.root().unwrap().text_content(), Some("1"));
  assert_eq!(SHARED_CALLS.load(Ordering::Relaxed), 1);
  assert_eq!(client.inspect()[0].observers, 2);

  client.invalidate(shared_amount::all());
  second.tick_futures();
  assert_eq!(second.root().unwrap().text_content(), Some("2"));
  assert_eq!(first.root().unwrap().text_content(), Some("1"));
  first.tick_futures();
  assert_eq!(first.root().unwrap().text_content(), Some("2"));
  drop(first);
  client.invalidate(shared_amount());
  second.tick_futures();
  assert_eq!(second.root().unwrap().text_content(), Some("3"));
  assert_eq!(client.inspect()[0].observers, 1);

  let isolated = QueryClient::new();
  let mut third = Tree::new();
  third.mount_root::<SharedRoot>(&mut app, isolated.clone());
  third.tick_futures();
  assert_eq!(third.root().unwrap().text_content(), Some("4"));
  isolated.invalidate(shared_amount::all());
  third.tick_futures();
  second.tick_futures();
  assert_eq!(third.root().unwrap().text_content(), Some("5"));
  assert_eq!(second.root().unwrap().text_content(), Some("3"));
  drop(second);
  let mut later = Tree::new();
  later.mount_root::<SharedRoot>(&mut app, client);
  later.tick_futures();
  assert_eq!(later.root().unwrap().text_content(), Some("3"));
  assert_eq!(SHARED_CALLS.load(Ordering::Relaxed), 5);
}
