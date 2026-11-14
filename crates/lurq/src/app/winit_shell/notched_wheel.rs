//! Which wheel events are a mouse's notches. A notch is animated ([`Tree::wheel`](crate::app::Tree::wheel)); the
//! travel of a touchpad or of a precise wheel moves the content with the fingers
//! ([`Tree::scroll`](crate::app::Tree::scroll)), and animating it would fall behind them.

use winit::event::MouseScrollDelta;

/// Whether a wheel event is of a mouse's notches.
///
/// A delta in pixels is a touchpad's. One in lines is a mouse's on macOS, where a touchpad tells pixels and the
/// system gives a slow wheel part of a line. Windows and X11 tell a touchpad's travel in lines too, a small part
/// of one with every event (`WM_MOUSEWHEEL` with less than `WHEEL_DELTA`, a scroll valuator's fraction), as
/// they tell a wheel that turns without notches: there a mouse's notch is a whole number of lines.
pub(super) fn is_notched(delta: MouseScrollDelta) -> bool {
  match delta {
    MouseScrollDelta::LineDelta(x, y) => cfg!(target_os = "macos") || (x.fract() == 0.0 && y.fract() == 0.0),
    MouseScrollDelta::PixelDelta(_) => false,
  }
}

#[cfg(test)]
mod tests {
  use winit::{dpi::PhysicalPosition, event::MouseScrollDelta};

  use super::is_notched;

  #[test]
  fn a_whole_number_of_lines_is_a_mouses_notches() {
    assert!(is_notched(MouseScrollDelta::LineDelta(0.0, 1.0)));
    assert!(
      is_notched(MouseScrollDelta::LineDelta(0.0, -3.0)),
      "a fast spin, told as one event"
    );
    assert!(
      is_notched(MouseScrollDelta::LineDelta(-1.0, 0.0)),
      "a wheel tilted sideways"
    );
  }

  #[test]
  fn pixels_are_a_touchpads() {
    assert!(!is_notched(MouseScrollDelta::PixelDelta(PhysicalPosition::new(
      0.0, 40.0
    ))));
  }

  #[test]
  #[cfg(not(target_os = "macos"))]
  fn part_of_a_line_is_a_touchpads_where_the_system_tells_it_in_lines() {
    assert!(!is_notched(MouseScrollDelta::LineDelta(0.0, 0.125)));
    assert!(!is_notched(MouseScrollDelta::LineDelta(0.0, -1.5)));
  }
}
