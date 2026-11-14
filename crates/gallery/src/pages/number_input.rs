//! The page of the number input: its playground, the input bound to a signal and to one that may be none, its
//! layouts, sizes, formats and states, one in a field, the board's mobile quantity stepper made by restyling it,
//! and what the keys do.

use std::sync::Arc;

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{PaletteColor, RadiusSize},
  },
  components::{Column, Row},
  core::Signal,
  layout::{Alignment, text_style::FontWeight},
  node::{Element, border::Border},
  ui::{
    Field, FontFamily, Interaction, NumberFormat, NumberInput, NumberInputLayout, NumberInputPart, NumberInputSize,
    Recipe,
  },
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, keys, props_table, quiet},
  playground::{Playground, Spec, divider, flag, prop},
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

/// The least values, the greatest, the steps and the precisions the playground offers.
const MINS: [(&str, Option<f64>); 4] = [("None", None), ("0", Some(0.0)), ("1", Some(1.0)), ("-10", Some(-10.0))];
const MAXES: [(&str, Option<f64>); 4] = [
  ("None", None),
  ("10", Some(10.0)),
  ("99", Some(99.0)),
  ("1000", Some(1000.0)),
];
const STEPS: [(&str, f64); 4] = [("1", 1.0), ("0.5", 0.5), ("0.25", 0.25), ("5", 5.0)];
const PRECISIONS: [(&str, Option<usize>); 4] = [("Step's", None), ("0", Some(0)), ("1", Some(1)), ("2", Some(2))];
const SIZES: [(&str, NumberInputSize); 3] = [
  ("Sm", NumberInputSize::Sm),
  ("Md", NumberInputSize::Md),
  ("Lg", NumberInputSize::Lg),
];
const LAYOUTS: [(&str, NumberInputLayout); 3] = [
  ("Split", NumberInputLayout::Split),
  ("Stacked", NumberInputLayout::Stacked),
  ("Bare", NumberInputLayout::Bare),
];

/// What each key does to a focused number input.
const KEYS: [(&str, &str); 7] = [
  (
    "Up",
    "One step up, from what is typed: a value between two steps goes to the next one.",
  ),
  ("Down", "One step down."),
  ("Page Up", "Ten steps up."),
  ("Page Down", "Ten steps down."),
  (
    "Home",
    "To the least value, where there is one; else the caret goes to the start.",
  ),
  (
    "End",
    "To the greatest value, where there is one; else the caret goes to the end.",
  ),
  (
    "Enter",
    "Puts what is typed right (rounded, in the range) and goes on to send the form.",
  ),
];

/// The board's mobile quantity stepper (Mobile/Quantity Stepper) as rules laid over the number input's: a pill of
/// 44 in the secondary fill with no edge, its buttons 44 square with glyphs of 20 and no lines, the number in the
/// interface's type at 16, semibold.
fn quantity_stepper() -> Recipe {
  use NumberInputPart::{Buttons, Control, Decrease, Divider, Icon, Increase};
  let edge = || Border::inside(1.0, PaletteColor::Secondary);
  Recipe::new()
    .base(Control, |s| {
      s.height(44.0)
        .radius(RadiusSize::Full)
        .background(PaletteColor::Secondary)
        .border(edge())
        .font_family(FontFamily::Sans)
        .font_size(16.0)
        .weight(FontWeight::SemiBold)
    })
    .hovered(Control, |s| s.border(edge()))
    .base(Buttons, |s| s.padding_x(0.0).padding_y(0.0).radius(RadiusSize::Full))
    .base(Decrease, |s| s.width(44.0))
    .base(Increase, |s| s.width(44.0))
    .base(Divider, |s| s.width(0.0))
    .base(Icon, |s| s.square(20.0))
}

/// A value as the page says it: none, or the number.
fn says(value: Option<f64>) -> String {
  match value {
    Some(value) => format!("the signal has: {value}"),
    None => "the signal has: None".to_owned(),
  }
}

