//! The page of the zoom scroll: content of a size the app says, which two fingers pinch larger and smaller, a
//! double tap or a double click goes into and back out of, and a wheel or a drag move about, in a list that
//! scrolls it both ways. Its playground is a plan of tiles with the props as controls and buttons that set the
//! scale from outside; its section is a viewer for a mouse.

use std::sync::Arc;

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{PaletteColor, RadiusSize},
  },
  components::{Column, Grid, Row, ZoomScroll, ZoomScrollProps},
  core::Signal,
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::Element,
  ui::{Button, ButtonSize, ButtonVariant},
};

use super::kit::{controls, tint, well};
use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, mono, props_table, quiet, text},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

/// The size the plan is laid out at, at a scale of 1: larger than the view in any window, so that there is
/// somewhere to pan to before it is brought closer.
const PLAN: (f32, f32) = (960.0, 600.0);
const COLUMNS: usize = 8;
const ROWS: usize = 5;
/// The choices of the playground, scales as hundredths: 0 is none where a prop may be none.
const MINS: [(&str, i32); 3] = [("0.5", 50), ("1", 100), ("2", 200)];
const MAXES: [(&str, i32); 3] = [("2", 200), ("4", 400), ("8", 800)];
const DOUBLE_TAPS: [(&str, i32); 3] = [("None", 0), ("Some(2.0)", 200), ("Some(3.0)", 300)];
const WHEELS: [(&str, i32); 3] = [("None", 0), ("1.25", 125), ("1.5", 150)];
/// How high the view of the content is.
const VIEW: f32 = 300.0;

/// A plan of tiles, eight across and five down, each with its name and a line too small to read until it is
/// brought closer.
fn plan() -> Grid {
  let mut grid = Grid::with_columns(COLUMNS)
    .size(PLAN.0, PLAN.1)
    .gap(6.0)
    .padding(6.0)
    .background(PaletteColor::Muted);
  for row in 0..ROWS {
    for column in 0..COLUMNS {
      let name = format!("{}{}", (b'A' + column as u8) as char, row + 1);
      grid = grid.child(
        Column::new()
          .padding(6.0)
          .spacing(2.0)
          .justify(Justify::SpaceBetween)
          .rounded(RadiusSize::Sm)
          .background(tint(row * COLUMNS + column))
          .child(text(&name, 14.0, FontWeight::Bold).color(PaletteColor::Background))
          .child(
            text(&format!("cell {name}, row {}", row + 1), 6.0, FontWeight::Normal)
              .color(PaletteColor::Background)
              .nowrap(),
          ),
      );
    }
  }
  grid
}

/// Scales as the playground keeps them, in hundredths, as the code writes them.
fn scale(hundredths: i32) -> f32 {
  hundredths as f32 / 100.0
}

/// The zoom scroll of the playground, as its props say.
#[derive(Clone, Copy)]
struct Zoomed {
  min: i32,
  max: i32,
  double_tap: i32,
  wheel: i32,
  drag: bool,
  bars: bool,
}

impl Zoomed {
  fn props(self, zoom: &Signal<f32>) -> ZoomScrollProps {
    let mut props = ZoomScrollProps::new(PLAN.0, PLAN.1)
      .range(scale(self.min), scale(self.max))
      .double_tap((self.double_tap > 0).then(|| scale(self.double_tap)))
      .drag_pan(self.drag)
      .scrollbars(self.bars)
      .zoom(zoom.clone());
    if self.wheel > 0 {
      props = props.wheel_zoom(scale(self.wheel));
    }
    props
  }

  /// The view of the plan, with buttons over it that set the scale from outside.
  fn build(self, ctx: &mut Ctx, zoom: &Signal<f32>) -> Element {
    let (min, max) = (scale(self.min), scale(self.max));
    let step = |label: &str, to: Arc<dyn Fn(f32) -> f32 + Send + Sync>| {
      let zoom = zoom.clone();
      Button::new(label)
        .variant(ButtonVariant::Outline)
        .size(ButtonSize::Sm)
        .on_click(move |_| zoom.set(to(zoom.get_untracked()).clamp(min, max)))
    };
    let view = ZoomScroll::mount(ctx, self.props(zoom), plan());
    Column::new()
      .width(FILL)
      .spacing(12.0)
      .child(
        controls()
          .child(step("Smaller", Arc::new(|zoom| zoom / 1.5)))
          .child(step("Larger", Arc::new(|zoom| zoom * 1.5)))
          .child(step("1 : 1", Arc::new(|_| 1.0)))
          .child(mono(&format!("zoom {:.2}", zoom.get().clamp(min, max)), 12.0).color(PaletteColor::MutedForeground)),
      )
      .child(well().height(VIEW).clip().child(view))
      .into()
  }

