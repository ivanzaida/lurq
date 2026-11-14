//! The page of the multi select: its playground, a multi select bound to a signal, its states and sizes, the count
//! past a maximum and tags that go on to the next line, the clear control, and a multi select in a form.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row, SelectOption},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{ComboboxSize, ComboboxState, Field, Input, Interaction, MultiSelect, MultiSelectPart},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet, segmented_fill},
  playground::{Playground, Spec, divider, field, flag, prop},
};

/// The languages the multi selects of the page choose from: the value, and its label.
const LANGUAGES: [(&str, &str); 7] = [
  ("rust", "Rust"),
  ("wgsl", "WGSL"),
  ("typescript", "TypeScript"),
  ("swift", "Swift"),
  ("kotlin", "Kotlin"),
  ("go", "Go"),
  ("zig", "Zig"),
];

/// What the playground's value can be: nothing chosen, the board's two, or four.
const VALUES: [(&str, &[&str]); 3] = [
  ("none", &[]),
  ("Rust, WGSL", &["rust", "wgsl"]),
  ("four", &["rust", "wgsl", "typescript", "swift"]),
];

/// The sizes, as the board names them, and how the code names them.
const SIZES: [(ComboboxSize, &str, &str, f32); 3] = [
  (ComboboxSize::Sm, "sm", "ComboboxSize::Sm", 32.0),
  (ComboboxSize::Md, "md", "ComboboxSize::Md", 36.0),
  (ComboboxSize::Lg, "lg", "ComboboxSize::Lg", 40.0),
];

/// How many tags the playground's box shows at most: all, or a number.
const MOST: [(&str, Option<usize>); 4] = [("all", None), ("1", Some(1)), ("2", Some(2)), ("3", Some(3))];

fn languages() -> Vec<(String, &'static str)> {
  LANGUAGES
    .iter()
    .map(|(value, label)| ((*value).to_owned(), *label))
    .collect()
}

fn owned(values: &[&str]) -> Vec<String> {
  values.iter().map(|value| (*value).to_owned()).collect()
}

