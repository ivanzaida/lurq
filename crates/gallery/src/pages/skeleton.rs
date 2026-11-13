//! The page of the skeleton: its playground, the line, the circle and the
//! block, the paragraph, and the media row and the card of the design built
//! from them.

use std::sync::Arc;

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{PaletteColor, RadiusSize},
  },
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{Length, Skeleton, SkeletonAnimation},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, line, props_table, quiet, segmented_fill},
  playground::{Playground, Spec, prop},
};

const FORMS: [&str; 4] = ["line", "circle", "block", "paragraph"];

const ANIMATIONS: [(&str, SkeletonAnimation); 3] = [
  ("pulse", SkeletonAnimation::Pulse),
  ("shimmer", SkeletonAnimation::Shimmer),
  ("none", SkeletonAnimation::None),
];

/// The names of a list with the index of each: what a control of the panel chooses between.
fn indexed(names: &[&'static str]) -> Vec<(&'static str, usize)> {
  names.iter().enumerate().map(|(index, name)| (*name, index)).collect()
}

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new().spacing(9.0).child(shown).child(quiet(caption, 12.0))
}

/// The design's media row: a circle of 40, and two lines beside it.
fn media_row(animation: SkeletonAnimation) -> Row {
  Row::new()
    .width(300.0)
    .spacing(12.0)
    .align_items(Alignment::Center)
    .child(Skeleton::circle(40.0).animation(animation))
    .child(
      Column::new()
        .flex(1.0)
        .spacing(8.0)
        .child(Skeleton::line().width(220.0).animation(animation))
        .child(Skeleton::line().width(140.0).height(10.0).animation(animation)),
    )
}

/// The design's card: an image, a title, two lines and a button, in a card of 280.
fn card(animation: SkeletonAnimation) -> Column {
  Column::new()
    .width(280.0)
    .padding(16.0)
    .spacing(14.0)
    .rounded(RadiusSize::Xl)
    .background(PaletteColor::Card)
    .border(line())
    .child(Skeleton::block(Length::Fill, 132.0).animation(animation))
    .child(Skeleton::line().width(180.0).height(16.0).animation(animation))
    .child(
      Column::new()
        .width(FILL)
        .spacing(8.0)
        .child(Skeleton::line().animation(animation))
        .child(Skeleton::line().width(210.0).animation(animation)),
    )
    .child(
      Skeleton::block(88.0, 32.0)
        .style("bone", |s| s.radius(RadiusSize::Md))
        .animation(animation),
    )
}

pub struct SkeletonPage {
  frame: Frame,
  playground: Playground,
  /// The index of the form in [`FORMS`].
  form: Signal<usize>,
  lines: Signal<usize>,
  /// The index of the animation in [`ANIMATIONS`].
  animation: Signal<usize>,
}

impl SkeletonPage {
  /// The skeleton the panel's props make, and the Rust that builds it.
  fn skeleton(form: usize, lines: usize, animation: SkeletonAnimation) -> (Skeleton, String) {
    let (skeleton, call) = match FORMS[form] {
      "circle" => (Skeleton::circle(40.0), "Skeleton::circle(40.0)".to_owned()),
      "block" => (
        Skeleton::block(200.0, 112.0),
        "Skeleton::block(200.0, 112.0)".to_owned(),
      ),
      "paragraph" => (Skeleton::paragraph(lines), format!("Skeleton::paragraph({lines})")),
      _ => (Skeleton::line(), "Skeleton::line()".to_owned()),
    };
    let call = match animation {
      SkeletonAnimation::Pulse => call,
      other => format!("{call}.animation(SkeletonAnimation::{other:?})"),
    };
    (skeleton.animation(animation), call)
  }

  /// The playground: the skeleton as the panel's props make it, in a column of 300.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (form, lines) = (self.form.get(), self.lines.get());
    let animation = ANIMATIONS[self.animation.get()].1;
    let shown = move || -> Element {
      Column::new()
        .width(300.0)
        .child(Self::skeleton(form, lines, animation).0)
        .into()
    };
    let (_, call) = Self::skeleton(form, lines, animation);
    let (to_form, to_lines, to_animation) = (self.form.clone(), self.lines.clone(), self.animation.clone());
    let counts: Vec<(&str, usize)> = vec![("2", 2), ("3", 3), ("4", 4), ("5", 5)];
    let controls = vec![
      prop(
        "form",
        "line · circle · block · paragraph",
        segmented_fill(&indexed(&FORMS), &form, move |chosen| to_form.set(chosen)),
      ),
      prop(
        "lines",
        "usize",
        segmented_fill(&counts, &lines, move |chosen| to_lines.set(chosen)),
      ),
      prop(
        "animation",
        "SkeletonAnimation",
        segmented_fill(
          &indexed(&ANIMATIONS.map(|(name, _)| name)),
          &self.animation.get(),
          move |chosen| to_animation.set(chosen),
        ),
      ),
    ];
    let bones = if FORMS[form] == "paragraph" { lines } else { 1 };
    let tree = vec![
      format!("root      {}   nothing to tooling", self.playground.measured()),
      format!(
        "└─ bone × {bones}   muted{}",
        if animation == SkeletonAnimation::Shimmer {
          ", a sheen as wide as it crossing it"
        } else {
          ""
        }
      ),
    ];
    let reset = {
      let (form, lines, animation) = (self.form.clone(), self.lines.clone(), self.animation.clone());
      Arc::new(move || {
        form.set(0);
        lines.set(4);
        animation.set(0);
      })
    };
    self.playground.card(
      ctx,
      Spec {
        title: "Skeleton",
        make: Arc::new(shown),
        shown: shown(),
        fill: false,
        forceable: false,
        facts: format!("{:?}", animation).to_lowercase(),
        event: "nothing",
        code: vec![call],
        tree,
        controls,
        changed: [form != 0, lines != 4, self.animation.get() != 0]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset,
      },
    )
  }
}

