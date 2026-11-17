//! The page of the tree select: its playground, the board's select of a file, any number, only the leaves, in a
//! field, and its states.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{Field, Interaction, TreeItem, TreeSelect, TreeSelectSize},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

type Id = &'static str;

/// A workspace of crates.
fn project() -> Vec<TreeItem<Id>> {
  vec![
    TreeItem::folder("crates", "crates").items([
      TreeItem::folder("lurq", "lurq").items([
        TreeItem::file("lib.rs", "lib.rs"),
        TreeItem::file("theme.rs", "theme.rs"),
        TreeItem::file("a-long-name", "a-file-whose-name-is-longer-than-the-trigger.rs"),
      ]),
      TreeItem::folder("demo", "demo").item(TreeItem::file("main.rs", "main.rs")),
      TreeItem::folder("gallery", "gallery")
        .item(TreeItem::file("shell.rs", "shell.rs"))
        .item(TreeItem::file("pages.rs", "pages.rs").disabled(true)),
    ]),
    TreeItem::folder("docs", "docs").item(TreeItem::file("index.md", "index.md")),
    TreeItem::file("Cargo.toml", "Cargo.toml"),
    TreeItem::file("README.md", "README.md"),
  ]
}

/// The board's tree: crates, with lurq (lib.rs, theme.rs) and demo in it; Cargo.toml.
fn board() -> Vec<TreeItem<Id>> {
  vec![
    TreeItem::folder("crates", "crates").items([
      TreeItem::folder("lurq", "lurq").items([
        TreeItem::file("lib.rs", "lib.rs"),
        TreeItem::file("theme.rs", "theme.rs"),
      ]),
      TreeItem::folder("demo", "demo").item(TreeItem::file("main.rs", "main.rs")),
    ]),
    TreeItem::file("Cargo.toml", "Cargo.toml"),
  ]
}

const SIZES: [(&str, TreeSelectSize); 3] = [
  ("Sm", TreeSelectSize::Sm),
  ("Md", TreeSelectSize::Md),
  ("Lg", TreeSelectSize::Lg),
];

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .max_width(FILL)
    .spacing(9.0)
    .child(Row::new().max_width(FILL).overflow_visible().child(shown))
    .child(quiet(caption, 12.0))
}

pub struct TreeSelectPage {
  frame: Frame,
  playground: Playground,
  multiple: Signal<bool>,
  size: Signal<TreeSelectSize>,
  placeholder: Signal<String>,
  path: Signal<bool>,
  leaves_only: Signal<bool>,
  clearable: Signal<bool>,
  full_width: Signal<bool>,
  invalid: Signal<bool>,
  read_only: Signal<bool>,
  disabled: Signal<bool>,
  played: Signal<Vec<Id>>,
  file: Signal<Option<Id>>,
  many: Signal<Vec<Id>>,
  leaf: Signal<Option<Id>>,
  entry: Signal<Option<Id>>,
}

impl TreeSelectPage {
  /// The playground: the select as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (multiple, size, placeholder) = (self.multiple.get(), self.size.get(), self.placeholder.get());
    let (path, leaves_only, clearable) = (self.path.get(), self.leaves_only.get(), self.clearable.get());
    let (full_width, invalid, read_only, disabled) = (
      self.full_width.get(),
      self.invalid.get(),
      self.read_only.get(),
      self.disabled.get(),
    );
    let chosen = self.played.get();
    let record = self.playground.recorder();
    let forced = self.playground.forced();
    // A select of one or none and one of any number are of two types: what both are given is said once.
    macro_rules! dressed {
      ($select:expr, $record:expr, $placeholder:expr, $playground:expr) => {{
        let opened = $record;
        let select = $select;
        let select = match $playground {
          Some(playground) => playground.styled(select),
          None => select,
        };
        select
          .label("File")
          .placeholder($placeholder)
          .size(size)
          .path(path)
          .leaves_only(leaves_only)
          .clearable(clearable)
          .full_width(full_width)
          .invalid(invalid)
          .read_only(read_only)
          .disabled(disabled)
          .force(forced)
          .on_open_change(move |open| opened(format!("on_open_change({open})")))
      }};
    }
    let build = {
      let (set, record, placeholder) = (self.played.clone(), record.clone(), placeholder.clone());
      move |ctx: &mut Ctx, chosen: Vec<Id>, playground: Option<&Playground>| -> Element {
        let (set, changed) = (set.clone(), record.clone());
        if multiple {
          dressed!(
            TreeSelect::controlled_multiple(chosen, project(), move |next| {
              changed(format!("on_change({next:?})"));
              set.set(next.to_vec());
            }),
            record.clone(),
            &placeholder,
            playground
          )
          .mount(ctx)
        } else {
          dressed!(
            TreeSelect::controlled_optional(chosen.last().copied(), project(), move |next| {
              changed(format!("on_change({next:?})"));
              set.set(next.iter().copied().collect());
            }),
            record.clone(),
            &placeholder,
            playground
          )
          .mount(ctx)
        }
      }
    };
    let shown = {
      let (build, chosen, playground) = (build.clone(), chosen.clone(), &self.playground);
      Shown::built(move |ctx| build(ctx, chosen, Some(playground)))
    };
    {
      let (build, chosen) = (build.clone(), self.played.clone());
      self.playground.window_shows(move |ctx| build(ctx, chosen.get(), None));
    }
    let make = {
      let (chosen, placeholder) = (chosen.clone(), placeholder.clone());
      move || -> Element {
        let quiet = |_: String| {};
        if multiple {
          dressed!(
            TreeSelect::controlled_multiple(chosen.clone(), project(), |_| {}),
            quiet,
            &placeholder,
            None::<&Playground>
          )
          .into()
        } else {
          dressed!(
            TreeSelect::controlled_optional(chosen.last().copied(), project(), |_| {}),
            quiet,
            &placeholder,
            None::<&Playground>
          )
          .into()
        }
      }
    };

