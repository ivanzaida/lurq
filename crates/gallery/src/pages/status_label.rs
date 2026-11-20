//! The page of the status label: its playground, its tones, its two sizes, a
//! label whose icon turns while what it says is under way, words alone, and
//! labels in the rows of a table of runs and in the header of a step.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx, theme::PaletteColor},
  components::{Column, Row},
  core::Signal,
  layout::{Alignment, text_style::FontWeight},
  node::Element,
  ui::{Separator, SpinMotion, StatusLabel, StatusLabelSize, StatusLabelVariant, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, card, mono, props_table, quiet, segmented_fill, text},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

/// The tones, as the code names them, each with the word and the icon the page shows it with.
const VARIANTS: [(StatusLabelVariant, &str, &str, &str, &str); 6] = [
  (
    StatusLabelVariant::Neutral,
    "neutral",
    "StatusLabelVariant::Neutral",
    "Running",
    icons::PLAY_CIRCLE,
  ),
  (
    StatusLabelVariant::Muted,
    "muted",
    "StatusLabelVariant::Muted",
    "Queued",
    icons::HOURGLASS,
  ),
  (
    StatusLabelVariant::Info,
    "info",
    "StatusLabelVariant::Info",
    "In review",
    icons::EYE,
  ),
  (
    StatusLabelVariant::Success,
    "success",
    "StatusLabelVariant::Success",
    "Completed",
    icons::CHECK_CIRCLE,
  ),
  (
    StatusLabelVariant::Warning,
    "warning",
    "StatusLabelVariant::Warning",
    "Waiting for you",
    icons::ALERT_CIRCLE,
  ),
  (
    StatusLabelVariant::Destructive,
    "destructive",
    "StatusLabelVariant::Destructive",
    "Failed",
    icons::CLOSE_CIRCLE,
  ),
];

/// The sizes, as the code names them, with the side of the icon and the size of the words.
const SIZES: [(StatusLabelSize, &str, &str, &str); 2] = [
  (
    StatusLabelSize::Sm,
    "sm",
    "StatusLabelSize::Sm",
    "icon 12 · words 11 · gap 4",
  ),
  (
    StatusLabelSize::Md,
    "md",
    "StatusLabelSize::Md",
    "icon 14 · words 12 · gap 6",
  ),
];

/// The icons the playground offers, as the code names them: none first.
const ICONS: [(&str, Option<&str>, &str); 7] = [
  ("none", None, ""),
  ("spinner", Some(icons::SPINNER), "icons::SPINNER"),
  ("refresh", Some(icons::REFRESH), "icons::REFRESH"),
  ("check-circle", Some(icons::CHECK_CIRCLE), "icons::CHECK_CIRCLE"),
  ("close-circle", Some(icons::CLOSE_CIRCLE), "icons::CLOSE_CIRCLE"),
  ("alert-circle", Some(icons::ALERT_CIRCLE), "icons::ALERT_CIRCLE"),
  ("hourglass", Some(icons::HOURGLASS), "icons::HOURGLASS"),
];

/// The motions the playground offers: the theme's first, which is no call.
const MOTIONS: [(&str, Option<SpinMotion>, &str); 3] = [
  ("theme", None, ""),
  ("smooth", Some(SpinMotion::SMOOTH), "SpinMotion::SMOOTH"),
  ("stepped 12", Some(SpinMotion::stepped(12)), "SpinMotion::stepped(12)"),
];

/// The word the playground starts with.
const START: &str = "Running · 03 Implement";

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .max_width(FILL)
    .spacing(9.0)
    .child(Row::new().max_width(FILL).child(shown))
    .child(quiet(caption, 12.0))
}

/// What is on a stage, side by side; what does not fit goes on to the next line.
fn items() -> Row {
  Row::new().width(FILL).spacing(28.0).wrap().align_items(Alignment::End)
}

/// A row of the table of runs: the run's name, how long it took, and its state.
fn run(name: &str, took: &str, state: StatusLabel) -> Row {
  Row::new()
    .width(FILL)
    .padding_horizontal(16.0)
    .padding_vertical(10.0)
    .spacing(12.0)
    .align_items(Alignment::Center)
    .child(
      Column::new()
        .flex(1.0)
        .min_width(0.0)
        .child(text(name, 13.0, FontWeight::Medium).width(FILL)),
    )
    .child(mono(took, 12.0).color(PaletteColor::MutedForeground))
    .child(Row::new().width(150.0).min_width(0.0).child(state))
}

/// The props of the playground's label.
#[derive(Clone)]
struct Props {
  text: String,
  variant: usize,
  size: usize,
  icon: usize,
  spin: bool,
  motion: usize,
}