pub struct NumberInputPage {
  frame: Frame,
  playground: Playground,
  value: Signal<f64>,
  min: Signal<Option<f64>>,
  max: Signal<Option<f64>>,
  step: Signal<f64>,
  precision: Signal<Option<usize>>,
  size: Signal<NumberInputSize>,
  layout: Signal<NumberInputLayout>,
  invalid: Signal<bool>,
  disabled: Signal<bool>,
  quantity: Signal<f64>,
  limit: Signal<Option<f64>>,
  price: Signal<f64>,
  weight: Signal<f64>,
  share: Signal<f64>,
  seats: Signal<f64>,
  stepper: Signal<f64>,
}

impl NumberInputPage {
  /// The playground: the input as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let value = self.value.get();
    let (min, max, step, precision) = (self.min.get(), self.max.get(), self.step.get(), self.precision.get());
    let (size, layout, invalid, disabled) = (
      self.size.get(),
      self.layout.get(),
      self.invalid.get(),
      self.disabled.get(),
    );
    let plain = {
      let bound = self.value.clone();
      move || {
        let mut input = NumberInput::new(bound.clone())
          .step(step)
          .size(size)
          .layout(layout)
          .invalid(invalid)
          .disabled(disabled)
          .label("Quantity");
        if let Some(min) = min {
          input = input.min(min);
        }
        if let Some(max) = max {
          input = input.max(max);
        }
        if let Some(precision) = precision {
          input = input.precision(precision);
        }
        input
      }
    };
    let input = plain().force(self.playground.forced());

    let mut code = vec![
      "// quantity: Signal<f64>".to_owned(),
      "NumberInput::new(quantity.clone())".to_owned(),
    ];
    if let Some(min) = min {
      code.push(format!("    .min({min:?})"));
    }
    if let Some(max) = max {
      code.push(format!("    .max({max:?})"));
    }
    if step != 1.0 {
      code.push(format!("    .step({step:?})"));
    }
    if let Some(precision) = precision {
      code.push(format!("    .precision({precision})"));
    }
    if size != NumberInputSize::Md {
      code.push(format!(
        "    .size(NumberInputSize::{})",
        SIZES.iter().find(|(_, s)| *s == size).unwrap().0
      ));
    }
    if layout != NumberInputLayout::Split {
      code.push(format!(
        "    .layout(NumberInputLayout::{})",
        LAYOUTS.iter().find(|(_, l)| *l == layout).unwrap().0
      ));
    }
    if invalid {
      code.push("    .invalid(true)".to_owned());
    }
    if disabled {
      code.push("    .disabled(true)".to_owned());
    }
    code.push("    .label(\"Quantity\")".to_owned());

    let mut tree = vec![format!("root          {}", self.playground.measured())];
    match layout {
      NumberInputLayout::Split => {
        tree.push("├─ control     the box, the number in its middle".to_owned());
        tree.push("└─ buttons".to_owned());
        tree.push("   ├─ decrease   minus".to_owned());
        tree.push("   ├─ divider".to_owned());
        tree.push("   ├─ divider".to_owned());
        tree.push("   └─ increase   plus".to_owned());
      }
      NumberInputLayout::Stacked => {
        tree.push("├─ control     the box, the number at its start".to_owned());
        tree.push("└─ buttons".to_owned());
        tree.push("   ├─ divider".to_owned());
        tree.push("   ├─ increase   chevron up".to_owned());
        tree.push("   └─ decrease   chevron down".to_owned());
      }
      NumberInputLayout::Bare => tree.push("└─ control     the box, the number at its start".to_owned()),
    }

