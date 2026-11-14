# Calendar, Date Picker, Date Range Picker

`ui::Calendar`, `ui::DatePicker` and `ui::DateRangePicker`, from the board's Calendar (`AXv2g`), Date Picker (`bb77i`)
and Date Range Picker (`DLu7f`) in `Ovt1D` (Components / light), as they are used in the Composite inputs' open date
picker (`BCo7B`, `pSVTD`), the Large overlays (`hQHGa`), the date-time picker (`Xohje`) and the Panels (`dS1EP`).
Branch `claude/ds-calendar-940d43`. This session owned dates (a date type, the days of a month, weeks, locales' first
day and names) and the keys of a grid of days.

## Changed outside the component's own files

- **A date type of lurq's own, in `crates/lurq/src/core`** (new files `core/date.rs` and `core/date_locale.rs`,
  `pub mod date; mod date_locale;` and their `pub use` in `core/mod.rs`): `Date` (a year, a month and a day, years 1 to
  9999; `new`, `today`, `from_unix_days` / `unix_days`, `weekday`, `ordinal`, `add_days`, `add_months` (the last day of
  a shorter month), `add_years`, `days_since`, `start_of_week` / `end_of_week`, `iso_week`, `clamp_to`, `Display` and
  `FromStr` as ISO 8601), `YearMonth` (`new`, `of`, `current`, `days`, `first`, `last`, `contains`, `add_months`,
  `months_since`, `weeks(first_day)`: six weeks of seven days), `DateRange` (`new` in order, `day`, `start`, `end`,
  `days`, `contains`), `Weekday`, `is_leap_year`, `days_in_month`, `ParseDateError`, and `DateLocale` with
  `DateOrder`: the first day of the week, the names of months and days, the short form a field shows and what it
  reads back (`for_tag`, `english`, `with_first_day`, `week`, `month_name`, `month_title`, `format`, `format_range`,
  `day_label`, `parse`, `parse_range`). Each implements `DevtoolsInspectable`, so it goes in a signal with the
  `devtools` feature. Their unit tests are in the two files.
  - `Date::today()` reads the local date: on Windows by `GetLocalTime` of `kernel32` (one `extern "system"` of eight
    `u16`s), on Unix but wasm by `time` and `localtime_r` of the C library; elsewhere (the browser build) it is today in
    UTC. Each is behind its `cfg` in `core/date.rs`, as the memory "Platform differences by cfg" has it.
- **Engine: a key's handler moving the focus to an element of its group by its key.**
  `KeyboardEvent::focus_key(key)` (`app/events/keyboard_event.rs`): once the key's handlers have run and what they
  changed is rendered, the focus goes to the stop of the focus group around the focused element whose key
  (`.key(..)`) is `key`, from the keyboard (it shows its ring) and into view; the key's default is prevented. For a
  group whose elements are reached by what the app knows of them and not by their order: a day a week on, in the month
  the key shows next. The keys of a calendar's grid are built on it: `FocusGroup` moves by the order of the stops, which
  is not a calendar's (Up is seven days back, PageDown a month, and either may be a day that is not built yet).
  - `app/events/event_control.rs`: one field of the shared state, `focus_key: Mutex<Option<Arc<str>>>`, and
    `request_focus_key` / `take_focus_key` (crate-private).
  - `app/runtime/keyed_focus.rs` (new): `Tree::focus_group_around_focus` (the innermost group that moves the focus,
    `FocusGroup::none` ending it) and `Tree::focus_keyed_in_group(group, key)` (the group's stops by
    `tab_navigation::group_stop_nodes`, the first whose `component_key` is the key, focused with the keyboard's
    modality and scrolled into view). `app/runtime.rs`: `mod keyed_focus;`, and in `dispatch_key_down` five lines after
    the handlers run that take the request with the group's node id, and three after `apply_reactive_updates_after_event`
    that carry it out. The group is found again by its node id after the render, which a group that stays keeps.
  - Its tests, written and not run: `crates/lurq/tests/input/tab_order/keyed_focus.rs`, registered in
    `tests/input/tab_order/mod.rs`.
  - `docs/src/content/docs/focus-navigation.md`, Focus Groups, wants a paragraph the merge adds (this session does not
    edit `docs/`): "A key's handler moves the focus to an element of its group by its key with
    `event.focus_key(key)`: once the handlers have run and what they changed is rendered, the focus goes to the stop of
    the group around the focus whose `.key(..)` is `key`, as the arrows would move it. A calendar reaches the day a
    week on so, which may be in the month the key shows next. The key's default is prevented."
