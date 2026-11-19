//! The page of the transfer: its playground, the board's features, disabled items, a long list, its narrow form, a
//! disabled transfer, and its props.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx, theme::Density},
  components::{Column, SelectOption},
  core::Signal,
  node::Element,
  ui::Transfer,
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, segmented_fill},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

type Id = &'static str;

/// The densities, as the code names them.
const DENSITIES: [(&str, Density); 4] = [
  ("compact", Density::Compact),
  ("default", Density::Default),
  ("comfortable", Density::Comfortable),
  ("touch", Density::Touch),
];

/// The board's features: canvas, svg and router available, wgpu and winit enabled, and more, with `router`
/// disabled where `disabled`.
fn features(disabled: bool) -> Vec<SelectOption<Id>> {
  [
    "canvas", "svg", "router", "markdown", "form", "mcp", "devtools", "wgpu", "winit",
  ]
  .into_iter()
  .map(|id| SelectOption::new(id, id).disabled(disabled && id == "router"))
  .collect()
}

/// The board's five features alone.
fn board() -> Vec<(Id, Id)> {
  ["canvas", "svg", "router", "wgpu", "winit"]
    .into_iter()
    .map(|id| (id, id))
    .collect()
}

pub struct TransferPage {
  frame: Frame,
  playground: Playground,
  source_title: Signal<String>,
  target_title: Signal<String>,
  item_disabled: Signal<bool>,
  disabled: Signal<bool>,
  density: Signal<usize>,
  played: Signal<Vec<Id>>,
  enabled: Signal<Vec<Id>>,
  some_disabled: Signal<Vec<Id>>,
  narrow: Signal<Vec<Id>>,
}

impl TransferPage {
  /// The playground: the transfer as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (source_title, target_title) = (self.source_title.get(), self.target_title.get());
    let (item_disabled, disabled) = (self.item_disabled.get(), self.disabled.get());
    let (density_name, density) = DENSITIES[self.density.get().min(DENSITIES.len() - 1)];
    let record = self.playground.recorder();
    let forced = self.playground.forced();
    let dress = {
      let (source_title, target_title) = (source_title.clone(), target_title.clone());
      move |transfer: Transfer<Id>| {
        transfer
          .source_title(&source_title)
          .target_title(&target_title)
          .label("Features")
          .disabled(disabled)
          .density(density)
          .force(forced)
      }
    };
    let build = {
      let (dress, played, record) = (dress.clone(), self.played.clone(), record.clone());
      move |ctx: &mut Ctx, style: &dyn Fn(Transfer<Id>) -> Transfer<Id>| -> Element {
        let (changed, checked) = (record.clone(), record.clone());
        dress(style(Transfer::new(played.clone(), features(item_disabled))))
          .on_change(move |value| changed(format!("on_change({value:?})")))
          .on_checked_change(move |value| checked(format!("on_checked_change({value:?})")))
          .mount(ctx)
      }
    };
    let shown = {
      let (build, playground) = (build.clone(), &self.playground);
      Shown::built(move |ctx| build(ctx, &|transfer| playground.styled(transfer)))
    };
    self
      .playground
      .window_shows(move |ctx| build(ctx, &|transfer| transfer));
    let make = {
      let (dress, value) = (dress.clone(), self.played.get());
      move || -> Element { dress(Transfer::controlled(value.clone(), features(item_disabled), |_| {})).into() }
    };

    let mut code = vec![
      "// enabled: Signal<Vec<&str>>".to_owned(),
      "Transfer::new(enabled, [".to_owned(),
      "    SelectOption::new(\"canvas\", \"canvas\"),".to_owned(),
      format!(
        "    SelectOption::new(\"router\", \"router\"){},",
        if item_disabled { ".disabled(true)" } else { "" }
      ),
      "    ..".to_owned(),
      "])".to_owned(),
      format!(".source_title({source_title:?})"),
      format!(".target_title({target_title:?})"),
      ".label(\"Features\")".to_owned(),
    ];
    if density != Density::Default {
      code.push(format!(".density(Density::{density:?})"));
    }
    if disabled {
      code.push(".disabled(true)".to_owned());
    }
    code.push(".on_change(|enabled: &[&str]| /* use it */)".to_owned());
    code.push(".mount(ctx)".to_owned());

    let tree = vec![
      format!(
        "root          {}   the lists 12 from the buttons",
        self.playground.measured()
      ),
      "├─ list      190, an edge of 1, radius lg; as wide as the room when stacked".to_owned(),
      "│  ├─ header 36 on the muted fill: select all, title 13 at 500, count 11 mono".to_owned(),
      format!("│  └─ items  a listbox with checkboxes, items at {density_name}; scrolls past 264"),
      "└─ actions   two buttons of 32, 8 apart: primary to the right, outline back".to_owned(),
    ];

