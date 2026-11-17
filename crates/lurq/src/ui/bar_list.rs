//! A bar list: named values as bars of one scale, longest to shortest as the app orders them. The slowest tests,
//! the largest files, the pages visited most.

use std::sync::Arc;

use super::{
  names,
  part_style::{FontFamily, Length, PartStyle},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::{fading, pressable},
  theme::{Themed, current_theme},
};
use crate::{
  app::theme::{BorderSize, OpacityLevel, PaletteColor, RadiusSize, ShadowStyle, Theme, TypographyStyle},
  components::{Column, Row},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{
    BackgroundColor, CursorIcon, Element, FocusGroup, border::Border as Edge, dimension::Dimension, outline::Outline,
  },
};

/// The parts of a bar list a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BarListPart {
  /// The box around it all: the board's card.
  Root,
  /// The title over the description.
  Header,
  /// What the list shows: "Slowest tests".
  Title,
  /// The line under the title: what the values are, "Seconds".
  Description,
  /// The column of the rows.
  Rows,
  /// A row: its name, its track and its value. What is pressed where it is pressed.
  Row,
  /// The name of a row, on one line, cut with an ellipsis where it is longer than its room.
  Name,
  /// The room a bar grows in.
  Track,
  /// A row's bar, as long as its value is of the scale.
  Bar,
  /// The value written after the bar.
  Value,
  /// What is said where there is nothing to show.
  Empty,
}

names!(BarListPart {
  Root => "root",
  Header => "header",
  Title => "title",
  Description => "description",
  Rows => "rows",
  Row => "row",
  Name => "name",
  Track => "track",
  Bar => "bar",
  Value => "value",
  Empty => "empty",
});

/// The states of a bar list a recipe styles, besides what the pointer and the keyboard do to a row that is pressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BarListState {
  /// Of the list: drawn in the board's card. A look; off, the rows stand on what holds them.
  Framed,
  /// Of a row: it is pressed, the list's keys moving between it and the others.
  Pressable,
  /// Of a row: it is pressed and takes nothing now.
  Disabled,
}

names!(BarListState {
  Framed => "framed",
  Pressable => "pressable",
  Disabled => "disabled",
});

/// What builds a bar list's nodes: a theme's in place of the default one.
pub type BarListRenderer = Arc<dyn Fn(BarListView) -> Element + Send + Sync>;

type Select = Arc<dyn Fn() + Send + Sync>;
type Told = Arc<dyn Fn(&str) + Send + Sync>;
type ValueText = Arc<dyn Fn(f64) -> String + Send + Sync>;

