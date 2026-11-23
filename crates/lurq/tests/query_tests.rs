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

// A mutation in a component: its state through renders, and a run that outlives its component.

mod tasks {
  use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
  };

  /// A service with one task's title, whose answers to a rename wait to be let go.
  #[derive(Default)]
  pub struct Service {
    pub title: Mutex<String>,
    pub reads: Mutex<usize>,
    pub renames: Mutex<Vec<String>>,
    answer: Mutex<Option<Result<(), String>>>,
    waker: Mutex<Option<Waker>>,
  }

  impl Service {
    pub fn answer(&self, answer: Result<(), String>) {
      *self.answer.lock().unwrap() = Some(answer);
      if let Some(waker) = self.waker.lock().unwrap().take() {
        waker.wake();
      }
    }
  }

  struct Answer(Arc<Service>);
  impl Future for Answer {
    type Output = Result<(), String>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
      *self.0.waker.lock().unwrap() = Some(cx.waker().clone());
      self.0.answer.lock().unwrap().take().map_or(Poll::Pending, Poll::Ready)
    }
  }

  #[derive(Clone)]
  pub struct Api(pub Arc<Service>);
  impl PartialEq for Api {
    fn eq(&self, other: &Self) -> bool {
      Arc::ptr_eq(&self.0, &other.0)
    }
  }
  impl Eq for Api {}
  impl std::hash::Hash for Api {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
      Arc::as_ptr(&self.0).hash(state);
    }
  }

  #[lurq::query]
  pub async fn get_title(api: Api) -> Result<String, String> {
    *api.0.reads.lock().unwrap() += 1;
    let title = api.0.title.lock().unwrap().clone();
    Ok(title)
  }

  #[lurq::mutation(invalidates = [get_title::all()])]
  pub async fn rename(api: Api, title: String) -> Result<String, String> {
    api.0.renames.lock().unwrap().push(title.clone());
    Answer(api.0.clone()).await?;
    *api.0.title.lock().unwrap() = title.clone();
    Ok(title)
  }
}

type Rename = std::sync::Arc<dyn Fn(&str) -> bool + Send + Sync>;

#[derive(Clone)]
struct TaskProps {
  client: QueryClient,
  api: tasks::Api,
  shown: Signal<bool>,
  /// The row's rename, as a press handler would hold it.
  rename: std::sync::Arc<std::sync::Mutex<Option<Rename>>>,
}
impl PartialEq for TaskProps {
  fn eq(&self, other: &Self) -> bool {
    self.client == other.client && self.api == other.api && self.shown.id() == other.shown.id()
  }
}
#[cfg(feature = "devtools")]
impl lurq::app::component::DevtoolsInspectable for TaskProps {
  fn write_info(&self, _: &mut Vec<lurq::app::component::ComponentInfo>) {}
}

struct TaskRow;
impl Component for TaskRow {
  type Props = TaskProps;
  fn create(_: &mut Ctx) -> Self {
    Self
  }
  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<TaskProps>().clone();
    let title = ctx.query(tasks::get_title(props.api.clone()));
    // Shown at once, and taken back if the service refuses.
    let rename = ctx
      .mutation(tasks::rename)
      .optimistic(|client, (api, title)| {
        let before = client.get_data(tasks::get_title(api.clone()));
        client.update_data(tasks::get_title(api.clone()), |_| title.clone());
        before
      })
      .on_error(|client, (api, _), _, before| {
        if let Some(before) = before {
          client.set_data(tasks::get_title(api.clone()), (*before).clone());
        }
      });
    let run = rename.clone();
    let api = props.api.clone();
    *props.rename.lock().unwrap() = Some(std::sync::Arc::new(move |title: &str| {
      run.run((api.clone(), title.to_owned()))
    }));
    Text::new(&format!(
      "{}|{}|{}",
      title.data().map_or("loading".to_owned(), |title| (*title).clone()),
      if rename.pending() { "saving" } else { "idle" },
      rename.error().map_or(String::new(), |error| (*error).clone()),
    ))
  }
}