/// A multi select of the languages that holds `values`, bound to a signal of its own: for a stage that shows a
/// state.
fn showing(values: &[&str]) -> MultiSelect {
  MultiSelect::new(Signal::new(owned(values)), languages()).placeholder("Select languages")
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

pub struct MultiSelectPage {
  frame: Frame,
  playground: Playground,
  value: Signal<Vec<String>>,
  placeholder: Signal<String>,
  size: Signal<usize>,
  most: Signal<usize>,
  invalid: Signal<bool>,
  warning: Signal<bool>,
  valid: Signal<bool>,
  disabled: Signal<bool>,
  full_width: Signal<bool>,
  clearable: Signal<bool>,
  loading: Signal<bool>,
  languages: Signal<Vec<String>>,
  reviewers: Signal<Vec<String>>,
  name: Signal<String>,
  topics: Signal<Vec<String>>,
}

impl MultiSelectPage {
  /// The playground: the multi select as the panel's props make it, bound to the page's signal.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let placeholder = self.placeholder.get();
    let (size, size_name, size_code, height) = SIZES[self.size.get().min(SIZES.len() - 1)];
    let (most_name, most) = MOST[self.most.get().min(MOST.len() - 1)];
    let flags = [
      ("invalid", self.invalid.get()),
      ("warning", self.warning.get()),
      ("valid", self.valid.get()),
      ("disabled", self.disabled.get()),
      ("full_width", self.full_width.get()),
      ("clearable", self.clearable.get()),
      ("loading", self.loading.get()),
    ];
    let on = |name: &str| flags.iter().any(|(flag, set)| *flag == name && *set);
    let make = {
      let (placeholder, value) = (placeholder.clone(), self.value.clone());
      let set: Vec<bool> = flags.iter().map(|(_, set)| *set).collect();
      move || {
        let mut select = MultiSelect::new(value.clone(), languages())
          .placeholder(&placeholder)
          .size(size)
          .invalid(set[0])
          .warning(set[1])
          .valid(set[2])
          .disabled(set[3])
          .full_width(set[4])
          .clearable(set[5])
          .loading(set[6]);
        if let Some(most) = most {
          select = select.max_shown(most);
        }
        select
      }
    };
    let shown = make().force(self.playground.forced());

    let mut code = vec![
      "// languages: Signal<Vec<String>>".to_owned(),
      "MultiSelect::new(languages.clone(), [".to_owned(),
    ];
    for (value, label) in LANGUAGES {
      code.push(format!("    ({value:?}.to_owned(), {label:?}),"));
    }
    code.push("])".to_owned());
    code.push(format!("    .placeholder({placeholder:?})"));
    if size != ComboboxSize::Md {
      code.push(format!("    .size({size_code})"));
    }
    if let Some(most) = most {
      code.push(format!("    .max_shown({most})"));
    }
    for (name, set) in flags {
      if set {
        code.push(format!("    .{name}(true)"));
      }
    }

    let value = self.value.get();
    let tree = vec![
      format!(
        "box          {}   at least {height} px high",
        self.playground.measured()
      ),
      format!("├─ tags      {} chosen, {most_name} shown", value.len()),
      "├─ input     after the tags".to_owned(),
      "└─ chevron   16 × 16 px".to_owned(),
      "list         4 under it   a box before each option, checked where chosen".to_owned(),
    ];

    let sizes: Vec<(&str, usize)> = SIZES.iter().enumerate().map(|(index, size)| (size.1, index)).collect();
    let mosts: Vec<(&str, usize)> = MOST.iter().enumerate().map(|(index, most)| (most.0, index)).collect();
    let (to_size, to_most, to_value) = (self.size.clone(), self.most.clone(), self.value.clone());
    let reset = {
      let (value, placeholder, size, most) = (
        self.value.clone(),
        self.placeholder.clone(),
        self.size.clone(),
        self.most.clone(),
      );
      let flags = [
        self.invalid.clone(),
        self.warning.clone(),
        self.valid.clone(),
        self.disabled.clone(),
        self.full_width.clone(),
        self.clearable.clone(),
        self.loading.clone(),
      ];
      move || {
        value.set(owned(&["rust", "wgsl"]));
        placeholder.set("Select languages".to_owned());
        size.set(1);
        most.set(0);
        for flag in &flags {
          flag.set(false);
        }
      }
    };
    let changed = flags.iter().filter(|(_, set)| *set).count()
      + usize::from(size != ComboboxSize::Md)
      + usize::from(most.is_some())
      + usize::from(value != owned(&["rust", "wgsl"]))
      + usize::from(placeholder != "Select languages");
    let preset = VALUES
      .iter()
      .position(|(_, values)| owned(values) == value)
      .unwrap_or(VALUES.len());
    let presets: Vec<(&str, usize)> = VALUES
      .iter()
      .enumerate()
      .map(|(index, (name, _))| (*name, index))
      .collect();
    self.playground.card(
      ctx,
      Spec {
        title: "Multi Select",
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
            "Signal<Vec<T>>",
            self
              .playground
              .select(0, &presets, &preset, move |index| to_value.set(owned(VALUES[index].1))),
          ),
          prop("placeholder", "&str", field(&self.placeholder)),
          prop(
            "size",
            "ComboboxSize",
            segmented_fill(&sizes, &self.size.get(), move |index| to_size.set(index)),
          ),
          prop(
            "max_shown",
            "usize",
            segmented_fill(&mosts, &self.most.get(), move |index| to_most.set(index)),
          ),
          divider(),
          flag("invalid", &self.invalid),
          flag("warning", &self.warning),
          flag("valid", &self.valid),
          flag("disabled", &self.disabled),
          flag("full_width", &self.full_width),
          flag("clearable", &self.clearable),
          flag("loading", &self.loading),
        ],
        changed,
        reset: Arc::new(reset),
      },
    )
  }
}

