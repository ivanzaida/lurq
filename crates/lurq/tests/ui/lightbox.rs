//! A lightbox: its top row, its stage and its strip as the design draws them, turning to another picture,
//! bringing one closer, its caption, its keys and the wheel, and over the window while its signal says so.

use std::sync::{
  Arc, Mutex,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::events::ScrollPhase,
  components::{Column, Row},
  core::Signal,
  images::ImageData,
  layout::quad::QuadContent,
  node::color::Color,
  ui::{Lightbox, Picture},
};

use super::stage::Stage;

const WHITE: Color = Color::new(255, 255, 255, 255);
const WHITE_60: Color = Color::new(255, 255, 255, 0x99);

/// Three pictures of pixels at hand, each with a title, the second with a line under it.
fn pictures() -> Vec<Picture> {
  let pixels = || ImageData::from_rgba(vec![180; 8 * 6 * 4], 8, 6);
  vec![
    Picture::new(pixels()).title("Lake at dusk"),
    Picture::new(pixels())
      .title("The ridge")
      .description("Ivan Zaida · 4032 × 3024"),
    Picture::new(pixels()),
  ]
}

/// A lightbox of 420 by 290 in a page, as the board draws it, 8 in from the page's corner.
fn in_page(lightbox: impl Fn(Lightbox) -> Lightbox + Send + Sync + 'static) -> Stage {
  let open = Signal::new(true);
  Stage::first_frame(600.0, 400.0, move |_| {
    Row::new()
      .padding(8.0)
      .child(lightbox(
        Lightbox::new(open.clone(), pictures())
          .inline(true)
          .width(420.0)
          .height(290.0)
          .id("viewer"),
      ))
      .into()
  })
}

/// The stage is laid out, and the picture then laid out in the room it has.
fn settled(stage: &mut Stage) {
  stage.frame();
  stage.pass();
}

fn images(stage: &Stage) -> usize {
  stage
    .quads()
    .iter()
    .filter(|quad| matches!(quad.content, QuadContent::Image { .. }))
    .count()
}

#[test]
fn it_is_the_top_row_the_stage_and_the_strip_of_the_design() {
  let mut stage = in_page(|lightbox| lightbox);
  settled(&mut stage);
  assert_eq!(stage.bounds("viewer"), (8.0, 8.0, 420.0, 290.0));
  assert_eq!(
    stage.fill_color("viewer"),
    Some(Color::new(0x09, 0x09, 0x0B, 0xF2)),
    "near black, in a light theme too"
  );

  // The counter in mono 12: which one in white, how many at 60%.
  assert_eq!(stage.text("1"), (WHITE, 12.0));
  assert_eq!(stage.text("/ 3"), (WHITE_60, 12.0));
  assert_eq!(stage.text("100%"), (WHITE, 13.0));

  // A top row of 52 with buttons of 36, 8 from its end; the strip of 52 along the bottom; the stage between.
  let (x, y, width, height) = stage.bounds("viewer-close");
  assert_eq!((width, height), (36.0, 36.0));
  assert_eq!((x + width, y), (8.0 + 420.0 - 8.0, 8.0 + 8.0));
  let (level_x, _, level_width, level_height) = stage.bounds("viewer-level");
  assert_eq!((level_width, level_height), (56.0, 36.0));
  let (out_x, ..) = stage.bounds("viewer-zoom-out");
  assert_eq!(level_x - (out_x + 36.0), 4.0, "4 between the buttons");
  assert_eq!(
    stage.bounds("viewer-stage"),
    (8.0, 8.0 + 52.0, 420.0, 290.0 - 52.0 - 52.0)
  );

  // The strip's pictures are 44 by 32, 6 apart, in the middle of the row.
  let (first_x, first_y, thumb_width, thumb_height) = stage.bounds("viewer-thumb-0");
  let (second_x, ..) = stage.bounds("viewer-thumb-1");
  assert_eq!((thumb_width, thumb_height), (44.0, 32.0));
  assert_eq!(second_x - (first_x + 44.0), 6.0);
  assert_eq!(first_y, 8.0 + 290.0 - 52.0 + 10.0);
  assert_eq!(first_x, 8.0 + (420.0 - (3.0 * 44.0 + 2.0 * 6.0)) / 2.0);

  // The picture itself and the three of the strip.
  assert_eq!(images(&stage), 4);
}

