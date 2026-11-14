//! A data list: terms with their values, side by side with the terms as wide as the widest, or one over the other.

use std::sync::Arc;

use super::{
  Orientation, names,
  part_style::PartStyle,
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe},
  theme::{Themed, current_theme},
};
use crate::{
  app::theme::{BorderSize, PaletteColor, Theme, TypographyStyle},
  components::{Column, Grid, Row},
  layout::{Alignment, grid::GridTrack},
  node::{Element, border::Border, dimension::Dimension},
};

/// The parts of a data list a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DataListPart {
  /// The list: the terms in their column and the values in theirs, or each item under the one before.
  Root,
  /// What a term and its value stand in, one over the other, where they do: the line under it.
  Item,
  /// The term: what the value is of. Side by side, the room after it is the room between it and its value.
  Term,
  /// The value: a text in its type, or what the app puts there.
  Value,
}

names!(DataListPart {
  Root => "root",
  Item => "item",
  Term => "term",
  Value => "value",
});

/// The states of a data list a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DataListState {
  /// Each term beside its value: one of `Horizontal` and `Vertical` is on.
  Horizontal,
  /// Each term over its value, in place of beside it.
  Vertical,
  /// Of an item, a term or a value: a line runs under it, as under every item but the last. One over the other,
  /// the line is the item's; side by side, its term's and its value's, which meet.
  Separator,
}

names!(DataListState {
  Horizontal => "horizontal",
  Vertical => "vertical",
  Separator => "separator",
});

/// Which way a data list's terms stand to their values: [`Orientation`], beside them or over them.
pub type DataListOrientation = Orientation;

/// What builds a data list's nodes: a theme's in place of the default one.
pub type DataListRenderer = Arc<dyn Fn(DataListView) -> Element + Send + Sync>;

/// What stands as an item's value.
enum Shown {
  Text(Arc<str>),
  Element(Element),
}

/// Terms with their values: what a thing is, item by item. The version of a build, the details of an order, the
/// properties of a file.
///
/// ```
/// use lurq::ui::{Badge, DataList, Orientation};
///
/// let about = DataList::new()
///   .text("Version", "0.60.0")
///   .text("Renderer", "wgpu")
///   .text("License", "MIT")
///   .item("Status", Badge::new("Stable").variant("success"));
/// // Each term over its value, for a narrow column.
/// let narrow = DataList::new().orientation(Orientation::Vertical).text("Version", "0.60.0");
/// ```
///
/// Side by side, the terms are in one column as wide as the widest of them, and the values in the column of the
/// rest of the room; a line runs under each item but the last. To what reads the tree each term is a term and each
/// value its definition.
pub struct DataList {
  items: Vec<(Arc<str>, Shown)>,
  orientation: Orientation,
  label: Option<Arc<str>>,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Default for DataList {
  fn default() -> Self {
    Self::new()
  }
}

impl DataList {
  /// A data list with no items yet.
  pub fn new() -> Self {
    Self {
      items: Vec::new(),
      orientation: Orientation::Horizontal,
      label: None,
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// An item: `term`, and what the app puts as its value (a badge, a link, a row of tags).
  pub fn item(mut self, term: &str, value: impl Into<Element>) -> Self {
    self.items.push((Arc::from(term), Shown::Element(value.into())));
    self
  }

  /// Items of `(term, value)`, each as [`item`](Self::item) adds it.
  pub fn items<'a, V: Into<Element>>(mut self, items: impl IntoIterator<Item = (&'a str, V)>) -> Self {
    for (term, value) in items {
      self = self.item(term, value);
    }
    self
  }

  /// An item whose value is a text, in the value's type.
  pub fn text(mut self, term: &str, value: &str) -> Self {
    self.items.push((Arc::from(term), Shown::Text(Arc::from(value))));
    self
  }

  /// Items whose values are texts, each as [`text`](Self::text) adds it.
  pub fn texts<'a>(mut self, items: impl IntoIterator<Item = (&'a str, &'a str)>) -> Self {
    for (term, value) in items {
      self = self.text(term, value);
    }
    self
  }

  /// Which way each term stands to its value: beside it (`Horizontal`, unless told otherwise), or over it
  /// (`Vertical`), for a narrow column.
  pub fn orientation(mut self, orientation: Orientation) -> Self {
    self.orientation = orientation;
    self
  }

  /// What the list is called, to what reads the tree. It draws nothing.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
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