- `ui/mod.rs`: `mod calendar; mod date_picker; mod date_range_picker;` and their `pub use`.
- No keyframes were registered (none is used). No dependency was added.
- **Found, not fixed**: `cargo check -p gallery --features devtools` fails on the branch this was cut from, in
  `ui/combobox.rs` (lines 232 and 308: `Signal<T>` and `Signal<Vec<T>>` of a `T` with no `DevtoolsInspectable` bound).
  The pickers' generic core had the same fault and has the bound (`Picked: DevtoolsInspectable`); the combobox is not
  this session's.

## For `docs/src/content/docs/design-system.md`

## Calendar

The days of a month in weeks, which a person chooses a date from, or a range of dates: a due date, a day to book, the week of a report.

```rust
use lurq::{
  core::{Date, DateRange, Signal, YearMonth},
  ui::{Calendar, CalendarMemory},
};

// due: Signal<Option<Date>>; in a component's render, mounted so that it keeps the month it shows.
Calendar::new(due.clone()).min(Date::today()).mount(ctx);

// The month shown in a signal of the app's: it is not the value.
Calendar::new(due.clone()).month(shown.clone()).mount(ctx);

// A range with two presses, two months side by side, weekends not to be chosen.
Calendar::range(stay.clone())
  .months(2)
  .disabled_days(|date| date.weekday().days_from_monday() >= 5)
  .mount(ctx);

// What it keeps, kept by the app in a field of its component, made once: for a calendar built without ctx.
Calendar::new(due.clone()).memory(&self.memory);
```

| Builder | |
| --- | --- |
| `new(Signal<Option<Date>>)`, `range(Signal<Option<DateRange>>)` | What is chosen: a date, or a range chosen with two presses |
| `month(Signal<YearMonth>)` | The month shown, the first where there are several: not the value. The heading's buttons and the keys set it |
| `memory(&CalendarMemory)` | What the calendar keeps between renders, the app's: the month it shows where the app gives none, a range's first press and where it would end |
| `mount(ctx)` | The calendar with a memory of its own, in the app's locale (`App::i18n`) |
| `months(n)` | How many months side by side, each with its heading |
| `min(Date)`, `max(Date)` | The least and the greatest date that can be chosen. The months beyond are not shown |
| `disabled_days(Fn(Date) -> bool)` | Which days cannot be chosen besides those out of the range |
| `locale(DateLocale)`, `first_day(Weekday)` | The names of months and days and the first day of the week, in place of the app's locale's |
| `week_numbers(bool)` | The ISO 8601 week number before each week |
| `outside_days(bool)` | The days of the months before and after in the first and the last week; on, as the design draws them, and none with several months |
| `today(Date)` | The day marked as today, in place of the system's |
| `disabled(bool)` | It takes no input |
| `on_change(Fn(Date))`, `on_range_change(Fn(DateRange))` | Told of a choice, the value set already |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. Each day has the id with the date after it: `due-2026-11-12`; the buttons `-previous` and `-next` |

