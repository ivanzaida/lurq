//! The page of the date field: its playground, a date bound to a signal and one that may be none, one in a field,
//! its sizes and states, the locales' orders, and what the keys do.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::{Date, DateLocale, Signal},
  layout::Alignment,
  node::Element,
  ui::{DateField, DateFieldSize, DateMemory, Field, Kbd},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .spacing(10.0)
    .child(Row::new().child(shown))
    .child(quiet(caption, 12.0))
}

/// What is on a stage, side by side; what does not fit goes on to the next line.
fn items() -> Row {
  Row::new().width(FILL).spacing(28.0).wrap().align_items(Alignment::End)
}

const SIZES: [(&str, DateFieldSize); 3] = [
  ("Sm", DateFieldSize::Sm),
  ("Md", DateFieldSize::Md),
  ("Lg", DateFieldSize::Lg),
];
const LOCALES: [(&str, &str); 5] = [
  ("en", "en"),
  ("en-GB", "en-GB"),
  ("uk", "uk"),
  ("de", "de"),
  ("fr", "fr"),
];

/// What each key does on a segment.
pub(crate) const SEGMENT_KEYS: [(&str, &str); 7] = [
  (
    "Tab / Shift + Tab",
    "The next or the previous segment: each is a stop, as in a browser's own date input.",
  ),
  ("Left / Right", "The previous or the next segment."),
  (
    "Digits",
    "Fill the segment and go on to the next once it is full, or once no other digit could follow.",
  ),
  (
    "Up / Down",
    "Step the segment, round from the last to the first; an empty year starts at this year.",
  ),
  ("Home / End", "The least and the greatest of the segment."),
  ("Backspace", "Takes the last digit typed, or empties the segment."),
  ("A / P", "In a time on a clock of twelve hours, the half of the day."),
];

/// The keys of a field of segments, a row for each.
pub(crate) fn keys_list(keys: &[(&str, &str)]) -> Column {
  keys
    .iter()
    .fold(Column::new().width(FILL).spacing(10.0), |list, (key, does)| {
      list.child(
        Row::new()
          .width(FILL)
          .align_items(Alignment::Center)
          .child(Row::new().width(200.0).child(Kbd::new(key)))
          .child(Column::new().flex(1.0).child(quiet(does, 13.0))),
      )
    })
}

pub struct DateFieldPage {
  frame: Frame,
  playground: Playground,
  value: Signal<Option<Date>>,
  size: Signal<DateFieldSize>,
  locale: Signal<&'static str>,
  invalid: Signal<bool>,
  read_only: Signal<bool>,
  disabled: Signal<bool>,
  memory: DateMemory,
  born: Signal<Date>,
  due: Signal<Option<Date>>,
  form: Signal<Option<Date>>,
  kept: [DateMemory; 8],
}

impl DateFieldPage {
  /// The playground: the field as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (value, size, locale) = (self.value.get(), self.size.get(), self.locale.get());
    let (invalid, read_only, disabled) = (self.invalid.get(), self.read_only.get(), self.disabled.get());
    let plain = {
      let bound = self.value.clone();
      move |memory: &DateMemory| {
        DateField::optional(bound.clone())
          .size(size)
          .locale(DateLocale::for_tag(locale))
          .invalid(invalid)
          .read_only(read_only)
          .disabled(disabled)
          .label("Due date")
          .memory(memory)
      }
    };
    let record = self.playground.recorder();
    let shown = plain(&self.memory)
      .force(self.playground.forced())
      .on_change(move |date| match date {
        Some(date) => record(format!("on_change(Some({date}))")),
        None => record("on_change(None)".to_owned()),
      });
    let detached = DateMemory::new();
    let make: Arc<dyn Fn() -> Element + Send + Sync> = Arc::new(move || plain(&detached).into());

