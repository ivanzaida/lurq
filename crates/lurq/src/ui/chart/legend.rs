//! The legend of a chart: the board's Chart Legend, a key in each series' color and its name, and the legend a pie has
//! beside it, one item under another with its share.

use std::sync::Arc;

use super::{
  super::{
    Orientation, names,
    part_style::FontFamily,
    recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
    shared::{fading, pressable},
    theme::{Themed, current_theme},
  },
  ChartSize,
};
use crate::{
  app::theme::{OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle},
  components::{Column, Row},
  core::{Signal, SignalValue},
  layout::Alignment,
  node::{BackgroundColor, CursorIcon, Element, FocusGroup},
};

/// The parts of a legend a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChartLegendPart {
  /// The row of the items, which wraps where it is narrower than they are.
  Root,
  /// A series' key and its name; what is pressed to hide it, where the legend hides series.
  Item,
  /// The mark in the series' color.
  Key,
  /// The series' name.
  Label,
  /// What an item says after its name, where it is given one: a slice's share of a pie.
  Value,
}

names!(ChartLegendPart {
  Root => "root",
  Item => "item",
  Key => "key",
  Label => "label",
  Value => "value",
});

/// The states of a legend and its items a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChartLegendState {
  /// The keys are dots and the names muted, as a chart's own legend draws them.
  Dot,
  /// A press of an item hides its series, or shows it again.
  Pressable,
  /// The item's series is hidden.
  Hidden,
  /// The items stand one under another, each as wide as the legend, its value at its end: the legend beside a pie.
  Vertical,
}

names!(ChartLegendState {
  Dot => "dot",
  Pressable => "pressable",
  Hidden => "hidden",
  Vertical => "vertical",
});

/// The mark of a series in a legend.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ChartLegendShape {
  /// A square of 10 with corners of 2, the name in the foreground: the board's Chart Legend.
  #[default]
  Square,
  /// A dot of 8, the name in the muted foreground: the legend under a chart's plot.
  Dot,
}

/// One item of a legend: a series' name and its color, and what it says after the name where it is given a value.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ChartLegendItem {
  pub name: Arc<str>,
  pub color: BackgroundColor,
  pub value: Option<Arc<str>>,
}

impl ChartLegendItem {
  pub fn new(name: &str, color: impl Into<BackgroundColor>) -> Self {
    Self {
      name: Arc::from(name),
      color: color.into(),
      value: None,
    }
  }

  /// What it says after its name, in the mono type: a slice's share, a series' total. The app's words.
  pub fn value(mut self, value: &str) -> Self {
    self.value = Some(Arc::from(value));
    self
  }
}

type HiddenHeard = Arc<dyn Fn(&[String]) + Send + Sync>;

/// What renders a legend: a theme's in place of the default.
pub type ChartLegendRenderer = Arc<dyn Fn(ChartLegendView) -> Element + Send + Sync>;

