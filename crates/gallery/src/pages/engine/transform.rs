//! The page of transforms: an element turned, scaled, skewed or moved where it is painted, with all that is in it,
//! while layout keeps its slot where it was. Its playground transforms a tile as its props say; its sections are the
//! demo's Transforms page, with the slot that stays and text kept sharp after them.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx, theme::PaletteColor},
  components::{Column, Rect, Row, Stack, Text},
  core::Signal,
  layout::{Alignment, StackAlignment, layout_kind::Justify},
  node::{CursorIcon, Element, TextTransformMode, transform::Transform2D},
};

use super::{
  borders::slide,
  kit::{sample, samples, tile, well, well_line},
};
use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Shown, Spec},
};

/// How large the playground's tile is.
const TILE: (f32, f32) = (120.0, 64.0);
/// How large the cell a sample stands in the middle of is: room for what a transform paints past the sample.
const CELL: (f32, f32) = (120.0, 110.0);

/// The props of the playground's transform, in the order a person reads them.
#[derive(Clone, Copy, PartialEq)]
struct Pose {
  rotate: i32,
  /// The scale along each axis, in percent.
  scale_x: i32,
  scale_y: i32,
  /// The skew along each axis, in degrees.
  skew_x: i32,
  skew_y: i32,
  move_x: i32,
  move_y: i32,
}

impl Pose {
  const REST: Self = Self {
    rotate: 0,
    scale_x: 100,
    scale_y: 100,
    skew_x: 0,
    skew_y: 0,
    move_x: 0,
    move_y: 0,
  };

  /// The transforms that are not the identity, each with its Rust, the first applied last: moved, turned,
  /// scaled, then skewed, as CSS reads `translate rotate scale skew`.
  fn parts(self) -> Vec<(Transform2D, String)> {
    let mut parts = Vec::new();
    if self.move_x != 0 || self.move_y != 0 {
      parts.push((
        Transform2D::translate(self.move_x as f32, self.move_y as f32),
        format!("Transform2D::translate({}.0, {}.0)", self.move_x, self.move_y),
      ));
    }
    if self.rotate != 0 {
      parts.push((
        Transform2D::rotate_deg(self.rotate as f32),
        format!("Transform2D::rotate_deg({}.0)", self.rotate),
      ));
    }
    if self.scale_x != 100 || self.scale_y != 100 {
      let (x, y) = (self.scale_x as f32 / 100.0, self.scale_y as f32 / 100.0);
      parts.push((Transform2D::scale(x, y), format!("Transform2D::scale({x:?}, {y:?})")));
    }
    if self.skew_x != 0 || self.skew_y != 0 {
      parts.push((
        Transform2D::skew((self.skew_x as f32).to_radians(), (self.skew_y as f32).to_radians()),
        format!(
          "Transform2D::skew({}f32.to_radians(), {}f32.to_radians())",
          self.skew_x, self.skew_y
        ),
      ));
    }
    parts
  }

  fn transform(self) -> Option<Transform2D> {
    self
      .parts()
      .into_iter()
      .map(|(transform, _)| transform)
      .reduce(|all, next| all.then(&next))
  }
}

pub struct TransformPage {
  frame: Frame,
  playground: Playground,
  rotate: Signal<i32>,
  scale_x: Signal<i32>,
  scale_y: Signal<i32>,
  skew_x: Signal<i32>,
  skew_y: Signal<i32>,
  move_x: Signal<i32>,
  move_y: Signal<i32>,
}

impl TransformPage {
  fn pose(&self) -> Pose {
    Pose {
      rotate: self.rotate.get(),
      scale_x: self.scale_x.get(),
      scale_y: self.scale_y.get(),
      skew_x: self.skew_x.get(),
      skew_y: self.skew_y.get(),
      move_x: self.move_x.get(),
      move_y: self.move_y.get(),
    }
  }

