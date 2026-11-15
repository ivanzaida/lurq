//! A picture: its box as a browser's `img` sizes it, the pixels placed by the fit and the anchor, what stands in
//! for them where there are none and when they did not load, the bar over it, what a small one keeps, the form
//! that follows the width where it is not said, a press and a choice.

#![cfg(feature = "raster")]

use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::theme::{PaletteColor, ThemePalette},
  components::{Column, Row},
  images::ImageData,
  layout::{
    StackAlignment,
    quad::{Quad, QuadContent},
  },
  node::{Element, color::Color, dimension::Dimension},
  ui::{Length, Picture, PictureFit, PictureShape, PictureSize, icons},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// The character the theme's icon font draws `name` with.
fn glyph(name: &str) -> String {
  icons::ALL
    .iter()
    .find(|(held, _)| *held == name)
    .map(|(_, glyph)| glyph.to_string())
    .unwrap_or_else(|| panic!("no icon {name}"))
}

fn pixels(width: u32, height: u32) -> ImageData {
  ImageData::from_rgba(vec![200; (width * height * 4) as usize], width, height)
}

/// A scene with `picture` at the start of a row, so that the picture is as large as it makes itself.
fn at_start(picture: impl Fn() -> Picture + Send + Sync + 'static) -> Stage {
  Stage::new(move |_| Row::new().padding(8.0).child(picture()).into())
}

/// Where the quad that draws the picture's pixels lies, `(x, y, width, height)`, and the part of them it draws.
fn image_quad(stage: &Stage) -> Option<((f32, f32, f32, f32), [f32; 2], [f32; 2])> {
  stage.quads().into_iter().find_map(|quad| match quad.content {
    QuadContent::Image { uv_min, uv_max, .. } => Some(((quad.x, quad.y, quad.width, quad.height), uv_min, uv_max)),
    _ => None,
  })
}

fn corner(quad: &Quad) -> f32 {
  quad.border_radius.map_or(0.0, |radius| radius.top_left)
}

#[test]
fn a_picture_is_its_pixels_in_a_rounded_muted_box() {
  let mut stage = at_start(|| Picture::new(pixels(320, 180)).size_px(320.0, 180.0).id("lake"));
  assert_eq!(stage.bounds("lake"), (8.0, 8.0, 320.0, 180.0));
  let fill = stage.fill("lake").expect("the box is filled");
  match fill.content {
    QuadContent::Rect { color, .. } => assert_eq!(color, light(PaletteColor::Muted)),
    _ => panic!("a filled rectangle"),
  }
  assert_eq!(corner(&fill), 8.0, "the corners of a surface");
  let (image, uv_min, uv_max) = image_quad(&stage).expect("the pixels are drawn");
  assert_eq!(image, (8.0, 8.0, 320.0, 180.0));
  assert_eq!((uv_min, uv_max), ([0.0, 0.0], [1.0, 1.0]));
}

#[test]
fn its_sides_are_an_imgs() {
  // Neither said: the pixels' own size.
  let mut own = at_start(|| Picture::new(pixels(80, 40)).id("lake"));
  let (_, _, width, height) = own.bounds("lake");
  assert_eq!((width, height), (80.0, 40.0));
  // One said: the other follows the pixels' ratio.
  let mut by_width = at_start(|| Picture::new(pixels(200, 100)).width(120.0).id("lake"));
  let (_, _, width, height) = by_width.bounds("lake");
  assert_eq!((width, height), (120.0, 60.0));
  let mut by_height = at_start(|| Picture::new(pixels(200, 100)).height(30.0).id("lake"));
  let (_, _, width, height) = by_height.bounds("lake");
  assert_eq!((width, height), (60.0, 30.0));
  // A ratio in place of the pixels'.
  let mut by_ratio = at_start(|| Picture::new(pixels(200, 100)).width(120.0).aspect_ratio(1.0).id("lake"));
  let (_, _, width, height) = by_ratio.bounds("lake");
  assert_eq!((width, height), (120.0, 120.0));
}

