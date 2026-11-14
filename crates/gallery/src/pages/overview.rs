//! What the gallery is, and what of the design system is in it so far.

use lurq::{
  app::{component::Component, ctx::Ctx, theme::TypographyStyle},
  components::{Column, Grid, Row, Text},
  layout::Alignment,
  node::Element,
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, card, code, muted, styled},
  pages::{Group, PAGES},
};

pub struct Overview {
  frame: Frame,
}

/// A card of the "what is here" grid: a name, a number and a line about it.
fn fact(name: &str, number: usize, about: &str) -> Column {
  card()
    .padding(20.0)
    .spacing(6.0)
    .child(muted(name, TypographyStyle::Label))
    .child(styled(&number.to_string(), TypographyStyle::Heading, |_| {}))
    .child(muted(about, TypographyStyle::Description).width(FILL))
}

impl Component for Overview {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self { frame: Frame::new(ctx) }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let components = PAGES.iter().filter(|page| page.group != Group::GettingStarted).count();
    let here = Grid::with_columns(3)
      .gap(16.0)
      .width(FILL)
      .child(fact(
        "Pages",
        PAGES.len(),
        "A page is here when what it shows is built.",
      ))
      .child(fact(
        "Components",
        components,
        "Each lands with its page: a playground, its variants, sizes and states.",
      ))
      .child(fact(
        "Palette roles",
        42,
        "In a light and a dark set. Switch the theme in the header: every page follows.",
      ));

    let mut start = card().padding(20.0).spacing(2.0);
    for line in [
      "use lurq::ui::{Button, ButtonVariant};",
      "",
      "Row::new()",
      "  .spacing(8.0)",
      "  .child(Button::new(\"Cancel\").variant(ButtonVariant::Outline))",
      "  .child(Button::new(\"Save\").on_click(|_| save()))",
    ] {
      start = start.child(code(line).height(18.0));
    }

    let mut layers = Column::new().width(FILL).spacing(12.0);
    for (name, what) in [
      (
        "Tokens",
        "The palette and the scales: what blue is, how round \"md\" is. One line changes every component.",
      ),
      (
        "Recipes",
        "For each component, the style of each of its parts for each variant, size and state. A theme lays rules \
         over one, replaces one whole, or starts with none.",
      ),
      (
        "Renderers",
        "What builds a component's nodes. A theme gives a component, or one part of it, another: the same behavior \
         in a different anatomy.",
      ),
    ] {
      layers = layers.child(
        Row::new()
          .width(FILL)
          .spacing(16.0)
          .align_items(Alignment::Start)
          .child(Text::new(name).variant(TypographyStyle::Label).width(96.0))
          .child(muted(what, TypographyStyle::Body).flex(1.0)),
      );
    }

    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Getting started", "Overview"],
        title: "lurq Design System",
        description: "The components lurq ships with, drawn by the theme. This gallery is built with them and \
                      shows each one as it is in the code: what is on a page is what the crate does today.",
        import: Some("use lurq::ui::*;"),
        status: None,
        since: None,
        source: file!(),
      },
      vec![
        Section::new(
          "What is here",
          "The design system is being built a component at a time, and the gallery grows with it.",
          here,
        ),
        Section::new(
          "A component in an app",
          "A component is a builder that goes where an element goes. It takes the theme of what renders it.",
          start,
        ),
        Section::new(
          "What a theme changes",
          "Three layers, each with the design as its default.",
          card().padding(20.0).child(layers),
        ),
      ],
    )
  }
}
