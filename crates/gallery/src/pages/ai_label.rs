//! The page of the AI label: its playground, the board's mark, its sizes,
//! another word, the label beside what it marks, and one with a tooltip that
//! explains it.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx, theme::PaletteColor},
  components::{Column, Row},
  core::Signal,
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::Element,
  ui::{AiLabel, AiLabelSize, Tooltip, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, card, props_table, quiet, segmented_fill, text},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

const SIZES: [(&str, AiLabelSize); 3] = [
  ("sm", AiLabelSize::Sm),
  ("md", AiLabelSize::Md),
  ("lg", AiLabelSize::Lg),
];

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

/// The props of the playground's label.
#[derive(Clone)]
struct Props {
  text: String,
  label: String,
  size: usize,
  icon: bool,
}

impl Props {
  fn label(&self) -> AiLabel {
    let mut label = AiLabel::new().text(&self.text).size(SIZES[self.size].1);
    if !self.label.is_empty() {
      label = label.label(&self.label);
    }
    if !self.icon {
      label = label.icon(None);
    }
    label
  }

  fn code(&self) -> Vec<String> {
    let mut code = vec!["AiLabel::new()".to_owned()];
    if self.text != "AI" {
      code.push(format!("    .text({:?})", self.text));
    }
    if !self.label.is_empty() {
      code.push(format!("    .label({:?})", self.label));
    }
    if SIZES[self.size].1 != AiLabelSize::Md {
      code.push(format!("    .size(AiLabelSize::{:?})", SIZES[self.size].1));
    }
    if !self.icon {
      code.push("    .icon(None)".to_owned());
    }
    code
  }

  fn tree(&self, measured: String) -> Vec<String> {
    let name = if self.label.is_empty() { &self.text } else { &self.label };
    let mut tree = vec![format!("root      {measured}   note {name:?}")];
    if self.icon {
      tree.push("├─ icon   sparkles".to_owned());
    }
    tree.push(format!("└─ label  {:?}", self.text));
    tree
  }
}

pub struct AiLabelPage {
  frame: Frame,
  playground: Playground,
  text: Signal<String>,
  label: Signal<String>,
  /// The index of the size in [`SIZES`].
  size: Signal<usize>,
  icon: Signal<bool>,
}

impl AiLabelPage {
  fn props(&self) -> Props {
    Props {
      text: self.text.get(),
      label: self.label.get(),
      size: self.size.get(),
      icon: self.icon.get(),
    }
  }

