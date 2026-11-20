//! The page of the card: its playground, the board's card, its variants,
//! cards that are pressed, and the settings card and the stat card an app
//! composes from it.

use std::sync::Arc;

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{PaletteColor, RadiusSize},
  },
  components::{Column, Row},
  core::Signal,
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{BoxShadowValue, Element},
  ui::{
    Badge, BadgeVariant, Button, ButtonSize, ButtonVariant, Card, CardPart, CardSize, CardVariant, Field, Input,
    Interaction, Length, Select, SelectSize, Switch, icons,
  },
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, icon, mono, phone, props_table, quiet, segmented_fill, text},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

/// The variants, as the board names them and as the code does.
const VARIANTS: [(CardVariant, &str, &str); 3] = [
  (CardVariant::Outline, "outline", "CardVariant::Outline"),
  (CardVariant::Elevated, "elevated", "CardVariant::Elevated"),
  (CardVariant::Filled, "filled", "CardVariant::Filled"),
];

/// The sizes, as the board names them and as the code does.
const SIZES: [(CardSize, &str, &str); 2] = [
  (CardSize::Md, "md", "CardSize::Md"),
  (CardSize::Sm, "sm", "CardSize::Sm"),
];

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .max_width(FILL)
    .spacing(10.0)
    .child(Row::new().max_width(FILL).child(shown))
    .child(quiet(caption, 12.0))
}

/// What is on a stage, side by side; what does not fit goes on to the next line.
fn items() -> Row {
  Row::new()
    .width(FILL)
    .spacing(24.0)
    .wrap()
    .align_items(Alignment::Start)
}

/// A card at the width the board draws it, and no wider than what holds it. It cuts nothing off: a card's
/// shadow falls outside it.
fn held(width: f32, card: Card) -> Column {
  Column::new()
    .width(width)
    .max_width(FILL)
    .overflow_visible()
    .child(card.width(Length::Fill))
}

/// A row of the content: what the setting is on the left, its switch at the end.
fn setting(title: &str, on: &Signal<bool>) -> Row {
  let set = on.clone();
  Row::new()
    .width(FILL)
    .align_items(Alignment::Center)
    .justify(Justify::SpaceBetween)
    .child(text(title, 14.0, FontWeight::Normal))
    .child(
      Switch::new(title)
        .label_hidden(true)
        .on(on.get())
        .on_change(move |next| set.set(next)),
    )
}

/// One of the board's cards of the styles: a small card 196 wide.
fn style_card(variant: CardVariant, title: &str, description: &str) -> Card {
  Card::new()
    .width(196.0)
    .size(CardSize::Sm)
    .variant(variant)
    .title(title)
    .description(description)
}

pub struct CardPage {
  frame: Frame,
  playground: Playground,
  title: Signal<String>,
  description: Signal<String>,
  action: Signal<bool>,
  footer: Signal<bool>,
  variant: Signal<usize>,
  size: Signal<usize>,
  pressable: Signal<bool>,
  disabled: Signal<bool>,
  dark: Signal<bool>,
  accent: Signal<String>,
  opened: Signal<String>,
  theme: Signal<String>,
}

impl CardPage {
  /// The playground: the card as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (title, description) = (self.title.get(), self.description.get());
    let (action, footer) = (self.action.get(), self.footer.get());
    let (variant, variant_name, variant_code) = VARIANTS[self.variant.get().min(VARIANTS.len() - 1)];
    let (size, size_name, size_code) = SIZES[self.size.get().min(SIZES.len() - 1)];
    let (pressable, disabled) = (self.pressable.get(), self.disabled.get());
    let plain = {
      let (title, description) = (title.clone(), description.clone());
      move || {
        let mut card = Card::new()
          .width(360.0)
          .variant(variant)
          .size(size)
          .disabled(disabled)
          .child(quiet("What the app puts in the card goes here.", 14.0));
        if !title.is_empty() {
          card = card.title(&title);
        }
        if !description.is_empty() {
          card = card.description(&description);
        }
        if action {
          card = card.action(
            Button::new("More")
              .variant(ButtonVariant::Ghost)
              .size(ButtonSize::Sm)
              .icon(icons::MORE_HORIZONTAL)
              .icon_only(true),
          );
        }
        if footer {
          card = card
            .footer(Button::new("Cancel").variant(ButtonVariant::Outline))
            .footer(Button::new("Save"));
        }
        card
      }
    };
    let record = self.playground.recorder();
    let mut card = plain().force(self.playground.forced());
    if pressable {
      card = card.on_click(move |_| record("on_click".to_owned()));
    }
    let make = {
      let plain = plain.clone();
      move || {
        let card = plain();
        if pressable { card.on_click(|_| ()) } else { card }.into()
      }
    };

