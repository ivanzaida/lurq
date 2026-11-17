//! `lurq_touch`: gestures of fingers, played through the tree's model of a finger ([`Tree::finger`]) as a touch
//! screen's would be: every finger's own events, the pointer the first one stands for, and what the engine makes of
//! them (a tap, a long press, a pan, a swipe, the edge pan, a pinch, a rotation, three fingers' editing commands).
//!
//! A gesture is played at once, on a clock of its own that ends now: its fingers move at the speed it says, a long
//! press rests for its time, and what it leaves moving (momentum, a list springing back) goes on in real time.

use std::f32::consts::{PI, TAU};

use super::{
  interact::{arg_f32, resolve_point},
  windows::window_tree_mut,
};
use crate::{
  app::{Tree, events::TouchPhase},
  mcp::{
    McpState,
    shared::{McpToolOutput, McpToolResult},
  },
  time::{Duration, Instant},
};

/// The ids of the fingers a gesture plays, far from any a screen gives.
const FIRST_FINGER: u64 = 0x7000_0000;
/// How often a moving gesture reports its fingers, in milliseconds: a display's frame.
const STEP_MS: f32 = 16.0;
/// How far apart fingers that land together are, in logical pixels.
const FINGER_GAP: f32 = 40.0;

/// One moment of a gesture: where each of its fingers is, at a time from its start, in logical pixels relative to
/// the gesture's point. A finger absent from a moment is up.
struct Moment {
  at: f32,
  fingers: Vec<(f32, f32)>,
}

fn moment(at: f32, fingers: Vec<(f32, f32)>) -> Moment {
  Moment { at, fingers }
}

/// Fingers side by side about the point.
fn side_by_side(count: usize) -> Vec<(f32, f32)> {
  (0..count)
    .map(|index| ((index as f32 - (count as f32 - 1.0) / 2.0) * FINGER_GAP, 0.0))
    .collect()
}

/// Fingers on a circle of `radius` about the point, turned by `angle`.
fn around(count: usize, radius: f32, angle: f32) -> Vec<(f32, f32)> {
  (0..count)
    .map(|index| {
      let turn = angle + TAU * index as f32 / count as f32 + if count == 2 { 0.0 } else { -PI / 2.0 };
      (radius * turn.cos(), radius * turn.sin())
    })
    .collect()
}

/// Moments every [`STEP_MS`] from `from` for `duration` milliseconds, the fingers where `at` puts them at each share
/// of the way, 0 to 1.
fn moving(from: f32, duration: f32, at: impl Fn(f32) -> Vec<(f32, f32)>) -> Vec<Moment> {
  let steps = (duration / STEP_MS).ceil().max(1.0) as usize;
  (0..=steps)
    .map(|step| {
      let share = step as f32 / steps as f32;
      moment(from + duration * share, at(share))
    })
    .collect()
}

fn arg_usize(args: &serde_json::Value, name: &str) -> Option<usize> {
  args
    .get(name)
    .and_then(serde_json::Value::as_u64)
    .map(|value| value as usize)
}

fn arg_str<'a>(args: &'a serde_json::Value, name: &str) -> Option<&'a str> {
  args.get(name).and_then(serde_json::Value::as_str)
}

/// The way a gesture goes, as `direction` says: a unit step in logical pixels.
fn direction(args: &serde_json::Value) -> Result<(f32, f32), String> {
  match arg_str(args, "direction").unwrap_or("left") {
    "left" => Ok((-1.0, 0.0)),
    "right" => Ok((1.0, 0.0)),
    "up" => Ok((0.0, -1.0)),
    "down" => Ok((0.0, 1.0)),
    other => Err(format!("unknown direction {other:?}: left, right, up or down")),
  }
}