  /// The playground: the label as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let props = self.props();
    let size = self.size.clone();
    let sizes: Vec<(&str, usize)> = SIZES
      .iter()
      .enumerate()
      .map(|(index, (name, _))| (*name, index))
      .collect();
    let controls = vec![
      prop("text", "&str", field(&self.text)),
      prop("label", "&str", field(&self.label)),
      prop(
        "size",
        "AiLabelSize",
        segmented_fill(&sizes, &self.size.get(), move |chosen| size.set(chosen)),
      ),
      divider(),
      flag("icon", &self.icon),
    ];
    let changed = [
      props.text != "AI",
      !props.label.is_empty(),
      props.size != 1,
      !props.icon,
    ]
    .into_iter()
    .filter(|changed| *changed)
    .count();
    let reset = {
      let (text, label, size, icon) = (
        self.text.clone(),
        self.label.clone(),
        self.size.clone(),
        self.icon.clone(),
      );
      Arc::new(move || {
        text.set("AI".to_owned());
        label.set(String::new());
        size.set(1);
        icon.set(true);
      })
    };
    self.playground.card(
      ctx,
      Spec {
        title: "AI Label",
        make: {
          let props = props.clone();
          Arc::new(move || props.label().into())
        },
        shown: Shown::of(self.playground.styled(props.label())),
        fill: false,
        forceable: false,
        facts: "radius 4".to_owned(),
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

impl Component for AiLabelPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      text: ctx.signal("AI".to_owned()),
      label: ctx.signal(String::new()),
      size: ctx.signal(1),
      icon: ctx.signal(true),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let mark = items().child(item(AiLabel::new(), "ai label"));
    let sizes = items()
      .child(item(AiLabel::new().size(AiLabelSize::Sm), "sm · 18"))
      .child(item(AiLabel::new(), "md · 22"))
      .child(item(AiLabel::new().size(AiLabelSize::Lg), "lg · 26"));
    let words = items()
      .child(item(AiLabel::new().text("Draft"), "another word"))
      .child(item(
        AiLabel::new().text("Suggested").icon(icons::LIGHTBULB),
        "another icon",
      ))
      .child(item(AiLabel::new().icon(None), "no icon"));
    let beside = card()
      .width(320.0)
      .max_width(FILL)
      .padding(16.0)
      .spacing(8.0)
      .child(
        Row::new()
          .width(FILL)
          .spacing(8.0)
          .align_items(Alignment::Center)
          .child(text("Summary", 14.0, FontWeight::Medium))
          .child(AiLabel::new().size(AiLabelSize::Sm).label("Generated by AI")),
      )
      .child(
        Column::new().width(FILL).child(
          text(
            "Three reviewers asked for a smaller diff; the tests of the scroll pass on every platform.",
            13.0,
            FontWeight::Normal,
          )
          .color(PaletteColor::MutedForeground)
          .width(FILL),
        ),
      );
    let explained = Row::new()
      .width(FILL)
      .padding_vertical(72.0)
      .justify(Justify::Center)
      .child(
        Tooltip::new("Written by a model from the thread. Check it before you rely on it.")
          .force_open(Some(true))
          .mount(ctx, AiLabel::new()),
      );
    let rows = [
      (
        "text",
        "&str",
        "\"AI\"",
        "The word it draws: the app's word for it, or the word in another language.",
      ),
      (
        "label",
        "&str",
        "the text",
        "What it is called to tooling, where that is more than the word: \"Generated by AI\".",
      ),
      (
        "icon",
        "impl Into<IconChoice>",
        "sparkles",
        "The icon before the word, by its name in the theme's set; None for no icon.",
      ),
      ("size", "impl Into<Name>", "Md", "An AiLabelSize: 18, 22 or 26 high."),
      (
        "style, rules, state",
        "",
        "",
        "Over the theme's recipe: AiLabelPart::Root, Icon, Label.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Surfaces & overlays", "AI Label"],
        title: "AI Label",
        description: "The mark on what a model made or filled in, and on what asks one: sparkles and a word in \
                      the info color.",
        import: Some("use lurq::ui::AiLabel;"),
        status: Some("New"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of AiLabel is a control on the right. It is read and not operated: it takes no press and \
           no focus.",
          playground,
        ),
        Section::new(
          "The label",
          "22 high with the small radius, on the muted fill of the info color in an edge of it, with sparkles 4 \
           before \"AI\" in 11 and semibold.",
          self.frame.demo(0, mark, "AiLabel::new()", &["AiLabel::new()"]),
        ),
        Section::new(
          "Sizes",
          "18 in a row of a table, 22 beside a title or a field's label, 26 beside the heading of a page.",
          self.frame.demo(
            1,
            sizes,
            ".size(AiLabelSize::Sm)",
            &[
              "AiLabel::new().size(AiLabelSize::Sm)",
              "AiLabel::new()",
              "AiLabel::new().size(AiLabelSize::Lg)",
            ],
          ),
        ),
        Section::new(
          "Another word",
          "The word and the icon are the app's to say: a draft, a suggestion, the word in another language.",
          self.frame.demo(
            2,
            words,
            ".text(\"Draft\")",
            &[
              "AiLabel::new().text(\"Draft\")",
              "AiLabel::new().text(\"Suggested\").icon(icons::LIGHTBULB)",
              "AiLabel::new().icon(None)",
            ],
          ),
        ),
        Section::new(
          "Beside what it marks",
          "After the title of what was generated. Its label says to tooling more than the word it draws.",
          self.frame.demo(
            3,
            beside,
            ".label(\"Generated by AI\")",
            &[
              "Row::new()",
              "    .child(title)",
              "    .child(AiLabel::new().size(AiLabelSize::Sm).label(\"Generated by AI\"))",
            ],
          ),
        ),
        Section::new(
          "Explained",
          "What explains the mark stands on it as on any element: a tooltip mounted on it shows while the \
           pointer rests on the label.",
          self.frame.demo(
            4,
            explained,
            "Tooltip::new(..).mount(ctx, AiLabel::new())",
            &[
              "Tooltip::new(\"Written by a model from the thread. Check it before you rely on it.\")",
              "    .mount(ctx, AiLabel::new())",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What an AI label is given.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
