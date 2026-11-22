//! The page of glass: a box that bends, blurs and colours what is painted behind it, as a piece of glass lying on
//! the page does. Its playground is one glass over a pattern, with the numbers of its material; its sections show
//! the materials, the lens, a glass that follows what is behind it, shapes that flow together in a container, a
//! glass that answers a press, one that changes shape and one that comes and goes.

use std::sync::Arc;

use lurq::{
  animation::{Easing, Transition},
  app::{
    component::Component,
    ctx::Ctx,
    theme::{PaletteColor, RadiusSize},
  },
  components::{Column, Rect, Row, ScrollVertical, Stack},
  core::Signal,
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{Element, Glass},
  ui::{Button, ButtonSize},
};

use super::kit::tint;
use crate::{
  doc::{Frame, Header, Section},
  kit::{fit, props_table, text},
  playground::{Playground, Shown, Spec, flag, prop},
};

/// How wide a scene is: it cannot be narrower, and scrolls sideways in a card that is.
const SCENE: f32 = 520.0;
/// How high.
const HIGH: f32 = 200.0;

/// The materials the playground offers.
const MATERIALS: [(&str, usize); 3] = [("Glass::regular()", 0), ("Glass::panel()", 1), ("Glass::clear()", 2)];
const BENDS: [(&str, i32); 4] = [("0", 0), ("20", 20), ("45", 45), ("70", 70)];
const BEZELS: [(&str, i32); 3] = [("8", 8), ("19.5", 20), ("32", 32)];
const BLURS: [(&str, i32); 4] = [("0", 0), ("3.8", 4), ("10.5", 10), ("24", 24)];
const RADII: [(&str, i32); 3] = [("16", 16), ("36", 36), ("RadiusSize::Full", 999)];

fn material(index: usize) -> Glass {
  match index {
    1 => Glass::panel(),
    2 => Glass::clear(),
    _ => Glass::regular(),
  }
}

/// Tiles of the theme's chart colours, `size` a side, in a pattern that runs aslant: what a lens is seen to bend.
fn tiles(size: f32) -> Column {
  let (columns, rows) = ((SCENE / size).ceil() as usize, (HIGH / size).ceil() as usize);
  let mut all = Column::new();
  for row in 0..rows {
    all = all.child(Row::new().with_children((0..columns).map(|column| {
      Rect::new(size, size).background(if (row + column) % 2 == 0 {
        tint(row * 2 + column)
      } else {
        PaletteColor::Background
      })
    })));
  }
  all
}

/// What lies behind the glass: tiles and lines of words, so that a bend, a blur and a tint can each be seen.
fn behind() -> Stack {
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
        .color(PaletteColor::Foreground)
        .nowrap(),
    );
  }
  Stack::new()
    .size(SCENE, HIGH)
    .rounded(RadiusSize::Md)
    .clip()
    .child(tiles(40.0))
    .child(words)
}

/// A pill of `glass` with `caption` on it, in the glass's own ink.
fn pill(caption: &str, glass: Glass) -> Row {
  Row::new()
    .size(140.0, 56.0)
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .rounded(RadiusSize::Full)
    .glass(glass)
    .child(text(caption, 14.0, FontWeight::SemiBold).nowrap())
}

/// `over` laid in the middle of `back`.
fn stage(back: Stack, over: impl Into<Element>) -> Stack {
  back.child(
    Row::new()
      .size(SCENE, HIGH)
      .align_items(Alignment::Center)
      .justify(Justify::Center)
      .overflow_visible()
      .child(over),
  )
}

/// The three materials side by side, and a coloured one.
fn materials() -> Stack {
  stage(
    behind(),
    Row::new()
      .spacing(14.0)
      .overflow_visible()
      .child(pill("regular", Glass::regular()).width(110.0))
      .child(pill("panel", Glass::panel()).width(110.0))
      .child(pill("clear", Glass::clear()).width(110.0))
      .child(
        pill(
          "tinted",
          Glass::regular().tint(lurq::node::color::Color::new(0, 122, 255, 190)),
        )
        .width(110.0),
      ),
  )
}

/// The same clear glass with no bend, with half the system's and with the system's, over small tiles.
fn lens() -> Stack {
  let glass = |caption: &str, bend: f32| {
    Column::new()
      .size(132.0, 132.0)
      .align_items(Alignment::Center)
      .justify(Justify::Center)
      .rounded(40.0)
      .glass(Glass::clear().bend(bend))
      .child(
        text(caption, 13.0, FontWeight::SemiBold)
          .color(PaletteColor::Foreground)
          .nowrap(),
      )
  };
  stage(
    Stack::new()
      .size(SCENE, HIGH)
      .rounded(RadiusSize::Md)
      .clip()
      .child(tiles(20.0)),
    Row::new()
      .spacing(24.0)
      .overflow_visible()
      .child(glass("bend 0", 0.0))
      .child(glass("bend 20", 20.0))
      .child(glass("bend 45", 45.0)),
  )
}

