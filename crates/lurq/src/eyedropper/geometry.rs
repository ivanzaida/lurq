//! Where a pick reads and draws, in the physical pixels of the virtual desktop: every monitor in one plane, the
//! primary's top left at `(0, 0)`, a monitor to its left or above at negative coordinates. Kept apart from the
//! platform so that it is tested everywhere.

/// A rectangle of physical pixels, `right` and `bottom` past its last column and row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Rect {
  pub left: i32,
  pub top: i32,
  pub right: i32,
  pub bottom: i32,
}

impl Rect {
  pub(crate) fn new(left: i32, top: i32, width: i32, height: i32) -> Self {
    Self {
      left,
      top,
      right: left + width,
      bottom: top + height,
    }
  }

  pub(crate) fn width(&self) -> i32 {
    self.right - self.left
  }

  pub(crate) fn height(&self) -> i32 {
    self.bottom - self.top
  }

  pub(crate) fn contains(&self, (x, y): (i32, i32)) -> bool {
    x >= self.left && x < self.right && y >= self.top && y < self.bottom
  }
}

/// How many pixels the loupe shows across: the one taken in the middle and five on each side.
pub(crate) const CELLS: i32 = 11;

/// The pixels the loupe shows around `cursor`, row by row from the top left: each its point on the desktop, or `None`
/// where it is off every one of `monitors`.
pub(crate) fn cells(cursor: (i32, i32), monitors: &[Rect]) -> Vec<Option<(i32, i32)>> {
  let half = CELLS / 2;
  (-half..=half)
    .flat_map(|dy| (-half..=half).map(move |dx| (cursor.0 + dx, cursor.1 + dy)))
    .map(|point| monitors.iter().any(|monitor| monitor.contains(point)).then_some(point))
    .collect()
}

/// The rectangle around every one of `monitors`: what a picture of the whole desktop covers.
pub(crate) fn around(monitors: &[Rect]) -> Rect {
  monitors
    .iter()
    .copied()
    .reduce(|all, monitor| Rect {
      left: all.left.min(monitor.left),
      top: all.top.min(monitor.top),
      right: all.right.max(monitor.right),
      bottom: all.bottom.max(monitor.bottom),
    })
    .unwrap_or(Rect::new(0, 0, 0, 0))
}

/// The place of `point` in a picture of `desktop` read whole, row by row: the offset of its pixel. `None` off it.
pub(crate) fn offset_in(point: (i32, i32), desktop: Rect) -> Option<usize> {
  desktop
    .contains(point)
    .then(|| ((point.1 - desktop.top) * desktop.width() + (point.0 - desktop.left)) as usize)
}

/// `logical` pixels at a monitor of `dpi` (96 is 100%), rounded to the nearest physical pixel.
pub(crate) fn scaled(logical: i32, dpi: u32) -> i32 {
  ((logical as f64) * (dpi as f64) / 96.0).round() as i32
}

/// Where a loupe of `size` stands for a pointer at `cursor` on `monitor`: `gap` past the pointer to its right and below
/// it, or to its left or above where that would leave the monitor, and inside the monitor whatever it is.
pub(crate) fn loupe_origin(cursor: (i32, i32), size: (i32, i32), gap: i32, monitor: Rect) -> (i32, i32) {
  let along = |at: i32, length: i32, start: i32, end: i32| {
    let after = at + gap;
    let place = if after + length <= end {
      after
    } else {
      at - gap - length
    };
    place.clamp(start, (end - length).max(start))
  };
  (
    along(cursor.0, size.0, monitor.left, monitor.right),
    along(cursor.1, size.1, monitor.top, monitor.bottom),
  )
}

#[cfg(test)]
mod tests {
  use super::*;