    let mut code = vec!["Card::new()".to_owned()];
    if !title.is_empty() {
      code.push(format!("    .title({title:?})"));
    }
    if !description.is_empty() {
      code.push(format!("    .description({description:?})"));
    }
    if variant != CardVariant::Outline {
      code.push(format!("    .variant({variant_code})"));
    }
    if size != CardSize::Md {
      code.push(format!("    .size({size_code})"));
    }
    code.push("    .width(360.0)".to_owned());
    if action {
      code.push("    .action(Button::new(\"More\").icon(icons::MORE_HORIZONTAL).icon_only(true))".to_owned());
    }
    code.push("    .child(Text::new(\"What the app puts in the card goes here.\"))".to_owned());
    if footer {
      code.push("    .footer(Button::new(\"Cancel\").variant(ButtonVariant::Outline))".to_owned());
      code.push("    .footer(Button::new(\"Save\"))".to_owned());
    }
    if disabled {
      code.push("    .disabled(true)".to_owned());
    }
    if pressable {
      code.push("    .on_click(|_| open_project())".to_owned());
    }

    let padding = if size == CardSize::Sm { 16 } else { 24 };
    let mut tree = vec![format!(
      "root        {}   {variant_name} · radius 12{}",
      self.playground.measured(),
      if pressable { " · pressed" } else { "" }
    )];
    tree.push(format!(
      "{} content  padded {padding}",
      if footer { "├─" } else { "└─" }
    ));
    let bar = if footer { "│" } else { " " };
    if !title.is_empty() || !description.is_empty() || action {
      tree.push(format!(
        "{bar}  ├─ header  {}{}",
        match (title.is_empty(), description.is_empty()) {
          (false, false) => "title · description",
          (false, true) => "title",
          (true, false) => "description",
          (true, true) => "",
        },
        if action { " · action" } else { "" }
      ));
    }
    tree.push(format!("{bar}  └─ body"));
    if footer {
      tree.push("└─ footer   Cancel · Save, under a line".to_owned());
    }