/// A regular glass over a page that scrolls from light to dark: it turns with what is behind it.
fn follows() -> Stack {
  let band = |role: PaletteColor, words: &str, ink: PaletteColor| {
    Column::new()
      .size(SCENE, 150.0)
      .padding(16.0)
      .background(role)
      .child(text(words, 16.0, FontWeight::SemiBold).color(ink))
  };
  let page = Column::new()
    .child(band(
      PaletteColor::Background,
      "Scroll this: light behind the glass",
      PaletteColor::Foreground,
    ))
    .child(band(
      PaletteColor::Foreground,
      "dark behind the glass",
      PaletteColor::Background,
    ))
    .child(band(PaletteColor::Chart1, "a colour", PaletteColor::Background))
    .child(band(
      PaletteColor::Background,
      "and light again.",
      PaletteColor::Foreground,
    ));
  stage(
    Stack::new()
      .size(SCENE, HIGH)
      .rounded(RadiusSize::Md)
      .clip()
      .child(ScrollVertical::new(page).key("glass-follows").size(SCENE, HIGH)),
    pill("follows", Glass::regular()).pointer_events_none(),
  )
}

/// Glasses in a container: near each other they are one drop.
fn drops(together: bool) -> Stack {
  let drop = |width: f32| {
    Row::new()
      .size(width, 56.0)
      .rounded(RadiusSize::Full)
      .glass(Glass::regular())
  };
  let gap = |width: f32| {
    Rect::new(if together { 0.0 } else { width }, 1.0)
      .transition(Transition::width().duration_ms(600).easing(Easing::Spring { velocity: 0.0 }))
  };
  stage(
    behind(),
    Row::new()
      .glass_container(36.0)
      .overflow_visible()
      .align_items(Alignment::Center)
      .child(drop(56.0))
      .child(gap(44.0))
      .child(drop(120.0))
      .child(gap(44.0))
      .child(drop(56.0)),
  )
}

/// A glass that answers a press: it grows, is lit from where it is touched, and springs back.
fn pressed() -> Stack {
  stage(
    behind(),
    Row::new()
      .spacing(20.0)
      .overflow_visible()
      .child(pill("Press me", Glass::regular().interactive(true)))
      .child(
        Row::new()
          .size(56.0, 56.0)
          .align_items(Alignment::Center)
          .justify(Justify::Center)
          .rounded(RadiusSize::Full)
          .glass(Glass::regular().interactive(true))
          .child(text("+", 22.0, FontWeight::Medium)),
      )
      .child(pill("clear", Glass::clear().interactive(true))),
  )
}

/// A glass that changes its size and its corners: it flows from one shape to the other.
fn shaped(wide: bool) -> Stack {
  let moved = Transition::all()
    .duration_ms(520)
    .easing(Easing::Spring { velocity: 0.0 });
  stage(
    behind(),
    Row::new()
      .size(if wide { 300.0 } else { 56.0 }, if wide { 120.0 } else { 56.0 })
      .rounded(if wide { 32.0 } else { 28.0 })
      .glass(Glass::regular())
      .transition(moved),
  )
}

/// A glass that comes and goes as a material: its bend, its blur and its light grow in together.
fn coming(there: bool) -> Stack {
  let mut pane = Row::new()
    .size(200.0, 72.0)
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .rounded(RadiusSize::Full)
    .transition(Transition::glass().duration_ms(450));
  if there {
    pane = pane.glass(Glass::regular());
  }
  stage(behind(), pane)
}

/// A button under a demo that turns `state`.
fn turn(state: &Signal<bool>, on: &str, off: &str) -> Button {
  let (set, now) = (state.clone(), state.get());
  Button::new(if now { on } else { off })
    .size(ButtonSize::Sm)
    .on_click(move |_| set.set(!now))
}

/// A demo with the button that changes it under it.
fn with_button(scene: Element, button: Button) -> Column {
  Column::new().spacing(12.0).overflow_visible().child(scene).child(button)
}