  /// A desktop of two monitors: a 1920 by 1080 one to the left of the primary, and the primary, 2560 by 1440 at 150%.
  fn two_monitors() -> (Rect, Rect, Rect) {
    let left = Rect::new(-1920, 0, 1920, 1080);
    let primary = Rect::new(0, 0, 2560, 1440);
    (left, primary, around(&[left, primary]))
  }

  #[test]
  fn the_middle_cell_is_the_pixel_under_the_pointer_on_any_monitor() {
    let (left, primary, desktop) = two_monitors();
    assert_eq!(desktop, Rect::new(-1920, 0, 4480, 1440));
    for cursor in [(100, 200), (-1500, 300), (-1, 0), (2559, 1439)] {
      let cells = cells(cursor, &[left, primary]);
      assert_eq!(cells.len(), 121);
      assert_eq!(cells[60], Some(cursor), "the middle of 11 by 11 is the pointer's");
    }
    assert_eq!(cells((100, 200), &[left, primary])[0], Some((95, 195)));
  }

  #[test]
  fn cells_off_the_desktop_are_none() {
    let (left, primary, _) = two_monitors();
    let monitors = [left, primary];
    // The top left corner of the left monitor: five rows above and five columns before are off every monitor.
    let corner = cells((-1920, 0), &monitors);
    assert_eq!(
      corner.iter().filter(|cell| cell.is_some()).count(),
      36,
      "6 by 6 are on it"
    );
    assert_eq!(corner[60], Some((-1920, 0)));
    assert_eq!(corner[0], None);
    // At the bottom of the left monitor, which is 1080 high, beside the primary, which is 1440: the rows under the
    // left monitor are off it, those of the primary are on.
    let edge = cells((-1, 1079), &monitors);
    assert_eq!(edge[60], Some((-1, 1079)));
    assert_eq!(edge[60 + 11], None, "under the left monitor");
    assert_eq!(edge[60 + 11 + 1], Some((0, 1080)), "the primary goes on lower");
  }

  #[test]
  fn a_pixel_of_a_desktop_read_whole_is_found_from_its_left_top() {
    let (_, _, desktop) = two_monitors();
    assert_eq!(offset_in((-1920, 0), desktop), Some(0));
    assert_eq!(offset_in((0, 0), desktop), Some(1920), "the primary starts 1920 in");
    assert_eq!(offset_in((-1919, 1), desktop), Some(4480 + 1));
    assert_eq!(offset_in((2559, 1439), desktop), Some(4480 * 1440 - 1));
    assert_eq!(offset_in((2560, 0), desktop), None);
    assert_eq!(offset_in((-1921, 0), desktop), None);
  }

  #[test]
  fn sizes_follow_the_scale_of_the_monitor() {
    assert_eq!(scaled(10, 96), 10);
    assert_eq!(scaled(10, 144), 15, "150%");
    assert_eq!(scaled(11, 144), 17, "16.5 rounds up");
    assert_eq!(scaled(122, 120), 153, "125%");
    assert_eq!(scaled(10, 192), 20, "200%");
  }

  #[test]
  fn the_loupe_stands_beside_the_pointer_and_inside_its_monitor() {
    let (left, primary, _) = two_monitors();
    let size = (183, 222);
    assert_eq!(
      loupe_origin((100, 100), size, 36, primary),
      (136, 136),
      "right and below"
    );
    // Near the right and the bottom edges, it goes to the left and above.
    assert_eq!(
      loupe_origin((2500, 1400), size, 36, primary),
      (2500 - 36 - 183, 1400 - 36 - 222)
    );
    // On the left monitor near its right edge (the primary's left), it stays on the left monitor.
    let (x, y) = loupe_origin((-20, 500), size, 24, left);
    assert!(x + size.0 <= 0 && x >= -1920, "{x}");
    assert_eq!(y, 524);
    // A monitor too small for it either way: at its start.
    let tiny = Rect::new(0, 0, 100, 100);
    assert_eq!(loupe_origin((50, 50), size, 24, tiny), (0, 0));
  }
}
