//! A link preview: the board's card, row and inline form at rest, what it shows while the page is read, what a page
//! says and what the app gives over it, a page that tells nothing and one that could not be read, a video, the one
//! link it is to the pointer, the keyboard and tooling, the close of one in a composer, a text longer than its
//! room, a narrow room, a theme's rule, and a page like GitHub's with its picture and its icons, read through the
//! app's loader.

use std::{
  sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
  },
  time::Duration,
};

use lurq::{
  app::{
    open_url,
    theme::{PaletteColor, ShadowStyle, ThemePalette},
  },
  components::Column,
  core::Locale,
  layout::quad::QuadContent,
  node::{BoxShadowValue, color::Color, dimension::Dimension},
  resources::{LinkFetch, LinkFetcher, LinkMeta, LinkPreviewError, LinkRequest, LoadWaker, wait_link_fetches},
  ui::{Case, Interaction, LinkPreview, LinkPreviewPart, LinkPreviewState, LinkPreviewVariant},
};
use parking_lot::Mutex;

use super::stage::Stage;

const URL: &str = "https://lurq.dev/docs/layout/rows-and-columns";
const TITLE: &str = "Layout in lurq: rows, columns and gaps";
const DESCRIPTION: &str = "How a row shares its width.";

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// What the page at [`URL`] says.
fn page() -> LinkMeta {
  LinkMeta::new(URL)
    .title(TITLE)
    .description(DESCRIPTION)
    .site("lurq.dev")
}

/// A fetcher of the tests' own: it answers what it is set to, counts what it is asked, and wakes who asked once
/// it is set anew.
#[derive(Clone)]
struct Pages {
  answer: Arc<Mutex<LinkFetch>>,
  asked: Arc<AtomicUsize>,
  waiting: Arc<Mutex<Vec<LoadWaker>>>,
}

impl Pages {
  fn answering(answer: LinkFetch) -> Self {
    Self {
      answer: Arc::new(Mutex::new(answer)),
      asked: Arc::new(AtomicUsize::new(0)),
      waiting: Arc::new(Mutex::new(Vec::new())),
    }
  }

  fn set(&self, answer: LinkFetch) {
    *self.answer.lock() = answer;
    let waiting = std::mem::take(&mut *self.waiting.lock());
    waiting.iter().for_each(|wake| wake());
  }

  fn asked(&self) -> usize {
    self.asked.load(Ordering::SeqCst)
  }
}

impl LinkFetcher for Pages {
  fn fetch(&self, request: &LinkRequest<'_>) -> LinkFetch {
    self.asked.fetch_add(1, Ordering::SeqCst);
    let answer = self.answer.lock().clone();
    if answer == LinkFetch::Loading {
      self.waiting.lock().push(request.wake().clone());
    }
    answer
  }
}

/// What a preview opens, in place of the browser, for as long as it is held.
fn opened() -> (Arc<Mutex<Vec<String>>>, open_url::StandIn) {
  let opened = Arc::new(Mutex::new(Vec::new()));
  let log = opened.clone();
  let held = open_url::stand_in(move |url| log.lock().push(url.to_owned()));
  (opened, held)
}

/// The longest a test waits for its fetchers to answer: they answer at once, on a thread of lurq's.
const WAIT: Duration = Duration::from_secs(30);

/// Draws what the previews' fetchers answered: they are asked off the UI thread, and answer after the frame that
/// asked them.
fn settle(stage: &mut Stage) {
  loop {
    match wait_link_fetches(WAIT) {
      Some(0) => return,
      Some(_) => stage.pass(),
      None => panic!("a fetcher did not answer"),
    }
  }
}

fn stage_of(preview: impl Fn() -> LinkPreview + Send + Sync + 'static) -> Stage {
  let mut stage = Stage::new(move |_| Column::new().padding(8.0).child(preview()).into());
  settle(&mut stage);
  stage
}

fn underlined(stage: &Stage, content: &str) -> bool {
  stage.quads().iter().any(|quad| {
    matches!(&quad.content, QuadContent::Text { text, style, .. } if text == content && style.underline.is_some())
  })
}

#[test]
fn a_card_is_the_boards_surface_with_the_site_the_title_and_the_description() {
  let pages = Pages::answering(LinkFetch::ready(page()));
  let fetcher = pages.clone();
  let mut stage = stage_of(move || LinkPreview::new(URL).fetcher(fetcher.clone()).id("preview"));
  let (_, _, width, height) = stage.bounds("preview");
  assert_eq!(width, 360.0, "the board's card is 360 wide");
  assert_eq!(stage.fill_color("preview"), Some(light(PaletteColor::Card)));
  let (color, size) = stage.text(TITLE);
  assert_eq!((color, size), (light(PaletteColor::CardForeground), 14.0));
  let (color, size) = stage.text(DESCRIPTION);
  assert_eq!((color, size), (light(PaletteColor::MutedForeground), 13.0));
  let (color, size) = stage.text("lurq.dev");
  assert_eq!((color, size), (light(PaletteColor::MutedForeground), 12.0));
  // Without a picture, the text alone: 12 over the site, 6 between the lines, 14 under the description.
  let site = stage.text_quad("lurq.dev").unwrap();
  let (_, top, ..) = stage.bounds("preview");
  assert!(site.y - top >= 12.0 && site.y - top < 16.0, "{}", site.y - top);
  assert!(height < 120.0, "the text alone: {height}");
  // The recipe's numbers that are not painted as a box of their own.
  let theme = stage.app.theme().clone();
  let card = theme.resolve::<LinkPreview>(&Case::new().variant(LinkPreviewVariant::Card), None);
  assert!(card.part(LinkPreviewPart::Text).rest().gap.is_some());
  assert_eq!(
    card.part(LinkPreviewPart::Root).rest().shadow,
    Some(BoxShadowValue::Theme(ShadowStyle::Sm))
  );
  assert_eq!(pages.asked() > 0, true, "the page was asked for");
}

#[cfg(feature = "raster")]
#[test]
fn a_card_with_a_picture_has_it_on_top_at_the_ratio_of_the_board_and_a_video_has_its_mark_and_length() {
  use lurq::{images::ImageData, ui::Picture};
  let pixels = || Picture::new(ImageData::from_rgba(vec![120; 8 * 4 * 4], 8, 4));
  let mut stage = stage_of(move || {
    LinkPreview::new(URL)
      .title("A gallery page in ten minutes")
      .description("From an empty file to a page with a playground.")
      .site("lurq.dev")
      .no_icon()
      .image(pixels())
      .video(250.0)
      .id("preview")
  });
  let (_, top, width, height) = stage.bounds("preview");
  assert_eq!(width, 360.0);
  let title = stage.text_quad("A gallery page in ten minutes").unwrap();
  // The picture is 360 by 188, and the text is under it.
  assert!((title.y - top - 188.0 - 12.0 - 18.0).abs() < 8.0, "{}", title.y - top);
  assert!(height > 188.0 + 60.0, "{height}");
  assert!(stage.paints("4:10"), "the length of the video");
  assert_ne!(
    stage.font_of("4:10"),
    stage.font_of("A gallery page in ten minutes"),
    "in the mono type"
  );
}

