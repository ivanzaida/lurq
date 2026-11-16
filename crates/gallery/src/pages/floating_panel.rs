//! The page of the floating panel: its playground over a canvas, several panels over one content, and the header of
//! a panel on its own.

use std::sync::Arc;

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{PaletteColor, RadiusSize},
  },
  components::{Column, Row, Stack},
  core::Signal,
  layout::{Alignment, text_style::FontWeight},
  node::{Element, dimension::Dimension},
  ui::{FloatingPanel, FloatingPanels, MenuItem, PanelHeader, PanelPosition, PanelSize, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, fit, icon, line, phone, props_table, quiet, text},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

/// The layers of the board's panel: their names, and which is chosen and locked.
const LAYERS: [&str; 4] = ["Header", "Blur backdrop", "Card", "Background"];

/// The rows of the board's Layers panel: an eye, the name, a lock on the one chosen.
fn layers() -> Column {
  let mut rows = Column::new().spacing(2.0);
  for (index, name) in LAYERS.iter().enumerate() {
    let chosen = index == 1;
    let mut row = Row::new()
      .height(28.0)
      .padding_horizontal(8.0)
      .spacing(8.0)
      .align_items(Alignment::Center)
      .rounded(6.0)
      .child(icon(icons::EYE, 14.0).color(PaletteColor::MutedForeground))
      .child(Row::new().flex(1.0).child(text(name, 13.0, FontWeight::Normal)));
    if chosen {
      row = row
        .background(PaletteColor::Highlight)
        .child(icon(icons::LOCK, 13.0).color(PaletteColor::MutedForeground));
    }
    rows = rows.child(row);
  }
  rows
}

/// How wide the board's canvas is.
const CANVAS: f32 = 560.0;

/// How wide the playground's canvas is: the stage's, and in a window as narrow as a phone the board's, which the
/// stage scrolls to: a stage that narrow is little wider than a panel.
fn canvas_width() -> Dimension {
  if phone() { CANVAS.into() } else { FILL }
}

/// A canvas the panels float over: the content of the app's window, 560 by 340.
fn canvas() -> Column {
  Column::new()
    .width(FILL)
    .height(340.0)
    .rounded(RadiusSize::Md)
    .background(PaletteColor::Muted)
    .border(line())
    .align_items(Alignment::Center)
    .justify(lurq::layout::layout_kind::Justify::Center)
    .child(quiet("the canvas: what the panels float over", 12.0))
}

pub struct FloatingPanelPage {
  frame: Frame,
  playground: Playground,
  title: Signal<String>,
  position: Signal<PanelPosition>,
  size: Signal<Option<PanelSize>>,
  open: Signal<bool>,
  shown: Signal<bool>,
  collapsible: Signal<bool>,
  resizable: Signal<bool>,
  closable: Signal<bool>,
  media: Signal<bool>,
  more: Signal<bool>,
  header_more: Signal<bool>,
  /// Whether the playground's panel is active, as it tells.
  active: Signal<bool>,
}

impl FloatingPanelPage {
  /// The playground: the panel as the panel's props make it, over a canvas.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let title = self.title.get();
    let (collapsible, resizable, closable, media) = (
      self.collapsible.get(),
      self.resizable.get(),
      self.closable.get(),
      self.media.get(),
    );
    let shown_panel = self.shown.get();
    let active = self.active.clone();
    let (position, size, open, more, shown) = (
      self.position.clone(),
      self.size.clone(),
      self.open.clone(),
      self.more.clone(),
      self.shown.clone(),
    );
    let plain = {
      let title = title.clone();
      move |record: Option<Arc<dyn Fn(String) + Send + Sync>>| {
        let mut panel = FloatingPanel::new(&title)
          .id("panel")
          .position(position.clone())
          .size(size.clone())
          .open(open.clone())
          .collapsible(collapsible)
          .resizable(resizable)
          .menu(more.clone(), || [MenuItem::new("Lock all"), MenuItem::new("Show all")])
          .child(layers());
        let active = active.clone();
        let told = record.clone();
        panel = panel.on_active_change(move |now| {
          active.set(now);
          if let Some(record) = &told {
            record(format!("on_active_change({now})"));
          }
        });
        if media {
          panel = panel.media(icon(icons::LAYERS, 14.0).color(PaletteColor::MutedForeground));
        }
        if closable {
          let shown = shown.clone();
          let told = record.clone();
          panel = panel.on_dismiss(move || {
            shown.set(false);
            if let Some(record) = &told {
              record("on_dismiss".to_owned());
            }
          });
        }
        if let Some(record) = record {
          let (moved, sized, turned) = (record.clone(), record.clone(), record);
          panel = panel
            .on_position_change(move |(x, y)| moved(format!("on_position_change({x}, {y})")))
            .on_size_change(move |(width, height)| sized(format!("on_size_change({width}, {height})")))
            .on_open_change(move |open| turned(format!("on_open_change({open})")));
        }
        panel
      }
    };
    let record = self.playground.recorder();
    let shown = {
      let (plain, again) = (plain.clone(), self.shown.clone());
      Shown::built(move |ctx| {
        let record: Arc<dyn Fn(String) + Send + Sync> = Arc::new(record);
        let mut stage = Stack::new().width(canvas_width()).overflow_visible().child(canvas());
        if shown_panel {
          stage = stage.child(plain(Some(record)).mount(ctx));
        } else {
          stage = stage.child(
            Column::new()
              .width(FILL)
              .height(340.0)
              .align_items(Alignment::Center)
              .justify(lurq::layout::layout_kind::Justify::Center)
              .child(lurq::ui::Button::new("Show the panel again").on_click(move |_| again.set(true))),
          );
        }
        stage.into()
      })
    };

