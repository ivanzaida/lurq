//! A picture: an image in its box, with what it shows while it loads and when it cannot be shown, and what lies
//! over it.

use std::sync::Arc;

use super::{
  Menu, MenuItem, Skeleton, SkeletonAnimation, SkeletonPart, Tooltip, icons, names,
  part_style::{Length, PartAnimation, PartTransition},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::{fading, glyph, pressable},
  theme::{Themed, current_theme},
};
use crate::{
  animation::Transition,
  app::{
    component::Component,
    ctx::{Ctx, Placement},
    events::MouseEvent,
    theme::{MotionDuration, MotionEasing, OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::{Column, Row, Stack},
  core::{ElementRef, Signal},
  layout::{
    Alignment, StackAlignment,
    layout_kind::{Justify, SizeBounds},
    text_style::{FontStyle, FontWeight, TextAlign},
  },
  node::{
    CursorIcon, Element, EventHandler, HitTestBehavior, IntoMouseEventHandler, NodeUpdate, border::Border,
    color::Color, dimension::Dimension, outline::Outline,
  },
};

/// A picture this wide and wider is drawn whole: the large form.
pub const PICTURE_LARGE_FROM: f32 = 240.0;
/// A picture this wide and narrower is a thumbnail: the small form.
pub const PICTURE_SMALL_UP_TO: f32 = 96.0;
/// A picture this wide and narrower is as small as an avatar: the smallest form.
pub const PICTURE_SMALLEST_UP_TO: f32 = 40.0;
/// The ratio of a picture's box that has a width and nothing to tell its height: no height of its own, no ratio,
/// and no pixels yet. The video player's.
const WIDE: f32 = 16.0 / 9.0;

/// The forms of a picture, by the room it has: what it keeps of what lies over it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PictureSize {
  /// As small as an avatar, 40 wide and under: the picture, or a glyph where there is none.
  Xs,
  /// A thumbnail, 96 wide and under: the picture, the select mark and the count; what it does is a glyph or
  /// a line.
  Sm,
  /// Compact, under 240 wide: a title on one line and the menu's button alone.
  Md,
  /// From 240 wide: everything.
  #[default]
  Lg,
}

names!(PictureSize {
  Xs => "xs",
  Sm => "sm",
  Md => "md",
  Lg => "lg",
});

impl PictureSize {
  const ALL: [Self; 4] = [Self::Xs, Self::Sm, Self::Md, Self::Lg];

  /// The form that fits `width`.
  pub fn for_width(width: f32) -> Self {
    if width <= PICTURE_SMALLEST_UP_TO {
      Self::Xs
    } else if width <= PICTURE_SMALL_UP_TO {
      Self::Sm
    } else if width < PICTURE_LARGE_FROM {
      Self::Md
    } else {
      Self::Lg
    }
  }

  /// The widths of a picture this form is for: what [`for_width`](Self::for_width) says, as bounds.
  pub fn bounds(self) -> SizeBounds {
    match self {
      Self::Xs => SizeBounds::new().max_width(PICTURE_SMALLEST_UP_TO),
      Self::Sm => SizeBounds::new()
        .above_width(PICTURE_SMALLEST_UP_TO)
        .max_width(PICTURE_SMALL_UP_TO),
      Self::Md => SizeBounds::new()
        .above_width(PICTURE_SMALL_UP_TO)
        .below_width(PICTURE_LARGE_FROM),
      Self::Lg => SizeBounds::new().min_width(PICTURE_LARGE_FROM),
    }
  }

  fn of(name: Name) -> Self {
    Self::ALL
      .into_iter()
      .find(|size| Name::from(*size) == name)
      .unwrap_or_default()
  }
}

/// How a picture's pixels fill its box.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PictureFit {
  /// As small as fills the box, at its own ratio: what does not fit is cut.
  #[default]
  Cover,
  /// As large as fits the box whole, at its own ratio: the box shows beside it.
  Contain,
  /// Stretched to the box, whatever its own ratio.
  Fill,
  /// At its own size: what does not fit is cut, and the box shows beside what is smaller.
  None,
}

/// The shape of a picture's box.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PictureShape {
  /// The corners of a surface.
  #[default]
  Rounded,
  /// A circle, where the box is square.
  Circle,
}

/// What a picture's box shows while its pixels are on their way.
#[derive(Clone, Debug, Default)]
pub enum PicturePlaceholder {
  /// The skeleton's sheen over the box.
  #[default]
  Skeleton,
  /// A small version of the picture, blurred: a thumbnail of a few hundred bytes the app has at hand, a few
  /// pixels it made from a hash, or a small file it loads first. It shows once it is there, and the picture
  /// comes in over it. The box alone until then. Made by [`blur`](Self::blur) and
  /// [`blur_pixels`](Self::blur_pixels).
  Blur(PictureSmall),
  /// The box alone.
  None,
}

impl PicturePlaceholder {
  /// [`Blur`](Self::Blur) of what `source` gives, read and decoded as the picture is: bytes, a file, a
  /// resource, an address.
  pub fn blur(source: impl Into<crate::resources::Source>) -> Self {
    Self::Blur(PictureSmall(Source::Encoded {
      source: source.into(),
      cap: None,
    }))
  }

  /// [`Blur`](Self::Blur) of pixels the app has.
  #[cfg(feature = "raster")]
  pub fn blur_pixels(data: crate::images::ImageData) -> Self {
    Self::Blur(PictureSmall(Source::Data(data)))
  }
}

/// The small version of a picture that stands in for it, blurred: see [`PicturePlaceholder::Blur`].
#[derive(Clone)]
pub struct PictureSmall(#[cfg_attr(not(feature = "raster"), allow(dead_code))] Source);

impl std::fmt::Debug for PictureSmall {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str("PictureSmall")
  }
}

/// How far the small version of a picture is blurred while it stands in for it.
#[cfg(feature = "raster")]
const BLUR: f32 = 24.0;
/// How much larger than the box the blurred version is drawn, so that the soft edge the blur gives it lies
/// outside the box and is cut off.
#[cfg(feature = "raster")]
const BLUR_OVERDRAW: f32 = 1.25;

/// The parts of a picture a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PicturePart {
  /// The picture with its caption under it: a figure. Its gap is between the two.
  Figure,
  /// The picture's box: its fill shows while it loads, beside a picture that does not fill it and where there
  /// is none. Rounded, and what it holds is cut to it.
  Root,
  /// The pixels.
  Image,
  /// The line around the box, over the picture.
  Edge,
  /// What stands in for the pixels while they are on their way, as a whole: what it leaves with is how it
  /// goes when they are there, over them. Only a box whose size does not wait for the pixels has it.
  StandIn,
  /// The sheen that crosses the box while it loads.
  Sheen,
  /// The dark layer over the picture under a count.
  Veil,
  /// The app's own layer over the whole picture.
  Overlay,
  /// The middle of the box: what stands for a picture that is not there.
  Center,
  /// The glyph of a box with no picture to show.
  Mark,
  /// What the picture is called, under the glyph of a box with none.
  Label,
  /// The glyph of a picture that did not load.
  ErrorIcon,
  /// What the picture is called, in place of one that did not load.
  ErrorLabel,
  /// The line that says it did not load.
  ErrorDescription,
  /// The button that asks for it again.
  Retry,
  /// How far a load has come, in the middle of the box: the rail and the words under it.
  Progress,
  /// The rail of the load.
  Track,
  /// How much of it is loaded.
  Fill,
  /// The words under the rail.
  ProgressLabel,
  /// The line of the load along the bottom edge of a small picture.
  ProgressLine,
  /// How many more pictures the row has, over the veil.
  Count,
  /// The row along the top: the badge at its start, the select mark at its end.
  Top,
  /// The disc that says the picture is chosen, and the ring of one that can be.
  Select,
  /// The check in the disc.
  SelectIcon,
  /// The bar along the bottom: the caption and the actions, on a gradient. Its background is the color the
  /// gradient ends in at the bottom, from nothing at its top.
  Controls,
  /// The column of the bar's title and description.
  Text,
  /// The title in the bar.
  Title,
  /// The line under the title in the bar.
  Description,
  /// The row of the bar's buttons.
  Actions,
  /// A button of the bar.
  Control,
  /// The glyph of a button of the bar.
  ControlIcon,
  /// The panel of the menu behind More.
  MoreMenu,
  /// The caption under the picture.
  Caption,
}

names!(PicturePart {
  Figure => "figure",
  Root => "root",
  Image => "image",
  Edge => "edge",
  StandIn => "stand_in",
  Sheen => "sheen",
  Veil => "veil",
  Overlay => "overlay",
  Center => "center",
  Mark => "mark",
  Label => "label",
  ErrorIcon => "error_icon",
  ErrorLabel => "error_label",
  ErrorDescription => "error_description",
  Retry => "retry",
  Progress => "progress",
  Track => "track",
  Fill => "fill",
  ProgressLabel => "progress_label",
  ProgressLine => "progress_line",
  Count => "count",
  Top => "top",
  Select => "select",
  SelectIcon => "select_icon",
  Controls => "controls",
  Text => "text",
  Title => "title",
  Description => "description",
  Actions => "actions",
  Control => "control",
  ControlIcon => "control_icon",
  MoreMenu => "more_menu",
  Caption => "caption",
});

/// The states of a picture a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PictureState {
  /// It is the box alone: no fill, no corners, no line around it.
  Bare,
  /// Its shape is [`PictureShape::Circle`].
  Circle,
  /// It is pressed as a whole: it opens, or it is chosen.
  Pressable,
  /// It can be chosen: the ring of its mark shows under the pointer and with the focus.
  Selectable,
  /// It is chosen.
  Selected,
  /// It takes nothing.
  Disabled,
  /// Its bar shows whatever the pointer does.
  Shown,
  /// A button of its bar whose menu is open.
  On,
}

names!(PictureState {
  Bare => "bare",
  Circle => "circle",
  Pressable => "pressable",
  Selectable => "selectable",
  Selected => "selected",
  Disabled => "disabled",
  Shown => "shown",
  On => "on",
});

/// What builds a picture's nodes: a theme's in place of the default one.
pub type PictureRenderer = Arc<dyn Fn(PictureView) -> Element + Send + Sync>;

type Action = Arc<dyn Fn() + Send + Sync>;
type Told<T> = Arc<dyn Fn(T) + Send + Sync>;

/// Where a picture's pixels come from.
#[derive(Clone)]
enum Source {
  /// Nowhere: the box says so.
  None,
  #[cfg(feature = "raster")]
  Data(crate::images::ImageData),
  /// Encoded bytes to read and decode: a file, bytes, a resource of the app, an address. `cap` is the most
  /// pixels across and down it is decoded to.
  Encoded {
    source: crate::resources::Source,
    cap: Option<(u32, u32)>,
  },
}

/// Where a picture's pixels are, as its component found them when it rendered.
#[derive(Clone)]
enum Phase {
  /// It has no source.
  Empty,
  #[cfg(feature = "raster")]
  Ready(crate::images::ImageData),
  /// On their way: the bytes read and how many there are, where the app's loader reads them and knows.
  #[cfg_attr(not(feature = "image"), allow(dead_code))]
  Loading(Option<(u64, u64)>),
  /// They did not come, or were no picture: why.
  Failed(Arc<str>),
}

