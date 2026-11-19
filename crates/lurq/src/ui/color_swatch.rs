//! A color swatch: one color, shown as a small square or circle, over a checkerboard where it is not opaque.

use std::sync::Arc;

use super::{
  names,
  part_style::{Length, PartStyle},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::{fading, pressable},
  theme::{Themed, current_theme},
};
use crate::{
  app::theme::{OpacityLevel, PaletteColor, RadiusSize, Theme},
  components::{Column, Row, Stack},
  layout::layout_kind::Justify,
  node::{
    CursorIcon, Element, EventHandler, IntoMouseEventHandler, color::Color, dimension::Dimension, outline::Outline,
  },
};

/// The parts of a color swatch a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColorSwatchPart {
  /// The swatch: its side, its corners, its edge, what is pressed and focused.
  Root,
  /// The checkerboard under a color that is not opaque: its light squares, and the side of a square.
  Checker,
  /// The dark squares of the checkerboard.
  CheckerCell,
}

names!(ColorSwatchPart {
  Root => "root",
  Checker => "checker",
  CheckerCell => "checker_cell",
});

/// The states of a color swatch a recipe styles, besides what the pointer and the keyboard do to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColorSwatchState {
  /// It is the one chosen: a ring in the foreground around it.
  Selected,
  /// It is a circle.
  Round,
  /// It is pressed: it has `on_click`.
  Pressable,
  /// It takes no input.
  Disabled,
}

names!(ColorSwatchState {
  Selected => "selected",
  Round => "round",
  Pressable => "pressable",
  Disabled => "disabled",
});

/// The sizes of a color swatch: the side of the square.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ColorSwatchSize {
  /// 20: the swatches of a color picker.
  Sm,
  /// 24: the board's swatch, and those of a swatch picker.
  #[default]
  Md,
  /// 32.
  Lg,
}

names!(ColorSwatchSize {
  Sm => "sm",
  Md => "md",
  Lg => "lg",
});

/// The shape of a color swatch.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ColorSwatchShape {
  /// A square with the small radius: the board's swatch.
  #[default]
  Square,
  /// A circle: the swatches under a color picker's rails.
  Round,
}

/// What builds a color swatch's nodes: a theme's in place of the default one.
pub type ColorSwatchRenderer = Arc<dyn Fn(ColorSwatchView) -> Element + Send + Sync>;

