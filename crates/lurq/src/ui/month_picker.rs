//! A month picker: the twelve months of a year in a grid, a month of which a person chooses, and the years before and
//! after under the buttons of its heading. What it shares with a year picker, the grid and its keys, is here too.

use std::{marker::PhantomData, sync::Arc};

use super::{
  button::{Button, ButtonPart, ButtonSize, ButtonVariant},
  calendar::{DateMemory, MonthSource},
  date_field::FieldValue,
  field::{FieldControl, InField},
  icons, names,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::{fading, pressable},
  text_box::{Judgment, TextBoxState},
  theme::{Themed, current_date_locale, current_theme},
};
use crate::{
  app::{
    component::Component,
    ctx::Ctx,
    events::{KeyboardEvent, MouseEvent},
    theme::{BorderSize, OpacityLevel, PaletteColor, RadiusSize, ShadowStyle, Theme, TypographyStyle},
  },
  components::{Column, Row},
  core::{DateText, Signal, YearMonth},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{CursorIcon, Element, FocusGroup, border::Border, color::Color},
};

/// The parts of a month picker and of a year picker a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MonthPickerPart {
  /// The panel: its fill, its edge, its corners, its shadow, its padding, and the room between the heading and the
  /// grid.
  Root,
  /// The row of the heading: the button before, the title, the button after.
  Header,
  /// The year, or the years, in the heading.
  Title,
  /// What the picker says of a button that shows the year (or the years) before or after: rules over the button's
  /// root.
  Nav,
  /// What the picker says of the glyph of those buttons: rules over the button's icon.
  NavIcon,
  /// The rows of the cells, one under another: its gap is the room between them.
  Grid,
  /// A row of four cells: its gap is the room between them.
  Row,
  /// A month, or a year: what is pressed, focused and chosen.
  Cell,
}

names!(MonthPickerPart {
  Root => "root",
  Header => "header",
  Title => "title",
  Nav => "nav",
  NavIcon => "nav_icon",
  Grid => "grid",
  Row => "row",
  Cell => "cell",
});

/// The states of a month picker and of a year picker a recipe styles. The picker is in `Disabled`; a cell in the
/// others.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MonthPickerState {
  /// The picker takes no input.
  Disabled,
  /// The cell is the chosen one.
  Selected,
  /// The cell is this month, or this year.
  Current,
  /// The cell cannot be chosen: out of the range.
  Unavailable,
}

names!(MonthPickerState {
  Disabled => "disabled",
  Selected => "selected",
  Current => "current",
  Unavailable => "unavailable",
});

/// What a grid of a picker shows a page of: a month of a year, or a year.
pub(crate) trait Unit: Copy + Ord + Send + Sync + std::fmt::Debug + 'static {
  /// The page it is on: its year, or the first year of its decade.
  fn page(self) -> i32;
  /// The twelve cells of `page`.
  fn cells(page: i32) -> Vec<Self>;
  /// The cell `by` after it, where there is one.
  fn shift(self, by: i32) -> Option<Self>;
  /// How far a page goes: a year, or ten.
  const PAGE: i32;
  /// What its cell says: `Nov`, `2026`.
  fn label(self, text: &dyn DateText) -> String;
  /// What its cell is called to what reads the tree: `November 2026`, `2026`.
  fn name(self, text: &dyn DateText) -> String;
  /// The heading of `page`: `2026`, `2020 – 2031`.
  fn title(page: i32, text: &dyn DateText) -> String;
  /// What the buttons of the heading are called.
  fn nav_labels(text: &dyn DateText) -> (String, String);
  /// What its id and its key end in: `2026-11`, `2026`.
  fn tag(self) -> String;
  /// The page as the month a memory keeps it in.
  fn as_month(page: i32) -> YearMonth;
}

impl Unit for YearMonth {
  const PAGE: i32 = 1;

  fn page(self) -> i32 {
    self.year()
  }

  fn cells(page: i32) -> Vec<Self> {
    (1..=12).filter_map(|month| YearMonth::new(page, month)).collect()
  }

  fn shift(self, by: i32) -> Option<Self> {
    let shifted = self.add_months(by);
    (shifted.months_since(self) == by).then_some(shifted)
  }

  fn label(self, text: &dyn DateText) -> String {
    text.month_short(self.month())
  }

