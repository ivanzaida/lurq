//! A page of text with one turning 16 px spinner, to measure what an animation costs. With `steps` the spinner
//! turns in 12 steps a second, with `continuous` it turns smoothly. `--minimized` minimizes the window after a
//! second and restores it 2 s before the end. The example prints its pid, closes itself after the given number of
//! seconds (default 30) and prints how many frames it presented. Measure its CPU time from outside, for example with
//! PowerShell's `(Get-Process -Id <pid>).CPU`.
//!
//! cargo run -p lurq --release --example spinner_pacing --features winit,wgpu -- steps 30

use std::{
  f32::consts::TAU,
  sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
  },
  time::{Duration, Instant},
};

use lurq::{
  animation::{AnimatableProperty, Animation, Easing, Keyframes, KeyframesId, StepPosition},
  app::{
    App, Tree,
    component::Component,
    ctx::{Ctx, Timeout},
    wgpu_render::WgpuRenderEngine,
    winit_shell::WinitWindow,
  },
  components::{Column, Rect, Row, Text},
  node::{Element, transform::Decomposed},
};

const TURN: KeyframesId = KeyframesId::new(1);
const MINIMIZE_AFTER: Duration = Duration::from_secs(1);
const RESTORE_BEFORE_END: Duration = Duration::from_secs(2);

#[derive(Clone, Copy, Debug, PartialEq, lurq::DevtoolsInspectable)]
struct Options {
  /// 12 steps a turn, or a smooth turn.
  stepped: bool,
  seconds: u64,
  minimize: bool,
}

struct SpinnerPage {
  _close: Timeout,
  _minimize: Option<(Timeout, Timeout)>,
}

impl Component for SpinnerPage {
  type Props = Options;

  fn create(ctx: &mut Ctx) -> Self {
    let options = *ctx.props::<Options>();
    let window = ctx.window();
    let close = ctx.create_timeout(Duration::from_secs(options.seconds), move || window.close());
    close.start();
    let minimize = options.minimize.then(|| {
      let window = ctx.window();
      let minimize = ctx.create_timeout(MINIMIZE_AFTER, move || window.set_minimized(true));
      minimize.start();
      let window = ctx.window();
      let restore_at = Duration::from_secs(options.seconds) - RESTORE_BEFORE_END;
      let restore = ctx.create_timeout(restore_at, move || window.set_minimized(false));
      restore.start();
      (minimize, restore)
    });
    Self {
      _close: close,
      _minimize: minimize,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let easing = if ctx.props::<Options>().stepped {
      Easing::Steps {
        count: 12,
        position: StepPosition::JumpEnd,
      }
    } else {
      Easing::Linear
    };
    let spinner = Rect::new(16.0, 16.0)
      .background("#2563eb")
      .rounded(3.0)
      .animation(Animation::new(TURN).duration_ms(1000).easing(easing).infinite());
    let mut page = Column::new()
      .spacing(4.0)
      .child(Row::new().spacing(8.0).child(spinner).child(Text::new("Working")));
    for row in 0..30 {
      page = page.child(Text::new(&format!(
        "Row {row}: a line of text that stays still while the spinner turns"
      )));
    }
    page
  }
}

fn main() {
  let args: Vec<String> = std::env::args().skip(1).collect();
  let stepped = match args.first().map(String::as_str) {
    Some("steps") | None => true,
    Some("continuous") => false,
    Some(other) => panic!("unknown mode {other}: use steps or continuous"),
  };
  let seconds = args
    .get(1)
    .map(|value| value.parse().expect("the run time in whole seconds"))
    .unwrap_or(30);
  let minimize = args.iter().any(|arg| arg == "--minimized");

  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_render_engine_factory(|| Box::new(WgpuRenderEngine::new()));
  tree.register_keyframes(
    Keyframes::new(TURN)
      .frame(0.0, |f| {
        f.set(AnimatableProperty::Transform, Decomposed::IDENTITY.with_rotate(0.0));
      })
      .frame(1.0, |f| {
        f.set(AnimatableProperty::Transform, Decomposed::IDENTITY.with_rotate(TAU));
      }),
  );
  tree.mount_root::<SpinnerPage>(
    &mut app,
    Options {
      stepped,
      seconds,
      minimize,
    },
  );
  println!("pid {}", std::process::id());

  let started_at = Instant::now();
  let restore_at = Duration::from_secs(seconds) - RESTORE_BEFORE_END;
  let frames = Arc::new(AtomicU64::new(0));
  let frames_minimized = Arc::new(AtomicU64::new(0));
  let frames_restored = Arc::new(AtomicU64::new(0));
  let first_restored_at = Arc::new(AtomicU64::new(0));
  let painted = (
    frames.clone(),
    frames_minimized.clone(),
    frames_restored.clone(),
    first_restored_at.clone(),
  );
  WinitWindow::new(app, tree)
    .with_size(640, 760)
    .with_position(-12000, -12000)
    .with_start_without_focus(true)
    .with_title("lurq spinner pacing")
    .on_paint(move |_, _, _| {
      painted.0.fetch_add(1, Ordering::Relaxed);
      let at = started_at.elapsed();
      if !minimize || at < MINIMIZE_AFTER + Duration::from_millis(100) {
        return;
      }
      if at < restore_at {
        painted.1.fetch_add(1, Ordering::Relaxed);
      } else if painted.2.fetch_add(1, Ordering::Relaxed) == 0 {
        let micros = u64::try_from(at.as_micros()).expect("the example runs for less than 584,000 years");
        painted.3.store(micros, Ordering::Relaxed);
      }
    })
    .run();

  let elapsed = started_at.elapsed().as_secs_f64();
  let frames = frames.load(Ordering::Relaxed);
  println!("frames {frames} in {elapsed:.1} s: {:.1} fps", frames as f64 / elapsed);
  if minimize {
    let minimized = frames_minimized.load(Ordering::Relaxed);
    let restored = frames_restored.load(Ordering::Relaxed);
    let first = Duration::from_micros(first_restored_at.load(Ordering::Relaxed)).saturating_sub(restore_at);
    println!(
      "frames while minimized: {minimized}; after the restore: {restored} in {} s, the first {:.1} ms after it",
      RESTORE_BEFORE_END.as_secs(),
      first.as_secs_f64() * 1000.0
    );
  }
}
