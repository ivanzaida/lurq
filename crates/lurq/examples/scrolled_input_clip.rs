//! A multiline text input that scrolls its own text, inside a page that scrolls: the input's text must stay
//! inside the input's box after the page moves. Also a single-line input with long text and a text clipped by its
//! parent in the same page.
//!
//! cargo run -p lurq --example scrolled_input_clip --features winit,wgpu,screenshot -- --scroll -90,-33 --capture
//! out.png
//!
//! The window focuses the multiline input and types `--lines` lines into it (12 by default) key by key, so the
//! input scrolls its text to its last line. Then it scrolls the page by each `--scroll` amount, a wheel notch each
//! as the MCP server sends one, with the pointer beside the inputs; at a scale of 1.5 an odd amount moves the page
//! by half a physical pixel. It saves its frame to `--capture` and closes once the file is written; without
//! `--capture` it stays open.

use std::{
  cell::RefCell,
  sync::atomic::{AtomicU64, Ordering},
  time::{Duration, Instant},
};

use lurq::{
  app::{
    App, Tree,
    component::Component,
    ctx::{Ctx, Interval, Timeout},
    events::ScrollPhase,
    wgpu_render::WgpuRenderEngine,
    window::WindowHandle,
    winit_shell::WinitWindow,
  },
  components::{Column, Row, ScrollVertical, Text, TextInput, TextInputController},
  core::Signal,
  layout::text_style::TextStyle,
  node::{border::Border, color::Color, padding::Padding},
};

const INPUT: &str = "multiline-input";
const FOCUS_AFTER: Duration = Duration::from_millis(400);
const TYPE_FROM: Duration = Duration::from_millis(800);
const TYPE_EVERY: Duration = Duration::from_millis(15);
const SCROLL_EVERY_AFTER_TYPING: Duration = Duration::from_millis(600);
const SCROLL_EVERY: Duration = Duration::from_millis(300);
const FILE_CHECK_EVERY: Duration = Duration::from_millis(500);

thread_local! {
  static WINDOW: RefCell<Option<WindowHandle>> = const { RefCell::new(None) };
}

#[derive(Clone, Debug, PartialEq, lurq::DevtoolsInspectable)]
struct Options {
  capture: String,
}

struct Page {
  value: Signal<String>,
  line: Signal<String>,
  placed: Signal<bool>,
  controller: TextInputController,
  focused: std::sync::atomic::AtomicBool,
  _timers: Vec<Timeout>,
  _close_when_saved: Option<Interval>,
}

/// The body's type of a design: 14 with a line of 1.5, light on the dark page.
fn body() -> TextStyle {
  TextStyle {
    font_size: 14.0,
    line_height: 1.5,
    color: Color::from_hex("#e5e7eb"),
    ..TextStyle::default()
  }
}

impl Component for Page {
  type Props = Options;

