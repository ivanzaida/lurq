//! A link preview: what a link says about itself, shown before it is followed.

use std::{any::TypeId, sync::Arc};

use parking_lot::Mutex;

use super::{
  Icon, Picture, PictureFit, PicturePart, Skeleton, icons, names,
  part_style::{FontFamily, Length},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::{fading, pressable},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    component::Component,
    ctx::{Ctx, FocusHandle},
    events::MouseEvent,
    theme::{BorderSize, IconSize, OpacityLevel, PaletteColor, RadiusSize, ShadowStyle, Theme, TypographyStyle},
  },
  components::{self, Column, Row, Stack, TextOverflow},
  core::{DisplayWords, ElementRef, Locale, Signal},
  layout::{
    Alignment, StackAlignment,
    layout_kind::Justify,
    text_style::{FontWeight, TextUnderline},
  },
  node::{CursorIcon, Element, HitTestBehavior, border::Border, color::Color, dimension::Dimension},
  resources::{LinkFetch, LinkFetcher, LinkMeta, LinkPreviewError, LoadWaker, Source, fetch_off_thread, link_fetcher},
};

/// How many pixels across and down a site's icon is decoded to, or an SVG one drawn at: an icon of 16 on a screen
/// of up to four times the pixels.
const ICON_PIXELS: u32 = 64;

/// How much wider than high the picture of a card is: 360 by 188, the ratio of OpenGraph's pictures.
pub const LINK_PREVIEW_PICTURE_RATIO: f32 = 360.0 / 188.0;
const CARD_PICTURE_RATIO: f32 = LINK_PREVIEW_PICTURE_RATIO;

/// How much wider than high, or higher than wide, a picture may be and still be cut to the square of a row: one
/// further from square is drawn whole in it. GitHub's card of a repository, 2 : 1, cut to the square showed the
/// middle of its name.
const SQUARE_ENOUGH: f32 = 1.5;

/// The forms of a link preview.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LinkPreviewVariant {
  /// A card of 360: the picture over the site, the title and a line of the description.
  #[default]
  Card,
  /// A row of 432 for lists and messages: the text beside a picture of 72.
  Compact,
  /// 24 high, in a sentence: the site's icon, the title and the site.
  Inline,
}

names!(LinkPreviewVariant {
  Card => "card",
  Compact => "compact",
  Inline => "inline",
});

/// The parts of a link preview a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LinkPreviewPart {
  /// The preview: one link, what is pressed.
  Root,
  /// The box of the picture: the top of a card, the square at the end of a compact one.
  Media,
  /// The mark of a video in the middle of its picture.
  Play,
  /// The glyph in the mark.
  PlayIcon,
  /// The length of a video, at the lower end of its picture.
  Length,
  /// The column of the site, the title and the description.
  Text,
  /// The row of the site's icon and its name.
  Site,
  /// The site's icon, or the link mark where it has none.
  Icon,
  /// The name of the site, or its domain.
  Domain,
  Title,
  Description,
  /// The lines that stand for the text while the page is read.
  Skeleton,
  /// The button that takes the preview out of what is being written.
  Close,
  /// The glyph of the close button.
  CloseIcon,
}

names!(LinkPreviewPart {
  Root => "root",
  Media => "media",
  Play => "play",
  PlayIcon => "play_icon",
  Length => "length",
  Text => "text",
  Site => "site",
  Icon => "icon",
  Domain => "domain",
  Title => "title",
  Description => "description",
  Skeleton => "skeleton",
  Close => "close",
  CloseIcon => "close_icon",
});

/// The states of a link preview a recipe styles, besides what the pointer and the keyboard do to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LinkPreviewState {
  /// The page is being read: the skeleton stands for what is not known yet.
  Loading,
  /// It shows a picture.
  Media,
  /// It is a video: the play mark and the length over its picture.
  Video,
  /// The page told nothing: the address, the link mark and a line that says so.
  Empty,
  /// It has a close button: it is in what is being written.
  Removable,
  /// It shows the site's own icon, on the white plate favicons are drawn for, and not the link mark.
  SiteIcon,
  /// Its picture shows what is behind it, as a logo drawn on nothing does (`ImageData::has_transparency`): it
  /// stands whole on a light plate, the ground it was drawn for.
  Transparent,
  /// It takes no input.
  Disabled,
}

names!(LinkPreviewState {
  Loading => "loading",
  Media => "media",
  Video => "video",
  Empty => "empty",
  Removable => "removable",
  SiteIcon => "site_icon",
  Transparent => "transparent",
  Disabled => "disabled",
});

/// A picture a preview is given: an address or a source to read, or a [`Picture`] of the app's.
#[derive(Clone)]
pub enum LinkImage {
  Source(Source),
  Picture(Box<Picture>),
}

impl From<&str> for LinkImage {
  fn from(address: &str) -> Self {
    Self::Source(Source::resource(address))
  }
}

impl From<String> for LinkImage {
  fn from(address: String) -> Self {
    Self::from(address.as_str())
  }
}

impl From<Source> for LinkImage {
  fn from(source: Source) -> Self {
    Self::Source(source)
  }
}

impl From<Picture> for LinkImage {
  fn from(picture: Picture) -> Self {
    Self::Picture(Box::new(picture))
  }
}

impl LinkImage {
  /// The picture, and the key that tells it from another.
  fn picture(&self) -> (Picture, u64) {
    match self {
      Self::Source(source) => (Picture::from_source(source.clone()), source.cache_key()),
      Self::Picture(picture) => ((**picture).clone(), picture_key(picture)),
    }
  }
}

fn picture_key(picture: &Picture) -> u64 {
  picture.source_key().unwrap_or(0)
}

/// A field a preview is given, or told it has none of, or left to what is fetched.
#[derive(Clone)]
enum Given<T> {
  Fetched,
  None,
  Is(T),
}

impl<T> Given<T> {
  fn decided(&self) -> bool {
    !matches!(self, Self::Fetched)
  }
}

type OpenHandler = Arc<dyn Fn(&str) + Send + Sync>;
type RemoveHandler = Arc<dyn Fn() + Send + Sync>;

/// What builds a link preview's nodes: a theme's in place of the default one.
pub type LinkPreviewRenderer = Arc<dyn Fn(LinkPreviewView) -> Element + Send + Sync>;