    let mut code = vec![
      "// due: Signal<Option<Date>>".to_owned(),
      "DateField::optional(due.clone())".to_owned(),
    ];
    if size != DateFieldSize::Md {
      code.push(format!(
        "    .size(DateFieldSize::{})",
        SIZES.iter().find(|(_, held)| *held == size).unwrap().0
      ));
    }
    if locale != "en" {
      code.push(format!("    .locale(DateLocale::for_tag({locale:?}))"));
    }
    for (on, call) in [(invalid, "invalid"), (read_only, "read_only"), (disabled, "disabled")] {
      if on {
        code.push(format!("    .{call}(true)"));
      }
    }
    code.push("    .label(\"Due date\")".to_owned());
    code.push("    .mount(ctx)".to_owned());
    let tree = vec![
      format!("root           {}", self.playground.measured()),
      "├─ segments     a spin button each, 2 apart".to_owned(),
      "│  └─ segment   mono 13, filled on the info color while focused".to_owned(),
      "└─ icon         the calendar's glyph, 16 px".to_owned(),
    ];
    let to = (
      self.value.clone(),
      self.size.clone(),
      self.locale.clone(),
      [self.invalid.clone(), self.read_only.clone(), self.disabled.clone()],
    );
    let (set_size, set_locale) = (self.size.clone(), self.locale.clone());
    self.playground.card(
      ctx,
      Spec {
        title: "Date Field",
        make,
        shown: Shown::of(shown),
        fill: false,
        forceable: true,
        facts: format!("value {}", value.map_or("None".to_owned(), |date| date.to_string())),
        event: "on_change",
        code,
        tree,
        controls: vec![
          prop(
            "size",
            "DateFieldSize",
            self.playground.select(0, &SIZES, &size, move |size| set_size.set(size)),
          ),
          prop(
            "locale",
            "impl DateText",
            self
              .playground
              .select(1, &LOCALES, &locale, move |locale| set_locale.set(locale)),
          ),
          divider(),
          flag("invalid", &self.invalid),
          flag("read_only", &self.read_only),
          flag("disabled", &self.disabled),
        ],
        changed: [size != DateFieldSize::Md, locale != "en", invalid, read_only, disabled]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset: Arc::new(move || {
          to.0.set(Some(Date::today()));
          to.1.set(DateFieldSize::Md);
          to.2.set("en");
          for flag in &to.3 {
            flag.set(false);
          }
        }),
      },
    )
  }
}

