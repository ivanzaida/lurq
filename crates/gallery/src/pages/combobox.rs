//! The page of the combobox: its playground, a combobox bound to a signal, its states and sizes, what an app filters
//! and fetches, custom values, groups and disabled options, and a combobox in a form.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row, SelectOption},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{Combobox, ComboboxSize, ComboboxState, Field, Input, Interaction},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet, segmented_fill},
  playground::{Playground, Spec, divider, field, flag, prop},
};

/// The frameworks the comboboxes of the page choose from: the value, and its label.
const FRAMEWORKS: [(&str, &str); 6] = [
  ("lurq", "lurq"),
  ("gpui", "GPUI"),
  ("slint", "Slint"),
  ("iced", "Iced"),
  ("egui", "egui"),
  ("dioxus", "Dioxus"),
];

/// What the playground's value can be: no option's, which shows the placeholder, or one of the frameworks.
const VALUES: [(&str, &str); 4] = [("none", ""), ("lurq", "lurq"), ("gpui", "gpui"), ("slint", "slint")];

/// The sizes, as the board names them, and how the code names them.
const SIZES: [(ComboboxSize, &str, &str, f32); 3] = [
  (ComboboxSize::Sm, "sm", "ComboboxSize::Sm", 32.0),
  (ComboboxSize::Md, "md", "ComboboxSize::Md", 36.0),
  (ComboboxSize::Lg, "lg", "ComboboxSize::Lg", 40.0),
];

/// The countries the page fetches from, as an app would ask a server for them.
const COUNTRIES: [&str; 12] = [
  "Argentina",
  "Austria",
  "Belgium",
  "Brazil",
  "Canada",
  "Denmark",
  "Estonia",
  "Finland",
  "Portugal",
  "Spain",
  "Sweden",
  "Ukraine",
];

fn frameworks() -> Vec<(String, &'static str)> {
  FRAMEWORKS
    .iter()
    .map(|(value, label)| ((*value).to_owned(), *label))
    .collect()
}

