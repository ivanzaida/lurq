//! The page of the sheet: its playground, a sheet from each edge of the
//! window, the ones the board draws as they are and restyled into the
//! board's phone and tablet sheets, and a sheet at the bottom a finger drags.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx, theme::PaletteColor},
  components::{Column, Row, SheetDetent},
  core::Signal,
  layout::{Alignment, text_style::FontWeight},
  node::Element,
  ui::{Button, ButtonVariant, Input, Sheet, SheetPart, SheetSide, SheetSize},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet, segmented_fill, text},
  playground::{Playground, Spec, divider, field, flag, prop},
};

/// The edges, as the code names them.
const SIDES: [(SheetSide, &str, &str); 4] = [
  (SheetSide::End, "end", "SheetSide::End"),
  (SheetSide::Start, "start", "SheetSide::Start"),
  (SheetSide::Top, "top", "SheetSide::Top"),
  (SheetSide::Bottom, "bottom", "SheetSide::Bottom"),
];

/// The sizes, as the code names them.
const SIZES: [(Option<SheetSize>, &str, &str); 5] = [
  (None, "none", ""),
  (Some(SheetSize::Sm), "sm", "SheetSize::Sm"),
  (Some(SheetSize::Md), "md", "SheetSize::Md"),
  (Some(SheetSize::Lg), "lg", "SheetSize::Lg"),
  (Some(SheetSize::Full), "full", "SheetSize::Full"),
];

/// A line of a sheet's body: 13, in the muted color.
fn paragraph(content: &str) -> lurq::components::Text {
  text(content, 13.0, FontWeight::Normal).color(PaletteColor::MutedForeground)
}

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .spacing(9.0)
    .overflow_visible()
    .child(Row::new().overflow_visible().child(shown))
    .child(quiet(caption, 12.0))
}

/// What is on a stage, side by side; what does not fit goes on to the next line.
fn items() -> Row {
  Row::new()
    .width(FILL)
    .spacing(28.0)
    .wrap()
    .overflow_visible()
    .align_items(Alignment::Start)
}

/// A button that opens the sheet `open` holds.
fn opener(label: &str, open: &Signal<bool>) -> Button {
  let open = open.clone();
  Button::new(label)
    .variant(ButtonVariant::Outline)
    .on_click(move |_| open.set(true))
}

/// The board's sheet: a profile to edit.
fn edit_profile(open: &Signal<bool>, name: &Signal<String>) -> Sheet {
  let (close, save) = (open.clone(), open.clone());
  Sheet::new(open.clone())
    .title("Edit profile")
    .description("Changes save when you close the panel.")
    .close_button(true)
    .child(Input::new(name.clone()))
    .footer(
      Button::new("Close")
        .variant(ButtonVariant::Outline)
        .on_click(move |_| close.set(false)),
    )
    .footer(Button::new("Save").on_click(move |_| save.set(false)))
}

/// The board's sheet at the bottom: a goal to set.
fn move_goal(open: &Signal<bool>) -> Sheet {
  let (cancel, submit) = (open.clone(), open.clone());
  Sheet::new(open.clone())
    .side(SheetSide::Bottom)
    .title("Move goal")
    .description("Set your daily activity goal.")
    .footer(
      Button::new("Cancel")
        .variant(ButtonVariant::Outline)
        .on_click(move |_| cancel.set(false)),
    )
    .footer(Button::new("Submit").on_click(move |_| submit.set(false)))
}

pub struct SheetPage {
  frame: Frame,
  playground: Playground,
  open: Signal<bool>,
  side: Signal<usize>,
  size: Signal<usize>,
  title: Signal<String>,
  description: Signal<String>,
  close_button: Signal<bool>,
  dismiss_on_veil: Signal<bool>,
  dismiss_on_escape: Signal<bool>,
  sides: [Signal<bool>; 4],
  dragged: Signal<bool>,
  name: Signal<String>,
  search: Signal<String>,
  event: Signal<String>,
}