/// A chart's legend: a key in each series' color and its name, in a row that wraps. Given the series the app hides
/// ([`hidden`](Self::hidden)), a press of an item hides its series or shows it again, and the arrows go between
/// the items.
///
/// ```
/// use lurq::{app::theme::PaletteColor, ui::{ChartLegend, ChartLegendItem}};
///
/// let legend = ChartLegend::new([
///   ChartLegendItem::new("Layout", PaletteColor::Chart1),
///   ChartLegendItem::new("Text", PaletteColor::Chart2),
/// ]);
/// ```
pub struct ChartLegend {
  items: Vec<ChartLegendItem>,
  shape: ChartLegendShape,
  orientation: Orientation,
  size: Option<ChartSize>,
  hidden: Vec<String>,
  write_hidden: Option<HiddenHeard>,
  on_hidden_change: Vec<HiddenHeard>,
  label: Option<Arc<str>>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl ChartLegend {
  pub fn new(items: impl IntoIterator<Item = ChartLegendItem>) -> Self {
    Self {
      items: items.into_iter().collect(),
      shape: ChartLegendShape::Square,
      orientation: Orientation::Horizontal,
      size: None,
      hidden: Vec::new(),
      write_hidden: None,
      on_hidden_change: Vec::new(),
      label: None,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// An item more.
  pub fn item(mut self, item: ChartLegendItem) -> Self {
    self.items.push(item);
    self
  }

  /// Items more.
  pub fn items(mut self, items: impl IntoIterator<Item = ChartLegendItem>) -> Self {
    self.items.extend(items);
    self
  }

  /// The mark of each series: the board's square unless told.
  pub fn shape(mut self, shape: ChartLegendShape) -> Self {
    self.shape = shape;
    self
  }

  /// How the items stand: in a row that wraps unless told, or one under another, each as wide as the legend with
  /// its value at its end and its name in the foreground, as the board's pie has them.
  pub fn orientation(mut self, orientation: Orientation) -> Self {
    self.orientation = orientation;
    self
  }

  /// The size of the chart it stands in, which the items one under another follow: 7 apart with names of 12 in
  /// `Sm`, 8 apart with names of 13 in `Md`.
  pub fn size(mut self, size: impl Into<ChartSize>) -> Self {
    self.size = Some(size.into());
    self
  }

  /// The series hidden, by their names: those the signal has. A press of an item hides its series, or shows it
  /// again.
  pub fn hidden(mut self, hidden: Signal<Vec<String>>) -> Self
  where
    Vec<String>: SignalValue,
  {
    self.hidden = hidden.get();
    self.write_hidden = Some(Arc::new(move |next: &[String]| hidden.set(next.to_vec())));
    self
  }

  /// The series hidden, as the app holds them: `on_change` is told the next after each press.
  pub fn hidden_controlled(
    mut self,
    hidden: impl IntoIterator<Item = String>,
    on_change: impl Fn(&[String]) + Send + Sync + 'static,
  ) -> Self {
    self.hidden = hidden.into_iter().collect();
    self.write_hidden = Some(Arc::new(on_change));
    self
  }

  /// Called with the series hidden after each press that changes them, bound or not. A second handler is called
  /// after the first.
  pub fn on_hidden_change(mut self, handler: impl Fn(&[String]) + Send + Sync + 'static) -> Self {
    self.on_hidden_change.push(Arc::new(handler));
    self
  }

  /// What the legend is called to tooling: "Legend" unless told.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
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

  /// Draws every item as it is under the pointer, held down or focused, with nothing pointing at it.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the row; an item's is the legend's with `-item-` and its place.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  pub fn build(self, theme: &Theme) -> Element {
    let mut case = Case::new()
      .breakpoint(super::super::current_breakpoint())
      .size(self.size.unwrap_or_default())
      .state(ChartLegendState::Dot, self.shape == ChartLegendShape::Dot)
      .state(ChartLegendState::Vertical, self.orientation.is_vertical())
      .state(ChartLegendState::Pressable, self.write_hidden.is_some());
    case.states.extend(self.states.iter().copied());
    let recipe = theme.resolve::<ChartLegend>(&case, Some(&self.rules));
    let hidden_recipe =
      theme.resolve::<ChartLegend>(&case.clone().state(ChartLegendState::Hidden, true), Some(&self.rules));
    let view = ChartLegendView {
      items: self.items,
      hidden: self.hidden,
      recipe,
      hidden_recipe,
      orientation: self.orientation,
      write: self.write_hidden,
      heard: self.on_hidden_change,
      label: self
        .label
        .unwrap_or_else(|| crate::ui::current_locale().data.legend.clone()),
      forced: self.forced,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<ChartLegend>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<ChartLegend> for Element {
  fn from(legend: ChartLegend) -> Self {
    legend.build(&current_theme())
  }
}

/// A legend as a renderer sees it.
pub struct ChartLegendView {
  pub items: Vec<ChartLegendItem>,
  /// The names of the series hidden.
  pub hidden: Vec<String>,
  pub recipe: ResolvedRecipe,
  /// The style of an item whose series is hidden.
  pub hidden_recipe: ResolvedRecipe,
  /// How the items stand: in a row, or one under another.
  pub orientation: Orientation,
  write: Option<HiddenHeard>,
  heard: Vec<HiddenHeard>,
  label: Arc<str>,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl ChartLegendView {
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The item of `item`, at `place`: its key and its name, pressed to hide its series where the legend hides them.
  pub fn item(&self, place: usize, item: &ChartLegendItem) -> Element {
    let hidden = self.hidden.iter().any(|name| **name == *item.name);
    let recipe = if hidden { &self.hidden_recipe } else { &self.recipe };
    let part = |name: ChartLegendPart| recipe.part(name).forced(self.forced);
    let key = Row::new()
      .part(&part(ChartLegendPart::Key))
      .background(item.color.clone());
    let label_part = part(ChartLegendPart::Label);
    let mut label = label_part.text(&item.name).part(&label_part).nowrap();
    let vertical = self.orientation.is_vertical();
    if vertical {
      // The name takes the room the value leaves, and ends in an ellipsis where it is longer.
      label = label
        .flex_full(1.0, 1.0, Some(0.0))
        .min_width(crate::node::dimension::Dimension::Px(0.0))
        .text_overflow(crate::node::node_kind::TextOverflow::Elipsis);
    }
    let value = item.value.as_ref().map(|value| {
      let part = part(ChartLegendPart::Value);
      part.text(value).part(&part).nowrap()
    });
    let name: Arc<str> = match &item.value {
      Some(value) => Arc::from(format!("{}, {value}", item.name)),
      None => item.name.clone(),
    };
    let mut row = match &self.write {
      Some(write) => {
        let (write, heard) = (write.clone(), self.heard.clone());
        let next: Vec<String> = if hidden {
          self
            .hidden
            .iter()
            .filter(|name| **name != *item.name)
            .cloned()
            .collect()
        } else {
          self.hidden.iter().cloned().chain([item.name.to_string()]).collect()
        };
        pressable(&part(ChartLegendPart::Item), false)
          .describe("a11y_role", "checkbox")
          .describe("a11y_checked", if hidden { "false" } else { "true" })
          .describe("a11y_name", name.clone())
          .on_click(move |_| {
            write(&next);
            for handler in &heard {
              handler(&next);
            }
          })
      }
      None => components_row(&part(ChartLegendPart::Item))
        .describe("a11y_role", "listitem")
        .describe("a11y_name", name),
    };
    row = row.align_items(Alignment::Center).child(key).child(label);
    if let Some(value) = value {
      row = row.child(value);
    }
    if vertical {
      row = row.width(crate::node::dimension::Dimension::Pct(100.0));
    }
    if let Some(id) = &self.id {
      row = row.id(format!("{id}-item-{place}"));
    }
    row.into()
  }

  /// How the default renderer builds a legend: its items in a row that wraps, the arrows between them where they
  /// are pressed.
  pub fn render(self) -> Element {
    let items: Vec<Element> = self
      .items
      .iter()
      .enumerate()
      .map(|(place, item)| self.item(place, item))
      .collect();
    let part = self.part(ChartLegendPart::Root);
    if self.orientation.is_vertical() {
      let mut root = Column::new()
        .align_items(Alignment::Stretch)
        .part(&part)
        .describe("a11y_role", "list")
        .describe("a11y_name", self.label.clone());
      if self.write.is_some() {
        root = root.focus_group(FocusGroup::vertical().one_stop(true));
      }
      if let Some(id) = &self.id {
        root = root.id(id.clone());
      }
      if let Some(key) = &self.key {
        root = root.key(key.clone());
      }
      return root.with_children(items).into();
    }
    let mut root = Row::new()
      .wrap()
      .align_items(Alignment::Center)
      .part(&part)
      .describe("a11y_role", "list")
      .describe("a11y_name", self.label.clone());
    if self.write.is_some() {
      root = root.focus_group(FocusGroup::horizontal().one_stop(true));
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root.with_children(items).into()
  }
}

/// An item that is not pressed: a row in the item's style.
fn components_row(part: &ResolvedPart) -> crate::components::Button {
  crate::components::Button::empty().part(part)
}

impl crate::ui::Styled for ChartLegend {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for ChartLegend {
  const NAME: &'static str = "chart_legend";
  type Renderer = ChartLegendRenderer;

  fn parts() -> crate::ui::Declared {
    ChartLegendPart::declared()
  }

  fn states() -> crate::ui::Declared {
    ChartLegendState::declared()
  }

  /// The board's Chart Legend (`OjIRm`): a row of 16 between items, each a key of 10 with corners of 2 and its
  /// name 12 in the foreground, 6 apart. Under a chart's plot the keys are dots of 8 and the names muted, as the
  /// board's charts draw them. Where it hides series, an item is pressed, under the pointer in the foreground and
  /// faded while its series is hidden; the focus from the keyboard is the ring around it. An item's value is mono 11
  /// in the muted foreground. One under another (the board's Pie Chart `dDHPj` and Donut Chart `GJ5Mg`), the items
  /// are 8 apart in `Md` and 7 in `Sm`, the key and the name 8 apart, the name 13 (12 in `Sm`) in the card's
  /// foreground and the value at the end.
  fn recipe() -> Recipe {
    use ChartLegendPart::{Item, Key, Label, Root, Value};
    use ChartLegendState::{Dot, Hidden, Pressable, Vertical};
    use PaletteColor::{Foreground, MutedForeground};
    Recipe::new()
      .base(Root, |s| s.gap(16.0))
      .base(Item, |s| s.gap(6.0))
      .base(Key, |s| s.square(10.0).radius(2.0))
      .base(Label, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(12.0)
          .foreground(Foreground)
      })
      .state(Dot, Key, |s| s.square(8.0).radius(RadiusSize::Full))
      .state(Dot, Label, |s| s.foreground(MutedForeground))
      .state(Pressable, Item, |s| {
        fading(s.radius(RadiusSize::Sm).cursor(CursorIcon::Pointer))
      })
      .hovered(Label, |s| s.foreground(Foreground))
      .focus_visible(Item, |s| s.focus_ring())
      .state(Hidden, Item, |s| s.opacity(OpacityLevel::Disabled))
      .base(Value, |s| {
        s.font_family(FontFamily::Mono)
          .font_size(11.0)
          .foreground(MutedForeground)
      })
      // One under another, as the board's pie and donut have them: the key and the name 8 apart, the name in the
      // card's foreground, the value at the end.
      .state(Vertical, Root, |s| s.gap(8.0))
      .state(Vertical, Item, |s| s.gap(8.0))
      .state(Vertical, Label, |s| s.font_size(13.0).foreground(PaletteColor::CardForeground))
      .when(Selector::new().size(ChartSize::Sm).state(Vertical), Root, |s| s.gap(7.0))
      .when(Selector::new().size(ChartSize::Sm).state(Vertical), Label, |s| s.font_size(12.0))
  }
}
