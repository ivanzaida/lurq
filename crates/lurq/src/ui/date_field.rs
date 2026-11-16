//! A date field: a date typed in by its segments (the day, the month and the year), each stepped by the arrows. What
//! it shares with a time field, the segments and their keys, is here too.

use std::{marker::PhantomData, sync::Arc};

use super::{
  calendar::DateMemory,
  field::{FieldControl, InField},
  icon::Icon,
  icons, names,
  part_style::Length,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::{IconChoice, fading, shown_icon},
  text_box::{self, BoxStates, Judgment, TextBoxSize, TextBoxState},
  theme::{Themed, current_date_locale, current_theme},
};
use crate::{
  app::{
    component::{Component, DevtoolsInspectable},
    ctx::Ctx,
    events::KeyboardEvent,
    theme::{ControlSize, IconSize, OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::Row,
  core::{Date, DateOrder, DateSegment, DateText, DayPeriod, HourCycle, Signal, Time, YearMonth},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{CursorIcon, Element, FocusGroup, RadiusValue},
};

/// What a field or a picker holds of a value of kind `T`: the value itself, which is always there once there is one
/// (`new`, `controlled`), or the value or none (`optional`, `controlled_optional`).
pub trait FieldValue<T>: Copy + PartialEq + Send + Sync + std::fmt::Debug + DevtoolsInspectable + 'static {
  /// The value it holds, where it holds one.
  fn get(self) -> Option<T>;

  /// What it is of `value`: `None` where it cannot hold it, a field that always holds a value given none.
  fn of(value: Option<T>) -> Option<Self>;
}

macro_rules! field_values {
  ($($kind:ty),* $(,)?) => {
    $(
      impl FieldValue<$kind> for $kind {
        fn get(self) -> Option<$kind> {
          Some(self)
        }

        fn of(value: Option<$kind>) -> Option<Self> {
          value
        }
      }

      impl FieldValue<$kind> for Option<$kind> {
        fn get(self) -> Option<$kind> {
          self
        }

        fn of(value: Option<$kind>) -> Option<Self> {
          Some(value)
        }
      }
    )*
  };
}

field_values!(Date, Time, YearMonth, i32);

/// The parts of a date field and of a time field a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DateFieldPart {
  /// The box: its edge, its fill, its height, its padding, and the room between its segments and its glyph.
  Root,
  /// The row of the segments: the room between them and what stands between them.
  Segments,
  /// A segment: the day, the month, the year, the hours, the minutes, the seconds, the half of the day. It is what
  /// takes the focus, and is filled while it has it.
  Segment,
  /// What stands between two segments: `/`, `.`, `:`.
  Separator,
  /// The glyph at the end of the box.
  Icon,
}

names!(DateFieldPart {
  Root => "root",
  Segments => "segments",
  Segment => "segment",
  Separator => "separator",
  Icon => "icon",
});

/// The states of a date field and of a time field a recipe styles. The field is in its judgment, `ReadOnly`,
/// `Disabled` and `FullWidth`; a segment in `Focused` and `Placeholder`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DateFieldState {
  /// What it holds is wrong.
  Invalid,
  /// What it holds is allowed with a caveat.
  Warning,
  /// What it holds was checked and is right.
  Valid,
  /// It can be read and not changed.
  ReadOnly,
  /// It takes no input.
  Disabled,
  /// It is as wide as what holds it.
  FullWidth,
  /// The segment has the focus: what is typed goes in it, and the arrows step it.
  Focused,
  /// The segment is empty, and shows what it is for: `dd`, `hh`.
  Placeholder,
  /// It is in a fluid field ([`Field::fluid`](super::Field::fluid)): the field's label is inside its box, over
  /// its segments.
  Fluid,
}

names!(DateFieldState {
  Invalid => "invalid",
  Warning => "warning",
  Valid => "valid",
  ReadOnly => "read_only",
  Disabled => "disabled",
  FullWidth => "full_width",
  Focused => "focused",
  Placeholder => "placeholder",
  Fluid => "fluid",
});

/// The sizes of a date field and of a time field: an input's, the heights of the theme's controls.
pub type DateFieldSize = TextBoxSize;

/// What a field of segments has typed while it is not a whole value: a number for each segment, or none.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Typed(pub(crate) Vec<Option<u32>>);

impl DevtoolsInspectable for Typed {
  fn inspect(&self, formatter: &mut crate::app::component::DevtoolsFormatter<'_>) {
    formatter.value_debug(&self.0);
  }
}

/// A value typed in by segments: a date, or a time.
pub(crate) trait Segmented: Copy + Ord + Send + Sync + std::fmt::Debug + 'static {
  /// The segments, in the order the locale writes them.
  fn layout(text: &dyn DateText, cycle: HourCycle, seconds: bool) -> Vec<DateSegment>;

  /// The number of `segment` in it.
  fn part(self, segment: DateSegment, cycle: HourCycle) -> u32;

  /// The value the segments make, where they make one.
  /// What the field has no segment for is `base`'s, the value it holds: the seconds of a time shown without them.
  fn join(parts: &[(DateSegment, u32)], cycle: HourCycle, base: Option<Self>) -> Option<Self>;

  /// What stands between two segments.
  fn separator(text: &dyn DateText, before: DateSegment, after: DateSegment) -> Option<char>;
}

impl Segmented for Date {
  fn layout(text: &dyn DateText, _: HourCycle, _: bool) -> Vec<DateSegment> {
    use DateSegment::{Day, Month, Year};
    match text.date_order() {
      DateOrder::MonthDayYear => vec![Month, Day, Year],
      DateOrder::DayMonthYear => vec![Day, Month, Year],
      DateOrder::YearMonthDay => vec![Year, Month, Day],
    }
  }