/// A link preview: what a page says about itself, shown before its link is followed. The site with its icon, the
/// title, a line of the description and the picture, as one link.
///
/// ```
/// use lurq::ui::{LinkPreview, LinkPreviewVariant};
///
/// // What the page says, fetched.
/// let docs = LinkPreview::new("https://lurq.dev/docs/layout");
/// // The app's own title over the page's; the rest is fetched.
/// let labelled = LinkPreview::new("https://lurq.dev/docs/layout").title("Layout, the guide");
/// // Everything given: nothing is fetched.
/// let known = LinkPreview::new("https://lurq.dev/docs/layout")
///   .title("Layout in lurq")
///   .description("Rows, columns and the space between.")
///   .site("lurq.dev")
///   .no_image()
///   .no_icon()
///   .variant(LinkPreviewVariant::Compact);
/// // In a message being written: a close drops it.
/// let draft = LinkPreview::new("https://lurq.dev").on_remove(|| println!("dropped"));
/// ```
///
/// It works from the address alone: it asks a [`LinkFetcher`] for what the page says about itself
/// ([`LinkMeta`]) off the UI thread, shows its skeleton until the fetcher answers and while the page is read, and
/// then the page's title, description, site, icon and picture. Each of them the app gives ([`title`](Self::title),
/// [`description`](Self::description), [`site`](Self::site), [`image`](Self::image), [`icon`](Self::icon),
/// [`video`](Self::video)) or says there is none of ([`no_image`](Self::no_image), [`no_icon`](Self::no_icon)) is shown
/// in place of what is fetched; where everything it shows is given, nothing is fetched. The fetcher is lurq's own
/// ([`PageFetcher`](crate::resources::PageFetcher), feature `http`), or the app's
/// ([`fetcher`](Self::fetcher), [`set_link_fetcher`](crate::resources::set_link_fetcher)). A page that tells
/// nothing, one that could not be read, and every page in a build without `http`, shows the address, the link
/// mark and a line that says so, under what the app gave.
///
/// The whole preview is one link, a stop of Tab, named by its title to tooling and described by its site and
/// description. A click, Enter or Space open the address in the browser ([`open_url`](crate::app::open_url()));
/// an app that decides itself gives [`on_open`](Self::on_open). Given [`on_remove`](Self::on_remove), it has a
/// close button, a stop of Tab of its own, which drops it from what is being written.
pub struct LinkPreview {
  url: Arc<str>,
  title: Option<Arc<str>>,
  description: Option<Arc<str>>,
  site: Option<Arc<str>>,
  site_shown: bool,
  image: Given<LinkImage>,
  icon: Given<LinkImage>,
  video: Option<Option<f64>>,
  variant: Name,
  width: Option<Length>,
  loading: bool,
  disabled: bool,
  label: Option<Arc<str>>,
  fetcher: Option<(Arc<dyn LinkFetcher>, FetcherId)>,
  words: Option<DisplayWords>,
  no_preview_text: Option<Arc<str>>,
  remove_label: Option<Arc<str>>,
  on_open: Vec<OpenHandler>,
  on_remove: Vec<RemoveHandler>,
  return_focus: Option<ElementRef>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Clone for LinkPreview {
  fn clone(&self) -> Self {
    Self {
      url: self.url.clone(),
      title: self.title.clone(),
      description: self.description.clone(),
      site: self.site.clone(),
      site_shown: self.site_shown,
      image: self.image.clone(),
      icon: self.icon.clone(),
      video: self.video,
      variant: self.variant,
      width: self.width,
      loading: self.loading,
      disabled: self.disabled,
      label: self.label.clone(),
      fetcher: self.fetcher.clone(),
      words: self.words.clone(),
      no_preview_text: self.no_preview_text.clone(),
      remove_label: self.remove_label.clone(),
      on_open: self.on_open.clone(),
      on_remove: self.on_remove.clone(),
      return_focus: self.return_focus.clone(),
      states: self.states.clone(),
      rules: self.rules.clone(),
      forced: self.forced,
      id: self.id.clone(),
      key: self.key.clone(),
    }
  }
}

impl LinkPreview {
  /// A preview of the page at `url`, which shows what the page says about itself.
  pub fn new(url: &str) -> Self {
    Self {
      url: Arc::from(url.trim()),
      title: None,
      description: None,
      site: None,
      site_shown: true,
      image: Given::Fetched,
      icon: Given::Fetched,
      video: None,
      variant: LinkPreviewVariant::default().into(),
      width: None,
      loading: false,
      disabled: false,
      label: None,
      fetcher: None,
      words: None,
      no_preview_text: None,
      remove_label: None,
      on_open: Vec::new(),
      on_remove: Vec::new(),
      return_focus: None,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// The title, in place of the page's. It is also what the preview is called to tooling.
  pub fn title(mut self, title: &str) -> Self {
    self.title = Some(Arc::from(title));
    self
  }

  /// A line or two about the page, in place of the page's.
  pub fn description(mut self, description: &str) -> Self {
    self.description = Some(Arc::from(description));
    self
  }

  /// The name of the site, in place of the page's: "lurq.dev". Where neither says one, it is the domain of the
  /// address.
  pub fn site(mut self, site: &str) -> Self {
    self.site = Some(Arc::from(site));
    self
  }

  /// The picture, in place of the page's: an address, a [`Source`], or a [`Picture`] of the app's.
  pub fn image(mut self, image: impl Into<LinkImage>) -> Self {
    self.image = Given::Is(image.into());
    self
  }

  /// No picture, whatever the page has: the text alone.
  pub fn no_image(mut self) -> Self {
    self.image = Given::None;
    self
  }

  /// No name of the site beside its icon: the title alone in a sentence. Tooling still reads it.
  pub fn no_site(mut self) -> Self {
    self.site_shown = false;
    self
  }

  /// The site's icon, in place of the page's: an address, a [`Source`], or a [`Picture`].
  pub fn icon(mut self, icon: impl Into<LinkImage>) -> Self {
    self.icon = Given::Is(icon.into());
    self
  }

  /// No icon of the site's, whatever the page has: the link mark.
  pub fn no_icon(mut self) -> Self {
    self.icon = Given::None;
    self
  }

  /// The page is a video, `length` seconds long where that is known: the play mark and the length over its
  /// picture.
  pub fn video(mut self, length: impl Into<Option<f64>>) -> Self {
    self.video = Some(length.into());
    self
  }

  /// A [`LinkPreviewVariant`], or a form the theme's recipe names.
  pub fn variant(mut self, variant: impl Into<Name>) -> Self {
    self.variant = variant.into();
    self
  }

  /// A width of its own in place of the form's: a number of logical pixels, or another [`Length`]. It is never
  /// wider than its room.
  pub fn width(mut self, width: impl Into<Length>) -> Self {
    self.width = Some(width.into());
    self
  }

  /// Shows the skeleton whatever is known: for an app that reads the page itself and has nothing to give yet.
  pub fn loading(mut self, loading: bool) -> Self {
    self.loading = loading;
    self
  }

  /// Whether the preview takes no input: it is dimmed, is no stop of Tab, and opens nothing.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// What the preview is called to tooling, in place of its title.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// What fetches what the page says, for this preview: in place of the app's
  /// ([`set_link_fetcher`](crate::resources::set_link_fetcher)) and of lurq's own. Any `Fn(&str) -> LinkFetch`.
  /// It is asked off the UI thread, and may block: the preview is its skeleton until it answers. It is asked once
  /// for an address, and again when it wakes the preview; a render that gives a fetcher of the same type asks it
  /// nothing anew.
  pub fn fetcher(mut self, fetcher: impl LinkFetcher + 'static) -> Self {
    let id = FetcherId::of(&fetcher);
    self.fetcher = Some((Arc::new(fetcher), id));
    self
  }

  /// The same, shared with other previews.
  pub fn shared_fetcher(mut self, fetcher: Arc<dyn LinkFetcher>) -> Self {
    let id = FetcherId::shared(&fetcher);
    self.fetcher = Some((fetcher, id));
    self
  }

  /// The locale its words are taken from, in place of the app's.
  pub fn locale(mut self, locale: &Locale) -> Self {
    self.words = Some(locale.display.clone());
    self
  }

  /// The line under the address of a page that told nothing: "No preview for this page." unless told.
  pub fn no_preview_text(mut self, text: &str) -> Self {
    self.no_preview_text = Some(Arc::from(text));
    self
  }

  /// The name of the close button: "Remove preview" unless told.
  pub fn remove_label(mut self, label: &str) -> Self {
    self.remove_label = Some(Arc::from(label));
    self
  }

  /// Called with the address on a click, Enter or Space, in place of opening it in the browser: the app
  /// decides where it goes.
  pub fn on_open(mut self, handler: impl Fn(&str) + Send + Sync + 'static) -> Self {
    self.on_open.push(Arc::new(handler));
    self
  }

  /// Called when the close button is pressed: the preview has one once this is given. The app takes the preview
  /// out of what it shows, and puts the focus where the person goes on writing.
  pub fn on_remove(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_remove.push(Arc::new(handler));
    self
  }

  /// Where the focus goes once the close button is pressed and the app has taken the preview out: `element`.
  /// Without one it goes on to what follows the preview in the order of Tab, or to what comes before it where
  /// nothing follows, and is never left to nothing.
  pub fn return_focus(mut self, element: &ElementRef) -> Self {
    self.return_focus = Some(element.clone());
    self
  }

  /// Puts the preview in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this preview over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this preview.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the preview as it is under the pointer, held down or focused from the keyboard, with nothing pointing
  /// at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the link; the close button is `{id}-remove`.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// The address it previews.
  pub fn url(&self) -> &str {
    &self.url
  }

  /// Whether it shows nothing it was not given: the title, the description and the site given, and the picture
  /// and the icon given or said to be none (an inline one shows no description and no picture).
  pub fn needs_nothing_fetched(&self) -> bool {
    let inline = self.variant == Name::from(LinkPreviewVariant::Inline);
    self.title.is_some()
      && (self.site.is_some() || !self.site_shown)
      && self.icon.decided()
      && (inline || (self.description.is_some() && self.image.decided()))
  }

  /// The preview mounted in `ctx`: what converting it into an element does once the render that returns it has
  /// returned.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let key = self.key.clone();
    let props = LinkPreviewProps(self);
    match key {
      Some(key) => ctx.mount_keyed::<LinkPreviewHost>(&key, props),
      None => ctx.mount::<LinkPreviewHost>(props),
    }
  }

  /// Builds the preview in `theme` as it is with nothing fetched: what it was given, and the address for the
  /// rest. The still picture of it, for a test; converted into an element it fetches.
  pub fn build(self, theme: &Theme) -> Element {
    let fetched = if self.needs_nothing_fetched() {
      None
    } else {
      Some(LinkFetch::ready(LinkMeta::new(&self.url)))
    };
    self.view(theme, fetched, None).render_themed()
  }

  fn view(self, theme: &Theme, fetched: Option<LinkFetch>, live: Option<&Live>) -> LinkPreviewView {
    let words = self
      .words
      .clone()
      .unwrap_or_else(|| super::current_locale().display.clone());
    let variant = LinkPreviewVariant::ALL
      .into_iter()
      .find(|variant| Name::from(*variant) == self.variant)
      .unwrap_or_default();
    let (meta, failure, pending) = match fetched {
      None => (None, None, false),
      Some(LinkFetch::Loading) => (None, None, true),
      Some(LinkFetch::Ready(meta)) => (Some(meta), None, false),
      Some(LinkFetch::Failed(error)) => (None, Some(error), false),
    };
    let meta = meta.as_deref();
    let title = self.title.clone().or_else(|| meta.and_then(|meta| meta.title.clone()));
    let description = self
      .description
      .clone()
      .or_else(|| meta.and_then(|meta| meta.description.clone()));
    let site = self
      .site
      .clone()
      .or_else(|| meta.and_then(|meta| meta.site.clone()))
      .unwrap_or_else(|| Arc::from(domain_of(&self.url)));
    let failed = |key: u64, which: Option<&Signal<Option<u64>>>| which.is_some_and(|failed| failed.get() == Some(key));
    let pick = |given: &Given<LinkImage>, fetched: Option<&Arc<str>>| -> Option<(Picture, u64)> {
      match given {
        Given::None => None,
        Given::Is(image) => Some(image.picture()),
        Given::Fetched => fetched.map(|address| LinkImage::from(&**address).picture()),
      }
    };
    let image = pick(&self.image, meta.and_then(|meta| meta.image.as_ref()))
      .filter(|(_, key)| !failed(*key, live.map(|live| &live.failed_image)));
    // The first of the site's icons that has not failed: the next is tried where one does not load.
    let failed_icons = live.map(|live| live.failed_icon.get()).unwrap_or_default();
    let icon = match &self.icon {
      Given::None => None,
      Given::Is(icon) => Some(icon.picture()),
      Given::Fetched => self
        .page_icons(meta, live.is_some())
        .iter()
        .map(|address| {
          let (picture, key) = LinkImage::from(&**address).picture();
          // An icon is drawn at 16: what is decoded of it is small, and an SVG is drawn at this size.
          (picture.downscale_to(ICON_PIXELS, ICON_PIXELS), key)
        })
        .find(|(_, key)| !failed_icons.contains(key)),
    }
    .filter(|(_, key)| !failed_icons.contains(key));
    let video = self
      .video
      .or_else(|| meta.filter(|meta| meta.video).map(|meta| meta.video_length));
    let loading = self.loading || (pending && self.title.is_none());
    let empty = !loading && title.is_none() && description.is_none() && image.is_none();
    let fallback = match &failure {
      Some(error) => words.link_error(error),
      None => self
        .no_preview_text
        .clone()
        .unwrap_or_else(|| words.link_no_preview.clone()),
    };
    let (title, description) = if empty {
      (Arc::from(address_of(&self.url)), Some(fallback))
    } else {
      (title.unwrap_or_else(|| Arc::from(address_of(&self.url))), description)
    };
    let shown_image = if loading || empty || variant == LinkPreviewVariant::Inline {
      None
    } else {
      image
    };
    // Whether the picture shows what is behind it, and how wide it is, once its pixels are there: until then it
    // is drawn as any is, and their coming renders the preview again to look.
    let looked = shown_image.as_ref().and_then(|(picture, _)| looked_at(picture, live));
    let transparent = looked.is_some_and(|(transparent, _)| transparent);
    let far_from_square = looked.is_some_and(|(_, ratio)| !(1.0 / SQUARE_ENOUGH..=SQUARE_ENOUGH).contains(&ratio));
    let whole = transparent || (variant == LinkPreviewVariant::Compact && far_from_square);
    let shown_image = shown_image.map(|(picture, key)| match live {
      Some(live) if looked.is_none() => {
        let tick = live.tick.clone();
        (
          picture.on_load(move || tick.set(tick.get_untracked().wrapping_add(1))),
          key,
        )
      }
      _ => (picture, key),
    });
    let removable = !self.on_remove.is_empty();
    let case = Case {
      variant: Some(self.variant),
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(LinkPreviewState::Loading, loading)
    .state(
      LinkPreviewState::Media,
      shown_image.is_some() || (loading && variant != LinkPreviewVariant::Inline),
    )
    .state(LinkPreviewState::Video, video.is_some() && shown_image.is_some())
    .state(LinkPreviewState::Empty, empty)
    .state(LinkPreviewState::Removable, removable)
    .state(LinkPreviewState::SiteIcon, icon.is_some() && !empty && !loading)
    .state(LinkPreviewState::Transparent, transparent)
    .state(LinkPreviewState::Disabled, self.disabled);
    let mut rules = self.rules.clone();
    if let Some(width) = self.width {
      rules = Recipe::new()
        .base(LinkPreviewPart::Root, |s| s.width(width))
        .extend(&rules);
    }
    LinkPreviewView {
      url: self.url.clone(),
      variant,
      title,
      description,
      site,
      site_shown: self.site_shown,
      image: shown_image.map(|(picture, _)| picture),
      transparent,
      whole,
      icon: if empty {
        None
      } else {
        icon.clone().map(|(picture, _)| picture)
      },
      video: if empty { None } else { video },
      loading,
      empty,
      error: failure,
      disabled: self.disabled,
      label: self.label.clone(),
      remove_label: self.remove_label.clone().unwrap_or_else(|| words.link_remove.clone()),
      words,
      recipe: theme.resolve::<LinkPreview>(&case, Some(&rules)),
      forced: self.forced,
      hovered: live.map(|live| live.hovered.clone()),
      failed_image: live.map(|live| live.failed_image.clone()),
      failed_icon: live.map(|live| live.failed_icon.clone()),
      focus: live.map(|live| live.focus.clone()),
      return_focus: self.return_focus.clone(),
      image_key: self.image_key(meta),
      icon_key: icon.as_ref().map_or(0, |(_, key)| *key),
      on_open: self.on_open,
      on_remove: self.on_remove,
      id: self.id,
      key: self.key,
    }
  }

  fn image_key(&self, meta: Option<&LinkMeta>) -> u64 {
    match &self.image {
      Given::Is(image) => image.picture().1,
      Given::Fetched => meta
        .and_then(|meta| meta.image.as_ref())
        .map_or(0, |address| Source::resource(address).cache_key()),
      Given::None => 0,
    }
  }

  /// The site's icons the page names, the best first ([`LinkMeta::icons`]), and after them the icon a site keeps
  /// at `/favicon.ico`: read through the app's loader as the rest is, each tried where the one before it did not
  /// load, and the link mark where none does. A preview with nothing fetched, and one in no app, asks for none
  /// at `/favicon.ico`.
  fn page_icons(&self, meta: Option<&LinkMeta>, in_an_app: bool) -> Vec<Arc<str>> {
    let Some(meta) = meta else {
      return Vec::new();
    };
    let mut icons = if meta.icons.is_empty() {
      meta.icon.iter().cloned().collect()
    } else {
      meta.icons.clone()
    };
    if in_an_app
      && let Some(favicon) = favicon_of(&meta.url)
      && !icons.contains(&favicon)
    {
      icons.push(favicon);
    }
    icons
  }
}

impl LinkPreviewVariant {
  const ALL: [Self; 3] = [Self::Card, Self::Compact, Self::Inline];
}

/// Where the pixels of `picture` are there: whether they show what is behind them, and how much wider than high
/// they are. Whether they show through is read once for them and kept by a preview in an app, since an opaque
/// picture has each of its pixels looked at.
#[cfg(feature = "raster")]
fn looked_at(picture: &Picture, live: Option<&Live>) -> Option<(bool, f32)> {
  let pixels = picture.pixels()?;
  let ratio = pixels.width() as f32 / pixels.height().max(1) as f32;
  let Some(live) = live else {
    return Some((pixels.has_transparency(), ratio));
  };
  let mut seen = live.seen.lock();
  let transparent = match *seen {
    Some((id, said)) if id == pixels.id() => said,
    _ => {
      let said = pixels.has_transparency();
      *seen = Some((pixels.id(), said));
      said
    }
  };
  Some((transparent, ratio))
}

#[cfg(not(feature = "raster"))]
fn looked_at(_: &Picture, _: Option<&Live>) -> Option<(bool, f32)> {
  None
}

/// The icon a site keeps where its pages name none: `/favicon.ico` at the origin of `url`, an address on the web.
fn favicon_of(url: &str) -> Option<Arc<str>> {
  let (scheme, after) = url.split_once("://")?;
  if !scheme.eq_ignore_ascii_case("http") && !scheme.eq_ignore_ascii_case("https") {
    return None;
  }
  let host = after.split(['/', '?', '#']).next().filter(|host| !host.is_empty())?;
  Some(Arc::from(format!("{scheme}://{host}/favicon.ico")))
}

/// The domain of `url`, as a site is named where nothing names it: its host, without `www.`.
fn domain_of(url: &str) -> String {
  let after = url.split_once("://").map_or(url, |(_, after)| after);
  let host = after.split(['/', '?', '#']).next().unwrap_or(after);
  let host = host.rsplit('@').next().unwrap_or(host);
  let host = host.split(':').next().unwrap_or(host);
  host.strip_prefix("www.").unwrap_or(host).to_owned()
}

/// `url` as a preview writes it where the page told nothing: without its scheme, `www.` and a last `/`.
fn address_of(url: &str) -> String {
  let after = url.split_once("://").map_or(url, |(_, after)| after);
  let after = after.strip_prefix("www.").unwrap_or(after);
  after.strip_suffix('/').unwrap_or(after).to_owned()
}

/// A preview in a tree is its component, declared: the component that renders the tree mounts it.
impl From<LinkPreview> for Element {
  fn from(preview: LinkPreview) -> Self {
    let key = preview.key.clone();
    let props = LinkPreviewProps(preview);
    match key {
      Some(key) => Element::component_keyed::<LinkPreviewHost>(key, props),
      None => Element::component::<LinkPreviewHost>(props),
    }
  }
}

/// Which fetcher a preview asks: one of a type, made anew by every render that gives it, or one shared, by where
/// it is. A preview given another asks it anew.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FetcherId {
  Type(TypeId),
  Shared(usize),
}

impl FetcherId {
  fn of<F: 'static>(_: &F) -> Self {
    Self::Type(TypeId::of::<F>())
  }

