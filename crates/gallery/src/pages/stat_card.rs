//! The page of the stat card: its playground, the board's frames per second, a row of cards of a dashboard, and the
//! small card with a sparkline.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx, theme::PaletteColor},
  components::Row,
  core::Signal,
  node::Element,
  ui::{Sparkline, StatCard, StatCardPart, StatCardSize, StatCardVariant},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{props_table, segmented_fill},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

const VARIANTS: [(&str, StatCardVariant); 5] = [
  ("Neutral", StatCardVariant::Neutral),
  ("Info", StatCardVariant::Info),
  ("Success", StatCardVariant::Success),
  ("Warning", StatCardVariant::Warning),
  ("Destructive", StatCardVariant::Destructive),
];

const SIZES: [(&str, StatCardSize); 2] = [("Md", StatCardSize::Md), ("Sm", StatCardSize::Sm)];

const FRAMES: [f64; 8] = [118.0, 124.0, 121.0, 133.0, 129.0, 141.0, 138.0, 144.0];

#[derive(Clone, Copy)]
struct Props {
  size: usize,
  variant: usize,
  delta: bool,
  description: bool,
  media: bool,
  long: bool,
  click: bool,
}

pub struct StatCardPage {
  frame: Frame,
  playground: Playground,
  size: Signal<usize>,
  variant: Signal<usize>,
  delta: Signal<bool>,
  description: Signal<bool>,
  media: Signal<bool>,
  long: Signal<bool>,
  click: Signal<bool>,
}

fn card(props: Props, record: Option<Arc<dyn Fn(String) + Send + Sync>>) -> StatCard {
  let label = if props.long {
    "Frames per second on the second monitor, at 144 Hz"
  } else {
    "Frames per second"
  };
  let mut card = StatCard::new(label, "144")
    .size(SIZES[props.size].1)
    .variant(VARIANTS[props.variant].1)
    .style(StatCardPart::Root, |s| s.width(220.0));
  if props.delta {
    card = card.delta("+12.5%");
  }
  if props.description {
    card = card.description("vs 0.59");
  }
  if props.media {
    card = card.media(
      Sparkline::new(FRAMES)
        .color(PaletteColor::Success)
        .label("Frames, the last minute"),
    );
  }
  if props.click
    && let Some(record) = record
  {
    card = card.on_click(move |_| record("on_click".to_owned()));
  }
  card
}

impl StatCardPage {
  fn props(&self) -> Props {
    Props {
      size: self.size.get().min(1),
      variant: self.variant.get().min(VARIANTS.len() - 1),
      delta: self.delta.get(),
      description: self.description.get(),
      media: self.media.get(),
      long: self.long.get(),
      click: self.click.get(),
    }
  }

  fn play(&self, ctx: &mut Ctx) -> Element {
    let props = self.props();
    let record: Arc<dyn Fn(String) + Send + Sync> = Arc::new(self.playground.recorder());
    let shown = card(props, Some(record)).force(if props.click { self.playground.forced() } else { None });
    let make = move || -> Element { card(props, None).into() };
    let mut code = vec!["StatCard::new(\"Frames per second\", \"144\")".to_owned()];
    if props.size == 1 {
      code.push("    .size(StatCardSize::Sm)".to_owned());
    }
    if props.delta {
      code.push("    .delta(\"+12.5%\")".to_owned());
    }
    code.push(format!("    .variant(StatCardVariant::{})", VARIANTS[props.variant].0));
    if props.description {
      code.push("    .description(\"vs 0.59\")".to_owned());
    }
    if props.media {
      code.push("    .media(Sparkline::new(frames).color(PaletteColor::Success))".to_owned());
    }
    if props.click {
      code.push("    .on_click(|_| /* open the frames */)".to_owned());
    }
    let tree = vec![
      format!(
        "root        {}   the card, {} in",
        self.playground.measured(),
        if props.size == 1 { 16 } else { 20 }
      ),
      format!("├─ label    {}, muted", if props.size == 1 { 12 } else { 13 }),
      format!(
        "├─ body     value {} at 600{}",
        if props.size == 1 { 24 } else { 28 },
        if props.media { " · media at the end" } else { "" }
      ),
      "└─ footer   the change, a badge in its tone · description mono 11, 8 apart".to_owned(),
    ];
    let segmented = |signal: &Signal<usize>, names: Vec<(&'static str, usize)>| {
      let choose = signal.clone();
      segmented_fill(&names, &signal.get(), move |index| choose.set(index))
    };
    let places = |names: &[&'static str]| -> Vec<(&'static str, usize)> {
      names
        .iter()
        .copied()
        .enumerate()
        .map(|(index, name)| (name, index))
        .collect()
    };
    let controls = vec![
      prop(
        "size",
        "StatCardSize",
        segmented(&self.size, places(&SIZES.map(|(name, _)| name))),
      ),
      prop(
        "variant",
        "StatCardVariant",
        segmented(&self.variant, places(&VARIANTS.map(|(name, _)| name))),
      ),
      divider(),
      flag("delta", &self.delta),
      flag("description", &self.description),
      flag("media: a sparkline", &self.media),
      flag("a long label", &self.long),
      flag("on_click", &self.click),
    ];
    let changed = [
      props.size != 0,
      props.variant != 2,
      !props.delta,
      !props.description,
      props.media,
      props.long,
      props.click,
    ]
    .into_iter()
    .filter(|changed| *changed)
    .count();
    let resets = (
      [(self.size.clone(), 0), (self.variant.clone(), 2)],
      [
        (self.delta.clone(), true),
        (self.description.clone(), true),
        (self.media.clone(), false),
        (self.long.clone(), false),
        (self.click.clone(), false),
      ],
    );
    self.playground.card(
      ctx,
      Spec {
        title: "Stat Card",
        make: Arc::new(make),
        shown: Shown::of(self.playground.styled(shown)),
        fill: false,
        forceable: props.click,
        facts: "144".to_owned(),
        event: if props.click { "on_click" } else { "" },
        code,
        tree,
        controls,
        changed,
        reset: Arc::new(move || {
          for (signal, value) in &resets.0 {
            signal.set(*value);
          }
          for (signal, value) in &resets.1 {
            signal.set(*value);
          }
        }),
      },
    )
  }
}

