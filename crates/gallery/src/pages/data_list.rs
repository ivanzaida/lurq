//! The page of the data list: its playground, the board's details of a build, one over the other, and values that
//! are elements.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  core::Signal,
  node::Element,
  ui::{Badge, DataList, DataListPart, Link, Tag},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::props_table,
  playground::{Playground, Spec, divider, flag},
};

/// The board's data list: what a build is.
fn board() -> DataList {
  DataList::new()
    .label("Build")
    .text("Version", "0.60.0")
    .text("Renderer", "wgpu")
    .text("License", "MIT")
    .item("Status", Badge::new("Stable").variant("success"))
    .style(DataListPart::Root, |s| s.width(300.0))
}

pub struct DataListPage {
  frame: Frame,
  playground: Playground,
  vertical: Signal<bool>,
  badge: Signal<bool>,
  long: Signal<bool>,
}

impl DataListPage {
  /// The playground: the details of a build as the panel's props make them.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (vertical, badge, long) = (self.vertical.get(), self.badge.get(), self.long.get());
    let make = move || -> Element {
      let mut list = DataList::new()
        .vertical(vertical)
        .text("Version", "0.60.0")
        .text("Renderer", "wgpu")
        .text("License", "MIT");
      if long {
        list = list.text(
          "Description",
          "A GPU UI toolkit for Rust: a tree of elements laid out with flex and grid, drawn with wgpu.",
        );
      }
      list = if badge {
        list.item("Status", Badge::new("Stable").variant("success"))
      } else {
        list.text("Status", "Stable")
      };
      list.style(DataListPart::Root, |s| s.width(320.0)).into()
    };

    let mut code = vec!["DataList::new()".to_owned()];
    if vertical {
      code.push("    .vertical(true)".to_owned());
    }
    code.push("    .text(\"Version\", \"0.60.0\")".to_owned());
    code.push("    .text(\"Renderer\", \"wgpu\")".to_owned());
    code.push("    .text(\"License\", \"MIT\")".to_owned());
    if long {
      code.push("    .text(\"Description\", \"A GPU UI toolkit for Rust: ..\")".to_owned());
    }
    code.push(if badge {
      "    .item(\"Status\", Badge::new(\"Stable\").variant(\"success\"))".to_owned()
    } else {
      "    .text(\"Status\", \"Stable\")".to_owned()
    });

    let tree = if vertical {
      vec![
        format!("root        {}", self.playground.measured()),
        "└─ item     10 over and under, a line of 1 under all but the last".to_owned(),
        "   ├─ term     12, muted".to_owned(),
        "   └─ value    13, 2 under its term".to_owned(),
      ]
    } else {
      vec![
        format!("root        {}   a grid of two columns", self.playground.measured()),
        "├─ term     13, muted; its column as wide as the widest term, 12 after it".to_owned(),
        "└─ value    13; the rest of the width".to_owned(),
        "   10 over and under each, a line of 1 under all but the last".to_owned(),
      ]
    };
    let resets = (self.vertical.clone(), self.badge.clone(), self.long.clone());
    self.playground.card(
      ctx,
      Spec {
        title: "Data List",
        make: Arc::new(make.clone()),
        shown: make(),
        fill: false,
        forceable: false,
        facts: format!("{} items", if long { 5 } else { 4 }),
        event: "",
        code,
        tree,
        controls: vec![
          flag("vertical", &self.vertical),
          divider(),
          flag("a badge as a value", &self.badge),
          flag("a long value", &self.long),
        ],
        changed: [vertical, !badge, long].into_iter().filter(|changed| *changed).count(),
        reset: Arc::new(move || {
          resets.0.set(false);
          resets.1.set(true);
          resets.2.set(false);
        }),
      },
    )
  }
}

impl Component for DataListPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      vertical: ctx.signal(false),
      badge: ctx.signal(true),
      long: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let narrow = DataList::new()
      .vertical(true)
      .label("Order")
      .text("Order", "#4021")
      .text("Placed", "14 November 2026")
      .text("Ship to", "Ivan Zaida, 12 Khreshchatyk St, Kyiv")
      .style(DataListPart::Root, |s| s.width(220.0));
    let elements = DataList::new()
      .label("Package")
      .item("Repository", Link::new("github.com/lurq/lurq"))
      .item(
        "Keywords",
        lurq::components::Row::new()
          .spacing(6.0)
          .child(Tag::new("gui"))
          .child(Tag::new("wgpu"))
          .child(Tag::new("layout")),
      )
      .item("Status", Badge::new("Preview").variant("warning"))
      .style(DataListPart::Root, |s| s.width(360.0));

    let rows = [
      (
        "text",
        "(&str, &str)",
        "",
        "An item whose value is a text, in the value's type.",
      ),
      (
        "item",
        "(&str, impl Into<Element>)",
        "",
        "An item whose value is what the app puts there: a badge, a link, tags.",
      ),
      (
        "vertical",
        "bool",
        "false",
        "Each term over its value, for a narrow column. Side by side, the terms' column is as wide as the widest.",
      ),
      ("label", "&str", "", "What the list is called, to tooling."),
      ("style / rules", "DataListPart", "", "Root, Item, Term, Value."),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Data", "Data List"],
        title: "Data List",
        description: "Terms with their values: what a thing is, item by item. The details of a build, of an \
                      order, of a file.",
        import: Some("use lurq::ui::DataList;"),
        status: Some("Preview"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of DataList is a control on the right. A data list is read, not operated: nothing on the \
           stage is counted.",
          playground,
        ),
        Section::new(
          "Side by side",
          "The board's data list. The terms are one column, as wide as the widest of them, and the values start \
           12 after it; a line runs under each item but the last.",
          self.frame.demo(
            0,
            board(),
            "DataList::new().text(term, value)",
            &[
              "DataList::new()",
              "    .label(\"Build\")",
              "    .text(\"Version\", \"0.60.0\")",
              "    .text(\"Renderer\", \"wgpu\")",
              "    .text(\"License\", \"MIT\")",
              "    .item(\"Status\", Badge::new(\"Stable\").variant(\"success\"))",
            ],
          ),
        ),
        Section::new(
          "One over the other",
          "For a narrow column: each term over its value, a long value going on to the next line.",
          self.frame.demo(
            1,
            narrow,
            "DataList::new().vertical(true)",
            &[
              "DataList::new()",
              "    .vertical(true)",
              "    .text(\"Order\", \"#4021\")",
              "    .text(\"Placed\", \"14 November 2026\")",
              "    .text(\"Ship to\", \"Ivan Zaida, 12 Khreshchatyk St, Kyiv\")",
            ],
          ),
        ),
        Section::new(
          "Values that are elements",
          "A value can be anything the design system has: a link, a row of tags, a badge.",
          self.frame.demo(
            2,
            elements,
            "DataList::new().item(term, element)",
            &[
              "DataList::new()",
              "    .item(\"Repository\", Link::new(\"github.com/lurq/lurq\"))",
              "    .item(\"Keywords\", Row::new().spacing(6.0).child(Tag::new(\"gui\"))..)",
              "    .item(\"Status\", Badge::new(\"Preview\").variant(\"warning\"))",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a data list is given.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
