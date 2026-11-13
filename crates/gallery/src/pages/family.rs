//! The rest of the button family: a toggle, a group of buttons, a group of
//! toggles, a segmented control and a link. Each page is live: what is on
//! it is the component, bound to a signal of the page.

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{
    Button, ButtonGroup, ButtonGroupVariant, ButtonSize, Choice, Interaction, Link, SegmentedControl, Toggle,
    ToggleGroup, icons,
  },
};

use super::family_play::{ButtonGroupPlay, LinkPlay, SegmentedPlay, ToggleGroupPlay, TogglePlay};
use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, demo_card, props_table, quiet, text},
};

const SIZES: [(ButtonSize, &str); 5] = [
  (ButtonSize::Xs, "xs · 28"),
  (ButtonSize::Sm, "sm · 32"),
  (ButtonSize::Md, "md · 36"),
  (ButtonSize::Lg, "lg · 40"),
  (ButtonSize::Xl, "xl · 44"),
];

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .spacing(9.0)
    .child(Row::new().child(shown))
    .child(quiet(caption, 12.0))
}

/// What is on a stage, side by side, standing on one line; what does not fit goes on to the next.
fn items() -> Row {
  Row::new().width(FILL).spacing(28.0).wrap().align_items(Alignment::End)
}

// --- Toggle ---------------------------------------------------------------

pub struct TogglePage {
  frame: Frame,
  play: TogglePlay,
  bold: Signal<bool>,
  italic: Signal<bool>,
  pinned: Signal<bool>,
}

impl Component for TogglePage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      play: TogglePlay::new(ctx),
      bold: ctx.signal(true),
      italic: ctx.signal(false),
      pinned: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let bound = |label: &str, glyph: &str, on: &Signal<bool>| {
      let set = on.clone();
      Toggle::new(label)
        .icon(glyph)
        .on(on.get())
        .on_change(move |next| set.set(next))
    };
    let live = items()
      .child(item(bound("Bold", icons::BOLD, &self.bold), "on(bold.get())"))
      .child(item(bound("Italic", icons::ITALIC, &self.italic), "off"))
      .child(item(
        bound("Pin", icons::PIN, &self.pinned).icon_only(true),
        "icon_only(true)",
      ));
    let mut sizes = items();
    for (size, caption) in SIZES {
      sizes = sizes.child(item(Toggle::new("Bold").icon(icons::BOLD).size(size).on(true), caption));
    }
    let states = items()
      .child(item(Toggle::new("Bold"), "off"))
      .child(item(Toggle::new("Bold").force(Some(Interaction::Hovered)), "hover"))
      .child(item(Toggle::new("Bold").on(true), "on"))
      .child(item(
        Toggle::new("Bold").on(true).force(Some(Interaction::Hovered)),
        "on, hover",
      ))
      .child(item(
        Toggle::new("Bold").force(Some(Interaction::FocusVisible)),
        "focus-visible",
      ))
      .child(item(Toggle::new("Bold").on(true).disabled(true), "disabled"));
    let rows = [
      (
        "label",
        "&str",
        "",
        "The text in the toggle, and what an icon alone is called.",
      ),
      (
        "on",
        "bool",
        "false",
        "Whether it is on. The app says so: the toggle keeps nothing.",
      ),
      (
        "on_change",
        "impl Fn(bool)",
        "",
        "Called on a press with what the toggle is to be: on if it was off.",
      ),
      (
        "icon",
        "impl Into<Name>",
        "none",
        "An icon of the theme's set before the label.",
      ),
      (
        "icon_only",
        "bool",
        "false",
        "The icon alone: a square as wide as the toggle is high.",
      ),
      ("size", "ButtonSize", "Md", "As high as a button of that size."),
      ("disabled", "bool", "false", "Half the opacity; it takes no input."),
    ];
    let playground = self.play.card(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Actions", "Toggle"],
        title: "Toggle",
        description: "A button that stays on until it is pressed again: bold in an editor, a filter that holds, a \
                      panel that is shown.",
        import: Some("use lurq::ui::Toggle;"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Toggle is a control on the right. The toggle on the stage is live: a press turns it on, and the \
           switch beside it follows.",
          playground,
        ),
        Section::new(
          "Toggle",
          "Press one: it is on until it is pressed again. The page holds whether each is on, in a signal.",
          demo_card(
            live,
            "Toggle::new(\"Bold\").icon(icons::BOLD).on(bold.get()).on_change(..)",
          ),
        ),
        Section::new(
          "Sizes",
          "A toggle is as high as a button of the same size.",
          demo_card(sizes, ".size(ButtonSize::Md)"),
        ),
        Section::new(
          "States",
          "Off it has no fill until the pointer is on it. On it has the fill of what is picked out, a step darker \
           under the pointer.",
          demo_card(states, ".on(true)"),
        ),
        Section::new("Props", "What a toggle is given.", props_table(&rows, &["label"])),
      ],
    )
  }
}