#[test]
fn the_pixels_cover_the_box_unless_told() {
  // 200 by 100 in a square of 100: the middle half across is kept.
  let cover = at_start(|| Picture::new(pixels(200, 100)).size_px(100.0, 100.0));
  let (_, uv_min, uv_max) = image_quad(&cover).unwrap();
  assert_eq!((uv_min, uv_max), ([0.25, 0.0], [0.75, 1.0]));

  let anchored = at_start(|| {
    Picture::new(pixels(200, 100))
      .size_px(100.0, 100.0)
      .anchor(StackAlignment::CenterEnd)
  });
  let (_, uv_min, uv_max) = image_quad(&anchored).unwrap();
  assert_eq!((uv_min, uv_max), ([0.5, 0.0], [1.0, 1.0]), "the end is kept");

  let contain = at_start(|| {
    Picture::new(pixels(200, 100))
      .size_px(100.0, 100.0)
      .fit(PictureFit::Contain)
      .anchor(StackAlignment::TopCenter)
  });
  let (quad, uv_min, uv_max) = image_quad(&contain).unwrap();
  assert_eq!(quad, (8.0, 8.0, 100.0, 50.0));
  assert_eq!((uv_min, uv_max), ([0.0, 0.0], [1.0, 1.0]));

  let fill = at_start(|| {
    Picture::new(pixels(200, 100))
      .size_px(100.0, 100.0)
      .fit(PictureFit::Fill)
  });
  let (quad, uv_min, uv_max) = image_quad(&fill).unwrap();
  assert_eq!((quad.2, quad.3), (100.0, 100.0));
  assert_eq!((uv_min, uv_max), ([0.0, 0.0], [1.0, 1.0]));

  let own = at_start(|| Picture::new(pixels(40, 20)).size_px(100.0, 100.0).fit(PictureFit::None));
  let (quad, ..) = image_quad(&own).unwrap();
  assert_eq!(quad, (38.0, 48.0, 40.0, 20.0));
}

#[test]
fn a_circle_and_a_bare_one() {
  let mut circle = at_start(|| {
    Picture::new(pixels(64, 64))
      .size_px(64.0, 64.0)
      .shape(PictureShape::Circle)
      .id("face")
  });
  assert_eq!(corner(&circle.fill("face").unwrap()), 999.0);
  let mut bare = at_start(|| Picture::new(pixels(64, 64)).size_px(64.0, 64.0).bare(true).id("plain"));
  assert!(bare.fill("plain").is_none(), "a bare picture has no fill of its own");
  assert!(bare.border("plain").is_none(), "and no line around it");
}

#[test]
fn a_picture_with_no_source_shows_a_glyph_and_what_it_would_be_called() {
  let mut stage = at_start(|| {
    Picture::empty()
      .label("The lake at dusk")
      .size_px(320.0, 180.0)
      .id("lake")
  });
  assert_eq!(stage.bounds("lake"), (8.0, 8.0, 320.0, 180.0));
  assert!(stage.paints(&glyph(icons::IMAGE)));
  let (color, size) = stage.text("The lake at dusk");
  assert_eq!(color, light(PaletteColor::MutedForeground));
  assert_eq!(size, 13.0);
  assert!(image_quad(&stage).is_none());
}

#[test]
fn a_thumbnail_with_no_source_is_the_glyph_alone() {
  let stage = at_start(|| Picture::empty().label("The lake at dusk").size_px(96.0, 96.0));
  assert!(stage.paints(&glyph(icons::IMAGE)));
  assert!(!stage.paints("The lake at dusk"), "a thumbnail has no room for words");
}

#[test]
fn a_box_with_a_width_and_nothing_else_is_wide() {
  let mut stage = at_start(|| Picture::empty().width(320.0).id("lake"));
  let (_, _, width, height) = stage.bounds("lake");
  assert_eq!(
    (width, height),
    (320.0, 180.0),
    "16 by 9 where nothing tells the height"
  );
}

fn with_bar() -> Picture {
  Picture::new(pixels(640, 360))
    .title("Lake at dusk")
    .description("Ivan Zaida")
    .force_bar_shown(Some(true))
}