/// How far the load of a picture has come: what [`Picture::progress_text`] is given.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PictureLoad {
  /// How much of it is there, of 1.
  pub fraction: f64,
  /// The bytes read and how many there are, where the app's loader loads it and knows.
  pub bytes: Option<(u64, u64)>,
}

impl PictureLoad {
  /// "42%", and "42% · 1.4 of 3.4 MB" where the bytes are known.
  fn text(self) -> String {
    let percent = format!("{}%", (self.fraction * 100.0).round());
    match self.bytes {
      Some((loaded, total)) => {
        let megabytes = |bytes: u64| bytes as f64 / (1024.0 * 1024.0);
        format!("{percent} · {:.1} of {:.1} MB", megabytes(loaded), megabytes(total))
      }
      None => percent,
    }
  }
}

/// Why this build shows no picture decoded from encoded bytes.
#[cfg(not(feature = "image"))]
const NO_CODECS: &str = "this build has no `image` feature: it decodes no picture";

/// The words a picture writes itself.
#[derive(Clone, PartialEq)]
struct Words {
  error_description: Arc<str>,
  retry: Arc<str>,
  download: Arc<str>,
  more: Arc<str>,
  open: Arc<str>,
}

impl Default for Words {
  fn default() -> Self {
    Self {
      error_description: Arc::from("The picture didn't load."),
      retry: Arc::from("Try again"),
      download: Arc::from("Download"),
      more: Arc::from("More"),
      open: Arc::from("Open"),
    }
  }
}

