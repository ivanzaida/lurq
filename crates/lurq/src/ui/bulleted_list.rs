//! A list of lines, each with a mark before it: a bullet, a number, an icon or a mark of the app's own.

use std::sync::Arc;

use super::{
  icon::Icon,
  names,
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe},
  theme::{Themed, current_breakpoint, current_theme},
};
use crate::{
  app::theme::{BorderSize, PaletteColor, RadiusSize, Theme, TypographyStyle},
  components::{Column, Row},
  layout::{Alignment, layout_kind::Justify},
  node::{Element, border::Border, color::Color, dimension::Dimension},
  ui::{Length, PartStyle},
};

/// The numbers of the design's list (`pencil/lurq-design-system.pen`, Bulleted List `btfDc`, and the lists of the
/// Markdown document): what [`BulletedList`]'s recipe is written with, and what a Markdown document's lists start
/// from ([`ThemeMarkdown`](crate::app::theme::ThemeMarkdown)), so that the two are one look from one source.
pub(crate) mod look {
  /// The room between two items, and between what an item holds under its line.
  pub const ITEM_GAP: f32 = 6.0;
  /// The side of a bullet.
  pub const BULLET: f32 = 5.0;
  /// The room between a bullet, an icon or a mark and the item's line.
  pub const BULLET_GAP: f32 = 10.0;
  /// The width a number is set in, at its end.
  pub const NUMBER_WIDTH: f32 = 18.0;
  /// The room between a number and the item's line.
  pub const NUMBER_GAP: f32 = 8.0;
}

/// The side of an icon that is an item's mark: the size of the item's text.
const ICON: f32 = 14.0;

/// The parts of a list a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BulletedListPart {
  /// The list: the room between its items.
  Root,
  /// An item: its mark and what it says, side by side.
  Item,
  /// The box the mark of an item stands in, as high as one line of the item: its width, and the type and the color
  /// of a number.
  Mark,
  /// The bullet in that box.
  Bullet,
  /// What the item says.
  Label,
  /// The item's line and what it holds under it: the room between them.
  Body,
}

names!(BulletedListPart {
  Root => "root",
  Item => "item",
  Mark => "mark",
  Bullet => "bullet",
  Label => "label",
  Body => "body",
});

/// The states of a list a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BulletedListState {
  /// Its items are numbered.
  Numbered,
  /// Its items have an icon for a bullet, or one of them has an icon or a mark of its own: every mark has an
  /// icon's room.
  Icon,
  /// It is in an item of another list: its bullets are rings.
  Nested,
}

names!(BulletedListState {
  Numbered => "numbered",
  Icon => "icon",
  Nested => "nested",
});

/// What stands before each item of a list.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum BulletedListMarker {
  /// A bullet: a dot, and a ring in a list that is in another's item.
  #[default]
  Bullet,
  /// The item's number, from the list's [`start`](BulletedList::start).
  Number,
  /// An icon of the theme's set: a check, an arrow.
  Icon(Name),
  /// Nothing: the lines alone, as far in as a bullet would set them.
  None,
}

impl From<&str> for BulletedListMarker {
  /// An icon of the theme's set, by its name.
  fn from(icon: &str) -> Self {
    Self::Icon(Name::from(icon))
  }
}

impl From<Name> for BulletedListMarker {
  fn from(icon: Name) -> Self {
    Self::Icon(icon)
  }
}

/// What builds the nodes of a list: a theme's in place of the default one.
pub type BulletedListRenderer = Arc<dyn Fn(BulletedListView) -> Element + Send + Sync>;

