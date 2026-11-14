//! The page of the action bar: its playground over a list whose rows are
//! selected, the bar at the top, and a phone's selection bar.

use std::sync::Arc;

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{BorderSize, PaletteColor, RadiusSize},
  },
  components::{Column, Row, Stack},
  core::Signal,
  layout::{Alignment, text_style::FontWeight},
  node::{Element, border::Border},
  ui::{ActionBar, ActionBarPart, ActionBarPlacement, Button, Checkbox, Length, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, text},
  playground::{Playground, Spec, divider, field, flag, prop},
};

/// The rows of the list the bar acts on.
const FILES: [&str; 5] = [
  "Quarterly report.pdf",
  "Roadmap.md",
  "Logo.svg",
  "Invoices.xlsx",
  "Notes.txt",
];

/// Where the playground puts the bar, as the code names it.
const PLACEMENTS: [(&str, ActionBarPlacement); 2] =
  [("Bottom", ActionBarPlacement::Bottom), ("Top", ActionBarPlacement::Top)];

/// The list of files, each with a check box bound to whether it is selected, `height` high.
fn list(selected: &Signal<Vec<usize>>, height: f32) -> Element {
  let mut rows = Column::new()
    .width(FILL)
    .height(height)
    .padding(8.0)
    .rounded(RadiusSize::Lg)
    .background(PaletteColor::Background)
    .border(Border::inside(BorderSize::Sm, PaletteColor::Border));
  let chosen = selected.get();
  for (index, name) in FILES.iter().enumerate() {
    let set = selected.clone();
    rows = rows.child(
      Row::new()
        .height(36.0)
        .padding_horizontal(8.0)
        .spacing(10.0)
        .align_items(Alignment::Center)
        .child(
          Checkbox::new(name)
            .checked(chosen.contains(&index))
            .on_change(move |on| {
              set.update(|chosen| {
                chosen.retain(|held| *held != index);
                if on {
                  chosen.push(index);
                }
              });
            }),
        ),
    );
  }
  rows.into()
}

/// The bar of the selection in `selected`: Move, Duplicate and Delete, and the clear button.
fn bar(selected: &Signal<Vec<usize>>, record: impl Fn(String) + Clone + Send + Sync + 'static) -> ActionBar {
  let count = selected.get().len();
  let action = |label: &'static str, icon: &str| {
    let record = record.clone();
    Button::new(label)
      .icon(icon)
      .on_click(move |_| record(format!("{label}: {count} files")))
  };
  let clear = selected.clone();
  let told = record.clone();
  ActionBar::new(count)
    .action(action("Move", icons::FOLDER_IMPORT))
    .action(action("Duplicate", icons::COPY))
    .destructive_action(action("Delete", icons::TRASH))
    .on_clear(move || {
      clear.set(Vec::new());
      told("on_clear".to_owned());
    })
}

pub struct ActionBarPage {
  frame: Frame,
  playground: Playground,
  selected: Signal<Vec<usize>>,
  placement: Signal<usize>,
  text: Signal<String>,
  escape: Signal<bool>,
  /// The selection of the examples under the playground.
  top: Signal<Vec<usize>>,
  phone: Signal<Vec<usize>>,
}

impl ActionBarPage {
  /// The playground: a list whose rows a person selects, and over it the bar as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (placement_name, placement) = PLACEMENTS[self.placement.get().min(PLACEMENTS.len() - 1)];
    let (text, escape) = (self.text.get(), self.escape.get());
    let count = self.selected.get().len();
    let plain = {
      let (selected, text, record) = (self.selected.clone(), text.clone(), self.playground.recorder());
      move || {
        let mut made = bar(&selected, record.clone())
          .placement(placement)
          .escape_clears(escape);
        if !text.is_empty() {
          made = made.text(&text);
        }
        made
      }
    };
    let over = {
      let (selected, plain) = (self.selected.clone(), plain.clone());
      move || -> Element {
        Stack::new()
          .width(480.0)
          .height(260.0)
          .overflow_visible()
          .child(list(&selected, 260.0))
          .child(plain())
          .into()
      }
    };

