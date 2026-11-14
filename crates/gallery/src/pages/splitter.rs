//! The page of the splitter: its playground, a window's panes with a layout
//! the page saves, the board's Resizable with its grips, and the divider at
//! rest, under the pointer and dragged.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx, events::MouseEvent},
  components::{Column, Row},
  core::Signal,
  layout::{Alignment, layout_kind::Justify},
  node::Element,
  ui::{Button, ButtonSize, ButtonVariant, Interaction, Splitter, SplitterOrientation, SplitterPane},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Spec, divider, flag, prop},
};

/// What a pane shows: its name in the middle of it.
fn label(name: &str) -> Element {
  Column::new()
    .size(FILL, FILL)
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .child(quiet(name, 12.0))
    .into()
}

/// The orientations, the least sizes and the most sizes of the side pane the playground offers.
const ORIENTATIONS: [(&str, SplitterOrientation); 2] = [
  ("Horizontal", SplitterOrientation::Horizontal),
  ("Vertical", SplitterOrientation::Vertical),
];
const MINS: [(&str, i32); 3] = [("80", 80), ("120", 120), ("0", 0)];
const MAXES: [(&str, i32); 3] = [("320", 320), ("240", 240), ("none", 0)];

/// What a stage holds a splitter in: as wide as the stage, and so high.
fn room(splitter: impl Into<Element>, height: f32) -> Column {
  Column::new()
    .width(FILL)
    .height(height)
    .align_items(Alignment::Stretch)
    .overflow_visible()
    .child(splitter)
}

pub struct SplitterPage {
  frame: Frame,
  playground: Playground,
  orientation: Signal<SplitterOrientation>,
  min: Signal<i32>,
  max: Signal<i32>,
  collapsible: Signal<bool>,
  nested: Signal<bool>,
  grip: Signal<bool>,
  framed: Signal<bool>,
  disabled: Signal<bool>,
  /// The layout of the window's panes, as the page saves it.
  layout: Signal<Vec<f32>>,
}

