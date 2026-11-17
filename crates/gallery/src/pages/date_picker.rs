//! The page of the date picker: its playground, a date bound to a signal, one in a field, its sizes and states, a
//! format of the app's own, and what the keys do.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::{Date, DateLocale, DateStyle, Signal},
  layout::Alignment,
  node::Element,
  ui::{DateMemory, DatePicker, DatePickerSize, Field, Interaction, Kbd},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .max_width(FILL)
    .spacing(10.0)
    .child(Row::new().max_width(FILL).child(shown))
    .child(quiet(caption, 12.0))
}

/// What is on a stage, side by side; what does not fit goes on to the next line.
fn items() -> Row {
  Row::new().width(FILL).spacing(28.0).wrap().align_items(Alignment::End)
}

const SIZES: [(&str, DatePickerSize); 3] = [
  ("Sm", DatePickerSize::Sm),
  ("Md", DatePickerSize::Md),
  ("Lg", DatePickerSize::Lg),
];
const LOCALES: [(&str, &str); 5] = [
  ("en", "en"),
  ("en-US", "en-US"),
  ("en-GB", "en-GB"),
  ("uk", "uk"),
  ("de", "de"),
];
/// The least date the playground offers: none, or today.
const MINS: [(&str, bool); 2] = [("None", false), ("Today", true)];
/// Whether weekends cannot be chosen.
const BLOCKED: [(&str, bool); 2] = [("None", false), ("Weekends", true)];

/// What each key does in a date picker.
const KEYS: [(&str, &str); 6] = [
  (
    "Down",
    "In the field, opens the calendar with the focus on the chosen day, today or the first of the month.",
  ),
  (
    "Enter",
    "In the field, writes what is typed as the picker writes dates, and goes on to send the form.",
  ),
  (
    "Arrows, Home, End, Page Up / Down",
    "In the calendar, move between days, weeks and months.",
  ),
  (
    "Enter / Space",
    "In the calendar, choose the day: the calendar closes and the focus is back in the field.",
  ),
  (
    "Escape",
    "Closes the calendar with nothing chosen; the focus is back in the field.",
  ),
  (
    "Tab",
    "From the field, past its buttons, which the pointer presses; from the open calendar, through it.",
  ),
];

fn weekend(date: Date) -> bool {
  date.weekday().days_from_monday() >= 5
}

fn says(date: Option<Date>) -> String {
  match date {
    Some(date) => format!("the signal has: {date}"),
    None => "the signal has: None".to_owned(),
  }
}

pub struct DatePickerPage {
  frame: Frame,
  playground: Playground,
  value: Signal<Option<Date>>,
  placeholder: Signal<String>,
  size: Signal<DatePickerSize>,
  locale: Signal<&'static str>,
  min: Signal<bool>,
  blocked: Signal<bool>,
  clearable: Signal<bool>,
  week_numbers: Signal<bool>,
  invalid: Signal<bool>,
  read_only: Signal<bool>,
  disabled: Signal<bool>,
  memory: DateMemory,
  due: Signal<Option<Date>>,
  start: Signal<Option<Date>>,
  iso: Signal<Option<Date>>,
  kept: [DateMemory; 12],
}