  fn play(&self, ctx: &mut Ctx) -> Element {
    let pose = self.pose();
    let record = self.playground.recorder();
    let build = move || -> Element {
      let record = record.clone();
      let mut shown = tile("Hello", 0, TILE.0, TILE.1)
        .cursor(CursorIcon::Pointer)
        .hovered(|style| style.background(PaletteColor::Primary))
        .on_click(move |_| record("on_click".to_owned()));
      if let Some(transform) = pose.transform() {
        shown = shown.transform(transform);
      }
      // The slot layout gives the tile, dashed under it: the transform moves what is painted, not the slot.
      Stack::new()
        .size(TILE.0, TILE.1)
        .overflow_visible()
        .child(
          Rect::new(FILL, FILL)
            .rounded(lurq::app::theme::RadiusSize::Sm)
            .border(well_line()),
        )
        .child(shown)
        .into()
    };

    let parts = pose.parts();
    let mut code = vec![
      "Row::new()".to_owned(),
      format!("    .size({}.0, {}.0)", TILE.0, TILE.1),
      "    .on_click(|_| ..)".to_owned(),
    ];
    match parts.as_slice() {
      [] => {}
      [(_, only)] => code.push(format!("    .transform({only})")),
      [(_, first), rest @ ..] => {
        code.push("    .transform(".to_owned());
        code.push(format!("        {first}"));
        for (_, next) in rest {
          code.push(format!("            .then(&{next})"));
        }
        code.push("    )".to_owned());
      }
    }
    code.push("    .child(Text::new(\"Hello\"))".to_owned());
    let tree = vec![
      format!("row   {}", self.playground.measured()),
      format!(
        "│  transform: {}",
        if parts.is_empty() {
          "none".to_owned()
        } else {
          format!("{} parts", parts.len())
        }
      ),
      "└─ text \"Hello\"".to_owned(),
    ];

    let reset = {
      let signals = [
        (self.rotate.clone(), Pose::REST.rotate),
        (self.scale_x.clone(), Pose::REST.scale_x),
        (self.scale_y.clone(), Pose::REST.scale_y),
        (self.skew_x.clone(), Pose::REST.skew_x),
        (self.skew_y.clone(), Pose::REST.skew_y),
        (self.move_x.clone(), Pose::REST.move_x),
        (self.move_y.clone(), Pose::REST.move_y),
      ];
      Arc::new(move || {
        for (signal, rest) in &signals {
          signal.set(*rest);
        }
      })
    };
    let controls = vec![
      slide("rotate_deg", "f32", &self.rotate, -180, 180),
      slide("scale x, %", "f32", &self.scale_x, 20, 200),
      slide("scale y, %", "f32", &self.scale_y, 20, 200),
      slide("skew x, °", "f32", &self.skew_x, -45, 45),
      slide("skew y, °", "f32", &self.skew_y, -45, 45),
      slide("translate x", "f32", &self.move_x, -60, 60),
      slide("translate y", "f32", &self.move_y, -60, 60),
    ];
    let changed = [
      pose.rotate != Pose::REST.rotate,
      pose.scale_x != Pose::REST.scale_x,
      pose.scale_y != Pose::REST.scale_y,
      pose.skew_x != Pose::REST.skew_x,
      pose.skew_y != Pose::REST.skew_y,
      pose.move_x != Pose::REST.move_x,
      pose.move_y != Pose::REST.move_y,
    ]
    .into_iter()
    .filter(|changed| *changed)
    .count();
    self.playground.card(
      ctx,
      Spec {
        title: "Transform",
        make: Arc::new(build.clone()),
        shown: Shown::built(move |_| build()),
        fill: false,
        forceable: false,
        facts: "the slot is dashed".to_owned(),
        event: "on_click",
        code,
        tree,
        controls,
        changed,
        reset,
      },
    )
  }
}

/// A sample in the middle of a cell with room round it for what its transform paints, with what it is under it.
fn cell(caption: &str, shown: impl Into<Element>) -> Column {
  Column::new()
    .spacing(12.0)
    .align_items(Alignment::Center)
    .overflow_visible()
    .child(
      Row::new()
        .size(CELL.0, CELL.1)
        .align_items(Alignment::Center)
        .justify(Justify::Center)
        .overflow_visible()
        .child(shown),
    )
    .child(quiet(caption, 12.0))
}