  fn name(self, text: &dyn DateText) -> String {
    text.month_title(self)
  }

  fn title(page: i32, _: &dyn DateText) -> String {
    page.to_string()
  }

  fn nav_labels(text: &dyn DateText) -> (String, String) {
    (text.previous_year_label(), text.next_year_label())
  }

  fn tag(self) -> String {
    self.to_string()
  }

  fn as_month(page: i32) -> YearMonth {
    YearMonth::new(page.clamp(1, 9999), 1).unwrap_or_else(|| YearMonth::new(2000, 1).unwrap())
  }
}

impl Unit for i32 {
  const PAGE: i32 = 10;

  fn page(self) -> i32 {
    self - self.rem_euclid(10)
  }

  fn cells(page: i32) -> Vec<Self> {
    (page..page + 12).filter(|year| (1..=9999).contains(year)).collect()
  }

  fn shift(self, by: i32) -> Option<Self> {
    let year = self + by;
    (1..=9999).contains(&year).then_some(year)
  }

  fn label(self, _: &dyn DateText) -> String {
    self.to_string()
  }

  fn name(self, _: &dyn DateText) -> String {
    self.to_string()
  }

  fn title(page: i32, text: &dyn DateText) -> String {
    text.year_range_title(page.max(1), (page + 11).min(9999))
  }

  fn nav_labels(text: &dyn DateText) -> (String, String) {
    (text.previous_years_label(), text.next_years_label())
  }

  fn tag(self) -> String {
    self.to_string()
  }

  fn as_month(page: i32) -> YearMonth {
    <YearMonth as Unit>::as_month(page)
  }
}

/// What a month picker and a year picker are built from.
#[derive(Clone)]
pub(crate) struct Periods<U: Unit, V: FieldValue<U>> {
  pub value: Signal<V>,
  pub controlled: bool,
  pub min: Option<U>,
  pub max: Option<U>,
  pub current: Option<U>,
  pub locale: Option<Arc<dyn DateText>>,
  pub disabled: bool,
  pub invalid: bool,
  pub label: Option<Arc<str>>,
  pub previous_label: Option<Arc<str>>,
  pub next_label: Option<Arc<str>>,
  pub on_change: Vec<Arc<dyn Fn(V) + Send + Sync>>,
  pub memory: Option<DateMemory>,
  pub states: Vec<Name>,
  pub rules: Recipe,
  pub forced: Option<Interaction>,
  pub id: Option<Arc<str>>,
  pub key: Option<Arc<str>>,
}