/// A picture the app gives that shows what is behind it has its plate in the first frame; in a row, a square of
/// 72 with the picture 8 from its edges.
#[cfg(feature = "raster")]
#[test]
fn a_given_picture_on_nothing_has_its_plate_in_the_first_frame_and_in_a_row() {
  use lurq::{images::ImageData, ui::Picture};
  // A dark square in the middle of nothing.
  let rgba: Vec<u8> = (0..32 * 32)
    .flat_map(|at| {
      let (x, y) = (at % 32, at / 32);
      if (8..24).contains(&x) && (8..24).contains(&y) {
        [16, 16, 16, 255]
      } else {
        [0, 0, 0, 0]
      }
    })
    .collect();
  let data = ImageData::from_rgba(rgba, 32, 32);
  assert!(data.has_transparency());
  assert!(
    !ImageData::from_rgba(vec![255; 4 * 4 * 4], 4, 4).has_transparency(),
    "opaque"
  );
  let made = data.clone();
  let stage = stage_of(move || {
    LinkPreview::new(URL)
      .title(TITLE)
      .site("lurq.dev")
      .no_icon()
      .image(Picture::new(made.clone()))
      .variant(LinkPreviewVariant::Compact)
  });
  let white = Color::new(255, 255, 255, 255);
  let quads = stage.quads();
  let plate = quads
    .iter()
    .find(|quad| {
      quad.width == 72.0
        && quad.height == 72.0
        && matches!(quad.content, QuadContent::Rect { color, .. } if color == white)
    })
    .expect("the plate, a square of 72");
  let picture = quads
    .iter()
    .find(|quad| matches!(&quad.content, QuadContent::Image { data: shown, .. } if shown.id() == data.id()))
    .expect("the picture");
  assert_eq!(
    (picture.width, picture.height),
    (56.0, 56.0),
    "whole, 8 from the plate's edges"
  );
  assert_eq!((picture.x - plate.x, picture.y - plate.y), (8.0, 8.0));
}

/// In a row's square, a picture much wider than square (GitHub's 2:1 card of a repository) is drawn whole in its
/// own muted box, and one near square is cut to the square as before. A card cuts both to its ratio.
#[cfg(feature = "raster")]
#[test]
fn in_a_row_a_wide_picture_is_whole_and_a_square_one_covers_the_square() {
  use lurq::{images::ImageData, ui::Picture};
  let opaque = |width: u32, height: u32| ImageData::from_rgba(vec![255; (width * height * 4) as usize], width, height);
  let (wide, square, tall) = (opaque(40, 20), opaque(20, 20), opaque(20, 40));
  let shown = |stage: &Stage, data: &ImageData| {
    stage
      .quads()
      .into_iter()
      .find(|quad| matches!(&quad.content, QuadContent::Image { data: shown, .. } if shown.id() == data.id()))
      .unwrap_or_else(|| panic!("the picture of {}x{}", data.width(), data.height()))
  };
  let row = |data: &ImageData, variant: LinkPreviewVariant| {
    let made = data.clone();
    stage_of(move || {
      LinkPreview::new(URL)
        .title(TITLE)
        .site("lurq.dev")
        .no_icon()
        .image(Picture::new(made.clone()))
        .variant(variant)
        .id("preview")
    })
  };
  let stage = row(&wide, LinkPreviewVariant::Compact);
  let picture = shown(&stage, &wide);
  assert_eq!((picture.width, picture.height), (72.0, 36.0), "whole");
  let muted = light(PaletteColor::Muted);
  assert!(
    stage.quads().iter().any(|quad| quad.width == 72.0
      && quad.height == 72.0
      && matches!(quad.content, QuadContent::Rect { color, .. } if color == muted)),
    "in its own muted box"
  );
  assert!(
    !stage.quads().iter().any(|quad| quad.width == 72.0
      && quad.height == 72.0
      && matches!(quad.content, QuadContent::Rect { color, .. } if color == Color::new(255, 255, 255, 255))),
    "and on no plate"
  );
  let stage = row(&tall, LinkPreviewVariant::Compact);
  let picture = shown(&stage, &tall);
  assert_eq!((picture.width, picture.height), (36.0, 72.0), "whole");
  let stage = row(&square, LinkPreviewVariant::Compact);
  let picture = shown(&stage, &square);
  assert_eq!((picture.width, picture.height), (72.0, 72.0), "covered");
  // A card keeps its ratio and its crop.
  let stage = row(&wide, LinkPreviewVariant::Card);
  let picture = shown(&stage, &wide);
  assert_eq!(picture.width, 360.0);
  assert!((picture.height - 188.0).abs() < 1.0, "{}", picture.height);
}

#[test]
fn while_the_page_is_read_it_is_the_skeleton_and_then_what_the_page_says() {
  let pages = Pages::answering(LinkFetch::Loading);
  let fetcher = pages.clone();
  let mut stage = stage_of(move || LinkPreview::new(URL).fetcher(fetcher.clone()).id("preview"));
  assert!(!stage.paints(TITLE));
  assert!(!stage.paints("lurq.dev"), "nothing but the skeleton");
  let (_, _, width, _) = stage.bounds("preview");
  assert_eq!(width, 360.0);
  pages.set(LinkFetch::ready(page()));
  stage.pass();
  settle(&mut stage);
  assert!(stage.paints(TITLE));
  assert!(stage.paints(DESCRIPTION));
}