  fn part(self, segment: DateSegment, _: HourCycle) -> u32 {
    match segment {
      DateSegment::Day => self.day() as u32,
      DateSegment::Month => self.month() as u32,
      _ => self.year() as u32,
    }
  }

  fn join(parts: &[(DateSegment, u32)], _: HourCycle, _: Option<Self>) -> Option<Self> {
    let of = |wanted: DateSegment| {
      parts
        .iter()
        .find(|(segment, _)| *segment == wanted)
        .map(|(_, value)| *value)
    };
    Date::new(
      of(DateSegment::Year)? as i32,
      of(DateSegment::Month)? as u8,
      of(DateSegment::Day)? as u8,
    )
  }

  fn separator(text: &dyn DateText, _: DateSegment, _: DateSegment) -> Option<char> {
    Some(text.date_separator())
  }
}

impl Segmented for Time {
  fn layout(_: &dyn DateText, cycle: HourCycle, seconds: bool) -> Vec<DateSegment> {
    let mut layout = vec![DateSegment::Hour, DateSegment::Minute];
    if seconds {
      layout.push(DateSegment::Second);
    }
    if cycle == HourCycle::H12 {
      layout.push(DateSegment::Period);
    }
    layout
  }

  fn part(self, segment: DateSegment, cycle: HourCycle) -> u32 {
    match (segment, cycle) {
      (DateSegment::Hour, HourCycle::H23) => self.hour() as u32,
      (DateSegment::Hour, HourCycle::H12) => self.hour12().0 as u32,
      (DateSegment::Minute, _) => self.minute() as u32,
      (DateSegment::Second, _) => self.second() as u32,
      (DateSegment::Period, _) => self.period() as u32,
      _ => 0,
    }
  }

  fn join(parts: &[(DateSegment, u32)], cycle: HourCycle, base: Option<Self>) -> Option<Self> {
    let of = |wanted: DateSegment| {
      parts
        .iter()
        .find(|(segment, _)| *segment == wanted)
        .map(|(_, value)| *value)
    };
    let (hour, minute) = (of(DateSegment::Hour)? as u8, of(DateSegment::Minute)? as u8);
    let second = of(DateSegment::Second).map_or_else(|| base.map_or(0, Time::second), |second| second as u8);
    match cycle {
      HourCycle::H23 => Time::new(hour, minute, second),
      HourCycle::H12 => {
        let period = if of(DateSegment::Period)? == 0 {
          DayPeriod::Am
        } else {
          DayPeriod::Pm
        };
        Time::from_hour12(hour, period, minute, second)
      }
    }
  }

  fn separator(text: &dyn DateText, _: DateSegment, after: DateSegment) -> Option<char> {
    (after != DateSegment::Period).then(|| text.time_separator())
  }
}

/// The least and the greatest number of `segment`, and how many digits it has.
pub(crate) fn range(segment: DateSegment, cycle: HourCycle) -> (u32, u32, usize) {
  match (segment, cycle) {
    (DateSegment::Day, _) => (1, 31, 2),
    (DateSegment::Month, _) => (1, 12, 2),
    (DateSegment::Year, _) => (1, 9999, 4),
    (DateSegment::Hour, HourCycle::H23) => (0, 23, 2),
    (DateSegment::Hour, HourCycle::H12) => (1, 12, 2),
    (DateSegment::Minute | DateSegment::Second, _) => (0, 59, 2),
    (DateSegment::Period, _) => (0, 1, 0),
  }
}

/// The key of a segment: what the keys move the focus to.
fn segment_key(index: usize) -> String {
  format!("segment-{index}")
}

/// What a field of segments keeps from one render to the next: the app's memory, or one made for a build.
#[derive(Clone)]
pub(crate) struct Kept {
  typed: Signal<Option<Typed>>,
  focused: Signal<Option<usize>>,
  digits: Arc<parking_lot::Mutex<Option<(usize, usize, u32)>>>,
  seen: Arc<parking_lot::Mutex<Option<String>>>,
}

impl Kept {
  pub fn of(memory: Option<&DateMemory>) -> Self {
    match memory {
      Some(memory) => Self {
        typed: memory.typed.clone(),
        focused: memory.focused.clone(),
        digits: memory.digits.clone(),
        seen: memory.seen.clone(),
      },
      None => Self {
        typed: Signal::new(None),
        focused: Signal::new(None),
        digits: Default::default(),
        seen: Default::default(),
      },
    }
  }
}

/// What a date field and a time field are built from.
#[derive(Clone)]
pub(crate) struct Segments<T: Segmented, V: FieldValue<T>> {
  pub value: Signal<V>,
  pub controlled: bool,
  pub boxed: BoxStates,
  pub full_width: bool,
  pub min: Option<T>,
  pub max: Option<T>,
  /// The minutes the arrows step a time's minutes by.
  pub step: u32,
  pub seconds: bool,
  pub hour_cycle: Option<HourCycle>,
  pub today: Option<Date>,
  pub locale: Option<Arc<dyn DateText>>,
  pub icon: Option<IconChoice>,
  pub on_change: Vec<Arc<dyn Fn(V) + Send + Sync>>,
  pub memory: Option<DateMemory>,
  // Read only where the engine has forms.
  #[cfg_attr(not(feature = "form"), allow(dead_code))]
  pub name: Option<Arc<str>>,
  pub states: Vec<Name>,
  pub rules: Recipe,
  pub forced: Option<Interaction>,
  pub id: Option<Arc<str>>,
  pub key: Option<Arc<str>>,
  kind: PhantomData<fn() -> T>,
}

