//! The page of a box's background picture: pixels any box of the engine has under its children, fitted to it as
//! `background_size` says. Its playground is a box with one of four pictures as its background; its sections are
//! the demo's backgrounds (covering their boxes and contained in them), the moving ones (a GIF and a WebP), and
//! backgrounds read from the app's resources.
//!
//! A picture that is the content of a box, with its loading and its error, is `ui::Picture`, which the Picture
//! page shows; this page shows only what a box has under its children.

use std::sync::{Arc, OnceLock};

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{PaletteColor, RadiusSize},
  },
  components::{Column, Stack},
  core::Signal,
  images::ImageData,
  layout::{Alignment, StackAlignment, text_style::FontWeight},
  node::{BackgroundSize, Element},
  ui::{Length, Skeleton},
};

use super::kit::samples;
use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, line, props_table, quiet, text},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

/// A picture the page shows: what it is, its file in `assets/engine`, and its bytes.
struct Asset {
  label: &'static str,
  file: &'static str,
  bytes: &'static [u8],
}

/// The demo's four pictures: a photograph, a drawing with a transparent ground, and two that move.
const ASSETS: [Asset; 4] = [
  Asset {
    label: "JPG",
    file: "skebob.jpg",
    bytes: include_bytes!("../../../assets/engine/skebob.jpg"),
  },
  Asset {
    label: "PNG, alpha",
    file: "transparent.png",
    bytes: include_bytes!("../../../assets/engine/transparent.png"),
  },
  Asset {
    label: "GIF, 48 frames",
    file: "six-seven.gif",
    bytes: include_bytes!("../../../assets/engine/six-seven.gif"),
  },
  Asset {
    label: "WebP, 12 frames",
    file: "animated-webp-supported.webp",
    bytes: include_bytes!("../../../assets/engine/animated-webp-supported.webp"),
  },
];
/// The indices of the two that move.
const MOVING: [usize; 2] = [2, 3];

/// The four pictures decoded, every frame of those that move. `ImageData::from_bytes` decodes them whole, which
/// takes a moment in a debug build: once for the process, off the window's thread where there is another.
static DECODED: OnceLock<Vec<ImageData>> = OnceLock::new();

fn decode() -> Vec<ImageData> {
  ASSETS
    .iter()
    .map(|asset| ImageData::from_bytes(asset.bytes).expect("the page's own picture decodes"))
    .collect()
}

/// Decodes the pictures if nothing has yet, and says so to `ready` when they are.
fn load(ready: &Signal<bool>) {
  if DECODED.get().is_some() {
    return;
  }
  let ready = ready.clone();
  // A browser's page has one thread: the pictures are decoded on it.
  #[cfg(target_arch = "wasm32")]
  {
    DECODED.get_or_init(decode);
    ready.set(true);
  }
  #[cfg(not(target_arch = "wasm32"))]
  std::thread::spawn(move || {
    DECODED.get_or_init(decode);
    ready.set(true);
  });
}

/// The `index`th picture's pixels, once they are decoded.
fn pixels(index: usize) -> Option<ImageData> {
  DECODED.get().map(|all| all[index].clone())
}

/// The line chart the SVG page shows, with the pictures among the files a resource is read from.
#[cfg(not(target_arch = "wasm32"))]
const CHART: (&str, &[u8]) = (
  "line-chart-parallel-svgrepo-com.svg",
  include_bytes!("../../../assets/engine/line-chart-parallel-svgrepo-com.svg"),
);

/// The path of one of the engine pages' pictures as a resource of the app, on a desktop: the files are written
/// once to a folder of the system's for temporary files. A path that is whole is read as it is: the gallery sets
/// no root for its resources. `None` where the folder cannot be written.
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn resource(file: &str) -> Option<String> {
  static FOLDER: OnceLock<Option<std::path::PathBuf>> = OnceLock::new();
  let folder = FOLDER
    .get_or_init(|| {
      let folder = std::env::temp_dir().join("lurq-gallery-engine");
      std::fs::create_dir_all(&folder).ok()?;
      let files = ASSETS.iter().map(|asset| (asset.file, asset.bytes)).chain([CHART]);
      for (name, bytes) in files {
        std::fs::write(folder.join(name), bytes).ok()?;
      }
      Some(folder)
    })
    .as_ref()?;
  Some(folder.join(file).to_string_lossy().into_owned())
}

/// What stands where a picture will be while it is decoded.
fn waiting(width: impl Into<Length>, height: impl Into<Length>) -> Element {
  Skeleton::block(width, height).into()
}