/// A picture: an image in its box.
///
/// ```
/// use lurq::ui::{Picture, PictureFit};
///
/// let lake = Picture::from_file("photos/lake.jpg")
///   .label("The lake at dusk, the ridge behind it")
///   .width(320.0)
///   .aspect_ratio(3.0 / 2.0);
/// let logo = Picture::from_resource("brand/logo.png").fit(PictureFit::Contain).size_px(120.0, 48.0);
/// ```
///
/// Its width and its height are a browser's `img`'s. With neither said it is as large as its pixels; with one
/// said the other follows the pixels' ratio; with both said, or with an [`aspect_ratio`](Self::aspect_ratio),
/// it is a box, which the pixels cover unless [`fit`](Self::fit) says otherwise. A picture whose pixels are not
/// there yet has no size of its own: say one, or a ratio, and the page does not move when they come.
///
/// While it decodes the box shows the skeleton's sheen; a picture that did not load shows what it is called
/// ([`label`](Self::label)), that it did not load, and a button that asks for it again; a picture with no source
/// ([`empty`](Self::empty)) shows a glyph and what it would be called.
///
/// What lies over it: a [`badge`](Self::badge) at the top, and a bar along the bottom with a
/// [`title`](Self::title), a [`description`](Self::description) and its buttons, which shows under the pointer
/// and with the focus. A small picture keeps less of that: see [`PictureSize`]. Its form follows its width where
/// [`size`](Self::size) does not say it.
///
/// A picture is a component. Laid in a tree (`Row::new().child(Picture::from_url(..))`) it is mounted by the
/// component that renders the tree, and keeps what is its own for as long as it is rendered: the load of its
/// source and how far it has come, whether the menu behind More is open, its buttons' tooltips. In a list that
/// changes give it a [`key`](Self::key). In a tree no component renders it shows nothing.
#[derive(Clone)]
pub struct Picture {
  source: Source,
  label: Option<Arc<str>>,
  decorative: bool,
  width: Option<Length>,
  height: Option<Length>,
  aspect_ratio: Option<f32>,
  fit: PictureFit,
  anchor: StackAlignment,
  shape: PictureShape,
  bare: bool,
  size: Option<Name>,
  placeholder: PicturePlaceholder,
  loading: bool,
  progress: Option<f64>,
  progress_text: Option<Arc<dyn Fn(PictureLoad) -> String + Send + Sync>>,
  words: Words,
  on_retry: Vec<Action>,
  on_load: Vec<Action>,
  on_error: Vec<Told<Arc<str>>>,
  fallback: Option<Arc<dyn Fn() -> Element + Send + Sync>>,
  caption: Option<Arc<str>>,
  title: Option<Arc<str>>,
  description: Option<Arc<str>>,
  badge: Option<Element>,
  on_download: Vec<Action>,
  menu_items: Vec<MenuItem>,
  actions: Vec<Element>,
  bar: bool,
  force_bar_shown: Option<bool>,
  children: Vec<Element>,
  count: Option<Arc<str>>,
  on_click: Vec<EventHandler<MouseEvent>>,
  selectable: bool,
  selected: bool,
  on_change: Vec<Told<bool>>,
  lightbox: bool,
  open: Option<Signal<bool>>,
  on_open_change: Vec<Told<bool>>,
  disabled: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Picture {
  fn of(source: Source) -> Self {
    Self {
      source,
      label: None,
      decorative: false,
      width: None,
      height: None,
      aspect_ratio: None,
      fit: PictureFit::default(),
      anchor: StackAlignment::Center,
      shape: PictureShape::default(),
      bare: false,
      size: None,
      placeholder: PicturePlaceholder::default(),
      loading: false,
      progress: None,
      progress_text: None,
      words: Words::default(),
      on_retry: Vec::new(),
      on_load: Vec::new(),
      on_error: Vec::new(),
      fallback: None,
      caption: None,
      title: None,
      description: None,
      badge: None,
      on_download: Vec::new(),
      menu_items: Vec::new(),
      actions: Vec::new(),
      bar: true,
      force_bar_shown: None,
      children: Vec::new(),
      count: None,
      on_click: Vec::new(),
      selectable: false,
      selected: false,
      on_change: Vec::new(),
      lightbox: false,
      open: None,
      on_open_change: Vec::new(),
      disabled: false,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// A picture of pixels the app has.
  #[cfg(feature = "raster")]
  pub fn new(data: crate::images::ImageData) -> Self {
    Self::of(Source::Data(data))
  }

  /// A picture with no source: its box, with a glyph and what it would be called in it.
  pub fn empty() -> Self {
    Self::of(Source::None)
  }

  /// A picture read from `source` and decoded, off the UI thread: a file, bytes, a resource of the app or an
  /// address, as [`Source`](crate::resources::Source) says them. A resource and an address are read by the
  /// app's [`Loader`](crate::resources::Loader). The pixels are kept under the source's key.
  ///
  /// A build without the feature `image` has no decoder: the picture shows that it did not load, and why.
  pub fn from_source(source: impl Into<crate::resources::Source>) -> Self {
    Self::of(Source::Encoded {
      source: source.into(),
      cap: None,
    })
  }

  /// A picture read from a file and decoded. Its key is the path with the file's size and the time it was
  /// changed.
  pub fn from_file(path: impl Into<std::path::PathBuf>) -> Self {
    Self::from_source(crate::resources::Source::file(path))
  }

  /// A picture decoded from encoded bytes (PNG, JPEG, the first frame of a GIF or WebP, BMP, TIFF). The bytes
  /// are hashed for the key its pixels are kept under, unless [`cache_key`](Self::cache_key) gives one.
  pub fn from_bytes(bytes: impl Into<Arc<[u8]>>) -> Self {
    Self::from_source(crate::resources::Source::bytes(bytes))
  }

  /// A picture that is a resource of the app: a path under the root the app gave its loader
  /// (`app.loader().root(..)`).
  pub fn from_resource(path: &str) -> Self {
    Self::from_source(crate::resources::Source::resource(path))
  }

  /// A picture at an address on the network. Outside a browser it needs the feature `http`; a build without it
  /// shows that the picture did not load, and why.
  pub fn from_url(url: &str) -> Self {
    Self::from_source(crate::resources::Source::url(url))
  }

  /// The key a decoded picture's pixels are kept under, in place of the one its source makes: the same key is
  /// the same pixels. An app that stores a hash of what it shows gives it.
  pub fn cache_key(mut self, key: u64) -> Self {
    if let Source::Encoded { source, .. } = &mut self.source {
      *source = source.clone().key(key);
    }
    self
  }

  /// Decodes no larger than `width` by `height` pixels, keeping the ratio and never enlarging: a thumbnail of
  /// a large photo does not keep all of it.
  pub fn downscale_to(mut self, width: u32, height: u32) -> Self {
    if let Source::Encoded { cap, .. } = &mut self.source {
      *cap = Some((width, height));
    }
    self
  }

  /// What the picture is called: what it shows of itself where there are no pixels, and its name to tooling
  /// that reads the tree. An `alt` text.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// Whether the picture says nothing the words around it do not: tooling that reads the tree passes it by.
  pub fn decorative(mut self, decorative: bool) -> Self {
    self.decorative = decorative;
    self
  }

  /// The box's width. Not said, it follows the height by the pixels' ratio, or is the pixels' own.
  pub fn width(mut self, width: impl Into<Length>) -> Self {
    self.width = Some(width.into());
    self
  }

  /// The box's height. Not said, it follows the width by the pixels' ratio, or is the pixels' own.
  pub fn height(mut self, height: impl Into<Length>) -> Self {
    self.height = Some(height.into());
    self
  }

  /// A box of `width` by `height` pixels.
  pub fn size_px(self, width: f32, height: f32) -> Self {
    self.width(width).height(height)
  }

  /// The box's width over its height, which gives the side that is not said from the one that is, and with
  /// neither said makes the box as wide as its room. It holds the box of a picture whose pixels are not there
  /// yet.
  pub fn aspect_ratio(mut self, ratio: f32) -> Self {
    self.aspect_ratio = Some(ratio);
    self
  }

  /// How the pixels fill the box: they cover it unless told.
  pub fn fit(mut self, fit: PictureFit) -> Self {
    self.fit = fit;
    self
  }

  /// Where the pixels are held in the box where they are cut or do not fill it: the middle unless told.
  pub fn anchor(mut self, anchor: StackAlignment) -> Self {
    self.anchor = anchor;
    self
  }

  pub fn shape(mut self, shape: PictureShape) -> Self {
    self.shape = shape;
    self
  }

  /// The pixels alone: no fill of the box, no corners, no line around it.
  pub fn bare(mut self, bare: bool) -> Self {
    self.bare = bare;
    self
  }

  /// Which form it is drawn in, a [`PictureSize`], in place of the one its width gives it.
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.size = Some(size.into());
    self
  }

  /// What the box shows while the pixels are on their way.
  pub fn placeholder(mut self, placeholder: PicturePlaceholder) -> Self {
    self.placeholder = placeholder;
    self
  }

  /// Shows the box as it is while the pixels are on their way, whatever its source: for a picture the app is
  /// still fetching and has no source for yet.
  pub fn loading(mut self, loading: bool) -> Self {
    self.loading = loading;
    self
  }

  /// How far the load has come, of 1, where the app knows: a rail in the middle of the box in place of the
  /// sheen, and along the bottom edge of a small picture. `None` for a load that cannot tell. A picture the
  /// app's loader reads from an address shows it by itself where the server says how much there is.
  pub fn progress(mut self, progress: impl Into<Option<f64>>) -> Self {
    self.progress = progress.into().map(|progress| progress.clamp(0.0, 1.0));
    self
  }

  /// The words under the rail of a load, from how far it has come: "42%" unless told, and "42% · 1.4 of
  /// 3.4 MB" where the app's loader knows the bytes.
  pub fn progress_text(mut self, text: impl Fn(PictureLoad) -> String + Send + Sync + 'static) -> Self {
    self.progress_text = Some(Arc::new(text));
    self
  }

  /// The line that says a picture did not load: "The picture didn't load." unless told.
  pub fn error_description(mut self, description: &str) -> Self {
    self.words.error_description = Arc::from(description);
    self
  }

  /// The button that asks again for a picture that did not load: "Try again" unless told.
  pub fn retry_label(mut self, label: &str) -> Self {
    self.words.retry = Arc::from(label);
    self
  }

  /// What the download button is called: "Download" unless told.
  pub fn download_label(mut self, label: &str) -> Self {
    self.words.download = Arc::from(label);
    self
  }

  /// What the button that opens the picture in its lightbox is called: "Open" unless told.
  pub fn open_label(mut self, label: &str) -> Self {
    self.words.open = Arc::from(label);
    self
  }

  /// What the menu's button is called: "More" unless told.
  pub fn more_label(mut self, label: &str) -> Self {
    self.words.more = Arc::from(label);
    self
  }

  /// Called on a press of the button that asks again for a picture that did not load, before the picture
  /// asks. A second handler is called after the first.
  pub fn on_retry(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_retry.push(Arc::new(handler));
    self
  }

  /// What the box shows wherever it has no pixels (while they are on their way, when they did not come, with
  /// no source), in place of what a picture shows there itself: an avatar's initials. It goes out over the
  /// pixels as the picture's own stand-in does.
  pub(crate) fn fallback(mut self, fallback: impl Fn() -> Element + Send + Sync + 'static) -> Self {
    self.fallback = Some(Arc::new(fallback));
    self
  }

  /// Called once the picture's pixels are there: once for a source, and again for one asked for anew.
  pub fn on_load(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_load.push(Arc::new(handler));
    self
  }

  /// Called once the picture did not come, with why: the load failed, or what it brought was no picture.
  pub fn on_error(mut self, handler: impl Fn(&str) + Send + Sync + 'static) -> Self {
    self.on_error.push(Arc::new(move |reason: Arc<str>| handler(&reason)));
    self
  }

  /// A caption under the picture, which makes it a figure: "Figure 1. The lake at dusk."
  pub fn caption(mut self, caption: &str) -> Self {
    self.caption = Some(Arc::from(caption));
    self
  }

  /// The title in the bar over the picture's bottom edge.
  pub fn title(mut self, title: &str) -> Self {
    self.title = Some(Arc::from(title));
    self
  }

  /// The line under the title in the bar.
  pub fn description(mut self, description: &str) -> Self {
    self.description = Some(Arc::from(description));
    self
  }

  /// What stands at the top of the picture, at its start: a badge.
  pub fn badge(mut self, badge: impl Into<Element>) -> Self {
    self.badge = Some(badge.into());
    self
  }

  /// Called on a press of the download button, which the bar has once this is given. A second handler is
  /// called after the first.
  pub fn on_download(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_download.push(Arc::new(handler));
    self
  }

  /// An item of the menu behind More, which the bar has once it has an item. The menu opens on a mounted
  /// picture.
  pub fn menu_item(mut self, item: MenuItem) -> Self {
    self.menu_items.push(item);
    self
  }

  /// Items of the menu behind More, in order.
  pub fn menu_items(mut self, items: impl IntoIterator<Item = MenuItem>) -> Self {
    self.menu_items.extend(items);
    self
  }

  /// A button of the app's own in the bar, before the picture's: each call adds one.
  pub fn action(mut self, action: impl Into<Element>) -> Self {
    self.actions.push(action.into());
    self
  }

  /// Buttons of the app's own in the bar, in order.
  pub fn actions(mut self, actions: impl IntoIterator<Item = impl Into<Element>>) -> Self {
    self.actions.extend(actions.into_iter().map(Into::into));
    self
  }

  /// Whether the bar is drawn. Without it the picture is its pixels, its badge and the app's layer.
  pub fn bar(mut self, bar: bool) -> Self {
    self.bar = bar;
    self
  }

  /// Shows the bar, and the ring of the mark of a picture that can be chosen, or hides the bar, whatever the
  /// pointer and the focus do: for a picture that always says what it is, and for a gallery's page of states.
  pub fn force_bar_shown(mut self, shown: Option<bool>) -> Self {
    self.force_bar_shown = shown;
    self
  }

  /// Something of the app's own over the whole picture: a layer the size of the box, under the bar.
  pub fn child(mut self, child: impl Into<Element>) -> Self {
    self.children.push(child.into());
    self
  }

  /// Things of the app's own over the picture, in order.
  pub fn children(mut self, children: impl IntoIterator<Item = impl Into<Element>>) -> Self {
    self.children.extend(children.into_iter().map(Into::into));
    self
  }

  /// A count over a veil: "+5" on the last picture of a row that could not hold them all. A gallery's.
  pub(crate) fn count(mut self, count: &str) -> Self {
    self.count = Some(Arc::from(count));
    self
  }

  /// Called on a press of the picture: a click, or Enter or Space while it has the focus. A picture given one
  /// is pressed as a whole; a press of a button of its bar is that button's.
  pub fn on_click(mut self, handler: impl IntoMouseEventHandler) -> Self {
    self.on_click.push(handler.into_event_handler());
    self
  }

  /// Whether the picture can be chosen: a press chooses it, and the ring of its mark shows under the pointer.
  pub fn selectable(mut self, selectable: bool) -> Self {
    self.selectable = selectable;
    self
  }

  /// Whether the picture is chosen: the mark at its top, and a line around it. The app says so; the picture
  /// tells [`on_change`](Self::on_change) what a press would make it.
  pub fn selected(mut self, selected: bool) -> Self {
    self.selected = selected;
    self
  }

  /// Called on a press of a picture that can be chosen, with what it is to be. A second handler is called
  /// after the first.
  pub fn on_change(mut self, handler: impl Fn(bool) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// A press of the picture, or Enter or Space while it has the focus, opens it in a [`Lightbox`] over the
  /// window: the picture alone, whole, to bring closer. The lightbox shows its [`title`](Self::title) and its
  /// [`description`](Self::description). The bar of a large picture has a button that opens it too, the
  /// last one: a picture that can be chosen is chosen by a press, and opened by that button.
  pub fn lightbox(mut self, lightbox: bool) -> Self {
    self.lightbox = lightbox;
    self
  }

  /// Whether its lightbox is open, to read and to set. Without it the picture keeps it.
  pub fn open(mut self, open: Signal<bool>) -> Self {
    self.open = Some(open);
    self
  }

  /// Called when its lightbox opens, and when it is closed.
  pub fn on_open_change(mut self, handler: impl Fn(bool) + Send + Sync + 'static) -> Self {
    self.on_open_change.push(Arc::new(handler));
    self
  }

  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Puts the picture in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this picture over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this picture.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the picture as under the pointer, pressed or focused from the keyboard, whatever the pointer does.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the picture's box; its parts are `{id}-retry`, `{id}-download`, `{id}-more`.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// The picture mounted in `ctx`: what converting it into an element does once the render that returns it
  /// has returned.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let key = self.key.clone();
    let props = PictureProps(self);
    match key {
      Some(key) => ctx.mount_keyed::<PictureHost>(&key, props),
      None => ctx.mount::<PictureHost>(props),
    }
  }

  /// The picture as a viewer shows it: the pixels alone, whole, in all the room there is, with nothing over
  /// them and nothing a press does. What it is called and how it loads stay.
  /// Starts bringing the picture's pixels on without showing it: a file and bytes are decoded, a resource and
  /// an address asked of `loader` and decoded once their bytes are there. What a picture shown later then has
  /// at once. The load to hold while it is on its way, where the loader reads it: a load nobody holds is let
  /// go. Called again once it has ended, it decodes what it brought.
  #[cfg(feature = "image")]
  pub(crate) fn fetch(&self, loader: Option<&crate::resources::Loader>) -> Option<crate::resources::Load> {
    use crate::{
      images::{ImageKey, ImageSizeCap, ImageSource, decoded_images},
      resources::{LoadState, SourceKind},
    };
    let Source::Encoded { source, cap } = &self.source else {
      return None;
    };
    let key = ImageKey::new(source.cache_key());
    let cap = cap.map(|(width, height)| ImageSizeCap::new(width, height));
    let decode = |from: ImageSource| {
      decoded_images().load(key, cap, || from, None);
    };
    match &source.kind {
      SourceKind::Bytes(bytes) => decode(ImageSource::Bytes(bytes.clone())),
      SourceKind::File(path) => decode(ImageSource::File(path.clone())),
      SourceKind::Resource(_) | SourceKind::Url(_) => {
        if decoded_images().state(key, cap).is_some() {
          return None;
        }
        let load = loader?.load(source);
        match load.state() {
          LoadState::Loading { .. } => return Some(load),
          LoadState::Ready(bytes) => decode(ImageSource::Bytes(bytes)),
          LoadState::Failed(_) => {}
        }
      }
    }
    None
  }

  pub(crate) fn in_viewer(mut self) -> Self {
    self = self.stripped();
    self.width = Some(Length::Fill);
    self.height = Some(Length::Fill);
    self.aspect_ratio = None;
    self.fit = PictureFit::Contain;
    self.shape = PictureShape::Rounded;
    self.bare = true;
    self.size = Some(PictureSize::Lg.into());
    self
  }

  /// The picture as a small one of `width` by `height` in a strip or a grid of them: the pixels covering the
  /// box, with nothing over them and nothing a press does.
  pub(crate) fn as_thumb(mut self, width: f32, height: f32) -> Self {
    self = self.stripped();
    self.width = Some(Length::Px(width));
    self.height = Some(Length::Px(height));
    self.aspect_ratio = None;
    self.fit = PictureFit::Cover;
    self.bare = true;
    self.size = None;
    // What holds it is what tooling reads.
    self.decorative = true;
    self
  }

  /// The picture as one of a gallery's: the pixels covering all of the room it is given, with nothing over
  /// them and nothing a press does yet. What it is called and how it loads stay, and so do its box and its
  /// corners.
  pub(crate) fn in_grid(mut self) -> Self {
    self = self.stripped();
    self.width = Some(Length::Fill);
    self.height = Some(Length::Fill);
    self.aspect_ratio = None;
    self.fit = PictureFit::Cover;
    self
  }

  /// The picture with what lies over it and what a press of it does taken off, and no name in the tree of its
  /// own: what another component shows of it.
  fn stripped(mut self) -> Self {
    self.caption = None;
    self.badge = None;
    self.bar = false;
    self.force_bar_shown = None;
    self.children.clear();
    self.count = None;
    self.on_click.clear();
    self.selectable = false;
    self.selected = false;
    self.on_change.clear();
    self.lightbox = false;
    self.open = None;
    self.on_open_change.clear();
    self.id = None;
    self.key = None;
    self
  }

  /// The title and the description of its bar.
  pub(crate) fn caption_of(&self) -> (Option<Arc<str>>, Option<Arc<str>>) {
    (self.title.clone(), self.description.clone())
  }

  /// What it is called: its label, or its title.
  pub(crate) fn label_of(&self) -> Option<Arc<str>> {
    self.label.clone().or_else(|| self.title.clone())
  }

  /// Whether a press of the picture as a whole does something.
  fn pressable(&self) -> bool {
    !self.disabled && (self.selectable || self.lightbox || !self.on_click.is_empty())
  }

  /// The forms to build, each with the widths it lives at: the one that is said, or that a width in pixels
  /// gives, alone and at every width; else all of them, for layout to show the one that fits.
  fn forms(&self, theme: &Theme) -> Vec<(PictureSize, Option<SizeBounds>)> {
    if let Some(size) = self.size {
      return vec![(PictureSize::of(size), None)];
    }
    match self.width.and_then(|width| width.resolve(theme)) {
      Some(Dimension::Px(width)) => vec![(PictureSize::for_width(width), None)],
      // A box that takes the size of what is in it has no width to choose a form by before it is laid out.
      None if self.aspect_ratio.is_none() => vec![(PictureSize::Lg, None)],
      _ => PictureSize::ALL
        .into_iter()
        .map(|size| (size, Some(size.bounds())))
        .collect(),
    }
  }
}

/// Pixels the app has are a picture of them.
#[cfg(feature = "raster")]
impl From<crate::images::ImageData> for Picture {
  fn from(data: crate::images::ImageData) -> Self {
    Self::new(data)
  }
}

/// A picture in a tree is its component, declared: the component that renders the tree mounts it.
impl From<Picture> for Element {
  fn from(picture: Picture) -> Self {
    let key = picture.key.clone();
    let props = PictureProps(picture);
    match key {
      Some(key) => Element::component_keyed::<PictureHost>(key, props),
      None => Element::component::<PictureHost>(props),
    }
  }
}

/// A picture to mount.
#[derive(Clone)]
struct PictureProps(Picture);

/// A picture is built anew by every render of what holds it, and is the picture it was where all it is made
/// of can be told to be the same: then its component is not rendered again, and what it rendered last stands.
/// A closure and an element are made anew by each render and nothing compares them: a picture with a handler,
/// a badge, a button, a layer, a menu or rules of its own is another picture each time, as it was.
impl PartialEq for PictureProps {
  fn eq(&self, other: &Self) -> bool {
    match (self.0.plain(), other.0.plain()) {
      (Some(this), Some(that)) => this == that,
      _ => false,
    }
  }
}

/// Which pixels a source is: what tells two sources apart without reading them.
#[derive(PartialEq)]
enum SourceId {
  None,
  /// Pixels the app has, by the buffer's id.
  #[cfg(feature = "raster")]
  Data(u64),
  /// What is read and decoded, by the source's key and the size it is decoded to.
  Encoded(u64, Option<(u32, u32)>),
}

impl Source {
  fn id(&self) -> SourceId {
    match self {
      Source::None => SourceId::None,
      #[cfg(feature = "raster")]
      Source::Data(data) => SourceId::Data(data.id()),
      Source::Encoded { source, cap } => SourceId::Encoded(source.cache_key(), *cap),
    }
  }
}

/// All a picture is made of that can be compared, where it is made of nothing else.
#[derive(PartialEq)]
struct Plain<'a> {
  source: SourceId,
  /// What stands in for it: 0 for the sheen, 1 for the box alone, 2 for a small version, and which.
  placeholder: (u8, SourceId),
  texts: [&'a Option<Arc<str>>; 7],
  words: &'a Words,
  lengths: (&'a Option<Length>, &'a Option<Length>, Option<f32>),
  shown: (PictureFit, StackAlignment, PictureShape, &'a Option<Name>, &'a [Name]),
  flags: [bool; 8],
  /// The signal that says whether its lightbox is open, by its id.
  open: Option<usize>,
  load: (Option<f64>, Option<bool>, Option<Interaction>),
}

impl Picture {
  /// The picture as what can be compared, or `None` where it holds a closure, an element or rules.
  fn plain(&self) -> Option<Plain<'_>> {
    // Every field is named, so that a new one is put on one side or the other here.
    let Picture {
      source,
      label,
      decorative,
      width,
      height,
      aspect_ratio,
      fit,
      anchor,
      shape,
      bare,
      size,
      placeholder,
      loading,
      progress,
      progress_text,
      words,
      on_retry,
      on_load,
      on_error,
      fallback,
      caption,
      title,
      description,
      badge,
      on_download,
      menu_items,
      actions,
      bar,
      force_bar_shown,
      children,
      count,
      on_click,
      selectable,
      selected,
      on_change,
      lightbox,
      open,
      on_open_change,
      disabled,
      states,
      rules,
      forced,
      id,
      key,
    } = self;
    let made_anew = progress_text.is_some()
      || fallback.is_some()
      || badge.is_some()
      || !(on_retry.is_empty()
        && on_load.is_empty()
        && on_error.is_empty()
        && on_download.is_empty()
        && menu_items.is_empty()
        && actions.is_empty()
        && children.is_empty()
        && on_click.is_empty()
        && on_change.is_empty()
        && on_open_change.is_empty()
        && rules.is_empty());
    if made_anew {
      return None;
    }
    Some(Plain {
      source: source.id(),
      placeholder: match placeholder {
        PicturePlaceholder::Skeleton => (0, SourceId::None),
        PicturePlaceholder::None => (1, SourceId::None),
        PicturePlaceholder::Blur(PictureSmall(small)) => (2, small.id()),
      },
      texts: [label, caption, title, description, count, id, key],
      words,
      lengths: (width, height, *aspect_ratio),
      shown: (*fit, *anchor, *shape, size, states),
      flags: [
        *decorative,
        *bare,
        *loading,
        *bar,
        *selectable,
        *selected,
        *disabled,
        *lightbox,
      ],
      open: open.as_ref().map(Signal::id),
      load: (*progress, *force_bar_shown, *forced),
    })
  }
}

#[cfg(feature = "devtools")]
impl crate::app::component::DevtoolsInspectable for PictureProps {}

/// What a picture keeps between renders.
#[derive(Clone)]
struct Live {
  /// Whether the menu behind More is open.
  menu: Signal<bool>,
  /// The More button, which the menu hangs from.
  more: ElementRef,
  /// Whether its lightbox is open, where the app names no signal for it.
  viewer: Signal<bool>,
  /// Counts on when a load or a decode of the picture's comes further or ends, and when it is asked for again:
  /// the picture renders again and looks.
  tick: Signal<u64>,
  /// Counts `tick` on: what a load and a decode are given to call, the same one for the picture's life.
  wake: Action,
  /// The load of the picture's bytes while it is on its way, with its source's key: while it is held the app's
  /// loader keeps it.
  held: Arc<parking_lot::Mutex<Option<(u64, crate::resources::Load)>>>,
  /// The load of the small version that stands in for the picture, as `held` is the picture's.
  held_small: Arc<parking_lot::Mutex<Option<(u64, crate::resources::Load)>>>,
  /// What the app was last told of a source, by its key: that it came, or that it did not.
  told: Arc<parking_lot::Mutex<Option<(u64, bool)>>>,
  /// What to tell the app once this render is laid out.
  news: Arc<parking_lot::Mutex<Vec<Action>>>,
}

/// The component of a picture.
struct PictureHost {
  live: Live,
}

impl Component for PictureHost {
  type Props = PictureProps;

