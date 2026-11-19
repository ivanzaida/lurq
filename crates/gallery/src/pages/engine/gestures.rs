//! The page of the gestures of fingers: a square that two fingers pinch larger and smaller and turn about their
//! middle, that a finger drags, taps, taps twice and holds, and a line for each gesture with what it told last.
//! What the phone gives is tried where it is shown.

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    events::{GesturePhase, LongPressEvent, PanEvent, PinchEvent, RotateEvent, TapEvent},
    theme::{PaletteColor, RadiusSize},
  },
  components::{Column, Row, Stack},
  core::Signal,
  layout::{Alignment, StackAlignment, layout_kind::Justify, text_style::FontWeight},
  node::{Element, transform::Transform2D},
  ui::{Button, ButtonSize, ButtonVariant},
};

use super::kit::{controls, tint, well};
use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, mono, props_table, quiet, text},
};

/// The side of the square, at a scale of 1.
const SIDE: f32 = 120.0;
/// How high the pad the fingers move in is.
const PAD: f32 = 360.0;
/// The least and the most the square is scaled to.
const SCALES: (f32, f32) = (0.25, 6.0);

/// The gestures the page hears, in the order of their lines.
#[derive(Clone, Copy, PartialEq, Eq, lurq::DevtoolsInspectable)]
enum Gesture {
  Pinch,
  Rotate,
  Pan,
  Tap,
  DoubleTap,
  LongPress,
}

const GESTURES: [Gesture; 6] = [
  Gesture::Pinch,
  Gesture::Rotate,
  Gesture::Pan,
  Gesture::Tap,
  Gesture::DoubleTap,
  Gesture::LongPress,
];

impl Gesture {
  fn handler(self) -> &'static str {
    match self {
      Gesture::Pinch => "on_pinch",
      Gesture::Rotate => "on_rotate",
      Gesture::Pan => "on_pan",
      Gesture::Tap => "on_tap",
      Gesture::DoubleTap => "on_double_tap",
      Gesture::LongPress => "on_long_press",
    }
  }

  fn index(self) -> usize {
    GESTURES.iter().position(|gesture| *gesture == self).unwrap_or(0)
  }
}

/// Where the square is: what the gestures did to it.
#[derive(Clone, Copy, PartialEq, lurq::DevtoolsInspectable)]
struct Pose {
  scale: f32,
  /// Clockwise, in radians.
  angle: f32,
  x: f32,
  y: f32,
  /// The tint a tap moves on.
  tint: usize,
}

impl Pose {
  const REST: Pose = Pose {
    scale: 1.0,
    angle: 0.0,
    x: 0.0,
    y: 0.0,
    tint: 0,
  };

  /// Scaled and turned about its middle, then moved.
  fn transform(self) -> Transform2D {
    Transform2D::translate(self.x, self.y)
      .then(&Transform2D::rotate(self.angle))
      .then(&Transform2D::scale_uniform(self.scale))
  }
}

fn phase(phase: GesturePhase) -> &'static str {
  match phase {
    GesturePhase::Start => "start",
    GesturePhase::Change => "change",
    GesturePhase::End => "end",
  }
}

/// What the page heard: a line for each gesture, and which came last.
#[derive(Clone)]
struct Heard {
  lines: Signal<Vec<String>>,
  last: Signal<Option<Gesture>>,
}

impl Heard {
  fn tell(&self, gesture: Gesture, line: String) {
    self.lines.update(|lines| lines[gesture.index()] = line);
    self.last.set(Some(gesture));
  }
}

pub struct GesturesPage {
  frame: Frame,
  pose: Signal<Pose>,
  heard: Heard,
}

