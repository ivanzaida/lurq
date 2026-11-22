//! What a glass costs a frame, measured in the app itself, on a desktop or a phone: the Glass screen's page
//! scrolls under the same pieces drawn with nothing behind them, with a backdrop blur of the glass's radius, and
//! as glass. Each case runs for a few seconds inside a profiling recording, and a line of its medians is
//! written to stderr. Built with the `probe` feature and started by `LURQ_GLASS_PROBE=1` (or `--glass-probe`):
//!
//! ```text
//! LURQ_GLASS_PROBE=1 cargo run -p mobile_demo --release --features probe
//! ```
//!
//! `LURQ_GLASS_PROBE_SECONDS` is how long a case is recorded (3 by default).
//!
//! Two changes, one before every frame:
//! - `scroll`: the page moves under the pieces, so every backdrop is new in every frame;
//! - `still`: a dot on the piece painted last changes colour, and nothing behind any piece does.
//!
//! `cpu` is the pass without its wait for a frame and its present; `gpu` is the time of the frame's commands
//! on the GPU (wgpu's timestamp queries), where the adapter has them.

use std::{
  sync::OnceLock,
  time::{Duration, Instant},
};

use lurq::{
  app::{
    App, Tree,
    component::Component,
    ctx::Ctx,
    profiler::{ProfilingHandle, RecordingOptions, SampleData, SessionId},
    winit_shell::WinitWindow,
  },
  components::{Column, Rect, Row, ScrollVertical, Stack},
  core::Signal,
  layout::{
    Alignment,
    layout_kind::{Justify, ScrollState},
  },
  node::{Element, Glass, HitTestBehavior, color::Color},
};

use crate::{
  glass::{band, board},
  ui::FILL,
};

/// Frames before a case is recorded, and after it for its last GPU times to arrive.
const SETTLE: Duration = Duration::from_millis(500);
const DRAIN: Duration = Duration::from_millis(200);
const LAUNCH: Duration = Duration::from_secs(2);
/// How fast the page moves, in points a second.
const SPEED: f64 = 700.0;
/// How far the page goes before it turns back.
const TRAVEL: f64 = 1400.0;
const DOT: [&str; 2] = ["#22d3ee", "#f97316"];

#[derive(Clone, Copy, PartialEq)]
enum Over {
  Nothing,
  Blur,
  Glass,
  Clear,
}

#[derive(Clone, Copy, PartialEq)]
enum Pieces {
  /// A floating bar of tabs.
  Bar,
  /// What a screen of the system has: two round buttons at the top, the bar of tabs and a round button by it.
  Bars,
  /// The same, the bar and its button one glass.
  Merged,
  /// A sheet over the lower half.
  Sheet,
  /// Eight pills.
  Eight,
  /// Twelve bars, one under another: most of the screen. More than an app shows, to find where frames are lost.
  Twelve,
  /// Four pieces as large as the screen, one on another: each has the one under it behind it.
  Layers,
}

#[derive(Clone, Copy, PartialEq)]
enum Change {
  Scroll,
  Still,
  /// Nothing changes: a glass must ask for no frame by itself.
  Idle,
}

#[derive(Clone, Copy)]
struct Case {
  name: &'static str,
  pieces: Pieces,
  over: Over,
}

const CASES: [Case; 16] = [
  Case {
    name: "bar-nothing",
    pieces: Pieces::Bar,
    over: Over::Nothing,
  },
  Case {
    name: "bar-blur",
    pieces: Pieces::Bar,
    over: Over::Blur,
  },
  Case {
    name: "bar-glass",
    pieces: Pieces::Bar,
    over: Over::Glass,
  },
  Case {
    name: "bar-clear",
    pieces: Pieces::Bar,
    over: Over::Clear,
  },
  Case {
    name: "bars-nothing",
    pieces: Pieces::Bars,
    over: Over::Nothing,
  },
  Case {
    name: "bars-blur",
    pieces: Pieces::Bars,
    over: Over::Blur,
  },
  Case {
    name: "bars-glass",
    pieces: Pieces::Bars,
    over: Over::Glass,
  },
  Case {
    name: "bars-merged",
    pieces: Pieces::Merged,
    over: Over::Glass,
  },
  Case {
    name: "sheet-blur",
    pieces: Pieces::Sheet,
    over: Over::Blur,
  },
  Case {
    name: "sheet-glass",
    pieces: Pieces::Sheet,
    over: Over::Glass,
  },
  Case {
    name: "eight-blur",
    pieces: Pieces::Eight,
    over: Over::Blur,
  },
  Case {
    name: "eight-glass",
    pieces: Pieces::Eight,
    over: Over::Glass,
  },
  Case {
    name: "twelve-blur",
    pieces: Pieces::Twelve,
    over: Over::Blur,
  },
  Case {
    name: "twelve-glass",
    pieces: Pieces::Twelve,
    over: Over::Glass,
  },
  Case {
    name: "layers-blur",
    pieces: Pieces::Layers,
    over: Over::Blur,
  },
  Case {
    name: "layers-glass",
    pieces: Pieces::Layers,
    over: Over::Glass,
  },
];