  fn create(ctx: &mut Ctx) -> Self {
    let tick = ctx.signal(0u64);
    let wake: Action = {
      let tick = tick.clone();
      Arc::new(move || tick.set(tick.get_untracked().wrapping_add(1)))
    };
    Self {
      live: Live {
        menu: ctx.signal(false),
        more: ElementRef::new(),
        viewer: ctx.signal(false),
        tick,
        wake,
        held: Arc::default(),
        held_small: Arc::default(),
        told: Arc::default(),
        news: Arc::default(),
      },
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let picture = ctx.props::<PictureProps>().0.clone();
    let theme = current_theme();
    // Read, so that a load that comes further renders the picture again.
    self.live.tick.get();
    let phase = self.phase(&picture, ctx);
    self.tell(&picture, &phase);
    let loader = ctx.app_handle().map(|app| app.loader().clone());
    #[cfg(feature = "raster")]
    let small = self.small(&picture, &phase, ctx);
    #[cfg_attr(not(feature = "raster"), allow(unused_mut))]
    let mut view = view(picture, &theme, &self.live, phase, loader, Some(ctx));
    #[cfg(feature = "raster")]
    {
      view.small = small;
    }
    view.render_themed()
  }

  /// The app hears of a picture that came or did not once the render that found it is laid out, not in it.
  fn after_layout(&self) {
    let news = std::mem::take(&mut *self.live.news.lock());
    news.iter().for_each(|tell| tell());
  }

  /// A picture that is gone waits for no load: the loader stops one nobody else holds.
  fn on_unmounted(&self) {
    *self.live.held.lock() = None;
    *self.live.held_small.lock() = None;
  }
}

impl PictureHost {
  /// Where the picture's pixels are now. A source to read and decode is brought on from here: asked of the
  /// app's loader and of the pixels' cache, each of which counts `tick` on when it has more.
  fn phase(&self, picture: &Picture, ctx: &Ctx) -> Phase {
    match &picture.source {
      Source::None => Phase::Empty,
      #[cfg(feature = "raster")]
      Source::Data(data) => Phase::Ready(data.clone()),
      #[cfg(not(feature = "image"))]
      Source::Encoded { .. } => {
        let _ = ctx;
        Phase::Failed(Arc::from(NO_CODECS))
      }
      #[cfg(feature = "image")]
      Source::Encoded { source, cap } => self.decoded(source, *cap, ctx, &self.live.held),
    }
  }

  /// The pixels of the small version that stands in for the picture while it is on its way, once they are
  /// there: the ones the app gave, or read and decoded as the picture is.
  #[cfg(feature = "raster")]
  fn small(&self, picture: &Picture, phase: &Phase, ctx: &Ctx) -> Option<crate::images::ImageData> {
    let waits = picture.loading || matches!(phase, Phase::Loading(_));
    let PicturePlaceholder::Blur(PictureSmall(small)) = &picture.placeholder else {
      return None;
    };
    if !waits {
      *self.live.held_small.lock() = None;
      return None;
    }
    match small {
      Source::Data(data) => Some(data.clone()),
      #[cfg(feature = "image")]
      Source::Encoded { source, cap } => match self.decoded(source, *cap, ctx, &self.live.held_small) {
        Phase::Ready(data) => Some(data),
        _ => None,
      },
      _ => {
        let _ = ctx;
        None
      }
    }
  }

