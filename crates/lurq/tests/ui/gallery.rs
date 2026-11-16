//! A gallery: its rows as the design draws them, the picture that stands for the rest, choosing among its
//! pictures by their keys, and the lightbox that holds them all.

use std::sync::{Arc, Mutex};

use lurq::{
  components::{Column, Row},
  core::Signal,
  images::ImageData,
  layout::quad::QuadContent,
  ui::{Gallery, Picture},
};

use super::stage::Stage;

fn pictures(count: usize) -> Vec<Picture> {
  (0..count)
    .map(|at| Picture::new(ImageData::from_rgba(vec![160; 8 * 6 * 4], 8, 6)).label(&format!("Picture {at}")))
    .collect()
}

/// A gallery of 392 wide, as the board draws it, 8 in from the corner of a window of 600 by 500.
fn in_page(gallery: impl Fn() -> Gallery + Send + Sync + 'static) -> Stage {
  let mut stage = Stage::first_frame(600.0, 500.0, move |_| {
    Row::new()
      .padding(8.0)
      .child(gallery().width(392.0).id("photos"))
      .into()
  });
  stage.pass();
  stage
}

fn images(stage: &Stage) -> usize {
  stage
    .quads()
    .iter()
    .filter(|quad| matches!(quad.content, QuadContent::Image { .. }))
    .count()
}

#[test]
fn it_is_rows_of_four_pictures_of_92_by_68_eight_apart() {
  let mut stage = in_page(|| Gallery::new(pictures(8)));
  assert_eq!(stage.bounds("photos"), (8.0, 8.0, 392.0, 144.0));
  assert_eq!(stage.bounds("photos-0"), (8.0, 8.0, 92.0, 68.0));
  assert_eq!(stage.bounds("photos-1"), (8.0 + 100.0, 8.0, 92.0, 68.0));
  assert_eq!(stage.bounds("photos-4"), (8.0, 8.0 + 76.0, 92.0, 68.0));
  assert_eq!(stage.bounds("photos-7"), (8.0 + 300.0, 8.0 + 76.0, 92.0, 68.0));
  assert_eq!(images(&stage), 8);
}

#[test]
fn a_row_it_does_not_fill_keeps_its_pictures_as_wide_as_the_others() {
  let mut stage = in_page(|| Gallery::new(pictures(6)).columns(4));
  assert_eq!(stage.bounds("photos-5"), (8.0 + 100.0, 8.0 + 76.0, 92.0, 68.0));
  // Three in a row are wider, and as much higher.
  let mut stage = in_page(|| Gallery::new(pictures(3)).columns(3).gap(10.0).aspect_ratio(1.0));
  let (_, _, width, height) = stage.bounds("photos-2");
  assert_eq!((width, height), (124.0, 124.0));
  assert_eq!(stage.bounds("photos-1").0, 8.0 + 134.0);
}

#[test]
fn the_last_one_shown_stands_for_the_rest() {
  let mut stage = in_page(|| Gallery::new(pictures(12)).max_shown(8));
  assert!(stage.tree.get_element_by_id("photos-7").is_some());
  assert!(stage.tree.get_element_by_id("photos-8").is_none());
  assert!(stage.paints("+5"), "itself and the four after it");
  let stage = in_page(|| {
    Gallery::new(pictures(12))
      .max_shown(8)
      .more_text(|more| format!("{more} more"))
  });
  assert!(stage.paints("5 more"));
  let stage = in_page(|| Gallery::new(pictures(8)).max_shown(8));
  assert!(
    !stage.paints("+1"),
    "with no more than it shows, none stands for another"
  );
}

#[test]
fn a_press_opens_the_picture_in_a_lightbox_of_them_all() {
  let open = Signal::new(None::<usize>);
  let told = Arc::new(Mutex::new(Vec::new()));
  let (which, tell) = (open.clone(), told.clone());
  let mut stage = in_page(move || {
    let tell = tell.clone();
    Gallery::new(pictures(8))
      .open(which.clone())
      .on_open_change(move |at| tell.lock().unwrap().push(at))
  });
  assert!(!stage.paints("/ 8"), "closed, the lightbox is not there");

  stage.click("photos-2");
  stage.pass();
  assert_eq!(open.get(), Some(2));
  assert!(stage.paints("/ 8"), "the lightbox's counter");
  assert!(stage.paints("3"));

  stage.key("Escape");
  stage.pass();
  assert_eq!(open.get(), None);
  assert!(!stage.paints("/ 8"));
  assert_eq!(*told.lock().unwrap(), [Some(2), None]);

  // The app opens one itself.
  open.set(Some(5));
  stage.pass();
  stage.pass();
  assert!(stage.paints("/ 8") && stage.paints("6"));
}

