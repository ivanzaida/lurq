//! How far a turn of the wheel scrolls in a browser: as far as on a desktop.
//!
//! On a desktop winit tells a wheel's notch as one line, which the shell
//! scrolls [`WHEEL_LINE`](super::WHEEL_LINE) pixels for. A browser tells the
//! same notch as the distance it would scroll a page itself: about a hundred
//! CSS pixels, which winit turns into physical ones. Taken as it comes that is
//! 150 pixels a notch on a display scaled by 1.5, where the desktop app
//! scrolls 40.
//!
//! So a browser's pixels are counted back into notches. A touchpad's small
//! steps come the same way, as parts of a notch, as they do on a desktop.
//!
//! Not on macOS: there a browser gives the distance the system wants
//! scrolled, as winit does on the desktop, and it is taken as it is.

use winit::event::MouseScrollDelta;

use super::WHEEL_LINE;

/// The CSS pixels a browser tells one notch of a wheel as, where it tells pixels.
const NOTCH_PIXELS: f32 = 100.0;
/// The lines a browser tells one notch as, where it tells lines.
const NOTCH_LINES: f32 = 3.0;

/// The pixels to scroll for `delta`, in a window scaled by `scale`. `system_distance` says the browser's pixels
/// are the distance the system wants scrolled, as on macOS.
pub(super) fn scroll_distance(delta: MouseScrollDelta, scale: f64, system_distance: bool) -> (f32, f32) {
  match delta {
    MouseScrollDelta::LineDelta(x, y) => (x / NOTCH_LINES * WHEEL_LINE, y / NOTCH_LINES * WHEEL_LINE),
    MouseScrollDelta::PixelDelta(physical) if system_distance => (physical.x as f32, physical.y as f32),
    MouseScrollDelta::PixelDelta(physical) => {
      let notches = |pixels: f64| (pixels / scale.max(f64::MIN_POSITIVE)) as f32 / NOTCH_PIXELS;
      (notches(physical.x) * WHEEL_LINE, notches(physical.y) * WHEEL_LINE)
    }
  }
}

#[cfg(test)]
mod tests {
  use winit::dpi::PhysicalPosition;

  use super::*;

  fn pixels(x: f64, y: f64) -> MouseScrollDelta {
    MouseScrollDelta::PixelDelta(PhysicalPosition::new(x, y))
  }

  #[test]
  fn a_notch_scrolls_as_far_as_on_a_desktop_whatever_the_scale() {
    // A notch is 100 CSS pixels, which winit gives multiplied by the scale.
    assert_eq!(scroll_distance(pixels(0.0, -100.0), 1.0, false), (0.0, -WHEEL_LINE));
    assert_eq!(scroll_distance(pixels(0.0, -150.0), 1.5, false), (0.0, -WHEEL_LINE));
    assert_eq!(scroll_distance(pixels(200.0, 0.0), 2.0, false), (WHEEL_LINE, 0.0));
  }

  #[test]
  fn a_touchpads_step_is_a_part_of_a_notch() {
    let (_, y) = scroll_distance(pixels(0.0, 15.0), 1.5, false);
    assert!((y - WHEEL_LINE * 0.1).abs() < 1e-4, "{y}");
  }

  #[test]
  fn a_notch_told_in_lines_is_one_notch() {
    assert_eq!(
      scroll_distance(MouseScrollDelta::LineDelta(0.0, -3.0), 1.5, false),
      (0.0, -WHEEL_LINE)
    );
  }

  #[test]
  fn the_systems_distance_is_taken_as_it_is() {
    assert_eq!(scroll_distance(pixels(4.0, -30.0), 2.0, true), (4.0, -30.0));
  }
}
