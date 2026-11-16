//! The page of the quote: its playground, the board's quote, and a quote of paragraphs in a document.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx, theme::TypographyStyle},
  components::{Column, Text},
  core::Signal,
  node::{Element, dimension::Dimension},
  ui::Blockquote,
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Shown, Spec, field, flag, prop},
};

/// The board's words (`mlxnG`).
const SAID: &str = "A design system is a set of decisions you no longer have to make.";

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str, width: f32) -> Column {
  Column::new()
    .max_width(FILL)
    .width(width)
    .spacing(10.0)
    .child(shown)
    .child(quiet(caption, 12.0))
}

pub struct BlockquotePage {
  frame: Frame,
  playground: Playground,
  text: Signal<String>,
  label: Signal<String>,
  paragraphs: Signal<bool>,
}

impl BlockquotePage {
  /// The playground: the quote as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (text, label, paragraphs) = (self.text.get(), self.label.get(), self.paragraphs.get());
    let make = {
      let (text, label) = (text.clone(), label.clone());
      move || -> Blockquote {
        let mut quote = Blockquote::new(&text);
        if !label.is_empty() {
          quote = quote.label(&label);
        }
        if paragraphs {
          quote = quote
            .child(
              Text::new("Every one of them was made once, by someone who thought it through.")
                .variant(TypographyStyle::Prose),
            )
            .child(Text::new("The rest of us get to build.").variant(TypographyStyle::Prose));
        }
        quote
      }
    };
    let mut code = vec![format!("Blockquote::new({text:?})")];
    if !label.is_empty() {
      code.push(format!("    .label({label:?})"));
    }
    if paragraphs {
      code.push("    .child(Text::new(\"Every one of them ..\").variant(TypographyStyle::Prose))".to_owned());
      code.push("    .child(Text::new(\"The rest of us get to build.\").variant(TypographyStyle::Prose))".to_owned());
    }
    let mut tree = vec![format!(
      "root       {}   line 2 · Border · padded 16 after it, 4 above and below",
      self.playground.measured()
    )];
    if !text.is_empty() {
      tree.push(format!(
        "{}─ label   14 italic · lines 1.5 · MutedForeground",
        if paragraphs { "├" } else { "└" }
      ));
    }
    if paragraphs {
      tree.push("└─ body    8 apart".to_owned());
    }
    let reset = {
      let (text, label, paragraphs) = (self.text.clone(), self.label.clone(), self.paragraphs.clone());
      Arc::new(move || {
        text.set(SAID.to_owned());
        label.set(String::new());
        paragraphs.set(false);
      })
    };
    self.playground.card(
      ctx,
      Spec {
        title: "Blockquote",
        make: Arc::new({
          let make = make.clone();
          move || make().into()
        }),
        shown: Shown::of(make()),
        fill: true,
        forceable: false,
        facts: if paragraphs {
          "text · body".to_owned()
        } else {
          "text".to_owned()
        },
        event: "",
        code,
        tree,
        controls: vec![
          prop("new", "&str", field(&self.text)),
          prop("label", "&str", field(&self.label)),
          flag("child", &self.paragraphs),
        ],
        changed: [text != SAID, !label.is_empty(), paragraphs]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset,
      },
    )
  }
}

impl Component for BlockquotePage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      text: ctx.signal(SAID.to_owned()),
      label: ctx.signal(String::new()),
      paragraphs: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let board = item(Blockquote::new(SAID), "the board's quote", 380.0);
    let document = item(
      Column::new()
        .width(Dimension::Pct(100.0))
        .spacing(12.0)
        .child(
          Text::new("A node can blur itself, or blur whatever is painted behind it.")
            .variant(TypographyStyle::Article)
            .width(Dimension::Pct(100.0)),
        )
        .child(
          Blockquote::new("")
            .label("From the design notes")
            .child(
              Text::new(
                "Blur is skipped when the radius is zero, so a panel that fades its blur in costs nothing at rest.",
              )
              .variant(TypographyStyle::Prose)
              .width(Dimension::Pct(100.0)),
            )
            .child(
              Text::new("On integrated GPUs, keep the radius under 24.")
                .variant(TypographyStyle::Prose)
                .width(Dimension::Pct(100.0)),
            ),
        ),
      "in a document · child · child",
      420.0,
    );
    let rows = [
      (
        "new",
        "&str",
        "",
        "What it says, 14 in italics on lines of 1.5. An empty text says nothing.",
      ),
      (
        "child / children",
        "impl Into<Element>",
        "",
        "Elements under the text, 8 apart: paragraphs, a list, a block of code.",
      ),
      (
        "label",
        "&str",
        "",
        "What it is called to tooling: who said it, where it is from.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Data", "Blockquote"],
        title: "Blockquote",
        description: "A quote set apart from the text around it by a line at its start: words someone said, a \
                      passage of another text, Markdown's >. It is read, and is no stop of Tab.",
        import: Some("use lurq::ui::Blockquote;"),
        status: Some("Beta"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Blockquote is a control on the right.",
          playground,
        ),
        Section::new(
          "Blockquote",
          "The board's quote: a line of 2 at its start, its text in italics in the muted color.",
          self.frame.demo(
            0,
            board,
            "Blockquote::new(..)",
            &[&format!("Blockquote::new({SAID:?})")],
          ),
        ),
        Section::new(
          "In a document",
          "A quote of paragraphs, as Markdown's > holds them: what it holds takes its color.",
          self.frame.demo(
            1,
            document,
            ".child(..)",
            &[
              "Blockquote::new(\"\")",
              "    .label(\"From the design notes\")",
              "    .child(Text::new(\"Blur is skipped when ..\").variant(TypographyStyle::Prose))",
              "    .child(Text::new(\"On integrated GPUs, ..\").variant(TypographyStyle::Prose))",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a quote is given.",
          props_table(&rows, &["new"], self.frame.room()),
        ),
      ],
    )
  }
}