pub struct GlassPage {
  frame: Frame,
  playground: Playground,
  /// An index into [`MATERIALS`].
  material: Signal<usize>,
  bend: Signal<i32>,
  bezel: Signal<i32>,
  blur: Signal<i32>,
  radius: Signal<i32>,
  interactive: Signal<bool>,
  together: Signal<bool>,
  wide: Signal<bool>,
  there: Signal<bool>,
}

impl GlassPage {
  fn play(&self, ctx: &mut Ctx) -> Element {
    let kind = self.material.get().min(MATERIALS.len() - 1);
    let (bend, bezel, blur, radius) = (self.bend.get(), self.bezel.get(), self.blur.get(), self.radius.get());
    let interactive = self.interactive.get();
    let bezel_value = if bezel == 20 { 19.5 } else { bezel as f32 };
    let blur_value = match blur {
      4 => 3.8,
      10 => 10.5,
      other => other as f32,
    };
    let build = move || -> Element {
      let glass = material(kind)
        .bend(bend as f32)
        .bezel(bezel_value)
        .blur(blur_value)
        .interactive(interactive);
      let pane = Row::new()
        .size(220.0, 120.0)
        .align_items(Alignment::Center)
        .justify(Justify::Center)
        .glass(glass)
        .child(text("Glass", 15.0, FontWeight::SemiBold).nowrap());
      let pane = if radius == 999 {
        pane.rounded(RadiusSize::Full)
      } else {
        pane.rounded(radius as f32)
      };
      stage(
        Stack::new()
          .size(SCENE, HIGH)
          .rounded(RadiusSize::Md)
          .clip()
          .child(tiles(20.0)),
        pane,
      )
      .into()
    };

    let mut code = vec![
      "Row::new()".to_owned(),
      "    .size(220.0, 120.0)".to_owned(),
      format!(
        "    .rounded({})",
        if radius == 999 {
          "RadiusSize::Full".to_owned()
        } else {
          format!("{radius}.0")
        }
      ),
      "    .glass(".to_owned(),
      format!("        {}", MATERIALS[kind].0),
    ];
    let default = material(kind);
    if bend as f32 != default.bend {
      code.push(format!("            .bend({bend}.0)"));
    }
    if bezel_value != default.bezel {
      code.push(format!("            .bezel({bezel_value:?})"));
    }
    if blur_value != default.blur {
      code.push(format!("            .blur({blur_value:?})"));
    }
    if interactive {
      code.push("            .interactive(true)".to_owned());
    }
    code.push("    )".to_owned());
    code.push("    .child(Text::new(\"Glass\"))".to_owned());
    let tree = vec![
      format!("row   {}", self.playground.measured()),
      "└─ text \"Glass\"".to_owned(),
    ];

    let (set_material, set_bend, set_bezel, set_blur, set_radius) = (
      self.material.clone(),
      self.bend.clone(),
      self.bezel.clone(),
      self.blur.clone(),
      self.radius.clone(),
    );
    // A material chosen brings its own blur.
    let material_blur = self.blur.clone();
    let reset = {
      let (material, bend, bezel, blur, radius, interactive) = (
        self.material.clone(),
        self.bend.clone(),
        self.bezel.clone(),
        self.blur.clone(),
        self.radius.clone(),
        self.interactive.clone(),
      );
      Arc::new(move || {
        material.set(0);
        bend.set(45);
        bezel.set(20);
        blur.set(10);
        radius.set(36);
        interactive.set(false);
      })
    };
    let controls = vec![
      prop(
        "glass",
        "Glass",
        self.playground.select(0, &MATERIALS, &kind, move |value| {
          set_material.set(value);
          material_blur.set(if value == 2 { 4 } else { 10 });
        }),
      ),
      prop(
        "bend",
        "f32",
        self
          .playground
          .select(1, &BENDS, &bend, move |value| set_bend.set(value)),
      ),
      prop(
        "bezel",
        "f32",
        self
          .playground
          .select(2, &BEZELS, &bezel, move |value| set_bezel.set(value)),
      ),
      prop(
        "blur",
        "f32",
        self
          .playground
          .select(3, &BLURS, &blur, move |value| set_blur.set(value)),
      ),
      prop(
        "rounded",
        "impl Into<RadiusValue>",
        self
          .playground
          .select(4, &RADII, &radius, move |value| set_radius.set(value)),
      ),
      flag("interactive", &self.interactive),
    ];
    self.playground.card(
      ctx,
      Spec {
        title: "Glass",
        make: Arc::new(build),
        shown: Shown::built(move |_| build()),
        fill: false,
        forceable: false,
        facts: format!("bend {bend}, bezel {bezel_value:?}, blur {blur_value:?}"),
        event: "",
        code,
        tree,
        controls,
        changed: [kind != 0, bend != 45, bezel != 20, blur != 10, radius != 36, interactive]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset,
      },
    )
  }
}