impl Props {
  fn label(&self) -> StatusLabel {
    let mut label = StatusLabel::new(&self.text)
      .variant(VARIANTS[self.variant].0)
      .size(SIZES[self.size].0)
      .spin(self.spin);
    if let Some(icon) = ICONS[self.icon].1 {
      label = label.icon(icon);
    }
    if let Some(motion) = MOTIONS[self.motion].1 {
      label = label.spin_motion(motion);
    }
    label
  }

  fn code(&self) -> Vec<String> {
    let mut code = vec![format!("StatusLabel::new({:?})", self.text)];
    if ICONS[self.icon].1.is_some() {
      code.push(format!("    .icon({})", ICONS[self.icon].2));
    }
    if VARIANTS[self.variant].0 != StatusLabelVariant::default() {
      code.push(format!("    .variant({})", VARIANTS[self.variant].2));
    }
    if SIZES[self.size].0 != StatusLabelSize::default() {
      code.push(format!("    .size({})", SIZES[self.size].2));
    }
    if self.spin {
      code.push("    .spin(true)".to_owned());
    }
    if MOTIONS[self.motion].1.is_some() {
      code.push(format!("    .spin_motion({})", MOTIONS[self.motion].2));
    }
    code
  }

  fn tree(&self, measured: String) -> Vec<String> {
    let mut tree = vec![format!(
      "root      {measured}   {} · {}   status {:?}",
      VARIANTS[self.variant].1, SIZES[self.size].1, self.text
    )];
    if let Some(icon) = ICONS[self.icon].1 {
      let turning = if self.spin { "   spinning" } else { "" };
      tree.push(format!("├─ icon   {icon}{turning}"));
    }
    tree.push(format!("└─ label  {:?}", self.text));
    tree
  }
}

pub struct StatusLabelPage {
  frame: Frame,
  playground: Playground,
  text: Signal<String>,
  /// The index of the tone in [`VARIANTS`].
  variant: Signal<usize>,
  /// The index of the size in [`SIZES`].
  size: Signal<usize>,
  /// The index of the icon in [`ICONS`].
  icon: Signal<usize>,
  spin: Signal<bool>,
  /// The index of the motion in [`MOTIONS`].
  motion: Signal<usize>,
}

impl StatusLabelPage {
  fn props(&self) -> Props {
    Props {
      text: self.text.get(),
      variant: self.variant.get().min(VARIANTS.len() - 1),
      size: self.size.get().min(SIZES.len() - 1),
      icon: self.icon.get().min(ICONS.len() - 1),
      spin: self.spin.get(),
      motion: self.motion.get().min(MOTIONS.len() - 1),
    }
  }