impl Component for SkeletonPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      form: ctx.signal(0),
      lines: ctx.signal(4),
      animation: ctx.signal(0),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let shapes = Row::new()
      .width(FILL)
      .spacing(28.0)
      .wrap()
      .align_items(Alignment::End)
      .child(item(Column::new().width(200.0).child(Skeleton::line()), "line"))
      .child(item(Skeleton::circle(40.0), "circle"))
      .child(item(Skeleton::block(200.0, 112.0), "block"))
      .child(item(
        Column::new()
          .width(200.0)
          .child(Skeleton::line().animation(SkeletonAnimation::Shimmer)),
        "shimmer",
      ));
    let paragraph = Column::new().width(300.0).child(Skeleton::paragraph(4));
    let composed = Row::new()
      .width(FILL)
      .spacing(28.0)
      .wrap()
      .align_items(Alignment::Start)
      .child(item(
        Skeleton::status("Loading people", media_row(SkeletonAnimation::Pulse)),
        "media row",
      ))
      .child(item(
        Skeleton::status("Loading the article", card(SkeletonAnimation::Shimmer)),
        "card",
      ));
    let rows = [
      (
        "line()",
        "Skeleton",
        "",
        "A line of text: as wide as the room it has, 12 high, radius-sm.",
      ),
      ("circle(size)", "Skeleton", "", "A face, an icon: round, size across."),
      (
        "block(width, height)",
        "Skeleton",
        "",
        "An image, a chart, a card: radius-lg. Length::Fill is all the room it has.",
      ),
      (
        "paragraph(lines)",
        "Skeleton",
        "",
        "Lines 10 apart, as wide as the room they have; the one before the last a little shorter, the last shorter still.",
      ),
      (
        "width, height",
        "impl Into<Length>",
        "",
        "The app's size, over the shape's. For a paragraph the width of all of it and the height of each line.",
      ),
      (
        "animation",
        "SkeletonAnimation",
        "Pulse",
        "It fades to half and back, a sheen crosses it, or it stands still. Where motion is reduced it stands still.",
      ),
      (
        "Skeleton::status(label, content)",
        "Column",
        "",
        "One status around a group of skeletons, which are nothing to tooling themselves: what is loading, said once.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Data", "Skeleton"],
        title: "Skeleton",
        description: "A placeholder in the shape of what is loading. Three shapes build every case, and their fill \
                      is the muted one.",
        import: Some("use lurq::ui::{Skeleton, SkeletonAnimation};"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "The form, the number of lines of a paragraph, and how it moves while it waits.",
          playground,
        ),
        Section::new(
          "Shapes",
          "A line, a circle and a block, pulsing; and a line a sheen crosses, the design's shimmer. Every skeleton \
           on screen pulses as one.",
          self.frame.demo(
            0,
            shapes,
            "Skeleton::line()",
            &[
              "Skeleton::line()",
              "Skeleton::circle(40.0)",
              "Skeleton::block(200.0, 112.0)",
              "Skeleton::line().animation(SkeletonAnimation::Shimmer)",
            ],
          ),
        ),
        Section::new(
          "Paragraph",
          "Lines of text one under another, the last shorter, as a paragraph ends.",
          self
            .frame
            .demo(1, paragraph, "Skeleton::paragraph(4)", &["Skeleton::paragraph(4)"]),
        ),
        Section::new(
          "Composed",
          "A row of a list and a card, built of the three shapes, each in one status that tooling reads.",
          self.frame.demo(
            2,
            composed,
            "Skeleton::status(\"Loading people\", row)",
            &[
              "Skeleton::status(\"Loading people\",",
              "    Row::new()",
              "        .spacing(12.0)",
              "        .align_items(Alignment::Center)",
              "        .child(Skeleton::circle(40.0))",
              "        .child(",
              "            Column::new()",
              "                .flex(1.0)",
              "                .spacing(8.0)",
              "                .child(Skeleton::line().width(220.0))",
              "                .child(Skeleton::line().width(140.0).height(10.0)),",
              "        ),",
              ")",
              "",
              "Skeleton::status(\"Loading the article\",",
              "    Column::new()",
              "        .width(280.0)",
              "        .padding(16.0)",
              "        .spacing(14.0)",
              "        .child(Skeleton::block(Length::Fill, 132.0))",
              "        .child(Skeleton::line().width(180.0).height(16.0))",
              "        .child(Skeleton::line())",
              "        .child(Skeleton::line().width(210.0))",
              "        .child(Skeleton::block(88.0, 32.0).style(SkeletonPart::Bone, |s| s.radius(RadiusSize::Md))),",
              ")",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a skeleton is made with and given.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
