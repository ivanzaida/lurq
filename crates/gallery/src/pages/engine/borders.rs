//! The page of borders: a line on some sides of a box or on all of them, inside its edge, on it or outside it,
//! solid or dashed, round its corners. Its playground is one box whose border is what the controls say; its
//! sections are the demo's cases at the radius the playground is set to, corners of their own, and the widths of
//! the theme.

use std::sync::Arc;

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{BorderSize, PaletteColor, RadiusSize},
  },
  components::{Column, Row},
  core::Signal,
  layout::{Alignment, text_style::FontWeight},
  node::{
    Element,
    border::{Border, BorderPlacement, BorderRadius},
  },
  ui::Slider,
};

use super::kit::{sample, samples};
use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, mono, props_table, text},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

/// Where the line is: an index into [`PLACED`], which a signal keeps.
const PLACEMENTS: [(&str, usize); 3] = [("Inside", 0), ("Center", 1), ("Outside", 2)];
const PLACED: [BorderPlacement; 3] = [
  BorderPlacement::Inside,
  BorderPlacement::Center,
  BorderPlacement::Outside,
];
/// The names of the sides, in the order the page keeps them: top, right, bottom, left.
const SIDES: [&str; 4] = ["top", "right", "bottom", "left"];
/// How large the playground's box is, where the stage has the room.
const BOX: (f32, f32) = (300.0, 150.0);

/// A control of a playground's panel that is a number between `min` and `max`: a slider with the value beside it,
/// under the name of its prop and its type. For a value whose every step is worth trying, a corner's radius or a
/// border's width, where a select of a few would hide the one that goes wrong.
pub(super) fn slide(label: &str, kind: &str, value: &Signal<i32>, min: i32, max: i32) -> Element {
  prop(
    label,
    kind,
    Row::new()
      .width(FILL)
      .spacing(12.0)
      .align_items(Alignment::Center)
      .child(Slider::new(value.clone()).min(min).max(max).label(label).length(196.0))
      .child(mono(&value.get().to_string(), 12.0).nowrap()),
  )
}

/// The border the controls say, in the primary color: what the playground puts on each side that is on.
#[derive(Clone, Copy, PartialEq)]
struct Line {
  width: i32,
  placement: usize,
  dashed: bool,
  dash: i32,
  gap: i32,
}

impl Line {
  fn border(self) -> Border {
    let border = Border::new(
      self.width as f32,
      PaletteColor::Primary,
      PLACED[self.placement.min(PLACED.len() - 1)],
    );
    if self.dashed {
      border.dash(self.dash as f32, self.gap as f32)
    } else {
      border
    }
  }

  /// The Rust of the border.
  fn code(self) -> String {
    let placement = PLACEMENTS[self.placement.min(PLACEMENTS.len() - 1)].0.to_lowercase();
    let mut code = format!("Border::{placement}({}.0, PaletteColor::Primary)", self.width);
    if self.dashed {
      code.push_str(&format!(".dash({}.0, {}.0)", self.dash, self.gap));
    }
    code
  }
}

pub struct BordersPage {
  frame: Frame,
  playground: Playground,
  radius: Signal<i32>,
  width: Signal<i32>,
  /// An index into [`PLACED`].
  placement: Signal<usize>,
  dashed: Signal<bool>,
  dash: Signal<i32>,
  gap: Signal<i32>,
  top: Signal<bool>,
  right: Signal<bool>,
  bottom: Signal<bool>,
  left: Signal<bool>,
}

impl BordersPage {
  fn sides(&self) -> [bool; 4] {
    [self.top.get(), self.right.get(), self.bottom.get(), self.left.get()]
  }