#[test]
fn the_round_buttons_and_the_strip_turn_to_another_picture() {
  let turned = Arc::new(Mutex::new(Vec::new()));
  let tell = turned.clone();
  let mut stage = in_page(move |lightbox| {
    let tell = tell.clone();
    lightbox.on_index_change(move |index| tell.lock().unwrap().push(index))
  });
  settled(&mut stage);
  // At the first there is none before it, and the one after is a round button of 36, 12 in from the end.
  assert!(stage.tree.get_element_by_id("viewer-previous").is_none());
  let (x, y, width, height) = stage.bounds("viewer-next");
  assert_eq!((width, height), (36.0, 36.0));
  assert_eq!(x + width, 8.0 + 420.0 - 12.0);
  let (_, stage_y, _, stage_height) = stage.bounds("viewer-stage");
  assert_eq!(
    y + height / 2.0,
    stage_y + stage_height / 2.0,
    "in the middle of the stage's side"
  );
  assert!(
    stage.paints("Lake at dusk"),
    "the picture's title lies over its lower edge"
  );

  stage.click("viewer-next");
  assert_eq!(stage.text("2"), (WHITE, 12.0));
  assert_eq!(stage.text("The ridge"), (WHITE, 14.0));
  assert_eq!(stage.text("Ivan Zaida · 4032 × 3024"), (WHITE_60, 13.0));
  assert!(stage.tree.get_element_by_id("viewer-previous").is_some());

  stage.click("viewer-thumb-2");
  assert!(stage.paints("3"));
  assert!(
    stage.tree.get_element_by_id("viewer-next").is_none(),
    "the last has none after it"
  );
  assert!(
    !stage.paints("The ridge"),
    "and no caption, having neither a title nor a line"
  );
  assert_eq!(*turned.lock().unwrap(), [1, 2]);
}

#[test]
fn with_wrap_the_first_comes_after_the_last() {
  let mut stage = in_page(|lightbox| lightbox.wrap(true));
  settled(&mut stage);
  stage.click("viewer-previous");
  assert!(stage.paints("3"), "before the first is the last");
  stage.click("viewer-next");
  assert_eq!(stage.text("1"), (WHITE, 12.0));
}

#[test]
fn the_buttons_bring_the_picture_closer_and_fit_it_again() {
  let told = Arc::new(Mutex::new(Vec::new()));
  let tell = told.clone();
  let mut stage = in_page(move |lightbox| {
    let tell = tell.clone();
    lightbox.on_zoom_change(move |zoom| tell.lock().unwrap().push((zoom * 100.0).round() as u32))
  });
  settled(&mut stage);
  stage.click("viewer-zoom-in");
  assert!(stage.paints("150%"));
  assert!(
    !stage.paints("Lake at dusk"),
    "closer than it fits, the caption is out of the way"
  );
  stage.click("viewer-zoom-in");
  assert!(stage.paints("225%"));
  stage.click("viewer-zoom-out");
  assert!(stage.paints("150%"));
  // The level fits it again.
  stage.click("viewer-level");
  assert!(stage.paints("100%"));
  assert!(stage.paints("Lake at dusk"));
  stage.click("viewer-zoom-out");
  assert!(stage.paints("100%"), "no further than it fits");
  assert_eq!(*told.lock().unwrap(), [150, 225, 150, 100]);
}

#[test]
fn it_goes_no_closer_than_it_is_told() {
  let mut stage = in_page(|lightbox| lightbox.zoom_max(2.0));
  settled(&mut stage);
  stage.click("viewer-zoom-in");
  stage.click("viewer-zoom-in");
  stage.click("viewer-zoom-in");
  assert!(stage.paints("200%"));
}

#[test]
fn the_keys_turn_bring_closer_and_fit() {
  let mut stage = in_page(|lightbox| lightbox);
  settled(&mut stage);
  // The focus is in the lightbox once something in it is pressed.
  stage.click("viewer-level");
  stage.key("ArrowRight");
  assert_eq!(stage.text("2"), (WHITE, 12.0));
  stage.key("ArrowLeft");
  assert_eq!(stage.text("1"), (WHITE, 12.0));
  stage.key("+");
  assert!(stage.paints("150%"));
  // Closer than it fits, the arrows move the picture and turn to no other.
  stage.key("ArrowRight");
  assert_eq!(stage.text("1"), (WHITE, 12.0));
  stage.key("-");
  assert!(stage.paints("100%"));
  stage.key("+");
  stage.key("0");
  assert!(stage.paints("100%"));
}

