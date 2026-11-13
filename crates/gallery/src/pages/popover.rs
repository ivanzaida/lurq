//! The page of the popover: its playground, the popover of the design with a
//! field in it, the sides it is placed on, and a few settings in one.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Placement, Row},
  core::{ElementRef, Signal},
  layout::Alignment,
  node::Element,
  ui::{Button, ButtonVariant, Field, Input, Popover, Switch},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table},
  playground::{Playground, Spec, divider, flag, prop},
};

/// The sides a popover is placed on, as the playground offers them.
const PLACEMENTS: [(&str, Placement); 12] = [
  ("BottomStart", Placement::BottomStart),
  ("Bottom", Placement::Bottom),
  ("BottomEnd", Placement::BottomEnd),
  ("TopStart", Placement::TopStart),
  ("Top", Placement::Top),
  ("TopEnd", Placement::TopEnd),
  ("RightStart", Placement::RightStart),
  ("Right", Placement::Right),
  ("RightEnd", Placement::RightEnd),
  ("LeftStart", Placement::LeftStart),
  ("Left", Placement::Left),
  ("LeftEnd", Placement::LeftEnd),
];

/// The widths the playground offers: the design's in the middle.
const WIDTHS: [(&str, f32); 3] = [("240", 240.0), ("292", 292.0), ("340", 340.0)];

/// A popover opened and closed by an outlined button called `label`.
fn with_trigger(label: &str, anchor: &ElementRef, open: &Signal<bool>) -> Popover {
  Popover::new(anchor.clone(), open.clone()).trigger(Button::new(label).variant(ButtonVariant::Outline))
}

/// The popover of the design: what it is about, and a field to set it.
fn dimensions(anchor: &ElementRef, open: &Signal<bool>, width: &Signal<String>) -> Popover {
  with_trigger("Dimensions", anchor, open)
    .title("Dimensions")
    .description("Set the size of the layer.")
    .child(Field::new("Width", Input::new(width.clone()).full_width(true)).full_width(true))
}

pub struct PopoverPage {
  frame: Frame,
  playground: Playground,
  anchor: ElementRef,
  open: Signal<bool>,
  placement: Signal<Placement>,
  width: Signal<f32>,
  header: Signal<bool>,
  layer: Signal<String>,
  /// The popover of the design and the field in it.
  design: (ElementRef, Signal<bool>),
  design_width: Signal<String>,
  /// One popover on each side of its button.
  sides: [(ElementRef, Signal<bool>); 4],
  /// A few settings, each a switch.
  settings: (ElementRef, Signal<bool>),
  mentions: Signal<bool>,
  replies: Signal<bool>,
}

