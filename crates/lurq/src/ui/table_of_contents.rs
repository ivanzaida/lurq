//! A table of contents: the list of a page's sections, with the one being read marked.

use std::{
  sync::{Arc, Mutex},
  time::Duration,
};

use super::{
  names,
  part_style::{Length, PartStyle},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::{fading, one_line, pressable},
  theme::{Themed, current_breakpoint, current_theme},
};
use crate::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{BorderSize, OpacityLevel, PaletteColor, Theme, TypographyStyle},
  },
  components::{self, Column, Row, TextOverflow},
  core::{ElementRef, RectObserver, Signal},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{CursorIcon, Element, FocusGroup, Outline, border::Border, dimension::Dimension},
  time::Instant,
  ui::FontFamily,
};

/// How long the entry that was pressed stays the marked one while the page scrolls to its section.
const HELD: Duration = Duration::from_millis(250);

/// The parts of a table of contents a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TableOfContentsPart {
  /// The list: its width, and the room between its entries.
  Root,
  /// What the list is called, over its entries.
  Title,
  /// An entry: what is pressed, its height, the line at its start and the room before its label.
  Entry,
  /// The room an entry of a level under the first is set in by, for each level: its width.
  Indent,
  /// What an entry says.
  Label,
}

names!(TableOfContentsPart {
  Root => "root",
  Title => "title",
  Entry => "entry",
  Indent => "indent",
  Label => "label",
});

/// The states of a table of contents a recipe styles, besides what the pointer and the keyboard do to it. Each is
/// an entry's own, resolved on its parts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TableOfContentsState {
  /// The entry's section is the one being read.
  Current,
  /// The entry takes no input.
  Disabled,
}

names!(TableOfContentsState {
  Current => "current",
  Disabled => "disabled",
});

/// What builds a table of contents' nodes: a theme's in place of the default one.
pub type TableOfContentsRenderer = Arc<dyn Fn(TableOfContentsView) -> Element + Send + Sync>;