impl Component for StatCardPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      size: ctx.signal(0),
      variant: ctx.signal(2),
      delta: ctx.signal(true),
      description: ctx.signal(true),
      media: ctx.signal(false),
      long: ctx.signal(false),
      click: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let board = StatCard::new("Frames per second", "144")
      .delta("+12.5%")
      .variant(StatCardVariant::Success)
      .description("vs 0.59")
      .style(StatCardPart::Root, |s| s.width(200.0));
    let width = |card: StatCard| card.style(StatCardPart::Root, |s| s.width(180.0));
    let dashboard = Row::new()
      .spacing(16.0)
      .wrap()
      .child(width(
        StatCard::new("Build time", "4m 12s")
          .delta("-18s")
          .variant(StatCardVariant::Success)
          .description("vs last week"),
      ))
      .child(width(
        StatCard::new("Failed runs", "7")
          .delta("+3")
          .variant(StatCardVariant::Destructive)
          .description("vs last week"),
      ))
      .child(width(
        StatCard::new("Open issues", "212")
          .delta("0")
          .variant(StatCardVariant::Neutral)
          .description("vs last week"),
      ));
    let small = StatCard::new("Frames per second", "144")
      .size(StatCardSize::Sm)
      .media(
        Sparkline::new(FRAMES)
          .color(PaletteColor::Success)
          .label("Frames, the last minute"),
      )
      .style(StatCardPart::Root, |s| s.width(200.0));

    let rows = [
      (
        "new",
        "(&str, &str)",
        "",
        "What the number is, and the number as the app writes it.",
      ),
      (
        "delta",
        "&str",
        "",
        "How it moved, as the app writes it: a badge in the variant's tone.",
      ),
      (
        "variant",
        "StatCardVariant",
        "Neutral",
        "The tone of the change: Info, Success, Warning, Destructive.",
      ),
      (
        "description",
        "&str",
        "",
        "What the change is measured against: \"vs 0.59\".",
      ),
      (
        "media",
        "impl Into<Element>",
        "",
        "What stands at the end of the number's row: a sparkline of its last values.",
      ),
      (
        "size",
        "StatCardSize",
        "Md",
        "Sm: the card the board draws around its sparkline, 16 in, a value of 24.",
      ),
      (
        "on_click",
        "impl IntoMouseEventHandler",
        "",
        "The card is pressed and a stop of Tab. A second handler is called after the first.",
      ),
      (
        "style / rules",
        "StatCardPart",
        "",
        "Root, Label, Body, Value, Media, Footer, Description.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Charts", "Stat Card"],
        title: "Stat Card",
        description: "One number with what it is and how it moved: frames per second, the time a build takes, \
                      the runs that failed.",
        import: Some("use lurq::ui::StatCard;"),
        status: Some("Preview"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of StatCard is a control on the right. A stat card is read, not operated: with on_click it \
           is pressed, and each press is counted.",
          playground,
        ),
        Section::new(
          "Frames per second",
          "The board's stat card: its label, its number, and its change as a badge in its tone beside what it is \
           measured against.",
          self.frame.demo(
            0,
            board,
            "StatCard::new(label, value).delta(..).variant(..)",
            &[
              "StatCard::new(\"Frames per second\", \"144\")",
              "    .delta(\"+12.5%\")",
              "    .variant(StatCardVariant::Success)",
              "    .description(\"vs 0.59\")",
            ],
          ),
        ),
        Section::new(
          "A dashboard",
          "Cards side by side: a change for the better, one for the worse, and one with no judgment.",
          self.frame.demo(
            1,
            dashboard,
            ".variant(StatCardVariant::Destructive)",
            &[
              "StatCard::new(\"Build time\", \"4m 12s\").delta(\"-18s\").variant(StatCardVariant::Success)",
              "StatCard::new(\"Failed runs\", \"7\").delta(\"+3\").variant(StatCardVariant::Destructive)",
              "StatCard::new(\"Open issues\", \"212\").delta(\"0\")",
            ],
          ),
        ),
        Section::new(
          "With a sparkline",
          "The small size is the card the board draws around its sparkline, which media puts at the end of the \
           number's row.",
          self.frame.demo(
            2,
            small,
            ".size(StatCardSize::Sm).media(sparkline)",
            &[
              "StatCard::new(\"Frames per second\", \"144\")",
              "    .size(StatCardSize::Sm)",
              "    .media(Sparkline::new(frames).color(PaletteColor::Success))",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a stat card is given.",
          props_table(&rows, &["new"], self.frame.room()),
        ),
      ],
    )
  }
}