- The calendar does not keep what is chosen: the app's signal does. It keeps the month it shows, starting at the month of what is chosen, or of today, and follows the value to its month when the app sets it to a date that is not shown; mounted, or with a `CalendarMemory`, it keeps it between renders. Converted into an element with neither it keeps nothing: its buttons show no other month unless the app gives it the month, and a range shows only once it is chosen.
- A range: the first press starts it, the pointer and the keys show where it would end, and the second ends it, before the first or after it. Its ends are chosen days, and the days between are one band.
- The days are one stop of Tab, at the chosen day, today or the first of the month; the buttons of the heading are stops of their own before them. Left and Right go a day, Up and Down a week, Home and End to the ends of the week, PageUp and PageDown a month and with Shift a year: into the month before or after, which is then shown, and kept between the least and the greatest date. A day that cannot be chosen takes the focus too and says so: the keys pass no day by. Enter and Space choose. The keys are the days' own, through the engine's `KeyboardEvent::focus_key`.
- To what reads the tree each month is a `grid` called by its month (`November 2026`), of `row`s of `gridcell`s called by their day (`Thursday, Nov 12, 2026`) that say `a11y_selected`, `a11y_current` (`date`, for today) and `a11y_disabled`; the names of the days are `columnheader`s.
- Parts: `CalendarPart::Root` (the panel), `Months` (the row of headings and the row of days, its gap the room between months), `Header`, `Title`, `Nav` and `NavIcon` (rules over the outline buttons and their glyphs), `Grid`, `Weekdays`, `Weekday`, `Week`, `WeekNumber`, `Day`. States of the calendar: `CalendarState::Disabled`, `Bare` (in a picker's panel: no fill or edge of its own); of a day: `Selected`, `Today`, `Outside`, `Unavailable`, `RangeStart`, `RangeEnd`, `InRange`.
- In the design the panel is 248 wide, on `Popover` with an edge of 1 in `Border`, `radius-lg` and 12 of padding; the heading is 32 high, the month in the body's type at 500 between two outline buttons of 28 (`radius-md`, edge in `Input`, chevrons of 16 in `MutedForeground`), 8 over the days. A day is 32 square with `radius-md`, its number at 13; the names of the days are at 12 in `MutedForeground`; the rows are 8 apart. Today and a day under the pointer are on `Highlight` (the board's `accent`); the chosen day is on `Primary`, its number in `PrimaryForeground` at 500. A day of the month beside, and one that cannot be chosen, is in `MutedForeground` at half opacity. The days between a range's ends are on `Highlight` with no radius.
- A theme's renderer is given `CalendarView`: `view.months`, `view.chosen`, `view.draft`, `view.hover`, `view.shown_range()`, `view.day_states(date, outside)`, `view.day_recipe(states)`, `view.entry()`, and the pieces `nav(by)`, `header(index)`, `headers()`, `weekdays()`, `day(date, month)`, `week(days, month)`, `grid(index)`, `grids()`, `panel()` and `render()`.

## Date Picker

A field that holds a date, typed in it or chosen from a calendar that opens under it: a due date, a date of birth, the day of a booking.

```rust
use lurq::{
  core::{Date, Signal},
  ui::{CalendarMemory, DatePicker, Field},
};

// due: Signal<Option<Date>>
DatePicker::new(due.clone()).placeholder("Pick a date").clearable(true).mount(ctx);

// In a field, which names it and judges it, with what it keeps kept by the app (made once, in `create`).
Field::new("Due date", DatePicker::new(due.clone()).min(Date::today()).memory(&self.memory))
  .error("The due date has passed.");

// Written and read the app's way.
DatePicker::new(due.clone()).format(|date| date.to_string()).parse(|text| text.parse().ok());
```

