//! The page of the select: its playground, a select bound to a signal, its
//! states and sizes, options that are disabled or have a detail, and a select
//! beside an input in a form.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row, SelectOption},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{Field, Input, Interaction, Select, SelectSize, SelectState},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet, segmented_fill},
  playground::{Playground, Spec, divider, field, flag, prop},
};

/// The themes the selects of the page choose from: the value, and its label.
const THEMES: [(&str, &str); 3] = [("system", "System"), ("light", "Light"), ("dark", "Dark")];

/// What the playground's value can be: no option's, which shows the placeholder, or one of the themes.
const VALUES: [(&str, &str); 4] = [("none", ""), ("system", "system"), ("light", "light"), ("dark", "dark")];

/// The sizes, as the board names them, and how the code names them.
const SIZES: [(SelectSize, &str, &str, f32); 3] = [
  (SelectSize::Sm, "sm", "SelectSize::Sm", 32.0),
  (SelectSize::Md, "md", "SelectSize::Md", 36.0),
  (SelectSize::Lg, "lg", "SelectSize::Lg", 40.0),
];

fn themes() -> Vec<(String, &'static str)> {
  THEMES
    .iter()
    .map(|(value, label)| ((*value).to_owned(), *label))
    .collect()
}

/// A select of the themes that holds `value`, bound to a signal of its own: for a stage that shows a state.
fn showing(value: &str) -> Select {
  Select::new(Signal::new(value.to_owned()), themes())
}

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .spacing(9.0)
    .child(Row::new().child(shown))
    .child(quiet(caption, 12.0))
}

/// What is on a stage, side by side; what does not fit goes on to the next line.
fn items() -> Row {
  Row::new()
    .width(FILL)
    .spacing(28.0)
    .wrap()
    .align_items(Alignment::Start)
}

pub struct SelectPage {
  frame: Frame,
  playground: Playground,
  value: Signal<String>,
  placeholder: Signal<String>,
  size: Signal<usize>,
  invalid: Signal<bool>,
  disabled: Signal<bool>,
  full_width: Signal<bool>,
  theme: Signal<String>,
  zone: Signal<String>,
  name: Signal<String>,
  channel: Signal<String>,
}

impl SelectPage {
  /// The playground: the select as the panel's props make it, bound to the page's signal.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let placeholder = self.placeholder.get();
    let (size, size_name, size_code, height) = SIZES[self.size.get().min(SIZES.len() - 1)];
    let flags = [
      ("invalid", self.invalid.get()),
      ("disabled", self.disabled.get()),
      ("full_width", self.full_width.get()),
    ];
    let on = |name: &str| flags.iter().any(|(flag, set)| *flag == name && *set);
    let make = {
      let (placeholder, value) = (placeholder.clone(), self.value.clone());
      let set: Vec<bool> = flags.iter().map(|(_, set)| *set).collect();
      move || {
        Select::new(value.clone(), themes())
          .placeholder(&placeholder)
          .size(size)
          .invalid(set[0])
          .disabled(set[1])
          .full_width(set[2])
      }
    };
    let shown = make().force(self.playground.forced());

    let mut code = vec![
      "// theme: Signal<String>".to_owned(),
      "Select::new(theme.clone(), [".to_owned(),
    ];
    for (value, label) in THEMES {
      code.push(format!("    ({value:?}.to_owned(), {label:?}),"));
    }
    code.push("])".to_owned());
    code.push(format!("    .placeholder({placeholder:?})"));
    if size != SelectSize::Md {
      code.push(format!("    .size({size_code})"));
    }
    for (name, set) in flags {
      if set {
        code.push(format!("    .{name}(true)"));
      }
    }

    let value = self.value.get();
    let label = THEMES.iter().find(|(held, _)| *held == value).map(|(_, label)| *label);
    let shows = match label {
      Some(label) => format!("value {label:?}"),
      None => format!("placeholder {placeholder:?}"),
    };
    let tree = vec![
      format!("trigger      {}", self.playground.measured()),
      format!("├─ value     {height} px high   {shows}"),
      "└─ chevron   16 × 16 px".to_owned(),
      "menu         4 under it   items of 32, the chosen one checked".to_owned(),
    ];