    let mut code = vec![
      format!("FloatingPanel::new({title:?})"),
      "    .position(position) // Signal<(f32, f32)>".to_owned(),
      "    .size(size)         // Signal<Option<(f32, f32)>>".to_owned(),
      "    .open(open)".to_owned(),
    ];
    for (on, call) in [
      (media, "    .media(Icon::new(icons::LAYERS))"),
      (!collapsible, "    .collapsible(false)"),
      (!resizable, "    .resizable(false)"),
      (closable, "    .on_dismiss(move || shown.set(false))"),
    ] {
      if on {
        code.push(call.to_owned());
      }
    }
    code.push("    .menu(more, || [MenuItem::new(\"Lock all\"), MenuItem::new(\"Show all\")])".to_owned());
    code.push("    .child(layers)".to_owned());
    code.push("    .mount(ctx)".to_owned());

    let (x, y) = self.position.get();
    let tree = vec![
      format!("layer       as large as the canvas · the panel at {x}, {y}"),
      format!(
        "└─ panel    {} · popover · radius-lg · shadow lg",
        match self.size.get() {
          Some((width, height)) => format!("{width} × {height}"),
          None => "240 wide, as high as its rows".to_owned(),
        }
      ),
      "   ├─ title bar  30 · muted · grip 14 · title 12/600 · buttons 22".to_owned(),
      "   ├─ body       padded 4 · rows 2 apart".to_owned(),
      "   └─ footer     22 · the corner's grip 10".to_owned(),
    ];

    let to_title = self.title.clone();
    let controls = vec![
      prop("title", "&str", field(&self.title)),
      divider(),
      flag("media", &self.media),
      flag("collapsible", &self.collapsible),
      flag("resizable", &self.resizable),
      flag("on_dismiss", &self.closable),
    ];
    let resets = (
      to_title,
      self.position.clone(),
      self.size.clone(),
      self.open.clone(),
      self.shown.clone(),
      [self.collapsible.clone(), self.resizable.clone(), self.closable.clone()],
      self.media.clone(),
    );
    let make = plain.clone();
    self.playground.card(
      ctx,
      Spec {
        title: "Floating Panel",
        make: Arc::new(move || Stack::new().width(FILL).child(canvas()).child(make(None)).into()),
        shown,
        fill: !phone(),
        forceable: false,
        facts: format!(
          "at {x}, {y} · {}",
          if self.active.get() { "active" } else { "not active" }
        ),
        event: "on_position_change",
        code,
        tree,
        controls,
        changed: [
          title != "Layers",
          media,
          !collapsible,
          !resizable,
          !closable,
          self.size.get().is_some(),
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          resets.0.set("Layers".to_owned());
          resets.1.set((24.0, 24.0));
          resets.2.set(None);
          resets.3.set(true);
          resets.4.set(true);
          for flag in &resets.5 {
            flag.set(true);
          }
          resets.6.set(false);
        }),
      },
    )
  }
}

