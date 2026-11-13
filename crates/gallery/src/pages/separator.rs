//! The page of the separator: its playground, a line across a column, one
//! down a toolbar, and one with a word in its middle.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx, theme::PaletteColor},
  components::{Column, Row},
  core::Signal,
  layout::{Alignment, text_style::FontWeight},
  node::Element,
  ui::{Button, ButtonSize, ButtonVariant, Separator, SeparatorOrientation, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, text},
  playground::{Playground, Spec, divider, field, flag, prop},
};

/// The orientations, as the code names them.
const ORIENTATIONS: [(SeparatorOrientation, &str, &str); 2] = [
  (
    SeparatorOrientation::Horizontal,
    "horizontal",
    "Separator::horizontal()",
  ),
  (SeparatorOrientation::Vertical, "vertical", "Separator::vertical()"),
];

/// A block of text that stands for what a separator is between.
fn block(title: &str, about: &str) -> Column {
  Column::new()
    .spacing(4.0)
    .child(text(title, 14.0, FontWeight::Medium))
    .child(text(about, 13.0, FontWeight::Normal).color(PaletteColor::MutedForeground))
}

/// A button of a toolbar: an icon alone.
fn tool(label: &str, icon: &'static str) -> Button {
  Button::new(label)
    .variant(ButtonVariant::Ghost)
    .size(ButtonSize::Sm)
    .icon(icon)
    .icon_only(true)
}

pub struct SeparatorPage {
  frame: Frame,
  playground: Playground,
  orientation: Signal<usize>,
  label: Signal<String>,
  decorative: Signal<bool>,
}

impl SeparatorPage {
  /// The playground: the separator as the panel's props make it, between two blocks.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (orientation, name, call) = ORIENTATIONS[self.orientation.get().min(ORIENTATIONS.len() - 1)];
    let (label, decorative) = (self.label.get(), self.decorative.get());
    let plain = {
      let label = label.clone();
      move || {
        let mut separator = Separator::new(orientation).decorative(decorative);
        if !label.is_empty() {
          separator = separator.label(&label);
        }
        separator
      }
    };
    // Between two blocks: under each other across a column, side by side down a row that stretches it.
    let between = {
      let plain = plain.clone();
      move || -> Element {
        match orientation {
          SeparatorOrientation::Horizontal => Column::new()
            .width(320.0)
            .spacing(16.0)
            .child(block("Profile", "Your name and picture."))
            .child(plain())
            .child(block("Account", "Your email and password."))
            .into(),
          SeparatorOrientation::Vertical => Row::new()
            .height(64.0)
            .spacing(16.0)
            .align_items(Alignment::Stretch)
            .child(block("Profile", "Your name and picture."))
            .child(plain())
            .child(block("Account", "Your email and password."))
            .into(),
        }
      }
    };

    let mut code = vec![call.to_owned()];
    if !label.is_empty() {
      code.push(format!("    .label({label:?})"));
    }
    if !decorative {
      code.push("    .decorative(false)".to_owned());
    }
    let mut tree = vec![format!("root      {}", self.playground.measured())];
    if label.is_empty() {
      tree.push("└─ line    1 px · border".to_owned());
    } else {
      tree.push("├─ line    1 px · border".to_owned());
      tree.push(format!("├─ label   {label:?} · 12 · muted"));
      tree.push("└─ line    1 px · border".to_owned());
    }

    let (to_orientation, to_label, to_decorative) =
      (self.orientation.clone(), self.label.clone(), self.decorative.clone());
    let orientations: Vec<(&str, usize)> = ORIENTATIONS
      .iter()
      .enumerate()
      .map(|(index, orientation)| (orientation.1, index))
      .collect();
    let choose = self.orientation.clone();
    self.playground.card(
      ctx,
      Spec {
        title: "Separator",
        make: Arc::new(between.clone()),
        shown: between(),
        fill: false,
        forceable: false,
        facts: name.to_owned(),
        event: "",
        code,
        tree,
        controls: vec![
          prop(
            "orientation",
            "SeparatorOrientation",
            crate::kit::segmented_fill(&orientations, &self.orientation.get(), move |index| choose.set(index)),
          ),
          prop("label", "&str", field(&self.label)),
          divider(),
          flag("decorative", &self.decorative),
        ],
        changed: [
          orientation != SeparatorOrientation::Horizontal,
          !label.is_empty(),
          !decorative,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          to_orientation.set(0);
          to_label.set(String::new());
          to_decorative.set(true);
        }),
      },
    )
  }
}