    let sizes: Vec<(&str, usize)> = SIZES.iter().enumerate().map(|(index, size)| (size.1, index)).collect();
    let (to_size, to_value) = (self.size.clone(), self.value.clone());
    let reset = {
      let (value, placeholder, size) = (self.value.clone(), self.placeholder.clone(), self.size.clone());
      let flags = [self.invalid.clone(), self.disabled.clone(), self.full_width.clone()];
      move || {
        value.set(String::new());
        placeholder.set("Select theme".to_owned());
        size.set(1);
        for flag in &flags {
          flag.set(false);
        }
      }
    };
    let changed = flags.iter().filter(|(_, set)| *set).count()
      + usize::from(size != SelectSize::Md)
      + usize::from(label.is_some())
      + usize::from(placeholder != "Select theme");
    self.playground.card(
      ctx,
      Spec {
        title: "Select",
        make: Arc::new(move || make().into()),
        shown: shown.into(),
        fill: on("full_width"),
        forceable: true,
        facts: format!("{size_name} · radius 6"),
        event: "value",
        code,
        tree,
        controls: vec![
          prop(
            "value",
            "Signal<T>",
            self.playground.select(
              0,
              &VALUES.map(|(name, value)| (name, value.to_owned())),
              &value,
              move |value| to_value.set(value),
            ),
          ),
          prop("placeholder", "&str", field(&self.placeholder)),
          prop(
            "size",
            "SelectSize",
            segmented_fill(&sizes, &self.size.get(), move |index| to_size.set(index)),
          ),
          divider(),
          flag("invalid", &self.invalid),
          flag("disabled", &self.disabled),
          flag("full_width", &self.full_width),
        ],
        changed,
        reset: Arc::new(reset),
      },
    )
  }
}

