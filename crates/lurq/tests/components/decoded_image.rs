//! Images decoded at runtime: off the UI thread, downscaled to a cap, cached
//! by content, with a placeholder while loading and an error view for a
//! broken file.

use std::{
  sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
  },
  time::Duration,
};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx},
  components::{Column, Image, Rect, Text, VirtualizedList},
  images::{
    DecodedImages, ImageDecodeError, ImageKey, ImageSizeCap, ImageSource, decode_image, decoded_images,
    wait_for_decoded_images,
  },
  node::{BackgroundSize, Element, color::Color},
};

use crate::support::{render_pass, render_pass_with_app, run_pass};

const RED_64X32: &[u8] = include_bytes!("../assets/images/red_64x32.png");
const BLUE_8X8: &[u8] = include_bytes!("../assets/images/blue_8x8.png");
const PLACEHOLDER: &str = "#777777";
const WAIT: Duration = Duration::from_secs(10);

/// A key no other test uses: the cache is process-wide.
fn unique_key(bytes: &[u8], salt: &str) -> ImageKey {
  let mut salted = bytes.to_vec();
  salted.extend_from_slice(salt.as_bytes());
  ImageKey::of_bytes(&salted)
}

#[test]
fn decode_image_reads_the_pixels_and_downscales_to_the_cap() {
  let source = ImageSource::Bytes(Arc::from(RED_64X32));

  let full = decode_image(&source, None).expect("a PNG decodes");
  assert_eq!((full.width(), full.height()), (64, 32));
  assert_eq!(&full.data()[..4], &[255, 0, 0, 255]);

  let capped = decode_image(&source, Some(ImageSizeCap::new(16, 16))).expect("decodes capped");
  assert_eq!(
    (capped.width(), capped.height()),
    (16, 8),
    "fits the cap, keeps the aspect ratio"
  );
  assert_eq!(capped.data().len(), 16 * 8 * 4);
  assert_eq!(&capped.data()[..4], &[255, 0, 0, 255]);

  let small = decode_image(&source, Some(ImageSizeCap::new(640, 640))).expect("decodes under the cap");
  assert_eq!((small.width(), small.height()), (64, 32), "never upscaled");
}

#[test]
fn a_broken_file_is_an_error_not_a_panic() {
  let error = decode_image(&ImageSource::Bytes(Arc::from(&b"not an image"[..])), None)
    .err()
    .expect("not an image");
  assert!(!error.message().is_empty());
  assert_eq!(error.to_string(), error.message());

  let missing = decode_image(&ImageSource::File("Z:/nowhere/missing.png".into()), None)
    .err()
    .expect("missing file");
  assert!(missing.message().contains("missing.png"));
}

#[test]
fn an_image_from_bytes_shows_its_placeholder_then_the_image() {
  let key = unique_key(RED_64X32, "placeholder");
  let mut runtime = Tree::new();
  runtime.set_root(
    Image::from_bytes(RED_64X32)
      .cache_key(key)
      .placeholder(Rect::new(10.0, 10.0).background(PLACEHOLDER))
      .size(40.0, 40.0),
  );

  let before = render_pass(&mut runtime);
  assert!(
    before
      .rects
      .iter()
      .any(|rect| rect.color == Color::from_hex(PLACEHOLDER)),
    "the placeholder shows while the decode runs: {:?}",
    before.rects
  );
  assert!(before.images.is_empty());

  assert!(wait_for_decoded_images(WAIT), "the decode ends");
  let after = render_pass(&mut runtime);
  assert_eq!(after.images.len(), 1, "the image shows once decoded");
  assert_eq!((after.images[0].image_width, after.images[0].image_height), (64, 32));
  assert!(
    !after
      .rects
      .iter()
      .any(|rect| rect.color == Color::from_hex(PLACEHOLDER)),
    "the placeholder is gone"
  );
  assert!(
    decoded_images()
      .state(key, None)
      .is_some_and(|state| state.ready().is_some())
  );
}

