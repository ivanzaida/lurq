//! A picture: an image in its box, with what it shows while it loads and when it cannot be shown, and what lies
//! over it.

use std::sync::Arc;

use super::{
  Menu, MenuItem, Skeleton, SkeletonAnimation, SkeletonPart, Tooltip, icons, names,
  part_style::{Length, PartTransition},
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
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PicturePlaceholder {
  /// The skeleton's sheen over the box.
  #[default]
  Skeleton,
  /// The box alone.
  None,
}

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
  #[cfg(feature = "image")]
  Resource(Arc<str>),
  #[cfg(feature = "image")]
  Decoded {
    source: crate::images::ImageSource,
    key: crate::images::ImageKey,
    cap: Option<crate::images::ImageSizeCap>,
  },
}

/// The words a picture writes itself.
#[derive(Clone)]
struct Words {
  error_description: Arc<str>,
  retry: Arc<str>,
  download: Arc<str>,
  more: Arc<str>,
}

impl Default for Words {
  fn default() -> Self {
    Self {
      error_description: Arc::from("The picture didn't load."),
      retry: Arc::from("Try again"),
      download: Arc::from("Download"),
      more: Arc::from("More"),
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
/// A picture laid in the tree is still: its bar shows and its buttons are pressed, but the menu behind More and
/// the buttons' tooltips are kept by a mounted one, [`mount`](Self::mount).
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
  progress: Option<f64>,
  progress_text: Option<Arc<dyn Fn(f64) -> String + Send + Sync>>,
  words: Words,
  on_retry: Vec<Action>,
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
      progress: None,
      progress_text: None,
      words: Words::default(),
      on_retry: Vec::new(),
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

  /// A picture loaded as a resource of the app: a path under its resource root, or an address.
  #[cfg(feature = "image")]
  pub fn from_resource(path: &str) -> Self {
    Self::of(Source::Resource(Arc::from(path)))
  }

  /// A picture decoded from encoded bytes (PNG, JPEG, the first frame of a GIF or WebP, BMP, TIFF), off the UI
  /// thread. The bytes are hashed for the key its pixels are kept under, unless [`cache_key`](Self::cache_key)
  /// gives one.
  #[cfg(feature = "image")]
  pub fn from_bytes(bytes: impl Into<Arc<[u8]>>) -> Self {
    let bytes: Arc<[u8]> = bytes.into();
    let key = crate::images::ImageKey::of_bytes(&bytes);
    Self::from_source(crate::images::ImageSource::Bytes(bytes), key)
  }

  /// A picture read from a file and decoded, off the UI thread. Its key is the path with the file's size and
  /// the time it was changed.
  #[cfg(feature = "image")]
  pub fn from_file(path: impl Into<std::path::PathBuf>) -> Self {
    let path = path.into();
    let key = crate::images::ImageKey::of_path(&path);
    Self::from_source(crate::images::ImageSource::File(path), key)
  }

  /// A picture decoded from `source`, its pixels kept under `key`: neither hashed nor copied.
  #[cfg(feature = "image")]
  pub fn from_source(source: crate::images::ImageSource, key: crate::images::ImageKey) -> Self {
    Self::of(Source::Decoded { source, key, cap: None })
  }

  /// The key a decoded picture's pixels are kept under: the same key is the same pixels.
  #[cfg(feature = "image")]
  pub fn cache_key(mut self, key: crate::images::ImageKey) -> Self {
    if let Source::Decoded { key: held, .. } = &mut self.source {
      *held = key;
    }
    self
  }

  /// Decodes no larger than `width` by `height` pixels, keeping the ratio and never enlarging: a thumbnail of
  /// a large photo does not keep all of it.
  #[cfg(feature = "image")]
  pub fn downscale_to(mut self, width: u32, height: u32) -> Self {
    if let Source::Decoded { cap, .. } = &mut self.source {
      *cap = Some(crate::images::ImageSizeCap::new(width, height));
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

  /// How far the load has come, of 1, where the app knows: a rail in the middle of the box in place of the
  /// sheen, and along the bottom edge of a small picture. `None` for a load that cannot tell.
  pub fn progress(mut self, progress: impl Into<Option<f64>>) -> Self {
    self.progress = progress.into().map(|progress| progress.clamp(0.0, 1.0));
    self
  }

  /// The words under the rail of a load, from how far it has come, of 1: "42%" unless told.
  pub fn progress_text(mut self, text: impl Fn(f64) -> String + Send + Sync + 'static) -> Self {
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

  /// Shows the bar, or hides it, whatever the pointer and the focus do: for a picture that always says what it
  /// is, and for a gallery's page of states.
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

  /// A count over a veil: "+5" on the last picture of a row that could not hold them all.
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

  /// Builds the picture in `theme`: the still form, with its menu closed. Converting a picture into an element
  /// does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    view(self, theme, &Live::still(), None).render_themed()
  }

  /// The picture mounted where it is declared: it keeps whether the menu behind More is open, and its buttons
  /// have their tooltips.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let key = self.key.clone();
    let props = PictureProps(self);
    match key {
      Some(key) => ctx.mount_keyed::<PictureHost>(&key, props),
      None => ctx.mount::<PictureHost>(props),
    }
  }

  /// Whether a press of the picture as a whole does something.
  fn pressable(&self) -> bool {
    !self.disabled && (self.selectable || !self.on_click.is_empty())
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

impl From<Picture> for Element {
  fn from(picture: Picture) -> Self {
    picture.build(&current_theme())
  }
}

/// A picture to mount.
#[derive(Clone)]
struct PictureProps(Picture);

/// A picture is built anew by every render of what holds it, so props are never the same as before.
impl PartialEq for PictureProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

#[cfg(feature = "devtools")]
impl crate::app::component::DevtoolsInspectable for PictureProps {}

/// What a mounted picture keeps between renders; the still form makes a new one each time.
#[derive(Clone)]
struct Live {
  /// Whether the menu behind More is open.
  menu: Signal<bool>,
  /// The More button, which the menu hangs from.
  more: ElementRef,
  mounted: bool,
}

impl Live {
  fn still() -> Self {
    Self {
      menu: Signal::new(false),
      more: ElementRef::new(),
      mounted: false,
    }
  }
}

/// What keeps a mounted picture's menu.
struct PictureHost {
  live: Live,
}

impl Component for PictureHost {
  type Props = PictureProps;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      live: Live {
        menu: ctx.signal(false),
        more: ElementRef::new(),
        mounted: true,
      },
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let picture = ctx.props::<PictureProps>().0.clone();
    let theme = current_theme();
    view(picture, &theme, &self.live, Some(ctx)).render_themed()
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
fn view(picture: Picture, theme: &Theme, live: &Live, mut ctx: Option<&mut Ctx>) -> PictureView {
  let pressable = picture.pressable();
  let bar_has = picture.title.is_some()
    || picture.description.is_some()
    || !picture.actions.is_empty()
    || !picture.on_download.is_empty()
    || !picture.menu_items.is_empty();
  let menu_open = live.mounted && live.menu.get();
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
      if live.mounted {
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
    }
    bars.push(Some(PictureBar {
      title: picture.title.clone(),
      description: picture.description.clone().filter(|_| large),
      buttons,
      menu,
    }));
  }

  PictureView {
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
  /// The buttons at the bar's end, in order: the app's, download, More.
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
  #[cfg_attr(not(feature = "image"), allow(dead_code))]
  fn loading_maker(&self) -> impl Fn() -> Element + Send + Sync + 'static {
    let (forms, known) = (self.forms.clone(), self.known);
    let (progress, placeholder) = (self.picture.progress, self.picture.placeholder);
    let text = match (progress, &self.picture.progress_text) {
      (Some(progress), Some(text)) => text(progress),
      (Some(progress), None) => format!("{}%", (progress * 100.0).round()),
      (None, _) => String::new(),
    };
    move || {
      let mut stand_in = known.stand_in();
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
          (None, _) if placeholder == PicturePlaceholder::Skeleton => Some(
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
  #[cfg(feature = "image")]
  fn error_maker(&self) -> impl Fn() -> Element + Send + Sync + 'static {
    let (forms, known) = (self.forms.clone(), self.known);
    let (label, words) = (self.picture.label.clone(), self.picture.words.clone());
    let retry_id = self.id("retry");
    let retry: Action = {
      let handlers = self.picture.on_retry.clone();
      let source = self.picture.source.clone();
      Arc::new(move || {
        handlers.iter().for_each(|handler| handler());
        if let Source::Decoded { key, cap, .. } = &source {
          crate::images::decoded_images().forget(*key, *cap);
        }
      })
    };
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

  /// The pixels, placed in the box as the fit and the anchor say, with what stands in for them while they are
  /// on their way and when they did not come. Nothing for a picture with no source.
  #[allow(deprecated)]
  pub fn image(&self) -> Option<Element> {
    #[cfg(not(feature = "raster"))]
    return None;
    #[cfg(feature = "raster")]
    {
      use crate::{components::Image, node::BackgroundSize};
      let image = match &self.picture.source {
        Source::None => return None,
        Source::Data(data) => Image::new(data.clone()),
        #[cfg(feature = "image")]
        Source::Resource(path) => Image::from_resource(path),
        #[cfg(feature = "image")]
        Source::Decoded { source, key, cap } => {
          let image = Image::from_source(source.clone(), *key);
          let image = match cap {
            Some(cap) => image.downscale_to(cap.width, cap.height),
            None => image,
          };
          let error = self.error_maker();
          image
            .placeholder_view(self.loading_maker())
            .error_view(move |_| error())
        }
      };
      let (x, y) = anchor_shares(self.picture.anchor);
      let image = image
        .fit(match self.picture.fit {
          PictureFit::Cover => BackgroundSize::Cover,
          PictureFit::Contain => BackgroundSize::Contain,
          PictureFit::Fill => BackgroundSize::Stretch,
          PictureFit::None => BackgroundSize::Natural,
        })
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
    move |event: MouseEvent| {
      for handler in &clicks {
        handler.call(&event);
      }
      if selectable {
        changes.iter().for_each(|handler| handler(next));
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
    inner = match self.image() {
      Some(image) => inner.child(image),
      None => inner.child(self.empty()),
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
    if let Some(width) = picture.width.and_then(|width| width.resolve(theme)) {
      NodeUpdate::width(node, width);
    }
    if let Some(height) = picture.height.and_then(|height| height.resolve(theme)) {
      NodeUpdate::height(node, height);
    }
    if let Some(ratio) = picture.aspect_ratio {
      NodeUpdate::aspect_ratio(node, ratio);
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
      figure = figure.width(width);
    }
    if let Some(key) = &self.picture.key {
      figure = figure.key(key.clone());
    }
    figure.into()
  }
}

/// Where `anchor` holds a picture in its box, as shares of the room across and down.
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
      .base(Sheen, |s| s.radius(0.0))
      .base(Veil, |s| s.background(black_60))
      .base(Center, |s| s.padding_x(24.0).padding_y(24.0).gap(12.0))
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