impl Component for DateFieldPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 2),
      value: ctx.signal(Some(Date::today())),
      size: ctx.signal(DateFieldSize::Md),
      locale: ctx.signal("en"),
      invalid: ctx.signal(false),
      read_only: ctx.signal(false),
      disabled: ctx.signal(false),
      memory: DateMemory::new(),
      born: ctx.signal(Date::new(1990, 4, 12).unwrap()),
      due: ctx.signal(None),
      form: ctx.signal(None),
      kept: std::array::from_fn(|_| DateMemory::new()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let says = |date: Option<Date>| {
      format!(
        "the signal has: {}",
        date.map_or("None".to_owned(), |date| date.to_string())
      )
    };
    let bound = items()
      .child(item(
        DateField::new(self.born.clone())
          .label("Date of birth")
          .memory(&self.kept[0]),
        &says(Some(self.born.get())),
      ))
      .child(item(
        DateField::optional(self.due.clone())
          .label("Due date")
          .memory(&self.kept[1]),
        &says(self.due.get()),
      ));
    let fields = items().child(
      Field::new(
        "Start date",
        DateField::optional(self.form.clone())
          .min(Date::today())
          .memory(&self.kept[2]),
      )
      .description("From today on."),
    );
    let today = Date::today();
    let states = items()
      .child(item(
        DateField::new(Signal::new(today))
          .size(DateFieldSize::Sm)
          .memory(&self.kept[3]),
        "Sm",
      ))
      .child(item(
        DateField::new(Signal::new(today)).invalid(true).memory(&self.kept[4]),
        "invalid",
      ))
      .child(item(
        DateField::new(Signal::new(today)).read_only(true).memory(&self.kept[5]),
        "read only",
      ))
      .child(item(DateField::new(Signal::new(today)).disabled(true), "disabled"));
    let locales = items()
      .child(item(
        DateField::new(Signal::new(today))
          .locale(DateLocale::for_tag("en-GB"))
          .memory(&self.kept[6]),
        "en-GB: the day first, as the board draws it",
      ))
      .child(item(
        DateField::optional(Signal::new(None))
          .locale(DateLocale::for_tag("de"))
          .memory(&self.kept[7]),
        "de, empty: TT.MM.JJJJ",
      ));
    let rows = [
      (
        "new / optional",
        "Signal<Date> / Signal<Option<Date>>",
        "",
        "The date the field holds, which a date typed whole sets; optional holds none while a segment is empty.",
      ),
      (
        "controlled / controlled_optional",
        "Date / Option<Date>, Fn",
        "",
        "The app's date and what it is told of a date typed, which it answers with the date it renders next.",
      ),
      (
        "memory",
        "&DateMemory",
        "",
        "What the field keeps of what is half typed, kept by the app. mount(ctx) keeps one of its own.",
      ),
      (
        "min / max",
        "Date",
        "None",
        "The least and the greatest date that is the field's.",
      ),
      (
        "locale",
        "impl DateText",
        "app's",
        "The order of the segments, what stands between them, their names and what an empty one shows.",
      ),
      (
        "today",
        "Date",
        "system's",
        "The day an empty year starts at under the arrows.",
      ),
      (
        "icon",
        "impl Into<IconChoice>",
        "calendar",
        "The glyph at the end, or None for none.",
      ),
      (
        "size, invalid, warning, valid, read_only, disabled, full_width, label, name",
        "",
        "",
        "As an input's; DateFieldSize is TextBoxSize. The form holds the date in ISO 8601.",
      ),
      (
        "on_change",
        "Fn(Date) / Fn(Option<Date>)",
        "",
        "Called when a person changes the date: once every segment is filled, or when one is emptied.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Inputs", "Date Field"],
        title: "Date Field",
        description: "A date typed in by its segments, the day, the month and the year, each stepped by the arrows: \
                      a date of birth, a date in a form where a calendar would be in the way.",
        import: Some("use lurq::ui::DateField;"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of DateField is a control on the right. The field on the stage is live: press a segment and \
           type, or step it with the arrows.",
          playground,
        ),
        Section::new(
          "Date field",
          "A date that is always there, and one that may be none: the second holds none while a segment is empty.",
          self.frame.demo(
            0,
            bound,
            "DateField::new(born.clone())",
            &[
              "// born: Signal<Date>, due: Signal<Option<Date>>",
              "DateField::new(born.clone()).label(\"Date of birth\").mount(ctx)",
              "DateField::optional(due.clone()).label(\"Due date\").mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "In a field",
          "A field names it and judges it; what it keeps is in a DateMemory the page made once.",
          self.frame.demo(
            1,
            fields,
            "Field::new(\"Start date\", DateField::optional(start.clone()).memory(&memory))",
            &[
              "Field::new(",
              "    \"Start date\",",
              "    DateField::optional(start.clone()).min(Date::today()).memory(&memory),",
              ")",
              ".description(\"From today on.\")",
            ],
          ),
        ),
        Section::new(
          "Sizes and states",
          "An input's sizes and judgments, read only and disabled.",
          self.frame.demo(
            2,
            states,
            ".invalid(true)",
            &[
              "DateField::new(value.clone()).size(DateFieldSize::Sm)",
              "DateField::new(value.clone()).invalid(true)",
              "DateField::new(value.clone()).read_only(true)",
              "DateField::new(value.clone()).disabled(true)",
            ],
          ),
        ),
        Section::new(
          "Locales",
          "The order of the segments, the separator, their names and what an empty one shows are the locale's: the \
           field's own, else the app's (app.set_date_locale), else English, which writes the month first.",
          self.frame.demo(
            3,
            locales,
            ".locale(DateLocale::for_tag(\"en-GB\"))",
            &[
              "DateField::new(value.clone()).locale(DateLocale::for_tag(\"en-GB\"))",
              "DateField::optional(value.clone()).locale(DateLocale::for_tag(\"de\"))",
            ],
          ),
        ),
        Section::new("Keyboard", "Each segment is a spin button.", keys_list(&SEGMENT_KEYS)),
        Section::new(
          "Props",
          "What a date field is given.",
          props_table(&rows, &["new / optional"], self.frame.room()),
        ),
      ],
    )
  }
}
