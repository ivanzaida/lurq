//! The page of the hover card: its playground, a person's card on a link in
//! a line of text, a card with a button in it, and one open with its arrow.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Placement, Row},
  core::Signal,
  layout::{Alignment, layout_kind::Justify},
  node::Element,
  ui::{Avatar, Button, ButtonSize, ButtonVariant, HoverCard, Link, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Spec, divider, field, flag, prop},
};

/// The sides the playground offers.
const PLACEMENTS: [(&str, Placement); 6] = [
  ("Bottom", Placement::Bottom),
  ("BottomStart", Placement::BottomStart),
  ("Top", Placement::Top),
  ("TopStart", Placement::TopStart),
  ("Right", Placement::Right),
  ("Left", Placement::Left),
];

/// The board's card: who `@lurq` is.
fn person(title: &str, description: &str, media: bool) -> HoverCard {
  let mut card = HoverCard::new()
    .title(title)
    .description(description)
    .fact(icons::CALENDAR_DAYS, "Joined March 2024");
  if media {
    card = card.media(Avatar::new("LQ"));
  }
  card
}

/// A line of text with `link` in it: what the pointer rests on.
fn sentence(before: &str, link: impl Into<Element>, after: &str) -> Row {
  Row::new()
    .align_items(Alignment::Center)
    .spacing(4.0)
    .child(quiet(before, 14.0))
    .child(link)
    .child(quiet(after, 14.0))
}

/// Room around what is on a stage, for the card under it.
fn room(content: impl Into<Element>, below: f32) -> Row {
  Row::new()
    .width(FILL)
    .padding_top(40.0)
    .padding_bottom(below)
    .justify(Justify::Center)
    .child(content)
}

pub struct HoverCardPage {
  frame: Frame,
  playground: Playground,
  title: Signal<String>,
  description: Signal<String>,
  media: Signal<bool>,
  /// Whether it holds a button: a dialog to tooling, which Tab goes into.
  button: Signal<bool>,
  /// The placement, as its index in [`PLACEMENTS`].
  placement: Signal<usize>,
  arrow: Signal<bool>,
  open: Signal<bool>,
  disabled: Signal<bool>,
}

impl HoverCardPage {
  /// The playground: a link with the card the panel's props make.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (title, description) = (self.title.get(), self.description.get());
    let (media, button, chosen) = (self.media.get(), self.button.get(), self.placement.get());
    let (placement_name, placement) = PLACEMENTS[chosen.min(PLACEMENTS.len() - 1)];
    let (arrow, open, disabled) = (self.arrow.get(), self.open.get(), self.disabled.get());
    let record = self.playground.recorder();
    let plain = {
      let (title, description) = (title.clone(), description.clone());
      move || {
        let mut card = person(&title, &description, media)
          .placement(placement)
          .arrow(arrow)
          .disabled(disabled);
        if button {
          let record = record.clone();
          card = card.child(
            Button::new("Follow")
              .size(ButtonSize::Sm)
              .on_click(move |_| record("pressed: Follow".to_owned())),
          );
        }
        card
      }
    };
    let shown = plain().open(open.then_some(true)).mount(ctx, Link::new(&title));