  /// The id of the list's root.
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
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(DataListState::Horizontal, !self.orientation.is_vertical())
    .state(DataListState::Vertical, self.orientation.is_vertical());
    let view = DataListView {
      items: self
        .items
        .into_iter()
        .map(|(term, shown)| DataListItem {
          term,
          value: match &shown {
            Shown::Text(text) => Some(text.clone()),
            Shown::Element(_) => None,
          },
          element: match shown {
            Shown::Element(element) => Some(element),
            Shown::Text(_) => None,
          },
        })
        .collect(),
      orientation: self.orientation,
      label: self.label,
      recipe: theme.resolve::<DataList>(&case, Some(&self.rules)),
      separator: theme.resolve::<DataList>(&case.state(DataListState::Separator, true), Some(&self.rules)),
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<DataList>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<DataList> for Element {
  fn from(list: DataList) -> Self {
    list.build(&current_theme())
  }
}

/// One item of a data list as a renderer sees it.
pub struct DataListItem {
  pub term: Arc<str>,
  /// Its value, where it is a text.
  pub value: Option<Arc<str>>,
  /// Its value, where the app put an element there.
  pub element: Option<Element>,
}

/// A data list as a renderer sees it: its items, and the style of its parts.
pub struct DataListView {
  pub items: Vec<DataListItem>,
  /// Which way each term stands to its value.
  pub orientation: Orientation,
  pub label: Option<Arc<str>>,
  /// The style of each part, with no line under it: of the last item.
  pub recipe: ResolvedRecipe,
  /// The style of each part with a line under it: of every item but the last.
  pub separator: ResolvedRecipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl DataListView {
  /// The style of a part, with no line under it.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The term of `item`, in the style of its part, with the line under it where `lined`: a term to what reads the
  /// tree.
  pub fn term(&self, item: &DataListItem, lined: bool) -> Row {
    let part = self.of(lined).part(DataListPart::Term);
    Row::new()
      .align_items(Alignment::Center)
      .describe("a11y_role", "term")
      .part(&part)
      .child(part.text(&item.term))
  }

  /// The value of `item`, in the style of its part, with the line under it where `lined`: its definition to what
  /// reads the tree.
  pub fn value(&self, item: &mut DataListItem, lined: bool) -> Row {
    let part = self.of(lined).part(DataListPart::Value);
    let fill = Dimension::Pct(100.0);
    let mut value = Row::new()
      .align_items(Alignment::Center)
      .min_width(Dimension::Px(0.0))
      .describe("a11y_role", "definition")
      .part(&part);
    if let Some(text) = &item.value {
      value = value.child(
        Column::new()
          .width(fill)
          .min_width(Dimension::Px(0.0))
          .child(part.text(text).width(fill)),
      );
    }
    if let Some(element) = item.element.take() {
      value = value.child(element);
    }
    value
  }

  fn of(&self, lined: bool) -> &ResolvedRecipe {
    if lined { &self.separator } else { &self.recipe }
  }

  /// How the default renderer builds a data list: side by side, a grid of two columns, the terms' as wide as the
  /// widest and the values' the rest; one over the other, a column of items.
  pub fn render(mut self) -> Element {
    let items = std::mem::take(&mut self.items);
    let count = items.len();
    let root_part = self.part(DataListPart::Root);
    if self.orientation.is_vertical() {
      let mut root = Column::new().align_items(Alignment::Stretch).part(&root_part);
      for (index, mut item) in items.into_iter().enumerate() {
        let lined = index + 1 < count;
        let term = self.term(&item, false);
        let value = self.value(&mut item, false);
        root = root.child(
          Column::new()
            .align_items(Alignment::Stretch)
            .part(&self.of(lined).part(DataListPart::Item))
            .child(term)
            .child(value),
        );
      }
      if let Some(label) = &self.label {
        root = root.describe("a11y_name", label.clone());
      }
      if let Some(id) = &self.id {
        root = root.id(id.clone());
      }
      if let Some(key) = &self.key {
        root = root.key(key.clone());
      }
      return root.into();
    }
    let mut grid = Grid::new()
      .columns([GridTrack::Auto, GridTrack::Fr(1.0)])
      .gap(0.0)
      // A term and its value are as high as the higher of them, so that the lines under them meet.
      .align_items(Alignment::Stretch)
      .part(&root_part);
    for (index, mut item) in items.into_iter().enumerate() {
      let lined = index + 1 < count;
      grid = grid.child(self.term(&item, lined)).child(self.value(&mut item, lined));
    }
    if let Some(label) = &self.label {
      grid = grid.describe("a11y_name", label.clone());
    }
    if let Some(id) = &self.id {
      grid = grid.id(id.clone());
    }
    if let Some(key) = &self.key {
      grid = grid.key(key.clone());
    }
    grid.into()
  }
}

impl Themed for DataList {
  const NAME: &'static str = "data_list";
  type Renderer = DataListRenderer;

  /// The design's data list (`pencil/lurq-design-system.pen`, Data List): each term and its value 10 from the line
  /// over them and the one under them, 12 apart, both in the body's type at 13, the term in the muted color and
  /// the value in the foreground; a line of 1 in the border's color under each item but the last. The board gives
  /// the terms a column of 100; here it is as wide as the widest term. One over the other, which the board does not
  /// draw, the term is 12 and 2 over its value.
  fn recipe() -> Recipe {
    use DataListPart::{Item, Root, Term, Value};
    use DataListState::{Separator, Vertical};
    use PaletteColor::{Foreground, MutedForeground};
    let line = || Border::inside(BorderSize::Sm, PaletteColor::Border);
    Recipe::new()
      .base(Root, |s| s.foreground(Foreground))
      .base(Term, |s| {
        s.padding_y(10.0)
          .padding_end(12.0)
          .typography(TypographyStyle::Body)
          .font_size(13.0)
          .foreground(MutedForeground)
      })
      .base(Value, |s| {
        s.padding_y(10.0)
          .gap(8.0)
          .typography(TypographyStyle::Body)
          .font_size(13.0)
          .foreground(Foreground)
      })
      .state(Separator, Term, |s| s.underline(line()))
      .state(Separator, Value, |s| s.underline(line()))
      // One over the other: the item holds the room and the line, the term is 12 and 2 over its value.
      .state(Vertical, Item, |s| s.padding_y(10.0).gap(2.0))
      .state(Separator, Item, |s| s.underline(line()))
      .state(Vertical, Term, |s| s.padding_y(0.0).padding_end(0.0).font_size(12.0))
      .state(Vertical, Value, |s| s.padding_y(0.0))
  }
}