impl DatePickerPage {
  /// The playground: the picker as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (value, placeholder, size, locale) = (
      self.value.get(),
      self.placeholder.get(),
      self.size.get(),
      self.locale.get(),
    );
    let (min, blocked, clearable, week_numbers) = (
      self.min.get(),
      self.blocked.get(),
      self.clearable.get(),
      self.week_numbers.get(),
    );
    let (invalid, read_only, disabled) = (self.invalid.get(), self.read_only.get(), self.disabled.get());
    let open = self.memory.is_open();
    let today = Date::today();
    let plain = {
      let (bound, placeholder) = (self.value.clone(), placeholder.clone());
      move |memory: &DateMemory| {
        let mut picker = DatePicker::new(bound.clone())
          .placeholder(&placeholder)
          .size(size)
          .locale(DateLocale::for_tag(locale))
          .clearable(clearable)
          .week_numbers(week_numbers)
          .invalid(invalid)
          .read_only(read_only)
          .disabled(disabled)
          .label("Due date")
          .memory(memory);
        if min {
          picker = picker.min(today);
        }
        if blocked {
          picker = picker.disabled_days(weekend);
        }
        picker
      }
    };
    let (record, record_open, record_dismiss, record_submit) = (
      self.playground.recorder(),
      self.playground.recorder(),
      self.playground.recorder(),
      self.playground.recorder(),
    );
    let said = |date: Option<Date>| date.map_or("None".to_owned(), |date| format!("Some({date})"));
    let shown = plain(&self.memory)
      .force(self.playground.forced())
      .on_change(move |date| record(format!("on_change({})", said(date))))
      .on_submit(move |date| record_submit(format!("on_submit({})", said(date))))
      .on_open_change(move |open| record_open(format!("on_open_change({open})")))
      .on_dismiss(move || record_dismiss("on_dismiss()".to_owned()));
    let detached = DateMemory::new();
    let make: Arc<dyn Fn() -> Element + Send + Sync> = Arc::new(move || plain(&detached).into());

    let mut code = vec![
      "// due: Signal<Option<Date>>".to_owned(),
      "DatePicker::new(due.clone())".to_owned(),
      format!("    .placeholder({placeholder:?})"),
    ];
    if size != DatePickerSize::Md {
      code.push(format!(
        "    .size(DatePickerSize::{})",
        SIZES.iter().find(|(_, held)| *held == size).unwrap().0
      ));
    }
    if locale != "en" {
      code.push(format!("    .locale(DateLocale::for_tag({locale:?}))"));
    }
    if min {
      code.push("    .min(Date::today())".to_owned());
    }
    if blocked {
      code.push("    .disabled_days(|date| date.weekday().days_from_monday() >= 5)".to_owned());
    }
    for (on, call) in [
      (clearable, "clearable"),
      (week_numbers, "week_numbers"),
      (invalid, "invalid"),
      (read_only, "read_only"),
      (disabled, "disabled"),
    ] {
      if on {
        code.push(format!("    .{call}(true)"));
      }
    }
    code.push("    .label(\"Due date\")".to_owned());
    code.push("    .mount(ctx)".to_owned());

    let tree = vec![
      format!("root           {}", self.playground.measured()),
      "├─ field        the input, typed in: a combobox".to_owned(),
      "│  ├─ trigger   24 × 24 px, the calendar's glyph".to_owned(),
      if clearable {
        "│  └─ clear     24 × 24 px, a cross".to_owned()
      } else {
        "│  └─ (clear)   where clearable".to_owned()
      },
      format!(
        "└─ panel        a popover 4 under the field, {}",
        if open { "open" } else { "closed" }
      ),
      "   └─ calendar  bare in the panel".to_owned(),
    ];
    let to = (
      self.value.clone(),
      self.placeholder.clone(),
      self.size.clone(),
      self.locale.clone(),
      self.min.clone(),
      self.blocked.clone(),
      [
        self.clearable.clone(),
        self.week_numbers.clone(),
        self.invalid.clone(),
        self.read_only.clone(),
        self.disabled.clone(),
      ],
    );
    let (set_size, set_locale, set_min, set_blocked) = (
      self.size.clone(),
      self.locale.clone(),
      self.min.clone(),
      self.blocked.clone(),
    );
    self.playground.card(
      ctx,
      Spec {
        title: "Date Picker",
        make,
        shown: Shown::of(self.playground.styled(shown)),
        fill: false,
        forceable: true,
        facts: format!(
          "value {} · {}",
          value.map_or("None".to_owned(), |date| date.to_string()),
          if open { "open" } else { "closed" }
        ),
        event: "on_change",
        code,
        tree,
        controls: vec![
          prop("placeholder", "&str", field(&self.placeholder)),
          prop(
            "size",
            "DatePickerSize",
            self.playground.select(0, &SIZES, &size, move |size| set_size.set(size)),
          ),
          prop(
            "locale",
            "impl DateText",
            self
              .playground
              .select(1, &LOCALES, &locale, move |locale| set_locale.set(locale)),
          ),
          prop(
            "min",
            "Date",
            self.playground.select(2, &MINS, &min, move |min| set_min.set(min)),
          ),
          prop(
            "disabled_days",
            "Fn(Date) -> bool",
            self
              .playground
              .select(3, &BLOCKED, &blocked, move |blocked| set_blocked.set(blocked)),
          ),
          divider(),
          flag("clearable", &self.clearable),
          flag("week_numbers", &self.week_numbers),
          flag("invalid", &self.invalid),
          flag("read_only", &self.read_only),
          flag("disabled", &self.disabled),
        ],
        changed: [
          placeholder != "Pick a date",
          size != DatePickerSize::Md,
          locale != "en",
          min,
          blocked,
          clearable,
          week_numbers,
          invalid,
          read_only,
          disabled,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          to.0.set(Some(Date::today()));
          to.1.set("Pick a date".to_owned());
          to.2.set(DatePickerSize::Md);
          to.3.set("en");
          to.4.set(false);
          to.5.set(false);
          for flag in &to.6 {
            flag.set(false);
          }
        }),
      },
    )
  }
}