    let mut code = vec![if multiple {
      "// files: Signal<Vec<&str>>".to_owned()
    } else {
      "// file: Signal<Option<&str>>".to_owned()
    }];
    code.push(if multiple {
      "TreeSelect::multiple(files, items)".to_owned()
    } else {
      "TreeSelect::optional(file, items)".to_owned()
    });
    code.push(".label(\"File\")".to_owned());
    if !placeholder.is_empty() {
      code.push(format!(".placeholder({placeholder:?})"));
    }
    if size != TreeSelectSize::Md {
      code.push(format!(".size(TreeSelectSize::{size:?})"));
    }
    for (on, call) in [
      (!path && !multiple, ".path(false)"),
      (leaves_only, ".leaves_only(true)"),
      (clearable, ".clearable(true)"),
      (full_width, ".full_width(true)"),
      (invalid, ".invalid(true)"),
      (read_only, ".read_only(true)"),
      (disabled, ".disabled(true)"),
    ] {
      if on {
        code.push(call.to_owned());
      }
    }
    code.push(".mount(ctx)".to_owned());

    let tree = vec![
      format!("trigger     {}   a select's box", self.playground.measured()),
      "├─ value    the path to what is chosen, or the labels of any number; an ellipsis where it is cut".to_owned(),
      "├─ clear    16   where it is clearable and something is chosen".to_owned(),
      "└─ chevron  16".to_owned(),
      "panel       260 wide, 4 under the trigger, padded 8".to_owned(),
      "└─ tree     a tree view of the items, which scrolls past 280".to_owned(),
    ];

    let flags = [
      self.multiple.clone(),
      self.leaves_only.clone(),
      self.clearable.clone(),
      self.full_width.clone(),
      self.invalid.clone(),
      self.read_only.clone(),
      self.disabled.clone(),
    ];
    let (reset_size, reset_placeholder, reset_path, reset_chosen) = (
      self.size.clone(),
      self.placeholder.clone(),
      self.path.clone(),
      self.played.clone(),
    );
    let set_size = self.size.clone();
    let mut controls = vec![
      flag("multiple", &self.multiple),
      prop(
        "size",
        "TreeSelectSize",
        crate::kit::segmented(&SIZES, &size, move |size| set_size.set(size)),
      ),
      prop("placeholder", "&str", field(&self.placeholder)),
      divider(),
    ];
    // The path is what a select of one says: of any number the trigger says their labels.
    if !multiple {
      controls.push(flag("path", &self.path));
    }
    controls.extend([
      flag("leaves_only", &self.leaves_only),
      flag("clearable", &self.clearable),
      flag("full_width", &self.full_width),
      flag("invalid", &self.invalid),
      flag("read_only", &self.read_only),
      flag("disabled", &self.disabled),
    ]);
    self.playground.card(
      ctx,
      Spec {
        title: "Tree Select",
        make: Arc::new(make),
        shown,
        fill: full_width,
        forceable: true,
        facts: format!("{} chosen", chosen.len()),
        event: "on_change",
        code,
        tree,
        controls,
        changed: [
          multiple,
          size != TreeSelectSize::Md,
          placeholder != "Select a file",
          !path,
          leaves_only,
          clearable,
          full_width,
          invalid,
          read_only,
          disabled,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          for flag in &flags {
            flag.set(false);
          }
          reset_size.set(TreeSelectSize::Md);
          reset_placeholder.set("Select a file".to_owned());
          reset_path.set(true);
          reset_chosen.set(vec!["lib.rs"]);
        }),
      },
    )
  }
}