/// Cells side by side; what does not fit goes on to the next line.
fn cells(all: Vec<Column>) -> Row {
  samples().spacing(16.0).overflow_visible().with_children(all)
}

/// A square of the `index`th tint, 60 on a side.
fn square(index: usize) -> Row {
  tile("", index, 60.0, 60.0)
}

/// A tile of the `index`th tint with a word in it, 100 by 50.
fn worded(index: usize, transform: Transform2D) -> Row {
  tile("Hello", index, 100.0, 50.0).transform(transform)
}

fn rotated() -> Row {
  cells(
    [0.0, 15.0, 30.0, 45.0, 90.0, 180.0]
      .into_iter()
      .map(|degrees: f32| {
        cell(
          &format!("{degrees}°"),
          square(0).transform(Transform2D::rotate_deg(degrees)),
        )
      })
      .collect(),
  )
}

fn scaled() -> Row {
  cells(
    [
      ("0.5", 0.5, 0.5),
      ("0.75", 0.75, 0.75),
      ("1.0", 1.0, 1.0),
      ("1.5", 1.5, 1.5),
      ("1.4 × 0.7", 1.4, 0.7),
      ("0.6 × 1.4", 0.6, 1.4),
    ]
    .into_iter()
    .map(|(caption, x, y)| cell(caption, square(1).transform(Transform2D::scale(x, y))))
    .collect(),
  )
}

fn skewed() -> Row {
  cells(
    [
      ("x 10°", 10.0, 0.0),
      ("x 20°", 20.0, 0.0),
      ("x -20°", -20.0, 0.0),
      ("y 10°", 0.0, 10.0),
      ("y 20°", 0.0, 20.0),
      ("x and y 10°", 10.0, 10.0),
    ]
    .into_iter()
    .map(|(caption, x, y): (&str, f32, f32)| {
      cell(
        caption,
        square(2).transform(Transform2D::skew(x.to_radians(), y.to_radians())),
      )
    })
    .collect(),
  )
}

fn composed() -> Row {
  cells(vec![
    cell(
      "rotate, then scale",
      square(0).transform(Transform2D::rotate_deg(30.0).then(&Transform2D::scale(1.3, 0.8))),
    ),
    cell(
      "scale, then rotate",
      square(3).transform(Transform2D::scale(1.3, 0.8).then(&Transform2D::rotate_deg(30.0))),
    ),
    cell(
      "rotate, then skew",
      square(1).transform(Transform2D::rotate_deg(15.0).then(&Transform2D::skew(0.3, 0.0))),
    ),
    cell(
      "scale, then skew",
      square(4).transform(Transform2D::scale(1.2, 0.8).then(&Transform2D::skew(0.2, 0.0))),
    ),
    cell(
      "all three",
      square(2).transform(
        Transform2D::rotate_deg(20.0)
          .then(&Transform2D::scale(1.2, 0.8))
          .then(&Transform2D::skew(0.15, 0.0)),
      ),
    ),
  ])
}

fn worded_all() -> Row {
  cells(vec![
    cell("rotate 15°", worded(0, Transform2D::rotate_deg(15.0))),
    cell("rotate 45°", worded(1, Transform2D::rotate_deg(45.0))),
    cell("scale 1.3", worded(2, Transform2D::scale_uniform(1.3))),
    cell("skew x", worded(4, Transform2D::skew(0.3, 0.0))),
    cell(
      "rotate and scale",
      worded(3, Transform2D::rotate_deg(20.0).then(&Transform2D::scale(1.2, 0.9))),
    ),
  ])
}

