//! The page of the board's Result, `ui::Outcome` in the code: its playground, the five variants, what the app puts
//! under the text, and one with no box on a page.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  layout::{Alignment, layout_kind::Justify},
  node::Element,
  ui::{Badge, BadgeVariant, Button, ButtonVariant, InlineCode, Outcome, OutcomeVariant, Separator, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet, segmented_fill, text},
  playground::{Playground, Spec, divider, field, flag, prop},
};

/// The variants, as the code names them, with what the board says in each.
const VARIANTS: [(OutcomeVariant, &str, &str, &str, &str, &str, &str); 5] = [
  (
    OutcomeVariant::Success,
    "success",
    "OutcomeVariant::Success",
    "Build succeeded",
    "412 tests passed in 38 seconds.",
    "View log",
    "Deploy",
  ),
  (
    OutcomeVariant::Error,
    "error",
    "OutcomeVariant::Error",
    "Build failed",
    "3 tests failed in render/blur.rs.",
    "View log",
    "Retry",
  ),
  (
    OutcomeVariant::Warning,
    "warning",
    "OutcomeVariant::Warning",
    "Built with warnings",
    "12 warnings, no errors.",
    "View log",
    "Continue",
  ),
  (
    OutcomeVariant::Info,
    "info",
    "OutcomeVariant::Info",
    "Build queued",
    "2 builds are ahead of yours.",
    "Cancel",
    "View queue",
  ),
  (
    OutcomeVariant::NotFound,
    "not found",
    "OutcomeVariant::NotFound",
    "Nothing here",
    "This page was moved or never existed.",
    "Go back",
    "Home",
  ),
];

/// How many buttons the playground gives it.
const ACTIONS: [(&str, usize); 3] = [("none", 0), ("one", 1), ("two", 2)];

/// The board's outcome of `variant`, with its two buttons.
fn of(index: usize) -> Outcome {
  let (variant, _, _, title, description, least, main) = VARIANTS[index];
  Outcome::new(variant, title)
    .description(description)
    .action(Button::new(least).variant(ButtonVariant::Outline))
    .action(Button::new(main))
}

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new().spacing(9.0).child(shown).child(quiet(caption, 12.0))
}

/// A line of a summary: what, and how much.
fn line(what: &str, much: &str) -> Row {
  Row::new()
    .width(FILL)
    .justify(Justify::SpaceBetween)
    .child(quiet(what, 12.0))
    .child(text(much, 13.0, lurq::layout::text_style::FontWeight::Medium))
}

pub struct OutcomePage {
  frame: Frame,
  playground: Playground,
  variant: Signal<usize>,
  title: Signal<String>,
  description: Signal<String>,
  actions: Signal<usize>,
  bordered: Signal<bool>,
  extra: Signal<bool>,
}

impl OutcomePage {
  /// The playground: the outcome as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let index = self.variant.get().min(VARIANTS.len() - 1);
    let (variant, variant_name, variant_code, _, _, least, main) = VARIANTS[index];
    let (title, description) = (self.title.get(), self.description.get());
    let (_, actions) = ACTIONS[self.actions.get().min(ACTIONS.len() - 1)];
    let (bordered, extra) = (self.bordered.get(), self.extra.get());

    let plain = {
      let (title, description) = (title.clone(), description.clone());
      move || -> Element {
        let mut outcome = Outcome::new(variant, &title).bordered(bordered);
        if !description.is_empty() {
          outcome = outcome.description(&description);
        }
        if extra {
          outcome = outcome.extra(InlineCode::new("cargo test --workspace"));
        }
        if actions == 2 {
          outcome = outcome.action(Button::new(least).variant(ButtonVariant::Outline));
        }
        if actions >= 1 {
          outcome = outcome.action(Button::new(main));
        }
        outcome.into()
      }
    };

    let mut code = vec![format!("Outcome::new({variant_code}, {title:?})")];
    if !description.is_empty() {
      code.push(format!("    .description({description:?})"));
    }
    if !bordered {
      code.push("    .bordered(false)".to_owned());
    }
    if extra {
      code.push("    .extra(InlineCode::new(\"cargo test --workspace\"))".to_owned());
    }
    if actions == 2 {
      code.push(format!(
        "    .action(Button::new({least:?}).variant(ButtonVariant::Outline))"
      ));
    }
    if actions >= 1 {
      code.push(format!("    .action(Button::new({main:?}))"));
    }

