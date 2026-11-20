//! A checkbox's, a radio's and a switch's label over several lines: within the width the control is given, the
//! mark beside its first line.

use std::sync::{Arc, Mutex};

use lurq::{
  components::Column,
  node::{Element, dimension::Dimension},
  ui::{Checkbox, Radio, Switch},
};

use super::stage::Stage;

const LONG: &str = "Keep the worktree of every run in the project's folder until it is merged or removed by hand";

/// The controls `make` builds for a label, in a column 200 wide.
fn stage(make: impl Fn(&str, &str) -> Element + Send + Sync + 'static) -> Stage {
  Stage::new(move |_| {
    Column::new()
      .width(Dimension::Px(200.0))
      .spacing(12.0)
      .child(make("long", LONG))
      .child(make("short", "Sync"))
      .into()
  })
}

/// That the control of `id` wraps its long label within 200, its mark (`mark`) on the label's first line, and that
/// a short label leaves it narrow.
fn wraps_beside_first_line(stage: &mut Stage, id: &str, mark: &str) {
  let (label_x, label_y, label_width, label_height) = stage.bounds(&format!("{id}-label"));
  let (_, short_y, _, short_height) = stage.bounds("short-label");
  assert!(label_height >= short_height * 2.0, "{id}: more lines ({label_height} by {short_height})");
  assert!(label_x + label_width <= 200.5, "{id}: within the width given ({label_x} + {label_width})");
  let (_, mark_y, _, mark_height) = stage.bounds(mark);
  let line_middle = label_y + short_height / 2.0;
  assert!(
    (mark_y + mark_height / 2.0 - line_middle).abs() <= 1.0,
    "{id}: the mark on the first line ({mark_y} + {mark_height} by {label_y} + {short_height})"
  );
  let (_, short_mark_y, _, short_mark_height) = stage.bounds(&mark.replacen(id, "short", 1));
  assert!(
    (short_mark_y + short_mark_height / 2.0 - (short_y + short_height / 2.0)).abs() <= 1.0,
    "{id}: a short label's mark on its line"
  );
  let (short_label_x, _, short_label_width, _) = stage.bounds("short-label");
  assert!(short_label_x + short_label_width < 150.0, "a short label leaves the control narrow");
}

#[test]
fn a_checkbox_s_label_wraps_with_its_box_beside_the_first_line() {
  let mut stage = stage(|id, label| Checkbox::new(label).label_wraps(true).id(id).into());
  wraps_beside_first_line(&mut stage, "long", "long");
}

#[test]
fn a_checkbox_pressed_as_a_whole_wraps_its_label_and_a_press_of_any_line_checks_it() {
  let asked = Arc::new(Mutex::new(Vec::new()));
  let told = asked.clone();
  let mut stage = stage(move |id, label| {
    let told = told.clone();
    Checkbox::new(label)
      .label_wraps(true)
      .whole(true)
      .on_change(move |next| told.lock().unwrap().push(next))
      .id(id)
      .into()
  });
  wraps_beside_first_line(&mut stage, "long", "long-box");
  let (_, _, whole_width, _) = stage.bounds("long");
  assert!(whole_width <= 200.5, "the whole within the width given");
  stage.click("long-label");
  assert_eq!(*asked.lock().unwrap(), [true]);
}

#[test]
fn a_radio_s_and_a_switch_s_labels_wrap_with_their_mark_beside_the_first_line() {
  let mut radios = stage(|id, label| Radio::new(label).label_wraps(true).id(id).into());
  wraps_beside_first_line(&mut radios, "long", "long");
  let mut switches = stage(|id, label| Switch::new(label).label_wraps(true).whole(true).id(id).into());
  wraps_beside_first_line(&mut switches, "long", "long-track");
}

#[test]
fn a_label_stays_on_one_line_unless_told() {
  let mut stage = stage(|id, label| Checkbox::new(label).id(id).into());
  let (_, _, _, long_height) = stage.bounds("long-label");
  let (_, _, _, short_height) = stage.bounds("short-label");
  assert_eq!(long_height, short_height, "one line");
}

