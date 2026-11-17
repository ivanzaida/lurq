//! The page of the engine's SVG node: a vector picture parsed by usvg and rasterized at the size it is drawn, whose
//! fills, strokes and opacity can be given the theme's colors. Its playground shows one of three pictures as its
//! props say; its sections are the demo's SVG showcase (from a string and from bytes, its own size, a size given,
//! strokes, colors given, from a resource) with a picture drawn at any size.

use std::sync::Arc;

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{PaletteColor, RadiusSize},
  },
  components::{Column, Svg},
  core::Signal,
  layout::text_style::FontWeight,
  node::{Element, color::Color},
  svg::SvgData,
};

use super::kit::{controls, number, sample, samples, well};
use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, fit, line, props_table, text},
  playground::{Playground, Shown, Spec, divider, prop},
};

/// The demo's picture: a line chart of two series over two axes, 800 by 800, drawn with strokes in colors of
/// its own and two arrowheads filled.
const CHART: &str = include_str!("../../../assets/engine/line-chart-parallel-svgrepo-com.svg");
const CHART_BYTES: &[u8] = include_bytes!("../../../assets/engine/line-chart-parallel-svgrepo-com.svg");
/// A picture of strokes only, 96 by 96: an icon of a picture.
const ICON: &str = concat!(
  r##"<svg xmlns="http://www.w3.org/2000/svg" width="96" height="96" viewBox="0 0 24 24" fill="none" "##,
  r##"stroke="#202020" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"##,
  r##"<rect x="3" y="3" width="18" height="18" rx="2"/><circle cx="9" cy="9" r="2"/>"##,
  r##"<path d="M21 15 L16 10 L5 21"/></svg>"##,
);
/// A picture of fills, 120 by 80: a circle, and a square with an edge.
const SHAPES: &str = concat!(
  r##"<svg xmlns="http://www.w3.org/2000/svg" width="120" height="80" viewBox="0 0 120 80">"##,
  r##"<circle cx="30" cy="40" r="24" fill="#3b82f6"/>"##,
  r##"<rect x="62" y="16" width="48" height="48" rx="8" fill="#f97316" stroke="#202020" stroke-width="3"/></svg>"##,
);

/// The pictures of the playground, with how the code names each.
const SOURCES: [(&str, &str); 3] = [("CHART", CHART), ("ICON", ICON), ("SHAPES", SHAPES)];
const PICTURES: [(&str, usize); 3] = [("Chart", 0), ("Icon", 1), ("Shapes", 2)];
/// The sizes: a width, from which the height follows, and the picture's own size, which the chart's 800 by 800
/// is too large to be shown at here.
const SIZES: [(&str, usize); 4] = [("width 48", 0), ("width 120", 1), ("width 240", 2), ("own size", 3)];
const WIDTHS: [f32; 3] = [48.0, 120.0, 240.0];
/// The colors a fill or a stroke can be given: none, which keeps the picture's own, or a role of the theme.
const ROLES: [(&str, Option<PaletteColor>); 4] = [
  ("as drawn", None),
  ("Foreground", Some(PaletteColor::Foreground)),
  ("Primary", Some(PaletteColor::Primary)),
  ("Chart2", Some(PaletteColor::Chart2)),
];
/// The choices of the color selects: an index into [`ROLES`], which a signal keeps.
const ROLE_CHOICES: [(&str, usize); 4] = [("as drawn", 0), ("Foreground", 1), ("Primary", 2), ("Chart2", 3)];
/// The opacities, in percent.
const OPACITIES: [(&str, i32); 3] = [("1.0", 100), ("0.6", 60), ("0.3", 30)];

/// The color of a role in the theme a picture is built in: what an override is given.
fn color(role: &PaletteColor) -> Color {
  lurq::ui::current_theme().palette_color(role.clone())
}

/// A picture's data with the colors of the roles chosen, and an opacity under 100.
fn data(source: &str, fill: usize, stroke: usize, opacity: i32) -> SvgData {
  let mut data = SvgData::from_str(source);
  if let Some(role) = &ROLES[fill.min(ROLES.len() - 1)].1 {
    data = data.with_fill(color(role));
  }
  if let Some(role) = &ROLES[stroke.min(ROLES.len() - 1)].1 {
    data = data.with_stroke(color(role));
  }
  if opacity < 100 {
    data = data.with_opacity(opacity as f32 / 100.0);
  }
  data
}

