//! A QR code: a text an app gives, drawn as a code a phone's camera reads.

mod encode;

use std::sync::Arc;

pub use encode::QrMatrix;

use super::{
  names,
  part_style::Length,
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    theme::{BorderSize, PaletteColor, RadiusSize, Theme},
    window::Appearance,
  },
  components::{Column, Row, Stack},
  layout::{Alignment, StackAlignment, layout_kind::Justify},
  node::{BackgroundColor, Element, border::Border, color::Color, dimension::Dimension},
};

/// How much of a QR code can be lost and the code still read: a smudge, a crease, a logo over its middle.
/// Each level takes more modules for the same text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum QrLevel {
  /// About 7% of the code.
  Low,
  /// About 15%: what a code on a screen needs.
  #[default]
  Medium,
  /// About 25%.
  Quartile,
  /// About 30%: a code printed where it wears, or with a logo over its middle.
  High,
}

names!(QrLevel {
  Low => "low",
  Medium => "medium",
  Quartile => "quartile",
  High => "high",
});

impl QrLevel {
  /// Its place in the tables of the standard, from `Low` to `High`.
  fn index(self) -> usize {
    match self {
      Self::Low => 0,
      Self::Medium => 1,
      Self::Quartile => 2,
      Self::High => 3,
    }
  }

  /// The two bits the format of a code names the level by.
  fn format_bits(self) -> u8 {
    match self {
      Self::Low => 1,
      Self::Medium => 0,
      Self::Quartile => 3,
      Self::High => 2,
    }
  }
}

/// The parts of a QR code a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum QrCodePart {
  /// The card the code is drawn on: its edge, its corners, its padding, and its fill, the light of the code.
  Root,
  /// A dark module: its fill.
  Module,
  /// The well of the light fill a logo stands in, over the middle of the code.
  Media,
}

names!(QrCodePart {
  Root => "root",
  Module => "module",
  Media => "media",
});

/// The states of a QR code a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum QrCodeState {
  /// It has a logo over its middle.
  Media,
}

names!(QrCodeState {
  Media => "media",
});

/// What builds a QR code's nodes: a theme's in place of the default one.
pub type QrCodeRenderer = Arc<dyn Fn(QrCodeView) -> Element + Send + Sync>;

/// The side of a QR code where neither the app nor the recipe says one.
const DEFAULT_LENGTH: f32 = 160.0;

/// How much of the side of the code a logo's well covers, at most: what the highest level still reads under.
const MEDIA_SHARE: f32 = 0.22;