impl Component for TreeSelectPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      multiple: ctx.signal(false),
      size: ctx.signal(TreeSelectSize::Md),
      placeholder: ctx.signal("Select a file".to_owned()),
      path: ctx.signal(true),
      leaves_only: ctx.signal(false),
      clearable: ctx.signal(false),
      full_width: ctx.signal(false),
      invalid: ctx.signal(false),
      read_only: ctx.signal(false),
      disabled: ctx.signal(false),
      played: ctx.signal(vec!["lib.rs"]),
      file: ctx.signal(Some("lib.rs")),
      many: ctx.signal(vec!["lib.rs", "theme.rs", "Cargo.toml"]),
      leaf: ctx.signal(None),
      entry: ctx.signal(None),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let open = TreeSelect::controlled("lib.rs", board(), |_| {})
      .label("File")
      .inline(true);
    let one = TreeSelect::optional(self.file.clone(), board())
      .label("File")
      .placeholder("Select a file")
      .clearable(true)
      .key("one")
      .mount(ctx);
    let many = TreeSelect::multiple(self.many.clone(), project())
      .label("Files")
      .placeholder("Select files")
      .max_shown(2)
      .clearable(true)
      .key("many")
      .mount(ctx);
    let leaves = TreeSelect::optional(self.leaf.clone(), project())
      .label("File")
      .placeholder("Select a file")
      .leaves_only(true)
      .path(false)
      .key("leaves")
      .mount(ctx);
    let in_field = Field::new(
      "Entry point",
      TreeSelect::optional(self.entry.clone(), board())
        .placeholder("Select a file")
        .leaves_only(true)
        .full_width(true),
    )
    .description("The file the crate is built from.")
    .required(true)
    .mount(ctx);
    let state = |select: TreeSelect<Id, Id>| select.style(lurq::ui::TreeSelectPart::Trigger, |s| s.width(200.0));
    let plain = || TreeSelect::controlled("Cargo.toml", board(), |_| {});
    let states = Row::new()
      .spacing(28.0)
      .wrap()
      .align_items(Alignment::Start)
      .child(item(state(plain()), "at rest"))
      .child(item(
        state(plain().force(Some(Interaction::Hovered))),
        "under the pointer",
      ))
      .child(item(
        state(plain().force(Some(Interaction::FocusVisible))),
        "focused from the keyboard",
      ))
      .child(item(state(plain().invalid(true)), "invalid"))
      .child(item(state(plain().read_only(true)), "read only"))
      .child(item(state(plain().disabled(true)), "disabled"));

    let rows = [
      (
        "value",
        "Signal<T>",
        "",
        "What is chosen, the first argument of `new`; `optional` takes a `Signal<Option<T>>` and `multiple` a \
         `Signal<Vec<T>>`. `controlled`, `controlled_optional` and `controlled_multiple` are the same held by the \
         app.",
      ),
      (
        "items",
        "impl IntoIterator<Item = TreeItem<T>>",
        "",
        "The tree: a tree view's items.",
      ),
      (
        "on_change",
        "impl Fn(&T) / Fn(&Option<T>) / Fn(&[T])",
        "",
        "Told what is chosen after a choice a person makes, after the signal is set.",
      ),
      (
        "open",
        "Signal<bool>",
        "kept where mounted",
        "Whether the panel is open. `on_open_change(bool)` hears each change a person makes, `on_dismiss()` Escape \
         and a press outside.",
      ),
      (
        "branches",
        "Signal<Vec<T>>",
        "kept where mounted",
        "The branches that are open; `branches_controlled(open, on_branch_change)` held by the app. \
         `on_branch_change(&T, bool)` hears each one opened or closed.",
      ),
      (
        "leaves_only",
        "bool",
        "false",
        "A press of a row that holds others opens or closes it in place of choosing it.",
      ),
      (
        "path",
        "bool",
        "true",
        "The trigger of a select of one says the whole path, with `separator_text` between its steps, or the label \
         alone.",
      ),
      (
        "max_shown",
        "usize",
        "all",
        "How many labels the trigger of a select of any number says before it counts the rest.",
      ),
      (
        "placeholder",
        "&str",
        "",
        "What the trigger says while nothing is chosen.",
      ),
      ("size", "TreeSelectSize", "Md", "Sm 32, Md 36, Lg 40: a select's."),
      (
        "clearable",
        "bool",
        "false",
        "A cross before the chevron that takes what is chosen away; `clear_label` names it.",
      ),
      (
        "invalid / warning / valid",
        "bool",
        "false",
        "The judgment of what is chosen, in the trigger's edge.",
      ),
      (
        "read_only / disabled",
        "bool",
        "false",
        "The panel does not open; a disabled one takes no focus either.",
      ),
      (
        "full_width",
        "bool",
        "false",
        "As wide as what holds it, and its panel as wide.",
      ),
      (
        "label",
        "&str",
        "",
        "What the select is called to what reads the tree; `panel_label` for its panel and its tree.",
      ),
      (
        "placement / offset / collision / inline",
        "",
        "",
        "Where the panel stands, as a popover's; `dismiss_on_escape` and `dismiss_on_outside_press` too.",
      ),
      (
        "style / rules",
        "TreeSelectPart",
        "",
        "Trigger, Value, Placeholder, Chevron, Clear, ClearIcon, Panel, Tree, Row, RowIcon, RowLabel, Empty.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Inputs", "Tree Select"],
        title: "Tree Select",
        description: "A select whose options are a tree: a file of a project, a category of a catalogue, one \
                      or any number.",
        import: Some("use lurq::ui::{TreeItem, TreeSelect};"),
        status: Some("Preview"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of TreeSelect is a control on the right. The select on the stage is live: a press opens its \
           tree, the keys in it are a tree's, and a choice closes it.",
          playground,
        ),
        Section::new(
          "Open",
          "The board's select with its panel drawn in place: the trigger says the path to what is chosen, and \
           the branches it is in are open.",
          self.frame.demo(
            0,
            open,
            "TreeSelect::controlled(value, items, on_change).inline(true)",
            &[
              "TreeSelect::controlled(\"lib.rs\", [",
              "    TreeItem::folder(\"crates\", \"crates\").items([",
              "        TreeItem::folder(\"lurq\", \"lurq\").items([",
              "            TreeItem::file(\"lib.rs\", \"lib.rs\"),",
              "            TreeItem::file(\"theme.rs\", \"theme.rs\"),",
              "        ]),",
              "        TreeItem::folder(\"demo\", \"demo\").item(TreeItem::file(\"main.rs\", \"main.rs\")),",
              "    ]),",
              "    TreeItem::file(\"Cargo.toml\", \"Cargo.toml\"),",
              "], |file| { /* the app's file */ })",
              ".label(\"File\")",
              ".inline(true)",
            ],
          ),
        ),
        Section::new(
          "One or none",
          "`TreeSelect::optional` with a `Signal<Option<&str>>`, mounted so that it keeps whether it is open and \
           which branches are. A press, Enter, Space or Down opens it with the focus on the chosen row; a press of \
           a row or Space chooses it, Enter does on a row that holds nothing; Escape closes it. The cross clears.",
          self.frame.demo(
            1,
            one,
            "TreeSelect::optional(file, items).clearable(true).mount(ctx)",
            &[
              "// file: Signal<Option<&str>>",
              "TreeSelect::optional(file, items)",
              "    .label(\"File\")",
              "    .placeholder(\"Select a file\")",
              "    .clearable(true)",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Any number",
          "`TreeSelect::multiple` with a `Signal<Vec<&str>>`: a press of a row, or Space on it, adds it or takes \
           it out, and the panel stays open. The trigger says the labels, here two of them before it counts the \
           rest.",
          self.frame.demo(
            2,
            many,
            "TreeSelect::multiple(files, items).max_shown(2).mount(ctx)",
            &[
              "// files: Signal<Vec<&str>>",
              "TreeSelect::multiple(files, items)",
              "    .label(\"Files\")",
              "    .max_shown(2)",
              "    .clearable(true)",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Only the leaves",
          "With `leaves_only` a press of a folder opens or closes it, and only a file is chosen. Here the trigger \
           says the file's name alone.",
          self.frame.demo(
            3,
            leaves,
            ".leaves_only(true).path(false)",
            &[
              "TreeSelect::optional(file, items)",
              "    .leaves_only(true)",
              "    .path(false)",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "In a field",
          "A field names the select, judges it and disables it, and a mounted field mounts it.",
          self.frame.demo(
            4,
            Column::new().width(320.0).max_width(FILL).child(in_field),
            "Field::new(label, TreeSelect::optional(..)).mount(ctx)",
            &[
              "Field::new(",
              "    \"Entry point\",",
              "    TreeSelect::optional(entry, items).leaves_only(true).full_width(true),",
              ")",
              ".description(\"The file the crate is built from.\")",
              ".required(true)",
              ".mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "States",
          "The trigger is a select's: its edge takes the ring's color under the pointer, the focus is a line of 2 \
           inside it, a judgment colors its edge, a read only one is on the muted fill and a disabled one faded.",
          self.frame.demo(
            5,
            states,
            ".invalid(true) / .read_only(true) / .disabled(true)",
            &[
              "TreeSelect::controlled(\"Cargo.toml\", items, |_| {}).invalid(true)",
              "TreeSelect::controlled(\"Cargo.toml\", items, |_| {}).read_only(true)",
              "TreeSelect::controlled(\"Cargo.toml\", items, |_| {}).disabled(true)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a tree select is given.",
          props_table(&rows, &["value", "items"], self.frame.room()),
        ),
      ],
    )
  }
}