fn case_shown() -> Signal<usize> {
  static SHOWN: OnceLock<Signal<usize>> = OnceLock::new();
  SHOWN.get_or_init(|| Signal::new(0)).clone()
}

fn scroll() -> ScrollState {
  static SCROLL: OnceLock<ScrollState> = OnceLock::new();
  SCROLL.get_or_init(ScrollState::new).clone()
}

pub(crate) fn wanted() -> bool {
  std::env::var_os("LURQ_GLASS_PROBE").is_some() || std::env::args().any(|argument| argument == "--glass-probe")
}

/// A piece of the case's material.
fn piece(row: Row, over: Over, panel: bool) -> Row {
  let glass = if panel { Glass::panel() } else { Glass::regular() };
  match over {
    Over::Nothing => row.background(Color::new(30, 41, 59, 190)),
    Over::Blur => row.background(Color::new(30, 41, 59, 120)).backdrop_blur(glass.blur),
    Over::Glass => row.glass(glass),
    Over::Clear => row.glass(Glass::clear()),
  }
}

fn dot() -> Rect {
  Rect::new(10.0, 10.0).background(DOT[0]).rounded(5.0).id("probe-dot")
}

fn round(size: f32, over: Over) -> Row {
  piece(Row::new().size(size, size).rounded(size / 2.0), over, false)
}

fn bar(over: Over) -> Row {
  piece(
    Row::new()
      .flex(1.0)
      .height(62.0)
      .rounded(31.0)
      .align_items(Alignment::Center)
      .justify(Justify::Center),
    over,
    false,
  )
  .child(dot())
}

struct ProbePage {
  /// Keeps the loop turning while nothing is drawn, so the case of no change ends.
  _awake: lurq::app::ctx::Interval,
}

impl Component for ProbePage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let awake = ctx.create_interval(Duration::from_millis(50), || {});
    awake.start();
    Self { _awake: awake }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let case = CASES[case_shown().get().min(CASES.len() - 1)];
    let safe_area = ctx.safe_area();
    let mut page = Column::new().width(FILL);
    for index in 0..14 {
      page = page.child(band(index, "Scroll me under the glass"));
      if index % 2 == 0 {
        page = page.child(board());
      }
    }

    let over = case.over;
    let top = Row::new().width(FILL).justify(Justify::SpaceBetween).overflow_visible();
    let top = if matches!(case.pieces, Pieces::Bars | Pieces::Merged) {
      top.child(round(44.0, over)).child(round(44.0, over))
    } else {
      top
    };
    let bottom: Element = match case.pieces {
      Pieces::Bar => Row::new().width(FILL).overflow_visible().child(bar(over)).into(),
      Pieces::Bars | Pieces::Merged => {
        let row = Row::new()
          .width(FILL)
          .spacing(10.0)
          .overflow_visible()
          .child(bar(over))
          .child(round(62.0, over));
        if case.pieces == Pieces::Merged {
          row.glass_container(14.0).into()
        } else {
          row.into()
        }
      }
      Pieces::Sheet => piece(
        Row::new()
          .width(FILL)
          .height(420.0)
          .rounded(38.0)
          .align_items(Alignment::Center)
          .justify(Justify::Center),
        over,
        true,
      )
      .child(dot())
      .into(),
      Pieces::Twelve => {
        let mut bars = Column::new().width(FILL).spacing(6.0).overflow_visible();
        for row in 0..12 {
          let line = piece(
            Row::new()
              .width(FILL)
              .height(54.0)
              .rounded(27.0)
              .align_items(Alignment::Center)
              .justify(Justify::Center),
            over,
            false,
          );
          bars = bars.child(if row == 11 { line.child(dot()) } else { line });
        }
        bars.into()
      }
      Pieces::Layers => {
        let mut layers = Stack::new().width(FILL).height(700.0).overflow_visible();
        for layer in 0..4 {
          let sheet = piece(
            Row::new()
              .size(FILL, FILL)
              .rounded(38.0)
              .align_items(Alignment::Center)
              .justify(Justify::Center),
            over,
            true,
          );
          layers = layers.child(if layer == 3 { sheet.child(dot()) } else { sheet });
        }
        layers.into()
      }
      Pieces::Eight => {
        let mut pills = Column::new().width(FILL).spacing(14.0).overflow_visible();
        for row in 0..4 {
          let mut line = Row::new().width(FILL).spacing(14.0).overflow_visible();
          for column in 0..2 {
            let pill = piece(
              Row::new()
                .flex(1.0)
                .height(52.0)
                .rounded(26.0)
                .align_items(Alignment::Center)
                .justify(Justify::Center),
              over,
              false,
            );
            line = line.child(if row == 3 && column == 1 {
              pill.child(dot())
            } else {
              pill
            });
          }
          pills = pills.child(line);
        }
        pills.into()
      }
    };
    let pieces = Column::new()
      .size(FILL, FILL)
      .hit_test(HitTestBehavior::ContentOnly)
      .justify(Justify::SpaceBetween)
      .padding_top(safe_area.top + 8.0)
      .padding_bottom(safe_area.bottom + 8.0)
      .padding_horizontal(16.0)
      .overflow_visible()
      .child(top)
      .child(bottom);

    Stack::new()
      .size(FILL, FILL)
      .background("#0f172a")
      .child(
        ScrollVertical::new(page)
          .key("probe-page")
          .with_scroll_state(scroll())
          .size(FILL, FILL),
      )
      .child(pieces)
  }
}