const PICTURES: [(&str, usize); 4] = [("JPG", 0), ("PNG", 1), ("GIF", 2), ("WebP", 3)];
const FITS: [(&str, BackgroundSize); 4] = [
  ("Stretch", BackgroundSize::Stretch),
  ("Contain", BackgroundSize::Contain),
  ("Cover", BackgroundSize::Cover),
  ("Natural", BackgroundSize::Natural),
];
/// The choices of the fit's select: an index into [`FITS`], which a signal keeps.
const FIT_CHOICES: [(&str, usize); 4] = [("Stretch", 0), ("Contain", 1), ("Cover", 2), ("Natural", 3)];
const SIZES: [(&str, usize); 4] = [("160 × 160", 0), ("160 × 180", 1), ("200 × 200", 2), ("FILL × 220", 3)];
const RADII: [(&str, i32); 3] = [("0", 0), ("8", 8), ("24", 24)];

/// The Rust of a size, as the playground's code says it.
fn size_code(size: usize) -> &'static str {
  match size {
    0 => ".size(160.0, 160.0)",
    1 => ".size(160.0, 180.0)",
    2 => ".size(200.0, 200.0)",
    _ => ".size(Dimension::Pct(100.0), 220.0)",
  }
}

/// What the playground shows: a box with the `picture`th picture as its background, and a label in it.
fn shown(picture: usize, fit: usize, size: usize, radius: i32, border: bool) -> Element {
  let Some(pixels) = pixels(picture) else {
    return waiting(200.0, 200.0);
  };
  let fit = FITS[fit.min(FITS.len() - 1)].1;
  let mut stage = Stack::new()
    .background(PaletteColor::Muted)
    .background_image(pixels)
    .background_size(fit)
    .child(label(ASSETS[picture].label).absolute_position(10.0, 10.0));
  stage = match size {
    0 => stage.size(160.0, 160.0),
    1 => stage.size(160.0, 180.0),
    2 => stage.size(200.0, 200.0),
    _ => stage.size(FILL, 220.0),
  };
  if radius > 0 {
    stage = stage.rounded(radius as f32).clip();
  }
  if border {
    stage = stage.border(line());
  }
  stage.into()
}

/// A label over a picture: what a child of a box with a background is.
fn label(content: &str) -> Column {
  Column::new()
    .padding_horizontal(8.0)
    .padding_vertical(4.0)
    .rounded(RadiusSize::Sm)
    .background(PaletteColor::Card)
    .foreground(PaletteColor::CardForeground)
    .child(text(content, 12.0, FontWeight::SemiBold).nowrap())
}

/// A box of 160 by 132 with the `index`th picture as its background, fitted as `fit` says, and a label in it.
fn backdrop(index: usize, fit: BackgroundSize, height: f32) -> Element {
  let Some(pixels) = pixels(index) else {
    return waiting(Length::Fill, height);
  };
  Stack::new()
    .size(FILL, height)
    .background(PaletteColor::Muted)
    .background_image(pixels)
    .background_size(fit)
    .rounded(6.0)
    .clip()
    .border(line())
    .child(label(ASSETS[index].label).absolute_position(10.0, 10.0))
    .into()
}

/// The four pictures as backgrounds, covering their boxes and then contained in them.
fn backgrounds() -> Column {
  let line_of = |name: &str, fit: BackgroundSize| {
    Column::new()
      .width(FILL)
      .spacing(10.0)
      .child(quiet(name, 12.0))
      .child(samples().with_children(ASSETS.iter().enumerate().map(|(index, asset)| {
        Column::new()
          .width(160.0)
          .max_width(FILL)
          .spacing(6.0)
          .child(backdrop(index, fit, 132.0))
          .child(quiet(asset.file, 11.0))
      })))
  };
  Column::new()
    .width(FILL)
    .spacing(20.0)
    .child(line_of("BackgroundSize::Cover", BackgroundSize::Cover))
    .child(line_of("BackgroundSize::Contain", BackgroundSize::Contain))
}

/// The two pictures that move, as the backgrounds of boxes with a label in their middle.
fn moving_backgrounds() -> lurq::components::Row {
  samples().with_children(MOVING.into_iter().map(|index| {
    let shown: Element = match pixels(index) {
      Some(pixels) => Stack::new()
        .size(FILL, 220.0)
        .stack_align(StackAlignment::Center)
        .background(PaletteColor::Muted)
        .background_image(pixels)
        .background_cover()
        .rounded(8.0)
        .clip()
        .border(line())
        .child(
          label(ASSETS[index].label)
            .align_items(Alignment::Center)
            .child(quiet("background image", 11.0)),
        )
        .into(),
      None => waiting(Length::Fill, 220.0),
    };
    Column::new()
      .width(260.0)
      .max_width(FILL)
      .spacing(6.0)
      .child(shown)
      .child(quiet(ASSETS[index].file, 11.0))
  }))
}