/// A combobox of the frameworks that holds `value`, bound to a signal of its own: for a stage that shows a state.
fn showing(value: &str) -> Combobox {
  Combobox::new(Signal::new(value.to_owned()), frameworks()).placeholder("Select framework")
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

pub struct ComboboxPage {
  frame: Frame,
  playground: Playground,
  value: Signal<String>,
  placeholder: Signal<String>,
  empty: Signal<String>,
  size: Signal<usize>,
  invalid: Signal<bool>,
  warning: Signal<bool>,
  valid: Signal<bool>,
  disabled: Signal<bool>,
  full_width: Signal<bool>,
  loading: Signal<bool>,
  allow_custom: Signal<bool>,
  framework: Signal<String>,
  country: Signal<String>,
  fetched: Signal<Vec<String>>,
  tag: Signal<String>,
  crate_name: Signal<String>,
  name: Signal<String>,
  editor: Signal<String>,
}

impl ComboboxPage {
  /// The playground: the combobox as the panel's props make it, bound to the page's signal.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (placeholder, empty) = (self.placeholder.get(), self.empty.get());
    let (size, size_name, size_code, height) = SIZES[self.size.get().min(SIZES.len() - 1)];
    let flags = [
      ("invalid", self.invalid.get()),
      ("warning", self.warning.get()),
      ("valid", self.valid.get()),
      ("disabled", self.disabled.get()),
      ("full_width", self.full_width.get()),
      ("loading", self.loading.get()),
      ("allow_custom", self.allow_custom.get()),
    ];
    let on = |name: &str| flags.iter().any(|(flag, set)| *flag == name && *set);
    let make = {
      let (placeholder, empty, value) = (placeholder.clone(), empty.clone(), self.value.clone());
      let set: Vec<bool> = flags.iter().map(|(_, set)| *set).collect();
      move || {
        Combobox::new(value.clone(), frameworks())
          .placeholder(&placeholder)
          .empty(&empty)
          .size(size)
          .invalid(set[0])
          .warning(set[1])
          .valid(set[2])
          .disabled(set[3])
          .full_width(set[4])
          .loading(set[5])
          .allow_custom(set[6])
      }
    };
    let shown = make().force(self.playground.forced());

    let mut code = vec![
      "// framework: Signal<String>".to_owned(),
      "Combobox::new(framework.clone(), [".to_owned(),
    ];
    for (value, label) in FRAMEWORKS {
      code.push(format!("    ({value:?}.to_owned(), {label:?}),"));
    }
    code.push("])".to_owned());
    code.push(format!("    .placeholder({placeholder:?})"));
    if empty != "No results" {
      code.push(format!("    .empty({empty:?})"));
    }
    if size != ComboboxSize::Md {
      code.push(format!("    .size({size_code})"));
    }
    for (name, set) in flags {
      if set {
        code.push(format!("    .{name}(true)"));
      }
    }

    let value = self.value.get();
    let label = FRAMEWORKS
      .iter()
      .find(|(held, _)| *held == value)
      .map(|(_, label)| *label);
    let shows = match label {
      Some(label) => format!("shows {label:?}"),
      None if !value.is_empty() => format!("shows {value:?}, typed and kept"),
      None => format!("placeholder {placeholder:?}"),
    };
    let tree = vec![
      format!("box          {}", self.playground.measured()),
      format!("├─ input     {height} px high   {shows}"),
      "└─ chevrons  16 × 16 px".to_owned(),
      "list         4 under it   what is typed filters it, the chosen item checked".to_owned(),
    ];

    let sizes: Vec<(&str, usize)> = SIZES.iter().enumerate().map(|(index, size)| (size.1, index)).collect();
    let (to_size, to_value) = (self.size.clone(), self.value.clone());
    let reset = {
      let (value, placeholder, empty, size) = (
        self.value.clone(),
        self.placeholder.clone(),
        self.empty.clone(),
        self.size.clone(),
      );
      let flags = [
        self.invalid.clone(),
        self.warning.clone(),
        self.valid.clone(),
        self.disabled.clone(),
        self.full_width.clone(),
        self.loading.clone(),
        self.allow_custom.clone(),
      ];
      move || {
        value.set(String::new());
        placeholder.set("Select framework".to_owned());
        empty.set("No results".to_owned());
        size.set(1);
        for flag in &flags {
          flag.set(false);
        }
      }
    };
    let changed = flags.iter().filter(|(_, set)| *set).count()
      + usize::from(size != ComboboxSize::Md)
      + usize::from(!value.is_empty())
      + usize::from(placeholder != "Select framework")
      + usize::from(empty != "No results");
    let chosen = VALUES
      .iter()
      .find(|(_, held)| *held == value)
      .map(|(_, held)| (*held).to_owned())
      .unwrap_or_default();
    self.playground.card(
      ctx,
      Spec {
        title: "Combobox",
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
              &chosen,
              move |value| to_value.set(value),
            ),
          ),
          prop("placeholder", "&str", field(&self.placeholder)),
          prop("empty", "&str", field(&self.empty)),
          prop(
            "size",
            "ComboboxSize",
            segmented_fill(&sizes, &self.size.get(), move |index| to_size.set(index)),
          ),
          divider(),
          flag("invalid", &self.invalid),
          flag("warning", &self.warning),
          flag("valid", &self.valid),
          flag("disabled", &self.disabled),
          flag("full_width", &self.full_width),
          flag("loading", &self.loading),
          flag("allow_custom", &self.allow_custom),
        ],
        changed,
        reset: Arc::new(reset),
      },
    )
  }
}