    let set_density = self.density.clone();
    let densities: Vec<(&str, usize)> = DENSITIES
      .iter()
      .enumerate()
      .map(|(index, (name, _))| (*name, index))
      .collect();
    let controls = vec![
      prop("source_title", "&str", field(&self.source_title)),
      prop("target_title", "&str", field(&self.target_title)),
      prop(
        "density",
        "Density",
        segmented_fill(&densities, &self.density.get(), move |value| set_density.set(value)),
      ),
      divider(),
      flag("a disabled item", &self.item_disabled),
      flag("disabled", &self.disabled),
    ];
    let resets = (
      self.source_title.clone(),
      self.target_title.clone(),
      self.item_disabled.clone(),
      self.disabled.clone(),
      self.density.clone(),
      self.played.clone(),
    );
    self.playground.card(
      ctx,
      Spec {
        title: "Transfer",
        make: Arc::new(make),
        shown,
        fill: true,
        forceable: true,
        facts: format!("{} on the right", self.played.get().len()),
        event: "on_change",
        code,
        tree,
        controls,
        changed: [
          source_title != "Available",
          target_title != "Enabled",
          item_disabled,
          disabled,
          density != Density::Default,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          resets.0.set("Available".to_owned());
          resets.1.set("Enabled".to_owned());
          resets.2.set(false);
          resets.3.set(false);
          resets.4.set(1);
          resets.5.set(vec!["wgpu", "winit"]);
        }),
      },
    )
  }
}