/// The four pictures as the app's resources, read and decoded by the engine: each the background of a box.
#[cfg(not(target_arch = "wasm32"))]
fn resources() -> Element {
  let mut row = samples();
  for asset in &ASSETS {
    let Some(path) = resource(asset.file) else {
      return text(
        "The folder for temporary files cannot be written: there is no file to read.",
        13.0,
        FontWeight::Normal,
      )
      .color(PaletteColor::MutedForeground)
      .into();
    };
    row = row.child(
      Column::new()
        .width(160.0)
        .max_width(FILL)
        .spacing(6.0)
        .child(
          Stack::new()
            .size(FILL, 132.0)
            .background(PaletteColor::Muted)
            .background_image(path.as_str())
            .background_cover()
            .rounded(6.0)
            .clip()
            .border(line())
            .child(label(asset.label).absolute_position(10.0, 10.0)),
        )
        .child(text("background_image(path)", 12.0, FontWeight::SemiBold))
        .child(quiet(asset.file, 11.0)),
    );
  }
  row.into()
}

/// A browser's build reads no file: a line says what the desktop shows.
#[cfg(target_arch = "wasm32")]
fn resources() -> Element {
  text(
    "A browser's build reads no file of the app's: on a desktop the four pictures are read from their files here, \
     each the background of a box. The sections above show the same pictures.",
    13.0,
    FontWeight::Normal,
  )
  .color(PaletteColor::MutedForeground)
  .into()
}

pub struct BackgroundsPage {
  frame: Frame,
  playground: Playground,
  /// Whether the pictures are decoded.
  ready: Signal<bool>,
  /// An index into [`ASSETS`].
  picture: Signal<usize>,
  /// An index into [`FITS`].
  fit: Signal<usize>,
  /// An index into [`SIZES`].
  size: Signal<usize>,
  radius: Signal<i32>,
  border: Signal<bool>,
}

impl BackgroundsPage {
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (picture, fit_at, size, radius) = (self.picture.get(), self.fit.get(), self.size.get(), self.radius.get());
    let border = self.border.get();
    let picture = picture.min(ASSETS.len() - 1);
    let fit_name = FITS[fit_at.min(FITS.len() - 1)].0;
    let build = move || shown(picture, fit_at, size, radius, border);

    let asset = &ASSETS[picture];
    let mut code = vec![
      format!(
        "let pixels = ImageData::from_bytes(include_bytes!(\"{}\"))?;",
        asset.file
      ),
      "Stack::new()".to_owned(),
      format!("    {}", size_code(size)),
      "    .background(PaletteColor::Muted)".to_owned(),
      "    .background_image(pixels)".to_owned(),
    ];
    if fit_at != 0 {
      code.push(format!("    .background_size(BackgroundSize::{fit_name})"));
    }
    if radius > 0 {
      code.push(format!("    .rounded({radius}.0)"));
      code.push("    .clip()".to_owned());
    }
    if border {
      code.push("    .border(Border::inside(1.0, PaletteColor::Border))".to_owned());
    }
    code.push(format!(
      "    .child(label(\"{}\").absolute_position(10.0, 10.0))",
      asset.label
    ));

    let facts = match pixels(picture) {
      Some(pixels) => format!(
        "{} × {} px, {}",
        pixels.width(),
        pixels.height(),
        if pixels.is_animated() { "animated" } else { "still" }
      ),
      None => "decoding".to_owned(),
    };
    let tree = vec![
      format!("stack   {}   background image", self.playground.measured()),
      "└─ label".to_owned(),
    ];

    let (set_picture, set_fit, set_size, set_radius) = (
      self.picture.clone(),
      self.fit.clone(),
      self.size.clone(),
      self.radius.clone(),
    );
    let reset = {
      let (picture, fit, size, radius, border) = (
        self.picture.clone(),
        self.fit.clone(),
        self.size.clone(),
        self.radius.clone(),
        self.border.clone(),
      );
      Arc::new(move || {
        picture.set(0);
        fit.set(2);
        size.set(2);
        radius.set(8);
        border.set(true);
      })
    };
    let controls = vec![
      prop(
        "background_image",
        "ImageData",
        self
          .playground
          .select(0, &PICTURES, &picture, move |value| set_picture.set(value)),
      ),
      prop(
        "background_size",
        "BackgroundSize",
        self
          .playground
          .select(1, &FIT_CHOICES, &fit_at, move |value| set_fit.set(value)),
      ),
      prop(
        "size",
        "impl Into<Dimension>",
        self
          .playground
          .select(2, &SIZES, &size, move |value| set_size.set(value)),
      ),
      prop(
        "rounded",
        "impl Into<RadiusValue>",
        self
          .playground
          .select(3, &RADII, &radius, move |value| set_radius.set(value)),
      ),
      divider(),
      flag("border", &self.border),
    ];
    self.playground.card(
      ctx,
      Spec {
        title: "Background",
        make: Arc::new(build),
        shown: Shown::built(move |_| build()),
        fill: size == 3,
        forceable: false,
        facts,
        event: "",
        code,
        tree,
        controls,
        changed: [picture != 0, fit_at != 2, size != 2, radius != 8, !border]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset,
      },
    )
  }
}