/// What the playground shows.
fn shown(picture: usize, size: usize, fill: usize, stroke: usize, opacity: i32) -> Element {
  let svg = Svg::new(data(SOURCES[picture].1, fill, stroke, opacity));
  match WIDTHS.get(size) {
    Some(width) => svg.width(*width).into(),
    None => svg.into(),
  }
}

/// The chart from a string and from the same bytes, each as wide as its well and 120 high.
fn sources() -> lurq::components::Row {
  let preview = |svg: Svg, caption: &str| {
    sample(
      well()
        .width(320.0)
        .max_width(FILL)
        .padding(8.0)
        .child(svg.size(FILL, 120.0).rounded(RadiusSize::Sm).clip()),
      caption,
    )
  };
  samples()
    .child(preview(Svg::from_str(CHART), "Svg::from_str"))
    .child(preview(Svg::from_bytes(CHART_BYTES), "Svg::from_bytes"))
}

/// The icon and the shapes at their own sizes, in wells that show their bounds.
fn own_sizes() -> lurq::components::Row {
  samples()
    .child(sample(well().child(Svg::from_str(ICON)), "ICON: 96 × 96"))
    .child(sample(well().child(Svg::from_str(SHAPES)), "SHAPES: 120 × 80"))
}

/// The chart in the theme's colors and as drawn: its strokes given one color, its fills and strokes two, and the
/// whole of it faded.
fn colored() -> lurq::components::Row {
  let preview = |data: SvgData, caption: &str| {
    sample(
      well()
        .width(150.0)
        .max_width(FILL)
        .padding(12.0)
        .child(Svg::new(data).width(FILL)),
      caption,
    )
  };
  samples()
    .child(preview(SvgData::from_str(CHART), "as drawn"))
    .child(preview(
      SvgData::from_str(CHART).with_stroke(color(&PaletteColor::Primary)),
      "with_stroke(Primary)",
    ))
    .child(preview(
      SvgData::from_str(CHART)
        .with_fill(color(&PaletteColor::Chart2))
        .with_stroke(color(&PaletteColor::Foreground)),
      "with_fill + with_stroke",
    ))
    .child(preview(
      SvgData::from_str(CHART)
        .with_stroke(color(&PaletteColor::Foreground))
        .with_opacity(0.4),
      "with_opacity(0.4)",
    ))
}

/// The chart read from a file of the app's resources, on a desktop.
#[cfg(not(target_arch = "wasm32"))]
fn from_resource() -> Element {
  match super::images::resource("line-chart-parallel-svgrepo-com.svg") {
    Some(path) => sample(
      well()
        .width(320.0)
        .max_width(FILL)
        .padding(8.0)
        .child(Svg::from_resource(&path).size(FILL, 120.0)),
      "Svg::from_resource(path)",
    )
    .into(),
    None => text(
      "The folder for temporary files cannot be written: there is no file to read.",
      13.0,
      FontWeight::Normal,
    )
    .color(PaletteColor::MutedForeground)
    .into(),
  }
}

/// A browser's build reads no file: a line says what the desktop shows.
#[cfg(target_arch = "wasm32")]
fn from_resource() -> Element {
  text(
    "A browser's build reads no file of the app's: on a desktop the chart is read from its file here. The \
     sections above show it parsed from the page's own text.",
    13.0,
    FontWeight::Normal,
  )
  .color(PaletteColor::MutedForeground)
  .into()
}

pub struct SvgPage {
  frame: Frame,
  playground: Playground,
  /// An index into [`SOURCES`].
  picture: Signal<usize>,
  /// An index into [`SIZES`].
  size: Signal<usize>,
  /// Indices into [`ROLES`].
  fill: Signal<usize>,
  stroke: Signal<usize>,
  /// In percent.
  opacity: Signal<i32>,
  /// The width the chart is drawn at in the section that sizes it.
  side: Signal<i32>,
}