    let to = (
      self.value.clone(),
      self.min.clone(),
      self.max.clone(),
      self.step.clone(),
      self.precision.clone(),
      self.size.clone(),
      self.layout.clone(),
      self.invalid.clone(),
      self.disabled.clone(),
    );
    let (set_min, set_max, set_step, set_precision, set_size, set_layout) = (
      self.min.clone(),
      self.max.clone(),
      self.step.clone(),
      self.precision.clone(),
      self.size.clone(),
      self.layout.clone(),
    );
    self.playground.card(
      ctx,
      Spec {
        title: "Number Input",
        make: Arc::new(move || plain().into()),
        shown: input.into(),
        fill: false,
        forceable: true,
        facts: format!("value {value}"),
        event: "value",
        code,
        tree,
        controls: vec![
          prop(
            "min",
            "f64",
            self.playground.select(0, &MINS, &min, move |min| set_min.set(min)),
          ),
          prop(
            "max",
            "f64",
            self.playground.select(1, &MAXES, &max, move |max| set_max.set(max)),
          ),
          prop(
            "step",
            "f64",
            self.playground.select(2, &STEPS, &step, move |step| set_step.set(step)),
          ),
          prop(
            "precision",
            "usize",
            self.playground.select(3, &PRECISIONS, &precision, move |precision| {
              set_precision.set(precision)
            }),
          ),
          prop(
            "size",
            "NumberInputSize",
            self.playground.select(4, &SIZES, &size, move |size| set_size.set(size)),
          ),
          prop(
            "layout",
            "NumberInputLayout",
            self
              .playground
              .select(5, &LAYOUTS, &layout, move |layout| set_layout.set(layout)),
          ),
          divider(),
          flag("invalid", &self.invalid),
          flag("disabled", &self.disabled),
        ],
        changed: [
          min.is_some(),
          max.is_some(),
          step != 1.0,
          precision.is_some(),
          size != NumberInputSize::Md,
          layout != NumberInputLayout::Split,
          invalid,
          disabled,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          to.0.set(12.0);
          to.1.set(None);
          to.2.set(None);
          to.3.set(1.0);
          to.4.set(None);
          to.5.set(NumberInputSize::Md);
          to.6.set(NumberInputLayout::Split);
          to.7.set(false);
          to.8.set(false);
        }),
      },
    )
  }
}