#[test]
fn a_field_the_app_gives_is_shown_over_the_pages_and_the_rest_is_fetched() {
  let pages = Pages::answering(LinkFetch::ready(page()));
  let fetcher = pages.clone();
  let mut stage = stage_of(move || {
    LinkPreview::new(URL)
      .title("Our label")
      .fetcher(fetcher.clone())
      .id("preview")
  });
  assert!(stage.paints("Our label"));
  assert!(!stage.paints(TITLE));
  assert!(stage.paints(DESCRIPTION), "the page's description");
  assert!(pages.asked() > 0);

  // Everything it shows given: nothing is asked of the fetcher.
  let pages = Pages::answering(LinkFetch::ready(page()));
  let fetcher = pages.clone();
  let mut stage = stage_of(move || {
    LinkPreview::new(URL)
      .title("Known")
      .description("All of it.")
      .site("example.com")
      .no_image()
      .no_icon()
      .fetcher(fetcher.clone())
  });
  stage.pass();
  assert!(stage.paints("Known") && stage.paints("All of it.") && stage.paints("example.com"));
  assert_eq!(pages.asked(), 0, "nothing is fetched");
  // Where the page says no site, it is the domain of the address.
  let mut stage = stage_of(|| {
    LinkPreview::new("https://www.example.org/a")
      .fetcher(|url: &str| LinkFetch::ready(LinkMeta::new(url).title("Untitled site")))
  });
  assert!(stage.paints("example.org"));
  stage.pass();
}

#[test]
fn a_page_that_tells_nothing_shows_its_address_and_one_that_failed_says_why() {
  let mut stage = stage_of(|| {
    LinkPreview::new(URL)
      .variant(LinkPreviewVariant::Compact)
      .fetcher(|url: &str| LinkFetch::ready(LinkMeta::new(url)))
  });
  assert!(stage.paints("lurq.dev/docs/layout/rows-and-columns"));
  assert!(stage.paints("No preview for this page."));
  assert!(stage.paints("lurq.dev"));
  let mut failed = stage_of(|| {
    LinkPreview::new(URL)
      .variant(LinkPreviewVariant::Compact)
      .fetcher(|_: &str| LinkFetch::Failed(LinkPreviewError::NotFound))
  });
  assert!(failed.paints("lurq.dev/docs/layout/rows-and-columns"));
  assert!(failed.paints("The page was not found."));
  failed.pass();
  // In the words of its locale, and in the app's own where it gives them.
  let mut ukrainian = stage_of(|| {
    LinkPreview::new(URL)
      .locale(&Locale::for_tag("uk"))
      .fetcher(|_: &str| LinkFetch::Failed(LinkPreviewError::Server(503)))
  });
  assert!(ukrainian.paints("Сайт відповів 503."));
  ukrainian.pass();
  let mut own = stage_of(|| {
    LinkPreview::new(URL)
      .no_preview_text("Nothing to show.")
      .fetcher(|url: &str| LinkFetch::ready(LinkMeta::new(url)))
  });
  assert!(own.paints("Nothing to show."));
}

#[test]
fn enter_space_and_a_click_open_the_address_unless_the_app_decides() {
  let (opened, _held) = opened();
  let mut stage = stage_of(|| {
    LinkPreview::new(URL)
      .fetcher(|_: &str| LinkFetch::ready(page()))
      .id("preview")
  });
  stage.key("Tab");
  assert!(stage.has_ring(light(PaletteColor::Ring)), "the ring from the keyboard");
  stage.key("Enter");
  stage.key(" ");
  stage.click("preview");
  assert_eq!(*opened.lock(), [URL, URL, URL]);

  // The app decides: told the address, and nothing is opened.
  let told = Arc::new(Mutex::new(Vec::<String>::new()));
  let log = told.clone();
  let mut stage = stage_of(move || {
    let log = log.clone();
    LinkPreview::new(URL)
      .fetcher(|_: &str| LinkFetch::ready(page()))
      .on_open(move |url| log.lock().push(url.to_owned()))
      .id("preview")
  });
  stage.click("preview");
  assert_eq!(*told.lock(), [URL]);
  assert_eq!(opened.lock().len(), 3, "nothing more was opened");

  // Disabled, nothing.
  let mut stage = stage_of(|| {
    LinkPreview::new(URL)
      .fetcher(|_: &str| LinkFetch::ready(page()))
      .disabled(true)
      .id("preview")
  });
  stage.click("preview");
  stage.key("Tab");
  stage.key("Enter");
  assert_eq!(opened.lock().len(), 3);
}

#[test]
fn the_close_of_one_in_a_composer_is_its_own_stop_and_tells_the_app() {
  let (opened, _held) = opened();
  let removed = Arc::new(AtomicUsize::new(0));
  let count = removed.clone();
  let mut stage = stage_of(move || {
    let count = count.clone();
    LinkPreview::new(URL)
      .variant(LinkPreviewVariant::Compact)
      .fetcher(|_: &str| LinkFetch::ready(page()))
      .on_remove(move || {
        count.fetch_add(1, Ordering::SeqCst);
      })
      .id("preview")
  });
  // The link first, then its close.
  stage.key("Tab");
  stage.key("Tab");
  stage.key(" ");
  assert_eq!(removed.load(Ordering::SeqCst), 1);
  assert!(opened.lock().is_empty(), "the close opens nothing");
  stage.click("preview-remove");
  assert_eq!(removed.load(Ordering::SeqCst), 2);
  assert!(opened.lock().is_empty(), "a press of the close is the close's alone");
  let (_, _, width, height) = stage.bounds("preview-remove");
  assert_eq!((width, height), (28.0, 28.0));
}

#[test]
fn under_the_pointer_it_lifts_and_its_title_is_underlined() {
  let mut stage = stage_of(|| {
    LinkPreview::new(URL)
      .fetcher(|_: &str| LinkFetch::ready(page()))
      .id("preview")
  });
  assert!(!underlined(&stage, TITLE));
  stage.hover("preview");
  stage.pass();
  assert!(underlined(&stage, TITLE), "the title is underlined");
  let theme = stage.app.theme().clone();
  let card = theme.resolve::<LinkPreview>(&Case::new().variant(LinkPreviewVariant::Card), None);
  let hovered = card.part(LinkPreviewPart::Root).styles().hovered.clone();
  assert_eq!(hovered.shadow, Some(BoxShadowValue::Theme(ShadowStyle::Md)), "lifted");
  assert!(hovered.veil.is_some(), "the hover's veil");
  // Forced, as a gallery draws it.
  let stage = stage_of(|| {
    LinkPreview::new(URL)
      .fetcher(|_: &str| LinkFetch::ready(page()))
      .force(Some(Interaction::Hovered))
  });
  assert!(underlined(&stage, TITLE));
}