impl Component for FloatingPanelPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      title: ctx.signal("Layers".to_owned()),
      position: ctx.signal((24.0, 24.0)),
      size: ctx.signal(None),
      open: ctx.signal(true),
      shown: ctx.signal(true),
      collapsible: ctx.signal(true),
      resizable: ctx.signal(true),
      closable: ctx.signal(true),
      media: ctx.signal(false),
      more: ctx.signal(false),
      header_more: ctx.signal(false),
      active: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let playground = self.play(ctx);
    let several = Stack::new().width(FILL).overflow_visible().child(canvas()).child(
      FloatingPanels::new()
        .panel(
          FloatingPanel::new("Layers")
            .default_position((24.0, 24.0))
            .child(layers()),
        )
        .panel(
          FloatingPanel::new("Properties").default_position((200.0, 80.0)).child(
            Column::new()
              .padding(8.0)
              .spacing(6.0)
              .child(text("Opacity 100%", 13.0, FontWeight::Normal))
              .child(text("Blur 24", 13.0, FontWeight::Normal)),
          ),
        )
        .mount(ctx),
    );
    let header = Row::new().width(FILL).child(
      Column::new().width(240.0).child(
        PanelHeader::new("Explorer")
          .menu(self.header_more.clone(), || {
            [MenuItem::new("Collapse folders"), MenuItem::new("Reveal in files")]
          })
          .on_close(|| ()),
      ),
    );
    let panel_rows = [
      ("new", "&str", "", "A panel called by its title."),
      (
        "child, children",
        "impl Into<Element>",
        "",
        "What it holds, under its title bar.",
      ),
      (
        "position",
        "Signal<(f32, f32)>",
        "its own",
        "Where it stands in its room, which a drag and the keys set. position_controlled and default_position too.",
      ),
      (
        "size",
        "Signal<Option<(f32, f32)>>",
        "its own",
        "None: 240 wide and as high as what it holds. A size makes what it holds scroll. size_controlled, \
         default_size, min_size (160 × 80).",
      ),
      (
        "open",
        "Signal<bool>",
        "its own",
        "Whether it shows what it holds or its title bar alone; its collapse button turns it. default_open.",
      ),
      (
        "on_open_change",
        "Fn(bool)",
        "",
        "Called when the collapse button turns it.",
      ),
      (
        "on_position_change, on_size_change",
        "Fn((f32, f32))",
        "",
        "Called with its new place, its new size, when a person moves or resizes it.",
      ),
      (
        "on_dismiss",
        "Fn()",
        "",
        "Called when its close button or Escape in it closes it; the app takes it away.",
      ),
      (
        "on_active_change",
        "Fn(bool)",
        "",
        "Called when a press in it or the focus coming into it makes it active, and when a press outside it or the \
         focus going outside it makes it not. FloatingPanels has active(Signal<Option<Arc<str>>>) and \
         on_active_change(Fn(Option<&str>)) for the panel of a layer that is active.",
      ),
      (
        "media",
        "impl Into<Element>",
        "",
        "Between the grip and the title: an icon, a swatch.",
      ),
      (
        "button, menu",
        "icon, label, Fn() / Signal<bool>, Fn() -> items",
        "",
        "A button of the title bar; the menu its \"more\" button opens.",
      ),
      (
        "collapsible, resizable, close_button",
        "bool",
        "true",
        "Its collapse button; its edges and corner; its close.",
      ),
      (
        "dismiss_on_escape",
        "bool",
        "true",
        "Escape in it closes it, where it has on_dismiss.",
      ),
      ("label", "&str", "its title", "What it is called to tooling."),
      (
        "move_label, resize_label, collapse_label, more_label, close_label",
        "&str",
        "\"Move\", …",
        "What its grips and buttons are called to tooling.",
      ),
    ];
    let header_rows = [
      ("new", "&str", "", "A header called by its title, written in capitals."),
      ("media", "impl Into<Element>", "", "What stands before the title."),
      ("button", "icon, &str, Fn()", "", "A button at its end."),
      (
        "menu",
        "Signal<bool>, Fn() -> items",
        "",
        "The menu its \"more\" button opens.",
      ),
      (
        "on_close",
        "Fn()",
        "",
        "Shows its close button and is called on a press of it.",
      ),
      ("level", "u32", "2", "The level of the heading it is to tooling."),
    ];
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Desktop", "Floating Panel"],
        title: "Floating Panel",
        description: "A tool window inside the app's window, over its content: layers, a color picker, the \
                      properties of what is chosen. Moved by its title bar, resized by its edges.",
        import: Some("use lurq::ui::{FloatingPanel, FloatingPanels, PanelHeader};"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of FloatingPanel is a control on the right. The panel on the stage is live: drag its title \
           bar, its edges or its corner; it stays on the canvas. From the keyboard its grip and its corner's grip \
           take the arrow keys, by 10 and by 1 with Shift.",
          playground,
        ),
        Section::new(
          "Several panels",
          "FloatingPanels holds the panels over one content: a press anywhere in one brings it to the front and makes \
           it the active one, its title in the foreground while the others' are muted. A press on the canvas leaves \
           none active, and the one in front stays in front.",
          self.frame.demo(
            0,
            // The panels stand where the board has them, on a canvas of its width: a narrower card scrolls it
            // sideways.
            fit("several", self.frame.stage_room(), CANVAS, several),
            "FloatingPanels::new().panel(layers).panel(properties).mount(ctx)",
            &[
              "Stack::new()",
              "    .child(canvas)",
              "    .child(",
              "        FloatingPanels::new()",
              "            .panel(FloatingPanel::new(\"Layers\").default_position((24.0, 24.0)).child(layers))",
              "            .panel(FloatingPanel::new(\"Properties\").default_position((200.0, 80.0)).child(properties))",
              "            .mount(ctx),",
              "    )",
            ],
          ),
        ),
        Section::new(
          "Panel header",
          "The header of a panel that stands on its own in a layout: its title in small capitals, its buttons at its \
           end.",
          self.frame.demo(
            1,
            header,
            "PanelHeader::new(\"Explorer\")",
            &[
              "PanelHeader::new(\"Explorer\")",
              "    .menu(more, || [MenuItem::new(\"Collapse folders\"), MenuItem::new(\"Reveal in files\")])",
              "    .on_close(move || explorer.set(false))",
            ],
          ),
        ),
        Section::new(
          "FloatingPanel props",
          "What a panel is given.",
          props_table(&panel_rows, &["new"], self.frame.room()),
        ),
        Section::new(
          "PanelHeader props",
          "What a header is given.",
          props_table(&header_rows, &["new"], self.frame.room()),
        ),
      ],
    )
  }
}
