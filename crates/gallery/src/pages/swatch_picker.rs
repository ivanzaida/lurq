//! The page of the swatch picker: its playground, the board's panel of eighteen colors, the app's own values,
//! the bare grid in a popover's place, its states, and its props.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::{Element, color::Color},
  ui::{ColorSwatchSize, Interaction, Swatch, SwatchPicker},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

/// The board's swatches: reds to greens, teals to pinks, and the greys.
const BOARD: [(&str, &str); 18] = [
  ("#EF4444", "Red"),
  ("#F97316", "Orange"),
  ("#F59E0B", "Amber"),
  ("#EAB308", "Yellow"),
  ("#84CC16", "Lime"),
  ("#22C55E", "Green"),
  ("#14B8A6", "Teal"),
  ("#06B6D4", "Cyan"),
  ("#3B82F6", "Blue"),
  ("#6366F1", "Indigo"),
  ("#A855F7", "Purple"),
  ("#EC4899", "Pink"),
  ("#FAFAFA", "White"),
  ("#D4D4D8", "Light grey"),
  ("#A1A1AA", "Grey"),
  ("#71717A", "Mid grey"),
  ("#3F3F46", "Dark grey"),
  ("#18181B", "Black"),
];

/// The board's swatches, each called by its name; the one at `disabled` cannot be chosen, where it is given.
fn board(disabled: Option<usize>) -> Vec<Swatch> {
  BOARD
    .iter()
    .enumerate()
    .map(|(index, (hex, name))| {
      Swatch::new(Color::from_hex(hex))
        .label(name)
        .disabled(Some(index) == disabled)
    })
    .collect()
}

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .max_width(FILL)
    .spacing(9.0)
    .child(Row::new().max_width(FILL).overflow_visible().child(shown))
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

/// The labels of an app's own values: what a tag of a list of tasks is colored by.
#[derive(Clone, Copy, Debug, PartialEq, Eq, lurq::DevtoolsInspectable)]
enum Label {
  Bug,
  Feature,
  Docs,
  Question,
}

const COLUMNS: [(&str, usize); 3] = [("3", 3), ("4", 4), ("6", 6)];
const SIZES: [(&str, usize); 3] = [("Sm (20)", 0), ("Md (24)", 1), ("Lg (32)", 2)];

fn size_of(index: usize) -> ColorSwatchSize {
  [ColorSwatchSize::Sm, ColorSwatchSize::Md, ColorSwatchSize::Lg][index.min(2)]
}

pub struct SwatchPickerPage {
  frame: Frame,
  playground: Playground,
  played: Signal<Color>,
  label: Signal<String>,
  columns: Signal<usize>,
  size: Signal<usize>,
  framed: Signal<bool>,
  invalid: Signal<bool>,
  disabled: Signal<bool>,
  disabled_swatch: Signal<bool>,
  color: Signal<Color>,
  label_color: Signal<Option<Label>>,
  bare: Signal<Color>,
}

impl SwatchPickerPage {
  /// The playground: the picker as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let label = self.label.get();
    let (columns, size) = (self.columns.get(), self.size.get());
    let (framed, invalid, disabled, disabled_swatch) = (
      self.framed.get(),
      self.invalid.get(),
      self.disabled.get(),
      self.disabled_swatch.get(),
    );
    let chosen = self.played.get();
    let make = {
      let label = label.clone();
      move |value: Signal<Color>| {
        SwatchPicker::new(value, board(disabled_swatch.then_some(1)))
          .columns(columns)
          .size(size_of(size))
          .framed(framed)
          .invalid(invalid)
          .disabled(disabled)
          .label(&label)
      }
    };
    let record = self.playground.recorder();
    let picker = make(self.played.clone())
      .force(self.playground.forced())
      .on_change(move |color: &Color| record(format!("on_change({})", color.to_hex_upper())));

    let mut code = vec![
      "// color: Signal<Color>".to_owned(),
      "SwatchPicker::new(color, [\"#EF4444\", \"#F97316\", /* .. */ \"#18181B\"])".to_owned(),
    ];
    if columns != 6 {
      code.push(format!("    .columns({columns})"));
    }
    if size != 1 {
      code.push(format!("    .size(ColorSwatchSize::{:?})", size_of(size)));
    }
    if !framed {
      code.push("    .framed(false)".to_owned());
    }
    if invalid {
      code.push("    .invalid(true)".to_owned());
    }
    if disabled {
      code.push("    .disabled(true)".to_owned());
    }
    code.push(format!("    .label({label:?})"));
    code.push("    .on_change(|color: &Color| save(color))".to_owned());
    if disabled_swatch {
      code.push("// the second: Swatch::new(orange).disabled(true)".to_owned());
    }