#[test]
fn the_compact_row_and_the_inline_form_are_the_boards() {
  let mut stage = stage_of(|| {
    LinkPreview::new(URL)
      .variant(LinkPreviewVariant::Compact)
      .fetcher(|_: &str| LinkFetch::ready(page()))
      .no_image()
      .id("row")
  });
  let (_, _, width, _) = stage.bounds("row");
  // The stage is 400 wide and padded 8: the row of 432 is as wide as its room.
  assert_eq!(width, 384.0);
  let mut stage = stage_of(|| {
    LinkPreview::new(URL)
      .variant(LinkPreviewVariant::Inline)
      .title("Layout in lurq")
      .fetcher(|_: &str| LinkFetch::ready(page()))
      .id("inline")
  });
  let (_, _, _, height) = stage.bounds("inline");
  assert_eq!(height, 24.0);
  assert_eq!(stage.text("Layout in lurq"), (light(PaletteColor::Foreground), 13.0));
  assert!(stage.paints("lurq.dev"));
  assert!(!stage.paints(DESCRIPTION), "an inline one shows no description");
  // The title alone.
  let stage = stage_of(|| {
    LinkPreview::new(URL)
      .variant(LinkPreviewVariant::Inline)
      .title("Layout in lurq")
      .no_site()
      .fetcher(|_: &str| LinkFetch::ready(page()))
  });
  assert!(stage.paints("Layout in lurq") && !stage.paints("lurq.dev"));
}

#[test]
fn a_title_longer_than_its_room_ends_in_an_ellipsis_and_a_narrow_room_takes_it() {
  let long = "A title that goes on and on, far past the width of a card of three hundred and sixty, and on again \
              past a second line as well";
  let mut stage = Stage::new(move |_| {
    Column::new()
      .width(Dimension::Px(220.0))
      .child(
        LinkPreview::new(URL)
          .fetcher(move |url: &str| LinkFetch::ready(LinkMeta::new(url).title(long).site("lurq.dev")))
          .id("card"),
      )
      .child(
        LinkPreview::new(URL)
          .variant(LinkPreviewVariant::Inline)
          .fetcher(move |url: &str| LinkFetch::ready(LinkMeta::new(url).title(long).site("lurq.dev")))
          .id("inline"),
      )
      .into()
  });
  settle(&mut stage);
  let (_, _, card, height) = stage.bounds("card");
  assert_eq!(card, 220.0, "a card is never wider than its room");
  assert!(height < 140.0, "two lines of title at most: {height}");
  let (_, _, inline, _) = stage.bounds("inline");
  assert!(inline <= 220.0, "{inline}");
  assert!(stage.cut_text_quad(long).is_some(), "cut with an ellipsis");
  assert!(stage.paints("lurq.dev"), "the site keeps its room in a sentence");
}

/// A fetcher that holds its pages: it gives them on the UI thread, at once.
struct Held;

impl LinkFetcher for Held {
  fn fetch(&self, _: &LinkRequest<'_>) -> LinkFetch {
    LinkFetch::ready(page())
  }

  fn held(&self, _: &str) -> Option<LinkFetch> {
    Some(LinkFetch::ready(page()))
  }
}

#[test]
fn the_first_frame_is_the_settled_one() {
  // A page the fetcher holds is shown in the first frame, with no frame of the skeleton before it.
  let scene = |_: &mut lurq::app::ctx::Ctx| {
    Column::new()
      .padding(8.0)
      .child(LinkPreview::new(URL).fetcher(Held).id("preview"))
      .into()
  };
  let mut stage = Stage::first_frame(400.0, 300.0, scene);
  let first = stage.bounds("preview");
  assert!(stage.paints(TITLE));
  stage.frame();
  assert_eq!(stage.bounds("preview"), first);
  assert_eq!(wait_link_fetches(WAIT), Some(0), "nothing was asked off the UI thread");
}

/// A fetcher that blocks until it is let go, as one that reads a slow disk or asks a server would. It tells
/// whether it was ever asked on the thread that draws, where it does not block, so that the test fails and does
/// not hang.
struct Blocking {
  gate: Arc<Mutex<std::sync::mpsc::Receiver<()>>>,
  drawing: std::thread::ThreadId,
  on_the_drawing_thread: Arc<AtomicBool>,
  asked: Arc<AtomicUsize>,
}

impl LinkFetcher for Blocking {
  fn fetch(&self, _: &LinkRequest<'_>) -> LinkFetch {
    self.asked.fetch_add(1, Ordering::SeqCst);
    if std::thread::current().id() == self.drawing {
      self.on_the_drawing_thread.store(true, Ordering::SeqCst);
      return LinkFetch::Loading;
    }
    // Let go once the test drops its end of the channel: nothing is ever sent.
    let _ = self.gate.lock().recv();
    LinkFetch::ready(page())
  }
}

#[test]
fn a_fetcher_that_blocks_holds_no_frame_and_the_previews_are_skeletons_until_it_answers() {
  const PREVIEWS: usize = 12;
  let (let_go, gate) = std::sync::mpsc::channel::<()>();
  let on_the_drawing_thread = Arc::new(AtomicBool::new(false));
  let asked = Arc::new(AtomicUsize::new(0));
  let fetcher: Arc<dyn LinkFetcher> = Arc::new(Blocking {
    gate: Arc::new(Mutex::new(gate)),
    drawing: std::thread::current().id(),
    on_the_drawing_thread: on_the_drawing_thread.clone(),
    asked: asked.clone(),
  });
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .with_children((0..PREVIEWS).map(|at| {
        let variant = if at % 2 == 0 {
          LinkPreviewVariant::Card
        } else {
          LinkPreviewVariant::Compact
        };
        LinkPreview::new(URL)
          .variant(variant)
          .shared_fetcher(fetcher.clone())
          .id(format!("preview-{at}"))
      }))
      .into()
  });
  // The passes return while every fetch blocks, and each preview is its skeleton.
  stage.frame();
  assert!(
    !on_the_drawing_thread.load(Ordering::SeqCst),
    "the fetcher was asked on the thread that draws"
  );
  assert!(asked.load(Ordering::SeqCst) > 0, "the fetcher was asked");
  assert!(
    !stage.paints(TITLE) && !stage.paints("lurq.dev"),
    "nothing is known yet"
  );
  for at in 0..PREVIEWS {
    let (_, _, width, height) = stage.bounds(&format!("preview-{at}"));
    assert!(width > 0.0 && height > 0.0, "preview {at} is drawn: {width} x {height}");
  }
  assert_eq!(
    wait_link_fetches(Duration::ZERO),
    None,
    "the fetches are still on their way"
  );
  // Let go: the fetches answer, and the previews show the page.
  drop(let_go);
  settle(&mut stage);
  assert!(stage.paints(TITLE));
  assert!(!on_the_drawing_thread.load(Ordering::SeqCst));
}