#[test]
fn the_wheel_brings_the_picture_closer_about_the_pointer() {
  let zoom = Signal::new(1.0f32);
  let shown = zoom.clone();
  let mut stage = in_page(move |lightbox| lightbox.zoom(shown.clone()));
  settled(&mut stage);
  let (x, y, width, height) = stage.bounds("viewer-stage");
  // A quarter in from the stage's corner.
  let pointer = (x + width / 4.0, y + height / 4.0);
  stage.tree.mouse_move(pointer.0, pointer.1);
  stage
    .tree
    .scroll(pointer.0, pointer.1, 0.0, -120.0, ScrollPhase::Scroll);
  stage.pass();
  assert_eq!(zoom.get(), 1.5, "the lightbox's own signal, or the app's");
  assert!(stage.paints("150%"));
  stage.tree.scroll(pointer.0, pointer.1, 0.0, 120.0, ScrollPhase::Scroll);
  stage.pass();
  assert_eq!(zoom.get(), 1.0);
}

#[test]
fn it_shows_the_picture_the_apps_signal_says() {
  let index = Signal::new(1usize);
  let which = index.clone();
  let mut stage = in_page(move |lightbox| lightbox.index(which.clone()));
  settled(&mut stage);
  assert!(stage.paints("The ridge"));
  stage.click("viewer-next");
  assert_eq!(index.get(), 2);
  index.set(0);
  stage.pass();
  assert!(stage.paints("Lake at dusk"));
}

#[test]
fn one_picture_has_no_counter_no_strip_and_no_round_buttons() {
  let open = Signal::new(true);
  let mut stage = Stage::first_frame(600.0, 400.0, move |_| {
    Row::new()
      .padding(8.0)
      .child(
        Lightbox::new(open.clone(), pictures().into_iter().take(1))
          .inline(true)
          .width(420.0)
          .height(290.0)
          .id("viewer"),
      )
      .into()
  });
  settled(&mut stage);
  assert!(!stage.paints("/ 1"));
  assert!(stage.tree.get_element_by_id("viewer-thumb-0").is_none());
  assert!(stage.tree.get_element_by_id("viewer-next").is_none());
  assert_eq!(
    stage.bounds("viewer-stage"),
    (8.0, 60.0, 420.0, 290.0 - 52.0),
    "the stage has the strip's room too"
  );
  assert_eq!(images(&stage), 1);
}

#[test]
fn it_is_over_the_window_while_its_signal_says_so_and_escape_closes_it() {
  let open = Signal::new(false);
  let dismissed = Arc::new(AtomicUsize::new(0));
  let (shown, tell) = (open.clone(), dismissed.clone());
  let mut stage = Stage::first_frame(600.0, 400.0, move |_| {
    let tell = tell.clone();
    Column::new()
      .size(600.0, 400.0)
      .child(
        Lightbox::new(shown.clone(), pictures())
          .id("viewer")
          .on_download(|_| {})
          .on_dismiss(move || {
            tell.fetch_add(1, Ordering::SeqCst);
          }),
      )
      .into()
  });
  settled(&mut stage);
  assert!(!stage.paints("/ 3"), "closed, it is not there");

  open.set(true);
  settled(&mut stage);
  assert_eq!(stage.bounds("viewer"), (0.0, 0.0, 600.0, 400.0), "it takes the window");
  assert!(stage.paints("/ 3"));
  assert!(
    stage.tree.get_element_by_id("viewer-download").is_some(),
    "told what to do, Download is there"
  );

  stage.key("Escape");
  assert!(!open.get());
  assert_eq!(dismissed.load(Ordering::SeqCst), 1);

  open.set(true);
  settled(&mut stage);
  stage.click("viewer-close");
  assert!(!open.get());
  assert_eq!(dismissed.load(Ordering::SeqCst), 2);
}