// --- Button group ---------------------------------------------------------

pub struct ButtonGroupPage {
  frame: Frame,
  play: ButtonGroupPlay,
  zoom: Signal<u32>,
}

impl Component for ButtonGroupPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      play: ButtonGroupPlay::new(ctx),
      zoom: ctx.signal(100),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let (out, back, closer) = (self.zoom.clone(), self.zoom.clone(), self.zoom.clone());
    let outline = items()
      .child(item(
        ButtonGroup::new()
          .child(Button::new("Years"))
          .child(Button::new("Months"))
          .child(Button::new("Days")),
        "three buttons",
      ))
      .child(item(
        ButtonGroup::new()
          .child(
            Button::new("Zoom out")
              .icon(icons::ZOOM_OUT)
              .icon_only(true)
              .on_click(move |_| out.update(|zoom| *zoom = zoom.saturating_sub(10).max(10))),
          )
          .child(Button::new(&format!("{}%", self.zoom.get())).on_click(move |_| back.set(100)))
          .child(
            Button::new("Zoom in")
              .icon(icons::ZOOM_IN)
              .icon_only(true)
              .on_click(move |_| closer.update(|zoom| *zoom = (*zoom + 10).min(400))),
          ),
        "live: each button is pressed on its own",
      ));
    let split = |variant: ButtonGroupVariant, label: &str| {
      ButtonGroup::new()
        .variant(variant)
        .child(Button::new(label))
        .child(Button::new("How").icon(icons::CHEVRON_DOWN).icon_only(true))
    };
    let filled = items()
      .child(item(split(ButtonGroupVariant::Primary, "Merge"), "primary"))
      .child(item(split(ButtonGroupVariant::Secondary, "Export"), "secondary"))
      .child(item(split(ButtonGroupVariant::Destructive, "Delete"), "destructive"));
    let mut sizes = items();
    for (size, caption) in SIZES {
      sizes = sizes.child(item(
        ButtonGroup::new()
          .size(size)
          .child(Button::new("Day"))
          .child(Button::new("Week")),
        caption,
      ));
    }
    let rows = [
      ("child, children", "Button", "", "The buttons of the group, in order."),
      (
        "variant",
        "ButtonGroupVariant",
        "Outline",
        "Outline: a line around the group and one between its buttons. Primary, Secondary, Destructive: buttons \
         in that fill, a pixel apart.",
      ),
      ("size", "ButtonSize", "Md", "The size of every button in the group."),
    ];
    let playground = self.play.card(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Actions", "Button Group"],
        title: "Button Group",
        description: "Buttons that belong together, joined into one shape: the units of a period, the steps of a \
                      zoom, an action and the choice of how to do it.",
        import: Some("use lurq::ui::ButtonGroup;"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "The group on the stage follows the controls on the right, and each of its buttons is pressed on its own: \
           the Events tab says which.",
          playground,
        ),
        Section::new(
          "Outline",
          "One line around the buttons and one between them. The group gives its buttons their size and squares \
           their corners; each keeps what a press of it does.",
          demo_card(outline, "ButtonGroup::new().child(Button::new(\"Years\")).child(..)"),
        ),
        Section::new(
          "Split button",
          "A filled group of two: the action, and beside it what opens the choice of how to do it. What it opens \
           is the app's: the design system has no menu yet.",
          demo_card(filled, ".variant(ButtonGroupVariant::Primary)"),
        ),
        Section::new(
          "Sizes",
          "The group gives its size to every button in it.",
          demo_card(sizes, ".size(ButtonSize::Md)"),
        ),
        Section::new("Props", "What a group is given.", props_table(&rows, &[])),
      ],
    )
  }
}

// --- Toggle group ---------------------------------------------------------

pub struct ToggleGroupPage {
  frame: Frame,
  play: ToggleGroupPlay,
  align: Signal<usize>,
  marks: Signal<Vec<usize>>,
}