#[test]
fn the_bar_says_what_the_picture_is() {
  let stage = at_start(|| with_bar().size_px(320.0, 180.0));
  let (color, size) = stage.text("Lake at dusk");
  assert_eq!((color, size), (Color::new(255, 255, 255, 255), 14.0));
  let (color, size) = stage.text("Ivan Zaida");
  assert_eq!((color, size), (Color::new(255, 255, 255, 0x99), 13.0));
  let title = stage.text_quad("Lake at dusk").unwrap();
  assert_eq!(
    title.x,
    8.0 + 12.0 + 4.0,
    "12 from the side, and 4 more beside the buttons' row"
  );
}

#[test]
fn a_compact_picture_keeps_the_title_alone_and_a_thumbnail_nothing() {
  let compact = at_start(|| with_bar().size_px(200.0, 113.0));
  let (_, size) = compact.text("Lake at dusk");
  assert_eq!(size, 13.0);
  assert!(!compact.paints("Ivan Zaida"));
  let thumbnail = at_start(|| with_bar().size_px(96.0, 96.0));
  assert!(!thumbnail.paints("Lake at dusk"));
  let said = at_start(|| with_bar().size_px(320.0, 180.0).size(PictureSize::Sm));
  assert!(
    !said.paints("Lake at dusk"),
    "the form that is said, whatever the width"
  );
}

#[test]
fn the_form_follows_the_width_where_it_is_not_said() {
  // A picture that fills a column, with a ratio: the column's width decides what it keeps.
  let in_column = |width: f32| {
    Stage::new(move |_| {
      Column::new()
        .width(width)
        .child(with_bar().width(Length::Fill).aspect_ratio(16.0 / 9.0).id("lake"))
        .into()
    })
  };
  let mut wide = in_column(320.0);
  assert_eq!(wide.bounds("lake"), (0.0, 0.0, 320.0, 180.0));
  assert!(wide.paints("Lake at dusk") && wide.paints("Ivan Zaida"));
  let compact = in_column(200.0);
  assert!(compact.paints("Lake at dusk"));
  assert!(!compact.paints("Ivan Zaida"));
  let thumbnail = in_column(96.0);
  assert!(!thumbnail.paints("Lake at dusk"));
}

#[test]
fn the_bars_buttons_are_pressed_and_the_picture_is_not() {
  let (downloads, opens) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
  let (download, open) = (downloads.clone(), opens.clone());
  let mut stage = at_start(move || {
    let (download, open) = (download.clone(), open.clone());
    with_bar()
      .size_px(320.0, 180.0)
      .id("lake")
      .on_download(move || {
        download.fetch_add(1, Ordering::SeqCst);
      })
      .on_click(move |_| {
        open.fetch_add(1, Ordering::SeqCst);
      })
  });
  let (x, y, width, height) = stage.bounds("lake-download");
  assert_eq!((width, height), (36.0, 36.0));
  assert_eq!(
    (x + width, y + height),
    (8.0 + 320.0 - 12.0, 8.0 + 180.0 - 8.0),
    "12 from the end and 8 from the bottom"
  );
  stage.click("lake-download");
  assert_eq!((downloads.load(Ordering::SeqCst), opens.load(Ordering::SeqCst)), (1, 0));
  stage.click("lake");
  assert_eq!((downloads.load(Ordering::SeqCst), opens.load(Ordering::SeqCst)), (1, 1));
}

#[test]
fn a_picture_that_can_be_chosen_tells_what_it_is_to_be() {
  let told = Arc::new(AtomicUsize::new(0));
  let teller = told.clone();
  let mut stage = at_start(move || {
    let teller = teller.clone();
    Picture::new(pixels(160, 100))
      .size_px(160.0, 100.0)
      .selectable(true)
      .selected(true)
      .id("lake")
      .on_change(move |chosen| {
        teller.store(if chosen { 2 } else { 1 }, Ordering::SeqCst);
      })
  });
  assert!(stage.paints(&glyph(icons::CHECK)), "a chosen picture has its check");
  stage.click("lake");
  assert_eq!(
    told.load(Ordering::SeqCst),
    1,
    "a press of a chosen one is to make it not chosen"
  );
}

