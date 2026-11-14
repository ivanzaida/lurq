//! The page of the search field: its playground, a query the page holds and filters a list by, the hint of a key,
//! its states and its sizes.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{InputSize, Interaction, SearchField},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet, segmented_fill},
  playground::{Playground, Spec, divider, field, flag, prop},
};

/// The sizes, as the board names them, and how the code names them.
const SIZES: [(InputSize, &str, &str); 3] = [
  (InputSize::Sm, "sm", "InputSize::Sm"),
  (InputSize::Md, "md", "InputSize::Md"),
  (InputSize::Lg, "lg", "InputSize::Lg"),
];

/// What the page's list holds: the components the query filters.
const COMPONENTS: [&str; 8] = [
  "Button",
  "Card",
  "Checkbox",
  "Dialog",
  "Input",
  "Password Input",
  "Search Field",
  "Select",
];

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

/// A search field that holds `value`: for a stage that shows a state.
fn showing(value: &str) -> SearchField {
  SearchField::new(Signal::new(value.to_owned())).placeholder("Search components")
}

pub struct SearchFieldPage {
  frame: Frame,
  playground: Playground,
  value: Signal<String>,
  placeholder: Signal<String>,
  shortcut: Signal<String>,
  size: Signal<usize>,
  invalid: Signal<bool>,
  read_only: Signal<bool>,
  disabled: Signal<bool>,
  full_width: Signal<bool>,
  query: Signal<String>,
  submitted: Signal<String>,
}

impl SearchFieldPage {
  /// The playground: the search field as the panel's props make it, bound to the page's signal.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let placeholder = self.placeholder.get();
    let shortcut = self.shortcut.get();
    let (size, size_name, size_code) = SIZES[self.size.get().min(SIZES.len() - 1)];
    let flags = [
      ("invalid", self.invalid.get()),
      ("read_only", self.read_only.get()),
      ("disabled", self.disabled.get()),
      ("full_width", self.full_width.get()),
    ];
    let on = |name: &str| flags.iter().any(|(flag, set)| *flag == name && *set);
    let make = {
      let (placeholder, shortcut, value) = (placeholder.clone(), shortcut.clone(), self.value.clone());
      let set: Vec<bool> = flags.iter().map(|(_, set)| *set).collect();
      move || {
        let mut search = SearchField::new(value.clone())
          .placeholder(&placeholder)
          .size(size)
          .invalid(set[0])
          .read_only(set[1])
          .disabled(set[2])
          .full_width(set[3]);
        if !shortcut.is_empty() {
          search = search.shortcut(&shortcut);
        }
        search
      }
    };
    let record = self.playground.recorder();
    let typed = record.clone();
    let shown = make()
      .force(self.playground.forced())
      .on_input(move |event| typed(format!("on_input({:?})", event.new_value)))
      .on_submit(move |query| record(format!("on_submit({query:?})")));

    let mut code = vec![
      "// query: Signal<String>".to_owned(),
      "SearchField::new(query.clone())".to_owned(),
      format!("    .placeholder({placeholder:?})"),
    ];
    if !shortcut.is_empty() {
      code.push(format!("    .shortcut({shortcut:?})"));
    }
    if size != InputSize::Md {
      code.push(format!("    .size({size_code})"));
    }
    for (name, set) in flags {
      if set {
        code.push(format!("    .{name}(true)"));
      }
    }
    code.push("    .on_submit(|query| search(query))".to_owned());

    let filled = !self.value.get().is_empty();
    let editable = !on("read_only") && !on("disabled");
    let mut tree = vec![
      format!("root        {}", self.playground.measured()),
      "├─ icon     16 × 16 px   search".to_owned(),
    ];
    if filled && editable {
      tree.push(format!("├─ control  value {:?}", self.value.get()));
      tree.push("└─ clear    16 × 16 px   \"Clear search\"".to_owned());
    } else if !filled && !shortcut.is_empty() {
      tree.push(format!("├─ control  placeholder {placeholder:?}"));
      tree.push(format!("└─ shortcut 20 px high   {shortcut:?}"));
    } else {
      tree.push("└─ control  searchbox".to_owned());
    }

    let sizes: Vec<(&str, usize)> = SIZES.iter().enumerate().map(|(index, size)| (size.1, index)).collect();
    let to_size = self.size.clone();
    let reset = {
      let (value, placeholder, shortcut, size) = (
        self.value.clone(),
        self.placeholder.clone(),
        self.shortcut.clone(),
        self.size.clone(),
      );
      let flags = [
        self.invalid.clone(),
        self.read_only.clone(),
        self.disabled.clone(),
        self.full_width.clone(),
      ];
      move || {
        value.set(String::new());
        placeholder.set("Search components".to_owned());
        shortcut.set("/".to_owned());
        size.set(1);
        for flag in &flags {
          flag.set(false);
        }
      }
    };
    let changed = flags.iter().filter(|(_, set)| *set).count()
      + usize::from(size != InputSize::Md)
      + usize::from(filled)
      + usize::from(shortcut != "/")
      + usize::from(placeholder != "Search components");
    self.playground.card(
      ctx,
      Spec {
        title: "Search Field",
        make: Arc::new(move || make().into()),
        shown: shown.into(),
        fill: on("full_width"),
        forceable: true,
        facts: format!("{size_name} · radius 6"),
        event: "on_submit",
        code,
        tree,
        controls: vec![
          prop("value", "Signal<String>", field(&self.value)),
          prop("placeholder", "&str", field(&self.placeholder)),
          prop("shortcut", "&str", field(&self.shortcut)),
          prop(
            "size",
            "InputSize",
            segmented_fill(&sizes, &self.size.get(), move |index| to_size.set(index)),
          ),
          divider(),
          flag("invalid", &self.invalid),
          flag("read_only", &self.read_only),
          flag("disabled", &self.disabled),
          flag("full_width", &self.full_width),
        ],
        changed,
        reset: Arc::new(reset),
      },
    )
  }
}