  fn shared(fetcher: &Arc<dyn LinkFetcher>) -> Self {
    Self::Shared(Arc::as_ptr(fetcher).cast::<()>() as usize)
  }
}

/// What a preview asked its fetcher, and what it was told.
#[derive(Default)]
struct Asked {
  /// The address and the fetcher asked.
  of: Option<(Arc<str>, FetcherId)>,
  /// What the fetcher last said: none before it first answered.
  answer: Option<LinkFetch>,
  /// It is being asked, off the UI thread.
  asking: bool,
  /// The fetcher woke the preview since it was asked: it is asked again.
  again: bool,
  /// Which asking this is: what an asking of another address or fetcher answers is let fall.
  round: u64,
}

/// A preview to mount.
#[derive(Clone)]
struct LinkPreviewProps(LinkPreview);

/// A preview is built anew by every render of what holds it, with handlers nothing compares: it renders again
/// each time.
impl PartialEq for LinkPreviewProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

#[cfg(feature = "devtools")]
impl crate::app::component::DevtoolsInspectable for LinkPreviewProps {}

/// What a preview keeps between renders.
#[derive(Clone)]
struct Live {
  /// Counts on when the fetcher answers or wakes it: the preview renders again and looks.
  tick: Signal<u64>,
  /// What it asked its fetcher and what it was told: written from the thread it asks on.
  asked: Arc<Mutex<Asked>>,
  /// Whether the pointer is on it: its title is underlined.
  hovered: Signal<bool>,
  /// The key of a picture that did not load: the preview is drawn without it.
  failed_image: Signal<Option<u64>>,
  /// The keys of the icons that did not load: the next is tried, and the link mark stands for them all.
  failed_icon: Signal<Vec<u64>>,
  /// Whether the picture's pixels show what is behind them, by the id of the pixels: read once for them.
  seen: Arc<Mutex<Option<(u64, bool)>>>,
  /// What moves the focus once the close button took the preview out.
  focus: FocusHandle,
}

/// The component of a preview.
struct LinkPreviewHost {
  live: Live,
}

impl Component for LinkPreviewHost {
  type Props = LinkPreviewProps;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      live: Live {
        tick: ctx.signal(0u64),
        asked: Arc::new(Mutex::new(Asked::default())),
        hovered: ctx.signal(false),
        failed_image: ctx.signal(None),
        failed_icon: ctx.signal(Vec::new()),
        seen: Arc::default(),
        focus: ctx.focus_handle(),
      },
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let preview = ctx.props::<LinkPreviewProps>().0.clone();
    // Read, so that a fetch that comes further renders the preview again.
    self.live.tick.get();
    let fetched = if preview.needs_nothing_fetched() {
      None
    } else {
      let (fetcher, id) = preview.fetcher.clone().unwrap_or_else(|| {
        let fetcher = link_fetcher();
        let id = FetcherId::shared(&fetcher);
        (fetcher, id)
      });
      Some(self.fetched(ctx, &preview.url, fetcher, id))
    };
    let theme = current_theme();
    preview.view(&theme, fetched, Some(&self.live)).render_themed()
  }
}