impl Component for SelectPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let playground = Playground::new(ctx, 1);
    let value = ctx.signal(String::new());
    let record = playground.recorder();
    ctx.watch(&value, move |value| record(format!("value = {value:?}")));
    Self {
      frame: Frame::new(ctx),
      playground,
      value,
      placeholder: ctx.signal("Select theme".to_owned()),
      size: ctx.signal(1),
      invalid: ctx.signal(false),
      disabled: ctx.signal(false),
      full_width: ctx.signal(false),
      theme: ctx.signal("system".to_owned()),
      zone: ctx.signal(String::new()),
      name: ctx.signal("Ivan Zaida".to_owned()),
      channel: ctx.signal("stable".to_owned()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let says = |value: &Signal<String>| format!("the signal has: {:?}", value.get());
    let live = items().child(item(Select::new(self.theme.clone(), themes()), &says(&self.theme)));
    let states = items()
      .child(item(showing("").placeholder("Select theme"), "default"))
      .child(item(showing("light"), "filled"))
      .child(item(showing("light").force(Some(Interaction::Hovered)), "hover"))
      .child(item(showing("light").force(Some(Interaction::FocusVisible)), "focus"))
      .child(item(showing("light").state(SelectState::Open, true), "open"))
      .child(item(showing("").placeholder("Select theme").invalid(true), "invalid"))
      .child(item(showing("").placeholder("Select theme").disabled(true), "disabled"));
    let mut sizes = items();
    for (size, name, _, height) in SIZES {
      sizes = sizes.child(item(showing("light").size(size), &format!("{name} · {height}")));
    }
    let zones = items().child(item(
      Select::new(
        self.zone.clone(),
        [
          SelectOption::new("Europe/Kyiv".to_owned(), "Kyiv").detail("UTC+2"),
          SelectOption::new("Europe/Lisbon".to_owned(), "Lisbon").detail("UTC+0"),
          SelectOption::new("America/New_York".to_owned(), "New York").detail("UTC−5"),
          SelectOption::new("Asia/Tokyo".to_owned(), "Tokyo")
            .detail("Not in your plan")
            .disabled(true),
        ],
      )
      .placeholder("Select a time zone")
      .invalid(self.zone.get().is_empty()),
      &says(&self.zone),
    ));
    let form = Column::new()
      .width(420.0)
      .spacing(16.0)
      .child(Field::new("Display name", Input::new(self.name.clone()).full_width(true)).horizontal(true))
      .child(
        Field::new(
          "Update channel",
          Select::new(
            self.channel.clone(),
            [
              ("stable".to_owned(), "Stable"),
              ("beta".to_owned(), "Beta"),
              ("nightly".to_owned(), "Nightly"),
            ],
          )
          .full_width(true),
        )
        .horizontal(true),
      );

    let rows = [
      (
        "value",
        "Signal<T>",
        "",
        "What the select is bound to: the option with its value is chosen, and choosing another sets it.",
      ),
      (
        "options",
        "impl IntoIterator<Item = impl Into<SelectOption<T>>>",
        "",
        "(value, label) pairs, or SelectOptions with a detail, an element beside the label, or disabled.",
      ),
      (
        "placeholder",
        "&str",
        "",
        "What it says while no option is chosen, in the muted foreground.",
      ),
      (
        "size",
        "SelectSize",
        "Md",
        "Sm 32, Md 36, Lg 40 high; text of 13, 14, 15, as an input's.",
      ),
      (
        "invalid",
        "bool",
        "false",
        "Its edge is in the destructive color, and so is its ring.",
      ),
      (
        "disabled",
        "bool",
        "false",
        "Half the opacity on the muted fill; it does not open, and Tab passes it by.",
      ),
      (
        "full_width",
        "bool",
        "false",
        "As wide as what holds it, in place of 260.",
      ),
      (
        "label",
        "&str",
        "",
        "What it is called, to tooling that reads the tree.",
      ),
      (
        "name",
        "impl Into<Arc<str>>",
        "",
        "The name of its value in the form it is in: the chosen option's label.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Inputs", "Select"],
        title: "Select",
        description: "One value chosen from a list that opens under a trigger shaped like an input: a theme, a time \
                      zone, a page size.",
        import: Some("use lurq::ui::Select;"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Select is a control on the right. The select on the stage is live: what is chosen in it goes \
           into the value on the right, and the other way.",
          playground,
        ),
        Section::new(
          "Select",
          "The page holds the value, in a signal. A press opens the list; so do Tab and then Enter, Space or an \
           arrow. The arrows and the first letters move through it, and Enter or a press chooses.",
          self.frame.demo(
            0,
            live,
            "Select::new(theme.clone(), [(\"system\".to_owned(), \"System\"), ..])",
            &[
              "// theme: Signal<String>",
              "Select::new(theme.clone(), [",
              "    (\"system\".to_owned(), \"System\"),",
              "    (\"light\".to_owned(), \"Light\"),",
              "    (\"dark\".to_owned(), \"Dark\"),",
              "])",
            ],
          ),
        ),
        Section::new(
          "States",
          "The edge says the state, as an input's does: the ring's color under the pointer, a line of 2 inside it \
           in the focus and while the list is open, the destructive color while it is invalid.",
          self.frame.demo(
            1,
            states,
            ".invalid(true)",
            &[
              "Select::new(theme, themes).placeholder(\"Select theme\")",
              "Select::new(theme, themes)",
              "",
              "// Drawn as it is under the pointer, focused or open, with nothing pointing at it: for a gallery, a test.",
              "Select::new(theme, themes).force(Some(Interaction::Hovered))",
              "Select::new(theme, themes).force(Some(Interaction::FocusVisible))",
              "Select::new(theme, themes).state(SelectState::Open, true)",
              "",
              "Select::new(theme, themes).placeholder(\"Select theme\").invalid(true)",
              "Select::new(theme, themes).placeholder(\"Select theme\").disabled(true)",
            ],
          ),
        ),
        Section::new(
          "Sizes",
          "The heights of the theme's controls with the text a step smaller or larger, as an input has them.",
          self.frame.demo(
            2,
            sizes,
            ".size(SelectSize::Sm)",
            &[
              "Select::new(theme, themes).size(SelectSize::Sm)",
              "Select::new(theme, themes)",
              "Select::new(theme, themes).size(SelectSize::Lg)",
            ],
          ),
        ),
        Section::new(
          "Options",
          "An option can have a second line under its label, and one that cannot be chosen is dimmed and passed by. \
           Nothing is chosen until the signal has an option's value; here that is invalid.",
          self.frame.demo(
            3,
            zones,
            "SelectOption::new(value, \"Kyiv\").detail(\"UTC+2\")",
            &[
              "// zone: Signal<String>",
              "Select::new(zone.clone(), [",
              "    SelectOption::new(\"Europe/Kyiv\".to_owned(), \"Kyiv\").detail(\"UTC+2\"),",
              "    SelectOption::new(\"Europe/Lisbon\".to_owned(), \"Lisbon\").detail(\"UTC+0\"),",
              "    SelectOption::new(\"America/New_York\".to_owned(), \"New York\").detail(\"UTC−5\"),",
              "    SelectOption::new(\"Asia/Tokyo\".to_owned(), \"Tokyo\")",
              "        .detail(\"Not in your plan\")",
              "        .disabled(true),",
              "])",
              ".placeholder(\"Select a time zone\")",
              ".invalid(zone.get().is_empty())",
            ],
          ),
        ),
        Section::new(
          "In a form",
          "A select stands beside an input: the same box, the same height, the same ring.",
          self.frame.demo(
            4,
            form,
            "Field::new(\"Update channel\", select).horizontal(true)",
            &[
              "// name, channel: Signal<String>",
              "Column::new()",
              "    .width(420.0)",
              "    .spacing(16.0)",
              "    .child(Field::new(\"Display name\", Input::new(name.clone()).full_width(true)).horizontal(true))",
              "    .child(",
              "        Field::new(",
              "            \"Update channel\",",
              "            Select::new(channel.clone(), [",
              "                (\"stable\".to_owned(), \"Stable\"),",
              "                (\"beta\".to_owned(), \"Beta\"),",
              "                (\"nightly\".to_owned(), \"Nightly\"),",
              "            ])",
              "            .full_width(true),",
              "        )",
              "        .horizontal(true),",
              "    )",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a select is given.",
          props_table(&rows, &["value", "options"], self.frame.room()),
        ),
      ],
    )
  }
}