#[test]
fn a_rule_of_a_theme_changes_it() {
  let mut stage = stage_of(|| {
    LinkPreview::new(URL)
      .fetcher(|_: &str| LinkFetch::ready(page()))
      .style(LinkPreviewPart::Title, |s| s.font_size(18.0))
      .style(LinkPreviewPart::Root, |s| s.background(PaletteColor::Secondary))
      .id("preview")
  });
  assert_eq!(stage.text(TITLE).1, 18.0);
  assert_eq!(stage.fill_color("preview"), Some(light(PaletteColor::Secondary)));
  let theme = stage.app.theme().clone();
  let loading = theme.resolve::<LinkPreview>(&Case::new().state(LinkPreviewState::Loading, true), None);
  assert!(loading.part(LinkPreviewPart::Skeleton).rest().gap.is_some());
}

#[test]
fn an_inline_one_stands_in_a_sentence_keeps_its_line_and_is_a_stop_of_tab() {
  use lurq::{
    components::Text,
    layout::{quad::RichTextSpan, text_style::TextStyle},
  };
  let (opened, _held) = opened();
  let body = || TextStyle {
    font_size: 14.0,
    line_height: 1.5,
    ..TextStyle::default()
  };
  let sentence = move |with: bool| {
    let mut text = Text::rich([
      // On one line in the stage's 400.
      RichTextSpan::new("The gaps are in ", body()),
      RichTextSpan::object(body()),
      RichTextSpan::new(" today.", body()),
    ])
    .id("sentence");
    if with {
      text = text.object(
        LinkPreview::new(URL)
          .variant(LinkPreviewVariant::Inline)
          .title("Layout in lurq")
          .site("lurq.dev")
          .no_icon()
          .id("inline"),
      );
    }
    text
  };
  let mut stage = Stage::new(move |_| Column::new().child(sentence(true)).into());
  let (_, top, _, line) = stage.bounds("sentence");
  let (x, y, width, height) = stage.bounds("inline");
  assert_eq!(height, 24.0);
  assert!(x > 80.0 && width > 60.0, "after the words before it: {x}, {width}");
  assert!(
    (y + height / 2.0 - (top + line / 2.0)).abs() < 1.0,
    "on the middle of its line"
  );
  let mut plain = Stage::new(move |_| Column::new().child(sentence(false)).into());
  let (_, _, _, plain_line) = plain.bounds("sentence");
  assert_eq!(line, plain_line, "the line keeps its height");
  stage.key("Tab");
  stage.key("Enter");
  assert_eq!(*opened.lock(), [URL]);
}

#[cfg(any(feature = "devtools", feature = "mcp"))]
#[test]
fn tooling_reads_one_link_named_by_its_title_and_described_by_its_site_and_description() {
  let mut stage = stage_of(|| {
    LinkPreview::new(URL)
      .fetcher(|_: &str| LinkFetch::ready(page()))
      .video(250.0)
      .on_remove(|| ())
      .id("preview")
  });
  let mut said = |id: &str, name: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(name))
  };
  assert_eq!(said("preview", "a11y_role").as_deref(), Some("link"));
  assert_eq!(said("preview", "a11y_name").as_deref(), Some(TITLE));
  let description = said("preview", "a11y_description").unwrap();
  assert!(
    description.contains("lurq.dev") && description.contains(DESCRIPTION),
    "{description}"
  );
  assert!(description.contains("Video, 4:10"), "{description}");
  assert_eq!(said("preview-remove", "a11y_role").as_deref(), Some("button"));
  assert_eq!(said("preview-remove", "a11y_name").as_deref(), Some("Remove preview"));
}

/// The id of what has the focus.
fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

/// A draft with a button before a preview in it and, where `after`, one after it: the close of the preview takes
/// it out, and the focus goes where `return_focus` says, or on.
fn draft(after: bool, return_to: Option<lurq::core::ElementRef>) -> Stage {
  use lurq::{core::Signal, ui::Button};
  let shown = Signal::new(true);
  let mut stage = Stage::new(move |_| {
    let mut column = Column::new().padding(8.0).child(Button::new("Before").id("before"));
    if let Some(element) = &return_to {
      column = column.child(Button::new("Back here").id("back").ref_element(element.clone()));
    }
    if shown.get() {
      let gone = shown.clone();
      let mut preview = LinkPreview::new(URL)
        .variant(LinkPreviewVariant::Compact)
        .fetcher(|_: &str| LinkFetch::ready(page()))
        .no_image()
        .on_remove(move || gone.set(false))
        .id("preview");
      if let Some(element) = &return_to {
        preview = preview.return_focus(element);
      }
      column = column.child(preview);
    }
    if after {
      column = column.child(Button::new("After").id("after"));
    }
    column.into()
  });
  settle(&mut stage);
  stage
}

#[test]
fn after_the_close_the_focus_goes_on_or_back_and_never_to_nothing() {
  // What follows the preview takes the focus.
  let mut stage = draft(true, None);
  stage.key("Tab");
  stage.key("Tab");
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("preview-remove"));
  stage.key("Enter");
  stage.pass();
  assert!(
    stage.tree.get_element_by_id_mut("preview").is_none(),
    "the app took it out"
  );
  assert_eq!(focused(&stage).as_deref(), Some("after"));

  // Where nothing follows it, what comes before it.
  let mut stage = draft(false, None);
  stage.key("Tab");
  stage.key("Tab");
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("preview-remove"));
  stage.key(" ");
  stage.pass();
  assert_eq!(focused(&stage).as_deref(), Some("before"));

  // Where the app says.
  let back = lurq::core::ElementRef::new();
  let mut stage = draft(true, Some(back));
  stage.click("preview-remove");
  stage.pass();
  assert_eq!(focused(&stage).as_deref(), Some("back"));
}

#[cfg(feature = "image")]
#[test]
fn a_page_that_names_no_icon_has_its_sites_favicon_tried_and_the_link_mark_where_there_is_none() {
  let asked = Arc::new(Mutex::new(Vec::<String>::new()));
  let app = lurq::app::App::new();
  let log = asked.clone();
  // A loader that loads nothing and tells what it was asked for.
  app.loader().allow(move |address| {
    log.lock().push(address.to_owned());
    false
  });
  let mut stage = Stage::in_app(app, |_| {
    Column::new()
      .padding(8.0)
      .child(
        LinkPreview::new("https://lurq.dev/docs/a")
          .variant(LinkPreviewVariant::Compact)
          .fetcher(|url: &str| LinkFetch::ready(LinkMeta::new(url).title("A").site("lurq.dev")))
          .no_image(),
      )
      .child(
        LinkPreview::new("https://lurq.dev/docs/b")
          .variant(LinkPreviewVariant::Compact)
          .fetcher(|url: &str| LinkFetch::ready(LinkMeta::new(url).title("B").site("lurq.dev")))
          .no_image(),
      )
      .into()
  });
  settle(&mut stage);
  stage.pass();
  stage.pass();
  assert_eq!(
    *asked.lock(),
    ["https://lurq.dev/favicon.ico"],
    "asked of the loader once for the site, and kept"
  );
  let link_mark = lurq::ui::icons::ALL
    .iter()
    .find_map(|(name, glyph)| (*name == lurq::ui::icons::LINK).then(|| glyph.to_string()))
    .unwrap();
  let marks = stage
    .quads()
    .iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if *text == link_mark))
    .count();
  assert_eq!(marks, 2, "the link mark stands for the icon that is not there");
}