impl LinkPreviewHost {
  /// What the fetcher said of `url`, or that it is on its way. It is asked off the UI thread, once for an address
  /// and a fetcher and again each time it wakes the preview, so that a fetcher that blocks holds no frame: until
  /// it answers, the preview is its skeleton. What it already holds is taken at once.
  fn fetched(&self, ctx: &Ctx, url: &Arc<str>, fetcher: Arc<dyn LinkFetcher>, id: FetcherId) -> LinkFetch {
    let mut asked = self.live.asked.lock();
    let anew = asked.of.as_ref().is_none_or(|(of, of_id)| of != url || *of_id != id);
    if anew {
      *asked = Asked {
        of: Some((url.clone(), id)),
        round: asked.round.wrapping_add(1),
        ..Asked::default()
      };
    }
    if anew || (asked.again && !asked.asking) {
      // What the fetcher holds is had at once, and shown in this frame.
      if let Some(held) = fetcher.held(url).filter(|held| *held != LinkFetch::Loading) {
        asked.answer = Some(held.clone());
        asked.again = false;
        return held;
      }
      asked.asking = true;
      asked.again = false;
      let round = asked.round;
      let bump = {
        let tick = self.live.tick.clone();
        move || tick.set(tick.get_untracked().wrapping_add(1))
      };
      let wake: LoadWaker = {
        let (state, bump) = (self.live.asked.clone(), bump.clone());
        Arc::new(move || {
          {
            let mut asked = state.lock();
            if asked.round != round {
              return;
            }
            asked.again = true;
          }
          bump();
        })
      };
      let state = self.live.asked.clone();
      let loader = ctx.app_handle().map(|app| app.loader().clone());
      fetch_off_thread(fetcher, url.clone(), loader, wake, move |answer| {
        let told = {
          let mut asked = state.lock();
          if asked.round != round {
            return;
          }
          let changed = asked.answer.as_ref() != Some(&answer);
          asked.answer = Some(answer);
          asked.asking = false;
          changed || asked.again
        };
        if told {
          bump();
        }
      });
    }
    asked.answer.clone().unwrap_or(LinkFetch::Loading)
  }
}