  /// Where the pixels of `source` are, brought on from here; `held` keeps its load while it is on its way.
  #[cfg(feature = "image")]
  fn decoded(
    &self,
    source: &crate::resources::Source,
    cap: Option<(u32, u32)>,
    ctx: &Ctx,
    held: &parking_lot::Mutex<Option<(u64, crate::resources::Load)>>,
  ) -> Phase {
    use crate::{
      images::{ImageKey, ImageLoadState, ImageSizeCap, ImageSource, decoded_images},
      resources::{LoadState, SourceKind},
    };
    let key = ImageKey::new(source.cache_key());
    let cap = cap.map(|(width, height)| ImageSizeCap::new(width, height));
    let wake = &self.live.wake;
    let decode = |from: ImageSource| match decoded_images().load(key, cap, || from, Some(wake.clone())) {
      ImageLoadState::Loading => Phase::Loading(None),
      ImageLoadState::Ready(data) => Phase::Ready(data),
      ImageLoadState::Failed(error) => Phase::Failed(Arc::from(error.message())),
    };
    match &source.kind {
      // A file and bytes are read by what decodes them.
      SourceKind::Bytes(bytes) => decode(ImageSource::Bytes(bytes.clone())),
      SourceKind::File(path) => decode(ImageSource::File(path.clone())),
      // A resource and an address are read by the app's loader first.
      SourceKind::Resource(_) | SourceKind::Url(_) => {
        let mut held = held.lock();
        // Its pixels are there, or known to be no picture: nothing to load.
        match decoded_images().state(key, cap) {
          Some(ImageLoadState::Ready(data)) => {
            *held = None;
            return Phase::Ready(data);
          }
          Some(ImageLoadState::Failed(error)) => {
            *held = None;
            return Phase::Failed(Arc::from(error.message()));
          }
          _ => {}
        }
        let load = match &*held {
          Some((of, load)) if *of == source.cache_key() => load.clone(),
          _ => {
            let Some(app) = ctx.app_handle() else {
              return Phase::Failed(Arc::from("the picture is in no app: nothing loads it"));
            };
            let load = app.loader().load(source);
            *held = Some((source.cache_key(), load.clone()));
            load
          }
        };
        // Asked to be told before it is looked at: a load that ends in between is not missed.
        load.wake_with(wake.clone());
        match load.state() {
          LoadState::Loading { loaded, total } => Phase::Loading(total.map(|total| (loaded, total))),
          LoadState::Failed(error) => Phase::Failed(Arc::from(error.to_string())),
          LoadState::Ready(bytes) => decode(ImageSource::Bytes(bytes)),
        }
      }
    }
  }

  /// Notes what the app is to be told of this render's find: that the picture came, or that it did not, once
  /// for a source and again for one asked for anew.
  fn tell(&self, picture: &Picture, phase: &Phase) {
    let came = match phase {
      #[cfg(feature = "raster")]
      Phase::Ready(_) => true,
      Phase::Failed(_) => false,
      Phase::Empty | Phase::Loading(_) => return,
    };
    let key = match &picture.source {
      Source::Encoded { source, .. } => source.cache_key(),
      _ => 0,
    };
    let mut told = self.live.told.lock();
    if *told == Some((key, came)) {
      return;
    }
    *told = Some((key, came));
    let mut news = self.live.news.lock();
    match phase {
      Phase::Failed(reason) => news.extend(picture.on_error.iter().map(|handler| {
        let (handler, reason) = (handler.clone(), reason.clone());
        Arc::new(move || handler(reason.clone())) as Action
      })),
      _ => news.extend(picture.on_load.iter().cloned()),
    }
  }
}

/// A layer as large as the box it is in.
fn layer() -> Stack {
  Stack::new().size(Dimension::Pct(100.0), Dimension::Pct(100.0))
}

/// Which sides of the picture's box are known before its pixels are: the width, the height.
#[derive(Clone, Copy)]
struct Known {
  width: bool,
  height: bool,
}

impl Known {
  /// A box for what stands in for the pixels: as large as the picture's where that is known, and of the wide
  /// ratio on a side nothing tells.
  fn stand_in(self) -> Stack {
    let full = Dimension::Pct(100.0);
    let stack = Stack::new().stack_align(StackAlignment::Center);
    match (self.width, self.height) {
      (true, true) => stack.size(full, full),
      (true, false) => stack.width(full).aspect_ratio(WIDE),
      (false, true) => stack.height(full).aspect_ratio(WIDE),
      (false, false) => stack,
    }
  }
}

/// One form of a picture as its view holds it: the widths it lives at, and the style of its parts there.
#[derive(Clone)]
pub struct PictureForm {
  pub size: PictureSize,
  /// The widths of the picture at which this form shows; `None` for the one form of a picture, at every width.
  pub bounds: Option<SizeBounds>,
  /// The style of each part in this form.
  pub recipe: ResolvedRecipe,
  /// The style of each part of a button whose menu is open.
  on_recipe: ResolvedRecipe,
}

impl PictureForm {
  fn part(&self, part: PicturePart) -> ResolvedPart {
    self.recipe.part(part)
  }

  /// `layer` as a layer of the picture's box in this form: as large as the box came to be, and laid out only
  /// while the box has a width of this form.
  fn placed(&self, layer: Stack) -> Stack {
    layer.drop_outside(self.bounds.unwrap_or_default())
  }
}

/// What stands in the middle of a box with no picture: its glyph, and what it is called under it.
fn empty_center(form: &PictureForm, label: Option<&Arc<str>>) -> Element {
  let mark = glyph(&form.part(PicturePart::Mark), Name::new(icons::IMAGE), false);
  let mut center = middle(form).child(mark);
  if let (Some(label), PictureSize::Md | PictureSize::Lg) = (label, form.size) {
    center = center.child(wrapped(&form.part(PicturePart::Label), label));
  }
  center.into()
}

/// The column in the middle of the box, in the style of its part.
fn middle(form: &PictureForm) -> Column {
  Column::new()
    .part(&form.part(PicturePart::Center))
    .size(Dimension::Pct(100.0), Dimension::Pct(100.0))
    .align_items(Alignment::Center)
    .justify(Justify::Center)
}

/// `text` in the style of `part`, wrapping in the width it is given and centered in it.
fn wrapped(part: &ResolvedPart, text: &str) -> Element {
  Column::new()
    .width(Dimension::Pct(100.0))
    .align_items(Alignment::Center)
    .child(
      part
        .text(text)
        .text_align(TextAlign::Center)
        .width(Dimension::Pct(100.0))
        .part(part),
    )
    .into()
}

/// The rail of a load that is `progress` of 1 along, in the style of `track` and `fill`.
fn rail(track: &ResolvedPart, fill: &ResolvedPart, progress: f64) -> Row {
  Row::new().part(track).clip().child(
    Row::new()
      .part(fill)
      .size(Dimension::Pct(progress as f32 * 100.0), Dimension::Pct(100.0)),
  )
}

/// Builds the view of `picture` in `theme`, with its bar made, and its tooltips where `ctx` is given.
fn view(
  picture: Picture,
  theme: &Theme,
  live: &Live,
  phase: Phase,
  loader: Option<crate::resources::Loader>,
  mut ctx: Option<&mut Ctx>,
) -> PictureView {
  let pressable = picture.pressable();
  let bar_has = picture.title.is_some()
    || picture.description.is_some()
    || !picture.actions.is_empty()
    || !picture.on_download.is_empty()
    || !picture.menu_items.is_empty()
    || picture.lightbox;
  let menu_open = live.menu.get();
  let case = |size: PictureSize, on: bool| {
    Case {
      variant: None,
      size: Some(size.into()),
      density: None,
      breakpoint: super::current_breakpoint(),
      states: picture.states.clone(),
    }
    .state(PictureState::Bare, picture.bare)
    .state(PictureState::Circle, picture.shape == PictureShape::Circle)
    .state(PictureState::Pressable, pressable)
    .state(PictureState::Selectable, picture.selectable)
    .state(PictureState::Selected, picture.selected)
    .state(PictureState::Disabled, picture.disabled)
    .state(
      PictureState::Shown,
      picture.force_bar_shown.unwrap_or(menu_open) && picture.force_bar_shown != Some(false),
    )
    .state(PictureState::On, on)
  };
  let forms: Vec<PictureForm> = picture
    .forms(theme)
    .into_iter()
    .map(|(size, bounds)| PictureForm {
      size,
      bounds,
      recipe: theme.resolve::<Picture>(&case(size, false), Some(&picture.rules)),
      on_recipe: theme.resolve::<Picture>(&case(size, true), Some(&picture.rules)),
    })
    .collect();
  let id = |suffix: &str| picture.id.as_ref().map(|id| format!("{id}-{suffix}"));

  // The bar's own buttons, once for every form that has them: an element is built for one place.
  let mut bars = Vec::with_capacity(forms.len());
  for form in &forms {
    let shown = picture.bar && bar_has && picture.force_bar_shown != Some(false);
    if !shown || matches!(form.size, PictureSize::Xs | PictureSize::Sm) {
      bars.push(None);
      continue;
    }
    let large = form.size == PictureSize::Lg;
    let mut buttons: Vec<Element> = Vec::new();
    if large {
      buttons.extend(picture.actions.iter().cloned());
      if !picture.on_download.is_empty() {
        let handlers = picture.on_download.clone();
        let press: Action = Arc::new(move || handlers.iter().for_each(|handler| handler()));
        let button = control(
          form,
          false,
          icons::DOWNLOAD,
          &picture.words.download,
          id("download"),
          press,
        );
        buttons.push(tip(ctx.as_deref_mut(), button, &picture.words.download));
      }
    }
    // A compact picture keeps the menu's button alone: what its other buttons do is in the menu.
    let mut menu = None;
    if !picture.menu_items.is_empty() {
      let open = live.menu.clone();
      let press: Action = Arc::new(move || open.set(!open.get_untracked()));
      let button = control(
        form,
        menu_open,
        icons::MORE_HORIZONTAL,
        &picture.words.more,
        id("more"),
        press,
      );
      let button = Row::new().ref_element(live.more.clone()).child(button);
      buttons.push(tip(ctx.as_deref_mut(), button.into(), &picture.words.more));
      menu = Some(
        Menu::new(live.menu.clone(), live.more.clone())
          .label(&picture.words.more)
          .placement(Placement::TopEnd)
          .offset(4.0)
          .rules(&form.recipe.rules_for(PicturePart::MoreMenu, super::MenuPart::Panel))
          .items(picture.menu_items.iter().cloned())
          .build(theme),
      );
    }
    // Open larger is the last, where a video has fullscreen.
    if large && picture.lightbox {
      let open = picture.open.clone().unwrap_or_else(|| live.viewer.clone());
      let told = picture.on_open_change.clone();
      let press: Action = Arc::new(move || {
        if !open.get_untracked() {
          open.set(true);
          told.iter().for_each(|tell| tell(true));
        }
      });
      let button = control(form, false, icons::FULLSCREEN, &picture.words.open, id("open"), press);
      buttons.push(tip(ctx.as_deref_mut(), button, &picture.words.open));
    }
    bars.push(Some(PictureBar {
      title: picture.title.clone(),
      description: picture.description.clone().filter(|_| large),
      buttons,
      menu,
    }));
  }

  // Asked for again: the app is told, the pixels' cache and the loader forget that it failed, the load that
  // was held is let go, and the picture renders again and asks anew.
  let retry: Action = {
    let handlers = picture.on_retry.clone();
    let (source, live) = (picture.source.clone(), live.clone());
    Arc::new(move || {
      handlers.iter().for_each(|handler| handler());
      #[cfg(feature = "image")]
      if let Source::Encoded { source, cap } = &source {
        let cap = cap.map(|(width, height)| crate::images::ImageSizeCap::new(width, height));
        crate::images::decoded_images().forget(crate::images::ImageKey::new(source.cache_key()), cap);
        if let Some(loader) = &loader {
          loader.forget(source);
        }
      }
      #[cfg(not(feature = "image"))]
      let _ = (&source, &loader);
      *live.held.lock() = None;
      *live.told.lock() = None;
      (live.wake)();
    })
  };

  PictureView {
    phase,
    retry,
    viewer: picture.open.clone().unwrap_or_else(|| live.viewer.clone()),
    #[cfg(feature = "raster")]
    small: None,
    known: Known {
      width: picture.width.is_some() || picture.aspect_ratio.is_some(),
      height: picture.height.is_some() || picture.aspect_ratio.is_some(),
    },
    pressable,
    forms,
    bars,
    picture,
  }
}

/// A button of the bar: its glyph and its name, and on where its menu is open.
fn control(
  form: &PictureForm,
  on: bool,
  icon: &'static str,
  label: &str,
  id: Option<String>,
  press: Action,
) -> Element {
  let recipe = if on { &form.on_recipe } else { &form.recipe };
  let mut button = pressable(&recipe.part(PicturePart::Control), false)
    .describe("a11y_role", "button")
    .describe("a11y_name", Arc::<str>::from(label))
    .child(glyph(&recipe.part(PicturePart::ControlIcon), Name::new(icon), false))
    .on_click(move |event: MouseEvent| {
      // The press is the button's, not the picture's it lies over.
      event.stop_propagation();
      press();
    });
  if let Some(id) = id {
    button = button.id(id);
  }
  button.into()
}

/// `element` with a tooltip of `text` over it, where the picture is mounted.
fn tip(ctx: Option<&mut Ctx>, element: Element, text: &str) -> Element {
  match ctx {
    Some(ctx) => Tooltip::new(text)
      .placement(Placement::Top)
      .arrow(true)
      .offset(4.0)
      .mount(ctx, element),
    None => element,
  }
}

/// The bar of a picture in one of its forms: what it says and its buttons, made.
#[derive(Clone)]
pub struct PictureBar {
  pub title: Option<Arc<str>>,
  /// The line under the title; a compact picture has none.
  pub description: Option<Arc<str>>,
  /// The buttons at the bar's end, in order: the app's, download, More, open.
  pub buttons: Vec<Element>,
  /// The menu behind More, which takes no room, where the picture is mounted.
  pub menu: Option<Element>,
}

/// A picture as a renderer sees it: what it was given, and each of its forms with the style of its parts.
pub struct PictureView {
  /// The forms it is built in: one, or all of them for layout to show the one that fits its width.
  pub forms: Vec<PictureForm>,
  /// The bar of each form, where that form has one.
  pub bars: Vec<Option<PictureBar>>,
  /// Whether a press of it as a whole does something.
  pub pressable: bool,
  known: Known,
  picture: Picture,
  /// Where its pixels are.
  phase: Phase,
  /// Whether its lightbox is open: the app's signal, or the picture's own.
  viewer: Signal<bool>,
  /// The pixels of the small version that stands in for it, blurred, while it is on its way.
  #[cfg(feature = "raster")]
  small: Option<crate::images::ImageData>,
  /// Asks for the picture again.
  retry: Action,
}

impl PictureView {
  /// The form whose style the box itself takes: the only one, or the large one.
  fn base(&self) -> &PictureForm {
    self.forms.last().expect("a picture has a form")
  }

