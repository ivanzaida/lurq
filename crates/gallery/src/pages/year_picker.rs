//! The page of the year picker: its playground, a year bound to a signal and one that may be none, a range, and what
//! the keys do.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::{Date, Signal},
  layout::Alignment,
  node::Element,
  ui::{DateMemory, YearPicker},
};

use super::month_picker::grid_keys;
use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Shown, Spec, flag},
};

fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .max_width(FILL)
    .spacing(10.0)
    .child(Row::new().max_width(FILL).child(shown))
    .child(quiet(caption, 12.0))
}

fn items() -> Row {
  Row::new()
    .width(FILL)
    .spacing(28.0)
    .wrap()
    .align_items(Alignment::Start)
}

pub struct YearPickerPage {
  frame: Frame,
  playground: Playground,
  value: Signal<Option<i32>>,
  ranged: Signal<bool>,
  disabled: Signal<bool>,
  memory: DateMemory,
  year: Signal<i32>,
  founded: Signal<Option<i32>>,
  kept: [DateMemory; 2],
}

impl YearPickerPage {
  fn play(&self, ctx: &mut Ctx) -> Element {
    let value = self.value.get();
    let (ranged, disabled) = (self.ranged.get(), self.disabled.get());
    let this_year = Date::today().year();
    let plain = {
      let bound = self.value.clone();
      move |memory: &DateMemory| {
        let mut picker = YearPicker::optional(bound.clone())
          .disabled(disabled)
          .label("Year")
          .memory(memory);
        if ranged {
          picker = picker.min(1990).max(this_year);
        }
        picker
      }
    };
    let record = self.playground.recorder();
    let shown = plain(&self.memory)
      .force(self.playground.forced())
      .on_change(move |year| match year {
        Some(year) => record(format!("on_change(Some({year}))")),
        None => record("on_change(None)".to_owned()),
      });
    let detached = DateMemory::new();
    let make: Arc<dyn Fn() -> Element + Send + Sync> = Arc::new(move || plain(&detached).into());
    let mut code = vec![
      "// year: Signal<Option<i32>>".to_owned(),
      "YearPicker::optional(year.clone())".to_owned(),
    ];
    if ranged {
      code.push("    .min(1990)".to_owned());
      code.push("    .max(Date::today().year())".to_owned());
    }
    if disabled {
      code.push("    .disabled(true)".to_owned());
    }
    code.push("    .label(\"Year\")".to_owned());
    code.push("    .mount(ctx)".to_owned());
    let tree = vec![
      format!("root           {}   padding 12", self.playground.measured()),
      "├─ header       the years shown between two bare chevrons".to_owned(),
      "└─ grid         twelve years from a decade, four to a row".to_owned(),
    ];
    let to = (self.value.clone(), [self.ranged.clone(), self.disabled.clone()]);
    self.playground.card(
      ctx,
      Spec {
        title: "Year Picker",
        make,
        shown: Shown::of(shown),
        fill: false,
        forceable: true,
        facts: format!("value {}", value.map_or("None".to_owned(), |year| year.to_string())),
        event: "on_change",
        code,
        tree,
        controls: vec![flag("min / max", &self.ranged), flag("disabled", &self.disabled)],
        changed: [ranged, disabled].into_iter().filter(|changed| *changed).count(),
        reset: Arc::new(move || {
          to.0.set(Some(Date::today().year()));
          for flag in &to.1 {
            flag.set(false);
          }
        }),
      },
    )
  }
}

impl Component for YearPickerPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      value: ctx.signal(Some(Date::today().year())),
      ranged: ctx.signal(false),
      disabled: ctx.signal(false),
      memory: DateMemory::new(),
      year: ctx.signal(Date::today().year()),
      founded: ctx.signal(None),
      kept: std::array::from_fn(|_| DateMemory::new()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let this_year = Date::today().year();
    let bound = items()
      .child(item(
        YearPicker::new(self.year.clone()).memory(&self.kept[0]),
        &format!("the signal has: {}", self.year.get()),
      ))
      .child(item(
        YearPicker::optional(self.founded.clone())
          .min(1900)
          .max(this_year)
          .memory(&self.kept[1]),
        &format!(
          "the signal has: {} · from 1900 to this year",
          self.founded.get().map_or("None".to_owned(), |year| year.to_string())
        ),
      ));
    let rows = [
      (
        "new / optional",
        "Signal<i32> / Signal<Option<i32>>",
        "",
        "The year the picker holds, which a press sets; optional holds none until one is chosen.",
      ),
      (
        "controlled / controlled_optional",
        "i32 / Option<i32>, Fn",
        "",
        "The app's year and what it is told of a choice.",
      ),
      (
        "min / max",
        "i32",
        "None",
        "The least and the greatest year that can be chosen.",
      ),
      ("today", "i32", "system's", "The year marked as this year."),
      (
        "previous_label / next_label",
        "&str",
        "locale's",
        "What the buttons of the heading are called: Previous years, Next years.",
      ),
      (
        "locale, memory, label, invalid, disabled",
        "",
        "",
        "As a month picker's.",
      ),
      (
        "on_change",
        "Fn(i32) / Fn(Option<i32>)",
        "",
        "Called when a person chooses another year.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Inputs", "Year Picker"],
        title: "Year Picker",
        description: "Twelve years from the start of a decade, of which a person chooses one, and the decades before \
                      and after under the buttons of its heading: a year of birth, the year of a report.",
        import: Some("use lurq::ui::YearPicker;"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of YearPicker is a control on the right. Press a year, the chevrons, or Tab into the years and \
           use the arrows.",
          playground,
        ),
        Section::new(
          "Year picker",
          "A year bound to a signal, and one that may be none, from 1900 to this year.",
          self.frame.demo(
            0,
            bound,
            "YearPicker::new(year.clone())",
            &[
              "YearPicker::new(year.clone()).mount(ctx)",
              "YearPicker::optional(founded.clone()).min(1900).max(Date::today().year()).mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Keyboard",
          "The years are a grid; Page Up and Down go ten years.",
          grid_keys(),
        ),
        Section::new(
          "Props",
          "What a year picker is given.",
          props_table(&rows, &["new / optional"], self.frame.room()),
        ),
      ],
    )
  }
}
