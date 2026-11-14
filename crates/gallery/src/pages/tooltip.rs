//! The page of the tooltip: its playground, a toolbar whose buttons have
//! tooltips, the four sides with and without an arrow, a shortcut and a long
//! text.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  layout::{Alignment, layout_kind::Justify},
  node::Element,
  ui::{Button, ButtonVariant, Tooltip, TooltipSide},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Spec, divider, field, flag, prop},
};

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .spacing(10.0)
    .align_items(Alignment::Center)
    .child(Row::new().child(shown))
    .child(quiet(caption, 12.0))
}

/// What is on a stage, side by side, with room around each for its tooltip.
fn items() -> Row {
  Row::new()
    .width(FILL)
    .padding_vertical(48.0)
    .spacing(72.0)
    .wrap()
    .justify(Justify::Center)
    .align_items(Alignment::End)
}

/// The sides the playground offers, and the index of each in the panel's select.
const SIDES: [(&str, TooltipSide); 4] = [
  ("Top", TooltipSide::Top),
  ("Bottom", TooltipSide::Bottom),
  ("Left", TooltipSide::Left),
  ("Right", TooltipSide::Right),
];

const CHOICES: [(&str, usize); 4] = [("Top", 0), ("Bottom", 1), ("Left", 2), ("Right", 3)];

/// A button of a toolbar: its icon alone.
fn tool(label: &str, icon: &str) -> Button {
  Button::new(label)
    .variant(ButtonVariant::Ghost)
    .icon(icon)
    .icon_only(true)
}

pub struct TooltipPage {
  frame: Frame,
  playground: Playground,
  text: Signal<String>,
  shortcut: Signal<String>,
  /// The side, as its index in [`SIDES`].
  side: Signal<usize>,
  arrow: Signal<bool>,
  open: Signal<bool>,
  disabled: Signal<bool>,
}

impl TooltipPage {
  /// The playground: a button with the tooltip the panel's props make.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (text, shortcut, chosen) = (self.text.get(), self.shortcut.get(), self.side.get());
    let (side_name, side) = SIDES[chosen.min(SIDES.len() - 1)];
    let (arrow, open, disabled) = (self.arrow.get(), self.open.get(), self.disabled.get());
    let plain = {
      let (text, shortcut) = (text.clone(), shortcut.clone());
      move || {
        Tooltip::new(&text)
          .shortcut(&shortcut)
          .side(side)
          .arrow(arrow)
          .disabled(disabled)
      }
    };
    let (record, words) = (self.playground.recorder(), text.clone());
    let button = Button::new("Copy")
      .variant(ButtonVariant::Outline)
      .icon("copy")
      .on_click(move |_| record(format!("pressed: {words:?} went")));
    let shown = plain().open(open.then_some(true)).mount(ctx, button);

    let mut code = vec![format!("Tooltip::new({text:?})")];
    if !shortcut.is_empty() {
      code.push(format!("    .shortcut({shortcut:?})"));
    }
    if side != TooltipSide::Top {
      code.push(format!("    .side(TooltipSide::{side_name})"));
    }
    if arrow {
      code.push("    .arrow(true)".to_owned());
    }
    if open {
      code.push("    .open(Some(true))".to_owned());
    }
    if disabled {
      code.push("    .disabled(true)".to_owned());
    }
    code.push("    .mount(ctx, Button::new(\"Copy\").icon(\"copy\"))".to_owned());

    let mut tree = vec![
      "button    the element it describes, its root the anchor".to_owned(),
      format!("└─ root   6 from the button, {}", side_name.to_lowercase()),
      format!(
        "   {} content  primary, radius 6, 10 × 6 padding, at most 200 wide",
        if arrow { "├─" } else { "└─" }
      ),
      format!(
        "   {}  {} label   12, primary foreground",
        if arrow { "│" } else { " " },
        if shortcut.is_empty() { "└─" } else { "├─" }
      ),
    ];
    if !shortcut.is_empty() {
      tree.push(format!(
        "   {}  └─ shortcut  16 high, mono 10   {shortcut:?}",
        if arrow { "│" } else { " " }
      ));
    }
    if arrow {
      tree.push("   └─ arrow    12 × 6, the box's color".to_owned());
    }

