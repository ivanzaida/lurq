//! A date range picker: a field that holds a range of dates, typed or chosen from a calendar that opens under it,
//! with ranges an app offers beside it.

use std::sync::Arc;

use super::{
  date_picker::{Picker, PickerRenderer, PickerView, mount_picker, picker_methods, picker_recipe},
  field::{FieldControl, InField},
  part_style::PartStyle,
  recipe::{Interaction, Name, Recipe},
  theme::{Themed, current_theme},
};
use crate::{
  app::{ctx::Ctx, theme::Theme},
  core::{Date, DateRange, Signal, Weekday, YearMonth},
  node::Element,
};

/// What builds a date range picker's nodes: a theme's in place of the default one.
pub type DateRangePickerRenderer = PickerRenderer<DateRange>;

/// A date range picker as a renderer sees it.
pub type DateRangePickerView = PickerView<DateRange>;

/// A date range picker: a field that holds a range of dates, typed in it or chosen from a calendar that opens under
/// it, with ranges the app offers beside the calendar. The dates of a stay, the period of a report.
///
/// ```
/// use lurq::{
///   core::{Date, DateRange, Signal},
///   ui::DateRangePicker,
/// };
///
/// # fn render(ctx: &mut lurq::app::ctx::Ctx) {
/// let today = Date::today();
/// let period = Signal::new(None::<DateRange>);
/// let picker = DateRangePicker::new(period.clone())
///   .placeholder("Pick dates")
///   .months(2)
///   .preset("Last 7 days", DateRange::new(today.add_days(-6), today))
///   .preset("Last 30 days", DateRange::new(today.add_days(-29), today))
///   .mount(ctx);
/// # }
/// ```
///
/// It is a [`DatePicker`](super::DatePicker) over two dates, in one field as the design draws it: `Nov 9 – Nov 15,
/// 2026`, typed so or as two dates of any form the locale reads with a dash or `to` between them. In the calendar the
/// first press starts the range, the pointer and the keys show where it would end, and the second press ends it and
/// closes the calendar. A preset chooses its range at once and closes it too. Escape or a press outside drops a range
/// half chosen.
#[derive(Clone)]
pub struct DateRangePicker {
  pub(crate) core: Picker<DateRange>,
}

impl DateRangePicker {
  /// A picker bound to `value`: it shows the signal's range, and what is typed or chosen is set on it.
  pub fn new(value: Signal<Option<DateRange>>) -> Self {
    Self {
      core: Picker::new(value),
    }
  }

  /// A picker that shows `value` and tells `on_change` of what is typed, chosen or emptied, which it does not set:
  /// the app answers with the value it renders it with next, and the field shows what the app holds until it does.
  pub fn controlled(value: Option<DateRange>, on_change: impl Fn(Option<DateRange>) + Send + Sync + 'static) -> Self {
    let mut picker = Self::new(Signal::new(value)).on_change(on_change);
    picker.core.controlled = true;
    picker
  }

  picker_methods!(DateRange);

  /// How many months the calendar shows side by side: one as the design draws it, or two.
  pub fn months(mut self, months: usize) -> Self {
    self.core.calendar = self.core.calendar.months(months);
    self
  }

  /// A range offered beside the calendar under `label`, after those offered already: "Last 7 days", "This month".
  pub fn preset(mut self, label: &str, range: DateRange) -> Self {
    self.core.presets.push((Arc::from(label), range));
    self
  }

  /// Ranges offered beside the calendar, each under its label, after those offered already.
  pub fn presets<'a>(mut self, presets: impl IntoIterator<Item = (&'a str, DateRange)>) -> Self {
    for (label, range) in presets {
      self = self.preset(label, range);
    }
    self
  }

  /// Builds the picker in `theme`, with nothing kept but what the app gives it. Converting a picker into an element
  /// does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    self.core.build(theme)
  }

  /// The picker mounted where it is declared: it keeps whether its calendar is open, the field it opens under, the
  /// month the calendar shows and the first press of a range, and writes dates as the app says
  /// (`App::set_date_locale`).
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    mount_picker(ctx, self.core)
  }
}

impl From<DateRangePicker> for Element {
  fn from(picker: DateRangePicker) -> Self {
    picker.build(&current_theme())
  }
}

impl FieldControl for DateRangePicker {
  /// Called by the field's label unless it has a name of its own, judged as the field is unless the app judged the
  /// picker itself, and disabled where the field is.
  fn in_field(mut self, field: &InField) -> Element {
    self.core.boxed.in_field(field);
    self.into()
  }

  /// Told what the field says, as in `in_field`, and mounted: the same control as outside a field.
  fn mount_in_field(mut self, field: &InField, ctx: &mut Ctx) -> Element {
    self.core.boxed.in_field(field);
    self.mount(ctx)
  }

  fn has_fluid_form(&self) -> bool {
    true
  }
}

impl crate::ui::Styled for DateRangePicker {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for DateRangePicker {
  const NAME: &'static str = "date_range_picker";
  type Renderer = DateRangePickerRenderer;

  /// The design's date range picker (`pencil/lurq-design-system.pen`, Date Range Picker): the date picker's field,
  /// 260 wide, with the glyph of a calendar's range and the range in one line, and under it the calendar, whose ends
  /// of the range are chosen days and whose days between them are one band on the highlight. Its presets, which the
  /// board does not draw, are a column of small ghost buttons beside the calendar, a line between them.
  fn recipe() -> Recipe {
    picker_recipe()
  }
}
