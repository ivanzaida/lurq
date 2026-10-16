//! How the selected texts of several nodes join into the copied text: one
//! line per visual line, its texts in x order with a space between them.
//!
//! A visual line is the texts drawn side by side: their boxes share some
//! height and no width. The cells of a table row are one line however tall
//! each cell is and wherever its words sit in it; paragraphs stacked in a
//! column, or texts stacked beside a taller one, are lines of their own.

/// The gap, in logical pixels, below which two boxes count as touching, not
/// overlapping, and two texts as joined, not apart.
const EPSILON: f32 = 1.0;

/// The selected text of one node and where it is drawn.
pub(super) struct SelectedTextFragment {
  /// The left and right of the selection's laid-out lines.
  pub(super) x: f32,
  pub(super) end_x: f32,
  /// The top and bottom of the node's box.
  pub(super) top: f32,
  pub(super) bottom: f32,
  pub(super) text: String,
}

impl SelectedTextFragment {
  /// Whether `other` is drawn beside this fragment, on the same visual line.
  fn beside(&self, other: &Self) -> bool {
    overlap(self.top, self.bottom, other.top, other.bottom) && !overlap(self.x, self.end_x, other.x, other.end_x)
  }
}

fn overlap(start: f32, end: f32, other_start: f32, other_end: f32) -> bool {
  start < other_end - EPSILON && other_start < end - EPSILON
}

/// The copied text of `fragments`, in reading order.
pub(super) fn join_selected_text(mut fragments: Vec<SelectedTextFragment>) -> String {
  fragments.sort_by(|a, b| a.top.total_cmp(&b.top).then(a.x.total_cmp(&b.x)));
  let mut lines: Vec<Vec<SelectedTextFragment>> = Vec::new();
  for fragment in fragments {
    match lines.last_mut() {
      Some(line) if line.iter().all(|other| fragment.beside(other)) => line.push(fragment),
      _ => lines.push(vec![fragment]),
    }
  }

  let mut selected = String::new();
  for (index, line) in lines.iter_mut().enumerate() {
    if index > 0 {
      selected.push('\n');
    }
    line.sort_by(|a, b| a.x.total_cmp(&b.x));
    let mut previous_end_x: Option<f32> = None;
    for fragment in line.iter() {
      if let Some(end_x) = previous_end_x
        && fragment.x - end_x > EPSILON
        && !selected.chars().last().is_some_and(char::is_whitespace)
        && !fragment.text.chars().next().is_some_and(char::is_whitespace)
      {
        selected.push(' ');
      }
      selected.push_str(&fragment.text);
      previous_end_x = Some(fragment.end_x);
    }
  }
  selected
}

#[cfg(test)]
mod tests {
  use super::{SelectedTextFragment, join_selected_text};

  fn fragment(text: &str, [x, top, end_x, bottom]: [f32; 4]) -> SelectedTextFragment {
    SelectedTextFragment {
      x,
      end_x,
      top,
      bottom,
      text: text.to_owned(),
    }
  }

  #[test]
  fn texts_stacked_beside_a_taller_text_keep_their_own_lines() {
    let copied = join_selected_text(vec![
      fragment("third", [200.0, 40.0, 260.0, 60.0]),
      fragment("tall", [0.0, 0.0, 100.0, 60.0]),
      fragment("second", [200.0, 20.0, 260.0, 40.0]),
      fragment("first", [200.0, 0.0, 260.0, 20.0]),
    ]);
    assert_eq!(copied, "tall first\nsecond\nthird");
  }

  #[test]
  fn boxes_that_only_touch_are_lines_of_their_own() {
    let copied = join_selected_text(vec![
      fragment("below", [0.0, 20.0, 50.0, 40.0]),
      fragment("above", [60.0, 0.0, 110.0, 20.5]),
    ]);
    assert_eq!(copied, "above\nbelow");
  }
}