enum Stage {
  Settling { since: Instant },
  Recording { session: SessionId, since: Instant },
  Draining { session: SessionId, since: Instant },
}

struct Driver {
  profiling: ProfilingHandle,
  seconds: Duration,
  started: Instant,
  /// The case and its change.
  at: (usize, Change),
  stage: Stage,
  steps: usize,
}

impl Driver {
  fn changes(case: usize) -> &'static [Change] {
    // A glass that asks for frames by itself would show in the bar of glass.
    if CASES[case].name == "bar-glass" {
      &[Change::Scroll, Change::Still, Change::Idle]
    } else {
      &[Change::Scroll, Change::Still]
    }
  }

  /// The change of the frame to come.
  fn change(&mut self, tree: &mut Tree) {
    self.steps += 1;
    match self.at.1 {
      Change::Scroll => {
        let along = self.started.elapsed().as_secs_f64() * SPEED % (TRAVEL * 2.0);
        let y = if along > TRAVEL { TRAVEL * 2.0 - along } else { along };
        scroll().set_scroll(0.0, y as f32);
      }
      Change::Still => {
        if let Some(mut dot) = tree.get_element_by_id_mut("probe-dot") {
          dot.set_background(DOT[self.steps % 2]);
        }
      }
      Change::Idle => {}
    }
  }

  fn tick(&mut self, tree: &mut Tree) {
    if self.steps == 0 {
      keep_the_screen_on();
    }
    self.change(tree);
    match self.stage {
      Stage::Settling { since } => {
        // The app's first seconds are its launch, not the first case.
        if since.elapsed() < SETTLE || self.started.elapsed() < LAUNCH {
          return;
        }
        let session = self
          .profiling
          .start_recording(RecordingOptions::default())
          .expect("a profiling session")
          .id;
        self.stage = Stage::Recording {
          session,
          since: Instant::now(),
        };
      }
      Stage::Recording { session, since } => {
        if since.elapsed() >= self.seconds {
          self.stage = Stage::Draining {
            session,
            since: Instant::now(),
          };
        }
      }
      Stage::Draining { session, since } => {
        if since.elapsed() < DRAIN {
          return;
        }
        self.report(session);
        self.next();
      }
    }
  }

  fn next(&mut self) {
    let (case, change) = self.at;
    let changes = Self::changes(case);
    let position = changes.iter().position(|other| *other == change).unwrap_or(0);
    self.at = if position + 1 < changes.len() {
      (case, changes[position + 1])
    } else if case + 1 < CASES.len() {
      case_shown().set(case + 1);
      (case + 1, Change::Scroll)
    } else {
      eprintln!("GLASSPROBE done");
      std::process::exit(0);
    };
    self.stage = Stage::Settling { since: Instant::now() };
  }

  fn report(&self, session: SessionId) {
    let report = self.profiling.end(session).expect("the recording to end");
    let seconds = self.seconds.as_secs_f64();
    let (mut cpu, mut layout, mut quads, mut encode, mut submit, mut gpu, mut intervals) = (
      Vec::new(),
      Vec::new(),
      Vec::new(),
      Vec::new(),
      Vec::new(),
      Vec::new(),
      Vec::new(),
    );
    let (mut frames, mut missed) = (0_u32, 0_u32);
    let mut refresh = None;
    for sample in &report.samples {
      let SampleData::Pass(pass) = &sample.data else {
        continue;
      };
      // The frames of the recorded seconds; the ones after them only brought the GPU times of the last.
      if !pass.rendered || sample.completed_ms - report.started_ms > seconds * 1000.0 {
        continue;
      }
      frames += 1;
      let frame = pass.frame.clone().unwrap_or_default();
      cpu.push(ms(
        pass.total.saturating_sub(frame.render.acquire + frame.render.present),
      ));
      layout.push(ms(pass.layout_update));
      quads.push(ms(frame.quad_resolve));
      encode.push(ms(frame.render.encode));
      submit.push(ms(frame.render.submit));
      if let Some(time) = pass.gpu {
        gpu.push(ms(time.ui));
      }
      if let Some(present) = pass.present {
        intervals.extend(present.interval.map(ms));
        missed += present.missed_vsyncs;
        refresh = present.refresh_interval.or(refresh);
      }
    }
    let change = match self.at.1 {
      Change::Scroll => "scroll",
      Change::Still => "still",
      Change::Idle => "idle",
    };
    eprintln!(
      "GLASSPROBE,{},{change},frames={frames},fps={:.1},interval={:.2}/{:.2},missed={missed},refresh={:.2},cpu={:.3}/{:.3},\
       layout={:.3},quads={:.3},encode={:.3},submit={:.3},gpu={:.3}/{:.3},gpu_frames={}",
      CASES[self.at.0].name,
      f64::from(frames) / seconds,
      at(&mut intervals, 0.5),
      at(&mut intervals, 0.99),
      refresh.map_or(f64::NAN, ms),
      at(&mut cpu, 0.5),
      at(&mut cpu, 0.95),
      at(&mut layout, 0.5),
      at(&mut quads, 0.5),
      at(&mut encode, 0.5),
      at(&mut submit, 0.5),
      at(&mut gpu, 0.5),
      at(&mut gpu, 0.95),
      gpu.len(),
    );
  }
}