impl<T: Segmented, V: FieldValue<T>> Segments<T, V> {
  pub fn new(value: Signal<V>, controlled: bool) -> Self {
    Self {
      value,
      controlled,
      boxed: BoxStates::default(),
      full_width: false,
      min: None,
      max: None,
      step: 1,
      seconds: false,
      hour_cycle: None,
      today: None,
      locale: None,
      icon: None,
      on_change: Vec::new(),
      memory: None,
      name: None,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
      kind: PhantomData,
    }
  }

  /// The view of the field in `theme`, as the themed component `C` is drawn.
  pub fn view<C: Themed>(self, theme: &Theme, glyph: &'static str) -> SegmentFieldView {
    let text = self.locale.clone().unwrap_or_else(current_date_locale);
    let cycle = self.hour_cycle.unwrap_or_else(|| text.hour_cycle());
    let layout = T::layout(&*text, cycle, self.seconds);
    let kept = Kept::of(self.memory.as_ref());
    // Read so that what renders the field renders it again when the app's value changes; a value set from outside
    // since the last build shows in place of what was typed.
    let value = self.value.get();
    let shown_value = format!("{value:?}");
    {
      let mut seen = kept.seen.lock();
      if seen.as_deref() != Some(shown_value.as_str()) {
        if seen.is_some() && kept.typed.get_untracked().is_some() {
          kept.typed.set(None);
        }
        *seen = Some(shown_value);
      }
    }
    let typed = kept.typed.get();
    let focused = kept.focused.get();
    let values: Vec<Option<u32>> = match &typed {
      Some(Typed(values)) if values.len() == layout.len() => values.clone(),
      _ => layout
        .iter()
        .map(|segment| value.get().map(|value| value.part(*segment, cycle)))
        .collect(),
    };
    let digits = *kept.digits.lock();
    let segments = layout
      .iter()
      .enumerate()
      .map(|(index, segment)| {
        let (min, max, width) = range(*segment, cycle);
        let number = values[index];
        let typing = digits.filter(|(at, ..)| *at == index).map(|(_, count, _)| count);
        let shown = match (number, segment) {
          (None, _) => text.segment_placeholder(*segment),
          (Some(number), DateSegment::Period) => {
            text.day_period_name(if number == 0 { DayPeriod::Am } else { DayPeriod::Pm })
          }
          (Some(number), _) => format!("{number:0width$}", width = typing.unwrap_or(width)),
        };
        let value_text = number.map(|number| match segment {
          DateSegment::Period => shown.clone(),
          DateSegment::Month => text.month_name(number as u8),
          _ => number.to_string(),
        });
        SegmentShown {
          segment: *segment,
          text: shown,
          value: number,
          value_text,
          min,
          max,
          label: text.segment_label(*segment),
          separator: (index > 0)
            .then(|| T::separator(&*text, layout[index - 1], *segment))
            .flatten(),
          focused: focused == Some(index),
        }
      })
      .collect();
    let disabled = self.boxed.disabled;
    let case = self
      .boxed
      .case(self.states.clone())
      .state(DateFieldState::FullWidth, self.full_width);
    let recipe = theme.resolve::<C>(&case, Some(&self.rules));
    let resolve: Resolve = {
      let (theme, rules) = (theme.clone(), self.rules.clone());
      Arc::new(move |case: &Case| theme.resolve::<C>(case, Some(&rules)))
    };
    #[cfg(feature = "form")]
    let form_value = value.get().map(|value| format!("{value:?}")).unwrap_or_default();
    let model: Arc<dyn SegmentModel> = Arc::new(Model {
      value: self.value.clone(),
      controlled: self.controlled,
      on_change: self.on_change.clone(),
      kept,
      layout,
      cycle,
      min: self.min,
      max: self.max,
      step: self.step.max(1),
      year: self.today.unwrap_or_else(Date::today).year() as u32,
      read_only: self.boxed.read_only,
      disabled,
    });
    SegmentFieldView {
      segments,
      label: self.boxed.label.clone(),
      invalid: self.boxed.judgment() == Some(Judgment::Invalid),
      read_only: self.boxed.read_only,
      disabled,
      icon: shown_icon(self.icon, glyph),
      recipe,
      case,
      resolve,
      forced: self.forced,
      #[cfg(feature = "form")]
      name: self.name.map(|name| (name, form_value)),
      model,
      id: self.id,
      key: self.key,
    }
  }
}

/// What a segment shows, as a renderer sees it.
#[derive(Clone, Debug)]
pub struct SegmentShown {
  pub segment: DateSegment,
  /// What it shows: its number, or what it is for while it is empty (`dd`).
  pub text: String,
  /// Its number, where it has one: for the half of the day, 0 for AM and 1 for PM.
  pub value: Option<u32>,
  /// What its number is called to what reads the tree: `November` for the month 11.
  pub value_text: Option<String>,
  pub min: u32,
  pub max: u32,
  /// What it is called to what reads the tree: `Day`.
  pub label: String,
  /// What stands before it, where something does.
  pub separator: Option<char>,
  /// Whether it has the focus.
  pub focused: bool,
}

/// What the handlers of a field of segments do.
pub(crate) trait SegmentModel: Send + Sync {
  /// What a key on the segment at `index` does.
  fn key(&self, index: usize, event: &KeyboardEvent);
  /// The segment at `index` took the focus.
  fn focus(&self, index: usize);
  /// The segment at `index` lost it.
  fn blur(&self, index: usize);
}