impl Component for ComboboxPage {
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
      placeholder: ctx.signal("Select framework".to_owned()),
      empty: ctx.signal("No results".to_owned()),
      size: ctx.signal(1),
      invalid: ctx.signal(false),
      warning: ctx.signal(false),
      valid: ctx.signal(false),
      disabled: ctx.signal(false),
      full_width: ctx.signal(false),
      loading: ctx.signal(false),
      allow_custom: ctx.signal(false),
      framework: ctx.signal("lurq".to_owned()),
      country: ctx.signal(String::new()),
      fetched: ctx.signal(Vec::new()),
      tag: ctx.signal(String::new()),
      crate_name: ctx.signal("wgpu".to_owned()),
      name: ctx.signal("Ivan Zaida".to_owned()),
      editor: ctx.signal("zed".to_owned()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let says = |value: &Signal<String>| format!("the signal has: {:?}", value.get());
    let live = items().child(item(
      Combobox::new(self.framework.clone(), frameworks()).placeholder("Select framework"),
      &says(&self.framework),
    ));
    let states = items()
      .child(item(showing(""), "default"))
      .child(item(showing("lurq"), "filled"))
      .child(item(showing("lurq").force(Some(Interaction::Hovered)), "hover"))
      .child(item(showing("lurq").force(Some(Interaction::FocusVisible)), "focus"))
      .child(item(showing("lurq").state(ComboboxState::Open, true), "open"))
      .child(item(showing("").invalid(true), "invalid"))
      .child(item(showing("gpui").warning(true), "warning"))
      .child(item(showing("lurq").valid(true), "valid"))
      .child(item(showing("").disabled(true), "disabled"));
    let mut sizes = items();
    for (size, name, _, height) in SIZES {
      sizes = sizes.child(item(showing("lurq").size(size), &format!("{name} · {height}")));
    }

    // The app fetches: what it has fetched for what was typed are the options, all of them shown, and the list says
    // it is loading while what was typed is new. Here the "server" answers at once.
    let fetched = self.fetched.get();
    let (to_fetch, country) = (self.fetched.clone(), self.country.clone());
    let fetching = items().child(item(
      Combobox::new(
        country.clone(),
        fetched
          .iter()
          .map(|name| (name.clone(), name.as_str()))
          .collect::<Vec<_>>(),
      )
      .placeholder("Search countries")
      .filter(|_, _| true)
      .empty("Type to search")
      .on_input(move |typed| {
        let typed = typed.trim().to_lowercase();
        let found = COUNTRIES
          .iter()
          .filter(|name| !typed.is_empty() && name.to_lowercase().starts_with(&typed))
          .map(|name| (*name).to_owned())
          .collect();
        to_fetch.set(found);
      }),
      &says(&self.country),
    ));
    let loading = items()
      .child(item(showing("").loading(true), "loading: the list ends in a spinner"))
      .child(item(
        Combobox::new(Signal::new(String::new()), frameworks())
          .placeholder("Select framework")
          .filter(|label, typed| label.to_lowercase().starts_with(&typed.to_lowercase())),
        "a filter of the app's: labels that start with it",
      ));
    let custom = items().child(item(
      Combobox::new(
        self.tag.clone(),
        [
          ("bug".to_owned(), "bug"),
          ("feature".to_owned(), "feature"),
          ("docs".to_owned(), "docs"),
        ],
      )
      .placeholder("Add a label")
      .allow_custom(true),
      &says(&self.tag),
    ));
    let crates = items().child(item(
      Combobox::new(
        self.crate_name.clone(),
        [
          SelectOption::new("wgpu".to_owned(), "wgpu")
            .group("Graphics")
            .detail("Cross-platform GPU API"),
          SelectOption::new("vello".to_owned(), "vello").hint("beta"),
          SelectOption::new("winit".to_owned(), "winit").group("Windowing"),
          SelectOption::new("sdl2".to_owned(), "sdl2")
            .detail("Not supported here")
            .disabled(true),
          SelectOption::new("tokio".to_owned(), "tokio").group("Async"),
          SelectOption::new("smol".to_owned(), "smol"),
        ],
      )
      .placeholder("Select crate"),
      &says(&self.crate_name),
    ));
    let form = Column::new()
      .width(420.0)
      .spacing(16.0)
      .child(Field::new("Display name", Input::new(self.name.clone()).full_width(true)).horizontal(true))
      .child(
        Field::new(
          "Editor",
          Combobox::new(
            self.editor.clone(),
            [
              ("zed".to_owned(), "Zed"),
              ("helix".to_owned(), "Helix"),
              ("code".to_owned(), "VS Code"),
              ("vim".to_owned(), "Vim"),
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
        "What the combobox is bound to: the option with its value is chosen, and choosing another sets it.",
      ),
      (
        "options",
        "impl IntoIterator<Item = impl Into<SelectOption<T>>>",
        "",
        "(value, label) pairs, or SelectOptions with a detail, a hint, an element beside the label, a group they \
         start, or disabled: a select's.",
      ),
      (
        "placeholder",
        "&str",
        "",
        "What the box says while nothing is typed, in the muted foreground.",
      ),
      (
        "filter",
        "Fn(&str, &str) -> bool",
        "contains",
        "Which options are shown while something is typed: (label, typed). By default the labels that contain it, \
         without regard to case.",
      ),
      (
        "on_input",
        "Fn(&str)",
        "",
        "Called with what the box will hold at each edit: for an app that fetches its options.",
      ),
      (
        "loading",
        "bool",
        "false",
        "The list ends in a row with a spinner that says more are loading.",
      ),
      (
        "empty",
        "&str",
        "\"No results\"",
        "What the list says while what is typed leaves no option.",
      ),
      (
        "allow_custom",
        "bool",
        "false",
        "What is typed may be no option's: Enter or leaving keeps it, and a value of String becomes it.",
      ),
      (
        "query",
        "Signal<String>",
        "",
        "Where the app keeps what is typed, to read or set it. Without one the combobox keeps it.",
      ),
      (
        "size",
        "ComboboxSize",
        "Md",
        "Sm 32, Md 36, Lg 40 high; text of 13, 14, 15, as an input's.",
      ),
      (
        "invalid / warning / valid",
        "bool",
        "false",
        "Its edge and its ring in the destructive, warning or success color. Invalid is shown over them.",
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
        breadcrumb: &["Components", "Inputs", "Combobox"],
        title: "Combobox",
        description: "A select a person types in: what is typed filters the list. A framework, a country, an \
                      assignee among many.",
        import: Some("use lurq::ui::Combobox;"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Combobox is a control on the right. The combobox on the stage is live: what is chosen in it \
           goes into the value on the right, and the other way.",
          playground,
        ),
        Section::new(
          "Combobox",
          "The page holds the value, in a signal. Typing opens the list and filters it, the first match \
           highlighted; Down and Up move the highlight, Enter chooses, Escape closes the list and a second Escape \
           clears what was typed. The focus stays in the box the whole time. Leaving it puts back the chosen \
           label.",
          self.frame.demo(
            0,
            live,
            "Combobox::new(framework.clone(), [(\"lurq\".to_owned(), \"lurq\"), ..])",
            &[
              "// framework: Signal<String>",
              "Combobox::new(framework.clone(), [",
              "    (\"lurq\".to_owned(), \"lurq\"),",
              "    (\"gpui\".to_owned(), \"GPUI\"),",
              "    (\"slint\".to_owned(), \"Slint\"),",
              "    (\"iced\".to_owned(), \"Iced\"),",
              "    (\"egui\".to_owned(), \"egui\"),",
              "    (\"dioxus\".to_owned(), \"Dioxus\"),",
              "])",
              ".placeholder(\"Select framework\")",
            ],
          ),
        ),
        Section::new(
          "States",
          "The box is a select's, and its edge says the state: the ring's color under the pointer, a line of 2 \
           inside it while the focus is in it and while the list is open; the destructive, warning and success \
           colors for what is chosen.",
          self.frame.demo(
            1,
            states,
            ".invalid(true)",
            &[
              "Combobox::new(framework, frameworks).placeholder(\"Select framework\")",
              "Combobox::new(framework, frameworks)",
              "",
              "// Drawn as it is under the pointer, focused or open, with nothing pointing at it: for a gallery, a test.",
              "Combobox::new(framework, frameworks).force(Some(Interaction::Hovered))",
              "Combobox::new(framework, frameworks).force(Some(Interaction::FocusVisible))",
              "Combobox::new(framework, frameworks).state(ComboboxState::Open, true)",
              "",
              "Combobox::new(framework, frameworks).invalid(true)",
              "Combobox::new(framework, frameworks).warning(true)",
              "Combobox::new(framework, frameworks).valid(true)",
              "Combobox::new(framework, frameworks).disabled(true)",
            ],
          ),
        ),
        Section::new(
          "Sizes",
          "The heights of the theme's controls with the text a step smaller or larger, as an input and a select \
           have them.",
          self.frame.demo(
            2,
            sizes,
            ".size(ComboboxSize::Sm)",
            &[
              "Combobox::new(framework, frameworks).size(ComboboxSize::Sm)",
              "Combobox::new(framework, frameworks)",
              "Combobox::new(framework, frameworks).size(ComboboxSize::Lg)",
            ],
          ),
        ),
        Section::new(
          "Options an app fetches",
          "An app that asks a server hears what is typed (on_input), gives the options it fetched, and keeps every \
           one of them (a filter that keeps all). Until something is typed the list says so.",
          self.frame.demo(
            3,
            fetching,
            ".on_input(|typed| fetch(typed)).filter(|_, _| true)",
            &[
              "// country: Signal<String>, fetched: Signal<Vec<String>>",
              "Combobox::new(",
              "    country.clone(),",
              "    fetched.get().iter().map(|name| (name.clone(), name.as_str())).collect::<Vec<_>>(),",
              ")",
              ".placeholder(\"Search countries\")",
              ".filter(|_, _| true)",
              ".empty(\"Type to search\")",
              ".on_input(move |typed| fetched.set(search(typed)))",
            ],
          ),
        ),
        Section::new(
          "Loading and filters",
          "While more options are loading, the list ends in a row with a spinner. A filter of the app's says which \
           options a text leaves: here, the labels that start with it.",
          self.frame.demo(
            4,
            loading,
            ".loading(true)",
            &[
              "Combobox::new(framework, frameworks).loading(true)",
              "",
              "Combobox::new(framework, frameworks)",
              "    .filter(|label, typed| label.to_lowercase().starts_with(&typed.to_lowercase()))",
            ],
          ),
        ),
        Section::new(
          "Custom values",
          "What is typed may be no option's: typing highlights nothing, and Enter or leaving the box keeps it. A \
           label of an option, in any case, chooses the option; anything else becomes the value of a combobox of \
           String.",
          self.frame.demo(
            5,
            custom,
            ".allow_custom(true)",
            &[
              "// tag: Signal<String>",
              "Combobox::new(tag.clone(), [",
              "    (\"bug\".to_owned(), \"bug\"),",
              "    (\"feature\".to_owned(), \"feature\"),",
              "    (\"docs\".to_owned(), \"docs\"),",
              "])",
              ".placeholder(\"Add a label\")",
              ".allow_custom(true)",
            ],
          ),
        ),
        Section::new(
          "Groups and options",
          "The options are a select's: a detail under the label, a hint at its end, groups with their label and a \
           line between them, and options that cannot be chosen. A group keeps its label while what is typed \
           leaves any of its options.",
          self.frame.demo(
            6,
            crates,
            "SelectOption::new(value, \"wgpu\").group(\"Graphics\")",
            &[
              "// crate_name: Signal<String>",
              "Combobox::new(crate_name.clone(), [",
              "    SelectOption::new(\"wgpu\".to_owned(), \"wgpu\")",
              "        .group(\"Graphics\")",
              "        .detail(\"Cross-platform GPU API\"),",
              "    SelectOption::new(\"vello\".to_owned(), \"vello\").hint(\"beta\"),",
              "    SelectOption::new(\"winit\".to_owned(), \"winit\").group(\"Windowing\"),",
              "    SelectOption::new(\"sdl2\".to_owned(), \"sdl2\")",
              "        .detail(\"Not supported here\")",
              "        .disabled(true),",
              "    SelectOption::new(\"tokio\".to_owned(), \"tokio\").group(\"Async\"),",
              "    SelectOption::new(\"smol\".to_owned(), \"smol\"),",
              "])",
              ".placeholder(\"Select crate\")",
            ],
          ),
        ),
        Section::new(
          "In a form",
          "A combobox stands beside an input: the same box, the same height, the same ring. The field names it and \
           judges it.",
          self.frame.demo(
            7,
            form,
            "Field::new(\"Editor\", combobox).horizontal(true)",
            &[
              "// name, editor: Signal<String>",
              "Column::new()",
              "    .width(420.0)",
              "    .spacing(16.0)",
              "    .child(Field::new(\"Display name\", Input::new(name.clone()).full_width(true)).horizontal(true))",
              "    .child(",
              "        Field::new(",
              "            \"Editor\",",
              "            Combobox::new(editor.clone(), [",
              "                (\"zed\".to_owned(), \"Zed\"),",
              "                (\"helix\".to_owned(), \"Helix\"),",
              "                (\"code\".to_owned(), \"VS Code\"),",
              "                (\"vim\".to_owned(), \"Vim\"),",
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
          "What a combobox is given.",
          props_table(&rows, &["value", "options"], self.frame.room()),
        ),
      ],
    )
  }
}