/// The moments of a gesture, and the point it is played at if the gesture puts it itself (an edge pan's edge).
fn timeline(
  gesture: &str,
  args: &serde_json::Value,
  window: (f32, f32),
  point: Option<(f32, f32)>,
) -> Result<(Vec<Moment>, Option<(f32, f32)>), String> {
  let fingers = arg_usize(args, "fingers").unwrap_or(1).clamp(1, 5);
  let still = |count: usize| side_by_side(count);
  Ok(match gesture {
    "tap" | "double_tap" => {
      let count = if gesture == "double_tap" {
        2
      } else {
        arg_usize(args, "count").unwrap_or(1).clamp(1, 3)
      };
      let mut moments = Vec::new();
      for tap in 0..count {
        let at = tap as f32 * 150.0;
        moments.push(moment(at, still(fingers)));
        moments.push(moment(at + 60.0, still(fingers)));
        moments.push(moment(at + 61.0, Vec::new()));
      }
      (moments, None)
    }
    "long_press" => {
      let held = arg_f32(args, "duration_ms").unwrap_or(700.0).max(0.0);
      let mut moments = moving(0.0, held, |_| still(fingers));
      moments.push(moment(held + 1.0, Vec::new()));
      (moments, None)
    }
    "pan" | "swipe" | "three_finger_swipe" => {
      let fingers = if gesture == "three_finger_swipe" { 3 } else { fingers };
      let (dx, dy) = match (arg_f32(args, "dx"), arg_f32(args, "dy")) {
        (Some(dx), Some(dy)) if gesture == "pan" => (dx, dy),
        _ => {
          let way = direction(args)?;
          let distance = arg_f32(args, "distance").unwrap_or(200.0);
          (way.0 * distance, way.1 * distance)
        }
      };
      // A pan goes at an even pace and rests before it lifts, leaving no momentum; a swipe lifts at its speed.
      let speed = arg_f32(args, "speed").unwrap_or(if gesture == "pan" { 400.0 } else { 1200.0 });
      let duration = dx.hypot(dy) / speed.max(1.0) * 1000.0;
      let fingers_at = move |share: f32| {
        still(fingers)
          .into_iter()
          .map(|(x, y)| (x + dx * share, y + dy * share))
          .collect::<Vec<_>>()
      };
      let mut moments = moving(0.0, duration, fingers_at);
      let mut end = duration;
      if gesture == "pan" {
        end += 150.0;
        moments.push(moment(end, fingers_at(1.0)));
      }
      moments.push(moment(end + 1.0, Vec::new()));
      (moments, None)
    }
    "edge_pan" => {
      let edge = arg_str(args, "edge").unwrap_or("left");
      let distance = arg_f32(args, "distance").unwrap_or(window.0 / 2.0);
      let (start, way) = match (edge, point) {
        ("left", point) => ((2.0, point.map_or(window.1 / 2.0, |p| p.1)), (1.0, 0.0)),
        ("right", point) => ((window.0 - 2.0, point.map_or(window.1 / 2.0, |p| p.1)), (-1.0, 0.0)),
        ("top", point) => ((point.map_or(window.0 / 2.0, |p| p.0), 2.0), (0.0, 1.0)),
        ("bottom", point) => ((point.map_or(window.0 / 2.0, |p| p.0), window.1 - 2.0), (0.0, -1.0)),
        (other, _) => return Err(format!("unknown edge {other:?}: left, right, top or bottom")),
      };
      let speed = arg_f32(args, "speed").unwrap_or(600.0);
      let duration = distance / speed.max(1.0) * 1000.0;
      let mut moments = moving(0.0, duration, |share| {
        vec![(way.0 * distance * share, way.1 * distance * share)]
      });
      moments.push(moment(duration + 1.0, Vec::new()));
      (moments, Some(start))
    }
    "pinch" | "rotate" | "three_finger_pinch" => {
      let count = if gesture == "three_finger_pinch" { 3 } else { 2 };
      let radius = arg_f32(args, "distance").unwrap_or(if count == 3 { 80.0 } else { 60.0 });
      let scale = arg_f32(args, "scale").unwrap_or(if gesture == "rotate" { 1.0 } else { 2.0 });
      let turn = arg_f32(args, "angle")
        .unwrap_or(if gesture == "rotate" { 45.0 } else { 0.0 })
        .to_radians();
      let duration = arg_f32(args, "duration_ms").unwrap_or(300.0);
      let mut moments = moving(0.0, duration, |share| {
        around(count, radius * (1.0 + (scale - 1.0) * share), turn * share)
      });
      moments.push(moment(duration + 1.0, Vec::new()));
      (moments, None)
    }
    "zoom" => {
      // One finger: a tap, then a second landing held and dragged, down to zoom in.
      let scale = arg_f32(args, "scale").unwrap_or(2.0).max(0.05);
      let down = 150.0 * scale.log2();
      let duration = arg_f32(args, "duration_ms").unwrap_or(400.0);
      let mut moments = vec![
        moment(0.0, vec![(0.0, 0.0)]),
        moment(60.0, vec![(0.0, 0.0)]),
        moment(61.0, Vec::new()),
        moment(150.0, vec![(0.0, 0.0)]),
      ];
      // The finger travels past the slop before the zoom starts; the scale counts from there.
      let slop = 12.0 * if down < 0.0 { -1.0 } else { 1.0 };
      moments.extend(moving(170.0, duration, |share| vec![(0.0, slop + down * share)]));
      moments.push(moment(170.0 + duration + 1.0, Vec::new()));
      (moments, None)
    }
    other => {
      return Err(format!(
        "unknown gesture {other:?}: tap, double_tap, long_press, pan, swipe, edge_pan, pinch, rotate, zoom, \
         three_finger_swipe, three_finger_pinch"
      ));
    }
  })
}