/// A QR code: a text an app gives (an address, a code to pair a device, a ticket), drawn as a code a phone's
/// camera reads.
///
/// ```
/// use lurq::ui::{QrCode, QrLevel};
///
/// let pair = QrCode::new("https://lurq.dev/pair?code=7F3K-92QD").label("Pair your phone");
/// // A larger one, that still reads with a quarter of it covered or worn.
/// let ticket = QrCode::new("TICKET-2026-11-16-0042").level(QrLevel::Quartile).length(220.0);
/// ```
///
/// It is drawn in the light and the dark look alike, dark modules on the light fill of a card, as the design
/// draws it: a phone reads a code dark on light, and not every phone reads it the other way. Its colors are the
/// roles of its recipe read in the theme's light palette (`background` and `foreground`), whatever the
/// appearance; its edge is the window's. The modules are whole logical pixels and are drawn on whole device
/// pixels (`snap_to_pixels`), so they are crisp at 100%, 150% and 200%.
///
/// The code is the smallest that holds the text's UTF-8 bytes at the level asked for, and at a higher level
/// where that fits in the same size. A text too long for the level asked for is drawn at the highest level
/// that holds it; one that no level holds (more than 2953 bytes) draws the card empty, and tooling still reads
/// the text. To tooling it is an image called by its label, whose description is the text.
#[derive(Clone)]
pub struct QrCode {
  text: Arc<str>,
  level: QrLevel,
  quiet_zone: usize,
  length: Option<Length>,
  media: Option<Element>,
  label: Option<Arc<str>>,
  /// The words it was told to speak in, in place of the app's.
  locale: Option<crate::core::DisplayWords>,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl QrCode {
  /// The code of `text`.
  pub fn new(text: &str) -> Self {
    Self {
      text: Arc::from(text),
      level: QrLevel::default(),
      quiet_zone: 0,
      length: None,
      media: None,
      label: None,
      locale: None,
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// How much of the code can be lost and it still read: `Medium` unless the app says another. A code with a
  /// logo is made at `High` whatever this says.
  pub fn level(mut self, level: QrLevel) -> Self {
    self.level = level;
    self
  }

  /// How many modules of light the code has around it inside the card's padding: none unless the app says. The
  /// card's padding of 10 is light already, and is the quiet zone a phone needs where the modules are small;
  /// a code of large modules, or one printed on a page, takes a quiet zone of 2 to 4.
  pub fn quiet_zone(mut self, modules: usize) -> Self {
    self.quiet_zone = modules;
    self
  }

  /// The side of the code, its card and padding included: 160 unless the app or a recipe says another. The
  /// modules are as large as fit in whole logical pixels, and the code stands in the middle of what is left.
  pub fn length(mut self, length: impl Into<Length>) -> Self {
    self.length = Some(length.into());
    self
  }

  /// A logo over the middle of the code, in a well of the light fill: the code is made at the highest level,
  /// which still reads with a fifth of its side covered. The well is that fifth, or the logo's size with its
  /// padding where the logo is larger: a logo larger than a fifth of the side may leave a code no phone reads.
  pub fn media(mut self, media: impl Into<Element>) -> Self {
    self.media = Some(media.into());
    self
  }

  /// What tooling calls the code: the locale's "QR code" unless the app says what it is for ("Pair your phone").
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// The words this code speaks in, in place of the app's ([`App::set_locale`](crate::app::App::set_locale)): what
  /// tooling calls it. A label told by itself stays what it was told.
  pub fn locale(mut self, locale: &crate::core::Locale) -> Self {
    self.locale = Some(locale.display.clone());
    self
  }

  /// Puts the code in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this code over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this code.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// The id of the root.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the code in `theme`. Converting a code into an element does this in the theme of what is being
  /// rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(QrCodeState::Media, self.media.is_some());
    let mut rules = Recipe::new();
    if let Some(length) = self.length {
      rules = rules.base(QrCodePart::Root, |s| s.square(length));
    }
    let rules = rules.extend(&self.rules);
    let asked = if self.media.is_some() {
      QrLevel::High
    } else {
      self.level
    };
    // A text too long for the level asked for is drawn at the highest that holds it.
    let matrix = [QrLevel::High, QrLevel::Quartile, QrLevel::Medium, QrLevel::Low]
      .into_iter()
      .filter(|level| level.index() <= asked.index())
      .find_map(|level| QrMatrix::encode(&self.text, level));
    let view = QrCodeView {
      // Its own locale's word, else the app's, read where it is built.
      label: self.label.unwrap_or_else(|| match self.locale {
        Some(words) => words.qr_code,
        None => super::current_locale().display.qr_code.clone(),
      }),
      text: self.text,
      matrix,
      quiet_zone: self.quiet_zone,
      media: self.media,
      recipe: theme.resolve::<QrCode>(&case, Some(&rules)).owned_by(self.id.as_ref()),
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<QrCode>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<QrCode> for Element {
  fn from(code: QrCode) -> Self {
    code.build(&current_theme())
  }
}

/// A run of dark modules a renderer draws as one box: its first column and row, and how many of each it covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QrRect {
  pub x: usize,
  pub y: usize,
  pub width: usize,
  pub height: usize,
}

/// A QR code as a renderer sees it.
pub struct QrCodeView {
  /// What tooling calls it.
  pub label: Arc<str>,
  /// What it encodes: its description to tooling.
  pub text: Arc<str>,
  /// Its modules; `None` where no level holds the text.
  pub matrix: Option<QrMatrix>,
  /// The modules of light around the code, inside the card's padding.
  pub quiet_zone: usize,
  /// The logo over its middle.
  pub media: Option<Element>,
  /// The style of each part of this code.
  pub recipe: ResolvedRecipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl QrCodeView {
  /// The style of a part, as this code is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The fill of a part, read in the theme's light palette: a code is dark on light in either look.
  pub fn light_fill(&self, part: impl Into<Name>) -> Option<Color> {
    let theme = self.theme();
    self.part(part).rest().background.map(|fill| match fill {
      BackgroundColor::Color(color) => color,
      BackgroundColor::Palette(role) => theme.palette_color_for(Appearance::Light, role),
      BackgroundColor::CurrentForeground => theme.palette_color_for(Appearance::Light, PaletteColor::Foreground),
    })
  }

  /// The dark modules as boxes: each run of dark modules in a row, and the same run in the rows under it, is one.
  pub fn rects(&self) -> Vec<QrRect> {
    let Some(matrix) = &self.matrix else {
      return Vec::new();
    };
    let size = matrix.size();
    let mut rects: Vec<QrRect> = Vec::new();
    // The boxes still open, which the next row may make taller: by the run they are of.
    let mut open: Vec<usize> = Vec::new();
    for y in 0..size {
      let mut runs = Vec::new();
      let mut x = 0;
      while x < size {
        if matrix.dark(x, y) {
          let start = x;
          while x < size && matrix.dark(x, y) {
            x += 1;
          }
          runs.push((start, x - start));
        } else {
          x += 1;
        }
      }
      let mut still_open = Vec::new();
      for (start, width) in runs {
        match open
          .iter()
          .find(|&&index| rects[index].x == start && rects[index].width == width)
        {
          Some(&index) => {
            rects[index].height += 1;
            still_open.push(index);
          }
          None => {
            rects.push(QrRect {
              x: start,
              y,
              width,
              height: 1,
            });
            still_open.push(rects.len() - 1);
          }
        }
      }
      open = still_open;
    }
    rects
  }

  /// The side of the card and the room inside its padding, in logical pixels.
  fn room(&self) -> (f32, f32) {
    let theme = self.theme();
    let rest = self.part(QrCodePart::Root).rest();
    let side = match rest.width.and_then(|width| width.resolve(theme)) {
      Some(Dimension::Px(px)) => px,
      _ => DEFAULT_LENGTH,
    };
    let [top, end, bottom, start] = rest
      .paddings()
      .map(|padding| padding.map_or(0.0, |padding| padding.resolve(&theme.spacing(), 0.0)));
    (side, (side - start - end).min(side - top - bottom).max(0.0))
  }

  /// How large a module is: as large as fits in whole logical pixels, and never less than one.
  pub fn module(&self) -> f32 {
    let Some(matrix) = &self.matrix else {
      return 0.0;
    };
    let modules = (matrix.size() + 2 * self.quiet_zone) as f32;
    (self.room().1 / modules).floor().max(1.0)
  }

  /// The modules in the room inside the card's padding, the code in its middle: a box for each run of dark
  /// modules, laid where it is.
  pub fn code(&self) -> Stack {
    let (_, room) = self.room();
    let mut code = Stack::new().size(room, room);
    let Some(matrix) = &self.matrix else {
      return code;
    };
    let module = self.module();
    let inset = ((room - module * matrix.size() as f32) / 2.0).max(0.0);
    let part = self.part(QrCodePart::Module);
    let fill = self.light_fill(QrCodePart::Module);
    for rect in self.rects() {
      let mut cell = Row::new().part(&part).absolute(
        inset + rect.x as f32 * module,
        inset + rect.y as f32 * module,
        Dimension::Px(rect.width as f32 * module),
        Dimension::Px(rect.height as f32 * module),
      );
      if let Some(fill) = fill {
        cell = cell.background(fill);
      }
      code = code.child(cell);
    }
    code
  }

  /// The well a logo stands in, in the middle of the code: at least an odd number of modules across, a fifth of
  /// the code's side, so that it sits on the code's middle module, and as large as the logo with its padding.
  pub fn well(&self, media: Element) -> Row {
    let (_, room) = self.room();
    let modules = self.matrix.as_ref().map_or(0, QrMatrix::size);
    let mut across = ((modules as f32 * MEDIA_SHARE) as usize).max(1);
    if across % 2 == 0 {
      across -= 1;
    }
    let side = (across as f32 * self.module()).min(room);
    let mut well = Row::new()
      .part(&self.part(QrCodePart::Media))
      .min_width(side)
      .min_height(side)
      .align_items(Alignment::Center)
      .justify(Justify::Center)
      .child(media);
    if let Some(fill) = self.light_fill(QrCodePart::Media) {
      well = well.background(fill);
    }
    well
  }

  /// How the default renderer builds a code: the card, its fill the light of the code, with the modules on it
  /// and a logo over their middle. All of it is drawn on whole device pixels. To tooling it is an image called
  /// by its label, described by its text.
  pub fn render(mut self) -> Element {
    let media = self.media.take();
    let mut code = Stack::new().stack_align(StackAlignment::Center).child(self.code());
    if let Some(media) = media {
      code = code.child(self.well(media));
    }
    let mut root = Column::new()
      .part(&self.part(QrCodePart::Root))
      .align_items(Alignment::Center)
      .justify(Justify::Center)
      .snap_to_pixels(true)
      .describe("a11y_role", "img")
      .describe("a11y_name", self.label.clone())
      .describe("a11y_description", self.text.clone())
      .child(code);
    if let Some(fill) = self.light_fill(QrCodePart::Root) {
      root = root.background(fill);
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root.into()
  }
}

impl crate::ui::Styled for QrCode {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for QrCode {
  const NAME: &'static str = "qr_code";
  type Renderer = QrCodeRenderer;

  fn parts() -> crate::ui::Declared {
    QrCodePart::declared()
  }

  fn states() -> crate::ui::Declared {
    QrCodeState::declared()
  }

  /// The design's QR code (`pencil/lurq-design-system.pen`, QR Code): a card with an edge of 1 inside it in the
  /// border's color, `radius-lg` round, 10 of padding, white, with modules of near black on it, in the light
  /// look and in the dark one alike: the roles `background` and `foreground` read in the light palette. The
  /// board draws eleven modules of 7, a picture of a code; a code of a real text has 21 or more, so the card is
  /// 160 unless the app says another side. The board draws no logo: its well is the card's fill, `radius-sm`
  /// round, with 2 of padding.
  fn recipe() -> Recipe {
    use PaletteColor::{Background, Foreground};
    use QrCodePart::{Media, Module, Root};
    Recipe::new()
      .base(Root, |s| {
        s.square(DEFAULT_LENGTH)
          .padding_x(10.0)
          .padding_y(10.0)
          .radius(RadiusSize::Lg)
          .border(Border::inside(BorderSize::Sm, PaletteColor::Border))
          .background(Background)
      })
      .base(Module, |s| s.background(Foreground))
      .base(Media, |s| {
        s.background(Background)
          .radius(RadiusSize::Sm)
          .padding_x(2.0)
          .padding_y(2.0)
      })
  }
}