struct Model<T: Segmented, V: FieldValue<T>> {
  value: Signal<V>,
  controlled: bool,
  on_change: Vec<Arc<dyn Fn(V) + Send + Sync>>,
  kept: Kept,
  layout: Vec<DateSegment>,
  cycle: HourCycle,
  min: Option<T>,
  max: Option<T>,
  step: u32,
  /// The year an empty year starts at under the arrows.
  year: u32,
  read_only: bool,
  disabled: bool,
}

impl<T: Segmented, V: FieldValue<T>> Model<T, V> {
  /// What the segments hold now: what was typed, or the value's.
  fn values(&self) -> Vec<Option<u32>> {
    match self.kept.typed.get_untracked() {
      Some(Typed(values)) if values.len() == self.layout.len() => values,
      _ => {
        let value = self.value.get_untracked().get();
        self
          .layout
          .iter()
          .map(|segment| value.map(|value| value.part(*segment, self.cycle)))
          .collect()
      }
    }
  }

  /// Keeps `values` as typed, and writes nothing yet.
  fn keep(&self, values: Vec<Option<u32>>) {
    if self.kept.typed.get_untracked().as_ref() != Some(&Typed(values.clone())) {
      self.kept.typed.set(Some(Typed(values)));
    }
  }

  /// Keeps `values` as typed and writes the value they make, where they make one in the range; where they make none,
  /// a field that may hold none holds none.
  fn store(&self, values: Vec<Option<u32>>) {
    let whole: Option<Vec<(DateSegment, u32)>> = self
      .layout
      .iter()
      .zip(&values)
      .map(|(segment, value)| value.map(|value| (*segment, value)))
      .collect();
    let made = whole
      .and_then(|parts| T::join(&parts, self.cycle, self.value.get_untracked().get()))
      .filter(|value| self.min.is_none_or(|min| *value >= min) && self.max.is_none_or(|max| *value <= max));
    self.keep(values);
    match made {
      Some(value) => self.write(V::of(Some(value))),
      None => self.write(V::of(None)),
    }
  }

  /// Sets the value where it changed, and tells the handlers; a controlled field only tells them.
  fn write(&self, value: Option<V>) {
    let Some(value) = value else {
      return;
    };
    if self.value.get_untracked() == value {
      return;
    }
    // The field's own change is no change from outside: what is typed stays.
    *self.kept.seen.lock() = Some(format!("{value:?}"));
    if !self.controlled {
      self.value.set(value);
    }
    for handler in &self.on_change {
      handler(value);
    }
  }
}

impl<T: Segmented, V: FieldValue<T>> SegmentModel for Model<T, V> {
  fn key(&self, index: usize, event: &KeyboardEvent) {
    if self.disabled || self.read_only || event.ctrl || event.alt || event.meta || event.composing {
      return;
    }
    let Some(&segment) = self.layout.get(index) else {
      return;
    };
    let (min, max, digits) = range(segment, self.cycle);
    let mut values = self.values();
    let current = values[index];
    let next = |event: &KeyboardEvent| {
      if index + 1 < self.layout.len() {
        event.focus_key(segment_key(index + 1));
      }
    };
    let key = event.key.as_str();
    match key {
      _ if segment != DateSegment::Period && key.len() == 1 && key.as_bytes()[0].is_ascii_digit() => {
        event.prevent_default();
        let digit = (key.as_bytes()[0] - b'0') as u32;
        let mut typing = self.kept.digits.lock();
        let (count, number) = match *typing {
          Some((at, count, number)) if at == index && count < digits => (count + 1, number * 10 + digit),
          _ => (1, digit),
        };
        let (count, number) = if number > max { (1, digit) } else { (count, number) };
        // Done where every digit is typed, or where no digit more would make a number in the range.
        if count >= digits || number * 10 > max {
          *typing = None;
          values[index] = Some(number.clamp(min, max));
          drop(typing);
          self.store(values);
          next(event);
        } else {
          // Half typed: shown, and no value until it is done or the focus leaves it.
          *typing = Some((index, count, number));
          values[index] = Some(number);
          drop(typing);
          self.keep(values);
        }
      }
      "a" | "A" | "p" | "P" if segment == DateSegment::Period => {
        event.prevent_default();
        values[index] = Some(if key.eq_ignore_ascii_case("a") { 0 } else { 1 });
        self.store(values);
        next(event);
      }
      "ArrowUp" | "ArrowDown" => {
        event.prevent_default();
        *self.kept.digits.lock() = None;
        let up = key == "ArrowUp";
        let step = if segment == DateSegment::Minute { self.step } else { 1 };
        values[index] = Some(match current {
          None if segment == DateSegment::Year => self.year,
          None if up => min,
          None => max,
          Some(value) if segment == DateSegment::Year => {
            if up {
              (value + 1).min(max)
            } else {
              value.saturating_sub(1).max(min)
            }
          }
          Some(value) => {
            // Round the dial: the minutes after 59 are 0 again, and a step of 15 goes from 50 to 0.
            let span = max - min + 1;
            let at = value - min;
            if up {
              ((at / step + 1) * step % span) + min
            } else {
              let back = if at % step == 0 {
                at + span - step
              } else {
                at - at % step + span
              };
              back % span + min
            }
          }
        });
        self.store(values);
      }
      "Home" | "End" => {
        event.prevent_default();
        *self.kept.digits.lock() = None;
        values[index] = Some(if key == "Home" { min } else { max });
        self.store(values);
      }
      "Backspace" | "Delete" => {
        event.prevent_default();
        let mut typing = self.kept.digits.lock();
        match *typing {
          Some((at, count, number)) if at == index && count > 1 => {
            *typing = Some((index, count - 1, number / 10));
            values[index] = Some(number / 10);
            drop(typing);
            self.keep(values);
            return;
          }
          _ => {
            *typing = None;
            values[index] = None;
          }
        }
        drop(typing);
        self.store(values);
      }
      _ => {}
    }
  }