  fn code(self) -> Vec<String> {
    let mut code = vec![
      format!("let props = ZoomScrollProps::new({:.1}, {:.1})", PLAN.0, PLAN.1),
      format!("    .range({}, {})", scale(self.min), scale(self.max)),
    ];
    if self.double_tap != 200 {
      code.push(match self.double_tap {
        0 => "    .double_tap(None)".to_owned(),
        value => format!("    .double_tap(Some({:.1}))", scale(value)),
      });
    }
    if self.wheel > 0 {
      code.push(format!("    .wheel_zoom({})", scale(self.wheel)));
    }
    if self.drag {
      code.push("    .drag_pan(true)".to_owned());
    }
    if !self.bars {
      code.push("    .scrollbars(false)".to_owned());
    }
    code.push("    .zoom(zoom.clone());".to_owned());
    code.push(String::new());
    code.push("ZoomScroll::mount(ctx, props, plan)".to_owned());
    code
  }
}

pub struct ZoomScrollPage {
  frame: Frame,
  playground: Playground,
  /// Scales in hundredths; 0 for none.
  min: Signal<i32>,
  max: Signal<i32>,
  double_tap: Signal<i32>,
  wheel: Signal<i32>,
  drag: Signal<bool>,
  bars: Signal<bool>,
  /// The scale the plan of the playground is shown at, and that of the viewer.
  zoom: Signal<f32>,
  viewer_zoom: Signal<f32>,
}

