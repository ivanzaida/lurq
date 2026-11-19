//! The page of the info label: its playground, the board's label with its
//! tooltip, the same mark after a field's label, the sides the tooltip takes,
//! and a label longer than its room.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Placement, Row},
  core::Signal,
  layout::{Alignment, layout_kind::Justify},
  node::Element,
  ui::{Field, InfoLabel, Input},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, card, props_table, quiet, segmented_fill},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

/// The sides the playground offers.
const PLACEMENTS: [(&str, Placement); 4] = [
  ("TopStart", Placement::TopStart),
  ("Top", Placement::Top),
  ("Bottom", Placement::Bottom),
  ("Right", Placement::Right),
];

/// Room around what is on a stage, for the tooltip over it.
fn room(content: impl Into<Element>) -> Row {
  Row::new()
    .width(FILL)
    .padding_vertical(56.0)
    .spacing(64.0)
    .wrap()
    .justify(Justify::Center)
    .align_items(Alignment::Center)
    .overflow_visible()
    .child(content)
}

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .max_width(FILL)
    .spacing(9.0)
    .overflow_visible()
    .child(Row::new().max_width(FILL).overflow_visible().child(shown))
    .child(quiet(caption, 12.0))
}

/// The props of the playground's label.
#[derive(Clone)]
struct Props {
  label: String,
  info: String,
  placement: usize,
  arrow: bool,
  open: bool,
}

impl Props {
  fn label(&self) -> InfoLabel {
    InfoLabel::new(&self.label)
      .info(&self.info)
      .placement(PLACEMENTS[self.placement].1)
      .arrow(self.arrow)
      .force_open(self.open.then_some(true))
  }

  fn code(&self) -> Vec<String> {
    let mut code = vec![format!("InfoLabel::new({:?})", self.label)];
    if !self.info.is_empty() {
      code.push(format!("    .info({:?})", self.info));
    }
    if self.placement != 0 {
      code.push(format!("    .placement(Placement::{})", PLACEMENTS[self.placement].0));
    }
    if self.arrow {
      code.push("    .arrow(true)".to_owned());
    }
    if self.open {
      code.push("    .force_open(Some(true))".to_owned());
    }
    code.push("    .mount(ctx)".to_owned());
    code
  }

  fn tree(&self, measured: String) -> Vec<String> {
    let mut tree = vec![format!("root      {measured}")];
    if self.info.is_empty() {
      tree.push(format!("└─ label  {:?}", self.label));
    } else {
      tree.push(format!("├─ label  {:?}", self.label));
      tree.push(format!(
        "└─ mark   \"i\" 14, a stop of Tab   img \"More information\", described by {:?}",
        self.info
      ));
      tree.push(format!(
        "tooltip   {}, over the window   {:?}",
        PLACEMENTS[self.placement].0, self.info
      ));
    }
    tree
  }
}

pub struct InfoLabelPage {
  frame: Frame,
  playground: Playground,
  label: Signal<String>,
  info: Signal<String>,
  /// The placement, as its index in [`PLACEMENTS`].
  placement: Signal<usize>,
  arrow: Signal<bool>,
  open: Signal<bool>,
  key: Signal<String>,
}

impl InfoLabelPage {
  fn props(&self) -> Props {
    Props {
      label: self.label.get(),
      info: self.info.get(),
      placement: self.placement.get().min(PLACEMENTS.len() - 1),
      arrow: self.arrow.get(),
      open: self.open.get(),
    }
  }

  /// The playground: the label as the panel's props make it, mounted, with room for its tooltip.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let props = self.props();
    let placement = self.placement.clone();
    let choices: Vec<(&str, usize)> = PLACEMENTS
      .iter()
      .enumerate()
      .map(|(index, (name, _))| (*name, index))
      .collect();
    let controls = vec![
      prop("label", "&str", field(&self.label)),
      prop("info", "&str", field(&self.info)),
      prop(
        "placement",
        "Placement",
        segmented_fill(&choices, &props.placement, move |chosen| placement.set(chosen)),
      ),
      divider(),
      flag("arrow", &self.arrow),
      flag("force open", &self.open),
    ];
    let changed = [
      props.label != "API key",
      props.info != "Stored in the OS keychain",
      props.placement != 0,
      props.arrow,
      !props.open,
    ]
    .into_iter()
    .filter(|changed| *changed)
    .count();
    let reset = {
      let (label, info) = (self.label.clone(), self.info.clone());
      let (placement, arrow, open) = (self.placement.clone(), self.arrow.clone(), self.open.clone());
      Arc::new(move || {
        label.set("API key".to_owned());
        info.set("Stored in the OS keychain".to_owned());
        placement.set(0);
        arrow.set(false);
        open.set(true);
      })
    };
    {
      let props = props.clone();
      self
        .playground
        .window_shows(move |ctx| room(props.label().mount(ctx)).into());
    }
    let shown = {
      let (props, playground) = (props.clone(), &self.playground);
      Shown::built(move |ctx| room(playground.measure(playground.styled(props.label()).mount(ctx))).into()).measured()
    };
    self.playground.card(
      ctx,
      Spec {
        title: "Info Label",
        make: {
          let props = props.clone();
          Arc::new(move || props.label().into())
        },
        shown,
        fill: true,
        forceable: false,
        facts: if props.open {
          "tooltip forced open".to_owned()
        } else {
          "tooltip on hover and focus".to_owned()
        },
        event: "",
        code: props.code(),
        tree: props.tree(self.playground.measured()),
        controls,
        changed,
        reset,
      },
    )
  }
}

