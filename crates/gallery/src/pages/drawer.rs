//! The page of the drawer: its playground, the board's navigation drawer
//! beside the page it pushes aside, and the same drawer over the page with a
//! veil, as on a narrow window.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx, theme::PaletteColor},
  components::{Column, Row},
  core::{ElementRef, Signal},
  layout::{Alignment, text_style::FontWeight},
  node::{BoxShadowValue, Element, border::Border},
  ui::{
    Avatar, Button, ButtonSize, ButtonVariant, Drawer, Length, Menu, MenuItem, MenuPart, Separator, SheetSide, icons,
  },
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, line, props_table, segmented_fill, text},
  playground::{Playground, Spec, divider, flag, prop},
};

/// The sides, as the code names them.
const SIDES: [(SheetSide, &str, &str); 4] = [
  (SheetSide::Start, "start", "SheetSide::Start"),
  (SheetSide::End, "end", "SheetSide::End"),
  (SheetSide::Top, "top", "SheetSide::Top"),
  (SheetSide::Bottom, "bottom", "SheetSide::Bottom"),
];

/// The board's navigation drawer: who is signed in, and where to go.
fn navigation() -> Vec<Element> {
  let account = Column::new()
    .width(FILL)
    .padding(20.0)
    .spacing(10.0)
    .child(Avatar::new("Ivan Zaida"))
    .child(
      Column::new()
        .spacing(2.0)
        .child(text("Ivan Zaida", 16.0, FontWeight::SemiBold))
        .child(text("ivan@lurq.dev", 13.0, FontWeight::Normal).color(PaletteColor::MutedForeground)),
    );
  let places = Menu::new(ElementRef::default(), Signal::new(true))
    .inline(true)
    .items([
      MenuItem::new("Inbox").icon(icons::INBOX).checked(true),
      MenuItem::new("Starred").icon(icons::STAR),
      MenuItem::new("Sent").icon(icons::SEND),
      MenuItem::new("Settings").icon(icons::SETTINGS),
    ])
    // A list in the drawer, not a panel over the window.
    .style(MenuPart::Panel, |s| {
      s.width(Length::Fill)
        .background(PaletteColor::Background)
        .border(Border::inside(0.0, PaletteColor::Border))
        .shadow(BoxShadowValue::none())
        .padding_x(8.0)
        .padding_y(8.0)
    });
  vec![account.into(), Separator::horizontal().into(), places.into()]
}

/// A layout of the size of a small window, with `drawer` at its `side` and the page beside it: a bar with a
/// button that opens and closes the drawer, and what the page shows.
fn layout(drawer: Drawer, side: SheetSide, open: &Signal<bool>) -> Element {
  let toggle = open.clone();
  let page = Column::new()
    .flex(1.0)
    .height(FILL)
    .padding(16.0)
    .spacing(12.0)
    .child(
      Row::new()
        .spacing(8.0)
        .align_items(Alignment::Center)
        .child(
          Button::new("Menu")
            .variant(ButtonVariant::Ghost)
            .size(ButtonSize::Sm)
            .icon(icons::MENU)
            .icon_only(true)
            .on_click(move |_| toggle.update(|open| *open = !*open)),
        )
        .child(text("Inbox", 15.0, FontWeight::SemiBold)),
    )
    .child(text("3 unread conversations", 13.0, FontWeight::Normal).color(PaletteColor::MutedForeground));
  let frame = |content: Element| -> Element {
    Column::new()
      .size(640.0, 340.0)
      .border(line())
      .rounded(8.0)
      .clip()
      .child(content)
      .into()
  };
  let drawer: Element = drawer.into();
  match side {
    SheetSide::Start => frame(Row::new().size(FILL, FILL).child(drawer).child(page).into()),
    SheetSide::End => frame(Row::new().size(FILL, FILL).child(page).child(drawer).into()),
    SheetSide::Top => frame(Column::new().size(FILL, FILL).child(drawer).child(page).into()),
    SheetSide::Bottom => frame(Column::new().size(FILL, FILL).child(page).child(drawer).into()),
  }
}

pub struct DrawerPage {
  frame: Frame,
  playground: Playground,
  open: Signal<bool>,
  side: Signal<usize>,
  overlay: Signal<bool>,
  dismiss_on_veil: Signal<bool>,
  dismiss_on_escape: Signal<bool>,
  beside: Signal<bool>,
  over: Signal<bool>,
}

impl DrawerPage {
  /// The playground: the drawer in a layout, with the button that opens and closes it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (side, side_name, side_code) = SIDES[self.side.get().min(SIDES.len() - 1)];
    let overlay = self.overlay.get();
    let (on_veil, on_escape) = (self.dismiss_on_veil.get(), self.dismiss_on_escape.get());
    let make = move |open: Signal<bool>| {
      Drawer::new(open)
        .side(side)
        .label("Navigation")
        .overlay(overlay)
        .dismiss_on_veil(on_veil)
        .dismiss_on_escape(on_escape)
        .children(navigation())
    };
    let shown = layout(make(self.open.clone()), side, &self.open);

    let mut code = vec![
      "// open: Signal<bool>".to_owned(),
      "Drawer::new(open.clone())".to_owned(),
    ];
    if side != SheetSide::Start {
      code.push(format!("    .side({side_code})"));
    }
    code.push("    .label(\"Navigation\")".to_owned());
    if overlay {
      code.push("    .overlay(true)".to_owned());
      for (name, set) in [("dismiss_on_veil", on_veil), ("dismiss_on_escape", on_escape)] {
        if !set {
          code.push(format!("    .{name}(false)"));
        }
      }
    }
    code.push("    .children(navigation)".to_owned());