#[test]
fn the_same_key_decodes_once_and_the_second_tree_shows_it_at_once() {
  let key = unique_key(RED_64X32, "cache");
  let cap = ImageSizeCap::new(32, 32);
  let build = || {
    Image::from_bytes(RED_64X32)
      .cache_key(key)
      .downscale_to(cap.width, cap.height)
      .size(32.0, 16.0)
  };
  let mut first = Tree::new();
  first.set_root(build());
  render_pass(&mut first);
  assert!(wait_for_decoded_images(WAIT));
  let completed = decoded_images().completed();

  let mut second = Tree::new();
  second.set_root(build());
  let snapshot = render_pass(&mut second);

  assert_eq!(snapshot.images.len(), 1, "a cache hit resolves in the first pass");
  assert_eq!(
    (snapshot.images[0].image_width, snapshot.images[0].image_height),
    (32, 16),
    "the capped decode"
  );
  assert_eq!(decoded_images().completed(), completed, "nothing decoded again");
  assert!(
    decoded_images()
      .state(key, Some(cap))
      .is_some_and(|state| state.ready().is_some())
  );
  assert!(
    decoded_images().state(key, None).is_none(),
    "the full-size decode was never asked for"
  );
}

#[test]
fn a_broken_image_shows_its_error_view() {
  let seen = Arc::new(std::sync::Mutex::new(None));
  let mut runtime = Tree::new();
  let log = seen.clone();
  runtime.set_root(
    Image::from_bytes(&b"broken bytes"[..])
      .cache_key(unique_key(b"broken bytes", "error"))
      .placeholder(Rect::new(10.0, 10.0).background(PLACEHOLDER))
      .error_view(move |error: &ImageDecodeError| {
        *log.lock().unwrap() = Some(error.message().to_owned());
        Text::new("broken").id("error-text")
      })
      .size(40.0, 40.0),
  );
  run_pass(&mut runtime);
  assert!(runtime.get_element_by_id("error-text").is_none(), "not failed yet");

  assert!(wait_for_decoded_images(WAIT));
  let snapshot = render_pass(&mut runtime);

  assert!(
    runtime.get_element_by_id("error-text").is_some(),
    "the error view replaced the placeholder"
  );
  assert!(seen.lock().unwrap().as_ref().is_some_and(|message| !message.is_empty()));
  assert!(snapshot.images.is_empty());
  assert!(
    !snapshot
      .rects
      .iter()
      .any(|rect| rect.color == Color::from_hex(PLACEHOLDER))
  );
}

#[test]
fn fit_places_the_pixels_in_the_box() {
  let key = unique_key(RED_64X32, "fit");
  decoded_images().insert(
    key,
    None,
    decode_image(&ImageSource::Bytes(Arc::from(RED_64X32)), None).unwrap(),
  );
  let build = |fit: BackgroundSize| Image::from_bytes(RED_64X32).cache_key(key).fit(fit).size(40.0, 40.0);

  let mut contain = Tree::new();
  contain.set_root(build(BackgroundSize::Contain));
  let contain = render_pass(&mut contain).images[0];
  assert_eq!(
    (contain.width, contain.height),
    (40.0, 20.0),
    "contain fits the 2:1 image in the box"
  );
  assert_eq!(contain.y, 10.0, "centered");
  assert_eq!((contain.uv_min, contain.uv_max), ([0.0, 0.0], [1.0, 1.0]));

  let mut cover = Tree::new();
  cover.set_root(build(BackgroundSize::Cover));
  let cover = render_pass(&mut cover).images[0];
  assert_eq!((cover.width, cover.height), (40.0, 40.0), "cover fills the box");
  assert!(
    cover.uv_min[0] > 0.0 && cover.uv_max[0] < 1.0,
    "and crops the sides: {cover:?}"
  );

  let mut stretch = Tree::new();
  stretch.set_root(build(BackgroundSize::Stretch));
  let stretch = render_pass(&mut stretch).images[0];
  assert_eq!((stretch.width, stretch.height), (40.0, 40.0));
  assert_eq!((stretch.uv_min, stretch.uv_max), ([0.0, 0.0], [1.0, 1.0]));
}