/// Three tiles in a row, the middle one turned and grown: its neighbours stay where layout put them.
fn slot() -> Column {
  let middle = Stack::new()
    .size(72.0, 48.0)
    .stack_align(StackAlignment::Center)
    .overflow_visible()
    .child(
      Rect::new(FILL, FILL)
        .rounded(lurq::app::theme::RadiusSize::Sm)
        .border(well_line()),
    )
    .child(tile("B", 1, 72.0, 48.0).transform(Transform2D::rotate_deg(30.0).then(&Transform2D::scale_uniform(1.3))));
  well().overflow_visible().child(
    Row::new()
      .width(FILL)
      .padding(28.0)
      .wrap()
      .spacing(12.0)
      .align_items(Alignment::Center)
      .overflow_visible()
      .child(tile("A", 0, 72.0, 48.0))
      .child(middle)
      .child(tile("C", 2, 72.0, 48.0)),
  )
}

/// A label turned a little, drawn from the glyphs as they are and from glyphs rasterized at the angle.
fn sharp() -> Row {
  let label = |mode: TextTransformMode| {
    let theme = lurq::ui::current_theme();
    let mut style = theme.typography_style(lurq::app::theme::TypographyStyle::Body);
    style.font_size = theme.scaled(18.0);
    Text::styled("A static label, turned", style)
      .text_transform_mode(mode)
      .transform(Transform2D::rotate_deg(-8.0))
  };
  samples()
    .spacing(40.0)
    .padding_vertical(16.0)
    .overflow_visible()
    .child(
      sample(
        label(TextTransformMode::Bitmap).overflow_visible(),
        "TextTransformMode::Bitmap",
      )
      .overflow_visible(),
    )
    .child(
      sample(
        label(TextTransformMode::Rasterized).overflow_visible(),
        "TextTransformMode::Rasterized",
      )
      .overflow_visible(),
    )
}