    let (to_text, to_shortcut, to_side) = (self.text.clone(), self.shortcut.clone(), self.side.clone());
    let (to_arrow, to_open, to_disabled) = (self.arrow.clone(), self.open.clone(), self.disabled.clone());
    let set_side = self.side.clone();
    self.playground.card(
      ctx,
      Spec {
        title: "Tooltip",
        make: Arc::new(move || plain().into()),
        shown,
        fill: false,
        forceable: false,
        facts: if open {
          "open".to_owned()
        } else {
          "on hover and focus".to_owned()
        },
        event: "on_click",
        code,
        tree,
        controls: vec![
          prop("text", "&str", field(&self.text)),
          prop("shortcut", "&str", field(&self.shortcut)),
          prop(
            "side",
            "TooltipSide",
            self
              .playground
              .select(0, &CHOICES, &chosen, move |side| set_side.set(side)),
          ),
          divider(),
          flag("arrow", &self.arrow),
          flag("open", &self.open),
          flag("disabled", &self.disabled),
        ],
        changed: [!shortcut.is_empty(), side != TooltipSide::Top, arrow, !open, disabled]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset: Arc::new(move || {
          to_text.set("Copy link".to_owned());
          to_shortcut.set(String::new());
          to_side.set(0);
          to_arrow.set(false);
          to_open.set(true);
          to_disabled.set(false);
        }),
      },
    )
  }
}

