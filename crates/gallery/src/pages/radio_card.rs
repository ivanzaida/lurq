//! The page of the radio card: its playground, a group bound to a signal as the board draws it, plans side by side
//! with their prices, and the states of one card.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{Interaction, RadioCard, RadioCardGroup, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Spec, divider, flag},
};

/// The choices of the playground, as the board draws the first.
const DENSITIES: [(&str, &str); 3] = [
  ("Comfortable", "Roomy rows, 36px controls."),
  ("Compact", "Dense rows, 28px controls."),
  ("Spacious", "Large rows, for touch."),
];

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .spacing(9.0)
    .overflow_visible()
    .child(Row::new().overflow_visible().child(shown))
    .child(quiet(caption, 12.0))
}

/// What is on a stage, side by side; what does not fit goes on to the next line.
fn items() -> Row {
  Row::new()
    .width(FILL)
    .spacing(28.0)
    .wrap()
    .align_items(Alignment::Start)
    .overflow_visible()
}

pub struct RadioCardPage {
  frame: Frame,
  playground: Playground,
  chosen: Signal<usize>,
  horizontal: Signal<bool>,
  descriptions: Signal<bool>,
  invalid: Signal<bool>,
  disabled: Signal<bool>,
  last_disabled: Signal<bool>,
  density: Signal<usize>,
  plan: Signal<&'static str>,
}

impl RadioCardPage {
  /// The playground: the group as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let chosen = self.chosen.get().min(DENSITIES.len() - 1);
    let (horizontal, descriptions) = (self.horizontal.get(), self.descriptions.get());
    let (invalid, disabled, last_disabled) = (self.invalid.get(), self.disabled.get(), self.last_disabled.get());
    let cards = move || {
      DENSITIES.iter().enumerate().map(move |(index, (title, description))| {
        let mut card = RadioCard::new(title).disabled(last_disabled && index + 1 == DENSITIES.len());
        if descriptions {
          card = card.description(description);
        }
        (index, card)
      })
    };
    let plain = move |choose: Box<dyn Fn(usize) + Send + Sync>| {
      RadioCardGroup::controlled(Some(chosen), cards(), choose)
        .horizontal(horizontal)
        .invalid(invalid)
        .disabled(disabled)
    };
    let (record, set) = (self.playground.recorder(), self.chosen.clone());
    let group = plain(Box::new(move |index| {
      set.set(index);
      record(format!("chose: {}", DENSITIES[index].0));
    }));

    let mut code = vec!["RadioCardGroup::new(density, [".to_owned()];
    for (index, (title, description)) in DENSITIES.iter().enumerate() {
      let mut card = format!("RadioCard::new({title:?})");
      if descriptions {
        card.push_str(&format!(".description({description:?})"));
      }
      if last_disabled && index + 1 == DENSITIES.len() {
        card.push_str(".disabled(true)");
      }
      code.push(format!("    (Density::{title}, {card}),"));
    }
    code.push("])".to_owned());
    if horizontal {
      code.push("    .horizontal(true)".to_owned());
    }
    if invalid {
      code.push("    .invalid(true)".to_owned());
    }
    if disabled {
      code.push("    .disabled(true)".to_owned());
    }

    let mut tree = vec![format!(
      "root          {}   {}",
      self.playground.measured(),
      if horizontal {
        "side by side, 12 apart"
      } else {
        "12 apart"
      }
    )];
    for (index, (title, _)) in DENSITIES.iter().enumerate() {
      let last = index + 1 == DENSITIES.len();
      let (branch, under) = if last { ("└─", " ") } else { ("├─", "│") };
      tree.push(format!(
        "{branch} card       {title:?}{}",
        if index == chosen { "   chosen, edge 2" } else { "" }
      ));
      tree.push(format!(
        "{under}  ├─ text    title{}",
        if descriptions { " · description" } else { "" }
      ));
      tree.push(format!(
        "{under}  └─ mark    16 × 16 px, the radio's{}",
        if index == chosen { ", the dot in it" } else { "" }
      ));
    }

