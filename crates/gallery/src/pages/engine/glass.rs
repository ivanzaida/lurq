//! The page of glass: a box that bends, blurs and colours what is painted behind it, as a piece of glass lying on
//! the page does. Its sections show the two materials, a tint, what each number of the material does, shapes
//! that flow together in a container, and a glass that comes and goes.

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{PaletteColor, RadiusSize},
  },
  components::{Column, Rect, Row, Stack},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{Element, Glass},
};

use super::kit::tint;
use crate::{
  doc::{Frame, Header, Section},
  kit::{fit, text},
};

/// How wide a scene is: it cannot be narrower, and scrolls sideways in a card that is.
const SCENE: f32 = 520.0;
/// How high.
const HIGH: f32 = 200.0;

/// What lies behind the glass: tiles of the theme's chart colours in a slanted pattern, and lines of words, so
/// that a bend, a blur and a tint can each be seen.
fn behind() -> Stack {
  let mut tiles = Column::new();
  for row in 0..5 {
    tiles = tiles.child(
      Row::new().with_children((0..13).map(|column| Rect::new(40.0, 40.0).background(tint((row * 2 + column) % 5)))),
    );
  }
  let mut words = Column::new().padding(14.0).spacing(6.0);
  for line in [
    "Glass bends what is behind it",
    "along its edge, as a lens does,",
    "and leaves its middle where it is.",
    "It blurs, saturates and tints it,",
    "and has a light along its rim.",
  ] {
    words = words.child(
      text(line, 20.0, FontWeight::Bold)
        .color(PaletteColor::Background)
        .nowrap(),
    );
  }
  Stack::new()
    .size(SCENE, HIGH)
    .rounded(RadiusSize::Md)
    .clip()
    .child(tiles)
    .child(words)
}

/// A pill of `glass` with `caption` on it.
fn pill(caption: &str, glass: Glass) -> Row {
  Row::new()
    .size(140.0, 56.0)
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .rounded(RadiusSize::Full)
    .glass(glass)
    .foreground(PaletteColor::Foreground)
    .child(text(caption, 14.0, FontWeight::SemiBold).nowrap())
}

/// The scene with `over` laid in its middle.
fn scene(over: impl Into<Element>) -> Stack {
  behind().child(
    Row::new()
      .size(SCENE, HIGH)
      .align_items(Alignment::Center)
      .justify(Justify::Center)
      .overflow_visible()
      .child(over),
  )
}

/// The two materials side by side, and a tinted one.
fn materials() -> Stack {
  scene(
    Row::new()
      .spacing(20.0)
      .overflow_visible()
      .child(pill("regular", Glass::regular()))
      .child(pill("clear", Glass::clear()))
      .child(pill(
        "tinted",
        Glass::regular().tint(lurq::node::color::Color::new(0, 122, 255, 150)),
      )),
  )
}

/// TEMP: the probe's pattern and glasses, to hold against the system's.
fn probe() -> Stack {
  use lurq::node::color::Color;
  let colors: [(u8, u8, u8); 12] = [
    (255, 0, 0), (0, 255, 0), (0, 0, 255), (255, 255, 0), (0, 255, 255), (255, 0, 255),
    (128, 0, 0), (0, 128, 0), (0, 0, 128), (204, 102, 51), (51, 102, 204), (77, 179, 102),
  ];
  let mut tiles = Column::new();
  for row in 0..36 {
    tiles = tiles.child(Row::new().with_children((0..21).map(|column| {
      let c = colors[(row * 3 + column) % 12];
      Rect::new(20.0, 20.0).background(if (row + column) % 2 == 0 {
        Color::new(c.0, c.1, c.2, 255)
      } else {
        Color::new(230, 230, 230, 255)
      })
    })));
  }
  let at = |x: f32, y: f32, w: f32, h: f32, r: f32, glass: Glass| {
    Row::new().size(w, h).rounded(r).glass(glass).absolute_position(x - w / 2.0, y - h / 2.0)
  };
  Stack::new()
    .size(402.0, 720.0)
    .clip()
    .child(tiles)
    .child(at(201.0, 80.0, 220.0, 56.0, 28.0, Glass::regular()))
    .child(at(201.0, 200.0, 220.0, 56.0, 28.0, Glass::clear()))
    .child(at(201.0, 360.0, 200.0, 140.0, 36.0, Glass::panel()))
    .child(at(100.0, 520.0, 56.0, 56.0, 28.0, Glass::regular()))
    .child(at(302.0, 520.0, 56.0, 56.0, 28.0, Glass::clear()))
    .child(at(201.0, 660.0, 300.0, 64.0, 32.0, Glass::regular()))
}

/// Glasses in a container: near each other they are one drop.
fn drops() -> Stack {
  let drop = |width: f32| {
    Row::new()
      .size(width, 56.0)
      .rounded(RadiusSize::Full)
      .glass(Glass::regular())
  };
  scene(
    Column::new().spacing(24.0).overflow_visible().child(
      Row::new()
        .glass_container(40.0)
        .overflow_visible()
        .spacing(6.0)
        .child(drop(56.0))
        .child(drop(120.0))
        .child(Rect::new(30.0, 1.0))
        .child(drop(56.0))
        .child(Rect::new(60.0, 1.0))
        .child(drop(56.0)),
    ),
  )
}

pub struct GlassPage {
  frame: Frame,
}

impl Component for GlassPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self { frame: Frame::new(ctx) }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Engine", "Glass"],
        title: "Glass",
        description: "A box that is a piece of glass over what is painted behind it: it bends it along its edge as \
                      a lens does, blurs, saturates and tints it, and has a light on its rim and a shadow under it.",
        import: Some("use lurq::node::Glass;"),
        status: Some("New"),
        since: None,
        source: file!(),
      },
      vec![
        Section::new("TEMP probe", "probe", self.frame.demo(9, probe(), "", &[])),
        Section::new(
          "Materials",
          "The system's two, and a tint. Regular blurs what is behind enough for words on it to be read over \
           anything; clear leaves it sharp, for over a picture.",
          self.frame.demo(
            0,
            fit("glass-materials", self.frame.stage_room(), SCENE, materials()),
            ".glass(Glass::regular())",
            &[
              "Row::new()",
              "    .rounded(RadiusSize::Full)",
              "    .glass(Glass::regular())",
              "",
              "Row::new().rounded(RadiusSize::Full).glass(Glass::clear())",
              "Row::new().rounded(RadiusSize::Full).glass(Glass::regular().tint(blue))",
            ],
          ),
        ),
        Section::new(
          "Shapes that flow together",
          "The glasses in a container are one surface: two that come within the container's distance of each \
           other join into one drop, and part again further off.",
          self.frame.demo(
            1,
            fit("glass-drops", self.frame.stage_room(), SCENE, drops()),
            ".glass_container(40.0)",
            &[
              "Row::new()",
              "    .glass_container(40.0)",
              "    .spacing(6.0)",
              "    .child(Row::new().rounded(RadiusSize::Full).glass(Glass::regular()))",
              "    .child(Row::new().rounded(RadiusSize::Full).glass(Glass::regular()))",
            ],
          ),
        ),
      ],
    )
  }
}