impl Component for TooltipPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 1),
      text: ctx.signal("Copy link".to_owned()),
      shortcut: ctx.signal(String::new()),
      side: ctx.signal(0),
      arrow: ctx.signal(false),
      open: ctx.signal(true),
      disabled: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let mut toolbar = Row::new().spacing(4.0).align_items(Alignment::Center);
    for (label, icon, keys) in [
      ("Cut", "cut", "Ctrl+X"),
      ("Copy", "copy", "Ctrl+C"),
      ("Paste", "paste", "Ctrl+V"),
      ("Bold", "bold", "Ctrl+B"),
      ("Italic", "italic", "Ctrl+I"),
    ] {
      toolbar = toolbar.child(Tooltip::new(label).shortcut(keys).mount(ctx, tool(label, icon)));
    }
    let toolbar = Row::new()
      .width(FILL)
      .padding_vertical(40.0)
      .justify(Justify::Center)
      .child(toolbar);

    let mut sides = items();
    for (name, side) in SIDES {
      let button = Button::new(name).variant(ButtonVariant::Outline);
      let tooltip = Tooltip::new("Zoom in").side(side).arrow(true).open(Some(true));
      sides = sides.child(item(tooltip.mount(ctx, button), &name.to_lowercase()));
    }
    let plain = Tooltip::new("Zoom in").open(Some(true));
    sides = sides.child(item(
      plain.mount(ctx, Button::new("Plain").variant(ButtonVariant::Outline)),
      "no arrow",
    ));

    let long = "Blur radius is in logical pixels and scales with the display.";
    let more = items()
      .child(item(
        Tooltip::new("Toggle sidebar")
          .shortcut("Ctrl+B")
          .arrow(true)
          .open(Some(true))
          .mount(ctx, tool("Toggle sidebar", "panel-left")),
        "with a shortcut",
      ))
      .child(item(
        Tooltip::new(long)
          .open(Some(true))
          .mount(ctx, Button::new("Blur").variant(ButtonVariant::Outline)),
        "a long text wraps at 200",
      ))
      .child(item(
        Tooltip::new("Nothing to export yet").mount(
          ctx,
          Button::new("Export").variant(ButtonVariant::Outline).disabled(true),
        ),
        "a disabled button: hover it",
      ));

    let rows = [
      (
        "text",
        "&str",
        "",
        "What it says. It is what tooling is told the element is described by, and what the element is called if \
         nothing else names it.",
      ),
      (
        "mount",
        "(ctx, impl Into<Element>)",
        "",
        "Puts the tooltip on the element it describes and returns that element. The pointer and the focus are \
         its root's.",
      ),
      (
        "shortcut",
        "&str",
        "",
        "The keys of a shortcut, after the text in a chip of their own.",
      ),
      (
        "side",
        "TooltipSide",
        "Top",
        "The side of the element it is on, where there is room; on the other one where there is not.",
      ),
      (
        "arrow",
        "bool",
        "false",
        "A triangle from the box to the element, turned with the side it is on.",
      ),
      (
        "delay",
        "Duration",
        "500 ms",
        "How long the pointer rests before it shows. None within 300 ms of another tooltip going, and none for \
         the keyboard's focus.",
      ),
      (
        "open",
        "Option<bool>",
        "None",
        "Shown whatever the pointer and the focus do, or never: for a gallery, a test, a tour.",
      ),
      ("disabled", "bool", "false", "It never shows."),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Surfaces & overlays", "Tooltip"],
        title: "Tooltip",
        description: "A short text beside an element while the pointer rests on it or the keyboard's focus is on \
                      it: what an icon button does, and the keys that do it too.",
        import: Some("use lurq::ui::{Tooltip, TooltipSide};"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Tooltip is a control on the right. With open on, the tooltip stays; turn it off and rest \
           the pointer on the button, or reach it with Tab. A press of the button puts the tooltip away.",
          playground,
        ),
        Section::new(
          "On a toolbar",
          "Rest the pointer on a button: its tooltip shows after half a second, and the next one along shows at \
           once. The pointer can be moved onto a tooltip to read it. Tab shows them with no delay, and Escape puts \
           one away.",
          self.frame.demo(
            0,
            toolbar,
            "Tooltip::new(\"Copy\").shortcut(\"Ctrl+C\").mount(ctx, button)",
            &[
              "let copy = Button::new(\"Copy\")",
              "    .variant(ButtonVariant::Ghost)",
              "    .icon(\"copy\")",
              "    .icon_only(true);",
              "Tooltip::new(\"Copy\").shortcut(\"Ctrl+C\").mount(ctx, copy)",
            ],
          ),
        ),
        Section::new(
          "Sides",
          "Above the element unless it says otherwise, 6 from it and centered on it. The arrow points at the \
           element, and turns with the tooltip where there is no room on its side.",
          self.frame.demo(
            1,
            sides,
            ".side(TooltipSide::Bottom).arrow(true)",
            &[
              "let zoom = Button::new(\"Bottom\").variant(ButtonVariant::Outline);",
              "Tooltip::new(\"Zoom in\")",
              "    .side(TooltipSide::Bottom)",
              "    .arrow(true)",
              "    .mount(ctx, zoom)",
            ],
          ),
        ),
        Section::new(
          "A shortcut, a longer text, a disabled button",
          "A shortcut is a chip after the text. A text longer than 200 wraps, with 8 above and below it. A disabled \
           button shows its tooltip too: it can say why it is disabled.",
          self.frame.demo(
            2,
            more,
            ".shortcut(\"Ctrl+B\")",
            &[
              "Tooltip::new(\"Toggle sidebar\")",
              "    .shortcut(\"Ctrl+B\")",
              "    .arrow(true)",
              "    .mount(ctx, sidebar_button)",
              "",
              "Tooltip::new(\"Blur radius is in logical pixels and scales with the display.\")",
              "    .mount(ctx, blur_button)",
              "",
              "let export = Button::new(\"Export\").variant(ButtonVariant::Outline).disabled(true);",
              "Tooltip::new(\"Nothing to export yet\").mount(ctx, export)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a tooltip is given.",
          props_table(&rows, &["text", "mount"], self.frame.room()),
        ),
      ],
    )
  }
}