    let mut code = vec![format!("ActionBar::new({count})")];
    if !text.is_empty() {
      code.push(format!("    .text({text:?})"));
    }
    code.push("    .action(Button::new(\"Move\").icon(icons::FOLDER_IMPORT))".to_owned());
    code.push("    .action(Button::new(\"Duplicate\").icon(icons::COPY))".to_owned());
    code.push("    .destructive_action(Button::new(\"Delete\").icon(icons::TRASH))".to_owned());
    code.push("    .on_clear(move || selected.set(Vec::new()))".to_owned());
    if placement != ActionBarPlacement::Bottom {
      code.push(format!("    .placement(ActionBarPlacement::{placement_name})"));
    }
    if escape {
      code.push("    .escape_clears(true)".to_owned());
    }
    let tree = vec![
      "root          the room over the list · padding 16 · takes no press".to_owned(),
      "└─ bar        44 high · popover · round · edge · shadow lg".to_owned(),
      "   ├─ count   13/600 · padding 16 before it".to_owned(),
      "   ├─ separator  1 × 20 · 6 either side".to_owned(),
      "   ├─ action  ghost · round · 32 high · icon 14 · label 13".to_owned(),
      "   ├─ separator".to_owned(),
      "   └─ clear   round 32 · close 15".to_owned(),
    ];

    let (to_placement, to_text, to_escape, to_selected) = (
      self.placement.clone(),
      self.text.clone(),
      self.escape.clone(),
      self.selected.clone(),
    );
    let set_placement = self.placement.clone();
    let placements: Vec<(&str, usize)> = PLACEMENTS
      .iter()
      .enumerate()
      .map(|(index, (name, _))| (*name, index))
      .collect();
    self.playground.card(
      ctx,
      Spec {
        title: "Action Bar",
        make: Arc::new(over.clone()),
        shown: over(),
        fill: false,
        forceable: false,
        facts: format!("{count} selected"),
        event: "on_click / on_clear",
        code,
        tree,
        controls: vec![
          prop("text", "&str", field(&self.text)),
          prop(
            "placement",
            "ActionBarPlacement",
            crate::kit::segmented_fill(&placements, &self.placement.get(), move |index| {
              set_placement.set(index)
            }),
          ),
          divider(),
          flag("escape_clears", &self.escape),
        ],
        changed: [placement != ActionBarPlacement::Bottom, !text.is_empty(), escape]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset: Arc::new(move || {
          to_placement.set(0);
          to_text.set(String::new());
          to_escape.set(false);
          to_selected.set(vec![0, 1, 2]);
        }),
      },
    )
  }
}