    let to = (self.title.clone(), self.description.clone());
    let picks = [self.variant.clone(), self.size.clone()];
    let flags = [
      self.action.clone(),
      self.footer.clone(),
      self.pressable.clone(),
      self.disabled.clone(),
    ];
    let (to_variant, to_size) = (self.variant.clone(), self.size.clone());
    let variants: Vec<(&str, usize)> = VARIANTS.iter().enumerate().map(|(index, v)| (v.1, index)).collect();
    let sizes: Vec<(&str, usize)> = SIZES.iter().enumerate().map(|(index, s)| (s.1, index)).collect();
    self.playground.card(
      ctx,
      Spec {
        title: "Card",
        make: Arc::new(make),
        shown: Shown::of(self.playground.styled(card)),
        fill: false,
        forceable: pressable,
        facts: format!("{variant_name} · {size_name}"),
        event: "on_click",
        code,
        tree,
        controls: vec![
          prop("title", "&str", field(&self.title)),
          prop("description", "&str", field(&self.description)),
          prop(
            "variant",
            "CardVariant",
            segmented_fill(&variants, &self.variant.get(), move |index| to_variant.set(index)),
          ),
          prop(
            "size",
            "CardSize",
            segmented_fill(&sizes, &self.size.get(), move |index| to_size.set(index)),
          ),
          divider(),
          flag("action", &self.action),
          flag("footer", &self.footer),
          flag("on_click", &self.pressable),
          flag("disabled", &self.disabled),
        ],
        changed: [
          title != "Appearance",
          description != "How lurq looks on this device.",
          variant != CardVariant::Outline,
          size != CardSize::Md,
          action,
          !footer,
          pressable,
          disabled,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          to.0.set("Appearance".to_owned());
          to.1.set("How lurq looks on this device.".to_owned());
          for pick in &picks {
            pick.set(0);
          }
          for flag in &flags {
            flag.set(false);
          }
          flags[1].set(true);
        }),
      },
    )
  }

  /// The board's card: the appearance of the app, a field and a switch, and two buttons.
  fn appearance(&self) -> Card {
    Card::new()
      .title("Appearance")
      .description("How lurq looks on this device.")
      .child(
        Field::new("Accent", Input::new(self.accent.clone()).full_width(true)).description("Used for primary actions."),
      )
      .child(setting("Dark mode", &self.dark))
      .footer(Button::new("Cancel").variant(ButtonVariant::Outline))
      .footer(Button::new("Save"))
  }

  /// A project the page opens on a press.
  fn project(&self, name: &str, about: &str) -> Element {
    let (opened, said) = (self.opened.clone(), name.to_owned());
    // The column cuts nothing off: an elevated card's shadow falls outside it. It takes its share of a row,
    // and the stage's width where the cards are one under another.
    let holder = Column::new().overflow_visible();
    let holder = if phone() { holder.width(FILL) } else { holder.flex(1.0) };
    holder
      .child(
        Card::new()
          .variant(CardVariant::Elevated)
          .size(CardSize::Sm)
          .title(name)
          .description(about)
          .on_click(move |_| opened.set(said.clone())),
      )
      .into()
  }

  /// The board's settings card: an icon, what the setting is, and a select at its end, padded 14 with the
  /// corners of `radius-lg` and no shadow.
  fn settings_card(&self) -> Card {
    Card::new()
      .style(CardPart::Root, |s| {
        s.radius(RadiusSize::Lg).shadow(BoxShadowValue::none())
      })
      .style(CardPart::Content, |s| s.padding_x(14.0).padding_y(14.0))
      .child(
        Row::new()
          .width(FILL)
          .spacing(12.0)
          .align_items(Alignment::Center)
          .child(icon("palette", 20.0))
          .child(
            Column::new()
              .flex(1.0)
              .spacing(2.0)
              .child(text("App theme", 13.0, FontWeight::Medium))
              .child(text("Choose how lurq looks.", 12.0, FontWeight::Normal).color(PaletteColor::MutedForeground)),
          )
          // The board's select is 110 wide: a select keeps a width of its own, which would leave the text beside
          // it none.
          .child(
            Row::new().width(110.0).flex_shrink(0.0).child(
              Select::new(
                self.theme.clone(),
                [
                  ("system".to_owned(), "System"),
                  ("light".to_owned(), "Light"),
                  ("dark".to_owned(), "Dark"),
                ],
              )
              .size(SelectSize::Sm)
              .full_width(true)
              .label("App theme"),
            ),
          ),
      )
  }

  /// The board's stat card: what is counted, its number, and how it changed, laid out as a column is.
  fn stat_card() -> Card {
    Card::new()
      .width(200.0)
      .padding(20.0)
      .gap(8.0)
      .child(text("Frames per second", 13.0, FontWeight::Normal).color(PaletteColor::MutedForeground))
      .child(text("144", 28.0, FontWeight::SemiBold))
      .child(
        Row::new()
          .spacing(8.0)
          .align_items(Alignment::Center)
          .child(Badge::new("+12.5%").variant(BadgeVariant::Success))
          .child(mono("vs 0.59", 11.0).color(PaletteColor::MutedForeground)),
      )
  }
}