impl Component for TransformPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      rotate: ctx.signal(Pose::REST.rotate),
      scale_x: ctx.signal(Pose::REST.scale_x),
      scale_y: ctx.signal(Pose::REST.scale_y),
      skew_x: ctx.signal(Pose::REST.skew_x),
      skew_y: ctx.signal(Pose::REST.skew_y),
      move_x: ctx.signal(Pose::REST.move_x),
      move_y: ctx.signal(Pose::REST.move_y),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let rows = [
      (
        "transform",
        "Transform2D",
        "identity",
        "Turns, scales, skews or moves what the element paints, with all that is in it, about its middle. Layout \
         keeps the element's slot; the pointer finds it where it is painted.",
      ),
      (
        "Transform2D::rotate_deg",
        "f32",
        "",
        "A turn by degrees, clockwise; Transform2D::rotate takes radians.",
      ),
      (
        "Transform2D::scale",
        "(f32, f32)",
        "",
        "A scale along each axis; Transform2D::scale_uniform scales both by one factor.",
      ),
      (
        "Transform2D::skew",
        "(f32, f32)",
        "",
        "A shear along x and along y, by angles in radians.",
      ),
      (
        "Transform2D::translate",
        "(f32, f32)",
        "",
        "A move by pixels, where it is painted only.",
      ),
      (
        "then",
        "&Transform2D",
        "",
        "Composes two transforms: a.then(&b) applies b first, then a.",
      ),
      (
        "text_transform_mode",
        "TextTransformMode",
        "Bitmap",
        "On a text: Bitmap moves the glyphs as they are, for text that moves; Rasterized draws them at the angle, \
         sharper, for text that stays turned.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Engine", "Transform"],
        title: "Transform",
        description: "A transform turns, scales, skews or moves what an element paints, with everything in it, about \
                      its middle. It is paint only: layout keeps the element's slot and nothing beside it moves.",
        import: Some("use lurq::node::transform::Transform2D;"),
        status: Some("Stable"),
        since: None,
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "A tile over the dashed slot layout gave it. Turn it, scale it, skew it and move it: the slot stays where \
           it is. Hover it and click it where it is painted: the pointer finds it there.",
          playground,
        ),
        Section::new(
          "Rotate",
          "A turn about the element's middle, by degrees. It is drawn by the GPU and changes no layout.",
          self.frame.demo(
            0,
            rotated(),
            ".transform(Transform2D::rotate_deg(30.0))",
            &[
              "Row::new()",
              "    .size(60.0, 60.0)",
              "    .rounded(RadiusSize::Sm)",
              "    .transform(Transform2D::rotate_deg(30.0))",
            ],
          ),
        ),
        Section::new(
          "Scale",
          "By one factor, or by one along each axis.",
          self.frame.demo(
            1,
            scaled(),
            ".transform(Transform2D::scale(1.4, 0.7))",
            &[
              "Row::new().size(60.0, 60.0).transform(Transform2D::scale_uniform(1.5))",
              "Row::new().size(60.0, 60.0).transform(Transform2D::scale(1.4, 0.7))",
            ],
          ),
        ),
        Section::new(
          "Skew",
          "A shear along x, along y, or both, by an angle in radians.",
          self.frame.demo(
            2,
            skewed(),
            ".transform(Transform2D::skew(20f32.to_radians(), 0.0))",
            &[
              "Row::new().size(60.0, 60.0).transform(Transform2D::skew(20f32.to_radians(), 0.0))",
              "Row::new().size(60.0, 60.0).transform(Transform2D::skew(0.0, 10f32.to_radians()))",
            ],
          ),
        ),
        Section::new(
          "Composed",
          "Transforms composed with then are applied from right to left: a turn then a scale is not a scale then a \
           turn, as the first two show.",
          self.frame.demo(
            3,
            composed(),
            "Transform2D::rotate_deg(30.0).then(&Transform2D::scale(1.3, 0.8))",
            &[
              "// The scale is applied first, then the turn.",
              "Row::new().transform(Transform2D::rotate_deg(30.0).then(&Transform2D::scale(1.3, 0.8)))",
              "",
              "Row::new().transform(",
              "    Transform2D::rotate_deg(20.0)",
              "        .then(&Transform2D::scale(1.2, 0.8))",
              "        .then(&Transform2D::skew(0.15, 0.0)),",
              ")",
            ],
          ),
        ),
        Section::new(
          "With text",
          "A transform takes the whole of what is in the element, text and all.",
          self.frame.demo(
            4,
            worded_all(),
            ".transform(Transform2D::rotate_deg(45.0))",
            &[
              "Row::new()",
              "    .size(100.0, 50.0)",
              "    .align_items(Alignment::Center)",
              "    .justify(Justify::Center)",
              "    .transform(Transform2D::rotate_deg(45.0))",
              "    .child(Text::new(\"Hello\"))",
            ],
          ),
        ),
        Section::new(
          "The slot stays",
          "The middle tile is turned and grown, and paints over its slot, dashed; the tiles beside it stay where \
           layout put them.",
          self.frame.demo(
            5,
            slot(),
            ".transform(Transform2D::rotate_deg(30.0).then(&Transform2D::scale_uniform(1.3)))",
            &[
              "Row::new()",
              "    .spacing(12.0)",
              "    .child(Rect::new(72.0, 48.0))",
              "    .child(Rect::new(72.0, 48.0).transform(",
              "        Transform2D::rotate_deg(30.0).then(&Transform2D::scale_uniform(1.3)),",
              "    ))",
              "    .child(Rect::new(72.0, 48.0))",
            ],
          ),
        ),
        Section::new(
          "Text kept sharp",
          "A text that moves keeps its glyphs and moves them, which costs nothing per frame. A text that stays turned \
           can have its glyphs drawn at the angle, with sharper edges.",
          self.frame.demo(
            6,
            sharp(),
            ".text_transform_mode(TextTransformMode::Rasterized)",
            &[
              "Text::new(\"A static label, turned\")",
              "    .text_transform_mode(TextTransformMode::Rasterized)",
              "    .transform(Transform2D::rotate_deg(-8.0))",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What an element is given for its transform, and what a Transform2D is made of.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