  /// The style of a part of the box, as this picture is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.base().recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.base().recipe.theme()
  }

  /// What the picture is called to tooling that reads the tree, if it is anything to it.
  pub fn label(&self) -> Option<Arc<str>> {
    (!self.picture.decorative)
      .then(|| self.picture.label.clone().or_else(|| self.picture.title.clone()))
      .flatten()
  }

  fn id(&self, suffix: &str) -> Option<String> {
    self.picture.id.as_ref().map(|id| format!("{id}-{suffix}"))
  }

  fn render_themed(self) -> Element {
    match self.theme().renderer::<Picture>() {
      Some(renderer) => renderer(self),
      None => self.render(),
    }
  }

  /// What stands in for the pixels while they are on their way, for each time it is shown: the sheen, or how
  /// far the load has come where the app says.
  fn loading_maker(&self) -> impl Fn(Option<(u64, u64)>) -> Element + Send + Sync + 'static {
    let (forms, known) = (self.forms.clone(), self.known);
    let said = self.picture.progress;
    let sheen = matches!(self.picture.placeholder, PicturePlaceholder::Skeleton);
    let words = self.picture.progress_text.clone();
    // The small version, blurred, in the place the pixels will have: over the whole box, under no rail.
    #[cfg(feature = "raster")]
    let blurred = self.small.clone().map(|data| {
      let (fit, (x, y)) = (fit_of(self.picture.fit), anchor_shares(self.picture.anchor));
      let part = self.part(PicturePart::Image);
      move || -> Element {
        #[allow(deprecated)]
        let image = crate::components::Image::new(data.clone())
          .fit(fit)
          .image_anchor(x, y)
          .part(&part)
          .size(Dimension::Pct(100.0), Dimension::Pct(100.0))
          .blur(BLUR)
          .transform(crate::node::transform::Transform2D::scale_uniform(BLUR_OVERDRAW));
        layer().child(image).into()
      }
    });
    move |bytes: Option<(u64, u64)>| {
      // What the app says of the load comes before what the loader knows.
      let load = match (said, bytes) {
        (Some(fraction), _) => Some(PictureLoad { fraction, bytes }),
        (None, Some((loaded, total))) if total > 0 => Some(PictureLoad {
          fraction: (loaded as f64 / total as f64).clamp(0.0, 1.0),
          bytes,
        }),
        _ => None,
      };
      let progress = load.map(|load| load.fraction);
      let text = match (load, &words) {
        (Some(load), Some(words)) => words(load),
        (Some(load), None) => load.text(),
        (None, _) => String::new(),
      };
      let mut stand_in = known.stand_in();
      #[cfg(feature = "raster")]
      if let Some(blurred) = &blurred {
        return stand_in.child(blurred()).into();
      }
      for form in &forms {
        let content: Option<Element> = match (progress, form.size) {
          (Some(progress), PictureSize::Lg | PictureSize::Md) => {
            let mut block = Column::new()
              .part(&form.part(PicturePart::Progress))
              .align_items(Alignment::Center)
              .child(rail(
                &form.part(PicturePart::Track),
                &form.part(PicturePart::Fill),
                progress,
              ));
            if form.size == PictureSize::Lg {
              block = block.child(form.part(PicturePart::ProgressLabel).text(&text).nowrap());
            }
            Some(middle(form).child(block).into())
          }
          (Some(progress), PictureSize::Sm) => Some(
            Column::new()
              .size(Dimension::Pct(100.0), Dimension::Pct(100.0))
              .justify(Justify::End)
              .child(rail(
                &form.part(PicturePart::ProgressLine),
                &form.part(PicturePart::Fill),
                progress,
              ))
              .into(),
          ),
          // As small as an avatar, the box alone says it loads.
          (_, PictureSize::Xs) => None,
          (None, _) if sheen => Some(
            Skeleton::block(Length::Fill, Length::Fill)
              .animation(SkeletonAnimation::Shimmer)
              .rules(&form.recipe.rules_for(PicturePart::Sheen, SkeletonPart::Bone))
              .build(form.recipe.theme()),
          ),
          (None, _) => None,
        };
        if let Some(content) = content {
          stand_in = stand_in.child(form.placed(layer().child(content)));
        }
      }
      stand_in.into()
    }
  }

