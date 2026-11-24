//! A time field: a time typed in by its segments (the hours, the minutes, the seconds where it has them and the half
//! of the day on a clock of twelve hours), each stepped by the arrows.

use std::sync::Arc;

use super::{
  calendar::DateMemory,
  date_field::{
    FieldHost, FieldProps, FieldValue, Mountable, SegmentFieldView, Segments, segment_methods, segments_recipe,
  },
  field::{FieldControl, InField},
  icons,
  recipe::Recipe,
  theme::{Themed, current_theme},
};
use crate::{
  app::{ctx::Ctx, theme::Theme},
  core::{HourCycle, Signal, Time},
  node::Element,
};

/// A time field: a time typed in by its segments, as a date field is a date. The time of a meeting, an alarm, the
/// opening hours of a shop.
///
/// ```
/// use lurq::{
///   core::{Signal, Time},
///   ui::TimeField,
/// };
///
/// # fn render(ctx: &mut lurq::app::ctx::Ctx) {
/// let starts = Signal::new(Time::new(9, 30, 0).unwrap());
/// // Mounted, so that it keeps what is half typed; the minutes step by a quarter of an hour.
/// let field = TimeField::new(starts.clone()).label("Starts at").step(15).mount(ctx);
///
/// // With seconds, and a time that may be none.
/// let lap = Signal::new(None::<Time>);
/// let timed = TimeField::optional(lap.clone()).seconds(true).mount(ctx);
/// # }
/// ```
///
/// Its segments are a date field's: each a spin button and a stop of Tab, Left and Right between them, digits that
/// fill one and go on, Up and Down round the clock, Backspace that empties. On a clock of twelve hours (the locale's,
/// or [`TimeField::hour_cycle`]) the last segment is the half of the day, which `A` and `P` and the arrows set. The
/// value is a time only when every segment is filled with one in the range ([`TimeField::min`], [`TimeField::max`]).
#[derive(Clone)]
pub struct TimeField<V: FieldValue<Time> = Time> {
  pub(crate) core: Segments<Time, V>,
}

impl TimeField<Time> {
  /// A field bound to `value`: it shows the signal's time, and a time typed whole is set on it.
  pub fn new(value: Signal<Time>) -> Self {
    Self {
      core: Segments::new(value, false),
    }
  }

  /// A field that shows `value` and tells `on_change` of a time typed whole, which it does not set: the app answers
  /// with the time it renders it with next.
  pub fn controlled(value: Time, on_change: impl Fn(Time) + Send + Sync + 'static) -> Self {
    Self {
      core: Segments::new(Signal::new(value), true),
    }
    .on_change(on_change)
  }
}

impl TimeField<Option<Time>> {
  /// A field bound to `value`, which may hold no time: it is none while a segment is empty.
  pub fn optional(value: Signal<Option<Time>>) -> Self {
    Self {
      core: Segments::new(value, false),
    }
  }

  /// A field that shows `value`, which may be none, and tells `on_change` of what is typed, which it does not set.
  pub fn controlled_optional(value: Option<Time>, on_change: impl Fn(Option<Time>) + Send + Sync + 'static) -> Self {
    Self {
      core: Segments::new(Signal::new(value), true),
    }
    .on_change(on_change)
  }
}

impl<V: FieldValue<Time>> TimeField<V> {
  segment_methods!(Time);

  /// The minutes Up and Down step the minutes by: 1 by default, 15 for a quarter of an hour. What is typed may be
  /// any minute.
  pub fn step(mut self, minutes: impl Into<f64>) -> Self {
    self.core.step = minutes.into().round().clamp(1.0, 60.0) as u32;
    self
  }

  /// Whether the field has a segment for the seconds.
  pub fn seconds(mut self, seconds: bool) -> Self {
    self.core.seconds = seconds;
    self
  }

  /// How the hours are counted, in place of the locale's: from 0 to 23, or from 1 to 12 with a segment for the half
  /// of the day.
  pub fn hour_cycle(mut self, cycle: HourCycle) -> Self {
    self.core.hour_cycle = Some(cycle);
    self
  }

  /// Builds the field in `theme`, with what it keeps where it has a memory. Converting a field into an element does
  /// this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let view = self.core.view::<TimeField>(theme, icons::TIMER);
    match theme.renderer::<TimeField>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }

  /// The field mounted where it is declared: it keeps what is half typed in its segments between renders.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let key = self.core.key.clone();
    // Keyed, it keeps its host where what is before it comes and goes.
    let props = FieldProps { field: self };
    match key {
      Some(key) => ctx.mount_keyed::<FieldHost<TimeField<V>>>(&key, props).key(key),
      None => ctx.mount::<FieldHost<TimeField<V>>>(props),
    }
  }
}

impl<V: FieldValue<Time>> From<TimeField<V>> for Element {
  fn from(field: TimeField<V>) -> Self {
    field.build(&current_theme())
  }
}

impl<V: FieldValue<Time>> FieldControl for TimeField<V> {
  /// Called by the field's label unless it has a name of its own, judged as the field is unless the app judged it
  /// itself, and disabled where the field is.
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

impl<V: FieldValue<Time>> Mountable for TimeField<V> {
  fn keep(&mut self, memory: &DateMemory) {
    if self.core.memory.is_none() {
      self.core.memory = Some(memory.clone());
    }
  }
}

/// What builds a time field's nodes: a theme's in place of the default one.
pub type TimeFieldRenderer = Arc<dyn Fn(SegmentFieldView) -> Element + Send + Sync>;

/// A time field as a renderer sees it.
pub type TimeFieldView = SegmentFieldView;

impl<V: FieldValue<Time>> crate::ui::Styled for TimeField<V> {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl<V: FieldValue<Time>> Themed for TimeField<V> {
  const NAME: &'static str = "time_field";
  type Renderer = TimeFieldRenderer;

  /// The design's time field (`pencil/lurq-design-system.pen`, Time Field): a date field's box and segments, 120 by
  /// 36, the hours and the minutes with a colon between them, and a timer's glyph at the end.
  fn recipe() -> Recipe {
    segments_recipe(120.0)
  }
}