impl SheetPage {
  /// The playground: a button on the stage that opens the sheet the panel's props make.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (title, description) = (self.title.get(), self.description.get());
    let (side, side_name, side_code) = SIDES[self.side.get().min(SIDES.len() - 1)];
    let (size, size_name, size_code) = SIZES[self.size.get().min(SIZES.len() - 1)];
    let close_button = self.close_button.get();
    let (on_veil, on_escape) = (self.dismiss_on_veil.get(), self.dismiss_on_escape.get());
    let make = {
      let (title, description) = (title.clone(), description.clone());
      move |open: Signal<bool>| {
        let mut sheet = Sheet::new(open)
          .side(side)
          .close_button(close_button)
          .dismiss_on_veil(on_veil)
          .dismiss_on_escape(on_escape)
          .child(paragraph(
            "What the app puts in the sheet scrolls between the header and the footer.",
          ));
        if !title.is_empty() {
          sheet = sheet.title(&title);
        }
        if !description.is_empty() {
          sheet = sheet.description(&description);
        }
        if let Some(size) = size {
          sheet = sheet.size(size);
        }
        sheet
      }
    };
    let record = self.playground.recorder();
    let footer = |label: &'static str, variant: ButtonVariant| {
      let (open, record) = (self.open.clone(), record.clone());
      Button::new(label).variant(variant).on_click(move |_| {
        open.set(false);
        record(format!("on_click({label:?})"));
      })
    };
    let sheet = make(self.open.clone())
      .footer(footer("Cancel", ButtonVariant::Outline))
      .footer(footer("Save", ButtonVariant::Primary));
    let shown = Row::new().child(opener("Open sheet", &self.open)).child(sheet);

    let mut code = vec![
      "// open: Signal<bool>".to_owned(),
      "Sheet::new(open.clone())".to_owned(),
    ];
    if side != SheetSide::End {
      code.push(format!("    .side({side_code})"));
    }
    if !title.is_empty() {
      code.push(format!("    .title({title:?})"));
    }
    if !description.is_empty() {
      code.push(format!("    .description({description:?})"));
    }
    if size.is_some() {
      code.push(format!("    .size({size_code})"));
    }
    for (name, set, default) in [
      ("close_button", close_button, false),
      ("dismiss_on_veil", on_veil, true),
      ("dismiss_on_escape", on_escape, true),
    ] {
      if set != default {
        code.push(format!("    .{name}({set})"));
      }
    }
    code.push("    .child(paragraph)".to_owned());
    code.push("    .footer(Button::new(\"Cancel\").variant(ButtonVariant::Outline))".to_owned());
    code.push("    .footer(Button::new(\"Save\"))".to_owned());

    let across = if side.is_side() {
      "320 px wide, the window high"
    } else {
      "the window wide"
    };
    let mut tree = vec![
      "veil        the window   scrim, fades".to_owned(),
      format!("└─ panel    {across}   background · shadow lg · slides from the {side_name}"),
    ];
    if side == SheetSide::Bottom {
      tree.push("   ├─ grabber  handle 40 × 4".to_owned());
    }
    tree.push(format!(
      "   ├─ header   {}",
      if close_button { "title · close 24 px" } else { "title" }
    ));
    tree.push("   ├─ body     scrolls".to_owned());
    tree.push(if side == SheetSide::Bottom {
      "   └─ footer   Save over Cancel, as wide as the sheet".to_owned()
    } else {
      "   └─ footer   Cancel · Save, under a line".to_owned()
    });

    let to = (
      self.title.clone(),
      self.description.clone(),
      self.side.clone(),
      self.size.clone(),
    );
    let flags = [self.close_button.clone()];
    let dismissals = [self.dismiss_on_veil.clone(), self.dismiss_on_escape.clone()];
    let (to_side, to_size) = (self.side.clone(), self.size.clone());
    let sides: Vec<(&str, usize)> = SIDES.iter().enumerate().map(|(index, side)| (side.1, index)).collect();
    let sizes: Vec<(&str, usize)> = SIZES.iter().enumerate().map(|(index, size)| (size.1, index)).collect();
    self.playground.card(
      ctx,
      Spec {
        title: "Sheet",
        make: Arc::new(move || {
          let sheet = make(Signal::new(true))
            .inline(true)
            .footer(Button::new("Cancel").variant(ButtonVariant::Outline))
            .footer(Button::new("Save"));
          // Drawn where it is declared, a sheet at a side is as high as the board draws it.
          if side.is_side() {
            sheet.style(SheetPart::Panel, |s| s.height(380.0)).into()
          } else {
            sheet.into()
          }
        }),
        shown: shown.into(),
        fill: false,
        forceable: false,
        facts: format!(
          "{side_name} · {size_name} · {}",
          if self.open.get() { "open" } else { "closed" }
        ),
        event: "on_click",
        code,
        tree,
        controls: vec![
          prop(
            "side",
            "SheetSide",
            segmented_fill(&sides, &self.side.get(), move |index| to_side.set(index)),
          ),
          prop(
            "size",
            "SheetSize",
            segmented_fill(&sizes, &self.size.get(), move |index| to_size.set(index)),
          ),
          prop("title", "&str", field(&self.title)),
          prop("description", "&str", field(&self.description)),
          divider(),
          flag("close_button", &self.close_button),
          flag("dismiss_on_veil", &self.dismiss_on_veil),
          flag("dismiss_on_escape", &self.dismiss_on_escape),
        ],
        changed: [
          title != "Edit profile",
          description != "Changes save when you close the panel.",
          side != SheetSide::End,
          size.is_some(),
          !close_button,
          !on_veil,
          !on_escape,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          to.0.set("Edit profile".to_owned());
          to.1.set("Changes save when you close the panel.".to_owned());
          to.2.set(0);
          to.3.set(0);
          for flag in &flags {
            flag.set(true);
          }
          for flag in &dismissals {
            flag.set(true);
          }
        }),
      },
    )
  }
}