#[test]
fn a_selection_copies_it_as_a_markdown_link_and_one_that_told_nothing_as_its_address() {
  let copied = Arc::new(Mutex::new(Vec::<String>::new()));
  let log = copied.clone();
  let app = lurq::app::App::new();
  let renderer: lurq::ui::LinkPreviewRenderer = Arc::new(move |view: lurq::ui::LinkPreviewView| {
    log.lock().push(view.copy_text());
    view.render()
  });
  app.theme().set_renderer::<LinkPreview>(renderer);
  let mut stage = Stage::in_app(app, |_| {
    Column::new()
      .child(
        LinkPreview::new(URL)
          .variant(LinkPreviewVariant::Inline)
          .title("Layout [in] lurq")
          .site("lurq.dev")
          .no_icon(),
      )
      .child(
        LinkPreview::new("https://lurq.dev/x")
          .variant(LinkPreviewVariant::Inline)
          .fetcher(|url: &str| LinkFetch::ready(LinkMeta::new(url))),
      )
      .into()
  });
  settle(&mut stage);
  let copied = copied.lock().clone();
  assert!(copied.contains(&format!(r"[Layout \[in\] lurq]({URL})")), "{copied:?}");
  assert!(copied.contains(&"https://lurq.dev/x".to_owned()), "{copied:?}");
}

/// A page like GitHub's, its picture and its icons, served by the app's loader from a folder of the test's own:
/// the tags of GitHub's own page, with its addresses made resources of the app.
#[cfg(feature = "image")]
mod a_real_page {
  use lurq::{
    app::{
      App,
      theme::{PaletteColor, ThemePalette},
      window::Appearance,
    },
    components::Column,
    layout::quad::{Quad, QuadContent},
    node::color::Color,
    resources::{LinkFetcher, PageFetcher, wait_link_fetches},
    ui::{LinkPreview, LinkPreviewVariant},
  };

  use super::{Arc, Duration, Stage};

