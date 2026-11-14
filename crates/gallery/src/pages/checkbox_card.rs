//! The page of the checkbox card: its playground, the board's cards checked and not, their states, cards over one
//! list bound to a signal, and cards with an icon or a price.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{CheckboxCard, CheckboxCardGroup, Interaction, Orientation, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

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

pub struct CheckboxCardPage {
  frame: Frame,
  playground: Playground,
  title: Signal<String>,
  description: Signal<String>,
  checked: Signal<bool>,
  icon: Signal<bool>,
  price: Signal<bool>,
  invalid: Signal<bool>,
  disabled: Signal<bool>,
  full_width: Signal<bool>,
  add_ons: Signal<Vec<&'static str>>,
  /// What the app holds of the cards one under the other.
  reports: Signal<Vec<&'static str>>,
}

impl CheckboxCardPage {
  /// The playground: one card as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (title, description) = (self.title.get(), self.description.get());
    let (checked, icon, price) = (self.checked.get(), self.icon.get(), self.price.get());
    let (invalid, disabled, full_width) = (self.invalid.get(), self.disabled.get(), self.full_width.get());
    let plain = {
      let (title, description) = (title.clone(), description.clone());
      move || {
        let mut card = CheckboxCard::new(&title)
          .checked(checked)
          .invalid(invalid)
          .disabled(disabled)
          .full_width(full_width);
        if !description.is_empty() {
          card = card.description(&description);
        }
        if icon {
          card = card.icon(icons::BELL);
        }
        if price {
          card = card.price("$4");
        }
        card
      }
    };
    let (record, set) = (self.playground.recorder(), self.checked.clone());
    let card = plain()
      .force(self.playground.forced())
      .bind(set)
      .on_change(move |checked| record(format!("on_change: {checked}")));
    let make = {
      let plain = plain.clone();
      move || plain().into()
    };

    let mut code = vec![format!("CheckboxCard::new({title:?})")];
    if !description.is_empty() {
      code.push(format!("    .description({description:?})"));
    }
    if icon {
      code.push("    .icon(icons::BELL)".to_owned());
    }
    if price {
      code.push("    .price(\"$4\")".to_owned());
    }
    if invalid {
      code.push("    .invalid(true)".to_owned());
    }
    if disabled {
      code.push("    .disabled(true)".to_owned());
    }
    if full_width {
      code.push("    .full_width(true)".to_owned());
    }
    code.push("    .bind(analytics)".to_owned());

    let mut tree = vec![format!(
      "root          {}   padded 16 · radius-lg · edge {}",
      self.playground.measured(),
      if checked { "2, primary" } else { "1, input" }
    )];
    if icon {
      tree.push("├─ icon       20 × 20 px".to_owned());
    }
    tree.push("├─ text       4 apart".to_owned());
    tree.push(format!("│  ├─ heading  title{}", if price { " · price" } else { "" }));
    if !description.is_empty() {
      tree.push("│  └─ description".to_owned());
    }
    tree.push(format!(
      "└─ box        16 × 16 px, the checkbox's{}",
      if checked { ", checked" } else { "" }
    ));

    let texts = (self.title.clone(), self.description.clone());
    let flags = [
      self.checked.clone(),
      self.icon.clone(),
      self.price.clone(),
      self.invalid.clone(),
      self.disabled.clone(),
      self.full_width.clone(),
    ];
    self.playground.card(
      ctx,
      Spec {
        title: "Checkbox Card",
        make: Arc::new(make),
        shown: Shown::of(card),
        fill: full_width,
        forceable: true,
        facts: format!("checked: {checked}"),
        event: "on_change",
        code,
        tree,
        controls: vec![
          prop("title", "&str", field(&self.title)),
          prop("description", "&str", field(&self.description)),
          divider(),
          flag("checked", &self.checked),
          flag("icon", &self.icon),
          flag("price", &self.price),
          flag("invalid", &self.invalid),
          flag("disabled", &self.disabled),
          flag("full_width", &self.full_width),
        ],
        changed: [
          title != "Analytics",
          description != "Share anonymous usage data.",
          !checked,
          icon,
          price,
          invalid,
          disabled,
          full_width,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          texts.0.set("Analytics".to_owned());
          texts.1.set("Share anonymous usage data.".to_owned());
          for (index, flag) in flags.iter().enumerate() {
            flag.set(index == 0);
          }
        }),
      },
    )
  }
}