| Builder | |
| --- | --- |
| `new(Signal<Option<Date>>)` | The date the field holds |
| `memory(&CalendarMemory)`, `mount(ctx)` | What it keeps between renders (whether the calendar is open, the field it opens under, the month shown): the app's, or its own where it is mounted |
| `open(Signal<bool>)`, `month(Signal<YearMonth>)` | Whether the calendar is open and the month it shows, where the app holds them |
| `placeholder(&str)` | What the field says while it is empty |
| `format(Fn(Date) -> String)`, `parse(Fn(&str) -> Option<Date>)` | How the field writes the date and reads what is typed, in place of the locale's |
| `locale(DateLocale)` | How dates are written and read, and the calendar's names and first day |
| `min`, `max`, `disabled_days`, `first_day`, `week_numbers`, `today` | As the calendar's: what cannot be chosen cannot be typed either |
| `clearable(bool)` | A button at the end of the field that empties it while it holds a date |
| `size(InputSize)`, `invalid`, `warning`, `valid`, `read_only`, `disabled`, `label`, `full_width`, `name` | As an [input](#input)'s. A read-only picker opens no calendar |
| `on_change(Fn(Option<Date>))` | Told of each change, by typing, a choice or the button that empties it |
| `state`, `style`, `rules`, `force`, `id`, `key` | As an input has them. `id` is the field's; the button has it with `-trigger`, the cross `-clear`, the panel `-panel`, the calendar `-calendar` |

- What is typed is read as it is typed, in the locale's short form (`Nov 12, 2026`), in its numbers (`11/12/2026`, `12.11.2026`) or as ISO 8601, a month by its name or the start of it in the locale's words or in English: a date that can be chosen is the value at once, and the field keeps what is half typed while it has the focus. The year has four digits, so `Nov 12, 20` is no date yet. When the focus leaves the field, or on Enter, the text is written again as the picker writes the value; an emptied field is no date, and anything else takes back the date it had.
- The calendar opens in a [popover](#popover) under the field, from the button at its start or from Down in the field, with the focus on the chosen day, today or the first of the month. A choice closes it and the focus goes back to the field, as Escape and a press outside do.
- To what reads the tree the field is a `combobox` that says `a11y_haspopup` `dialog` and `a11y_expanded`, as the date picker of the ARIA patterns is; the panel is a `dialog` called "Choose date". The button that opens it and the cross that empties it are pressed by the pointer and are no stops of Tab.
- In a [field](#field) the picker is called by the field's label, judged as the field is, and disabled where it is. A picker in a field keeps what it keeps in a `CalendarMemory`, since a field is given the picker's builder.
- Parts: `DatePickerPart::Trigger` and `TriggerIcon` (the button at the start and its glyph), `Clear` and `ClearIcon`, `Panel` (rules over the popover's root), and a range picker's `Presets` and `Preset`. The field is an `Input` in the theme's dress, the panel a `Popover`, the days a `Calendar` in the state `Bare`. States: `DatePickerState::Open`, `Disabled`, `FullWidth`, and the input's judgment, size and read only.
- In the design the field is the input, 260 by 36, with a calendar's glyph of 16 in `MutedForeground` 12 from its start and the date 8 after it in the body's type. The glyph is in a button of 24 with `radius-sm`, on `Highlight` under the pointer. The calendar opens 4 under the field, in the popover's panel with no padding of its own, as wide as the calendar.
- A theme's renderer is given `DatePickerView` (`PickerView<Date>`): `view.text`, `view.value`, `view.open`, and the pieces `control()`, `trigger()`, `clear()`, `field()`, `calendar()`, `panel()` and `render()`.

## Date Range Picker

A field that holds a range of dates, typed or chosen from a calendar under it, with ranges offered beside it: the dates of a stay, the period of a report.

```rust
use lurq::{
  core::{Date, DateRange, Signal},
  ui::DateRangePicker,
};

let today = Date::today();
// period: Signal<Option<DateRange>>
DateRangePicker::new(period.clone())
  .placeholder("Pick dates")
  .months(2)
  .preset("Last 7 days", DateRange::new(today.add_days(-6), today))
  .preset("Last 30 days", DateRange::new(today.add_days(-29), today))
  .mount(ctx);
```

| Builder | |
| --- | --- |
| `new(Signal<Option<DateRange>>)` | The range the field holds |
| `months(n)` | How many months the calendar shows side by side: one as the design draws it, or two |
| `preset(label, DateRange)` | A range offered beside the calendar; a press chooses it and closes the calendar. The one chosen is on the secondary fill |
| everything else | As a [date picker](#date-picker)'s, over a range: `format(Fn(DateRange) -> String)`, `parse(Fn(&str) -> Option<DateRange>)`, `on_change(Fn(Option<DateRange>))` |

- The range is in one field, as the design draws it: `Nov 9 – Nov 15, 2026`, the year once where both ends are in it. What is typed is read as two dates in any form the locale reads, with a dash or `to` between them; the year of the first may be left out where it is the second's.
- In the calendar the first press starts the range, the pointer and the keys show where it would end, and the second press ends it and closes the calendar. Escape or a press outside drops a range half chosen. Neither end may be a day that cannot be chosen.
- The field's button has the glyph of a calendar's range; the panel is a `dialog` called "Choose dates".
- A theme's renderer is given `DateRangePickerView` (`PickerView<DateRange>`), with `presets()` besides the date picker's pieces.

## For `CHANGELOG.md`

- `ui::Calendar`: the days of a month to choose a date or a range from (two presses, the pointer showing where it would end), one or several months, the least and greatest dates and the app's days that cannot be chosen, week numbers, the locale's names and first day, the keys of a grid of days on the engine's new `KeyboardEvent::focus_key`, and `CalendarMemory` for what it keeps; with `core::Date`, `YearMonth`, `DateRange`, `Weekday` and `DateLocale`, lurq's own dates and how a locale writes and reads them.
- `ui::DatePicker`: a field that holds a date, typed in the locale's short form, its numbers or ISO 8601 and read as it is typed, or chosen from a calendar in a popover from its button or Down; `format` and `parse` of the app's own, clearable, an input's sizes and judgments, and in a `Field` like an input.
- `ui::DateRangePicker`: a date picker over a range in one field, with presets beside the calendar and one or two months.

## Decided

- **lurq's own date type, no dependency.** The repo has none (`chrono`, `time`, `jiff` are in no manifest), and `i18n` keeps only a tag. A calendar needs a year, a month and a day and their arithmetic, nothing of times or zones; an app that keeps `chrono` or `time` converts at the edge by the year, month and day or by `from_unix_days` / `unix_days` (`NaiveDate::num_days_from_ce() - 719_163`). It is in `core`, not beside `i18n`, which is a feature that is off by default.
- **The locale.** `DateLocale::for_tag` knows English, Ukrainian, German, French, Spanish and Polish; any other tag is English. lurq's locale is the `i18n` tag (`App::i18n().locale()`, "en" by default), which a mounted calendar or picker reads (and is rendered again when it changes); one built without `ctx` is in English unless given `locale(..)`. Plain `en` starts weeks on **Monday** and writes `Nov 12, 2026`, as the board draws them; `en-US` starts on Sunday, `en-GB` writes `12 Nov 2026`; the others write numbers (`12.11.2026`). A heading starts with a capital (`Листопад 2026`).
- **What a calendar keeps is the app's, or a mount's.** A builder has no `Ctx`, and a calendar must keep the month it shows, a range's first press and where the pointer is, and a picker whether it is open and the `ElementRef` of its field. `CalendarMemory` holds them (plain signals and a ref, made once by the app, as the anchor of a `Popover` is), and `mount(ctx)` gives the component one of its own and the app's locale. The memory is what lets a picker sit in a `Field`, which takes the picker's builder: a mounted element cannot be told a field's label. Without either, a calendar built anew in each render keeps nothing, which its docs say.
- **The month shown is not the value**, and each has its own event: `month(Signal<YearMonth>)` and `on_change`. The gallery's Events tab has a line for each. A month kept by the calendar follows the value when the app sets it to a date that is not shown.
- **A day that cannot be chosen takes the focus** and says it cannot be chosen (`a11y_disabled`), as the ARIA date picker dialog has it; Enter does nothing on it. The keys pass no day by, so Up is always the same day a week back. The keys stop at the least and greatest dates.
- **The days of the months beside** are drawn as the board has them, are pressed to choose them and show their month, and are not stops of the keys (the keys reach the day in its own month, which is then shown). With several months they are empty cells: they are in the month beside.
- **The grid's keys are the days' own handlers with `KeyboardEvent::focus_key`**, not a mode of `FocusGroup`: a group moves by the order of its stops, and a calendar's moves are by dates that may not be built yet. The days of all the months shown are one `FocusGroup::both().one_stop(true)` (for Tab's one stop at the entry day, `focus_entry`); every key the group would take is the days' first, so it never moves the focus itself. The headings are in a row of their own over the days so that their buttons are stops of their own and not in that group.
- **The picker's field is the input with its icon turned into a button**: `Input::view` gives the box and its padding, and the picker lays its own button over the input's icon (the board draws no button there; a glyph 16 in a button of 24, so that it can be pressed). The button is not a stop of Tab (Down opens the calendar from the field, as the ARIA date picker combobox has it), and nor is the clear cross.
- **The calendar is the panel**: the popover's root gets the picker's `Panel` rules (no padding, as wide as its content), and the calendar is in the state `Bare` so that the popover's fill, edge, radius and shadow are the panel's, which are the board's calendar's with its shadow.
- **A range picker's presets** are the brief's; the board draws none. They are small ghost buttons in a column before the calendar with a vertical `Separator` between, the chosen one on the secondary fill, and a press chooses the range at once and closes the calendar.
- **No views of months or years.** The board's calendar heading is a text, not a button that goes to the Month Picker (`rm9hw`) or the Year Picker (`zuvYp`), so the calendar does not have those views. What they would take of this: `YearMonth`, `DateLocale::months_short` (Jan … Dec, the Month Picker's cells) and `month_title`, the panel and the outline buttons of the calendar's recipe, and `KeyboardEvent::focus_key` for a grid of four columns. **Date Field (`Ga91O`)** would take `Date`, `DateLocale::order` and `separator` (its segments: `12 / 11 / 2026`, in mono) and `Date::new` for what a segment may hold; its glyph is the picker's. **Event Calendar (`eh5zV`)** would take `YearMonth::weeks(first_day)`, `DateLocale::weekdays_short` and `month_title`. **Mobile/Date Picker Sheet (`nfwiS`)** would take `Calendar` whole in a `Sheet`, with its Cancel and Done buttons over a calendar kept apart from the value until Done (a `CalendarMemory` and a `Signal` of the draft); it draws today as a ring rather than a fill, a style its sheet would lay over the calendar's recipe.
- **The gallery's pages** are in Inputs, after Mentions: Calendar (a playground of a date or a range, one or two
  months, the locale, the first day, the least date, weekends out, week numbers, the days beside, disabled; the date
  and the month shown in two signals, a range over two months, days that cannot be chosen, locales, the keys), Date
  Picker (a playground of every prop; one bound to a signal, two in fields, sizes, states, how dates are written, the
  keys) and Date Range Picker (a playground with presets and two months; a stay over two months, presets, one in a
  field). Each component on a stage keeps what it keeps in a `CalendarMemory` of the page's; the Events tab records
  `on_change`, `on_range_change` and, on the calendar's page, the month shown.
- **Labels** of the buttons ("Previous month", "Next month", "Choose date", "Choose dates", "Clear") are English, as other components' are.

## Not done

## Differs from the board

- Today and a day under the pointer are both on the highlight: the board draws today so (its `9`) and draws no day under the pointer, and the hover of a ghost control in the system is that fill.
- The board's chosen day keeps `PrimaryForeground` at 500; a chosen day that is today is drawn chosen, as the board does not show.
- A day that cannot be chosen is not drawn on the board: it is the muted foreground at half opacity, as a day of the month beside is, with no fill under the pointer.
- The field's glyph is in a button of 24 that the board does not draw, with the highlight under the pointer; the glyph is where the board has it.
- The range picker's calendar on the board shows three weeks without a heading (a sketch of the band); the component shows the whole calendar with its heading. Two months and the presets are not on the board.
- A clearable picker's cross and a calendar's week numbers are not on the board.