  /// A PNG of `width` by `height`, each pixel what `pixel` says of it.
  pub(super) fn png(width: u32, height: u32, pixel: impl Fn(u32, u32) -> [u8; 4]) -> Vec<u8> {
    let mut bytes = Vec::new();
    {
      let mut encoder = png::Encoder::new(&mut bytes, width, height);
      encoder.set_color(png::ColorType::Rgba);
      encoder.set_depth(png::BitDepth::Eight);
      let mut writer = encoder.write_header().unwrap();
      let pixels: Vec<u8> = (0..height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .flat_map(|(x, y)| pixel(x, y))
        .collect();
      writer.write_image_data(&pixels).unwrap();
    }
    bytes
  }

  /// GitHub's icon as it draws it: a dark disc on nothing, made for the light bar of a browser's tabs.
  pub(super) fn dark_disc() -> Vec<u8> {
    png(32, 32, |x, y| {
      let (dx, dy) = (x as f32 - 15.5, y as f32 - 15.5);
      if dx * dx + dy * dy < 15.0 * 15.0 {
        [0x24, 0x29, 0x2E, 255]
      } else {
        [0, 0, 0, 0]
      }
    })
  }

  pub(super) const SVG_ICON: &str = r##"<svg width="32" height="32" viewBox="0 0 32 32" xmlns="http://www.w3.org/2000/svg"><circle cx="16" cy="16" r="15" fill="#24292E"/></svg>"##;

  /// A folder of resources with the files of the test `tag` under `tag/`, so that what one test keeps of an
  /// address is not another's.
  pub(super) fn site(tag: &str, files: &[(&str, Vec<u8>)]) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("lurq-link-preview-{}", std::process::id()));
    for (name, bytes) in files {
      let path = root.join(tag).join(name);
      std::fs::create_dir_all(path.parent().unwrap()).unwrap();
      std::fs::write(path, bytes).unwrap();
    }
    root
  }

  /// GitHub's page with its addresses under `tag/`.
  pub(super) fn githubs_page(tag: &str) -> Vec<u8> {
    include_str!("../assets/link_preview/github.html")
      .replace("https://opengraph.githubassets.com/", &format!("{tag}/"))
      .replace("https://github.githubassets.com/", &format!("{tag}/"))
      .into_bytes()
  }

  /// Where GitHub's page has its picture, after its server's address.
  pub(super) const PICTURE: &str = "2d62901bfc9f6e2966648a4a1f90879fbc91ac9f4e750c54aa814d1061b81963/ivanzaida/lurq";

  /// A stage with a card of `url` in the dark theme, read by a fetcher of its own through `app`'s loader.
  pub(super) fn card(app: App, url: &str) -> Stage {
    card_with(app, url, |preview| preview)
  }

  /// A stage with a card of `url` in the dark theme, as `made` makes it, read by a fetcher of its own through
  /// `app`'s loader.
  pub(super) fn card_with(
    app: App,
    url: &str,
    made: impl Fn(LinkPreview) -> LinkPreview + Send + Sync + 'static,
  ) -> Stage {
    app.theme().set_preferred_appearance(Some(Appearance::Dark));
    let fetcher: Arc<dyn LinkFetcher> = Arc::new(PageFetcher::new());
    let url = url.to_owned();
    Stage::in_app(app, move |_| {
      Column::new()
        .padding(8.0)
        .child(made(
          LinkPreview::new(&url)
            .variant(LinkPreviewVariant::Card)
            .shared_fetcher(fetcher.clone())
            .id("preview"),
        ))
        .into()
    })
  }

  /// A page of Wikipedia's kind whose picture is `picture.png` of its own, and that names no icon.
  fn page_with_a_picture(tag: &str, picture: Vec<u8>) -> std::path::PathBuf {
    let page = format!(
      r#"<head><title>Rust (programming language)</title><meta property="og:site_name" content="Wikipedia">
      <meta property="og:image" content="{tag}/picture.png"></head>"#
    );
    site(tag, &[("page.html", page.into_bytes()), ("picture.png", picture)])
  }

  /// Rust's logo as Wikipedia gives it: a dark ring on nothing, square.
  fn dark_ring() -> Vec<u8> {
    png(96, 96, |x, y| {
      let (dx, dy) = (x as f32 - 47.5, y as f32 - 47.5);
      let distance = (dx * dx + dy * dy).sqrt();
      if (24.0..46.0).contains(&distance) {
        [0x10, 0x10, 0x10, 255]
      } else {
        [0, 0, 0, 0]
      }
    })
  }

  /// The rectangles of `color` that are `width` by `height`.
  fn boxes(stage: &Stage, color: Color, width: f32, height: f32) -> Vec<Quad> {
    stage
      .quads()
      .into_iter()
      .filter(|quad| {
        (quad.width - width).abs() < 1.0
          && (quad.height - height).abs() < 1.0
          && matches!(quad.content, QuadContent::Rect { color: fill, .. } if fill == color)
      })
      .collect()
  }

  /// The card's picture: the one image in it that is not the site's icon.
  fn the_picture(stage: &Stage) -> Option<Quad> {
    images(stage).into_iter().find(|quad| quad.width != 16.0)
  }

  /// Passes until `done` holds, while the fetcher, the loader and the decoder work on their threads.
  pub(super) fn until(stage: &mut Stage, what: &str, done: impl Fn(&Stage) -> bool) {
    for _ in 0..1000 {
      let _ = wait_link_fetches(Duration::from_secs(10));
      stage.pass();
      if done(stage) {
        return;
      }
      std::thread::sleep(Duration::from_millis(10));
    }
    panic!("{what} did not come");
  }

  fn images(stage: &Stage) -> Vec<Quad> {
    stage
      .quads()
      .into_iter()
      .filter(|quad| matches!(quad.content, QuadContent::Image { .. }))
      .collect()
  }

  /// The icon's pixels: an image of 16 by 16.
  pub(super) fn icon(stage: &Stage) -> Option<Quad> {
    images(stage)
      .into_iter()
      .find(|quad| quad.width == 16.0 && quad.height == 16.0)
  }

  /// The picture of the card: an image as wide as it.
  pub(super) fn picture(stage: &Stage) -> Option<Quad> {
    images(stage).into_iter().find(|quad| quad.width == 360.0)
  }

  fn pixels_of(quad: &Quad) -> (u32, u32) {
    match &quad.content {
      QuadContent::Image { data, .. } => (data.width(), data.height()),
      _ => (0, 0),
    }
  }

  fn link_mark() -> String {
    lurq::ui::icons::ALL
      .iter()
      .find_map(|(name, glyph)| (*name == lurq::ui::icons::LINK).then(|| glyph.to_string()))
      .unwrap()
  }

  #[test]
  fn githubs_page_shows_its_picture_and_its_icon_on_a_white_plate() {
    let tag = "github";
    let root = site(
      tag,
      &[
        ("page.html", githubs_page(tag)),
        (PICTURE, png(120, 60, |_, _| [40, 160, 90, 255])),
        ("favicons/favicon.png", dark_disc()),
        ("favicons/favicon.svg", SVG_ICON.as_bytes().to_vec()),
      ],
    );
    let app = App::new();
    app.loader().root(&root);
    let mut stage = card(app, &format!("{tag}/page.html"));
    until(&mut stage, "the picture and the icon", |stage| {
      picture(stage).is_some() && icon(stage).is_some()
    });
    assert!(stage.paints("GitHub - ivanzaida/lurq"));
    assert!(stage.paints("GitHub"));
    let picture = picture(&stage).unwrap();
    assert!((picture.height - 188.0).abs() < 1.0, "{}", picture.height);
    let icon = icon(&stage).unwrap();
    assert_eq!(pixels_of(&icon), (32, 32), "the PNG, before the SVG");
    assert!(!stage.paints(&link_mark()));
    // The dark disc is on a white plate, and in no box of a picture's own.
    let under = |quad: &Quad| quad.width == 16.0 && (quad.x - icon.x).abs() < 0.5 && (quad.y - icon.y).abs() < 0.5;
    let filled = |color: Color| {
      stage
        .quads()
        .iter()
        .any(|quad| under(quad) && matches!(quad.content, QuadContent::Rect { color: fill, .. } if fill == color))
    };
    assert!(
      filled(Color::new(255, 255, 255, 255)),
      "the icon stands on a white plate"
    );
    assert!(
      !filled(ThemePalette::dark().get(PaletteColor::Muted)),
      "no box of a picture's own around it"
    );
  }

  #[test]
  fn a_picture_on_nothing_stands_whole_on_a_light_plate_in_the_dark_theme_and_the_light() {
    let tag = "on-nothing";
    let root = page_with_a_picture(tag, dark_ring());
    let app = App::new();
    app.loader().root(&root);
    let mut stage = card(app, &format!("{tag}/page.html"));
    let white = Color::new(255, 255, 255, 255);
    until(&mut stage, "the picture on its plate", |stage| {
      the_picture(stage).is_some() && !boxes(stage, white, 360.0, 188.0).is_empty()
    });
    let check = |stage: &Stage, theme: &str| {
      let plate = boxes(stage, white, 360.0, 188.0);
      assert_eq!(plate.len(), 1, "{theme}: the plate, at the card's ratio");
      let plate = &plate[0];
      let picture = the_picture(stage).unwrap();
      // Whole: as high as the plate inside its padding of 16, as wide at its own ratio, in the middle.
      assert!((picture.height - 156.0).abs() < 1.0, "{theme}: {}", picture.height);
      assert!((picture.width - 156.0).abs() < 1.0, "{theme}: {}", picture.width);
      assert!((picture.y - plate.y - 16.0).abs() < 1.0, "{theme}");
      assert!(
        (picture.x + picture.width / 2.0 - plate.x - 180.0).abs() < 1.0,
        "{theme}"
      );
      // In no muted box of a picture's own.
      let muted = stage.app.theme().palette().get(PaletteColor::Muted);
      assert!(boxes(stage, muted, 328.0, 156.0).is_empty(), "{theme}");
      assert!(boxes(stage, muted, 360.0, 188.0).is_empty(), "{theme}");
      assert!(stage.paints("Rust (programming language)"), "{theme}");
    };
    check(&stage, "dark");
    stage.app.theme().set_preferred_appearance(Some(Appearance::Light));
    for _ in 0..3 {
      stage.pass();
    }
    assert_eq!(
      stage.app.theme().palette().get(PaletteColor::Card),
      ThemePalette::light().get(PaletteColor::Card)
    );
    check(&stage, "light");
  }

  #[test]
  fn an_opaque_picture_covers_its_box_with_no_plate_and_a_few_clear_corners_leave_it_opaque() {
    let tag = "opaque";
    // A photo with its corners cut round: a few pixels of nothing, under one in a hundred.
    let root = page_with_a_picture(
      tag,
      png(120, 60, |x, y| {
        let corner = !(2..118).contains(&x) && !(2..58).contains(&y);
        if corner { [0, 0, 0, 0] } else { [40, 160, 90, 255] }
      }),
    );
    let app = App::new();
    app.loader().root(&root);
    let mut stage = card(app, &format!("{tag}/page.html"));
    until(&mut stage, "the picture", |stage| picture(stage).is_some());
    for _ in 0..3 {
      stage.pass();
    }
    let picture = picture(&stage).unwrap();
    assert!((picture.height - 188.0).abs() < 1.0, "covered: {}", picture.height);
    let white = Color::new(255, 255, 255, 255);
    assert!(boxes(&stage, white, 360.0, 188.0).is_empty(), "no plate");
  }

  #[test]
  fn a_themes_rule_restyles_the_plate() {
    use lurq::ui::{LinkPreviewPart, LinkPreviewState, Recipe};
    let tag = "restyled";
    let root = page_with_a_picture(tag, dark_ring());
    let app = App::new();
    app.loader().root(&root);
    let sand = Color::new(240, 220, 180, 255);
    let rules = Recipe::new().state(LinkPreviewState::Transparent, LinkPreviewPart::Media, move |s| {
      s.background(sand)
    });
    let mut stage = card_with(app, &format!("{tag}/page.html"), move |preview| preview.rules(&rules));
    until(&mut stage, "the plate in the theme's color", |stage| {
      !boxes(stage, sand, 360.0, 188.0).is_empty()
    });
    assert!(boxes(&stage, Color::new(255, 255, 255, 255), 360.0, 188.0).is_empty());
  }

  #[cfg(feature = "canvas")]
  #[test]
  fn an_svg_icon_alone_is_drawn() {
    let tag = "svg-only";
    let page = format!(
      r#"<head><title>Drawn</title><meta property="og:site_name" content="svg.example">
      <link rel="icon" type="image/svg+xml" href="{tag}/icon.svg"></head>"#
    );
    let root = site(
      tag,
      &[
        ("page.html", page.into_bytes()),
        ("icon.svg", SVG_ICON.as_bytes().to_vec()),
      ],
    );
    let app = App::new();
    app.loader().root(&root);
    let mut stage = card(app, &format!("{tag}/page.html"));
    until(&mut stage, "the icon", |stage| icon(stage).is_some());
    assert_eq!(
      pixels_of(&icon(&stage).unwrap()),
      (64, 64),
      "the drawing at the size an icon is decoded to"
    );
    assert!(!stage.paints(&link_mark()));
  }

  #[test]
  fn an_icon_that_fails_gives_way_to_the_next_and_the_link_mark_stands_for_them_all() {
    // The PNG is no picture and the SVG is: the SVG is drawn, where the build draws one.
    let tag = "next-icon";
    let root = site(
      tag,
      &[
        ("page.html", githubs_page(tag)),
        ("favicons/favicon.png", b"<html>not a picture</html>".to_vec()),
        ("favicons/favicon.svg", SVG_ICON.as_bytes().to_vec()),
      ],
    );
    let app = App::new();
    app.loader().root(&root);
    let mut stage = card(app, &format!("{tag}/page.html"));
    if cfg!(feature = "canvas") {
      until(&mut stage, "the SVG icon", |stage| icon(stage).is_some());
      assert_eq!(pixels_of(&icon(&stage).unwrap()), (64, 64));
      assert!(!stage.paints(&link_mark()));
    }

    // Neither loads: the link mark, and no box where the icon would be.
    let tag = "no-icon";
    let root = site(
      tag,
      &[
        ("page.html", githubs_page(tag)),
        ("favicons/favicon.svg", b"<svg broken".to_vec()),
      ],
    );
    let app = App::new();
    app.loader().root(&root);
    let mut stage = card(app, &format!("{tag}/page.html"));
    until(&mut stage, "the link mark", |stage| stage.paints(&link_mark()));
    for _ in 0..5 {
      stage.pass();
    }
    assert!(stage.paints(&link_mark()), "it stays");
    assert!(icon(&stage).is_none());
    assert!(stage.paints("GitHub - ivanzaida/lurq"));
  }
}