impl Component for ToggleGroupPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      play: ToggleGroupPlay::new(ctx),
      align: ctx.signal(0),
      marks: ctx.signal(vec![0]),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let aligned = ["Left", "Center", "Right"][self.align.get().min(2)];
    let names = ["bold", "italic", "underline"];
    let marked: Vec<&str> = self
      .marks
      .get()
      .into_iter()
      .filter_map(|mark| names.get(mark).copied())
      .collect();
    let marks = || {
      [
        Choice::new(0usize, "Bold").icon(icons::BOLD).icon_only(true),
        Choice::new(1, "Italic").icon(icons::ITALIC).icon_only(true),
        Choice::new(2, "Underline").icon(icons::UNDERLINE).icon_only(true),
      ]
    };
    let one = items().child(item(
      ToggleGroup::new(self.align.clone(), [(0usize, "Left"), (1, "Center"), (2, "Right")]),
      &format!("the signal has: {aligned}"),
    ));
    let many = items().child(item(
      ToggleGroup::multiple(self.marks.clone(), marks()),
      &format!(
        "the signal has: {}",
        if marked.is_empty() {
          "nothing".to_owned()
        } else {
          marked.join(", ")
        }
      ),
    ));
    let mut sizes = items();
    for (size, caption) in SIZES {
      sizes = sizes.child(item(
        ToggleGroup::multiple(self.marks.clone(), marks()).size(size),
        caption,
      ));
    }
    let rows = [
      (
        "new",
        "Signal<T>, choices",
        "",
        "Exactly one toggle is on: the one with the signal's value. A press of another sets the signal.",
      ),
      (
        "multiple",
        "Signal<Vec<T>>, choices",
        "",
        "Any number are on. A press adds a value to the signal, or takes it out.",
      ),
      (
        "controlled",
        "values, choices, press",
        "",
        "The app holds what is on, and is told the value of each toggle pressed.",
      ),
      (
        "choices",
        "(T, &str) or Choice<T>",
        "",
        "A value and what it is called; a Choice adds an icon, the icon alone, or disabled.",
      ),
      ("size", "ButtonSize", "Md", "The size of every toggle in the group."),
      ("disabled", "bool", "false", "The whole group takes no input."),
    ];
    let playground = self.play.card(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Actions", "Toggle Group"],
        title: "Toggle Group",
        description: "Toggles that are one choice: the alignment of a paragraph, of which one is on, or the marks \
                      of a text, of which any number are.",
        import: Some("use lurq::ui::{Choice, ToggleGroup};"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Bound to one value or to a list of them: the control on the right says which, and the stage says what is on.",
          playground,
        ),
        Section::new(
          "One of them",
          "Bound to a signal of one value: the toggle with that value is on.",
          demo_card(one, "ToggleGroup::new(align, [(Align::Left, \"Left\"), ..])"),
        ),
        Section::new(
          "Any number",
          "Bound to a signal of a list: a press adds a value, and a press of a toggle that is on takes it out.",
          demo_card(
            many,
            "ToggleGroup::multiple(marks, [Choice::new(..).icon(..).icon_only(true), ..])",
          ),
        ),
        Section::new(
          "Sizes",
          "The group gives its size to every toggle in it. These are all bound to the signal above.",
          demo_card(sizes, ".size(ButtonSize::Md)"),
        ),
        Section::new("Props", "What a group of toggles is given.", props_table(&rows, &[])),
      ],
    )
  }
}

// --- Segmented control ----------------------------------------------------

pub struct SegmentedControlPage {
  frame: Frame,
  play: SegmentedPlay,
  period: Signal<usize>,
  view: Signal<usize>,
}

impl Component for SegmentedControlPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      play: SegmentedPlay::new(ctx),
      period: ctx.signal(1),
      view: ctx.signal(0),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let periods = [(0usize, "Day"), (1, "Week"), (2, "Month")];
    let chosen = periods[self.period.get().min(2)].1;
    let live = items()
      .child(item(
        SegmentedControl::new(self.period.clone(), periods),
        &format!("the signal has: {chosen}"),
      ))
      .child(item(
        SegmentedControl::new(
          self.view.clone(),
          [
            Choice::new(0usize, "List").icon(icons::LIST),
            Choice::new(1, "Table").icon(icons::TABLE),
            Choice::new(2, "Board").icon(icons::DASHBOARD),
          ],
        ),
        "with icons",
      ))
      .child(item(
        SegmentedControl::new(
          self.view.clone(),
          [
            Choice::new(0usize, "List").icon(icons::LIST).icon_only(true),
            Choice::new(1, "Table").icon(icons::TABLE).icon_only(true),
            Choice::new(2, "Board").icon(icons::DASHBOARD).icon_only(true),
          ],
        ),
        "icons alone",
      ));
    let mut sizes = items();
    for (size, caption) in SIZES {
      sizes = sizes.child(item(
        SegmentedControl::new(self.period.clone(), periods).size(size),
        caption,
      ));
    }
    let wide = Column::new()
      .width(FILL)
      .spacing(20.0)
      .child(
        Column::new()
          .width(FILL)
          .spacing(9.0)
          .child(SegmentedControl::new(self.period.clone(), periods).full_width(true))
          .child(quiet("full_width(true): the segments share the width", 12.0)),
      )
      .child(item(
        SegmentedControl::new(self.period.clone(), periods).disabled(true),
        "disabled(true)",
      ));
    let rows = [
      (
        "new",
        "Signal<T>, choices",
        "",
        "The chosen segment is the one with the signal's value. A press of another sets the signal.",
      ),
      (
        "controlled",
        "value, choices, choose",
        "",
        "The app holds what is chosen, and is told the value of each segment pressed.",
      ),
      (
        "choices",
        "(T, &str) or Choice<T>",
        "",
        "A value and what it is called; a Choice adds an icon, the icon alone, or disabled.",
      ),
      (
        "size",
        "ButtonSize",
        "Md",
        "The track is as high as a button of that size.",
      ),
      (
        "full_width",
        "bool",
        "false",
        "As wide as what it is in, its segments sharing the width.",
      ),
      ("disabled", "bool", "false", "Half the opacity; no segment is pressed."),
    ];
    let playground = self.play.card(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Actions", "Segmented Control"],
        title: "Segmented Control",
        description: "One of a few choices that are all in view, side by side on a track: the period of a chart, \
                      the appearance of an app. The switches in this gallery's header are one.",
        import: Some("use lurq::ui::SegmentedControl;"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "The control on the stage follows the controls on the right, and says which of its segments is chosen.",
          playground,
        ),
        Section::new(
          "Segmented control",
          "Exactly one segment is chosen. Each is a button: Tab reaches it, and a click, Enter or Space chooses it.",
          demo_card(live, "SegmentedControl::new(period, [(Period::Day, \"Day\"), ..])"),
        ),
        Section::new(
          "Sizes",
          "The track is as high as a button of the same size. These are bound to the signal above.",
          demo_card(sizes, ".size(ButtonSize::Md)"),
        ),
        Section::new(
          "Width and state",
          "Its own width is that of its segments, unless it fills what it is in.",
          demo_card(wide, ".full_width(true)"),
        ),
        Section::new("Props", "What a segmented control is given.", props_table(&rows, &[])),
      ],
    )
  }
}