impl PopoverPage {
  /// The playground: the popover as the panel's props make it, and the button that opens it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (placement, width, header, open) = (
      self.placement.get(),
      self.width.get(),
      self.header.get(),
      self.open.get(),
    );
    let placement_name = PLACEMENTS
      .iter()
      .find_map(|(name, held)| (*held == placement).then_some(*name))
      .unwrap_or("BottomStart");
    let plain = move |anchor: &ElementRef, open: &Signal<bool>, layer: &Signal<String>, trigger: Button| {
      let mut popover = Popover::new(anchor.clone(), open.clone())
        .trigger(trigger)
        .placement(placement)
        .width(width)
        .child(Field::new("Width", Input::new(layer.clone()).full_width(true)).full_width(true));
      if header {
        popover = popover.title("Dimensions").description("Set the size of the layer.");
      }
      popover
    };
    let button = || Button::new("Dimensions").variant(ButtonVariant::Outline);
    // The trigger turns the signal; this handler of the page's is told of the press as well, before it.
    let (record, told) = (self.playground.recorder(), self.open.clone());
    let trigger = button().on_click(move |_| record(format!("open: {}", !told.get())));
    let shown = plain(&self.anchor, &self.open, &self.layer, trigger);

    let mut code = vec![
      "// anchor: ElementRef, open: Signal<bool>".to_owned(),
      "Popover::new(anchor.clone(), open.clone())".to_owned(),
      "    .trigger(Button::new(\"Dimensions\").variant(ButtonVariant::Outline))".to_owned(),
    ];
    if header {
      code.push("    .title(\"Dimensions\")".to_owned());
      code.push("    .description(\"Set the size of the layer.\")".to_owned());
    }
    if placement != Placement::BottomStart {
      code.push(format!("    .placement(Placement::{placement_name})"));
    }
    if width != 292.0 {
      code.push(format!("    .width({width:.1})"));
    }
    code.push("    .child(Field::new(\"Width\", Input::new(width).full_width(true)).full_width(true))".to_owned());

    let mut tree = vec![format!("trigger   {}", self.playground.measured())];
    tree.push(format!(
      "popover   {width} px wide   {}   {}",
      placement_name,
      if open { "open" } else { "closed" }
    ));
    if header {
      tree.push("├─ header".to_owned());
      tree.push("│  ├─ title         \"Dimensions\"".to_owned());
      tree.push("│  └─ description   \"Set the size of the layer.\"".to_owned());
    }
    tree.push("└─ body".to_owned());
    tree.push("   └─ Field \"Width\"".to_owned());

    let (to_placement, to_width, to_header, to_open) = (
      self.placement.clone(),
      self.width.clone(),
      self.header.clone(),
      self.open.clone(),
    );
    let (set_placement, set_width) = (self.placement.clone(), self.width.clone());
    self.playground.card(
      ctx,
      Spec {
        title: "Popover",
        make: Arc::new(move || {
          let (anchor, open, layer) = (ElementRef::new(), Signal::new(false), Signal::new("320px".to_owned()));
          plain(&anchor, &open, &layer, button()).into()
        }),
        shown: shown.into(),
        fill: false,
        forceable: false,
        facts: if open { "open".to_owned() } else { "closed".to_owned() },
        event: "open",
        code,
        tree,
        controls: vec![
          prop(
            "placement",
            "Placement",
            self.playground.select(0, &PLACEMENTS, &placement, move |placement| {
              set_placement.set(placement)
            }),
          ),
          prop(
            "width",
            "impl Into<Length>",
            self
              .playground
              .select(1, &WIDTHS, &width, move |width| set_width.set(width)),
          ),
          divider(),
          flag("title and description", &self.header),
          flag("open", &self.open),
        ],
        changed: [placement != Placement::BottomStart, width != 292.0, !header, open]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset: Arc::new(move || {
          to_placement.set(Placement::BottomStart);
          to_width.set(292.0);
          to_header.set(true);
          to_open.set(false);
        }),
      },
    )
  }
}