    let side = [20, 24, 32][size.min(2)];
    let tree = vec![
      format!(
        "root      {}   {}",
        self.playground.measured(),
        if framed { "padded 10" } else { "bare" }
      ),
      format!("├─ row    {columns} swatches, 6 apart"),
      format!("│  └─ swatch  {side} × {side}   the chosen one ringed 2"),
      format!("└─ …      {} rows, 6 apart", BOARD.len().div_ceil(columns)),
    ];

    let (to_label, to_columns, to_size, to_framed) = (
      self.label.clone(),
      self.columns.clone(),
      self.size.clone(),
      self.framed.clone(),
    );
    let flags = [
      self.invalid.clone(),
      self.disabled.clone(),
      self.disabled_swatch.clone(),
    ];
    let (set_columns, set_size) = (self.columns.clone(), self.size.clone());
    let played = self.played.clone();
    self.playground.card(
      ctx,
      Spec {
        title: "SwatchPicker",
        make: Arc::new(move || make(Signal::new(chosen)).into()),
        shown: Shown::of(self.playground.styled(picker)),
        fill: false,
        forceable: true,
        facts: chosen.to_hex_upper(),
        event: "on_change",
        code,
        tree,
        controls: vec![
          prop("label", "&str", field(&self.label)),
          prop(
            "columns",
            "usize",
            self
              .playground
              .select(0, &COLUMNS, &columns, move |next| set_columns.set(next)),
          ),
          prop(
            "size",
            "ColorSwatchSize",
            self.playground.select(1, &SIZES, &size, move |next| set_size.set(next)),
          ),
          divider(),
          flag("framed", &self.framed),
          flag("invalid", &self.invalid),
          flag("disabled", &self.disabled),
          flag("a disabled swatch", &self.disabled_swatch),
        ],
        changed: [columns != 6, size != 1, !framed, invalid, disabled, disabled_swatch]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset: Arc::new(move || {
          to_label.set("Color".to_owned());
          to_columns.set(6);
          to_size.set(1);
          to_framed.set(true);
          played.set(Color::from_hex("#3B82F6"));
          for flag in &flags {
            flag.set(false);
          }
        }),
      },
    )
  }
}