impl Component for GlassPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 5),
      material: ctx.signal(0),
      bend: ctx.signal(45),
      bezel: ctx.signal(20),
      blur: ctx.signal(10),
      radius: ctx.signal(36),
      interactive: ctx.signal(false),
      together: ctx.signal(true),
      wide: ctx.signal(false),
      there: ctx.signal(true),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let rows = [
      (
        "glass",
        "Glass",
        "none",
        "Makes the element a piece of glass over what is painted behind it, with its box and its corners. The \
         element paints itself over the glass: it wants no fill of its own.",
      ),
      (
        "glass_container",
        "f32",
        "none",
        "The glass elements under this one are one surface: two that come within this distance of each other \
         flow into one shape. Up to eight of them.",
      ),
      (
        "Glass::regular()",
        "",
        "",
        "The glass of buttons, bars and menus. It blurs what is behind it and follows it: light with dark words \
         over a light backdrop, dark with light words over a dark one.",
      ),
      (
        "Glass::panel()",
        "",
        "",
        "The glass of a large surface: a sheet, a sidebar. Whiter, and light or dark by the app's appearance.",
      ),
      (
        "Glass::clear()",
        "",
        "",
        "A glass that leaves what is behind it nearly sharp, for over a picture or a video. It gives no colour to \
         what is on it.",
      ),
      (
        "bend",
        "f32",
        "45",
        "How far inside itself the very edge shows the backdrop from: the strength of the lens. 0 bends nothing.",
      ),
      (
        "bezel",
        "f32",
        "19.5",
        "The width of the curved edge. The glass is flat inside it.",
      ),
      (
        "blur",
        "f32",
        "10.5",
        "The blur of what is behind, as backdrop_blur's radius.",
      ),
      (
        "tint",
        "Color",
        "white, half",
        "The colour of the glass, laid over what is behind by its alpha. A coloured glass keeps its colour over \
         anything.",
      ),
      (
        "saturation, brightness",
        "f32",
        "1.25, 0.1",
        "What the glass does to the colours behind it before its tint.",
      ),
      (
        "rim",
        "f32",
        "0.68",
        "The light along the rim. It leans with the device where the device tells how it is held.",
      ),
      (
        "shadow",
        "GlassShadow",
        "soft, 8 down",
        "The shadow under the glass; it follows the shapes of a container as they flow together.",
      ),
      (
        "over_dark",
        "Option<GlassLook>",
        "a dark glass",
        "The look over a dark backdrop. None for a glass that looks the same over anything.",
      ),
      (
        "follows",
        "GlassFollows",
        "Backdrop",
        "What decides between the two looks: what is behind the glass, or the app's appearance alone.",
      ),
      (
        "ink",
        "Option<Color>",
        "black",
        "The colour of what is on the glass: the foreground of the element's words where it names none.",
      ),
      (
        "interactive",
        "bool",
        "false",
        "The glass answers a press: held down it grows and is lit from where it is touched; let go it springs \
         back.",
      ),
      (
        "amount",
        "f32",
        "1",
        "How much of the material is there. A transition of the glass takes it to and from nothing.",
      ),
    ];
    let playground = self.play(ctx);
    let (together, wide, there) = (self.together.get(), self.wide.get(), self.there.get());
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Engine", "Glass"],
        title: "Glass",
        description: "A box that is a piece of glass over what is painted behind it: it bends it along its edge as \
                      a lens does, blurs, saturates and tints it, and has a light on its rim and a shadow under it. \
                      The lens and the materials are the ones measured on a phone's system.",
        import: Some("use lurq::node::Glass;"),
        status: Some("New"),
        since: None,
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "One glass over a pattern. Choose its material, how far its edge bends, how wide the bend is and how \
           much it blurs, and its corners; make it answer a press and hold it down.",
          playground,
        ),
        Section::new(
          "Materials",
          "Regular blurs what is behind it enough for words on it to be read over anything. Panel is the regular \
           glass of a large surface, whiter. Clear leaves what is behind it nearly sharp, for over a picture. A \
           tint colours the glass.",
          self.frame.demo(
            0,
            fit("glass-materials", self.frame.stage_room(), SCENE, materials()),
            ".glass(Glass::regular())",
            &[
              "Row::new()",
              "    .rounded(RadiusSize::Full)",
              "    .glass(Glass::regular())",
              "    .child(Text::new(\"regular\"))",
              "",
              "Row::new().rounded(RadiusSize::Full).glass(Glass::panel())",
              "Row::new().rounded(RadiusSize::Full).glass(Glass::clear())",
              "Row::new().rounded(RadiusSize::Full).glass(Glass::regular().tint(blue))",
            ],
          ),
        ),
        Section::new(
          "The lens",
          "The middle of a glass shows what is behind it where it is; its edge pulls it in from further inside, \
           the more the nearer the edge, so that at the very edge the picture runs backwards. The same clear \
           glass with no bend, with a little, and with the system's.",
          self.frame.demo(
            1,
            fit("glass-lens", self.frame.stage_room(), SCENE, lens()),
            ".glass(Glass::clear().bend(45.0))",
            &[
              "Column::new().rounded(40.0).glass(Glass::clear().bend(0.0))",
              "Column::new().rounded(40.0).glass(Glass::clear().bend(20.0))",
              "Column::new().rounded(40.0).glass(Glass::clear().bend(45.0))",
            ],
          ),
        ),
        Section::new(
          "It follows what is behind it",
          "A regular glass is light, with dark words on it, over something light, and dark with light words over \
           something dark. Scroll the page under this one: it turns as the page goes by, and what is on it turns \
           with it.",
          self.frame.demo(
            2,
            fit("glass-follows", self.frame.stage_room(), SCENE, follows()),
            ".glass(Glass::regular())",
            &[
              "Stack::new()",
              "    .child(ScrollVertical::new(page))",
              "    .child(",
              "        Row::new()",
              "            .rounded(RadiusSize::Full)",
              "            .glass(Glass::regular())",
              "            // No colour of its own: the glass gives it one.",
              "            .child(Text::new(\"follows\")),",
              "    )",
            ],
          ),
        ),
        Section::new(
          "Shapes that flow together",
          "The glasses in a container are one surface: two that come within the container's distance of each \
           other join into one drop, and part again further off.",
          self.frame.demo(
            3,
            with_button(
              fit("glass-drops", self.frame.stage_room(), SCENE, drops(together)),
              turn(&self.together, "Move them apart", "Bring them together"),
            ),
            ".glass_container(36.0)",
            &[
              "Row::new()",
              "    .glass_container(36.0)",
              "    .child(Row::new().rounded(RadiusSize::Full).glass(Glass::regular()))",
              "    .child(Rect::new(gap, 1.0).transition(Transition::width()))",
              "    .child(Row::new().rounded(RadiusSize::Full).glass(Glass::regular()))",
            ],
          ),
        ),
        Section::new(
          "It answers a press",
          "An interactive glass grows a little while it is held down and is lit from where the pointer or the \
           finger is; let go, it springs back.",
          self.frame.demo(
            4,
            fit("glass-press", self.frame.stage_room(), SCENE, pressed()),
            ".glass(Glass::regular().interactive(true))",
            &[
              "Row::new()",
              "    .rounded(RadiusSize::Full)",
              "    .glass(Glass::regular().interactive(true))",
              "    .on_click(..)",
              "    .child(Text::new(\"Press me\"))",
            ],
          ),
        ),
        Section::new(
          "It changes shape",
          "A glass is its element's box: an element that grows, moves or changes its corners with a transition \
           is a glass that flows from one shape to the other.",
          self.frame.demo(
            5,
            with_button(
              fit("glass-shape", self.frame.stage_room(), SCENE, shaped(wide)),
              turn(&self.wide, "Make it a button", "Open it"),
            ),
            ".transition(Transition::all())",
            &[
              "Row::new()",
              "    .size(if open { 300.0 } else { 56.0 }, if open { 120.0 } else { 56.0 })",
              "    .rounded(if open { 32.0 } else { 28.0 })",
              "    .glass(Glass::regular())",
              "    .transition(Transition::all().easing(Easing::Spring { velocity: 0.0 }))",
            ],
          ),
        ),
        Section::new(
          "It comes and goes as a material",
          "A transition of the glass takes its amount to and from nothing: its bend, its blur and its light grow \
           in together, where another thing would fade in.",
          self.frame.demo(
            6,
            with_button(
              fit("glass-coming", self.frame.stage_room(), SCENE, coming(there)),
              turn(&self.there, "Take the glass away", "Bring the glass back"),
            ),
            ".transition(Transition::glass())",
            &[
              "let mut pane = Row::new()",
              "    .rounded(RadiusSize::Full)",
              "    .transition(Transition::glass().duration_ms(450));",
              "if there {",
              "    pane = pane.glass(Glass::regular());",
              "}",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What an element is given to be glass, and the numbers of the material.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