impl Component for PopoverPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 2),
      anchor: ElementRef::new(),
      open: ctx.signal(false),
      placement: ctx.signal(Placement::BottomStart),
      width: ctx.signal(292.0),
      header: ctx.signal(true),
      layer: ctx.signal("320px".to_owned()),
      design: (ElementRef::new(), ctx.signal(false)),
      design_width: ctx.signal("320px".to_owned()),
      sides: [
        (ElementRef::new(), ctx.signal(false)),
        (ElementRef::new(), ctx.signal(false)),
        (ElementRef::new(), ctx.signal(false)),
        (ElementRef::new(), ctx.signal(false)),
      ],
      settings: (ElementRef::new(), ctx.signal(false)),
      mentions: ctx.signal(true),
      replies: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let (anchor, open) = &self.design;
    let design = dimensions(anchor, open, &self.design_width);

    let mut sides = Row::new()
      .width(FILL)
      .padding_vertical(96.0)
      .spacing(16.0)
      .wrap()
      .align_items(Alignment::Center)
      .justify(lurq::layout::layout_kind::Justify::Center);
    let placements = [
      ("Top", Placement::Top),
      ("Right", Placement::Right),
      ("Bottom", Placement::Bottom),
      ("Left", Placement::Left),
    ];
    for ((name, placement), (anchor, open)) in placements.into_iter().zip(&self.sides) {
      let popover = with_trigger(name, anchor, open)
        .placement(placement)
        .width(200.0)
        .title(name)
        .description(&format!("Placement::{name}"));
      sides = sides.child(popover);
    }

    let (anchor, open) = &self.settings;
    let switch = |label: &str, on: &Signal<bool>| {
      let set = on.clone();
      Switch::new(label).on(on.get()).on_change(move |next| set.set(next))
    };
    let settings = with_trigger("Notifications", anchor, open)
      .title("Notifications")
      .description("What you are told of while you are away.")
      .child(switch("Mentions", &self.mentions))
      .child(switch("Replies to you", &self.replies));

    let rows = [
      (
        "anchor",
        "impl Into<ElementRef>",
        "",
        "The element the popover is shown beside. A press of it is not a press outside the popover.",
      ),
      (
        "open",
        "Signal<bool>",
        "",
        "Whether it is shown. The app's: a press outside it or Escape sets it to false.",
      ),
      (
        "trigger",
        "impl Into<Element>",
        "",
        "The button that opens and closes it: put before it, its anchor, and pressed to turn the signal.",
      ),
      ("title", "&str", "", "What the popover is about, at the top of it."),
      (
        "description",
        "&str",
        "",
        "What it is for, under the title, in the muted color.",
      ),
      (
        "child",
        "impl Into<Element>",
        "",
        "Something it holds, under the header: a field, a row of settings, anything.",
      ),
      (
        "placement",
        "Placement",
        "BottomStart",
        "The side of the anchor it is on, and the edge it lines up with. It goes to the other side where there is no room.",
      ),
      ("width", "impl Into<Length>", "292", "How wide the panel is."),
      (
        "offset",
        "impl Into<SpacingValue>",
        "SpacingSize::Xs",
        "How far it is from its anchor: 4 in the design's theme.",
      ),
      (
        "collision",
        "CollisionStrategy",
        "FlipThenClamp",
        "What it does where there is no room on its side.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Surfaces & overlays", "Popover"],
        title: "Popover",
        description: "A panel beside the button that opens it, over the window, holding anything: the size of a \
                      layer, a few settings, a form. A press outside it or Escape closes it.",
        import: Some("use lurq::ui::Popover;"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Popover is a control on the right. The button on the stage opens it and the focus goes into \
           it; a press outside it or Escape closes it, the focus goes back to the button, and the open switch \
           follows.",
          playground,
        ),
        Section::new(
          "Popover",
          "The popover of the design: a title, a description and a field. The page holds whether it is open, in a \
           signal, which its trigger turns. It fades in, and the field takes the focus.",
          self.frame.demo(
            0,
            design,
            "Popover::new(anchor, open).trigger(..).title(..).description(..).child(..)",
            &[
              "// anchor: ElementRef, open: Signal<bool>, width: Signal<String>",
              "Popover::new(anchor.clone(), open.clone())",
              "    .trigger(Button::new(\"Dimensions\").variant(ButtonVariant::Outline))",
              "    .title(\"Dimensions\")",
              "    .description(\"Set the size of the layer.\")",
              "    .child(Field::new(\"Width\", Input::new(width).full_width(true)).full_width(true))",
            ],
          ),
        ),
        Section::new(
          "Placement",
          "Over, beside or under its button, 4 from it. Where there is no room on its side it goes to the other.",
          self.frame.demo(
            1,
            sides,
            ".placement(Placement::Top)",
            &[
              "Popover::new(anchor.clone(), open.clone())",
              "    .trigger(Button::new(\"Top\").variant(ButtonVariant::Outline))",
              "    .placement(Placement::Top)",
              "    .width(200.0)",
              "    .title(\"Top\")",
              "    .description(\"Placement::Top\")",
            ],
          ),
        ),
        Section::new(
          "Settings",
          "What it holds is anything: here two switches, each bound to a signal of the page.",
          self.frame.demo(
            2,
            settings,
            ".child(Switch::new(\"Mentions\"))",
            &[
              "// mentions, replies: Signal<bool>",
              "Popover::new(anchor.clone(), open.clone())",
              "    .trigger(Button::new(\"Notifications\").variant(ButtonVariant::Outline))",
              "    .title(\"Notifications\")",
              "    .description(\"What you are told of while you are away.\")",
              "    .child(Switch::new(\"Mentions\").on(mentions.get()).on_change(..))",
              "    .child(Switch::new(\"Replies to you\").on(replies.get()).on_change(..))",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a popover is given.",
          props_table(&rows, &["anchor", "open"], self.frame.room()),
        ),
      ],
    )
  }
}