impl ZoomScrollPage {
  fn play(&self, ctx: &mut Ctx) -> Element {
    let zoomed = Zoomed {
      min: self.min.get(),
      max: self.max.get(),
      double_tap: self.double_tap.get(),
      wheel: self.wheel.get(),
      drag: self.drag.get(),
      bars: self.bars.get(),
    };
    {
      let zoom = self.zoom.clone();
      self.playground.window_shows(move |ctx| zoomed.build(ctx, &zoom));
    }
    let tree = vec![
      format!("zoom scroll   {}", self.playground.measured()),
      "└─ scroll both".to_owned(),
      "   └─ stack   as large as the plan is shown".to_owned(),
      format!("      └─ plan   {} × {}, scaled", PLAN.0, PLAN.1),
    ];
    let (set_min, set_max, set_double_tap, set_wheel) = (
      self.min.clone(),
      self.max.clone(),
      self.double_tap.clone(),
      self.wheel.clone(),
    );
    let controls = vec![
      prop(
        "range min",
        "f32",
        self
          .playground
          .select(0, &MINS, &zoomed.min, move |value| set_min.set(value)),
      ),
      prop(
        "range max",
        "f32",
        self
          .playground
          .select(1, &MAXES, &zoomed.max, move |value| set_max.set(value)),
      ),
      prop(
        "double_tap",
        "Option<f32>",
        self
          .playground
          .select(2, &DOUBLE_TAPS, &zoomed.double_tap, move |value| {
            set_double_tap.set(value)
          }),
      ),
      prop(
        "wheel_zoom",
        "f32",
        self
          .playground
          .select(3, &WHEELS, &zoomed.wheel, move |value| set_wheel.set(value)),
      ),
      divider(),
      flag("drag_pan", &self.drag),
      flag("scrollbars", &self.bars),
    ];
    let reset = {
      let (min, max, double_tap, wheel, drag, bars, zoom) = (
        self.min.clone(),
        self.max.clone(),
        self.double_tap.clone(),
        self.wheel.clone(),
        self.drag.clone(),
        self.bars.clone(),
        self.zoom.clone(),
      );
      Arc::new(move || {
        min.set(100);
        max.set(400);
        double_tap.set(200);
        wheel.set(0);
        drag.set(false);
        bars.set(true);
        zoom.set(1.0);
      })
    };
    let zoom = self.zoom.clone();
    self.playground.card(
      ctx,
      Spec {
        title: "Zoom Scroll",
        make: Arc::new(|| Column::new().into()),
        shown: Shown::built(move |ctx| zoomed.build(ctx, &zoom)),
        fill: true,
        forceable: false,
        facts: format!(
          "zoom {:.2}",
          self.zoom.get().clamp(scale(zoomed.min), scale(zoomed.max))
        ),
        event: "zoom",
        code: zoomed.code(),
        tree,
        controls,
        changed: [
          zoomed.min != 100,
          zoomed.max != 400,
          zoomed.double_tap != 200,
          zoomed.wheel != 0,
          zoomed.drag,
          !zoomed.bars,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset,
      },
    )
  }

  /// The plan as a viewer for a mouse shows a picture: the wheel brings it closer about the pointer, a drag
  /// carries it, and no bars.
  fn viewer(&self, ctx: &mut Ctx) -> Column {
    let props = ZoomScrollProps::new(PLAN.0, PLAN.1)
      .range(1.0, 8.0)
      .wheel_zoom(1.5)
      .drag_pan(true)
      .scrollbars(false)
      .zoom(self.viewer_zoom.clone());
    let view = ZoomScroll::mount(ctx, props, plan());
    Column::new()
      .width(FILL)
      .spacing(12.0)
      .child(Row::new().width(FILL).align_items(Alignment::Center).child(quiet(
        &format!("zoom {:.2}", self.viewer_zoom.get().clamp(1.0, 8.0)),
        12.0,
      )))
      .child(well().height(VIEW).clip().child(view))
  }
}

impl Component for ZoomScrollPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let playground = Playground::new(ctx, 4);
    let zoom = ctx.signal(1.0);
    playground.watch(ctx, &zoom, |zoom| format!("zoom {zoom:.2}"));
    Self {
      frame: Frame::new(ctx),
      playground,
      min: ctx.signal(100),
      max: ctx.signal(400),
      double_tap: ctx.signal(200),
      wheel: ctx.signal(0),
      drag: ctx.signal(false),
      bars: ctx.signal(true),
      zoom,
      viewer_zoom: ctx.signal(1.0),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let rows = [
      (
        "ZoomScrollProps::new",
        "(f32, f32)",
        "",
        "The size the content is laid out at, at a scale of 1. It is laid out once and drawn scaled: its text is \
         not laid out again at each scale.",
      ),
      (
        "range",
        "(f32, f32)",
        "1, 4",
        "The least and the most scale. A picture larger than the view has a least below 1, the scale it fits at.",
      ),
      (
        "double_tap",
        "Option<f32>",
        "Some(2.0)",
        "The scale a double tap or a double click goes to from the least, as a share of it, and back. None \
         leaves them to the content.",
      ),
      (
        "wheel_zoom",
        "f32",
        "",
        "The wheel brings the content closer about the pointer, by so much a notch, in place of scrolling it: \
         away from the hand closer, toward it further. Ctrl and the wheel, and a touchpad's pinch, zoom without \
         it too; a touchpad's two fingers scroll either way.",
      ),
      (
        "drag_pan",
        "bool",
        "false",
        "A press held and moved carries the content, as a finger does.",
      ),
      (
        "scrollbars",
        "bool",
        "true",
        "The bars, where the content is larger than the view.",
      ),
      (
        "zoom",
        "Signal<f32>",
        "",
        "The scale, to read and to set: one set from outside keeps the middle of the view where it is.",
      ),
      (
        "scroll",
        "ScrollState",
        "",
        "The state of the list that scrolls the content, to read where it is and to scroll it.",
      ),
      (
        "ZoomScroll::mount",
        "(&mut Ctx, ZoomScrollProps, content)",
        "",
        "The zoom scroll around the content. It fills what it is put in.",
      ),
    ];
    let playground = self.play(ctx);
    let viewer = self.viewer(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Engine", "Zoom Scroll"],
        title: "Zoom Scroll",
        description: "Content two fingers pinch larger and smaller, in a list that scrolls it both ways: a photo, \
                      a map or a page looked at closely. The pinch keeps the point between the fingers under them.",
        import: Some("use lurq::components::{ZoomScroll, ZoomScrollProps};"),
        status: Some("Stable"),
        since: None,
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "A plan of forty tiles, 960 by 600, larger than its view. Double-click a tile to go into it and again to \
           come back out; on a touch screen or a touchpad, pinch it, or turn the wheel with Ctrl held. With \
           wheel_zoom the wheel alone brings it closer about the pointer, turned away from you; with drag_pan a \
           drag carries it. The buttons set the scale from outside, about the middle of the view.",
          playground,
        ),
        Section::new(
          "A viewer for a mouse",
          "As a viewer of pictures has it: the wheel brings the plan closer about the pointer and a drag carries \
           it, up to eight times, with no bars over it.",
          self.frame.demo(
            0,
            viewer,
            ".wheel_zoom(1.5).drag_pan(true)",
            &[
              "let props = ZoomScrollProps::new(960.0, 600.0)",
              "    .range(1.0, 8.0)",
              "    .wheel_zoom(1.5)",
              "    .drag_pan(true)",
              "    .scrollbars(false)",
              "    .zoom(zoom.clone());",
              "",
              "Column::new().height(300.0).child(ZoomScroll::mount(ctx, props, plan))",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a zoom scroll is given.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
