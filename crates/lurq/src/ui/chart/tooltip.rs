//! The tooltip of a chart's point: the board's Chart Tooltip, a title and a row for each series.

use std::sync::Arc;

use super::super::{
  names,
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe},
  theme::{Themed, current_theme},
};
use crate::{
  app::theme::{BorderSize, PaletteColor, RadiusSize, ShadowStyle, Theme, TypographyStyle},
  components::{Column, Row},
  layout::{Alignment, text_style::FontWeight},
  node::{BackgroundColor, Element, border::Border as Edge, dimension::Dimension, node_kind::TextOverflow},
  ui::part_style::FontFamily,
};

/// The parts of a chart's tooltip a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChartTooltipPart {
  /// The panel: the popover's fill, its edge, its corners, its shadow.
  Panel,
  /// What the point is: a category, a date.
  Title,
  /// A series' row: its dot, its name, its value.
  Row,
  /// The dot in the series' color.
  Key,
  /// The series' name.
  Label,
  /// The series' value at the point.
  Value,
}

names!(ChartTooltipPart {
  Panel => "panel",
  Title => "title",
  Row => "row",
  Key => "key",
  Label => "label",
  Value => "value",
});

/// One row of a tooltip: a series' color, its name and its value as written.
#[derive(Clone, Debug, PartialEq)]
pub struct ChartTooltipRow {
  pub color: BackgroundColor,
  pub name: Arc<str>,
  pub value: Arc<str>,
}

impl ChartTooltipRow {
  pub fn new(color: impl Into<BackgroundColor>, name: &str, value: &str) -> Self {
    Self {
      color: color.into(),
      name: Arc::from(name),
      value: Arc::from(value),
    }
  }
}

/// What renders a tooltip: a theme's in place of the default.
pub type ChartTooltipRenderer = Arc<dyn Fn(ChartTooltipView) -> Element + Send + Sync>;

/// The tooltip of a chart's point, as the board draws it: what the point is over a row for each series, its dot,
/// its name and its value. A chart shows it at the point under the pointer or the focus; it stands on its own too.
///
/// ```
/// use lurq::{app::theme::PaletteColor, ui::{ChartTooltip, ChartTooltipRow}};
///
/// let tooltip = ChartTooltip::new("Build 0.59")
///   .row(ChartTooltipRow::new(PaletteColor::Chart1, "wgpu", "6.9 ms"))
///   .row(ChartTooltipRow::new(PaletteColor::Chart2, "DirectX 12", "9.4 ms"));
/// ```
pub struct ChartTooltip {
  title: Arc<str>,
  rows: Vec<ChartTooltipRow>,
  rules: Recipe,
  states: Vec<Name>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl ChartTooltip {
  pub fn new(title: &str) -> Self {
    Self {
      title: Arc::from(title),
      rows: Vec::new(),
      rules: Recipe::new(),
      states: Vec::new(),
      id: None,
      key: None,
    }
  }

  /// A row more.
  pub fn row(mut self, row: ChartTooltipRow) -> Self {
    self.rows.push(row);
    self
  }

  /// Rows more.
  pub fn rows(mut self, rows: impl IntoIterator<Item = ChartTooltipRow>) -> Self {
    self.rows.extend(rows);
    self
  }

  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  pub fn style(
    mut self,
    part: impl Into<Name>,
    style: impl FnOnce(super::super::PartStyle) -> super::super::PartStyle,
  ) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  pub fn build(self, theme: &Theme) -> Element {
    let mut case = Case::new().breakpoint(super::super::current_breakpoint());
    case.states.extend(self.states.iter().copied());
    let view = ChartTooltipView {
      title: self.title,
      rows: self.rows,
      recipe: theme.resolve::<ChartTooltip>(&case, Some(&self.rules)),
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<ChartTooltip>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<ChartTooltip> for Element {
  fn from(tooltip: ChartTooltip) -> Self {
    tooltip.build(&current_theme())
  }
}

/// A tooltip as a renderer sees it.
pub struct ChartTooltipView {
  pub title: Arc<str>,
  pub rows: Vec<ChartTooltipRow>,
  pub recipe: ResolvedRecipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl ChartTooltipView {
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// A row: the dot, the name, which gives way and ends in an ellipsis, and the value.
  pub fn row(&self, row: &ChartTooltipRow) -> Row {
    Row::new()
      .part(&self.part(ChartTooltipPart::Row))
      .align_items(Alignment::Center)
      .child(
        Row::new()
          .part(&self.part(ChartTooltipPart::Key))
          .background(row.color.clone()),
      )
      .child(
        self
          .part(ChartTooltipPart::Label)
          .text(&row.name)
          .part(&self.part(ChartTooltipPart::Label))
          .nowrap()
          .text_overflow(TextOverflow::Elipsis)
          .flex_full(1.0, 1.0, Some(0.0))
          .min_width(Dimension::Px(0.0)),
      )
      .child(
        self
          .part(ChartTooltipPart::Value)
          .text(&row.value)
          .part(&self.part(ChartTooltipPart::Value))
          .nowrap(),
      )
  }

  /// How the default renderer builds a tooltip: the panel with the title over the rows. It is a tooltip to what
  /// reads the tree, named by its title.
  pub fn render(self) -> Element {
    let mut panel = Column::new()
      .part(&self.part(ChartTooltipPart::Panel))
      .align_items(Alignment::Stretch)
      .describe("a11y_role", "tooltip")
      .describe("a11y_name", self.title.clone())
      .child(
        self
          .part(ChartTooltipPart::Title)
          .text(&self.title)
          .part(&self.part(ChartTooltipPart::Title))
          .nowrap(),
      );
    for row in &self.rows {
      panel = panel.child(self.row(row));
    }
    if let Some(id) = &self.id {
      panel = panel.id(id.clone());
    }
    if let Some(key) = &self.key {
      panel = panel.key(key.clone());
    }
    panel.into()
  }
}

impl crate::ui::Styled for ChartTooltip {
  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for ChartTooltip {
  const NAME: &'static str = "chart_tooltip";
  type Renderer = ChartTooltipRenderer;

  fn parts() -> crate::ui::Declared {
    ChartTooltipPart::declared()
  }

  /// The board's Chart Tooltip (`hYUgv`): a panel at least 140 wide on the popover's fill with an edge of 1, the
  /// medium radius and the medium shadow, padded 8 and 10 with 6 between its rows; the title 12/600 in the popover's
  /// foreground; each row a dot of 7, the name 11 in the muted foreground taking the room, the value mono 11 in the
  /// popover's foreground, 6 apart.
  fn recipe() -> Recipe {
    use ChartTooltipPart::{Key, Label, Panel, Row, Title, Value};
    use PaletteColor::{MutedForeground, Popover, PopoverForeground};
    Recipe::new()
      .base(Panel, |s| {
        s.min_width(140.0)
          .padding_y(8.0)
          .padding_x(10.0)
          .gap(6.0)
          .background(Popover)
          .foreground(PopoverForeground)
          .radius(RadiusSize::Md)
          .border(Edge::inside(BorderSize::Sm, PaletteColor::Border))
          .shadow(ShadowStyle::Md)
      })
      .base(Title, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(12.0)
          .weight(FontWeight::SemiBold)
          .foreground(PopoverForeground)
      })
      .base(Row, |s| s.gap(6.0))
      .base(Key, |s| s.square(7.0).radius(RadiusSize::Full))
      .base(Label, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(11.0)
          .foreground(MutedForeground)
      })
      .base(Value, |s| {
        s.font_family(FontFamily::Mono)
          .font_size(11.0)
          .foreground(PopoverForeground)
      })
  }
}
