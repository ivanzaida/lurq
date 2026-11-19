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

/// How much of the pixel at `(x, y)` a rectangle of `width` by `height` from `(0, 0)` with corners of `radius`
/// covers, from 0 to 1: whole inside, a share of it on a corner's arc, none past it.
pub(crate) fn rounded_coverage(x: i32, y: i32, width: i32, height: i32, radius: f32) -> f32 {
  if x < 0 || y < 0 || x >= width || y >= height {
    return 0.0;
  }
  let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
  let radius = radius.max(0.0).min(width as f32 / 2.0).min(height as f32 / 2.0);
  // The center of the corner's circle the pixel is beside, where it is in a corner.
  let cx = if px < radius {
    radius
  } else if px > width as f32 - radius {
    width as f32 - radius
  } else {
    return 1.0;
  };
  let cy = if py < radius {
    radius
  } else if py > height as f32 - radius {
    height as f32 - radius
  } else {
    return 1.0;
  };
  let distance = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt();
  (radius - distance + 0.5).clamp(0.0, 1.0)
}

/// Makes the loupe's picture whole, as one finished frame: `pixels` (row by row, `0x00RRGGBB`, as GDI drew them) of a
/// loupe of `width` by `height` become premultiplied `0xAARRGGBB` with its corners of `radius` cut out, smooth, and an
/// edge of one pixel in `border` that follows them. What a layered window is given to show at once.
pub(crate) fn finish_loupe(pixels: &mut [u32], width: i32, height: i32, radius: f32, border: (u8, u8, u8)) {
  for y in 0..height {
    for x in 0..width {
      let at = (y * width + x) as usize;
      let Some(pixel) = pixels.get_mut(at) else {
        return;
      };
      let outer = rounded_coverage(x, y, width, height, radius);
      let inner = rounded_coverage(x - 1, y - 1, width - 2, height - 2, (radius - 1.0).max(0.0));
      let channel = |shift: u32, edge: u8| {
        let drawn = ((*pixel >> shift) & 0xff) as f32;
        let color = edge as f32 + (drawn - edge as f32) * inner;
        ((color * outer).round() as u32).min(255)
      };
      let (r, g, b) = (channel(16, border.0), channel(8, border.1), channel(0, border.2));
      let a = ((outer * 255.0).round() as u32).min(255);
      *pixel = (a << 24) | (r << 16) | (g << 8) | b;
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_finished_loupe_is_opaque_inside_with_its_corners_cut_and_its_edge_drawn() {
    let (width, height, radius) = (40, 30, 8.0);
    let panel = 0x0018_181b;
    let mut pixels = vec![panel; (width * height) as usize];
    // A cell drawn in the middle keeps its color.
    pixels[(15 * width + 20) as usize] = 0x0025_63eb;
    finish_loupe(&mut pixels, width, height, radius, (0x3f, 0x3f, 0x46));
    let at = |x: i32, y: i32| pixels[(y * width + x) as usize];
    assert_eq!(at(20, 15), 0xff25_63eb, "the middle opaque and as drawn");
    assert_eq!(at(20, 5), 0xff18_181b, "the panel opaque");
    assert_eq!(at(0, 0), 0, "a corner cut out: transparent, nothing under its alpha");
    assert_eq!(at(width - 1, height - 1), 0);
    assert_eq!(at(20, 0), 0xff3f_3f46, "the edge at the top");
    assert_eq!(at(0, 15), 0xff3f_3f46, "the edge at the left");
    // On a corner's arc: partly covered, premultiplied (no channel over its alpha).
    let arc = at(2, 2);
    let alpha = arc >> 24;
    assert!(alpha > 0 && alpha < 255, "{arc:08x}");
    assert!(
      [(arc >> 16) & 0xff, (arc >> 8) & 0xff, arc & 0xff]
        .iter()
        .all(|channel| *channel <= alpha)
    );
    // Every pixel is decided: none is left as GDI drew it, with no alpha.
    assert!(
      pixels
        .iter()
        .all(|pixel| *pixel >> 24 == 0xff || *pixel >> 24 < 0xff && *pixel != panel)
    );
  }

  #[test]
  fn coverage_is_whole_inside_partial_on_an_arc_and_none_past_it() {
    assert_eq!(rounded_coverage(10, 10, 20, 20, 4.0), 1.0);
    assert_eq!(rounded_coverage(0, 10, 20, 20, 4.0), 1.0, "a straight edge");
    assert_eq!(rounded_coverage(0, 0, 20, 20, 4.0), 0.0, "past the arc");
    let on = rounded_coverage(1, 1, 20, 20, 4.0);
    assert!(on > 0.0 && on < 1.0, "{on}");
    assert_eq!(rounded_coverage(-1, 5, 20, 20, 4.0), 0.0, "outside");
    assert_eq!(rounded_coverage(0, 0, 20, 20, 0.0), 1.0, "square corners");
  }

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