// --- Link -----------------------------------------------------------------

pub struct LinkPage {
  frame: Frame,
  play: LinkPlay,
  presses: Signal<u32>,
}

impl Component for LinkPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      play: LinkPlay::new(ctx),
      presses: ctx.signal(0),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let count = self.presses.clone();
    let live = items()
      .child(item(
        Link::new("Read the docs")
          .external()
          .on_click(move |_| count.update(|presses| *presses += 1)),
        &format!("pressed {} times", self.presses.get()),
      ))
      .child(item(Link::new("Privacy policy"), "no icon"))
      .child(item(
        Link::new("Next page").icon(icons::ARROW_RIGHT),
        "icon(icons::ARROW_RIGHT)",
      ));
    let states = items()
      .child(item(Link::new("Read the docs"), "at rest"))
      .child(item(
        Link::new("Read the docs").force(Some(Interaction::Hovered)),
        "hover",
      ))
      .child(item(
        Link::new("Read the docs").force(Some(Interaction::FocusVisible)),
        "focus-visible",
      ))
      .child(item(Link::new("Read the docs").disabled(true), "disabled"));
    let sentence = Row::new()
      .spacing(4.0)
      .align_items(Alignment::Center)
      .child(text(
        "By going on you agree to the",
        14.0,
        lurq::layout::text_style::FontWeight::Normal,
      ))
      .child(Link::new("terms of use"))
      .child(text("and the", 14.0, lurq::layout::text_style::FontWeight::Normal))
      .child(Link::new("privacy policy").external());
    let rows = [
      ("label", "&str", "", "The text of the link."),
      (
        "on_click",
        "impl Fn(MouseEvent)",
        "",
        "A click, and Enter and Space while it is focused. Where the link goes is what its press does.",
      ),
      (
        "icon",
        "impl Into<Name>",
        "none",
        "An icon of the theme's set after the text.",
      ),
      (
        "external",
        "",
        "",
        "The icon of a link that leaves the app: an arrow up and to the right.",
      ),
      ("disabled", "bool", "false", "Half the opacity; it takes no input."),
    ];
    let playground = self.play.card(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Actions", "Link"],
        title: "Link",
        description: "Text that goes somewhere when it is pressed. It has no box, so it sits in a line of text or \
                      beside one, and it is underlined under the pointer.",
        import: Some("use lurq::ui::Link;"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Link is a control on the right. The state switch draws it under the pointer and focused with \
           nothing pointing at it.",
          playground,
        ),
        Section::new(
          "Link",
          "Pressed by a click, and by Enter or Space while it is focused.",
          demo_card(live, "Link::new(\"Read the docs\").external().on_click(..)"),
        ),
        Section::new(
          "States",
          "A line under it while the pointer is on it, and the theme's ring when it is focused from the keyboard.",
          demo_card(states, "Link::new(\"Read the docs\")"),
        ),
        Section::new(
          "In a line",
          "Beside text of its size it stands on the same line.",
          demo_card(sentence, "Row::new().child(text).child(Link::new(\"terms of use\"))"),
        ),
        Section::new("Props", "What a link is given.", props_table(&rows, &["label"])),
      ],
    )
  }
}