impl Component for DatePickerPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 4),
      value: ctx.signal(Some(Date::today())),
      placeholder: ctx.signal("Pick a date".to_owned()),
      size: ctx.signal(DatePickerSize::Md),
      locale: ctx.signal("en"),
      min: ctx.signal(false),
      blocked: ctx.signal(false),
      clearable: ctx.signal(false),
      week_numbers: ctx.signal(false),
      invalid: ctx.signal(false),
      read_only: ctx.signal(false),
      disabled: ctx.signal(false),
      memory: DateMemory::new(),
      due: ctx.signal(None),
      start: ctx.signal(None),
      iso: ctx.signal(Some(Date::today())),
      kept: std::array::from_fn(|_| DateMemory::new()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let today = Date::today();
    let bound = items().child(item(
      DatePicker::new(self.due.clone())
        .placeholder("Pick a date")
        .clearable(true)
        .memory(&self.kept[0]),
      &says(self.due.get()),
    ));
    let fields = items()
      .child(
        Field::new(
          "Start date",
          DatePicker::new(self.start.clone())
            .placeholder("Pick a date")
            .min(today)
            .memory(&self.kept[1]),
        )
        .description("From today on."),
      )
      .child(
        Field::new(
          "End date",
          DatePicker::new(Signal::new(Some(today.add_days(-3)))).memory(&self.kept[2]),
        )
        .error("The end is before the start."),
      );
    let sizes = items()
      .child(item(
        DatePicker::new(Signal::new(Some(today)))
          .size(DatePickerSize::Sm)
          .memory(&self.kept[3]),
        "Sm · 32",
      ))
      .child(item(
        DatePicker::new(Signal::new(Some(today))).memory(&self.kept[4]),
        "Md · 36",
      ))
      .child(item(
        DatePicker::new(Signal::new(Some(today)))
          .size(DatePickerSize::Lg)
          .memory(&self.kept[5]),
        "Lg · 40",
      ));
    let states = items()
      .child(item(
        DatePicker::new(Signal::new(None))
          .placeholder("Pick a date")
          .memory(&self.kept[6]),
        "empty",
      ))
      .child(item(
        DatePicker::new(Signal::new(Some(today))).force(Some(Interaction::FocusVisible)),
        "focused",
      ))
      .child(item(
        DatePicker::new(Signal::new(Some(today))).read_only(true),
        "read only",
      ))
      .child(item(
        DatePicker::new(Signal::new(Some(today))).disabled(true),
        "disabled",
      ));
    let formats = items()
      .child(item(
        DatePicker::new(self.iso.clone())
          .format(|date| date.to_string())
          .parse(|text| text.parse().ok())
          .memory(&self.kept[7]),
        "ISO 8601, the app's own",
      ))
      .child(item(
        DatePicker::new(Signal::new(Some(today)))
          .locale(DateLocale::for_tag("uk"))
          .memory(&self.kept[8]),
        "uk",
      ))
      .child(item(
        DatePicker::new(Signal::new(Some(today)))
          .locale(DateLocale::for_tag("en-GB"))
          .memory(&self.kept[9]),
        "en-GB",
      ));
    let own = items()
      .child(item(
        DatePicker::new(Signal::new(Some(today)))
          .locale(DateLocale::for_tag("en-GB").with_style(DateStyle::Long))
          .memory(&self.kept[10]),
        "en-GB, the long style",
      ))
      .child(item(
        DatePicker::new(Signal::new(Some(today)))
          .locale(super::calendar::RomanDates)
          .memory(&self.kept[11]),
        "a DateText of the page's own: type 3.XII.2026",
      ));
    let keyboard = KEYS
      .into_iter()
      .fold(Column::new().width(FILL).spacing(10.0), |list, (key, does)| {
        list.child(
          Row::new()
            .width(FILL)
            .align_items(Alignment::Center)
            .child(Row::new().width(240.0).child(Kbd::new(key)))
            // What the key does wraps in the room beside the key.
            .child(Column::new().flex(1.0).child(quiet(does, 13.0))),
        )
      });
    let rows = [
      (
        "value",
        "Signal<Option<Date>>",
        "",
        "The date the field holds: typed in it, or chosen from its calendar.",
      ),
      (
        "memory",
        "&DateMemory",
        "",
        "What the picker keeps between renders, kept by the app: whether it is open, the month shown. mount(ctx) \
         keeps one of its own.",
      ),
      ("placeholder", "&str", "", "What the field says while it is empty."),
      (
        "controlled",
        "Option<Date>, Fn(Option<Date>)",
        "",
        "The app's date and what it is told of an edit or a choice, which it answers with the date it renders next.",
      ),
      (
        "size",
        "DatePickerSize",
        "Md",
        "The height of the field, an input's: 32, 36 or 40.",
      ),
      (
        "format / parse",
        "Fn(Date) -> String / Fn(&str) -> Option<Date>",
        "locale's",
        "How the field writes the date and reads what is typed.",
      ),
      (
        "locale",
        "impl DateText",
        "app's",
        "How dates are written and read, and the calendar's names and first day.",
      ),
      (
        "min / max",
        "Date",
        "None",
        "The least and the greatest date that can be chosen or typed.",
      ),
      (
        "disabled_days",
        "Fn(Date) -> bool",
        "",
        "Which days cannot be chosen or typed besides those out of the range.",
      ),
      (
        "clearable",
        "bool",
        "false",
        "A button at the end of the field that empties it.",
      ),
      ("week_numbers", "bool", "false", "Week numbers in the calendar."),
      (
        "open",
        "Signal<bool>",
        "kept",
        "Whether the calendar is open, where the app holds it.",
      ),
      (
        "placement / offset / collision",
        "Placement / SpacingValue / CollisionStrategy",
        "BottomStart / 4 / FlipThenClamp",
        "Where the calendar opens beside the field, how far from it, and where it goes with no room.",
      ),
      (
        "inline",
        "bool",
        "false",
        "The panel drawn in place under the field, open or not: for a page that holds a calendar.",
      ),
      (
        "month",
        "Signal<YearMonth>",
        "kept",
        "The month the calendar shows, where the app holds it.",
      ),
      (
        "invalid / warning / valid",
        "bool",
        "false",
        "The judgment of what it holds: the edge in the judgment's color. A field gives its own.",
      ),
      (
        "read_only",
        "bool",
        "false",
        "Read and copied, not changed: it opens no calendar.",
      ),
      ("disabled", "bool", "false", "Half the opacity; it takes no input."),
      (
        "label",
        "&str",
        "",
        "What it is called to tooling. A field gives it its label.",
      ),
      (
        "trigger_label / clear_label / panel_label",
        "&str",
        "English",
        "What the button, the cross and the panel are called to tooling: Choose date, Clear, Choose date.",
      ),
      (
        "previous_label / next_label",
        "&str",
        "English",
        "What the calendar's buttons are called: Previous month, Next month.",
      ),
      ("name", "&str", "", "The name of its value in the form it is in."),
      (
        "on_change",
        "Fn(Option<Date>)",
        "",
        "Called when a person changes the date, after a choice closed the calendar: not for the date it holds.",
      ),
      (
        "on_input",
        "Fn(&TextInputEvent)",
        "",
        "Called before an edit of the text, which it may refuse.",
      ),
      (
        "on_submit",
        "Fn(Option<Date>)",
        "",
        "Called on Enter, once the text is put right, with the date it holds.",
      ),
      (
        "on_open_change / on_dismiss",
        "Fn(bool) / Fn()",
        "",
        "Called when the calendar opens or closes, and when Escape or a press outside closed it.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Inputs", "Date Picker"],
        title: "Date Picker",
        description: "A field that holds a date, typed in it or chosen from a calendar that opens under it: a due \
                      date, a date of birth, the day of a booking.",
        import: Some("use lurq::ui::DatePicker;"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of DatePicker is a control on the right. The picker on the stage is live: type a date in it, \
           press its button or Down to open the calendar, and choose a day.",
          playground,
        ),
        Section::new(
          "Date picker",
          "The page holds the date in a signal. What is typed is the date as soon as it is one, in the locale's \
           short form, its numbers or ISO 8601; when the focus leaves, it is written as the picker writes dates.",
          self.frame.demo(
            0,
            bound,
            "DatePicker::new(due.clone())",
            &[
              "// due: Signal<Option<Date>>",
              "DatePicker::new(due.clone())",
              "    .placeholder(\"Pick a date\")",
              "    .clearable(true)",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "In a field",
          "A field names it and judges it. What the picker keeps is in a DateMemory the page made once.",
          self.frame.demo(
            1,
            fields,
            "Field::new(\"Start date\", DatePicker::new(start.clone()).memory(&memory))",
            &[
              "// start: Signal<Option<Date>>, memory: DateMemory, both fields of the component",
              "Field::new(",
              "    \"Start date\",",
              "    DatePicker::new(start.clone()).min(Date::today()).memory(&memory),",
              ")",
              ".description(\"From today on.\")",
            ],
          ),
        ),
        Section::new(
          "Sizes",
          "The heights of the input: 32, 36 and 40.",
          self.frame.demo(
            2,
            sizes,
            ".size(DatePickerSize::Sm)",
            &[
              "DatePicker::new(value.clone()).size(DatePickerSize::Sm)",
              "DatePicker::new(value.clone())",
              "DatePicker::new(value.clone()).size(DatePickerSize::Lg)",
            ],
          ),
        ),
        Section::new(
          "States",
          "Empty with its placeholder, focused, read only and disabled, as an input is.",
          self.frame.demo(
            3,
            states,
            ".read_only(true)",
            &[
              "DatePicker::new(value.clone()).placeholder(\"Pick a date\")",
              "DatePicker::new(value.clone()).force(Some(Interaction::FocusVisible))",
              "DatePicker::new(value.clone()).read_only(true)",
              "DatePicker::new(value.clone()).disabled(true)",
            ],
          ),
        ),
        Section::new(
          "How dates are written",
          "The locale writes and reads them; an app writes them its own way with format and parse.",
          self.frame.demo(
            4,
            formats,
            ".format(|date| date.to_string())",
            &[
              "DatePicker::new(value.clone())",
              "    .format(|date| date.to_string())",
              "    .parse(|text| text.parse().ok())",
              "",
              "DatePicker::new(value.clone()).locale(DateLocale::for_tag(\"uk\"))",
              "DatePicker::new(value.clone()).locale(DateLocale::for_tag(\"en-GB\"))",
            ],
          ),
        ),
        Section::new(
          "A locale of the app's own",
          "A style covers the common forms (numbers, the month's short or long name, ISO 8601); a DateText of the \
           app's own covers the rest, here the months in Roman numerals, which the field writes and reads. Each is \
           given to one picker with .locale(..), or to the whole app with app.set_date_locale(..).",
          self.frame.demo(
            5,
            own,
            ".locale(RomanDates)",
            &[
              "DatePicker::new(value.clone())",
              "    .locale(DateLocale::for_tag(\"en-GB\").with_style(DateStyle::Long))",
              "",
              "// RomanDates: the page's own DateText, as on the Calendar's page.",
              "DatePicker::new(value.clone()).locale(RomanDates)",
              "",
              "// Or once, for every calendar and picker of the app:",
              "app.set_date_locale(RomanDates)",
            ],
          ),
        ),
        Section::new(
          "Keyboard",
          "The field is a combobox that opens a dialog; the calendar in it is a grid of days.",
          keyboard,
        ),
        Section::new(
          "Props",
          "What a date picker is given.",
          props_table(&rows, &["value"], self.frame.room()),
        ),
      ],
    )
  }
}
