//! The page of the toggletip: its playground, one beside a field's label,
//! one with a button in it, and the sides with an arrow.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Placement, Row},
  core::Signal,
  layout::{Alignment, layout_kind::Justify},
  node::Element,
  ui::{Button, ButtonSize, ButtonVariant, Toggletip},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, text},
  playground::{Playground, Spec, divider, field, flag, prop},
};

/// The sides the playground offers.
const PLACEMENTS: [(&str, Placement); 4] = [
  ("Top", Placement::Top),
  ("Bottom", Placement::Bottom),
  ("Right", Placement::Right),
  ("Left", Placement::Left),
];

/// A label and the toggletip that says more about it, side by side.
fn labelled(label: &str, tip: impl Into<Element>) -> Row {
  Row::new()
    .align_items(Alignment::Center)
    .spacing(4.0)
    .child(text(label, 14.0, lurq::layout::text_style::FontWeight::Medium))
    .child(tip)
}

/// Room around what is on a stage, for the panel over it.
fn room(content: impl Into<Element>) -> Row {
  Row::new()
    .width(FILL)
    .padding_vertical(96.0)
    .spacing(48.0)
    .wrap()
    .justify(Justify::Center)
    .align_items(Alignment::Center)
    .child(content)
}

pub struct ToggletipPage {
  frame: Frame,
  playground: Playground,
  label: Signal<String>,
  content: Signal<String>,
  link: Signal<bool>,
  /// The placement, as its index in [`PLACEMENTS`].
  placement: Signal<usize>,
  arrow: Signal<bool>,
  open: Signal<bool>,
  disabled: Signal<bool>,
}

impl ToggletipPage {
  /// The playground: a label and its toggletip as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (label, content, link, chosen) = (
      self.label.get(),
      self.content.get(),
      self.link.get(),
      self.placement.get(),
    );
    let (placement_name, placement) = PLACEMENTS[chosen.min(PLACEMENTS.len() - 1)];
    let (arrow, open, disabled) = (self.arrow.get(), self.open.get(), self.disabled.get());
    let record = self.playground.recorder();
    let plain = {
      let (label, content) = (label.clone(), content.clone());
      move || {
        let mut tip = Toggletip::new(&label)
          .content(&content)
          .placement(placement)
          .arrow(arrow)
          .disabled(disabled);
        if link {
          let record = record.clone();
          tip = tip.link("Learn more", move |_| record("pressed: Learn more".to_owned()));
        }
        tip
      }
    };
    let shown = labelled("Pixel size", plain().open(open.then_some(true)).mount(ctx));

    let mut code = vec![
      format!("Toggletip::new({label:?})"),
      format!("    .content({content:?})"),
    ];
    if link {
      code.push("    .link(\"Learn more\", |_| open_docs())".to_owned());
    }
    if placement != Placement::Top {
      code.push(format!("    .placement(Placement::{placement_name})"));
    }
    if arrow {
      code.push("    .arrow(true)".to_owned());
    }
    if open {
      code.push("    .open(Some(true))".to_owned());
    }
    if disabled {
      code.push("    .disabled(true)".to_owned());
    }
    code.push("    .mount(ctx)".to_owned());

    let mut tree = vec![
      format!("trigger   a ghost button of 20, a circle, \"i\" 14   {label:?}, expanded: {open}"),
      format!("root      6 from it, {}", placement_name.to_lowercase()),
      format!(
        "{} panel  230 wide, primary, radius 6, 12 padding, 8 apart   a status",
        if arrow { "├─" } else { "└─" }
      ),
    ];
    let bar = if arrow { "│" } else { " " };
    tree.push(format!(
      "{bar}  {} text   13, primary foreground, lines of 1.4",
      if link { "├─" } else { "└─" }
    ));
    if link {
      tree.push(format!("{bar}  └─ link   13 at 600, arrow-up-right 14"));
    }
    if arrow {
      tree.push("└─ arrow   12 × 6, the panel's fill".to_owned());
    }

    let (to_label, to_content, to_link, to_placement) = (
      self.label.clone(),
      self.content.clone(),
      self.link.clone(),
      self.placement.clone(),
    );
    let (to_arrow, to_open, to_disabled) = (self.arrow.clone(), self.open.clone(), self.disabled.clone());
    let set_placement = self.placement.clone();
    let choices: Vec<(&str, usize)> = PLACEMENTS
      .iter()
      .enumerate()
      .map(|(index, (name, _))| (*name, index))
      .collect();
    self.playground.card(
      ctx,
      Spec {
        title: "Toggletip",
        make: Arc::new(move || plain().into()),
        shown: shown.into(),
        fill: false,
        forceable: false,
        facts: if open {
          "open".to_owned()
        } else {
          "on a press".to_owned()
        },
        event: if link { "link" } else { "" },
        code,
        tree,
        controls: vec![
          prop("label", "&str", field(&self.label)),
          prop("content", "&str", field(&self.content)),
          prop(
            "placement",
            "Placement",
            self
              .playground
              .select(0, &choices, &chosen, move |index| set_placement.set(index)),
          ),
          divider(),
          flag("link", &self.link),
          flag("arrow", &self.arrow),
          flag("open", &self.open),
          flag("disabled", &self.disabled),
        ],
        changed: [!link, placement != Placement::Top, arrow, !open, disabled]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset: Arc::new(move || {
          to_label.set("What is a logical pixel?".to_owned());
          to_content.set("Scales with the display, so 1px stays crisp.".to_owned());
          to_link.set(true);
          to_placement.set(0);
          to_arrow.set(false);
          to_open.set(true);
          to_disabled.set(false);
        }),
      },
    )
  }
}