impl SplitterPage {
  /// The splitter as the panel's props make it.
  fn plain(&self) -> impl Fn() -> Splitter + Send + Sync + Clone + 'static {
    let orientation = self.orientation.get();
    let (min, max) = (self.min.get(), self.max.get());
    let (collapsible, nested) = (self.collapsible.get(), self.nested.get());
    let (grip, framed, disabled) = (self.grip.get(), self.framed.get(), self.disabled.get());
    move || {
      let mut side = SplitterPane::new(label("Files"))
        .size(180.0)
        .min(min as f32)
        .collapsible(collapsible)
        .label("Files");
      if max > 0 {
        side = side.max(max as f32);
      }
      let middle = if nested {
        let across = match orientation {
          SplitterOrientation::Horizontal => Splitter::vertical(),
          SplitterOrientation::Vertical => Splitter::horizontal(),
        };
        SplitterPane::new(
          across
            .grip(grip)
            .disabled(disabled)
            .pane(SplitterPane::new(label("Editor")).min(60.0).label("Editor"))
            .pane(
              SplitterPane::new(label("Terminal"))
                .size(90.0)
                .min(60.0)
                .collapsible(true)
                .label("Terminal"),
            ),
        )
        .min(120.0)
      } else {
        SplitterPane::new(label("Editor")).min(120.0).label("Editor")
      };
      Splitter::new(orientation)
        .grip(grip)
        .framed(framed)
        .disabled(disabled)
        .pane(side)
        .pane(middle)
        .pane(
          SplitterPane::new(label("Preview"))
            .share(0.25)
            .min(80.0)
            .label("Preview"),
        )
    }
  }

  /// The playground: the splitter as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let orientation = self.orientation.get();
    let (min, max) = (self.min.get(), self.max.get());
    let (collapsible, nested) = (self.collapsible.get(), self.nested.get());
    let (grip, framed, disabled) = (self.grip.get(), self.framed.get(), self.disabled.get());
    let vertical = orientation == SplitterOrientation::Vertical;
    let plain = self.plain();
    let record = self.playground.recorder();
    // A change of a prop lays the panes out again from what they are given.
    let key = format!("{vertical}-{min}-{max}-{collapsible}-{nested}-{grip}-{framed}-{disabled}");
    let splitter = plain()
      .key(key)
      .force(self.playground.forced())
      .on_resize_end(move |sizes| {
        let sizes: Vec<String> = sizes.iter().map(|size| format!("{size:.0}")).collect();
        record(format!("on_resize_end([{}])", sizes.join(", ")));
      })
      .mount(ctx);

    let mut code = vec![format!(
      "Splitter::{}()",
      if vertical { "vertical" } else { "horizontal" }
    )];
    if grip {
      code.push("    .grip(true)".to_owned());
    }
    if framed {
      code.push("    .framed(true)".to_owned());
    }
    if disabled {
      code.push("    .disabled(true)".to_owned());
    }
    let mut side = format!("    .pane(SplitterPane::new(files).size(180.0).min({min}.0)");
    if max > 0 {
      side.push_str(&format!(".max({max}.0)"));
    }
    if collapsible {
      side.push_str(".collapsible(true)");
    }
    side.push(')');
    code.push(side);
    if nested {
      code.push("    .pane(SplitterPane::new(".to_owned());
      code.push(format!(
        "        Splitter::{}()",
        if vertical { "horizontal" } else { "vertical" }
      ));
      code.push("            .pane(SplitterPane::new(editor).min(60.0))".to_owned());
      code.push("            .pane(SplitterPane::new(terminal).size(90.0).min(60.0).collapsible(true)),".to_owned());
      code.push("    ).min(120.0))".to_owned());
    } else {
      code.push("    .pane(SplitterPane::new(editor).min(120.0))".to_owned());
    }
    code.push("    .pane(SplitterPane::new(preview).share(0.25).min(80.0))".to_owned());
    code.push("    .on_resize_end(|sizes| save(sizes))".to_owned());
    code.push("    .mount(ctx)".to_owned());

    let line = if vertical { "400 × 1" } else { "1 × fill" };
    let mut tree = vec![
      format!("root        {}", self.playground.measured()),
      "├─ pane      Files     180 px".to_owned(),
      format!("├─ handle    12 across, line {line}"),
      format!(
        "├─ pane      Editor    the rest{}",
        if nested { ", a splitter across" } else { "" }
      ),
      "├─ handle".to_owned(),
      "└─ pane      Preview   a quarter".to_owned(),
    ];
    if grip {
      tree.push("   grip      12 × 20 on each line".to_owned());
    }

    let (set_orientation, set_min, set_max) = (self.orientation.clone(), self.min.clone(), self.max.clone());
    let flags = [
      self.collapsible.clone(),
      self.nested.clone(),
      self.grip.clone(),
      self.framed.clone(),
      self.disabled.clone(),
    ];
    let reset = {
      let (orientation, min, max) = (self.orientation.clone(), self.min.clone(), self.max.clone());
      Arc::new(move || {
        orientation.set(SplitterOrientation::Horizontal);
        min.set(80);
        max.set(320);
        for flag in &flags {
          flag.set(false);
        }
      })
    };
    let make = {
      let plain = plain.clone();
      Arc::new(move || room(plain(), 360.0).into())
    };
    self.playground.card(
      ctx,
      Spec {
        title: "Splitter",
        make,
        shown: room(splitter, 300.0).into(),
        fill: true,
        forceable: true,
        facts: format!("3 panes{}", if nested { ", one split across" } else { "" }),
        event: "on_resize_end",
        code,
        tree,
        controls: vec![
          prop(
            "orientation",
            "SplitterOrientation",
            self
              .playground
              .select(0, &ORIENTATIONS, &orientation, move |orientation| {
                set_orientation.set(orientation)
              }),
          ),
          prop(
            "min",
            "f32 (Files)",
            self.playground.select(1, &MINS, &min, move |min| set_min.set(min)),
          ),
          prop(
            "max",
            "f32 (Files)",
            self.playground.select(2, &MAXES, &max, move |max| set_max.set(max)),
          ),
          divider(),
          flag("collapsible", &self.collapsible),
          flag("nested", &self.nested),
          flag("grip", &self.grip),
          flag("framed", &self.framed),
          flag("disabled", &self.disabled),
        ],
        changed: [
          vertical,
          min != 80,
          max != 320,
          collapsible,
          nested,
          grip,
          framed,
          disabled,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset,
      },
    )
  }

  /// A window's panes: a side bar that closes, an editor over a terminal, and an outline; the page keeps the layout.
  fn window(&self, ctx: &mut Ctx) -> Element {
    let splitter = Splitter::horizontal()
      .sizes(self.layout.clone())
      .pane(
        SplitterPane::new(label("Explorer"))
          .size(200.0)
          .min(140.0)
          .max(360.0)
          .collapsible(true)
          .label("Explorer"),
      )
      .pane(
        SplitterPane::new(
          Splitter::vertical()
            .key("window-editor")
            .pane(SplitterPane::new(label("lib.rs")).min(80.0).label("Editor"))
            .pane(
              SplitterPane::new(label("Terminal"))
                .size(110.0)
                .min(60.0)
                .collapsible(true)
                .label("Terminal"),
            )
            .mount(ctx),
        )
        .min(200.0),
      )
      .pane(
        SplitterPane::new(label("Outline"))
          .share(0.2)
          .min(100.0)
          .label("Outline"),
      )
      .mount(ctx);
    let reset = {
      let layout = self.layout.clone();
      move |_: MouseEvent| layout.set(Vec::new())
    };
    Column::new()
      .width(FILL)
      .spacing(12.0)
      .child(
        Column::new()
          .width(FILL)
          .height(320.0)
          .align_items(Alignment::Stretch)
          .child(splitter),
      )
      .child(
        Row::new().child(
          Button::new("Reset layout")
            .variant(ButtonVariant::Outline)
            .size(ButtonSize::Sm)
            .on_click(reset),
        ),
      )
      .into()
  }

  /// The board's Resizable: a box with a side bar, and an editor over a terminal, with a grip on each divider.
  fn resizable(ctx: &mut Ctx) -> Element {
    let splitter = Splitter::horizontal()
      .grip(true)
      .framed(true)
      .pane(SplitterPane::new(label("Sidebar")).size(140.0).min(100.0))
      .pane(SplitterPane::new(
        Splitter::vertical()
          .grip(true)
          .pane(SplitterPane::new(label("Editor")).min(60.0))
          .pane(SplitterPane::new(label("Terminal")).size(60.0).min(40.0)),
      ))
      .mount(ctx);
    Row::new()
      .child(
        Column::new()
          .width(420.0)
          .height(180.0)
          .align_items(Alignment::Stretch)
          .child(splitter),
      )
      .into()
  }

  /// The divider at rest, under the pointer and dragged, in boxes of the board's specimen.
  fn states() -> Element {
    let specimen = |interaction: Option<Interaction>, caption: &str| {
      Column::new()
        .spacing(10.0)
        .child(
          Column::new()
            .width(130.0)
            .height(84.0)
            .align_items(Alignment::Stretch)
            .child(
              Splitter::horizontal()
                .framed(true)
                .force(interaction)
                .pane(SplitterPane::new(label("A")))
                .pane(SplitterPane::new(label("B")).share(0.5)),
            ),
        )
        .child(quiet(caption, 12.0))
    };
    Row::new()
      .width(FILL)
      .spacing(24.0)
      .wrap()
      .child(specimen(None, "idle"))
      .child(specimen(Some(Interaction::Hovered), "hover"))
      .child(specimen(Some(Interaction::Pressed), "dragging"))
      .child(specimen(Some(Interaction::FocusVisible), "focus-visible"))
      .into()
  }
}