impl Component for InfoLabelPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      label: ctx.signal("API key".to_owned()),
      info: ctx.signal("Stored in the OS keychain".to_owned()),
      placement: ctx.signal(0),
      arrow: ctx.signal(false),
      open: ctx.signal(true),
      key: ctx.signal(String::new()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let board = room(
      InfoLabel::new("API key")
        .info("Stored in the OS keychain")
        .force_open(Some(true))
        .mount(ctx),
    );
    let live = room(item(
      InfoLabel::new("Region")
        .info("Where the data of the workspace is kept")
        .mount(ctx),
      "rest the pointer on it, or Tab to its mark",
    ));
    let on_field = Column::new()
      .width(FILL)
      .padding_vertical(40.0)
      .align_items(Alignment::Center)
      .overflow_visible()
      .child(
        Field::new(
          "API key",
          Input::new(self.key.clone()).placeholder("sk-…").full_width(true),
        )
        .info("Stored in the OS keychain")
        .required(true)
        .description("Paste the key of the service account.")
        .mount(ctx),
      );
    let sides = [
      ("top start", Placement::TopStart),
      ("bottom", Placement::Bottom),
      ("right", Placement::Right),
    ]
    .into_iter()
    .fold(
      Row::new()
        .width(FILL)
        .padding_vertical(64.0)
        .spacing(96.0)
        .wrap()
        .justify(Justify::Center)
        .overflow_visible(),
      |row, (name, placement)| {
        row.child(
          InfoLabel::new(name)
            .info("Kept here")
            .placement(placement)
            .arrow(true)
            .force_open(Some(true))
            .mount(ctx),
        )
      },
    );
    let narrow = card()
      .width(200.0)
      .max_width(FILL)
      .padding(12.0)
      .overflow_visible()
      .child(
        InfoLabel::new("The key of the service account the nightly export signs with")
          .info("Stored in the OS keychain")
          .mount(ctx),
      );
    let rows = [
      ("label", "&str", "", "The text of the label."),
      (
        "info",
        "&str",
        "",
        "What the mark explains, in a few words: the text of its tooltip. Without one there is no mark.",
      ),
      (
        "info_label",
        "&str",
        "\"More information\"",
        "What the mark is called to tooling.",
      ),
      (
        "icon",
        "impl Into<Name>",
        "info",
        "The icon of the mark, by its name in the theme's set.",
      ),
      (
        "placement",
        "Placement",
        "TopStart",
        "Which side of the label the tooltip is on, and which edge of it it lines up with.",
      ),
      (
        "offset, collision, arrow",
        "",
        "",
        "How far the tooltip stands, what it does with no room on its side, and whether it has an arrow: a \
         tooltip's own.",
      ),
      (
        "force_open",
        "Option<bool>",
        "None",
        "The tooltip shown whatever the pointer and the focus do, or never.",
      ),
      (
        "mount",
        "&mut Ctx",
        "",
        "Puts the label where it goes with its tooltip on it. Converted into an element, it is the label and \
         the mark with nothing that shows.",
      ),
      (
        "style, rules, state, force",
        "",
        "",
        "Over the theme's recipe: InfoLabelPart::Root, Label, Mark. force draws the mark hovered or focused.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Inputs", "Info Label"],
        title: "Info Label",
        description: "What a field or a setting is called, with a mark after it that explains it in a tooltip.",
        import: Some("use lurq::ui::InfoLabel;"),
        status: Some("New"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of InfoLabel is a control on the right. The tooltip is forced open on the stage; without \
           that it shows while the pointer rests on the label and when Tab comes to the mark.",
          playground,
        ),
        Section::new(
          "With a tooltip",
          "A field label's text with an \"i\" of 14 in the muted color 6 after it, and the tooltip above them, \
           from the label's start.",
          self.frame.demo(
            0,
            board,
            ".info(\"Stored in the OS keychain\")",
            &[
              "InfoLabel::new(\"API key\")",
              "    .info(\"Stored in the OS keychain\")",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Pointer and keyboard",
          "The tooltip shows after the pointer has rested on the label or its mark, and at once when the \
           keyboard's focus comes to the mark, which is a stop of Tab. Escape puts it away.",
          self.frame.demo(
            1,
            live,
            "InfoLabel::new(\"Region\").info(..).mount(ctx)",
            &[
              "InfoLabel::new(\"Region\")",
              "    .info(\"Where the data of the workspace is kept\")",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "On a field",
          "A field draws the same mark after its own label, and after the mark of a required field. The mark is \
           the first stop of Tab in the field, before its control.",
          self.frame.demo(
            2,
            on_field,
            "Field::new(..).info(..).mount(ctx)",
            &[
              "Field::new(\"API key\", Input::new(key).full_width(true))",
              "    .info(\"Stored in the OS keychain\")",
              "    .required(true)",
              "    .description(\"Paste the key of the service account.\")",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Sides",
          "The tooltip takes a placement, an offset and an arrow as any tooltip does.",
          self.frame.demo(
            3,
            sides,
            ".placement(Placement::Bottom).arrow(true)",
            &[
              "InfoLabel::new(\"bottom\")",
              "    .info(\"Kept here\")",
              "    .placement(Placement::Bottom)",
              "    .arrow(true)",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "In a narrow room",
          "A label longer than its room ends in an ellipsis, and the mark keeps its place after it.",
          self.frame.demo(
            4,
            narrow,
            "InfoLabel::new(a long label)",
            &[
              "// In a card 200 wide.",
              "InfoLabel::new(\"The key of the service account the nightly export signs with\")",
              "    .info(\"Stored in the OS keychain\")",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What an info label is given.",
          props_table(&rows, &["label"], self.frame.room()),
        ),
      ],
    )
  }
}