#[test]
fn told_not_to_open_it_tells_the_app_which_was_pressed() {
  let pressed = Arc::new(Mutex::new(Vec::new()));
  let tell = pressed.clone();
  let mut stage = in_page(move || {
    let tell = tell.clone();
    Gallery::new(pictures(8))
      .lightbox(false)
      .on_select(move |at| tell.lock().unwrap().push(at))
  });
  stage.click("photos-6");
  assert_eq!(*pressed.lock().unwrap(), [6]);
  assert!(!stage.paints("/ 8"));
}

#[test]
fn a_press_chooses_by_the_apps_keys_and_enter_opens() {
  let chosen = Signal::new(vec!["b"]);
  let open = Signal::new(None::<usize>);
  let (keys, which) = (chosen.clone(), open.clone());
  let mut stage = Stage::first_frame(600.0, 500.0, move |_| {
    let named = ["a", "b", "c", "d"].into_iter().zip(pictures(4));
    Row::new()
      .padding(8.0)
      .child(
        Gallery::keyed(named)
          .selectable(true)
          .selected(keys.clone())
          .open(which.clone())
          .width(392.0)
          .id("photos"),
      )
      .into()
  });
  stage.pass();
  stage.click("photos-2");
  assert_eq!(chosen.get(), ["b", "c"], "by its key, after the ones chosen before it");
  assert_eq!(open.get(), None, "a press chooses, and opens nothing");
  stage.click("photos-1");
  assert_eq!(chosen.get(), ["c"], "a press of a chosen one takes it out");

  // The focus is on the picture pressed last: Enter opens it, and chooses nothing.
  stage.key("Enter");
  stage.pass();
  assert_eq!(open.get(), Some(1));
  assert_eq!(chosen.get(), ["c"]);
}

#[test]
fn without_a_signal_it_keeps_what_is_chosen_itself() {
  let told = Arc::new(Mutex::new(Vec::new()));
  let tell = told.clone();
  let mut stage = in_page(move || {
    let tell = tell.clone();
    Gallery::new(pictures(4))
      .selectable(true)
      .on_selection_change(move |keys| tell.lock().unwrap().push(keys.to_vec()))
  });
  stage.click("photos-0");
  stage.click("photos-3");
  stage.click("photos-0");
  assert_eq!(*told.lock().unwrap(), [vec![0], vec![0, 3], vec![3]]);
}

#[test]
fn a_gallery_wider_than_its_room_is_as_wide_as_the_room() {
  let mut stage = Stage::first_frame(600.0, 500.0, |_| {
    Column::new()
      .size(600.0, 500.0)
      .child(
        Row::new()
          .width(200.0)
          .child(Gallery::new(pictures(8)).width(392.0).id("photos")),
      )
      .into()
  });
  stage.pass();
  let (x, _, width, _) = stage.bounds("photos");
  assert_eq!((x, width), (0.0, 200.0));
  // Two in a row: four of 92 do not fit.
  let (_, _, cell, _) = stage.bounds("photos-0");
  assert_eq!(cell, (200.0 - 8.0) / 2.0);
}

/// A gallery of eight pictures, at most six in a row, in a room `room` wide: how many stand in its first row, and
/// how wide the first picture is, in the first frame and once it has settled.
fn in_a_row_at(room: f32, gallery: impl Fn() -> Gallery + Send + Sync + 'static) -> [(usize, f32); 2] {
  let mut stage = Stage::first_frame(1000.0, 600.0, move |_| {
    Column::new()
      .size(1000.0, 600.0)
      .child(Row::new().width(room).child(gallery().id("photos")))
      .into()
  });
  let mut read = |stage: &mut Stage| {
    let (_, top, width, _) = stage.bounds("photos-0");
    let in_a_row = (0..8)
      .take_while(|at| stage.bounds(&format!("photos-{at}")).1 == top)
      .count();
    (in_a_row, width)
  };
  let first = read(&mut stage);
  stage.pass();
  [first, read(&mut stage)]
}

#[test]
fn a_narrow_room_has_fewer_pictures_in_a_row_from_the_first_frame() {
  let gallery = || Gallery::new(pictures(8)).columns(6);
  for (room, in_a_row) in [(300.0, 3), (520.0, 5), (900.0, 6)] {
    let [first, settled] = in_a_row_at(room, gallery);
    let width = (room - (in_a_row - 1) as f32 * 8.0) / in_a_row as f32;
    assert_eq!(settled.0, in_a_row, "at {room}");
    assert!((settled.1 - width).abs() < 0.01, "at {room}: {} wide", settled.1);
    assert_eq!(first, settled, "at {room}, the first frame is the settled one");
  }
}

#[test]
fn a_least_width_of_nothing_keeps_the_columns_in_any_room() {
  let [_, settled] = in_a_row_at(300.0, || Gallery::new(pictures(8)).columns(6).min_picture_width(0.0));
  assert_eq!(settled.0, 6);
  let [_, settled] = in_a_row_at(300.0, || Gallery::new(pictures(8)).columns(6).min_picture_width(140.0));
  assert_eq!(settled.0, 2);
}