impl Component for CardPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      title: ctx.signal("Appearance".to_owned()),
      description: ctx.signal("How lurq looks on this device.".to_owned()),
      action: ctx.signal(false),
      footer: ctx.signal(true),
      variant: ctx.signal(0),
      size: ctx.signal(0),
      pressable: ctx.signal(false),
      disabled: ctx.signal(false),
      dark: ctx.signal(true),
      accent: ctx.signal("Zinc".to_owned()),
      opened: ctx.signal(String::new()),
      theme: ctx.signal("system".to_owned()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let styles = items()
      .child(item(
        style_card(CardVariant::Outline, "Outline", "A hairline border."),
        "outline",
      ))
      .child(item(
        style_card(CardVariant::Elevated, "Elevated", "A soft shadow, no border."),
        "elevated",
      ))
      .child(item(
        style_card(CardVariant::Filled, "Filled", "A muted fill for grouping."),
        "filled",
      ))
      .child(item(
        style_card(CardVariant::Outline, "Interactive", "Border and lift on hover.")
          .on_click(|_| ())
          .force(Some(Interaction::Hovered)),
        "interactive · hover",
      ))
      .child(item(
        style_card(CardVariant::Outline, "Focused", "From the keyboard.")
          .on_click(|_| ())
          .force(Some(Interaction::FocusVisible)),
        "interactive · focus-visible",
      ));
    let opened = self.opened.get();
    let cards = [
      self.project("lurq", "A Rust UI toolkit with GPU-backed rendering."),
      self.project("gallery", "Every component of the design system, a page each."),
      self.project("devtools", "The tree, the styles and the timings of a running app."),
    ];
    // Side by side where each has room for its words, one under another in a window as narrow as a phone.
    let together: Element = if phone() {
      cards
        .into_iter()
        .fold(
          Column::new().width(FILL).spacing(16.0).overflow_visible(),
          |column, card| column.child(card),
        )
        .into()
    } else {
      cards
        .into_iter()
        .fold(
          Row::new()
            .width(FILL)
            .spacing(16.0)
            .overflow_visible()
            .align_items(Alignment::Stretch),
          |row, card| row.child(card),
        )
        .into()
    };
    let projects = Column::new()
      .width(FILL)
      .spacing(12.0)
      .overflow_visible()
      .child(together)
      .child(quiet(
        &if opened.is_empty() {
          "nothing opened yet".to_owned()
        } else {
          format!("opened: {opened}")
        },
        12.0,
      ));
    let composed = items()
      .child(item(held(340.0, self.settings_card()), "settings card"))
      .child(item(Self::stat_card(), "stat card"));
    // Cards of one height in a row, each a share of it: the body takes the height, and what grows in it fills it.
    let figure = |label: &str, value: &str, note: Option<&str>| {
      let mut card = Card::new()
        .height(118.0)
        .flex(1.0)
        .min_width(150.0)
        .padding(16.0)
        .gap(4.0)
        .child(quiet(label, 12.0))
        .child(text(value, 22.0, FontWeight::SemiBold))
        .child(Column::new().flex(1.0));
      if let Some(note) = note {
        card = card.child(quiet(note, 12.0));
      }
      card
    };
    let figures = Row::new()
      .width(FILL)
      .spacing(12.0)
      .wrap()
      .child(figure("Runs", "1,284", Some("12% more than last week")))
      .child(figure("Tokens", "3.7M", None))
      .child(figure("Cost", "$41.20", Some("in budget")));
    let rows = [
      (
        "title",
        "&str",
        "",
        "What the card is about, 16/600. It is what the card is called unless it has a label, and wraps in its width.",
      ),
      (
        "label",
        "&str",
        "the title",
        "What the card is called to what reads the tree.",
      ),
      (
        "description",
        "&str",
        "",
        "More of it, under the title, 14 in the muted color.",
      ),
      (
        "action",
        "impl Into<Element>",
        "",
        "At the end of the header, beside the title: a button, a menu, a badge. A second takes its place.",
      ),
      (
        "child / children",
        "impl Into<Element>",
        "",
        "Added to the body, under the header, 16 apart.",
      ),
      (
        "footer",
        "impl Into<Element>",
        "",
        "Added to a row at the end of the card, under a line: its least action first.",
      ),
      (
        "variant",
        "CardVariant",
        "Outline",
        "Outline: an edge and the faintest shadow. Elevated: a soft shadow. Filled: the muted color.",
      ),
      (
        "size",
        "CardSize",
        "Md",
        "Md is padded 24; Sm is padded 16, with smaller text.",
      ),
      (
        "width",
        "impl Into<Length>",
        "",
        "A width of its own. Without one the card is as wide as the room it is given.",
      ),
      (
        "height / min_width / max_width / min_height / max_height",
        "impl Into<Length>",
        "",
        "A height of its own, and the least and the most it is wide and high. Given a height, the card gives \
         what its header leaves to its body, and its footer stays at the bottom.",
      ),
      (
        "padding",
        "impl Into<Padding>",
        "the size's",
        "The room inside it around its header and its body. The footer keeps its own.",
      ),
      (
        "gap",
        "impl Into<SpacingValue>",
        "the size's",
        "The room between its children, and between its header and its body.",
      ),
      (
        "align_items",
        "Alignment",
        "Start",
        "How its children are aligned across its width, as a column aligns them.",
      ),
      (
        "flex / flex_shrink",
        "f32",
        "",
        "Its share of the room left in a row or a column, and what it gives up where there is too little.",
      ),
      (
        "absolute",
        "x, y, width, height",
        "",
        "Where it stands in the Stack that holds it.",
      ),
      (
        "describe",
        "name, value",
        "",
        "What its surface says to tooling: the state of what it shows.",
      ),
      (
        "on_click",
        "impl Fn(MouseEvent)",
        "",
        "Makes the card pressed as a whole: one stop of Tab, an edge of the ring's color and a lift under the \
         pointer. A press of its action or its footer is theirs.",
      ),
      (
        "disabled",
        "bool",
        "false",
        "Half the opacity; a pressed card is not pressed.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Surfaces", "Card"],
        title: "Card",
        description: "A surface that holds one thing: a group of settings, a summary of a project, a form of a few \
                      fields. A title, a description and an action in its header, its content, and a footer of \
                      buttons, each where it has them.",
        import: Some("use lurq::ui::Card;"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Card is a control on the right. With on_click on, the card on the stage is pressed as a \
           whole, and each press is counted.",
          playground,
        ),
        Section::new(
          "Card",
          "The board's card: a header, a field and a switch in its content, and two buttons in its footer. A card \
           that is not pressed is read: Tab goes to each control in it.",
          self.frame.demo(
            0,
            items().child(held(360.0, self.appearance())),
            "Card::new().title(..).description(..).child(..).footer(..)",
            &[
              "Card::new()",
              "    .width(360.0)",
              "    .title(\"Appearance\")",
              "    .description(\"How lurq looks on this device.\")",
              "    .child(Field::new(\"Accent\", Input::new(accent.clone())).description(\"Used for primary actions.\"))",
              "    .child(dark_mode_row)",
              "    .footer(Button::new(\"Cancel\").variant(ButtonVariant::Outline))",
              "    .footer(Button::new(\"Save\"))",
            ],
          ),
        ),
        Section::new(
          "Variants",
          "Outline is the card of most places, elevated stands over a busy page, filled groups what is in it. A \
           card that is pressed takes an edge of the ring's color and the medium shadow under the pointer.",
          self.frame.demo(
            1,
            styles,
            ".variant(CardVariant::Elevated)",
            &[
              "Card::new().size(CardSize::Sm).title(\"Outline\").description(\"A hairline border.\")",
              "Card::new().size(CardSize::Sm).variant(CardVariant::Elevated).title(\"Elevated\")",
              "Card::new().size(CardSize::Sm).variant(CardVariant::Filled).title(\"Filled\")",
              "",
              "// Drawn under the pointer with nothing pointing at it: for a gallery, a test.",
              "Card::new().size(CardSize::Sm).title(\"Interactive\")",
              "    .on_click(|_| open())",
              "    .force(Some(Interaction::Hovered))",
            ],
          ),
        ),
        Section::new(
          "Pressed as a whole",
          "Each card is one stop of Tab; Enter or Space opens it. Side by side, each is in a column that takes its \
           share of the row.",
          self.frame.demo(
            2,
            projects,
            ".on_click(move |_| opened.set(\"lurq\"))",
            &[
              "// opened: Signal<String>",
              "Row::new()",
              "    .spacing(16.0)",
              "    .align_items(Alignment::Stretch)",
              "    .child(Column::new().flex(1.0).child(",
              "        Card::new()",
              "            .variant(CardVariant::Elevated)",
              "            .size(CardSize::Sm)",
              "            .title(\"lurq\")",
              "            .description(\"A Rust UI toolkit with GPU-backed rendering.\")",
              "            .on_click(move |_| opened.set(\"lurq\".to_owned())),",
              "    ))",
            ],
          ),
        ),
        Section::new(
          "Composed from a card",
          "A surface of the app's own is a card with its parts styled, or laid out from its own builder as a \
           column is: a padding and a radius of its own, and what it holds as the content. The board's settings card and stat card, drawn here by hand, are components \
           of their own: SettingsCard and StatCard.",
          self.frame.demo(
            3,
            composed,
            ".style(CardPart::Content, |s| s.padding_x(14.0).padding_y(14.0))",
            &[
              "Card::new()",
              "    .width(340.0)",
              "    .style(CardPart::Root, |s| s.radius(RadiusSize::Lg).shadow(BoxShadowValue::none()))",
              "    .style(CardPart::Content, |s| s.padding_x(14.0).padding_y(14.0))",
              "    .child(Row::new().child(icon).child(text).child(select))",
              "",
              "// Laid out as a column is: its padding, the room between its children.",
              "Card::new()",
              "    .width(200.0)",
              "    .padding(20.0)",
              "    .gap(8.0)",
              "    .child(label)",
              "    .child(value)",
              "    .child(delta)",
            ],
          ),
        ),
        Section::new(
          "Of one height, side by side",
          "A card given a height of its own gives what its header leaves to its body: a child with flex(1.0) fills \
           it, so the last line of each figure stands at the bottom whatever is over it. Where the row is narrow, \
           the cards go on to the next line. What is in a card is \
           clipped to its rounded corners.",
          self.frame.demo(
            4,
            figures,
            ".height(118.0).flex(1.0)",
            &[
              "Row::new()",
              "    .spacing(12.0)",
              "    .child(",
              "        Card::new()",
              "            .height(118.0)",
              "            .flex(1.0)",
              "            .padding(16.0)",
              "            .gap(4.0)",
              "            .child(Text::new(\"Runs\"))",
              "            .child(Text::new(\"1,284\"))",
              "            .child(Column::new().flex(1.0)) // the room left",
              "            .child(Text::new(\"12% more than last week\")),",
              "    )",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a card is given. Every part is optional: a card with children alone is a padded surface.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