impl Component for BackgroundsPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let ready = ctx.signal(DECODED.get().is_some());
    load(&ready);
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 4),
      ready,
      picture: ctx.signal(0),
      fit: ctx.signal(2),
      size: ctx.signal(2),
      radius: ctx.signal(8),
      border: ctx.signal(true),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    // Read so that the page is drawn again when the pictures are decoded.
    let _ = self.ready.get();
    let rows = [
      (
        "background_image",
        "impl Into<ImageKind>",
        "",
        "On any box: pixels drawn under its children, an ImageData or, on a desktop, a resource's path.",
      ),
      (
        "background_size",
        "BackgroundSize",
        "Stretch",
        "How the pixels fill the box: stretched to it, contained in it whole, covering it and cut, or at their own \
         size. background_cover() and background_contain() say the same.",
      ),
      (
        "ImageData::from_bytes / from_file",
        "&[u8] / Path",
        "",
        "The pixels a background is given, decoded by the app: a GIF or a WebP keeps all its frames, and moves. \
         ImageData::from_rgba makes them of RGBA.",
      ),
      (
        "App::set_resource_root",
        "PathBuf",
        "",
        "Where a background's path is read from: the engine reads and decodes it off the window's thread, every \
         frame of it. A path that is whole is read as it is. A desktop build with http.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Engine", "Backgrounds"],
        title: "Backgrounds",
        description: "Any box of the engine can have a picture under its children, fitted to the box as its \
                      background size says. A picture that is what a box shows, with its loading and its \
                      error, is ui::Picture: the Picture page shows it.",
        import: Some("use lurq::{images::ImageData, node::BackgroundSize};"),
        status: Some("Stable"),
        since: None,
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "A box with one of the four pictures as its background. Choose how the picture fills the box, the \
           box's size, its corners and its edge; the label is a child of the box, drawn over the picture.",
          playground,
        ),
        Section::new(
          "Cover and contain",
          "Cover fills the box and cuts what is past it; contain shows the whole picture and the box's own fill \
           beside it. A box that is to show a picture as its content, not under its children, is a ui::Picture \
           (the Picture page).",
          self.frame.demo(
            0,
            backgrounds(),
            ".background_image(pixels).background_cover()",
            &[
              "let pixels = ImageData::from_bytes(include_bytes!(\"skebob.jpg\"))?;",
              "Stack::new()",
              "    .size(Dimension::Pct(100.0), 132.0)",
              "    .background(PaletteColor::Muted)",
              "    .background_image(pixels)",
              "    .background_cover() // or .background_contain()",
              "    .rounded(6.0)",
              "    .clip()",
              "    .child(label(\"JPG\").absolute_position(10.0, 10.0))",
            ],
          ),
        ),
        Section::new(
          "Animated backgrounds",
          "A GIF and a WebP decoded with every frame: the background shows each frame for as long as the file \
           says, and the window draws while one is in view.",
          self.frame.demo(
            1,
            moving_backgrounds(),
            ".background_image(gif).background_cover()",
            &[
              "// Every frame, with how long each is shown.",
              "let gif = ImageData::from_bytes(include_bytes!(\"six-seven.gif\"))?;",
              "assert!(gif.is_animated());",
              "Stack::new()",
              "    .size(Dimension::Pct(100.0), 220.0)",
              "    .stack_align(StackAlignment::Center)",
              "    .background_image(gif)",
              "    .background_cover()",
              "    .child(label(\"GIF, 48 frames\"))",
            ],
          ),
        ),
        Section::new(
          "From the app's resources",
          "A path among the app's resources (App::set_resource_root) is read by the engine off the window's thread \
           and decoded with every frame: a GIF read so moves. A path that is whole is read as it is: the page \
           writes its pictures to the system's folder for temporary files and gives their paths.",
          self.frame.demo(
            2,
            resources(),
            ".background_image(\"six-seven.gif\")",
            &[
              "app.set_resource_root(PathBuf::from(\"assets\"));",
              "",
              "Stack::new()",
              "    .size(Dimension::Pct(100.0), 132.0)",
              "    .background_image(\"six-seven.gif\")",
              "    .background_cover()",
            ],
          ),
        ),
        Section::new(
          "Props",
          "The background any box can have, and the pixels it is given.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