struct TaskScreen;
impl Component for TaskScreen {
  type Props = TaskProps;
  fn create(ctx: &mut Ctx) -> Self {
    ctx.provide(ctx.props::<TaskProps>().client.clone());
    Self
  }
  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<TaskProps>().clone();
    let mut column = Column::new();
    if props.shown.get() {
      column = column.child(ctx.mount_keyed::<TaskRow>("row", props));
    }
    column
  }
}

#[test]
fn a_components_mutation_keeps_its_state_through_renders_and_finishes_without_its_component() {
  let service = std::sync::Arc::new(tasks::Service::default());
  *service.title.lock().unwrap() = "Draft".to_owned();
  let api = tasks::Api(service.clone());
  let client = QueryClient::new();
  let shown = Signal::new(true);
  let rename = std::sync::Arc::new(std::sync::Mutex::new(None));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<TaskScreen>(
    &mut app,
    TaskProps {
      client: client.clone(),
      api: api.clone(),
      shown: shown.clone(),
      rename: rename.clone(),
    },
  );
  let row = |tree: &Tree| {
    let root = tree.root().unwrap();
    root
      .children()
      .iter()
      .next()
      .and_then(|row| row.text_content().map(str::to_owned))
  };
  let run = |title: &str| {
    let rename: Rename = rename.lock().unwrap().clone().expect("the row rendered");
    rename(title)
  };
  tree.tick_futures();
  assert_eq!(row(&tree).as_deref(), Some("Draft|idle|"));
  assert_eq!(*service.reads.lock().unwrap(), 1);

  // The new title shows while the service has not answered, and the row says it is saving.
  assert!(run("Plan"));
  tree.tick_futures();
  assert_eq!(row(&tree).as_deref(), Some("Plan|saving|"));
  assert_eq!(*service.renames.lock().unwrap(), ["Plan"]);
  // A render in between keeps the mutation and its state.
  tree.request_redraw();
  tree.pass(&mut app, &support::TestSurface);
  assert_eq!(row(&tree).as_deref(), Some("Plan|saving|"));

  // Refused: the title is taken back, and the error is the row's.
  service.answer(Err("no rights".to_owned()));
  tree.tick_futures();
  assert_eq!(row(&tree).as_deref(), Some("Draft|idle|no rights"));
  assert_eq!(*service.reads.lock().unwrap(), 1, "nothing was fetched for any of it");

  // Accepted: the next run clears the error, and its success fetches what the definition invalidates.
  assert!(run("Plan"));
  tree.tick_futures();
  assert_eq!(row(&tree).as_deref(), Some("Plan|saving|"));
  service.answer(Ok(()));
  tree.tick_futures();
  tree.tick_futures();
  assert_eq!(row(&tree).as_deref(), Some("Plan|idle|"));
  assert_eq!(*service.reads.lock().unwrap(), 2);

  // A run on its way when its component unmounts finishes, and the cache has its effect.
  assert!(run("Shipped"));
  tree.tick_futures();
  shown.set(false);
  tree.request_redraw();
  tree.pass(&mut app, &support::TestSurface);
  assert_eq!(row(&tree), None);
  service.answer(Ok(()));
  tree.tick_futures();
  assert_eq!(*service.title.lock().unwrap(), "Shipped");
  assert_eq!(*service.renames.lock().unwrap(), ["Plan", "Plan", "Shipped"]);
  // Nothing observes the title, so it is stale and not fetched; the row fetches it when it is back.
  assert_eq!(*service.reads.lock().unwrap(), 2);
  shown.set(true);
  tree.request_redraw();
  tree.pass(&mut app, &support::TestSurface);
  tree.tick_futures();
  assert_eq!(row(&tree).as_deref(), Some("Shipped|idle|"));
  assert_eq!(*service.reads.lock().unwrap(), 3);
}