    let mut tree = vec![format!(
      "root      {}   {variant_name}{}   status",
      self.playground.measured(),
      if bordered { " · box" } else { "" }
    )];
    tree.push(format!("├─ mark    48 · {}", variant.icon()));
    tree.push("│  └─ icon   24".to_owned());
    tree.push("├─ text".to_owned());
    tree.push("│  ├─ title".to_owned());
    if !description.is_empty() {
      tree.push("│  └─ description".to_owned());
    }
    if extra {
      tree.push("├─ extra".to_owned());
    }
    if actions > 0 {
      tree.push(format!("└─ actions   {actions}"));
    }

    let variants: Vec<(&str, usize)> = VARIANTS.iter().enumerate().map(|(index, v)| (v.1, index)).collect();
    let counts: Vec<(&str, usize)> = ACTIONS.iter().enumerate().map(|(index, a)| (a.0, index)).collect();
    // Choosing a variant says what the board says in it.
    let chosen = (self.variant.clone(), self.title.clone(), self.description.clone());
    let to_actions = self.actions.clone();
    let reset = (
      self.variant.clone(),
      self.title.clone(),
      self.description.clone(),
      self.actions.clone(),
      self.bordered.clone(),
      self.extra.clone(),
    );
    let make = plain.clone();
    self.playground.card(
      ctx,
      Spec {
        title: "Outcome",
        make: Arc::new(move || make()),
        shown: plain(),
        fill: false,
        forceable: false,
        facts: variant_name.to_owned(),
        event: "",
        code,
        tree,
        controls: vec![
          prop(
            "variant",
            "OutcomeVariant",
            self.playground.select(0, &variants, &index, move |index| {
              chosen.0.set(index);
              chosen.1.set(VARIANTS[index].3.to_owned());
              chosen.2.set(VARIANTS[index].4.to_owned());
            }),
          ),
          prop("title", "&str", field(&self.title)),
          prop("description", "&str", field(&self.description)),
          divider(),
          flag("extra", &self.extra),
          prop(
            "action",
            "impl Into<Element>",
            segmented_fill(&counts, &self.actions.get(), move |index| to_actions.set(index)),
          ),
          flag("bordered", &self.bordered),
        ],
        changed: [
          index != 0,
          title != VARIANTS[index].3,
          description != VARIANTS[index].4,
          extra,
          actions != 2,
          !bordered,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          reset.0.set(0);
          reset.1.set(VARIANTS[0].3.to_owned());
          reset.2.set(VARIANTS[0].4.to_owned());
          reset.3.set(2);
          reset.4.set(true);
          reset.5.set(false);
        }),
      },
    )
  }
}