#[test]
fn an_image_from_a_file_decodes_and_is_keyed_by_the_path() {
  let dir = std::env::temp_dir().join(format!("lurq-decoded-image-{}", std::process::id()));
  std::fs::create_dir_all(&dir).unwrap();
  let path = dir.join("blue.png");
  std::fs::write(&path, BLUE_8X8).unwrap();
  let key = ImageKey::of_path(&path);
  let mut runtime = Tree::new();
  runtime.set_root(Image::from_file(&path).size(8.0, 8.0));
  render_pass(&mut runtime);
  assert!(wait_for_decoded_images(WAIT));

  let snapshot = render_pass(&mut runtime);

  assert_eq!(snapshot.images.len(), 1);
  assert_eq!(
    (snapshot.images[0].image_width, snapshot.images[0].image_height),
    (8, 8)
  );
  assert!(
    decoded_images()
      .state(key, None)
      .is_some_and(|state| state.ready().is_some())
  );
  std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_budget_drops_the_least_recently_used_ready_images() {
  // A cache of its own: the process-wide one is shared with every test.
  let cache = DecodedImages::new();
  let red = decode_image(&ImageSource::Bytes(Arc::from(RED_64X32)), None).unwrap();
  let (old, new) = (ImageKey::new(1), ImageKey::new(2));
  cache.insert(old, None, red.clone());
  cache.insert(new, None, red);
  assert_eq!(cache.used_bytes(), 2 * 64 * 32 * 4);
  // Asking for the old one makes the new one the least recently used.
  assert!(cache.load(old, None, || unreachable!("cached"), None).ready().is_some());

  cache.set_budget(64 * 32 * 4 + 1);

  assert!(cache.state(old, None).is_some(), "the recently used image stays");
  assert!(cache.state(new, None).is_none(), "the least recently used image went");
  assert_eq!(cache.used_bytes(), 64 * 32 * 4);
  assert_eq!(cache.pending(), 0);
}

#[derive(Clone, PartialEq, lurq::DevtoolsInspectable)]
struct RowData {
  id: usize,
}

/// A counter shared with the rows, equal by identity so props compare.
#[derive(Clone)]
struct Shared(Arc<AtomicUsize>);

impl PartialEq for Shared {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

#[cfg(feature = "devtools")]
impl lurq::app::component::DevtoolsInspectable for Shared {}

struct ThumbnailRow;

impl Component for ThumbnailRow {
  type Props = (RowData, Shared);

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let (row, builds) = ctx.props::<Self::Props>().clone();
    builds.0.fetch_add(1, Ordering::SeqCst);
    Column::new().height(20.0).child(
      Image::from_bytes(RED_64X32)
        .cache_key(unique_key(RED_64X32, "virtualized"))
        .downscale_to(16, 16)
        .size(16.0, 8.0)
        .id(format!("thumb-{}", row.id)),
    )
  }
}

struct ThumbnailList;

impl Component for ThumbnailList {
  type Props = Shared;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let builds = ctx.props::<Self::Props>().clone();
    let items: Vec<RowData> = (0..40).map(|id| RowData { id }).collect();
    VirtualizedList::new(ctx, items)
      .size(100.0, 60.0)
      .overscan_px(0.0)
      .mount_keyed::<ThumbnailRow, _, _, _>(|row| row.id.to_string(), move |row| (row.clone(), builds.clone()))
  }
}

#[test]
fn virtualized_rows_built_as_they_scroll_share_one_decode() {
  let builds = Arc::new(AtomicUsize::new(0));
  let mut app = App::new();
  let mut runtime = Tree::new();
  runtime.mount_root::<ThumbnailList>(&mut app, Shared(builds.clone()));
  render_pass_with_app(&mut runtime, &mut app);
  assert!(wait_for_decoded_images(WAIT));
  let completed = decoded_images().completed();
  let first = render_pass_with_app(&mut runtime, &mut app);
  assert_eq!(
    first.images.len(),
    3,
    "the visible rows show their thumbnails: {:?}",
    first.images
  );

  // Scroll to the end: rows are dropped and new ones built.
  runtime.scroll(50.0, 30.0, 0.0, -2000.0, lurq::app::events::ScrollPhase::Scroll);
  let scrolled = render_pass_with_app(&mut runtime, &mut app);

  assert!(builds.load(Ordering::SeqCst) > 3, "new rows were built");
  assert_eq!(
    scrolled.images.len(),
    3,
    "the new rows show their thumbnails at once: {:?}",
    scrolled.images
  );
  assert_eq!(decoded_images().completed(), completed, "without decoding again");
}