  /// What stands in for a picture that did not load: its glyph, what it is called, that it did not load, and
  /// the button that asks for it again. A thumbnail is the glyph alone, and a press of it asks again.
  fn error_maker(&self) -> impl Fn() -> Element + Send + Sync + 'static {
    let (forms, known) = (self.forms.clone(), self.known);
    let (label, words) = (self.picture.label.clone(), self.picture.words.clone());
    // A build with no decoder shows what it has to decode as failed, and says why: here and once in the log.
    #[cfg(not(feature = "image"))]
    let words = {
      static SAID: std::sync::Once = std::sync::Once::new();
      SAID.call_once(|| tracing::warn!("{NO_CODECS}"));
      Words {
        error_description: Arc::from(NO_CODECS),
        ..words
      }
    };
    let retry_id = self.id("retry");
    let retry = self.retry.clone();
    move || {
      let mut stand_in = known.stand_in();
      for form in &forms {
        let icon = glyph(
          &form.part(PicturePart::ErrorIcon),
          Name::new(icons::ALERT_CIRCLE),
          false,
        );
        let content: Element = match form.size {
          PictureSize::Lg | PictureSize::Md => {
            let mut text = Column::new()
              .width(Dimension::Pct(100.0))
              .align_items(Alignment::Center);
            if let Some(label) = &label {
              text = text.child(wrapped(&form.part(PicturePart::ErrorLabel), label));
            }
            if form.size == PictureSize::Lg {
              text = text.child(wrapped(
                &form.part(PicturePart::ErrorDescription),
                &words.error_description,
              ));
            }
            let part = form.part(PicturePart::Retry);
            let press = retry.clone();
            let mut button = pressable(&part, false)
              .describe("a11y_role", "button")
              .describe("a11y_name", words.retry.clone())
              .child(part.text(&words.retry).nowrap())
              .on_click(move |event: MouseEvent| {
                event.stop_propagation();
                press();
              });
            if let Some(id) = &retry_id {
              button = button.id(id.clone());
            }
            middle(form).child(icon).child(text).child(button).into()
          }
          // A thumbnail is too small for words: the glyph, pressed to ask again, and its name says why.
          PictureSize::Sm | PictureSize::Xs => {
            let press = retry.clone();
            let name = match &label {
              Some(label) => format!("{label}. {} {}", words.error_description, words.retry),
              None => format!("{} {}", words.error_description, words.retry),
            };
            middle(form)
              .describe("a11y_role", "button")
              .describe("a11y_name", name)
              .cursor(CursorIcon::Pointer)
              .on_click(move |event: MouseEvent| {
                event.stop_propagation();
                press();
              })
              .child(icon)
              .into()
          }
        };
        stand_in = stand_in.child(form.placed(layer().child(content)));
      }
      stand_in.into()
    }
  }

  /// The pixels, placed in the box as the fit and the anchor say. Nothing while they are not there.
  #[allow(deprecated)]
  pub fn image(&self) -> Option<Element> {
    #[cfg(not(feature = "raster"))]
    return None;
    #[cfg(feature = "raster")]
    {
      use crate::components::Image;
      let Phase::Ready(data) = &self.phase else {
        return None;
      };
      let (x, y) = anchor_shares(self.picture.anchor);
      let image = Image::new(data.clone())
        .fit(fit_of(self.picture.fit))
        .image_anchor(x, y)
        .part(&self.part(PicturePart::Image));
      let full = Dimension::Pct(100.0);
      // As large as the box on the sides the box knows; on the others the pixels say.
      Some(match (self.known.width, self.known.height) {
        (true, true) => image.size(full, full).into(),
        (true, false) => image.width(full).into(),
        (false, true) => image.height(full).into(),
        (false, false) => image.into(),
      })
    }
  }

  /// What a box with no source shows: in each form, the glyph and what the picture would be called.
  pub fn empty(&self) -> Element {
    let mut stand_in = self.known.stand_in();
    for form in &self.forms {
      let content = empty_center(form, self.picture.label.as_ref());
      stand_in = stand_in.child(form.placed(layer().child(content)));
    }
    stand_in.into()
  }

  /// The veil and the count over it, where the picture stands for more of them.
  pub fn count(&self) -> Vec<Element> {
    let Some(count) = &self.picture.count else {
      return Vec::new();
    };
    let mut layers = vec![
      layer()
        .part(&self.part(PicturePart::Veil))
        .drop_outside(SizeBounds::new())
        .pointer_events_none()
        .into(),
    ];
    for form in self.forms.iter().filter(|form| form.size != PictureSize::Xs) {
      let text = form.part(PicturePart::Count).text(count).nowrap();
      layers.push(
        form
          .placed(layer().pointer_events_none().child(middle(form).child(text)))
          .into(),
      );
    }
    layers
  }

  /// The app's layer over the picture.
  pub fn overlay(&self) -> Option<Element> {
    if self.picture.children.is_empty() {
      return None;
    }
    let mut overlay = layer()
      .part(&self.part(PicturePart::Overlay))
      .drop_outside(SizeBounds::new())
      .hit_test(HitTestBehavior::ContentOnly)
      .with_children(self.picture.children.iter().cloned());
    if let Some(id) = self.id("overlay") {
      overlay = overlay.id(id);
    }
    Some(overlay.into())
  }

  /// The row along the top of each form that has one: the badge at its start, the select mark at its end.
  pub fn top(&self) -> Vec<Element> {
    let picture = &self.picture;
    let marked = picture.selectable || picture.selected;
    let mut rows = Vec::new();
    for form in &self.forms {
      let badge = picture
        .badge
        .clone()
        .filter(|_| matches!(form.size, PictureSize::Lg | PictureSize::Md));
      let mark = (marked && form.size != PictureSize::Xs).then(|| {
        let mut mark = Row::new()
          .part(&form.part(PicturePart::Select))
          .align_items(Alignment::Center)
          .justify(Justify::Center);
        if picture.selected {
          mark = mark.child(glyph(
            &form.part(PicturePart::SelectIcon),
            Name::new(icons::CHECK),
            false,
          ));
        }
        mark
      });
      if badge.is_none() && mark.is_none() {
        continue;
      }
      let mut row = Row::new()
        .part(&form.part(PicturePart::Top))
        .width(Dimension::Pct(100.0))
        .align_items(Alignment::Start);
      if let Some(badge) = badge {
        row = row.child(badge);
      }
      row = row.child(Row::new().flex(1.0));
      if let Some(mark) = mark {
        row = row.child(mark);
      }
      rows.push(
        form
          .placed(
            layer()
              .stack_align(StackAlignment::TopStart)
              .pointer_events_none()
              .child(row),
          )
          .into(),
      );
    }
    rows
  }

  /// The bar along the bottom of each form that has one: the title and the description at its start, the
  /// buttons at its end, on a gradient.
  pub fn controls(&self) -> Vec<Element> {
    let mut bars = Vec::new();
    for (form, bar) in self.forms.iter().zip(&self.bars) {
      let Some(bar) = bar else {
        continue;
      };
      let part = form.part(PicturePart::Controls);
      let end = part
        .rest()
        .background
        .clone()
        .unwrap_or(crate::node::BackgroundColor::Color(Color::new(0, 0, 0, 0xB3)));
      let clear = crate::node::BackgroundColor::Color(Color::new(0, 0, 0, 0));
      let mut text = Column::new()
        .part(&form.part(PicturePart::Text))
        .flex(1.0)
        .min_width(0.0);
      if let Some(title) = &bar.title {
        let part = form.part(PicturePart::Title);
        text = text.child(part.text(title).width(Dimension::Pct(100.0)).part(&part));
      }
      if let Some(description) = &bar.description {
        let part = form.part(PicturePart::Description);
        text = text.child(part.text(description).width(Dimension::Pct(100.0)).part(&part));
      }
      let buttons = Row::new()
        .part(&form.part(PicturePart::Actions))
        .align_items(Alignment::Center)
        .overflow_visible()
        .with_children(bar.buttons.iter().cloned());
      let mut controls = Row::new()
        .part(&part)
        .width(Dimension::Pct(100.0))
        .align_items(Alignment::End)
        .overflow_visible()
        .background_gradient(crate::node::Gradient::linear(
          180.0,
          [(clear, 0.0), (end, 1.0)].map(|(color, at)| crate::node::gradient::GradientStop::at(color, at)),
        ))
        .child(text)
        .child(buttons);
      if let Some(menu) = &bar.menu {
        controls = controls.child(menu.clone());
      }
      if let Some(id) = self.id("controls") {
        controls = controls.id(id);
      }
      bars.push(
        form
          .placed(
            layer()
              .stack_align(StackAlignment::BottomStart)
              .hit_test(HitTestBehavior::ContentOnly)
              .child(controls),
          )
          .into(),
      );
    }
    bars
  }

  /// The line around the box, over what is in it. A bare picture has none.
  pub fn edge(&self) -> Option<Element> {
    (!self.picture.bare).then(|| {
      layer()
        .part(&self.part(PicturePart::Edge))
        .drop_outside(SizeBounds::new())
        .pointer_events_none()
        .into()
    })
  }

  /// What a press of the picture as a whole does: tells the app's handlers, and what it is to be where it can
  /// be chosen.
  fn press(&self) -> impl Fn(MouseEvent) + Send + Sync + 'static {
    let (clicks, changes) = (self.picture.on_click.clone(), self.picture.on_change.clone());
    let (selectable, next) = (self.picture.selectable, !self.picture.selected);
    // A picture that is chosen by a press opens nothing by it.
    let opens =
      (self.picture.lightbox && !selectable).then(|| (self.viewer.clone(), self.picture.on_open_change.clone()));
    move |event: MouseEvent| {
      for handler in &clicks {
        handler.call(&event);
      }
      if selectable {
        changes.iter().for_each(|handler| handler(next));
      }
      if let Some((open, told)) = &opens
        && !open.get_untracked()
      {
        open.set(true);
        told.iter().for_each(|tell| tell(true));
      }
    }
  }

  /// The picture's box, with everything in it: the pixels or what stands in for them, the app's layer, the
  /// veil and the count, the top row, the bar and the line around it.
  pub fn frame(&self) -> Element {
    let picture = &self.picture;
    let theme = self.theme();
    let full = Dimension::Pct(100.0);
    let mut inner = Stack::new().stack_align(StackAlignment::Center);
    // The stack is as large as the box where the box says a size, and as the pixels where it does not.
    if self.known.width {
      inner = inner.width(full);
    }
    if self.known.height {
      inner = inner.height(full);
    }
    // A box whose size does not wait for the pixels lets what stood in for them go out over them: it is the
    // second child, kept while it leaves, and the pixels are the first, under it. A box the pixels size has
    // one child, which is gone when they come.
    let fades = self.known.width && self.known.height;
    let waiting = |stand_in: Element| -> Element {
      if !fades {
        return stand_in;
      }
      Stack::new()
        .size(full, full)
        .key("stand-in")
        .part(&self.part(PicturePart::StandIn))
        .child(stand_in)
        .into()
    };
    let (pixels, over) = match (&self.phase, self.image()) {
      (_, Some(image)) if !picture.loading => (Some(image), None),
      // What stands there in place of the picture's own stand-ins, whatever the reason there are no pixels.
      _ if picture.fallback.is_some() => (
        None,
        picture.fallback.as_ref().map(|fallback| {
          // In the middle of the box, as what it stands in for would be.
          let centered = Stack::new()
            .stack_align(StackAlignment::Center)
            .size(full, full)
            .child(fallback());
          waiting(centered.into())
        }),
      ),
      // What the app is still fetching itself: the stand-in of a load, with no pixels to come by themselves.
      _ if picture.loading => (None, Some(waiting(self.loading_maker()(None)))),
      (_, Some(image)) => (Some(image), None),
      (Phase::Loading(bytes), _) => (None, Some(waiting(self.loading_maker()(*bytes)))),
      (Phase::Failed(_), _) => (None, Some(self.error_maker()())),
      _ => (None, Some(self.empty())),
    };
    inner = if fades {
      inner.presence().child(pixels.unwrap_or_default()).with_children(over)
    } else {
      inner.with_children(pixels.or(over))
    };
    if let Some(overlay) = self.overlay() {
      inner = inner.child(overlay);
    }
    inner = inner
      .with_children(self.count())
      .with_children(self.top())
      .with_children(self.controls());
    if let Some(edge) = self.edge() {
      inner = inner.child(edge);
    }
    // Its lightbox, which takes no room: the picture as the app gave it, alone.
    if picture.lightbox {
      let told = picture.on_open_change.clone();
      let mut viewer = super::Lightbox::new(self.viewer.clone(), [picture.clone()])
        .on_dismiss(move || told.iter().for_each(|tell| tell(false)))
        .key("viewer");
      if let Some(label) = self.label() {
        viewer = viewer.label(&label);
      }
      if let Some(id) = self.id("viewer") {
        viewer = viewer.id(id);
      }
      inner = inner.child(viewer);
    }

    let part = self.part(PicturePart::Root).forced(picture.forced);
    let role = match (picture.selectable, self.pressable) {
      (true, _) => "checkbox",
      (false, true) => "button",
      (false, false) => "img",
    };
    // The box is what the pointer and the focus are in: the bar and the ring of the mark show by it.
    let mut root: Element = if self.pressable {
      pressable(&part, false)
        .clip()
        .interaction_group(true)
        .on_click(self.press())
        .child(inner)
        .into()
    } else {
      Row::new()
        .part(&part)
        .clip()
        .interaction_group(true)
        .disabled(picture.disabled)
        .child(inner)
        .into()
    };
    let node = &mut *root.node;
    let width = picture.width.and_then(|width| width.resolve(theme));
    let height = picture.height.and_then(|height| height.resolve(theme));
    if let Some(width) = width {
      NodeUpdate::width(node, width);
      // The width said is the most the box takes: in a room narrower than it the box is as wide as the room.
      NodeUpdate::max_width(node, Dimension::Pct(100.0));
    }
    match (width, height, picture.aspect_ratio) {
      // A box of so many pixels by so many keeps its shape where the room makes it narrower: its height follows
      // its width. The ratio is a hair under the box's own and the height said is the most, so that at its
      // full width the box is as high as said, and not a rounding of it.
      (Some(Dimension::Px(wide)), Some(Dimension::Px(high)), None) if wide > 0.0 && high > 0.0 => {
        NodeUpdate::aspect_ratio(node, wide / high * (1.0 - 4.0 * f32::EPSILON));
        NodeUpdate::max_height(node, high);
      }
      (_, height, ratio) => {
        if let Some(height) = height {
          NodeUpdate::height(node, height);
        }
        if let Some(ratio) = ratio {
          NodeUpdate::aspect_ratio(node, ratio);
        }
      }
    }
    match self.label() {
      Some(label) => {
        NodeUpdate::describe(node, Arc::from("a11y_role"), Arc::from(role));
        NodeUpdate::describe(node, Arc::from("a11y_name"), label);
      }
      None if role != "img" => NodeUpdate::describe(node, Arc::from("a11y_role"), Arc::from(role)),
      None => {}
    }
    if picture.selectable {
      let checked = if picture.selected { "true" } else { "false" };
      NodeUpdate::describe(node, Arc::from("a11y_checked"), Arc::from(checked));
    }
    if let Some(id) = &picture.id {
      NodeUpdate::id(node, id.clone());
    }
    // A figure is keyed as a whole: its box is then one child of it among two.
    if let (Some(key), None) = (&picture.key, &picture.caption) {
      node.set_component_key(Some(key));
    }
    root
  }

  /// How the default renderer builds a picture: its box, and its caption under it where it has one.
  pub fn render(self) -> Element {
    let frame = self.frame();
    let Some(caption) = &self.picture.caption else {
      return frame;
    };
    let part = self.part(PicturePart::Caption);
    let mut figure = Column::new()
      .part(&self.part(PicturePart::Figure))
      .align_items(Alignment::Center)
      .describe("a11y_role", "figure")
      .child(frame)
      .child(wrapped(&part, caption));
    if let Some(width) = self.picture.width.and_then(|width| width.resolve(self.theme())) {
      figure = figure.width(width).max_width(Dimension::Pct(100.0));
    }
    if let Some(key) = &self.picture.key {
      figure = figure.key(key.clone());
    }
    figure.into()
  }
}