impl Component for SplitterPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 3),
      orientation: ctx.signal(SplitterOrientation::Horizontal),
      min: ctx.signal(80),
      max: ctx.signal(320),
      collapsible: ctx.signal(false),
      nested: ctx.signal(false),
      grip: ctx.signal(false),
      framed: ctx.signal(false),
      disabled: ctx.signal(false),
      // Not a signal of the page's: the page is not rendered again as a divider moves.
      layout: Signal::new(Vec::new()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let rows = [
      (
        "horizontal / vertical",
        "Splitter",
        "",
        "Panes side by side, or one over the other. Splitter::new(orientation) takes it as a value.",
      ),
      (
        "pane",
        "SplitterPane",
        "",
        "A pane after the others, with a divider before it.",
      ),
      (
        "sizes",
        "Signal<Vec<f32>>",
        "its own",
        "The sizes of the panes in pixels, as a divider last left them: the app saves them and sets them back.",
      ),
      (
        "on_resize",
        "impl Fn(&[f32])",
        "",
        "Called with the sizes each time a divider moves.",
      ),
      (
        "on_resize_end",
        "impl Fn(&[f32])",
        "",
        "Called with the sizes once a divider has moved: let go, or moved by a key.",
      ),
      ("step", "f32", "10", "What an arrow key moves a divider by."),
      ("large_step", "f32", "50", "What an arrow key moves it by with Shift."),
      (
        "grip",
        "bool",
        "false",
        "A grip in the middle of each divider: the board's Resizable.",
      ),
      (
        "framed",
        "bool",
        "false",
        "A box of its own with a fill, an edge and round corners: the board's specimen.",
      ),
      ("disabled", "bool", "false", "The dividers take no input."),
      (
        "SplitterPane::size",
        "f32",
        "",
        "Pixels to begin with, kept as the splitter grows and shrinks.",
      ),
      (
        "SplitterPane::share",
        "f32",
        "",
        "A share of the splitter to begin with, from 0 to 1, kept as it grows and shrinks. A pane with neither \
         takes the rest.",
      ),
      (
        "SplitterPane::min / max",
        "f32",
        "0 / none",
        "The least and the most the pane is.",
      ),
      (
        "SplitterPane::collapsible",
        "bool",
        "false",
        "It closes under half its least size, and opens by a drag, a double press or Enter.",
      ),
      (
        "SplitterPane::label",
        "&str",
        "",
        "What the pane is called; the divider after it is called so to tooling.",
      ),
    ];
    let playground = self.play(ctx);
    let window = self.window(ctx);
    let resizable = Self::resizable(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Desktop", "Splitter"],
        title: "Splitter",
        description: "Panes that share a room by the dividers between them, which a person drags: the side bar, \
                      the editor and the terminal of a window.",
        import: Some("use lurq::ui::{Splitter, SplitterPane};"),
        status: Some("Beta"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop is a control on the right. Drag a divider, or press it twice to put it back; Tab reaches it, \
           the arrow keys move it, with Shift by more, Home and End take the pane before it to its least and most, \
           and Enter closes the pane that closes.",
          playground,
        ),
        Section::new(
          "A window's panes",
          "A side bar of pixels that closes, an editor over a terminal, and an outline that keeps a fifth of the \
           window. The page keeps the layout in a signal, which it would save; Reset layout empties it, and the \
           panes go back to what they were given.",
          self.frame.demo(
            0,
            window,
            ".sizes(layout.clone())",
            &[
              "// layout: Signal<Vec<f32>>, saved with the window",
              "Splitter::horizontal()",
              "    .sizes(layout.clone())",
              "    .pane(SplitterPane::new(explorer).size(200.0).min(140.0).max(360.0).collapsible(true))",
              "    .pane(",
              "        SplitterPane::new(",
              "            Splitter::vertical()",
              "                .pane(SplitterPane::new(editor).min(80.0))",
              "                .pane(SplitterPane::new(terminal).size(110.0).min(60.0).collapsible(true))",
              "                .mount(ctx),",
              "        )",
              "        .min(200.0),",
              "    )",
              "    .pane(SplitterPane::new(outline).share(0.2).min(100.0))",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Resizable",
          "The board's Resizable is a splitter with a grip on each divider, in a box of its own.",
          self.frame.demo(
            1,
            resizable,
            ".grip(true)",
            &[
              "Splitter::horizontal()",
              "    .grip(true)",
              "    .framed(true)",
              "    .pane(SplitterPane::new(sidebar).size(140.0).min(100.0))",
              "    .pane(SplitterPane::new(",
              "        Splitter::vertical()",
              "            .grip(true)",
              "            .pane(SplitterPane::new(editor).min(60.0))",
              "            .pane(SplitterPane::new(terminal).size(60.0).min(40.0)),",
              "    ))",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "States",
          "At rest the divider is a line of 1 in the color of an edge. Under the pointer and focused from the \
           keyboard it is 3 wide in the ring's color, and while it is dragged in the primary color.",
          self.frame.demo(
            2,
            Self::states(),
            ".force(Some(Interaction::Hovered))",
            &[
              "Splitter::horizontal()",
              "    .framed(true)",
              "    .pane(SplitterPane::new(a))",
              "    .pane(SplitterPane::new(b).share(0.5))",
              "",
              "// Drawn as under the pointer, dragged or focused, with nothing pointing at it.",
              "    .force(Some(Interaction::Hovered))",
              "    .force(Some(Interaction::Pressed))",
              "    .force(Some(Interaction::FocusVisible))",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a splitter and its panes are given.",
          props_table(&rows, &["pane"], self.frame.room()),
        ),
      ],
    )
  }
}