    let mut code = vec!["HoverCard::new()".to_owned()];
    if media {
      code.push("    .media(Avatar::new(\"LQ\"))".to_owned());
    }
    code.push(format!("    .title({title:?})"));
    code.push(format!("    .description({description:?})"));
    code.push("    .fact(icons::CALENDAR_DAYS, \"Joined March 2024\")".to_owned());
    if button {
      code.push("    .child(Button::new(\"Follow\").size(ButtonSize::Sm))".to_owned());
    }
    if placement != Placement::Bottom {
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
    code.push(format!("    .mount(ctx, Link::new({title:?}))"));

    let mut tree = vec![
      "link      what it is about, its anchor".to_owned(),
      format!("└─ root   8 from the link, {}", placement_name.to_lowercase()),
      format!(
        "   {} panel  300 wide, popover, edge 1, radius lg, shadow md, 16 padding",
        if arrow { "├─" } else { "└─" }
      ),
    ];
    let bar = if arrow { "│" } else { " " };
    if media {
      tree.push(format!("   {bar}  ├─ media    Avatar, 12 before the text"));
    }
    tree.push(format!("   {bar}  └─ text     6 apart"));
    tree.push(format!("   {bar}     ├─ title        14, medium"));
    tree.push(format!("   {bar}     ├─ description  13, muted, lines of 1.5"));
    tree.push(format!(
      "   {bar}     {} facts        icon 14 and 12, muted",
      if button { "├─" } else { "└─" }
    ));
    if button {
      tree.push(format!("   {bar}     └─ Button \"Follow\""));
    }
    if arrow {
      tree.push("   └─ arrow  18 × 9, the panel's fill and edge".to_owned());
    }

    let (to_title, to_description, to_media, to_button) = (
      self.title.clone(),
      self.description.clone(),
      self.media.clone(),
      self.button.clone(),
    );
    let (to_placement, to_arrow, to_open, to_disabled) = (
      self.placement.clone(),
      self.arrow.clone(),
      self.open.clone(),
      self.disabled.clone(),
    );
    let set_placement = self.placement.clone();
    let choices: Vec<(&str, usize)> = PLACEMENTS
      .iter()
      .enumerate()
      .map(|(index, (name, _))| (*name, index))
      .collect();
    self.playground.card(
      ctx,
      Spec {
        title: "Hover Card",
        make: Arc::new(move || plain().into()),
        shown,
        fill: false,
        forceable: false,
        facts: if open {
          "open".to_owned()
        } else {
          "on hover and focus".to_owned()
        },
        event: if button { "on_click" } else { "" },
        code,
        tree,
        controls: vec![
          prop("title", "&str", field(&self.title)),
          prop("description", "&str", field(&self.description)),
          prop(
            "placement",
            "Placement",
            self
              .playground
              .select(0, &choices, &chosen, move |index| set_placement.set(index)),
          ),
          divider(),
          flag("media", &self.media),
          flag("a button in it", &self.button),
          flag("arrow", &self.arrow),
          flag("open", &self.open),
          flag("disabled", &self.disabled),
        ],
        changed: [!media, button, placement != Placement::Bottom, arrow, !open, disabled]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset: Arc::new(move || {
          to_title.set("@lurq".to_owned());
          to_description.set("A Rust UI toolkit with GPU-backed rendering.".to_owned());
          to_media.set(true);
          to_button.set(false);
          to_placement.set(0);
          to_arrow.set(false);
          to_open.set(true);
          to_disabled.set(false);
        }),
      },
    )
  }
}