/// GitHub's server of the pictures of its pages answers "429 Too Many Requests" to a first request now and then,
/// and the picture to the next: the preview's picture comes all the same.
#[cfg(all(feature = "image", feature = "http"))]
#[test]
fn a_picture_whose_server_is_busy_at_first_comes_all_the_same() {
  use std::io::{Read, Write};

  use a_real_page::{PICTURE, card, dark_disc, githubs_page, icon, picture, png, until};

  let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
  let origin = format!("http://{}", listener.local_addr().unwrap());
  let page = String::from_utf8(githubs_page("TAG"))
    .unwrap()
    .replace("TAG/", &format!("{origin}/"))
    .into_bytes();
  let (picture_bytes, icon_bytes) = (png(120, 60, |_, _| [40, 160, 90, 255]), dark_disc());
  let asked_for_the_picture = Arc::new(AtomicUsize::new(0));
  let asked = asked_for_the_picture.clone();
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
      let request = String::from_utf8_lossy(&request).into_owned();
      let path = request.split_whitespace().nth(1).unwrap_or("/").to_owned();
      let busy: &[u8] = b"Too many requests, please try again later.";
      let (status, kind, body): (&str, &str, &[u8]) = if path == "/page" {
        ("200 OK", "text/html; charset=utf-8", &page)
      } else if path == format!("/{PICTURE}") {
        if asked.fetch_add(1, Ordering::SeqCst) == 0 {
          ("429 Too Many Requests", "text/html", busy)
        } else {
          ("200 OK", "image/png", &picture_bytes)
        }
      } else if path == "/favicons/favicon.png" {
        ("200 OK", "image/png", &icon_bytes)
      } else {
        ("404 Not Found", "text/plain", b"")
      };
      let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nRetry-After: 556\r\nConnection: \
         close\r\n\r\n",
        body.len()
      );
      let _ = stream.write_all(head.as_bytes()).and_then(|()| stream.write_all(body));
    }
  });
  let mut stage = card(lurq::app::App::new(), &format!("{origin}/page"));
  until(&mut stage, "the picture and the icon", |stage| {
    picture(stage).is_some() && icon(stage).is_some()
  });
  assert_eq!(asked_for_the_picture.load(Ordering::SeqCst), 2, "asked again once");
}
