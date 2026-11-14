//! The page of the month picker: its playground, a month bound to a signal and one that may be none, a range, the
//! locales, and what the keys do.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::{DateLocale, Signal, YearMonth},
  layout::Alignment,
  node::Element,
  ui::{DateMemory, Kbd, MonthPicker},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .spacing(10.0)
    .child(Row::new().child(shown))
    .child(quiet(caption, 12.0))
}

fn items() -> Row {
  Row::new()
    .width(FILL)
    .spacing(28.0)
    .wrap()
    .align_items(Alignment::Start)
}

const LOCALES: [(&str, &str); 4] = [("en", "en"), ("uk", "uk"), ("de", "de"), ("fr", "fr")];

/// What each key does on a cell of a month or year picker.
pub(crate) const GRID_KEYS: [(&str, &str); 6] = [
  (
    "Tab",
    "Into the cells at the chosen one, this one or the first that can be chosen; Tab again leaves them.",
  ),
  (
    "Left / Right",
    "The cell before or after, into the page beside, which is then shown.",
  ),
  ("Up / Down", "The cell a row above or below: four before or after."),
  ("Home / End", "The first or the last cell of the row."),
  (
    "Page Up / Down",
    "The same cell a page before or after: a year of months, ten years.",
  ),
  ("Enter / Space", "Chooses the cell."),
];

/// The keys of a grid of a picker, a row for each.
pub(crate) fn grid_keys() -> Column {
  GRID_KEYS
    .into_iter()
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

pub struct MonthPickerPage {
  frame: Frame,
  playground: Playground,
  value: Signal<Option<YearMonth>>,
  locale: Signal<&'static str>,
  ranged: Signal<bool>,
  disabled: Signal<bool>,
  memory: DateMemory,
  report: Signal<YearMonth>,
  expiry: Signal<Option<YearMonth>>,
  kept: [DateMemory; 4],
}

impl MonthPickerPage {
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (value, locale) = (self.value.get(), self.locale.get());
    let (ranged, disabled) = (self.ranged.get(), self.disabled.get());
    let now = YearMonth::current();
    let plain = {
      let bound = self.value.clone();
      move |memory: &DateMemory| {
        let mut picker = MonthPicker::optional(bound.clone())
          .locale(DateLocale::for_tag(locale))
          .disabled(disabled)
          .label("Report month")
          .memory(memory);
        if ranged {
          picker = picker.min(now.add_months(-6)).max(now);
        }
        picker
      }
    };
    let record = self.playground.recorder();
    let shown = plain(&self.memory)
      .force(self.playground.forced())
      .on_change(move |month| match month {
        Some(month) => record(format!("on_change(Some({month}))")),
        None => record("on_change(None)".to_owned()),
      });
    let detached = DateMemory::new();
    let make: Arc<dyn Fn() -> Element + Send + Sync> = Arc::new(move || plain(&detached).into());
    let mut code = vec![
      "// month: Signal<Option<YearMonth>>".to_owned(),
      "MonthPicker::optional(month.clone())".to_owned(),
    ];
    if locale != "en" {
      code.push(format!("    .locale(DateLocale::for_tag({locale:?}))"));
    }
    if ranged {
      code.push("    .min(now.add_months(-6))".to_owned());
      code.push("    .max(now)".to_owned());
    }
    if disabled {
      code.push("    .disabled(true)".to_owned());
    }
    code.push("    .label(\"Report month\")".to_owned());
    code.push("    .mount(ctx)".to_owned());
    let tree = vec![
      format!("root           {}   padding 12", self.playground.measured()),
      "├─ header       the year between two bare chevrons, 28 high".to_owned(),
      "└─ grid         three rows of four months, 8 apart".to_owned(),
      "   └─ cell      34 high, 4 apart, radius 6".to_owned(),
    ];
    let to = (
      self.value.clone(),
      self.locale.clone(),
      [self.ranged.clone(), self.disabled.clone()],
    );
    let set_locale = self.locale.clone();
    self.playground.card(
      ctx,
      Spec {
        title: "Month Picker",
        make,
        shown: Shown::of(shown),
        fill: false,
        forceable: true,
        facts: format!("value {}", value.map_or("None".to_owned(), |month| month.to_string())),
        event: "on_change",
        code,
        tree,
        controls: vec![
          prop(
            "locale",
            "impl DateText",
            self
              .playground
              .select(0, &LOCALES, &locale, move |locale| set_locale.set(locale)),
          ),
          divider(),
          flag("min / max", &self.ranged),
          flag("disabled", &self.disabled),
        ],
        changed: [locale != "en", ranged, disabled]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset: Arc::new(move || {
          to.0.set(Some(YearMonth::current()));
          to.1.set("en");
          for flag in &to.2 {
            flag.set(false);
          }
        }),
      },
    )
  }
}