impl SvgPage {
  fn play(&self, ctx: &mut Ctx) -> Element {
    let picture = self.picture.get().min(SOURCES.len() - 1);
    // The chart is too large to be shown at its own size: the widest width stands for it.
    let size = if picture == 0 {
      self.size.get().min(2)
    } else {
      self.size.get()
    };
    let (fill, stroke, opacity) = (self.fill.get(), self.stroke.get(), self.opacity.get());
    let build = move || shown(picture, size, fill, stroke, opacity);

    let name = SOURCES[picture].0;
    let role = |at: usize| ROLES[at.min(ROLES.len() - 1)].0;
    let mut code = Vec::new();
    if fill == 0 && stroke == 0 && opacity == 100 {
      code.push(format!("Svg::from_str({name})"));
    } else {
      code.push("Svg::new(".to_owned());
      code.push(format!("    SvgData::from_str({name})"));
      if fill != 0 {
        code.push(format!(
          "        .with_fill(theme.palette_color(PaletteColor::{}))",
          role(fill)
        ));
      }
      if stroke != 0 {
        code.push(format!(
          "        .with_stroke(theme.palette_color(PaletteColor::{}))",
          role(stroke)
        ));
      }
      if opacity < 100 {
        code.push(format!("        .with_opacity({:.1})", opacity as f32 / 100.0));
      }
      code.push(")".to_owned());
    }
    if let Some(width) = WIDTHS.get(size) {
      code.push(format!("    .width({width:.1})"));
    }

    let (set_picture, set_size, set_fill, set_stroke, set_opacity) = (
      self.picture.clone(),
      self.size.clone(),
      self.fill.clone(),
      self.stroke.clone(),
      self.opacity.clone(),
    );
    let reset = {
      let (picture, size, fill, stroke, opacity) = (
        self.picture.clone(),
        self.size.clone(),
        self.fill.clone(),
        self.stroke.clone(),
        self.opacity.clone(),
      );
      Arc::new(move || {
        picture.set(0);
        size.set(2);
        fill.set(0);
        stroke.set(0);
        opacity.set(100);
      })
    };
    let sizes: &[(&str, usize)] = if picture == 0 { &SIZES[..3] } else { &SIZES };
    let controls = vec![
      prop(
        "source",
        "&str",
        self
          .playground
          .select(0, &PICTURES, &picture, move |value| set_picture.set(value)),
      ),
      prop(
        "width",
        "impl Into<Dimension>",
        self
          .playground
          .select(1, sizes, &size, move |value| set_size.set(value)),
      ),
      divider(),
      prop(
        "with_fill",
        "Color",
        self
          .playground
          .select(2, &ROLE_CHOICES, &fill, move |value| set_fill.set(value)),
      ),
      prop(
        "with_stroke",
        "Color",
        self
          .playground
          .select(3, &ROLE_CHOICES, &stroke, move |value| set_stroke.set(value)),
      ),
      prop(
        "with_opacity",
        "f32",
        self
          .playground
          .select(4, &OPACITIES, &opacity, move |value| set_opacity.set(value)),
      ),
    ];
    let own = match picture {
      0 => "800 × 800",
      1 => "96 × 96",
      _ => "120 × 80",
    };
    self.playground.card(
      ctx,
      Spec {
        title: "Svg",
        make: Arc::new(build),
        shown: Shown::built(move |_| build()),
        fill: false,
        forceable: false,
        facts: format!("its own size {own}"),
        event: "",
        code,
        tree: vec![format!("svg   {}", self.playground.measured())],
        controls,
        changed: [picture != 0, size != 2, fill != 0, stroke != 0, opacity != 100]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset,
      },
    )
  }

  /// The chart at the width the slider gives it: rasterized at that size, sharp at every one. Wider than the
  /// card, it scrolls sideways in it.
  fn scaled(&self) -> Column {
    let side = self.side.get().max(1) as f32;
    let chart = well().padding(12.0).child(
      Svg::new(SvgData::from_str(CHART).with_stroke(color(&PaletteColor::Foreground)))
        .width(side)
        .border(line()),
    );
    Column::new()
      .width(FILL)
      .spacing(16.0)
      .child(controls().child(number("width", &self.side, 24, 480)))
      .child(fit("engine-svg-scaled", self.frame.stage_room(), side + 26.0, chart))
  }
}