/// A link preview as a renderer sees it: what it shows, the style of its parts, and its behavior ready to be
/// dressed.
pub struct LinkPreviewView {
  pub url: Arc<str>,
  pub variant: LinkPreviewVariant,
  /// The title: the given one, the page's, or the address where the page told nothing.
  pub title: Arc<str>,
  /// The description: the given one, the page's, or the line that says the page told nothing.
  pub description: Option<Arc<str>>,
  /// The name of the site, or its domain.
  pub site: Arc<str>,
  /// Whether the name of the site is drawn beside its icon.
  pub site_shown: bool,
  /// The picture, where it shows one.
  pub image: Option<Picture>,
  /// The picture shows what is behind it (state `transparent`): it stands whole on the media's plate, and is
  /// not cut to the box.
  pub transparent: bool,
  /// The picture is drawn whole in its box, not cut to it: one that shows what is behind it, and in a row one
  /// much wider or higher than square, which the picture's own box shows beside.
  pub whole: bool,
  /// The site's icon; none is the link mark.
  pub icon: Option<Picture>,
  /// The length of a video in seconds, where it is one: `Some(None)` where its length is not known.
  pub video: Option<Option<f64>>,
  /// The page is being read.
  pub loading: bool,
  /// The page told nothing, or could not be read.
  pub empty: bool,
  /// Why the page could not be read, where it could not.
  pub error: Option<LinkPreviewError>,
  pub disabled: bool,
  pub label: Option<Arc<str>>,
  pub remove_label: Arc<str>,
  /// The words of its locale.
  pub words: DisplayWords,
  /// The style of each part of this preview.
  pub recipe: ResolvedRecipe,
  forced: Option<Interaction>,
  hovered: Option<Signal<bool>>,
  failed_image: Option<Signal<Option<u64>>>,
  failed_icon: Option<Signal<Vec<u64>>>,
  focus: Option<FocusHandle>,
  return_focus: Option<ElementRef>,
  image_key: u64,
  icon_key: u64,
  on_open: Vec<OpenHandler>,
  on_remove: Vec<RemoveHandler>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl LinkPreviewView {
  /// The style of a part, as this preview is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// Whether the title is underlined: under the pointer, or drawn as if it were.
  pub fn underlined(&self) -> bool {
    self.forced == Some(Interaction::Hovered) || self.hovered.as_ref().is_some_and(|hovered| hovered.get())
  }

  /// What the preview is called to tooling: its label, or its title.
  pub fn name(&self) -> Arc<str> {
    self.label.clone().unwrap_or_else(|| self.title.clone())
  }

  /// What a selection over it copies where it stands in a sentence: `[title](url)`, or the address alone where
  /// the page told nothing.
  pub fn copy_text(&self) -> String {
    if self.empty {
      self.url.to_string()
    } else {
      let title = self.title.replace('[', "\\[").replace(']', "\\]");
      format!("[{title}]({})", self.url)
    }
  }

  /// What tooling reads of it after its name: the site, the description, and that it is a video.
  pub fn tooling_description(&self) -> String {
    let mut said: Vec<String> = vec![self.site.to_string()];
    if let Some(description) = &self.description {
      said.push(description.to_string());
    }
    if let Some(length) = self.video {
      said.push(self.words.link_video_text(length));
    }
    said.join(" · ")
  }

  /// The site's icon at the side the recipe gives it: the picture on its plate, or the link mark. The picture is
  /// its pixels alone, with no box of a picture's own around them: what of an icon is not drawn shows the plate.
  /// One that does not load is let go, and the next of the site's icons is tried in its place.
  pub fn icon(&self) -> Element {
    let part = self.part(LinkPreviewPart::Icon);
    let side = part.rest().width.unwrap_or(Length::Icon(IconSize::Sm));
    match &self.icon {
      Some(picture) => {
        let mut picture = picture
          .clone()
          .width(side)
          .height(side)
          .decorative(true)
          .bare(true)
          .style(PicturePart::Root, |s| s.radius(RadiusSize::Sm));
        if let Some(failed) = &self.failed_icon {
          let (failed, key) = (failed.clone(), self.icon_key);
          picture = picture.on_error(move |_| {
            failed.update(|failed| {
              if !failed.contains(&key) {
                failed.push(key);
              }
            })
          });
        }
        Row::new().part(&part).child(picture).into()
      }
      None => Row::new()
        .part(&part)
        .justify(Justify::Center)
        .align_items(Alignment::Center)
        .child(Icon::new(icons::LINK).size(side).build(self.theme()))
        .into(),
    }
  }

  /// The row of the site's icon and its name.
  pub fn site(&self) -> Row {
    let part = self.part(LinkPreviewPart::Domain);
    let row = Row::new()
      .part(&self.part(LinkPreviewPart::Site))
      .align_items(Alignment::Center)
      .min_width(0.0)
      .child(self.icon());
    if !self.site_shown {
      return row;
    }
    row.child(
      Row::new().flex_shrink(1.0).min_width(0.0).child(
        part
          .text(&self.site)
          .nowrap()
          .text_overflow(TextOverflow::Elipsis)
          .part(&part),
      ),
    )
  }

  /// The title, underlined under the pointer: as many lines as its part allows, the last cut with an ellipsis.
  pub fn title(&self) -> Element {
    let part = self.part(LinkPreviewPart::Title);
    let mut text = part.text(&self.title);
    if self.variant == LinkPreviewVariant::Inline {
      text = text.nowrap().text_overflow(TextOverflow::Elipsis);
    } else {
      text = text
        .width(Dimension::Pct(100.0))
        .max_lines(self.lines(LinkPreviewPart::Title));
    }
    if self.underlined() {
      text = text.underline(TextUnderline::new());
    }
    let holder = if self.variant == LinkPreviewVariant::Inline {
      Row::new().flex_shrink(1.0).min_width(0.0)
    } else {
      Row::new().width(Dimension::Pct(100.0))
    };
    holder.child(text.part(&part)).into()
  }

  /// How many lines a text part has before it ends in an ellipsis: two in a card, one in a row.
  fn lines(&self, part: LinkPreviewPart) -> u32 {
    match (self.variant, part) {
      (LinkPreviewVariant::Card, _) => 2,
      _ => 1,
    }
  }

  /// The description, where it has one.
  pub fn description(&self) -> Option<Element> {
    let description = self.description.as_ref()?;
    let part = self.part(LinkPreviewPart::Description);
    Some(
      Row::new()
        .width(Dimension::Pct(100.0))
        .child(
          part
            .text(description)
            .width(Dimension::Pct(100.0))
            .max_lines(self.lines(LinkPreviewPart::Description))
            .part(&part),
        )
        .into(),
    )
  }

  /// The lines that stand for the site, the title and the description while the page is read.
  pub fn skeleton(&self) -> Column {
    let part = self.part(LinkPreviewPart::Skeleton);
    // A line of the board's width, or of the room where there is less: a skeleton keeps the width it is given.
    let line = |width: Length, height: f32| -> Element {
      let room = match width {
        Length::Px(width) => Dimension::Px(width),
        _ => Dimension::Pct(100.0),
      };
      Row::new()
        .width(room)
        .max_width(Dimension::Pct(100.0))
        .child(Skeleton::line().width(Length::Fill).height(height).build(self.theme()))
        .into()
    };
    let fill = Length::Fill;
    let lines: Vec<Element> = match self.variant {
      LinkPreviewVariant::Card => vec![
        line(Length::Px(88.0), 10.0),
        line(fill, 14.0),
        line(fill, 12.0),
        line(Length::Px(168.0), 12.0),
      ],
      _ => vec![
        line(Length::Px(88.0), 10.0),
        line(Length::Px(232.0), 14.0),
        line(Length::Px(168.0), 12.0),
      ],
    };
    Column::new()
      .part(&part)
      .width(Dimension::Pct(100.0))
      .with_children(lines)
  }

  /// The column of the site, the title and the description, or of the skeleton.
  pub fn text(&self) -> Column {
    let column = Column::new()
      .part(&self.part(LinkPreviewPart::Text))
      .flex(1.0)
      .min_width(0.0);
    if self.loading {
      return column.child(self.skeleton());
    }
    let mut column = column.child(self.site()).child(self.title());
    if let Some(description) = self.description() {
      column = column.child(description);
    }
    column
  }

  /// The play mark and the length of a video, over its picture.
  fn video_marks(&self) -> Vec<Element> {
    let Some(length) = self.video else {
      return Vec::new();
    };
    let layer = || {
      Stack::new()
        .size(Dimension::Pct(100.0), Dimension::Pct(100.0))
        .pointer_events_none()
    };
    let play_part = self.part(LinkPreviewPart::PlayIcon);
    let side = play_part.rest().width.unwrap_or(Length::Icon(IconSize::Md));
    let play = Row::new()
      .part(&self.part(LinkPreviewPart::Play))
      .justify(Justify::Center)
      .align_items(Alignment::Center)
      .child(
        Row::new()
          .part(&play_part)
          .child(Icon::new(icons::PLAY).size(side).build(self.theme())),
      );
    let mut marks = vec![layer().stack_align(StackAlignment::Center).child(play).into()];
    if let Some(length) = length {
      let part = self.part(LinkPreviewPart::Length);
      marks.push(
        layer()
          .stack_align(StackAlignment::BottomEnd)
          .child(
            Row::new()
              .part(&part)
              .child(part.text(&super::media_time_text(length, false)).nowrap().part(&part)),
          )
          .into(),
      );
    }
    marks
  }

  /// The picture in its box, with the marks of a video over it; the skeleton's picture while the page is read.
  pub fn media(&self) -> Option<Row> {
    let part = self.part(LinkPreviewPart::Media);
    let picture = match (&self.image, self.loading) {
      (_, true) => Picture::empty().loading(true).decorative(true),
      (Some(picture), false) => {
        let mut picture = picture.clone().decorative(true);
        if let Some(failed) = &self.failed_image {
          let (failed, key) = (failed.clone(), self.image_key);
          picture = picture.on_error(move |_| failed.set(Some(key)));
        }
        picture
      }
      (None, false) => return None,
    };
    // A picture that shows what is behind it is drawn whole, with no box of its own, in the media's plate and
    // inside its padding: a logo cut to the box loses its edges, which are its shape. The plate takes the card's
    // ratio in its place. One much wider or higher than the square of a row is drawn whole in its own muted box,
    // which shows above and below it, or beside it, as the picture's own box does while it loads.
    let transparent = self.transparent && !self.loading;
    let picture = match self.variant {
      _ if transparent => picture
        .fit(PictureFit::Contain)
        .bare(true)
        .width(Length::Fill)
        .height(Length::Fill),
      _ if self.whole && !self.loading => picture
        .fit(PictureFit::Contain)
        .width(Length::Fill)
        .height(Length::Fill),
      // As wide as the card, and as high as its ratio makes it; round at the top as the card is, which cuts it.
      LinkPreviewVariant::Card => picture
        .width(Length::Fill)
        .aspect_ratio(CARD_PICTURE_RATIO)
        .style(PicturePart::Root, |s| {
          s.corners(RadiusSize::Xl, RadiusSize::Xl, 0.0, 0.0)
        })
        .style(PicturePart::Edge, |s| {
          s.corners(RadiusSize::Xl, RadiusSize::Xl, 0.0, 0.0)
        }),
      _ => picture.width(Length::Fill).height(Length::Fill),
    };
    let picture = self.video_marks().into_iter().fold(picture, Picture::child);
    let mut media = Row::new().part(&part).overflow_visible();
    if transparent && self.variant == LinkPreviewVariant::Card {
      media = media.aspect_ratio(CARD_PICTURE_RATIO);
    }
    Some(media.child(picture))
  }

  /// The close button: a stop of Tab of its own, which tells the app to drop the preview.
  pub fn close(&self) -> Option<components::Button> {
    if self.on_remove.is_empty() {
      return None;
    }
    let part = self.part(LinkPreviewPart::Close);
    let icon = self.part(LinkPreviewPart::CloseIcon);
    let side = icon.rest().width.unwrap_or(Length::Icon(IconSize::Sm));
    let handlers = self.on_remove.clone();
    let (focus, return_focus) = (self.focus.clone(), self.return_focus.clone());
    let mut close = pressable(&part, self.disabled)
      .align_items(Alignment::Center)
      .describe("a11y_role", "button")
      .describe("a11y_name", self.remove_label.clone())
      .child(Row::new().part(&icon).child(Icon::new(icons::CLOSE).size(side).build(self.theme())))
      // A press of the close button is not a press of the link around it.
      .on_click(move |event: MouseEvent| {
        event.stop_propagation();
        for handler in &handlers {
          handler();
        }
        // The preview is on its way out with the focus in it: the focus goes where the app said, or on.
        match (&focus, &return_focus) {
          (Some(focus), Some(element)) => focus.focus(element),
          (Some(focus), None) => focus.focus_onward(),
          _ => {}
        }
      });
    if let Some(id) = &self.id {
      close = close.id(format!("{id}-remove"));
    }
    Some(close)
  }

  /// The link, in the style of its root, with nothing in it yet: a click, Enter and Space open the address, Tab
  /// reaches it, and a disabled one takes nothing. It is a link to what reads the tree, named by its title and
  /// described by its site and its description.
  pub fn root(&self) -> components::Button {
    let mut root = pressable(&self.part(LinkPreviewPart::Root), self.disabled)
      .justify(Justify::Start)
      // The close button is a stop of Tab of its own, after the link.
      .stops_inside(true)
      .describe("a11y_role", "link")
      .describe("a11y_name", self.name())
      .describe("a11y_description", self.tooling_description())
      // What a selection over it copies, in a sentence: a Markdown link, which lurq's own inputs paste as one.
      .copy_text(&self.copy_text());
    if !self.disabled {
      let (url, handlers) = (self.url.clone(), self.on_open.clone());
      root = root.on_click(move |_: MouseEvent| {
        if handlers.is_empty() {
          if let Err(error) = crate::app::open_url(&url) {
            tracing::warn!("the link preview did not open {url}: {error}");
          }
        } else {
          handlers.iter().for_each(|handler| handler(&url));
        }
      });
      if let Some(hovered) = &self.hovered {
        let (enter, leave) = (hovered.clone(), hovered.clone());
        root = root
          .on_mouse_enter(move || {
            if !enter.get_untracked() {
              enter.set(true);
            }
          })
          .on_mouse_leave(move || {
            if leave.get_untracked() {
              leave.set(false);
            }
          });
      }
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root
  }

  /// How the default renderer builds a preview: a card's picture over its text, with the close button over its
  /// corner; a compact one's text beside its picture and the close button; an inline one's icon, title and site.
  pub fn render(self) -> Element {
    match self.variant {
      LinkPreviewVariant::Card => {
        let mut column = Column::new().width(Dimension::Pct(100.0)).min_width(0.0);
        if let Some(media) = self.media() {
          column = column.child(media);
        }
        column = column.child(self.text());
        let mut stack = Stack::new()
          .width(Dimension::Pct(100.0))
          .stack_align(StackAlignment::TopStart)
          .overflow_visible()
          .child(column);
        if let Some(close) = self.close() {
          stack = stack.child(
            Stack::new()
              .size(Dimension::Pct(100.0), Dimension::Pct(100.0))
              .stack_align(StackAlignment::TopEnd)
              .hit_test(HitTestBehavior::ContentOnly)
              .padding_top(8.0)
              .padding_right(8.0)
              .child(close),
          );
        }
        self.root().align_items(Alignment::Stretch).child(stack).into()
      }
      LinkPreviewVariant::Compact => {
        let mut root = self.root().align_items(Alignment::Center).child(self.text());
        if let Some(media) = self.media() {
          root = root.child(media);
        }
        if let Some(close) = self.close() {
          root = root.child(close);
        }
        root.into()
      }
      LinkPreviewVariant::Inline => {
        let mut root = self.root().align_items(Alignment::Center);
        if self.loading {
          // The icon's box in the muted color, as the board draws it while the page is read.
          root = root
            .child(Row::new().part(&self.part(LinkPreviewPart::Icon)))
            .child(Skeleton::line().width(96.0).height(10.0).build(self.theme()));
        } else {
          root = root.child(self.icon()).child(self.title());
          if !self.empty && self.site_shown {
            let part = self.part(LinkPreviewPart::Domain);
            root = root.child(
              Row::new()
                .flex_shrink(0.0)
                .child(part.text(&self.site).nowrap().part(&part)),
            );
          }
        }
        if let Some(close) = self.close() {
          root = root.child(close);
        }
        root.into()
      }
    }
  }

  fn render_themed(self) -> Element {
    match self.theme().renderer::<LinkPreview>() {
      Some(renderer) => renderer(self),
      None => self.render(),
    }
  }
}

impl crate::ui::Styled for LinkPreview {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for LinkPreview {
  const NAME: &'static str = "link_preview";
  type Renderer = LinkPreviewRenderer;

  fn parts() -> crate::ui::Declared {
    LinkPreviewPart::declared()
  }

  fn states() -> crate::ui::Declared {
    LinkPreviewState::declared()
  }

  /// The design's link preview (`pencil/lurq-design-system.pen`, Link Preview, Link Preview/Compact and Link
  /// Preview/Inline). A card of 360 in the card's color with an edge of the border's, corners of `radius-xl` and
  /// the small shadow; its picture is 1.91 times as wide as high (188 at 360) with a line under it, and its text
  /// is padded 12, 16, 14 and 16 with 6 between: the site at 12 in the muted color after its icon of 16 with
  /// corners of `radius-sm`, 6 apart, the title at 14/500 at 1.4 on two lines, the description at 13 in the muted
  /// color at 1.5 on two. Under the pointer it lifts to the medium shadow on the hover's veil, and its title is
  /// underlined; from the keyboard it has the ring. A video has the play mark of 48 (a glyph of 20) in the middle
  /// of its picture and its length in the mono type at 11 on a dark tag 8 from the lower end. The close button of
  /// one in a composer is a disc of 28 on the picture, 8 from the top end. The compact one is a row of 432 padded
  /// 12 with 12 between: the text, padded 4 at its start with 4 between, beside a picture of 72 with corners of
  /// `radius-md`, and the close button of 28; its title and description are on one line each, and the icon is 14.
  /// The inline one is 24 high with corners of `radius-md`, padded 5 at its start and 8 at its end with 6 between:
  /// the icon of 14, the title at 13/500 in the foreground's color and the site at 12 in the muted. While the page
  /// is read, the skeleton's lines stand for the text, 10 apart: 88 by 10, the width by 14, the width by 12 and
  /// 168 by 12 in a card; 88, 232 and 168 in a row; a square of 14 and a line of 96 by 10 in a sentence. A site's
  /// own icon stands on a white plate, the ground favicons are drawn for, which a light card does not show; and so
  /// does a picture that shows what is behind it (a logo on nothing), whole, 16 from the plate's edges in a card and
  /// 8 in a row. In a row, a picture more than 1.5 times as wide as high, or as high as wide, is drawn whole in its
  /// own muted box.
  fn recipe() -> Recipe {
    use LinkPreviewPart::*;
    use LinkPreviewState::{Disabled, Loading, Media as WithMedia, SiteIcon, Transparent};
    use LinkPreviewVariant::{Card, Compact, Inline};
    use PaletteColor::{
      Border as Edge, Card as Surface, CardForeground, Foreground, Muted, MutedForeground, StateHover,
    };
    let edge = || Border::inside(BorderSize::Sm, Edge);
    let white = Color::new(255, 255, 255, 255);
    Recipe::new()
      .base(Root, |s| {
        fading(
          s.background(Surface)
            .foreground(CardForeground)
            .border(edge())
            .cursor(CursorIcon::Pointer)
            .max_width(super::part_style::Length::Fill),
        )
      })
      .variant(Card, Root, |s| s.width(360.0).radius(RadiusSize::Xl).shadow(ShadowStyle::Sm))
      .variant(Compact, Root, |s| {
        s.width(432.0)
          .radius(RadiusSize::Xl)
          .gap(12.0)
          .padding_x(12.0)
          .padding_y(12.0)
      })
      .variant(Inline, Root, |s| {
        s.height(24.0)
          .radius(RadiusSize::Md)
          .gap(6.0)
          .padding_start(5.0)
          .padding_end(8.0)
          .foreground(Foreground)
      })
      .hovered(Root, |s| s.veil(StateHover))
      .when(Selector::new().variant(Card).hovered(), Root, |s| s.shadow(ShadowStyle::Md))
      .focus_visible(Root, |s| s.focus_ring())
      .state(Disabled, Root, |s| s.opacity(OpacityLevel::Disabled))
      // The picture: the top of a card, with a line between it and the text, and a square at the end of a row.
      .variant(Card, Media, |s| s.width(super::part_style::Length::Fill))
      .when(Selector::new().variant(Card).state(WithMedia), Text, |s| s.top_line(edge()))
      .variant(Compact, Media, |s| s.square(72.0).radius(RadiusSize::Md))
      // A picture that shows what is behind it was drawn for a light page, as a site's icon is: a dark logo on
      // nothing would be dark on the dark card. It stands whole on a white plate, round at the top as the card is.
      .state(Transparent, Media, |s| s.background(white).padding_x(16.0).padding_y(16.0))
      .when(Selector::new().variant(Card).state(Transparent), Media, |s| {
        s.corners(RadiusSize::Xl, RadiusSize::Xl, 0.0, 0.0)
      })
      .when(Selector::new().variant(Compact).state(Transparent), Media, |s| {
        s.padding_x(8.0).padding_y(8.0)
      })
      .base(Play, |s| {
        s.square(48.0)
          .radius(RadiusSize::Full)
          .background(Color::new(0, 0, 0, 0x99))
          .foreground(white)
      })
      .base(PlayIcon, |s| s.square(20.0))
      .base(Length, |s| {
        s.padding_x(6.0)
          .padding_y(2.0)
          .radius(RadiusSize::Sm)
          .background(Color::new(0, 0, 0, 0xB3))
          .foreground(white)
          .font_family(FontFamily::Mono)
          .font_size(11.0)
      })
      // The text.
      .variant(Card, Text, |s| {
        s.gap(6.0)
          .padding_top(12.0)
          .padding_x(16.0)
          .padding_bottom(14.0)
      })
      .variant(Compact, Text, |s| s.gap(4.0).padding_start(4.0))
      .base(Site, |s| s.gap(6.0))
      .base(Icon, |s| s.square(14.0).foreground(MutedForeground))
      .variant(Card, Icon, |s| s.square(16.0))
      // A site's icon is drawn for the light bar of a browser's tabs: GitHub's is dark on nothing, and on a dark
      // card it would be a dark box. It stands on a white plate, which a light card does not show.
      .state(SiteIcon, Icon, |s| s.background(white).radius(RadiusSize::Sm))
      .when(Selector::new().variant(Inline).state(Loading), Icon, |s| {
        s.background(Muted).radius(RadiusSize::Sm)
      })
      .base(Domain, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(12.0)
          .weight(FontWeight::Normal)
          .foreground(MutedForeground)
      })
      .base(Title, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(14.0)
          .weight(FontWeight::Medium)
          .line_height(1.4)
          .foreground(CardForeground)
      })
      .variant(Inline, Title, |s| s.font_size(13.0).foreground(Foreground))
      .base(Description, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(13.0)
          .weight(FontWeight::Normal)
          .line_height(1.4)
          .foreground(MutedForeground)
      })
      .variant(Card, Description, |s| s.line_height(1.5))
      .base(Skeleton, |s| s.gap(10.0).padding_y(4.0))
      .variant(Card, Skeleton, |s| s.padding_top(2.0).padding_bottom(4.0))
      // The close button: a disc on a card's picture, a plain button at the end of a row.
      .base(Close, |s| {
        fading(
          s.square(28.0)
            .radius(RadiusSize::Md)
            .foreground(Foreground)
            .cursor(CursorIcon::Pointer),
        )
      })
      .hovered(Close, |s| s.veil(StateHover))
      .focus_visible(Close, |s| s.focus_ring())
      .when(Selector::new().variant(Card).state(WithMedia), Close, |s| {
        s.radius(RadiusSize::Full)
          .background(Color::new(0, 0, 0, 0x99))
          .foreground(white)
      })
      .variant(Inline, Close, |s| s.square(20.0))
      .base(CloseIcon, |s| s.square(16.0))
      .variant(Inline, CloseIcon, |s| s.square(14.0))
  }
}
