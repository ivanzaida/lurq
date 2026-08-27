//! Ring buffer behind the `lurq_logs` tool, filled by a `tracing` layer the
//! app opts into:
//!
//! ```ignore
//! tracing_subscriber::registry()
//!   .with(tracing_subscriber::fmt::layer())
//!   .with(lurq::mcp::log_layer())
//!   .init();
//! ```
//!
//! The buffer is a process-wide static so the layer can be installed before
//! `enable_mcp` runs; without the layer, `lurq_logs` reports that capture is
//! not installed.

use std::{
  collections::VecDeque,
  fmt::Write as _,
  sync::{Mutex, OnceLock},
};

const LOG_CAPACITY: usize = 2000;

struct LogBuffer {
  installed: bool,
  lines: VecDeque<String>,
}

fn buffer() -> &'static Mutex<LogBuffer> {
  static BUFFER: OnceLock<Mutex<LogBuffer>> = OnceLock::new();
  BUFFER.get_or_init(|| {
    Mutex::new(LogBuffer {
      installed: false,
      lines: VecDeque::new(),
    })
  })
}

pub(crate) fn push_line(line: String) {
  let mut buffer = buffer().lock().unwrap();
  if buffer.lines.len() >= LOG_CAPACITY {
    buffer.lines.pop_front();
  }
  buffer.lines.push_back(line);
}

/// (installed, matching lines — newest last, capped at `max_lines`)
pub(crate) fn recent_lines(max_lines: usize, filter: Option<&str>) -> (bool, Vec<String>) {
  let buffer = buffer().lock().unwrap();
  let lines = buffer
    .lines
    .iter()
    .filter(|line| filter.is_none_or(|needle| line.to_lowercase().contains(&needle.to_lowercase())))
    .rev()
    .take(max_lines)
    .cloned()
    .collect::<Vec<_>>()
    .into_iter()
    .rev()
    .collect();
  (buffer.installed, lines)
}

/// A `tracing_subscriber` layer that copies events into the `lurq_logs` ring
/// buffer. Install it alongside your normal subscriber (see module docs).
pub struct McpLogLayer {
  _private: (),
}

/// Create the layer for the `lurq_logs` ring buffer.
pub fn log_layer() -> McpLogLayer {
  buffer().lock().unwrap().installed = true;
  McpLogLayer { _private: () }
}

struct MessageVisitor {
  message: String,
  fields: String,
}

impl tracing::field::Visit for MessageVisitor {
  fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
    if field.name() == "message" {
      let _ = write!(self.message, "{value:?}");
    } else {
      let _ = write!(self.fields, " {}={value:?}", field.name());
    }
  }
}

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for McpLogLayer {
  fn on_event(&self, event: &tracing::Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
    let metadata = event.metadata();
    let mut visitor = MessageVisitor {
      message: String::new(),
      fields: String::new(),
    };
    event.record(&mut visitor);
    push_line(format!(
      "{} {} {}{}",
      metadata.level(),
      metadata.target(),
      visitor.message,
      visitor.fields
    ));
  }
}