/// One color, as a small square or circle: a color an app has, a choice among colors, what a color field holds.
///
/// ```
/// use lurq::{node::color::Color, ui::{ColorSwatch, ColorSwatchShape, ColorSwatchSize}};
///
/// let brand = ColorSwatch::new(Color::from_hex("#2563EB")).label("Brand blue");
/// // A color that is not opaque stands over a checkerboard.
/// let veil = ColorSwatch::new(Color::from_hex("#2563EB80")).shape(ColorSwatchShape::Round).size(ColorSwatchSize::Lg);
/// // A swatch that is pressed is a button.
/// let pick = ColorSwatch::new(Color::from_hex("#22C55E")).on_click(|_| println!("green"));
/// ```
///
/// To what reads the tree it is an `img` called by its label, or by its color's hex where it has none; one that has
/// `on_click` is a `button`, a stop of Tab, with the ring of the focus from the keyboard.
pub struct ColorSwatch {
  color: Color,
  label: Option<Arc<str>>,
  size: Name,
  shape: ColorSwatchShape,
  selected: bool,
  disabled: bool,
  on_click: Vec<EventHandler<crate::app::events::MouseEvent>>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl ColorSwatch {
  /// A swatch of `color`.
  pub fn new(color: impl Into<Color>) -> Self {
    Self {
      color: color.into(),
      label: None,
      size: ColorSwatchSize::default().into(),
      shape: ColorSwatchShape::default(),
      selected: false,
      disabled: false,
      on_click: Vec::new(),
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// What the swatch is called, to what reads the tree: "Brand blue". Its color's hex where it is not given.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// A [`ColorSwatchSize`]: `Sm` 20, `Md` 24 (the default), `Lg` 32. Or a size the theme's recipe names.
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.size = size.into();
    self
  }

  /// A square with the small radius (the default) or a circle.
  pub fn shape(mut self, shape: ColorSwatchShape) -> Self {
    self.shape = shape;
    self
  }

  /// Whether it is the one chosen: a ring of 2 in the foreground around it.
  pub fn selected(mut self, selected: bool) -> Self {
    self.selected = selected;
    self
  }

  /// Whether it takes no input: dimmed, and not pressed.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Called with the event of a press, or of Enter or Space while it has the focus: the swatch is then a button.
  /// Each call adds a handler.
  pub fn on_click(mut self, handler: impl IntoMouseEventHandler) -> Self {
    self.on_click.push(handler.into_event_handler());
    self
  }

  /// Puts the swatch in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this swatch over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this swatch: what the component it is in says of it in this place.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the swatch as it is under the pointer, held down or focused from the keyboard, with nothing pointing at
  /// it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the swatch's node.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the swatch in `theme`. Converting a swatch into an element does this in the theme of what is being
  /// rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let pressable = !self.on_click.is_empty();
    let case = Case {
      variant: None,
      size: Some(self.size),
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(ColorSwatchState::Selected, self.selected)
    .state(ColorSwatchState::Round, self.shape == ColorSwatchShape::Round)
    .state(ColorSwatchState::Pressable, pressable)
    .state(ColorSwatchState::Disabled, self.disabled);
    let view = ColorSwatchView {
      color: self.color,
      label: self.label.unwrap_or_else(|| Arc::from(self.color.to_hex_upper())),
      selected: self.selected,
      disabled: self.disabled,
      recipe: theme
        .resolve::<ColorSwatch>(&case, Some(&self.rules))
        .owned_by(self.id.as_ref()),
      on_click: self.on_click,
      forced: self.forced,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<ColorSwatch>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<ColorSwatch> for Element {
  fn from(swatch: ColorSwatch) -> Self {
    swatch.build(&current_theme())
  }
}

/// A color swatch as a renderer sees it: its color, the style of its parts, and its behavior ready to be dressed.
pub struct ColorSwatchView {
  pub color: Color,
  /// What it is called to what reads the tree: its label, or its color's hex.
  pub label: Arc<str>,
  pub selected: bool,
  pub disabled: bool,
  /// The style of each part of the swatch.
  pub recipe: ResolvedRecipe,
  on_click: Vec<EventHandler<crate::app::events::MouseEvent>>,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl ColorSwatchView {
  /// The style of a part, as the swatch is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// What the swatch holds: its color over all of it, over the checkerboard where it is not opaque.
  pub fn fill(&self) -> Element {
    let root = self.part(ColorSwatchPart::Root).rest();
    let side = |length: Option<Length>| match length.and_then(|length| length.resolve(self.theme())) {
      Some(Dimension::Px(px)) => px,
      _ => 24.0,
    };
    let (width, height) = (side(root.width), side(root.height));
    color_over_checker(
      self.color,
      width,
      height,
      &self.part(ColorSwatchPart::Checker),
      &self.part(ColorSwatchPart::CheckerCell),
    )
  }

  /// How the default renderer builds a swatch: a box in the style of the root holding its fill, pressed where it has
  /// `on_click`.
  pub fn render(self) -> Element {
    let part = self.part(ColorSwatchPart::Root);
    let fill = self.fill();
    if self.on_click.is_empty() {
      let mut root = Row::new()
        .part(&part)
        .child(fill)
        .describe("a11y_role", "img")
        .describe("a11y_name", self.label.clone());
      if let Some(id) = &self.id {
        root = root.id(id.clone());
      }
      if let Some(key) = &self.key {
        root = root.key(key.clone());
      }
      return root.into();
    }
    let mut root = pressable(&part, self.disabled)
      .justify(Justify::Start)
      .child(fill)
      .describe("a11y_role", "button")
      .describe("a11y_name", self.label.clone());
    if self.selected {
      root = root.describe("a11y_pressed", "true");
    }
    if !self.disabled {
      for handler in &self.on_click {
        root = root.on_click(handler.clone());
      }
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

/// `color` over a box of `width` by `height`: over a checkerboard of `checker`'s light squares and `cell`'s dark ones
/// where it is not opaque, alone where it is. What a swatch and a picker's rail of alpha are filled with.
pub(crate) fn color_over_checker(
  color: Color,
  width: f32,
  height: f32,
  checker: &ResolvedPart,
  cell: &ResolvedPart,
) -> Element {
  let color_layer = Row::new().size(width, height).background(color);
  if color.a() == 255 {
    return color_layer.into();
  }
  Stack::new()
    .size(width, height)
    .child(checkerboard(width, height, checker, cell))
    .child(color_layer)
    .into()
}

/// A checkerboard of `width` by `height`: `checker`'s fill under all of it, and `cell`'s squares, of `cell`'s side, on
/// every other square.
pub(crate) fn checkerboard(width: f32, height: f32, checker: &ResolvedPart, cell: &ResolvedPart) -> Element {
  let side = match cell.rest().width.and_then(|length| length.resolve(checker.theme())) {
    Some(Dimension::Px(px)) if px > 0.0 => px,
    _ => 4.0,
  };
  let (columns, rows) = ((width / side).ceil() as usize, (height / side).ceil() as usize);
  let mut board = Column::new().size(width, height).part(checker);
  for row in 0..rows {
    let mut line = Row::new().height(side);
    for column in 0..columns {
      let square = Row::new().size(side, side);
      line = line.child(if (row + column) % 2 == 1 {
        square.part(cell)
      } else {
        square
      });
    }
    board = board.child(line);
  }
  board.into()
}

impl crate::ui::Styled for ColorSwatch {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for ColorSwatch {
  const NAME: &'static str = "color_swatch";
  type Renderer = ColorSwatchRenderer;

  fn parts() -> crate::ui::Declared {
    ColorSwatchPart::declared()
  }

  fn states() -> crate::ui::Declared {
    ColorSwatchState::declared()
  }

  /// The design's swatch (`pencil/lurq-design-system.pen`, Color Swatch): 24 square with the small radius and an
  /// edge of 1 in `Border` inside it, filled with its color. Selected, as the swatch picker draws it, a ring of 2 in
  /// `Foreground` around it. The board draws no swatch focused or disabled: the focus is the theme's ring, and a
  /// disabled swatch is dimmed as every control is. The checkerboard under a color that is not opaque is of squares
  /// of 4, `Background` and `Muted`.
  fn recipe() -> Recipe {
    use ColorSwatchPart::{Checker, CheckerCell, Root};
    use PaletteColor::*;
    Recipe::new()
      .base(Root, |s| {
        fading(s)
          .square(24.0)
          .radius(RadiusSize::Sm)
          .border(crate::node::border::Border::inside(1.0, Border))
          .cursor(CursorIcon::Default)
      })
      .base(Checker, |s| s.background(Background))
      .base(CheckerCell, |s| s.square(4.0).background(Muted))
      .size(ColorSwatchSize::Sm, Root, |s| s.square(20.0))
      .size(ColorSwatchSize::Lg, Root, |s| s.square(32.0))
      .state(ColorSwatchState::Round, Root, |s| s.radius(RadiusSize::Full))
      .state(ColorSwatchState::Selected, Root, |s| {
        s.outline(Outline::new(2.0, Foreground))
      })
      .state(ColorSwatchState::Pressable, Root, |s| s.cursor(CursorIcon::Pointer))
      .focus_visible(Root, |s| s.focus_ring())
      .state(ColorSwatchState::Disabled, Root, |s| {
        s.opacity(OpacityLevel::Disabled).cursor(CursorIcon::Default)
      })
  }
}