/// An item of a [`BulletedList`]: a line, what it holds under it, and a list of its own.
///
/// ```
/// use lurq::ui::{BulletedList, BulletedListItem, icons};
///
/// let item = BulletedListItem::new("Semantic color roles")
///   .icon(icons::CHECK)
///   .list(BulletedList::new(["Light", "Dark"]));
/// ```
pub struct BulletedListItem {
  text: Arc<str>,
  body: Vec<Element>,
  list: Option<BulletedList>,
  icon: Option<Name>,
  media: Option<Element>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl BulletedListItem {
  /// An item that says `text`. An empty one says nothing, for an item of elements alone.
  pub fn new(text: &str) -> Self {
    Self {
      text: Arc::from(text),
      body: Vec::new(),
      list: None,
      icon: None,
      media: None,
      id: None,
      key: None,
    }
  }

  /// Adds an element under the item's line: a paragraph, a block of code.
  pub fn child(mut self, child: impl Into<Element>) -> Self {
    self.body.push(child.into());
    self
  }

  /// Adds elements under the item's line.
  pub fn children(mut self, children: impl IntoIterator<Item = impl Into<Element>>) -> Self {
    self.body.extend(children.into_iter().map(Into::into));
    self
  }

  /// A list in the item, under what it says and what it holds: its bullets are rings, and it keeps the marker it
  /// was given (numbers in a list of bullets).
  pub fn list(mut self, list: BulletedList) -> Self {
    self.list = Some(list);
    self
  }

  /// An icon of the theme's set as the mark of this item, in place of the list's.
  pub fn icon(mut self, icon: impl Into<Name>) -> Self {
    self.icon = Some(icon.into());
    self
  }

  /// An element of the app's as the mark of this item, in place of the list's: a status dot, a picture. It stands
  /// in the middle of the item's first line.
  pub fn media(mut self, media: impl Into<Element>) -> Self {
    self.media = Some(media.into());
    self
  }

  /// The id of the item's row.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }
}

impl From<&str> for BulletedListItem {
  fn from(text: &str) -> Self {
    Self::new(text)
  }
}

impl From<String> for BulletedListItem {
  fn from(text: String) -> Self {
    Self::new(&text)
  }
}

impl From<&String> for BulletedListItem {
  fn from(text: &String) -> Self {
    Self::new(text)
  }
}

/// A list of lines, each with a mark before it: the features of a plan, the steps of a recipe, what a release
/// changed. Markdown's `-` and `1.` are drawn with the same numbers.
///
/// ```
/// use lurq::ui::{BulletedList, BulletedListItem, BulletedListMarker, icons};
///
/// let roles = BulletedList::new(["Semantic color roles", "Light and dark palettes"]);
/// let steps = BulletedList::new(["Install", "Configure", "Run"]).marker(BulletedListMarker::Number);
/// let done = BulletedList::new(["Signed", "Notarized"]).marker(icons::CHECK);
/// let nested = BulletedList::new(["Tokens"]).item(
///   BulletedListItem::new("Components").list(BulletedList::new(["Button", "Switch"])),
/// );
/// ```
///
/// An item's line wraps at the list's width, and its mark stays in the middle of its first line. It is read and
/// not operated: to what reads the tree it is a `list` of `listitem`s, called by its label where it has one.
pub struct BulletedList {
  items: Vec<BulletedListItem>,
  marker: BulletedListMarker,
  start: u64,
  number_text: Arc<dyn Fn(u64) -> String + Send + Sync>,
  depth: usize,
  label: Option<Arc<str>>,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl BulletedList {
  /// A list of `items`: texts, or [`BulletedListItem`]s.
  pub fn new(items: impl IntoIterator<Item = impl Into<BulletedListItem>>) -> Self {
    Self {
      items: items.into_iter().map(Into::into).collect(),
      marker: BulletedListMarker::Bullet,
      start: 1,
      number_text: Arc::new(|number| format!("{number}.")),
      depth: 0,
      label: None,
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// Adds an item.
  pub fn item(mut self, item: impl Into<BulletedListItem>) -> Self {
    self.items.push(item.into());
    self
  }

  /// Adds items.
  pub fn items(mut self, items: impl IntoIterator<Item = impl Into<BulletedListItem>>) -> Self {
    self.items.extend(items.into_iter().map(Into::into));
    self
  }

  /// What stands before each item: a [`BulletedListMarker`], or the name of an icon of the theme's set.
  pub fn marker(mut self, marker: impl Into<BulletedListMarker>) -> Self {
    self.marker = marker.into();
    self
  }

  /// The number of the first item of a numbered list, in place of 1.
  pub fn start(mut self, start: u64) -> Self {
    self.start = start;
    self
  }

  /// How a numbered list writes the number of an item, in place of `1.`: `a)`, `i.`, a number in another script.
  pub fn number_text(mut self, text: impl Fn(u64) -> String + Send + Sync + 'static) -> Self {
    self.number_text = Arc::new(text);
    self
  }

  /// Its name to what reads the tree.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// Puts it in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this list.
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

  /// Builds the list in `theme`. Converting it into an element does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let mut case = Case::new()
      .breakpoint(current_breakpoint())
      .state(BulletedListState::Numbered, self.marker == BulletedListMarker::Number)
      // An icon or a mark of one item's own makes the room of an icon before every item, so that their lines start
      // together.
      .state(
        BulletedListState::Icon,
        self.marker != BulletedListMarker::Number
          && (matches!(self.marker, BulletedListMarker::Icon(_))
            || self.items.iter().any(|item| item.icon.is_some() || item.media.is_some())),
      )
      .state(BulletedListState::Nested, self.depth > 0);
    for state in &self.states {
      case = case.state(state.clone(), true);
    }
    let view = BulletedListView {
      items: self.items,
      marker: self.marker,
      start: self.start,
      number_text: self.number_text,
      depth: self.depth,
      label: self.label,
      recipe: theme.resolve::<BulletedList>(&case, Some(&self.rules)),
      rules: self.rules,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<BulletedList>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<BulletedList> for Element {
  fn from(list: BulletedList) -> Self {
    list.build(&current_theme())
  }
}

/// A list as a renderer sees it.
pub struct BulletedListView {
  pub items: Vec<BulletedListItem>,
  pub marker: BulletedListMarker,
  /// The number of the first item of a numbered list.
  pub start: u64,
  number_text: Arc<dyn Fn(u64) -> String + Send + Sync>,
  /// How many lists it is in: none for a list of its own.
  pub depth: usize,
  pub label: Option<Arc<str>>,
  /// The style of each part.
  pub recipe: ResolvedRecipe,
  /// What the app laid over the theme's recipe, which the lists in its items have too.
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl BulletedListView {
  /// The style of a part, as the list is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The list with nothing in it yet: a `list` to what reads the tree.
  pub fn root(&self) -> Column {
    let mut root = Column::new()
      .align_items(Alignment::Stretch)
      .overflow_visible()
      .part(&self.part(BulletedListPart::Root))
      .describe("a11y_role", "list");
    if let Some(label) = &self.label {
      root = root.describe("a11y_name", label.clone());
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root
  }

  /// How high one line of an item is: what its mark stands in the middle of.
  pub fn line_height(&self) -> f32 {
    let style = self.part(BulletedListPart::Label).rest().text.resolve(self.theme());
    style.font_size * style.line_height
  }

  /// What the `index`th item is numbered.
  pub fn number(&self, index: usize) -> String {
    (self.number_text)(self.start + index as u64)
  }

  /// The mark of the `index`th item, in a box as high as one line of it: the item's own icon or element, else the
  /// list's bullet, number or icon. A list with no marker has none, but for an item with a mark of its own.
  pub fn mark(&self, index: usize, icon: Option<Name>, media: Option<Element>) -> Option<Element> {
    let part = self.part(BulletedListPart::Mark);
    let held = Row::new()
      .height(self.line_height())
      .align_items(Alignment::Center)
      .justify(Justify::Center)
      .flex_shrink(0.0)
      .part(&part);
    if let Some(media) = media {
      // The app's mark is as wide as it is, and no narrower than the marks beside it.
      let least = part.rest().width.and_then(|width| width.resolve(self.theme()));
      let held = held.width(Dimension::Auto).overflow_visible();
      return Some(
        match least {
          Some(least) => held.min_width(least),
          None => held,
        }
        .child(media)
        .into(),
      );
    }
    let icon = icon.or(match &self.marker {
      BulletedListMarker::Icon(icon) => Some(*icon),
      _ => None,
    });
    if let Some(icon) = icon {
      let side = part.rest().width.unwrap_or(Length::Px(ICON));
      return Some(held.child(Icon::new(icon).size(side).build(self.theme())).into());
    }
    match self.marker {
      BulletedListMarker::Number => Some(
        held
          .justify(Justify::End)
          .child(part.text(&self.number(index)).nowrap())
          .into(),
      ),
      BulletedListMarker::None => None,
      // A list of icons draws a bullet for nothing: every item of it has the list's icon or its own.
      BulletedListMarker::Bullet | BulletedListMarker::Icon(_) => {
        Some(held.child(Row::new().part(&self.part(BulletedListPart::Bullet))).into())
      }
    }
  }

  /// What an item says, wrapping at the room it has; none where it says nothing.
  pub fn label(&self, text: &str) -> Option<Element> {
    if text.is_empty() {
      return None;
    }
    let part = self.part(BulletedListPart::Label);
    Some(part.text(text).part(&part).width(Dimension::Pct(100.0)).into())
  }

  /// The `index`th item: its mark, and beside it its line over what it holds and the list in it. A `listitem` to
  /// what reads the tree.
  pub fn item(&self, index: usize, item: BulletedListItem) -> Element {
    let mark = self.mark(index, item.icon, item.media);
    let mut body = Column::new()
      .flex(1.0)
      .min_width(0.0)
      .align_items(Alignment::Stretch)
      .overflow_visible()
      .part(&self.part(BulletedListPart::Body));
    if let Some(label) = self.label(&item.text) {
      body = body.child(label);
    }
    body = body.with_children(item.body);
    if let Some(mut list) = item.list {
      list.depth = self.depth + 1;
      // What the app said of this list it says of the lists in it, under what it said of each.
      list.rules = self.rules.clone().extend(&list.rules);
      body = body.child(list.build(self.theme()));
    }
    let mut row = Row::new()
      .align_items(Alignment::Start)
      .overflow_visible()
      .part(&self.part(BulletedListPart::Item))
      .describe("a11y_role", "listitem");
    if let Some(id) = item.id {
      row = row.id(id);
    }
    if let Some(key) = item.key {
      row = row.key(key);
    }
    row.with_children(mark).child(body).into()
  }

  /// How the default renderer builds a list: its items one under another.
  pub fn render(mut self) -> Element {
    let items = std::mem::take(&mut self.items);
    let rows: Vec<Element> = items
      .into_iter()
      .enumerate()
      .map(|(index, item)| self.item(index, item))
      .collect();
    self.root().with_children(rows).into()
  }
}

impl Themed for BulletedList {
  const NAME: &'static str = "bulleted_list";
  type Renderer = BulletedListRenderer;

  /// The design's list (`pencil/lurq-design-system.pen`, Bulleted List `btfDc`): its items 6 apart, each a dot of 5
  /// in `MutedForeground` in the middle of its first line, 10 before its text in the body's 14 in `Foreground`.
  /// Where the board is silent the list has the Markdown document's lists: a number is set at the end of a width of
  /// 18 in `MutedForeground`, 8 before the text; the bullet of a list in another's item is a ring; an icon is 14 in
  /// `MutedForeground`; what an item holds under its line is 6 apart.
  fn recipe() -> Recipe {
    use BulletedListPart::{Body, Bullet, Item, Label, Mark, Root};
    use BulletedListState::{Icon, Nested, Numbered};
    use PaletteColor::{Foreground, MutedForeground};
    Recipe::new()
      .base(Root, |s| {
        s.width(Length::Fill).gap(look::ITEM_GAP).foreground(Foreground)
      })
      .base(Item, |s| s.gap(look::BULLET_GAP))
      .base(Mark, |s| {
        s.width(look::BULLET)
          .typography(TypographyStyle::Body)
          .foreground(MutedForeground)
      })
      .base(Bullet, |s| {
        s.square(look::BULLET)
          .radius(RadiusSize::Full)
          .background(MutedForeground)
      })
      .base(Label, |s| s.typography(TypographyStyle::Body).foreground(Foreground))
      .base(Body, |s| s.gap(look::ITEM_GAP))
      .state(Numbered, Item, |s| s.gap(look::NUMBER_GAP))
      .state(Numbered, Mark, |s| s.width(look::NUMBER_WIDTH))
      .state(Icon, Mark, |s| s.width(ICON))
      .state(Nested, Bullet, |s| {
        s.background(Color::new(0, 0, 0, 0))
          .border(Border::inside(BorderSize::Sm, MutedForeground))
      })
  }
}