  fn create(ctx: &mut Ctx) -> Self {
    let options = ctx.props::<Options>().clone();
    WINDOW.with(|window| *window.borrow_mut() = Some(ctx.window()));
    let placed = ctx.signal(false);
    let mounted = placed.clone();
    let timer = ctx.create_timeout(FOCUS_AFTER, move || mounted.set(true));
    timer.start();
    let close_when_saved = (!options.capture.is_empty()).then(|| {
      let (window, path, seen) = (ctx.window(), options.capture.clone(), AtomicU64::new(0));
      let check = ctx.create_interval(FILE_CHECK_EVERY, move || {
        let size = std::fs::metadata(&path).map_or(0, |file| file.len());
        if size > 0 && seen.swap(size, Ordering::Relaxed) == size {
          window.close();
        }
      });
      check.start();
      check
    });
    Self {
      value: ctx.signal(String::new()),
      line: ctx
        .signal("A single-line input whose text runs well past its right edge, so that it scrolls across".to_owned()),
      placed,
      controller: TextInputController::new(),
      focused: std::sync::atomic::AtomicBool::new(false),
      _timers: vec![timer],
      _close_when_saved: close_when_saved,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<lurq::node::Element> {
    if self.placed.get() && !self.focused.swap(true, Ordering::Relaxed) {
      self.controller.focus(ctx);
    }
    let filler = |label: &str| {
      Column::new()
        .height(260.0)
        .background("#1f2937")
        .child(Text::new(label))
    };
    ScrollVertical::new(
      Column::new()
        .padding(24.0)
        .spacing(16.0)
        .background("#0b0b0f")
        .foreground("#e5e7eb")
        .child(filler("above"))
        .child(
          Row::new()
            .spacing(24.0)
            .child(
              TextInput::styled(self.value.clone(), body())
                .rows(3, 8)
                .id(INPUT)
                .width(280.0)
                .padding(Padding::symmetric(12.0, 8.0))
                .border(Border::inside(1.0, "#6b7280"))
                .rounded(6.0)
                .background("#0b0b0f")
                .controller(self.controller.clone()),
            )
            .child(
              Column::new()
                .spacing(16.0)
                .child(
                  TextInput::styled(self.line.clone(), body())
                    .width(220.0)
                    .padding(Padding::symmetric(12.0, 8.0))
                    .border(Border::inside(1.0, "#6b7280"))
                    .rounded(6.0)
                    .background("#0b0b0f"),
                )
                .child(
                  Column::new()
                    .width(220.0)
                    .height(30.0)
                    .clip()
                    .background("#111827")
                    .child(Text::new(
                      "A text taller than its clipped parent\nsecond line\nthird line",
                    )),
                ),
            ),
        )
        .child(filler("below"))
        .child(filler("further below")),
    )
  }
}

fn main() {
  let args: Vec<String> = std::env::args().skip(1).collect();
  let value_of = |flag: &str| {
    args.iter().position(|arg| arg == flag).map(|at| {
      args
        .get(at + 1)
        .unwrap_or_else(|| panic!("{flag} takes a value"))
        .clone()
    })
  };
  let capture = value_of("--capture").unwrap_or_default();
  if !capture.is_empty() && std::path::Path::new(&capture).exists() {
    std::fs::remove_file(&capture).expect("the earlier capture can be replaced");
  }
  let mut scrolls: Vec<f32> = value_of("--scroll")
    .map(|list| {
      list
        .split(',')
        .map(|delta| delta.trim().parse().expect("a number"))
        .collect()
    })
    .unwrap_or_default();
  scrolls.reverse();
  let capture_path = capture.clone();
  let mut captured = capture.is_empty();

  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_render_engine_factory(|| Box::new(WgpuRenderEngine::new()));
  tree.mount_root::<Page>(&mut app, Options { capture });

  let lines: usize = value_of("--lines").map_or(12, |lines| lines.parse().expect("a number"));
  let mut keys: Vec<String> = Vec::new();
  for line in 0..lines {
    if line > 0 {
      keys.push("Enter".to_owned());
    }
    keys.extend("йцуйцу".chars().map(String::from));
  }
  keys.reverse();

  let started_at = Instant::now();
  let mut next_step = TYPE_FROM;
  WinitWindow::new(app, tree)
    .with_size(760, 560)
    .on_tick(move |tree, _| {
      let elapsed = started_at.elapsed();
      if elapsed < next_step {
        return;
      }
      if let Some(key) = keys.pop() {
        let code = if key == "Enter" {
          "Enter".to_owned()
        } else {
          "KeyQ".to_owned()
        };
        tree.key_down(key.clone(), code.clone(), false, false, false);
        tree.key_up(key, code, false, false, false);
        next_step = elapsed
          + if keys.is_empty() {
            SCROLL_EVERY_AFTER_TYPING
          } else {
            TYPE_EVERY
          };
        return;
      }
      next_step = elapsed + SCROLL_EVERY;
      let scale = tree.scale_factor();
      let Some(bounds) = tree.get_element_by_id_mut(INPUT).and_then(|element| element.bounds()) else {
        return;
      };
      if let Some(delta) = scrolls.pop() {
        // Beside the inputs, over the page.
        let (x, y) = ((bounds.x + bounds.width + 260.0) * scale, (bounds.y + 10.0) * scale);
        // A wheel notch as the MCP server sends one: the pointer moves there, then a whole gesture.
        tree.mouse_move(x, y);
        tree.scroll(x, y, 0.0, 0.0, ScrollPhase::Start);
        tree.scroll(x, y, 0.0, delta, ScrollPhase::Scroll);
        tree.scroll(x, y, 0.0, 0.0, ScrollPhase::End);
        println!("scrolled {delta}: input box at y {} (logical), scale {scale}", bounds.y);
        return;
      }
      if !captured {
        captured = true;
        println!("input box {bounds:?}, scale {scale}");
        WINDOW.with(|window| {
          if let Some(window) = window.borrow().as_ref() {
            window.screenshot(capture_path.clone());
          }
        });
      }
    })
    .with_position(120, 120)
    .with_start_without_focus(true)
    .with_title("lurq scrolled input clip")
    .run();
}