#[test]
fn a_disabled_picture_takes_nothing() {
  let presses = Arc::new(AtomicUsize::new(0));
  let press = presses.clone();
  let mut stage = at_start(move || {
    let press = press.clone();
    Picture::new(pixels(160, 100))
      .size_px(160.0, 100.0)
      .disabled(true)
      .id("lake")
      .on_click(move |_| {
        press.fetch_add(1, Ordering::SeqCst);
      })
  });
  stage.click("lake");
  assert_eq!(presses.load(Ordering::SeqCst), 0);
}

#[test]
fn a_caption_makes_it_a_figure() {
  let mut stage = at_start(|| {
    Picture::new(pixels(212, 141))
      .size_px(212.0, 141.0)
      .caption("Figure 1. The lake at dusk.")
      .id("lake")
  });
  let (_, y, _, height) = stage.bounds("lake");
  let caption = stage
    .text_quad("Figure 1. The lake at dusk.")
    .expect("the caption is drawn");
  assert!(
    caption.y >= y + height + 8.0,
    "8 under the picture: {} under {}",
    caption.y,
    y + height
  );
  let (color, size) = stage.text("Figure 1. The lake at dusk.");
  assert_eq!((color, size), (light(PaletteColor::MutedForeground), 13.0));
}

#[cfg(feature = "image")]
mod decoded {
  use std::time::Duration;

  use lurq::images::wait_for_decoded_images;

  use super::*;

  const RED: &[u8] = include_bytes!("../assets/images/red_64x32.png");

  fn settled(stage: &mut Stage) {
    assert!(wait_for_decoded_images(Duration::from_secs(10)), "the decodes end");
    stage.pass();
  }

  #[test]
  fn a_decoded_picture_shows_its_pixels_once_they_are_there() {
    let mut stage = at_start(|| Picture::from_bytes(RED).cache_key(0x91C7_0001).width(128.0).id("red"));
    settled(&mut stage);
    let (_, _, width, height) = stage.bounds("red");
    assert_eq!((width, height), (128.0, 64.0), "the height follows the pixels' ratio");
    assert!(image_quad(&stage).is_some());
  }

  #[test]
  fn a_picture_that_did_not_load_says_so_and_asks_again() {
    let retries = Arc::new(AtomicUsize::new(0));
    let retry = retries.clone();
    let mut stage = at_start(move || {
      let retry = retry.clone();
      Picture::from_bytes(&b"not a picture"[..])
        .cache_key(0x91C7_0002)
        .label("The lake at dusk")
        .size_px(320.0, 180.0)
        .id("lake")
        .on_retry(move || {
          retry.fetch_add(1, Ordering::SeqCst);
        })
    });
    settled(&mut stage);
    assert!(stage.paints(&glyph(icons::ALERT_CIRCLE)));
    let (color, size) = stage.text("The lake at dusk");
    assert_eq!((color, size), (light(PaletteColor::Foreground), 14.0));
    assert!(stage.paints("The picture didn't load."));
    assert!(stage.paints("Try again"));
    let (x, y, width, height) = stage.bounds("lake-retry");
    assert_eq!(
      height, 32.0,
      "a button of 32, found by its id though no render built it"
    );
    assert!(
      (x + width / 2.0 - 168.0).abs() < 0.01,
      "in the middle of the box across"
    );
    assert!(y > 8.0 + 90.0, "under the words");

    stage.click("lake-retry");
    assert_eq!(retries.load(Ordering::SeqCst), 1);
    settled(&mut stage);
    assert!(stage.paints("Try again"), "it failed again, and says so again");
    stage.click("lake-retry");
    assert_eq!(
      retries.load(Ordering::SeqCst),
      2,
      "the button asks as often as it is pressed"
    );
  }