  fn focus(&self, index: usize) {
    *self.kept.digits.lock() = None;
    if self.kept.focused.get_untracked() != Some(index) {
      self.kept.focused.set(Some(index));
    }
  }

  fn blur(&self, index: usize) {
    // What is half typed in the segment the focus leaves is what it holds: a month of 1 is January.
    let typing = self.kept.digits.lock().take();
    if let Some((at, _, number)) = typing.filter(|(at, ..)| *at == index) {
      let (min, max, _) = range(self.layout[at], self.cycle);
      let mut values = self.values();
      values[at] = Some(number.clamp(min, max));
      self.store(values);
    }
    if self.kept.focused.get_untracked() == Some(index) {
      self.kept.focused.set(None);
    }
  }
}

/// How a field's recipe is resolved in a case of its own: a segment's.
type Resolve = Arc<dyn Fn(&Case) -> ResolvedRecipe + Send + Sync>;

/// A date field or a time field as a renderer sees it: its segments, the style of its parts, and its behavior ready
/// to be dressed.
pub struct SegmentFieldView {
  /// The segments, in the order the locale writes them.
  pub segments: Vec<SegmentShown>,
  /// What it is called to what reads the tree: its own label, or its field's.
  pub label: Option<Arc<str>>,
  pub invalid: bool,
  pub read_only: bool,
  pub disabled: bool,
  /// The glyph at its end, where it has one.
  pub icon: Option<Name>,
  /// The style of each part of this field.
  pub recipe: ResolvedRecipe,
  case: Case,
  resolve: Resolve,
  forced: Option<Interaction>,
  #[cfg(feature = "form")]
  name: Option<(Arc<str>, String)>,
  model: Arc<dyn SegmentModel>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl SegmentFieldView {
  /// The style of a part, as this field is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The style of the segment at `index`, in its states.
  fn segment_part(&self, index: usize) -> ResolvedPart {
    let shown = &self.segments[index];
    let case = self
      .case
      .clone()
      .state(DateFieldState::Focused, shown.focused)
      .state(DateFieldState::Placeholder, shown.value.is_none());
    (self.resolve)(&case)
      .part(DateFieldPart::Segment)
      .forced(self.forced.filter(|_| index == 0))
  }

  /// The segment at `index`: a spin button to what reads the tree, a stop of Tab, typed in and stepped.
  pub fn segment(&self, index: usize) -> Element {
    let shown = &self.segments[index];
    let part = self.segment_part(index);
    let (keys, focus, blur) = (self.model.clone(), self.model.clone(), self.model.clone());
    let mut segment = Row::new()
      .align_items(Alignment::Center)
      .justify(Justify::Center)
      .part(&part)
      .key(segment_key(index))
      .tab_index(if self.disabled { -1 } else { 0 })
      .disabled(self.disabled)
      .focus_label(shown.label.clone())
      .describe("a11y_role", "spinbutton")
      .describe("a11y_name", shown.label.clone())
      .describe("a11y_valuemin", shown.min.to_string())
      .describe("a11y_valuemax", shown.max.to_string())
      .on_key_down(move |event: KeyboardEvent| {
        if event.focus_within {
          keys.key(index, &event);
        }
      })
      .on_focus(move || focus.focus(index))
      .on_blur(move || blur.blur(index))
      .child(part.text(&shown.text).nowrap());
    match (shown.value, &shown.value_text) {
      (Some(value), Some(text)) => {
        segment = segment
          .describe("a11y_valuenow", value.to_string())
          .describe("a11y_valuetext", text.clone());
      }
      _ => {
        segment = segment.describe("a11y_valuetext", "Empty");
      }
    }
    if self.read_only {
      segment = segment.describe("a11y_readonly", "true");
    }
    if let Some(id) = &self.id {
      segment = segment.id(format!("{id}-{}", segment_name(shown.segment)));
    }
    segment.into()
  }

  /// What stands before the segment at `index`, where something does.
  pub fn separator(&self, index: usize) -> Option<Element> {
    let separator = self.segments[index].separator?;
    let part = self.part(DateFieldPart::Separator);
    Some(
      Row::new()
        .part(&part)
        .describe("a11y_hidden", "true")
        .child(part.text(&separator.to_string()).nowrap())
        .into(),
    )
  }

  /// The segments and what stands between them, one focus group whose Left and Right go from one to the next.
  pub fn segments_row(&self) -> Row {
    let mut row = Row::new()
      .align_items(Alignment::Center)
      .part(&self.part(DateFieldPart::Segments))
      .focus_group(FocusGroup::horizontal().wrap(false));
    for index in 0..self.segments.len() {
      if let Some(separator) = self.separator(index) {
        row = row.child(separator);
      }
      row = row.child(self.segment(index));
    }
    row
  }

  /// The glyph at the end of the box, where it has one. The pointer passes through it.
  pub fn icon(&self) -> Option<Element> {
    let name = self.icon?;
    let part = self.part(DateFieldPart::Icon);
    let side = part.rest().width.unwrap_or(Length::Icon(IconSize::Md));
    let mut icon = Row::new()
      .part(&part)
      .describe("a11y_hidden", "true")
      .child(Icon::new(name).size(side).build(self.theme()));
    if let Some(id) = &self.id {
      icon = icon.id(format!("{id}-icon"));
    }
    Some(icon.into())
  }

  /// How the default renderer builds the field: the box, its segments from its start, the glyph at its end. To what
  /// reads the tree it is a `group` called by its label.
  pub fn render(self) -> Element {
    let mut root = Row::new()
      .align_items(Alignment::Center)
      .part(&self.part(DateFieldPart::Root))
      .justify(Justify::SpaceBetween)
      .describe("a11y_role", "group")
      .child(self.segments_row());
    if let Some(icon) = self.icon() {
      root = root.child(icon);
    }
    if let Some(label) = &self.label {
      root = root.describe("a11y_name", label.clone());
    }
    if self.invalid {
      root = root.describe("a11y_invalid", "true");
    }
    #[cfg(feature = "form")]
    if let Some((name, value)) = &self.name {
      // What a form reads: the value in ISO 8601, in a text input no one sees or reaches.
      root = root.child(
        crate::components::TextInput::new(Signal::new(value.clone()))
          .name(name.clone())
          .focusable(false)
          .width(0.0)
          .height(0.0),
      );
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root.into()
  }
}

/// The name a segment's id ends in: `due-day`.
fn segment_name(segment: DateSegment) -> &'static str {
  match segment {
    DateSegment::Day => "day",
    DateSegment::Month => "month",
    DateSegment::Year => "year",
    DateSegment::Hour => "hour",
    DateSegment::Minute => "minute",
    DateSegment::Second => "second",
    DateSegment::Period => "period",
  }
}

/// The recipe of a date field and of a time field: the box `width` wide at least. In a fluid field, which the board
/// does not draw for them, the box is the fluid one, as wide as the field and 56 high, the segments and the glyph on
/// one line under the field's label; it is padded 9 at its start, so that the first segment's text starts where the
/// label does.
pub(crate) fn segments_recipe(width: f32) -> Recipe {
  use DateFieldPart::{Icon as Glyph, Root, Segment, Segments, Separator};
  use PaletteColor::{Background, Border, Foreground, Input, Muted, MutedForeground, Ring, Selection};
  let edge = |color: PaletteColor| text_box::edge(color);
  let recipe = Recipe::new()
    .base(Root, |s| {
      fading(
        s.min_width(width)
          .height(ControlSize::Md)
          .padding_x(10.0)
          .gap(2.0)
          .radius(RadiusSize::Md)
          .background(Background)
          .border(edge(Input))
          .cursor(CursorIcon::Text),
      )
    })
    .base(Segments, |s| s.gap(2.0))
    .base(Segment, |s| {
      fading(
        s.height(19.0)
          .padding_x(3.0)
          .padding_y(1.0)
          .radius(RadiusValue::Px(3.0))
          .typography(TypographyStyle::Mono)
          .font_size(13.0)
          .weight(FontWeight::Normal)
          .foreground(Foreground),
      )
    })
    .base(Separator, |s| {
      s.typography(TypographyStyle::Mono)
        .font_size(13.0)
        .foreground(MutedForeground)
    })
    .base(Glyph, |s| s.square(IconSize::Md).foreground(MutedForeground))
    .state(DateFieldState::Placeholder, Segment, |s| s.foreground(MutedForeground))
    .state(DateFieldState::Focused, Segment, |s| {
      // The segment with the focus is the selected text of the box, in the role a text input's selection is in.
      s.background(Selection).foreground(Foreground)
    })
    .hovered(Root, |s| s.border(edge(Ring)))
    .state(DateFieldState::FullWidth, Root, |s| s.width(Length::Fill))
    .state(TextBoxState::ReadOnly, Root, |s| {
      s.background(Muted).border(edge(Border))
    })
    .state(TextBoxState::Disabled, Root, |s| {
      s.opacity(OpacityLevel::Disabled)
        .background(Muted)
        .cursor(CursorIcon::Default)
    })
    .size(TextBoxSize::Sm, Root, |s| s.height(ControlSize::Sm))
    .size(TextBoxSize::Lg, Root, |s| s.height(ControlSize::Lg))
    .size(TextBoxSize::Sm, Segment, |s| s.font_size(12.0))
    .size(TextBoxSize::Sm, Separator, |s| s.font_size(12.0))
    .size(TextBoxSize::Lg, Segment, |s| s.font_size(14.0))
    .size(TextBoxSize::Lg, Separator, |s| s.font_size(14.0));
  let recipe = [Judgment::Invalid, Judgment::Warning, Judgment::Valid]
    .into_iter()
    .fold(recipe, |recipe, judgment| {
      let (state, color) = (TextBoxState::from(judgment), judgment.color());
      recipe.state(state, Root, |s| s.border(edge(color.clone()))).when(
        Selector::new().state(state).hovered(),
        Root,
        |s| s.border(edge(color)),
      )
    });
  // The root is the box. A segment is higher than a line of text by its padding, so the box is 56 and not as high
  // as what it holds: the segments stand in the middle of the value's line.
  text_box::fluid_rules(recipe, Root, Root).state(TextBoxState::Fluid, Root, |s| {
    s.height(text_box::FLUID_HEIGHT).padding_start(9.0)
  })
}

/// The builder methods a date field and a time field share.
macro_rules! segment_methods {
  ($kind:ty) => {
    /// What the field is called, to what reads the tree. A [`Field`](crate::ui::Field) around it gives it its label.
    pub fn label(mut self, label: &str) -> Self {
      self.core.boxed.label = Some(std::sync::Arc::from(label));
      self
    }

    /// A [`DateFieldSize`](crate::ui::DateFieldSize) (an input's), or a size the theme's recipe names.
    pub fn size(mut self, size: impl Into<crate::ui::Name>) -> Self {
      self.core.boxed.size = size.into();
      self
    }

    /// Whether what the field holds is wrong. Of invalid, warning and valid, the first that is set is the one shown.
    pub fn invalid(mut self, invalid: bool) -> Self {
      self.core.boxed.invalid = invalid;
      self
    }

    /// Whether what the field holds is allowed with a caveat.
    pub fn warning(mut self, warning: bool) -> Self {
      self.core.boxed.warning = warning;
      self
    }

    /// Whether what the field holds was checked and is right.
    pub fn valid(mut self, valid: bool) -> Self {
      self.core.boxed.valid = valid;
      self
    }

    /// Whether the field can be read and not changed: its segments take the focus, and no key changes them.
    pub fn read_only(mut self, read_only: bool) -> Self {
      self.core.boxed.read_only = read_only;
      self
    }

    /// Whether the field takes no input.
    pub fn disabled(mut self, disabled: bool) -> Self {
      self.core.boxed.disabled = disabled;
      self
    }

    /// Whether the field is as wide as what holds it.
    pub fn full_width(mut self, full_width: bool) -> Self {
      self.core.full_width = full_width;
      self
    }

    /// The least value that is the field's: a value typed before it is no value.
    pub fn min(mut self, min: $kind) -> Self {
      self.core.min = Some(min);
      self
    }

    /// The greatest value that is the field's.
    pub fn max(mut self, max: $kind) -> Self {
      self.core.max = Some(max);
      self
    }

    /// What the segments are called and show while empty, their order and what stands between them, in place of
    /// the app's ([`App::set_date_locale`](crate::app::App::set_date_locale), English where it set none).
    pub fn locale(mut self, locale: impl crate::core::DateText + 'static) -> Self {
      self.core.locale = Some(crate::core::shared_date_text(locale));
      self
    }

    /// The glyph at the end of the field: one of the theme's set, or `None` for none.
    pub fn icon(mut self, icon: impl Into<crate::ui::IconChoice>) -> Self {
      self.core.icon = Some(icon.into());
      self
    }

    /// Called when a person changes the value, with it: once every segment is filled, or, where the field may hold
    /// none, when one is emptied. Bound to a signal, the value is set already. Not called for what it holds already,
    /// nor when the app sets the value. A second handler is called after the first.
    pub fn on_change(mut self, handler: impl Fn(V) + Send + Sync + 'static) -> Self {
      self.core.on_change.push(std::sync::Arc::new(handler));
      self
    }

    /// What keeps what is typed in the segments while it is not a whole value, and which has the focus: the app's,
    /// made once. A field in a [`Field`](crate::ui::Field) keeps them so; a mounted one keeps its own.
    pub fn memory(mut self, memory: &crate::ui::DateMemory) -> Self {
      self.core.memory = Some(memory.clone());
      self
    }

    /// The name the value has in the form it is in, written in ISO 8601.
    #[cfg(feature = "form")]
    pub fn name(mut self, name: impl Into<std::sync::Arc<str>>) -> Self {
      self.core.name = Some(name.into());
      self
    }

    /// Puts the field in a state the theme's recipe names, or takes it out.
    pub fn state(mut self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
      let state = state.into();
      self.core.states.retain(|held| *held != state);
      if on {
        self.core.states.push(state);
      }
      self
    }

    /// Styles one part of this field over what the theme gives it.
    pub fn style(
      mut self,
      part: impl Into<crate::ui::Name>,
      style: impl FnOnce(crate::ui::PartStyle) -> crate::ui::PartStyle,
    ) -> Self {
      self.core.rules = self.core.rules.base(part, style);
      self
    }

    /// Lays rules over the theme's recipe for this field.
    pub fn rules(mut self, rules: &crate::ui::Recipe) -> Self {
      self.core.rules = self.core.rules.extend(rules);
      self
    }

    /// Draws the first segment as it is under the pointer or focused, with nothing pointing at it: for a gallery or a
    /// test.
    pub fn force(mut self, interaction: Option<crate::ui::Interaction>) -> Self {
      self.core.forced = interaction;
      self
    }

    /// The id of the field's box. Each segment has it with `-` and the segment's name after it: `due-day`,
    /// `start-minute`.
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
pub(crate) use segment_methods;

/// A date field: a date typed in by its segments, the day, the month and the year, in the order the locale writes
/// them. A date of birth, a date in a form where a calendar would be in the way.
///
/// ```
/// use lurq::{
///   core::{Date, Signal},
///   ui::{DateField, DateMemory, Field},
/// };
///
/// # fn render(ctx: &mut lurq::app::ctx::Ctx) {
/// // Mounted, so that it keeps what is half typed.
/// let born = Signal::new(Date::new(1990, 4, 12).unwrap());
/// let field = DateField::new(born.clone()).label("Date of birth").mount(ctx);
///
/// // A date that may be none, in a field, with what it keeps kept by the app (made once, in `create`).
/// let due = Signal::new(None::<Date>);
/// let memory = DateMemory::new();
/// let named = Field::new("Due date", DateField::optional(due.clone()).memory(&memory));
/// # }
/// ```
///
/// Each segment is a spin button and a stop of Tab, as a browser's own date input and the date fields of macOS
/// are: Tab and Shift+Tab go from one to the next, and so do Left and Right. Digits fill the focused segment and go on
/// to the next once it is full (`0`, `4` for April, or `4` alone, which no second digit could follow); Up and Down
/// step it, round from the last to the first, and an empty year starts at this year; Home and End go to its least
/// and greatest; Backspace takes the last digit typed, or empties it. The value is a date only when every segment is
/// filled with one that exists and is in the range ([`DateField::min`], [`DateField::max`]): until then a field of
/// `new` keeps the date it had and one of `optional` holds none.
///
/// What is typed while it is no date is kept from one render to the next where the field is mounted
/// ([`DateField::mount`]) or given a [`DateMemory`] the app keeps ([`DateField::memory`]), as a field in a
/// [`Field`](super::Field) is. The segments' names, what an empty one shows (`dd`, `mm`, `yyyy`), their order and
/// what stands between them are the locale's ([`DateField::locale`]).
#[derive(Clone)]
pub struct DateField<V: FieldValue<Date> = Date> {
  pub(crate) core: Segments<Date, V>,
}

impl DateField<Date> {
  /// A field bound to `value`: it shows the signal's date, and a date typed whole is set on it.
  pub fn new(value: Signal<Date>) -> Self {
    Self {
      core: Segments::new(value, false),
    }
  }

  /// A field that shows `value` and tells `on_change` of a date typed whole, which it does not set: the app answers
  /// with the date it renders it with next.
  pub fn controlled(value: Date, on_change: impl Fn(Date) + Send + Sync + 'static) -> Self {
    Self {
      core: Segments::new(Signal::new(value), true),
    }
    .on_change(on_change)
  }
}

impl DateField<Option<Date>> {
  /// A field bound to `value`, which may hold no date: it is none while a segment is empty.
  pub fn optional(value: Signal<Option<Date>>) -> Self {
    Self {
      core: Segments::new(value, false),
    }
  }

  /// A field that shows `value`, which may be none, and tells `on_change` of what is typed, which it does not set.
  pub fn controlled_optional(value: Option<Date>, on_change: impl Fn(Option<Date>) + Send + Sync + 'static) -> Self {
    Self {
      core: Segments::new(Signal::new(value), true),
    }
    .on_change(on_change)
  }
}

impl<V: FieldValue<Date>> DateField<V> {
  segment_methods!(Date);

  /// The day an empty year starts at under the arrows: today, in place of the system's.
  pub fn today(mut self, today: Date) -> Self {
    self.core.today = Some(today);
    self
  }

  /// Builds the field in `theme`, with what it keeps where it has a memory. Converting a field into an element does
  /// this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let view = self.core.view::<DateField>(theme, icons::CALENDAR);
    match theme.renderer::<DateField>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }

  /// The field mounted where it is declared: it keeps what is half typed in its segments between renders.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let key = self.core.key.clone();
    let mut element = ctx.mount::<FieldHost<DateField<V>>>(FieldProps { field: self });
    if let Some(key) = key {
      element = element.key(key);
    }
    element
  }
}

impl<V: FieldValue<Date>> From<DateField<V>> for Element {
  fn from(field: DateField<V>) -> Self {
    field.build(&current_theme())
  }
}

impl<V: FieldValue<Date>> FieldControl for DateField<V> {
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

/// What builds a date field's nodes: a theme's in place of the default one.
pub type DateFieldRenderer = Arc<dyn Fn(SegmentFieldView) -> Element + Send + Sync>;

/// A date field as a renderer sees it.
pub type DateFieldView = SegmentFieldView;

impl<V: FieldValue<Date>> Themed for DateField<V> {
  const NAME: &'static str = "date_field";
  type Renderer = DateFieldRenderer;