  /// The playground: the label as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let props = self.props();
    let variants: Vec<(&str, usize)> = VARIANTS.iter().enumerate().map(|(index, v)| (v.1, index)).collect();
    let sizes: Vec<(&str, usize)> = SIZES.iter().enumerate().map(|(index, s)| (s.1, index)).collect();
    let glyphs: Vec<(&str, usize)> = ICONS.iter().enumerate().map(|(index, i)| (i.0, index)).collect();
    let motions: Vec<(&str, usize)> = MOTIONS.iter().enumerate().map(|(index, m)| (m.0, index)).collect();
    let (to_variant, to_size) = (self.variant.clone(), self.size.clone());
    let (to_icon, to_motion) = (self.icon.clone(), self.motion.clone());
    let controls = vec![
      prop("label", "&str", field(&self.text)),
      prop(
        "variant",
        "StatusLabelVariant",
        self
          .playground
          .select(0, &variants, &props.variant, move |index| to_variant.set(index)),
      ),
      prop(
        "size",
        "StatusLabelSize",
        segmented_fill(&sizes, &props.size, move |index| to_size.set(index)),
      ),
      prop(
        "icon",
        "impl Into<Name>",
        self
          .playground
          .select(1, &glyphs, &props.icon, move |index| to_icon.set(index)),
      ),
      divider(),
      flag("spin", &self.spin),
      prop(
        "spin_motion",
        "SpinMotion",
        segmented_fill(&motions, &props.motion, move |index| to_motion.set(index)),
      ),
    ];
    let changed = [
      props.text != START,
      props.variant != 0,
      props.size != 1,
      props.icon != 1,
      !props.spin,
      props.motion != 0,
    ]
    .into_iter()
    .filter(|changed| *changed)
    .count();
    let reset = {
      let (text, variant, size) = (self.text.clone(), self.variant.clone(), self.size.clone());
      let (icon, spin, motion) = (self.icon.clone(), self.spin.clone(), self.motion.clone());
      Arc::new(move || {
        text.set(START.to_owned());
        variant.set(0);
        size.set(1);
        icon.set(1);
        spin.set(true);
        motion.set(0);
      })
    };
    self.playground.card(
      ctx,
      Spec {
        title: "Status Label",
        make: {
          let props = props.clone();
          Arc::new(move || props.label().into())
        },
        shown: Shown::of(self.playground.styled(props.label())),
        fill: false,
        forceable: false,
        facts: format!("{} · {}", VARIANTS[props.variant].1, SIZES[props.size].1),
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

impl Component for StatusLabelPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 2),
      text: ctx.signal(START.to_owned()),
      variant: ctx.signal(0),
      size: ctx.signal(1),
      icon: ctx.signal(1),
      spin: ctx.signal(true),
      motion: ctx.signal(0),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let variants = VARIANTS.iter().fold(items(), |row, (variant, name, _, word, icon)| {
      row.child(item(StatusLabel::new(word).icon(*icon).variant(*variant), name))
    });
    let sizes = SIZES.iter().fold(items(), |row, (size, name, _, numbers)| {
      row.child(item(
        StatusLabel::new("Completed")
          .icon(icons::CHECK_CIRCLE)
          .variant(StatusLabelVariant::Success)
          .size(*size),
        &format!("{name} · {numbers}"),
      ))
    });
    let running = items()
      .child(item(
        StatusLabel::new("Running · 03 Implement")
          .icon(icons::SPINNER)
          .spin(true),
        "spin",
      ))
      .child(item(
        StatusLabel::new("Syncing")
          .icon(icons::REFRESH)
          .variant(StatusLabelVariant::Info)
          .spin(true),
        "spin · info",
      ))
      .child(item(
        StatusLabel::new("Running")
          .icon(icons::SPINNER)
          .spin(true)
          .spin_motion(SpinMotion::stepped(12)),
        "stepped 12",
      ));
    let words = items()
      .child(item(
        StatusLabel::new("Draft").variant(StatusLabelVariant::Muted),
        "muted",
      ))
      .child(item(
        StatusLabel::new("Merged").variant(StatusLabelVariant::Success),
        "success",
      ))
      .child(item(
        StatusLabel::new("Skipped")
          .variant(StatusLabelVariant::Muted)
          .size(StatusLabelSize::Sm),
        "muted · sm",
      ));
    let runs = card()
      .width(560.0)
      .max_width(FILL)
      .child(run(
        "Implement the status label",
        "4m 12s",
        StatusLabel::new("Running · 03 Implement")
          .icon(icons::SPINNER)
          .spin(true)
          .size(StatusLabelSize::Sm),
      ))
      .child(Separator::new())
      .child(run(
        "Review the gallery's pages",
        "—",
        StatusLabel::new("Queued")
          .icon(icons::HOURGLASS)
          .variant(StatusLabelVariant::Muted)
          .size(StatusLabelSize::Sm),
      ))
      .child(Separator::new())
      .child(run(
        "Verify the design",
        "1m 03s",
        StatusLabel::new("Waiting for you")
          .icon(icons::ALERT_CIRCLE)
          .variant(StatusLabelVariant::Warning)
          .size(StatusLabelSize::Sm),
      ))
      .child(Separator::new())
      .child(run(
        "Build the release",
        "7m 48s",
        StatusLabel::new("Completed")
          .icon(icons::CHECK_CIRCLE)
          .variant(StatusLabelVariant::Success)
          .size(StatusLabelSize::Sm),
      ))
      .child(Separator::new())
      .child(run(
        "Run the tests on macOS",
        "2m 31s",
        StatusLabel::new("Failed")
          .icon(icons::CLOSE_CIRCLE)
          .variant(StatusLabelVariant::Destructive)
          .size(StatusLabelSize::Sm),
      ));
    let step = card()
      .width(420.0)
      .max_width(FILL)
      .padding(16.0)
      .spacing(6.0)
      .child(text("03 Implement", 16.0, FontWeight::SemiBold).width(FILL))
      .child(
        StatusLabel::new("Running · started 4 minutes ago")
          .icon(icons::SPINNER)
          .spin(true),
      );
    let rows = [
      (
        "label",
        "&str",
        "",
        "The words, on one line cut with an ellipsis where the room is short; what tooling calls it.",
      ),
      (
        "icon",
        "impl Into<Name>",
        "none",
        "An icon of the theme's set before the words, in their color.",
      ),
      (
        "variant",
        "impl Into<Name>",
        "Neutral",
        "A StatusLabelVariant: Neutral, Muted, Info, Success, Warning or Destructive; or a tone the recipe names.",
      ),
      (
        "size",
        "impl Into<Name>",
        "Md",
        "A StatusLabelSize: an icon of 14 and words of 12, 6 apart; or 12 and 11, 4 apart.",
      ),
      ("spin", "bool", "false", "Whether the icon turns, as a spinner does."),
      (
        "spin_motion",
        "SpinMotion",
        "the theme's",
        "How the icon turns where it spins: smooth, or in steps a second.",
      ),
      (
        "style, rules, state",
        "",
        "",
        "Over the theme's recipe: StatusLabelPart::Root, Icon, Label; StatusLabelState::Spinning.",
      ),
      (
        "id, key",
        "impl Into<Arc<str>>",
        "",
        "The root's id, with <id>-icon and <id>-label.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Data", "Status Label"],
        title: "Status Label",
        description: "What a thing is doing, as an icon and a word in one tone, with no fill and no edge: the \
                      state of a run in a row of a table.",
        import: Some("use lurq::ui::StatusLabel;"),
        status: Some("New"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of StatusLabel is a control on the right. It is read and not operated: it takes no press, \
           no focus and no stop of Tab.",
          playground,
        ),
        Section::new(
          "Variants",
          "The icon and the words in the tone's strong color, at medium weight: the foreground, the muted \
           foreground, info, success, warning and destructive.",
          self.frame.demo(
            0,
            variants,
            ".variant(StatusLabelVariant::Success)",
            &[
              "StatusLabel::new(\"Running\").icon(icons::PLAY_CIRCLE)",
              "StatusLabel::new(\"Queued\").icon(icons::HOURGLASS).variant(StatusLabelVariant::Muted)",
              "StatusLabel::new(\"In review\").icon(icons::EYE).variant(StatusLabelVariant::Info)",
              "StatusLabel::new(\"Completed\").icon(icons::CHECK_CIRCLE).variant(StatusLabelVariant::Success)",
              "StatusLabel::new(\"Waiting for you\").icon(icons::ALERT_CIRCLE).variant(StatusLabelVariant::Warning)",
              "StatusLabel::new(\"Failed\").icon(icons::CLOSE_CIRCLE).variant(StatusLabelVariant::Destructive)",
            ],
          ),
        ),
        Section::new(
          "Sizes",
          "Medium beside a title or in a header; small in a dense row of a table.",
          self.frame.demo(
            1,
            sizes,
            ".size(StatusLabelSize::Sm)",
            &[
              "StatusLabel::new(\"Completed\")",
              "    .icon(icons::CHECK_CIRCLE)",
              "    .variant(StatusLabelVariant::Success)",
              "    .size(StatusLabelSize::Sm)",
              "StatusLabel::new(\"Completed\")",
              "    .icon(icons::CHECK_CIRCLE)",
              "    .variant(StatusLabelVariant::Success)",
            ],
          ),
        ),
        Section::new(
          "Under way",
          "While what it says is under way its icon turns, as a spinner does: by the theme's motion, or in steps \
           of its own.",
          self.frame.demo(
            2,
            running,
            ".spin(true)",
            &[
              "StatusLabel::new(\"Running · 03 Implement\").icon(icons::SPINNER).spin(true)",
              "StatusLabel::new(\"Syncing\")",
              "    .icon(icons::REFRESH)",
              "    .variant(StatusLabelVariant::Info)",
              "    .spin(true)",
              "StatusLabel::new(\"Running\")",
              "    .icon(icons::SPINNER)",
              "    .spin(true)",
              "    .spin_motion(SpinMotion::stepped(12))",
            ],
          ),
        ),
        Section::new(
          "Words alone",
          "With no icon it is the words in the tone's color.",
          self.frame.demo(
            3,
            words,
            "StatusLabel::new(\"Draft\")",
            &[
              "StatusLabel::new(\"Draft\").variant(StatusLabelVariant::Muted)",
              "StatusLabel::new(\"Merged\").variant(StatusLabelVariant::Success)",
              "StatusLabel::new(\"Skipped\").variant(StatusLabelVariant::Muted).size(StatusLabelSize::Sm)",
            ],
          ),
        ),
        Section::new(
          "In a table of runs",
          "The state of each run in the last column, small. In a column narrower than its words they are cut \
           with an ellipsis; the icon keeps its size.",
          self.frame.demo(
            4,
            runs,
            ".size(StatusLabelSize::Sm)",
            &[
              "Row::new()",
              "    .child(name)",
              "    .child(took)",
              "    .child(Row::new().width(150.0).min_width(0.0).child(",
              "        StatusLabel::new(\"Running · 03 Implement\")",
              "            .icon(icons::SPINNER)",
              "            .spin(true)",
              "            .size(StatusLabelSize::Sm),",
              "    ))",
            ],
          ),
        ),
        Section::new(
          "In a step's header",
          "Under the title of a step, at its medium size.",
          self.frame.demo(
            5,
            step,
            "StatusLabel::new(..).spin(true)",
            &[
              "Column::new()",
              "    .child(title)",
              "    .child(",
              "        StatusLabel::new(\"Running · started 4 minutes ago\")",
              "            .icon(icons::SPINNER)",
              "            .spin(true),",
              "    )",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a status label is given.",
          props_table(&rows, &["label"], self.frame.room()),
        ),
      ],
    )
  }
}