impl Component for MonthPickerPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 1),
      value: ctx.signal(Some(YearMonth::current())),
      locale: ctx.signal("en"),
      ranged: ctx.signal(false),
      disabled: ctx.signal(false),
      memory: DateMemory::new(),
      report: ctx.signal(YearMonth::current()),
      expiry: ctx.signal(None),
      kept: std::array::from_fn(|_| DateMemory::new()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let now = YearMonth::current();
    let bound = items()
      .child(item(
        MonthPicker::new(self.report.clone()).max(now).memory(&self.kept[0]),
        &format!("the signal has: {} · up to this month", self.report.get()),
      ))
      .child(item(
        MonthPicker::optional(self.expiry.clone())
          .min(now)
          .memory(&self.kept[1]),
        &format!(
          "the signal has: {} · from this month",
          self.expiry.get().map_or("None".to_owned(), |month| month.to_string())
        ),
      ));
    let locales = items()
      .child(item(
        MonthPicker::new(Signal::new(now))
          .locale(DateLocale::for_tag("uk"))
          .memory(&self.kept[2]),
        "uk",
      ))
      .child(item(
        MonthPicker::new(Signal::new(now))
          .locale(DateLocale::for_tag("de"))
          .memory(&self.kept[3]),
        "de",
      ));
    let rows = [
      (
        "new / optional",
        "Signal<YearMonth> / Signal<Option<YearMonth>>",
        "",
        "The month the picker holds, which a press sets; optional holds none until one is chosen.",
      ),
      (
        "controlled / controlled_optional",
        "YearMonth / Option<YearMonth>, Fn",
        "",
        "The app's month and what it is told of a choice.",
      ),
      (
        "min / max",
        "YearMonth",
        "None",
        "The least and the greatest month that can be chosen; the years beyond are not shown.",
      ),
      ("today", "YearMonth", "system's", "The month marked as this month."),
      (
        "locale",
        "impl DateText",
        "app's",
        "The short names of the months and the names of the buttons.",
      ),
      (
        "previous_label / next_label",
        "&str",
        "locale's",
        "What the buttons of the heading are called: Previous year, Next year.",
      ),
      (
        "memory",
        "&DateMemory",
        "",
        "The year shown, kept by the app; mount(ctx) keeps its own.",
      ),
      (
        "label, invalid, disabled",
        "",
        "",
        "Its name to tooling, a field's judgment, no input.",
      ),
      (
        "on_change",
        "Fn(YearMonth) / Fn(Option<YearMonth>)",
        "",
        "Called when a person chooses another month.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Inputs", "Month Picker"],
        title: "Month Picker",
        description: "The twelve months of a year, of which a person chooses one, and the years before and after \
                      under the buttons of its heading: the month of a report, the expiry of a card.",
        import: Some("use lurq::ui::MonthPicker;"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of MonthPicker is a control on the right. Press a month, the chevrons, or Tab into the months \
           and use the arrows.",
          playground,
        ),
        Section::new(
          "Month picker",
          "A month bound to a signal, up to this month; and one that may be none, from this month on.",
          self.frame.demo(
            0,
            bound,
            "MonthPicker::new(report.clone()).max(now)",
            &[
              "let now = YearMonth::current();",
              "MonthPicker::new(report.clone()).max(now).mount(ctx)",
              "MonthPicker::optional(expiry.clone()).min(now).mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Locales",
          "The months' short names are the locale's: the picker's own, else the app's, else English.",
          self.frame.demo(
            1,
            locales,
            ".locale(DateLocale::for_tag(\"uk\"))",
            &[
              "MonthPicker::new(month.clone()).locale(DateLocale::for_tag(\"uk\"))",
              "MonthPicker::new(month.clone()).locale(DateLocale::for_tag(\"de\"))",
            ],
          ),
        ),
        Section::new("Keyboard", "The months are a grid.", grid_keys()),
        Section::new(
          "Props",
          "What a month picker is given.",
          props_table(&rows, &["new / optional"], self.frame.room()),
        ),
      ],
    )
  }
}