impl GesturesPage {
  /// The pad the fingers move in, with the square in its middle that follows them.
  fn pad(&self) -> Element {
    let pose = self.pose.get();
    let square = Row::new()
      .size(SIDE, SIDE)
      .align_items(Alignment::Center)
      .justify(Justify::Center)
      .rounded(RadiusSize::Lg)
      .background(tint(pose.tint))
      .transform(pose.transform())
      .child(
        text("lurq", 18.0, FontWeight::Bold)
          .color(PaletteColor::Background)
          .nowrap(),
      );
    let (pinch, rotate, pan, tap, double_tap, long_press) = (
      (self.pose.clone(), self.heard.clone()),
      (self.pose.clone(), self.heard.clone()),
      (self.pose.clone(), self.heard.clone()),
      (self.pose.clone(), self.heard.clone()),
      (self.pose.clone(), self.heard.clone()),
      self.heard.clone(),
    );
    well()
      .height(PAD)
      .clip()
      .id("gestures-pad")
      .child(
        Stack::new()
          .size(FILL, FILL)
          .stack_align(StackAlignment::Center)
          .child(square),
      )
      .on_pinch(move |event: PinchEvent| {
        let (pose, heard) = &pinch;
        pose.update(|pose| pose.scale = (pose.scale * event.delta).clamp(SCALES.0, SCALES.1));
        heard.tell(
          Gesture::Pinch,
          format!(
            "{}  scale {:.2}  delta {:.3}  at {:.0}, {:.0}",
            phase(event.phase),
            event.scale,
            event.delta,
            event.x,
            event.y
          ),
        );
      })
      .on_rotate(move |event: RotateEvent| {
        let (pose, heard) = &rotate;
        pose.update(|pose| pose.angle += event.delta);
        heard.tell(
          Gesture::Rotate,
          format!(
            "{}  {:.1}°  delta {:.2}°  at {:.0}, {:.0}",
            phase(event.phase),
            event.rotation.to_degrees(),
            event.delta.to_degrees(),
            event.x,
            event.y
          ),
        );
      })
      .on_pan(move |event: PanEvent| {
        let (pose, heard) = &pan;
        pose.update(|pose| {
          pose.x += event.delta_x;
          pose.y += event.delta_y;
        });
        heard.tell(
          Gesture::Pan,
          format!(
            "{}  {} finger{}  moved {:.0}, {:.0}  speed {:.0}, {:.0} pt/s",
            phase(event.phase),
            event.fingers,
            if event.fingers == 1 { "" } else { "s" },
            event.translation_x,
            event.translation_y,
            event.velocity_x,
            event.velocity_y
          ),
        );
      })
      .on_tap(move |event: TapEvent| {
        let (pose, heard) = &tap;
        if event.fingers == 1 && event.count == 1 {
          pose.update(|pose| pose.tint += 1);
        }
        heard.tell(
          Gesture::Tap,
          format!(
            "{} finger{}  tap {}  at {:.0}, {:.0}",
            event.fingers,
            if event.fingers == 1 { "" } else { "s" },
            event.count,
            event.x,
            event.y
          ),
        );
      })
      .on_double_tap(move |event: TapEvent| {
        let (pose, heard) = &double_tap;
        // As a picture is gone into and back out of: twice as large, or as it was.
        pose.update(|pose| pose.scale = if pose.scale > 1.01 { 1.0 } else { 2.0 });
        heard.tell(
          Gesture::DoubleTap,
          format!(
            "{} finger{}  at {:.0}, {:.0}",
            event.fingers,
            if event.fingers == 1 { "" } else { "s" },
            event.x,
            event.y
          ),
        );
      })
      .on_long_press(move |event: LongPressEvent| {
        long_press.tell(Gesture::LongPress, format!("at {:.0}, {:.0}", event.x, event.y));
      })
      .into()
  }