impl Component for SwatchPickerPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 2),
      played: ctx.signal(Color::from_hex("#3B82F6")),
      label: ctx.signal("Color".to_owned()),
      columns: ctx.signal(6),
      size: ctx.signal(1),
      framed: ctx.signal(true),
      invalid: ctx.signal(false),
      disabled: ctx.signal(false),
      disabled_swatch: ctx.signal(false),
      color: ctx.signal(Color::from_hex("#3B82F6")),
      label_color: ctx.signal(None),
      bare: ctx.signal(Color::from_hex("#22C55E")),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let live = items().child(item(
      SwatchPicker::new(self.color.clone(), board(None)).label("Color"),
      &format!("the signal has: {}", self.color.get().to_hex_upper()),
    ));
    let named = items().child(item(
      SwatchPicker::optional(
        self.label_color.clone(),
        [
          Swatch::of(Label::Bug, Color::from_hex("#EF4444")).label("Bug"),
          Swatch::of(Label::Feature, Color::from_hex("#22C55E")).label("Feature"),
          Swatch::of(Label::Docs, Color::from_hex("#3B82F6")).label("Docs"),
          Swatch::of(Label::Question, Color::from_hex("#A855F7")).label("Question"),
        ],
      )
      .columns(4)
      .label("Label"),
      &format!("the signal has: {:?}", self.label_color.get()),
    ));
    let bare = items().child(item(
      SwatchPicker::new(self.bare.clone(), board(None))
        .framed(false)
        .size(ColorSwatchSize::Sm)
        .label("Color"),
      "framed(false): the grid alone",
    ));
    let state = |forced: Option<Interaction>, disabled: bool| {
      SwatchPicker::new(
        Signal::new(Color::from_hex("#3B82F6")),
        board(None).into_iter().take(12),
      )
      .disabled(disabled)
      .force(forced)
    };
    let states = items()
      .child(item(state(None, false), "the chosen one ringed"))
      .child(item(
        state(Some(Interaction::FocusVisible), false),
        "focused from the keyboard",
      ))
      .child(item(
        SwatchPicker::new(
          Signal::new(Color::from_hex("#EF4444")),
          board(Some(1)).into_iter().take(12),
        ),
        "a disabled swatch",
      ))
      .child(item(state(None, true), "disabled"))
      .child(item(
        SwatchPicker::new(
          Signal::new(Color::from_hex("#3B82F6")),
          board(None).into_iter().take(12),
        )
        .invalid(true),
        "invalid",
      ));

    let rows = [
      (
        "value",
        "Signal<T>",
        "",
        "What is chosen: the color of a swatch, or the app's value it carries. `optional(Signal<Option<T>>, ..)` may \
         hold none, `controlled(value, .., on_change)` is held by the app.",
      ),
      (
        "swatches",
        "impl IntoIterator<Item = impl Into<Swatch<T>>>",
        "",
        "The swatches: a `Color`, a hex (`\"#EF4444\"`), or a `Swatch` with a `label`, `disabled`, `id` and `key`; \
         `Swatch::of(value, color)` carries the app's value. `swatch` and `swatches` add more.",
      ),
      (
        "on_change",
        "impl Fn(&T)",
        "",
        "Told the value a person chooses; not for the one chosen already, nor when the app sets the signal.",
      ),
      (
        "columns",
        "usize",
        "6",
        "How many swatches a row holds. Up and Down move a row.",
      ),
      ("size", "ColorSwatchSize", "Md", "The swatches: Sm 20, Md 24, Lg 32."),
      (
        "framed",
        "bool",
        "true",
        "A panel of its own: the popover's fill, an edge, a shadow, padded 10. Out of it the grid alone.",
      ),
      (
        "label",
        "&str",
        "",
        "What the picker is called, to what reads the tree.",
      ),
      ("invalid", "bool", "false", "The panel's edge in the destructive color."),
      ("disabled", "bool", "false", "No swatch takes input; each is dimmed."),
      (
        "style / rules",
        "SwatchPickerPart",
        "",
        "Root, Row, Swatch (laid over each swatch's root).",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Inputs", "Swatch Picker"],
        title: "Swatch Picker",
        description: "One color chosen among swatches in a grid: the color of a label, of a highlight, of an event \
                      in a calendar.",
        import: Some("use lurq::ui::{Swatch, SwatchPicker};"),
        status: Some("Preview"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of SwatchPicker is a control on the right. The picker on the stage is live: a click, Enter or \
           Space chooses a swatch, the arrows move the focus in two directions.",
          playground,
        ),
        Section::new(
          "Swatch picker",
          "The board's panel of eighteen colors, bound to a `Signal<Color>`. Tab reaches the chosen swatch; Left \
           and Right move along a row and on to the next, Up and Down a row, Home and End to the first and the last.",
          self.frame.demo(
            0,
            live,
            "SwatchPicker::new(color, swatches)",
            &[
              "// color: Signal<Color>",
              "SwatchPicker::new(color, [",
              "    \"#EF4444\", \"#F97316\", \"#F59E0B\", \"#EAB308\", \"#84CC16\", \"#22C55E\",",
              "    \"#14B8A6\", \"#06B6D4\", \"#3B82F6\", \"#6366F1\", \"#A855F7\", \"#EC4899\",",
              "    \"#FAFAFA\", \"#D4D4D8\", \"#A1A1AA\", \"#71717A\", \"#3F3F46\", \"#18181B\",",
              "])",
              ".label(\"Color\")",
            ],
          ),
        ),
        Section::new(
          "The app's own values",
          "Each swatch carries a value of the app's, here the kind of a label, and the picker is bound to a \
           `Signal<Option<Label>>`: none is chosen until a person chooses.",
          self.frame.demo(
            1,
            named,
            "SwatchPicker::optional(label, [Swatch::of(Label::Bug, red), ..])",
            &[
              "// label: Signal<Option<Label>>",
              "SwatchPicker::optional(label, [",
              "    Swatch::of(Label::Bug, Color::from_hex(\"#EF4444\")).label(\"Bug\"),",
              "    Swatch::of(Label::Feature, Color::from_hex(\"#22C55E\")).label(\"Feature\"),",
              "    Swatch::of(Label::Docs, Color::from_hex(\"#3B82F6\")).label(\"Docs\"),",
              "    Swatch::of(Label::Question, Color::from_hex(\"#A855F7\")).label(\"Question\"),",
              "])",
              ".columns(4)",
              ".label(\"Label\")",
            ],
          ),
        ),
        Section::new(
          "The grid alone",
          "Out of its panel, for a popover or a color picker that frames it, with the small swatches.",
          self.frame.demo(
            2,
            bare,
            ".framed(false)",
            &[
              "SwatchPicker::new(color, swatches)",
              "    .framed(false)",
              "    .size(ColorSwatchSize::Sm)",
            ],
          ),
        ),
        Section::new(
          "States",
          "The board draws the chosen swatch ringed in the foreground. The ring of the focus is the theme's, on the \
           swatch Tab stops on; a disabled swatch is dimmed and passed by, a disabled picker dims every swatch, and \
           an invalid one has its edge in the destructive color.",
          self.frame.demo(
            3,
            states,
            ".force(Some(Interaction::FocusVisible))",
            &[
              "SwatchPicker::new(color, swatches)",
              "",
              "// Drawn focused from the keyboard with nothing focused: for a gallery, a test.",
              "SwatchPicker::new(color, swatches).force(Some(Interaction::FocusVisible))",
              "",
              "SwatchPicker::new(color, [Swatch::new(red), Swatch::new(orange).disabled(true), ..])",
              "SwatchPicker::new(color, swatches).disabled(true)",
              "SwatchPicker::new(color, swatches).invalid(true)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a swatch picker is given.",
          props_table(&rows, &["value", "swatches"], self.frame.room()),
        ),
      ],
    )
  }
}
