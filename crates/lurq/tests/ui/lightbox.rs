//! A lightbox: its top row, its stage and its strip as the design draws them, turning to another picture,
//! bringing one closer, its caption, its keys and the wheel, and over the window while its signal says so.

#![cfg(feature = "raster")]

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
  ui::{Lightbox, LightboxVariant, Picture},
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
  Stage::first_frame(800.0, 400.0, move |_| {
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
fn the_strip_shows_as_many_as_it_has_room_for_about_the_one_shown() {
  let open = Signal::new(true);
  let index = Signal::new(0usize);
  let which = index.clone();
  let mut stage = Stage::first_frame(800.0, 400.0, move |_| {
    let pixels = || ImageData::from_rgba(vec![180; 8 * 6 * 4], 8, 6);
    Row::new()
      .padding(8.0)
      .child(
        Lightbox::new(open.clone(), (0..12).map(|_| Picture::new(pixels())))
          .index(which.clone())
          .inline(true)
          .width(420.0)
          .height(290.0)
          .id("viewer"),
      )
      .into()
  });
  settled(&mut stage);
  let shown = |stage: &mut Stage| {
    (0..12)
      .filter(|at| stage.tree.get_element_by_id(&format!("viewer-thumb-{at}")).is_some())
      .collect::<Vec<_>>()
  };
  // Eight of 44 with 6 between them are 394, and a ninth would not leave 8 at each end of 420.
  assert_eq!(shown(&mut stage), (0..8).collect::<Vec<_>>());
  let (first_x, ..) = stage.bounds("viewer-thumb-0");
  assert_eq!(
    first_x,
    8.0 + (420.0 - 394.0) / 2.0,
    "in the middle of the row, none cut"
  );

  index.set(6);
  stage.pass();
  assert_eq!(shown(&mut stage), (2..10).collect::<Vec<_>>(), "about the one shown");
  index.set(11);
  stage.pass();
  assert_eq!(shown(&mut stage), (4..12).collect::<Vec<_>>(), "and to the last");
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
  let mut stage = Stage::first_frame(800.0, 400.0, move |_| {
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
  let mut stage = Stage::first_frame(800.0, 400.0, move |_| {
    let tell = tell.clone();
    Column::new()
      .size(800.0, 400.0)
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
  assert_eq!(stage.bounds("viewer"), (0.0, 0.0, 800.0, 400.0), "it takes the window");
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

#[test]
fn its_keys_are_its_own_only_while_the_focus_is_in_it() {
  let mut stage = in_page(|lightbox| lightbox);
  settled(&mut stage);
  // Nothing in it was pressed: the focus is elsewhere, and a key pressed there is not the lightbox's.
  stage.key("ArrowRight");
  stage.key("+");
  assert_eq!(stage.text("1"), (WHITE, 12.0));
  assert!(stage.paints("100%"));
}

#[test]
fn in_a_page_it_is_no_wider_than_a_room_narrower_than_it() {
  let open = Signal::new(true);
  let mut stage = Stage::first_frame(800.0, 400.0, move |_| {
    Column::new()
      .size(800.0, 400.0)
      .child(
        Row::new().id("room").width(280.0).child(
          Lightbox::new(open.clone(), pictures())
            .inline(true)
            .width(420.0)
            .height(290.0)
            .id("viewer"),
        ),
      )
      .into()
  });
  settled(&mut stage);
  assert_eq!(stage.bounds("viewer"), (0.0, 0.0, 280.0, 290.0));
  for id in ["viewer-close", "viewer-next", "viewer-zoom-in", "viewer-level"] {
    let (x, _, width, _) = stage.bounds(id);
    assert!(x >= 0.0 && x + width <= 280.5, "{id} is in the panel: {width} at {x}");
  }
}

#[test]
fn the_level_gives_way_where_the_top_row_has_no_room_for_it() {
  let open = Signal::new(true);
  let mut stage = Stage::first_frame(800.0, 400.0, move |_| {
    Column::new()
      .size(800.0, 400.0)
      .child(
        Row::new().id("room").width(240.0).child(
          Lightbox::new(open.clone(), pictures())
            .inline(true)
            .width(420.0)
            .height(290.0)
            .on_download(|_| {})
            .id("viewer"),
        ),
      )
      .into()
  });
  settled(&mut stage);
  assert!(stage.tree.get_element_by_id("viewer-level").is_none());
  for id in ["viewer-zoom-out", "viewer-zoom-in", "viewer-download", "viewer-close"] {
    let (x, _, width, _) = stage.bounds(id);
    assert!(x >= 0.0 && x + width <= 240.5, "{id} is in the panel: {width} at {x}");
  }
  // The buttons about it still bring the picture closer.
  stage.click("viewer-zoom-in");
  settled(&mut stage);
  assert!(stage.tree.get_element_by_id("viewer-zoom-out").is_some());
}

/// What a lightbox brings on ahead: the pictures about the one shown, asked of the loader once its own pixels came.
#[cfg(feature = "image")]
mod ahead {
  use std::time::Duration;

  use lurq::{
    images::{ImageKey, decoded_images},
    resources::Source,
  };

  use super::*;

  const RED: &[u8] = include_bytes!("../assets/images/red_64x32.png");

  /// Five pictures that are resources of the app, `<name>-0.png` to `<name>-4.png`, in a folder of this test's
  /// own; the ones in `missing` are not there.
  fn app_with(name: &str, missing: &[usize]) -> (lurq::app::App, Vec<String>) {
    let folder = std::env::temp_dir().join(format!("lurq-lightbox-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let names: Vec<String> = (0..5).map(|at| format!("{name}-{at}.png")).collect();
    for (at, file) in names.iter().enumerate() {
      let _ = std::fs::remove_file(folder.join(file));
      if !missing.contains(&at) {
        std::fs::write(folder.join(file), RED).unwrap();
      }
    }
    let app = lurq::app::App::new();
    app.loader().root(&folder);
    (app, names)
  }

  /// Whether the pixels of the resource `name` were asked for: they are decoded, or on their way.
  fn asked(name: &str) -> bool {
    decoded_images()
      .state(ImageKey::new(Source::resource(name).cache_key()), None)
      .is_some()
  }

  fn lightbox_of(app: lurq::app::App, names: &[String], preload: Option<usize>) -> Stage {
    let names = names.to_vec();
    Stage::first_frame_in(app, 800.0, 400.0, move |_| {
      let mut lightbox = Lightbox::new(Signal::new(true), names.iter().map(|name| Picture::from_resource(name)))
        .inline(true)
        .width(420.0)
        .height(290.0)
        // The strip decodes its small pictures small: what it asks for is not the pictures themselves.
        .index(Signal::new(2))
        .id("viewer");
      if let Some(preload) = preload {
        lightbox = lightbox.preload(preload);
      }
      Column::new().size(800.0, 400.0).child(lightbox).into()
    })
  }

  /// Passes until `done`, a second at most.
  fn until(stage: &mut Stage, done: impl Fn() -> bool) -> bool {
    for _ in 0..100 {
      stage.pass();
      if done() {
        return true;
      }
      std::thread::sleep(Duration::from_millis(10));
    }
    false
  }

  #[test]
  fn the_pictures_beside_the_one_shown_are_asked_for_once_it_came() {
    let (app, names) = app_with("beside", &[]);
    // When is the next test's: nothing is asked for ahead of a picture that did not come. A picture that is a
    // file on this disk comes within the first frame, and the ones beside it with it.
    let mut stage = lightbox_of(app, &names, None);
    assert!(
      until(&mut stage, || asked(&names[1]) && asked(&names[3])),
      "the one before and the one after"
    );
    assert!(asked(&names[2]));
    // One each way, and no further.
    std::thread::sleep(Duration::from_millis(100));
    stage.pass();
    assert!(!asked(&names[0]) && !asked(&names[4]));
  }

  #[test]
  fn nothing_is_asked_for_ahead_of_one_that_did_not_come() {
    let (app, names) = app_with("failed", &[2]);
    let mut stage = lightbox_of(app, &names, None);
    for _ in 0..20 {
      stage.pass();
      std::thread::sleep(Duration::from_millis(10));
    }
    assert!(!asked(&names[1]) && !asked(&names[3]));
  }

  #[test]
  fn told_none_it_asks_for_nothing_ahead_and_told_two_it_asks_for_two_each_way() {
    let (app, names) = app_with("none", &[]);
    let mut stage = lightbox_of(app, &names, Some(0));
    assert!(until(&mut stage, || asked(&names[2])));
    std::thread::sleep(Duration::from_millis(100));
    stage.pass();
    assert!(!asked(&names[1]) && !asked(&names[3]));

    let (app, names) = app_with("two", &[]);
    let mut stage = lightbox_of(app, &names, Some(2));
    assert!(until(&mut stage, || names.iter().all(|name| asked(name))));
  }
}

/// A page with a button that opens a lightbox of the pictures over the window.
fn opened_by_a_button() -> (Stage, Signal<bool>) {
  let open = Signal::new(false);
  let shown = open.clone();
  let mut stage = Stage::first_frame(800.0, 400.0, move |_| {
    let press = shown.clone();
    Column::new()
      .size(800.0, 400.0)
      .padding(8.0)
      .child(
        lurq::ui::Button::new("Open")
          .id("open")
          .on_click(move |_| press.set(true)),
      )
      .child(Lightbox::new(shown.clone(), pictures()).id("viewer"))
      .into()
  });
  stage.pass();
  (stage, open)
}

fn focused_id(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

#[test]
fn opened_by_a_press_the_stage_takes_the_focus_with_no_ring_and_the_keys_work_at_once() {
  let (mut stage, open) = opened_by_a_button();
  stage.click("open");
  stage.pass();
  assert!(open.get());
  assert_eq!(focused_id(&stage).as_deref(), Some("viewer-stage"));
  assert!(!stage.tree.focus_visible(), "a pointer opened it: no ring");

  stage.key("ArrowRight");
  stage.pass();
  assert_eq!(stage.text_quad("2").is_some(), true, "the arrow turned at once");
  // Tab goes on to the buttons: the round ones over the stage first, which follow it.
  stage.key("Tab");
  assert_eq!(focused_id(&stage).as_deref(), Some("viewer-previous"));
  stage.key("Escape");
  stage.pass();
  assert!(!open.get());
}

#[test]
fn opened_by_a_key_the_ring_shows_on_the_stage() {
  let (mut stage, open) = opened_by_a_button();
  stage.key("Tab");
  assert_eq!(focused_id(&stage).as_deref(), Some("open"));
  stage.key("Enter");
  stage.pass();
  assert!(open.get());
  assert_eq!(focused_id(&stage).as_deref(), Some("viewer-stage"));
  assert!(stage.tree.focus_visible(), "the keyboard opened it: the ring shows");
  let (x, y, width, height) = stage.bounds("viewer-stage");
  let white = Color::new(255, 255, 255, 255);
  assert!(
    stage.quads().iter().any(|quad| {
      quad.border.is_some()
        && matches!(quad.content, QuadContent::Rect { .. })
        && (quad.x - x).abs() < 3.0
        && (quad.y - y).abs() < 3.0
        && (quad.width - width).abs() < 5.0
        && (quad.height - height).abs() < 5.0
    }) || stage.has_ring(white),
    "a white ring about the stage"
  );
}

#[test]
fn closer_than_it_fits_no_scroll_bar_is_painted_over_the_picture() {
  let mut stage = in_page(|lightbox| lightbox);
  settled(&mut stage);
  stage.click("viewer-zoom-in");
  stage.click("viewer-zoom-in");
  settled(&mut stage);
  let (x, y, width, height) = stage.bounds("viewer-stage");
  let bars: Vec<_> = stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(quad.content, QuadContent::Rect { color, .. } if color.a() > 0))
    .filter(|quad| {
      quad.x >= x && quad.y >= y && quad.x + quad.width <= x + width + 0.5 && quad.y + quad.height <= y + height + 0.5
    })
    .filter(|quad| (quad.width <= 12.0 && quad.height >= 40.0) || (quad.height <= 12.0 && quad.width >= 40.0))
    .map(|quad| (quad.x, quad.y, quad.width, quad.height))
    .collect();
  assert!(bars.is_empty(), "no bar over the picture: {bars:?}");
}

#[test]
fn opened_again_the_picture_is_fitted() {
  let (mut stage, open) = opened_by_a_button();
  stage.click("open");
  stage.pass();
  stage.key("+");
  stage.pass();
  assert!(stage.paints("150%"));
  stage.key("Escape");
  stage.pass();
  assert!(!open.get());
  open.set(true);
  stage.pass();
  stage.pass();
  assert!(stage.paints("100%"), "fitted, as the first time");
}

/// The phone's viewer, as the board draws it: a page of 360 by 640 with the lightbox in it, at the second of
/// three pictures.
fn on_a_phone(lightbox: impl Fn(Lightbox) -> Lightbox + Send + Sync + 'static) -> (Stage, Signal<usize>) {
  let index = Signal::new(1);
  let shown = index.clone();
  let mut stage = Stage::first_frame(360.0, 640.0, move |_| {
    Column::new()
      .size(360.0, 640.0)
      .child(lightbox(
        Lightbox::new(Signal::new(true), pictures())
          .inline(true)
          .variant(LightboxVariant::Phone)
          .index(shown.clone())
          .id("viewer"),
      ))
      .into()
  });
  settled(&mut stage);
  settled(&mut stage);
  (stage, index)
}

#[test]
fn on_a_phone_it_is_the_boards_viewer() {
  let told = Arc::new(std::sync::Mutex::new(Vec::new()));
  let tell = told.clone();
  let (mut stage, _) = on_a_phone(move |lightbox| {
    let tell = tell.clone();
    lightbox
      .on_share(move |at| tell.lock().unwrap().push(at))
      .on_download(|_| {})
  });
  assert_eq!(stage.bounds("viewer"), (0.0, 0.0, 360.0, 640.0));
  assert_eq!(stage.fill_color("viewer"), Some(Color::new(0, 0, 0, 255)), "black");
  // A top row of 44: Close 4 in, Share and More at the end, the counter in the middle.
  assert_eq!(stage.bounds("viewer-close"), (4.0, 0.0, 44.0, 44.0));
  assert_eq!(stage.bounds("viewer-more"), (360.0 - 4.0 - 44.0, 0.0, 44.0, 44.0));
  assert_eq!(stage.bounds("viewer-share").0, 360.0 - 4.0 - 88.0);
  let counter = stage.text_quad("2").expect("the counter");
  assert!(
    (counter.y + counter.height / 2.0 - 22.0).abs() < 2.0,
    "in the middle of the row's height: {} + {}",
    counter.y,
    counter.height
  );
  assert!(
    (counter.x + 20.0 - 180.0).abs() < 20.0,
    "in the middle of the row: {}",
    counter.x
  );
  // No zoom buttons, round buttons or strip: a pinch, a double tap and a swipe do their work.
  for id in ["viewer-zoom-in", "viewer-previous", "viewer-thumb-0"] {
    assert!(stage.tree.get_element_by_id(id).is_none(), "{id}");
  }
  // The caption under the picture, a title of 15, and a dot for each picture, the one shown white.
  let (color, size) = stage.text("The ridge");
  assert_eq!((color, size), (Color::new(255, 255, 255, 255), 15.0));
  let (x0, y0, width, height) = stage.bounds("viewer-page-0");
  assert_eq!((width, height), (8.0, 8.0));
  assert_eq!(stage.bounds("viewer-page-1").0, x0 + 16.0);
  assert_eq!(stage.bounds("viewer-page-1").1, y0);
  assert_eq!(stage.fill_color("viewer-page-1"), Some(Color::new(255, 255, 255, 255)));
  assert_eq!(stage.fill_color("viewer-page-0"), Some(Color::new(255, 255, 255, 0x59)));
  assert_eq!(y0 + 8.0 + 8.0, 640.0, "8 under the dots");

  stage.click("viewer-share");
  assert_eq!(*told.lock().unwrap(), [1]);
}

#[test]
fn more_holds_download_and_the_apps_items() {
  let told = Arc::new(std::sync::Mutex::new(Vec::new()));
  let tell = told.clone();
  let (mut stage, _) = on_a_phone(move |lightbox| {
    let tell = tell.clone();
    lightbox
      .on_download(move |at| tell.lock().unwrap().push(at))
      .menu_item(lurq::ui::MenuItem::new("Copy link"))
  });
  stage.click("viewer-more");
  stage.pass();
  assert!(stage.paints("Download"));
  assert!(stage.paints("Copy link"));
  let (x, y) = {
    let quad = stage.text_quad("Download").unwrap();
    (quad.x + 4.0, quad.y + quad.height / 2.0)
  };
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, lurq::app::events::MouseButton::Left);
  stage.tree.mouse_up(x, y, lurq::app::events::MouseButton::Left);
  stage.pass();
  assert_eq!(*told.lock().unwrap(), [1]);
}

#[test]
fn on_a_phone_the_keys_turn_and_the_pages_follow() {
  let (mut stage, index) = on_a_phone(|lightbox| lightbox);
  // Tab to Close, in the lightbox: its keys are its own while the focus is in it.
  stage.key("Tab");
  stage.key("ArrowRight");
  settled(&mut stage);
  settled(&mut stage);
  assert_eq!(index.get(), 2);
  assert_eq!(stage.bounds("viewer-pages-slide-2").0, 0.0, "the third page is in view");
}

#[test]
fn a_drag_down_closes_it_and_a_short_one_does_not() {
  use lurq::app::events::TouchPhase;
  let open = Signal::new(true);
  let shown = open.clone();
  let mut stage = Stage::first_frame_in(for_a_finger(), 390.0, 844.0, move |_| {
    Column::new()
      .size(390.0, 844.0)
      .child(Lightbox::new(shown.clone(), pictures()).id("viewer"))
      .into()
  });
  settled(&mut stage);
  settled(&mut stage);
  assert!(
    stage.tree.get_element_by_id("viewer-zoom-in").is_none(),
    "an app for a finger: the phone's form"
  );
  let (x, y) = stage.center("viewer-stage");
  let mut drag = |stage: &mut Stage, by: f32| {
    let route = stage.tree.touch_event(1, TouchPhase::Start, x, y, None);
    eprintln!("PROBE route {route:?}");
    for step in 1..=10 {
      // Slowly: a throw down closes it however far it went.
      std::thread::sleep(std::time::Duration::from_millis(30));
      stage
        .tree
        .touch_event(1, TouchPhase::Move, x, y + by * step as f32 / 10.0, None);
      stage.frame();
    }
    stage.tree.touch_event(1, TouchPhase::End, x, y + by, None);
    settled(stage);
  };
  drag(&mut stage, 40.0);
  assert!(open.get(), "not far enough");
  drag(&mut stage, 220.0);
  assert!(!open.get(), "far enough: it closed");
}

/// An app a finger points in, as an app on a phone starts: pinned, so that the stage's own clicks leave it.
fn for_a_finger() -> lurq::app::App {
  let app = lurq::app::App::new();
  app.set_pointer(lurq::app::Pointer::Coarse);
  app
}

/// Whether the lightbox `viewer` is in the phone's form: the dots under the picture, and no zoom buttons.
fn phones(stage: &mut Stage) -> bool {
  let dots = stage.tree.get_element_by_id("viewer-page-0").is_some();
  let zoom = stage.tree.get_element_by_id("viewer-zoom-in").is_some();
  assert_ne!(dots, zoom, "one form or the other");
  dots
}

#[test]
fn in_an_app_for_a_finger_it_is_the_phones_form_however_wide_the_window_is() {
  for (width, height) in [(1440.0, 900.0), (1000.0, 700.0), (390.0, 844.0), (320.0, 640.0)] {
    let mut stage = Stage::first_frame_in(for_a_finger(), width, height, move |_| {
      Column::new()
        .size(width, height)
        .child(Lightbox::new(Signal::new(true), pictures()).id("viewer"))
        .into()
    });
    settled(&mut stage);
    assert!(phones(&mut stage), "over a window of {width}");
    assert_eq!(stage.bounds("viewer"), (0.0, 0.0, width, height), "the whole screen");
  }
  // In a page too, and told otherwise it is the desktop's.
  for (variant, phone) in [(None, true), (Some(LightboxVariant::Desktop), false)] {
    let mut stage = Stage::first_frame_in(for_a_finger(), 1000.0, 700.0, move |_| {
      let lightbox = Lightbox::new(Signal::new(true), pictures())
        .inline(true)
        .width(420.0)
        .height(290.0)
        .id("viewer");
      Row::new()
        .child(match variant {
          Some(variant) => lightbox.variant(variant),
          None => lightbox,
        })
        .into()
    });
    settled(&mut stage);
    assert_eq!(phones(&mut stage), phone, "{variant:?}");
  }
}

#[test]
fn a_narrow_window_of_a_desktop_has_the_desktops_form_laid_out_to_fit() {
  for (width, height) in [(390.0, 844.0), (320.0, 640.0)] {
    let mut stage = Stage::first_frame(width, height, move |_| {
      Column::new()
        .size(width, height)
        .child(
          Lightbox::new(Signal::new(true), pictures())
            .index(Signal::new(1))
            .on_download(|_| {})
            .id("viewer"),
        )
        .into()
    });
    settled(&mut stage);
    assert!(
      !phones(&mut stage),
      "a window of {width} at the default density: the desktop's"
    );
    assert_eq!(stage.bounds("viewer"), (0.0, 0.0, width, height));
    for id in [
      "viewer-close",
      "viewer-download",
      "viewer-zoom-in",
      "viewer-zoom-out",
      "viewer-previous",
      "viewer-next",
      "viewer-thumb-0",
      "viewer-thumb-2",
    ] {
      let (x, y, w, h) = stage.bounds(id);
      assert!(
        x >= 0.0 && x + w <= width + 0.5 && y >= 0.0 && y + h <= height + 0.5,
        "{id} is in a window of {width}: {w} at {x}"
      );
    }
    // Told, it is the phone's in a narrow window as in a wide one.
    let mut stage = Stage::first_frame(width, height, move |_| {
      Column::new()
        .size(width, height)
        .child(
          Lightbox::new(Signal::new(true), pictures())
            .variant(LightboxVariant::Phone)
            .id("viewer"),
        )
        .into()
    });
    settled(&mut stage);
    assert!(phones(&mut stage));
  }
}

#[test]
fn the_density_is_how_large_things_are_and_not_which_form() {
  use lurq::app::{Pointer, theme::Density};
  for (density, pointer, phone) in [
    (Density::Compact, Pointer::Coarse, true),
    (Density::Touch, Pointer::Fine, false),
  ] {
    let app = lurq::app::App::new();
    app.set_pointer(pointer);
    app.theme().set_density(density);
    let mut stage = Stage::first_frame_in(app, 1000.0, 700.0, move |_| {
      Column::new()
        .size(1000.0, 700.0)
        .child(Lightbox::new(Signal::new(true), pictures()).id("viewer"))
        .into()
    });
    settled(&mut stage);
    assert_eq!(phones(&mut stage), phone, "{density:?} with a {pointer:?} pointer");
  }
}

#[test]
fn an_open_lightbox_keeps_its_form_under_the_finger_that_lands_on_it_until_it_closes() {
  use lurq::app::{Pointer, events::TouchPhase};
  let open = Signal::new(false);
  let shown = open.clone();
  let mut stage = Stage::first_frame(800.0, 600.0, move |_| {
    Column::new()
      .size(800.0, 600.0)
      .child(Lightbox::new(shown.clone(), pictures()).id("viewer"))
      .into()
  });
  stage.tree.set_pointer(Pointer::Fine);
  open.set(true);
  settled(&mut stage);
  assert!(!phones(&mut stage), "opened under a mouse");

  // A finger lands on it: the pointer is coarse, and the lightbox stays as it opened.
  let (x, y) = stage.center("viewer-stage");
  stage.tree.touch_event(1, TouchPhase::Start, x, y, None);
  stage.tree.touch_event(1, TouchPhase::End, x, y, None);
  settled(&mut stage);
  assert_eq!(stage.tree.pointer(), Pointer::Coarse);
  assert!(!phones(&mut stage), "it does not change under the finger");

  // Closed and opened again, it is the phone's.
  open.set(false);
  settled(&mut stage);
  open.set(true);
  settled(&mut stage);
  assert!(phones(&mut stage), "opened under a finger");

  // The app or a test setting the pointer reaches it at once.
  stage.tree.set_pointer(Pointer::Fine);
  settled(&mut stage);
  assert!(!phones(&mut stage), "told");
}

#[test]
fn a_window_as_wide_as_a_desktops_has_the_desktops_form() {
  let mut stage = Stage::first_frame(1000.0, 700.0, move |_| {
    Column::new()
      .size(1000.0, 700.0)
      .child(Lightbox::new(Signal::new(true), pictures()).id("viewer"))
      .into()
  });
  settled(&mut stage);
  assert!(stage.tree.get_element_by_id("viewer-zoom-in").is_some());
  assert!(stage.tree.get_element_by_id("viewer-page-0").is_none());
}