impl<U: Unit, V: FieldValue<U>> Periods<U, V> {
  pub fn new(value: Signal<V>, controlled: bool) -> Self {
    Self {
      value,
      controlled,
      min: None,
      max: None,
      current: None,
      locale: None,
      disabled: false,
      invalid: false,
      label: None,
      previous_label: None,
      next_label: None,
      on_change: Vec::new(),
      memory: None,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// Takes what the field it is in says: the field's label where the picker has no name of its own, invalid where
  /// the field is, and disabled where it is. A panel of a grid has no fluid form: a fluid field draws it plain.
  pub fn in_field(&mut self, field: &InField) {
    if self.label.is_none() && !field.label.is_empty() {
      self.label = Some(field.label.clone());
    }
    self.invalid |= field.judgment == Some(Judgment::Invalid);
    self.disabled |= field.disabled;
  }

  /// The view of the picker in `theme`, as the themed component `C` is drawn; `now` is this month or this year.
  pub fn view<C: Themed>(self, theme: &Theme, now: U) -> PeriodsView {
    let text = self.locale.clone().unwrap_or_else(current_date_locale);
    let value = self.value.get().get();
    let current = self.current.unwrap_or(now);
    // The page shown is kept where the picker has a memory; it starts at the page of what is chosen, or of now, in
    // the range.
    let start = value
      .unwrap_or(current)
      .max(self.min.unwrap_or(current).min(current))
      .page();
    let start = self.min.map_or(start, |min| start.max(min.page()));
    let page = match &self.memory {
      Some(memory) => MonthSource::Own(memory.month.clone(), U::as_month(start)),
      None => MonthSource::Own(Signal::new(None), U::as_month(start)),
    };
    let shown = page.get().year();
    let shown = if U::PAGE == 10 {
      shown - shown.rem_euclid(10)
    } else {
      shown
    };
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(MonthPickerState::Disabled, self.disabled)
    .state(TextBoxState::Invalid, self.invalid);
    let recipe = theme.resolve::<C>(&case, Some(&self.rules)).owned_by(self.id.as_ref());
    let resolve: Resolve = {
      let (theme, rules, owner) = (theme.clone(), self.rules.clone(), recipe.clone());
      Arc::new(move |case: &Case| theme.resolve::<C>(case, Some(&rules)).owned_like(&owner))
    };
    let available = |cell: U| self.min.is_none_or(|min| cell >= min) && self.max.is_none_or(|max| cell <= max);
    let cells: Vec<U> = U::cells(shown);
    let entry = value
      .filter(|value| cells.contains(value))
      .or_else(|| cells.contains(&current).then_some(current))
      .or_else(|| cells.iter().copied().find(|cell| available(*cell)))
      .unwrap_or(cells[0]);
    let shown_cells = cells
      .iter()
      .map(|cell| CellShown {
        label: cell.label(&*text),
        name: cell.name(&*text),
        tag: cell.tag(),
        selected: value == Some(*cell),
        current: *cell == current,
        unavailable: !available(*cell),
        entry: *cell == entry,
      })
      .collect();
    let (previous, next) = U::nav_labels(&*text);
    let first = cells[0];
    let last = cells[cells.len() - 1];
    let model: Arc<dyn PeriodModel> = Arc::new(Model {
      value: self.value.clone(),
      controlled: self.controlled,
      on_change: self.on_change.clone(),
      page,
      cells,
      min: self.min,
      max: self.max,
    });
    PeriodsView {
      title: U::title(shown, &*text),
      cells: shown_cells,
      before: self
        .min
        .is_none_or(|min| first.shift(-1).is_some_and(|before| before >= min)),
      after: self
        .max
        .is_none_or(|max| last.shift(1).is_some_and(|after| after <= max)),
      previous_label: self.previous_label.map_or(previous, |label| label.to_string()),
      next_label: self.next_label.map_or(next, |label| label.to_string()),
      label: self.label,
      disabled: self.disabled,
      invalid: self.invalid,
      recipe,
      case,
      resolve,
      forced: self.forced,
      model,
      id: self.id,
      key: self.key,
    }
  }
}

/// How a picker's recipe is resolved in a case of its own: a cell's.
type Resolve = Arc<dyn Fn(&Case) -> ResolvedRecipe + Send + Sync>;

/// What a cell shows, as a renderer sees it.
#[derive(Clone, Debug)]
pub struct CellShown {
  /// What it says: `Nov`, `2026`.
  pub label: String,
  /// What it is called to what reads the tree: `November 2026`.
  pub name: String,
  /// What its id ends in: `2026-11`, `2026`.
  pub tag: String,
  pub selected: bool,
  pub current: bool,
  pub unavailable: bool,
  /// Whether Tab enters the grid at it: the chosen one, this one, or the first that can be chosen.
  pub entry: bool,
}

/// What the handlers of a picker's grid do.
pub(crate) trait PeriodModel: Send + Sync {
  /// Chooses the cell at `index`.
  fn choose(&self, index: usize);
  /// Shows the page `by` pages on.
  fn turn(&self, by: i32);
  /// What a key on the cell at `index` does.
  fn key(&self, index: usize, event: &KeyboardEvent);
}

struct Model<U: Unit, V: FieldValue<U>> {
  value: Signal<V>,
  controlled: bool,
  on_change: Vec<Arc<dyn Fn(V) + Send + Sync>>,
  page: MonthSource,
  cells: Vec<U>,
  min: Option<U>,
  max: Option<U>,
}

impl<U: Unit, V: FieldValue<U>> Model<U, V> {
  fn available(&self, cell: U) -> bool {
    self.min.is_none_or(|min| cell >= min) && self.max.is_none_or(|max| cell <= max)
  }

  fn write(&self, cell: U) {
    let Some(value) = V::of(Some(cell)) else {
      return;
    };
    if self.value.get_untracked() == value {
      return;
    }
    if !self.controlled {
      self.value.set(value);
    }
    for handler in &self.on_change {
      handler(value);
    }
  }

  fn show(&self, cell: U) {
    let page = U::as_month(cell.page());
    self.page.set(page);
  }
}

impl<U: Unit, V: FieldValue<U>> PeriodModel for Model<U, V> {
  fn choose(&self, index: usize) {
    if let Some(cell) = self.cells.get(index).copied().filter(|cell| self.available(*cell)) {
      self.write(cell);
    }
  }

  fn turn(&self, by: i32) {
    let page = self.page.get_untracked().year() + by * U::PAGE;
    self.page.set(U::as_month(page));
  }

  fn key(&self, index: usize, event: &KeyboardEvent) {
    if !event.focus_within || event.ctrl || event.alt || event.meta || event.composing {
      return;
    }
    let Some(&cell) = self.cells.get(index) else {
      return;
    };
    let column = (index % 4) as i32;
    // A page of months is a year: PageUp is the same month a year before. A page of years is ten.
    let page = if U::PAGE == 1 { 12 } else { U::PAGE };
    let by = match event.key.as_str() {
      "ArrowLeft" => -1,
      "ArrowRight" => 1,
      "ArrowUp" => -4,
      "ArrowDown" => 4,
      "Home" => -column,
      "End" => 3 - column,
      "PageUp" => -page,
      "PageDown" => page,
      _ => return,
    };
    let Some(target) = cell.shift(by) else {
      event.prevent_default();
      return;
    };
    let target = match (self.min, self.max) {
      (Some(min), _) if target < min => min,
      (_, Some(max)) if target > max => max,
      _ => target,
    };
    if !self.cells.contains(&target) {
      self.show(target);
    }
    event.focus_key(cell_key(&target.tag()));
  }
}

/// The key of a cell: what the keys move the focus to.
fn cell_key(tag: &str) -> String {
  format!("cell-{tag}")
}

/// A month picker or a year picker as a renderer sees it: its page, its cells, the style of its parts, and its
/// behavior ready to be dressed.
pub struct PeriodsView {
  /// The heading: `2026`, `2020 – 2031`.
  pub title: String,
  /// The twelve cells, four to a row.
  pub cells: Vec<CellShown>,
  /// Whether there is a page before and after in the range.
  pub before: bool,
  pub after: bool,
  pub previous_label: String,
  pub next_label: String,
  pub label: Option<Arc<str>>,
  pub disabled: bool,
  pub invalid: bool,
  /// The style of each part of this picker.
  pub recipe: ResolvedRecipe,
  case: Case,
  resolve: Resolve,
  forced: Option<Interaction>,
  model: Arc<dyn PeriodModel>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl PeriodsView {
  /// The style of a part, as this picker is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// A button of the heading: the page before (`by` -1) or after (1), a bare glyph in the design.
  pub fn nav(&self, by: i32) -> Element {
    let (label, icon, suffix, there) = if by < 0 {
      (&self.previous_label, icons::CHEVRON_LEFT, "previous", self.before)
    } else {
      (&self.next_label, icons::CHEVRON_RIGHT, "next", self.after)
    };
    let rules = self
      .recipe
      .rules_for(MonthPickerPart::Nav, ButtonPart::Root)
      .extend(&self.recipe.rules_for(MonthPickerPart::NavIcon, ButtonPart::Icon));
    let model = self.model.clone();
    let mut button = Button::new(label)
      .variant(ButtonVariant::Ghost)
      .size(ButtonSize::Xs)
      .icon(icon)
      .icon_only(true)
      .disabled(self.disabled || !there)
      .rules(&rules)
      .on_click(move |_: MouseEvent| model.turn(by));
    // Named by its side, whether the id is the component's own or one it was lent.
    if let Some(id) = self.recipe.keyed_id(MonthPickerPart::Nav, suffix) {
      button = button.id(id);
    }
    button.build(self.theme())
  }

  /// The heading: the button before, the title, the button after.
  pub fn header(&self) -> Row {
    let title = self.part(MonthPickerPart::Title);
    Row::new()
      .align_items(Alignment::Center)
      .justify(Justify::SpaceBetween)
      .part(&self.part(MonthPickerPart::Header))
      .child(self.nav(-1))
      .child(Row::new().part(&title).child(title.text(&self.title).nowrap()))
      .child(self.nav(1))
  }

  /// The style of the cell at `index`, in its states.
  fn cell_part(&self, index: usize) -> ResolvedPart {
    let shown = &self.cells[index];
    let case = self
      .case
      .clone()
      .state(MonthPickerState::Selected, shown.selected)
      .state(MonthPickerState::Current, shown.current)
      .state(MonthPickerState::Unavailable, shown.unavailable);
    // A cell is named by its month or year, not by its place: `x-cell-2026-11`.
    (self.resolve)(&case)
      .part(MonthPickerPart::Cell)
      .keyed(&shown.tag)
      .forced(self.forced.filter(|_| shown.entry))
  }

  /// The cell at `index`: pressed or Enter to choose it; the keys go to the cell beside, above and below, and on to
  /// the page before or after. A `gridcell` to what reads the tree.
  pub fn cell(&self, index: usize) -> Element {
    let shown = &self.cells[index];
    let part = self.cell_part(index);
    let (press, keys) = (self.model.clone(), self.model.clone());
    let mut cell = pressable(&part, self.disabled)
      .align_items(Alignment::Center)
      .key(cell_key(&shown.tag))
      .focus_entry(shown.entry)
      .describe("a11y_role", "gridcell")
      .describe("a11y_name", shown.name.clone())
      .describe("a11y_selected", if shown.selected { "true" } else { "false" })
      .on_key_down(move |event: KeyboardEvent| keys.key(index, &event))
      .child(part.text(&shown.label).nowrap());
    if shown.current {
      cell = cell.describe("a11y_current", "date");
    }
    if shown.unavailable {
      cell = cell.describe("a11y_disabled", "true");
    } else if !self.disabled {
      cell = cell.on_click(move |_: MouseEvent| press.choose(index));
    }
    cell.flex(1.0).into()
  }

  /// The cells, four to a row: one focus group, which Tab stops in once at the entry cell.
  pub fn grid(&self) -> Column {
    let mut grid = Column::new()
      .align_items(Alignment::Stretch)
      .part(&self.part(MonthPickerPart::Grid))
      .describe("a11y_role", "grid")
      .describe("a11y_name", self.title.clone())
      .focus_group(FocusGroup::both().one_stop(true).wrap(false));
    for start in (0..self.cells.len()).step_by(4) {
      let mut row = Row::new()
        .part(&self.part(MonthPickerPart::Row))
        .describe("a11y_role", "row");
      for index in start..(start + 4).min(self.cells.len()) {
        row = row.child(self.cell(index));
      }
      grid = grid.child(row);
    }
    grid
  }

  /// How the default renderer builds the picker: the heading over the grid, in the panel.
  pub fn render(self) -> Element {
    let mut panel = Column::new()
      .align_items(Alignment::Stretch)
      .part(&self.part(MonthPickerPart::Root))
      .child(self.header())
      .child(self.grid());
    if let Some(label) = &self.label {
      panel = panel
        .describe("a11y_role", "group")
        .describe("a11y_name", label.clone());
    }
    if self.invalid {
      panel = panel.describe("a11y_invalid", "true");
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

/// The recipe of a month picker and of a year picker.
pub(crate) fn periods_recipe() -> Recipe {
  use MonthPickerPart::{Cell, Grid, Header, Nav, NavIcon, Root, Row as Line, Title};
  use MonthPickerState::{Current, Disabled, Selected, Unavailable};
  use PaletteColor::{
    Border as Edge, Destructive, Foreground, Highlight, HighlightForeground, MutedForeground, Popover,
    PopoverForeground, Primary, PrimaryForeground,
  };
  let none = Color::new(0, 0, 0, 0);
  Recipe::new()
    .base(Root, |s| {
      s.width(252.0)
        .padding_x(12.0)
        .padding_y(12.0)
        .gap(8.0)
        .radius(RadiusSize::Lg)
        .background(Popover)
        .foreground(PopoverForeground)
        .border(Border::inside(BorderSize::Sm, Edge))
        .shadow(ShadowStyle::Md)
    })
    .base(Header, |s| s.height(28.0))
    .base(Title, |s| {
      s.typography(TypographyStyle::Body)
        .weight(FontWeight::Medium)
        .foreground(Foreground)
    })
    .base(Nav, |s| {
      s.square(16.0)
        .padding_x(0.0)
        .padding_y(0.0)
        .radius(RadiusSize::Sm)
        .background(none)
    })
    .base(NavIcon, |s| s.square(16.0).foreground(Foreground))
    .base(Grid, |s| s.gap(8.0))
    .base(Line, |s| s.gap(4.0))
    .base(Cell, |s| {
      fading(
        s.height(34.0)
          .radius(RadiusSize::Md)
          .typography(TypographyStyle::Body)
          .font_size(13.0)
          .foreground(Foreground)
          .cursor(CursorIcon::Pointer),
      )
    })
    .hovered(Cell, |s| s.background(Highlight).foreground(HighlightForeground))
    .focus_visible(Cell, |s| s.focus_ring())
    .state(Current, Cell, |s| {
      s.background(Highlight).foreground(HighlightForeground)
    })
    .state(Unavailable, Cell, |s| {
      s.foreground(MutedForeground).opacity(0.5).cursor(CursorIcon::Default)
    })
    .when(Selector::new().state(Unavailable).hovered(), Cell, |s| {
      s.background(none).foreground(MutedForeground)
    })
    .state(Selected, Cell, |s| {
      s.background(Primary)
        .foreground(PrimaryForeground)
        .weight(FontWeight::Medium)
    })
    .when(Selector::new().state(Selected).hovered(), Cell, |s| {
      s.background(Primary).foreground(PrimaryForeground)
    })
    .state(Disabled, Root, |s| s.opacity(OpacityLevel::Disabled))
    .state(Disabled, Cell, |s| s.cursor(CursorIcon::Default))
    .state(TextBoxState::Invalid, Root, |s| {
      s.border(Border::inside(BorderSize::Sm, Destructive))
    })
}

/// The builder methods a month picker and a year picker share.
macro_rules! period_methods {
  ($unit:ty) => {
    /// The least that can be chosen: the cells before it cannot, and the pages before its page are not shown.
    pub fn min(mut self, min: $unit) -> Self {
      self.core.min = Some(min);
      self
    }

    /// The greatest that can be chosen.
    pub fn max(mut self, max: $unit) -> Self {
      self.core.max = Some(max);
      self
    }

    /// The names of the months and of the buttons, in place of the app's
    /// ([`App::set_date_locale`](crate::app::App::set_date_locale), English where it set none).
    pub fn locale(mut self, locale: impl crate::core::DateText + 'static) -> Self {
      self.core.locale = Some(crate::core::shared_date_text(locale));
      self
    }

    /// Whether the picker takes no input.
    pub fn disabled(mut self, disabled: bool) -> Self {
      self.core.disabled = disabled;
      self
    }

    /// Whether what the picker holds is wrong: its edge is in the destructive color.
    pub fn invalid(mut self, invalid: bool) -> Self {
      self.core.invalid = invalid;
      self
    }

    /// What the picker is called to what reads the tree: its panel is then a `group`.
    pub fn label(mut self, label: &str) -> Self {
      self.core.label = Some(std::sync::Arc::from(label));
      self
    }

    /// What the button that shows the page before is called, in place of the locale's.
    pub fn previous_label(mut self, label: &str) -> Self {
      self.core.previous_label = Some(std::sync::Arc::from(label));
      self
    }

    /// What the button that shows the page after is called, in place of the locale's.
    pub fn next_label(mut self, label: &str) -> Self {
      self.core.next_label = Some(std::sync::Arc::from(label));
      self
    }

    /// Called when a person chooses another cell, with what is chosen. Bound to a signal, the value is set already.
    /// Not called for what is chosen already, nor when the app sets the value. A second handler is called after the
    /// first.
    pub fn on_change(mut self, handler: impl Fn(V) + Send + Sync + 'static) -> Self {
      self.core.on_change.push(std::sync::Arc::new(handler));
      self
    }

    /// What keeps the page shown from one render to the next: the app's, made once. A mounted picker keeps its own.
    pub fn memory(mut self, memory: &crate::ui::DateMemory) -> Self {
      self.core.memory = Some(memory.clone());
      self
    }

    /// Puts the picker in a state the theme's recipe names, or takes it out.
    pub fn state(mut self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
      let state = state.into();
      self.core.states.retain(|held| *held != state);
      if on {
        self.core.states.push(state);
      }
      self
    }

    /// Styles one part of this picker over what the theme gives it.
    pub fn style(
      mut self,
      part: impl Into<crate::ui::Name>,
      style: impl FnOnce(crate::ui::PartStyle) -> crate::ui::PartStyle,
    ) -> Self {
      self.core.rules = self.core.rules.base(part, style);
      self
    }

    /// Lays rules over the theme's recipe for this picker.
    pub fn rules(mut self, rules: &crate::ui::Recipe) -> Self {
      self.core.rules = self.core.rules.extend(rules);
      self
    }

    /// Draws the cell Tab would enter at as it is under the pointer, held down or focused, with nothing pointing at
    /// it: for a gallery or a test.
    pub fn force(mut self, interaction: Option<crate::ui::Interaction>) -> Self {
      self.core.forced = interaction;
      self
    }

    /// The id of the panel. Each cell has it with `-cell-` and its month or year after it (`due-cell-2026-11`,
    /// `founded-cell-2026`); the buttons have it with `-nav-previous` and `-nav-next`.
    pub fn id(mut self, id: impl Into<std::sync::Arc<str>>) -> Self {
      self.core.id = Some(id.into());
      self
    }

    pub fn key(mut self, key: impl Into<std::sync::Arc<str>>) -> Self {
      self.core.key = Some(key.into());
      self
    }
  };
}
pub(crate) use period_methods;

/// A month picker: the twelve months of a year, of which a person chooses one, and the years before and after under
/// the buttons of its heading. The month of a report, the expiry of a card.
///
/// ```
/// use lurq::{
///   core::{Signal, YearMonth},
///   ui::MonthPicker,
/// };
///
/// # fn render(ctx: &mut lurq::app::ctx::Ctx) {
/// let month = Signal::new(YearMonth::new(2026, 11).unwrap());
/// // Mounted, so that it keeps the year it shows.
/// let picker = MonthPicker::new(month.clone()).min(YearMonth::new(2020, 1).unwrap()).mount(ctx);
///
/// // A month that may be none yet.
/// let expiry = Signal::new(None::<YearMonth>);
/// let picker = MonthPicker::optional(expiry.clone()).mount(ctx);
/// # }
/// ```
///
/// To the keyboard the months are one stop of Tab, at the chosen one, this month or the first that can be chosen:
/// Left and Right go a month, Up and Down four (a row), Home and End to the ends of the row, PageUp and PageDown a
/// year, into the year before or after, which is then shown. Enter and Space choose. To tooling the months are a
/// `grid` called by its year, of `gridcell`s that say whether they are chosen, this month, or out of the range.
#[derive(Clone)]
pub struct MonthPicker<V: FieldValue<YearMonth> = YearMonth> {
  pub(crate) core: Periods<YearMonth, V>,
}

impl MonthPicker<YearMonth> {
  /// A picker bound to `value`, which a choice sets.
  pub fn new(value: Signal<YearMonth>) -> Self {
    Self {
      core: Periods::new(value, false),
    }
  }

  /// A picker that shows `value` and tells `on_change` of a choice, which it does not set.
  pub fn controlled(value: YearMonth, on_change: impl Fn(YearMonth) + Send + Sync + 'static) -> Self {
    Self {
      core: Periods::new(Signal::new(value), true),
    }
    .on_change(on_change)
  }
}

impl MonthPicker<Option<YearMonth>> {
  /// A picker bound to `value`, which holds no month until one is chosen.
  pub fn optional(value: Signal<Option<YearMonth>>) -> Self {
    Self {
      core: Periods::new(value, false),
    }
  }

  /// A picker that shows `value`, which may be none, and tells `on_change` of a choice, which it does not set.
  pub fn controlled_optional(
    value: Option<YearMonth>,
    on_change: impl Fn(Option<YearMonth>) + Send + Sync + 'static,
  ) -> Self {
    Self {
      core: Periods::new(Signal::new(value), true),
    }
    .on_change(on_change)
  }
}

impl<V: FieldValue<YearMonth>> MonthPicker<V> {
  period_methods!(YearMonth);

  /// The month marked as this month, in place of the system's.
  pub fn today(mut self, month: YearMonth) -> Self {
    self.core.current = Some(month);
    self
  }

  /// Builds the picker in `theme`, with the year it shows kept where it has a memory. Converting a picker into an
  /// element does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let view = self.core.view::<MonthPicker>(theme, YearMonth::current());
    match theme.renderer::<MonthPicker>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }

  /// The picker mounted where it is declared: it keeps the year it shows between renders.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let key = self.core.key.clone();
    // Keyed, it keeps its host where what is before it comes and goes.
    let props = PeriodProps { picker: self };
    match key {
      Some(key) => ctx.mount_keyed::<PeriodHost<MonthPicker<V>>>(&key, props).key(key),
      None => ctx.mount::<PeriodHost<MonthPicker<V>>>(props),
    }
  }
}

impl<V: FieldValue<YearMonth>> From<MonthPicker<V>> for Element {
  fn from(picker: MonthPicker<V>) -> Self {
    picker.build(&current_theme())
  }
}

impl<V: FieldValue<YearMonth>> FieldControl for MonthPicker<V> {
  /// Called by the field's label unless it has a name of its own, invalid where the field is, and disabled where it
  /// is.
  fn in_field(mut self, field: &InField) -> Element {
    self.core.in_field(field);
    self.into()
  }

  /// Told what the field says, as in `in_field`, and mounted: the same control as outside a field.
  fn mount_in_field(mut self, field: &InField, ctx: &mut Ctx) -> Element {
    self.core.in_field(field);
    self.mount(ctx)
  }
}

/// What builds a month picker's nodes: a theme's in place of the default one.
pub type MonthPickerRenderer = Arc<dyn Fn(PeriodsView) -> Element + Send + Sync>;

/// A month picker as a renderer sees it.
pub type MonthPickerView = PeriodsView;

impl<V: FieldValue<YearMonth>> crate::ui::Styled for MonthPicker<V> {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl<V: FieldValue<YearMonth>> Themed for MonthPicker<V> {
  const NAME: &'static str = "month_picker";
  type Renderer = MonthPickerRenderer;

  fn parts() -> crate::ui::Declared {
    MonthPickerPart::declared()
  }

  fn states() -> crate::ui::Declared {
    MonthPickerState::declared()
  }

  /// The design's month picker (`pencil/lurq-design-system.pen`, Month Picker): a panel 252 wide in the popover's
  /// colors with an edge of 1 in the border's color, the large radius, the popover's shadow and 12 of padding. The
  /// heading, 28 high, has the year in the body's type at medium weight between two bare chevrons of 16 in the
  /// foreground, 8 over the grid. The months are in three rows of four, 8 apart, cells 34 high and 4 apart with the
  /// medium radius, their short names at 13; the chosen one on the primary fill, its name in the primary's foreground
  /// at medium weight. This month, which the board does not draw, is on the highlight as a calendar's today is; one
  /// out of the range is muted at half opacity, as a calendar's day is.
  fn recipe() -> Recipe {
    periods_recipe()
  }
}

/// A picker of the grid, to mount.
pub(crate) trait PeriodMountable: Clone + Into<Element> + Send + Sync + 'static {
  /// Its memory, where it has none.
  fn keep(&mut self, memory: &DateMemory);
}

impl<V: FieldValue<YearMonth>> PeriodMountable for MonthPicker<V> {
  fn keep(&mut self, memory: &DateMemory) {
    if self.core.memory.is_none() {
      self.core.memory = Some(memory.clone());
    }
  }
}

#[derive(Clone)]
pub(crate) struct PeriodProps<P> {
  pub picker: P,
}

/// The picker is built anew by every render of what holds it, so props are never the same as before.
impl<P> PartialEq for PeriodProps<P> {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

impl<P> crate::app::component::DevtoolsInspectable for PeriodProps<P> {}

/// What a mounted picker keeps between renders.
pub(crate) struct PeriodHost<P> {
  memory: DateMemory,
  picker: PhantomData<fn() -> P>,
}

impl<P: PeriodMountable> Component for PeriodHost<P> {
  type Props = PeriodProps<P>;

  fn create(_ctx: &mut Ctx) -> Self {
    Self {
      memory: DateMemory::new(),
      picker: PhantomData,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let mut picker = ctx.props::<PeriodProps<P>>().picker.clone();
    picker.keep(&self.memory);
    picker.into()
  }
}