impl Component for NumberInputPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let playground = Playground::new(ctx, 6);
    let value = ctx.signal(12.0);
    let record = playground.recorder();
    ctx.watch(&value, move |value| record(format!("value = {value}")));
    Self {
      frame: Frame::new(ctx),
      playground,
      value,
      min: ctx.signal(None),
      max: ctx.signal(None),
      step: ctx.signal(1.0),
      precision: ctx.signal(None),
      size: ctx.signal(NumberInputSize::Md),
      layout: ctx.signal(NumberInputLayout::Split),
      invalid: ctx.signal(false),
      disabled: ctx.signal(false),
      quantity: ctx.signal(1.0),
      limit: ctx.signal(None),
      price: ctx.signal(1250.0),
      weight: ctx.signal(2.5),
      share: ctx.signal(0.25),
      seats: ctx.signal(3.0),
      stepper: ctx.signal(2.0),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let live = items()
      .child(item(
        NumberInput::new(self.quantity.clone())
          .min(1.0)
          .max(99.0)
          .label("Quantity"),
        &says(Some(self.quantity.get())),
      ))
      .child(item(
        NumberInput::optional(self.limit.clone())
          .min(0.0)
          .placeholder("No limit")
          .layout(NumberInputLayout::Stacked)
          .label("Limit"),
        &says(self.limit.get()),
      ));
    let layouts = items()
      .child(item(NumberInput::new(Signal::new(12.0)).label("Split"), "Split"))
      .child(item(
        NumberInput::new(Signal::new(12.0))
          .layout(NumberInputLayout::Stacked)
          .label("Stacked"),
        "Stacked",
      ))
      .child(item(
        NumberInput::new(Signal::new(12.0))
          .layout(NumberInputLayout::Bare)
          .label("Bare"),
        "Bare",
      ));
    let sizes = items()
      .child(item(
        NumberInput::new(Signal::new(12.0))
          .size(NumberInputSize::Sm)
          .label("Small"),
        "Sm · 32",
      ))
      .child(item(NumberInput::new(Signal::new(12.0)).label("Medium"), "Md · 36"))
      .child(item(
        NumberInput::new(Signal::new(12.0))
          .size(NumberInputSize::Lg)
          .label("Large"),
        "Lg · 40",
      ));
    let percent = NumberFormat::custom(
      |value| format!("{:.0}%", value * 100.0),
      |text| {
        text
          .trim()
          .trim_end_matches('%')
          .trim()
          .parse::<f64>()
          .ok()
          .map(|percent| percent / 100.0)
      },
    );
    let formats = items()
      .child(item(
        NumberInput::new(self.price.clone())
          .min(0.0)
          .step(0.5)
          .precision(2)
          .format(NumberFormat::new().prefix("$").grouping(true))
          .layout(NumberInputLayout::Stacked)
          .label("Price"),
        &says(Some(self.price.get())),
      ))
      .child(item(
        NumberInput::new(self.weight.clone())
          .min(0.0)
          .step(0.5)
          .format(NumberFormat::new().suffix(" kg"))
          .label("Weight"),
        &says(Some(self.weight.get())),
      ))
      .child(item(
        NumberInput::new(self.share.clone())
          .min(0.0)
          .max(1.0)
          .step(0.05)
          .format(percent)
          .label("Share"),
        &says(Some(self.share.get())),
      ));
    let states = items()
      .child(item(
        NumberInput::new(Signal::new(1.0))
          .min(1.0)
          .max(10.0)
          .label("At the least"),
        "at the least: minus disabled",
      ))
      .child(item(
        NumberInput::new(Signal::new(10.0))
          .min(1.0)
          .max(10.0)
          .label("At the greatest"),
        "at the greatest: plus disabled",
      ))
      .child(item(
        NumberInput::new(Signal::new(12.0))
          .force(Some(Interaction::FocusVisible))
          .label("Focused"),
        "focus-visible",
      ))
      .child(item(
        NumberInput::new(Signal::new(120.0)).invalid(true).label("Invalid"),
        "invalid",
      ))
      .child(item(
        NumberInput::new(Signal::new(12.0)).read_only(true).label("Read only"),
        "read only",
      ))
      .child(item(
        NumberInput::new(Signal::new(12.0)).disabled(true).label("Disabled"),
        "disabled",
      ));
    let field = items()
      .child(
        Field::new("Seats", NumberInput::new(self.seats.clone()).min(1.0).max(12.0))
          .description("From 1 to 12 at one table."),
      )
      .child(Field::new("Guests", NumberInput::new(Signal::new(14.0)).min(1.0)).error("No more than 12 at one table."));
    let stepper = items().child(item(
      NumberInput::new(self.stepper.clone())
        .min(0.0)
        .max(20.0)
        .rules(&quantity_stepper())
        .label("Quantity"),
      &says(Some(self.stepper.get())),
    ));
    let keyboard = KEYS
      .into_iter()
      .fold(Column::new().width(FILL).spacing(10.0), |list, (key, does)| {
        list.child(
          Row::new()
            .width(FILL)
            .align_items(Alignment::Center)
            .child(Row::new().width(140.0).child(keys(key)))
            .child(quiet(does, 13.0)),
        )
      });
    let rows = [
      (
        "value",
        "Signal<f64> / Signal<Option<f64>> / Option<f64>",
        "",
        "What the input is bound to: NumberInput::new for a number there always is, NumberInput::optional for one \
         that may be none (an emptied field), NumberInput::controlled for the app's own, told back by on_change.",
      ),
      (
        "min",
        "f64",
        "None",
        "The least value. Without one a minus can be typed, and Home moves the caret.",
      ),
      (
        "max",
        "f64",
        "None",
        "The greatest value. Without one End moves the caret.",
      ),
      (
        "step",
        "f64",
        "1",
        "How far apart the values a step takes it to are, from the least. Page Up and Page Down take ten.",
      ),
      (
        "precision",
        "usize",
        "the step's",
        "How many decimals are kept and shown: as many as the step has where it is not given. With none, no \
         decimal separator can be typed.",
      ),
      (
        "format",
        "NumberFormat",
        "the number",
        "A prefix, a suffix, grouping, a locale's separators, or the app's own formatter and parser. Shown out of \
         the focus; in it the number is typed as it is.",
      ),
      (
        "layout",
        "NumberInputLayout",
        "Split",
        "Split: a minus at the start and a plus at the end, the number in the middle. Stacked: a chevron up over \
         one down at the end. Bare: no buttons.",
      ),
      (
        "size",
        "NumberInputSize",
        "Md",
        "32, 36 or 40 high, the type 13, 14 or 15.",
      ),
      ("placeholder", "&str", "", "What an empty input says."),
      (
        "invalid / warning / valid",
        "bool",
        "false",
        "The judgment of what it holds: the edge in the destructive, warning or success color.",
      ),
      (
        "read_only",
        "bool",
        "false",
        "Read and copied, not changed: typing, keys and buttons do nothing.",
      ),
      (
        "disabled",
        "bool",
        "false",
        "Half the opacity; it takes no input, and Tab passes it by.",
      ),
      (
        "label",
        "&str",
        "",
        "What it is called to tooling that reads the tree. A field gives it its label.",
      ),
      (
        "on_change",
        "Fn(Option<f64>)",
        "",
        "Called with the value each time it changes.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Inputs", "Number Input"],
        title: "Number Input",
        description: "A number a person types or steps: a quantity, a price, a size in pixels.",
        import: Some("use lurq::ui::NumberInput;"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of NumberInput is a control on the right. The input on the stage is live: type in it, press \
           or hold its buttons, or Tab to it and use the arrow keys, Page Up and Page Down, Home and End.",
          playground,
        ),
        Section::new(
          "Number input",
          "The page holds each value in a signal the input is bound to. What is typed is the value as soon as it \
           is a number; on Enter and when the focus leaves, it is rounded and kept in the range. The second may be \
           none: emptied, its signal holds None, not nought.",
          self.frame.demo(
            0,
            live,
            "NumberInput::new(quantity.clone())",
            &[
              "// quantity: Signal<f64>, limit: Signal<Option<f64>>",
              "NumberInput::new(quantity.clone())",
              "    .min(1.0)",
              "    .max(99.0)",
              "    .label(\"Quantity\")",
              "",
              "NumberInput::optional(limit.clone())",
              "    .min(0.0)",
              "    .placeholder(\"No limit\")",
              "    .layout(NumberInputLayout::Stacked)",
              "    .label(\"Limit\")",
            ],
          ),
        ),
        Section::new(
          "Layouts",
          "Split is the board's: a minus and a plus at the ends, each with a line between it and the number. \
           Stacked puts a chevron up over one down at the end; Bare has no buttons, and the keys step it.",
          self.frame.demo(
            1,
            layouts,
            ".layout(NumberInputLayout::Stacked)",
            &[
              "NumberInput::new(value.clone()).label(\"Split\")",
              "",
              "NumberInput::new(value.clone())",
              "    .layout(NumberInputLayout::Stacked)",
              "    .label(\"Stacked\")",
              "",
              "NumberInput::new(value.clone())",
              "    .layout(NumberInputLayout::Bare)",
              "    .label(\"Bare\")",
            ],
          ),
        ),
        Section::new(
          "Sizes",
          "The heights of the theme's controls, the buttons as wide as the box is high.",
          self.frame.demo(
            2,
            sizes,
            ".size(NumberInputSize::Sm)",
            &[
              "NumberInput::new(value.clone()).size(NumberInputSize::Sm).label(\"Small\")",
              "NumberInput::new(value.clone()).label(\"Medium\")",
              "NumberInput::new(value.clone()).size(NumberInputSize::Lg).label(\"Large\")",
            ],
          ),
        ),
        Section::new(
          "Format",
          "A prefix and grouping, a unit after the number, and the app's own formatter and parser for a share \
           shown as a percentage. In the focus the number is typed as it is; out of it, it is written.",
          self.frame.demo(
            3,
            formats,
            ".format(NumberFormat::new().prefix(\"$\"))",
            &[
              "// price, weight, share: Signal<f64>",
              "NumberInput::new(price.clone())",
              "    .min(0.0)",
              "    .step(0.5)",
              "    .precision(2)",
              "    .format(NumberFormat::new().prefix(\"$\").grouping(true))",
              "    .layout(NumberInputLayout::Stacked)",
              "    .label(\"Price\")",
              "",
              "NumberInput::new(weight.clone())",
              "    .min(0.0)",
              "    .step(0.5)",
              "    .format(NumberFormat::new().suffix(\" kg\"))",
              "    .label(\"Weight\")",
              "",
              "NumberInput::new(share.clone())",
              "    .min(0.0)",
              "    .max(1.0)",
              "    .step(0.05)",
              "    .format(NumberFormat::custom(",
              "        |value| format!(\"{:.0}%\", value * 100.0),",
              "        |text| {",
              "            text.trim().trim_end_matches('%').trim()",
              "                .parse::<f64>().ok().map(|percent| percent / 100.0)",
              "        },",
              "    ))",
              "    .label(\"Share\")",
            ],
          ),
        ),
        Section::new(
          "States",
          "At an end of its range the button toward it is dimmed and takes nothing. The ring of the focus is the \
           box's; a judgment colors its edge, and a disabled one is dimmed whole.",
          self.frame.demo(
            4,
            states,
            ".invalid(true)",
            &[
              "NumberInput::new(value.clone()).min(1.0).max(10.0)",
              "",
              "// Drawn focused with nothing focused: for a gallery, a test.",
              "NumberInput::new(value.clone()).force(Some(Interaction::FocusVisible))",
              "",
              "NumberInput::new(value.clone()).invalid(true)",
              "NumberInput::new(value.clone()).read_only(true)",
              "NumberInput::new(value.clone()).disabled(true)",
            ],
          ),
        ),
        Section::new(
          "In a field",
          "A field names it and judges it: the input is called by the field's label and is invalid while the \
           field has an error. In a form, bind it to the form's number: NumberInput::new(form.number(\"seats\")).",
          self.frame.demo(
            5,
            field,
            "Field::new(\"Seats\", NumberInput::new(seats.clone()))",
            &[
              "// seats: Signal<f64>",
              "Field::new(\"Seats\", NumberInput::new(seats.clone()).min(1.0).max(12.0))",
              "    .description(\"From 1 to 12 at one table.\")",
              "",
              "Field::new(\"Guests\", NumberInput::new(guests.clone()).min(1.0))",
              "    .error(\"No more than 12 at one table.\")",
            ],
          ),
        ),
        Section::new(
          "Mobile quantity stepper",
          "The board's stepper for a touch screen is the number input restyled: rules laid over its parts make it \
           a pill of 44 in the secondary fill, its buttons 44 square without lines, the number semibold.",
          self.frame.demo(
            6,
            stepper,
            ".rules(&quantity_stepper())",
            &[
              "fn quantity_stepper() -> Recipe {",
              "    use NumberInputPart::{Buttons, Control, Decrease, Divider, Icon, Increase};",
              "    let edge = || Border::inside(1.0, PaletteColor::Secondary);",
              "    Recipe::new()",
              "        .base(Control, |s| {",
              "            s.height(44.0)",
              "                .radius(RadiusSize::Full)",
              "                .background(PaletteColor::Secondary)",
              "                .border(edge())",
              "                .font_family(FontFamily::Sans)",
              "                .font_size(16.0)",
              "                .weight(FontWeight::SemiBold)",
              "        })",
              "        .hovered(Control, |s| s.border(edge()))",
              "        .base(Buttons, |s| s.padding_x(0.0).padding_y(0.0).radius(RadiusSize::Full))",
              "        .base(Decrease, |s| s.width(44.0))",
              "        .base(Increase, |s| s.width(44.0))",
              "        .base(Divider, |s| s.width(0.0))",
              "        .base(Icon, |s| s.square(20.0))",
              "}",
              "",
              "// stepper: Signal<f64>",
              "NumberInput::new(stepper.clone())",
              "    .min(0.0)",
              "    .max(20.0)",
              "    .rules(&quantity_stepper())",
              "    .label(\"Quantity\")",
            ],
          ),
        ),
        Section::new(
          "Keyboard",
          "What the keys do while the input has the focus. Its buttons are not stops of Tab; held, they step on \
           every twentieth of a second. A wheel turned over it scrolls the page and leaves the number as it is.",
          keyboard,
        ),
        Section::new(
          "Props",
          "What a number input is given.",
          props_table(&rows, &["value"], self.frame.room()),
        ),
      ],
    )
  }
}