impl Component for SheetPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      open: ctx.signal(false),
      side: ctx.signal(0),
      size: ctx.signal(0),
      title: ctx.signal("Edit profile".to_owned()),
      description: ctx.signal("Changes save when you close the panel.".to_owned()),
      close_button: ctx.signal(true),
      dismiss_on_veil: ctx.signal(true),
      dismiss_on_escape: ctx.signal(true),
      sides: [
        ctx.signal(false),
        ctx.signal(false),
        ctx.signal(false),
        ctx.signal(false),
      ],
      dragged: ctx.signal(false),
      name: ctx.signal("Ivan Zaida".to_owned()),
      search: ctx.signal(String::new()),
      event: ctx.signal("Design review".to_owned()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    // A sheet from each edge, each opened by its own button.
    let mut edges = Row::new().spacing(12.0).wrap();
    for ((side, name, _), open) in SIDES.iter().zip(&self.sides) {
      let close = open.clone();
      edges = edges.child(opener(&format!("From the {name}"), open)).child(
        Sheet::new(open.clone())
          .side(*side)
          .title("Filters")
          .description("Narrow the list to what you are looking for.")
          .close_button(true)
          .child(Input::new(self.search.clone()).placeholder("Search"))
          .footer(Button::new("Apply").on_click(move |_| close.set(false))),
      );
    }

    let drawn = items()
      .child(item(
        edit_profile(&Signal::new(false), &self.name)
          .inline(true)
          .style(SheetPart::Panel, |s| s.height(380.0)),
        "sheet",
      ))
      .child(item(
        move_goal(&Signal::new(false))
          .inline(true)
          .style(SheetPart::Panel, |s| s.width(340.0)),
        "sheet · bottom",
      ));

    // The board's phone and tablet sheets are the same parts restyled.
    let restyled = items()
      .child(item(
        Sheet::new(Signal::new(false))
          .side(SheetSide::Bottom)
          .inline(true)
          .title("Share")
          .description("Send a link to this document.")
          .footer(Button::new("Done"))
          .style(SheetPart::Panel, |s| {
            s.width(360.0).corners(20.0, 20.0, 0.0, 0.0).padding_bottom(16.0)
          })
          .style(SheetPart::Handle, |s| s.width(36.0).background(PaletteColor::Input))
          .style(SheetPart::Header, |s| s.padding_x(16.0).padding_top(16.0))
          .style(SheetPart::Title, |s| s.font_size(18.0))
          .style(SheetPart::Description, |s| s.font_size(14.0))
          .style(SheetPart::Footer, |s| s.padding_x(16.0).padding_top(16.0)),
        "mobile · bottom sheet",
      ))
      .child(item(
        Sheet::new(Signal::new(false))
          .side(SheetSide::Top)
          .inline(true)
          .title("New event")
          .close_button(true)
          .child(Input::new(self.event.clone()))
          .style(SheetPart::Panel, |s| {
            s.width(520.0)
              .height(360.0)
              .radius(16.0)
              .border(lurq::node::border::Border::inside(1.0, PaletteColor::Border))
          }),
        "tablet · form sheet",
      ));

    let dragged = Row::new().child(opener("Move goal…", &self.dragged)).child(
      move_goal(&self.dragged)
        .detents([SheetDetent::Content, SheetDetent::Large])
        .child(paragraph(
          "Drag the grabber up for the whole window, or down to put the sheet away.",
        ))
        .mount(ctx),
    );

    let rows = [
      (
        "open",
        "Signal<bool>",
        "",
        "Whether it is shown. The app's: Escape, a press on the veil and the close button set it to false.",
      ),
      (
        "side",
        "SheetSide",
        "End",
        "The edge it comes from: Start and End as high as the window, Top and Bottom as wide as it.",
      ),
      (
        "size",
        "SheetSize",
        "320 px / content",
        "Sm 320, Md 400, Lg 540 wide at a side; Sm 240, Md 360, Lg 480 high at the top or the bottom; Full the window.",
      ),
      (
        "title",
        "&str",
        "",
        "What the sheet is, 16/600. It is what the sheet is called.",
      ),
      (
        "description",
        "&str",
        "",
        "More of it under the title, 13 in the muted color.",
      ),
      ("label", "&str", "", "What the sheet is called where it has no title."),
      (
        "child",
        "impl Into<Element>",
        "",
        "Adds an element to the body, which scrolls.",
      ),
      (
        "footer",
        "impl Into<Element>",
        "",
        "Adds an element to the footer: a row at the end under a line, or at the bottom buttons as wide as the sheet.",
      ),
      (
        "close_button",
        "bool",
        "false",
        "A ghost button with the close icon at the end of the header.",
      ),
      (
        "grabber",
        "bool",
        "at the bottom",
        "The handle at the top that says the sheet can be dragged.",
      ),
      (
        "dismiss_on_veil",
        "bool",
        "true",
        "Whether a press on the veil closes it.",
      ),
      ("dismiss_on_escape", "bool", "true", "Whether Escape closes it."),
      (
        "detents",
        "[SheetDetent]",
        "its size",
        "The heights a mounted sheet at the bottom stands at; a finger moves it between them.",
      ),
      (
        "inline",
        "bool",
        "false",
        "The panel where it is declared, without the veil: for a gallery, a screenshot, a test.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Surfaces", "Sheet"],
        title: "Sheet",
        description: "A panel that slides in from an edge of the window, over the page, for what goes with the page \
                      without leaving it: edit a profile, filter a list, share a document.",
        import: Some("use lurq::ui::Sheet;"),
        status: Some("Stable"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Sheet is a control on the right. The button on the stage opens the sheet they make; its \
           buttons, Escape and a press on the veil close it.",
          playground,
        ),
        Section::new(
          "From each edge",
          "It slides in from its edge as the veil fades in, and out again; with motion reduced it fades where it \
           stands. The focus goes into it as it opens and back to its button as it closes, and Tab stays inside.",
          self.frame.demo(
            0,
            edges,
            "Sheet::new(open.clone()).side(SheetSide::Start)",
            &[
              "// open: Signal<bool>",
              "Sheet::new(open.clone())",
              "    .side(SheetSide::Start)",
              "    .title(\"Filters\")",
              "    .description(\"Narrow the list to what you are looking for.\")",
              "    .close_button(true)",
              "    .child(Input::new(search.clone()).placeholder(\"Search\"))",
              "    .footer(Button::new(\"Apply\"))",
            ],
          ),
        ),
        Section::new(
          "As the board draws them",
          "The panel alone, without the veil, where it is declared: the sheet at a side, and the one at the \
           bottom with its grabber and its buttons as wide as it.",
          self.frame.demo(
            1,
            drawn,
            ".inline(true)",
            &[
              "Sheet::new(open.clone())",
              "    .title(\"Edit profile\")",
              "    .description(\"Changes save when you close the panel.\")",
              "    .close_button(true)",
              "    .child(Input::new(name.clone()))",
              "    .footer(Button::new(\"Close\").variant(ButtonVariant::Outline))",
              "    .footer(Button::new(\"Save\"))",
              "    .style(SheetPart::Panel, |s| s.height(380.0))",
              "    .inline(true)",
              "",
              "Sheet::new(open.clone())",
              "    .side(SheetSide::Bottom)",
              "    .title(\"Move goal\")",
              "    .description(\"Set your daily activity goal.\")",
              "    .footer(Button::new(\"Cancel\").variant(ButtonVariant::Outline))",
              "    .footer(Button::new(\"Submit\"))",
              "    .style(SheetPart::Panel, |s| s.width(340.0))",
              "    .inline(true)",
            ],
          ),
        ),
        Section::new(
          "Restyled",
          "The phone's bottom sheet and the tablet's form sheet of the board are the same parts with other numbers.",
          self.frame.demo(
            2,
            restyled,
            ".style(SheetPart::Panel, ..)",
            &[
              "Sheet::new(open.clone())",
              "    .side(SheetSide::Bottom)",
              "    .title(\"Share\")",
              "    .description(\"Send a link to this document.\")",
              "    .footer(Button::new(\"Done\"))",
              "    .style(SheetPart::Panel, |s| s.width(360.0).corners(20.0, 20.0, 0.0, 0.0).padding_bottom(16.0))",
              "    .style(SheetPart::Handle, |s| s.width(36.0).background(PaletteColor::Input))",
              "    .style(SheetPart::Header, |s| s.padding_x(16.0).padding_top(16.0))",
              "    .style(SheetPart::Title, |s| s.font_size(18.0))",
              "    .style(SheetPart::Description, |s| s.font_size(14.0))",
              "    .style(SheetPart::Footer, |s| s.padding_x(16.0).padding_top(16.0))",
              "",
              "Sheet::new(open.clone())",
              "    .side(SheetSide::Top)",
              "    .title(\"New event\")",
              "    .close_button(true)",
              "    .child(Input::new(event.clone()))",
              "    .style(SheetPart::Panel, |s| s.width(520.0).height(360.0).radius(16.0).border(edge))",
            ],
          ),
        ),
        Section::new(
          "A sheet a finger drags",
          "Mounted, a sheet at the bottom is the engine's sheet in this look: a finger drags it between its \
           heights and down to put it away, and it rises over the on-screen keyboard.",
          self.frame.demo(
            3,
            dragged,
            ".detents([..]).mount(ctx)",
            &[
              "// open: Signal<bool>",
              "Sheet::new(open.clone())",
              "    .side(SheetSide::Bottom)",
              "    .title(\"Move goal\")",
              "    .description(\"Set your daily activity goal.\")",
              "    .detents([SheetDetent::Content, SheetDetent::Large])",
              "    .footer(Button::new(\"Cancel\").variant(ButtonVariant::Outline))",
              "    .footer(Button::new(\"Submit\"))",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a sheet is given.",
          props_table(&rows, &["open"], self.frame.room()),
        ),
      ],
    )
  }
}