impl Component for OutcomePage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 1),
      variant: ctx.signal(0),
      title: ctx.signal(VARIANTS[0].3.to_owned()),
      description: ctx.signal(VARIANTS[0].4.to_owned()),
      actions: ctx.signal(2),
      bordered: ctx.signal(true),
      extra: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let variants = (0..VARIANTS.len()).fold(
      Row::new()
        .width(FILL)
        .spacing(28.0)
        .wrap()
        .align_items(Alignment::Start),
      |row, index| row.child(item(of(index), VARIANTS[index].1)),
    );
    let summary = Column::new()
      .width(FILL)
      .spacing(8.0)
      .child(Separator::horizontal())
      .child(line("Amount", "€ 48.00"))
      .child(line("Card", "•••• 4242"))
      .child(line("Reference", "INV-1042"));
    let extra = Row::new().width(FILL).child(
      Outcome::new(OutcomeVariant::Success, "Payment received")
        .description("A receipt is on its way to your inbox.")
        .icon(icons::CHECK_CIRCLE)
        .extra(summary)
        .action(Button::new("Done")),
    );
    let page = Column::new()
      .width(FILL)
      .padding(32.0)
      .align_items(Alignment::Center)
      .child(
        Outcome::new(OutcomeVariant::NotFound, "Nothing here")
          .description("This page was moved or never existed.")
          .bordered(false)
          .action(
            Button::new("Go back")
              .variant(ButtonVariant::Outline)
              .icon(icons::ARROW_LEFT),
          )
          .action(Button::new("Home")),
      )
      .child(
        Row::new()
          .padding_top(16.0)
          .child(Badge::new("404").variant(BadgeVariant::Secondary)),
      );
    let rows = [
      (
        "variant",
        "OutcomeVariant",
        "",
        "What came of it: success, error, warning, info, or not found. Its mark and its colors.",
      ),
      (
        "title",
        "&str",
        "",
        "What came of it, in a line. It is what the outcome is called.",
      ),
      ("description", "&str", "", "More of it, under the title."),
      (
        "icon",
        "impl Into<Name>",
        "the variant's",
        "Another icon in the mark, in the variant's colors.",
      ),
      (
        "media",
        "impl Into<Element>",
        "",
        "An element of the app's in place of the mark.",
      ),
      (
        "extra",
        "impl Into<Element>",
        "",
        "What stands under the text, over the buttons, as wide as the outcome.",
      ),
      (
        "action",
        "impl Into<Element>",
        "",
        "A button at the bottom, after those there: the least first, the main one last.",
      ),
      (
        "bordered",
        "bool",
        "true",
        "A box of its own with a fill and an edge; none on a page or in a dialog.",
      ),
      ("width", "impl Into<Length>", "300", "A width of its own."),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Data", "Result"],
        title: "Result",
        description: "What a page or a dialog says when something is over: payment received, build failed, nothing \
                      here. In the code it is Outcome, so that a glob import does not hide std's Result.",
        import: Some("use lurq::ui::{Outcome, OutcomeVariant};"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Outcome is a control on the right. It is read; its buttons are operated as themselves.",
          playground,
        ),
        Section::new(
          "Variants",
          "A round mark of 48 in the variant's muted color, its icon in the variant's color; not found is the muted \
           fill with the foreground.",
          self.frame.demo(
            0,
            variants,
            "Outcome::new(OutcomeVariant::Success, ..)",
            &[
              "Outcome::new(OutcomeVariant::Success, \"Build succeeded\")",
              "    .description(\"412 tests passed in 38 seconds.\")",
              "    .action(Button::new(\"View log\").variant(ButtonVariant::Outline))",
              "    .action(Button::new(\"Deploy\"))",
              "Outcome::new(OutcomeVariant::Error, \"Build failed\")",
              "    .description(\"3 tests failed in render/blur.rs.\")",
              "    .action(Button::new(\"View log\").variant(ButtonVariant::Outline))",
              "    .action(Button::new(\"Retry\"))",
              "Outcome::new(OutcomeVariant::Warning, \"Built with warnings\")",
              "    .description(\"12 warnings, no errors.\")",
              "    .action(Button::new(\"View log\").variant(ButtonVariant::Outline))",
              "    .action(Button::new(\"Continue\"))",
              "Outcome::new(OutcomeVariant::Info, \"Build queued\")",
              "    .description(\"2 builds are ahead of yours.\")",
              "    .action(Button::new(\"Cancel\").variant(ButtonVariant::Outline))",
              "    .action(Button::new(\"View queue\"))",
              "Outcome::new(OutcomeVariant::NotFound, \"Nothing here\")",
              "    .description(\"This page was moved or never existed.\")",
              "    .action(Button::new(\"Go back\").variant(ButtonVariant::Outline))",
              "    .action(Button::new(\"Home\"))",
            ],
          ),
        ),
        Section::new(
          "More under the text",
          "What the app puts under the text stands over the buttons, as wide as the outcome: a summary, a list. The \
           mark may take another icon, in the variant's colors.",
          self.frame.demo(
            1,
            extra,
            ".extra(element)",
            &[
              "let summary = Column::new()",
              "    .spacing(8.0)",
              "    .child(Separator::horizontal())",
              "    .child(line(\"Amount\", \"€ 48.00\"))",
              "    .child(line(\"Card\", \"•••• 4242\"))",
              "    .child(line(\"Reference\", \"INV-1042\"));",
              "",
              "Outcome::new(OutcomeVariant::Success, \"Payment received\")",
              "    .description(\"A receipt is on its way to your inbox.\")",
              "    .icon(icons::CHECK_CIRCLE)",
              "    .extra(summary)",
              "    .action(Button::new(\"Done\"))",
            ],
          ),
        ),
        Section::new(
          "On a page",
          "With no box it stands on a page or in a dialog, centred by what holds it.",
          self.frame.demo(
            2,
            page,
            ".bordered(false)",
            &[
              "Column::new()",
              "    .align_items(Alignment::Center)",
              "    .child(",
              "        Outcome::new(OutcomeVariant::NotFound, \"Nothing here\")",
              "            .description(\"This page was moved or never existed.\")",
              "            .bordered(false)",
              "            .action(Button::new(\"Go back\").variant(ButtonVariant::Outline).icon(icons::ARROW_LEFT))",
              "            .action(Button::new(\"Home\")),",
              "    )",
              "    .child(Badge::new(\"404\").variant(BadgeVariant::Secondary))",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What an outcome is given.",
          props_table(&rows, &["variant", "title"], self.frame.room()),
        ),
      ],
    )
  }
}
