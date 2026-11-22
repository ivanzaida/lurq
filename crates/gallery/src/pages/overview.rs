//! What the gallery is, and what of the design system is in it so far.

use lurq::{
  app::{component::Component, ctx::Ctx, theme::TypographyStyle},
  components::{Column, Grid, Row, Text},
  layout::Alignment,
  node::Element,
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, card, code, fit, muted, phone, styled},
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

/// How wide the longest line of the first example is.
const START: f32 = 480.0;

impl Component for Overview {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self { frame: Frame::new(ctx) }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let components = PAGES
      .iter()
      .filter(|page| !matches!(page.group, Group::GettingStarted | Group::Engine))
      .count();
    let engine = PAGES.iter().filter(|page| page.group == Group::Engine).count();
    // Side by side where each has room for its words, one under another in a window as narrow as a phone.
    let here = Grid::with_columns(if phone() { 1 } else { 3 })
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
        lurq::app::theme::ThemePalette::roles().len(),
        "In a light and a dark set. Switch the theme in the header: every page follows.",
      ));

    let mut lines = Column::new().spacing(2.0);
    for line in [
      "use lurq::ui::{Button, ButtonVariant};",
      "",
      "Row::new()",
      "  .spacing(8.0)",
      "  .child(Button::new(\"Cancel\").variant(ButtonVariant::Outline))",
      "  .child(Button::new(\"Save\").on_click(|_| save()))",
    ] {
      lines = lines.child(code(line).nowrap().height(18.0));
    }
    // A line of code is not broken: in a card narrower than the longest the code scrolls sideways.
    let start = card()
      .padding(20.0)
      .child(fit("start", (self.frame.room() - 42.0).max(0.0), START, lines));

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
        Section::new(
          "Under the components",
          "What the design system is built of is the engine's, and an app can use it as it is.",
          card().padding(20.0).child(
            muted(
              &format!(
                "The Engine group at the end of the side bar has {engine} pages, one for each of the engine's \
                 primitives: rows, columns and stacks, grids, scrolling, paint, transforms, text, images, canvas, \
                 animation, the reactive model, the router. They are built as these pages are, with a playground \
                 first."
              ),
              TypographyStyle::Body,
            )
            .width(FILL),
          ),
        ),
      ],
    )
  }
}