  /// The design's date field (`pencil/lurq-design-system.pen`, Date Field): a box 180 by 36 on the background, an edge
  /// of 1 in the color of an input's and the medium radius, 10 of padding. Its segments, 2 apart, are in the mono type
  /// at 13 in the foreground with 3 of padding across and 1 down and a radius of 3; the separators between them in the
  /// muted foreground. The segment with the focus is on the selection's color, as a text input's selected text is (the
  /// board fills it with `info` under white, for which the palette has no pair of roles). A calendar's glyph of 16 in
  /// the muted foreground is at the end.
  fn recipe() -> Recipe {
    segments_recipe(180.0)
  }
}

/// A field of segments, to mount.
pub(crate) trait Mountable: Clone + Into<Element> + Send + Sync + 'static {
  /// Its memory, where it has none.
  fn keep(&mut self, memory: &DateMemory);
}

impl<V: FieldValue<Date>> Mountable for DateField<V> {
  fn keep(&mut self, memory: &DateMemory) {
    if self.core.memory.is_none() {
      self.core.memory = Some(memory.clone());
    }
  }
}

#[derive(Clone)]
pub(crate) struct FieldProps<F> {
  pub field: F,
}

/// The field is built anew by every render of what holds it, so props are never the same as before.
impl<F> PartialEq for FieldProps<F> {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

#[cfg(feature = "devtools")]
impl<F> DevtoolsInspectable for FieldProps<F> {}

/// What a mounted field keeps between renders.
pub(crate) struct FieldHost<F> {
  memory: DateMemory,
  field: PhantomData<fn() -> F>,
}

impl<F: Mountable> Component for FieldHost<F> {
  type Props = FieldProps<F>;

  fn create(_ctx: &mut Ctx) -> Self {
    Self {
      memory: DateMemory::new(),
      field: PhantomData,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let mut field = ctx.props::<FieldProps<F>>().field.clone();
    field.keep(&self.memory);
    field.into()
  }
}