impl Component for ActionBarPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      selected: ctx.signal(vec![0, 1, 2]),
      placement: ctx.signal(0),
      text: ctx.signal(String::new()),
      escape: ctx.signal(false),
      top: ctx.signal(vec![1, 3]),
      phone: ctx.signal(vec![0, 2, 4]),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let quiet = |_: String| {};
    let top = Stack::new()
      .width(FILL)
      .height(240.0)
      .overflow_visible()
      .child(list(&self.top, 240.0))
      .child(
        bar(&self.top, quiet)
          .placement(ActionBarPlacement::Top)
          .escape_clears(true),
      );
    // The board's Mobile/Selection Bar: the same parts, restyled into a bar across the top of a phone.
    let phone_bar = {
      let count = self.phone.get().len();
      let clear = self.phone.clone();
      ActionBar::new(count)
        .placement(ActionBarPlacement::Top)
        .action(Button::new("Archive").icon(icons::ARCHIVE).icon_only(true))
        .destructive_action(Button::new("Delete").icon(icons::TRASH).icon_only(true))
        .on_clear(move || clear.set(Vec::new()))
        .style(ActionBarPart::Root, |s| s.padding_x(0.0).padding_y(0.0))
        .style(ActionBarPart::Bar, |s| {
          s.width(Length::Fill)
            .height(56.0)
            .padding_start(4.0)
            .padding_end(4.0)
            .radius(0.0)
            .background(PaletteColor::Highlight)
            .shadow(lurq::app::theme::ShadowStyle::Sm)
        })
        .style(ActionBarPart::Count, |s| s.font_size(17.0))
        .style(ActionBarPart::Separator, |s| s.width(0.0).padding_x(0.0))
        .style(ActionBarPart::Action, |s| s.square(44.0).padding_x(0.0))
        .style(ActionBarPart::ActionIcon, |s| s.square(22.0))
        .style(ActionBarPart::Clear, |s| s.square(44.0))
        .style(ActionBarPart::ClearIcon, |s| s.square(22.0))
    };
    let phone = Stack::new()
      .width(360.0)
      .height(240.0)
      .overflow_visible()
      .child(list(&self.phone, 240.0))
      .child(phone_bar);

    let rows = [
      (
        "new",
        "usize",
        "",
        "How many are selected. The bar is shown while it is more than none, and comes in and leaves as it \
         changes.",
      ),
      (
        "text",
        "&str",
        "\"{n} selected\"",
        "What the bar says of the selection.",
      ),
      (
        "action / destructive_action",
        "Button",
        "",
        "An action on what is selected: a round ghost button; the one that destroys in the destructive color.",
      ),
      (
        "on_clear",
        "Fn()",
        "",
        "What clearing the selection does: the round button with a cross at the end, and Escape where it clears.",
      ),
      (
        "placement",
        "ActionBarPlacement",
        "Bottom",
        "At the bottom of the room it is given, or at its top, 16 from the edge and in the middle.",
      ),
      (
        "escape_clears",
        "bool",
        "false",
        "Escape clears the selection wherever the focus is, unless what has it took the key.",
      ),
      (
        "clear_label",
        "&str",
        "\"Clear selection\"",
        "What the clear button is called, to what reads the tree.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Navigation", "Action Bar"],
        title: "Action Bar",
        description: "The toolbar of a selection: over the list whose rows are selected, it says how many are, \
                      offers what can be done to them, and clears them.",
        import: Some("use lurq::ui::{ActionBar, ActionBarPlacement};"),
        status: Some("New"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Select rows of the list: the bar comes up from the bottom while any is selected and goes as none is. \
           Its cross clears them, and so does Escape with escape_clears on. Tab reaches the bar once; the arrow \
           keys move between its buttons.",
          playground,
        ),
        Section::new(
          "At the top",
          "The bar can stand at the top of what it acts on; it comes down from the edge. Escape clears here.",
          self.frame.demo(
            0,
            top,
            ".placement(ActionBarPlacement::Top)",
            &[
              "Stack::new()",
              "    .child(list)",
              "    .child(",
              "        ActionBar::new(selected.len())",
              "            .action(Button::new(\"Move\").icon(icons::FOLDER_IMPORT))",
              "            .on_clear(move || selection.set(Vec::new()))",
              "            .placement(ActionBarPlacement::Top)",
              "            .escape_clears(true),",
              "    )",
            ],
          ),
        ),
        Section::new(
          "A phone's selection bar",
          "The same parts restyled into the board's selection bar of a phone: across the top, 56 high, in the fill \
           of what is picked out, with icons of 22 in targets of 44.",
          self.frame.demo(
            1,
            Column::new()
              .spacing(8.0)
              .child(phone)
              .child(text("Mobile/Selection Bar", 12.0, FontWeight::Normal)),
            ".style(ActionBarPart::Bar, |s| s.height(56.0).radius(0.0))",
            &[
              "ActionBar::new(count)",
              "    .placement(ActionBarPlacement::Top)",
              "    .action(Button::new(\"Archive\").icon(icons::ARCHIVE).icon_only(true))",
              "    .destructive_action(Button::new(\"Delete\").icon(icons::TRASH).icon_only(true))",
              "    .style(ActionBarPart::Bar, |s| s.width(Length::Fill).height(56.0).radius(0.0)",
              "        .background(PaletteColor::Highlight))",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What an action bar is given.",
          props_table(&rows, &["new"], self.frame.room()),
        ),
      ],
    )
  }
}