impl Component for TransferPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      source_title: ctx.signal("Available".to_owned()),
      target_title: ctx.signal("Enabled".to_owned()),
      item_disabled: ctx.signal(false),
      disabled: ctx.signal(false),
      density: ctx.signal(1),
      played: ctx.signal(vec!["wgpu", "winit"]),
      enabled: ctx.signal(vec!["wgpu", "winit"]),
      some_disabled: ctx.signal(vec!["wgpu"]),
      narrow: ctx.signal(vec!["wgpu", "winit"]),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let the_board = Transfer::new(self.enabled.clone(), board())
      .source_title("Available")
      .target_title("Enabled")
      .label("Features")
      .key("board")
      .mount(ctx);
    let some_disabled = Transfer::new(self.some_disabled.clone(), features(true))
      .source_title("Available")
      .target_title("Enabled")
      .label("Features")
      .key("disabled-items")
      .mount(ctx);
    let long = ctx.mount::<LongTransfer>(());
    let narrow = Column::new().width(320.0).max_width(FILL).child(
      Transfer::new(self.narrow.clone(), board())
        .source_title("Available")
        .target_title("Enabled")
        .label("Features")
        .key("narrow")
        .mount(ctx),
    );
    let disabled = Transfer::controlled(["wgpu", "winit"], board(), |_| {})
      .source_title("Available")
      .target_title("Enabled")
      .disabled(true)
      .key("disabled")
      .mount(ctx);

    let rows = [
      (
        "value",
        "Signal<Vec<T>>",
        "",
        "The values of the items on the right, in the order they came there. `Transfer::controlled(value, items, \
         on_change)` is the same held by the app.",
      ),
      (
        "items",
        "impl IntoIterator<Item = impl Into<SelectOption<T>>>",
        "",
        "A listbox's options: `(value, label)` or a `SelectOption` with `disabled`, `detail`, `hint`, `leading`, \
         `trailing`. The left list has those not on the right, in this order.",
      ),
      (
        "on_change",
        "Fn(&[T])",
        "",
        "Told the whole new value after a move, after the signal is set; not when the app sets it.",
      ),
      (
        "checked",
        "Signal<Vec<T>>",
        "kept where mounted",
        "What is checked in either list. `checked_controlled(checked, on_checked_change)` held by the app; \
         `on_checked_change(&[T])` hears each change.",
      ),
      (
        "source_title / target_title",
        "&str",
        "\"Available\" / \"Selected\"",
        "What the lists are called, in their headers and to tooling.",
      ),
      (
        "count_text",
        "Fn(usize, usize) -> String",
        "\"1/3\"",
        "What a header says of how many are checked, of how many.",
      ),
      (
        "select_all_label / add_label / remove_label",
        "&str",
        "\"Select all\" / \"Add\" / \"Remove\"",
        "What the header's checkbox and the two buttons are called to tooling.",
      ),
      ("empty_text", "&str", "\"No items\"", "What a list with no items says."),
      ("label", "&str", "", "What the transfer is called, to tooling."),
      ("disabled", "bool", "false", "Nothing in it takes input."),
      (
        "density",
        "Density",
        "the theme's",
        "How high an item is: 24, 32, 40 or 44.",
      ),
      (
        "style / rules",
        "TransferPart",
        "",
        "Root, List, Header, Title, Count, Items, Item, ItemLabel, Empty, Actions.",
      ),
      (
        "mount",
        "&mut Ctx",
        "",
        "Keeps what is checked, finds the narrow form by its room, and builds only the items near the view of a \
         list of more than two hundred.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Inputs", "Transfer"],
        title: "Transfer",
        description: "Two lists side by side, and the buttons between them that move what is checked from one to \
                      the other: the features of a build, the columns a table shows.",
        import: Some("use lurq::ui::Transfer;"),
        status: Some("Preview"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Transfer is a control on the right. The transfer on the stage is live: a press checks an \
           item, the buttons move what is checked, and each move is counted.",
          playground,
        ),
        Section::new(
          "The board's transfer",
          "Check an item with a press, or several with Shift; the checkbox in a header checks the whole list. The \
           button with the chevron to the right moves what is checked on the left to the end of the right, the \
           other moves it back, and each waits while its list has nothing checked. Each list is one stop of Tab: \
           Up and Down move, Space checks, Shift with the arrows checks on the way, Ctrl+A checks them all.",
          self.frame.demo(
            0,
            the_board,
            "Transfer::new(enabled, items).mount(ctx)",
            &[
              "// enabled: Signal<Vec<&str>>",
              "Transfer::new(enabled, [",
              "    (\"canvas\", \"canvas\"),",
              "    (\"svg\", \"svg\"),",
              "    (\"router\", \"router\"),",
              "    (\"wgpu\", \"wgpu\"),",
              "    (\"winit\", \"winit\"),",
              "])",
              ".source_title(\"Available\")",
              ".target_title(\"Enabled\")",
              ".label(\"Features\")",
              ".mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Disabled items",
          "A disabled item is faded, is not checked by a press or by the header's checkbox, and does not move.",
          self.frame.demo(
            1,
            some_disabled,
            "SelectOption::new(\"router\", \"router\").disabled(true)",
            &[
              "Transfer::new(enabled, [",
              "    SelectOption::new(\"canvas\", \"canvas\"),",
              "    SelectOption::new(\"router\", \"router\").disabled(true),",
              "    ..",
              "])",
              ".mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "A long list",
          "Two thousand columns: a list scrolls past eight items, and one of more than two hundred builds only the \
           items near its view. End goes to the last, PageDown a view down.",
          self.frame.demo(
            2,
            long,
            "Transfer::new(columns, (0..2000).map(..)).mount(ctx)",
            &[
              "// columns: Signal<Vec<u32>>",
              "Transfer::new(columns, (0..2000u32).map(|i| SelectOption::new(i, format!(\"Column {i}\"))))",
              "    .source_title(\"Columns\")",
              "    .target_title(\"Shown\")",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "In a narrow room",
          "Where the lists do not fit side by side, a mounted transfer lays them one over the other, the buttons \
           in a row between them and their chevrons turned down and up: chosen by its own room, here 320 wide.",
          self.frame.demo(
            3,
            narrow,
            "Column::new().width(320.0).child(Transfer::new(..).mount(ctx))",
            &[
              "Column::new().width(320.0).child(",
              "    Transfer::new(enabled, items)",
              "        .source_title(\"Available\")",
              "        .target_title(\"Enabled\")",
              "        .mount(ctx),",
              ")",
            ],
          ),
        ),
        Section::new(
          "Disabled",
          "A disabled transfer is faded and takes nothing.",
          self.frame.demo(
            4,
            disabled,
            ".disabled(true)",
            &[
              "Transfer::controlled([\"wgpu\", \"winit\"], items, |_| {})",
              "    .disabled(true)",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a transfer is given.",
          props_table(&rows, &["value", "items"], self.frame.room()),
        ),
      ],
    )
  }
}

/// Two thousand columns, in a component of its own so that the page's other signals do not build them again.
struct LongTransfer {
  columns: Signal<Vec<u32>>,
}

impl Component for LongTransfer {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      columns: ctx.signal(vec![0, 1, 2]),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    Transfer::new(
      self.columns.clone(),
      (0..2000u32).map(|i| SelectOption::new(i, format!("Column {i}"))),
    )
    .source_title("Columns")
    .target_title("Shown")
    .label("Columns")
    .mount(ctx)
  }
}