impl Component for SearchFieldPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      value: ctx.signal(String::new()),
      placeholder: ctx.signal("Search components".to_owned()),
      shortcut: ctx.signal("/".to_owned()),
      size: ctx.signal(1),
      invalid: ctx.signal(false),
      read_only: ctx.signal(false),
      disabled: ctx.signal(false),
      full_width: ctx.signal(false),
      query: ctx.signal(String::new()),
      submitted: ctx.signal(String::new()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let query = self.query.get().to_lowercase();
    let mut list = Column::new().spacing(4.0);
    let matching: Vec<&str> = COMPONENTS
      .into_iter()
      .filter(|name| name.to_lowercase().contains(&query))
      .collect();
    if matching.is_empty() {
      list = list.child(quiet("Nothing matches.", 13.0));
    }
    for name in matching {
      list = list.child(quiet(name, 13.0));
    }
    let submitted = self.submitted.clone();
    let live = items()
      .child(item(
        SearchField::new(self.query.clone())
          .placeholder("Search components")
          .shortcut("/")
          .on_submit(move |query| submitted.set(query.to_owned())),
        &format!(
          "the signal has: {:?}; Enter sent: {:?}",
          self.query.get(),
          self.submitted.get()
        ),
      ))
      .child(list);
    let states = items()
      .child(item(showing("").shortcut("/"), "default, with a key"))
      .child(item(showing("blur"), "filled"))
      .child(item(showing("blur").force(Some(Interaction::FocusVisible)), "focus"))
      .child(item(showing("").shortcut("Ctrl K"), "a chord"))
      .child(item(showing("blur").read_only(true), "read only"))
      .child(item(showing("").disabled(true), "disabled"));
    let mut sizes = items();
    for (size, name, _) in SIZES {
      sizes = sizes.child(item(showing("blur").size(size), name));
    }

    let rows = [
      (
        "value",
        "Signal<String>",
        "",
        "What the field holds; what is typed is set on it, and the app filters by it.",
      ),
      ("placeholder", "&str", "", "What it says while it is empty."),
      (
        "shortcut",
        "&str",
        "",
        "The key that reaches the field, at its end while it is empty. The app binds the key.",
      ),
      (
        "on_submit",
        "impl Fn(&str)",
        "",
        "Called on Enter with what the field holds; the focus stays in it.",
      ),
      (
        "on_input",
        "impl Fn(&TextInputEvent)",
        "",
        "Called before an edit, which it may refuse.",
      ),
      (
        "clear_label",
        "&str",
        "\"Clear search\"",
        "What the clear button is called to tooling, in another language.",
      ),
      ("size", "InputSize", "Md", "Sm 32, Md 36, Lg 40 high, as an input."),
      (
        "invalid / warning / valid",
        "bool",
        "false",
        "The color of the edge, as an input's.",
      ),
      (
        "read_only",
        "bool",
        "false",
        "It can be focused, not changed: no clear button, and Escape does not empty it.",
      ),
      ("disabled", "bool", "false", "Half the opacity; it takes no input."),
      (
        "full_width",
        "bool",
        "false",
        "As wide as what holds it, in place of 280.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Inputs", "Search Field"],
        title: "Search Field",
        description: "An input for a query: the search icon before the text, a button that empties it, and the \
                      hint of the key that reaches it.",
        import: Some("use lurq::ui::SearchField;"),
        status: Some("New"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of SearchField is a control on the right. The field on the stage is live: what is typed in it \
           goes into the value, Escape empties it and Enter submits it.",
          playground,
        ),
        Section::new(
          "Search Field",
          "The page holds the query, in a signal, and filters the list by it as it changes. The clear button \
           appears with text; Escape does what it does, so Tab passes it by. To tooling the field is a searchbox.",
          self.frame.demo(
            0,
            live,
            "SearchField::new(query.clone()).shortcut(\"/\")",
            &[
              "// query, sent: Signal<String>",
              "SearchField::new(query.clone())",
              "    .placeholder(\"Search components\")",
              "    .shortcut(\"/\")",
              "    .on_submit(move |query| sent.set(query.to_owned()))",
              "",
              "let query = query.get().to_lowercase();",
              "for name in COMPONENTS.iter().filter(|name| name.to_lowercase().contains(&query)) {",
              "    list = list.child(Text::new(name));",
              "}",
            ],
          ),
        ),
        Section::new(
          "States",
          "The box is an input's: its edge says its state. The hint shows while the field is empty, the clear \
           button while it holds something that can be taken away.",
          self.frame.demo(
            1,
            states,
            ".shortcut(\"Ctrl K\")",
            &[
              "SearchField::new(query).placeholder(\"Search components\").shortcut(\"/\")",
              "SearchField::new(query).placeholder(\"Search components\")",
              "SearchField::new(query).force(Some(Interaction::FocusVisible))",
              "SearchField::new(query).shortcut(\"Ctrl K\")",
              "SearchField::new(query).read_only(true)",
              "SearchField::new(query).disabled(true)",
            ],
          ),
        ),
        Section::new(
          "Sizes",
          "An input's three heights. The icon and the clear button stay 16.",
          self.frame.demo(
            2,
            sizes,
            ".size(InputSize::Sm)",
            &[
              "SearchField::new(query).size(InputSize::Sm)",
              "SearchField::new(query)",
              "SearchField::new(query).size(InputSize::Lg)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a search field is given.",
          props_table(&rows, &["value"], self.frame.room()),
        ),
      ],
    )
  }
}