impl Component for MultiSelectPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let playground = Playground::new(ctx, 1);
    let value = ctx.signal(owned(&["rust", "wgsl"]));
    let record = playground.recorder();
    ctx.watch(&value, move |value: &Vec<String>| record(format!("value = {value:?}")));
    Self {
      frame: Frame::new(ctx),
      playground,
      value,
      placeholder: ctx.signal("Select languages".to_owned()),
      size: ctx.signal(1),
      most: ctx.signal(0),
      invalid: ctx.signal(false),
      warning: ctx.signal(false),
      valid: ctx.signal(false),
      disabled: ctx.signal(false),
      full_width: ctx.signal(false),
      clearable: ctx.signal(false),
      loading: ctx.signal(false),
      languages: ctx.signal(owned(&["rust", "wgsl"])),
      reviewers: ctx.signal(owned(&["ivan", "ivy", "ivo", "mara"])),
      name: ctx.signal("Ivan Zaida".to_owned()),
      topics: ctx.signal(Vec::new()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let says = |value: &Signal<Vec<String>>| format!("the signal has: {:?}", value.get());
    let live = items().child(item(
      MultiSelect::new(self.languages.clone(), languages()).placeholder("Select languages"),
      &says(&self.languages),
    ));
    let states = items()
      .child(item(showing(&[]), "default"))
      .child(item(showing(&["rust", "wgsl"]), "filled"))
      .child(item(showing(&["rust"]).force(Some(Interaction::Hovered)), "hover"))
      .child(item(showing(&["rust"]).force(Some(Interaction::FocusVisible)), "focus"))
      .child(item(showing(&["rust"]).state(ComboboxState::Open, true), "open"))
      .child(item(showing(&[]).invalid(true), "invalid"))
      .child(item(showing(&["go"]).warning(true), "warning"))
      .child(item(showing(&["rust"]).valid(true), "valid"))
      .child(item(showing(&["rust", "wgsl"]).disabled(true), "disabled"));
    let mut sizes = items();
    for (size, name, _, height) in SIZES {
      sizes = sizes.child(item(
        showing(&["rust", "wgsl"]).size(size),
        &format!("{name} · at least {height}"),
      ));
    }
    let people = [
      SelectOption::new("ivan".to_owned(), "Ivan Zaida").detail("Maintainer"),
      SelectOption::new("ivy".to_owned(), "Ivy Chan"),
      SelectOption::new("ivo".to_owned(), "Ivo Kovac"),
      SelectOption::new("mara".to_owned(), "Mara Lind"),
      SelectOption::new("tom".to_owned(), "Tom Berg").disabled(true),
    ];
    let most = items()
      .child(item(
        MultiSelect::new(self.reviewers.clone(), people.clone())
          .placeholder("Add reviewers")
          .max_shown(2),
        &format!("max_shown(2): {}", says(&self.reviewers)),
      ))
      .child(item(
        MultiSelect::new(self.reviewers.clone(), people).style(MultiSelectPart::Trigger, |s| s.width(240.0)),
        "every tag, in a box of 240: they go on to the next line",
      ));
    let clearable = items()
      .child(item(
        MultiSelect::new(self.languages.clone(), languages()).clearable(true),
        "clearable",
      ))
      .child(item(showing(&[]).loading(true), "loading"));
    let form = Column::new()
      .width(440.0)
      .spacing(16.0)
      .child(Field::new("Display name", Input::new(self.name.clone()).full_width(true)).horizontal(true))
      .child(
        Field::new(
          "Topics",
          MultiSelect::new(
            self.topics.clone(),
            [
              ("gpu".to_owned(), "gpu"),
              ("ui".to_owned(), "ui"),
              ("text".to_owned(), "text"),
              ("layout".to_owned(), "layout"),
            ],
          )
          .placeholder("Select topics")
          .full_width(true),
        )
        .horizontal(true)
        .invalid(self.topics.get().is_empty()),
      );

    let rows = [
      (
        "value",
        "Signal<Vec<T>>",
        "",
        "What the multi select is bound to: the options with its values are chosen, in that order as tags; \
         choosing one adds it, or takes it out.",
      ),
      (
        "options",
        "impl IntoIterator<Item = impl Into<SelectOption<T>>>",
        "",
        "(value, label) pairs, or SelectOptions: a select's.",
      ),
      (
        "placeholder",
        "&str",
        "",
        "What the box says while nothing is chosen and nothing is typed.",
      ),
      (
        "max_shown",
        "usize",
        "",
        "How many tags the box shows: the rest are one tag that counts them, \"+3\".",
      ),
      (
        "clearable",
        "bool",
        "false",
        "A cross at the end of the box that takes every value away.",
      ),
      (
        "filter",
        "Fn(&str, &str) -> bool",
        "contains",
        "Which options are shown while something is typed: (label, typed).",
      ),
      (
        "on_input",
        "Fn(&str)",
        "",
        "Called with what the box will hold at each edit.",
      ),
      (
        "loading / empty",
        "bool / &str",
        "false / \"No results\"",
        "The row at the end of the list: a spinner while more are loading, or what it says while there are none.",
      ),
      (
        "size",
        "ComboboxSize",
        "Md",
        "Sm 32, Md 36, Lg 40 high at least, with tags of 20, 24, 28.",
      ),
      (
        "invalid / warning / valid",
        "bool",
        "false",
        "Its edge and its ring in the destructive, warning or success color.",
      ),
      (
        "disabled",
        "bool",
        "false",
        "Half the opacity on the muted fill; it does not open, and its tags stay.",
      ),
      (
        "full_width",
        "bool",
        "false",
        "As wide as what holds it, in place of 280.",
      ),
      (
        "label / name",
        "&str / impl Into<Arc<str>>",
        "",
        "What it is called to tooling; the name of its value in a form.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Inputs", "Multi Select"],
        title: "Multi Select",
        description: "Several values chosen from a list that what is typed filters, held as tags in the box: the \
                      labels of an issue, the people a review goes to.",
        import: Some("use lurq::ui::MultiSelect;"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of MultiSelect is a control on the right. The multi select on the stage is live: what is \
           chosen in it goes into the value on the right, and the other way.",
          playground,
        ),
        Section::new(
          "Multi Select",
          "The page holds the values, in a signal. Typing filters the list; Enter or a press chooses an option or \
           takes it back, and the list stays open. A tag's cross takes its value away, and so does Backspace in the \
           empty input, the last one's.",
          self.frame.demo(
            0,
            live,
            "MultiSelect::new(languages.clone(), [(\"rust\".to_owned(), \"Rust\"), ..])",
            &[
              "// languages: Signal<Vec<String>>",
              "MultiSelect::new(languages.clone(), [",
              "    (\"rust\".to_owned(), \"Rust\"),",
              "    (\"wgsl\".to_owned(), \"WGSL\"),",
              "    (\"typescript\".to_owned(), \"TypeScript\"),",
              "    (\"swift\".to_owned(), \"Swift\"),",
              "    (\"kotlin\".to_owned(), \"Kotlin\"),",
              "    (\"go\".to_owned(), \"Go\"),",
              "    (\"zig\".to_owned(), \"Zig\"),",
              "])",
              ".placeholder(\"Select languages\")",
            ],
          ),
        ),
        Section::new(
          "States",
          "The box is a select's, and its edge says the state, as a combobox's does. A disabled one keeps its tags \
           and takes none away.",
          self.frame.demo(
            1,
            states,
            ".invalid(true)",
            &[
              "MultiSelect::new(languages, options).placeholder(\"Select languages\")",
              "MultiSelect::new(languages, options)",
              "",
              "// Drawn as it is under the pointer, focused or open, with nothing pointing at it.",
              "MultiSelect::new(languages, options).force(Some(Interaction::Hovered))",
              "MultiSelect::new(languages, options).force(Some(Interaction::FocusVisible))",
              "MultiSelect::new(languages, options).state(ComboboxState::Open, true)",
              "",
              "MultiSelect::new(languages, options).invalid(true)",
              "MultiSelect::new(languages, options).warning(true)",
              "MultiSelect::new(languages, options).valid(true)",
              "MultiSelect::new(languages, options).disabled(true)",
            ],
          ),
        ),
        Section::new(
          "Sizes",
          "The least heights of the theme's controls, with tags of 20, 24 and 28 in them.",
          self.frame.demo(
            2,
            sizes,
            ".size(ComboboxSize::Sm)",
            &[
              "MultiSelect::new(languages, options).size(ComboboxSize::Sm)",
              "MultiSelect::new(languages, options)",
              "MultiSelect::new(languages, options).size(ComboboxSize::Lg)",
            ],
          ),
        ),
        Section::new(
          "Many chosen",
          "Past max_shown tags the rest are one tag that counts them. Without it every tag is shown, and they go on \
           to the next line where they do not fit: the box grows with them.",
          self.frame.demo(
            3,
            most,
            ".max_shown(2)",
            &[
              "// reviewers: Signal<Vec<String>>",
              "let people = [",
              "    SelectOption::new(\"ivan\".to_owned(), \"Ivan Zaida\").detail(\"Maintainer\"),",
              "    SelectOption::new(\"ivy\".to_owned(), \"Ivy Chan\"),",
              "    SelectOption::new(\"ivo\".to_owned(), \"Ivo Kovac\"),",
              "    SelectOption::new(\"mara\".to_owned(), \"Mara Lind\"),",
              "    SelectOption::new(\"tom\".to_owned(), \"Tom Berg\").disabled(true),",
              "];",
              "MultiSelect::new(reviewers.clone(), people.clone())",
              "    .placeholder(\"Add reviewers\")",
              "    .max_shown(2)",
              "",
              "MultiSelect::new(reviewers.clone(), people)",
              "    .style(MultiSelectPart::Trigger, |s| s.width(240.0))",
            ],
          ),
        ),
        Section::new(
          "Clear and loading",
          "A clearable box has a cross at its end while something is chosen, which takes every value away. While \
           more options are loading the list ends in a row with a spinner.",
          self.frame.demo(
            4,
            clearable,
            ".clearable(true)",
            &[
              "MultiSelect::new(languages.clone(), options).clearable(true)",
              "",
              "MultiSelect::new(languages, options).placeholder(\"Select languages\").loading(true)",
            ],
          ),
        ),
        Section::new(
          "In a form",
          "A multi select stands beside an input: the same box at least, the same ring. The field names it and \
           judges it: here, nothing chosen is invalid.",
          self.frame.demo(
            5,
            form,
            "Field::new(\"Topics\", multi_select).horizontal(true)",
            &[
              "// name: Signal<String>, topics: Signal<Vec<String>>",
              "Column::new()",
              "    .width(440.0)",
              "    .spacing(16.0)",
              "    .child(Field::new(\"Display name\", Input::new(name.clone()).full_width(true)).horizontal(true))",
              "    .child(",
              "        Field::new(",
              "            \"Topics\",",
              "            MultiSelect::new(topics.clone(), [",
              "                (\"gpu\".to_owned(), \"gpu\"),",
              "                (\"ui\".to_owned(), \"ui\"),",
              "                (\"text\".to_owned(), \"text\"),",
              "                (\"layout\".to_owned(), \"layout\"),",
              "            ])",
              "            .placeholder(\"Select topics\")",
              "            .full_width(true),",
              "        )",
              "        .horizontal(true)",
              "        .invalid(topics.get().is_empty()),",
              "    )",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a multi select is given.",
          props_table(&rows, &["value", "options"], self.frame.room()),
        ),
      ],
    )
  }
}