  /// The square where it is, a Reset, and a line for each gesture, the last one in the foreground.
  fn told(&self) -> Column {
    let pose = self.pose.get();
    let reset = {
      let pose = self.pose.clone();
      Button::new("Reset")
        .variant(ButtonVariant::Outline)
        .size(ButtonSize::Sm)
        .on_click(move |_| pose.set(Pose::REST))
    };
    let (lines, last) = (self.heard.lines.get(), self.heard.last.get());
    let mut told = Column::new().width(FILL).spacing(6.0);
    for gesture in GESTURES {
      let line = &lines[gesture.index()];
      let shown = if last == Some(gesture) {
        mono(line, 12.0).color(PaletteColor::Foreground)
      } else {
        quiet(line, 12.0)
      };
      told = told.child(
        Row::new()
          .width(FILL)
          .spacing(12.0)
          .align_items(Alignment::Center)
          .child(mono(gesture.handler(), 12.0).width(110.0).flex_shrink(0.0))
          .child(Column::new().flex(1.0).min_width(0.0).child(shown.width(FILL))),
      );
    }
    Column::new()
      .width(FILL)
      .spacing(12.0)
      .child(controls().child(reset).child(quiet(
        &format!(
          "scale {:.2}  angle {:.1}°  moved {:.0}, {:.0}",
          pose.scale,
          pose.angle.to_degrees(),
          pose.x,
          pose.y
        ),
        12.0,
      )))
      .child(told)
  }
}

impl Component for GesturesPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      pose: ctx.signal(Pose::REST),
      heard: Heard {
        lines: ctx.signal(vec!["—".to_owned(); GESTURES.len()]),
        last: ctx.signal(None),
      },
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let rows = [
      (
        "on_pinch",
        "PinchEvent",
        "",
        "Two fingers apart or together: scale since the start, delta since the event before (multiply by it), the \
         middle of the fingers, and the phase.",
      ),
      (
        "on_rotate",
        "RotateEvent",
        "",
        "Two fingers turning about their middle: rotation since the start and delta since the event before, in \
         radians, clockwise.",
      ),
      (
        "on_pan",
        "PanEvent",
        "",
        "Fingers moving: how many, the translation since they landed, the delta since the event before, and how \
         fast.",
      ),
      (
        "on_tap",
        "TapEvent",
        "",
        "Fingers that touched and lifted without travelling: how many, and which tap of a row it is.",
      ),
      (
        "on_double_tap",
        "TapEvent",
        "",
        "The second of two taps in a row. An element that listens for it holds its single click back.",
      ),
      ("on_long_press", "LongPressEvent", "", "A finger held still: where."),
    ];
    let pad = Column::new()
      .width(FILL)
      .spacing(16.0)
      .child(self.pad())
      .child(self.told());
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Engine", "Gestures"],
        title: "Gestures",
        description: "What fingers do on an element, told as gestures: two fingers pinch and turn, a finger \
                      drags, taps, taps twice and holds. Each handler is told on the element the fingers landed \
                      on, and goes up from it.",
        import: Some("use lurq::app::events::{PinchEvent, RotateEvent, PanEvent, TapEvent, LongPressEvent};"),
        status: Some("Stable"),
        since: None,
        source: file!(),
      },
      vec![
        Section::new(
          "Pinch, turn and drag",
          "On a phone or a touch screen: pinch the square with two fingers to scale it, turn them to turn it, drag \
           it with one. A tap gives it another color, a double tap doubles it or brings it back, and a long press \
           is only told. Each gesture's line says what it told last; the one that came last is in the foreground.",
          self.frame.demo(
            0,
            pad,
            ".on_pinch(..).on_rotate(..).on_pan(..)",
            &[
              "Column::new()",
              "    .child(square.transform(",
              "        Transform2D::translate(x, y)",
              "            .then(&Transform2D::rotate(angle))",
              "            .then(&Transform2D::scale_uniform(scale)),",
              "    ))",
              "    .on_pinch(move |event: PinchEvent| scale.update(|s| *s *= event.delta))",
              "    .on_rotate(move |event: RotateEvent| angle.update(|a| *a += event.delta))",
              "    .on_pan(move |event: PanEvent| at.update(|(x, y)| {",
              "        *x += event.delta_x;",
              "        *y += event.delta_y;",
              "    }))",
            ],
          ),
        ),
        Section::new(
          "Handlers",
          "What an element listens with, and what each is told.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
