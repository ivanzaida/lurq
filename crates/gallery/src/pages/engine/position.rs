//! The page of positions: a box moved from where it is laid out by an offset, which moves nothing around it; a
//! child of a stack placed at a point of its own; and the nine points a stack puts its children at. Its
//! playground places one box in a stack as its props say; its sections are the demo's Positioning page, with a box
//! centred on a point by its own size after them.

use std::sync::Arc;

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{PaletteColor, RadiusSize},
  },
  components::{Column, Grid, Rect, Stack},
  core::Signal,
  layout::StackAlignment,
  node::Element,
};

use super::kit::{sample, samples, tile, tint, well, well_line};
use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, fit, line, phone, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

const POINTS: [(&str, StackAlignment); 9] = [
  ("TopStart", StackAlignment::TopStart),
  ("TopCenter", StackAlignment::TopCenter),
  ("TopEnd", StackAlignment::TopEnd),
  ("CenterStart", StackAlignment::CenterStart),
  ("Center", StackAlignment::Center),
  ("CenterEnd", StackAlignment::CenterEnd),
  ("BottomStart", StackAlignment::BottomStart),
  ("BottomCenter", StackAlignment::BottomCenter),
  ("BottomEnd", StackAlignment::BottomEnd),
];
/// The choices of the playground's select of a point: an index into [`POINTS`], which a signal keeps.
const POINT_CHOICES: [(&str, usize); 9] = [
  ("TopStart", 0),
  ("TopCenter", 1),
  ("TopEnd", 2),
  ("CenterStart", 3),
  ("Center", 4),
  ("CenterEnd", 5),
  ("BottomStart", 6),
  ("BottomCenter", 7),
  ("BottomEnd", 8),
];
/// How the playground's box is placed: where the stack's point puts it, or at a point of its own.
const PLACES: [(&str, usize); 2] = [("stack_align", 0), ("absolute_position", 1)];
const OFFSETS: [(&str, i32); 5] = [("-40", -40), ("0", 0), ("24", 24), ("64", 64), ("160", 160)];
/// The size of the playground's box, and the height of its stack.
const BOX: (f32, f32) = (72.0, 44.0);
const STACK_HEIGHT: f32 = 200.0;

/// Where a box would be laid out with no offset: its bounds dashed.
fn ghost(width: f32, height: f32) -> Rect {
  Rect::new(width, height).rounded(RadiusSize::Sm).border(well_line())
}

/// The box of the playground, as its props say.
#[derive(Clone, Copy)]
struct Placed {
  absolute: bool,
  point: usize,
  x: i32,
  y: i32,
  visible: bool,
}

impl Placed {
  fn build(self) -> Element {
    let (_, point) = POINTS[self.point.min(POINTS.len() - 1)];
    let (x, y) = (self.x as f32, self.y as f32);
    let mut stack = Stack::new()
      .width(FILL)
      .height(STACK_HEIGHT)
      .stack_align(point)
      .rounded(RadiusSize::Sm)
      .background(PaletteColor::Background)
      .border(line());
    stack = if self.absolute {
      stack.child(tile("box", 0, BOX.0, BOX.1).absolute_position(x, y))
    } else {
      stack
        .child(ghost(BOX.0, BOX.1))
        .child(tile("box", 0, BOX.0, BOX.1).relative(x, y))
    };
    if self.visible {
      stack = stack.overflow_visible();
    }
    // The well leaves room around the stack, where what it does not cut off shows.
    well().padding(44.0).overflow_visible().child(stack).into()
  }