impl Component for CheckboxCardPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      title: ctx.signal("Analytics".to_owned()),
      description: ctx.signal("Share anonymous usage data.".to_owned()),
      checked: ctx.signal(true),
      icon: ctx.signal(false),
      price: ctx.signal(false),
      invalid: ctx.signal(false),
      disabled: ctx.signal(false),
      full_width: ctx.signal(false),
      add_ons: ctx.signal(vec!["backups"]),
      reports: ctx.signal(vec!["crash"]),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let card = |title: &str| CheckboxCard::new(title).description("Send reports when lurq fails.");
    let states = items()
      .child(item(card("Not checked"), "not checked"))
      .child(item(card("Checked").checked(true), "checked"))
      .child(item(card("Hovered").force(Some(Interaction::Hovered)), "hovered"))
      .child(item(
        card("Focused").force(Some(Interaction::FocusVisible)),
        "focus-visible",
      ))
      .child(item(card("Invalid").invalid(true), "invalid"))
      .child(item(card("Disabled").disabled(true), "disabled"))
      .child(item(
        card("Disabled, checked").checked(true).disabled(true),
        "disabled, checked",
      ));
    let add_ons = self.add_ons.get();
    let group = items().child(item(
      CheckboxCardGroup::new(
        self.add_ons.clone(),
        [
          (
            "backups",
            CheckboxCard::new("Backups")
              .description("Every night, kept for 30 days.")
              .icon(icons::SHIELD)
              .price("$4"),
          ),
          (
            "support",
            CheckboxCard::new("Priority support")
              .description("An answer within a working day.")
              .icon(icons::MAIL)
              .price("$20"),
          ),
          (
            "seats",
            CheckboxCard::new("Extra seats")
              .description("Ten more people in the workspace.")
              .icon(icons::USERS)
              .price("$30")
              .disabled(true),
          ),
        ],
      )
      .orientation(Orientation::Horizontal),
      &format!("the signal has: {add_ons:?}"),
    ));
    let (reports, set_reports) = (self.reports.get(), self.reports.clone());
    let column = items().child(item(
      Column::new().width(320.0).child(CheckboxCardGroup::controlled(
        reports.clone(),
        [
          (
            "crash",
            CheckboxCard::new("Crash reports")
              .description("Send reports when lurq fails.")
              .full_width(true),
          ),
          (
            "usage",
            CheckboxCard::new("Usage data")
              .description("Which components are used, never what is in them.")
              .full_width(true),
          ),
        ],
        move |next| set_reports.set(next.to_vec()),
      )),
      &format!("the app holds: {reports:?}"),
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
        "checked",
        "bool",
        "false",
        "Whether it is checked: the box checked and the edge of 2 in the primary color. The app holds it.",
      ),
      (
        "bind",
        "Signal<bool>",
        "",
        "In place of checked and a handler: checked while the signal is true, which a press turns.",
      ),
      (
        "on_change",
        "impl Fn(bool)",
        "",
        "Called on a press of the card or Space on it, with what it is to be. Bound, after the signal is set.",
      ),
      (
        "invalid",
        "bool",
        "false",
        "The edge and the box in the destructive color.",
      ),
      (
        "disabled",
        "bool",
        "false",
        "Half the opacity; it takes no input and Tab passes it by.",
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
        "checked",
        "Signal<Vec<T>>",
        "",
        "CheckboxCardGroup::new: the cards whose values the signal has are checked; a press adds or takes out.",
      ),
      (
        "cards",
        "(T, CheckboxCard)",
        "",
        "A card for each value, with its own title, description, icon, price and disabled.",
      ),
      (
        "controlled",
        "impl IntoIterator<T>, impl Fn(&[T])",
        "",
        "In place of a signal: the app holds what is checked and is told the whole new set a press asks for.",
      ),
      (
        "on_change",
        "impl Fn(&[T])",
        "",
        "Called on a press with the whole new set checked, after the signal is set.",
      ),
      (
        "orientation",
        "Orientation",
        "Vertical",
        "Horizontal: side by side, 12 apart, going on to the next line where there is no room.",
      ),
      (
        "label",
        "&str",
        "",
        "What the group is called to tooling. It draws nothing: a field or a fieldset gives it its label.",
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
        breadcrumb: &["Components", "Inputs", "Checkbox Card"],
        title: "Checkbox Card",
        description: "A checkbox whose whole card is pressed, with a title, a description and an icon or a price: \
                      the add-ons of a plan, the reports a team is sent.",
        import: Some("use lurq::ui::{CheckboxCard, CheckboxCardGroup, Orientation};"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of CheckboxCard is a control on the right. The card on the stage is live: a press anywhere on \
           it, or Space while it has the focus, turns it.",
          playground,
        ),
        Section::new(
          "States",
          "The edge is an input's until the card is checked, then 2 of the primary color, and the box is the \
           checkbox's own. Under the pointer the edge takes the ring's color; the ring of the focus is around the \
           card, which is the one stop of Tab.",
          self.frame.demo(
            0,
            states,
            ".checked(true)",
            &[
              "CheckboxCard::new(\"Checked\").description(\"Send reports when lurq fails.\").checked(true)",
              "",
              "// Drawn as under the pointer or focused, with nothing pointing at it: for a gallery, a test.",
              "CheckboxCard::new(\"Focused\").force(Some(Interaction::FocusVisible))",
              "",
              "CheckboxCard::new(\"Invalid\").invalid(true)",
              "CheckboxCard::new(\"Disabled\").disabled(true)",
            ],
          ),
        ),
        Section::new(
          "Cards over one list",
          "The page holds what is checked in a signal. Each card is its own stop of Tab; a card disabled on its own \
           is drawn, dimmed, and takes nothing.",
          self.frame.demo(
            1,
            group,
            "CheckboxCardGroup::new(add_ons, [(\"backups\", CheckboxCard::new(\"Backups\")), ..])",
            &[
              "// add_ons: Signal<Vec<&'static str>>",
              "CheckboxCardGroup::new(add_ons, [",
              "    (\"backups\", CheckboxCard::new(\"Backups\")",
              "        .description(\"Every night, kept for 30 days.\")",
              "        .icon(icons::SHIELD)",
              "        .price(\"$4\")),",
              "    (\"support\", CheckboxCard::new(\"Priority support\").icon(icons::MAIL).price(\"$20\")),",
              "    (\"seats\", CheckboxCard::new(\"Extra seats\").icon(icons::USERS).price(\"$30\").disabled(true)),",
              "])",
              ".orientation(Orientation::Horizontal)",
            ],
          ),
        ),
        Section::new(
          "One under another",
          "Cards that fill a column: each is as wide as what holds it. The app holds what is checked, and is told \
           the whole new set a press asks for.",
          self.frame.demo(
            2,
            column,
            ".full_width(true)",
            &[
              "// reports: Signal<Vec<&'static str>>",
              "let set = reports.clone();",
              "Column::new().width(320.0).child(CheckboxCardGroup::controlled(reports.get(), [",
              "    (\"crash\", CheckboxCard::new(\"Crash reports\").full_width(true)),",
              "    (\"usage\", CheckboxCard::new(\"Usage data\").full_width(true)),",
              "], move |next| set.set(next.to_vec())))",
            ],
          ),
        ),
        Section::new(
          "CheckboxCard props",
          "What one card is given.",
          props_table(&card_rows, &["title"], self.frame.room()),
        ),
        Section::new(
          "CheckboxCardGroup props",
          "What a group is given.",
          props_table(&group_rows, &["checked", "cards"], self.frame.room()),
        ),
      ],
    )
  }
}