impl Component for SvgPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 5),
      picture: ctx.signal(0),
      size: ctx.signal(2),
      fill: ctx.signal(0),
      stroke: ctx.signal(0),
      opacity: ctx.signal(100),
      side: ctx.signal(160),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let rows = [
      (
        "from_str / from_bytes",
        "&str / &[u8]",
        "",
        "A picture parsed from its text. The parse is kept by the text's hash, so building the same picture again at \
         each render parses nothing. Text that is no SVG panics: an app gives SVG it knows.",
      ),
      (
        "new",
        "SvgData",
        "",
        "A picture of data the app has parsed, with colors given to it.",
      ),
      (
        "from_resource",
        "&str",
        "",
        "A file under the app's resource root, or at a whole path, read off the window's thread. A desktop build \
         with http.",
      ),
      (
        "width / height / size",
        "impl Into<Dimension>",
        "its own",
        "The box the picture is drawn in. A side alone gives the other the picture's proportions; none is its \
         own size, the width and height it says.",
      ),
      (
        "SvgData::with_fill",
        "Color",
        "",
        "Every fill of the picture in one color, at each shape's own opacity.",
      ),
      (
        "SvgData::with_stroke",
        "Color",
        "",
        "Every stroke of the picture in one color.",
      ),
      (
        "SvgData::with_opacity",
        "f32",
        "1.0",
        "The whole picture faded, from 0 to 1.",
      ),
    ];
    let playground = self.play(ctx);
    let scaled = self.scaled();
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Engine", "SVG"],
        title: "SVG",
        description: "A vector picture, parsed by usvg and rasterized at the size it is drawn, so it is sharp at any \
                      size and scale. Its fills, strokes and opacity can be given colors of the theme. Under lurq's \
                      canvas feature.",
        import: Some("use lurq::{components::Svg, svg::SvgData};"),
        status: Some("Stable"),
        since: None,
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "One of three pictures at a width, from which its height follows. Give its fills and its strokes a color \
           of the theme, and fade it.",
          playground,
        ),
        Section::new(
          "From a string or from bytes",
          "The same picture parsed from its text and from its bytes, in a box as wide as its well and 120 high.",
          self.frame.demo(
            0,
            sources(),
            "Svg::from_str(CHART)",
            &[
              "const CHART: &str = include_str!(\"line-chart.svg\");",
              "const CHART_BYTES: &[u8] = include_bytes!(\"line-chart.svg\");",
              "",
              "Svg::from_str(CHART).size(Dimension::Pct(100.0), 120.0)",
              "Svg::from_bytes(CHART_BYTES).size(Dimension::Pct(100.0), 120.0)",
            ],
          ),
        ),
        Section::new(
          "Its own size",
          "A picture given no size is as wide and as high as it says it is. The chart says 800 by 800, larger than \
           this page, so these are two smaller ones.",
          self.frame.demo(
            1,
            own_sizes(),
            "Svg::from_str(ICON)",
            &[
              "// <svg width=\"96\" height=\"96\" viewBox=\"0 0 24 24\"> ..",
              "Svg::from_str(ICON)",
              "",
              "// <svg width=\"120\" height=\"80\" viewBox=\"0 0 120 80\"> ..",
              "Svg::from_str(SHAPES)",
            ],
          ),
        ),
        Section::new(
          "Drawn at its size",
          "The chart's strokes at the width the slider gives it, and as high: the picture is rasterized again at \
           each size, so its lines are as sharp at 24 as at 480.",
          self.frame.demo(
            2,
            scaled,
            ".width(160.0)",
            &[
              "Svg::new(SvgData::from_str(CHART).with_stroke(foreground))",
              &format!("    .width({}.0)", self.side.get()),
            ],
          ),
        ),
        Section::new(
          "Colors of the theme",
          "The chart as its file draws it, whose dark axes are dark in either theme, and given the theme's colors: \
           its strokes, its fills and strokes, and the whole of it faded. A color given is the theme's as it is \
           when the picture is built.",
          self.frame.demo(
            3,
            colored(),
            "SvgData::from_str(CHART).with_stroke(primary)",
            &[
              "let theme = lurq::ui::current_theme();",
              "Svg::new(",
              "    SvgData::from_str(CHART)",
              "        .with_fill(theme.palette_color(PaletteColor::Chart2))",
              "        .with_stroke(theme.palette_color(PaletteColor::Foreground)),",
              ")",
            ],
          ),
        ),
        Section::new(
          "From a resource",
          "A path among the app's resources is read off the window's thread and parsed when it is there. A path \
           that is whole is read as it is: the page writes the chart to the system's folder for temporary files and \
           gives its path.",
          self.frame.demo(
            4,
            from_resource(),
            "Svg::from_resource(\"line-chart.svg\")",
            &[
              "app.set_resource_root(PathBuf::from(\"assets\"));",
              "",
              "Svg::from_resource(\"line-chart.svg\").size(Dimension::Pct(100.0), 120.0)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "How a picture is made, the box it is drawn in, and the colors its data can be given.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