/// An entry of a [`TableOfContents`]: what a section is called, and the element that is the section.
///
/// ```
/// use lurq::{core::ElementRef, ui::TableOfContentsEntry};
///
/// let extra = ElementRef::new();
/// let entry = TableOfContentsEntry::new("Extra roles", &extra).level(2);
/// ```
#[derive(Clone)]
pub struct TableOfContentsEntry {
  label: Arc<str>,
  anchor: ElementRef,
  level: usize,
  disabled: bool,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl TableOfContentsEntry {
  /// An entry called `label` for the section `anchor` is on: the ref the app gave the section's element
  /// (`.ref_element(anchor.clone())`), or its heading.
  pub fn new(label: &str, anchor: &ElementRef) -> Self {
    Self {
      label: Arc::from(label),
      anchor: anchor.clone(),
      level: 1,
      disabled: false,
      id: None,
      key: None,
    }
  }

  /// How deep the section is: 1, the default, for a section of the page, 2 for a section of a section, which is
  /// set in.
  pub fn level(mut self, level: usize) -> Self {
    self.level = level.max(1);
    self
  }

  /// Whether the entry takes no input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
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
}

impl From<(&str, &ElementRef)> for TableOfContentsEntry {
  fn from((label, anchor): (&str, &ElementRef)) -> Self {
    Self::new(label, anchor)
  }
}

/// The list of a page's sections, with the one being read marked: "On this page" beside an article, the outline of
/// a document, the steps of a long form.
///
/// ```
/// use lurq::{
///   app::{component::Component, ctx::Ctx},
///   components::{Column, Row, ScrollVertical, Text},
///   core::ElementRef,
///   node::Element,
///   ui::{TableOfContents, TableOfContentsEntry},
/// };
///
/// struct Article {
///   page: ElementRef,
///   theme: ElementRef,
///   palette: ElementRef,
///   roles: ElementRef,
/// }
///
/// impl Component for Article {
///   type Props = ();
///
///   fn create(_ctx: &mut Ctx) -> Self {
///     Self {
///       page: ElementRef::new(),
///       theme: ElementRef::new(),
///       palette: ElementRef::new(),
///       roles: ElementRef::new(),
///     }
///   }
///
///   fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
///     let sections = Column::new()
///       .child(Text::new("Runtime theme").ref_element(self.theme.clone()))
///       .child(Text::new("Palette").ref_element(self.palette.clone()))
///       .child(Text::new("Extra roles").ref_element(self.roles.clone()));
///     // Mounted, it follows the scroll of the page and goes to a section when its entry is pressed.
///     let contents = TableOfContents::new([("Runtime theme", &self.theme), ("Palette", &self.palette)])
///       .entry(TableOfContentsEntry::new("Extra roles", &self.roles).level(2))
///       .container(&self.page)
///       .mount(ctx);
///     Row::new()
///       .child(ScrollVertical::new(sections).ref_element(self.page.clone()))
///       .child(contents)
///   }
/// }
/// ```
///
/// A section is an element of the app's with a ref on it, which the entry is given. Mounted
/// ([`TableOfContents::mount`]), the list follows the scroll: the entry marked is the last whose section's top has
/// come up to the reading line, a little under the top of the container the app names
/// ([`TableOfContents::container`]), or of the window where it names none. A press of an entry scrolls its section
/// to the start of what scrolls it ([`Ctx::scroll_into_view_start`]) and marks the entry, which stays the marked one
/// while the page gets there. Where the container is scrolled to its end, so that the last section is all in view,
/// the last entry is marked, though its top cannot come up to the reading line; scrolled back, the one whose section
/// is at the line again. The list adds observers of its own to the refs of its sections
/// ([`ElementRef::add_rect_observer_before_paint`]): one the app registered on them is kept.
///
/// Converted into an element on its own it is the still form of itself: it marks the entry its signal says
/// ([`TableOfContents::current`]), the first where it has none, follows no scroll, and a press tells its handlers
/// and scrolls nothing.
///
/// To the keyboard it is one stop of Tab, at the marked entry: Up and Down move between the entries, Home and End
/// go to the first and the last, Enter and Space go to the section. To what reads the tree it is a `navigation`
/// called by its label or its title, of `link`s, of which the marked one says it is the current location
/// (`a11y_current`).
#[derive(Clone)]
pub struct TableOfContents {
  entries: Vec<TableOfContentsEntry>,
  title: Arc<str>,
  container: Option<ElementRef>,
  reading_line: f32,
  current: Option<Signal<usize>>,
  on_change: Vec<Arc<dyn Fn(usize) + Send + Sync>>,
  on_select: Vec<Arc<dyn Fn(usize) + Send + Sync>>,
  label: Option<Arc<str>>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl TableOfContents {
  /// A list of `entries`: [`TableOfContentsEntry`]s, or pairs of a label and the ref of a section.
  pub fn new(entries: impl IntoIterator<Item = impl Into<TableOfContentsEntry>>) -> Self {
    Self {
      entries: entries.into_iter().map(Into::into).collect(),
      title: super::current_locale().navigation.on_this_page.clone(),
      container: None,
      reading_line: 24.0,
      current: None,
      on_change: Vec::new(),
      on_select: Vec::new(),
      label: None,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// Adds an entry.
  pub fn entry(mut self, entry: impl Into<TableOfContentsEntry>) -> Self {
    self.entries.push(entry.into());
    self
  }

  /// Adds entries.
  pub fn entries(mut self, entries: impl IntoIterator<Item = impl Into<TableOfContentsEntry>>) -> Self {
    self.entries.extend(entries.into_iter().map(Into::into));
    self
  }

  /// What the list is called, over its entries, in place of "On this page": in another language, or for another
  /// kind of list. An empty one draws no title.
  pub fn title(mut self, title: &str) -> Self {
    self.title = Arc::from(title);
    self
  }

  /// The container whose scroll the list follows: the element that scrolls the sections, by the ref the app gave
  /// it. The reading line is measured from its top. With none it is measured from the top of the window.
  pub fn container(mut self, container: &ElementRef) -> Self {
    self.container = Some(container.clone());
    self
  }

  /// How far under the top of the container a section's top is when the section becomes the one being read: 24
  /// unless told. More for a container whose top is under a bar that sticks over it.
  pub fn reading_line(mut self, offset: impl Into<f32>) -> Self {
    self.reading_line = offset.into();
    self
  }

  /// The entry marked, the app's: the list sets it as the page scrolls and as an entry is pressed, and marks what
  /// the app sets. Without it a mounted list keeps the entry itself.
  pub fn current(mut self, current: Signal<usize>) -> Self {
    self.current = Some(current);
    self
  }

  /// Called when another section becomes the one being read, by the scroll or by a press, with its place in the
  /// list: after the signal is set where the list has one, and not when the app sets it. A second handler is called
  /// after the first.
  pub fn on_change(mut self, handler: impl Fn(usize) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// Called when an entry is pressed, with its place in the list, before the list goes to its section. A second
  /// handler is called after the first.
  pub fn on_select(mut self, handler: impl Fn(usize) + Send + Sync + 'static) -> Self {
    self.on_select.push(Arc::new(handler));
    self
  }

  /// What the list is called to what reads the tree, in place of its title.
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

  /// Draws the entry Tab would stop on, the marked one, as it is under the pointer, held down or focused from the
  /// keyboard, with nothing pointing at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the list. Each entry has its own, where it was given one.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// The list mounted where it is declared: it follows the scroll of its container, keeps the entry marked where
  /// the app gave it no signal, and goes to a section when its entry is pressed.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let key = self.key.clone();
    let mut element = ctx.mount::<Host>(Props { contents: self });
    if let Some(key) = key {
      element = element.key(key);
    }
    element
  }

  /// Builds the list in `theme`, still: it marks what its signal says, the first entry where it has none.
  /// Converting it into an element does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let current = self.current.as_ref().map_or(0, Signal::get);
    let chosen = self.current.clone();
    let on_change = self.on_change.clone();
    // A press marks the entry, where the app gave a signal to mark it in, and tells of the change.
    let go: Arc<dyn Fn(usize) + Send + Sync> = Arc::new(move |index| {
      if let Some(chosen) = &chosen
        && chosen.get_untracked() != index
      {
        chosen.set(index);
        for handler in &on_change {
          handler(index);
        }
      }
    });
    self.build_with(theme, current, go)
  }

  /// Builds the list in `theme` with `current` marked, a press of an entry doing `go` after the app's handlers
  /// were told.
  fn build_with(self, theme: &Theme, current: usize, go: Arc<dyn Fn(usize) + Send + Sync>) -> Element {
    let mut case = Case::new().breakpoint(current_breakpoint());
    for state in &self.states {
      case = case.state(*state, true);
    }
    let resolve = |entry: Case| theme.resolve::<TableOfContents>(&entry, Some(&self.rules));
    let view = TableOfContentsView {
      current: current.min(self.entries.len().saturating_sub(1)),
      recipe: resolve(case.clone()),
      marked: resolve(case.clone().state(TableOfContentsState::Current, true)),
      disabled: resolve(case.clone().state(TableOfContentsState::Disabled, true)),
      entries: self.entries,
      title: self.title,
      label: self.label,
      forced: self.forced,
      on_select: self.on_select,
      go,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<TableOfContents>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<TableOfContents> for Element {
  fn from(contents: TableOfContents) -> Self {
    contents.build(&current_theme())
  }
}

/// A table of contents as a renderer sees it: its entries, the one marked, the style of its parts, and its behavior
/// ready to be dressed.
pub struct TableOfContentsView {
  pub entries: Vec<TableOfContentsEntry>,
  /// The place of the entry marked.
  pub current: usize,
  /// What the list is called over its entries: empty where it has no title.
  pub title: Arc<str>,
  pub label: Option<Arc<str>>,
  /// The style of each part of the list, and of an entry that is neither marked nor disabled.
  pub recipe: ResolvedRecipe,
  /// The style of the parts of the marked entry.
  marked: ResolvedRecipe,
  /// The style of the parts of an entry that takes no input.
  disabled: ResolvedRecipe,
  forced: Option<Interaction>,
  on_select: Vec<Arc<dyn Fn(usize) + Send + Sync>>,
  go: Arc<dyn Fn(usize) + Send + Sync>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl TableOfContentsEntry {
  /// What the entry says.
  pub fn label(&self) -> &str {
    &self.label
  }

  /// How deep its section is: 1 for a section of the page.
  pub fn depth(&self) -> usize {
    self.level
  }

  /// Whether it takes no input.
  pub fn is_disabled(&self) -> bool {
    self.disabled
  }
}

impl TableOfContentsView {
  /// The style of a part of the list, as it is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The recipe the `index`th entry is drawn by: the marked one's, a disabled one's, or the list's.
  pub fn entry_recipe(&self, index: usize) -> &ResolvedRecipe {
    if self.entries.get(index).is_some_and(|entry| entry.disabled) {
      &self.disabled
    } else if index == self.current {
      &self.marked
    } else {
      &self.recipe
    }
  }

  /// The list with nothing in it yet: a `navigation` to what reads the tree, called by its label or its title,
  /// and one group to the keys, which Tab stops in once.
  pub fn root(&self) -> Column {
    let name = self.label.clone().unwrap_or_else(|| self.title.clone());
    let mut root = Column::new()
      .align_items(Alignment::Stretch)
      .focus_group(FocusGroup::vertical().one_stop(true).wrap(false))
      .part(&self.part(TableOfContentsPart::Root))
      .describe("a11y_role", "navigation");
    if !name.is_empty() {
      root = root.describe("a11y_name", name);
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root
  }

  /// The title over the entries, in capitals as the design draws it; none where the list has no title.
  pub fn title(&self) -> Option<Row> {
    if self.title.is_empty() {
      return None;
    }
    let part = self.part(TableOfContentsPart::Title);
    Some(
      Row::new().align_items(Alignment::Center).part(&part).child(
        Row::new().flex_shrink(1.0).min_width(0.0).child(
          part
            .text(&self.title.to_uppercase())
            .nowrap()
            .text_overflow(TextOverflow::Elipsis),
        ),
      ),
    )
  }

  /// The `index`th entry: what is pressed and focused, set in by its level, its label on one line. A `link` to what
  /// reads the tree, which says whether it is the current location.
  pub fn entry(&self, index: usize) -> components::Button {
    let entry = &self.entries[index];
    let recipe = self.entry_recipe(index);
    let current = index == self.current;
    let part = recipe
      .part(TableOfContentsPart::Entry)
      .forced(self.forced.filter(|_| current));
    let label = recipe.part(TableOfContentsPart::Label);
    let indent = recipe.part(TableOfContentsPart::Indent).rest().width;
    let mut row = pressable(&part, entry.disabled)
      .justify(Justify::Start)
      .align_items(Alignment::Center)
      .focus_entry(current)
      .describe("a11y_role", "link")
      .describe("a11y_name", entry.label.clone());
    if current {
      row = row.describe("a11y_current", "location");
    }
    // An entry of a level under the first is set in by the indent for each level over it.
    if entry.level > 1
      && let Some(Dimension::Px(step)) = indent.and_then(|width| width.resolve(self.theme()))
    {
      row = row.child(Row::new().width(step * (entry.level - 1) as f32).flex_shrink(0.0));
    }
    row = row.child(one_line(&label, &entry.label));
    if !entry.disabled {
      let (told, go) = (self.on_select.clone(), self.go.clone());
      row = row.on_click(move |_| {
        for handler in &told {
          handler(index);
        }
        go(index);
      });
    }
    if let Some(id) = &entry.id {
      row = row.id(id.clone());
    }
    if let Some(key) = &entry.key {
      row = row.key(key.clone());
    }
    row
  }

  /// How the default renderer builds a table of contents: its title over its entries.
  pub fn render(self) -> Element {
    let entries: Vec<Element> = (0..self.entries.len()).map(|index| self.entry(index).into()).collect();
    self
      .root()
      .with_children(self.title().map(Element::from))
      .with_children(entries)
      .into()
  }
}

impl crate::ui::Styled for TableOfContents {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for TableOfContents {
  const NAME: &'static str = "table_of_contents";
  type Renderer = TableOfContentsRenderer;

  fn parts() -> crate::ui::Declared {
    TableOfContentsPart::declared()
  }

  fn states() -> crate::ui::Declared {
    TableOfContentsState::declared()
  }

  /// The design's list (`pencil/lurq-design-system.pen`, Table of Contents `i7m0J`): 200 wide, its entries 2
  /// apart under a title in the mono face at 11, in capitals a pixel apart, in `MutedForeground`, 12 in and 6 over
  /// the first entry. An entry is 28 high with a line of 1 in `Border` at its start, its label 12 after it in 13
  /// in `MutedForeground`; an entry of the second level is 12 further in. The marked one has a line of 2 in
  /// `Primary` and its label in `Foreground` at medium weight. Where the board is silent: an entry's label is in
  /// `Foreground` under the pointer, as the gallery's list had it; the focus is a line of 2 in the ring's color
  /// inside the entry; a disabled one is at the theme's disabled opacity; an entry is a row, and follows the
  /// density.
  fn recipe() -> Recipe {
    use PaletteColor::{Foreground, MutedForeground, Primary, Ring};
    use TableOfContentsPart::{Entry, Indent, Label, Root, Title};
    use TableOfContentsState::{Current, Disabled};
    Recipe::new()
      .base(Root, |s| s.width(200.0).max_width(Length::Fill).gap(2.0))
      .base(Title, |s| {
        s.padding_start(12.0)
          .padding_bottom(6.0)
          .font_family(FontFamily::Mono)
          .font_size(11.0)
          // The board's line of 15, where the face's own is 14.3: a title of 21 with the 6 under it.
          .line_height(15.0 / 11.0)
          .letter_spacing(1.0)
          .foreground(MutedForeground)
      })
      .base(Entry, |s| {
        fading(
          s.height(28.0)
            .padding_start(12.0)
            .start_line(Border::inside(BorderSize::Sm, PaletteColor::Border))
            .foreground(MutedForeground)
            .cursor(CursorIcon::Pointer),
        )
      })
      .row_density(Selector::new(), Entry, 28.0)
      .base(Indent, |s| s.width(12.0))
      .base(Label, |s| s.typography(TypographyStyle::Body).font_size(13.0))
      .hovered(Entry, |s| s.foreground(Foreground))
      .focus_visible(Entry, |s| s.outline(Outline::new(2.0, Ring).offset(-2.0)))
      .state(Current, Entry, |s| {
        s.start_line(Border::inside(BorderSize::Md, Primary))
          .foreground(Foreground)
      })
      .state(Current, Label, |s| s.weight(FontWeight::Medium))
      .state(Disabled, Entry, |s| {
        s.opacity(OpacityLevel::Disabled).cursor(CursorIcon::Default)
      })
      .when(Selector::new().state(Disabled).hovered(), Entry, |s| {
        s.foreground(MutedForeground)
      })
  }
}

/// A table of contents.
#[derive(Clone)]
struct Props {
  contents: TableOfContents,
}

/// The list is built anew by every render of what holds it, so props are never the same as before.
impl PartialEq for Props {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

impl crate::app::component::DevtoolsInspectable for Props {}

/// The entry a scroll marks, of sections whose tops and bottoms in the window are `spans`: the last whose top has come
/// up to the reading line, `offset` under the top of the container (of the window where there is none), or the last
/// of all where the container is scrolled to its end: the last section is all in it, and the first has gone up out
/// of it.
fn marked_by_scroll(spans: &[(f32, f32)], container: Option<&ElementRef>, offset: f32) -> usize {
  let (top, bottom) = container.map_or((0.0, None), |container| {
    let (_, y, _, height) = container.rect();
    (y, Some(y + height))
  });
  let line = top + offset;
  let read = spans.iter().rposition(|(start, _)| *start <= line).unwrap_or(0);
  match (bottom, spans.first(), spans.last()) {
    (Some(bottom), Some((first, _)), Some((_, end))) if *first < top - 0.5 && *end <= bottom + 0.5 => spans.len() - 1,
    _ => read,
  }
}

/// What a mounted table of contents keeps between renders.
struct Host {
  /// The entry marked, where the app gave no signal for it.
  own: Signal<usize>,
  /// Where the top and the bottom of each section are in the window, as they were last laid out.
  tops: Arc<Mutex<Vec<(f32, f32)>>>,
  /// The observers it added to the sections' refs in its last render: replaced by each render, and removed with
  /// the list.
  watching: Mutex<Vec<RectObserver>>,
  /// Until when the entry that was pressed stays the marked one, wherever the page scrolls to.
  held: Arc<Mutex<Option<Instant>>>,
  /// The entry whose section the next render scrolls to.
  wanted: Arc<Mutex<Option<usize>>>,
  /// Bumped when an entry is pressed, so that the list renders and scrolls to its section.
  asked: Signal<u32>,
}

impl Component for Host {
  type Props = Props;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      own: ctx.signal(0),
      tops: Arc::default(),
      watching: Mutex::default(),
      held: Arc::default(),
      wanted: Arc::default(),
      asked: ctx.signal(0),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let contents = ctx.props::<Props>().contents.clone();
    let current = contents.current.clone().unwrap_or_else(|| self.own.clone());
    let count = contents.entries.len();
    self
      .tops
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner())
      .resize(count, (f32::MAX, f32::MAX));
    // The entry marked is the last whose section's top has come up to the reading line, or the last of all where
    // the container is scrolled to its end. A section says where it is whenever that changes, which scrolling does.
    let mut watching = Vec::with_capacity(count);
    for (index, entry) in contents.entries.iter().enumerate() {
      let (tops, held, current) = (self.tops.clone(), self.held.clone(), current.clone());
      let (container, offset) = (contents.container.clone(), contents.reading_line);
      let on_change = contents.on_change.clone();
      watching.push(entry.anchor.add_rect_observer_before_paint(move |rect| {
        let mut tops = tops.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if index >= tops.len() {
          return;
        }
        tops[index] = (rect.y, rect.y + rect.height);
        if held
          .lock()
          .unwrap_or_else(|poisoned| poisoned.into_inner())
          .is_some_and(|until| Instant::now() < until)
        {
          return;
        }
        let read = marked_by_scroll(&tops, container.as_ref(), offset);
        drop(tops);
        if current.get_untracked() != read {
          current.set(read);
          for handler in &on_change {
            handler(read);
          }
        }
      }));
    }
    // The observers of the render before are removed as these take their place.
    *self.watching.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = watching;
    // Reading the signal is what renders the list when an entry is pressed.
    let _ = self.asked.get();
    if let Some(wanted) = self
      .wanted
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner())
      .take()
      && let Some(entry) = contents.entries.get(wanted)
    {
      ctx.scroll_into_view_start(&entry.anchor);
    }
    let go: Arc<dyn Fn(usize) + Send + Sync> = {
      let (wanted, held, asked) = (self.wanted.clone(), self.held.clone(), self.asked.clone());
      let (current, on_change) = (current.clone(), contents.on_change.clone());
      Arc::new(move |index| {
        *wanted.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(index);
        // A section near the end of what scrolls cannot come up to the reading line: its entry is the marked one
        // because it was pressed, until the page is scrolled again.
        *held.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(Instant::now() + HELD);
        if current.get_untracked() != index {
          current.set(index);
          for handler in &on_change {
            handler(index);
          }
        }
        asked.update(|count| *count = count.wrapping_add(1));
      })
    };
    let marked = current.get();
    contents.build_with(&current_theme(), marked, go)
  }
}
