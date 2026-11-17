//! A link preview: the board's card, row and inline form at rest, what it shows while the page is read, what a page
//! says and what the app gives over it, a page that tells nothing and one that could not be read, a video, the one
//! link it is to the pointer, the keyboard and tooling, the close of one in a composer, a text longer than its
//! room, a narrow room, and a theme's rule.

use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
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
  resources::{LinkFetch, LinkFetcher, LinkMeta, LinkPreviewError, LinkRequest, LoadWaker},
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

fn stage_of(preview: impl Fn() -> LinkPreview + Send + Sync + 'static) -> Stage {
  Stage::new(move |_| Column::new().padding(8.0).child(preview()).into())
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
  let (_, _, card, height) = stage.bounds("card");
  assert_eq!(card, 220.0, "a card is never wider than its room");
  assert!(height < 140.0, "two lines of title at most: {height}");
  let (_, _, inline, _) = stage.bounds("inline");
  assert!(inline <= 220.0, "{inline}");
  assert!(stage.cut_text_quad(long).is_some(), "cut with an ellipsis");
  assert!(stage.paints("lurq.dev"), "the site keeps its room in a sentence");
}

#[test]
fn the_first_frame_is_the_settled_one() {
  let scene = |_: &mut lurq::app::ctx::Ctx| {
    Column::new()
      .padding(8.0)
      .child(
        LinkPreview::new(URL)
          .fetcher(|_: &str| LinkFetch::ready(page()))
          .id("preview"),
      )
      .into()
  };
  let mut stage = Stage::first_frame(400.0, 300.0, scene);
  let first = stage.bounds("preview");
  assert!(stage.paints(TITLE));
  stage.frame();
  assert_eq!(stage.bounds("preview"), first);
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

#[cfg(feature = "markdown")]
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
