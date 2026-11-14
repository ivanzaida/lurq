//! The page of the input group: its playground, an address with text before and after it, icons, a button and a
//! select joined with an input, its states and its sizes.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{Button, ButtonVariant, Field, Input, InputAddon, InputGroup, InputSize, Select, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet, segmented_fill},
  playground::{Playground, Spec, divider, field, flag, prop},
};

/// What the playground offers before the input, and how the code says it.
const BEFORE: [(&str, &str); 4] = [
  ("none", ""),
  ("text", ".before(\"https://\")"),
  ("icon", ".before(InputAddon::icon(icons::LINK))"),
  ("select", ".before(Select::new(scheme.clone(), SCHEMES))"),
];

/// What the playground offers after the input, and how the code says it.
const AFTER: [(&str, &str); 4] = [
  ("none", ""),
  ("text", ".after(\".com\")"),
  ("icon", ".after(InputAddon::icon(icons::CHECK_CIRCLE))"),
  ("button", ".after(Button::new(\"Go\").variant(ButtonVariant::Ghost))"),
];

/// The schemes a select before an address chooses between.
const SCHEMES: [(&str, &str); 2] = [("https", "https://"), ("http", "http://")];

/// The sizes, as the board names them, and how the code names them.
const SIZES: [(InputSize, &str, &str); 3] = [
  (InputSize::Sm, "sm", "InputSize::Sm"),
  (InputSize::Md, "md", "InputSize::Md"),
  (InputSize::Lg, "lg", "InputSize::Lg"),
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

/// An input that holds `value`: for a stage that shows a state.
fn showing(value: &str) -> Input {
  Input::new(Signal::new(value.to_owned()))
}

pub struct InputGroupPage {
  frame: Frame,
  playground: Playground,
  value: Signal<String>,
  placeholder: Signal<String>,
  before: Signal<usize>,
  after: Signal<usize>,
  size: Signal<usize>,
  invalid: Signal<bool>,
  disabled: Signal<bool>,
  full_width: Signal<bool>,
  scheme: Signal<&'static str>,
  site: Signal<String>,
  email: Signal<String>,
  invited: Signal<String>,
}

impl InputGroupPage {
  /// The playground: the group as the panel's props make it, its input bound to the page's signal.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let placeholder = self.placeholder.get();
    let (before, after) = (
      self.before.get().min(BEFORE.len() - 1),
      self.after.get().min(AFTER.len() - 1),
    );
    let (size, size_name, size_code) = SIZES[self.size.get().min(SIZES.len() - 1)];
    let flags = [
      ("invalid", self.invalid.get()),
      ("disabled", self.disabled.get()),
      ("full_width", self.full_width.get()),
    ];
    let on = |name: &str| flags.iter().any(|(flag, set)| *flag == name && *set);
    let record = self.playground.recorder();
    let make = {
      let (placeholder, value, scheme) = (placeholder.clone(), self.value.clone(), self.scheme.clone());
      let set: Vec<bool> = flags.iter().map(|(_, set)| *set).collect();
      let record = record.clone();
      move || {
        let mut group = InputGroup::new(Input::new(value.clone()).placeholder(&placeholder))
          .size(size)
          .invalid(set[0])
          .disabled(set[1])
          .full_width(set[2]);
        group = match before {
          1 => group.before("https://"),
          2 => group.before(InputAddon::icon(icons::LINK)),
          3 => group.before(Select::new(scheme.clone(), SCHEMES)),
          _ => group,
        };
        let record = record.clone();
        match after {
          1 => group.after(".com"),
          2 => group.after(InputAddon::icon(icons::CHECK_CIRCLE)),
          3 => group.after(
            Button::new("Go")
              .variant(ButtonVariant::Ghost)
              .on_click(move |_| record("on_click(Go)".to_owned())),
          ),
          _ => group,
        }
      }
    };
    let shown = make();

    let mut code = vec![
      "// value: Signal<String>".to_owned(),
      format!("InputGroup::new(Input::new(value.clone()).placeholder({placeholder:?}))"),
    ];
    for line in [BEFORE[before].1, AFTER[after].1] {
      if !line.is_empty() {
        code.push(format!("    {line}"));
      }
    }
    if size != InputSize::Md {
      code.push(format!("    .size({size_code})"));
    }
    for (name, set) in flags {
      if set {
        code.push(format!("    .{name}(true)"));
      }
    }

    let mut tree = vec![format!("root        {}", self.playground.measured())];
    match before {
      1 => tree.push("├─ addon    \"https://\"   muted fill".to_owned()),
      3 => tree.push("├─ select   https:// · http://".to_owned()),
      _ => {}
    }
    if matches!(before, 1 | 3) {
      tree.push("├─ line     1 px".to_owned());
    }
    tree.push(format!(
      "{} input    {}",
      if after == 0 { "└─" } else { "├─" },
      if before == 2 {
        "with the icon before its text"
      } else {
        "no edge of its own"
      }
    ));
    match after {
      1 => tree.extend([
        "├─ line     1 px".to_owned(),
        "└─ addon    \".com\"   muted fill".to_owned(),
      ]),
      2 => tree.push("└─ icon     16 × 16 px   over the end of the input".to_owned()),
      3 => tree.extend(["├─ line     1 px".to_owned(), "└─ button   \"Go\"".to_owned()]),
      _ => {}
    }

    let befores: Vec<(&str, usize)> = BEFORE.iter().enumerate().map(|(index, item)| (item.0, index)).collect();
    let afters: Vec<(&str, usize)> = AFTER.iter().enumerate().map(|(index, item)| (item.0, index)).collect();
    let sizes: Vec<(&str, usize)> = SIZES.iter().enumerate().map(|(index, size)| (size.1, index)).collect();
    let (to_before, to_after, to_size) = (self.before.clone(), self.after.clone(), self.size.clone());
    let reset = {
      let (value, placeholder, before, after, size) = (
        self.value.clone(),
        self.placeholder.clone(),
        self.before.clone(),
        self.after.clone(),
        self.size.clone(),
      );
      let flags = [self.invalid.clone(), self.disabled.clone(), self.full_width.clone()];
      move || {
        value.set("lurq".to_owned());
        placeholder.set("site".to_owned());
        before.set(1);
        after.set(1);
        size.set(1);
        for flag in &flags {
          flag.set(false);
        }
      }
    };
    let changed = flags.iter().filter(|(_, set)| *set).count()
      + usize::from(before != 1)
      + usize::from(after != 1)
      + usize::from(size != InputSize::Md)
      + usize::from(self.value.get() != "lurq")
      + usize::from(placeholder != "site");
    self.playground.card(
      ctx,
      Spec {
        title: "Input Group",
        make: Arc::new(move || make().into()),
        shown: shown.into(),
        fill: on("full_width"),
        forceable: false,
        facts: format!("{size_name} · radius 6"),
        event: if after == 3 { "on_click" } else { "" },
        code,
        tree,
        controls: vec![
          prop("input", "Input", field(&self.value)),
          prop("placeholder", "&str", field(&self.placeholder)),
          prop(
            "before",
            "impl Into<InputAddon>",
            self
              .playground
              .select(0, &befores, &before, move |index| to_before.set(index)),
          ),
          prop(
            "after",
            "impl Into<InputAddon>",
            self
              .playground
              .select(1, &afters, &after, move |index| to_after.set(index)),
          ),
          prop(
            "size",
            "InputSize",
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

impl Component for InputGroupPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 2),
      value: ctx.signal("lurq".to_owned()),
      placeholder: ctx.signal("site".to_owned()),
      before: ctx.signal(1),
      after: ctx.signal(1),
      size: ctx.signal(1),
      invalid: ctx.signal(false),
      disabled: ctx.signal(false),
      full_width: ctx.signal(false),
      scheme: ctx.signal("https"),
      site: ctx.signal("lurq.dev".to_owned()),
      email: ctx.signal(String::new()),
      invited: ctx.signal(String::new()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let texts = items()
      .child(item(
        InputGroup::new(Input::new(self.site.clone())).before("https://"),
        "prefix",
      ))
      .child(item(InputGroup::new(showing("lurq")).after(".com"), "suffix"))
      .child(item(
        InputGroup::new(showing("lurq")).before("https://").after(".com"),
        "prefix and suffix",
      ));
    let (email, invited) = (self.email.clone(), self.invited.clone());
    let joined = items()
      .child(item(
        InputGroup::new(Input::new(self.email.clone()).placeholder("Email"))
          .before(InputAddon::icon(icons::MAIL))
          .after(
            Button::new("Invite")
              .variant(ButtonVariant::Ghost)
              .on_click(move |_| invited.set(email.get())),
          ),
        &format!("invited: {:?}", self.invited.get()),
      ))
      .child(item(
        InputGroup::new(Input::new(self.site.clone())).before(Select::new(self.scheme.clone(), SCHEMES)),
        &format!("the scheme is {:?}", self.scheme.get()),
      ))
      .child(item(
        InputGroup::new(showing("12"))
          .after("px")
          .after(InputAddon::icon(icons::INFO)),
        "two after it",
      ));
    let states = items()
      .child(item(
        InputGroup::new(showing("lurq")).before("https://").invalid(true),
        "invalid",
      ))
      .child(item(
        InputGroup::new(showing("lurq")).before("https://").valid(true),
        "valid",
      ))
      .child(item(
        InputGroup::new(showing("lurq"))
          .before("https://")
          .after(Button::new("Go").variant(ButtonVariant::Ghost))
          .disabled(true),
        "disabled",
      ));
    let mut sizes = items();
    for (size, name, _) in SIZES {
      sizes = sizes.child(item(
        InputGroup::new(showing("lurq")).before("https://").size(size),
        name,
      ));
    }
    let in_field = items().child(
      Field::new(
        "Website",
        InputGroup::new(Input::new(self.site.clone()))
          .before("https://")
          .full_width(true),
      )
      .description("Where the project lives.")
      .full_width(true),
    );

    let rows = [
      (
        "input",
        "Input",
        "",
        "The input the group is joined around; it keeps its own value and handlers.",
      ),
      (
        "before",
        "impl Into<InputAddon>",
        "",
        "What stands before the input, in order: a text, an icon, a Button, a Select.",
      ),
      ("after", "impl Into<InputAddon>", "", "What stands after it, in order."),
      (
        "size",
        "InputSize",
        "Md",
        "The input's, and each button's and select's in the group.",
      ),
      (
        "invalid / warning / valid",
        "bool",
        "false",
        "The color of the box's edge, and of the input's ring.",
      ),
      (
        "disabled",
        "bool",
        "false",
        "Half the opacity on the muted fill; nothing in it takes input.",
      ),
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
        breadcrumb: &["Components", "Inputs", "Input Group"],
        title: "Input Group",
        description: "An input joined with what stands before and after it into one box: a scheme, a domain, a \
                      unit, an icon, the button that sends it.",
        import: Some("use lurq::ui::{Input, InputAddon, InputGroup};"),
        status: Some("New"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of InputGroup is a control on the right. The input on the stage is live: what is typed in it \
           goes into the value.",
          playground,
        ),
        Section::new(
          "Text before and after",
          "A text addon is on the muted fill, a line between it and the input. The corners of the box are the \
           group's; the input has no edge of its own.",
          self.frame.demo(
            0,
            texts,
            "InputGroup::new(input).before(\"https://\").after(\".com\")",
            &[
              "// site: Signal<String>",
              "InputGroup::new(Input::new(site.clone())).before(\"https://\")",
              "InputGroup::new(Input::new(site.clone())).after(\".com\")",
              "InputGroup::new(Input::new(site.clone())).before(\"https://\").after(\".com\")",
            ],
          ),
        ),
        Section::new(
          "Icons, buttons and selects",
          "An icon beside the input is in its box. A button and a select keep what they do, with the group's \
           corners at its ends and square ones inside; the ring of the focus is the focused part's, over its \
           neighbours.",
          self.frame.demo(
            1,
            joined,
            ".after(Button::new(\"Invite\"))",
            &[
              "// email, invited, site: Signal<String>; scheme: Signal<&str>",
              "InputGroup::new(Input::new(email.clone()).placeholder(\"Email\"))",
              "    .before(InputAddon::icon(icons::MAIL))",
              "    .after(Button::new(\"Invite\").variant(ButtonVariant::Ghost).on_click(move |_| invited.set(email.get())))",
              "",
              "InputGroup::new(Input::new(site.clone()))",
              "    .before(Select::new(scheme.clone(), [(\"https\", \"https://\"), (\"http\", \"http://\")]))",
              "",
              "InputGroup::new(Input::new(width)).after(\"px\").after(InputAddon::icon(icons::INFO))",
            ],
          ),
        ),
        Section::new(
          "States",
          "The box's edge says what the input holds. A disabled group is dimmed whole.",
          self.frame.demo(
            2,
            states,
            ".invalid(true)",
            &[
              "InputGroup::new(input).before(\"https://\").invalid(true)",
              "InputGroup::new(input).before(\"https://\").valid(true)",
              "InputGroup::new(input).before(\"https://\").after(Button::new(\"Go\")).disabled(true)",
            ],
          ),
        ),
        Section::new(
          "Sizes",
          "An input's three heights; the text of an addon is a step under the input's.",
          self.frame.demo(
            3,
            sizes,
            ".size(InputSize::Sm)",
            &[
              "InputGroup::new(input).before(\"https://\").size(InputSize::Sm)",
              "InputGroup::new(input).before(\"https://\")",
              "InputGroup::new(input).before(\"https://\").size(InputSize::Lg)",
            ],
          ),
        ),
        Section::new(
          "In a field",
          "A field names the input in the group and judges it.",
          self.frame.demo(
            4,
            in_field,
            "Field::new(\"Website\", group)",
            &[
              "Field::new(\"Website\", InputGroup::new(Input::new(site.clone())).before(\"https://\").full_width(true))",
              "    .description(\"Where the project lives.\")",
              "    .full_width(true)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What an input group is given.",
          props_table(&rows, &["input"], self.frame.room()),
        ),
      ],
    )
  }
}