  fn code(self) -> Vec<String> {
    let (name, _) = POINTS[self.point.min(POINTS.len() - 1)];
    let mut code = vec!["Stack::new()".to_owned(), format!("    .height({STACK_HEIGHT:.1})")];
    if !self.absolute {
      code.push(format!("    .stack_align(StackAlignment::{name})"));
    }
    if self.visible {
      code.push("    .overflow_visible()".to_owned());
    }
    let (width, height) = BOX;
    if self.absolute {
      code.push(format!(
        "    .child(Rect::new({width:.1}, {height:.1}).absolute_position({}.0, {}.0))",
        self.x, self.y
      ));
    } else if self.x == 0 && self.y == 0 {
      code.push(format!("    .child(Rect::new({width:.1}, {height:.1}))"));
    } else {
      code.push(format!(
        "    .child(Rect::new({width:.1}, {height:.1}).relative({}.0, {}.0))",
        self.x, self.y
      ));
    }
    code
  }
}

pub struct PositionPage {
  frame: Frame,
  playground: Playground,
  /// An index into [`PLACES`].
  place: Signal<usize>,
  /// An index into [`POINTS`].
  point: Signal<usize>,
  x: Signal<i32>,
  y: Signal<i32>,
  visible: Signal<bool>,
}

impl PositionPage {
  fn play(&self, ctx: &mut Ctx) -> Element {
    let placed = Placed {
      absolute: self.place.get() == 1,
      point: self.point.get(),
      x: self.x.get(),
      y: self.y.get(),
      visible: self.visible.get(),
    };
    let tree = if placed.absolute {
      vec![
        format!("stack   {}", self.playground.measured()),
        format!("└─ box   at {}, {}", placed.x, placed.y),
      ]
    } else {
      vec![
        format!("stack   {}", self.playground.measured()),
        "├─ ghost   where the box is laid out".to_owned(),
        format!("└─ box     moved {}, {}", placed.x, placed.y),
      ]
    };
    let (set_place, set_point, set_x, set_y) = (self.place.clone(), self.point.clone(), self.x.clone(), self.y.clone());
    let mut controls = vec![prop(
      "placed by",
      "",
      self
        .playground
        .select(0, &PLACES, &self.place.get(), move |value| set_place.set(value)),
    )];
    // A child placed at a point of its own is not where the stack's point puts it: the point is offered while
    // the stack places the box.
    if !placed.absolute {
      controls.push(prop(
        "stack_align",
        "StackAlignment",
        self
          .playground
          .select(1, &POINT_CHOICES, &placed.point, move |value| set_point.set(value)),
      ));
    }
    let (x_name, y_name) = if placed.absolute {
      ("absolute_position x", "absolute_position y")
    } else {
      ("relative x", "relative y")
    };
    controls.push(prop(
      x_name,
      "f32",
      self
        .playground
        .select(2, &OFFSETS, &placed.x, move |value| set_x.set(value)),
    ));
    controls.push(prop(
      y_name,
      "f32",
      self
        .playground
        .select(3, &OFFSETS, &placed.y, move |value| set_y.set(value)),
    ));
    controls.push(divider());
    controls.push(flag("overflow_visible", &self.visible));
    let reset = {
      let (place, point, x, y, visible) = (
        self.place.clone(),
        self.point.clone(),
        self.x.clone(),
        self.y.clone(),
        self.visible.clone(),
      );
      Arc::new(move || {
        place.set(0);
        point.set(0);
        x.set(24);
        y.set(24);
        visible.set(false);
      })
    };
    self.playground.card(
      ctx,
      Spec {
        title: "Position",
        make: Arc::new(move || placed.build()),
        shown: Shown::built(move |_| placed.build()),
        fill: true,
        forceable: false,
        facts: if placed.absolute {
          "placed at a point of its own".to_owned()
        } else {
          format!("moved {}, {}", placed.x, placed.y)
        },
        event: "",
        code: placed.code(),
        tree,
        controls,
        changed: [
          placed.absolute,
          placed.point != 0,
          placed.x != 24,
          placed.y != 24,
          placed.visible,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset,
      },
    )
  }
}

/// A box moved by 20 and 10 from where its dashed ghost is, in a stack 90 wide that cuts it off at its edge, or
/// that lets it show past it.
fn moved(visible: bool) -> Stack {
  let shifted = tile("A", 0, 80.0, 40.0)
    .absolute_position(0.0, 0.0)
    .relative(20.0, 10.0);
  let mut stack = Stack::new()
    .size(90.0, 98.0)
    .border(well_line())
    .child(ghost(80.0, 40.0).absolute_position(0.0, 0.0))
    .child(shifted)
    .child(tile("B", 1, 80.0, 40.0).absolute_position(0.0, 54.0));
  if visible {
    stack = stack.overflow_visible();
  }
  stack
}

/// The same two boxes with no offset, one under the other, and moved with the stack cutting them off and not.
fn offsets() -> lurq::components::Row {
  let normal = Column::new()
    .width(90.0)
    .spacing(14.0)
    .child(tile("A", 0, 80.0, 40.0))
    .child(tile("B", 1, 80.0, 40.0));
  samples()
    .spacing(40.0)
    .child(sample(normal, "no offset"))
    .child(sample(moved(false), "relative(20.0, 10.0)"))
    .child(sample(
      Column::new().overflow_visible().child(moved(true)),
      "+ overflow_visible()",
    ))
}

/// A stack 400 by 280 with a caption and two boxes at points of their own.
fn absolute() -> Stack {
  Stack::new()
    .size(400.0, 280.0)
    .rounded(RadiusSize::Md)
    .background(PaletteColor::Muted)
    .border(well_line())
    .child(quiet("Stack 400 × 280", 11.0).absolute_position(8.0, 8.0))
    .child(tile("absolute_position(20, 40)", 0, 200.0, 80.0).absolute_position(20.0, 40.0))
    .child(tile("absolute(220, 160)", 1, 0.0, 0.0).absolute(220.0, 160.0, 160.0, 100.0))
}

/// A stack for each of the nine points, with a box at it.
fn points() -> Grid {
  // Three to a row where there is room for the names under them.
  let mut grid = Grid::with_columns(if phone() { 2 } else { 3 }).width(FILL).gap(8.0);
  for (index, (name, point)) in POINTS.into_iter().enumerate() {
    grid = grid.child(sample(
      Stack::new()
        .width(FILL)
        .height(72.0)
        .padding(6.0)
        .stack_align(point)
        .rounded(RadiusSize::Sm)
        .background(PaletteColor::Muted)
        .border(well_line())
        .child(Rect::new(32.0, 22.0).rounded(RadiusSize::Sm).background(tint(index))),
      name,
    ));
  }
  grid
}

/// Three boxes of three sizes, each with its middle on the point where two lines cross.
fn centred() -> Stack {
  let (x, y) = (120.0, 70.0);
  Stack::new()
    .size(240.0, 140.0)
    .rounded(RadiusSize::Md)
    .background(PaletteColor::Muted)
    .border(well_line())
    .child(
      tile("", 0, 112.0, 72.0)
        .absolute_position(x, y)
        .offset_by_size(-0.5, -0.5),
    )
    .child(
      tile("", 1, 64.0, 44.0)
        .absolute_position(x, y)
        .offset_by_size(-0.5, -0.5),
    )
    .child(
      tile("", 2, 20.0, 20.0)
        .absolute_position(x, y)
        .offset_by_size(-0.5, -0.5),
    )
    .child(
      Rect::new(240.0, 1.0)
        .background(PaletteColor::Foreground)
        .absolute_position(0.0, y),
    )
    .child(
      Rect::new(1.0, 140.0)
        .background(PaletteColor::Foreground)
        .absolute_position(x, 0.0),
    )
}

impl Component for PositionPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 4),
      place: ctx.signal(0),
      point: ctx.signal(0),
      x: ctx.signal(24),
      y: ctx.signal(24),
      visible: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let rows = [
      (
        "relative / offset",
        "(f32, f32)",
        "0, 0",
        "Moves the box by so much from where it is laid out. Its parent keeps its room where it was, and nothing \
         around it moves.",
      ),
      (
        "offset_by_size",
        "(f32, f32)",
        "0, 0",
        "Moves the box by shares of its own width and height, once it is laid out: -0.5, -0.5 puts its middle \
         where its corner was.",
      ),
      (
        "absolute_position",
        "(f32, f32)",
        "",
        "On a child of a stack: its corner at a point of the stack, at the size it makes itself. It takes no part \
         in the stack's size.",
      ),
      (
        "absolute",
        "(f32, f32, Dimension, Dimension)",
        "",
        "The same, at a size it is told.",
      ),
      (
        "stack_align",
        "StackAlignment",
        "Center",
        "On a stack: which of its nine points its children stand at.",
      ),
      (
        "overflow_visible",
        "()",
        "",
        "Draws what is in the box past its edge. A box cuts off what is in it unless told.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Engine", "Position"],
        title: "Position",
        description: "Where a box is drawn: moved from where it is laid out, at a point of a stack, or at one of a \
                      stack's nine points.",
        import: Some("use lurq::{components::Stack, layout::StackAlignment};"),
        status: Some("Stable"),
        since: None,
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "A box in a stack whose edge is drawn. Placed by the stack, it stands at the chosen point and is moved \
           from there, its dashed ghost where it is laid out; placed at a point of its own, the numbers are that \
           point. Move it past the edge and let the stack draw what is past it.",
          playground,
        ),
        Section::new(
          "Relative offset",
          "An offset moves a box and nothing else: B stays where it was. A box moved past the edge of its stack is \
           cut off there, unless the stack draws what overflows.",
          self.frame.demo(
            0,
            offsets(),
            ".relative(20.0, 10.0)",
            &[
              "Stack::new()",
              "    .size(90.0, 98.0)",
              "    .child(Rect::new(80.0, 40.0).absolute_position(0.0, 0.0).relative(20.0, 10.0))",
              "    .child(Rect::new(80.0, 40.0).absolute_position(0.0, 54.0))",
              "",
              "// Draws the part of A past its right edge.",
              "Stack::new().size(90.0, 98.0).overflow_visible()",
            ],
          ),
        ),
        Section::new(
          "Absolute positioning",
          "A child of a stack at a point of its own: at the size it makes itself, or at one it is told. Such \
           children take no part in the stack's size, which is told here.",
          self.frame.demo(
            1,
            fit("absolute", self.frame.stage_room(), 400.0, absolute()),
            ".absolute_position(20.0, 40.0)",
            &[
              "Stack::new()",
              "    .size(400.0, 280.0)",
              "    .child(Text::new(\"Stack 400 × 280\").absolute_position(8.0, 8.0))",
              "    .child(Rect::new(200.0, 80.0).absolute_position(20.0, 40.0))",
              "    .child(Rect::new(0.0, 0.0).absolute(220.0, 160.0, 160.0, 100.0))",
            ],
          ),
        ),
        Section::new(
          "Nine points",
          "A stack puts each child at one of nine points of its box, inside its padding.",
          self.frame.demo(
            2,
            points(),
            ".stack_align(StackAlignment::BottomEnd)",
            &[
              "Stack::new()",
              "    .height(72.0)",
              "    .padding(6.0)",
              "    .stack_align(StackAlignment::BottomEnd)",
              "    .child(Rect::new(32.0, 22.0))",
            ],
          ),
        ),
        Section::new(
          "Centred on a point",
          "Boxes of three sizes with their corner at the point where the lines cross, each moved back by half of \
           itself: their middles are on the point, whatever their size.",
          self.frame.demo(
            3,
            centred(),
            ".offset_by_size(-0.5, -0.5)",
            &[
              "Stack::new()",
              "    .size(240.0, 140.0)",
              "    .child(Rect::new(112.0, 72.0).absolute_position(120.0, 70.0).offset_by_size(-0.5, -0.5))",
              "    .child(Rect::new(64.0, 44.0).absolute_position(120.0, 70.0).offset_by_size(-0.5, -0.5))",
              "    .child(Rect::new(20.0, 20.0).absolute_position(120.0, 70.0).offset_by_size(-0.5, -0.5))",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What moves a box, and what a stack is given to place its children.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
