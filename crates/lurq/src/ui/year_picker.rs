//! A year picker: twelve years in a grid, a year of which a person chooses, and the years before and after under the
//! buttons of its heading.

use std::sync::Arc;

use super::{
  calendar::DateMemory,
  date_field::FieldValue,
  field::{FieldControl, InField},
  month_picker::{PeriodHost, PeriodMountable, PeriodProps, Periods, PeriodsView, period_methods, periods_recipe},
  recipe::Recipe,
  theme::{Themed, current_theme},
};
use crate::{
  app::{ctx::Ctx, theme::Theme},
  core::{Date, Signal},
  node::Element,
};

/// A year picker: twelve years from the start of a decade (2020 to 2031), of which a person chooses one, and the
/// decades before and after under the buttons of its heading. The year of a report, a year of birth, the year a
/// company was founded.
///
/// ```
/// use lurq::{core::Signal, ui::YearPicker};
///
/// # fn render(ctx: &mut lurq::app::ctx::Ctx) {
/// let year = Signal::new(2026);
/// // Mounted, so that it keeps the years it shows.
/// let picker = YearPicker::new(year.clone()).min(1900).max(2026).mount(ctx);
///
/// // A year that may be none yet.
/// let founded = Signal::new(None::<i32>);
/// let picker = YearPicker::optional(founded.clone()).mount(ctx);
/// # }
/// ```
///
/// It is a [`MonthPicker`](super::MonthPicker) of years: the same panel, heading and grid, the same keys, with PageUp
/// and PageDown going ten years. Its parts and states are the month picker's (`MonthPickerPart`, `MonthPickerState`).
#[derive(Clone)]
pub struct YearPicker<V: FieldValue<i32> = i32> {
  pub(crate) core: Periods<i32, V>,
}

impl YearPicker<i32> {
  /// A picker bound to `value`, which a choice sets.
  pub fn new(value: Signal<i32>) -> Self {
    Self {
      core: Periods::new(value, false),
    }
  }

  /// A picker that shows `value` and tells `on_change` of a choice, which it does not set.
  pub fn controlled(value: i32, on_change: impl Fn(i32) + Send + Sync + 'static) -> Self {
    Self {
      core: Periods::new(Signal::new(value), true),
    }
    .on_change(on_change)
  }
}

impl YearPicker<Option<i32>> {
  /// A picker bound to `value`, which holds no year until one is chosen.
  pub fn optional(value: Signal<Option<i32>>) -> Self {
    Self {
      core: Periods::new(value, false),
    }
  }

  /// A picker that shows `value`, which may be none, and tells `on_change` of a choice, which it does not set.
  pub fn controlled_optional(value: Option<i32>, on_change: impl Fn(Option<i32>) + Send + Sync + 'static) -> Self {
    Self {
      core: Periods::new(Signal::new(value), true),
    }
    .on_change(on_change)
  }
}

impl<V: FieldValue<i32>> YearPicker<V> {
  period_methods!(i32);

  /// The year marked as this year, in place of the system's.
  pub fn today(mut self, year: i32) -> Self {
    self.core.current = Some(year);
    self
  }

  /// Builds the picker in `theme`, with the years it shows kept where it has a memory. Converting a picker into an
  /// element does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let view = self.core.view::<YearPicker>(theme, Date::today().year());
    match theme.renderer::<YearPicker>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }

  /// The picker mounted where it is declared: it keeps the years it shows between renders.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let key = self.core.key.clone();
    let mut element = ctx.mount::<PeriodHost<YearPicker<V>>>(PeriodProps { picker: self });
    if let Some(key) = key {
      element = element.key(key);
    }
    element
  }
}

impl<V: FieldValue<i32>> From<YearPicker<V>> for Element {
  fn from(picker: YearPicker<V>) -> Self {
    picker.build(&current_theme())
  }
}

impl<V: FieldValue<i32>> FieldControl for YearPicker<V> {
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

impl<V: FieldValue<i32>> PeriodMountable for YearPicker<V> {
  fn keep(&mut self, memory: &DateMemory) {
    if self.core.memory.is_none() {
      self.core.memory = Some(memory.clone());
    }
  }
}

/// What builds a year picker's nodes: a theme's in place of the default one.
pub type YearPickerRenderer = Arc<dyn Fn(PeriodsView) -> Element + Send + Sync>;

/// A year picker as a renderer sees it.
pub type YearPickerView = PeriodsView;

impl<V: FieldValue<i32>> crate::ui::Styled for YearPicker<V> {
  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl<V: FieldValue<i32>> Themed for YearPicker<V> {
  const NAME: &'static str = "year_picker";
  type Renderer = YearPickerRenderer;

  /// The design's year picker (`pencil/lurq-design-system.pen`, Year Picker): the month picker's panel, heading and
  /// grid, with twelve years from the start of a decade, `2020 – 2031` in the heading.
  fn recipe() -> Recipe {
    periods_recipe()
  }
}