/// How the engine fills a box with pixels for `fit`.
#[cfg(feature = "raster")]
fn fit_of(fit: PictureFit) -> crate::node::BackgroundSize {
  use crate::node::BackgroundSize;
  match fit {
    PictureFit::Cover => BackgroundSize::Cover,
    PictureFit::Contain => BackgroundSize::Contain,
    PictureFit::Fill => BackgroundSize::Stretch,
    PictureFit::None => BackgroundSize::Natural,
  }
}

/// Where `anchor` holds a picture in its box, as shares of the room across and down.
#[cfg(feature = "raster")]
fn anchor_shares(anchor: StackAlignment) -> (f32, f32) {
  use StackAlignment::*;
  match anchor {
    TopStart => (0.0, 0.0),
    TopCenter => (0.5, 0.0),
    TopEnd => (1.0, 0.0),
    CenterStart => (0.0, 0.5),
    Center => (0.5, 0.5),
    CenterEnd => (1.0, 0.5),
    BottomStart => (0.0, 1.0),
    BottomCenter => (0.5, 1.0),
    BottomEnd => (1.0, 1.0),
  }
}

impl Themed for Picture {
  const NAME: &'static str = "picture";
  type Renderer = PictureRenderer;

  /// The design's picture (`pencil/lurq-design-system.pen`, Picture): a box in the muted fill with the corners
  /// of a surface and a line of the border's color around it, over the pixels. Over the picture everything is
  /// white on black in both themes, as on the video: a bar along the bottom on a gradient to 70% black, 24 above
  /// its row, 12 beside it and 8 under it, with a title of 14 medium, a line of 13 at 60% white, and buttons of
  /// 36 with glyphs of 20; a compact one has 20, 8 and 6 around a title of 13 and a button of 28 with a glyph of
  /// 16. The box with no picture, the error, the rail of a load and the mark follow the theme: the error is a
  /// glyph of 24, the picture's name at 14 medium, a line of 13 and an outlined button of 32; the rail is 160 by
  /// 4 in the input's color with the primary on it; the mark is a disc of 20 in the primary color, 12 from the
  /// top and the end, and 16 at 6 on a thumbnail. A chosen picture has a line of 2 in the primary color outside
  /// it; the focus draws the ring.
  fn recipe() -> Recipe {
    use PaletteColor::{Background, Foreground, Input, Muted, MutedForeground, Primary, PrimaryForeground, StateHover};
    use PicturePart::*;
    use PictureSize::{Md, Sm, Xs};
    use PictureState::{Bare, Circle, Disabled, On, Pressable, Selectable, Selected, Shown};
    let white = Color::new(255, 255, 255, 255);
    let white_60 = Color::new(255, 255, 255, 0x99);
    let white_20 = Color::new(255, 255, 255, 0x33);
    let white_12 = Color::new(255, 255, 255, 0x1F);
    let black_40 = Color::new(0, 0, 0, 0x66);
    let black_60 = Color::new(0, 0, 0, 0x99);
    let black_70 = Color::new(0, 0, 0, 0xB3);
    let appear = |s: super::PartStyle| {
      s.transition(PartTransition::new(Transition::opacity(), MotionDuration::Base).easing(MotionEasing::Enter))
    };
    Recipe::new()
      .base(Figure, |s| s.gap(8.0))
      .base(Root, |s| s.radius(RadiusSize::Lg).background(Muted))
      .size(Sm, Root, |s| s.radius(RadiusSize::Md))
      .size(Xs, Root, |s| s.radius(RadiusSize::Md))
      .state(Circle, Root, |s| s.radius(RadiusSize::Full))
      .state(Bare, Root, |s| s.radius(0.0).background(Color::new(0, 0, 0, 0)))
      .state(Pressable, Root, |s| s.cursor(CursorIcon::Pointer))
      .state(Selected, Root, |s| s.outline(Outline::new(2.0, Primary)))
      .state(Disabled, Root, |s| {
        s.opacity(OpacityLevel::Disabled).cursor(CursorIcon::Default)
      })
      .when(Selector::new().state(Pressable).focus_visible(), Root, |s| {
        s.focus_ring()
      })
      .base(Edge, |s| {
        s.radius(RadiusSize::Lg)
          .border(Border::inside(1.0, PaletteColor::Border))
      })
      .size(Sm, Edge, |s| s.radius(RadiusSize::Md))
      .size(Xs, Edge, |s| s.radius(RadiusSize::Md))
      .state(Circle, Edge, |s| s.radius(RadiusSize::Full))
      .base(StandIn, |s| {
        s.exit(PartAnimation::new(
          super::FADE_OUT,
          MotionDuration::Base,
          MotionEasing::Exit,
        ))
      })
      .base(Sheen, |s| s.radius(0.0))
      .base(Veil, |s| s.background(black_60))
      // What stands in the middle is centered, so the room over and under it is the least it keeps: a box only
      // just higher than what it says shows all of it.
      .base(Center, |s| s.padding_x(24.0).padding_y(8.0).gap(12.0))
      .size(Md, Center, |s| s.padding_x(12.0).padding_y(12.0).gap(8.0))
      .size(Sm, Center, |s| s.padding_x(8.0).padding_y(8.0))
      .size(Xs, Center, |s| s.padding_x(0.0).padding_y(0.0))
      .base(Mark, |s| s.square(28.0).foreground(MutedForeground))
      .size(Md, Mark, |s| s.square(24.0))
      .size(Sm, Mark, |s| s.square(20.0))
      .size(Xs, Mark, |s| s.square(16.0))
      .base(Label, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(13.0)
          .line_height(1.4)
          .foreground(MutedForeground)
      })
      .base(ErrorIcon, |s| s.square(24.0).foreground(MutedForeground))
      .size(Sm, ErrorIcon, |s| s.square(20.0))
      .size(Xs, ErrorIcon, |s| s.square(16.0))
      .base(ErrorLabel, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(14.0)
          .weight(FontWeight::Medium)
          .line_height(1.4)
          .foreground(Foreground)
          .padding_bottom(4.0)
      })
      .size(Md, ErrorLabel, |s| s.font_size(13.0).padding_bottom(0.0))
      .base(ErrorDescription, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(13.0)
          .foreground(MutedForeground)
      })
      .base(Retry, |s| {
        fading(
          s.height(32.0)
            .padding_x(16.0)
            .radius(RadiusSize::Md)
            .background(Background)
            .border(Border::inside(1.0, Input))
            .foreground(Foreground)
            .typography(TypographyStyle::Body)
            .font_size(13.0)
            .weight(FontWeight::Medium)
            .cursor(CursorIcon::Pointer),
        )
      })
      .size(Md, Retry, |s| s.height(28.0).padding_x(12.0).font_size(12.0))
      .hovered(Retry, |s| s.veil(StateHover))
      .focus_visible(Retry, |s| s.focus_ring())
      .base(Progress, |s| s.gap(8.0))
      .base(Track, |s| {
        s.width(160.0).height(4.0).radius(RadiusSize::Full).background(Input)
      })
      .size(Md, Track, |s| s.width(96.0))
      .base(Fill, |s| s.background(Primary))
      .base(ProgressLabel, |s| {
        s.typography(TypographyStyle::Mono)
          .font_size(11.0)
          .foreground(MutedForeground)
      })
      .base(ProgressLine, |s| s.width(Length::Fill).height(2.0).background(Input))
      .base(Count, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(20.0)
          .weight(FontWeight::SemiBold)
          .foreground(white)
      })
      .size(Sm, Count, |s| s.font_size(13.0))
      .base(Top, |s| s.padding_x(12.0).padding_y(12.0))
      .size(Sm, Top, |s| s.padding_x(6.0).padding_y(6.0))
      .base(Select, |s| {
        appear(
          s.square(20.0)
            .radius(RadiusSize::Full)
            .background(black_40)
            .border(Border::inside(1.5, white))
            .opacity(0.0),
        )
      })
      .size(Sm, Select, |s| s.square(16.0))
      .when(Selector::new().state(Selectable).within(), Select, |s| s.opacity(1.0))
      .when(Selector::new().state(Selectable).state(Shown), Select, |s| {
        s.opacity(1.0)
      })
      .state(Selected, Select, |s| {
        s.background(Primary)
          .border(Border::inside(1.5, PrimaryForeground))
          .opacity(1.0)
      })
      .base(SelectIcon, |s| s.square(12.0).foreground(PrimaryForeground))
      .size(Sm, SelectIcon, |s| s.square(10.0))
      .base(Controls, |s| {
        appear(
          s.padding_top(24.0)
            .padding_x(12.0)
            .padding_bottom(8.0)
            .gap(12.0)
            .background(black_70)
            .opacity(0.0),
        )
      })
      .size(Md, Controls, |s| {
        s.padding_top(20.0).padding_x(8.0).padding_bottom(6.0).gap(8.0)
      })
      .within(Controls, |s| s.opacity(1.0))
      .state(Shown, Controls, |s| s.opacity(1.0))
      .base(Text, |s| s.gap(2.0).padding_start(4.0).padding_bottom(4.0))
      .size(Md, Text, |s| s.padding_bottom(6.0))
      .base(Title, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(14.0)
          .weight(FontWeight::Medium)
          .foreground(white)
      })
      .size(Md, Title, |s| s.font_size(13.0))
      .base(Description, |s| {
        s.typography(TypographyStyle::Body).font_size(13.0).foreground(white_60)
      })
      .base(Actions, |s| s.gap(4.0))
      .base(Control, |s| {
        fading(
          s.square(36.0)
            .radius(RadiusSize::Md)
            .foreground(white)
            .cursor(CursorIcon::Pointer),
        )
      })
      .size(Md, Control, |s| s.square(28.0))
      .hovered(Control, |s| s.background(white_12))
      .focus_visible(Control, |s| s.outline(Outline::new(2.0, white)))
      .state(On, Control, |s| s.background(white_20))
      .base(ControlIcon, |s| s.square(20.0))
      .size(Md, ControlIcon, |s| s.square(16.0))
      .base(MoreMenu, |s| s.width(216.0))
      .base(Caption, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(13.0)
          .font_style(FontStyle::Italic)
          .line_height(1.4)
          .foreground(MutedForeground)
      })
  }
}