  fn play(&self, ctx: &mut Ctx) -> Element {
    let line = Line {
      width: self.width.get(),
      placement: self.placement.get(),
      dashed: self.dashed.get(),
      dash: self.dash.get(),
      gap: self.gap.get(),
    };
    let (radius, sides) = (self.radius.get(), self.sides());
    // As large as the design has it where the stage has the room, and narrower in a narrow one, with room left
    // round it for a border outside it.
    let room = self.playground.stage_room();
    let width = if room > 0.0 {
      BOX.0.min(room - 2.0 * 16.0).max(80.0)
    } else {
      BOX.0
    };
    let build = move || -> Element {
      let border = line.border();
      let mut shown = Column::new()
        .size(width, BOX.1)
        .background(PaletteColor::Muted)
        .rounded(radius as f32);
      if sides.iter().all(|on| *on) {
        return shown.border(border).into();
      }
      if sides[0] {
        shown = shown.border_top(border.clone());
      }
      if sides[1] {
        shown = shown.border_right(border.clone());
      }
      if sides[2] {
        shown = shown.border_bottom(border.clone());
      }
      if sides[3] {
        shown = shown.border_left(border);
      }
      shown.into()
    };

    let mut code = vec![
      "Column::new()".to_owned(),
      format!("    .size({width}.0, {}.0)", BOX.1),
      "    .background(PaletteColor::Muted)".to_owned(),
      format!("    .rounded({radius}.0)"),
    ];
    let on: Vec<&str> = SIDES
      .iter()
      .zip(sides)
      .filter_map(|(name, on)| on.then_some(*name))
      .collect();
    if on.len() == 4 {
      code.push(format!("    .border({})", line.code()));
    } else {
      for side in &on {
        code.push(format!("    .border_{side}({})", line.code()));
      }
    }
    let placement = PLACEMENTS[line.placement.min(PLACEMENTS.len() - 1)].0.to_lowercase();
    let tree = vec![
      format!("column   {}", self.playground.measured()),
      if on.is_empty() {
        "└─ no border".to_owned()
      } else {
        format!("└─ border: {}, {placement}, {}", on.join(", "), line.width)
      },
    ];

    let set_placement = self.placement.clone();
    let reset = {
      let (radius, width, placement, dashed, dash, gap) = (
        self.radius.clone(),
        self.width.clone(),
        self.placement.clone(),
        self.dashed.clone(),
        self.dash.clone(),
        self.gap.clone(),
      );
      let sides = [
        self.top.clone(),
        self.right.clone(),
        self.bottom.clone(),
        self.left.clone(),
      ];
      Arc::new(move || {
        radius.set(16);
        width.set(4);
        placement.set(0);
        dashed.set(false);
        dash.set(8);
        gap.set(6);
        for (index, side) in sides.iter().enumerate() {
          side.set(index == 3);
        }
      })
    };
    let mut controls = vec![
      prop(
        "placement",
        "BorderPlacement",
        self
          .playground
          .select(0, &PLACEMENTS, &line.placement, move |value| set_placement.set(value)),
      ),
      slide("width", "f32", &self.width, 1, 16),
      slide("rounded", "f32", &self.radius, 0, 60),
      divider(),
      flag("border_top", &self.top),
      flag("border_right", &self.right),
      flag("border_bottom", &self.bottom),
      flag("border_left", &self.left),
      divider(),
      flag("dashed", &self.dashed),
    ];
    // The lengths of a dash and of a gap are there while the line is dashed.
    if line.dashed {
      controls.push(slide("dash", "f32", &self.dash, 1, 30));
      controls.push(slide("gap", "f32", &self.gap, 1, 30));
    }
    self.playground.card(
      ctx,
      Spec {
        title: "Borders",
        make: Arc::new(build),
        shown: Shown::built(move |_| build()),
        fill: false,
        forceable: false,
        facts: format!("radius {radius}"),
        event: "",
        code,
        tree,
        controls,
        changed: [
          radius != 16,
          line.width != 4,
          line.placement != 0,
          sides != [false, false, false, true],
          line.dashed,
          line.dashed && (line.dash != 8 || line.gap != 6),
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset,
      },
    )
  }

  /// A box of the radius the playground is set to, with what it is in it.
  fn case(&self, caption: &str) -> Column {
    Column::new()
      .width(150.0)
      .max_width(FILL)
      .height(76.0)
      .padding(10.0)
      .background(PaletteColor::Muted)
      .rounded(self.radius.get() as f32)
      .child(text(caption, 12.0, FontWeight::Medium))
  }

  /// The demo's cases, each at the radius the playground is set to.
  fn cases(&self) -> Row {
    let line = |width: f32| Border::inside(width, PaletteColor::MutedForeground);
    // Room between the boxes and round them: a border outside one is painted past its edge.
    Row::new().width(FILL).spacing(18.0).padding(8.0).wrap().with_children([
      self
        .case("left only")
        .border_left(Border::inside(5.0, PaletteColor::Primary)),
      self
        .case("top 1, bottom 6")
        .border_top(line(1.0))
        .border_bottom(line(6.0)),
      self
        .case("three sides")
        .border_top(line(2.0))
        .border_left(line(2.0))
        .border_right(line(2.0)),
      self
        .case("two colors")
        .border_top(Border::inside(3.0, PaletteColor::Warning))
        .border_bottom(Border::inside(3.0, PaletteColor::Info)),
      self
        .case("outside 4")
        .border(Border::outside(4.0, PaletteColor::Primary)),
      self.case("center 4").border(Border::center(4.0, PaletteColor::Primary)),
      self
        .case("outside, left")
        .border_left(Border::outside(5.0, PaletteColor::Primary)),
      // A border is painted over what is in the box: the caption starts under it.
      self
        .case("wider than radius")
        .padding_top(30.0)
        .border_top(Border::inside(24.0, PaletteColor::Ring)),
      self.case("dashed").border(line(1.5).dashed()),
      self
        .case("dash 2 4")
        .border(Border::inside(2.0, PaletteColor::Primary).dash(2.0, 4.0)),
      self
        .case("dashed outside")
        .border(Border::outside(3.0, PaletteColor::Info).dashed()),
      self.case("dashed bottom").border_bottom(line(2.0).dashed()),
    ])
  }
}

/// A box with a radius of its own at each corner, and a border round them.
fn cornered(caption: &str, radii: BorderRadius, border: Border) -> Column {
  sample(
    Column::new()
      .width(150.0)
      .max_width(FILL)
      .height(76.0)
      .background(PaletteColor::Muted)
      .corner_radius_custom(radii)
      .border(border),
    caption,
  )
}

/// Boxes with corners of their own: a leaf, a tab, a box rounded at its top only, a pill.
fn corners() -> Row {
  let tab = Column::new()
    .width(150.0)
    .max_width(FILL)
    .height(76.0)
    .background(PaletteColor::Muted)
    .corner_radius_top_left(12.0)
    .corner_radius_top_right(12.0)
    .border_top(Border::inside(2.0, PaletteColor::Primary))
    .border_left(Border::inside(2.0, PaletteColor::Primary))
    .border_right(Border::inside(2.0, PaletteColor::Primary));
  samples()
    .padding(8.0)
    .child(cornered(
      "24, 0, 24, 0",
      BorderRadius::new(24.0, 0.0, 24.0, 0.0),
      Border::inside(2.0, PaletteColor::Primary),
    ))
    .child(sample(tab, "a tab: three sides"))
    .child(cornered(
      "16, 16, 0, 0",
      BorderRadius::new(16.0, 16.0, 0.0, 0.0),
      Border::center(3.0, PaletteColor::Info).dashed(),
    ))
    .child(sample(
      Row::new()
        .width(150.0)
        .max_width(FILL)
        .height(40.0)
        .background(PaletteColor::Muted)
        .rounded(RadiusSize::Full)
        .border(Border::outside(2.0, PaletteColor::Primary).dashed()),
      "RadiusSize::Full",
    ))
}

impl Component for BordersPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 1),
      radius: ctx.signal(16),
      width: ctx.signal(4),
      placement: ctx.signal(0),
      dashed: ctx.signal(false),
      dash: ctx.signal(8),
      gap: ctx.signal(6),
      top: ctx.signal(false),
      right: ctx.signal(false),
      bottom: ctx.signal(false),
      left: ctx.signal(true),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let theme = ctx.theme().clone();
    let rows = [
      (
        "border_inside",
        "(f32, color)",
        "",
        "A line of one width on all four sides, inside the box. border_center and border_outside put it on the \
         edge and outside it.",
      ),
      (
        "border",
        "Border",
        "",
        "A border on all four sides, as Border says: its width, its color, where it is and its dashes.",
      ),
      (
        "border_top",
        "Border",
        "",
        "A border on one side; border_right, border_bottom and border_left the others. A side with none shows \
         nothing at its edge.",
      ),
      (
        "Border::inside",
        "(width, color)",
        "",
        "A border inside the box. Border::center is on its edge and Border::outside outside it, where layout and \
         hit testing do not reach. The width is pixels or a BorderSize; the color a color or a PaletteColor.",
      ),
      (
        "dashed",
        "()",
        "",
        "Cuts the line into dashes three times its width long, with gaps as long.",
      ),
      (
        "dash",
        "(f32, f32)",
        "",
        "The lengths of a dash and of a gap. The period is fitted to the outline, so the last dash meets the first \
         with a whole gap between them.",
      ),
      (
        "rounded",
        "impl Into<RadiusValue>",
        "0",
        "The radius of every corner, pixels or a RadiusSize: the border follows it.",
      ),
      (
        "corner_radius_custom",
        "BorderRadius",
        "",
        "A radius of its own at each corner, from the top left round to the bottom left; corner_radius_top_left \
         and the others set one.",
      ),
    ];
    let playground = self.play(ctx);
    let widths = {
      let mut row = samples().padding(8.0);
      for (name, size) in [("Sm", BorderSize::Sm), ("Md", BorderSize::Md), ("Lg", BorderSize::Lg)] {
        let value = theme.border_size_value(size.clone());
        row = row.child(sample(
          Column::new()
            .width(150.0)
            .max_width(FILL)
            .height(56.0)
            .background(PaletteColor::Muted)
            .rounded(RadiusSize::Md)
            .border(Border::inside(size, PaletteColor::Primary)),
          &format!("BorderSize::{name}: {value}"),
        ));
      }
      row
    };
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Engine", "Borders"],
        title: "Borders",
        description: "A line on some sides of a box or on all of them: inside its edge, on it or outside it, solid \
                      or dashed. It follows the box's corners, and a side with no border shows nothing.",
        import: Some("use lurq::node::border::{Border, BorderPlacement};"),
        status: Some("Stable"),
        since: None,
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "One box whose border is what the controls say. Change the sides, the radius and the width and look at \
           the corners: a side with no border shows nothing at its edge at any radius, and a border on one side \
           follows the corners at its ends and fades out along them. Try a fractional display scale too.",
          playground,
        ),
        Section::new(
          "Cases",
          "The cases worth a look, at the radius the playground is set to. Dashes go round the corners and close on \
           themselves: no half dash where the pattern ends.",
          self.frame.demo(
            0,
            self.cases(),
            ".border_left(Border::inside(5.0, PaletteColor::Primary))",
            &[
              "Column::new().rounded(16.0).border_left(Border::inside(5.0, PaletteColor::Primary))",
              "Column::new().border_top(line(1.0)).border_bottom(line(6.0))",
              "Column::new().border_top(Border::inside(3.0, PaletteColor::Warning))",
              "    .border_bottom(Border::inside(3.0, PaletteColor::Info))",
              "Column::new().border(Border::outside(4.0, PaletteColor::Primary))",
              "Column::new().border(Border::center(4.0, PaletteColor::Primary))",
              "Column::new().border(line(1.5).dashed())",
              "Column::new().border(Border::inside(2.0, PaletteColor::Primary).dash(2.0, 4.0))",
              "Column::new().border_bottom(line(2.0).dashed())",
            ],
          ),
        ),
        Section::new(
          "Corners of their own",
          "A radius for each corner, and a border that follows them: a leaf, a tab with no line at its foot, a box \
           rounded at its top with a dashed line on its edge, and a pill.",
          self.frame.demo(
            1,
            corners(),
            ".corner_radius_custom(BorderRadius::new(24.0, 0.0, 24.0, 0.0))",
            &[
              "Column::new()",
              "    .corner_radius_custom(BorderRadius::new(24.0, 0.0, 24.0, 0.0))",
              "    .border(Border::inside(2.0, PaletteColor::Primary))",
              "",
              "// A tab: rounded at its top, with no line at its foot.",
              "Column::new()",
              "    .corner_radius_top_left(12.0)",
              "    .corner_radius_top_right(12.0)",
              "    .border_top(Border::inside(2.0, PaletteColor::Primary))",
              "    .border_left(Border::inside(2.0, PaletteColor::Primary))",
              "    .border_right(Border::inside(2.0, PaletteColor::Primary))",
            ],
          ),
        ),
        Section::new(
          "Widths of the theme",
          "A border's width can be a step of the theme's, as a color can be a role: a change of the theme changes \
           every border drawn with it.",
          self.frame.demo(
            2,
            widths,
            "Border::inside(BorderSize::Sm, PaletteColor::Border)",
            &[
              "Column::new().border(Border::inside(BorderSize::Sm, PaletteColor::Primary))",
              "Column::new().border(Border::inside(BorderSize::Md, PaletteColor::Primary))",
              "Column::new().border(Border::inside(BorderSize::Lg, PaletteColor::Primary))",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a box is given for its border and its corners, and what a Border is.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
