use std::{
  future::Future,
  panic::{self, AssertUnwindSafe},
  pin::Pin,
  sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
    mpsc,
  },
  task::{Context, Poll},
  thread,
  time::Duration,
};

use lurq::{
  app::{
    App, Tree,
    component::Component,
    ctx::{Ctx, FutureAction, FutureStatus},
  },
  components::Text,
  node::Element,
};

const RUNS: usize = 3;

/// Runs `f` on its own thread so a deadlock fails the test instead of hanging it.
fn within_deadline<R: Send + 'static>(f: impl FnOnce() -> R + Send + 'static) -> R {
  let (done, finished) = mpsc::channel();
  thread::spawn(move || {
    let result = panic::catch_unwind(AssertUnwindSafe(f));
    let _ = done.send(result);
  });
  match finished.recv_timeout(Duration::from_secs(10)) {
    Ok(Ok(value)) => value,
    Ok(Err(payload)) => panic::resume_unwind(payload),
    Err(_) => panic!("deadlock: restarting a future action from a watch on its state did not return"),
  }
}

/// Resolves on its second poll, so the action stays pending across one tick.
struct YieldOnce(bool);

impl Future for YieldOnce {
  type Output = ();

  fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
    if self.0 {
      Poll::Ready(())
    } else {
      self.0 = true;
      cx.waker().wake_by_ref();
      Poll::Pending
    }
  }
}

#[derive(Clone, Default)]
struct Probe {
  action: Arc<Mutex<Option<FutureAction<usize, usize, String>>>>,
  runs: Arc<AtomicUsize>,
  watching: Arc<AtomicBool>,
}

#[cfg(feature = "devtools")]
impl lurq::app::component::DevtoolsInspectable for Probe {
  fn write_info(&self, _buffer: &mut Vec<lurq::app::component::ComponentInfo>) {}
}

impl PartialEq for Probe {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.runs, &other.runs)
  }
}

struct RestartingAction;

impl Component for RestartingAction {
  type Props = Probe;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let probe = ctx.props::<Probe>().clone();
    let runs = probe.runs.clone();
    let action = ctx.future_action(move |run: usize| {
      runs.fetch_add(1, Ordering::SeqCst);
      async move {
        YieldOnce(false).await;
        Ok::<_, String>(run)
      }
    });
    *probe.action.lock().unwrap() = Some(action.clone());

    if !probe.watching.swap(true, Ordering::SeqCst) {
      let restart = action.clone();
      ctx.watch(&action.state(), move |state| {
        if state.status == FutureStatus::Fulfilled && state.data.is_some_and(|run| run < RUNS) {
          restart.run(state.data.unwrap_or_default() + 1);
        }
      });
    }

    let state = action.state().get();
    let label = match (state.status, state.data) {
      (FutureStatus::Fulfilled, Some(run)) => format!("done {run}"),
      (FutureStatus::Pending, _) => "pending".to_owned(),
      _ => "idle".to_owned(),
    };
    Text::new(&label)
  }
}

#[test]
fn future_action_restarted_from_a_watch_on_its_own_state_completes() {
  let (label, runs) = within_deadline(|| {
    let probe = Probe::default();
    let mut app = App::new();
    let mut tree = Tree::new();
    tree.mount_root::<RestartingAction>(&mut app, probe.clone());

    probe.action.lock().unwrap().clone().unwrap().run(1);
    for _ in 0..RUNS * 2 {
      tree.tick_futures();
    }

    let label = tree.root().unwrap().text_content().map(str::to_owned);
    (label, probe.runs.load(Ordering::SeqCst))
  });
  assert_eq!(runs, RUNS);
  assert_eq!(label.as_deref(), Some("done 3"));
}

#[cfg(feature = "tokio")]
#[test]
fn tokio_future_action_restarted_from_a_watch_on_its_own_state_completes() {
  let (label, runs) = within_deadline(|| {
    let runtime = tokio::runtime::Builder::new_current_thread().build().unwrap();
    let probe = Probe::default();
    let mut app = App::new().with_tokio_handle(runtime.handle().clone());
    let mut tree = Tree::new();
    tree.mount_root::<RestartingAction>(&mut app, probe.clone());

    probe.action.lock().unwrap().clone().unwrap().run(1);
    for _ in 0..RUNS * 4 {
      runtime.block_on(async {
        for _ in 0..4 {
          tokio::task::yield_now().await;
        }
      });
      tree.tick_futures();
    }

    let label = tree.root().unwrap().text_content().map(str::to_owned);
    (label, probe.runs.load(Ordering::SeqCst))
  });
  assert_eq!(runs, RUNS);
  assert_eq!(label.as_deref(), Some("done 3"));
}