  #[test]
  fn the_app_is_told_once_that_a_picture_came() {
    let came = Arc::new(AtomicUsize::new(0));
    let tell = came.clone();
    let mut stage = at_start(move || {
      let tell = tell.clone();
      Picture::from_bytes(RED)
        .cache_key(0x91C7_0010)
        .width(128.0)
        .on_load(move || {
          tell.fetch_add(1, Ordering::SeqCst);
        })
    });
    settled(&mut stage);
    stage.pass();
    stage.pass();
    assert!(image_quad(&stage).is_some());
    assert_eq!(came.load(Ordering::SeqCst), 1);
  }

  #[test]
  fn the_app_is_told_why_a_picture_did_not_come_and_again_when_it_is_asked_for_again() {
    let reasons = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let tell = reasons.clone();
    let mut stage = at_start(move || {
      let tell = tell.clone();
      Picture::from_bytes(&b"no picture at all"[..])
        .cache_key(0x91C7_0011)
        .size_px(320.0, 180.0)
        .id("broken")
        .on_error(move |reason| tell.lock().unwrap().push(reason.to_owned()))
    });
    settled(&mut stage);
    stage.pass();
    assert_eq!(reasons.lock().unwrap().len(), 1);
    assert!(!reasons.lock().unwrap()[0].is_empty(), "it says why");

    stage.click("broken-retry");
    settled(&mut stage);
    stage.pass();
    assert_eq!(reasons.lock().unwrap().len(), 2);
  }

  #[test]
  fn a_picture_keeps_its_place_among_others_by_its_key() {
    let order = lurq::core::Signal::new(false);
    let turned = order.clone();
    let mut stage = Stage::new(move |_| {
      let red = Picture::from_bytes(RED)
        .cache_key(0x91C7_0012)
        .size_px(64.0, 32.0)
        .key("red")
        .id("red");
      let none = Picture::empty().size_px(64.0, 32.0).key("none").id("none");
      let row = Row::new().padding(8.0);
      match turned.get() {
        false => row.child(red).child(none).into(),
        true => row.child(none).child(red).into(),
      }
    });
    settled(&mut stage);
    assert_eq!(stage.bounds("red").0, 8.0);
    order.set(true);
    stage.pass();
    assert_eq!(
      stage.bounds("red").0,
      8.0 + 64.0,
      "it moved, and it is the picture it was"
    );
    assert!(image_quad(&stage).is_some());
  }

  /// Passes until `done` holds, while the app's loader and the decoder work on their threads.
  fn until(stage: &mut Stage, what: &str, done: impl Fn(&Stage) -> bool) {
    for _ in 0..500 {
      stage.pass();
      if done(stage) {
        return;
      }
      std::thread::sleep(Duration::from_millis(10));
    }
    panic!("{what} did not come");
  }