impl Component for ToggletipPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 1),
      label: ctx.signal("What is a logical pixel?".to_owned()),
      content: ctx.signal("Scales with the display, so 1px stays crisp.".to_owned()),
      link: ctx.signal(true),
      placement: ctx.signal(0),
      arrow: ctx.signal(false),
      open: ctx.signal(true),
      disabled: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let workspace = Toggletip::new("What is a workspace?")
      .content("A workspace holds the projects a team shares, and who may open them.")
      .link("Learn more", |_| {});
    let field = room(labelled("Workspace", workspace.mount(ctx)));

    let billing = Toggletip::new("Why do we ask?")
      .content("The country decides the tax on your invoice.")
      .child(
        Button::new("Change country")
          .variant(ButtonVariant::Secondary)
          .size(ButtonSize::Xs),
      )
      .placement(Placement::Bottom);
    let with_button = room(labelled("Country", billing.mount(ctx)));

    let mut sides = Row::new()
      .width(FILL)
      .padding_vertical(96.0)
      .spacing(120.0)
      .wrap()
      .justify(Justify::Center)
      .align_items(Alignment::Center);
    for (name, placement) in [("Top", Placement::Top), ("Right", Placement::Right)] {
      let tip = Toggletip::new(name)
        .content(&format!("Placement::{name}, with its arrow."))
        .placement(placement)
        .arrow(true)
        .open(Some(true));
      sides = sides.child(labelled(name, tip.mount(ctx)));
    }

    let rows = [
      (
        "label",
        "&str",
        "",
        "What the button is called to tooling: the question the panel answers.",
      ),
      (
        "mount",
        "(ctx)",
        "",
        "Puts the button where it goes, and its panel over the window while it is open.",
      ),
      ("content", "&str", "", "What the panel says."),
      (
        "link",
        "(&str, impl IntoMouseEventHandler)",
        "",
        "A link under the text, with an arrow after it. Tab goes into it from the button.",
      ),
      (
        "child",
        "impl Into<Element>",
        "",
        "Something the panel holds under its text: a button.",
      ),
      ("icon", "impl Into<Name>", "icons::INFO", "The button's icon."),
      (
        "placement",
        "Placement",
        "Top",
        "The side of the button the panel is on; the other side where there is no room.",
      ),
      ("arrow", "bool", "false", "A triangle from the panel to the button."),
      (
        "open",
        "Option<bool>",
        "None",
        "Shown whatever the button says, or never.",
      ),
      ("disabled", "bool", "false", "The button takes nothing."),
      ("width", "impl Into<Length>", "230", "How wide the panel is."),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Surfaces & overlays", "Toggletip"],
        title: "Toggletip",
        description: "A tooltip opened by a press: a small button beside a label that opens a few words about it, \
                      for what must be reachable by a finger and told by a screen reader.",
        import: Some("use lurq::ui::Toggletip;"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Toggletip is a control on the right. With open off, the button opens and closes the panel; \
           a press outside it and Escape close it, and the focus stays on the button.",
          playground,
        ),
        Section::new(
          "Beside a label",
          "The panel is a status, which a screen reader reads as it opens. Tab goes from the button into its link.",
          self.frame.demo(
            0,
            field,
            "Toggletip::new(\"What is a workspace?\").content(..).link(..).mount(ctx)",
            &[
              "Toggletip::new(\"What is a workspace?\")",
              "    .content(\"A workspace holds the projects a team shares, and who may open them.\")",
              "    .link(\"Learn more\", |_| open_docs())",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "With a button",
          "What it holds is anything short: here a button, under the panel's button.",
          self.frame.demo(
            1,
            with_button,
            ".child(Button::new(\"Change country\"))",
            &[
              "Toggletip::new(\"Why do we ask?\")",
              "    .content(\"The country decides the tax on your invoice.\")",
              "    .child(Button::new(\"Change country\").variant(ButtonVariant::Secondary).size(ButtonSize::Xs))",
              "    .placement(Placement::Bottom)",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Sides and an arrow",
          "Above the button unless it says otherwise, 6 from it; the arrow is the tooltip's, and points at the \
           button's middle.",
          self.frame.demo(
            2,
            sides,
            ".placement(Placement::Right).arrow(true)",
            &[
              "Toggletip::new(\"Right\")",
              "    .content(\"Placement::Right, with its arrow.\")",
              "    .placement(Placement::Right)",
              "    .arrow(true)",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a toggletip is given.",
          props_table(&rows, &["label", "mount"], self.frame.room()),
        ),
      ],
    )
  }
}
