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
//! A mouse's notch is animated and a touchpad's step is not (see
//! `notched_wheel`), and a browser tells both in pixels. What comes to about a
//! whole number of notches is taken for a mouse's and scrolls that number of
//! them; see [`is_notched`].
//!
//! Not on macOS: there a browser gives the distance the system wants
//! scrolled, as winit does on the desktop, and it is taken as it is.

use winit::event::MouseScrollDelta;

use super::WHEEL_LINE;

/// The CSS pixels a browser tells one notch of a wheel as, where it tells pixels.
const NOTCH_PIXELS: f32 = 100.0;
/// The lines a browser tells one notch as, where it tells lines.
const NOTCH_LINES: f32 = 3.0;
/// How far from a whole number of notches a mouse's wheel may be told, as a part of that number.
///
/// Chrome and Edge tell a notch on Windows as 100 pixels, less a rounding error at a scale that is no binary
/// fraction (99.999998 at 1.1). Firefox tells it as six lines of the page's font, which the default one makes 102
/// pixels at some scales and 108 at others.
const NOTCH_TOLERANCE: f32 = 0.1;

/// The notches a browser's `physical` pixels are, in a window scaled by `scale`.
fn notches(physical: f64, scale: f64) -> f32 {
  (physical / scale.max(f64::MIN_POSITIVE)) as f32 / NOTCH_PIXELS
}

/// Whether `x` and `y`, counted in notches, are a mouse's: about a whole number of them along one axis and
/// nothing along the other.
fn whole_notches(x: f32, y: f32) -> bool {
  let whole = |notches: f32| (notches - notches.round()).abs() <= NOTCH_TOLERANCE * notches.round().abs();
  whole(x) && whole(y) && (x.round() != 0.0 || y.round() != 0.0)
}

/// The pixels to scroll for `delta`, in a window scaled by `scale`. `system_distance` says the browser's pixels
/// are the distance the system wants scrolled, as on macOS.
///
/// A mouse's notches ([`is_notched`]) scroll [`WHEEL_LINE`](super::WHEEL_LINE) each, however far from a hundred
/// pixels the browser told them.
pub(super) fn scroll_distance(delta: MouseScrollDelta, scale: f64, system_distance: bool) -> (f32, f32) {
  match delta {
    MouseScrollDelta::LineDelta(x, y) => (x / NOTCH_LINES * WHEEL_LINE, y / NOTCH_LINES * WHEEL_LINE),
    MouseScrollDelta::PixelDelta(physical) if system_distance => (physical.x as f32, physical.y as f32),
    MouseScrollDelta::PixelDelta(physical) => {
      let (mut x, mut y) = (notches(physical.x, scale), notches(physical.y, scale));
      if whole_notches(x, y) {
        (x, y) = (x.round(), y.round());
      }
      (x * WHEEL_LINE, y * WHEEL_LINE)
    }
  }
}

/// Whether `delta` is of a mouse's notches, in a window scaled by `scale`, as `notched_wheel::is_notched` says
/// it of a desktop's wheel.
///
/// A whole number of lines is, as on a desktop. So are pixels that come to about a whole number of notches
/// along one axis and to nothing along the other: a touchpad tells its travel in steps of any length, most of
/// them a small part of a notch. A step of a touchpad that happens to be a notch long is taken for a mouse's
/// and animated. A notch a browser tells as another distance than [`NOTCH_PIXELS`] is taken for a touchpad's
/// and scrolls at once: a system set to scroll other than three lines a notch, a page the browser has zoomed
/// (Chrome tells a notch as 100 pixels of the screen, 80 of a page zoomed to 125%), a page whose font makes
/// Firefox's lines taller.
///
/// With `system_distance` no pixels are: the system's distance for a notch is whatever its acceleration made
/// of it, and nothing tells it from a touchpad's.
pub(super) fn is_notched(delta: MouseScrollDelta, scale: f64, system_distance: bool) -> bool {
  match delta {
    MouseScrollDelta::LineDelta(x, y) => x.fract() == 0.0 && y.fract() == 0.0,
    MouseScrollDelta::PixelDelta(_) if system_distance => false,
    MouseScrollDelta::PixelDelta(physical) => whole_notches(notches(physical.x, scale), notches(physical.y, scale)),
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

  #[test]
  fn a_notch_is_a_mouses_whatever_the_scale() {
    for scale in [1.0, 1.25, 1.5, 2.0] {
      assert!(is_notched(pixels(0.0, -100.0 * scale), scale, false), "down at {scale}");
      assert!(is_notched(pixels(0.0, 100.0 * scale), scale, false), "up at {scale}");
    }
  }

  #[test]
  fn a_notch_told_a_little_off_is_a_mouses_and_scrolls_a_notch() {
    // Chrome at a scale that is no binary fraction; Firefox's six lines of 17 pixels, and of 18.
    for (told, scale) in [(99.999_997_8, 1.1), (102.0, 1.5), (108.0, 1.25), (108.0, 3.0)] {
      let delta = pixels(0.0, -told * scale);
      assert!(is_notched(delta, scale, false), "{told} at {scale}");
      assert_eq!(
        scroll_distance(delta, scale, false),
        (0.0, -WHEEL_LINE),
        "{told} at {scale}"
      );
    }
    // Two notches of Firefox's as one event.
    assert_eq!(
      scroll_distance(pixels(0.0, 216.0 * 2.0), 2.0, false),
      (0.0, 2.0 * WHEEL_LINE)
    );
  }

  #[test]
  fn notches_told_as_one_event_are_a_mouses() {
    assert!(is_notched(pixels(0.0, -300.0 * 1.5), 1.5, false), "a fast spin");
    assert!(
      is_notched(pixels(100.0 * 1.25, 0.0), 1.25, false),
      "a wheel tilted sideways"
    );
  }

  #[test]
  fn a_touchpads_step_is_not_a_notch() {
    assert!(!is_notched(pixels(0.0, 15.0), 1.5, false));
    assert!(!is_notched(pixels(-3.0, 22.5), 1.0, false));
    assert!(
      !is_notched(pixels(0.0, -60.0 * 1.25), 1.25, false),
      "more than half a notch"
    );
    assert!(!is_notched(pixels(0.0, -140.0 * 2.0), 2.0, false), "a fast swipe");
    assert!(!is_notched(pixels(0.0, 88.0), 1.0, false), "most of a notch");
    assert!(
      !is_notched(pixels(0.0, 80.0 * 1.25), 1.25, false),
      "a notch on a page zoomed to 125%"
    );
    assert!(
      !is_notched(pixels(4.0 * 1.5, -100.0 * 1.5), 1.5, false),
      "a slanted swipe that is a notch long"
    );
    assert!(!is_notched(pixels(0.0, 0.0), 1.0, false), "no travel");
  }

  #[test]
  fn the_systems_distance_is_not_a_notch() {
    assert!(!is_notched(pixels(0.0, -100.0), 1.0, true));
    assert!(!is_notched(pixels(0.0, 200.0), 2.0, true));
  }

  #[test]
  fn a_whole_number_of_lines_is_a_mouses_notches() {
    assert!(is_notched(MouseScrollDelta::LineDelta(0.0, -3.0), 1.5, false));
    assert!(is_notched(MouseScrollDelta::LineDelta(1.0, 0.0), 1.0, true));
    assert!(!is_notched(MouseScrollDelta::LineDelta(0.0, 0.25), 1.0, false));
  }
}
