//! An idle window with the work a real app keeps around: a clock interval of a minute, a 2 s interval, and a
//! stream fed by a thread every 3 s on a Tokio runtime. With nothing to draw, the event loop should wait for the
//! earliest timer or a wake from the stream and use no CPU in between. The example prints when each timer fires
//! and how long a stream item took to reach a render, and closes itself after the given number of seconds
//! (default 30). Measure its CPU time from outside, for example with PowerShell's `(Get-Process -Id <pid>).CPU`.
//!
//! cargo run -p lurq --example idle_wait --features winit,wgpu,tokio -- 30

use std::{
  sync::{Arc, Mutex},
  time::{Duration, Instant},
};

use lurq::{
  app::{
    App, Tree,
    component::Component,
    ctx::{Ctx, Interval, StreamEmitter, Timeout},
    wgpu_render::WgpuRenderEngine,
    winit_shell::WinitWindow,
  },
  components::{Column, Text},
  core::Signal,
  node::Element,
};

const POLL_EVERY: Duration = Duration::from_secs(2);
const STREAM_EVERY: Duration = Duration::from_secs(3);

struct IdleWait {
  ticks: Signal<u32>,
  _clock: Interval,
  _poll: Interval,
  _close: Timeout,
  /// Stream items carry the microseconds since this instant at which they were sent: a plain number, which any
  /// feature set (`devtools` included) can keep in a stream's state.
  epoch: Instant,
  last_item: Mutex<Option<u64>>,
}

impl Component for IdleWait {
  type Props = u64;

  fn create(ctx: &mut Ctx) -> Self {
    let seconds = *ctx.props::<u64>();
    let ticks = ctx.signal(0_u32);
    let clock = ctx.create_interval(Duration::from_secs(60), || println!("clock: minute"));
    clock.start();
    let poll = ctx.create_interval(POLL_EVERY, {
      let ticks = ticks.clone();
      let started = Arc::new(Mutex::new(Instant::now()));
      move || {
        let mut last = started.lock().expect("the poll interval runs on one thread");
        let late = last.elapsed().saturating_sub(POLL_EVERY);
        *last = Instant::now();
        println!("poll: fired {:.1} ms late", late.as_secs_f64() * 1000.0);
        ticks.update(|ticks| *ticks += 1);
      }
    });
    poll.start();
    let window = ctx.window();
    let close = ctx.create_timeout(Duration::from_secs(seconds), move || window.close());
    close.start();
    Self {
      ticks,
      _clock: clock,
      _poll: poll,
      _close: close,
      epoch: Instant::now(),
      last_item: Mutex::new(None),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let epoch = self.epoch;
    let item = ctx
      .stream((), move |_, emitter: StreamEmitter<u64, ()>| async move {
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
        std::thread::spawn(move || {
          loop {
            std::thread::sleep(STREAM_EVERY);
            if sender.send(micros_since(epoch)).is_err() {
              return;
            }
          }
        });
        while let Some(sent_at) = receiver.recv().await {
          if !emitter.emit(sent_at) {
            return;
          }
        }
      })
      .state()
      .get();
    if let Some(sent_at) = item.data {
      let mut last = self.last_item.lock().expect("renders run on one thread");
      if *last != Some(sent_at) {
        *last = Some(sent_at);
        println!(
          "stream: item rendered {:.1} ms after it was sent",
          micros_since(epoch).saturating_sub(sent_at) as f64 / 1000.0
        );
      }
    }
    Column::new()
      .child(Text::new("Idle: the event loop waits for the next timer or wake"))
      .child(Text::new(&format!("polls: {}", self.ticks.get())))
  }
}

fn micros_since(epoch: Instant) -> u64 {
  u64::try_from(epoch.elapsed().as_micros()).expect("the example runs for less than 584,000 years")
}

fn main() {
  let seconds = std::env::args()
    .nth(1)
    .map(|value| value.parse().expect("the run time in whole seconds"))
    .unwrap_or(30);
  let runtime = tokio::runtime::Builder::new_multi_thread()
    .worker_threads(1)
    .build()
    .expect("a Tokio runtime");
  let mut app = App::new().with_tokio_handle(runtime.handle().clone());
  let mut tree = Tree::new();
  tree.set_render_engine_factory(|| Box::new(WgpuRenderEngine::new()));
  tree.mount_root::<IdleWait>(&mut app, seconds);
  println!("pid {}", std::process::id());
  WinitWindow::new(app, tree)
    .with_size(420, 120)
    .with_position(-2500, 40)
    .with_start_without_focus(true)
    .with_title("lurq idle wait")
    .run();
}