impl Component for SeparatorPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      orientation: ctx.signal(0),
      label: ctx.signal(String::new()),
      decorative: ctx.signal(true),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let across = Column::new()
      .width(FILL)
      .spacing(16.0)
      .child(block("lurq", "A Rust UI toolkit with GPU-backed rendering."))
      .child(Separator::horizontal())
      .child(
        Row::new()
          .height(20.0)
          .spacing(12.0)
          .align_items(Alignment::Stretch)
          .child(text("Docs", 13.0, FontWeight::Normal))
          .child(Separator::vertical())
          .child(text("Source", 13.0, FontWeight::Normal))
          .child(Separator::vertical())
          .child(text("Changelog", 13.0, FontWeight::Normal)),
      );
    let toolbar = Row::new()
      .spacing(4.0)
      .align_items(Alignment::Center)
      .child(tool("Undo", icons::UNDO))
      .child(tool("Redo", icons::REDO))
      .child(
        Separator::vertical()
          .length(20.0)
          .style(lurq::ui::SeparatorPart::Root, |s| s.padding_x(4.0)),
      )
      .child(tool("Bold", icons::BOLD))
      .child(tool("Italic", icons::ITALIC));
    let or = Column::new()
      .width(320.0)
      .spacing(16.0)
      .child(
        Button::new("Continue with GitHub")
          .variant(ButtonVariant::Outline)
          .full_width(true),
      )
      .child(Separator::horizontal().label("or"))
      .child(Button::new("Continue with email").full_width(true));
    let rows = [
      (
        "horizontal / vertical",
        "Separator",
        "",
        "A line across a column, as wide as the room it is given; or down a row, as high as a row that stretches \
         its children.",
      ),
      (
        "label",
        "&str",
        "",
        "A word in the middle, 12 in the muted color, with the line on either side 8 from it.",
      ),
      (
        "length",
        "f32",
        "",
        "How long it is: its width across a column, its height down a row that does not stretch it.",
      ),
      (
        "decorative",
        "bool",
        "true",
        "Decoration alone, which tooling passes by. Off, tooling is told it is a separator.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Data", "Separator"],
        title: "Separator",
        description: "A line between things: sections of a page, groups of a toolbar, the two ways to sign in.",
        import: Some("use lurq::ui::Separator;"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Separator is a control on the right. The separator stands between two blocks: under each \
           other across a column, side by side down a row.",
          playground,
        ),
        Section::new(
          "Across and down",
          "A line of 1 in the border's color. Across a column it is as wide as the column; down a row it is as high \
           as the row, where the row stretches its children.",
          self.frame.demo(
            0,
            across,
            "Separator::horizontal()",
            &[
              "Column::new()",
              "    .child(heading)",
              "    .child(Separator::horizontal())",
              "    .child(",
              "        Row::new()",
              "            .height(20.0)",
              "            .align_items(Alignment::Stretch)",
              "            .child(Text::new(\"Docs\"))",
              "            .child(Separator::vertical())",
              "            .child(Text::new(\"Source\")),",
              "    )",
            ],
          ),
        ),
        Section::new(
          "In a toolbar",
          "Down a row that centers what is in it, the separator is given a length; a padding of its root keeps it \
           off the buttons on either side.",
          self.frame.demo(
            1,
            toolbar,
            "Separator::vertical().length(20.0)",
            &[
              "Row::new()",
              "    .align_items(Alignment::Center)",
              "    .child(undo)",
              "    .child(redo)",
              "    .child(Separator::vertical().length(20.0).style(SeparatorPart::Root, |s| s.padding_x(4.0)))",
              "    .child(bold)",
              "    .child(italic)",
            ],
          ),
        ),
        Section::new(
          "With a label",
          "A word in the middle of the line: two ways to do one thing.",
          self.frame.demo(
            2,
            or,
            "Separator::horizontal().label(\"or\")",
            &[
              "Column::new()",
              "    .child(Button::new(\"Continue with GitHub\").variant(ButtonVariant::Outline).full_width(true))",
              "    .child(Separator::horizontal().label(\"or\"))",
              "    .child(Button::new(\"Continue with email\").full_width(true))",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a separator is given.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