  /// A folder of this test's own.
  fn folder(name: &str) -> std::path::PathBuf {
    let folder = std::env::temp_dir().join(format!("lurq-picture-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    folder
  }

  #[test]
  fn a_resource_under_the_apps_root_is_loaded_and_decoded() {
    let folder = folder("root");
    std::fs::write(folder.join("red.png"), RED).unwrap();
    let app = lurq::app::App::new();
    app.loader().root(&folder);
    let mut stage = Stage::in_app(app, |_| {
      Row::new()
        .padding(8.0)
        .child(Picture::from_resource("red.png").width(128.0).id("red"))
        .into()
    });
    until(&mut stage, "the picture", |stage| image_quad(stage).is_some());
    let (_, _, width, height) = stage.bounds("red");
    assert_eq!((width, height), (128.0, 64.0), "the height follows the pixels' ratio");
  }

  #[test]
  fn a_resource_that_is_not_there_says_so_and_comes_when_it_is_asked_for_again() {
    let folder = folder("late");
    let _ = std::fs::remove_file(folder.join("late.png"));
    let app = lurq::app::App::new();
    app.loader().root(&folder);
    let mut stage = Stage::in_app(app, |_| {
      Row::new()
        .padding(8.0)
        .child(
          Picture::from_resource("late.png")
            .label("The lake at dusk")
            .size_px(320.0, 180.0)
            .id("late"),
        )
        .into()
    });
    until(&mut stage, "the error", |stage| stage.paints("Try again"));
    assert!(stage.paints("The lake at dusk"));
    assert!(stage.paints("The picture didn't load."));
    assert!(image_quad(&stage).is_none());

    // Asked again with nothing changed, it fails again and says so again.
    stage.click("late-retry");
    until(&mut stage, "the error again", |stage| stage.paints("Try again"));

    std::fs::write(folder.join("late.png"), RED).unwrap();
    stage.click("late-retry");
    until(&mut stage, "the picture", |stage| image_quad(stage).is_some());
    assert!(!stage.paints("Try again"));
  }

  #[test]
  fn a_resource_that_is_no_picture_is_read_again_when_it_is_asked_for_again() {
    let folder = folder("mended");
    std::fs::write(folder.join("mended.png"), b"not a picture").unwrap();
    let app = lurq::app::App::new();
    app.loader().root(&folder);
    let mut stage = Stage::in_app(app, |_| {
      Row::new()
        .padding(8.0)
        .child(Picture::from_resource("mended.png").size_px(320.0, 180.0).id("mended"))
        .into()
    });
    until(&mut stage, "the error", |stage| stage.paints("Try again"));

    // It was read, and what was read is no picture: asked again, it is read again and not decoded again
    // from the bytes that failed.
    std::fs::write(folder.join("mended.png"), RED).unwrap();
    stage.click("mended-retry");
    until(&mut stage, "the picture", |stage| image_quad(stage).is_some());
    assert!(!stage.paints("Try again"));
  }

  /// A server on this machine that answers every request with `body` as a PNG, until the test ends.
  #[cfg(feature = "http")]
  fn serve(body: &'static [u8]) -> String {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}/red.png", listener.local_addr().unwrap());
    std::thread::spawn(move || {
      for stream in listener.incoming() {
        let Ok(mut stream) = stream else { return };
        let mut request = Vec::new();
        let mut chunk = [0u8; 1024];
        while !request.windows(4).any(|window| window == b"\r\n\r\n") {
          match stream.read(&mut chunk) {
            Ok(read) if read > 0 => request.extend_from_slice(&chunk[..read]),
            _ => break,
          }
        }
        let head = format!(
          "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
          body.len()
        );
        let _ = stream.write_all(head.as_bytes()).and_then(|()| stream.write_all(body));
      }
    });
    address
  }

  #[cfg(feature = "http")]
  #[test]
  fn a_picture_at_an_address_is_loaded_and_decoded() {
    let address = serve(RED);
    let mut stage = Stage::new(move |_| {
      Row::new()
        .padding(8.0)
        .child(Picture::from_url(&address).width(128.0).id("red"))
        .into()
    });
    until(&mut stage, "the picture", |stage| image_quad(stage).is_some());
    let (_, _, width, height) = stage.bounds("red");
    assert_eq!((width, height), (128.0, 64.0));
  }

  #[cfg(not(feature = "http"))]
  #[test]
  fn a_build_without_http_shows_an_address_as_a_picture_that_did_not_load() {
    let mut stage = Stage::new(|_| {
      Row::new()
        .padding(8.0)
        .child(
          Picture::from_url("https://example.com/lake.jpg")
            .label("The lake at dusk")
            .size_px(320.0, 180.0),
        )
        .into()
    });
    until(&mut stage, "the error", |stage| stage.paints("Try again"));
    assert!(stage.paints("The lake at dusk"));
  }

  #[test]
  fn a_thumbnail_that_did_not_load_is_a_glyph() {
    let mut stage = at_start(|| {
      Picture::from_bytes(&b"not a picture either"[..])
        .cache_key(0x91C7_0003)
        .label("The lake at dusk")
        .size_px(96.0, 96.0)
    });
    settled(&mut stage);
    assert!(stage.paints(&glyph(icons::ALERT_CIRCLE)));
    assert!(!stage.paints("Try again"));
    assert!(!stage.paints("The lake at dusk"));
  }
}

/// The scene is an element: a picture is one.
#[test]
fn a_picture_is_an_element() {
  let _: Element = Picture::empty().into();
  let _ = Dimension::Px(1.0);
}