    let (to_chosen, flags, to_descriptions) = (
      self.chosen.clone(),
      [
        self.horizontal.clone(),
        self.invalid.clone(),
        self.disabled.clone(),
        self.last_disabled.clone(),
      ],
      self.descriptions.clone(),
    );
    self.playground.card(
      ctx,
      Spec {
        title: "Radio Card",
        make: Arc::new(move || plain(Box::new(|_| {})).into()),
        shown: group.into(),
        fill: false,
        forceable: false,
        facts: format!("chosen: {}", DENSITIES[chosen].0),
        event: "chose",
        code,
        tree,
        controls: vec![
          flag("horizontal", &self.horizontal),
          flag("description", &self.descriptions),
          flag("invalid", &self.invalid),
          flag("disabled", &self.disabled),
          divider(),
          flag("Spacious disabled", &self.last_disabled),
        ],
        changed: [chosen != 0, horizontal, !descriptions, invalid, disabled, last_disabled]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset: Arc::new(move || {
          to_chosen.set(0);
          to_descriptions.set(true);
          for flag in &flags {
            flag.set(false);
          }
        }),
      },
    )
  }
}

impl Component for RadioCardPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      chosen: ctx.signal(0),
      horizontal: ctx.signal(false),
      descriptions: ctx.signal(true),
      invalid: ctx.signal(false),
      disabled: ctx.signal(false),
      last_disabled: ctx.signal(false),
      density: ctx.signal(0),
      plan: ctx.signal("team"),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let bound = items().child(item(
      RadioCardGroup::new(
        self.density.clone(),
        [
          (
            0usize,
            RadioCard::new("Comfortable").description("Roomy rows, 36px controls."),
          ),
          (1, RadioCard::new("Compact").description("Dense rows, 28px controls.")),
        ],
      )
      .horizontal(true),
      &format!("the signal has: {}", self.density.get()),
    ));
    let plans = items().child(item(
      RadioCardGroup::new(
        self.plan.clone(),
        [
          (
            "free",
            RadioCard::new("Free")
              .description("One person, three projects.")
              .icon(icons::USERS)
              .price("$0"),
          ),
          (
            "team",
            RadioCard::new("Team")
              .description("Up to ten people.")
              .icon(icons::USERS)
              .price("$12"),
          ),
          (
            "enterprise",
            RadioCard::new("Enterprise")
              .description("Talk to us first.")
              .icon(icons::SHIELD)
              .price("$40")
              .disabled(true),
          ),
        ],
      )
      .horizontal(true),
      &format!("the signal has: {:?}", self.plan.get()),
    ));
    let card = |title: &str| RadioCard::new(title).description("Roomy rows, 36px controls.");
    let states = items()
      .child(item(card("Not chosen"), "not chosen"))
      .child(item(card("Chosen").chosen(true), "chosen"))
      .child(item(card("Hovered").force(Some(Interaction::Hovered)), "hovered"))
      .child(item(
        card("Focused").force(Some(Interaction::FocusVisible)),
        "focus-visible",
      ))
      .child(item(card("Invalid").invalid(true), "invalid"))
      .child(item(card("Disabled").disabled(true), "disabled"))
      .child(item(
        card("Disabled, chosen").chosen(true).disabled(true),
        "disabled, chosen",
      ));
    let card_rows = [
      (
        "title",
        "&str",
        "",
        "What the choice is: the card's name to what reads the tree.",
      ),
      (
        "description",
        "&str",
        "",
        "More of it, under the title in the muted color: what the card is described by.",
      ),
      ("icon", "impl Into<Name>", "", "An icon of 20 before the text."),
      ("price", "&str", "", "What the choice costs, at the end of the title."),
      (
        "chosen",
        "bool",
        "false",
        "Whether it is the choice made: the dot in the mark and the edge of 2 in the primary color.",
      ),
      (
        "on_choose",
        "impl Fn()",
        "",
        "Called on a press while the card is not chosen. A press of the one chosen does nothing.",
      ),
      (
        "invalid",
        "bool",
        "false",
        "The edge and the mark in the destructive color.",
      ),
      (
        "disabled",
        "bool",
        "false",
        "Half the opacity; it takes no input and the arrow keys pass it by.",
      ),
      (
        "full_width",
        "bool",
        "false",
        "As wide as what holds it, in place of 220.",
      ),
    ];
    let group_rows = [
      (
        "value",
        "Signal<T>",
        "",
        "RadioCardGroup::new: the card whose value the signal has is chosen, and a press of another sets it.",
      ),
      (
        "cards",
        "(T, RadioCard)",
        "",
        "A card for each value, with its own title, description, icon, price and disabled.",
      ),
      (
        "controlled",
        "Option<T>, impl Fn(T)",
        "",
        "In place of a signal: the app holds the choice, none while it is None, and is told each value pressed.",
      ),
      (
        "horizontal",
        "bool",
        "false",
        "Side by side, 12 apart and as high as the highest, going on to the next line where there is no room.",
      ),
      (
        "invalid",
        "bool",
        "false",
        "Every card's edge in the destructive color.",
      ),
      (
        "disabled",
        "bool",
        "false",
        "Every card dimmed; the group takes no input.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Inputs", "Radio Card"],
        title: "Radio Card",
        description: "A radio whose whole card is pressed, with a title, a description and an icon or a price: the \
                      plan of an account, the density of a list.",
        import: Some("use lurq::ui::{RadioCard, RadioCardGroup};"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of RadioCardGroup is a control on the right. The group on the stage is live: a press of a card \
           chooses it, and a press of the one chosen does nothing.",
          playground,
        ),
        Section::new(
          "Radio card group",
          "The page holds the choice in a signal. Tab stops in the group once, at the chosen card; the arrow keys \
           move the focus to the next card and choose it, as in a group of radios.",
          self.frame.demo(
            0,
            bound,
            "RadioCardGroup::new(density, [(0, RadioCard::new(\"Comfortable\")), ..])",
            &[
              "// density: Signal<usize>",
              "RadioCardGroup::new(density, [",
              "    (0, RadioCard::new(\"Comfortable\").description(\"Roomy rows, 36px controls.\")),",
              "    (1, RadioCard::new(\"Compact\").description(\"Dense rows, 28px controls.\")),",
              "])",
              ".horizontal(true)",
            ],
          ),
        ),
        Section::new(
          "Plans with a price",
          "An icon before the text and a price at the end of the title. A plan that cannot be chosen is drawn, \
           dimmed, and the arrow keys pass it by.",
          self.frame.demo(
            1,
            plans,
            "RadioCard::new(\"Team\").icon(icons::USERS).price(\"$12\")",
            &[
              "// plan: Signal<&'static str>",
              "RadioCardGroup::new(plan, [",
              "    (\"free\", RadioCard::new(\"Free\").description(\"One person, three projects.\")",
              "        .icon(icons::USERS).price(\"$0\")),",
              "    (\"team\", RadioCard::new(\"Team\").description(\"Up to ten people.\")",
              "        .icon(icons::USERS).price(\"$12\")),",
              "    (\"enterprise\", RadioCard::new(\"Enterprise\").description(\"Talk to us first.\")",
              "        .icon(icons::SHIELD).price(\"$40\").disabled(true)),",
              "])",
              ".horizontal(true)",
            ],
          ),
        ),
        Section::new(
          "States",
          "One card, alone. Its edge is an input's until it is chosen, then 2 of the primary color with the dot in \
           the radio's mark. Under the pointer the edge takes the ring's color; the ring of the focus is the card's.",
          self.frame.demo(
            2,
            states,
            ".chosen(true)",
            &[
              "RadioCard::new(\"Chosen\").description(\"Roomy rows, 36px controls.\").chosen(true)",
              "",
              "// Drawn as under the pointer or focused, with nothing pointing at it: for a gallery, a test.",
              "RadioCard::new(\"Focused\").force(Some(Interaction::FocusVisible))",
              "",
              "RadioCard::new(\"Invalid\").invalid(true)",
              "RadioCard::new(\"Disabled\").disabled(true)",
            ],
          ),
        ),
        Section::new(
          "RadioCardGroup props",
          "What a group is given.",
          props_table(&group_rows, &["value", "cards"], self.frame.room()),
        ),
        Section::new(
          "RadioCard props",
          "What one card is given, where it is not in a group.",
          props_table(&card_rows, &["title"], self.frame.room()),
        ),
      ],
    )
  }
}