impl Component for HoverCardPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 1),
      title: ctx.signal("@lurq".to_owned()),
      description: ctx.signal("A Rust UI toolkit with GPU-backed rendering.".to_owned()),
      media: ctx.signal(true),
      button: ctx.signal(false),
      placement: ctx.signal(0),
      arrow: ctx.signal(false),
      open: ctx.signal(true),
      disabled: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let card = person("@lurq", "A Rust UI toolkit with GPU-backed rendering.", true);
    let line = room(
      sentence(
        "Built by",
        card.mount(ctx, Link::new("@lurq")),
        "and the people who send patches.",
      ),
      150.0,
    );

    let with_button = HoverCard::new()
      .media(Avatar::new("Ivan Zaida"))
      .title("Ivan Zaida")
      .description("Maintains the renderer and the design system.")
      .fact(icons::CALENDAR_DAYS, "Joined January 2023")
      .child(
        Row::new()
          .spacing(8.0)
          .child(Button::new("Follow").size(ButtonSize::Sm))
          .child(
            Button::new("Message")
              .variant(ButtonVariant::Outline)
              .size(ButtonSize::Sm),
          ),
      );
    let controls = room(
      sentence(
        "Reviewed by",
        with_button.mount(ctx, Link::new("Ivan Zaida")),
        "yesterday.",
      ),
      190.0,
    );

    let file = HoverCard::new()
      .title("design-system.pen")
      .description("The board every component of lurq::ui is drawn on.")
      .fact(icons::CALENDAR_DAYS, "Changed today")
      .plain_fact("2.4 MB")
      .placement(Placement::Right)
      .arrow(true)
      .open(Some(true));
    let open = Row::new().width(FILL).padding_vertical(64.0).padding_left(24.0).child(
      file.mount(
        ctx,
        Button::new("design-system.pen")
          .variant(ButtonVariant::Outline)
          .icon("file"),
      ),
    );

    let rows = [
      (
        "mount",
        "(ctx, impl Into<Element>)",
        "",
        "Puts the card on the element it is about and returns that element. The pointer and the focus are its \
         root's, or those of anything in it.",
      ),
      (
        "title",
        "&str",
        "",
        "What it is about: a name, a page's title. What tooling calls it.",
      ),
      ("description", "&str", "", "A line or two about it, in the muted color."),
      (
        "media",
        "impl Into<Element>",
        "",
        "What stands at its start beside the text: an avatar.",
      ),
      (
        "fact",
        "(impl Into<Name>, &str)",
        "",
        "A fact in the row under the description: an icon and a short text. plain_fact has no icon.",
      ),
      (
        "child",
        "impl Into<Element>",
        "",
        "Something it holds under its text: a link, a row of buttons. With a control in it the card is a dialog \
         to tooling, and Tab goes into it from the element.",
      ),
      (
        "placement",
        "Placement",
        "Bottom",
        "The side of the element it is on, and the edge it lines up with; the other side where there is no room.",
      ),
      (
        "arrow",
        "bool",
        "false",
        "A triangle with the panel's edge from the panel to the element, turned with the side it is on.",
      ),
      (
        "open_delay",
        "impl Into<TransitionDuration>",
        "MotionDuration::Slower",
        "How long the pointer rests before it opens: 400 in the design's theme. None for the keyboard's focus.",
      ),
      (
        "close_delay",
        "impl Into<TransitionDuration>",
        "MotionDuration::Slow",
        "How long it stays after the pointer left the element and the card: time to cross between them.",
      ),
      (
        "open",
        "Option<bool>",
        "None",
        "Shown whatever the pointer and the focus do, or never.",
      ),
      ("disabled", "bool", "false", "It never opens."),
      ("width", "impl Into<Length>", "300", "How wide the panel is."),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Surfaces & overlays", "Hover Card"],
        title: "Hover Card",
        description: "More about what the pointer rests on, a person, a link, a file, in a panel beside it. Unlike \
                      a tooltip it holds what a person reads and presses.",
        import: Some("use lurq::ui::HoverCard;"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of HoverCard is a control on the right. With open on, the card stays; turn it off and rest the \
           pointer on the link, or reach it with Tab.",
          playground,
        ),
        Section::new(
          "On a link",
          "Rest the pointer on the name: the card opens after the theme's slower duration, and the pointer can be \
           moved onto it across the room between them. It closes a moment after the pointer has left both.",
          self.frame.demo(
            0,
            line,
            "HoverCard::new().title(..).description(..).mount(ctx, link)",
            &[
              "HoverCard::new()",
              "    .media(Avatar::new(\"LQ\"))",
              "    .title(\"@lurq\")",
              "    .description(\"A Rust UI toolkit with GPU-backed rendering.\")",
              "    .fact(icons::CALENDAR_DAYS, \"Joined March 2024\")",
              "    .mount(ctx, Link::new(\"@lurq\"))",
            ],
          ),
        ),
        Section::new(
          "With buttons",
          "A card with controls is a dialog to tooling. Tab from the link opens it at once and goes into its buttons, \
           then on to what follows the link; Escape closes it and brings the focus back.",
          self.frame.demo(
            1,
            controls,
            ".child(Row::new().child(Button::new(\"Follow\")))",
            &[
              "HoverCard::new()",
              "    .media(Avatar::new(\"Ivan Zaida\"))",
              "    .title(\"Ivan Zaida\")",
              "    .description(\"Maintains the renderer and the design system.\")",
              "    .fact(icons::CALENDAR_DAYS, \"Joined January 2023\")",
              "    .child(",
              "        Row::new()",
              "            .spacing(8.0)",
              "            .child(Button::new(\"Follow\").size(ButtonSize::Sm))",
              "            .child(Button::new(\"Message\").variant(ButtonVariant::Outline).size(ButtonSize::Sm)),",
              "    )",
              "    .mount(ctx, Link::new(\"Ivan Zaida\"))",
            ],
          ),
        ),
        Section::new(
          "Beside it, with an arrow",
          "On any side of the element, the arrow pointing at its middle with the panel's edge around it. Facts \
           stand in a row.",
          self.frame.demo(
            2,
            Column::new().width(FILL).child(open),
            ".placement(Placement::Right).arrow(true)",
            &[
              "HoverCard::new()",
              "    .title(\"design-system.pen\")",
              "    .description(\"The board every component of lurq::ui is drawn on.\")",
              "    .fact(icons::CALENDAR_DAYS, \"Changed today\")",
              "    .plain_fact(\"2.4 MB\")",
              "    .placement(Placement::Right)",
              "    .arrow(true)",
              "    .mount(ctx, file_button)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a hover card is given.",
          props_table(&rows, &["mount"], self.frame.room()),
        ),
      ],
    )
  }
}