/// A phone nobody touches dims its screen and locks in the middle of the cases: the system's idle timer is
/// off while the probe runs.
#[cfg(target_os = "ios")]
fn keep_the_screen_on() {
  use std::ffi::{c_char, c_void};
  unsafe extern "C" {
    fn objc_getClass(name: *const c_char) -> *mut c_void;
    fn sel_registerName(name: *const c_char) -> *mut c_void;
    fn objc_msgSend();
  }
  // SAFETY: `+[UIApplication sharedApplication]` and `-setIdleTimerDisabled:` are called with their own
  // signatures, on the main thread, after the application started.
  unsafe {
    let shared: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
      std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
    let set: unsafe extern "C" fn(*mut c_void, *mut c_void, bool) =
      std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
    let application = shared(
      objc_getClass(c"UIApplication".as_ptr()),
      sel_registerName(c"sharedApplication".as_ptr()),
    );
    if !application.is_null() {
      set(application, sel_registerName(c"setIdleTimerDisabled:".as_ptr()), true);
    }
  }
}

#[cfg(not(target_os = "ios"))]
fn keep_the_screen_on() {}

fn ms(duration: Duration) -> f64 {
  duration.as_secs_f64() * 1000.0
}

/// The value `part` of the way through the sorted values.
fn at(values: &mut [f64], part: f64) -> f64 {
  if values.is_empty() {
    return f64::NAN;
  }
  values.sort_by(f64::total_cmp);
  values[((values.len() - 1) as f64 * part).round() as usize]
}

pub(crate) fn run(mut app: App, mut tree: Tree) {
  let seconds = std::env::var("LURQ_GLASS_PROBE_SECONDS")
    .ok()
    .and_then(|value| value.parse().ok())
    .unwrap_or(3.0);
  tree.mount_root::<ProbePage>(&mut app, ());
  let gpu = tree.profiling_handle();
  let mut driver = Driver {
    profiling: gpu,
    seconds: Duration::from_secs_f64(seconds),
    started: Instant::now(),
    at: (0, Change::Scroll),
    stage: Stage::Settling { since: Instant::now() },
    steps: 0,
  };
  eprintln!("GLASSPROBE start, {seconds} s a case");
  let window = WinitWindow::new(app, tree)
    .with_title("lurq glass probe")
    .on_tick(move |tree, _| driver.tick(tree));
  #[cfg(not(target_os = "ios"))]
  let window = window.with_size(393, 852);
  window.run();
}