/// Plays the moments on `tree`, at `point` in physical pixels, on a clock that ends now.
fn play(tree: &mut Tree, moments: &[Moment], point: (f32, f32)) {
  let scale = tree.scale_factor();
  let total = moments.last().map_or(0.0, |moment| moment.at);
  let now = Instant::now();
  let start = now.checked_sub(Duration::from_secs_f32(total / 1000.0)).unwrap_or(now);
  let saved_clock = tree.press_clock();
  let mut down: Vec<(f32, f32)> = Vec::new();
  for moment in moments {
    let at = start + Duration::from_secs_f32(moment.at.max(0.0) / 1000.0);
    tree.set_press_clock(Some(at));
    // What time brought since the last moment: a press after resting, a long press.
    tree.finger_tick(at);
    let place = |(x, y): (f32, f32)| (point.0 + x * scale, point.1 + y * scale);
    for (index, finger) in moment.fingers.iter().enumerate() {
      let (x, y) = place(*finger);
      let phase = if index < down.len() {
        TouchPhase::Move
      } else {
        TouchPhase::Start
      };
      if phase == TouchPhase::Move && down[index] == *finger {
        continue;
      }
      tree.finger_at(FIRST_FINGER + index as u64, phase, x, y, None, false, at);
    }
    for (index, finger) in down.iter().enumerate().skip(moment.fingers.len()).rev() {
      let (x, y) = place(*finger);
      tree.finger_at(FIRST_FINGER + index as u64, TouchPhase::End, x, y, None, false, at);
    }
    down = moment.fingers.clone();
  }
  tree.set_press_clock(saved_clock);
  tree.request_redraw();
}

pub(super) fn touch_tool(tree: &mut Tree, state: &McpState, args: &serde_json::Value) -> McpToolResult {
  let gesture = arg_str(args, "gesture").ok_or("`gesture` is required")?.to_owned();
  let (window, point) = resolve_point(tree, state, args, "ref", "x", "y")?;
  let target = window_tree_mut(tree, &window, state.include_devtools)?;
  let scale = target.scale_factor();
  let size = target.window().info().logical_size();
  let logical = point.map(|(x, y)| (x / scale, y / scale));
  let (moments, placed) = timeline(&gesture, args, (size.width, size.height), logical)?;
  let point = match (placed, point) {
    (Some((x, y)), _) => (x * scale, y * scale),
    (None, Some(point)) => point,
    (None, None) => return Err("this gesture needs a `ref` or `x`/`y` coordinates".into()),
  };
  crate::app::events::with_synthetic_input(|| play(target, &moments, point));
  Ok(McpToolOutput::Json(serde_json::json!({
    "ok": true,
    "gesture": gesture,
    "window": window,
    "x": point.0,
    "y": point.1,
    "duration_ms": moments.last().map_or(0.0, |moment| moment.at),
  })))
}