    let tree = if overlay {
      vec![
        "sheet       over the page, at its edge".to_owned(),
        "└─ body     what the drawer holds".to_owned(),
      ]
    } else {
      let across = if side.is_side() {
        "0 ↔ 300 px wide"
      } else {
        "0 ↔ its content's height"
      };
      vec![
        format!("root        {across}   in the layout"),
        "└─ panel    background · line toward the page · fades".to_owned(),
        "   └─ body  scrolls".to_owned(),
      ]
    };

    let to_side = self.side.clone();
    let to = (self.side.clone(), self.overlay.clone());
    let dismissals = [self.dismiss_on_veil.clone(), self.dismiss_on_escape.clone()];
    let sides: Vec<(&str, usize)> = SIDES.iter().enumerate().map(|(index, side)| (side.1, index)).collect();
    let mut controls = vec![
      prop(
        "side",
        "SheetSide",
        segmented_fill(&sides, &self.side.get(), move |index| to_side.set(index)),
      ),
      divider(),
      flag("open", &self.open),
      flag("overlay", &self.overlay),
    ];
    if overlay {
      controls.push(flag("dismiss_on_veil", &self.dismiss_on_veil));
      controls.push(flag("dismiss_on_escape", &self.dismiss_on_escape));
    }
    self.playground.card(
      ctx,
      Spec {
        title: "Drawer",
        make: Arc::new(move || {
          let open = Signal::new(true);
          layout(make(open.clone()).overlay(false), side, &open)
        }),
        shown,
        fill: false,
        forceable: false,
        facts: format!(
          "{side_name} · {} · {}",
          if overlay { "over the page" } else { "beside the page" },
          if self.open.get() { "open" } else { "closed" }
        ),
        event: "",
        code,
        tree,
        controls,
        changed: [side != SheetSide::Start, overlay, !on_veil, !on_escape]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset: Arc::new(move || {
          to.0.set(0);
          to.1.set(false);
          for flag in &dismissals {
            flag.set(true);
          }
        }),
      },
    )
  }
}

impl Component for DrawerPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      open: ctx.signal(true),
      side: ctx.signal(0),
      overlay: ctx.signal(false),
      dismiss_on_veil: ctx.signal(true),
      dismiss_on_escape: ctx.signal(true),
      beside: ctx.signal(true),
      over: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let beside = layout(
      Drawer::new(self.beside.clone())
        .label("Navigation")
        .children(navigation()),
      SheetSide::Start,
      &self.beside,
    );
    let over = layout(
      Drawer::new(self.over.clone())
        .label("Navigation")
        .overlay(true)
        .children(navigation()),
      SheetSide::Start,
      &self.over,
    );
    let rows = [
      (
        "open",
        "Signal<bool>",
        "",
        "Whether it is open. The app's; over the page, Escape and a press on the veil set it to false.",
      ),
      (
        "side",
        "SheetSide",
        "Start",
        "The side of the layout it stands at: Start and End open with its width, Top and Bottom with its height.",
      ),
      (
        "label",
        "&str",
        "",
        "What it is called to tooling: a complementary region, or a dialog over the page.",
      ),
      (
        "child",
        "impl Into<Element>",
        "",
        "Adds an element to the drawer, which scrolls.",
      ),
      ("width", "f32", "300", "A width of its own at the start or the end."),
      (
        "height",
        "f32",
        "content",
        "A height of its own at the top or the bottom.",
      ),
      (
        "overlay",
        "bool",
        "false",
        "Over the page with a veil, as a Sheet at its edge, for a window too narrow to push the page aside.",
      ),
      (
        "dismiss_on_veil",
        "bool",
        "true",
        "Over the page, whether a press on the veil closes it.",
      ),
      (
        "dismiss_on_escape",
        "bool",
        "true",
        "Over the page, whether Escape closes it.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Surfaces", "Drawer"],
        title: "Drawer",
        description: "A panel at one side of a layout, beside the page rather than over it, that opens and closes: \
                      the navigation of an app, a pane of details or of filters.",
        import: Some("use lurq::ui::Drawer;"),
        status: Some("Stable"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Drawer is a control on the right. The button in the page's bar opens and closes it.",
          playground,
        ),
        Section::new(
          "Beside the page",
          "As it opens its room grows from nothing to its 300 and pushes the page aside, and what it holds fades \
           in; as it closes what it held is gone, so that Tab does not reach it.",
          self.frame.demo(
            0,
            beside,
            "Drawer::new(open.clone())",
            &[
              "// open: Signal<bool>",
              "Row::new()",
              "    .child(Drawer::new(open.clone()).label(\"Navigation\").children(navigation))",
              "    .child(page)",
            ],
          ),
        ),
        Section::new(
          "Over the page",
          "On a window too narrow to push the page aside it lies over the page with a veil: a Sheet at its edge, \
           which Escape and a press on the veil close.",
          self.frame.demo(
            1,
            over,
            ".overlay(true)",
            &[
              "// open: Signal<bool>",
              "Row::new()",
              "    .child(Drawer::new(open.clone()).label(\"Navigation\").overlay(true).children(navigation))",
              "    .child(page)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a drawer is given.",
          props_table(&rows, &["open"], self.frame.room()),
        ),
      ],
    )
  }
}