/// A row of a [`BarList`]: a name and its value.
pub struct BarListItem {
  name: Arc<str>,
  value: f64,
  color: Option<BackgroundColor>,
  on_select: Vec<Select>,
  disabled: bool,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl BarListItem {
  /// A row of `name` and its `value`: `BarListItem::new("text_pipeline", 9.2)`.
  pub fn new(name: &str, value: impl Into<f64>) -> Self {
    Self {
      name: Arc::from(name),
      value: value.into(),
      color: None,
      on_select: Vec::new(),
      disabled: false,
      id: None,
      key: None,
    }
  }

  /// The color this row's bar is drawn in, a role of the palette or a color, in place of the list's: the one row
  /// to point at.
  pub fn color(mut self, color: impl Into<BackgroundColor>) -> Self {
    self.color = Some(color.into());
    self
  }

  /// Called when the row is pressed, or Enter or Space is pressed while it has the focus, before the list's
  /// [`on_select`](BarList::on_select). A row with a handler is pressed. A second handler is called after the
  /// first.
  pub fn on_select(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_select.push(Arc::new(handler));
    self
  }

  /// Whether a row that is pressed takes nothing now: it is faded and the keys pass it by.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// The id of the row.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// The key of the row, which the list's `on_select` is told in place of its name where two rows have one name.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }
}

impl<N: AsRef<str>, V: Into<f64>> From<(N, V)> for BarListItem {
  fn from((name, value): (N, V)) -> Self {
    Self::new(name.as_ref(), value)
  }
}

/// A bar list: named values as bars of one scale. The slowest tests of a run, the largest files, the pages visited
/// most.
///
/// ```
/// use lurq::ui::{BarList, BarListItem};
///
/// let slowest = BarList::new([("text_pipeline", 9.2), ("canvas_mesh", 7.0), ("layout_flex", 4.7)])
///   .title("Slowest tests")
///   .description("Seconds");
/// // Rows that are pressed, told which by name: one stop of Tab, the arrows between them.
/// let files = BarList::new([BarListItem::new("target/", 27.4), BarListItem::new("node_modules/", 3.1)])
///   .title("Largest folders")
///   .value_text(|gigabytes| format!("{gigabytes:.1} GB"))
///   .on_select(|name| println!("open {name}"));
/// ```
///
/// Each row is its name, a bar in a track and its value. The bars share one scale, from 0 to
/// [`max`](Self::max): the largest value rounded up to a round number (1, 2, 2.5 or 5 of a power of ten), as the
/// board draws 9.2 of 10, unless the app gives it. A value under 0 draws no bar; one over the scale fills its track.
/// The rows are in the app's order. A name longer than its room is cut with an ellipsis, the value keeping its
/// room. A list with no rows says [`empty_text`](Self::empty_text).
///
/// It is read and not operated, but where a row is pressed (its own `on_select`, or the list's, which makes every
/// row pressed) the pressed rows are one stop of Tab, and the arrows, Home, End and the first letters of a name move
/// between them. To tooling it is a `figure` called by its title, holding a `list` whose rows say their name and
/// their value.
pub struct BarList {
  items: Vec<BarListItem>,
  title: Option<Arc<str>>,
  description: Option<Arc<str>>,
  label: Option<Arc<str>>,
  max: Option<f64>,
  color: BackgroundColor,
  value_text: Option<ValueText>,
  empty_text: Option<Arc<str>>,
  framed: bool,
  on_select: Vec<Told>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl BarList {
  /// A bar list of `items`: [`BarListItem`]s or `(name, value)` pairs.
  pub fn new(items: impl IntoIterator<Item = impl Into<BarListItem>>) -> Self {
    Self {
      items: items.into_iter().map(Into::into).collect(),
      title: None,
      description: None,
      label: None,
      max: None,
      color: BackgroundColor::Palette(PaletteColor::Chart1),
      value_text: None,
      empty_text: None,
      framed: true,
      on_select: Vec::new(),
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// Adds a row after those it has.
  pub fn item(mut self, item: impl Into<BarListItem>) -> Self {
    self.items.push(item.into());
    self
  }

  /// Adds rows after those it has.
  pub fn items(mut self, items: impl IntoIterator<Item = impl Into<BarListItem>>) -> Self {
    self.items.extend(items.into_iter().map(Into::into));
    self
  }

  /// What the list shows, over its rows: "Slowest tests". It is the list's name to tooling unless it has a label.
  pub fn title(mut self, title: &str) -> Self {
    self.title = Some(Arc::from(title));
    self
  }

  /// The line under the title: what the values are, "Seconds".
  pub fn description(mut self, description: &str) -> Self {
    self.description = Some(Arc::from(description));
    self
  }

  /// What the list is called, to what reads the tree, where it has no title or another name is wanted.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// The value a bar fills its track at. Not given, the largest value rounded up to a round number.
  pub fn max(mut self, max: impl Into<f64>) -> Self {
    self.max = Some(max.into());
    self
  }

  /// The color the bars are drawn in, a role of the palette or a color, as a chart's series takes it: the first
  /// of the chart colors, `Chart1` (`series_color(0)`), unless given.
  pub fn color(mut self, color: impl Into<BackgroundColor>) -> Self {
    self.color = color.into();
    self
  }

  /// Writes a row's value as the app does: `|seconds| format!("{seconds:.1} s")`. Not given, the values are
  /// written with as many decimals as the most precise of them has, up to 2.
  pub fn value_text(mut self, text: impl Fn(f64) -> String + Send + Sync + 'static) -> Self {
    self.value_text = Some(Arc::new(text));
    self
  }

  /// What a list with no rows says: "No data" when not given.
  pub fn empty_text(mut self, text: &str) -> Self {
    self.empty_text = Some(Arc::from(text));
    self
  }

  /// Whether the list is drawn in the board's card, as it is unless told otherwise: off, the rows stand on what
  /// holds them (a card of the app's, a panel).
  pub fn framed(mut self, framed: bool) -> Self {
    self.framed = framed;
    self
  }

  /// Called with the key of the row pressed (its name, unless it has a `key`), after the row's own handlers: every
  /// row that is not disabled is then pressed. A second handler is called after the first.
  pub fn on_select(mut self, handler: impl Fn(&str) + Send + Sync + 'static) -> Self {
    self.on_select.push(Arc::new(handler));
    self
  }

  /// Puts the list in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this list over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this list.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the first row that is pressed as it is under the pointer, held down or focused from the keyboard, with
  /// nothing pointing at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the list's box.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the list in `theme`. Converting a list into an element does this in the theme of what is being
  /// rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let view = self.view(theme);
    match theme.renderer::<BarList>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }

  fn view(self, theme: &Theme) -> BarListView {
    let base = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(BarListState::Framed, self.framed);
    let recipe = theme.resolve::<BarList>(&base, Some(&self.rules));
    let max = self
      .max
      .unwrap_or_else(|| round_up(self.items.iter().map(|item| item.value).fold(0.0, f64::max)));
    let decimals = self.items.iter().map(|item| decimals_of(item.value)).max().unwrap_or(0);
    let text: ValueText = self
      .value_text
      .unwrap_or_else(|| Arc::new(move |value: f64| format!("{value:.decimals$}")));
    let told: Arc<[Told]> = self.on_select.into();
    let forced_at = self
      .items
      .iter()
      .position(|item| (!item.on_select.is_empty() || !told.is_empty()) && !item.disabled);
    let mut grouped = false;
    let rows = self
      .items
      .into_iter()
      .enumerate()
      .map(|(index, item)| {
        let mut handlers = item.on_select;
        if !told.is_empty() {
          let (told, key) = (told.clone(), item.key.clone().unwrap_or_else(|| item.name.clone()));
          handlers.push(Arc::new(move || {
            for handler in told.iter() {
              handler(&key);
            }
          }));
        }
        grouped |= !handlers.is_empty();
        let case = base
          .clone()
          .state(BarListState::Pressable, !handlers.is_empty())
          .state(BarListState::Disabled, item.disabled);
        let fraction = if max > 0.0 {
          (item.value / max).clamp(0.0, 1.0)
        } else {
          0.0
        };
        BarListRow {
          value_text: text(item.value),
          name: item.name,
          value: item.value,
          fraction: fraction as f32,
          color: item.color.unwrap_or(self.color.clone()),
          on_select: handlers,
          disabled: item.disabled,
          forced: if forced_at == Some(index) { self.forced } else { None },
          recipe: theme.resolve::<BarList>(&case, Some(&self.rules)),
          id: item.id,
          key: item.key,
        }
      })
      .collect();
    BarListView {
      rows,
      title: self.title,
      description: self.description,
      label: self.label,
      max,
      empty_text: self
        .empty_text
        .unwrap_or_else(|| super::current_locale().data.no_data.clone()),
      grouped,
      recipe,
      id: self.id,
      key: self.key,
    }
  }
}

impl From<BarList> for Element {
  fn from(list: BarList) -> Self {
    list.build(&current_theme())
  }
}

/// `value` rounded up to a round number: 1, 2, 2.5 or 5 of a power of ten, as a scale's end is. 1 for nothing.
fn round_up(value: f64) -> f64 {
  if !value.is_finite() || value <= 0.0 {
    return 1.0;
  }
  let power = 10f64.powf(value.log10().floor());
  [1.0, 2.0, 2.5, 5.0, 10.0]
    .into_iter()
    .map(|step| step * power)
    .find(|end| *end >= value * (1.0 - 1e-9))
    .unwrap_or(10.0 * power)
}

/// How many decimals `value` needs, up to 2.
fn decimals_of(value: f64) -> usize {
  (0..2)
    .find(|places| {
      let scaled = value * 10f64.powi(*places as i32);
      (scaled - scaled.round()).abs() < 1e-6
    })
    .unwrap_or(2)
}

/// A bar list as a renderer sees it: its rows, its header and the style of its box.
pub struct BarListView {
  /// The rows, in the app's order.
  pub rows: Vec<BarListRow>,
  /// What the list shows.
  pub title: Option<Arc<str>>,
  /// The line under the title.
  pub description: Option<Arc<str>>,
  /// What it is called to tooling where it is not its title.
  pub label: Option<Arc<str>>,
  /// The value a bar fills its track at.
  pub max: f64,
  /// What a list with no rows says.
  pub empty_text: Arc<str>,
  /// Whether a row is pressed: the pressed rows are then one stop of Tab, and the keys move between them.
  pub grouped: bool,
  recipe: ResolvedRecipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl BarListView {
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The title over the description, where it has either.
  pub fn header(&self) -> Option<Column> {
    if self.title.is_none() && self.description.is_none() {
      return None;
    }
    let mut header = Column::new()
      .width(Dimension::Pct(100.0))
      .part(&self.part(BarListPart::Header));
    // A text takes its color from the part around it.
    let text = |part: ResolvedPart, content: &str| {
      Column::new()
        .width(Dimension::Pct(100.0))
        .part(&part)
        .child(part.text(content))
    };
    if let Some(title) = &self.title {
      header = header.child(text(self.part(BarListPart::Title), title));
    }
    if let Some(description) = &self.description {
      header = header.child(text(self.part(BarListPart::Description), description));
    }
    Some(header)
  }

  /// The column of `rows`, a `list` to tooling, one focus group where a row is pressed; or what an empty list says.
  pub fn rows(&self, rows: Vec<Element>) -> Element {
    if rows.is_empty() {
      return Row::new()
        .part(&self.part(BarListPart::Empty))
        .child(self.part(BarListPart::Empty).text(&self.empty_text))
        .into();
    }
    let mut column = Column::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Stretch)
      .overflow_visible()
      .part(&self.part(BarListPart::Rows))
      .describe("a11y_role", "list");
    if self.grouped {
      column = column.focus_group(FocusGroup::vertical().one_stop(true).wrap(false).typeahead(true));
    }
    column.with_children(rows).into()
  }

  /// The box in the style of its part, a `figure` to tooling called by its label or its title, holding `content`.
  pub fn root(&self, content: Vec<Element>) -> Column {
    let mut root = Column::new()
      .align_items(Alignment::Stretch)
      .part(&self.part(BarListPart::Root))
      .describe("a11y_role", "figure");
    if let Some(name) = self.label.as_ref().or(self.title.as_ref()) {
      root = root.describe("a11y_name", name.clone());
    }
    if let Some(description) = &self.description {
      root = root.describe("a11y_description", description.clone());
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root.with_children(content)
  }

  /// How the default renderer builds a bar list: its header over its rows, in its box.
  pub fn render(mut self) -> Element {
    let rows: Vec<Element> = std::mem::take(&mut self.rows)
      .into_iter()
      .map(BarListRow::render)
      .collect();
    let mut content: Vec<Element> = Vec::new();
    content.extend(self.header().map(Into::into));
    content.push(self.rows(rows));
    self.root(content).into()
  }
}

/// A row of a bar list as a renderer sees it.
pub struct BarListRow {
  /// Its name.
  pub name: Arc<str>,
  /// Its value.
  pub value: f64,
  /// Its value as it is written.
  pub value_text: String,
  /// How much of its track its bar fills, from 0 to 1.
  pub fraction: f32,
  /// The color of its bar.
  pub color: BackgroundColor,
  on_select: Vec<Select>,
  /// Whether a row that is pressed takes nothing now.
  pub disabled: bool,
  forced: Option<Interaction>,
  recipe: ResolvedRecipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl BarListRow {
  /// The style of a part, as this row is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// Whether the row is pressed.
  pub fn pressable(&self) -> bool {
    !self.on_select.is_empty()
  }

  /// Its name on one line, in the room its part gives it, cut with an ellipsis where it is longer.
  pub fn name(&self) -> Row {
    let part = self.part(BarListPart::Name);
    Row::new()
      .min_width(Dimension::Px(0.0))
      .part(&part)
      .child(part.text(&self.name).nowrap().width(Dimension::Pct(100.0)))
  }

  /// Its track, as wide as the row leaves, with its bar in it.
  pub fn track(&self) -> Row {
    let bar = self.part(BarListPart::Bar);
    Row::new()
      .flex(1.0)
      .min_width(Dimension::Px(0.0))
      .align_items(Alignment::Center)
      .part(&self.part(BarListPart::Track))
      .child(
        Row::new()
          .width(Dimension::Pct(self.fraction * 100.0))
          .part(&bar)
          .background(self.color.clone()),
      )
  }

  /// Its value as it is written.
  pub fn value(&self) -> Row {
    let part = self.part(BarListPart::Value);
    Row::new().part(&part).child(part.text(&self.value_text).nowrap())
  }

  /// The row in the style of its part, holding `children` side by side: a `listitem` that says its name and value
  /// to tooling, a button inside it where it is pressed.
  pub fn root(&self, children: Vec<Element>) -> Element {
    let part = self.part(BarListPart::Row);
    let line = Row::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Center)
      .with_children(children);
    let mut row = Column::new()
      .align_items(Alignment::Stretch)
      .overflow_visible()
      .describe("a11y_role", "listitem")
      .describe("a11y_name", self.name.clone())
      .describe("a11y_valuetext", self.value_text.clone());
    if let Some(key) = &self.key {
      row = row.key(key.clone());
    }
    if !self.pressable() {
      if let Some(id) = &self.id {
        row = row.id(id.clone());
      }
      return row.child(line.part(&part)).into();
    }
    let handlers = self.on_select.clone();
    let mut button = pressable(&part, self.disabled)
      .width(Dimension::Pct(100.0))
      .justify(Justify::Start)
      .align_items(Alignment::Stretch)
      .describe("a11y_role", "button")
      .describe("a11y_name", format!("{}: {}", self.name, self.value_text))
      .focus_label(self.name.clone())
      .on_click(move |_| {
        for handler in &handlers {
          handler();
        }
      });
    if self.disabled {
      button = button.describe("a11y_disabled", "true");
    }
    if let Some(id) = &self.id {
      button = button.id(id.clone());
    }
    row.child(button.child(line)).into()
  }

  /// How the default renderer builds a row: its name, its track and its value.
  pub fn render(self) -> Element {
    let children = vec![self.name().into(), self.track().into(), self.value().into()];
    self.root(children)
  }
}

impl crate::ui::Styled for BarList {
  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for BarList {
  const NAME: &'static str = "bar_list";
  type Renderer = BarListRenderer;

  fn parts() -> crate::ui::Declared {
    BarListPart::declared()
  }

  fn states() -> crate::ui::Declared {
    BarListState::declared()
  }

  /// The design's bar list (`pencil/lurq-design-system.pen`, Bar List, in Charts' Bars): the card (the card's fill,
  /// an edge of 1 in the border's color, the extra large radius, the small shadow), 20 in, its title and its rows
  /// 16 apart. The title is 15 at 600 in the card's foreground over the description, 12 in the muted color, 3
  /// under it. A row is its name, the mono type at 11 in the card's foreground in a column of 92, a track of 26
  /// high and a bar of 14 in it with the small radius, and its value, the mono type at 11 in the muted color, 10
  /// apart. A row is as high as the theme's density makes a row of 26.
  ///
  /// The board draws nothing else of it: a row that is pressed has the veils of the pointer, a ring of 2 inside it
  /// and the small radius, and is faded where it is disabled; an empty list says so in the muted color.
  fn recipe() -> Recipe {
    use BarListPart::{Bar, Description, Empty, Header, Name, Root, Row, Rows, Title, Track, Value};
    use BarListState::{Disabled, Framed, Pressable};
    use PaletteColor::{Card, CardForeground, MutedForeground, Ring, StateHover, StatePressed};
    Recipe::new()
      .base(Root, |s| s.width(Length::Fill).gap(16.0))
      .state(Framed, Root, |s| {
        s.background(Card)
          .border(Edge::inside(BorderSize::Sm, PaletteColor::Border))
          .radius(RadiusSize::Xl)
          .shadow(ShadowStyle::Sm)
          .padding_x(20.0)
          .padding_y(20.0)
      })
      .base(Header, |s| s.gap(3.0))
      .base(Title, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(15.0)
          .weight(FontWeight::SemiBold)
          .foreground(CardForeground)
      })
      .base(Description, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(12.0)
          .foreground(MutedForeground)
      })
      .base(Rows, |s| s.gap(16.0))
      .base(Row, |s| fading(s.gap(10.0).radius(RadiusSize::Sm)))
      .state(Pressable, Row, |s| s.cursor(CursorIcon::Pointer))
      .when(Selector::new().state(Pressable).hovered(), Row, |s| s.veil(StateHover))
      .when(Selector::new().state(Pressable).pressed(), Row, |s| {
        s.veil(StatePressed)
      })
      .focus_visible(Row, |s| s.outline(Outline::new(BorderSize::Md, Ring).offset(-2.0)))
      .state(Disabled, Row, |s| {
        s.opacity(OpacityLevel::Disabled).cursor(CursorIcon::Default)
      })
      .base(Name, |s| {
        s.width(92.0)
          .font_family(FontFamily::Mono)
          .font_size(11.0)
          .foreground(CardForeground)
      })
      .base(Track, |s| s.height(26.0))
      // A row follows the theme's density: 18, 34 and 38 at compact, comfortable and touch; its bar keeps its 14.
      .row_density(Selector::new(), Track, 26.0)
      .base(Bar, |s| s.height(14.0).radius(RadiusSize::Sm))
      .base(Value, |s| {
        s.font_family(FontFamily::Mono)
          .font_size(11.0)
          .foreground(MutedForeground)
      })
      .base(Empty, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(13.0)
          .foreground(MutedForeground)
      })
  }
}
